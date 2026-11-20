//! Hit testing (SPEC §5.1).
//!
//! Priority at one position: selected-note resize handle > note body >
//! background. Overlapping notes resolve selected-first, then topmost (last
//! drawn). Handle zones are `min(6 px, w/3)` inside each edge of a *selected*
//! note plus 3 px outside it, and where the two zones would overlap on a short
//! note the tail wins (explicit rule; precise edits remain available in the
//! inspector).

use std::collections::BTreeSet;

use verdant_domain::{Edge, NoteId, Track};

use crate::viewport::{MIN_NOTE_BODY_PX, Viewport};

pub const HANDLE_PX: f64 = 6.0;
pub const HANDLE_OUTSIDE_PX: f64 = 3.0;
/// Finger contact is imprecise: larger edge zones for touch input.
pub const TOUCH_HANDLE_PX: f64 = 14.0;
pub const TOUCH_HANDLE_OUTSIDE_PX: f64 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Background,
    Body(NoteId),
    Handle(NoteId, Edge),
}

pub fn hit_test(track: &Track, vp: &Viewport, selection: &BTreeSet<NoteId>, x: f64, y: f64) -> Hit {
    hit_test_with(track, vp, selection, x, y, HANDLE_PX, HANDLE_OUTSIDE_PX)
}

/// Hit test with touch-sized edge zones.
pub fn hit_test_touch(track: &Track, vp: &Viewport, selection: &BTreeSet<NoteId>, x: f64, y: f64) -> Hit {
    hit_test_with(track, vp, selection, x, y, TOUCH_HANDLE_PX, TOUCH_HANDLE_OUTSIDE_PX)
}

pub fn hit_test_with(track: &Track, vp: &Viewport, selection: &BTreeSet<NoteId>, x: f64, y: f64, handle_px: f64, outside_px: f64) -> Hit {
    let tick = vp.x_to_tick(x);
    let slack = (outside_px + handle_px).max(MIN_NOTE_BODY_PX) / vp.px_per_tick;
    let candidates: Vec<_> = track.notes_in_range((tick - slack).floor() as i64, (tick + slack).ceil() as i64 + 1).collect();

    // 1. Handles of selected notes.
    for n in candidates.iter().rev().filter(|n| selection.contains(&n.id)) {
        let r = vp.note_rect(n);
        if y < r.y || y >= r.bottom() {
            continue;
        }
        let zone = handle_px.min(r.w / 3.0);
        let in_end = x >= r.right() - zone && x <= r.right() + outside_px;
        let in_start = x >= r.x - outside_px && x < r.x + zone;
        if in_end {
            return Hit::Handle(n.id.clone(), Edge::End);
        }
        if in_start {
            return Hit::Handle(n.id.clone(), Edge::Start);
        }
    }
    // 2. Bodies: selected first, then topmost.
    let mut body: Option<&NoteId> = None;
    for n in candidates.iter().rev() {
        if vp.note_body_rect(n).contains(x, y) {
            if selection.contains(&n.id) {
                return Hit::Body(n.id.clone());
            }
            body.get_or_insert(&n.id);
        }
    }
    body.map(|id| Hit::Body(id.clone())).unwrap_or(Hit::Background)
}
