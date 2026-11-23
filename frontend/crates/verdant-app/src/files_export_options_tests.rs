//! Consumer-path regressions for the native score export options window.
use super::*;
use serde_json::json;
use verdant_domain::{Note, Project, Track};
use verdant_exchange::{Disposition, ExportOptions, Format, UstEncoding};

struct ExportDirectory(PathBuf);
impl ExportDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-export-options-{}", verdant_domain::NoteId::generate()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for ExportDirectory {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

fn score() -> Project {
    let mut project = Project::new_empty();
    project.voice_refs.clear();
    project.tracks[0].voice_ref = None;
    project.tracks[0].name = "first".into();
    let mut first = Note::new(0, project.ppq, 60);
    first.lyric = "a".into();
    first.portamento_ms = Some(0);
    project.tracks[0].notes = vec![first];
    project.tracks[0].refresh_index();
    let mut track = Track::new("selected", "teal");
    track.voice_ref = None;
    let mut note = Note::new(0, project.ppq, 67);
    note.lyric = "か".into();
    note.portamento_ms = Some(0);
    track.notes = vec![note];
    track.refresh_index();
    project.tracks.push(track);
    project.extensions.insert("score_import".into(), json!({
        "source": { "format": "ust" },
        "fields": [{
            "location": "/#SETTING/Flags", "disposition": "retained_unsupported", "source_value": "g-5"
        }]
    }));
    project.extensions.insert("exchange_svp".into(), json!({"vendor": "original unsupported value"}));
    project
}

#[test]
fn export_dialog_choices_reach_cp932_track_and_extension_writer() {
    let directory = ExportDirectory::new();
    let project = score();
    let chosen = resolve_score_options(Format::Ust, ExportOptions {
        track: Some(1), ust_encoding: UstEncoding::Cp932, preserve_extensions: false,
        ..Default::default()
    }, "").unwrap();
    let path = directory.0.join("selected.ust");
    let report = export_score_file(Format::Ust, &project, &chosen, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.windows(b"Lyric=\x82\xa9\r\n".len()).any(|w| w == b"Lyric=\x82\xa9\r\n"));
    assert!(bytes.windows(b"NoteNum=67\r\n".len()).any(|w| w == b"NoteNum=67\r\n"));
    assert!(!bytes.windows(b"NoteNum=60\r\n".len()).any(|w| w == b"NoteNum=60\r\n"));
    assert!(!bytes.windows(b"Flags=".len()).any(|w| w == b"Flags="));
    assert!(report.issues.iter().any(|i| i.path == "/#SETTING/Flags" && i.disposition == Disposition::Unsupported));

    let retained = resolve_score_options(Format::Ust, ExportOptions {
        preserve_extensions: true, ..chosen.clone()
    }, "").unwrap();
    let retained_report = export_score_file(Format::Ust, &project, &retained, &path).unwrap();
    let retained_bytes = std::fs::read(&path).unwrap();
    assert!(retained_bytes.windows(b"Flags=g-5\r\n".len()).any(|w| w == b"Flags=g-5\r\n"));
    assert!(retained_report.issues.iter().any(|i| i.path == "/#SETTING/Flags" && i.disposition == Disposition::RetainedUnsupported));

    let utf8 = resolve_score_options(Format::Ust, ExportOptions {
        ust_encoding: UstEncoding::Utf8, ..chosen
    }, "").unwrap();
    export_score_file(Format::Ust, &project, &utf8, &path).unwrap();
    assert!(std::fs::read(&path).unwrap().windows("Lyric=か\r\n".len()).any(|w| w == "Lyric=か\r\n".as_bytes()));
}

