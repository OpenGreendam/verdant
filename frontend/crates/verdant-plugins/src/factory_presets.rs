//! Shared, versioned factory library and parameter admission for native devices.
//! Recipes are original Verdant settings. Public presets always contain a full
//! default state plus overrides, so applying one cannot inherit hidden settings.
use std::{collections::{BTreeMap, BTreeSet}, sync::OnceLock};
use serde::{Deserialize, Serialize};
use crate::{builtin_dsp::*, ParamDesc};

pub const LIBRARY_VERSION: &str = "1.0.0";
pub const LIBRARY_SCHEMA: &str = "verdant-factory-presets/1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactoryPreset {
    pub id: String,
    pub device: String,
    pub name: String,
    pub category: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub parameters: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Library { schema: String, version: String, presets: Vec<FactoryPreset> }

/// Descriptors have the same order, units, defaults and native step grids as
/// the actual host. No child process, GUI, audio device or DSP buffer is needed.
pub fn parameter_descriptors(device: &str) -> Option<Vec<ParamDesc>> {
    if is_builtin_instrument(device) {
        let specs = instrument_specs(device);
        let defaults = SynthParams::from_json_for(device, None);
        return Some(table_descs(&specs, &specs.iter().map(|s| defaults.get(s.id)).collect::<Vec<_>>()));
    }
    let mut descriptors = if device == EQ_ID {
        EqParams::default().descs()
    } else {
        let specs = match device {
            COMPRESSOR_ID => COMP_SPECS, GATE_ID => GATE_SPECS,
            DEESSER_ID => DEESSER_SPECS, LIMITER_ID => LIMITER_SPECS,
            REVERB_ID => REVERB_SPECS, DELAY_ID => DELAY_SPECS,
            SATURATION_ID => SATURATION_SPECS, MULTIBAND_ID => MULTIBAND_SPECS,
            FILTER_ID => FILTER_SPECS, SIMPLON_ID => SIMPLON_SPECS,
            MICRO_ID => MICRO_SPECS, _ => return None,
        };
        table_descs(specs, &table_defaults(specs))
    };
    let targets = descriptors.len();
    let mut shared = table_descs(MOD_SPECS, &table_defaults(MOD_SPECS));
    for d in &mut shared {
        if d.id.ends_with("_target") { d.max = (targets - 1) as f64; d.step_count = (targets - 1) as i64; }
    }
    descriptors.extend(shared);
    Some(descriptors)
}

pub fn default_parameters(device: &str) -> Result<BTreeMap<String, f64>, String> {
    let descriptors = parameter_descriptors(device).ok_or_else(|| format!("unknown native device {device}"))?;
    Ok(descriptors.into_iter().map(|d| (d.id, d.default_value.unwrap_or(d.value))).collect())
}

/// Validate a whole parameter map before returning any updated state.
pub fn complete_parameters(device: &str, overrides: &BTreeMap<String, f64>) -> Result<BTreeMap<String, f64>, String> {
    let descriptors = parameter_descriptors(device).ok_or_else(|| format!("unknown native device {device}"))?;
    let mut result: BTreeMap<_, _> = descriptors.iter().map(|d| (d.id.clone(), d.default_value.unwrap_or(d.value))).collect();
    for (id, value) in overrides {
        let d = descriptors.iter().find(|d| &d.id == id).ok_or_else(|| format!("{device}: unknown parameter {id}"))?;
        if !value.is_finite() || !(d.min..=d.max).contains(value) {
            return Err(format!("{id} must be finite and within {}..{}", d.min, d.max));
        }
        if d.step_count > 0 && d.max > d.min {
            let step = (value - d.min) / (d.max - d.min) * d.step_count as f64;
            if (step - step.round()).abs() > 8. * f64::EPSILON * step.abs().max(1.) {
                return Err(format!("{id} is off its native step grid"));
            }
        }
        result.insert(id.clone(), *value);
    }
    Ok(result)
}

