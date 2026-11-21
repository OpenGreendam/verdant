//! Native GPUI editors for the built-in suite. Window identity follows the
//! persisted instance UUID (or the live key for ambiguous legacy identities),
//! never its mutable slot number.
//! The child host remains responsible for audio; this view shares the same
//! parameter transaction, history and state cache as the docked device chain.

use gpui::{AppContext, Bounds, Context, FocusHandle, FontWeight, IntoElement,
    KeyDownEvent, Render, Role, WeakEntity, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, size};
use crate::devices::DeviceTarget;
use crate::editor::Editor;
use verdant_plugins::InsertSpec;

#[derive(Clone)]
enum Binding {
    Insert { instance: Option<String>, key: verdant_plugins::InsertKey, id: String },
    Instrument { id: String },
}

impl Binding {
    fn id(&self) -> &str { match self { Self::Insert { id, .. } | Self::Instrument { id } => id } }
    fn identity(&self) -> String {
        match self {
            Self::Insert { instance: Some(id), .. } => format!("instance/{id}"),
            Self::Insert { key, .. } => format!("live/{}", key.0),
            Self::Instrument { id } => format!("instrument/{id}"),
        }
    }
}

pub(crate) struct NativePluginWindow {
    editor: WeakEntity<Editor>,
    project: String,
    generation: u64,
    target: DeviceTarget,
    binding: Binding,
    focus: FocusHandle,
    generic: bool,
}

impl NativePluginWindow {
    /// Only open effect windows poll their own host. IPC/FFT work runs on the
    /// background executor; the render path reads the handle's cached frame.
    fn spawn_signal_poll(&self, cx: &mut Context<Self>) {
        if !matches!(self.binding, Binding::Insert { .. }) { return; }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(100)).await;
                let next = this.update(cx, |view, cx| {
                    let editor = view.editor.upgrade()?;
                    let ed = editor.read(cx);
                    if ed.project().project_id != view.project || ed.document_generation != view.generation { return None; }
                    let Binding::Insert { instance, key, id } = &view.binding else { return None };
                    let specs = ed.device_insert_specs(&view.target);
                    let slot = match instance {
                        Some(instance) => instance_slot(&specs, instance),
                        None => (0..specs.len()).find(|slot| ed.live_device_insert(&view.target, *slot).is_some_and(|handle| handle.key() == *key)),
                    }?;
                    if specs[slot].id != *id { return None; }
                    ed.live_device_insert(&view.target, slot)
                });
                let handle = match next { Ok(Some(handle)) => handle, Ok(None) => continue, Err(_) => break };
                let result = cx.background_spawn(async move { handle.refresh_telemetry() }).await;
                if result.is_ok() && this.update(cx, |_, cx| cx.notify()).is_err() { break; }
            }
        }).detach();
    }
}

fn instance_slot(specs: &[InsertSpec], instance: &str) -> Option<usize> {
    if instance.is_empty() { return None; }
    let mut matching = specs.iter().enumerate().filter(|(_, spec)| spec.instance_id == instance);
    let (slot, _) = matching.next()?;
    matching.next().is_none().then_some(slot)
}

impl Editor {
    pub(crate) fn open_builtin_editor(&mut self, target: DeviceTarget, slot: usize, cx: &mut Context<Self>) {
        let specs = self.device_insert_specs(&target);
        let Some(spec) = specs.get(slot) else { return };
        let Some(handle) = self.live_device_insert(&target, slot) else { return };
        let binding = Binding::Insert {
            instance: instance_slot(&specs, &spec.instance_id).map(|_| spec.instance_id.clone()),
            key: handle.key(), id: spec.id.clone(),
        };
        self.fetch_device_params(&target, slot, cx);
        self.queue_native_window(target, binding, spec.name.clone(), cx);
    }

