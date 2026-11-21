//! Space: an eight-line feedback-delay-network reverb and a stereo delay.

use super::filters::{db_to_lin, one_pole_coefficient, rbj, Biquad, DelayLine, Rbj, IDENTITY};
use super::{Spec, TableEffect};
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

// ---------------------------------------------------------------------------
// Reverb
// ---------------------------------------------------------------------------

pub static REVERB_SPECS: &[Spec] = &[
    Spec::new("predelay", "预延迟", 0.0, 250.0, 10.0, "ms"),
    Spec::new("decay", "衰减时间", 0.2, 20.0, 2.0, "s").log(),
    Spec::new("size", "空间大小", 10.0, 100.0, 60.0, "%"),
    Spec::new("damping", "高频阻尼", 1000.0, 20000.0, 8000.0, "Hz").log(),
    Spec::new("diffusion", "扩散", 0.0, 100.0, 75.0, "%"),
    Spec::new("width", "宽度", 0.0, 150.0, 100.0, "%"),
    Spec::new("low_cut", "低切", 20.0, 1000.0, 120.0, "Hz").log(),
    Spec::new("high_cut", "高切", 1000.0, 20000.0, 12000.0, "Hz").log(),
    Spec::new("mix", "干湿比", 0.0, 100.0, 25.0, "%"),
    Spec::new("modulation", "调制", 0.0, 100.0, 30.0, "%"),
];

const LINES: usize = 8;
/// Base line lengths in milliseconds at 100 % size (mutually prime-ish).
const LINE_MS: [f64; LINES] = [29.7, 37.1, 41.1, 43.7, 47.9, 53.3, 59.9, 67.1];
const DIFFUSER_MS: [f64; 4] = [4.77, 3.59, 12.73, 9.30];
const MOD_DEPTH_MS: f64 = 0.6;

/// Feedback gain giving a 60 dB decay over `rt60` seconds for a line of
/// `delay` samples.
pub fn decay_gain(delay: f64, rt60: f64, sample_rate: f64) -> f64 { 10f64.powf(-3.0 * delay / (rt60.max(0.05) * sample_rate)) }

/// Level of the reverb tail `t` seconds after an impulse, in dB, as drawn by
/// the editor (exponential decay reaching −60 dB at the decay time).
pub fn reverb_envelope_db(decay: f64, predelay_ms: f64, t: f64) -> Option<f64> {
    let t = t - predelay_ms * 0.001;
    (t >= 0.0).then(|| -60.0 * t / decay.max(0.05))
}

pub struct ReverbProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    predelay: [DelayLine; 2],
    diffusers: [[DelayLine; 4]; 2],
    lines: [DelayLine; LINES],
    damp: [f64; LINES],
    damp_coeff: f64,
    lengths: [f64; LINES],
    gains: [f64; LINES],
    phase: f64,
    post: [[Biquad; 2]; 2],
}

impl ReverbProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let fs = sample_rate.max(8_000.0);
        let samples = |ms: f64| (ms * 0.001 * fs).ceil() as usize + 4;
        let mut p = Self {
            values: super::table_defaults(REVERB_SPECS), sample_rate,
            predelay: [DelayLine::new(samples(250.0)), DelayLine::new(samples(250.0))],
            diffusers: std::array::from_fn(|_| DIFFUSER_MS.map(|ms| DelayLine::new(samples(ms)))),
            lines: LINE_MS.map(|ms| DelayLine::new(samples(ms + MOD_DEPTH_MS * 2.0))),
            damp: [0.0; LINES], damp_coeff: 0.0, lengths: [1.0; LINES], gains: [0.0; LINES], phase: 0.0,
            post: Default::default(),
        };
        p.refresh();
        p
    }
}

