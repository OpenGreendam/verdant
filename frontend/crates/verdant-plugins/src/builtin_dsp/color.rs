//! Color: multi-style saturation with a tone tilt and dry/wet mix.

use super::filters::{db_to_lin, rbj, Biquad, Rbj};
use super::{Spec, TableEffect};

pub const SATURATION_STYLES: [&str; 4] = ["磁带", "电子管", "软削波", "硬削波"];

pub static SATURATION_SPECS: &[Spec] = &[
    Spec::new("drive", "驱动", 0.0, 36.0, 6.0, "dB"),
    Spec::new("style", "风格", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("tone", "音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("output", "输出", -24.0, 12.0, 0.0, "dB"),
];

/// The transfer curve of a style for an input sample (already driven).
/// Every curve is odd except the tube's, which adds even harmonics.
pub fn shape(style: usize, x: f64) -> f64 {
    match style {
        0 => x.tanh(),
        1 => (x + 0.25 * x * x.abs()).tanh(),
        2 => 1.5 * if x.abs() < 1.0 { x - x * x * x / 3.0 } else { x.signum() * 2.0 / 3.0 },
        _ => x.clamp(-1.0, 1.0),
    }
}

pub struct SaturationProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    tilt: [[Biquad; 2]; 2],
    /// DC blocker state per channel (the tube curve is asymmetric).
    dc: [(f64, f64); 2],
    drive: f64,
    makeup: f64,
}

impl SaturationProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(SATURATION_SPECS), sample_rate, tilt: Default::default(), dc: [(0.0, 0.0); 2], drive: 1.0, makeup: 1.0 };
        p.refresh();
        p
    }
    /// Driven transfer curve for the editor: output for an input in −1…1.
    pub fn curve(&self, x: f64) -> f64 { shape(self.values[1].round() as usize, x * self.drive) * self.makeup }
}

impl TableEffect for SaturationProcessor {
    const SPECS: &'static [Spec] = SATURATION_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let fs = self.sample_rate;
        let tone = self.values[2];
        let (low, high) = (rbj(Rbj::LowShelf, 800.0, 0.6, -tone / 2.0, fs), rbj(Rbj::HighShelf, 800.0, 0.6, tone / 2.0, fs));
        for channel in &mut self.tilt { channel[0].set(low); channel[1].set(high); }
        self.drive = db_to_lin(self.values[0]);
        // Rough loudness compensation: driving harder should not just be louder.
        self.makeup = db_to_lin(self.values[4]) / self.drive.powf(0.5).max(1.0);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { self.sample_rate = sample_rate; self.tilt = Default::default(); self.refresh(); }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], _key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let style = self.values[1].round() as usize;
        let wet = self.values[3] / 100.0;
        let r_coeff = 1.0 - 2.0 * std::f64::consts::PI * 10.0 / self.sample_rate;
        for i in 0..in_l.len() {
            let x = [in_l[i] as f64, in_r[i] as f64];
            let mut y = [0.0; 2];
            for c in 0..2 {
                let s = shape(style, x[c] * self.drive);
                // DC blocker: y = s − s₁ + R·y₁.
                let (prev_in, prev_out) = self.dc[c];
                let blocked = s - prev_in + r_coeff * prev_out;
                self.dc[c] = (s, blocked);
                let toned = { let t = self.tilt[c][0].run(blocked); self.tilt[c][1].run(t) };
                y[c] = x[c] * (1.0 - wet) + toned * self.makeup * wet;
            }
            out_l[i] = y[0] as f32;
            out_r[i] = y[1] as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    #[test]
    fn every_style_is_bounded_and_monotonic() {
        for style in 0..4 {
            let mut previous = f64::NEG_INFINITY;
            for i in -400..=400 {
                let y = shape(style, i as f64 * 0.01);
                assert!(y.is_finite() && y.abs() <= 1.6, "style {style} at {i}: {y}");
                assert!(y >= previous - 1e-12, "style {style} not monotonic at {i}");
                previous = y;
            }
        }
    }

    #[test]
    fn saturation_adds_harmonics_and_mix_zero_is_dry() {
        let x = sine(1_000.0, 0.8, 9_600);
        let mut s = SaturationProcessor::new(FS);
        s.set("drive", 24.0).unwrap();
        let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        s.process(&x, &x, None, &mut l, &mut r);
        // Third-harmonic energy appears.
        let probe = |y: &[f32], f: f64| -> f64 {
            let (mut re, mut im) = (0.0, 0.0);
            for (n, v) in y[4_800..].iter().enumerate() {
                let phase = std::f64::consts::TAU * f * n as f64 / FS;
                re += *v as f64 * phase.cos();
                im += *v as f64 * phase.sin();
            }
            (re * re + im * im).sqrt()
        };
        assert!(probe(&l, 3_000.0) > probe(&l, 1_000.0) * 0.05);
        assert!(probe(&x, 3_000.0) < probe(&x, 1_000.0) * 1e-3);
        let mut dry = SaturationProcessor::new(FS);
        dry.set("mix", 0.0).unwrap();
        dry.process(&x, &x, None, &mut l, &mut r);
        assert_eq!(l, x);
        assert!(rms_db(&x) > -10.0);
    }
}
