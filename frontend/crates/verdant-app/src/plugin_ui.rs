//! Host-backed effect browser, generic parameter editing and insert lifetime.
//! Mixer insertion opens a searchable name/vendor/id catalog with format tabs;
//! selection journals the requested slot with a persistent instance UUID.
//! Parameter controls use native bounds/steps and capture opaque state for undo.
//! A processor is acquired once for a fresh host, then moved with its host
//! across admitted track and master topology changes. Both buses share state
//! capture, parameter transactions and callback-acknowledged host retirement.
//! Every remap pauses and invalidates old automation; playback resumes only
//! after its chain and a timeline for the current document/build are acknowledged.
//! Device changes never enter this path; host startup stays on the worker.

use super::*;
use verdant_audio::engine::{ChainInput, MASTER_BUS};
use verdant_plugins::{InsertHandle, InsertSlot, InsertSpec, InsertStatus, ParamDesc, PluginClient, PluginClientOpts};
use gpui::{Bounds, ElementInputHandler, EntityInputHandler, IntoElement, Pixels, Point, Render,
    SharedString, TextRun, UTF16Selection, WindowBounds, WindowOptions, canvas, div, fill,
    point, prelude::*, px, size};
use std::ops::Range;

#[derive(Clone)]
enum InsertOwner {
    Track(TrackId),
    Master,
}

struct PluginBrowserWindow {
    editor: gpui::WeakEntity<Editor>,
    client: Arc<PluginClient>,
    browser: verdant_plugins::insert::PluginBrowser,
    track: TrackId,
    slot: usize,
    search: verdant_session::TextEditState,
    focus: FocusHandle,
    line: Option<gpui::ShapedLine>,
    bounds: Option<Bounds<Pixels>>,
    scanning: bool,
    message: String,
}

impl Editor {
    pub fn plugin_parameter_row(&self, track: TrackId, slot: usize, param: ParamDesc, busy: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.insert_parameter_row(InsertOwner::Track(track), slot, param, busy, cx)
    }

    pub fn master_plugin_parameter_row(&self, slot: usize, param: ParamDesc, busy: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.insert_parameter_row(InsertOwner::Master, slot, param, busy, cx)
    }

    fn insert_parameter_row(&self, owner: InsertOwner, slot: usize, param: ParamDesc, busy: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let th = self.theme;
        let (inc_owner, dec_owner, click_inc_owner, click_dec_owner) = (owner.clone(), owner.clone(), owner.clone(), owner.clone());
        let (inc, dec, click_inc, click_dec) = (param.clone(), param.clone(), param.clone(), param.clone());
        let valid = verdant_plugins::insert::parameter_editor_value(&param, param.value).is_ok();
        let busy = busy || !valid;
        let steps = if param.step_count > 0 { format!("步长 {}", (param.max - param.min) / param.step_count as f64) }
            else { "连续 · Shift 微调".into() };
        let default = self.live_owned_insert(&owner, slot).and_then(|handle| self.insert_runtime.defaults.get(&handle.key()))
            .and_then(|defaults| defaults.get(&param.id)).copied();
        let reset_param = param.clone();
        let slider_owner = owner.clone();
        let slider_param = param.clone();
        let entity = cx.entity();
        let mut row = div().id(SharedString::from(format!("parameter-{}", param.id))).role(gpui::Role::SpinButton).aria_label(param.name.clone())
            .aria_numeric_value(param.value).aria_min_numeric_value(param.min).aria_max_numeric_value(param.max)
            .aria_value(format!("{} {}", param.value, param.unit))
            .a11y_synthetic_children(move |builder| { if busy { builder.parent_node().set_disabled(); } })
            .flex().flex_col().gap(px(4.)).border_b_1().border_color(th.shade).pb(px(8.))
            .child(div().flex().items_center().justify_between().gap(px(6.))
                .child(div().text_sm().truncate().child(param.name.clone()))
                .child(div().text_xs().font_family("monospace").child(format!("{:.3} {}", param.value, param.unit))))
            .child(div().text_xs().text_color(th.text_secondary).child(if valid { format!("{} … {} · {steps}", param.min, param.max) } else { "插件返回了无效的参数范围".into() }))
            .when(valid && param.max > param.min, |r| {
                r.child(crate::dialogs::horizontal_slider(
                    entity,
                    format!("plug-slider-{slot}-{}", param.id),
                    param.value,
                    param.min,
                    param.max,
                    default,
                    th,
                    move |ed, next_val, commit, cx| {
                        if !busy && commit {
                            let snapped = if slider_param.step_count > 0 {
                                let step = (slider_param.max - slider_param.min) / slider_param.step_count as f64;
                                (slider_param.min + ((next_val - slider_param.min) / step).round() * step).clamp(slider_param.min, slider_param.max)
                            } else {
                                next_val.clamp(slider_param.min, slider_param.max)
                            };
                            ed.set_owned_plugin_param(slider_owner.clone(), slot, slider_param.clone(), snapped, cx);
                        }
                    },
                ))
            })
            .child(div().flex().items_center().gap(px(7.))
                .child(browser_button("decrease", "−", false, th).on_click(cx.listener(move |ed, event: &gpui::ClickEvent, _, cx| {
                    if !busy { ed.step_owned_plugin_param(click_dec_owner.clone(), slot, click_dec.clone(), -1., event.modifiers().shift, cx); }
                })))
                .child(div().flex_1().text_xs().text_color(th.text_secondary).child("拖拽滑块调节 · 双击滑块复位"))
                .child(browser_button("increase", "+", false, th).on_click(cx.listener(move |ed, event: &gpui::ClickEvent, _, cx| {
                    if !busy { ed.step_owned_plugin_param(click_inc_owner.clone(), slot, click_inc.clone(), 1., event.modifiers().shift, cx); }
                }))))
            .on_a11y_action(gpui::AccessibleAction::Increment, crate::view::a11y_listener(cx, move |ed, _, _, cx| {
                if !busy { ed.step_owned_plugin_param(inc_owner.clone(), slot, inc.clone(), 1., false, cx); }
            }))
            .on_a11y_action(gpui::AccessibleAction::Decrement, crate::view::a11y_listener(cx, move |ed, _, _, cx| {
                if !busy { ed.step_owned_plugin_param(dec_owner.clone(), slot, dec.clone(), -1., false, cx); }
            }));
        if let Some(default) = default {
            row = row.child(browser_button("reset-default", format!("恢复默认值 {default}"), false, th)
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if !busy { ed.set_owned_plugin_param(owner.clone(), slot, reset_param.clone(), default, cx); }
                })));
        }
        row.into_any_element()
    }

    /// The mixer and command menu share this browser and its selected slot.
    pub fn open_plugin_browser(&mut self, track: TrackId, slot: usize, cx: &mut Context<Self>) {
        let Some(client) = self.plugins.clone() else { self.notice("插件宿主不可用"); cx.notify(); return; };
        let editor = cx.entity().downgrade();
        let bounds = Bounds::centered(None, size(px(720.), px(580.)), cx);
        if let Err(error) = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default()
        }, move |window, cx| {
            window.set_window_title("插入效果浏览器 — Verdant");
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                PluginBrowserWindow { editor, browser: verdant_plugins::insert::PluginBrowser::new(client.catalog()),
                    client, track, slot, search: verdant_session::TextEditState::new(""), focus,
                    line: None, bounds: None, scanning: false, message: String::new() }
            })
        }) { self.notice(format!("无法打开插件浏览器：{error}")); cx.notify(); }
    }
}

