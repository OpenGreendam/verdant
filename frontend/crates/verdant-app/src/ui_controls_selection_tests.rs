use super::*;
use gpui::{AppContext, Render, Subscription, TestAppContext, size};
use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{
    all_owners, resolved, replace_props, dispatch_accessibility_action, drive_owners,
}};

struct Harness { editor: Entity<Editor>, cmd: Cmd, arg: Arg, _changed: Subscription }
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cmd = self.cmd; let arg = self.arg.clone();
        self.editor.update(cx, |ed, cx| {
            // A mode's active styling is intentionally not Boolean authority.
            selectable_command_button(ed, cmd, arg, "Source selectable command", "source-selectable-command", true, cx)
                .w(px(240.)).h(px(40.)).into_any_element()
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
        let (cmd, arg) = match family {
            0 => (Cmd::ToggleMute, Arg::Track(track)),
            1 => (Cmd::ToggleSolo, Arg::Track(track)),
            2 => (Cmd::ToggleTrackAutomation, Arg::Track(track)),
            3 => (Cmd::ToggleMasterMute, Arg::None),
            4 => (Cmd::ToggleMasterAutomation, Arg::None),
            5 => (Cmd::TogglePhonemeApproximation, Arg::Note(note_id)),
            _ => (Cmd::SetToolPencil, Arg::None),
        };
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        Harness { editor, cmd, arg, _changed: changed }
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
    let window = fixture(6, cx);
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
