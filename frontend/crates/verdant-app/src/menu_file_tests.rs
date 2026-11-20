use super::*;
use crate::editor::WorkflowAction;
use verdant_domain::{Note, Project};

fn score() -> Project {
    let mut project = Project::new_empty();
    project.voice_refs.clear();
    project.tracks[0].voice_ref = None;
    let mut note = Note::new(240, 480, 64);
    note.lyric = "a".into();
    note.portamento_ms = Some(0);
    project.tracks[0].notes = vec![note];
    project.tracks[0].refresh_index();
    project
}

#[test]
fn file_menu_routes_all_formats_through_real_score_codecs() {
    let items = file_menu_items(&crate::files::Prefs::default());
    let project = score();
    let options = verdant_exchange::ExportOptions {
        // Public fixture identity from verdant-exchange/tests/formats.rs;
        // this is test input, never an application default or license claim.
        vocaloid_voice: Some(verdant_exchange::VocaloidVoice {
            comp_id: "BNGW7FG7E5TRSNC3".into(), name: "KAITO_V3_English".into(), language_id: 1, program: 0,
        }),
        ..Default::default()
    };
    let mut groups = items.iter().filter_map(|item| match &item.command {
        Some(MenuCommand::Submenu(children)) => Some((item.label.to_string(), children)),
        _ => None,
    });
    let (import_label, imports) = groups.next().unwrap();
    let (export_label, exports) = groups.next().unwrap();
    assert!(groups.next().is_none());
    assert_eq!([import_label.as_str(), export_label.as_str()], ["导入", "导出"]);
    assert_eq!((imports.len(), exports.len()), (7, 7));
    assert!(!items.iter().any(|item| matches!(&item.command,
        Some(MenuCommand::File(FileAction::Import(_) | FileAction::Export(_))))));
    assert_eq!(SCORE_FORMATS.map(|(_, name)| name), ["UST", "USTX", "MIDI", "UFDATA", "SVP", "VSQX", "VPR"]);
    for ((format, name), (import, export)) in SCORE_FORMATS.into_iter().zip(imports.iter().zip(exports)) {
        assert_eq!(import.label.to_string(), format!("从 {name} 导入…"));
        assert_eq!(export.label.to_string(), format!("导出成 {name}…"));
        assert!(import.enabled && export.enabled);
        let Some(MenuCommand::File(FileAction::Export(export_format))) = &export.command else { panic!("export command"); };
        let exported = verdant_exchange::export(*export_format, &project, &options).unwrap();
        assert_eq!(exported.report.format, format);
        let Some(MenuCommand::File(FileAction::Import(import_format))) = &import.command else { panic!("import command"); };
        let (imported, report) = import_score_bytes(*import_format, &exported.bytes, Some(format!("score.{}", format.extension()))).unwrap();
        let notes = imported.tracks.iter().flat_map(|track| &track.notes).collect::<Vec<_>>();
        assert_eq!(notes.len(), 1, "{name}");
        assert_eq!(notes[0].midi_note, 64, "{name}");
        assert_eq!(notes[0].start_tick * project.ppq, 240 * imported.ppq, "{name} onset");
        assert_eq!(notes[0].duration_tick * project.ppq, 480 * imported.ppq, "{name} duration");
        if matches!(format, Format::Ust | Format::Ustx) {
            assert_eq!(report["schema"], "verdant-score-import/1");
            assert_eq!(report["source"]["format"], serde_json::to_value(format).unwrap());
        } else {
            assert_eq!(report["schema"], "verdant-exchange/1");
            assert_eq!(report["format"], serde_json::to_value(format).unwrap());
        }
    }
    assert!(items.iter().any(|i| matches!(&i.command, Some(MenuCommand::Editor(MenuAction::Workflow(WorkflowAction::New))))));
    assert!(items.iter().any(|i| matches!(&i.command, Some(MenuCommand::File(FileAction::Open)))));
    assert!(items.iter().any(|i| matches!(&i.command, Some(MenuCommand::Editor(MenuAction::Save)))));
    assert!(items.iter().any(|i| matches!(&i.command, Some(MenuCommand::Editor(MenuAction::SaveAs)))));
}

