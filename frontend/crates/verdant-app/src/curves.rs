//! Curve pointer ownership, previews and semantic point actions.
//!
//! Expression/DYN region controls operate on selected notes in one transaction.
//! Bypass keeps point IDs and geometry in their ordinary curves, replaces only
//! values with the engine's exact neutral value, and retains authored values in
//! point extensions. Save, undo and note stretching keep that data.
use super::*;
use verdant_domain::PointId;
use verdant_domain::model::{PitchPoint, GainPoint};
use verdant_session::{CancelReason, InputEvent, PointerId};

const DISABLED_LANES: &str = "verdant.disabled_lanes";
const AUTHORED_VALUE: &str = "verdant.authored_lane_value";

pub(crate) fn region_parameter(param: Param) -> bool {
    param == Param::Dyn || (param.is_expression() && param != Param::Portamento)
}

#[cfg(test)]
#[path = "expression_lane_tests.rs"]
mod expression_lane_tests;

pub(crate) fn lane_disabled(note: &Note, param: Param) -> bool {
    note.curves.unknown.get(DISABLED_LANES).and_then(|v| v.get(param.short()))
        .and_then(serde_json::Value::as_bool) == Some(true)
}

fn delete_region_points(note: &mut Note, param: Param, selected: &std::collections::BTreeSet<(NoteId, PointId)>) {
    let note_id = note.id.clone();
    let keep = |id: &PointId| !selected.is_empty() && !selected.contains(&(note_id.clone(), id.clone()));
    if param == Param::Dyn { note.curves.gain_millidecibel_points.retain(|point| keep(&point.id)); }
    else if let Some(points) = note.curves.expression_points.get_mut(param.short()) { points.retain(|point| keep(&point.id)); }
}

/// Neutral values still travel through the real DYN/expression renderer. No
/// synthetic tension control or model-dynamics setting is introduced here.
fn set_region_lane_enabled(note: &mut Note, param: Param, enabled: bool) -> Result<(), String> {
    if !region_parameter(param) || lane_disabled(note, param) == !enabled { return Ok(()); }
    if let Some(value) = note.curves.unknown.get(DISABLED_LANES) {
        if !value.is_object() { return Err("区域曲线开关数据无效".into()); }
    }
    // Validate reserved metadata before changing any point. The caller commits
    // the complete cloned note set only when every region succeeded.
    let check = |unknown: &BTreeMap<String, serde_json::Value>, gain: bool| -> Result<(), String> {
        if let Some(value) = unknown.get(AUTHORED_VALUE) {
            if !enabled { return Err("控制点包含冲突的区域曲线数据".into()); }
            let valid = if gain { value.as_i64().is_some_and(|v| (-24_000..=6000).contains(&v)) }
                else { value.as_u64().is_some_and(|v| v <= 127) };
            if !valid { return Err("控制点保存的曲线值无效".into()); }
        }
        Ok(())
    };
    if param == Param::Dyn {
        for point in &note.curves.gain_millidecibel_points { check(&point.unknown, true)?; }
        for point in &mut note.curves.gain_millidecibel_points {
            if enabled {
                if let Some(value) = point.unknown.remove(AUTHORED_VALUE) { point.millidecibels = value.as_i64().unwrap(); }
            } else {
                point.unknown.insert(AUTHORED_VALUE.into(), point.millidecibels.into());
                point.millidecibels = 0;
            }
        }
    } else if let Some(points) = note.curves.expression_points.get_mut(param.short()) {
        for point in points.iter() { check(&point.unknown, false)?; }
        for point in points {
            if enabled {
                if let Some(value) = point.unknown.remove(AUTHORED_VALUE) { point.value = value.as_u64().unwrap() as u8; }
            } else {
                point.unknown.insert(AUTHORED_VALUE.into(), point.value.into());
                point.value = param.default_value() as u8;
            }
        }
    }
    if enabled {
        if let Some(map) = note.curves.unknown.get_mut(DISABLED_LANES).and_then(serde_json::Value::as_object_mut) {
            map.remove(param.short());
            if map.is_empty() { note.curves.unknown.remove(DISABLED_LANES); }
        }
    } else {
        note.curves.unknown.entry(DISABLED_LANES).or_insert_with(|| serde_json::json!({}))
            .as_object_mut().unwrap().insert(param.short().into(), true.into());
    }
    Ok(())
}

impl Editor {
    pub(crate) fn region_lane_disabled(&self, note: &Note, param: Param) -> bool {
        lane_disabled(note, param)
    }

