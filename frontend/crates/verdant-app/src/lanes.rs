//! Lanes share the roll's time transform. Pointer previews stay in UI state
//! and one release commits one domain transaction.
//! Drawing tools start at the unsnapped press position, even over authored
//! points. The point-selection tool owns existing-point dragging and marquee.
//!
//! The first parameter panel also owns the tempo-event lane: click to insert,
//! drag a point to edit its tick/BPM, double-click or right-click for exact BPM,
//! and right-click to delete. Tick zero remains anchored. Escape/touch cancel
//! discard previews; one release creates one undoable command batch.
//!
//! DYN/BRE/BRI/CLE/GEN/OPE/GWL headers affect the selected note regions. The
//! switch bypasses values without deleting point geometry; interpolation and
//! point deletion each commit one undo step. Tension is not an engine control.
use gpui::{App, Bounds, Context, CursorStyle, DispatchPhase, Entity, Hitbox, HitboxBehavior, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, BorderStyle, TouchDragEvent, TouchPhase, Window, canvas, fill, prelude::*, px,quad};
use verdant_backend::params::Param;
use verdant_session::PointerId;
use crate::editor::{Editor, LaneTool, MenuAction, Tool};
use crate::paint::{Texts, b, flush_texts, polyline};
use crate::theme::with_alpha;

pub const DYN_MIN_DB: f64 = -24.0;
pub const DYN_MAX_DB: f64 = 6.0;
pub const VIB_MAX_CENTS: f64 = 200.0;
pub const EXPRESSION_MAX: f64 = 127.0;
const PAD: f64 = 6.0;

// This panel owns its pointer preview independently of expression strokes.
// GPUI recreates canvases during notification, so the active gesture lives in
// app state, identified by editor, track, note and the immutable base revision.
#[derive(Default)]
struct PhonemePanelState {
    drag: Option<PhonemeBoundaryDrag>,
}
impl gpui::Global for PhonemePanelState {}

struct PhonemeBoundaryDrag {
    editor: gpui::EntityId,
    track: verdant_domain::TrackId,
    note: verdant_domain::NoteId,
    revision: verdant_domain::Revision,
    pointer: PointerId,
    boundary: usize,
    start_x: f64,
    initial_seconds: f64,
    position_seconds: f64,
    total_seconds: f64,
    lower_seconds: f64,
    upper_seconds: f64,
}

impl PhonemeBoundaryDrag {
    fn valid(&self) -> bool {
        self.position_seconds.is_finite()
            && self.position_seconds >= self.lower_seconds
            && self.position_seconds <= self.upper_seconds
    }
    fn matches(&self, editor: &Entity<Editor>, ed: &Editor) -> bool {
        self.editor == editor.entity_id() && self.track == ed.session.track
            && self.revision == ed.revision() && panel_note(ed).is_some_and(|note| note.id == self.note)
    }
}

fn panel_note(ed: &Editor) -> Option<&verdant_domain::Note> {
    let track = ed.active_track()?;
    ed.session.selection.iter().filter_map(|id| ed.index.position(id).map(|i| (i, id)))
        .min_by_key(|(i, _)| *i).and_then(|(_, id)| ed.index.note(track, id))
}

/// Inspector entry point. Text rows remain ordinary GPUI elements so the
/// resolved segment names and scheduled durations are available to readers.
pub fn phoneme_panel(ed: &Editor, editor: Entity<Editor>, cx: &App) -> impl IntoElement {
    let th = ed.theme;
    let mut panel = gpui::div().id("phoneme-timing-panel").flex().flex_col().gap(px(4.0))
        .mt(px(8.0)).pt(px(8.0)).border_t_1().border_color(th.shade)
        .child(gpui::div().text_sm().child("音素边界"))
        .child(gpui::div().h(px(54.0)).w_full().child(phoneme_strip(editor.clone())));
    let Some(note) = panel_note(ed) else {
        return panel.child(gpui::div().text_xs().text_color(th.text_secondary).child("选择音符以查看音素时序"));
    };
    let cached_budget = ed.note_phone_budget(note, &ed.tempo());
    let budget = match &*cached_budget {
        Ok(budget) => budget,
        Err(reason) => return panel.child(gpui::div().text_xs().text_color(th.text_secondary).child(reason.clone())),
    };
    let schedule = ed.scheduled_phoneme_timing(&note.id);
    let anchored = ed.project().render_preferences.phoneme_timing.is_some();
    panel = panel.child(gpui::div().text_xs().text_color(th.text_secondary).child(
        if budget.editable && budget.labels.len() > 1 { "拖动竖线调整相邻音素预算；松开提交一次编辑。Esc／右键取消。" }
        else { "音素时序只读：单音素、后端能力或千分比精度不允许边界编辑。" }));
    let drag = cx.try_global::<PhonemePanelState>().and_then(|state| state.drag.as_ref())
        .filter(|drag| drag.matches(&editor, ed) && drag.note == note.id);
    let mut preview = None;
    if let Some(drag) = drag.filter(|drag| drag.valid()) {
        if let Some(weights) = budget.weights_permille.as_ref() {
            let mut durations: Vec<_> = weights.iter().map(|&w| budget.total_seconds * w as f64 / 1000.0).collect();
            let left: u32 = weights[..drag.boundary - 1].iter().sum();
            let right: u32 = weights[..=drag.boundary].iter().sum();
            let end = (drag.position_seconds / budget.total_seconds * 1000.0).round() as u32;
            durations[drag.boundary - 1] = budget.total_seconds * (end - left) as f64 / 1000.0;
            durations[drag.boundary] = budget.total_seconds * (right - end) as f64 / 1000.0;
            preview = Some(durations);
        }
    }
    let durations = preview.as_deref().unwrap_or(&budget.durations_seconds);
    let mut rows = crate::proto_scroll::viewport("phoneme-duration-rows", "音素时长", "vertical").max_h(px(180.0))
        .flex().flex_col().gap(px(3.0));
    for (index, (label, duration)) in budget.labels.iter().zip(durations).enumerate() {
        let scheduled = schedule.as_ref().and_then(|s| s.durations.get(index)).copied()
            .or_else(|| (!anchored).then_some(budget.durations_seconds[index]));
        let text = match scheduled {
            Some(seconds) => format!("{} · 预算 {:.1} ms · 计划 {:.1} ms", label, duration * 1000.0, seconds * 1000.0),
            None => format!("{} · 预算 {:.1} ms · 计划待预检", label, duration * 1000.0),
        };
        rows = rows.child(gpui::div().text_xs().child(text));
    }
    panel = panel.child(rows);
    if let Some(schedule) = schedule {
        panel = panel.child(gpui::div().text_xs().text_color(th.text_secondary).child(format!(
            "计划辅音先行 {:.1} ms · 让出尾部 {:.1} ms", schedule.prefix_seconds * 1000.0, schedule.tail_trim_seconds * 1000.0)));
    }
    if anchored {
        panel = panel.child(gpui::div().text_xs().text_color(th.text_secondary)
            .child("上图编辑音符内预算；元音对拍的计划长度还取决于相邻音符，提交后重新预检。"));
    }
    if drag.is_some_and(|drag| !drag.valid()) {
        let ms = budget.minimum_segment_seconds * 1000.0;
        panel = panel.child(gpui::div().text_xs().text_color(th.error)
            .child(format!("相邻音素各须至少 {ms:.0} ms。该位置无法提交。")));
    }
    panel.child(gpui::div().text_xs().text_color(th.text_tertiary)
        .child("当前模型未提供独立预发声／重叠偏移、音量包络、逐音素力度或爆破控制。"))
}

