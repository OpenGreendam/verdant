//! Six-band parametric EQ. Each band can take any of eight shapes; cuts
//! cascade up to four Butterworth sections (12–48 dB/oct); each band runs on
//! both channels, one side, or the mid/side signal.

use super::filters::{butterworth_qs, db_to_lin, response_db, rbj, Coefficients, Lattice, LatticeState, Rbj, IDENTITY};
use super::{desc, Spec};
use crate::proto::ParamDesc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind { Bell, LowShelf, LowCut, HighShelf, HighCut, Notch, BandPass, TiltShelf }

impl BandKind {
    pub const ALL: [BandKind; 8] = [Self::Bell, Self::LowShelf, Self::LowCut, Self::HighShelf, Self::HighCut, Self::Notch, Self::BandPass, Self::TiltShelf];
    pub fn index(self) -> usize { Self::ALL.iter().position(|k| *k == self).unwrap_or(0) }
    pub fn from_index(value: f64) -> Self { Self::ALL[(value.round().max(0.0) as usize).min(7)] }
    /// Cuts, notches and band-passes have no gain; their Q shapes the corner.
    pub fn has_gain(self) -> bool { matches!(self, Self::Bell | Self::LowShelf | Self::HighShelf | Self::TiltShelf) }
    pub fn has_slope(self) -> bool { matches!(self, Self::LowCut | Self::HighCut) }
    pub fn label(self) -> &'static str {
        match self {
            Self::Bell => "峰值", Self::LowShelf => "低架", Self::LowCut => "低切", Self::HighShelf => "高架",
            Self::HighCut => "高切", Self::Notch => "陷波", Self::BandPass => "带通", Self::TiltShelf => "倾斜",
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

pub const EQ_BANDS: usize = 6;
/// Up to four biquad sections per band (48 dB/oct cuts).
pub const MAX_SECTIONS: usize = 4;
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

#[derive(Clone, Debug, PartialEq)]
pub struct EqParams { pub bands: [Band; EQ_BANDS], pub output_db: f64 }

impl Default for EqParams {
    fn default() -> Self {
        let bands = EQ_LAYOUT.map(|(kind, freq, q, on)| Band { kind, on, freq, gain_db: 0.0, q, slope: 1, place: Placement::Stereo });
        Self { bands, output_db: 0.0 }
    }
}

pub fn eq_param_id(band: usize, field: &str) -> String { format!("b{}_{field}", band + 1) }

/// Index layout: `[on, freq, gain, q] × bands`, `output`, then the appended
/// `[shape, slope, place] × bands`.
const LEGACY: usize = EQ_BANDS * 4 + 1;

/// Formatting/range metadata of one EQ parameter.
pub fn eq_spec(id: &str) -> Option<Spec> {
    if id == "output" { return Some(Spec::new("output", "输出", -GAIN_RANGE, GAIN_RANGE, 0.0, "dB")); }
    let (_, field) = id.strip_prefix('b')?.split_once('_')?;
    Some(match field {
        "on" => Spec::new("on", "开关", 0.0, 1.0, 1.0, "").steps(1),
        "freq" => Spec::new("freq", "频率", FREQ_MIN, FREQ_MAX, 1000.0, "Hz").log(),
        "gain" => Spec::new("gain", "增益", -GAIN_RANGE, GAIN_RANGE, 0.0, "dB"),
        "q" => Spec::new("q", "Q", Q_MIN, Q_MAX, 1.0, "").log(),
        "shape" => Spec::new("shape", "形状", 0.0, 7.0, 0.0, "").steps(7),
        "slope" => Spec::new("slope", "陡度", 0.0, 3.0, 1.0, "").steps(3),
        "place" => Spec::new("place", "声像", 0.0, 4.0, 0.0, "").steps(4),
        _ => return None,
    })
}

impl EqParams {
    /// Parameter order is stable: the index is the automation id.
    pub fn descs(&self) -> Vec<ParamDesc> {
        let mut out = Vec::with_capacity(Self::param_count());
        for (i, b) in self.bands.iter().enumerate() {
            let n = i + 1;
            out.push(desc(eq_param_id(i, "on"), format!("频段 {n} 开关"), if b.on { 1.0 } else { 0.0 }, 0.0, 1.0, "", 1));
            out.push(desc(eq_param_id(i, "freq"), format!("频段 {n} 频率"), b.freq, FREQ_MIN, FREQ_MAX, "Hz", 0));
            out.push(desc(eq_param_id(i, "gain"), format!("频段 {n} 增益"), b.gain_db, -GAIN_RANGE, GAIN_RANGE, "dB", 0));
            out.push(desc(eq_param_id(i, "q"), format!("频段 {n} Q"), b.q, Q_MIN, Q_MAX, "", 0));
        }
        out.push(desc("output", "输出", self.output_db, -GAIN_RANGE, GAIN_RANGE, "dB", 0));
        for (i, b) in self.bands.iter().enumerate() {
            let n = i + 1;
            out.push(desc(eq_param_id(i, "shape"), format!("频段 {n} 形状"), b.kind.index() as f64, 0.0, 7.0, "", 7));
            out.push(desc(eq_param_id(i, "slope"), format!("频段 {n} 陡度"), b.slope as f64, 0.0, 3.0, "", 3));
            out.push(desc(eq_param_id(i, "place"), format!("频段 {n} 声像"), b.place.index() as f64, 0.0, 4.0, "", 4));
        }
        // Defaults are the layout's, not the current values.
        for (d, v) in out.iter_mut().zip(Self::default().values()) { d.default_value = Some(v); }
        out
    }

    pub fn values(&self) -> Vec<f64> {
        let mut v = Vec::with_capacity(Self::param_count());
        for b in &self.bands { v.extend([if b.on { 1.0 } else { 0.0 }, b.freq, b.gain_db, b.q]); }
        v.push(self.output_db);
        for b in &self.bands { v.extend([b.kind.index() as f64, b.slope as f64, b.place.index() as f64]); }
        v
    }

    pub fn param_count() -> usize { LEGACY + EQ_BANDS * 3 }

    pub fn index_of(id: &str) -> Option<usize> {
        if id == "output" { return Some(EQ_BANDS * 4); }
        let (n, field) = id.strip_prefix('b')?.split_once('_')?;
        let band: usize = n.parse().ok()?;
        if band == 0 || band > EQ_BANDS { return None; }
        let b = band - 1;
        Some(match field {
            "on" => b * 4, "freq" => b * 4 + 1, "gain" => b * 4 + 2, "q" => b * 4 + 3,
            "shape" => LEGACY + b * 3, "slope" => LEGACY + b * 3 + 1, "place" => LEGACY + b * 3 + 2,
            _ => return None,
        })
    }

    /// Set by stable index; values outside the parameter's range are refused.
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        if !value.is_finite() { return Err("non-finite EQ parameter".into()); }
        if index == EQ_BANDS * 4 {
            if !(-GAIN_RANGE..=GAIN_RANGE).contains(&value) { return Err("EQ output outside ±24 dB".into()); }
            self.output_db = value;
            return Ok(());
        }
        if index >= LEGACY {
            let band = self.bands.get_mut((index - LEGACY) / 3).ok_or("unknown EQ parameter")?;
            match (index - LEGACY) % 3 {
                0 if (0.0..=7.0).contains(&value) => band.kind = BandKind::from_index(value),
                1 if (0.0..=3.0).contains(&value) => band.slope = value.round() as usize,
                2 if (0.0..=4.0).contains(&value) => band.place = Placement::from_index(value),
                _ => return Err("EQ parameter outside its range".into()),
            }
            return Ok(());
        }
        let band = self.bands.get_mut(index / 4).ok_or("unknown EQ parameter")?;
        match index % 4 {
            0 if (0.0..=1.0).contains(&value) => band.on = value >= 0.5,
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
}

impl EqProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { params: EqParams::default(), sample_rate, coeffs: [[Lattice::default(); MAX_SECTIONS]; EQ_BANDS],
            sections: [1; EQ_BANDS], state: Default::default(), output: 1.0 };
        p.refresh();
        p
    }
    pub fn set_sample_rate(&mut self, sample_rate: f64) { self.sample_rate = sample_rate; self.state = Default::default(); self.refresh(); }
    pub fn refresh(&mut self) {
        for (i, b) in self.params.bands.iter().enumerate() {
            let (sections, n) = b.sections(self.sample_rate);
            for k in 0..MAX_SECTIONS { self.coeffs[i][k] = Lattice::new(sections[k]); }
            // Bypass drops the whole tail; fewer sections drop the unused ones.
            let keep = if b.on { n } else { 0 };
            for channel in &mut self.state { for k in keep..MAX_SECTIONS { channel[i][k] = LatticeState::default(); } }
            self.sections[i] = n;
        }
        self.output = db_to_lin(self.params.output_db);
    }
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        self.params.set_index(index, value)?;
        self.refresh();
        Ok(())
    }

    #[inline]
    fn band(&mut self, b: usize, channel: usize, mut x: f64) -> f64 {
        for k in 0..self.sections[b] { x = self.state[channel][b][k].run(&self.coeffs[b][k], x); }
        x
    }

    pub fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        let on: [bool; EQ_BANDS] = std::array::from_fn(|i| self.params.bands[i].on);
        let place: [Placement; EQ_BANDS] = std::array::from_fn(|i| self.params.bands[i].place);
        // `params` is public: also honor bypass set directly before a block.
        for (b, &enabled) in on.iter().enumerate() {
            if !enabled { for channel in &mut self.state { channel[b] = Default::default(); } }
        }
        for i in 0..in_l.len() {
            let (mut l, mut r) = (in_l[i] as f64, in_r[i] as f64);
            for b in 0..EQ_BANDS {
                if !on[b] { continue; }
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
        assert_eq!(EqParams::index_of("b7_q"), None);
        assert!(p.set("b2_freq", 5.0).is_err(), "below 20 Hz refused");
        assert!(p.set("b2_gain", f64::NAN).is_err());
        assert!(p.set("b2_shape", 8.0).is_err());
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
}
