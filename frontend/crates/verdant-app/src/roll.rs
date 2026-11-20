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
    loop_drag: Option<(gpui::EntityId, bool, bool, i64, i64)>, // (editor, is_arrange, dragging_start, start_tick, end_tick)
}
impl gpui::Global for RollExtraDragState {}

pub(crate) fn has_extra_drag(editor: gpui::EntityId, cx: &App) -> bool {
    cx.try_global::<RollExtraDragState>().is_some_and(|s| {
        s.mod_drag.as_ref().is_some_and(|d| d.editor == editor)
            || s.loop_drag.is_some_and(|(eid, _, _, _, _)| eid == editor)
    })
}

pub(crate) fn cancel_extra_drag(editor: gpui::EntityId, cx: &mut App) {
    if let Some(state) = cx.try_global::<RollExtraDragState>() {
        let cancel_mod = state.mod_drag.as_ref().is_some_and(|d| d.editor == editor);
        let cancel_loop = state.loop_drag.is_some_and(|(eid, _, _, _, _)| eid == editor);
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
    let ed_prepaint = editor.clone();
    canvas(
        move |bounds, window, cx| {
            let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
            ed_prepaint.update(cx, |ed, _| {
                ed.session.viewport.width = bounds.size.width.as_f32() as f64;
                ed.session.viewport.height = bounds.size.height.as_f32() as f64;
                if ed.needs_fit {
                    ed.needs_fit = false;
                    ed.fit_view();
                }
            });
            hitbox
        },
        move |bounds, hitbox, window, cx| {
            let mut texts = Texts::new();
            paint_roll(&editor, bounds, window, cx, &mut texts);
            flush_texts(window, cx, bounds, texts);
            install_roll_input(&editor, bounds, &hitbox, window, cx);
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

fn install_roll_input(editor: &Entity<Editor>, bounds: Bounds<Pixels>, hitbox: &Hitbox, window: &mut Window, cx: &mut App) {
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

    // ---- mouse ----
    let h = hitbox.clone();
    let e = editor.clone();
    window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !h.is_hovered(window) {
            return;
        }
        let l = ev.position - bounds.origin;
        let (x, y) = (l.x.as_f32() as f64, l.y.as_f32() as f64);
        match ev.button {
            MouseButton::Left => {
                if e.read(cx).tool == Tool::Pitch { return; }
                let maybe_mod = hit_mod_handle(e.read(cx), x, y).map(|mut d| { d.editor = e.entity_id(); d });
                if let Some(mod_drag) = maybe_mod {
                    cx.global_mut::<RollExtraDragState>().mod_drag = Some(mod_drag);
                    window.capture_pointer(h.id);
                    e.update(cx, |ed, cx| { window.focus(&ed.focus_roll, cx); cx.notify(); });
                    cx.stop_propagation();
                    return;
                }
                e.update(cx, |ed, cx| {
                    pointer_down(ed, PointerId::Mouse, x, y, mods(&ev.modifiers), ev.click_count as u32, &h, window, cx)
                });
                cx.stop_propagation();
            }
            MouseButton::Right => {
                e.update(cx, |ed, cx| roll_context_menu(ed, ev.position, x, y, window, cx));
                cx.stop_propagation();
            }
            _ => {}
        }
    });
    let e = editor.clone();
    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let l = ev.position - bounds.origin;
        let (lx, ly) = (l.x.as_f32() as f64, l.y.as_f32() as f64);
        let updated_mod = {
            let Some(drag) = cx.try_global::<RollExtraDragState>().and_then(|s| s.mod_drag.as_ref()) else {
                e.update(cx, |ed, cx| {
                    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==PointerId::Mouse) || matches!(ed.session.pointer_owner(), Some(PointerId::Mouse)) {
                        pointer_move(ed, PointerId::Mouse, lx, ly, mods(&ev.modifiers), window, cx)
                    }
                });
                return;
            };
            if drag.editor != e.entity_id() { return; }
            let ed = e.read(cx);
            let tick = ed.session.viewport.x_to_tick(lx).max(0.0);
            let sec = ed.tempo().seconds_at(tick);
            let next_porta = ((sec - drag.note_start_seconds).max(0.0) * 1000.0).round().clamp(0.0, 2000.0) as u32;
            let next_delay = (((lx - drag.note_rect_x) / drag.note_rect_w.max(1.0)).clamp(0.0, 1.0) * 1000.0).round() as u32;
            let next_depth = (drag.initial_depth + (drag.origin_y - ly) * 1.5).round().clamp(0.0, 200.0) as u32;
            Some((next_porta, next_delay, next_depth))
        };
        if let Some((p_ms, dl_pm, dp_c)) = updated_mod {
            if let Some(drag) = cx.global_mut::<RollExtraDragState>().mod_drag.as_mut() {
                drag.portamento_ms = p_ms;
                drag.delay_permille = dl_pm;
                drag.depth_cents = dp_c;
            }
            e.update(cx, |_, cx| cx.notify());
        }
    });
    let e = editor.clone();
    window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left {
            return;
        }
        if cx.try_global::<RollExtraDragState>().and_then(|s| s.mod_drag.as_ref()).is_some_and(|d| d.editor == e.entity_id()) {
            let finished = cx.global_mut::<RollExtraDragState>().mod_drag.take();
            window.release_pointer();
            if let Some(drag) = finished {
                e.update(cx, |ed, cx| {
                    if !drag.test_matches(ed) { cx.notify(); return; }
                    let ids: Vec<NoteId> = if ed.session.selection.contains(&drag.note_id) {
                        ed.session.selection_in_order(ed.project())
                    } else {
                        vec![drag.note_id]
                    };
                    match drag.kind {
                        ModHandleKind::Portamento => {
                            ed.commit(drag.base, CommandBatch::single("拖拽调整滑音时长", Command::SetPortamento {
                                track: drag.track,
                                note_ids: ids,
                                portamento_ms: Some(drag.portamento_ms),
                            }), cx);
                        }
                        ModHandleKind::Vibrato => {
                            let mut notes = ed.selected_notes();
                            for n in &mut notes {
                                if drag.depth_cents == 0 {
                                    n.vibrato = None;
                                } else {
                                    let mut vib = n.vibrato.clone().unwrap_or(verdant_domain::Vibrato::DEFAULT);
                                    vib.depth_cents = drag.depth_cents;
                                    vib.delay_permille = drag.delay_permille;
                                    n.vibrato = Some(vib);
                                }
                            }
                            if !notes.is_empty() {
                                ed.commit(drag.base, CommandBatch::single("拖拽调整音符颤音", Command::ReplaceNotes {
                                    track: drag.track,
                                    notes,
                                }), cx);
                            }
                        }
                    }
                    cx.notify();
                });
            }
            return;
        }
        e.update(cx, |ed, cx| {
            if ed.note_creation.as_ref().is_some_and(|c| c.pointer == PointerId::Mouse) {
                let l = ev.position - bounds.origin;
                pointer_move(ed, PointerId::Mouse, l.x.as_f32() as f64, l.y.as_f32() as f64, mods(&ev.modifiers), window, cx);
            }
            if ed.note_creation.as_ref().is_some_and(|c|c.pointer==PointerId::Mouse) || matches!(ed.session.pointer_owner(), Some(PointerId::Mouse)) {
                pointer_up(ed, PointerId::Mouse, window, cx)
            }
        });
    });

    // ---- touch: claim drags that start on a note (or with an editing tool);
    // unclaimed drags stay pans, handled by the scroll listener below ----
    let h = hitbox.clone();
    let e = editor.clone();
    window.on_mouse_event(move |ev: &TouchDragEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let l = ev.position - bounds.origin;
        let (x, y) = (l.x.as_f32() as f64, l.y.as_f32() as f64);
        match ev.phase {
            TouchPhase::Started => {
                let s = ev.start_position - bounds.origin;
                let (sx, sy) = (s.x.as_f32() as f64, s.y.as_f32() as f64);
                if !h.is_hovered_at(ev.start_position, window) {
                    return;
                }
                let claim = e.read(cx).active_track().is_some_and(|t| {
                    let ed = e.read(cx);
                    ed.tool != Tool::Pointer
                        || !matches!(hit_test_touch(t, &ed.session.viewport, &ed.session.selection, sx, sy), Hit::Background)
                });
                if claim {
                    window.prevent_default();
                    e.update(cx, |ed, cx| {
                        pointer_down(ed, PointerId::Touch(0), sx, sy, Modifiers::default(), 1, &h, window, cx);
                        pointer_move(ed, PointerId::Touch(0), x, y, Modifiers::default(), window, cx);
                    });
                }
            }
            TouchPhase::Moved => e.update(cx, |ed, cx| {
                if ed.note_creation.as_ref().is_some_and(|c|c.pointer==PointerId::Touch(0)) || matches!(ed.session.pointer_owner(), Some(PointerId::Touch(0))) || ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == PointerId::Touch(0) && s.roll) {
                    pointer_move(ed, PointerId::Touch(0), x, y, Modifiers::default(), window, cx);
                }
            }),
            TouchPhase::Ended => e.update(cx, |ed, cx| {
                if ed.note_creation.as_ref().is_some_and(|c| c.pointer == PointerId::Touch(0))
                    || ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == PointerId::Touch(0) && s.roll)
                {
                    pointer_move(ed, PointerId::Touch(0), x, y, Modifiers::default(), window, cx);
                }
                if ed.note_creation.as_ref().is_some_and(|c|c.pointer==PointerId::Touch(0)) || matches!(ed.session.pointer_owner(), Some(PointerId::Touch(0))) || ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == PointerId::Touch(0) && s.roll) {
                    pointer_up(ed, PointerId::Touch(0), window, cx);
                }
            }),
            TouchPhase::Cancelled => e.update(cx, |ed, cx| cancel(ed, PointerId::Touch(0), CancelReason::PointerCancel, window, cx)),
        }
    });
    let h = hitbox.clone();
    let e = editor.clone();
    window.on_mouse_event(move |ev: &LongPressEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        let l = ev.position - bounds.origin;
        let (x, y) = (l.x.as_f32() as f64, l.y.as_f32() as f64);
        match ev.phase {
            TouchPhase::Started => {
                if !h.is_hovered_at(ev.start_position, window) {
                    return;
                }
                window.prevent_default();
                e.update(cx, |ed, cx| {
                    if ed.session.pointer_owner().is_some_and(|p| p != PointerId::Touch(0)) || ed.lane_stroke.is_some() { return; }
                    let p = ed.store().snapshot();
                    let Some(t) = p.track(&ed.session.track) else { return };
                    match hit_test_touch(t, &ed.session.viewport, &ed.session.selection, x, y) {
                        // Secondary activation on a note: edit its lyric.
                        Hit::Body(id) | Hit::Handle(id, _) => ed.begin_lyric(id, window, cx),
                        // On empty space: start a marquee that follows the finger.
                        Hit::Background => {
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
                            ed.apply_effects(fx, window, cx);
                        }
                    }
                });
            }
            TouchPhase::Moved => e.update(cx, |ed, cx| {
                if matches!(ed.session.pointer_owner(), Some(PointerId::Touch(_))) {
                    pointer_move(ed, PointerId::Touch(0), x, y, Modifiers::default(), window, cx);
                }
            }),
            TouchPhase::Ended => e.update(cx, |ed, cx| {
                if matches!(ed.session.pointer_owner(), Some(PointerId::Touch(_))) {
                    pointer_up(ed, PointerId::Touch(0), window, cx);
                }
            }),
            TouchPhase::Cancelled => e.update(cx, |ed, cx| cancel(ed, PointerId::Touch(0), CancelReason::PointerCancel, window, cx)),
        }
    });
    let h = hitbox.clone();
    let e = editor.clone();
    window.on_mouse_event(move |ev: &PinchEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !h.is_hovered(window) {
            return;
        }
        let l = ev.position - bounds.origin;
        e.update(cx, |ed, cx| {
            ed.session.viewport.zoom_x_at(l.x.as_f32() as f64, 1.0 + ev.delta as f64);
            ed.notify_viewport(crate::clock::Space::Editor, cx);
        });
        cx.stop_propagation();
    });
    install_roll_scroll(editor, bounds, hitbox, ScrollSurface::Roll, window);
}

