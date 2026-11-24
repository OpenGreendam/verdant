//! Piano roll for the selected track, its keyboard and ruler (SPEC §5).
//!
//! Notes are flat fills in the track's colour with a 1 px darker edge;
//! selection is a light edge, errors a red edge plus a word in the info
//! line — no glows, gradients or hatching. A thin line follows the *model*
//! pitch (base + PIT + vibrato + portamento) where a note carries any; the
//! authored target and measured output F0 are labelled separately.
//! Other-track melodies are optional dimmed fills behind the active notes;
//! they share the viewport transform but never enter the active-track hit index.
//!
//! Input: mouse via the session gesture machine; touch via GPUI's claimed
//! `TouchDragEvent` (drag on a note), `LongPressEvent` (note → lyric,
//! background → marquee), `PinchEvent` (zoom), with finger-sized thresholds.
//! Pencil background drags preview one snapped range and commit on release;
//! clicks, returning to the anchor and cancellation leave no document edit.
//! The pitch pen starts anywhere, previews absolute pitch continuously, and
//! writes note-relative PIT for every crossed note in one release transaction.

use std::{collections::HashMap, ops::Range, sync::Arc};

use gpui::{
    App, BorderStyle, Bounds, CursorStyle, DispatchPhase, Entity, Hitbox, HitboxBehavior, LongPressEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PinchEvent, Pixels, ScrollDelta, ScrollEvent, TouchDragEvent, TouchPhase, Window, canvas, fill,
    prelude::*, px, quad,
};
use verdant_backend::f0::{automatic_transition, base_cents, note_cents_in_span};
use verdant_backend::{CacheEntry, F0Kind, F0Track, RenderScheduler};
use verdant_domain::timing::note_name;
use verdant_domain::{Command, CommandBatch, Note, NoteId, Project, TempoMap, Track, TrackId};
use verdant_session::{CancelReason, Effect, Hit, InputEvent, Modifiers, PointerId, Preview, Rect, Viewport, hit_test_touch};

use crate::editor::{Editor, Tool};
use crate::paint::{Texts, b, flush_texts, is_black_key, polyline};
use crate::theme::{mix, with_alpha};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModHandleKind {
    Portamento,
    Vibrato,
}

#[derive(Clone)]
struct NoteModDrag {
    editor: gpui::EntityId,
    track: TrackId,
    note_id: NoteId,
    base: verdant_domain::Revision,
    kind: ModHandleKind,
    note_start_seconds: f64,
    note_rect_x: f64,
    note_rect_w: f64,
    origin_y: f64,
    initial_depth: f64,
    portamento_ms: u32,
    delay_permille: u32,
    depth_cents: u32,
}

#[derive(Default)]
struct RollExtraDragState {
    mod_drag: Option<NoteModDrag>,
    loop_drag: Option<LoopPreview>,
}
#[derive(Clone,Copy)]
struct LoopPreview { editor:gpui::EntityId, arrange:bool, dragging_start:bool, start:i64, end:i64, snap:bool }
impl gpui::Global for RollExtraDragState {}

pub(crate) fn has_extra_drag(editor: gpui::EntityId, cx: &App) -> bool {
    cx.try_global::<RollExtraDragState>().is_some_and(|s| {
        s.mod_drag.as_ref().is_some_and(|d| d.editor == editor)
            || s.loop_drag.is_some_and(|drag| drag.editor == editor)
    })
}

pub(crate) fn has_note_mod_drag(editor: gpui::EntityId, cx: &App) -> bool {
    cx.try_global::<RollExtraDragState>().and_then(|state| state.mod_drag.as_ref()).is_some_and(|drag| drag.editor == editor)
}

impl Editor {
    fn retire_roll_surface(&mut self, surface: &crate::host_pointer::SurfaceToken, cx: &mut gpui::Context<Self>) {
        if !self.roll_surface.as_ref().is_some_and(|current| current.same(surface)) { return; }
        self.roll_surface = None;
        let project = self.store().snapshot();
        let effects = self.session.handle_pointer(&project, self.revision(), PointerId::Mouse, InputEvent::Cancel(CancelReason::FocusLost));
        self.note_creation = None;
        if self.lane_stroke.as_ref().is_some_and(|stroke| stroke.roll) { self.lane_stroke = None; }
        if has_note_mod_drag(cx.entity_id(), cx) { cx.global_mut::<RollExtraDragState>().mod_drag = None; }
        self.release_roll_capture_deferred(cx);
        self.compiled_pitch_move.cancel_deferred(cx);
        for effect in effects {
            match effect {
                Effect::ReleasePointer => {},
                Effect::Notice(message) => self.notice(message),
                _ => unreachable!("detach only cancels the live roll owner"),
            }
        }
        cx.notify();
    }
}

pub(crate) fn cancel_extra_drag(editor: gpui::EntityId, cx: &mut App) {
    if let Some(state) = cx.try_global::<RollExtraDragState>() {
        let cancel_mod = state.mod_drag.as_ref().is_some_and(|d| d.editor == editor);
        let cancel_loop = state.loop_drag.is_some_and(|drag| drag.editor == editor);
        if cancel_mod {
            cx.global_mut::<RollExtraDragState>().mod_drag = None;
        }
        if cancel_loop {
            cx.global_mut::<RollExtraDragState>().loop_drag = None;
        }
    }
}

fn note_mod_handle_positions(ed: &Editor, n: &Note, r: &Rect, tempo: &TempoMap) -> ((f64, f64), (f64, f64)) {
    let vp = ed.session.viewport;
    let s0 = tempo.seconds_at(n.start_tick as f64);
    let porta_ms = n.portamento_ms.unwrap_or(0);
    let porta_tick = tempo.tick_at(s0 + porta_ms as f64 / 1000.0);
    let porta_x = vp.tick_to_x(porta_tick).clamp(r.x + 4.0, (r.right() - 6.0).max(r.x + 4.0));
    let delay_frac = n.vibrato.as_ref().map_or(0.35, |v| v.delay_permille as f64 / 1000.0).clamp(0.0, 1.0);
    let vib_x = (r.x + r.w * delay_frac).clamp(r.x + 6.0, (r.right() - 4.0).max(r.x + 6.0));
    ((porta_x, r.y + 1.5), (vib_x, r.bottom() - 1.5))
}

/// Verified contours for the current audio timeline. Historical renders can
/// still satisfy undo by digest, but never draw at their outdated coordinates.
#[derive(Default)]
pub struct RenderedPitchCache {
    plan_revision: Option<u64>,
    generation: u64,
    tracks: HashMap<TrackId, PitchSpans>,
    contours: HashMap<String, CachedContour>,
}

struct CachedContour {
    contour: Arc<PitchContour>,
    generation: u64,
}

struct PitchSpan {
    start_seconds: f64,
    end_seconds: f64,
    contour: Arc<PitchContour>,
}

/// Sorted spans with subtree end bounds: one long phrase cannot turn a
/// late, narrow viewport into a scan of every preceding phrase.
#[derive(Default)]
struct PitchSpans {
    spans: Vec<PitchSpan>,
    max_end: Vec<f64>,
}

impl PitchSpans {
    fn index(&mut self) {
        self.spans.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));
        self.max_end.resize(self.spans.len(), f64::NEG_INFINITY);
        fn build(spans: &[PitchSpan], ends: &mut [f64]) -> f64 {
            if spans.is_empty() { return f64::NEG_INFINITY; }
            let mid = spans.len() / 2;
            let (left, rest) = ends.split_at_mut(mid);
            let (root, right) = rest.split_at_mut(1);
            root[0] = spans[mid].end_seconds.max(build(&spans[..mid], left)).max(build(&spans[mid + 1..], right));
            root[0]
        }
        build(&self.spans, &mut self.max_end);
    }

    fn visit_visible(&self, from: f64, to: f64, mut visit: impl FnMut(&PitchSpan)) {
        fn walk(spans: &[PitchSpan], ends: &[f64], from: f64, to: f64, visit: &mut impl FnMut(&PitchSpan)) {
            if spans.is_empty() { return; }
            let mid = spans.len() / 2;
            if ends[mid] < from || spans[0].start_seconds > to { return; }
            walk(&spans[..mid], &ends[..mid], from, to, visit);
            let span = &spans[mid];
            if span.start_seconds <= to && span.end_seconds >= from { visit(span); }
            if span.start_seconds <= to { walk(&spans[mid + 1..], &ends[mid + 1..], from, to, visit); }
        }
        walk(&self.spans, &self.max_end, from, to, &mut visit);
    }
}

struct PitchContour {
    f0: F0Track,
    /// Allows LOD to break at skipped unvoiced/invalid samples without
    /// scanning those samples on every paint or bridging separate syllables.
    unvoiced: Vec<Range<usize>>,
}

impl PitchContour {
    fn new(f0: &F0Track) -> Self {
        let mut unvoiced = Vec::new();
        let mut start = None;
        for (i, &hz) in f0.hz.iter().enumerate() {
            if hz <= 0.0 || !hz.is_finite() {
                if start.is_none() { start = Some(i); }
            } else if let Some(from) = start.take() {
                unvoiced.push(from..i);
            }
        }
        if let Some(from) = start { unvoiced.push(from..f0.hz.len()); }
        Self { f0: f0.clone(), unvoiced }
    }

    fn points<'a>(&'a self, start_seconds: f64, end_seconds: f64, from: f64, to: f64, tempo: &'a TempoMap, vp: Viewport)
        -> impl Iterator<Item = Option<(f64, f64)>> + 'a {
        let hop = self.f0.hop_seconds;
        let valid = hop.is_finite() && hop > 0.0 && !self.f0.hz.is_empty();
        // Use the artifact's real first frame time, not an assumed half hop.
        // Include adjacent points so the viewport clips a crossing segment.
        let origin = start_seconds + self.f0.first_frame_seconds;
        let first = if valid { (((from - origin) / hop).floor().max(0.0) as usize).min(self.f0.hz.len()) } else { 0 };
        let end = if valid { (((to - origin) / hop).ceil().max(0.0) as usize).saturating_add(1).min(self.f0.hz.len()) } else { 0 };
        let last = end.saturating_sub(1);
        // Choose LOD before reading F0 values or constructing a path. Scale
        // the budget by this span's own on-screen width, not the whole
        // viewport: many narrow phrases must not each pay a full-width curve.
        let span_px = (vp.tick_to_x(tempo.tick_at(end_seconds)) - vp.tick_to_x(tempo.tick_at(origin))).max(1.0);
        let stride = last.saturating_sub(first).div_ceil(span_px.ceil().max(1.0) as usize).max(1);
        let indices = std::iter::successors((first < end).then_some(first), move |&i| (i < last).then(|| i.saturating_add(stride).min(last)));
        let mut previous = None;
        indices.flat_map(move |i| {
            let gap = previous.is_some_and(|prev| {
                let run = self.unvoiced.partition_point(|run| run.end <= prev + 1);
                self.unvoiced.get(run).is_some_and(|run| run.start < i)
            });
            previous = Some(i);
            let hz = self.f0.hz[i];
            let point = (hz > 0.0 && hz.is_finite()).then(|| {
                let x = vp.tick_to_x(tempo.tick_at(origin + i as f64 * hop));
                (x, vp.pitch_to_y(69.0 + 12.0 * (hz as f64 / 440.0).log2() + 0.5))
            });
            [gap.then_some(None), Some(point)].into_iter().flatten()
        })
    }
}

