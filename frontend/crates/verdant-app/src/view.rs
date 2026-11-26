//! Application chrome and native DAW/SVS layout. Time-dependent LCD, meters and
//! playheads are retained child entities; this view paints editing state.

use gpui::{
    AnyElement, Context, Div, Entity, FontWeight, IntoElement, Render, Role,
    SharedString, Stateful, StyleRefinement, WeakEntity, Window, div, prelude::*, px, svg,
};
use verdant_audio::{DeviceStatus, OutputKind, Transport};
use verdant_backend::adapter::NoteSupport;
use verdant_backend::params::{self, Param, Support};
use verdant_backend::Severity;
#[cfg(test)]
use verdant_backend::Route;
use verdant_domain::Vibrato;
use verdant_domain::model::VibratoPreset;

use crate::actions::*;
use crate::assets::icon;
use crate::automation_lane::{AUTO_LANE_H, automation_curve_canvas, automation_header};
use crate::dialogs::{NOTE_ROOT_NAMES, audio_clip_inspector_section, modal_dialog_layer};
use crate::disclosures::i18n::{self, Surface};
use crate::editor::{Bottom, Editor, LaneTool, Tool, WorkerStatus, WorkflowAction};
use crate::generated::controls::{
    self as controls, CommandArg as Arg, CommandId as Cmd, NumericId as Num, SelectId,
};
use crate::phoneme_lane::{PHONEME_LANE_H, timeline_phoneme_lane};
use crate::theme::{Theme, mix};
use crate::ui_controls::AdwStates;
use crate::ui_controls::{
    BAR_BTN_H, PANEL_BTN_H, RightDockTab, SplitterKind, WidthClass,
    generated_select_button, horizontal_timeline_bar, icon_command_button, icon_selectable_button,
    numeric_pill, route_short_name, select_button, selectable_button, selectable_command_button,
    splitter_bar,
};

fn select_parameter_lane(ed:&mut Editor,param:Param,window:&mut Window,cx:&mut Context<Editor>) {
    ed.cancel_edits(verdant_session::CancelReason::Escape,window,cx);ed.point_selection.clear();
    if let Some(slot)=ed.integrations.lanes.iter().position(|value|*value==param) {ed.integrations.active_panel=slot;}
    else {let slot=ed.integrations.active_panel.min(ed.integrations.lanes.len()-1);ed.integrations.lanes[slot]=param;}
    ed.param=param;cx.notify();
}

const RULER_H: f32 = 24.0;
const KEYS_W: f32 = 52.0;

/// Keep blocking errors visible even when earlier notes have many warnings.
/// Preserve source order within each severity and copy only visible track IDs.
fn prioritized_diagnostics(
    plans: &[verdant_backend::PhrasePlan],
    limit: usize,
) -> (Vec<(verdant_domain::TrackId, &verdant_backend::Diagnostic)>, usize) {
    let entries = || plans.iter().flat_map(|plan|
        plan.diagnostics.iter().map(move |diagnostic| (&plan.track, diagnostic)));
    let visible: Vec<_> = entries().filter(|(_, diagnostic)| diagnostic.severity == Severity::Error)
        .chain(entries().filter(|(_, diagnostic)| diagnostic.severity == Severity::Warning))
        .take(limit)
        .map(|(track, diagnostic)| (track.clone(), diagnostic))
        .collect();
    let omitted = plans.iter().map(|plan| plan.diagnostics.len()).sum::<usize>()
        .saturating_sub(visible.len());
    (visible, omitted)
}

fn device_status_text(status: DeviceStatus) -> String {
    match status {
        DeviceStatus::Starting => "正在初始化音频输出…".to_string(),
        DeviceStatus::Running(info) => match info.kind {
            OutputKind::Device { name, channels, format } => {
                format!("设备运行中 · {name} ({channels}ch {format})")
            }
            OutputKind::SilentClock { reason } => format!("静默时钟运行中 · {reason}"),
        },
        DeviceStatus::Recovering { device, attempt } => {
            format!("设备 {device} 断开，正在尝试第 {attempt} 次重连…")
        }
        DeviceStatus::Failed(err) => format!("设备故障 ({:?}) · {}", err.kind, err.message),
    }
}

/// Timeline layout and drawing, cached independently of the surrounding controls.
pub struct StaticEditor {
    editor: WeakEntity<Editor>,
}

impl StaticEditor {
    pub fn new(editor: WeakEntity<Editor>, cx: &mut Context<Self>) -> Self {
        if let Some(owner) = editor.upgrade() {
            cx.observe(&owner, |_, _, cx| cx.notify()).detach();
        }
        Self { editor }
    }
}

impl Render for StaticEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.editor.upgrade() else {
            return div().into_any_element();
        };
        editor.update(cx, |ed, cx| ed.bottom_content(window, cx))
    }
}

#[derive(Clone, Copy)]
enum ChromePane {
    ControlBar,
    PropertyDock,
    Arrangement,
}

struct ChromeView {
    editor: WeakEntity<Editor>,
    pane: ChromePane,
}

impl Render for ChromeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.editor.upgrade() else { return div().into_any_element() };
        editor.update(cx, |ed, cx| match self.pane {
            ChromePane::ControlBar => ed.control_bar(cx).into_any_element(),
            ChromePane::PropertyDock => ed.right_synthv_dock(cx).into_any_element(),
            ChromePane::Arrangement => ed.arrangement_content(cx),
        })
    }
}

pub(crate) struct ChromeViews {
    control_bar: Entity<ChromeView>,
    property_dock: Entity<ChromeView>,
    arrangement: Entity<ChromeView>,
}

impl ChromeViews {
    pub(crate) fn new(editor: WeakEntity<Editor>, cx: &mut Context<Editor>) -> Self {
        let mut view = |pane| cx.new(|cx: &mut Context<ChromeView>| {
            if let Some(owner) = editor.upgrade() {
                cx.observe(&owner, |_, _, cx| cx.notify()).detach();
            }
            ChromeView { editor: editor.clone(), pane }
        });
        Self {
            control_bar: view(ChromePane::ControlBar),
            property_dock: view(ChromePane::PropertyDock),
            arrangement: view(ChromePane::Arrangement),
        }
    }

    pub(crate) fn invalidate_arrangement(&self, cx: &mut Context<Editor>) {
        self.arrangement.update(cx, |_, cx| cx.notify());
    }
}

fn cached_view<V: Render>(view: &Entity<V>, style: StyleRefinement, a11y: bool) -> AnyElement {
    if a11y { view.clone().into_any_element() } else { view.clone().cached(style).into_any_element() }
}


fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, th: Theme) -> crate::proto_controls::Action {
    selectable_button(id, label, false, th)
}

fn toggle(id: &'static str, name: &'static str, active: bool, size: f32, th: Theme) -> crate::proto_controls::Action {
    crate::proto_controls::action(id, name)
        .role(Role::Button)
        .aria_label(name)
        .aria_selected(active)
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .when(active, |d| d.bg(th.checked))
        .hover(move |s| s.bg(if active { th.checked } else { th.hover })).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, if active { th.checked } else { th.hover })))
        .child(svg().path(icon(name)).size(px(16.0)).text_color(th.text))
}

pub fn command_button(
    ed: &Editor,
    cmd: Cmd,
    arg: Arg,
    label: impl Into<SharedString>,
    key: impl Into<gpui::ElementId>,
    cx: &mut Context<Editor>,
) -> crate::proto_controls::Action {
    selectable_command_button(ed, cmd, arg, label, key, false, cx)
}

fn command_icon(
    ed: &Editor,
    cmd: Cmd,
    name: &'static str,
    label: &'static str,
    key: &'static str,
    active: bool,
    cx: &mut Context<Editor>,
) -> crate::proto_controls::Action {
    let th = ed.theme;
    crate::ui_controls::flat_command_button(ed, cmd, Arg::None, "", key, active, cx)
        .aria_label(label)
        .size(px(crate::ui_controls::bar_button_size(ed.touch_ui)))
        .flex()
        .items_center()
        .justify_center()
        .child(svg().path(icon(name)).size(px(16.0)).text_color(th.text))
}

pub fn numeric_field(ed: &Editor, id: Num, arg: Arg, key: impl Into<SharedString>, cx: &mut Context<Editor>) -> Stateful<Div> {
    numeric_pill(ed, id, arg, key, false, cx)
}

pub fn a11y_listener(
    cx: &Context<Editor>,
    listener: impl Fn(&mut Editor, Option<&gpui::accesskit::ActionData>, &mut Window, &mut Context<Editor>) + 'static,
) -> impl FnMut(Option<&gpui::accesskit::ActionData>, &mut Window, &mut gpui::App) + 'static {
    let weak = cx.weak_entity();
    move |data, window, cx| {
        let _ = weak.update(cx, |ed, cx| listener(ed, data, window, cx));
    }
}

/// The 46 px top bar every pane shares, so pane dividers line up.
pub(crate) const PANE_HEADER_H: f32 = 46.0;

pub(crate) fn pane_header(th: Theme) -> Div {
    div().h(px(PANE_HEADER_H)).flex_none().w_full().min_w_0().px(px(8.0)).flex().items_center().gap(px(6.0))
        .bg(th.surface).border_b_1().border_color(th.shade)
}

/// libadwaita preferences group (AdwPreferencesGroup): a heading over a
/// boxed list. Each child becomes a row of the rounded card, with a hairline
/// between rows. Builder methods mirror `Div`, so call sites read the same.
pub(crate) struct Group {
    title: SharedString,
    /// Each row and whether it is flush (draws its own full-width padding,
    /// like an AdwComboRow) instead of being padded by the card.
    rows: Vec<(AnyElement, bool)>,
    th: Theme,
}

pub(crate) fn section(title: &str, th: Theme) -> Group {
    Group { title: title.to_string().into(), rows: Vec::new(), th }
}

impl Group {
    pub(crate) fn child(mut self, row: impl IntoElement) -> Self {
        self.rows.push((row.into_any_element(), false));
        self
    }
    pub(crate) fn children(mut self, rows: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.rows.extend(rows.into_iter().map(|row| (row.into_any_element(), false)));
        self
    }
    /// A full-width row that pads itself, so its leading icon lines up with
    /// the content of the padded rows around it.
    pub(crate) fn flush_child(mut self, row: impl IntoElement) -> Self {
        self.rows.push((row.into_any_element(), true));
        self
    }
}

impl IntoElement for Group {
    type Element = AnyElement;
    fn into_element(self) -> Self::Element {
        let th = self.th;
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .mx(px(12.0))
            .mt(px(18.0))
            // The heading starts on the card's edge (AdwPreferencesGroup).
            .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(th.text).child(self.title))
            .child(div().flex().flex_col().rounded(px(12.0)).bg(th.card).overflow_hidden()
                .children(self.rows.into_iter().enumerate().map(|(i, (row, flush))| {
                    div().min_h(px(50.0)).flex().flex_col().justify_center().gap(px(8.0))
                        .when(!flush, |d| d.px(px(12.0)).py(px(8.0)))
                        .when(i > 0, |d| d.border_t_1().border_color(th.shade))
                        .child(row)
                })))
            .into_any_element()
    }
}

/// AdwActionRow: title on the left, value (dimmed) on the right.
fn row(name: &str, value: impl Into<SharedString>, th: Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .text_sm()
        .child(div().flex_1().min_w_0().truncate().text_color(th.text).child(name.to_string()))
        .child(div().max_w(px(150.0)).flex_none().truncate().text_color(th.text_secondary).child(value.into()))
}

/// A GtkSwitch: 46×26 track, white knob, accent when on.
pub(crate) fn switch(on: bool, th: Theme) -> Div {
    div().w(px(46.0)).h(px(26.0)).flex_none().rounded(px(13.0)).p(px(3.0)).flex().items_center()
        .when(on, |d| d.justify_end())
        .bg(if on { th.accent } else { th.control_pressed })
        .child(div().size(px(20.0)).rounded(px(10.0)).bg(gpui::rgb(0xffffff)).shadow_sm())
}

/// AdwSwitchRow: title (and optional subtitle) with a switch; the whole row toggles.
pub(crate) fn switch_row(ed: &Editor, cmd: Cmd, key: &'static str, title: &str, subtitle: Option<String>, on: bool, cx: &mut Context<Editor>) -> crate::proto_controls::Action {
    let th = ed.theme;
    let on = ed.source_command_selection(cmd, &Arg::None).unwrap_or(on);
    let action = crate::proto_controls::action(key, title.to_string())
        .flex().items_center().gap(px(12.0)).cursor_pointer()
        .child(div().flex_1().min_w_0().flex().flex_col()
            .child(div().text_sm().text_color(th.text).child(title.to_string()))
            .children(subtitle.map(|s| div().text_xs().text_color(th.text_secondary).child(s))))
        .child(switch(on, th));
    crate::ui_controls::bind_command_action(ed, cmd, Arg::None, action, on, cx)
}

