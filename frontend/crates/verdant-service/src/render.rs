use std::{collections::HashSet, path::{Path, PathBuf}, thread, time::{Duration, Instant}};

use serde_json::{Value, json};
use verdant_backend::{
    Action, ArtifactCache, Capabilities, Phrase, PhrasePlan, RenderScheduler, Request,
    Route, Severity, SlotStatus, VerifiedAudio, WorkerClient, WorkerConfig, WorkerEvent, WorkerMessage,
    adapter::{plan_all, plan_all_desktop},
    artifact::verify_and_decode_with_options,
    scheduler::CacheEntry,
    wire::PROTOCOL,
};
use verdant_domain::{Project, TrackId};

const RECEIVE_INTERVAL: Duration = Duration::from_millis(20);
const EXIT_GRACE: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct RenderError {
    pub code: &'static str,
    pub message: String,
    pub details: Option<Value>,
}

impl RenderError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), details: None }
    }

    fn worker(message: impl Into<String>) -> Self {
        Self::new("WORKER_FAILED", message)
    }
}

pub struct RenderSession {
    client: WorkerClient,
    messages: async_channel::Receiver<(u64, WorkerMessage)>,
    workspace: PathBuf,
    scheduler: RenderScheduler,
    exited: bool,
}

pub struct RenderedPhrase {
    pub phrase: Phrase,
    pub digest: String,
    pub audio: VerifiedAudio,
}

impl RenderSession {
    pub fn start(config: WorkerConfig, timeout: Duration) -> Result<Self, RenderError> {
        let (client, messages) = WorkerClient::spawn(config, 1)
            .map_err(|error| RenderError::worker(error.to_string()))?;
        let mut session = Self {
            client, messages, workspace: PathBuf::new(), scheduler: RenderScheduler::new(), exited: false,
        };
        let deadline = Instant::now() + timeout;
        match session.receive(deadline, &|| false)? {
            event @ WorkerEvent::Ready { .. } => {
                let WorkerEvent::Ready { workspace, .. } = &event else { unreachable!() };
                let reported = Path::new(workspace).canonicalize().map_err(|e| RenderError::worker(e.to_string()))?;
                let expected = session.client.config.workspace.canonicalize().map_err(|e| RenderError::worker(e.to_string()))?;
                if reported != expected {
                    return Err(RenderError::worker("worker reported a different workspace"));
                }
                session.workspace = expected;
                session.scheduler.on_event(event);
            }
            _ => return Err(RenderError::worker("worker did not announce readiness")),
        }
        let request_id = "cli-capabilities";
        session.send(&Request::capabilities(request_id))?;
        loop {
            match session.receive(deadline, &|| false)? {
                WorkerEvent::Capabilities { request_id: id, capabilities } if id.as_deref() == Some(request_id) => {
                    if capabilities.protocol != PROTOCOL || capabilities.test_double {
                        return Err(RenderError::new("VERSION_MISMATCH", "incompatible native worker capability response"));
                    }
                    let mut capabilities = *capabilities;
                    let unlocked_original = capabilities.voices.as_ref().is_some_and(|catalog| {
                        catalog.voices.get(verdant_backend::voices::DESKTOP_ORIGINAL_REFERENCE)
                            .is_some_and(verdant_backend::voices::is_desktop_original)
                    });
                    capabilities.routes.retain(|route| route == Route::VerdantVoice.as_str()
                        || route == Route::C0.as_str() && unlocked_original);
                    capabilities.files.remove("vocoder");
                    capabilities.other.remove("vocoder");
                    capabilities.other.remove("neural");
                    session.scheduler.capabilities = Some(capabilities);
                    return Ok(session);
                }
                WorkerEvent::Rejected { code, message, .. } => return Err(RenderError::worker(format!("{code}: {message}"))),
                _ => {}
            }
        }
    }

    pub fn capabilities(&self) -> &Capabilities {
        self.scheduler.capabilities.as_ref().expect("session completed capability handshake")
    }