impl RenderedPitchCache {
    pub fn rebuild(&mut self, scheduler: &RenderScheduler) {
        self.generation = self.generation.wrapping_add(1);
        for track in self.tracks.values_mut() { track.spans.clear(); }
        for (phrase, entry) in scheduler.playable_phrases() {
            for f0 in &entry.audio.f0_tracks {
                let contour = self.contour(entry, &f0.track);
                self.tracks.entry(phrase.track.clone()).or_default().spans.push(PitchSpan {
                    start_seconds: phrase.start_seconds,
                    end_seconds: phrase.start_seconds + entry.audio.samples.len() as f64 / entry.audio.sample_rate as f64,
                    contour,
                });
            }
        }
        self.tracks.retain(|_, track| !track.spans.is_empty());
        self.contours.retain(|_, contour| contour.generation == self.generation);
        for track in self.tracks.values_mut() { track.index(); }
        self.plan_revision = Some(scheduler.plan_revision);
    }

    fn contour(&mut self, entry: &CacheEntry, f0: &F0Track) -> Arc<PitchContour> {
        let key = format!("{}:{:?}", entry.digest, f0.kind);
        if let Some(cached) = self.contours.get_mut(&key) {
            if !Arc::ptr_eq(&cached.contour.f0.hz, &f0.hz) || cached.contour.f0.hop_seconds != f0.hop_seconds
                || cached.contour.f0.first_frame_seconds != f0.first_frame_seconds {
                cached.contour = Arc::new(PitchContour::new(f0));
            }
            cached.generation = self.generation;
            return cached.contour.clone();
        }
        let contour = Arc::new(PitchContour::new(f0));
        self.contours.insert(key, CachedContour { contour: contour.clone(), generation: self.generation });
        contour
    }

    fn track(&self, track: &TrackId, plan_revision: u64) -> Option<&PitchSpans> {
        (self.plan_revision == Some(plan_revision)).then(|| self.tracks.get(track)).flatten()
    }
}

fn mods(m: &gpui::Modifiers) -> Modifiers {
    Modifiers { shift: m.shift, secondary: m.control || m.platform, alt: m.alt }
}

pub fn piano_roll(editor: Entity<Editor>) -> impl IntoElement {
    let draw=editor.clone();let cancel_editor=editor.clone();
    crate::proto_editor_surface::surface(editor,"piano-roll","钢琴卷帘：音符、音高与表情编辑",0.0,
        move |input|native_piano_roll(draw.clone(),input).into_any_element(),
        move |_,window,cx|cancel_editor.update(cx,|ed,cx| {
            cancel(ed,PointerId::Mouse,CancelReason::PointerCancel,window,cx);
            cancel(ed,PointerId::Touch(0),CancelReason::PointerCancel,window,cx);
            if has_note_mod_drag(cx.entity_id(),cx) { cx.global_mut::<RollExtraDragState>().mod_drag=None; }
            ed.release_roll_capture(window,cx);cx.notify();
        }))
}

fn native_piano_roll(editor: Entity<Editor>, input: crate::proto_editor_surface::Input) -> impl IntoElement {
    let ed_prepaint = editor.clone();
    canvas(
        move |bounds, window, cx| {
            let weak = ed_prepaint.downgrade();
            let key = gpui::SharedString::from(format!("roll-surface-{:?}", ed_prepaint.entity_id()));
            let surface = crate::host_pointer::track_surface(key, window, cx, move |surface, cx| {
                let _ = weak.update(cx, |editor, cx| editor.retire_roll_surface(&surface, cx));
            });
            let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
            ed_prepaint.update(cx, |ed, cx| {
                if let Some(previous) = ed.roll_surface.clone().filter(|previous| !previous.same(&surface)) {
                    ed.retire_roll_surface(&previous, cx);
                }
                ed.roll_surface = Some(surface.clone());
                ed.session.viewport.width = bounds.size.width.as_f32() as f64;
                ed.session.viewport.height = bounds.size.height.as_f32() as f64;
                if ed.needs_fit {
                    ed.needs_fit = false;
                    ed.fit_view();
                }
            });
            (hitbox, surface)
        },
        move |bounds, (hitbox, surface), window, cx| {
            let mut texts = Texts::new();
            paint_roll(&editor, bounds, window, cx, &mut texts);
            flush_texts(window, cx, bounds, texts);
            install_roll_input(&editor,bounds,&hitbox,&surface,&input,window,cx);
        },
    )
    .size_full()
}

