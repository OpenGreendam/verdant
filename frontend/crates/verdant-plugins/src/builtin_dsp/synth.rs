//! Built-in instruments: a polyphonic four-oscillator subtractive synth and
//! a deliberately compact single-oscillator sound. An
//! instrument track renders its notes through it, the same way for playback
//! and export, so what plays is what exports.

use super::filters::db_to_lin;
use super::{table_defaults, table_index, Spec};
use std::f64::consts::{PI, TAU};

pub const SYNTH_ID: &str = "verdant.builtin.synth";
pub const ONE_SYNTH_ID: &str = "verdant.builtin.one";

pub fn is_builtin_instrument(id: &str) -> bool { matches!(id, SYNTH_ID | ONE_SYNTH_ID) }
pub fn instrument_name(id: &str) -> Option<&'static str> {
    match id { SYNTH_ID => Some("Verdant Synth"), ONE_SYNTH_ID => Some("Verdant One"), _ => None }
}

pub const WAVES: [&str; 4] = ["锯齿", "方波", "三角", "正弦"];
/// The compact instrument has its own genuine single-oscillator surface.
/// Hidden polyphonic-synth controls are not accepted when loading One state.
pub const ONE_PARAM_IDS: &[&str] = &["osc1", "attack", "decay", "sustain", "release", "cutoff", "resonance", "env_amount", "gain",
    "lfo_rate", "lfo_pitch", "lfo_filter", "filter_shape", "pulse_width", "drive", "noise", "voices", "portamento", "keytrack", "velocity", "filter_character", "osc_model", "bend_range"];

