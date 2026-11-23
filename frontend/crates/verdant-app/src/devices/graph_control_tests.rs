use super::*;
use gpui::{FocusHandle, Focusable, TestAppContext, WindowHandle, point, size};
use verdant_graph_point::native::gpui_native_owner::{dispatch_accessibility_action, drive_owners, resolved};

struct Slot;
impl Render for Slot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(80.))
    }
}
struct Harness {
    root: Entity<GraphPoint>,
    show: bool,
    outside: FocusHandle,
    _lease: MoveGestureLease,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().flex_col().items_start()
            .when(self.show, |body| body.child(div().size(px(80.)).child(self.root.clone())))
            .child(div().id("outside-point").track_focus(&self.outside).size(px(20.)))
    }
}
struct Pointer(GraphPointHandle);
impl MoveGestureBinding for Pointer {
    fn should_start(&self, _: &MoveGestureSample, _: &mut Window, _: &mut App) -> bool {
        state(&self.0, "disabled") == Some(json!(false))
    }
    fn on_start(&self, _: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        self.0.call("begin", vec![false.into(), false.into()], window, cx).unwrap();
    }
    fn on_move(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        self.0.call("sample", sample_args(sample), window, cx).unwrap();
    }
    fn on_end(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        self.0.call("finish", sample_args(sample), window, cx).unwrap();
    }
    fn on_cancel(&self, _: MoveGestureCancelReason, context: Option<MoveGestureCancelContext<'_>>) {
        if let Some(context) = context {
            self.0.call("cancel", vec![], context.window, context.cx).unwrap();
        }
    }
}
fn sample_args(sample: &MoveGestureSample) -> Vec<Value> {
    vec![sample.total_delta.x.into(), sample.total_delta.y.into(), 100.0.into(), 100.0.into(), sample.modifiers.shift.into()]
}
fn props(x: f64, y: f64, revision: &str) -> Value {
    json!({"x":x,"y":y,"label":"EQ band","xLabel":"Frequency","yLabel":"Gain","revision":revision})
}
fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
    let window = cx.open_window(size(px(300.), px(240.)), |window, cx| {
        let root = GraphPoint::create(props(0.5, 0.5, "r1"), None, window, cx).unwrap();
        let slot = cx.new(|_| Slot);
        root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(slot.into())], cx)).unwrap();
        let handle = root.read(cx).handle();
        let lease = attach_move_gesture(handle.owner(), Box::new(Pointer(handle))).unwrap();
        Harness { root, show: true, outside: cx.focus_handle(), _lease: lease }
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window
}
fn commits(handle: &GraphPointHandle) -> Vec<Commit> {
    assert!(handle.take_failures().is_empty());
    handle.take_events().into_iter().filter_map(|(name, value)| {
        assert!(valid_output(&name, &value), "invalid {name}: {value}");
        (name == "valueCommit").then(|| serde_json::from_value(value).unwrap())
    }).collect()
}
fn near(actual: f64, expected: f64) { assert!((actual-expected).abs() < 1e-10, "{actual} != {expected}"); }

#[test]
fn graph_output_batch_has_a_closed_protocol_and_rejects_malformed_coordinates() {
    assert!(valid_output("valueCommit", &json!({"x":0.2,"y":0.6,"reset":false,"revision":"r1"})));
    for (name, payload) in [
        ("valueCommit", json!({"x":1.1,"y":0.5,"reset":false,"revision":"r1"})),
        ("valueCommit", json!({"x":0.5,"y":0.5,"reset":false})),
        ("selected", json!({"revision":"r1","extra":true})),
        ("interactionCancel", json!({})),
        ("exact", json!({"axis":"z","revision":"r1"})),
        ("newUnrecognizedEffect", json!({"revision":"r1"})),
    ] { assert!(!valid_output(name, &payload), "{name}: {payload}"); }
}

