//! Native Adwaita drawing around compiler-owned action semantics.
//!
//! Fluent `on_click` installs a domain command, not a second input handler:
//! commands are dispatched only from the compiled root's accepted activation.
use std::{cell::RefCell, rc::Rc};
use gpui::{prelude::*, App, ClickEvent, Context, Div, ElementId, Entity, Focusable,
    IntoElement, Role, SharedString, Stateful, StyleRefinement, Subscription, Window, div};
use serde_json::{Value, json};
use verdant_action_control::{ActionControl, native::gpui_native_owner::{NativeFailure, OwnerRef}};

/// An accepted source activation. Portable modifiers remain available for
/// keyboard activation, which GPUI's synthetic `ClickEvent` cannot represent.
/// Physical click metadata is retained only after that same source accepted it.
pub(crate) struct Activation {
    native: Option<ClickEvent>,
    native_context: Option<gpui::MouseDownEvent>,
    position: gpui::Point<gpui::Pixels>,
    modifiers: gpui::Modifiers,
    assistive: bool,
}
impl Activation {
    pub(crate) fn modifiers(&self) -> gpui::Modifiers { self.modifiers }
    pub(crate) fn is_mouse(&self) -> bool { self.native_context.is_some() || matches!(self.native, Some(ClickEvent::Mouse(_))) }
    pub(crate) fn is_assistive(&self) -> bool { self.assistive }
    pub(crate) fn native_event(&self) -> Option<&ClickEvent> { self.native.as_ref() }
    pub(crate) fn position(&self) -> gpui::Point<gpui::Pixels> { self.position }
    fn decode(payload: &Value, native: Option<&ClickEvent>, context: Option<&gpui::MouseDownEvent>, owner: &OwnerRef) -> Option<Self> {
        payload.get("selected")?.as_bool()?;
        payload.get("key")?.as_str()?;
        let origin = payload.get("origin")?.as_str()?;
        if !matches!(origin, "press" | "keyboard" | "assistive" | "programmatic") { return None; }
        Some(Self {
            native: if origin == "press" { native.cloned() } else { None },
            native_context: if origin == "press" { context.cloned() } else { None },
            position: if origin == "press" {
                context.map(|event| event.position).or_else(|| native.map(ClickEvent::position))
            } else { None }.unwrap_or_else(|| owner.borrow().bounds.unwrap_or_default().bottom_left()),
            modifiers: gpui::Modifiers {
                shift: payload.get("shiftKey")?.as_bool()?,
                control: payload.get("ctrlKey")?.as_bool()?,
                platform: payload.get("metaKey")?.as_bool()?,
                alt: payload.get("altKey")?.as_bool()?,
                ..Default::default()
            },
            assistive: origin == "assistive",
        })
    }
}

type Click = Rc<dyn Fn(&Activation, &mut Window, &mut App)>;
type Navigation = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type Assistive = Rc<RefCell<dyn FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App)>>;

#[derive(IntoElement)]
pub(crate) struct Action {
    shell: Stateful<Div>, label: SharedString, description: SharedString,
    role: &'static str, selected: bool, selectable: bool, disabled: bool, tab_stop: bool, expanded: Option<bool>,
    clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, context_menu: Option<Click>, position: usize, count: usize,
}