pub static SYNTH_SPECS: &[Spec] = &[
    Spec::new("osc1", "振荡器 1", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("osc2", "振荡器 2", 0.0, 3.0, 1.0, "").steps(3),
    Spec::new("osc2_semi", "音程 2", -24.0, 24.0, 0.0, "st").steps(48),
    Spec::new("detune", "失谐", 0.0, 50.0, 8.0, "ct"),
    Spec::new("osc2_mix", "混合 2", 0.0, 100.0, 40.0, "%"),
    Spec::new("attack", "起音", 1.0, 2000.0, 5.0, "ms").log(),
    Spec::new("decay", "衰减", 1.0, 4000.0, 300.0, "ms").log(),
    Spec::new("sustain", "延音", 0.0, 100.0, 70.0, "%"),
    Spec::new("release", "释音", 1.0, 4000.0, 250.0, "ms").log(),
    Spec::new("cutoff", "截止频率", 50.0, 20000.0, 4000.0, "Hz").log(),
    Spec::new("resonance", "共振", 0.5, 10.0, 0.9, "").log(),
    Spec::new("env_amount", "滤波包络", 0.0, 100.0, 30.0, "%"),
    Spec::new("width", "宽度", 0.0, 100.0, 50.0, "%"),
    Spec::new("gain", "音量", -24.0, 6.0, -8.0, "dB"),
    // Append only: existing project automation indices 0..13 are stable.
    Spec::new("osc3", "振荡器 3", 0.0, 3.0, 2.0, "").steps(3),
    Spec::new("osc3_semi", "音程 3", -24.0, 24.0, 0.0, "st").steps(48),
    Spec::new("osc3_mix", "混合 3", 0.0, 100.0, 0.0, "%"),
    Spec::new("osc4", "振荡器 4", 0.0, 3.0, 3.0, "").steps(3),
    Spec::new("osc4_semi", "音程 4", -24.0, 24.0, -12.0, "st").steps(48),
    Spec::new("osc4_mix", "混合 4", 0.0, 100.0, 0.0, "%"),
    Spec::new("noise", "噪声", 0.0, 100.0, 0.0, "%"),
    Spec::new("unison", "齐奏声部", 1.0, 8.0, 1.0, "").steps(7),
    Spec::new("unison_detune", "齐奏失谐", 0.0, 50.0, 15.0, "ct"),
    Spec::new("lfo_rate", "LFO 速率", 0.01, 20.0, 1.0, "Hz").log(),
    Spec::new("lfo_pitch", "LFO 音高", 0.0, 12.0, 0.0, "st"),
    Spec::new("lfo_filter", "LFO 滤波", -48.0, 48.0, 0.0, "st"),
    Spec::new("filter_shape", "滤波形状", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("filter2_enabled", "滤波器 2", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("cutoff2", "截止频率 2", 50.0, 20000.0, 200.0, "Hz").log(),
    Spec::new("resonance2", "共振 2", 0.5, 10.0, 0.9, "Q").log(),
    Spec::new("filter2_shape", "滤波形状 2", 0.0, 3.0, 1.0, "").steps(3),
    Spec::new("filter_routing", "滤波路由", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("pulse_width", "脉冲宽度", 5.0, 95.0, 50.0, "%"),
    Spec::new("drive", "驱动", 0.0, 30.0, 0.0, "dB"),
    Spec::new("voices", "最大复音", 1.0, 64.0, 64.0, "").steps(63),
    Spec::new("portamento", "滑音", 0.0, 2000.0, 0.0, "ms"),
    Spec::new("keytrack", "滤波键盘跟踪", 0.0, 100.0, 0.0, "%"),
    Spec::new("velocity", "力度响应", 0.0, 100.0, 100.0, "%"),
    Spec::new("filter3_enabled","滤波器 3",0.0,1.0,0.0,"").steps(1),
    Spec::new("cutoff3","截止频率 3",20.0,20000.0,1000.0,"Hz"),
    Spec::new("resonance3","共振 3",0.5,10.0,0.9,"Q"),
    Spec::new("filter3_shape","滤波形状 3",0.0,3.0,0.0,"").steps(3),
    Spec::new("filter4_enabled","滤波器 4",0.0,1.0,0.0,"").steps(1),
    Spec::new("cutoff4","截止频率 4",20.0,20000.0,1000.0,"Hz"),
    Spec::new("resonance4","共振 4",0.5,10.0,0.9,"Q"),
    Spec::new("filter4_shape","滤波形状 4",0.0,3.0,0.0,"").steps(3),
    Spec::new("filter1_pan","滤波声像 1",-100.0,100.0,0.0,"%"),
    Spec::new("filter2_pan","滤波声像 2",-100.0,100.0,0.0,"%"),
    Spec::new("filter3_pan","滤波声像 3",-100.0,100.0,0.0,"%"),
    Spec::new("filter4_pan","滤波声像 4",-100.0,100.0,0.0,"%"),
    Spec::new("filter_character","滤波性格",0.0,5.0,0.0,"").steps(5),
    Spec::new("osc_model","振荡器模型",0.0,4.0,0.0,"").steps(4),
    Spec::new("high_quality","高品质过采样",0.0,2.0,0.0,"").steps(2).fixed(),
    Spec::new("mpe","MPE",0.0,1.0,0.0,"").steps(1),
    Spec::new("bend_range","弯音范围",0.0,96.0,2.0,"st"),
    Spec::new("mpe_range","MPE 弯音范围",0.0,96.0,48.0,"st"),
    Spec::new("arp_on","琶音器",0.0,1.0,0.0,"").steps(1),
    Spec::new("arp_rate","琶音速率",0.3,20.0,4.0,"Hz"),
    Spec::new("arp_sync","琶音同步",0.0,8.0,5.0,"").steps(8),
    Spec::new("arp_tempo","琶音备用速度",20.0,300.0,120.0,"BPM"),
    Spec::new("arp_order","琶音顺序",0.0,4.0,0.0,"").steps(4),
    Spec::new("arp_octaves","琶音八度",1.0,3.0,1.0,"").steps(2),
    Spec::new("arp_gate","琶音连音",1.0,100.0,70.0,"%"),
    Spec::new("arp_groove","琶音摇摆",-75.0,75.0,0.0,"%"),
    Spec::new("arp_latch","琶音保持",0.0,2.0,0.0,"").steps(2),
    Spec::new("fx_drive","效果 · 驱动",0.0,100.0,0.0,"%"),
    Spec::new("fx_drive_style","驱动类型",0.0,3.0,0.0,"").steps(3),
    Spec::new("fx_chorus","效果 · 合唱",0.0,100.0,0.0,"%"),
    Spec::new("fx_chorus_rate","合唱速率",0.0,10.0,0.5,"Hz"),
    Spec::new("fx_chorus_depth","合唱深度",0.0,20.0,6.0,"ms"),
    Spec::new("fx_phaser","效果 · 相位",0.0,100.0,0.0,"%"),
    Spec::new("fx_phaser_rate","相位速率",0.0,10.0,0.3,"Hz"),
    Spec::new("fx_phaser_feedback","相位反馈",-90.0,90.0,25.0,"%"),
    Spec::new("fx_flanger","相位/镶边",0.0,1.0,0.0,"").steps(1),
    Spec::new("fx_delay","效果 · 延迟",0.0,100.0,0.0,"%"),
    Spec::new("fx_delay_time","延迟时间",1.0,2000.0,300.0,"ms"),
    Spec::new("fx_delay_feedback","延迟反馈",0.0,95.0,30.0,"%"),
    Spec::new("fx_delay_sync","延迟同步",0.0,8.0,0.0,"").steps(8),
    Spec::new("fx_reverb","效果 · 混响",0.0,100.0,0.0,"%"),
    Spec::new("fx_reverb_time","混响衰减",0.1,20.0,2.0,"s"),
    Spec::new("fx_reverb_brightness","混响亮度",0.0,100.0,50.0,"%"),
    Spec::new("fx_compressor","效果 · 压缩",0.0,100.0,0.0,"%"),
    Spec::new("fx_comp_threshold","压缩阈值",-60.0,0.0,-18.0,"dB"),
    Spec::new("fx_comp_ratio","压缩比",1.0,20.0,4.0,""),
];

/// Native controls use instrument-specific defaults/ranges. One shares the
/// processor but never presents poly-synth controls that its loader ignores.
pub fn instrument_specs(id: &str) -> Vec<Spec> {
    let mut specs:Vec<Spec>=SYNTH_SPECS.iter().filter(|s|id!=ONE_SYNTH_ID||ONE_PARAM_IDS.contains(&s.id)).map(|s|{
        let mut s=*s;if id==ONE_SYNTH_ID&&s.id=="voices"{s.max=10.;s.default=10.;s.steps=9;}s
    }).collect();
    let targets=specs.len();specs.extend(super::MOD_SPECS.iter().map(|s|{
        let mut s=*s;if s.id.ends_with("_target"){s.max=(targets-1)as f64;s.steps=(targets-1)as i64;}s
    }));specs
}

/// Starting sounds: (name, [(id, value)]). Unlisted ids keep their defaults.
pub const SYNTH_PRESETS: &[(&str, &[(&str, f64)])] = &[
    ("温暖铺底", &[("osc1", 0.0), ("osc2", 0.0), ("detune", 14.0), ("attack", 400.0), ("decay", 1200.0), ("sustain", 80.0),
        ("release", 900.0), ("cutoff", 1800.0), ("resonance", 0.8), ("env_amount", 15.0), ("width", 90.0), ("gain", -12.0)]),
    ("拨弦", &[("osc1", 0.0), ("osc2", 1.0), ("osc2_mix", 25.0), ("attack", 2.0), ("decay", 280.0), ("sustain", 0.0),
        ("release", 180.0), ("cutoff", 900.0), ("resonance", 1.4), ("env_amount", 70.0), ("width", 40.0), ("gain", -8.0)]),
    ("贝斯", &[("osc1", 0.0), ("osc2", 1.0), ("osc2_semi", -12.0), ("osc2_mix", 50.0), ("detune", 3.0), ("attack", 3.0),
        ("decay", 200.0), ("sustain", 60.0), ("release", 80.0), ("cutoff", 700.0), ("resonance", 1.2), ("env_amount", 45.0), ("width", 0.0), ("gain", -6.0)]),
    ("主音", &[("osc1", 1.0), ("osc2", 0.0), ("osc2_semi", 12.0), ("osc2_mix", 30.0), ("detune", 6.0), ("attack", 8.0),
        ("decay", 400.0), ("sustain", 75.0), ("release", 200.0), ("cutoff", 5000.0), ("resonance", 1.0), ("env_amount", 20.0), ("width", 30.0), ("gain", -10.0)]),
    ("电钢", &[("osc1", 3.0), ("osc2", 2.0), ("osc2_semi", 12.0), ("osc2_mix", 20.0), ("detune", 4.0), ("attack", 2.0),
        ("decay", 1500.0), ("sustain", 20.0), ("release", 400.0), ("cutoff", 6000.0), ("resonance", 0.7), ("env_amount", 10.0), ("width", 60.0), ("gain", -6.0)]),
    ("四振荡器弦乐", &[("osc1", 0.0), ("osc2", 0.0), ("osc3", 0.0), ("osc3_mix", 35.0), ("osc3_semi", 12.0),
        ("osc4", 2.0), ("osc4_mix", 20.0), ("unison", 4.0), ("unison_detune", 12.0), ("attack", 250.0), ("release", 700.0),
        ("cutoff", 2800.0), ("lfo_rate", 0.3), ("lfo_filter", 8.0), ("gain", -12.0)]),
    ("噪声打击", &[("osc2_mix", 0.0), ("noise", 100.0), ("attack", 1.0), ("decay", 120.0), ("sustain", 0.0),
        ("release", 80.0), ("filter_shape", 2.0), ("cutoff", 1800.0), ("resonance", 2.0), ("gain", -10.0)]),
];

pub const ONE_PRESETS: &[(&str, &[(&str, f64)])] = &[
    ("单振荡器贝斯", &[("osc1", 0.0), ("cutoff", 600.0), ("env_amount", 45.0), ("decay", 180.0), ("sustain", 55.0), ("release", 80.0)]),
    ("脉冲主音", &[("osc1", 1.0), ("pulse_width", 30.0), ("cutoff", 4500.0), ("lfo_rate", 5.0), ("lfo_pitch", 0.1), ("drive", 3.0)]),
    ("柔和长音", &[("osc1", 2.0), ("cutoff", 2200.0), ("attack", 350.0), ("release", 900.0), ("lfo_rate", 0.25), ("lfo_filter", 6.0)]),
];

/// One note to play, in samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthNote { pub start: u64, pub length: u64, pub pitch: f64, pub velocity: f64 }

/// Validated synth settings: values in `SYNTH_SPECS` order.
#[derive(Clone, Debug, PartialEq)]
pub struct SynthParams { pub values: Vec<f64>, pub modulation: super::modulation::ModParams }

impl Default for SynthParams { fn default() -> Self { Self { values: table_defaults(SYNTH_SPECS), modulation: Default::default() } } }

impl SynthParams {
    pub fn one_defaults() -> Self {
        let mut params = Self::default();
        params.set("osc2_mix", 0.0).expect("known parameter");
        params.set("width", 0.0).expect("known parameter");
        params.set("voices", 10.0).expect("known parameter");
        params
    }
    /// The instrument's own defaults apply before saved fields, including
    /// old files that do not contain the appended synthesizer parameters.
    pub fn from_json_for(id: &str, value: Option<&serde_json::Value>) -> Self {
        let mut p = if id == ONE_SYNTH_ID { Self::one_defaults() } else { Self::default() };
        if let Some(map) = value.and_then(serde_json::Value::as_object) {
            for (key, value) in map {
                if id == ONE_SYNTH_ID && !ONE_PARAM_IDS.contains(&key.as_str()) && super::modulation_parameter(key).is_none() { continue; }
                if let Some(value) = value.as_f64() {
                    if id == ONE_SYNTH_ID && key == "voices" && value > 10.0 { continue; }
                    let _ = p.set(key, value);
                }
            }
        }
        p
    }
    pub fn get(&self, id: &str) -> f64 { table_index(SYNTH_SPECS, id).map_or_else(||self.modulation.get(id), |i| self.values[i]) }
    pub fn set(&mut self, id: &str, value: f64) -> Result<(), String> {
        if super::modulation_parameter(id).is_some() { return self.modulation.set(id,value); }
        let i = table_index(SYNTH_SPECS, id).ok_or_else(|| format!("unknown synth parameter {id}"))?;
        SYNTH_SPECS[i].check(value)?;
        self.values[i] = value;
        Ok(())
    }
    /// From a `{"id": value}` object; unknown ids and bad values are ignored
    /// so an older or newer project still opens.
    pub fn from_json(value: Option<&serde_json::Value>) -> Self {
        Self::from_json_for(SYNTH_ID, value)
    }
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Object(SYNTH_SPECS.iter().zip(&self.values).chain(super::MOD_SPECS.iter().zip(&self.modulation.values)).map(|(s, v)| (s.id.to_string(), serde_json::json!(v))).collect())
    }
    pub fn preset(name: &str) -> Option<Self> {
        let (_, values) = SYNTH_PRESETS.iter().find(|(n, _)| *n == name)?;
        let mut p = Self::default();
        for (id, v) in values.iter() { p.set(id, *v).ok()?; }
        Some(p)
    }
    pub fn preset_for(id: &str, name: &str) -> Option<Self> {
        if id != ONE_SYNTH_ID { return Self::preset(name); }
        let (_, values) = ONE_PRESETS.iter().find(|(n, _)| *n == name)?;
        let mut p = Self::one_defaults();
        for (id, value) in values.iter() { p.set(id, *value).ok()?; }
        Some(p)
    }
    /// Amplitude envelope level `t` seconds into a note held `held` seconds.
    pub fn envelope(&self, t: f64, held: f64) -> f64 {
        let (a, d, s, r) = (self.get("attack") / 1000.0, self.get("decay") / 1000.0, self.get("sustain") / 100.0, self.get("release") / 1000.0);
        let level = |t: f64| if t < a { t / a } else if t < a + d { 1.0 - (1.0 - s) * (t - a) / d } else { s };
        if t <= held { level(t) } else { (level(held) * (1.0 - (t - held) / r.max(1e-4))).max(0.0) }
    }
    pub fn release_seconds(&self) -> f64 { self.get("release") / 1000.0 }
}

/// PolyBLEP residual for band-limited steps.
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt { let x = t / dt; x + x - x * x - 1.0 }
    else if t > 1.0 - dt { let x = (t - 1.0) / dt; x * x + x + x + 1.0 }
    else { 0.0 }
}

pub(super) fn osc(wave: usize, phase: f64, dt: f64, pulse_width: f64) -> f64 {
    // A transposed oscillator beyond Nyquist contributes no aliased pitch.
    if !(0.0..0.49).contains(&dt) { return 0.0; }
    match wave {
        0 => (2.0 * phase - 1.0) - blep(phase, dt),
        1 => {
            let mut v = if phase < pulse_width { 1.0 } else { -1.0 };
            v += blep(phase, dt);
            v -= blep((phase + 1.0 - pulse_width).fract(), dt);
            v - (2.0 * pulse_width - 1.0)
        }
        2 => 1.0 - 4.0 * (phase - 0.5).abs(),
        _ => (TAU * phase).sin(),
    }
}

/// Zero-delay-feedback state-variable low-pass (Simper).
#[derive(Clone, Copy, Default)]
pub(super) struct Svf { ic1: f64, ic2: f64 }

impl Svf {
    #[inline]
    pub(super) fn run(&mut self, x: f64, cutoff: f64, q: f64, sample_rate: f64, shape: usize) -> f64 {
        let g = (PI * cutoff.clamp(20.0, sample_rate * 0.45) / sample_rate).tan();
        let k = 1.0 / q.max(0.3);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let (a2, a3) = (g * a1, g * g * a1);
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match shape { 1 => x - k * v1 - v2, 2 => k * v1, 3 => x - k * v1, _ => v2 }
    }
}

/// Render `notes` to interleaved stereo PCM covering every note's release.
#[allow(dead_code)]
fn render_legacy(params: &SynthParams, notes: &[SynthNote], sample_rate: f64) -> Vec<f32> {
    let release = (params.release_seconds() * sample_rate).ceil() as u64;
    let frames = notes.iter().map(|n| n.start + n.length + release).max().unwrap_or(0) as usize;
    let mut out = vec![0.0f32; frames * 2];
    let voice_limit = params.get("voices").round().clamp(1.0, 64.0) as usize;
    let mut note_order: Vec<usize> = (0..notes.len()).collect();
    note_order.sort_by_key(|&index| (notes[index].start, index));
    let mut ends: Vec<u64> = notes.iter().map(|note| note.start + note.length + release).collect();
    let mut previous_pitch: Vec<f64> = notes.iter().map(|note| note.pitch).collect();
    let mut active: Vec<usize> = Vec::with_capacity(voice_limit);
    let mut previous = None;
    for &index in &note_order {
        let start = notes[index].start;
        active.retain(|&old| ends[old] > start);
        if let Some(pitch) = previous { previous_pitch[index] = pitch; }
        previous = Some(notes[index].pitch);
        if active.len() == voice_limit {
            // `active` is in note-on order, including sounding release tails.
            // Steal the oldest sounding voice, not the first N score notes.
            let stolen = active.remove(0);
            ends[stolen] = start;
        }
        active.push(index);
    }
    let waves = ["osc1", "osc2", "osc3", "osc4"].map(|id| params.get(id).round() as usize);
    let semitones = [0.0, params.get("osc2_semi").round(), params.get("osc3_semi").round(), params.get("osc4_semi").round()];
    let detune = params.get("detune");
    let mixes = [1.0 - params.get("osc2_mix") * 0.005, params.get("osc2_mix") * 0.01, params.get("osc3_mix") * 0.01, params.get("osc4_mix") * 0.01];
    let noise_mix = params.get("noise") * 0.01;
    let unison = params.get("unison").round().clamp(1.0, 8.0) as usize;
    let unison_detune = params.get("unison_detune");
    let lfo_rate = params.get("lfo_rate");
    let lfo_pitch = params.get("lfo_pitch");
    let lfo_filter = params.get("lfo_filter");
    let pulse_width = params.get("pulse_width") * 0.01;
    let portamento = params.get("portamento") * 0.001;
    let keytrack = params.get("keytrack") * 0.01;
    let velocity_depth = params.get("velocity") * 0.01;
    let (cutoff, q, amount) = (params.get("cutoff"), params.get("resonance"), params.get("env_amount") / 100.0 * 4.0);
    let shape = params.get("filter_shape").round() as usize;
    let second = params.get("filter2_enabled") >= 0.5;
    let (cutoff2, q2, shape2) = (params.get("cutoff2"), params.get("resonance2"), params.get("filter2_shape").round() as usize);
    let parallel = params.get("filter_routing") >= 0.5;
    let drive_db = params.get("drive");
    let drive = db_to_lin(drive_db);
    let drive_norm = drive.tanh();
    let width = params.get("width") / 100.0;
    let gain = db_to_lin(params.get("gain"));
    for (note_index, note) in notes.iter().enumerate() {
        let held = note.length as f64 / sample_rate;
        let base = 440.0 * 2f64.powf((note.pitch - 69.0) / 12.0);
        // Left and right detune in opposite directions; width scales it.
        let mut frequencies = [[[0.0f64; 2]; 4]; 8];
        let mut phase = [[[0.0f64; 2]; 4]; 8];
        // Decorrelate voices deterministically from their pitch and start.
        let seed = (note.pitch * 7.31 + note.start as f64 * 1e-5).fract();
        for voice in 0..unison {
            let spread = if unison == 1 { 0.0 } else { voice as f64 * 2.0 / (unison - 1) as f64 - 1.0 };
            let offset = voice as f64 * 0.618_033_988_75;
            for oscillator in 0..4 {
                for channel in 0..2 {
                    let sign = if channel == 0 { 1.0 } else { -1.0 };
                    let cents = if oscillator == 0 { sign * detune * width * 0.5 } else { -sign * detune };
                    frequencies[voice][oscillator][channel] = base * 2f64.powf((semitones[oscillator] * 100.0 + cents + spread * unison_detune) / 1200.0);
                    phase[voice][oscillator][channel] = if oscillator == 0 { offset.fract() }
                        else { (seed + offset + channel as f64 * 0.37 + (oscillator - 1) as f64 * 0.271).fract() };
                }
            }
        }
        let mut noise_state = [note.start ^ note.pitch.to_bits() ^ 0x9e37_79b9_7f4a_7c15, note.start ^ note.pitch.to_bits() ^ 0xd1b5_4a32_d192_ed03];
        let mut filters = [[Svf::default(); 2]; 2];
        let natural_end = (note.start + note.length + release) as usize;
        let end = ends[note_index] as usize;
        let tracked = 2f64.powf((note.pitch - 60.0) * keytrack / 12.0);
        let velocity_gain = 1.0 + (note.velocity - 1.0) * velocity_depth;
        for i in note.start as usize..end.min(frames) {
            let t = (i - note.start as usize) as f64 / sample_rate;
            let env = params.envelope(t, held);
            if t > held && env <= 0.0 { break; }
            let lfo = (TAU * lfo_rate * t).sin();
            let glide = if portamento > 0.0 { (previous_pitch[note_index] - note.pitch) * (1.0 - (t / portamento).min(1.0)) } else { 0.0 };
            let pitch_factor = 2f64.powf((lfo_pitch * lfo + glide) / 12.0);
            let cut = cutoff * tracked * 2f64.powf(amount * env + lfo_filter * lfo / 12.0);
            for c in 0..2 {
                let mut s = 0.0;
                for voice in 0..unison {
                    for oscillator in 0..4 {
                        if mixes[oscillator] == 0.0 { continue; }
                        let increment = frequencies[voice][oscillator][c] * pitch_factor / sample_rate;
                        s += osc(waves[oscillator], phase[voice][oscillator][c], increment, pulse_width) * mixes[oscillator];
                        phase[voice][oscillator][c] = (phase[voice][oscillator][c] + increment).fract();
                    }
                }
                s /= unison as f64;
                noise_state[c] = noise_state[c].wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let noise = ((noise_state[c] >> 11) as f64 / ((1u64 << 53) as f64)) * 2.0 - 1.0;
                s = (s + noise * noise_mix) / (1.0 + mixes[2] + mixes[3] + noise_mix * 0.5);
                if drive_db > 0.0 { s = (s * drive).tanh() / drive_norm; }
                let mut y = filters[0][c].run(s, cut, q, sample_rate, shape);
                if second {
                    let second_input = if parallel { s } else { y };
                    let filtered = filters[1][c].run(second_input, cutoff2 * tracked, q2, sample_rate, shape2);
                    y = if parallel { 0.5 * (y + filtered) } else { filtered };
                }
                let steal_fade = if end < natural_end { ((end - i) as f64 / (sample_rate * 0.001).max(1.0)).min(1.0) } else { 1.0 };
                let y = y * env * velocity_gain * gain * steal_fade;
                out[i * 2 + c] += y as f32;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn mono(pcm: &[f32]) -> Vec<f32> { pcm.chunks(2).map(|f| f[0]).collect() }

    #[test]
    fn a_note_sounds_at_its_pitch_for_its_length_then_releases() {
        let mut p = SynthParams::default();
        for (id, v) in [("osc1", 3.0), ("osc2_mix", 0.0), ("width", 0.0), ("cutoff", 20000.0), ("env_amount", 0.0), ("release", 50.0)] { p.set(id, v).unwrap(); }
        let note = SynthNote { start: 4_800, length: 24_000, pitch: 69.0, velocity: 1.0 };
        let pcm = render(&p, &[note], FS);
        assert_eq!(pcm.len(), (4_800 + 24_000 + 2_400) * 2);
        let x = mono(&pcm);
        assert!(x[..4_800].iter().all(|v| *v == 0.0), "silent before the note");
        // Count zero crossings over half a second of sustain: 440 Hz.
        let body = &x[9_600..9_600 + 24_000 / 2 * 2 / 2];
        let crossings = body.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((crossings as f64 - 440.0 * body.len() as f64 / FS).abs() <= 2.0, "{crossings}");
        assert!(x[4_800 + 24_000 + 2_399].abs() < 1e-3, "released by the end of the tail");
    }

    #[test]
    fn envelope_follows_adsr() {
        let mut p = SynthParams::default();
        for (id, v) in [("attack", 100.0), ("decay", 100.0), ("sustain", 50.0), ("release", 200.0)] { p.set(id, v).unwrap(); }
        assert!((p.envelope(0.05, 1.0) - 0.5).abs() < 1e-9);
        assert!((p.envelope(0.15, 1.0) - 0.75).abs() < 1e-9);
        assert!((p.envelope(0.5, 1.0) - 0.5).abs() < 1e-9);
        assert!((p.envelope(1.1, 1.0) - 0.25).abs() < 1e-9);
        assert_eq!(p.envelope(1.3, 1.0), 0.0);
    }

    #[test]
    fn stereo_width_decorrelates_and_presets_and_json_round_trip() {
        let p = SynthParams::preset("温暖铺底").unwrap();
        let pcm = render(&p, &[SynthNote { start: 0, length: 9_600, pitch: 60.0, velocity: 0.8 }], FS);
        let (l, r): (Vec<f32>, Vec<f32>) = pcm.chunks(2).map(|f| (f[0], f[1])).unzip();
        assert!(l.iter().zip(&r).any(|(a, b)| (a - b).abs() > 1e-3));
        assert!(peak(&l) < 1.0 && peak(&l) > 0.01);
        assert_eq!(SynthParams::from_json(Some(&p.to_json())), p);
        assert_eq!(SynthParams::from_json(Some(&serde_json::json!({"cutoff": -5, "nope": 1}))), SynthParams::default());
        for (name, _) in SYNTH_PRESETS { assert!(SynthParams::preset(name).is_some(), "{name}"); }
    }

    #[test]
    fn chords_sum_and_render_is_deterministic() {
        let p = SynthParams::default();
        let chord: Vec<SynthNote> = [60.0, 64.0, 67.0].iter().map(|&pitch| SynthNote { start: 0, length: 4_800, pitch, velocity: 0.7 }).collect();
        let a = render(&p, &chord, FS);
        assert_eq!(a, render(&p, &chord, FS));
        assert!(rms_db(&mono(&a)) > rms_db(&mono(&render(&p, &chord[..1], FS))));
    }

    #[test]
    fn appended_oscillators_noise_unison_and_modulation_reach_the_audio_path() {
        let note = [SynthNote { start: 0, length: 6000, pitch: 60.0, velocity: 0.8 }];
        let base = SynthParams::default();
        let reference = render(&base, &note, FS);
        for (id, value) in [("osc3_mix", 75.0), ("osc4_mix", 75.0), ("noise", 50.0), ("unison", 4.0),
            ("lfo_pitch", 1.0), ("lfo_filter", 36.0), ("filter_shape", 1.0), ("filter2_enabled", 1.0), ("pulse_width", 20.0), ("drive", 18.0)] {
            let mut p = base.clone();
            p.set(id, value).unwrap();
            let pcm = render(&p, &note, FS);
            assert!(pcm.iter().all(|sample| sample.is_finite()), "{id}");
            assert!(pcm.iter().zip(&reference).any(|(a, b)| (a - b).abs() > 1e-4), "{id} must affect real samples");
            assert_eq!(pcm, render(&p, &note, FS), "{id} deterministic offline playback");
        }
    }

    #[test]
    fn one_ignores_hidden_oscillators_and_has_its_own_defaults_and_presets() {
        let state = serde_json::json!({"osc2_mix": 100.0, "osc3_mix": 100.0, "osc4_mix": 100.0, "noise": 100.0, "unison": 8.0, "cutoff": 800.0});
        let one = SynthParams::from_json_for(ONE_SYNTH_ID, Some(&state));
        for id in ["osc2_mix", "osc3_mix", "osc4_mix", "width"] { assert_eq!(one.get(id), 0.0); }
        assert_eq!(one.get("noise"), 100.0);
        assert_eq!(one.get("voices"), 10.0);
        assert_eq!(one.get("unison"), 1.0);
        assert_eq!(one.get("cutoff"), 800.0);
        assert_eq!(SynthParams::from_json_for(ONE_SYNTH_ID, Some(&one.to_json())), one);
        for (name, _) in ONE_PRESETS { assert!(SynthParams::preset_for(ONE_SYNTH_ID, name).is_some()); }
        assert!(is_builtin_instrument(SYNTH_ID) && is_builtin_instrument(ONE_SYNTH_ID));
        assert!(!is_builtin_instrument("verdant.builtin.eq"));
    }

    #[test]
    fn legacy_parameter_indexes_are_unchanged_and_new_parameters_round_trip() {
        let original = ["osc1", "osc2", "osc2_semi", "detune", "osc2_mix", "attack", "decay", "sustain", "release", "cutoff", "resonance", "env_amount", "width", "gain"];
        assert_eq!(SYNTH_SPECS[..14].iter().map(|s| s.id).collect::<Vec<_>>(), original);
        let mut p = SynthParams::default();
        for spec in &SYNTH_SPECS[14..] { p.set(spec.id, spec.max).unwrap(); }
        assert_eq!(SynthParams::from_json(Some(&p.to_json())), p);
        // Exercise the maximal voice/FX controls for a bounded audio interval.
        // Full offline draining with 95% feedback can legitimately take minutes.
        let mut synth=super::super::RealtimeSynth::new(&p,FS);synth.note_on(0,120,120.0,1.0);
        let (mut l,mut r)=(vec![0.0;4096],vec![0.0;4096]);synth.process(&mut l,&mut r);
        assert!(l.iter().chain(&r).all(|sample|sample.is_finite()));assert!(peak(&l)<16.0&&peak(&r)<16.0);
        synth.release_all();synth.process(&mut l,&mut r);assert!(l.iter().chain(&r).all(|sample|sample.is_finite()));
    }

    #[test]
    fn voice_limit_steals_only_overlapping_voices_and_new_notes_still_sound() {
        let mut p = SynthParams::one_defaults();
        for (id, value) in [("voices", 1.0), ("osc1", 3.0), ("release", 10.0), ("cutoff", 20000.0), ("env_amount", 0.0)] { p.set(id, value).unwrap(); }
        let notes = [SynthNote { start: 0, length: 24000, pitch: 60.0, velocity: 0.8 },
            SynthNote { start: 12000, length: 12000, pitch: 72.0, velocity: 0.8 },
            SynthNote { start: 30000, length: 6000, pitch: 67.0, velocity: 0.8 }];
        let pcm = render(&p, &notes, FS);
        let newest_only = render(&p, &notes[1..], FS);
        assert_eq!(&pcm[13000 * 2..24000 * 2], &newest_only[13000 * 2..24000 * 2]);
        assert!(rms_db(&pcm[31000 * 2..35000 * 2]) > -30.0);
    }

    #[test]
    fn velocity_keytracking_and_portamento_have_independent_effects() {
        let notes = [SynthNote { start: 0, length: 4000, pitch: 48.0, velocity: 0.2 }, SynthNote { start: 5000, length: 5000, pitch: 72.0, velocity: 0.8 }];
        let reference = render(&SynthParams::one_defaults(), &notes, FS);
        for (id, value) in [("velocity", 0.0), ("keytrack", 100.0), ("portamento", 200.0)] {
            let mut p = SynthParams::one_defaults();
            p.set(id, value).unwrap();
            let pcm = render(&p, &notes, FS);
            assert!(pcm.iter().zip(&reference).any(|(a, b)| (a - b).abs() > 0.001), "{id}");
        }
        let specs = instrument_specs(ONE_SYNTH_ID);
        assert!(specs.iter().all(|spec| ONE_PARAM_IDS.contains(&spec.id)||super::super::modulation_parameter(spec.id).is_some()));
        assert_eq!(specs.iter().find(|spec| spec.id == "voices").unwrap().max, 10.0);
    }
}

pub use super::synth_live::{render, render_performance, SynthPerformanceNote, SynthExpression, RealtimeSynth, SYNTH_CONTROL_EVENT, SYNTH_PREVIEW_NOTE, SYNTH_PREVIEW_RELEASE, SYNTH_PREVIEW_PANIC, SYNTH_RESET_METERS};
