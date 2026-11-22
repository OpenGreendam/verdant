//! Native four-filter, dual-filter and envelope-filter processors.
//!
//! Their small-signal plots evaluate the very same biquads as the audio
//! path. Drive is nonlinear and modulation moves the response over time;
//! the static plot deliberately represents their unmodulated linear filter.

use super::filters::{Biquad, Coefficients, Rbj, db_to_lin, lin_to_db, rbj, time_coefficient};
use super::{Spec, TableEffect};
use super::quality::Oversampler;
use super::filters::DelayLine;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

pub const FILTER_SHAPES: [&str; 8] = ["低通", "高通", "带通", "陷波", "低架", "高架", "峰值", "全通"];
pub const FILTER_CHARACTERS: [&str; 11] = ["干净", "圆润", "粗粝", "硬质", "中空", "柔和", "电子管", "极端", "二极管", "整流", "金属"];
pub const FILTER_ROUTINGS: [&str; 4] = ["串联", "并联", "双组并联", "交错立体声"];
pub const FILTER_SLOPES: [&str; 4] = ["12 dB/oct", "24 dB/oct", "36 dB/oct", "48 dB/oct"];
const GLOBALS: usize = 8;
const STRIDE: usize = 6;

macro_rules! filter_specs {
    ($(($n:literal, $frequency:expr, $shape:expr, $enabled:expr)),+) => { &[
        Spec::new("routing", "路由", 0.0, 3.0, 0.0, "").steps(3),
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
            Spec::new(concat!("f", stringify!($n), ".shape"), "形状", 0.0, 7.0, $shape, "").steps(7),
            Spec::new(concat!("f", stringify!($n), ".slope"), "斜率", 0.0, 3.0, 0.0, "").steps(3),
            Spec::new(concat!("f", stringify!($n), ".drive"), "驱动", 0.0, 36.0, 0.0, "dB"),
        )+
        Spec::new("oversampling", "过采样", 0.0, 5.0, 0.0, "").steps(5).fixed(),
        Spec::new("channel_mode", "处理模式", 0.0, 1.0, 0.0, "").steps(1),
        $(
            Spec::new(concat!("f", stringify!($n), ".character"), concat!("滤波器 ", stringify!($n), " 染色"), 0.0, 10.0, 0.0, "").steps(10),
            Spec::new(concat!("f", stringify!($n), ".feedback"), concat!("滤波器 ", stringify!($n), " 反馈"), -95.0, 95.0, 0.0, "%"),
            Spec::new(concat!("f", stringify!($n), ".pan"), concat!("滤波器 ", stringify!($n), " 声像"), -100.0, 100.0, 0.0, "%"),
            Spec::new(concat!("f", stringify!($n), ".level"), concat!("滤波器 ", stringify!($n), " 电平"), -24.0, 24.0, 0.0, "dB"),
            Spec::new(concat!("f", stringify!($n), ".gain"), concat!("滤波器 ", stringify!($n), " 均衡增益"), -24.0, 24.0, 0.0, "dB"),
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
    Spec::new("character", "染色", 0.0, 10.0, 0.0, "").steps(10),
    Spec::new("oversampling", "过采样", 0.0, 5.0, 0.0, "").steps(5).fixed(),
    Spec::new("feedback", "反馈", -95.0, 95.0, 0.0, "%"),
    Spec::new("gain", "滤波电平", -24.0, 24.0, 0.0, "dB"),
    Spec::new("pan", "声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("channel_mode", "处理模式", 0.0, 1.0, 0.0, "").steps(1),
];

fn shape(value: f64) -> Rbj {
    match value.round() as u8 { 1 => Rbj::HighPass, 2 => Rbj::BandPass, 3 => Rbj::Notch, 4 => Rbj::LowShelf, 5 => Rbj::HighShelf, 6 => Rbj::Bell, _ => Rbj::LowPass }
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

fn filter_coefficients(v: &[f64], modulation: f64, sample_rate: f64, gain: f64) -> [Coefficients; 4] {
    let frequency = (v[1] * 2f64.powf(modulation / 12.0)).clamp(10.0, sample_rate * 0.45);
    std::array::from_fn(|section| {
        let c = rbj(shape(v[3]), frequency, if section == 0 { v[2] } else { FRAC_1_SQRT_2 }, gain, sample_rate);
        if v[3].round() as usize == 7 { [c[4], c[3], 1.0, c[3], c[4]] } else { c }
    })
}

/// Response of one bank's real serial/parallel topology, including dry/wet
/// and output gain. `count` is four for Filter and two for Simplon.
pub fn filter_response_db(values: &[f64], count: usize, freq: f64, sample_rate: f64) -> f64 {
    if values.len() < GLOBALS + count * STRIDE { return 0.0; }
    let extra = GLOBALS + count * STRIDE;
    let setting = |offset: usize| values.get(extra + offset).copied().unwrap_or(0.0);
    let quality = setting(0).round().clamp(0.0, 5.0) as u32;
    let rate = sample_rate * (1u32 << quality) as f64;
    let unit = |index: usize, channel: usize| -> (f64, f64) {
        let v = &values[GLOBALS + index * STRIDE..][..STRIDE];
        if v[0] < 0.5 { return (1.0, 0.0); }
        let p = 2 + index * 5; let kind = setting(p).round() as usize; let drive = db_to_lin(v[5]);
        let derivative = (character_sample(kind, 1e-7, drive) - character_sample(kind, -1e-7, drive)) / 2e-7;
        let mut h = (derivative, 0.0);
        for c in filter_coefficients(v, 0.0, rate, setting(p + 4)).into_iter().take(v[4].round().clamp(0.0, 3.0) as usize + 1) {
            h = complex_mul(h, complex_response(c, freq, rate));
        }
        let phase = TAU * freq / rate;
        let feedback = complex_mul(h, (phase.cos() * setting(p + 1) * 0.01, -phase.sin() * setting(p + 1) * 0.01));
        let denominator = (1.0 - feedback.0, -feedback.1);
        let norm = denominator.0.powi(2) + denominator.1.powi(2);
        h = ((h.0 * denominator.0 + h.1 * denominator.1) / norm.max(1e-20), (h.1 * denominator.0 - h.0 * denominator.1) / norm.max(1e-20));
        let pan = setting(p + 2) * 0.01;
        let gain = db_to_lin(setting(p + 3)) * if channel == 0 { 1.0 - pan.max(0.0) } else { 1.0 + pan.min(0.0) };
        (h.0 * gain, h.1 * gain)
    };
    let route = values[0].round() as usize;
    let mut power = 0.0;
    for channel in 0..2 {
        let mut h = (1.0, 0.0);
        match route {
            1 => {
                let mut sum = (0.0, 0.0); let mut active = 0;
                for index in 0..count { if values[GLOBALS + index * STRIDE] >= 0.5 {
                    let band = unit(index, channel); sum.0 += band.0; sum.1 += band.1; active += 1;
                } }
                if active > 0 { h = (sum.0 / active as f64, sum.1 / active as f64); }
            }
            2 => {
                let mut sum = (0.0, 0.0); let mut groups = 0;
                for start in (0..count).step_by(2) {
                    let mut band = (1.0, 0.0); let mut active = false;
                    for index in start..(start + 2).min(count) { active |= values[GLOBALS + index * STRIDE] >= 0.5; band = complex_mul(band, unit(index, channel)); }
                    if active { sum.0 += band.0; sum.1 += band.1; groups += 1; }
                }
                if groups > 0 { h = (sum.0 / groups as f64, sum.1 / groups as f64); }
            }
            _ => { for index in 0..count { if route != 3 || index % 2 == channel { h = complex_mul(h, unit(index, channel)); } } }
        }
        let wet = values[1] * 0.01;
        h = (1.0 + wet * (h.0 - 1.0), h.1 * wet);
        power += (h.0.powi(2) + h.1.powi(2)) * 0.5;
    }
    lin_to_db(power.sqrt()) + values[2]
}

fn micro_bank_values(v: &[f64]) -> [f64; GLOBALS + STRIDE + 7] {
    [0.0, v[7], v[8], 0.5, 0.0, v[4], v[5], v[6], 1.0, v[0], v[1], v[2], 0.0, v[3], v[10], v[14], v[9], v[11], v[13], v[12], 0.0]
}

pub fn micro_response_db(values: &[f64], freq: f64, sample_rate: f64) -> f64 {
    if values.len() < MICRO_SPECS.len() { return 0.0; }
    filter_response_db(&micro_bank_values(values), 1, freq, sample_rate)
}

/// Nonlinear coloration is applied inside each unit's feedback path. The
/// default characteristic keeps the previous RBJ response and saved sound.
fn character_sample(kind: usize, x: f64, drive: f64) -> f64 {
    let z = x * drive;
    match kind {
        0 => if drive > 1.0 { z.tanh() / drive.tanh() } else { x },
        1 => (z * 0.7).tanh() / (drive * 0.7).tanh(),
        2 => (z + 0.15 * (z * 2.0).sin()).tanh() / drive.tanh(),
        3 => z.clamp(-0.85, 0.85) / drive.min(0.85),
        4 => (z - 0.2 * (z * 3.0).sin()).tanh() / (drive * 0.8).tanh(),
        5 => z.atan() / drive.atan(),
        6 => ((z + 0.2).tanh() - 0.2f64.tanh()) / drive.tanh(),
        7 => (z * 1.5).sin().tanh() / 1.5f64.sin().tanh(),
        8 => z / (1.0 + z.abs()) * (1.0 + drive) / drive,
        9 => (z.abs() - 0.25 * z).tanh() / drive.tanh(),
        10 => (z + 0.35 * (z * 5.0).sin()).tanh() / drive.tanh(),
        _ => x,
    }
}
struct FilterBank {
    sample_rate: f64, filters: [[[Biquad; 4]; 2]; 4], phase: f64, envelope: f64, attack: f64, release: f64,
    quality: [Oversampler; 2], dry: [DelayLine; 2], feedback: [[f64; 2]; 4],
}
impl FilterBank {
    fn new(sample_rate: f64) -> Self {
        Self { sample_rate, filters: [[[Biquad::default(); 4]; 2]; 4], phase: 0.0, envelope: 0.0, attack: 0.0, release: 0.0,
            quality: [Oversampler::new(), Oversampler::new()], dry: [DelayLine::new(34), DelayLine::new(34)], feedback: [[0.0; 2]; 4] }
    }
    fn configure(&mut self, index: usize, values: &[f64], modulation: f64, gain: f64) {
        let coefficients = filter_coefficients(values, modulation, self.sample_rate * self.quality[0].factor() as f64, gain);
        for channel in &mut self.filters[index] { for (filter, coefficient) in channel.iter_mut().zip(coefficients) { filter.set(coefficient); } }
    }
    fn refresh(&mut self, values: &[f64], count: usize) {
        self.attack = time_coefficient(values[6], self.sample_rate); self.release = time_coefficient(values[7], self.sample_rate);
        let extra = GLOBALS + count * STRIDE;
        self.quality.iter_mut().for_each(|q| q.set_quality(values[extra].round() as usize));
        for index in 0..count { self.configure(index, &values[GLOBALS + index * STRIDE..][..STRIDE], 0.0, values[extra + 2 + index * 5 + 4]); }
    }
    fn unit(&mut self, values: &[f64], count: usize, index: usize, channel: usize, input: f64, drive: f64) -> f64 {
        let v = &values[GLOBALS + index * STRIDE..][..STRIDE];
        if v[0] < 0.5 { return input; }
        let extra = GLOBALS + count * STRIDE + 2 + index * 5;
        let feedback = self.feedback[index][channel] * values[extra + 1] * 0.01;
        let mut sample = character_sample(values[extra].round() as usize, input + feedback, drive);
        let sections = v[4].round().clamp(0.0, 3.0) as usize + 1;
        for filter in self.filters[index][channel].iter_mut().take(sections) { sample = filter.run(sample); }
        self.feedback[index][channel] = sample.clamp(-4.0, 4.0);
        let pan = values[extra + 2] * 0.01;
        sample * db_to_lin(values[extra + 3]) * if channel == 0 { 1.0 - pan.max(0.0) } else { 1.0 + pan.min(0.0) }
    }
    fn process(&mut self, values: &[f64], count: usize, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let routing = values[0].round() as usize; let wet = values[1] * 0.01; let output = db_to_lin(values[2]);
        let extra = GLOBALS + count * STRIDE; let ms = values[extra + 1] >= 0.5;
        let modulated = values[4] != 0.0 || values[5] != 0.0;
        let drive: [f64; 4] = std::array::from_fn(|index| if index < count { db_to_lin(values[GLOBALS + index * STRIDE + 5]) } else { 1.0 });
        for i in 0..in_l.len() {
            let original = [in_l[i] as f64, in_r[i] as f64];
            let input = if ms { [(original[0] + original[1]) * 0.5, (original[0] - original[1]) * 0.5] } else { original };
            let level = key.map_or_else(|| input[0].abs().max(input[1].abs()), |(l, r)| (l[i] as f64).abs().max((r[i] as f64).abs()));
            let coefficient = if level > self.envelope { self.attack } else { self.release };
            self.envelope = level + coefficient * (self.envelope - level);
            let modulation = values[4] * (TAU * self.phase).sin() + values[5] * self.envelope.min(1.0);
            self.phase = (self.phase + values[3] / self.sample_rate).fract();
            if modulated { for index in 0..count { self.configure(index, &values[GLOBALS + index * STRIDE..][..STRIDE], modulation, values[extra + 2 + index * 5 + 4]); } }
            let mut high = [[0.0; 32]; 2]; let factor = self.quality[0].factor();
            for channel in 0..2 { self.quality[channel].upsample(input[channel], &mut high[channel]); }
            for tick in 0..factor { for channel in 0..2 {
                let input = high[channel][tick]; let mut result = input;
                match routing {
                    1 => {
                        let mut sum = 0.0; let mut active = 0;
                        for (index, gain) in drive.iter().enumerate().take(count) { if values[GLOBALS + index * STRIDE] >= 0.5 {
                            sum += self.unit(values, count, index, channel, input, *gain); active += 1;
                        } }
                        if active > 0 { result = sum / active as f64; }
                    }
                    2 => {
                        let mut sum = 0.0; let mut groups = 0;
                        for start in (0..count).step_by(2) {
                            let mut group = input; let mut active = false;
                            for (index, gain) in drive.iter().enumerate().take((start + 2).min(count)).skip(start) {
                                active |= values[GLOBALS + index * STRIDE] >= 0.5; group = self.unit(values, count, index, channel, group, *gain);
                            }
                            if active { sum += group; groups += 1; }
                        }
                        if groups > 0 { result = sum / groups as f64; }
                    }
                    _ => { for (index, gain) in drive.iter().enumerate().take(count) {
                        if routing != 3 || index % 2 == channel { result = self.unit(values, count, index, channel, result, *gain); }
                    } }
                }
                high[channel][tick] = result;
            } }
            let mut result = [0.0; 2];
            for channel in 0..2 {
                let processed = self.quality[channel].downsample(&high[channel]); self.dry[channel].push(input[channel]);
                let delay = self.quality[channel].latency(); let dry = if delay > 0 { self.dry[channel].tap(delay) } else { input[channel] };
                result[channel] = (dry + wet * (processed - dry)) * output;
            }
            let result = if ms { [result[0] + result[1], result[0] - result[1]] } else { result };
            out_l[i] = result[0] as f32; out_r[i] = result[1] as f32;
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
            fn latency(&self) -> u32 { self.bank.quality[0].latency() as u32 }
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
    fn latency(&self) -> u32 { self.bank.quality[0].latency() as u32 }
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
    #[test]
    fn new_routings_shapes_and_characters_change_the_processed_signal() {
        let x = sine(1200.0, 0.00001, 12000);
        for route in 0..4 {
            for shape in 4..8 {
                let mut p = FilterProcessor::new(FS);
                for (id, value) in [("routing", route as f64), ("f1.shape", shape as f64), ("f1.freq", 1000.0), ("f1.gain", 9.0), ("f2.enabled", 1.0), ("f2.freq", 4000.0)] { p.set(id, value).unwrap(); }
                let mut left = vec![0.0; x.len()]; let mut right = left.clone();
                p.process(&x, &x, None, &mut left, &mut right);
                let measured = 10.0 * ((10f64.powf(rms_db(&left[6000..]) / 10.0) + 10f64.powf(rms_db(&right[6000..]) / 10.0)) * 0.5).log10() - rms_db(&x[6000..]);
                assert!((measured - p.response_db(1200.0)).abs() < 0.05, "route {route}, shape {shape}, measured {measured}, plot {}", p.response_db(1200.0));
            }
        }
        let x = sine(1200.0, 0.6, 6000); let mut fingerprints = Vec::new();
        for character in 0..FILTER_CHARACTERS.len() {
            let mut p = FilterProcessor::new(FS); p.set("f1.character", character as f64).unwrap(); p.set("f1.drive", 12.0).unwrap();
            fingerprints.push(run(&mut p, &x, None));
        }
        for pair in fingerprints.windows(2) { assert!(pair[0].iter().zip(&pair[1]).any(|(a,b)| (a-b).abs() > 0.001)); }
        let mut p = MicroProcessor::new(FS); p.set("oversampling", 3.0).unwrap(); p.set("mix", 0.0).unwrap();
        let y = run(&mut p, &x, None); assert_eq!(p.latency(), 32);
        assert_eq!(&y[32..], &x[..x.len()-32]);
    }

}
