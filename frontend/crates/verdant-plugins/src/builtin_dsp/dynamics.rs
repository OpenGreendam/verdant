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
    fn reset(&mut self) { for f in self.hp.iter_mut().chain(&mut self.lp) { f.reset(); } }
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

pub const COMP_STYLES: [&str; 3] = ["干净", "平滑", "冲击"];

pub static COMP_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -60.0, 0.0, -18.0, "dB"),
    Spec::new("ratio", "压缩比", 1.0, 20.0, 3.0, ":1").log(),
    Spec::new("attack", "启动", 0.1, 200.0, 10.0, "ms").log(),
    Spec::new("release", "释放", 5.0, 2000.0, 120.0, "ms").log(),
    Spec::new("knee", "拐点", 0.0, 24.0, 6.0, "dB"),
    Spec::new("makeup", "补偿增益", 0.0, 24.0, 0.0, "dB"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
    Spec::new("style", "风格", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("lookahead", "前视", 0.0, MAX_LOOKAHEAD_MS, 0.0, "ms").fixed(),
    Spec::new("range", "最大压缩", 0.0, 60.0, 60.0, "dB"),
    Spec::new("sc_hpf", "侧链高通", 20.0, 1000.0, 20.0, "Hz").log(),
    Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
];

/// Stereo-linked compressor: the louder channel drives one gain reduction.
pub struct CompProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    attack: f64,
    release: f64,
    rms: f64,
    rms_coeff: f64,
    gr_db: f64,
    makeup: f64,
    wet: f64,
    last_out: f64,
    key: KeyFilter,
    lookahead: Lookahead,
}

impl CompProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(COMP_SPECS), sample_rate, attack: 0.0, release: 0.0, rms: 0.0, rms_coeff: 0.0,
            gr_db: 0.0, makeup: 1.0, wet: 1.0, last_out: 0.0, key: KeyFilter::default(), lookahead: Lookahead::new(sample_rate) };
        p.refresh();
        p
    }
    /// Current gain reduction in dB (≤ 0), for meters.
    pub fn gain_reduction_db(&self) -> f64 { self.gr_db }
    pub fn curve_db(&self, input_db: f64) -> f64 {
        let v = &self.values;
        compressor_curve_db(v[0], v[1], v[4], input_db).max(input_db - v[9])
    }
}

impl TableEffect for CompProcessor {
    const SPECS: &'static [Spec] = COMP_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values;
        self.attack = time_coefficient(v[2], self.sample_rate);
        self.release = time_coefficient(v[3], self.sample_rate);
        self.rms_coeff = time_coefficient(10.0, self.sample_rate);
        self.makeup = db_to_lin(v[5]);
        self.wet = v[6] / 100.0;
        self.key.set(v[10], 20_000.0, self.sample_rate);
        self.lookahead.set(v[8], self.sample_rate);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.gr_db = 0.0;
        self.rms = 0.0;
        self.key.reset();
        self.lookahead = Lookahead::new(sample_rate);
        self.refresh();
    }
    fn latency(&self) -> u32 { self.lookahead.samples as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let style = self.values[7].round() as u8;
        let external = key.is_some();
        let listen = self.values[11] >= 0.5;
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let (kl, kr) = match key { Some((a, b)) => (a[i] as f64, b[i] as f64), _ => (l, r) };
            let (kl, kr) = self.key.run(kl, kr);
            // Feedback style listens to its own output unless keyed externally.
            let level = if style == 2 && !external { self.last_out } else { kl.abs().max(kr.abs()) };
            let level = if style == 1 {
                self.rms = level * level + self.rms_coeff * (self.rms - level * level);
                self.rms.sqrt()
            } else { level };
            let level_db = lin_to_db(level);
            let target = self.curve_db(level_db) - level_db;
            let coeff = if target < self.gr_db { self.attack } else { self.release };
            self.gr_db = target + coeff * (self.gr_db - target);
            let (dl, dr) = self.lookahead.run(l, r);
            if listen {
                out_l[i] = kl as f32;
                out_r[i] = kr as f32;
                continue;
            }
            let g = db_to_lin(self.gr_db) * self.makeup;
            let (yl, yr) = (dl * (1.0 - self.wet) + dl * g * self.wet, dr * (1.0 - self.wet) + dr * g * self.wet);
            self.last_out = yl.abs().max(yr.abs()) / self.makeup.max(1e-9);
            out_l[i] = yl as f32;
            out_r[i] = yr as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// Gate / expander
// ---------------------------------------------------------------------------

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
];

