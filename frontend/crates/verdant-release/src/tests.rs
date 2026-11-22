//! Synthetic bytes only. No voice, runtime library, bank or recording fixtures.
use std::{collections::BTreeMap, fs, io::{Cursor, Write}, path::Path, process::Command};
use sha2::{Digest, Sha256};
use crate::{archive, files, policy, source};

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut output);
        for (path, bytes) in entries {
            archive.start_file(*path, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();
    }
    output.into_inner()
}

fn tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (path, bytes) in entries { source::append_bytes(&mut archive, path, bytes, 0).unwrap(); }
    archive.into_inner().unwrap()
}

fn audit(root: &Path) -> anyhow::Result<serde_json::Value> { archive::check(root, None, None, None, false, false) }

#[test]
fn rejects_nested_weights_and_disguised_containers() {
    let root = tempfile::tempdir().unwrap();
    let payload = zip(&[("weights.onnx", b"synthetic, not model bytes")]);
    fs::write(root.path().join("notes.json"), zip(&[("inner.zip", &payload)])).unwrap();
    assert!(audit(root.path()).is_err());
}

#[test]
fn refuses_native_binaries_without_current_package_admission() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("verdant-worker"), b"\x7fELFsynthetic native header").unwrap();
    assert!(audit(root.path()).is_err());
    fs::remove_file(root.path().join("verdant-worker")).unwrap();
    fs::write(root.path().join("RELEASE-ASSETS.json"), b"{\"format\":\"verdant-native-package/1\"}").unwrap();
    assert!(audit(root.path()).is_err());
}

#[test]
fn empty_directory_cannot_establish_a_runtime_package() {
    let root = tempfile::tempdir().unwrap();
    assert!(archive::check(root.path(), None, None, None, true, false).is_err());
    fs::create_dir(root.path().join("models")).unwrap();
    assert!(audit(root.path()).is_err());
}

#[test]
fn rejects_unsafe_paths_and_portable_name_collisions() {
    for path in ["../outside.txt", "/absolute.txt", "C:/outside.txt", "safe.txt:stream", "..\\outside.txt", "weights.onnx. ", "NUL.txt"] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("source.zip"), zip(&[(path, b"synthetic")])).unwrap();
        assert!(audit(root.path()).is_err(), "{path}");
    }
    for entries in [vec![("A.txt", b"a".as_slice()), ("a.txt", b"b".as_slice())],
        vec![("item", b"a".as_slice()), ("item/child.txt", b"b".as_slice())]] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("source.zip"), zip(&entries)).unwrap();
        assert!(audit(root.path()).is_err());
    }
}

#[test]
fn detects_crc_errors_tar_trailers_and_depth_limits() {
    let content = b"unique synthetic source content";
    let mut payload = zip(&[("source.rs", content)]);
    let position = payload.windows(content.len()).position(|bytes| bytes == content).unwrap();
    payload[position] ^= 1;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source.zip");
    fs::write(&path, payload).unwrap();
    assert!(audit(root.path()).is_err());
    fs::remove_file(&path).unwrap();
    let mut payload = tar(&[("source.rs", b"fn main() {}")]);
    payload.extend_from_slice(b"nonzero trailer");
    fs::write(root.path().join("source.tar"), payload).unwrap();
    assert!(audit(root.path()).is_err());
    fs::remove_file(root.path().join("source.tar")).unwrap();
    let mut payload = zip(&[("source.rs", b"fn main() {}")]);
    for _ in 0..9 { payload = zip(&[("nested.zip", &payload)]); }
    fs::write(path, payload).unwrap();
    assert!(audit(root.path()).is_err());
}

