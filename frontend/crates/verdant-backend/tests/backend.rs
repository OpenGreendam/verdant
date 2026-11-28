//! Backend tier tests:
//! * T08/T09/T10 from `spec/examples/contract-traces.json` (state policy).
//! * Real artifact decoding/verification against generic synthetic PCM WAV and
//!   NPZ arrays generated in temporary directories (voice and original WAV selection).
//! * T12 failure modes (path escape, hash mismatch, worker crash).
//! * The real `backend/worker.py` handshake in this checkout (no model files
//!   present → capabilities say so and render is refused before running).
//! * The full render → verify → bind loop against the protocol test double.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;
use verdant_backend::artifact::{decode_wav, resolve_inside, verify_and_decode};
use verdant_backend::wire::{Artifact, Capabilities, parse_event_value};
use verdant_backend::{Action, JobState, RenderScheduler, Route, SlotStatus, WorkerClient, WorkerConfig, WorkerEvent, WorkerMessage, plan};
use verdant_domain::model::{Note, PitchPoint, Project};
use verdant_domain::digest::sha256_hex;
use verdant_domain::{NoteId, PointId, Pronunciation, PronunciationSource};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const TRACES: &str = include_str!("../../../../spec/examples/contract-traces.json");

fn case(id: &str) -> Value {
    let v: Value = serde_json::from_str(TRACES).unwrap();
    v["cases"].as_array().unwrap().iter().find(|c| c["id"] == id).cloned().unwrap()
}

fn fixture_capabilities() -> Capabilities { // Synthetic protocol metadata, not a claim that model files are present.
// No default test depends on the private validation workspace.
serde_json::from_value(serde_json::json!({
    "engine": "synthetic backend test capabilities", "protocol": "ogd-worker/1", "native_version": "fixture-1",
    "routes":["verdant-voice"], "test_double":true, "max_active_jobs": 1, "max_phrase_seconds": 30.0,
    "native_score": {
        "inventory": verdant_backend::pinyin::recovered_syllables(),
        "english_vowel_proxies": ["a","i","u","e","o"], "continuous_f0_control": false,
        "coarticulation_recovered": false, "sub_syllable_timing": true,
        "phoneme_duration_control": true
    },
    
    "candidate_voice": {"name":"sn", "display_name":"童声01（楠楠）", "vid":10008, "historical_identity_confirmed":true},
    "files": {
        "token":{"available":true,"sha256":"11".repeat(32)},
        "model":{"available":true,"sha256":"22".repeat(32)},
        "vocoder":{"available":true,"sha256":"33".repeat(32)}
    }
})).unwrap() }

struct SyntheticArtifacts {
    workspace: PathBuf,
    artifacts: Vec<Artifact>,
}

impl SyntheticArtifacts {
    fn new(compression: CompressionMethod) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let workspace = std::env::temp_dir().join(format!(
            "verdant-synthetic-artifacts-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(workspace.join("jobs/synthetic")).unwrap();
        let native = synthetic_wav(16_000, 5600);
        let neural = synthetic_wav(44_100, 15_435);
        let times: Vec<f64> = (0..70).map(|index| index as f64 * 0.005).collect();
        let f0: Vec<f64> = (0..70).map(|index| if index % 7 == 0 { 0.0 } else { 220.0 + (index % 5) as f64 * 55.0 }).collect();
        let npz = synthetic_npz(&f0, &times, compression);
        let mut artifacts = Vec::new();
        for (path, bytes) in [
            ("jobs/synthetic/verdant-voice.wav", native),
            ("jobs/synthetic/native.wav", neural),
            ("jobs/synthetic/native.npz", npz),
        ] {
            std::fs::write(workspace.join(path), &bytes).unwrap();
            artifacts.push(Artifact { path: path.into(), bytes: bytes.len() as u64, sha256: sha256_hex(&bytes) });
        }
        Self { workspace, artifacts }
    }

    fn verify(&self, route: Route) -> verdant_backend::VerifiedAudio {
        verify_and_decode(&self.workspace, route, &self.artifacts).unwrap()
    }
}

impl Drop for SyntheticArtifacts {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.workspace);
    }
}

/// Generic signed PCM samples: exact fractions, no model-derived waveform.
fn synthetic_wav(rate: u32, frames: usize) -> Vec<u8> {
    let data_size = (frames * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    let pattern = [0i16, 8192, -8192, 16384, -16384];
    for index in 0..frames {
        bytes.extend_from_slice(&pattern[index % pattern.len()].to_le_bytes());
    }
    bytes
}

fn synthetic_npy(values: &[f64]) -> Vec<u8> {
    let mut header = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': ({},), }}", values.len());
    let padding = (64 - (10 + header.len() + 1) % 64) % 64;
    header.push_str(&" ".repeat(padding));
    header.push('\n');
    let mut bytes = b"\x93NUMPY\x01\x00".to_vec();
    bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
    bytes.extend_from_slice(header.as_bytes());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn synthetic_npz(f0: &[f64], times: &[f64], compression: CompressionMethod) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(compression);
    for (name, values) in [("f0_hz.npy", f0), ("frame_times_seconds.npy", times)] {
        zip.start_file(name, options).unwrap();
        zip.write_all(&synthetic_npy(values)).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn sung(start: i64, dur: i64, midi: i32, lyric: &str, py: &str) -> Note {
    let mut n = Note::new(start, dur, midi);
    n.lyric = lyric.into();
    n.pronunciation = Pronunciation {
        pinyin: py.into(),
        source: PronunciationSource::Manual,
        frontend_profile: "english-vowel-proxy-native-0.2".into(),
        ..Default::default()
    };
    n
}

fn project_with(notes: Vec<Note>) -> Project {
    let mut p = Project::new_empty();
    p.tracks[0].notes = notes;
    p
}

#[test]
fn t08_stale_completion_caches_but_does_not_bind() {
    let c = case("T08");
    let mut s = RenderScheduler::new();
    s.on_event(WorkerEvent::Ready { session_id: "s".into(), workspace: "/w".into() });
    s.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(fixture_capabilities()) });
    let pending = &c["initial"]["pending"];
    s.adopt_job(pending["job_id"].as_str().unwrap(), pending["revision"].as_u64().unwrap(), "digest-of-r8-content", JobState::Running);
    // The project is now at r9 and wants different content.
    let p = project_with(vec![sung(0, 960, 62, "啊", "a")]);
    let plan9 = plan(&p, &p.tracks[0].id.clone(), Route::VerdantVoice, s.capabilities.as_ref().unwrap());
    s.set_plan(c["initial"]["project_revision"].as_u64().unwrap(), &plan9, 10008);
    for e in c["events"].as_array().unwrap() {
        let ev = parse_event_value(e["record"].clone()).unwrap();
        for a in s.on_event(ev) {
            if let Action::Verify { job_id, .. } = a {
                // Empty-artifact record: verification must fail, never bind.
                s.on_verified(&job_id, Err("no playable artifact".into()));
            }
        }
    }
    let a = &c["assertions"];
    assert!(a["may_record_history"].as_bool().unwrap());
    assert_eq!(s.job("job_old").unwrap().state, JobState::Completed, "history recorded");
    assert!(a["playable_binding"].is_null() && s.playable().is_empty());
    assert_eq!(s.plan_revision, a["project_revision"].as_u64().unwrap());
}

#[test]
fn t09_cancel_ack_is_not_terminal() {
    let c = case("T09");
    let mut s = RenderScheduler::new();
    s.adopt_job("job1", 8, "d", JobState::Running);
    let mut state_after_ack = String::new();
    for e in c["events"].as_array().unwrap() {
        match e["type"].as_str().unwrap() {
            "CancelRequested" => {
                assert!(s.request_cancel("job1").is_some());
            }
            "BackendEvent" => {
                let ev = parse_event_value(e["record"].clone()).unwrap();
                let is_ack = matches!(ev, WorkerEvent::CancelAck { .. });
                s.on_event(ev);
                if is_ack {
                    state_after_ack = s.job("job1").unwrap().display_state().into();
                }
            }
            _ => unreachable!(),
        }
    }
    // A late duplicate terminal must be ignored, not double counted as state.
    s.on_event(WorkerEvent::Completed {
        job_id: "job1".into(),
        revision: 8,
        artifacts: vec![],
        identity_status: None,
        elapsed_seconds: None,
    });
    let a = &c["assertions"];
    assert_eq!(state_after_ack, a["state_after_ack"].as_str().unwrap());
    assert_eq!(s.job("job1").unwrap().display_state(), a["final_state"].as_str().unwrap());
    assert_eq!(s.job("job1").unwrap().state, JobState::Cancelled);
    assert_eq!(s.anomalies.len(), 1, "second terminal recorded as anomaly");
    assert!(!a["load_partial_audio"].as_bool().unwrap() && s.playable().is_empty());
}

#[test]
fn t10_continuous_pitch_fails_preflight_with_location() {
    let c = case("T10");
    let mut n = sung(0, 960, 60, "啊", "a");
    for pt in c["initial"]["pitch_curve"].as_array().unwrap() {
        n.curves.pitch_millicent_points.push(PitchPoint {
            id: PointId::generate(),
            tick: pt["tick"].as_i64().unwrap(),
            millicents: pt["millicents"].as_i64().unwrap(),
            unknown: Default::default(),
        });
    }
    let id = n.id.clone();
    let p = project_with(vec![n]);
    let caps = fixture_capabilities();
    assert!(!caps.native_score.as_ref().unwrap().continuous_f0_control);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(pl.phrases.is_empty(), "backend_request_count = 0");
    let d = pl.errors_for(&id).find(|d| d.code == c["assertions"]["diagnostic"].as_str().unwrap()).unwrap();
    assert!(d.message.contains("音高弯曲"), "{}", d.message);
}

#[test]
fn preflight_refuses_unset_and_invalid_syllables() {
    let mut unset = Note::new(0, 480, 60);
    unset.lyric.clear();
    let wo = sung(480, 480, 62, "无效", "shong3");
    let ok = sung(2400, 480, 64, "啊", "a1");
    let (u, f) = (unset.id.clone(), wo.id.clone());
    let p = project_with(vec![unset, wo, ok]);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &fixture_capabilities());
    assert_eq!(pl.errors_for(&u).next().unwrap().code, "E_PRONUNCIATION_UNSET");
    assert_eq!(pl.errors_for(&f).next().unwrap().code, "E_UNSUPPORTED_SYLLABLE");
    assert_eq!(pl.phrases.len(), 1, "the recovered-inventory note still renders");
}

