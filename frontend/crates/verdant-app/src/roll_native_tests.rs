//! Real native input -> production roll/session -> durable project tests.
use super::*;
use std::{cell::Cell, rc::Rc};
use gpui::{AppContext, Context, FocusHandle, Render, ScrollDelta, TestAppContext, WindowHandle, div, point, size};

struct Harness {
    editor: Entity<Editor>,
    show: bool,
    pitch: bool,
    outside: FocusHandle,
    block_up: Rc<Cell<bool>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.editor.read(cx).focus_roll.clone();
        let block_up = self.block_up.clone();
        div().relative().size_full()
            .when(self.show, |view| view.child(div().id("native-roll").track_focus(&focus).w(px(480.0)).h(px(180.0))
                .child(if self.pitch { crate::proto_roll::view(self.editor.read(cx).compiled_roll.clone()).into_any_element() }
                    else { piano_roll(self.editor.clone()).into_any_element() })))
            .child(div().id("outside-roll").track_focus(&self.outside).absolute().left(px(510.0)).top(px(190.0)).size(px(20.0)))
            .child(canvas(|_, _, _| {}, move |_, _, window, _| {
                let touch_block = block_up.clone();
                window.on_mouse_event(move |event: &TouchDragEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble && event.phase == TouchPhase::Ended && touch_block.get() { cx.stop_propagation(); }
                });
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble && block_up.get() { cx.stop_propagation(); }
                });
            }).size(px(0.0)))
    }
}

fn open(cx: &mut TestAppContext, pitch: bool) -> WindowHandle<Harness> {
    let mut project = Project::new_empty();
    project.tracks[0].notes = vec![Note::new(100, 100, 60)];
    project.tracks[0].refresh_index();
    let window = cx.open_window(size(px(580.0), px(240.0)), |window, cx| {
        let editor = cx.new(|cx| {
            let mut editor = Editor::for_native_test(project, window, cx);
            editor.needs_fit = false;
            editor.session.snap.grid_ticks = 0;
            editor.session.viewport.start_tick = 0.0;
            editor.session.viewport.px_per_tick = 1.0;
            editor.session.viewport.top_pitch = 65.0;
            editor.session.viewport.row_height = 20.0;
            if pitch { editor.tool = Tool::Pitch; }
            editor
        });
        Harness { editor, show: true, pitch, outside: cx.focus_handle(), block_up: Rc::new(Cell::new(false)) }
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window
}

fn mouse_down(window: &mut Window, cx: &mut App, position: gpui::Point<Pixels>) {
    window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
        position, button: MouseButton::Left, ..Default::default()
    }), cx);
}

fn mouse_up(window: &mut Window, cx: &mut App, position: gpui::Point<Pixels>) {
    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
        position, button: MouseButton::Left, ..Default::default()
    }), cx);
}

fn touch(window: &mut Window, cx: &mut App, phase: TouchPhase, position: gpui::Point<Pixels>) {
    window.dispatch_event(gpui::PlatformInput::Touch(gpui::TouchEvent {
        id: gpui::TouchId(11), phase, position, ..Default::default()
    }), cx);
}

fn note(host: &Harness, cx: &App) -> Note { host.editor.read(cx).project().tracks[0].notes[0].clone() }

fn editing_source(window: &Window) -> verdant_editor_surface::EditorSurfaceHandle {
    let owner = verdant_editor_surface::native::gpui_native_owner::all_owners().into_iter().find(|owner| {
        let source = owner.borrow();
        source.mounted && source.window_id == Some(window.window_handle().window_id()) && source.exposes.contains_key("pointer")
    }).expect("compiled roll editing root");
    verdant_editor_surface::EditorSurfaceHandle { owner }
}

