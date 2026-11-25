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
/// libadwaita metrics: header bar buttons 32 px, toolbar controls 30 px,
/// fields and buttons in panes 34 px.
pub const TOP_BTN_H: f32 = 32.0;
pub const BAR_BTN_H: f32 = 30.0;
pub const PANEL_BTN_H: f32 = 34.0;
pub const HDR_BTN_H: f32 = 22.0;

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
            inspector_width: 300.0,
            right_dock_width: 340.0,
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
            inspector_width: if self.inspector_width.is_finite() { self.inspector_width.clamp(260.0, 440.0) } else { 300.0 },
            right_dock_width: if self.right_dock_width.is_finite() { self.right_dock_width.clamp(280.0, 480.0) } else { 340.0 },
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

/// libadwaita state layers. Every clickable surface shows rest, hover and
/// pressed distinctly, and a checked surface keeps its own darker ramp, so a
/// press never looks like a hover and a toggle never looks like a press.
///
/// | kind   | rest            | hover          | pressed        |
/// |--------|-----------------|----------------|----------------|
/// | raised | `control` (10%) | `control_pressed` (15%) | `checked` (30%) |
/// | flat   | clear           | text @ 7%      | text @ 16%     |
/// | checked| `checked` (30%) | +7% toward text| +14% toward text|
pub(crate) trait AdwStates: StatefulInteractiveElement + Styled + Sized {
    fn raised_states(self, th: Theme, checked: bool) -> Self {
        let (rest, hover, press) = states(th, checked, false);
        self.bg(rest).border_color(rest).hover(move |s| s.bg(hover).border_color(hover)).active(move |s| s.bg(press).border_color(press))
    }
    fn flat_states(self, th: Theme, checked: bool) -> Self {
        let (rest, hover, press) = states(th, checked, true);
        self.bg(rest).hover(move |s| s.bg(hover)).active(move |s| s.bg(press))
    }
    /// Only the pressed layer, for surfaces whose rest and hover fills were
    /// set where they were built (the generated command surfaces).
    fn pressed_state(self, th: Theme, checked: bool, flat: bool) -> Self {
        let (_, _, press) = states(th, checked, flat);
        self.active(move |s| s.bg(press).border_color(press))
    }
}
impl<T: StatefulInteractiveElement + Styled> AdwStates for T {}

/// The pressed fill for a surface hovered with `hover`: 10% further toward
/// the text colour, darker in light themes and lighter in dark ones.
pub(crate) fn pressed_fill(th: Theme, hover: gpui::Rgba) -> gpui::Rgba { crate::theme::mix(hover, th.text, 0.10) }

/// (rest, hover, pressed) fills for a surface.
pub(crate) fn states(th: Theme, checked: bool, flat: bool) -> (gpui::Rgba, gpui::Rgba, gpui::Rgba) {
    use crate::theme::{mix, with_alpha};
    if checked { (th.checked, mix(th.checked, th.text, 0.07), mix(th.checked, th.text, 0.14)) }
    else if flat { (with_alpha(th.text, 0.0), with_alpha(th.text, 0.07), with_alpha(th.text, 0.16)) }
    else { (th.control, th.control_pressed, th.checked) }
}

