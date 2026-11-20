use std::fs;
use std::process::Command;
use std::time::{SystemTime,UNIX_EPOCH};
struct Scratch(std::path::PathBuf);
impl Scratch{fn new()->Self{let path=std::env::temp_dir().join(format!("verdant-exchange-cli-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&path).unwrap();Self(path)}}
impl Drop for Scratch{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
#[test]
fn cli_converts_real_smf_and_protects_existing_input_and_report_destinations(){
    let scratch=Scratch::new();let source=scratch.0.join("score.mid");let native=scratch.0.join("score.verp");let report=scratch.0.join("read.json");let out=scratch.0.join("score.svp");let export_report=scratch.0.join("write.json");
    let events=[0,0x90,60,90,0x83,0x60,0x80,60,0,0,0xff,0x2f,0];let mut bytes=b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk".to_vec();bytes.extend_from_slice(&(events.len()as u32).to_be_bytes());bytes.extend_from_slice(&events);fs::write(&source,&bytes).unwrap();
    let binary=env!("CARGO_BIN_EXE_verdant-exchange");let read=Command::new(binary).arg("import").arg(&source).arg("--output").arg(&native).arg("--report").arg(&report).output().unwrap();assert!(read.status.success(),"{}",String::from_utf8_lossy(&read.stderr));let p:verdant_domain::Project=serde_json::from_slice(&fs::read(&native).unwrap()).unwrap();assert_eq!(p.ppq,480);assert_eq!(p.tracks[0].notes[0].duration_tick,480);let r:verdant_exchange::ExchangeReport=serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();assert_eq!(r.source_ppq,480);
    let write=Command::new(binary).arg("export").arg(&native).arg("--output").arg(&out).arg("--report").arg(&export_report).output().unwrap();assert!(write.status.success(),"{}",String::from_utf8_lossy(&write.stderr));let exported=fs::read(&out).unwrap();let again=verdant_exchange::import(verdant_exchange::Format::Svp,&exported,&Default::default()).unwrap();assert_eq!(again.project.tracks[0].notes[0].duration_tick,705600000);
    let protected=Command::new(binary).arg("export").arg(&native).arg("--format").arg("midi").arg("--output").arg(&source).arg("--report").arg(scratch.0.join("unused.json")).output().unwrap();assert!(!protected.status.success());assert_eq!(fs::read(&source).unwrap(),bytes);assert!(!scratch.0.join("unused.json").exists());
    let missing=scratch.0.join("requires-voice.vsqx");let missing_report=scratch.0.join("requires-voice.json");let failure=Command::new(binary).arg("export").arg(&native).arg("--output").arg(&missing).arg("--report").arg(&missing_report).output().unwrap();assert!(!failure.status.success());assert!(String::from_utf8_lossy(&failure.stderr).contains("actual vocaloid_voice"));assert!(!missing.exists());assert!(!missing_report.exists());
}

#[test]
fn cli_ufdata_roundtrip_uses_verp() {
    let scratch = Scratch::new();
    let mut project = verdant_domain::Project::new_empty();
    project.ppq = 480;
    project.tracks[0].notes.push(verdant_domain::Note::new(240, 480, 67));
    project.tracks[0].notes[0].lyric = "la".into();
    let encoded = verdant_exchange::export(verdant_exchange::Format::Ufdata, &project, &Default::default()).unwrap();
    let source = scratch.0.join("source.ufdata");
    fs::write(&source, &encoded.bytes).unwrap();
    let binary = env!("CARGO_BIN_EXE_verdant-exchange");
    let requested = scratch.0.join("native");
    let imported = Command::new(binary).arg("import").arg(&source).arg("--format").arg("ufdata")
        .arg("--output").arg(&requested).output().unwrap();
    assert!(imported.status.success(), "{}", String::from_utf8_lossy(&imported.stderr));
    let native = requested.with_extension("verp");
    assert!(native.is_file());
    assert!(!requested.exists());
    let saved = verdant_domain::ProjectPersistence::read_saved(&native).unwrap();
    assert_eq!(saved.tracks[0].notes[0].midi_note, 67);
    assert_eq!(saved.tracks[0].notes[0].lyric, "la");
    let output = scratch.0.join("export.ufdata");
    let result = Command::new(binary).arg("export").arg(&native).arg("--output").arg(&output).output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let roundtrip = verdant_exchange::import(verdant_exchange::Format::Ufdata, &fs::read(output).unwrap(), &Default::default()).unwrap();
    let note = &roundtrip.project.tracks[0].notes[0];
    assert_eq!((note.start_tick, note.duration_tick, note.midi_note), (240, 480, 67));
    assert_eq!(fs::read(source).unwrap(), encoded.bytes);
}

#[test]
fn cli_import_and_export_reject_retired_project_paths_and_names() {
    let scratch = Scratch::new();
    let binary = env!("CARGO_BIN_EXE_verdant-exchange");
    let report = scratch.0.join("not-created.exchange.json");
    for (operation, source, format, destination) in [
        ("import", "old.gdmproj", "ufdata", "out.verp"),
        ("import", "old.verdant.JSON", "midi", "out.verp"),
        ("import", "score.mid", "gdmproj", "out.verp"),
        ("import", "score.mid", "midi", "out.json"),
        ("export", "old.GDMProj", "ufdata", "out.ufdata"),
        ("export", "old.json", "ufdata", "out.ufdata"),
        ("export", "song.verp", "ufdata", "out.gdmproj"),
    ] {
        let output = scratch.0.join(destination);
        let result = Command::new(binary).arg(operation).arg(scratch.0.join(source))
            .arg("--format").arg(format).arg("--output").arg(&output).arg("--report").arg(&report)
            .output().unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains(verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE));
        assert!(!output.exists() && !report.exists());
    }
}

#[test]
fn cli_rejects_complete_foreign_native_shape_and_unsupported_major_version() {
    let scratch = Scratch::new();
    for (field, invalid) in [("format", "other-project"), ("version", "1.0")] {
        let mut project = serde_json::to_value(verdant_domain::Project::new_empty()).unwrap();
        project[field] = invalid.into();
        let bytes = serde_json::to_vec(&project).unwrap();
        let source = scratch.0.join(format!("invalid-{field}.verp"));
        let output = scratch.0.join(format!("invalid-{field}.ufdata"));
        let report = scratch.0.join(format!("invalid-{field}.exchange.json"));
        fs::write(&source, &bytes).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_verdant-exchange")).arg("export").arg(&source)
            .arg("--output").arg(&output).arg("--report").arg(&report).output().unwrap();
        assert!(!result.status.success(), "accepted wrong {field}");
        assert!(String::from_utf8_lossy(&result.stderr).contains(&format!("/{field}")));
        assert!(!output.exists() && !report.exists());
        assert_eq!(fs::read(source).unwrap(), bytes);
    }
}