fn support_text(support: Support) -> String {
    match support {
        Support::Synthesis(s) | Support::PostProcess(s) => s.to_string(),
        Support::Unsupported(s) => format!("当前路线不可渲染：{s}（数据仍会保留）"),
    }
}

impl Editor {
    fn control_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.theme;
        let class = self.width_class;
        let btn = crate::ui_controls::bar_button_size(self.touch_ui);
        let (wide, medium_up, phone) = (class == WidthClass::Wide, class >= WidthClass::Medium, class == WidthClass::Phone);
        let chrome = self.window_chrome;
        let bar_h = crate::ui_controls::bar_height(self.touch_ui);
        let captions = crate::window_chrome::caption_buttons(chrome, bar_h, th, cx);
        let playing = self.audio.transport() == Transport::Playing;
        let pb = self.playback_settings();
        let recording = self.integrations.recording.is_some();
        let total = self.scheduler.wanted().len();
        let ready = self.scheduler.summary().get("ready").copied().unwrap_or(0);
        let errors = self.plans.iter().flat_map(|p| &p.diagnostics).filter(|d| d.severity == Severity::Error).count();
        let state = if let Some(job) = self.scheduler.active() {
            format!("渲染中 · {} ({ready}/{total})", job.display_state())
        } else if errors > 0 {
            format!("{ready}/{total} 就绪 · {errors} 处错误")
        } else {
            format!("{ready}/{total} 乐句就绪")
        };
        let voice = match (&self.worker_status, self.scheduler.capabilities.as_ref()) {
            (_, Some(c)) if c.test_double => "协议测试替身 · 非声库".to_string(),
            (_, Some(c)) => self.active_track()
                .and_then(|track| verdant_backend::adapter::desktop_voice(self.project(), track, c).ok().flatten())
                .map(|(_, voice)| voice.display_name.clone())
                .unwrap_or_else(|| "尚无已校验的桌面声库".into()),
            (WorkerStatus::Exited(s), _) => format!("后端已退出：{s}"),
            (WorkerStatus::SpawnFailed(s), _) => format!("后端无法启动：{s}"),
            _ => "正在连接后端…".into(),
        };

