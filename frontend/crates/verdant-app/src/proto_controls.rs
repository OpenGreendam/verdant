//! Native Adwaita drawing around compiler-owned action semantics.
//!
//! Fluent `on_click` installs a domain command, not a second input handler:
//! commands are dispatched only from the compiled root's accepted activation.
use std::{cell::RefCell, rc::Rc};
use gpui::{prelude::*, App, ClickEvent, Context, Div, ElementId, Entity, Focusable,
    IntoElement, Role, SharedString, Stateful, StyleRefinement, Subscription, Window, div};
use serde_json::{Value, json};
use verdant_action_control::{ActionControl};

type Click = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type ContextMenu = Rc<dyn Fn(gpui::Point<gpui::Pixels>, &mut Window, &mut App)>;
type Submit = Rc<dyn Fn(&mut Window, &mut App)>;
type Navigation = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type Assistive = Rc<RefCell<dyn FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut App)>>;

#[derive(IntoElement)]
pub(crate) struct Action {
    shell: Stateful<Div>, label: SharedString, description: SharedString,
    authority: String,
    role: &'static str, selected: bool, selectable: bool, disabled: bool, expanded: Option<bool>,
    clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, submit: Option<Submit>, context: Option<ContextMenu>, position: usize, count: usize,
}