impl PluginBrowserWindow {
    fn edit_search(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut verdant_session::TextEditState)) {
        if !self.search.is_composing() { edit(&mut self.search); }
        cx.notify();
    }

    fn rescan(&mut self, cx: &mut Context<Self>) {
        if self.scanning { return; }
        self.scanning = true;
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            cx.background_spawn(async move { client.scan_default_dirs() }).await;
            let _ = this.update(cx, |view, cx| {
                view.scanning = false;
                view.browser.refresh(view.client.catalog());
                let _ = view.editor.update(cx, |ed, cx| {
                    ed.insert_runtime.retry_missing = true;
                    ed.restore_inserts(cx);
                    cx.notify();
                });
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}

fn browser_button(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, selected: bool, th: Theme) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    div().id(id).role(gpui::Role::Button).aria_label(label.clone()).aria_selected(selected)
        .px(px(9.)).py(px(6.)).rounded(px(4.)).bg(if selected { th.control_pressed } else { th.control })
        .text_sm().child(label)
}

impl Render for PluginBrowserWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = Theme::for_appearance(window.appearance());
        self.browser.query = self.search.text.clone();
        let mut formats = div().flex().gap(px(6.));
        for (label, format) in [("全部", None), ("CLAP", Some(verdant_plugins::PluginFormat::Clap)),
            ("VST3", Some(verdant_plugins::PluginFormat::Vst3)), ("AUv2", Some(verdant_plugins::PluginFormat::Au)),
            ("内置", Some(verdant_plugins::PluginFormat::Builtin))] {
            formats = formats.child(browser_button(label, label, self.browser.format == format, th)
                .on_click(cx.listener(move |view, _, _, cx| { view.browser.format = format; cx.notify(); })));
        }
        let entries = self.browser.entries();
        let count = entries.len();
        let mut rows = div().id("plugin-browser-rows").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(px(6.));
        for (index, desc) in entries.into_iter().enumerate() {
            let (format, id) = (desc.format.tag().to_owned(), desc.id.clone());
            rows = rows.child(div().id(SharedString::from(format!("plugin-row-{index}"))).flex_none().flex().items_center().gap(px(10.))
                .p(px(10.)).rounded(px(5.)).bg(th.control)
                .child(div().flex_1().min_w_0().flex().flex_col().gap(px(3.))
                    .child(div().text_sm().child(desc.name.clone()))
                    .child(div().text_xs().text_color(th.text_secondary).child(format!("{} · {} · {}", desc.vendor, desc.format.tag().to_uppercase(), desc.version))))
                .child(browser_button(SharedString::from(format!("plugin-insert-{index}")), "插入", false, th)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        let track = view.track.clone();
                        let slot = view.slot;
                        match view.editor.update(cx, |ed, cx| ed.add_insert_at(&track, slot, &format, &id, cx)) {
                            Ok(true) => window.remove_window(),
                            Ok(false) => { view.message = "暂时无法插入；请检查主窗口提示后重试".into(); cx.notify(); }
                            Err(_) => { view.message = "工程窗口已关闭".into(); cx.notify(); }
                        }
                    }))));
        }
        if count == 0 { rows = rows.child(div().text_sm().text_color(th.text_secondary).child("没有匹配的插件")); }
        for (path, reason) in self.client.blacklisted() {
            rows = rows.child(div().text_xs().text_color(th.warning).child(format!("无法载入 {}：{reason}", path.display())));
        }
        let entity = cx.entity();
        div().id("plugin-browser").size_full().p(px(18.)).bg(th.surface).text_color(th.text)
            .font_family(crate::theme::system_ui_font(&cx.text_system().all_font_names()))
            .flex().flex_col().gap(px(10.)).key_context("LyricEditor").track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &crate::actions::LyricCancel, window, _| {
                if !view.search.is_composing() { window.remove_window(); }
            }))
            .on_action(cx.listener(|view, _: &crate::actions::Backspace, _, cx| view.edit_search(cx, |s| s.backspace())))
            .on_action(cx.listener(|view, _: &crate::actions::Delete, _, cx| view.edit_search(cx, |s| s.delete())))
            .on_action(cx.listener(|view, _: &crate::actions::Left, _, cx| view.edit_search(cx, |s| s.left())))
            .on_action(cx.listener(|view, _: &crate::actions::Right, _, cx| view.edit_search(cx, |s| s.right())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectLeft, _, cx| view.edit_search(cx, |s| s.select_left())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectRight, _, cx| view.edit_search(cx, |s| s.select_right())))
            .on_action(cx.listener(|view, _: &crate::actions::Home, _, cx| view.edit_search(cx, |s| s.home())))
            .on_action(cx.listener(|view, _: &crate::actions::End, _, cx| view.edit_search(cx, |s| s.end())))
            .on_action(cx.listener(|view, _: &crate::actions::SelectAllText, _, cx| view.edit_search(cx, |s| s.select_all())))
            .on_action(cx.listener(|view, _: &crate::actions::LyricSpace, _, cx| view.edit_search(cx, |s| s.replace_text_in_range(None, " "))))
            .on_action(cx.listener(|view, _: &crate::actions::PasteText, _, cx| {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) { view.edit_search(cx, |s| s.replace_text_in_range(None, &text)); }
            }))
            .child(div().flex().items_center().gap(px(10.)).child(div().flex_1().text_lg().child(format!("插入效果 · 槽位 {}", self.slot + 1)))
                .child(browser_button("rescan", if self.scanning { "扫描中…" } else { "重新扫描" }, false, th)
                    .on_click(cx.listener(|view, _, _, cx| view.rescan(cx)))))
            .child(div().id("plugin-search").role(gpui::Role::TextInput).aria_label("按名称、厂商或标识搜索插件")
                .aria_value(self.search.text.clone()).h(px(36.)).flex_none().bg(th.control).border_1().border_color(th.accent).overflow_hidden()
                .child(canvas(|_, _, _| (), move |bounds, _, window, cx| paint_browser_search(&entity, bounds, window, cx)).size_full()))
            .child(formats).child(div().text_xs().text_color(th.text_secondary).child(format!("{count} 个插件 · 名称 / 厂商 / 格式")))
            .child(rows).child(div().text_xs().text_color(th.warning).child(self.message.clone()))
    }
}

impl EntityInputHandler for PluginBrowserWindow {
    fn text_for_range(&mut self, range: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let (text, adjusted) = self.search.text_for_range_utf16(range); *actual = Some(adjusted); Some(text)
    }
    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.search.range_to_utf16(&self.search.selection), reversed: self.search.reversed })
    }
    fn set_selected_text_range(&mut self, range: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        self.search.selection = self.search.range_from_utf16(&range); self.search.reversed = false;
        window.notify_text_selection_changed(&self.focus); cx.notify();
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> { Some(self.search.text.encode_utf16().count()) }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> { self.search.marked.as_ref().map(|range| self.search.range_to_utf16(range)) }
    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) { self.search.unmark_text(); window.notify_text_input_changed(&self.focus); cx.notify(); }
    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.search.replace_text_in_range(range, text); window.notify_text_input_changed(&self.focus); cx.notify();
    }
    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, selected: Option<Range<usize>>, window: &mut Window, cx: &mut Context<Self>) {
        self.search.replace_and_mark_text_in_range(range, text, selected); window.notify_text_input_changed(&self.focus); cx.notify();
    }
    fn bounds_for_range(&mut self, range: Range<usize>, bounds: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let line = self.line.as_ref()?; let range = self.search.range_from_utf16(&range);
        Some(Bounds::from_corners(point(bounds.left() + px(6.) + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + px(6.) + line.x_for_index(range.end), bounds.bottom())))
    }
    fn character_index_for_point(&mut self, point: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let local = self.bounds?.localize(&point)?;
        Some(self.search.offset_to_utf16(self.line.as_ref()?.closest_index_for_x(local.x - px(6.))))
    }
}

fn paint_browser_search(entity: &gpui::Entity<PluginBrowserWindow>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let view = entity.read(cx);
    let (state, focus) = (view.search.clone(), view.focus.clone());
    let th = Theme::for_appearance(window.appearance());
    window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
    let placeholder = state.text.is_empty();
    let shown: SharedString = if placeholder { "搜索名称、厂商或标识…".into() } else { state.text.clone().into() };
    let base = TextRun { len: shown.len(), font: window.text_style().font(), color: if placeholder { th.text_tertiary.into() } else { th.text.into() },
        background_color: None, underline: None, strikethrough: None };
    let runs = if let Some(marked) = state.marked.as_ref().filter(|_| !placeholder) {
        vec![TextRun { len: marked.start, ..base.clone() }, TextRun { len: marked.end - marked.start,
            underline: Some(gpui::UnderlineStyle { color: Some(th.accent.into()), thickness: px(1.), wavy: false }), ..base.clone() },
            TextRun { len: shown.len() - marked.end, ..base }]
    } else { vec![base] };
    let line = window.text_system().shape_line(shown, px(15.), &runs, None);
    let origin = point(bounds.left() + px(6.), bounds.top() + px(8.));
    if !placeholder && !state.selection.is_empty() {
        let start = line.x_for_index(state.selection.start); let end = line.x_for_index(state.selection.end);
        window.paint_quad(fill(Bounds::new(point(origin.x + start, origin.y), size(end - start, px(20.))), crate::theme::with_alpha(th.accent, 0.3)));
    }
    let _ = line.paint(origin, px(20.), gpui::TextAlign::Left, None, window, cx);
    if focus.is_focused(window) && state.selection.is_empty() {
        let x = if placeholder { px(0.) } else { line.x_for_index(state.cursor()) };
        window.paint_quad(fill(Bounds::new(point(origin.x + x, origin.y), size(px(1.), px(20.))), th.accent));
    }
    entity.update(cx, |view, _| { view.line = Some(line); view.bounds = Some(bounds); });
}

fn encoded(specs: &[InsertSpec]) -> Vec<serde_json::Value> {
    specs.iter().map(|s| serde_json::to_value(s).expect("serializable InsertSpec")).collect()
}

#[derive(Default)]
pub(crate) struct InsertRuntime {
    tracks: Vec<AppliedTrack>,
    master: AppliedMaster,
    pending: Option<PreparedInserts>,
    retired: Vec<(u64, Vec<Arc<InsertHandle>>)>,
    retry_missing: bool,
    hold: Option<TransportHold>,
    defaults: HashMap<verdant_plugins::InsertKey, HashMap<String, f64>>,
    open_instance: Option<(InsertOwner, String)>,
}

struct AppliedTrack {
    id: TrackId,
    specs: Vec<InsertSpec>,
    // Some only when native serialization differs from the persisted restore
    // blob (notably an empty/default state). This is the admitted live baseline.
    live_states: Vec<Option<String>>,
    processors: Vec<Option<usize>>,
    padding: u32,
}

#[derive(Default)]
struct AppliedMaster {
    specs: Vec<InsertSpec>,
    live_states: Vec<Option<String>>,
    processors: Vec<Option<usize>>,
}

struct PreparedInserts {
    tracks: Vec<AppliedTrack>,
    master: AppliedMaster,
    slots: HashMap<TrackId, Vec<InsertSlot>>,
    master_slots: Vec<InsertSlot>,
    layout: Vec<Vec<ChainInput>>,
    master_layout: Vec<ChainInput>,
    fresh: Vec<Arc<InsertHandle>>,
}

struct TransportHold {
    position: u64,
    playing: bool,
    chain_revision: u64,
    timeline: Option<InsertTimelineReceipt>,
}

struct InsertTimelineReceipt {
    revision: u64,
    generation: u64,
    project_revision: Revision,
}

impl TransportHold {
    fn ready(&self, retired_chain: u64, applied_timeline: u64, generation: u64, project_revision: Revision) -> bool {
        self.chain_revision <= retired_chain && self.timeline.as_ref().is_some_and(|timeline|
            timeline.generation == generation && timeline.project_revision == project_revision && timeline.revision <= applied_timeline)
    }

