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
    tools_open: bool,
    keyboard_open: bool,
    held_notes: std::collections::BTreeSet<u8>,
    pending_notes: std::collections::BTreeSet<u8>,
    preview_started: bool,
}

impl NativePluginWindow {
    fn note(&mut self, note: u8, pressed: bool, cx: &mut gpui::App) {
        // A release before the deferred first press cancels that press.
        if !pressed { self.pending_notes.remove(&note); }
        let changed = if pressed { self.held_notes.insert(note) } else { self.held_notes.remove(&note) };
        if !changed { return; }
        let DeviceTarget::Track(track) = &self.target else { return; };
        if !matches!(self.binding, Binding::Instrument { .. }) { return; }
        if pressed { self.preview_started = true; }
        let generation = self.generation; let id = self.binding.id().to_string();
        let _ = self.editor.update(cx, |ed, cx| {
            if ed.document_generation == generation && ed.project().track(track).and_then(|t|t.instrument()).and_then(|s|s.get("id")).and_then(serde_json::Value::as_str) == Some(id.as_str()) {
                ed.play_instrument_key(track, note, pressed, cx);
            }
        });
    }
    fn release_notes(&mut self, cx: &mut gpui::App) {
        self.pending_notes.clear();
        let held: Vec<_> = self.held_notes.iter().copied().collect();
        let had_preview = std::mem::take(&mut self.preview_started);
        for key in held { self.note(key, false, cx); }
        if had_preview && matches!(self.binding, Binding::Instrument { .. }) {
            let DeviceTarget::Track(track) = &self.target else { return };
            let generation = self.generation; let id = self.binding.id().to_string();
            let _ = self.editor.update(cx, |ed, cx| {
                if ed.document_generation == generation && ed.project().track(track).and_then(|t| t.instrument())
                    .and_then(|spec| spec.get("id")).and_then(serde_json::Value::as_str) == Some(id.as_str()) {
                    ed.stop_instrument_preview(track, cx);
                }
            });
        }
    }

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
                    let handle = ed.live_device_insert(&view.target, slot)?;
                    let reference = crate::devices::workflow::reference_handle(ed, &view.target, slot, handle.key());
                    Some((handle, reference))
                });
                let handle = match next { Ok(Some(handle)) => handle, Ok(None) => continue, Err(_) => break };
                let result = cx.background_spawn(async move {
                    handle.0.refresh_telemetry()?;
                    if let Some(reference) = handle.1 { reference.refresh_telemetry()?; }
                    Ok::<_, verdant_plugins::PluginError>(())
                }).await;
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
                            view.release_notes(cx);
                            let _ = view.editor.update(cx, |ed, cx| {
                                ed.cancel_device_drag_in(window.window_handle(), cx);
                                ed.close_menu(window, cx);
                            });
                        }
                    }).detach();
                    cx.on_release(|view: &mut NativePluginWindow, cx| {
                        let notes: Vec<_> = view.held_notes.iter().copied().collect();
                        let had_preview = view.preview_started;
                        if !had_preview && notes.is_empty() { return; }
                        let owner = view.editor.clone(); let target = view.target.clone(); let generation = view.generation;
                        let id = view.binding.id().to_string();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |ed, cx| {
                                let DeviceTarget::Track(track) = target else { return };
                                if ed.document_generation != generation || ed.project().track(&track).and_then(|t| t.instrument())
                                    .and_then(|spec| spec.get("id")).and_then(serde_json::Value::as_str) != Some(id.as_str()) { return; }
                                for key in notes { ed.play_instrument_key(&track, key, false, cx); }
                                if had_preview { ed.stop_instrument_preview(&track, cx); }
                            });
                        });
                    }).detach();
                    let view = NativePluginWindow { editor: observed.downgrade(), project, generation, target, binding, focus, generic: false, tools_open: false, keyboard_open: true, held_notes: Default::default(), pending_notes: Default::default(), preview_started: false };
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
        let native_entity = cx.entity();
        let note_down = native_entity.clone();
        let note_up = native_entity.clone();
        let instrument_window = matches!(self.binding, Binding::Instrument { .. });
        let project = &self.project;
        let generation = self.generation;
        let binding = &self.binding;
        let target = &self.target;
        let focus = self.focus.clone();
        let generic = self.generic;
        let tools_open = self.tools_open;
        let keyboard_open = self.keyboard_open;
        let toggle_tools = cx.listener(|view: &mut Self, _: &gpui::ClickEvent, _, cx| { view.tools_open = !view.tools_open; cx.notify(); });
        let toggle_keyboard = cx.listener(|view: &mut Self, _: &gpui::ClickEvent, _, cx| { view.keyboard_open = !view.keyboard_open; cx.notify(); });
        let toggle_generic = cx.listener(|view: &mut Self, _: &gpui::ClickEvent, _, cx| { view.generic = !view.generic; cx.notify(); });
        editor.update(cx, |ed, cx| {
            if &ed.project().project_id != project || ed.document_generation != generation {
                ed.cancel_device_drag_in(window.window_handle(), cx);
                window.remove_window();
                return div().into_any_element();
            }
            let th = ed.theme;
            let zoom = ed.prefs.extra.get("plugin_ui_zoom").and_then(serde_json::Value::as_f64).unwrap_or(1.).clamp(0.75, 2.);
            window.set_rem_size(px(16. * zoom as f32));
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
                        } else { div().p(gpui::rems(24./16.)).child("正在读取参数…").into_any_element() }
                    } else { div().p(gpui::rems(24./16.)).text_color(th.warning).child("插件未运行，工程参数已保留。请在效果器链重新加载。").into_any_element() };
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
                    let body = if generic { crate::devices::generic_editor(ed, &editor_entity, target, crate::instrument::INSTRUMENT_SLOT, key, &descs) }
                        else { crate::devices::synth_editor(ed, &editor_entity, track, key, &descs, cx) };
                    (crate::instrument::INSTRUMENT_SLOT, verdant_plugins::builtin_dsp::instrument_name(id).unwrap_or("Verdant Synth").into(), false, false, body)
                }
            };
            window.set_window_title(&format!("{name} — Verdant"));
            let bypass_target = target.clone();
            let midi_target = target.clone();
            let is_insert = matches!(binding, Binding::Insert { .. });
            let expected_key = ed.live_device_insert(target, slot).map(|handle| handle.key());
            let modulation_key = expected_key.or_else(|| if let (DeviceTarget::Track(track), Binding::Instrument { id }) = (target, binding) {
                Some(crate::instrument::instrument_key_for_generation(track, id, generation))
            } else { None });
            let modulation_target = target.clone();
            let bus = match target {
                DeviceTarget::Track(track) => ed.project().track(track).map(|track| track.name.clone()).unwrap_or_default(),
                DeviceTarget::Master => "主总线".into(),
            };
            let button = |id: &'static str, label: &'static str| div().id(id).role(Role::Button)
                .aria_label(label).cursor_pointer().flex_none().px(px(10.)).py(px(6.)).rounded(px(6.))
                .bg(gpui::transparent_black()).hover(move |s| s.bg(th.hover)).text_size(px(13.)).child(label);
            let key_editor = editor_entity.clone();
            let mut root = div().id("builtin-native-editor").role(Role::Group)
                .aria_label(format!("{name} 原生编辑器")).size_full().relative()
                .bg(th.view).text_color(th.text).font(crate::theme::plugin_ui_font(ed.ui_font.clone()))
                .flex().flex_col().track_focus(&focus)
                .on_key_up(move |event: &gpui::KeyUpEvent, _, cx| {
                    if instrument_window { if let Some(note) = typing_note(&event.keystroke.key) {
                        note_up.update(cx, |view, cx| { view.note(note, false, cx); cx.notify(); });
                    } }
                })
                // Some X11 backends report auto-repeat with is_held=false.
                // De-duplicate physical note presses before any Editor update,
                // deferred work or notification, including the first-press gap.
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let key = event.keystroke.key.as_str();
                    if instrument_window && !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform && !event.keystroke.modifiers.alt
                        && key_editor.read(cx).menu.is_none() {
                        if let Some(note) = typing_note(key) {
                            let already_down = { let view = note_down.read(cx); view.held_notes.contains(&note) || view.pending_notes.contains(&note) };
                            if !already_down {
                                let reserved = note_down.update(cx, |view, _| reserve_note_press(&view.held_notes, &mut view.pending_notes, note));
                                if reserved {
                                    let entity = note_down.downgrade();
                                    cx.defer(move |cx| {
                                        let Some(entity) = entity.upgrade() else { return };
                                        entity.update(cx, |view, cx| {
                                            if view.pending_notes.remove(&note) { view.note(note, true, cx); cx.notify(); }
                                        });
                                    });
                                }
                            }
                            window.prevent_default(); cx.stop_propagation(); return;
                        }
                    }
                    if event.keystroke.modifiers.control || event.keystroke.modifiers.platform {
                        if key == "z" {
                            key_editor.update(cx, |ed, cx| {
                                if event.keystroke.modifiers.shift { ed.redo(cx); } else { ed.undo(cx); }
                            });
                            window.prevent_default(); cx.stop_propagation();
                        }
                    } else if key == "escape" {
                        key_editor.update(cx, |ed, cx| {
                            ed.cancel_device_drag_in(window.window_handle(), cx);
                            if ed.menu.is_some() { ed.close_menu(window, cx); }
                        });
                        cx.stop_propagation();
                    }
                })
                .child(div().flex_none().min_h(px(48.)).py(px(6.)).px(px(16.)).flex().flex_wrap().items_center().gap(px(10.))
                    .bg(th.headerbar).border_b_1().border_color(th.shade)
                    .child(div().flex_1().min_w_0().flex().items_baseline().gap(px(12.))
                        .child(div().truncate().text_size(px(17.)).font_weight(FontWeight::BOLD).child(name.clone()))
                        .child(div().truncate().text_size(px(12.)).text_color(th.text_secondary).child(bus)))
                    .when(is_insert, |header| header
                        .child(button("native-bypass", if bypassed { "启用" } else { "旁通" })
                            .aria_selected(bypassed).on_click(cx.listener(move |ed, _, _, cx| {
                                if !busy && ed.document_generation == generation && expected_key.is_some_and(|key| ed.device_target_matches(&bypass_target, slot, key)) {
                                    ed.toggle_device_bypass(bypass_target.clone(), slot, cx);
                                }
                            }))))
                    .child(button("native-generic", if generic { "图形界面" } else { "参数列表" }).on_click(toggle_generic))
                    .when(instrument_window, |header| header.child(button("native-keyboard", if keyboard_open { "收起键盘" } else { "演奏键盘" }).aria_selected(keyboard_open).on_click(toggle_keyboard)))
                    .child(button("native-tools", "工具").aria_selected(tools_open).on_click(toggle_tools))
                    .child(button("native-close", "关闭").on_click(cx.listener(|ed, _, window, cx| {
                        ed.cancel_device_drag_in(window.window_handle(), cx);
                        ed.close_menu(window, cx);
                        window.remove_window();
                    }))))
                .when(tools_open, |root| root.child(div().flex_none().px(px(16.)).py(px(4.)).flex().flex_wrap().items_center().gap(px(6.))
                    .child(button("native-midi-input", "MIDI 输入").on_click(cx.listener(move |ed, _, window, cx| {
                        ed.open_device_midi_input(midi_target.clone(), window, cx);
                    })))
                    .child(button("native-midi-stop", "停止 MIDI").on_click(cx.listener(|ed, _, _, cx| {
                        ed.stop_instrument_monitor(cx);
                        cx.notify();
                    })))
                    .child(button("native-instances", "全部实例").on_click(cx.listener(|_, _, _, cx| {
                        let editor = cx.entity();
                        cx.defer(move |cx| crate::devices::workflow::instances::open(editor, cx));
                    })))
                    .when(modulation_key.is_some() && !generic, |toolbar| toolbar
                        .child(button("native-modulation", "调制").on_click(cx.listener(move |ed, _, _, cx| {
                            if let Some(key) = modulation_key { crate::devices::toggle_modulation_panel(ed, modulation_target.clone(), slot, key, cx); }
                        }))))
                    .child(button("native-zoom-out", "−").aria_label("缩小插件界面").on_click(cx.listener(|ed, _, _, cx| {
                        let current = ed.prefs.extra.get("plugin_ui_zoom").and_then(serde_json::Value::as_f64).unwrap_or(1.);
                        ed.prefs.extra.insert("plugin_ui_zoom".into(), ((current - 0.125).clamp(0.75, 2.)).into()); ed.prefs_dirty = true; cx.notify();
                    })))
                    .child(div().text_size(px(12.)).child(format!("{:.0}%", zoom * 100.)))
                    .child(button("native-zoom-in", "＋").aria_label("放大插件界面").on_click(cx.listener(|ed, _, _, cx| {
                        let current = ed.prefs.extra.get("plugin_ui_zoom").and_then(serde_json::Value::as_f64).unwrap_or(1.);
                        ed.prefs.extra.insert("plugin_ui_zoom".into(), ((current + 0.125).clamp(0.75, 2.)).into()); ed.prefs_dirty = true; cx.notify();
                    })))
                    .child(button("native-zoom-reset", "100%").on_click(cx.listener(|ed, _, _, cx| {
                        ed.prefs.extra.insert("plugin_ui_zoom".into(), 1.0.into()); ed.prefs_dirty = true; cx.notify();
                    })))
                    .child(button("native-fullscreen", "全屏").on_click(|_, window, _| window.toggle_fullscreen()))))
                .child(crate::devices::modulation_source_bar(ed, target, slot, if generic { None } else { modulation_key }))
                .child(div().id("native-plugin-scroll").flex_1().min_h_0().overflow_y_scroll().overflow_x_scroll()
                    .p(gpui::rems(12./16.)).child(body))
                .when(instrument_window && keyboard_open, |root| root.child(div().flex_none().px(gpui::rems(12./16.)).pt(gpui::rems(8./16.)).border_t_1().border_color(th.shade)
                    .child(performance_keyboard(&native_entity, &self.held_notes, th))))
                .child(div().flex_none().px(px(16.)).py(px(6.)).text_size(px(12.)).text_color(th.text_secondary)
                    .child(if busy { "正在提交参数…" } else if bypassed { "效果器已旁通" } else { "拖动调节 · Shift 微调 · 点击数值精确输入 · Alt 点击复位" }));
            if let Some(notice) = ed.notices.back() {
                root = root.child(div().id("native-notice").role(Role::Status).px(px(16.)).py(px(8.)).bg(th.card)
                    .max_h(px(52.)).overflow_hidden().text_size(px(14.)).child(notice.text.clone()));
            }
            if let Some(menu) = ed.menu.as_ref().filter(|menu| menu.window == window.window_handle()) {
                root = root.child(crate::menu::context_menu_layer(ed, menu, window, cx));
            }
            root.into_any_element()
        })
    }
}