/// Paint-only geometry: no note IDs or hitboxes are registered for ghosts.
/// All tracks use document ticks, so the current viewport already supplies
/// the project's PPQ scaling, pan and zoom. Reuse its interval/pitch bounds
/// rather than scanning every note in every background track each frame.
fn other_track_notes<'a>(project: &'a Project, active: &'a TrackId, vp: Viewport, enabled: bool)
    -> impl Iterator<Item = (&'a Track, Rect)> + 'a {
    let (from, to) = vp.visible_ticks();
    let (low, high) = vp.visible_rows();
    project.tracks.iter().filter(move |track| enabled && &track.id != active).flat_map(move |track| {
        track.notes_in_pitch_range(from.floor() as i64, to.ceil() as i64, low, high).filter_map(move |note| {
            let rect = vp.note_rect(note);
            (rect.right() > 0.0 && rect.x < vp.width && rect.bottom() > 0.0 && rect.y < vp.height)
                .then_some((track, rect))
        })
    })
}

fn pitch_note_key(note: &Note) -> (i64, &NoteId) {
    (note.start_tick, &note.id)
}

/// Evaluate glides in the displayed note order, including notes that entered
/// the viewport during a drag and new neighbours across the move boundary.
fn visit_pitch_preview(
    track: &Track,
    moved: &HashMap<&NoteId, Note>,
    from: i64,
    to: i64,
    mut visit: impl FnMut(&Note, Option<&Note>, Option<&Note>),
) {
    let mut projected: Vec<_> = moved.values().collect();
    projected.sort_unstable_by(|a, b| pitch_note_key(a).cmp(&pitch_note_key(b)));

    let visible_committed = track.notes_in_range(from, to).filter(|note| !moved.contains_key(&note.id));
    let visible_projected = projected.iter().copied().filter(|note| note.start_tick < to && note.end_tick() > from);
    for note in visible_committed.chain(visible_projected) {
        let key = pitch_note_key(note);
        let start = track.notes.partition_point(|candidate| pitch_note_key(candidate) < key);
        let end = track.notes.partition_point(|candidate| pitch_note_key(candidate) <= key);
        let prev = track.notes[..start].iter().rev().find(|candidate| !moved.contains_key(&candidate.id));
        let next = track.notes[end..].iter().find(|candidate| !moved.contains_key(&candidate.id));
        let start = projected.partition_point(|candidate| pitch_note_key(candidate) < key);
        let end = projected.partition_point(|candidate| pitch_note_key(candidate) <= key);
        let prev = prev.into_iter().chain(start.checked_sub(1).map(|i| projected[i]))
            .max_by(|a, b| pitch_note_key(a).cmp(&pitch_note_key(b)));
        let next = next.into_iter().chain(projected.get(end).copied())
            .min_by(|a, b| pitch_note_key(a).cmp(&pitch_note_key(b)));
        visit(note, prev, next);
    }
}

#[cfg(test)]
#[path = "roll_other_track_tests.rs"]
mod other_track_tests;

#[cfg(test)]
#[path = "roll_native_tests.rs"]
mod native_tests;

fn paint_roll(editor: &Entity<Editor>, bounds: Bounds<Pixels>, window: &mut Window, cx: &App, texts: &mut Texts) {
    let o = bounds.origin;
    let ed = editor.read(cx);
    let th = ed.theme;
    let vp = ed.session.viewport;
    let project = ed.store().snapshot();
    let Some(track) = project.track(&ed.session.track) else { return };
    let tempo = ed.tempo();
    let (w, h) = (vp.width, vp.height);
    let color = th.track(track.color.as_deref());

    window.paint_quad(fill(bounds, th.roll_white));
    let (lo, hi) = vp.visible_rows();
    for midi in lo..=hi {
        let y = vp.pitch_to_y(midi as f64 + 1.0);
        if is_black_key(midi) {
            window.paint_quad(fill(b(0.0, y, w, vp.row_height, o), th.roll_black));
        }
        if scale_contains(midi, ed.integrations.scale_root, &ed.integrations.scale_intervals) {
            window.paint_quad(fill(b(0., y, w, vp.row_height, o), with_alpha(th.accent, 0.08)));
        }
        if midi.rem_euclid(12) == 0 {
            window.paint_quad(fill(b(0.0, y + vp.row_height - 1.0, w, 1.0, o), th.grid_beat));
        }
    }
    // Grid.
    let (t0, t1) = vp.visible_ticks();
    let meter=ed.meter();
    let beat=meter.beat_ticks_at(t0.max(0.)as i64);let bar=meter.bar_ticks_at(t0.max(0.)as i64);
    let sub=ed.session.snap.grid_ticks.max(1);
    let step=if sub as f64*vp.px_per_tick>=10.{sub}else if beat as f64*vp.px_per_tick>=10.{beat}else{bar};
    if step>=bar { for t in meter.bars(t0.ceil()as i64,t1.ceil()as i64){window.paint_quad(fill(b(vp.tick_to_x(t as f64),0.,1.,h,o),th.grid_bar));} }
    else {for(t,is_bar,is_beat)in meter.grid(t0.ceil()as i64,t1.ceil()as i64,step){let color=if is_bar{th.grid_bar}else if is_beat{th.grid_beat}else{th.grid_sub};window.paint_quad(fill(b(vp.tick_to_x(t as f64),0.,1.,h,o),color));}}

    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        // Background only: active notes and their selection edges paint over
        // these fills. Input below continues to query only the active track.
        for (other, rect) in other_track_notes(&project, &track.id, vp, ed.integrations.other_tracks) {
            let color = with_alpha(th.track(other.color.as_deref()), 0.2);
            window.paint_quad(fill(b(rect.x, rect.y + 1.0, rect.w.max(1.0), rect.h - 2.0, o), color));
        }

        let pitch_preview = ed.session.gesture.as_ref()
            .filter(|g| matches!(g.preview, Preview::Move { .. } | Preview::Resize { .. }));
        let preview_moving = pitch_preview.is_some();
        let projected: HashMap<_, _> = pitch_preview.into_iter().flat_map(|gesture| {
            gesture.ids.iter().filter_map(|id| {
                let note = ed.index.note(track, id)?;
                Some((id, gesture.preview.project_note(note)?))
            })
        }).collect();
        let rubber = match ed.session.gesture.as_ref().map(|g| &g.preview) {
            Some(Preview::RubberBand { hits, .. }) => Some(hits),
            _ => None,
        };
        // Moving notes can enter the viewport from an offscreen pitch.
        // Ordinary paint can prune hidden pitch subtrees at the interval index.
        let (min_pitch, max_pitch) = if preview_moving { (i32::MIN, i32::MAX) } else { (lo, hi) };
        let visible = track.notes_in_pitch_range(
            t0.floor() as i64, (t1.ceil() as i64).saturating_add(1), min_pitch, max_pitch,
        );
        let mut right_by_pitch = [f64::NEG_INFINITY; 128];
        let disjoint_bodies = !preview_moving && visible.clone().all(|note| {
            let rect = vp.note_rect(note);
            let Some(right) = right_by_pitch.get_mut(note.midi_note as usize) else { return false };
            if rect.x < *right || (note.is_muted() && rect.w >= 8.0 && rect.h >= 8.0) {
                return false;
            }
            *right = rect.x + rect.w.max(verdant_session::viewport::MIN_NOTE_BODY_PX);
            true
        });
        let pinned = ed.session.gesture.as_ref().filter(|_| preview_moving).into_iter()
            .flat_map(|gesture| gesture.ids.iter())
            .filter_map(|id| ed.index.note(track, id))
            .filter(|n| n.end_tick() <= t0.floor() as i64 || n.start_tick >= (t1.ceil() as i64).saturating_add(1));
        let paint_notes = |window: &mut Window| for n in visible.chain(pinned) {
            let selected = ed.session.selection.contains(&n.id) || rubber.is_some_and(|r| r.contains(&n.id));
            let error = ed.error_notes.contains(&n.id);
            let preview = projected.get(&n.id);
            if preview_moving && preview.is_some() {
                let r = vp.note_rect(n);
                window.paint_quad(quad(
                    b(r.x, r.y + 1.0, r.w, r.h - 2.0, o),
                    px(3.0),
                    gpui::transparent_black(),
                    px(1.0),
                    with_alpha(color, 0.7),
                    BorderStyle::Dashed,
                ));
            }
            let shown = preview.unwrap_or(n);
            let r = vp.note_rect(shown);
            if r.right() < 0.0 || r.x > w || r.bottom() < 0.0 || r.y > h {
                continue;
            }
            let rendered = ed.ready_notes.contains(&n.id);
            let body = if n.is_muted() { mix(color, th.roll_white, 0.75) } else if rendered { color } else { mix(color, th.roll_white, 0.35) };
            let edge = if error {
                th.error
            } else if selected {
                if th.dark { crate::theme::white() } else { crate::theme::black() }
            } else {
                mix(color, crate::theme::black(), 0.35)
            };
            let editing = ed.lyric.as_ref().is_some_and(|l| l.note_id == n.id);
            window.paint_quad(quad(
                b(r.x, r.y + 1.0, r.w.max(verdant_session::viewport::MIN_NOTE_BODY_PX), r.h - 2.0, o),
                px(3.0),
                body,
                px(if selected || error { 1.5 } else { 1.0 }),
                edge,
                BorderStyle::Solid,
            ));
            if n.is_muted() && r.w >= 8.0 && r.h >= 8.0 {
                let left = r.x + 3.0;
                let right = (left + 6.0).min(r.right() - 2.0);
                let middle = r.y + r.h / 2.0;
                let radius = ((r.h - 4.0) / 2.0).min(3.0);
                for (top, bottom) in [(middle - radius, middle + radius), (middle + radius, middle - radius)] {
                    polyline(window, o, [Some((left, top)), Some((right, bottom))], 1.5, th.text_secondary);
                }
            }
            if r.w >= 24.0 && vp.row_height >= 12.0 && !editing {
                let fs = (vp.row_height * 0.6).clamp(9.0, 14.0) as f32;
                let ty = r.y + (r.h - fs as f64 * 1.25) / 2.0;
                let mut label = if n.lyric.is_empty() { "·".to_string() } else { n.lyric.clone() };
                if r.w > 44.0 {
                    let reading = verdant_domain::pronunciation_display(&n.pronunciation);
                    if !reading.is_empty() && reading != n.lyric { label = format!("{label}  {}", reading); }
                }
                let inset = if n.is_muted() { 14.0 } else { 5.0 };
                texts.push((label, gpui::point(o.x + px(r.x as f32 + inset), o.y + px(ty as f32)), fs, if n.is_muted() { th.text_secondary } else { th.on_track() }));
            }
        };
        if disjoint_bodies {
            window.paint_layer(paint_notes);
        } else {
            paint_notes(window);
        }

        if let Some(creation)=&ed.note_creation {
            let r=vp.note_rect(&creation.note);
            let width = if creation.note.duration_tick == 0 { ed.session.snap.grid_ticks.max(1) as f64 * vp.px_per_tick } else { r.w };
            window.paint_quad(quad(b(r.x,r.y+1.,width.max(2.),r.h-2.,o),px(3.),with_alpha(color,0.65),px(1.5),th.accent,BorderStyle::Solid));
        }

        // Same automatic / authored contour as the phrase control track.
        let line_color = if th.dark { with_alpha(crate::theme::white(), 0.85) } else { with_alpha(crate::theme::black(), 0.75) };
        let mut model_contour = Vec::new();
        let mut append_contour = |n: &Note, prev: Option<&Note>, next: Option<&Note>| {
            if n.is_muted() { return; }
            let x0 = vp.tick_to_x(n.start_tick as f64).max(0.0);
            let x1 = vp.tick_to_x(n.end_tick() as f64).min(w);
            // A sub-pixel line cannot render its curve; the note fill already
            // marks coverage. Below that, skip notes whose pitch cannot reach
            // the viewport vertically (PIT bounds + vibrato + neighbour glides).
            if x1 <= 0.0 || x0 >= w || x1 - x0 < 4.0 {
                return;
            }
            if !n.has_f0_modulation()
                && !prev.is_some_and(|p| automatic_transition(p, n, &tempo).is_some())
                && !next.is_some_and(|p| automatic_transition(n, p, &tempo).is_some()) {
                return;
            }
            if !pitch_line_visible(n, prev, next, vp, h) {
                return;
            }
            let s = tempo.seconds_at(n.start_tick as f64);
            let e = tempo.seconds_at(n.end_tick() as f64);
            let steps = ((x1 - x0) / 2.0).ceil().max(2.0) as usize;
            let pts = (0..=steps).map(|k| {
                let x = x0 + (x1 - x0) * k as f64 / steps as f64;
                let t = tempo.seconds_at(vp.x_to_tick(x)).clamp(s, e - 1e-6);
                note_cents_in_span(n, prev, next, &tempo, t, s, e).map(|c| (x, vp.pitch_to_y(c / 100.0 + 0.5)))
            });
            model_contour.push(None);
            model_contour.extend(pts);
        };
        let (from, to) = (t0.floor() as i64, (t1.ceil() as i64).saturating_add(1));
        if preview_moving {
            visit_pitch_preview(track, &projected, from, to, append_contour);
        } else {
            for note in ed.index.notes_in_range(track, from, to) {
                let position = ed.index.position(&note.id);
                let prev = position.and_then(|i| i.checked_sub(1)).map(|i| &track.notes[i]);
                let next = position.and_then(|i| track.notes.get(i + 1));
                append_contour(note, prev, next);
            }
        }
        polyline(window, o, model_contour, 1.5, line_color);

        // Separate controls from output estimates. WORLD estimates can have
        // octave/voicing errors; none of these curves is labelled ground truth.
        if ed.show_f0 && !preview_moving {
            if let Some(pitch) = ed.roll_pitch.track(&track.id, ed.revision().get()) {
                let from_seconds = tempo.seconds_at(vp.x_to_tick(-4.0));
                let to_seconds = tempo.seconds_at(vp.x_to_tick(w + 4.0));
                let mut visible_kinds = [false; 3];
                pitch.visit_visible(from_seconds, to_seconds, |span| {
                    let contour = &span.contour;
                    let (slot, color, width) = match contour.f0.kind {
                        F0Kind::NativeTarget => (0, th.text_secondary, 1.0),
                        F0Kind::NeuralSupplied => (1, th.accent, 1.0),
                        F0Kind::NeuralEstimate => (2, th.warning, 1.5),
                    };
                    visible_kinds[slot] = true;
                    polyline(window, o, contour.points(span.start_seconds, span.end_seconds, from_seconds, to_seconds, &tempo, vp), width, with_alpha(color, 0.85));
                });
                let mut row = 0;
                for (slot, kind, color) in [(0, F0Kind::NativeTarget, th.text_secondary), (1, F0Kind::NeuralSupplied, th.accent), (2, F0Kind::NeuralEstimate, th.warning)] {
                    if visible_kinds[slot] {
                        texts.push((kind.label().to_string(), gpui::point(o.x + px(8.0), o.y + px(7.0 + row as f32 * 15.0)), 10.0, color));
                        row += 1;
                    }
                }
            }
        }

        if let Some(Preview::RubberBand { from, to, .. }) = ed.session.gesture.as_ref().map(|g| &g.preview) {
            let (x0, x1) = (vp.tick_to_x(from.tick.min(to.tick)), vp.tick_to_x(from.tick.max(to.tick)));
            let (y0, y1) = (vp.pitch_to_y(from.pitch.max(to.pitch)), vp.pitch_to_y(from.pitch.min(to.pitch)));
            window.paint_quad(quad(b(x0, y0, x1 - x0, y1 - y0, o), px(0.0), th.selection, px(1.0), th.accent, BorderStyle::Solid));
        }

        if let Some(stroke) = ed.lane_stroke.as_ref().filter(|s| s.roll) {
            polyline(window, o, stroke.samples.iter().map(|(tick, pitch)| Some((vp.tick_to_x(*tick), vp.pitch_to_y(*pitch)))), 2.0, th.accent);
            if let Some((tick, pitch)) = stroke.samples.last() {
                window.paint_quad(fill(b(vp.tick_to_x(*tick)-2.0, vp.pitch_to_y(*pitch)-2.0, 4.0, 4.0, o), th.accent));
            }
        }

        // On-note SVS Portamento (top-left diamond) and Vibrato (bottom circle) handles on selected notes.
        let active_mod = cx.try_global::<RollExtraDragState>()
            .and_then(|s| s.mod_drag.as_ref())
            .filter(|d| d.test_matches(ed));
        if ed.tool == Tool::Pointer && ed.session.selection.len() <= 4 {
            for sel_id in &ed.session.selection {
                let Some(n) = ed.index.note(track, sel_id) else { continue };
                if n.is_muted() { continue; }
                let r = vp.note_rect(n);
                if r.right() < 0.0 || r.x > w || r.w < 26.0 || r.h < 10.0 { continue; }
                let (mut p_pos, mut v_pos) = note_mod_handle_positions(ed, n, &r, &tempo);
                if let Some(drag) = active_mod.filter(|d| &d.note_id == sel_id) {
                    match drag.kind {
                        ModHandleKind::Portamento => {
                            let s0 = tempo.seconds_at(n.start_tick as f64);
                            let pt = tempo.tick_at(s0 + drag.portamento_ms as f64 / 1000.0);
                            p_pos.0 = vp.tick_to_x(pt).clamp(r.x + 4.0, (r.right() - 6.0).max(r.x + 4.0));
                            texts.push((
                                format!("滑音 {} ms", drag.portamento_ms),
                                gpui::point(o.x + px(p_pos.0 as f32 + 6.0), o.y + px((p_pos.1 - 14.0).max(2.0) as f32)),
                                10.0,
                                th.warning,
                            ));
                        }
                        ModHandleKind::Vibrato => {
                            v_pos.0 = (r.x + r.w * (drag.delay_permille as f64 / 1000.0)).clamp(r.x + 6.0, (r.right() - 4.0).max(r.x + 6.0));
                            texts.push((
                                format!("颤音 {}¢ · 起始 {:.0}%", drag.depth_cents, drag.delay_permille as f64 / 10.0),
                                gpui::point(o.x + px(v_pos.0 as f32 + 6.0), o.y + px((v_pos.1 + 4.0).min(h - 14.0) as f32)),
                                10.0,
                                th.accent,
                            ));
                        }
                    }
                }
                window.paint_quad(quad(
                    b(p_pos.0 - 3.5, p_pos.1 - 3.5, 7.0, 7.0, o),
                    px(1.5),
                    th.warning,
                    px(1.0),
                    th.surface,
                    BorderStyle::Solid,
                ));
                window.paint_quad(quad(
                    b(v_pos.0 - 4.0, v_pos.1 - 4.0, 8.0, 8.0, o),
                    px(4.0),
                    if n.vibrato.is_some() { th.accent } else { th.control_pressed },
                    px(1.0),
                    th.playhead,
                    BorderStyle::Solid,
                ));
            }
        }
    });

    if track.notes.is_empty() {
        texts.push((
            "用铅笔工具（2）拖动绘制音符，或双击空白处添加音符".into(),
            gpui::point(o.x + px(20.0), o.y + px(18.0)),
            13.0,
            th.text_tertiary,
        ));
    }
}

