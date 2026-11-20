use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Shape and dtype are part of the public numerical/artifact contract.
/// Values preserve f32 inputs exactly after conversion to f64; dtype retains the storage policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NumericArray {
    pub shape: Vec<usize>,
    pub dtype: String,
    pub values: Vec<f64>,
}
impl NumericArray {
    pub fn new(shape: Vec<usize>, dtype: impl Into<String>, values: Vec<f64>) -> anyhow::Result<Self> {
        let count = shape.iter().try_fold(1usize, |a, &b| a.checked_mul(b)).ok_or_else(|| anyhow::anyhow!("array shape overflow"))?;
        anyhow::ensure!(count == values.len(), "array shape does not match values");
        anyhow::ensure!(values.iter().all(|v| v.is_finite()), "array contains nonfinite values");
        Ok(Self { shape, dtype: dtype.into(), values })
    }
}
pub type Arrays = BTreeMap<String, NumericArray>;
#[derive(Clone, Debug)]
pub struct SelectedPhone {
    pub arrays: Arrays,
    pub metadata: Value,
}
#[derive(Clone, Debug)]
pub struct AcousticControlTrack {
    pub f0_hz: Vec<f64>,
    pub voiced: Vec<bool>,
    pub phonetic_classes: Vec<String>,
    pub phone_identity: Vec<i64>,
    pub segment_identity: Vec<i64>,
    pub continuity_identity: Vec<i64>,
    pub period_seconds: f64,
    pub origin_seconds: f64,
}
#[derive(Clone, Debug)]
pub struct RunParameters {
    pub start_frame: usize,
    pub frames: usize,
    pub arrays: Arrays,
    pub controls: AcousticControlTrack,
    pub metadata: Value,
}
#[derive(Clone, Debug)]
pub struct NativePlan {
    pub runs: Vec<RunParameters>,
    pub duration_frames: usize,
    pub sample_rate: u32,
    pub hop_samples: usize,
    pub metadata: Value,
}
#[derive(Debug)]
pub struct AudioResult {
    pub sample_rate: u32,
    pub samples: Vec<f64>,
    pub source_dtype: String,
    pub arrays: Arrays,
    pub metadata: Value,
}
