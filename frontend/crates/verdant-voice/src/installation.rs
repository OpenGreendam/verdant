use std::{collections::BTreeMap, fs, io::{self, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};

use serde_json::{Value, json};
use verdant_backend::voices::{DESKTOP_ORIGINAL_REFERENCE, PackageReceipt, VoiceAdmission, VoiceAssets, VoiceError, VoiceRegistry, verify_original_raw_pair};
use verdant_synthesis::model;
use verdant_voice_format as format;

use crate::{Error, Result, args::{self, ReviewedPolicy}, files, platform::{self, RegistryLock}, require, strict_json};

fn registry_error(error: VoiceError) -> Error { Error::Invalid(error.to_string()) }

struct RegistryEdit {
    path: PathBuf,
    before: Option<Vec<u8>>,
    mode: u32,
    document: Value,
    registry: VoiceRegistry,
    lock: RegistryLock,
}

impl RegistryEdit {
    fn begin(path: &Path) -> Result<Self> {
        let path = args::absolute(path)?;
        let parent = path.parent().ok_or_else(|| Error::Invalid("registry needs a parent directory".into()))?;
        files::create_private_parents(parent)?;
        let path = fs::canonicalize(parent)?.join(path.file_name().ok_or_else(|| Error::Invalid("registry needs a file name".into()))?);
        let lock = RegistryLock::acquire(&path)?;
        let (before, mode, document, registry) = if files::present(&path)? {
            platform::require_registry_owner(&path)?;
            let bytes = files::read(&path)?;
            let document = strict_json::parse(&bytes)?;
            let registry = VoiceRegistry::load(&path).map_err(registry_error)?;
            require(files::read(&path)? == bytes, "registry changed during inspection")?;
            let mode = platform::file_mode(&files::regular(&path)?);
            (Some(bytes), mode, document, registry)
        } else {
            (None, 0o600, json!({"version":1,"voices":{}}), VoiceRegistry::new())
        };
        Ok(Self { path, before, mode, document, registry, lock })
    }

    fn save_entry(&mut self, reference: &str) -> Result<()> {
        let serialization = serde_json::to_value(&self.registry)?;
        let entry = serialization["voices"][reference].as_object()
            .ok_or_else(|| Error::Invalid("validated voice has no registry entry".into()))?;
        let voices = self.document.as_object_mut().expect("validated registry object")
            .entry("voices").or_insert_with(|| json!({}));
        voices[reference] = Value::Object(entry.clone());
        Ok(())
    }

    fn save_default(&mut self) -> Result<()> {
        let serialized = serde_json::to_value(&self.registry)?;
        let document = self.document.as_object_mut().expect("validated registry object");
        for key in ["default_voice", "managed_default"] {
            if let Some(value) = serialized.get(key) { document.insert(key.into(), value.clone()); }
            else { document.remove(key); }
        }
        Ok(())
    }

    fn unchanged(&self) -> Result<()> {
        self.lock.unchanged()?;
        if let Some(before) = &self.before {
            platform::require_registry_owner(&self.path)?;
            require(files::read(&self.path)? == *before, "registry changed during the transaction")
        } else {
            require(!files::present(&self.path)?, "registry appeared during the transaction")
        }
    }

    fn commit(&self) -> Result<()> {
        let after = strict_json::bytes(&self.document)?;
        self.unchanged()?;
        if let Err(error) = files::atomic_bytes(&self.path, &after, self.mode, self.before.is_some()) {
            if matches!(&error, Error::Io(cause) if cause.kind() == io::ErrorKind::AlreadyExists) { return Err(error); }
            // A directory sync can fail after the replacement became visible.
            // Restore only while our intended bytes still occupy the registry.
            if self.lock.unchanged().is_ok() && files::read(&self.path).is_ok_and(|current| current == after) {
                let restored = if let Some(before) = &self.before {
                    files::atomic_bytes(&self.path, before, self.mode, true)
                } else {
                    fs::remove_file(&self.path).map_err(Error::from)
                        .and_then(|()| files::sync_directory(self.path.parent().expect("registry parent")))
                };
                if let Err(rollback) = restored {
                    return Err(Error::Invalid(format!("{error}; registry restoration failed: {rollback}")));
                }
            }
            return Err(error);
        }
        Ok(())
    }
}

const MAX_RECORD_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ZIP_BYTES: u64 = 512 * 1024 * 1024;

fn member_limit(name: &str) -> Option<u64> {
    if name == "manifest.json" { Some(format::MAX_MANIFEST_BYTES as u64) }
    else if format::PACKAGE_FILES.iter().any(|&(_, file)| file == name) { Some(format::MAX_ARTIFACT_BYTES as u64) }
    else if format::RECORD_FILES.iter().any(|&(_, file)| file == name) { Some(MAX_RECORD_BYTES) }
    else { None }
}

fn root_member(name: &str) -> Result<u64> {
    member_limit(name).ok_or_else(|| Error::Invalid(format!("unexpected voice package member: {name:?}")))
}

fn zip_directory(mut file: &fs::File, length: u64) -> Result<(u64, usize)> {
    // Package sizes and member counts fit classic single-disk ZIP. Bound the
    // central directory before the ZIP library allocates per-member metadata.
    const END_BYTES: usize = 22;
    const ENTRY_BYTES: usize = 46;
    let tail_size = length.min((END_BYTES + u16::MAX as usize) as u64) as usize;
    let mut tail = vec![0; tail_size];
    file.seek(SeekFrom::Start(length - tail_size as u64))?;
    file.read_exact(&mut tail)?;
    let word = |bytes: &[u8], offset| u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
    let double_word = |bytes: &[u8], offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as u64;
    let end = tail.windows(END_BYTES).enumerate().rev().find_map(|(index, header)| {
        (&header[..4] == b"PK\x05\x06" && index + END_BYTES + word(header, 20) as usize == tail_size).then_some(index)
    })
        .ok_or_else(|| Error::Invalid("missing voice package ZIP end record".into()))?;
    let header = &tail[end..];
    let count = word(header, 10) as usize;
    require(word(header, 4) == 0 && word(header, 6) == 0 && word(header, 8) as usize == count,
        "voice package ZIP must have one disk")?;
    require(count <= format::PACKAGE_FILES.len() + format::RECORD_FILES.len(), "too many voice package ZIP members")?;
    let start = double_word(header, 16);
    let directory_end = start + double_word(header, 12);
    require(directory_end == length - tail_size as u64 + end as u64, "invalid voice package ZIP directory range")?;
    let mut offset = start;
    let mut entry = [0; ENTRY_BYTES];
    for _ in 0..count {
        require(offset + ENTRY_BYTES as u64 <= directory_end, "truncated voice package ZIP directory")?;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut entry)?;
        require(&entry[..4] == b"PK\x01\x02", "invalid voice package ZIP directory entry")?;
        offset += ENTRY_BYTES as u64 + u64::from(word(&entry, 28)) + u64::from(word(&entry, 30)) + u64::from(word(&entry, 32));
    }
    require(offset == directory_end, "voice package ZIP directory contains unaccounted entries")?;
    Ok((start, count))
}

