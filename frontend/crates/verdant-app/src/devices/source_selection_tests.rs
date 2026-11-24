//! Exercise the production 0/1 parameter switch with a real durable instrument
//! transaction. This does not start or simulate an external plug-in host.
use super::*;
use gpui::{AppContext, Render, Subscription, TestAppContext, WindowHandle, size};
use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{
    all_owners, resolved, replace_props, dispatch_accessibility_action, drive_owners,
}};

const PARAMETER: &str = "filter2_enabled";
const LABEL: &str = "RCIP boolean parameter";

struct Harness { editor: Entity<Editor>, target: Target, _changed: Subscription }
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target = self.target.clone();
        self.editor.update(cx, |ed, cx| {
            let params = current_params(ed, &target).expect("real instrument parameters");
            div().size_full().child(toggle(ed, &target.track, target.slot, &params, PARAMETER, LABEL, cx))
        })
    }
}

fn fixture(cx: &mut TestAppContext) -> WindowHandle<Harness> {
    let window = cx.open_window(size(px(320.), px(120.)), |window, cx| {
        let editor = cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx));
        let ed = editor.read(cx);
        let target = target_for(ed, &DeviceTarget::Track(ed.session.track.clone()), crate::instrument::INSTRUMENT_SLOT)
            .expect("real instrument target");
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        Harness { editor, target, _changed: changed }
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window
}

fn source(window: &Window) -> ActionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name")
                .is_some_and(|name| resolved(name).text() == LABEL)
    }).expect("production parameter switch uses generated ActionControl");
    ActionControlHandle { owner }
}

fn value(host: &Harness, cx: &gpui::App) -> f64 {
    param(&current_params(host.editor.read(cx), &host.target).unwrap(), PARAMETER).unwrap().value
}

#[gpui::test]
fn device_boolean_switch_preserves_source_value_and_duplicate_proposals(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |_, window, cx| {
        let source = source(window);
        let mut props = source.owner.borrow().props.clone();
        props["selected"] = serde_json::json!(true);
        replace_props(&source.owner, props); drive_owners(window, cx);
        dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        assert_eq!(value(host, cx), 0.0, "source false is the accepted parameter value, not a host-side toggle");
        assert_eq!(host.editor.read(cx).revision().get(), 0, "same-value source proposal cannot write history");
        let source = source(window);
        let mut props = source.owner.borrow().props.clone();
        props["selected"] = serde_json::json!(false);
        replace_props(&source.owner, props); drive_owners(window, cx);
        dispatch_accessibility_action(&source.owner, "activate", None);
        dispatch_accessibility_action(&source.owner, "activate", None);
        drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        assert_eq!(value(host, cx), 1.0);
        assert_eq!(host.editor.read(cx).revision().get(), 1, "duplicate source true is one real parameter transaction");
    }).unwrap();
}

#[gpui::test]
fn device_boolean_switch_rejects_an_old_revision_before_redraw(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |host, window, cx| {
        let source = source(window);
        host.editor.update(cx, |ed, cx| {
            let track = ed.session.track.clone();
            assert!(ed.set_instrument_params(&track, vec![("gain".into(), -7.0)], cx));
        });
        assert_eq!(host.editor.read(cx).revision().get(), 1);
        // Retained native owner has not been replaced by a new frame yet.
        dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        assert_eq!(value(host, cx), 0.0, "an old parameter proposal cannot cross a project revision");
        assert_eq!(host.editor.read(cx).revision().get(), 1, "only the intervening real edit reaches history");
    }).unwrap();
}