pub struct GateProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    attack: f64,
    release: f64,
    hold_samples: usize,
    held: usize,
    gr_db: f64,
    key: KeyFilter,
    lookahead: Lookahead,
}

impl GateProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(GATE_SPECS), sample_rate, attack: 0.0, release: 0.0, hold_samples: 0, held: 0,
            gr_db: 0.0, key: KeyFilter::default(), lookahead: Lookahead::new(sample_rate) };
        p.refresh();
        p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.gr_db }
    pub fn curve_db(&self, input_db: f64) -> f64 { expander_curve_db(self.values[0], self.values[2], self.values[1], input_db) }
}

impl TableEffect for GateProcessor {
    const SPECS: &'static [Spec] = GATE_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let v = &self.values;
        self.attack = time_coefficient(v[3], self.sample_rate);
        self.release = time_coefficient(v[5], self.sample_rate);
        self.hold_samples = (v[4] * 0.001 * self.sample_rate) as usize;
        self.key.set(v[7], v[8], self.sample_rate);
        self.lookahead.set(v[6], self.sample_rate);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.gr_db = 0.0;
        self.key.reset();
        self.lookahead = Lookahead::new(sample_rate);
        self.refresh();
    }
    fn latency(&self) -> u32 { self.lookahead.samples as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let listen = self.values[9] >= 0.5;
        let wet = self.values[10] / 100.0;
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let (kl, kr) = match key { Some((a, b)) => (a[i] as f64, b[i] as f64), _ => (l, r) };
            let (kl, kr) = self.key.run(kl, kr);
            let level_db = lin_to_db(kl.abs().max(kr.abs()));
            let target = self.curve_db(level_db) - level_db;
            if target >= self.gr_db {
                // Opening: follow the attack and restart the hold time.
                self.held = self.hold_samples;
                self.gr_db = target + self.attack * (self.gr_db - target);
            } else if self.held > 0 {
                self.held -= 1;
            } else {
                self.gr_db = target + self.release * (self.gr_db - target);
            }
            let (dl, dr) = self.lookahead.run(l, r);
            if listen { out_l[i] = kl as f32; out_r[i] = kr as f32; continue; }
            let g = db_to_lin(self.gr_db) * wet + (1.0 - wet);
            out_l[i] = (dl * g) as f32;
            out_r[i] = (dr * g) as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// De-esser
// ---------------------------------------------------------------------------

pub const DEESSER_MODES: [&str; 2] = ["分频", "全频"];

pub static DEESSER_SPECS: &[Spec] = &[
    Spec::new("threshold", "阈值", -60.0, 0.0, -24.0, "dB"),
    Spec::new("range", "最大衰减", 0.0, 24.0, 12.0, "dB"),
    Spec::new("freq", "频率", 2000.0, 16000.0, 6000.0, "Hz").log(),
    Spec::new("mode", "模式", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("release", "释放", 10.0, 500.0, 60.0, "ms").log(),
    Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
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
    pub values: Vec<f64>,
    sample_rate: f64,
    attack: f64,
    release: f64,
    gr_db: f64,
    split: HighBand,
    detect: HighBand,
}

impl DeesserProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { values: super::table_defaults(DEESSER_SPECS), sample_rate, attack: 0.0, release: 0.0, gr_db: 0.0,
            split: HighBand::default(), detect: HighBand::default() };
        p.refresh();
        p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.gr_db }
    /// Reduction in dB (≤ 0) for a detected sibilance level.
    pub fn reduction_db(&self, level_db: f64) -> f64 { -((level_db - self.values[0]) * 0.75).clamp(0.0, self.values[1]) }
}

