//! Graphical DAW Automation Lanes displayed inline under tracks in Arrangement.
//! Replaces semicolon-delimited millidecibel/permille text entry with direct
//! click-to-add, drag-to-move, and right-click Linear/Step point editing.

use std::collections::{HashMap, HashSet};

use gpui::{
    BorderStyle, ClickEvent, Context, DispatchPhase, Entity, Global, HitboxBehavior, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Role, SharedString, Window, canvas,
    div, fill, point, prelude::*, px, quad,
};
use verdant_domain::automation::{
    AUTOMATION_KEY, AutomationDocument, AutomationLane, AutomationOwner, AutomationPoint,
    AutomationTarget, Interpolation,
};
use verdant_domain::{Command, CommandBatch, Revision, TrackId};

use crate::arrange::HEADER_W;
use crate::editor::{Editor, MenuAction};
use crate::menu::MenuItem;
use crate::paint::{Texts, b, flush_texts, polyline};
use crate::theme::with_alpha;
use crate::ui_controls::{format_gain_db, format_pan_percent, selectable_button};

pub const AUTO_LANE_H: f64 = 74.0;
const PAD_Y: f64 = 8.0;

#[derive(Clone, Debug, Default)]
pub struct AutomationUiState {
    pub expanded_tracks: HashSet<TrackId>,
    pub selected_targets: HashMap<TrackId, AutomationTarget>,
    pub show_master: bool,
    pub master_target: Option<AutomationTarget>,
}

impl AutomationUiState {
    pub fn track_target(&self, track_id: &TrackId) -> AutomationTarget {
        self.selected_targets
            .get(track_id)
            .cloned()
            .unwrap_or_else(|| AutomationTarget::TrackGain { track_id: track_id.clone() })
    }

    pub fn master_target(&self) -> AutomationTarget {
        self.master_target.clone().unwrap_or(AutomationTarget::MasterGain)
    }

    pub fn toggle_track(&mut self, track_id: &TrackId) {
        if !self.expanded_tracks.remove(track_id) {
            self.expanded_tracks.insert(track_id.clone());
        }
    }
}

/// Domain value bounds `(min, max)` for UI vertical mapping of an `AutomationTarget`.
pub fn target_domain_range(ed: &Editor, target: &AutomationTarget) -> (f64, f64) {
    match target {
        AutomationTarget::TrackGain { .. } | AutomationTarget::MasterGain => (-48_000.0, 6_000.0),
        AutomationTarget::TrackPan { .. } | AutomationTarget::MasterPan => (-1_000.0, 1_000.0),
        AutomationTarget::PluginParameter { owner, instance_id, parameter_id } => {
            let handle = match owner {
                AutomationOwner::Track { track_id } => ed.insert_specs(track_id)
                    .iter()
                    .position(|s| &s.instance_id == instance_id)
                    .and_then(|slot| ed.live_insert(track_id, slot)),
                AutomationOwner::Master => ed.master_insert_specs()
                    .iter()
                    .position(|s| &s.instance_id == instance_id)
                    .and_then(|slot| ed.live_master_insert(slot)),
            };
            if let Some(h) = handle {
                if let Some(params) = ed.plugin_params.get(&h.key()) {
                    if let Some(p) = params.iter().find(|p| &p.id == parameter_id) {
                        if p.min.is_finite() && p.max.is_finite() && p.max > p.min {
                            return (p.min, p.max);
                        }
                    }
                }
            }
            (0.0, 1.0)
        }
    }
}

pub fn format_automation_value(target: &AutomationTarget, raw_value: f64) -> String {
    match target {
        AutomationTarget::TrackGain { .. } | AutomationTarget::MasterGain => format_gain_db(raw_value / 1000.0),
        AutomationTarget::TrackPan { .. } | AutomationTarget::MasterPan => format_pan_percent(raw_value / 10.0),
        AutomationTarget::PluginParameter { .. } => format!("{raw_value:.2}"),
    }
}