#[gpui::test]
fn note_final_up_sample_commits_through_a_stopped_child_bubble(cx: &mut TestAppContext) {
    let window = open(cx, false);
    window.update(cx, |host, window, cx| {
        mouse_down(window, cx, point(px(150.0), px(90.0)));
        assert_eq!(host.editor.read(cx).session.pointer_owner(), Some(PointerId::Mouse));
        assert!(window.captured_hitbox().is_some());
        host.block_up.set(true);
        // The OS coalesces every move; the terminal sample is still an edit.
        mouse_up(window, cx, point(px(210.0), px(90.0)));
        assert_eq!(note(host, cx).start_tick, 160);
        assert_eq!(host.editor.read(cx).revision().get(), 1);
        assert!(host.editor.read(cx).session.pointer_owner().is_none());
        assert!(window.captured_hitbox().is_none());
    }).unwrap();
}

#[gpui::test]
fn note_exit_blur_detach_and_project_change_release_capture_before_up(cx: &mut TestAppContext) {
    for reason in ["exit", "escape", "blur", "detach", "project"] {
        let window = open(cx, false);
        window.update(cx, |host, window, cx| {
            mouse_down(window, cx, point(px(150.0), px(90.0)));
            assert!(host.editor.read(cx).session.pointer_owner().is_some());
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |_,window,cx| {
            let source=editing_source(window);
            assert_eq!(source.state("focused").unwrap(),serde_json::json!(true),"roll owns focus before {reason}; native focus: {:?}",window.focused(cx));
        }).unwrap();
        window.update(cx, |host, window, cx| match reason {
            "exit" => { window.dispatch_event(gpui::PlatformInput::MouseExited(gpui::MouseExitEvent::default()), cx); }
            "escape" => {},
            "blur" => window.focus(&host.outside, cx),
            "detach" => { host.show = false; cx.notify(); }
            "project" => host.editor.update(cx, |editor, cx| {
                let track = editor.session.track.clone();
                let mut header = editor.project().track(&track).unwrap().header();
                header.name = "Changed while dragging".into();
                assert!(editor.commit(editor.revision(), CommandBatch::single("External project change", Command::SetTrackHeader { track, header }), cx));
            }),
            _ => unreachable!(),
        }).unwrap();
        if reason == "escape" { cx.simulate_keystrokes(window.into(), "escape"); }
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            if reason == "blur" {
                let source=editing_source(window);
                assert_eq!(source.state("focused").unwrap(), serde_json::json!(false), "physical roll blur; failures {:?}", source.take_failures());
                assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"), "source roll blur; failures {:?}", source.take_failures());
            }
            assert!(host.editor.read(cx).session.pointer_owner().is_none(), "{reason} retired logical ownership");
            assert!(window.captured_hitbox().is_none(), "{reason} released native capture without up");
            let revision = host.editor.read(cx).revision();
            mouse_up(window, cx, point(px(210.0), px(90.0)));
            assert_eq!(host.editor.read(cx).revision(), revision);
            assert_eq!(note(host, cx).start_tick, 100);
        }).unwrap();
    }
}

#[gpui::test]
fn note_touch_keeps_its_native_arena_when_mouse_up_and_exit_arrive(cx: &mut TestAppContext) {
    let window = open(cx, false);
    window.update(cx, |host, window, cx| {
        touch(window, cx, TouchPhase::Started, point(px(150.0), px(90.0)));
        touch(window, cx, TouchPhase::Moved, point(px(165.0), px(90.0)));
        assert_eq!(host.editor.read(cx).session.pointer_owner(), Some(PointerId::Touch(0)));
        assert!(host.editor.read(cx).roll_capture.is_none(), "touch ownership is the GPUI arena, not mouse capture");
        mouse_down(window, cx, point(px(170.0), px(90.0)));
        mouse_up(window, cx, point(px(230.0), px(90.0)));
        window.dispatch_event(gpui::PlatformInput::MouseExited(gpui::MouseExitEvent::default()), cx);
        assert_eq!(host.editor.read(cx).session.pointer_owner(), Some(PointerId::Touch(0)));
        assert_eq!(host.editor.read(cx).revision().get(), 0);
        host.block_up.set(true);
        touch(window, cx, TouchPhase::Ended, point(px(210.0), px(90.0)));
        assert_eq!(note(host, cx).start_tick, 160);
        assert_eq!(host.editor.read(cx).revision().get(), 1);
        assert!(host.editor.read(cx).session.pointer_owner().is_none());
    }).unwrap();
}