#[gpui::test]
fn graph_keyboard_and_accessibility_share_controlled_axes(cx: &mut TestAppContext) {
    let window = open(cx);
    window.update(cx, |host, window, cx| {
        let source = host.root.read(cx);
        assert_eq!(resolved(&source.owner.borrow().accessible_role).text(), "slider");
        assert!(source.owner.borrow().modules.a11y_node.is_some());
        window.focus(&source.focus_handle(cx), cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    cx.simulate_keystrokes(window.into(), "right");
    window.update(cx, |host, window, cx| {
        let handle = host.root.read(cx).handle();
        let outputs = commits(&handle);
        assert_eq!(outputs.len(), 1); near(outputs[0].x, 0.51); near(outputs[0].y, 0.5);
        assert_eq!(outputs[0].revision, "r1");
        assert_eq!(state(&handle, "x"), Some(json!(0.5)), "proposal waits for accepted props");
        host.root.update(cx, |root, cx| root.set_props(props(0.51, 0.5, "r2"), window, cx)).unwrap();
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    cx.simulate_keystrokes(window.into(), "shift-up");
    window.update(cx, |host, window, cx| {
        let handle = host.root.read(cx).handle();
        let outputs = commits(&handle);
        assert_eq!(outputs.len(), 1); near(outputs[0].x, 0.51); near(outputs[0].y, 0.501);
        assert_eq!(state(&handle, "axis"), Some(json!("y")));
        dispatch_accessibility_action(&handle.owner, "activate", None);
        drive_owners(window, cx);
        assert_eq!(state(&handle, "axis"), Some(json!("x")), "native Click honors authored axis activation");
        dispatch_accessibility_action(&handle.owner, "setValue", Some(&gpui::accesskit::ActionData::NumericValue(0.8)));
        drive_owners(window, cx);
        let outputs = commits(&handle);
        assert_eq!(outputs.len(), 1); near(outputs[0].x, 0.8); near(outputs[0].y, 0.5);
        near(resolved(&handle.owner.borrow().accessible_states["valueNow"]).num(), 0.51);
    }).unwrap();
}

#[gpui::test]
fn graph_native_pointer_uses_terminal_sample_and_detach_rejects_late_release(cx: &mut TestAppContext) {
    let window = open(cx);
    let handle = window.update(cx, |host, window, cx| {
        let handle = host.root.read(cx).handle();
        let start = point(px(40.), px(40.));
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {position:start,button:MouseButton::Left,..Default::default()}), cx);
        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {position:start+point(px(10.),px(-10.)),pressed_button:Some(MouseButton::Left),..Default::default()}), cx);
        near(state(&handle, "x").unwrap().as_f64().unwrap(), 0.6);
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {position:start+point(px(20.),px(-30.)),button:MouseButton::Left,..Default::default()}), cx);
        let outputs = commits(&handle);
        assert_eq!(outputs.len(), 1); near(outputs[0].x, 0.7); near(outputs[0].y, 0.8);
        assert_eq!(state(&handle, "x"), Some(json!(0.5)));
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {position:start,button:MouseButton::Left,..Default::default()}), cx);
        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {position:start+point(px(10.),px(-10.)),pressed_button:Some(MouseButton::Left),..Default::default()}), cx);
        host.show = false; cx.notify(); handle
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |_, window, cx| {
        assert!(!handle.owner.borrow().mounted);
        assert_eq!(state(&handle, "phase"), Some(json!("idle")));
        assert_eq!(state(&handle, "x"), Some(json!(0.5)));
        assert!(commits(&handle).is_empty());
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {position:point(px(60.),px(10.)),button:MouseButton::Left,..Default::default()}), cx);
        assert!(commits(&handle).is_empty());
        assert_eq!(handle.call("begin", vec![false.into(),false.into()], window, cx).unwrap(), json!(false));
    }).unwrap();
}

#[gpui::test]
fn graph_authority_change_cancels_the_retired_revision(cx: &mut TestAppContext) {
    let window = open(cx);
    window.update(cx, |host, window, cx| {
        let handle = host.root.read(cx).handle();
        handle.call("begin", vec![false.into(),false.into()], window, cx).unwrap();
        handle.call("sample", vec![20.0.into(),(-30.0).into(),100.0.into(),100.0.into(),false.into()], window, cx).unwrap();
        handle.take_events();
        host.root.update(cx, |root, cx| root.set_props(props(0.2,0.3,"r2"),window,cx)).unwrap();
        assert_eq!(handle.take_events(), vec![("interactionCancel".into(),json!({"revision":"r1"}))]);
        assert_eq!(state(&handle,"phase"),Some(json!("idle")));
        assert_eq!(state(&handle,"x"),Some(json!(0.2)));
        assert_eq!(state(&handle,"y"),Some(json!(0.3)));
        assert_eq!(handle.call("finish",vec![20.0.into(),(-30.0).into(),100.0.into(),100.0.into(),false.into()],window,cx).unwrap(),json!(false));
        assert!(commits(&handle).is_empty());
    }).unwrap();
}