fn phoneme_strip(editor: Entity<Editor>) -> impl IntoElement {
    let draw = editor.clone(); let cancel = editor.clone();
    crate::proto_editor_surface::surface(editor, "phoneme-boundaries", "音素时长边界", 0.0,
        move |input| native_phoneme_strip(draw.clone(), input).into_any_element(),
        move |_, window, cx| {
            let owned = cx.try_global::<PhonemePanelState>().and_then(|state| state.drag.as_ref())
                .is_some_and(|drag| drag.editor == cancel.entity_id());
            if owned { cx.global_mut::<PhonemePanelState>().drag = None;  cancel.update(cx, |_, cx| cx.notify()); }
        })
}

fn native_phoneme_strip(editor: Entity<Editor>, input: crate::proto_editor_surface::Input) -> impl IntoElement {
    canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
        if cx.try_global::<PhonemePanelState>().is_none() { cx.set_global(PhonemePanelState::default()); }
        let stale = cx.global::<PhonemePanelState>().drag.as_ref().is_some_and(|drag|
            drag.editor == editor.entity_id() && !drag.matches(&editor, editor.read(cx)));
        if stale {
            cx.global_mut::<PhonemePanelState>().drag = None;
            input.cancel(window, cx);
        }
        let mut text = Texts::new();
        {
            let ed = editor.read(cx);
            let th = ed.theme;
            window.paint_quad(fill(bounds, th.roll_black));
            if let Some(note) = panel_note(ed) {
              let cached_budget = ed.note_phone_budget(note, &ed.tempo());
              if let Ok(budget) = &*cached_budget {
                let width = (bounds.size.width.as_f32() as f64 - 2.0 * PAD).max(1.0);
                let drag = cx.global::<PhonemePanelState>().drag.as_ref().filter(|drag|
                    drag.matches(&editor, ed) && drag.note == note.id);
                let mut seconds = 0.0;
                for (index, &duration) in budget.durations_seconds.iter().enumerate() {
                    let start = PAD + seconds / budget.total_seconds * width;
                    seconds += duration;
                    let end = PAD + seconds / budget.total_seconds * width;
                    window.paint_quad(fill(b(start, 17.0, (end - start - 1.0).max(0.0), 18.0, bounds.origin), th.selection));
                    let sym = crate::phoneme_lane::short_phoneme_label(&budget.labels[index]);
                    text.push((sym.to_string(), gpui::point(bounds.origin.x + px((start + 3.0) as f32), bounds.origin.y + px(19.0)), 10.0, th.text));
                    if index + 1 < budget.labels.len() {
                        let x = drag.filter(|drag| drag.boundary == index + 1)
                            .map_or(end, |drag| PAD + drag.position_seconds / budget.total_seconds * width)
                            .clamp(PAD, PAD + width);
                        let color = if drag.is_some_and(|drag| drag.boundary == index + 1 && !drag.valid()) { th.error }
                            else if budget.editable { th.accent } else { th.text_tertiary };
                        window.paint_quad(fill(b(x - 1.0, 8.0, 2.0, 34.0, bounds.origin), color));
                        window.paint_quad(fill(b(x - 4.0, 7.0, 8.0, 5.0, bounds.origin), color));
                    }
                }
                text.push(("0 ms".into(), gpui::point(bounds.origin.x + px(PAD as f32), bounds.origin.y + px(39.0)), 9.0, th.text_tertiary));
                text.push((format!("{:.1} ms", budget.total_seconds * 1000.0), gpui::point(bounds.origin.x + px((width - 46.0).max(PAD) as f32), bounds.origin.y + px(39.0)), 9.0, th.text_tertiary));
              }
            }
        }
        flush_texts(window, cx, bounds, text);
        install_phoneme_input(&editor, bounds, &hitbox, &input, window);
    }).size_full()
}

fn begin_phoneme_drag(editor: &Entity<Editor>, pointer: PointerId, x: f64, bounds: Bounds<Pixels>,
    window: &mut Window, cx: &mut App) -> bool {
    if cx.global::<PhonemePanelState>().drag.is_some() { return false; }
    let (track, note, revision, boundary, position, total_seconds, lower_seconds, upper_seconds) = {
        let ed = editor.read(cx);
        let Some(note) = panel_note(ed) else { return false };
        let cached_budget = ed.note_phone_budget(note, &ed.tempo());
        let Ok(budget) = &*cached_budget else { return false };
        if !budget.editable { return false; }
        let Some(weights) = budget.weights_permille.as_ref() else { return false };
        let width = (bounds.size.width.as_f32() as f64 - 2.0 * PAD).max(1.0);
        let mut sum = 0u32;
        let mut nearest = None;
        for (index, &weight) in weights.iter().take(weights.len().saturating_sub(1)).enumerate() {
            sum += weight;
            let position = budget.total_seconds * sum as f64 / 1000.0;
            let distance = (x - (PAD + sum as f64 / 1000.0 * width)).abs();
            if distance <= 7.0 && nearest.as_ref().is_none_or(|&(_, _, best)| distance < best) {
                nearest = Some((index + 1, position, distance));
            }
        }
        let Some((boundary, position, _)) = nearest else { return false };
        let left: u32 = weights[..boundary - 1].iter().sum();
        let right: u32 = weights[..=boundary].iter().sum();
        let minimum = budget.minimum_permille();
        let total_seconds = budget.total_seconds;
        (ed.session.track.clone(), note.id.clone(), ed.revision(), boundary, position, total_seconds,
            total_seconds * (left + minimum) as f64 / 1000.0,
            total_seconds * (right - minimum) as f64 / 1000.0)
    };
    let drag = PhonemeBoundaryDrag {
        editor: editor.entity_id(), track, note, revision, pointer, boundary, start_x: x,
        initial_seconds: position, position_seconds: position, total_seconds, lower_seconds, upper_seconds,
    };
    cx.global_mut::<PhonemePanelState>().drag = Some(drag);
    editor.update(cx, |_, cx| cx.notify());
    true
}

fn move_phoneme_drag(editor: &Entity<Editor>, pointer: PointerId, x: f64, bounds: Bounds<Pixels>, cx: &mut App) {
    let matches = cx.global::<PhonemePanelState>().drag.as_ref().is_some_and(|drag|
        drag.pointer == pointer && drag.matches(editor, editor.read(cx)));
    if !matches { return; }
    let width = (bounds.size.width.as_f32() as f64 - 2.0 * PAD).max(1.0);
    if let Some(drag) = cx.global_mut::<PhonemePanelState>().drag.as_mut() {
        drag.position_seconds = drag.initial_seconds + (x - drag.start_x) / width * drag.total_seconds;
    }
    editor.update(cx, |_, cx| cx.notify());
}

