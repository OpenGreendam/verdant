//! DSP of Verdant's built-in mixing effects and the curves their editors
//! draw. The host process runs the processors; the editor evaluates the same
//! coefficient and gain-computer functions, so the picture is the sound.
//!
//! - `verdant.builtin.eq`: six-band parametric EQ (low cut, low shelf, three
//!   bells, high shelf) built from RBJ biquads.
//! - `verdant.builtin.compressor`: stereo-linked feed-forward compressor with
//!   a soft knee, attack/release smoothing of the gain reduction, makeup and
//!   dry/wet mix.
//!
//! Parameters are plain values in their own units (Hz, dB, ms, %, ratio).

use crate::proto::ParamDesc;

pub const EQ_ID: &str = "verdant.builtin.eq";
pub const COMPRESSOR_ID: &str = "verdant.builtin.compressor";

/// Built-in effect ids other than the diagnostic gain.
pub const EFFECT_IDS: [&str; 2] = [EQ_ID, COMPRESSOR_ID];

pub fn display_name(id: &str) -> Option<&'static str> {
    match id {
        EQ_ID => Some("Verdant EQ"),
        COMPRESSOR_ID => Some("Verdant Compressor"),
        _ => None,
    }
}

fn desc(id: impl Into<String>, name: impl Into<String>, value: f64, min: f64, max: f64, unit: &str, steps: i64) -> ParamDesc {
    ParamDesc { id: id.into(), name: name.into(), value, min, max, unit: unit.into(), step_count: steps, default_value: Some(value) }
}

fn db_to_lin(db: f64) -> f64 { 10f64.powf(db / 20.0) }

// ---------------------------------------------------------------------------
// EQ
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind { LowCut, LowShelf, Bell, HighShelf, HighCut }

impl BandKind {
    /// Cuts have no gain; their Q shapes the corner.
    pub fn has_gain(self) -> bool { !matches!(self, Self::LowCut | Self::HighCut) }
}

pub const EQ_BANDS: usize = 6;
/// (kind, default frequency, default Q, default on).
pub const EQ_LAYOUT: [(BandKind, f64, f64, bool); EQ_BANDS] = [
    (BandKind::LowCut, 60.0, 0.707, false),
    (BandKind::LowShelf, 150.0, 0.707, true),
    (BandKind::Bell, 400.0, 1.0, true),
    (BandKind::Bell, 1_500.0, 1.0, true),
    (BandKind::Bell, 4_500.0, 1.0, true),
    (BandKind::HighShelf, 10_000.0, 0.707, true),
];
pub const FREQ_MIN: f64 = 20.0;
pub const FREQ_MAX: f64 = 20_000.0;
pub const GAIN_RANGE: f64 = 24.0;
pub const Q_MIN: f64 = 0.1;
pub const Q_MAX: f64 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band { pub kind: BandKind, pub on: bool, pub freq: f64, pub gain_db: f64, pub q: f64 }

#[derive(Clone, Debug, PartialEq)]
pub struct EqParams { pub bands: [Band; EQ_BANDS], pub output_db: f64 }

impl Default for EqParams {
    fn default() -> Self {
        let bands = EQ_LAYOUT.map(|(kind, freq, q, on)| Band { kind, on, freq, gain_db: 0.0, q });
        Self { bands, output_db: 0.0 }
    }
}

pub fn eq_param_id(band: usize, field: &str) -> String { format!("b{}_{field}", band + 1) }