#[gpui::test]
fn note_mod_handle_blur_uses_the_same_cancel_executor(cx: &mut TestAppContext) {
    let window = open(cx, false);
    window.update(cx, |host, _, cx| host.editor.update(cx, |editor, cx| {
        editor.session.selection.insert(editor.project().tracks[0].notes[0].id.clone());
        cx.notify();
    })).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let position = {
            let editor = host.editor.read(cx);
            let note = &editor.project().tracks[0].notes[0];
            let (portamento, _) = note_mod_handle_positions(editor, note, &editor.session.viewport.note_rect(note), &editor.tempo());
            point(px(portamento.0 as f32), px(portamento.1 as f32))
        };
        mouse_down(window, cx, position);
        assert!(has_note_mod_drag(host.editor.entity_id(), cx));
        assert!(window.captured_hitbox().is_some());
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| window.focus(&host.outside, cx)).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let source=editing_source(window);
        assert_eq!(source.state("focused").unwrap(), serde_json::json!(false), "physical modulation blur; failures {:?}", source.take_failures());
        assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"), "source modulation blur; failures {:?}", source.take_failures());
        assert!(!has_note_mod_drag(host.editor.entity_id(), cx));
        assert!(window.captured_hitbox().is_none());
        mouse_up(window, cx, point(px(190.0), px(80.0)));
        assert_eq!(host.editor.read(cx).revision().get(), 0);
        assert_eq!(note(host, cx).portamento_ms, None);
    }).unwrap();
}

#[gpui::test]
fn production_pitch_touch_uses_final_sample_and_ignores_a_rival_mouse(cx: &mut TestAppContext) {
    let window = open(cx, true);
    window.update(cx, |host, window, cx| {
        touch(window, cx, TouchPhase::Started, point(px(110.0), px(90.0)));
        assert!(host.editor.read(cx).lane_stroke.as_ref().is_some_and(|stroke| stroke.roll && stroke.pointer == PointerId::Touch(0)));
        mouse_up(window, cx, point(px(180.0), px(20.0)));
        assert_eq!(host.editor.read(cx).revision().get(), 0);
        assert!(host.editor.read(cx).lane_stroke.is_some());
        touch(window, cx, TouchPhase::Ended, point(px(190.0), px(70.0)));
        assert!(host.editor.read(cx).lane_stroke.is_none());
        assert_eq!(host.editor.read(cx).revision().get(), 1);
        let drawn = note(host, cx).curves.pitch_millicent_points;
        assert!(drawn.iter().any(|point| point.tick == 90 && point.millicents == 100_000), "final touch sample reached the note curve: {drawn:?}");
    }).unwrap();
}

#[gpui::test]
fn invalid_terminal_sample_cancels_note_pencil_and_pitch_previews(cx: &mut TestAppContext) {
    for tool in [Tool::Pointer, Tool::Pencil, Tool::Pitch] {
        let window = open(cx, tool == Tool::Pitch);
        window.update(cx, |host, window, cx| {
            host.editor.update(cx, |editor, _| editor.tool = tool);
            let start = if tool == Tool::Pencil { point(px(260.0), px(90.0)) } else { point(px(150.0), px(90.0)) };
            mouse_down(window, cx, start);
            window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                position: start + point(px(30.0), px(-10.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
            }), cx);
            mouse_up(window, cx, point(px(f32::NAN), px(90.0)));
            let editor = host.editor.read(cx);
            assert_eq!(editor.revision().get(), 0, "invalid terminal sample cannot commit {tool:?}");
            assert!(editor.note_creation.is_none());
            assert!(editor.lane_stroke.is_none());
            assert!(editor.session.pointer_owner().is_none());
            assert!(window.captured_hitbox().is_none());
            assert_eq!(note(host, cx).start_tick, 100);
        }).unwrap();
    }
}