fn end_phoneme_drag(editor: &Entity<Editor>, pointer: PointerId, cancel: bool, window: &mut Window, cx: &mut App) {
    if !cx.global::<PhonemePanelState>().drag.as_ref().is_some_and(|drag|
        drag.editor == editor.entity_id() && drag.pointer == pointer) { return; }
    let Some(drag) = cx.global_mut::<PhonemePanelState>().drag.take() else { return };
    
    editor.update(cx, |ed, cx| {
        
        if cancel { cx.notify(); return; }
        if !drag.matches(editor, ed) { ed.notice("音符或工程已改变，已取消音素拖动"); cx.notify(); return; }
        if (drag.position_seconds - drag.initial_seconds).abs() < 1e-12 { cx.notify(); return; }
        let result = ed.scheduler.capabilities.as_ref().ok_or_else(|| "后端能力声明已失效".to_string())
            .and_then(|caps| verdant_backend::phoneme_timing::drag_boundary(ed.project(), &drag.track,
                &drag.note, caps, drag.boundary, drag.position_seconds));
        match result {
            Ok(batch) => { ed.commit(drag.revision, batch, cx); }
            Err(reason) => { ed.notice(reason); cx.notify(); }
        }
    });
}

fn install_phoneme_input(editor: &Entity<Editor>, bounds: Bounds<Pixels>, hitbox: &Hitbox,
    input: &crate::proto_editor_surface::Input, window: &mut Window) {
    if hitbox.is_hovered(window) { window.set_cursor_style(CursorStyle::ResizeLeftRight, hitbox); }
    let e = editor.clone();
    crate::proto_editor_surface::install_pointer_input(input,bounds,hitbox,window,move |event,window,cx| {
        use crate::proto_editor_surface::PointerPhase;
        match event.phase {
            PointerPhase::Begin => begin_phoneme_drag(&e,event.pointer,event.x,bounds,window,cx),
            PointerPhase::Sample => { move_phoneme_drag(&e,event.pointer,event.x,bounds,cx); true }
            PointerPhase::Finish => { end_phoneme_drag(&e,event.pointer,false,window,cx); true }
            PointerPhase::Context => { end_phoneme_drag(&e,event.pointer,true,window,cx); true }
        }
    });
}

pub fn value_at(param: Param, y: f64, height: f64, pit_range: f64) -> f64 {
    let f = 1.0 - ((y - PAD) / (height - 2.0 * PAD).max(1.0)).clamp(0.0, 1.0);
    match param {
        Param::Pit => ((f * 2.0 - 1.0) * pit_range).round(),
        Param::Dyn => ((DYN_MIN_DB + f * (DYN_MAX_DB - DYN_MIN_DB)) * 10.0).round() / 10.0,
        Param::Vibrato => (f * VIB_MAX_CENTS).round(),
        _ => (f * EXPRESSION_MAX).round(),
    }
}
pub fn y_of(param: Param, value: f64, height: f64, pit_range: f64) -> f64 {
    let fraction = match param {
        Param::Pit => (value / pit_range + 1.0) / 2.0,
        Param::Dyn => (value - DYN_MIN_DB) / (DYN_MAX_DB - DYN_MIN_DB),
        Param::Vibrato => value / VIB_MAX_CENTS,
        _ => value / EXPRESSION_MAX,
    };
    PAD + (1.0 - fraction.clamp(0.0, 1.0)) * (height - 2.0 * PAD).max(1.0)
}

pub fn lane(ed: &Editor, editor: Entity<Editor>, param: Param, slot: usize) -> impl IntoElement {
    gpui::div().size_full().flex().flex_col()
        .child(gpui::div().flex_1().min_h_0().child(parameter_lane(ed, editor, param, slot)))
}

fn parameter_lane(ed: &Editor, editor: Entity<Editor>, param: Param, slot: usize) -> impl IntoElement {
    let surface_editor = editor.clone();
    let cancel = editor.clone();
    let surface = crate::proto_editor_surface::surface(editor.clone(), format!("parameter-{slot}-{param:?}"),
        format!("{param:?} 参数曲线"), 0.0,
        move |input| {
          let surface_editor = surface_editor.clone();
          canvas(|bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
        let mut text = Texts::new();
        paint_lane(&surface_editor,bounds,param,window,cx,&mut text);
        flush_texts(window, cx, bounds, text);
        install_input(&surface_editor,bounds,&hitbox,param,slot,&input,window,cx);
        crate::roll::install_roll_scroll(&surface_editor, bounds, &hitbox, crate::roll::ScrollSurface::Timeline,&input,window);
          }).size_full().into_any_element()
        },
        move |_,window,cx| cancel.update(cx, |ed,cx| {
            if ed.integrations.active_panel == slot && ed.lane_stroke.as_ref().is_some_and(|s| !s.roll && s.param == param) {
                ed.lane_stroke = None;  cx.notify();
            }
        }));
    if param == Param::Dyn || (param.is_expression() && param != Param::Portamento) {
        gpui::div().flex().flex_col().size_full()
            .child(region_controls(ed, editor, param, slot))
            .child(gpui::div().flex_1().min_h_0().child(surface)).into_any_element()
    } else { surface.into_any_element() }
}

fn region_controls(ed: &Editor, editor: Entity<Editor>, param: Param, slot: usize) -> impl IntoElement {
    use verdant_domain::editing::Interpolation;
    let selected = ed.selected_notes();
    let any_disabled = !selected.is_empty() && selected.iter().all(|n| ed.region_lane_disabled(n, param));
    let current_interp = selected.first().map(|n| n.curves.interpolation(param.short()));
    let (th, enabled) = (ed.theme, !any_disabled);
    let button = move |key: &str, label: &str, active: bool| crate::proto_controls::action(
        format!("region-{}-{slot}-{key}", param.short()), format!("{}：{label}", param.name()))
        .authority(crate::proto_controls::editor_authority(ed))
        .aria_selected(active)
        .cursor_pointer()
        .h(px(20.0))
        .px(px(6.0))
        .rounded(px(4.0))
        .flex()
        .items_center()
        .bg(if active { th.accent } else { th.control })
        .border_1()
        .border_color(if active { th.accent } else { th.control })
        .text_xs()
        .text_color(if active { th.accent_text } else { th.text })
        .child(label.to_owned());
    let toggle_editor = editor.clone();
    let delete_editor = editor.clone();
    gpui::div().h(px(26.0)).px(px(8.0)).flex_none().flex().items_center().gap(px(4.0)).bg(th.surface).border_b_1().border_color(th.shade)
        .child(gpui::div().text_xs().text_color(th.text_secondary).child("所选音符区域："))
        .child(button("enable", if enabled { "已启用" } else { "已旁路" }, enabled).on_click(move |_, window, cx| {
            toggle_editor.update(cx, |ed, cx| {
                ed.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
                if ed.param != param { ed.point_selection.clear(); }
                ed.param = param; ed.integrations.active_panel = slot;
                ed.toggle_region_lane(param, cx);
            });
        }))
        .children([(Interpolation::Linear, "线性"), (Interpolation::Hold, "阶梯"),
            (Interpolation::Cosine, "余弦"), (Interpolation::Cubic, "单调")].into_iter().map(move |(mode, label)| {
                let editor = editor.clone();
                button(label, label, current_interp == Some(mode)).on_click(move |_, window, cx| {
                    editor.update(cx, |ed, cx| {
                        ed.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
                        if ed.param != param { ed.point_selection.clear(); }
                        ed.param = param; ed.integrations.active_panel = slot;
                        ed.interpolate_region_lane(param, mode, cx);
                    });
                })
            }))
        .child(button("delete", "删点 / 清空", false).on_click(move |_, window, cx| {
            delete_editor.update(cx, |ed, cx| {
                ed.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
                if ed.param != param { ed.point_selection.clear(); }
                ed.param = param; ed.integrations.active_panel = slot;
                ed.delete_region_lane_points(param, cx);
            });
        }))
}

