//! Persisted synthesis choices. Missing settings retain an older project's
//! original sound; the app explicitly selects `native_v10` for new projects.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SynthesisSettings {
    #[serde(default)]
    pub vowel_balance_db: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wideband: Option<WidebandSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_correction: Option<PhoneticCorrectionSettings>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WidebandSettings {
    pub method: String,
    pub presence_db: f64,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhoneticCorrectionSettings {
    pub method: String,
    pub strength: f64,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VocoderHighbandSettings {
    pub method: String,
    pub strength: f64,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

/// Spectral-envelope transformation, independent of score F0 and synthesis presets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VocalTractSettings {
    pub method: String,
    pub ratio: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_ratios: Option<[f64; 2]>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

/// Optional model-derived attenuation of explicitly voiced vowel segments.
/// Independent of vocal-tract colour, authored pitch, and synthesis seed policy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelDynamicsSettings {
    pub method: String,
    pub strength: f64,
    pub max_attenuation_db: f64,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

/// Score-to-phone alignment. Missing settings preserve syllable-start timing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhonemeTimingSettings {
    pub method: String,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

/// Optional attenuation of explicitly unvoiced consonants from model energy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsonantDynamicsSettings {
    pub method: String,
    pub strength: f64,
    pub max_attenuation_db: f64,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

impl ConsonantDynamicsSettings {
    pub fn unvoiced_model_envelope_v1() -> Self {
        Self { method: "unvoiced-model-envelope-v1".into(), strength: 0.25, max_attenuation_db: 6.0, unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.method != "unvoiced-model-envelope-v1" {
            return Err(format!("unsupported consonant-dynamics method {:?}", self.method));
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("consonant-dynamics strength must be finite and in 0..=1".into());
        }
        if !self.max_attenuation_db.is_finite() || !(0.0..=12.0).contains(&self.max_attenuation_db) {
            return Err("consonant-dynamics max_attenuation_db must be finite and in 0..=12".into());
        }
        Ok(())
    }
}

/// Final reconstructed-waveform obstruent protection, independent of source energy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputTransientsSettings {
    pub method: String,
    pub strength: f64,
    pub max_attenuation_db: f64,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

impl OutputTransientsSettings {
    pub fn phoneme_output_envelope_v1() -> Self {
        Self { method: "phoneme-output-envelope-v1".into(), strength: 1., max_attenuation_db: 6., unknown: Map::new() }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.method != "phoneme-output-envelope-v1" || !self.strength.is_finite()
            || !(0.0..=1.0).contains(&self.strength) || !self.max_attenuation_db.is_finite()
            || !(0.0..=12.0).contains(&self.max_attenuation_db) {
            return Err("output transients require phoneme-output-envelope-v1, strength 0..1 and cap 0..12 dB".into());
        }
        Ok(())
    }
}

impl PhonemeTimingSettings {
    pub fn vowel_anchor_v1() -> Self {
        Self { method: "vowel-anchor-v1".into(), unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.method != "vowel-anchor-v1" {
            return Err(format!("unsupported phoneme-timing method {:?}", self.method));
        }
        Ok(())
    }
}

impl ModelDynamicsSettings {
    /// An opt-in conservative preset; absent settings remain off.
    pub fn voiced_envelope_v1() -> Self {
        Self { method: "voiced-envelope-v1".into(), strength: 0.25, max_attenuation_db: 6.0, unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.method != "voiced-envelope-v1" {
            return Err(format!("unsupported model-dynamics method {:?}", self.method));
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("model-dynamics strength must be finite and in 0..=1".into());
        }
        if !self.max_attenuation_db.is_finite() || !(0.0..=12.0).contains(&self.max_attenuation_db) {
            return Err("model-dynamics max_attenuation_db must be finite and in 0..=12".into());
        }
        Ok(())
    }
}

impl VocalTractSettings {
    pub fn bilinear_v1(ratio: f64) -> Self {
        Self { method: "bilinear-v1".into(), ratio, anchor_ratios: None, unknown: Map::new() }
    }

    pub fn anchored_warp_v1(ratio: f64, anchor_ratios: [f64; 2]) -> Self {
        Self { method: "anchored-warp-v1".into(), ratio, anchor_ratios: Some(anchor_ratios), unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self.method.as_str() {
            "bilinear-v1" if self.anchor_ratios.is_none() => {},
            "bilinear-v1" => return Err("bilinear-v1 does not accept anchor_ratios".into()),
            "anchored-warp-v1" => {
                let anchors = self.anchor_ratios.ok_or("anchored-warp-v1 requires two anchor_ratios")?;
                if anchors.iter().any(|value| !value.is_finite() || !(0.8..=1.25).contains(value)) {
                    return Err("vocal-tract anchor ratios must be finite and in 0.8..=1.25".into());
                }
            },
            _ => return Err(format!("unsupported vocal-tract method {:?}", self.method)),
        }
        if !self.ratio.is_finite() || !(0.8..=1.25).contains(&self.ratio) {
            return Err("vocal-tract ratio must be finite and in 0.8..=1.25".into());
        }
        Ok(())
    }
}

impl VocoderHighbandSettings {
    pub fn source_coherent_v1() -> Self {
        Self { method: "source-coherent-v1".into(), strength: 1.0, unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.method != "source-coherent-v1" {
            return Err(format!("unsupported vocoder high-band method {:?}", self.method));
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("vocoder high-band strength must be finite and in 0..=1".into());
        }
        Ok(())
    }
}

impl PhoneticCorrectionSettings {
    pub fn articulatory_v1() -> Self {
        Self { method: "articulatory-v1".into(), strength: 1.0, unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.method != "articulatory-v1" {
            return Err(format!("unsupported phonetic correction method {:?}", self.method));
        }
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("phonetic correction strength must be finite and in 0..=1".into());
        }
        Ok(())
    }
}

impl SynthesisSettings {
    pub fn native_v10() -> Self {
        Self { vowel_balance_db: 18.0, wideband: Some(WidebandSettings::native_v10()), phonetic_correction: None, unknown: Map::new() }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.vowel_balance_db.is_finite() || !(0.0..=18.0).contains(&self.vowel_balance_db) {
            return Err("vowel_balance_db must be finite and in 0..=18".into());
        }
        if let Some(wideband) = &self.wideband {
            if wideband.method != "matched-balanced" {
                return Err(format!("unsupported wideband method {:?}", wideband.method));
            }
            if !wideband.presence_db.is_finite() || !(0.0..=24.0).contains(&wideband.presence_db) {
                return Err("wideband presence_db must be finite and in 0..=24".into());
            }
        }
        if let Some(correction) = &self.phonetic_correction {
            correction.validate()?;
        }
        Ok(())
    }
}

impl WidebandSettings {
    pub fn native_v10() -> Self {
        Self { method: "matched-balanced".into(), presence_db: 24.0, unknown: Map::new() }
    }
}
