//! In-host diagnostic plugin (`verdant.builtin.gain`): real DSP, real params
//! and state — the bring-up path per PLUGIN-HOST.md step 1, and the
//! integration test's way to prove the whole pipe without external plugins.

use super::{HostedPlugin, ScanOutcome};
use verdant_plugins::proto::{AutomationParam, ParamDesc, ParamEvent, ParamTarget};
use verdant_plugins::proto::ScannedPlugin;
use std::path::Path;

/// Real in-host gain: stereo `x * g`. `gain_db` is a real parameter and
/// `state()` round-trips it so the whole control path is exercised.
pub struct Gain {
    /// Linear gain applied each block (host worker only).
    gain: f32,
    /// The parameter's display value in dB.
    gain_db: f64,
    sample_rate: f64,
    max_frames: u32,
}

impl Gain {
    pub fn new() -> Self {
        Self {
            gain: 1.0,
            gain_db: 0.0,
            sample_rate: 48_000.0,
            max_frames: 8192,
        }
    }

    fn db_to_lin(db: f64) -> f32 {
        (10.0f64.powf(db / 20.0)) as f32
    }
}

impl HostedPlugin for Gain {
    fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) -> Result<(), String> {
        let g = self.gain;
        for i in 0..in_l.len() {
            out_l[i] = in_l[i] * g;
            out_r[i] = in_r[i] * g;
        }
        Ok(())
    }

    fn process_events(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32],
        events: &[ParamEvent]) -> Result<(), String> {
        if events.is_empty() { return self.process(in_l, in_r, out_l, out_r); }
        if in_l.len() != in_r.len() || in_l.len() != out_l.len() || in_l.len() != out_r.len()
            || events.len() > verdant_plugins::MAX_PARAM_EVENTS {
            return Err("invalid builtin automation block".into());
        }
        let mut previous = 0;
        for &(id, offset, value) in events {
            if id != 0 || offset as usize >= in_l.len() || offset < previous {
                return Err("invalid builtin native parameter event".into());
            }
            if !value.is_finite() || !(-60.0..=24.0).contains(&value) {
                return Err("builtin event outside finite native bounds".into());
            }
            previous = offset;
        }
        let mut next = 0;
        for i in 0..in_l.len() {
            while next < events.len() && events[next].1 as usize == i {
                self.gain_db = events[next].2;
                self.gain = Self::db_to_lin(self.gain_db);
                next += 1;
            }
            out_l[i] = in_l[i] * self.gain;
            out_r[i] = in_r[i] * self.gain;
        }
        Ok(())
    }

    fn automation_params(&mut self) -> Result<Vec<AutomationParam>, String> {
        let desc = self.params()?.remove(0);
        let target = ParamTarget::from_desc(0, &desc).map_err(|e| e.to_string())?;
        Ok(vec![AutomationParam { id: desc.id, target }])
    }

    fn params(&mut self) -> Result<Vec<ParamDesc>, String> {
        Ok(vec![ParamDesc {
            id: "gain_db".into(),
            name: "Gain (dB)".into(),
            value: self.gain_db,
            min: -60.0,
            max: 24.0,
            unit: "dB".into(),
            step_count: 0,
            default_value: Some(0.0),
        }])
    }

    fn set_param(&mut self, id: &str, value: f64) -> Result<(), String> {
        match id {
            "gain_db" => {
                if !value.is_finite() || !(-60.0..=24.0).contains(&value) {
                    return Err("builtin parameter outside finite native bounds".into());
                }
                self.gain_db = value;
                self.gain = Self::db_to_lin(self.gain_db);
                Ok(())
            }
            _ => Err(format!("unknown param {id}")),
        }
    }

    fn state(&mut self) -> Result<Vec<u8>, String> {
        Ok(format!("{}", self.gain_db).into_bytes())
    }

    fn set_state(&mut self, data: &[u8]) -> Result<(), String> {
        let s = std::str::from_utf8(data).map_err(|e| e.to_string())?;
        let db: f64 = s.trim().parse().map_err(|e| format!("bad state: {e}"))?;
        self.set_param("gain_db", db)
    }

    fn reconfigure(&mut self, sample_rate: f64, max_frames: u32) -> Result<(), String> {
        self.sample_rate = sample_rate;
        self.max_frames = max_frames;
        Ok(())
    }
    fn set_offline(&mut self, _enabled: bool) -> Result<(), String> { Ok(()) }
}

