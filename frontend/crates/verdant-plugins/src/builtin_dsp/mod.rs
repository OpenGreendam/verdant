//! DSP of Verdant's built-in mixing suite and the curves their editors draw.
//! The host process runs the processors; the editor evaluates the same
//! coefficient and gain-computer functions, so the picture is the sound.
//!
//! | id | effect |
//! |----|--------|
//! | `verdant.builtin.eq` | 24-band dynamic EQ: nine shapes, 12–48 dB/oct cuts, stereo/L/R/M/S per band |
//! | `verdant.builtin.compressor` | compressor: styles, lookahead, range, external sidechain, key filter |
//! | `verdant.builtin.gate` | gate/expander: six styles, hold, hysteresis, lookahead, external key |
//! | `verdant.builtin.deesser` | de-esser: split or wide-band reduction above a frequency |
//! | `verdant.builtin.limiter` | lookahead brickwall limiter |
//! | `verdant.builtin.reverb` | eight-line FDN reverb with predelay, damping and post EQ |
//! | `verdant.builtin.delay` | stereo/ping-pong delay with filtered, saturated feedback |
//! | `verdant.builtin.saturation` | six-band saturation with eight styles, feedback and modulation |
//! | `verdant.builtin.multiband` | six-band upward/downward compression and expansion |
//! | `verdant.builtin.filter` | four modulated filters with serial/parallel routing |
//! | `verdant.builtin.simplon` | dual filter with serial/parallel routing |
//! | `verdant.builtin.micro` | envelope-controlled low/high-pass filter |
//!
//! Parameters are plain values in their own units (Hz, dB, ms, %, ratio).
//! A parameter's position in `descs()` is its automation index and never
//! changes; new parameters are only appended.

pub mod filters;
mod eq;
mod dynamics;
mod space;
mod color;
mod synth;
mod multiband;
mod filter_fx;

pub use eq::*;
pub use dynamics::*;
pub use space::*;
pub use color::*;
pub use synth::*;
pub use multiband::*;
pub use filter_fx::*;
pub use filters::{db_to_lin, lin_to_db};

use crate::proto::ParamDesc;

pub const EQ_ID: &str = "verdant.builtin.eq";
pub const COMPRESSOR_ID: &str = "verdant.builtin.compressor";
pub const GATE_ID: &str = "verdant.builtin.gate";
pub const DEESSER_ID: &str = "verdant.builtin.deesser";
pub const LIMITER_ID: &str = "verdant.builtin.limiter";
pub const REVERB_ID: &str = "verdant.builtin.reverb";
pub const DELAY_ID: &str = "verdant.builtin.delay";
pub const SATURATION_ID: &str = "verdant.builtin.saturation";
pub const MULTIBAND_ID: &str = "verdant.builtin.multiband";
pub const FILTER_ID: &str = "verdant.builtin.filter";
pub const SIMPLON_ID: &str = "verdant.builtin.simplon";
pub const MICRO_ID: &str = "verdant.builtin.micro";

/// Private built-in event, below the note-event namespace. Compiled from the
/// project tempo map and delivered at sample offsets through the existing
/// automation transport; it is never a saved/user parameter or a foreign
/// plug-in's native parameter id.
pub const TRANSPORT_TEMPO_EVENT: u32 = 0x7fff_fffe;

pub fn uses_transport_tempo(format: &str, id: &str) -> bool {
    format == "builtin" && id == DELAY_ID
}

/// Built-in effect ids other than the diagnostic gain, in browser order.
pub const EFFECT_IDS: [&str; 12] = [EQ_ID, COMPRESSOR_ID, GATE_ID, DEESSER_ID, LIMITER_ID, REVERB_ID, DELAY_ID, SATURATION_ID,
    MULTIBAND_ID, FILTER_ID, SIMPLON_ID, MICRO_ID];