pub(crate) fn action(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Action {
    Action { shell: div().id(id).relative(), label: label.into(), description: "".into(), authority: String::new(),
        role: "button", selected: false, selectable: false, disabled: false, expanded: None,
        clicks: Vec::new(), assistive: None, navigation: None, submit: None, context: None, position: 0, count: 0 }
}

/// Project and selection facts used by commands whose meaning depends on the
/// active editor target. This is data authority; input admission stays authored.
pub(crate) fn editor_authority(editor: &crate::editor::Editor) -> String {
    json!({"project":editor.project().project_id,"generation":editor.document_generation,
        "revision":editor.revision().to_string(),"track":editor.session.track,
        "notes":editor.session.selection,"clip":editor.integrations.selected_clip}).to_string()
}

impl Action {
    /// Immutable domain facts identifying this rendered command. A changed
    /// target/document retires a held press even when its label stays the same.
    pub(crate) fn authority(mut self, authority: impl Into<String>) -> Self { self.authority = authority.into(); self }
    pub(crate) fn on_context_menu(mut self, listener: impl Fn(gpui::Point<gpui::Pixels>, &mut Window, &mut App) + 'static) -> Self { self.context = Some(Rc::new(listener)); self }
    pub(crate) fn on_enter(mut self, listener: impl Fn(&mut Window, &mut App) + 'static) -> Self { self.submit = Some(Rc::new(listener)); self }
    pub(crate) fn navigation(mut self, position: usize, count: usize, listener: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.position = position; self.count = count; self.navigation = Some(Rc::new(listener)); self
    }
    pub(crate) fn on_key_down(mut self, listener: impl Fn(&gpui::KeyDownEvent, &mut Window, &mut App) + 'static) -> Self {
        // Enter/Space always follow the source press protocol, including legacy
        // call sites that previously installed a second activation shortcut.
        self.shell = self.shell.on_key_down(move |event, window, cx| {
            if !matches!(event.keystroke.key.as_str(), "enter" | "space") { listener(event, window, cx); }
        }); self
    }
    pub(crate) fn on_click(mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.clicks.push(Rc::new(listener)); self
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
    pub(crate) fn tab_index(self, _: isize) -> Self { self }
    pub(crate) fn focusable(self) -> Self { self }
}
impl Styled for Action { fn style(&mut self) -> &mut StyleRefinement { self.shell.style() } }
impl ParentElement for Action { fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) { self.shell.extend(elements); } }
impl InteractiveElement for Action { fn interactivity(&mut self) -> &mut gpui::Interactivity { self.shell.interactivity() } }
impl StatefulInteractiveElement for Action {}

struct Binding {
    root: Entity<ActionControl>, clicks: Vec<Click>, assistive: Option<Assistive>, navigation: Option<Navigation>, submit: Option<Submit>, context: Option<ContextMenu>,
    props: Value, authority: String, revision: u64, _changes: Subscription,
}

/// The native compiler represents authored numbers as finite f64 values.
/// Accept integral JSON numbers regardless of serde's integer/float storage.
pub(crate) fn source_integer(value: Option<&Value>) -> Option<i64> {
    let number = value?.as_f64()?;
    (number.is_finite() && number.fract() == 0.0 && number >= i64::MIN as f64 && number < i64::MAX as f64).then_some(number as i64)
}
pub(crate) fn source_index(value: Option<&Value>) -> Option<usize> {
    usize::try_from(source_integer(value)?).ok()
}

fn execute(events: Vec<(String, Value)>, failures: Vec<verdant_action_control::native::gpui_native_owner::NativeFailure>,
    revision: &str, clicks: &[Click], assistive: Option<&Assistive>, navigation: Option<&Navigation>, submit: Option<&Submit>, context: Option<&ContextMenu>,
    fallback: &ClickEvent, window: &mut Window, cx: &mut App) {
    if !failures.is_empty() { eprintln!("compiled action failed: {failures:?}"); return; }
    for (name, payload) in events {
        if payload.get("revision").and_then(Value::as_str) != Some(revision) { continue; }
        if name == "context" {
            if let Some(context) = context {
                let position = if payload.get("keyboard").and_then(Value::as_bool) == Some(true) { match fallback { ClickEvent::Keyboard(event) => event.bounds.center(), _ => fallback.position() } }
                    else { gpui::point(gpui::px(payload.get("x").and_then(Value::as_f64).unwrap_or(0.) as f32), gpui::px(payload.get("y").and_then(Value::as_f64).unwrap_or(0.) as f32)) };
                context(position, window, cx);
            }
        } else if name == "submit" { if let Some(submit) = submit { submit(window, cx); } }
        else if name == "navigate" {
            if let (Some(navigate), Some(index)) = (navigation, source_index(payload.get("index"))) { navigate(index, window, cx); }
        } else if name == "activate" {
            if !clicks.is_empty() { for click in clicks { click(fallback, window, cx); } }
            else if let Some(assistive) = assistive { assistive.borrow_mut()(None, window, cx); }
        }
    }
}
fn fallback(handle: &verdant_action_control::ActionControlHandle) -> ClickEvent {
    let bounds = handle.owner.borrow().native_hitbox.as_ref().map(|hitbox| hitbox.bounds).unwrap_or_default();
    ClickEvent::Keyboard(gpui::KeyboardClickEvent { bounds, ..Default::default() })
}
fn deliver(binding: &Entity<Binding>, event: &ClickEvent, window: &mut Window, cx: &mut App) {
    let (events, failures, revision, clicks, assistive, navigation, submit, context) = binding.update(cx, |binding, cx| {
        let handle = binding.root.read(cx).handle();
        (handle.take_events(), handle.take_failures(), binding.revision.to_string(),
            binding.clicks.clone(), binding.assistive.clone(), binding.navigation.clone(), binding.submit.clone(), binding.context.clone())
    });
    execute(events, failures, &revision, &clicks, assistive.as_ref(), navigation.as_ref(), submit.as_ref(), context.as_ref(), event, window, cx);
}

impl RenderOnce for Action {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key = self.shell.interactivity().element_id.clone().expect("compiled action has a stable id");
        let mut props = json!({"label": self.label.as_ref(), "description": self.description.as_ref(),
            "role": self.role, "disabled": self.disabled, "selected": self.selected,
            "submitOnEnter": self.submit.is_some(), "contextMenu":self.context.is_some(), "selectable": self.selectable, "expanded":self.expanded.unwrap_or(false), "expandable":self.expanded.is_some(), "revision": "0", "position":self.position, "count":self.count});
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let initial_props = props.clone();
        let initial_authority = self.authority.clone();
        let state = window.use_keyed_state(key, cx, move |window, cx| {
            let root = ActionControl::create(initial_props.clone(), initial_parent, window, cx)
                .unwrap_or_else(|error| panic!("compiled action setup: {error:?}"));
            let weak = cx.entity().downgrade();
            let host = window.window_handle();
            let changes = cx.observe(&root, move |binding: &mut Binding, _, cx| {
                let handle = binding.root.read(cx).handle();
                let events = handle.take_events(); let failures = handle.take_failures();
                let fallback = fallback(&handle);
                let revision = binding.revision.to_string();
                // Retain the command belonging to this accepted event. A later
                // paint must not substitute another track/slot's callback.
                let clicks = binding.clicks.clone(); let assistive = binding.assistive.clone(); let navigation = binding.navigation.clone(); let submit = binding.submit.clone(); let context = binding.context.clone();
                let weak = weak.clone();
                cx.defer(move |cx| {
                    let Some(binding) = weak.upgrade() else { return };
                    if binding.read(cx).revision.to_string() != revision { return; }
                    let _ = host.update(cx, |_, window, cx| execute(events, failures, &revision, &clicks,
                        assistive.as_ref(), navigation.as_ref(), submit.as_ref(), context.as_ref(), &fallback, window, cx));
                });
                cx.notify();
            });
            Binding { root, clicks: Vec::new(), assistive: None, navigation: None, submit: None, context: None, props: initial_props, authority: initial_authority,
                revision: 0, _changes: changes }
        });
        let has_context = self.context.is_some();
        let (root, focus, context_revision) = state.update(cx, |binding, cx| {
            binding.clicks = self.clicks; binding.assistive = self.assistive; binding.navigation = self.navigation; binding.submit = self.submit; binding.context = self.context;
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
            (binding.root.clone(), binding.root.read(cx).focus_handle(cx), binding.revision.to_string())
        });
        let clicks = state.clone();
        if has_context {
            let binding = state.clone();
            self.shell = self.shell.on_mouse_down(gpui::MouseButton::Right, move |event, window, cx| {
                let source = binding.read(cx).root.read(cx).handle();
                match source.call("requestContext",vec![(event.position.x.as_f32() as f64).into(),(event.position.y.as_f32() as f64).into(),false.into(),context_revision.clone().into()],window,cx) {
                    Ok(Value::Bool(true)) => { window.prevent_default(); cx.stop_propagation(); },
                    Ok(_) => {}, Err(error) => eprintln!("compiled action context: {error:?}"),
                }
            });
        }
        self.shell.track_focus(&focus)
            .on_click(move |event, window, cx| deliver(&clicks, event, window, cx))
            .child(root)
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
                    .on_click(cx.listener(|host, _, _, cx| { host.activations += 1; host.selected = !host.selected; cx.notify(); })))
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
        window.update(cx,|host,_,cx|{assert_eq!(host.contexts.len(),2);assert_eq!(host.contexts[0],host.contexts[1]);host.disabled=true;cx.notify();}).unwrap();
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