#[test]
fn phrases_split_on_rests_with_absolute_timing() {
    // 120 BPM: 480 ticks = 0.25 s. A 0.5 s rest stays inside the phrase as
    // gap_after_seconds; a 0.75 s rest (≥ PHRASE_BREAK_SECONDS) splits.
    let p = project_with(vec![
        sung(0, 480, 60, "啊", "a"),
        sung(480, 480, 62, "咿", "i"),
        sung(1920, 480, 64, "呜", "u"),
        sung(3840, 960, 67, "哦", "o"),
    ]);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &fixture_capabilities());
    assert!(!pl.has_errors(), "{:?}", pl.diagnostics);
    assert_eq!(pl.phrases.len(), 2);
    let a = &pl.phrases[0];
    assert_eq!(a.score.segments.len(), 3);
    assert_eq!(a.score.segments[0].duration_seconds, 0.25);
    assert_eq!(a.score.segments[0].vowel.as_deref(), Some("a"));
    assert!((a.score.segments[1].f0_hz.unwrap() - 293.665).abs() < 0.001);
    assert_eq!(a.score.segments[1].gap_after_seconds, 0.5);
    assert_eq!(a.seconds, 1.25);
    let b = &pl.phrases[1];
    assert_eq!(b.start_seconds, 2.0);
    assert_eq!(b.score.segments[0].duration_seconds, 0.5);
    // Exact wire shape accepted by the real worker schema subset.
    let payload = serde_json::to_value(verdant_backend::adapter::payload_for(a, Route::VerdantVoice, 10008)).unwrap();
    assert_eq!(payload["kind"], "native_score");
    assert!(payload.get("shifts").is_none(), "C0 must not send a shift matrix");
}

#[test]
fn synthetic_worker_artifacts_verify_and_decode_for_both_routes() {
    for compression in [CompressionMethod::Stored, CompressionMethod::Deflated] {
        let fixture = SyntheticArtifacts::new(compression);
        for (route, rate, frames) in [(Route::VerdantVoice, 16_000, 5600), (Route::C0, 44_100, 15_435)] {
            let audio = fixture.verify(route);
            assert_eq!(audio.sample_rate, rate);
            assert_eq!(audio.samples.len(), frames);
            assert_eq!(&audio.samples[..5], &[0.0, 0.25, -0.25, 0.5, -0.5]);
            assert!(audio.samples.iter().all(|sample| sample.is_finite()));
            let f0 = &audio.f0_tracks.first().expect("published synthetic native.npz must decode").track;
            assert_eq!(f0.hz.len(), 70);
            assert!((f0.hop_seconds - 0.005).abs() < 1e-9);
            assert_eq!(f0.hz[0], 0.0);
            assert_eq!(f0.hz[1], 275.0);
            assert_eq!(f0.hz[7], 0.0);
            assert!(f0.hz.iter().all(|value| value.is_finite()));
        }
    }
}

#[test]
fn t12_rejects_escape_and_hash_mismatch() {
    let fixture = SyntheticArtifacts::new(CompressionMethod::Stored);
    assert!(resolve_inside(&fixture.workspace, "../worker.stderr.log").is_err());
    assert!(resolve_inside(&fixture.workspace, "/etc/passwd").is_err());
    let mut bad = fixture.artifacts[0].clone();
    bad.sha256 = "00".into();
    assert!(verify_and_decode(&fixture.workspace, Route::VerdantVoice, std::slice::from_ref(&bad)).unwrap_err().contains("SHA-256"));
    bad.bytes = 1;
    assert!(verify_and_decode(&fixture.workspace, Route::VerdantVoice, &[bad]).unwrap_err().contains("bytes"));
    assert!(decode_wav(b"RIFF\0\0\0\0WAVEjunk").is_err());
}

