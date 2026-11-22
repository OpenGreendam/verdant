//! Mixer strips bind persisted inserts to real isolated hosts. Fader/pan
//! previews use audio atomics; one release is one undoable edit. Retained
//! child meters repaint independently of these controls and insert slots.
//! Master bus owns a real Master Fader, Master Pan, Master Mute, Master
//! Inserts and stereo peak/clip metering shared with offline WAV export.

use gpui::{
    AccessibleAction, BorderStyle, Context, DispatchPhase, Entity, FontWeight,
    HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Role,
    SharedString, canvas, div, fill, point, prelude::*, px, quad,
};
use verdant_domain::TrackId;
use verdant_plugins::{InsertSlot, InsertSpec, InsertStatus};

use crate::editor::{Editor, MenuAction, MixerControl};
use crate::generated::controls::{CommandArg, CommandId, NumericId};
use crate::menu::MenuItem;
use crate::paint::{Texts, b, flush_texts};
use crate::theme::{Theme, mix};
use crate::ui_controls::{numeric_pill, selectable_command_button};
use crate::view::command_button;

pub const STRIP_W: f32 = 196.0;

pub fn fader_pos(mdb: i32) -> f64 {
    (10f64.powf(mdb as f64 / 20_000.0) / 2.0).powf(0.25).clamp(0.0, 1.0)
}

pub fn fader_mdb(pos: f64) -> i32 {
    if pos <= 0.02 {
        -96_000
    } else {
        (20_000.0 * (2.0 * pos.clamp(0.0, 1.0).powi(4)).log10()).round().clamp(-96_000.0, 6_000.0) as i32
    }
}

fn peak_db_label(peaks: (f32, f32)) -> (String, bool) {
    let p = peaks.0.max(peaks.1);
    if p >= 0.999 {
        ("CLIP".into(), true)
    } else if p <= 1e-4 {
        ("-∞".into(), false)
    } else {
        (format!("{:+.1}", 20.0 * p.log10()), false)
    }
}

pub fn mixer(ed: &Editor, entity: &Entity<Editor>, cx: &mut Context<Editor>) -> impl IntoElement {
    let th = ed.theme;
    let draw = entity.clone();
    div()
        .id("mixer")
        .size_full()
        .flex()
        .bg(th.view)
        .p(px(8.0))
        .gap(px(8.0))
        .child(div().flex_1().min_w_0().h_full().child(crate::proto_scroll::native_content("mixer-strips-region", "混音轨道", move |scroll, _, cx| {
            draw.update(cx, |ed, cx| mixer_strips(ed, &draw, scroll, cx))
        }).axes("horizontal")))
        .child(master_strip(ed, entity, cx))
        .children(parameter_panel(ed, cx))
}

fn mixer_strips(ed: &Editor, entity: &Entity<Editor>, scroll: &gpui::ScrollHandle, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let mut strips = Vec::with_capacity(ed.project().tracks.len());
    for (i, track) in ed.project().tracks.iter().enumerate() {
        let header = ed.mixer_drag
            .as_ref()
            .filter(|d| d.track == track.id)
            .map(|d| d.header.clone())
            .unwrap_or_else(|| track.header());
        strips.push(strip(ed, entity, i, track.id.clone(), header, cx).into_any_element());
    }
    div().id("mixer-strips").size_full().overflow_x_scroll().track_scroll(scroll)
        .flex().gap(px(8.0)).children(strips).into_any_element()
}

fn insert_state(loading: bool, runtime: Option<&InsertSlot>, spec: &InsertSpec) -> (String, bool) {
    if loading {
        return ("正在重建插件链…".into(), false);
    }
    match runtime {
        Some(InsertSlot::Live(handle)) if handle.spec().id == spec.id && handle.spec().format == spec.format => match handle.status() {
            InsertStatus::Running => (
                format!(
                    "{} · DSP {} + 传输 {} 帧 · xrun {}",
                    if spec.bypass { "旁通" } else { "运行" },
                    handle.plugin_latency_samples(),
                    handle.transport_latency_samples(),
                    handle.xruns()
                ),
                false,
            ),
            InsertStatus::Crashed { detail } => (format!("崩溃已旁通 · {detail}"), true),
        },
        Some(InsertSlot::Missing { reason, .. }) => (format!("缺失 · {reason}"), true),
        _ => ("正在载入插件…".into(), false),
    }
}

