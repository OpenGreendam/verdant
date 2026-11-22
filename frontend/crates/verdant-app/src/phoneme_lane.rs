//! Timeline-aligned SVS Phoneme Timing Lane rendered directly under the Piano Roll.
//! Shares `ed.session.viewport` (`tick_to_x`) 1:1 with notes and F0 curves, displays
//! actual phoneme symbols for every visible note, visualizes vowel-on-beat consonant
//! pre-utterance overhang, and lets users drag phoneme boundaries directly in context.

use gpui::{
    BorderStyle, Bounds, CursorStyle, DispatchPhase, Entity, Global, Hitbox, HitboxBehavior,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Window,
    canvas, fill, point, prelude::*, px, quad,
};
use verdant_domain::{NoteId, Revision, TrackId};
use verdant_session::PointerId;

use crate::editor::Editor;
use crate::paint::{Texts, b, flush_texts};
use crate::theme::{mix, with_alpha};

pub const PHONEME_LANE_H: f32 = 30.0;

#[derive(Default)]
struct TimelinePhonemeDragState {
    drag: Option<TimelinePhonemeDrag>,
}
impl Global for TimelinePhonemeDragState {}

#[derive(Clone)]
struct TimelinePhonemeDrag {
    editor: gpui::EntityId,
    track: TrackId,
    note: NoteId,
    revision: Revision,
    pointer: PointerId,
    boundary: usize,
    note_start_seconds: f64,
    total_seconds: f64,
    position_seconds: f64,
    lower_seconds: f64,
    upper_seconds: f64,
}

impl TimelinePhonemeDrag {
    fn valid(&self) -> bool {
        self.position_seconds.is_finite()
            && self.position_seconds >= self.lower_seconds - 1e-6
            && self.position_seconds <= self.upper_seconds + 1e-6
    }

    fn matches(&self, editor: &Entity<Editor>, ed: &Editor) -> bool {
        self.editor == editor.entity_id() && self.track == ed.session.track && self.revision == ed.revision()
    }
}

pub fn short_phoneme_label(raw: &str) -> &str {
    raw.split(" → ").next().unwrap_or(raw).trim()
}

