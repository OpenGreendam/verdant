use std::{collections::BTreeMap, fmt::Write as _, fs, path::PathBuf};

const BASE: &str = "src/neural_ffi";
const ORT_FUNCTIONS: &[&str] = &[
    "GetErrorMessage", "CreateEnv", "CreateSessionFromArray", "Run", "CreateSessionOptions",
    "SetSessionExecutionMode", "DisableMemPattern", "DisableCpuMemArena",
    "SetSessionGraphOptimizationLevel", "SetIntraOpNumThreads", "SetInterOpNumThreads",
    "SessionGetInputCount", "SessionGetOutputCount", "SessionGetInputTypeInfo",
    "SessionGetOutputTypeInfo", "SessionGetInputName", "SessionGetOutputName",
    "CreateTensorWithDataAsOrtValue", "GetTensorMutableData", "CastTypeInfoToTensorInfo",
    "GetTensorElementType", "GetDimensionsCount", "GetDimensions", "GetSymbolicDimensions",
    "GetTensorShapeElementCount", "GetTensorTypeAndShape", "CreateCpuMemoryInfo",
    "AllocatorFree", "GetAllocatorWithDefaultOptions", "ReleaseEnv", "ReleaseStatus",
    "ReleaseMemoryInfo", "ReleaseSession", "ReleaseValue", "ReleaseTypeInfo",
    "ReleaseTensorTypeAndShapeInfo", "ReleaseSessionOptions",
];
const ENUMS: &[&str] = &[
    "ONNXTensorElementDataType", "OrtLoggingLevel", "ExecutionMode",
    "GraphOptimizationLevel", "OrtAllocatorType", "OrtMemType",
];

// Only the small, fixed C declaration vocabulary used below is admitted. These
// bindings come from the shipped headers, without a host libclang or downloads.
fn without_comments(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    while let Some(index) = rest.find('/') {
        result.push_str(&rest[..index]);
        rest = &rest[index..];
        if rest.starts_with("/*") {
            let end = rest.find("*/").expect("unterminated native header comment");
            result.push(' ');
            rest = &rest[end + 2..];
        } else if rest.starts_with("//") {
            rest = rest.find('\n').map_or("", |end| &rest[end..]);
        } else {
            result.push('/');
            rest = &rest[1..];
        }
    }
    result.push_str(rest);
    result
}

fn unannotated(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    while let Some(index) = rest.find(|c: char| c.is_ascii_alphabetic() || c == '_') {
        result.push_str(&rest[..index]);
        rest = &rest[index..];
        let end = rest.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(rest.len());
        let annotation = rest.starts_with('_');
        if !annotation { result.push_str(&rest[..end]); }
        rest = &rest[end..];
        if annotation && rest.starts_with('(') {
            let (_, tail) = parenthesized(rest);
            rest = tail;
        }
    }
    result.push_str(rest);
    result
}

fn parenthesized(source: &str) -> (&str, &str) {
    assert!(source.starts_with('('), "expected native declaration argument list");
    let mut depth = 0;
    for (index, byte) in source.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 { return (&source[1..index], &source[index + 1..]); }
            }
            _ => {}
        }
    }
    panic!("unterminated native declaration argument list");
}

fn rust_type(source: &str) -> String {
    let normalized = source.replace('*', " * ");
    let mut words = normalized.split_whitespace().peekable();
    let mut is_const = words.next_if_eq(&"const").is_some();
    let _ = words.next_if_eq(&"enum");
    let base = words.next().expect("missing native C type");
    let mut result = match base {
        "void" => "c_void",
        "char" => "c_char",
        "int" => "c_int",
        "double" => "f64",
        "size_t" => "usize",
        "uint32_t" => "u32",
        "int64_t" => "i64",
        name if ENUMS.contains(&name) => "c_int",
        name if matches!(name, "OrtApi" | "OrtEnv" | "OrtStatus" | "OrtSessionOptions" |
            "OrtSession" | "OrtRunOptions" | "OrtValue" | "OrtMemoryInfo" | "OrtAllocator" |
            "OrtTypeInfo" | "OrtTensorTypeAndShapeInfo" | "DioOption" | "HarvestOption") => name,
        other => panic!("unsupported native C type {other}"),
    }.to_owned();
    for word in words {
        match word {
            "*" => { result = format!("*{} {result}", if is_const { "const" } else { "mut" }); is_const = false; }
            "const" => is_const = true,
            other => panic!("unsupported native C declarator {other}"),
        }
    }
    assert!(!is_const || result.starts_with('*'), "unsupported const native scalar");
    result
}