pub(crate) fn action(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Action {
    Action { shell: div().id(id).relative(), label: label.into(), description: "".into(),
        role: "button", selected: false, selectable: false, disabled: false, tab_stop: true, expanded: None,
        clicks: Vec::new(), assistive: None, navigation: None, context_menu: None, position: 0, count: 0 }
}

impl Action {
    pub(crate) fn navigation(mut self, position: usize, count: usize, listener: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.position = position; self.count = count; self.navigation = Some(Rc::new(listener)); self
    }
    pub(crate) fn on_key_down(mut self, listener: impl Fn(&gpui::KeyDownEvent, &mut Window, &mut App) + 'static) -> Self {
        // Enter/Space always follow the source press protocol, including legacy
        // call sites that previously installed a second activation shortcut.
        self.shell = self.shell.on_key_down(move |event, window, cx| {
            if !window.default_prevented() && !matches!(event.keystroke.key.as_str(), "enter" | "space") { listener(event, window, cx); }
        }); self
    }
    pub(crate) fn on_click(mut self, listener: impl Fn(&Activation, &mut Window, &mut App) + 'static) -> Self {
        self.clicks.push(Rc::new(listener)); self
    }
    pub(crate) fn on_context_menu(mut self, listener: impl Fn(&Activation, &mut Window, &mut App) + 'static) -> Self {
        self.context_menu = Some(Rc::new(listener)); self
    }
    pub(crate) fn on_a11y_action(mut self, action: gpui::AccessibleAction,
        listener: impl FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App) + 'static) -> Self {
        if action == gpui::AccessibleAction::Click { self.assistive = Some(Rc::new(RefCell::new(listener))); }
        else { self.shell = self.shell.on_a11y_action(action, listener); }
        self
    }
    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = match role {
            Role::Switch => "switch", Role::CheckBox => "checkbox", Role::Tab => "tab",
            Role::RadioButton => "radio", Role::ComboBox => "combobox",
            Role::MenuItem => "menuitem", Role::MenuItemCheckBox => "menuitemcheckbox",
            Role::MenuItemRadio => "menuitemradio", Role::ListBoxOption => "option", Role::Row => "row",
            _ => "button",
        }; self
    }
    pub(crate) fn aria_label(mut self, label: impl Into<SharedString>) -> Self { self.label = label.into(); self }
    pub(crate) fn aria_description(mut self, text: impl Into<SharedString>) -> Self { self.description = text.into(); self }
    pub(crate) fn aria_selected(mut self, selected: bool) -> Self { self.selected = selected; self.selectable = true; self }
    pub(crate) fn disabled(mut self, disabled: bool) -> Self { self.disabled = disabled; self }
    pub(crate) fn aria_disabled(self, disabled: bool) -> Self { self.disabled(disabled) }
    pub(crate) fn aria_expanded(mut self, expanded: bool) -> Self { self.expanded = Some(expanded); self }
    // Tab participation and the real focus target belong to asFocusable.
    pub(crate) fn tab_index(mut self, index: isize) -> Self { self.tab_stop = index >= 0; self }
    pub(crate) fn focusable(self) -> Self { self }
}
impl Styled for Action { fn style(&mut self) -> &mut StyleRefinement { self.shell.style() } }
impl ParentElement for Action { fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) { self.shell.extend(elements); } }
impl InteractiveElement for Action { fn interactivity(&mut self) -> &mut gpui::Interactivity { self.shell.interactivity() } }
impl StatefulInteractiveElement for Action {}

struct Binding {
    root: Entity<ActionControl>, clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, context_menu: Option<Click>,
    props: Value, revision: u64, pending: Vec<PendingDelivery>, _changes: Subscription,
}

struct PendingDelivery {
    events: Vec<(String, Value)>, failures: Vec<NativeFailure>, revision: String,
    clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, context_menu: Option<Click>,
    native: Option<ClickEvent>, native_context: Option<gpui::MouseDownEvent>, owner: OwnerRef, epoch: u64,
}

/// The native compiler represents authored numbers as finite f64 values.
/// Accept integral JSON numbers regardless of serde's integer/float storage.
pub(crate) fn source_integer(value: Option<&Value>) -> Option<i64> {
    let number = value?.as_f64()?;
    (number.is_finite() && number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0).then_some(number as i64)
}
pub(crate) fn source_index(value: Option<&Value>) -> Option<usize> {
    usize::try_from(source_integer(value)?).ok()
}