pub fn timeline_phoneme_lane(editor:Entity<Editor>)->impl IntoElement {
    let draw=editor.clone();let cancel=editor.clone();
    crate::proto_editor_surface::surface(editor,"timeline-phonemes","时间线音素边界",0.0,
        move |input|native_timeline_phonemes(draw.clone(),input).into_any_element(),
        move |_,window,cx| {
            let owned=cx.try_global::<TimelinePhonemeDragState>().and_then(|s|s.drag.as_ref()).is_some_and(|d|d.editor==cancel.entity_id());
            if owned { cx.global_mut::<TimelinePhonemeDragState>().drag=None;cancel.update(cx,|_,cx|cx.notify()); }
        })
}
fn native_timeline_phonemes(editor:Entity<Editor>,input:crate::proto_editor_surface::Input)->impl IntoElement {
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |bounds, hitbox, window, cx| {
            if cx.try_global::<TimelinePhonemeDragState>().is_none() {
                cx.set_global(TimelinePhonemeDragState::default());
            }
            let stale = cx.global::<TimelinePhonemeDragState>()
                .drag
                .as_ref()
                .is_some_and(|d| d.editor == editor.entity_id() && !d.matches(&editor, editor.read(cx)));
            if stale {
                cx.global_mut::<TimelinePhonemeDragState>().drag = None;
                input.cancel(window, cx);
            }

            let mut texts = Texts::new();
            let mut hovered_boundary = false;
            {
                let ed = editor.read(cx);
                let th = ed.theme;
                let vp = ed.session.viewport;
                let o = bounds.origin;
                let (w, h) = (bounds.size.width.as_f32() as f64, bounds.size.height.as_f32() as f64);
                let (t0, t1) = (vp.start_tick, vp.x_to_tick(w));
                let tempo = ed.tempo();
                let meter = ed.meter();
                let from = t0.ceil() as i64;
                let to = t1.ceil() as i64;
                let sub = ed.session.snap.grid_ticks.max(1);
                let beat = meter.beat_ticks_at(from);
                let active_drag = cx.global::<TimelinePhonemeDragState>()
                    .drag
                    .as_ref()
                    .filter(|d| d.matches(&editor, ed));

                window.paint_quad(fill(bounds, th.roll_black));
                window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                    let step = if sub as f64 * vp.px_per_tick >= 10.0 { sub } else { beat };
                    if step as f64 * vp.px_per_tick >= 10.0 {
                        for (tick, is_bar, _) in meter.grid(from, to, step) {
                            let gx = vp.tick_to_x(tick as f64);
                            window.paint_quad(fill(b(gx, 0.0, 1.0, h, o), if is_bar { th.grid_bar } else { th.grid_sub }));
                        }
                    } else {
                        for tick in meter.bars(from, to) {
                            window.paint_quad(fill(b(vp.tick_to_x(tick as f64), 0.0, 1.0, h, o), th.grid_bar));
                        }
                    }

                    let Some(track) = ed.active_track() else { return };
                    let track_color = th.track(track.color.as_deref());
                    let mouse_local_x = (window.mouse_position().x - o.x).as_f32() as f64;
                    let panel_hovered = hitbox.is_hovered(window);
                    let notes = ed.index.notes_in_range(track, t0.floor() as i64, t1.ceil() as i64);
                    let compact_band = |note: &verdant_domain::Note| {
                        let x = vp.tick_to_x(note.start_tick as f64);
                        let width = vp.tick_to_x(note.end_tick() as f64) - x;
                        (x, width)
                    };
                    // Keep a one-pixel minimum in time-scale overview. A
                    // two-pixel minimum visually joins neighbouring notes at
                    // this zoom, and prevents batching their distinct bands.
                    let mut previous_right = f64::NEG_INFINITY;
                    let compact_disjoint = notes.clone().all(|note| {
                        let (x, width) = compact_band(note);
                        if width >= 5.0 || ed.session.selection.contains(&note.id) || x < previous_right {
                            return false;
                        }
                        previous_right = x + width.max(1.0);
                        true
                    });
                    if compact_disjoint {
                        window.paint_layer(|window| {
                            for note in notes {
                                let (x, width) = compact_band(note);
                                let color = if ed.error_notes.contains(&note.id) { th.error } else { track_color };
                                window.paint_quad(fill(b(x, 3.0, width.max(1.0), h - 6.0, o), with_alpha(color, 0.35)));
                            }
                        });
                        return;
                    }
                    for note in notes {
                        let x_start = vp.tick_to_x(note.start_tick as f64);
                        let x_end = vp.tick_to_x(note.end_tick() as f64);
                        let nw = (x_end - x_start).max(2.0);
                        let is_selected = ed.session.selection.contains(&note.id);
                        if x_end - x_start < 5.0 && !is_selected {
                            let color = if ed.error_notes.contains(&note.id) { th.error } else { track_color };
                            window.paint_quad(fill(b(x_start, 3.0, (x_end - x_start).max(1.0), h - 6.0, o), with_alpha(color, 0.35)));
                            continue;
                        }

                        if let Some(prefix) = ed.scheduled_phoneme_prefix(&note.id) {
                            if prefix > 1e-4 {
                                let onset_s = tempo.seconds_at(note.start_tick as f64);
                                let pre_tick = tempo.tick_at((onset_s - prefix).max(0.0));
                                let pre_x = vp.tick_to_x(pre_tick);
                                if x_start > pre_x + 1.0 {
                                    window.paint_quad(quad(
                                        b(pre_x, 4.0, x_start - pre_x, h - 8.0, o),
                                        px(3.0),
                                        with_alpha(th.warning, 0.16),
                                        px(1.0),
                                        with_alpha(th.warning, 0.65),
                                        BorderStyle::Solid,
                                    ));
                                }
                            }
                        }

                        if note.is_continuation() {
                            window.paint_quad(quad(
                                b(x_start, 3.0, nw, h - 6.0, o),
                                px(3.0),
                                with_alpha(track_color, if is_selected { 0.30 } else { 0.16 }),
                                px(1.0),
                                if is_selected { th.accent } else { with_alpha(track_color, 0.45) },
                                BorderStyle::Solid,
                            ));
                            if nw >= 14.0 {
                                texts.push(("—".into(), point(o.x + px((x_start + 4.0) as f32), o.y + px(7.0)), 10.0, th.text_secondary));
                            }
                            continue;
                        }

                        let budget = ed.note_phone_budget(note, &tempo);

                        if let Ok(budget) = &*budget {
                            let note_s0 = tempo.seconds_at(note.start_tick as f64);
                            let mut elapsed = 0.0;
                            let preview = active_drag.filter(|d| d.note == note.id && d.valid()).and_then(|d| {
                                let weights = budget.weights_permille.as_ref()?;
                                let left: u32 = weights.get(..d.boundary.checked_sub(1)?)?.iter().sum();
                                let right: u32 = weights.get(..=d.boundary)?.iter().sum();
                                let end = (d.position_seconds / budget.total_seconds * 1000.0).round() as u32;
                                Some((d.boundary, budget.total_seconds * f64::from(end.saturating_sub(left)) / 1000.0,
                                    budget.total_seconds * f64::from(right.saturating_sub(end)) / 1000.0))
                            });
                            for (idx, (&original, raw_lbl)) in budget.durations_seconds.iter().zip(&budget.labels).enumerate() {
                                let dur = match preview {
                                    Some((boundary, left, _)) if idx + 1 == boundary => left,
                                    Some((boundary, _, right)) if idx == boundary => right,
                                    _ => original,
                                };
                                let seg_t0 = tempo.tick_at(note_s0 + elapsed);
                                elapsed += dur;
                                let seg_t1 = tempo.tick_at(note_s0 + elapsed);
                                let sx0 = vp.tick_to_x(seg_t0);
                                let sx1 = vp.tick_to_x(seg_t1);
                                let sw = (sx1 - sx0).max(1.0);
                                let is_consonant = budget.labels.len() > 1 && idx == 0;
                                let fill_col = if is_consonant {
                                    mix(th.roll_black, track_color, if is_selected { 0.42 } else { 0.26 })
                                } else {
                                    mix(th.roll_black, track_color, if is_selected { 0.28 } else { 0.16 })
                                };
                                window.paint_quad(quad(
                                    b(sx0, 3.0, sw, h - 6.0, o),
                                    px(2.0),
                                    fill_col,
                                    px(1.0),
                                    if is_selected { th.accent } else { with_alpha(track_color, 0.55) },
                                    BorderStyle::Solid,
                                ));
                                let lbl = short_phoneme_label(raw_lbl);
                                if sw >= 14.0 {
                                    texts.push((
                                        lbl.to_string(),
                                        point(o.x + px((sx0 + 3.0) as f32), o.y + px(7.0)),
                                        10.5,
                                        th.text,
                                    ));
                                }
                                if idx + 1 < budget.labels.len() {
                                    let drag_here = active_drag.filter(|d| d.note == note.id && d.boundary == idx + 1);
                                    let bx = if let Some(d) = drag_here {
                                        vp.tick_to_x(tempo.tick_at(note_s0 + d.position_seconds))
                                    } else {
                                        sx1
                                    };
                                    if panel_hovered && budget.editable && (mouse_local_x - bx).abs() <= 6.0 {
                                        hovered_boundary = true;
                                    }
                                    let handle_col = if drag_here.is_some_and(|d| !d.valid()) {
                                        th.error
                                    } else if drag_here.is_some() || (panel_hovered && budget.editable && (mouse_local_x - bx).abs() <= 6.0) {
                                        th.warning
                                    } else if budget.editable {
                                        th.accent
                                    } else {
                                        th.text_tertiary
                                    };
                                    window.paint_quad(fill(b(bx - 1.0, 2.0, 2.0, h - 4.0, o), handle_col));
                                    if budget.editable {
                                        window.paint_quad(fill(b(bx - 3.0, 2.0, 6.0, 4.0, o), handle_col));
                                    }
                                }
                            }
                        } else {
                            let fallback = if !note.pronunciation.pinyin.is_empty() {
                                note.pronunciation.pinyin.as_str()
                            } else if !note.lyric.is_empty() {
                                note.lyric.as_str()
                            } else {
                                "?"
                            };
                            let has_err = ed.error_notes.contains(&note.id);
                            window.paint_quad(quad(
                                b(x_start, 3.0, nw, h - 6.0, o),
                                px(3.0),
                                with_alpha(if has_err { th.error } else { track_color }, 0.2),
                                px(1.0),
                                if has_err { th.error } else if is_selected { th.accent } else { th.separator },
                                BorderStyle::Solid,
                            ));
                            if nw >= 16.0 {
                                texts.push((
                                    fallback.to_string(),
                                    point(o.x + px((x_start + 4.0) as f32), o.y + px(7.0)),
                                    10.0,
                                    if has_err { th.error } else { th.text_secondary },
                                ));
                            }
                        }
                    }
                });
                window.paint_quad(fill(b(0.0, h - 1.0, w, 1.0, o), th.shade));
            }
            flush_texts(window, cx, bounds, texts);

            let dragging = cx.global::<TimelinePhonemeDragState>()
                .drag
                .as_ref()
                .is_some_and(|d| d.editor == editor.entity_id());
            if hovered_boundary || dragging {
                window.set_cursor_style(CursorStyle::ResizeLeftRight, &hitbox);
            }
            install_timeline_phoneme_input(&editor,bounds,&hitbox,&input,window);
            crate::roll::install_roll_scroll(&editor, bounds, &hitbox, crate::roll::ScrollSurface::Timeline,&input,window);
        },
    )
    .size_full()
}