    /// A stroke crossing a bypassed region must not overwrite retained values.
    pub(crate) fn region_lane_editable(&self, param: Param, from: f64, to: f64) -> bool {
        !region_parameter(param) || !self.active_track().is_some_and(|track|
            self.index.notes_in_range(track, from.min(to).floor() as i64,
                (from.max(to).ceil() as i64).saturating_add(1)).any(|note| lane_disabled(note, param)
                && (note.start_tick as f64) <= from.max(to) && (note.end_tick() as f64) > from.min(to)))
    }

    pub(crate) fn toggle_region_lane(&mut self, param: Param, cx: &mut Context<Self>) {
        let mut notes = self.selected_notes();
        if notes.is_empty() { self.notice("先选择要启用或旁路曲线的音符区域"); cx.notify(); return; }
        let enabled = notes.iter().any(|note| lane_disabled(note, param));
        if enabled && !verdant_backend::params::support(param, self.route(), self.scheduler.capabilities.as_ref()).is_effective() {
            self.notice(format!("当前后端不能执行 {} 参数", param.short())); cx.notify(); return;
        }
        for note in &mut notes {
            if let Err(error) = set_region_lane_enabled(note, param, enabled) { self.notice(error); cx.notify(); return; }
        }
        self.commit(self.revision(), CommandBatch::single(if enabled { "启用区域曲线" } else { "旁路区域曲线" },
            Command::ReplaceNotes { track: self.session.track.clone(), notes }), cx);
        self.point_selection.clear();
        cx.notify();
    }

    pub(crate) fn interpolate_region_lane(&mut self, param: Param, mode: verdant_domain::editing::Interpolation, cx: &mut Context<Self>) {
        let mut notes = self.selected_notes();
        if notes.is_empty() { self.notice("先选择要设置插值的音符区域"); cx.notify(); return; }
        for note in &mut notes { note.curves.set_interpolation(param.short(), mode); }
        self.commit(self.revision(), CommandBatch::single("区域曲线插值",
            Command::ReplaceNotes { track: self.session.track.clone(), notes }), cx);
    }

    pub(crate) fn delete_region_lane_points(&mut self, param: Param, cx: &mut Context<Self>) {
        let mut notes = self.selected_notes();
        let selected = self.point_selection.clone();
        if notes.is_empty() { self.notice("先选择要删除控制点的音符区域"); cx.notify(); return; }
        for note in &mut notes { delete_region_points(note, param, &selected); }
        self.commit(self.revision(), CommandBatch::single(if selected.is_empty() { "清除区域曲线" } else { "删除曲线控制点" },
            Command::ReplaceNotes { track: self.session.track.clone(), notes }), cx);
        self.point_selection.clear();
        cx.notify();
    }

    pub fn cancel_edits(&mut self, reason: CancelReason, window: &mut Window, cx: &mut Context<Self>) {
        // The remaining native-only lane/mixer controls predate the owned
        // planner capture tokens. Limit their cleanup to an active editor
        // interaction and its currently captured hitbox.
        let legacy_capture = (self.arrange_drag.is_some() || self.mixer_drag.is_some()
            || self.master_drag.is_some() || self.numeric_preview.is_some()
            || self.lane_stroke.as_ref().is_some_and(|stroke| !stroke.roll)
            || crate::ui_controls::has_numeric_scrub(cx.entity_id(), cx)
            || crate::roll::has_extra_drag(cx.entity_id(), cx))
            .then(|| window.captured_hitbox()).flatten();
        crate::ruler_navigation::cancel(cx.entity_id(), window, cx);
        let project = self.store().snapshot();
        let effects = self.session.handle_pointer(&project, self.revision(), PointerId::Mouse, InputEvent::Cancel(reason));
        self.lane_stroke = None;
        self.compiled_pitch_move.cancel(window, cx);
        self.note_creation = None;
        self.arrange_drag = None;
        self.mixer_drag = None;
        if matches!(reason, CancelReason::ProjectChanged) { self.cancel_control_drags(cx); }
        else { self.cancel_control_drags_in(window.window_handle(), cx); }
        self.sync_mixer();
        self.release_roll_capture(window, cx);
        if legacy_capture.is_some() && window.captured_hitbox() == legacy_capture { window.release_pointer(); }
        self.apply_effects(effects, window, cx);
    }

    pub fn begin_curve(&mut self, param: Param, pointer: PointerId, note: Option<NoteId>, point: Option<PointId>, roll: bool, tick: f64, value: f64, screen: (f64, f64), erase: bool) -> bool {
        if self.session.pointer_owner().is_some() || self.lane_stroke.is_some() || self.mixer_drag.is_some()
            || self.device_drag.is_some() || self.note_creation.is_some() || self.has_proto_pointer_contact() { return false; }
        self.lane_stroke = Some(LaneStroke {
            param, pointer, note, point, roll, base: self.revision(), tool: if roll { LaneTool::Pencil } else { self.lane_tool },
            origin_screen: screen, axis: None, samples: vec![(tick, value)], erase,
        });
        true
    }

