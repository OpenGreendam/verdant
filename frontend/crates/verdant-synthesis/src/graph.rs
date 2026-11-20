//! Rust-owned ONNX graph execution for the current voice controller and decoder.
//!
//! `build.rs` derives the ONNX Runtime 1.23.2 ABI and compiles layout assertions
//! for the target. Vendored headers, notices and source fingerprints live under
//! `neural_ffi`. The runtime library is loaded separately.
use std::path::PathBuf;
use anyhow::{ensure, Context, Result};
use serde_json::Value;

mod ort_abi;
mod ort_graph;
pub(crate) use ort_graph::{GraphSession, GraphTensor, TensorData};

fn library_path(options: &Value) -> Result<PathBuf> {
    if let Some(value) = options.get("onnxruntime_library") {
        let name = value.as_str().context("onnxruntime_library must be a path string")?;
        ensure!(!name.is_empty(), "onnxruntime_library must not be empty");
        return Ok(PathBuf::from(name));
    }
    if let Some(value) = std::env::var_os("VERDANT_ONNXRUNTIME_LIBRARY") {
        ensure!(!value.is_empty(), "VERDANT_ONNXRUNTIME_LIBRARY must not be empty");
        return Ok(PathBuf::from(value));
    }
    let executable = std::env::current_exe().context("locating native ONNX Runtime beside the executable")?;
    let parent = executable.parent().context("executable has no parent directory")?;
    let library_name = if cfg!(target_os = "windows") { "onnxruntime.dll" }
        else if cfg!(target_os = "macos") { "libonnxruntime.1.23.2.dylib" }
        else { "libonnxruntime.so.1.23.2" };
    let packaged = if cfg!(target_os = "macos") { parent.join("../Frameworks").join(library_name) }
        else if cfg!(windows) { parent.join(library_name) }
        else { parent.join("../lib").join(library_name) };
    for candidate in [packaged, parent.join(library_name)] {
        if candidate.is_file() { return Ok(candidate); }
    }
    // The normal platform loader also supports an explicitly installed runtime.
    Ok(PathBuf::from(library_name))
}
