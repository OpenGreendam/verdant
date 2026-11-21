//! Dynamics: compressor, gate/expander, de-esser and limiter. Every
//! detector can listen to the routed sidechain key instead of the input.

use super::filters::{db_to_lin, lin_to_db, rbj, time_coefficient, Biquad, DelayLine, Rbj, IDENTITY};
use super::{Spec, TableEffect};

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
pub const COMP_STYLES: [&str; 8] = ["干净", "平滑", "冲击", "人声", "总线", "光学", "向上", "泵动"];
pub const DYNAMICS_CHANNEL_MODES: [&str; 3] = ["立体声", "中声道", "侧声道"];
pub const DETECTOR_MODES: [&str; 2] = ["峰值", "RMS"];
pub const COMP_CHARACTERS: [&str; 4] = ["关闭", "电子管", "二极管", "明亮"];

pub static COMP_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -60.0, 0.0, -18.0, "dB"),
    Spec::new("ratio", "压缩比", 1.0, 20.0, 3.0, ":1").log(),
    Spec::new("attack", "启动", 0.1, 200.0, 10.0, "ms").log(),
    Spec::new("release", "释放", 5.0, 2000.0, 120.0, "ms").log(),
    Spec::new("knee", "拐点", 0.0, 24.0, 6.0, "dB"),
    Spec::new("makeup", "补偿增益", 0.0, 24.0, 0.0, "dB"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("style", "风格", 0.0, 7.0, 0.0, "").steps(7),
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
    Spec::new("character_drive", "染色驱动", 0.0, 24.0, 0.0, "dB"),
    Spec::new("character_pre", "压缩前染色", 0.0, 1.0, 0.0, "").steps(1),
];

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
}
impl CompProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(COMP_SPECS), sample_rate, attack: 0.0, release: 0.0,
            rms: [0.0; 2], rms_coeff: 0.0, gr: [0.0; 2], makeup: 1.0, wet: 1.0,
            last_out: [0.0; 2], key: KeyFilter::default(), lookahead: Lookahead::new(sample_rate), held: [0; 2], hold_samples: 0 };
        p.refresh(); p
    }
    pub fn gain_reduction_db(&self) -> f64 { if self.values[7].round() as usize == 6 { self.gr[0].max(self.gr[1]) } else { self.gr[0].min(self.gr[1]) } }
    pub fn curve_db(&self, input_db: f64) -> f64 {
        let v = &self.values;
        compressor_style_curve_db(v[7].round() as usize, v[0], v[1], v[4], v[9], input_db)
    }
}
impl TableEffect for CompProcessor {
    const SPECS: &'static [Spec] = COMP_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values;
        let style = v[7].round() as usize;
        let attack = v[2] * if style == 5 { 2.0 } else { 1.0 };
        let release = v[3] * if style == 5 { 2.5 } else if style == 7 { 0.5 } else { 1.0 };
        self.attack = time_coefficient(attack, self.sample_rate);
        self.release = time_coefficient(release, self.sample_rate);
        self.rms_coeff = time_coefficient(10.0, self.sample_rate);
        let compensation = if v[15] >= 0.5 { (-self.curve_db(-12.0) - 12.0).clamp(-24.0, 24.0) } else { 0.0 };
        self.makeup = db_to_lin(v[5] + compensation);
        self.wet = v[6] / 100.0;
        self.key.set(v[10], v[12], self.sample_rate);
        self.lookahead.set(v[8], self.sample_rate);
        self.hold_samples = (v[13] * 0.001 * self.sample_rate) as usize;
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn latency(&self) -> u32 { self.lookahead.samples as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let style = self.values[7].round() as usize;
        let mode = self.values[17].round() as usize;
        let rms = self.values[18] >= 0.5 || matches!(style, 1 | 5);
        let listen = self.values[11] >= 0.5;
        let input = db_to_lin(self.values[19]); let output = db_to_lin(self.values[20]);
        let character = self.values[21].round() as usize;
        let drive = db_to_lin(self.values[22]); let pre = self.values[23] >= 0.5;
        let link = if mode == 0 { self.values[16] / 100.0 } else { 0.0 };
        let auto_release = self.values[14] >= 0.5;
        for i in 0..in_l.len() {
            let dry = [in_l[i] as f64 * input, in_r[i] as f64 * input];
            let signal = dry.map(|x| if pre { compressor_character(character, x, drive) } else { x });
            let (kl, kr) = key.map_or((signal[0], signal[1]), |(a, b)| (a[i] as f64, b[i] as f64));
            let (kl, kr) = self.key.run(kl, kr);
            let detector = stereo_domain(kl, kr, mode);
            let mut target = [0.0; 2];
            for c in 0..2 {
                let mut level = if style == 2 && key.is_none() { self.last_out[c] } else { detector[c].abs() };
                if rms { self.rms[c] = level * level + self.rms_coeff * (self.rms[c] - level * level); level = self.rms[c].max(0.0).sqrt(); }
                let db = lin_to_db(level);
                target[c] = self.curve_db(db) - db;
                if style == 6 && level < 1e-8 { target[c] = 0.0; }
            }
            let linked = if style == 6 { target[0].max(target[1]) } else { target[0].min(target[1]) };
            for c in 0..2 {
                let target = if selected_channel(c, mode) { target[c] + (linked - target[c]) * link } else { 0.0 };
                let engaging = if style == 6 { target > self.gr[c] } else { target < self.gr[c] };
                if engaging {
                    self.held[c] = self.hold_samples;
                    self.gr[c] = target + self.attack * (self.gr[c] - target);
                } else if self.held[c] > 0 { self.held[c] -= 1; }
                else {
                    let release = if auto_release { self.release.powf((1.0 + self.gr[c].abs() / 12.0).recip()) } else { self.release };
                    self.gr[c] = target + release * (self.gr[c] - target);
                }
            }
            let (dl, dr) = self.lookahead.run(dry[0], dry[1]);
            if listen { out_l[i] = kl as f32; out_r[i] = kr as f32; continue; }
            let mut audio = stereo_domain(dl, dr, mode);
            for c in 0..2 {
                let gain = if selected_channel(c, mode) { db_to_lin(self.gr[c]) * self.makeup } else { 1.0 };
                let processed = if !selected_channel(c, mode) { audio[c] } else if pre { compressor_character(character, audio[c], drive) * gain } else { compressor_character(character, audio[c] * gain, drive) };
                self.last_out[c] = (audio[c] * db_to_lin(self.gr[c])).abs();
                audio[c] = (audio[c] * (1.0 - self.wet) + processed * self.wet) * output;
            }
            let y = from_stereo_domain(audio, mode);
            out_l[i] = y[0] as f32; out_r[i] = y[1] as f32;
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
    key: KeyFilter, lookahead: Lookahead,
}
impl GateProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(GATE_SPECS), sample_rate, attack: 0.0, release: 0.0,
            hold_samples: 0, held: [0; 2], open: [false; 2], gr: [0.0; 2], envelope: [0.0; 2], envelope_attack: 0.0, envelope_release: 0.0,
            key: KeyFilter::default(), lookahead: Lookahead::new(sample_rate) };
        p.refresh(); p
    }
    pub fn gain_reduction_db(&self) -> f64 { if self.values[11].round() as usize == 4 { self.gr[0].max(self.gr[1]) } else { self.gr[0].min(self.gr[1]) } }
    pub fn curve_db(&self, input_db: f64) -> f64 {
        gate_style_curve_db(self.values[11].round() as usize, self.values[0], self.values[2], self.values[1], self.values[12], input_db)
    }
}
impl TableEffect for GateProcessor {
    const SPECS: &'static [Spec] = GATE_SPECS;
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
            let detector = stereo_domain(kl, kr, mode).map(f64::abs);
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
                let coeff = if opening { self.attack } else { self.release };
                self.gr[c] = target + coeff * (self.gr[c] - target);
            }
            let (dl, dr) = self.lookahead.run(l, r);
            if listen { out_l[i] = kl as f32; out_r[i] = kr as f32; continue; }
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

