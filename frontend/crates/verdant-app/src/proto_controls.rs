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
    selected: bool,
    native: Option<ClickEvent>,
    native_context: Option<gpui::MouseDownEvent>,
    position: gpui::Point<gpui::Pixels>,
    modifiers: gpui::Modifiers,
    assistive: bool,
}
impl Activation {
    /// The source's controlled selection proposal, accepted only after the
    /// consumer has checked its document/target authority.
    pub(crate) fn selected(&self) -> bool { self.selected }
    pub(crate) fn modifiers(&self) -> gpui::Modifiers { self.modifiers }
    pub(crate) fn is_mouse(&self) -> bool { self.native_context.is_some() || matches!(self.native, Some(ClickEvent::Mouse(_))) }
    pub(crate) fn is_assistive(&self) -> bool { self.assistive }
    pub(crate) fn native_event(&self) -> Option<&ClickEvent> { self.native.as_ref() }
    pub(crate) fn position(&self) -> gpui::Point<gpui::Pixels> { self.position }
    fn decode(payload: &Value, native: Option<&ClickEvent>, context: Option<&gpui::MouseDownEvent>, owner: &OwnerRef) -> Option<Self> {
        let selected = payload.get("selected")?.as_bool()?;
        payload.get("key")?.as_str()?;
        let origin = payload.get("origin")?.as_str()?;
        if !matches!(origin, "press" | "keyboard" | "assistive" | "programmatic") { return None; }
        Some(Self {
            selected,
            native: if origin == "press" { native.cloned() } else { None },
            native_context: if origin == "press" { context.cloned() } else { None },
            position: if origin == "press" {
                context.map(|event| event.position).or_else(|| native.map(ClickEvent::position)).or_else(|| {
                    if payload.get("keyboard").and_then(Value::as_bool) != Some(false) { return None; }
                    let x = payload.get("x")?.as_f64()?; let y = payload.get("y")?.as_f64()?;
                    (x.is_finite() && y.is_finite()).then(|| gpui::point(gpui::px(x as f32), gpui::px(y as f32)))
                })
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
type Submit = Rc<dyn Fn(&mut Window, &mut App)>;
type Navigation = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type Assistive = Rc<RefCell<dyn FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App)>>;

thread_local! {
    static ACTIVATION_ORIGIN: RefCell<Option<(gpui::AnyWindowHandle, OwnerRef)>> = const { RefCell::new(None) };
}

/// Geometry follows the accepted Action, including keyboard and assistive
/// commands whose application callback opens a menu indirectly.
pub(crate) fn activation_origin(window: &Window) -> Option<OwnerRef> {
    ACTIVATION_ORIGIN.with(|origin| origin.borrow().as_ref()
        .filter(|(host, _)| *host == window.window_handle()).map(|(_, owner)| owner.clone()))
}

struct ActivationOrigin(Option<(gpui::AnyWindowHandle, OwnerRef)>);
impl ActivationOrigin {
    fn enter(window: &Window, owner: OwnerRef) -> Self {
        Self(ACTIVATION_ORIGIN.with(|origin| origin.replace(Some((window.window_handle(), owner)))))
    }
}
impl Drop for ActivationOrigin {
    fn drop(&mut self) { ACTIVATION_ORIGIN.with(|origin| { origin.replace(self.0.take()); }); }
}

#[derive(IntoElement)]
pub(crate) struct Action {
    shell: Stateful<Div>, children: Vec<gpui::AnyElement>, label: SharedString, description: SharedString,
    authority: String,
    role: &'static str, selected: bool, selectable: bool, disabled: bool, tab_stop: bool, expanded: Option<bool>,
    clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, context_menu: Option<Click>, submit: Option<Submit>, position: usize, count: usize,
    /// Pressed styling. The compiled root claims the physical press, so GPUI
    /// never marks the shell active; the root's exposed `pressed` state does.
    pressed_style: Option<StyleRefinement>,
}

pub(crate) fn action(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Action {
    Action { shell: div().id(id).relative(), children: Vec::new(), label: label.into(), description: "".into(), authority: String::new(),
        role: "button", selected: false, selectable: false, disabled: false, tab_stop: true, expanded: None,
        clicks: Vec::new(), assistive: None, navigation: None, context_menu: None, submit: None, position: 0, count: 0, pressed_style: None }
}

/// Project and selection facts used by commands whose meaning depends on the
/// active editor target. This is data authority; input admission stays authored.
pub(crate) fn editor_authority(editor: &crate::editor::Editor) -> String {
    json!({"project":editor.project().project_id,"generation":editor.document_generation,
        "revision":editor.revision().to_string(),"track":editor.session.track,
        "notes":editor.session.selection,"clip":editor.integrations.selected_clip}).to_string()
}

impl Action {
    pub(crate) fn authority(mut self, authority: impl Into<String>) -> Self { self.authority = authority.into(); self }
    pub(crate) fn on_enter(mut self, listener: impl Fn(&mut Window, &mut App) + 'static) -> Self { self.submit = Some(Rc::new(listener)); self }
    pub(crate) fn on_context_menu(self, listener: impl Fn(gpui::Point<gpui::Pixels>, &mut Window, &mut App) + 'static) -> Self {
        self.on_context_activation(move |event, window, cx| listener(event.position(), window, cx))
    }
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
    pub(crate) fn on_context_activation(mut self, listener: impl Fn(&Activation, &mut Window, &mut App) + 'static) -> Self {
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
impl ParentElement for Action { fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) { self.children.extend(elements); } }
impl InteractiveElement for Action { fn interactivity(&mut self) -> &mut gpui::Interactivity { self.shell.interactivity() } }
impl StatefulInteractiveElement for Action {
    /// Applied while the compiled root reports `pressed`, not on GPUI's own
    /// mouse-down bookkeeping, which never reaches the shell.
    fn active(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self where Self: Sized {
        self.pressed_style = Some(f(StyleRefinement::default())); self
    }
}

struct Binding {
    root: Entity<ActionControl>, clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, context_menu: Option<Click>,
    props: Value, authority: String, submit: Option<Submit>, revision: u64, pending: Vec<PendingDelivery>, _changes: Subscription,
    /// Pressed layer drawn under the label while the compiled root reports
    /// `pressed`. It lives here because this entity re-renders on every root
    /// change; the parent view that built the shell may be cached.
    pressed: Option<StyleRefinement>,
}

impl Render for Binding {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let down = self.pressed.is_some()
            && self.root.read(cx).handle().state("pressed").ok().and_then(|value| value.as_bool()).unwrap_or(false);
        let mut layer = div().absolute().top_0().left_0().size_full();
        if let (true, Some(pressed)) = (down, &self.pressed) { layer.style().refine(pressed); }
        div().absolute().top_0().left_0().size_full().child(layer).child(self.root.clone())
    }
}

struct PendingDelivery {
    events: Vec<(String, Value)>, failures: Vec<NativeFailure>, revision: String, submit: Option<Submit>,
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
        && owner.props.get("revision").and_then(Value::as_str) == Some(delivery.revision.as_str())
        && owner.props.get("disabled").and_then(Value::as_bool) != Some(true)
}
fn execute(delivery: PendingDelivery, window: &mut Window, cx: &mut App) {
    if !delivery.failures.is_empty() { eprintln!("compiled action failed: {:?}", delivery.failures); return; }
    for (name, payload) in &delivery.events {
        if !live(&delivery) { return; }
        if payload.get("revision").and_then(Value::as_str) != Some(delivery.revision.as_str()) { continue; }
        if name == "submit" { if let Some(submit) = &delivery.submit { submit(window, cx); } }
        else if name == "navigate" {
            if let (Some(navigate), Some(index)) = (delivery.navigation.as_ref(), source_index(payload.get("index"))) { navigate(index, window, cx); }
        } else if name == "activate" || name == "context" {
            let Some(activation) = Activation::decode(payload, delivery.native.as_ref(), delivery.native_context.as_ref(), &delivery.owner) else { continue; };
            if name == "context" {
                if let Some(context) = &delivery.context_menu { context(&activation, window, cx); }
                continue;
            }
            let _origin = ActivationOrigin::enter(window, delivery.owner.clone());
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
            events, failures, revision: binding.revision.to_string(), submit: binding.submit.clone(), clicks: binding.clicks.clone(),
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
            "contextMenu":has_context_menu, "submitOnEnter":self.submit.is_some(),
            "contextOnly":has_context_menu && self.clicks.is_empty() && self.assistive.is_none(),
            "selectable": self.selectable, "expanded":self.expanded.unwrap_or(false), "expandable":self.expanded.is_some(), "revision": "0", "position":self.position, "count":self.count});
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let initial_props = props.clone();
        let initial_authority = self.authority.clone();
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
                    // A press or release changes the shell's pressed styling,
                    // which lives in the parent view's render.
                    let _ = host.update(cx, |_, window, cx| { flush(&binding, window, cx); window.refresh(); });
                });
                cx.notify();
            });
            Binding { root, clicks: Vec::new(), assistive: None, navigation: None, context_menu: None, submit: None, authority: initial_authority, props: initial_props,
                revision: 0, pending: Vec::new(), _changes: changes, pressed: None }
        });
        let root = state.update(cx, |binding, cx| {
            binding.clicks = self.clicks; binding.assistive = self.assistive; binding.navigation = self.navigation; binding.context_menu = self.context_menu; binding.submit = self.submit;
            if crate::proto_parent::changed(&binding.root.read(cx).handle().owner, &parent) {
                binding.root.update(cx, |root, cx| root.set_parent(parent, cx)).unwrap_or_else(|error| panic!("compiled action parent: {error:?}"));
            }
            // Changed semantic policy or domain authority retires a held press
            // and callbacks queued by a previous render of this stable key.
            // Live accessible labels are presentation, not a new command.
            props["revision"] = binding.revision.to_string().into();
            if props != binding.props || binding.authority != self.authority {
                let mut policy=props.clone();let mut previous=binding.props.clone();
                for property in ["label","description"] {
                    policy.as_object_mut().expect("action props").remove(property);
                    previous.as_object_mut().expect("action props").remove(property);
                }
                if policy!=previous || binding.authority!=self.authority {binding.revision+=1;}
                props["revision"] = binding.revision.to_string().into();
                binding.root.update(cx, |root, cx| root.set_props(props.clone(), window, cx))
                    .unwrap_or_else(|error| panic!("compiled action props: {error:?}"));
                binding.props = props;
                binding.authority = self.authority;
            }
            binding.root.clone()
        });
        let clicks = state.clone();
        let mut pressed = self.pressed_style.take();
        if let Some(pressed) = pressed.as_mut() {
            // The layer shares the shell's corners so it fills the same shape.
            pressed.corner_radii = self.shell.style().corner_radii.clone();
        }
        state.update(cx, |binding, cx| { if binding.pressed != pressed { binding.pressed = pressed; cx.notify(); } });
        let _ = &root;
        let layer_view = state.clone();
        // Only the generated root registers its focus handle. Registering it
        // again on the shell duplicates Tab stops and bypasses disabled focus.
        self.shell
            .on_click(move |event, window, cx| deliver(&clicks, event, window, cx))
            .when(has_context_menu, |shell| shell.on_mouse_down(gpui::MouseButton::Right, move |event, window, cx| {
                state.update(cx, |binding, cx| collect(binding, None, Some(event), cx));
                flush(&state, window, cx);
            }))
            // This source owns the background rectangle. Foreground controls
            // stay above it in native hit testing, so their own compiled root
            // claims the physical sample before this background can see it.
            .child(layer_view)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext, WindowHandle, px, size};
    use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{all_owners, dispatch_accessibility_action, drive_owners, resolved}};

    struct Harness { selected: bool, disabled: bool, activations: usize, tab: usize }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().flex().flex_col().items_start()
                .child(action("native-action", "Native compiled action").role(Role::Switch)
                    .aria_selected(self.selected).disabled(self.disabled).w(px(160.)).h(px(36.)).child("Action")
                    .on_click(cx.listener(|host, event: &Activation, _, cx| { host.activations += 1; host.selected = event.selected(); cx.notify(); })))
                .child(div().flex().children((0..3).map(|index| {
                    let entity = cx.entity();
                    action(("native-tab", index), format!("Native tab {index}")).role(Role::Tab)
                        .aria_selected(index == self.tab).w(px(60.)).h(px(30.)).child(index.to_string())
                        .navigation(index, 3, move |index, _, cx| entity.update(cx, |host, cx| { host.tab = index; cx.notify(); }))
                        .on_click(cx.listener(move |host, _, _, cx| { host.tab = index; cx.notify(); }))
                })))
        }
    }
    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(300.), px(180.)), |_, _| Harness { selected: false, disabled: false, activations: 0, tab: 0 });
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
        // A production native event can synchronously update the root entity;
        // event transport must not hold the fixture's root-entity borrow.
        cx.update_window(window.into(), |_, window, cx| {
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
        for key in ["enter", "space"] {
            cx.simulate_keystrokes(window.into(), key);
            cx.update_window(window.into(), |_, window, cx| {
                window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                    keystroke: gpui::Keystroke::parse(key).unwrap(),
                }), cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
        }
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
    struct ContextHarness { contexts:Vec<gpui::Point<gpui::Pixels>>, primary:usize, disabled:bool }
    impl Render for ContextHarness {
        fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
            let host=cx.entity();
            action("context-action","Context action").w(px(180.)).h(px(40.)).disabled(self.disabled).child("Menu")
                .on_context_menu(move|position,_,cx|host.update(cx,|host,cx|{host.contexts.push(position);cx.notify();}))
                .on_click(cx.listener(|host,_,_,cx|{host.primary+=1;cx.notify();}))
        }
    }
    #[gpui::test]
    fn native_common_action_context_shares_pointer_keyboard_and_disabled_admission(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(240.),px(120.)),|_,_|ContextHarness{contexts:Vec::new(),primary:0,disabled:false});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        cx.update_window(window.into(),|_,window,cx| {
            let position=handle("Context action",window).owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent{position,button:MouseButton::Right,..Default::default()}),cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent{position,button:MouseButton::Right,..Default::default()}),cx);
        }).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx|{assert_eq!(host.contexts.len(),1);assert_eq!(host.primary,0);handle("Context action",window).call("focusSelf",vec![],window,cx).unwrap();}).unwrap();
        cx.simulate_keystrokes(window.into(),"shift-f10");crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx| {
            let bounds=handle("Context action",window).owner.borrow().bounds.unwrap();
            assert_eq!(host.contexts.len(),2);
            assert_eq!(host.contexts,vec![bounds.center(),bounds.bottom_left()], "pointer uses its actual sample; keyboard uses the live menu edge");
            host.disabled=true;cx.notify();
        }).unwrap();
        crate::native_test_support::frames(window,cx);
        cx.update_window(window.into(),|_,window,cx| {let position=handle("Context action",window).owner.borrow().bounds.unwrap().center();window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent{position,button:MouseButton::Right,..Default::default()}),cx);}).unwrap();
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_|assert_eq!(host.contexts.len(),2)).unwrap();
    }

    struct AuthorityHarness { target: usize, calls: Vec<usize> }
    impl Render for AuthorityHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let target = self.target;
            action("stable-command-key", "Remove").authority(target.to_string())
                .w(px(150.)).h(px(40.)).child("Remove")
                .on_click(cx.listener(move |host, _, _, cx| { host.calls.push(target); cx.notify(); }))
        }
    }
    #[gpui::test]
    fn native_common_action_retires_a_held_press_when_its_same_label_target_changes(cx: &mut TestAppContext) {
        let window = cx.open_window(size(px(240.), px(120.)), |_, _| AuthorityHarness { target: 0, calls: Vec::new() });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, cx| {
            let source = handle("Remove", window);
            let position = source.owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
            assert_eq!(source.state("pressed").unwrap(), json!(true));
        }).unwrap();
        window.update(cx, |host, _, cx| { host.target = 1; cx.notify(); }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, cx| {
            let source = handle("Remove", window);
            assert_eq!(source.state("pressed").unwrap(), json!(false));
            let position = source.owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert!(host.calls.is_empty(), "release cannot activate the replacement command")).unwrap();
        cx.update_window(window.into(), |_, window, cx| {
            let position = handle("Remove", window).owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| assert_eq!(host.calls, vec![1], "a fresh press uses the new target once")).unwrap();
    }

    struct LiveTextHarness { tick:usize, activations:usize }
    impl Render for LiveTextHarness {
        fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
            action("live-position-action",format!("Playback position {}",self.tick)).authority("same transport menu")
                .w(px(180.)).h(px(40.)).child(format!("Position {}",self.tick))
                .on_click(cx.listener(|host,_,_,cx|{host.activations+=1;cx.notify();}))
        }
    }
    #[gpui::test]
    fn native_common_action_live_readout_does_not_retire_the_same_held_command(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(240.),px(120.)),|_,_|LiveTextHarness{tick:0,activations:0});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        cx.update_window(window.into(),|_,window,cx| {
            let source=handle("Playback position 0",window);let position=source.owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent{position,button:MouseButton::Left,..Default::default()}),cx);
            assert_eq!(source.state("pressed").unwrap(),json!(true));
        }).unwrap();
        window.update(cx,|host,_,cx|{host.tick=1;cx.notify();}).unwrap();crate::native_test_support::frames(window,cx);
        cx.update_window(window.into(),|_,window,cx| {
            let source=handle("Playback position 1",window);
            assert_eq!(source.state("pressed").unwrap(),json!(true),"a readout repaint preserves accepted command authority");
            let position=source.owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent{position,button:MouseButton::Left,..Default::default()}),cx);
        }).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_|assert_eq!(host.activations,1)).unwrap();
    }
}

