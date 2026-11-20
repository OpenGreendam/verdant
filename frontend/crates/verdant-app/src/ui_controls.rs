//! Reusable DAW/SVS UI widgets: active-state buttons, scrubbable musical numeric
//! controls, draggable pane splitters, and timeline scroll/zoom bars.

use gpui::{
    AccessibleAction, App, ClickEvent, Context, CursorStyle, DispatchPhase, Div, Entity,
    FontWeight, Global, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Role, SharedString, Stateful, Window, canvas, div, fill, prelude::*, px,
    svg,
};
use verdant_backend::Route;
use verdant_domain::timing::note_name;

use crate::assets::icon;
use crate::editor::Editor;
use crate::generated::controls::{
    self, CommandArg as Arg, CommandId as Cmd, NumericId as Num, NumericPhase, NumericTarget,
    SelectId,
};
use crate::menu::MenuItem;
use crate::paint::b;
use crate::theme::{Theme, with_alpha};

pub const LAYOUT_PREFS_KEY: &str = "ui_layout";

/// Design-system uniform row height tokens so horizontal toolbars/rows never stair-step.
pub const TOP_BTN_H: f32 = 28.0;
pub const BAR_BTN_H: f32 = 26.0;
pub const PANEL_BTN_H: f32 = 28.0;
pub const HDR_BTN_H: f32 = 22.0;
pub const RIGHT_RAIL_W: f32 = 36.0;

pub fn route_display_name(route: Route) -> &'static str {
    match route {
        Route::VerdantVoice => "verdant-voice",
        Route::C0 => "原始声库",
    }
}

pub fn route_short_name(route: Route) -> &'static str {
    route_display_name(route)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorTab {
    #[default]
    TrackNote,
    Synthesis,
    SystemHistory,
}

/// Window-width classes. Below `Medium` the side panels stop being columns
/// and open as drawers over the timeline, and the toolbar folds its menus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum WidthClass {
    /// Phones and very narrow windows.
    Phone,
    /// Tablets in portrait, half-screen windows.
    Compact,
    /// Laptops and tablets in landscape.
    Medium,
    #[default]
    Wide,
}

impl WidthClass {
    pub fn of(width: f32) -> Self {
        match width {
            w if w < 760.0 => Self::Phone,
            w if w < 1100.0 => Self::Compact,
            w if w < 1440.0 => Self::Medium,
            _ => Self::Wide,
        }
    }
    /// Side panels are docked columns; otherwise they open as drawers.
    pub fn docks_panels(self) -> bool { self >= Self::Medium }
}

/// A side panel shown as an overlay in narrow windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drawer { Left, Right }

/// Toolbar control size: 28 px for a pointer, 36 px once touch input is seen
/// (close to the 44 pt / 48 dp platform guidance within a dense toolbar).
pub fn bar_button_size(touch: bool) -> f32 { if touch { 36.0 } else { TOP_BTN_H } }
pub fn bar_height(touch: bool) -> f32 { if touch { 56.0 } else { 48.0 } }

/// Synthesizer V Studio-style right property dock tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightDockTab {
    /// Note properties now live in the left inspector; layouts saved with the
    /// retired note tab open on the voice tab.
    #[default]
    #[serde(alias = "note_phoneme")]
    VoiceTimbre,
    Dictionary,
    RenderInserts,
    HistorySystem,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LayoutPrefs {
    pub inspector_width: f32,
    pub right_dock_width: f32,
    pub show_right_dock: bool,
    pub right_dock_tab: RightDockTab,
    pub arrange_height: f32,
    pub lane_height: f32,
    pub inspector_tab: InspectorTab,
    pub show_tempo_lane: bool,
    pub show_phoneme_lane: bool,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            inspector_width: 242.0,
            right_dock_width: 292.0,
            show_right_dock: true,
            right_dock_tab: RightDockTab::VoiceTimbre,
            arrange_height: 160.0,
            lane_height: 184.0,
            inspector_tab: InspectorTab::TrackNote,
            show_tempo_lane: false,
            show_phoneme_lane: true,
        }
    }
}

impl LayoutPrefs {
    pub fn clamped(self) -> Self {
        Self {
            inspector_width: if self.inspector_width.is_finite() { self.inspector_width.clamp(210.0, 420.0) } else { 242.0 },
            right_dock_width: if self.right_dock_width.is_finite() { self.right_dock_width.clamp(244.0, 440.0) } else { 292.0 },
            show_right_dock: self.show_right_dock,
            right_dock_tab: self.right_dock_tab,
            arrange_height: if self.arrange_height.is_finite() { self.arrange_height.clamp(92.0, 460.0) } else { 160.0 },
            lane_height: if self.lane_height.is_finite() { self.lane_height.clamp(112.0, 420.0) } else { 184.0 },
            inspector_tab: self.inspector_tab,
            show_tempo_lane: self.show_tempo_lane,
            show_phoneme_lane: self.show_phoneme_lane,
        }
    }

    pub fn from_prefs(prefs: &crate::files::Prefs) -> Self {
        prefs.extra.get(LAYOUT_PREFS_KEY)
            .and_then(|v| serde_json::from_value::<Self>(v.clone()).ok())
            .map(Self::clamped)
            .unwrap_or_default()
    }

    pub fn store(&self, prefs: &mut crate::files::Prefs) {
        if let Ok(val) = serde_json::to_value(self.clamped()) {
            prefs.extra.insert(LAYOUT_PREFS_KEY.into(), val);
        }
    }
}