    fn resume(self, audio: &Control) {
        audio.seek(self.position);
        if self.playing { audio.play(); }
    }
}

impl InsertRuntime {
    fn matches_project(&self, project: &Project) -> bool {
        self.tracks.len() == project.tracks.len() && self.master.specs == super::project_audio::master_insert_specs(project)
            && self.tracks.iter().zip(&project.tracks).all(|(applied, desired)|
                applied.id == desired.id && applied.specs == verdant_plugins::project::read_inserts(&desired.unknown))
    }

    fn hold_transport(&mut self, audio: &mut Control, sample_rate: u32, timeline_generation: &mut u64) -> bool {
        let fresh = self.hold.is_none();
        if fresh {
            self.hold = Some(TransportHold { position: audio.position(), playing: audio.shared.playback_requested(),
                chain_revision: audio.retired_chain_revision(), timeline: None });
        }
        // Keep the last submitted chain receipt until a remap succeeds. If a
        // rejected/stale build is abandoned and the document returns to that
        // topology, its replacement timeline can still release the hold.
        self.hold.as_mut().expect("held transport").timeline = None;
        audio.pause();
        audio.set_timeline(Timeline::new(sample_rate, Vec::new()));
        *timeline_generation += 1;
        fresh
    }

    /// An armed hold whose matching timeline receipt was dropped while a
    /// build was still loading needs one fresh timeline to release. When the
    /// document is already back on the installed topology the unchanged
    /// restore path is the only remaining trigger for that rebuild.
    fn needs_timeline_receipt(&self) -> bool {
        self.hold.as_ref().is_some_and(|hold| hold.timeline.is_none())
    }
}

fn retire(client: &PluginClient, handles: Vec<Arc<InsertHandle>>) {
    for handle in handles { client.remove(handle.key()); drop(handle); }
}

fn same_plugin(a: &InsertSpec, b: &InsertSpec) -> bool { a.format == b.format && a.id == b.id && (a.instance_id.is_empty() || b.instance_id.is_empty() || a.instance_id == b.instance_id) }

fn slot_matches(before: &[InsertSpec], after: &[InsertSpec]) -> Vec<Option<usize>> {
    let mut result = vec![None; after.len()];
    let mut used = vec![false; before.len()];
    // Exact matches win across the complete chain before matching changed
    // slots. This preserves identity when duplicate plugins are reordered.
    for pass in 0..3 {
        for (target, spec) in after.iter().enumerate() {
            if result[target].is_some() { continue; }
            if let Some(source) = before.iter().enumerate().position(|(i, old)| !used[i] && match pass {
                0 => old == spec,
                1 => same_plugin(old, spec) && old.state_b64 == spec.state_b64,
                _ => same_plugin(old, spec),
            }) { used[source] = true; result[target] = Some(source); }
        }
    }
    result
}

fn copied_slot(slot: &InsertSlot, spec: &InsertSpec) -> InsertSlot {
    match slot {
        InsertSlot::Live(handle) => InsertSlot::Live(handle.clone()),
        InsertSlot::Missing { reason, .. } => InsertSlot::Missing { spec: spec.clone(), reason: reason.clone() },
    }
}

fn restore_slot(client: Option<&Arc<PluginClient>>, spec: InsertSpec)
    -> (InsertSlot, std::option::Option<std::string::String>, Option<Box<dyn verdant_audio::Processor>>)
{
    let restored = client.map(|client| client.restore(&spec)).unwrap_or_else(||
        InsertSlot::Missing { spec: spec.clone(), reason: "插件宿主不可用".into() });
    let InsertSlot::Live(handle) = restored else { return (restored, None, None) };
    let result = handle.get_state_b64().map_err(|error| format!("无法读取插件初始状态：{error}"))
        .and_then(|state| handle.processor().map(|processor| (state, processor)).ok_or_else(|| "无法获取插件音频处理器".to_string()));
    match result {
        Ok((state, processor)) => (InsertSlot::Live(handle), (state != spec.state_b64).then_some(state), Some(processor)),
        Err(reason) => {
            if let Some(client) = client { client.remove(handle.key()); }
            (InsertSlot::Missing { spec, reason }, None, None)
        }
    }
}

fn check_removed_states(specs: &[InsertSpec], live_states: &[Option<String>], slots: &[InsertSlot],
    retained: &HashSet<verdant_plugins::InsertKey>) -> Result<(), String>
{
    for (index, (slot, spec)) in slots.iter().zip(specs).enumerate() {
        let InsertSlot::Live(handle) = slot else { continue };
        if retained.contains(&handle.key()) || !matches!(handle.status(), InsertStatus::Running) { continue; }
        match handle.get_state_b64() {
            Ok(state) if state == live_states[index].as_deref().unwrap_or(&spec.state_b64) => {},
            Ok(_) => return Err("插件仍有未保存的原生窗口修改；请先保存插件状态再重试".into()),
            Err(error) => return Err(format!("插件状态读取失败，保留原音频链：{error}")),
        }
    }
    Ok(())
}

fn capture_chain(specs: &[InsertSpec], live_states: &[Option<String>], slots: &[InsertSlot], full_snapshot: bool)
    -> Result<(Vec<InsertSpec>, Vec<Option<String>>), String>
{
    let mut captured = specs.to_vec();
    let mut states = live_states.to_vec();
    for (index, slot) in slots.iter().enumerate() {
        let InsertSlot::Live(handle) = slot else { continue };
        if !matches!(handle.status(), InsertStatus::Running) { continue; }
        let state = handle.get_state_b64().map_err(|error| format!("插件状态未保存：{error}"))?;
        let admitted = if full_snapshot { specs[index].state_b64.as_str() }
            else { live_states[index].as_deref().unwrap_or(&specs[index].state_b64) };
        if state != admitted { captured[index].state_b64 = state; states[index] = None; }
    }
    Ok((captured, states))
}

impl Editor {
    pub(super) fn start_plugins(&mut self, path: Option<&Path>, cx: &mut Context<Self>) {
        match PluginClient::new(PluginClientOpts {
            plughost_path: path.map(Path::to_path_buf),
            cache_dir: crate::files::state_dir().join("plugin-cache"),
            sample_rate: self.audio_info.sample_rate as f64,
            max_frames: verdant_audio::device::MAX_BLOCK,
        }) {
            Ok(client) => { self.plugins = Some(client); self.rescan_plugins(cx); }
            Err(e) => self.notice(format!("插件宿主不可用：{e}")),
        }
    }

    pub(super) fn native_bindings(&self)->super::project_audio::NativeBindings{
        let tracks=self.project().tracks.iter().map(|track|{
            let Some(applied)=self.insert_runtime.tracks.iter().find(|applied|applied.id==track.id)else{return Vec::new();};
            applied.specs.iter().enumerate().map(|(slot,spec)|super::project_audio::NativeBinding{instance_id:spec.instance_id.clone(),processor:applied.processors[slot],bypass:spec.bypass,handle:self.live_insert(&track.id,slot)}).collect()
        }).collect();
        let master=self.insert_runtime.master.specs.iter().enumerate().map(|(slot,spec)|super::project_audio::NativeBinding{instance_id:spec.instance_id.clone(),processor:self.insert_runtime.master.processors[slot],bypass:spec.bypass,handle:self.live_master_insert(slot)}).collect();
        super::project_audio::NativeBindings{tracks,master}
    }

    pub fn insert_specs(&self, track: &TrackId) -> Vec<InsertSpec> {
        self.project().track(track).map(|t| verdant_plugins::project::read_inserts(&t.unknown)).unwrap_or_default()
    }

    pub fn master_insert_specs(&self) -> Vec<InsertSpec> {
        super::project_audio::master_insert_specs(self.project())
    }

    pub fn master_insert_slots(&self) -> &[InsertSlot] { &self.master_inserts }

    fn owned_insert_specs(&self, owner: &InsertOwner) -> Vec<InsertSpec> {
        match owner { InsertOwner::Track(track) => self.insert_specs(track), InsertOwner::Master => self.master_insert_specs() }
    }

    fn live_owned_insert(&self, owner: &InsertOwner, slot: usize) -> Option<Arc<InsertHandle>> {
        match owner { InsertOwner::Track(track) => self.live_insert(track, slot), InsertOwner::Master => self.live_master_insert(slot) }
    }

    fn insert_commands(&self, owner: &InsertOwner, specs: &[InsertSpec]) -> Result<Vec<Command>, String> {
        match owner {
            InsertOwner::Track(track) => Ok(vec![Command::SetTrackInserts { track: track.clone(), inserts: encoded(specs) }]),
            InsertOwner::Master => {
                use verdant_domain::automation::{AutomationDocument, AutomationOwner, AutomationTarget, AUTOMATION_KEY, MASTER_INSERTS_KEY};
                let mut document = AutomationDocument::from_project(self.project()).map_err(|error| error.to_string())?;
                let before = document.lanes.len();
                document.lanes.retain(|lane| !matches!(&lane.target, AutomationTarget::PluginParameter {
                    owner: AutomationOwner::Master, instance_id, ..
                } if !specs.iter().any(|spec| &spec.instance_id == instance_id)));
                let mut commands = vec![Command::SetProjectExtension { key: MASTER_INSERTS_KEY.into(),
                    value: (!specs.is_empty()).then(|| serde_json::Value::Array(encoded(specs))) }];
                if before != document.lanes.len() {
                    commands.push(Command::SetProjectExtension { key: AUTOMATION_KEY.into(),
                        value: document.extension_value().map_err(|error| error.to_string())? });
                }
                Ok(commands)
            }
        }
    }

    pub fn plugin_catalog(&self) -> Vec<verdant_plugins::PluginDesc> {
        self.plugins.as_ref().map(|c| c.catalog()).unwrap_or_default()
    }

