//! Client scheduling against protocol events and a deterministic monotonic clock.
//! PCM is supplied only at the verifier boundary; artifact decoding has its own suite.

use std::sync::Arc;
use std::time::{Duration, Instant};

use verdant_backend::scheduler::{RENDER_DEBOUNCE, VerifiedAudio};
use verdant_backend::wire::{Artifact, Op};
use verdant_backend::{Action, Capabilities, JobState, PhrasePlan, RenderScheduler, Request, Route, SlotStatus, WorkerEvent, plan};
use verdant_domain::{Note, Project, Pronunciation, PronunciationSource, TrackId};

fn scheduler() -> RenderScheduler {
    let caps: Capabilities = serde_json::from_value(serde_json::json!({
        "engine": "scheduler protocol fixture", "protocol": "ogd-worker/1", "native_version": "fixture-1",
        "routes":["verdant-voice"], "test_double":true, "max_active_jobs": 1, "max_phrase_seconds": 30.0,
        "native_score": {"english_vowel_proxies": ["a"]},
        "files": {
            "token": {"available": true, "sha256": "11".repeat(32)},
            "model": {"available": true, "sha256": "22".repeat(32)}
        }
    })).unwrap();
    let mut scheduler = RenderScheduler::new();
    scheduler.on_event(WorkerEvent::Ready { session_id: "scheduler-test".into(), workspace: "/unused".into() });
    scheduler.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(caps) });
    scheduler
}

fn project() -> Project {
    let mut project = Project::new_empty();
    let mut note = Note::new(0, 960, 60);
    note.lyric = "啊".into();
    note.pronunciation = Pronunciation {
        pinyin: "a".into(),
        source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(),
        ..Default::default()
    };
    project.tracks[0].notes = vec![note];
    project
}

fn phrase_plan(scheduler: &RenderScheduler, project: &Project) -> PhrasePlan {
    let plan = plan(project, &project.tracks[0].id, Route::VerdantVoice, scheduler.capabilities.as_ref().unwrap());
    assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
    assert_eq!(plan.phrases.len(), 1);
    plan
}

fn timed_plan(scheduler: &RenderScheduler, track: TrackId, phrases: &[(f64, i32)]) -> PhrasePlan {
    let mut project = project();
    let mut plan = phrase_plan(scheduler, &project);
    plan.track = track.clone();
    plan.phrases = phrases.iter().enumerate().map(|(index, &(start_seconds, midi))| {
        project.tracks[0].notes[0].midi_note = midi;
        let mut phrase = phrase_plan(scheduler, &project).phrases.remove(0);
        phrase.track = track.clone();
        phrase.index = index;
        phrase.start_seconds = start_seconds;
        phrase.seconds = 1.0;
        phrase
    }).collect();
    plan
}

fn requests(actions: Vec<Action>, op: Op) -> Vec<Request> {
    actions.into_iter().map(|action| {
        let Action::Send(request) = action else { panic!("expected wire request") };
        assert_eq!(request.op, op);
        request
    }).collect()
}

fn request(actions: Vec<Action>, op: Op) -> Request {
    assert_eq!(actions.len(), 1, "{actions:?}");
    let Action::Send(request) = actions.into_iter().next().unwrap() else { panic!("expected wire request") };
    assert_eq!(request.op, op);
    request
}

fn completed(request: &Request) -> WorkerEvent {
    WorkerEvent::Completed {
        job_id: request.job_id.clone().unwrap(), revision: request.revision.unwrap(),
        artifacts: vec![Artifact { path: "jobs/test/native.wav".into(), bytes: 48, sha256: "33".repeat(32) }],
        identity_status: None, elapsed_seconds: None,
    }
}

fn verified_audio() -> VerifiedAudio {
    VerifiedAudio {
        path: "unused-verified.wav".into(), sha256: "33".repeat(32), sample_rate: 16000,
        samples: Arc::from([0.0_f32, 0.25]), f0_tracks: Vec::new(), time_map: None,
    }
}

fn verify_completion(scheduler: &mut RenderScheduler, render: &Request) {
    let actions = scheduler.on_event(completed(render));
    assert_eq!(actions.len(), 1, "{actions:?}");
    let Action::Verify { job_id, .. } = &actions[0] else { panic!("completed content needs verification") };
    assert!(scheduler.playable().is_empty(), "unverified artifacts must not bind");
    scheduler.on_verified(job_id, Ok(verified_audio()));
}

