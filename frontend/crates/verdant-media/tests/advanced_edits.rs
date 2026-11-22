mod common;

use verdant_domain::{TempoMap, Track};
use verdant_media::{AudioClip, FadeShape, ImportLimits, decode_bytes, prepare_clip};

fn clip(samples: &[f32], channels: u16) -> (AudioClip, verdant_media::DecodedAudio) {
    let audio = decode_bytes(&common::float32(samples, channels, 48_000), Some("wav"), ImportLimits::default()).unwrap();
    let track = Track::new("Audio", "blue");
    (AudioClip::new(common::source(&audio), track.id, 0), audio)
}

#[test]
fn legacy_clip_defaults_and_fractional_source_frame_cuts_roundtrip() {
    let (clip, _) = clip(&vec![0.5; 48_000], 1);
    let mut old = serde_json::to_value(&clip).unwrap();
    for key in ["name", "start_subtick", "playback_rate", "preserve_pitch", "reversed", "fade_shape", "fade_window"] {
        old.as_object_mut().unwrap().remove(key);
    }
    assert_eq!(serde_json::from_value::<AudioClip>(old).unwrap(), clip);
    let tempo = TempoMap::constant(123.0);
    let (left, right) = clip.split_at(clip.tick_at_frame(12_347, &tempo), &tempo).unwrap();
    assert_eq!(left.duration_frames, 12_347);
    assert!(right.start_subtick > 0.0);
    assert!((left.end_seconds(&tempo) - right.start_seconds(&tempo)).abs() < 1e-12);
    assert_eq!(left.join_right(&right, &tempo).unwrap(), clip);
    let encoded = serde_json::to_string(&right).unwrap();
    assert_eq!(serde_json::from_str::<AudioClip>(&encoded).unwrap(), right);
}

#[test]
fn splitting_a_fade_or_reversed_region_preserves_every_retained_sample() {
    let input: Vec<f32> = (0..2048).flat_map(|i| [i as f32 / 2048.0, -(i as f32 / 2048.0)]).collect();
    let (mut clip, audio) = clip(&input, 2);
    clip.fade_in_frames = 1500; clip.fade_out_frames = 1800;
    clip.fade_shape = FadeShape::EqualPower;
    let tempo = TempoMap::constant(117.0);
    for reversed in [false, true] {
        clip.reversed = reversed;
        let original = prepare_clip(&clip, &audio, &tempo, 48_000, 0).unwrap();
        let (left, right) = clip.split_at(clip.tick_at_frame(723, &tempo), &tempo).unwrap();
        let a = prepare_clip(&left, &audio, &tempo, 48_000, 0).unwrap();
        let b = prepare_clip(&right, &audio, &tempo, 48_000, 0).unwrap();
        assert_eq!(a.end(), b.start);
        assert_eq!([a.samples.as_ref(), b.samples.as_ref()].concat(), original.samples.as_ref());
        assert_eq!(left.join_right(&right, &tempo).unwrap(), clip);
        assert_eq!(audio.samples.as_ref(), input);
    }
}

fn tone_frequency(samples: &[f32], channels: usize, rate: f64) -> f64 {
    let frames = samples.len() / channels;
    let (begin, end) = (frames / 5, frames * 4 / 5);
    let crossings: Vec<f64> = (begin + 1..end).filter_map(|i| {
        let (a, b) = (samples[(i - 1) * channels] as f64, samples[i * channels] as f64);
        if a < 0.0 && b >= 0.0 { Some(i as f64 - b / (b - a)) } else { None }
    }).collect();
    (crossings.len() - 1) as f64 * rate / (crossings.last().unwrap() - crossings[0])
}