fn master_strip(ed: &Editor, entity: &Entity<Editor>, cx: &mut Context<Editor>) -> impl IntoElement {
    let th = ed.theme;
    let specs = ed.master_insert_specs();
    let master = ed.master_bus();
    let (peak_str, clipped) = peak_db_label(ed.master_meter);
    let draw = entity.clone();
    let slots = div().h(px(108.0)).flex_none().child(crate::proto_scroll::native_content("master-slots-region", "主总线插入插件", move |scroll, _, cx| {
        draw.update(cx, |ed, cx| master_insert_slots(ed, scroll, cx))
    }));

    div()
        .id("mixer-master")
        .w(px(STRIP_W + 12.0))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .rounded(px(12.0))
        .bg(th.card)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_sm().font_weight(FontWeight::BOLD).child("主总线"))
                .child(
                    div()
                        .px(px(5.0))
                        .py(px(1.0))
                        .rounded(px(6.0))
                        .bg(if clipped { th.error } else { th.view })
                        .text_size(px(10.0))
                        .font_family("monospace")
                        .text_color(if clipped { th.accent_text } else { th.text_secondary })
                        .child(peak_str),
                ),
        )
        .child(div().text_xs().text_color(th.text_secondary).child("立体声输出 · 软削波保护"))
        .child(slots)
        .child(
            route_button("add-master-insert", "+ 主总线插入…", false, th)
                .flex_none()
                .on_click(cx.listener(|ed, _: &crate::proto_controls::Activation, window, cx| {
                    master_plugin_picker(ed, window.mouse_position(), window, cx);
                    cx.stop_propagation();
                })),
        )
        .child(numeric_pill(ed, NumericId::MasterPan, CommandArg::None, "mix-master-pan", true, cx))
        .child(div().h(px(22.0)).flex_none().child(master_pan_control(entity.clone(), master.pan_permille)))
        .child(
            div()
                .flex_1()
                .min_h(px(96.0))
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(master_fader(entity.clone(), master.gain_millidecibels)))
                .child(div().w(px(24.0)).h_full().child(ed.playback.anchor(crate::clock::Surface::Master))),
        )
        .child(numeric_pill(ed, NumericId::MasterGainDb, CommandArg::None, "mix-master-gain", true, cx))
        .child(
            div()
                .flex()
                .p(px(2.0))
                .gap(px(2.0))
                .rounded(px(8.0))
                .bg(th.control)
                .child(
                    selectable_command_button(
                        ed,
                        CommandId::ToggleMasterMute,
                        CommandArg::None,
                        if master.muted { "M 已静音" } else { "M 静音" },
                        "master-mute",
                        master.muted,
                        cx,
                    )
                    .flex_1(),
                )
                .child(
                    selectable_command_button(
                        ed,
                        CommandId::ToggleMasterAutomation,
                        CommandArg::None,
                        "A 自动化",
                        "master-auto",
                        ed.automation_ui.show_master,
                        cx,
                    )
                    .flex_1(),
                ),
        )
        .child(command_button(
            ed,
            CommandId::ExportAudio,
            CommandArg::None,
            if ed.export_busy { "正在导出…" } else { "导出 WAV…" },
            "export-wav",
            cx,
        ))
}