#[test]
fn published_wav_and_npz_corruption_fails_for_both_routes() {
    for route in [Route::VerdantVoice, Route::C0] {
        for corrupt_npz in [false, true] {
            let fixture = SyntheticArtifacts::new(CompressionMethod::Deflated);
            let artifact = if corrupt_npz { &fixture.artifacts[2] } else if route == Route::VerdantVoice { &fixture.artifacts[0] } else { &fixture.artifacts[1] };
            let path = fixture.workspace.join(&artifact.path);
            let mut bytes = std::fs::read(&path).unwrap();
            bytes[0] ^= 1;
            std::fs::write(&path, bytes).unwrap();
            assert!(verify_and_decode(&fixture.workspace, route, &fixture.artifacts).is_err(), "{route:?}: corrupted {} was accepted", artifact.path);
        }
        // Matching size/hash is insufficient when the actual NPZ cannot decode.
        let mut fixture = SyntheticArtifacts::new(CompressionMethod::Stored);
        let bad = b"not a numpy archive";
        let npz = &mut fixture.artifacts[2];
        std::fs::write(fixture.workspace.join(&npz.path), bad).unwrap();
        npz.bytes = bad.len() as u64;
        npz.sha256 = sha256_hex(bad);
        assert!(verify_and_decode(&fixture.workspace, route, &fixture.artifacts).is_err());

        // Omission is distinct from a published-but-corrupt optional F0 track.
        let mut fixture = SyntheticArtifacts::new(CompressionMethod::Stored);
        fixture.artifacts.retain(|artifact| !artifact.path.ends_with("/native.npz"));
        assert!(fixture.verify(route).f0_tracks.is_empty());
    }
}

// ---- process-level tests -------------------------------------------------------------

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("verdant-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn next(rx: &async_channel::Receiver<(u64, WorkerMessage)>, timeout: Duration) -> WorkerMessage {
    let deadline = Instant::now() + timeout;
    loop {
        match rx.try_recv() {
            Ok((_, m)) => return m,
            Err(async_channel::TryRecvError::Empty) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => panic!("no worker message: {e:?}"),
        }
    }
}

fn wait_for(rx: &async_channel::Receiver<(u64, WorkerMessage)>, mut pred: impl FnMut(&WorkerMessage) -> bool) -> Vec<WorkerMessage> {
    let mut seen = Vec::new();
    loop {
        let m = next(rx, Duration::from_secs(60));
        let done = pred(&m);
        seen.push(m);
        if done {
            return seen;
        }
    }
}

#[test]
fn real_rust_worker_handshake_without_models() {
    let ws = scratch("realworker");
    let binary = std::env::var_os("VERDANT_WORKER_BINARY").map(PathBuf::from).unwrap_or_else(|| {
        std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("verdant-worker")
    });
    let cfg = WorkerConfig::native_worker(binary, &ws, &[]);
    let (client, rx) = WorkerClient::spawn(cfg, 1).unwrap();
    let WorkerMessage::Event(WorkerEvent::Ready { session_id, workspace }) = next(&rx, Duration::from_secs(20)) else {
        panic!("ready first")
    };
    assert!(!session_id.is_empty() && workspace.ends_with(ws.file_name().unwrap().to_str().unwrap()));
    client.send(&verdant_backend::Request::capabilities("cap")).unwrap();
    let WorkerMessage::Event(WorkerEvent::Capabilities { capabilities, .. }) = next(&rx, Duration::from_secs(20)) else { panic!() };
    assert_eq!(capabilities.protocol, "ogd-worker/1");
    assert!(!capabilities.test_double);
    // This checkout ships no model files: the editor must see that.
    assert!(capabilities.missing_files(Route::VerdantVoice).contains(&"verdant_voice_manifest"));
    // And the preflight blocks rendering before any request is sent.
    let p = project_with(vec![sung(0, 960, 62, "啊", "a")]);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &capabilities);
    assert!(pl.diagnostics.iter().any(|d| d.code == "E_BACKEND_FILES"));
    // The worker itself also refuses (defence in depth), as a rejection.
    let payload = verdant_backend::adapter::payload_for(
        &plan(&p, &p.tracks[0].id, Route::VerdantVoice, &Capabilities { test_double: true, ..(*capabilities).clone() }).phrases[0],
        Route::VerdantVoice,
        10008,
    );
    client.send(&verdant_backend::Request::render("r1", "real_probe_1", 1, Route::VerdantVoice, payload)).unwrap();
    match next(&rx, Duration::from_secs(20)) {
        WorkerMessage::Event(WorkerEvent::Rejected { code, message, .. }) => {
            assert_eq!(code, "INVALID_REQUEST");
            assert!(!message.is_empty());
        }
        m => panic!("{m:?}"),
    }
    client.send(&verdant_backend::Request::cancel("c1", "never_started")).unwrap();
    match next(&rx, Duration::from_secs(20)) {
        WorkerMessage::Event(WorkerEvent::CancelAck { active_match, previous_terminal, .. }) => {
            assert!(!active_match && previous_terminal.is_none());
        }
        m => panic!("{m:?}"),
    }
    client.send(&verdant_backend::Request::shutdown("bye")).unwrap();
    wait_for(&rx, |m| matches!(m, WorkerMessage::Exited { .. }));
    let _ = std::fs::remove_dir_all(ws);
}

fn drive(s: &mut RenderScheduler, client: &WorkerClient, ws: &Path, m: WorkerMessage) {
    match m {
        WorkerMessage::Event(ev) => {
            for a in s.on_event(ev) {
                if let Action::Verify { job_id, route, wideband, artifacts } = a {
                    let r = verdant_backend::artifact::verify_and_decode_with_options(ws, route, wideband, &artifacts);
                    s.on_verified(&job_id, r);
                }
            }
        }
        WorkerMessage::Exited { status } => s.on_worker_exit(&status),
        WorkerMessage::Malformed { .. } => {}
    }
    for a in s.poll() {
        if let Action::Send(req) = a {
            client.send(&req).unwrap();
        }
    }
}

fn test_double(ws: &Path, extra: &[&str]) -> (WorkerClient, async_channel::Receiver<(u64, WorkerMessage)>) {
    let binary = std::env::var_os("VERDANT_TEST_WORKER_BINARY").map(PathBuf::from).unwrap_or_else(|| {
        std::env::current_exe().unwrap().parent().unwrap().parent().unwrap()
            .join(format!("verdant-test-worker{}", std::env::consts::EXE_SUFFIX))
    });
    let mut cfg = WorkerConfig::native_worker(binary, ws, extra);
    cfg.label = "explicit Rust protocol fixture".into();
    WorkerClient::spawn(cfg, 7).unwrap()
}

fn connect(s: &mut RenderScheduler, client: &WorkerClient, rx: &async_channel::Receiver<(u64, WorkerMessage)>, ws: &Path) {
    let ready = next(rx, Duration::from_secs(20));
    drive(s, client, ws, ready);
    client.send(&verdant_backend::Request::capabilities("cap")).unwrap();
    let caps = next(rx, Duration::from_secs(20));
    drive(s, client, ws, caps);
    assert!(s.capabilities.as_ref().unwrap().test_double);
}

#[test]
fn full_loop_render_verify_bind_with_test_double() {
    let ws = scratch("loop");
    let (client, rx) = test_double(&ws, &[]);
    let mut s = RenderScheduler::new();
    connect(&mut s, &client, &rx, &ws);
    for route in [Route::VerdantVoice] {
        let p = project_with(vec![sung(0, 480, 60, "啊", "a"), sung(480, 480, 64, "咿", "i"), sung(3840, 480, 67, "呜", "u")]);
        let pl = plan(&p, &p.tracks[0].id, route, s.capabilities.as_ref().unwrap());
        assert_eq!(pl.phrases.len(), 2);
        s.set_plan(8, &pl, 10008);
        for a in s.poll() {
            if let Action::Send(r) = a {
                client.send(&r).unwrap();
            }
        }
        let deadline = Instant::now() + Duration::from_secs(90);
        while !s.all_ready() {
            assert!(Instant::now() < deadline, "timed out: {:?}", s.summary());
            let m = next(&rx, Duration::from_secs(60));
            drive(&mut s, &client, &ws, m);
            assert!((0..2).all(|i| !matches!(s.slot_status(i), Some(SlotStatus::Failed { .. }))), "{:?}", s.jobs());
        }
        let playable = s.playable();
        assert_eq!(playable.len(), 2);
        assert_eq!(playable[1].0, 2.0, "second phrase starts at its timeline position");
        let expected_rate = 48_000;
        assert!(playable.iter().all(|(_, c)| c.audio.sample_rate == expected_rate && c.revision == 8));
        assert!(!playable[0].1.audio.f0_tracks.is_empty());
    }
    client.send(&verdant_backend::Request::shutdown("bye")).unwrap();
    wait_for(&rx, |m| matches!(m, WorkerMessage::Exited { .. }));
    let _ = std::fs::remove_dir_all(ws);
}