pub(crate) fn begin_pitch_stroke(
    ed: &mut Editor, pointer: PointerId, x: f64, y: f64, window: &mut Window,
    cx: &mut gpui::Context<Editor>,
) -> bool {
    if ed.tool != Tool::Pitch || ed.note_creation.is_some() || ed.session.pointer_owner().is_some()
        || ed.lane_stroke.is_some() || ed.mixer_drag.is_some()
    {
        return false;
    }
    if ed.lyric.is_some() {
        ed.end_lyric(true, None, window, cx);
        if ed.lyric.is_some() { return false; }
    }
    let Some(track) = ed.active_track() else { return false; };
    let (handle, outside) = if matches!(pointer, PointerId::Touch(_)) { (22.0, 14.0) } else { (7.0, 4.0) };
    let hit = ed.index.hit_test(track, &ed.session.viewport, &ed.session.selection, x, y, handle, outside);
    if let Hit::Body(id) | Hit::Handle(id, _) = hit {
        ed.session.selection = [id].into();
    }
    window.focus(&ed.focus_roll, cx);
    let music = ed.session.viewport.to_music(x, y);
    let started = ed.begin_curve(verdant_backend::params::Param::Pit, pointer, None, None, true,
        music.tick.max(0.0), music.pitch, (x, y), false);
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
    window: &mut Window,
    cx: &mut gpui::Context<Editor>,
) {
    if ed.tool == Tool::Pitch {
        if begin_pitch_stroke(ed, pointer, x, y, window, cx) { window.capture_pointer(h.id); }
        return;
    }
    if ed.note_creation.is_some() || ed.session.pointer_owner().is_some() || ed.lane_stroke.is_some() || ed.mixer_drag.is_some() { return; }
    if ed.lyric.is_some() {
        ed.end_lyric(true, None, window, cx);
        if ed.lyric.is_some() { return; }
    }
    window.focus(&ed.focus_roll, cx);
    let p = ed.store().snapshot();
    let Some(track) = p.track(&ed.session.track) else { return };
    // Bounded candidate scan through the note index; touch gets the
    // finger-sized thresholds.
    let (handle_px, outside_px) = match pointer {
        PointerId::Touch(_) => (22.0, 14.0),
        _ => (7.0, 4.0),
    };
    let hit = ed
        .index
        .hit_test(track, &ed.session.viewport, &ed.session.selection, x, y, handle_px, outside_px);
    let music = ed.session.viewport.to_music(x, y);
    match (ed.tool, &hit) {
        (Tool::Eraser, Hit::Body(id) | Hit::Handle(id, _)) => {
            let cmd = Command::DeleteNotes { track: ed.session.track.clone(), note_ids: vec![id.clone()] };
            ed.session.selection.remove(id);
            ed.commit(ed.revision(), CommandBatch::single("擦除音符", cmd), cx);
            return;
        }
        (Tool::Eraser, Hit::Background) => return,
        (Tool::Pencil, Hit::Background) => {
            let meter = ed.meter();
            let origin = meter.signature_at(music.tick.max(0.) as i64).tick;
            let start = if m.alt { music.tick.round() as i64 } else { origin + ed.session.snap.snap(music.tick-origin as f64) }.max(0);
            // Zero is an armed preview only; finish_note_draw refuses it.
            let note = Note::new(start, 0, (music.pitch.floor() as i32).clamp(0,127));
            ed.note_creation = Some(crate::editor::workflow::NoteCreation { pointer, base: ed.revision(), origin_tick: start, note });
            window.capture_pointer(h.id); cx.notify(); return;
        }
        _ => {}
    }
    let fx =
        ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::PointerDown { hit, music, screen: (x, y), mods: m, click_count: clicks });
    if fx.contains(&Effect::CapturePointer) {
        window.capture_pointer(h.id);
    }
    ed.apply_effects(fx, window, cx);
}