    pub(crate) fn open_builtin_instrument_editor(&mut self, track: &verdant_domain::TrackId, cx: &mut Context<Self>) {
        let Some(spec) = self.project().track(track).and_then(|track| track.instrument()) else { return };
        let id = spec.get("id").and_then(serde_json::Value::as_str).unwrap_or_default();
        if spec.get("format").and_then(serde_json::Value::as_str) != Some("builtin")
            || !verdant_plugins::builtin_dsp::is_builtin_instrument(id) { return; }
        let name = verdant_plugins::builtin_dsp::instrument_name(id).unwrap_or("Verdant Synth").to_string();
        self.queue_native_window(DeviceTarget::Track(track.clone()), Binding::Instrument { id: id.into() }, name, cx);
    }

    fn queue_native_window(&mut self, target: DeviceTarget, binding: Binding, name: String, cx: &mut Context<Self>) {
        let project = self.project().project_id.clone();
        let generation = self.document_generation;
        let owner = match &target { DeviceTarget::Track(track) => format!("track/{track}"), DeviceTarget::Master => "master".into() };
        let window_key = format!("{generation}/{project}/{owner}/{}", binding.identity());
        let weak = cx.weak_entity();
        // GPUI draws a new window immediately. Opening outside this Editor
        // borrow prevents that first draw from borrowing the owner reentrantly.
        cx.defer(move |cx| {
            let Some(editor) = weak.upgrade() else { return };
            if editor.read(cx).project().project_id != project || editor.read(cx).document_generation != generation { return; }
            let existing = editor.read(cx).native_plugin_windows.get(&window_key).copied();
            if let Some(handle) = existing {
                if handle.update(cx, |_, window, _| window.activate_window()).is_ok() { return; }
            }
            let (width, height) = crate::devices::native_editor_size(binding.id());
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let observed = editor.clone();
            let result = cx.open_window(WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(400.), px(300.))),
                app_id: Some("verdant".into()), ..Default::default()
            }, move |window, cx| {
                window.set_window_title(&format!("{name} — Verdant"));
                let closing = observed.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let _ = closing.update(cx, |ed, cx| {
                        ed.cancel_device_drag_in(window.window_handle(), cx);
                        ed.close_menu(window, cx);
                    });
                    true
                });
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    cx.observe(&observed, |_, _, cx| cx.notify()).detach();
                    cx.observe_window_activation(window, |view: &mut NativePluginWindow, window, cx| {
                        if !window.is_window_active() {
                            let _ = view.editor.update(cx, |ed, cx| {
                                ed.cancel_device_drag_in(window.window_handle(), cx);
                                ed.close_menu(window, cx);
                            });
                        }
                    }).detach();
                    let view = NativePluginWindow { editor: observed.downgrade(), project, generation, target, binding, focus, generic: false };
                    view.spawn_signal_poll(cx);
                    view
                })
            });
            editor.update(cx, |ed, cx| {
                ed.native_plugin_windows.retain(|_, handle| handle.entity(cx).is_ok());
                match result {
                    Ok(handle) => { ed.native_plugin_windows.insert(window_key, handle); }
                    Err(error) => ed.notice(format!("无法打开内置插件界面：{error}")),
                }
                cx.notify();
            });
        });
    }
}

