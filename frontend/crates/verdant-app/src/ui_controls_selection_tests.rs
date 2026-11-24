use super::*;
use gpui::{AppContext, Render, Subscription, TestAppContext, size};
use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{
    all_owners, resolved, replace_props, dispatch_accessibility_action, drive_owners,
}};

struct Harness { editor: Entity<Editor>, cmd: Cmd, arg: Arg, surface: u8, _changed: Subscription }
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cmd = self.cmd; let arg = self.arg.clone(); let surface = self.surface;
        self.editor.update(cx, |ed, cx| {
            // A mode's active styling is intentionally not Boolean authority.
            let button = match surface {
                1 => flat_command_button(ed, cmd, arg, "Source selectable command", "source-flat-command", true, cx),
                2 => crate::view::switch_row(ed, cmd, "source-switch-row", "Source selectable command", None, true, cx),
                _ => selectable_command_button(ed, cmd, arg, "Source selectable command", "source-selectable-command", true, cx),
            };
            button.w(px(240.)).h(px(40.)).into_any_element()
        })
    }
}

fn fixture(family: u8, cx: &mut TestAppContext) -> gpui::WindowHandle<Harness> {
    cx.open_window(size(px(300.), px(160.)), |window, cx| {
        let mut project = verdant_domain::Project::new_empty();
        let track = project.tracks[0].id.clone();
        let mut note = verdant_domain::Note::new(0, 960, 60);
        note.pronunciation.language = Some("en".into());
        note.pronunciation.alphabet = Some("ipa".into());
        note.pronunciation.phonemes = vec!["a".into()];
        let note_id = note.id.clone();
        project.tracks[0].notes.push(note);
        let editor = cx.new(|cx| Editor::for_native_test(project, window, cx));
        editor.update(cx, |ed, _| {
            ed.layout.show_phoneme_lane = false;
            ed.layout.show_tempo_lane = false;
            ed.integrations.other_tracks = false;
            ed.show_f0 = false;
            ed.prefs_dirty = false;
        });
        let (cmd, arg) = match family {
            0 => (Cmd::ToggleMute, Arg::Track(track)),
            1 => (Cmd::ToggleSolo, Arg::Track(track)),
            2 => (Cmd::ToggleTrackAutomation, Arg::Track(track)),
            3 => (Cmd::ToggleMasterMute, Arg::None),
            4 => (Cmd::ToggleMasterAutomation, Arg::None),
            5 => (Cmd::TogglePhonemeApproximation, Arg::Note(note_id)),
            10 => (Cmd::ToggleLoop, Arg::None),
            11 => (Cmd::ToggleMetronome, Arg::None),
            12 => (Cmd::TogglePhonemeLane, Arg::None),
            13 => (Cmd::ToggleTempoLane, Arg::None),
            14 => (Cmd::ToggleOtherTracks, Arg::None),
            15 => (Cmd::ToggleF0Overlay, Arg::None),
            16 => (Cmd::TogglePhonemeTiming, Arg::None),
            17 => (Cmd::ToggleOutputTransients, Arg::None),
            _ => (Cmd::SetToolPencil, Arg::None),
        };
        let surface = if family == 16 || family == 17 { 2 } else if family >= 10 { 1 } else { 0 };
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        Harness { editor, cmd, arg, surface, _changed: changed }
    })
}

fn source(window: &Window) -> ActionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name")
                .is_some_and(|name| resolved(name).text() == "Source selectable command")
    }).expect("selectable command uses generated ActionControl");
    ActionControlHandle { owner }
}

#[gpui::test]
fn boolean_commands_consume_source_selection_once_and_reject_stale_document_events(cx: &mut TestAppContext) {
    for family in 0..6 {
        let window = fixture(family, cx);
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.editor.read(cx).source_command_selection(host.cmd, &host.arg), Some(false));
            let source = source(window);
            let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(true);
            replace_props(&source.owner, props); drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.editor.read(cx).source_command_selection(host.cmd, &host.arg), Some(false),
                "family {family} accepts source false rather than toggling accepted false");
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            let source = source(window);
            let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(false);
            replace_props(&source.owner, props); drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        let revision = window.update(cx, |host, window, cx| {
            assert_eq!(host.editor.read(cx).source_command_selection(host.cmd, &host.arg), Some(true));
            let revision = host.editor.read(cx).revision().get();
            assert_eq!(revision, if family == 2 || family == 4 { 0 } else { 1 });
            let source = source(window);
            host.editor.update(cx, |ed, _| ed.document_generation += 1);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
            revision
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            assert_eq!(host.editor.read(cx).source_command_selection(host.cmd, &host.arg), Some(true));
            assert_eq!(host.editor.read(cx).revision().get(), revision);
        }).unwrap();
        window.update(cx, |_, window, _| window.remove_window()).unwrap(); cx.run_until_parked();
    }
}

