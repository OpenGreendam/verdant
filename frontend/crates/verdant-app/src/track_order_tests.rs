//! Track-header regression coverage through the real document reducer,
//! playback preparation, automation compiler and audio callback. PCM is owned
//! synthetic test material; no voice bank, worker or audio device is needed.
use std::{path::PathBuf, sync::atomic::Ordering};

use verdant_backend::VerifiedAudio;
use verdant_domain::{Command, CommandBatch, Note, NoteId, Project, ProjectStore, Track};
use verdant_domain::automation::{AUTOMATION_KEY, AutomationDocument, AutomationLane, AutomationOwner, AutomationPoint, AutomationTarget};
use verdant_media::{AssetPolicy, AudioClip, MediaImporter};

use crate::editor::project_audio::{self, PhraseSource};

const RATE: u32 = 48_000;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-track-order-{}", NoteId::generate()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

fn add_lane(document: &mut AutomationDocument, target: AutomationTarget, value: f64) {
    let mut lane = AutomationLane::new(target);
    lane.set_point(AutomationPoint::new(0, value)).unwrap();
    document.set_lane(lane);
}

fn fixture(scratch: &Scratch) -> (Project, Vec<PhraseSource>, AssetPolicy) {
    // A tiny real PCM16 WAV exercises authorized import and persisted clip
    // ownership instead of substituting already-indexed engine clips.
    let path = scratch.0.join("reference.wav");
    let frames = 256u32;
    let data_bytes = frames * 2;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for _ in 0..frames { wav.extend_from_slice(&4096i16.to_le_bytes()); }
    std::fs::write(&path, wav).unwrap();
    let policy = AssetPolicy::for_file(&path).unwrap();
    let imported = MediaImporter::new(policy.clone(), Default::default()).import_file(&path).unwrap();
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 120_000_000;
    project.tracks.push(Track::new("和声", "blue"));
    project.tracks.push(Track::new("伴奏", "orange"));
    let mut document = AutomationDocument::default();
    let mut sources = Vec::new();
    for (index, track) in project.tracks.iter_mut().enumerate() {
        track.gain_millidecibels = -2000 * index as i32;
        track.pan_permille = -300 + index as i32 * 300;
        let note = Note::new(0, 960, 60 + index as i32);
        let note_id = note.id.clone();
        track.notes.push(note);
        track.refresh_index();
        let instance = format!("insert-{index}");
        track.unknown.insert("inserts".into(), serde_json::json!([
            {"instance_id": instance, "format": "clap", "id": "test-plugin"}
        ]));
        let mut clip = AudioClip::new(imported.source.clone(), track.id.clone(), (index as i64 + 1) * 480);
        clip.gain = 0.25 + index as f32 * 0.25;
        verdant_media::write_clips(track, &[clip]).unwrap();
        add_lane(&mut document, AutomationTarget::TrackGain { track_id: track.id.clone() }, -3000.0 * (index + 1) as f64);
        add_lane(&mut document, AutomationTarget::TrackPan { track_id: track.id.clone() }, -500.0 + 500.0 * index as f64);
        add_lane(&mut document, AutomationTarget::PluginParameter {
            owner: AutomationOwner::Track { track_id: track.id.clone() },
            instance_id: instance, parameter_id: "cutoff".into(),
        }, 100.0 * (index + 1) as f64);
        sources.push(PhraseSource {
            track: track.id.clone(), start_seconds: 0.0, note_ids: vec![note_id], phoneme_timing: None,
            playback_key: None,
            audio: VerifiedAudio {
                path: scratch.0.join(format!("synthetic-phrase-{index}.wav")),
                sha256: format!("synthetic in-memory phrase {index}; no file read"),
                sample_rate: RATE, samples: vec![0.1 * (index + 1) as f32; frames as usize].into(),
                f0_tracks: Vec::new(), time_map: None,
            },
        });
    }
    project.extensions.insert(AUTOMATION_KEY.into(), document.extension_value().unwrap().unwrap());
    (project, sources, policy)
}

