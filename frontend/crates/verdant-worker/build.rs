use std::{env, fs, path::{Path, PathBuf}};
use sha2::{Digest, Sha256};

fn collect(path: &Path, files: &mut Vec<PathBuf>) {
    let metadata = fs::symlink_metadata(path)
        .unwrap_or_else(|error| panic!("required synthesis source input {}: {error}", path.display()));
    if metadata.is_file() { files.push(path.to_owned()); return; }
    assert!(metadata.is_dir(), "required synthesis source input must be a regular file or directory: {}", path.display());
    let mut entries: Vec<_> = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("required synthesis source directory {}: {error}", path.display()))
        .map(|entry| entry.unwrap_or_else(|error| panic!("required synthesis source entry in {}: {error}", path.display())).path()).collect();
    entries.sort();
    for entry in entries {
        if entry.file_name().is_some_and(|s| s == "target" || s == "__pycache__") { continue; }
        collect(&entry, files);
    }
}
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../../..").canonicalize().unwrap();
    let mut files = Vec::new();
    // The worker calls into verdant-backend (voice registry, artifact resolution),
    // so a backend change must change the burned-in source fingerprint too.
    for name in ["frontend/crates/verdant-synthesis", "frontend/crates/verdant-worker", "frontend/crates/verdant-backend", "frontend/crates/verdant-voice-format", "frontend/Cargo.toml", "frontend/Cargo.lock",
        "backend/worker-request.schema.json",
        // These frozen inputs are required even though their containing crate
        // directories are collected above. An incomplete checkout must fail.
        "frontend/crates/verdant-backend/src/data/pinyin_inventory.json",
        "frontend/crates/verdant-backend/src/data/phoneme_inventory.json",
        "frontend/crates/verdant-synthesis/src/model/data/original_context_profile.json",
        "frontend/crates/verdant-synthesis/src/model/data/original_context_extensions.json",
        "frontend/crates/verdant-synthesis/src/model/data/original_context_hierarchy.json",
        "frontend/crates/verdant-synthesis/src/model/data/original_context_pos.json",
        "frontend/crates/verdant-synthesis/src/model/contracts/contracts.json",
        "frontend/crates/verdant-synthesis/src/model/contracts/contracts-source-filter.json",
        "frontend/crates/verdant-backend/tests/fixtures/kana_v1.json",
        "frontend/crates/verdant-backend/tests/fixtures/phonemes_v2.json",
        "frontend/crates/verdant-backend/tests/fixtures/japanese_prosody_v1.json",
        "frontend/crates/verdant-synthesis/src/model/fixtures/original-hierarchy-synthetic-fixture.json",
        "frontend/crates/verdant-synthesis/src/voice/fixtures/neighbor-contexts-v2.json"] {
        collect(&root.join(name), &mut files);
    }
    files.sort(); files.dedup();
    let mut source = String::from("pub const SOURCE_FILES: &[(&str, &str)] = &[\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let key = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let digest = Sha256::digest(fs::read(&path)
            .unwrap_or_else(|error| panic!("required synthesis source bytes {}: {error}", path.display())));
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        source.push_str(&format!("({key:?}, {hex:?}),\n"));
    }
    source.push_str("];\n");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("sources.rs"), source).unwrap();
}
