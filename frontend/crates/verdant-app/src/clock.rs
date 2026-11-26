//! Clock surfaces are siblings of the cached static editor. Static layout
//! records their exact bounds/masks; the playback layer uses those anchors
//! after static prepaint, including scroll clipping and same-frame resizing.
use std::{cell::RefCell, rc::Rc, time::Instant};
use gpui::{AnyElement, App, AppContext, Bounds, ContentMask, Context, Entity, IntoElement, Pixels, Render, Role, WeakEntity, Window, canvas, div, fill, prelude::*, px};
use verdant_domain::timing::{MeterMap, TempoMap};
use verdant_session::{EditorSession, viewport::Viewport};
use crate::editor::Editor;
use crate::paint::{b, p};

#[derive(Clone, Copy)]
pub enum Space { Editor, Arrange }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Surface { Lcd, Roll, PhonemeLane, TempoLane, Lane, EditorRuler, Arrange, ArrangeRuler, Meter(usize), HeaderMeter(usize), Master }

#[derive(Clone, Copy)]
struct Anchor {
    surface: Surface,
    bounds: Bounds<Pixels>,
    mask: ContentMask<Pixels>,
}

pub struct PlaybackViews {
    pub driver: Entity<ClockDriver>,
    pub lcd: Entity<LcdView>,
    pub meters: Vec<Entity<MeterView>>,
    pub header_meters: Vec<Entity<MeterView>>,
    pub master: Entity<MeterView>,
    anchors: Rc<RefCell<Vec<Anchor>>>,
}

fn observe<T: 'static>(editor: &WeakEntity<Editor>, cx: &mut Context<T>) {
    if let Some(ed) = editor.upgrade() { cx.observe(&ed, |_, _, cx| cx.notify()).detach(); }
}

impl PlaybackViews {
    pub fn new(editor: WeakEntity<Editor>, cx: &mut Context<Editor>) -> Self {
        let driver = cx.new(|cx| {
            observe(&editor, cx);
            ClockDriver { editor: editor.clone(), transport: None, follow_frame: None }
        });
        let lcd = cx.new(|cx| { observe(&editor, cx); LcdView { editor: editor.clone() } });
        let mut meter = |slot, vertical| cx.new(|cx| { observe(&editor, cx); MeterView { editor: editor.clone(), slot, vertical } });
        let meters = (0..verdant_audio::MAX_TRACKS).map(|i| meter(Some(i), true)).collect();
        let header_meters = (0..verdant_audio::MAX_TRACKS).map(|i| meter(Some(i), false)).collect();
        let master = meter(None, true);
        Self { driver, lcd, meters, header_meters, master, anchors: Rc::new(RefCell::new(Vec::new())) }
    }

    pub fn clear_arrangement_anchors(&self) {
        self.anchors.borrow_mut().retain(|anchor| !is_arrangement_anchor(anchor.surface));
    }

    pub fn clear_bottom_anchors(&self) {
        self.anchors.borrow_mut().retain(|anchor| anchor.surface == Surface::Lcd || is_arrangement_anchor(anchor.surface));
    }

    /// A geometry-only placeholder in the static subtree. It neither owns a
    /// dynamic child nor inserts a hitbox that could intercept editor input.
    pub fn anchor(&self, surface: Surface) -> impl IntoElement + use<> {
        let anchors = self.anchors.clone();
        canvas(move |bounds, window, _| {
            let mut anchors = anchors.borrow_mut();
            let anchor = Anchor { surface, bounds, mask: window.content_mask() };
            if surface == Surface::Lcd {
                // The control bar is cached separately and may repaint while
                // the timeline keeps its geometry, or the reverse.
                if let Some(lcd) = anchors.iter_mut().find(|other| other.surface == surface) {
                    *lcd = anchor;
                    return;
                }
            }
            anchors.push(anchor);
        }, |_, (), _, _| {}).size_full()
    }