/// Conservative vertical reach of a note's model pitch line, in semitones.
/// True when any drawn sample could land inside the viewport; false means
/// the curve provably stays outside, so painting it is free to skip. The
/// bound mirrors the transitions in `verdant_backend::f0`: authored PIT
/// extent, detune, vibrato depth, plus neighbour base pitch when a glide
/// (automatic transition or explicit portamento) can form at either edge.
fn pitch_line_visible(n: &Note, prev: Option<&Note>, next: Option<&Note>, vp: Viewport, height: f64) -> bool {
    let mut lo = base_cents(n);
    let mut hi = lo;
    for point in &n.curves.pitch_millicent_points {
        let v = base_cents(n) + point.millicents as f64 / 1000.0;
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if let Some(v) = n.vibrato.as_ref().filter(|v| v.depth_cents > 0) {
        lo -= v.depth_cents as f64;
        hi += v.depth_cents as f64;
    }
    let glides_into = |a: &Note, b: &Note| {
        !a.is_muted() && !b.is_muted() && a.end_tick() == b.start_tick && b.portamento_ms.is_none()
            && a.portamento_ms.is_none_or(|ms| ms == 0)
            && a.curves.pitch_millicent_points.is_empty() && b.curves.pitch_millicent_points.is_empty()
            && !a.vibrato.as_ref().is_some_and(|v| v.depth_cents > 0)
            && !b.vibrato.as_ref().is_some_and(|v| v.depth_cents > 0)
            && (base_cents(a) - base_cents(b)).abs() >= 1e-9
    };
    let explicit_into = |a: &Note, b: &Note| {
        !a.is_muted() && !b.is_muted() && a.end_tick() >= b.start_tick
            && b.portamento_ms.is_some_and(|ms| ms > 0)
    };
    for (a, b) in prev.map(|p| (p, n)).into_iter().chain(next.map(|q| (n, q))) {
        if glides_into(a, b) || explicit_into(a, b) {
            lo = lo.min(base_cents(a)).min(base_cents(b));
            hi = hi.max(base_cents(a)).max(base_cents(b));
        }
    }
    let top = vp.pitch_to_y(hi / 100.0 + 0.5);
    let bottom = vp.pitch_to_y(lo / 100.0 + 0.5);
    bottom >= -4.0 && top <= height + 4.0
}

impl NoteModDrag {
    fn test_matches(&self, ed: &Editor) -> bool {
        self.track == ed.session.track && self.base == ed.revision()
    }

    fn sample_values(&self, ed: &Editor, x: f64, y: f64) -> (u32, u32, u32) {
        let tick = ed.session.viewport.x_to_tick(x).max(0.0);
        let sec = ed.tempo().seconds_at(tick);
        (
            ((sec - self.note_start_seconds).max(0.0) * 1000.0).round().clamp(0.0, 2000.0) as u32,
            (((x - self.note_rect_x) / self.note_rect_w.max(1.0)).clamp(0.0, 1.0) * 1000.0).round() as u32,
            (self.initial_depth + (self.origin_y - y) * 1.5).round().clamp(0.0, 200.0) as u32,
        )
    }
}

fn hit_mod_handle(ed: &Editor, x: f64, y: f64) -> Option<NoteModDrag> {
    if ed.tool != Tool::Pointer || ed.session.selection.len() > 4 {
        return None;
    }
    let track = ed.active_track()?;
    let tempo = ed.tempo();
    let vp = ed.session.viewport;
    for sel_id in &ed.session.selection {
        let Some(n) = ed.index.note(track, sel_id) else { continue };
        if n.is_muted() { continue; }
        let r = vp.note_rect(n);
        if r.w < 26.0 || r.h < 10.0 { continue; }
        let (p_pos, v_pos) = note_mod_handle_positions(ed, n, &r, &tempo);
        if (x - p_pos.0).abs() <= 6.0 && (y - p_pos.1).abs() <= 6.0 {
            return Some(NoteModDrag {
                editor: gpui::EntityId::from(0),
                track: ed.session.track.clone(),
                note_id: n.id.clone(),
                base: ed.revision(),
                kind: ModHandleKind::Portamento,
                note_start_seconds: tempo.seconds_at(n.start_tick as f64),
                note_rect_x: r.x,
                note_rect_w: r.w,
                origin_y: y,
                initial_depth: n.vibrato.as_ref().map_or(35.0, |v| v.depth_cents as f64),
                portamento_ms: n.portamento_ms.unwrap_or(0),
                delay_permille: n.vibrato.as_ref().map_or(350, |v| v.delay_permille),
                depth_cents: n.vibrato.as_ref().map_or(35, |v| v.depth_cents),
            });
        }
        if (x - v_pos.0).abs() <= 6.5 && (y - v_pos.1).abs() <= 6.5 {
            let init_depth = n.vibrato.as_ref().map_or(35, |v| v.depth_cents);
            let init_delay = n.vibrato.as_ref().map_or(350, |v| v.delay_permille);
            return Some(NoteModDrag {
                editor: gpui::EntityId::from(0),
                track: ed.session.track.clone(),
                note_id: n.id.clone(),
                base: ed.revision(),
                kind: ModHandleKind::Vibrato,
                note_start_seconds: tempo.seconds_at(n.start_tick as f64),
                note_rect_x: r.x,
                note_rect_w: r.w,
                origin_y: y,
                initial_depth: init_depth as f64,
                portamento_ms: n.portamento_ms.unwrap_or(0),
                delay_permille: init_delay,
                depth_cents: init_depth,
            });
        }
    }
    None
}

fn cancel_roll_mouse(ed: &mut Editor, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    let owns_mouse = ed.session.pointer_owner() == Some(PointerId::Mouse)
        || ed.note_creation.as_ref().is_some_and(|creation| creation.pointer == PointerId::Mouse)
        || ed.lane_stroke.as_ref().is_some_and(|stroke| stroke.roll && stroke.pointer == PointerId::Mouse)
        || has_note_mod_drag(cx.entity_id(), cx);
    if owns_mouse { ed.cancel_edits(CancelReason::PointerCancel, window, cx); }
}

fn install_roll_input(editor: &Entity<Editor>, bounds: Bounds<Pixels>, hitbox: &Hitbox,
    surface: &crate::host_pointer::SurfaceToken, input: &crate::proto_editor_surface::Input, window: &mut Window, cx: &mut App) {
    if cx.try_global::<RollExtraDragState>().is_none() {
        cx.set_global(RollExtraDragState::default());
    }
    {
        let ed = editor.read(cx);
        if hitbox.is_hovered(window) && ed.session.gesture.is_none() {
            let m = window.mouse_position() - bounds.origin;
            let (mx, my) = (m.x.as_f32() as f64, m.y.as_f32() as f64);
            if hit_mod_handle(ed, mx, my).is_some() {
                window.set_cursor_style(CursorStyle::PointingHand, hitbox);
            } else if let Some(track) = ed.active_track() {
                let cursor =
                    match (ed.tool, ed.index.hit_test(track, &ed.session.viewport, &ed.session.selection, mx, my, 7.0, 4.0))
                    {
                        (_, Hit::Handle(..)) => CursorStyle::ResizeLeftRight,
                        (Tool::Pencil | Tool::Eraser | Tool::Pitch, _) => CursorStyle::Crosshair,
                        _ => CursorStyle::Arrow,
                    };
                window.set_cursor_style(cursor, hitbox);
            }
        }
    }

    let (e,h,live,policy)=(editor.clone(),hitbox.clone(),surface.clone(),input.clone());
    crate::proto_editor_surface::install_pointer_input(input,bounds,hitbox,window,move |event,window,cx| {
        use crate::proto_editor_surface::PointerPhase;
        if !live.is_live() { return false; }
        match event.phase {
            PointerPhase::Begin => {
                e.update(cx,|ed,cx| pointer_down(ed,event.pointer,event.x,event.y,mods(&event.modifiers),
                    event.clicks as u32,&h,&policy,window,cx))
            }
            PointerPhase::Sample => {
                let values=cx.try_global::<RollExtraDragState>().and_then(|s|s.mod_drag.as_ref())
                    .filter(|d|d.editor==e.entity_id()).map(|d|d.sample_values(e.read(cx),event.x,event.y));
                if let Some((portamento,delay,depth))=values {
                    if let Some(drag)=cx.global_mut::<RollExtraDragState>().mod_drag.as_mut() {
                        drag.portamento_ms=portamento;drag.delay_permille=delay;drag.depth_cents=depth;
                    }
                    e.update(cx,|_,cx|cx.notify());
                } else { e.update(cx,|ed,cx| {
                    if let Some(intent)=policy.motion(ed,event.x,event.y,event.modifiers,window,cx) {
                        pointer_move(ed,event.pointer,event.x,event.y,mods(&event.modifiers),&intent,&policy,window,cx);
                    }
                }); }
                true
            }
            PointerPhase::Finish => {
                if has_note_mod_drag(e.entity_id(),cx) { finish_note_modulation(&e,window,cx); }
                else { e.update(cx,|ed,cx|pointer_up(ed,event.pointer,event.x,event.y,mods(&event.modifiers),window,cx)); }
                true
            }
            PointerPhase::Context => { e.update(cx,|ed,cx|roll_context_menu(ed,event.position,event.x,event.y,window,cx));false }
        }
    });

    let h = hitbox.clone();
    let e = editor.clone();
    let long_surface = surface.clone();
    let gate=input.clone();
    window.on_mouse_event(move |ev: &LongPressEvent, phase, window, cx| {
        let expected_phase = if ev.phase == TouchPhase::Started { DispatchPhase::Bubble } else { DispatchPhase::Capture };
        if phase != expected_phase || !long_surface.is_live() {
            return;
        }
        if !crate::host_pointer::contains_window_point(window, (f64::from(ev.position.x), f64::from(ev.position.y))) {
            gate.cancel_pointer(PointerId::Touch(0),window,cx);
            return;
        }
        let l = ev.position - bounds.origin;
        let (x, y) = (l.x.as_f32() as f64, l.y.as_f32() as f64);
        match ev.phase {
            TouchPhase::Started => {
                if !h.is_hovered_at(ev.start_position, window) {
                    return;
                }
                let ed = e.read(cx);
                if ed.has_proto_pointer_contact() || ed.device_drag.is_some() || ed.note_creation.is_some()
                    || ed.mixer_drag.is_some() || has_note_mod_drag(e.entity_id(), cx) { return; }
                if !gate.begin(PointerId::Touch(0),x,y,window,cx) { return; }
                window.prevent_default();
                e.update(cx, |ed, cx| {
                    if ed.session.pointer_owner().is_some_and(|p| p != PointerId::Touch(0)) || ed.lane_stroke.is_some() { return; }
                    let p = ed.store().snapshot();
                    let Some(t) = p.track(&ed.session.track) else { return };
                    let hit=hit_test_touch(t, &ed.session.viewport, &ed.session.selection, x, y);
                    let Some(intent)=gate.intent(ed,"noteGesture",vec![note_tool(ed.tool).into(),
                        (!matches!(hit,Hit::Background)).into(),true.into(),true.into(),false.into(),false.into()],window,cx) else { return; };
                    match intent.kind.as_str() {
                        "note-lyric" => if let Hit::Body(id) | Hit::Handle(id, _) = hit { ed.begin_lyric(id, window, cx); },
                        "note-marquee" => {
                            let music = ed.session.viewport.to_music(x, y);
                            let fx = ed.session.handle_pointer(
                                &p,
                                ed.revision(),
                                PointerId::Touch(0),
                                InputEvent::PointerDown {
                                    hit: Hit::Background,
                                    music,
                                    screen: (x, y),
                                    mods: Modifiers::default(),
                                    click_count: 1,
                                },
                            );
                            ed.apply_pointer_effects(fx, h.id, window, cx);
                        }
                        _ => {},
                    }
                });
                if e.read(cx).session.pointer_owner()!=Some(PointerId::Touch(0)) { gate.cancel_pointer(PointerId::Touch(0),window,cx); }
            }
            TouchPhase::Moved => { if !gate.sample(PointerId::Touch(0),x,y,window,cx) { return; } e.update(cx, |ed, cx| {
                if matches!(ed.session.pointer_owner(), Some(PointerId::Touch(_))) {
                    if let Some(intent)=gate.motion(ed,x,y,gpui::Modifiers::default(),window,cx) {
                        pointer_move(ed, PointerId::Touch(0), x, y, Modifiers::default(),&intent,&gate,window,cx);
                    }
                }
            }); },
            TouchPhase::Ended => {
                if gate.sample(PointerId::Touch(0),x,y,window,cx) { e.update(cx,|ed,cx| {
                    if let Some(intent)=gate.motion(ed,x,y,gpui::Modifiers::default(),window,cx) {
                        pointer_move(ed,PointerId::Touch(0),x,y,Modifiers::default(),&intent,&gate,window,cx);
                    }
                }); }
                if !gate.finish(PointerId::Touch(0),x,y,window,cx) { return; } e.update(cx, |ed, cx| {
                if matches!(ed.session.pointer_owner(), Some(PointerId::Touch(_))) {
                    pointer_up(ed, PointerId::Touch(0), x, y, Modifiers::default(), window, cx);
                }
            }); },
            TouchPhase::Cancelled => gate.cancel_pointer(PointerId::Touch(0),window,cx),
        }
    });
    let h = hitbox.clone();
    let e = editor.clone();
    let gate=input.clone();
    window.on_mouse_event(move |ev: &PinchEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !h.is_hovered(window) {
            return;
        }
        let Some(intent) = gate.pinch(1.0+f64::from(ev.delta),window,cx) else {return;};
        let l = ev.position - bounds.origin;
        e.update(cx, |ed, cx| {
            if intent.apply(&mut ed.session.viewport, (f64::from(l.x), f64::from(l.y))) { ed.notify_viewport(crate::clock::Space::Editor, cx); }
        });
        cx.stop_propagation();
    });
    install_roll_scroll(editor,bounds,hitbox,ScrollSurface::Roll,input,window);
}

