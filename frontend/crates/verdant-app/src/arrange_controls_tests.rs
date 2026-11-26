//! Actual track header consumer: source list selection/context and nested action ownership.
use super::*;
use gpui::{AppContext, Render, TestAppContext, WindowHandle, size};
use verdant_collection_control::{CollectionControlHandle, native::gpui_native_owner::{all_owners, resolved}};
use crate::editor::WorkflowAction;
use verdant_domain::{Note, ProjectStore, Track, parts::Parts};

struct Harness { editor: Entity<Editor> }
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.editor.update(cx, |editor, cx| track_headers(editor, cx).into_any_element())
    }
}
fn source(window: &Window) -> CollectionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "编曲轨道")
    }).expect("mounted actual track list");
    CollectionControlHandle { owner }
}

fn open_group_fixture(project: Project, cx: &mut TestAppContext) -> WindowHandle<Harness> {
    let window = cx.open_window(size(px(320.), px(420.)), |window, cx| Harness {
        editor: cx.new(|cx| Editor::for_native_test(project, window, cx)),
    });
    let editor = window.update(cx, |host, window, cx| {
        window.activate_window();
        // Native fixtures skip startup restoration. Install the empty insert
        // topology before edits so Undo and Redo can use their normal guard.
        host.editor.update(cx, |ed, cx| ed.restore_inserts(cx));
        host.editor.clone()
    }).unwrap();
    crate::native_test_support::wait_for_transport_ready(window, cx, &editor);
    window
}

#[gpui::test]
fn lead_in_moves_linked_instances_and_loose_notes_in_one_undo_step(cx: &mut TestAppContext) {
    let mut project = Project::new_empty();
    project.tracks.push(Track::new("Harmony", "blue"));
    let first_track = project.tracks[0].id.clone();
    let second_track = project.tracks[1].id.clone();
    let grouped = Note::new(0, 480, 60);
    let loose_first = Note::new(1200, 240, 64);
    let loose_second = Note::new(2400, 240, 67);
    project.tracks[0].notes = vec![grouped.clone(), loose_first.clone()];
    project.tracks[1].notes = vec![loose_second.clone()];

    let mut parts = Parts::default();
    let original = parts.create(&project, &first_track, std::slice::from_ref(&grouped.id), "Original".into()).unwrap();
    let linked = parts.duplicate(&original, 960, true).unwrap();
    let linked_part = parts.instances.last_mut().unwrap();
    assert_eq!(linked_part.id, linked);
    linked_part.track = second_track.clone();
    linked_part.source_offset_tick = 120;
    linked_part.duration_tick = 360;
    let linked_note = linked_part.bindings[&grouped.id].clone();
    assert_ne!(linked_note, grouped.id);
    parts.create_empty(&project, &second_track, 4800, 480, "Empty".into()).unwrap();
    let mut store = ProjectStore::new(project);
    store.apply(store.revision(), CommandBatch::single("fixture groups", parts.command())).unwrap().unwrap();
    let window = open_group_fixture(store.project().clone(), cx);
    window.update(cx, |host, _, cx| host.editor.update(cx, |ed, cx| {
        let before = Parts::load(ed.project()).unwrap();
        let depth = ed.store().undo_depth();
        let bar = ed.meter().bar_ticks_at(0);
        let assert_positions = |ed: &Editor, offset| {
            let first = ed.project().track(&first_track).unwrap();
            let second = ed.project().track(&second_track).unwrap();
            assert_eq!(first.notes.len(), 2);
            assert_eq!(second.notes.len(), 2);
            assert_eq!(first.note(&grouped.id).unwrap().start_tick, offset);
            assert_eq!(first.note(&loose_first.id).unwrap().start_tick, 1200 + offset);
            let clipped = second.note(&linked_note).unwrap();
            assert_eq!((clipped.start_tick, clipped.duration_tick), (960 + offset, 360));
            assert_eq!(second.note(&loose_second.id).unwrap().start_tick, 2400 + offset);
            let mut expected = before.clone();
            for part in &mut expected.instances { part.start_tick += offset; }
            assert_eq!(Parts::load(ed.project()).unwrap(), expected, "source notes, bindings, trims and empty groups survive");
        };
        assert_positions(ed, 0);
        ed.shift_notes_one_bar(cx);
        assert_eq!(ed.store().undo_depth(), depth + 1);
        assert_positions(ed, bar);
        ed.undo(cx);
        assert_eq!(ed.store().undo_depth(), depth);
        assert_positions(ed, 0);
        ed.redo(cx);
        assert_positions(ed, bar);
    })).unwrap();
}