impl EqParams {
    /// Parameter order is stable: the index is the automation id.
    pub fn descs(&self) -> Vec<ParamDesc> {
        let mut out = Vec::with_capacity(EQ_BANDS * 4 + 1);
        for (i, b) in self.bands.iter().enumerate() {
            let n = i + 1;
            out.push(desc(eq_param_id(i, "on"), format!("频段 {n} 开关"), if b.on { 1.0 } else { 0.0 }, 0.0, 1.0, "", 1));
            out.push(desc(eq_param_id(i, "freq"), format!("频段 {n} 频率"), b.freq, FREQ_MIN, FREQ_MAX, "Hz", 0));
            out.push(desc(eq_param_id(i, "gain"), format!("频段 {n} 增益"), b.gain_db, -GAIN_RANGE, GAIN_RANGE, "dB", 0));
            out.push(desc(eq_param_id(i, "q"), format!("频段 {n} Q"), b.q, Q_MIN, Q_MAX, "", 0));
        }
        out.push(desc("output", "输出", self.output_db, -GAIN_RANGE, GAIN_RANGE, "dB", 0));
        // Defaults are the layout's, not the current values.
        let defaults = Self::default();
        let reference = defaults.values();
        for (d, v) in out.iter_mut().zip(reference) { d.default_value = Some(v); }
        out
    }

    pub fn values(&self) -> Vec<f64> { self.descs_values() }

    fn descs_values(&self) -> Vec<f64> {
        let mut v = Vec::with_capacity(EQ_BANDS * 4 + 1);
        for b in &self.bands { v.extend([if b.on { 1.0 } else { 0.0 }, b.freq, b.gain_db, b.q]); }
        v.push(self.output_db);
        v
    }

    pub fn param_count() -> usize { EQ_BANDS * 4 + 1 }

    pub fn index_of(id: &str) -> Option<usize> {
        if id == "output" { return Some(EQ_BANDS * 4); }
        let rest = id.strip_prefix('b')?;
        let (n, field) = rest.split_once('_')?;
        let band: usize = n.parse().ok()?;
        if band == 0 || band > EQ_BANDS { return None; }
        let f = match field { "on" => 0, "freq" => 1, "gain" => 2, "q" => 3, _ => return None };
        Some((band - 1) * 4 + f)
    }

    /// Set by stable index; values outside the parameter's range are refused.
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        if !value.is_finite() { return Err("non-finite EQ parameter".into()); }
        if index == EQ_BANDS * 4 {
            if !(-GAIN_RANGE..=GAIN_RANGE).contains(&value) { return Err("EQ output outside ±24 dB".into()); }
            self.output_db = value;
            return Ok(());
        }
        let band = self.bands.get_mut(index / 4).ok_or("unknown EQ parameter")?;
        match index % 4 {
            0 => band.on = value >= 0.5,
            1 if (FREQ_MIN..=FREQ_MAX).contains(&value) => band.freq = value,
            2 if (-GAIN_RANGE..=GAIN_RANGE).contains(&value) => band.gain_db = value,
            3 if (Q_MIN..=Q_MAX).contains(&value) => band.q = value,
            _ => return Err("EQ parameter outside its range".into()),
        }
        Ok(())
    }

    pub fn set(&mut self, id: &str, value: f64) -> Result<(), String> {
        let index = Self::index_of(id).ok_or_else(|| format!("unknown EQ parameter {id}"))?;
        self.set_index(index, value)
    }

    /// The combined response in dB at `freq` (output gain included).
    pub fn response_db(&self, freq: f64, sample_rate: f64) -> f64 {
        self.output_db + self.bands.iter().filter(|b| b.on).map(|b| band_response_db(b, freq, sample_rate)).sum::<f64>()
    }
}