pub fn short_target_name(ed: &Editor, target: &AutomationTarget) -> String {
    match target {
        AutomationTarget::TrackGain { .. } => "音量 (dB)".into(),
        AutomationTarget::TrackPan { .. } => "声像 (%)".into(),
        AutomationTarget::MasterGain => "主音量 (dB)".into(),
        AutomationTarget::MasterPan => "主声像 (%)".into(),
        AutomationTarget::PluginParameter { owner, instance_id, parameter_id } => {
            let plug_name = match owner {
                AutomationOwner::Track { track_id } => ed.insert_specs(track_id)
                    .into_iter()
                    .find(|s| &s.instance_id == instance_id)
                    .map(|s| s.name)
                    .unwrap_or_else(|| "插件".into()),
                AutomationOwner::Master => ed.master_insert_specs()
                    .into_iter()
                    .find(|s| &s.instance_id == instance_id)
                    .map(|s| s.name)
                    .unwrap_or_else(|| "主插件".into()),
            };
            format!("{plug_name} · {parameter_id}")
        }
    }
}

pub fn default_fallback_value(ed: &Editor, target: &AutomationTarget) -> f64 {
    match target {
        AutomationTarget::TrackGain { track_id } => ed.project().track(track_id).map_or(0.0, |t| t.gain_millidecibels as f64),
        AutomationTarget::TrackPan { track_id } => ed.project().track(track_id).map_or(0.0, |t| t.pan_permille as f64),
        AutomationTarget::MasterGain => crate::mix::MasterBusSettings::from_project(ed.project()).gain_millidecibels as f64,
        AutomationTarget::MasterPan => crate::mix::MasterBusSettings::from_project(ed.project()).pan_permille as f64,
        AutomationTarget::PluginParameter { .. } => {
            let (lo, hi) = target_domain_range(ed, target);
            (lo + hi) * 0.5
        }
    }
}

fn value_to_y(val: f64, range: (f64, f64), height: f64) -> f64 {
    let frac = ((val - range.0) / (range.1 - range.0).max(1e-9)).clamp(0.0, 1.0);
    PAD_Y + (1.0 - frac) * (height - PAD_Y * 2.0).max(1.0)
}

fn y_to_value(y: f64, range: (f64, f64), height: f64, target: &AutomationTarget) -> f64 {
    let frac = 1.0 - ((y - PAD_Y) / (height - PAD_Y * 2.0).max(1.0)).clamp(0.0, 1.0);
    let raw = range.0 + frac * (range.1 - range.0);
    match target {
        AutomationTarget::TrackGain { .. } | AutomationTarget::MasterGain => (raw / 100.0).round() * 100.0,
        AutomationTarget::TrackPan { .. } | AutomationTarget::MasterPan => (raw / 10.0).round() * 10.0,
        AutomationTarget::PluginParameter { .. } => raw,
    }
}

#[derive(Default)]
struct AutoPointDragState {
    drag: Option<AutoPointDrag>,
}
impl Global for AutoPointDragState {}

#[derive(Clone)]
struct AutoPointDrag {
    editor: gpui::EntityId,
    target: AutomationTarget,
    base: Revision,
    original_tick: Option<i64>,
    tick: i64,
    value: f64,
    interpolation: Interpolation,
}