        let top_menu_btn = |id: &'static str, label: &'static str| {
            crate::proto_controls::action(id, label)
                .h(px(btn))
                .px(px(8.0))
                .gap(px(4.0))
                .flex()
                .items_center()
                .rounded(px(6.0))
                .text_sm()
                .text_color(th.text)
                .hover(move |s| s.bg(th.hover)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover)))
                .child(label)
                .when(wide, |d| d.child(svg().path(icon("chevron-down")).size(px(12.0)).text_color(th.text_secondary)))
        };

        // Window title (AdwWindowTitle): the project, and render state as subtitle.
        let title = self.project_path.as_ref()
            .and_then(|p| p.file_stem()).map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "未命名工程".into());
        let dirty = self.store().is_dirty();
        let window_title = crate::proto_controls::action("window-title", "渲染状态与声库").flex().flex_col().items_center().justify_center().min_w_0().max_w(px(220.0)).flex_shrink(1.0)
            .child(div().max_w_full().truncate().text_sm().font_weight(FontWeight::BOLD).text_color(th.text)
                .child(if dirty { format!("• {title}") } else { title }))
            .child(div().max_w_full().truncate().text_xs().text_color(if errors > 0 { th.error } else { th.text_secondary })
                .child(format!("{state} · {voice}")))
            .on_click(cx.listener(|ed, _, _, cx| ed.open_right_dock(RightDockTab::RenderInserts, cx)));
        // View switcher: a linked toggle group, like AdwToggleGroup.
        // GPUI's default accessible Click synthesizes mouse down/up. During
        // a captured parameter drag that up would commit its toolbar-position
        // sample before the view switch could cancel it. Semantic activation
        // must call the same command directly, without inventing pointer input.
        let view_switcher = div().id("view-switcher").role(Role::TabList).aria_label("主要编辑视图").flex().flex_none().items_center().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control)
            .child(toggle("editor-toggle", "piano", self.bottom == Bottom::Editor, btn - 4.0, th).aria_label("钢琴卷帘 (E)").role(Role::Tab)
                .navigation(0, 4, { let editor = cx.entity(); move |index, window, cx| {
                    if let Some(bottom) = [Bottom::Editor, Bottom::Mixer, Bottom::Table, Bottom::Devices].get(index) { editor.update(cx, |ed, cx| ed.show_bottom(*bottom, window, cx)); }
                } })
                .on_a11y_action(gpui::AccessibleAction::Click, a11y_listener(cx, |ed, _, w, cx| ed.show_bottom(Bottom::Editor, w, cx)))
                .on_click(cx.listener(|ed, _, w, cx| ed.show_bottom(Bottom::Editor, w, cx))))
            .child(toggle("mixer-toggle", "sliders-vertical", self.bottom == Bottom::Mixer, btn - 4.0, th).aria_label("混音器 (X)").role(Role::Tab)
                .navigation(1, 4, { let editor = cx.entity(); move |index, window, cx| {
                    if let Some(bottom) = [Bottom::Editor, Bottom::Mixer, Bottom::Table, Bottom::Devices].get(index) { editor.update(cx, |ed, cx| ed.show_bottom(*bottom, window, cx)); }
                } })
                .on_a11y_action(gpui::AccessibleAction::Click, a11y_listener(cx, |ed, _, w, cx| ed.show_bottom(Bottom::Mixer, w, cx)))
                .on_click(cx.listener(|ed, _, w, cx| ed.show_bottom(Bottom::Mixer, w, cx))))
            .child(toggle("table-toggle", "keyboard", self.bottom == Bottom::Table, btn - 4.0, th).aria_label("音符表 (T)").role(Role::Tab)
                .navigation(2, 4, { let editor = cx.entity(); move |index, window, cx| {
                    if let Some(bottom) = [Bottom::Editor, Bottom::Mixer, Bottom::Table, Bottom::Devices].get(index) { editor.update(cx, |ed, cx| ed.show_bottom(*bottom, window, cx)); }
                } })
                .on_a11y_action(gpui::AccessibleAction::Click, a11y_listener(cx, |ed, _, w, cx| ed.show_bottom(Bottom::Table, w, cx)))
                .on_click(cx.listener(|ed, _, w, cx| ed.show_bottom(Bottom::Table, w, cx))))
            .child(toggle("devices-toggle", "sliders-horizontal", self.bottom == Bottom::Devices, btn - 4.0, th).aria_label("效果器链").role(Role::Tab)
                .navigation(3, 4, { let editor = cx.entity(); move |index, window, cx| {
                    if let Some(bottom) = [Bottom::Editor, Bottom::Mixer, Bottom::Table, Bottom::Devices].get(index) { editor.update(cx, |ed, cx| ed.show_bottom(*bottom, window, cx)); }
                } })
                .on_a11y_action(gpui::AccessibleAction::Click, a11y_listener(cx, |ed, _, w, cx| ed.show_bottom(Bottom::Devices, w, cx)))
                .on_click(cx.listener(|ed, _, w, cx| ed.show_bottom(Bottom::Devices, w, cx))));
        let _ = top_menu_btn;
        let corners = crate::window_chrome::corner_radii(chrome);

        div()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .rounded_tl(corners.top_left)
            .rounded_tr(corners.top_right)
            .h(px(bar_h))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(6.0))
            .when(chrome.leading_inset() > 0.0, |bar| bar.pl(px(chrome.leading_inset())))
            .when(captions.is_some(), |bar| bar.pr_0())
            .bg(th.headerbar)
            .border_b_1()
            .border_color(th.shade)
            // Start: views, history, title.
            .when(!phone, |bar| bar.child(view_switcher))
            .when(!phone, |bar| bar
                .child(command_icon(self, Cmd::Undo, "undo-2", "撤销 (Ctrl+Z)", "undo", false, cx))
                .child(command_icon(self, Cmd::Redo, "redo-2", "重做 (Ctrl+Shift+Z)", "redo", false, cx)))
            .when(medium_up, |bar| bar.child(crate::window_chrome::drag_region("titlebar-drag-title-l", chrome, cx).max_w(px(24.0))).child(window_title))
            // Empty header stretches move the window.
            .child(crate::window_chrome::drag_region("titlebar-drag-left", chrome, cx))
            // Centre: transport as one linked group, then the LCD.
            .child(div().id("transport").flex().flex_none().items_center().rounded(px(8.0)).bg(th.control).overflow_hidden()
            .when(!phone, |bar| bar.child(command_icon(self, Cmd::GoToStart, "skip-back", "回到开头", "go-start", false, cx).rounded_none()))
            .child(command_icon(self, Cmd::TogglePlay, if playing { "pause" } else { "play" }, "播放 / 暂停 (Space)", "play", playing, cx).rounded_none())
            .child(command_icon(self, Cmd::Stop, "square", "停止", "stop", false, cx).rounded_none())
            .when(!phone, |bar| bar.child(
                crate::proto_controls::action("transport-record", if recording { "完成并提交 MIDI 录音" } else { "选择 MIDI 输入开始录音" })
                    .aria_selected(recording)
                    .size(px(btn))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(recording, |d| d.bg(th.error))
                    .hover(move |s| s.bg(if recording { th.error } else { th.hover })).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, if recording { th.error } else { th.hover })))
                    .child(svg().path(icon("circle-dot")).size(px(16.0)).text_color(if recording { th.accent_text } else { th.error }))
                    .on_click(cx.listener(move |ed, _, w, cx| {
                        use crate::editor::integrations::IntegrationAction as I;
                        let act = if recording { I::FinishRecording } else { I::MidiPorts };
                        ed.run_workflow(WorkflowAction::Integration(act), w, cx);
                    })),
            )))
            // The LCD keeps its full width until phones, where it takes the
            // space that is left and shows the position only.
            .child(div().h(px(36.0)).when(phone, |d| d.flex_1().min_w(px(112.0)).max_w(px(252.0))).when(!phone, |d| d.w(px(252.0)).flex_none())
                .child(self.playback.anchor(crate::clock::Surface::Lcd)))
            .when(!phone, |bar| bar.child(command_icon(self, Cmd::ToggleLoop, "repeat", "循环播放", "transport-loop", pb.looping, cx)))
            .when(!phone, |bar| bar.child(command_icon(self, Cmd::ToggleMetronome, "bell", "节拍器", "transport-click", pb.metronome, cx)))
            .when(!phone, |bar| bar.child(crate::window_chrome::drag_region("titlebar-drag-right", chrome, cx)))
            // End: render (badge carries errors), primary menu, dock.
            .when(!phone, |bar| bar.child(command_icon(self, Cmd::RenderNow, "audio-waveform", "立即渲染", "render", self.scheduler.active().is_some(), cx)
                .relative()
                .when(errors > 0, |d| d.aria_label(format!("立即渲染（{errors} 处错误）")).child(
                    div().absolute().top(px(4.0)).right(px(4.0)).size(px(7.0)).rounded(px(3.5)).bg(th.error)))))
            .child(crate::proto_controls::action("primary-menu", "主菜单").size(px(btn)).flex().items_center().justify_center().rounded(px(6.0))
                .hover(move |s| s.bg(th.hover)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover)))
                .child(svg().path(icon("menu")).size(px(16.0)).text_color(th.text))
                .on_click(cx.listener(move |ed, ev: &crate::proto_controls::Activation, w, cx| {
                    ed.main_menu(ev.position(), w, cx)
                })))
            .child(command_icon(self, Cmd::ToggleRightDock, "panel-right", "属性面板", "right-dock-toggle", self.right_dock_visible(), cx))
            .children(captions)
            .when(th.pride, |bar| bar.relative().child(
                // Paint only: no hitbox, layout space, label or event handler.
                gpui::canvas(|_, _, _| (), |bounds, (), window, _| {
                    let colors = [0xe40303, 0xff8c00, 0xffed00, 0x008026, 0x004dff, 0x750787];
                    let width = bounds.size.width / colors.len() as f32;
                    for (index, color) in colors.into_iter().enumerate() {
                        let left = width * index as f32;
                        let right = if index == colors.len() - 1 { bounds.size.width } else { width * (index + 1) as f32 };
                        window.paint_quad(gpui::fill(gpui::Bounds::new(
                            gpui::point(bounds.origin.x + left, bounds.origin.y),
                            gpui::size(right - left, bounds.size.height),
                        ), gpui::rgb(color)));
                    }
                }).absolute().top_0().left_0().w_full().h(px(2.0)),
            ))
    }

    /// Left Track & Voice Panel: sleek track setup, Singer/Language/Engine `<Select>` dropdowns,
    /// track mix & automation controls, and quick track list switcher.
    /// The current track's voice, language and mix: shown in the 属性 panel
    /// below the selection, as Synthesizer V shows track properties.
    fn track_sections(&self, project: &verdant_domain::Project, track: &verdant_domain::Track, cx: &mut Context<Self>) -> Div {
        let th = self.theme;
        let caps = self.scheduler.capabilities.as_ref();
        let rename_id = track.id.clone();
        let auto_open = self.automation_ui.expanded_tracks.contains(&track.id);

        let instrument = track.instrument().map(|spec| (
            spec.get("name").and_then(serde_json::Value::as_str).unwrap_or("乐器").to_string(),
            spec.get("preset").and_then(serde_json::Value::as_str).filter(|p| !p.is_empty()).map(str::to_string)
                .unwrap_or_else(|| match spec.get("format").and_then(serde_json::Value::as_str) { Some("builtin") | None => "自定义".into(), Some(f) => f.to_uppercase() })));
        let mut track_section = section(if instrument.is_some() { "乐器" } else { "声库与语言" }, th)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().size(px(10.0)).rounded(px(2.5)).bg(th.track(track.color.as_deref())))
                    .child(div().flex_1().text_sm().font_weight(FontWeight::SEMIBOLD).truncate().child(track.name.clone()))
                    .child(
                        button("insp-rename-track", "重命名", th)
                            .h(px(24.0))
                            .px(px(7.0))
                            .on_click(cx.listener(move |ed, _, w, cx| {
                                ed.run_workflow(WorkflowAction::RenameTrack(rename_id.clone()), w, cx);
                            })),
                    ),
            );
        if let Some((name, preset)) = &instrument {
            track_section = track_section.child(row(name, preset.clone(), th))
                .child(button("insp-open-instrument", "编辑乐器", th).h(px(PANEL_BTN_H))
                    .on_click(cx.listener(|ed, _, w, cx| {
                        let track = ed.session.track.clone();
                        ed.show_bottom(Bottom::Devices, w, cx);
                        ed.open_builtin_instrument_editor(&track, cx);
                    })));
        } else {
            track_section = track_section.flush_child(self.singer_language_controls(cx));
        }

        if let Some((_, voice)) = caps.filter(|_| instrument.is_none()).and_then(|c| verdant_backend::adapter::desktop_voice(project, track, c).ok().flatten()) {
            track_section = track_section.child(row("当前声源", voice.display_name.clone(), th));
        }

        let mix_section = section("混音", th)
            .child(numeric_field(self, Num::TrackGainDb, Arg::Track(track.id.clone()), "track-gain", cx))
            .child(numeric_field(self, Num::TrackPan, Arg::Track(track.id.clone()), "track-pan", cx))
            .child(
                div()
                    .flex()
                    .gap(px(5.0))
                    .child(
                        selectable_command_button(
                            self,
                            Cmd::ToggleMute,
                            Arg::Track(track.id.clone()),
                            "M 静音",
                            "left-mute-btn",
                            track.muted,
                            cx,
                        )
                        .h(px(PANEL_BTN_H))
                        .flex_1(),
                    )
                    .child(
                        command_button(self, Cmd::ToggleSolo, Arg::Track(track.id.clone()), "S 独奏", "left-solo-btn", cx)
                            .h(px(PANEL_BTN_H))
                            .flex_1()
                            .aria_selected(track.solo)
                            .border_1()
                            .border_color(if track.solo { th.warning } else { th.control })
                            .when(track.solo, |d| d.bg(th.warning).text_color(th.on_track()).font_weight(FontWeight::SEMIBOLD)),
                    )
                    .child(
                        selectable_command_button(
                            self,
                            Cmd::ToggleTrackAutomation,
                            Arg::Track(track.id.clone()),
                            "A 自动化",
                            "insp-track-auto",
                            auto_open,
                            cx,
                        )
                        .h(px(PANEL_BTN_H))
                        .flex_1(),
                    ),
            );

        div().flex().flex_col().child(track_section).child(mix_section)
    }

    /// Synthesizer V Studio-style right dock: a vertical bar of panel buttons
    /// at the window edge; each open panel becomes a column, and panels opened
    /// later stack further to the left.
    fn right_synthv_dock(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.theme;
        let project = self.store().snapshot();
        let Some(track) = project.track(&self.session.track) else {
            return div().into_any_element();
        };
        let errors = self.plans.iter().flat_map(|p| &p.diagnostics).filter(|d| d.severity == Severity::Error).count();
        let rail = div()
            .id("dock-panels").role(Role::Toolbar).aria_label("面板")
            .w(px(crate::ui_controls::PANEL_RAIL_WIDTH)).h_full().flex_none()
            .flex().flex_col().items_center().gap(px(4.0)).py(px(8.0))
            .border_l_1().border_color(th.shade)
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|ed, _, window, _| {
                if ed.lyric.is_some() { window.prevent_default(); }
            }))
            .children(RightDockTab::ORDER.into_iter().map(|tab| {
                let open = self.layout.panel_open(tab);
                crate::proto_controls::action(SharedString::from(format!("dock-panel-{tab:?}")), tab.label())
                    .role(Role::Switch).aria_toggled(if open { gpui::Toggled::True } else { gpui::Toggled::False })
                    .aria_label(format!("{}面板", tab.label()))
                    .relative().w(px(44.0)).py(px(6.0)).flex().flex_col().items_center().gap(px(3.0)).rounded(px(8.0))
                    .flat_states(th, open)
                    .child(svg().path(icon(tab.icon())).size(px(16.0)).text_color(if open { th.text } else { th.text_secondary }))
                    .child(div().text_size(px(10.5)).text_color(if open { th.text } else { th.text_secondary })
                        .when(open, |d| d.font_weight(FontWeight::SEMIBOLD)).child(tab.label()))
                    .when(tab == RightDockTab::RenderInserts && errors > 0, |d| d.child(
                        div().absolute().top(px(4.0)).right(px(8.0)).size(px(6.0)).rounded(px(3.0)).bg(th.error)))
                    .on_click(cx.listener(move |ed, _, _, cx| ed.toggle_right_panel(tab, cx)))
            }));

        // Rightmost first: ORDER runs outward from the bar, so later panels
        // sit further left.
        let open: Vec<RightDockTab> = RightDockTab::ORDER.into_iter().filter(|tab| self.layout.panel_open(*tab)).collect();
        let columns = open.into_iter().rev().map(|tab| {
            let content = match tab {
                RightDockTab::VoiceTimbre => self.right_dock_properties_panel(&project, cx),
                RightDockTab::Synthesis => self.right_dock_synthesis_panel(&project, cx),
                RightDockTab::RenderInserts => self.right_dock_render_inserts_tab(track, cx),
                RightDockTab::Dictionary => self.right_dock_dictionary_tab(&project, cx),
                RightDockTab::HistorySystem => self.right_dock_history_system_tab(cx),
            };
            div().w(px(self.layout.right_dock_width)).h_full().flex_none().flex().flex_col().border_l_1().border_color(th.shade)
                .child(div().h(px(46.0)).flex_none().pl(px(12.0)).pr(px(4.0)).flex().items_center().gap(px(8.0))
                    .border_b_1().border_color(th.shade)
                    .child(div().flex_1().text_sm().font_weight(FontWeight::BOLD).child(tab.label()))
                    .child(crate::proto_controls::action(SharedString::from(format!("close-panel-{tab:?}")), format!("关闭{}面板", tab.label()))
                        .size(px(28.0)).flex_none().rounded(px(6.0)).flex().items_center().justify_center()
                        .hover(move |s| s.bg(th.hover)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover)))
                        .child(svg().path(icon("x")).size(px(13.0)).text_color(th.text_secondary))
                        .on_click(cx.listener(move |ed, _, _, cx| ed.toggle_right_panel(tab, cx)))))
                .child(crate::proto_scroll::viewport(SharedString::from(format!("right-panel-scroll-{tab:?}")), format!("{}面板", tab.label()), "vertical")
                    .flex_1().min_h_0().child(div().pb(px(18.0)).child(content)))
        });

        div()
            .id("right-property-dock")
            .rounded_br(crate::window_chrome::corner_radii(self.window_chrome).bottom_right)
            .w(px(self.layout.right_dock_total_width()))
            .h_full()
            .flex_none()
            .flex()
            .bg(th.surface)
            .children(columns)
            .child(rail)
            .into_any_element()
    }

    /// Properties of the current selection (notes or an audio clip). Shown
    /// at the top of the left inspector only while something is selected, so
    /// the inspector follows what the user is working on.
    fn selection_inspector(&self, track: &verdant_domain::Track, entity: Entity<Self>, cx: &mut Context<Self>) -> Option<Div> {
        if self.session.selection.is_empty() {
            return audio_clip_inspector_section(self, entity, cx);
        }
        let th = self.theme;
        let caps = self.scheduler.capabilities.as_ref();
        let mut notes = section("选中音符", th).child(row("当前选区", format!("{} 个音符", self.session.selection.len()), th));
        let first = self
            .session
            .selection
            .iter()
            .filter_map(|id| self.index.position(id).map(|i| (i, id)))
            .min_by_key(|(i, _)| *i)
            .and_then(|(_, id)| self.index.note(track, id));

        if let Some(note) = first {
            let id = note.id.clone();
            let state = match self.resolved_note_support(note) {
                Some(Ok(NoteSupport::VowelProxy { vowel })) => format!("显式元音代理 /{vowel}/"),
                Some(Ok(NoteSupport::Pinyin(token))) => format!("✓ 可渲染 · {} ({} + {})", token.label_token, token.initial, token.final_),
                Some(Ok(NoteSupport::Phonemes(p))) => format!(
                    "✓ {} · IPA /{}/ · {} 音素{}",
                    p.language,
                    verdant_backend::phonemes::format_canonical(&p.canonical, "ipa").unwrap_or_default(),
                    p.phones.len(),
                    if p.phones.iter().any(|phone| phone.support != "native") { " (近似)" } else { "" }
                ),
                Some(Err(d)) => d.message,
                _ => "等待后端能力声明".into(),
            };
            notes = notes
                .child(row("歌词", if note.lyric.is_empty() { "（空）".into() } else { note.lyric.clone() }, th))
                .child(row("当前读音", verdant_domain::pronunciation_display(&note.pronunciation), th))
                .child(
                    icon_command_button(self, Cmd::EditLyric, Arg::Note(id.clone()), "pencil", "内联编辑歌词 / 音标 (Enter)", "edit-lyric", false, cx)
                        .h(px(PANEL_BTN_H)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if self.error_notes.contains(&id) { th.error } else { th.success })
                        .child(state),
                );

            let readings = verdant_domain::hanzi_readings(&note.lyric);
            if !readings.is_empty() {
                let lyric_base = note.lyric.clone();
                let cur_py = note.pronunciation.pinyin.clone();
                notes = notes
                    .child(div().text_xs().text_color(th.text_secondary).child(if readings.len() > 1 {
                        "多音字候选（点击切换）："
                    } else {
                        "标准读音（点击填入）："
                    }))
                    .child(div().flex().flex_wrap().gap(px(4.0)).children(readings.into_iter().enumerate().map(|(ri, py)| {
                        let nid = id.clone();
                        let full = format!("{lyric_base}/{py}");
                        let active = cur_py == py;
                        selectable_button(SharedString::from(format!("insp-py-{ri}")), py, active, th).on_click(cx.listener(
                            move |ed, _, _, cx| {
                                ed.commit_lyric_text(&nid, &full, cx);
                            },
                        ))
                    })));
            }

            if note.pronunciation.alphabet.is_some() {
                notes = notes.child(
                    selectable_command_button(
                        self,
                        Cmd::TogglePhonemeApproximation,
                        Arg::Note(id.clone()),
                        if note.pronunciation.allow_approximation { "跨语言近似映射 · 已允许" } else { "允许跨语言近似映射" },
                        "phoneme-approximation",
                        note.pronunciation.allow_approximation,
                        cx,
                    )
                    .h(px(PANEL_BTN_H)),
                );
            }
            if let Some(reading) = note.pronunciation.reading_provenance.as_ref().and_then(|value| value.get("reading")) {
                if let Some(original) = reading.get("original_input").and_then(|value| value.as_str()) {
                    notes = notes.child(row("假名来源", original.to_string(), th));
                }
            }

            for (kind, key) in [
                (Num::NotePitchSemitone, "note-pitch"),
                (Num::NoteDetuneCents, "note-detune"),
                (Num::NoteStartTick, "note-start"),
                (Num::NoteDurationTicks, "note-duration"),
            ] {
                notes = notes.child(numeric_field(self, kind, Arg::Note(id.clone()), key, cx));
            }

            if let Some(ns) = caps.and_then(|c| c.native_score.as_ref()) {
                notes = notes.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(div().text_xs().text_color(th.text_secondary).child("元音试音："))
                        .children(ns.english_vowel_proxies.iter().map(|v| {
                            let vowel = v.clone();
                            selectable_button(
                                SharedString::from(format!("vowel-{v}")),
                                v.clone(),
                                note.pronunciation.pinyin == *v,
                                th,
                            )
                            .px(px(7.0))
                            .on_click(cx.listener(move |ed, _, _, cx| ed.set_vowel(&vowel, cx)))
                        })),
                );
            }

            let vibrato_on = note.vibrato.is_some();
            let mut mod_sec = section("音高过渡与颤音包络", th)
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.0))
                        .child(
                            selectable_button("vibrato-off", "无颤音", !vibrato_on, th)
                                .on_click(cx.listener(|ed, _, _, cx| ed.set_vibrato(None, cx))),
                        )
                        .children(VibratoPreset::ALL.into_iter().map(|preset| {
                            button(SharedString::from(format!("insp-vib-{preset:?}")), preset.label(), th)
                                .on_click(cx.listener(move |ed, _, _, cx| ed.apply_vibrato_preset(preset, cx)))
                        }))
                        .child(
                            icon_selectable_button("vibrato-envelope-modal", "sliders-horizontal", "包络…", false, th)
                                .on_click(cx.listener(|ed, _, w, cx| ed.open_vibrato_dialog(w, cx))),
                        ),
                );
            for (kind, key) in [
                (Num::VibratoDepthCents, "vibrato-depth"),
                (Num::VibratoRateHz, "vibrato-rate"),
                (Num::VibratoDelayPercent, "vibrato-delay"),
                (Num::PortamentoMs, "portamento"),
            ] {
                mod_sec = mod_sec.child(numeric_field(self, kind, Arg::None, key, cx));
            }
            let glide_status = match note.portamento_ms {
                None => "自动连音滑音（≤ 80 ms）".to_string(),
                Some(0) => "入音滑音已关闭 (0 ms)".to_string(),
                Some(ms) => format!("手动入音滑音：{ms} ms"),
            };
            mod_sec = mod_sec.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.0))
                    .child(div().text_xs().text_color(th.text_secondary).child(glide_status))
                    .child(
                        selectable_button("portamento-auto", "重置自动", note.portamento_ms.is_none(), th)
                            .on_click(cx.listener(|ed, _, _, cx| ed.set_portamento(None, cx))),
                    ),
            );

            let mut phone_sec = section("音素时序分配", th).child(crate::lanes::phoneme_panel(self, entity.clone(), cx));
            if note.pronunciation.phoneme_durations_permille.is_some() {
                phone_sec = phone_sec.child(
                    command_button(self, Cmd::ResetPhonemeDurations, Arg::Note(id), "恢复默认音素时序", "reset-phonemes", cx)
                        .h(px(PANEL_BTN_H)),
                );
            }

            Some(div()
                .flex()
                .flex_col()
                .children(audio_clip_inspector_section(self, entity, cx))
                .child(notes)
                .child(mod_sec)
                .child(phone_sec))
        } else {
            Some(div().flex().flex_col().children(audio_clip_inspector_section(self, entity, cx)).child(notes))
        }
    }

    /// Selection properties, as in Synthesizer V's note properties panel.
    fn right_dock_properties_panel(&self, project: &verdant_domain::Project, cx: &mut Context<Self>) -> Div {
        let Some(track) = project.track(&self.session.track) else { return div() };
        // Selection first while there is one, then the track it belongs to.
        div().flex().flex_col()
            .children(self.selection_inspector(track, cx.entity(), cx))
            .child(self.track_sections(project, track, cx))
    }

    /// Project-wide synthesis settings.
    fn right_dock_synthesis_panel(&self, project: &verdant_domain::Project, cx: &mut Context<Self>) -> Div {
        let th = self.theme;
        let route = self.route();
        let output_transients = project.render_preferences.output_transients.as_ref();
        div().child(section("引擎与速度", th)
            .child(row("合成引擎", route_short_name(route), th))
            .child(numeric_field(self, Num::TempoBpm, Arg::None, "tempo", cx))
            .child(switch_row(self, Cmd::ToggleOutputTransients, "output-transients", "尾辅音瞬态约束",
                Some(match output_transients {
                    None => "关闭".to_string(),
                    Some(d) if d.strength == 0. || d.max_attenuation_db == 0. => "旁路".to_string(),
                    Some(d) => format!("最多衰减 {:.0} dB", d.max_attenuation_db),
                }),
                output_transients.is_some(), cx)))
    }

    fn right_dock_dictionary_tab(&self, project: &verdant_domain::Project, cx: &mut Context<Self>) -> Div {
        use crate::editor::integrations::IntegrationAction as I;
        let th = self.theme;
        let settings = verdant_backend::phonemizer::settings_from_project(project).ok();
        let active_phonemizer = settings.as_ref().map(|s| s.selected.clone()).unwrap_or_else(|| "默认读音器".into());

        let phonemizer_sec = section("默认读音器", th).child(
            select_button("dock-phonemizer-select", Some("languages"), active_phonemizer, "选择默认读音器", th).on_click(cx.listener(
                |ed, _, w, cx| {
                    ed.run_workflow(WorkflowAction::Integration(I::Phonemizers), w, cx);
                },
            )),
        );

        let mut seg_sec = section("多语言时间分段映射", th);
        if let Some(s) = &settings {
            if s.segments.is_empty() {
                seg_sec = seg_sec.child(div().text_xs().text_color(th.text_tertiary).child("尚未添加区间分段；可在同一轨道不同小节切换中日英读音器。"));
            } else {
                let meter = self.meter();
                for (key, seg) in &s.segments {
                    let tname = project.track(&seg.track).map(|t| t.name.as_str()).unwrap_or("轨道");
                    let p0 = meter.position_at(seg.start_tick);
                    let p1 = meter.position_at(seg.end_tick);
                    let edit_key = key.clone();
                    let del_key = key.clone();
                    seg_sec = seg_sec.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                selectable_button(
                                    SharedString::from(format!("dock-seg-{key}")),
                                    format!("{tname} · {}:{}–{}:{} → {}", p0.bar, p0.beat, p1.bar, p1.beat, seg.selected),
                                    false,
                                    th,
                                )
                                .flex_1()
                                .on_click(cx.listener(move |ed, _, w, cx| {
                                    ed.run_workflow(WorkflowAction::Integration(I::EditSegment(edit_key.clone())), w, cx);
                                })),
                            )
                            .child(
                                selectable_button(SharedString::from(format!("dock-seg-del-{key}")), "×", false, th)
                                    .w(px(26.0))
                                    .px(px(0.0))
                                    .on_click(cx.listener(move |ed, _, w, cx| {
                                        ed.run_workflow(WorkflowAction::Integration(I::RemoveSegment(del_key.clone())), w, cx);
                                    })),
                            ),
                    );
                }
            }
        }
        seg_sec = seg_sec.child(
            icon_selectable_button("dock-add-segment", "plus", "添加多语言区间分段…", false, th)
                .h(px(PANEL_BTN_H))
                .on_click(cx.listener(|ed, _, w, cx| {
                    ed.run_workflow(WorkflowAction::Integration(I::AddSegment), w, cx);
                })),
        );

        let mut dict_sec = section("工程用户词典", th);
        if let Some(s) = &settings {
            let total_entries = s.dictionary.phrases.len() + s.dictionary.lyrics.len();
            if total_entries == 0 {
                dict_sec = dict_sec.child(div().text_xs().text_color(th.text_tertiary).child("工程词典为空；可添加专有名词或多音词固定读音。"));
            } else {
                for (key, entry) in s.dictionary.phrases.iter().take(16) {
                    let k_edit = key.clone();
                    let k_del = key.clone();
                    dict_sec = dict_sec.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                selectable_button(
                                    SharedString::from(format!("dock-dict-p-{key}")),
                                    format!("词组 · {}", entry.lyrics.join("")),
                                    false,
                                    th,
                                )
                                .flex_1()
                                .on_click(cx.listener(move |ed, _, w, cx| {
                                    ed.run_workflow(WorkflowAction::Integration(I::EditDictionaryEntry(k_edit.clone(), true)), w, cx);
                                })),
                            )
                            .child(
                                selectable_button(SharedString::from(format!("dock-dict-p-del-{key}")), "×", false, th)
                                    .w(px(26.0))
                                    .px(px(0.0))
                                    .on_click(cx.listener(move |ed, _, w, cx| {
                                        ed.run_workflow(WorkflowAction::Integration(I::RemoveDictionaryEntry(k_del.clone(), true)), w, cx);
                                    })),
                            ),
                    );
                }
                for (key, _) in s.dictionary.lyrics.iter().take(16) {
                    let k_edit = key.clone();
                    let k_del = key.clone();
                    dict_sec = dict_sec.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                selectable_button(SharedString::from(format!("dock-dict-l-{key}")), format!("单字 · {key}"), false, th)
                                    .flex_1()
                                    .on_click(cx.listener(move |ed, _, w, cx| {
                                        ed.run_workflow(WorkflowAction::Integration(I::EditDictionaryEntry(k_edit.clone(), false)), w, cx);
                                    })),
                            )
                            .child(
                                selectable_button(SharedString::from(format!("dock-dict-l-del-{key}")), "×", false, th)
                                    .w(px(26.0))
                                    .px(px(0.0))
                                    .on_click(cx.listener(move |ed, _, w, cx| {
                                        ed.run_workflow(WorkflowAction::Integration(I::RemoveDictionaryEntry(k_del.clone(), false)), w, cx);
                                    })),
                            ),
                    );
                }
            }
        }
        dict_sec = dict_sec
            .child(
                icon_selectable_button("dock-add-dict", "plus", "新增词典词条…", false, th)
                    .h(px(PANEL_BTN_H))
                    .on_click(cx.listener(|ed, _, w, cx| {
                        ed.run_workflow(WorkflowAction::Integration(I::AddDictionaryEntry), w, cx);
                    })),
            );

        div().flex().flex_col().child(phonemizer_sec).child(seg_sec).child(dict_sec)
    }

    fn right_dock_render_inserts_tab(&self, track: &verdant_domain::Track, cx: &mut Context<Self>) -> Div {
        let th = self.theme;
        let total = self.scheduler.wanted().len();
        let ready = self.scheduler.summary().get("ready").copied().unwrap_or(0);

        let mut diagnostics = section("队列与预检", th)
            .child(row("渲染队列", format!("{ready} / {total} 乐句已就绪"), th))
            .child(
                icon_command_button(self, Cmd::RenderNow, Arg::None, "audio-waveform", "立即重新渲染全部乐句", "dock-render-now", false, cx)
                    .h(px(PANEL_BTN_H)),
            );

        // A consonant that has to start before the song does is fixed by
        // giving the song a bar of lead-in, offered here instead of a dead end.
        let needs_lead_in = self.plans.iter().flat_map(|plan| &plan.diagnostics)
            .any(|d| d.code == "E_PHONEME_TIMING_PREROLL");
        if needs_lead_in {
            diagnostics = diagnostics.child(
                div().flex().flex_col().gap(px(6.0)).p(px(10.0)).rounded(px(8.0)).bg(th.control)
                    .child(div().text_sm().child("开头的辅音没有起唱空间"))
                    .child(div().text_xs().text_color(th.text_secondary)
                        .child("元音对齐拍点时，辅音要提前开始；第一个音符离曲首太近。把所有音符后移一小节即可。"))
                    .child(button("shift-one-bar", "所有音符后移一小节", th).h(px(PANEL_BTN_H))
                        .on_click(cx.listener(|ed, _, _, cx| ed.shift_notes_one_bar(cx)))));
        }
        let (all_diags, omitted_diags) = prioritized_diagnostics(&self.plans, 14);
        if all_diags.is_empty() {
            diagnostics = diagnostics.child(div().text_xs().text_color(th.success).child("✓ 当前全部轨道预检通过，无阻断问题"));
        } else {
            for (i, (diag_track, d)) in all_diags.into_iter().enumerate() {
                let note_id = d.note_id.clone();
                let track_name = self.project().track(&diag_track).map(|t| t.name.clone()).unwrap_or_default();
                let is_err = d.severity == Severity::Error;
                diagnostics = diagnostics.child(
                    crate::proto_controls::action(SharedString::from(format!("diagnostic-{i}")), d.message.clone())
                        .w_full()
                        .p(px(7.0))
                        .rounded(px(5.0))
                        .bg(th.control)
                        .border_1()
                        .border_color(if is_err { th.error } else { th.warning })
                        .hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.control_pressed)))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if is_err { th.error } else { th.warning })
                                .child(format!("{} · {track_name}（点击定位）", if is_err { "错误" } else { "提示" })),
                        )
                        .child(div().text_xs().text_color(th.text).child(d.message.clone()))
                        .on_click(cx.listener(move |ed, _, _, cx| {
                            if ed.lyric.is_some() {
                                ed.notice("歌词编辑进行中；请先确认或取消歌词，再定位诊断");
                                cx.stop_propagation();
                                cx.notify();
                                return;
                            }
                            if !ed.select_track(diag_track.clone(), cx) {
                                cx.stop_propagation();
                                return;
                            }
                            if let Some(id) = &note_id {
                                ed.session.selection = [id.clone()].into();
                                if let Some(n) = ed.active_track().and_then(|t| ed.index.note(t, id)).cloned() {
                                    ed.session.viewport.reveal(&n);
                                }
                            }
                            ed.bottom = Bottom::Editor;
                            cx.notify();
                        })),
                );
            }
        }

        if omitted_diags > 0 {
            diagnostics = diagnostics.child(div().text_xs().text_color(th.text_secondary)
                .child(format!("另有 {omitted_diags} 条诊断；当前优先显示阻断错误")));
        }

        if !self.integrations.report.is_empty() {
            diagnostics = diagnostics.child(section("最近导入 / 导出报告（只读）", th)
                .child(div().text_xs().text_color(th.text_secondary).child(self.integrations.report.clone())));
        }

        // Active Track Audio Insert FX Section (CLAP / VST3)
        let specs = self.insert_specs(&track.id);
        let mut fx_sec = section("本轨音频插入效果器 (CLAP / VST3)", th);
        if specs.is_empty() {
            fx_sec = fx_sec.child(div().text_xs().text_color(th.text_tertiary).child("当前轨道尚未挂载音频效果器插件。"));
        } else {
            for (slot, spec) in specs.iter().enumerate() {
                let tid_open = track.id.clone();
                fx_sec = fx_sec.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            selectable_button(
                                SharedString::from(format!("dock-fx-{slot}")),
                                format!("{} · {}", slot + 1, spec.name),
                                !spec.bypass,
                                th,
                            )
                            .flex_1()
                            .on_click(cx.listener(move |ed, _, _, cx| {
                                ed.bottom = Bottom::Mixer;
                                ed.show_insert_params(tid_open.clone(), slot, cx);
                            })),
                        )
                        .child(command_button(
                            self,
                            Cmd::PluginBypass,
                            Arg::TrackSlot(track.id.clone(), slot as u32),
                            if spec.bypass { "启用" } else { "旁通" },
                            SharedString::from(format!("dock-fx-byp-{slot}")),
                            cx,
                        ))
                        .child(command_button(
                            self,
                            Cmd::PluginRemove,
                            Arg::TrackSlot(track.id.clone(), slot as u32),
                            "×",
                            SharedString::from(format!("dock-fx-rm-{slot}")),
                            cx,
                        )),
                );
            }
        }
        fx_sec = fx_sec.child(
            icon_command_button(
                self,
                Cmd::PluginAdd,
                Arg::TrackSlot(track.id.clone(), specs.len() as u32),
                "plus",
                "添加插入效果器…",
                "dock-add-fx",
                false,
                cx,
            )
            .h(px(PANEL_BTN_H)),
        );

        div().flex().flex_col().child(diagnostics).child(fx_sec)
    }

    fn right_dock_history_system_tab(&self, cx: &mut Context<Self>) -> Div {
        let th = self.theme;
        let caps = self.scheduler.capabilities.as_ref();

        let hist_snap = self.store().history();
        let total_steps = hist_snap.entries.len();
        let cur_pos = hist_snap.position;
        let proj_id = self.project().project_id.clone();
        let base_rev = self.revision();
        let mut history = section("编辑历史（点击任意步直接回溯）", th).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child(format!("当前步：{cur_pos} / {total_steps}")))
                .child(button("open-history-win", "独立窗口…", th).on_click(cx.listener(|ed, _, w, cx| ed.show_history_panel(w, cx)))),
        );
        let start_pos = (total_steps + 1).saturating_sub(10);
        for pos in (start_pos..=total_steps).rev() {
            let lbl = if pos == 0 { "初始状态".to_string() } else { hist_snap.entries[pos - 1].label.to_string() };
            let is_cur = pos == cur_pos;
            let is_future = pos > cur_pos;
            let pid = proj_id.clone();
            history = history.child(
                crate::proto_controls::action(SharedString::from(format!("insp-hist-{pos}")), format!("历史 {pos}：{lbl}"))
                    .aria_selected(is_cur)
                    .h(px(BAR_BTN_H))
                    .px(px(8.0))
                    .rounded(px(5.0))
                    .bg(if is_cur { th.selection } else { th.control })
                    .border_1()
                    .border_color(if is_cur { th.accent } else { th.control })
                    .hover(move |s| s.bg(th.control_pressed)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.control_pressed)))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_xs()
                    .text_color(if is_future { th.text_tertiary } else { th.text })
                    .child(div().w(px(22.0)).flex_none().text_color(th.text_secondary).child(format!("#{pos}")))
                    .child(div().flex_1().truncate().child(lbl))
                    .child(div().text_size(px(10.0)).text_color(if is_cur { th.accent } else { th.text_secondary }).child(
                        if is_cur { "当前" } else if is_future { "重做" } else { "" },
                    ))
                    .on_click(cx.listener(move |ed, _, _, cx| {
                        ed.jump_history(&pid, base_rev, pos, cx);
                    })),
            );
        }

        let appearance = section("界面外观", th)
            .child(generated_select_button(self, SelectId::AppearanceMode, Arg::None, "dock-appearance-select", cx))
            .child(crate::theme::pride_controls(self, cx));

        let dev_summary = match &self.audio_info.kind {
            OutputKind::Device { name, .. } => name.clone(),
            OutputKind::SilentClock { .. } => "静默时钟（不发声）".into(),
        };
        let audio = section("音频输出设备", th)
            .child(row("当前输出", output_text(self), th))
            .child(row("工程", format!("{}{} · {} 轨 · {} 音符", self.revision(), if self.store().is_dirty() { " · 未保存" } else { "" },
                self.project().tracks.len(), self.project().note_count()), th))
            .child(
                select_button("choose-device", Some("volume-2"), dev_summary, "选择音频输出设备", th).on_click(cx.listener(
                    |ed, ev: &crate::proto_controls::Activation, window, cx| {
                        let items = ed.device_menu_items();
                        ed.show_menu(ev.position(), items, window, cx);
                    },
                )),
            )
            .child(div().text_xs().text_color(th.text_secondary).child(device_status_text(self.engine.device_status())))
            .child(command_button(self, Cmd::ReopenDevice, Arg::None, "重新连接音频设备", "recover-device", cx).h(px(PANEL_BTN_H)));

        let mut backend = section("合成后端与资产状态", th)
            .child(command_button(self, Cmd::StartWorker, Arg::None, "重启合成后端", "restart-worker", cx).h(px(PANEL_BTN_H)));
        if let Some(caps) = caps {
            backend = backend.child(row("后端进程", caps.engine.clone(), th));
            let mut internal_total = 0usize;
            let mut internal_ok = 0usize;
            for (name, file) in &caps.files {
                if name.starts_with("synthesis:") {
                    internal_total += 1;
                    if file.available {
                        internal_ok += 1;
                    }
                    continue;
                }
                let display_name = match name.as_str() {
                    "model" => "声库模型",
                    "vocoder" => "声码器",
                    "token" => "词表资产",
                    "onnxruntime" => "ONNX 运行时",
                    "worker_binary" => "合成守护进程",
                    other => other,
                };
                backend = backend.child(row(display_name, if file.available { "✓ 已就绪" } else { "✕ 未配置" }, th));
            }
            if internal_total > 0 {
                backend = backend.child(row("内置管线契约", format!("✓ {internal_ok}/{internal_total} 项通过"), th));
            }
        }

        div().flex().flex_col().child(history).child(appearance).child(audio).child(backend)
    }

    fn editor_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.theme;
        let scale_active = !self.integrations.scale_intervals.is_empty();
        let scale_label = if scale_active {
            format!("音阶 · {}", NOTE_ROOT_NAMES[(self.integrations.scale_root % 12) as usize])
        } else {
            "音阶高亮".into()
        };

        crate::proto_scroll::viewport("editor-header-scroll", "钢琴卷帘工具栏", "horizontal")
            .h(px(40.0))
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .items_center()
            .whitespace_nowrap()
            .gap(px(5.0))
            .px(px(8.0))
            .bg(th.surface)
            .border_b_1()
            .border_color(th.shade)
            // Tools: an icon toggle group (labels are the tooltips / names).
            .child(div().id("tool-group").role(Role::RadioGroup).aria_label("编辑工具").flex().flex_none().items_center().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control)
                .children([
                    (Cmd::SetToolPointer, Tool::Pointer, "mouse-pointer-2", "指针 (V)"),
                    (Cmd::SetToolPencil, Tool::Pencil, "pencil", "铅笔 (P)"),
                    (Cmd::SetToolEraser, Tool::Eraser, "eraser", "橡皮 (E)"),
                    (Cmd::SetToolPitch, Tool::Pitch, "spline", "音高笔"),
                ].into_iter().enumerate().map(|(index,(cmd, tool, icon_name, label))| {
                    let on = self.tool == tool; let entity=cx.entity();
                    crate::ui_controls::flat_command_button(self, cmd, Arg::None, "", SharedString::from(format!("tool-{icon_name}")), on, cx)
                        .role(Role::RadioButton).aria_label(label).size(px(26.0)).px_0()
                        .navigation(index,4,move|index,window,cx| {if let Some(command)=[Cmd::SetToolPointer,Cmd::SetToolPencil,Cmd::SetToolEraser,Cmd::SetToolPitch].get(index) {
                            entity.update(cx,|ed,cx|controls::dispatch(ed,*command,Arg::None,window,cx));
                        }})
                        .child(svg().path(icon(icon_name)).size(px(15.0)).text_color(th.text))
                })))
            .child(numeric_pill(self, Num::SnapGridTicks, Arg::None, "snap", true, cx).flex_none())
            .child(numeric_pill(self, Num::DefaultNoteTicks, Arg::None, "new-note-length", true, cx).flex_none())
            .child(crate::ui_controls::flat_icon_command_button(self, Cmd::TogglePhonemeLane, "mic-vocal", "音素条", "roll-phoneme-toggle", self.layout.show_phoneme_lane, cx).h(px(28.0)).flex_none())
            .child(crate::ui_controls::flat_command_button(self, Cmd::ToggleTempoLane, Arg::None, "BPM 线", "roll-tempo-toggle", self.layout.show_tempo_lane, cx).h(px(28.0)).flex_none().px(px(9.0)).text_sm())
            .child(crate::ui_controls::flat_icon_command_button(self, Cmd::OpenScaleDialog, "sliders-horizontal", scale_label, "roll-scale-btn", scale_active, cx).h(px(28.0)).flex_none())
            .child(crate::ui_controls::flat_icon_command_button(self, Cmd::ToggleOtherTracks, "layers", "多轨重影", "roll-ghost-notes", self.integrations.other_tracks, cx).h(px(28.0)).flex_none())
            .child(div().flex_1())
            .child(crate::ui_controls::flat_icon_command_button(self, Cmd::FitView, "maximize-2", "适合音符", "fit-notes", false, cx).h(px(28.0)).flex_none())
            .child(crate::ui_controls::flat_icon_command_button(self, Cmd::ToggleF0Overlay, "activity", "F0 对照", "show-f0", self.show_f0, cx).h(px(28.0)).flex_none())
    }

    fn lane_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.theme;
        let route = self.route();
        let caps = self.scheduler.capabilities.as_ref();
        let mut tabs = crate::proto_scroll::viewport("lane-tabs-scroll", "参数泳道标签", "horizontal")
            .h(px(40.0))
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .items_center()
            .whitespace_nowrap()
            .gap(px(4.0))
            .px(px(8.0))
            .bg(th.surface)
            .border_t_1()
            .border_color(th.shade);
        // Parameter lanes: one toggle group (AdwToggleGroup).
        let mut group = div().id("lane-group").role(Role::TabList).aria_label("参数泳道").flex().flex_none().items_center().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control);
        for (index,(param, has_data)) in params::ALL.into_iter().zip(self.active_param_data()).enumerate() {
            let active = param == self.param;
            let shown = self.integrations.lanes.contains(&param);
            let supported = params::support(param, route, caps).is_effective();
            let text_label = if has_data { format!("{} •", param.short()) } else { param.short().to_string() };
            let mut tab_btn = selectable_button(SharedString::from(format!("lane-{}", param.short())), text_label, active, th);
            if !supported && !active {
                tab_btn = tab_btn.text_color(th.text_tertiary);
            }
            // A parameter shown in another lane is marked; clicking it goes there.
            if shown && !active { tab_btn = tab_btn.font_weight(FontWeight::SEMIBOLD); }
            let entity=cx.entity();
            group=group.child(tab_btn.role(Role::Tab).h(px(24.0)).text_sm()
                .navigation(index,params::ALL.len(),move|index,window,cx| {if let Some(param)=params::ALL.get(index) {
                    entity.update(cx,|ed,cx|select_parameter_lane(ed,*param,window,cx));
                }})
                .on_click(cx.listener(move|ed,_,window,cx|select_parameter_lane(ed,param,window,cx))));
        }
        tabs = tabs.child(group);
        tabs.child(div().flex_1()).child(
            crate::ui_controls::flat_icon_command_button(self, Cmd::ToggleThreeLanes, "plus", "添加泳道", "add-lane", false, cx)
                .h(px(28.0)).flex_none())
    }

    fn curve_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.theme;
        crate::proto_scroll::viewport("curve-toolbar-scroll", "曲线工具栏", "horizontal")
            .h(px(40.0))
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .items_center()
            .whitespace_nowrap()
            .gap(px(5.0))
            .px(px(8.0))
            .bg(th.surface)
            .border_b_1()
            .border_color(th.shade)
            // Lane tools: one toggle group.
            .child(div().id("lane-tool-group").flex().flex_none().items_center().p(px(2.0)).gap(px(2.0)).rounded(px(8.0)).bg(th.control)
                .children([
                    (Cmd::SetLanePencil, LaneTool::Pencil, Some("pencil"), "自由绘制", "curve-Pencil"),
                    (Cmd::SetLaneLine, LaneTool::Line, Some("spline"), "直线", "curve-Line"),
                    (Cmd::SetLaneEraser, LaneTool::Eraser, Some("eraser"), "擦除", "curve-Eraser"),
                    (Cmd::SetLaneSelect, LaneTool::Select, Some("mouse-pointer-2"), "框选点", "curve-Select"),
                ].into_iter().map(|(cmd, tool, ic, label, key)| {
                    let on = self.lane_tool == tool;
                    crate::ui_controls::flat_command_button(self, cmd, Arg::None, "", key, on, cx)
                        .aria_label(label).h(px(24.0)).px(px(8.0)).text_sm()
                        .children(ic.map(|ic| svg().path(icon(ic)).size(px(14.0)).mr(px(5.0)).text_color(th.text)))
                        .child(label)
                })))
            .child(crate::ui_controls::flat_icon_command_button(self, Cmd::OpenTransformPoints, "sliders-horizontal", "点集变换…", "transform-points", false, cx).h(px(28.0)).flex_none())
            .child(crate::ui_controls::flat_command_button(self, Cmd::CopyCurve, Arg::None, "复制曲线", "copy-curve", false, cx).h(px(28.0)).flex_none().px(px(9.0)).text_sm())
            .child(crate::ui_controls::flat_command_button(self, Cmd::PasteCurve, Arg::None, "粘贴曲线", "paste-curve", false, cx).h(px(28.0)).flex_none().px(px(9.0)).text_sm())
            .when(self.param == Param::Pit, |d| {
                d.child(
                    generated_select_button(self, SelectId::PitchRangeCents, Arg::None, "pitch-range-select", cx)
                        .h(px(BAR_BTN_H))
                        .flex_none()
                        .w(px(104.0)),
                )
            })
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_xs()
                    .text_color(th.text_secondary)
                    .child(support_text(params::support(self.param, self.route(), self.scheduler.capabilities.as_ref()))),
            )
    }

    pub fn show_bottom(&mut self, bottom: Bottom, window: &mut Window, cx: &mut Context<Self>) {
        self.end_lyric(true, None, window, cx);
        if self.lyric.is_some() {
            return;
        }
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        self.bottom = if self.bottom == bottom { Bottom::Hidden } else { bottom };
        match self.bottom {
            Bottom::Table => window.focus(&self.focus_table, cx),
            Bottom::Editor => window.focus(&self.focus_roll, cx),
            _ => window.focus(&self.focus_arrange, cx),
        }
        cx.notify();
    }
}

