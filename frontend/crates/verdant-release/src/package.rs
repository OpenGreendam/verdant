use std::{fs::{self, OpenOptions}, io::{Read, Seek, SeekFrom}, path::{Path, PathBuf}, process::{Command, Stdio}, time::{Duration, Instant}};
use anyhow::{Context, Result, ensure};
use flate2::{Compression, GzBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use crate::{Args, archive, files::{self, Entry}, policy, source, target::{ORT_VERSION, Target}};

pub(crate) const FORMAT: &str = "verdant-native-package/2";
pub(crate) const PROGRAMS: &[(&str, &str)] = &[
    ("verdant", "gpui"), ("verdant-worker", "worker"), ("verdant-plughost", "plughost"),
    ("verdant-cli", "cli"), ("verdant-voice", "voice"), ("verdant-release", "release"),
];
const DOCUMENTS: &[&str] = &[
    "LICENSE", "NOTICE.md", "README.md", "RELEASE.md", "SYNTHESIS-FIDELITY.md", "CONTRIBUTING.md",
    "backend/RUNTIME.md", "frontend/README.md", "frontend/P0-DECISIONS.md", "docs/verdant-voice.md",
    "frontend/docs/PLUGIN-HOST.md", "frontend/docs/THIRD_PARTY-PLUGINS.md",
    "frontend/crates/verdant-app/assets/icons/LICENSE-lucide.txt",
];
const LAUNCHER: &str = "#!/bin/sh\nset -eu\nbindir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\nprefix=$(CDPATH= cd -- \"$bindir/..\" && pwd)\nexport VERDANT_DATA_DIR=\"$prefix/share/verdant\"\nexport VERDANT_PLUGIN_HOST=\"$prefix/libexec/verdant/verdant-plughost\"\nexport VERDANT_WORKER_BINARY=\"$prefix/bin/verdant-worker\"\nexport VERDANT_ONNXRUNTIME_LIBRARY=\"$prefix/lib/libonnxruntime.so.1.23.2\"\nexec \"$prefix/libexec/verdant/verdant\" \"$@\"\n";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Program {
    pub name: String, pub role: String, pub path: String, pub sha256: String,
    pub runtime_features: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Library { pub path: String, pub sha256: String, pub input_sha256: String, pub version: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceOffer {
    pub path: String, pub sha256: String, pub source_selection_sha256: String,
    pub source_commit: String, pub files: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bundle { pub directory: String, pub policy_path: String, pub policy_sha256: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VoiceVersion { pub name: String, pub version: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub format: String, pub name: String, pub version: String, pub target: String,
    pub voice_package: VoiceVersion, pub development_only_model_data: bool,
    pub production_runtime: String, pub production_python_required: bool,
    pub artifact_policy: String, pub bundle: Option<Bundle>,
    pub native_libraries: Vec<Library>, pub programs: Vec<Program>, pub source_offer: SourceOffer,
    pub files: Vec<Entry>,
}

pub(crate) fn version(root: &Path) -> Result<String> {
    let mut text = String::new(); files::regular(&root.join("frontend/Cargo.toml"))?.take(files::MAX_JSON + 1).read_to_string(&mut text)?;
    ensure!(text.len() as u64 <= files::MAX_JSON, "workspace manifest exceeds limit");
    let manifest: toml::Value = toml::from_str(&text)?;
    let version = manifest.get("workspace").and_then(|v| v.get("package")).and_then(|v| v.get("version"))
        .and_then(toml::Value::as_str).context("workspace version is missing")?.to_owned();
    semver::Version::parse(&version).context("workspace version must be semver")?;
    ensure!(version.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b)), "unsafe workspace version");
    Ok(version)
}

// Files, rather than pipes, bound both output and wait time even if a failed
// child leaves a descendant holding stdout open. The inputs are local build products.
struct NativeChild(std::process::Child);
impl Drop for NativeChild {
    fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}

fn run_bounded(command: &mut Command, timeout: Duration) -> Result<String> {
    let mut output = tempfile::tempfile()?;
    let mut errors = tempfile::tempfile()?;
    let mut child = NativeChild(command.stdin(Stdio::null()).stdout(output.try_clone()?).stderr(errors.try_clone()?).spawn()
        .with_context(|| format!("starting {command:?}"))?);
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait()? { break status; }
        if start.elapsed() >= timeout || output.metadata()?.len() > files::MAX_JSON || errors.metadata()?.len() > files::MAX_JSON {
            anyhow::bail!("native command exceeded deadline/output limit: {command:?}");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    ensure!(output.metadata()?.len() <= files::MAX_JSON && errors.metadata()?.len() <= files::MAX_JSON, "native command output exceeds limit");
    errors.seek(SeekFrom::Start(0))?;
    if !status.success() {
        let mut text = String::new(); errors.read_to_string(&mut text)?;
        anyhow::bail!("native command failed ({status}): {command:?}: {text}");
    }
    output.seek(SeekFrom::Start(0))?;
    let mut text = String::new(); output.read_to_string(&mut text)?;
    Ok(text)
}

fn probe(path: &Path, role: &str, version: &str) -> Result<Value> {
    let text = run_bounded(Command::new(path).arg("--runtime-features"), Duration::from_secs(15))?;
    let features: Value = files::parse_json(text.as_bytes())?;
    ensure!(features["schema"] == "verdant-runtime-features/1" && features["role"] == role && features["version"] == version
        && features["voice_package"] == json!({"name":"verdant-voice","version":version}), "app/worker/voice identity mismatch: {}", path.display());
    Ok(features)
}

fn runtime(directory: Option<PathBuf>, wheel: Option<PathBuf>, target: Target, stage: &Path) -> Result<Vec<Library>> {
    ensure!(directory.is_some() != wheel.is_some(), "choose exactly one --onnxruntime-dir or --onnxruntime-wheel");
    let mut archive = if let Some(wheel) = wheel {
        ensure!(files::digest(&wheel)? == target.runtime().wheel_sha256, "unapproved ONNX Runtime wheel");
        Some(zip::ZipArchive::new(files::regular(&wheel)?)?)
    } else { None };
    let mut libraries = Vec::new();
    for member in target.runtime().files {
        let destination = stage.join(if member.library { target.library_path(member.name) }
            else { format!("{}/onnxruntime/{}", target.docs_dir(), member.name) });
        if let Some(archive) = archive.as_mut() {
            let indices: Vec<_> = archive.file_names().enumerate().filter_map(|(index, name)|
                (name.rsplit('/').next() == Some(member.name)).then_some(index)).collect();
            ensure!(indices.len() == 1, "ambiguous/missing pinned runtime member: {}", member.name);
            let mut input = archive.by_index(indices[0])?;
            ensure!(input.is_file() && input.size() <= 128 * 1024 * 1024 && input.unix_mode().is_none_or(|mode| mode & 0o170000 == 0 || mode & 0o170000 == 0o100000), "invalid runtime wheel member");
            fs::create_dir_all(destination.parent().context("runtime destination parent")?)?;
            let mut output = OpenOptions::new().create_new(true).write(true).open(&destination)?;
            let copied = std::io::copy(&mut input.by_ref().take(128 * 1024 * 1024 + 1), &mut output)?;
            ensure!(copied == input.size(), "truncated runtime wheel member");
            output.sync_all()?;
            files::set_mode(&destination, if member.library { 0o755 } else { 0o644 })?;
        } else {
            let directory = directory.as_ref().context("runtime directory is missing")?;
            files::copy(&directory.join(member.name), &destination, member.library)?;
        }
        ensure!(files::digest(&destination)? == member.sha256, "unexpected ONNX Runtime {ORT_VERSION} bytes/license: {}", member.name);
        if member.library {
            target.admit(&destination, true)?;
            libraries.push(Library { path: destination.strip_prefix(stage)?.to_str().context("runtime path is not UTF-8")?.replace('\\', "/"),
                sha256: member.sha256.into(), input_sha256: member.sha256.into(), version: ORT_VERSION.into() });
        }
    }
    Ok(libraries)
}

fn selected_copy(blobs: &mut source::Blobs, selected: &source::Selection, relative: &str, destination: &Path) -> Result<()> {
    let entry = selected.files.iter().find(|entry| entry.path == relative).with_context(|| format!("package resource is absent from selected source: {relative}"))?;
    blobs.copy(entry, destination)
}

fn macos(stage: &Path, target: Target, version: &str) -> Result<()> {
    let plist = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>verdant</string><key>CFBundleIdentifier</key><string>org.verdant.Verdant</string><key>CFBundleName</key><string>Verdant</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>{version}</string><key>CFBundleVersion</key><string>{version}</string><key>LSMinimumSystemVersion</key><string>13.0</string><key>NSHighResolutionCapable</key><true/></dict></plist>\n");
    files::write_new(&stage.join("Verdant.app/Contents/Info.plist"), plist.as_bytes(), false)?;
    for (program, _) in PROGRAMS {
        run_bounded(Command::new("codesign").args(["--force", "--sign", "-", "--timestamp=none"]).arg(stage.join(target.program_path(program))), Duration::from_secs(60))?;
    }
    // Preserve the provider's exact runtime dylib bytes/signature. Re-signing
    // those would invalidate their pinned identity in a portable audit.
    run_bounded(Command::new("codesign").args(["--force", "--sign", "-", "--timestamp=none"]).arg(stage.join("Verdant.app")), Duration::from_secs(60))?;
    run_bounded(Command::new("codesign").args(["--verify", "--deep", "--strict"]).arg(stage.join("Verdant.app")), Duration::from_secs(60))?;
    Ok(())
}

fn zip_time(epoch: u64) -> Result<zip::DateTime> {
    let epoch = epoch.clamp(315_532_800, 4_354_819_199);
    let days = epoch / 86_400 + 719_468;
    let era = days / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    year += u64::from(month <= 2);
    zip::DateTime::from_date_and_time(year as u16, month as u8, day as u8, (epoch / 3600 % 24) as u8,
        (epoch / 60 % 60) as u8, (epoch % 60) as u8).context("invalid ZIP timestamp")
}

fn assembled_archive(stage: &Path, destination: &Path, name: &str, epoch: u64, windows: bool) -> Result<()> {
    let entries = files::walk(stage)?.iter().map(|path| files::record(stage, path)).collect::<Result<Vec<_>>>()?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(destination)?;
    if windows {
        let mut archive = zip::ZipWriter::new(&mut file);
        for entry in entries {
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
                .last_modified_time(zip_time(epoch)?).unix_permissions(entry.mode);
            archive.start_file(format!("{name}/{}", entry.path), options)?;
            let mut source = files::regular(&stage.join(&entry.path))?;
            std::io::copy(&mut source, &mut archive)?;
        }
        archive.finish()?;
    } else {
        let compressed = GzBuilder::new().mtime(u32::try_from(epoch)?).write(&mut file, Compression::default());
        let mut archive = tar::Builder::new(compressed);
        for entry in entries { source::append(&mut archive, stage, &entry, name, epoch)?; }
        archive.into_inner()?.finish()?;
    }
    file.sync_all()?;
    Ok(())
}

fn publish(prepared: &Path, output: &Path, name: &str, windows: bool) -> Result<(PathBuf, String)> {
    let filename = format!("{name}{}", if windows { ".zip" } else { ".tar.gz" });
    let archive = prepared.join(&filename);
    let checksum_name = format!("{name}.sha256");
    let checksum = prepared.join(&checksum_name);
    let sha256 = files::digest(&archive)?;
    let destinations = [(checksum, output.join(checksum_name)), (archive, output.join(filename))];
    let mut owned = Vec::new();
    for (source, destination) in &destinations {
        if let Err(error) = fs::hard_link(source, destination) {
            for (source, destination) in owned {
                if fs::symlink_metadata(&destination).is_ok_and(|meta| meta.is_file())
                    && same_file::is_same_file(&source, &destination).unwrap_or(false) {
                    let _ = fs::remove_file(destination);
                }
            }
            return Err(error).with_context(|| format!("publishing {} without overwrite", destination.display()));
        }
        owned.push((source.clone(), destination.clone()));
    }
    Ok((destinations[1].1.clone(), sha256))
}

pub(crate) fn run(mut args: Args) -> Result<Value> {
    let root = files::absolute(&args.path("--repo")?)?;
    let binaries = files::absolute(&args.path("--binaries")?)?;
    let output = files::absolute(&args.path("--output")?)?;
    let selection_path = files::absolute(&args.path("--source-selection")?)?;
    let selection_sha256 = args.required("--source-selection-sha256")?;
    let runtime_directory = args.take("--onnxruntime-dir").map(PathBuf::from);
    let runtime_wheel = args.take("--onnxruntime-wheel").map(PathBuf::from);
    let target = args.take("--target").map(|value| Target::parse(&value)).transpose()?.unwrap_or(Target::native()?);
    ensure!(target == Target::native()?, "package probes/signing must run on the native target build host");
    let version = version(&root)?;
    if let Some(asserted) = args.take("--version") { ensure!(asserted == version, "--version does not match the workspace"); }
    let epoch = args.epoch()?;
    let bundle = args.take("--voice-bundle").map(PathBuf::from);
    let policy = policy::external(&mut args)?;
    ensure!(bundle.is_some() == policy.is_some(), "--voice-bundle requires the explicit external artifact policy and vice versa");
    args.finish()?;
    let selected = source::load(&root, &selection_path, &selection_sha256)?;
    if let (Some(bundle), Some(policy)) = (&bundle, &policy) {
        ensure!(policy.policy.compatible_app_version == version, "bundle policy targets another app version");
        policy.policy.bundle(bundle)?;
    }
    let name = format!("verdant-{version}-{}", target.name());
    fs::create_dir_all(&output)?;
    for suffix in [if target.windows() { ".zip" } else { ".tar.gz" }, ".sha256"] {
        ensure!(fs::symlink_metadata(output.join(format!("{name}{suffix}"))).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound), "refusing to overwrite an existing/inaccessible publication");
    }
    let scratch = tempfile::Builder::new().prefix(".verdant-package-").tempdir_in(&output)?;
    let stage = scratch.path().join(&name); fs::create_dir(&stage)?;
    for (program, _) in PROGRAMS {
        let filename = format!("{program}{}", target.extension());
        let destination = stage.join(target.program_path(program));
        files::copy(&binaries.join(filename), &destination, true)?;
        target.admit(&destination, false)?;
    }
    let native_libraries = runtime(runtime_directory, runtime_wheel, target, &stage)?;
    let data = stage.join(target.data_dir()); let docs = stage.join(target.docs_dir());
    let mut source_blobs = source::Blobs::open(&root)?;
    selected_copy(&mut source_blobs, &selected, "backend/worker-request.schema.json", &data.join("backend/worker-request.schema.json"))?;
    for relative in DOCUMENTS { selected_copy(&mut source_blobs, &selected, relative, &docs.join(relative))?; }
    for entry in &selected.files {
        let synthesis = entry.path.strip_prefix("frontend/crates/verdant-synthesis/");
        let name = entry.path.rsplit('/').next().unwrap_or("");
        if let Some(relative) = synthesis.filter(|_| name.contains("LICENSE") || name.contains("NOTICE")) {
            selected_copy(&mut source_blobs, &selected, &entry.path, &docs.join("synthesis").join(relative))?;
        } else if let Some(relative) = entry.path.strip_prefix("spec/licenses/") {
            selected_copy(&mut source_blobs, &selected, &entry.path, &docs.join("source-licenses").join(relative))?;
        }
    }
    if !target.windows() && !target.mac() {
        files::write_new(&stage.join("bin/verdant"), LAUNCHER.as_bytes(), true)?;
        for (relative, destination) in [("frontend/packaging/org.verdant.Verdant.desktop", "share/applications/org.verdant.Verdant.desktop"),
            ("frontend/packaging/verdant-project.xml", "share/mime/packages/verdant-project.xml")] {
            selected_copy(&mut source_blobs, &selected, relative, &stage.join(destination))?;
        }
        let mut dependencies = String::new();
        for path in PROGRAMS.iter().map(|(name, _)| target.program_path(name)).chain(native_libraries.iter().map(|library| library.path.clone())) {
            let text = run_bounded(Command::new("ldd").arg(stage.join(&path)), Duration::from_secs(15))?;
            ensure!(!text.contains("not found"), "unresolved native dependencies in {path}: {text}");
            dependencies.push_str(&format!("[{path}]\n{text}\n"));
        }
        files::write_new(&docs.join("SYSTEM-LIBRARIES.txt"), dependencies.as_bytes(), false)?;
    }
    let bundle_receipt = if let (Some(bundle), Some(policy)) = (&bundle, &policy) {
        let directory = format!("{}/bundled-voice", target.data_dir());
        for member in &policy.policy.files { files::copy(&files::input(bundle, &member.path)?, &stage.join(&directory).join(&member.path), false)?; }
        let policy_path = format!("{}/BUNDLE-POLICY.json", target.data_dir());
        files::copy(&policy.path, &stage.join(&policy_path), false)?;
        ensure!(files::digest(&stage.join(&policy_path))? == policy.sha256, "external artifact policy changed during assembly");
        Some(Bundle { directory, policy_path, policy_sha256: policy.sha256.clone() })
    } else { None };
    if target.mac() { macos(&stage, target, &version)?; }
    let mut programs = Vec::new();
    for (name, role) in PROGRAMS {
        let path = target.program_path(name);
        let runtime_features = if matches!(*role, "gpui" | "worker") { Some(probe(&stage.join(&path), role, &version)?) } else { None };
        if *role == "worker" {
            let pin = runtime_features.as_ref().and_then(|features| features.get("bundled_voice_policy_sha256"));
            if let Some(policy) = &policy {
                ensure!(pin.and_then(Value::as_str) == Some(policy.sha256.as_str()), "worker was not built with this exact bundled artifact policy pin");
            } else {
                ensure!(pin.is_none_or(Value::is_null), "worker has a bundled policy pin but package has no authorized bundle");
            }
        }
        programs.push(Program { name: (*name).into(), role: (*role).into(), sha256: files::digest(&stage.join(&path))?, path, runtime_features });
    }
    let source_path = format!("{}/verdant-source.tar.gz", target.docs_dir());
    source::create(&root, &selection_path, &selection_sha256, &selected, &stage.join(&source_path), epoch)?;
    let source_offer = SourceOffer { path: source_path.clone(), sha256: files::digest(&stage.join(source_path))?,
        source_selection_sha256: selection_sha256, source_commit: selected.source_commit.clone(), files: selected.files.len() };
    let members = files::walk(&stage)?.iter().map(|path| files::record(&stage, path)).collect::<Result<Vec<_>>>()?;
    let count = members.len();
    let manifest = Manifest { format: FORMAT.into(), name: name.clone(), version: version.clone(), target: target.name().into(),
        voice_package: VoiceVersion { name: "verdant-voice".into(), version }, development_only_model_data: false,
        production_runtime: "Rust".into(), production_python_required: false,
        artifact_policy: if bundle_receipt.is_some() { "authorized-voice/1" } else { "code-runtime/no-model-data/1" }.into(),
        bundle: bundle_receipt, native_libraries, programs, source_offer, files: members };
    files::write_new_json(&stage.join("RELEASE-ASSETS.json"), &manifest)?;
    archive::check(&stage, policy.as_ref(), Some(&selected.source_commit), Some(&manifest.source_offer.source_selection_sha256), true, false)?;
    let prepared = scratch.path().join("publication"); fs::create_dir(&prepared)?;
    let archive_path = prepared.join(format!("{name}{}", if target.windows() { ".zip" } else { ".tar.gz" }));
    assembled_archive(&stage, &archive_path, &name, epoch, target.windows())?;
    // Assemble the checksum before the final audit; publication makes neither
    // path visible until both complete artifacts have passed the real guard.
    let sha256 = files::digest(&archive_path)?;
    let checksum_path = prepared.join(format!("{name}.sha256"));
    files::write_new(&checksum_path, format!("{sha256}  {}\n", archive_path.file_name().context("no archive name")?.to_str().context("non-UTF-8 archive name")?).as_bytes(), false)?;
    archive::check(&prepared, policy.as_ref(), Some(&selected.source_commit), Some(&manifest.source_offer.source_selection_sha256), true, false)?;
    let (archive, sha256) = publish(&prepared, &output, &name, target.windows())?;
    Ok(json!({"status":"packaged","archive":archive,"sha256":sha256,"payload_files":count,
        "target":target.name(),"source_commit":selected.source_commit,"voice_bundled":manifest.bundle.is_some()}))
}

#[cfg(test)]
mod publication_tests {
    use super::*;

    fn prepared(root: &Path, name: &str) -> PathBuf {
        let prepared = root.join("prepared"); fs::create_dir(&prepared).unwrap();
        let filename = format!("{name}.zip");
        files::write_new(&prepared.join(&filename), b"synthetic publication bytes", false).unwrap();
        let sha256 = files::digest(&prepared.join(&filename)).unwrap();
        files::write_new(&prepared.join(format!("{name}.sha256")), format!("{sha256}  {filename}\n").as_bytes(), false).unwrap();
        prepared
    }

    #[test]
    fn publication_never_replaces_existing_paths_and_rolls_back_only_its_checksum() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("output"); fs::create_dir(&output).unwrap();
        let name = "verdant-0.1.0-windows-x86_64";
        let prepared = prepared(root.path(), name);
        let archive = output.join(format!("{name}.zip"));
        fs::write(&archive, b"previous publication").unwrap();
        assert!(publish(&prepared, &output, name, true).is_err());
        assert_eq!(fs::read(&archive).unwrap(), b"previous publication");
        assert!(!output.join(format!("{name}.sha256")).exists());
        fs::remove_file(&archive).unwrap();
        let checksum = output.join(format!("{name}.sha256"));
        fs::write(&checksum, b"previous checksum").unwrap();
        assert!(publish(&prepared, &output, name, true).is_err());
        assert_eq!(fs::read(&checksum).unwrap(), b"previous checksum");
        assert!(!archive.exists());
    }
}
