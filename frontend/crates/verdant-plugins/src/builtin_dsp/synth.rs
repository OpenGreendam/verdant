//! Built-in instrument: a polyphonic two-oscillator subtractive synth. An
//! instrument track renders its notes through it, the same way for playback
//! and export, so what plays is what exports.

use super::filters::db_to_lin;
use super::{table_defaults, table_index, Spec};
use std::f64::consts::{PI, TAU};

pub const SYNTH_ID: &str = "verdant.builtin.synth";

pub const WAVES: [&str; 4] = ["锯齿", "方波", "三角", "正弦"];

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
];

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
];

/// One note to play, in samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthNote { pub start: u64, pub length: u64, pub pitch: f64, pub velocity: f64 }

/// Validated synth settings: values in `SYNTH_SPECS` order.
#[derive(Clone, Debug, PartialEq)]
pub struct SynthParams { pub values: Vec<f64> }

impl Default for SynthParams { fn default() -> Self { Self { values: table_defaults(SYNTH_SPECS) } } }

impl SynthParams {
    pub fn get(&self, id: &str) -> f64 { table_index(SYNTH_SPECS, id).map_or(0.0, |i| self.values[i]) }
    pub fn set(&mut self, id: &str, value: f64) -> Result<(), String> {
        let i = table_index(SYNTH_SPECS, id).ok_or_else(|| format!("unknown synth parameter {id}"))?;
        SYNTH_SPECS[i].check(value)?;
        self.values[i] = value;
        Ok(())
    }
    /// From a `{"id": value}` object; unknown ids and bad values are ignored
    /// so an older or newer project still opens.
    pub fn from_json(value: Option<&serde_json::Value>) -> Self {
        let mut p = Self::default();
        if let Some(map) = value.and_then(serde_json::Value::as_object) {
            for (id, v) in map { if let Some(v) = v.as_f64() { let _ = p.set(id, v); } }
        }
        p
    }
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Object(SYNTH_SPECS.iter().zip(&self.values).map(|(s, v)| (s.id.to_string(), serde_json::json!(v))).collect())
    }
    pub fn preset(name: &str) -> Option<Self> {
        let (_, values) = SYNTH_PRESETS.iter().find(|(n, _)| *n == name)?;
        let mut p = Self::default();
        for (id, v) in values.iter() { p.set(id, *v).ok()?; }
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

fn osc(wave: usize, phase: f64, dt: f64) -> f64 {
    match wave {
        0 => (2.0 * phase - 1.0) - blep(phase, dt),
        1 => {
            let mut v = if phase < 0.5 { 1.0 } else { -1.0 };
            v += blep(phase, dt);
            v -= blep((phase + 0.5).fract(), dt);
            v
        }
        2 => 1.0 - 4.0 * (phase - 0.5).abs(),
        _ => (TAU * phase).sin(),
    }
}

/// Zero-delay-feedback state-variable low-pass (Simper).
#[derive(Clone, Copy, Default)]
struct Svf { ic1: f64, ic2: f64 }

impl Svf {
    #[inline]
    fn low(&mut self, x: f64, cutoff: f64, q: f64, sample_rate: f64) -> f64 {
        let g = (PI * cutoff.clamp(20.0, sample_rate * 0.45) / sample_rate).tan();
        let k = 1.0 / q.max(0.3);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let (a2, a3) = (g * a1, g * g * a1);
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v2
    }
}

/// Render `notes` to interleaved stereo PCM covering every note's release.
pub fn render(params: &SynthParams, notes: &[SynthNote], sample_rate: f64) -> Vec<f32> {
    let release = (params.release_seconds() * sample_rate).ceil() as u64;
    let frames = notes.iter().map(|n| n.start + n.length + release).max().unwrap_or(0) as usize;
    let mut out = vec![0.0f32; frames * 2];
    let (w1, w2) = (params.get("osc1").round() as usize, params.get("osc2").round() as usize);
    let semi = params.get("osc2_semi").round();
    let detune = params.get("detune");
    let mix2 = params.get("osc2_mix") / 100.0;
    let (cutoff, q, amount) = (params.get("cutoff"), params.get("resonance"), params.get("env_amount") / 100.0 * 4.0);
    let width = params.get("width") / 100.0;
    let gain = db_to_lin(params.get("gain"));
    for note in notes {
        let held = note.length as f64 / sample_rate;
        let base = 440.0 * 2f64.powf((note.pitch - 69.0) / 12.0);
        // Left and right detune in opposite directions; width scales it.
        let f1 = [base * 2f64.powf(detune * width / 2400.0), base * 2f64.powf(-detune * width / 2400.0)];
        let f2 = [base * 2f64.powf((semi * 100.0 - detune) / 1200.0), base * 2f64.powf((semi * 100.0 + detune) / 1200.0)];
        let mut phase = [[0.0f64; 2]; 2];
        // Decorrelate voices deterministically from their pitch and start.
        let seed = (note.pitch * 7.31 + note.start as f64 * 1e-5).fract();
        phase[1] = [seed, (seed + 0.37).fract()];
        let mut filters = [Svf::default(); 2];
        let end = (note.start + note.length + release) as usize;
        for i in note.start as usize..end.min(frames) {
            let t = (i - note.start as usize) as f64 / sample_rate;
            let env = params.envelope(t, held);
            if t > held && env <= 0.0 { break; }
            let cut = cutoff * 2f64.powf(amount * env);
            for c in 0..2 {
                let (d1, d2) = (f1[c] / sample_rate, f2[c] / sample_rate);
                let s = osc(w1, phase[0][c], d1) * (1.0 - mix2 * 0.5) + osc(w2, phase[1][c], d2) * mix2;
                phase[0][c] = (phase[0][c] + d1).fract();
                phase[1][c] = (phase[1][c] + d2).fract();
                let y = filters[c].low(s, cut, q, sample_rate) * env * note.velocity * gain;
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
}