#[test]
fn source_offer_requires_exact_membership_bytes_modes_and_external_selection() {
    let root = tempfile::tempdir().unwrap();
    let content = b"synthetic public source\n";
    let selection = source::Selection { schema: source::SCHEMA.into(), source_commit: "a".repeat(40), exclusions: BTreeMap::new(),
        files: source::REQUIRED.iter().map(|path| source::SelectedFile { path: (*path).into(), bytes: content.len() as u64,
            sha256: hex::encode(Sha256::digest(content)), mode: 0o644, git_blob: "b".repeat(40) }).collect() };
    let selection_bytes = files::json_bytes(&selection).unwrap();
    let selection_sha = hex::encode(Sha256::digest(&selection_bytes));
    let names = selection.files.iter().map(|file| format!("verdant-source/{}", file.path)).collect::<Vec<_>>();
    let mut entries = names.iter().map(|name| (name.as_str(), content.as_slice())).collect::<Vec<_>>();
    entries.push(("verdant-source/SOURCE-SELECTION.json", &selection_bytes));
    let path = root.path().join("source.tar");
    fs::write(&path, tar(&entries)).unwrap();
    assert!(archive::check(root.path(), None, Some(&selection.source_commit), Some(&selection_sha), false, false).is_ok());
    assert!(archive::check(root.path(), None, None, Some(&"c".repeat(64)), false, false).is_err());
    entries.push(("verdant-source/extra.rs", b"undeclared source"));
    fs::write(&path, tar(&entries)).unwrap();
    assert!(audit(root.path()).is_err());
}

const NATIVE_VALIDATION_FILES: &[&str] = &[
    "tools/a11y/README.md", "tools/a11y/observe-atspi.py", "tools/a11y/run-isolated.py",
    "tools/native-input/README.md", "tools/native-input/ci.py", "tools/native-input/probe.py",
    "tools/native-input/run-linux-isolated.py", "tools/native-input/x11_screenshot.py",
    "tools/native-input/test_ci.py", "tools/native-input/test_probe.py",
];

fn git(root: &Path, arguments: &[&str]) {
    let output = Command::new("git").arg("-C").arg(root)
        .args(["-c", "user.name=Release fixture", "-c", "user.email=release-fixture@example.invalid",
            "-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(arguments).output().unwrap();
    assert!(output.status.success(), "git {arguments:?}: {}", String::from_utf8_lossy(&output.stderr));
}

fn source_repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--quiet"]);
    for path in source::REQUIRED.iter().chain(NATIVE_VALIDATION_FILES.iter()) {
        let destination = root.path().join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        files::write_new(&destination, format!("synthetic public source: {path}\n").as_bytes(), false).unwrap();
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "--quiet", "-m", "Synthetic public verification sources"]);
    root
}

#[test]
fn selected_native_validation_sources_survive_the_real_source_archive_guard() {
    let root = source_repository();
    let additional_docs = ["frontend/proto-ui/README.md", "frontend/proto-ui/VALIDATION-COMPLETION-2026-11-22.md",
        "frontend/proto-ui/NATIVE-INPUT-2026-11-22.json"];
    for path in additional_docs {
        files::write_new(&root.path().join(path), b"synthetic validation documentation\n", false).unwrap();
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "--quiet", "-m", "Synthetic validation documentation"]);
    let selection = source::select(root.path()).unwrap();
    for path in NATIVE_VALIDATION_FILES.iter().copied().chain(additional_docs) {
        let selected = selection.files.iter().find(|file| file.path == path).unwrap();
        let content = fs::read(root.path().join(path)).unwrap();
        assert_eq!(selected.sha256, hex::encode(Sha256::digest(&content)), "{path}");
        assert_eq!(selected.bytes, content.len() as u64, "{path}");
    }
    let proof = tempfile::tempdir().unwrap();
    let selection_path = proof.path().join("selection.json");
    files::write_new_json(&selection_path, &selection).unwrap();
    let sha256 = files::digest(&selection_path).unwrap();
    assert_eq!(source::load(root.path(), &selection_path, &sha256).unwrap(), selection);
    let output = tempfile::tempdir().unwrap();
    source::create(root.path(), &selection_path, &sha256, &selection, &output.path().join("source.tar.gz"), 0).unwrap();
    let report = archive::check(output.path(), None, Some(&selection.source_commit), Some(&sha256), false, false).unwrap();
    assert_eq!(report["verified_source_offers"], 1);
}

