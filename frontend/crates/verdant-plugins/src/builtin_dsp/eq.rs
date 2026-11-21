//! Twenty-four-band parametric/dynamic EQ. Each band can take nine shapes; cuts
//! cascade up to four Butterworth sections (12–48 dB/oct); each band runs on
//! both channels, one side, or the mid/side signal.

use super::filters::{butterworth_qs, db_to_lin, lin_to_db, response_db, rbj, time_coefficient, Biquad, Coefficients, Lattice, LatticeState, Rbj, IDENTITY};
use super::{desc, Spec};
use crate::proto::ParamDesc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind { Bell, LowShelf, LowCut, HighShelf, HighCut, Notch, BandPass, TiltShelf, AllPass }

impl BandKind {
    pub const ALL: [BandKind; 9] = [Self::Bell, Self::LowShelf, Self::LowCut, Self::HighShelf, Self::HighCut, Self::Notch, Self::BandPass, Self::TiltShelf, Self::AllPass];
    pub fn index(self) -> usize { Self::ALL.iter().position(|k| *k == self).unwrap_or(0) }
    pub fn from_index(value: f64) -> Self { Self::ALL[(value.round().max(0.0) as usize).min(8)] }
    /// Cuts, notches and band-passes have no gain; their Q shapes the corner.
    pub fn has_gain(self) -> bool { matches!(self, Self::Bell | Self::LowShelf | Self::HighShelf | Self::TiltShelf) }
    pub fn has_slope(self) -> bool { matches!(self, Self::LowCut | Self::HighCut) }
    pub fn label(self) -> &'static str {
        match self {
            Self::Bell => "峰值", Self::LowShelf => "低架", Self::LowCut => "低切", Self::HighShelf => "高架",
            Self::HighCut => "高切", Self::Notch => "陷波", Self::BandPass => "带通", Self::TiltShelf => "倾斜", Self::AllPass => "全通",
        }
    }
}

/// Where a band acts in the stereo image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement { Stereo, Left, Right, Mid, Side }

impl Placement {
    pub const ALL: [Placement; 5] = [Self::Stereo, Self::Left, Self::Right, Self::Mid, Self::Side];
    pub fn from_index(value: f64) -> Self { Self::ALL[(value.round().max(0.0) as usize).min(4)] }
    pub fn index(self) -> usize { Self::ALL.iter().position(|p| *p == self).unwrap_or(0) }
    pub fn label(self) -> &'static str {
        match self { Self::Stereo => "立体声", Self::Left => "左", Self::Right => "右", Self::Mid => "中", Self::Side => "侧" }
    }
    pub fn short(self) -> &'static str {
        match self { Self::Stereo => "", Self::Left => "L", Self::Right => "R", Self::Mid => "M", Self::Side => "S" }
    }
}

pub const EQ_BANDS: usize = 24;
/// Up to four biquad sections per band (48 dB/oct cuts).
pub const MAX_SECTIONS: usize = 4;
/// (kind, default frequency, default Q, default on).
pub const EQ_LAYOUT: [(BandKind, f64, f64, bool); EQ_BANDS] = {
    let mut layout = [(BandKind::Bell, 1000.0, 1.0, false); EQ_BANDS];
    let legacy = [
    (BandKind::LowCut, 60.0, 0.707, false),
    (BandKind::LowShelf, 150.0, 0.707, true),
    (BandKind::Bell, 400.0, 1.0, true),
    (BandKind::Bell, 1_500.0, 1.0, true),
    (BandKind::Bell, 4_500.0, 1.0, true),
    (BandKind::HighShelf, 10_000.0, 0.707, true),
    ];
    let mut i = 0;
    while i < legacy.len() { layout[i] = legacy[i]; i += 1; }
    layout
};
pub const FREQ_MIN: f64 = 20.0;
pub const FREQ_MAX: f64 = 20_000.0;
pub const GAIN_RANGE: f64 = 24.0;
pub const Q_MIN: f64 = 0.1;
pub const Q_MAX: f64 = 18.0;
/// Cut slopes in dB/oct by slope index.
pub const SLOPES: [u32; 4] = [12, 24, 36, 48];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band { pub kind: BandKind, pub on: bool, pub freq: f64, pub gain_db: f64, pub q: f64, pub slope: usize, pub place: Placement }