#[test]
fn edit_during_render_cancels_and_never_binds_old_content() {
    let ws = scratch("stale");
    let (client, rx) = test_double(&ws, &["--delay", "1.5"]);
    let mut s = RenderScheduler::new();
    connect(&mut s, &client, &rx, &ws);
    let caps = s.capabilities.clone().unwrap();
    let old = project_with(vec![sung(0, 960, 60, "啊", "a")]);
    s.set_plan(1, &plan(&old, &old.tracks[0].id, Route::VerdantVoice, &caps), 10008);
    for a in s.poll() {
        if let Action::Send(r) = a {
            client.send(&r).unwrap();
        }
    }
    // Wait until the job is really running, then edit (r2).
    loop {
        let m = next(&rx, Duration::from_secs(20));
        let started = matches!(&m, WorkerMessage::Event(WorkerEvent::Progress { phase: Some(p), .. }) if p == "process_started");
        drive(&mut s, &client, &ws, m);
        if started {
            break;
        }
    }
    let old_job = s.active().unwrap().job_id.clone();
    let new = project_with(vec![sung(0, 960, 65, "啊", "a")]);
    s.set_plan(2, &plan(&new, &new.tracks[0].id, Route::VerdantVoice, &caps), 10008);
    for a in s.poll() {
        if let Action::Send(r) = a {
            client.send(&r).unwrap();
        }
    }
    assert_eq!(s.job(&old_job).unwrap().display_state(), "cancel_requested");
    let deadline = Instant::now() + Duration::from_secs(60);
    while !s.all_ready() {
        assert!(Instant::now() < deadline);
        let m = next(&rx, Duration::from_secs(60));
        drive(&mut s, &client, &ws, m);
    }
    let old = s.job(&old_job).unwrap();
    assert!(matches!(old.state, JobState::Cancelled | JobState::Completed), "{:?}", old.state);
    assert_eq!(old.terminal_events, 1);
    let bound = s.playable();
    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0].1.revision, 2, "only the current content is bound");
    client.send(&verdant_backend::Request::shutdown("bye")).unwrap();
    wait_for(&rx, |m| matches!(m, WorkerMessage::Exited { .. }));
    let _ = std::fs::remove_dir_all(ws);
}

#[test]
fn t12_worker_crash_and_corrupt_artifact_fail_visibly() {
    for (flag, expect) in [("--crash-after-accept", "后端进程退出"), ("--corrupt-artifact", "制品校验失败")] {
        let ws = scratch(&format!("t12{flag}"));
        let (client, rx) = test_double(&ws, &[flag]);
        let mut s = RenderScheduler::new();
        connect(&mut s, &client, &rx, &ws);
        let caps = s.capabilities.clone().unwrap();
        let p = project_with(vec![sung(0, 480, 60, "啊", "a")]);
        s.set_plan(3, &plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps), 10008);
        for a in s.poll() {
            if let Action::Send(r) = a {
                client.send(&r).unwrap();
            }
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            assert!(Instant::now() < deadline);
            if let Some(SlotStatus::Failed { message }) = s.slot_status(0) {
                assert!(message.contains(expect), "{message}");
                break;
            }
            let m = next(&rx, Duration::from_secs(60));
            drive(&mut s, &client, &ws, m);
        }
        assert!(s.playable().is_empty(), "nothing invalid is ever playable");
        drop(client);
        let _ = std::fs::remove_dir_all(ws);
    }
}

#[test]
fn ids_used_in_tests_are_opaque() {
    // Guard: fixture alias IDs and UUIDs are both plain opaque strings.
    assert_eq!(NoteId::from("n1").as_str(), "n1");
}

// ---- parameters: PIT / vibrato / portamento / DYN, multi-track -----------------

use verdant_backend::f0::{self, ControlTrack};
use verdant_backend::params::{self, Param, Support};
use verdant_domain::model::GainPoint;
use verdant_domain::{TempoMap, Track, Vibrato};

fn continuous_caps() -> Capabilities {
    let mut caps = fixture_capabilities();
    caps.native_score.as_mut().unwrap().continuous_f0_control = true;
    caps
}

#[test]
fn parameter_matrix_is_capability_driven() {
    let caps = continuous_caps();
    assert!(matches!(params::support(Param::Pit, Route::VerdantVoice, Some(&fixture_capabilities())), Support::Unsupported(_)));
    assert!(matches!(params::support(Param::Pit, Route::VerdantVoice, Some(&caps)), Support::Synthesis(_)));
    assert!(matches!(params::support(Param::Vibrato, Route::VerdantVoice, Some(&caps)), Support::Synthesis(_)));
    assert!(matches!(params::support(Param::Dyn, Route::VerdantVoice, Some(&caps)), Support::PostProcess(_)));
    for p in [Param::Breathiness, Param::Brightness, Param::Clearness, Param::Gender, Param::Opening] {
        assert!(!params::support(p, Route::VerdantVoice, Some(&caps)).is_effective(), "{p:?} has no backend mapping");
    }
    let mut no_ctl = caps.clone();
    no_ctl.native_score.as_mut().unwrap().continuous_f0_control = false;
    assert!(!params::support(Param::Pit, Route::VerdantVoice, Some(&no_ctl)).is_effective());
}

#[test]
fn control_track_follows_pit_vibrato_portamento_and_passes_worker_checks() {
    let tempo = TempoMap::constant(120.0);
    let mut a = sung(0, 960, 60, "啊", "a"); // 0.5 s at C4
    let mut b = sung(960, 960, 64, "咿", "i"); // legato into E4
    b.portamento_ms = Some(100);
    b.vibrato = Some(Vibrato { depth_cents: 50, rate_millihertz: 5000, delay_permille: 500, unknown: Default::default() });
    a.curves.pitch_millicent_points = vec![
        verdant_domain::model::PitchPoint { id: "p0".into(), tick: 0, millicents: -100_000, unknown: Default::default() },
        verdant_domain::model::PitchPoint { id: "p1".into(), tick: 240, millicents: 0, unknown: Default::default() },
    ];
    let notes = vec![&a, &b];
    let t: ControlTrack = f0::control_track(&notes, &tempo, 0.0, 1.0);
    f0::validate(&t).unwrap();
    assert_eq!(t.frame_times_seconds.len(), 201);
    let hz = |s: f64| t.f0_hz[(s / f0::HOP_SECONDS).round() as usize];
    // PIT scoops from a semitone below C4 up to C4 by tick 240 (0.125 s).
    assert!((hz(0.0) - verdant_domain::timing::midi_to_hz(59, 0)).abs() < 0.5);
    assert!((hz(0.2) - verdant_domain::timing::midi_to_hz(60, 0)).abs() < 0.5);
    // Portamento: just after the boundary the pitch is still near C4.
    assert!(hz(0.505) < verdant_domain::timing::midi_to_hz(62, 0));
    assert!((hz(0.68) - verdant_domain::timing::midi_to_hz(64, 0)).abs() < 1.0, "glide complete after 100 ms");
    // Vibrato starts at half the note and swings ±50 cents.
    let late: Vec<f64> = (150..200).map(|i| t.f0_hz[i]).collect();
    let (lo, hi) = late.iter().fold((f64::MAX, 0.0f64), |(l, h), &v| (l.min(v), h.max(v)));
    let e4 = verdant_domain::timing::midi_to_hz(64, 0);
    assert!(hi > e4 * 1.02 && lo < e4 * 0.98, "vibrato amplitude visible: {lo}..{hi}");
}