#[test]
fn three_rapid_edits_render_only_the_latest_revision() {
    let mut scheduler = scheduler();
    let mut project = project();
    let start = Instant::now();
    for (index, midi) in [60, 62, 64].into_iter().enumerate() {
        let now = start + Duration::from_millis(index as u64 * 40);
        project.tracks[0].notes[0].midi_note = midi;
        let plan = phrase_plan(&scheduler, &project);
        scheduler.schedule_plans_at(index as u64 + 1, &[plan], 10008, now);
        assert!(scheduler.poll_at(now).is_empty());
        assert!(scheduler.jobs().is_empty());
    }
    let deadline = start + Duration::from_millis(80) + RENDER_DEBOUNCE;
    assert_eq!(scheduler.next_poll_at(), Some(deadline));
    assert!(scheduler.poll_at(deadline - Duration::from_millis(1)).is_empty());
    let render = request(scheduler.poll_at(deadline), Op::Render);
    assert_eq!(render.revision, Some(3));
    assert_eq!(render.payload.as_ref(), Some(&scheduler.wanted()[0].payload));
    assert_eq!(scheduler.jobs().len(), 1);
    assert_eq!(scheduler.active().unwrap().revision, 3);
    assert!(scheduler.poll_at(deadline + RENDER_DEBOUNCE).is_empty());
    verify_completion(&mut scheduler, &render);
    assert!(scheduler.all_ready());
    assert_eq!(scheduler.playable()[0].1.revision, 3);
}

#[test]
fn superseded_jobs_cancel_once_and_never_publish_partial_artifacts() {
    // Supersession is safe both before admission arrives and during execution.
    for admitted in [false, true] {
        let mut scheduler = scheduler();
        let mut project = project();
        let start = Instant::now();
        let plan = phrase_plan(&scheduler, &project);
        scheduler.set_plan(1, &plan, 10008);
        let old = request(scheduler.poll_at(start), Op::Render);
        let old_id = old.job_id.as_deref().unwrap();
        let old_digest = scheduler.job(old_id).unwrap().digest.clone();
        if admitted {
            scheduler.on_event(WorkerEvent::Accepted {
                request_id: Some(old.request_id.clone()), job_id: old_id.into(), revision: 1, job_directory: None,
            });
            scheduler.on_event(WorkerEvent::Progress { job_id: old_id.into(), stage: Some("native".into()), phase: None, fraction: None });
        }
        project.tracks[0].notes[0].midi_note = 65;
        let plan = phrase_plan(&scheduler, &project);
        let edit = start + Duration::from_millis(10);
        scheduler.schedule_plans_at(2, &[plan], 10008, edit);
        let cancel = request(scheduler.poll_at(edit), Op::Cancel);
        assert_eq!(cancel.job_id, old.job_id);
        assert!(scheduler.request_cancel(old_id).is_none(), "cancel requests are idempotent");
        assert!(scheduler.poll_at(edit).is_empty());
        if !admitted {
            scheduler.on_event(WorkerEvent::Accepted {
                request_id: Some(old.request_id.clone()), job_id: old_id.into(), revision: 1, job_directory: None,
            });
            assert_eq!(scheduler.job(old_id).unwrap().display_state(), "cancel_requested");
        }
        scheduler.on_event(WorkerEvent::CancelAck {
            request_id: Some(cancel.request_id), job_id: old_id.into(), active_match: true, previous_terminal: None,
        });
        let deadline = edit + RENDER_DEBOUNCE;
        assert!(scheduler.poll_at(deadline).is_empty(), "ack must not release the occupied worker slot");
        assert_eq!(scheduler.jobs().len(), 1);
        scheduler.on_event(WorkerEvent::Cancelled { job_id: old_id.into(), revision: 1, partial_outputs_valid: true });
        assert_eq!(scheduler.job(old_id).unwrap().state, JobState::Cancelled);
        // Neither a stray verifier callback nor a late completed terminal may
        // turn cancelled/partial output into a cache entry.
        scheduler.on_verified(old_id, Ok(verified_audio()));
        assert!(scheduler.on_event(completed(&old)).is_empty());
        assert!(!scheduler.has_digest(&old_digest));
        assert_eq!(scheduler.cache_len(), 0);
        assert!(scheduler.playable().is_empty());
        let latest = request(scheduler.poll_at(deadline), Op::Render);
        assert_eq!(latest.revision, Some(2));
        assert_ne!(latest.job_id, old.job_id);
        verify_completion(&mut scheduler, &latest);
        assert_eq!(scheduler.cache_len(), 1);
        assert!(!scheduler.has_digest(&old_digest));
        assert_eq!(scheduler.playable()[0].1.job_id, latest.job_id.unwrap());
    }
}

