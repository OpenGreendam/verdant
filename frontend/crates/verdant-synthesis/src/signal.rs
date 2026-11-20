//! Numerical kernels shared by production synthesis stages.
//!
//! Polyphase interpolation follows scipy.signal.resample_poly's reduced ratio,
//! 20*max(up,down)+1 Kaiser FIR, zero extension and centered delay removal.
//! STFT cells retain the whole-waveform Hann grid and spectrum normalization;
//! inverse transforms overlap-add chronologically, with no independent blocks.
use anyhow::{ensure, Result};
use rustfft::{Fft, FftPlanner};
pub use rustfft::num_complex::Complex64;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::sync::Arc;

thread_local! {
    static FFT_PLANNER: RefCell<FftPlanner<f64>> = RefCell::new(FftPlanner::new());
}

fn fft_plan(n: usize, inverse: bool) -> Arc<dyn Fft<f64>> {
    FFT_PLANNER.with(|planner| {
        if inverse { planner.borrow_mut().plan_fft_inverse(n) }
        else { planner.borrow_mut().plan_fft_forward(n) }
    })
}

fn finite(x: &[f64]) -> Result<()> {
    ensure!(x.iter().all(|v| v.is_finite()), "signal contains nonfinite values");
    Ok(())
}

/// Real forward FFT, with NumPy's unnormalized forward convention and zero padding.
pub fn rfft(x: &[f64], n: usize) -> Result<Vec<Complex64>> {
    ensure!(n > 0, "FFT size must be positive");
    finite(x)?;
    let mut work = vec![Complex64::new(0., 0.); n];
    for (z, &v) in work.iter_mut().zip(x) { z.re = v; }
    fft_plan(n, false).process(&mut work);
    Ok(work[..n / 2 + 1].to_vec())
}

fn inverse_into(z: &[Complex64], n: usize, work: &mut [Complex64], plan: &dyn Fft<f64>, scratch: &mut [Complex64]) {
    work.fill(Complex64::new(0., 0.));
    let bins = n / 2 + 1;
    work[..bins].copy_from_slice(z);
    work[0].im = 0.;
    if n % 2 == 0 { work[n / 2].im = 0.; }
    for k in 1..=(n - 1) / 2 { work[n - k] = work[k].conj(); }
    plan.process_with_scratch(work, scratch);
}

/// Real inverse FFT. The input must contain exactly floor(n/2)+1 bins.
pub fn irfft(z: &[Complex64], n: usize) -> Result<Vec<f64>> {
    ensure!(n > 0 && z.len() == n / 2 + 1, "inverse FFT shape mismatch");
    ensure!(z.iter().all(|v| v.re.is_finite() && v.im.is_finite()), "nonfinite inverse FFT input");
    let plan = fft_plan(n, true);
    let mut work = vec![Complex64::new(0., 0.); n];
    let mut scratch = vec![Complex64::new(0., 0.); plan.get_inplace_scratch_len()];
    inverse_into(z, n, &mut work, plan.as_ref(), &mut scratch);
    Ok(work.into_iter().map(|v| v.re / n as f64).collect())
}

pub fn hann(n: usize, periodic: bool) -> Vec<f64> {
    if n == 0 { return Vec::new(); }
    if n == 1 { return vec![1.]; }
    let denominator = if periodic { n } else { n - 1 } as f64;
    (0..n).map(|i| 0.5 - 0.5 * (2. * PI * i as f64 / denominator).cos()).collect()
}

fn bessel_i0(x: f64) -> f64 {
    let mut term = 1.;
    let mut sum = 1.;
    let quarter_square = x * x * 0.25;
    for k in 1..10000 {
        term *= quarter_square / (k as f64 * k as f64);
        sum += term;
        if term <= sum * f64::EPSILON || !sum.is_finite() { break; }
    }
    sum
}

fn sinc(x: f64) -> f64 {
    if x == 0. { 1. } else { (PI * x).sin() / (PI * x) }
}