#[test]
fn voice_f0_modulation_requires_continuous_control() {
    let mut n = sung(0, 960, 62, "啊", "a");
    n.vibrato = Some(Vibrato::DEFAULT);
    let id = n.id.clone();
    let p = project_with(vec![n, sung(3840, 480, 64, "咿", "i")]);
    let c0 = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &fixture_capabilities());
    assert_eq!(c0.errors_for(&id).next().unwrap().code, "E_RENDER_CAPABILITY");
    assert_eq!(c0.phrases.len(), 1, "only the phrase with the bad note is blocked");
    assert_eq!(c0.blocked.len(), 1);
    assert!(c0.phrases.iter().all(|ph| ph.control_track.is_none()));
    let c1 = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &continuous_caps());
    assert!(!c1.has_errors());
    assert_eq!(c1.phrases.len(), 2);
    let payload = serde_json::to_value(verdant_backend::adapter::payload_for(&c1.phrases[0], Route::VerdantVoice, 10008)).unwrap();
    assert!(payload["control_track"]["f0_hz"].as_array().unwrap().len() > 100);
    assert!(payload.get("control_track").is_some());
    let plain = serde_json::to_value(verdant_backend::adapter::payload_for(&c1.phrases[1], Route::VerdantVoice, 10008)).unwrap();
    assert!(plain.get("control_track").is_none(), "no modulation → native F0 path unchanged");
}

#[test]
fn dyn_is_post_gain_and_does_not_change_the_render_request() {
    let mut n = sung(0, 960, 62, "啊", "a");
    let p0 = project_with(vec![n.clone()]);
    n.curves.gain_millidecibel_points =
        vec![GainPoint { id: "g0".into(), tick: 0, millidecibels: -6000, unknown: Default::default() }, GainPoint { id: "g1".into(), tick: 960, millidecibels: 0, unknown: Default::default() }];
    let p1 = project_with(vec![n.clone()]);
    let caps = continuous_caps();
    let a = plan(&p0, &p0.tracks[0].id, Route::VerdantVoice, &caps);
    let b = plan(&p1, &p1.tracks[0].id, Route::VerdantVoice, &caps);
    assert!(!b.has_errors());
    let da = verdant_backend::adapter::render_digest(&verdant_backend::adapter::payload_for(&a.phrases[0], Route::VerdantVoice, 1), Route::VerdantVoice, &caps);
    let db = verdant_backend::adapter::render_digest(&verdant_backend::adapter::payload_for(&b.phrases[0], Route::VerdantVoice, 1), Route::VerdantVoice, &caps);
    assert_eq!(da, db, "editing DYN must not trigger a re-render");
    let env = verdant_backend::post::dyn_envelope(&[&n], &TempoMap::constant(120.0), 0.0, 1000, 500).unwrap();
    assert!((env[0] - 0.501).abs() < 0.01, "-6 dB at the start");
    assert!(env[499] > 0.99);
}

#[test]
fn multi_track_plans_diagnose_retired_route_and_keep_current_track() {
    let mut p = project_with(vec![sung(0, 960, 62, "啊", "a")]);
    let mut harmony = Track::new("和声", "blue");
    harmony.render_route = Some("C1".into());
    let mut h = sung(0, 960, 66, "哦", "o");
    h.vibrato = Some(Vibrato::DEFAULT);
    harmony.notes.push(h);
    let hid = harmony.id.clone();
    p.tracks.push(harmony);
    let caps = continuous_caps();
    let plans = verdant_backend::plan_all(&p, &hid, &caps);
    assert_eq!(plans[0].track, hid, "edited track first");
    assert!(plans[0].diagnostics.iter().any(|error| error.code == "E_RENDER_ROUTE"));
    assert_eq!(plans[1].route, Route::VerdantVoice);
    let mut s = RenderScheduler::new();
    s.on_event(WorkerEvent::Ready { session_id: "s".into(), workspace: "/w".into() });
    s.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(caps) });
    s.set_plans(3, &plans, 10008);
    assert_eq!(s.wanted().len(), 1);
    let Some(Action::Send(req)) = s.poll().into_iter().next() else { panic!() };
    assert_eq!(req.route, Some(Route::VerdantVoice));
    assert!(req.job_id.as_ref().unwrap().len() <= 64);
    assert!(req.payload.as_ref().unwrap().control_track.is_none());
}

#[test]
fn voice_control_track_full_loop_with_test_double() {
    let ws = scratch("ctl");
    let (client, rx) = test_double(&ws, &[]);
    let mut s = RenderScheduler::new();
    connect(&mut s, &client, &rx, &ws);
    let mut n = sung(0, 960, 62, "啊", "a");
    n.vibrato = Some(Vibrato::DEFAULT);
    n.curves.pitch_millicent_points = vec![
        verdant_domain::model::PitchPoint { id: "x".into(), tick: 0, millicents: -200_000, unknown: Default::default() },
        verdant_domain::model::PitchPoint { id: "y".into(), tick: 200, millicents: 0, unknown: Default::default() },
    ];
    let p = project_with(vec![n]);
    let caps = s.capabilities.clone().unwrap();
    s.set_plan(5, &plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps), 10008);
    for a in s.poll() {
        if let Action::Send(r) = a {
            assert!(r.payload.as_ref().unwrap().control_track.is_some());
            client.send(&r).unwrap();
        }
    }
    let deadline = Instant::now() + Duration::from_secs(120);
    while !s.all_ready() {
        assert!(Instant::now() < deadline, "{:?}", s.jobs());
        let m = next(&rx, Duration::from_secs(90));
        drive(&mut s, &client, &ws, m);
        assert!(!matches!(s.slot_status(0), Some(SlotStatus::Failed { .. })), "{:?}", s.jobs());
    }
    assert_eq!(s.playable().len(), 1);
    client.send(&verdant_backend::Request::shutdown("bye")).unwrap();
    wait_for(&rx, |m| matches!(m, WorkerMessage::Exited { .. }));
    let _ = std::fs::remove_dir_all(ws);
}

// ---- recovered make-pinyin-score contract ------------------------------------

use verdant_backend::cache::CacheError;
use verdant_backend::{ArtifactCache, SubSyllableTiming, sub_syllable_timing};
use verdant_domain::TrackId;

