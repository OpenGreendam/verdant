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
    preview_started: bool,
}

impl gpui::Focusable for NativePluginWindow {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle { self.focus.clone() }
}

#[derive(Clone, Copy)]
enum ToolbarAction {
    Bypass { slot: usize, expected_key: Option<verdant_plugins::InsertKey>, busy: bool },
    Generic,
    Tools,
    Keyboard,
    Close,
    MidiInput,
    MidiStop,
    Instances,
    Modulation { slot: usize, key: Option<verdant_plugins::InsertKey> },
    Zoom(f64),
    ZoomReset,
    Fullscreen,
}

fn toolbar_command(editor: WeakEntity<Editor>, view: WeakEntity<NativePluginWindow>, action: ToolbarAction) -> crate::devices::SemanticCommand {
    crate::devices::semantic_command(editor, view, move |view, _, window, cx| view.toolbar_action(action, window, cx))
}

impl NativePluginWindow {
    fn toolbar_action(&mut self, action: ToolbarAction, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(action, ToolbarAction::Close) {
            let _ = self.editor.update(cx, |ed, cx| ed.close_menu(window, cx));
            window.remove_window();
            return;
        }
        let Some(editor) = self.editor.upgrade() else { return };
        let ed = editor.read(cx);
        if ed.document_generation != self.generation || ed.project().project_id != self.project { return; }
        match action {
            ToolbarAction::Generic => { self.generic = !self.generic; cx.notify(); }
            ToolbarAction::Tools => { self.tools_open = !self.tools_open; cx.notify(); }
            ToolbarAction::Keyboard => {
                self.keyboard_open = !self.keyboard_open;
                if !self.keyboard_open { self.cancel_keyboard_input(window, cx); }
                cx.notify();
            }
            ToolbarAction::Fullscreen => window.toggle_fullscreen(),
            _ => {
                let target = self.target.clone();
                editor.update(cx, |ed, cx| match action {
                    ToolbarAction::Bypass { slot, expected_key, busy } => {
                        if !busy && expected_key.is_some_and(|key| ed.device_target_matches(&target, slot, key)) {
                            ed.toggle_device_bypass(target, slot, cx);
                        }
                    }
                    ToolbarAction::MidiInput => ed.open_device_midi_input(target, window, cx),
                    ToolbarAction::MidiStop => { ed.stop_instrument_monitor(cx); cx.notify(); }
                    ToolbarAction::Instances => {
                        let editor = cx.entity();
                        cx.defer(move |cx| crate::devices::workflow::instances::open(editor, cx));
                    }
                    ToolbarAction::Modulation { slot, key } => {
                        if let Some(key) = key { crate::devices::toggle_modulation_panel(ed, target, slot, key, cx); }
                    }
                    ToolbarAction::Zoom(delta) => {
                        let current = ed.prefs.extra.get("plugin_ui_zoom").and_then(serde_json::Value::as_f64).unwrap_or(1.);
                        ed.prefs.extra.insert("plugin_ui_zoom".into(), ((current + delta).clamp(0.75, 2.)).into());
                        ed.prefs_dirty = true; cx.notify();
                    }
                    ToolbarAction::ZoomReset => {
                        ed.prefs.extra.insert("plugin_ui_zoom".into(), 1.0.into()); ed.prefs_dirty = true; cx.notify();
                    }
                    ToolbarAction::Generic | ToolbarAction::Tools | ToolbarAction::Keyboard | ToolbarAction::Close | ToolbarAction::Fullscreen => unreachable!(),
                });
            }
        }
    }

