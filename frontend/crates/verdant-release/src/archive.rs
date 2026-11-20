//! No member is extracted to an archive-supplied path. Ordinary content is
//! streamed; only nested containers are spooled. All limits fail closed and
//! share one budget across the entire staging directory.
use std::{cell::RefCell, collections::{BTreeMap, BTreeSet}, fs, io::{Cursor, Read, Seek, SeekFrom, Write}, path::Path, rc::Rc};
use anyhow::{Context, Result, bail, ensure};
use flate2::read::MultiGzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use crate::{files::{self, Entry}, package::{self, Manifest}, policy::{self, AdmittedPolicy}, source, target::Target};

const MAX_DEPTH: usize = 8;
const MAX_MEMBERS: u64 = 100_000;
const MAX_ARCHIVE: u64 = 512 * 1024 * 1024;
const MAX_NESTED: u64 = 256 * 1024 * 1024;
const MAX_EXPANDED: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Default)]
struct Counts { members: u64, expanded: u64, packages: u64, sources: u64 }
#[derive(Clone, Default)]
struct Budget(Rc<RefCell<Counts>>);
impl Budget {
    fn consume(&self, bytes: u64) -> Result<()> {
        let mut counts = self.0.borrow_mut();
        ensure!(bytes <= MAX_EXPANDED.saturating_sub(counts.expanded), "expanded archive bytes exceed verification limit");
        counts.expanded += bytes;
        Ok(())
    }
    fn member(&self, bytes: u64) -> Result<()> {
        {
            let mut counts = self.0.borrow_mut();
            ensure!(counts.members < MAX_MEMBERS, "archive member count exceeds verification limit");
            counts.members += 1;
        }
        self.consume(bytes)
    }
}

struct Measured<R> { inner: R, budget: Budget, bytes: u64 }
impl<R: Read> Read for Measured<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.budget.consume(count as u64).map_err(std::io::Error::other)?;
        self.bytes += count as u64;
        Ok(count)
    }
}

#[derive(Clone, Copy, Debug)]
enum Kind { Zip, Tar, Gzip, Unsupported }
fn kind(name: &str, prefix: &[u8]) -> Option<Kind> {
    let name = name.to_ascii_lowercase();
    if name.ends_with(".zip") || prefix.starts_with(b"PK\x03\x04") || prefix.starts_with(b"PK\x05\x06") || prefix.starts_with(b"PK\x07\x08") { Some(Kind::Zip) }
    else if prefix.starts_with(b"\x1f\x8b") || name.ends_with(".tar.gz") || name.ends_with(".tgz") { Some(Kind::Gzip) }
    else if prefix.starts_with(b"BZh") || prefix.starts_with(b"\xfd7zXZ\0")
        || [".tar.xz", ".txz", ".tar.bz2", ".tbz2", ".7z", ".rar", ".cab"].iter().any(|suffix| name.ends_with(suffix))
        || prefix.starts_with(b"7z\xbc\xaf\x27\x1c") || prefix.starts_with(b"Rar!") || prefix.starts_with(b"MSCF") { Some(Kind::Unsupported) }
    else if name.ends_with(".tar") || prefix.get(257..262) == Some(b"ustar") { Some(Kind::Tar) }
    else { None }
}