fn pointer_move(ed: &mut Editor, pointer: PointerId, mut x: f64, y: f64, m: Modifiers, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer) {
        let tick = ed.session.viewport.x_to_tick(x);
        let origin = ed.meter().signature_at(tick.max(0.) as i64).tick;
        let snapped = if m.alt { tick.round() as i64 } else { origin + ed.session.snap.snap(tick - origin as f64) };
        let minimum = if m.alt { 1 } else { ed.session.snap.grid_ticks.max(1) };
        if let Some(creation) = &mut ed.note_creation {
            verdant_domain::editing::preview_note_draw(&mut creation.note, creation.origin_tick, snapped, minimum);
        }
        cx.notify();return;
    }
    if ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer && s.roll) {
        let tick = ed.session.viewport.x_to_tick(x);
        let pitch = ed.session.viewport.y_to_pitch(y);
        ed.roll_pitch_sample(pointer, tick, pitch, (x, y), m.shift); cx.notify();
        return;
    }
    if ed.session.gesture.is_none() {
        return;
    }
    let w = ed.session.viewport.width;
    if x > w - 16.0 {
        ed.session.viewport.pan_px((x - (w - 16.0)).min(40.0), 0.0);
        ed.session.autoscroll_active = true;
        x = x.min(w + 40.0);
    } else if x < 16.0 {
        ed.session.viewport.pan_px((x - 16.0).max(-40.0), 0.0);
        ed.session.autoscroll_active = true;
    } else {
        ed.session.autoscroll_active = false;
    }
    let music = ed.session.viewport.to_music(x, y);
    let p = ed.store().snapshot();
    let fx = ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::PointerMove { music, screen: (x, y), mods: m });
    ed.apply_effects(fx, window, cx);
}