fn master_plugin_picker(ed: &mut Editor, anchor: gpui::Point<gpui::Pixels>, window: &mut gpui::Window, cx: &mut Context<Editor>) {
    let mut items = ed
        .plugin_catalog()
        .into_iter()
        .map(|plugin| {
            MenuItem::action(
                format!("{} · {}", plugin.name, plugin.format.tag().to_uppercase()),
                MenuAction::AddMasterInsert { format: plugin.format.tag().into(), id: plugin.id },
            )
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        items.push(MenuItem::disabled("没有可用插件；请检查插件宿主"));
    }
    items.push(MenuItem::separator());
    items.push(MenuItem::action("重新扫描插件", MenuAction::RescanPlugins));
    ed.show_menu(anchor, items, window, cx);
}

fn strip(
    ed: &Editor,
    entity: &Entity<Editor>,
    index: usize,
    id: TrackId,
    header: verdant_domain::TrackHeader,
    cx: &mut Context<Editor>,
) -> impl IntoElement {
    let th = ed.theme;
    let color = th.track(header.color.as_deref());
    let selected = ed.session.track == id;
    let auto_on = ed.automation_ui.expanded_tracks.contains(&id);
    let track_peaks = ed.meters.get(index).copied().unwrap_or((0.0, 0.0));
    let (peak_str, clipped) = peak_db_label(track_peaks);
    let (select, menu, header_menu) = (
        id.clone(),
        id.clone(),
        id.clone(),
    );
    let specs = ed.insert_specs(&id);
    let draw = entity.clone(); let slots_track = id.clone();
    let slots = div().h(px(108.0)).flex_none().child(crate::proto_scroll::native_content("track-slots-region", "轨道插入插件", move |scroll, _, cx| {
        draw.update(cx, |ed, cx| track_insert_slots(ed, &slots_track, scroll, cx))
    }));

    crate::proto_controls::action(SharedString::from(format!("mixer-track-{id}")), header.name.clone()).aria_selected(selected)
        .w(px(STRIP_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .rounded(px(12.0))
        .bg(if selected { mix(th.card, th.accent, 0.14) } else { th.card })
        .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
            if !ed.select_track(select.clone(), cx) {
                cx.stop_propagation();
            }
        }))
        .on_context_menu(
            cx.listener(move |ed, ev: &crate::proto_controls::Activation, window, cx| {
                ed.track_menu(menu.clone(), ev.position(), window, cx);
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(div().size(px(10.0)).rounded(px(5.0)).bg(color))
                .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::BOLD).child(header.name.clone()))
                .child(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(6.0))
                        .bg(if clipped { th.error } else { th.view })
                        .text_size(px(9.5))
                        .font_family("monospace")
                        .text_color(if clipped { th.accent_text } else { th.text_secondary })
                        .child(peak_str),
                )
                .child(
                    crate::proto_controls::action("track-menu", format!("{}：轨道菜单", header.name))
                        .aria_description("重命名、上移、下移或复制轨道")
                        .size(px(26.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .hover(move |s| s.bg(th.hover))
                        .child(gpui::svg().path(crate::assets::icon("menu")).size(px(14.0)).text_color(th.text))
                        .on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, window, cx| {
                            ed.track_menu(header_menu.clone(), event.position(), window, cx);
                            cx.stop_propagation();
                        })),
                ),
        )
        .child(slots)
        .child(command_button(ed, CommandId::PluginAdd, CommandArg::TrackSlot(id.clone(), specs.len() as u32), "+ 插入效果…", SharedString::from(format!("add-insert-{id}")), cx))
        .child(numeric_pill(ed, NumericId::TrackPan, CommandArg::Track(id.clone()), format!("mix-pan-{id}"), true, cx))
        .child(div().h(px(22.0)).flex_none().child(pan_control(entity.clone(), id.clone(), header.pan_permille)))
        .child(
            div()
                .flex_1()
                .min_h(px(96.0))
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(fader(entity.clone(), id.clone(), header.gain_millidecibels)))
                .child(div().w(px(24.0)).h_full().child(ed.playback.anchor(crate::clock::Surface::Meter(index)))),
        )
        .child(numeric_pill(ed, NumericId::TrackGainDb, CommandArg::Track(id.clone()), format!("mix-gain-{id}"), true, cx))
        .child(
            div()
                .flex()
                .p(px(2.0))
                .gap(px(2.0))
                .rounded(px(8.0))
                .bg(th.control)
                .child(selectable_command_button(ed, CommandId::ToggleMute, CommandArg::Track(id.clone()), "M", SharedString::from(format!("mix-mute-{id}")), header.muted, cx).flex_1())
                .child(
                    command_button(ed, CommandId::ToggleSolo, CommandArg::Track(id.clone()), "S", SharedString::from(format!("mix-solo-{id}")), cx)
                        .flex_1()
                        .aria_selected(header.solo)
                        .border_1()
                        .rounded(px(6.0))
                        .border_color(if header.solo { th.warning } else { th.control })
                        .when(header.solo, |d| d.bg(th.warning).text_color(th.on_track()).font_weight(FontWeight::SEMIBOLD)),
                )
                .child(
                    selectable_command_button(
                        ed,
                        CommandId::ToggleTrackAutomation,
                        CommandArg::Track(id.clone()),
                        "A",
                        SharedString::from(format!("mix-auto-{id}")),
                        auto_on,
                        cx,
                    )
                    .flex_1(),
                ),
        )
}

fn master_insert_slots(ed: &Editor, scroll: &gpui::ScrollHandle, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme; let specs = ed.master_insert_specs();
    let mut slots = div().id("master-insert-slots").size_full().overflow_y_scroll().track_scroll(scroll).flex().flex_col().gap(px(3.0));
    for (slot, spec) in specs.iter().enumerate() {
        let runtime = ed.master_insert_slots().get(slot);
        let (state, warning) = insert_state(ed.inserts_loading, runtime, spec);
        let busy = ed.inserts_loading || matches!(runtime, Some(InsertSlot::Live(handle)) if ed.plugin_busy.contains(&handle.key()));
        slots = slots.child(
            div()
                .id(SharedString::from(format!("master-insert-{slot}")))
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .rounded(px(8.0))
                .bg(th.control)
                .p(px(4.0))
                .child(
                    crate::proto_controls::action("master-insert-parameters", format!("主总线插入 {}，{}，{}", slot + 1, spec.name, state))
                        .aria_description("打开主总线插件原生界面")
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(div().text_xs().truncate().child(format!("{} · {}", slot + 1, spec.name)))
                        .child(div().text_size(px(9.0)).truncate().text_color(if warning { th.warning } else { th.text_secondary }).child(state))
                        .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                            ed.open_master_insert_editor(slot, cx);
                            cx.stop_propagation();
                        })),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(3.0))
                        .child(
                            route_button("master-insert-bypass", if spec.bypass { "启用" } else { "旁通" }, spec.bypass, th)
                                .disabled(busy)
                                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                                    if !busy { ed.toggle_master_insert_bypass(slot, cx); }
                                    cx.stop_propagation();
                                })),
                        )
                        .child(
                            route_button("master-insert-remove", "移除", false, th)
                                .disabled(busy)
                                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                                    if !busy { ed.remove_master_insert(slot, cx); }
                                    cx.stop_propagation();
                                })),
                        ),
                ),
        );
    }
    if specs.is_empty() {
        slots = slots.child(div().px(px(6.0)).py(px(4.0)).text_xs().text_color(th.text_tertiary).child("无主总线插入"));
    }
    slots.into_any_element()
}

