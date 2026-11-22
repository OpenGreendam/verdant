//! Twenty-four-band parametric, dynamic and spectral EQ. Ten shapes support
//! legacy IIR slopes and an explicit continuous-slope FIR mode. Stereo transfer
//! matrices preserve the order of Left/Right/Mid/Side bands.

use super::filters::{db_to_lin, lin_to_db, response_db, rbj, time_coefficient, Biquad, DelayLine, Coefficients, Lattice, LatticeState, Rbj, IDENTITY};
use super::{desc, Spec};
use crate::proto::ParamDesc;
use super::spectral::{Complex, PhaseFir, SpectralFrame, SPECTRAL_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandKind { Bell, LowShelf, LowCut, HighShelf, HighCut, Notch, BandPass, TiltShelf, AllPass, FlatTilt }

impl BandKind {
    pub const ALL: [BandKind; 10] = [Self::Bell, Self::LowShelf, Self::LowCut, Self::HighShelf, Self::HighCut, Self::Notch, Self::BandPass, Self::TiltShelf, Self::AllPass, Self::FlatTilt];
    pub fn index(self) -> usize { Self::ALL.iter().position(|k| *k == self).unwrap_or(0) }
    pub fn from_index(value: f64) -> Self { Self::ALL[(value.round().max(0.0) as usize).min(9)] }
    /// Cuts, notches and band-passes have no gain; their Q shapes the corner.
    pub fn has_gain(self) -> bool { matches!(self, Self::Bell | Self::LowShelf | Self::HighShelf | Self::TiltShelf | Self::FlatTilt) }
    /// Legacy discrete slope control; the continuous control supports all shapes.
    pub fn has_slope(self) -> bool { matches!(self, Self::LowCut | Self::HighCut) }
    pub fn min_slope(self)->f64 {match self{
        Self::Bell|Self::Notch=>12.0,Self::LowCut|Self::HighCut|Self::BandPass=>0.0,_=>6.0
    }}
    pub fn label(self) -> &'static str {
        match self {
            Self::Bell => "峰值", Self::LowShelf => "低架", Self::LowCut => "低切", Self::HighShelf => "高架",
            Self::HighCut => "高切", Self::Notch => "陷波", Self::BandPass => "带通", Self::TiltShelf => "倾斜", Self::AllPass => "全通", Self::FlatTilt=>"平直倾斜",
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
/// Up to eight biquad sections per band (96 dB/oct cuts).
pub const MAX_SECTIONS: usize = 8;
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
pub const SLOPES: [u32; 11] = [12, 24, 36, 48, 6, 18, 30, 60, 72, 84, 96];

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
            BandKind::FlatTilt => {
                // Eight logarithmically spaced shelves approximate a straight
                // dB/log-frequency tilt. Normalize at the chosen pivot rather
                // than treating it as the corner of one shelving section.
                for (i,section) in out.iter_mut().enumerate() {
                    let corner=20.0*1000.0_f64.powf((i as f64+0.5)/MAX_SECTIONS as f64);
                    *section=rbj(Rbj::HighShelf,corner,0.5,g/MAX_SECTIONS as f64,sample_rate);
                }
                let pivot: f64=out.iter().map(|&c|response_db(c,f,sample_rate)).sum();
                let normalization=db_to_lin(-pivot);for coefficient in &mut out[0][..3]{*coefficient*=normalization;}
                MAX_SECTIONS
            }
            BandKind::LowCut | BandKind::HighCut => {
                let kind = if self.kind == BandKind::LowCut { Rbj::HighPass } else { Rbj::LowPass };
                let order = (SLOPES[self.slope.min(SLOPES.len()-1)] / 6) as usize;
                let resonance = q / std::f64::consts::FRAC_1_SQRT_2;
                let first_order = order % 2;
                if first_order == 1 {
                    let a = (std::f64::consts::PI * f.clamp(1.0, sample_rate*0.499) / sample_rate).tan();
                    let d = 1.0 + a;
                    out[0] = if self.kind == BandKind::LowCut { [1.0/d,-1.0/d,0.0,(a-1.0)/d,0.0] }
                        else { [a/d,a/d,0.0,(a-1.0)/d,0.0] };
                }
                for i in 0..order/2 {
                    let bq = 1.0/(2.0*(std::f64::consts::PI*(2*i+1) as f64/(2*order) as f64).sin());
                    out[first_order+i] = rbj(kind,f,if i==0 {bq*resonance}else{bq},0.0,sample_rate);
                }
                order.div_ceil(2)
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
    pub phase: usize,
    pub resolution: usize,
    pub spectral: [bool; EQ_BANDS],
    pub density: [f64; EQ_BANDS],
    pub spectral_tilt: [bool; EQ_BANDS],
    pub brickwall: [bool; EQ_BANDS],
    /// Explicit fixed-latency continuous-slope mode, appended to the old ABI.
    pub slope_continuous: [bool; EQ_BANDS],
    pub slope_db: [f64; EQ_BANDS],
}

impl Default for EqParams {
    fn default() -> Self {
        let bands = EQ_LAYOUT.map(|(kind, freq, q, on)| Band { kind, on, freq, gain_db: 0.0, q, slope: 1, place: Placement::Stereo });
        Self { bands, output_db: 0.0, dynamics: [DynamicBand::default(); EQ_BANDS], solo: 0, auto_gain: false, phase: 0, resolution: 1, spectral: [false; EQ_BANDS], density: [50.0; EQ_BANDS], spectral_tilt: [false; EQ_BANDS], brickwall: [false; EQ_BANDS], slope_continuous: [false;EQ_BANDS], slope_db:[12.0;EQ_BANDS] }
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
const ADVANCED_START: usize = GLOBAL_START + 4;
const ADVANCED_FIELDS: [&str; 4] = ["spectral", "density", "spectral_tilt", "brickwall"];
const CONTINUOUS_START:usize=ADVANCED_START+EQ_BANDS*4;
const CONTINUOUS_FIELDS:[&str;2]=["slope_continuous","slope_db"];
const BAND_FIELDS: [&str; 7] = ["on", "freq", "gain", "q", "shape", "slope", "place"];
const DYNAMIC_FIELDS: [&str; 5] = ["dynamic", "dyn_range", "dyn_threshold", "dyn_attack", "dyn_release"];

/// Formatting/range metadata of one EQ parameter.
pub fn eq_spec(id: &str) -> Option<Spec> {
    if matches!(id, "output" | "solo" | "auto_gain" | "phase" | "resolution") { return eq_field_spec(id); }
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
        "phase" => Spec::new("phase", "相位模式", 0.0, 2.0, 0.0, "").steps(2).fixed(),
        "resolution" => Spec::new("resolution", "线性相位精度", 0.0, 2.0, 1.0, "").steps(2).fixed(),
        "spectral" => Spec::new("spectral", "频谱动态", 0.0, 1.0, 0.0, "").steps(1).fixed(),
        "density" => Spec::new("density", "频谱密度", 0.0, 100.0, 50.0, "%"),
        "spectral_tilt" => Spec::new("spectral_tilt", "频谱倾斜", 0.0, 1.0, 0.0, "").steps(1),
        "brickwall" => Spec::new("brickwall", "砖墙切频", 0.0, 1.0, 0.0, "").steps(1).fixed(),
        "slope_continuous"=>Spec::new("slope_continuous","连续陡度（FIR）",0.0,1.0,0.0,"").steps(1).fixed(),
        "slope_db"=>Spec::new("slope_db","连续陡度",0.0,96.0,12.0,"dB/oct"),
        "on" => Spec::new("on", "开关", 0.0, 1.0, 1.0, "").steps(1),
        "freq" => Spec::new("freq", "频率", FREQ_MIN, FREQ_MAX, 1000.0, "Hz").log(),
        "gain" => Spec::new("gain", "增益", -GAIN_RANGE, GAIN_RANGE, 0.0, "dB"),
        "q" => Spec::new("q", "Q", Q_MIN, Q_MAX, 1.0, "").log(),
        "shape" => Spec::new("shape", "形状", 0.0, 9.0, 0.0, "").steps(9),
        "slope" => Spec::new("slope", "陡度", 0.0, (SLOPES.len()-1) as f64, 1.0, "").steps((SLOPES.len()-1) as i64),
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
        if index < ADVANCED_START { return match index-GLOBAL_START { 0=>Some((None,"solo")),1=>Some((None,"auto_gain")),2=>Some((None,"phase")),3=>Some((None,"resolution")),_=>None }; }
        if index<CONTINUOUS_START {let k=index-ADVANCED_START;return Some((Some(k/4),ADVANCED_FIELDS[k%4]));}
        let k=index-CONTINUOUS_START;
        (k<EQ_BANDS*2).then(||(Some(k/2),CONTINUOUS_FIELDS[k%2]))
    }

    fn value_at(&self, index: usize) -> f64 {
        let Some((band, field)) = Self::field_at(index) else { return 0.0; };
        let Some(i) = band else { return match field { "output" => self.output_db, "solo" => self.solo as f64, "phase" => self.phase as f64, "resolution" => self.resolution as f64, _ => if self.auto_gain { 1.0 } else { 0.0 } }; };
        let b = &self.bands[i];
        let d = &self.dynamics[i];
        match field {
            "on" => if b.on { 1.0 } else { 0.0 }, "freq" => b.freq, "gain" => b.gain_db, "q" => b.q,
            "shape" => b.kind.index() as f64, "slope" => b.slope as f64, "place" => b.place.index() as f64,
            "dynamic" => if d.enabled { 1.0 } else { 0.0 }, "dyn_range" => d.range_db, "dyn_threshold" => d.threshold_db,
            "dyn_attack" => d.attack_ms, "dyn_release" => d.release_ms, "spectral" => self.spectral[i] as u8 as f64, "density" => self.density[i], "spectral_tilt" => self.spectral_tilt[i] as u8 as f64, "brickwall" => self.brickwall[i] as u8 as f64, "slope_continuous"=>self.slope_continuous[i] as u8 as f64,"slope_db"=>self.slope_db[i], _ => unreachable!(),
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

    pub fn param_count() -> usize { CONTINUOUS_START + EQ_BANDS*2 }

    pub fn index_of(id: &str) -> Option<usize> {
        match id { "output" => return Some(24), "solo" => return Some(GLOBAL_START), "auto_gain" => return Some(GLOBAL_START + 1), "phase" => return Some(GLOBAL_START+2), "resolution" => return Some(GLOBAL_START+3), _ => {} }
        let (n, field) = id.strip_prefix('b')?.split_once('_')?;
        let band: usize = n.parse().ok()?;
        if band == 0 || band > EQ_BANDS { return None; }
        let b = band - 1;
        if let Some(f)=CONTINUOUS_FIELDS.iter().position(|s|*s==field){return Some(CONTINUOUS_START+b*2+f);}
        if let Some(f) = ADVANCED_FIELDS.iter().position(|s|*s==field) { return Some(ADVANCED_START+b*4+f); }
        if let Some(f) = DYNAMIC_FIELDS.iter().position(|s| *s == field) { return Some(DYNAMIC_START + b * 5 + f); }
        let f = BAND_FIELDS.iter().position(|s| *s == field)?;
        Some(if b < ORIGINAL_BANDS { if f < 4 { b * 4 + f } else { LEGACY + b * 3 + f - 4 } } else { ORIGINAL_COUNT + (b - ORIGINAL_BANDS) * 7 + f })
    }

    /// Set by stable index; values outside the parameter's range are refused.
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        let (band, field) = Self::field_at(index).ok_or("unknown EQ parameter")?;
        eq_field_spec(field).ok_or("unknown EQ parameter")?.check(value)?;
        let Some(i) = band else {
            match field { "output" => self.output_db = value, "solo" => self.solo = value.round() as usize, "auto_gain" => self.auto_gain = value >= 0.5, "phase" => self.phase=value.round() as usize, "resolution" => self.resolution=value.round() as usize, _ => unreachable!() }
            return Ok(());
        };
        let b = &mut self.bands[i];
        let d = &mut self.dynamics[i];
        match field {
            "on" => b.on = value >= 0.5, "freq" => b.freq = value, "gain" => b.gain_db = value, "q" => b.q = value,
            "shape" => b.kind = BandKind::from_index(value), "slope" => b.slope = value.round() as usize, "place" => b.place = Placement::from_index(value),
            "dynamic" => d.enabled = value >= 0.5, "dyn_range" => d.range_db = value, "dyn_threshold" => d.threshold_db = value,
            "dyn_attack" => d.attack_ms = value, "dyn_release" => d.release_ms = value, "spectral" => self.spectral[i]=value>=0.5, "density" => self.density[i]=value, "spectral_tilt" => self.spectral_tilt[i]=value>=0.5, "brickwall" => self.brickwall[i]=value>=0.5, "slope_continuous"=>self.slope_continuous[i]=value>=0.5,"slope_db"=>self.slope_db[i]=value, _ => unreachable!(),
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

    /// FIR mode has one fixed common delay, including when a band is bypassed.
    pub fn uses_fir(&self)->bool {self.phase==2||self.brickwall.iter().any(|x|*x)||self.slope_continuous.iter().any(|x|*x)}
    pub fn effective_slope(&self,index:usize)->f64{
        self.slope_db[index].clamp(self.bands[index].kind.min_slope(),96.0)
    }
    /// Includes the band's own continuous slope and brickwall configuration.
    pub fn band_response_db(&self,index:usize,frequency:f64,sample_rate:f64)->f64{
        let band=&self.bands[index];
        if self.brickwall[index]&&band.kind.has_slope(){
            return if (band.kind==BandKind::LowCut&&frequency<band.freq)||(band.kind==BandKind::HighCut&&frequency>band.freq){-180.0}else{0.0};
        }
        let fs=sample_rate*if self.phase==1&&!self.uses_fir(){2.0}else{1.0};
        band_response_db_with_slope(band,frequency,fs,self.slope_continuous[index].then_some(self.effective_slope(index)))
    }
    /// Combined display response, including the actual continuous target.
    pub fn response_db(&self,frequency:f64,sample_rate:f64)->f64 {
        self.output_db+(0..EQ_BANDS).filter(|&i|self.bands[i].on).map(|i|self.band_response_db(i,frequency,sample_rate)).sum::<f64>()
    }

    /// Equal-log-frequency static energy compensation; no programme follower.
    pub fn auto_gain_db(&self,sample_rate:f64)->f64{
        if !self.auto_gain{return 0.0;}
        let fs=sample_rate*if self.phase==1&&!self.uses_fir(){2.0}else{1.0};
        let filters=self.bands.map(|band|band.sections(fs));
        let energy=(0..48).map(|probe|{
            let frequency=FREQ_MIN*(FREQ_MAX/FREQ_MIN).powf((probe as f64+0.5)/48.0);
            let db=(0..EQ_BANDS).filter(|&i|self.bands[i].on).map(|i|{
                if self.slope_continuous[i]||self.brickwall[i]{self.band_response_db(i,frequency,sample_rate)}
                else {filters[i].0[..filters[i].1].iter().map(|&c|response_db(c,frequency,fs)).sum::<f64>()}
            }).sum::<f64>();
            10.0_f64.powf(db.clamp(-180.0,180.0)/10.0)
        }).sum::<f64>()/48.0;
        (-10.0*energy.max(1e-12).log10()).clamp(-24.0,24.0)
    }
}

/// The first biquad section of a band.
pub fn band_coefficients(band: &Band, sample_rate: f64) -> Coefficients { band.sections(sample_rate).0[0] }

/// The band's response in dB (all sections).
pub fn band_response_db(band: &Band, freq: f64, sample_rate: f64) -> f64 {
    let (sections, n) = band.sections(sample_rate);
    sections[..n].iter().map(|&c| response_db(c, freq, sample_rate)).sum()
}

/// Curve target used by both the editor and FIR renderer. Cut roll-off is a
/// continuous Butterworth order, with no rounding to an integer cascade.
/// Shelves and resonant shapes use the same continuous transition exponent.
pub fn band_response_db_with_slope(band:&Band,frequency:f64,sample_rate:f64,slope:Option<f64>)->f64{
    let Some(slope)=slope else{return band_response_db(band,frequency,sample_rate);};
    let slope=slope.clamp(band.kind.min_slope(),96.0);
    let corner=band.freq.clamp(1.0,sample_rate*0.499);
    let ratio=frequency.clamp(1e-9,sample_rate*0.5)/corner;
    let log_ratio=ratio.ln();
    let order=slope/(20.0*2.0_f64.log10());
    let q=band.q.clamp(Q_MIN,Q_MAX);
    let db_per_ln=10.0/std::f64::consts::LN_10;
    let transition=|log_x:f64|softplus(log_x);
    let db=match band.kind{
        BandKind::LowCut|BandKind::HighCut=>{
            if slope==0.0{return 0.0;}
            let log_x=if band.kind==BandKind::LowCut{-log_ratio}else{log_ratio};
            // Normalize in the finite pass band. At order zero this tends
            // continuously to unity instead of a discontinuous -3 dB shelf.
            let attenuation=(transition(2.0*order*log_x)-transition(-2.0*order*64.0_f64.ln())).max(0.0);
            let resonance=(20.0*(q/std::f64::consts::FRAC_1_SQRT_2).log10()).clamp(-12.0,24.0)
                *(order-1.0).clamp(0.0,1.0)*(-0.5*(log_ratio*q.max(0.5)*2.0).powi(2)).exp();
            -db_per_ln*attenuation+resonance
        }
        BandKind::Bell=>{
            let x=(q*(ratio-ratio.recip())).abs();
            band.gain_db/(1.0+x.powf(order))
        }
        BandKind::Notch=>{
            let x=(q*(ratio-ratio.recip())).abs().max(1e-30);
            let power=2.0*order*x.ln();
            db_per_ln*(power-transition(power))
        }
        BandKind::BandPass=>{
            if slope==0.0{return 0.0;}
            let x=(q*(ratio-ratio.recip())).abs().max(1e-30);
            -db_per_ln*(transition(2.0*order*x.ln())-transition(2.0*order*0.001_f64.ln())).max(0.0)
        }
        BandKind::LowShelf|BandKind::HighShelf|BandKind::TiltShelf=>{
            let q_factor=1.0+((q/std::f64::consts::FRAC_1_SQRT_2).sqrt().clamp(0.25,4.0)-1.0)*((slope-6.0)/6.0).clamp(0.0,1.0);
            let low=(-softplus(2.0*order*q_factor*log_ratio)).exp();
            band.gain_db*match band.kind{BandKind::LowShelf=>low,BandKind::HighShelf=>1.0-low,_=>0.5-low}
        }
        BandKind::FlatTilt=>band.gain_db*log_ratio/1000.0_f64.ln()*(slope/12.0).sqrt(),
        BandKind::AllPass=>0.0,
    };
    db.clamp(-180.0,96.0)
}
#[inline]fn softplus(x:f64)->f64 {if x>40.0{x}else if x< -40.0{x.exp()}else{x.exp().ln_1p()}}

/// Fractional all-pass phase with unit magnitude. A short Nyquist transition
/// ends on the real axis, as required by a real-valued discrete-time kernel.
fn continuous_allpass(band:&Band,frequency:f64,sample_rate:f64,slope:f64)->Complex{
    let slope=slope.clamp(6.0,96.0);let order=slope/6.0;
    let q=0.5+(band.q.clamp(Q_MIN,Q_MAX)-0.5)*((slope-6.0)/6.0).clamp(0.0,1.0);
    let nyquist=(2.0*frequency/sample_rate).clamp(0.0,1.0);
    let denominator=(std::f64::consts::PI*band.freq.clamp(1.0,sample_rate*0.499)/sample_rate).tan();
    let ratio=(std::f64::consts::PI*0.5*nyquist.min(1.0-1e-12)).tan()/denominator;
    let mut phase= -order*(ratio/q).atan2(1.0-ratio*ratio);
    if nyquist>0.9{
        let t=(nyquist-0.9)*10.0;let t=t*t*(3.0-2.0*t);
        let endpoint= -order.round()*std::f64::consts::PI;
        phase+=(endpoint-phase)*t;
    }
    Complex::new(phase.cos(),phase.sin())
}
fn biquad_transfer([b0,b1,b2,a1,a2]:Coefficients,cos:f64,sin:f64,cos2:f64,sin2:f64)->Complex{
    let(nr,ni)=(b0+b1*cos+b2*cos2,-b1*sin-b2*sin2);
    let(dr,di)=(1.0+a1*cos+a2*cos2,-a1*sin-a2*sin2);let energy=(dr*dr+di*di).max(1e-24);
    Complex::new((nr*dr+ni*di)/energy,(ni*dr-nr*di)/energy)
}

/// Stereo EQ processor. Coefficients are recomputed only when a parameter
/// changes. IIR changes apply from the next sample; FIR targets are snapshotted
/// and designed in bounded 32-frame work steps, then published on a convolution
/// boundary. Bypassing an IIR band discards its state.
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
    fir: PhaseFir,
    processing_started:bool,
    fir_clock:usize,
    fir_dirty:bool,
    fir_target:EqParams,
    fir_filters:[([Coefficients;MAX_SECTIONS],usize);EQ_BANDS],
    spectral_frame: SpectralFrame,
    spectral_envelopes: Vec<f64>,
    spectral_weights: Vec<f64>,
    spectral_weight_signature:[[f64;5];EQ_BANDS],
    spectral_attacks: [f64; EQ_BANDS],
    spectral_releases: [f64; EQ_BANDS],
    linear_dynamic_gain:[f64;EQ_BANDS],
    natural_input: [HalfBand; 2],
    natural_output: [HalfBand; 2],
    key_alignment:[DelayLine;2],

}

impl EqProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut p = Self { params: EqParams::default(), sample_rate, coeffs: [[Lattice::default(); MAX_SECTIONS]; EQ_BANDS],
            sections: [1; EQ_BANDS], state: Default::default(), output: 1.0, auto_compensation_db: 0.0,
            detector: Default::default(), dynamic_gain: [0.0; EQ_BANDS], applied_gain: [0.0; EQ_BANDS],
            attack: [0.0; EQ_BANDS], release: [0.0; EQ_BANDS], envelope: [0.0; EQ_BANDS], clock: 0, solo_filter: Default::default(), fir: PhaseFir::new(), processing_started:false,fir_clock:0,fir_dirty:false,
            fir_target:EqParams::default(),fir_filters:[([IDENTITY;MAX_SECTIONS],1);EQ_BANDS],spectral_frame: SpectralFrame::new(),
            spectral_envelopes: vec![0.0; EQ_BANDS*(SPECTRAL_SIZE/2+1)],
            spectral_weights: vec![0.0; EQ_BANDS*(SPECTRAL_SIZE/2+1)],spectral_weight_signature:[[f64::NAN;5];EQ_BANDS],
            spectral_attacks: [0.0;EQ_BANDS], spectral_releases:[0.0;EQ_BANDS],linear_dynamic_gain:[0.0;EQ_BANDS],
            natural_input: Default::default(), natural_output: Default::default(),key_alignment:std::array::from_fn(|_|DelayLine::new(4096)) };
        p.refresh();
        p
    }
    /// The host calls this after event validation, before frame-zero automation.
    /// Parameters installed before this point are synchronous static setup.
    pub fn begin_audio(&mut self){self.processing_started=true;}
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        let params = self.params.clone();
        *self = Self::new(sample_rate);
        self.params = params;
        self.refresh();
    }
    /// Signed live gain change for the native editor's dynamic-range overlay.
    pub fn dynamic_gain_db(&self, band: usize) -> f64 {
        if band>=EQ_BANDS{return 0.0;}
        if self.params.spectral[band] {
            let values=&self.spectral_envelopes[band*(SPECTRAL_SIZE/2+1)..(band+1)*(SPECTRAL_SIZE/2+1)];
            if self.params.dynamics[band].range_db<0.0{values.iter().copied().fold(0.0,f64::min)}else{values.iter().copied().fold(0.0,f64::max)}
        }else if self.linear_enabled(){self.linear_dynamic_gain[band]}else{self.dynamic_gain[band]}
    }
    fn refresh_band(&mut self, i: usize) {
        let b = &self.params.bands[i];
        let dynamic = &self.params.dynamics[i];
        if !b.on || !dynamic.enabled || !b.kind.has_gain() || self.params.spectral[i] {
            self.dynamic_gain[i] = 0.0; self.envelope[i] = 0.0;
            self.detector[i].iter_mut().for_each(Biquad::reset);
        }
        let effective = Band { gain_db: b.gain_db + self.dynamic_gain[i], ..*b };
        let (sections, n) = effective.sections(self.dsp_sample_rate());
        self.applied_gain[i] = effective.gain_db;
        for k in 0..MAX_SECTIONS { self.coeffs[i][k] = Lattice::new(sections[k]); }
        let detector_kind = match b.kind { BandKind::LowShelf => Rbj::LowPass, BandKind::HighShelf => Rbj::HighPass, _ => Rbj::BandPass };
        let detect = rbj(detector_kind, b.freq, b.q, 0.0, self.dsp_sample_rate());
        for filter in &mut self.detector[i] { filter.set(detect); }
        self.attack[i] = time_coefficient(dynamic.attack_ms, self.dsp_sample_rate());
        self.release[i] = time_coefficient(dynamic.release_ms, self.dsp_sample_rate());
        // Bypass drops the whole tail; fewer sections drop the unused ones.
        let keep = if b.on { n } else { 0 };
        for channel in &mut self.state { for k in keep..MAX_SECTIONS { channel[i][k] = LatticeState::default(); } }
        self.sections[i] = n;
        let frames_per_second=self.sample_rate/(SPECTRAL_SIZE/4) as f64;
        self.spectral_attacks[i]=time_coefficient(dynamic.attack_ms,frames_per_second);
        self.spectral_releases[i]=time_coefficient(dynamic.release_ms,frames_per_second);
        let signature=[b.freq,b.q,b.kind.index() as f64,self.params.slope_continuous[i] as u8 as f64,self.params.slope_db[i]];
        if dynamic.enabled && self.spectral_weight_signature[i]!=signature {
        self.spectral_weight_signature[i]=signature;
        for bin in 0..=SPECTRAL_SIZE/2 {
            let f=(bin as f64*self.sample_rate/SPECTRAL_SIZE as f64).max(1.0);
            let width=(1.0/b.q.max(0.1)).max(0.02);
            self.spectral_weights[i*(SPECTRAL_SIZE/2+1)+bin]=if self.params.slope_continuous[i]&&b.kind.has_gain(){
                band_response_db_with_slope(&Band{gain_db:1.0,..*b},f,self.sample_rate,Some(self.params.effective_slope(i)))
            }else{match b.kind {
                BandKind::LowShelf=>(1.0+(f/b.freq).powf(4.0)).recip(),
                BandKind::HighShelf=>(1.0+(b.freq/f).powf(4.0)).recip(),
                _=>(-0.5*((f/b.freq).ln()/width).powi(2)).exp(),
            }};
        }
        }
    }
    fn refresh_output(&mut self) {
        // Equal-log-frequency energy matching is a static compensation, not
        // a loudness normalizer; it never follows the programme envelope.
        self.auto_compensation_db = self.params.auto_gain_db(self.sample_rate);
        self.output = db_to_lin(self.params.output_db + self.auto_compensation_db);
    }
    fn refresh_solo(&mut self) {
        if let Some(band) = self.params.solo.checked_sub(1).and_then(|i| self.params.bands.get(i)) {
            let fs=self.dsp_sample_rate();
            for filter in &mut self.solo_filter { filter.set(rbj(Rbj::BandPass, band.freq, band.q.max(0.5), 0.0, fs)); }
        } else { self.solo_filter.iter_mut().for_each(Biquad::reset); }
    }
    pub fn refresh(&mut self) {
        for band in 0..EQ_BANDS { self.refresh_band(band); }
        self.refresh_output();
        self.refresh_solo();
        self.refresh_phase();
    }
    pub fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        let (band, field) = EqParams::field_at(index).ok_or("unknown EQ parameter")?;
        let old_latency=self.latency();let old_rate=self.dsp_sample_rate();
        let previous=self.params.value_at(index);
        self.params.set_index(index, value)?;
        if self.params.value_at(index)==previous{return Ok(());}
        if let Some(band) = band {
            self.refresh_band(band);
            if BAND_FIELDS.contains(&field)||CONTINUOUS_FIELDS.contains(&field)||field=="brickwall" { self.refresh_output(); }
            if self.params.solo == band + 1 { self.refresh_solo(); }
        } else {
            match field {
                "output" => self.output = db_to_lin(self.params.output_db + self.auto_compensation_db),
                "auto_gain" => self.refresh_output(),
                "solo" => self.refresh_solo(),
                "phase" => { for band in 0..EQ_BANDS { self.refresh_band(band); } },
                "resolution" => {},
                _ => unreachable!(),
            }
        }
        // Switching the FIR reservation also changes Natural mode's 2x
        // coefficient rate. Refresh every band, not only the edited one.
        if old_rate!=self.dsp_sample_rate(){for i in 0..EQ_BANDS{self.refresh_band(i);}self.refresh_output();self.refresh_solo();}
        if matches!(field,"phase"|"resolution"|"brickwall"|"slope_continuous"){
            // Fixed controls are admitted outside processing: the declared
            // latency and active kernels change together, never asynchronously.
            self.refresh_phase();
        }else if self.linear_enabled()&&(BAND_FIELDS.contains(&field)||matches!(field,"slope_db"|"solo")){
            if self.processing_started{self.fir_dirty=true;}else{self.refresh_phase();}
        }
        if old_latency!=self.latency() {
            self.fir.reset(); self.spectral_frame.reset();
            self.natural_input=Default::default();self.natural_output=Default::default();
            self.spectral_envelopes.fill(0.0);self.linear_dynamic_gain.fill(0.0);
            for delay in &mut self.key_alignment{delay.clear();}
        }
        Ok(())
    }

    fn linear_enabled(&self)->bool { self.params.uses_fir() }
    fn spectral_enabled(&self)->bool { self.params.spectral.iter().any(|x|*x) || self.linear_enabled() }
    fn dsp_sample_rate(&self)->f64 { self.sample_rate*if self.params.phase==1 && !self.linear_enabled(){2.0}else{1.0} }
    pub fn latency(&self)->u32 {
        (if self.linear_enabled(){[384,1152,2176][self.params.resolution.min(2)]}else if self.params.phase==1{16}else{0})
            + if self.spectral_enabled(){SPECTRAL_SIZE as u32}else{0}
    }
    fn refresh_phase(&mut self) {
        self.fir_dirty=false;self.fir.cancel_design();
        if !self.linear_enabled(){return;}
        let params=&self.params;let fs=self.sample_rate;
        let filters=params.bands.map(|b|b.sections(fs));
        self.fir.design_complex(fs,params.resolution,|frequency|phase_response(params,&filters,fs,frequency));
    }

    fn update_phase(&mut self){
        self.fir.commit_ready();
        if self.fir_clock%32!=0{return;}
        if self.fir.design_idle(){
            if !self.fir_dirty{return;}
            // EqParams owns only fixed arrays. The complete snapshot and all
            // work buffers stay allocated for the processor's lifetime.
            self.fir_target=self.params.clone();
            self.fir_filters=self.fir_target.bands.map(|b|b.sections(self.sample_rate));
            self.fir.begin_design(self.fir_target.resolution);self.fir_dirty=false;
        }
        let params=&self.fir_target;let filters=&self.fir_filters;let fs=self.sample_rate;
        self.fir.design_step(fs,64,|frequency|phase_response(params,filters,fs,frequency));
        self.fir.commit_ready();
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
        for b in 0..EQ_BANDS { if !self.params.bands[b].on { for channel in &mut self.state {channel[b]=Default::default();} } }
        let linear=self.linear_enabled();let spectral=self.spectral_enabled();
        if !in_l.is_empty(){self.begin_audio();}
        for i in 0..in_l.len() {
            if linear{self.update_phase();}
            let input=[in_l[i] as f64,in_r[i] as f64];
            let key=key.map_or(input,|(l,r)|[l[i] as f64,r[i] as f64]);
            let mut audio=if linear { self.fir.run(input) } else if self.params.phase==1 {
                let first=[self.natural_input[0].run(input[0]*2.0),self.natural_input[1].run(input[1]*2.0)];
                let first=self.process_sample(first,key);
                let result=[self.natural_output[0].run(first[0]),self.natural_output[1].run(first[1])];
                let second=[self.natural_input[0].run(0.0),self.natural_input[1].run(0.0)];
                let second=self.process_sample(second,key);
                self.natural_output[0].run(second[0]);self.natural_output[1].run(second[1]);
                result
            } else {self.process_sample(input,key)};
            if spectral {
                let lag=if linear{self.fir.latency() as usize}else if self.params.phase==1{16}else{0};
                let mut key=key;
                for c in 0..2{self.key_alignment[c].push(key[c]);key[c]=self.key_alignment[c].tap(lag);}
                let params=&self.params;let env=&mut self.spectral_envelopes;let weights=&self.spectral_weights;
                let attack=&self.spectral_attacks;let release=&self.spectral_releases;let fs=self.sample_rate;let linear_gain=&mut self.linear_dynamic_gain;
                audio=self.spectral_frame.run(audio,key,|bin,audio,key,key_bins|{
                    if params.solo>0{return;}
                    let f=(bin as f64*fs/SPECTRAL_SIZE as f64).max(1.0);
                    if bin==0&&linear {
                        for b in 0..EQ_BANDS {
                            let d=params.dynamics[b];if !d.enabled||params.spectral[b]{continue;}
                            let mut energy=0.0;
                            for k in 1..SPECTRAL_SIZE/2 {
                                let a=key_bins[k];let c=key_bins[SPECTRAL_SIZE+k];
                                let amplitude=match params.bands[b].place {
                                    Placement::Left=>a.norm(),Placement::Right=>c.norm(),
                                    Placement::Mid=>a.add(c).norm()*0.5,Placement::Side=>a.add(c.scale(-1.0)).norm()*0.5,
                                    Placement::Stereo=>a.norm().max(c.norm()),
                                };
                                energy+=amplitude*amplitude*weights[b*(SPECTRAL_SIZE/2+1)+k].abs();
                            }
                            let level=(energy*2.0).sqrt()/(SPECTRAL_SIZE as f64*0.707);
                            let target=d.range_db*((lin_to_db(level)-d.threshold_db)/12.0).clamp(0.0,1.0);
                            let coefficient=if target.abs()>linear_gain[b].abs(){attack[b]}else{release[b]};
                            linear_gain[b]=target+coefficient*(linear_gain[b]-target);
                        }
                    }
                    for b in 0..EQ_BANDS {
                        let band=params.bands[b];let d=params.dynamics[b];
                        if !band.on || !d.enabled || !band.kind.has_gain() || (!params.spectral[b]&&!linear) {continue;}
                        let index=b*(SPECTRAL_SIZE/2+1)+bin;let weight=weights[index];
                        if weight.abs()<1e-5 {continue;}
                        let radius=((100.0-params.density[b])*0.16).round() as usize;
                        let mut neighborhood=0.0_f64;
                        for k in bin.saturating_sub(radius)..=(bin+radius).min(SPECTRAL_SIZE/2) {
                            let pair=[key_bins[k],key_bins[SPECTRAL_SIZE+k]];
                            let amplitude=match band.place {
                                Placement::Left=>pair[0].norm(),Placement::Right=>pair[1].norm(),
                                Placement::Mid=>pair[0].add(pair[1]).norm()*0.5,Placement::Side=>pair[0].add(pair[1].scale(-1.0)).norm()*0.5,
                                Placement::Stereo=>pair[0].norm().max(pair[1].norm()),
                            };
                            let distance=(k as f64-bin as f64).abs()/(radius+1) as f64;
                            neighborhood=neighborhood.max(amplitude*(1.0-distance));
                        }
                        let key_level=neighborhood.max(match band.place {
                            Placement::Left=>key[0].norm(),Placement::Right=>key[1].norm(),
                            Placement::Mid=>key[0].add(key[1]).norm()*0.5,Placement::Side=>key[0].add(key[1].scale(-1.0)).norm()*0.5,
                            Placement::Stereo=>key[0].norm().max(key[1].norm()),
                        })*2.0/(SPECTRAL_SIZE as f64*0.637);
                        let level=lin_to_db(key_level)+if params.spectral_tilt[b]{3.0*(f/1000.0).log2()}else{0.0};
                        let threshold=d.threshold_db;
                        let target=if params.spectral[b]{d.range_db*((level-threshold)/12.0).clamp(0.0,1.0)*weight}else{linear_gain[b]*weight};
                        let coefficient=if target.abs()>env[index].abs(){attack[b]}else{release[b]};
                        env[index]=if params.spectral[b]{target+coefficient*(env[index]-target)}else{target};
                        let g=db_to_lin(env[index]);
                        match band.place {
                            Placement::Stereo=>{audio[0]=audio[0].scale(g);audio[1]=audio[1].scale(g);}
                            Placement::Left=>audio[0]=audio[0].scale(g),Placement::Right=>audio[1]=audio[1].scale(g),
                            Placement::Mid|Placement::Side=>{
                                let m=audio[0].add(audio[1]).scale(0.5);let s=audio[0].add(audio[1].scale(-1.0)).scale(0.5);
                                let (m,s)=if band.place==Placement::Mid{(m.scale(g),s)}else{(m,s.scale(g))};
                                audio[0]=m.add(s);audio[1]=m.add(s.scale(-1.0));
                            }
                        }
                    }
                });
            }
            out_l[i]=(audio[0]*self.output) as f32;out_r[i]=(audio[1]*self.output) as f32;
            self.fir_clock=self.fir_clock.wrapping_add(1);
        }
    }
    fn process_sample(&mut self,input:[f64;2],key:[f64;2])->[f64;2] {

            let (mut l, mut r)=(input[0],input[1]);
            let (key_l,key_r)=(key[0],key[1]);
            if self.params.solo > 0 && self.params.solo <= EQ_BANDS {
                let band = &self.params.bands[self.params.solo - 1];
                let (sl, sr) = match band.place {
                    Placement::Stereo => (l, r), Placement::Left => (l, l), Placement::Right => (r, r),
                    Placement::Mid => ((l + r) * 0.5, (l + r) * 0.5), Placement::Side => ((l - r) * 0.5, (l - r) * 0.5),
                };
                return [self.solo_filter[0].run(sl),self.solo_filter[1].run(sr)];
            }
            for b in 0..EQ_BANDS {
                if !self.params.bands[b].on { continue; }
                let dynamic = self.params.dynamics[b];
                if dynamic.enabled && !self.params.spectral[b] && self.params.bands[b].kind.has_gain() {
                    let (kl, kr) = match self.params.bands[b].place {
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
                        let (sections, n) = effective.sections(self.dsp_sample_rate());
                        for k in 0..n { self.coeffs[b][k] = Lattice::new(sections[k]); }
                        self.applied_gain[b] = gain;
                    }
                }
                match self.params.bands[b].place {
                    Placement::Stereo => { l = self.band(b, 0, l); r = self.band(b, 1, r); }
                    Placement::Left => l = self.band(b, 0, l),
                    Placement::Right => r = self.band(b, 1, r),
                    Placement::Mid => { let (m, s) = ((l + r) * 0.5, (l - r) * 0.5); let m = self.band(b, 0, m); l = m + s; r = m - s; }
                    Placement::Side => { let (m, s) = ((l + r) * 0.5, (l - r) * 0.5); let s = self.band(b, 0, s); l = m + s; r = m - s; }
                }
            }
            self.clock = self.clock.wrapping_add(1);
            [l,r]
    }
}

fn phase_response(params:&EqParams,filters:&[([Coefficients;MAX_SECTIONS],usize);EQ_BANDS],fs:f64,frequency:f64)->[Complex;4]{
            if let Some(b)=params.solo.checked_sub(1).and_then(|i|params.bands.get(i)){
                let g=db_to_lin(response_db(rbj(Rbj::BandPass,b.freq,b.q.max(0.5),0.0,fs),frequency,fs));
                return (match b.place {Placement::Stereo=>[g,0.0,0.0,g],Placement::Left=>[g,0.0,g,0.0],
                    Placement::Right=>[0.0,g,0.0,g],Placement::Mid=>[g*0.5;4],Placement::Side=>[g*0.5,-g*0.5,g*0.5,-g*0.5]}).map(|g|Complex::new(g,0.0));
            }
            let one=Complex::new(1.0,0.0);let zero=Complex::default();let mut matrix=[one,zero,zero,one];
            let w=std::f64::consts::TAU*frequency/fs;let(sin,cos)=w.sin_cos();let cos2=2.0*cos*cos-1.0;let sin2=2.0*sin*cos;
            for(i,b)in params.bands.iter().enumerate(){
                if !b.on||(b.kind.has_gain()&&b.gain_db==0.0){continue;}
                let g=if params.brickwall[i]&&b.kind.has_slope(){
                    Complex::new(if(b.kind==BandKind::LowCut&&frequency<b.freq)||(b.kind==BandKind::HighCut&&frequency>b.freq){0.0}else{1.0},0.0)
                }else if params.slope_continuous[i]{
                    if b.kind==BandKind::AllPass{continuous_allpass(b,frequency,fs,params.effective_slope(i))}
                    else{Complex::new(db_to_lin(band_response_db_with_slope(b,frequency,fs,Some(params.effective_slope(i)))),0.0)}
                }else{
                    let transfer=filters[i].0[..filters[i].1].iter().fold(one,|h,&c|h.mul(biquad_transfer(c,cos,sin,cos2,sin2)));
                    if b.kind==BandKind::AllPass{transfer}else{Complex::new(transfer.norm(),0.0)}
                };
                let sum=g.add(one).scale(0.5);let difference=g.add(one.scale(-1.0)).scale(0.5);
                let a=match b.place{Placement::Stereo=>[g,zero,zero,g],Placement::Left=>[g,zero,zero,one],
                    Placement::Right=>[one,zero,zero,g],Placement::Mid=>[sum,difference,difference,sum],
                    Placement::Side=>[sum,difference.scale(-1.0),difference.scale(-1.0),sum]};
                matrix=[a[0].mul(matrix[0]).add(a[1].mul(matrix[2])),a[0].mul(matrix[1]).add(a[1].mul(matrix[3])),
                    a[2].mul(matrix[0]).add(a[3].mul(matrix[2])),a[2].mul(matrix[1]).add(a[3].mul(matrix[3]))];
            }matrix

}

/// A 33-tap, unity-DC half-band FIR. Two filters at 2x give 16 base samples.
#[derive(Clone,Copy)]
struct HalfBand { memory:[f64;33], coefficients:[f64;33], position:usize }
impl Default for HalfBand {
    fn default()->Self {
        let mut coefficients=std::array::from_fn(|i| {
            let n=i as f64-16.0;
            let sinc=if n==0.0{0.5}else{(std::f64::consts::PI*n*0.5).sin()/(std::f64::consts::PI*n)};
            sinc*(0.42-0.5*(std::f64::consts::TAU*i as f64/32.0).cos()+0.08*(2.0*std::f64::consts::TAU*i as f64/32.0).cos())
        });
        let norm:f64=coefficients.iter().sum();for h in &mut coefficients{*h/=norm;}
        Self{memory:[0.0;33],coefficients,position:0}
    }
}
impl HalfBand {
    fn run(&mut self,x:f64)->f64 {
        self.memory[self.position]=x;
        let sum=self.coefficients.iter().enumerate().map(|(i,h)|h*self.memory[(self.position+33-i)%33]).sum();
        self.position=(self.position+1)%33;sum
    }
}

mod fit;
pub use fit::{EqFitReport, EqMatchOptions, eq_fit_curve, eq_fit_curve_report, eq_match_spectra, eq_match_spectra_with_options};

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
        assert!(p.set("b2_shape", 10.0).is_err());
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

    fn amplitude_at(x:&[f32],frequency:f64)->f64 {
        let (re,im)=x.iter().enumerate().fold((0.0,0.0),|(re,im),(i,&x)|{
            let phase=std::f64::consts::TAU*frequency*i as f64/FS;(re+x as f64*phase.cos(),im+x as f64*phase.sin())
        });2.0*re.hypot(im)/x.len() as f64
    }
    #[test]
    fn phase_modes_report_real_latency_and_linear_impulse_is_symmetric() {
        for (mode,latency) in [(0,0),(1,16),(2,3200)] {
            let mut eq=EqProcessor::new(FS);eq.set_index(EqParams::index_of("phase").unwrap(),mode as f64).unwrap();
            assert_eq!(eq.latency(),latency);
            let mut input=vec![0.0;4096];input[0]=1.0;let(mut l,mut r)=(input.clone(),input.clone());
            eq.process(&input,&input,&mut l,&mut r);
            let maximum=l.iter().enumerate().max_by(|(_,a),(_,b)|a.abs().total_cmp(&b.abs())).unwrap().0;
            assert_eq!(maximum,latency as usize);
        }
        let mut eq=single_band_eq(bell(1500.0,9.0,1.0));
        eq.set_index(EqParams::index_of("phase").unwrap(),2.0).unwrap();
        let mut x=vec![0.0;4096];x[0]=1.0;let(mut l,mut r)=(x.clone(),x.clone());eq.process(&x,&x,&mut l,&mut r);
        let center=eq.latency() as usize;
        for i in 1..512 {assert!((l[center-i]-l[center+i]).abs()<1e-7);}
        let x=sine(1500.0,0.1,24000);let(mut l,mut r)=(vec![0.0;x.len()],vec![0.0;x.len()]);
        eq.process(&x,&x,&mut l,&mut r);
        assert!((rms_db(&l[12000..])-rms_db(&x[12000..])-9.0).abs()<0.2);
    }
    #[test]
    fn spectral_dynamics_reduces_only_loud_frequencies_and_never_audibly_injects_key() {
        let configure=||{
            let mut eq=single_band_eq(bell(2200.0,0.0,0.3));
            for(id,v)in[("b4_dynamic",1.0),("b4_spectral",1.0),("b4_density",100.0),("b4_dyn_range",-18.0),("b4_dyn_threshold",-25.0),("b4_dyn_attack",0.1)] {
                eq.set_index(EqParams::index_of(id).unwrap(),v).unwrap();
            }eq
        };
        let x:Vec<f32>=sine(1000.0,0.5,48000).iter().zip(sine(4000.0,0.01,48000)).map(|(a,b)|a+b).collect();
        let(mut l,mut r)=(vec![0.0;x.len()],vec![0.0;x.len()]);
        let mut eq=configure();assert_eq!(eq.latency(),2048);eq.process(&x,&x,&mut l,&mut r);
        assert!(amplitude_at(&l[24000..],1000.0)<0.15);
        assert!((amplitude_at(&l[24000..],4000.0)/0.01-1.0).abs()<0.08);
        let zeros=vec![0.0;x.len()];configure().process_with_sidechain(&zeros,&zeros,Some((&x,&x)),&mut l,&mut r);
        assert_eq!(peak(&l),0.0);
    }
    #[test]
    fn advanced_slopes_and_curve_fit_have_real_responses() {
        let mut b=Band{kind:BandKind::HighCut,..bell(2000.0,0.0,0.707)};
        for index in 4..SLOPES.len() {
            b.slope=index;let high=band_response_db(&b,10000.0,FS);
            assert!(high < -(SLOPES[index] as f64)*1.5,"{}: {high}",SLOPES[index]);
        }
        let target=bell(1000.0,9.0,1.0);
        let points:Vec<_>=(0..96).map(|i|{let f=20.0*1000.0_f64.powf(i as f64/95.0);(f,band_response_db(&target,f,FS))}).collect();
        let mut fit=EqParams::default();
        for(id,value)in eq_fit_curve(&points,8).unwrap(){fit.set(&id,value).unwrap();}
        let error=points.iter().map(|&(f,g)|(fit.response_db(f,FS)-g).powi(2)).sum::<f64>()/points.len() as f64;
        assert!(error.sqrt()<1.0,"fit RMS error {}",error.sqrt());
        assert!(eq_fit_curve(&[(20.0,0.0),(f64::NAN,1.0)],4).is_err());
        let source:Vec<_>=points.iter().map(|p|(p.0,-20.0)).collect();
        let target:Vec<_>=points.iter().map(|p|(p.0,-14.0)).collect();
        let matched=eq_match_spectra(&source,&target,8).unwrap();
        assert!(matched.iter().filter(|(id,_)|id.ends_with("_on")).all(|(_,v)|*v==0.0));
    }

    #[test]
    fn flat_tilt_is_anchored_to_pivot_and_distinct_from_tilt_shelf() {
        let tilt=Band{kind:BandKind::FlatTilt,..bell(1000.0,12.0,1.0)};
        assert!(band_response_db(&tilt,1000.0,FS).abs()<1e-8);
        let samples=[100.0,200.0,400.0,800.0,1600.0,3200.0,6400.0].map(|f|band_response_db(&tilt,f,FS));
        for step in samples.windows(2){assert!(step[1]>step[0]+0.5&&step[1]<step[0]+1.8);}
        let shelf=Band{kind:BandKind::TiltShelf,..tilt};
        assert!((band_response_db(&tilt,400.0,FS)-band_response_db(&shelf,400.0,FS)).abs()>1.0);
        let mut p=single_band_eq(tilt);let x=sine(400.0,0.1,24000);let(mut l,mut r)=(vec![0.0;x.len()],vec![0.0;x.len()]);
        p.process(&x,&x,&mut l,&mut r);
        assert!((rms_db(&l[12000..])-rms_db(&x[12000..])-band_response_db(&tilt,400.0,FS)).abs()<0.05);
        assert_eq!(BandKind::AllPass.index(),8);assert_eq!(BandKind::FlatTilt.index(),9);
    }

    #[test]
    fn continuous_slopes_are_fractional_for_every_shape_and_keep_legacy_indices(){
        assert_eq!(EqParams::index_of("b1_slope_continuous"),Some(389));
        assert_eq!(EqParams::index_of("b24_slope_db"),Some(436));
        assert!(!eq_spec("b1_slope_continuous").unwrap().automatable);
        assert!(eq_spec("b1_slope_db").unwrap().automatable);
        for kind in BandKind::ALL {
            let band=Band{kind,..bell(1000.0,12.0,0.707)};
            let probe=1600.0;
            if kind==BandKind::AllPass {
                let a=continuous_allpass(&band,probe,FS,17.25);
                let b=continuous_allpass(&band,probe,FS,17.26);
                assert!((a.norm()-1.0).abs()<1e-12);
                assert!((a.re-b.re).abs()+(a.im-b.im).abs()>1e-5);
            }else{
                let a=band_response_db_with_slope(&band,probe,FS,Some(17.25));
                let b=band_response_db_with_slope(&band,probe,FS,Some(17.26));
                assert!((a-b).abs()>1e-6&& (a-b).abs()<0.1,"{kind:?}: {a}, {b}");
                let minimum=band_response_db_with_slope(&band,probe,FS,Some(kind.min_slope()));
                assert_eq!(band_response_db_with_slope(&band,probe,FS,Some(0.0)),minimum);
            }
        }
        let cut=Band{kind:BandKind::HighCut,..bell(500.0,0.0,std::f64::consts::FRAC_1_SQRT_2)};
        for slope in [0.0,0.25,3.5,7.125,19.75,63.5,96.0]{
            let a=band_response_db_with_slope(&cut,8000.0,FS,Some(slope));
            let b=band_response_db_with_slope(&cut,16000.0,FS,Some(slope));
            if slope>=7.0&&slope<=30.0{assert!((a-b-slope).abs()<0.03,"fractional rolloff {slope}: {}",a-b);}
            assert!(a.is_finite()&&b.is_finite());
        }
        for kind in [BandKind::LowCut,BandKind::HighCut,BandKind::BandPass]{
            let band=Band{kind,..cut};
            for frequency in [20.0,1000.0,20000.0]{assert_eq!(band_response_db_with_slope(&band,frequency,FS,Some(0.0)),0.0);}
        }
        let mut params=EqParams::default();params.set("b1_slope_continuous",1.0).unwrap();params.set("b1_slope_db",7.125).unwrap();
        let mut restored=EqParams::default();for(id,value)in parse_state(&state_json(&params.descs())).unwrap(){restored.set(&id,value).unwrap();}
        assert_eq!(params,restored);
    }

    #[test]
    fn continuous_curve_targets_are_the_actual_audio_response(){
        for(kind,slope,frequency)in[
            (BandKind::Bell,17.25,1600.0),(BandKind::LowShelf,9.5,1500.0),
            (BandKind::HighShelf,9.5,1500.0),(BandKind::LowCut,7.125,1600.0),
            (BandKind::HighCut,7.125,1600.0),(BandKind::Notch,19.75,1500.0),
            (BandKind::BandPass,3.25,2500.0),(BandKind::TiltShelf,9.125,1600.0),
            (BandKind::FlatTilt,17.125,2000.0)
        ]{
            let mut eq=single_band_eq(Band{kind,..bell(1000.0,9.0,0.707)});
            eq.params.slope_continuous[3]=true;eq.params.slope_db[3]=slope;eq.params.resolution=2;eq.refresh();
            assert_eq!(eq.latency(),4224);
            let x=sine(frequency,0.01,24000);let(mut l,mut r)=(vec![0.0;x.len()],vec![0.0;x.len()]);
            eq.process(&x,&x,&mut l,&mut r);
            let measured=rms_db(&l[12000..])-rms_db(&x[12000..]);
            let target=eq.params.band_response_db(3,frequency,FS);
            assert!((measured-target).abs()<0.15,"{kind:?} {slope}: {measured} vs {target}");
        }
    }

    #[test]
    fn fractional_allpass_changes_phase_with_unity_gain_in_fir_path(){
        let band=Band{kind:BandKind::AllPass,..bell(2000.0,0.0,0.707)};
        let frequency=1500.0;let x=sine(frequency,0.1,24000);
        for slope in [6.0,9.125,12.0,17.25,48.0]{
            let mut eq=single_band_eq(band);eq.params.slope_continuous[3]=true;eq.params.slope_db[3]=slope;eq.params.resolution=2;eq.refresh();
            let(mut l,mut r)=(vec![0.0;x.len()],vec![0.0;x.len()]);eq.process(&x,&x,&mut l,&mut r);
            assert!((rms_db(&l[12000..])-rms_db(&x[12000..])).abs()<0.05,"allpass gain at {slope}");
            let(re,im)=l[12000..].iter().enumerate().fold((0.0,0.0),|(re,im),(i,&value)|{
                let angle=std::f64::consts::TAU*frequency*(i+12000)as f64/FS;
                (re+value as f64*angle.cos(),im+value as f64*angle.sin())
            });
            let target=continuous_allpass(&band,frequency,FS,slope);
            let phase=target.im.atan2(target.re)-std::f64::consts::TAU*frequency*eq.latency()as f64/FS;
            let error=re.atan2(im)-phase;
            assert!((error.cos()-1.0).hypot(error.sin())<0.01,"allpass phase at {slope}: {error}");
        }
    }

    #[test]
    fn continuous_slope_automation_keeps_latency_and_block_partition_identity(){
        let build=||{
            let mut eq=single_band_eq(bell(2000.0,6.0,0.707));
            eq.params.slope_continuous[3]=true;eq.params.slope_db[3]=17.25;eq.params.resolution=0;eq.refresh();eq
        };
        let input=sine(1700.0,0.1,10000);
        let run=|partition:usize|{
            let mut eq=build();let(mut l,mut r)=(vec![0.0;input.len()],vec![0.0;input.len()]);
            let mut at=0;while at<input.len(){
                if at==1307{eq.set_index(EqParams::index_of("b4_slope_db").unwrap(),18.125).unwrap();}
                let end=(at+partition).min(input.len()).min(if at<1307{1307}else{input.len()});
                eq.process(&input[at..end],&input[at..end],&mut l[at..end],&mut r[at..end]);at=end;
            }
            assert_eq!(eq.latency(),2432);
            eq.set_index(EqParams::index_of("b4_on").unwrap(),0.0).unwrap();assert_eq!(eq.latency(),2432);
            let old=eq.params.clone();assert!(eq.set_index(EqParams::index_of("b4_slope_db").unwrap(),f64::NAN).is_err());assert_eq!(eq.params,old);
            (l,r)
        };
        assert_eq!(run(10000),run(37));
    }

    #[test]
    fn natural_mode_coefficients_are_restored_after_fir_controls(){
        let mut changed=single_band_eq(bell(8000.0,9.0,1.0));
        changed.set_index(EqParams::index_of("b1_slope_continuous").unwrap(),1.0).unwrap();
        changed.set_index(EqParams::index_of("phase").unwrap(),1.0).unwrap();
        changed.set_index(EqParams::index_of("b1_slope_continuous").unwrap(),0.0).unwrap();
        let mut reference=single_band_eq(bell(8000.0,9.0,1.0));
        reference.set_index(EqParams::index_of("phase").unwrap(),1.0).unwrap();
        let x=sine(8000.0,0.1,4096);let(mut al,mut ar,mut bl,mut br)=(vec![0.0;x.len()],vec![0.0;x.len()],vec![0.0;x.len()],vec![0.0;x.len()]);
        changed.process(&x,&x,&mut al,&mut ar);reference.process(&x,&x,&mut bl,&mut br);
        assert_eq!(al,bl);assert_eq!(ar,br);assert_eq!(changed.latency(),16);
    }

    fn staged_eq()->EqProcessor{
        let mut p=EqProcessor::new(FS);
        p.params.phase=2;p.params.resolution=2;
        for i in 0..EQ_BANDS{
            p.params.bands[i]=Band{kind:BandKind::Bell,on:true,freq:80.0*160.0_f64.powf(i as f64/23.0),
                gain_db:if i%2==0{1.0}else{-1.0},q:0.8,slope:1,place:Placement::ALL[i%5]};
            p.params.slope_continuous[i]=true;p.params.slope_db[i]=18.3;
        }
        p.refresh();p
    }
    fn staged_eq_render(chunks:&[usize])->(Vec<f32>,EqProcessor,usize,usize){
        let mut p=staged_eq();p.begin_audio();
        let initial_designs=p.fir.designs_started;let initial_steps=p.fir.work_steps;
        let x:Vec<f32>=(0..22000).map(|i|((i*719%997)as f32/997.0-0.5)*0.2).collect();
        let mut l=vec![0.0;x.len()];let mut r=l.clone();let(mut frame,mut chunk)=(0,0);
        while frame<x.len(){
            if frame<3200&&frame%32==0{
                let before=p.fir.designs_started;
                let tick=(frame/32)as f64;
                for band in 0..8{
                    let (field,value)=match band%3{0=>("freq",200.0+band as f64*400.0+tick),1=>("gain",2.0+tick*0.01),_=>("slope_db",18.5+tick*0.03)};
                    p.set_index(EqParams::index_of(&format!("b{}_{field}",band+1)).unwrap(),value).unwrap();
                }
                assert_eq!(p.fir.designs_started,before,
                    "parameter writes must only dirty the current target");
            }
            let mut end=(frame+chunks[chunk%chunks.len()]).min(x.len());
            if frame<3200{end=end.min((frame/32+1)*32);}
            p.process(&x[frame..end],&x[frame..end],&mut l[frame..end],&mut r[frame..end]);
            frame=end;chunk+=1;
        }
        (l,p,initial_designs,initial_steps)
    }
    #[test]
    fn fir_automation_is_coalesced_partition_invariant_and_reaches_latest_curve(){
        let (whole,p,initial,steps)=staged_eq_render(&[22000]);
        let (split,other,_,_)=staged_eq_render(&[1,7,113,256,31]);
        assert_eq!(whole,split);
        assert_eq!(p.fir_clock,22000);assert_eq!(p.fir_clock,other.fir_clock);
        assert!(p.fir.design_idle()&&!p.fir_dirty);
        assert_eq!(p.fir_target,p.params);
        let designs=p.fir.designs_started-initial;
        assert!((2..=4).contains(&designs),"{designs} snapshots for 800 parameter writes");
        assert_eq!(p.fir.designs_committed,p.fir.designs_started);
        assert!(p.fir.work_steps-steps<=22000_usize.div_ceil(32));
        let mut settled=staged_eq();settled.params=p.params.clone();settled.refresh();
        let x:Vec<f32>=(0..22000).map(|i|((i*719%997)as f32/997.0-0.5)*0.2).collect();
        let mut expected=vec![0.0;x.len()];let mut right=expected.clone();
        settled.process(&x,&x,&mut expected,&mut right);
        assert_eq!(&whole[18000..],&expected[18000..],"the final target must fully reach the audio path");
        assert_eq!(p.latency(),4224);
    }
    #[test]
    fn setup_and_fixed_fir_latency_changes_publish_synchronously(){
        let mut p=EqProcessor::new(FS);
        for(id,value)in[("phase",2.0),("b3_gain",6.0),("b3_freq",2200.0)]{
            p.set_index(EqParams::index_of(id).unwrap(),value).unwrap();
            assert!(p.fir.design_idle()&&!p.fir_dirty);
        }
        assert_eq!(p.fir.latency(),1152);
        p.begin_audio();
        p.set_index(EqParams::index_of("b3_freq").unwrap(),3000.0).unwrap();
        p.process(&[0.2;32],&[0.2;32],&mut[0.0;32],&mut[0.0;32]);
        assert!(!p.fir.design_idle());
        p.set_index(EqParams::index_of("resolution").unwrap(),0.0).unwrap();
        assert!(p.fir.design_idle()&&!p.fir_dirty);
        assert_eq!(p.fir.latency(),384);assert_eq!(p.latency(),2432);
        assert_eq!(p.fir_clock,32,"a fixed control must not reset the work clock");
        let mut l=vec![1.0;2432];let mut r=l.clone();
        p.process(&vec![0.0;2432],&vec![0.0;2432],&mut l,&mut r);
        assert!(l.iter().chain(r.iter()).all(|&v|v==0.0));
    }

}