#[test]
fn varispeed_and_channel_linked_pitch_preserving_stretch_have_correct_duration_and_pitch() {
    let input: Vec<f32> = (0..48_000).flat_map(|i| {
        let sample = (i as f32 * std::f32::consts::TAU * 440.0 / 48_000.0).sin() * 0.5;
        [sample, -sample]
    }).collect();
    let (mut clip, audio) = clip(&input, 2);
    let tempo = TempoMap::constant(120.0);
    for speed in [0.75, 1.5] {
        clip.playback_rate = speed;
        clip.preserve_pitch = false;
        let resampled = prepare_clip(&clip, &audio, &tempo, 48_000, 0).unwrap();
        assert_eq!(resampled.frames(), (48_000.0 / speed).round() as u64);
        assert!((tone_frequency(&resampled.samples, 2, 48_000.0) - 440.0 * speed).abs() < 0.2);
        clip.preserve_pitch = true;
        let stretched = prepare_clip(&clip, &audio, &tempo, 48_000, 0).unwrap();
        assert_eq!(stretched.frames(), (48_000.0 / speed).round() as u64);
        let frequency = tone_frequency(&stretched.samples, 2, 48_000.0);
        assert!((frequency - 440.0).abs() < 2.0, "pitch {frequency}, speed {speed}");
        assert!(stretched.samples.chunks_exact(2).all(|frame| frame[0].is_finite() && (frame[0] + frame[1]).abs() < 1e-6));
    }
}

#[test]
fn varispeed_applies_antialias_filter_and_revealing_source_frames_is_nondestructive() {
    let input: Vec<f32> = (0..48_000).map(|i| (i as f32 * std::f32::consts::TAU * 14_000.0 / 48_000.0).sin() * 0.5).collect();
    let (mut clip, audio) = clip(&input, 1);
    let tempo = TempoMap::constant(120.0);
    let trimmed = clip.trim_source(100, 1000, 15.0, &tempo).unwrap();
    let restored = trimmed.trim_source(0, 48_000, 0.0, &tempo).unwrap();
    assert_eq!(restored, clip);
    clip.playback_rate = 3.0;
    let prepared = prepare_clip(&clip, &audio, &tempo, 48_000, 0).unwrap();
    let body = &prepared.samples[100..prepared.samples.len()-100];
    let rms = (body.iter().map(|sample| sample * sample).sum::<f32>() / body.len() as f32).sqrt();
    assert!(rms < 0.01, "aliased RMS {rms}");
    for invalid in [f64::NAN, f64::INFINITY, 0.0, -1.0, 32.0] { clip.playback_rate = invalid; assert!(clip.validate().is_err()); }
}

#[test]
fn restoring_a_trimmed_fade_recovers_the_original_envelope_and_sample_count() {
    let (mut clip,audio)=clip(&vec![0.25;48_000],1);
    clip.fade_in_frames=20_000;clip.fade_out_frames=25_000;clip.reversed=true;
    let tempo=TempoMap::constant(127.0);
    let trimmed=clip.slice_frames(1000,40_001,&tempo).unwrap();
    let restored=trimmed.trim_source(clip.source_offset_frames,clip.duration_frames,clip.tick(),&tempo).unwrap();
    assert_eq!(restored,clip);
    let original=prepare_clip(&clip,&audio,&tempo,44_100,0).unwrap();
    assert_eq!(prepare_clip(&restored,&audio,&tempo,44_100,0).unwrap().samples,original.samples);
}

#[test]
fn split_regions_share_an_exact_engine_boundary_at_every_output_rate() {
    let (mut clip,audio)=clip(&vec![0.25;48_001],1);
    clip.set_tick(123.456).unwrap();
    let tempo=TempoMap::constant(137.0);
    for rate in [16_000,44_100,48_000,96_000] {
        for speed in [0.75,1.0,1.37,2.5] {
            clip.playback_rate=speed;
            let (left,right)=clip.split_at(clip.tick_at_frame(12_347,&tempo),&tempo).unwrap();
            let a=prepare_clip(&left,&audio,&tempo,rate,0).unwrap();
            let b=prepare_clip(&right,&audio,&tempo,rate,0).unwrap();
            let whole=prepare_clip(&clip,&audio,&tempo,rate,0).unwrap();
            assert_eq!(a.end(),b.start,"rate {rate}, speed {speed}");
            assert_eq!(a.frames()+b.frames(),whole.frames(),"rate {rate}, speed {speed}");
        }
    }
}