fn track_insert_slots(ed: &Editor, id: &TrackId, scroll: &gpui::ScrollHandle, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme; let specs = ed.insert_specs(id);
    let mut slots = div().id("insert-slots").size_full().overflow_y_scroll().track_scroll(scroll).flex().flex_col().gap(px(3.0));
    for (slot, spec) in specs.iter().enumerate() {
        let runtime = ed.inserts.get(&id).and_then(|s| s.get(slot));
        let (state, warning) = insert_state(ed.inserts_loading, runtime, spec);
        let (open_track, menu_track) = (id.clone(), id.clone());
        let native = matches!(runtime, Some(InsertSlot::Live(h)) if h.has_editor()
            || (spec.format == "builtin" && verdant_plugins::builtin_dsp::has_native_editor(&spec.id)));
        let spec_bypass = spec.bypass;
        let expected_key = ed.live_insert(&id, slot).map(|handle| handle.key());
        slots = slots.child(
            crate::proto_controls::action(SharedString::from(format!("insert-{slot}")), format!("插入 {}，{}，{}", slot + 1, spec.name, state))
                .aria_description("点击打开原生界面；右键打开旁通和移除菜单")
                .flex_none()
                .min_h(px(38.0))
                .px(px(6.0))
                .py(px(3.0))
                .rounded(px(8.0))
                .bg(th.control)
                .flex()
                .flex_col()
                .child(div().text_xs().truncate().child(format!("{} · {}", slot + 1, spec.name)))
                .child(div().text_size(px(9.0)).truncate().text_color(if warning { th.warning } else { th.text_secondary }).child(state))
                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    if expected_key.is_none_or(|key| ed.live_insert(&open_track, slot).is_some_and(|handle| handle.key() == key)) && ed.select_track(open_track.clone(), cx) {
                        ed.open_insert_editor(&open_track, slot, cx);
                    }
                    cx.stop_propagation();
                }))
                .on_context_menu(
                    cx.listener(move |ed, ev: &crate::proto_controls::Activation, window, cx| {
                        let items = vec![
                            MenuItem::action(
                                if spec_bypass { "启用插件" } else { "旁通插件" },
                                MenuAction::BypassInsert { track: menu_track.clone(), slot, expected_key },
                            ),
                            MenuItem::action("打开原生插件窗口", MenuAction::OpenInsertEditor { track: menu_track.clone(), slot, expected_key })
                                .enabled(native),
                            MenuItem::separator(),
                            MenuItem::action("移除插件", MenuAction::RemoveInsert { track: menu_track.clone(), slot, expected_key }),
                        ];
                        ed.show_menu(ev.position(), items, window, cx);
                        cx.stop_propagation();
                    }),
                ),
        );
    }
    if specs.is_empty() {
        slots = slots.child(div().px(px(6.0)).py(px(4.0)).text_xs().text_color(th.text_tertiary).child("无插入效果"));
    }
    slots.into_any_element()
}

fn route_button(key: &'static str, label: &'static str, selected: bool, th: Theme) -> crate::proto_controls::Action {
    crate::proto_controls::action(key, label)
        .aria_selected(selected)
        .h(px(crate::ui_controls::BAR_BTN_H))
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(if selected { th.checked } else { th.control })
        .text_xs()
        .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(th.text)
        .hover(move |s| s.bg(if selected { th.checked } else { th.control_pressed }))
        .child(label)
}

