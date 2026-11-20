//! Phone-local detector filters and reflected lookahead envelopes.
//! Butterworth poles, bilinear gain, nearest SOS pairing, steady-state
//! initial conditions and odd forward/backward extension match SciPy's
//! production third-order `butter(..., output="sos")`/`sosfiltfilt` policy.
use anyhow::{ensure, Result};
use crate::signal::{self, BorderMode, Complex64};

pub(super) fn rms(samples: &[f64], width: usize) -> Result<Vec<f64>> {
    let squared: Vec<f64> = samples.iter().map(|v| v * v).collect();
    let mut result = signal::uniform_filter1d(&squared, width, BorderMode::Reflect)?;
    for value in &mut result { *value = value.max(0.0).sqrt(); }
    ensure!(result.iter().all(|v| v.is_finite()), "output RMS detector produced nonfinite values");
    Ok(result)
}

pub(super) fn percentile(samples: &[f64], percentage: f64) -> f64 {
    let position = (samples.len() - 1) as f64 * (percentage / 100.0);
    let low = position.floor() as usize;
    let high = position.ceil() as usize;
    let mut ordered = samples.to_vec();
    ordered.select_nth_unstable_by(low, f64::total_cmp);
    let a = ordered[low];
    if low == high { return a; }
    let b = ordered[low + 1..].iter().copied().fold(f64::INFINITY, f64::min);
    let fraction = position - low as f64;
    // NumPy's linear quantile interpolator uses its upper-end formula for
    // fractions >= 1/2 to avoid cancellation at the upper endpoint.
    if fraction >= 0.5 { b - (b - a) * (1.0 - fraction) }
    else { a + (b - a) * fraction }
}

fn reflect(index: i64, count: usize) -> usize {
    if count == 1 { return 0; }
    let period = 2 * count as i64;
    let index = index.rem_euclid(period);
    if index < count as i64 { index as usize } else { (period - 1 - index) as usize }
}

pub(super) fn smooth_majorant(values: &[f64], radius: usize) -> Result<Vec<f64>> {
    ensure!(!values.is_empty() && radius > 0 && values.iter().all(|v| v.is_finite()),
        "lookahead requires finite nonempty values and a positive radius");
    let size = 2 * radius + 1;
    let expanded = signal::maximum_filter1d(values, size, BorderMode::Reflect)?;
    let hann = signal::hann(size + 2, false);
    let mut weights = hann[1..hann.len() - 1].to_vec();
    let sum = weights.iter().sum::<f64>();
    for weight in &mut weights { *weight /= sum; }
    let mut result = vec![0.0; values.len()];
    for (index, out) in result.iter_mut().enumerate() {
        // ndimage's symmetric correlation evaluates the center then paired
        // samples from the outside inward. Reflection is half-sample.
        let mut value = expanded[index] * weights[radius];
        for distance in (1..=radius).rev() {
            let left = expanded[reflect(index as i64 - distance as i64, values.len())];
            let right = expanded[reflect(index as i64 + distance as i64, values.len())];
            value += (left + right) * weights[radius - distance];
        }
        *out = values[index].max(value);
    }
    Ok(result)
}