impl TableEffect for ReverbProcessor {
    const SPECS: &'static [Spec] = REVERB_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let fs = self.sample_rate;
        let size = self.values[2] / 100.0;
        for i in 0..LINES {
            // Integer lengths: interpolating a static tap would low-pass the loop.
            self.lengths[i] = (LINE_MS[i] * size * 0.001 * fs).round().max(8.0);
            self.gains[i] = decay_gain(self.lengths[i], self.values[1], fs);
        }
        // At the top of the range damping is off rather than a faint low-pass.
        self.damp_coeff = if self.values[3] < 19_999.0 { one_pole_coefficient(self.values[3], fs) } else { 0.0 };
        let low = if self.values[6] > 20.5 { rbj(Rbj::HighPass, self.values[6], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY };
        let high = if self.values[7] < 19_999.0 { rbj(Rbj::LowPass, self.values[7], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY };
        for channel in &mut self.post { channel[0].set(low); channel[1].set(high); }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], _key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let fs = self.sample_rate;
        let predelay = (self.values[0] * 0.001 * fs) as usize;
        let diffusion = self.values[4] / 100.0 * 0.7;
        let width = self.values[5] / 100.0;
        let wet = self.values[8] / 100.0;
        let depth = self.values[9] / 100.0 * MOD_DEPTH_MS * 0.001 * fs;
        let step = TAU * 0.37 / fs;
        let diffuser_len: [usize; 4] = DIFFUSER_MS.map(|ms| ((ms * 0.001 * fs) as usize).max(1));
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            self.predelay[0].push(l);
            self.predelay[1].push(r);
            let mut x = [self.predelay[0].tap(predelay), self.predelay[1].tap(predelay)];
            // Input diffusion: four Schroeder allpasses per channel.
            for c in 0..2 {
                for (k, line) in self.diffusers[c].iter_mut().enumerate() {
                    let delayed = line.tap(diffuser_len[k] - 1);
                    let v = x[c] + diffusion * delayed;
                    line.push(v);
                    x[c] = delayed - diffusion * v;
                }
            }
            // Read the lines (two of them slowly modulated).
            self.phase += step;
            if self.phase > TAU { self.phase -= TAU; }
            let mut y = [0.0; LINES];
            for k in 0..LINES {
                let wobble = match k { 1 => self.phase.sin(), 6 => (self.phase * 1.37).cos(), _ => 0.0 } * depth;
                let tap = self.lines[k].tap_frac(self.lengths[k] - 1.0 + wobble);
                // One-pole damping inside the loop shortens the highs' decay.
                self.damp[k] = tap + self.damp_coeff * (self.damp[k] - tap);
                y[k] = self.damp[k] * self.gains[k];
            }
            let (wl, wr) = (y[0] - y[2] + y[4] - y[6], y[1] - y[3] + y[5] - y[7]);
            // Householder feedback: lossless, maximally mixing.
            let sum: f64 = y.iter().sum::<f64>() * (2.0 / LINES as f64);
            for k in 0..LINES {
                let input = if k % 2 == 0 { x[0] } else { x[1] };
                self.lines[k].push(y[k] - sum + input * 0.35);
            }
            let (mid, side) = ((wl + wr) * 0.5, (wl - wr) * 0.5 * width);
            let (mut el, mut er) = (mid + side, mid - side);
            el = { let t = self.post[0][0].run(el); self.post[0][1].run(t) };
            er = { let t = self.post[1][0].run(er); self.post[1][1].run(t) };
            out_l[i] = (l * (1.0 - wet) + el * wet * 0.5) as f32;
            out_r[i] = (r * (1.0 - wet) + er * wet * 0.5) as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// Delay
// ---------------------------------------------------------------------------

pub static DELAY_SPECS: &[Spec] = &[
    Spec::new("time", "时间", 1.0, 2000.0, 375.0, "ms").log(),
    Spec::new("offset", "左右偏移", -50.0, 50.0, 0.0, "%"),
    Spec::new("feedback", "反馈", 0.0, 95.0, 35.0, "%"),
    Spec::new("pingpong", "乒乓", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("low_cut", "低切", 20.0, 2000.0, 150.0, "Hz").log(),
    Spec::new("high_cut", "高切", 1000.0, 20000.0, 8000.0, "Hz").log(),
    Spec::new("drive", "驱动", 0.0, 24.0, 0.0, "dB"),
    Spec::new("width", "宽度", 0.0, 100.0, 100.0, "%"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 25.0, "%"),
];

pub struct DelayProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    lines: [DelayLine; 2],
    filters: [[Biquad; 2]; 2],
    /// Smoothed delay times (samples) so time changes glide instead of click.
    times: [f64; 2],
    drive: f64,
    /// Before the first block, time changes jump instead of gliding.
    started: bool,
}

impl DelayProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let capacity = (2.0 * 1.5 * sample_rate.max(8_000.0)) as usize + 8;
        let mut p = Self { values: super::table_defaults(DELAY_SPECS), sample_rate, lines: [DelayLine::new(capacity), DelayLine::new(capacity)],
            filters: Default::default(), times: [0.0; 2], drive: 1.0, started: false };
        p.refresh();
        p.times = p.targets();
        p
    }
    fn targets(&self) -> [f64; 2] {
        let base = self.values[0] * 0.001 * self.sample_rate;
        let offset = self.values[1] / 100.0;
        [base * (1.0 - offset.min(0.0).abs()) * (1.0 + offset.max(0.0)), base * (1.0 + offset.min(0.0).abs()) * (1.0 - offset.max(0.0))]
            .map(|t| t.clamp(1.0, (self.lines[0].capacity() - 4) as f64))
    }
    /// Echo `n` (1-based) level in dB for the editor's echo display.
    pub fn echo_db(&self, n: u32) -> f64 { 20.0 * (self.values[2] / 100.0).max(1e-6).log10() * (n.saturating_sub(1)) as f64 }
}