/// scipy.signal.firwin's scaled, symmetric Kaiser-windowed FIR.
pub fn firwin(taps: usize, cutoffs: &[f64], rate: f64, pass_zero: bool, beta: f64) -> Result<Vec<f64>> {
    ensure!(taps > 0 && rate.is_finite() && rate > 0., "positive FIR length and rate required");
    ensure!(!cutoffs.is_empty() && cutoffs.iter().all(|v| v.is_finite() && *v > 0. && *v < rate * 0.5), "FIR cutoffs must be inside Nyquist");
    ensure!(cutoffs.windows(2).all(|w| w[0] < w[1]), "FIR cutoffs must increase strictly");
    ensure!(beta.is_finite() && beta >= 0., "Kaiser beta must be finite and nonnegative");
    let pass_nyquist = (cutoffs.len() % 2 == 0) == pass_zero;
    ensure!(!pass_nyquist || taps % 2 == 1, "even FIR cannot pass Nyquist");
    let mut edges = Vec::with_capacity(cutoffs.len() + 2);
    if pass_zero { edges.push(0.); }
    edges.extend(cutoffs.iter().map(|c| c / (rate * 0.5)));
    if pass_nyquist { edges.push(1.); }
    let alpha = (taps - 1) as f64 * 0.5;
    let denominator = bessel_i0(beta);
    ensure!(denominator.is_finite(), "Kaiser beta overflows coefficient precision");
    let mut result = Vec::with_capacity(taps);
    for i in 0..taps {
        let m = i as f64 - alpha;
        let mut h = 0.;
        for band in edges.chunks_exact(2) {
            h += band[1] * sinc(band[1] * m);
            h -= band[0] * sinc(band[0] * m);
        }
        let ratio = if taps == 1 { 0. } else { m / alpha };
        h *= bessel_i0(beta * (1. - ratio * ratio).max(0.).sqrt()) / denominator;
        result.push(h);
    }
    let scale_frequency = if edges[0] == 0. { 0. }
        else if edges[1] == 1. { 1. }
        else { (edges[0] + edges[1]) * 0.5 };
    let scale: f64 = result.iter().enumerate().map(|(i, v)| v * (PI * (i as f64 - alpha) * scale_frequency).cos()).sum();
    ensure!(scale.is_finite() && scale != 0., "FIR normalization has no finite passband");
    for h in &mut result { *h /= scale; }
    Ok(result)
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 { let remainder = a % b; a = b; b = remainder; }
    a
}

pub fn resample_poly(x: &[f64], up: usize, down: usize, beta: f64) -> Result<Vec<f64>> {
    ensure!(up > 0 && down > 0, "polyphase rates must be positive integers");
    finite(x)?;
    if x.is_empty() { return Ok(Vec::new()); }
    let divisor = gcd(up, down);
    let (up, down) = (up / divisor, down / divisor);
    if up == 1 && down == 1 { return Ok(x.to_vec()); }
    let maximum = up.max(down);
    let taps = maximum.checked_mul(20).and_then(|n| n.checked_add(1)).ok_or_else(|| anyhow::anyhow!("polyphase filter length overflow"))?;
    let h = firwin(taps, &[1. / maximum as f64], 2., true, beta)?;
    resample_poly_with_taps(x, up, down, &h)
}

/// Explicit taps retain SciPy's reduced-ratio up-scaling and centered alignment.
pub fn resample_poly_with_taps(x: &[f64], up: usize, down: usize, taps: &[f64]) -> Result<Vec<f64>> {
    ensure!(up > 0 && down > 0 && !taps.is_empty(), "positive rates and nonempty FIR required");
    finite(x)?;
    finite(taps)?;
    if x.is_empty() { return Ok(Vec::new()); }
    let divisor = gcd(up, down);
    let (up, down) = (up / divisor, down / divisor);
    if up == 1 && down == 1 { return Ok(x.to_vec()); }
    let count = x.len().checked_mul(up).and_then(|n| n.checked_add(down - 1)).ok_or_else(|| anyhow::anyhow!("resampled length overflow"))? / down;
    let half = (taps.len() - 1) / 2;
    let prepad = down - half % down;
    let remove = (half + prepad) / down;
    let mut h = vec![0.; prepad];
    h.extend(taps.iter().map(|v| v * up as f64));
    let mut result = vec![0.; count];
    for (output, value) in result.iter_mut().enumerate() {
        let at = (output + remove).checked_mul(down).ok_or_else(|| anyhow::anyhow!("resampling index overflow"))?;
        let first = if at >= h.len() { (at - h.len() + up) / up } else { 0 };
        let last = (at / up).min(x.len() - 1);
        if first <= last {
            let mut sum = 0.;
            for source in first..=last { sum += x[source] * h[at - source * up]; }
            *value = sum;
        }
    }
    Ok(result)
}