fn output_text(ed: &Editor) -> String {
    match &ed.audio_info.kind {
        OutputKind::Device { name, channels, format } => {
            let buffer = ed.audio_info.buffer_frames.map_or_else(|| "系统决定".into(), |frames| format!("{frames} 帧"));
            let underruns = ed.audio.shared.underruns.load(std::sync::atomic::Ordering::Relaxed);
            format!(
                "{name} · {} Hz · {channels} 声道 · {format} · 缓冲 {buffer} · 欠载 {underruns}",
                ed.audio_info.device_sample_rate
            )
        }
        OutputKind::SilentClock { reason } => format!("静默时钟（不发声）：{reason}"),
    }
}

impl Editor {
    fn arrange_pane_height(&self) -> f32 {
        let expanded_count = self.project().tracks.iter()
            .filter(|track| self.automation_ui.expanded_tracks.contains(&track.id)).count();
        let rows = expanded_count + usize::from(self.automation_ui.show_master);
        self.layout.arrange_height + 34.0 + rows.min(2) as f32 * AUTO_LANE_H as f32
    }

    fn arrangement_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let th = self.theme;
        let entity = cx.entity();
        let locale = i18n::current(cx);
        let arrangement_semantics = Surface::Arrangement.semantics(locale);
        self.playback.clear_arrangement_anchors();

