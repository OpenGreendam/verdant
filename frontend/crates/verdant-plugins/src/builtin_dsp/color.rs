//! Six-band saturation with complementary crossovers, per-band coloration,
//! dynamics, feedback and drive modulation. The first five automation IDs
//! retain their original order; the single-band default keeps old projects.

use super::filters::{db_to_lin, lin_to_db, rbj, time_coefficient, Biquad, Rbj};
use super::{Spec, TableEffect};
use super::quality::{Oversampler, LinearSplit};
use super::filters::DelayLine;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

pub const SATURATION_STYLES: [&str; 28] = ["磁带", "电子管", "软削波", "硬削波", "折返", "整流", "变压器", "位深压缩", "细腻电子管", "干净电子管", "温暖电子管", "破裂电子管", "细腻磁带", "干净磁带", "温暖磁带", "老式磁带", "细腻变压器", "温暖变压器", "彩色变压器", "美式放大器", "英式放大器", "柔和放大器", "粗糙放大器", "尖叫放大器", "功率放大器", "涂抹", "破碎降调", "摧毁"];
pub const SATURATION_BAND_STYLES: [&str; 29] = ["跟随全局", "磁带", "电子管", "软削波", "硬削波", "折返", "整流", "变压器", "位深压缩", "细腻电子管", "干净电子管", "温暖电子管", "破裂电子管", "细腻磁带", "干净磁带", "温暖磁带", "老式磁带", "细腻变压器", "温暖变压器", "彩色变压器", "美式放大器", "英式放大器", "柔和放大器", "粗糙放大器", "尖叫放大器", "功率放大器", "涂抹", "破碎降调", "摧毁"];
pub const SATURATION_BANDS: usize = 6;
pub const SATURATION_BAND_START: usize = 18;
pub const SATURATION_BAND_STRIDE: usize = 10;
pub static SATURATION_SPECS: &[Spec] = &[
    Spec::new("drive", "驱动", 0.0, 36.0, 6.0, "dB"),
    Spec::new("style", "风格", 0.0, 27.0, 0.0, "").steps(27),
    Spec::new("tone", "音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("output", "输出", -24.0, 12.0, 0.0, "dB"),
    Spec::new("band_count", "频段数", 1.0, 6.0, 1.0, "").steps(5),
    Spec::new("split1", "分频 1", 30.0, 18000.0, 150.0, "Hz").log(),
    Spec::new("split2", "分频 2", 30.0, 18000.0, 500.0, "Hz").log(),
    Spec::new("split3", "分频 3", 30.0, 18000.0, 2000.0, "Hz").log(),
    Spec::new("split4", "分频 4", 30.0, 18000.0, 6000.0, "Hz").log(),
    Spec::new("split5", "分频 5", 30.0, 18000.0, 12000.0, "Hz").log(),
    Spec::new("mod_rate", "驱动调制速率", 0.01, 20.0, 0.5, "Hz").log(),
    Spec::new("mod_depth", "驱动调制深度", 0.0, 24.0, 0.0, "dB"),
    Spec::new("feedback", "反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("feedback_freq", "反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("dynamics", "动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("dc_filter", "去直流", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("channel_mode", "处理模式", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b1_on", "频段 1 开关", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("b1_drive", "频段 1 驱动", -24.0, 36.0, 0.0, "dB"),
    Spec::new("b1_style", "频段 1 风格", 0.0, 28.0, 0.0, "").steps(28),
    Spec::new("b1_tone", "频段 1 音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("b1_mix", "频段 1 干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("b1_level", "频段 1 电平", -60.0, 24.0, 0.0, "dB"),
    Spec::new("b1_pan", "频段 1 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("b1_dynamics", "频段 1 动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("b1_solo", "频段 1 独听", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b1_mute", "频段 1 静音", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b2_on", "频段 2 开关", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("b2_drive", "频段 2 驱动", -24.0, 36.0, 0.0, "dB"),
    Spec::new("b2_style", "频段 2 风格", 0.0, 28.0, 0.0, "").steps(28),
    Spec::new("b2_tone", "频段 2 音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("b2_mix", "频段 2 干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("b2_level", "频段 2 电平", -60.0, 24.0, 0.0, "dB"),
    Spec::new("b2_pan", "频段 2 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("b2_dynamics", "频段 2 动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("b2_solo", "频段 2 独听", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b2_mute", "频段 2 静音", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b3_on", "频段 3 开关", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("b3_drive", "频段 3 驱动", -24.0, 36.0, 0.0, "dB"),
    Spec::new("b3_style", "频段 3 风格", 0.0, 28.0, 0.0, "").steps(28),
    Spec::new("b3_tone", "频段 3 音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("b3_mix", "频段 3 干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("b3_level", "频段 3 电平", -60.0, 24.0, 0.0, "dB"),
    Spec::new("b3_pan", "频段 3 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("b3_dynamics", "频段 3 动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("b3_solo", "频段 3 独听", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b3_mute", "频段 3 静音", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b4_on", "频段 4 开关", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("b4_drive", "频段 4 驱动", -24.0, 36.0, 0.0, "dB"),
    Spec::new("b4_style", "频段 4 风格", 0.0, 28.0, 0.0, "").steps(28),
    Spec::new("b4_tone", "频段 4 音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("b4_mix", "频段 4 干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("b4_level", "频段 4 电平", -60.0, 24.0, 0.0, "dB"),
    Spec::new("b4_pan", "频段 4 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("b4_dynamics", "频段 4 动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("b4_solo", "频段 4 独听", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b4_mute", "频段 4 静音", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b5_on", "频段 5 开关", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("b5_drive", "频段 5 驱动", -24.0, 36.0, 0.0, "dB"),
    Spec::new("b5_style", "频段 5 风格", 0.0, 28.0, 0.0, "").steps(28),
    Spec::new("b5_tone", "频段 5 音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("b5_mix", "频段 5 干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("b5_level", "频段 5 电平", -60.0, 24.0, 0.0, "dB"),
    Spec::new("b5_pan", "频段 5 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("b5_dynamics", "频段 5 动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("b5_solo", "频段 5 独听", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b5_mute", "频段 5 静音", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b6_on", "频段 6 开关", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("b6_drive", "频段 6 驱动", -24.0, 36.0, 0.0, "dB"),
    Spec::new("b6_style", "频段 6 风格", 0.0, 28.0, 0.0, "").steps(28),
    Spec::new("b6_tone", "频段 6 音色", -12.0, 12.0, 0.0, "dB"),
    Spec::new("b6_mix", "频段 6 干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("b6_level", "频段 6 电平", -60.0, 24.0, 0.0, "dB"),
    Spec::new("b6_pan", "频段 6 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("b6_dynamics", "频段 6 动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("b6_solo", "频段 6 独听", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("b6_mute", "频段 6 静音", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("oversampling", "过采样", 0.0, 5.0, 0.0, "").steps(5).fixed(),
    Spec::new("phase", "分频相位", 0.0, 1.0, 0.0, "").steps(1).fixed(),
    Spec::new("cabinet", "箱体", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("cabinet_mix", "箱体混合", 0.0, 100.0, 100.0, "%"),
    Spec::new("b1_feedback", "频段 1 反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("b1_feedback_freq", "频段 1 反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("b1_drive_pan", "频段 1 驱动平衡", -100.0, 100.0, 0.0, "%"),
    Spec::new("b1_bass", "频段 1 低频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b1_mid", "频段 1 中频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b1_treble", "频段 1 高频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b1_presence", "频段 1 临场", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b1_channel", "频段 1 声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("b2_feedback", "频段 2 反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("b2_feedback_freq", "频段 2 反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("b2_drive_pan", "频段 2 驱动平衡", -100.0, 100.0, 0.0, "%"),
    Spec::new("b2_bass", "频段 2 低频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b2_mid", "频段 2 中频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b2_treble", "频段 2 高频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b2_presence", "频段 2 临场", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b2_channel", "频段 2 声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("b3_feedback", "频段 3 反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("b3_feedback_freq", "频段 3 反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("b3_drive_pan", "频段 3 驱动平衡", -100.0, 100.0, 0.0, "%"),
    Spec::new("b3_bass", "频段 3 低频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b3_mid", "频段 3 中频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b3_treble", "频段 3 高频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b3_presence", "频段 3 临场", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b3_channel", "频段 3 声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("b4_feedback", "频段 4 反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("b4_feedback_freq", "频段 4 反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("b4_drive_pan", "频段 4 驱动平衡", -100.0, 100.0, 0.0, "%"),
    Spec::new("b4_bass", "频段 4 低频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b4_mid", "频段 4 中频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b4_treble", "频段 4 高频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b4_presence", "频段 4 临场", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b4_channel", "频段 4 声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("b5_feedback", "频段 5 反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("b5_feedback_freq", "频段 5 反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("b5_drive_pan", "频段 5 驱动平衡", -100.0, 100.0, 0.0, "%"),
    Spec::new("b5_bass", "频段 5 低频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b5_mid", "频段 5 中频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b5_treble", "频段 5 高频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b5_presence", "频段 5 临场", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b5_channel", "频段 5 声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("b6_feedback", "频段 6 反馈", 0.0, 90.0, 0.0, "%"),
    Spec::new("b6_feedback_freq", "频段 6 反馈频率", 20.0, 16000.0, 1000.0, "Hz").log(),
    Spec::new("b6_drive_pan", "频段 6 驱动平衡", -100.0, 100.0, 0.0, "%"),
    Spec::new("b6_bass", "频段 6 低频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b6_mid", "频段 6 中频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b6_treble", "频段 6 高频", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b6_presence", "频段 6 临场", -18.0, 18.0, 0.0, "dB"),
    Spec::new("b6_channel", "频段 6 声道", 0.0, 2.0, 0.0, "").steps(2),
];

/// Stateless waveshaper used by audio and the transfer-curve editor. The
/// tube and rectifier are asymmetric; DC removal follows in the processor.
pub fn shape(style: usize, x: f64) -> f64 {
    match style {
        0 => x.tanh(),
        1 => ((x + 0.15).tanh() - 0.15f64.tanh()) / (1.0 - 0.15f64.tanh()),
        2 => 1.5 * if x.abs() < 1.0 { x - x * x * x / 3.0 } else { x.signum() * 2.0 / 3.0 },
        3 => x.clamp(-1.0, 1.0),
        4 => { let f = (x + 1.0).rem_euclid(4.0); if f < 2.0 { f - 1.0 } else { 3.0 - f } },
        5 => x.abs().tanh(),
        6 => (x + 0.12 * (x * 2.0).sin()).tanh(),
        7 => (x.clamp(-1.0, 1.0) * 31.0).round() / 31.0,
        8 => (x * 0.7).tanh() / 0.7,
        9 => x / (1.0 + x.powi(4)).powf(0.25),
        10 => ((x + 0.3).tanh() - 0.3f64.tanh()) * 1.1,
        11 => ((x + 0.5).tanh() - 0.5f64.tanh()) * 0.9,
        12 => (x * 0.8).atan() * 0.95,
        13 => x / (1.0 + x.abs().powi(3)).cbrt(),
        14 => (x + 0.08 * x.sin()).tanh(),
        15 => (x * 1.2).tanh() * 0.85 + 0.1 * (x * 0.4).sin(),
        16 => (x + 0.02 * (x * 3.0).sin()).tanh(),
        17 => (x + 0.2 * x.sin()).tanh(),
        18 => ((x * 1.4).atan() * 0.7 + 0.08 * (x * 5.0).sin()).tanh(),
        19 => ((x + 0.08).tanh() - 0.08f64.tanh()).tanh() * 1.25,
        20 => ((x * 1.6).tanh() - 0.08 * (x * 0.5).tanh()).tanh() * 1.3,
        21 => (x.atan() * 0.9).tanh() * 1.2,
        22 => (x.clamp(-0.7, 0.9) * 1.6).tanh(),
        23 => (x * 2.2 + 0.15 * (x * 7.0).sin()).tanh(),
        24 => (x / (1.0 + 0.2 * x.abs())).tanh(),
        25 => (x * 0.9).tanh(),
        26 => (x * 1.4).tanh(),
        27 => (x.clamp(-1.0, 1.0) * 15.0).round() / 15.0,
        _ => x.tanh(),
    }
}

/// Memoryless band transfer used by the UI without constructing DSP history.
pub fn saturation_band_curve(values: &[f64], band: usize, x: f64) -> f64 {
    let b = band.min(SATURATION_BANDS - 1); let p = SATURATION_BAND_START + b * SATURATION_BAND_STRIDE;
    if values.len() <= p + 9 { return x; }
    if values[p + 9] >= 0.5 { return 0.0; }
    let output = db_to_lin(values[4]); if values[p] < 0.5 { return x * output; }
    let style = if values[p + 2] < 0.5 { values[1] } else { values[p + 2] - 1.0 };
    let mix = values[p + 4] / 100.0; let drive = db_to_lin(values[0] + values[p + 1]);
    let wet = shape(style.round() as usize, x * drive) / drive.sqrt().max(1.0);
    (x * (1.0 - mix) + wet * mix) * db_to_lin(values[p + 5]) * output
}

pub struct SaturationProcessor {
    pub values: Vec<f64>, sample_rate: f64,
    split: [[Biquad; 2]; 5],
    tilt: [[[Biquad; 2]; 2]; SATURATION_BANDS],
    dc: [[(f64, f64); 2]; SATURATION_BANDS],
    feedback_filter: [[Biquad; 2]; SATURATION_BANDS],
    feedback: [[f64; 2]; SATURATION_BANDS],
    envelope: [[f64; 2]; SATURATION_BANDS],
    env_coeff: f64, phase: f64,
    drive: [f64; SATURATION_BANDS], makeup: [f64; SATURATION_BANDS],
    quality: [[Oversampler; 2]; SATURATION_BANDS], linear: [[LinearSplit; 2]; 5],
    dry: [DelayLine; 2], band_dry: [[DelayLine; 2]; SATURATION_BANDS],
    extra_tone: [[[Biquad; 4]; 2]; SATURATION_BANDS], cabinet: [[Biquad; 3]; 2],
    creative: [[[f64; 4]; 2]; SATURATION_BANDS],
}
impl SaturationProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(SATURATION_SPECS), sample_rate,
            split: Default::default(), tilt: Default::default(), dc: [[(0.0, 0.0); 2]; SATURATION_BANDS],
            feedback_filter: Default::default(), feedback: [[0.0; 2]; SATURATION_BANDS],
            envelope: [[0.0; 2]; SATURATION_BANDS], env_coeff: 0.0, phase: 0.0,
            drive: [1.0; SATURATION_BANDS], makeup: [1.0; SATURATION_BANDS],
            quality: std::array::from_fn(|_| [Oversampler::new(), Oversampler::new()]),
            linear: std::array::from_fn(|_| [LinearSplit::default(), LinearSplit::default()]),
            dry: [DelayLine::new(100), DelayLine::new(100)],
            band_dry: std::array::from_fn(|_| [DelayLine::new(34), DelayLine::new(34)]),
            extra_tone: Default::default(), cabinet: Default::default(), creative: [[[0.0; 4]; 2]; SATURATION_BANDS] };
        p.refresh(); p
    }
    pub fn crossover_frequencies(&self) -> [f64; 5] {
        let mut frequencies = std::array::from_fn(|i| self.values[6 + i]);
        frequencies.sort_by(f64::total_cmp);
        frequencies
    }
    /// One band's memoryless driven characteristic; filter and envelope
    /// history are intentionally not misrepresented as a static curve.
    pub fn band_curve(&self, band: usize, x: f64) -> f64 { saturation_band_curve(&self.values, band, x) }
    pub fn curve(&self, x: f64) -> f64 {
        let wet = self.values[3] / 100.0;
        x * (1.0 - wet) + self.band_curve(0, x) * wet
    }
}
impl TableEffect for SaturationProcessor {
    const SPECS: &'static [Spec] = SATURATION_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let fs = self.sample_rate;
        for (i, freq) in self.crossover_frequencies().iter().enumerate() {
            for filter in &mut self.split[i] { filter.set(rbj(Rbj::LowPass, *freq, FRAC_1_SQRT_2, 0.0, fs)); }
            self.linear[i].iter_mut().for_each(|f| f.configure(*freq, fs));
        }
        for b in 0..SATURATION_BANDS {
            let p = SATURATION_BAND_START + b * SATURATION_BAND_STRIDE;
            let tone = self.values[2] + self.values[p + 3];
            let (low, high) = (rbj(Rbj::LowShelf, 800.0, 0.6, -tone / 2.0, fs), rbj(Rbj::HighShelf, 800.0, 0.6, tone / 2.0, fs));
            for channel in &mut self.tilt[b] { channel[0].set(low); channel[1].set(high); }
            let extra = 82 + b * 8;
            let frequency = if self.values[extra] > 0.0 { self.values[extra + 1] } else { self.values[14] };
            for filter in &mut self.feedback_filter[b] { filter.set(rbj(Rbj::BandPass, frequency, 0.707, 0.0, fs)); }
            self.quality[b].iter_mut().for_each(|q| q.set_quality(self.values[78].round() as usize));
            for channel in &mut self.extra_tone[b] {
                for (index, filter) in channel.iter_mut().enumerate() {
                    let shape = [Rbj::LowShelf, Rbj::Bell, Rbj::HighShelf, Rbj::Bell][index];
                    filter.set(rbj(shape, [180.0, 900.0, 4500.0, 2500.0][index], 0.707, self.values[extra + 3 + index], fs));
                }
            }
            self.drive[b] = db_to_lin(self.values[0] + self.values[p + 1]);
            self.makeup[b] = 1.0 / self.drive[b].powf(0.5).max(1.0);
        }
        self.env_coeff = time_coefficient(10.0, fs);
        let cabinet = self.values[80].round() as usize;
        let (low, high, body) = match cabinet { 1 => (90.0, 5500.0, 3.0), 2 => (70.0, 4200.0, 5.0), 3 => (45.0, 6500.0, 2.0), _ => (120.0, 3500.0, 6.0) };
        for channel in &mut self.cabinet {
            channel[0].set(rbj(Rbj::HighPass, low, 0.707, 0.0, fs));
            channel[1].set(rbj(Rbj::LowPass, high, 0.707, 0.0, fs));
            channel[2].set(rbj(Rbj::Bell, 220.0, 1.1, body, fs));
        }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn latency(&self) -> u32 { (self.quality[0][0].latency() + if self.values[79] >= 0.5 { LinearSplit::LATENCY } else { 0 }) as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let count = (self.values[5].round() as usize).clamp(1, SATURATION_BANDS);
        let wet = self.values[3] / 100.0; let feedback = self.values[13] / 100.0;
        let output = db_to_lin(self.values[4]);
        let band_gain: [f64; SATURATION_BANDS] = std::array::from_fn(|b| db_to_lin(self.values[SATURATION_BAND_START + b * SATURATION_BAND_STRIDE + 5]));
        let dc_on = self.values[16] >= 0.5; let ms = self.values[17] >= 0.5;
        let dc_coeff = (-TAU * 10.0 / self.sample_rate).exp();
        let any_solo = (0..count).any(|b| self.values[SATURATION_BAND_START + b * SATURATION_BAND_STRIDE + 8] >= 0.5);
        for i in 0..in_l.len() {
            let original = [in_l[i] as f64, in_r[i] as f64];
            let x = if ms { [(original[0] + original[1]) * 0.5, (original[0] - original[1]) * 0.5] } else { original };
            let mut bands = [[0.0; 2]; SATURATION_BANDS]; let mut remainder = x;
            self.dry[0].push(original[0]); self.dry[1].push(original[1]);
            if self.values[79] >= 0.5 {
                let mut previous = [0.0; 2]; let mut delayed = [0.0; 2];
                for b in 0..count.max(2) - 1 { for c in 0..2 {
                    let (low, high) = self.linear[b][c].run(x[c]); delayed[c] = low + high;
                    if b < count - 1 { bands[b][c] = low - previous[c]; previous[c] = low; }
                } }
                bands[count - 1] = [delayed[0] - previous[0], delayed[1] - previous[1]];
            } else {
                for b in 0..count - 1 { for c in 0..2 { let low = self.split[b][c].run(remainder[c]); bands[b][c] = low; remainder[c] -= low; } }
                bands[count - 1] = remainder;
            }
            self.phase = (self.phase + TAU * self.values[11] / self.sample_rate).rem_euclid(TAU);
            let modulation = if self.values[12] == 0.0 { 1.0 } else { db_to_lin(self.values[12] * self.phase.sin()) };
            let mut y = [0.0; 2];
            for b in 0..count {
                let p = SATURATION_BAND_START + b * SATURATION_BAND_STRIDE;
                let audible = self.values[p + 9] < 0.5 && (!any_solo || self.values[p + 8] >= 0.5);
                let enabled = self.values[p] >= 0.5;
                let mix = self.values[p + 4] / 100.0;
                let style = if self.values[p + 2] < 0.5 { self.values[1] } else { self.values[p + 2] - 1.0 };
                let dynamics = ((self.values[15] + self.values[p + 7]) / 100.0).clamp(-1.0, 1.0);
                let pan = self.values[p + 6] / 100.0; let extra = 82 + b * 8;
                let local_feedback = (feedback + self.values[extra] / 100.0).min(0.95);
                for c in 0..2 {
                    let signal = bands[b][c];
                    self.band_dry[b][c].push(signal);
                    let delayed_signal = if self.quality[b][c].latency() > 0 { self.band_dry[b][c].tap(self.quality[b][c].latency()) } else { signal };
                    let selected = self.values[extra + 7] < 0.5 || self.values[extra + 7].round() as usize == c + 1;
                    if !enabled || !selected {
                        self.quality[b][c].run(signal, |x| x);
                        if audible { y[c] += delayed_signal; }
                        self.feedback[b][c] = 0.0;
                        self.feedback_filter[b][c].reset();
                        self.dc[b][c] = (0.0, 0.0);
                        self.envelope[b][c] = 0.0;
                        self.tilt[b][c].iter_mut().for_each(Biquad::reset);
                        continue;
                    }
                    let detector = key.map_or(signal.abs(), |(l, r)| if c == 0 { l[i].abs() as f64 } else { r[i].abs() as f64 });
                    self.envelope[b][c] = detector + self.env_coeff * (self.envelope[b][c] - detector);
                    let dynamic_gain = if dynamics == 0.0 { 1.0 } else {
                        let db = lin_to_db(self.envelope[b][c]);
                        let dynamic_db = if dynamics > 0.0 { ((-18.0 - db) * dynamics * 0.5).clamp(-18.0, 12.0) }
                            else { ((db + 30.0).min(0.0) * -dynamics * 2.0).max(-60.0) };
                        db_to_lin(dynamic_db)
                    };
                    let drive_pan = self.values[extra + 2] * 0.12 * if c == 0 { -1.0 } else { 1.0 };
                    let drive = dynamic_gain * self.drive[b] * modulation * db_to_lin(drive_pan);
                    let style = style.round() as usize;
                    let shaped = self.quality[b][c].run(signal + self.feedback[b][c] * local_feedback, |x| shape(style, x * drive));
                    let memory = &mut self.creative[b][c];
                    let shaped = match style {
                        25 => { let y = -0.72 * shaped + memory[0]; memory[0] = shaped + 0.72 * y; y }
                        26 => { if shaped >= 0.0 && memory[0] < 0.0 { memory[1] = if memory[1] > 0.0 { -1.0 } else { 1.0 }; } memory[0] = shaped; memory[1] * shaped.abs() }
                        27 => { memory[0] += 1.0; if memory[0] >= (2.0 + self.values[0] / 6.0) { memory[0] = 0.0; memory[1] = shaped; } memory[1] }
                        _ => shaped,
                    };
                    self.feedback[b][c] = self.feedback_filter[b][c].run(shaped).clamp(-2.0, 2.0);
                    let (prev_in, prev_out) = self.dc[b][c];
                    let blocked = shaped - prev_in + dc_coeff * prev_out; self.dc[b][c] = (shaped, blocked);
                    let colored = if dc_on { blocked } else { shaped };
                    let mut toned = { let low = self.tilt[b][c][0].run(colored); self.tilt[b][c][1].run(low) };
                    for filter in &mut self.extra_tone[b][c] { toned = filter.run(toned); }
                    let balance = if c == 0 { 1.0 - pan.max(0.0) } else { 1.0 + pan.min(0.0) };
                    // Level and pan trim the complete band, including its
                    // dry portion. Muted/unsoloed bands keep processing their
                    // history so unmuting cannot revive a paused old tail.
                    if audible { y[c] += (delayed_signal * (1.0 - mix) + toned * self.makeup[b] * mix) * band_gain[b] * balance; }
                }
            }
            let mut y = if ms { [y[0] + y[1], y[0] - y[1]] } else { y };
            if self.values[80] >= 0.5 { for c in 0..2 {
                let mut cabinet = y[c]; for filter in &mut self.cabinet[c] { cabinet = filter.run(cabinet); }
                y[c] += (cabinet - y[c]) * self.values[81] * 0.01;
            } }
            let delay = self.latency() as usize;
            let dry = if delay == 0 { original } else { [self.dry[0].tap(delay), self.dry[1].tap(delay)] };
            out_l[i] = (dry[0] * (1.0 - wet) + y[0] * output * wet) as f32;
            out_r[i] = (dry[1] * (1.0 - wet) + y[1] * output * wet) as f32;
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

    #[test]
    fn six_complementary_bands_recombine_and_solo_mute_are_audible() {
        let x = sine(1000.0, 0.2, 12000);
        let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        let mut fx = SaturationProcessor::new(FS); fx.set("band_count", 6.0).unwrap();
        for band in 1..=6 { fx.set(&format!("b{band}_mix"), 0.0).unwrap(); }
        fx.process(&x, &x, None, &mut l, &mut r);
        assert!(l.iter().zip(&x).all(|(a, b)| (a - b).abs() < 1e-6), "neutral complementary bands must sum to dry");
        fx.set("b1_solo", 1.0).unwrap(); fx.process(&x, &x, None, &mut l, &mut r);
        assert!(rms_db(&x[6000..]) - rms_db(&l[6000..]) > 25.0, "low band solo must reject a 1 kHz signal");
        fx.set("b1_mute", 1.0).unwrap(); fx.process(&x, &x, None, &mut l, &mut r);
        assert_eq!(peak(&l), 0.0);
    }

    #[test]
    fn new_shapes_are_bounded_and_tube_adds_even_harmonics() {
        for style in 0..SATURATION_STYLES.len() { for i in -500..=500 { let y = shape(style, i as f64 / 10.0); assert!(y.is_finite() && y.abs() < 1.6); } }
        assert!((shape(1, 0.5) + shape(1, -0.5)).abs() > 0.01, "tube transfer must actually be asymmetric");
        assert!((shape(0, 0.5) + shape(0, -0.5)).abs() < 1e-12);
        let mut fx = SaturationProcessor::new(FS); fx.set("mix", 0.0).unwrap();
        for x in [-1.0, -0.2, 0.0, 0.5, 1.0] { assert_eq!(fx.curve(x), x); }
    }

    #[test]
    fn band_level_trims_the_dry_portion_and_bypass_removes_band_processing() {
        let mut fx = SaturationProcessor::new(FS);
        for (id, value) in [("b1_mix", 0.0), ("b1_level", -6.0), ("output", -3.0)] { fx.set(id, value).unwrap(); }
        let left = [0.1; 32]; let right = [-0.2; 32];
        let (mut l, mut r) = ([0.0; 32], [0.0; 32]);
        fx.process(&left, &right, None, &mut l, &mut r);
        let gain = db_to_lin(-9.0);
        for i in 0..32 {
            assert!((l[i] as f64 - left[i] as f64 * gain).abs() < 1e-8);
            assert!((r[i] as f64 - right[i] as f64 * gain).abs() < 1e-8);
        }
        assert!((fx.band_curve(0, 0.1) - 0.1 * gain).abs() < 1e-12);
        for (id, value) in [("b1_on", 0.0), ("b1_pan", 100.0), ("b1_level", -60.0), ("output", 0.0)] { fx.set(id, value).unwrap(); }
        fx.process(&left, &right, None, &mut l, &mut r);
        assert_eq!(l, left); assert_eq!(r, right);
        assert_eq!(fx.band_curve(0, 0.1), 0.1);
        fx.set("b1_mute", 1.0).unwrap();
        fx.process(&left, &right, None, &mut l, &mut r);
        assert_eq!(l, [0.0; 32]); assert_eq!(r, [0.0; 32]);
        assert_eq!(fx.band_curve(0, 0.1), 0.0);
    }

    #[test]
    fn multiband_feedback_and_modulation_stay_finite_at_each_sample_rate() {
        for sample_rate in [44100.0, 48000.0, 96000.0] {
            let mut fx = SaturationProcessor::new(sample_rate);
            for (id, value) in [("band_count", 6.0), ("drive", 36.0), ("feedback", 90.0), ("mod_depth", 24.0), ("mod_rate", 20.0)] { fx.set(id, value).unwrap(); }
            let x = sine(440.0, 0.2, 8192); let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
            fx.process(&x, &x, None, &mut l, &mut r);
            assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 10.0));
        }
    }
    #[test]
    fn linear_crossover_quality_alignment_and_cabinet_are_real_audio() {
        let x = sine(4000.0, 0.2, 6000);
        let mut p = SaturationProcessor::new(FS);
        for (id, value) in [("phase", 1.0), ("oversampling", 2.0), ("band_count", 6.0)] { p.set(id, value).unwrap(); }
        for b in 1..=6 { p.set(&format!("b{b}_mix"), 0.0).unwrap(); }
        let mut left = vec![0.0; x.len()]; let mut right = left.clone(); p.process(&x,&x,None,&mut left,&mut right);
        assert_eq!(p.latency(), 96);
        assert!(left[96..].iter().zip(&x).all(|(a,b)| (a-b).abs()<1e-6));
        let mut cabinet = SaturationProcessor::new(FS); cabinet.set("cabinet", 4.0).unwrap();
        let mut raw = SaturationProcessor::new(FS);
        let mut reference = left.clone(); raw.process(&x,&x,None,&mut reference,&mut right);
        cabinet.process(&x,&x,None,&mut left,&mut right);
        assert!(rms_db(&left[3000..]) < rms_db(&reference[3000..]) - 2.0);
    }
    #[test]
    fn creative_styles_have_history_and_band_feedback_stays_independent() {
        let x = sine(700.0, 0.3, 6000); let mut signatures = Vec::new();
        for style in 0..SATURATION_STYLES.len() {
            let mut p = SaturationProcessor::new(FS); p.set("style",style as f64).unwrap(); p.set("drive",18.0).unwrap();
            let mut left = vec![0.0; x.len()]; let mut right = left.clone(); p.process(&x,&x,None,&mut left,&mut right);
            assert!(left.iter().all(|v| v.is_finite())); signatures.push(left);
        }
        for (index,pair) in signatures.windows(2).enumerate() { assert!(pair[0].iter().zip(&pair[1]).any(|(a,b)| (a-b).abs()>0.001), "styles {index}/{}", index+1); }
        let mut p = SaturationProcessor::new(FS); p.set("b1_channel", 1.0).unwrap(); p.set("drive", 30.0).unwrap();
        let mut left = vec![0.0; x.len()]; let mut right = left.clone(); p.process(&x,&x,None,&mut left,&mut right);
        assert_eq!(right,x); assert_ne!(left,x);
    }

}