/// Renders the left header box for an expanded track or master automation lane.
pub fn automation_header(
    ed: &Editor,
    owner_track: Option<TrackId>,
    target: AutomationTarget,
    cx: &mut Context<Editor>,
) -> impl IntoElement {
    let th = ed.theme;
    let doc = AutomationDocument::from_project(ed.project()).unwrap_or_default();
    let lane = doc.lane(&target);
    let enabled = lane.is_none_or(|l| l.enabled);
    let playhead_tick = ed.tempo().tick_at(ed.playhead_seconds());
    let fallback = default_fallback_value(ed, &target);
    let live_val = lane.map_or(fallback, |l| l.value_at(playhead_tick, fallback));
    let label = short_target_name(ed, &target);
    let id_key = owner_track.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "master".into());
    let (pick_owner, toggle_target, clear_target) = (owner_track.clone(), target.clone(), target.clone());

    div()
        .id(SharedString::from(format!("auto-header-{id_key}")))
        .w(px(HEADER_W))
        .h(px(AUTO_LANE_H as f32))
        .flex_none()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(4.0))
        .px(px(10.0))
        .bg(th.lcd)
        .border_b_1()
        .border_r_1()
        .border_color(th.shade)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .id(SharedString::from(format!("auto-target-{id_key}")))
                        .role(Role::Button)
                        .aria_label("切换自动化目标参数")
                        .flex_1()
                        .h(px(22.0))
                        .px(px(6.0))
                        .rounded(px(4.0))
                        .bg(th.control)
                        .border_1()
                        .border_color(th.shade)
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_xs()
                        .child(div().truncate().child(format!("自动化 · {label}")))
                        .child("▾")
                        .on_click(cx.listener(move |ed, ev: &ClickEvent, window, cx| {
                            show_automation_target_picker(ed, pick_owner.clone(), ev.position(), window, cx);
                            cx.stop_propagation();
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(4.0))
                .text_xs()
                .child(
                    div()
                        .text_color(if enabled { th.accent } else { th.text_tertiary })
                        .child(format_automation_value(&target, live_val)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(4.0))
                        .child(
                            selectable_button(
                                SharedString::from(format!("auto-enable-{id_key}")),
                                if enabled { "启用" } else { "旁通" },
                                enabled,
                                th,
                            )
                            .h(px(20.0))
                            .px(px(5.0))
                            .on_click(cx.listener(move |ed, _, _, cx| {
                                ed.toggle_automation_lane_enabled(&toggle_target, cx);
                            })),
                        )
                        .child(
                            selectable_button(SharedString::from(format!("auto-clear-{id_key}")), "清空", false, th)
                                .h(px(20.0))
                                .px(px(5.0))
                                .on_click(cx.listener(move |ed, _, _, cx| {
                                    ed.clear_automation_lane(&clear_target, cx);
                                })),
                        ),
                ),
        )
}

fn show_automation_target_picker(
    ed: &mut Editor,
    owner_track: Option<TrackId>,
    anchor: gpui::Point<gpui::Pixels>,
    window: &mut Window,
    cx: &mut Context<Editor>,
) {
    let mut items = Vec::new();
    if let Some(tid) = owner_track {
        items.push(MenuItem::action(
            "轨道音量 (dB)",
            MenuAction::SelectAutomationTarget {
                track: Some(tid.clone()),
                target: AutomationTarget::TrackGain { track_id: tid.clone() },
            },
        ));
        items.push(MenuItem::action(
            "轨道声像 (%)",
            MenuAction::SelectAutomationTarget {
                track: Some(tid.clone()),
                target: AutomationTarget::TrackPan { track_id: tid.clone() },
            },
        ));
        for (slot, spec) in ed.insert_specs(&tid).into_iter().enumerate() {
            if spec.instance_id.is_empty() {
                continue;
            }
            if let Some(handle) = ed.live_insert(&tid, slot) {
                if let Ok(params) = handle.params() {
                    items.push(MenuItem::separator());
                    items.push(MenuItem::disabled(format!("插件 · {}", spec.name)));
                    for p in params {
                        items.push(MenuItem::action(
                            format!("{} [{:.1}..{:.1}]", p.name, p.min, p.max),
                            MenuAction::SelectAutomationTarget {
                                track: Some(tid.clone()),
                                target: AutomationTarget::PluginParameter {
                                    owner: AutomationOwner::Track { track_id: tid.clone() },
                                    instance_id: spec.instance_id.clone(),
                                    parameter_id: p.id,
                                },
                            },
                        ));
                    }
                }
            }
        }
    } else {
        items.push(MenuItem::action(
            "主总线音量 (dB)",
            MenuAction::SelectAutomationTarget { track: None, target: AutomationTarget::MasterGain },
        ));
        items.push(MenuItem::action(
            "主总线声像 (%)",
            MenuAction::SelectAutomationTarget { track: None, target: AutomationTarget::MasterPan },
        ));
    }
    ed.show_menu(anchor, items, window, cx);
}