impl Render for NativePluginWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.editor.upgrade() else {
            window.remove_window();
            return div().into_any_element();
        };
        let project = &self.project;
        let generation = self.generation;
        let binding = &self.binding;
        let target = &self.target;
        let focus = self.focus.clone();
        let generic = self.generic;
        let toggle_generic = cx.listener(|view: &mut Self, _: &gpui::ClickEvent, _, cx| { view.generic = !view.generic; cx.notify(); });
        editor.update(cx, |ed, cx| {
            if &ed.project().project_id != project || ed.document_generation != generation {
                ed.cancel_device_drag_in(window.window_handle(), cx);
                window.remove_window();
                return div().into_any_element();
            }
            let th = ed.theme;
            let editor_entity = cx.entity();
            let (slot, name, bypassed, busy, body) = match binding {
                Binding::Insert { instance, key, id } => {
                    let specs = ed.device_insert_specs(target);
                    let slot = match instance {
                        Some(instance) => instance_slot(&specs, instance),
                        None => (0..specs.len()).find(|slot| ed.live_device_insert(target, *slot).is_some_and(|handle| handle.key() == *key)),
                    }.filter(|slot| specs[*slot].id == *id);
                    let Some(slot) = slot else {
                        ed.cancel_device_drag_in(window.window_handle(), cx);
                        window.remove_window();
                        return div().into_any_element();
                    };
                    let spec = &specs[slot];
                    ed.fetch_device_params(target, slot, cx);
                    let live = ed.live_device_insert(target, slot);
                    let running = live.as_ref().is_some_and(|handle| matches!(handle.status(), verdant_plugins::InsertStatus::Running));
                    let busy = ed.inserts_loading || live.as_ref().is_some_and(|handle| ed.plugin_busy.contains(&handle.key()));
                    let body = if let Some(handle) = live.filter(|_| running) {
                        if let Some(mut params) = ed.plugin_params.get(&handle.key()).cloned() {
                            crate::devices::preview_and_animate(ed, handle.key(), &mut params, window);
                            if generic { crate::devices::generic_editor(ed, &editor_entity, target, slot, handle.key(), &params) }
                            else { crate::devices::render_builtin_editor(ed, &editor_entity, target, slot, handle.key(), &params, cx) }
                        } else { div().p(px(24.)).child("正在读取参数…").into_any_element() }
                    } else { div().p(px(24.)).text_color(th.warning).child("插件未运行，工程参数已保留。请在效果器链重新加载。").into_any_element() };
                    (slot, spec.name.clone(), spec.bypass, busy, body)
                }
                Binding::Instrument { id } => {
                    let track = match target { DeviceTarget::Track(track) => track, DeviceTarget::Master => unreachable!() };
                    let current = ed.project().track(track).filter(|track| track.instrument().and_then(|spec| spec.get("id")).and_then(serde_json::Value::as_str) == Some(id.as_str()));
                    let Some(params) = current.and_then(crate::instrument::synth_params) else {
                        ed.cancel_device_drag_in(window.window_handle(), cx);
                        window.remove_window();
                        return div().into_any_element();
                    };
                    let key = crate::instrument::instrument_key_for_generation(track, id, generation);
                    let mut descs = crate::instrument::synth_descs_for(id, &params);
                    crate::devices::preview_and_animate(ed, key, &mut descs, window);
                    let body = div().flex().flex_col().gap(px(10.))
                        .child(crate::instrument::preset_button(ed, track, id, key, cx))
                        .child(crate::devices::synth_editor(ed, &editor_entity, track, key, &descs, cx)).into_any_element();
                    (crate::instrument::INSTRUMENT_SLOT, verdant_plugins::builtin_dsp::instrument_name(id).unwrap_or("Verdant Synth").into(), false, false, body)
                }
            };
            window.set_window_title(&format!("{name} — Verdant"));
            let bypass_target = target.clone();
            let is_insert = matches!(binding, Binding::Insert { .. });
            let expected_key = ed.live_device_insert(target, slot).map(|handle| handle.key());
            let bus = match target {
                DeviceTarget::Track(track) => ed.project().track(track).map(|track| track.name.clone()).unwrap_or_default(),
                DeviceTarget::Master => "主总线".into(),
            };
            let button = |id: &'static str, label: &'static str| div().id(id).role(Role::Button)
                .aria_label(label).cursor_pointer().px(px(10.)).py(px(6.)).rounded(px(6.))
                .bg(th.control).hover(move |s| s.bg(th.hover)).text_sm().child(label);
            let mut root = div().id("builtin-native-editor").role(Role::Group)
                .aria_label(format!("{name} 原生编辑器")).size_full().relative()
                .bg(th.view).text_color(th.text).font_family(ed.ui_font.clone())
                .flex().flex_col().track_focus(&focus)
                .on_key_down(cx.listener(|ed, event: &KeyDownEvent, window, cx| {
                    let key = event.keystroke.key.as_str();
                    if event.keystroke.modifiers.control || event.keystroke.modifiers.platform {
                        if key == "z" {
                            if event.keystroke.modifiers.shift { ed.redo(cx); } else { ed.undo(cx); }
                            window.prevent_default(); cx.stop_propagation();
                        }
                    } else if key == "escape" {
                        ed.cancel_device_drag_in(window.window_handle(), cx);
                        if ed.menu.is_some() { ed.close_menu(window, cx); }
                        cx.stop_propagation();
                    }
                }))
                .child(div().flex_none().h(px(56.)).px(px(16.)).flex().items_center().gap(px(10.))
                    .bg(th.headerbar).border_b_1().border_color(th.shade)
                    .child(div().flex_1().min_w_0().flex().flex_col()
                        .child(div().text_lg().font_weight(FontWeight::BOLD).child(name.clone()))
                        .child(div().text_xs().text_color(th.text_secondary).child(bus)))
                    .when(is_insert, |header| header
                        .child(button("native-bypass", if bypassed { "启用" } else { "旁通" })
                            .aria_selected(bypassed).on_click(cx.listener(move |ed, _, _, cx| {
                                if !busy && ed.document_generation == generation && expected_key.is_some_and(|key| ed.device_target_matches(&bypass_target, slot, key)) {
                                    ed.toggle_device_bypass(bypass_target.clone(), slot, cx);
                                }
                            })))
                        .child(button("native-generic", if generic { "图形界面" } else { "参数列表" }).on_click(toggle_generic)))
                    .child(button("native-close", "关闭").on_click(cx.listener(|ed, _, window, cx| {
                        ed.cancel_device_drag_in(window.window_handle(), cx);
                        ed.close_menu(window, cx);
                        window.remove_window();
                    }))))
                .child(div().id("native-plugin-scroll").flex_1().min_h_0().overflow_y_scroll().overflow_x_scroll()
                    .p(px(16.)).child(body))
                .child(div().flex_none().px(px(16.)).py(px(6.)).text_xs().text_color(th.text_secondary)
                    .child(if busy { "正在提交参数…" } else if bypassed { "效果器已旁通" } else { "拖动调节 · Shift 微调 · 点击数值精确输入 · Alt 点击复位" }));
            if let Some(notice) = ed.notices.back() {
                root = root.child(div().id("native-notice").role(Role::Status).px(px(16.)).py(px(8.)).bg(th.card)
                    .text_sm().child(notice.text.clone()));
            }
            if let Some(menu) = ed.menu.as_ref().filter(|menu| menu.window == window.window_handle()) {
                root = root.child(crate::menu::context_menu_layer(ed, menu, window, cx));
            }
            root.into_any_element()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_follows_instance_after_reorder_and_cannot_alias_replacement() {
        let a = InsertSpec::new("builtin", "verdant.builtin.eq", "EQ", "");
        let b = InsertSpec::new("builtin", "verdant.builtin.eq", "EQ", "");
        assert_eq!(instance_slot(&[a.clone(), b.clone()], &a.instance_id), Some(0));
        assert_eq!(instance_slot(&[b.clone(), a.clone()], &a.instance_id), Some(1));
        assert_eq!(instance_slot(&[b], &a.instance_id), None);
    }

    #[test]
    fn ambiguous_legacy_identity_requires_a_live_instance_key() {
        let mut a = InsertSpec::new("builtin", "verdant.builtin.eq", "EQ", "");
        a.instance_id.clear();
        assert_eq!(instance_slot(&[a.clone()], ""), None);
        a.instance_id = "legacy-duplicate".into();
        assert_eq!(instance_slot(&[a.clone(), a.clone()], &a.instance_id), None);
        let binding = |key| Binding::Insert { instance: None, key: verdant_plugins::InsertKey(key), id: a.id.clone() };
        assert_ne!(binding(1).identity(), binding(2).identity());
    }
}