pub(super) struct Butterworth3 { sections: [[f64; 6]; 2], initial: [[f64; 2]; 2] }
impl Butterworth3 {
    pub(super) fn new(cutoff: f64, rate: f64, highpass: bool) -> Result<Self> {
        ensure!(rate.is_finite() && rate > 0.0 && cutoff.is_finite()
            && cutoff > 0.0 && cutoff < rate / 2.0, "invalid output detector cutoff");
        let normalized = 2.0 * cutoff / rate;
        let warped = 4.0 * (std::f64::consts::PI * normalized / 2.0).tan();
        let mut poles = [Complex64::new(0.0, 0.0); 3];
        for (index, m) in [-2.0, 0.0, 2.0].into_iter().enumerate() {
            let theta = std::f64::consts::PI * m / 6.0;
            poles[index] = -Complex64::new(theta.cos(), theta.sin());
        }
        let mut gain = if highpass {
            let denominator = (-poles[0]) * (-poles[1]) * (-poles[2]);
            (Complex64::new(1.0, 0.0) / denominator).re
        } else { warped.powi(3) };
        for pole in &mut poles { *pole = if highpass { warped / *pole } else { *pole * warped }; }
        let mut denominator = Complex64::new(1.0, 0.0);
        for pole in &poles { denominator *= Complex64::new(4.0, 0.0) - *pole; }
        let numerator = if highpass { 4.0_f64.powi(3) } else { 1.0 };
        gain *= (Complex64::new(numerator, 0.0) / denominator).re;
        for pole in &mut poles {
            *pole = (Complex64::new(4.0, 0.0) + *pole) /
                (Complex64::new(4.0, 0.0) - *pole);
        }
        let real_pole = poles[1].re;
        let complex_pole = poles[0];
        let zero = if highpass { 1.0 } else { -1.0 };
        // Nearest pairing adds one pole/zero at the origin. The complex pair
        // is the closest to the unit circle and is placed in the last SOS.
        let origin_in_last = complex_pole.norm() < (complex_pole - zero).norm();
        let pair = [1.0, -2.0 * zero, zero * zero];
        let single = [1.0, -zero, 0.0];
        let (first_b, last_b) = if origin_in_last { (pair, single) } else { (single, pair) };
        let mut sections = [
            [first_b[0] * gain, first_b[1] * gain, first_b[2] * gain, 1.0, -real_pole, 0.0],
            [last_b[0], last_b[1], last_b[2], 1.0, -2.0 * complex_pole.re, complex_pole.norm_sqr()],
        ];
        // Preserve exact zero coefficients for odd-order state admission.
        sections[0][5] = 0.0;
        let mut initial = [[0.0; 2]; 2];
        let mut scale = 1.0;
        for (index, section) in sections.iter().enumerate() {
            let b0 = section[0];
            let b1 = section[1];
            let b2 = section[2];
            let a1 = section[4];
            let a2 = section[5];
            let rhs0 = b1 - a1 * b0;
            let rhs1 = b2 - a2 * b0;
            let zi0 = (rhs0 + rhs1) / (1.0 + a1 + a2);
            let zi1 = rhs1 - a2 * zi0;
            initial[index] = [scale * zi0, scale * zi1];
            scale *= (b0 + b1 + b2) / (1.0 + a1 + a2);
        }
        ensure!(sections.iter().flatten().chain(initial.iter().flatten()).all(|v| v.is_finite()),
            "output detector filter design produced nonfinite coefficients");
        Ok(Self { sections, initial })
    }

    fn forward(&self, samples: &mut [f64]) {
        let first = samples[0];
        let mut state = self.initial;
        for section in &mut state { for value in section { *value *= first; } }
        for sample in samples {
            let mut input = *sample;
            for (section, state) in self.sections.iter().zip(&mut state) {
                let output = section[0] * input + state[0];
                state[0] = section[1] * input - section[4] * output + state[1];
                state[1] = section[2] * input - section[5] * output;
                input = output;
            }
            *sample = input;
        }
    }

    pub(super) fn filtfilt(&self, samples: &[f64]) -> Result<Vec<f64>> {
        ensure!(samples.len() > 1, "forward/backward filter requires at least two samples");
        // The caller's explicit padlen is min(n-1, 3*(2*len(sos)+1)),
        // including very short admitted phone regions.
        let pad = (samples.len() - 1).min(15);
        let mut work = Vec::with_capacity(samples.len() + 2 * pad);
        for index in (1..=pad).rev() { work.push(2.0 * samples[0] - samples[index]); }
        work.extend_from_slice(samples);
        for index in 1..=pad { work.push(2.0 * samples[samples.len() - 1] - samples[samples.len() - 1 - index]); }
        self.forward(&mut work);
        work.reverse();
        self.forward(&mut work);
        work.reverse();
        work.copy_within(pad..pad + samples.len(), 0);
        work.truncate(samples.len());
        ensure!(work.iter().all(|v| v.is_finite()), "output detector produced nonfinite filtered samples");
        Ok(work)
    }
}