fn pointer_up(ed: &mut Editor, pointer: PointerId, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer){
        let creation = ed.note_creation.take().expect("owned creation");
        window.release_pointer();
        let id = creation.note.id.clone();
        if let Some(batch) = verdant_domain::editing::finish_note_draw(ed.session.track.clone(), Some(creation.note)) {
            if ed.commit(creation.base, batch, cx) { ed.session.selection = [id].into(); }
        }
        cx.notify();
        return;
    }
    if ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer && s.roll) {
        window.release_pointer(); ed.commit_roll_pitch(cx); return;
    }
    let p = ed.store().snapshot();
    let fx = ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::PointerUp);
    window.release_pointer();
    ed.apply_effects(fx, window, cx);
}

fn cancel(ed: &mut Editor, pointer: PointerId, reason: CancelReason, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    let p = ed.store().snapshot();
    let owns = ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer) || ed.session.pointer_owner() == Some(pointer) || ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer);
    if ed.note_creation.as_ref().is_some_and(|c|c.pointer==pointer){ed.note_creation=None;}
    if !owns { return; }
    if ed.lane_stroke.as_ref().is_some_and(|s| s.pointer == pointer) { ed.lane_stroke = None; }
    let fx = ed.session.handle_pointer(&p, ed.revision(), pointer, InputEvent::Cancel(reason));
    window.release_pointer();
    ed.apply_effects(fx, window, cx);
}