    pub fn rescan_plugins(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.plugins.clone() else { self.notice("插件宿主不可用"); cx.notify(); return };
        cx.spawn(async move |this, cx| {
            let summary = cx.background_spawn(async move { client.scan_default_dirs() }).await;
            let _ = this.update(cx, |ed, cx| {
                ed.notice(format!("插件扫描完成：{} 个可用插件", ed.plugin_catalog().len()));
                let _ = summary;
                ed.insert_runtime.retry_missing = true;
                ed.restore_inserts(cx);
                cx.notify();
            });
        }).detach();
    }

    pub fn restore_inserts(&mut self, cx: &mut Context<Self>) {
        self.reclaim_inserts(cx);
        let desired: Vec<_> = self.project().tracks.iter()
            .map(|t| (t.id.clone(), verdant_plugins::project::read_inserts(&t.unknown))).collect();
        let desired_master = self.master_insert_specs();
        if let Some(prepared) = self.insert_runtime.pending.take() {
            if prepared.master.specs == desired_master && prepared.tracks.iter().map(|t| (&t.id, &t.specs)).eq(desired.iter().map(|(id, specs)| (id, specs))) {
                self.install_inserts(prepared, cx);
                return;
            }
            if let Some(client) = self.plugins.clone() {
                cx.background_spawn(async move { retire(&client, prepared.fresh); }).detach();
            }
            self.inserts_loading = false;
        }
        if self.inserts_loading { self.insert_specs_applied.clear(); return; }
        let unchanged = self.insert_runtime.master.specs == desired_master && self.insert_runtime.tracks.iter().map(|t| (&t.id, &t.specs))
            .eq(desired.iter().map(|(id, specs)| (id, specs)));
        let latencies: Vec<u32> = desired.iter().map(|(id, specs)| self.inserts.get(id).map_or(0, |slots|
            slots.iter().zip(specs).filter_map(|(slot, spec)| match slot {
                InsertSlot::Live(h) if !spec.bypass && matches!(h.status(), InsertStatus::Running) => Some(h.latency_samples()),
                _ => None,
            }).fold(0u32, u32::saturating_add))).collect();
        let maximum = latencies.iter().copied().max().unwrap_or(0);
        let padding_unchanged = unchanged && self.insert_runtime.tracks.iter().zip(&latencies)
            .all(|(track, latency)| track.padding == maximum - latency);
        let retry_needed = self.insert_runtime.retry_missing && self.inserts.values().flatten().chain(&self.master_inserts).any(|slot| match slot {
            InsertSlot::Missing { .. } => true,
            InsertSlot::Live(handle) => !matches!(handle.status(), InsertStatus::Running),
        });
        if unchanged && padding_unchanged && !retry_needed {
            self.insert_runtime.retry_missing = false;
            self.mark_inserts_applied();
            if self.insert_runtime.needs_timeline_receipt() {
                // The matching receipt was dropped while a stale build was
                // still loading; republish a timeline for the held transport.
                self.rebuild_timeline(cx);
            }
            return;
        }
        if !self.plugin_busy.is_empty() {
            self.insert_specs_applied.clear();
            return;
        }
        let retry_missing = std::mem::take(&mut self.insert_runtime.retry_missing);
        let mut retained = HashSet::new();
        let mut tracks = Vec::with_capacity(desired.len());
        let mut slots = HashMap::new();
        let mut layout = Vec::with_capacity(desired.len());
        let mut restores = Vec::new();
        for (id, specs) in &desired {
            let old = self.insert_runtime.tracks.iter().enumerate().find(|(_, t)| &t.id == id);
            let matches = old.map(|(_, t)| slot_matches(&t.specs, specs)).unwrap_or_else(|| vec![None; specs.len()]);
            let mut output_slots = Vec::with_capacity(specs.len());
            let mut live_states = Vec::with_capacity(specs.len());
            let mut processors = Vec::with_capacity(specs.len());
            let mut inputs = Vec::new();
            for (slot, spec) in specs.iter().enumerate() {
                let previous = old.and_then(|(track, t)| matches[slot].map(|i| (track, t, i)))
                    .and_then(|(track, t, i)| self.inserts.get(id)?.get(i).map(|s| (track, t, i, s)));
                let keep = previous.is_some_and(|(_, t, i, previous)| t.specs[i].state_b64 == spec.state_b64 && match previous {
                    InsertSlot::Live(h) => !retry_missing || matches!(h.status(), InsertStatus::Running),
                    InsertSlot::Missing { .. } => !retry_missing,
                });
                if keep {
                    let (track, old, index, previous) = previous.expect("matched slot");
                    if let InsertSlot::Live(handle) = previous {
                        retained.insert(handle.key());
                        live_states.push(old.live_states[index].clone());
                        processors.push(Some(inputs.len()));
                        inputs.push(ChainInput::Retained { track, processor: old.processors[index].expect("live processor position") });
                    } else { live_states.push(None); processors.push(None); }
                    output_slots.push(copied_slot(previous, spec));
                } else {
                    // The background task fills this missing slot and appends
                    // its new processor at the pre-recorded position below.
                    processors.push(None);
                    live_states.push(None);
                    output_slots.push(InsertSlot::Missing { spec: spec.clone(), reason: "正在恢复插件".into() });
                    restores.push((tracks.len(), slot, spec.clone()));
                }
            }
            slots.insert(id.clone(), output_slots);
            layout.push(inputs);
            tracks.push(AppliedTrack { id: id.clone(), specs: specs.clone(), live_states, processors, padding: 0 });
        }
        let old_master = &self.insert_runtime.master;
        let matches = slot_matches(&old_master.specs, &desired_master);
        let mut master = AppliedMaster { specs: desired_master.clone(), ..Default::default() };
        let mut master_slots = Vec::with_capacity(desired_master.len());
        let mut master_layout = Vec::new();
        for (slot, spec) in desired_master.iter().enumerate() {
            let previous = matches[slot].and_then(|index| self.master_inserts.get(index).map(|previous| (index, previous)));
            let keep = previous.is_some_and(|(index, previous)| old_master.specs[index].state_b64 == spec.state_b64 && match previous {
                InsertSlot::Live(handle) => !retry_missing || matches!(handle.status(), InsertStatus::Running),
                InsertSlot::Missing { .. } => !retry_missing,
            });
            if keep {
                let (index, previous) = previous.expect("matched master slot");
                if let InsertSlot::Live(handle) = previous {
                    retained.insert(handle.key());
                    master.live_states.push(old_master.live_states[index].clone());
                    master.processors.push(Some(master_layout.len()));
                    master_layout.push(ChainInput::Retained { track: MASTER_BUS,
                        processor: old_master.processors[index].expect("live master processor position") });
                } else { master.live_states.push(None); master.processors.push(None); }
                master_slots.push(copied_slot(previous, spec));
            } else {
                master.live_states.push(None);
                master.processors.push(None);
                master_slots.push(InsertSlot::Missing { spec: spec.clone(), reason: "正在恢复插件".into() });
                restores.push((MASTER_BUS, slot, spec.clone()));
            }
        }
        // A necessary removal/reload must not discard a native-editor edit
        // that has not entered the journal. Ordinary add/reorder/rescan paths
        // retain those live owners and never read stale persisted state into them.
        let capture = self.insert_runtime.tracks.iter().try_for_each(|track| {
            check_removed_states(&track.specs, &track.live_states,
                self.inserts.get(&track.id).map(Vec::as_slice).unwrap_or(&[]), &retained)
        }).and_then(|()| check_removed_states(&old_master.specs, &old_master.live_states, &self.master_inserts, &retained));
        if let Err(error) = capture {
            self.notice(error);
            self.insert_specs_applied.clear();
            cx.notify();
            return;
        }
        self.hold_insert_transport(cx);
        self.insert_gen += 1;
        self.inserts_loading = true;
        self.insert_specs_applied.clear();
        let generation = self.insert_gen;
        let client = self.plugins.clone();
        let old_padding: Vec<_> = self.insert_runtime.tracks.iter().map(|t| (t.id.clone(), t.padding,
            t.processors.iter().flatten().count())).collect();
        cx.spawn(async move |this, cx| {
            let build_client = client.clone();
            let prepared = cx.background_spawn(async move {
                let mut fresh = Vec::new();
                let mut new_processors = HashMap::new();
                for (track, slot, spec) in restores {
                    let (restored, state, processor) = restore_slot(build_client.as_ref(), spec);
                    if let InsertSlot::Live(handle) = &restored { fresh.push(handle.clone()); }
                    if let Some(processor) = processor { new_processors.insert((track, slot), processor); }
                    if track == MASTER_BUS {
                        master.live_states[slot] = state;
                        master_slots[slot] = restored;
                    } else {
                        tracks[track].live_states[slot] = state;
                        slots.get_mut(&tracks[track].id).expect("prepared track")[slot] = restored;
                    }
                }
                // Rebuild only the vector of ownership coordinates; existing
                // processors themselves stay on the callback until remapped.
                for (track, applied) in tracks.iter_mut().enumerate() {
                    let mut retained = std::mem::take(&mut layout[track]).into_iter();
                    for slot in 0..applied.specs.len() {
                        if let Some(processor) = new_processors.remove(&(track, slot)) {
                            applied.processors[slot] = Some(layout[track].len());
                            layout[track].push(ChainInput::New(processor));
                        } else if applied.processors[slot].is_some() {
                            applied.processors[slot] = Some(layout[track].len());
                            layout[track].push(retained.next().expect("retained slot coordinate"));
                        }
                    }
                }
                let mut retained_master = std::mem::take(&mut master_layout).into_iter();
                for slot in 0..master.specs.len() {
                    if let Some(processor) = new_processors.remove(&(MASTER_BUS, slot)) {
                        master.processors[slot] = Some(master_layout.len());
                        master_layout.push(ChainInput::New(processor));
                    } else if master.processors[slot].is_some() {
                        master.processors[slot] = Some(master_layout.len());
                        master_layout.push(retained_master.next().expect("retained master slot coordinate"));
                    }
                }
                let latencies: Vec<u32> = tracks.iter().map(|t| slots[&t.id].iter().zip(&t.specs)
                    .filter_map(|(slot, spec)| match slot {
                        InsertSlot::Live(h) if !spec.bypass && matches!(h.status(), InsertStatus::Running) => Some(h.latency_samples()),
                        _ => None,
                    }).fold(0u32, u32::saturating_add)).collect();
                let maximum = latencies.iter().copied().max().unwrap_or(0);
                for (track, applied) in tracks.iter_mut().enumerate() {
                    applied.padding = maximum - latencies[track];
                    if applied.padding > 0 {
                        let old = old_padding.iter().enumerate().find(|(_, (id, padding, _))|
                            id == &applied.id && *padding == applied.padding);
                        if let Some((source, (_, _, processor))) = old {
                            layout[track].push(ChainInput::Retained { track: source, processor: *processor });
                        } else { layout[track].push(ChainInput::New(Box::new(verdant_plugins::DelayPad::new(applied.padding)))); }
                    }
                }
                PreparedInserts { tracks, master, slots, master_slots, layout, master_layout, fresh }
            }).await;
            let mut prepared = Some(prepared);
            let _ = this.update(cx, |ed, cx| {
                ed.inserts_loading = false;
                let current = ed.project().tracks.iter().map(|t| (&t.id, verdant_plugins::project::read_inserts(&t.unknown))).collect::<Vec<_>>();
                if ed.insert_gen == generation && ed.master_insert_specs() == desired_master && current.iter().map(|(id, specs)| (*id, specs))
                    .eq(desired.iter().map(|(id, specs)| (id, specs))) {
                    ed.install_inserts(prepared.take().expect("prepared install"), cx);
                } else {
                    ed.restore_inserts(cx);
                }
                cx.notify();
            });
            if let (Some(client), Some(prepared)) = (client, prepared) {
                cx.background_spawn(async move { retire(&client, prepared.fresh); }).await;
            }
        }).detach();
    }