fn finish_note_modulation(editor:&Entity<Editor>,window:&mut Window,cx:&mut App) {
    let Some(drag)=cx.global_mut::<RollExtraDragState>().mod_drag.take() else { return };
    editor.update(cx,|ed,cx| {
        ed.release_roll_capture(window,cx);
        if !drag.test_matches(ed) { cx.notify();return; }
        let ids:Vec<NoteId>=if ed.session.selection.contains(&drag.note_id) { ed.session.selection_in_order(ed.project()) } else { vec![drag.note_id] };
        match drag.kind {
            ModHandleKind::Portamento => { ed.commit(drag.base,CommandBatch::single("拖拽调整滑音时长",Command::SetPortamento {
                track:drag.track,note_ids:ids,portamento_ms:Some(drag.portamento_ms) }),cx); }
            ModHandleKind::Vibrato => {
                let mut notes=ed.selected_notes();
                for note in &mut notes {
                    if drag.depth_cents==0 { note.vibrato=None; }
                    else { let mut vibrato=note.vibrato.clone().unwrap_or(verdant_domain::Vibrato::DEFAULT);
                        vibrato.depth_cents=drag.depth_cents;vibrato.delay_permille=drag.delay_permille;note.vibrato=Some(vibrato); }
                }
                if !notes.is_empty() { ed.commit(drag.base,CommandBatch::single("拖拽调整音符颤音",Command::ReplaceNotes { track:drag.track,notes }),cx); }
            }
        }
        cx.notify();
    });
}

fn note_tool(tool:Tool)->&'static str {
    match tool { Tool::Pointer=>"pointer",Tool::Pencil=>"pencil",Tool::Eraser=>"eraser",Tool::Pitch=>"pitch" }
}

fn begin_pitch_stroke(
    ed: &mut Editor, pointer: PointerId, x: f64, y: f64, window: &mut Window,
    cx: &mut gpui::Context<Editor>,
) -> bool {
    if ed.note_creation.is_some() || ed.session.pointer_owner().is_some()
        || ed.lane_stroke.is_some() || ed.mixer_drag.is_some() || ed.device_drag.is_some() || ed.has_proto_pointer_contact()
    {
        return false;
    }
    if ed.lyric.is_some() {
        ed.end_lyric(true, None, window, cx);
        if ed.lyric.is_some() { return false; }
    }
    if ed.active_track().is_none() { return false; }
    let music = ed.session.viewport.to_music(x, y);
    let started = ed.begin_curve(verdant_backend::params::Param::Pit, pointer, None, None, true,
        music.tick.max(0.0), music.pitch, (x, y), false,crate::editor::LaneTool::Pencil);
    if started { cx.notify(); }
    started
}

#[allow(clippy::too_many_arguments)]
fn pointer_down(
    ed: &mut Editor,
    pointer: PointerId,
    x: f64,
    y: f64,
    m: Modifiers,
    clicks: u32,
    h: &Hitbox,
    input:&crate::proto_editor_surface::Input,
    window: &mut Window,
    cx: &mut gpui::Context<Editor>,
) -> bool {
    if ed.note_creation.is_some() || ed.session.pointer_owner().is_some() || ed.lane_stroke.is_some()
        || ed.mixer_drag.is_some() || ed.device_drag.is_some() || ed.has_proto_pointer_contact()
        || has_note_mod_drag(cx.entity_id(),cx) { return false; }
    if ed.lyric.is_some() {
        ed.end_lyric(true, None, window, cx);
        if ed.lyric.is_some() { return false; }
    }
    let p = ed.store().snapshot();
    let Some(track) = p.track(&ed.session.track) else { return false; };
    // Bounded candidate scan through the note index; touch gets the
    // finger-sized thresholds.
    let (handle_px, outside_px) = match pointer {
        PointerId::Touch(_) => (22.0, 14.0),
        _ => (7.0, 4.0),
    };
    let hit = ed
        .index
        .hit_test(track, &ed.session.viewport, &ed.session.selection, x, y, handle_px, outside_px);
    let mod_handle=hit_mod_handle(ed,x,y);
    let Some(intent)=input.intent(ed,"noteGesture",vec![note_tool(ed.tool).into(),
        (!matches!(hit,Hit::Background)).into(),matches!(pointer,PointerId::Touch(_)).into(),false.into(),
        mod_handle.is_some().into(),m.alt.into()],window,cx) else { return false; };
    let music = ed.session.viewport.to_music(x, y);
    match intent.kind.as_str() {
        "pitch-draw" => {
            if intent.selection=="replace" && let Hit::Body(id) | Hit::Handle(id,_) = &hit { ed.session.selection=[id.clone()].into(); }
            let started=begin_pitch_stroke(ed,pointer,x,y,window,cx);
            if started && pointer==PointerId::Mouse { ed.capture_roll_pointer(h.id,window,cx); }
            return started;
        }
        "note-modulation" => {
            let Some(mut drag)=mod_handle else { return false; };
            drag.editor=cx.entity_id();cx.global_mut::<RollExtraDragState>().mod_drag=Some(drag);
            ed.capture_roll_pointer(h.id,window,cx);cx.notify();return true;
        }
        "erase-note" => {
            let (Hit::Body(id) | Hit::Handle(id,_))=&hit else { return false; };
            let cmd = Command::DeleteNotes { track: ed.session.track.clone(), note_ids: vec![id.clone()] };
            ed.session.selection.remove(id);
            ed.commit(ed.revision(), CommandBatch::single("擦除音符", cmd), cx);
            return false;
        }
        "draw-note" => {
            let meter = ed.meter();
            let origin = meter.signature_at(music.tick.max(0.) as i64).tick;
            let start = if intent.snap { origin + ed.session.snap.snap(music.tick-origin as f64) } else { music.tick.round() as i64 }.max(0);
            // Zero is an armed preview only; finish_note_draw refuses it.
            let note = Note::new(start, 0, (music.pitch.floor() as i32).clamp(0,127));
            ed.note_creation = Some(crate::editor::workflow::NoteCreation { pointer, base: ed.revision(), origin_tick: start, note });
            if pointer == PointerId::Mouse { ed.capture_roll_pointer(h.id, window, cx); }
            cx.notify(); return true;
        }
        "note-edit" => {},
        _ => return false,
    }
    let fx =
        ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::PointerDown { hit, music, screen: (x, y), mods: m, click_count: clicks });
    ed.apply_pointer_effects(fx, h.id, window, cx);
    ed.session.pointer_owner()==Some(pointer)
}

fn pointer_move(ed: &mut Editor, pointer: PointerId, mut x: f64, y: f64, m: Modifiers,
    intent:&crate::proto_editor_surface::MotionIntent,input:&crate::proto_editor_surface::Input,
    window: &mut Window, cx: &mut gpui::Context<Editor>) {
    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer) {
        let tick = ed.session.viewport.x_to_tick(x);
        let origin = ed.meter().signature_at(tick.max(0.) as i64).tick;
        let snapped = if intent.snap { origin + ed.session.snap.snap(tick - origin as f64) } else { tick.round() as i64 };
        let minimum = if intent.snap { ed.session.snap.grid_ticks.max(1) } else { 1 };
        if let Some(creation) = &mut ed.note_creation {
            verdant_domain::editing::preview_note_draw(&mut creation.note, creation.origin_tick, snapped, minimum);
        }
        cx.notify();return;
    }
    if ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer && s.roll) {
        let tick = ed.session.viewport.x_to_tick(x);
        let pitch = ed.session.viewport.y_to_pitch(y);
        ed.roll_pitch_sample(pointer, tick, pitch, &intent.axis); cx.notify();
        return;
    }
    if ed.session.gesture.is_none() {
        return;
    }
    if let Some(pan)=input.edge_pan(ed,x,ed.session.viewport.width,window,cx) {
        ed.session.viewport.pan_px(pan.delta,0.0);
        ed.session.autoscroll_active=pan.active;
        x=pan.position;
    }
    let music = ed.session.viewport.to_music(x, y);
    let p = ed.store().snapshot();
    let fx = ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::PointerMove { music, screen: (x, y), mods: m });
    ed.apply_effects(fx, window, cx);
}

