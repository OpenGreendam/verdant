//! Native hosting for the compiled TextField prototype. Proto-UI owns the
//! editor, IME lease, cursor/selection, clipboard, text undo, focus and textbox
//! semantics. This boundary accepts draft text and routes domain requests.

use std::{cell::RefCell, ops::Range, rc::Rc};
use gpui::{App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, Render, RenderOnce, SharedString, Subscription, Window, div, prelude::*, px};
use serde::Deserialize;
use serde_json::{Value, json};
use verdant_text_field::{TextField as CompiledTextField, TextFieldHandle,
    native::gpui_native_owner::text_select_utf16};
use crate::theme::Theme;

#[derive(Clone, Debug)]
pub(crate) enum Event {
    Changed { value: String, composing: bool },
    Replaced { value: String },
    CompositionEnd { value: String },
    Submit { value: String },
    Cancel,
    Advance { value: String, forward: bool },
    Blur { value: String },
    Failure(String),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Options {
    pub auto_focus: bool,
    pub select_all: bool,
    pub submit_on_enter: bool,
    pub cancel_on_escape: bool,
    pub advance_on_tab: bool,
    pub commit_on_blur: bool,
    pub inline: bool,
    pub numeric: Option<Numeric>,
}
#[derive(Clone, Debug)]
pub(crate) struct Numeric { pub min: f64, pub max: f64, pub step: f64, pub default: f64 }

type Callback = Rc<dyn Fn(Event, &mut Window, &mut App)>;

pub(crate) struct Field {
    root: Entity<CompiledTextField>,
    handle: TextFieldHandle,
    pub focus: FocusHandle,
    value: String,
    theme: Theme,
    options: Options,
    pending_reset: bool,
    pending_selection: Option<Range<usize>>,
    callback: Option<Callback>,
    _changes: Subscription,
}
impl EventEmitter<Event> for Field {}

impl Field {
    pub fn new(label: &str, value: &str, theme: Theme, window: &mut Window, cx: &mut App) -> Entity<Self> {
        Self::with_options(label, value, theme, Options::default(), window, cx)
    }
    pub fn with_options(label: &str, value: &str, theme: Theme, options: Options, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::create(label, value, theme, options, window, cx))
    }
    fn create(label: &str, value: &str, theme: Theme, options: Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let props = json!({
            "defaultValue": value, "ariaLabel": label, "placeholder": label,
            "autoFocus": options.auto_focus, "selectAllOnMount": options.select_all,
            "submitOnEnter": options.submit_on_enter, "cancelOnEscape": options.cancel_on_escape,
            "advanceOnTab": options.advance_on_tab, "commitOnBlur": options.commit_on_blur,
            "numeric": options.numeric.is_some(),
            "numericMin": options.numeric.as_ref().map(|n| n.min).unwrap_or(0.),
            "numericMax": options.numeric.as_ref().map(|n| n.max).unwrap_or(1.),
            "numericStep": options.numeric.as_ref().map(|n| n.step).unwrap_or(1.),
            "numericDefault": options.numeric.as_ref().map(|n| n.default).unwrap_or(0.),
            "inputMode": if options.numeric.is_some() { "numeric" } else { "text" },
        });
        let root = CompiledTextField::create(props, crate::proto_parent::current(), window, cx)
            .unwrap_or_else(|error| panic!("compiled text field setup failed: {error:?}"));
        let focus = root.read(cx).focus_handle(cx);
        let handle = root.read(cx).handle();
        let changes = cx.observe_in(&root, window, |field, _, window, cx| {
            field.flush_reset(window, cx);
            field.drain(window, cx);
            cx.notify();
        });
        Self { root, handle, focus, value: value.into(), theme, options,
            pending_reset: false, pending_selection: None, callback: None, _changes: changes }
    }
    pub fn value(&self) -> &str { &self.value }
    pub fn is_composing(&self) -> bool {
        !self.pending_reset && self.handle.state("composing").ok().and_then(|value| value.as_bool()).unwrap_or(false)
    }
    pub fn set(&mut self, value: &str, cx: &mut Context<Self>) {
        self.value = value.into();
        self.pending_reset = true;
        self.pending_selection = Some(0..value.encode_utf16().count());
        cx.notify();
    }
    pub fn set_selection(&mut self, selection: Range<usize>, cx: &mut Context<Self>) {
        self.pending_selection = Some(selection);
        cx.notify();
    }
    pub fn invoke(&mut self, method: &str, arguments: Vec<Value>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.flush_reset(window, cx);
        let result = match self.handle.call(method, arguments, window, cx) {
            Ok(value) => value.as_bool().unwrap_or(true),
            Err(error) => { self.publish(Event::Failure(format!("文本控件操作失败：{error:?}")), window, cx); false }
        };
        self.drain(window, cx);
        result
    }
    fn epoch(&self) -> Option<u64> { natural(&self.handle.state("viewEpoch").ok()?) }
    fn publish(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(callback) = self.callback.clone() { callback(event.clone(), window, cx); }
        cx.emit(event);
    }
    fn flush_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_reset {
            // An explicit replacement retires undelivered events from the old
            // draft before the source resets its native TextControl lease.
            self.handle.take_events();
            match self.handle.call("resetValue", vec![json!(self.value)], window, cx) {
                Ok(value) if value == json!(true) => self.pending_reset = false,
                Ok(_) => return, // The first render may precede native mount.
                Err(error) => { self.publish(Event::Failure(format!("文本替换失败：{error:?}")), window, cx); return; }
            }
        }
        if let Some(range) = self.pending_selection.clone() {
            if self.handle.call("selectRange", vec![json!(range.start), json!(range.end)], window, cx).ok() == Some(json!(true)) {
                self.pending_selection = None;
            }
        }
    }
    fn drain(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_reset { self.handle.take_events(); return; }
        let events = self.handle.take_events();
        let failures = self.handle.take_failures();
        if !failures.is_empty() {
            self.publish(Event::Failure(format!("文本控件未接受操作：{failures:?}")), window, cx);
            return;
        }
        for (name, detail) in events {
            let event = match decode(&name, detail) {
                Ok(event) => event,
                Err(error) => { self.publish(Event::Failure(error), window, cx); continue; }
            };
            match event {
                Output::Input(input) => {
                    self.value = input.value.clone();
                    self.publish(Event::Changed { value: input.value, composing: input.composing }, window, cx);
                }
                Output::CompositionEnd(value) => {
                    self.value = value.clone();
                    self.publish(Event::CompositionEnd { value }, window, cx);
                }
                Output::Composition(value) => {
                    self.value = value.clone();
                    self.publish(Event::Changed { value, composing: true }, window, cx);
                }
                Output::Terminal(name, detail) if self.epoch() == Some(detail.view_epoch) => {
                    match name.as_str() {
                        "submit" => self.publish(Event::Submit { value: detail.value }, window, cx),
                        "cancel" => self.publish(Event::Cancel, window, cx),
                        "advance" => self.publish(Event::Advance { value: detail.value, forward: detail.forward }, window, cx),
                        "blurCommit" => self.publish(Event::Blur { value: detail.value }, window, cx),
                        _ => unreachable!(),
                    }
                }
                Output::Selection(detail) if self.epoch() == Some(detail.view_epoch) => {
                    text_select_utf16(&self.handle.owner, detail.start, detail.end, window, cx);
                }
                Output::Replacement(detail) if self.epoch() == Some(detail.view_epoch) && !self.is_composing() => {
                    if self.handle.call("resetValue", vec![json!(detail.value)], window, cx).ok() == Some(json!(true)) {
                        self.value = detail.value.clone();
                        self.publish(Event::Replaced { value: detail.value }, window, cx);
                        let _ = self.handle.call("selectRange", vec![json!(detail.start), json!(detail.end)], window, cx);
                    }
                }
                _ => {},
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all="camelCase")]
struct InputDetail { value: String, composing: bool, #[allow(dead_code)] data: Option<String>, #[allow(dead_code)] input_type: Option<String> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionDetail { value: String, #[allow(dead_code)] data: Option<String> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all="camelCase")]
struct TerminalDetail { value: String, forward: bool, #[serde(deserialize_with="whole_u64")] view_epoch: u64 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all="camelCase")]
struct SelectionDetail {
    #[serde(deserialize_with="whole_usize")] start: usize,
    #[serde(deserialize_with="whole_usize")] end: usize,
    #[serde(deserialize_with="whole_u64")] view_epoch: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all="camelCase")]
struct ReplacementDetail {
    value: String,
    #[serde(deserialize_with="whole_usize")] start: usize,
    #[serde(deserialize_with="whole_usize")] end: usize,
    #[serde(deserialize_with="whole_u64")] view_epoch: u64,
}
fn natural(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_f64().filter(|n| n.is_finite() && *n >= 0. && n.fract() == 0. && *n <= 9_007_199_254_740_991.).map(|n| n as u64))
}
fn whole_u64<'de, D: serde::Deserializer<'de>>(input: D) -> Result<u64, D::Error> {
    natural(&Value::deserialize(input)?).ok_or_else(|| serde::de::Error::custom("expected a nonnegative portable integer"))
}
fn whole_usize<'de, D: serde::Deserializer<'de>>(input: D) -> Result<usize, D::Error> {
    usize::try_from(whole_u64(input)?).map_err(serde::de::Error::custom)
}
enum Output { Input(InputDetail), Composition(String), CompositionEnd(String), Terminal(String, TerminalDetail), Selection(SelectionDetail), Replacement(ReplacementDetail), Ignored }
fn decode(name: &str, value: Value) -> Result<Output, String> {
    let parse = |error: serde_json::Error| format!("文本控件事件 {name} 无效：{error}");
    Ok(match name {
        "valueChange" => Output::Input(serde_json::from_value(value).map_err(parse)?),
        "compositionEnd" => Output::CompositionEnd(serde_json::from_value::<CompositionDetail>(value).map_err(parse)?.value),
        "compositionStart" | "compositionUpdate" => Output::Composition(serde_json::from_value::<CompositionDetail>(value).map_err(parse)?.value),
        "submit" | "cancel" | "advance" | "blurCommit" => Output::Terminal(name.into(), serde_json::from_value(value).map_err(parse)?),
        "selectionRequest" => Output::Selection(serde_json::from_value(value).map_err(parse)?),
        "replacementRequest" => Output::Replacement(serde_json::from_value(value).map_err(parse)?),
        "change" => Output::Ignored,
        _ => return Err(format!("文本控件返回未知事件：{name}")),
    })
}

impl Render for Field {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Fields are often constructed before a modal's lazy slot is mounted.
        // Bind the actual compiled parent at rendering time as well as create.
        let parent = crate::proto_parent::current();
        if crate::proto_parent::changed(&self.handle.owner, &parent) {
            if let Err(error) = self.root.update(cx, |root, cx| root.set_parent(parent, cx)) {
                let error = format!("文本控件父级绑定失败：{error:?}");
                cx.defer_in(window, move |field, window, cx| field.publish(Event::Failure(error), window, cx));
            }
        }
        self.flush_reset(window, cx);
        let th = self.theme;
        let focused = self.handle.state("focused").ok().and_then(|value| value.as_bool()).unwrap_or(false);
        div().id(SharedString::from(format!("proto-text-{}", cx.entity_id())))
            .w_full().overflow_hidden().text_color(th.text).text_size(px(15.)).px(px(6.)).py(px(4.))
            .when(self.options.inline, |view| view.h_full())
            .when(!self.options.inline, |view| view.h(px(36.)).flex_none().rounded(px(6.)).bg(th.control)
                .border_1().border_color(if focused { th.accent } else { th.shade }))
            .child(self.root.clone())
    }
}

#[derive(IntoElement)]
pub(crate) struct Inline {
    key: SharedString, label: String, value: String, theme: Theme, options: Options,
    callback: Callback, selection: Option<Range<usize>>, commands: Option<Commands>,
}
pub(crate) fn inline(key: impl Into<SharedString>, label: impl Into<String>, value: impl Into<String>, theme: Theme,
    options: Options, callback: impl Fn(Event, &mut Window, &mut App) + 'static) -> Inline {
    Inline { key: key.into(), label: label.into(), value: value.into(), theme, options, callback: Rc::new(callback), selection: None, commands: None }
}
impl Inline {
    pub fn selection(mut self, selection: Range<usize>) -> Self { self.selection = Some(selection); self }
    pub fn commands(mut self, commands: Commands) -> Self { self.commands = Some(commands); self }
}
impl RenderOnce for Inline {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (label, value, theme, options) = (self.label, self.value.clone(), self.theme, self.options);
        let field = window.use_keyed_state(self.key, cx, move |window, cx| Field::create(&label, &value, theme, options, window, cx));
        field.update(cx, |field, cx| {
            field.callback = Some(self.callback);
            field.theme = self.theme;
            if field.value() != self.value {
                field.set(&self.value, cx);
                if let Some(selection) = self.selection { field.set_selection(selection, cx); }
            }
        });
        if let Some(commands) = self.commands { *commands.0.borrow_mut() = Some(field.downgrade()); }
        field
    }
}

/// Handles point at the compiled editor entity, never a second cursor or text
/// buffer. Application shortcuts invoke the same authored requests as keys.
#[derive(Clone, Default)]
pub(crate) struct Commands(Rc<RefCell<Option<gpui::WeakEntity<Field>>>>);
impl Commands {
    pub fn invoke(&self, method: &str, arguments: Vec<Value>, window: &mut Window, cx: &mut App) {
        let Some(field) = self.0.borrow().clone() else { return };
        let _ = field.update(cx, |field, cx| { field.invoke(method, arguments, window, cx); });
    }
    pub fn replace(&self, value: &str, selection: Option<Range<usize>>, window: &mut Window, cx: &mut App) {
        let Some(field) = self.0.borrow().clone() else { return };
        let _ = field.update(cx, |field, cx| {
            if field.is_composing() { return; }
            field.set(value, cx);
            if let Some(selection) = selection { field.set_selection(selection, cx); }
            field.flush_reset(window, cx);
            if !field.pending_reset { field.publish(Event::Replaced { value: field.value.clone() }, window, cx); }
            field.drain(window, cx);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, WindowHandle, size};

    struct Harness { field: Entity<Field>, events: Vec<Event>, workspace_commands:usize }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().key_context("Verdant")
                .on_action(cx.listener(|host, _: &crate::actions::EditLyric, _, _| host.workspace_commands += 1))
                .on_action(cx.listener(|host, _: &crate::actions::DeleteSelection, _, _| host.workspace_commands += 1))
                .on_action(cx.listener(|host, _: &crate::actions::SelectAll, _, _| host.workspace_commands += 1))
                .on_action(cx.listener(|host, _: &crate::actions::GoToStart, _, _| host.workspace_commands += 1))
                .on_action(cx.listener(|host, _: &crate::actions::Escape, _, _| host.workspace_commands += 1))
                .on_action(cx.listener(|host, _: &crate::actions::Undo, _, _| host.workspace_commands += 1))
                .on_action(cx.listener(|host, _: &crate::actions::Redo, _, _| host.workspace_commands += 1))
                .child(div().key_context("PianoRoll").child(div().key_context("LyricEditor").child(self.field.clone())))
        }
    }
    fn frames(window: WindowHandle<Harness>, cx: &mut TestAppContext) {
        crate::native_test_support::frames(window, cx);
    }
    fn open(cx: &mut TestAppContext, value: &str, numeric: bool) -> WindowHandle<Harness> {
        let value = value.to_owned();
        let window = cx.open_window(size(px(360.), px(100.)), move |window, cx| {
            let field = Field::with_options("Text", &value, Theme::for_appearance(window.appearance()),
                Options { auto_focus: true, select_all: true, submit_on_enter: true, cancel_on_escape: true,
                    advance_on_tab:true, numeric: numeric.then_some(Numeric { min: 1., max: 32767., step: 1., default: 480. }), ..Default::default() }, window, cx);
            cx.subscribe_in(&field, window, |view: &mut Harness, _, event, _, _| view.events.push(event.clone())).detach();
            Harness { field, events: Vec::new(), workspace_commands:0 }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        frames(window, cx);
        window
    }

    #[test]
    fn source_json_float_indices_are_validated_without_losing_epoch() {
        assert!(matches!(decode("selectionRequest", json!({"start":0.0,"end":4.0,"viewEpoch":1.0})).unwrap(),
            Output::Selection(SelectionDetail { start: 0, end: 4, view_epoch: 1 })));
        for bad in [json!(-1.), json!(0.5), json!(9_007_199_254_740_992.0), json!("1")] {
            assert!(decode("selectionRequest", json!({"start":bad,"end":4.,"viewEpoch":1.})).is_err());
        }
    }

    #[gpui::test]
    fn compiled_native_editor_selects_initial_unicode_and_keeps_typing_caret(cx: &mut TestAppContext) {
        let window = open(cx, "a😀候", false);
        cx.simulate_input(window.into(), "xyz");
        frames(window, cx);
        window.update(cx, |host, _, cx| {
            assert_eq!(host.field.read(cx).value(), "xyz", "source select-all must replace the complete UTF-16 value and retain the typing caret");
            assert!(!host.events.iter().any(|event| matches!(event, Event::Failure(_))));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "ctrl-z");
        frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "xy")).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        frames(window, cx);
        window.update(cx, |host, _, _| assert!(host.events.iter().any(|event| matches!(event, Event::Submit { value } if value == "xy")))).unwrap();
    }

    #[gpui::test]
    fn programmatic_replacement_delivers_source_selection_before_the_next_native_input(cx: &mut TestAppContext) {
        let window = open(cx, "old draft", false);
        window.update(cx, |host, _, cx| {
            host.events.clear();
            host.field.update(cx, |field, cx| field.set("a😀候", cx));
        }).unwrap();
        frames(window, cx);
        window.update(cx, |host, _, cx| {
            let field = host.field.read(cx);
            assert_eq!(field.value(), "a😀候");
            assert!(!field.pending_reset);
            assert!(field.pending_selection.is_none());
            assert!(field.handle.take_events().is_empty(), "a render-time selectionRequest must reach the retained field observer without another input");
            assert!(host.events.is_empty(), "explicit reset and selection do not synthesize a user edit");
        }).unwrap();
        frames(window, cx);
        window.update(cx, |host, _, cx| {
            assert!(host.field.read(cx).handle.take_events().is_empty(), "idle rendering does not re-emit selection requests");
            assert!(host.events.is_empty());
        }).unwrap();
        // No Ctrl+A or explicit range command: the source-authored selection
        // from Field::set must already have reached the live native editor.
        cx.simulate_input(window.into(), "z"); frames(window, cx);
        window.update(cx, |host, _, cx| {
            assert_eq!(host.field.read(cx).value(), "z", "the next native edit replaces the complete UTF-16 value");
            assert_eq!(host.events.iter().filter(|event| matches!(event, Event::Changed { value, composing:false } if value == "z")).count(), 1);
            assert!(!host.events.iter().any(|event| matches!(event, Event::Failure(_))));
        }).unwrap();
    }

    #[gpui::test]
    fn compiled_numeric_text_steps_and_clears_without_document_commands(cx: &mut TestAppContext) {
        let window = open(cx, "", true);
        cx.simulate_keystrokes(window.into(), "up");
        frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "481")).unwrap();
        cx.simulate_keystrokes(window.into(), "delete");
        frames(window, cx);
        window.update(cx, |host, _, cx| {
            assert_eq!(host.field.read(cx).value(), "");
            assert!(!host.events.iter().any(|event| matches!(event, Event::Submit { .. } | Event::Failure(_))));
        }).unwrap();
    }

    #[gpui::test]
    fn native_editor_owns_the_registered_text_shortcuts(cx: &mut TestAppContext) {
        cx.update(|cx| { let bindings = crate::shortcuts::bindings(cx); cx.bind_keys(bindings); });
        let window = open(cx, "abc", false);
        cx.simulate_input(window.into(), "x y"); frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "x y")).unwrap();
        cx.simulate_keystrokes(window.into(), "left backspace home shift-right delete end"); frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "y")).unwrap();
        cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("候😀".into())));
        cx.simulate_keystrokes(window.into(), "ctrl-a ctrl-v"); frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "候😀")).unwrap();
        cx.simulate_keystrokes(window.into(), "ctrl-z"); frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "y", "Ctrl-Z remains native text undo")).unwrap();
        cx.simulate_keystrokes(window.into(), "ctrl-shift-z"); frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "候😀")).unwrap();
        cx.simulate_keystrokes(window.into(), "ctrl-z ctrl-y left shift-left right"); frames(window, cx);
        window.update(cx, |host, _, cx| assert_eq!(host.field.read(cx).value(), "候😀")).unwrap();
        cx.simulate_keystrokes(window.into(), "enter tab shift-tab escape"); frames(window, cx);
        window.update(cx, |host, _, _| {
            assert_eq!(host.workspace_commands, 0, "nested workspace commands cannot shadow the compiled editor");
            assert_eq!(host.events.iter().filter(|event| matches!(event, Event::Submit { value } if value == "候😀")).count(), 1);
            assert_eq!(host.events.iter().filter(|event| matches!(event, Event::Advance { forward:true, .. })).count(), 1);
            assert_eq!(host.events.iter().filter(|event| matches!(event, Event::Advance { forward:false, .. })).count(), 1);
            assert_eq!(host.events.iter().filter(|event| matches!(event, Event::Cancel)).count(), 1);
            assert!(!host.events.iter().any(|event| matches!(event, Event::Failure(_))));
        }).unwrap();
    }
}