#[test]
fn pinyin_emits_recovered_initial_final_contexts() {
    // 120 BPM: 960 ticks = 0.5 s. "ba1" has initial b + final a; bare "a1"
    // is final-only; both carry explicit recovered contexts and midi_note.
    let p = project_with(vec![sung(0, 960, 62, "吧", "ba1"), sung(960, 960, 62, "啊", "a1")]);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &fixture_capabilities());
    assert!(!pl.has_errors(), "{:?}", pl.diagnostics);
    assert_eq!(pl.phrases.len(), 1);
    let segs = &pl.phrases[0].score.segments;
    assert_eq!(segs.len(), 3, "initial + final for ba1, final-only for a1");

    let init = &segs[0];
    assert_eq!(init.label, "ba1: initial b");
    assert!(!init.voiced, "obstruent initial is unvoiced (only r is voiced)");
    assert!((init.duration_seconds - 0.11).abs() < 1e-9, "min(0.11, 0.22*0.5)");
    assert_eq!(init.gap_after_seconds, 0.0);
    let ctx = init.context.as_ref().unwrap();
    assert_eq!(ctx.label, "diagnostic_chinese_initial_b");
    assert_eq!(ctx.features.get("C_SM").map(String::as_str), Some("b"));
    assert_eq!(ctx.features.get("C_Tone").map(String::as_str), Some("1"));
    assert_eq!(ctx.answers.get("C_SM"), Some(&true));
    assert_eq!(ctx.answers.get("C_EN"), Some(&false));
    assert_eq!(ctx.answers.get("C_SilSp"), Some(&false));
    assert_eq!(ctx.anchor_questions, vec!["C_SM==b".to_string()]);
    assert!(ctx.features.get("L_SM").is_none() && ctx.answers.get("L_SM").is_none());
    assert!(ctx.assumptions.iter().any(|s| s.contains("recovered original binary profile")));
    assert_eq!(
        init.excitation_note.as_deref(),
        Some("noise for obstruent initial; no recovered burst/closure or mixed-excitation timing")
    );
    assert!(init.midi_note.is_some() && init.f0_hz.is_none() && init.vowel.is_none());

    let fin = &segs[1];
    assert_eq!(fin.label, "ba1: final a");
    assert!(fin.voiced);
    assert!((fin.duration_seconds - 0.39).abs() < 1e-9);
    let fctx = fin.context.as_ref().unwrap();
    assert_eq!(fctx.label, "diagnostic_chinese_final_a");
    assert_eq!(fctx.features.get("C_YM").map(String::as_str), Some("a"));
    assert_eq!(fctx.features.get("L_SM").map(String::as_str), Some("b"));
    assert_eq!(fctx.answers.get("L_SM"), Some(&true));
    assert_eq!(fctx.answers.get("C_SM"), Some(&false));
    assert_eq!(fctx.anchor_questions, vec!["C_YM==a".to_string()]);

    let fin_only = &segs[2];
    assert_eq!(fin_only.label, "a1: final a");
    assert!(fin_only.context.as_ref().unwrap().answers.get("L_SM").is_none());
    assert!(fin_only.context.as_ref().unwrap().features.get("L_SM").is_none());

    // The wire payload carries the recovered fields verbatim.
    let payload = serde_json::to_value(verdant_backend::adapter::payload_for(&pl.phrases[0], Route::VerdantVoice, 10008)).unwrap();
    let s0 = &payload["score"]["segments"][0];
    assert!(s0.get("vowel").is_none() && s0.get("context_mode").is_none() && s0.get("f0_hz").is_none());
    assert_eq!(s0["midi_note"], 62.0);
    assert_eq!(s0["context"]["anchor_questions"][0], "C_SM==b");

    // A single-letter prefix must not hide a recovered two-letter initial.
    for (syllable, initial) in [("zha1", "zh"), ("cha2", "ch"), ("sha3", "sh")] {
        let p = project_with(vec![sung(0, 960, 62, "", syllable)]);
        let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &fixture_capabilities());
        assert!(!pl.has_errors(), "{syllable}: {:?}", pl.diagnostics);
        let segments = &pl.phrases[0].score.segments;
        assert_eq!(segments[0].context.as_ref().unwrap().features["C_SM"], initial);
        assert_eq!(segments[1].context.as_ref().unwrap().features["L_SM"], initial);
    }
}

#[test]
fn pinyin_boundaries_are_explicit_diagnostics() {
    let caps = fixture_capabilities();
    // Unsupported syllable, out-of-domain durations, non-token input.
    for (py, expect) in [("shong3", "E_UNSUPPORTED_SYLLABLE"), ("fai5", "E_UNSUPPORTED_SYLLABLE"), ("ba11", "E_UNSUPPORTED_SYLLABLE")] {
        let p = project_with(vec![sung(0, 960, 60, "x", py)]);
        let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps);
        assert_eq!(pl.diagnostics.iter().find(|d| d.severity == verdant_backend::Severity::Error).unwrap().code, expect, "{py}");
        assert!(pl.phrases.is_empty());
    }
    // Short phoneme budgets and notes beyond the worker phrase cap fail.
    for dur in [60, 59520] {
        let p = project_with(vec![sung(0, dur, 60, "吧", "ba1")]);
        let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps);
        assert!(pl.errors_for(&p.tracks[0].notes[0].id).any(|d| d.code == "E_SYLLABLE_DOMAIN"), "dur {dur}");
    }
    // A valid syllable missing from an older worker capability is refused.
    let mut shrunk = caps.clone();
    shrunk.native_score.as_mut().unwrap().inventory.retain(|s| s != "nu");
    let p = project_with(vec![sung(0, 960, 60, "怒", "nu")]);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &shrunk);
    assert_eq!(pl.errors_for(&p.tracks[0].notes[0].id).next().unwrap().code, "E_RENDER_CAPABILITY");
}

#[test]
fn sub_syllable_timing_is_capability_driven() {
    let caps = fixture_capabilities();
    assert_eq!(sub_syllable_timing(&caps), SubSyllableTiming::Supported, "non-empty recovered inventory");
    let mut denied = caps.clone();
    denied.native_score.as_mut().unwrap().sub_syllable_timing = Some(false);
    assert!(matches!(sub_syllable_timing(&denied), SubSyllableTiming::SyllableTotal(_)));
    let p = project_with(vec![sung(0, 960, 60, "吧", "ba1")]);
    let pl = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &denied);
    assert_eq!(pl.errors_for(&p.tracks[0].notes[0].id).next().unwrap().code, "E_RENDER_CAPABILITY");
    let mut none_caps = caps.clone();
    none_caps.native_score.as_mut().unwrap().inventory.clear();
    none_caps.native_score.as_mut().unwrap().sub_syllable_timing = None;
    assert!(matches!(sub_syllable_timing(&none_caps), SubSyllableTiming::SyllableTotal(_)));
}

