//! Minimal Rust ownership layer over the pinned ONNX Runtime C API.
//!
//! build.rs derives declarations, enum values and table layout from the vendored
//! headers and compiles matching C layout assertions for the selected target.
#![allow(non_snake_case)]

use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    marker::PhantomData,
    mem::{align_of, size_of, size_of_val},
    path::Path,
    ptr::{self, NonNull},
};

use anyhow::{bail, ensure, Context, Result};
use libloading::Library;

include!(concat!(env!("OUT_DIR"), "/native_abi.rs"));

pub(super) const MAX_TENSORS: usize = 32;
const MAX_RANK: usize = 8;

#[derive(Clone, Copy)]
struct Api(NonNull<OrtApi>);

impl Api {
    fn table(&self) -> &OrtApi {
        // Api is only constructed after version/API admission. NativeSession
        // retains the originating Library until every handle has been released.
        unsafe { self.0.as_ref() }
    }

    fn check(self, status: *mut OrtStatus) -> Result<()> {
        if status.is_null() { return Ok(()); }
        let status = Owned { pointer: status, release: self.table().ReleaseStatus };
        let message = unsafe { (self.table().GetErrorMessage)(status.pointer) };
        if message.is_null() { bail!("native neural backend: ONNX Runtime returned an error without a message"); }
        let message = unsafe { CStr::from_ptr(message) };
        bail!("native neural backend: {}", message.to_string_lossy())
    }
}

struct Owned<T> {
    pointer: *mut T,
    release: unsafe extern "system" fn(*mut T),
}

impl<T> Owned<T> {
    fn new(release: unsafe extern "system" fn(*mut T)) -> Self {
        Self { pointer: ptr::null_mut(), release }
    }
}

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        if !self.pointer.is_null() { unsafe { (self.release)(self.pointer) }; }
    }
}

struct Values {
    api: Api,
    pointers: [*mut OrtValue; MAX_TENSORS],
    len: usize,
}

impl Values {
    fn new(api: Api) -> Self { Self { api, pointers: [ptr::null_mut(); MAX_TENSORS], len: 0 } }

    fn clear(&mut self) {
        for pointer in &mut self.pointers[..self.len] {
            if !pointer.is_null() {
                unsafe { (self.api.table().ReleaseValue)(*pointer) };
                *pointer = ptr::null_mut();
            }
        }
        self.len = 0;
    }
}

impl Drop for Values {
    fn drop(&mut self) { self.clear(); }
}

pub(super) enum InputData<'a> {
    F32(&'a mut [f32]),
    F64(&'a mut [f64]),
    I64(&'a mut [i64]),
    Bool(&'a mut [u8]),
}

impl InputData<'_> {
    fn dtype(&self) -> c_int {
        match self { Self::F32(_) => FLOAT, Self::F64(_) => DOUBLE, Self::I64(_) => INT64, Self::Bool(_) => BOOL }
    }

    fn len(&self) -> usize {
        match self { Self::F32(v) => v.len(), Self::F64(v) => v.len(), Self::I64(v) => v.len(), Self::Bool(v) => v.len() }
    }

    fn storage(&mut self) -> (*mut c_void, usize) {
        match self {
            Self::F32(v) => (v.as_mut_ptr().cast(), size_of_val(&**v)),
            Self::F64(v) => (v.as_mut_ptr().cast(), size_of_val(&**v)),
            Self::I64(v) => (v.as_mut_ptr().cast(), size_of_val(&**v)),
            Self::Bool(v) => (v.as_mut_ptr().cast(), size_of_val(&**v)),
        }
    }
}

pub(super) struct TensorInput<'a> {
    pub name: &'a CStr,
    pub shape: &'a [i64],
    pub data: InputData<'a>,
}

pub(super) struct TensorInfo {
    pub name: CString,
    pub shape: Vec<i64>,
    pub dtype: c_int,
}

pub(super) struct TensorOutput<'session> {
    data: NonNull<c_void>,
    elements: usize,
    pub dtype: c_int,
    pub shape: Vec<i64>,
    _session: PhantomData<&'session NativeSession>,
}