#[cfg(test)]
mod integrated_route_tests {
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
                    .on_context_activation(cx.listener(|host, event: &Activation, _, cx| {
                        host.contexts.push((event.is_mouse(), event.modifiers().shift, event.position())); cx.notify();
                    }))
                    .on_click(cx.listener(|host, event: &Activation, _, cx| {
                        host.activations += 1; host.selected = event.selected();
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
    fn native_action_has_one_reverse_tab_stop_and_disabled_click_does_not_steal_focus(cx: &mut TestAppContext) {
        let window = open(cx);
        cx.update_window(window.into(), |_, window, cx| {
            handle("Separate assistive action", window).call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "shift-tab");
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, _| {
            assert_eq!(handle("Native tab 0", window).state("focused").unwrap(), Value::Bool(true),
                "Shift+Tab must leave the action instead of finding its duplicate shell handle");
            assert_eq!(handle("Separate assistive action", window).state("focused").unwrap(), Value::Bool(false));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "shift-tab");
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, _| {
            assert_eq!(handle("Native compiled action", window).state("focused").unwrap(), Value::Bool(true));
        }).unwrap();
        window.update(cx, |host, _, cx| { host.disabled = true; cx.notify(); }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, cx| {
            handle("Separate assistive action", window).call("focusSelf", vec![], window, cx).unwrap();
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.update_window(window.into(), |_, window, cx| {
            let position = handle("Native compiled action", window).owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position, button: MouseButton::Left, ..Default::default() }), cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, _| {
            assert_eq!(handle("Native compiled action", window).state("focused").unwrap(), Value::Bool(false));
            assert_eq!(handle("Separate assistive action", window).state("focused").unwrap(), Value::Bool(true));
            assert_eq!(host.activations, 0);
        }).unwrap();
    }



    #[gpui::test]
    fn native_action_preserves_shift_and_physical_origin_and_delivers_the_assistive_route(cx: &mut TestAppContext) {
        let window = open(cx);
        cx.update_window(window.into(), |_, window, cx| {
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
        window.update(cx, |host, _, _| {
            assert_eq!(host.samples, vec![(true, true, false), (true, false, false)]);
        }).unwrap();
        cx.update_window(window.into(), |_, window, cx| {
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
        let position = cx.update_window(window.into(), |_, window, cx| {
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
        cx.update_window(window.into(), |_, window, cx| click("Nested child action", false, window, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| {
            assert_eq!(host.child_clicks, 1, "foreground child must receive the physical click through its own source");
            assert_eq!(host.parent_clicks, 0, "child activation must not run the background source command");
        }).unwrap();
        cx.update_window(window.into(), |_, window, cx| click("Nested scalar", false, window, cx)).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| { assert_eq!(host.scalar_taps, 1); assert_eq!(host.parent_clicks, 0); }).unwrap();
        cx.update_window(window.into(), |_, window, cx| click("Nested parent action", true, window, cx)).unwrap();
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
        cx.update_window(window.into(), |_, window, cx| {
            let scalar = handle("Nested scalar", window);
            assert_eq!(scalar.state("focused").unwrap(), Value::Bool(true));
            assert_eq!(handle("Nested child action", window).state("focused").unwrap(), Value::Bool(false));
            dispatch_accessibility_action(&scalar.owner, "increment", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| {
            assert_eq!(host.scalar_commits, 1); assert!((host.scalar_value-0.6).abs()<1e-10);
            assert_eq!(host.parent_clicks, 1); assert_eq!(host.child_clicks, 2);
        }).unwrap();
        cx.update_window(window.into(), |_, window, cx| {
            dispatch_accessibility_action(&handle("Nested child action", window).owner, "activate", None); drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, _| { assert_eq!(host.child_clicks, 3); assert_eq!(host.parent_clicks, 1); assert_eq!(host.scalar_commits, 1); }).unwrap();
    }
}

#[cfg(test)]
mod submit_native_tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, px, size};
    use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{all_owners,resolved}};
    struct Harness { submissions:usize, activations:usize, selected:bool, submit_enabled:bool }
    impl Render for Harness {
        fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
            let entity=cx.entity();
            div().size_full().child(action("submit-control","Submit choice").role(Role::Switch)
                .aria_selected(self.selected).w(px(160.)).h(px(40.)).child("Choice")
                .on_click(cx.listener(|host,_,_,cx| {host.activations+=1;host.selected=!host.selected;cx.notify();}))
                .when(self.submit_enabled,move|action|action.on_enter(move|_,cx|entity.update(cx,|host,cx| {host.submissions+=1;cx.notify();}))))
        }
    }
    fn source(window:&Window)->ActionControlHandle {
        let owner=all_owners().into_iter().find(|owner| {let owner=owner.borrow();owner.mounted && owner.window_id==Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|name|resolved(name).text()=="Submit choice")}).unwrap();
        ActionControlHandle{owner}
    }
    fn release_key(window:gpui::WindowHandle<Harness>,cx:&mut TestAppContext,key:&'static str) {
        // GPUI simulate_keystrokes dispatches KeyDown/text only. Every physical
        // release retires its keyboard lease, including a submitted Enter.
        cx.update_window(window.into(),|_,window,cx| {
            window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke:gpui::Keystroke::parse(key).unwrap(),
            }),cx);
        }).unwrap();
    }
    #[gpui::test]
    fn native_enter_default_response_does_not_activate_the_choice_when_the_dialog_remains_open(cx:&mut TestAppContext) {
        let window=cx.open_window(size(px(280.),px(100.)),|_,_|Harness{submissions:0,activations:0,selected:false,submit_enabled:true});
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,cx| {source(window).call("focusSelf",vec![],window,cx).unwrap();}).unwrap();
        cx.simulate_keystrokes(window.into(),"enter");crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_| {assert_eq!((host.submissions,host.activations),(1,0));assert!(!host.selected);}).unwrap();
        release_key(window,cx,"enter");crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_|assert_eq!((host.submissions,host.activations),(1,0))).unwrap();
        cx.simulate_keystrokes(window.into(),"space");crate::native_test_support::frames(window,cx);
        release_key(window,cx,"space");crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,_| {assert_eq!((host.submissions,host.activations),(1,1));assert!(host.selected);}).unwrap();
        cx.update_window(window.into(),|_,window,cx| {
            let position=source(window).owner.borrow().bounds.unwrap().center();
            window.dispatch_event(gpui::PlatformInput::MouseDown(gpui::MouseDownEvent{position,button:gpui::MouseButton::Left,..Default::default()}),cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(gpui::MouseUpEvent{position,button:gpui::MouseButton::Left,..Default::default()}),cx);
        }).unwrap();crate::native_test_support::frames(window,cx);
        window.update(cx,|host,_,cx| {assert_eq!((host.submissions,host.activations),(1,2));host.submit_enabled=false;cx.notify();}).unwrap();
        crate::native_test_support::frames(window,cx);
        window.update(cx,|_,window,cx| {source(window).call("focusSelf",vec![],window,cx).unwrap();}).unwrap();
        cx.simulate_keystrokes(window.into(),"enter");release_key(window,cx,"enter");crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,_| {assert_eq!((host.submissions,host.activations),(1,3));assert!(source(window).take_failures().is_empty());}).unwrap();
    }
}
