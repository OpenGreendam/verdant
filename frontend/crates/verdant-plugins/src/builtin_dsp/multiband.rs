//! Six-band dynamics with complementary, zero-latency crossovers.
//!
//! Every split stores its low band and passes `input - low` to the next
//! split. With zero range and gain the bands sum to the original sample,
//! including transients. Detector filters have separate state, so an
//! external key never enters the audible signal. This is an independent
//! implementation, not a model of a proprietary multiband processor.

use super::filters::{db_to_lin, lin_to_db, one_pole_coefficient, time_coefficient};
use super::{Spec, TableEffect};

pub const MULTIBAND_BANDS: usize = 6;
pub const MULTIBAND_MODES: [&str; 2] = ["压缩", "扩展"];
const GLOBALS: usize = 9;
const STRIDE: usize = 10;

macro_rules! band_specs {
    ($($n:literal),+) => { &[
        Spec::new("band_count", "频段数", 1.0, 6.0, 6.0, "").steps(5),
        Spec::new("xover1", "分频 1", 20.0, 200.0, 120.0, "Hz").log(),
        Spec::new("xover2", "分频 2", 201.0, 800.0, 500.0, "Hz").log(),
        Spec::new("xover3", "分频 3", 801.0, 3000.0, 2000.0, "Hz").log(),
        Spec::new("xover4", "分频 4", 3001.0, 8000.0, 6000.0, "Hz").log(),
        Spec::new("xover5", "分频 5", 8001.0, 20000.0, 12000.0, "Hz").log(),
        Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
        Spec::new("output", "输出", -24.0, 24.0, 0.0, "dB"),
        Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
        $(
            Spec::new(concat!("b", stringify!($n), ".enabled"), concat!("频段 ", stringify!($n), " 启用"), 0.0, 1.0, 1.0, "").steps(1),
            Spec::new(concat!("b", stringify!($n), ".threshold"), "阈值", -72.0, 0.0, -24.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".range"), "动态范围", -48.0, 24.0, 0.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".ratio"), "比率", 1.0, 20.0, 3.0, ":1").log(),
            Spec::new(concat!("b", stringify!($n), ".attack"), "启动", 0.1, 200.0, 10.0, "ms").log(),
            Spec::new(concat!("b", stringify!($n), ".release"), "释放", 5.0, 2000.0, 120.0, "ms").log(),
            Spec::new(concat!("b", stringify!($n), ".knee"), "拐点", 0.0, 24.0, 6.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".gain"), "频段增益", -24.0, 24.0, 0.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".solo"), "独奏", 0.0, 1.0, 0.0, "").steps(1),
            Spec::new(concat!("b", stringify!($n), ".mode"), "模式", 0.0, 1.0, 0.0, "").steps(1),
        )+
    ]};
}

pub static MULTIBAND_SPECS: &[Spec] = band_specs!(1, 2, 3, 4, 5, 6);

/// Effective crossover frequencies; used by both DSP and the native graph.
/// Their non-overlapping parameter ranges preserve their displayed order.
pub fn multiband_crossovers(values: &[f64], sample_rate: f64) -> [f64; 5] {
    std::array::from_fn(|i| values.get(i + 1).copied().unwrap_or(MULTIBAND_SPECS[i + 1].default).min(sample_rate * 0.45))
}

#[derive(Clone, Copy, Default)]
struct Splitter { memory: [[f64; 2]; 5] }

impl Splitter {
    #[inline]
    fn run(&mut self, input: [f64; 2], coefficients: &[f64; 5], count: usize) -> [[f64; 2]; 6] {
        let mut bands = [[0.0; 2]; 6];
        let mut high = input;
        for band in 0..count - 1 {
            let a = coefficients[band];
            for channel in 0..2 {
                let low = high[channel] + a * (self.memory[band][channel] - high[channel]);
                self.memory[band][channel] = low;
                bands[band][channel] = low;
                high[channel] -= low;
            }
        }
        bands[count - 1] = high;
        bands
    }
}

