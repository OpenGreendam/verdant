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
}