#[allow(clippy::too_many_arguments)]
fn pointer_up(ed: &mut Editor, pointer: PointerId, x: f64, y: f64, mods: Modifiers, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    if !x.is_finite() || !y.is_finite() {
        cancel(ed, pointer, CancelReason::PointerCancel, window, cx);
        ed.notice("卷帘终止坐标无效；未提交修改");
        cx.notify();
        return;
    }
    // The physical adapter supplies the last sample before source finish.
    // A completed source lease must never be sampled again by its host.
    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer){
        let creation = ed.note_creation.take().expect("owned creation");
        ed.release_roll_capture(window, cx);
        let id = creation.note.id.clone();
        if let Some(batch) = verdant_domain::editing::finish_note_draw(ed.session.track.clone(), Some(creation.note)) {
            if ed.commit(creation.base, batch, cx) { ed.session.selection = [id].into(); }
        }
        cx.notify();
        return;
    }
    if ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer && s.roll) {
        ed.release_roll_capture(window, cx); ed.commit_roll_pitch(cx); return;
    }
    let p = ed.store().snapshot();
    let music = ed.session.viewport.to_music(x, y);
    let fx = ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::PointerUpAt { music, screen: (x, y), mods });
    ed.apply_effects(fx, window, cx);
}

fn cancel(ed: &mut Editor, pointer: PointerId, reason: CancelReason, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    let p = ed.store().snapshot();
    let owns = ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer) || ed.session.pointer_owner() == Some(pointer) || ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer);
    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer){ed.note_creation=None;}
    if !owns { return; }
    if ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer) { ed.lane_stroke = None; }
    let fx = ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::Cancel(reason));
    ed.release_roll_capture(window, cx);
    ed.apply_effects(fx, window, cx);
}

/// Right-click in the roll: note menu on a note, background menu otherwise.
/// Anchored at the pointer (window coordinates); items feed
/// `Editor::run_menu_action`.
/// Right-click in the roll. On a note: act on it (it joins or becomes the
/// selection first, as in every desktop editor). On empty space: actions at
/// that time and pitch, then tools.
fn roll_context_menu(ed: &mut Editor, position: gpui::Point<Pixels>, x: f64, y: f64, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    use crate::editor::{MenuAction as A, WorkflowAction as W};
    use crate::menu::MenuItem;
    let project = ed.store().snapshot();
    let Some(track) = project.track(&ed.session.track) else { return };
    let vp = &ed.session.viewport;
    let hit = ed.index.hit_test(track, vp, &ed.session.selection, x, y, 7.0, 4.0);
    let music = vp.to_music(x, y);
    let tick = music.tick.floor().max(0.0) as i64;
    let snapped = ed.session.snap.snap(music.tick).max(0);
    if let Hit::Body(id) | Hit::Handle(id, _) = &hit { if !ed.session.selection.contains(id) { ed.session.selection = [id.clone()].into(); } }
    let count = ed.session.selection.len();
    let has_notes = !ed.note_clipboard.is_empty();
    let tool = ed.tool;
    let items = match &hit {
        Hit::Body(id) | Hit::Handle(id, _) => {
            let id = id.clone();
            vec![
                MenuItem::action("编辑歌词", A::EditLyric(id)).with_hint("Enter"),
                MenuItem::action("属性", A::ShowProperties).with_hint("I"),
                MenuItem::separator(),
                MenuItem::action("剪切", A::Cut).with_hint("Ctrl+X"),
                MenuItem::action("复制", A::Copy).with_hint("Ctrl+C"),
                MenuItem::action("粘贴", A::PasteAtTick(snapped)).with_hint("Ctrl+V").enabled(has_notes),
                MenuItem::action("重复", A::Duplicate).with_hint("Ctrl+D"),
                MenuItem::action("删除", A::DeleteSelection).with_hint("Del"),
                MenuItem::separator(),
                MenuItem::action("在此分割", A::Workflow(W::Split(snapped))),
                MenuItem::action("连接相邻音符", A::Workflow(W::Join)).with_hint("Ctrl+J").enabled(count >= 2),
                MenuItem::action("量化到网格", A::Workflow(W::Quantize)).with_hint("Q"),
                MenuItem::action("静音 / 取消静音", A::Workflow(W::MuteNotes)),
                MenuItem::separator(),
                MenuItem::submenu("歌词", vec![
                    MenuItem::action("批量填写歌词…", A::Workflow(W::BatchLyrics(false))).with_hint("Ctrl+L"),
                    MenuItem::action("逐字填写歌词…", A::Workflow(W::BatchLyrics(true))),
                    MenuItem::action("设为同音节延音 (+)", A::Workflow(W::ContinueSyllable)).enabled(count >= 2),
                ]),
                MenuItem::action("颤音包络与滑音曲率…", A::Workflow(W::VibratoShape)),
                MenuItem::action("从选中音符创建片段", A::Workflow(W::CreatePart)),
            ]
        }
        Hit::Background => {
            let midi = music.pitch.round() as i32;
            vec![
                MenuItem::action("在此添加音符", A::AddNoteAt(snapped, midi)),
                MenuItem::action("在此粘贴", A::PasteAtTick(snapped)).with_hint("Ctrl+V").enabled(has_notes),
                MenuItem::action("全选", A::SelectAll).with_hint("Ctrl+A"),
                MenuItem::separator(),
                MenuItem::action("播放头移到此处", A::SeekTick(tick)),
                MenuItem::action("循环起点设在此处", A::Workflow(W::LoopStart(snapped))),
                MenuItem::action("循环终点设在此处", A::Workflow(W::LoopEnd(snapped))),
                MenuItem::separator(),
                MenuItem::header("工具"),
                MenuItem::action("指针", A::SetTool(Tool::Pointer)).checked(tool == Tool::Pointer),
                MenuItem::action("铅笔", A::SetTool(Tool::Pencil)).checked(tool == Tool::Pencil),
                MenuItem::action("橡皮", A::SetTool(Tool::Eraser)).checked(tool == Tool::Eraser),
                MenuItem::action("音高笔", A::SetTool(Tool::Pitch)).checked(tool == Tool::Pitch),
            ]
        }
    };
    ed.show_menu(position, items, window, cx);
}

// ---------------------------------------------------------------------------------
// Ruler (shared by the editor; the tracks area has its own) and keyboard
// ---------------------------------------------------------------------------------

/// Roll, parameter lanes and their headers all navigate the same viewport.
/// Non-pitch surfaces anchor pitch zoom at the middle of the visible roll;
/// the keyboard anchors time zoom at the roll's left edge.
#[derive(Clone, Copy)]
pub(crate) enum ScrollSurface { Roll, Timeline, Keyboard }

pub(crate) fn install_roll_scroll(editor:&Entity<Editor>,bounds:Bounds<Pixels>,hitbox:&Hitbox,surface:ScrollSurface,input:&crate::proto_editor_surface::Input,window:&mut Window) {
    let (e, h) = (editor.clone(), hitbox.clone());let input=input.clone();
    window.on_mouse_event(move |ev: &ScrollEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {return;}
        // The visible keys are compiled native siblings above the paint canvas.
        // Their parent keyboard still owns wheel navigation over the clipped
        // keyboard bounds, including the key hitboxes themselves.
        let under_keyboard=matches!(surface,ScrollSurface::Keyboard)&&bounds.contains(&ev.position)&&h.content_mask.bounds.contains(&ev.position);
        if !under_keyboard&&!h.should_handle_scroll(window){return;}
        let local = ev.position - bounds.origin;
        let Some(intent)=input.wheel(ev.delta,ev.modifiers,false,window,cx) else {return;};
        e.update(cx, |ed, cx| {
            let vp = &mut ed.session.viewport;
            let anchor = match surface {
                ScrollSurface::Roll => (local.x.as_f32() as f64, local.y.as_f32() as f64),
                ScrollSurface::Timeline => (local.x.as_f32() as f64, vp.height * 0.5),
                ScrollSurface::Keyboard => (0.0, local.y.as_f32() as f64),
            };
            if intent.apply(vp, anchor) { ed.notify_viewport(crate::clock::Space::Editor, cx); }
        });
        cx.stop_propagation();
    });
}

/// Bar ruler for a horizontal view. `arrange` selects which viewport.
pub fn ruler(editor: Entity<Editor>, arrange: bool) -> impl IntoElement {
    let draw=editor.clone();let cancel=editor.clone();let commands=editor.clone();
    let surface = crate::proto_editor_surface::surface(editor,format!("ruler-{arrange}"),"时间标尺与循环范围",0.0,
        move |input|native_ruler(draw.clone(),arrange,input).into_any_element(),
        move |_,window,cx| {
            crate::ruler_navigation::cancel(cancel.entity_id(),window,cx);
            let owned=cx.try_global::<RollExtraDragState>().and_then(|s|s.loop_drag).is_some_and(|drag|drag.editor==cancel.entity_id()&&drag.arrange==arrange);
            if owned { cx.global_mut::<RollExtraDragState>().loop_drag=None;cancel.update(cx,|_,cx|cx.notify()); }
        });
    if arrange {
        surface.commands("audio", move |command, window, cx| commands.update(cx, |ed, cx| {
            crate::editor::integrations::audio_edit_ui::source_command(ed, command, window, cx);
        }))
    } else { surface }
}

