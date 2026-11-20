//! Synthetic bytes only. No voice, runtime library, bank or recording fixtures.
use std::{collections::BTreeMap, fs, io::{Cursor, Write}, path::Path};
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
