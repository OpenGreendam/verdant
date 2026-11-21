//! Native four-filter, dual-filter and envelope-filter processors.
//!
//! Their small-signal plots evaluate the very same biquads as the audio
//! path. Drive is nonlinear and modulation moves the response over time;
//! the static plot deliberately represents their unmodulated linear filter.

use super::filters::{Biquad, Coefficients, Rbj, db_to_lin, lin_to_db, rbj, time_coefficient};
use super::{Spec, TableEffect};
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

pub const FILTER_SHAPES: [&str; 4] = ["低通", "高通", "带通", "陷波"];
pub const FILTER_ROUTINGS: [&str; 2] = ["串联", "并联"];
pub const FILTER_SLOPES: [&str; 4] = ["12 dB/oct", "24 dB/oct", "36 dB/oct", "48 dB/oct"];
const GLOBALS: usize = 8;
const STRIDE: usize = 6;

macro_rules! filter_specs {
    ($(($n:literal, $frequency:expr, $shape:expr, $enabled:expr)),+) => { &[
        Spec::new("routing", "路由", 0.0, 1.0, 0.0, "").steps(1),
        Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
        Spec::new("output", "输出", -24.0, 24.0, 0.0, "dB"),
        Spec::new("lfo_rate", "LFO 速率", 0.01, 20.0, 0.5, "Hz").log(),
        Spec::new("lfo_depth", "LFO 深度", -48.0, 48.0, 0.0, "st"),
        Spec::new("env_depth", "包络深度", -48.0, 48.0, 0.0, "st"),
        Spec::new("env_attack", "包络启动", 0.1, 1000.0, 10.0, "ms").log(),
        Spec::new("env_release", "包络释放", 1.0, 4000.0, 250.0, "ms").log(),
        $(
            Spec::new(concat!("f", stringify!($n), ".enabled"), concat!("滤波器 ", stringify!($n), " 启用"), 0.0, 1.0, $enabled, "").steps(1),
            Spec::new(concat!("f", stringify!($n), ".freq"), "截止频率", 20.0, 20000.0, $frequency, "Hz").log(),
            Spec::new(concat!("f", stringify!($n), ".resonance"), "共振", 0.3, 12.0, FRAC_1_SQRT_2, "Q").log(),
            Spec::new(concat!("f", stringify!($n), ".shape"), "形状", 0.0, 3.0, $shape, "").steps(3),
            Spec::new(concat!("f", stringify!($n), ".slope"), "斜率", 0.0, 3.0, 0.0, "").steps(3),
            Spec::new(concat!("f", stringify!($n), ".drive"), "驱动", 0.0, 36.0, 0.0, "dB"),
        )+
    ]};
}

