//! Native regressions for both production timeline scrollbars. Pointer input
//! crosses the generated gesture adapter; no test calls the source exposes.
use super::*;
use gpui::{AppContext, Bounds, Point, Render, Subscription, TestAppContext, WindowHandle, point, size};
use verdant_continuous_control::{ContinuousControlHandle, native::gpui_native_owner::{
    all_owners, drive_owners, replace_props, resolved,
}};

const INITIAL: f64 = 960.0;
const OTHER: f64 = 240.0;
const VISIBLE: f64 = 3200.0;

struct Harness {
    editor: Entity<Editor>,
    arrange: bool,
    _changed: Subscription,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let arrange = self.arrange;
        let bar = self.editor.update(cx, |ed, cx| {
            horizontal_timeline_bar(cx.entity(), arrange, ed.theme, cx).into_any_element()
        });
        div().size_full()
            .child(div().w(px(420.0)).h(px(16.0)).child(bar))
            .child(canvas(|_, _, _| {}, |_, _, window, _| {
                window.on_mouse_event(|_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble { cx.stop_propagation(); }
                });
            }).size(px(0.0)))
    }
}

fn open(arrange: bool, cx: &mut TestAppContext) -> WindowHandle<Harness> {
    let window = cx.open_window(size(px(460.0), px(120.0)), |window, cx| {
        let editor = cx.new(|cx| {
            let mut ed = Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx);
            ed.needs_fit = false;
            ed.arrange.width = 320.0;
            ed.arrange.px_per_tick = 0.1;
            ed.arrange.start_tick = if arrange { INITIAL } else { OTHER };
            ed.session.viewport.width = 320.0;
            ed.session.viewport.px_per_tick = 0.1;
            ed.session.viewport.start_tick = if arrange { OTHER } else { INITIAL };
            ed
        });
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        Harness { editor, arrange, _changed: changed }
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window
}

fn source(window: &Window, arrange: bool) -> ContinuousControlHandle {
    let label = if arrange { "编曲区水平滚动" } else { "钢琴卷帘水平滚动" };
    let owners = all_owners().into_iter().filter(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.props.get("role").and_then(serde_json::Value::as_str) == Some("scrollbar")
            && owner.props.get("label").and_then(serde_json::Value::as_str) == Some(label)
    }).collect::<Vec<_>>();
    assert_eq!(owners.len(), 1, "one production scrollbar: {label}");
    ContinuousControlHandle { owner: owners[0].clone() }
}

fn number(source: &ContinuousControlHandle, name: &str) -> f64 {
    source.state(name).unwrap().as_f64().unwrap()
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-5, "expected {expected}, got {actual}");
}

fn viewport(editor: &Editor, arrange: bool) -> f64 {
    if arrange { editor.arrange.start_tick } else { editor.session.viewport.start_tick }
}

fn set_viewport(editor: &mut Editor, arrange: bool, value: f64) {
    if arrange { editor.arrange.start_tick = value; }
    else { editor.session.viewport.start_tick = value; }
}

// Geometry facts used to choose real screen positions and predict their tick
// coordinates. The tests specify the jump/drag policy separately below.
struct Geometry {
    bounds: Bounds<Pixels>,
    maximum: f64,
    accepted: f64,
    thumb: f64,
    travel: f64,
}

impl Geometry {
    fn new(source: &ContinuousControlHandle, visible: f64) -> Self {
        let accepted = number(source, "value");
        let owner = source.owner.borrow();
        let bounds = owner.native_hitbox.as_ref().expect("painted scrollbar").bounds;
        let maximum = owner.props["max"].as_f64().unwrap();
        let width = f64::from(bounds.size.width);
        assert!(width > 48.0 && bounds.size.height > px(0.0));
        let thumb = (width * (visible / (maximum + visible)).clamp(0.04, 1.0)).max(24.0).min(width);
        Self { bounds, maximum, accepted, thumb, travel: (width - thumb).max(1.0) }
    }