fn complex_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Exact small-signal magnitude of the same complementary splitter used by
/// the processor. This is a band response, not a fabricated live analyzer.
pub fn multiband_band_response_db(values: &[f64], band: usize, freq: f64, sample_rate: f64) -> f64 {
    let count = values.first().copied().unwrap_or(6.0).round().clamp(1.0, 6.0) as usize;
    if band >= count { return -180.0; }
    let w = std::f64::consts::TAU * freq.clamp(0.0, sample_rate * 0.5) / sample_rate;
    let mut h = (1.0, 0.0);
    for (index, crossover) in multiband_crossovers(values, sample_rate).into_iter().enumerate().take(count - 1) {
        let a = one_pole_coefficient(crossover, sample_rate);
        let (re, im) = (1.0 - a * w.cos(), a * w.sin());
        let denominator = re * re + im * im;
        let low = ((1.0 - a) * re / denominator, -(1.0 - a) * im / denominator);
        if index == band {
            h = complex_mul(h, low);
            break;
        }
        h = complex_mul(h, (1.0 - low.0, -low.1));
    }
    lin_to_db(h.0.hypot(h.1))
}

fn soft_positive(x: f64, knee: f64) -> f64 {
    if x <= -knee * 0.5 { 0.0 }
    else if knee > 0.0 && x < knee * 0.5 { (x + knee * 0.5).powi(2) / (2.0 * knee) }
    else { x }
}

/// Static signed gain change. Negative range selects downward action and
/// positive range selects upward action; expansion reverses the trigger side.
pub fn multiband_gain_db(level: f64, threshold: f64, range: f64, ratio: f64, knee: f64, expand: bool) -> f64 {
    let amount = if expand { ratio.max(1.0) - 1.0 } else { 1.0 - 1.0 / ratio.max(1.0) };
    let distance = if (range < 0.0) != expand { level - threshold } else { threshold - level };
    let gain = soft_positive(distance, knee.max(0.0)) * amount;
    if range < 0.0 { -gain.min(-range) } else { gain.min(range) }
}

pub struct MultibandProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    split: Splitter,
    detector_split: Splitter,
    coefficients: [f64; 5],
    attack: [f64; 6],
    release: [f64; 6],
    detector: [f64; 6],
    gain_db: [f64; 6],
    detector_coefficient: f64,
}

impl MultibandProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut processor = Self { values: super::table_defaults(MULTIBAND_SPECS), sample_rate,
            split: Splitter::default(), detector_split: Splitter::default(), coefficients: [0.0; 5],
            attack: [0.0; 6], release: [0.0; 6], detector: [0.0; 6], gain_db: [0.0; 6], detector_coefficient: 0.0 };
        processor.refresh();
        processor
    }
    pub fn band_gain_db(&self) -> [f64; 6] { self.gain_db }
}