#[test]
fn completion_racing_a_requested_cancel_is_not_verified_or_cached() {
    let mut scheduler = scheduler();
    let mut project = project();
    let start = Instant::now();
    scheduler.set_plan(1, &phrase_plan(&scheduler, &project), 10008);
    let old = request(scheduler.poll_at(start), Op::Render);
    let old_id = old.job_id.as_deref().unwrap();
    project.tracks[0].notes[0].midi_note = 65;
    scheduler.schedule_plans_at(2, &[phrase_plan(&scheduler, &project)], 10008, start);
    request(scheduler.poll_at(start), Op::Cancel);
    assert!(scheduler.on_event(completed(&old)).is_empty(), "no Verify action may reach durable storage");
    scheduler.on_verified(old_id, Ok(verified_audio()));
    assert_eq!(scheduler.job(old_id).unwrap().state, JobState::Completed, "retain the worker's first terminal fact");
    assert!(scheduler.job(old_id).unwrap().artifacts.is_empty());
    assert_eq!(scheduler.cache_len(), 0);
    assert!(scheduler.playable().is_empty());
    assert!(scheduler.poll_at(start).is_empty());
    assert_eq!(request(scheduler.poll_at(start + RENDER_DEBOUNCE), Op::Render).revision, Some(2));
}

#[test]
fn unchanged_content_reuses_cache_and_changed_content_at_same_revision_does_not() {
    let mut scheduler = scheduler();
    let mut project = project();
    let original = phrase_plan(&scheduler, &project);
    let start = Instant::now();
    scheduler.set_plan(7, &original, 10008);
    let first = request(scheduler.poll_at(start), Op::Render);
    verify_completion(&mut scheduler, &first);
    let digest = scheduler.wanted()[0].digest.clone();
    scheduler.schedule_plans_at(8, std::slice::from_ref(&original), 10008, start);
    assert_eq!(scheduler.wanted()[0].digest, digest);
    assert!(scheduler.all_ready());
    assert!(scheduler.next_poll_at().is_none());
    assert!(scheduler.poll_at(start).is_empty());
    assert_eq!(scheduler.jobs().len(), 1);
    assert!(matches!(scheduler.slot_status(0), Some(SlotStatus::Ready { rendered_revision: 7, .. })));
    project.tracks[0].notes[0].midi_note = 67;
    let changed = phrase_plan(&scheduler, &project);
    scheduler.schedule_plans_at(8, &[changed], 10008, start);
    assert_ne!(scheduler.wanted()[0].digest, digest);
    assert!(scheduler.playable().is_empty());
    assert!(scheduler.poll_at(start).is_empty());
    let next = request(scheduler.poll_at(start + RENDER_DEBOUNCE), Op::Render);
    assert_eq!(next.revision, Some(8));
    assert_ne!(next.payload, first.payload);
    scheduler.schedule_plans_at(9, &[original], 10008, start + RENDER_DEBOUNCE);
    request(scheduler.poll_at(start + RENDER_DEBOUNCE), Op::Cancel);
    assert!(scheduler.all_ready(), "undo binds the original digest immediately, even during cancellation");
    assert_eq!(scheduler.playable()[0].1.job_id, first.job_id.unwrap());
}

#[test]
fn unchanged_content_does_not_restart_debounce_or_cancel_an_active_render() {
    let mut scheduler = scheduler();
    let project = project();
    let plan = phrase_plan(&scheduler, &project);
    let start = Instant::now();
    scheduler.schedule_plans_at(1, std::slice::from_ref(&plan), 10008, start);
    scheduler.schedule_plans_at(2, std::slice::from_ref(&plan), 10008, start + Duration::from_millis(100));
    assert_eq!(scheduler.next_poll_at(), Some(start + RENDER_DEBOUNCE));
    let render = request(scheduler.poll_at(start + RENDER_DEBOUNCE), Op::Render);
    assert_eq!(render.revision, Some(2));
    scheduler.schedule_plans_at(3, &[plan], 10008, start + RENDER_DEBOUNCE);
    assert!(scheduler.poll_at(start + RENDER_DEBOUNCE).is_empty());
    assert!(!scheduler.active().unwrap().cancel_requested);
    verify_completion(&mut scheduler, &render);
    assert!(scheduler.all_ready());
    let job_id = render.job_id.as_deref().unwrap();
    scheduler.on_verified(job_id, Err("late duplicate verifier callback".into()));
    assert!(scheduler.all_ready(), "duplicate verification cannot revoke a valid cache entry");
    assert!(scheduler.job(job_id).unwrap().error.is_none());
}

