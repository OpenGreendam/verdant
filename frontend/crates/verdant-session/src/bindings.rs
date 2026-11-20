//! Explicit domain/session bindings for the authored Proto UI planner.
//! The compiled source chooses guards, transitions and ordered effects;
//! this file implements only the named operations over immutable input and
//! the real EditorSession/ProjectStore command boundary.

use serde::{Deserialize, Serialize};
use verdant_pitch_gesture_surface::PitchGestureSurfaceHandle;

use crate::session::{DRAG_THRESHOLD_PX, TOUCH_DRAG_THRESHOLD_PX};
use crate::{EditorSession, Effect, Gesture, GestureKind, Hit, InputEvent, Phase, PointerId, Preview};
use verdant_domain::{Command, CommandBatch, Edge, NoteId, Project, Revision};
use verdant_domain::command::{clamp_group_move, resized};

#[derive(Clone, Copy)]
pub(crate) enum Machine { Move, Duplicate, Resize, Marquee }

impl Machine {
    fn active(session: &EditorSession) -> Option<Self> {
        session.gesture.as_ref().map(|g| match g.kind {
            GestureKind::Move { duplicate: false } => Self::Move,
            GestureKind::Move { duplicate: true } => Self::Duplicate,
            GestureKind::Resize { .. } => Self::Resize,
            GestureKind::RubberBand { .. } => Self::Marquee,
        })
    }

    fn as_str(self) -> &'static str {
        match self { Self::Move => "move", Self::Duplicate => "duplicate", Self::Resize => "resize", Self::Marquee => "marquee" }
    }
}

#[derive(Default, Serialize)]
struct GuardFacts {
    hit_background: bool,
    hit_selected_note: bool,
    hit_unselected_note: bool,
    hit_note_body: bool,
    hit_resize_handle: bool,
    toggle_modifier: bool,
    extend_modifier: bool,
    duplicate_modifier: bool,
    pointer_threshold_crossed: bool,
    base_revision_matches: bool,
    base_revision_matches_and_delta_nonzero: bool,
}

fn guard_facts(session: &EditorSession, project: &Project, revision: Revision, event: &InputEvent) -> GuardFacts {
    let mut facts = GuardFacts::default();
    match event {
        InputEvent::PointerDown { hit, mods, .. } if session.gesture.is_none() => {
            // All idle candidates share this immutable hit lookup and selection snapshot.
            let track = project.track(&session.track);
            let note = match hit {
                Hit::Body(id) | Hit::Handle(id, _) => track.and_then(|t| t.note(id)),
                _ => None,
            };
            facts.hit_note_body = matches!(hit, Hit::Body(_)) && note.is_some();
            facts.hit_resize_handle = matches!(hit, Hit::Handle(_, _)) && note.is_some();
            facts.hit_background = matches!(hit, Hit::Background) && track.is_some();
            if facts.hit_note_body {
                facts.hit_selected_note = note.is_some_and(|n| session.selection.contains(&n.id));
                facts.hit_unselected_note = !facts.hit_selected_note;
            }
            facts.toggle_modifier = mods.secondary;
            facts.extend_modifier = mods.shift && !mods.secondary;
            facts.duplicate_modifier = mods.alt;
        }
        InputEvent::PointerMove { screen, .. } => {
            if let Some(g) = session.gesture.as_ref().filter(|g| g.phase == Phase::Armed) {
                let dx = screen.0 - g.origin_screen.0;
                let dy = screen.1 - g.origin_screen.1;
                let threshold = if matches!(session.pointer_owner(), Some(PointerId::Touch(_))) { TOUCH_DRAG_THRESHOLD_PX } else { DRAG_THRESHOLD_PX };
                facts.pointer_threshold_crossed = dx * dx + dy * dy > threshold * threshold;
            }
        }
        InputEvent::PointerUp => {
            if let Some(g) = session.gesture.as_ref().filter(|g| g.phase == Phase::Dragging) {
                facts.base_revision_matches = g.base_revision == revision;
                if facts.base_revision_matches {
                    // Resize can scan the selected notes. Only a dragging release
                    // needs the delta, and a stale revision never needs that scan.
                    facts.base_revision_matches_and_delta_nonzero = match g.preview {
                        Preview::Move { delta_tick, delta_semitones, .. } => delta_tick != 0 || delta_semitones != 0,
                        Preview::Resize { edge, delta_tick } => project.track(&g.track).is_some_and(|track| {
                            g.ids.iter().filter_map(|id| track.note(id)).any(|n| resized(n, edge, delta_tick) != (n.start_tick, n.duration_tick))
                        }),
                        _ => false,
                    };
                }
            }
        }
        _ => {}
    }
    facts
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
enum NoteGesturePhase {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "armed")]
    Armed,
    #[serde(rename = "dragging")]
    Dragging,
}

