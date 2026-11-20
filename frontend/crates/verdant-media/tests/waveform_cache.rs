mod common;

use std::sync::Arc;
use verdant_media::{DecodedAudio, ImportLimits, WaveformCache, WaveformPreview, decode_bytes};

fn audio(samples: &[f32], channels: u16, rate: u32) -> DecodedAudio {
    decode_bytes(&common::float32(samples, channels, rate), Some("wav"), ImportLimits::default()).unwrap()
}

fn assert_exact_windows(audio: &DecodedAudio, preview: &WaveformPreview) {
    let channels = audio.channels as usize;
    for bin in 0..preview.bins {
        let first = preview.start_frame + (u128::from(preview.frame_count) * bin as u128 / preview.bins as u128) as u64;
        let end = preview.start_frame + (u128::from(preview.frame_count) * (bin + 1) as u128 / preview.bins as u128) as u64;
        for channel in 0..channels {
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            let mut squared = 0.0;
            for frame in first..end {
                let value = audio.samples[frame as usize * channels + channel];
                min = min.min(value);
                max = max.max(value);
                squared += f64::from(value) * f64::from(value);
            }
            let peak = preview.peaks[bin * channels + channel];
            assert_eq!((peak.min, peak.max), (min, max), "bin {bin}, channel {channel}");
            let rms = (squared / (end - first) as f64).sqrt() as f32;
            assert!((peak.rms - rms).abs() <= 2.0 * f32::EPSILON * rms.max(1.0), "bin {bin}, channel {channel}");
        }
    }
}

#[test]
fn coarse_zoom_uses_exact_frame_windows_and_trim_and_rate_change_identity() {
    // Odd source length and unaligned trims exercise partial blocks at both
    // edges, odd pyramid levels, bin boundaries and independent stereo peaks.
    let mut samples = (0..4099).flat_map(|frame| [
        ((frame * 37 % 101) as f32 - 50.0) / 64.0,
        ((frame * 13 % 79) as f32 - 39.0) / 64.0,
    ]).collect::<Vec<_>>();
    samples[128 * 2] = -1.0;
    samples[129 * 2] = 0.9375;
    samples[4016 * 2 + 1] = -0.9375;
    samples[4017 * 2 + 1] = 1.0;
    let audio = audio(&samples, 2, 48_000);
    let pcm_owners = Arc::strong_count(&audio.samples);
    let mut cache = WaveformCache::new(64 * 1024, 256).unwrap();
    let original = cache.preview(&audio, 129..4017, 11).unwrap();
    let coarse = cache.preview(&audio, 129..4017, 3).unwrap();
    assert_exact_windows(&audio, &original);
    assert_exact_windows(&audio, &coarse);
    assert!(Arc::ptr_eq(&coarse, &cache.preview(&audio, 129..4017, 3).unwrap()));
    assert_eq!(cache.len(), 1, "zoom levels must share one source pyramid");
    assert!(coarse.peaks.iter().all(|peak| peak.min > -1.0 && peak.max < 1.0));
    let whole_source = cache.preview(&audio, 0..audio.frames(), 1).unwrap();
    assert_exact_windows(&audio, &whole_source);

    // Trim both sides while keeping bin count unchanged. Neither the retained
    // preview nor the newly requested preview may contain the other's edges.
    let trimmed = cache.preview(&audio, 130..4016, 3).unwrap();
    assert!(!Arc::ptr_eq(&coarse, &trimmed));
    assert_eq!((trimmed.start_frame, trimmed.frame_count), (130, 3886));
    assert_exact_windows(&audio, &trimmed);
    assert!(trimmed.peaks.iter().all(|peak| peak.min > -0.9375 && peak.max < 0.9375));
    assert_exact_windows(&audio, &coarse);
    assert!(Arc::ptr_eq(&coarse, &cache.preview(&audio, 129..4017, 3).unwrap()), "undoing a trim can reuse its exact preview");
    assert_eq!(cache.len(), 1);

    let mut other_rate = audio.clone();
    other_rate.sample_rate = 44_100;
    let at_other_rate = cache.preview(&other_rate, 129..4017, 3).unwrap();
    assert!(!Arc::ptr_eq(&coarse, &at_other_rate));
    assert_eq!(at_other_rate.sample_rate, 44_100);
    assert_eq!(cache.len(), 2);
    assert_eq!(Arc::strong_count(&audio.samples), pcm_owners + 1, "only the explicit DecodedAudio clone may retain PCM");
}

#[test]
fn source_lru_evicts_the_least_recently_used_source_and_preserves_live_previews() {
    let a = audio(&[-0.75, 0.25, 0.5, -0.5], 1, 48_000);
    let b = audio(&[-0.25, 0.75, 0.5, -0.5], 1, 48_000);
    let c = audio(&[-0.5, 0.5, 0.75, -0.25], 1, 48_000);
    let mut sizing = WaveformCache::new(64 * 1024, 8).unwrap();
    sizing.preview(&a, 0..4, 2).unwrap();
    let budget = sizing.retained_bytes() * 2;
    let mut cache = WaveformCache::new(budget, 8).unwrap();
    let live_a = cache.preview(&a, 0..4, 2).unwrap();
    let live_b = cache.preview(&b, 0..4, 2).unwrap();
    let b_peaks = live_b.peaks.clone();
    assert_eq!(cache.len(), 2);
    // Reading A must refresh source recency, even on an exact preview hit.
    assert!(Arc::ptr_eq(&live_a, &cache.preview(&a, 0..4, 2).unwrap()));
    let live_c = cache.preview(&c, 0..4, 2).unwrap();
    assert_eq!(cache.len(), 2);
    assert!(cache.retained_bytes() <= budget);
    assert!(Arc::ptr_eq(&live_a, &cache.preview(&a, 0..4, 2).unwrap()), "recent source A must survive C");
    let reloaded_b = cache.preview(&b, 0..4, 2).unwrap();
    assert!(!Arc::ptr_eq(&live_b, &reloaded_b), "least recent source B must have been evicted");
    assert_eq!(live_b.peaks, b_peaks, "eviction must not change a live preview");
    assert_eq!(live_b.peaks, reloaded_b.peaks);
    assert_exact_windows(&a, &live_a);
    assert_exact_windows(&b, &live_b);
    assert_exact_windows(&c, &live_c);
    assert!(cache.retained_bytes() <= budget);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.retained_bytes(), 0);
    assert_exact_windows(&b, &live_b);
}

#[test]
fn tiny_budget_discards_old_trim_previews_and_keeps_source_and_live_peaks() {
    let audio = audio(&[-1.0, 0.25, 0.5, -0.5, 0.75, 1.0, -0.25, 0.0], 2, 48_000);
    let mut cache = WaveformCache::new(600, 4).unwrap();
    let original = cache.preview(&audio, 0..4, 2).unwrap();
    let trimmed = cache.preview(&audio, 1..4, 2).unwrap();
    assert_eq!(cache.len(), 1);
    assert!(cache.retained_bytes() <= 600);
    assert_exact_windows(&audio, &original);
    assert_exact_windows(&audio, &trimmed);
    assert!(!Arc::ptr_eq(&original, &cache.preview(&audio, 0..4, 2).unwrap()));
    assert!(cache.preview(&audio, 0..4, 5).is_err());
}