fn live(delivery: &PendingDelivery) -> bool {
    let owner = delivery.owner.borrow();
    owner.present && owner.mounted && !owner.disposed && owner.epoch == delivery.epoch
}
fn execute(delivery: PendingDelivery, window: &mut Window, cx: &mut App) {
    if !delivery.failures.is_empty() { eprintln!("compiled action failed: {:?}", delivery.failures); return; }
    for (name, payload) in &delivery.events {
        if !live(&delivery) { return; }
        if payload.get("revision").and_then(Value::as_str) != Some(delivery.revision.as_str()) { continue; }
        if name == "navigate" {
            if let (Some(navigate), Some(index)) = (delivery.navigation.as_ref(), source_index(payload.get("index"))) { navigate(index, window, cx); }
        } else if name == "activate" || name == "context" {
            let Some(activation) = Activation::decode(payload, delivery.native.as_ref(), delivery.native_context.as_ref(), &delivery.owner) else { continue; };
            if name == "context" {
                if let Some(context) = &delivery.context_menu { context(&activation, window, cx); }
                continue;
            }
            if activation.is_assistive() && let Some(assistive) = &delivery.assistive {
                assistive.borrow_mut()(None, window, cx);
            } else if !delivery.clicks.is_empty() {
                for click in &delivery.clicks {
                    if !live(&delivery) { return; }
                    click(&activation, window, cx);
                }
            } else if let Some(assistive) = &delivery.assistive { assistive.borrow_mut()(None, window, cx); }
        }
    }
}
fn collect(binding: &mut Binding, native: Option<&ClickEvent>, context: Option<&gpui::MouseDownEvent>, cx: &App) {
    let handle = binding.root.read(cx).handle();
    let events = handle.take_events(); let failures = handle.take_failures();
    if !events.is_empty() || !failures.is_empty() {
        let epoch = handle.owner.borrow().epoch;
        binding.pending.push(PendingDelivery {
            events, failures, revision: binding.revision.to_string(), clicks: binding.clicks.clone(),
            assistive: binding.assistive.clone(), navigation: binding.navigation.clone(),
            context_menu: binding.context_menu.clone(), native: native.cloned(), native_context: context.cloned(), owner: handle.owner, epoch,
        });
    }
    // A source notification can run before the shell sees the physical click.
    // Pair that click with its already accepted batch before deferred delivery.
    if let Some(native) = native {
        for delivery in &mut binding.pending {
            if delivery.native.is_none() && delivery.revision == binding.revision.to_string()
                && delivery.events.iter().any(|(name, payload)| name == "activate" && payload.get("origin").and_then(Value::as_str) == Some("press")) {
                delivery.native = Some(native.clone());
            }
        }
    }
    if let Some(context) = context {
        for delivery in &mut binding.pending {
            if delivery.native_context.is_none() && delivery.revision == binding.revision.to_string()
                && delivery.events.iter().any(|(name, payload)| name == "context" && payload.get("origin").and_then(Value::as_str) == Some("press")) {
                delivery.native_context = Some(context.clone());
            }
        }
    }
}
fn flush(binding: &Entity<Binding>, window: &mut Window, cx: &mut App) {
    let pending = binding.update(cx, |binding, _| std::mem::take(&mut binding.pending));
    for delivery in pending {
        if binding.read(cx).revision.to_string() != delivery.revision { continue; }
        execute(delivery, window, cx);
    }
}
fn deliver(binding: &Entity<Binding>, event: &ClickEvent, window: &mut Window, cx: &mut App) {
    binding.update(cx, |binding, cx| collect(binding, Some(event), None, cx));
    flush(binding, window, cx);
}