#[gpui::test]
fn production_roll_window_bounds_cancel_mouse_and_touch_without_leave(cx: &mut TestAppContext) {
    for tool in [Tool::Pointer, Tool::Pencil, Tool::Pitch] {
        for is_touch in [false, true] {
            for terminal_only in [false, true] {
                let window = open(cx, tool == Tool::Pitch);
                window.update(cx, |host, window, cx| {
                    host.editor.update(cx, |editor, _| editor.tool = tool);
                    let start = if tool == Tool::Pencil { point(px(260.0), px(90.0)) } else { point(px(150.0), px(90.0)) };
                    let preview = start + point(px(30.0), px(-10.0));
                    if is_touch {
                        touch(window, cx, TouchPhase::Started, start);
                        touch(window, cx, TouchPhase::Moved, preview);
                    } else {
                        mouse_down(window, cx, start);
                        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                            position: preview, pressed_button: Some(MouseButton::Left), ..Default::default()
                        }), cx);
                    }
                    let editor = host.editor.read(cx);
                    assert!(editor.session.pointer_owner().is_some() || editor.note_creation.is_some() || editor.lane_stroke.is_some());
                    // No MouseExited notification: reproduce an implicit grab
                    // delivering a negative move or an outside terminal sample.
                    let outside = if terminal_only { point(px(610.0), px(90.0)) } else { point(px(-20.0), px(90.0)) };
                    if is_touch {
                        touch(window, cx, if terminal_only { TouchPhase::Ended } else { TouchPhase::Moved }, outside);
                    } else if terminal_only { mouse_up(window, cx, outside); }
                    else {
                        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                            position: outside, pressed_button: Some(MouseButton::Left), ..Default::default()
                        }), cx);
                    }
                    let editor = host.editor.read(cx);
                    assert!(editor.note_creation.is_none(), "{tool:?} touch={is_touch} up={terminal_only}");
                    assert!(editor.lane_stroke.is_none());
                    assert!(editor.session.pointer_owner().is_none());
                    assert!(window.captured_hitbox().is_none());
                    assert_eq!(editor.revision().get(), 0);
                    if !terminal_only {
                        if is_touch { touch(window, cx, TouchPhase::Ended, preview); }
                        else { mouse_up(window, cx, preview); }
                    }
                    assert_eq!(host.editor.read(cx).revision().get(), 0, "late release cannot revive the cancelled edit");
                    assert_eq!(note(host, cx).start_tick, 100);
                }).unwrap();
                crate::native_test_support::frames(window, cx);
            }
        }
    }
}

#[gpui::test]
fn note_mod_window_bounds_cancel_before_outside_sample_or_commit(cx: &mut TestAppContext) {
    for terminal_only in [false, true] {
        let window = open(cx, false);
        window.update(cx, |host, _, cx| host.editor.update(cx, |editor, cx| {
            editor.session.selection.insert(editor.project().tracks[0].notes[0].id.clone());
            cx.notify();
        })).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let position = {
                let editor = host.editor.read(cx);
                let note = &editor.project().tracks[0].notes[0];
                let (portamento, _) = note_mod_handle_positions(editor, note, &editor.session.viewport.note_rect(note), &editor.tempo());
                point(px(portamento.0 as f32), px(portamento.1 as f32))
            };
            mouse_down(window, cx, position);
            assert!(has_note_mod_drag(host.editor.entity_id(), cx));
            let outside = point(px(170.0), px(-20.0));
            if terminal_only { mouse_up(window, cx, outside); }
            else {
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: outside, pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
            }
            assert!(!has_note_mod_drag(host.editor.entity_id(), cx));
            assert!(window.captured_hitbox().is_none());
            if !terminal_only { mouse_up(window, cx, point(px(190.0), px(80.0))); }
            assert_eq!(host.editor.read(cx).revision().get(), 0);
            assert_eq!(note(host, cx).portamento_ms, None);
        }).unwrap();
    }
}