fn native_ruler(editor:Entity<Editor>,arrange:bool,input:crate::proto_editor_surface::Input)->impl IntoElement {
    canvas(
        move |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |bounds, hitbox, window, cx| {
            if cx.try_global::<RollExtraDragState>().is_none() {
                cx.set_global(RollExtraDragState::default());
            }
            let mut texts = Texts::new();
            let mut over_loop_handle = false;
            {
                let o = bounds.origin;
                let ed = editor.read(cx);
                let th = ed.theme;
                let vp = if arrange { ed.arrange } else { ed.session.viewport };
                let (w, hgt) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
                window.paint_quad(fill(bounds, th.surface));
                let meter=ed.meter();
                let(t0,t1)=(vp.start_tick,vp.start_tick+w/vp.px_per_tick);

                // Multi-language phonemizer segment banners on Piano Roll ruler
                if !arrange {
                    // The segment index admits only disjoint ranges per track.
                    window.paint_layer(|window| {
                        for seg in ed.language_segments.visible(&ed.session.track, t0.floor() as i64, t1.ceil() as i64) {
                            let sx0 = vp.tick_to_x(seg.start_tick as f64).max(0.0);
                            let sx1 = vp.tick_to_x(seg.end_tick as f64).min(w);
                            if sx1 > sx0 + 2.0 {
                                let short_name = match seg.selected.as_str() {
                                    "japanese-kana-v1" => "日语假名",
                                    "mandarin-dictionary-v1" => "普通话",
                                    "explicit-only-v1" => "显式音标",
                                    other => other,
                                };
                                window.paint_quad(quad(
                                    b(sx0, hgt - 11.0, sx1 - sx0, 10.0, o),
                                    px(2.0),
                                    with_alpha(th.warning, 0.22),
                                    px(1.0),
                                    with_alpha(th.warning, 0.65),
                                    BorderStyle::Solid,
                                ));
                                if sx1 - sx0 >= 36.0 {
                                    texts.push((short_name.into(), gpui::point(o.x + px(sx0 as f32 + 4.0), o.y + px((hgt - 11.5) as f32)), 8.5, th.warning));
                                }
                            }
                        }
                    });
                }

                let mut last_label=f64::NEG_INFINITY;
                for tick in meter.bars(t0.ceil()as i64,t1.ceil()as i64){
                    let x=vp.tick_to_x(tick as f64);
                    window.paint_quad(fill(b(x,hgt*0.45,1.,hgt*0.55,o),th.grid_bar));
                    if x-last_label>=44.{texts.push((meter.position_at(tick).bar.to_string(),gpui::point(o.x+px(x as f32+4.),o.y+px(4.)),11.,th.text_secondary));last_label=x;}
                }
                for event in ed.project().tempo_map.iter().filter(|p|(p.tick as f64)>=t0&&(p.tick as f64)<t1){let x=vp.tick_to_x(event.tick as f64);texts.push((format!("♩{:.1}",event.micro_bpm as f64/1e6),gpui::point(o.x+px(x as f32+3.),o.y+px((hgt-10.)as f32)),9.,th.accent));}
                for event in ed.project().time_signatures.iter().filter(|p|(p.tick as f64)>=t0&&(p.tick as f64)<t1){let x=vp.tick_to_x(event.tick as f64);texts.push((format!("{}/{}",event.numerator,event.denominator),gpui::point(o.x+px(x as f32+56.),o.y+px((hgt-10.)as f32)),9.,th.text_secondary));}

                let transport=ed.playback_settings();
                let loop_preview = cx.global::<RollExtraDragState>()
                    .loop_drag
                    .filter(|drag| drag.editor == editor.entity_id())
                    .map(|drag| (drag.start,drag.end));
                let (ls, le) = loop_preview.unwrap_or((transport.loop_start, transport.loop_end));
                if le > ls {
                    let a = vp.tick_to_x(ls as f64);
                    let z = vp.tick_to_x(le as f64);
                    let loop_col = if transport.looping || loop_preview.is_some() { th.accent } else { th.text_tertiary };
                    window.paint_quad(fill(b(a, 0.0, (z - a).max(2.0), 4.5, o), with_alpha(loop_col, 0.85)));
                    window.paint_quad(fill(b(a - 2.0, 0.0, 4.0, 10.0, o), loop_col));
                    window.paint_quad(fill(b(z - 2.0, 0.0, 4.0, 10.0, o), loop_col));
                    if hitbox.is_hovered(window) {
                        let mx = (window.mouse_position().x - o.x).as_f32() as f64;
                        let my = (window.mouse_position().y - o.y).as_f32() as f64;
                        if my <= 12.0 && ((mx - a).abs() <= 6.0 || (mx - z).abs() <= 6.0) {
                            over_loop_handle = true;
                        }
                    }
                }
                window.paint_quad(fill(b(0.0, hgt - 1.0, w, 1.0, o), th.shade));
            }
            flush_texts(window, cx, bounds, texts);
            if over_loop_handle || cx.global::<RollExtraDragState>().loop_drag.is_some_and(|drag| drag.editor == editor.entity_id() && drag.arrange == arrange) {
                window.set_cursor_style(CursorStyle::ResizeLeftRight, &hitbox);
            }
            if arrange {
                crate::arrange::install_timeline_scroll(&editor,bounds,&hitbox,&input,window);
            } else {
                install_roll_scroll(&editor,bounds,&hitbox,ScrollSurface::Timeline,&input,window);
            }
            crate::ruler_navigation::install(&editor, arrange, bounds, &hitbox, window, cx);
            let (e,h_down,policy)=(editor.clone(),hitbox.clone(),input.clone());
            crate::proto_editor_surface::install_pointer_input(&input,bounds,&hitbox,window,move |event,window,cx| {
                use crate::proto_editor_surface::PointerPhase;
                let (x,y)=(event.x,event.y);
                match event.phase {
                    PointerPhase::Begin|PointerPhase::Context => {
                let mut started_loop_drag = None;
                let mut scrub = false;
                e.update(cx, |ed, cx| {
                    let vp = if arrange { ed.arrange } else { ed.session.viewport };
                    let raw_tick=vp.start_tick+x/vp.px_per_tick;
                    let segment=ed.language_segments.at(&ed.session.track,ed.session.snap.snap(raw_tick).max(0)).map(|segment|segment.key.clone());
                    let pb=ed.playback_settings();let(ls,le)=(pb.loop_start,pb.loop_end);
                    let loop_band=le>ls&&y<=12.;
                    let start_hit=loop_band&&(x-vp.tick_to_x(ls as f64)).abs()<=6.5;
                    let end_hit=loop_band&&(x-vp.tick_to_x(le as f64)).abs()<=6.5;
                    let Some(intent)=policy.intent(ed,"rulerGesture",vec![(event.phase==PointerPhase::Context).into(),
                        (event.clicks as u64).into(),arrange.into(),segment.is_some().into(),start_hit.into(),end_hit.into(),
                        event.modifiers.shift.into(),event.modifiers.alt.into()],window,cx) else{return;};
                    let tick=if intent.snap {ed.session.snap.snap(raw_tick)} else {raw_tick.round() as i64}.max(0);
                    if intent.kind=="ruler-menu" {
                        use crate::menu::MenuItem as I;use crate::editor::{MenuAction as M,WorkflowAction as A};
                        let tempo=ed.project().tempo_map.iter().rev().find(|p|p.tick<=tick).map(|p|p.tick).unwrap_or(0);
                        let meter=ed.project().time_signatures.iter().rev().find(|p|p.tick<=tick).map(|p|p.tick).unwrap_or(0);
                        let looping=ed.playback_settings().looping;
                        let mut items=vec![
                            I::action("播放头移到此处",M::SeekTick(tick)),I::action("跳转到小节…",M::Workflow(A::GoToBar)),I::separator(),
                            I::action("循环起点设在此处",M::Workflow(A::LoopStart(tick))),I::action("循环终点设在此处",M::Workflow(A::LoopEnd(tick))),I::boolean_command("循环播放",crate::generated::controls::CommandId::ToggleLoop,crate::generated::controls::CommandArg::None,looping),I::separator(),
                            I::header("速度与拍号"),I::action("在此处编辑 / 新增速度…",M::Workflow(A::EditTempo(tick))),I::action("删除前一速度事件",M::Workflow(A::DeleteTempo(tempo))).enabled(tempo>0),
                            I::action("在此处编辑 / 新增拍号…",M::Workflow(A::EditMeter(tick))),I::action("删除前一拍号事件",M::Workflow(A::DeleteMeter(meter))).enabled(meter>0)];
                        if !arrange {
                            items.push(I::separator());
                            items.push(I::action("添加多语言读音器分段…", M::Workflow(A::Integration(crate::editor::integrations::IntegrationAction::AddSegment))));
                        }
                        ed.show_menu(event.position,items,window,cx);
                        return;
                    }
                    if intent.kind=="edit-segment" {
                        if let Some(key)=segment { let _=ed.open_segment_dialog(Some(key),window,cx); }
                        return;
                    }
                    if matches!(intent.kind.as_str(),"drag-loop-start"|"drag-loop-end") {
                        started_loop_drag=Some((intent.kind=="drag-loop-start",ls,le,intent.snap));return;
                    }
                    let min_loop_len = ed.project().ppq.max(1);
                    if intent.kind=="set-loop-start"{ed.edit_playback("循环起点",cx,|s|{s.loop_start=tick;if s.loop_end<=tick{s.loop_end=tick+min_loop_len;}s.looping=true;});}
                    else if intent.kind=="set-loop-end"{ed.edit_playback("循环终点",cx,|s|{s.loop_end=tick.max(s.loop_start+1);s.looping=true;});}
                    else if intent.kind=="scrub" { scrub = true; }
                });
                if let Some((drag_start, ls, le,snap)) = started_loop_drag {
                    cx.global_mut::<RollExtraDragState>().loop_drag = Some(LoopPreview{editor:e.entity_id(),arrange,dragging_start:drag_start,start:ls,end:le,snap});
                    return true;
                } else if scrub { return crate::ruler_navigation::begin_pointer(&e,arrange,bounds,&h_down,&policy,window,cx,event.position); }
                false

                    }
                    PointerPhase::Sample => {
                        if crate::ruler_navigation::is_active(e.entity_id(),cx) {
                            crate::ruler_navigation::sample(&e,arrange,event.position,window,cx);
                        } else { sample_loop_range(&e,arrange,x,cx); }
                        true
                    }
                    PointerPhase::Finish => {
                        if crate::ruler_navigation::is_active(e.entity_id(),cx) { crate::ruler_navigation::finish(&e,arrange,event.position,window,cx); }
                        else { finish_loop_range(&e,arrange,window,cx); }
                        true
                    }
                }
            });
        },
    )
    .size_full()
}

fn sample_loop_range(editor:&Entity<Editor>,arrange:bool,x:f64,cx:&mut App) {
    let Some(drag)=cx.try_global::<RollExtraDragState>().and_then(|s|s.loop_drag) else { return };
    if drag.editor!=editor.entity_id()||drag.arrange!=arrange { return; }
    let ed=editor.read(cx);let vp=if arrange { ed.arrange } else { ed.session.viewport };
    let raw=vp.x_to_tick(x);let tick=if drag.snap{ed.session.snap.snap(raw)}else{raw.round() as i64}.max(0);let minimum=ed.project().ppq.max(1).min(120);
    let (ls,le)=if drag.dragging_start { (tick.min(drag.end-1).max(0),drag.end) } else { (drag.start,tick.max(drag.start+minimum)) };
    if let Some(drag)=cx.global_mut::<RollExtraDragState>().loop_drag.as_mut() { drag.start=ls;drag.end=le; }
    editor.update(cx,|_,cx|cx.notify());
}
fn finish_loop_range(editor:&Entity<Editor>,arrange:bool,window:&mut Window,cx:&mut App) {
    let matched=cx.try_global::<RollExtraDragState>().and_then(|s|s.loop_drag).is_some_and(|drag|drag.editor==editor.entity_id()&&drag.arrange==arrange);
    if !matched { return; }
    if let Some(LoopPreview{start,end,..})=cx.global_mut::<RollExtraDragState>().loop_drag.take() {
        editor.update(cx,|ed,cx| {
            ed.edit_playback("拖拽循环范围",cx,|s| { s.loop_start=start;s.loop_end=end.max(start+1);s.looping=true; });cx.notify();
        });
    }
}

/// Scale degrees are relative to the configured root, repeated in every octave.
fn scale_contains(midi: i32, root: u8, intervals: &[u8]) -> bool {
    intervals.contains(&((midi - i32::from(root)).rem_euclid(12) as u8))
}

const BLACK_KEY_WIDTH: f64 = 0.6;

/// White keys extend halfway behind adjacent black keys. The same geometry
/// determines which white key is played on the exposed right-hand portion.
fn keyboard_key_rect(vp: Viewport, midi: i32, width: f64) -> (f64, f64, f64) {
    if is_black_key(midi) {
        (vp.pitch_to_y(f64::from(midi) + 1.0), width * BLACK_KEY_WIDTH, vp.row_height)
    } else {
        let above = if midi < 127 && is_black_key(midi + 1) { 0.5 } else { 0.0 };
        let below = if midi > 0 && is_black_key(midi - 1) { 0.5 } else { 0.0 };
        (vp.pitch_to_y(f64::from(midi) + 1.0 + above), width, vp.row_height * (1.0 + above + below))
    }
}

fn keyboard_pitch_at(vp: Viewport, x: f64, y: f64, width: f64, height: f64) -> Option<i32> {
    if !(0.0..width).contains(&x) || !(0.0..height).contains(&y) {
        return None;
    }
    let pitch = vp.y_to_pitch(y);
    // Paint rectangles include their top edge and exclude their bottom edge.
    let row = pitch.ceil() as i32 - 1;
    if !(0..=127).contains(&row) {
        return None;
    }
    Some(if is_black_key(row) && x >= width * BLACK_KEY_WIDTH {
        if pitch > f64::from(row) + 0.5 { row + 1 } else { row - 1 }
    } else {
        row
    })
}