#[gpui::test]
fn an_active_mode_button_remains_a_command_when_source_selection_is_false(cx: &mut TestAppContext) {
    for family in [6, 18] {
        let window = fixture(family, cx);
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.editor.read(cx).source_command_selection(host.cmd, &host.arg), None);
            assert_eq!(host.editor.read(cx).tool, crate::editor::Tool::Pointer);
            dispatch_accessibility_action(&source(window).owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            assert_eq!(host.editor.read(cx).tool, crate::editor::Tool::Pencil);
            assert_eq!(host.editor.read(cx).revision().get(), 0, "a mode change creates no document transaction");
        }).unwrap();
        window.update(cx, |_, window, _| window.remove_window()).unwrap(); cx.run_until_parked();
    }
}

#[gpui::test]
fn flat_controls_and_synthesis_switch_rows_preserve_proposals_and_ui_authority(cx: &mut TestAppContext) {
    for family in 10..18 {
        let window = fixture(family, cx);
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            // The first native frame records window bounds in preferences.
            // Establish a clean baseline after that observer, before the input.
            host.editor.update(cx, |ed, _| ed.prefs_dirty = false);
            assert_eq!(host.editor.read(cx).source_command_selection(host.cmd, &host.arg), Some(false));
            let source = source(window);
            let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(true);
            replace_props(&source.owner, props); drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let ed = host.editor.read(cx);
            assert_eq!(ed.source_command_selection(host.cmd, &host.arg), Some(false), "family {family} accepts source false");
            assert_eq!(ed.revision().get(), 0);
            assert!(!ed.prefs_dirty, "family {family}: a no-op must not dirty layout preferences");
            let source = source(window);
            let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(false);
            replace_props(&source.owner, props); drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        let persisted = matches!(family, 10 | 11 | 16 | 17);
        window.update(cx, |host, window, cx| {
            let ed = host.editor.read(cx);
            assert_eq!(ed.source_command_selection(host.cmd, &host.arg), Some(true));
            assert_eq!(ed.revision().get(), if persisted { 1 } else { 0 });
            if family == 10 {
                let settings = ed.playback_settings();
                assert!(settings.loop_end > settings.loop_start, "enabling a loop retains the ordinary range repair");
            }
            if family == 12 || family == 13 {
                let stored = LayoutPrefs::from_prefs(&ed.prefs);
                assert_eq!(stored.show_phoneme_lane, family == 12);
                assert_eq!(stored.show_tempo_lane, family == 13);
                assert!(ed.prefs_dirty);
            }
            if family == 16 {
                assert_eq!(ed.project().render_preferences.phoneme_timing, Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1()));
            }
            if family == 17 {
                assert_eq!(ed.project().render_preferences.output_transients, Some(verdant_domain::OutputTransientsSettings::phoneme_output_envelope_v1()));
            }
            let source = source(window);
            // A UI-only change has no document revision. The captured accepted
            // value must still retire this queued proposal before redraw.
            host.editor.update(cx, |ed, cx| ed.accept_source_command_selection(host.cmd, &host.arg, false, cx));
            let mut props = source.owner.borrow().props.clone(); props["selected"] = serde_json::json!(false);
            replace_props(&source.owner, props); drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert_eq!(ed.source_command_selection(host.cmd, &host.arg), Some(false), "family {family} rejects a retired accepted value");
            assert_eq!(ed.revision().get(), if persisted { 2 } else { 0 });
        }).unwrap();
        window.update(cx, |_, window, _| window.remove_window()).unwrap(); cx.run_until_parked();
    }
}