/// Editors for built-in processors are drawn by the desktop, independently
/// of the child host's external plug-in window capability.
pub fn has_native_editor(id: &str) -> bool {
    EFFECT_IDS.contains(&id) || id == crate::BUILTIN_GAIN_ID
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorKind { Builtin, Hosted, Parameters }

/// The default editor is a capability decision, not a user-facing extra
/// switch. A third-party id cannot impersonate an internal editor.
pub fn preferred_editor(format: &str, id: &str, host_has_editor: bool) -> EditorKind {
    if format == "builtin" && has_native_editor(id) { EditorKind::Builtin }
    else if host_has_editor { EditorKind::Hosted }
    else { EditorKind::Parameters }
}

pub fn display_name(id: &str) -> Option<&'static str> {
    Some(match id {
        EQ_ID => "Verdant EQ",
        COMPRESSOR_ID => "Verdant Compressor",
        GATE_ID => "Verdant Gate",
        DEESSER_ID => "Verdant De-esser",
        LIMITER_ID => "Verdant Limiter",
        REVERB_ID => "Verdant Reverb",
        DELAY_ID => "Verdant Delay",
        SATURATION_ID => "Verdant Saturation",
        MULTIBAND_ID => "Verdant Multiband",
        FILTER_ID => "Verdant Filter",
        SIMPLON_ID => "Verdant Dual Filter",
        MICRO_ID => "Verdant Micro Filter",
        _ => return None,
    })
}

/// Short localized label for device headers and the add menu.
pub fn short_label(id: &str) -> Option<&'static str> {
    Some(match id {
        EQ_ID => "均衡器",
        COMPRESSOR_ID => "压缩器",
        GATE_ID => "门限",
        DEESSER_ID => "齿音消除",
        LIMITER_ID => "限制器",
        REVERB_ID => "混响",
        DELAY_ID => "延迟",
        SATURATION_ID => "饱和",
        MULTIBAND_ID => "多段动态",
        FILTER_ID => "调制滤波器",
        SIMPLON_ID => "双滤波器",
        MICRO_ID => "微型滤波器",
        _ => return None,
    })
}

/// Whether the effect reads the sidechain key when one is routed to it.
pub fn takes_sidechain(id: &str) -> bool {
    matches!(id, EQ_ID | COMPRESSOR_ID | GATE_ID | DEESSER_ID | LIMITER_ID | REVERB_ID | DELAY_ID | SATURATION_ID | MULTIBAND_ID | FILTER_ID | SIMPLON_ID | MICRO_ID)
}

fn desc(id: impl Into<String>, name: impl Into<String>, value: f64, min: f64, max: f64, unit: &str, steps: i64) -> ParamDesc {
    ParamDesc { id: id.into(), name: name.into(), value, min, max, unit: unit.into(), step_count: steps, default_value: Some(value) }
}

// ---------------------------------------------------------------------------
// Parameter tables
// ---------------------------------------------------------------------------

/// One parameter of a table-driven effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub unit: &'static str,
    /// Discrete steps (0 = continuous).
    pub steps: i64,
    /// Latency-defining parameters are fixed during playback and export.
    pub automatable: bool,
    /// Displayed on a logarithmic scale (frequencies, times).
    pub log: bool,
}

impl Spec {
    pub const fn new(id: &'static str, name: &'static str, min: f64, max: f64, default: f64, unit: &'static str) -> Self {
        Self { id, name, min, max, default, unit, steps: 0, automatable: true, log: false }
    }
    pub const fn steps(mut self, steps: i64) -> Self { self.steps = steps; self }
    pub const fn fixed(mut self) -> Self { self.automatable = false; self }
    pub const fn log(mut self) -> Self { self.log = true; self }
    pub fn check(&self, value: f64) -> Result<(), String> {
        if value.is_finite() && (self.min..=self.max).contains(&value) { Ok(()) }
        else { Err(format!("{} outside {}…{}", self.id, self.min, self.max)) }
    }
}

pub fn table_defaults(specs: &[Spec]) -> Vec<f64> { specs.iter().map(|s| s.default).collect() }

pub fn table_descs(specs: &[Spec], values: &[f64]) -> Vec<ParamDesc> {
    specs.iter().zip(values).map(|(s, &v)| {
        let mut d = desc(s.id, s.name, v, s.min, s.max, s.unit, s.steps);
        d.default_value = Some(s.default);
        d
    }).collect()
}

