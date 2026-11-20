//! Render scheduler: a pure state machine between the phrase plan and the
//! worker (SPEC §11). No I/O happens here; the app performs the returned
//! [`Action`]s and feeds worker events back.
//!
//! Correctness rules:
//! * A job's lifecycle is `submitted → accepted → running → completed |
//!   failed | cancelled` (or `rejected` before acceptance). The first terminal
//!   event wins; later ones are counted as protocol anomalies and ignored.
//! * `cancel_requested` is a local overlay; `cancel_ack` never makes a job
//!   cancelled (T09).
//! * Verified, uncancelled completion writes the content cache; it becomes the *playable binding*
//!   for a phrase only if its content digest is what the current plan wants.
//!   An old revision's result therefore never replaces the current binding
//!   (T08), while undo back to identical content legitimately hits the cache.
//! * Results become playable only after artifact verification succeeds.
//! * Cancellation is sent once, including while admission is pending; the
//!   worker slot remains occupied until its first terminal event. Once locally
//!   cancelled, a job can never publish artifacts, even if completion races it.
//! * [`RenderScheduler::schedule_plans`] replaces pending edits immediately,
//!   but waits for a quiet interval before submitting their newest content.
//!   The caller wakes at [`RenderScheduler::next_poll_at`] and polls again.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::adapter::{Phrase, PhrasePlan, payload_for, render_digest};
use crate::wire::{Artifact, Capabilities, Payload, Request, Route, WorkerEvent};

/// Quiet interval after an edit; subsequent phrases in the same plan render
/// immediately when a worker slot becomes available.
pub const RENDER_DEBOUNCE: Duration = Duration::from_millis(180);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Submitted,
    Accepted,
    Running,
    Completed,
    Failed,
    Cancelled,
    Rejected,
}

impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, JobState::Completed | JobState::Failed | JobState::Cancelled | JobState::Rejected)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Job {
    pub job_id: String,
    pub request_id: String,
    pub revision: u64,
    pub digest: String,
    pub route: Route,
    /// Captured at submission so stale jobs verify their own requested output.
    pub wideband: bool,
    pub state: JobState,
    pub cancel_requested: bool,
    pub stage: Option<String>,
    pub phase: Option<String>,
    pub error: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub phrase_index: usize,
    pub start_seconds: f64,
    pub terminal_events: u32,
}

impl Job {
    /// What the UI shows: the local cancel overlay never masks a terminal fact.
    pub fn display_state(&self) -> &'static str {
        match (self.state, self.cancel_requested) {
            (s, _) if s.is_terminal() => match s {
                JobState::Completed => "completed",
                JobState::Failed => "failed",
                JobState::Cancelled => "cancelled",
                _ => "rejected",
            },
            (_, true) => "cancel_requested",
            (JobState::Submitted, _) => "submitted",
            (JobState::Accepted, _) => "accepted",
            _ => "running",
        }
    }
}

/// Decoded, verified mono PCM ready for the audio engine.
#[derive(Clone, Debug)]
pub struct VerifiedAudio {
    pub path: std::path::PathBuf,
    pub sha256: String,
    pub sample_rate: u32,
    pub samples: Arc<[f32]>,
    /// Independently labelled controls and estimates, each from verified bytes.
    pub f0_tracks: Vec<VerifiedF0Track>,
    /// Actual render clock → authored score clock, from the native artifact.
    pub time_map: Option<crate::post::TimeMap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum F0Kind {
    NativeTarget,
    NeuralSupplied,
    NeuralEstimate,
}

impl F0Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::NativeTarget => "合成目标 F0",
            Self::NeuralSupplied => "声码器输入 F0",
            Self::NeuralEstimate => "声码器输出 F0 估计（表达处理前，WORLD）",
        }
    }

    pub fn cache_file(self) -> &'static str {
        match self {
            Self::NativeTarget => "native.npz",
            Self::NeuralSupplied | Self::NeuralEstimate => "neural.npz",
        }
    }
}

#[derive(Clone, Debug)]
pub struct F0Track {
    pub kind: F0Kind,
    /// First actual analysis/control frame center relative to the audio start.
    pub first_frame_seconds: f64,
    pub hop_seconds: f64,
    pub hz: Arc<[f32]>,
}