pub const DEESSER_DETECTION: [&str; 2] = ["全能", "单人声"];
pub static DEESSER_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -60.0, 0.0, -24.0, "dB"),
    Spec::new("range", "最大衰减", 0.0, 24.0, 12.0, "dB"),
    Spec::new("freq", "频率", 2000.0, 16000.0, 6000.0, "Hz").log(),
    Spec::new("mode", "模式", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("release", "释放", 10.0, 500.0, 60.0, "ms").log(),
    Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("detection", "检测算法", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("sc_lpf", "检测上限", 4000.0, 20000.0, 20000.0, "Hz").log(),
    Spec::new("lookahead", "前视", 0.0, 15.0, 0.0, "ms").fixed(),
    Spec::new("stereo_link", "立体声联动", 0.0, 100.0, 100.0, "%"),
    Spec::new("channel_mode", "处理声道", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("attack", "启动", 0.05, 20.0, 0.5, "ms").log(),
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
    energy: [[f64; 2]; 2], energy_coeff: f64,
}
impl DeesserProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(DEESSER_SPECS), sample_rate, attack: 0.0, release: 0.0, gr: [0.0; 2],
            split: HighBand::default(), detect: HighBand::default(), upper: KeyFilter::default(), lookahead: Lookahead::new(sample_rate),
            energy: [[0.0; 2]; 2], energy_coeff: 0.0 };
        p.refresh(); p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.gr[0].min(self.gr[1]) }
    pub fn reduction_db(&self, level_db: f64) -> f64 { -((level_db - self.values[0]) * 0.75).clamp(0.0, self.values[1]) }
}
impl TableEffect for DeesserProcessor {
    const SPECS: &'static [Spec] = DEESSER_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        self.attack = time_coefficient(self.values[12], self.sample_rate);
        self.release = time_coefficient(self.values[4], self.sample_rate);
        self.energy_coeff = time_coefficient(5.0, self.sample_rate);
        self.split.set(self.values[2], self.sample_rate); self.detect.set(self.values[2], self.sample_rate);
        self.upper.set(20.0, self.values[7].max(self.values[2]), self.sample_rate);
        self.lookahead.set(self.values[8], self.sample_rate);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); *self = Self::new(sample_rate); self.values = values; self.refresh(); }
    fn latency(&self) -> u32 { self.lookahead.samples as u32 }
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
                let confidence = if vocal { ((self.energy[c][0] / self.energy[c][1].max(1e-15) - 0.08) / 0.32).clamp(0.0, 1.0) } else { 1.0 };
                target[c] = self.reduction_db(lin_to_db(high[c].abs())) * confidence;
            }
            let linked = target[0].min(target[1]);
            for c in 0..2 {
                let target = if selected_channel(c, mode) { target[c] + (linked - target[c]) * link } else { 0.0 };
                let coeff = if target < self.gr[c] { self.attack } else { self.release };
                self.gr[c] = target + coeff * (self.gr[c] - target);
            }
            let (dl, dr) = self.lookahead.run(l, r);
            if listen { let y = from_stereo_domain(high, mode); out_l[i] = y[0] as f32; out_r[i] = y[1] as f32; continue; }
            let mut audio = stereo_domain(dl, dr, mode);
            for c in 0..2 {
                let gain = if selected_channel(c, mode) { 1.0 + (db_to_lin(self.gr[c]) - 1.0) * wet } else { 1.0 };
                if wide { audio[c] *= gain; }
                else {
                    let (low, high) = self.split.run(c, audio[c]);
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

pub const LIMITER_STYLES: [&str; 4] = ["透明", "平滑", "冲击", "安全"];
pub static LIMITER_SPECS: &[Spec] = &[
    Spec::new("gain", "输入增益", 0.0, 24.0, 0.0, "dB"),
    Spec::new("ceiling", "输出上限", -12.0, 0.0, -0.3, "dB"),
    Spec::new("release", "释放", 1.0, 1000.0, 50.0, "ms").log(),
    Spec::new("lookahead", "前视", 0.1, 10.0, 3.0, "ms").fixed(),
    Spec::new("link", "立体声联动", 0.0, 100.0, 100.0, "%"),
    Spec::new("style", "风格", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("dc_filter", "去直流", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("unity_gain", "等响比较", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("audition", "监听限制差分", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("attack", "平滑启动", 0.0, 10.0, 3.0, "ms"),
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

/// Brickwall lookahead limiter. Per channel: the gain each sample needs is
/// held by a sliding minimum over the lookahead window, released
/// exponentially, then box-averaged over the window so gain changes begin
/// before the peak arrives. The averaged gain never exceeds what any sample
/// in its window needs, so the delayed output stays under the ceiling.
pub struct LimiterProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    window: usize,
    audio: [DelayLine; 2],
    minimum: [SlidingMinimum; 2],
    env: [f64; 2],
    held: [DelayLine; 2],
    sum: [f64; 2],
    release: f64,
    drive: f64,
    ceiling: f64,
    gr_db: f64,
    clock: u64,
    dc: [(f64, f64); 2],
}

impl LimiterProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let capacity = lookahead_samples(10.0, sample_rate.max(8_000.0)) + 2;
        let line = || [DelayLine::new(capacity), DelayLine::new(capacity)];
        let mut p = Self { values: super::table_defaults(LIMITER_SPECS), sample_rate, window: 1, audio: line(),
            minimum: [SlidingMinimum::new(capacity), SlidingMinimum::new(capacity)],
            env: [1.0; 2], held: line(), sum: [0.0; 2], release: 0.0, drive: 1.0, ceiling: 1.0, gr_db: 0.0,
            clock: 0, dc: [(0.0, 0.0); 2] };
        p.refresh();
        p.reset_window();
        p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.gr_db }
    fn reset_window(&mut self) {
        for c in 0..2 {
            self.audio[c].clear();
            self.minimum[c].clear();
            for _ in 0..self.held[c].capacity() { self.held[c].push(1.0); }
            self.env[c] = 1.0;
            self.sum[c] = self.window as f64;
        }
        self.clock = 0;
        self.dc = [(0.0, 0.0); 2];
    }
}

impl TableEffect for LimiterProcessor {
    const SPECS: &'static [Spec] = LIMITER_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values;
        self.drive = db_to_lin(v[0]);
        self.ceiling = db_to_lin(v[1]);
        let release_scale = match v[5].round() as usize { 1 => 2.0, 2 => 0.5, 3 => 3.0, _ => 1.0 };
        self.release = time_coefficient(v[2] * release_scale, self.sample_rate);
        let window = lookahead_samples(v[3], self.sample_rate).clamp(1, self.audio[0].capacity() - 2);
        if window != self.window { self.window = window; self.reset_window(); }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { *self = Self { values: self.values.clone(), ..Self::new(sample_rate) }; self.refresh(); self.reset_window(); }
    fn latency(&self) -> u32 { self.window as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let link = self.values[4] / 100.0;
        let w = self.window;
        let dc_filter = self.values[6] >= 0.5;
        let unity = if self.values[7] >= 0.5 { self.drive.recip() } else { 1.0 };
        let audition = self.values[8] >= 0.5;
        let smooth = (self.values[9] * 0.001 * self.sample_rate / w as f64).clamp(0.0, 1.0);
        let dc_coeff = (-std::f64::consts::TAU * 5.0 / self.sample_rate).exp();
        for i in 0..in_l.len() {
            let mut x = [in_l[i] as f64 * self.drive, in_r[i] as f64 * self.drive];
            if dc_filter { for c in 0..2 { let previous = self.dc[c]; let filtered = x[c] - previous.0 + dc_coeff * previous.1; self.dc[c] = (x[c], filtered); x[c] = filtered; } }
            let trigger = key.map_or([0.0; 2], |(a, b)| [a[i] as f64 * self.drive, b[i] as f64 * self.drive]);
            let own: [f64; 2] = std::array::from_fn(|c| (self.ceiling / x[c].abs().max(trigger[c].abs()).max(1e-12)).min(1.0));
            let linked = own[0].min(own[1]);
            let mut y = [0.0; 2];
            for c in 0..2 {
                let need = own[c] + (linked - own[c]) * link;
                let hold = self.minimum[c].push(self.clock, need, w);
                self.env[c] = if hold < self.env[c] { hold } else { hold + self.release * (self.env[c] - hold) };
                self.held[c].push(self.env[c]);
                self.sum[c] += self.env[c] - self.held[c].tap(w);
                let average = (self.sum[c] / w as f64).clamp(0.0, 1.0);
                let gain = self.env[c] + (average - self.env[c]) * smooth;
                self.audio[c].push(x[c]);
                let dry = self.audio[c].tap(w);
                let limited = (dry * gain).clamp(-self.ceiling, self.ceiling);
                y[c] = (if audition { dry - limited } else { limited }) * unity;
                if c == 0 { self.gr_db = lin_to_db(gain); } else { self.gr_db = self.gr_db.min(lin_to_db(gain)); }
            }
            out_l[i] = y[0] as f32;
            out_r[i] = y[1] as f32;
            self.clock = self.clock.wrapping_add(1);
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
}