// ---------------------------------------------------------------------------
// Built-in mixing effects (EQ, compressor): DSP lives in
// `verdant_plugins::builtin_dsp`, shared with the editor's curves.
// ---------------------------------------------------------------------------

use verdant_plugins::builtin_dsp::{self as dsp, CompProcessor, DeesserProcessor, DelayProcessor, EqParams, EqProcessor,
    GateProcessor, LimiterProcessor, ReverbProcessor, SaturationProcessor, TableEffect};

/// What a built-in effect provides to the generic host adapter.
pub trait Effect {
    /// Automation indices and bounds remain fixed for the effect's lifetime.
    fn descs(&self) -> Vec<ParamDesc>;
    fn index_of(id: &str) -> Option<usize>;
    fn set_index(&mut self, index: usize, value: f64) -> Result<(), String>;
    fn run(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]);
    fn set_sample_rate(&mut self, sample_rate: f64);
    fn latency(&self) -> u32 { 0 }
    /// Latency-defining parameters are not offered for automation.
    fn automatable(&self, _index: usize) -> bool { true }
}

impl Effect for EqProcessor {
    fn descs(&self) -> Vec<ParamDesc> { self.params.descs() }
    fn index_of(id: &str) -> Option<usize> { EqParams::index_of(id) }
    fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> { EqProcessor::set_index(self, index, value) }
    fn run(&mut self, in_l: &[f32], in_r: &[f32], _key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) { self.process(in_l, in_r, out_l, out_r) }
    fn set_sample_rate(&mut self, sample_rate: f64) { EqProcessor::set_sample_rate(self, sample_rate) }
}

/// Adapter for every table-driven effect.
pub struct Table<T: TableEffect>(pub T);

impl<T: TableEffect> Effect for Table<T> {
    fn descs(&self) -> Vec<ParamDesc> { self.0.descs() }
    fn index_of(id: &str) -> Option<usize> { dsp::table_index(T::SPECS, id) }
    fn set_index(&mut self, index: usize, value: f64) -> Result<(), String> { self.0.set_index(index, value) }
    fn run(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) { self.0.process(in_l, in_r, key, out_l, out_r) }
    fn set_sample_rate(&mut self, sample_rate: f64) { self.0.set_sample_rate(sample_rate) }
    fn latency(&self) -> u32 { self.0.latency() }
    fn automatable(&self, index: usize) -> bool { T::SPECS.get(index).is_some_and(|s| s.automatable) }
}

/// Generic host adapter: parameters by id, automation by stable index,
/// sample-accurate events and JSON state.
pub struct Builtin<E: Effect> {
    fx: E,
    /// Cache immutable validation metadata before entering the audio path.
    /// Current values still come from the effect for parameter/state queries.
    validation_descs: Vec<ParamDesc>,
}

impl<E: Effect> Builtin<E> {
    pub fn new(fx: E) -> Self {
        let validation_descs = fx.descs();
        Self { fx, validation_descs }
    }