/// Renders the right-hand interactive graphical automation curve aligned with `ed.arrange`.
pub fn automation_curve_canvas(editor: Entity<Editor>, target: AutomationTarget) -> impl IntoElement {
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |bounds, hitbox, window, cx| {
            if cx.try_global::<AutoPointDragState>().is_none() {
                cx.set_global(AutoPointDragState::default());
            }
            let mut texts = Texts::new();
            let target_paint = target.clone();
            {
                let ed = editor.read(cx);
                let th = ed.theme;
                let vp = ed.arrange;
                let o = bounds.origin;
                let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
                let (from, to) = (vp.start_tick.floor() as i64, vp.x_to_tick(w).ceil() as i64);
                let range = target_domain_range(ed, &target_paint);
                let fallback = default_fallback_value(ed, &target_paint);
                let doc = AutomationDocument::from_project(ed.project()).unwrap_or_default();
                let stored_lane = doc.lane(&target_paint);
                let enabled = stored_lane.is_none_or(|l| l.enabled);

                window.paint_quad(fill(bounds, th.roll_black));
                window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                    for bar_tick in ed.meter().bars(from, to) {
                        window.paint_quad(fill(b(vp.tick_to_x(bar_tick as f64), 0.0, 1.0, h, o), th.grid_beat));
                    }
                    let base_y = value_to_y(fallback, range, h);
                    window.paint_quad(fill(b(0.0, base_y, w, 1.0, o), with_alpha(th.text_tertiary, 0.35)));

                    let active_drag = cx.global::<AutoPointDragState>()
                        .drag
                        .as_ref()
                        .filter(|d| d.editor == editor.entity_id() && d.target == target_paint);

                    let mut working = stored_lane.cloned().unwrap_or_else(|| AutomationLane::new(target_paint.clone()));
                    if let Some(drag) = active_drag {
                        if let Some(orig) = drag.original_tick {
                            working.remove_point(orig);
                        }
                        let _ = working.set_point(AutomationPoint {
                            tick: drag.tick,
                            value: drag.value,
                            interpolation: drag.interpolation,
                            unknown: Default::default(),
                        });
                    }

                    let line_col = if enabled { th.accent } else { th.text_tertiary };
                    if working.points.is_empty() {
                        window.paint_quad(fill(b(0.0, base_y - 0.75, w, 1.5, o), with_alpha(line_col, 0.65)));
                        texts.push((
                            "点击添加自动化控制点 · 拖拽调整 · 右键切换阶梯/删除".into(),
                            point(o.x + px(8.0), o.y + px(4.0)),
                            9.5,
                            th.text_tertiary,
                        ));
                    } else {
                        let mut poly = Vec::new();
                        let first_x = vp.tick_to_x(working.points[0].tick as f64);
                        if first_x > 0.0 {
                            poly.push(Some((0.0, base_y)));
                            poly.push(Some((first_x, base_y)));
                        }
                        for (i, pt) in working.points.iter().enumerate() {
                            let x = vp.tick_to_x(pt.tick as f64);
                            let y = value_to_y(pt.value, range, h);
                            poly.push(Some((x, y)));
                            if pt.interpolation == Interpolation::Step {
                                if let Some(next) = working.points.get(i + 1) {
                                    poly.push(Some((vp.tick_to_x(next.tick as f64), y)));
                                }
                            }
                        }
                        if let Some(last) = working.points.last() {
                            let lx = vp.tick_to_x(last.tick as f64);
                            let ly = value_to_y(last.value, range, h);
                            if lx < w {
                                poly.push(Some((w, ly)));
                            }
                        }
                        polyline(window, o, poly, 1.75, line_col);

                        for pt in &working.points {
                            let x = vp.tick_to_x(pt.tick as f64);
                            if !(-8.0..=w + 8.0).contains(&x) {
                                continue;
                            }
                            let y = value_to_y(pt.value, range, h);
                            let is_step = pt.interpolation == Interpolation::Step;
                            window.paint_quad(quad(
                                b(x - 4.0, y - 4.0, 8.0, 8.0, o),
                                px(if is_step { 1.5 } else { 4.0 }),
                                if active_drag.is_some_and(|d| d.tick == pt.tick) { th.warning } else { line_col },
                                px(1.0),
                                th.surface,
                                BorderStyle::Solid,
                            ));
                        }
                    }

                    if let Some(drag) = active_drag {
                        let dx = vp.tick_to_x(drag.tick as f64).clamp(6.0, (w - 96.0).max(6.0));
                        let dy = (value_to_y(drag.value, range, h) - 18.0).clamp(2.0, (h - 16.0).max(2.0));
                        texts.push((
                            format_automation_value(&target_paint, drag.value),
                            point(o.x + px(dx as f32), o.y + px(dy as f32)),
                            10.0,
                            th.warning,
                        ));
                    }
                });
                window.paint_quad(fill(b(0.0, h - 1.0, w, 1.0, o), th.shade));
            }
            flush_texts(window, cx, bounds, texts);

            let (e_down, h_down, t_down) = (editor.clone(), hitbox.clone(), target.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || !h_down.is_hovered(window) {
                    return;
                }
                let local = ev.position - bounds.origin;
                let (lx, ly) = (local.x.as_f32() as f64, local.y.as_f32() as f64);
                let h = bounds.size.height.as_f32() as f64;

                e_down.update(cx, |ed, cx| {
                    let vp = ed.arrange;
                    let range = target_domain_range(ed, &t_down);
                    let doc = AutomationDocument::from_project(ed.project()).unwrap_or_default();
                    let hit_pt = doc.lane(&t_down).and_then(|lane| {
                        lane.points
                            .iter()
                            .find(|pt| (vp.tick_to_x(pt.tick as f64) - lx).abs() <= 7.0 && (value_to_y(pt.value, range, h) - ly).abs() <= 8.0)
                            .cloned()
                    });

                    if ev.button == MouseButton::Right {
                        if let Some(pt) = hit_pt {
                            let next_interp = if pt.interpolation == Interpolation::Step {
                                Interpolation::Linear
                            } else {
                                Interpolation::Step
                            };
                            let items = vec![
                                MenuItem::action(
                                    if pt.interpolation == Interpolation::Step { "切换为线性插值 (Linear)" } else { "切换为阶梯保持 (Step)" },
                                    MenuAction::AutomationPointInterp {
                                        target: t_down.clone(),
                                        tick: pt.tick,
                                        interpolation: next_interp,
                                    },
                                ),
                                MenuItem::separator(),
                                MenuItem::action(
                                    "删除此控制点",
                                    MenuAction::DeleteAutomationPoint { target: t_down.clone(), tick: pt.tick },
                                ),
                            ];
                            ed.show_menu(ev.position, items, window, cx);
                        }
                        cx.stop_propagation();
                        return;
                    }

                    if ev.button != MouseButton::Left {
                        return;
                    }

                    let raw_tick = vp.x_to_tick(lx).max(0.0);
                    let snapped_tick = if ev.modifiers.alt {
                        raw_tick.round() as i64
                    } else {
                        ed.session.snap.snap(raw_tick).max(0)
                    };
                    let val = y_to_value(ly, range, h, &t_down);
                    let drag = if let Some(pt) = hit_pt {
                        AutoPointDrag {
                            editor: e_down.entity_id(),
                            target: t_down.clone(),
                            base: ed.revision(),
                            original_tick: Some(pt.tick),
                            tick: pt.tick,
                            value: pt.value,
                            interpolation: pt.interpolation,
                        }
                    } else {
                        AutoPointDrag {
                            editor: e_down.entity_id(),
                            target: t_down.clone(),
                            base: ed.revision(),
                            original_tick: None,
                            tick: snapped_tick,
                            value: val,
                            interpolation: Interpolation::Linear,
                        }
                    };
                    cx.global_mut::<AutoPointDragState>().drag = Some(drag);
                    window.capture_pointer(h_down.id);
                    cx.notify();
                    cx.stop_propagation();
                });
            });

            let (e_move, t_move) = (editor.clone(), target.clone());
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                let next = {
                    let Some(drag) = cx.try_global::<AutoPointDragState>().and_then(|s| s.drag.as_ref()) else { return };
                    if drag.editor != e_move.entity_id() || drag.target != t_move {
                        return;
                    }
                    let ed = e_move.read(cx);
                    let local = ev.position - bounds.origin;
                    let h = bounds.size.height.as_f32() as f64;
                    let range = target_domain_range(ed, &t_move);
                    let raw_tick = ed.arrange.x_to_tick(local.x.as_f32() as f64).max(0.0);
                    let tick = if ev.modifiers.alt { raw_tick.round() as i64 } else { ed.session.snap.snap(raw_tick).max(0) };
                    let value = y_to_value(local.y.as_f32() as f64, range, h, &t_move);
                    Some((tick, value))
                };
                if let Some((tick, value)) = next {
                    if let Some(drag) = cx.global_mut::<AutoPointDragState>().drag.as_mut() {
                        drag.tick = tick;
                        drag.value = value;
                    }
                    e_move.update(cx, |_, cx| cx.notify());
                }
            });

            let (e_up, t_up) = (editor.clone(), target);
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left {
                    return;
                }
                let matched = cx.try_global::<AutoPointDragState>()
                    .and_then(|s| s.drag.as_ref())
                    .is_some_and(|d| d.editor == e_up.entity_id() && d.target == t_up);
                if !matched {
                    return;
                }
                let Some(drag) = cx.global_mut::<AutoPointDragState>().drag.take() else { return };
                window.release_pointer();
                e_up.update(cx, |ed, cx| {
                    ed.commit_automation_point_drag(
                        drag.base,
                        drag.target,
                        drag.original_tick,
                        drag.tick,
                        drag.value,
                        drag.interpolation,
                        cx,
                    );
                });
            });
        },
    )
    .size_full()
}