    fn element(&self, surface: Surface) -> Option<AnyElement> {
        Some(match surface {
            Surface::Lcd => self.lcd.clone().into_any_element(),
            Surface::Meter(i) => self.meters[i].clone().into_any_element(),
            Surface::HeaderMeter(i) => self.header_meters[i].clone().into_any_element(),
            Surface::Master => self.master.clone().into_any_element(),
            Surface::Roll | Surface::PhonemeLane | Surface::TempoLane | Surface::Lane |
            Surface::EditorRuler | Surface::Arrange | Surface::ArrangeRuler => return None,
        })
    }
}

fn is_arrangement_anchor(surface: Surface) -> bool {
    matches!(surface, Surface::Arrange | Surface::ArrangeRuler | Surface::HeaderMeter(_))
}

/// Build child layout during prepaint, after the toolbar and central content
/// have recorded their anchors and before the side drawers paint. Reusing a
/// static cache leaves the recorded anchors intact.
pub fn playback_layer(editor: Entity<Editor>, touch_seen: bool) -> impl IntoElement {
    let touch_editor = editor.clone();
    canvas(move |_, window, cx| {
        let ed = editor.read(cx);
        let anchors = ed.playback.anchors.borrow().clone();
        let tick = ed.tempo().tick_at(ed.frame_playhead);
        let editor_x = ed.session.viewport.tick_to_x(tick);
        let arrange_x = ed.arrange.tick_to_x(tick);
        let playhead = ed.theme.playhead;
        let mut elements = Vec::with_capacity(anchors.len() + 1);
        for anchor in &anchors {
            let mut element = editor.read(cx).playback.element(anchor.surface);
            if let Some(element) = element.as_mut() {
                window.with_content_mask(Some(anchor.mask), |window| {
                    element.layout_as_root(anchor.bounds.size.into(), window, cx);
                    element.prepaint_at(anchor.bounds.origin, window, cx);
                });
            }
            elements.push((*anchor, element));
        }
        // Keep IME and lyric actions outside PianoRoll, above the playhead,
        // under the same Editor action ancestor and below popup menus.
        if let Some(anchor) = anchors.iter().find(|a| a.surface == Surface::Roll) {
            let lyric = editor.update(cx, |ed, cx| {
                crate::lyric_view::lyric_overlay(ed, &editor, cx).map(|lyric|
                    div().relative().size_full().child(lyric).into_any_element())
            });
            if let Some(mut element) = lyric {
                window.with_content_mask(Some(anchor.mask), |window| {
                    element.layout_as_root(anchor.bounds.size.into(), window, cx);
                    element.prepaint_at(anchor.bounds.origin, window, cx);
                });
                elements.push((*anchor, Some(element)));
            }
        }
        (elements, editor_x, arrange_x, playhead)
    }, move |_, (mut elements, editor_x, arrange_x, playhead), window, cx| {
        // The first touch anywhere switches the toolbar to touch-sized targets.
        if !touch_seen {
            let mark = |editor: &Entity<Editor>, cx: &mut App| editor.update(cx, |ed, cx| if !ed.touch_ui && !ed.prefs.extra.contains_key("touch_layout") { ed.touch_ui = true; cx.notify(); });
            let editor = touch_editor.clone();
            window.on_mouse_event(move |_: &gpui::TouchDragEvent, phase, _, cx| if phase == gpui::DispatchPhase::Capture { mark(&editor, cx) });
            let editor = touch_editor.clone();
            window.on_mouse_event(move |_: &gpui::LongPressEvent, phase, _, cx| if phase == gpui::DispatchPhase::Capture { mark(&editor, cx) });
        }
        for (anchor, element) in &mut elements {
            window.with_content_mask(Some(anchor.mask), |window| {
                if let Some(element) = element {
                    element.paint(window, cx);
                } else {
                    let x = if is_arrangement_anchor(anchor.surface) { arrange_x } else { editor_x };
                    let ruler = matches!(anchor.surface, Surface::EditorRuler | Surface::ArrangeRuler);
                    paint_playhead(anchor.bounds, x, ruler, playhead, window);
                }
            });
        }
    }).absolute().inset_0()
}