impl TableEffect for MultibandProcessor {
    const SPECS: &'static [Spec] = MULTIBAND_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        self.coefficients = multiband_crossovers(&self.values, self.sample_rate).map(|freq| one_pole_coefficient(freq, self.sample_rate));
        self.detector_coefficient = time_coefficient(2.0, self.sample_rate);
        for band in 0..6 {
            let offset = GLOBALS + band * STRIDE;
            self.attack[band] = time_coefficient(self.values[offset + 4], self.sample_rate);
            self.release[band] = time_coefficient(self.values[offset + 5], self.sample_rate);
        }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.split = Splitter::default();
        self.detector_split = Splitter::default();
        self.detector = [0.0; 6];
        self.gain_db = [0.0; 6];
        self.refresh();
    }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let count = self.values[0].round().clamp(1.0, 6.0) as usize;
        let wet = self.values[6] * 0.01;
        let output = db_to_lin(self.values[7]);
        let listen = self.values[8] >= 0.5;
        let any_solo = (0..count).any(|band| self.values[GLOBALS + band * STRIDE + 8] >= 0.5);
        for i in 0..in_l.len() {
            let input = [in_l[i] as f64, in_r[i] as f64];
            let sidechain = key.map_or(input, |(l, r)| [l[i] as f64, r[i] as f64]);
            let bands = self.split.run(input, &self.coefficients, count);
            let keys = self.detector_split.run(sidechain, &self.coefficients, count);
            // Apply only each band's change to the original signal. Apart
            // from guaranteeing exact unity at neutral settings, this avoids
            // tiny cancellation residuals on silent input after a transient.
            let mut result = if listen || any_solo { [0.0; 2] } else { input };
            for band in 0..count {
                let v = &self.values[GLOBALS + band * STRIDE..][..STRIDE];
                let level_squared = keys[band][0].powi(2).max(keys[band][1].powi(2));
                self.detector[band] = level_squared + self.detector_coefficient * (self.detector[band] - level_squared);
                let target = if v[0] >= 0.5 {
                    multiband_gain_db(lin_to_db(self.detector[band].sqrt()), v[1], v[2], v[3], v[6], v[9] >= 0.5)
                } else { 0.0 };
                let coefficient = if target.abs() > self.gain_db[band].abs() { self.attack[band] } else { self.release[band] };
                self.gain_db[band] = target + coefficient * (self.gain_db[band] - target);
                let gain = if v[0] >= 0.5 { db_to_lin(self.gain_db[band] + v[7]) } else { 1.0 };
                if !any_solo || v[8] >= 0.5 {
                    for channel in 0..2 {
                        result[channel] += if listen { keys[band][channel] }
                            else { bands[band][channel] * if any_solo { gain } else { gain - 1.0 } };
                    }
                }
            }
            out_l[i] = ((if listen { result[0] } else { input[0] + wet * (result[0] - input[0]) }) * output) as f32;
            out_r[i] = ((if listen { result[1] } else { input[1] + wet * (result[1] - input[1]) }) * output) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn run(p: &mut MultibandProcessor, x: &[f32], key: Option<&[f32]>) -> Vec<f32> {
        let mut l = vec![0.0; x.len()];
        let mut r = l.clone();
        p.process(x, x, key.map(|k| (k, k)), &mut l, &mut r);
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        l
    }

    #[test]
    fn neutral_splits_reconstruct_transients_at_each_sample_rate_and_band_count() {
        let x: Vec<f32> = (0..4096).map(|i| ((i * 1973 % 997) as f32 / 498.0) - 1.0).collect();
        for sr in [8_000.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            for count in 1..=6 {
                let mut p = MultibandProcessor::new(sr);
                p.set("band_count", count as f64).unwrap();
                assert_eq!(run(&mut p, &x, None), x, "{sr} Hz / {count} bands");
            }
        }
    }

    #[test]
    fn sidechain_changes_dynamics_but_does_not_leak_into_output() {
        let x = sine(440.0, 0.1, 48_000);
        let loud_key = sine(2000.0, 0.9, x.len());
        let quiet_key = vec![0.0; x.len()];
        let configure = || {
            let mut p = MultibandProcessor::new(FS);
            for (id, value) in [("band_count", 1.0), ("b1.threshold", -30.0), ("b1.range", -36.0), ("b1.ratio", 10.0)] { p.set(id, value).unwrap(); }
            p
        };
        let a = run(&mut configure(), &x, Some(&loud_key));
        let b = run(&mut configure(), &x, Some(&quiet_key));
        assert!(rms_db(&a[24_000..]) < rms_db(&b[24_000..]) - 15.0);
        assert!(run(&mut configure(), &quiet_key, Some(&loud_key)).iter().all(|x| *x == 0.0));
    }

    #[test]
    fn audition_magnitude_agrees_with_display_and_streaming_is_partition_invariant() {
        let mut p = MultibandProcessor::new(FS);
        p.set("b2.solo", 1.0).unwrap();
        let x = sine(400.0, 0.2, 48_000);
        let expected = multiband_band_response_db(&p.values, 1, 400.0, FS);
        let actual = run(&mut p, &x, None);
        assert!((rms_db(&actual[24_000..]) - rms_db(&x[24_000..]) - expected).abs() < 0.02);
        let mut streaming = MultibandProcessor::new(FS);
        streaming.set("b2.solo", 1.0).unwrap();
        let mut assembled = Vec::new();
        for block in x.chunks(137) { assembled.extend(run(&mut streaming, block, None)); }
        assert_eq!(actual, assembled);
    }

    #[test]
    fn signed_ranges_implement_four_dynamics_quadrants_and_reset_clears_history() {
        assert!(multiband_gain_db(-10.0, -20.0, -24.0, 4.0, 0.0, false) < 0.0);
        assert!(multiband_gain_db(-30.0, -20.0, 12.0, 4.0, 0.0, false) > 0.0);
        assert!(multiband_gain_db(-30.0, -20.0, -24.0, 4.0, 0.0, true) < 0.0);
        assert!(multiband_gain_db(-10.0, -20.0, 12.0, 4.0, 0.0, true) > 0.0);
        let mut p = MultibandProcessor::new(FS);
        p.set("b1.gain", -12.0).unwrap();
        run(&mut p, &sine(80.0, 0.5, 4096), None);
        p.set_sample_rate(96_000.0);
        assert!(run(&mut p, &vec![0.0; 1024], None).iter().all(|x| *x == 0.0));
        assert!(p.set("b1.range", f64::NAN).is_err());
    }
}