fn install_timeline_phoneme_input(editor:&Entity<Editor>,bounds:Bounds<Pixels>,hitbox:&Hitbox,
    input:&crate::proto_editor_surface::Input,window:&mut Window) {
    let editor=editor.clone();
    let policy=input.clone();
    crate::proto_editor_surface::install_pointer_input(input,bounds,hitbox,window,move |event,window,cx| {
        use crate::proto_editor_surface::PointerPhase;
        let local_x=event.x;
        match event.phase {
            PointerPhase::Begin => {
        let hit = {
            let ed = editor.read(cx);
            let vp = ed.session.viewport;
            let tempo = ed.tempo();
            let Some(track) = ed.active_track() else { return false };
            let click_tick = vp.x_to_tick(local_x);
            let mut boundary_hit = None;
            if ed.scheduler.capabilities.is_some() {
                for note in ed.index.notes_in_range(track, (click_tick - ed.project().ppq as f64) as i64, (click_tick + ed.project().ppq as f64) as i64) {
                    let budget = ed.note_phone_budget(note, &tempo);
                    let Ok(budget) = &*budget else { continue };
                    if !budget.editable { continue; }
                    let Some(weights) = &budget.weights_permille else { continue };
                    let note_s0 = tempo.seconds_at(note.start_tick as f64);
                    let mut sum = 0u32;
                    for (idx, &w_permille) in weights.iter().take(weights.len().saturating_sub(1)).enumerate() {
                        sum += w_permille;
                        let pos_s = budget.total_seconds * sum as f64 / 1000.0;
                        let bx = vp.tick_to_x(tempo.tick_at(note_s0 + pos_s));
                        let dist = (local_x - bx).abs();
                        if dist <= 6.5 && boundary_hit.as_ref().is_none_or(|&(_, _, best): &(TimelinePhonemeDrag, NoteId, f64)| dist < best) {
                            let left: u32 = weights[..idx].iter().sum();
                            let right: u32 = weights[..=idx + 1].iter().sum();
                            let min_pm = budget.minimum_permille();
                            boundary_hit = Some((
                                TimelinePhonemeDrag {
                                    editor: editor.entity_id(),
                                    track: ed.session.track.clone(),
                                    note: note.id.clone(),
                                    revision: ed.revision(),
                                    pointer:event.pointer,
                                    boundary: idx + 1,
                                    note_start_seconds: note_s0,
                                    total_seconds: budget.total_seconds,
                                    position_seconds: pos_s,
                                    lower_seconds: budget.total_seconds * (left + min_pm) as f64 / 1000.0,
                                    upper_seconds: budget.total_seconds * right.saturating_sub(min_pm) as f64 / 1000.0,
                                },
                                note.id.clone(),
                                dist,
                            ));
                        }
                    }
                }
            }
            if let Some((drag, nid, _)) = boundary_hit {
                Some((Some(drag), Some(nid)))
            } else {
                let clicked_note = ed.index.notes_in_range(track, click_tick.floor() as i64, click_tick.ceil() as i64)
                    .find(|n| (n.start_tick as f64..=n.end_tick() as f64).contains(&click_tick))
                    .map(|n| n.id.clone());
                Some((None, clicked_note))
            }
        };

        if let Some((maybe_drag, maybe_note)) = hit {
            let intent=editor.update(cx,|ed,cx|policy.intent(ed,"phonemeGesture",vec![maybe_drag.is_some().into(),
                maybe_note.is_some().into(),maybe_drag.is_some().into()],window,cx));
            let Some(intent)=intent else{return false;};
            editor.update(cx, |ed, cx| {
                if intent.selection=="replace" && let Some(nid) = maybe_note {
                    ed.session.selection = [nid].into();
                }
                cx.notify();
            });
            if intent.kind=="phoneme-drag" && let Some(drag) = maybe_drag {
                cx.global_mut::<TimelinePhonemeDragState>().drag = Some(drag);
                return true;
            }
        }
                false
            }
            PointerPhase::Sample => {
        let next_pos = {
            let Some(drag) = cx.try_global::<TimelinePhonemeDragState>().and_then(|s| s.drag.as_ref()) else { return false };
            let ed = editor.read(cx);
            if !drag.matches(&editor, ed) {
                return false;
            }
            let tick = ed.session.viewport.x_to_tick(local_x).max(0.0);
            let abs_s = ed.tempo().seconds_at(tick);
            Some(abs_s - drag.note_start_seconds)
        };
        if let Some(pos_s) = next_pos {
            if let Some(drag) = cx.global_mut::<TimelinePhonemeDragState>().drag.as_mut() {
                drag.position_seconds = pos_s;
            }
            editor.update(cx, |_, cx| cx.notify());
        }
                true
            }
            PointerPhase::Finish => {
        let matched = cx.try_global::<TimelinePhonemeDragState>()
            .and_then(|s| s.drag.as_ref())
            .is_some_and(|d| d.editor == editor.entity_id() && d.pointer == event.pointer);
        if !matched {
            return false;
        }
        let Some(drag) = cx.global_mut::<TimelinePhonemeDragState>().drag.take() else { return false };

        editor.update(cx, |ed, cx| {
            if !drag.matches(&editor, ed) {
                ed.notice("工程或选区已变化，取消本次音素边界调整");
                cx.notify();
                return;
            }
            let clamped = drag.position_seconds.clamp(drag.lower_seconds, drag.upper_seconds);
            let res = ed.scheduler.capabilities.as_ref()
                .ok_or_else(|| "后端能力尚未就绪".to_string())
                .and_then(|caps| verdant_backend::phoneme_timing::drag_boundary(
                    ed.project(), &drag.track, &drag.note, caps, drag.boundary, clamped,
                ));
            match res {
                Ok(batch) => { ed.commit(drag.revision, batch, cx); }
                Err(err) => { ed.notice(err); cx.notify(); }
            }
        });
                true
            }
            PointerPhase::Context => false,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_phoneme_label_strips_model_mapping_arrow() {
        assert_eq!(short_phoneme_label("kː → k"), "kː");
        assert_eq!(short_phoneme_label("sh"), "sh");
        assert_eq!(short_phoneme_label("ang4"), "ang4");
    }
}