/// Right-click in the roll: note menu on a note, background menu otherwise.
/// Anchored at the pointer (window coordinates); items feed
/// `Editor::run_menu_action`.
fn roll_context_menu(ed: &mut Editor, position: gpui::Point<Pixels>, x: f64, y: f64, window: &mut Window, cx: &mut gpui::Context<Editor>) {
    use crate::editor::MenuAction as A;
    use crate::menu::MenuItem;
    let project = ed.store().snapshot();
    let Some(track) = project.track(&ed.session.track) else { return };
    let vp = &ed.session.viewport;
    let hit = ed.index.hit_test(track, vp, &ed.session.selection, x, y, 7.0, 4.0);
    let music = vp.to_music(x, y);
    if let Hit::Body(id) | Hit::Handle(id, _) = &hit { if !ed.session.selection.contains(id) { ed.session.selection = [id.clone()].into(); } }
    let has_selection = !ed.session.selection.is_empty();
    let has_notes = !ed.note_clipboard.is_empty();
    let items = match &hit {
        Hit::Body(id) | Hit::Handle(id, _) => {
            let id = id.clone();
            vec![
                MenuItem::action("剪切", A::Cut).enabled(has_selection),
                MenuItem::action("复制", A::Copy).enabled(has_selection),
                MenuItem::action("复制一份", A::Duplicate).enabled(has_selection),
                MenuItem::action("编辑歌词", A::EditLyric(id)),
                MenuItem::action("删除", A::DeleteSelection).enabled(has_selection),
            ]
        }
        Hit::Background => {
            let tick = music.tick.floor() as i64;
            let midi = music.pitch.round() as i32;
            vec![
                MenuItem::action(format!("在光标处粘贴"), A::PasteAtTick(tick)).enabled(has_notes),
                MenuItem::action("添加音符", A::AddNoteAt(tick, midi)),
                MenuItem::action("全选", A::SelectAll),
                MenuItem::separator(),
                MenuItem::action("指针", A::SetTool(Tool::Pointer)),
                MenuItem::action("铅笔", A::SetTool(Tool::Pencil)),
                MenuItem::action("橡皮", A::SetTool(Tool::Eraser)),
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

pub(crate) fn install_roll_scroll(editor: &Entity<Editor>, bounds: Bounds<Pixels>, hitbox: &Hitbox, surface: ScrollSurface, window: &mut Window) {
    let (e, h) = (editor.clone(), hitbox.clone());
    window.on_mouse_event(move |ev: &ScrollEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble || !h.should_handle_scroll(window) { return; }
        let local = ev.position - bounds.origin;
        e.update(cx, |ed, cx| {
            let vp = &mut ed.session.viewport;
            let anchor = match surface {
                ScrollSurface::Roll => (local.x.as_f32() as f64, local.y.as_f32() as f64),
                ScrollSurface::Timeline => (local.x.as_f32() as f64, vp.height * 0.5),
                ScrollSurface::Keyboard => (0.0, local.y.as_f32() as f64),
            };
            scroll_viewport(vp, ev.delta, ev.modifiers, anchor);
            ed.notify_viewport(crate::clock::Space::Editor, cx);
        });
        cx.stop_propagation();
    });
}

fn scroll_viewport(vp: &mut Viewport, delta: ScrollDelta, modifiers: gpui::Modifiers, anchor: (f64, f64)) {
    let d = match delta {
        ScrollDelta::Pixels(p) => (p.x.as_f32() as f64, p.y.as_f32() as f64),
        ScrollDelta::Lines(l) => (l.x as f64 * 40.0, l.y as f64 * 40.0),
    };
    if modifiers.control || modifiers.platform {
        vp.zoom_x_at(anchor.0, 1.0015f64.powf(d.1));
    } else if modifiers.alt {
        vp.zoom_y_at(anchor.1, 1.0015f64.powf(d.1));
    } else if modifiers.shift {
        // GPUI normalizes Shift+vertical wheel into X on some platforms.
        vp.pan_px(-(if d.0 != 0.0 { d.0 } else { d.1 }), 0.0);
    } else {
        vp.pan_px(-d.0, -d.1);
    }
}

/// Bar ruler for a horizontal view. `arrange` selects which viewport.
pub fn ruler(editor: Entity<Editor>, arrange: bool) -> impl IntoElement {
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
                    .filter(|(eid, _, _, _, _)| *eid == editor.entity_id())
                    .map(|(_, _, _, s, e)| (s, e));
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
                window.paint_quad(fill(b(0.0, hgt - 1.0, w, 1.0, o), th.separator));
            }
            flush_texts(window, cx, bounds, texts);
            if over_loop_handle || cx.global::<RollExtraDragState>().loop_drag.is_some_and(|(eid, arr, _, _, _)| eid == editor.entity_id() && arr == arrange) {
                window.set_cursor_style(CursorStyle::ResizeLeftRight, &hitbox);
            }
            if arrange {
                crate::arrange::install_timeline_scroll(&editor, bounds, &hitbox, window);
            } else {
                install_roll_scroll(&editor, bounds, &hitbox, ScrollSurface::Timeline, window);
            }
            crate::ruler_navigation::install(&editor, arrange, bounds, &hitbox, window, cx);
            let (e, h_down) = (editor.clone(), hitbox.clone());
            window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || !h_down.is_hovered(window) {
                    return;
                }
                if !matches!(ev.button, MouseButton::Left | MouseButton::Right) { return; }
                if ev.button == MouseButton::Left && window.captured_hitbox().is_some() { return; }
                let local = ev.position - bounds.origin;
                let (x, y) = (local.x.as_f32() as f64, local.y.as_f32() as f64);
                let mut started_loop_drag = None;
                let mut scrub = false;
                e.update(cx, |ed, cx| {
                    let vp = if arrange { ed.arrange } else { ed.session.viewport };
                    let tick=ed.session.snap.snap(vp.start_tick+x/vp.px_per_tick).max(0);
                    if ev.button==MouseButton::Right {
                        use crate::menu::MenuItem as I;use crate::editor::{MenuAction as M,WorkflowAction as A};
                        let tempo=ed.project().tempo_map.iter().rev().find(|p|p.tick<=tick).map(|p|p.tick).unwrap_or(0);
                        let meter=ed.project().time_signatures.iter().rev().find(|p|p.tick<=tick).map(|p|p.tick).unwrap_or(0);
                        let mut items=vec![I::action("在此处编辑 / 新增速度…",M::Workflow(A::EditTempo(tick))),I::action("删除前一速度事件",M::Workflow(A::DeleteTempo(tempo))).enabled(tempo>0),I::action("在此处编辑 / 新增拍号…",M::Workflow(A::EditMeter(tick))),I::action("删除前一拍号事件",M::Workflow(A::DeleteMeter(meter))).enabled(meter>0),I::separator(),I::action("设循环起点",M::Workflow(A::LoopStart(tick))),I::action("设循环终点",M::Workflow(A::LoopEnd(tick))),I::action("循环播放开关",M::Workflow(A::ToggleLoop))];
                        if !arrange {
                            items.push(I::separator());
                            items.push(I::action("添加多语言读音器分段…", M::Workflow(A::Integration(crate::editor::integrations::IntegrationAction::AddSegment))));
                        }
                        ed.show_menu(ev.position,items,window,cx);
                        return;
                    }
                    if ev.click_count >= 2 && !arrange {
                        {
                            if let Some(segment) = ed.language_segments.at(&ed.session.track, tick) {
                                let key = segment.key.clone();
                                let _ = ed.open_segment_dialog(Some(key), window, cx);
                                return;
                            }
                        }
                    }
                    let pb = ed.playback_settings();
                    if pb.loop_end > pb.loop_start && y <= 12.0 {
                        let a = vp.tick_to_x(pb.loop_start as f64);
                        let z = vp.tick_to_x(pb.loop_end as f64);
                        if (x - a).abs() <= 6.5 {
                            started_loop_drag = Some((true, pb.loop_start, pb.loop_end));
                            return;
                        } else if (x - z).abs() <= 6.5 {
                            started_loop_drag = Some((false, pb.loop_start, pb.loop_end));
                            return;
                        }
                    }
                    let min_loop_len = ed.project().ppq.max(1);
                    if ev.modifiers.shift{ed.edit_playback("循环起点",cx,|s|{s.loop_start=tick;if s.loop_end<=tick{s.loop_end=tick+min_loop_len;}s.looping=true;});}
                    else if ev.modifiers.alt{ed.edit_playback("循环终点",cx,|s|{s.loop_end=tick.max(s.loop_start+1);s.looping=true;});}
                    else { scrub = true; }
                });
                if let Some((drag_start, ls, le)) = started_loop_drag {
                    cx.global_mut::<RollExtraDragState>().loop_drag = Some((e.entity_id(), arrange, drag_start, ls, le));
                    window.capture_pointer(h_down.id);
                    cx.stop_propagation();
                } else if scrub && crate::ruler_navigation::begin(&e, arrange, bounds, &h_down, window, cx, ev.position) {
                    cx.stop_propagation();
                }
            });
            let e_move = editor.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble { return; }
                let next = {
                    let Some((eid, arr, drag_start, ls, le)) = cx.try_global::<RollExtraDragState>().and_then(|s| s.loop_drag) else { return };
                    if eid != e_move.entity_id() || arr != arrange { return; }
                    let ed = e_move.read(cx);
                    let vp = if arrange { ed.arrange } else { ed.session.viewport };
                    let lx = (ev.position.x - bounds.origin.x).as_f32() as f64;
                    let snapped = ed.session.snap.snap(vp.start_tick + lx / vp.px_per_tick).max(0);
                    let min_step = ed.project().ppq.max(1);
                    if drag_start {
                        (snapped.min(le - 1).max(0), le)
                    } else {
                        (ls, snapped.max(ls + min_step.min(120)))
                    }
                };
                if let Some(tuple) = cx.global_mut::<RollExtraDragState>().loop_drag.as_mut() {
                    tuple.3 = next.0;
                    tuple.4 = next.1;
                }
                e_move.update(cx, |_, cx| cx.notify());
            });
            let e_up = editor.clone();
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left { return; }
                let matched = cx.try_global::<RollExtraDragState>()
                    .and_then(|s| s.loop_drag)
                    .is_some_and(|(eid, arr, _, _, _)| eid == e_up.entity_id() && arr == arrange);
                if matched {
                    let finished = cx.global_mut::<RollExtraDragState>().loop_drag.take();
                    window.release_pointer();
                    if let Some((_, _, _, ls, le)) = finished {
                        e_up.update(cx, |ed, cx| {
                            ed.edit_playback("拖拽循环范围", cx, |s| {
                                s.loop_start = ls;
                                s.loop_end = le.max(ls + 1);
                                s.looping = true;
                            });
                            cx.notify();
                        });
                    }
                }
            });
        },
    )
    .size_full()
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
pub fn keyboard(editor: Entity<Editor>) -> impl IntoElement {
    canvas(
        |bounds, window, cx| {
            let pressed = window.use_keyed_state("piano-key-pressed", cx, |_, _| None::<i32>);
            (window.insert_hitbox(bounds, HitboxBehavior::Normal), pressed)
        },
        move |bounds, (hitbox, pressed), window, cx| {
            let mut texts = Texts::new();
            let pressed_pitch = *pressed.read(cx);
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
                            if pressed_pitch == Some(midi) {
                                window.paint_quad(fill(b(0.0, y, key_width, height, o), with_alpha(th.accent, 0.6)));
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
                window.paint_quad(fill(b(w - 1.0, 0.0, 1.0, bounds.size.height.as_f32() as f64, o), th.separator));
            }
            flush_texts(window, cx, bounds, texts);
            let (e, h, key) = (editor.clone(), hitbox.clone(), pressed.clone());
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || event.button != MouseButton::Left || !h.is_hovered(window) { return; }
                let p = event.position - bounds.origin;
                let pitch = keyboard_pitch_at(e.read(cx).session.viewport, p.x.as_f32() as f64, p.y.as_f32() as f64,
                    bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
                let Some(pitch) = pitch else { return; };
                key.update(cx, |pressed, cx| { *pressed = Some(pitch); cx.notify(); });
                window.capture_pointer(h.id);
                e.update(cx, |ed, cx| {
                    ed.audition_pitch(pitch);
                    window.focus(&ed.focus_roll, cx);
                    cx.notify();
                });
                cx.stop_propagation();
            });
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left { return; }
                pressed.update(cx, |pressed, cx| {
                    if pressed.take().is_some() {
                        window.release_pointer();
                        cx.notify();
                    }
                });
            });
            install_roll_scroll(&editor, bounds, &hitbox, ScrollSurface::Keyboard, window);
        },
    )
    .size_full()
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
mod scroll_tests {
    use super::*;