#[gpui::test]
fn late_surface_cancellation_cannot_retire_the_next_contact_on_the_same_root(cx: &mut TestAppContext) {
    let window = open(cx, false);
    window.update(cx, |host, window, cx| {
        let position = point(px(150.0), px(90.0));
        mouse_down(window, cx, position);
        crate::proto_editor_surface::cancel_from_editor_deferred(host.editor.entity_id(), cx);
        mouse_up(window, cx, position);
        mouse_down(window, cx, position);
        assert!(crate::proto_editor_surface::has_contact(host.editor.entity_id(), cx));
    }).unwrap();
    cx.run_until_parked();
    window.update(cx, |host, window, cx| {
        assert_eq!(host.editor.read(cx).session.pointer_owner(), Some(PointerId::Mouse));
        assert!(window.captured_hitbox().is_some(), "old cleanup preserves the replacement native capture");
        mouse_up(window, cx, point(px(210.0), px(90.0)));
        assert_eq!(note(host, cx).start_tick, 160);
        assert_eq!(host.editor.read(cx).revision().get(), 1);
        assert!(!crate::proto_editor_surface::has_contact(host.editor.entity_id(), cx));
    }).unwrap();
}

#[gpui::test]
fn compiled_roll_wheel_uses_pixel_and_line_transport_with_source_shift_policy(cx: &mut TestAppContext) {
    let window = open(cx, false);
    let initial = Viewport { start_tick: 2000.0, ..Viewport::default() };
    for delta in [
        ScrollDelta::Pixels(point(px(-40.0), px(0.0))),
        ScrollDelta::Pixels(point(px(0.0), px(-40.0))),
        ScrollDelta::Lines(point(-1.0, 0.0)),
        ScrollDelta::Lines(point(0.0, -1.0)),
        ScrollDelta::Pixels(point(px(-40.0), px(80.0))),
    ] {
        window.update(cx, |host, window, cx| {
            host.editor.update(cx, |ed, _| ed.session.viewport = initial);
            window.dispatch_event(gpui::PlatformInput::Scroll(ScrollEvent {
                position: point(px(300.0), px(100.0)), delta,
                modifiers: gpui::Modifiers { shift: true, ..Default::default() }, ..Default::default()
            }), cx);
            let vp = host.editor.read(cx).session.viewport;
            assert_eq!(vp.start_tick, initial.start_tick + 40.0 / initial.px_per_tick);
            assert_eq!(vp.top_pitch, initial.top_pitch);
            assert_eq!(vp.px_per_tick, initial.px_per_tick);
            assert_eq!(vp.row_height, initial.row_height);
        }).unwrap();
    }
}

#[gpui::test]
fn compiled_roll_wheel_keeps_source_modifier_priority_and_native_zoom_anchor(cx: &mut TestAppContext) {
    let window = open(cx, false);
    let initial = Viewport { start_tick: 2000.0, ..Viewport::default() };
    let anchor = (300.0, 100.0);
    for (modifiers, horizontal) in [
        (gpui::Modifiers { control: true, alt: true, shift: true, ..Default::default() }, true),
        (gpui::Modifiers { platform: true, alt: true, shift: true, ..Default::default() }, true),
        (gpui::Modifiers { alt: true, shift: true, ..Default::default() }, false),
    ] {
        window.update(cx, |host, window, cx| {
            host.editor.update(cx, |ed, _| ed.session.viewport = initial);
            window.dispatch_event(gpui::PlatformInput::Scroll(ScrollEvent {
                position: point(px(anchor.0 as f32), px(anchor.1 as f32)),
                delta: ScrollDelta::Lines(point(1.0, 1.0)), modifiers, ..Default::default()
            }), cx);
            let vp = host.editor.read(cx).session.viewport;
            if horizontal {
                assert!(vp.px_per_tick > initial.px_per_tick);
                assert!((vp.x_to_tick(anchor.0) - initial.x_to_tick(anchor.0)).abs() < 1e-9);
                assert_eq!(vp.top_pitch, initial.top_pitch);
                assert_eq!(vp.row_height, initial.row_height);
            } else {
                assert!(vp.row_height > initial.row_height);
                assert!((vp.y_to_pitch(anchor.1) - initial.y_to_pitch(anchor.1)).abs() < 1e-9);
                assert_eq!(vp.start_tick, initial.start_tick);
                assert_eq!(vp.px_per_tick, initial.px_per_tick);
            }
        }).unwrap();
    }
}