fn stage_zip(input: &Path, stage: &Path) -> Result<()> {
    let file = files::open_regular(input)?;
    let before = file.metadata()?;
    require(before.len() <= MAX_ZIP_BYTES, "voice package ZIP exceeds its size limit")?;
    let (directory, count) = zip_directory(&file, before.len())?;
    let mut archive = zip::ZipArchive::new(&file).map_err(|error| Error::Invalid(format!("invalid voice package ZIP: {error}")))?;
    require(archive.len() == count && archive.central_directory_start() == directory && archive.offset() == 0,
        "voice package ZIP has duplicate entries or a mismatched directory")?;
    let mut total = 0;
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).map_err(|error| Error::Invalid(format!("invalid ZIP member: {error}")))?;
        let name = std::str::from_utf8(member.name_raw()).map_err(|_| Error::Invalid("ZIP member name must be UTF-8".into()))?;
        let limit = root_member(name)?;
        let size = member.size();
        require(!member.is_dir() && member.unix_mode().is_none_or(|mode| matches!(mode & 0o170000, 0 | 0o100000)),
            format!("ZIP member is not a regular file: {name}"))?;
        require(size <= limit && size <= MAX_ZIP_BYTES - total, format!("ZIP member exceeds its size limit: {name}"))?;
        total += size;
        let mut output = fs::OpenOptions::new().write(true).create_new(true).open(stage.join(name))?;
        let copied = io::copy(&mut (&mut member).take(size + 1), &mut output)?;
        require(copied == size, "ZIP member size changed during extraction")?;
        output.flush()?;
    }
    require(files::same_file(&before, &file.metadata()?) && files::matches_path(&file, input)?, "voice package ZIP changed during extraction")
}