fn argument(source: &str) -> (String, String) {
    let source = unannotated(source);
    let source = source.trim();
    let (source, array) = source.strip_suffix("[]").map_or((source, false), |s| (s.trim(), true));
    let name_start = source.rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map_or(0, |index| index + 1);
    let name = &source[name_start..];
    assert!(!name.is_empty(), "missing native argument name");
    let mut kind = rust_type(source[..name_start].trim());
    if array { kind = format!("*mut {kind}"); }
    (name.to_owned(), kind)
}

fn signature(result: &str, args: &str, abi: &str) -> String {
    let args = if args.trim().is_empty() || args.trim() == "void" { Vec::new() } else {
        args.split(',').map(|arg| argument(arg).1).collect::<Vec<_>>()
    };
    let result = if result.trim() == "void" { "()".to_owned() } else { rust_type(result) };
    format!("unsafe extern \"{abi}\" fn({}) -> {result}", args.join(", "))
}

fn api_struct(header: &str, name: &str, last: &str, used: &[&str], rust: &mut String, c: &mut String) {
    let marker = format!("struct {name} {{");
    let body = header.split_once(&marker).expect("missing native API struct").1;
    writeln!(rust, "#[repr(C)]\npub(super) struct {name} {{").unwrap();
    let mut fields = Vec::new();
    for declaration in body.split(';') {
        let declaration = declaration.trim();
        let (field, result, args) = if let Some(tail) = declaration.strip_prefix("ORT_API2_STATUS(") {
            let (field, tail) = tail.split_once(',').expect("missing native status arguments");
            (field.trim().to_owned(), "OrtStatus*".to_owned(), parenthesized(&format!("({tail}")).0.to_owned())
        } else if let Some(tail) = declaration.strip_prefix("ORT_CLASS_RELEASE(") {
            let class = tail.split_once(')').expect("missing native release class").0;
            (format!("Release{class}"), "void".to_owned(), format!("Ort{class}* input"))
        } else {
            let (result, tail) = declaration.split_once("(ORT_API_CALL*").expect("unsupported native API field");
            let (field, tail) = tail.split_once(')').expect("missing native API field name");
            (field.trim().to_owned(), result.trim().to_owned(), parenthesized(tail.trim()).0.to_owned())
        };
        let kind = if used.contains(&field.as_str()) { signature(&result, &args, "system") }
            else { "unsafe extern \"system\" fn()".to_owned() };
        writeln!(rust, "    pub {field}: {kind},").unwrap();
        let slot = fields.len();
        writeln!(c, "_Static_assert(offsetof({name}, {field}) == {slot} * sizeof(void (ORT_API_CALL *)(void)), \"{name}.{field} offset\");").unwrap();
        writeln!(c, "_Static_assert(sizeof((({name}*)0)->{field}) == sizeof(void (ORT_API_CALL *)(void)), \"{name}.{field} width\");").unwrap();
        fields.push(field.clone());
        if field == last { break; }
    }
    assert_eq!(fields.last().map(String::as_str), Some(last), "missing native API prefix end");
    for field in used { assert!(fields.iter().any(|name| name == field), "missing native API function {field}"); }
    writeln!(rust, "}}\nconst _: () = {{").unwrap();
    writeln!(rust, "    assert!(std::mem::size_of::<{name}>() == {} * std::mem::size_of::<unsafe extern \"system\" fn()>());", fields.len()).unwrap();
    for (slot, field) in fields.iter().enumerate() {
        writeln!(rust, "    assert!(std::mem::offset_of!({name}, {field}) == {slot} * std::mem::size_of::<unsafe extern \"system\" fn()>());").unwrap();
    }
    writeln!(rust, "}};").unwrap();
}

fn enum_values(header: &str, name: &str) -> BTreeMap<String, i32> {
    let marker = format!("typedef enum {name} {{");
    let body = header.split_once(&marker).expect("missing native enum").1.split_once('}').unwrap().0;
    let mut result = BTreeMap::new();
    let mut next = 0;
    for field in body.split(',').map(str::trim).filter(|v| !v.is_empty()) {
        let (field, value) = match field.split_once('=') {
            Some((field, value)) => (field.trim(), value.trim().parse::<i32>()
                .unwrap_or_else(|_| *result.get(value.trim()).expect("unsupported native enum expression"))),
            None => (field, next),
        };
        result.insert(field.to_owned(), value);
        next = value + 1;
    }
    result
}