    fn install_inserts(&mut self, mut prepared: PreparedInserts, cx: &mut Context<Self>) {
        // Builds/retries may have allowed another timeline preparation to
        // start against the old coordinates. Invalidate it before publishing
        // any track/master, same-track, host-replacement or padding remap.
        self.hold_insert_transport(cx);
        match self.audio.remap_chains_with_master(std::mem::take(&mut prepared.layout), std::mem::take(&mut prepared.master_layout)) {
            Err((layout, master_layout)) => {
                prepared.layout = layout;
                prepared.master_layout = master_layout;
                self.insert_runtime.pending = Some(prepared);
                self.inserts_loading = true;
                self.insert_specs_applied.clear();
                self.notice("音频链替换队列已满；保留当前插件，等待重试");
            }
            Ok(revision) => {
                let panel_key = self.insert_panel.as_ref().and_then(|(track, slot)| self.live_insert(track, *slot)).map(|h| h.key());
                let master_panel_key = self.master_insert_panel.and_then(|slot| self.live_master_insert(slot)).map(|handle| handle.key());
                let active: HashSet<_> = prepared.slots.values().flatten().chain(&prepared.master_slots).filter_map(|s| match s {
                    InsertSlot::Live(h) => Some(h.key()), _ => None,
                }).collect();
                let old = std::mem::replace(&mut self.inserts, prepared.slots);
                let old_master = std::mem::replace(&mut self.master_inserts, prepared.master_slots);
                let removed = old.into_values().flatten().chain(old_master).filter_map(|s| match s {
                    InsertSlot::Live(h) if !active.contains(&h.key()) => Some(h), _ => None,
                }).collect::<Vec<_>>();
                if !removed.is_empty() { self.insert_runtime.retired.push((revision, removed)); }
                self.insert_runtime.tracks = prepared.tracks;
                self.insert_runtime.master = prepared.master;
                for track in &self.insert_runtime.tracks {
                    for (slot, spec) in self.inserts[&track.id].iter().zip(&track.specs) {
                        if let InsertSlot::Live(handle) = slot { handle.set_bypass(spec.bypass); }
                    }
                }
                for (slot, spec) in self.master_inserts.iter().zip(&self.insert_runtime.master.specs) {
                    if let InsertSlot::Live(handle) = slot { handle.set_bypass(spec.bypass); }
                }
                self.plugin_params.retain(|key, _| active.contains(key));
                self.insert_runtime.defaults.retain(|key, _| active.contains(key));
                self.plugin_busy.retain(|key| active.contains(key));
                self.plugin_counters.retain(|key, _| active.contains(key));
                let opening = self.insert_runtime.open_instance.take();
                if let Some((owner, instance)) = opening.as_ref() {
                    self.insert_panel = match owner {
                        InsertOwner::Track(id) => self.insert_runtime.tracks.iter().find(|track| &track.id == id)
                            .and_then(|track| track.specs.iter().position(|spec| &spec.instance_id == instance).map(|slot| (track.id.clone(), slot))),
                        InsertOwner::Master => None,
                    };
                    self.master_insert_panel = match owner {
                        InsertOwner::Master => self.insert_runtime.master.specs.iter().position(|spec| &spec.instance_id == instance),
                        InsertOwner::Track(_) => None,
                    };
                } else if let Some(key) = panel_key {
                    self.insert_panel = self.inserts.iter().find_map(|(track, slots)| slots.iter().position(|slot|
                        matches!(slot, InsertSlot::Live(h) if h.key() == key)).map(|slot| (track.clone(), slot)));
                } else if self.insert_panel.as_ref().is_some_and(|(track, slot)| self.inserts.get(track).is_none_or(|slots| *slot >= slots.len())) {
                    self.insert_panel = None;
                }
                if opening.is_none() {
                    if let Some(key) = master_panel_key {
                        self.master_insert_panel = self.master_inserts.iter().position(|slot| matches!(slot, InsertSlot::Live(handle) if handle.key() == key));
                    } else if self.master_insert_panel.is_some_and(|slot| slot >= self.master_inserts.len()) {
                        self.master_insert_panel = None;
                    }
                }
                self.inserts_loading = false;
                if let Some(hold) = &mut self.insert_runtime.hold {
                    hold.chain_revision = revision;
                    hold.timeline = None;
                }
                self.mark_inserts_applied();
                self.rebuild_timeline(cx);
                if let Some((track, slot)) = self.insert_panel.clone() {
                    if self.live_insert(&track, slot).is_some_and(|handle| !self.plugin_params.contains_key(&handle.key())) {
                        self.show_insert_params(track, slot, cx);
                    }
                }
                if let Some(slot) = self.master_insert_panel {
                    if self.live_master_insert(slot).is_some_and(|handle| !self.plugin_params.contains_key(&handle.key())) {
                        self.show_master_insert_params(slot, cx);
                    }
                }
            }
        }
        cx.notify();
    }

    fn mark_inserts_applied(&mut self) {
        if self.inserts_loading || self.insert_runtime.retry_missing || !self.insert_runtime.retired.is_empty() || self.insert_runtime.hold.is_some() {
            self.insert_specs_applied.clear();
        } else {
            self.insert_specs_applied = self.insert_runtime.tracks.iter().map(|t| (t.id.clone(), t.specs.clone())).collect();
        }
    }

    fn reclaim_inserts(&mut self, cx: &mut Context<Self>) {
        let revision = self.audio.retired_chain_revision();
        let mut handles = Vec::new();
        self.insert_runtime.retired.retain_mut(|(retired, old)| {
            if *retired <= revision { handles.append(old); false } else { true }
        });
        if let Some(client) = self.plugins.clone() {
            if !handles.is_empty() { cx.background_spawn(async move { retire(&client, handles); }).detach(); }
        }
        let ready = !self.inserts_loading && self.insert_runtime.hold.as_ref().is_some_and(|hold|
            hold.ready(revision, self.audio.applied_timeline_revision(), self.timeline_gen, self.revision())
                && self.insert_runtime.matches_project(self.project()));
        if ready {
            self.insert_runtime.hold.take().expect("ready transport").resume(&self.audio);
        }
    }

    /// Track channel settings are published before restoration, so track-ID
    /// changes need the hold early. Every insert rebuild/remap also holds the
    /// transport independently, including edits within a track or the master.
    pub(super) fn prepare_insert_topology(&mut self, cx: &mut Context<Self>) {
        let changed = !self.insert_runtime.tracks.iter().map(|t| &t.id).eq(self.project().tracks.iter().map(|t| &t.id));
        if !changed { return; }
        self.hold_insert_transport(cx);
    }

    fn hold_insert_transport(&mut self, cx: &mut Context<Self>) {
        if self.insert_runtime.hold_transport(&mut self.audio, self.audio_info.sample_rate, &mut self.timeline_gen) {
            self.notice("音频链正在重新对齐，短暂暂停以保留插件状态和自动化");
        }
        cx.notify();
    }

    pub(super) fn insert_timeline_ready(&mut self, revision: u64, cx: &mut Context<Self>) {
        if !self.inserts_loading && self.insert_runtime.hold.is_some() && self.insert_runtime.matches_project(self.project()) {
            let project_revision = self.revision();
            if let Some(hold) = &mut self.insert_runtime.hold {
                hold.timeline = Some(InsertTimelineReceipt { revision, generation: self.timeline_gen, project_revision });
            }
        }
        self.reclaim_inserts(cx);
        self.mark_inserts_applied();
    }