#[test]
fn recent_file_preferences_persist_order_dedupe_and_menu_destinations() {
    let mut prefs = crate::files::Prefs::default();
    prefs.extra.insert("other_setting".into(), serde_json::json!({"keep": true}));
    for i in 0..25 { remember_recent_file(&mut prefs, Path::new(&format!("/scores/{i}.verp"))); }
    remember_recent_file(&mut prefs, Path::new("/scores/20.verp"));
    remember_recent_file(&mut prefs, Path::new("/scores/20.verp"));
    remember_recent_file(&mut prefs, Path::new("/scores/imported.ustx"));
    let bytes = serde_json::to_vec_pretty(&prefs).unwrap();
    let restored: crate::files::Prefs = serde_json::from_slice(&bytes).unwrap();
    let recent = recent_project_paths(&restored);
    assert_eq!(recent.len(), 20);
    assert_eq!(recent[0], Path::new("/scores/20.verp"));
    assert_eq!(recent[1], Path::new("/scores/24.verp"));
    assert_eq!(recent.iter().filter(|p| p.as_path() == Path::new("/scores/20.verp")).count(), 1);
    assert_eq!(restored.extra["other_setting"]["keep"], true);
    let destinations = file_menu_items(&restored).into_iter().filter_map(|i| match i.command {
        Some(MenuCommand::File(FileAction::Recent(path))) => Some(path), _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(destinations, recent);
    // Old prefs can contain duplicates; they must not become duplicate menu rows.
    let legacy: crate::files::Prefs = serde_json::from_value(serde_json::json!({"recent_projects":["a.json","a.json","b.mid"]})).unwrap();
    assert!(recent_project_paths(&legacy).is_empty());
}

#[test]
fn file_extension_admission_rejects_unknown_and_preserves_aliases() {
    let error = file_format(Path::new("song.wav")).unwrap_err();
    assert!(error.contains("不支持的文件扩展名"));
    assert!(error.contains("song.wav"));
    assert!(error.contains("USTX") && error.contains("VPR"));
    assert!(file_format(Path::new("song")).is_err());
    assert_eq!(file_format(Path::new("song.MIDI")).unwrap(), Some(Format::Midi));
    assert_eq!(file_format(Path::new("song.MID")).unwrap(), Some(Format::Midi));
    assert_eq!(file_format(Path::new("song.VERP")).unwrap(), None);
    assert_eq!(file_format(Path::new("song.UFDATA")).unwrap(), Some(Format::Ufdata));
    for (format, _) in SCORE_FORMATS {
        assert_eq!(file_format(Path::new(&format!("song.{}", format.extension()))).unwrap(), Some(format));
    }
}

#[test]
fn file_menu_rejects_retired_project_extensions_explicitly() {
    for path in ["old.json", "old.verdant.JSON", "old.gdmproj", "old.GDMProj"] {
        assert_eq!(file_format(Path::new(path)).unwrap_err(), verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE);
    }
}

#[test]
fn file_import_entry_preserves_duplicate_key_and_ust_encoding_admission() {
    let duplicate_ustx = b"resolution: 480\nresolution: 960\n";
    assert!(import_score_bytes(Format::Ustx, duplicate_ustx, None).unwrap_err().contains("duplicate"));
    let duplicate_svp = b"{\"version\":100,\"version\":100}";
    assert!(import_score_bytes(Format::Svp, duplicate_svp, None).unwrap_err().contains("duplicate"));
    let ambiguous_ust = b"[#SETTING]\nTempo=120\n[#0000]\nLength=480\nLyric=\x82\xa0\nNoteNum=60\n[#TRACKEND]\n";
    let (_, report) = import_score_bytes(Format::Ust, ambiguous_ust, None).unwrap();
    assert_eq!(report["source"]["encoding"], "cp932");
}