#[derive(Clone, Debug)]
pub struct VerifiedF0Track {
    pub track: F0Track,
    pub source: std::path::PathBuf,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub digest: String,
    pub job_id: String,
    pub revision: u64,
    pub route: Route,
    pub audio: VerifiedAudio,
    /// Phrase slot this content was last rendered/imported for (ghost lookup).
    pub track: Option<verdant_domain::TrackId>,
    pub phrase_index: Option<usize>,
    /// Timeline placement of the slot at render/import time.
    pub start_seconds: f64,
    pub seconds: f64,
    /// Newest plan revision that wanted this digest.
    pub last_wanted_revision: u64,
}

#[derive(Clone, Debug)]
pub struct Wanted {
    pub digest: String,
    pub payload: Payload,
    pub voice_ref: Option<String>,
    pub phrase: Phrase,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum SlotStatus {
    Ready { job_id: String, rendered_revision: u64 },
    Rendering { job_id: String, display: &'static str, stage: Option<String> },
    Verifying { job_id: String },
    Queued,
    Failed { message: String },
}

#[derive(Clone, Debug)]
pub enum Action {
    Send(Request),
    /// Verify and decode these artifacts, then call `on_verified`.
    Verify {
        job_id: String,
        route: Route,
        wideband: bool,
        artifacts: Vec<Artifact>,
    },
}

#[derive(Default)]
pub struct RenderScheduler {
    pub session_id: Option<String>,
    pub capabilities: Option<Capabilities>,
    pub plan_revision: u64,
    pub route: Option<Route>,
    wanted: Vec<Wanted>,
    jobs: Vec<Job>,
    by_id: HashMap<String, usize>,
    cache: HashMap<String, CacheEntry>,
    failed: HashMap<String, String>,
    verifying: HashMap<String, String>,
    /// Per phrase slot, every content digest a job was submitted for, oldest
    /// first; basis for older-edit ghost lookup.
    slot_history: HashMap<(verdant_domain::TrackId, usize), Vec<String>>,
    counter: u64,
    submit_after: Option<Instant>,
    playhead_seconds: Option<f64>,
    pub anomalies: Vec<String>,
    voice_bindings: HashMap<verdant_domain::TrackId,(String,crate::wire::VoiceEntry)>,
}

impl RenderScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{prefix}{}", self.counter)
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    pub fn job(&self, id: &str) -> Option<&Job> {
        self.by_id.get(id).map(|&i| &self.jobs[i])
    }

    pub fn active(&self) -> Option<&Job> {
        self.active_jobs().next_back()
    }

    /// Every occupied worker slot, including jobs awaiting cancellation.
    pub fn active_jobs(&self) -> impl DoubleEndedIterator<Item = &Job> {
        self.jobs.iter().filter(|j| !j.state.is_terminal())
    }

    /// Focus pending work around the transport without changing wanted order
    /// or render identity: overlapping phrases first, starts within the next
    /// six seconds, endings within the previous three seconds, then everything
    /// else by distance. Within each tier, free tracks are preferred when two
    /// slots are available; equal distances retain wanted order. A non-finite
    /// value restores the default plan order.
    pub fn set_playhead_seconds(&mut self, seconds: f64) {
        self.playhead_seconds = seconds.is_finite().then_some(seconds.max(0.0));
    }

    pub fn wanted(&self) -> &[Wanted] {
        &self.wanted
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// Freeze catalog source identity together with the project snapshot used for planning.
    pub fn set_project_voices(&mut self, project: &verdant_domain::Project) {
        self.bind_project_voices(project, false);
    }

    /// Bind the very same admitted sources used by the desktop planner.
    pub fn set_desktop_project_voices(&mut self, project: &verdant_domain::Project) {
        self.bind_project_voices(project, true);
    }

    fn bind_project_voices(&mut self, project: &verdant_domain::Project, desktop: bool) {
        self.voice_bindings.clear();
        let Some(caps)=&self.capabilities else{return;};
        for track in &project.tracks {
            let selected = if desktop {
                crate::adapter::desktop_voice(project, track, caps)
            } else {
                caps.voice_for_track(project, track)
            };
            if let Ok(Some((reference,voice))) = selected {
                self.voice_bindings.insert(track.id.clone(),(reference.into(),voice.clone()));
            }
        }
    }

    /// Replace the plan for `revision` (single track; see [`set_plans`]).
    /// Like [`Self::set_plans`], this requests immediate rendering.
    pub fn set_plan(&mut self, revision: u64, plan: &PhrasePlan, native_seed: u32) {
        self.set_plans(revision, std::slice::from_ref(plan), native_seed);
    }

    /// Replace the wanted set with every renderable phrase of every track,
    /// in the given priority order. Blocked phrases never enter; phrases
    /// whose content digest is unchanged keep their cached binding.
    /// This immediate entry point is for an explicitly requested render or a
    /// caller that has already debounced planning. Edits use [`Self::schedule_plans`].
    pub fn set_plans(&mut self, revision: u64, plans: &[PhrasePlan], native_seed: u32) {
        self.replace_plans(revision, plans, native_seed);
        self.submit_after = None;
    }

    /// Coalesce edits into the latest plan, including its revision and timeline
    /// placement. Unchanged render content neither restarts the debounce nor
    /// cancels a matching in-flight job; verified content binds immediately.
    /// Poll after this call to cancel superseded work, then wake at
    /// [`Self::next_poll_at`] for the replacement render.
    pub fn schedule_plans(&mut self, revision: u64, plans: &[PhrasePlan], native_seed: u32) {
        self.schedule_plans_at(revision, plans, native_seed, Instant::now());
    }

    /// Clock-injected form of [`Self::schedule_plans`] for deterministic hosts.
    pub fn schedule_plans_at(&mut self, revision: u64, plans: &[PhrasePlan], native_seed: u32, now: Instant) {
        if self.replace_plans(revision, plans, native_seed) {
            self.submit_after = Some(now + RENDER_DEBOUNCE);
        }
    }

    fn replace_plans(&mut self, revision: u64, plans: &[PhrasePlan], native_seed: u32) -> bool {
        self.plan_revision = revision;
        self.route = plans.first().map(|p| p.route);
        let Some(caps) = self.capabilities.as_ref() else {
            let changed = !self.wanted.is_empty();
            self.wanted.clear();
            return changed;
        };
        let voice_caps: HashMap<verdant_domain::TrackId, Capabilities> = self.voice_bindings.iter()
            .map(|(track, (_, voice))| (track.clone(), caps.for_voice(voice)))
            .collect();
        let wanted: Vec<_> = plans
            .iter()
            .flat_map(|plan| plan.phrases.iter().map(move |ph| (plan.route, ph)))
            .filter(|(route, phrase)| *route != Route::C0 || self.voice_bindings.get(&phrase.track)
                .is_some_and(|(reference, voice)| reference == crate::voices::DESKTOP_ORIGINAL_REFERENCE
                    && crate::voices::is_desktop_original(voice)))
            .map(|(route, ph)| {
                let payload = payload_for(ph, route, native_seed);
                let (voice_ref,digest)=if let Some((reference,voice))=self.voice_bindings.get(&ph.track) {
                    let content=render_digest(&payload,route,voice_caps.get(&ph.track).expect("bound voice capability snapshot"));
                    let identity=serde_json::to_vec(&("verdant-render-voice-v1",reference,&voice.cache_identity,&voice.source_sha256,content)).expect("voice identity");
                    (Some(reference.clone()),verdant_domain::digest::sha256_hex(&identity))
                } else {(None,render_digest(&payload,route,caps))};
                Wanted { digest, payload, voice_ref, phrase: ph.clone() }
            })
            .collect();
        let changed = !self.wanted.iter().map(|w| &w.digest).eq(wanted.iter().map(|w| &w.digest));
        self.wanted = wanted;
        changed
    }

    /// Earliest timer wakeup needed to submit pending content into a free slot.
    /// At capacity, terminal events drive polling; cancel acknowledgements do
    /// not release slots. No timer is needed for already cached work.
    pub fn next_poll_at(&self) -> Option<Instant> {
        let deadline = self.submit_after?;
        if self.active_jobs().count() >= self.capacity() || self.session_id.is_none() {
            return None;
        }
        self.wanted.iter().any(|w| self.pending(w)).then_some(deadline)
    }

    /// Status of a track's phrase by its per-track index.
    pub fn phrase_status(&self, track: &verdant_domain::TrackId, index: usize) -> Option<SlotStatus> {
        let i = self.wanted.iter().position(|w| &w.phrase.track == track && w.phrase.index == index)?;
        self.slot_status(i)
    }

    /// Newest verified-but-unwanted cache entry last rendered for this
    /// phrase slot — the older-edit ghost the UI may draw (labelled by
    /// `revision`, e.g. 旧版 rN). `None` when nothing verified was rendered
    /// for a superseded digest of this slot.
    pub fn ghost(&self, track: &verdant_domain::TrackId, phrase_index: usize) -> Option<&CacheEntry> {
        let current = self.wanted.iter().find(|w| &w.phrase.track == track && w.phrase.index == phrase_index).map(|w| &w.digest);
        let history = self.slot_history.get(&(track.clone(), phrase_index))?;
        history
            .iter()
            .rev()
            .filter(|&d| Some(d) != current)
            .find_map(|d| self.cache.get(d))
    }

    /// Every verified cache entry whose content is no longer wanted —
    /// all older-edit ghosts, sorted by timeline position.
    pub fn stale_slots(&self) -> Vec<&CacheEntry> {
        let mut out: Vec<&CacheEntry> = self
            .cache
            .values()
            .filter(|c| !self.wanted.iter().any(|w| w.digest == c.digest))
            .collect();
        out.sort_by(|a, b| a.start_seconds.partial_cmp(&b.start_seconds).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    /// Whether a digest already has a verified binding (pre-restore check).
    pub fn has_digest(&self, digest: &str) -> bool {
        self.cache.contains_key(digest)
    }

    /// Adopt a verified entry restored from the durable cache (or any other
    /// verified source). Binding stays digest-keyed exactly like `on_verified`:
    /// the entry becomes playable only if a wanted phrase asks for this
    /// content; an older edit's entry lands as a ghost instead. Any previous
    /// failure marker for this digest is cleared — verified content wins.
    pub fn import_verified(&mut self, entry: CacheEntry) {
        if let Some(w) = self.wanted.iter().find(|w| w.digest == entry.digest) {
            self.slot_history.entry((w.phrase.track.clone(), w.phrase.index)).or_default().push(entry.digest.clone());
        }
        self.failed.remove(&entry.digest);
        self.cache.insert(entry.digest.clone(), entry);
    }

    /// Forget a failure so the phrase is tried again.
    pub fn retry_failed(&mut self) {
        self.failed.clear();
    }

    pub fn slot_status(&self, index: usize) -> Option<SlotStatus> {
        let w = self.wanted.get(index)?;
        if let Some(c) = self.cache.get(&w.digest) {
            return Some(SlotStatus::Ready { job_id: c.job_id.clone(), rendered_revision: c.revision });
        }
        if let Some(job_id) = self.verifying.get(&w.digest) {
            return Some(SlotStatus::Verifying { job_id: job_id.clone() });
        }
        if let Some(j) = self.jobs.iter().rev().find(|j| j.digest == w.digest && !j.state.is_terminal()) {
            return Some(SlotStatus::Rendering { job_id: j.job_id.clone(), display: j.display_state(), stage: j.stage.clone() });
        }
        if let Some(m) = self.failed.get(&w.digest) {
            return Some(SlotStatus::Failed { message: m.clone() });
        }
        Some(SlotStatus::Queued)
    }

    /// The current playable binding: verified audio for every wanted phrase
    /// that has it, positioned on the timeline.
    pub fn playable(&self) -> Vec<(f64, &CacheEntry)> {
        self.wanted.iter().filter_map(|w| self.cache.get(&w.digest).map(|c| (w.phrase.start_seconds, c))).collect()
    }

    /// Like [`playable`], with the phrase (track, notes) each result binds to.
    pub fn playable_phrases(&self) -> Vec<(&Phrase, &CacheEntry)> {
        self.wanted.iter().filter_map(|w| self.cache.get(&w.digest).map(|c| (&w.phrase, c))).collect()
    }

    pub fn all_ready(&self) -> bool {
        !self.wanted.is_empty() && self.wanted.iter().all(|w| self.cache.contains_key(&w.digest))
    }

    fn capacity(&self) -> usize {
        self.capabilities.as_ref().map_or(0, |caps| caps.max_active_jobs.min(2) as usize)
    }

    fn pending(&self, wanted: &Wanted) -> bool {
        !self.cache.contains_key(&wanted.digest)
            && !self.failed.contains_key(&wanted.digest)
            && !self.verifying.contains_key(&wanted.digest)
            && !self.active_jobs().any(|job| job.digest == wanted.digest)
    }

    fn priority(&self, wanted: &Wanted) -> (u8, f64) {
        let Some(playhead) = self.playhead_seconds else { return (0, 0.0) };
        let start = wanted.phrase.start_seconds;
        let end = start + wanted.phrase.seconds;
        if start <= playhead && playhead < end {
            (0, 0.0)
        } else if start >= playhead {
            let distance = start - playhead;
            (if distance <= 6.0 { 1 } else { 3 }, distance)
        } else {
            let distance = (playhead - end).max(0.0);
            (if distance <= 3.0 { 2 } else { 3 }, distance)
        }
    }

    fn next_pending(&self) -> Option<usize> {
        let spread = self.capacity() > 1;
        let mut occupied_tracks = [None; 2];
        if spread {
            for (track, job) in occupied_tracks.iter_mut().zip(self.active_jobs()) {
                *track = self.wanted.iter().find(|wanted| wanted.digest == job.digest).map(|wanted| &wanted.phrase.track);
            }
        }
        self.wanted.iter().enumerate().filter(|(_, wanted)| self.pending(wanted)).min_by(|(_, a), (_, b)| {
            let (a_tier, a_distance) = self.priority(a);
            let (b_tier, b_distance) = self.priority(b);
            a_tier.cmp(&b_tier)
                .then_with(|| {
                    let a_occupied = occupied_tracks.contains(&Some(&a.phrase.track));
                    let b_occupied = occupied_tracks.contains(&Some(&b.phrase.track));
                    a_occupied.cmp(&b_occupied)
                })
                .then_with(|| a_distance.total_cmp(&b_distance))
        }).map(|(index, _)| index)
    }

    /// Fill advertised worker capacity (at most two desktop jobs) with distinct
    /// digests. Only content absent from the wanted set is superseded.
    pub fn poll(&mut self) -> Vec<Action> {
        self.poll_at(Instant::now())
    }

    /// Always cancel superseded content; when dispatch is disabled, leave new
    /// work pending without registering jobs the caller will not send.
    pub fn poll_pending(&mut self, dispatch: bool) -> Vec<Action> {
        self.poll_pending_at(Instant::now(), dispatch)
    }

    /// Clock-injected form of [`Self::poll`]. Cancellation is immediate even
    /// during debounce; cancelled jobs occupy capacity until terminal events.
    pub fn poll_at(&mut self, now: Instant) -> Vec<Action> {
        self.poll_pending_at(now, true)
    }

    fn poll_pending_at(&mut self, now: Instant, dispatch: bool) -> Vec<Action> {
        let mut out = Vec::new();
        if self.session_id.is_none() || self.capabilities.is_none() {
            return out;
        }
        for i in 0..self.jobs.len() {
            let active = &self.jobs[i];
            if active.state.is_terminal() || active.cancel_requested {
                continue;
            }
            let still_wanted = self.wanted.iter().any(|w| w.digest == active.digest);
            if !still_wanted {
                let job_id = active.job_id.clone();
                if let Some(req) = self.request_cancel(&job_id) {
                    out.push(Action::Send(req));
                }
            }
        }
        if !dispatch || self.submit_after.is_some_and(|deadline| now < deadline) {
            return out;
        }
        self.submit_after = None;
        while self.active_jobs().count() < self.capacity() {
            let Some(index) = self.next_pending() else { break };
            let request_id = self.next_id("render-");
            let w = &self.wanted[index];
            let route = w.phrase.route;
            let track_tag: String = w.phrase.track.as_str().chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect();
            let job_id = format!("v_r{}_{}_p{}_{}_{}", self.plan_revision, track_tag, w.phrase.index, &w.digest[..10], self.counter);
            self.by_id.insert(job_id.clone(), self.jobs.len());
            self.jobs.push(Job {
                job_id: job_id.clone(),
                request_id: request_id.clone(),
                revision: self.plan_revision,
                digest: w.digest.clone(),
                route,
                wideband: w.payload.wideband.is_some(),
                state: JobState::Submitted,
                cancel_requested: false,
                stage: None,
                phase: None,
                error: None,
                artifacts: Vec::new(),
                phrase_index: w.phrase.index,
                start_seconds: w.phrase.start_seconds,
                terminal_events: 0,
            });
            let hist = self.slot_history.entry((w.phrase.track.clone(), w.phrase.index)).or_default();
            if hist.last() != Some(&w.digest) {
                hist.push(w.digest.clone());
            }
            let mut request=Request::render(request_id,job_id,self.plan_revision,route,w.payload.clone());
            request.voice_ref=w.voice_ref.clone();
            out.push(Action::Send(request));
        }
        out
    }

    /// Mark a job `cancel_requested` and produce the cancel request. The job
    /// stays non-terminal until the worker reports its real terminal event.
    pub fn request_cancel(&mut self, job_id: &str) -> Option<Request> {
        let i = *self.by_id.get(job_id)?;
        if self.jobs[i].state.is_terminal() || self.jobs[i].cancel_requested {
            return None;
        }
        self.jobs[i].cancel_requested = true;
        let rid = self.next_id("cancel-");
        Some(Request::cancel(rid, job_id))
    }

    /// Insert a job that exists outside `poll` (contract fixtures, restarts).
    pub fn adopt_job(&mut self, job_id: &str, revision: u64, digest: &str, state: JobState) {
        self.by_id.insert(job_id.to_string(), self.jobs.len());
        self.jobs.push(Job {
            job_id: job_id.into(),
            request_id: String::new(),
            revision,
            digest: digest.into(),
            route: Route::VerdantVoice,
            wideband: false,
            state,
            cancel_requested: false,
            stage: None,
            phase: None,
            error: None,
            artifacts: Vec::new(),
            phrase_index: 0,
            start_seconds: 0.0,
            terminal_events: 0,
        });
    }

    fn terminal(&mut self, job_id: &str, state: JobState) -> Option<usize> {
        let Some(&i) = self.by_id.get(job_id) else {
            self.anomalies.push(format!("terminal {state:?} for unknown job {job_id}"));
            return None;
        };
        self.jobs[i].terminal_events += 1;
        if self.jobs[i].state.is_terminal() {
            self.anomalies.push(format!("second terminal {state:?} for {job_id} ignored"));
            return None;
        }
        self.jobs[i].state = state;
        Some(i)
    }

    pub fn on_event(&mut self, event: WorkerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        match event {
            WorkerEvent::Ready { session_id, .. } => {
                // A new worker session: anything in flight belonged to the old
                // process and can never complete.
                for j in self.jobs.iter_mut().filter(|j| !j.state.is_terminal()) {
                    j.state = JobState::Failed;
                    j.error = Some("worker session ended before this job finished".into());
                }
                self.session_id = Some(session_id);
                self.capabilities = None;
            }
            WorkerEvent::Capabilities { capabilities, .. } => self.capabilities = Some(*capabilities),
            WorkerEvent::Accepted { job_id, revision, .. } => match self.by_id.get(&job_id) {
                Some(&i) if self.jobs[i].state == JobState::Submitted => {
                    if self.jobs[i].revision != revision {
                        self.anomalies.push(format!("accepted {job_id} with revision {revision}, sent {}", self.jobs[i].revision));
                    }
                    self.jobs[i].state = JobState::Accepted;
                }
                _ => self.anomalies.push(format!("unexpected accepted for {job_id}")),
            },
            WorkerEvent::Progress { job_id, stage, phase, .. } => {
                if let Some(&i) = self.by_id.get(&job_id) {
                    let j = &mut self.jobs[i];
                    if !j.state.is_terminal() {
                        j.state = JobState::Running;
                        j.stage = stage;
                        j.phase = phase;
                    }
                }
            }
            WorkerEvent::Completed { job_id, revision, artifacts, .. } => {
                if let Some(i) = self.terminal(&job_id, JobState::Completed) {
                    let j = &mut self.jobs[i];
                    if j.revision != revision {
                        self.anomalies.push(format!("completed {job_id} reports revision {revision}, sent {}", j.revision));
                    }
                    if j.cancel_requested {
                        // Completion can race a cancel request. Do not send
                        // these artifacts to verification or durable storage.
                        return out;
                    }
                    j.artifacts = artifacts.clone();
                    self.verifying.insert(j.digest.clone(), job_id.clone());
                    out.push(Action::Verify { job_id, route: j.route, wideband: j.wideband, artifacts });
                }
            }
            WorkerEvent::Failed { job_id, code, message, .. } => {
                if let Some(i) = self.terminal(&job_id, JobState::Failed) {
                    let msg = format!("{code}: {message}");
                    self.jobs[i].error = Some(msg.clone());
                    self.failed.insert(self.jobs[i].digest.clone(), msg);
                }
            }
            WorkerEvent::Cancelled { job_id, .. } => {
                // Partial outputs are never cached, whatever the flag says.
                self.terminal(&job_id, JobState::Cancelled);
            }
            WorkerEvent::Rejected { job_id: Some(job_id), code, message, .. } if self.by_id.contains_key(&job_id) => {
                let i = self.by_id[&job_id];
                if self.jobs[i].state == JobState::Submitted {
                    self.jobs[i].state = JobState::Rejected;
                    self.jobs[i].terminal_events += 1;
                    self.jobs[i].error = Some(format!("{code}: {message}"));
                    if code != "BUSY" {
                        self.failed.insert(self.jobs[i].digest.clone(), format!("{code}: {message}"));
                    }
                } else if !code.is_empty() {
                    // e.g. a rejected *cancel* for a known job: informational.
                    self.anomalies.push(format!("rejected {code} for {job_id} in state {:?}", self.jobs[i].state));
                }
            }
            WorkerEvent::Rejected { code, message, request_id, .. } => {
                // Request-level rejection without a job: match by request id.
                if let Some(i) = request_id.as_ref().and_then(|r| self.jobs.iter().position(|j| &j.request_id == r)) {
                    if self.jobs[i].state == JobState::Submitted {
                        self.jobs[i].state = JobState::Rejected;
                        self.jobs[i].error = Some(format!("{code}: {message}"));
                        self.failed.insert(self.jobs[i].digest.clone(), format!("{code}: {message}"));
                    }
                } else {
                    self.anomalies.push(format!("rejected {code}: {message}"));
                }
            }
            WorkerEvent::CancelAck { .. } => {
                // Deliberately no state change: wait for the real terminal.
            }
            WorkerEvent::VoiceAdmitted { .. } | WorkerEvent::Shutdown { .. } | WorkerEvent::Unknown { .. } => {}
        }
        out
    }

    pub fn on_verified(&mut self, job_id: &str, result: Result<VerifiedAudio, String>) {
        let Some(&i) = self.by_id.get(job_id) else { return };
        let j = &self.jobs[i];
        if j.state != JobState::Completed || j.cancel_requested || self.verifying.get(&j.digest).map(String::as_str) != Some(job_id) {
            return;
        }
        self.verifying.remove(&j.digest);
        match result {
            Ok(audio) => {
                let wanted = self.wanted.iter().find(|w| w.digest == j.digest);
                let entry = CacheEntry {
                    digest: j.digest.clone(),
                    job_id: j.job_id.clone(),
                    revision: j.revision,
                    route: j.route,
                    audio,
                    track: wanted.map(|w| w.phrase.track.clone()),
                    phrase_index: wanted.map(|w| w.phrase.index),
                    start_seconds: wanted.map(|w| w.phrase.start_seconds).unwrap_or(0.0),
                    seconds: wanted.map(|w| w.phrase.seconds).unwrap_or(0.0),
                    last_wanted_revision: self.plan_revision,
                };
                self.cache.insert(j.digest.clone(), entry);
            }
            Err(e) => {
                let msg = format!("制品校验失败: {e}");
                self.jobs[i].error = Some(msg.clone());
                self.failed.insert(self.jobs[i].digest.clone(), msg);
            }
        }
    }

    /// The worker process ended (EOF, crash). Every unfinished request is
    /// closed as a visible failure (SPEC §12, T12).
    pub fn on_worker_exit(&mut self, reason: &str) {
        for j in self.jobs.iter_mut().filter(|j| !j.state.is_terminal()) {
            j.state = JobState::Failed;
            j.error = Some(format!("worker exited: {reason}"));
            self.failed.insert(j.digest.clone(), format!("后端进程退出：{reason}"));
        }
        self.session_id = None;
        self.capabilities = None;
    }

    /// Summary counts for the status bar.
    pub fn summary(&self) -> BTreeMap<&'static str, usize> {
        let mut m = BTreeMap::new();
        for i in 0..self.wanted.len() {
            let k = match self.slot_status(i) {
                Some(SlotStatus::Ready { .. }) => "ready",
                Some(SlotStatus::Rendering { .. }) | Some(SlotStatus::Verifying { .. }) => "rendering",
                Some(SlotStatus::Failed { .. }) => "failed",
                _ => "queued",
            };
            *m.entry(k).or_default() += 1;
        }
        m
    }
}
