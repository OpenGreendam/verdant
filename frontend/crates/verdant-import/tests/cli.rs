use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct OwnedDirectory(PathBuf);
impl Drop for OwnedDirectory { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

#[test]
fn cli_writes_loadable_private_project_report_catalog_and_refuses_overwrites() {
    let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("verdant-import-cli-{}-{unique}", std::process::id()));
    fs::create_dir(&path).unwrap();
    let root = OwnedDirectory(path);
    let source = root.0.join("source.ust");
    let output = root.0.join("auditions/song.verp");
    let catalog = root.0.join("auditions/manifest.json");
    let bytes = b"[#SETTING]\nTempo=183\n[#0000]\nLength=480\nLyric=ma\nNoteNum=60\n[#TRACKEND]\n";
    fs::write(&source, bytes).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_verdant-import")).arg(&source)
        .arg("--output").arg(&output).arg("--catalog").arg(&catalog).arg("--pinyin").output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let project_bytes = fs::read(&output).unwrap();
    let report_bytes = fs::read(output.with_extension("import.json")).unwrap();
    let (project, _) = verdant_domain::io::parse_project(std::str::from_utf8(&project_bytes).unwrap()).unwrap();
    assert_eq!(project.tempo_map[0].micro_bpm, 183_000_000);
    assert_eq!(project.tracks[0].notes[0].pronunciation.pinyin, "ma");
    let report: verdant_import::ImportReport = serde_json::from_slice(&report_bytes).unwrap();
    assert_eq!(report.source.sha256.len(), 64);
    let manifest: verdant_import::AuditionCatalog = serde_json::from_slice(&fs::read(&catalog).unwrap()).unwrap();
    assert_eq!(manifest.schema, "verdant-audition-catalog/1");
    assert_eq!(manifest.projects[0].path, "song.verp");
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&output).unwrap().permissions().mode() & 0o777, 0o600);
    }
    let overwrite = Command::new(env!("CARGO_BIN_EXE_verdant-import")).arg(&source).arg("--output").arg(&output).output().unwrap();
    assert!(!overwrite.status.success());
    assert_eq!(fs::read(&output).unwrap(), project_bytes);
    assert_eq!(fs::read(output.with_extension("import.json")).unwrap(), report_bytes);
    let input_alias = Command::new(env!("CARGO_BIN_EXE_verdant-import")).arg(&source).arg("--output").arg(&source).output().unwrap();
    assert!(!input_alias.status.success());
    assert_eq!(fs::read(&source).unwrap(), bytes);
}

#[test]
fn cli_extensionless_output_is_verp_and_catalog_opens_the_actual_file() {
    let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("verdant-import-verp-{}-{unique}", std::process::id()));
    fs::create_dir(&path).unwrap();
    let root = OwnedDirectory(path);
    let source = root.0.join("source.ust");
    let bytes = b"[#SETTING]\nTempo=120\n[#0000]\nLength=480\nLyric=la\nNoteNum=64\n[#TRACKEND]\n";
    fs::write(&source, bytes).unwrap();
    let requested = root.0.join("auditions/song");
    let catalog = root.0.join("auditions/manifest.json");
    let result = Command::new(env!("CARGO_BIN_EXE_verdant-import")).arg(&source)
        .arg("--output").arg(&requested).arg("--catalog").arg(&catalog).output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let manifest: verdant_import::AuditionCatalog = serde_json::from_slice(&fs::read(&catalog).unwrap()).unwrap();
    assert_eq!(manifest.projects[0].path, "song.verp");
    let native = catalog.parent().unwrap().join(&manifest.projects[0].path);
    assert_eq!(native, requested.with_extension("verp"));
    assert!(!requested.exists());
    let (store, recovery) = verdant_domain::ProjectPersistence::open(&native).unwrap();
    assert!(!recovery.recovered_anything());
    assert_eq!(store.project().format, "greendam-project");
    assert_eq!(store.project().tracks[0].notes[0].midi_note, 64);
    assert_eq!(store.project().tracks[0].notes[0].lyric, "la");
    assert!(native.with_extension("import.json").is_file());
    assert_eq!(fs::read(source).unwrap(), bytes);
}