fn main() {
    let read_header = |relative: &str| {
        let path = format!("{BASE}/{relative}");
        println!("cargo:rerun-if-changed={path}");
        without_comments(&fs::read_to_string(path).expect("reading vendored native header"))
    };
    let header = read_header("onnxruntime/onnxruntime_c_api.h");
    println!("cargo:rerun-if-changed={BASE}/onnxruntime/onnxruntime_ep_c_api.h");
    let mut rust = String::from("// Derived from the pinned native headers by build.rs.\n");
    let mut c = String::from("#include <stddef.h>\n#include \"onnxruntime/onnxruntime_c_api.h\"\n");
    for name in ["OrtEnv", "OrtStatus", "OrtSessionOptions", "OrtSession", "OrtRunOptions", "OrtValue",
        "OrtMemoryInfo", "OrtAllocator", "OrtTypeInfo", "OrtTensorTypeAndShapeInfo"] {
        writeln!(rust, "#[repr(C)]\npub(super) struct {name} {{ _private: [u8; 0] }}").unwrap();
    }
    api_struct(&header, "OrtApiBase", "GetVersionString", &["GetApi", "GetVersionString"], &mut rust, &mut c);
    api_struct(&header, "OrtApi", "ReleaseSessionOptions", ORT_FUNCTIONS, &mut rust, &mut c);
    let version = header.lines().find_map(|line| line.strip_prefix("#define ORT_API_VERSION "))
        .expect("missing ORT_API_VERSION").trim().parse::<u32>().expect("invalid ORT_API_VERSION");
    assert_eq!(version, 23, "Verdant requires the pinned ONNX Runtime API23 header");
    writeln!(rust, "pub(super) const API_VERSION: u32 = {version};").unwrap();
    writeln!(c, "_Static_assert(ORT_API_VERSION == {version}, \"ORT API version\");").unwrap();
    for (enumeration, native, binding) in [
        ("ONNXTensorElementDataType", "ONNX_TENSOR_ELEMENT_DATA_TYPE_FLOAT", "FLOAT"),
        ("ONNXTensorElementDataType", "ONNX_TENSOR_ELEMENT_DATA_TYPE_INT64", "INT64"),
        ("ONNXTensorElementDataType", "ONNX_TENSOR_ELEMENT_DATA_TYPE_BOOL", "BOOL"),
        ("ONNXTensorElementDataType", "ONNX_TENSOR_ELEMENT_DATA_TYPE_DOUBLE", "DOUBLE"),
        ("OrtLoggingLevel", "ORT_LOGGING_LEVEL_WARNING", "WARNING"),
        ("ExecutionMode", "ORT_SEQUENTIAL", "SEQUENTIAL"),
        ("GraphOptimizationLevel", "ORT_ENABLE_ALL", "ENABLE_ALL"),
        ("OrtAllocatorType", "OrtArenaAllocator", "ARENA_ALLOCATOR"),
        ("OrtMemType", "OrtMemTypeDefault", "MEM_TYPE_DEFAULT"),
    ] {
        let value = enum_values(&header, enumeration)[native];
        writeln!(rust, "pub(super) const {binding}: c_int = {value};").unwrap();
        writeln!(c, "_Static_assert({native} == {value}, \"{native} value\");").unwrap();
    }
    for enumeration in ENUMS {
        writeln!(c, "_Static_assert(sizeof({enumeration}) == sizeof(int), \"{enumeration} width\");").unwrap();
        writeln!(c, "_Static_assert(_Alignof({enumeration}) == _Alignof(int), \"{enumeration} alignment\");").unwrap();
    }
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    fs::write(output.join("native_abi.rs"), rust).expect("writing native Rust declarations");
    let assertions = output.join("native_abi_assertions.c");
    fs::write(&assertions, c).expect("writing native ABI assertions");
    // This translation unit contains only compile-time assertions, no C glue,
    // executable helper, host probing or generated native entry points.
    cc::Build::new().std("c11").include(BASE)
        .file(assertions).compile("verdant_native_abi_assertions");
}