/// Clicking plays the existing short reference sine through the audio engine;
/// it does not render the selected singer or alter notes in the project.
#[derive(gpui::IntoElement)]
struct PianoKeyboard { editor:Entity<Editor>,input:crate::proto_editor_surface::Input }
pub fn keyboard(editor:Entity<Editor>)->impl IntoElement {
    let draw=editor.clone();
    crate::proto_editor_surface::surface(editor,"reference-keyboard","参考钢琴键盘",0.0,
        move |input|PianoKeyboard{editor:draw.clone(),input}.into_any_element(),|_,_,_|{})
}
impl gpui::RenderOnce for PianoKeyboard {
    fn render(self,_:&mut Window,cx:&mut App)->impl IntoElement {
        let ed=self.editor.read(cx);let vp=ed.session.viewport;let th=ed.theme;
        let revision=format!("{}:{}",ed.project().project_id,ed.document_generation);
        let scope=format!("reference-keyboard-{:?}",self.editor.entity_id());
        let drawing=self.editor.clone();
        let mut root=gpui::div().relative().size_full().overflow_hidden()
            .child(native_keyboard_paint(drawing,self.input));
        let (lo,hi)=vp.visible_rows();
        for black in [false,true] {
            for midi in (lo-1).max(0)..=(hi+1).min(127) {
                if is_black_key(midi)!=black { continue; }
                let (y,width,height)=keyboard_key_rect(vp,midi,1.0);
                let editor=self.editor.downgrade();
                let key=crate::proto_performance_key::key(scope.clone(),midi as u8,note_name(midi),revision.clone(),false,
                    move |pressed|gpui::div().size_full().bg(if pressed{with_alpha(th.accent,0.6)}else{gpui::transparent_black().into()}).into_any_element(),
                    move |pressed,cx|{if pressed{let _=editor.update(cx,|ed,cx|{ed.audition_pitch(midi);cx.notify();});}}).momentary();
                root=root.child(gpui::div().absolute().left(px(0.)).top(px(y as f32)).w(gpui::relative(width as f32)).h(px(height as f32)).child(key));
            }
        }
        root
    }
}
fn native_keyboard_paint(editor:Entity<Editor>,input:crate::proto_editor_surface::Input)->impl IntoElement {
    canvas(|bounds,window,_|window.insert_hitbox(bounds,HitboxBehavior::Normal),move |bounds,hitbox,window,cx| {
            let mut texts = Texts::new();
            {
                let o = bounds.origin;
                let ed = editor.read(cx);
                let th = ed.theme;
                let vp = ed.session.viewport;
                let w = bounds.size.width.as_f32() as f64;
                window.paint_quad(fill(bounds, th.key_white));
                window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                    let (lo, hi) = vp.visible_rows();
                    // Paint white keys first, then the narrower black keys.
                    // Include neighbours whose extended white portion is visible.
                    for black in [false, true] {
                        for midi in (lo - 1).max(0)..=(hi + 1).min(127) {
                            if is_black_key(midi) != black { continue; }
                            let (y, key_width, height) = keyboard_key_rect(vp, midi, w);
                            window.paint_quad(fill(b(0.0, y, key_width, height, o), if black { th.key_black } else { th.key_white }));
                            if scale_contains(midi, ed.integrations.scale_root, &ed.integrations.scale_intervals) {
                                window.paint_quad(fill(b(0.0, y, key_width, height, o), with_alpha(th.accent, 0.22)));
                            }
                            if !black {
                                window.paint_quad(fill(b(0.0, y + height - 0.5, key_width, 0.5, o), th.separator));
                            }
                            if midi.rem_euclid(12) == 0 && vp.row_height >= 9.0 {
                                let fs = (vp.row_height as f32 * 0.55).clamp(8.0, 11.0);
                                texts.push((
                                    note_name(midi),
                                    gpui::point(o.x + px(w as f32 - 26.0), o.y + px(y as f32 + 1.0)),
                                    fs,
                                    crate::theme::black(),
                                ));
                            }
                        }
                    }
                });
                window.paint_quad(fill(b(w - 1.0, 0.0, 1.0, bounds.size.height.as_f32() as f64, o), th.shade));
            }
            flush_texts(window, cx, bounds, texts);
            install_roll_scroll(&editor,bounds,&hitbox,ScrollSurface::Keyboard,&input,window);
    }).size_full()
}

/// Rect (canvas coordinates) of a note in the editor, for overlays.
pub fn note_overlay_rect(ed: &Editor, id: &NoteId) -> Option<(f64, f64, f64, f64)> {
    let n = ed.active_track()?.note(id)?;
    let r = ed.session.viewport.note_rect(n);
    Some((r.x, r.y, r.w, r.h))
}

#[cfg(test)]
mod keyboard_tests {
    use super::*;

    #[test]
    fn scale_highlights_exact_configured_degrees_across_all_midi_octaves() {
        for (root, degrees, expected_classes) in [
            (2, &[0, 2, 4, 5, 7, 9, 11][..], &[1, 2, 4, 6, 7, 9, 11][..]), // D major
            (10, &[0, 2, 3, 5, 7, 8, 10][..], &[0, 1, 3, 5, 6, 8, 10][..]), // B-flat minor
            (11, &[0, 3, 7][..], &[2, 6, 11][..]), // B minor triad
        ] {
            let actual: Vec<_> = (0..=127).filter(|&midi| scale_contains(midi, root, degrees)).collect();
            let expected: Vec<_> = (0..=127).filter(|midi| expected_classes.contains(&(midi % 12))).collect();
            assert_eq!(actual, expected, "root {root}, degrees {degrees:?}");
        }
    }

    #[test]
    fn empty_scale_configuration_highlights_nothing() {
        for root in 0..12 {
            assert!((0..=127).all(|midi| !scale_contains(midi, root, &[])));
        }
    }

    #[test]
    fn keyboard_click_matches_painted_keys_and_exposed_white_extensions() {
        let width = 80.0;
        for row_height in [6.0, 18.0, 64.0] {
            for top_pitch in [128.0, 128.375] {
                let vp = Viewport { top_pitch, row_height, height: 130.0 * row_height, ..Viewport::default() };
                for midi in 0..=127 {
                    let (y, key_width, height) = keyboard_key_rect(vp, midi, width);
                    // On the right of white keys, both extensions belong to
                    // that white key, even alongside an adjacent black row.
                    let x = if is_black_key(midi) { key_width * 0.5 } else { width * 0.9 };
                    for click_y in [y, y + height * 0.5, y + height - 0.01] {
                        assert_eq!(keyboard_pitch_at(vp, x, click_y, width, vp.height), Some(midi),
                            "MIDI {midi}, x={x}, y={click_y}, row height={row_height}, top={top_pitch}");
                    }
                }
            }
        }

        let vp = Viewport { top_pitch: 72.0, row_height: 10.0, height: 200.0, ..Viewport::default() };
        // C-sharp 4 is the black rectangle at y=100..110. Its exposed
        // right-hand neighbours are D4 above the midpoint and C4 below it.
        assert_eq!(keyboard_pitch_at(vp, 10.0, 100.0, width, vp.height), Some(61));
        assert_eq!(keyboard_pitch_at(vp, 10.0, 105.0, width, vp.height), Some(61));
        assert_eq!(keyboard_pitch_at(vp, 70.0, 102.0, width, vp.height), Some(62));
        assert_eq!(keyboard_pitch_at(vp, 70.0, 105.0, width, vp.height), Some(60));
        assert_eq!(keyboard_pitch_at(vp, 70.0, 108.0, width, vp.height), Some(60));
        assert_eq!(keyboard_pitch_at(vp, width * BLACK_KEY_WIDTH, 102.0, width, vp.height), Some(62));
        for (x, y) in [(-1.0, 105.0), (width, 105.0), (10.0, -1.0), (10.0, vp.height)] {
            assert_eq!(keyboard_pitch_at(vp, x, y, width, vp.height), None);
        }
    }
}

#[cfg(test)]
mod contour_lod_tests {
    use super::*;

    #[test]
    fn moved_pitch_preview_enters_view_with_its_new_neighbours() {
        let mut track = Track::new("Pitch preview", "teal");
        track.notes = vec![Note::new(0, 100, 60), Note::new(100, 100, 62), Note::new(300, 100, 65)];
        track.refresh_index();
        let original = &track.notes[0];
        let preview = Preview::Move { delta_tick: 200, delta_semitones: 7, duplicate: false };
        let moved = HashMap::from([(&original.id, preview.project_note(original).unwrap())]);
        let mut contour = Vec::new();
        visit_pitch_preview(&track, &moved, 0, 400, |note, prev, next| {
            contour.push((note.start_tick, note.midi_note, prev.map(|n| n.start_tick), next.map(|n| n.start_tick)));
        });
        contour.sort_unstable();
        assert_eq!(contour, [(100, 62, None, Some(200)), (200, 67, Some(100), Some(300)), (300, 65, Some(200), None)]);

        let mut visible = Vec::new();
        visit_pitch_preview(&track, &moved, 200, 300, |note, _, _| visible.push(note.start_tick));
        assert_eq!(visible, [200]);
    }

    /// A span that covers only part of the viewport must LOD against its own
    /// on-screen width, not the full canvas: many narrow phrases would each
    /// otherwise pay a whole-width curve every frame.
    #[test]
    fn narrow_span_lod_uses_span_width_not_viewport() {
        let tempo = TempoMap::constant(120.0);
        // 4 s span at 5 ms hop = 800 frames, but only ~40 px wide at this zoom.
        let f0 = F0Track { kind: F0Kind::NeuralEstimate, first_frame_seconds: 0.0, hop_seconds: 0.005, hz: (0..800).map(|i| 220.0 + i as f32 / 10.0).collect() };
        let contour = PitchContour::new(&f0);
        let mut vp = Viewport::default();
        vp.width = 1600.0;
        vp.px_per_tick = 5.0 / 960.0; // 5 px per quarter → 4 s span ≈ 40 px
        let points: Vec<_> = contour.points(0.0, 4.0, 0.0, 4.0, &tempo, vp).flatten().collect();
        assert!(points.len() <= 44, "span must emit ≈ its own pixel width in samples, got {}", points.len());
        assert!(points.len() >= 30, "LOD must keep ≈1 sample per pixel, got {}", points.len());
        // Same span stretched over the whole viewport samples every frame.
        vp.px_per_tick = 200.0;
        let wide: Vec<_> = contour.points(0.0, 4.0, 0.0, 4.0, &tempo, vp).flatten().collect();
        assert!(wide.len() > 700, "zoomed-in span keeps near-full resolution, got {}", wide.len());
    }

    #[test]
    fn pitch_line_visible_bounds_curves_vertically() {
        let mut vp = Viewport::default();
        vp.height = 400.0;
        vp.top_pitch = 90.0;
        vp.row_height = 10.0;
        let mut n = Note::new(0, 960, 60);
        // Plain note at row 60 is visible; a far-offscreen note is not.
        assert!(pitch_line_visible(&n, None, None, vp, 400.0));
        n.midi_note = 24;
        assert!(!pitch_line_visible(&n, None, None, vp, 400.0));
        // A huge authored PIT drop pulls the far note back into view.
        n.curves.pitch_millicent_points = vec![verdant_domain::model::PitchPoint { id: verdant_domain::PointId::generate(), tick: 0, millicents: 4_000_000, unknown: Default::default() }];
        assert!(pitch_line_visible(&n, None, None, vp, 400.0));
    }
}