    pub fn plan(&self, project: &Project, explicit_routes: bool, tracks: &[TrackId]) -> Vec<PhrasePlan> {
        let Some(first) = project.tracks.first() else { return Vec::new() };
        let mut plans = if explicit_routes {
            plan_all(project, &first.id, self.capabilities())
        } else {
            plan_all_desktop(project, &first.id, self.capabilities())
        };
        plans.retain(|plan| (tracks.is_empty() || tracks.contains(&plan.track))
            && project.track(&plan.track).is_some_and(|track| !track.notes.is_empty()));
        plans
    }

    pub fn render(
        mut self, project: &Project, revision: u64, plans: &[PhrasePlan], explicit_routes: bool,
        seed: u32, cache: &ArtifactCache, timeout: Duration, cancelled: impl Fn() -> bool,
        mut progress: impl FnMut(Value),
    ) -> Result<Vec<RenderedPhrase>, RenderError> {
        for plan in plans {
            if !plan.blocked.is_empty() || plan.diagnostics.iter().any(|d| d.severity == Severity::Error) {
                let voice_error = plan.diagnostics.iter().any(|d| matches!(d.code, "E_VOICE_SELECTION" | "E_RENDER_CAPABILITY"));
                return Err(RenderError {
                    code: if voice_error { "VOICE_NOT_ADMITTED" } else { "INVALID_PROJECT" },
                    message: format!("render preflight failed for track {}", plan.track),
                    details: Some(json!({"track":plan.track,"blocked":plan.blocked,"diagnostics":plan.diagnostics})),
                });
            }
        }
        if explicit_routes {
            self.scheduler.set_project_voices(project);
        } else {
            self.scheduler.set_desktop_project_voices(project);
        }
        self.scheduler.set_plans(revision, plans, seed);
        let expected = plans.iter().map(|plan| plan.phrases.len()).sum::<usize>();
        if self.scheduler.wanted().len() != expected {
            return Err(RenderError::new("UNSUPPORTED_CAPABILITY", "the worker cannot schedule every planned phrase"));
        }
        let mut restored = HashSet::new();
        let mut entries = Vec::new();
        for wanted in self.scheduler.wanted() {
            if restored.insert(wanted.digest.as_str()) {
                match cache.restore(&wanted.digest) {
                    Ok(Some(entry)) => entries.push(entry),
                    Ok(None) => {},
                    Err(error) => progress(json!({"stage":"cache","digest":wanted.digest,"warning":error.to_string()})),
                }
            }
        }
        for entry in entries {
            self.scheduler.import_verified(entry);
        }
        if expected > 0 && self.capabilities().max_active_jobs == 0 && !self.scheduler.all_ready() {
            return Err(RenderError::new("UNSUPPORTED_CAPABILITY", "worker has no render capacity"));
        }
        let deadline = Instant::now() + timeout;
        while expected > 0 && !self.scheduler.all_ready() {
            if cancelled() {
                self.cancel_active();
                return Err(RenderError::new("CANCELLED", "render cancelled"));
            }
            for action in self.scheduler.poll() {
                self.perform(action, cache, &mut progress)?;
            }
            let event = match self.receive(deadline, &cancelled) {
                Ok(event) => event,
                Err(error) if error.code == "CANCELLED" => {
                    self.cancel_active();
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
            match &event {
                WorkerEvent::Accepted { job_id, .. } => progress(json!({"stage":"accepted","phrase_job_id":job_id})),
                WorkerEvent::Progress { job_id, stage, phase, fraction } =>
                    progress(json!({"stage":stage,"phase":phase,"phrase_job_id":job_id,"fraction":fraction})),
                WorkerEvent::Completed { job_id, revision: reported, .. }
                    if self.scheduler.job(job_id).is_none_or(|job| job.revision != *reported) => {
                        return Err(RenderError::worker("completion does not match a submitted phrase revision"));
                    }
                WorkerEvent::Ready { .. } | WorkerEvent::Capabilities { .. } =>
                    return Err(RenderError::worker("worker changed its session or capabilities during rendering")),
                WorkerEvent::Cancelled { job_id, .. } =>
                    return Err(RenderError::worker(format!("unexpected cancellation of {job_id}"))),
                _ => {}
            }
            for action in self.scheduler.on_event(event) {
                self.perform(action, cache, &mut progress)?;
            }
            for index in 0..expected {
                if let Some(SlotStatus::Failed { message }) = self.scheduler.slot_status(index) {
                    return Err(RenderError::worker(message));
                }
            }
        }
        if cancelled() {
            return Err(RenderError::new("CANCELLED", "render cancelled"));
        }
        Ok(self.scheduler.playable_phrases().into_iter().map(|(phrase, entry)| RenderedPhrase {
            phrase: phrase.clone(), digest: entry.digest.clone(), audio: entry.audio.clone(),
        }).collect())
    }

    fn perform(&mut self, action: Action, cache: &ArtifactCache, progress: &mut impl FnMut(Value)) -> Result<(), RenderError> {
        match action {
            Action::Send(request) => self.send(&request),
            Action::Verify { job_id, route, wideband, artifacts } => {
                let audio = verify_and_decode_with_options(&self.workspace, route, wideband, &artifacts)
                    .map_err(|error| RenderError::new("ARTIFACT_INVALID", error))?;
                let job = self.scheduler.job(&job_id).ok_or_else(|| RenderError::worker("unknown completed phrase"))?;
                let wanted = self.scheduler.wanted().iter().find(|wanted| wanted.digest == job.digest)
                    .ok_or_else(|| RenderError::worker("completed phrase no longer belongs to the plan"))?;
                let entry = CacheEntry {
                    digest: job.digest.clone(), job_id: job_id.clone(), revision: job.revision, route, audio: audio.clone(),
                    track: Some(wanted.phrase.track.clone()), phrase_index: Some(wanted.phrase.index),
                    start_seconds: wanted.phrase.start_seconds, seconds: wanted.phrase.seconds,
                    last_wanted_revision: job.revision,
                };
                if let Err(error) = cache.store(&entry) {
                    progress(json!({"stage":"cache","digest":entry.digest,"warning":error.to_string()}));
                }
                self.scheduler.on_verified(&job_id, Ok(audio));
                progress(json!({"stage":"verified","phrase_job_id":job_id,"digest":entry.digest}));
                Ok(())
            }
        }
    }

    fn send(&self, request: &Request) -> Result<(), RenderError> {
        self.client.send(request).map_err(RenderError::worker)
    }

    fn receive(&mut self, deadline: Instant, cancelled: &impl Fn() -> bool) -> Result<WorkerEvent, RenderError> {
        loop {
            if cancelled() {
                return Err(RenderError::new("CANCELLED", "render cancelled"));
            }
            match self.messages.try_recv() {
                Ok((_, WorkerMessage::Event(event))) => return Ok(event),
                Ok((_, WorkerMessage::Malformed { error, .. })) => return Err(RenderError::worker(error)),
                Ok((_, WorkerMessage::Exited { status })) => {
                    self.exited = true;
                    return Err(RenderError::worker(format!("worker exited: {status}")));
                }
                Err(async_channel::TryRecvError::Closed) => return Err(RenderError::worker("worker event stream closed")),
                Err(async_channel::TryRecvError::Empty) => {}
            }
            if Instant::now() >= deadline {
                return Err(RenderError::worker("worker response deadline expired"));
            }
            thread::sleep(RECEIVE_INTERVAL);
        }
    }

    fn cancel_active(&mut self) {
        let ids: Vec<_> = self.scheduler.active_jobs().map(|job| job.job_id.clone()).collect();
        for id in ids {
            if let Some(request) = self.scheduler.request_cancel(&id) {
                let _ = self.send(&request);
            }
        }
        let deadline = Instant::now() + EXIT_GRACE;
        while self.scheduler.active().is_some() {
            match self.receive(deadline, &|| false) {
                Ok(event) => { self.scheduler.on_event(event); }
                Err(_) => break,
            }
        }
    }

    fn await_exit(&mut self, deadline: Instant) -> bool {
        while Instant::now() < deadline {
            match self.messages.try_recv() {
                Ok((_, WorkerMessage::Exited { .. })) | Err(async_channel::TryRecvError::Closed) => return true,
                Err(async_channel::TryRecvError::Empty) => thread::sleep(RECEIVE_INTERVAL),
                _ => {}
            }
        }
        false
    }
}

impl Drop for RenderSession {
    fn drop(&mut self) {
        self.client.close();
        if !self.exited && !self.await_exit(Instant::now() + EXIT_GRACE) {
            self.client.kill();
            let _ = self.await_exit(Instant::now() + EXIT_GRACE);
        }
    }
}