    pub(super) fn insert_transport_ready(&mut self, cx: &mut Context<Self>) -> bool {
        self.reclaim_inserts(cx);
        if self.inserts_loading || self.insert_runtime.hold.is_some() {
            self.notice("音频链正在重新对齐，请稍后播放"); cx.notify(); return false;
        }
        true
    }

    pub(super) fn stop_insert_transport(&mut self) {
        if let Some(hold) = &mut self.insert_runtime.hold { hold.playing = false; hold.position = 0; }
    }

    pub(super) fn seek_insert_transport(&mut self, frame: u64) {
        if let Some(hold) = &mut self.insert_runtime.hold { hold.position = frame; }
    }

    pub fn commit_inserts(&mut self, track: &TrackId, specs: Vec<InsertSpec>, label: &str, cx: &mut Context<Self>) {
        self.commit_owned_inserts(&InsertOwner::Track(track.clone()), specs, label, cx);
    }

    fn commit_owned_inserts(&mut self, owner: &InsertOwner, specs: Vec<InsertSpec>, label: &str, cx: &mut Context<Self>) -> bool {
        match self.insert_commands(owner, &specs) {
            Ok(commands) => self.commit(self.revision(), CommandBatch { label: label.into(), commands }, cx),
            Err(error) => { self.notice(format!("插件配置未提交：{error}")); cx.notify(); false }
        }
    }

    pub fn add_insert(&mut self, track: &TrackId, format: &str, id: &str, cx: &mut Context<Self>) {
        self.add_insert_at(track, self.insert_specs(track).len(), format, id, cx);
    }

    pub fn add_insert_at(&mut self, track: &TrackId, slot: usize, format: &str, id: &str, cx: &mut Context<Self>) -> bool {
        if self.project().track(track).is_none() { self.notice("目标轨道已移除"); cx.notify(); return false; }
        self.add_owned_insert_at(InsertOwner::Track(track.clone()), slot, format, id, cx)
    }

    pub fn add_master_insert(&mut self, format: &str, id: &str, cx: &mut Context<Self>) {
        self.add_owned_insert_at(InsertOwner::Master, self.master_insert_specs().len(), format, id, cx);
    }

    fn add_owned_insert_at(&mut self, owner: InsertOwner, slot: usize, format: &str, id: &str, cx: &mut Context<Self>) -> bool {
        if !self.flush_insert_states(cx) { return false; }
        let browser = verdant_plugins::insert::PluginBrowser::new(self.plugin_catalog());
        let mut specs = self.owned_insert_specs(&owner);
        match browser.insert(&mut specs, slot, format, id) {
            Ok(instance) => {
                let name = specs[slot].name.clone();
                let committed = self.commit_owned_inserts(&owner, specs, &format!("插入 {name}"), cx);
                if committed {
                    self.insert_runtime.open_instance = Some((owner.clone(), instance));
                    match owner {
                        InsertOwner::Track(track) => { self.insert_panel = Some((track, slot)); self.master_insert_panel = None; }
                        InsertOwner::Master => { self.master_insert_panel = Some(slot); self.insert_panel = None; }
                    }
                }
                committed
            }
            Err(error) => { self.notice(format!("无法插入插件：{error}")); cx.notify(); false }
        }
    }

    pub fn toggle_insert_bypass(&mut self, track: &TrackId, slot: usize, cx: &mut Context<Self>) {
        self.toggle_owned_insert_bypass(InsertOwner::Track(track.clone()), slot, cx);
    }

    pub fn toggle_master_insert_bypass(&mut self, slot: usize, cx: &mut Context<Self>) {
        self.toggle_owned_insert_bypass(InsertOwner::Master, slot, cx);
    }

    fn toggle_owned_insert_bypass(&mut self, owner: InsertOwner, slot: usize, cx: &mut Context<Self>) {
        if !self.flush_insert_states(cx) { return; }
        let mut specs = self.owned_insert_specs(&owner);
        let Some(s) = specs.get_mut(slot) else { return };
        s.bypass = !s.bypass;
        self.commit_owned_inserts(&owner, specs, "插件旁通", cx);
    }

    pub fn remove_insert(&mut self, track: &TrackId, slot: usize, cx: &mut Context<Self>) {
        self.remove_owned_insert(InsertOwner::Track(track.clone()), slot, cx);
    }

    pub fn remove_master_insert(&mut self, slot: usize, cx: &mut Context<Self>) {
        self.remove_owned_insert(InsertOwner::Master, slot, cx);
    }

    fn remove_owned_insert(&mut self, owner: InsertOwner, slot: usize, cx: &mut Context<Self>) {
        if let Some(handle) = self.live_owned_insert(&owner, slot) {
            if handle.has_editor() && matches!(handle.status(), InsertStatus::Running) {
                if let Err(e) = handle.close_editor() { self.notice(format!("无法关闭待移除插件窗口：{e}")); cx.notify(); return; }
            }
        }
        if !self.flush_insert_states(cx) { return; }
        let mut specs = self.owned_insert_specs(&owner);
        if slot >= specs.len() { return; }
        specs.remove(slot);
        self.insert_panel = None;
        self.master_insert_panel = None;
        self.insert_runtime.open_instance = None;
        self.commit_owned_inserts(&owner, specs, "移除插件", cx);
    }

    pub fn live_insert(&self, track: &TrackId, slot: usize) -> Option<Arc<InsertHandle>> {
        match self.inserts.get(track)?.get(slot)? { InsertSlot::Live(h) => Some(h.clone()), _ => None }
    }

    pub fn live_master_insert(&self, slot: usize) -> Option<Arc<InsertHandle>> {
        match self.master_inserts.get(slot)? { InsertSlot::Live(handle) => Some(handle.clone()), _ => None }
    }

    /// A generic parameter panel is always available for a live plugin;
    /// native editor availability is a separate capability.
    pub fn show_insert_params(&mut self, track: TrackId, slot: usize, cx: &mut Context<Self>) {
        self.show_owned_insert_params(InsertOwner::Track(track), slot, cx);
    }

    pub fn show_master_insert_params(&mut self, slot: usize, cx: &mut Context<Self>) {
        self.show_owned_insert_params(InsertOwner::Master, slot, cx);
    }