    /// Roll samples use absolute viewport pitch (MIDI + 0.5 at a row's centre).
    /// Pointer moves only extend UI state; the document changes once on release.
    pub fn roll_pitch_sample(&mut self, pointer: PointerId, mut tick: f64, mut pitch: f64, screen: (f64, f64), shift: bool) {
        if !tick.is_finite() || !pitch.is_finite() { return; }
        let vp = self.session.viewport;
        let Some(stroke) = self.lane_stroke.as_mut().filter(|s| s.pointer == pointer && s.roll) else { return };
        let origin = stroke.samples[0];
        if shift {
            let (dx, dy) = (screen.0-stroke.origin_screen.0, screen.1-stroke.origin_screen.1);
            if stroke.axis.is_none() && dx.abs().max(dy.abs()) >= 3.0 { stroke.axis = Some(dx.abs() >= dy.abs()); }
            match stroke.axis { Some(true) => pitch = origin.1, Some(false) => tick = origin.0, None => {} }
        } else { stroke.axis = None; }
        // Coalesce subpixel motion, retaining the current endpoint rather than
        // growing a path whose extra vertices cannot affect the display.
        let sample = (tick.max(0.0), pitch);
        if stroke.samples.len() > 1 && stroke.samples.last().is_some_and(|last|
            (last.0-sample.0).abs()*vp.px_per_tick < 1.0 && (last.1-sample.1).abs()*vp.row_height < 1.0) {
            *stroke.samples.last_mut().unwrap() = sample;
        } else { stroke.samples.push(sample); }
    }

    pub fn commit_roll_pitch(&mut self, cx: &mut Context<Self>) {
        let Some(stroke) = self.lane_stroke.take() else { return };
        if stroke.roll && stroke.base == self.revision() {
            let batch = self.active_track().and_then(|track|
                crate::curve_span::pitch_line_batch(track, &self.tempo(), &stroke.samples, false));
            if let Some(batch) = batch { self.commit(stroke.base, batch, cx); }
        }
        cx.notify();
    }

    pub fn curve_sample(&mut self, pointer: PointerId, mut tick: f64, mut value: f64, screen: (f64, f64), shift: bool, alt: bool) {
        let Some(stroke) = self.lane_stroke.as_ref().filter(|s| s.pointer == pointer) else { return };
        if !alt && stroke.param != Param::Portamento { tick = self.session.snap.snap(tick) as f64; }
        if let Some(note) = stroke.note.as_ref().and_then(|id| self.active_track().and_then(|t| self.index.note(t, id))) {
            tick = tick.clamp(note.start_tick as f64, note.end_tick() as f64);
        }
        let Some(stroke) = self.lane_stroke.as_mut() else { return };
        let origin = stroke.samples[0];
        if shift {
            let (dx, dy) = (screen.0 - stroke.origin_screen.0, screen.1 - stroke.origin_screen.1);
            if stroke.axis.is_none() && dx.abs().max(dy.abs()) >= 3.0 { stroke.axis = Some(dx.abs() >= dy.abs()); }
            match stroke.axis { Some(true) => value = origin.1, Some(false) => tick = origin.0, None => {}, }
        } else { stroke.axis = None; }
        value = match stroke.param {
            Param::Pit => value.clamp(-1200.0, 1200.0),
            Param::Dyn => value.clamp(-24.0, 6.0),
            Param::Vibrato => value.clamp(0.0, 200.0),
            _ if stroke.param.is_expression() => value.clamp(0.0, 127.0).round(),
            _ => return,
        };
        if stroke.erase { value = stroke.param.default_value(); }
        if matches!(stroke.tool,LaneTool::Line|LaneTool::Select) || stroke.point.is_some() {
            if stroke.samples.len() > 1 { stroke.samples[1] = (tick, value); }
            else { stroke.samples.push((tick, value)); }
        } else if let Some(last) = stroke.samples.last_mut().filter(|s| s.0 == tick) { last.1 = value; }
        else { stroke.samples.push((tick, value)); }
    }