pub static FILTER_SPECS: &[Spec] = filter_specs!((1, 5000.0, 0.0, 1.0), (2, 200.0, 1.0, 0.0), (3, 1000.0, 2.0, 0.0), (4, 8000.0, 3.0, 0.0));
pub static SIMPLON_SPECS: &[Spec] = filter_specs!((1, 5000.0, 0.0, 1.0), (2, 100.0, 1.0, 1.0));
pub static MICRO_SPECS: &[Spec] = &[
    Spec::new("freq", "截止频率", 20.0, 20000.0, 3000.0, "Hz").log(),
    Spec::new("resonance", "共振", 0.3, 12.0, FRAC_1_SQRT_2, "Q").log(),
    Spec::new("shape", "形状", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("drive", "驱动", 0.0, 36.0, 0.0, "dB"),
    Spec::new("env_depth", "包络深度", -48.0, 48.0, 0.0, "st"),
    Spec::new("env_attack", "包络启动", 0.1, 1000.0, 10.0, "ms").log(),
    Spec::new("env_release", "包络释放", 1.0, 4000.0, 250.0, "ms").log(),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("output", "输出", -24.0, 24.0, 0.0, "dB"),
];

fn shape(value: f64) -> Rbj {
    match value.round() as u8 { 1 => Rbj::HighPass, 2 => Rbj::BandPass, 3 => Rbj::Notch, _ => Rbj::LowPass }
}

fn complex_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

fn complex_response(c: Coefficients, freq: f64, sample_rate: f64) -> (f64, f64) {
    let w = TAU * freq.clamp(0.0, sample_rate * 0.5) / sample_rate;
    let numerator = (c[0] + c[1] * w.cos() + c[2] * (2.0 * w).cos(), -c[1] * w.sin() - c[2] * (2.0 * w).sin());
    let denominator = (1.0 + c[3] * w.cos() + c[4] * (2.0 * w).cos(), -c[3] * w.sin() - c[4] * (2.0 * w).sin());
    let magnitude = (denominator.0 * denominator.0 + denominator.1 * denominator.1).max(1e-30);
    ((numerator.0 * denominator.0 + numerator.1 * denominator.1) / magnitude,
     (numerator.1 * denominator.0 - numerator.0 * denominator.1) / magnitude)
}

fn filter_coefficients(v: &[f64], modulation: f64, sample_rate: f64) -> [Coefficients; 4] {
    let frequency = (v[1] * 2f64.powf(modulation / 12.0)).clamp(10.0, sample_rate * 0.45);
    std::array::from_fn(|section| rbj(shape(v[3]), frequency, if section == 0 { v[2] } else { FRAC_1_SQRT_2 }, 0.0, sample_rate))
}

/// Response of one bank's real serial/parallel topology, including dry/wet
/// and output gain. `count` is four for Filter and two for Simplon.
pub fn filter_response_db(values: &[f64], count: usize, freq: f64, sample_rate: f64) -> f64 {
    if values.len() < GLOBALS + count * STRIDE { return 0.0; }
    let parallel = values[0] >= 0.5;
    let mut combined = if parallel { (0.0, 0.0) } else { (1.0, 0.0) };
    let mut active = 0usize;
    for index in 0..count {
        let v = &values[GLOBALS + index * STRIDE..][..STRIDE];
        if v[0] < 0.5 { continue; }
        active += 1;
        let sections = v[4].round().clamp(0.0, 3.0) as usize + 1;
        let mut h = (1.0, 0.0);
        for c in filter_coefficients(v, 0.0, sample_rate).into_iter().take(sections) { h = complex_mul(h, complex_response(c, freq, sample_rate)); }
        if parallel { combined.0 += h.0; combined.1 += h.1; }
        else { combined = complex_mul(combined, h); }
    }
    if active == 0 { combined = (1.0, 0.0); }
    else if parallel { combined.0 /= active as f64; combined.1 /= active as f64; }
    let wet = values[1] * 0.01;
    combined = (1.0 + wet * (combined.0 - 1.0), combined.1 * wet);
    lin_to_db(combined.0.hypot(combined.1)) + values[2]
}

fn micro_bank_values(v: &[f64]) -> [f64; GLOBALS + STRIDE] {
    [0.0, v[7], v[8], 0.5, 0.0, v[4], v[5], v[6], 1.0, v[0], v[1], v[2], 0.0, v[3]]
}

pub fn micro_response_db(values: &[f64], freq: f64, sample_rate: f64) -> f64 {
    if values.len() < MICRO_SPECS.len() { return 0.0; }
    filter_response_db(&micro_bank_values(values), 1, freq, sample_rate)
}

struct FilterBank {
    sample_rate: f64,
    filters: [[[Biquad; 4]; 2]; 4],
    phase: f64,
    envelope: f64,
    attack: f64,
    release: f64,
}

impl FilterBank {
    fn new(sample_rate: f64) -> Self {
        Self { sample_rate, filters: [[[Biquad::default(); 4]; 2]; 4], phase: 0.0, envelope: 0.0, attack: 0.0, release: 0.0 }
    }
    fn configure(&mut self, index: usize, values: &[f64], modulation: f64) {
        let coefficients = filter_coefficients(values, modulation, self.sample_rate);
        for channel in &mut self.filters[index] {
            for (filter, coefficient) in channel.iter_mut().zip(coefficients) { filter.set(coefficient); }
        }
    }
    fn refresh(&mut self, values: &[f64], count: usize) {
        self.attack = time_coefficient(values[6], self.sample_rate);
        self.release = time_coefficient(values[7], self.sample_rate);
        for index in 0..count { self.configure(index, &values[GLOBALS + index * STRIDE..][..STRIDE], 0.0); }
    }
    fn process(&mut self, values: &[f64], count: usize, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let parallel = values[0] >= 0.5;
        let wet = values[1] * 0.01;
        let output = db_to_lin(values[2]);
        let phase_increment = values[3] / self.sample_rate;
        let modulated = values[4] != 0.0 || values[5] != 0.0;
        let drive: [f64; 4] = std::array::from_fn(|index| if index < count { db_to_lin(values[GLOBALS + index * STRIDE + 5]) } else { 1.0 });
        let norm = drive.map(f64::tanh);
        for i in 0..in_l.len() {
            let input = [in_l[i] as f64, in_r[i] as f64];
            let level = key.map_or_else(|| input[0].abs().max(input[1].abs()), |(l, r)| (l[i] as f64).abs().max((r[i] as f64).abs()));
            let coefficient = if level > self.envelope { self.attack } else { self.release };
            self.envelope = level + coefficient * (self.envelope - level);
            let modulation = values[4] * (TAU * self.phase).sin() + values[5] * self.envelope.min(1.0);
            self.phase = (self.phase + phase_increment).fract();
            let mut result = if parallel { [0.0; 2] } else { input };
            let mut active = 0usize;
            for index in 0..count {
                let v = &values[GLOBALS + index * STRIDE..][..STRIDE];
                if v[0] < 0.5 { continue; }
                active += 1;
                if modulated { self.configure(index, v, modulation); }
                let sections = v[4].round().clamp(0.0, 3.0) as usize + 1;
                for channel in 0..2 {
                    let mut sample = if parallel { input[channel] } else { result[channel] };
                    if v[5] > 0.0 { sample = (sample * drive[index]).tanh() / norm[index]; }
                    for filter in self.filters[index][channel].iter_mut().take(sections) { sample = filter.run(sample); }
                    if parallel { result[channel] += sample; } else { result[channel] = sample; }
                }
            }
            if active == 0 { result = input; }
            else if parallel { result.iter_mut().for_each(|sample| *sample /= active as f64); }
            out_l[i] = ((input[0] + wet * (result[0] - input[0])) * output) as f32;
            out_r[i] = ((input[1] + wet * (result[1] - input[1])) * output) as f32;
        }
    }
}

macro_rules! processor {
    ($name:ident, $specs:ident, $count:expr) => {
        pub struct $name { pub values: Vec<f64>, bank: FilterBank }
        impl $name {
            pub fn new(sample_rate: f64) -> Self {
                let mut processor = Self { values: super::table_defaults($specs), bank: FilterBank::new(sample_rate) };
                processor.refresh();
                processor
            }
            pub fn response_db(&self, freq: f64) -> f64 { filter_response_db(&self.values, $count, freq, self.bank.sample_rate) }
        }
        impl TableEffect for $name {
            const SPECS: &'static [Spec] = $specs;
            fn values(&self) -> &[f64] { &self.values }
            fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
            fn refresh(&mut self) { self.bank.refresh(&self.values, $count); }
            fn set_sample_rate(&mut self, sample_rate: f64) { self.bank = FilterBank::new(sample_rate); self.refresh(); }
            fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
                self.bank.process(&self.values, $count, in_l, in_r, key, out_l, out_r);
            }
        }
    }
}