impl Band {
    /// The biquad sections this band runs, in order.
    pub fn sections(&self, sample_rate: f64) -> ([Coefficients; MAX_SECTIONS], usize) {
        let mut out = [IDENTITY; MAX_SECTIONS];
        let (f, q, g) = (self.freq, self.q.clamp(Q_MIN, Q_MAX), self.gain_db);
        let n = match self.kind {
            BandKind::Bell => { out[0] = rbj(Rbj::Bell, f, q, g, sample_rate); 1 }
            BandKind::LowShelf => { out[0] = rbj(Rbj::LowShelf, f, q, g, sample_rate); 1 }
            BandKind::HighShelf => { out[0] = rbj(Rbj::HighShelf, f, q, g, sample_rate); 1 }
            BandKind::Notch => { out[0] = rbj(Rbj::Notch, f, q, 0.0, sample_rate); 1 }
            BandKind::BandPass => { out[0] = rbj(Rbj::BandPass, f, q, 0.0, sample_rate); 1 }
            BandKind::AllPass => {
                let c = rbj(Rbj::LowPass, f, q, 0.0, sample_rate);
                out[0] = [c[4], c[3], 1.0, c[3], c[4]];
                1
            }
            BandKind::TiltShelf => {
                out[0] = rbj(Rbj::LowShelf, f, q, -g / 2.0, sample_rate);
                out[1] = rbj(Rbj::HighShelf, f, q, g / 2.0, sample_rate);
                2
            }
            BandKind::LowCut | BandKind::HighCut => {
                let kind = if self.kind == BandKind::LowCut { Rbj::HighPass } else { Rbj::LowPass };
                let qs = butterworth_qs(self.slope.min(3) + 1);
                // Q sets the resonance of the sharpest section; the others
                // keep their Butterworth values so the slope stays clean.
                let resonance = q / std::f64::consts::FRAC_1_SQRT_2;
                for (i, &bq) in qs.iter().enumerate() {
                    let section_q = if i + 1 == qs.len() { bq * resonance } else { bq };
                    out[i] = rbj(kind, f, section_q, 0.0, sample_rate);
                }
                qs.len()
            }
        };
        (out, n)
    }
}

/// Dynamics on a bell or shelf: signed range is the maximum gain change.
/// A separate detector follows the band's original input (or routed key).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicBand { pub enabled: bool, pub range_db: f64, pub threshold_db: f64, pub attack_ms: f64, pub release_ms: f64 }

impl Default for DynamicBand {
    fn default() -> Self { Self { enabled: false, range_db: -6.0, threshold_db: -24.0, attack_ms: 10.0, release_ms: 150.0 } }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EqParams {
    pub bands: [Band; EQ_BANDS],
    pub output_db: f64,
    pub dynamics: [DynamicBand; EQ_BANDS],
    /// Zero disables audition; 1..=24 selects a band-pass audition filter.
    pub solo: usize,
    pub auto_gain: bool,
}

impl Default for EqParams {
    fn default() -> Self {
        let bands = EQ_LAYOUT.map(|(kind, freq, q, on)| Band { kind, on, freq, gain_db: 0.0, q, slope: 1, place: Placement::Stereo });
        Self { bands, output_db: 0.0, dynamics: [DynamicBand::default(); EQ_BANDS], solo: 0, auto_gain: false }
    }
}

pub fn eq_param_id(band: usize, field: &str) -> String { format!("b{}_{field}", band + 1) }

// The original six-band automation ABI is immutable: output stays at 24,
// b1_shape at 25 and b6_place at 42. New bands and dynamics are appended.
const ORIGINAL_BANDS: usize = 6;
const LEGACY: usize = ORIGINAL_BANDS * 4 + 1;
const ORIGINAL_COUNT: usize = LEGACY + ORIGINAL_BANDS * 3;
const DYNAMIC_START: usize = ORIGINAL_COUNT + (EQ_BANDS - ORIGINAL_BANDS) * 7;
const GLOBAL_START: usize = DYNAMIC_START + EQ_BANDS * 5;
const BAND_FIELDS: [&str; 7] = ["on", "freq", "gain", "q", "shape", "slope", "place"];
const DYNAMIC_FIELDS: [&str; 5] = ["dynamic", "dyn_range", "dyn_threshold", "dyn_attack", "dyn_release"];

/// Formatting/range metadata of one EQ parameter.
pub fn eq_spec(id: &str) -> Option<Spec> {
    if matches!(id, "output" | "solo" | "auto_gain") { return eq_field_spec(id); }
    let (n, field) = id.strip_prefix('b')?.split_once('_')?;
    let band: usize = n.parse().ok()?;
    if !(1..=EQ_BANDS).contains(&band) { return None; }
    eq_field_spec(field)
}

fn eq_field_spec(field: &str) -> Option<Spec> {
    Some(match field {
        "output" => Spec::new("output", "输出", -GAIN_RANGE, GAIN_RANGE, 0.0, "dB"),
        "solo" => Spec::new("solo", "独听频段", 0.0, EQ_BANDS as f64, 0.0, "").steps(EQ_BANDS as i64),
        "auto_gain" => Spec::new("auto_gain", "自动增益", 0.0, 1.0, 0.0, "").steps(1),
        "on" => Spec::new("on", "开关", 0.0, 1.0, 1.0, "").steps(1),
        "freq" => Spec::new("freq", "频率", FREQ_MIN, FREQ_MAX, 1000.0, "Hz").log(),
        "gain" => Spec::new("gain", "增益", -GAIN_RANGE, GAIN_RANGE, 0.0, "dB"),
        "q" => Spec::new("q", "Q", Q_MIN, Q_MAX, 1.0, "").log(),
        "shape" => Spec::new("shape", "形状", 0.0, 8.0, 0.0, "").steps(8),
        "slope" => Spec::new("slope", "陡度", 0.0, 3.0, 1.0, "").steps(3),
        "place" => Spec::new("place", "声像", 0.0, 4.0, 0.0, "").steps(4),
        "dynamic" => Spec::new("dynamic", "动态", 0.0, 1.0, 0.0, "").steps(1),
        "dyn_range" => Spec::new("dyn_range", "动态范围", -24.0, 24.0, -6.0, "dB"),
        "dyn_threshold" => Spec::new("dyn_threshold", "动态阈值", -60.0, 0.0, -24.0, "dB"),
        "dyn_attack" => Spec::new("dyn_attack", "动态启动", 0.1, 200.0, 10.0, "ms").log(),
        "dyn_release" => Spec::new("dyn_release", "动态释放", 5.0, 2000.0, 150.0, "ms").log(),
        _ => return None,
    })
}

impl EqParams {
    /// Reverse mapping for stable host automation and descriptor order.
    fn field_at(index: usize) -> Option<(Option<usize>, &'static str)> {
        if index < 24 { return Some((Some(index / 4), BAND_FIELDS[index % 4])); }
        if index == 24 { return Some((None, "output")); }
        if index < ORIGINAL_COUNT { return Some((Some((index - LEGACY) / 3), BAND_FIELDS[4 + (index - LEGACY) % 3])); }
        if index < DYNAMIC_START { return Some((Some(ORIGINAL_BANDS + (index - ORIGINAL_COUNT) / 7), BAND_FIELDS[(index - ORIGINAL_COUNT) % 7])); }
        if index < GLOBAL_START { return Some((Some((index - DYNAMIC_START) / 5), DYNAMIC_FIELDS[(index - DYNAMIC_START) % 5])); }
        match index - GLOBAL_START { 0 => Some((None, "solo")), 1 => Some((None, "auto_gain")), _ => None }
    }