impl RenderOnce for Action {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key = self.shell.interactivity().element_id.clone().expect("compiled action has a stable id");
        let has_context_menu = self.context_menu.is_some();
        let mut props = json!({"label": self.label.as_ref(), "description": self.description.as_ref(),
            "role": self.role, "disabled": self.disabled, "selected": self.selected, "tabStop":self.tab_stop,
            "contextMenu":has_context_menu,
            "contextOnly":has_context_menu && self.clicks.is_empty() && self.assistive.is_none(),
            "selectable": self.selectable, "expanded":self.expanded.unwrap_or(false), "expandable":self.expanded.is_some(), "revision": "0", "position":self.position, "count":self.count});
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let initial_props = props.clone();
        let state = window.use_keyed_state(key, cx, move |window, cx| {
            let root = ActionControl::create(initial_props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|error| panic!("compiled action setup: {error:?}"));
            let weak = cx.entity().downgrade();
            let host = window.window_handle();
            let changes = cx.observe(&root, move |binding: &mut Binding, _, cx| {
                // Retain the command belonging to this accepted event. A later
                // paint must not substitute another track/slot's callback.
                collect(binding, None, None, cx);
                let weak = weak.clone();
                cx.defer(move |cx| {
                    let Some(binding) = weak.upgrade() else { return };
                    let _ = host.update(cx, |_, window, cx| flush(&binding, window, cx));
                });
                cx.notify();
            });
            Binding { root, clicks: Vec::new(), assistive: None, navigation: None, context_menu: None, props: initial_props,
                revision: 0, pending: Vec::new(), _changes: changes }
        });
        let (root, focus) = state.update(cx, |binding, cx| {
            binding.clicks = self.clicks; binding.assistive = self.assistive; binding.navigation = self.navigation; binding.context_menu = self.context_menu;
            if crate::proto_parent::changed(&binding.root.read(cx).handle().owner, &parent) {
                binding.root.update(cx, |root, cx| root.set_parent(parent, cx)).unwrap_or_else(|error| panic!("compiled action parent: {error:?}"));
            }
            // Changed labels/roles/selection/policy retire pending proposals.
            props["revision"] = binding.revision.to_string().into();
            if props != binding.props {
                binding.revision += 1;
                props["revision"] = binding.revision.to_string().into();
                binding.root.update(cx, |root, cx| root.set_props(props.clone(), window, cx))
                    .unwrap_or_else(|error| panic!("compiled action props: {error:?}"));
                binding.props = props;
            }
            (binding.root.clone(), binding.root.read(cx).focus_handle(cx))
        });
        let clicks = state.clone();
        self.shell.track_focus(&focus)
            .on_click(move |event, window, cx| deliver(&clicks, event, window, cx))
            .when(has_context_menu, |shell| shell.on_mouse_down(gpui::MouseButton::Right, move |event, window, cx| {
                state.update(cx, |binding, cx| collect(binding, None, Some(event), cx));
                flush(&state, window, cx);
            }))
            .child(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext, WindowHandle, px, size};
    use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{all_owners, dispatch_accessibility_action, drive_owners, resolved}};

    struct Harness { selected: bool, disabled: bool, activations: usize, tab: usize, samples: Vec<(bool, bool, bool)>, assistive_commands: usize, contexts: Vec<(bool, bool, gpui::Point<gpui::Pixels>)> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let assistive = cx.entity();
            div().size_full().flex().flex_col().items_start()
                .child(action("native-action", "Native compiled action").role(Role::Switch)
                    .aria_selected(self.selected).disabled(self.disabled).w(px(160.)).h(px(36.)).child("Action")
                    .on_context_menu(cx.listener(|host, event: &Activation, _, cx| {
                        host.contexts.push((event.is_mouse(), event.modifiers().shift, event.position())); cx.notify();
                    }))
                    .on_click(cx.listener(|host, event: &Activation, _, cx| {
                        host.activations += 1; host.selected = !host.selected;
                        host.samples.push((event.modifiers().shift, event.is_mouse(), event.is_assistive()));
                        cx.notify();
                    })))
                .child(div().flex().children((0..3).map(|index| {
                    let entity = cx.entity();
                    action(("native-tab", index), format!("Native tab {index}")).role(Role::Tab)
                        .aria_selected(index == self.tab).w(px(60.)).h(px(30.)).child(index.to_string())
                        .navigation(index, 3, move |index, _, cx| entity.update(cx, |host, cx| { host.tab = index; cx.notify(); }))
                        .on_click(cx.listener(move |host, _, _, cx| { host.tab = index; cx.notify(); }))
                })))
                .child(action("separate-assistive-action", "Separate assistive action").w(px(200.)).h(px(30.)).child("Assistive action")
                    .on_click(cx.listener(|host, _, _, cx| { host.activations += 1; cx.notify(); }))
                    .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| assistive.update(cx, |host, cx| { host.assistive_commands += 1; cx.notify(); })))
        }
    }
    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(300.), px(180.)), |_, _| Harness { selected: false, disabled: false, activations: 0, tab: 0, samples: Vec::new(), assistive_commands: 0, contexts: Vec::new() });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window
    }
    fn handle(name: &str, window: &Window) -> ActionControlHandle {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.declared_events.contains("activate") && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == name)
        }).expect("actual common widget mounts an authored action root");
        ActionControlHandle { owner }
    }

    #[gpui::test]
    fn native_common_action_dispatches_once_for_pointer_keyboard_and_assistive_routes(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |_, window, cx| {
            let source = handle("Native compiled action", window);
            let position = source.owner.borrow().bounds.expect("laid out semantic root").center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.activations, 1); assert!(host.selected);
            let source = handle("Native compiled action", window);
            assert_eq!(resolved(&source.owner.borrow().accessible_states["checked"]).bool(), true);
            source.call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "enter space");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.activations, 3, "each native terminal key must activate exactly once");
            let source = handle("Native compiled action", window);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| { assert_eq!(host.activations, 4); host.disabled = true; cx.notify(); }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, cx| {
            let source = handle("Native compiled action", window);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert_eq!(host.activations, 4)).unwrap();
    }

    #[gpui::test]
    fn native_common_tabs_use_authored_arrow_navigation_and_real_focus(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |_, window, cx| { handle("Native tab 0", window).call("focusSelf", vec![], window, cx).unwrap(); }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "left");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, _| {
            assert_eq!(host.tab, 2);
            assert_eq!(handle("Native tab 2", window).state("focused").unwrap(), json!(true));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "home");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert_eq!(host.tab, 0)).unwrap();
    }

    #[gpui::test]
    fn native_action_preserves_shift_and_physical_origin_and_delivers_the_assistive_route(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |_, window, cx| {
            let source = handle("Native compiled action", window);
            let position = source.owner.borrow().bounds.expect("laid out semantic root").center();
            let modifiers = gpui::Modifiers { shift: true, ..Default::default() };
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, modifiers, button: MouseButton::Left, ..Default::default() }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, modifiers, button: MouseButton::Left, ..Default::default() }), cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.samples, vec![(true, true, false)]);
            handle("Native compiled action", window).call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "shift-enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.samples, vec![(true, true, false), (true, false, false)]);
            let source = handle("Separate assistive action", window);
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| {
            assert_eq!(host.activations, 2, "assistive routing must not call the distinct pointer command");
            assert_eq!(host.assistive_commands, 1);
        }).unwrap();
    }

    #[gpui::test]
    fn native_context_menu_uses_compiled_mouse_and_named_keyboard_routes(cx: &mut TestAppContext) {
        let window = open(cx);
        let position = window.update(cx, |_, window, cx| {
            let source = handle("Native compiled action", window);
            let position = source.owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                position, button: MouseButton::Right, modifiers: gpui::Modifiers { shift: true, ..Default::default() }, ..Default::default()
            }), cx);
            position
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.contexts, vec![(true, true, position)]);
            assert_eq!(host.activations, 0);
            handle("Native compiled action", window).call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "shift-f10 menu");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, _| {
            let keyboard_anchor = handle("Native compiled action", window).owner.borrow().bounds.unwrap().bottom_left();
            assert_eq!(host.contexts, vec![(true, true, position), (false, true, keyboard_anchor), (false, false, keyboard_anchor)]);
            assert_eq!(host.activations, 0);
        }).unwrap();
    }

    struct NestedHarness {
        editor: Entity<crate::editor::Editor>, parent_clicks: usize, child_clicks: usize,
        scalar_taps: usize, scalar_commits: usize, scalar_value: f64, scalar_focus: Option<gpui::FocusHandle>,
    }
    impl Render for NestedHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            use crate::proto_scalar::{scalar, EventKind, Sample, Spec};
            let mut spec = Spec::slider(self.scalar_value, 0., 1., "Nested scalar");
            spec.step = 0.1; spec.tap_action = "activate";
            let owner = cx.entity(); let focus_owner = owner.clone();
            div().size_full().child(action("nested-parent", "Nested parent action")
                .w(px(300.)).h(px(200.)).p(px(12.)).flex().flex_col().items_start().gap(px(12.))
                .on_click(cx.listener(|host, _, _, cx| { host.parent_clicks += 1; cx.notify(); }))
                .child(action("nested-child", "Nested child action").w(px(120.)).h(px(32.)).child("Child button")
                    .on_click(cx.listener(|host, _, _, cx| { host.child_clicks += 1; cx.notify(); })))
                .child(div().w(px(150.)).h(px(40.)).child(scalar(self.editor.clone(), "nested-scalar", spec,
                    |sample, bounds, _| {
                        let value = (sample.position.x / f64::from(bounds.size.width)).clamp(0., 1.);
                        Sample { value, fine: value, jump: false }
                    },
                    |display, _, _, _| div().size_full().child(format!("Scalar {:.1}", display.value)).into_any_element(),
                    move |_, event, _, cx| owner.update(cx, |host, cx| {
                        if event.kind == EventKind::Activate { host.scalar_taps += 1; }
                        if event.kind == EventKind::Commit { host.scalar_commits += 1; host.scalar_value = event.value; }
                        cx.notify();
                    }))
                    .on_focus_handle(move |focus, _, cx| focus_owner.update(cx, |host, _| host.scalar_focus = Some(focus.clone())))))
                .child("Parent background"))
        }
    }

    #[gpui::test]
    fn native_nested_actions_and_scalar_keep_independent_pointer_focus_and_accessibility(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(340.), px(240.)), |window, cx| {
            let editor = cx.new(|cx| crate::editor::Editor::for_native_test(verdant_domain::Project::new_empty(), window, cx));
            NestedHarness { editor, parent_clicks:0, child_clicks:0, scalar_taps:0, scalar_commits:0, scalar_value:0.5, scalar_focus:None }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        let click = |name: &str, background: bool, window: &mut Window, cx: &mut App| {
            let bounds = handle(name, window).owner.borrow().bounds.expect("mounted nested source");
            let position = if background { bounds.bottom_right() - gpui::point(px(4.), px(4.)) } else { bounds.center() };
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button:MouseButton::Left, ..Default::default() }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button:MouseButton::Left, ..Default::default() }), cx);
        };
        window.update(cx, |_, window, cx| click("Nested child action", false, window, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| {
            assert_eq!(host.child_clicks, 1, "foreground child must receive the physical click through its own source");
            assert_eq!(host.parent_clicks, 0, "child activation must not run the background source command");
        }).unwrap();
        window.update(cx, |_, window, cx| click("Nested scalar", false, window, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| { assert_eq!(host.scalar_taps, 1); assert_eq!(host.parent_clicks, 0); }).unwrap();
        window.update(cx, |_, window, cx| click("Nested parent action", true, window, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.parent_clicks, 1); assert_eq!(host.child_clicks, 1); assert_eq!(host.scalar_taps, 1);
            let parent = handle("Nested parent action", window); let child = handle("Nested child action", window); let scalar = handle("Nested scalar", window);
            assert_ne!(parent.owner.borrow().modules.a11y_node, child.owner.borrow().modules.a11y_node);
            assert_ne!(child.owner.borrow().modules.a11y_node, scalar.owner.borrow().modules.a11y_node);
            assert_eq!(resolved(&scalar.owner.borrow().accessible_role).text(), "slider");
            child.call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.child_clicks, 2); assert_eq!(host.parent_clicks, 1);
            assert_eq!(handle("Nested child action", window).state("focused").unwrap(), Value::Bool(true));
            assert_eq!(handle("Nested parent action", window).state("focused").unwrap(), Value::Bool(false));
            let focus = host.scalar_focus.as_ref().expect("compiled scalar focus").clone(); window.focus(&focus, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |_, window, cx| {
            let scalar = handle("Nested scalar", window);
            assert_eq!(scalar.state("focused").unwrap(), Value::Bool(true));
            assert_eq!(handle("Nested child action", window).state("focused").unwrap(), Value::Bool(false));
            dispatch_accessibility_action(&scalar.owner, "increment", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.scalar_commits, 1); assert!((host.scalar_value-0.6).abs()<1e-10);
            assert_eq!(host.parent_clicks, 1); assert_eq!(host.child_clicks, 2);
            dispatch_accessibility_action(&handle("Nested child action", window).owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| { assert_eq!(host.child_clicks, 3); assert_eq!(host.parent_clicks, 1); assert_eq!(host.scalar_commits, 1); }).unwrap();
    }
}