fn parameter_panel(ed: &Editor, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    if let Some(slot) = ed.master_insert_panel {
        return master_parameter_panel(ed, slot, cx);
    }
    let (track, slot) = ed.insert_panel.as_ref()?;
    let specs = ed.insert_specs(track);
    let spec = specs.get(*slot)?;
    let th = ed.theme;
    let mut panel = div()
        .id("plugin-parameters")
        .w(px(320.0))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(12.0))
        .bg(th.surface)
        .border_l_1()
        .border_color(th.shade)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(spec.name.clone()))
                .child(route_button("close-params", "关闭", false, th).flex_none().px(px(7.0)).on_click(cx.listener(|ed, _: &crate::proto_controls::Activation, _, cx| {
                    ed.insert_panel = None;
                    cx.notify();
                }))),
        )
        .child(
            div()
                .flex()
                .gap(px(5.0))
                .child(command_button(
                    ed,
                    CommandId::PluginBypass,
                    CommandArg::TrackSlot(track.clone(), *slot as u32),
                    if spec.bypass { "启用" } else { "旁通" },
                    "bypass",
                    cx,
                ))
                .child(command_button(ed, CommandId::PluginRemove, CommandArg::TrackSlot(track.clone(), *slot as u32), "移除", "remove", cx)),
        );
    if let Some(handle) = ed.live_insert(track, *slot) {
        if handle.has_editor() || (spec.format == "builtin" && verdant_plugins::builtin_dsp::has_native_editor(&spec.id)) {
            panel = panel.child(command_button(
                ed,
                CommandId::PluginOpenEditor,
                CommandArg::TrackSlot(track.clone(), *slot as u32),
                "打开原生插件窗口",
                "native-editor",
                cx,
            ));
        }
        let running = matches!(handle.status(), InsertStatus::Running);
        let busy = ed.inserts_loading || ed.plugin_busy.contains(&handle.key()) || !running;
        let (state, warning) = insert_state(ed.inserts_loading, ed.inserts.get(track).and_then(|slots| slots.get(*slot)), spec);
        let (refresh_track, refresh_slot) = (track.clone(), *slot);
        panel = panel
            .child(
                route_button(
                    "refresh-params",
                    if !running { "插件已停止" } else if busy { "正在读取 / 提交…" } else { "刷新插件参数" },
                    false,
                    th,
                )
                .flex_none()
                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    if !busy {
                        ed.show_insert_params(refresh_track.clone(), refresh_slot, cx);
                    }
                })),
            )
            .child(div().text_xs().text_color(if warning { th.warning } else { th.text_secondary }).child(state));
        let draw = cx.entity(); let params_track = track.clone(); let params_slot = *slot;
        panel = panel.child(div().flex_1().min_h_0().child(crate::proto_scroll::native_content("parameter-region", "轨道插件参数", move |scroll, _, cx| {
            draw.update(cx, |ed, cx| track_parameter_rows(ed, &params_track, params_slot, scroll, cx))
        })));
    } else {
        let reason = match ed.inserts.get(track).and_then(|s| s.get(*slot)) {
            Some(InsertSlot::Missing { reason, .. }) => reason.as_str(),
            _ => "正在载入插件",
        };
        panel = panel
            .child(div().text_sm().text_color(th.warning).child(reason.to_string()))
            .child(div().text_xs().text_color(th.text_secondary).child("已保留此插入的标识、参数状态和未知字段。安装插件后可重新扫描恢复。"));
    }
    Some(panel.into_any_element())
}

fn master_parameter_panel(ed: &Editor, slot: usize, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let specs = ed.master_insert_specs();
    let spec = specs.get(slot)?;
    let th = ed.theme;
    let busy = ed.inserts_loading || ed.live_master_insert(slot).is_some_and(|handle| ed.plugin_busy.contains(&handle.key()));
    let mut panel = div()
        .id("master-plugin-parameters")
        .w(px(320.0))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(12.0))
        .bg(th.surface)
        .border_l_1()
        .border_color(th.shade)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("主总线 · {}", spec.name)))
                .child(route_button("close-master-params", "关闭", false, th).flex_none().px(px(7.0)).on_click(cx.listener(|ed, _: &crate::proto_controls::Activation, _, cx| {
                    ed.master_insert_panel = None;
                    cx.notify();
                }))),
        )
        .child(
            div()
                .flex()
                .gap(px(5.0))
                .child(
                    route_button("master-param-bypass", if spec.bypass { "启用" } else { "旁通" }, spec.bypass, th)
                        .disabled(busy)
                        .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                            if !busy { ed.toggle_master_insert_bypass(slot, cx); }
                            cx.stop_propagation();
                        })),
                )
                .child(
                    route_button("master-param-remove", "移除", false, th)
                        .disabled(busy)
                        .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                            if !busy { ed.remove_master_insert(slot, cx); }
                            cx.stop_propagation();
                        })),
                ),
        );
    if let Some(handle) = ed.live_master_insert(slot) {
        if handle.has_editor() || (spec.format == "builtin" && verdant_plugins::builtin_dsp::has_native_editor(&spec.id)) {
            panel = panel.child(route_button("master-native-editor", "打开原生插件窗口", false, th).flex_none().on_click(cx.listener(
                move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    ed.open_master_insert_editor(slot, cx);
                    cx.stop_propagation();
                },
            )));
        }
        let running = matches!(handle.status(), InsertStatus::Running);
        let busy = busy || !running;
        let (state, warning) = insert_state(ed.inserts_loading, ed.master_insert_slots().get(slot), spec);
        panel = panel
            .child(
                route_button(
                    "refresh-master-params",
                    if !running { "插件已停止" } else if busy { "正在读取 / 提交…" } else { "刷新插件参数" },
                    false,
                    th,
                )
                .flex_none()
                .disabled(busy)
                .on_click(cx.listener(move |ed, _: &crate::proto_controls::Activation, _, cx| {
                    if !busy { ed.show_master_insert_params(slot, cx); }
                    cx.stop_propagation();
                })),
            )
            .child(div().text_xs().text_color(if warning { th.warning } else { th.text_secondary }).child(state));
        let draw = cx.entity();
        panel = panel.child(div().flex_1().min_h_0().child(crate::proto_scroll::native_content("master-parameter-region", "主总线插件参数", move |scroll, _, cx| {
            draw.update(cx, |ed, cx| master_parameter_rows(ed, slot, scroll, cx))
        })));
    } else {
        let reason = match ed.master_insert_slots().get(slot) {
            Some(InsertSlot::Missing { reason, .. }) => reason.as_str(),
            _ => "正在载入插件",
        };
        panel = panel
            .child(div().text_sm().text_color(th.warning).child(reason.to_string()))
            .child(div().text_xs().text_color(th.text_secondary).child("已保留此主总线插入的标识、参数状态和未知字段。安装插件后可重新扫描恢复。"));
    }
    Some(panel.into_any_element())
}