pub struct ClockDriver {
    editor: WeakEntity<Editor>,
    transport: Option<verdant_audio::Transport>,
    follow_frame: Option<Instant>,
}

pub(crate) fn toggle_follow(editor: &mut Editor, cx: &mut Context<Editor>) {
    let enabled = !editor.session.follow_playhead;
    set_follow(editor, enabled, cx);
}

/// Commands may request a toggle; a compiled control supplies its accepted
/// selected proposal directly. Repeating that proposal is a no-op.
pub(crate) fn set_follow(editor: &mut Editor, enabled: bool, cx: &mut Context<Editor>) {
    if editor.session.follow_playhead == enabled { return; }
    editor.session.follow_playhead = enabled;
    editor.playback.driver.update(cx, |driver, cx| {
        driver.follow_frame = enabled.then(Instant::now);
        cx.notify();
    });
    cx.notify();
}

impl Render for ClockDriver {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let driver = cx.weak_entity();
        // Native next-frame callbacks run before Window::draw. Notifications
        // there reach observers; in-draw notify only dirties the current view
        // and would leave its StaticEditor sibling's cache untouched.
        window.on_next_frame(move |window, cx| {
            let _ = driver.update(cx, |driver, cx| {
                let Some(editor) = driver.editor.upgrade() else { return };
                let (animate, transport) = editor.update(cx, |ed, cx| {
                    let mut animate = ed.clock_pump(window, cx);
                    let transport = ed.audio.transport();
                    let playing = transport == verdant_audio::Transport::Playing;
                    if ed.session.follow_playhead && (playing || driver.follow_frame.is_some()) {
                        let now = Instant::now();
                        let elapsed = driver.follow_frame.map_or(0.0, |last| (now - last).as_secs_f64().min(0.05));
                        let tick = ed.tempo().tick_at(ed.frame_playhead);
                        let roll_before = ed.session.viewport.start_tick;
                        let arrange_before = ed.arrange.start_tick;
                        let editing = ed.session.pointer_owner().is_some() || ed.note_creation.is_some()
                            || ed.lane_stroke.is_some() || ed.arrange_drag.is_some()
                            || crate::roll::has_extra_drag(cx.entity_id(), cx)
                            || crate::ruler_navigation::is_active(cx.entity_id(), cx);
                        let settling = !editing && follow_playhead(&mut ed.session, &mut ed.arrange, tick, elapsed);
                        if roll_before != ed.session.viewport.start_tick { ed.notify_viewport(Space::Editor, cx); }
                        if arrange_before != ed.arrange.start_tick { ed.notify_viewport(Space::Arrange, cx); }
                        driver.follow_frame = (playing || settling).then_some(now);
                        animate |= settling;
                    } else {
                        driver.follow_frame = None;
                    }
                    if driver.transport.is_some_and(|previous| previous != transport) {
                        // Automatic stop/device transitions are discrete
                        // control-bar changes; regular clock ticks stay local.
                        cx.notify();
                    }
                    (animate, transport)
                });
                driver.transport = Some(transport);
                if animate { cx.notify(); }
            });
        });
        div().size(px(0.0))
    }
}

fn paint_playhead(bounds: Bounds<Pixels>, x: f64, ruler: bool, color: gpui::Rgba, window: &mut Window) {
    let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
    if !(0.0..=w).contains(&x) { return; }
    if ruler {
        let mut tri = gpui::PathBuilder::fill();
        tri.move_to(p(x - 5.0, h - 8.0, bounds.origin));
        tri.line_to(p(x + 5.0, h - 8.0, bounds.origin));
        tri.line_to(p(x, h, bounds.origin));
        tri.close();
        if let Ok(path) = tri.build() { window.paint_path(path, color); }
    } else {
        window.paint_quad(fill(b(x, 0.0, 1.0, h, bounds.origin), color));
    }
}