const TEMPO_TOP: f64 = 20.0;
type TempoPreview = std::rc::Rc<std::cell::RefCell<Option<TempoDrag>>>;

struct TempoDrag {
    base: verdant_domain::Revision,
    project_id: String,
    pointer: PointerId,
    source_tick: Option<i64>,
    tick: i64,
    micro_bpm: u64,
    start_tick: i64,
    start_bpm: u64,
    origin: (f64, f64),
    range: (f64, f64),
    moved: bool,
}

fn tempo_range(points: &[verdant_domain::model::TempoPoint]) -> (f64, f64) {
    let (low, high) = points.iter().fold((f64::INFINITY, 0.0_f64), |(low, high), point| {
        let bpm = point.micro_bpm as f64 / 1e6;
        (low.min(bpm), high.max(bpm))
    });
    ((low - 20.0).max(0.000001), (high + 20.0).min(u64::MAX as f64 / 1e6))
}

fn tempo_y(micro_bpm: u64, height: f64, range: (f64, f64)) -> f64 {
    let fraction = ((micro_bpm as f64 / 1e6 - range.0) / (range.1 - range.0)).clamp(0.0, 1.0);
    TEMPO_TOP + (1.0 - fraction) * (height - TEMPO_TOP - PAD).max(1.0)
}

fn tempo_value(y: f64, height: f64, range: (f64, f64)) -> u64 {
    let fraction = 1.0 - ((y - TEMPO_TOP) / (height - TEMPO_TOP - PAD).max(1.0)).clamp(0.0, 1.0);
    (((range.0 + fraction * (range.1 - range.0)) * 10.0).round() * 100_000.0).max(1.0) as u64
}

fn tempo_hit(ed: &Editor, x: f64, y: f64, height: f64, range: (f64, f64)) -> Option<i64> {
    ed.project().tempo_map.iter().find(|point| {
        (ed.session.viewport.tick_to_x(point.tick as f64) - x).abs() <= 7.0
            && (tempo_y(point.micro_bpm, height, range) - y).abs() <= 7.0
    }).map(|point| point.tick)
}

/// Merge the pending point into the sorted map without copying the project or
/// mutating document facts during pointer motion.
fn tempo_points<'a>(points: &'a [verdant_domain::model::TempoPoint], drag: Option<&'a TempoDrag>) -> impl Iterator<Item = (i64, u64)> + 'a {
    let split = drag.map_or(points.len(), |drag| points.partition_point(|point| point.tick < drag.tick));
    let source = drag.and_then(|drag| drag.source_tick);
    let unchanged = move |point: &'a verdant_domain::model::TempoPoint| {
        (Some(point.tick) != source).then_some((point.tick, point.micro_bpm))
    };
    points[..split].iter().filter_map(unchanged)
        .chain(drag.map(|drag| (drag.tick, drag.micro_bpm)))
        .chain(points[split..].iter().filter_map(unchanged))
}

pub fn tempo_lane(editor: Entity<Editor>) -> impl IntoElement {
    let held: std::rc::Rc<std::cell::RefCell<Option<TempoPreview>>> = Default::default();
    let draw_state = held.clone(); let draw = editor.clone(); let cancel = editor.clone();
    crate::proto_editor_surface::surface(editor, "tempo-points", "速度曲线", 0.0,
        move |input| native_tempo_lane(draw.clone(),input,draw_state.clone()).into_any_element(),
        move |_,window,cx| {
            if let Some(preview) = held.borrow().as_ref() {
                if preview.borrow_mut().take().is_some() {  cancel.update(cx,|_,cx|cx.notify()); }
            }
        })
}

fn native_tempo_lane(editor: Entity<Editor>, input: crate::proto_editor_surface::Input,
    held: std::rc::Rc<std::cell::RefCell<Option<TempoPreview>>>) -> impl IntoElement {
    let prepaint_editor = editor.clone();
    let prepaint_input = input.clone();
    canvas(move |bounds, window, cx| {
        let state = window.use_keyed_state("tempo-event-preview", cx, |_, _| TempoPreview::default());
        let preview = state.read(cx).clone();
        *held.borrow_mut() = Some(preview.clone());
        let ed = prepaint_editor.read(cx);
        let stale = preview.borrow().as_ref().is_some_and(|drag| {
            drag.base != ed.revision() || drag.project_id != ed.project().project_id || !window.is_window_active()
        });
        if stale { preview.borrow_mut().take(); prepaint_input.cancel(window,cx); }
        (window.insert_hitbox(bounds, HitboxBehavior::Normal), preview)
    }, move |bounds, (hitbox, preview), window, cx| {
        let mut text = Texts::new();
        paint_tempo(&editor, bounds, preview.borrow().as_ref(), window, cx, &mut text);
        flush_texts(window, cx, bounds, text);
        install_tempo_input(&editor, bounds, &hitbox, &preview, &input, window);
        crate::roll::install_roll_scroll(&editor, bounds, &hitbox, crate::roll::ScrollSurface::Timeline,&input,window);
    }).size_full()
}

fn paint_tempo(editor: &Entity<Editor>, bounds: Bounds<Pixels>, drag: Option<&TempoDrag>, window: &mut Window, cx: &App, text: &mut Texts) {
    let ed = editor.read(cx);
    let th = ed.theme;
    let (width, height) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
    let range = drag.map_or_else(|| tempo_range(&ed.project().tempo_map), |drag| drag.range);
    let origin = bounds.origin;
    window.paint_quad(fill(bounds, th.surface));
    window.paint_quad(fill(b(0.0, height - 1.0, width, 1.0, origin), th.shade));
    text.push(("速度 BPM · 拖动移动 / 调速 · Shift 微调 · Alt 不吸附 · 双击输入 · 右键增删".into(),
        gpui::point(origin.x + px(6.0), origin.y + px(2.0)), 10.0, th.text_tertiary));
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        let mut previous_x = 0.0;
        let mut events = tempo_points(&ed.project().tempo_map, drag).peekable();
        let mut previous_y = events.peek().map(|(_, bpm)| tempo_y(*bpm, height, range));
        let mut label_right = -1.0;
        for (tick, micro_bpm) in events {
            let x = ed.session.viewport.tick_to_x(tick as f64);
            let y = tempo_y(micro_bpm, height, range);
            if let Some(last_y) = previous_y {
                if x > 0.0 {
                    polyline(window, origin, [Some((previous_x, last_y)), Some((x.min(width), last_y))], 1.5, th.accent);
                    if x > width { previous_x = width; break; }
                    polyline(window, origin, [Some((x, last_y)), Some((x, y))], 1.5, th.accent);
                }
            }
            previous_x = x.max(0.0);
            previous_y = Some(y);
            if (0.0..=width).contains(&x) {
                let active = drag.is_some_and(|drag| drag.tick == tick);
                window.paint_quad(fill(b(x - 4.0, y - 4.0, 8.0, 8.0, origin), if active { th.text } else { th.accent }));
                if active || x >= label_right {
                    let bpm = micro_bpm as f64 / 1e6;
                    text.push((format!("{bpm:.2}"), gpui::point(origin.x + px((x + 6.0) as f32), origin.y + px((y - 7.0).max(TEMPO_TOP) as f32)), 10.0, th.text));
                    label_right = x + 64.0;
                }
            }
        }
        if let Some(y) = previous_y {
            if previous_x < width { polyline(window, origin, [Some((previous_x, y)), Some((width, y))], 1.5, th.accent); }
        }
    });
}