fn master_parameter_rows(ed: &Editor, slot: usize, scroll: &gpui::ScrollHandle, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let mut list = div().id("master-parameter-scroll").size_full().overflow_y_scroll().track_scroll(scroll).flex().flex_col().gap(px(8.0));
    if let Some(handle) = ed.live_master_insert(slot) {
        let busy = ed.inserts_loading || ed.plugin_busy.contains(&handle.key()) || !matches!(handle.status(), InsertStatus::Running);
        if let Some(params) = ed.plugin_params.get(&handle.key()) {
            if params.is_empty() { list = list.child(div().text_sm().text_color(th.text_secondary).child("此插件未提供可编辑参数")); }
            for param in params { list = list.child(ed.master_plugin_parameter_row(slot, param.clone(), busy, cx)); }
        }
    }
    list.into_any_element()
}

fn track_parameter_rows(ed: &Editor, track: &TrackId, slot: usize, scroll: &gpui::ScrollHandle, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let mut list = div().id("parameter-scroll").size_full().overflow_y_scroll().track_scroll(scroll).flex().flex_col().gap(px(8.0));
    if let Some(handle) = ed.live_insert(track, slot) {
        let busy = ed.inserts_loading || ed.plugin_busy.contains(&handle.key()) || !matches!(handle.status(), InsertStatus::Running);
        if let Some(params) = ed.plugin_params.get(&handle.key()) {
            if params.is_empty() { list = list.child(div().text_sm().text_color(th.text_secondary).child("此插件未提供可编辑参数")); }
            for param in params { list = list.child(ed.plugin_parameter_row(track.clone(), slot, param.clone(), busy, cx)); }
        }
    }
    list.into_any_element()
}

fn paint_fader_surface(window: &mut gpui::Window, cx: &mut gpui::App, bounds: gpui::Bounds<gpui::Pixels>, th: Theme, mdb: i32) {
    let o = bounds.origin;
    let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
    let x = w * 0.58;
    let usable = (h - 14.0).max(1.0);
    let mut texts = Texts::new();
    for (db, lbl) in [(6.0_f64, "+6"), (0.0, " 0"), (-6.0, "-6"), (-12.0, "-12"), (-24.0, "-24"), (-48.0, "-48")] {
        let y = 7.0 + (1.0 - fader_pos((db * 1000.0) as i32)) * usable;
        let is_zero = db.abs() < 1e-6;
        window.paint_quad(fill(b(x - 15.0, y, if is_zero { 9.0 } else { 6.0 }, if is_zero { 1.5 } else { 1.0 }, o), if is_zero { th.text_secondary } else { th.text_tertiary }));
        if h >= 110.0 {
            texts.push((lbl.into(), point(o.x + px((x - 38.0).max(1.0) as f32), o.y + px((y - 5.5) as f32)), 8.5, if is_zero { th.text_secondary } else { th.text_tertiary }));
        }
    }
    // GtkScale: a rounded trough, the accent highlight up to the value, a
    // round white slider with a hairline outline.
    window.paint_quad(quad(b(x - 3.0, 7.0, 6.0, usable, o), px(3.0), th.control_pressed, px(0.0), th.control_pressed, BorderStyle::Solid));
    let y = 7.0 + (1.0 - fader_pos(mdb)) * usable;
    window.paint_quad(quad(b(x - 3.0, y, 6.0, (7.0 + usable - y).max(0.0), o), px(3.0), th.accent, px(0.0), th.accent, BorderStyle::Solid));
    window.paint_quad(quad(b(x - 10.0, y - 10.0, 20.0, 20.0, o), px(10.0), gpui::rgb(0xffffff), px(1.0), th.shade, BorderStyle::Solid));
    flush_texts(window, cx, bounds, texts);
}

