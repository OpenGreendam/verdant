//! Offline, channel-linked time scaling. No allocation or analysis runs in the
//! realtime callback. WSOLA follows the waveform-similarity overlap-add method:
//! https://www.isca-archive.org/eurospeech_1993/roelands93_eurospeech.html
//! It preserves local pitch and stereo phase; extreme ratios can repeat/smear
//! transients, so quality is not represented as a substitute for auditioning.
use crate::{ImportLimits, MediaError};

/// Arbitrary-ratio bandlimited conversion for varispeed. Every output channel
/// uses the same fractional source position and the same normalized kernel.
pub(crate) fn resample_to_frames(input: &[f32], channels: u16, frames: u64, limits: ImportLimits) -> Result<Vec<f32>, MediaError> {
    let channel_count = channels as usize;
    let source_frames = input.len() / channel_count;
    let len = limits.pcm_len(frames, channels)?;
    if frames == 0 || source_frames == 0 { return Err(MediaError::Invalid("empty time-scaled audio")); }
    if frames as usize == source_frames { return Ok(input.to_vec()); }
    let step = source_frames as f64 / frames as f64;
    let cutoff = (1.0 / step).min(1.0) * 0.96;
    let radius = (24.0 / cutoff).ceil().min(512.0) as i64;
    let mut output = vec![0.0; len];
    for (i, frame) in output.chunks_exact_mut(channel_count).enumerate() {
        let position = i as f64 * step;
        let center = position.floor() as i64;
        let mut weight_sum = 0.0;
        let mut sum = [0.0_f64; 2];
        for j in center - radius + 1..=center + radius {
            let x = j as f64 - position;
            let phase = std::f64::consts::PI * x * cutoff;
            let sinc = if phase.abs() < 1e-12 { 1.0 } else { phase.sin() / phase };
            let window = 0.5 + 0.5 * (std::f64::consts::PI * x / radius as f64).cos();
            let weight = cutoff * sinc * window;
            let index = j.clamp(0, source_frames as i64 - 1) as usize * channel_count;
            for c in 0..channel_count { sum[c] += input[index + c] as f64 * weight; }
            weight_sum += weight;
        }
        for c in 0..channel_count { frame[c] = (sum[c] / weight_sum) as f32; }
    }
    Ok(output)
}

pub(crate) fn preserve_pitch(input: &[f32], channels: u16, sample_rate: u32, rate: f64, limits: ImportLimits) -> Result<Vec<f32>, MediaError> {
    let channels = channels as usize;
    let source_frames = input.len() / channels;
    let output_frames = (source_frames as f64 / rate).round().max(1.0) as u64;
    let len = limits.pcm_len(output_frames, channels as u16)?;
    if output_frames == 0 { return Err(MediaError::Invalid("time stretch shorter than one source frame")); }
    if rate == 1.0 { return Ok(input.to_vec()); }
    let frames = output_frames as usize;
    if len.checked_mul(4).and_then(|n| frames.checked_mul(4).and_then(|weights| n.checked_add(weights))).is_none_or(|n| n > limits.max_pcm_bytes) {
        return Err(MediaError::Limit("time-stretch workspace"));
    }
    // A sub-period excerpt does not contain enough information to infer pitch.
    // Repeating that excerpt is deterministic and preserves its sample clock.
    if source_frames < 32 {
        return Ok((0..len).map(|i| input[i % input.len()]).collect());
    }
    let grain = ((sample_rate as usize * 40 / 1000).clamp(64, 8192)).min(source_frames);
    let hop = (grain / 2).max(1);
    let search = (sample_rate as usize * 12 / 1000).clamp(8, 4096);
    let mut output = vec![0.0_f32; len];
    let mut weights = vec![0.0_f32; frames];
    let window: Vec<f32> = (0..grain).map(|i| (0.5 - 0.5 * (2.0 * std::f64::consts::PI * (i as f64 + 0.5) / grain as f64).cos()) as f32).collect();
    let max_start = source_frames - grain;
    let mut previous = 0usize;
    for out in (0..frames).step_by(hop) {
        let expected = ((out as f64 * rate).round() as usize).min(max_start);
        let chosen = if out == 0 || max_start == 0 { 0 } else {
            let low = expected.saturating_sub(search);
            let high = expected.saturating_add(search).min(max_start);
            let reference = (previous + hop).min(source_frames - hop);
            let score = |candidate: usize, stride: usize| -> f64 {
                let (mut xy, mut xx, mut yy) = (0.0_f64, 0.0_f64, 0.0_f64);
                for i in (0..hop).step_by(stride) {
                    for channel in 0..channels {
                        let a = input[(reference + i) * channels + channel] as f64;
                        let b = input[(candidate + i) * channels + channel] as f64;
                        xy += a * b; xx += a * a; yy += b * b;
                    }
                }
                let correlation = if xx * yy <= 1e-24 { 0.0 } else { xy / (xx * yy).sqrt() };
                correlation - candidate.abs_diff(expected) as f64 * 1e-7
            };
            let coarse_step = (sample_rate as usize / 12_000).clamp(1, 8);
            let mut best = expected;
            let mut best_score = score(best, 4);
            for candidate in (low..=high).step_by(coarse_step) {
                let value = score(candidate, 4);
                if value > best_score { best_score = value; best = candidate; }
            }
            let fine_low = best.saturating_sub(coarse_step).max(low);
            let fine_high = best.saturating_add(coarse_step).min(high);
            best_score = score(best, 1);
            for candidate in fine_low..=fine_high {
                let value = score(candidate, 1);
                if value > best_score { best_score = value; best = candidate; }
            }
            best
        };
        previous = chosen;
        for i in 0..grain.min(frames - out) {
            let weight = window[i];
            weights[out + i] += weight;
            for channel in 0..channels {
                output[(out + i) * channels + channel] += input[(chosen + i) * channels + channel] * weight;
            }
        }
    }
    for (i, frame) in output.chunks_exact_mut(channels).enumerate() {
        let weight = weights[i].max(1e-12);
        for sample in frame { *sample /= weight; }
    }
    Ok(output)
}