impl<'session> TensorOutput<'session> {
    fn checked_data<T>(&self, dtype: c_int) -> Result<*const T> {
        ensure!(self.dtype == dtype, "unexpected ONNX output tensor type {}", self.dtype);
        ensure!(self.elements <= isize::MAX as usize / size_of::<T>() &&
            self.data.as_ptr().addr() % align_of::<T>() == 0, "invalid ONNX output allocation");
        Ok(self.data.as_ptr().cast())
    }

    pub fn f32_values(&self) -> Result<&'session [f32]> {
        let data = self.checked_data::<f32>(FLOAT)?;
        Ok(unsafe { std::slice::from_raw_parts(data, self.elements) })
    }

    pub fn f64_values(&self) -> Result<&'session [f64]> {
        let data = self.checked_data::<f64>(DOUBLE)?;
        Ok(unsafe { std::slice::from_raw_parts(data, self.elements) })
    }

    pub fn i64_values(&self) -> Result<&'session [i64]> {
        let data = self.checked_data::<i64>(INT64)?;
        Ok(unsafe { std::slice::from_raw_parts(data, self.elements) })
    }

    pub fn bool_values(&self) -> Result<&'session [u8]> {
        let data = self.checked_data::<u8>(BOOL)?;
        Ok(unsafe { std::slice::from_raw_parts(data, self.elements) })
    }
}

struct AllocatedName<'session> {
    session: &'session NativeSession,
    allocator: *mut OrtAllocator,
    pointer: *mut c_char,
}

impl Drop for AllocatedName<'_> {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            let api = self.session.api.table();
            let status = unsafe { (api.AllocatorFree)(self.allocator, self.pointer.cast()) };
            // Cleanup never hides the primary creation/metadata error.
            if !status.is_null() { unsafe { (api.ReleaseStatus)(status) }; }
        }
    }
}

pub(super) struct NativeSession {
    api: Api,
    // Rust drops fields in declaration order. Library must remain last, and
    // outputs/session must be released before their memory/options/environment.
    outputs: Values,
    memory: Owned<OrtMemoryInfo>,
    session: Owned<OrtSession>,
    options: Owned<OrtSessionOptions>,
    environment: Owned<OrtEnv>,
    pub input_count: usize,
    pub output_count: usize,
    _library: Library,
}

impl NativeSession {
    pub fn new(model: &[u8], path: &Path, threads: usize) -> Result<Self> {
        ensure!(!model.is_empty() && threads > 0 && threads <= c_int::MAX as usize,
            "invalid ORT creation arguments");
        let library = unsafe { Library::new(path) }
            .with_context(|| format!("loading native ONNX Runtime {}", path.display()))?;
        let base = unsafe {
            let get_base = library.get::<unsafe extern "system" fn() -> *const OrtApiBase>(b"OrtGetApiBase\0")
                .context("ONNX Runtime has no OrtGetApiBase export")?;
            get_base().as_ref().context("ONNX Runtime returned a null API base")?
        };
        let version = unsafe { (base.GetVersionString)() };
        ensure!(!version.is_null() && unsafe { CStr::from_ptr(version) }.to_bytes() == b"1.23.2",
            "ONNX Runtime version must be exactly 1.23.2");
        let api = Api(NonNull::new(unsafe { (base.GetApi)(API_VERSION) }.cast_mut())
            .context("ONNX Runtime C API23 unavailable")?);
        let table = api.table();
        let mut state = Self {
            api, outputs: Values::new(api), memory: Owned::new(table.ReleaseMemoryInfo),
            session: Owned::new(table.ReleaseSession), options: Owned::new(table.ReleaseSessionOptions),
            environment: Owned::new(table.ReleaseEnv), input_count: 0, output_count: 0, _library: library,
        };
        api.check(unsafe { (table.CreateEnv)(WARNING, c"verdant-synthesis".as_ptr(), &mut state.environment.pointer) })?;
        api.check(unsafe { (table.CreateSessionOptions)(&mut state.options.pointer) })?;
        api.check(unsafe { (table.SetIntraOpNumThreads)(state.options.pointer, threads as c_int) })?;
        api.check(unsafe { (table.SetInterOpNumThreads)(state.options.pointer, 1) })?;
        api.check(unsafe { (table.SetSessionExecutionMode)(state.options.pointer, SEQUENTIAL) })?;
        api.check(unsafe { (table.SetSessionGraphOptimizationLevel)(state.options.pointer, ENABLE_ALL) })?;
        api.check(unsafe { (table.DisableCpuMemArena)(state.options.pointer) })?;
        api.check(unsafe { (table.DisableMemPattern)(state.options.pointer) })?;
        api.check(unsafe { (table.CreateSessionFromArray)(state.environment.pointer, model.as_ptr().cast(),
            model.len(), state.options.pointer, &mut state.session.pointer) })?;
        ensure!(!state.session.pointer.is_null(), "ONNX Runtime returned a null session");
        api.check(unsafe { (table.SessionGetInputCount)(state.session.pointer, &mut state.input_count) })?;
        api.check(unsafe { (table.SessionGetOutputCount)(state.session.pointer, &mut state.output_count) })?;
        ensure!((1..=MAX_TENSORS).contains(&state.input_count) && (1..=MAX_TENSORS).contains(&state.output_count),
            "ONNX graph tensor count is outside the supported contract");
        api.check(unsafe { (table.CreateCpuMemoryInfo)(ARENA_ALLOCATOR, MEM_TYPE_DEFAULT, &mut state.memory.pointer) })?;
        Ok(state)
    }