fn begin_tempo(ed: &mut Editor, preview: &TempoPreview, pointer: PointerId, x: f64, y: f64, height: f64, modifiers: gpui::Modifiers, window: &mut Window, cx: &mut Context<Editor>) -> bool {
    if preview.borrow().is_some() || ed.session.gesture.is_some() || ed.lane_stroke.is_some() || !ed.commit_pending_lyric(cx) { return false; }
    let range = tempo_range(&ed.project().tempo_map);
    let hit = tempo_hit(ed, x, y, height, range);
    let raw_tick = ed.session.viewport.x_to_tick(x);
    let tick = hit.unwrap_or_else(|| if modifiers.alt { raw_tick.round() as i64 } else { ed.session.snap.snap(raw_tick) }).max(0);
    let original = ed.project().tempo_map.iter().find(|point| point.tick == tick);
    let source_tick = original.map(|point| point.tick);
    let micro_bpm = if hit.is_some() { original.unwrap().micro_bpm } else { tempo_value(y, height, range) };
    *preview.borrow_mut() = Some(TempoDrag {
        base: ed.revision(), project_id: ed.project().project_id.clone(), pointer, source_tick,
        tick, micro_bpm, start_tick: tick, start_bpm: micro_bpm, origin: (x, y), range, moved: false,
    });
    
    cx.notify();
    true
}

fn move_tempo(ed: &mut Editor, preview: &TempoPreview, pointer: PointerId, x: f64, y: f64, height: f64, modifiers: gpui::Modifiers, cx: &mut Context<Editor>) {
    let mut preview = preview.borrow_mut();
    let Some(drag) = preview.as_mut().filter(|drag| drag.pointer == pointer) else { return };
    if drag.base != ed.revision() || drag.project_id != ed.project().project_id { preview.take(); cx.notify(); return; }
    let (dx, dy) = (x - drag.origin.0, y - drag.origin.1);
    if !drag.moved && dx.abs().max(dy.abs()) < 3.0 { return; }
    drag.moved = true;
    drag.tick = drag.start_tick;
    drag.micro_bpm = drag.start_bpm;
    if drag.source_tick != Some(0) && dx.abs() >= 3.0 {
        let raw_tick = drag.start_tick as f64 + dx / ed.session.viewport.px_per_tick;
        let tick = if modifiers.alt { raw_tick.round() as i64 } else { ed.session.snap.snap(raw_tick) };
        let points = &ed.project().tempo_map;
        let low = points.iter().rev().find(|point| point.tick < drag.start_tick).map_or(0, |point| point.tick.saturating_add(1));
        let high = points.iter().find(|point| point.tick > drag.start_tick).map_or(i64::MAX, |point| point.tick.saturating_sub(1));
        drag.tick = tick.clamp(low, high);
    }
    if dy.abs() >= 3.0 {
        let delta_bpm = -dy * (drag.range.1 - drag.range.0) / (height - TEMPO_TOP - PAD).max(1.0)
            * if modifiers.shift { 0.1 } else { 1.0 };
        drag.micro_bpm = (((drag.start_bpm as f64 / 1e6 + delta_bpm).max(0.000001) * 10.0).round() * 100_000.0).max(1.0) as u64;
    }
    cx.notify();
}

fn end_tempo(ed: &mut Editor, preview: &TempoPreview, pointer: PointerId, window: &mut Window, cx: &mut Context<Editor>) {
    if !preview.borrow().as_ref().is_some_and(|drag| drag.pointer == pointer) { return; }
    let drag = preview.borrow_mut().take().unwrap();
    
    if drag.base != ed.revision() || drag.project_id != ed.project().project_id { cx.notify(); return; }
    let mut commands = Vec::with_capacity(2);
    if let Some(tick) = drag.source_tick.filter(|tick| *tick != drag.tick) {
        commands.push(verdant_domain::Command::SetTempoEvent { tick, micro_bpm: None });
    }
    commands.push(verdant_domain::Command::SetTempoEvent { tick: drag.tick, micro_bpm: Some(drag.micro_bpm) });
    ed.commit(drag.base, verdant_domain::CommandBatch { label: "编辑速度事件".into(), commands }, cx);
    cx.notify();
}

fn cancel_tempo(preview: &TempoPreview, pointer: PointerId, window: &mut Window, cx: &mut Context<Editor>) {
    if !preview.borrow().as_ref().is_some_and(|drag| drag.pointer == pointer) { return; }
    preview.borrow_mut().take();
    
    cx.notify();
}

fn tempo_context(ed: &mut Editor, x: f64, y: f64, height: f64, window: &mut Window, cx: &mut Context<Editor>) {
    use crate::editor::WorkflowAction;
    use crate::menu::MenuItem;
    let hit = tempo_hit(ed, x, y, height, tempo_range(&ed.project().tempo_map));
    let tick = hit.unwrap_or_else(|| ed.session.snap.snap(ed.session.viewport.x_to_tick(x)).max(0));
    ed.show_menu(window.mouse_position(), vec![
        MenuItem::action("编辑 / 新增速度事件…", MenuAction::Workflow(WorkflowAction::EditTempo(tick))),
        MenuItem::action("删除速度事件", MenuAction::Workflow(WorkflowAction::DeleteTempo(tick))).enabled(hit.is_some() && tick != 0),
    ], window, cx);
}

fn install_tempo_input(editor: &Entity<Editor>, bounds: Bounds<Pixels>, hitbox: &Hitbox,
    preview: &TempoPreview, input: &crate::proto_editor_surface::Input, window: &mut Window) {
    if hitbox.is_hovered(window) { window.set_cursor_style(CursorStyle::Crosshair, hitbox); }
    let (e,state) = (editor.clone(),preview.clone());
    let height = f64::from(bounds.size.height);
    crate::proto_editor_surface::install_pointer_input(input,bounds,hitbox,window,move |event,window,cx| {
        use crate::proto_editor_surface::PointerPhase;
        e.update(cx, |ed,cx| match event.phase {
            PointerPhase::Begin => {
                if event.clicks >= 2 {
                    let tick = tempo_hit(ed,event.x,event.y,height,tempo_range(&ed.project().tempo_map))
                        .unwrap_or_else(||ed.session.snap.snap(ed.session.viewport.x_to_tick(event.x)).max(0));
                    ed.run_workflow(crate::editor::WorkflowAction::EditTempo(tick),window,cx);
                    false
                } else { begin_tempo(ed,&state,event.pointer,event.x,event.y,height,event.modifiers,window,cx) }
            }
            PointerPhase::Sample => { move_tempo(ed,&state,event.pointer,event.x,event.y,height,event.modifiers,cx); true }
            PointerPhase::Finish => { end_tempo(ed,&state,event.pointer,window,cx); true }
            PointerPhase::Context => { state.borrow_mut().take(); tempo_context(ed,event.x,event.y,height,window,cx); true }
        })
    });
}

