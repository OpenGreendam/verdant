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
fn vsqx_options_can_discard_source_before_singer_entry_but_writer_requires_a_voice() {
    let imported = verdant_exchange::import(Format::Vsqx,
        include_bytes!("../../verdant-exchange/tests/fixtures/score.vsqx"), &Default::default()).unwrap();
    let without_table = ExportOptions { preserve_extensions: false, ..Default::default() };
    let selected = resolve_score_options(Format::Vsqx, without_table, "").unwrap();
    assert!(!selected.preserve_extensions && selected.vocaloid_voice.is_none());
    let error = verdant_exchange::export(Format::Vsqx, &imported.project, &selected).err().unwrap();
    assert!(error.to_string().contains("actual vocaloid_voice metadata"));
    assert!(verdant_exchange::export(Format::Vsqx, &imported.project, &ExportOptions::default()).is_ok());
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

#[gpui::test]
fn native_export_dialog_uses_compiled_text_selection_radio_and_submit(cx: &mut gpui::TestAppContext) {
    use verdant_action_control::native::gpui_native_owner::{all_owners, dispatch_accessibility_action, drive_owners, resolved};
    let (send, receive) = async_channel::bounded(1);
    let window = cx.open_window(gpui::size(px(560.), px(620.)), move |window, cx| {
        ScoreExportDialog::new(Format::Midi, ExportOptions::default(), vec!["first".into(), "selected".into()], send, window, cx)
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |view, window, cx| {
        let focus = view.ppq.read(cx).focus.clone(); window.focus(&focus, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    cx.simulate_input(window.into(), "1920");
    crate::native_test_support::frames(window, cx);
    cx.simulate_keystrokes(window.into(), "up");
    crate::native_test_support::frames(window, cx);
    window.update(cx, |view, _, cx| {
        assert_eq!(view.ppq.read(cx).value(), "1921");
    }).unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        let radio = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|name| resolved(name).text() == "第 2 轨 · selected")
        }).expect("actual export track is a compiled radio control");
        dispatch_accessibility_action(&radio, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |view, window, cx| {
        assert_eq!(view.options.track, Some(1));
        let focus = view.ppq.read(cx).focus.clone(); window.focus(&focus, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    cx.simulate_keystrokes(window.into(), "enter");
    cx.run_until_parked();
    let options = receive.try_recv().expect("one compiled text submit confirms the native export window");
    assert_eq!(options.midi_ppq, Some(1921));
    assert_eq!(options.track, Some(1));
    assert!(receive.try_recv().is_err());
}

#[gpui::test]
fn native_export_text_submit_cannot_bypass_a_disabled_overlay_owner(cx: &mut gpui::TestAppContext) {
    use verdant_overlay_control::native::gpui_native_owner::{replace_props, drive_owners};
    let (send, receive) = async_channel::bounded(1);
    let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
        ScoreExportDialog::new(Format::Midi, ExportOptions::default(), vec!["Track".into()], send, window, cx)
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    cx.simulate_input(window.into(), "1920");
    crate::native_test_support::frames(window, cx);
    window.update(cx, |dialog, window, cx| {
        let (overlay, _) = dialog.overlay.as_ref().expect("mounted source response owner");
        let owner = overlay.upgrade().expect("live response owner");
        let mut props = owner.borrow().props.clone();
        props["disabled"] = json!(true);
        replace_props(&owner, props); drive_owners(window, cx);
        let focus = dialog.ppq.read(cx).focus.clone(); window.focus(&focus, cx);
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "enter escape");
    crate::native_test_support::frames(window, cx);
    assert!(receive.try_recv().is_err(), "text submit must consult OverlayControl before invoking the export domain action");
    window.update(cx, |dialog, _, cx| {
        assert_eq!(dialog.ppq.read(cx).value(), "1920");
        assert!(dialog.error.is_none(), "the rejected UI intent never enters export validation");
    }).expect("disabled source rejects cancellation as well as submission");
}

#[gpui::test]
fn native_export_invalid_text_keeps_form_open_and_source_cancel_sends_nothing(cx: &mut gpui::TestAppContext) {
    let (send, receive) = async_channel::bounded(1);
    let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
        ScoreExportDialog::new(Format::Midi, ExportOptions::default(), vec!["Track".into()], send, window, cx)
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    cx.simulate_input(window.into(), "32768");
    crate::native_test_support::frames(window, cx);
    cx.simulate_keystrokes(window.into(), "enter");
    crate::native_test_support::frames(window, cx);
    assert!(receive.try_recv().is_err());
    window.update(cx, |dialog, _, cx| {
        assert_eq!(dialog.ppq.read(cx).value(), "32768");
        assert!(dialog.error.is_some(), "the business validator rejects the unrepresentable MIDI division");
    }).expect("failed validation retains the form and draft");
    cx.simulate_keystrokes(window.into(), "escape"); cx.run_until_parked();
    assert!(receive.try_recv().is_err(), "source cancellation never exports the rejected draft");
    assert!(window.update(cx, |_, _, _| ()).is_err(), "source cancel closes the options window");
}

fn preserve_source(window: &Window) -> verdant_action_control::ActionControlHandle {
    use verdant_action_control::native::gpui_native_owner::{all_owners, resolved};
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|name| resolved(name).text().contains("保留来源扩展"))
    }).expect("production Preserve checkbox is a compiled action");
    verdant_action_control::ActionControlHandle { owner }
}

fn export_overlay(dialog: &ScoreExportDialog) -> verdant_overlay_control::OverlayControlHandle {
    let (owner, _) = dialog.overlay.as_ref().expect("mounted export overlay");
    verdant_overlay_control::OverlayControlHandle { owner: owner.upgrade().expect("live export overlay") }
}

#[gpui::test]
fn native_export_preserve_accepts_source_selection_once_and_rejects_stale_form_callbacks(cx: &mut gpui::TestAppContext) {
    use verdant_action_control::native::gpui_native_owner::{replace_props, dispatch_accessibility_action, drive_owners};
    let (send, receive) = async_channel::bounded(1);
    let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
        ScoreExportDialog::new(Format::Midi, ExportOptions { preserve_extensions: false, ..Default::default() }, vec!["Track".into()], send, window, cx)
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |_, window, cx| {
        let source = preserve_source(window);
        // Deliberately diverge only the source acceptance. Its real assistive
        // activation proposes false while the form already holds false.
        let mut props = source.owner.borrow().props.clone(); props["selected"] = json!(true);
        replace_props(&source.owner, props); drive_owners(window, cx);
        for _ in 0..2 { dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx); }
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |dialog, window, cx| {
        assert!(!dialog.options.preserve_extensions, "source false must not become a host inversion to true");
        assert_eq!(dialog.options_generation, 0, "same-value proposals leave accepted form authority unchanged");
        let source = preserve_source(window);
        let mut props = source.owner.borrow().props.clone(); props["selected"] = json!(false);
        replace_props(&source.owner, props); drive_owners(window, cx);
        for _ in 0..2 { dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx); }
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |dialog, window, cx| {
        assert!(dialog.options.preserve_extensions, "the source true proposal reaches the actual export options");
        assert_eq!(dialog.options_generation, 1, "duplicate captured proposals cannot create a second acceptance");
        let source = preserve_source(window);
        // Accept another option before this still-mounted child's old callback
        // can be delivered; no paint has yet refreshed its captured authority.
        dialog.activate(ScoreOptionControl::Track(Some(0)), window, cx);
        dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |dialog, window, cx| {
        assert_eq!(dialog.options.track, Some(0)); assert!(dialog.options.preserve_extensions);
        assert_eq!(dialog.options_generation, 2, "a callback from the prior form authority cannot overwrite current acceptance");
        let handle = export_overlay(dialog); let basis = dialog.authority(cx);
        let index = dialog.controls.iter().position(|control| *control == ScoreOptionControl::Preserve).unwrap();
        assert_eq!(handle.call("choose", vec![index.into(), basis.clone().into()], window, cx).unwrap(), json!(true));
        for _ in 0..2 {
            assert_eq!(handle.call("chooseSelection", vec![index.into(), true.into(), basis.clone().into()], window, cx).unwrap(), json!(true));
        }
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |dialog, window, _| {
        assert!(dialog.options.preserve_extensions, "legacy index-only responses never reconstruct a checkbox toggle");
        assert_eq!(dialog.options_generation, 2); assert!(receive.try_recv().is_err());
        window.remove_window();
    }).unwrap(); cx.run_until_parked();
}

#[gpui::test]
fn native_export_preserve_cannot_bypass_inactive_or_retired_overlay_authority(cx: &mut gpui::TestAppContext) {
    use verdant_action_control::native::gpui_native_owner::{dispatch_accessibility_action, drive_owners};
    use verdant_overlay_control::native::gpui_native_owner::{replace_props, detach_native_view};
    for retirement in ["disabled", "closed", "revision", "detached"] {
        let (send, receive) = async_channel::bounded(1);
        let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
            ScoreExportDialog::new(Format::Midi, ExportOptions { preserve_extensions: false, ..Default::default() }, vec!["Track".into()], send, window, cx)
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |dialog, window, cx| {
            let source = preserve_source(window); let overlay = export_overlay(dialog);
            let mut props = overlay.owner.borrow().props.clone();
            match retirement {
                "disabled" => props["disabled"] = json!(true),
                "closed" => props["open"] = json!(false),
                "revision" => props["revision"] = json!("retired export authority"),
                _ => {},
            }
            if retirement == "detached" { detach_native_view(&overlay.owner); }
            else { replace_props(&overlay.owner, props); }
            verdant_overlay_control::native::gpui_native_owner::drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        cx.run_until_parked();
        window.update(cx, |dialog, window, _| {
            assert!(!dialog.options.preserve_extensions, "{retirement} must reject the child's source proposal");
            assert_eq!(dialog.options_generation, 0); assert!(receive.try_recv().is_err());
            window.remove_window();
        }).unwrap(); cx.run_until_parked();
    }
}

#[gpui::test]
fn native_export_terminal_response_retires_same_batch_export_callbacks(cx: &mut gpui::TestAppContext) {
    for cancel_first in [true, false] {
        // Capacity greater than one exposes duplicate exports instead of letting
        // channel backpressure hide a second callback after remove_window.
        let (send, receive) = async_channel::bounded(4);
        let window = cx.open_window(gpui::size(px(580.), px(620.)), move |window, cx| {
            ScoreExportDialog::new(Format::Ust, ExportOptions::default(), vec!["Track".into()], send, window, cx)
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |dialog, window, cx| {
            let handle = export_overlay(dialog); let basis = dialog.authority(cx);
            let export = dialog.controls.iter().position(|control| *control == ScoreOptionControl::Export).unwrap();
            let first = if cancel_first { dialog.controls.iter().position(|control| *control == ScoreOptionControl::Cancel).unwrap() } else { export };
            for index in [first, export] {
                assert_eq!(handle.call("choose", vec![index.into(), basis.clone().into()], window, cx).unwrap(), json!(true));
            }
        }).unwrap();
        cx.run_until_parked();
        if cancel_first { assert!(receive.try_recv().is_err(), "a queued export cannot execute after the preceding cancellation"); }
        else { assert!(receive.try_recv().is_ok(), "the first successful export is delivered once"); }
        assert!(receive.try_recv().is_err(), "closing the form rejects the rest of its source response batch");
        assert!(window.update(cx, |_, _, _| ()).is_err());
    }
}