impl NoteGesturePhase {
    fn current(session: &EditorSession) -> Self {
        match session.gesture.as_ref().map(|g| g.phase) {
            None => Self::Idle,
            Some(Phase::Armed) => Self::Armed,
            Some(Phase::Dragging) => Self::Dragging,
        }
    }

    fn as_str(self) -> &'static str {
        match self { Self::Idle => "idle", Self::Armed => "armed", Self::Dragging => "dragging" }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteGesturePlan {
    #[serde(rename = "nextPhase")]
    next_phase: NoteGesturePhase,
    consumed: bool,
    effects: Vec<Op>,
}

/// Effect implementations are explicit bindings, shared across protocol
/// units. This is an operation vocabulary, not another transition machine.
#[derive(Clone, Copy, Deserialize)]
enum Op {
    #[serde(rename = "Session.SelectOnlyHitNote")]
    SelectOnly,
    #[serde(rename = "Session.ToggleHitNote")]
    Toggle,
    #[serde(rename = "Session.ExtendSelectionToHit")]
    Extend,
    #[serde(rename = "Session.SelectIfUnselectedHit")]
    SelectIfUnselected,
    #[serde(rename = "Session.BeginMoveGesture")]
    BeginMove,
    #[serde(rename = "Session.BeginDuplicateGesture")]
    BeginDuplicate,
    #[serde(rename = "Session.BeginResizeGesture")]
    BeginResize,
    #[serde(rename = "Session.BeginMarqueeGesture")]
    BeginMarquee,
    #[serde(rename = "Session.UpdateMovePreview")]
    UpdateMove,
    #[serde(rename = "Session.UpdateResizePreview")]
    UpdateResize,
    #[serde(rename = "Session.UpdateMarqueePreview")]
    UpdateMarquee,
    #[serde(rename = "Domain.CommitMoveNotes")]
    CommitMove,
    #[serde(rename = "Domain.CommitDuplicateNotes")]
    CommitDuplicate,
    #[serde(rename = "Domain.CommitResizeNotes")]
    CommitResize,
    #[serde(rename = "Session.CommitMarqueeSelection")]
    CommitMarquee,
    #[serde(rename = "Session.ClickReleaseSelection")]
    ClickSelection,
    #[serde(rename = "Session.ReportConflictIfStale")]
    Conflict,
    #[serde(rename = "Session.ClearGesturePreview")]
    Clear,
    #[serde(rename = "Host.CapturePointer")]
    Capture,
    #[serde(rename = "Host.ReleasePointer")]
    Release,
}

fn plan(machine: Machine, phase: NoteGesturePhase, event: &InputEvent, facts: &GuardFacts) -> NoteGesturePlan {
    let event = match event {
        InputEvent::PointerDown { .. } => "PointerDown",
        InputEvent::PointerMove { .. } => "PointerMove",
        InputEvent::PointerUp => "PointerUp",
        InputEvent::Cancel(_) => "Cancel",
    };
    let decision = PitchGestureSurfaceHandle::call_data_method("planNoteGesture", vec![
        machine.as_str().into(),
        phase.as_str().into(),
        event.into(),
        serde_json::to_value(facts).expect("serializable note gesture guard facts"),
    ]).unwrap_or_else(|error| panic!("compiled note gesture planner failed: {error:?}"));
    // Decode every phase and effect before any operation can mutate the session.
    serde_json::from_value(decision)
        .unwrap_or_else(|error| panic!("compiled note gesture planner returned an incompatible decision: {error}"))
}

pub(crate) fn dispatch(session: &mut EditorSession, project: &Project, revision: Revision, event: &InputEvent, pointer: PointerId) -> Vec<Effect> {
    let active = Machine::active(session);
    let idle = [Machine::Resize, Machine::Duplicate, Machine::Move, Machine::Marquee];
    let candidates = match &active { Some(machine) => std::slice::from_ref(machine), None => &idle };
    let phase = NoteGesturePhase::current(session);
    let facts = guard_facts(session, project, revision, event);
    // Source-selected SelectOnly / Extend / Toggle operations may change
    // selection before Begin, so retain the pre-effect click semantics.
    let click_only = facts.hit_selected_note && !facts.toggle_modifier && !facts.extend_modifier && session.selection.len() > 1;
    let mut out = Vec::new();
    for &machine in candidates {
        let decision = plan(machine, phase, event, &facts);
        if decision.effects.is_empty() {
            assert_eq!(decision.next_phase, phase, "compiled note gesture planner changed phase without effects");
        } else {
            for op in decision.effects {
                apply(op, session, project, revision, event, pointer, click_only, &mut out);
            }
            match decision.next_phase {
                NoteGesturePhase::Idle => session.gesture = None,
                NoteGesturePhase::Armed | NoteGesturePhase::Dragging => {
                    let phase = if decision.next_phase == NoteGesturePhase::Armed { Phase::Armed } else { Phase::Dragging };
                    session.gesture.as_mut().expect("compiled note gesture planner retained an active phase without a gesture").phase = phase;
                }
            }
        }
        if decision.consumed { break; }
    }
    out
}

fn hit_id(event: &InputEvent) -> Option<&NoteId> {
    match event { InputEvent::PointerDown { hit: Hit::Body(id) | Hit::Handle(id, _), .. } => Some(id), _ => None }
}

#[allow(clippy::too_many_arguments)]
fn apply(op: Op, session: &mut EditorSession, project: &Project, revision: Revision, event: &InputEvent, pointer: PointerId, click_only: bool, out: &mut Vec<Effect>) {
    match op {
        Op::SelectOnly => { if let Some(id) = hit_id(event) { session.selection = [id.clone()].into(); } }
        Op::SelectIfUnselected => { if let Some(id) = hit_id(event) { if !session.selection.contains(id) { session.selection = [id.clone()].into(); } } }
        Op::Toggle => { if let Some(id) = hit_id(event) { if !session.selection.remove(id) { session.selection.insert(id.clone()); } } }
        Op::Extend => { if let Some(id) = hit_id(event) { session.extend_selection_to(project, id); } }
        Op::BeginMove | Op::BeginDuplicate | Op::BeginResize | Op::BeginMarquee => {
            let InputEvent::PointerDown { music, screen, mods, .. } = event else { return; };
            let kind = match op {
                Op::BeginMove => GestureKind::Move { duplicate: false },
                Op::BeginDuplicate => GestureKind::Move { duplicate: true },
                Op::BeginResize => {
                    let InputEvent::PointerDown { hit: Hit::Handle(_, edge), .. } = event else { return; };
                    GestureKind::Resize { edge: *edge }
                }
                Op::BeginMarquee => GestureKind::RubberBand { additive: mods.shift || mods.secondary },
                _ => unreachable!(),
            };
            let ids = if matches!(kind, GestureKind::RubberBand { .. }) { Vec::new() } else { session.selection_in_order(project) };
            session.gesture = Some(Gesture {
                kind, phase: Phase::Armed, track: session.track.clone(), base_revision: revision,
                origin_screen: *screen, origin: *music, anchor: hit_id(event).cloned(), ids,
                preview: Preview::None, click_selects_only_anchor: click_only,
            });
        }
        Op::UpdateMove | Op::UpdateResize | Op::UpdateMarquee => update_preview(op, session, project, event),
        Op::CommitMove | Op::CommitDuplicate | Op::CommitResize => commit(op, session, out),
        Op::CommitMarquee => {
            if let Some(Gesture { kind, preview: Preview::RubberBand { hits, .. }, .. }) = &session.gesture {
                if !matches!(kind, GestureKind::RubberBand { additive: true }) { session.selection.clear(); }
                session.selection.extend(hits.iter().cloned());
            }
        }
        Op::ClickSelection => {
            if let Some(g) = &session.gesture {
                match (&g.kind, &g.anchor) {
                    (GestureKind::RubberBand { additive: false }, _) => session.selection.clear(),
                    (GestureKind::Move { .. }, Some(id)) if g.click_selects_only_anchor => session.selection = [id.clone()].into(),
                    _ => {}
                }
            }
        }
        Op::Conflict => {
            if session.gesture.as_ref().is_some_and(|g| g.base_revision != revision) { out.push(Effect::Notice("拖拽开始后工程已变化，未提交".into())); }
        }
        Op::Clear => { session.gesture = None; session.autoscroll_active = false; }
        Op::Capture => session.capture_pointer(pointer, out),
        Op::Release => session.release_pointer(out),
    }
}

fn update_preview(op: Op, session: &mut EditorSession, project: &Project, event: &InputEvent) {
    let InputEvent::PointerMove { music, screen, mods } = event else { return; };
    let snap = if mods.alt && !matches!(session.gesture.as_ref().map(|g| &g.kind), Some(GestureKind::Move { duplicate: true })) {
        crate::Snap { grid_ticks: 0 }
    } else { session.snap };
    let Some(g) = &mut session.gesture else { return; };
    let Some(track) = project.track(&g.track) else { return; };
    match op {
        Op::UpdateMove => {
            let Some(anchor) = g.anchor.as_ref().and_then(|id| track.note(id)) else { return; };
            let mut dt = snap.snap(anchor.start_tick as f64 + music.tick - g.origin.tick) - anchor.start_tick;
            let mut ds = music.pitch.floor() as i32 - g.origin.pitch.floor() as i32;
            if mods.shift {
                let dx = screen.0 - g.origin_screen.0;
                let dy = screen.1 - g.origin_screen.1;
                if dx.abs() >= dy.abs() { ds = 0; } else { dt = 0; }
            }
            let (dt, ds) = clamp_group_move(g.ids.iter().filter_map(|id| track.note(id)), dt, ds);
            g.preview = Preview::Move { delta_tick: dt, delta_semitones: ds, duplicate: matches!(g.kind, GestureKind::Move { duplicate: true }) };
        }
        Op::UpdateResize => {
            let GestureKind::Resize { edge } = g.kind else { return; };
            let Some(anchor) = g.anchor.as_ref().and_then(|id| track.note(id)) else { return; };
            let edge_tick = match edge { Edge::Start => anchor.start_tick, Edge::End => anchor.end_tick() };
            g.preview = Preview::Resize { edge, delta_tick: snap.snap(edge_tick as f64 + music.tick - g.origin.tick) - edge_tick };
        }
        Op::UpdateMarquee => {
            let (t0, t1) = (g.origin.tick.min(music.tick), g.origin.tick.max(music.tick));
            let (p0, p1) = (g.origin.pitch.min(music.pitch), g.origin.pitch.max(music.pitch));
            let hits = track.notes_in_range(t0.floor() as i64, t1.ceil() as i64 + 1)
                .filter(|n| (n.start_tick as f64) < t1 && (n.end_tick() as f64) > t0 && (n.midi_note as f64) < p1 && (n.midi_note as f64 + 1.0) > p0)
                .map(|n| n.id.clone()).collect();
            g.preview = Preview::RubberBand { from: g.origin, to: *music, hits };
        }
        _ => unreachable!(),
    }
}

fn commit(op: Op, session: &EditorSession, out: &mut Vec<Effect>) {
    let Some(g) = &session.gesture else { return; };
    let (label, command, select_after) = match (op, &g.preview) {
        (Op::CommitMove, Preview::Move { delta_tick, delta_semitones, .. }) => (
            format!("移动 {} 个音符", g.ids.len()),
            Command::MoveNotes { track: g.track.clone(), note_ids: g.ids.clone(), delta_tick: *delta_tick, delta_semitones: *delta_semitones }, None),
        (Op::CommitDuplicate, Preview::Move { delta_tick, delta_semitones, .. }) => {
            // Cancellation never allocates a persistent identity (T03).
            let new_ids: Vec<NoteId> = g.ids.iter().map(|_| NoteId::generate()).collect();
            (format!("复制 {} 个音符", g.ids.len()), Command::DuplicateNotes { track: g.track.clone(), source_ids: g.ids.clone(), new_ids: new_ids.clone(), delta_tick: *delta_tick, delta_semitones: *delta_semitones }, Some(new_ids))
        }
        (Op::CommitResize, Preview::Resize { edge, delta_tick }) => (
            if *edge == Edge::End { "调整音符尾部" } else { "调整音符头部" }.into(),
            Command::ResizeNotes { track: g.track.clone(), note_ids: g.ids.clone(), edge: *edge, delta_tick: *delta_tick }, None),
        _ => return,
    };
    out.push(Effect::Commit { base: g.base_revision, batch: CommandBatch::single(label, command), select_after });
}