/// Ordinary button with unmistakable visual active state, uniform `BAR_BTN_H` height, and WCAG >= 4.5 contrast.
pub fn selectable_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    active: bool,
    th: Theme,
) -> Stateful<Div> {
    let label = label.into();
    div()
        .id(id.into())
        .role(Role::Button)
        .aria_label(label.clone())
        .aria_selected(active)
        .h(px(BAR_BTN_H))
        .px(px(8.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(if active { th.accent } else { th.control })
        .border_1()
        .border_color(if active { th.accent } else { th.control })
        .text_xs()
        .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(if active { th.accent_text } else { th.text })
        .hover(move |s| s.bg(if active { th.accent } else { th.control_pressed }))
        .child(label)
}

/// Button with a leading Lucide SVG icon and uniform `BAR_BTN_H` height.
pub fn icon_selectable_button(
    id: impl Into<SharedString>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    th: Theme,
) -> Stateful<Div> {
    let label = label.into();
    let fg = if active { th.accent_text } else { th.text };
    div()
        .id(id.into())
        .role(Role::Button)
        .aria_label(label.clone())
        .aria_selected(active)
        .h(px(BAR_BTN_H))
        .px(px(8.0))
        .gap(px(5.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(if active { th.accent } else { th.control })
        .border_1()
        .border_color(if active { th.accent } else { th.control })
        .text_xs()
        .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(fg)
        .hover(move |s| s.bg(if active { th.accent } else { th.control_pressed }))
        .child(svg().path(icon(icon_name)).size(px(13.5)).flex_none().text_color(fg))
        .child(label)
}

/// Native-style `<Select>` dropdown trigger with optional leading Lucide icon and trailing chevron.
pub fn select_button(
    id: impl Into<SharedString>,
    leading_icon: Option<&'static str>,
    label: impl Into<SharedString>,
    aria_name: impl Into<SharedString>,
    th: Theme,
) -> Stateful<Div> {
    let label = label.into();
    div()
        .id(id.into())
        .role(Role::Button)
        .aria_label(aria_name.into())
        .h(px(PANEL_BTN_H))
        .px(px(8.0))
        .gap(px(6.0))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .bg(th.control)
        .border_1()
        .border_color(th.control)
        .text_xs()
        .text_color(th.text)
        .hover(move |s| s.bg(th.control_pressed))
        .when_some(leading_icon, |d, ic| {
            d.child(svg().path(icon(ic)).size(px(13.5)).flex_none().text_color(th.text_secondary))
        })
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(svg().path(icon("chevron-down")).size(px(13.0)).flex_none().text_color(th.text_secondary))
}

/// Command button with explicit visual active state and uniform `BAR_BTN_H` height.
pub fn selectable_command_button(
    ed: &Editor,
    cmd: Cmd,
    arg: Arg,
    label: impl Into<SharedString>,
    key: impl Into<gpui::ElementId>,
    active: bool,
    cx: &mut Context<Editor>,
) -> Stateful<Div> {
    let label = label.into();
    let th = ed.theme;
    controls::command_button(
        cmd,
        arg,
        label.clone(),
        key,
        if active { th.accent_text.into() } else { th.text.into() },
        if active { th.accent.into() } else { th.control.into() },
        if active { th.accent.into() } else { th.control_pressed.into() },
        cx,
    )
    .role(Role::Button)
    .aria_label(label)
    .aria_selected(active)
    .h(px(BAR_BTN_H))
    .py(px(0.0))
    .px(px(8.0))
    .rounded(px(6.0))
    .border_1()
    .border_color(if active { th.accent } else { th.control })
    .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
}

/// libadwaita "flat" button for header bars: no fill until hovered; the
/// accent fill marks a toggle that is on.
pub fn flat_command_button(
    ed: &Editor,
    cmd: Cmd,
    arg: Arg,
    label: impl Into<SharedString>,
    key: impl Into<gpui::ElementId>,
    active: bool,
    cx: &mut Context<Editor>,
) -> Stateful<Div> {
    let label = label.into();
    let th = ed.theme;
    let clear = gpui::Rgba { a: 0.0, ..th.headerbar };
    controls::command_button(
        cmd,
        arg,
        label.clone(),
        key,
        if active { th.accent_text.into() } else { th.text.into() },
        if active { th.accent.into() } else { clear.into() },
        if active { th.accent.into() } else { th.hover.into() },
        cx,
    )
    .role(Role::Button)
    .aria_label(label)
    .aria_selected(active)
    .py(px(0.0))
    .rounded(px(6.0))
}

/// Command button with a leading Lucide SVG icon, text label, and uniform `BAR_BTN_H` height.
pub fn icon_command_button(
    ed: &Editor,
    cmd: Cmd,
    arg: Arg,
    icon_name: &'static str,
    label: impl Into<SharedString>,
    key: impl Into<gpui::ElementId>,
    active: bool,
    cx: &mut Context<Editor>,
) -> Stateful<Div> {
    let label = label.into();
    let th = ed.theme;
    let fg = if active { th.accent_text } else { th.text };
    selectable_command_button(ed, cmd, arg, "", key, active, cx)
        .aria_label(label.clone())
        .gap(px(5.0))
        .child(svg().path(icon(icon_name)).size(px(13.5)).flex_none().text_color(fg))
        .child(label)
}

/// Flat header-bar button with a leading icon and a text label.
pub fn flat_icon_command_button(
    ed: &Editor,
    cmd: Cmd,
    icon_name: &'static str,
    label: impl Into<SharedString>,
    key: impl Into<gpui::ElementId>,
    active: bool,
    cx: &mut Context<Editor>,
) -> Stateful<Div> {
    let label = label.into();
    let th = ed.theme;
    let fg = if active { th.accent_text } else { th.text };
    flat_command_button(ed, cmd, Arg::None, "", key, active, cx)
        .aria_label(label.clone())
        .px(px(9.0))
        .gap(px(6.0))
        .text_sm()
        .child(svg().path(icon(icon_name)).size(px(15.0)).flex_none().text_color(fg))
        .child(label)
}

/// Select dropdown shell with uniform height and trailing Lucide chevron.
pub fn generated_select_button(
    ed: &Editor,
    id: SelectId,
    arg: Arg,
    key: impl Into<gpui::ElementId>,
    cx: &mut Context<Editor>,
) -> Stateful<Div> {
    let th = ed.theme;
    let current = ed.current_select_option(id, &arg);
    let spec = controls::select(id);
    controls::select_control(
        id,
        arg,
        current,
        key,
        th.text.into(),
        th.control.into(),
        th.control_pressed.into(),
        cx,
    )
    .role(Role::Button)
    .aria_label(spec.label)
    .h(px(PANEL_BTN_H))
    .py(px(0.0))
    .gap(px(6.0))
    .rounded(px(6.0))
    .border_1()
    .border_color(th.control)
    .child(svg().path(icon("chevron-down")).size(px(13.0)).flex_none().text_color(th.text_secondary))
}

pub fn numeric_label_zh(id: Num) -> &'static str {
    match id {
        Num::TrackGainDb => "增益",
        Num::TrackPan => "声像",
        Num::MasterGainDb => "主推子",
        Num::MasterPan => "主声像",
        Num::TempoBpm => "速度",
        Num::SnapGridTicks => "网格",
        Num::DefaultNoteTicks => "新音符",
        Num::NoteStartTick => "起点",
        Num::NoteDurationTicks => "时值",
        Num::NotePitchSemitone => "音高",
        Num::NoteDetuneCents => "微调",
        Num::VibratoPhaseDegrees => "颤音相位",
        Num::PortamentoCurvePercent => "滑音曲率",
        Num::VibratoRateHz => "颤音速率",
        Num::VibratoDepthCents => "颤音深度",
        Num::VibratoDelayPercent => "颤音起始",
        Num::PortamentoMs => "入音滑音",
        Num::PhonemeInitialPercent => "初声占比",
        Num::PhonemeFinalPercent => "韵母占比",
        Num::PhonemePercent => "音素占比",
    }
}

/// Standard musical subdivisions expressed as `(numerator, denominator, label)` of a whole note (`4 * ppq`).
pub const MUSICAL_SUBDIVISIONS: &[(i64, i64, &str)] = &[
    (1, 64, "1/64"),
    (1, 48, "1/32T"),
    (1, 32, "1/32"),
    (1, 24, "1/16T"),
    (1, 16, "1/16"),
    (3, 32, "1/16."),
    (1, 12, "1/8T"),
    (1, 8, "1/8"),
    (3, 16, "1/8."),
    (1, 6, "1/4T"),
    (1, 4, "1/4"),
    (3, 8, "1/4."),
    (1, 2, "1/2"),
    (3, 4, "1/2."),
    (1, 1, "1 小节 (4/4)"),
    (2, 1, "2 小节"),
];

pub fn subdivision_ticks(ppq: i64, num: i64, den: i64) -> Option<i64> {
    let whole = ppq.checked_mul(4)?;
    let prod = whole.checked_mul(num)?;
    (den > 0 && prod % den == 0 && prod / den > 0).then_some(prod / den)
}

pub fn format_musical_ticks(ticks: i64, ppq: i64, allow_off: bool) -> String {
    if allow_off && ticks <= 0 {
        return "关 (自由)".into();
    }
    let ppq = ppq.max(1);
    if let Some(whole) = ppq.checked_mul(4) {
        for &(num, den, name) in MUSICAL_SUBDIVISIONS {
            if let Some(prod) = whole.checked_mul(num) {
                if prod % den == 0 && prod / den == ticks {
                    return name.into();
                }
            }
        }
    }
    if ticks % ppq == 0 {
        format!("{} 拍", ticks / ppq)
    } else {
        format!("{ticks} tick")
    }
}

pub fn format_pan_percent(percent: f64) -> String {
    let rounded = (percent * 10.0).round() / 10.0;
    if rounded.abs() < 0.05 {
        "居中".into()
    } else if rounded < 0.0 {
        format!("L{:.0}%", rounded.abs())
    } else {
        format!("R{:.0}%", rounded)
    }
}

pub fn format_gain_db(db: f64) -> String {
    if db <= -95.0 {
        "-∞ dB".into()
    } else {
        format!("{db:+.1} dB")
    }
}

pub fn format_numeric_value(ed: &Editor, id: Num, value: f64) -> String {
    let spec = controls::numeric(id);
    let ppq = ed.project().ppq.max(1);
    match id {
        Num::SnapGridTicks => format_musical_ticks(value.round() as i64, ppq, true),
        Num::DefaultNoteTicks => format_musical_ticks(value.round() as i64, ppq, false),
        Num::NotePitchSemitone => {
            let midi = (value.round() as i32).clamp(0, 127);
            format!("{} ({midi})", note_name(midi))
        }
        Num::NoteStartTick => {
            let tick = (value.round() as i64).max(0);
            let pos = ed.meter().position_at(tick);
            format!("{}:{:02}:{:03}", pos.bar, pos.beat, pos.tick)
        }
        Num::NoteDurationTicks => {
            let ticks = (value.round() as i64).max(1);
            let musical = format_musical_ticks(ticks, ppq, false);
            if musical.ends_with("tick") { musical } else { format!("{musical} ({ticks}t)") }
        }
        Num::TrackPan | Num::MasterPan => format_pan_percent(value),
        Num::TrackGainDb | Num::MasterGainDb => format_gain_db(value),
        Num::TempoBpm => format!("{value:.1} BPM"),
        _ => format!("{:.*} {}", spec.decimals, value, spec.unit),
    }
}

#[derive(Default)]
struct NumericScrubState {
    active: Option<ActiveScrub>,
}
impl Global for NumericScrubState {}

pub(crate) fn has_numeric_scrub(editor: gpui::EntityId, cx: &App) -> bool {
    cx.try_global::<NumericScrubState>()
        .and_then(|s| s.active.as_ref())
        .is_some_and(|s| s.editor == editor)
}

pub(crate) fn cancel_numeric_scrub(editor: gpui::EntityId, cx: &mut App) {
    if has_numeric_scrub(editor, cx) {
        cx.global_mut::<NumericScrubState>().active = None;
    }
}

#[derive(Clone)]
struct ActiveScrub {
    editor: gpui::EntityId,
    key: SharedString,
    id: Num,
    arg: Arg,
    start_x: f64,
    start_y: f64,
    initial_value: f64,
    current_value: f64,
    moved: bool,
}

/// Scrubbable, preset-clickable, localized DAW numeric pill backed by `NumericTarget`.
pub fn numeric_pill(
    ed: &Editor,
    id: Num,
    arg: Arg,
    key: impl Into<SharedString>,
    compact: bool,
    cx: &mut Context<Editor>,
) -> Stateful<Div> {
    let key: SharedString = key.into();
    let spec = controls::numeric(id);
    assert_eq!(spec.arg, arg.kind(), "numeric argument kind mismatch");
    let value = ed.numeric_value(id, arg.clone());
    let th = ed.theme;
    let zh_label = numeric_label_zh(id);
    let formatted = format_numeric_value(ed, id, value);
    let display_text = if compact {
        format!("{zh_label} {formatted}")
    } else {
        formatted.clone()
    };

    let scrubbing = cx.try_global::<NumericScrubState>()
        .and_then(|s| s.active.as_ref())
        .is_some_and(|s| s.key == key);

    let (dec_arg, inc_arg, a11y_inc, a11y_dec) = (arg.clone(), arg.clone(), arg.clone(), arg.clone());
    let dec_id = SharedString::from(format!("{key}-minus"));
    let inc_id = SharedString::from(format!("{key}-plus"));
    let scrub_key = key.clone();
    let scrub_arg = arg.clone();
    let entity = cx.entity();

    let stepper = |btn_id: SharedString, direction: f64, symbol: &'static str, step_arg: Arg| {
        div()
            .id(btn_id)
            .px(px(6.0))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(th.text_secondary)
            .hover(move |s| s.bg(th.control_pressed).text_color(th.text))
            .child(symbol)
            .on_click(cx.listener(move |target, ev: &ClickEvent, _, cx| {
                let cur = target.numeric_value(id, step_arg.clone());
                if let Some(next) = spec.stepped(cur, direction, ev.modifiers().shift) {
                    target.set_numeric(id, step_arg.clone(), next, NumericPhase::Preview, cx);
                    target.set_numeric(id, step_arg.clone(), next, NumericPhase::Commit, cx);
                }
                cx.stop_propagation();
            }))
    };

    let center = canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |bounds, hitbox, window, cx| {
            if cx.try_global::<NumericScrubState>().is_none() {
                cx.set_global(NumericScrubState::default());
            }
            let is_grid = matches!(id, Num::SnapGridTicks | Num::DefaultNoteTicks);
            if hitbox.is_hovered(window) {
                window.set_cursor_style(
                    if is_grid { CursorStyle::Arrow } else { CursorStyle::ResizeUpDown },
                    &hitbox,
                );
            }
            let (e_down, h_down, k_down, a_down) = (entity.clone(), hitbox.clone(), scrub_key.clone(), scrub_arg.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !h_down.is_hovered(window) {
                    return;
                }
                if ev.click_count >= 2 {
                    cx.global_mut::<NumericScrubState>().active = None;
                    e_down.update(cx, |ed, cx| {
                        let current = ed.numeric_value(id, a_down.clone());
                        ed.prompt_numeric_value(id, a_down.clone(), current, window, cx);
                    });
                    cx.stop_propagation();
                    return;
                }
                let start_val = e_down.read(cx).numeric_value(id, a_down.clone());
                cx.global_mut::<NumericScrubState>().active = Some(ActiveScrub {
                    editor: e_down.entity_id(),
                    key: k_down.clone(),
                    id,
                    arg: a_down.clone(),
                    start_x: ev.position.x.as_f32() as f64,
                    start_y: ev.position.y.as_f32() as f64,
                    initial_value: start_val,
                    current_value: start_val,
                    moved: false,
                });
                window.capture_pointer(h_down.id);
                cx.stop_propagation();
            });

            let (e_move, k_move) = (entity.clone(), scrub_key.clone());
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                let update = {
                    let Some(active) = cx.try_global::<NumericScrubState>().and_then(|s| s.active.as_ref()) else { return };
                    if active.editor != e_move.entity_id() || active.key != k_move {
                        return;
                    }
                    let dx = ev.position.x.as_f32() as f64 - active.start_x;
                    let dy = active.start_y - ev.position.y.as_f32() as f64;
                    let delta_px = if dx.abs() > dy.abs() { dx } else { dy };
                    if !active.moved && delta_px.abs() < 3.0 {
                        return;
                    }
                    let step = if ev.modifiers.shift { spec.fine_step } else { spec.step };
                    let steps = (delta_px / 4.0).round();
                    let next = (active.initial_value + steps * step).clamp(spec.min, spec.max);
                    Some((active.arg.clone(), next))
                };
                if let Some((arg_val, next)) = update {
                    if let Some(active) = cx.global_mut::<NumericScrubState>().active.as_mut() {
                        active.moved = true;
                        active.current_value = next;
                    }
                    e_move.update(cx, |ed, cx| {
                        ed.set_numeric(id, arg_val, next, NumericPhase::Preview, cx);
                        cx.notify();
                    });
                }
            });

            let (e_up, k_up) = (entity.clone(), scrub_key.clone());
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left {
                    return;
                }
                let matches = cx.try_global::<NumericScrubState>()
                    .and_then(|s| s.active.as_ref())
                    .is_some_and(|s| s.editor == e_up.entity_id() && s.key == k_up);
                if !matches {
                    return;
                }
                let finished = cx.global_mut::<NumericScrubState>().active.take();
                window.release_pointer();
                if let Some(scrub) = finished {
                    e_up.update(cx, |ed, cx| {
                        if scrub.moved {
                            ed.set_numeric(scrub.id, scrub.arg, scrub.current_value, NumericPhase::Commit, cx);
                        } else if matches!(scrub.id, Num::SnapGridTicks | Num::DefaultNoteTicks) {
                            show_grid_preset_menu(ed, scrub.id, bounds.bottom_left(), window, cx);
                        }
                        cx.notify();
                    });
                }
            });
        },
    )
    .size_full();

    let pill = div()
        .id(key.clone())
        .role(Role::SpinButton)
        .aria_label(zh_label)
        .aria_numeric_value(value)
        .aria_numeric_value_step(spec.step)
        .aria_min_numeric_value(spec.min)
        .aria_max_numeric_value(spec.max)
        .aria_value(formatted)
        .on_a11y_action(
            AccessibleAction::Increment,
            crate::view::a11y_listener(cx, move |ed, _, _, cx| {
                if let Some(v) = spec.stepped(ed.numeric_value(id, a11y_inc.clone()), 1.0, false) {
                    ed.set_numeric(id, a11y_inc.clone(), v, NumericPhase::Preview, cx);
                    ed.set_numeric(id, a11y_inc.clone(), v, NumericPhase::Commit, cx);
                }
            }),
        )
        .on_a11y_action(
            AccessibleAction::Decrement,
            crate::view::a11y_listener(cx, move |ed, _, _, cx| {
                if let Some(v) = spec.stepped(ed.numeric_value(id, a11y_dec.clone()), -1.0, false) {
                    ed.set_numeric(id, a11y_dec.clone(), v, NumericPhase::Preview, cx);
                    ed.set_numeric(id, a11y_dec.clone(), v, NumericPhase::Commit, cx);
                }
            }),
        );

    let inner_box = div()
        .h(px(if compact { BAR_BTN_H } else { PANEL_BTN_H }))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .bg(th.control)
        .border_1()
        .border_color(if scrubbing { th.accent } else { th.control })
        .overflow_hidden()
        .child(stepper(dec_id, -1.0, "−", dec_arg))
        .child(
            div()
                .relative()
                .h_full()
                .px(px(6.0))
                .min_w(px(if compact { 76.0 } else { 96.0 }))
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .border_l_1()
                .border_r_1()
                .border_color(th.shade)
                .text_xs()
                .text_color(th.text)
                .child(display_text)
                .child(div().absolute().inset_0().child(center)),
        )
        .child(stepper(inc_id, 1.0, "+", inc_arg));

    if compact {
        pill.flex().items_center().child(inner_box)
    } else {
        pill.flex()
            .items_center()
            .gap(px(8.0))
            .child(div().w(px(64.0)).flex_none().text_xs().text_color(th.text_secondary).child(zh_label))
            .child(div().flex_1().child(inner_box))
    }
}