impl TableEffect for DeesserProcessor {
    const SPECS: &'static [Spec] = DEESSER_SPECS;
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        self.attack = time_coefficient(0.5, self.sample_rate);
        self.release = time_coefficient(self.values[4], self.sample_rate);
        self.split.set(self.values[2], self.sample_rate);
        self.detect.set(self.values[2], self.sample_rate);
    }
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.gr_db = 0.0;
        self.split = HighBand::default();
        self.detect = HighBand::default();
        self.refresh();
    }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let listen = self.values[5] >= 0.5;
        let wide = self.values[3] >= 0.5;
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let (kl, kr) = match key { Some((a, b)) => (a[i] as f64, b[i] as f64), _ => (l, r) };
            let (hl, hr) = (self.detect.run(0, kl).1, self.detect.run(1, kr).1);
            let target = self.reduction_db(lin_to_db(hl.abs().max(hr.abs())));
            let coeff = if target < self.gr_db { self.attack } else { self.release };
            self.gr_db = target + coeff * (self.gr_db - target);
            let g = db_to_lin(self.gr_db);
            if listen { out_l[i] = hl as f32; out_r[i] = hr as f32; continue; }
            if wide {
                out_l[i] = (l * g) as f32;
                out_r[i] = (r * g) as f32;
            } else {
                let ((ll, lh), (rl, rh)) = (self.split.run(0, l), self.split.run(1, r));
                out_l[i] = (ll + lh * g) as f32;
                out_r[i] = (rl + rh * g) as f32;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Limiter
// ---------------------------------------------------------------------------

pub static LIMITER_SPECS: &[Spec] = &[
    Spec::new("gain", "输入增益", 0.0, 24.0, 0.0, "dB"),
    Spec::new("ceiling", "输出上限", -12.0, 0.0, -0.3, "dB"),
    Spec::new("release", "释放", 1.0, 1000.0, 50.0, "ms").log(),
    Spec::new("lookahead", "前视", 0.1, 10.0, 3.0, "ms").fixed(),
    Spec::new("link", "立体声联动", 0.0, 100.0, 100.0, "%"),
];

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
    need: [DelayLine; 2],
    env: [f64; 2],
    held: [DelayLine; 2],
    sum: [f64; 2],
    release: f64,
    drive: f64,
    ceiling: f64,
    gr_db: f64,
}

impl LimiterProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let capacity = lookahead_samples(10.0, sample_rate.max(8_000.0)) + 2;
        let line = || [DelayLine::new(capacity), DelayLine::new(capacity)];
        let mut p = Self { values: super::table_defaults(LIMITER_SPECS), sample_rate, window: 1, audio: line(), need: line(),
            env: [1.0; 2], held: line(), sum: [0.0; 2], release: 0.0, drive: 1.0, ceiling: 1.0, gr_db: 0.0 };
        p.refresh();
        p.reset_window();
        p
    }
    pub fn gain_reduction_db(&self) -> f64 { self.gr_db }
    fn reset_window(&mut self) {
        for c in 0..2 {
            self.audio[c].clear();
            for _ in 0..self.need[c].capacity() { self.need[c].push(1.0); self.held[c].push(1.0); }
            self.env[c] = 1.0;
            self.sum[c] = self.window as f64;
        }
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
        self.release = time_coefficient(v[2], self.sample_rate);
        let window = lookahead_samples(v[3], self.sample_rate).clamp(1, self.audio[0].capacity() - 2);
        if window != self.window { self.window = window; self.reset_window(); }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { *self = Self { values: self.values.clone(), ..Self::new(sample_rate) }; self.refresh(); self.reset_window(); }
    fn latency(&self) -> u32 { self.window as u32 }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], _key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let link = self.values[4] / 100.0;
        let w = self.window;
        for i in 0..in_l.len() {
            let x = [in_l[i] as f64 * self.drive, in_r[i] as f64 * self.drive];
            let own = x.map(|s| (self.ceiling / s.abs().max(1e-12)).min(1.0));
            let linked = own[0].min(own[1]);
            let mut y = [0.0; 2];
            for c in 0..2 {
                let need = own[c] + (linked - own[c]) * link;
                self.need[c].push(need);
                // Sliding minimum over the window (window ≤ 480 samples).
                let mut hold = need;
                for k in 1..=w { hold = hold.min(self.need[c].tap(k)); }
                self.env[c] = if hold < self.env[c] { hold } else { hold + self.release * (self.env[c] - hold) };
                self.held[c].push(self.env[c]);
                self.sum[c] += self.env[c] - self.held[c].tap(w);
                let gain = (self.sum[c] / w as f64).min(1.0);
                self.audio[c].push(x[c]);
                y[c] = (self.audio[c].tap(w) * gain).clamp(-self.ceiling, self.ceiling);
                if c == 0 { self.gr_db = lin_to_db(gain); }
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
}