/// Normalized biquad coefficients `[b0, b1, b2, a1, a2]` (a0 = 1).
pub fn band_coefficients(band: &Band, sample_rate: f64) -> [f64; 5] {
    // RBJ Audio EQ Cookbook. Frequencies near Nyquist are clamped so the
    // design stays stable at 48 kHz.
    let f = band.freq.clamp(FREQ_MIN, sample_rate * 0.49);
    let w0 = std::f64::consts::TAU * f / sample_rate;
    let (sin, cos) = w0.sin_cos();
    let q = band.q.clamp(Q_MIN, Q_MAX);
    let alpha = sin / (2.0 * q);
    let a = 10f64.powf(band.gain_db / 40.0);
    let (b0, b1, b2, a0, a1, a2) = match band.kind {
        BandKind::LowCut => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        BandKind::HighCut => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        BandKind::Bell => (1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a),
        BandKind::LowShelf => {
            let s = 2.0 * a.sqrt() * alpha;
            (a * ((a + 1.0) - (a - 1.0) * cos + s), 2.0 * a * ((a - 1.0) - (a + 1.0) * cos), a * ((a + 1.0) - (a - 1.0) * cos - s),
             (a + 1.0) + (a - 1.0) * cos + s, -2.0 * ((a - 1.0) + (a + 1.0) * cos), (a + 1.0) + (a - 1.0) * cos - s)
        }
        BandKind::HighShelf => {
            let s = 2.0 * a.sqrt() * alpha;
            (a * ((a + 1.0) + (a - 1.0) * cos + s), -2.0 * a * ((a - 1.0) + (a + 1.0) * cos), a * ((a + 1.0) + (a - 1.0) * cos - s),
             (a + 1.0) - (a - 1.0) * cos + s, 2.0 * ((a - 1.0) - (a + 1.0) * cos), (a + 1.0) - (a - 1.0) * cos - s)
        }
    };
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// |H(e^jw)| of one band in dB.
pub fn band_response_db(band: &Band, freq: f64, sample_rate: f64) -> f64 {
    let [b0, b1, b2, a1, a2] = band_coefficients(band, sample_rate);
    let w = std::f64::consts::TAU * freq.clamp(1.0, sample_rate * 0.5) / sample_rate;
    let (c1, s1) = (w.cos(), w.sin());
    let (c2, s2) = ((2.0 * w).cos(), (2.0 * w).sin());
    let num = ((b0 + b1 * c1 + b2 * c2).powi(2) + (b1 * s1 + b2 * s2).powi(2)).sqrt();
    let den = ((1.0 + a1 * c1 + a2 * c2).powi(2) + (a1 * s1 + a2 * s2).powi(2)).sqrt();
    20.0 * (num / den.max(1e-12)).max(1e-12).log10()
}

#[derive(Clone, Copy)]
struct BiquadCoefficients {
    k1: f64, k2: f64,
    t1: f64, t2: f64,
    b0: f64, c1: f64, c2: f64,
}

impl BiquadCoefficients {
    fn new([b0, b1, b2, a1, a2]: [f64; 5]) -> Self {
        // Schur reflection coefficients of the RBJ denominator. Its stable
        // poles give |k1|, |k2| < 1; the t values normalize the lattice.
        let k1 = a1 / (1.0 + a2);
        let k2 = a2;
        let t1 = ((1.0 - k1) * (1.0 + k1)).sqrt();
        let t2 = ((1.0 - k2) * (1.0 + k2)).sqrt();
        let residual2 = b2 - b0 * a2;
        // Output taps recover the original numerator exactly in steady state.
        let c1 = (b1 - b0 * a1 - k1 * residual2) / (t1 * t2);
        let c2 = residual2 / t2;
        Self { k1, k2, t1, t2, b0, c1, c2 }
    }
}

#[derive(Clone, Copy, Default)]
struct BiquadState { s1: f64, s2: f64 }

impl BiquadState {
    #[inline]
    fn run(&mut self, c: &BiquadCoefficients, x: f64) -> f64 {
        let y = c.b0 * x + c.c1 * self.s1 + c.c2 * self.s2;
        let reflected = c.t2 * x - c.k2 * self.s2;
        // This is an orthogonal rotation/reflection of (s1, reflected).
        // With x = 0, the squared state norm becomes s1² + k2²*s2²,
        // so even sample-by-sample changes cannot amplify stored state energy.
        let s1 = -c.k1 * self.s1 + c.t1 * reflected;
        self.s2 = c.t1 * self.s1 + c.k1 * reflected;
        self.s1 = s1;
        y
    }
}

/// Stereo EQ processor. Coefficients are recomputed only when a parameter
/// changes. Automation takes effect on the next processed sample, retaining
/// the normalized lattice state without interpolation or an extra audio path.
/// Abrupt changes can produce a bounded transient/click. Bypassing a band
/// discards its state immediately; re-enabling starts with an empty tail.
pub struct EqProcessor {
    pub params: EqParams,
    sample_rate: f64,
    coeffs: [BiquadCoefficients; EQ_BANDS],
    state: [[BiquadState; EQ_BANDS]; 2],
    output: f64,
}

impl EqProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { params: EqParams::default(), sample_rate, coeffs: [BiquadCoefficients::new([1.0, 0.0, 0.0, 0.0, 0.0]); EQ_BANDS], state: Default::default(), output: 1.0 };
        p.refresh();
        p
    }
    pub fn set_sample_rate(&mut self, sample_rate: f64) { self.sample_rate = sample_rate; self.state = Default::default(); self.refresh(); }
    pub fn refresh(&mut self) {
        for (i, (c, b)) in self.coeffs.iter_mut().zip(&self.params.bands).enumerate() {
            *c = BiquadCoefficients::new(band_coefficients(b, self.sample_rate));
            if !b.on { for channel in &mut self.state { channel[i] = BiquadState::default(); } }
        }
        self.output = db_to_lin(self.params.output_db);
    }
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        self.params.set_index(index, value)?;
        self.refresh();
        Ok(())
    }
    pub fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        let on: [bool; EQ_BANDS] = std::array::from_fn(|i| self.params.bands[i].on);
        // `params` is public: also honor bypass set directly before a block.
        for (b, &enabled) in on.iter().enumerate() {
            if !enabled { for channel in &mut self.state { channel[b] = BiquadState::default(); } }
        }
        for i in 0..in_l.len() {
            let (mut l, mut r) = (in_l[i] as f64, in_r[i] as f64);
            for b in 0..EQ_BANDS {
                if !on[b] { continue; }
                l = self.state[0][b].run(&self.coeffs[b], l);
                r = self.state[1][b].run(&self.coeffs[b], r);
            }
            out_l[i] = (l * self.output) as f32;
            out_r[i] = (r * self.output) as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// Compressor
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct CompParams {
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub knee_db: f64,
    pub makeup_db: f64,
    pub mix_pct: f64,
}

impl Default for CompParams {
    fn default() -> Self {
        Self { threshold_db: -18.0, ratio: 3.0, attack_ms: 10.0, release_ms: 120.0, knee_db: 6.0, makeup_db: 0.0, mix_pct: 100.0 }
    }
}

/// (id, name, min, max, unit) in stable automation order.
pub const COMP_PARAMS: [(&str, &str, f64, f64, &str); 7] = [
    ("threshold", "阈值", -60.0, 0.0, "dB"),
    ("ratio", "压缩比", 1.0, 20.0, ":1"),
    ("attack", "启动", 0.1, 200.0, "ms"),
    ("release", "释放", 5.0, 2000.0, "ms"),
    ("knee", "拐点", 0.0, 24.0, "dB"),
    ("makeup", "补偿增益", 0.0, 24.0, "dB"),
    ("mix", "干湿比", 0.0, 100.0, "%"),
];

impl CompParams {
    pub fn values(&self) -> [f64; 7] {
        [self.threshold_db, self.ratio, self.attack_ms, self.release_ms, self.knee_db, self.makeup_db, self.mix_pct]
    }
    pub fn descs(&self) -> Vec<ParamDesc> {
        let defaults = Self::default().values();
        COMP_PARAMS.iter().zip(self.values()).zip(defaults).map(|(((id, name, min, max, unit), v), d)| {
            let mut p = desc(*id, *name, v, *min, *max, unit, 0);
            p.default_value = Some(d);
            p
        }).collect()
    }
    pub fn index_of(id: &str) -> Option<usize> { COMP_PARAMS.iter().position(|(p, ..)| *p == id) }
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        let (_, _, min, max, _) = COMP_PARAMS.get(index).ok_or("unknown compressor parameter")?;
        if !value.is_finite() || !(*min..=*max).contains(&value) { return Err("compressor parameter outside its range".into()); }
        let slot = match index { 0 => &mut self.threshold_db, 1 => &mut self.ratio, 2 => &mut self.attack_ms, 3 => &mut self.release_ms, 4 => &mut self.knee_db, 5 => &mut self.makeup_db, _ => &mut self.mix_pct };
        *slot = value;
        Ok(())
    }
    pub fn set(&mut self, id: &str, value: f64) -> Result<(), String> {
        self.set_index(Self::index_of(id).ok_or_else(|| format!("unknown compressor parameter {id}"))?, value)
    }

    /// Static gain computer (soft knee, Giannoulis et al.): output level for
    /// an input level, both in dBFS, before makeup.
    pub fn curve_db(&self, input_db: f64) -> f64 {
        let (t, r, w) = (self.threshold_db, self.ratio.max(1.0), self.knee_db.max(0.0));
        let over = input_db - t;
        if 2.0 * over < -w {
            input_db
        } else if w > 0.0 && 2.0 * over.abs() <= w {
            input_db + (1.0 / r - 1.0) * (over + w / 2.0).powi(2) / (2.0 * w)
        } else {
            t + over / r
        }
    }
}