    fn show_owned_insert_params(&mut self, owner: InsertOwner, slot: usize, cx: &mut Context<Self>) {
        if self.inserts_loading { self.notice("插件链正在重建，请稍后读取参数"); cx.notify(); return; }
        self.insert_runtime.open_instance = None;
        match &owner {
            InsertOwner::Track(track) => { self.insert_panel = Some((track.clone(), slot)); self.master_insert_panel = None; }
            InsertOwner::Master => { self.master_insert_panel = Some(slot); self.insert_panel = None; }
        }
        let Some(handle) = self.live_owned_insert(&owner, slot) else { cx.notify(); return };
        let key = handle.key();
        if !self.plugin_busy.insert(key) { return; }
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { handle.params() }).await;
            let _ = this.update(cx, |ed, cx| {
                ed.plugin_busy.remove(&key);
                match result { Ok(params) => ed.cache_editor_params(key, params), Err(e) => ed.notice(format!("读取插件参数失败：{e}")) }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn cache_editor_params(&mut self, key: verdant_plugins::InsertKey, descriptors: Vec<verdant_plugins::ParamDesc>) {
        let mut defaults = HashMap::new();
        for desc in &descriptors {
            // Only a native default the host actually reported becomes a reset target.
            if let Ok(value) = verdant_plugins::insert::parameter_editor_default(desc) {
                defaults.insert(desc.id.clone(), value);
            }
        }
        self.insert_runtime.defaults.insert(key, defaults);
        self.plugin_params.insert(key, descriptors);
    }

    fn step_owned_plugin_param(&mut self, owner: InsertOwner, slot: usize, param: ParamDesc, direction: f64, fine: bool, cx: &mut Context<Self>) {
        let step = if param.step_count > 0 { (param.max - param.min) / param.step_count as f64 }
            else { (param.max - param.min) * if fine { 0.001 } else { 0.01 } };
        self.set_owned_plugin_param(owner, slot, param.clone(), param.value + direction * step, cx);
    }

    fn set_owned_plugin_param(&mut self, owner: InsertOwner, slot: usize, param: ParamDesc, value: f64, cx: &mut Context<Self>) {
        let value = match verdant_plugins::insert::parameter_editor_value(&param, value) {
            Ok(value) => value,
            Err(error) => { self.notice(format!("参数值无效：{error}")); cx.notify(); return; }
        };
        if value == param.value { return; }
        if !self.flush_insert_edits(cx) { return; }
        let Some(handle) = self.live_owned_insert(&owner, slot) else { return };
        let key = handle.key();
        if !self.plugin_busy.insert(key) { return; }
        cx.notify();
        let before = self.owned_insert_specs(&owner);
        cx.spawn(async move |this, cx| {
            let h = handle.clone();
            let result = cx.background_spawn(async move {
                let previous = h.get_state_b64()?;
                let changed = (|| {
                    // Re-read the descriptor after state capture: a native editor
                    // or program change may have changed the range since paint.
                    let desc = h.params()?.into_iter().find(|desc| desc.id == param.id)
                        .ok_or(verdant_plugins::AutomationError::UnknownParameter)?;
                    let value = verdant_plugins::insert::parameter_editor_value(&desc, value)?;
                    h.set_param(&desc.id, value)?;
                    Ok::<_, verdant_plugins::PluginError>((h.get_state_b64()?, h.params()?))
                })();
                Ok::<_, verdant_plugins::PluginError>((previous, changed))
            }).await;
            let _ = this.update(cx, |ed, cx| {
                ed.plugin_busy.remove(&key);
                match result {
                    Ok((previous, Ok((state, params)))) => {
                        if ed.owned_insert_specs(&owner) != before || ed.live_owned_insert(&owner, slot).is_none_or(|h| h.key() != key) {
                            ed.notice("参数编辑期间插入配置已改变；恢复本次修改前的实时状态");
                            ed.rollback_plugin_param(handle.clone(), previous, cx);
                        } else {
                            let mut specs = before;
                            if let Some(spec) = specs.get_mut(slot) { spec.state_b64 = state; }
                            if ed.commit_live_insert_specs(&owner, specs, "插件参数", cx) {
                                ed.cache_editor_params(key, params);
                            } else { ed.rollback_plugin_param(handle.clone(), previous, cx); }
                        }
                    }
                    Ok((previous, Err(e))) => {
                        ed.notice(format!("插件参数未保存：{e}；正在恢复修改前的实时状态"));
                        ed.rollback_plugin_param(handle.clone(), previous, cx);
                    }
                    Err(e) => {
                        ed.notice(format!("插件参数未保存：{e}"));
                    }
                }
                cx.notify();
            });
        }).detach();
    }

    fn rollback_plugin_param(&mut self, handle: Arc<InsertHandle>, previous: String, cx: &mut Context<Self>) {
        let key = handle.key();
        self.plugin_busy.insert(key);
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move {
                handle.set_state_b64(&previous)?;
                handle.params()
            }).await;
            let _ = this.update(cx, |ed, cx| {
                ed.plugin_busy.remove(&key);
                match result {
                    Ok(params) => { ed.cache_editor_params(key, params); ed.restore_inserts(cx); }
                    Err(e) => ed.notice(format!("插件参数回滚失败，保留当前宿主等待恢复：{e}")),
                }
                cx.notify();
            });
        }).detach();
    }

    fn applied_insert_state(&mut self, owner: &InsertOwner) -> Option<(&mut Vec<InsertSpec>, &mut Vec<std::option::Option<std::string::String>>)> {
        match owner {
            InsertOwner::Track(id) => self.insert_runtime.tracks.iter_mut().find(|track| &track.id == id)
                .map(|track| (&mut track.specs, &mut track.live_states)),
            InsertOwner::Master => Some((&mut self.insert_runtime.master.specs, &mut self.insert_runtime.master.live_states)),
        }
    }

    fn commit_live_insert_specs(&mut self, owner: &InsertOwner, specs: Vec<InsertSpec>, label: &str, cx: &mut Context<Self>) -> bool {
        let Some((applied, states)) = self.applied_insert_state(owner) else { return false };
        if *applied == specs { return true; }
        if applied.len() != specs.len() { return false; }
        let old_specs = std::mem::replace(applied, specs.clone());
        let old_states = states.clone();
        for (slot, spec) in specs.iter().enumerate() {
            if old_specs[slot].state_b64 != spec.state_b64 { states[slot] = None; }
        }
        let old_applied = std::mem::take(&mut self.insert_specs_applied);
        self.mark_inserts_applied();
        let committed = self.commit_owned_inserts(owner, specs, label, cx);
        if !committed {
            if let Some((applied, states)) = self.applied_insert_state(owner) { *applied = old_specs; *states = old_states; }
            self.insert_specs_applied = old_applied;
        }
        committed
    }

    pub fn open_insert_editor(&mut self, track: &TrackId, slot: usize, cx: &mut Context<Self>) {
        self.open_owned_insert_editor(InsertOwner::Track(track.clone()), slot, cx);
    }

    pub fn open_master_insert_editor(&mut self, slot: usize, cx: &mut Context<Self>) {
        self.open_owned_insert_editor(InsertOwner::Master, slot, cx);
    }

    fn open_owned_insert_editor(&mut self, owner: InsertOwner, slot: usize, cx: &mut Context<Self>) {
        if self.inserts_loading { self.notice("插件链正在重建，请稍后打开插件窗口"); cx.notify(); return; }
        let Some(handle) = self.live_owned_insert(&owner, slot) else { self.notice("插件缺失，保留了其状态"); cx.notify(); return };
        if !handle.has_editor() { self.show_owned_insert_params(owner, slot, cx); return; }
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { handle.open_editor(&handle.spec().name) }).await;
            let _ = this.update(cx, |ed, cx| { if let Err(e) = result { ed.notice(format!("无法打开插件窗口：{e}")); } cx.notify(); });
        }).detach();
    }

    /// Capture every live host's opaque state in one journaled transaction.
    /// This also covers changes made in a plugin's own native window. Missing
    /// and crashed inserts retain their last admitted state without replacement.
    pub(crate) fn flush_insert_states(&mut self, cx: &mut Context<Self>) -> bool {
        self.capture_insert_states(true, cx)
    }

    /// History navigation captures new native edits, not a serialization-only
    /// difference from an empty/default restore blob. Otherwise undoing the
    /// first full snapshot would immediately recapture it on the next Undo.
    pub(super) fn flush_insert_edits(&mut self, cx: &mut Context<Self>) -> bool {
        self.capture_insert_states(false, cx)
    }

    fn capture_insert_states(&mut self, full_snapshot: bool, cx: &mut Context<Self>) -> bool {
        self.reclaim_inserts(cx);
        if self.inserts_loading || self.insert_runtime.hold.is_some() { self.notice("插件链正在重建，请稍后保存或导出"); cx.notify(); return false; }
        if !self.plugin_busy.is_empty() { self.notice("插件参数正在提交，请稍后保存或导出"); cx.notify(); return false; }
        let desired: Vec<_> = self.project().tracks.iter().map(|t| (&t.id, verdant_plugins::project::read_inserts(&t.unknown))).collect();
        if self.insert_runtime.master.specs != self.master_insert_specs() || !self.insert_runtime.tracks.iter().map(|t| (&t.id, &t.specs))
            .eq(desired.iter().map(|(id, specs)| (*id, specs))) {
            self.notice("工程插入配置尚未应用，保留当前插件；请等待音频链恢复"); cx.notify(); return false;
        }
        let captured = (|| {
            let mut commands = Vec::new();
            let mut updates = Vec::new();
            for (index, track) in self.insert_runtime.tracks.iter().enumerate() {
                let (specs, states) = capture_chain(&track.specs, &track.live_states, &self.inserts[&track.id], full_snapshot)?;
                if specs != track.specs || states != track.live_states {
                    if specs != track.specs { commands.push(Command::SetTrackInserts { track: track.id.clone(), inserts: encoded(&specs) }); }
                    updates.push((index, specs, states));
                }
            }
            let (master_specs, master_states) = capture_chain(&self.insert_runtime.master.specs,
                &self.insert_runtime.master.live_states, &self.master_inserts, full_snapshot)?;
            if master_specs != self.insert_runtime.master.specs {
                commands.extend(self.insert_commands(&InsertOwner::Master, &master_specs)?);
            }
            Ok::<_, String>((commands, updates, master_specs, master_states))
        })();
        let (commands, updates, master_specs, master_states) = match captured {
            Ok(captured) => captured,
            Err(error) => { self.notice(error); cx.notify(); return false; }
        };
        // State capture changes persistence, not the running DSP identity.
        let mut previous = Vec::with_capacity(updates.len());
        for (index, specs, states) in updates {
            let track = &mut self.insert_runtime.tracks[index];
            let old_specs = std::mem::replace(&mut track.specs, specs);
            let old_states = std::mem::replace(&mut track.live_states, states);
            previous.push((index, old_specs, old_states));
        }
        let old_master_specs = std::mem::replace(&mut self.insert_runtime.master.specs, master_specs);
        let old_master_states = std::mem::replace(&mut self.insert_runtime.master.live_states, master_states);
        if commands.is_empty() { self.mark_inserts_applied(); return true; }
        let old_applied = std::mem::take(&mut self.insert_specs_applied);
        self.mark_inserts_applied();
        let committed = self.commit(self.revision(), CommandBatch { label: "保存插件状态".into(), commands }, cx);
        if !committed {
            for (index, specs, states) in previous {
                self.insert_runtime.tracks[index].specs = specs;
                self.insert_runtime.tracks[index].live_states = states;
            }
            self.insert_runtime.master.specs = old_master_specs;
            self.insert_runtime.master.live_states = old_master_states;
            self.insert_specs_applied = old_applied;
        }
        committed
    }
}

