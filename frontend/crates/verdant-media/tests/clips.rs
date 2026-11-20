mod common;
use std::sync::Arc;
use verdant_domain::{TempoMap, Track, model::TempoPoint};
use verdant_media::{AUDIO_CLIPS_KEY, AudioClip, ImportLimits, WaveformCache, clips_value, decode_bytes, prepare_clip, read_clips, write_clips};

#[test]
fn stable_document_round_trip_preserves_source_trim_track_and_other_extensions() {
    let audio = decode_bytes(&common::pcm16(&[1, 2, 3, 4], 1, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let mut track = Track::new("Audio", "blue");
    track.unknown.insert("another_extension".into(), serde_json::json!({"keep": true}));
    let mut clip = AudioClip::new(common::source(&audio), track.id.clone(), 960);
    clip.source_offset_frames = 1; clip.duration_frames = 2; clip.gain = 0.5; clip.pan = -0.25;
    let id = clip.id;
    write_clips(&mut track, &[clip.clone()]).unwrap();
    let reloaded: Track = serde_json::from_value(serde_json::to_value(&track).unwrap()).unwrap();
    let saved = read_clips(&reloaded).unwrap();
    assert_eq!(saved, vec![clip]); assert_eq!(saved[0].id, id);
    assert_eq!(reloaded.unknown["another_extension"]["keep"], true);
    assert!(clips_value(track.id.clone(), &[saved[0].clone(), saved[0].clone()]).is_err());
    write_clips(&mut track, &[]).unwrap();
    assert!(!track.unknown.contains_key(AUDIO_CLIPS_KEY));
    assert!(track.unknown.contains_key("another_extension"));
}

#[test]
fn musical_start_uses_absolute_tempo_and_trim_does_not_shift_source_channels() {
    let bytes = common::float32(&[0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4], 2, 48_000);
    let audio = decode_bytes(&bytes, Some("wav"), ImportLimits::default()).unwrap();
    let track = Track::new("Audio", "blue");
    let mut clip = AudioClip::new(common::source(&audio), track.id.clone(), 1920);
    clip.source_offset_frames = 1; clip.duration_frames = 2; clip.gain = 0.75;
    let tempo = TempoMap::new(&[
        TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: Default::default() },
        TempoPoint { tick: 960, micro_bpm: 60_000_000, unknown: Default::default() },
    ], 960).unwrap();
    let prepared = prepare_clip(&clip, &audio, &tempo, 48_000, 1).unwrap();
    assert_eq!((prepared.start, prepared.frames(), prepared.channels), (72_000, 2, 2));
    assert_eq!(prepared.samples.as_ref(), &[0.2, -0.2, 0.3, -0.3]);
    assert_eq!((prepared.gain, prepared.track), (0.75, 1));
    // Consumers share the same immutable PCM allocation for playback/export.
    let export_samples = prepared.samples.clone();
    assert!(Arc::ptr_eq(&prepared.samples, &export_samples));
    clip.muted = true;
    assert_eq!(prepare_clip(&clip, &audio, &tempo, 48_000, 1).unwrap().gain, 0.0);
}

#[test]
fn full_rate_matched_clip_reuses_pcm_and_source_fingerprint_changes_are_rejected() {
    let audio = decode_bytes(&common::float32(&[0.2; 16], 1, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let track = Track::new("Audio", "blue");
    let mut clip = AudioClip::new(common::source(&audio), track.id.clone(), 0);
    let prepared = prepare_clip(&clip, &audio, &TempoMap::constant(120.0), 48_000, 0).unwrap();
    assert!(Arc::ptr_eq(&prepared.samples, &audio.samples));
    clip.source.sha256.replace_range(..1, if &clip.source.sha256[..1] == "a" { "b" } else { "a" });
    assert!(prepare_clip(&clip, &audio, &TempoMap::constant(120.0), 48_000, 0).is_err());
}

#[test]
fn resampling_is_bandlimited_channel_separate_and_has_correct_dc_edge_gain() {
    for (from, to) in [(16_000, 48_000), (44_100, 48_000), (48_000, 16_000)] {
        let values: Vec<f32> = (0..1024).flat_map(|_| [0.25, -0.5]).collect();
        let audio = decode_bytes(&common::float32(&values, 2, from), Some("wav"), ImportLimits::default()).unwrap();
        let track = Track::new("Audio", "blue");
        let clip = AudioClip::new(common::source(&audio), track.id.clone(), 0);
        let prepared = prepare_clip(&clip, &audio, &TempoMap::constant(120.0), to, 0).unwrap();
        assert_eq!(prepared.frames(), (1024.0 * f64::from(to) / f64::from(from)).round() as u64);
        for frame in [0, prepared.frames() as usize / 2, prepared.frames() as usize - 1] {
            assert!((prepared.samples[frame * 2] - 0.25).abs() < 0.005, "{from}->{to} L frame {frame}");
            assert!((prepared.samples[frame * 2 + 1] + 0.5).abs() < 0.005, "{from}->{to} R frame {frame}");
        }
    }
}

#[test]
fn clip_pan_is_prepared_once_and_stereo_extremes_preserve_the_remaining_channel() {
    let track = Track::new("Audio", "blue");
    let mono = decode_bytes(&common::float32(&[0.5; 4], 1, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let mut clip = AudioClip::new(common::source(&mono), track.id.clone(), 0); clip.pan = 1.0;
    let right = prepare_clip(&clip, &mono, &TempoMap::constant(120.0), 48_000, 0).unwrap();
    assert_eq!((right.channels, right.pan), (2, 0.0));
    assert!(right.samples[0].abs() < 1e-7); assert_eq!(right.samples[1], 0.5);
    let stereo = decode_bytes(&common::float32(&[0.25, -0.5, 0.25, -0.5], 2, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let mut clip = AudioClip::new(common::source(&stereo), track.id.clone(), 0); clip.pan = -1.0;
    let left = prepare_clip(&clip, &stereo, &TempoMap::constant(120.0), 48_000, 0).unwrap();
    assert_eq!(left.samples.as_ref(), &[0.25, 0.0, 0.25, 0.0]);
}

#[test]
fn waveform_preserves_channel_extrema_and_lru_retention_is_bounded() {
    let audio = decode_bytes(&common::float32(&[-1.0, 0.25, 0.5, -0.5, 0.75, 1.0, -0.25, 0.0], 2, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let mut cache = WaveformCache::new(600, 4).unwrap();
    let preview = cache.preview(&audio, 0..4, 2).unwrap();
    assert_eq!((preview.peaks[0].min, preview.peaks[0].max), (-1.0, 0.5));
    assert_eq!((preview.peaks[1].min, preview.peaks[1].max), (-0.5, 0.25));
    assert!(Arc::ptr_eq(&preview, &cache.preview(&audio, 0..4, 2).unwrap()));
    cache.preview(&audio, 1..4, 2).unwrap();
    assert_eq!(cache.len(), 1); assert!(cache.retained_bytes() <= 600);
    assert!(cache.preview(&audio, 0..4, 5).is_err());
}
