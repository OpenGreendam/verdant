mod common;

use verdant_domain::{Command, CommandBatch, Project, ProjectStore, TempoMap, Track};
use verdant_media::{AUDIO_CLIPS_KEY, AudioClip, ImportLimits, clips_value, decode_bytes, prepare_clip, read_clips, write_clips};

#[test]
fn split_preserves_every_source_frame_and_identity_across_tempo_changes() {
    let pcm: Vec<f32> = (0..48_000).map(|i| (i % 100) as f32 / 100.0).collect();
    let audio = decode_bytes(&common::float32(&pcm, 1, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let track = Track::new("伴奏", "blue");
    let mut clip = AudioClip::new(common::source(&audio), track.id, 480);
    clip.source_offset_frames = 123;
    clip.duration_frames = 40_000;
    let tempo = TempoMap::new(&[
        verdant_domain::model::TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: Default::default() },
        verdant_domain::model::TempoPoint { tick: 960, micro_bpm: 90_000_000, unknown: Default::default() },
    ], 960).unwrap();
    let (left, right) = clip.split_at_tick(1200, &tempo).unwrap();
    assert_eq!(left.duration_frames + right.duration_frames, clip.duration_frames);
    assert_eq!(right.source_offset_frames, left.source_offset_frames + left.duration_frames);
    assert_eq!(left.id, clip.id);
    assert_ne!(right.id, clip.id);
    assert_eq!(left.source, clip.source);
    assert_eq!(right.source, clip.source);
    let left_pcm = prepare_clip(&left, &audio, &tempo, 48_000, 0).unwrap();
    let right_pcm = prepare_clip(&right, &audio, &tempo, 48_000, 0).unwrap();
    let original = prepare_clip(&clip, &audio, &tempo, 48_000, 0).unwrap();
    assert_eq!(left_pcm.frames() + right_pcm.frames(), original.frames());
    assert_eq!([left_pcm.samples.as_ref(), right_pcm.samples.as_ref()].concat(), original.samples.as_ref());
    assert_eq!(left.join_right(&right, &tempo).unwrap(), clip);
    assert!(clip.split_at_tick(clip.start_tick, &tempo).is_err());
    assert!(clip.split_at_tick(10_000, &tempo).is_err());
}

#[test]
fn join_rejects_timeline_gaps_source_gaps_and_different_sources_or_mix_settings() {
    let audio = decode_bytes(&common::float32(&vec![0.5; 48_000], 1, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let track = Track::new("伴奏", "blue");
    let clip = AudioClip::new(common::source(&audio), track.id, 0);
    let tempo = TempoMap::constant(120.0);
    let (left, right) = clip.split_at_tick(960, &tempo).unwrap();
    assert_eq!(left.join_right(&right, &tempo).unwrap(), clip);
    let mut invalid = right.clone();
    invalid.source.path = std::env::temp_dir().join("relocated-identical-source.wav").to_string_lossy().into_owned();
    assert_eq!(left.join_right(&invalid, &tempo).unwrap(), clip);
    invalid = right.clone();
    invalid.start_tick += 1;
    assert!(left.join_right(&invalid, &tempo).is_err());
    invalid.start_tick = right.start_tick - 1;
    assert!(left.join_right(&invalid, &tempo).is_err());
    invalid = right.clone(); invalid.source_offset_frames -= 1;
    assert!(left.join_right(&invalid, &tempo).is_err());
    invalid = right.clone(); invalid.source.sha256 = "ab".repeat(32);
    assert!(left.join_right(&invalid, &tempo).is_err());
    invalid = right.clone(); invalid.gain = 0.5;
    assert!(left.join_right(&invalid, &tempo).is_err());
    invalid = right.clone(); invalid.fade_in_frames = 100;
    assert!(left.join_right(&invalid, &tempo).is_err());
}

#[test]
fn fades_change_exact_edge_frames_on_both_channels_and_leave_source_pcm_immutable() {
    let input = [0.5, -0.5, 0.25, -0.25, 0.75, -0.75, 0.5, -0.5, 0.25, -0.25, 0.75, -0.75, 0.5, -0.5, 0.25, -0.25];
    let audio = decode_bytes(&common::float32(&input, 2, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let track = Track::new("伴奏", "blue");
    let mut clip = AudioClip::new(common::source(&audio), track.id, 0);
    clip.fade_in_frames = 3;
    clip.fade_out_frames = 3;
    let prepared = prepare_clip(&clip, &audio, &TempoMap::constant(120.0), 48_000, 0).unwrap();
    assert_eq!(prepared.samples.as_ref(), &[0.0, -0.0, 0.125, -0.125, 0.75, -0.75, 0.5, -0.5, 0.25, -0.25, 0.75, -0.75, 0.25, -0.25, 0.0, -0.0]);
    assert_eq!(audio.samples.as_ref(), &input);
    assert_eq!(prepared.frames(), clip.duration_frames);
    let copy = clip.duplicate_at_tick(960).unwrap();
    assert_ne!(copy.id, clip.id);
    assert_eq!(copy.source, clip.source);
    assert_eq!((copy.fade_in_frames, copy.fade_out_frames), (3, 3));
    // One-frame fades silence exactly their edge frame; overlapping envelopes
    // multiply rather than rewriting the source or dropping the other edge.
    clip.fade_in_frames = 1; clip.fade_out_frames = 1;
    let prepared = prepare_clip(&clip, &audio, &TempoMap::constant(120.0), 48_000, 0).unwrap();
    assert_eq!(&prepared.samples[2..14], &input[2..14]);
    assert_eq!(&prepared.samples[..2], &[0.0, -0.0]);
    assert_eq!(&prepared.samples[14..], &[0.0, -0.0]);
    clip.fade_in_frames = clip.duration_frames + 1;
    assert!(clip.validate().is_err());
}

#[test]
fn region_transactions_undo_atomically_and_old_documents_default_to_no_fades() {
    let audio = decode_bytes(&common::pcm16(&vec![100; 48_000], 1, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let mut track = Track::new("伴奏", "blue");
    let clip = AudioClip::new(common::source(&audio), track.id.clone(), 0);
    let mut old = serde_json::to_value(&clip).unwrap();
    old.as_object_mut().unwrap().remove("fade_in_frames");
    old.as_object_mut().unwrap().remove("fade_out_frames");
    assert_eq!(serde_json::from_value::<AudioClip>(old).unwrap(), clip);
    write_clips(&mut track, &[clip.clone()]).unwrap();
    let mut project = Project::new_empty(); project.tracks = vec![track.clone()];
    let mut store = ProjectStore::new(project);
    let tempo = TempoMap::constant(120.0);
    let (left, right) = clip.split_at_tick(960, &tempo).unwrap();
    let edits = [
        vec![left.clone(), right.clone()],
        vec![left.join_right(&right, &tempo).unwrap()],
        vec![{ let mut faded = clip.clone(); faded.fade_in_frames = 120; faded }],
        vec![clip.clone(), clip.duplicate_at_tick(1920).unwrap()],
        vec![{ let mut muted = clip.clone(); muted.muted = true; muted }],
    ];
    for clips in edits {
        let value = clips_value(track.id.clone(), &clips).unwrap();
        let command = Command::SetTrackData { track: track.id.clone(), key: AUDIO_CLIPS_KEY.into(), value };
        // Joining back to the initial clip is checked from the split state.
        if clips == vec![clip.clone()] {
            let value = clips_value(track.id.clone(), &[left.clone(), right.clone()]).unwrap();
            store.apply(store.revision(), CommandBatch::single("split", Command::SetTrackData { track: track.id.clone(), key: AUDIO_CLIPS_KEY.into(), value })).unwrap();
        }
        let before = store.project().tracks.clone();
        let depth = store.undo_depth();
        store.apply(store.revision(), CommandBatch::single("音频区域手势", command)).unwrap().unwrap();
        assert_eq!(store.undo_depth(), depth + 1);
        assert_eq!(read_clips(&store.project().tracks[0]).unwrap(), clips);
        store.undo().unwrap().unwrap();
        assert_eq!(store.project().tracks, before);
        // Return to the same baseline for the next independent gesture.
        if store.undo_depth() > 0 { store.undo().unwrap().unwrap(); }
    }
}