fn stage_directory(input: &Path, stage: &Path) -> Result<()> {
    files::directory(input)?;
    for entry in fs::read_dir(input)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| Error::Invalid("voice package member name must be UTF-8".into()))?;
        let limit = root_member(name)?;
        files::copy_bounded(&entry.path(), &stage.join(name), limit)?;
    }
    Ok(())
}

struct Identity {
    name: String,
    version: String,
    members: BTreeMap<String, (u64, String)>,
    source_sha256: BTreeMap<String, String>,
    receipt: PackageReceipt,
}

impl Identity {
    fn verify(root: &Path, policy: Option<&ReviewedPolicy>) -> Result<Self> {
        let (manifest, bytes) = format::read_manifest(root).map_err(|error| Error::Invalid(format!("invalid voice package manifest: {error:#}")))?;
        require(manifest.version == env!("CARGO_PKG_VERSION"), "voice package version does not match this Verdant version")?;
        let manifest_hash = files::digest(&bytes);
        let mut members = BTreeMap::from([("manifest.json".to_owned(), (bytes.len() as u64, manifest_hash.clone()))]);
        for artifact in manifest.artifacts.values().chain(manifest.records.values()) {
            members.insert(artifact.file.clone(), (artifact.bytes, artifact.sha256.clone()));
        }
        let mut runtime_hashes = BTreeMap::from([("verdant_voice_manifest".to_owned(), manifest_hash)]);
        runtime_hashes.extend(manifest.artifacts.into_iter().map(|(role, asset)| (role, asset.sha256)));
        verify_files(root, &members)?;
        let (package, policy_sha256) = if let Some(policy) = policy {
            let policy_path = args::absolute(&policy.path)?;
            let evidence = format::policy::verify(root, &policy_path, &policy.sha256)
                .map_err(|error| Error::Invalid(format!("artifact policy admission failed: {error:#}")))?;
            (model::load_voice_package_with_manifest_pin(root, &evidence.manifest_sha256), Some(evidence.policy_sha256))
        } else {
            (model::load_voice_package(root), None)
        };
        let package = package.map_err(|error| Error::Invalid(format!("voice package admission failed: {error:#}")))?;
        require(package.hashes == runtime_hashes, "voice package changed during pinned runtime admission")?;
        let source_sha256 = serde_json::from_value(package.voice.provenance["source_sha256"].clone())?;
        let mut receipt = PackageReceipt::new(runtime_hashes["verdant_voice_manifest"].clone(), runtime_hashes["bank_tensors"].clone(), manifest.version.clone());
        receipt.artifact_policy_sha256 = policy_sha256;
        Ok(Self { name: manifest.name, version: manifest.version, members, source_sha256, receipt })
    }
}

fn verify_files(root: &Path, members: &BTreeMap<String, (u64, String)>) -> Result<()> {
    let expected = members.iter().map(|(name, (size, hash))| (name.as_str(), (*size, hash.as_str()))).collect();
    files::verify_package(root, &expected)
}