#[test]
fn unreviewed_python_paths_still_fail_source_selection_and_embedded_proofs() {
    for path in ["tools/a11y/unreviewed.py", "tools/a11y/nested/observe-atspi.py",
        "tools/a11y-copy/observe-atspi.py", "tools/native-input/unreviewed.py",
        "tools/native-input/nested/probe.py", "tools/native-input-copy/probe.py",
        "tools/unreviewed.py", "backend/worker.py"] {
        let root = source_repository();
        let mut selection = source::select(root.path()).unwrap();
        let content = b"synthetic unreviewed Python\n";
        selection.files.push(source::SelectedFile { path: path.into(), bytes: content.len() as u64,
            sha256: hex::encode(Sha256::digest(content)), mode: 0o644, git_blob: "b".repeat(40) });
        let error = source::validate(&selection).unwrap_err().to_string();
        assert!(error.contains("unapproved corresponding-source input") && error.contains(path), "{error}");
        files::write_new(&root.path().join(path), content, false).unwrap();
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "--quiet", "-m", "Synthetic unreviewed Python"]);
        let error = source::select(root.path()).unwrap_err().to_string();
        assert!(error.contains("unclassified public source input") && error.contains(path), "{error}");
    }
}

#[test]
fn validation_sources_and_new_documents_must_match_committed_selection() {
    let root = source_repository();
    let selection = source::select(root.path()).unwrap();
    let proof = tempfile::tempdir().unwrap();
    let selection_path = proof.path().join("selection.json");
    files::write_new_json(&selection_path, &selection).unwrap();
    let sha256 = files::digest(&selection_path).unwrap();
    let script = root.path().join("tools/native-input/probe.py");
    let original = fs::read(&script).unwrap();
    fs::write(&script, b"changed after source selection\n").unwrap();
    let error = source::load(root.path(), &selection_path, &sha256).unwrap_err().to_string();
    assert!(error.contains("selected source differs from its committed blob") && error.contains("probe.py"), "{error}");
    fs::write(script, original).unwrap();
    let doc = "frontend/proto-ui/VALIDATION-COMPLETION-2026-11-22.md";
    files::write_new(&root.path().join(doc), b"synthetic validation documentation\n", false).unwrap();
    let error = source::select(root.path()).unwrap_err().to_string();
    assert!(error.contains("untracked public source must be committed") && error.contains(doc), "{error}");
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "--quiet", "-m", "Synthetic new corresponding-source document"]);
    assert!(source::select(root.path()).unwrap().files.iter().any(|file| file.path == doc));
    let error = source::load(root.path(), &selection_path, &sha256).unwrap_err().to_string();
    assert!(error.contains("selected-source membership, modes, ancestry or bytes changed"), "{error}");
}

fn bundle_policy() -> policy::Policy {
    let mut files = policy::REQUIRED_ROLES.iter().map(|(role, path)| policy::PolicyFile {
        role: (*role).into(), path: (*path).into(), bytes: 1, sha256: "a".repeat(64) }).collect::<Vec<_>>();
    files.extend([("license:voice", "licenses/voice.txt"), ("notice:voice", "notices/voice.txt"), ("authorization:voice", "licenses/grant.md")]
        .map(|(role, path)| policy::PolicyFile { role: role.into(), path: path.into(), bytes: 1, sha256: "b".repeat(64) }));
    policy::Policy { schema: "verdant-release-artifact-policy/1".into(), name: "verdant-voice".into(), version: "0.1.0".into(),
        compatible_app_version: "0.1.0".into(), authorization_status: "granted".into(), files,
        grants: vec![policy::Grant { id: "synthetic-grant".into(), rights_holder: Some("Synthetic rights holder".into()), release_authority: None,
            permission: "redistribute-bundled-runtime".into(), artifact_specific: true,
            evidence: policy::Evidence { path: "licenses/grant.md".into(), bytes: 1, sha256: "b".repeat(64) },
            artifacts: policy::REQUIRED_ROLES.iter().map(|(role, _)| policy::GrantedArtifact { role: (*role).into(), sha256: "a".repeat(64) }).collect() }] }
}