/// Ease both timeline views toward the playhead using elapsed time, so the
/// rate is consistent across display refresh rates. Returns true while settling.
fn follow_playhead(session: &mut EditorSession, arrange: &mut Viewport, tick: f64, elapsed: f64) -> bool {
    if !session.follow_playhead || session.autoscroll_active { return false; }
    let fraction = -(-elapsed / 0.09).exp_m1();
    let mut settling = false;
    for viewport in [&mut session.viewport, arrange] {
        let target = (tick - viewport.width * 0.35 / viewport.px_per_tick).max(0.0);
        let distance = target - viewport.start_tick;
        if distance.abs() * viewport.px_per_tick <= 0.25 {
            viewport.start_tick = target;
        } else {
            viewport.start_tick += distance * fraction;
            settling = true;
        }
    }
    settling
}

/// The audio thread publishes its position once per device buffer, so the
/// raw clock is a staircase (often 10–20 ms steps). Drawing it directly makes
/// the playhead stutter. This phase-locked estimator advances with wall time
/// between buffers and is pulled gently toward each new audio position: it
/// never runs backwards while playing, never leads the audio by more than
/// `MAX_LEAD`, and snaps on seeks, stops and large discontinuities.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayheadSmoother {
    state: Option<SmootherState>,
}

#[derive(Clone, Copy, Debug)]
struct SmootherState { estimate: f64, at: Instant, raw: f64 }

impl PlayheadSmoother {
    /// Larger corrections are treated as a seek or device reset.
    const SNAP: f64 = 0.25;
    /// Fraction of the phase error removed at each new audio position.
    const GAIN: f64 = 0.2;
    const MAX_LEAD: f64 = 0.06;

    pub fn reset(&mut self) { self.state = None; }

    pub fn sample(&mut self, raw: f64, now: Instant, playing: bool) -> f64 {
        let Some(mut st) = self.state.filter(|_| playing) else {
            self.state = playing.then_some(SmootherState { estimate: raw, at: now, raw });
            return raw;
        };
        let predicted = st.estimate + now.saturating_duration_since(st.at).as_secs_f64();
        let mut estimate = predicted;
        let mut snapped = false;
        if raw != st.raw {
            let error = raw - predicted;
            // Loop wraps and seeks move the audio clock backwards: follow them.
            snapped = error.abs() > Self::SNAP || raw < st.raw;
            estimate = if snapped { raw } else { predicted + error * Self::GAIN };
            st.raw = raw;
        }
        // Monotonic while playing, and bounded around the audible position.
        if !snapped { estimate = estimate.max(st.estimate); }
        estimate = estimate.clamp(raw - Self::SNAP, raw + Self::MAX_LEAD);
        st.estimate = estimate;
        st.at = now;
        self.state = Some(st);
        estimate
    }
}

struct LcdPosition { musical: String, time: String, signature: String, bpm: f64 }

/// Derive both readouts from one source frame. Musical boundaries use the
/// same absolute rounding as seek/render, including at tempo/meter changes.
fn lcd_position(frame: u64, sample_rate: u32, tempo: &TempoMap, meter: &MeterMap) -> LcdPosition {
    let seconds = frame as f64 / sample_rate as f64;
    let mut tick = tempo.tick_at(seconds).max(0.0).floor() as i64;
    if tempo.sample_at(tick.saturating_add(1), sample_rate) <= frame as i64 { tick = tick.saturating_add(1); }
    let position = meter.position_at(tick);
    let signature = meter.signature_at(tick);
    // Round once before decomposing, so 59.9996 seconds displays 01:00.000.
    let milliseconds = (seconds * 1000.0).round() as u64;
    LcdPosition {
        musical: format!("{:03}:{:02}:{:02}:{:03}", position.bar, position.beat, position.subdivision, position.tick),
        time: format!("{:02}:{:02}.{:03}", milliseconds / 60_000, milliseconds / 1000 % 60, milliseconds % 1000),
        signature: format!("{}/{}", signature.numerator, signature.denominator),
        bpm: tempo.bpm_at(tick),
    }
}

pub struct LcdView { editor: WeakEntity<Editor> }

/// Keep the LCD's four one-based fields aligned with the ruler's meter map.
pub(crate) fn position_label(meter: &verdant_domain::MeterMap, tick: f64) -> String {
    let position = meter.position_at(tick.max(0.0).floor() as i64);
    format!("{}  {}  {}  {}", position.bar, position.beat, position.subdivision, position.tick)
}