#[gpui::test]
fn ungroup_shortcut_targets_only_the_active_track(cx: &mut TestAppContext) {
    use crate::shortcuts::{WorkflowShortcut, dispatch};

    let mut project = Project::new_empty();
    project.tracks.push(Track::new("Harmony", "blue"));
    let first_track = project.tracks[0].id.clone();
    let second_track = project.tracks[1].id.clone();
    let first_note = Note::new(0, 480, 60);
    let second_note = Note::new(960, 480, 67);
    project.tracks[0].notes.push(first_note.clone());
    project.tracks[1].notes.push(second_note.clone());
    let mut parts = Parts::default();
    let first = parts.create(&project, &first_track, std::slice::from_ref(&first_note.id), "First".into()).unwrap();
    let second = parts.create(&project, &second_track, std::slice::from_ref(&second_note.id), "Second".into()).unwrap();
    project.extensions.insert(verdant_domain::parts::KEY.into(), serde_json::to_value(&parts).unwrap());

    let window = open_group_fixture(project, cx);
    window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
        let before = ed.revision();
        ed.run_workflow(WorkflowAction::SelectPart(first.clone()), window, cx);
        assert!(ed.select_track(first_track.clone(), cx));
        assert_eq!(ed.selected_part.as_deref(), Some(first.as_str()));
        assert!(ed.select_track(second_track.clone(), cx));
        assert!(ed.selected_part.is_none());
        dispatch(ed, &WorkflowShortcut::DissolvePart, window, cx);
        assert_eq!(ed.revision(), before);

        ed.selected_part = Some(first.clone());
        dispatch(ed, &WorkflowShortcut::DissolvePart, window, cx);
        assert_eq!(ed.revision(), before, "a stale cross-track target is rejected at dispatch");
        assert_eq!(Parts::load(ed.project()).unwrap(), parts);

        ed.run_workflow(WorkflowAction::SelectPart(second.clone()), window, cx);
        dispatch(ed, &WorkflowShortcut::DissolvePart, window, cx);
        let dissolved = Parts::load(ed.project()).unwrap();
        assert_eq!(dissolved.instance(&first), parts.instance(&first));
        assert!(dissolved.instance(&second).is_none());
        let loose = &ed.project().track(&second_track).unwrap().notes;
        assert_eq!(loose.len(), 1);
        assert_eq!(loose[0].start_tick, second_note.start_tick);
        ed.undo(cx);
        assert_eq!(Parts::load(ed.project()).unwrap(), parts);
        ed.redo(cx);
        assert_eq!(Parts::load(ed.project()).unwrap(), dissolved);
    })).unwrap();
}

#[gpui::test]
fn native_track_headers_own_nested_activation_arrow_selection_and_keyboard_context(cx: &mut TestAppContext) {
    let window = cx.open_window(size(px(320.), px(420.)), |window, cx| Harness {
        editor: cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx)),
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap(); crate::native_test_support::frames(window, cx);
    let original_color = window.update(cx, |host, _, cx| host.editor.read(cx).project().tracks[1].color.clone()).unwrap();
    window.update(cx, |_, window, cx| {
        let mut colors = all_owners().into_iter().filter(|owner| {
            let owner = owner.borrow(); owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == "切换轨道颜色")
        }).collect::<Vec<_>>();
        colors.sort_by(|a,b| a.borrow().bounds.unwrap().origin.y.partial_cmp(&b.borrow().bounds.unwrap().origin.y).unwrap());
        assert_eq!(colors.len(), 2);
        let position = colors[1].borrow().bounds.unwrap().center();
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button:MouseButton::Left, ..Default::default() }), cx);
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button:MouseButton::Left, ..Default::default() }), cx);
    }).unwrap(); crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let editor = host.editor.read(cx);
        assert_eq!(editor.session.track, editor.project().tracks[0].id, "child action must not also select its containing row");
        assert_ne!(editor.project().tracks[1].color, original_color);
        source(window).call("focusSelf", vec![], window, cx).unwrap();
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "down"); crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let editor = host.editor.read(cx); assert_eq!(editor.session.track, editor.project().tracks[1].id);
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "shift-f10"); crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.is_some(), "source context request reaches the admitted track command")).unwrap();
}