impl TableEffect for DelayProcessor {
    const SPECS: &'static [Spec] = DELAY_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let fs = self.sample_rate;
        let low = if self.values[4] > 20.5 { rbj(Rbj::HighPass, self.values[4], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY };
        let high = if self.values[5] < 19_999.0 { rbj(Rbj::LowPass, self.values[5], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY };
        for channel in &mut self.filters { channel[0].set(low); channel[1].set(high); }
        self.drive = db_to_lin(self.values[6]);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); self.times = self.targets(); }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], _key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let targets = self.targets();
        if !self.started { self.times = targets; self.started = true; }
        let feedback = self.values[2] / 100.0;
        let pingpong = self.values[3] >= 0.5;
        let width = self.values[7] / 100.0;
        let wet = self.values[8] / 100.0;
        let glide = (-1.0 / (0.05 * self.sample_rate)).exp();
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            for c in 0..2 { self.times[c] = targets[c] + glide * (self.times[c] - targets[c]); }
            let echo = [self.lines[0].tap_frac(self.times[0] - 1.0), self.lines[1].tap_frac(self.times[1] - 1.0)];
            // Filter and saturate what feeds back, so repeats darken and warm.
            let mut fed = [0.0; 2];
            for c in 0..2 {
                let f = { let t = self.filters[c][0].run(echo[c]); self.filters[c][1].run(t) };
                fed[c] = if self.drive > 1.0 { (f * self.drive).tanh() / self.drive } else { f };
            }
            if pingpong {
                // Mono input enters the left line; repeats alternate sides.
                self.lines[0].push((l + r) * 0.5 + fed[1] * feedback);
                self.lines[1].push(fed[0] * feedback);
            } else {
                self.lines[0].push(l + fed[0] * feedback);
                self.lines[1].push(r + fed[1] * feedback);
            }
            let (mid, side) = ((echo[0] + echo[1]) * 0.5, (echo[0] - echo[1]) * 0.5 * width);
            out_l[i] = (l * (1.0 - wet) + (mid + side) * wet) as f32;
            out_r[i] = (r * (1.0 - wet) + (mid - side) * wet) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn impulse_response(fx: &mut impl TableEffect, n: usize) -> (Vec<f32>, Vec<f32>) {
        let mut x = vec![0.0; n];
        x[0] = 1.0;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        fx.process(&x, &x, None, &mut l, &mut r);
        (l, r)
    }

    #[test]
    fn reverb_tail_decays_at_the_set_time_and_is_stereo() {
        for decay in [0.8, 3.0] {
            let mut rv = ReverbProcessor::new(FS);
            for (id, v) in [("decay", decay), ("mix", 100.0), ("predelay", 0.0), ("damping", 20_000.0), ("low_cut", 20.0), ("high_cut", 20_000.0), ("modulation", 0.0)] { rv.set(id, v).unwrap(); }
            let n = (FS * decay * 1.2) as usize;
            let (l, r) = impulse_response(&mut rv, n);
            let window = 2_400;
            let early = rms_db(&l[4_800..4_800 + window]);
            let at = (FS * decay * 0.5) as usize;
            let later = rms_db(&l[at..at + window]);
            // Halfway through the decay time the tail has fallen ~30 dB.
            let fall = early - later;
            let expected = 60.0 * (at - 4_800) as f64 / (FS * decay);
            assert!((fall - expected).abs() < 6.0, "decay {decay}: fell {fall}, expected {expected}");
            assert!(l.iter().zip(&r).any(|(a, b)| (a - b).abs() > 1e-4), "decorrelated channels");
            assert!(l.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn reverb_predelay_holds_the_tail_back_and_dry_passes() {
        let mut rv = ReverbProcessor::new(FS);
        rv.set("predelay", 50.0).unwrap();
        rv.set("mix", 100.0).unwrap();
        let (l, _) = impulse_response(&mut rv, 6_000);
        assert!(l[..2_400].iter().all(|v| v.abs() < 1e-9));
        assert!(peak(&l[2_400..]) > 1e-3);
        let mut dry = ReverbProcessor::new(FS);
        dry.set("mix", 0.0).unwrap();
        let x = sine(440.0, 0.3, 2_000);
        let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        dry.process(&x, &x, None, &mut l, &mut r);
        assert_eq!(l, x);
    }

    #[test]
    fn delay_repeats_at_the_set_time_with_feedback() {
        let mut d = DelayProcessor::new(FS);
        for (id, v) in [("time", 100.0), ("feedback", 50.0), ("mix", 100.0), ("low_cut", 20.0), ("high_cut", 20_000.0)] { d.set(id, v).unwrap(); }
        let (l, _) = impulse_response(&mut d, 20_000);
        let first = l.iter().position(|v| v.abs() > 0.1).unwrap();
        assert!((first as i64 - 4_800).abs() <= 2, "first echo at {first}");
        let second = peak(&l[9_000..10_000]);
        assert!((second as f64 - 0.5).abs() < 0.05, "second echo {second}");
    }

    #[test]
    fn pingpong_alternates_sides() {
        let mut d = DelayProcessor::new(FS);
        for (id, v) in [("time", 50.0), ("feedback", 80.0), ("mix", 100.0), ("pingpong", 1.0), ("low_cut", 20.0), ("high_cut", 20_000.0)] { d.set(id, v).unwrap(); }
        let (l, r) = impulse_response(&mut d, 8_000);
        assert!(peak(&l[2_300..2_500]) > 0.3 && peak(&r[2_300..2_500]) < 0.05);
        assert!(peak(&r[4_700..4_900]) > 0.2 && peak(&l[4_700..4_900]) < 0.05);
    }
}