/// Audio time can lie between ticks. Rounding to the nearest tick can skip a
/// measure just before its boundary, or go back too far just after it.
pub(crate) fn bar_navigation_tick(meter: &verdant_domain::MeterMap, tick: f64, forward: bool) -> i64 {
    if forward { meter.next_bar(tick.max(0.0).floor() as i64) }
    else { meter.previous_bar(tick.max(0.0).ceil() as i64) }
}

pub(crate) fn bar_target_tick(meter: &verdant_domain::MeterMap, text: &str) -> Result<i64, String> {
    let bar = text.trim().parse::<i64>().map_err(|_| "小节号须为从 1 开始的整数".to_string())?;
    meter.tick_at_bar(bar).ok_or_else(|| "小节号须从 1 开始，且不得超出工程时间范围".into())
}

fn position_menu(ed:&mut Editor,position:gpui::Point<gpui::Pixels>,window:&mut Window,cx:&mut Context<Editor>) {
    use crate::editor::{MenuAction as M,WorkflowAction as A};use crate::menu::MenuItem as I;
    use crate::generated::controls::{CommandId as C, CommandArg};
    let playback=ed.playback_settings();
    let items=vec![I::action("跳转到小节…",M::Workflow(A::GoToBar)),I::action("回到开头",M::SeekTick(0)),I::separator(),
        I::boolean("跟随播放头",crate::menu::MenuBoolean::FollowPlayhead,ed.session.follow_playhead),I::boolean_command("循环播放",C::ToggleLoop,CommandArg::None,playback.looping),
        I::boolean_command("节拍器",C::ToggleMetronome,CommandArg::None,playback.metronome),I::separator(),
        I::action("在播放头编辑 / 新增速度…",M::Workflow(A::EditTempo(ed.tempo().tick_at(ed.frame_playhead).max(0.) as i64))),
        I::action("在播放头编辑 / 新增拍号…",M::Workflow(A::EditMeter(ed.tempo().tick_at(ed.frame_playhead).max(0.) as i64)))];
    ed.show_menu(position,items,window,cx);
}

impl Render for LcdView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.editor.upgrade() else { return div().into_any_element() };
        let ed = editor.read(cx);
        if ed.audio.transport() == verdant_audio::Transport::Playing { window.request_animation_frame(); }
        let th = ed.theme;
        let seconds = ed.frame_playhead;
        let frame = (seconds.max(0.0) * ed.audio_info.sample_rate as f64).round() as u64;
        let position = lcd_position(frame, ed.audio_info.sample_rate, &ed.tempo(), &ed.meter());
        let follow = ed.session.follow_playhead;
        let follow_authority = crate::proto_controls::editor_authority(ed);
        let ui_font = ed.ui_font.clone();
        let compact = ed.width_class == crate::ui_controls::WidthClass::Phone;
        // Every readout uses a fixed-pitch face with tabular figures and a
        // fixed-width, centred column, so running digits never shift.
        // Adwaita numeric display: the UI face with tabular figures on a quiet
        // rounded fill (no inset "LCD" box); fixed-width columns keep running
        // digits from shifting.
        div().size_full().flex().items_center().gap(px(10.0)).pl(px(12.0)).pr(px(4.0)).rounded(px(8.0)).bg(th.control)
            .font_family(ui_font.clone()).font_features(crate::theme::tabular_figures())
            .child(crate::proto_controls::action("lcd-position","播放位置菜单").flex_1()
                .on_context_menu({let editor=editor.clone();move|position,window,cx|editor.update(cx,|ed,cx|position_menu(ed,position,window,cx))})
                .on_click({let editor=editor.clone();move|event,window,cx|editor.update(cx,|ed,cx|position_menu(ed,event.position(),window,cx))})
                .min_w_0().h_full().flex().flex_col().items_center().justify_center()
                .aria_label(format!("播放位置，小节:拍:细分:刻度 {}，时间 {}", position.musical, position.time))
                .child(div().text_size(px(15.0)).line_height(px(18.0)).font_weight(gpui::FontWeight::SEMIBOLD).whitespace_nowrap().child(position.musical))
                .child(div().text_size(px(11.5)).line_height(px(14.0)).whitespace_nowrap().text_color(th.text_secondary).child(position.time)))
            .when(!compact, |d| d.child(div().w(px(56.0)).h_full().flex_none().flex().flex_col().items_center().justify_center()
                .whitespace_nowrap()
                .child(div().text_size(px(13.0)).line_height(px(16.0)).font_weight(gpui::FontWeight::SEMIBOLD).child(format!("{:.1}", position.bpm)))
                .child(div().text_size(px(11.0)).line_height(px(14.0)).text_color(th.text_secondary).child(format!("BPM · {}", position.signature)))))
            .when(!compact, |d| d.child(crate::proto_controls::action("transport-follow", "跟随播放头").aria_selected(follow)
                .authority(follow_authority.clone())
                .flex_none().h(px(28.0)).px(px(8.0)).flex().items_center().rounded(px(6.0)).text_size(px(12.0))
                .when(follow, |d| d.bg(th.checked))
                .text_color(if follow { th.text } else { th.text_secondary }).hover(move |s| s.bg(if follow { th.checked } else { th.hover })).active(move |s| s.bg(crate::ui_controls::pressed_fill(th, if follow { th.checked } else { th.hover })))
                .child("跟随")
                .on_click(move |event, _, cx| editor.update(cx, |ed, cx| {
                    if crate::proto_controls::editor_authority(ed) != follow_authority
                        || ed.session.follow_playhead != follow { return; }
                    set_follow(ed, event.selected(), cx);
                })))).into_any_element()
    }
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;

