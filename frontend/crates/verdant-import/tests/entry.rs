use std::process::Command;
use verdant_import::ImportFormat;

#[test]
fn importer_entry_resolves_case_insensitive_ust_extensions() {
    assert_eq!(ImportFormat::from_extension(".UST"), Some(ImportFormat::Ust));
    assert_eq!(ImportFormat::from_extension("UsTx"), Some(ImportFormat::Ustx));
    assert_eq!(ImportFormat::from_extension("mid"), None);
    assert_eq!(ImportFormat::from_extension("wav"), None);
}

#[test]
fn cli_rejects_unsupported_extension_before_reading_or_creating_files() {
    let root = std::env::temp_dir().join(format!("verdant-import-unsupported-{}-{}", std::process::id(), verdant_domain::NoteId::generate()));
    // Neither path exists: a read attempt would report I/O, while extension
    // admission must explain which entry point can consume the source.
    let source = root.join("score.wav");
    let output = root.join("project.verp");
    let result = Command::new(env!("CARGO_BIN_EXE_verdant-import"))
        .arg(&source).arg("--output").arg(&output).output().unwrap();
    assert!(!result.status.success());
    let message = String::from_utf8_lossy(&result.stderr);
    assert!(message.contains("unsupported source extension"), "{message}");
    assert!(message.contains("score.wav"), "{message}");
    assert!(message.contains(".ust or .ustx"), "{message}");
    assert!(message.contains("verdant-exchange import"), "{message}");
    assert!(!root.exists());
}

#[test]
fn cli_rejects_retired_projects_before_opening_inputs_or_outputs() {
    let root = std::env::temp_dir().join(format!("verdant-import-retired-{}", verdant_domain::NoteId::generate()));
    let output = root.join("project.verp");
    let binary = env!("CARGO_BIN_EXE_verdant-import");
    for (source, format, destination) in [
        ("old.gdmproj", "ust", "project.verp"),
        ("old.verdant.JSON", "ustx", "project.verp"),
        ("song.ust", "GDMProj", "project.verp"),
        ("song.ust", "ust", "project.json"),
    ] {
        let result = Command::new(binary).arg(root.join(source)).arg("--format").arg(format)
            .arg("--output").arg(output.with_file_name(destination)).output().unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains(verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE));
        assert!(!root.exists());
    }
}