#[test]
fn expanded_pinyin_contexts_profiles_and_long_notes() {
    use verdant_backend::adapter::{note_support, NoteSupport};
    use verdant_backend::pinyin::{parse_token, recovered_syllables};
    let caps = fixture_capabilities();
    assert_eq!(recovered_syllables().len(), 418);
    for syllable in recovered_syllables() {
        assert_eq!(parse_token(&syllable).unwrap().syllable, syllable);
    }
    for (spelling, initial, final_) in [("jiang1", "j", "iang"), ("jiong2", "j", "iong"),
        ("zhi1", "zh", "iii"), ("zi1", "z", "ii"), ("lü4", "l", "v"), ("lv4", "l", "v"),
        ("lu:4", "l", "v"), ("wu3", "", "u"), ("jun1", "j", "vn"), ("shuang1", "sh", "uang"),
        ("diu3", "d", "iou"), ("dui4", "d", "uei"), ("dun5", "d", "uen"),
        ("jüan2", "j", "van"), ("ju:an2", "j", "van"), ("jvan2", "j", "van"),
        ("yuan2", "", "van"), ("wo3", "", "uo"), ("wen2", "", "uen")] {
        let token = parse_token(spelling).unwrap();
        assert_eq!((token.initial.as_str(), token.final_.as_str()), (initial, final_));
    }
    assert_eq!(parse_token("dun5").unwrap().tone, 0);
    for spelling in ["ra", "shong", "fai", "ba11", "ba6", "ba an", "mǎ2", "huar"] {
        assert!(parse_token(spelling).is_err(), "{spelling}");
    }
    let mut note = sung(0, 960, 60, "啊", "a");
    assert!(matches!(note_support(&note, &caps), Ok(NoteSupport::VowelProxy { .. })));
    note.pronunciation.frontend_profile = verdant_domain::HANZI_PINYIN_PROFILE.into();
    assert!(matches!(note_support(&note, &caps), Ok(NoteSupport::Pinyin(_))));
    note.pronunciation.source = PronunciationSource::ExplicitMap;
    note.pronunciation.frontend_profile = "english-vowel-proxy-native-0.2".into();
    assert!(matches!(note_support(&note, &caps), Ok(NoteSupport::Pinyin(_))));
    note.pronunciation = Default::default();
    note.lyric = "行".into();
    let error = note_support(&note, &caps).unwrap_err();
    assert_eq!(error.code, "E_PRONUNCIATION_AMBIGUOUS");
    assert!(error.message.contains("行") && error.message.contains("汉字/pinyin"));
    for ticks in [11520, 57600] {
        let p = project_with(vec![sung(0, ticks, 60, "江", "jiang1")]);
        let plan = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps);
        assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
        let segments = &plan.phrases[0].score.segments;
        assert_eq!(segments.len(), 2);
        assert!((segments.iter().map(|s| s.duration_seconds).sum::<f64>() - ticks as f64 / 1920.0).abs() < 1e-6);
    }
}

// ---- durable render cache ----------------------------------------------------

#[test]
fn explicit_phoneme_timing_changes_segments_and_cache_content_only_when_supported() {
    let caps = fixture_capabilities();
    let mut project = project_with(vec![sung(0, 960, 60, "吧", "ba1")]);
    let track = project.tracks[0].id.clone();
    let original = plan(&project, &track, Route::VerdantVoice, &caps);
    project.tracks[0].notes[0].pronunciation.phoneme_durations_permille = Some(vec![400, 600]);
    let changed = plan(&project, &track, Route::VerdantVoice, &caps);
    assert!(!changed.has_errors());
    let segments = &changed.phrases[0].score.segments;
    assert_eq!(segments[0].duration_seconds, 0.2);
    assert_eq!(segments[1].duration_seconds, 0.3);
    assert_eq!(segments[0].context, original.phrases[0].score.segments[0].context);
    let key = |phrase| verdant_backend::adapter::render_digest(
        &verdant_backend::adapter::payload_for(phrase, Route::VerdantVoice, 10008), Route::VerdantVoice, &caps);
    assert_ne!(key(&original.phrases[0]), key(&changed.phrases[0]));
    let mut denied = caps.clone();
    denied.native_score.as_mut().unwrap().phoneme_duration_control = false;
    assert!(plan(&project, &track, Route::VerdantVoice, &denied).has_errors());
    for weights in [vec![1000], vec![500, 400], vec![10, 990]] {
        project.tracks[0].notes[0].pronunciation.phoneme_durations_permille = Some(weights);
        let rejected = plan(&project, &track, Route::VerdantVoice, &caps);
        assert!(rejected.phrases.is_empty());
        assert!(rejected.errors_for(&project.tracks[0].notes[0].id).any(|error| error.code == "E_PHONEME_TIMING"));
    }
}

#[test]
fn minimum_phone_split_preserves_total_without_microsecond_prerounding() {
    let mut cv = sung(164, 4000, 60, "吧", "ba1");
    cv.pronunciation.phoneme_durations_permille = Some(vec![990, 10]);
    let mut project = project_with(vec![sung(0, 164, 60, "啊", "a"), cv]);
    project.tempo_map[0].micro_bpm = 100_000_000;
    let planned = plan(&project, &project.tracks[0].id, Route::VerdantVoice, &fixture_capabilities());
    assert!(!planned.has_errors(), "{:?}", planned.diagnostics);
    let segments = &planned.phrases[0].score.segments;
    assert_eq!(segments.len(), 3);
    assert!((segments[2].duration_seconds - 0.025).abs() < 1e-12);
    assert!((segments[1].duration_seconds + segments[2].duration_seconds - 2.5).abs() < 1e-12);
}


fn entry(digest: &str, track: &TrackId, index: usize) -> (SyntheticArtifacts, verdant_backend::CacheEntry) {
    let source = SyntheticArtifacts::new(CompressionMethod::Stored);
    let audio = source.verify(Route::VerdantVoice);
    let entry = verdant_backend::CacheEntry {
        digest: digest.into(),
        job_id: "job_x".into(),
        revision: 7,
        route: Route::VerdantVoice,
        audio,
        track: Some(track.clone()),
        phrase_index: Some(index),
        start_seconds: 1.5,
        seconds: 0.5,
        last_wanted_revision: 7,
    };
    (source, entry)
}