#[test]
fn a_multi_phrase_plan_debounces_once_and_dispatches_serially() {
    let mut scheduler = scheduler();
    let mut project = project();
    let mut next = project.tracks[0].notes[0].clone();
    next.id = verdant_domain::NoteId::generate();
    next.start_tick = 3840;
    next.midi_note = 67;
    project.tracks[0].notes.push(next);
    let plan = plan(&project, &project.tracks[0].id, Route::VerdantVoice, scheduler.capabilities.as_ref().unwrap());
    assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
    assert_eq!(plan.phrases.len(), 2);
    let start = Instant::now();
    scheduler.schedule_plans_at(1, &[plan], 10008, start);
    assert!(scheduler.poll_at(start).is_empty());
    let deadline = start + RENDER_DEBOUNCE;
    let first = request(scheduler.poll_at(deadline), Op::Render);
    assert!(scheduler.poll_at(deadline).is_empty());
    assert!(scheduler.next_poll_at().is_none());
    verify_completion(&mut scheduler, &first);
    // Finishing the first phrase releases the slot without imposing a second
    // edit debounce. An accepted event or cancel ack alone never releases it.
    let second = request(scheduler.poll_at(deadline), Op::Render);
    assert_ne!(first.job_id, second.job_id);
    assert_eq!(scheduler.jobs().len(), 2);
    assert_eq!(scheduler.jobs().iter().filter(|job| !job.state.is_terminal()).count(), 1);
    let actions = scheduler.on_event(completed(&second));
    assert!(matches!(actions.as_slice(), [Action::Verify { .. }]));
    scheduler.on_verified(second.job_id.as_deref().unwrap(), Ok(verified_audio()));
    assert!(scheduler.all_ready());
    assert_eq!(scheduler.playable().len(), 2);
}

#[test]
fn overlapping_tracks_fill_two_slots_and_keep_wanted_order() {
    let mut scheduler = scheduler();
    scheduler.capabilities.as_mut().unwrap().max_active_jobs = 2;
    let first = timed_plan(&scheduler, TrackId::generate(), &[(0.0, 60), (4.0, 62)]);
    let second = timed_plan(&scheduler, TrackId::generate(), &[(0.0, 67), (4.0, 69)]);
    scheduler.set_plans(1, &[first, second], 10008);
    let wanted = scheduler.wanted().to_vec();
    scheduler.set_playhead_seconds(0.5);
    let renders = requests(scheduler.poll(), Op::Render);
    assert_eq!(renders.len(), 2);
    assert_eq!(renders[0].payload.as_ref(), Some(&wanted[0].payload));
    assert_eq!(renders[1].payload.as_ref(), Some(&wanted[2].payload));
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert_eq!(scheduler.active().unwrap().job_id, renders[1].job_id.as_deref().unwrap());
    assert!(scheduler.poll().is_empty());
    assert!(scheduler.active_jobs().all(|job| !job.cancel_requested));
    assert!(scheduler.wanted().iter().zip(&wanted).all(|(a, b)| a.digest == b.digest && a.phrase == b.phrase));

    // A terminal completion releases exactly one slot, even while its content
    // is awaiting verification. The other active track keeps its slot.
    assert!(matches!(scheduler.on_event(completed(&renders[0])).as_slice(), [Action::Verify { .. }]));
    let next = request(scheduler.poll(), Op::Render);
    assert_eq!(next.payload.as_ref(), Some(&wanted[1].payload));
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert!(scheduler.poll().is_empty());
}