fn paint_lane(editor:&Entity<Editor>,bounds:Bounds<Pixels>,param:Param,window: &mut Window, cx: &App, text: &mut Texts) {
    let ed = editor.read(cx);
    let (th,vp)=(ed.theme,ed.session.viewport);
    let (w, height) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
    let o = bounds.origin;
    window.paint_quad(fill(bounds, th.roll_white));
    let Some(track) = ed.active_track() else { return };
    let color = th.track(track.color.as_deref());
    let range = ed.pit_range_cents;
    window.paint_quad(fill(b(0.0, y_of(param, param.default_value(), height, range), w, 1.0, o), th.grid_bar));
    let (from, to) = vp.visible_ticks();
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        let notes = ed.index.notes_in_range(track, from.floor() as i64, to.ceil() as i64 + 1);
        // Compact bands have no curves or handles. If even their one-pixel
        // minimum bounds are disjoint, replay two layers of quads instead of
        // recalculating their draw order one primitive at a time in GPUI.
        let compact_bands = if matches!(param, Param::Portamento | Param::Vibrato) {
            false
        } else {
            let mut previous_right = f64::NEG_INFINITY;
            notes.clone().all(|note| {
                let start = vp.tick_to_x(note.start_tick as f64);
                let left = start.max(0.0);
                let right = vp.tick_to_x(note.end_tick() as f64).min(w);
                if left < previous_right || right - left >= 4.0 {
                    return false;
                }
                previous_right = right.max(left + 1.0);
                true
            })
        };
        if compact_bands {
            window.paint_layer(|window| {
                for note in notes.clone() {
                    let left = vp.tick_to_x(note.start_tick as f64).max(0.0);
                    let right = vp.tick_to_x(note.end_tick() as f64).min(w);
                    window.paint_quad(fill(b(left, 0.0, (right - left).max(1.0), height, o), th.roll_black));
                }
            });
            window.paint_layer(|window| {
                for note in notes.clone() {
                    let start = vp.tick_to_x(note.start_tick as f64);
                    window.paint_quad(fill(b(start, 0.0, 1.0, height, o), th.grid_beat));
                }
            });
        }
        for note in notes {
            let (start, end) = (vp.tick_to_x(note.start_tick as f64), vp.tick_to_x(note.end_tick() as f64));
            let (left, right) = (start.max(0.0), end.min(w));
            if !compact_bands {
                window.paint_quad(fill(b(left, 0.0, (right - left).max(1.0), height, o), th.roll_black));
                window.paint_quad(fill(b(start, 0.0, 1.0, height, o), th.grid_beat));
            }
            if ed.region_lane_disabled(note, param) {
                text.push(("已旁路".into(), gpui::point(o.x + px((left + 4.0) as f32), o.y + px(16.0)), 10.0, th.text_tertiary));
            }
            match param {
                Param::Portamento => {
                    let preview = ed.lane_stroke.as_ref().filter(|stroke| stroke.param == param && !stroke.roll)
                        .and_then(|stroke| crate::curve_span::note_value(&stroke.samples, note.start_tick, note.duration_tick,
                            param.default_value(), stroke.erase));
                    let value = preview.unwrap_or_else(|| verdant_backend::f0::portamento_position(note));
                    let authored = !note.curves.expression("POR").is_empty() || preview.is_some();
                    let line_color = if preview.is_some() && ed.lane_stroke.as_ref().is_some_and(|stroke| stroke.erase) { th.error }
                        else if authored { color } else { with_alpha(th.text_tertiary, 0.6) };
                    let y = y_of(param, value, height, range);
                    polyline(window, o, [Some((left, y)), Some((right, y))], if authored { 2.0 } else { 1.0 }, line_color);
                }
                p if p != Param::Vibrato => {
                    // Below ~4 px a note renders as a flat band; the curve and
                    // its handles are invisible, so skip sampling entirely.
                    if right - left < 4.0 { continue; }
                    let points=param.point_samples(note);
                    let steps = ((right - left) / 2.0).ceil().max(1.0) as usize;
                    polyline(window, o, (0..=steps).map(|i| {
                        let x = left + (right - left) * i as f64 / steps as f64;
                        Some((x, y_of(param, param.interpolate_note(note, vp.x_to_tick(x) - note.start_tick as f64), height, range)))
                    }), if points.is_empty() { 1.0 } else { 2.0 }, if points.is_empty() { with_alpha(th.text_tertiary, 0.6) } else { color });
                    for (id,tick,value) in &points {
                        let x = vp.tick_to_x((note.start_tick + tick) as f64);
                        if (0.0..=w).contains(&x){let selected=!ed.point_selection.is_empty()&&ed.param==param&&ed.point_selection.contains(&(note.id.clone(),id.clone()));
                            let shift=ed.lane_stroke.as_ref().filter(|s|s.param==param&&s.point.is_some()&&selected).and_then(|s|Some((s.samples.last()?.0-s.samples.first()?.0,s.samples.last()?.1-s.samples.first()?.1))).unwrap_or((0.,0.));
                            let x=vp.tick_to_x((note.start_tick+tick)as f64+shift.0);let y=y_of(param,*value+shift.1,height,range);if selected{window.paint_quad(fill(b(x-4.,y-4.,8.,8.,o),th.text));}window.paint_quad(fill(b(x-2.,y-2.,4.,4.,o),th.accent));
                        }
                    }
                }
                Param::Vibrato => {
                    let preview = ed.lane_stroke.as_ref().filter(|stroke| stroke.param == param && !stroke.roll && stroke.tool != LaneTool::Select)
                        .and_then(|stroke| crate::curve_span::note_value(&stroke.samples, note.start_tick, note.duration_tick,
                            0., stroke.erase));
                    let value = preview.unwrap_or_else(|| note.vibrato.as_ref().map(|v| v.depth_cents).unwrap_or(0) as f64);
                    let y = y_of(param, value, height, range);
                    if value > 0.0 { window.paint_quad(fill(b(left + 2.0, y, (right - left - 4.0).max(1.0), height - PAD - y, o), with_alpha(color, 0.75))); }
                }
                _ => {},
            }
        }
        if let Some(stroke) = ed.lane_stroke.as_ref().filter(|s| s.param == param && !s.roll && s.param != Param::Portamento) {
            if stroke.tool==LaneTool::Select&&stroke.point.is_none(){if let(Some(first),Some(last))=(stroke.samples.first(),stroke.samples.last()){
                let(x0,x1)=(vp.tick_to_x(first.0),vp.tick_to_x(last.0));let(y0,y1)=(y_of(param,first.1,height,range),y_of(param,last.1,height,range));window.paint_quad(quad(b(x0.min(x1),y0.min(y1),(x1-x0).abs().max(1.),(y1-y0).abs().max(1.),o),px(0.),with_alpha(th.accent,0.1),px(1.),th.accent,BorderStyle::Dashed));
            }}else{
                let color = if stroke.erase { th.error } else { th.accent };
                polyline(window,o,stroke.samples.iter().map(|(tick,value)|Some((vp.tick_to_x(*tick),y_of(param,*value,height,range)))),2.,color);
                if let Some((tick, value)) = stroke.samples.last() {
                    window.paint_quad(fill(b(vp.tick_to_x(*tick)-2.,y_of(param,*value,height,range)-2.,4.,4.,o),color));
                }
            }
        }
    });
    let scale = match param { Param::Pit => format!("±{range:.0}¢ · 自由绘制不吸附 · 右键选择平滑或控制点"), Param::Dyn => "+6 / −24 dB".into(), Param::Vibrato => "0–200¢".into(), Param::Portamento => "按音符起点生效 · 0 提前 · 64 中央 · 127 延后".into(), _ => format!("0–127 · 默认 {:.0}", param.default_value()) };
    let scale_width = if matches!(param, Param::Pit | Param::Portamento) { 420.0 } else { 100.0 };
    text.push((scale, gpui::point(o.x + px((w - scale_width).max(6.0) as f32), o.y + px(3.0)), 10.0, th.text_tertiary));
}