/// Ordinary button with unmistakable visual active state, uniform `BAR_BTN_H` height, and WCAG >= 4.5 contrast.
pub fn selectable_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    active: bool,
    th: Theme,
) -> crate::proto_controls::Action {
    let label = label.into();
    crate::proto_controls::action(id.into(), label.clone())
        .role(Role::Button)
        .aria_label(label.clone())
        .aria_selected(active)
        .h(px(BAR_BTN_H))
        .px(px(11.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .border_1()
        .raised_states(th, active)
        .text_sm()
        .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(th.text)
        .child(label)
}

/// Button with a leading Lucide SVG icon and uniform `BAR_BTN_H` height.
pub fn icon_selectable_button(
    id: impl Into<SharedString>,
    icon_name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    th: Theme,
) -> crate::proto_controls::Action {
    let label = label.into();
    let fg = th.text;
    crate::proto_controls::action(id.into(), label.clone())
        .role(Role::Button)
        .aria_label(label.clone())
        .aria_selected(active)
        .h(px(BAR_BTN_H))
        .px(px(11.0))
        .gap(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .border_1()
        .raised_states(th, active)
        .text_sm()
        .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
        .text_color(fg)
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
) -> crate::proto_controls::Action {
    let label = label.into();
    crate::proto_controls::action(id.into(), label.clone())
        .role(Role::ComboBox)
        .aria_label(aria_name.into())
        .h(px(PANEL_BTN_H))
        .px(px(12.0))
        .gap(px(8.0))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .border_1()
        .raised_states(th, false)
        .text_sm()
        .text_color(th.text)
        .when_some(leading_icon, |d, ic| {
            d.child(svg().path(icon(ic)).size(px(13.5)).flex_none().text_color(th.text_secondary))
        })
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(svg().path(icon("chevron-down")).size(px(13.0)).flex_none().text_color(th.text_secondary))
}

/// Attach exactly one command consumer to a surface that has no listener yet.
/// Only classified Boolean commands accept the source's selected proposal.
pub(crate) fn bind_command_action(
    ed: &Editor,
    cmd: Cmd,
    arg: Arg,
    action: crate::proto_controls::Action,
    active: bool,
    cx: &mut Context<Editor>,
) -> crate::proto_controls::Action {
    let basis = crate::proto_controls::editor_authority(ed);
    let authority = format!("{basis}:{cmd:?}:{arg:?}");
    let selection = ed.source_command_selection(cmd, &arg);
    let button = if let Some(accepted) = selection {
        action.on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, _, cx| {
            if crate::proto_controls::editor_authority(ed) != basis
                || ed.source_command_selection(cmd, &arg) != Some(accepted) { return; }
            ed.accept_source_command_selection(cmd, &arg, event.selected(), cx);
        }))
    } else {
        action.on_click(cx.listener(move |ed, _, window, cx| controls::dispatch(ed, cmd, arg.clone(), window, cx)))
    };
    button
        .authority(authority)
        .role(if selection.is_some() { Role::Switch } else { Role::Button })
        .aria_selected(selection.unwrap_or(active))
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
) -> crate::proto_controls::Action {
    let label = label.into();
    let th = ed.theme;
    let active = ed.source_command_selection(cmd, &arg).unwrap_or(active);
    let (rest, hover, _) = states(th, active, false);
    let action = controls::command_surface(cmd, &arg, label.clone(), key, th.text.into(), rest.into(), hover.into());
    bind_command_action(ed, cmd, arg, action, active, cx)
    .aria_label(label)
    .h(px(BAR_BTN_H))
    .py(px(0.0))
    .px(px(11.0))
    .rounded(px(6.0))
    .text_sm()
    .border_1()
    .border_color(rest)
    .pressed_state(th, active, false)
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
) -> crate::proto_controls::Action {
    let label = label.into();
    let th = ed.theme;
    let active = ed.source_command_selection(cmd, &arg).unwrap_or(active);
    let (rest, hover, _) = states(th, active, true);
    let action = controls::command_surface(cmd, &arg, label.clone(), key, th.text.into(), rest.into(), hover.into());
    bind_command_action(ed, cmd, arg, action, active, cx)
    .aria_label(label)
    .py(px(0.0))
    .rounded(px(6.0))
    .pressed_state(th, active, true)
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
) -> crate::proto_controls::Action {
    let label = label.into();
    let th = ed.theme;
    let fg = th.text;
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
) -> crate::proto_controls::Action {
    let label = label.into();
    let th = ed.theme;
    let fg = th.text;
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
) -> crate::proto_controls::Action {
    let th = ed.theme;
    let current = ed.current_select_option(id, &arg);
    let spec = controls::select(id);
    let authority = format!("{}:{id:?}:{arg:?}:{current}", crate::proto_controls::editor_authority(ed));
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
    .authority(authority)
    .role(Role::ComboBox)
    .aria_label(spec.label)
    .h(px(PANEL_BTN_H))
    .py(px(0.0))
    .px(px(12.0))
    .gap(px(8.0))
    .text_sm()
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

// Host bookkeeping records the target of the live audio/draft preview. Gesture
// phase, threshold, values, cancellation and stepping live in ContinuousControl.
#[derive(Default)]
struct NumericScrubState { active: Option<(gpui::EntityId, SharedString)> }
impl Global for NumericScrubState {}
pub(crate) fn has_numeric_scrub(editor: gpui::EntityId, cx: &App) -> bool {
    cx.try_global::<NumericScrubState>().and_then(|state| state.active.as_ref())
        .is_some_and(|(owner, _)| *owner == editor)
}
pub(crate) fn cancel_numeric_scrub(editor: gpui::EntityId, cx: &mut App) {
    if has_numeric_scrub(editor, cx) { cx.global_mut::<NumericScrubState>().active = None; }
}

/// An authored SpinButton with native Adwaita content and project-unit callbacks.
pub fn numeric_pill(ed: &Editor, id: Num, arg: Arg, key: impl Into<SharedString>,
    compact: bool, cx: &mut Context<Editor>) -> Stateful<Div> {
    use crate::proto_scalar::{EventKind, Sample, Spec, scalar};
    let key: SharedString = key.into();
    let spec = controls::numeric(id);
    assert_eq!(spec.arg, arg.kind(), "numeric argument kind mismatch");
    let value = ed.numeric_value(id, arg.clone());
    let th = ed.theme;
    let label = numeric_label_zh(id);
    let entity = cx.entity();
    let display_editor = entity.clone();
    let event_key = key.clone();
    let event_arg = arg.clone();
    let valid_arg = arg.clone();
    let mut config = Spec::slider(value, spec.min, spec.max, label);
    config.step = spec.step;
    config.fine_step = spec.fine_step;
    config.default_value = spec.default;
    config.role = "spinbutton";
    config.drag_mode = "steps";
    config.double_action = "edit";
    config.tap_action = if matches!(id, Num::SnapGridTicks | Num::DefaultNoteTicks) { "activate" } else { "none" };
    config.preview_echo = true;
    let scalar = scalar(entity, key.clone(), config,
        // Step-mode scrubbing consumes raw displacement inside the prototype;
        // no host axis choice or pixel-to-step policy precedes its decision.
        |_, _, initial| Sample { value: initial, fine: initial, jump: false },
        move |display, actions, _, cx| {
            let formatted = format_numeric_value(display_editor.read(cx), id, display.value);
            let text = if compact { format!("{label} {formatted}") } else { formatted };
            let stepper = |direction: f64, symbol: &'static str| {
                let actions = actions.clone();
                crate::proto_controls::action(symbol, format!("{label}{}", if direction < 0.0 { "减少" } else { "增加" }))
                    .w(px(if compact { 24.0 } else { 30.0 })).h_full().flex_none()
                    .flex().items_center().justify_center().border_l_1().border_color(th.shade)
                    .text_color(th.text).hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.control_pressed)))
                    .child(svg().path(icon(symbol)).size(px(13.0)).text_color(th.text))
                    .on_click(move |event, window, cx| {
                        actions.invoke("adjust", vec![direction.into(), event.modifiers().shift.into()], window, cx);
                        cx.stop_propagation();
                    })
            };
            div().h(px(if compact { BAR_BTN_H } else { PANEL_BTN_H })).w_full().flex().items_center()
                .rounded(px(6.0)).bg(th.control).border_1()
                .border_color(if display.active || display.focus_visible { th.accent } else { th.control })
                .overflow_hidden()
                .child(div().h_full().px(px(if compact { 8.0 } else { 10.0 }))
                    .min_w(px(if compact { 76.0 } else { 64.0 })).flex_1().flex().items_center()
                    .when(compact, |d| d.justify_center()).text_sm().text_color(th.text).child(text))
                .child(stepper(-1.0, "minus")).child(stepper(1.0, "plus")).into_any_element()
        },
        move |ed, event, window, cx| {
            if cx.try_global::<NumericScrubState>().is_none() { cx.set_global(NumericScrubState::default()); }
            match event.kind {
                EventKind::Start => cx.global_mut::<NumericScrubState>().active = Some((cx.entity_id(), event_key.clone())),
                EventKind::Preview => ed.set_numeric(id, event_arg.clone(), event.value, NumericPhase::Preview, cx),
                EventKind::Commit => {
                    cancel_numeric_scrub(cx.entity_id(), cx);
                    ed.set_numeric(id, event_arg.clone(), event.value, NumericPhase::Commit, cx);
                }
                EventKind::Cancel | EventKind::Activate | EventKind::Edit => {
                    cancel_numeric_scrub(cx.entity_id(), cx);
                    if ed.numeric_preview.as_ref().is_some_and(|(active_id, active_arg, _)| *active_id == id && *active_arg == event_arg) {
                        ed.numeric_preview = None; ed.sync_mixer();
                    }
                    if event.kind == EventKind::Activate { show_grid_preset_menu(ed, id, window.mouse_position(), window, cx); }
                    if event.kind == EventKind::Edit { ed.prompt_numeric_value(id, event_arg.clone(), event.value, window, cx); }
                }
            }
            cx.notify();
        }).valid_when(move |ed| match &valid_arg {
            Arg::Track(track) => ed.project().track(track).is_some(),
            Arg::Note(note) | Arg::NotePhone(note, _) => ed.index.contains(note),
            _ => true,
        });
    let pill = div().id(key).h(px(if compact { BAR_BTN_H } else { PANEL_BTN_H }));
    if compact { pill.child(scalar) }
    else { pill.flex().items_center().gap(px(12.0))
        .child(div().flex_1().min_w(px(56.0)).text_sm().text_color(th.text).child(label))
        .child(div().flex_shrink_0().w(px(150.0)).h_full().child(scalar)) }
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