#[test]
fn default_order_spreads_tracks_and_never_dispatches_duplicate_digests() {
    for advertised in [1, 2, 8] {
        let mut scheduler = scheduler();
        scheduler.capabilities.as_mut().unwrap().max_active_jobs = advertised;
        let first = timed_plan(&scheduler, TrackId::generate(), &[(0.0, 60), (4.0, 62)]);
        let second = timed_plan(&scheduler, TrackId::generate(), &[(0.0, 67)]);
        let duplicate = first.clone();
        scheduler.set_plans(1, &[first, second, duplicate], 10008);
        let wanted = scheduler.wanted().to_vec();
        let renders = requests(scheduler.poll(), Op::Render);
        assert_eq!(renders.len(), advertised.min(2) as usize);
        assert_eq!(renders[0].payload.as_ref(), Some(&wanted[0].payload));
        if advertised > 1 {
            assert_eq!(renders[1].payload.as_ref(), Some(&wanted[2].payload));
            assert_ne!(scheduler.active_jobs().next().unwrap().digest, scheduler.active_jobs().next_back().unwrap().digest);
        }
        assert!(scheduler.poll().is_empty());
        for render in &renders {
            scheduler.on_event(completed(render));
        }
        let remaining = request(scheduler.poll(), Op::Render);
        assert_eq!(remaining.payload.as_ref(), Some(&wanted[1].payload));
        assert!(scheduler.poll().is_empty(), "active and verifying digests must not be submitted twice");
    }
}

#[test]
fn seeks_prioritize_overlap_then_upcoming_then_recent_then_distance_stably() {
    let mut scheduler = scheduler();
    let plan = timed_plan(&scheduler, TrackId::generate(), &[
        (40.0, 60), (7.0, 61), (15.0, 62), (10.0, 63), (20.0, 64),
        (0.0, 65), (8.0, 66), (10.0, 67),
    ]);
    scheduler.set_plan(1, &plan, 10008);
    let wanted = scheduler.wanted().to_vec();
    for expected in [3, 7, 2, 6, 1, 4, 5, 0] {
        scheduler.set_playhead_seconds(10.5);
        let render = request(scheduler.poll(), Op::Render);
        assert_eq!(render.payload.as_ref(), Some(&wanted[expected].payload));
        let actions = scheduler.on_event(completed(&render));
        assert!(matches!(actions.as_slice(), [Action::Verify { .. }]));
        scheduler.on_verified(render.job_id.as_deref().unwrap(), Ok(verified_audio()));
    }
    assert!(scheduler.all_ready());
    let cached_jobs: Vec<_> = scheduler.playable().iter().map(|(_, cache)| cache.job_id.clone()).collect();
    scheduler.set_playhead_seconds(40.5);
    assert!(scheduler.poll().is_empty());
    assert_eq!(scheduler.playable().iter().map(|(_, cache)| cache.job_id.clone()).collect::<Vec<_>>(), cached_jobs);
    assert!(scheduler.wanted().iter().zip(&wanted).all(|(a, b)| a.digest == b.digest && a.phrase == b.phrase));
}

#[test]
fn seek_reorders_pending_work_without_cancelling_current_content() {
    let mut scheduler = scheduler();
    let plan = timed_plan(&scheduler, TrackId::generate(), &[(0.0, 60), (5.0, 62), (20.0, 67)]);
    scheduler.set_plan(1, &plan, 10008);
    let wanted = scheduler.wanted().to_vec();
    let first = request(scheduler.poll(), Op::Render);
    scheduler.set_playhead_seconds(20.5);
    assert!(scheduler.poll().is_empty());
    assert!(!scheduler.active().unwrap().cancel_requested);
    verify_completion(&mut scheduler, &first);
    let next = request(scheduler.poll(), Op::Render);
    assert_eq!(next.payload.as_ref(), Some(&wanted[2].payload));
    assert!(scheduler.has_digest(&wanted[0].digest));
    scheduler.set_playhead_seconds(0.0);
    assert!(scheduler.poll().is_empty());
    assert!(!scheduler.active().unwrap().cancel_requested);
    assert!(scheduler.wanted().iter().zip(&wanted).all(|(a, b)| a.digest == b.digest && a.phrase == b.phrase));
}