fn fast_length(n: usize) -> Result<usize> {
    let mut best = n.checked_next_power_of_two().ok_or_else(|| anyhow::anyhow!("convolution size overflow"))?;
    let mut two = 1usize;
    while two < best {
        let mut three = two;
        while three < best {
            let mut five = three;
            while five < n { five = match five.checked_mul(5) { Some(v) => v, None => break }; }
            if five >= n { best = best.min(five); }
            three = match three.checked_mul(3) { Some(v) => v, None => break };
        }
        two = match two.checked_mul(2) { Some(v) => v, None => break };
    }
    Ok(best)
}

/// Centered linear convolution, retaining exactly the first operand's length.
pub fn fftconvolve_same(x: &[f64], h: &[f64]) -> Result<Vec<f64>> {
    ensure!(!x.is_empty() && !h.is_empty(), "convolution requires nonempty vectors");
    finite(x)?; finite(h)?;
    let full = x.len().checked_add(h.len() - 1).ok_or_else(|| anyhow::anyhow!("convolution length overflow"))?;
    let n = fast_length(full)?;
    let forward = fft_plan(n, false);
    let inverse = fft_plan(n, true);
    let mut a = vec![Complex64::new(0., 0.); n];
    let mut b = vec![Complex64::new(0., 0.); n];
    let scratch_len = forward.get_inplace_scratch_len().max(inverse.get_inplace_scratch_len());
    let mut scratch = vec![Complex64::new(0., 0.); scratch_len];
    for (v, &sample) in a.iter_mut().zip(x) { v.re = sample; }
    for (v, &sample) in b.iter_mut().zip(h) { v.re = sample; }
    forward.process_with_scratch(&mut a, &mut scratch);
    forward.process_with_scratch(&mut b, &mut scratch);
    for (v, filter) in a.iter_mut().zip(&b) { *v *= filter; }
    drop(b);
    inverse.process_with_scratch(&mut a, &mut scratch);
    let first = (h.len() - 1) / 2;
    Ok(a[first..first + x.len()].iter().map(|v| v.re / n as f64).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderMode { Nearest, Reflect }

fn border_index(index: isize, n: usize, mode: BorderMode) -> usize {
    if mode == BorderMode::Nearest { return index.clamp(0, n as isize - 1) as usize; }
    let period = 2 * n as isize;
    let reflected = index.rem_euclid(period);
    if reflected < n as isize { reflected as usize } else { (period - 1 - reflected) as usize }
}

/// Reusable normalized Gaussian taps, avoiding per-cell design/allocation.
pub struct GaussianKernel { radius: isize, weights: Vec<f64> }

impl GaussianKernel {
    pub fn new(sigma: f64) -> Result<Self> {
        ensure!(sigma.is_finite() && sigma > 0., "Gaussian sigma must be finite and positive");
        let radius_f = (4. * sigma + 0.5).floor();
        ensure!(radius_f < isize::MAX as f64 / 2., "Gaussian support overflow");
        let radius = radius_f as isize;
        let mut weights: Vec<f64> = (-radius..=radius).map(|i| (-0.5 / (sigma * sigma) * (i as f64 * i as f64)).exp()).collect();
        let sum: f64 = weights.iter().sum();
        for weight in &mut weights { *weight /= sum; }
        Ok(Self { radius, weights })
    }

    pub fn apply(&self, x: &[f64], mode: BorderMode, result: &mut [f64]) -> Result<()> {
        ensure!(!x.is_empty() && result.len() == x.len(), "aligned nonempty Gaussian vectors required");
        finite(x)?;
        for (i, output) in result.iter_mut().enumerate() {
            // ndimage's symmetric correlate1d visits center, outer pairs, inner pairs.
            let mut value = x[i] * self.weights[self.radius as usize];
            for k in (1..=self.radius).rev() {
                value += (x[border_index(i as isize - k, x.len(), mode)] + x[border_index(i as isize + k, x.len(), mode)]) * self.weights[(self.radius - k) as usize];
            }
            *output = value;
        }
        Ok(())
    }
}

/// Correlation by a symmetric normalized Gaussian, SciPy truncate=4/order=0.
pub fn gaussian_filter1d(x: &[f64], sigma: f64, mode: BorderMode) -> Result<Vec<f64>> {
    let kernel = GaussianKernel::new(sigma)?;
    let mut result = vec![0.; x.len()];
    kernel.apply(x, mode, &mut result)?;
    Ok(result)
}

fn extrema_filter(x: &[f64], size: usize, mode: BorderMode, maximum: bool) -> Result<Vec<f64>> {
    ensure!(!x.is_empty() && size > 0, "positive extrema width and nonempty vector required");
    finite(x)?;
    ensure!(size <= isize::MAX as usize && x.len() <= isize::MAX as usize - size, "extrema support overflow");
    let left = size / 2;
    let right = size - left - 1;
    let mut queue: VecDeque<(isize, f64)> = VecDeque::new();
    let mut result = Vec::with_capacity(x.len());
    for index in -(left as isize)..x.len() as isize + right as isize {
        let value = x[border_index(index, x.len(), mode)];
        while queue.back().is_some_and(|(_, old)| if maximum { *old <= value } else { *old >= value }) { queue.pop_back(); }
        queue.push_back((index, value));
        let first = index - size as isize + 1;
        while queue.front().is_some_and(|(old, _)| *old < first) { queue.pop_front(); }
        if index >= right as isize { result.push(queue.front().unwrap().1); }
    }
    Ok(result)
}

pub fn maximum_filter1d(x: &[f64], size: usize, mode: BorderMode) -> Result<Vec<f64>> { extrema_filter(x, size, mode, true) }
pub fn minimum_filter1d(x: &[f64], size: usize, mode: BorderMode) -> Result<Vec<f64>> { extrema_filter(x, size, mode, false) }

pub fn uniform_filter1d(x: &[f64], size: usize, mode: BorderMode) -> Result<Vec<f64>> {
    ensure!(!x.is_empty() && size > 0 && size < isize::MAX as usize, "positive uniform width and nonempty vector required");
    finite(x)?;
    let left = size / 2;
    let right = size - left - 1;
    let mut sum: f64 = (-(left as isize)..=right as isize).map(|i| x[border_index(i, x.len(), mode)]).sum();
    let mut result = Vec::with_capacity(x.len());
    result.push(sum / size as f64);
    for i in 1..x.len() {
        sum += x[border_index(i as isize + right as isize, x.len(), mode)] - x[border_index(i as isize - left as isize - 1, x.len(), mode)];
        result.push(sum / size as f64);
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StftBoundary { Zeros, Reflect, None }

/// One-sided, frame-major spectrum. Values are indexed frame*bins+frequency.
#[derive(Clone, Debug)]
pub struct Spectrum {
    pub fft_size: usize,
    pub hop: usize,
    pub frames: usize,
    pub bins: usize,
    pub values: Vec<Complex64>,
}

impl Spectrum {
    pub fn frame(&self, frame: usize) -> &[Complex64] { &self.values[frame * self.bins..(frame + 1) * self.bins] }
    pub fn frame_mut(&mut self, frame: usize) -> &mut [Complex64] { &mut self.values[frame * self.bins..(frame + 1) * self.bins] }
}

fn reflect_whole(index: isize, n: usize) -> usize {
    if n == 1 { return 0; }
    let period = 2 * (n - 1) as isize;
    let at = index.rem_euclid(period);
    if at < n as isize { at as usize } else { (period - at) as usize }
}

pub fn stft_hann(x: &[f64], fft_size: usize, hop: usize, boundary: StftBoundary, padded: bool) -> Result<Spectrum> {
    ensure!(!x.is_empty() && fft_size > 0 && hop > 0 && hop <= fft_size, "valid nonempty STFT geometry required");
    finite(x)?;
    let extension = if boundary == StftBoundary::None { 0 } else { fft_size / 2 };
    let extended = x.len().checked_add(2 * extension).ok_or_else(|| anyhow::anyhow!("STFT length overflow"))?;
    ensure!(extended >= fft_size, "STFT input is shorter than its declared window");
    let tail = if padded { (hop - (extended - fft_size) % hop) % hop } else { 0 };
    let frames = (extended + tail - fft_size) / hop + 1;
    let bins = fft_size / 2 + 1;
    let count = frames.checked_mul(bins).ok_or_else(|| anyhow::anyhow!("STFT shape overflow"))?;
    let mut result = Spectrum { fft_size, hop, frames, bins, values: Vec::with_capacity(count) };
    let window = hann(fft_size, true);
    let scale = 1. / window.iter().sum::<f64>();
    let plan = fft_plan(fft_size, false);
    let mut work = vec![Complex64::new(0., 0.); fft_size];
    let mut scratch = vec![Complex64::new(0., 0.); plan.get_inplace_scratch_len()];
    for frame in 0..frames {
        for sample in 0..fft_size {
            let at = frame * hop + sample;
            let original = at as isize - extension as isize;
            let value = if (0..x.len() as isize).contains(&original) { x[original as usize] }
                else if at >= extended { 0. }
                else if boundary == StftBoundary::Reflect { x[reflect_whole(original, x.len())] }
                else { 0. };
            work[sample] = Complex64::new(value * window[sample], 0.);
        }
        plan.process_with_scratch(&mut work, &mut scratch);
        result.values.extend(work[..bins].iter().map(|v| v * scale));
    }
    Ok(result)
}

/// Hann inverse STFT using the same chronological OLA as scipy.signal.istft.
pub fn istft_hann(z: &Spectrum, samples: usize, boundary: bool) -> Result<Vec<f64>> {
    istft_hann_modified(z, samples, boundary, None, None)
}

/// Inverse STFT with frame-aligned gain/subtraction, without a whole-spectrum copy.
pub fn istft_hann_modified(z: &Spectrum, samples: usize, boundary: bool, multiplier: Option<&[f64]>, subtract: Option<&Spectrum>) -> Result<Vec<f64>> {
    ensure!(z.fft_size > 0 && z.hop > 0 && z.hop <= z.fft_size && z.frames > 0 && z.bins == z.fft_size / 2 + 1 && z.values.len() == z.frames * z.bins, "inverse STFT geometry mismatch");
    ensure!(z.values.iter().all(|v| v.re.is_finite() && v.im.is_finite()), "nonfinite STFT spectrum");
    if let Some(gain) = multiplier { ensure!(gain.len() == z.values.len() && gain.iter().all(|v| v.is_finite()), "STFT gain shape mismatch"); }
    if let Some(sub) = subtract { ensure!(sub.fft_size == z.fft_size && sub.hop == z.hop && sub.frames == z.frames && sub.bins == z.bins && sub.values.len() == z.values.len(), "STFT subtraction shape mismatch"); }
    let length = z.fft_size + (z.frames - 1) * z.hop;
    let trim = if boundary { z.fft_size / 2 } else { 0 };
    ensure!(length >= 2 * trim && samples <= length - 2 * trim, "inverse STFT output length exceeds global grid");
    let window = hann(z.fft_size, true);
    let scale = window.iter().sum::<f64>() / z.fft_size as f64;
    let mut output = vec![0.; length];
    let mut norm = vec![0.; length];
    let mut work = vec![Complex64::new(0., 0.); z.fft_size];
    let mut modified = vec![Complex64::new(0., 0.); z.bins];
    let plan = fft_plan(z.fft_size, true);
    let mut scratch = vec![Complex64::new(0., 0.); plan.get_inplace_scratch_len()];
    for frame in 0..z.frames {
        for (bin, value) in modified.iter_mut().enumerate() {
            let index = frame * z.bins + bin;
            *value = z.values[index];
            if let Some(gain) = multiplier { *value *= gain[index]; }
            if let Some(sub) = subtract { *value -= sub.values[index]; }
        }
        inverse_into(&modified, z.fft_size, &mut work, plan.as_ref(), &mut scratch);
        for sample in 0..z.fft_size {
            let at = frame * z.hop + sample;
            output[at] += work[sample].re * scale * window[sample];
            norm[at] += window[sample] * window[sample];
        }
    }
    for sample in trim..trim + samples { output[sample] /= if norm[sample] > 1e-10 { norm[sample] } else { 1. }; }
    output.copy_within(trim..trim + samples, 0);
    output.truncate(samples);
    Ok(output)
}

pub fn peak(x: &[f64]) -> f64 { x.iter().fold(0_f64, |a, b| a.max(b.abs())) }
pub fn rms(x: &[f64]) -> f64 { (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt() }

/// NumPy rint's round-to-nearest, ties-to-even conversion.
pub fn round_ties_even(value: f64) -> f64 { value.round_ties_even() }

/// The native/wideband adapters use 32767 for encoding and 32768 for decoding.
pub fn pcm16_samples(x: &[f64], gain: f64, clip: f64) -> Result<Vec<f64>> {
    finite(x)?;
    ensure!(gain.is_finite() && gain >= 0. && clip.is_finite() && clip > 0. && clip <= 1., "invalid PCM16 conversion controls");
    Ok(x.iter().map(|v| round_ties_even((v * gain).clamp(-clip, clip) * 32767.) / 32768.).collect())
}

/// One constant gain for the generated increment; the protected base is untouched.
pub fn static_increment_headroom(base: &[f64], increment: &[f64], ceiling: f64) -> Result<(Vec<f64>, Value)> {
    ensure!(!base.is_empty() && base.len() == increment.len(), "nonempty aligned headroom vectors required");
    finite(base)?; finite(increment)?;
    ensure!(ceiling.is_finite() && ceiling > 0. && ceiling < 1., "headroom ceiling must be strictly between zero and one");
    let probe_base = resample_poly(base, 4, 1, 8.6)?;
    let probe_delta = resample_poly(increment, 4, 1, 8.6)?;
    let base_sample_peak = peak(base);
    let base_peak = peak(&probe_base);
    let raw_peak = probe_base.iter().zip(&probe_delta).fold(0_f64, |maximum, (b, d)| maximum.max((b + d).abs()));
    let native_exceeds = base_sample_peak.max(base_peak) > ceiling;
    let mut bound = if native_exceeds { 0. } else { 1. };
    let mut limiting_sample = None;
    let mut limiting_domain = None;
    if !native_exceeds {
        for (b, d, factor) in [(base, increment, 1usize), (probe_base.as_slice(), probe_delta.as_slice(), 4usize)] {
            for sign in [1., -1.] {
                for (sample, (&base_value, &delta_value)) in b.iter().zip(d).enumerate() {
                    if delta_value * sign > 0. {
                        let candidate = (sign * ceiling - base_value) / delta_value;
                        if candidate < bound {
                            bound = candidate;
                            limiting_sample = Some(sample as f64 / factor as f64);
                            limiting_domain = Some(if factor == 1 { "original samples" } else { "4x interpolated samples" });
                        }
                    }
                }
            }
        }
        bound = bound.max(0.);
        if bound < 1. { bound *= 1. - 16. * f64::EPSILON; }
    }
    drop(probe_base); drop(probe_delta);
    let result: Vec<f64> = base.iter().zip(increment).map(|(b, d)| b + bound * d).collect();
    let measured_peak = peak(&resample_poly(&result, 4, 1, 8.6)?);
    let output_peak = peak(&result);
    ensure!(native_exceeds || measured_peak.max(output_peak) <= ceiling + 2e-12, "measured output exceeds computed headroom bound");
    let reason = if native_exceeds { "native input exceeds measured ceiling; generated increment disabled" }
        else if bound < 1. { "bounded generated increment" } else { "headroom already sufficient" };
    Ok((result, json!({
        "policy":"one fixed increment gain over complete input waveform", "increment_gain":bound,
        "ceiling":ceiling,"oversample_factor":4,
        "oversample_window":"Kaiser beta=8.6, scipy.signal.resample_poly default length",
        "native_oversampled_peak":base_peak,"native_sample_peak":base_sample_peak,
        "unguarded_mix_oversampled_peak":raw_peak,"output_oversampled_peak":measured_peak,
        "output_sample_peak":output_peak,"native_exceeds_ceiling":native_exceeds,
        "waveform_scope_samples":base.len(),"limiting_sample_position":limiting_sample,
        "limiting_domain":limiting_domain,"reason":reason,"time_varying_gain":false,
        "native_gain":1.,"certified_true_peak_meter":false
    })))
}

#[cfg(test)] mod tests;