/// Reserve the first key event before deferral. Repeated KeyPress events may
/// all have is_held=false; the window's actual press state is authoritative.
fn reserve_note_press(held: &std::collections::BTreeSet<u8>, pending: &mut std::collections::BTreeSet<u8>, note: u8) -> bool {
    !held.contains(&note) && pending.insert(note)
}

fn typing_note(key: &str) -> Option<u8> {
    ["a", "w", "s", "e", "d", "f", "t", "g", "y", "h", "u", "j", "k"].iter().position(|&s| s == key).map(|i| 60 + i as u8)
}
fn performance_keyboard(view: &gpui::Entity<NativePluginWindow>, held: &std::collections::BTreeSet<u8>, th: crate::theme::Theme) -> gpui::AnyElement {
    let mut keyboard = div().relative().h(gpui::rems(60./16.)).w_full();
    for black_pass in [false, true] {
        let mut white = 0usize;
        for key in 48u8..72 {
            let black = matches!(key % 12, 1 | 3 | 6 | 8 | 10);
            if black == black_pass {
                let pressed = held.contains(&key);
                let down = view.clone(); let up = view.clone(); let outside = view.clone();
                let label = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][key as usize % 12];
                keyboard = keyboard.child(div().id(format!("play-note-{key}")).role(Role::Button).aria_label(format!("{label}{}", key / 12 - 1))
                    .absolute().top(gpui::rems(0.)).left(gpui::relative((white as f32 - if black { 0.32 } else { 0. }) / 14.))
                    .w(gpui::relative(if black { 0.64 / 14. } else { 1. / 14. }))
                    .h(gpui::rems(if black { 38./16. } else { 60./16. })).rounded(gpui::rems(4./16.))
                    .border_1().border_color(th.shade).bg(if pressed { th.accent } else if black { th.key_black } else { th.key_white })
                    .text_color(if black { th.text_secondary } else { th.text }).cursor_pointer().flex().items_end().justify_center().pb(gpui::rems(4./16.)).text_xs()
                    .child(if black { String::new() } else { format!("{label}{}", key / 12 - 1) })
                    .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| { down.update(cx, |view, cx| { view.note(key, true, cx); cx.notify(); }); })
                    .on_mouse_up(gpui::MouseButton::Left, move |_, _, cx| { up.update(cx, |view, cx| { view.note(key, false, cx); cx.notify(); }); })
                    .on_mouse_up_out(gpui::MouseButton::Left, move |_, _, cx| { outside.update(cx, |view, cx| { view.note(key, false, cx); cx.notify(); }); }));
            }
            if !black { white += 1; }
        }
    }
    div().flex().flex_col().gap(gpui::rems(6./16.)).child(keyboard)
        .child(div().text_xs().text_color(th.text_secondary).child("实时演奏 · 点击琴键或使用 A W S E D F T G Y H U J K")).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_keypresses_without_held_flag_schedule_only_one_note() {
        let mut held = std::collections::BTreeSet::new();
        let mut pending = std::collections::BTreeSet::new();
        let key = typing_note("a").unwrap();
        assert!(reserve_note_press(&held, &mut pending, key));
        for _ in 0..100 { assert!(!reserve_note_press(&held, &mut pending, key)); }
        assert!(pending.remove(&key)); held.insert(key);
        for _ in 0..100 { assert!(!reserve_note_press(&held, &mut pending, key)); }
        held.remove(&key);
        assert!(reserve_note_press(&held, &mut pending, key), "a new physical press is allowed after release");
    }

    #[test]
    fn early_key_release_or_blur_cancels_the_deferred_note() {
        let held = std::collections::BTreeSet::new();
        let mut pending = std::collections::BTreeSet::new();
        let first = typing_note("a").unwrap(); let second = typing_note("s").unwrap();
        assert!(reserve_note_press(&held, &mut pending, first));
        pending.remove(&first); // key-up before the deferred first press
        assert!(!pending.remove(&first), "the deferred callback must not start a released note");
        assert!(reserve_note_press(&held, &mut pending, first));
        assert!(reserve_note_press(&held, &mut pending, second));
        pending.clear(); // focus loss or window closure
        assert!(!pending.remove(&first)); assert!(!pending.remove(&second));
        assert!(reserve_note_press(&held, &mut pending, second));
    }

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