    pub fn tensor_info(&self, output: bool, index: usize) -> Result<TensorInfo> {
        ensure!(index < if output { self.output_count } else { self.input_count }, "ORT tensor index is out of range");
        let api = self.api;
        let table = api.table();
        let mut allocator = ptr::null_mut();
        api.check(unsafe { (table.GetAllocatorWithDefaultOptions)(&mut allocator) })?;
        ensure!(!allocator.is_null(), "ONNX Runtime returned a null allocator");
        let mut allocated = AllocatedName { session: self, allocator, pointer: ptr::null_mut() };
        let get_name = if output { table.SessionGetOutputName } else { table.SessionGetInputName };
        api.check(unsafe { get_name(self.session.pointer, index, allocator, &mut allocated.pointer) })?;
        ensure!(!allocated.pointer.is_null(), "ONNX Runtime returned a null tensor name");
        let name = unsafe { CStr::from_ptr(allocated.pointer) };
        ensure!(name.to_bytes().len() < 256, "ORT tensor name exceeds ABI capacity");
        name.to_str().context("non-UTF8 ONNX tensor name")?;
        let name = name.to_owned();
        let mut info = Owned::new(table.ReleaseTypeInfo);
        let get_type = if output { table.SessionGetOutputTypeInfo } else { table.SessionGetInputTypeInfo };
        api.check(unsafe { get_type(self.session.pointer, index, &mut info.pointer) })?;
        let mut tensor = ptr::null();
        api.check(unsafe { (table.CastTypeInfoToTensorInfo)(info.pointer, &mut tensor) })?;
        ensure!(!tensor.is_null(), "ORT interface is not a tensor");
        let mut rank = 0;
        api.check(unsafe { (table.GetDimensionsCount)(tensor, &mut rank) })?;
        ensure!(rank <= MAX_RANK, "ORT tensor rank exceeds ABI capacity");
        let mut shape = vec![0; rank];
        api.check(unsafe { (table.GetDimensions)(tensor, shape.as_mut_ptr(), rank) })?;
        let mut parameters = [ptr::null(); MAX_RANK];
        api.check(unsafe { (table.GetSymbolicDimensions)(tensor, parameters.as_mut_ptr(), rank) })?;
        for &parameter in &parameters[..rank] {
            let symbol = if parameter.is_null() { c"" } else { unsafe { CStr::from_ptr(parameter) } };
            ensure!(symbol.to_bytes().len() < 128, "ONNX dimension name exceeds ABI capacity");
            symbol.to_str().context("non-UTF8 ONNX dimension name")?;
        }
        let mut dtype = 0;
        api.check(unsafe { (table.GetTensorElementType)(tensor, &mut dtype) })?;
        Ok(TensorInfo { name, shape, dtype })
    }