fn native(prefix: &[u8]) -> bool {
    prefix.starts_with(b"\x7fELF") || prefix.starts_with(b"MZ")
        || [b"\xcf\xfa\xed\xfe".as_slice(), b"\xfe\xed\xfa\xcf", b"\xfe\xed\xfa\xce", b"\xce\xfa\xed\xfe", b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca"]
            .iter().any(|magic| prefix.starts_with(magic))
}

#[derive(Debug)]
enum Proof {
    Source { selection_sha256: String, source_commit: String, files: usize },
    Package { name: String, windows: bool },
    Other,
}
struct Observed { entry: Entry, json: Option<Vec<u8>>, native: Option<Vec<u8>>, nested: Option<Proof> }
#[derive(Default)]
struct Inventory { files: BTreeMap<String, Observed>, names: BTreeSet<String>, directories: BTreeSet<String> }
impl Inventory {
    fn name(&mut self, name: &str, directory: bool) -> Result<()> {
        files::portable_path(name)?;
        ensure!(!policy::forbidden_name(name, false), "restricted artifact identity/directory: {name}");
        let folded = name.to_ascii_lowercase();
        for (index, _) in folded.match_indices('/') {
            let ancestor = &folded[..index];
            ensure!(!self.names.contains(ancestor) || self.directories.contains(ancestor), "file used as an archive directory: {name}");
        }
        if !directory {
            let prefix = format!("{folded}/");
            ensure!(!self.names.range(prefix.clone()..).next().is_some_and(|path| path.starts_with(&prefix)), "file replaces an archive directory: {name}");
        }
        ensure!(self.names.insert(folded.clone()), "duplicate/colliding archive path: {name}");
        if directory { self.directories.insert(folded); }
        Ok(())
    }
    fn insert(&mut self, observed: Observed) -> Result<()> {
        self.name(&observed.entry.path, false)?;
        self.files.insert(observed.entry.path.clone(), observed);
        Ok(())
    }
}

struct Audit<'a> { budget: Budget, policy: Option<&'a AdmittedPolicy>, expected_commit: Option<&'a str>, expected_selection: Option<&'a str>, require_voice: bool }
impl Audit<'_> {
    fn member(&self, reader: &mut impl Read, name: &str, size: u64, mode: u32, depth: usize) -> Result<Observed> {
        self.budget.member(size)?;
        let mut prefix = vec![0u8; usize::try_from(size.min(512))?];
        reader.read_exact(&mut prefix).with_context(|| format!("truncated archive member: {name}"))?;
        let archive_kind = kind(name, &prefix);
        let is_native = native(&prefix);
        if archive_kind.is_some() {
            ensure!(depth < MAX_DEPTH, "archive nesting exceeds verification limit");
            ensure!(size <= if depth == 0 { MAX_ARCHIVE } else { MAX_NESTED }, "nested archive size exceeds verification limit");
        }
        let keep_json = name.ends_with("RELEASE-ASSETS.json") || name.ends_with("SOURCE-SELECTION.json")
            || name.ends_with("BUNDLE-POLICY.json") || name.ends_with("bundled-voice/manifest.json") || name.ends_with(".sha256");
        ensure!(!keep_json || size <= files::MAX_JSON, "admission JSON/checksum exceeds verification limit");
        let mut json_bytes = keep_json.then(|| prefix.clone());
        let mut native_header = is_native.then(|| prefix.clone());
        let mut spool = archive_kind.map(|_| tempfile::tempfile()).transpose()?;
        if let Some(spool) = &mut spool { spool.write_all(&prefix)?; }
        let mut hash = Sha256::new(); hash.update(&prefix);
        let mut remaining = size - prefix.len() as u64;
        let mut buffer = [0u8; 64 * 1024];
        while remaining > 0 {
            let wanted = usize::try_from(remaining.min(buffer.len() as u64))?;
            let count = reader.read(&mut buffer[..wanted])?;
            ensure!(count != 0, "truncated archive member: {name}");
            remaining -= count as u64; hash.update(&buffer[..count]);
            if let Some(bytes) = &mut json_bytes { bytes.extend_from_slice(&buffer[..count]); }
            if let Some(header) = &mut native_header {
                let take = count.min(65_562usize.saturating_sub(header.len()));
                header.extend_from_slice(&buffer[..take]);
            }
            if let Some(spool) = &mut spool { spool.write_all(&buffer[..count])?; }
        }
        let mut extra = [0u8; 1];
        ensure!(reader.read(&mut extra)? == 0, "archive member exceeds declared size: {name}");
        let nested = if let (Some(mut spool), Some(kind)) = (spool, archive_kind) {
            spool.seek(SeekFrom::Start(0))?;
            Some(self.container(&mut spool, kind, name, depth)?)
        } else { None };
        Ok(Observed { entry: Entry { path: name.into(), bytes: size, sha256: hex::encode(hash.finalize()), mode },
            json: json_bytes, native: native_header, nested })
    }

    fn container(&self, reader: &mut (impl Read + Seek), kind: Kind, location: &str, depth: usize) -> Result<Proof> {
        ensure!(depth < MAX_DEPTH, "archive nesting exceeds verification limit");
        let inventory = match kind {
            Kind::Zip => self.zip(reader, depth + 1),
            Kind::Tar => self.tar(reader, depth + 1),
            Kind::Gzip => self.tar(MultiGzDecoder::new(reader), depth + 1),
            Kind::Unsupported => bail!("unsupported archive compression/container; verification fails closed: {location}"),
        }.with_context(|| format!("cannot verify archive {location}"))?;
        self.validate(inventory, location, depth)
    }

    fn zip(&self, reader: &mut (impl Read + Seek), depth: usize) -> Result<Inventory> {
        let mut archive = zip::ZipArchive::new(reader)?;
        ensure!(archive.len() as u64 <= MAX_MEMBERS, "archive member count exceeds verification limit");
        let mut inventory = Inventory::default();
        for index in 0..archive.len() {
            let mut member = archive.by_index(index)?;
            let path = std::str::from_utf8(member.name_raw()).context("archive member paths must be UTF-8")?.to_owned();
            ensure!(!path.contains('\0'), "NUL archive path");
            let mode = member.unix_mode().unwrap_or(0);
            let directory = member.is_dir();
            let file_type = mode & 0o170000;
            ensure!(file_type == 0 || file_type == if directory { 0o040000 } else { 0o100000 }, "symlink/special ZIP member: {path}");
            if directory {
                self.budget.member(0)?;
                ensure!(member.size() == 0, "ZIP directory hides a payload: {path}");
                inventory.name(path.trim_end_matches('/'), true)?;
            } else {
                let size = member.size();
                let observed = self.member(&mut member, &path, size, if mode & 0o111 != 0 { 0o755 } else { 0o644 }, depth)?;
                inventory.insert(observed)?;
            }
        }
        Ok(inventory)
    }

    fn tar(&self, reader: impl Read, depth: usize) -> Result<Inventory> {
        let measured = Measured { inner: reader, budget: self.budget.clone(), bytes: 0 };
        let mut archive = tar::Archive::new(measured);
        let mut inventory = Inventory::default();
        let mut data_end = 0u64;
        for member in archive.entries()? {
            let mut member = member?;
            let path_bytes = member.path_bytes();
            let path = std::str::from_utf8(&path_bytes)?.to_owned();
            let kind = member.header().entry_type();
            ensure!(kind.is_file() || kind.is_dir(), "link/special tar member: {path}");
            let size = member.size();
            data_end = data_end.max(member.raw_file_position().checked_add(size.div_ceil(512) * 512).context("tar offset overflow")?);
            let mode = member.header().mode()?;
            ensure!(mode & 0o7000 == 0, "privileged tar member mode: {path}");
            if kind.is_dir() {
                self.budget.member(0)?;
                ensure!(size == 0, "tar directory hides a payload: {path}");
                inventory.name(path.trim_end_matches('/'), true)?;
            } else {
                let observed = self.member(&mut member, &path, size, if mode & 0o111 != 0 { 0o755 } else { 0o644 }, depth)?;
                inventory.insert(observed)?;
            }
        }
        let mut reader = archive.into_inner();
        let mut trailing = [0u8; 64 * 1024];
        loop {
            let count = reader.read(&mut trailing)?;
            if count == 0 { break; }
            ensure!(trailing[..count].iter().all(|byte| *byte == 0), "nonzero data after tar end-of-archive");
        }
        ensure!(reader.bytes >= data_end.checked_add(1024).context("tar offset overflow")? && reader.bytes % 512 == 0,
            "tar archive lacks complete end-of-archive blocks");
        Ok(inventory)
    }

    fn directory(&self, root: &Path) -> Result<Inventory> {
        fn visit(audit: &Audit<'_>, root: &Path, directory: &Path, inventory: &mut Inventory) -> Result<()> {
            ensure!(fs::symlink_metadata(directory)?.is_dir(), "symlink or special package directory");
            let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                let relative = path.strip_prefix(root)?.to_str().context("package path is not UTF-8")?.replace('\\', "/");
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    audit.budget.member(0)?;
                    inventory.name(&relative, true)?;
                    visit(audit, root, &path, inventory)?;
                } else {
                    ensure!(kind.is_file(), "symlink or special package member: {relative}");
                    let mut file = files::regular(&path)?;
                    let size = file.metadata()?.len();
                    let observed = audit.member(&mut file, &relative, size, files::file_mode(&path)?, 0)?;
                    inventory.insert(observed)?;
                }
            }
            Ok(())
        }
        let mut inventory = Inventory::default();
        visit(self, root, root, &mut inventory)?;
        Ok(inventory)
    }

    fn validate(&self, inventory: Inventory, location: &str, depth: usize) -> Result<Proof> {
        let package_manifests: Vec<_> = inventory.files.keys().filter(|name| name.as_str() == "RELEASE-ASSETS.json" || name.ends_with("/RELEASE-ASSETS.json")).cloned().collect();
        let source_manifests: Vec<_> = inventory.files.keys().filter(|name| name.as_str() == "SOURCE-SELECTION.json" || name.ends_with("/SOURCE-SELECTION.json")).cloned().collect();
        if !package_manifests.is_empty() {
            ensure!(depth == 0 && package_manifests.len() == 1, "nested or ambiguous native package manifest: {location}");
            return self.package(inventory, &package_manifests[0]);
        }
        if !source_manifests.is_empty() {
            ensure!(source_manifests.len() == 1, "ambiguous selected-source proof: {location}");
            return self.source(inventory, &source_manifests[0]);
        }
        for observed in inventory.files.values() {
            ensure!(!policy::forbidden_name(&observed.entry.path, true) && observed.native.is_none(),
                "unapproved release artifact: {location}:{}", observed.entry.path);
            if let Some(Proof::Package { name, windows }) = &observed.nested {
                let expected = format!("{name}{}", if *windows { ".zip" } else { ".tar.gz" });
                let (prefix, filename) = observed.entry.path.rsplit_once('/').unwrap_or(("", &observed.entry.path));
                ensure!(filename == expected, "native archive filename differs from its package identity");
                let checksum = format!("{}{name}.sha256", if prefix.is_empty() { String::new() } else { format!("{prefix}/") });
                ensure!(inventory.files.contains_key(&checksum), "runtime archive is missing its published checksum");
            }
        }
        self.check_checksums(&inventory)?;
        Ok(Proof::Other)
    }

    fn strip(&self, inventory: Inventory, manifest: &str, basename: &str) -> Result<Inventory> {
        let prefix = manifest.strip_suffix(basename).context("invalid manifest path")?;
        for directory in &inventory.directories {
            ensure!(directory.starts_with(&prefix.to_ascii_lowercase()) || directory == prefix.trim_end_matches('/'), "directory outside the manifest package root");
        }
        let mut stripped = Inventory::default();
        for (path, mut observed) in inventory.files {
            let path = path.strip_prefix(prefix).context("files outside the manifest package root")?;
            files::portable_path(path)?;
            observed.entry.path = path.into();
            stripped.insert(observed)?;
        }
        Ok(stripped)
    }

    fn source(&self, inventory: Inventory, manifest_path: &str) -> Result<Proof> {
        ensure!(manifest_path == "SOURCE-SELECTION.json" || manifest_path == "verdant-source/SOURCE-SELECTION.json", "unapproved source offer root");
        let mut inventory = self.strip(inventory, manifest_path, "SOURCE-SELECTION.json")?;
        let proof = inventory.files.remove("SOURCE-SELECTION.json").context("source selection is missing")?;
        let selection: source::Selection = files::parse_json(proof.json.as_deref().context("source selection content is missing")?)?;
        source::validate(&selection)?;
        if let Some(expected) = self.expected_commit { ensure!(selection.source_commit == expected, "source offer is from another selected commit"); }
        if let Some(expected) = self.expected_selection { ensure!(proof.entry.sha256 == expected, "source offer differs from the independently selected source digest"); }
        ensure!(inventory.files.len() == selection.files.len(), "source offer membership differs from its selected-source proof");
        for file in &selection.files {
            let observed = inventory.files.get(&file.path).context("source offer omits a selected file")?;
            ensure!(file.matches(&observed.entry) && observed.native.is_none(), "selected source bytes/mode changed or include a native binary: {}", file.path);
        }
        self.budget.0.borrow_mut().sources += 1;
        Ok(Proof::Source { selection_sha256: proof.entry.sha256, source_commit: selection.source_commit, files: selection.files.len() })
    }

    fn package(&self, inventory: Inventory, manifest_path: &str) -> Result<Proof> {
        let package_root = manifest_path.strip_suffix("RELEASE-ASSETS.json").context("invalid manifest root")?.trim_end_matches('/').to_owned();
        let mut inventory = self.strip(inventory, manifest_path, "RELEASE-ASSETS.json")?;
        let declaration = inventory.files.remove("RELEASE-ASSETS.json").context("package manifest missing")?;
        let manifest: Manifest = files::parse_json(declaration.json.as_deref().context("package manifest content missing")?)?;
        let target = Target::parse(&manifest.target)?;
        ensure!(manifest.format == package::FORMAT && semver::Version::parse(&manifest.version).is_ok()
            && manifest.name == format!("verdant-{}-{}", manifest.version, target.name())
            && (package_root.is_empty() || package_root == manifest.name)
            && manifest.voice_package.name == "verdant-voice" && manifest.voice_package.version == manifest.version
            && !manifest.development_only_model_data && manifest.production_runtime == "Rust" && !manifest.production_python_required,
            "old/unapproved package schema, runtime or app/voice identity");
        ensure!(inventory.files.len() == manifest.files.len(), "package membership differs from RELEASE-ASSETS.json");
        let mut indexed = BTreeSet::new();
        for file in &manifest.files {
            files::portable_path(&file.path)?;
            ensure!(indexed.insert(file.path.as_str()) && files::valid_sha(&file.sha256) && matches!(file.mode, 0o644 | 0o755), "invalid/repeated package index entry");
            ensure!(&inventory.files.get(&file.path).context("indexed package member missing")?.entry == file, "package bytes/mode changed: {}", file.path);
        }
        let mut admitted = BTreeSet::new();
        ensure!(manifest.programs.len() == package::PROGRAMS.len(), "package omits or adds a native program");
        for (name, role) in package::PROGRAMS {
            let expected = target.program_path(name);
            let program = manifest.programs.iter().find(|program| program.name == *name).context("required native program is missing")?;
            ensure!(program.path == expected && program.role == *role && admitted.insert(expected.clone()), "native program role/path changed");
            let observed = inventory.files.get(&expected).context("native program bytes missing")?;
            ensure!(observed.entry.sha256 == program.sha256, "native program digest mismatch");
            let native = observed.native.as_deref().context("program is not a compiled native executable")?;
            target.admit_reader(&mut Cursor::new(native), false)?;
            if matches!(*role, "gpui" | "worker") {
                let features = program.runtime_features.as_ref().context("app/worker runtime identity probe missing")?;
                ensure!(features["schema"] == "verdant-runtime-features/1" && features["role"] == *role && features["version"] == manifest.version
                    && features["voice_package"] == json!({"name":"verdant-voice","version":manifest.version}), "app/worker probe identity differs from package");
                if *role == "worker" {
                    let pin = features.get("bundled_voice_policy_sha256");
                    if let Some(bundle) = &manifest.bundle {
                        ensure!(pin.and_then(Value::as_str) == Some(bundle.policy_sha256.as_str()), "worker policy pin differs from bundle authorization");
                    } else { ensure!(pin.is_none_or(Value::is_null), "worker contains an undeclared bundle policy pin"); }
                }
            } else { ensure!(program.runtime_features.is_none(), "unexpected native runtime probe role"); }
        }
        let runtime = target.runtime();
        ensure!(manifest.native_libraries.len() == runtime.files.iter().filter(|file| file.library).count(), "unapproved native runtime library count");
        for input in runtime.files {
            let path = if input.library { target.library_path(input.name) } else { format!("{}/onnxruntime/{}", target.docs_dir(), input.name) };
            let observed = inventory.files.get(&path).context("pinned runtime library/license is missing")?;
            ensure!(observed.entry.sha256 == input.sha256, "runtime library/license bytes are not pinned: {path}");
            if input.library {
                let library = manifest.native_libraries.iter().find(|library| library.path == path).context("runtime index omits required library")?;
                ensure!(library.version == runtime.version && library.sha256 == input.sha256 && library.input_sha256 == input.sha256
                    && admitted.insert(path.clone()), "runtime input identity/version changed");
                target.admit_reader(&mut Cursor::new(observed.native.as_deref().context("runtime is not native")?), true)?;
            }
        }
        let source = inventory.files.get(&manifest.source_offer.path).context("nested corresponding source is missing")?;
        ensure!(manifest.source_offer.path == format!("{}/verdant-source.tar.gz", target.docs_dir())
            && source.entry.sha256 == manifest.source_offer.sha256, "source archive identity/path changed");
        match source.nested.as_ref() {
            Some(Proof::Source { selection_sha256, source_commit, files }) => {
                ensure!(selection_sha256 == &manifest.source_offer.source_selection_sha256 && source_commit == &manifest.source_offer.source_commit
                    && *files == manifest.source_offer.files, "nested source differs from selected-source proof");
            }
            _ => bail!("nested source offer lacks complete selected-source verification"),
        }
        for required in ["LICENSE", "NOTICE.md", "RELEASE.md"] {
            ensure!(inventory.files.contains_key(&format!("{}/{required}", target.docs_dir())), "package omits current license/notice/release policy");
        }
        self.bundle(&manifest, target, &inventory, &mut admitted)?;
        for observed in inventory.files.values() {
            ensure!(!(policy::forbidden_name(&observed.entry.path, true) || observed.native.is_some()) || admitted.contains(&observed.entry.path),
                "unapproved native/model artifact in package: {}", observed.entry.path);
        }
        self.budget.0.borrow_mut().packages += 1;
        Ok(Proof::Package { name: manifest.name, windows: target.windows() })
    }

    fn bundle(&self, manifest: &Manifest, target: Target, inventory: &Inventory, admitted: &mut BTreeSet<String>) -> Result<()> {
        let Some(bundle) = &manifest.bundle else {
            ensure!(!self.require_voice && manifest.artifact_policy == "code-runtime/no-model-data/1", "formal/bundled release requires an explicitly authorized complete voice");
            ensure!(!inventory.files.keys().any(|path| path.starts_with(&format!("{}/bundled-voice/", target.data_dir()))
                || path == &format!("{}/BUNDLE-POLICY.json", target.data_dir())), "undeclared voice bundle/policy");
            return Ok(());
        };
        let policy = self.policy.context("a packaged policy cannot authorize itself; supply --artifact-policy and its reviewed SHA256")?;
        ensure!(manifest.artifact_policy == "authorized-voice/1" && bundle.policy_sha256 == policy.sha256
            && bundle.directory == format!("{}/bundled-voice", target.data_dir()) && bundle.policy_path == format!("{}/BUNDLE-POLICY.json", target.data_dir())
            && policy.policy.version == manifest.version && policy.policy.compatible_app_version == manifest.version, "voice policy/version/layout binding changed");
        let declaration = inventory.files.get(&bundle.policy_path).context("embedded artifact policy is missing")?;
        ensure!(declaration.entry.sha256 == policy.sha256, "embedded artifact policy differs from independent admission");
        let prefix = format!("{}/", bundle.directory);
        ensure!(inventory.files.keys().filter(|path| path.starts_with(&prefix)).count() == policy.policy.files.len(), "missing/undeclared bundled voice artifact");
        for file in &policy.policy.files {
            let path = format!("{prefix}{}", file.path);
            let actual = inventory.files.get(&path).context("bundle omits an admitted role")?;
            ensure!(actual.entry.sha256 == file.sha256 && actual.entry.bytes == file.bytes && actual.native.is_none() && actual.nested.is_none(), "bundle role/digest/size changed or hides executable/archive: {}", file.role);
            admitted.insert(path);
        }
        let runtime = inventory.files.get(&format!("{prefix}manifest.json")).context("voice runtime manifest missing")?;
        policy.policy.runtime_manifest(runtime.json.as_deref().context("voice runtime manifest content missing")?)?;
        Ok(())
    }

    fn check_checksums(&self, inventory: &Inventory) -> Result<()> {
        for (path, observed) in &inventory.files {
            if !path.ends_with(".sha256") { continue; }
            let bytes = observed.json.as_deref().context("checksum content missing")?;
            let text = std::str::from_utf8(bytes)?;
            ensure!(text.ends_with('\n') && text.lines().count() == 1, "malformed release checksum");
            let (sha256, name) = text.trim_end_matches('\n').split_once("  ").context("malformed release checksum")?;
            ensure!(files::valid_sha(sha256) && !name.contains('/'), "unsafe release checksum target");
            let prefix = path.rsplit_once('/').map_or("", |(prefix, _)| prefix);
            let target = if prefix.is_empty() { name.to_owned() } else { format!("{prefix}/{name}") };
            let target = inventory.files.get(&target).context("checksum refers to a missing release archive")?;
            ensure!(target.entry.sha256 == sha256 && target.nested.is_some(), "release checksum mismatch or non-archive target");
        }
        Ok(())
    }
}