    fn process_with(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32],
        events: &[ParamEvent]) -> Result<(), String> {
        if events.is_empty() {
            if in_l.len() != in_r.len() || in_l.len() != out_l.len() || in_l.len() != out_r.len() { return Err("invalid builtin block".into()); }
            self.fx.run(in_l, in_r, key, out_l, out_r);
            return Ok(());
        }
        if in_l.len() != in_r.len() || in_l.len() != out_l.len() || in_l.len() != out_r.len()
            || events.len() > verdant_plugins::MAX_PARAM_EVENTS {
            return Err("invalid builtin automation block".into());
        }
        // Validate everything first: a bad last event must not leave partial state.
        let mut previous = 0;
        for &(_, offset, _) in events {
            if offset as usize >= in_l.len() || offset < previous { return Err("invalid builtin native parameter event".into()); }
            previous = offset;
        }
        self.validate(events.iter().map(|&(id, _, value)| (id as usize, value)))?;
        let mut start = 0usize;
        let mut next = 0usize;
        while start < in_l.len() {
            while next < events.len() && events[next].1 as usize == start {
                self.fx.set_index(events[next].0 as usize, events[next].2)?;
                next += 1;
            }
            let end = events.get(next).map_or(in_l.len(), |e| e.1 as usize);
            let key = key.map(|(kl, kr)| (&kl[start..end], &kr[start..end]));
            self.fx.run(&in_l[start..end], &in_r[start..end], key, &mut out_l[start..end], &mut out_r[start..end]);
            start = end;
        }
        Ok(())
    }

    /// Check a whole set of (index, value) writes before applying any.
    fn validate(&self, writes: impl IntoIterator<Item = (usize, f64)>) -> Result<(), String> {
        for (index, value) in writes {
            let d = self.validation_descs.get(index).ok_or("unknown builtin parameter")?;
            if !value.is_finite() || !(d.min..=d.max).contains(&value) {
                return Err(format!("{} outside {}…{}", d.id, d.min, d.max));
            }
        }
        Ok(())
    }
}

impl<E: Effect> HostedPlugin for Builtin<E> {
    fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) -> Result<(), String> {
        if in_l.len() != in_r.len() || in_l.len() != out_l.len() || in_l.len() != out_r.len() {
            return Err("invalid builtin block".into());
        }
        self.fx.run(in_l, in_r, None, out_l, out_r);
        Ok(())
    }

    fn process_events(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32],
        events: &[ParamEvent]) -> Result<(), String> {
        self.process_with(in_l, in_r, None, out_l, out_r, events)
    }

    fn process_keyed(&mut self, in_l: &[f32], in_r: &[f32], key: (&[f32], &[f32]), out_l: &mut [f32], out_r: &mut [f32],
        events: &[ParamEvent]) -> Result<(), String> {
        if key.0.len() != in_l.len() || key.1.len() != in_l.len() { return Err("invalid builtin sidechain block".into()); }
        self.process_with(in_l, in_r, Some(key), out_l, out_r, events)
    }

    fn latency(&mut self) -> u64 { self.fx.latency() as u64 }

    fn automation_params(&mut self) -> Result<Vec<AutomationParam>, String> {
        self.fx.descs().into_iter().enumerate().filter(|(i, _)| self.fx.automatable(*i)).map(|(i, d)| {
            let target = ParamTarget::from_desc(i as u32, &d).map_err(|e| e.to_string())?;
            Ok(AutomationParam { id: d.id, target })
        }).collect()
    }

    fn params(&mut self) -> Result<Vec<ParamDesc>, String> { Ok(self.fx.descs()) }

    fn set_param(&mut self, id: &str, value: f64) -> Result<(), String> {
        let index = E::index_of(id).ok_or_else(|| format!("unknown param {id}"))?;
        self.validate([(index, value)])?;
        self.fx.set_index(index, value)
    }

    fn state(&mut self) -> Result<Vec<u8>, String> { Ok(dsp::state_json(&self.fx.descs())) }

    fn set_state(&mut self, data: &[u8]) -> Result<(), String> {
        let writes = dsp::parse_state(data)?.into_iter()
            .map(|(id, v)| E::index_of(&id).map(|i| (i, v)).ok_or_else(|| format!("unknown param {id} in state")))
            .collect::<Result<Vec<_>, String>>()?;
        self.validate(writes.iter().copied())?;
        for (i, v) in writes { self.fx.set_index(i, v)?; }
        Ok(())
    }

    fn reconfigure(&mut self, sample_rate: f64, _max_frames: u32) -> Result<(), String> {
        self.fx.set_sample_rate(sample_rate);
        Ok(())
    }
    fn set_offline(&mut self, _enabled: bool) -> Result<(), String> { Ok(()) }
}