#[cfg(test)]
mod topology_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use verdant_audio::automation_clock::{NativeParameterLane, NativePluginSchedule};
    use verdant_audio::engine::{channel, GainProcessor};

    struct EventProbe { id: u32, observed: Arc<AtomicUsize> }
    impl verdant_audio::Processor for EventProbe {
        fn name(&self) -> &str { "automation ownership probe" }
        fn process(&mut self, _: &mut [f32], _: &mut [f32]) {}
        fn process_automated(&mut self, _: &mut [f32], _: &mut [f32], events: &[(u32, u32, f64)]) -> bool {
            assert!(events.iter().all(|event| event.0 == self.id), "another insert's automation reached this processor");
            self.observed.fetch_add(events.len(), Ordering::Relaxed);
            true
        }
    }

    fn timeline(bus: usize, ids: &[u32]) -> Timeline {
        let mut timeline = Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; 4096].into(), gain: 1.0, track: 0 }]);
        let schedules = ids.iter().map(|&id| Some(NativePluginSchedule { instance_id: id.to_string(), parameters: vec![
            NativeParameterLane { native_id: id, fallback: id as f64, curve: Default::default() },
        ] })).collect();
        if bus == MASTER_BUS { timeline.master_native_automation = schedules; }
        else { timeline.native_automation = vec![schedules]; }
        timeline
    }

    fn remap(audio: &mut Control, bus: usize, inputs: Vec<ChainInput>) -> Result<u64, (Vec<Vec<ChainInput>>, Vec<ChainInput>)> {
        let (tracks, master) = if bus == MASTER_BUS { (Vec::new(), inputs) } else { (vec![inputs], Vec::new()) };
        audio.remap_chains_with_master(tracks, master)
    }

    fn publish(runtime: &mut InsertRuntime, audio: &mut Control, timeline: Timeline, generation: u64, project_revision: Revision) {
        let revision = audio.set_timeline(timeline);
        runtime.hold.as_mut().unwrap().timeline = Some(InsertTimelineReceipt { revision, generation, project_revision });
    }

    fn ready(runtime: &InsertRuntime, audio: &mut Control, generation: u64, project_revision: Revision) -> bool {
        runtime.hold.as_ref().unwrap().ready(audio.retired_chain_revision(), audio.applied_timeline_revision(), generation, project_revision)
    }

    #[test]
    fn master_and_same_track_remaps_hold_automation_until_matching_receipts() {
        for bus in [0, MASTER_BUS] {
            let (mut audio, mut callback) = channel(48_000, 64);
            let observed = Arc::new(AtomicUsize::new(0));
            assert!(remap(&mut audio, bus, vec![
                ChainInput::New(Box::new(EventProbe { id: 1, observed: Arc::new(AtomicUsize::new(0)) })),
                ChainInput::New(Box::new(EventProbe { id: 2, observed: observed.clone() })),
            ]).is_ok());
            audio.set_timeline(timeline(bus, &[1, 2]));
            audio.seek(192);
            audio.play();
            let mut output = [0.0; 128];
            callback.process(&mut output, 2);
            let events = observed.load(Ordering::Relaxed);
            assert!(events > 0);
            let position = audio.position();

            // Saturate the actual chain ring before removing A. A rejected
            // B:1 -> B:0 remap must keep its ownership coordinates for retry.
            for _ in 0..verdant_audio::MAX_TRACKS {
                assert!(remap(&mut audio, bus, vec![ChainInput::Retained { track: bus, processor: 0 },
                    ChainInput::Retained { track: bus, processor: 1 }]).is_ok());
            }
            let mut runtime = InsertRuntime::default();
            let mut generation = 0;
            let project_revision = Revision::new(0).unwrap();
            runtime.hold_transport(&mut audio, 48_000, &mut generation);
            let (tracks, master) = remap(&mut audio, bus, vec![ChainInput::Retained { track: bus, processor: 1 }]).err().unwrap();
            assert!(!audio.shared.playback_requested());
            callback.process(&mut output, 2);
            assert_eq!(observed.load(Ordering::Relaxed), events);

            runtime.hold_transport(&mut audio, 48_000, &mut generation);
            let chain_revision = audio.remap_chains_with_master(tracks, master).ok().unwrap();
            runtime.hold.as_mut().unwrap().chain_revision = chain_revision;
            callback.process(&mut output, 2);
            assert!(!ready(&runtime, &mut audio, generation, project_revision), "a chain receipt alone cannot resume playback");
            assert_eq!(observed.load(Ordering::Relaxed), events);
            publish(&mut runtime, &mut audio, timeline(bus, &[2]), generation, project_revision);
            assert!(!ready(&runtime, &mut audio, generation, project_revision), "enqueueing a timeline is not an applied receipt");
            callback.process(&mut output, 2);
            assert!(ready(&runtime, &mut audio, generation, project_revision));

            let edited_revision = project_revision.next().unwrap();
            assert!(!ready(&runtime, &mut audio, generation, edited_revision), "an intervening edit invalidates the receipt before replan");
            generation += 1;
            assert!(!ready(&runtime, &mut audio, generation, project_revision), "a newer build cannot reuse an older receipt");
            publish(&mut runtime, &mut audio, timeline(bus, &[2]), generation, edited_revision);
            assert!(!ready(&runtime, &mut audio, generation, edited_revision));
            callback.process(&mut output, 2);
            assert!(ready(&runtime, &mut audio, generation, edited_revision));
            let hold = runtime.hold.take().unwrap();
            assert_eq!(hold.position, position);
            assert!(hold.playing);
            hold.resume(&audio);
            callback.process(&mut output, 2);
            assert!(observed.load(Ordering::Relaxed) > events, "retained B still receives its own automation after resume");
            assert_eq!(audio.position(), position + 64);
        }
    }

    #[test]
    fn abandoned_remap_preserves_chain_receipt_and_stop_seek_intent() {
        let (mut audio, mut callback) = channel(48_000, 64);
        let original_revision = remap(&mut audio, 0, Vec::new()).ok().unwrap();
        let mut original = timeline(0, &[]);
        original.loop_range = Some((96, 2048));
        audio.set_timeline(original);
        callback.process(&mut [0.0; 128], 2);
        audio.stop();
        audio.play();
        let mut runtime = InsertRuntime::default();
        let mut generation = 0;
        runtime.hold_transport(&mut audio, 48_000, &mut generation);
        assert_eq!(runtime.hold.as_ref().unwrap().position, 96, "pending Stop uses the loop start");
        assert!(runtime.hold.as_ref().unwrap().playing);

        for _ in 0..verdant_audio::MAX_TRACKS { assert!(remap(&mut audio, 0, Vec::new()).is_ok()); }
        let rejected = remap(&mut audio, 0, vec![ChainInput::New(Box::new(GainProcessor(1.0)))]).err().unwrap();
        assert!(matches!(&rejected.0[0][0], ChainInput::New(_)));
        // The document reverts before retrying this build, so the fresh
        // processor is retired on the control side and the old chain stays.
        drop(rejected);
        let hold = runtime.hold.as_mut().unwrap();
        hold.playing = false;
        hold.position = 1536;
        audio.stop();
        audio.seek(1536);
        runtime.hold_transport(&mut audio, 48_000, &mut generation);
        assert_eq!(runtime.hold.as_ref().unwrap().chain_revision, original_revision);
        publish(&mut runtime, &mut audio, timeline(0, &[]), generation, Revision::new(1).unwrap());
        callback.process(&mut [0.0; 128], 2);
        assert!(ready(&runtime, &mut audio, generation, Revision::new(1).unwrap()));
        runtime.hold.take().unwrap().resume(&audio);
        callback.process(&mut [0.0; 128], 2);
        assert!(!audio.shared.playback_requested());
        assert_eq!(audio.position(), 1536);
    }

    #[test]
    fn abandoned_build_republishes_dropped_timeline_receipt() {
        let (mut audio, mut callback) = channel(48_000, 64);
        remap(&mut audio, 0, vec![ChainInput::New(Box::new(GainProcessor(1.0)))]).ok().unwrap();
        let mut runtime = InsertRuntime::default();
        let mut generation = 0;
        runtime.hold_transport(&mut audio, 48_000, &mut generation);
        // The replacement timeline for the restored order finishes while its
        // abandoned build still holds `inserts_loading`, so its receipt is
        // discarded by `insert_timeline_ready`.
        let _dropped_receipt = audio.set_timeline(timeline(0, &[]));
        assert!(runtime.hold.as_ref().unwrap().timeline.is_none());
        assert!(runtime.needs_timeline_receipt());
        // The stale build's completion hits the unchanged restore path; the
        // kept topology never republishes a receipt on its own.
        callback.process(&mut [0.0; 128], 2);
        assert!(!runtime.hold.as_ref().unwrap().ready(
            audio.retired_chain_revision(), audio.applied_timeline_revision(), generation, Revision::new(1).unwrap()),
            "the dropped receipt can never release the hold");
        // The restored path's fresh rebuild arrives after loading cleared and
        // is stamped with the current generation.
        publish(&mut runtime, &mut audio, timeline(0, &[]), generation, Revision::new(1).unwrap());
        assert!(!runtime.needs_timeline_receipt());
        callback.process(&mut [0.0; 128], 2);
        assert!(ready(&runtime, &mut audio, generation, Revision::new(1).unwrap()));
    }

    #[test]
    fn matching_topology_includes_master_and_same_track_specs() {
        let specs = ["A", "B"].map(|id| {
            let mut spec = InsertSpec::new("builtin", id, id, "");
            spec.instance_id = id.into();
            spec
        }).to_vec();
        let mut project = Project::new_empty();
        project.tracks[0].unknown.insert("inserts".into(), serde_json::Value::Array(encoded(&specs)));
        let master_key = verdant_domain::automation::MASTER_INSERTS_KEY;
        project.extensions.insert(master_key.into(), serde_json::Value::Array(encoded(&specs)));
        let runtime = InsertRuntime { tracks: vec![AppliedTrack { id: project.tracks[0].id.clone(), specs: specs.clone(),
            live_states: vec![None; 2], processors: vec![Some(0), Some(1)], padding: 0 }],
            master: AppliedMaster { specs: specs.clone(), live_states: vec![None; 2], processors: vec![Some(0), Some(1)] },
            ..Default::default() };
        assert!(runtime.matches_project(&project));
        project.extensions.insert(master_key.into(), serde_json::Value::Array(encoded(&specs[1..])));
        assert!(!runtime.matches_project(&project), "unchanged track IDs do not admit a master removal");
        project.extensions.insert(master_key.into(), serde_json::Value::Array(encoded(&specs)));
        let mut changed = specs.clone();
        changed.reverse();
        project.tracks[0].unknown.insert("inserts".into(), serde_json::Value::Array(encoded(&changed)));
        assert!(!runtime.matches_project(&project), "same-track reordering changes the binding");
        changed = specs;
        changed[0].state_b64 = "restored-state".into();
        project.tracks[0].unknown.insert("inserts".into(), serde_json::Value::Array(encoded(&changed)));
        assert!(!runtime.matches_project(&project), "state replacement cannot reuse the old binding's receipt");
    }
}