    pub(super) fn commit_point_drag(&mut self, stroke: LaneStroke, cx: &mut Context<Self>) {
        if stroke.base != self.revision() { return; }
        if let (Some(note),Some(point),Some(first),Some(last))=(&stroke.note,&stroke.point,stroke.samples.first(),stroke.samples.last()){
            if self.point_selection.contains(&(note.clone(),point.clone()))&&self.point_selection.len()>1{
                if let Err(error)=self.transform_selected_points((last.0-first.0).round()as i64,1.,last.1-first.1,1.,cx){self.notice(error);cx.notify();}return;
            }
        }
        let (Some(id), Some(point), Some((absolute, value))) = (stroke.note, stroke.point, stroke.samples.last().copied()) else { return };
        let Some(note) = self.active_track().and_then(|t| self.index.note(t, &id)) else { return };
        if region_parameter(stroke.param) && lane_disabled(note, stroke.param) {
            self.notice("先启用此区域曲线，再移动控制点"); cx.notify(); return;
        }
        let tick = (absolute.round() as i64 - note.start_tick).clamp(0, note.duration_tick);
        let command = match stroke.param {
            Param::Pit => {
                let mut points = note.curves.pitch_millicent_points.clone();
                let Some(p) = points.iter_mut().find(|p| p.id == point) else { return };
                p.tick = tick; p.millicents = (value * 1000.0).round() as i64;
                Command::SetPitchCurve { track: self.session.track.clone(), note_id: id, points }
            }
            Param::Dyn => {
                let mut points = note.curves.gain_millidecibel_points.clone();
                let Some(p) = points.iter_mut().find(|p| p.id == point) else { return };
                p.tick = tick; p.millidecibels = (value * 1000.0).round() as i64;
                Command::SetGainCurve { track: self.session.track.clone(), note_id: id, points }
            }
            Param::Portamento => {
                if !note.curves.expression("POR").first().is_some_and(|p| p.id == point) { return; }
                let points = verdant_backend::params::portamento_points_for_value(note, value);
                Command::SetExpressionCurve { track: self.session.track.clone(), note_id: id, parameter: "POR".into(), points }
            }
            _ if stroke.param.is_expression() => {
                let mut points = note.curves.expression(stroke.param.short()).to_vec();
                let Some(p) = points.iter_mut().find(|p| p.id == point) else { return };
                p.tick = tick; p.value = value.round().clamp(0., 127.) as u8;
                Command::SetExpressionCurve { track: self.session.track.clone(), note_id: id, parameter: stroke.param.short().into(), points }
            }
            _ => return,
        };
        self.commit(stroke.base, CommandBatch::single("移动曲线控制点", command), cx);
    }

    pub fn step_curve_point(&mut self, note: &NoteId, point: &PointId, param: Param, delta: f64, cx: &mut Context<Self>) {
        if !verdant_backend::params::support(param, self.route(), self.scheduler.capabilities.as_ref()).is_effective() {
            self.notice(format!("当前后端不能执行 {} 参数", param.short()));
            cx.notify();
            return;
        }
        let Some(n) = self.active_track().and_then(|t| self.index.note(t, note)) else { return };
        if region_parameter(param) && lane_disabled(n, param) {
            self.notice("先启用此区域曲线，再编辑控制点"); cx.notify(); return;
        }
        let command = match param {
            Param::Pit => {
                let mut points: Vec<PitchPoint> = n.curves.pitch_millicent_points.clone();
                let Some(p) = points.iter_mut().find(|p| &p.id == point) else { return };
                p.millicents = (p.millicents + (delta * 1000.0).round() as i64).clamp(-1_200_000, 1_200_000);
                Command::SetPitchCurve { track: self.session.track.clone(), note_id: note.clone(), points }
            }
            Param::Dyn => {
                let mut points: Vec<GainPoint> = n.curves.gain_millidecibel_points.clone();
                let Some(p) = points.iter_mut().find(|p| &p.id == point) else { return };
                p.millidecibels = (p.millidecibels + (delta * 1000.0).round() as i64).clamp(-24_000, 6000);
                Command::SetGainCurve { track: self.session.track.clone(), note_id: note.clone(), points }
            }
            Param::Portamento => {
                if !n.curves.expression("POR").first().is_some_and(|p| &p.id == point) { return; }
                let value = verdant_backend::f0::portamento_position(n)+delta;
                let points = verdant_backend::params::portamento_points_for_value(n, value);
                Command::SetExpressionCurve { track: self.session.track.clone(), note_id: note.clone(), parameter: "POR".into(), points }
            }
            _ if param.is_expression() => {
                let mut points = n.curves.expression(param.short()).to_vec();
                let Some(p) = points.iter_mut().find(|p| &p.id == point) else { return };
                p.value = (p.value as f64 + delta).round().clamp(0., 127.) as u8;
                Command::SetExpressionCurve { track: self.session.track.clone(), note_id: note.clone(), parameter: param.short().into(), points }
            }
            _ => return,
        };
        self.commit(self.revision(), CommandBatch::single("曲线控制点步进", command), cx);
    }
}