/// Fader/pan geometry is native; the compiled control owns every edit phase,
/// keyboard and assistive route. MixerDrag remains only an audio preview buffer.
fn mixer_scalar(editor: Entity<Editor>, track: Option<TrackId>, control: MixerControl, raw: i32) -> impl IntoElement {
    use crate::proto_scalar::{EventKind, Sample, Spec, scalar};
    let pan = control == MixerControl::Pan;
    let value = raw as f64 / if pan { 10.0 } else { 1000.0 };
    let label = match (track.is_some(), pan) {
        (true, true) => "轨道声像", (true, false) => "轨道推子", (false, true) => "主声像", (false, false) => "主推子",
    };
    let mut spec = Spec::slider(value, if pan { -100.0 } else { -96.0 }, if pan { 100.0 } else { 6.0 }, label);
    spec.step = if pan { 1.0 } else { 0.5 };
    spec.fine_step = if pan { 0.1 } else { 0.1 };
    spec.default_value = 0.0;
    spec.preview_echo = true;
    spec.mixer_preview = true;
    let key = SharedString::from(format!("mixer-scalar-{:?}-{}", track, if pan { "pan" } else { "gain" }));
    let visual_editor = editor.clone();
    let valid_track = track.clone();
    scalar(editor, key, spec,
        move |sample, bounds, initial| {
            if pan {
                let delta = sample.total_delta.x / (f64::from(bounds.size.width) - 16.0).max(1.0) * 200.0;
                Sample { value: initial + delta, fine: initial + delta * 0.1, jump: false }
            } else {
                let delta = -sample.total_delta.y / (f64::from(bounds.size.height) - 14.0).max(1.0);
                let origin = fader_pos((initial * 1000.0).round() as i32);
                Sample { value: fader_mdb(origin + delta) as f64 / 1000.0,
                    fine: fader_mdb(origin + delta * 0.25) as f64 / 1000.0, jump: false }
            }
        },
        move |display, _, _, cx| {
            let th = visual_editor.read(cx).theme;
            canvas(|_, _, _| {}, move |bounds, _, window, cx| {
                if !pan { paint_fader_surface(window, cx, bounds, th, (display.value * 1000.0).round() as i32); }
                else {
                    let o = bounds.origin;
                    let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
                    window.paint_quad(quad(b(0.0, h / 2.0 - 3.0, w, 6.0, o), px(3.0), th.control_pressed, px(0.0), th.control_pressed, BorderStyle::Solid));
                    window.paint_quad(fill(b(w / 2.0 - 0.5, h / 2.0 - 6.0, 1.0, 12.0, o), th.text_tertiary));
                    let x = (display.value / 100.0 + 1.0) / 2.0 * (w - 16.0) + 8.0;
                    let (a, z) = if x < w / 2.0 { (x, w / 2.0) } else { (w / 2.0, x) };
                    window.paint_quad(quad(b(a, h / 2.0 - 3.0, z - a, 6.0, o), px(3.0), th.accent, px(0.0), th.accent, BorderStyle::Solid));
                    window.paint_quad(quad(b(x - 8.0, h / 2.0 - 8.0, 16.0, 16.0, o), px(8.0), gpui::rgb(0xffffff), px(1.0), th.shade, BorderStyle::Solid));
                }
                if display.focus_visible {
                    window.paint_quad(quad(bounds, px(4.0), gpui::rgba(0), px(1.5), th.accent, BorderStyle::Solid));
                }
            }).size_full().into_any_element()
        },
        move |ed, event, _, cx| {
            let raw = (event.value * if pan { 10.0 } else { 1000.0 }).round() as i32;
            let owned = if let Some(track) = &track { ed.mixer_drag.as_ref().is_some_and(|d| &d.track == track && d.control == control) }
                else { ed.master_drag.as_ref().is_some_and(|d| d.control == control) };
            match event.kind {
                EventKind::Start => {
                    if let Some(track) = &track { ed.begin_mixer_drag(control, track, 0.0, event.value); }
                    else { ed.begin_master_drag(control, 0.0, event.value); }
                }
                EventKind::Preview | EventKind::Commit if owned => {
                    if track.is_some() {
                        ed.update_mixer_drag(|header| if pan { header.pan_permille = raw; } else { header.gain_millidecibels = raw; });
                        if event.kind == EventKind::Commit { ed.end_mixer_drag(cx); }
                    } else {
                        ed.update_master_drag(|settings| if pan { settings.pan_permille = raw; } else { settings.gain_millidecibels = raw; });
                        if event.kind == EventKind::Commit { ed.end_master_drag(cx); }
                    }
                }
                EventKind::Commit => {
                    if let Some(track) = &track { ed.edit_header(track, label, cx,
                        |header| if pan { header.pan_permille = raw; } else { header.gain_millidecibels = raw; }); }
                    else { ed.edit_master_bus(label, cx,
                        |settings| if pan { settings.pan_permille = raw; } else { settings.gain_millidecibels = raw; }); }
                }
                EventKind::Cancel if owned => {
                    if track.is_some() { ed.mixer_drag = None; } else { ed.master_drag = None; }
                    ed.sync_mixer();
                }
                _ => {},
            }
            cx.notify();
        }).valid_when(move |ed| {
            valid_track.as_ref().is_none_or(|track| ed.project().track(track).is_some())
                && ed.mixer_drag.as_ref().is_none_or(|drag| valid_track.as_ref() == Some(&drag.track) && drag.control == control)
                && ed.master_drag.as_ref().is_none_or(|drag| valid_track.is_none() && drag.control == control)
        })
}
fn pan_control(editor: Entity<Editor>, id: TrackId, pan: i32) -> impl IntoElement { mixer_scalar(editor, Some(id), MixerControl::Pan, pan) }
fn fader(editor: Entity<Editor>, id: TrackId, mdb: i32) -> impl IntoElement { mixer_scalar(editor, Some(id), MixerControl::Fader, mdb) }
fn master_pan_control(editor: Entity<Editor>, pan: i32) -> impl IntoElement { mixer_scalar(editor, None, MixerControl::Pan, pan) }
fn master_fader(editor: Entity<Editor>, mdb: i32) -> impl IntoElement { mixer_scalar(editor, None, MixerControl::Fader, mdb) }