pub fn show_grid_preset_menu(
    ed: &mut Editor,
    id: Num,
    anchor: gpui::Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<Editor>,
) {
    let ppq = ed.project().ppq.max(1);
    let current = ed.numeric_value(id, Arg::None).round() as i64;
    let is_snap = id == Num::SnapGridTicks;
    let mut items = Vec::new();
    if is_snap {
        items.push(
            MenuItem::action(
                "关 · 不吸附网格",
                crate::editor::MenuAction::SetGridTicks { snap: true, ticks: 0 },
            )
            .checked(current == 0)
            .with_hint("Alt 临时不吸附"),
        );
        items.push(MenuItem::separator());
    }
    for &(num, den, name) in &[
        (1, 32, "1/32 音符"),
        (1, 24, "1/16 三连音 (1/16T)"),
        (1, 16, "1/16 音符"),
        (1, 12, "1/8 三连音 (1/8T)"),
        (1, 8, "1/8 音符"),
        (3, 16, "附点 1/8 音符"),
        (1, 6, "1/4 三连音 (1/4T)"),
        (1, 4, "1/4 音符 (一拍)"),
        (1, 2, "1/2 音符 (两拍)"),
        (1, 1, "全音符 (4 拍)"),
    ] {
        if let Some(ticks) = subdivision_ticks(ppq, num, den) {
            items.push(
                MenuItem::action(
                    name,
                    crate::editor::MenuAction::SetGridTicks { snap: is_snap, ticks },
                )
                .checked(current == ticks)
                .with_hint(format!("{ticks}t")),
            );
        }
    }
    ed.show_menu(anchor, items, window, cx);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitterKind {
    InspectorWidth,
    RightDockWidth,
    ArrangeHeight,
    LaneHeight,
}

#[derive(Default)]
struct SplitterState {
    drag: Option<ActiveSplitter>,
}
impl Global for SplitterState {}

struct ActiveSplitter {
    editor: gpui::EntityId,
    kind: SplitterKind,
    start_pos: f32,
    initial_size: f32,
}

/// Interactive draggable splitter bar between workspace panes.
pub fn splitter_bar(editor: Entity<Editor>, kind: SplitterKind, th: Theme) -> impl IntoElement {
    let vertical = matches!(kind, SplitterKind::InspectorWidth | SplitterKind::RightDockWidth);
    let id = match kind {
        SplitterKind::InspectorWidth => "splitter-inspector",
        SplitterKind::RightDockWidth => "splitter-right-dock",
        SplitterKind::ArrangeHeight => "splitter-arrange",
        SplitterKind::LaneHeight => "splitter-lane",
    };
    div()
        .id(id)
        .flex_none()
        .when(vertical, |d| d.w(px(5.0)).h_full())
        .when(!vertical, |d| d.h(px(5.0)).w_full())
        .bg(th.surface)
        .child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds, hitbox, window, cx| {
                    if cx.try_global::<SplitterState>().is_none() {
                        cx.set_global(SplitterState::default());
                    }
                    let dragging = cx.global::<SplitterState>()
                        .drag
                        .as_ref()
                        .is_some_and(|d| d.editor == editor.entity_id() && d.kind == kind);
                    let hovered = hitbox.is_hovered(window);
                    if hovered || dragging {
                        window.set_cursor_style(
                            if vertical { CursorStyle::ResizeLeftRight } else { CursorStyle::ResizeUpDown },
                            &hitbox,
                        );
                    }
                    let o = bounds.origin;
                    let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
                    let line_color = if dragging {
                        th.accent
                    } else if hovered {
                        with_alpha(th.accent, 0.7)
                    } else {
                        th.shade
                    };
                    if vertical {
                        window.paint_quad(fill(b((w - 1.0) / 2.0, 0.0, if dragging { 2.0 } else { 1.0 }, h, o), line_color));
                    } else {
                        window.paint_quad(fill(b(0.0, (h - 1.0) / 2.0, w, if dragging { 2.0 } else { 1.0 }, o), line_color));
                    }

                    let (e_down, h_down) = (editor.clone(), hitbox.clone());
                    window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !h_down.is_hovered(window) {
                            return;
                        }
                        if ev.click_count >= 2 {
                            e_down.update(cx, |ed, cx| {
                                let def = LayoutPrefs::default();
                                match kind {
                                    SplitterKind::InspectorWidth => ed.layout.inspector_width = def.inspector_width,
                                    SplitterKind::RightDockWidth => ed.layout.right_dock_width = def.right_dock_width,
                                    SplitterKind::ArrangeHeight => ed.layout.arrange_height = def.arrange_height,
                                    SplitterKind::LaneHeight => ed.layout.lane_height = def.lane_height,
                                }
                                ed.layout.store(&mut ed.prefs);
                                ed.prefs_dirty = true;
                                cx.notify();
                            });
                            cx.stop_propagation();
                            return;
                        }
                        let initial_size = {
                            let ed = e_down.read(cx);
                            match kind {
                                SplitterKind::InspectorWidth => ed.layout.inspector_width,
                                SplitterKind::RightDockWidth => ed.layout.right_dock_width,
                                SplitterKind::ArrangeHeight => ed.layout.arrange_height,
                                SplitterKind::LaneHeight => ed.layout.lane_height,
                            }
                        };
                        cx.global_mut::<SplitterState>().drag = Some(ActiveSplitter {
                            editor: e_down.entity_id(),
                            kind,
                            start_pos: if vertical { ev.position.x.as_f32() } else { ev.position.y.as_f32() },
                            initial_size,
                        });
                        window.capture_pointer(h_down.id);
                        cx.stop_propagation();
                    });

                    let e_move = editor.clone();
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        let next = {
                            let Some(drag) = cx.try_global::<SplitterState>().and_then(|s| s.drag.as_ref()) else { return };
                            if drag.editor != e_move.entity_id() || drag.kind != kind {
                                return;
                            }
                            let cur = if vertical { ev.position.x.as_f32() } else { ev.position.y.as_f32() };
                            let delta = cur - drag.start_pos;
                            match kind {
                                SplitterKind::InspectorWidth => drag.initial_size + delta,
                                SplitterKind::RightDockWidth => drag.initial_size - delta,
                                SplitterKind::ArrangeHeight => drag.initial_size + delta,
                                SplitterKind::LaneHeight => drag.initial_size - delta,
                            }
                        };
                        e_move.update(cx, |ed, cx| {
                            match kind {
                                SplitterKind::InspectorWidth => ed.layout.inspector_width = next,
                                SplitterKind::RightDockWidth => ed.layout.right_dock_width = next,
                                SplitterKind::ArrangeHeight => ed.layout.arrange_height = next,
                                SplitterKind::LaneHeight => ed.layout.lane_height = next,
                            }
                            ed.layout = ed.layout.clamped();
                            cx.notify();
                        });
                    });

                    let e_up = editor.clone();
                    window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left {
                            return;
                        }
                        let matched = cx.try_global::<SplitterState>()
                            .and_then(|s| s.drag.as_ref())
                            .is_some_and(|d| d.editor == e_up.entity_id() && d.kind == kind);
                        if matched {
                            cx.global_mut::<SplitterState>().drag = None;
                            window.release_pointer();
                            e_up.update(cx, |ed, cx| {
                                ed.layout.store(&mut ed.prefs);
                                ed.prefs_dirty = true;
                                cx.notify();
                            });
                        }
                    });
                },
            )
            .size_full(),
        )
}