    pub fn run<'input>(&mut self, inputs: impl ExactSizeIterator<Item = TensorInput<'input>>,
        outputs: impl ExactSizeIterator<Item = &'input CStr>) -> Result<()> {
        let input_count = inputs.len();
        let output_count = outputs.len();
        ensure!((1..=MAX_TENSORS).contains(&input_count) && (1..=MAX_TENSORS).contains(&output_count),
            "invalid ONNX graph arguments");
        self.outputs.clear();
        let api = self.api;
        let table = api.table();
        let mut values = Values::new(api);
        let mut input_names = [ptr::null(); MAX_TENSORS];
        let mut input_values = [ptr::null(); MAX_TENSORS];
        let mut output_names = [ptr::null(); MAX_TENSORS];
        for (index, mut input) in inputs.enumerate() {
            ensure!(!input.shape.is_empty() && input.shape.len() <= MAX_RANK, "invalid ONNX graph input tensor");
            let elements = input.shape.iter().try_fold(1usize, |count, &dimension|
                usize::try_from(dimension).ok().filter(|&value| value > 0).and_then(|value| count.checked_mul(value)))
                .context("invalid ONNX graph input dimensions")?;
            ensure!(elements == input.data.len(), "ONNX graph input storage differs from shape");
            let dtype = input.data.dtype();
            let (data, bytes) = input.data.storage();
            values.len = index + 1;
            api.check(unsafe { (table.CreateTensorWithDataAsOrtValue)(self.memory.pointer, data, bytes,
                input.shape.as_ptr(), input.shape.len(), dtype, &mut values.pointers[index]) })?;
            ensure!(!values.pointers[index].is_null(), "ONNX Runtime returned a null input tensor");
            input_names[index] = input.name.as_ptr();
            input_values[index] = values.pointers[index].cast_const();
        }
        for (index, name) in outputs.enumerate() { output_names[index] = name.as_ptr(); }
        // Run is synchronous. The Rust buffers and input OrtValues remain alive
        // throughout it; no feature data or per-input ownership boxes are copied.
        self.outputs.len = output_count;
        let result = api.check(unsafe { (table.Run)(self.session.pointer, ptr::null(), input_names.as_ptr(),
            input_values.as_ptr(), input_count, output_names.as_ptr(), output_count, self.outputs.pointers.as_mut_ptr()) });
        if result.is_err() { self.outputs.clear(); }
        result
    }

    pub fn output(&self, index: usize) -> Result<TensorOutput<'_>> {
        ensure!(index < self.outputs.len && !self.outputs.pointers[index].is_null(), "invalid ONNX output arguments");
        let api = self.api;
        let table = api.table();
        let output = self.outputs.pointers[index];
        let mut info = Owned::new(table.ReleaseTensorTypeAndShapeInfo);
        api.check(unsafe { (table.GetTensorTypeAndShape)(output, &mut info.pointer) })?;
        let mut dtype = 0;
        api.check(unsafe { (table.GetTensorElementType)(info.pointer, &mut dtype) })?;
        let mut rank = 0;
        api.check(unsafe { (table.GetDimensionsCount)(info.pointer, &mut rank) })?;
        ensure!(rank <= MAX_RANK, "ONNX output rank exceeds capacity");
        let mut shape = vec![0; rank];
        api.check(unsafe { (table.GetDimensions)(info.pointer, shape.as_mut_ptr(), rank) })?;
        let mut elements = 0;
        api.check(unsafe { (table.GetTensorShapeElementCount)(info.pointer, &mut elements) })?;
        let mut data = ptr::null_mut();
        api.check(unsafe { (table.GetTensorMutableData)(output, &mut data) })?;
        let data = NonNull::new(data).context("ONNX output is empty")?;
        ensure!(elements > 0, "ONNX output is empty");
        Ok(TensorOutput { data, elements, dtype, shape, _session: PhantomData })
    }
}