type ExpectedSource<'a> = Option<&'a str>;

pub(crate) fn check(root: &Path, policy: Option<&AdmittedPolicy>, expected_commit: ExpectedSource<'_>, expected_selection: ExpectedSource<'_>, require_package: bool, require_voice: bool) -> Result<Value> {
    ensure!(fs::symlink_metadata(root)?.is_dir(), "release staging input is not a regular directory");
    if let Some(commit) = expected_commit { ensure!(commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()), "invalid expected source commit"); }
    if let Some(sha256) = expected_selection { ensure!(files::valid_sha(sha256), "invalid expected selected-source digest"); }
    let audit = Audit { budget: Budget::default(), policy, expected_commit, expected_selection, require_voice };
    let inventory = audit.directory(root)?;
    let file_count = inventory.files.len();
    audit.validate(inventory, &root.display().to_string(), 0)?;
    let counts = audit.budget.0.borrow();
    ensure!(!require_package || counts.packages > 0, "staging contains no verified runtime package; an empty/source-only check is insufficient");
    ensure!((expected_commit.is_none() && expected_selection.is_none()) || counts.sources > 0, "staging contains no verified selected-source proof");
    Ok(json!({"status":"allowed","policy":"RELEASE.md","root":root,"file_count":file_count,
        "verified_packages":counts.packages,"verified_source_offers":counts.sources,
        "members_checked":counts.members,"expanded_bytes_checked":counts.expanded,"production_python_required":false}))
}

pub(crate) fn check_source_file(path: &Path) -> Result<()> {
    let audit = Audit { budget: Budget::default(), policy: None, expected_commit: None, expected_selection: None, require_voice: false };
    let mut file = files::regular(path)?;
    let size = file.metadata()?.len();
    let observed = audit.member(&mut file, "verdant-source.tar.gz", size, 0o644, 0)?;
    ensure!(matches!(observed.nested, Some(Proof::Source { .. })), "source archive did not establish complete selected-source proof");
    Ok(())
}