fn assert_channel_mapping(project: &Project, sources: &[PhraseSource], policy: &AssetPolicy) -> (Vec<f32>, Vec<(f32, f32)>) {
    let prepared = project_audio::prepare_timeline(project, sources, policy.clone(), RATE, Default::default(), None).unwrap();
    assert_eq!(prepared.phrase_count, project.tracks.len());
    assert_eq!(prepared.media_count, project.tracks.len());
    let tempo = verdant_domain::TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    let document = AutomationDocument::from_project(project).unwrap();
    let (mut control, mut rt) = verdant_audio::engine::channel(RATE, 64);
    for (index, track) in project.tracks.iter().enumerate() {
        let source = sources.iter().find(|source| source.track == track.id).unwrap();
        let phrase = prepared.timeline.clips.iter().find(|clip| clip.start == 0 && clip.track as usize == index).unwrap();
        assert_eq!(phrase.samples.as_ref(), source.audio.samples.as_ref(), "phrase PCM must follow its TrackId into the new mixer slot");
        let owned = verdant_media::read_clips(track).unwrap();
        assert_eq!(owned[0].track, track.id);
        let start = u64::try_from(tempo.sample_at(owned[0].start_tick, RATE))
            .expect("fixture clip must start at a nonnegative sample");
        let clip = prepared.timeline.clips.iter().find(|clip| clip.start == start).unwrap();
        assert_eq!(clip.track as usize, index, "imported clip must follow its owner into the new mixer slot");
        assert_eq!(clip.gain, owned[0].gain);
        let automation = &prepared.timeline.automation.tracks[index];
        for (target, actual) in [
            (AutomationTarget::TrackGain { track_id: track.id.clone() }, automation.gain.value(0)),
            (AutomationTarget::TrackPan { track_id: track.id.clone() }, automation.pan.value(0)),
        ] {
            assert_eq!(actual, Some(document.lane(&target).unwrap().points[0].value));
        }
        let instance = track.unknown["inserts"][0]["instance_id"].as_str().unwrap();
        assert_eq!(automation.plugins.len(), 1);
        assert_eq!(automation.plugins[0].instance_id, instance);
        assert_eq!(automation.plugins[0].parameter_id, "cutoff");
        let settings = super::channel_settings(&track.header(), false);
        control.set_channel(index, settings);
        let channel = &control.shared.channels[index];
        assert_eq!(f32::from_bits(channel.gain.load(Ordering::Relaxed)), settings.gain);
        assert_eq!(f32::from_bits(channel.pan.load(Ordering::Relaxed)), settings.pan);
    }
    control.set_timeline(prepared.timeline);
    control.play();
    let mut pcm = vec![0.0; 128];
    rt.process(&mut pcm, 2);
    let (meters, _) = control.take_meters(project.tracks.len());
    assert!(meters.iter().all(|(left, right)| *left > 0.0 && *right > 0.0));
    (pcm, meters)
}

#[test]
fn reorder_preserves_note_ids_automation_and_clip_owners_and_rebinds_audio_channels() {
    let scratch = Scratch::new();
    let (original, sources, policy) = fixture(&scratch);
    let original_tracks = original.tracks.clone();
    let original_automation = AutomationDocument::from_project(&original).unwrap();
    let (before_pcm, before_meters) = assert_channel_mapping(&original, &sources, &policy);
    let moved = original.tracks[2].id.clone();
    let mut store = ProjectStore::new(original);
    store.apply(store.revision(), CommandBatch::single("调整轨道顺序", Command::MoveTrack { track: moved.clone(), index: 0 })).unwrap().unwrap();
    assert_eq!(store.undo_depth(), 1, "one header edit must be one undo step");
    let reordered = store.project();
    assert_eq!(reordered.tracks.iter().map(|track| &track.id).collect::<Vec<_>>(),
        vec![&original_tracks[2].id, &original_tracks[0].id, &original_tracks[1].id]);
    assert_eq!(AutomationDocument::from_project(reordered).unwrap(), original_automation);
    for track in &original_tracks {
        assert_eq!(reordered.track(&track.id).unwrap(), track, "reordering must preserve complete tracks, including note IDs, clip IDs, owners and insert identities");
    }
    let (after_pcm, after_meters) = assert_channel_mapping(reordered, &sources, &policy);
    for (index, track) in reordered.tracks.iter().enumerate() {
        let previous = original_tracks.iter().position(|old| old.id == track.id).unwrap();
        assert_eq!(after_meters[index], before_meters[previous], "channel meters must follow the same track as clips and automation");
    }
    for (before, after) in before_pcm.iter().zip(&after_pcm) {
        assert!((before - after).abs() < 1e-6, "reordering must preserve the audible mix");
    }
    store.undo().unwrap().unwrap();
    assert_eq!(store.project().tracks, original_tracks);
    assert_channel_mapping(store.project(), &sources, &policy);
    store.redo().unwrap().unwrap();
    assert_eq!(store.project().tracks[0].id, moved);
    assert_channel_mapping(store.project(), &sources, &policy);
}
