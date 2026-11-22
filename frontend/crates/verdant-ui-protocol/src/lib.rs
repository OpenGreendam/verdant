//! Stable, typed boundary to the compiler-owned interaction planners.
//!
//! Generated package fingerprints, owner handles, JSON and string operation
//! names stop here. No transition rules are reimplemented in Rust. The pinned
//! upstream target still links its GPUI support crate: this adapter isolates
//! that dependency but does not claim a GPUI-free build or data-only backend.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use verdant_control_gesture_planner::ControlGesturePlannerHandle;
use verdant_note_gesture_planner::NoteGesturePlannerHandle;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GesturePhase { Idle, Armed, Dragging }

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

#[derive(Clone, Copy, Debug)]
pub enum NoteMachine { Move, Duplicate, Resize, Marquee }

impl NoteMachine {
    fn as_str(self) -> &'static str {
        match self { Self::Move => "move", Self::Duplicate => "duplicate", Self::Resize => "resize", Self::Marquee => "marquee" }
    }
}

#[derive(Default, Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum NoteEffect {
    #[serde(rename = "Session.SelectOnlyHitNote")] SelectOnly,
    #[serde(rename = "Session.ToggleHitNote")] Toggle,
    #[serde(rename = "Session.ExtendSelectionToHit")] Extend,
    #[serde(rename = "Session.SelectIfUnselectedHit")] SelectIfUnselected,
    #[serde(rename = "Session.BeginMoveGesture")] BeginMove,
    #[serde(rename = "Session.BeginDuplicateGesture")] BeginDuplicate,
    #[serde(rename = "Session.BeginResizeGesture")] BeginResize,
    #[serde(rename = "Session.BeginMarqueeGesture")] BeginMarquee,
    #[serde(rename = "Session.UpdateMovePreview")] UpdateMove,
    #[serde(rename = "Session.UpdateResizePreview")] UpdateResize,
    #[serde(rename = "Session.UpdateMarqueePreview")] UpdateMarquee,
    #[serde(rename = "Domain.CommitMoveNotes")] CommitMove,
    #[serde(rename = "Domain.CommitDuplicateNotes")] CommitDuplicate,
    #[serde(rename = "Domain.CommitResizeNotes")] CommitResize,
    #[serde(rename = "Session.CommitMarqueeSelection")] CommitMarquee,
    #[serde(rename = "Session.ClickReleaseSelection")] ClickSelection,
    #[serde(rename = "Session.ReportConflictIfStale")] Conflict,
    #[serde(rename = "Session.ClearGesturePreview")] Clear,
    #[serde(rename = "Host.CapturePointer")] Capture,
    #[serde(rename = "Host.ReleasePointer")] Release,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotePlan {
    #[serde(rename = "nextPhase")]
    pub next_phase: GesturePhase,
    pub consumed: bool,
    pub effects: Vec<NoteEffect>,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ControlGuards {
    pub hit_control: bool,
    pub threshold_crossed: bool,
    pub reset_modifier: bool,
    pub double_click: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum ControlEffect {
    #[serde(rename = "Control.Select")] Select,
    #[serde(rename = "Control.Reset")] Reset,
    #[serde(rename = "Control.Begin")] Begin,
    #[serde(rename = "Control.Preview")] Preview,
    #[serde(rename = "Control.Commit")] Commit,
    #[serde(rename = "Control.Discard")] Discard,
    #[serde(rename = "Host.CapturePointer")] Capture,
    #[serde(rename = "Host.ReleasePointer")] Release,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlan {
    #[serde(rename = "nextPhase")]
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

fn decode<T: serde::de::DeserializeOwned>(method: &str, value: Value) -> Result<T, PlannerError> {
    serde_json::from_value(value).map_err(|error| PlannerError(format!("{method} returned an incompatible decision: {error}")))
}

fn validate_result(phase: GesturePhase, next: GesturePhase, consumed: bool, empty: bool) -> Result<(), PlannerError> {
    if (!consumed && (!empty || phase != next)) || (empty && phase != next) {
        return Err(PlannerError("compiled planner changed state without an accepted effect".into()));
    }
    Ok(())
}

pub fn plan_note(machine: NoteMachine, phase: GesturePhase, event: GestureEvent, guards: &NoteGuards) -> Result<NotePlan, PlannerError> {
    let value = NoteGesturePlannerHandle::call_data_method("planNoteGesture", vec![
        machine.as_str().into(), phase.as_str().into(), event.as_str().into(),
        serde_json::to_value(guards).expect("serializable note guards"),
    ]).map_err(|error| PlannerError(format!("compiled note planner failed: {error:?}")))?;
    let plan: NotePlan = decode("planNoteGesture", value)?;
    validate_result(phase, plan.next_phase, plan.consumed, plan.effects.is_empty())?;
    Ok(plan)
}

pub fn plan_control(phase: GesturePhase, event: GestureEvent, guards: ControlGuards) -> Result<ControlPlan, PlannerError> {
    let value = ControlGesturePlannerHandle::call_data_method("planControlGesture", vec![
        phase.as_str().into(), event.as_str().into(), serde_json::to_value(guards).expect("serializable control guards"),
    ]).map_err(|error| PlannerError(format!("compiled control planner failed: {error:?}")))?;
    decode_control(phase, event, value)
}

fn decode_control(phase: GesturePhase, event: GestureEvent, value: Value) -> Result<ControlPlan, PlannerError> {
    let plan: ControlPlan = decode("planControlGesture", value)?;
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
        if !valid { return Err(PlannerError(format!("compiled control planner emitted {effect:?} during {event:?}"))); }
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
    if phase != GesturePhase::Idle && matches!(event, GestureEvent::Up | GestureEvent::Cancel)
        && (!plan.consumed || plan.next_phase != GesturePhase::Idle || plan.effects.last() != Some(&ControlEffect::Release)) {
        return Err(PlannerError("compiled terminal decision failed to retire the active control and its capture".into()));
    }
    Ok(plan)
}

pub fn drag_normalized(start: f64, delta: f64, travel: f64, fine: bool) -> Result<f64, PlannerError> {
    if !start.is_finite() || !delta.is_finite() || !travel.is_finite() {
        return Err(PlannerError("dragNormalized requires finite input coordinates".into()));
    }
    let value = ControlGesturePlannerHandle::call_data_method("dragNormalized", vec![start.into(), delta.into(), travel.into(), fine.into()])
        .map_err(|error| PlannerError(format!("compiled drag mapping failed: {error:?}")))?;
    value.as_f64().filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .ok_or_else(|| PlannerError("dragNormalized returned a non-finite or out-of-range value".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_wire_protocol_rejects_unknown_or_partial_results() {
        for value in [
            serde_json::json!({"nextPhase":"typo","consumed":true,"effects":[]}),
            serde_json::json!({"nextPhase":"idle","effects":[]}),
            serde_json::json!({"nextPhase":"idle","consumed":true,"effects":["Control.Commit","Control.Unknown"]}),
            serde_json::json!({"nextPhase":"idle","consumed":true,"effects":[],"ignored":1}),
        ] {
            assert!(decode::<ControlPlan>("test", value).is_err());
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
                serde_json::json!({"nextPhase":"armed","consumed":true,"effects":effects})).is_err());
        }
        assert!(decode_control(GesturePhase::Dragging, GestureEvent::Up,
            serde_json::json!({"nextPhase":"idle","consumed":true,"effects":["Control.Commit"]})).is_err());
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
}
