//! Offline windowed-sinc resampler (non-real-time; SPEC §10: rate conversion
//! happens before PCM reaches the audio callback).

use std::f64::consts::PI;

const ZERO_CROSSINGS: usize = 24;

fn blackman_harris(x: f64) -> f64 {
    // x in [-1, 1]
    let n = (x + 1.0) / 2.0;
    0.35875 - 0.48829 * (2.0 * PI * n).cos() + 0.14128 * (4.0 * PI * n).cos() - 0.01168 * (6.0 * PI * n).cos()
}

pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    let cutoff = ratio.min(1.0) * 0.97;
    let half = ZERO_CROSSINGS as f64 / cutoff;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let t = i as f64 / ratio;
        let lo = (t - half).ceil().max(0.0) as usize;
        let hi = ((t + half).floor() as usize).min(input.len() - 1);
        let mut acc = 0.0;
        let mut wsum = 0.0;
        for (k, &x) in input.iter().enumerate().take(hi + 1).skip(lo) {
            let d = t - k as f64;
            let arg = d * cutoff;
            let sinc = if arg.abs() < 1e-12 { 1.0 } else { (PI * arg).sin() / (PI * arg) };
            let w = sinc * blackman_harris(d / half) * cutoff;
            acc += x as f64 * w;
            wsum += w;
        }
        // Normalise near the edges where the kernel is truncated.
        let v = if wsum.abs() > 1e-9 && (lo == 0 || hi == input.len() - 1) { acc / wsum } else { acc };
        out.push(v as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(f: f64, sr: u32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2.0 * PI * f * i as f64 / sr as f64).sin() as f32).collect()
    }

    #[test]
    fn preserves_a_tone_across_rates() {
        for (from, to) in [(16000, 48000), (44100, 48000), (48000, 44100)] {
            let x = sine(440.0, from, from as usize / 2);
            let y = resample(&x, from, to);
            assert_eq!(y.len(), (x.len() as f64 * to as f64 / from as f64).round() as usize);
            let reference = sine(440.0, to, y.len());
            // Compare away from the edges.
            let err: f64 = y[200..y.len() - 200]
                .iter()
                .zip(&reference[200..reference.len() - 200])
                .map(|(a, b)| ((a - b) as f64).powi(2))
                .sum::<f64>()
                / (y.len() - 400) as f64;
            assert!(err.sqrt() < 0.01, "{from}->{to} rms err {}", err.sqrt());
        }
    }
}