    fn value_at(&self, index: usize) -> f64 {
        let Some((band, field)) = Self::field_at(index) else { return 0.0; };
        let Some(i) = band else { return match field { "output" => self.output_db, "solo" => self.solo as f64, _ => if self.auto_gain { 1.0 } else { 0.0 } }; };
        let b = &self.bands[i];
        let d = &self.dynamics[i];
        match field {
            "on" => if b.on { 1.0 } else { 0.0 }, "freq" => b.freq, "gain" => b.gain_db, "q" => b.q,
            "shape" => b.kind.index() as f64, "slope" => b.slope as f64, "place" => b.place.index() as f64,
            "dynamic" => if d.enabled { 1.0 } else { 0.0 }, "dyn_range" => d.range_db, "dyn_threshold" => d.threshold_db,
            "dyn_attack" => d.attack_ms, "dyn_release" => d.release_ms, _ => unreachable!(),
        }
    }

    /// Parameter order is stable: the index is the automation id.
    pub fn descs(&self) -> Vec<ParamDesc> {
        let defaults = Self::default();
        (0..Self::param_count()).map(|index| {
            let (band, field) = Self::field_at(index).expect("known EQ index");
            let id = band.map_or_else(|| field.to_string(), |b| eq_param_id(b, field));
            let spec = eq_spec(&id).expect("known EQ parameter");
            let name = band.map_or_else(|| spec.name.to_string(), |b| format!("频段 {} {}", b + 1, spec.name));
            let mut d = desc(id, name, self.value_at(index), spec.min, spec.max, spec.unit, spec.steps);
            d.default_value = Some(defaults.value_at(index));
            d
        }).collect()
    }

    pub fn values(&self) -> Vec<f64> { (0..Self::param_count()).map(|i| self.value_at(i)).collect() }

    pub fn param_count() -> usize { GLOBAL_START + 2 }

    pub fn index_of(id: &str) -> Option<usize> {
        match id { "output" => return Some(24), "solo" => return Some(GLOBAL_START), "auto_gain" => return Some(GLOBAL_START + 1), _ => {} }
        let (n, field) = id.strip_prefix('b')?.split_once('_')?;
        let band: usize = n.parse().ok()?;
        if band == 0 || band > EQ_BANDS { return None; }
        let b = band - 1;
        if let Some(f) = DYNAMIC_FIELDS.iter().position(|s| *s == field) { return Some(DYNAMIC_START + b * 5 + f); }
        let f = BAND_FIELDS.iter().position(|s| *s == field)?;
        Some(if b < ORIGINAL_BANDS { if f < 4 { b * 4 + f } else { LEGACY + b * 3 + f - 4 } } else { ORIGINAL_COUNT + (b - ORIGINAL_BANDS) * 7 + f })
    }