#[derive(IntoElement)]
struct Splitter { editor: Entity<Editor>, kind: SplitterKind, theme: Theme }

/// Layout policy (size constraints/persistence) remains in the host; input,
/// focus, accessibility and rollback use the same authored scalar as the mixer.
pub fn splitter_bar(editor: Entity<Editor>, kind: SplitterKind, th: Theme) -> impl IntoElement {
    Splitter { editor, kind, theme: th }
}
impl gpui::RenderOnce for Splitter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        use crate::proto_scalar::{EventKind, Sample, Spec, scalar};
        let kind = self.kind;
        let th = self.theme;
        let ed = self.editor.read(cx);
        let defaults = LayoutPrefs::default();
        let (id, label, value, default, min, max, vertical, sign) = match kind {
            SplitterKind::InspectorWidth => ("splitter-inspector", "左侧面板宽度", ed.layout.inspector_width, defaults.inspector_width, 260.0, 440.0, true, 1.0),
            SplitterKind::RightDockWidth => ("splitter-right-dock", "右侧面板宽度", ed.layout.right_dock_width, defaults.right_dock_width, 280.0, 480.0, true, -1.0),
            SplitterKind::ArrangeHeight => ("splitter-arrange", "编曲区高度", ed.layout.arrange_height, defaults.arrange_height, 92.0, 460.0, false, 1.0),
            SplitterKind::LaneHeight => ("splitter-lane", "参数轨高度", ed.layout.lane_height, defaults.lane_height, 112.0, 420.0, false, -1.0),
        };
        let mut spec = Spec::slider(value as f64, min, max, label);
        spec.role = "separator";
        spec.step = 8.0; spec.fine_step = 1.0; spec.default_value = default as f64;
        spec.preview_echo = true;
        let control = scalar(self.editor, id, spec,
            move |sample, _, initial| {
                let delta = if vertical { sample.total_delta.x } else { sample.total_delta.y } * sign;
                // LayoutPrefs stores f32. Propose that same representation so
                // its preview echo remains exact across a native repaint.
                Sample {
                    value: f64::from((initial + delta) as f32),
                    fine: f64::from((initial + delta * 0.1) as f32),
                    jump: false,
                }
            },
            move |display, _, _, _| canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds, hitbox, window, _| {
                    let hovered = hitbox.is_hovered(window);
                    if hovered || display.active { window.set_cursor_style(
                        if vertical { CursorStyle::ResizeLeftRight } else { CursorStyle::ResizeUpDown }, &hitbox); }
                    let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
                    let color = if display.active || display.focus_visible { th.accent }
                        else if hovered { with_alpha(th.accent, 0.7) } else { th.shade };
                    let width = if display.active || display.focus_visible { 2.0 } else { 1.0 };
                    window.paint_quad(fill(if vertical { b((w-width)/2.0, 0.0, width, h, bounds.origin) }
                        else { b(0.0, (h-width)/2.0, w, width, bounds.origin) }, color));
                }).size_full().into_any_element(),
            move |ed, event, _, cx| {
                if matches!(event.kind, EventKind::Preview | EventKind::Commit | EventKind::Cancel) {
                    let value = event.value as f32;
                    match kind {
                        SplitterKind::InspectorWidth => ed.layout.inspector_width = value,
                        SplitterKind::RightDockWidth => ed.layout.right_dock_width = value,
                        SplitterKind::ArrangeHeight => ed.layout.arrange_height = value,
                        SplitterKind::LaneHeight => ed.layout.lane_height = value,
                    }
                    ed.layout = ed.layout.clamped();
                    if event.kind == EventKind::Commit { ed.layout.store(&mut ed.prefs); ed.prefs_dirty = true; }
                    cx.notify();
                }
            });
        div().id(id).flex_none().when(vertical, |d| d.w(px(5.0)).h_full())
            .when(!vertical, |d| d.h(px(5.0)).w_full()).bg(th.surface).child(control)
    }
}