impl Editor {
    pub fn commit_automation_point_drag(
        &mut self,
        base: Revision,
        target: AutomationTarget,
        original_tick: Option<i64>,
        tick: i64,
        value: f64,
        interpolation: Interpolation,
        cx: &mut Context<Self>,
    ) {
        if base != self.revision() {
            self.notice("工程已变更，自动化控制点拖动已取消");
            cx.notify();
            return;
        }
        let result = (|| -> Result<(), String> {
            let mut doc = AutomationDocument::from_project(self.project()).map_err(|e| e.to_string())?;
            let mut lane = doc.lane(&target).cloned().unwrap_or_else(|| AutomationLane::new(target.clone()));
            if let Some(old_tick) = original_tick {
                lane.remove_point(old_tick);
            }
            lane.set_point(AutomationPoint { tick, value, interpolation, unknown: Default::default() })
                .map_err(|e| e.to_string())?;
            self.validate_plugin_lane(&lane)?;
            doc.set_lane(lane);
            doc.validate(self.project()).map_err(|e| e.to_string())?;
            let ext = doc.extension_value().map_err(|e| e.to_string())?;
            self.commit(base, CommandBatch::single("编辑自动化控制点", Command::SetProjectExtension {
                key: AUTOMATION_KEY.into(),
                value: ext,
            }), cx);
            Ok(())
        })();
        if let Err(err) = result {
            self.notice(err);
        }
        cx.notify();
    }