    fn thumb_point(&self) -> Point<Pixels> {
        let x = self.accepted / self.maximum * self.travel + self.thumb * 0.5;
        point(self.bounds.origin.x + px(x as f32), self.bounds.center().y)
    }

    fn slot_point(&self) -> Point<Pixels> {
        let x = f64::from(self.bounds.size.width) * 0.68;
        assert!(x > self.accepted / self.maximum * self.travel + self.thumb + 12.0);
        point(self.bounds.origin.x + px(x as f32), self.bounds.center().y)
    }

    fn centered(&self, position: Point<Pixels>) -> f64 {
        let x = f64::from(position.x) - f64::from(self.bounds.origin.x);
        ((x - self.thumb * 0.5) / self.travel).clamp(0.0, 1.0) * self.maximum
    }

    fn delta(&self, start: Point<Pixels>, end: Point<Pixels>) -> f64 {
        (f64::from(end.x) - f64::from(start.x)) / self.travel * self.maximum
    }
}

fn down(position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
        position, button: MouseButton::Left, ..Default::default()
    }), cx);
}

fn motion(position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
        position, pressed_button: Some(MouseButton::Left), ..Default::default()
    }), cx);
}

fn up(position: Point<Pixels>, fine: bool, window: &mut Window, cx: &mut App) {
    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
        position, button: MouseButton::Left,
        modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
    }), cx);
}

fn settled(host: &Harness, source: &ContinuousControlHandle, expected: f64, revision: u64,
    window: &Window, cx: &App) {
    let editor = host.editor.read(cx);
    near(viewport(editor, host.arrange), expected);
    near(viewport(editor, !host.arrange), OTHER);
    near(number(source, "value"), expected);
    near(resolved(&source.owner.borrow().accessible_states["valueNow"]).num(), expected);
    assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
    assert_eq!(editor.revision().get(), revision, "scrolling does not write the project");
    assert!(!editor.has_proto_pointer_contact());
    assert!(window.captured_hitbox().is_none());
    assert!(source.take_failures().is_empty());
}