        let track_height = self.project().tracks.len() as f64 * self.arrange_track_height;
        let expanded_tracks: Vec<_> = self
            .project()
            .tracks
            .iter()
            .filter(|t| self.automation_ui.expanded_tracks.contains(&t.id))
            .map(|t| (t.id.clone(), self.automation_ui.track_target(&t.id)))
            .collect();
        let auto_count = expanded_tracks.len() + usize::from(self.automation_ui.show_master);
        let content_h = track_height + auto_count as f64 * AUTO_LANE_H;

        let mut auto_rows: Vec<AnyElement> = Vec::with_capacity(auto_count);
        for (tid, target) in expanded_tracks {
            auto_rows.push(
                div()
                    .h(px(AUTO_LANE_H as f32))
                    .flex_none()
                    .flex()
                    .child(automation_header(self, Some(tid), target.clone(), cx))
                    .child(div().flex_1().min_w_0().relative().child(automation_curve_canvas(entity.clone(), target)))
                    .into_any_element(),
            );
        }
        if self.automation_ui.show_master {
            let m_target = self.automation_ui.master_target();
            auto_rows.push(
                div()
                    .h(px(AUTO_LANE_H as f32))
                    .flex_none()
                    .flex()
                    .child(automation_header(self, None, m_target.clone(), cx))
                    .child(div().flex_1().min_w_0().relative().child(automation_curve_canvas(entity.clone(), m_target)))
                    .into_any_element(),
            );
        }