fn down(ed:&mut Editor,param:Param,slot:usize,pointer: PointerId, x: f64, y: f64, height: f64, modifiers: gpui::Modifiers, window: &mut Window, cx: &mut Context<Editor>) -> bool {
    if ed.lane_stroke.is_some(){return false;}
    if ed.param!=param{ed.point_selection.clear();}ed.param=param;ed.integrations.active_panel=slot;
    if !verdant_backend::params::support(ed.param, ed.route(), ed.scheduler.capabilities.as_ref()).is_effective() { return false; }
    let erase = ed.lane_tool == LaneTool::Eraser || ed.tool == Tool::Eraser;
    let tick = ed.session.viewport.x_to_tick(x);
    if !ed.region_lane_editable(param, tick, tick) {
        ed.notice("先选择此音符区域并启用曲线，再编辑控制点"); cx.notify(); return false;
    }
    let value = if erase && param == Param::Pit { 0. } else { value_at(ed.param, y, height, ed.pit_range_cents) };
    let mut point = None;
    if ed.lane_tool==LaneTool::Select && !erase && !matches!(ed.param, Param::Vibrato | Param::Portamento) {
        if let Some(track) = ed.active_track() {
            let slack = 8.0 / ed.session.viewport.px_per_tick;
            for n in ed.index.notes_in_range(track, (tick - slack) as i64, (tick + slack) as i64 + 1) {
                let pts = ed.param.point_samples(n);
                for (id, offset, value) in pts {
                    let (px_, py_) = (ed.session.viewport.tick_to_x((n.start_tick + offset) as f64), y_of(ed.param, value, height, ed.pit_range_cents));
                    if (px_ - x).abs() <= 7.0 && (py_ - y).abs() <= 7.0 { point = Some((n.id.clone(), id, (n.start_tick + offset) as f64, value)); break; }
                }
                if point.is_some() { break; }
            }
        }
    }
    let (note, point, tick, value) = match point {
        Some((note, point, tick, value)) => {
            if modifiers.shift { if !ed.point_selection.insert((note.clone(),point.clone())){ed.point_selection.remove(&(note.clone(),point.clone()));} }
            else if !ed.point_selection.contains(&(note.clone(),point.clone())){ed.point_selection=[(note.clone(),point.clone())].into();}
            ed.session.selection.insert(note.clone());(Some(note),Some(point),tick,value)
        },
        None => (None, None, if modifiers.alt || ed.param == Param::Portamento || ed.lane_tool != LaneTool::Select { tick } else { ed.session.snap.snap(tick) as f64 }, value),
    };
    if !ed.begin_curve(ed.param, pointer, note, point, false, tick, value, (x, y), erase) { return false; }
     cx.notify(); true
}
fn moved(ed:&mut Editor,param:Param,pointer: PointerId, x: f64, y: f64, height: f64, modifiers: gpui::Modifiers, cx: &mut Context<Editor>) {
    if !ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer && !s.roll && s.param==param) { return; }
    let tick = ed.session.viewport.x_to_tick(x);
    let value=value_at(param,y,height,ed.pit_range_cents);
    let drawing = ed.lane_stroke.as_ref().is_some_and(|s| s.point.is_none() && s.tool != LaneTool::Select);
    ed.curve_sample(pointer, tick, value, (x, y), modifiers.shift, modifiers.alt || drawing); cx.notify();
}
fn up(ed:&mut Editor,param:Param,pointer: PointerId, window: &mut Window, cx: &mut Context<Editor>) {
    if !ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer && !s.roll && s.param==param) { return; }
    
    {
        let (from, to) = ed.lane_stroke.as_ref().unwrap().samples.iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(from, to), sample| (from.min(sample.0), to.max(sample.0)));
        if !ed.region_lane_editable(param, from, to) {
            ed.lane_stroke = None; ed.notice("绘制区域包含已旁路的曲线；先启用后再编辑"); cx.notify(); return;
        }
    }
    if param == Param::Pit && ed.lane_stroke.as_ref().is_some_and(|s| s.point.is_none() && s.tool != LaneTool::Select) {
        let Some(stroke) = ed.lane_stroke.take() else { return; };
        if stroke.base == ed.revision() {
            if let Some(track) = ed.active_track() {
                let line;
                let samples = if stroke.tool == LaneTool::Line && stroke.samples.len() > 1 {
                    line = [stroke.samples[0], *stroke.samples.last().unwrap()];
                    &line[..]
                } else { &stroke.samples[..] };
                if let Some(batch) = crate::curve_span::pitch_stroke_batch(track, &ed.tempo(), samples, matches!(stroke.tool, LaneTool::Pencil | LaneTool::Smooth), stroke.erase) {
                    ed.commit(stroke.base, batch, cx);
                }
            }
        }
        cx.notify();
    } else { ed.commit_lane_stroke(cx); }
}
fn context(ed: &mut Editor, tick: i64, window: &mut Window, cx: &mut Context<Editor>) {
    use crate::menu::MenuItem;
    let items = vec![MenuItem::action("自由绘制", MenuAction::SetLaneTool(LaneTool::Pencil)), MenuItem::action("直线", MenuAction::SetLaneTool(LaneTool::Line)),
        MenuItem::action("擦除", MenuAction::SetLaneTool(LaneTool::Eraser)), MenuItem::separator(),
        MenuItem::action("框选控制点",MenuAction::SetLaneTool(LaneTool::Select)),
        MenuItem::action("选择所选音符的全部点",MenuAction::Workflow(crate::editor::WorkflowAction::SelectPoints)),
        MenuItem::action("变换选中点…",MenuAction::Workflow(crate::editor::WorkflowAction::TransformPoints)),
        MenuItem::action("线性插值",MenuAction::Workflow(crate::editor::WorkflowAction::Interpolation(verdant_domain::editing::Interpolation::Linear))),
        MenuItem::action("阶梯插值",MenuAction::Workflow(crate::editor::WorkflowAction::Interpolation(verdant_domain::editing::Interpolation::Hold))),
        MenuItem::action("余弦插值",MenuAction::Workflow(crate::editor::WorkflowAction::Interpolation(verdant_domain::editing::Interpolation::Cosine))),
        MenuItem::action("单调曲线插值",MenuAction::Workflow(crate::editor::WorkflowAction::Interpolation(verdant_domain::editing::Interpolation::Cubic))),
        MenuItem::action("复制选中音符的曲线", MenuAction::CopyCurve), MenuItem::action("在光标处粘贴曲线", MenuAction::PasteCurveAt(tick)).enabled(ed.curve_clipboard.is_some())];
    ed.show_menu(window.mouse_position(), items, window, cx);
}
fn cancel_lane_pointer(ed: &mut Editor, param: Param, slot: usize, pointer: PointerId, window: &mut Window, cx: &mut Context<Editor>) {
    if ed.integrations.active_panel != slot || !ed.lane_stroke.as_ref()
        .is_some_and(|stroke| stroke.pointer == pointer && !stroke.roll && stroke.param == param) { return; }
    ed.lane_stroke = None;
    
    cx.notify();
}