/// Instantiate a built-in mixing effect by id.
pub fn effect(id: &str, sample_rate: f64) -> Option<Box<dyn HostedPlugin>> {
    match id {
        dsp::EQ_ID => Some(Box::new(Builtin::new(EqProcessor::new(sample_rate)))),
        dsp::COMPRESSOR_ID => Some(Box::new(Builtin::new(Table(CompProcessor::new(sample_rate))))),
        dsp::GATE_ID => Some(Box::new(Builtin::new(Table(GateProcessor::new(sample_rate))))),
        dsp::DEESSER_ID => Some(Box::new(Builtin::new(Table(DeesserProcessor::new(sample_rate))))),
        dsp::LIMITER_ID => Some(Box::new(Builtin::new(Table(LimiterProcessor::new(sample_rate))))),
        dsp::REVERB_ID => Some(Box::new(Builtin::new(Table(ReverbProcessor::new(sample_rate))))),
        dsp::DELAY_ID => Some(Box::new(Builtin::new(Table(DelayProcessor::new(sample_rate))))),
        dsp::SATURATION_ID => Some(Box::new(Builtin::new(Table(SaturationProcessor::new(sample_rate))))),
        _ => None,
    }
}

/// Catalog entries for every built-in mixing effect.
pub fn effect_descriptors() -> Vec<ScannedPlugin> {
    dsp::EFFECT_IDS.iter().map(|id| ScannedPlugin {
        format: "builtin".into(),
        id: (*id).into(),
        name: dsp::display_name(id).unwrap_or(id).into(),
        vendor: "Verdant".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        url: String::new(),
        features: vec!["audio-effect".into()],
        has_editor: false,
    }).collect()
}

/// `scan-file` on `builtin:` URIs is meaningless — real files only.
pub fn scan(_path: &Path) -> Result<ScanOutcome, verdant_plugins::error::PluginError> {
    Err(verdant_plugins::error::PluginError::Host(
        "builtin plugins are not scannable files".into(),
    ))
}