        div()
            .id("arrangement")
            .role(arrangement_semantics.role)
            .aria_label(arrangement_semantics.label)
            .aria_description(arrangement_semantics.description)
            .key_context("Arrange")
            .track_focus(&self.focus_arrange)
            
            .on_action(cx.listener(|ed, _: &CopyNotes, window, cx| { if let Err(error)=ed.run_audio_ui(crate::editor::integrations::audio_edit_ui::AudioUiAction::Copy,window,cx){ed.notice(error);cx.notify();} }))
            .on_action(cx.listener(|ed, _: &CutNotes, window, cx| { if let Err(error)=ed.run_audio_ui(crate::editor::integrations::audio_edit_ui::AudioUiAction::Cut,window,cx){ed.notice(error);cx.notify();} }))
            .on_action(cx.listener(|ed, _: &PasteNotes, window, cx| { if let Err(error)=ed.run_audio_ui(crate::editor::integrations::audio_edit_ui::AudioUiAction::Paste,window,cx){ed.notice(error);cx.notify();} }))
            .on_action(cx.listener(|ed, _: &DuplicateSelection, window, cx| { if let Err(error)=ed.run_audio_ui(crate::editor::integrations::audio_edit_ui::AudioUiAction::Duplicate,window,cx){ed.notice(error);cx.notify();} }))
            .on_action(cx.listener(|ed, _: &SelectAll, window, cx| { if let Err(error)=ed.run_audio_ui(crate::editor::integrations::audio_edit_ui::AudioUiAction::SelectAll,window,cx){ed.notice(error);cx.notify();} }))
            .flex()
            .flex_col()
            .min_h_0()
            .size_full()
            // Pane header (AdwToolbarView top bar): its bottom line meets the
            // inspector's and the dock's.
            // Flat buttons show no edge at rest, so their icons, not their
            // boxes, line up with the toolbars below; the boxes hang into the
            // margin.
            .child(pane_header(th).pl(px(1.0))
                .child(crate::ui_controls::flat_icon_command_button(self, Cmd::AddTrack, "plus", "人声轨", "add-track", false, cx).h(px(32.0)))
                .child(crate::proto_controls::action("add-instrument-track", "添加乐器轨").h(px(32.0)).px(px(9.0)).flex().items_center().gap(px(6.0))
                    .rounded(px(6.0)).text_sm().cursor_pointer().hover(move |s| s.bg(th.hover)).active(move |s| s.bg(th.control_pressed))
                    .child(svg().path(icon("instrument")).size(px(15.0)).flex_none().text_color(th.text)).child("乐器轨")
                    .on_click(cx.listener(|ed, _, _, cx| ed.add_instrument_track(cx))))
                .child(crate::ui_controls::flat_icon_command_button(self, Cmd::FitArrange, "maximize-2", "显示全部", "fit-arrange", false, cx).h(px(32.0)))
                .child(div().flex_1())
                .child(div().text_sm().text_color(th.text_secondary).child(format!("{} 条轨道", self.project().tracks.len()))))
            .child(crate::editor::integrations::audio_edit_ui::toolbar(self,cx))
            .child(
                div()
                    .h(px(RULER_H))
                    .flex_none()
                    .flex()
                    .child(div().w(px(crate::arrange::HEADER_W)).bg(th.surface).border_b_1().border_r_1().border_color(th.shade))
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .child(crate::roll::ruler(entity.clone(), true))
                            .child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::ArrangeRuler))),
                    ),
            )
            .child(
                crate::proto_scroll::viewport("arrangement-scroll", "编曲区", "vertical")
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .min_h(px(content_h as f32))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(px(track_height as f32))
                                    .flex_none()
                                    .flex()
                                    .child(crate::arrange::track_headers(self, cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .relative()
                                            .child(crate::arrange::region_lanes(entity.clone()))
                                            .child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::Arrange))),
                                    ),
                            )
                            .children(auto_rows),
                    ),
            )
            .child(
                div()
                    .flex()
                    .child(div().w(px(crate::arrange::HEADER_W)).h(px(16.0)).bg(th.surface).border_t_1().border_color(th.shade))
                    .child(div().flex_1().min_w_0().child(horizontal_timeline_bar(entity.clone(), true, th, cx))),
            )
            .into_any_element()
    }

    fn bottom_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let th = self.theme;
        let entity = cx.entity();
        let locale = i18n::current(cx);
        let roll_semantics = Surface::Roll.semantics(locale);
        let lanes_semantics = Surface::Lanes.semantics(locale);
        let mixer_semantics = Surface::Mixer.semantics(locale);
        self.playback.clear_bottom_anchors();
        let slot_count = self.integrations.lanes.len().max(1);
        // Lanes keep usable drawing heights inside one bounded vertical viewport.
        let each_lane_h = if slot_count == 1 { self.layout.lane_height } else { (self.layout.lane_height / slot_count as f32).max(72.0) };

        let bottom = match self.bottom {
            Bottom::Editor => Some(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(self.editor_header(cx))
                    // Optional decoupled Global BPM Tempo Strip
                    .when(self.layout.show_tempo_lane, |d| {
                        d.child(
                            div()
                                .h(px(46.0))
                                .flex_none()
                                .flex()
                                .border_b_1()
                                .border_color(th.shade)
                                .child(
                                    div()
                                        .w(px(KEYS_W))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(th.surface)
                                        .border_r_1()
                                        .border_color(th.shade)
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(th.accent)
                                        .child("BPM"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .relative()
                                        .child(crate::lanes::tempo_lane(entity.clone()))
                                        .child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::TempoLane))),
                                ),
                        )
                    })
                    // Piano Roll + Keyboard + Ruler
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(
                                div()
                                    .w(px(KEYS_W))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .child(div().h(px(RULER_H)).bg(th.surface).border_b_1().border_r_1().border_color(th.shade))
                                    .child(div().flex_1().child(crate::roll::keyboard(entity.clone()))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .h(px(RULER_H))
                                            .flex_none()
                                            .relative()
                                            .child(crate::roll::ruler(entity.clone(), false))
                                            .child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::EditorRuler))),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_h_0()
                                            .relative()
                                            .overflow_hidden()
                                            .child(
                                                div()
                                                    .id("piano-roll")
                                                    .role(roll_semantics.role)
                                                    .aria_label(roll_semantics.label)
                                                    .aria_description(roll_semantics.description)
                                                    .absolute()
                                                    .inset_0()
                                                    .key_context("PianoRoll")
                                                    .track_focus(&self.focus_roll)
                                                    .child(crate::proto_roll::view(self.compiled_roll.clone())),
                                            )
                                            .child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::Roll))),
                                    ),
                            ),
                    )
                    // Inline SVS Phoneme Timing Strip (1:1 aligned with Piano Roll!)
                    .when(self.layout.show_phoneme_lane, |d| {
                        d.child(
                            div()
                                .h(px(PHONEME_LANE_H))
                                .flex_none()
                                .flex()
                                .border_t_1()
                                .border_color(th.shade)
                                .child(
                                    div()
                                        .w(px(KEYS_W))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(th.surface)
                                        .border_r_1()
                                        .border_color(th.shade)
                                        .text_xs()
                                        .text_color(th.text_secondary)
                                        .child("音素"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .relative()
                                        .child(timeline_phoneme_lane(entity.clone()))
                                        .child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::PhonemeLane))),
                                ),
                        )
                    })
                    // Piano Roll Horizontal Scroll / Zoom Bar
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .child(div().w(px(KEYS_W)).h(px(16.0)).bg(th.surface).border_t_1().border_r_1().border_color(th.shade))
                            .child(div().flex_1().min_w_0().child(horizontal_timeline_bar(entity.clone(), false, th, cx))),
                    )
                    // Resizable Splitter between Roll and Parameter Lanes
                    .child(splitter_bar(entity.clone(), SplitterKind::LaneHeight, th))
                    .child(self.lane_tabs(cx))
                    .child(self.curve_toolbar(cx))
                    .child(crate::proto_scroll::viewport("parameter-lanes-scroll", "参数泳道", "vertical")
                        .reveal_item(self.integrations.active_panel, format!("{}:{}:{:?}:{:?}",
                            self.project().project_id, self.document_generation, self.integrations.lanes, self.param))
                        .h(px(self.layout.lane_height))
                        .max_h(gpui::relative(0.4))
                        .min_h_0()
                        .flex_none()
                        .flex()
                        .flex_col()
                        // Headers scroll the lane stack; canvases retain their
                        // shared timeline wheel/zoom handling.
                        .children((0..slot_count).map(|slot| {
                        let param = self.integrations.lanes.get(slot).copied().unwrap_or(self.param);
                        let active_slot = slot_count > 1 && self.integrations.active_panel == slot;
                        div()
                            .id(("parameter-lane", slot))
                            .role(lanes_semantics.role)
                            .aria_label(lanes_semantics.label)
                            .aria_description(lanes_semantics.description)
                            .h(px(each_lane_h))
                            .flex_none()
                            .flex()
                            .border_t_1()
                            .border_color(th.shade)
                            .child(
                                div()
                                    .w(px(KEYS_W))
                                    .flex_none()
                                    .p(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_between()
                                    .bg(if active_slot { mix(th.surface, th.accent, 0.16) } else { th.surface })
                                    // The last lane's label sits in the window's bottom-left corner.
                                    .when(slot + 1 == slot_count, |d| d.rounded_bl(crate::window_chrome::corner_radii(self.window_chrome).bottom_left))
                                    .border_r_1()
                                    .border_color(th.shade)
                                    .text_xs()
                                    .child(div().font_weight(FontWeight::SEMIBOLD).child(param.short()))
                                    .when(slot_count > 1, |d| d.child(crate::proto_controls::action(("remove-lane", slot), format!("移除 {} 泳道", param.short()))
                                        .size(px(22.0)).rounded(px(11.0)).flex().items_center().justify_center().hover(move |s| s.bg(th.hover)).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover)))
                                        .child(svg().path(icon("x")).size(px(12.0)).text_color(th.text_secondary))
                                        .on_click(cx.listener(move |ed, _, _, cx| ed.remove_lane(slot, cx)))))
                                    .child(div().text_size(px(9.0)).text_color(th.text_tertiary).child(match param {
                                        Param::Pit => format!("±{:.0}¢", self.pit_range_cents),
                                        Param::Dyn => "dB".into(),
                                        Param::Vibrato => "¢".into(),
                                        _ => "0-127".into(),
                                    })),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .relative()
                                    .child(crate::lanes::lane(self, entity.clone(), param, slot))
                                    .when(slot == 0, |d| d.child(div().absolute().inset_0().child(self.playback.anchor(crate::clock::Surface::Lane)))),
                            )
                    })))
                    .into_any_element(),
            ),
            Bottom::Mixer => Some(
                div()
                    .id("mixer")
                    .role(mixer_semantics.role)
                    .aria_label(mixer_semantics.label)
                    .aria_description(mixer_semantics.description)
                    .key_context("Mixer")
                    .flex_1()
                    .min_h_0()
                    .child(crate::mixer::mixer(self, &entity, cx))
                    .into_any_element(),
            ),
            Bottom::Table => Some(div().flex_1().min_h_0().child(crate::table::note_table(self, entity.clone(), window, cx)).into_any_element()),
            Bottom::Devices => Some(div().flex_1().min_h_0().child(crate::devices::device_view(self, &entity, window, cx)).into_any_element()),
            Bottom::Hidden => None,
        };

        div().size_full().min_w_0().min_h_0().flex().flex_col().children(bottom).into_any_element()
    }

    fn static_shell(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let th = self.theme;
        let entity = cx.entity();
        let a11y = window.is_a11y_active();
        let has_track = self.active_track().is_some();
        let docked = self.width_class.docks_panels();
        let column = |width| StyleRefinement::default().w(px(width)).h_full().flex_none();
        let fixed_arrangement = self.bottom != Bottom::Hidden;
        let arrange_height = px(self.arrange_pane_height());
        if !fixed_arrangement {
            self.playback.clear_bottom_anchors();
        }
        let center = div().flex_1().min_w_0().min_h_0().flex().flex_col()
            .child(
                div().min_h_0().w_full()
                    .when(fixed_arrangement, |d| d.h(arrange_height).flex_none())
                    .when(!fixed_arrangement, |d| d.flex_1())
                    .child(cached_view(&self.chrome.arrangement, StyleRefinement::default().size_full(), a11y)),
            )
            .when(fixed_arrangement, |d| d
                .child(splitter_bar(entity.clone(), SplitterKind::ArrangeHeight, th))
                .child(div().flex_1().min_w_0().min_h_0().child(cached_view(&self.static_view, StyleRefinement::default().size_full(), a11y))));
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(cached_view(&self.chrome.control_bar, StyleRefinement::default().w_full().h(px(crate::ui_controls::bar_height(self.touch_ui))).flex_none(), a11y))
            .children(self.recovered.as_ref().map(|text| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .py(px(5.0))
                    .bg(th.control)
                    .text_sm()
                    .child(div().flex_1().child(text.clone()))
                    .child(button("dismiss-recovery", "知道了", th).on_click(cx.listener(|ed, _, _, cx| {
                        ed.recovered = None;
                        cx.notify();
                    })))
            }))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(
                        // All playback anchors are laid out by the control bar
                        // and center above. Paint playback/lyrics before drawers,
                        // keeping drawers within the center and off the rails.
                        div().relative().flex_1().min_w_0().min_h_0().flex().child(center)
                            .child(crate::clock::playback_layer(entity.clone(), self.touch_ui))
                            .when(!docked && has_track && self.drawer.is_some(), |d| d
                                .child(div().id("drawer-scrim").absolute().inset_0().bg(gpui::rgba(0x0000_0066)).occlude()
                                    .on_click(cx.listener(|ed, _, _, cx| { ed.drawer = None; cx.notify(); })))
                                .child({
                                    div().id("side-drawer").absolute().top_0().bottom_0().right_0()
                                        .w(px(self.layout.right_dock_total_width())).max_w(gpui::relative(0.92)).flex().occlude()
                                        .border_l_1().border_color(th.shade)
                                        .shadow_lg()
                                        .child(cached_view(&self.chrome.property_dock, StyleRefinement::default().size_full(), a11y))
                                })),
                    )
                    .when(docked && self.layout.show_right_dock, |d| {
                        d.child(splitter_bar(entity, SplitterKind::RightDockWidth, th))
                            .when(has_track, |d| d.child(cached_view(&self.chrome.property_dock, column(self.layout.right_dock_total_width()), a11y)))
                    })

            )
            // No status bar, as in GNOME apps: transient messages are an
            // AdwToast at the bottom centre; project facts live in the title
            // and the system tab.
            .children(self.notices.back().map(|notice| {
                div().absolute().bottom(px(18.0)).left_0().right_0().flex().justify_center()
                    .child(div().id("toast").role(Role::Status).aria_label(notice.text.clone())
                        .max_w(px(560.0)).min_h(px(40.0)).px(px(18.0)).py(px(9.0)).rounded(px(20.0))
                        .flex().items_center().gap(px(12.0))
                        .bg(if th.dark { gpui::rgb(0x3c3c40) } else { gpui::rgb(0x2e2e32) })
                        .text_color(gpui::rgb(0xffffff)).text_sm().shadow_lg()
                        .child(div().flex_1().min_w_0().child(notice.text.clone()))
                        .child(crate::proto_controls::action("toast-dismiss", "关闭通知").size(px(24.0)).rounded(px(12.0))
                            .flex().items_center().justify_center().hover(|s| s.bg(gpui::rgba(0xffffff26)))
                            .child(svg().path(icon("x")).size(px(13.0)).text_color(gpui::rgb(0xffffff)))
                            .on_click(cx.listener(|ed, _, _, cx| { ed.notices.clear(); cx.notify(); }))))
            }))
            .into_any_element()
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme = Theme::resolve_appearance(window.appearance(), self.appearance_override, &self.prefs)
            .with_pride(self.theme.pride);
        crate::alert::remember_theme(self.theme);
        let chrome = crate::window_chrome::ChromeState::read(window);
        if chrome != self.window_chrome {
            self.window_chrome = chrome;
            cx.notify();
        }
        // Touch layout scales every rem-sized control (≈ +12 %) toward 44 px
        // touch targets; px-sized canvases keep their musical geometry.
        crate::proto_controls::set_popup_owner(self.menu.as_ref()
            .filter(|menu| menu.window == window.window_handle()).and_then(|menu| menu.anchor_owner()));
        let rem = px(crate::ui_controls::rem_size(self.touch_ui));
        if window.rem_size() != rem { window.set_rem_size(rem); }
        let class = WidthClass::of(window.viewport_size().width.as_f32());
        if class != self.width_class {
            // Drawers belong to narrow layouts; docked panels keep their prefs.
            self.width_class = class;
            self.drawer = None;
            cx.notify();
        }
        let th = self.theme;
        let entity = cx.entity();
        let static_content = self.static_shell(window, cx);
        let modal = modal_dialog_layer(self, &entity, cx);
        let text_prompt = crate::lyric_view::prompt_overlay(self, &entity, cx);
        let menu = self.menu.as_ref().filter(|menu| menu.window == window.window_handle())
            .map(|menu| crate::menu::context_menu_layer(self, menu, window, cx).into_any_element());
        let root = crate::window_chrome::track_title_drag(div().id("verdant"), cx)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(th.window)
            .text_color(th.text)
            .font(crate::theme::ui_font(self.ui_font.clone()))
            .key_context("Verdant")
            .on_action(cx.listener(|ed, _: &WorkflowMenu, w, cx| ed.workflow_menu(w, cx)))
            .on_action(cx.listener(|ed, action: &crate::shortcuts::WorkflowShortcut, w, cx| crate::shortcuts::dispatch(ed, action, w, cx)))
            .on_action(cx.listener(|ed, _: &crate::commands::ShowHistory, w, cx| ed.show_history_panel(w, cx)))
            .on_action(cx.listener(|ed, _: &NewProject, w, cx| ed.run_workflow(crate::editor::WorkflowAction::New, w, cx)))
            .on_action(cx.listener(|ed, _: &RecentProjects, w, cx| ed.run_workflow(crate::editor::WorkflowAction::Recent, w, cx)))
            .on_action(cx.listener(|ed, _: &RecoverProjects, w, cx| ed.run_workflow(crate::editor::WorkflowAction::Recovery, w, cx)))
            .on_action(cx.listener(|ed, _: &RevertProject, w, cx| ed.run_workflow(crate::editor::WorkflowAction::Revert, w, cx)))
            .on_action(cx.listener(|ed, _: &SplitNotes, w, cx| {
                let t = ed.tempo().tick_at(ed.playhead_seconds()).round() as i64;
                ed.run_workflow(crate::editor::WorkflowAction::Split(t), w, cx);
            }))
            .on_action(cx.listener(|ed, _: &JoinNotes, w, cx| ed.run_workflow(crate::editor::WorkflowAction::Join, w, cx)))
            .on_action(cx.listener(|ed, _: &QuantizeNotes, w, cx| ed.run_workflow(crate::editor::WorkflowAction::Quantize, w, cx)))
            .on_action(cx.listener(|ed, _: &MuteNotes, w, cx| ed.run_workflow(crate::editor::WorkflowAction::MuteNotes, w, cx)))
            .on_action(cx.listener(|ed, _: &BatchLyrics, w, cx| ed.run_workflow(crate::editor::WorkflowAction::BatchLyrics(false), w, cx)))
            .on_action(cx.listener(|ed, _: &RepeatSelection, w, cx| ed.run_workflow(crate::editor::WorkflowAction::RepeatSelection, w, cx)))
            .on_action(cx.listener(|ed, _: &ToggleLoop, w, cx| ed.run_workflow(crate::editor::WorkflowAction::ToggleLoop, w, cx)))
            .on_action(cx.listener(|ed, _: &ToggleMetronome, w, cx| ed.run_workflow(crate::editor::WorkflowAction::ToggleMetronome, w, cx)))
            .on_action(cx.listener(|ed, _: &OpenProject, w, cx| controls::dispatch(ed, Cmd::OpenProject, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &Save, w, cx| controls::dispatch(ed, Cmd::Save, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &SaveAs, w, cx| controls::dispatch(ed, Cmd::SaveAs, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &ExportProject, w, cx| controls::dispatch(ed, Cmd::ExportProject, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &ExportAudio, w, cx| controls::dispatch(ed, Cmd::ExportAudio, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &Undo, w, cx| controls::dispatch(ed, Cmd::Undo, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &Redo, w, cx| controls::dispatch(ed, Cmd::Redo, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &CutNotes, w, cx| controls::dispatch(ed, Cmd::Cut, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &CopyNotes, w, cx| controls::dispatch(ed, Cmd::Copy, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &PasteNotes, w, cx| controls::dispatch(ed, Cmd::Paste, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &DuplicateSelection, w, cx| controls::dispatch(ed, Cmd::DuplicateSelection, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &RenderNow, w, cx| controls::dispatch(ed, Cmd::RenderNow, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &NewTrack, w, cx| controls::dispatch(ed, Cmd::AddTrack, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &DeleteTrack, w, cx| controls::dispatch(ed, Cmd::DeleteActiveTrack, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &RestartWorker, w, cx| controls::dispatch(ed, Cmd::StartWorker, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &TogglePlay, w, cx| controls::dispatch(ed, Cmd::TogglePlay, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &Stop, w, cx| controls::dispatch(ed, Cmd::Stop, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &DeleteSelection, w, cx| controls::dispatch(ed, Cmd::DeleteSelection, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &SelectAll, w, cx| controls::dispatch(ed, Cmd::SelectAll, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &CancelRender, _, cx| ed.cancel_active(cx)))
            .on_action(cx.listener(|ed, _: &ToggleAutoRender, _, cx| {
                ed.auto_render = !ed.auto_render;
                ed.render_armed = ed.auto_render;
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &ShowShortcuts, _, cx| crate::shortcuts::open(cx)))
            .on_action(cx.listener(|_, _: &ShowDisclosures, _, cx| crate::disclosures::open(cx)))
            .on_action(cx.listener(|ed, _: &Quit, _, cx| {
                if ed.shutdown(cx) {
                    cx.quit();
                }
            }))
            .on_action(cx.listener(|ed, _: &TempoUp, _, cx| ed.nudge_tempo(1.0, cx)))
            .on_action(cx.listener(|ed, _: &TempoDown, _, cx| ed.nudge_tempo(-1.0, cx)))
            .on_action(cx.listener(|ed, _: &GoToStart, _, cx| {
                ed.seek_tick(0.0);
                cx.notify();
            }))
            .on_action(cx.listener(|ed, _: &BackBar, _, cx| seek_bar(ed, false, cx)))
            .on_action(cx.listener(|ed, _: &ForwardBar, _, cx| seek_bar(ed, true, cx)))
            .on_action(cx.listener(|ed, _: &ToggleInspector, _, cx| ed.toggle_properties_panel(cx)))
            .on_action(cx.listener(|ed, _: &ToggleEditor, w, cx| ed.show_bottom(Bottom::Editor, w, cx)))
            .on_action(cx.listener(|ed, _: &ToggleMixer, w, cx| ed.show_bottom(Bottom::Mixer, w, cx)))
            .on_action(cx.listener(|ed, _: &ToggleTable, w, cx| ed.show_bottom(Bottom::Table, w, cx)))
            .on_action(cx.listener(|ed, _: &ToolPointer, w, cx| controls::dispatch(ed, Cmd::SetToolPointer, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &ToolPencil, w, cx| controls::dispatch(ed, Cmd::SetToolPencil, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &ToolEraser, w, cx| controls::dispatch(ed, Cmd::SetToolEraser, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &ToolPitch, w, cx| controls::dispatch(ed, Cmd::SetToolPitch, Arg::None, w, cx)))
            .on_action(cx.listener(|ed, _: &MuteTrack, w, cx| controls::dispatch(ed, Cmd::ToggleMute, Arg::Track(ed.session.track.clone()), w, cx)))
            .on_action(cx.listener(|ed, _: &SoloTrack, w, cx| controls::dispatch(ed, Cmd::ToggleSolo, Arg::Track(ed.session.track.clone()), w, cx)))
            .on_action(cx.listener(|ed, _: &PrevTrack, _, cx| step_track(ed, -1, cx)))
            .on_action(cx.listener(|ed, _: &NextTrack, _, cx| step_track(ed, 1, cx)))
            .on_action(cx.listener(|ed, _: &SelectPrev, _, cx| step_note(ed, false, cx)))
            .on_action(cx.listener(|ed, _: &SelectNext, _, cx| step_note(ed, true, cx)))
            .on_action(cx.listener(|ed, _: &EditLyric, w, cx| {
                if let Some(id) = ed.session.selection.iter().next().cloned() {
                    ed.begin_lyric(id, w, cx);
                }
            }))
            .on_action({
                let editor = cx.weak_entity();
                move |_: &Escape, w: &mut Window, cx: &mut gpui::App| {
                    // The application keymap may resolve Escape before the
                    // prototype's key.down handler. Cancel its live lease before
                    // borrowing Editor: the authored cancel drains back into it.
                    let contact = editor.upgrade().and_then(|entity| {
                        let ed = entity.read(cx);
                        if ed.menu.is_some() || ed.modal_dialog.is_some() { return None; }
                        ed.proto_pointer_contact.as_ref().and_then(std::rc::Weak::upgrade)
                    });
                    if contact.is_some_and(|contact| contact.cancel_in_window(w, cx)) { return; }
                    let _ = editor.update(cx, |ed, cx| {
                        if ed.menu.is_some() {
                            ed.close_menu(w, cx);
                        } else if ed.modal_dialog.is_some() {
                            ed.close_modal_dialog(false, w, cx);
                        } else if ed.session.gesture.is_some()
                            || ed.lane_stroke.is_some()
                            || ed.mixer_drag.is_some()
                            || ed.master_drag.is_some()
                            || ed.note_creation.is_some()
                            || ed.has_control_drag(cx)
                            || crate::ruler_navigation::is_active(cx.entity_id(), cx)
                        {
                            ed.cancel_edits(verdant_session::CancelReason::Escape, w, cx);
                        } else {
                            ed.session.selection.clear();
                            cx.notify();
                        }
                    });
                }
            })
            .on_action(cx.listener(|ed, _: &NudgeLeft, w, cx| nudge(ed, -ed.session.snap.grid_ticks.max(1), 0, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeRight, w, cx| nudge(ed, ed.session.snap.grid_ticks.max(1), 0, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeUp, w, cx| nudge(ed, 0, 1, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeDown, w, cx| nudge(ed, 0, -1, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeLeftBar, w, cx| nudge(ed, ed.bar_nudge_delta(false), 0, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeRightBar, w, cx| nudge(ed, ed.bar_nudge_delta(true), 0, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeUpOctave, w, cx| nudge(ed, 0, 12, w, cx)))
            .on_action(cx.listener(|ed, _: &NudgeDownOctave, w, cx| nudge(ed, 0, -12, w, cx)))
            .on_action(cx.listener(|ed, _: &ZoomIn, window, cx| {
                crate::proto_editor_surface::zoom(ed, false, "zoom-in", window, cx);
            }))
            .on_action(cx.listener(|ed, _: &ZoomOut, window, cx| {
                crate::proto_editor_surface::zoom(ed, false, "zoom-out", window, cx);
            }))
            .on_action(cx.listener(|ed, _: &ToggleFollow, _, cx| crate::clock::toggle_follow(ed, cx)))
            .on_action(cx.listener(|ed, _: &ToggleF0, _, cx| {
                ed.show_f0 = !ed.show_f0;
                cx.notify();
            }))
            .on_action(cx.listener(|ed, _: &ToggleVibrato, _, cx| {
                let on = ed.selected_notes().first().is_some_and(|n| n.vibrato.is_some());
                ed.set_vibrato(if on { None } else { Some(Vibrato::DEFAULT) }, cx);
            }))
            .on_action(cx.listener(|ed, _: &VowelA, _, cx| ed.set_vowel("a", cx)))
            .on_action(cx.listener(|ed, _: &VowelI, _, cx| ed.set_vowel("i", cx)))
            .on_action(cx.listener(|ed, _: &VowelU, _, cx| ed.set_vowel("u", cx)))
            .on_action(cx.listener(|ed, _: &VowelE, _, cx| ed.set_vowel("e", cx)))
            .on_action(cx.listener(|ed, _: &VowelO, _, cx| ed.set_vowel("o", cx)))
            .on_action(cx.listener(|ed, _: &GridFiner, _, cx| {
                ed.session.snap.grid_ticks = (ed.session.snap.grid_ticks / 2).max((ed.project().ppq / 32).max(1));
                cx.notify();
            }))
            .on_action(cx.listener(|ed, _: &GridCoarser, _, cx| {
                ed.session.snap.grid_ticks = (ed.session.snap.grid_ticks * 2).min(ed.meter().bar_ticks_at(ed.tempo_event_tick()));
                cx.notify();
            }))
            .child(self.playback.driver.clone())
            .child(static_content)
            .children(modal)
            .children(menu)
            .children(text_prompt);
        crate::window_chrome::frame_window(root, chrome, th, window)
    }
}

fn seek_bar(ed: &mut Editor, forward: bool, cx: &mut Context<Editor>) {
    let tick = ed.tempo().tick_at(ed.playhead_seconds());
    let target = crate::clock::bar_navigation_tick(&ed.meter(), tick, forward);
    ed.seek_tick(target as f64);
    cx.notify();
}

fn step_track(ed: &mut Editor, delta: isize, cx: &mut Context<Editor>) {
    if ed.project().tracks.is_empty() {
        return;
    }
    let index = ed.track_index(&ed.session.track).unwrap_or(0) as isize;
    let next = (index + delta).clamp(0, ed.project().tracks.len() as isize - 1) as usize;
    if !ed.select_track(ed.project().tracks[next].id.clone(), cx) {
        cx.stop_propagation();
    }
}

fn step_note(ed: &mut Editor, forward: bool, cx: &mut Context<Editor>) {
    let project = ed.store().snapshot();
    ed.session.step_selection(&project, forward);
    cx.notify();
}

fn nudge(ed: &mut Editor, tick: i64, pitch: i32, window: &mut Window, cx: &mut Context<Editor>) {
    let project = ed.store().snapshot();
    let effects = ed.session.nudge_selection(&project, ed.revision(), tick, pitch);
    ed.apply_effects(effects, window, cx);
}

#[cfg(test)]
mod diagnostic_priority_tests {
    use super::prioritized_diagnostics;
    use verdant_backend::{Diagnostic, PhrasePlan, Route, Severity};
    use verdant_domain::{NoteId, TrackId};

    #[test]
    fn earlier_pitch_warnings_cannot_hide_later_blocking_notes() {
        let warning = || Diagnostic {
            code: "W_PITCH_UNVERIFIED", severity: Severity::Warning, note_id: None,
            message: "音高在接受范围内，但未经听评".into(), detail: String::new(),
        };
        let mut first = PhrasePlan {
            track: TrackId("low-octaves".into()), route: Route::VerdantVoice,
            phrases: Vec::new(), blocked: Vec::new(),
            diagnostics: (0..20).map(|_| warning()).collect(),
        };
        let c7 = NoteId("c7-note".into());
        first.diagnostics.push(Diagnostic {
            code: "E_PITCH_RANGE", severity: Severity::Error, note_id: Some(c7.clone()),
            message: "C7 超出音域".into(), detail: String::new(),
        });
        let c8 = NoteId("c8-note".into());
        let second = PhrasePlan {
            track: TrackId("high-octave".into()), route: Route::VerdantVoice,
            phrases: Vec::new(), blocked: Vec::new(),
            diagnostics: vec![Diagnostic {
                code: "E_PITCH_RANGE", severity: Severity::Error, note_id: Some(c8.clone()),
                message: "C8 超出音域".into(), detail: String::new(),
            }],
        };
        let plans = [first, second];
        let (visible, omitted) = prioritized_diagnostics(&plans, 14);
        assert_eq!(visible.len(), 14);
        assert_eq!(omitted, 8);
        assert_eq!(visible[0].0, plans[0].track);
        assert_eq!(visible[0].1.note_id.as_ref(), Some(&c7));
        assert_eq!(visible[1].0, plans[1].track);
        assert_eq!(visible[1].1.note_id.as_ref(), Some(&c8));
        assert!(visible[2..].iter().all(|(_, d)| d.severity == Severity::Warning));
    }
}