pub(crate) fn install(input: &Path, registry: &Path, set_default: bool, policy: Option<&ReviewedPolicy>) -> Result<Value> {
    let input = args::absolute(input)?;
    let parent = args::absolute(&platform::user_directory("data")?.join("voices"))?;
    let parent_resolved = files::resolve_missing(&parent)?;
    let metadata = fs::symlink_metadata(&input)?;
    if metadata.is_dir() {
        files::directory(&input)?;
        require(!parent_resolved.starts_with(fs::canonicalize(&input)?), "package source directory contains the installation root")?;
    }
    files::create_private_parents(&parent)?;
    let parent = fs::canonicalize(parent)?;
    let stage = files::OwnedDirectory::temporary(&parent, ".verdant-voice-stage-")?;
    if metadata.is_dir() { stage_directory(&input, &stage.path)?; }
    else { stage_zip(&input, &stage.path)?; }
    let identity = Identity::verify(&stage.path, policy)?;
    let reference = format!("{}-{}", identity.name, identity.version);
    let destination = parent.join(&reference);
    let mut edit = RegistryEdit::begin(registry)?;
    let package_lock = RegistryLock::acquire(&destination)?;
    let already_published = files::present(&destination)?;
    if already_published {
        verify_files(&destination, &identity.members)?;
    } else {
        for name in identity.members.keys() { files::sync_file(&stage.path.join(name))?; }
        files::sync_directory(&stage.path)?;
        platform::publish_directory(&stage.path, &destination)?;
    }

    let outcome = (|| -> Result<Value> {
        files::sync_directory(&parent)?;
        let assets = VoiceAssets { voice_package: Some(destination.clone()), ..VoiceAssets::default() };
        edit.registry.admit_with(reference.as_str(), format!("{} {}", identity.name, identity.version), assets, |_| {
            Ok(VoiceAdmission { source_sha256: identity.source_sha256.clone(), package_receipt: Some(identity.receipt.clone()), source_receipt: None })
        }).map_err(registry_error)?;
        edit.save_entry(&reference)?;
        let prior_default = edit.registry.default_reference().map(str::to_owned);
        if set_default {
            edit.registry.set_default(Some(&reference)).map_err(registry_error)?;
            edit.save_default()?;
        }
        verify_files(&destination, &identity.members)?;
        package_lock.unchanged()?;
        edit.commit()?;
        let bank_pin = model::compiled_voice_bank_sha256();
        let reason = match bank_pin {
            None => "missing compiled bank pin",
            Some(pin) if pin != identity.receipt.bank_sha256 => "compiled bank pin differs from installed package",
            Some(_) => "ONNX runtime admission was not checked during installation",
        };
        Ok(json!({"status":"installed", "installed":true, "render_ready":false, "reason":reason,
            "reference":reference, "directory":destination, "registry":edit.path,
            "default_voice":edit.registry.default_reference(), "default_changed":set_default && prior_default.as_deref() != Some(reference.as_str()),
            "package_receipt":identity.receipt}))
    })();
    if outcome.is_err() && !already_published && package_lock.unchanged().is_ok() && edit.unchanged().is_ok() {
        // Keep a published package if the registry commit's outcome is ambiguous.
        if let Err(error) = fs::remove_dir_all(&destination).and_then(|()| files::sync_directory(&parent).map_err(io::Error::other)) {
            return Err(Error::Invalid(format!("{}; could not remove unused package: {error}", outcome.unwrap_err())));
        }
    }
    outcome
}

pub(crate) fn list(registry: &Path) -> Result<Value> {
    let path = args::absolute(registry)?;
    if !files::present(&path)? { return Ok(json!({"registry":path, "default_voice":null, "voices":{}})); }
    let snapshot = files::read(&path)?;
    let registry = VoiceRegistry::load(&path).map_err(registry_error)?;
    require(files::read(&path)? == snapshot, "registry changed during inspection")?;
    let document = strict_json::parse(&snapshot)?;
    Ok(json!({"registry":path, "default_voice":registry.default_reference(), "voices":document.get("voices").cloned().unwrap_or_else(|| json!({}))}))
}

