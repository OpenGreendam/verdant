//! Dynamics: compressor, gate/expander, de-esser and limiter. Every
//! detector can listen to the routed sidechain key instead of the input.

use super::filters::{db_to_lin, lin_to_db, rbj, time_coefficient, Biquad, DelayLine, Rbj, IDENTITY};
use super::{Spec, TableEffect};
use super::quality::{Oversampler, TruePeak, LoudnessMeter, LoudnessReading, LinearSplit};

/// Longest lookahead any dynamics effect offers, in milliseconds.
pub const MAX_LOOKAHEAD_MS: f64 = 20.0;

fn lookahead_samples(ms: f64, sample_rate: f64) -> usize { (ms.max(0.0) * 0.001 * sample_rate).round() as usize }

/// Stereo key filter: optional high-pass and low-pass (12 dB/oct each).
#[derive(Clone, Copy, Default)]
struct KeyFilter { hp: [Biquad; 2], lp: [Biquad; 2], hp_on: bool, lp_on: bool }

impl KeyFilter {
    fn set(&mut self, hp: f64, lp: f64, sample_rate: f64) {
        self.hp_on = hp > 20.5;
        self.lp_on = lp < 19_999.0;
        for f in &mut self.hp { f.set(if self.hp_on { rbj(Rbj::HighPass, hp, std::f64::consts::FRAC_1_SQRT_2, 0.0, sample_rate) } else { IDENTITY }); }
        for f in &mut self.lp { f.set(if self.lp_on { rbj(Rbj::LowPass, lp, std::f64::consts::FRAC_1_SQRT_2, 0.0, sample_rate) } else { IDENTITY }); }
    }
    #[inline]
    fn run(&mut self, l: f64, r: f64) -> (f64, f64) {
        let (mut l, mut r) = (l, r);
        if self.hp_on { l = self.hp[0].run(l); r = self.hp[1].run(r); }
        if self.lp_on { l = self.lp[0].run(l); r = self.lp[1].run(r); }
        (l, r)
    }
}

/// Audio delayed by the lookahead, so the detector sees peaks early.
#[derive(Clone, Default)]
struct Lookahead { lines: [DelayLine; 2], samples: usize }

impl Lookahead {
    fn new(sample_rate: f64) -> Self {
        let capacity = lookahead_samples(MAX_LOOKAHEAD_MS, sample_rate.max(8_000.0)) + 2;
        Self { lines: [DelayLine::new(capacity), DelayLine::new(capacity)], samples: 0 }
    }
    fn set(&mut self, ms: f64, sample_rate: f64) {
        let samples = lookahead_samples(ms, sample_rate).min(self.lines[0].capacity() - 1);
        if samples != self.samples { self.lines.iter_mut().for_each(DelayLine::clear); }
        self.samples = samples;
    }
    #[inline]
    fn run(&mut self, l: f64, r: f64) -> (f64, f64) {
        if self.samples == 0 { return (l, r); }
        self.lines[0].push(l);
        self.lines[1].push(r);
        (self.lines[0].tap(self.samples), self.lines[1].tap(self.samples))
    }
}

/// Soft-knee static gain computer (Giannoulis et al.): output level for an
/// input level, both in dBFS, before makeup.
pub fn compressor_curve_db(threshold: f64, ratio: f64, knee: f64, input_db: f64) -> f64 {
    let (t, r, w) = (threshold, ratio.max(1.0), knee.max(0.0));
    let over = input_db - t;
    if 2.0 * over < -w { input_db }
    else if w > 0.0 && 2.0 * over.abs() <= w { input_db + (1.0 / r - 1.0) * (over + w / 2.0).powi(2) / (2.0 * w) }
    else { t + over / r }
}

/// Downward expander/gate static curve: output level for an input level,
/// never attenuating by more than `range` dB.
pub fn expander_curve_db(threshold: f64, ratio: f64, range: f64, input_db: f64) -> f64 {
    if input_db >= threshold { return input_db; }
    let reduced = threshold + (input_db - threshold) * ratio.max(1.0);
    reduced.max(input_db - range.max(0.0))
}

// ---------------------------------------------------------------------------
// Compressor
// ---------------------------------------------------------------------------

// The first three style indices belong to existing saved projects.
pub const COMP_STYLES: [&str; 14] = ["干净", "平滑", "冲击", "人声", "总线", "光学", "向上", "泵动", "灵活", "三段双向", "光电", "变跨导", "经典", "母带"];
pub const DYNAMICS_CHANNEL_MODES: [&str; 3] = ["立体声", "中声道", "侧声道"];
pub const DETECTOR_MODES: [&str; 2] = ["峰值", "RMS"];
pub const COMP_CHARACTERS: [&str; 4] = ["关闭", "电子管", "二极管", "明亮"];