#[gpui::test]
fn timeline_scrollbars_native_press_policy_selects_the_drag_origin(cx: &mut TestAppContext) {
    for arrange in [true, false] {
        for policy in ["jump", "drag"] {
            let window = open(arrange, cx);
            let (source, expected) = window.update(cx, |host, window, cx| {
                let source = source(window, arrange);
                // Change only authored policy. The production host receives
                // exactly the same slot geometry and native pointer positions.
                let mut props = source.owner.borrow().props.clone();
                props["trackPress"] = serde_json::json!(policy);
                replace_props(&source.owner, props);
                drive_owners(window, cx);
                let geometry = Geometry::new(&source, VISIBLE);
                let start = geometry.slot_point();
                let end = start + point(px(19.0), px(0.0));
                let origin = if policy == "jump" { geometry.centered(start) } else { INITIAL };
                down(start, window, cx);
                assert_eq!(source.state("phase").unwrap(),
                    serde_json::json!(if policy == "jump" { "dragging" } else { "armed" }));
                near(number(&source, "dragOrigin"), origin);
                near(number(&source, "initialValue"), INITIAL);
                near(number(&source, "value"), origin);
                near(viewport(host.editor.read(cx), arrange), origin);
                assert!(window.captured_hitbox().is_some());
                let expected = origin + geometry.delta(start, end);
                up(end, false, window, cx);
                near(viewport(host.editor.read(cx), arrange), expected);
                (source, expected)
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| settled(host, &source, expected, 0, window, cx)).unwrap();
        }
    }
}

#[gpui::test]
fn timeline_scrollbars_sample_terminal_shift_after_preview_repaint(cx: &mut TestAppContext) {
    for arrange in [true, false] {
        for slot in [false, true] {
            let window = open(arrange, cx);
            let (source, geometry, start, origin) = window.update(cx, |host, window, cx| {
                let source = source(window, arrange);
                let geometry = Geometry::new(&source, VISIBLE);
                let start = if slot { geometry.slot_point() } else { geometry.thumb_point() };
                let origin = if slot { geometry.centered(start) } else { INITIAL };
                down(start, window, cx);
                if slot {
                    motion(start + point(px(11.0), px(0.0)), window, cx);
                    assert_eq!(source.state("phase").unwrap(), serde_json::json!("dragging"));
                } else {
                    motion(start + point(px(2.0), px(2.0)), window, cx);
                    assert_eq!(source.state("phase").unwrap(), serde_json::json!("armed"),
                        "diagonal movement below three pixels remains armed");
                    near(viewport(host.editor.read(cx), arrange), INITIAL);
                }
                (source, geometry, start, origin)
            }).unwrap();
            // Render the Editor's echoed preview into the real scrollbar
            // again, including its candidate geometry and authority props.
            crate::native_test_support::frames(window, cx);
            let end = start + point(px(29.0), px(0.0));
            let expected = origin + geometry.delta(start, end) * 0.1;
            window.update(cx, |host, window, cx| {
                near(number(&source, "dragOrigin"), origin);
                near(number(&source, "initialValue"), INITIAL);
                assert!(window.captured_hitbox().is_some());
                // Shift appears only on MouseUp, whose bubble is stopped by
                // the Harness. This is also the thumb's first threshold pass.
                up(end, true, window, cx);
                near(viewport(host.editor.read(cx), arrange), expected);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                settled(host, &source, expected, 0, window, cx);
                up(start + point(px(43.0), px(0.0)), false, window, cx);
                settled(host, &source, expected, 0, window, cx);
            }).unwrap();
        }
    }
}

#[gpui::test]
fn timeline_scrollbars_cancel_disabled_and_reject_retired_authority(cx: &mut TestAppContext) {
    for arrange in [true, false] {
        for retirement in ["disabled", "revision", "generation"] {
            let window = open(arrange, cx);
            let (source, start) = window.update(cx, |_, window, cx| {
                let source = source(window, arrange);
                let start = Geometry::new(&source, VISIBLE).slot_point();
                down(start, window, cx);
                motion(start + point(px(13.0), px(0.0)), window, cx);
                assert_eq!(source.state("phase").unwrap(), serde_json::json!("dragging"));
                (source, start)
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            let expected = if retirement == "disabled" { INITIAL } else { 1700.0 };
            let revision = if retirement == "revision" { 1 } else { 0 };
            window.update(cx, |host, window, cx| {
                assert!(host.editor.read(cx).has_proto_pointer_contact());
                if retirement == "disabled" {
                    let mut props = source.owner.borrow().props.clone();
                    props["disabled"] = serde_json::json!(true);
                    replace_props(&source.owner, props);
                    drive_owners(window, cx);
                    assert_eq!(source.state("disabled").unwrap(), serde_json::json!(true));
                    assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
                } else {
                    host.editor.update(cx, |editor, cx| {
                        if retirement == "revision" {
                            editor.accept_source_command_selection(Cmd::ToggleMasterMute, &Arg::None, true, cx);
                        } else {
                            editor.document_generation += 1;
                        }
                        set_viewport(editor, arrange, expected);
                        cx.notify();
                    });
                }
                // This event reaches the still-mounted old target before a
                // repaint. Neither its commit nor rollback may overwrite a
                // new document authority's externally supplied viewport.
                up(start + point(px(31.0), px(0.0)), false, window, cx);
                near(viewport(host.editor.read(cx), arrange), expected);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                settled(host, &source, expected, revision, window, cx);
                up(start + point(px(47.0), px(0.0)), false, window, cx);
                settled(host, &source, expected, revision, window, cx);
            }).unwrap();
        }
    }
}

#[gpui::test]
fn native_editor_escape_cancels_timeline_scrollbars_before_late_release(cx: &mut TestAppContext) {
    for (arrange, replace_capture) in [(true, false), (false, false), (true, true)] {
        let mut project = crate::native_test_support::instrument_project();
        project.tracks.truncate(1);
        let note = verdant_domain::Note::new(0, 480, 60);
        let selected = note.id.clone();
        project.tracks[0].notes.push(note);
        let window = cx.open_window(size(px(1300.0), px(900.0)), |window, cx| {
            let bindings = crate::shortcuts::bindings(cx);
            cx.bind_keys(bindings);
            let mut editor = Editor::for_native_test(project, window, cx);
            editor.session.selection.insert(selected.clone());
            editor.bottom = crate::editor::Bottom::Editor;
            editor.needs_fit = false;
            editor.arrange.start_tick = INITIAL;
            editor.arrange.px_per_tick = 0.5;
            editor.session.viewport.start_tick = INITIAL;
            editor.session.viewport.px_per_tick = 0.5;
            editor
        });
        cx.update_window(window.into(), |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        let (visible, untouched, revision) = window.update(cx, |editor, _, _| {
            let vp = if arrange { editor.arrange } else { editor.session.viewport };
            (vp.width / vp.px_per_tick, viewport(editor, !arrange), editor.revision().get())
        }).unwrap();
        // Use the real Editor keymap and both actual view.rs entry points.
        // Dispatch outside an Editor entity update so its callback can borrow
        // that entity through the production scalar target.
        let (source, accepted, start) = cx.update_window(window.into(), |_, window, cx| {
            let source = source(window, arrange);
            let geometry = Geometry::new(&source, visible);
            let accepted = geometry.accepted;
            let start = geometry.slot_point();
            down(start, window, cx);
            motion(start + point(px(22.0), px(0.0)), window, cx);
            assert_eq!(source.state("phase").unwrap(), serde_json::json!("dragging"));
            assert_ne!(number(&source, "dragOrigin"), accepted);
            near(number(&source, "initialValue"), accepted);
            (source, accepted, start)
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        let contact = window.update(cx, |editor, _, _| {
            assert!(editor.has_proto_pointer_contact());
            assert!(editor.session.selection.contains(&selected));
            assert_eq!(editor.revision().get(), revision);
            editor.proto_pointer_contact.as_ref().and_then(std::rc::Weak::upgrade).unwrap()
        }).unwrap();
        let replacement = replace_capture.then(|| cx.update_window(window.into(), |_, window, cx| {
            crate::host_pointer::OwnedPointerCapture::acquire(window.captured_hitbox().unwrap(), window, cx)
        }).unwrap());
        cx.simulate_keystrokes(window.into(), "escape");
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, cx| {
            assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"),
                "production Editor Escape cancels the scrollbar; contexts={:?}", window.context_stack());
            near(number(&source, "value"), accepted);
            near(resolved(&source.owner.borrow().accessible_states["valueNow"]).num(), accepted);
            assert!(!contact.cancel_in_window(window, cx), "an idle contact cannot claim another Escape");
            if let Some(replacement) = replacement {
                assert!(replacement.is_current(window, cx), "old cancellation preserves a newer capture generation");
                replacement.release(window, cx);
            } else {
                assert!(window.captured_hitbox().is_none());
            }
            up(start + point(px(39.0), px(0.0)), false, window, cx);
            assert!(source.take_failures().is_empty());
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |editor, _, _| {
            near(viewport(editor, arrange), accepted);
            near(viewport(editor, !arrange), untouched);
            assert_eq!(editor.revision().get(), revision);
            assert!(!editor.has_proto_pointer_contact());
            assert!(editor.session.selection.contains(&selected), "the first Escape only cancels the active contact");
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "escape");
        window.update(cx, |editor, _, _| {
            assert!(editor.session.selection.is_empty(), "the next Escape reaches the existing Editor action");
        }).unwrap();
    }
}