#[test]
fn durable_cache_roundtrips_and_reports_corruption() {
    let dir = scratch("cache");
    let cache = ArtifactCache::open(&dir).unwrap();
    let (_source, e) = entry("digest-a", &TrackId::from("t1"), 2);
    cache.store(&e).unwrap();
    drop(cache);
    let cache = ArtifactCache::open(&dir).unwrap();
    assert_eq!(cache.entries(), vec!["digest-a".to_string()]);
    let r = cache.restore("digest-a").unwrap().unwrap();
    assert_eq!(r.audio.sample_rate, e.audio.sample_rate);
    assert_eq!(r.audio.samples.len(), e.audio.samples.len());
    assert_eq!(r.audio.sha256, e.audio.sha256);
    assert!(!r.audio.f0_tracks.is_empty());
    assert_eq!(r.track, e.track);
    assert_eq!(r.phrase_index, Some(2));
    assert_eq!(r.revision, 7);
    assert!((r.start_seconds - 1.5).abs() < 1e-9);
    // Clean miss.
    assert!(cache.restore("digest-zzz").unwrap().is_none());
    // Corrupt manifest → visible error.
    let mdir = dir.join("digest-a");
    std::fs::write(mdir.join("manifest.json"), b"{").unwrap();
    assert!(matches!(cache.restore("digest-a"), Err(CacheError::Corrupt(_))));
    // Restored manifest, truncated artifact bytes → visible error.
    cache.store(&e).unwrap();
    let wav = mdir.join("native.wav");
    let n = std::fs::metadata(&wav).unwrap().len();
    std::fs::File::options().write(true).open(&wav).unwrap().set_len(n - 8).unwrap();
    assert!(matches!(cache.restore("digest-a"), Err(CacheError::Corrupt(_))));
    // Crash leftovers: stale temp files cleaned at open; dir without a
    // committed manifest is a clean miss.
    std::fs::create_dir_all(dir.join("digest-torn")).unwrap();
    std::fs::write(dir.join("digest-torn/.manifest.json.tmp.1"), b"{").unwrap();
    let cache2 = ArtifactCache::open(&dir).unwrap();
    assert!(cache2.restore("digest-torn").unwrap().is_none());
    assert!(dir.join("digest-torn").read_dir().unwrap().next().is_none(), "staging removed at open");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn durable_cache_open_preserves_a_released_writers_pending_rename() {
    let root = scratch("cache-legacy-writer");
    let dir = root.join("digest-live");
    std::fs::create_dir_all(&dir).unwrap();
    let (_source, e) = entry("digest-live", &TrackId::from("t1"), 2);
    let bytes = std::fs::read(&e.audio.path).unwrap();
    let stage = dir.join(format!(".native.wav.tmp.{}.0", std::process::id()));
    let mut writer = std::fs::File::create_new(&stage).unwrap();
    writer.write_all(&bytes).unwrap();
    writer.sync_all().unwrap();
    let reader = ArtifactCache::open(&root).unwrap();
    assert!(reader.entries().is_empty());
    assert!(reader.restore(&e.digest).unwrap().is_none());
    drop(writer);
    std::fs::rename(&stage, dir.join("native.wav")).unwrap();
    assert_eq!(std::fs::read(dir.join("native.wav")).unwrap(), bytes);
    reader.store(&e).unwrap();
    let restored = ArtifactCache::open(&root).unwrap().restore(&e.digest).unwrap().unwrap();
    assert_eq!(restored.audio.sha256, e.audio.sha256);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn durable_cache_restores_sibling_when_abandoned_stage_is_read_only() {
    use std::fs::{self, File, Permissions};
    use std::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt;

    let root = scratch("cache-readonly-orphan");
    let cache = ArtifactCache::open(&root).unwrap();
    let (_source, sibling) = entry("digest-ready", &TrackId::from("t1"), 0);
    cache.store(&sibling).unwrap();

    let dir = root.join("digest-orphan");
    fs::create_dir(&dir).unwrap();
    let stage = dir.join(".verdant-cache-stage");
    fs::write(&stage, b"partial").unwrap();
    let probe = dir.join("unlink-probe");
    File::create_new(&probe).unwrap();
    let lock_path = root.join(".verdant-cache-writes/locks/digest-orphan");
    File::create_new(&lock_path).unwrap();
    let permissions = fs::metadata(&dir).unwrap().permissions();
    fs::set_permissions(&dir, Permissions::from_mode(0o555)).unwrap();
    let lock = File::options().read(true).write(true).open(&lock_path).unwrap();
    lock.try_lock().unwrap();
    drop(lock);

    // Root may bypass the mode bits. Use a sibling file to establish which
    // outcome the filesystem permits without disturbing the recovery stage.
    let unlink = fs::remove_file(probe);
    let reopened = ArtifactCache::open(&root);
    fs::set_permissions(&dir, permissions).unwrap();
    let reader = reopened.unwrap();
    match unlink {
        Err(e) if e.kind() == ErrorKind::PermissionDenied => assert_eq!(fs::read(&stage).unwrap(), b"partial"),
        Ok(()) => assert!(!stage.try_exists().unwrap()),
        Err(e) => panic!("unexpected unlink probe failure: {e}"),
    }
    assert!(reader.restore("digest-orphan").unwrap().is_none());
    let restored = reader.restore(&sibling.digest).unwrap().unwrap();
    assert_eq!(restored.audio.sha256, sibling.audio.sha256);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_render_cache_routes_miss_before_artifacts_are_read() {
    let dir = scratch("retired-cache");
    let cache = ArtifactCache::open(&dir).unwrap();
    for (index, route) in ["C0", "C1", "A"].into_iter().enumerate() {
        let digest = format!("retired-{index}");
        let entry = dir.join(&digest);
        std::fs::create_dir_all(&entry).unwrap();
        std::fs::write(entry.join("manifest.json"), serde_json::to_vec(&serde_json::json!({
            "version":2,"digest":digest,"job_id":"old","revision":0,"route":route,
            "slot":null,"artifacts":[]
        })).unwrap()).unwrap();
        assert!(cache.restore(&digest).unwrap().is_none());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn import_restored_entry_binds_and_ghosts_mark_old_edits() {
    let caps = fixture_capabilities();
    let dir = scratch("import");
    let cache = ArtifactCache::open(&dir).unwrap();
    let mut s = RenderScheduler::new();
    s.on_event(WorkerEvent::Ready { session_id: "s".into(), workspace: "/w".into() });
    s.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(caps.clone()) });
    let p = project_with(vec![sung(0, 960, 60, "啊", "a")]);
    s.set_plan(3, &plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps), 10008);
    let digest = s.wanted()[0].digest.clone();
    let (_source, cached) = entry(&digest, &p.tracks[0].id, 0);
    cache.store(&cached).unwrap();
    // Fresh scheduler (new session): restore → import → Ready without render.
    let restored = cache.restore(&digest).unwrap().unwrap();
    s.import_verified(restored);
    assert!(matches!(s.slot_status(0), Some(SlotStatus::Ready { rendered_revision: 7, .. })));
    assert_eq!(s.playable().len(), 1);
    // Edit away (r4, different pitch → different digest): old entry is a ghost.
    let mut p2 = p.clone();
    p2.tracks[0].notes[0].midi_note = 65;
    s.set_plan(4, &plan(&p2, &p2.tracks[0].id, Route::VerdantVoice, &caps), 10008);
    assert!(s.playable().is_empty(), "stale content never replaces current binding");
    let g = s.ghost(&p2.tracks[0].id, 0).expect("old edit ghost");
    assert_eq!(g.digest, digest);
    assert_eq!(g.revision, 7);
    assert_eq!(s.stale_slots().len(), 1);
    // Undo back to identical content: wanted again, ghost gone.
    s.set_plan(5, &plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps), 10008);
    assert!(s.ghost(&p.tracks[0].id, 0).is_none());
    assert!(s.stale_slots().is_empty());
    assert!(matches!(s.slot_status(0), Some(SlotStatus::Ready { .. })));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ghost_points_at_superseded_render_not_arbitrary_entries() {
    let caps = fixture_capabilities();
    let source = SyntheticArtifacts::new(CompressionMethod::Stored);
    let mut s = RenderScheduler::new();
    s.on_event(WorkerEvent::Ready { session_id: "s".into(), workspace: "/w".into() });
    s.on_event(WorkerEvent::Capabilities { request_id: None, capabilities: Box::new(caps.clone()) });
    let track = TrackId::from("t1");
    let mut p = project_with(vec![sung(0, 960, 60, "啊", "a")]);
    p.tracks[0].id = track.clone();
    let pl = plan(&p, &track, Route::VerdantVoice, &caps);
    s.set_plans(1, &[pl], 10008);
    let d1 = s.wanted()[0].digest.clone();
    // Submit r1 job (records slot history), complete it, verify it in.
    for a in s.poll() {
        if let Action::Send(req) = a {
            let job_id = req.job_id.clone().unwrap();
            s.on_event(WorkerEvent::Completed {
                job_id: job_id.clone(),
                revision: 1,
                artifacts: vec![],
                identity_status: None,
                elapsed_seconds: None,
            });
            s.on_verified(&job_id, Ok(source.verify(Route::VerdantVoice)));
        }
    }
    // r2 edits the note; r1 content becomes the ghost for this slot.
    let mut p2 = p.clone();
    p2.tracks[0].notes[0].midi_note = 64;
    let pl2 = plan(&p2, &track, Route::VerdantVoice, &caps);
    s.set_plans(2, &[pl2], 10008);
    let g = s.ghost(&track, 0).expect("superseded render");
    assert_eq!(g.digest, d1);
    assert_eq!(g.track.as_ref().unwrap().as_str(), "t1");
    assert_eq!(g.phrase_index, Some(0));
    // Ghost of a slot that never rendered is absent.
    assert!(s.ghost(&TrackId::from("other"), 0).is_none());
}