#[cfg(test)]
mod splitter_precision_tests {
    use super::*;
    use gpui::{AppContext, Render, Subscription, TestAppContext, point, size};

    struct Harness { editor: Entity<Editor>, kind: SplitterKind, _changed: Subscription }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = self.editor.read(cx).theme;
            div().size_full().child(div().w(px(200.0)).h(px(100.0))
                .child(splitter_bar(self.editor.clone(), self.kind, theme)))
        }
    }
    fn value(layout: LayoutPrefs, kind: SplitterKind) -> f32 {
        match kind {
            SplitterKind::InspectorWidth => layout.inspector_width,
            SplitterKind::RightDockWidth => layout.right_dock_width,
            SplitterKind::ArrangeHeight => layout.arrange_height,
            SplitterKind::LaneHeight => layout.lane_height,
        }
    }

    #[gpui::test]
    fn native_splitters_keep_f32_previews_through_repaint_and_release(cx: &mut TestAppContext) {
        for kind in [SplitterKind::InspectorWidth, SplitterKind::RightDockWidth,
            SplitterKind::ArrangeHeight, SplitterKind::LaneHeight] {
            for fine in [false, true] {
                let window = cx.open_window(size(px(400.0), px(240.0)), |window, cx| {
                    let editor = cx.new(|cx| Editor::for_native_test(verdant_domain::Project::new_empty(), window, cx));
                    let changed = cx.observe(&editor, |_, _, cx| cx.notify());
                    Harness { editor, kind, _changed: changed }
                });
                window.update(cx, |_, window, _| window.activate_window()).unwrap();
                crate::native_test_support::frames(window, cx);
                let (source, start, initial, saved_layout) = window.update(cx, |host, window, cx| {
                    let owners = verdant_continuous_control::native::gpui_native_owner::all_owners().into_iter()
                        .filter(|owner| {
                            let owner = owner.borrow();
                            owner.window_id == Some(window.window_handle().window_id()) && owner.mounted
                                && owner.props.get("role").and_then(serde_json::Value::as_str) == Some("separator")
                        }).collect::<Vec<_>>();
                    assert_eq!(owners.len(), 1, "the production splitter has one compiled scalar");
                    let source = verdant_continuous_control::ContinuousControlHandle { owner: owners[0].clone() };
                    let bounds = source.owner.borrow().native_hitbox.as_ref().expect("painted production splitter").bounds;
                    assert!(bounds.size.width > px(0.0) && bounds.size.height > px(0.0));
                    let start = bounds.center();
                    let editor = host.editor.read(cx);
                    (source, start, f64::from(value(editor.layout, kind)), editor.prefs.extra.get(LAYOUT_PREFS_KEY).cloned())
                }).unwrap();
                cx.update_window(window.into(), |_, window, cx| {
                    window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                        position: start, button: MouseButton::Left, ..Default::default()
                    }), cx);
                }).unwrap();
                crate::native_test_support::frames(window, cx);
                let vertical = matches!(kind, SplitterKind::InspectorWidth | SplitterKind::RightDockWidth);
                let sign = if matches!(kind, SplitterKind::RightDockWidth | SplitterKind::LaneHeight) { -1.0 } else { 1.0 };
                let mut last = start;
                let mut expected = initial as f32;
                for delta in [13.37, 27.19] {
                    last = if vertical { point(start.x + px(delta), start.y) } else { point(start.x, start.y + px(delta)) };
                    let distance = if vertical { f64::from(last.x) - f64::from(start.x) } else { f64::from(last.y) - f64::from(start.y) };
                    expected = (initial + distance * sign * if fine { 0.1 } else { 1.0 }) as f32;
                    cx.update_window(window.into(), |_, window, cx| {
                        window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                            position: last, pressed_button: Some(MouseButton::Left),
                            modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
                        }), cx);
                    }).unwrap();
                    // The observed Editor is rendered again with its f32 draft;
                    // retaining only the initial props would hide this bug.
                    crate::native_test_support::frames(window, cx);
                    window.update(cx, |host, window, cx| {
                        let editor = host.editor.read(cx);
                        assert_eq!(source.state("phase").unwrap(), serde_json::json!("dragging"), "{kind:?}, fine={fine}");
                        assert_eq!(value(editor.layout, kind), expected);
                        assert_eq!(source.state("value").unwrap().as_f64(), Some(f64::from(expected)));
                        // Window-bounds observation can independently dirty
                        // preferences; inspect the actual persisted layout.
                        assert_eq!(editor.prefs.extra.get(LAYOUT_PREFS_KEY), saved_layout.as_ref(),
                            "preview must not persist layout preferences");
                        assert!(window.captured_hitbox().is_some(), "the same drag must retain native capture after repaint");
                    }).unwrap();
                }
                cx.update_window(window.into(), |_, window, cx| {
                    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                        position: last, button: MouseButton::Left,
                        modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
                    }), cx);
                }).unwrap();
                crate::native_test_support::frames(window, cx);
                window.update(cx, |host, window, cx| {
                    let editor = host.editor.read(cx);
                    assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
                    assert_eq!(value(editor.layout, kind), expected);
                    assert_eq!(source.state("value").unwrap().as_f64(), Some(f64::from(expected)),
                        "release must retain the accepted layout for the next interaction");
                    assert!(editor.prefs_dirty, "release commits the production layout callback");
                    assert_eq!(LayoutPrefs::from_prefs(&editor.prefs), editor.layout);
                    assert_ne!(editor.prefs.extra.get(LAYOUT_PREFS_KEY), saved_layout.as_ref());
                    assert!(window.captured_hitbox().is_none());
                    assert!(source.take_failures().is_empty());
                }).unwrap();
            }
        }
    }
}