#[test]
fn export_dialog_midi_ppq_reaches_smf_header_and_full_report_survives_preferences() {
    let directory = ExportDirectory::new();
    let project = score();
    let chosen = resolve_score_options(Format::Midi, ExportOptions {
        track: Some(1), preserve_extensions: false, ..Default::default()
    }, "1920").unwrap();
    let path = directory.0.join("selected.mid");
    let report = export_score_file(Format::Midi, &project, &chosen, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], b"MThd");
    assert_eq!(u16::from_be_bytes([bytes[12], bytes[13]]), 1920);
    assert_eq!(u16::from_be_bytes([bytes[10], bytes[11]]), 1);
    assert!(report.has_losses());

    // This is the exact string handed to the report prompt and report viewer,
    // and the exact preferences surface serialized by save_prefs.
    let mut prefs = Prefs::default();
    let displayed = remember_export_report(&mut prefs, &report);
    let displayed_value: serde_json::Value = serde_json::from_str(&displayed).unwrap();
    let saved = serde_json::to_vec(&prefs).unwrap();
    let reopened: Prefs = serde_json::from_slice(&saved).unwrap();
    let persisted = &reopened.extra["last_exchange_report"];
    assert_eq!(&displayed_value, persisted);
    assert_eq!(persisted, &serde_json::to_value(&report).unwrap());
    let loss = persisted["issues"].as_array().unwrap().iter()
        .find(|i| i["path"] == "/extensions/exchange_svp").unwrap();
    assert_eq!(loss["disposition"], "unsupported");
    assert_eq!(loss["value"], json!({"vendor": "original unsupported value"}));
    assert!(!loss["detail"].as_str().unwrap().is_empty());
}

#[test]
fn export_dialog_rejects_invalid_midi_division_without_clamping() {
    for invalid in ["0", "32768", "-1", "480.5", "abc", "65536"] {
        assert!(resolve_score_options(Format::Midi, ExportOptions::default(), invalid).is_err(), "{invalid}");
    }
    assert_eq!(resolve_score_options(Format::Midi, ExportOptions::default(), "").unwrap().midi_ppq, None);
    assert_eq!(resolve_score_options(Format::Midi, ExportOptions::default(), "32767").unwrap().midi_ppq, Some(32767));
}

#[test]
fn export_dialog_cannot_discard_its_only_real_vsqx_voice_table() {
    let without_table = ExportOptions { preserve_extensions: false, ..Default::default() };
    assert!(resolve_score_options(Format::Vsqx, without_table, "").unwrap_err().contains("真实歌手元数据"));
    // The outer VSQX gate already requires an imported table for this path.
    assert!(resolve_score_options(Format::Vsqx, ExportOptions::default(), "").is_ok());
    let explicit = ExportOptions {
        preserve_extensions: false,
        vocaloid_voice: Some(verdant_exchange::VocaloidVoice {
            // Public fixture identity, never a production default.
            comp_id: "BNGW7FG7E5TRSNC3".into(), name: "KAITO_V3_English".into(), language_id: 1, program: 0,
        }),
        ..Default::default()
    };
    assert!(resolve_score_options(Format::Vsqx, explicit, "").is_ok());
}

#[gpui::test]
fn native_export_options_numeric_source_and_default_response_reach_exporter(cx: &mut gpui::TestAppContext) {
    let (send, receive) = async_channel::bounded(1);
    let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
        ScoreExportDialog::new(Format::Midi, ExportOptions::default(), vec!["Track".into()], send, window, cx)
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap(); crate::native_test_support::frames(window, cx);
    cx.simulate_input(window.into(), "1920"); crate::native_test_support::frames(window, cx);
    window.update(cx, |dialog, _, cx| assert_eq!(dialog.ppq.read(cx).value(), "1920")).unwrap();
    cx.simulate_keystrokes(window.into(), "enter"); cx.run_until_parked();
    assert_eq!(receive.try_recv().expect("source default response").midi_ppq, Some(1920));
}

#[gpui::test]
fn native_export_radio_enter_uses_default_response_without_changing_selection(cx: &mut gpui::TestAppContext) {
    use verdant_action_control::native::gpui_native_owner::{all_owners, resolved};
    let (send, receive) = async_channel::bounded(1);
    let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
        ScoreExportDialog::new(Format::Ust, ExportOptions::default(), vec!["Track".into()], send, window, cx)
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap(); crate::native_test_support::frames(window, cx);
    window.update(cx, |_, window, cx| {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "CP932（日文 Windows）")
        }).expect("real source radio");
        verdant_action_control::ActionControlHandle { owner }.call("focusSelf", vec![], window, cx).unwrap();
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "enter"); cx.run_until_parked();
    assert!(matches!(receive.try_recv().expect("source Enter default response").ust_encoding, UstEncoding::Utf8));
}