    /// Set by stable index; values outside the parameter's range are refused.
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        let (band, field) = Self::field_at(index).ok_or("unknown EQ parameter")?;
        eq_field_spec(field).ok_or("unknown EQ parameter")?.check(value)?;
        let Some(i) = band else {
            match field { "output" => self.output_db = value, "solo" => self.solo = value.round() as usize, "auto_gain" => self.auto_gain = value >= 0.5, _ => unreachable!() }
            return Ok(());
        };
        let b = &mut self.bands[i];
        let d = &mut self.dynamics[i];
        match field {
            "on" => b.on = value >= 0.5, "freq" => b.freq = value, "gain" => b.gain_db = value, "q" => b.q = value,
            "shape" => b.kind = BandKind::from_index(value), "slope" => b.slope = value.round() as usize, "place" => b.place = Placement::from_index(value),
            "dynamic" => d.enabled = value >= 0.5, "dyn_range" => d.range_db = value, "dyn_threshold" => d.threshold_db = value,
            "dyn_attack" => d.attack_ms = value, "dyn_release" => d.release_ms = value, _ => unreachable!(),
        }
        Ok(())
    }

    pub fn set(&mut self, id: &str, value: f64) -> Result<(), String> {
        let index = Self::index_of(id).ok_or_else(|| format!("unknown EQ parameter {id}"))?;
        self.set_index(index, value)
    }

    /// Build from a parameter list, ignoring unknown or invalid entries.
    pub fn from_descs(params: &[ParamDesc]) -> Self {
        let mut eq = Self::default();
        for p in params { let _ = eq.set(&p.id, p.value); }
        eq
    }

    /// The combined stereo response in dB at `freq` (output gain included).
    /// Bands placed on one side or on M/S are drawn at their own response.
    pub fn response_db(&self, freq: f64, sample_rate: f64) -> f64 {
        self.output_db + self.bands.iter().filter(|b| b.on).map(|b| band_response_db(b, freq, sample_rate)).sum::<f64>()
    }

    /// Static equal-log-frequency energy compensation. Compute this once
    /// per editor curve, then add it to `response_db` samples.
    pub fn auto_gain_db(&self, sample_rate: f64) -> f64 {
        if !self.auto_gain { return 0.0; }
        // Design each static band once, rather than once per probe frequency.
        // This function also runs after admitted parameter automation.
        let filters = self.bands.map(|band| band.sections(sample_rate));
        let energy = (0..48).map(|i| {
            let f = FREQ_MIN * (FREQ_MAX / FREQ_MIN).powf((i as f64 + 0.5) / 48.0);
            let db: f64 = self.bands.iter().zip(&filters).filter(|(band, _)| band.on)
                .map(|(_, (coefficients, count))| coefficients[..*count].iter().map(|&c| response_db(c, f, sample_rate)).sum::<f64>()).sum();
            10.0f64.powf(db / 10.0)
        }).sum::<f64>() / 48.0;
        (-10.0 * energy.max(1e-12).log10()).clamp(-24.0, 24.0)
    }
}

/// The first biquad section of a band.
pub fn band_coefficients(band: &Band, sample_rate: f64) -> Coefficients { band.sections(sample_rate).0[0] }

/// The band's response in dB (all sections).
pub fn band_response_db(band: &Band, freq: f64, sample_rate: f64) -> f64 {
    let (sections, n) = band.sections(sample_rate);
    sections[..n].iter().map(|&c| response_db(c, freq, sample_rate)).sum()
}

/// Stereo EQ processor. Coefficients are recomputed only when a parameter
/// changes and apply from the next sample; the normalized lattice keeps
/// switching bounded. Bypassing a band discards its state.
pub struct EqProcessor {
    pub params: EqParams,
    sample_rate: f64,
    coeffs: [[Lattice; MAX_SECTIONS]; EQ_BANDS],
    sections: [usize; EQ_BANDS],
    state: [[[LatticeState; MAX_SECTIONS]; EQ_BANDS]; 2],
    output: f64,
    auto_compensation_db: f64,
    detector: [[Biquad; 2]; EQ_BANDS],
    dynamic_gain: [f64; EQ_BANDS],
    applied_gain: [f64; EQ_BANDS],
    attack: [f64; EQ_BANDS],
    release: [f64; EQ_BANDS],
    envelope: [f64; EQ_BANDS],
    clock: usize,
    solo_filter: [Biquad; 2],
}