#[derive(IntoElement)]
struct TimelineScrollbar { editor: Entity<Editor>, arrange: bool, theme: Theme }
impl gpui::RenderOnce for TimelineScrollbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        use crate::proto_scalar::{EventKind, Sample, Spec, TrackPress, scalar};
        let ed = self.editor.read(cx);
        let arrange = self.arrange;
        let vp = if arrange { ed.arrange } else { ed.session.viewport };
        let visible = (vp.width / vp.px_per_tick.max(1e-6)).max(1.0);
        let max_end = ed.arrange_cache.end_tick().max(ed.project().ppq.saturating_mul(16)) as f64;
        let total = (max_end + visible * 0.5).max(vp.start_tick + visible).max(1.0);
        let max_scroll = (total - visible).max(1.0);
        let ratio = (visible / total).clamp(0.04, 1.0);
        let th = self.theme;
        let mut spec = Spec::slider(vp.start_tick.max(0.0), 0.0, max_scroll, if arrange { "编曲区水平滚动" } else { "钢琴卷帘水平滚动" });
        spec.role = "scrollbar"; spec.step = visible / 10.0; spec.fine_step = visible / 100.0;
        spec.default_value = 0.0; spec.preview_echo = true;
        scalar(self.editor, if arrange { "arrange-timeline-scroll" } else { "roll-timeline-scroll" }, spec,
            move |sample, bounds, origin| {
                let w = f64::from(bounds.size.width);
                let thumb_w = (w * ratio).max(24.0).min(w);
                let travel = (w - thumb_w).max(1.0);
                let delta = sample.total_delta.x / travel * max_scroll;
                Sample { value: origin + delta, fine: origin + delta * 0.1, jump: false }
            },
            move |display, _, _, _| canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |bounds, hitbox, window, _| {
                    let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
                    let thumb_w = (w * ratio).max(24.0).min(w);
                    let x = (display.value / max_scroll).clamp(0.0, 1.0) * (w - thumb_w).max(0.0);
                    window.paint_quad(fill(bounds, th.lcd));
                    window.paint_quad(gpui::quad(b(x, 2.0, thumb_w, (h-4.0).max(4.0), bounds.origin), px(4.0),
                        if display.active || hitbox.is_hovered(window) { th.control_pressed } else { th.control }, px(1.0),
                        if display.active || display.focus_visible { th.accent } else { th.separator }, gpui::BorderStyle::Solid));
                }).size_full().into_any_element(),
            move |ed, event, _, cx| {
                if matches!(event.kind, EventKind::Preview | EventKind::Commit | EventKind::Cancel) {
                    if arrange { ed.arrange.start_tick = event.value; } else { ed.session.viewport.start_tick = event.value; }
                    ed.notify_viewport(if arrange { crate::clock::Space::Arrange } else { crate::clock::Space::Editor }, cx);
                }
            })
            .track_begin(move |sample, bounds, accepted| {
                let w = f64::from(bounds.size.width);
                let thumb_w = (w * ratio).max(24.0).min(w);
                let travel = (w - thumb_w).max(1.0);
                let thumb_x = accepted / max_scroll * travel;
                let pointer_start = sample.position.x - sample.total_delta.x;
                TrackPress {
                    value: ((pointer_start - thumb_w * 0.5) / travel).clamp(0.0, 1.0) * max_scroll,
                    thumb_hit: (thumb_x..=thumb_x + thumb_w).contains(&pointer_start),
                }
            })
    }
}

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
        // The roll bar is outside the canvas's PianoRoll focus context.
        // Admit the workspace Escape route without claiming note-navigation
        // keys that belong to the authored scrollbar.
        .child(div().flex_1().h_full().key_context("TimelineScrollbar")
            .child(TimelineScrollbar { editor, arrange: is_arrange, theme: th }))
        .child(
            crate::proto_controls::action(SharedString::from(format!("{id_prefix}-zoom-out")), "水平缩小")
                .w(px(20.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l_1()
                .border_color(th.shade)
                .text_xs()
                .text_color(th.text_secondary)
                .hover(move |s| s.bg(th.control_pressed).text_color(th.text)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.control_pressed)))
                .child("−")
                .on_click(cx.listener(move |ed, _, window, cx| {
                    crate::proto_editor_surface::zoom(ed, is_arrange, "zoom-out", window, cx);
                })),
        )
        .child(
            crate::proto_controls::action(SharedString::from(format!("{id_prefix}-zoom-in")), "水平放大")
                .w(px(20.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .border_l_1()
                .border_color(th.shade)
                .text_xs()
                .text_color(th.text_secondary)
                .hover(move |s| s.bg(th.control_pressed).text_color(th.text)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.control_pressed)))
                .child("+")
                .on_click(cx.listener(move |ed, _, window, cx| {
                    crate::proto_editor_surface::zoom(ed, is_arrange, "zoom-in", window, cx);
                })),
        )
}