    pub fn toggle_automation_lane_enabled(&mut self, target: &AutomationTarget, cx: &mut Context<Self>) {
        let Ok(mut doc) = AutomationDocument::from_project(self.project()) else { return };
        let mut lane = doc.lane(target).cloned().unwrap_or_else(|| AutomationLane::new(target.clone()));
        lane.enabled = !lane.enabled;
        doc.set_lane(lane);
        if let Ok(ext) = doc.extension_value() {
            self.commit(self.revision(), CommandBatch::single("切换自动化曲线开关", Command::SetProjectExtension {
                key: AUTOMATION_KEY.into(),
                value: ext,
            }), cx);
        }
    }

    pub fn clear_automation_lane(&mut self, target: &AutomationTarget, cx: &mut Context<Self>) {
        let Ok(mut doc) = AutomationDocument::from_project(self.project()) else { return };
        if !doc.remove_lane(target) {
            return;
        }
        if let Ok(ext) = doc.extension_value() {
            self.commit(self.revision(), CommandBatch::single("清空自动化曲线", Command::SetProjectExtension {
                key: AUTOMATION_KEY.into(),
                value: ext,
            }), cx);
        }
    }

    pub fn set_automation_point_interp(
        &mut self,
        target: AutomationTarget,
        tick: i64,
        interpolation: Interpolation,
        cx: &mut Context<Self>,
    ) {
        let Ok(mut doc) = AutomationDocument::from_project(self.project()) else { return };
        let Some(mut lane) = doc.lane(&target).cloned() else { return };
        if let Some(pt) = lane.points.iter_mut().find(|p| p.tick == tick) {
            pt.interpolation = interpolation;
            doc.set_lane(lane);
            if let Ok(ext) = doc.extension_value() {
                self.commit(self.revision(), CommandBatch::single("切换自动化插值模式", Command::SetProjectExtension {
                    key: AUTOMATION_KEY.into(),
                    value: ext,
                }), cx);
            }
        }
    }