/// The descriptor the client always exposes for the builtin.
pub fn descriptor() -> ScannedPlugin {
    ScannedPlugin {
        format: "builtin".into(),
        id: verdant_plugins::BUILTIN_GAIN_ID.into(),
        name: "Verdant Gain (builtin)".into(),
        vendor: "Verdant".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        url: String::new(),
        features: vec!["audio-effect".into()],
        has_editor: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_plugins::shm::Ring;

    #[test]
    fn shared_quantum_events_reach_native_dsp_at_exact_sample_offsets() {
        let mut gain = Gain::new();
        let target = gain.automation_params().unwrap().remove(0).target;
        let events = [target.event(0, -6.0).unwrap(), target.event(3, 6.0).unwrap(), target.event(7, 0.0).unwrap()];
        let ring = Ring::create(8).unwrap();
        let input_l = [0.5; 8];
        let input_r = [-0.25; 8];
        ring.try_write_input_events(0, &input_l, &input_r, &events).unwrap();
        let mut il = [0.0; 8];
        let mut ir = [0.0; 8];
        let mut received = [(0, 0, 0.0); 3];
        assert_eq!(ring.read_input_events(0, &mut il, &mut ir, &mut received).unwrap(), (8, 3));
        let mut ol = [0.0; 8];
        let mut orr = [0.0; 8];
        gain.process_events(&il, &ir, &mut ol, &mut orr, &received).unwrap();
        ring.write_output(0, &ol, &orr).unwrap();
        assert!(ring.try_read_output(0, &mut il, &mut ir));
        for sample in 0..8 {
            let db = if sample < 3 { -6.0 } else if sample < 7 { 6.0 } else { 0.0 };
            let multiplier = Gain::db_to_lin(db);
            assert_eq!(il[sample], input_l[sample] * multiplier);
            assert_eq!(ir[sample], input_r[sample] * multiplier);
        }
        // Scheduled values are the plugin's real state, not an output-only shim.
        assert_eq!(gain.state().unwrap(), b"0");
        gain.process(&[1.0], &[1.0], &mut ol[..1], &mut orr[..1]).unwrap();
        assert_eq!(ol[0], 1.0);
    }

    #[test]
    fn invalid_last_event_cannot_partially_change_native_state_or_output() {
        let mut gain = Gain::new();
        let mut l = [7.0; 8];
        let mut r = [8.0; 8];
        assert!(gain.process_events(&[1.0; 8], &[1.0; 8], &mut l, &mut r,
            &[(0, 0, -12.0), (0, 8, 6.0)]).is_err());
        assert_eq!(gain.gain_db, 0.0);
        assert_eq!(l, [7.0; 8]);
        assert_eq!(r, [8.0; 8]);
    }

    #[test]
    fn builtin_effect_automation_preserves_offsets_and_equal_offset_order() {
        let mut processor = EqProcessor::new(48_000.0);
        for band in &mut processor.params.bands { band.on = false; }
        let mut eq = Builtin::new(processor);
        let output = EqParams::index_of("output").unwrap() as u32;
        let input_l = [0.125; 8];
        let input_r = [-0.25; 8];
        let mut l = [0.0; 8];
        let mut r = [0.0; 8];
        // Leave a prefix untouched, preserve the order of coincident writes,
        // and apply the last event to the final sample of the block.
        eq.process_events(&input_l, &input_r, &mut l, &mut r,
            &[(output, 2, -6.0), (output, 5, -12.0), (output, 5, 6.0), (output, 7, 0.0)]).unwrap();
        for sample in 0..8 {
            let db = if sample < 2 { 0.0 } else if sample < 5 { -6.0 } else if sample < 7 { 6.0 } else { 0.0 };
            let multiplier = 10.0f64.powf(db / 20.0);
            assert_eq!(l[sample], (input_l[sample] as f64 * multiplier) as f32);
            assert_eq!(r[sample], (input_r[sample] as f64 * multiplier) as f32);
        }
        assert_eq!(eq.params().unwrap()[output as usize].value, 0.0);
        eq.process_events(&input_l, &input_r, &mut l, &mut r, &[]).unwrap();
        assert_eq!(l, input_l);
        assert_eq!(r, input_r);
    }

    #[test]
    fn builtin_effect_validation_rejects_entire_automation_block() {
        let effects: [Box<dyn HostedPlugin>; 2] = [
            Box::new(Builtin::new(EqProcessor::new(48_000.0))),
            Box::new(Builtin::new(Table(CompProcessor::new(48_000.0)))),
        ];
        for mut fx in effects {
            let before = fx.params().unwrap();
            let min = before[0].min;
            let max = before[0].max;
            let valid = if before[0].value == min { max } else { min };
            let invalid_blocks = [
                [(0, 0, valid), (u32::MAX, 4, min)],
                [(0, 0, valid), (0, 4, max + 1.0)],
                [(0, 0, valid), (0, 4, f64::NAN)],
                [(0, 0, valid), (0, 4, f64::INFINITY)],
                [(0, 4, valid), (0, 3, min)],
                [(0, 0, valid), (0, 8, min)],
            ];
            for events in invalid_blocks {
                let mut l = [7.0; 8];
                let mut r = [8.0; 8];
                assert!(fx.process_events(&[0.1; 8], &[-0.2; 8], &mut l, &mut r, &events).is_err());
                assert_eq!(fx.params().unwrap(), before);
                assert_eq!(l, [7.0; 8]);
                assert_eq!(r, [8.0; 8]);
            }
        }
    }

    #[test]
    fn builtin_effects_take_params_automation_and_state_atomically() {
        let mut eq = effect(dsp::EQ_ID, 48_000.0).unwrap();
        let params = eq.params().unwrap();
        assert_eq!(params.len(), EqParams::param_count());
        eq.set_param("b4_gain", 6.0).unwrap();
        assert!(eq.set_param("b4_gain", 60.0).is_err());
        let state = eq.state().unwrap();
        let mut copy = effect(dsp::EQ_ID, 48_000.0).unwrap();
        copy.set_state(&state).unwrap();
        assert_eq!(copy.params().unwrap(), eq.params().unwrap());
        // A bad event anywhere leaves output and state untouched.
        let targets = eq.automation_params().unwrap();
        let gain = targets.iter().position(|p| p.id == "b4_gain").unwrap() as u32;
        let mut l = [7.0; 8];
        let mut r = [7.0; 8];
        assert!(eq.process_events(&[0.1; 8], &[0.1; 8], &mut l, &mut r, &[(gain, 0, -3.0), (gain, 4, 99.0)]).is_err());
        assert_eq!(l, [7.0; 8]);
        assert_eq!(eq.params().unwrap()[gain as usize].value, 6.0);
        eq.process_events(&[0.1; 8], &[0.1; 8], &mut l, &mut r, &[(gain, 0, -3.0), (gain, 4, 2.0)]).unwrap();
        assert_eq!(eq.params().unwrap()[gain as usize].value, 2.0);
        let mut comp = effect(dsp::COMPRESSOR_ID, 48_000.0).unwrap();
        comp.set_param("ratio", 8.0).unwrap();
        assert!(comp.set_state(br#"{"ratio": 0.5}"#).is_err(), "out of range state refused");
        assert!(comp.set_state(br#"{"nope": 1}"#).is_err(), "unknown id refused");
        assert_eq!(comp.params().unwrap()[1].value, 8.0);
        assert!(effect("verdant.builtin.unknown", 48_000.0).is_none());
        assert_eq!(effect_descriptors().len(), dsp::EFFECT_IDS.len());
    }

    #[test]
    fn keyed_builtins_hear_the_sidechain_and_report_latency() {
        let mut comp = effect(dsp::COMPRESSOR_ID, 48_000.0).unwrap();
        for (id, v) in [("threshold", -30.0), ("ratio", 10.0), ("knee", 0.0)] { comp.set_param(id, v).unwrap(); }
        let n = 24_000;
        let music: Vec<f32> = (0..n).map(|i| 0.02 * (i as f32 * 0.03).sin()).collect();
        let kick: Vec<f32> = (0..n).map(|i| (i as f32 * 0.008).sin()).collect();
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        comp.process_keyed(&music, &music, (&kick, &kick), &mut l, &mut r, &[]).unwrap();
        let energy = |x: &[f32]| x[12_000..].iter().map(|v| v * v).sum::<f32>();
        assert!(energy(&l) < energy(&music) * 0.1, "the key ducks the music");
        assert_eq!(comp.latency(), 0);
        comp.set_param("lookahead", 5.0).unwrap();
        assert_eq!(comp.latency(), 240);
        assert!(!comp.automation_params().unwrap().iter().any(|p| p.id == "lookahead"));
        for id in dsp::EFFECT_IDS {
            let mut fx = effect(id, 48_000.0).unwrap();
            let state = fx.state().unwrap();
            fx.set_state(&state).unwrap();
            let (mut l, mut r) = ([0.0; 64], [0.0; 64]);
            fx.process(&[0.1; 64], &[0.1; 64], &mut l, &mut r).unwrap();
            assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{id}");
        }
    }
}