/// Stereo-linked compressor: the louder channel drives one gain reduction.
pub struct CompProcessor {
    pub params: CompParams,
    sample_rate: f64,
    attack: f64,
    release: f64,
    /// Smoothed gain reduction in dB (≤ 0).
    gr_db: f64,
    makeup: f64,
    wet: f64,
}

impl CompProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { params: CompParams::default(), sample_rate, attack: 0.0, release: 0.0, gr_db: 0.0, makeup: 1.0, wet: 1.0 };
        p.refresh();
        p
    }
    pub fn set_sample_rate(&mut self, sample_rate: f64) { self.sample_rate = sample_rate; self.gr_db = 0.0; self.refresh(); }
    fn coefficient(ms: f64, sample_rate: f64) -> f64 { (-1.0 / (ms.max(0.01) * 0.001 * sample_rate)).exp() }
    pub fn refresh(&mut self) {
        self.attack = Self::coefficient(self.params.attack_ms, self.sample_rate);
        self.release = Self::coefficient(self.params.release_ms, self.sample_rate);
        self.makeup = db_to_lin(self.params.makeup_db);
        self.wet = self.params.mix_pct / 100.0;
    }
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        self.params.set_index(index, value)?;
        self.refresh();
        Ok(())
    }
    /// Current gain reduction in dB (≤ 0), for meters.
    pub fn gain_reduction_db(&self) -> f64 { self.gr_db }
    pub fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let level = l.abs().max(r.abs());
            let level_db = if level > 1e-9 { 20.0 * level.log10() } else { -180.0 };
            let target = self.params.curve_db(level_db) - level_db;
            // Falling gain (more reduction) follows attack, rising follows release.
            let coeff = if target < self.gr_db { self.attack } else { self.release };
            self.gr_db = target + coeff * (self.gr_db - target);
            let g = db_to_lin(self.gr_db) * self.makeup;
            out_l[i] = (l * (1.0 - self.wet) + l * g * self.wet) as f32;
            out_r[i] = (r * (1.0 - self.wet) + r * g * self.wet) as f32;
        }
    }
}