    pub fn delete_automation_point(&mut self, target: AutomationTarget, tick: i64, cx: &mut Context<Self>) {
        let Ok(mut doc) = AutomationDocument::from_project(self.project()) else { return };
        let Some(mut lane) = doc.lane(&target).cloned() else { return };
        if lane.remove_point(tick) {
            if lane.points.is_empty() {
                doc.remove_lane(&target);
            } else {
                doc.set_lane(lane);
            }
            if let Ok(ext) = doc.extension_value() {
                self.commit(self.revision(), CommandBatch::single("删除自动化控制点", Command::SetProjectExtension {
                    key: AUTOMATION_KEY.into(),
                    value: ext,
                }), cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_and_value_mapping_roundtrips_gain_and_pan() {
        let gain_target = AutomationTarget::MasterGain;
        let gain_range = (-48_000.0, 6_000.0);
        for mdb in [-48_000.0, -12_000.0, 0.0, 6_000.0] {
            let y = value_to_y(mdb, gain_range, 74.0);
            let back = y_to_value(y, gain_range, 74.0, &gain_target);
            assert!((back - mdb).abs() <= 100.0, "{mdb} -> {y} -> {back}");
        }
        let pan_target = AutomationTarget::MasterPan;
        let pan_range = (-1_000.0, 1_000.0);
        for pm in [-1_000.0, -350.0, 0.0, 500.0, 1_000.0] {
            let y = value_to_y(pm, pan_range, 74.0);
            let back = y_to_value(y, pan_range, 74.0, &pan_target);
            assert!((back - pm).abs() <= 10.0, "{pm} -> {y} -> {back}");
        }
    }
}