pub fn meter_fraction(peak: f32) -> f32 {
    if peak <= 1e-5 { 0.0 } else { ((20.0 * peak.log10() + 60.0) / 60.0).clamp(0.0, 1.0) }
}

#[cfg(test)]
#[path = "meter_navigation_tests.rs"]
mod meter_navigation_tests;

pub struct MeterView { editor: WeakEntity<Editor>, slot: Option<usize>, vertical: bool }
impl Render for MeterView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.editor.upgrade() else { return div().into_any_element() };
        let ed = editor.read(cx);
        let peaks = self.slot.and_then(|i| ed.meters.get(i).copied()).unwrap_or(ed.master_meter);
        if ed.audio.transport() == verdant_audio::Transport::Playing || peaks.0.max(peaks.1) > 1e-4 { window.request_animation_frame(); }
        let (slot, vertical) = (self.slot, self.vertical);
        gpui::canvas(|_, _, _| (), move |bounds, (), window, cx| {
            let ed = editor.read(cx);
            let th = ed.theme;
            let peaks = slot.and_then(|i| ed.meters.get(i).copied()).unwrap_or(ed.master_meter);
            let o = bounds.origin;
            let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
            if vertical {
                let top_pad = 6.0;
                let bar_h = (h - top_pad).max(1.0);
                for (i, v) in [peaks.0, peaks.1].into_iter().enumerate() {
                    let x = w / 2.0 - 8.0 + i as f64 * 9.0;
                    // Top clip LED
                    window.paint_quad(fill(b(x, 0.0, 6.0, 4.0, o), if v >= 0.999 { th.error } else { th.lcd }));
                    window.paint_quad(fill(b(x, top_pad, 6.0, bar_h, o), th.lcd));
                    let f = meter_fraction(v) as f64;
                    let fill_col = if v >= 0.999 { th.error } else if v > 0.707 { th.meter_high } else { th.meter_low };
                    window.paint_quad(fill(b(x, top_pad + bar_h * (1.0 - f), 6.0, bar_h * f, o), fill_col));
                }
            } else {
                window.paint_quad(fill(bounds, th.control));
                window.paint_quad(fill(b(0.0, 0.0, w * meter_fraction(peaks.0.max(peaks.1)) as f64, h, o), th.meter_low));
            }
        }).size_full().into_any_element()
    }
}