/// Built-in effect parameter state, serialized as `{"id": value, …}`.
pub fn state_json(descs: &[ParamDesc]) -> Vec<u8> {
    let map: serde_json::Map<String, serde_json::Value> = descs.iter().map(|d| (d.id.clone(), serde_json::json!(d.value))).collect();
    serde_json::to_vec(&serde_json::Value::Object(map)).expect("serializable state")
}

/// Parse state written by `state_json`; unknown ids are refused.
pub fn parse_state(data: &[u8]) -> Result<Vec<(String, f64)>, String> {
    let value: serde_json::Value = serde_json::from_slice(data).map_err(|e| format!("bad state: {e}"))?;
    let map = value.as_object().ok_or("state must be an object")?;
    map.iter().map(|(k, v)| v.as_f64().map(|v| (k.clone(), v)).ok_or_else(|| format!("state value {k} is not a number"))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    const FS: f64 = 48_000.0;

    fn sine(freq: f64, amp: f64, n: usize) -> Vec<f32> {
        (0..n).map(|i| (amp * (std::f64::consts::TAU * freq * i as f64 / FS).sin()) as f32).collect()
    }
    fn rms_db(x: &[f32]) -> f64 {
        let s = x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64;
        10.0 * s.max(1e-30).log10()
    }

    fn single_band_eq(band: Band) -> EqProcessor {
        let mut proc_ = EqProcessor::new(FS);
        for b in &mut proc_.params.bands { b.on = false; }
        proc_.params.bands[3] = band;
        proc_.refresh();
        proc_
    }

    #[test]
    fn flat_eq_is_transparent_and_a_bell_boosts_its_frequency() {
        let p = EqParams::default();
        for f in [30.0, 200.0, 1000.0, 8000.0] { assert!(p.response_db(f, FS).abs() < 1e-6, "flat at {f}"); }
        let mut proc_ = EqProcessor::new(FS);
        proc_.params.set("b4_gain", 9.0).unwrap();
        proc_.refresh();
        assert!((proc_.params.response_db(1500.0, FS) - 9.0).abs() < 0.05);
        // The processor matches the drawn curve.
        let x = sine(1500.0, 0.25, 48_000);
        let mut l = vec![0.0; x.len()];
        let mut r = vec![0.0; x.len()];
        proc_.process(&x, &x, &mut l, &mut r);
        let gain = rms_db(&l[24_000..]) - rms_db(&x[24_000..]);
        assert!((gain - 9.0).abs() < 0.1, "measured {gain}");
        // Far from the bell the level is unchanged.
        assert!(proc_.params.response_db(60.0, FS).abs() < 0.2);
    }

    #[test]
    fn eq_frequency_switching_after_an_impulse_stays_bounded() {
        let mut proc_ = single_band_eq(Band { kind: BandKind::Bell, on: true, freq: 6_000.0, gain_db: 24.0, q: 18.0 });
        let freq_index = EqParams::index_of("b4_freq").unwrap();
        let (mut l, mut r) = ([0.0], [0.0]);
        proc_.process(&[1.0], &[-0.5], &mut l, &mut r);
        for sample in 0..8_192 {
            let freq = if sample % 2 == 0 { 18_000.0 } else { 6_000.0 };
            proc_.set_index(freq_index, freq).unwrap();
            proc_.process(&[0.0], &[0.0], &mut l, &mut r);
            assert!(l[0].is_finite() && r[0].is_finite(), "non-finite tail at sample {sample}");
            // A generous ceiling above the band's maximum static gain still
            // catches the exponential growth of switched DF-II-T delays.
            assert!(l[0].abs() < 32.0 && r[0].abs() < 32.0, "growing tail at sample {sample}: {l:?}, {r:?}");
            if sample == 0 {
                assert!(l[0].abs() > 1e-3 && r[0].abs() > 1e-3, "automation must retain the existing tail");
            }
        }
        assert!(l[0].abs() < 1e-4 && r[0].abs() < 1e-4, "the switched tail must decay: {l:?}, {r:?}");
    }

    #[test]
    fn eq_automation_applies_at_the_next_sample() {
        let mut proc_ = single_band_eq(Band { kind: BandKind::Bell, on: true, freq: 6_000.0, gain_db: 0.0, q: 18.0 });
        let (mut prefix_l, mut prefix_r) = ([0.0; 8], [0.0; 8]);
        proc_.process(&[0.0; 8], &[0.0; 8], &mut prefix_l, &mut prefix_r);
        proc_.set_index(EqParams::index_of("b4_freq").unwrap(), 18_000.0).unwrap();
        proc_.set_index(EqParams::index_of("b4_gain").unwrap(), 24.0).unwrap();
        let [b0, b1, _, a1, _] = band_coefficients(&proc_.params.bands[3], FS);
        let (mut l, mut r) = ([0.0; 2], [0.0; 2]);
        proc_.process(&[1.0, 0.0], &[-1.0, 0.0], &mut l, &mut r);
        assert!((l[0] as f64 - b0).abs() < 1e-6, "the first sample uses the admitted gain");
        assert!((l[1] as f64 - (b1 - a1 * b0)).abs() < 1e-6, "the next sample uses the admitted frequency");
        assert_eq!(r, l.map(|v| -v));
    }

    #[test]
    fn eq_bypassed_band_discards_its_tail() {
        // Cover bypass/re-enable between samples, a dry interval, and direct
        // mutation of the public parameter state before processing a block.
        for (bypass_samples, use_setter) in [(0, true), (32, true), (32, false)] {
            let mut proc_ = single_band_eq(Band { kind: BandKind::Bell, on: true, freq: 6_000.0, gain_db: 24.0, q: 18.0 });
            let on_index = EqParams::index_of("b4_on").unwrap();
            let (mut tail_l, mut tail_r) = ([0.0; 2], [0.0; 2]);
            proc_.process(&[1.0, 0.0], &[-0.5, 0.0], &mut tail_l, &mut tail_r);
            assert!(tail_l[1].abs() > 1e-3 && tail_r[1].abs() > 1e-3);
            if use_setter { proc_.set_index(on_index, 0.0).unwrap(); }
            else { proc_.params.bands[3].on = false; }
            let (dry_l, dry_r) = ([0.25; 32], [-0.125; 32]);
            let (mut bypass_l, mut bypass_r) = ([0.0; 32], [0.0; 32]);
            proc_.process(&dry_l[..bypass_samples], &dry_r[..bypass_samples], &mut bypass_l[..bypass_samples], &mut bypass_r[..bypass_samples]);
            assert_eq!(&bypass_l[..bypass_samples], &dry_l[..bypass_samples]);
            assert_eq!(&bypass_r[..bypass_samples], &dry_r[..bypass_samples]);
            proc_.set_index(on_index, 1.0).unwrap();
            let (mut l, mut r) = ([0.0; 64], [0.0; 64]);
            proc_.process(&[0.0; 64], &[0.0; 64], &mut l, &mut r);
            assert_eq!(l, [0.0; 64], "a re-enabled band must not resurrect its left tail");
            assert_eq!(r, [0.0; 64], "a re-enabled band must not resurrect its right tail");
        }
    }

    #[test]
    fn eq_steady_response_matches_the_curve_after_parameter_changes() {
        let mut proc_ = single_band_eq(Band { kind: BandKind::Bell, on: true, freq: 1_500.0, gain_db: 9.0, q: 1.0 });
        for (kind, freq, gain_db, q, probe) in [
            (BandKind::LowCut, 1_000.0, 0.0, 0.1, 2_000.0),
            (BandKind::LowShelf, 1_000.0, 24.0, 18.0, 500.0),
            (BandKind::Bell, 6_000.0, 24.0, 18.0, 6_000.0),
            (BandKind::HighShelf, 10_000.0, -24.0, 0.1, 16_000.0),
            (BandKind::HighCut, 6_000.0, 0.0, 18.0, 3_000.0),
        ] {
            proc_.params.bands[3] = Band { kind, on: true, freq, gain_db, q };
            proc_.refresh();
            let x = sine(probe, 0.02, 48_000);
            let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
            proc_.process(&x, &x, &mut l, &mut r);
            let expected = proc_.params.response_db(probe, FS);
            for out in [&l, &r] {
                let measured = rms_db(&out[24_000..]) - rms_db(&x[24_000..]);
                assert!((measured - expected).abs() < 0.02, "{kind:?}: measured {measured}, curve {expected}");
            }
        }
    }

    #[test]
    fn shelves_and_cuts_shape_the_ends() {
        let mut p = EqParams::default();
        p.set("b6_gain", -6.0).unwrap();
        assert!((p.response_db(18_000.0, FS) + 6.0).abs() < 0.6);
        p.set("b1_on", 1.0).unwrap();
        p.set("b1_freq", 100.0).unwrap();
        assert!(p.response_db(25.0, FS) < -20.0, "low cut removes rumble");
        assert!(p.response_db(1000.0, FS).abs() < 0.5);
    }

    #[test]
    fn eq_parameters_round_trip_by_id_index_and_state() {
        let mut p = EqParams::default();
        assert_eq!(EqParams::index_of("b3_q"), Some(2 * 4 + 3));
        assert_eq!(EqParams::index_of("output"), Some(24));
        assert_eq!(EqParams::index_of("b7_q"), None);
        assert!(p.set("b2_freq", 5.0).is_err(), "below 20 Hz refused");
        assert!(p.set("b2_gain", f64::NAN).is_err());
        p.set("b2_gain", 3.5).unwrap();
        let descs = p.descs();
        assert_eq!(descs.len(), EqParams::param_count());
        assert_eq!(descs[4 + 2].value, 3.5);
        assert_eq!(descs[4 + 2].default_value, Some(0.0));
        let restored = parse_state(&state_json(&descs)).unwrap();
        let mut q = EqParams::default();
        for (id, v) in restored { q.set(&id, v).unwrap(); }
        assert_eq!(p, q);
    }

    #[test]
    fn compressor_curve_has_threshold_ratio_and_a_smooth_knee() {
        let p = CompParams { threshold_db: -20.0, ratio: 4.0, knee_db: 0.0, ..Default::default() };
        assert_eq!(p.curve_db(-30.0), -30.0);
        assert!((p.curve_db(-8.0) - (-17.0)).abs() < 1e-9);
        let soft = CompParams { knee_db: 10.0, ..p.clone() };
        // Continuous and monotonic across the knee.
        let mut previous = soft.curve_db(-40.0);
        for i in 1..=400 {
            let x = -40.0 + i as f64 * 0.1;
            let y = soft.curve_db(x);
            assert!(y >= previous - 1e-9 && (y - previous) <= 0.1 + 1e-9);
            previous = y;
        }
        assert!((soft.curve_db(-20.0) - (-20.0 - 0.75 * 25.0 / 20.0)).abs() < 1e-9);
    }

    #[test]
    fn compressor_reduces_loud_signals_and_leaves_quiet_ones() {
        let mut c = CompProcessor::new(FS);
        c.params.set("threshold", -20.0).unwrap();
        c.params.set("ratio", 4.0).unwrap();
        c.params.set("knee", 0.0).unwrap();
        c.refresh();
        let loud = sine(440.0, 1.0, 48_000);
        let mut l = vec![0.0; loud.len()];
        let mut r = vec![0.0; loud.len()];
        c.process(&loud, &loud, &mut l, &mut r);
        let reduction = rms_db(&loud[24_000..]) - rms_db(&l[24_000..]);
        assert!(reduction > 9.0 && reduction < 16.0, "reduction {reduction}");
        assert!(c.gain_reduction_db() < -5.0);
        let mut quiet_c = CompProcessor::new(FS);
        let quiet = sine(440.0, 0.01, 4_800);
        let mut ql = vec![0.0; quiet.len()];
        let mut qr = vec![0.0; quiet.len()];
        quiet_c.process(&quiet, &quiet, &mut ql, &mut qr);
        assert!((rms_db(&ql) - rms_db(&quiet)).abs() < 0.01);
        // Fully dry mix is transparent.
        let mut dry = CompProcessor::new(FS);
        dry.set_index(6, 0.0).unwrap();
        dry.process(&loud, &loud, &mut l, &mut r);
        assert_eq!(l, loud);
    }
}