#[derive(Default)]
struct ScrollThumbState {
    drag: Option<(gpui::EntityId, bool, f64, f64)>,
}
impl Global for ScrollThumbState {}

/// Horizontal timeline scrollbar + zoom buttons for Arrangement or Piano Roll.
pub fn horizontal_timeline_bar(editor: Entity<Editor>, is_arrange: bool, th: Theme, cx: &mut Context<Editor>) -> impl IntoElement {
    let space = if is_arrange { crate::clock::Space::Arrange } else { crate::clock::Space::Editor };
    let e_zoom_out = editor.clone();
    let e_zoom_in = editor.clone();
    let id_prefix = if is_arrange { "arrange-scroll" } else { "roll-scroll" };
    div()
        .id(id_prefix)
        .h(px(16.0))
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .bg(th.surface)
        .border_t_1()
        .border_color(th.shade)
        .child(
            div()
                .flex_1()
                .h_full()
                .child(
                    canvas(
                        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                        move |bounds, hitbox, window, cx| {
                            if cx.try_global::<ScrollThumbState>().is_none() {
                                cx.set_global(ScrollThumbState::default());
                            }
                            let (start_tick, visible_ticks, total_ticks) = {
                                let ed = editor.read(cx);
                                let vp = if is_arrange { ed.arrange } else { ed.session.viewport };
                                let vis = (vp.width / vp.px_per_tick.max(1e-6)).max(1.0);
                                let max_end = ed.arrange_cache.end_tick().max(ed.project().ppq.saturating_mul(16)) as f64;
                                let total = (max_end + vis * 0.5).max(vp.start_tick + vis).max(1.0);
                                (vp.start_tick.max(0.0), vis, total)
                            };
                            let o = bounds.origin;
                            let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
                            window.paint_quad(fill(bounds, th.lcd));
                            let ratio = (visible_ticks / total_ticks).clamp(0.04, 1.0);
                            let thumb_w = (w * ratio).max(24.0).min(w);
                            let travel = (w - thumb_w).max(0.0);
                            let max_scroll = (total_ticks - visible_ticks).max(1.0);
                            let thumb_x = if travel > 0.0 { (start_tick / max_scroll).clamp(0.0, 1.0) * travel } else { 0.0 };
                            let dragging = cx.global::<ScrollThumbState>()
                                .drag
                                .is_some_and(|(eid, arr, _, _)| eid == editor.entity_id() && arr == is_arrange);
                            window.paint_quad(
                                gpui::quad(
                                    b(thumb_x, 2.0, thumb_w, (h - 4.0).max(4.0), o),
                                    px(4.0),
                                    if dragging || hitbox.is_hovered(window) { th.control_pressed } else { th.control },
                                    px(1.0),
                                    if dragging { th.accent } else { th.separator },
                                    gpui::BorderStyle::Solid,
                                ),
                            );

                            let (e_down, h_down) = (editor.clone(), hitbox.clone());
                            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !h_down.is_hovered(window) {
                                    return;
                                }
                                let local_x = (ev.position.x - bounds.origin.x).as_f32() as f64;
                                let start = if (thumb_x..=thumb_x + thumb_w).contains(&local_x) {
                                    start_tick
                                } else {
                                    let target_frac = ((local_x - thumb_w * 0.5) / travel.max(1.0)).clamp(0.0, 1.0);
                                    let jump_tick = target_frac * max_scroll;
                                    e_down.update(cx, |ed, cx| {
                                        if is_arrange { ed.arrange.start_tick = jump_tick; } else { ed.session.viewport.start_tick = jump_tick; }
                                        ed.notify_viewport(space, cx);
                                    });
                                    jump_tick
                                };
                                cx.global_mut::<ScrollThumbState>().drag =
                                    Some((e_down.entity_id(), is_arrange, ev.position.x.as_f32() as f64, start));
                                window.capture_pointer(h_down.id);
                                cx.stop_propagation();
                            });

                            let e_move = editor.clone();
                            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                                if phase != DispatchPhase::Bubble {
                                    return;
                                }
                                let Some((eid, arr, ox, start)) = cx.try_global::<ScrollThumbState>().and_then(|s| s.drag) else { return };
                                if eid != e_move.entity_id() || arr != is_arrange || travel <= 0.0 {
                                    return;
                                }
                                let dx = ev.position.x.as_f32() as f64 - ox;
                                let next_tick = (start + dx / travel * max_scroll).max(0.0);
                                e_move.update(cx, |ed, cx| {
                                    if is_arrange { ed.arrange.start_tick = next_tick; } else { ed.session.viewport.start_tick = next_tick; }
                                    ed.notify_viewport(space, cx);
                                });
                            });

                            let e_up = editor.clone();
                            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                                if phase == DispatchPhase::Bubble && ev.button == MouseButton::Left {
                                    if cx.try_global::<ScrollThumbState>().and_then(|s| s.drag).is_some_and(|(eid, arr, _, _)| eid == e_up.entity_id() && arr == is_arrange) {
                                        cx.global_mut::<ScrollThumbState>().drag = None;
                                        window.release_pointer();
                                    }
                                }
                            });
                        },
                    )
                    .size_full(),
                ),
        )
        .child(
            div()
                .id(SharedString::from(format!("{id_prefix}-zoom-out")))
                .role(Role::Button)
                .aria_label("水平缩小")
                .w(px(20.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l_1()
                .border_color(th.shade)
                .text_xs()
                .text_color(th.text_secondary)
                .hover(move |s| s.bg(th.control_pressed).text_color(th.text))
                .child("−")
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if is_arrange {
                        let c = ed.arrange.width * 0.5;
                        ed.arrange.zoom_x_at(c, 0.8);
                    } else {
                        let c = ed.session.viewport.width * 0.5;
                        ed.session.viewport.zoom_x_at(c, 0.8);
                    }
                    let _ = &e_zoom_out;
                    ed.notify_viewport(space, cx);
                })),
        )
        .child(
            div()
                .id(SharedString::from(format!("{id_prefix}-zoom-in")))
                .role(Role::Button)
                .aria_label("水平放大")
                .w(px(20.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l_1()
                .border_color(th.shade)
                .text_xs()
                .text_color(th.text_secondary)
                .hover(move |s| s.bg(th.control_pressed).text_color(th.text))
                .child("+")
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if is_arrange {
                        let c = ed.arrange.width * 0.5;
                        ed.arrange.zoom_x_at(c, 1.25);
                    } else {
                        let c = ed.session.viewport.width * 0.5;
                        ed.session.viewport.zoom_x_at(c, 1.25);
                    }
                    let _ = &e_zoom_in;
                    ed.notify_viewport(space, cx);
                })),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn musical_ticks_format_standard_and_high_ppq_scores_cleanly() {
        for ppq in [480_i64, 960, 705_600_000] {
            assert_eq!(format_musical_ticks(0, ppq, true), "关 (自由)");
            assert_eq!(format_musical_ticks(ppq / 4, ppq, true), "1/16");
            assert_eq!(format_musical_ticks(ppq / 2, ppq, false), "1/8");
            assert_eq!(format_musical_ticks(ppq, ppq, false), "1/4");
            assert_eq!(format_musical_ticks(ppq * 4, ppq, false), "1 小节 (4/4)");
            assert_eq!(format_musical_ticks(ppq / 3, ppq, false), "1/8T");
        }
        assert_eq!(format_musical_ticks(137, 480, false), "137 tick");
    }

    #[test]
    fn pan_and_gain_format_musical_readouts() {
        assert_eq!(format_pan_percent(0.0), "居中");
        assert_eq!(format_pan_percent(-35.0), "L35%");
        assert_eq!(format_pan_percent(60.0), "R60%");
        assert_eq!(format_gain_db(-96.0), "-∞ dB");
        assert_eq!(format_gain_db(0.0), "+0.0 dB");
        assert_eq!(format_gain_db(-6.0), "-6.0 dB");
    }

    #[test]
    fn layout_prefs_clamp_and_roundtrip_without_touching_unrelated_prefs() {
        let mut prefs = crate::files::Prefs::default();
        prefs.extra.insert("keep_me".into(), serde_json::json!(42));
        let custom = LayoutPrefs {
            inspector_width: 9999.0,
            right_dock_width: 100.0,
            show_right_dock: false,
            right_dock_tab: RightDockTab::Dictionary,
            arrange_height: 10.0,
            lane_height: 240.0,
            inspector_tab: InspectorTab::Synthesis,
            show_tempo_lane: true,
            show_phoneme_lane: false,
        };
        custom.store(&mut prefs);
        let loaded = LayoutPrefs::from_prefs(&prefs);
        assert_eq!(loaded.inspector_width, 420.0);
        assert_eq!(loaded.right_dock_width, 244.0);
        assert!(!loaded.show_right_dock);
        assert_eq!(loaded.right_dock_tab, RightDockTab::Dictionary);
        assert_eq!(loaded.arrange_height, 92.0);
        assert_eq!(loaded.lane_height, 240.0);
        assert_eq!(loaded.inspector_tab, InspectorTab::Synthesis);
        assert!(loaded.show_tempo_lane);
        assert!(!loaded.show_phoneme_lane);
        assert_eq!(prefs.extra["keep_me"], serde_json::json!(42));
    }

    #[test]
    fn route_names_avoid_milestone_jargon() {
        assert_eq!(route_display_name(Route::VerdantVoice), "verdant-voice");
        assert_eq!(route_display_name(Route::C0), "原始声库");
        assert_eq!(route_short_name(Route::VerdantVoice), "verdant-voice");
        assert_eq!(route_short_name(Route::C0), "原始声库");
    }
}

#[cfg(test)]
mod width_class_tests {
    use super::*;

    #[test]
    fn width_classes_cover_phone_to_desktop() {
        assert_eq!(WidthClass::of(390.0), WidthClass::Phone);
        assert_eq!(WidthClass::of(820.0), WidthClass::Compact);
        assert_eq!(WidthClass::of(1180.0), WidthClass::Medium);
        assert_eq!(WidthClass::of(1920.0), WidthClass::Wide);
        assert!(!WidthClass::Compact.docks_panels() && WidthClass::Medium.docks_panels());
        assert!(bar_button_size(true) > bar_button_size(false));
    }

    #[test]
    fn layouts_saved_with_the_retired_note_tab_still_load() {
        let layout: LayoutPrefs = serde_json::from_value(serde_json::json!({"right_dock_tab": "note_phoneme", "inspector_width": 300.0})).unwrap();
        assert_eq!(layout.right_dock_tab, RightDockTab::VoiceTimbre);
        assert_eq!(layout.inspector_width, 300.0);
    }
}
