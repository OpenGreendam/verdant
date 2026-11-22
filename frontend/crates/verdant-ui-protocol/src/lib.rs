//! Stable, typed boundary to the compiler-owned interaction planners.
//!
//! The pinned compiler emits direct Rust functions, typed records and arrays
//! into two stable, std-only data crates. This boundary adds application enums
//! and effect-capability validation without reimplementing transition rules.
//! It has no GPUI, native owner, renderer or JSON dependency.

use std::fmt;
use verdant_control_gesture_data as control;
use verdant_note_gesture_data as note;

// These are protocol names, not transition rules. Keep their conversion total:
// an unknown generated operation must fail before any host effect is executed.
macro_rules! protocol_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl TryFrom<&str> for $name {
            type Error = PlannerError;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value {
                    $($wire => Ok(Self::$variant),)+
                    _ => Err(PlannerError(format!("compiled planner returned unknown {} {value:?}", stringify!($name)))),
                }
            }
        }
    };
}

protocol_enum!(GesturePhase { Idle => "idle", Armed => "armed", Dragging => "dragging" });

impl GesturePhase {
    fn as_str(self) -> &'static str {
        match self { Self::Idle => "idle", Self::Armed => "armed", Self::Dragging => "dragging" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureEvent { Down, Move, Up, Cancel }

impl GestureEvent {
    fn as_str(self) -> &'static str {
        match self { Self::Down => "PointerDown", Self::Move => "PointerMove", Self::Up => "PointerUp", Self::Cancel => "Cancel" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteMachine { Move, Duplicate, Resize, Marquee }

impl NoteMachine {
    fn as_str(self) -> &'static str {
        match self { Self::Move => "move", Self::Duplicate => "duplicate", Self::Resize => "resize", Self::Marquee => "marquee" }
    }
}

#[derive(Default)]
pub struct NoteGuards {
    pub hit_background: bool,
    pub hit_selected_note: bool,
    pub hit_unselected_note: bool,
    pub hit_note_body: bool,
    pub hit_resize_handle: bool,
    pub toggle_modifier: bool,
    pub extend_modifier: bool,
    pub duplicate_modifier: bool,
    pub pointer_threshold_crossed: bool,
    pub base_revision_matches: bool,
    pub base_revision_matches_and_delta_nonzero: bool,
}

protocol_enum!(NoteEffect {
    SelectOnly => "Session.SelectOnlyHitNote",
    Toggle => "Session.ToggleHitNote",
    Extend => "Session.ExtendSelectionToHit",
    SelectIfUnselected => "Session.SelectIfUnselectedHit",
    BeginMove => "Session.BeginMoveGesture",
    BeginDuplicate => "Session.BeginDuplicateGesture",
    BeginResize => "Session.BeginResizeGesture",
    BeginMarquee => "Session.BeginMarqueeGesture",
    UpdateMove => "Session.UpdateMovePreview",
    UpdateResize => "Session.UpdateResizePreview",
    UpdateMarquee => "Session.UpdateMarqueePreview",
    CommitMove => "Domain.CommitMoveNotes",
    CommitDuplicate => "Domain.CommitDuplicateNotes",
    CommitResize => "Domain.CommitResizeNotes",
    CommitMarquee => "Session.CommitMarqueeSelection",
    ClickSelection => "Session.ClickReleaseSelection",
    Conflict => "Session.ReportConflictIfStale",
    Clear => "Session.ClearGesturePreview",
    Capture => "Host.CapturePointer",
    Release => "Host.ReleasePointer",
});

#[derive(Debug)]
pub struct NotePlan {
    pub next_phase: GesturePhase,
    pub consumed: bool,
    pub effects: Vec<NoteEffect>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ControlGuards {
    pub hit_control: bool,
    pub threshold_crossed: bool,
    pub reset_modifier: bool,
    pub double_click: bool,
}

protocol_enum!(ControlEffect {
    Select => "Control.Select",
    Reset => "Control.Reset",
    Begin => "Control.Begin",
    Preview => "Control.Preview",
    Commit => "Control.Commit",
    Discard => "Control.Discard",
    Capture => "Host.CapturePointer",
    Release => "Host.ReleasePointer",
});

#[derive(Debug)]
pub struct ControlPlan {
    pub next_phase: GesturePhase,
    pub consumed: bool,
    pub effects: Vec<ControlEffect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerError(String);

impl fmt::Display for PlannerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}
impl std::error::Error for PlannerError {}

fn validate_result(phase: GesturePhase, next: GesturePhase, consumed: bool, empty: bool) -> Result<(), PlannerError> {
    if (!consumed && (!empty || phase != next)) || (empty && phase != next) {
        return Err(PlannerError("compiled planner changed state without an accepted effect".into()));
    }
    Ok(())
}

fn validate_ownership(phase: GesturePhase, next: GesturePhase, begin: bool, release: bool) -> Result<(), PlannerError> {
    // A phase is a promise that its host gesture and capture exist. Validate
    // resource acquisition/retirement, without deciding guards or transitions.
    let acquired = phase == GesturePhase::Idle && next != GesturePhase::Idle;
    let retired = phase != GesturePhase::Idle && next == GesturePhase::Idle;
    if begin != acquired || release != retired {
        return Err(PlannerError("compiled phase changed without matching gesture ownership effects".into()));
    }
    Ok(())
}

pub fn plan_note(machine: NoteMachine, phase: GesturePhase, event: GestureEvent, guards: &NoteGuards) -> Result<NotePlan, PlannerError> {
    let value = note::plan_note_gesture(
        machine.as_str().into(), phase.as_str().into(), event.as_str().into(),
        note::PlanNoteGestureGuards::new(
            guards.hit_background, guards.hit_selected_note, guards.hit_unselected_note,
            guards.hit_note_body, guards.hit_resize_handle, guards.toggle_modifier,
            guards.extend_modifier, guards.duplicate_modifier, guards.pointer_threshold_crossed,
            guards.base_revision_matches, guards.base_revision_matches_and_delta_nonzero,
        ),
    ).map_err(|error| PlannerError(format!("compiled note planner failed: {error}")))?;
    decode_note(machine, phase, event, &value)
}

fn decode_note(machine: NoteMachine, phase: GesturePhase, event: GestureEvent, value: &note::PlanNoteGestureResult) -> Result<NotePlan, PlannerError> {
    let plan = NotePlan {
        next_phase: value.next_phase.as_str().try_into()?,
        consumed: value.consumed,
        effects: value.effects.iter().map(|effect| NoteEffect::try_from(effect.as_str())).collect::<Result<_, _>>()?,
    };
    validate_result(phase, plan.next_phase, plan.consumed, plan.effects.is_empty())?;
    for (index, effect) in plan.effects.iter().enumerate() {
        let valid = match effect {
            NoteEffect::SelectOnly | NoteEffect::Toggle | NoteEffect::Extend | NoteEffect::SelectIfUnselected | NoteEffect::Capture => event == GestureEvent::Down,
            NoteEffect::BeginMove => event == GestureEvent::Down && machine == NoteMachine::Move,
            NoteEffect::BeginDuplicate => event == GestureEvent::Down && machine == NoteMachine::Duplicate,
            NoteEffect::BeginResize => event == GestureEvent::Down && machine == NoteMachine::Resize,
            NoteEffect::BeginMarquee => event == GestureEvent::Down && machine == NoteMachine::Marquee,
            NoteEffect::UpdateMove => event == GestureEvent::Move && matches!(machine, NoteMachine::Move | NoteMachine::Duplicate),
            NoteEffect::UpdateResize => event == GestureEvent::Move && machine == NoteMachine::Resize,
            NoteEffect::UpdateMarquee => event == GestureEvent::Move && machine == NoteMachine::Marquee,
            NoteEffect::CommitMove => event == GestureEvent::Up && machine == NoteMachine::Move,
            NoteEffect::CommitDuplicate => event == GestureEvent::Up && machine == NoteMachine::Duplicate,
            NoteEffect::CommitResize => event == GestureEvent::Up && machine == NoteMachine::Resize,
            NoteEffect::CommitMarquee => event == GestureEvent::Up && machine == NoteMachine::Marquee,
            NoteEffect::ClickSelection | NoteEffect::Conflict => event == GestureEvent::Up,
            NoteEffect::Clear | NoteEffect::Release => matches!(event, GestureEvent::Up | GestureEvent::Cancel),
        };
        let needs_owner = matches!(effect, NoteEffect::UpdateMove | NoteEffect::UpdateResize | NoteEffect::UpdateMarquee
            | NoteEffect::CommitMove | NoteEffect::CommitDuplicate | NoteEffect::CommitResize | NoteEffect::CommitMarquee
            | NoteEffect::ClickSelection | NoteEffect::Conflict | NoteEffect::Clear | NoteEffect::Release);
        if !valid || (needs_owner && phase == GesturePhase::Idle) || plan.effects[..index].contains(effect) {
            return Err(PlannerError(format!("compiled note planner emitted invalid or repeated {effect:?} during {machine:?}/{event:?}")));
        }
    }
    let begin = plan.effects.iter().position(|effect| matches!(effect, NoteEffect::BeginMove | NoteEffect::BeginDuplicate | NoteEffect::BeginResize | NoteEffect::BeginMarquee));
    let capture = plan.effects.iter().position(|effect| *effect == NoteEffect::Capture);
    if begin.is_some() != capture.is_some() || matches!((begin, capture), (Some(begin), Some(capture)) if begin >= capture) {
        return Err(PlannerError("compiled note capture must follow its begin effect".into()));
    }
    validate_ownership(phase, plan.next_phase, begin.is_some(), plan.effects.contains(&NoteEffect::Release))?;
    if phase != GesturePhase::Idle && matches!(event, GestureEvent::Up | GestureEvent::Cancel)
        && (!plan.consumed || plan.next_phase != GesturePhase::Idle
            || plan.effects.len() < 2 || plan.effects[plan.effects.len() - 2] != NoteEffect::Clear
            || plan.effects.last() != Some(&NoteEffect::Release)) {
        return Err(PlannerError("compiled terminal decision failed to retire the active note preview and its capture".into()));
    }
    Ok(plan)
}

pub fn plan_control(phase: GesturePhase, event: GestureEvent, guards: ControlGuards) -> Result<ControlPlan, PlannerError> {
    let value = control::plan_control_gesture(
        phase.as_str().into(), event.as_str().into(),
        control::PlanControlGestureGuards::new(guards.hit_control, guards.threshold_crossed, guards.reset_modifier, guards.double_click),
    ).map_err(|error| PlannerError(format!("compiled control planner failed: {error}")))?;
    decode_control(phase, event, &value)
}

fn decode_control(phase: GesturePhase, event: GestureEvent, value: &control::PlanControlGestureResult) -> Result<ControlPlan, PlannerError> {
    let plan = ControlPlan {
        next_phase: value.next_phase.as_str().try_into()?,
        consumed: value.consumed,
        effects: value.effects.iter().map(|effect| ControlEffect::try_from(effect.as_str())).collect::<Result<_, _>>()?,
    };
    validate_result(phase, plan.next_phase, plan.consumed, plan.effects.is_empty())?;
    // Decode and validate the whole response before a host selects, previews,
    // captures or writes a parameter. This is an effect capability boundary,
    // not a second implementation of the authored transition table.
    for effect in &plan.effects {
        let valid = match effect {
            ControlEffect::Select | ControlEffect::Reset | ControlEffect::Begin | ControlEffect::Capture => event == GestureEvent::Down,
            ControlEffect::Preview => event == GestureEvent::Move,
            ControlEffect::Commit => event == GestureEvent::Up,
            ControlEffect::Discard | ControlEffect::Release => matches!(event, GestureEvent::Up | GestureEvent::Cancel),
        };
        let needs_owner = matches!(effect, ControlEffect::Preview | ControlEffect::Commit | ControlEffect::Discard | ControlEffect::Release);
        if !valid || (needs_owner && phase == GesturePhase::Idle) {
            return Err(PlannerError(format!("compiled control planner emitted {effect:?} during {phase:?}/{event:?}")));
        }
    }
    for (index, effect) in plan.effects.iter().enumerate() {
        if plan.effects[..index].contains(effect) {
            return Err(PlannerError(format!("compiled control planner duplicated {effect:?}")));
        }
    }
    let begin = plan.effects.iter().position(|effect| *effect == ControlEffect::Begin);
    let capture = plan.effects.iter().position(|effect| *effect == ControlEffect::Capture);
    if begin.is_some() != capture.is_some() || matches!((begin, capture), (Some(begin), Some(capture)) if begin >= capture) {
        return Err(PlannerError("compiled control capture must follow its begin effect".into()));
    }
    validate_ownership(phase, plan.next_phase, begin.is_some(), plan.effects.contains(&ControlEffect::Release))?;
    if phase != GesturePhase::Idle && matches!(event, GestureEvent::Up | GestureEvent::Cancel)
        && (!plan.consumed || plan.next_phase != GesturePhase::Idle || plan.effects.last() != Some(&ControlEffect::Release)
            || plan.effects.contains(&ControlEffect::Commit) == plan.effects.contains(&ControlEffect::Discard)) {
        return Err(PlannerError("compiled terminal decision failed to retire the active control and its capture".into()));
    }
    Ok(plan)
}

pub fn drag_normalized(start: f64, delta: f64, travel: f64, fine: bool) -> Result<f64, PlannerError> {
    if !start.is_finite() || !delta.is_finite() || !travel.is_finite() {
        return Err(PlannerError("dragNormalized requires finite input coordinates".into()));
    }
    let value = control::drag_normalized(start, delta, travel, fine)
        .map_err(|error| PlannerError(format!("compiled drag mapping failed: {error}")))?;
    if value.is_finite() && (0.0..=1.0).contains(&value) { Ok(value) }
    else { Err(PlannerError("dragNormalized returned a non-finite or out-of-range value".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control_result(phase: &str, consumed: bool, effects: &[&str]) -> std::rc::Rc<control::PlanControlGestureResult> {
        control::PlanControlGestureResult::new(phase.into(), consumed, effects.iter().map(|value| (*value).into()).collect::<Vec<_>>().into())
    }

    fn note_result(phase: &str, effects: &[&str]) -> std::rc::Rc<note::PlanNoteGestureResult> {
        note::PlanNoteGestureResult::new(phase.into(), true, effects.iter().map(|value| (*value).into()).collect::<Vec<_>>().into())
    }

    #[test]
    fn typed_decisions_reject_unknown_protocol_names_before_effects() {
        // Missing/wrongly typed fields cannot cross the generated Rust API.
        // Source strings are still checked before any effect is returned.
        for value in [
            control_result("typo", true, &[]),
            control_result("idle", true, &["Control.Commit", "Control.Unknown"]),
        ] {
            assert!(decode_control(GesturePhase::Dragging, GestureEvent::Up, &value).is_err());
        }
        for value in [
            note_result("typo", &[]),
            note_result("idle", &["Domain.CommitMoveNotes", "Session.Unknown"]),
        ] {
            assert!(decode_note(NoteMachine::Move, GesturePhase::Dragging, GestureEvent::Up, &value).is_err());
        }
    }

    #[test]
    fn unconsumed_results_cannot_change_phase_or_emit_effects() {
        assert!(validate_result(GesturePhase::Idle, GesturePhase::Armed, false, true).is_err());
        assert!(validate_result(GesturePhase::Idle, GesturePhase::Idle, false, false).is_err());
        assert!(validate_result(GesturePhase::Armed, GesturePhase::Armed, true, true).is_ok());
    }

    #[test]
    fn known_effects_are_checked_before_any_host_mutation() {
        for effects in [
            vec!["Control.Commit"],
            vec!["Host.CapturePointer", "Control.Begin"],
            vec!["Control.Begin"],
            vec!["Control.Reset", "Control.Reset"],
        ] {
            assert!(decode_control(GesturePhase::Idle, GestureEvent::Down,
                &control_result("armed", true, &effects)).is_err());
        }
        assert!(decode_control(GesturePhase::Dragging, GestureEvent::Up,
            &control_result("idle", true, &["Control.Commit"])).is_err());
    }

    #[test]
    fn note_effect_capabilities_reject_cross_machine_and_unretired_results() {
        for effects in [
            vec!["Domain.CommitResizeNotes", "Session.ClearGesturePreview", "Host.ReleasePointer"],
            vec!["Domain.CommitMoveNotes"],
            vec!["Host.ReleasePointer", "Session.ClearGesturePreview"],
            vec!["Domain.CommitMoveNotes", "Domain.CommitMoveNotes", "Session.ClearGesturePreview", "Host.ReleasePointer"],
        ] {
            assert!(decode_note(NoteMachine::Move, GesturePhase::Dragging, GestureEvent::Up,
                &note_result("idle", &effects)).is_err());
        }
        assert!(decode_note(NoteMachine::Move, GesturePhase::Idle, GestureEvent::Down,
            &note_result("armed", &["Host.CapturePointer", "Session.BeginMoveGesture"])).is_err());
    }

    #[test]
    fn compiled_control_returns_consumption_and_finite_drag_mapping() {
        let miss = plan_control(GesturePhase::Idle, GestureEvent::Down, ControlGuards::default()).unwrap();
        assert!(!miss.consumed && miss.effects.is_empty());
        let down = plan_control(GesturePhase::Idle, GestureEvent::Down, ControlGuards { hit_control: true, ..Default::default() }).unwrap();
        assert!(down.consumed);
        assert_eq!(down.next_phase, GesturePhase::Armed);
        assert_eq!(down.effects, [ControlEffect::Select, ControlEffect::Begin, ControlEffect::Capture]);
        assert!(drag_normalized(f64::NAN, 0.0, 100.0, false).is_err());
        assert!(drag_normalized(0.5, f64::INFINITY, 100.0, false).is_err());
        assert_eq!(drag_normalized(0.5, 100.0, 100.0, false).unwrap(), 1.0);
        assert!((drag_normalized(0.5, 100.0, 100.0, true).unwrap() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn phase_promises_require_real_host_ownership_before_any_effects_are_returned() {
        // These packets used to pass validation and could mutate selection
        // before the session discovered that the promised gesture was absent.
        for (phase, event, next, effects) in [
            (GesturePhase::Idle, GestureEvent::Down, "armed", vec!["Session.SelectOnlyHitNote"]),
            (GesturePhase::Idle, GestureEvent::Down, "idle", vec!["Session.BeginMoveGesture", "Host.CapturePointer"]),
            (GesturePhase::Dragging, GestureEvent::Move, "idle", vec!["Session.UpdateMovePreview"]),
            (GesturePhase::Idle, GestureEvent::Move, "idle", vec!["Session.UpdateMovePreview"]),
            (GesturePhase::Dragging, GestureEvent::Up, "idle", vec!["Session.ClearGesturePreview", "Domain.CommitMoveNotes", "Host.ReleasePointer"]),
        ] {
            assert!(decode_note(NoteMachine::Move, phase, event, &note_result(next, &effects)).is_err(), "{phase:?}/{event:?}: {effects:?}");
        }
        for (phase, event, next, effects) in [
            (GesturePhase::Idle, GestureEvent::Down, "armed", vec!["Control.Select"]),
            (GesturePhase::Idle, GestureEvent::Down, "idle", vec!["Control.Begin", "Host.CapturePointer"]),
            (GesturePhase::Dragging, GestureEvent::Move, "idle", vec!["Control.Preview"]),
            (GesturePhase::Idle, GestureEvent::Move, "idle", vec!["Control.Preview"]),
            (GesturePhase::Dragging, GestureEvent::Up, "idle", vec!["Host.ReleasePointer"]),
            (GesturePhase::Dragging, GestureEvent::Up, "idle", vec!["Control.Commit", "Control.Discard", "Host.ReleasePointer"]),
        ] {
            assert!(decode_control(phase, event, &control_result(next, true, &effects)).is_err(), "{phase:?}/{event:?}: {effects:?}");
        }
        assert!(decode_note(NoteMachine::Move, GesturePhase::Idle, GestureEvent::Down,
            &note_result("idle", &["Session.ToggleHitNote"])).is_ok());
        assert!(decode_control(GesturePhase::Idle, GestureEvent::Down,
            &control_result("idle", true, &["Control.Select", "Control.Reset"])).is_ok());
    }
}