#[cfg(test)]
mod pointer_boundary_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, Window, WindowHandle, size};

    struct Harness { editor: Entity<Editor>, master: bool, control: MixerControl }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let track = self.editor.read(cx).session.track.clone();
            let control = match (self.master, self.control) {
                (false, MixerControl::Pan) => pan_control(self.editor.clone(), track, 0).into_any_element(),
                (false, MixerControl::Fader) => fader(self.editor.clone(), track, 0).into_any_element(),
                (true, MixerControl::Pan) => master_pan_control(self.editor.clone(), 0).into_any_element(),
                (true, MixerControl::Fader) => master_fader(self.editor.clone(), 0).into_any_element(),
            };
            div().size_full().child(div().w(px(140.0)).h(px(120.0)).child(control))
                .child(canvas(|_, _, _| {}, |_, _, window, _| {
                    window.on_mouse_event(|_: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Bubble { cx.stop_propagation(); }
                    });
                }).size(px(0.0)))
        }
    }
    fn open(cx: &mut TestAppContext, master: bool, control: MixerControl) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(320.0), px(200.0)), |window, cx| Harness {
            editor: cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx)), master, control,
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window
    }
    fn down(host: &Harness, window: &mut Window, cx: &mut gpui::App) {
        let label=match(host.master,host.control) {
            (false,MixerControl::Pan)=>"轨道声像",(false,MixerControl::Fader)=>"轨道推子",
            (true,MixerControl::Pan)=>"主声像",(true,MixerControl::Fader)=>"主推子",
        };
        let owner=verdant_continuous_control::native::gpui_native_owner::all_owners().into_iter().find(|owner| {
            let owner=owner.borrow();owner.window_id==Some(window.window_handle().window_id())
                && owner.props.get("label").and_then(serde_json::Value::as_str)==Some(label)
        }).expect("mixer renders an actual compiled scalar");
        let source=verdant_continuous_control::ContinuousControlHandle{owner};
        let position=point(px(70.0),px(60.0));
        assert!(source.owner.borrow().bounds.is_some_and(|bounds|bounds.contains(&position)),
            "compiled mixer scalar must cover the native fader/pan allocation; bounds={:?}",source.owner.borrow().bounds);
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position, button: MouseButton::Left, ..Default::default()
        }), cx);
        assert!(if host.master { host.editor.read(cx).master_drag.is_some() } else { host.editor.read(cx).mixer_drag.is_some() },
            "accepted authored scalar starts its matching audio preview; source_phase={:?}, failures={:?}",
            source.state("phase"),source.take_failures());
        assert!(window.captured_hitbox().is_some());
    }
    #[gpui::test]
    fn all_mixer_controls_discard_outside_motion_or_terminal_sample(cx: &mut TestAppContext) {
        for master in [false, true] { for control in [MixerControl::Pan, MixerControl::Fader] { for terminal in [false, true] {
            let window = open(cx, master, control);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: point(px(100.0), px(30.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                let outside = point(px(-20.0), px(60.0));
                if terminal {
                    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: outside, button: MouseButton::Left, ..Default::default() }), cx);
                } else {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent { position: outside, pressed_button: Some(MouseButton::Left), ..Default::default() }), cx);
                }
                assert!(host.editor.read(cx).mixer_drag.is_none() && host.editor.read(cx).master_drag.is_none());
                assert!(window.captured_hitbox().is_none(), "outside motion cancels before a leave/up event");
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: point(px(100.0), px(30.0)), button: MouseButton::Left, ..Default::default() }), cx);
                assert_eq!(host.editor.read(cx).revision().get(), 0);
            }).unwrap();
        } } }
    }
    #[gpui::test]
    fn all_mixer_controls_commit_the_in_window_up_sample_outside_the_control(cx: &mut TestAppContext) {
        for master in [false, true] { for control in [MixerControl::Pan, MixerControl::Fader] {
            let window = open(cx, master, control);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                    position: point(px(240.0), px(170.0)), button: MouseButton::Left, ..Default::default()
                }), cx);
                assert_eq!(host.editor.read(cx).revision().get(), 1, "a stopped bubble cannot discard the valid terminal sample");
                assert!(host.editor.read(cx).mixer_drag.is_none() && host.editor.read(cx).master_drag.is_none());
                assert!(window.captured_hitbox().is_none());
            }).unwrap();
        } }
    }
}