#[test]
fn disabled_dispatch_cancels_all_superseded_jobs_without_registering_new_work() {
    let mut scheduler = scheduler();
    scheduler.capabilities.as_mut().unwrap().max_active_jobs = 2;
    let track = TrackId::generate();
    let plan = timed_plan(&scheduler, track.clone(), &[(0.0, 60), (4.0, 62)]);
    scheduler.set_plan(1, &plan, 10008);
    assert!(scheduler.poll_pending(false).is_empty());
    assert!(scheduler.jobs().is_empty());
    let old = requests(scheduler.poll_pending(true), Op::Render);
    assert_eq!(old.len(), 2);
    let replacement = timed_plan(&scheduler, track, &[(0.0, 64), (4.0, 67)]);
    scheduler.set_plan(2, &replacement, 10008);
    let cancels = requests(scheduler.poll_pending(false), Op::Cancel);
    assert_eq!(cancels.len(), 2);
    for (cancel, render) in cancels.iter().zip(&old) {
        assert_eq!(cancel.job_id, render.job_id);
        scheduler.on_event(WorkerEvent::CancelAck {
            request_id: Some(cancel.request_id.clone()), job_id: cancel.job_id.clone().unwrap(),
            active_match: true, previous_terminal: None,
        });
    }
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert_eq!(scheduler.jobs().len(), 2);
    assert!(scheduler.poll_pending(true).is_empty(), "cancel acknowledgements do not release either slot");
    scheduler.on_event(WorkerEvent::Cancelled { job_id: old[0].job_id.clone().unwrap(), revision: 1, partial_outputs_valid: false });
    assert!(scheduler.poll_pending(false).is_empty());
    assert_eq!(scheduler.jobs().len(), 2, "disabled dispatch must not orphan a submitted job");
    let newest = request(scheduler.poll_pending(true), Op::Render);
    assert_eq!(newest.revision, Some(2));
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert!(scheduler.active_jobs().any(|job| job.job_id == old[1].job_id.as_deref().unwrap() && job.cancel_requested));
}

#[test]
fn partial_supersession_retains_the_other_track_and_its_occupied_slot() {
    let mut scheduler = scheduler();
    scheduler.capabilities.as_mut().unwrap().max_active_jobs = 2;
    let first_track = TrackId::generate();
    let second_track = TrackId::generate();
    let first = timed_plan(&scheduler, first_track.clone(), &[(0.0, 60)]);
    let second = timed_plan(&scheduler, second_track, &[(0.0, 67)]);
    scheduler.set_plans(1, &[first, second.clone()], 10008);
    let old = requests(scheduler.poll(), Op::Render);
    assert_eq!(old.len(), 2);
    let changed = timed_plan(&scheduler, first_track, &[(0.0, 62)]);
    let edit = Instant::now();
    scheduler.schedule_plans_at(2, &[changed, second], 10008, edit);
    scheduler.set_playhead_seconds(20.0);
    let cancel = request(scheduler.poll_at(edit), Op::Cancel);
    assert_eq!(cancel.job_id, old[0].job_id);
    assert!(!scheduler.job(old[1].job_id.as_deref().unwrap()).unwrap().cancel_requested);
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert!(scheduler.poll_at(edit + RENDER_DEBOUNCE).is_empty());
    scheduler.on_event(WorkerEvent::Cancelled { job_id: old[0].job_id.clone().unwrap(), revision: 1, partial_outputs_valid: false });
    let replacement = request(scheduler.poll_at(edit + RENDER_DEBOUNCE), Op::Render);
    assert_eq!(replacement.revision, Some(2));
    assert_eq!(replacement.payload.as_ref(), Some(&scheduler.wanted()[0].payload));
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert!(matches!(scheduler.on_event(completed(&old[1])).as_slice(), [Action::Verify { .. }]));
    scheduler.on_verified(old[1].job_id.as_deref().unwrap(), Ok(verified_audio()));
    assert!(scheduler.has_digest(&scheduler.wanted()[1].digest));
}

#[test]
fn a_free_second_slot_still_wakes_at_the_edit_debounce_deadline() {
    let mut scheduler = scheduler();
    scheduler.capabilities.as_mut().unwrap().max_active_jobs = 2;
    let track = TrackId::generate();
    let original = timed_plan(&scheduler, track.clone(), &[(0.0, 60)]);
    scheduler.set_plan(1, &original, 10008);
    request(scheduler.poll(), Op::Render);
    let expanded = timed_plan(&scheduler, track, &[(0.0, 60), (4.0, 62)]);
    let start = Instant::now();
    scheduler.schedule_plans_at(2, &[expanded], 10008, start);
    assert!(scheduler.poll_at(start).is_empty());
    assert_eq!(scheduler.next_poll_at(), Some(start + RENDER_DEBOUNCE));
    let next = request(scheduler.poll_at(start + RENDER_DEBOUNCE), Op::Render);
    assert_eq!(next.revision, Some(2));
    assert_eq!(scheduler.active_jobs().count(), 2);
    assert!(scheduler.active_jobs().all(|job| !job.cancel_requested));
}