pub(crate) fn remove(registry: &Path, reference: &str) -> Result<Value> {
    let mut edit = RegistryEdit::begin(registry)?;
    let was_default = edit.registry.default_reference() == Some(reference);
    edit.registry.remove(reference).map_err(registry_error)?;
    let assets = &edit.document["voices"][reference]["assets"];
    let package_path = ["voice_package", "cfinal_model", "voice_model"].into_iter()
        .find_map(|role| assets[role].as_str()).map(|path| {
            let path = Path::new(path);
            if path.is_absolute() { path.to_path_buf() } else { edit.path.parent().expect("registry parent").join(path) }
        });
    edit.document["voices"].as_object_mut().expect("validated voice entries").remove(reference);
    if was_default { edit.save_default()?; }
    edit.commit()?;
    Ok(json!({"status":"removed", "reference":reference, "registry":edit.path, "default_voice":edit.registry.default_reference(),
        "package_path":package_path, "files_retained":true}))
}

pub(crate) fn set_default(registry: &Path, reference: &str) -> Result<Value> {
    let mut edit = RegistryEdit::begin(registry)?;
    edit.registry.set_default(Some(reference)).map_err(registry_error)?;
    edit.save_default()?;
    edit.commit()?;
    Ok(json!({"status":"selected", "default_voice":reference, "registry":edit.path}))
}