    fn note(&mut self, note: u8, pressed: bool, cx: &mut gpui::App) {
        // The compiled key owns physical contacts and collapses an early
        // release before deferred delivery. This set records sounding notes.
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

    fn cancel_keyboard_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Source contact retirement precedes audio release. An accepted key
        // can still be waiting in the deferred native-to-audio delivery queue.
        let scope = format!("native-keyboard-{:?}", cx.entity_id());
        crate::proto_performance_key::cancel_scope(&scope, window, cx);
        self.release_notes(cx);
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
                    cx.on_blur(&focus, window, |view: &mut NativePluginWindow, window, cx| {
                        let _ = view.editor.update(cx, |ed, cx| ed.cancel_device_drag_in(window.window_handle(), cx));
                    }).detach();
                    cx.observe(&observed, |_, _, cx| cx.notify()).detach();
                    cx.observe_window_activation(window, |view: &mut NativePluginWindow, window, cx| {
                        if !window.is_window_active() {
                            view.cancel_keyboard_input(window, cx);
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
                    let view = NativePluginWindow { editor: observed.downgrade(), project, generation, target, binding, focus, generic: false, tools_open: false, keyboard_open: true, held_notes: Default::default(), preview_started: false };
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
        let keyboard_scope=format!("native-keyboard-{:?}",native_entity.entity_id());
        let keyboard_down=keyboard_scope.clone();let keyboard_up=keyboard_scope.clone();
        let instrument_window = matches!(self.binding, Binding::Instrument { .. });
        let project = &self.project;
        let generation = self.generation;
        let binding = &self.binding;
        let target = &self.target;
        let focus = self.focus.clone();
        let generic = self.generic;
        let tools_open = self.tools_open;
        let keyboard_open = self.keyboard_open;
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
            let is_insert = matches!(binding, Binding::Insert { .. });
            let expected_key = ed.live_device_insert(target, slot).map(|handle| handle.key());
            let modulation_key = expected_key.or_else(|| if let (DeviceTarget::Track(track), Binding::Instrument { id }) = (target, binding) {
                Some(crate::instrument::instrument_key_for_generation(track, id, generation))
            } else { None });
            let bus = match target {
                DeviceTarget::Track(track) => ed.project().track(track).map(|track| track.name.clone()).unwrap_or_default(),
                DeviceTarget::Master => "主总线".into(),
            };
            let button = |id: &'static str, label: &'static str, action| crate::devices::semantic_click(crate::proto_controls::action(id, label)
                .aria_label(label).authority(crate::proto_controls::editor_authority(ed))
                .cursor_pointer().flex_none().px(px(10.)).py(px(6.)).rounded(px(6.))
                .bg(gpui::transparent_black()).hover(move |s| s.bg(th.hover)).text_size(px(13.)).child(label),
                toolbar_command(editor_entity.downgrade(), native_entity.downgrade(), action));
            let key_editor = editor_entity.clone();
            let mut root = div().id("builtin-native-editor").role(Role::Group)
                .aria_label(format!("{name} 原生编辑器")).size_full().relative()
                .bg(th.view).text_color(th.text).font(crate::theme::plugin_ui_font(ed.ui_font.clone()))
                .flex().flex_col().track_focus(&focus)
                .on_key_up(move |event: &gpui::KeyUpEvent, window, cx| {
                    if instrument_window { crate::proto_performance_key::route_key(&keyboard_up,&event.keystroke.key,false,window,cx); }
                })
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if window.default_prevented() { return; }
                    let key=event.keystroke.key.as_str();
                    if instrument_window && keyboard_open && !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform && !event.keystroke.modifiers.alt
                        && key_editor.read(cx).menu.is_none()
                        && crate::proto_performance_key::route_key(&keyboard_down,key,true,window,cx) {
                        window.prevent_default();cx.stop_propagation();return;
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
                        .child(button("native-bypass", if bypassed { "启用" } else { "旁通" }, ToolbarAction::Bypass { slot, expected_key, busy }).aria_selected(bypassed)))
                    .child(button("native-generic", if generic { "图形界面" } else { "参数列表" }, ToolbarAction::Generic))
                    .when(instrument_window, |header| header.child(button("native-keyboard", if keyboard_open { "收起键盘" } else { "演奏键盘" }, ToolbarAction::Keyboard).aria_selected(keyboard_open)))
                    .child(button("native-tools", "工具", ToolbarAction::Tools).aria_selected(tools_open))
                    .child(button("native-close", "关闭", ToolbarAction::Close)))
                .when(tools_open, |root| root.child(div().flex_none().px(px(16.)).py(px(4.)).flex().flex_wrap().items_center().gap(px(6.))
                    .child(button("native-midi-input", "MIDI 输入", ToolbarAction::MidiInput))
                    .child(button("native-midi-stop", "停止 MIDI", ToolbarAction::MidiStop))
                    .child(button("native-instances", "全部实例", ToolbarAction::Instances))
                    .when(modulation_key.is_some() && !generic, |toolbar| toolbar
                        .child(button("native-modulation", "调制", ToolbarAction::Modulation { slot, key: modulation_key })))
                    .child(button("native-zoom-out", "−", ToolbarAction::Zoom(-0.125)).aria_label("缩小插件界面"))
                    .child(div().text_size(px(12.)).child(format!("{:.0}%", zoom * 100.)))
                    .child(button("native-zoom-in", "＋", ToolbarAction::Zoom(0.125)).aria_label("放大插件界面"))
                    .child(button("native-zoom-reset", "100%", ToolbarAction::ZoomReset))
                    .child(button("native-fullscreen", "全屏", ToolbarAction::Fullscreen))))
                .child(crate::devices::modulation_source_bar(ed, &editor, target, slot, if generic { None } else { modulation_key }))
                .child(crate::proto_scroll::viewport("native-plugin-scroll", "插件编辑器", "both").flex_1().min_h_0()
                    .p(gpui::rems(12./16.)).child(body))
                .when(instrument_window && keyboard_open, |root| root.child(div().flex_none().px(gpui::rems(12./16.)).pt(gpui::rems(8./16.)).border_t_1().border_color(th.shade)
                    .child(performance_keyboard(&native_entity,&keyboard_scope,format!("{}:{}:{}",self.project,self.generation,self.binding.identity()),th))))
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

/// Native key geometry; the compiled PerformanceKey owns press and key-repeat semantics.
fn performance_keyboard(view:&gpui::Entity<NativePluginWindow>,scope:&str,revision:String,th:crate::theme::Theme)->gpui::AnyElement {
    let mut keyboard=div().relative().h(gpui::rems(60./16.)).w_full();
    for black_pass in [false,true] {
        let mut white=0usize;
        for note in 48u8..=72 {
            let black=matches!(note%12,1|3|6|8|10);
            if black==black_pass {
                let label=["C","C♯","D","D♯","E","F","F♯","G","G♯","A","A♯","B"][note as usize%12];
                let name=format!("{label}{}",note/12-1);
                let display=if black { String::new() } else { name.clone() };
                let target=view.downgrade();
                let basis=revision.clone();
                let key=crate::proto_performance_key::key(scope,note,name,revision.clone(),false,
                    move |pressed|div().size_full().rounded(gpui::rems(4./16.)).border_1().border_color(th.shade)
                        .bg(if pressed{th.accent}else if black{th.key_black}else{th.key_white})
                        .text_color(if black{th.text_secondary}else{th.text}).cursor_pointer().flex().items_end().justify_center().pb(gpui::rems(4./16.)).text_xs()
                        .child(display.clone()).into_any_element(),
                    move |pressed,cx|{let _=target.update(cx,|view,cx| {
                        if pressed&&(!view.keyboard_open||format!("{}:{}:{}",view.project,view.generation,view.binding.identity())!=basis){return;}
                        view.note(note,pressed,cx);cx.notify();
                    });});
                keyboard=keyboard.child(div().absolute().top(gpui::rems(0.)).left(gpui::relative((white as f32-if black{0.32}else{0.})/15.))
                    .w(gpui::relative(if black{0.64/15.}else{1./15.})).h(gpui::rems(if black{38./16.}else{60./16.})).child(key));
            }
            if !black{white+=1;}
        }
    }
    div().flex().flex_col().gap(gpui::rems(6./16.)).child(keyboard)
        .child(div().text_xs().text_color(th.text_secondary).child("实时演奏 · 点击琴键或使用 A W S E D F T G Y H U J K")).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{App, Entity, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, WindowHandle, point};
    use verdant_normalized_knob::{NormalizedKnobHandle, native::gpui_native_owner::{all_owners, resolved}};

    fn native_windows(cx: &mut TestAppContext, count: usize) -> (WindowHandle<Editor>, Entity<Editor>, Vec<WindowHandle<NativePluginWindow>>) {
        let main = cx.open_window(size(px(1300.0), px(900.0)), |window, cx| {
            let mut project = crate::native_test_support::instrument_project();
            if count > 1 {
                project.tracks[1].unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(),
                    crate::instrument::synth_spec(&verdant_plugins::builtin_dsp::SynthParams::default(), "test"));
            }
            let mut editor = Editor::for_native_test(project, window, cx);
            editor.needs_fit = false;
            editor
        });
        let editor = main.update(cx, |ed, _, cx| {
            let tracks: Vec<_> = ed.project().tracks.iter().take(count).map(|track| track.id.clone()).collect();
            for track in tracks { ed.open_builtin_instrument_editor(&track, cx); }
            cx.entity()
        }).unwrap();
        cx.run_until_parked();
        let windows: Vec<_> = main.update(cx, |ed, _, _| ed.native_plugin_windows.values().copied().collect()).unwrap();
        assert_eq!(windows.len(), count);
        for window in &windows { crate::native_test_support::frames(*window, cx); }
        (main, editor, windows)
    }

    fn gain_knob(window: &Window) -> NormalizedKnobHandle {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.declared_events.contains("valueCommit")
                && owner.modules.accessible_text.get("name").is_some_and(|name| resolved(name).text() == "音量")
        }).expect("production native window's compiled gain knob");
        NormalizedKnobHandle { owner }
    }

    fn hold_knob(handle: &NormalizedKnobHandle, window: &mut Window, cx: &mut App) -> gpui::Point<gpui::Pixels> {
        let start = handle.owner.borrow().native_hitbox.as_ref().unwrap().bounds.center();
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position: start, button: MouseButton::Left, ..Default::default()
        }), cx);
        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
            position: start + point(px(0.0), px(-36.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
        }), cx);
        assert_eq!(handle.state("phase").unwrap(), serde_json::json!("dragging"));
        start
    }

    fn release_knob(start: gpui::Point<gpui::Pixels>, window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
            position: start + point(px(0.0), px(-36.0)), button: MouseButton::Left, ..Default::default()
        }), cx);
    }

    fn performance_key(note: u8, window: &Window) -> verdant_performance_key::PerformanceKeyHandle {
        let mut owners = verdant_performance_key::native::gpui_native_owner::all_owners().into_iter().filter(|owner| {
            let owner = owner.borrow();
            owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.declared_events.contains("pressedChange")
                && crate::proto_controls::source_integer(owner.props.get("note")) == Some(i64::from(note))
        });
        let owner = owners.next().expect("the production keyboard mounts the authored note");
        assert!(owners.next().is_none(), "one source control owns this note in this window");
        verdant_performance_key::PerformanceKeyHandle { owner }
    }

    // GPUI keeps handle_a11y_action crate-private. These tests enter the exact
    // semantic callback shared by the rendered mouse/accessible handlers while
    // using real window input, compiled leases and the durable project store.
    // AT-SPI action transport is validated separately by the OS consumer.
    #[gpui::test]
    fn toolbar_close_cancels_a_held_knob_before_detach_and_late_up(cx: &mut TestAppContext) {
        let (main, editor, windows) = native_windows(cx, 1);
        cx.update_window(windows[0].into(), |_, window, cx| {
            let knob = gain_knob(window);
            let accepted = knob.state("value").unwrap();
            let start = hold_knob(&knob, window, cx);
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            toolbar_command(editor.downgrade(), view.downgrade(), ToolbarAction::Close)(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted, "cancel restores accepted state before closing");
            assert!(!editor.read(cx).has_proto_pointer_contact());
            assert_eq!(editor.read(cx).revision().get(), 0);
            // Even if the old rendered input tree receives a queued release
            // before the window disappears, it has no accepted lease to finish.
            release_knob(start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 0);
        }).unwrap();
        crate::native_test_support::frames(main, cx);
        main.update(cx, |ed, _, _| assert_eq!(ed.revision().get(), 0)).unwrap();
    }

    #[gpui::test]
    fn toolbar_layout_toggles_cancel_the_owned_preview_before_redraw(cx: &mut TestAppContext) {
        let (_main, editor, windows) = native_windows(cx, 1);
        let native = windows[0];
        for action in [ToolbarAction::Tools, ToolbarAction::Keyboard] {
            // The rendered performance keys also end their held note on a
            // mouse-up outside. Give each layout command its own physical
            // typing press instead of carrying it through the prior drag's up.
            cx.update_window(native.into(), |_, window, cx| {
                let view = window.root::<NativePluginWindow>().flatten().unwrap();
                let focus = view.read(cx).focus.clone();
                window.focus(&focus, cx);
            }).unwrap();
            crate::native_test_support::frames(native, cx);
            cx.update_window(native.into(), |_, window, cx| {
                window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                    keystroke: gpui::Keystroke::parse("a").unwrap(), is_held: false, prefer_character_input: false,
                }), cx);
            }).unwrap();
            cx.run_until_parked();
            native.update(cx, |view, _, _| assert!(view.held_notes.contains(&60))).unwrap();
            cx.update_window(native.into(), |_, window, cx| {
                let knob = gain_knob(window);
                let accepted = knob.state("value").unwrap();
                let start = hold_knob(&knob, window, cx);
                let view = window.root::<NativePluginWindow>().flatten().unwrap();
                if matches!(action, ToolbarAction::Keyboard) {
                    assert!(view.read(cx).held_notes.contains(&60), "a performance note is still held before hiding its keyboard");
                    window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                        keystroke: gpui::Keystroke::parse("s").unwrap(), is_held: false, prefer_character_input: false,
                    }), cx);
                    assert!(crate::proto_performance_key::pressed(&format!("native-keyboard-{:?}",view.entity_id()),62,window,cx), "the authored key owns the second press before audio delivery");
                    assert_eq!(performance_key(62, window).state("pressed").unwrap(), serde_json::json!(true),
                        "the second physical key is accepted by the source before audio delivery");
                    assert!(!view.read(cx).held_notes.contains(&62), "audio delivery is still deferred");
                }
                let before = { let view = view.read(cx); (view.tools_open, view.keyboard_open) };
                toolbar_command(editor.downgrade(), view.downgrade(), action)(None, window, cx);
                let after = { let view = view.read(cx); (view.tools_open, view.keyboard_open) };
                match action {
                    ToolbarAction::Tools => {
                        assert_eq!(after, (!before.0, before.1));
                        assert!(view.read(cx).held_notes.contains(&60), "opening tools preserves held performance notes");
                    }
                    ToolbarAction::Keyboard => {
                        assert_eq!(after, (before.0, !before.1));
                        let view = view.read(cx);
                        assert!(view.held_notes.is_empty());
                        assert!(!view.preview_started);
                    }
                    _ => unreachable!(),
                }
                assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
                assert_eq!(knob.state("value").unwrap(), accepted);
                assert!(!editor.read(cx).has_proto_pointer_contact());
                release_knob(start, window, cx);
                assert_eq!(editor.read(cx).revision().get(), 0, "layout commands cannot commit a held preview");
                for key in if matches!(action, ToolbarAction::Keyboard) { &["a", "s"][..] } else { &["a"][..] } {
                    window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                        keystroke: gpui::Keystroke::parse(key).unwrap(),
                    }), cx);
                }
            }).unwrap();
            crate::native_test_support::frames(native, cx);
        }
        native.update(cx, |view, _, _| {
            assert!(view.held_notes.is_empty(), "queued note work cannot restart the hidden keyboard");
        }).unwrap();
        cx.update_window(native.into(), |_, window, cx| {
            let knob = gain_knob(window);
            let start = hold_knob(&knob, window, cx);
            release_knob(start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 1, "a fresh lease remains usable after both layout changes");
        }).unwrap();
    }

    #[gpui::test]
    fn toolbar_activation_preserves_another_window_and_a_replacement_lease(cx: &mut TestAppContext) {
        let (_main, editor, windows) = native_windows(cx, 2);
        let second = windows[1];
        let (knob, accepted, old_start) = cx.update_window(second.into(), |_, window, cx| {
            let knob = gain_knob(window);
            let accepted = knob.state("value").unwrap();
            let start = hold_knob(&knob, window, cx);
            (knob, accepted, start)
        }).unwrap();
        cx.update_window(windows[0].into(), |_, window, cx| {
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            let target = view.read(cx).target.clone();
            let key = crate::instrument::instrument_key_for_generation(target.track_id().unwrap(),
                verdant_plugins::builtin_dsp::SYNTH_ID, editor.read(cx).document_generation);
            toolbar_command(editor.downgrade(), view.downgrade(), ToolbarAction::Modulation {
                slot: crate::instrument::INSTRUMENT_SLOT, key: Some(key),
            })(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"), "another window's command cannot cancel the owner");
        }).unwrap();
        let new_start = cx.update_window(second.into(), |_, window, cx| {
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            let target = view.read(cx).target.clone();
            let track = target.track_id().unwrap();
            let key = crate::instrument::instrument_key_for_generation(track, verdant_plugins::builtin_dsp::SYNTH_ID, editor.read(cx).document_generation);
            toolbar_command(editor.downgrade(), view.downgrade(), ToolbarAction::Modulation {
                slot: crate::instrument::INSTRUMENT_SLOT, key: Some(key),
            })(None, window, cx);
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("idle"));
            assert_eq!(knob.state("value").unwrap(), accepted);
            release_knob(old_start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 0);
            // Start a new generation before queued observers of the cancelled
            // preview run. They must leave this accepted lease intact.
            hold_knob(&knob, window, cx)
        }).unwrap();
        crate::native_test_support::frames(second, cx);
        cx.update_window(second.into(), |_, window, cx| {
            assert_eq!(knob.state("phase").unwrap(), serde_json::json!("dragging"));
            assert!(editor.read(cx).has_proto_pointer_contact());
            release_knob(new_start, window, cx);
            assert_eq!(editor.read(cx).revision().get(), 1, "only the replacement drag commits");
            assert!(!editor.read(cx).has_proto_pointer_contact());
        }).unwrap();
    }

    #[gpui::test]
    fn repeated_keypresses_without_held_flag_schedule_only_one_note(cx: &mut TestAppContext) {
        let (_, _, windows) = native_windows(cx, 1);
        let native = windows[0];
        cx.update_window(native.into(), |_, window, cx| {
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            window.focus(&view.read(cx).focus.clone(), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        for _ in 0..2 {
            cx.update_window(native.into(), |_, window, cx| {
                for _ in 0..100 {
                    window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                        keystroke: gpui::Keystroke::parse("a").unwrap(), is_held: false, prefer_character_input: false,
                    }), cx);
                }
                assert_eq!(performance_key(60, window).state("pressed").unwrap(), serde_json::json!(true));
            }).unwrap();
            crate::native_test_support::frames(native, cx);
            native.update(cx, |view, _, _| assert_eq!(view.held_notes.iter().copied().collect::<Vec<_>>(), vec![60])).unwrap();
        }
        cx.update_window(native.into(), |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke: gpui::Keystroke::parse("a").unwrap() }), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| assert!(view.held_notes.is_empty())).unwrap();
        cx.update_window(native.into(), |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                keystroke: gpui::Keystroke::parse("a").unwrap(), is_held: false, prefer_character_input: false,
            }), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| assert!(view.held_notes.contains(&60), "a new physical press is admitted after release")).unwrap();
    }

    #[gpui::test]
    fn early_key_release_or_keyboard_close_cancels_the_deferred_note(cx: &mut TestAppContext) {
        let (_, editor, windows) = native_windows(cx, 1);
        let native = windows[0];
        cx.update_window(native.into(), |_, window, cx| {
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            window.focus(&view.read(cx).focus.clone(), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        cx.update_window(native.into(), |_, window, cx| {
            let key = gpui::Keystroke::parse("a").unwrap();
            window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent { keystroke: key.clone(), is_held: false, prefer_character_input: false }), cx);
            window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke: key }), cx);
            assert_eq!(performance_key(60, window).state("pressed").unwrap(), serde_json::json!(false));
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| assert!(view.held_notes.is_empty(), "release before delivery cannot start a note")).unwrap();
        cx.update_window(native.into(), |_, window, cx| {
            for key in ["a", "s"] {
                window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                    keystroke: gpui::Keystroke::parse(key).unwrap(), is_held: false, prefer_character_input: false,
                }), cx);
            }
            assert_eq!(performance_key(60, window).state("pressed").unwrap(), serde_json::json!(true));
            assert_eq!(performance_key(62, window).state("pressed").unwrap(), serde_json::json!(true));
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            assert!(view.read(cx).held_notes.is_empty(), "both source admissions precede deferred delivery");
            toolbar_command(editor.downgrade(), view.downgrade(), ToolbarAction::Keyboard)(None, window, cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| {
            assert!(!view.keyboard_open);
            assert!(view.held_notes.is_empty(), "queued input cannot restart the closed keyboard");
            assert!(!view.preview_started);
        }).unwrap();
    }

    #[gpui::test]
    fn window_deactivation_cancels_authored_typing_and_releases_audio(cx: &mut TestAppContext) {
        let (_, _, windows) = native_windows(cx, 1);
        let native = windows[0];
        cx.update_window(native.into(), |_, window, cx| {
            window.activate_window();
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            window.focus(&view.read(cx).focus.clone(), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        let key = cx.update_window(native.into(), |_, window, cx| {
            assert!(window.is_window_active());
            window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                keystroke: gpui::Keystroke::parse("a").unwrap(), is_held: false, prefer_character_input: false,
            }), cx);
            performance_key(60, window)
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| assert!(view.held_notes.contains(&60))).unwrap();
        gpui::VisualTestContext::from_window(native.into(), cx).deactivate_window();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, window, _| {
            assert!(!window.is_window_active());
            assert!(view.keyboard_open, "window loss does not hide the keyboard");
            assert!(view.held_notes.is_empty());
            assert!(!view.preview_started);
            assert_eq!(key.state("pressed").unwrap(), serde_json::json!(false), "source typing cannot outlive its window's active input scope");
        }).unwrap();
        cx.update_window(native.into(), |_, window, cx| {
            window.activate_window();
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            window.focus(&view.read(cx).focus.clone(), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        cx.update_window(native.into(), |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke: gpui::Keystroke::parse("a").unwrap() }), cx);
            window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                keystroke: gpui::Keystroke::parse("a").unwrap(), is_held: false, prefer_character_input: false,
            }), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| assert!(view.held_notes.contains(&60), "reactivation accepts a fresh physical press")).unwrap();
    }

    #[gpui::test]
    fn keyboard_scope_retirement_prevents_queued_note_on(cx: &mut TestAppContext) {
        let (_, _, windows) = native_windows(cx, 1);
        let native = windows[0];
        cx.update_window(native.into(), |_, window, cx| {
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            window.focus(&view.read(cx).focus.clone(), cx);
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        cx.update_window(native.into(), |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::KeyDown(KeyDownEvent {
                keystroke: gpui::Keystroke::parse("s").unwrap(), is_held: false, prefer_character_input: false,
            }), cx);
            let key = performance_key(62, window);
            assert_eq!(key.state("pressed").unwrap(), serde_json::json!(true));
            let view = window.root::<NativePluginWindow>().flatten().unwrap();
            assert!(view.read(cx).held_notes.is_empty(), "audio delivery is still pending");
            // Enter the production window-loss boundary before the queued
            // audio callback. The test above separately drives platform loss.
            view.update(cx, |view, cx| view.cancel_keyboard_input(window, cx));
            assert!(view.read(cx).keyboard_open, "this case cannot rely on the hidden-keyboard guard");
            assert_eq!(key.state("pressed").unwrap(), serde_json::json!(false));
        }).unwrap();
        crate::native_test_support::frames(native, cx);
        native.update(cx, |view, _, _| {
            assert!(view.held_notes.is_empty(), "a retired source cannot deliver a late note-on");
            assert!(!view.preview_started);
        }).unwrap();
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