processor!(FilterProcessor, FILTER_SPECS, 4);
processor!(SimplonProcessor, SIMPLON_SPECS, 2);

pub struct MicroProcessor { pub values: Vec<f64>, bank: FilterBank }
impl MicroProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut processor = Self { values: super::table_defaults(MICRO_SPECS), bank: FilterBank::new(sample_rate) };
        processor.refresh();
        processor
    }
    pub fn response_db(&self, freq: f64) -> f64 { micro_response_db(&self.values, freq, self.bank.sample_rate) }
}
impl TableEffect for MicroProcessor {
    const SPECS: &'static [Spec] = MICRO_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) { self.bank.refresh(&micro_bank_values(&self.values), 1); }
    fn set_sample_rate(&mut self, sample_rate: f64) { self.bank = FilterBank::new(sample_rate); self.refresh(); }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        self.bank.process(&micro_bank_values(&self.values), 1, in_l, in_r, key, out_l, out_r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn run(p: &mut impl TableEffect, x: &[f32], key: Option<&[f32]>) -> Vec<f32> {
        let mut l = vec![0.0; x.len()];
        let mut r = l.clone();
        p.process(x, x, key.map(|k| (k, k)), &mut l, &mut r);
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        l
    }

    #[test]
    fn plots_match_actual_filter_topology_and_all_shapes_and_slopes() {
        let x = sine(1500.0, 0.1, 24_000);
        for route in 0..=1 {
            for shape in 0..=3 {
                for slope in 0..=3 {
                    let mut p = FilterProcessor::new(FS);
                    for (id, value) in [("routing", route as f64), ("f1.freq", 1000.0), ("f1.shape", shape as f64), ("f1.slope", slope as f64),
                        ("f2.enabled", 1.0), ("f2.freq", 200.0), ("mix", 70.0)] { p.set(id, value).unwrap(); }
                    let expected = p.response_db(1500.0);
                    let y = run(&mut p, &x, None);
                    assert!((rms_db(&y[12_000..]) - rms_db(&x[12_000..]) - expected).abs() < 0.03, "route={route} shape={shape} slope={slope}");
                }
            }
        }
    }

    #[test]
    fn modulation_is_persistent_across_arbitrary_block_sizes() {
        let make = || {
            let mut p = FilterProcessor::new(FS);
            for (id, value) in [("lfo_depth", 18.0), ("env_depth", -12.0), ("f1.drive", 12.0), ("f1.resonance", 2.0)] { p.set(id, value).unwrap(); }
            p
        };
        let x = sine(1000.0, 0.3, 12_000);
        let complete = run(&mut make(), &x, None);
        let mut p = make();
        let mut streaming = Vec::new();
        for block in x.chunks(113) { streaming.extend(run(&mut p, block, None)); }
        assert_eq!(streaming, complete);
    }

    #[test]
    fn envelope_key_changes_cutoff_without_injecting_key_audio() {
        let make = || { let mut p = MicroProcessor::new(FS); p.set("freq", 300.0).unwrap(); p.set("env_depth", 48.0).unwrap(); p };
        let x = sine(2500.0, 0.1, 24_000);
        let key = vec![1.0; x.len()];
        let silence = vec![0.0; x.len()];
        let open = run(&mut make(), &x, Some(&key));
        let closed = run(&mut make(), &x, Some(&silence));
        assert!(rms_db(&open[12_000..]) > rms_db(&closed[12_000..]) + 20.0);
        assert!(run(&mut make(), &silence, Some(&key)).iter().all(|x| *x == 0.0));
    }

    #[test]
    fn each_filter_is_audible_dry_mix_is_exact_and_reset_clears_history() {
        let x = sine(2000.0, 0.3, 8192);
        for index in 1..=4 {
            let mut p = FilterProcessor::new(FS);
            p.set("f1.enabled", 0.0).unwrap();
            p.set(&format!("f{index}.enabled"), 1.0).unwrap();
            p.set(&format!("f{index}.freq"), 400.0).unwrap();
            p.set(&format!("f{index}.shape"), 0.0).unwrap();
            assert!(rms_db(&run(&mut p, &x, None)[4096..]) < rms_db(&x[4096..]) - 20.0);
            p.set("mix", 0.0).unwrap();
            assert_eq!(run(&mut p, &x, None), x);
        }
        let mut p = SimplonProcessor::new(FS);
        run(&mut p, &x, None);
        p.set_sample_rate(96_000.0);
        assert!(run(&mut p, &vec![0.0; 512], None).iter().all(|x| *x == 0.0));
    }

    #[test]
    fn extreme_modulation_stays_finite_at_supported_rates() {
        for sample_rate in [8_000.0, 44_100.0, 96_000.0, 192_000.0] {
            let mut p = FilterProcessor::new(sample_rate);
            for (id, value) in [("f1.resonance", 12.0), ("f1.freq", 20000.0), ("f1.slope", 3.0), ("lfo_depth", 48.0), ("lfo_rate", 20.0), ("env_depth", -48.0)] { p.set(id, value).unwrap(); }
            let y = run(&mut p, &sine(900.0, 0.8, 4096), None);
            assert!(peak(&y) < 1000.0);
        }
    }
}
