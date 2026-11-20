//! Persisted mixer and plugin automation, addressed by document identities.
//!
//! `Project.extensions["automation"]` is absent in old projects. The editor
//! commits a complete replacement through one SetProjectExtension transaction;
//! this module never mutates a Project or its undo history. Values before the
//! first point use the ordinary mixer/plugin setting; values after the last
//! point hold. Interpolation belongs to the outgoing edge of each point.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Project, TrackId};

pub const AUTOMATION_KEY: &str = "automation";
pub const MASTER_INSERTS_KEY: &str = "master_inserts";
pub const PLUGIN_INSTANCE_ID_KEY: &str = "instance_id";
pub const AUTOMATION_VERSION: u32 = 1;
const MAX_TICK: i64 = (1_i64 << 53) - 1;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationOwner {
    Track { track_id: TrackId },
    Master,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationTarget {
    TrackGain { track_id: TrackId },
    TrackPan { track_id: TrackId },
    MasterGain,
    MasterPan,
    /// Parameter IDs are the plugin format's persistent string IDs, not the
    /// current parameter-list index. Instance IDs survive insert reordering.
    PluginParameter { owner: AutomationOwner, instance_id: String, parameter_id: String },
}

impl AutomationTarget {
    pub fn owner(&self) -> AutomationOwner {
        match self {
            Self::TrackGain { track_id } | Self::TrackPan { track_id } => AutomationOwner::Track { track_id: track_id.clone() },
            Self::PluginParameter { owner, .. } => owner.clone(),
            Self::MasterGain | Self::MasterPan => AutomationOwner::Master,
        }
    }

    /// Gain is stored in millidecibels; pan in permille. Native plugin values
    /// are checked against the actual ParamDesc at host admission, not guessed
    /// to be normalized 0..1 here.
    pub fn accepts_value(&self, value: f64) -> bool {
        value.is_finite() && match self {
            Self::TrackGain { .. } | Self::MasterGain => (-96_000.0..=24_000.0).contains(&value),
            Self::TrackPan { .. } | Self::MasterPan => (-1000.0..=1000.0).contains(&value),
            Self::PluginParameter { .. } => true,
        }
    }
}

/// Convex interpolation of finite endpoints without overflowing their
/// difference when a native parameter spans both signs.
pub fn interpolate_value(left: f64, right: f64, fraction: f64) -> f64 {
    if fraction <= 0.0 { return left; }
    if fraction >= 1.0 { return right; }
    if left.is_sign_negative() == right.is_sign_negative() {
        left + (right - left) * fraction
    } else { left * (1.0 - fraction) + right * fraction }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    Step,
    #[default]
    Linear,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationPoint {
    pub tick: i64,
    pub value: f64,
    #[serde(default)]
    pub interpolation: Interpolation,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

impl AutomationPoint {
    pub fn new(tick: i64, value: f64) -> Self {
        Self { tick, value, interpolation: Interpolation::Linear, unknown: BTreeMap::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationLane {
    pub target: AutomationTarget,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub points: Vec<AutomationPoint>,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

fn yes() -> bool { true }
fn version() -> u32 { AUTOMATION_VERSION }

impl AutomationLane {
    pub fn new(target: AutomationTarget) -> Self {
        Self { target, enabled: true, points: Vec::new(), unknown: BTreeMap::new() }
    }

    /// Editing at an existing tick replaces that point. Persistence validation
    /// rejects duplicate/unsorted ticks rather than changing delivered bytes.
    pub fn set_point(&mut self, point: AutomationPoint) -> Result<(), AutomationError> {
        validate_point(&self.target, &point)?;
        match self.points.binary_search_by_key(&point.tick, |p| p.tick) {
            Ok(index) => self.points[index] = point,
            Err(index) => self.points.insert(index, point),
        }
        Ok(())
    }

    pub fn remove_point(&mut self, tick: i64) -> bool {
        if let Ok(index) = self.points.binary_search_by_key(&tick, |p| p.tick) {
            self.points.remove(index);
            true
        } else { false }
    }

    /// Evaluate in musical time for UI readback. Runtime/export compile tempo
    /// boundaries once, then use the same absolute sample-clock curves.
    pub fn value_at(&self, tick: f64, fallback: f64) -> f64 {
        if !self.enabled || !tick.is_finite() { return fallback; }
        let upper = self.points.partition_point(|p| p.tick as f64 <= tick);
        if upper == 0 { return fallback; }
        let left = &self.points[upper - 1];
        let Some(right) = self.points.get(upper) else { return left.value; };
        if left.interpolation == Interpolation::Step { return left.value; }
        let fraction = (tick - left.tick as f64) / (right.tick - left.tick) as f64;
        interpolate_value(left.value, right.value, fraction)
    }

    /// Discrete native parameters accept held steps only. Values themselves
    /// must additionally pass the real ParamTarget::event grid admission.
    pub fn validate_parameter_steps(&self, step_count: i64) -> Result<(), AutomationError> {
        if step_count < 0 || (step_count > 0 && self.points.windows(2).any(|points|
            points[0].interpolation == Interpolation::Linear && points[0].value != points[1].value)) {
            return Err(AutomationError::Invalid("stepped plugin parameters require step interpolation at every changing span".into()));
        }
        Ok(())
    }

    /// Must be called with the real host descriptor before scheduling a native
    /// plugin parameter; an absent plugin keeps authored data but blocks its
    /// audible/export application.
    pub fn validate_parameter_range(&self, minimum: f64, maximum: f64) -> Result<(), AutomationError> {
        if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum
            || self.points.iter().any(|p| !p.value.is_finite() || p.value < minimum || p.value > maximum) {
            return Err(AutomationError::Invalid("plugin automation is outside the admitted parameter range".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationDocument {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub lanes: Vec<AutomationLane>,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

impl Default for AutomationDocument {
    fn default() -> Self { Self { version: AUTOMATION_VERSION, lanes: Vec::new(), unknown: BTreeMap::new() } }
}

#[derive(Debug, thiserror::Error)]
pub enum AutomationError {
    #[error("automation JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid automation: {0}")]
    Invalid(String),
}

impl AutomationDocument {
    pub fn from_project(project: &Project) -> Result<Self, AutomationError> {
        let document = match project.extensions.get(AUTOMATION_KEY) {
            Some(value) => serde_json::from_value(value.clone())?,
            None => Self::default(),
        };
        document.validate(project)?;
        Ok(document)
    }

    /// None means remove the extension, preserving the old-project document
    /// shape when no automation has been authored. Future unknown data is kept.
    pub fn extension_value(&self) -> Result<Option<Value>, AutomationError> {
        if self.lanes.is_empty() && self.unknown.is_empty() && self.version == AUTOMATION_VERSION { return Ok(None); }
        Ok(Some(serde_json::to_value(self)?))
    }

    pub fn lane(&self, target: &AutomationTarget) -> Option<&AutomationLane> {
        self.lanes.iter().find(|lane| &lane.target == target)
    }

    pub fn set_lane(&mut self, lane: AutomationLane) {
        if let Some(existing) = self.lanes.iter_mut().find(|existing| existing.target == lane.target) {
            *existing = lane;
        } else { self.lanes.push(lane); }
    }

    pub fn remove_lane(&mut self, target: &AutomationTarget) -> bool {
        let old = self.lanes.len();
        self.lanes.retain(|lane| &lane.target != target);
        self.lanes.len() != old
    }

    /// Returns the extension that belongs in the same batch as RemoveTrack.
    /// The reducer therefore removes dependent lanes in one undoable intent.
    pub fn remove_track(&mut self, track_id: &TrackId) {
        self.lanes.retain(|lane| match &lane.target {
            AutomationTarget::TrackGain { track_id: id } | AutomationTarget::TrackPan { track_id: id } => id != track_id,
            AutomationTarget::PluginParameter { owner: AutomationOwner::Track { track_id: id }, .. } => id != track_id,
            _ => true,
        });
    }

    pub fn remove_plugin(&mut self, owner: &AutomationOwner, instance_id: &str) {
        self.lanes.retain(|lane| !matches!(&lane.target, AutomationTarget::PluginParameter { owner: o, instance_id: id, .. }
            if o == owner && id == instance_id));
    }

    pub fn validate(&self, project: &Project) -> Result<(), AutomationError> {
        if self.version != AUTOMATION_VERSION {
            return Err(AutomationError::Invalid(format!("unsupported schema version {}", self.version)));
        }
        let mut targets = BTreeSet::new();
        for lane in &self.lanes {
            if !targets.insert(&lane.target) {
                return Err(AutomationError::Invalid("more than one lane addresses the same parameter".into()));
            }
            validate_target(project, &lane.target)?;
            let mut previous = None;
            for point in &lane.points {
                validate_point(&lane.target, point)?;
                if previous.is_some_and(|tick| point.tick <= tick) {
                    return Err(AutomationError::Invalid("curve ticks must strictly increase".into()));
                }
                previous = Some(point.tick);
            }
        }
        Ok(())
    }
}

fn validate_point(target: &AutomationTarget, point: &AutomationPoint) -> Result<(), AutomationError> {
    if !(0..=MAX_TICK).contains(&point.tick) || !target.accepts_value(point.value) {
        return Err(AutomationError::Invalid(format!("invalid point at tick {}", point.tick)));
    }
    Ok(())
}

fn validate_target(project: &Project, target: &AutomationTarget) -> Result<(), AutomationError> {
    let owner = target.owner();
    let inserts = match &owner {
        AutomationOwner::Track { track_id } => {
            let track = project.track(track_id).ok_or_else(|| AutomationError::Invalid(format!("missing track {track_id}")))?;
            track.unknown.get("inserts")
        }
        AutomationOwner::Master => project.extensions.get(MASTER_INSERTS_KEY),
    };
    if let AutomationTarget::PluginParameter { instance_id, parameter_id, .. } = target {
        if instance_id.trim().is_empty() || parameter_id.trim().is_empty() {
            return Err(AutomationError::Invalid("plugin instance/parameter ID must not be empty".into()));
        }
        let count = inserts.and_then(Value::as_array).map_or(0, |inserts| inserts.iter()
            .filter(|insert| insert.get(PLUGIN_INSTANCE_ID_KEY).and_then(Value::as_str) == Some(instance_id.as_str())).count());
        if count != 1 {
            return Err(AutomationError::Invalid(format!("plugin instance {instance_id} must resolve to exactly one persisted insert")));
        }
    }
    Ok(())
}

/// Stable instance key accessor shared with the editor's insert migration.
pub fn plugin_instance_id(insert: &Map<String, Value>) -> Option<&str> {
    insert.get(PLUGIN_INSTANCE_ID_KEY).and_then(Value::as_str).filter(|id| !id.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_automation_preserves_old_project_bytes() {
        let project = Project::new_empty();
        let before = serde_json::to_value(&project).unwrap();
        let document = AutomationDocument::from_project(&project).unwrap();
        assert_eq!(document.extension_value().unwrap(), None);
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }

    #[test]
    fn interpolation_and_replacing_a_tick_preserve_other_points() {
        let mut lane = AutomationLane::new(AutomationTarget::MasterPan);
        lane.set_point(AutomationPoint::new(960, -1000.0)).unwrap();
        lane.set_point(AutomationPoint::new(1920, 1000.0)).unwrap();
        assert_eq!(lane.value_at(0.0, 123.0), 123.0);
        assert_eq!(lane.value_at(1440.0, 0.0), 0.0);
        assert_eq!(lane.value_at(3000.0, 0.0), 1000.0);
        let mut replacement = AutomationPoint::new(960, -500.0);
        replacement.interpolation = Interpolation::Step;
        lane.set_point(replacement).unwrap();
        assert_eq!(lane.points.len(), 2);
        assert_eq!(lane.value_at(1919.0, 0.0), -500.0);
        assert_eq!(lane.value_at(1920.0, 0.0), 1000.0);
    }

    #[test]
    fn track_and_plugin_identity_survive_reordering() {
        let mut project = Project::new_empty();
        let id = project.tracks[0].id.clone();
        project.tracks[0].unknown.insert("inserts".into(), serde_json::json!([
            {"instance_id":"persistent-compressor","format":"clap","id":"plugin.one"},
            {"instance_id":"persistent-eq","format":"clap","id":"plugin.two"}
        ]));
        let target = AutomationTarget::PluginParameter { owner: AutomationOwner::Track { track_id: id.clone() },
            instance_id: "persistent-compressor".into(), parameter_id: "threshold".into() };
        let mut lane = AutomationLane::new(target);
        lane.set_point(AutomationPoint::new(0, -12.0)).unwrap();
        let mut document = AutomationDocument::default();
        document.set_lane(lane);
        document.validate(&project).unwrap();
        project.tracks[0].unknown.get_mut("inserts").unwrap().as_array_mut().unwrap().reverse();
        project.tracks.push(crate::Track::new("other", "blue"));
        project.tracks.reverse();
        document.validate(&project).unwrap();
        document.remove_track(&id);
        assert!(document.lanes.is_empty());
    }

    #[test]
    fn invalid_curves_and_dangling_targets_fail_before_commit() {
        let project = Project::new_empty();
        let mut lane = AutomationLane::new(AutomationTarget::MasterGain);
        assert!(lane.set_point(AutomationPoint::new(0, f64::NAN)).is_err());
        assert!(lane.set_point(AutomationPoint::new(-1, 0.0)).is_err());
        lane.points = vec![AutomationPoint::new(1, 0.0), AutomationPoint::new(1, 1.0)];
        let mut document = AutomationDocument::default();
        document.set_lane(lane);
        assert!(document.validate(&project).is_err());
        document.lanes[0] = AutomationLane::new(AutomationTarget::TrackGain { track_id: "gone".into() });
        assert!(document.validate(&project).is_err());
    }

    #[test]
    fn finite_native_parameter_endpoints_interpolate_without_overflow() {
        assert_eq!(interpolate_value(-1.0e308, 1.0e308, 0.5), 0.0);
        assert_eq!(interpolate_value(-1.0e308, 1.0e308, 0.0), -1.0e308);
        assert_eq!(interpolate_value(-1.0e308, 1.0e308, 1.0), 1.0e308);
    }

    #[test]
    fn unknown_extension_fields_round_trip() {
        let mut project = Project::new_empty();
        let value = serde_json::json!({"version":1,"future":{"keep":true},"lanes":[
            {"target":{"kind":"master_gain"},"future_lane":7,"points":[{"tick":0,"value":-6000.0,"future_point":"x"}]}
        ]});
        project.extensions.insert(AUTOMATION_KEY.into(), value);
        let document = AutomationDocument::from_project(&project).unwrap();
        let encoded = document.extension_value().unwrap().unwrap();
        assert_eq!(encoded["future"], serde_json::json!({"keep":true}));
        assert_eq!(encoded["lanes"][0]["future_lane"], 7);
        assert_eq!(encoded["lanes"][0]["points"][0]["future_point"], "x");
    }
}