fn install_input(editor:&Entity<Editor>,bounds:Bounds<Pixels>,hitbox:&Hitbox,param:Param,slot:usize,
    input:&crate::proto_editor_surface::Input,window:&mut Window,_:&mut App) {
    if hitbox.is_hovered(window) { window.set_cursor_style(CursorStyle::Crosshair,hitbox); }
    let e=editor.clone(); let height=f64::from(bounds.size.height);
    crate::proto_editor_surface::install_pointer_input(input,bounds,hitbox,window,move |event,window,cx| {
        use crate::proto_editor_surface::PointerPhase;
        e.update(cx,|ed,cx| match event.phase {
            PointerPhase::Begin => down(ed,param,slot,event.pointer,event.x,event.y,height,event.modifiers,window,cx),
            PointerPhase::Sample => { if ed.integrations.active_panel==slot { moved(ed,param,event.pointer,event.x,event.y,height,event.modifiers,cx); } true }
            PointerPhase::Finish => { if ed.integrations.active_panel==slot { up(ed,param,event.pointer,window,cx); } true }
            PointerPhase::Context => { ed.param=param; ed.integrations.active_panel=slot; context(ed,ed.session.viewport.x_to_tick(event.x) as i64,window,cx); true }
        })
    });
}

#[cfg(test)]
mod pointer_boundary_tests {
    use super::*;
    use gpui::{AppContext, Render, TestAppContext, WindowHandle, div, point, size};

    struct Harness { editor: Entity<Editor>, tempo: bool, preview: TempoPreview }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let (editor, tempo, preview) = (self.editor.clone(), self.tempo, self.preview.clone());
            let cancel_editor = editor.clone(); let cancel_preview = preview.clone();
            let compiled = crate::proto_editor_surface::surface(editor.clone(), "lane-native-regression", "参数曲线", 0.0,
                move |input| {
                    let editor = editor.clone(); let preview = preview.clone();
                    canvas(|bounds,window,_|window.insert_hitbox(bounds,HitboxBehavior::Normal),move |bounds,hitbox,window,cx| {
                        if tempo { install_tempo_input(&editor,bounds,&hitbox,&preview,&input,window); }
                        else { install_input(&editor,bounds,&hitbox,Param::Dyn,0,&input,window,cx); }
                    }).size_full().into_any_element()
                },
                move |_,window,cx| {
                    cancel_preview.borrow_mut().take();
                    cancel_editor.update(cx,|ed,cx| { ed.lane_stroke=None;cx.notify(); });
                });
            div().size_full().child(div().w(px(160.0)).h(px(80.0)).child(compiled))
                .child(canvas(|_, _, _| {}, |_, _, window, _| {
                    window.on_mouse_event(|_: &MouseUpEvent, phase, _, cx| {
                        if phase == DispatchPhase::Bubble { cx.stop_propagation(); }
                    });
                }).size(px(0.0)))
        }
    }
    fn open(cx: &mut TestAppContext, tempo: bool) -> WindowHandle<Harness> {
        let mut project = verdant_domain::Project::new_empty();
        project.tracks[0].notes = vec![verdant_domain::Note::new(0, 300, 60)];
        project.tracks[0].refresh_index();
        let window = cx.open_window(size(px(320.0), px(200.0)), |window, cx| {
            let editor = cx.new(|cx| {
                let mut editor = Editor::for_native_test(project, window, cx);
                editor.scheduler.capabilities = Some(verdant_backend::Capabilities::default());
                editor.needs_fit = false;
                editor.session.viewport.start_tick = 0.0;
                editor.session.viewport.px_per_tick = 1.0;
                editor.session.snap.grid_ticks = 0;
                editor.lane_tool = LaneTool::Pencil;
                editor
            });
            Harness { editor, tempo, preview: TempoPreview::default() }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        window
    }
    fn owner(host: &Harness, cx: &App) -> Option<PointerId> {
        if host.tempo { host.preview.borrow().as_ref().map(|drag| drag.pointer) }
        else { host.editor.read(cx).lane_stroke.as_ref().map(|stroke| stroke.pointer) }
    }
    fn down(host: &Harness, window: &mut Window, cx: &mut App) {
        window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
            position: point(px(40.0), px(40.0)), button: MouseButton::Left, ..Default::default()
        }), cx);
        assert_eq!(owner(host, cx), Some(PointerId::Mouse));
        assert!(window.captured_hitbox().is_some());
    }
    fn touch(window: &mut Window, cx: &mut App, phase: TouchPhase, x: f32, y: f32) {
        window.dispatch_event(gpui::PlatformInput::Touch(gpui::TouchEvent {
            id: gpui::TouchId(11), phase, position: point(px(x), px(y)), ..Default::default()
        }), cx);
    }
    #[gpui::test]
    fn curve_and_tempo_cancel_outside_motion_or_terminal_sample(cx: &mut TestAppContext) {
        for tempo in [false, true] { for terminal in [false, true] {
            let window = open(cx, tempo);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: point(px(70.0), px(25.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                let outside = point(px(-20.0), px(40.0));
                if terminal {
                    window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: outside, button: MouseButton::Left, ..Default::default() }), cx);
                } else {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent { position: outside, pressed_button: Some(MouseButton::Left), ..Default::default() }), cx);
                }
                assert!(owner(host, cx).is_none());
                assert!(window.captured_hitbox().is_none());
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent { position: point(px(70.0), px(25.0)), button: MouseButton::Left, ..Default::default() }), cx);
                assert_eq!(host.editor.read(cx).revision().get(), 0);
            }).unwrap();
        } }
    }
    #[gpui::test]
    fn curve_and_tempo_accept_final_sample_outside_control_but_inside_window(cx: &mut TestAppContext) {
        for tempo in [false, true] {
            let window = open(cx, tempo);
            window.update(cx, |host, window, cx| {
                down(host, window, cx);
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                    position: point(px(200.0), px(60.0)), button: MouseButton::Left, ..Default::default()
                }), cx);
                assert_eq!(host.editor.read(cx).revision().get(), 1);
                if tempo { assert!(host.editor.read(cx).project().tempo_map.iter().any(|point| point.tick == 200)); }
                assert!(owner(host, cx).is_none());
                assert!(window.captured_hitbox().is_none());
            }).unwrap();
        }
    }
    #[gpui::test]
    fn lane_touch_owner_ignores_rival_mouse_and_cancels_its_own_outside_sample(cx: &mut TestAppContext) {
        for tempo in [false, true] {
            let window = open(cx, tempo);
            window.update(cx, |host, window, cx| {
                touch(window, cx, TouchPhase::Started, 40.0, 40.0);
                touch(window, cx, TouchPhase::Moved, 70.0, 25.0);
                assert_eq!(owner(host, cx), Some(PointerId::Touch(0)));
                window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                    position: point(px(-20.0), px(40.0)), pressed_button: Some(MouseButton::Left), ..Default::default()
                }), cx);
                assert_eq!(owner(host, cx), Some(PointerId::Touch(0)));
                touch(window, cx, TouchPhase::Moved, -20.0, 40.0);
                assert!(owner(host, cx).is_none());
                assert!(window.captured_hitbox().is_none());
                touch(window, cx, TouchPhase::Ended, 70.0, 25.0);
                assert_eq!(host.editor.read(cx).revision().get(), 0);
            }).unwrap();
        }
    }
}
