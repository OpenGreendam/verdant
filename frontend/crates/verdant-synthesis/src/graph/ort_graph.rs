//! Typed ONNX graph boundary used by verdant-voice. No fallback or Python execution.
use std::{collections::BTreeMap, ffi::c_int};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use super::{library_path, ort_abi::{BOOL, DOUBLE, FLOAT, INT64, InputData, NativeSession, TensorInfo, TensorInput}};

#[derive(Debug, Clone)]
pub(crate) enum TensorData { F32(Vec<f32>), F64(Vec<f64>), I64(Vec<i64>), Bool(Vec<u8>) }
impl TensorData {
    fn dtype(&self) -> c_int {
        match self { Self::F32(_) => FLOAT, Self::F64(_) => DOUBLE, Self::I64(_) => INT64, Self::Bool(_) => BOOL }
    }
    fn len(&self) -> usize {
        match self { Self::F32(v) => v.len(), Self::F64(v) => v.len(), Self::I64(v) => v.len(), Self::Bool(v) => v.len() }
    }
    fn input(&mut self) -> InputData<'_> {
        match self { Self::F32(v) => InputData::F32(v), Self::F64(v) => InputData::F64(v),
            Self::I64(v) => InputData::I64(v), Self::Bool(v) => InputData::Bool(v) }
    }
    fn validate(&self) -> Result<()> {
        ensure!(match self { Self::F32(v) => v.iter().all(|v| v.is_finite()),
            Self::F64(v) => v.iter().all(|v| v.is_finite()), Self::Bool(v) => v.iter().all(|v| *v <= 1),
            Self::I64(_) => true }, "invalid ONNX tensor data");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GraphTensor { pub shape: Vec<i64>, pub data: TensorData }
impl GraphTensor {
    pub fn new(shape: Vec<i64>, data: TensorData) -> Result<Self> {
        let tensor = Self { shape, data };
        tensor.validate()?;
        Ok(tensor)
    }
    fn validate(&self) -> Result<()> {
        let count = self.shape.iter().try_fold(1usize, |n, &d| usize::try_from(d).ok()
            .filter(|d| *d > 0).and_then(|d| n.checked_mul(d))).context("invalid ONNX tensor shape")?;
        ensure!(!self.shape.is_empty() && self.shape.len() <= 8 && count == self.data.len(), "ONNX tensor data and shape differ");
        self.data.validate()
    }
    pub fn f32(shape: Vec<i64>, data: Vec<f32>) -> Result<Self> { Self::new(shape, TensorData::F32(data)) }
    pub fn f32_values(&self) -> Result<&[f32]> {
        match &self.data { TensorData::F32(v) => Ok(v), _ => bail!("expected float32 output") }
    }
    pub fn f64_values(&self) -> Result<&[f64]> {
        match &self.data { TensorData::F64(v) => Ok(v), _ => bail!("expected float64 output") }
    }
}

pub(crate) struct GraphSession { native: NativeSession, inputs: Vec<TensorInfo>, outputs: Vec<TensorInfo> }
impl GraphSession {
    pub fn new(model: &[u8], options: &Value, threads: usize) -> Result<Self> {
        ensure!(!model.is_empty() && (1..=32).contains(&threads), "invalid ONNX graph model or threads");
        let path = library_path(options)?;
        let native = NativeSession::new(model, &path, threads)?;
        let mut inputs = (0..native.input_count).map(|i| native.tensor_info(false, i)).collect::<Result<Vec<_>>>()?;
        let outputs = (0..native.output_count).map(|i| native.tensor_info(true, i)).collect::<Result<Vec<_>>>()?;
        for info in inputs.iter().chain(&outputs) {
            ensure!(matches!(info.dtype, FLOAT | DOUBLE | INT64 | BOOL), "ONNX graph has unsupported tensor type {}", info.dtype);
        }
        // BTreeMap uses the same UTF-8 byte order. Cache this order once so
        // repeated controller/decoder calls need no name searches or CStrings.
        inputs.sort_unstable_by(|left, right| left.name.to_bytes().cmp(right.name.to_bytes()));
        Ok(Self { native, inputs, outputs })
    }
    /// Assert the complete ABI, including tensor types and every static axis.
    pub fn require_interface(&self, inputs: &[(&str, c_int, &[i64])], outputs: &[(&str, c_int, &[i64])]) -> Result<()> {
        for (actual, expected) in [(&self.inputs, inputs), (&self.outputs, outputs)] {
            ensure!(actual.len() == expected.len(), "ONNX graph tensor count differs from Verdant ONNX ABI");
            for (name, dtype, shape) in expected {
                let found = actual.iter().find(|item| item.name.to_bytes() == name.as_bytes())
                    .with_context(|| format!("ONNX graph lacks tensor {name}"))?;
                ensure!(found.dtype == *dtype && found.shape.len() == shape.len() &&
                    found.shape.iter().zip(*shape).all(|(&a, &b)| if b < 0 { a < 0 } else { a == b }),
                    "ONNX graph tensor {name} has wrong type or shape");
            }
        }
        Ok(())
    }
    pub fn run(&mut self, mut tensors: BTreeMap<String, GraphTensor>) -> Result<BTreeMap<String, GraphTensor>> {
        ensure!(tensors.len() == self.inputs.len(), "ONNX graph input count mismatch");
        for ((supplied, tensor), input) in tensors.iter().zip(&self.inputs) {
            let name = input.name.to_str().expect("validated ONNX name");
            ensure!(supplied == name, "missing ONNX input {name}");
            tensor.data.validate()?;
            ensure!(tensor.data.dtype() == input.dtype && tensor.shape.len() == input.shape.len() &&
                tensor.shape.iter().zip(&input.shape).all(|(&a, &b)| a > 0 && (b < 0 || a == b)),
                "ONNX graph input {name} has wrong type/shape");
        }
        let inputs = tensors.iter_mut().zip(&self.inputs).map(|((_, tensor), info)| {
            TensorInput { name: info.name.as_c_str(), shape: &tensor.shape, data: tensor.data.input() }
        });
        self.native.run(inputs, self.outputs.iter().map(|info| info.name.as_c_str()))?;
        let mut result = BTreeMap::new();
        for (index, info) in self.outputs.iter().enumerate() {
            let name = info.name.to_str().expect("validated ONNX name");
            let output = self.native.output(index)?;
            ensure!(output.dtype == info.dtype, "invalid ONNX graph output storage/type");
            // The owned graph API needs one copy before a later inference can
            // release the native OrtValue. Input buffers remain alive here too.
            let values = match output.dtype {
                FLOAT => TensorData::F32(output.f32_values()?.to_vec()),
                DOUBLE => TensorData::F64(output.f64_values()?.to_vec()),
                INT64 => TensorData::I64(output.i64_values()?.to_vec()),
                BOOL => TensorData::Bool(output.bool_values()?.to_vec()),
                _ => bail!("unsupported ONNX output dtype"),
            };
            let tensor = GraphTensor::new(output.shape, values)?;
            ensure!(tensor.shape.len() == info.shape.len() &&
                tensor.shape.iter().zip(&info.shape).all(|(&a, &b)| b < 0 || a == b),
                "ONNX graph output {name} shape drifted");
            result.insert(name.to_owned(), tensor);
        }
        Ok(result)
    }
}