#[test]
fn bundle_policy_requires_artifact_specific_complete_grants() {
    assert!(bundle_policy().validate().is_ok());
    let mut policy = bundle_policy();
    policy.grants[0].release_authority = Some("Release operator".into());
    assert!(policy.validate().is_err());
    policy.grants[0].rights_holder = None;
    assert!(policy.validate().is_ok());
    policy.grants[0].release_authority = None;
    assert!(policy.validate().is_err());
    let mut policy = bundle_policy(); policy.grants[0].artifact_specific = false;
    assert!(policy.validate().is_err());
    let mut policy = bundle_policy(); policy.grants[0].artifacts.pop();
    assert!(policy.validate().is_err());
    let mut policy = bundle_policy(); policy.grants[0].artifacts[0].sha256 = "f".repeat(64);
    assert!(policy.validate().is_err());
    let mut policy = bundle_policy(); policy.grants[0].permission = "source-use".into();
    assert!(policy.validate().is_err());
    let mut policy = bundle_policy(); policy.compatible_app_version = "0.2.0".into();
    assert!(policy.validate().is_err());
}

#[test]
fn bundle_policy_binds_canonical_paths_and_manifest_content() {
    use serde_json::{Map, json};
    let mut policy = bundle_policy();
    for (role, path) in [("notice:voice", "NOTICE.txt"), ("authorization:voice", "AUTHORIZATION.json")] {
        policy.files.iter_mut().find(|file| file.role == role).unwrap().path = path.into();
    }
    policy.grants[0].evidence.path = "AUTHORIZATION.json".into();
    policy.validate().unwrap();
    let artifacts: Map<_, _> = policy::REQUIRED_ROLES[1..].iter().map(|(role, _)| {
        let file = policy.files.iter().find(|file| file.role == *role).unwrap();
        (file.role.clone(), json!({"file":file.path,"bytes":file.bytes,"sha256":file.sha256}))
    }).collect();
    let mut manifest = json!({"schema":verdant_voice_format::SCHEMA,"name":"verdant-voice","version":"0.1.0",
        "strategy":verdant_voice_format::STRATEGY,"artifacts":artifacts,
        "records":{"notice":{"file":"NOTICE.txt","bytes":1,"sha256":"b".repeat(64)},
            "authorization":{"file":"AUTHORIZATION.json","bytes":1,"sha256":"b".repeat(64)}}});
    let bytes = serde_json::to_vec(&manifest).unwrap();
    policy.runtime_manifest(&bytes).unwrap();
    manifest["artifacts"]["bank_tensors"]["sha256"] = "e".repeat(64).into();
    assert!(policy.runtime_manifest(&serde_json::to_vec(&manifest).unwrap()).is_err());
    manifest["artifacts"]["bank_tensors"]["sha256"] = "a".repeat(64).into();
    manifest["artifacts"]["verdant_voice_decoder"]["sha256"] = "e".repeat(64).into();
    assert!(policy.runtime_manifest(&serde_json::to_vec(&manifest).unwrap()).is_err());
    manifest["artifacts"]["verdant_voice_decoder"]["sha256"] = "a".repeat(64).into();
    manifest["records"]["notice"]["sha256"] = "e".repeat(64).into();
    assert!(policy.runtime_manifest(&serde_json::to_vec(&manifest).unwrap()).is_err());
    assert!(policy.runtime_manifest(br#"{"schema":"verdant-voice-runtime/1"}"#).is_err());
    for role in ["license:voice", "notice:voice", "authorization:voice", "bank_tensors", "verdant_voice_decoder"] {
        let mut incomplete = policy.clone();
        incomplete.files.retain(|file| file.role != role);
        assert!(incomplete.validate().is_err(), "{role}");
    }
    let root = policy.files.iter_mut().find(|file| file.role == "verdant_voice_manifest").unwrap();
    root.path = "verdant-voice/manifest.json".into();
    assert!(policy.validate().is_err());
}

#[test]
fn admission_json_rejects_duplicate_keys_at_every_depth() {
    assert!(files::parse_json::<serde_json::Value>(br#"{"schema":"a","schema":"b"}"#).is_err());
    assert!(files::parse_json::<serde_json::Value>(br#"{"grant":{"sha256":"a","sha256":"b"}}"#).is_err());
}