pub fn table_index(specs: &[Spec], id: &str) -> Option<usize> { specs.iter().position(|s| s.id == id) }

/// Look a spec up by id across every built-in effect (editor formatting).
pub fn spec_for(effect: &str, id: &str) -> Option<Spec> {
    let specs: &[Spec] = match effect {
        COMPRESSOR_ID => COMP_SPECS,
        GATE_ID => GATE_SPECS,
        DEESSER_ID => DEESSER_SPECS,
        LIMITER_ID => LIMITER_SPECS,
        REVERB_ID => REVERB_SPECS,
        DELAY_ID => DELAY_SPECS,
        SATURATION_ID => SATURATION_SPECS,
        MULTIBAND_ID => MULTIBAND_SPECS,
        FILTER_ID => FILTER_SPECS,
        SIMPLON_ID => SIMPLON_SPECS,
        MICRO_ID => MICRO_SPECS,
        EQ_ID => return eq_spec(id),
        _ => return None,
    };
    specs.iter().find(|s| s.id == id).copied()
}

/// A table-driven effect: values live in one array ordered like `SPECS`.
pub trait TableEffect {
    const SPECS: &'static [Spec];
    fn values(&self) -> &[f64];
    fn values_mut(&mut self) -> &mut [f64];
    /// Recompute derived coefficients after a value change.
    fn refresh(&mut self);
    fn set_sample_rate(&mut self, sample_rate: f64);
    fn uses_transport_tempo() -> bool { false }
    fn set_transport_tempo(&mut self, _bpm: f64) {}
    /// Fill `out_*` from `in_*`; `key` is the routed sidechain, if any.
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]);
    /// Processing latency in samples.
    fn latency(&self) -> u32 { 0 }

    fn descs(&self) -> Vec<ParamDesc> { table_descs(Self::SPECS, self.values()) }
    fn value(&self, id: &str) -> f64 { table_index(Self::SPECS, id).map_or(0.0, |i| self.values()[i]) }
    fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> {
        Self::SPECS.get(index).ok_or("unknown parameter")?.check(value)?;
        self.values_mut()[index] = value;
        self.refresh();
        Ok(())
    }
    fn set(&mut self, id: &str, value: f64) -> Result<(), String> {
        self.set_index(table_index(Self::SPECS, id).ok_or_else(|| format!("unknown parameter {id}"))?, value)
    }
}

/// Built-in effect parameter state, serialized as `{"id": value, …}`.
pub fn state_json(descs: &[ParamDesc]) -> Vec<u8> {
    let map: serde_json::Map<String, serde_json::Value> = descs.iter().map(|d| (d.id.clone(), serde_json::json!(d.value))).collect();
    serde_json::to_vec(&serde_json::Value::Object(map)).expect("serializable state")
}

/// Parse state written by `state_json`. Unknown ids are refused by callers;
/// ids missing from older state keep their current values.
pub fn parse_state(data: &[u8]) -> Result<Vec<(String, f64)>, String> {
    let value: serde_json::Value = serde_json::from_slice(data).map_err(|e| format!("bad state: {e}"))?;
    let map = value.as_object().ok_or("state must be an object")?;
    map.iter().map(|(k, v)| v.as_f64().map(|v| (k.clone(), v)).ok_or_else(|| format!("state value {k} is not a number"))).collect()
}

#[cfg(test)]
pub(crate) mod test_util {
    pub const FS: f64 = 48_000.0;
    pub fn sine(freq: f64, amp: f64, n: usize) -> Vec<f32> {
        (0..n).map(|i| (amp * (std::f64::consts::TAU * freq * i as f64 / FS).sin()) as f32).collect()
    }
    pub fn rms_db(x: &[f32]) -> f64 {
        let s = x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64;
        10.0 * s.max(1e-30).log10()
    }
    pub fn peak(x: &[f32]) -> f32 { x.iter().fold(0.0f32, |m, v| m.max(v.abs())) }
}