    #[test]
    fn shift_wheel_accepts_normalized_x_and_vertical_y_in_pixels_and_lines() {
        let initial = Viewport { start_tick: 2000.0, ..Viewport::default() };
        for delta in [
            ScrollDelta::Pixels(gpui::point(px(-40.0), px(0.0))),
            ScrollDelta::Pixels(gpui::point(px(0.0), px(-40.0))),
            ScrollDelta::Lines(gpui::point(-1.0, 0.0)),
            ScrollDelta::Lines(gpui::point(0.0, -1.0)),
            ScrollDelta::Pixels(gpui::point(px(-40.0), px(80.0))),
        ] {
            let mut vp = initial;
            scroll_viewport(&mut vp, delta, gpui::Modifiers { shift: true, ..Default::default() }, (300.0, 200.0));
            assert_eq!(vp.start_tick, initial.start_tick + 40.0 / initial.px_per_tick);
            assert_eq!(vp.top_pitch, initial.top_pitch);
            assert_eq!(vp.px_per_tick, initial.px_per_tick);
            assert_eq!(vp.row_height, initial.row_height);
        }
    }

    #[test]
    fn wheel_zoom_modifiers_keep_priority_and_anchor_over_shift_pan() {
        let initial = Viewport { start_tick: 2000.0, ..Viewport::default() };
        let anchor = (300.0, 200.0);
        let delta = ScrollDelta::Lines(gpui::point(1.0, 1.0));
        for modifiers in [
            gpui::Modifiers { control: true, alt: true, shift: true, ..Default::default() },
            gpui::Modifiers { platform: true, alt: true, shift: true, ..Default::default() },
        ] {
            let mut vp = initial;
            scroll_viewport(&mut vp, delta, modifiers, anchor);
            assert!(vp.px_per_tick > initial.px_per_tick);
            assert!((vp.x_to_tick(anchor.0) - initial.x_to_tick(anchor.0)).abs() < 1e-9);
            assert_eq!(vp.top_pitch, initial.top_pitch);
            assert_eq!(vp.row_height, initial.row_height);
        }
        let mut vp = initial;
        scroll_viewport(&mut vp, delta, gpui::Modifiers { alt: true, shift: true, ..Default::default() }, anchor);
        assert!(vp.row_height > initial.row_height);
        assert!((vp.y_to_pitch(anchor.1) - initial.y_to_pitch(anchor.1)).abs() < 1e-9);
        assert_eq!(vp.start_tick, initial.start_tick);
        assert_eq!(vp.px_per_tick, initial.px_per_tick);
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