pub(crate) fn import_originals(registry: &Path, token: &Path, model_path: &Path) -> Result<Value> {
    let token = args::absolute(token)?;
    let model_path = args::absolute(model_path)?;
    files::regular(&token)?;
    files::regular(&model_path)?;
    let token = fs::canonicalize(token)?;
    let model_path = fs::canonicalize(model_path)?;
    files::path_text(&token)?;
    files::path_text(&model_path)?;
    let originals = verify_original_raw_pair(&token, &model_path).map_err(registry_error)?;
    let mut edit = RegistryEdit::begin(registry)?;
    let assets = VoiceAssets { token: Some(token.clone()), model: Some(model_path.clone()), ..VoiceAssets::default() };
    edit.registry.admit_with(DESKTOP_ORIGINAL_REFERENCE, "童声01 (C0 reconstruction)", assets, |_| {
        let voice = model::load_raw_voice(&token, &model_path).map_err(|error| format!("{error:#}"))?;
        if !voice.source_identity_confirmed() { return Err("raw loader did not verify the original voice identity".into()); }
        Ok(VoiceAdmission { source_sha256: originals.clone(), ..VoiceAdmission::default() })
    }).map_err(registry_error)?;
    edit.save_entry(DESKTOP_ORIGINAL_REFERENCE)?;
    require(verify_original_raw_pair(&token, &model_path).map_err(registry_error)? == originals, "raw source files changed during import")?;
    edit.commit()?;
    Ok(json!({"status":"imported", "reference":DESKTOP_ORIGINAL_REFERENCE, "registry":edit.path,
        "default_voice":edit.registry.default_reference(), "source_sha256":originals, "route":"C0"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

    fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(io::Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for &(name, bytes) in entries {
            writer.start_file(name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn zip_accepts_root_members_and_rejects_traversal_symlinks_and_duplicate_names() {
        let scratch = tempfile::tempdir().unwrap();
        let archive = scratch.path().join("package.zip");
        let stage = scratch.path().join("stage");
        fs::create_dir(&stage).unwrap();
        fs::write(&archive, zip_with(&[("manifest.json", b"bytes")])).unwrap();
        stage_zip(&archive, &stage).unwrap();
        assert_eq!(fs::read(stage.join("manifest.json")).unwrap(), b"bytes");
        fs::remove_file(stage.join("manifest.json")).unwrap();

        fs::write(&archive, zip_with(&[("../manifest.json", b"escape")])).unwrap();
        assert!(stage_zip(&archive, &stage).is_err());
        assert!(!scratch.path().join("manifest.json").exists());

        let mut writer = ZipWriter::new(io::Cursor::new(Vec::new()));
        writer.add_symlink("manifest.json", "elsewhere", SimpleFileOptions::default()).unwrap();
        fs::write(&archive, writer.finish().unwrap().into_inner()).unwrap();
        assert!(stage_zip(&archive, &stage).is_err());

        let mut duplicate = zip_with(&[("manifest.json", b"first"), ("manifest.jsoN", b"second")]);
        for index in 0..=duplicate.len() - b"manifest.jsoN".len() {
            if &duplicate[index..index + b"manifest.jsoN".len()] == b"manifest.jsoN" {
                duplicate[index + b"manifest.jsoN".len() - 1] = b'n';
            }
        }
        fs::write(&archive, duplicate).unwrap();
        assert!(stage_zip(&archive, &stage).is_err());
        assert_eq!(fs::read_dir(&stage).unwrap().count(), 0);
    }

    #[test]
    fn zip_rejects_oversized_expansion_before_writing_member() {
        let scratch = tempfile::tempdir().unwrap();
        let archive = scratch.path().join("package.zip");
        let stage = scratch.path().join("stage");
        fs::create_dir(&stage).unwrap();
        let uncompressed = vec![b'x'; format::MAX_MANIFEST_BYTES + 1];
        fs::write(&archive, zip_with(&[("manifest.json", &uncompressed)])).unwrap();
        assert!(stage_zip(&archive, &stage).is_err());
        assert_eq!(fs::read_dir(&stage).unwrap().count(), 0);
    }

    #[test]
    fn remove_retains_raw_files_and_clears_only_the_selected_reference() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path();
        fs::write(root.join("sample.tkn"), b"token").unwrap();
        fs::write(root.join("sample.bin"), b"model").unwrap();
        let registry = root.join("voices.json");
        let entry = json!({"display_name":"Sample", "assets":{"token":"sample.tkn", "model":"sample.bin"},
            "files":{"token":files::digest(b"token"), "model":files::digest(b"model")}});
        let before = json!({"version":1,"default_voice":"selected","managed_default":"selected",
            "voices":{"selected":entry,"other":entry}});
        fs::write(&registry, strict_json::bytes(&before).unwrap()).unwrap();
        let listed = list(&registry).unwrap();
        assert_eq!(listed["default_voice"], "selected");
        assert_eq!(listed["voices"], before["voices"]);
        assert_eq!(remove(&registry, "selected").unwrap()["files_retained"], true);
        let after = strict_json::parse(&fs::read(&registry).unwrap()).unwrap();
        assert_eq!(after, json!({"version":1,"voices":{"other":entry}}));
        assert_eq!(fs::read(root.join("sample.tkn")).unwrap(), b"token");
        assert_eq!(fs::read(root.join("sample.bin")).unwrap(), b"model");
        set_default(&registry, "other").unwrap();
        assert_eq!(list(&registry).unwrap()["default_voice"], "other");
    }

    #[test]
    fn registry_edit_refuses_a_competing_first_write() {
        let scratch = tempfile::tempdir().unwrap();
        let registry = scratch.path().join("voices.json");
        let edit = RegistryEdit::begin(&registry).unwrap();
        let competing = b"{\"version\":1,\"voices\":{}}\n";
        fs::write(&registry, competing).unwrap();
        assert!(edit.commit().is_err());
        assert_eq!(fs::read(&registry).unwrap(), competing);
    }

    #[test]
    fn original_import_checks_fixed_hashes_before_opening_registry() {
        let scratch = tempfile::tempdir().unwrap();
        let token = scratch.path().join("input.tkn");
        let model = scratch.path().join("input.bin");
        let registry = scratch.path().join("state/voices.json");
        fs::write(&token, b"invalid token").unwrap();
        fs::write(&model, b"invalid model").unwrap();
        let error = import_originals(&registry, &token, &model).unwrap_err();
        assert!(error.to_string().contains("do not match"));
        assert!(!registry.parent().unwrap().exists());
    }
}