impl EqProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { params: EqParams::default(), sample_rate, coeffs: [[Lattice::default(); MAX_SECTIONS]; EQ_BANDS],
            sections: [1; EQ_BANDS], state: Default::default(), output: 1.0, auto_compensation_db: 0.0,
            detector: Default::default(), dynamic_gain: [0.0; EQ_BANDS], applied_gain: [0.0; EQ_BANDS],
            attack: [0.0; EQ_BANDS], release: [0.0; EQ_BANDS], envelope: [0.0; EQ_BANDS], clock: 0, solo_filter: Default::default() };
        p.refresh();
        p
    }
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        let params = self.params.clone();
        *self = Self::new(sample_rate);
        self.params = params;
        self.refresh();
    }
    /// Signed live gain change for the native editor's dynamic-range overlay.
    pub fn dynamic_gain_db(&self, band: usize) -> f64 { self.dynamic_gain.get(band).copied().unwrap_or(0.0) }
    fn refresh_band(&mut self, i: usize) {
        let b = &self.params.bands[i];
        let dynamic = &self.params.dynamics[i];
        if !b.on || !dynamic.enabled || !b.kind.has_gain() {
            self.dynamic_gain[i] = 0.0; self.envelope[i] = 0.0;
            self.detector[i].iter_mut().for_each(Biquad::reset);
        }
        let effective = Band { gain_db: b.gain_db + self.dynamic_gain[i], ..*b };
        let (sections, n) = effective.sections(self.sample_rate);
        self.applied_gain[i] = effective.gain_db;
        for k in 0..MAX_SECTIONS { self.coeffs[i][k] = Lattice::new(sections[k]); }
        let detector_kind = match b.kind { BandKind::LowShelf => Rbj::LowPass, BandKind::HighShelf => Rbj::HighPass, _ => Rbj::BandPass };
        let detect = rbj(detector_kind, b.freq, b.q, 0.0, self.sample_rate);
        for filter in &mut self.detector[i] { filter.set(detect); }
        self.attack[i] = time_coefficient(dynamic.attack_ms, self.sample_rate);
        self.release[i] = time_coefficient(dynamic.release_ms, self.sample_rate);
        // Bypass drops the whole tail; fewer sections drop the unused ones.
        let keep = if b.on { n } else { 0 };
        for channel in &mut self.state { for k in keep..MAX_SECTIONS { channel[i][k] = LatticeState::default(); } }
        self.sections[i] = n;
    }
    fn refresh_output(&mut self) {
        // Equal-log-frequency energy matching is a static compensation, not
        // a loudness normalizer; it never follows the programme envelope.
        self.auto_compensation_db = self.params.auto_gain_db(self.sample_rate);
        self.output = db_to_lin(self.params.output_db + self.auto_compensation_db);
    }
    fn refresh_solo(&mut self) {
        if let Some(band) = self.params.solo.checked_sub(1).and_then(|i| self.params.bands.get(i)) {
            for filter in &mut self.solo_filter { filter.set(rbj(Rbj::BandPass, band.freq, band.q.max(0.5), 0.0, self.sample_rate)); }
        } else { self.solo_filter.iter_mut().for_each(Biquad::reset); }
    }
    pub fn refresh(&mut self) {
        for band in 0..EQ_BANDS { self.refresh_band(band); }
        self.refresh_output();
        self.refresh_solo();
    }
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        let (band, field) = EqParams::field_at(index).ok_or("unknown EQ parameter")?;
        self.params.set_index(index, value)?;
        if let Some(band) = band {
            self.refresh_band(band);
            if BAND_FIELDS.contains(&field) { self.refresh_output(); }
            if self.params.solo == band + 1 { self.refresh_solo(); }
        } else {
            match field {
                "output" => self.output = db_to_lin(self.params.output_db + self.auto_compensation_db),
                "auto_gain" => self.refresh_output(),
                "solo" => self.refresh_solo(),
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    #[inline]
    fn band(&mut self, b: usize, channel: usize, mut x: f64) -> f64 {
        for k in 0..self.sections[b] { x = self.state[channel][b][k].run(&self.coeffs[b][k], x); }
        x
    }

    pub fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        self.process_with_sidechain(in_l, in_r, None, out_l, out_r)
    }

    pub fn process_with_sidechain(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let on: [bool; EQ_BANDS] = std::array::from_fn(|i| self.params.bands[i].on);
        let place: [Placement; EQ_BANDS] = std::array::from_fn(|i| self.params.bands[i].place);
        // `params` is public: also honor bypass set directly before a block.
        for (b, &enabled) in on.iter().enumerate() {
            if !enabled { for channel in &mut self.state { channel[b] = Default::default(); } }
        }
        for i in 0..in_l.len() {
            let (mut l, mut r) = (in_l[i] as f64, in_r[i] as f64);
            let (key_l, key_r) = key.map_or((l, r), |(a, b)| (a[i] as f64, b[i] as f64));
            if self.params.solo > 0 && self.params.solo <= EQ_BANDS {
                let band = &self.params.bands[self.params.solo - 1];
                let (sl, sr) = match band.place {
                    Placement::Stereo => (l, r), Placement::Left => (l, l), Placement::Right => (r, r),
                    Placement::Mid => ((l + r) * 0.5, (l + r) * 0.5), Placement::Side => ((l - r) * 0.5, (l - r) * 0.5),
                };
                out_l[i] = self.solo_filter[0].run(sl) as f32;
                out_r[i] = self.solo_filter[1].run(sr) as f32;
                continue;
            }
            for b in 0..EQ_BANDS {
                if !on[b] { continue; }
                let dynamic = self.params.dynamics[b];
                if dynamic.enabled && self.params.bands[b].kind.has_gain() {
                    let (kl, kr) = match place[b] {
                        Placement::Stereo => (key_l, key_r), Placement::Left => (key_l, key_l), Placement::Right => (key_r, key_r),
                        Placement::Mid => ((key_l + key_r) * 0.5, (key_l + key_r) * 0.5),
                        Placement::Side => ((key_l - key_r) * 0.5, (key_l - key_r) * 0.5),
                    };
                    let level = self.detector[b][0].run(kl).abs().max(self.detector[b][1].run(kr).abs());
                    let coeff = if level > self.envelope[b] { self.attack[b] } else { self.release[b] };
                    self.envelope[b] = level + coeff * (self.envelope[b] - level);
                    let fraction = ((lin_to_db(self.envelope[b]) - dynamic.threshold_db) / 12.0).clamp(0.0, 1.0);
                    self.dynamic_gain[b] = dynamic.range_db * fraction;
                    let gain = self.params.bands[b].gain_db + self.dynamic_gain[b];
                    // Control-rate redesign bounds CPU cost; the normalized
                    // lattice preserves bounded state under coefficient changes.
                    if self.clock % 16 == 0 && (gain - self.applied_gain[b]).abs() > 0.01 {
                        let effective = Band { gain_db: gain, ..self.params.bands[b] };
                        let (sections, n) = effective.sections(self.sample_rate);
                        for k in 0..n { self.coeffs[b][k] = Lattice::new(sections[k]); }
                        self.applied_gain[b] = gain;
                    }
                }
                match place[b] {
                    Placement::Stereo => { l = self.band(b, 0, l); r = self.band(b, 1, r); }
                    Placement::Left => l = self.band(b, 0, l),
                    Placement::Right => r = self.band(b, 1, r),
                    Placement::Mid => { let (m, s) = ((l + r) * 0.5, (l - r) * 0.5); let m = self.band(b, 0, m); l = m + s; r = m - s; }
                    Placement::Side => { let (m, s) = ((l + r) * 0.5, (l - r) * 0.5); let s = self.band(b, 0, s); l = m + s; r = m - s; }
                }
            }
            out_l[i] = (l * self.output) as f32;
            out_r[i] = (r * self.output) as f32;
            self.clock = self.clock.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;
    use super::super::{parse_state, state_json};

    fn single_band_eq(band: Band) -> EqProcessor {
        let mut proc_ = EqProcessor::new(FS);
        for b in &mut proc_.params.bands { b.on = false; }
        proc_.params.bands[3] = band;
        proc_.refresh();
        proc_
    }
    fn bell(freq: f64, gain_db: f64, q: f64) -> Band { Band { kind: BandKind::Bell, on: true, freq, gain_db, q, slope: 0, place: Placement::Stereo } }

    #[test]
    fn flat_eq_is_transparent_and_a_bell_boosts_its_frequency() {
        let p = EqParams::default();
        for f in [30.0, 200.0, 1000.0, 8000.0] { assert!(p.response_db(f, FS).abs() < 1e-6, "flat at {f}"); }
        let mut proc_ = EqProcessor::new(FS);
        proc_.params.set("b4_gain", 9.0).unwrap();
        proc_.refresh();
        assert!((proc_.params.response_db(1500.0, FS) - 9.0).abs() < 0.05);
        let x = sine(1500.0, 0.25, 48_000);
        let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        proc_.process(&x, &x, &mut l, &mut r);
        let gain = rms_db(&l[24_000..]) - rms_db(&x[24_000..]);
        assert!((gain - 9.0).abs() < 0.1, "measured {gain}");
        assert!(proc_.params.response_db(60.0, FS).abs() < 0.2);
    }

    #[test]
    fn eq_frequency_switching_after_an_impulse_stays_bounded() {
        let mut proc_ = single_band_eq(bell(6_000.0, 24.0, 18.0));
        let freq_index = EqParams::index_of("b4_freq").unwrap();
        let (mut l, mut r) = ([0.0], [0.0]);
        proc_.process(&[1.0], &[-0.5], &mut l, &mut r);
        for sample in 0..8_192 {
            let freq = if sample % 2 == 0 { 18_000.0 } else { 6_000.0 };
            proc_.set_index(freq_index, freq).unwrap();
            proc_.process(&[0.0], &[0.0], &mut l, &mut r);
            assert!(l[0].is_finite() && r[0].is_finite(), "non-finite tail at sample {sample}");
            assert!(l[0].abs() < 32.0 && r[0].abs() < 32.0, "growing tail at sample {sample}: {l:?}, {r:?}");
            if sample == 0 { assert!(l[0].abs() > 1e-3 && r[0].abs() > 1e-3, "automation must retain the existing tail"); }
        }
        assert!(l[0].abs() < 1e-4 && r[0].abs() < 1e-4, "the switched tail must decay: {l:?}, {r:?}");
    }

    #[test]
    fn eq_automation_applies_at_the_next_sample() {
        let mut proc_ = single_band_eq(bell(6_000.0, 0.0, 18.0));
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
        for (bypass_samples, use_setter) in [(0, true), (32, true), (32, false)] {
            let mut proc_ = single_band_eq(bell(6_000.0, 24.0, 18.0));
            let on_index = EqParams::index_of("b4_on").unwrap();
            let (mut tail_l, mut tail_r) = ([0.0; 2], [0.0; 2]);
            proc_.process(&[1.0, 0.0], &[-0.5, 0.0], &mut tail_l, &mut tail_r);
            assert!(tail_l[1].abs() > 1e-3 && tail_r[1].abs() > 1e-3);
            if use_setter { proc_.set_index(on_index, 0.0).unwrap(); } else { proc_.params.bands[3].on = false; }
            let (dry_l, dry_r) = ([0.25; 32], [-0.125; 32]);
            let (mut bypass_l, mut bypass_r) = ([0.0; 32], [0.0; 32]);
            proc_.process(&dry_l[..bypass_samples], &dry_r[..bypass_samples], &mut bypass_l[..bypass_samples], &mut bypass_r[..bypass_samples]);
            assert_eq!(&bypass_l[..bypass_samples], &dry_l[..bypass_samples]);
            proc_.set_index(on_index, 1.0).unwrap();
            let (mut l, mut r) = ([0.0; 64], [0.0; 64]);
            proc_.process(&[0.0; 64], &[0.0; 64], &mut l, &mut r);
            assert_eq!(l, [0.0; 64]);
            assert_eq!(r, [0.0; 64]);
        }
    }

    #[test]
    fn every_shape_and_slope_sounds_like_its_curve() {
        let mut proc_ = single_band_eq(bell(1_500.0, 9.0, 1.0));
        let cases = [
            (BandKind::LowCut, 1_000.0, 0.0, 0.1, 0, 2_000.0),
            (BandKind::LowCut, 1_000.0, 0.0, 0.707, 3, 600.0),
            (BandKind::LowShelf, 1_000.0, 24.0, 18.0, 0, 500.0),
            (BandKind::Bell, 6_000.0, 24.0, 18.0, 0, 6_000.0),
            (BandKind::HighShelf, 10_000.0, -24.0, 0.1, 0, 16_000.0),
            (BandKind::HighCut, 6_000.0, 0.0, 18.0, 0, 3_000.0),
            (BandKind::HighCut, 2_000.0, 0.0, 0.707, 2, 3_000.0),
            (BandKind::Notch, 1_000.0, 0.0, 4.0, 0, 1_100.0),
            (BandKind::BandPass, 1_000.0, 0.0, 2.0, 0, 1_400.0),
            (BandKind::TiltShelf, 1_000.0, 12.0, 0.707, 0, 8_000.0),
        ];
        for (kind, freq, gain_db, q, slope, probe) in cases {
            proc_.params.bands[3] = Band { kind, on: true, freq, gain_db, q, slope, place: Placement::Stereo };
            proc_.refresh();
            let x = sine(probe, 0.02, 48_000);
            let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
            proc_.process(&x, &x, &mut l, &mut r);
            let expected = proc_.params.response_db(probe, FS);
            let measured = rms_db(&l[24_000..]) - rms_db(&x[24_000..]);
            assert!((measured - expected).abs() < 0.05, "{kind:?}/{slope}: measured {measured}, curve {expected}");
        }
    }

    #[test]
    fn steeper_cuts_remove_more_an_octave_below() {
        let mut band = Band { kind: BandKind::LowCut, on: true, freq: 1_000.0, gain_db: 0.0, q: 0.707, slope: 0, place: Placement::Stereo };
        let mut previous = 0.0;
        for slope in 0..4 {
            band.slope = slope;
            let at = band_response_db(&band, 500.0, FS);
            assert!((at + SLOPES[slope] as f64).abs() < 2.0, "slope {slope}: {at}");
            assert!(at < previous);
            previous = at;
        }
    }

    #[test]
    fn placement_confines_a_band_to_its_side_or_the_mid_signal() {
        let x = sine(1_000.0, 0.1, 24_000);
        let neg: Vec<f32> = x.iter().map(|v| -v).collect();
        let run = |place: Placement, l_in: &[f32], r_in: &[f32]| {
            let mut proc_ = single_band_eq(Band { place, ..bell(1_000.0, 12.0, 1.0) });
            let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
            proc_.process(l_in, r_in, &mut l, &mut r);
            (rms_db(&l[12_000..]) - rms_db(&l_in[12_000..]), rms_db(&r[12_000..]) - rms_db(&r_in[12_000..]))
        };
        let (l, r) = run(Placement::Left, &x, &x);
        assert!((l - 12.0).abs() < 0.1 && r.abs() < 0.01);
        let (l, r) = run(Placement::Right, &x, &x);
        assert!(l.abs() < 0.01 && (r - 12.0).abs() < 0.1);
        // A mono signal is all mid: a side band leaves it alone.
        let (l, r) = run(Placement::Side, &x, &x);
        assert!(l.abs() < 0.01 && r.abs() < 0.01);
        let (l, _) = run(Placement::Mid, &x, &x);
        assert!((l - 12.0).abs() < 0.1);
        // An anti-phase signal is all side.
        let (l, r) = run(Placement::Side, &x, &neg);
        assert!((l - 12.0).abs() < 0.1 && (r - 12.0).abs() < 0.1);
    }

    #[test]
    fn eq_parameters_round_trip_by_id_index_and_state() {
        let mut p = EqParams::default();
        assert_eq!(EqParams::index_of("b3_q"), Some(2 * 4 + 3));
        assert_eq!(EqParams::index_of("output"), Some(24));
        assert_eq!(EqParams::index_of("b1_shape"), Some(25));
        assert_eq!(EqParams::index_of("b6_place"), Some(25 + 17));
        assert_eq!(EqParams::index_of("b7_q"), Some(46));
        assert_eq!(EqParams::index_of("b25_q"), None);
        assert!(p.set("b2_freq", 5.0).is_err(), "below 20 Hz refused");
        assert!(p.set("b2_gain", f64::NAN).is_err());
        assert!(p.set("b2_shape", 9.0).is_err());
        p.set("b2_gain", 3.5).unwrap();
        p.set("b2_shape", BandKind::Notch.index() as f64).unwrap();
        p.set("b1_slope", 3.0).unwrap();
        p.set("b5_place", 4.0).unwrap();
        let descs = p.descs();
        assert_eq!(descs.len(), EqParams::param_count());
        assert_eq!(descs[4 + 2].value, 3.5);
        assert_eq!(descs[4 + 2].default_value, Some(0.0));
        let mut q = EqParams::default();
        for (id, v) in parse_state(&state_json(&descs)).unwrap() { q.set(&id, v).unwrap(); }
        assert_eq!(p, q);
        assert_eq!(EqParams::from_descs(&descs), p);
    }

    #[test]
    fn all_twenty_four_bands_keep_the_original_automation_indices() {
        let mut p = EqParams::default();
        assert_eq!(p.bands.iter().filter(|b| b.on).count(), 5);
        for (index, descriptor) in p.descs().iter().enumerate() {
            assert_eq!(EqParams::index_of(&descriptor.id), Some(index), "{}", descriptor.id);
            let mut restored = EqParams::default();
            restored.set_index(index, descriptor.value).unwrap();
        }
        p.set("b24_on", 1.0).unwrap(); p.set("b24_freq", 1000.0).unwrap(); p.set("b24_gain", 6.0).unwrap();
        assert!((p.response_db(1000.0, FS) - 6.0).abs() < 0.01);
        assert_eq!(p.descs()[42].id, "b6_place");
        assert_eq!(p.descs()[43].id, "b7_on");
    }

    #[test]
    fn dynamic_band_responds_to_its_frequency_and_external_key() {
        let configure = || {
            let mut p = single_band_eq(bell(1000.0, 0.0, 2.0));
            for (id, value) in [("b4_dynamic", 1.0), ("b4_dyn_range", -12.0), ("b4_dyn_threshold", -30.0), ("b4_dyn_attack", 1.0)] { p.params.set(id, value).unwrap(); }
            p.refresh(); p
        };
        let quiet = sine(1000.0, 0.001, 24000); let key = sine(1000.0, 0.5, 24000);
        let (mut l, mut r) = (vec![0.0; quiet.len()], vec![0.0; quiet.len()]);
        let mut p = configure(); p.process(&quiet, &quiet, &mut l, &mut r);
        assert!((rms_db(&l[12000..]) - rms_db(&quiet[12000..])).abs() < 0.05);
        p.process_with_sidechain(&quiet, &quiet, Some((&key, &key)), &mut l, &mut r);
        assert!(rms_db(&quiet[12000..]) - rms_db(&l[12000..]) > 10.0);
        assert!(p.dynamic_gain_db(3) < -10.0);
        let low_key = sine(40.0, 0.1, 24000); let mut p = configure();
        p.process_with_sidechain(&quiet, &quiet, Some((&low_key, &low_key)), &mut l, &mut r);
        assert!(p.dynamic_gain_db(3).abs() < 0.1, "an unrelated low frequency must not trigger this band");
    }

    #[test]
    fn allpass_is_flat_and_solo_auditions_the_band_only() {
        let allpass = Band { kind: BandKind::AllPass, ..bell(1000.0, 0.0, 1.0) };
        for f in [40.0, 300.0, 1000.0, 8000.0] { assert!(band_response_db(&allpass, f, FS).abs() < 1e-8); }
        let mut p = single_band_eq(bell(1000.0, 0.0, 4.0));
        p.set_index(EqParams::index_of("solo").unwrap(), 4.0).unwrap();
        let low = sine(100.0, 0.1, 12000); let target = sine(1000.0, 0.1, 12000);
        let (mut l, mut r) = (vec![0.0; 12000], vec![0.0; 12000]);
        p.process(&low, &low, &mut l, &mut r);
        assert!(rms_db(&low[6000..]) - rms_db(&l[6000..]) > 25.0);
        p.process(&target, &target, &mut l, &mut r);
        assert!((rms_db(&target[6000..]) - rms_db(&l[6000..])).abs() < 0.1);
    }
}