fn library() -> &'static [FactoryPreset] {
    static LIBRARY: OnceLock<Vec<FactoryPreset>> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let data: Library = serde_json::from_str(include_str!("factory_presets.json"))
            .expect("the shipped factory library is valid JSON");
        assert_eq!(data.schema, LIBRARY_SCHEMA);
        assert_eq!(data.version, LIBRARY_VERSION);
        let mut ids = BTreeSet::new();
        data.presets.into_iter().map(|mut preset| {
            assert!(ids.insert(preset.id.clone()), "factory preset IDs are unique");
            assert!(!preset.name.trim().is_empty() && !preset.category.trim().is_empty() && !preset.description.trim().is_empty());
            preset.parameters = complete_parameters(&preset.device, &preset.parameters)
                .expect("shipped factory settings satisfy native parameter descriptors");
            preset
        }).collect()
    })
}

pub fn factory_presets(device: &str) -> Vec<FactoryPreset> {
    library().iter().filter(|p| p.device == device).cloned().collect()
}
pub fn factory_preset(id: &str) -> Option<FactoryPreset> {
    library().iter().find(|p| p.id == id).cloned()
}
pub fn all_factory_presets() -> &'static [FactoryPreset] { library() }


/// Whether a parameter can be scheduled while audio is running.
pub fn parameter_is_automatable(device: &str, id: &str) -> bool {
    if is_builtin_instrument(device) {
        return instrument_specs(device).iter().find(|s| s.id == id).is_some_and(|s| s.automatable);
    }
    if let Some(spec) = modulation_parameter(id) { return spec.automatable; }
    if device == EQ_ID { return eq_spec(id).is_some_and(|s| s.automatable); }
    let specs = match device {
        COMPRESSOR_ID => COMP_SPECS, GATE_ID => GATE_SPECS, DEESSER_ID => DEESSER_SPECS,
        LIMITER_ID => LIMITER_SPECS, REVERB_ID => REVERB_SPECS, DELAY_ID => DELAY_SPECS,
        SATURATION_ID => SATURATION_SPECS, MULTIBAND_ID => MULTIBAND_SPECS,
        FILTER_ID => FILTER_SPECS, SIMPLON_ID => SIMPLON_SPECS, MICRO_ID => MICRO_SPECS,
        _ => return false,
    };
    specs.iter().find(|s| s.id == id).is_some_and(|s| s.automatable)
}

pub fn decode_native_state(state: &str) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    use base64::Engine;
    if state.is_empty() { return Ok(Default::default()); }
    if state.len() > 2 * 1024 * 1024 { return Err("native state exceeds 2 MiB".into()); }
    let bytes = base64::engine::general_purpose::STANDARD.decode(state).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid native state: {e}"))
}
pub fn encode_native_state(state: &serde_json::Map<String, serde_json::Value>) -> Result<String, String> {
    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(state).map_err(|e| e.to_string())?))
}
pub fn parameter_state(parameters: &BTreeMap<String, f64>) -> Result<String, String> {
    let state = parameters.iter().map(|(k,v)| (k.clone(), serde_json::json!(v))).collect();
    encode_native_state(&state)
}

/// Partial writes preserve future/unknown state keys; preset application uses
/// a complete replacement explicitly. Invalid known existing values fail closed.
pub fn patch_native_state(device: &str, state: &str, values: &BTreeMap<String, f64>) -> Result<String, String> {
    complete_parameters(device, values)?;
    let mut object = decode_native_state(state)?;
    let descriptors = parameter_descriptors(device).ok_or("unknown native device")?;
    let known: BTreeMap<_,_> = object.iter().filter(|(k,_)| descriptors.iter().any(|d| &d.id == *k))
        .map(|(k,v)| v.as_f64().map(|v|(k.clone(),v)).ok_or_else(|| format!("non-numeric parameter {k}")))
        .collect::<Result<_,_>>()?;
    complete_parameters(device, &known)?;
    for (k,v) in values { object.insert(k.clone(), serde_json::json!(v)); }
    encode_native_state(&object)
}