pub static COMP_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -60.0, 0.0, -18.0, "dB"),
    Spec::new("ratio", "压缩比", 1.0, 20.0, 3.0, ":1").log(),
    Spec::new("attack", "启动", 0.1, 200.0, 10.0, "ms").log(),
    Spec::new("release", "释放", 5.0, 2000.0, 120.0, "ms").log(),
    Spec::new("knee", "拐点", 0.0, 72.0, 6.0, "dB"),
    Spec::new("makeup", "补偿增益", 0.0, 24.0, 0.0, "dB"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("style", "风格", 0.0, 13.0, 0.0, "").steps(13),
    Spec::new("lookahead", "前视", 0.0, MAX_LOOKAHEAD_MS, 0.0, "ms").fixed(),
    Spec::new("range", "最大压缩", 0.0, 60.0, 60.0, "dB"),
    Spec::new("sc_hpf", "侧链高通", 20.0, 1000.0, 20.0, "Hz").log(),
    Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("sc_lpf", "侧链低通", 1000.0, 20000.0, 20000.0, "Hz").log(),
    Spec::new("hold", "保持", 0.0, 500.0, 0.0, "ms"),
    Spec::new("auto_release", "自动释放", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("auto_gain", "自动补偿", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("stereo_link", "立体声联动", 0.0, 100.0, 100.0, "%"),
    Spec::new("channel_mode", "处理声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("detection", "检测", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("input", "输入", -24.0, 24.0, 0.0, "dB"),
    Spec::new("output", "输出", -24.0, 24.0, 0.0, "dB"),
    Spec::new("character", "染色", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("character_drive", "染色驱动", -24.0, 24.0, 0.0, "dB"),
    Spec::new("character_pre", "压缩前染色", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("auto_threshold", "自动阈值", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("auto_target", "目标压缩量", 0.0, 24.0, 6.0, "dB"),
    Spec::new("oversampling", "过采样", 0.0, 5.0, 0.0, "").steps(5).fixed(),
    Spec::new("trigger_mode", "触发源", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("trigger_length", "触发长度", 1.0, 2000.0, 100.0, "ms").log(),
    Spec::new("sync", "触发节拍", 0.0, 8.0, 0.0, "").steps(8),
    Spec::new("bpm", "参考速度", 20.0, 300.0, 120.0, "BPM"),
    Spec::new("sc_eq_on", "侧链均衡器", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("sc1_on", "侧链 1 启用", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("sc1_freq", "侧链 1 频率", 20.0, 20000.0, 80.0, "Hz").log(),
    Spec::new("sc1_gain", "侧链 1 增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("sc1_q", "侧链 1 Q", 0.1, 12.0, 0.707, "Q").log(),
    Spec::new("sc1_shape", "侧链 1 形状", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("sc2_on", "侧链 2 启用", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("sc2_freq", "侧链 2 频率", 20.0, 20000.0, 250.0, "Hz").log(),
    Spec::new("sc2_gain", "侧链 2 增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("sc2_q", "侧链 2 Q", 0.1, 12.0, 0.707, "Q").log(),
    Spec::new("sc2_shape", "侧链 2 形状", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("sc3_on", "侧链 3 启用", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("sc3_freq", "侧链 3 频率", 20.0, 20000.0, 750.0, "Hz").log(),
    Spec::new("sc3_gain", "侧链 3 增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("sc3_q", "侧链 3 Q", 0.1, 12.0, 0.707, "Q").log(),
    Spec::new("sc3_shape", "侧链 3 形状", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("sc4_on", "侧链 4 启用", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("sc4_freq", "侧链 4 频率", 20.0, 20000.0, 2000.0, "Hz").log(),
    Spec::new("sc4_gain", "侧链 4 增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("sc4_q", "侧链 4 Q", 0.1, 12.0, 0.707, "Q").log(),
    Spec::new("sc4_shape", "侧链 4 形状", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("sc5_on", "侧链 5 启用", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("sc5_freq", "侧链 5 频率", 20.0, 20000.0, 6000.0, "Hz").log(),
    Spec::new("sc5_gain", "侧链 5 增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("sc5_q", "侧链 5 Q", 0.1, 12.0, 0.707, "Q").log(),
    Spec::new("sc5_shape", "侧链 5 形状", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("sc6_on", "侧链 6 启用", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("sc6_freq", "侧链 6 频率", 20.0, 20000.0, 12000.0, "Hz").log(),
    Spec::new("sc6_gain", "侧链 6 增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("sc6_q", "侧链 6 Q", 0.1, 12.0, 0.707, "Q").log(),
    Spec::new("sc6_shape", "侧链 6 形状", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("sc_channel", "侧链声道", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("auto_track", "阈值跟踪", 100.0, 5000.0, 1000.0, "ms").log(),
];

pub fn comp_sidechain_response_db(values: &[f64], frequency: f64, sample_rate: f64) -> f64 {
    if values.len() < COMP_SPECS.len() { return 0.0; }
    let mut db = 0.0;
    if values[10] > 20.5 { db += super::filters::response_db(rbj(Rbj::HighPass, values[10], std::f64::consts::FRAC_1_SQRT_2, 0.0, sample_rate), frequency, sample_rate); }
    if values[12] < 19999.0 { db += super::filters::response_db(rbj(Rbj::LowPass, values[12], std::f64::consts::FRAC_1_SQRT_2, 0.0, sample_rate), frequency, sample_rate); }
    if values[31] >= 0.5 {
        for band in 0..6 {
            let p = 32 + band * 5; if values[p] < 0.5 { continue; }
            let shape = match values[p + 4].round() as usize { 1 => Rbj::LowShelf, 2 => Rbj::HighShelf, 3 => Rbj::HighPass, 4 => Rbj::LowPass, _ => Rbj::Bell };
            db += super::filters::response_db(rbj(shape, values[p + 1], values[p + 3], values[p + 2], sample_rate), frequency, sample_rate);
        }
    }
    db
}

/// Gain-computer shared by audio and the native transfer-curve display.
pub fn compressor_style_curve_db(style: usize, threshold: f64, ratio: f64, knee: f64, range: f64, input_db: f64) -> f64 {
    if style == 6 {
        return input_db + ((threshold - input_db).max(0.0) * (1.0 - 1.0 / ratio.max(1.0))).min(range.min(24.0));
    }
    let (ratio, knee) = match style {
        3 => (ratio.max(2.0), knee.max(9.0)),
        4 => (ratio.min(6.0), knee.max(6.0)),
        5 => (ratio, knee.max(12.0)),
        7 => (ratio.max(6.0), knee * 0.25),
        8 => (ratio, knee.max(3.0)),
        9 => {
            let down = (input_db - threshold).max(0.0) * (1.0 - 1.0 / ratio.max(1.0));
            let up = (threshold - knee.max(6.0) - input_db).max(0.0) * (1.0 - 1.0 / ratio.sqrt().max(1.0));
            return input_db + up.min(range.min(24.0)) - down.min(range);
        }
        10 => (ratio, knee.max(15.0)),
        11 => (1.0 + (ratio - 1.0) * ((input_db - threshold + 18.0) / 36.0).clamp(0.0, 1.0), knee.max(18.0)),
        12 => (ratio, knee.max(4.0)),
        13 => (ratio.min(10.0), knee.max(12.0)),
        _ => (ratio, knee),
    };
    compressor_curve_db(threshold, ratio, knee, input_db).max(input_db - range)
}

fn stereo_domain(l: f64, r: f64, mode: usize) -> [f64; 2] {
    if mode == 0 { [l, r] } else { [(l + r) * 0.5, (l - r) * 0.5] }
}
fn from_stereo_domain(x: [f64; 2], mode: usize) -> [f64; 2] { if mode == 0 { x } else { [x[0] + x[1], x[0] - x[1]] } }
fn selected_channel(channel: usize, mode: usize) -> bool { mode == 0 || channel == mode - 1 }

fn compressor_character(kind: usize, x: f64, drive: f64) -> f64 {
    let z = x * drive;
    match kind {
        1 => ((z + 0.15).tanh() - 0.15f64.tanh()) / drive,
        2 => (z / (1.0 + z.abs())) / drive,
        3 => (z + 0.2 * z.powi(3)).tanh() / drive,
        _ => x,
    }
}

/// Stereo detector linking and M/S processing are independent: only selected
/// channels are reduced, and the sidechain never leaks into normal output.
pub struct CompProcessor {
    pub values: Vec<f64>, sample_rate: f64, attack: f64, release: f64,
    rms: [f64; 2], rms_coeff: f64, gr: [f64; 2], makeup: f64, wet: f64,
    last_out: [f64; 2], key: KeyFilter, lookahead: Lookahead, held: [usize; 2], hold_samples: usize,
    sc_eq: [[Biquad; 2]; 6], quality: [Oversampler; 2], dry_delay: [DelayLine; 2],
    average: f64, average_coefficient: f64, note: f64, trigger_decay: f64, beat: f64, host_tempo: Option<f64>,
    split: [[[Biquad; 2]; 2]; 2], band_gr: [[f64; 2]; 3],
}
impl CompProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(COMP_SPECS), sample_rate, attack: 0.0, release: 0.0,
            rms: [0.0; 2], rms_coeff: 0.0, gr: [0.0; 2], makeup: 1.0, wet: 1.0,
            last_out: [0.0; 2], key: KeyFilter::default(), lookahead: Lookahead::new(sample_rate), held: [0; 2], hold_samples: 0,
            sc_eq: Default::default(), quality: [Oversampler::new(), Oversampler::new()],
            dry_delay: [DelayLine::new(34), DelayLine::new(34)], average: 0.0, average_coefficient: 0.0,
            note: 0.0, trigger_decay: 0.0, beat: 1.0, host_tempo: None, split: Default::default(), band_gr: [[0.0; 2]; 3] };
        p.refresh(); p
    }
    pub fn gain_reduction_db(&self) -> f64 {
        if self.values[7].round() as usize == 9 { return self.band_gr.iter().flatten().copied().fold(0.0, f64::min); }
        if self.values[7].round() as usize == 6 { self.gr[0].max(self.gr[1]) } else { self.gr[0].min(self.gr[1]) }
    }
    pub fn curve_db(&self, input_db: f64) -> f64 {
        let v = &self.values;
        compressor_style_curve_db(v[7].round() as usize, v[0], v[1], v[4], v[9], input_db)
    }
    pub fn effective_threshold_db(&self) -> f64 {
        if self.values[24] < 0.5 || self.average < 1e-12 { self.values[0] } else {
            (lin_to_db(self.average.sqrt()) - self.values[25] / (1.0 - self.values[1].max(1.01).recip())).clamp(-60.0, 0.0)
        }
    }
    fn bands(&mut self, kind: usize, input: [f64; 2]) -> [[f64; 2]; 3] {
        let mut result = [[0.0; 2]; 3];
        for c in 0..2 {
            result[0][c] = self.split[kind][0][c].run(input[c]);
            let remainder = input[c] - result[0][c];
            result[1][c] = self.split[kind][1][c].run(remainder);
            result[2][c] = remainder - result[1][c];
        }
        result
    }
}
impl TableEffect for CompProcessor {
    const SPECS: &'static [Spec] = COMP_SPECS;
    fn gain_reduction_db(&self) -> Option<f64> { Some(CompProcessor::gain_reduction_db(self)) }
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values;
        let style = v[7].round() as usize;
        let attack = v[2] * match style { 5 => 2.0, 10 => 1.5, 11 => 0.6, 13 => 0.5, _ => 1.0 };
        let release = v[3] * match style { 5 => 2.5, 7 => 0.5, 10 => 3.0, 12 => 1.8, 13 => 1.5, _ => 1.0 };
        self.attack = time_coefficient(attack, self.sample_rate);
        self.release = time_coefficient(release, self.sample_rate);
        self.rms_coeff = time_coefficient(if style == 13 { 25.0 } else { 10.0 }, self.sample_rate);
        let compensation = if v[15] >= 0.5 { (-self.curve_db(-12.0) - 12.0).clamp(-24.0, 24.0) } else { 0.0 };
        self.makeup = db_to_lin(v[5] + compensation);
        self.wet = v[6] / 100.0;
        self.key.set(v[10], v[12], self.sample_rate);
        self.lookahead.set(v[8], self.sample_rate);
        self.hold_samples = (v[13] * 0.001 * self.sample_rate) as usize;
        self.quality.iter_mut().for_each(|q| q.set_quality(v[26].round() as usize));
        self.average_coefficient = time_coefficient(v[63], self.sample_rate);
        self.trigger_decay = time_coefficient(v[28], self.sample_rate);
        for band in 0..6 {
            let p = 32 + band * 5;
            let shape = match v[p + 4].round() as usize { 1 => Rbj::LowShelf, 2 => Rbj::HighShelf, 3 => Rbj::HighPass, 4 => Rbj::LowPass, _ => Rbj::Bell };
            let coefficient = if v[p] >= 0.5 { rbj(shape, v[p + 1], v[p + 3], v[p + 2], self.sample_rate) } else { IDENTITY };
            self.sc_eq[band].iter_mut().for_each(|f| f.set(coefficient));
        }
        for bank in &mut self.split { for (i, band) in bank.iter_mut().enumerate() {
            let coeff = rbj(Rbj::LowPass, if i == 0 { 90.0 } else { 2500.0 }, 0.707, 0.0, self.sample_rate);
            band.iter_mut().for_each(|f| f.set(coeff));
        } }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn latency(&self) -> u32 { (self.lookahead.samples + self.quality[0].latency()) as u32 }
    fn uses_transport_tempo() -> bool { true }
    fn set_transport_tempo(&mut self, bpm: f64) { if bpm.is_finite() && bpm >= 20.0 && bpm <= 1000.0 { self.host_tempo = Some(bpm); } }
    fn trigger_note(&mut self, velocity: f64) { if velocity.is_finite() && velocity > 0.0 { self.note = velocity.clamp(0.0, 1.0); } }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let style = self.values[7].round() as usize;
        let mode = self.values[17].round() as usize;
        let rms = self.values[18] >= 0.5 || matches!(style, 1 | 5 | 10 | 13);
        let listen = self.values[11] >= 0.5;
        let input = db_to_lin(self.values[19]); let output = db_to_lin(self.values[20]);
        let character = self.values[21].round() as usize;
        let drive = db_to_lin(self.values[22]); let pre = self.values[23] >= 0.5;
        let link = if mode == 0 { self.values[16] / 100.0 } else { 0.0 };
        let auto_release = self.values[14] >= 0.5 || matches!(style, 8 | 10 | 11 | 12 | 13);
        let sync = self.values[29].round() as usize;
        let beats = [1.0, 0.0625, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0][sync.min(8)];
        let bpm = self.host_tempo.unwrap_or(self.values[30]);
        let quality_delay = self.quality[0].latency();
        for i in 0..in_l.len() {
            let dry = [in_l[i] as f64 * input, in_r[i] as f64 * input];
            let signal = dry.map(|x| if pre { compressor_character(character, x, drive) } else { x });
            let (kl, kr) = key.map_or((signal[0], signal[1]), |(a, b)| (a[i] as f64, b[i] as f64));
            let (kl, kr) = self.key.run(kl, kr);
            let mut key_audio = [kl, kr];
            if self.values[31] >= 0.5 { for band in &mut self.sc_eq { for c in 0..2 { key_audio[c] = band[c].run(key_audio[c]); } } }
            match self.values[62].round() as usize {
                1 => key_audio = [(key_audio[0] + key_audio[1]) * 0.5; 2],
                2 => key_audio = [(key_audio[0] - key_audio[1]) * 0.5; 2],
                3 => key_audio[1] = key_audio[0], 4 => key_audio[0] = key_audio[1], _ => {}
            }
            if self.values[27] >= 1.5 {
                self.beat += bpm / (60.0 * self.sample_rate * beats);
                if self.beat >= 1.0 { self.beat = self.beat.fract(); self.note = 1.0; }
            }
            if self.values[27] >= 0.5 { key_audio = [self.note; 2]; }
            self.note *= self.trigger_decay;
            let detector = stereo_domain(key_audio[0], key_audio[1], mode);
            let energy = 0.5 * (detector[0] * detector[0] + detector[1] * detector[1]);
            self.average = energy + self.average_coefficient * (self.average - energy);
            let threshold = self.effective_threshold_db();
            let mut target = [0.0; 2];
            for c in 0..2 {
                let mut level = if matches!(style, 2 | 11 | 12) && key.is_none() && self.values[27] < 0.5 { self.last_out[c] } else { detector[c].abs() };
                if rms { self.rms[c] = level * level + self.rms_coeff * (self.rms[c] - level * level); level = self.rms[c].max(0.0).sqrt(); }
                let db = lin_to_db(level);
                target[c] = compressor_style_curve_db(style, threshold, self.values[1], self.values[4], self.values[9], db) - db;
                if matches!(style, 6 | 9) && level < 1e-8 { target[c] = 0.0; }
            }
            let linked = if style == 6 { target[0].max(target[1]) } else { target[0].min(target[1]) };
            for c in 0..2 {
                let target = if selected_channel(c, mode) { target[c] + (linked - target[c]) * link } else { 0.0 };
                let engaging = if style == 6 { target > self.gr[c] } else { target < self.gr[c] };
                if engaging { self.held[c] = self.hold_samples; self.gr[c] = target + self.attack * (self.gr[c] - target); }
                else if self.held[c] > 0 { self.held[c] -= 1; }
                else { let release = if auto_release { self.release.powf((1.0 + self.gr[c].abs() / 12.0).recip()) } else { self.release };
                    self.gr[c] = target + release * (self.gr[c] - target); }
            }
            let (dl, dr) = self.lookahead.run(dry[0], dry[1]);
            let mut audio = stereo_domain(dl, dr, mode);
            let mut ttm = [0.0; 2];
            if style == 9 {
                let bands = self.bands(0, audio); let detection = self.bands(1, detector);
                for b in 0..3 { for c in 0..2 {
                    let level = detection[b][c].abs(); let db = lin_to_db(level);
                    let target = if level < 1e-8 || !selected_channel(c, mode) { 0.0 } else {
                        compressor_style_curve_db(9, threshold - [3.0, 0.0, 6.0][b], self.values[1], self.values[4], self.values[9], db) - db };
                    let coefficient = if target.abs() > self.band_gr[b][c].abs() { self.attack } else { self.release };
                    self.band_gr[b][c] = target + coefficient * (self.band_gr[b][c] - target);
                    ttm[c] += bands[b][c] * db_to_lin(self.band_gr[b][c]);
                } }
            }
            for c in 0..2 {
                let gain = if selected_channel(c, mode) { db_to_lin(self.gr[c]) * self.makeup } else { 1.0 };
                let processed_input = if style == 9 { ttm[c] * self.makeup } else { audio[c] };
                let processed_gain = if style == 9 { 1.0 } else { gain };
                let selected = selected_channel(c, mode);
                let processed = self.quality[c].run(processed_input, |x| {
                    if !selected { x } else if pre { compressor_character(character, x, drive) * processed_gain }
                    else { compressor_character(character, x * processed_gain, drive) }
                });
                self.last_out[c] = (audio[c] * db_to_lin(self.gr[c])).abs();
                self.dry_delay[c].push(audio[c]);
                let delayed = if quality_delay > 0 { self.dry_delay[c].tap(quality_delay) } else { audio[c] };
                audio[c] = (delayed * (1.0 - self.wet) + processed * self.wet) * output;
            }
            let y = from_stereo_domain(audio, mode);
            if listen { out_l[i] = key_audio[0] as f32; out_r[i] = key_audio[1] as f32; }
            else { out_l[i] = y[0] as f32; out_r[i] = y[1] as f32; }
        }
    }
}

// ---------------------------------------------------------------------------
// Gate / expander
// ---------------------------------------------------------------------------

pub const GATE_STYLES: [&str; 6] = ["经典", "干净", "人声", "吉他", "向上", "闪避"];

pub static GATE_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -80.0, 0.0, -40.0, "dB"),
    Spec::new("range", "衰减范围", 0.0, 80.0, 40.0, "dB"),
    Spec::new("ratio", "扩展比", 1.0, 50.0, 50.0, ":1").log(),
    Spec::new("attack", "启动", 0.01, 50.0, 0.5, "ms").log(),
    Spec::new("hold", "保持", 0.0, 500.0, 50.0, "ms"),
    Spec::new("release", "释放", 5.0, 2000.0, 150.0, "ms").log(),
    Spec::new("lookahead", "前视", 0.0, 10.0, 0.0, "ms").fixed(),
    Spec::new("sc_hpf", "侧链高通", 20.0, 2000.0, 20.0, "Hz").log(),
    Spec::new("sc_lpf", "侧链低通", 1000.0, 20000.0, 20000.0, "Hz").log(),
    Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("style", "风格", 0.0, 5.0, 0.0, "").steps(5),
    Spec::new("knee", "拐点", 0.0, 24.0, 0.0, "dB"),
    Spec::new("hysteresis", "迟滞", 0.0, 12.0, 0.0, "dB"),
    Spec::new("stereo_link", "立体声联动", 0.0, 100.0, 100.0, "%"),
    Spec::new("channel_mode", "处理声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("output", "输出", -24.0, 24.0, 0.0, "dB"),
    Spec::new("detection", "检测", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("sc_channel", "侧链声道", 0.0, 4.0, 0.0, "").steps(4),
    Spec::new("program_release", "自适应释放", 0.0, 100.0, 0.0, "%"),
];

pub fn gate_style_curve_db(style: usize, threshold: f64, ratio: f64, range: f64, knee: f64, input_db: f64) -> f64 {
    if style == 4 { return input_db + ((input_db - threshold).max(0.0) * (ratio - 1.0)).min(range.min(24.0)); }
    if style == 5 { return compressor_curve_db(threshold, ratio, knee, input_db).max(input_db - range); }
    let ratio = if style == 3 { ratio.min(5.0) } else { ratio };
    let knee = if style == 1 || style == 2 { knee.max(6.0) } else { knee };
    let over = input_db - threshold;
    let reduction = if over >= knee * 0.5 { 0.0 }
        else if knee > 0.0 && over > -knee * 0.5 { -(ratio - 1.0) * (over - knee * 0.5).powi(2) / (2.0 * knee) }
        else { (ratio - 1.0) * over };
    input_db + reduction.max(-range)
}

pub struct GateProcessor {
    pub values: Vec<f64>, sample_rate: f64, attack: f64, release: f64,
    hold_samples: usize, held: [usize; 2], open: [bool; 2], gr: [f64; 2],
    envelope: [f64; 2], envelope_attack: f64, envelope_release: f64,
    key: KeyFilter, lookahead: Lookahead, rms_energy: [f64; 2],
}
impl GateProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(GATE_SPECS), sample_rate, attack: 0.0, release: 0.0,
            hold_samples: 0, held: [0; 2], open: [false; 2], gr: [0.0; 2], envelope: [0.0; 2], envelope_attack: 0.0, envelope_release: 0.0,
            key: KeyFilter::default(), lookahead: Lookahead::new(sample_rate), rms_energy: [0.0; 2] };
        p.refresh(); p
    }
    pub fn gain_reduction_db(&self) -> f64 { if self.values[11].round() as usize == 4 { self.gr[0].max(self.gr[1]) } else { self.gr[0].min(self.gr[1]) } }
    pub fn curve_db(&self, input_db: f64) -> f64 {
        gate_style_curve_db(self.values[11].round() as usize, self.values[0], self.values[2], self.values[1], self.values[12], input_db)
    }
}
impl TableEffect for GateProcessor {
    const SPECS: &'static [Spec] = GATE_SPECS;
    fn gain_reduction_db(&self) -> Option<f64> { Some(GateProcessor::gain_reduction_db(self)) }
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values;
        let style = v[11].round() as usize;
        self.attack = time_coefficient(if style == 2 { v[3].max(2.0) } else { v[3] }, self.sample_rate);
        self.release = time_coefficient(v[5] * if style == 3 { 1.5 } else { 1.0 }, self.sample_rate);
        self.envelope_attack = time_coefficient(0.1, self.sample_rate);
        self.envelope_release = time_coefficient(if style == 1 { 20.0 } else { 10.0 }, self.sample_rate);
        self.hold_samples = (v[4] * 0.001 * self.sample_rate) as usize;
        self.key.set(v[7], v[8], self.sample_rate); self.lookahead.set(v[6], self.sample_rate);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn latency(&self) -> u32 { self.lookahead.samples as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let listen = self.values[9] >= 0.5; let wet = self.values[10] / 100.0;
        let style = self.values[11].round() as usize; let mode = self.values[15].round() as usize;
        let link = if mode == 0 { self.values[14] / 100.0 } else { 0.0 };
        let output = db_to_lin(self.values[16]);
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let (kl, kr) = key.map_or((l, r), |(a, b)| (a[i] as f64, b[i] as f64));
            let (kl, kr) = self.key.run(kl, kr);
            let mut selected_key = [kl, kr];
            match self.values[18].round() as usize {
                1 => selected_key = [(kl + kr) * 0.5; 2], 2 => selected_key = [(kl - kr) * 0.5; 2],
                3 => selected_key[1] = kl, 4 => selected_key[0] = kr, _ => {}
            }
            let mut detector = stereo_domain(selected_key[0], selected_key[1], mode).map(f64::abs);
            if self.values[17] >= 0.5 { for c in 0..2 {
                self.rms_energy[c] += (detector[c] * detector[c] - self.rms_energy[c]) * (1.0 - self.envelope_release);
                detector[c] = self.rms_energy[c].max(0.0).sqrt();
            } }
            let linked = detector[0].max(detector[1]);
            for c in 0..2 {
                let level = detector[c] + (linked - detector[c]) * link;
                let e = if level > self.envelope[c] { self.envelope_attack } else { self.envelope_release };
                self.envelope[c] = level + e * (self.envelope[c] - level);
                let db = lin_to_db(self.envelope[c]);
                if db >= self.values[0] { self.open[c] = true; self.held[c] = self.hold_samples; }
                else if db < self.values[0] - self.values[13] {
                    if self.held[c] > 0 { self.held[c] -= 1; } else { self.open[c] = false; }
                }
                let target = if !selected_channel(c, mode) { 0.0 }
                    else if style < 4 && self.open[c] { 0.0 }
                    else { self.curve_db(db) - db };
                let opening = if style == 5 { target < self.gr[c] } else { target > self.gr[c] };
                let coeff = if opening { self.attack } else { self.release.powf((1.0 + self.values[19] * 0.01 * self.gr[c].abs() / 20.0).recip()) };
                self.gr[c] = target + coeff * (self.gr[c] - target);
            }
            let (dl, dr) = self.lookahead.run(l, r);
            if listen { out_l[i] = selected_key[0] as f32; out_r[i] = selected_key[1] as f32; continue; }
            let mut audio = stereo_domain(dl, dr, mode);
            for c in 0..2 {
                let gain = if selected_channel(c, mode) { db_to_lin(self.gr[c]) * wet + 1.0 - wet } else { 1.0 };
                audio[c] *= gain * output;
            }
            let y = from_stereo_domain(audio, mode); out_l[i] = y[0] as f32; out_r[i] = y[1] as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// De-esser
// ---------------------------------------------------------------------------

pub const DEESSER_MODES: [&str; 2] = ["分频", "全频"];

pub const DEESSER_DETECTION: [&str; 3] = ["全能", "单人声", "自适应齿音"];
pub static DEESSER_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -60.0, 0.0, -24.0, "dB"),
    Spec::new("range", "最大衰减", 0.0, 24.0, 12.0, "dB"),
    Spec::new("freq", "频率", 2000.0, 16000.0, 6000.0, "Hz").log(),
    Spec::new("mode", "模式", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("release", "释放", 10.0, 500.0, 60.0, "ms").log(),
    Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("detection", "检测算法", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("sc_lpf", "检测上限", 4000.0, 20000.0, 20000.0, "Hz").log(),
    Spec::new("lookahead", "前视", 0.0, 15.0, 0.0, "ms").fixed(),
    Spec::new("stereo_link", "立体声联动", 0.0, 100.0, 100.0, "%"),
    Spec::new("channel_mode", "处理声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("attack", "启动", 0.05, 20.0, 0.5, "ms").log(),
    Spec::new("phase", "分频相位", 0.0, 1.0, 0.0, "").steps(1).fixed(),
    Spec::new("sensitivity", "齿音灵敏度", 0.0, 100.0, 50.0, "%"),
    Spec::new("transient_protect", "瞬态保护", 0.0, 100.0, 0.0, "%"),
    Spec::new("hold", "齿音保持", 0.0, 250.0, 0.0, "ms"),
];

/// Linkwitz–Riley 24 dB/oct split per channel: the bands sum to an allpass,
/// so the unprocessed signal keeps a flat magnitude.
#[derive(Clone, Copy, Default)]
struct HighBand { high: [[Biquad; 2]; 2], low: [[Biquad; 2]; 2] }

impl HighBand {
    fn set(&mut self, freq: f64, sample_rate: f64) {
        let q = std::f64::consts::FRAC_1_SQRT_2;
        let (hp, lp) = (rbj(Rbj::HighPass, freq, q, 0.0, sample_rate), rbj(Rbj::LowPass, freq, q, 0.0, sample_rate));
        for channel in 0..2 { for k in 0..2 { self.high[channel][k].set(hp); self.low[channel][k].set(lp); } }
    }
    /// (low, high) bands of one sample.
    #[inline]
    fn run(&mut self, channel: usize, x: f64) -> (f64, f64) {
        let h = &mut self.high[channel];
        let t = h[0].run(x);
        let high = h[1].run(t);
        let l = &mut self.low[channel];
        let t = l[0].run(x);
        (l[1].run(t), high)
    }
}

pub struct DeesserProcessor {
    pub values: Vec<f64>, sample_rate: f64, attack: f64, release: f64, gr: [f64; 2],
    split: HighBand, detect: HighBand, upper: KeyFilter, lookahead: Lookahead,
    energy: [[f64; 2]; 2], energy_coeff: f64, linear: [LinearSplit; 2], dry_linear: [DelayLine; 2],
    analysis: [[Biquad; 2]; 6], spectral_energy: [[f64; 2]; 6], previous_energy: [f64; 2], held: [usize; 2],
}
impl DeesserProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(DEESSER_SPECS), sample_rate, attack: 0.0, release: 0.0, gr: [0.0; 2],
            split: HighBand::default(), detect: HighBand::default(), upper: KeyFilter::default(), lookahead: Lookahead::new(sample_rate),
            energy: [[0.0; 2]; 2], energy_coeff: 0.0, linear: [LinearSplit::default(), LinearSplit::default()],
            dry_linear: [DelayLine::new(66), DelayLine::new(66)], analysis: Default::default(),
            spectral_energy: [[0.0; 2]; 6], previous_energy: [0.0; 2], held: [0; 2] };
        p.refresh(); p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.gr[0].min(self.gr[1]) }
    pub fn reduction_db(&self, level_db: f64) -> f64 { -((level_db - self.values[0]) * 0.75).clamp(0.0, self.values[1]) }
}
impl TableEffect for DeesserProcessor {
    const SPECS: &'static [Spec] = DEESSER_SPECS;
    fn gain_reduction_db(&self) -> Option<f64> { Some(DeesserProcessor::gain_reduction_db(self)) }
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        self.attack = time_coefficient(self.values[12], self.sample_rate);
        self.release = time_coefficient(self.values[4], self.sample_rate);
        self.energy_coeff = time_coefficient(5.0, self.sample_rate);
        self.split.set(self.values[2], self.sample_rate); self.detect.set(self.values[2], self.sample_rate);
        self.upper.set(20.0, self.values[7].max(self.values[2]), self.sample_rate);
        self.lookahead.set(self.values[8], self.sample_rate);
        self.linear.iter_mut().for_each(|f| f.configure(self.values[2], self.sample_rate));
        for (b, bank) in self.analysis.iter_mut().enumerate() {
            let frequency = self.values[2] * (self.values[7].max(self.values[2] * 1.1) / self.values[2]).powf(b as f64 / 5.0);
            let coefficient = rbj(Rbj::BandPass, frequency, 2.5, 0.0, self.sample_rate);
            bank.iter_mut().for_each(|f| f.set(coefficient));
        }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn latency(&self) -> u32 { (self.lookahead.samples + if self.values[13] >= 0.5 { LinearSplit::LATENCY } else { 0 }) as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let listen = self.values[5] >= 0.5; let wide = self.values[3] >= 0.5;
        let vocal = self.values[6] >= 0.5; let mode = self.values[10].round() as usize;
        let link = if mode == 0 { self.values[9] / 100.0 } else { 0.0 }; let wet = self.values[11] / 100.0;
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let (kl, kr) = key.map_or((l, r), |(a, b)| (a[i] as f64, b[i] as f64));
            let detector = stereo_domain(kl, kr, mode);
            let high = [self.detect.run(0, detector[0]).1, self.detect.run(1, detector[1]).1];
            let (hl, hr) = self.upper.run(high[0], high[1]); let high = [hl, hr];
            let mut target = [0.0; 2];
            for c in 0..2 {
                self.energy[c][0] = high[c] * high[c] + self.energy_coeff * (self.energy[c][0] - high[c] * high[c]);
                self.energy[c][1] = detector[c] * detector[c] + self.energy_coeff * (self.energy[c][1] - detector[c] * detector[c]);
                // A vocal detector requires appreciable sibilant energy, so
                // strong voiced low harmonics cannot trigger de-essing alone.
                let mut confidence = if vocal { ((self.energy[c][0] / self.energy[c][1].max(1e-15) - 0.08) / 0.32).clamp(0.0, 1.0) } else { 1.0 };
                if self.values[6] >= 1.5 {
                    let mut log_sum = 0.0; let mut sum = 0.0;
                    for b in 0..6 {
                        let sample = self.analysis[b][c].run(detector[c]);
                        self.spectral_energy[b][c] = sample * sample + self.energy_coeff * (self.spectral_energy[b][c] - sample * sample);
                        let energy = self.spectral_energy[b][c].max(1e-20); sum += energy; log_sum += energy.ln();
                    }
                    let flatness = ((log_sum / 6.0).exp() / (sum / 6.0).max(1e-20)).clamp(0.0, 1.0);
                    confidence *= (flatness * (1.0 + self.values[14] * 0.06)).clamp(0.0, 1.0);
                }
                let rise = ((self.energy[c][1] - self.previous_energy[c]).max(0.0) / self.energy[c][1].max(1e-20) * 20.0).min(1.0);
                confidence *= 1.0 - rise * self.values[15] * 0.01;
                self.previous_energy[c] = self.energy[c][1];
                target[c] = self.reduction_db(lin_to_db(if self.values[6] >= 1.5 { self.energy[c][0].sqrt() } else { high[c].abs() })) * confidence;
            }
            let linked = target[0].min(target[1]);
            for c in 0..2 {
                let target = if selected_channel(c, mode) { target[c] + (linked - target[c]) * link } else { 0.0 };
                if target < self.gr[c] {
                    self.held[c] = (self.values[16] * 0.001 * self.sample_rate) as usize;
                    self.gr[c] = target + self.attack * (self.gr[c] - target);
                } else if self.held[c] > 0 { self.held[c] -= 1; }
                else { self.gr[c] = target + self.release * (self.gr[c] - target); }
            }
            let (dl, dr) = self.lookahead.run(l, r);
            if listen { let y = from_stereo_domain(high, mode); out_l[i] = y[0] as f32; out_r[i] = y[1] as f32; continue; }
            let mut audio = stereo_domain(dl, dr, mode);
            for c in 0..2 {
                let gain = if selected_channel(c, mode) { 1.0 + (db_to_lin(self.gr[c]) - 1.0) * wet } else { 1.0 };
                self.dry_linear[c].push(audio[c]);
                if self.values[13] >= 0.5 {
                    let (low, high) = self.linear[c].run(audio[c]);
                    let dry = self.dry_linear[c].tap(LinearSplit::LATENCY);
                    audio[c] = if wide { dry * gain } else if wet > 0.0 && selected_channel(c, mode) { low + high * gain } else { dry };
                } else if wide { audio[c] *= gain; }
                else { let (low, high) = self.split.run(c, audio[c]);
                    if wet > 0.0 && selected_channel(c, mode) { audio[c] = low + high * gain; }
                }
            }
            let y = from_stereo_domain(audio, mode); out_l[i] = y[0] as f32; out_r[i] = y[1] as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// Limiter
// ---------------------------------------------------------------------------

pub const LIMITER_STYLES: [&str; 8] = ["透明", "平滑", "冲击", "安全", "激进", "现代", "总线", "动态"];
pub static LIMITER_SPECS: &[Spec] = &[
    Spec::new("gain", "输入增益", 0.0, 24.0, 0.0, "dB"),
    Spec::new("ceiling", "输出上限", -12.0, 0.0, -0.3, "dB"),
    Spec::new("release", "释放", 1.0, 1000.0, 50.0, "ms").log(),
    Spec::new("lookahead", "前视", 0.1, 10.0, 3.0, "ms").fixed(),
    Spec::new("link", "立体声联动", 0.0, 100.0, 100.0, "%"),
    Spec::new("style", "风格", 0.0, 7.0, 0.0, "").steps(7),
    Spec::new("dc_filter", "去直流", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("unity_gain", "等响比较", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("audition", "监听限制差分", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("attack", "平滑启动", 0.0, 10.0, 3.0, "ms"),
    Spec::new("true_peak", "重建峰值保护", 0.0, 1.0, 0.0, "").steps(1).fixed(),
    Spec::new("oversampling", "过采样", 0.0, 5.0, 0.0, "").steps(5).fixed(),
    Spec::new("loudness_match", "响度目标", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("target_lufs", "目标响度", -36.0, -5.0, -23.0, "LUFS"),
    Spec::new("dither", "抖动", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("dither_bits", "目标位深", 8.0, 24.0, 24.0, "bit").steps(16),
    Spec::new("meter_pause", "暂停积分", 0.0, 1.0, 0.0, "").steps(1).fixed(),
    Spec::new("meter_reset", "重置计量", 0.0, 1.0, 0.0, "").steps(1).fixed(),
];

/// Fixed-capacity monotone queue: O(1) amortized minimum, no allocation in
/// the audio thread. The original per-sample window scan became expensive
/// at 96/192 kHz and long lookahead settings.
struct SlidingMinimum { entries: Vec<(u64, f64)>, head: usize, len: usize }
impl SlidingMinimum {
    fn new(capacity: usize) -> Self { Self { entries: vec![(0, 1.0); capacity], head: 0, len: 0 } }
    fn clear(&mut self) { self.head = 0; self.len = 0; }
    fn push(&mut self, clock: u64, value: f64, window: usize) -> f64 {
        let cap = self.entries.len();
        while self.len > 0 && clock.saturating_sub(self.entries[self.head].0) > window as u64 { self.head = (self.head + 1) % cap; self.len -= 1; }
        while self.len > 0 && self.entries[(self.head + self.len - 1) % cap].1 >= value { self.len -= 1; }
        self.entries[(self.head + self.len) % cap] = (clock, value); self.len += 1;
        self.entries[self.head].1
    }
}

/// One shared stereo gain engine. Quality changes its sampling rate; both
/// the sidechain and main path are interpolated before gain computation.
struct LimiterCore {
    window: usize, audio: [DelayLine; 2], minimum: [SlidingMinimum; 2],
    env: [f64; 2], held: [DelayLine; 2], sum: [f64; 2], clock: u64, gr_db: f64,
}
impl LimiterCore {
    fn new(sample_rate: f64) -> Self {
        let capacity = lookahead_samples(10.0, sample_rate.max(8000.0) * 32.0) + 2;
        Self { window: 1, audio: [DelayLine::new(capacity), DelayLine::new(capacity)],
            minimum: [SlidingMinimum::new(capacity), SlidingMinimum::new(capacity)], env: [1.0; 2],
            held: [DelayLine::new(capacity), DelayLine::new(capacity)], sum: [1.0; 2], clock: 0, gr_db: 0.0 }
    }
    fn reset(&mut self, window: usize) {
        self.window = window.clamp(1, self.audio[0].capacity() - 2);
        self.clock = 0; self.gr_db = 0.0;
        for c in 0..2 {
            self.audio[c].clear(); self.minimum[c].clear(); self.held[c].clear();
            for _ in 0..self.held[c].capacity() { self.held[c].push(1.0); }
            self.env[c] = 1.0; self.sum[c] = self.window as f64;
        }
    }
    fn run(&mut self, x: [f64; 2], trigger: [f64; 2], ceiling: f64, release: f64, link: f64, smooth: f64, style: usize) -> ([f64; 2], [f64; 2]) {
        let own: [f64; 2] = std::array::from_fn(|c| (ceiling / x[c].abs().max(trigger[c].abs()).max(1e-12)).min(1.0));
        let linked = own[0].min(own[1]); let mut y = [0.0; 2]; let mut dry = [0.0; 2];
        for c in 0..2 {
            let need = own[c] + (linked - own[c]) * link;
            let hold = self.minimum[c].push(self.clock, need, self.window);
            let release = match style {
                5 => release.powf((0.5 + 1.5 * self.env[c]).clamp(0.5, 2.0)),
                6 => release.powf(0.5 + 0.5 * (self.env[0] + self.env[1])),
                7 => release.powf((1.0 + 3.0 * (hold - self.env[c]).abs()).recip()),
                _ => release,
            };
            self.env[c] = if hold < self.env[c] { hold } else { hold + release * (self.env[c] - hold) };
            self.held[c].push(self.env[c]);
            self.sum[c] += self.env[c] - self.held[c].tap(self.window);
            let average = (self.sum[c] / self.window as f64).clamp(0.0, 1.0);
            let smoothing = if style == 4 { smooth * 0.15 } else { smooth };
            let gain = self.env[c] + (average - self.env[c]) * smoothing;
            self.audio[c].push(x[c]); dry[c] = self.audio[c].tap(self.window);
            y[c] = (dry[c] * gain).clamp(-ceiling, ceiling);
            if c == 0 { self.gr_db = lin_to_db(gain); } else { self.gr_db = self.gr_db.min(lin_to_db(gain)); }
        }
        self.clock = self.clock.wrapping_add(1);
        (y, dry)
    }
}

/// ISP guard using every FIR support's absolute-contribution bound. A sample
/// waits until all 33 supports containing it have been seen, then receives
/// their minimum gain. This conservative bound stays valid during gain
/// changes, unlike measuring a reconstructed peak and applying a late clip.
struct IspGuard {
    detector: [TruePeak; 2], minimum: [SlidingMinimum; 2], audio: [DelayLine; 2], clock: u64, gain: f64,
}
impl IspGuard {
    fn new() -> Self { Self { detector: [TruePeak::new(), TruePeak::new()],
        minimum: [SlidingMinimum::new(35), SlidingMinimum::new(35)], audio: [DelayLine::new(35), DelayLine::new(35)], clock: 0, gain: 1.0 } }
    fn reset(&mut self) {
        self.detector.iter_mut().for_each(TruePeak::reset); self.minimum.iter_mut().for_each(SlidingMinimum::clear);
        self.audio.iter_mut().for_each(DelayLine::clear); self.clock = 0; self.gain = 1.0;
    }
    fn run(&mut self, input: [f64; 2], ceiling: f64, link: f64) -> [f64; 2] {
        let bounds = std::array::from_fn::<_, 2, _>(|c| self.detector[c].run_bounded(input[c]).1);
        let own = bounds.map(|v| (ceiling / v.max(1e-12)).min(1.0)); let linked = own[0].min(own[1]);
        let mut y = [0.0; 2]; self.gain = 1.0;
        for c in 0..2 {
            let gain = self.minimum[c].push(self.clock, own[c] + (linked - own[c]) * link, 32);
            self.audio[c].push(input[c]); y[c] = self.audio[c].tap(32) * gain; self.gain = self.gain.min(gain);
        }
        self.clock += 1; y
    }
}

pub struct LimiterProcessor {
    pub values: Vec<f64>, sample_rate: f64, core: LimiterCore, quality: [Oversampler; 2], key_quality: [Oversampler; 2],
    isp: IspGuard, dry_isp: [DelayLine; 2], dc: [(f64, f64); 2], release: f64, drive: f64, ceiling: f64,
    meter: LoudnessMeter, input_meter: LoudnessMeter, target_gain_db: f64,
    random: [u64; 2], error: [[f64; 2]; 2], last_reset: bool, last_isp: bool, meter_epoch: u64,
}
impl LimiterProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(LIMITER_SPECS), sample_rate, core: LimiterCore::new(sample_rate),
            quality: [Oversampler::new(), Oversampler::new()], key_quality: [Oversampler::new(), Oversampler::new()], isp: IspGuard::new(),
            dry_isp: [DelayLine::new(35), DelayLine::new(35)], dc: [(0.0, 0.0); 2], release: 0.0, drive: 1.0, ceiling: 1.0,
            meter: LoudnessMeter::new(sample_rate), input_meter: LoudnessMeter::new(sample_rate), target_gain_db: 0.0,
            random: [0x4d595df4d0f33173, 0x94d049bb133111eb], error: [[0.0; 2]; 2], last_reset: false, last_isp: false, meter_epoch: 0 };
        p.refresh(); p.core.reset(p.core.window); p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.core.gr_db + if self.last_isp { lin_to_db(self.isp.gain) } else { 0.0 } }
    pub fn loudness(&self) -> LoudnessReading { self.meter.reading() }
    fn noise(&mut self, channel: usize) -> f64 {
        let mut value = self.random[channel]; value ^= value << 13; value ^= value >> 7; value ^= value << 17; self.random[channel] = value;
        (value >> 11) as f64 / (1u64 << 53) as f64
    }
}
impl TableEffect for LimiterProcessor {
    const SPECS: &'static [Spec] = LIMITER_SPECS;
    fn gain_reduction_db(&self) -> Option<f64> { Some(LimiterProcessor::gain_reduction_db(self)) }
    fn meter_control(&self) -> (bool, u64) { (self.values[16] >= 0.5, self.meter_epoch) }
    fn reset_meters(&mut self) {
        self.meter.reset(); self.input_meter.reset();
        self.meter_epoch = self.meter_epoch.wrapping_add(1);
        // Preserve target-gain smoothing and all signal-path history.
    }
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values; self.drive = db_to_lin(v[0]); self.ceiling = db_to_lin(v[1]);
        let isp = v[10] >= 0.5;
        let quality = (v[11].round() as usize).max(if isp { 2 } else { 0 });
        let factor = 1usize << quality.min(5);
        let rate = self.sample_rate * factor as f64;
        let release_scale = match v[5].round() as usize { 1 => 2.0, 2 => 0.5, 3 => 3.0, 4 => 0.2, 5 => 1.2, 6 => 1.8, 7 => 0.7, _ => 1.0 };
        self.release = time_coefficient(v[2] * release_scale, rate);
        let window = lookahead_samples(v[3], self.sample_rate).max(1) * factor;
        if self.core.window != window || self.quality[0].factor() != factor || isp != self.last_isp {
            self.core.reset(window); self.isp.reset(); self.dry_isp.iter_mut().for_each(DelayLine::clear);
        }
        self.quality.iter_mut().chain(&mut self.key_quality).for_each(|q| q.set_quality(quality));
        self.last_isp = isp;
        self.meter.set_paused(v[16] >= 0.5); self.input_meter.set_paused(v[16] >= 0.5);
        let reset = v[17] >= 0.5;
        if reset && !self.last_reset { self.reset_meters(); }
        self.last_reset = reset;
    }
    fn set_sample_rate(&mut self, sample_rate: f64) {
        let values = self.values.clone(); let epoch = self.meter_epoch;
        *self = Self::new(sample_rate); self.values = values; self.refresh();
        self.meter_epoch = epoch.wrapping_add(1);
    }
    fn latency(&self) -> u32 { (self.core.window / self.quality[0].factor() + self.quality[0].latency() + if self.last_isp { 32 } else { 0 }) as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let link = self.values[4] / 100.0; let style = self.values[5].round() as usize;
        let factor = self.quality[0].factor(); let rate = self.sample_rate * factor as f64;
        let smooth = (self.values[9] * 0.001 * rate / self.core.window as f64).clamp(0.0, 1.0);
        let dc_coeff = (-std::f64::consts::TAU * 5.0 / self.sample_rate).exp();
        let dither = self.values[14].round() as usize; let quantum = 2f64.powf(1.0 - self.values[15].round());
        let ceiling = if dither > 0 { (self.ceiling - 8.0 * quantum).max(0.01) } else { self.ceiling };
        let tracking = time_coefficient(1000.0, self.sample_rate);
        for i in 0..in_l.len() {
            let raw = [in_l[i] as f64, in_r[i] as f64];
            self.input_meter.push(raw[0], raw[1]);
            let measured = self.input_meter.reading().short_term_lufs;
            let desired = if self.values[12] >= 0.5 && measured > -70.0 { (self.values[13] - measured).clamp(-24.0, 24.0) } else { 0.0 };
            self.target_gain_db = desired + tracking * (self.target_gain_db - desired);
            let drive = self.drive * db_to_lin(self.target_gain_db);
            let mut x = raw.map(|x| x * drive);
            if self.values[6] >= 0.5 { for c in 0..2 { let previous = self.dc[c]; let filtered = x[c] - previous.0 + dc_coeff * previous.1; self.dc[c] = (x[c], filtered); x[c] = filtered; } }
            let trigger = key.map_or([0.0; 2], |(l, r)| [l[i] as f64 * drive, r[i] as f64 * drive]);
            let mut audio_high = [[0.0; 32]; 2]; let mut key_high = [[0.0; 32]; 2]; let mut dry_high = [[0.0; 32]; 2];
            for c in 0..2 { self.quality[c].upsample(x[c], &mut audio_high[c]); self.key_quality[c].upsample(trigger[c], &mut key_high[c]); }
            for sample in 0..factor {
                let (processed, dry) = self.core.run([audio_high[0][sample], audio_high[1][sample]], [key_high[0][sample], key_high[1][sample]], ceiling, self.release, link, smooth, style);
                for c in 0..2 { audio_high[c][sample] = processed[c]; dry_high[c][sample] = dry[c]; }
            }
            let mut result = [0.0; 2];
            // Use a separate decimator history for the difference signal:
            // key interpolation and key decimation do not share histories.
            let mut dry = [0.0; 2];
            for c in 0..2 { result[c] = self.quality[c].downsample(&audio_high[c]).clamp(-ceiling, ceiling); dry[c] = self.key_quality[c].downsample(&dry_high[c]); }
            if self.last_isp {
                result = self.isp.run(result, ceiling, link);
                for c in 0..2 { self.dry_isp[c].push(dry[c]); dry[c] = self.dry_isp[c].tap(32); }
            }
            for c in 0..2 {
                if dither > 0 {
                    let noise = (self.noise(c) - self.noise(c)) * quantum;
                    let feedback = match dither { 2 => 0.85 * self.error[c][0], 3 => 1.5 * self.error[c][0] - 0.5 * self.error[c][1], _ => 0.0 };
                    let shaped = result[c] + feedback;
                    let quantized = ((shaped + noise) / quantum).round() * quantum;
                    self.error[c] = [(shaped - quantized).clamp(-2.0 * quantum, 2.0 * quantum), self.error[c][0]];
                    result[c] = quantized;
                }
                if self.values[8] >= 0.5 { result[c] = dry[c] - result[c]; }
                if self.values[7] >= 0.5 { result[c] /= drive; }
            }
            self.meter.push(result[0], result[1]);
            out_l[i] = result[0] as f32; out_r[i] = result[1] as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn run(fx: &mut impl TableEffect, x: &[f32], key: Option<&[f32]>) -> Vec<f32> {
        let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        fx.process(x, x, key.map(|k| (k, k)), &mut l, &mut r);
        l
    }

    #[test]
    fn limiter_meter_pause_and_reset_epoch_preserve_audio_history() {
        let mut p = LimiterProcessor::new(FS);
        assert_eq!(p.meter_control(), (false, 0));
        assert!(!LIMITER_SPECS[16].automatable && !LIMITER_SPECS[17].automatable);
        let tone = sine(1000.0, 0.1, 48_000);
        run(&mut p, &tone, None);
        let first = p.loudness();
        assert!(first.integrated_lufs > -30.0 && first.integrated_lufs < -10.0);
        p.set("meter_pause", 1.0).unwrap();
        run(&mut p, &sine(1000.0, 0.01, 48_000), None);
        assert_eq!(p.loudness().integrated_lufs, first.integrated_lufs);
        assert!(p.loudness().momentary_lufs < first.momentary_lufs - 15.0);
        assert_eq!(p.meter_control(), (true, 0));
        p.set("meter_reset", 1.0).unwrap();
        assert_eq!(p.meter_control(), (true, 1));
        assert_eq!(p.loudness().integrated_lufs, -120.0);
        p.set("meter_reset", 1.0).unwrap();
        p.set("meter_reset", 0.0).unwrap();
        assert_eq!(p.meter_control(), (true, 1), "only a rising edge resets");
        p.set("meter_reset", 1.0).unwrap();
        assert_eq!(p.meter_control(), (true, 2));
        p.set("meter_pause", 0.0).unwrap();
        run(&mut p, &tone, None);
        assert!(p.loudness().integrated_lufs > -30.0);
        let retained = (p.core.clock, p.core.env, p.core.window, p.target_gain_db);
        p.reset_meters();
        assert_eq!(p.meter_control(), (false, 3));
        assert_eq!(p.loudness(), LoudnessReading::default());
        assert_eq!((p.core.clock, p.core.env, p.core.window, p.target_gain_db), retained);
        p.set_sample_rate(96_000.0);
        assert_eq!(p.meter_control(), (false, 4));
    }


    #[test]
    fn compressor_signed_character_drive_preserves_index_and_processes_finite_pcm() {
        assert_eq!(COMP_SPECS[22].id,"character_drive");
        assert_eq!((COMP_SPECS[22].min,COMP_SPECS[22].max),(-24.0,24.0));
        let input=sine(997.0,0.85,4096);
        for mode in 1..=3 {
            for pre in [0.0,1.0] {
                let mut previous:Option<Vec<f32>>=None;
                for drive in [-24.0,-12.0,0.0,12.0,24.0] {
                    let mut p=CompProcessor::new(FS);
                    for (id,v) in [("ratio",1.0),("threshold",0.0),("character",mode as f64),
                        ("character_pre",pre),("character_drive",drive)] {p.set(id,v).unwrap();}
                    let output=run(&mut p,&input,None);
                    assert!(output.iter().all(|sample|sample.is_finite()&&sample.abs()<2.0));
                    assert!(rms_db(&output)>-40.0);
                    if let Some(prior)=previous {
                        let change:f64=output.iter().zip(prior).map(|(a,b)|(a-b).abs() as f64).sum();
                        assert!(change/output.len() as f64>0.001,"mode={mode} pre={pre} drive={drive}");
                    }
                    previous=Some(output);
                }
            }
        }
        let mut p=CompProcessor::new(FS);
        assert!(p.set("character_drive",-24.01).is_err());
        assert!(p.set("character_drive",24.01).is_err());
    }


    #[test]
    fn compressor_wide_knee_is_continuous_monotone_and_changes_real_pcm() {
        assert_eq!(COMP_SPECS[4].id,"knee");
        assert_eq!(COMP_SPECS[4].max,72.0);
        for knee in [24.0,48.0,72.0] {
            let mut last=compressor_curve_db(-18.0,4.0,knee,-120.0);
            for i in 1..=1320 {
                let level=-120.0+f64::from(i)*0.1;
                let value=compressor_curve_db(-18.0,4.0,knee,level);
                assert!(value.is_finite()&&value>=last-1e-10&&value-last<=0.1+1e-10);
                last=value;
            }
        }
        assert!((compressor_curve_db(-18.0,4.0,72.0,-18.0)+24.75).abs()<1e-10);
        let input=sine(997.0,0.1,12000);
        let render=|knee| {
            let mut p=CompProcessor::new(FS);
            for (id,v) in [("threshold",-18.0),("ratio",4.0),("knee",knee)] {p.set(id,v).unwrap();}
            run(&mut p,&input,None)
        };
        let tight=render(24.0);
        let wide=render(72.0);
        assert!(wide.iter().all(|value|value.is_finite()));
        assert!(rms_db(&tight[6000..])-rms_db(&wide[6000..])>1.0);
        assert!(CompProcessor::new(FS).set("knee",72.01).is_err());
    }

    #[test]
    fn compressor_curve_has_threshold_ratio_and_a_smooth_knee() {
        assert_eq!(compressor_curve_db(-20.0, 4.0, 0.0, -30.0), -30.0);
        assert!((compressor_curve_db(-20.0, 4.0, 0.0, -8.0) + 17.0).abs() < 1e-9);
        let mut previous = compressor_curve_db(-20.0, 4.0, 10.0, -40.0);
        for i in 1..=400 {
            let y = compressor_curve_db(-20.0, 4.0, 10.0, -40.0 + i as f64 * 0.1);
            assert!(y >= previous - 1e-9 && (y - previous) <= 0.1 + 1e-9);
            previous = y;
        }
        assert!((compressor_curve_db(-20.0, 4.0, 10.0, -20.0) - (-20.0 - 0.75 * 25.0 / 20.0)).abs() < 1e-9);
    }

    #[test]
    fn compressor_reduces_loud_signals_and_leaves_quiet_ones() {
        for style in 0..3 {
            let mut c = CompProcessor::new(FS);
            c.set("threshold", -20.0).unwrap();
            c.set("ratio", 4.0).unwrap();
            c.set("knee", 0.0).unwrap();
            c.set("style", style as f64).unwrap();
            let loud = sine(440.0, 1.0, 48_000);
            let out = run(&mut c, &loud, None);
            let reduction = rms_db(&loud[24_000..]) - rms_db(&out[24_000..]);
            assert!(reduction > 6.0 && reduction < 16.0, "style {style}: reduction {reduction}");
            let mut quiet_c = CompProcessor::new(FS);
            let quiet = sine(440.0, 0.01, 4_800);
            assert!((rms_db(&run(&mut quiet_c, &quiet, None)) - rms_db(&quiet)).abs() < 0.01);
        }
        let mut dry = CompProcessor::new(FS);
        dry.set("mix", 0.0).unwrap();
        let loud = sine(440.0, 1.0, 4_800);
        assert_eq!(run(&mut dry, &loud, None), loud);
    }

    #[test]
    fn range_caps_the_gain_reduction() {
        let mut c = CompProcessor::new(FS);
        for (id, v) in [("threshold", -60.0), ("ratio", 20.0), ("knee", 0.0), ("range", 6.0)] { c.set(id, v).unwrap(); }
        let loud = sine(440.0, 1.0, 24_000);
        let out = run(&mut c, &loud, None);
        let reduction = rms_db(&loud[12_000..]) - rms_db(&out[12_000..]);
        assert!((reduction - 6.0).abs() < 0.3, "{reduction}");
    }

    #[test]
    fn an_external_key_ducks_the_input() {
        let mut c = CompProcessor::new(FS);
        for (id, v) in [("threshold", -30.0), ("ratio", 10.0), ("knee", 0.0)] { c.set(id, v).unwrap(); }
        let music = sine(220.0, 0.02, 24_000);
        let kick = sine(60.0, 1.0, 24_000);
        let ducked = run(&mut c, &music, Some(&kick));
        assert!(rms_db(&music[12_000..]) - rms_db(&ducked[12_000..]) > 15.0);
        // Without a routed key the detector listens to the input.
        let mut unkeyed = CompProcessor::new(FS);
        for (id, v) in [("threshold", -30.0), ("ratio", 10.0)] { unkeyed.set(id, v).unwrap(); }
        assert!((rms_db(&run(&mut unkeyed, &music, None)[12_000..]) - rms_db(&music[12_000..])).abs() < 0.01);
    }

    #[test]
    fn lookahead_delays_audio_and_reports_latency() {
        let mut c = CompProcessor::new(FS);
        c.set("lookahead", 5.0).unwrap();
        assert_eq!(c.latency(), 240);
        let mut impulse = vec![0.0; 512];
        impulse[0] = 0.01;
        let out = run(&mut c, &impulse, None);
        assert_eq!(out.iter().position(|v| *v != 0.0), Some(240));
        assert!(CompProcessor::SPECS.iter().find(|s| s.id == "lookahead").is_some_and(|s| !s.automatable));
    }

    #[test]
    fn gate_closes_below_threshold_by_its_range_and_opens_above() {
        let mut g = GateProcessor::new(FS);
        g.set("range", 30.0).unwrap();
        let quiet = sine(440.0, 0.001, 72_000);
        let out = run(&mut g, &quiet, None);
        let reduction = rms_db(&quiet[60_000..]) - rms_db(&out[60_000..]);
        assert!((reduction - 30.0).abs() < 0.5, "{reduction}");
        let mut g = GateProcessor::new(FS);
        let loud = sine(440.0, 0.5, 24_000);
        assert!((rms_db(&run(&mut g, &loud, None)[12_000..]) - rms_db(&loud[12_000..])).abs() < 0.01);
        // A routed key opens the gate on a quiet input.
        let mut keyed = GateProcessor::new(FS);
        let quiet = &quiet[..loud.len()];
        assert!((rms_db(&run(&mut keyed, quiet, Some(&loud))[12_000..]) - rms_db(&quiet[12_000..])).abs() < 0.01);
    }

    #[test]
    fn expander_curve_respects_ratio_and_range() {
        assert_eq!(expander_curve_db(-40.0, 2.0, 80.0, -30.0), -30.0);
        assert_eq!(expander_curve_db(-40.0, 2.0, 80.0, -50.0), -60.0);
        assert_eq!(expander_curve_db(-40.0, 50.0, 20.0, -50.0), -70.0);
    }

    #[test]
    fn deesser_reduces_sibilance_but_not_the_voice() {
        let mut d = DeesserProcessor::new(FS);
        d.set("threshold", -40.0).unwrap();
        let ess = sine(10_000.0, 0.5, 24_000);
        let voice = sine(300.0, 0.5, 24_000);
        let reduction = rms_db(&ess[12_000..]) - rms_db(&run(&mut d, &ess, None)[12_000..]);
        assert!(reduction > 9.0, "{reduction}");
        let mut d = DeesserProcessor::new(FS);
        d.set("threshold", -40.0).unwrap();
        assert!((rms_db(&voice[12_000..]) - rms_db(&run(&mut d, &voice, None)[12_000..])).abs() < 0.1);
        // Split mode leaves a quiet signal's level untouched at any frequency.
        for f in [2_000.0, 6_000.0, 9_000.0] {
            let mut d = DeesserProcessor::new(FS);
            let soft = sine(f, 0.001, 9_600);
            assert!((rms_db(&run(&mut d, &soft, None)[4_800..]) - rms_db(&soft[4_800..])).abs() < 0.05);
        }
    }

    #[test]
    fn limiter_holds_the_ceiling_and_reports_its_lookahead() {
        let mut lim = LimiterProcessor::new(FS);
        lim.set("gain", 18.0).unwrap();
        lim.set("ceiling", -1.0).unwrap();
        assert_eq!(lim.latency(), 144);
        let mut x = sine(100.0, 0.5, 48_000);
        x[10_000] = 1.0;
        let out = run(&mut lim, &x, None);
        let ceiling = db_to_lin(-1.0) as f32;
        assert!(peak(&out) <= ceiling + 1e-6, "peak {}", peak(&out));
        // It shapes gain rather than clipping: only the crests reach the
        // ceiling (a clipper would flatten most of every half cycle).
        let at_ceiling = out.iter().filter(|v| v.abs() > ceiling * 0.999).count();
        assert!(at_ceiling < 1_500, "{at_ceiling} samples at the ceiling");
        assert!(rms_db(&out) > rms_db(&x) + 3.0);
        // Below the ceiling it is a pure delay.
        let mut lim = LimiterProcessor::new(FS);
        let soft = sine(100.0, 0.1, 2_000);
        let out = run(&mut lim, &soft, None);
        assert!(out[144..].iter().zip(&soft).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn compressor_linking_mid_side_and_upward_modes_change_real_audio() {
        let loud = sine(440.0, 0.8, 24000); let quiet = sine(440.0, 0.005, 24000);
        let (mut l, mut r) = (vec![0.0; 24000], vec![0.0; 24000]);
        let mut c = CompProcessor::new(FS); c.set("stereo_link", 0.0).unwrap();
        c.process(&loud, &quiet, None, &mut l, &mut r);
        assert!((rms_db(&r[12000..]) - rms_db(&quiet[12000..])).abs() < 0.05);
        c.set("stereo_link", 100.0).unwrap(); c.process(&loud, &quiet, None, &mut l, &mut r);
        assert!(rms_db(&quiet[12000..]) - rms_db(&r[12000..]) > 6.0);
        let mut side = CompProcessor::new(FS); side.set("channel_mode", 2.0).unwrap();
        side.process(&loud, &loud, None, &mut l, &mut r);
        assert!(l.iter().zip(&loud).all(|(a, b)| (a - b).abs() < 1e-6));
        let mut upward = CompProcessor::new(FS); upward.set("style", 6.0).unwrap();
        let out = run(&mut upward, &quiet, None);
        assert!(rms_db(&out[12000..]) - rms_db(&quiet[12000..]) > 8.0);
    }

    #[test]
    fn gate_ducking_uses_the_key_and_upward_expansion_is_positive() {
        let music = sine(220.0, 0.01, 24000); let key = sine(60.0, 0.8, 24000);
        let mut duck = GateProcessor::new(FS);
        for (id, value) in [("style", 5.0), ("threshold", -30.0), ("range", 20.0)] { duck.set(id, value).unwrap(); }
        let out = run(&mut duck, &music, Some(&key));
        assert!(rms_db(&music[12000..]) - rms_db(&out[12000..]) > 15.0);
        assert!(gate_style_curve_db(4, -40.0, 2.0, 12.0, 0.0, -30.0) > -30.0);
        assert!(gate_style_curve_db(5, -40.0, 2.0, 12.0, 0.0, -30.0) < -30.0);
    }

    #[test]
    fn deesser_lookahead_and_wet_bypass_preserve_the_dry_signal() {
        let mut d = DeesserProcessor::new(FS);
        d.set("lookahead", 5.0).unwrap(); d.set("mix", 0.0).unwrap();
        let input = sine(8000.0, 0.5, 2000); let output = run(&mut d, &input, None);
        assert_eq!(d.latency(), 240);
        assert!(output[..240].iter().all(|v| *v == 0.0));
        assert_eq!(&output[240..], &input[..input.len() - 240]);
    }

    #[test]
    fn limiter_delta_reconstructs_driven_input_and_external_key_reduces_it() {
        let input = sine(220.0, 0.4, 12000);
        let configure = || { let mut p = LimiterProcessor::new(FS); p.set("gain", 12.0).unwrap(); p.set("ceiling", -3.0).unwrap(); p };
        let mut limited = configure(); let y = run(&mut limited, &input, None);
        let mut delta = configure(); delta.set("audition", 1.0).unwrap(); let removed = run(&mut delta, &input, None);
        let latency = limited.latency() as usize;
        for i in latency..input.len() { assert!((y[i] as f64 + removed[i] as f64 - input[i - latency] as f64 * db_to_lin(12.0)).abs() < 2e-7); }
        let quiet = sine(220.0, 0.01, 12000); let key = sine(60.0, 2.0, 12000);
        let mut p = LimiterProcessor::new(FS); let output = run(&mut p, &quiet, Some(&key));
        assert!(rms_db(&quiet[6000..]) - rms_db(&output[6000..]) > 5.0);
    }

    #[test]
    fn monotone_limiter_queue_matches_a_window_scan() {
        let mut queue = SlidingMinimum::new(32); let mut values = Vec::new();
        for i in 0usize..1000 { let value = ((i * 37 + i * i * 11) % 997) as f64 / 997.0; values.push(value);
            let expected = values[i.saturating_sub(17)..].iter().copied().fold(1.0f64, f64::min);
            assert_eq!(queue.push(i as u64, value, 17), expected);
        }
    }
    #[test]
    fn advanced_compressor_eq_auto_threshold_and_note_trigger_are_functional() {
        let x = sine(1000.0, 0.5, 24000);
        let mut filtered = CompProcessor::new(FS);
        for (id, value) in [("sc_eq_on", 1.0), ("sc3_freq", 1000.0), ("sc3_gain", -24.0), ("sc3_q", 0.5)] { filtered.set(id, value).unwrap(); }
        let plain = run(&mut CompProcessor::new(FS), &x, None); let eq = run(&mut filtered, &x, None);
        assert!(rms_db(&eq[12000..]) > rms_db(&plain[12000..]) + 5.0);
        let mut automatic = CompProcessor::new(FS);
        automatic.set("auto_threshold", 1.0).unwrap(); automatic.set("auto_track", 100.0).unwrap();
        run(&mut automatic, &x, None);
        let loud = automatic.effective_threshold_db();
        let quiet = sine(1000.0, 0.05, 48000); run(&mut automatic, &quiet, None);
        assert!(automatic.effective_threshold_db() < loud - 15.0);
        let mut midi = CompProcessor::new(FS); midi.set("trigger_mode", 1.0).unwrap(); midi.set("attack", 0.1).unwrap();
        let quiet = sine(220.0, 0.01, 2400);
        let untouched = run(&mut midi, &quiet, None);
        assert!((rms_db(&untouched) - rms_db(&quiet)).abs() < 0.01);
        midi.trigger_note(1.0); let ducked = run(&mut midi, &quiet, None);
        assert!(rms_db(&ducked[240..]) < rms_db(&quiet[240..]) - 6.0);
        let mut fingerprints = Vec::new();
        for style in 0..COMP_STYLES.len() {
            let mut p = CompProcessor::new(FS); p.set("style", style as f64).unwrap();
            let y = run(&mut p, &x[..6000], None); assert!(y.iter().all(|x| x.is_finite()));
            fingerprints.push(rms_db(&y[3000..]));
        }
        assert!(fingerprints.windows(2).filter(|v| (v[0]-v[1]).abs() > 0.001).count() >= 10);
    }

    #[test]
    fn linear_deesser_dry_path_and_adaptive_detection_are_partition_invariant() {
        let x = sine(9500.0, 0.7, 12000);
        let make = || { let mut p = DeesserProcessor::new(FS); p.set("phase", 1.0).unwrap(); p.set("detection", 2.0).unwrap(); p };
        let mut whole = make(); let y = run(&mut whole, &x, None); let mut chunks = make();
        let z: Vec<_> = x.chunks(173).flat_map(|block| run(&mut chunks, block, None)).collect();
        assert_eq!(y, z); assert_eq!(whole.latency(), 64);
        let mut dry = make(); dry.set("mix", 0.0).unwrap(); let y = run(&mut dry, &x, None);
        assert_eq!(&y[64..], &x[..x.len()-64]);
    }

    #[test]
    fn true_peak_limiter_protects_reconstruction_at_all_quality_settings() {
        for quality in [0, 1, 3, 5] {
            let mut p = LimiterProcessor::new(FS);
            for (id, value) in [("true_peak", 1.0), ("oversampling", quality as f64), ("gain", 12.0), ("ceiling", -1.0), ("lookahead", 0.1)] { p.set(id, value).unwrap(); }
            let x: Vec<_> = (0..3000).map(|n| (0.8 * (std::f64::consts::TAU * 12000.0 * n as f64 / FS + 0.7).sin()) as f32).collect();
            let y = run(&mut p, &x, None); let mut meter = TruePeak::new();
            let peak = y.iter().map(|v| meter.run(*v as f64)).fold(0.0, f64::max);
            assert!(peak <= db_to_lin(-1.0) + 1e-6, "quality {quality}, peak {peak}");
            assert_eq!(p.latency(), 5 + 64);
        }
        for style in 0..LIMITER_STYLES.len() {
            let mut p = LimiterProcessor::new(FS); p.set("style", style as f64).unwrap(); p.set("gain", 18.0).unwrap();
            let y = run(&mut p, &sine(500.0, 0.7, 3000), None);
            assert!(y.iter().all(|v| v.is_finite() && (*v as f64).abs() <= db_to_lin(-0.3) + 1e-6));
        }
    }

    #[test]
    fn dither_is_deterministic_quantized_and_not_a_dc_offset() {
        for mode in 1..=3 {
            let mut p = LimiterProcessor::new(FS); p.set("dither", mode as f64).unwrap(); p.set("dither_bits", 16.0).unwrap();
            let y = run(&mut p, &[0.0; 12000], None);
            assert!(y.iter().any(|v| *v != 0.0)); let q = 2f64.powi(-15);
            assert!(y.iter().all(|v| ((*v as f64 / q).round() - *v as f64 / q).abs() < 1e-10));
            assert!(y.iter().map(|v| *v as f64).sum::<f64>().abs() / (y.len() as f64) < q * 0.1);
        }
    }


    #[test]
    fn gate_expert_detector_routing_rms_and_program_release_change_measured_audio() {
        let input=sine(220.0,0.1,24000);let key=sine(1000.0,0.04,24000);
        let negative:Vec<f32>=key.iter().map(|v|-v).collect();
        let configure=||{let mut gate=GateProcessor::new(FS);for(id,v)in[("threshold",-29.0),("hold",0.0),("release",5.0)]{gate.set(id,v).unwrap();}gate};
        let mut peak=configure();let a=run(&mut peak,&input,Some(&key));
        let mut rms=configure();rms.set("detection",1.0).unwrap();let b=run(&mut rms,&input,Some(&key));
        assert!(rms_db(&a[12000..])-rms_db(&b[12000..])>30.0);
        let mut mid=configure();mid.set("sc_channel",1.0).unwrap();mid.set("sc_listen",1.0).unwrap();
        let(mut l,mut r)=(vec![0.0;24000],vec![0.0;24000]);
        mid.process(&input,&input,Some((&key,&negative)),&mut l,&mut r);
        assert!(l.iter().chain(&r).all(|v|*v==0.0));
        let mut side=configure();side.set("sc_channel",2.0).unwrap();side.set("sc_listen",1.0).unwrap();
        side.process(&input,&input,Some((&key,&negative)),&mut l,&mut r);
        assert_eq!(l,key);assert_eq!(r,key);
        let mut falling=vec![0.8;24000];falling[12000..].fill(0.0);
        let mut fixed=configure();fixed.set("release",40.0).unwrap();
        let normal=run(&mut fixed,&input,Some(&falling));
        let mut adaptive=configure();adaptive.set("release",40.0).unwrap();adaptive.set("program_release",100.0).unwrap();
        let slower=run(&mut adaptive,&input,Some(&falling));
        assert!(rms_db(&slower[18000..20000])-rms_db(&normal[18000..20000])>3.0);
    }

}