#[cfg(test)]
#[path = "ui_controls_selection_tests.rs"]
mod source_selection_tests;

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
        assert_eq!(loaded.inspector_width, 440.0);
        assert_eq!(loaded.right_dock_width, 280.0);
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
mod pointer_boundary_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, WindowHandle, point, size};

    struct Harness { editor: Entity<Editor> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let pill = self.editor.update(cx, |ed, cx| numeric_pill(ed, Num::MasterPan, Arg::None, "boundary-pan", true, cx));
            div().size_full().child(div().w(px(220.0)).child(pill))
                .child(canvas(|_, _, _| {}, |_, _, window, _| {
                    window.on_mouse_event(|_: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Bubble { cx.stop_propagation(); }
                    });
                }).size(px(0.0)))
        }
    }
    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        let window = cx.open_window(size(px(320.0), px(180.0)), |window, cx| Harness {
            editor: cx.new(|cx| Editor::for_native_test(crate::native_test_support::instrument_project(), window, cx)),
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window
    }
    fn down(host: &Harness, window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position: point(px(50.0), px(14.0)), button: MouseButton::Left, ..Default::default()
        }), cx);
        assert!(has_numeric_scrub(host.editor.entity_id(), cx));
        assert!(window.captured_hitbox().is_some());
    }
    #[gpui::test]
    fn numeric_scrub_outside_move_or_up_restores_preview_without_commit(cx: &mut TestAppContext) {
        for terminal in [false, true] {
            let window = open(cx);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: point(px(66.0), px(14.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                assert_eq!(host.editor.read(cx).numeric_value(Num::MasterPan, Arg::None), 4.0);
                let outside = point(px(70.0), px(180.0));
                if terminal {
                    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: outside, button: MouseButton::Left, ..Default::default() }), cx);
                } else {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent { position: outside, pressed_button: Some(MouseButton::Left), ..Default::default() }), cx);
                }
                assert!(!has_numeric_scrub(host.editor.entity_id(), cx));
                assert!(host.editor.read(cx).numeric_preview.is_none());
                assert!(window.captured_hitbox().is_none());
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: point(px(66.0), px(14.0)), button: MouseButton::Left, ..Default::default() }), cx);
                assert_eq!(host.editor.read(cx).numeric_value(Num::MasterPan, Arg::None), 0.0);
                assert_eq!(host.editor.read(cx).revision().get(), 0);
            }).unwrap();
        }
    }
    #[gpui::test]
    fn numeric_scrub_release_updates_final_sample_before_stopped_bubble(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| {
            down(host, window, cx);
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                position: point(px(82.0), px(14.0)), button: MouseButton::Left, ..Default::default()
            }), cx);
            assert_eq!(host.editor.read(cx).numeric_value(Num::MasterPan, Arg::None), 8.0);
            assert_eq!(host.editor.read(cx).revision().get(), 1);
            assert!(!has_numeric_scrub(host.editor.entity_id(), cx));
            assert!(window.captured_hitbox().is_none());
        }).unwrap();
    }

    #[gpui::test]
    fn numeric_scrub_native_motion_is_classified_by_the_compiled_source(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| {
            down(host, window, cx);
            window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(52.0), px(16.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
            }), cx);
            assert!(host.editor.read(cx).numeric_preview.is_none(), "diagonal motion below three pixels stays armed");
            window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(54.0), px(22.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
            }), cx);
            assert_eq!(host.editor.read(cx).numeric_value(Num::MasterPan, Arg::None), -2.0,
                "the prototype chooses the dominant downward axis");
            window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                position: point(px(50.0), px(2.0)), button: MouseButton::Left,
                modifiers: gpui::Modifiers { shift: true, ..Default::default() }, ..Default::default()
            }), cx);
            assert_eq!(host.editor.read(cx).numeric_value(Num::MasterPan, Arg::None), 1.5,
                "the terminal raw motion is sampled using the source's fine step");
            assert_eq!(host.editor.read(cx).revision().get(), 1);
            assert!(!has_numeric_scrub(host.editor.entity_id(), cx));
            assert!(window.captured_hitbox().is_none());
        }).unwrap();
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

#[cfg(test)]
#[path = "ui_controls_timeline_tests.rs"]
mod timeline_tests;
