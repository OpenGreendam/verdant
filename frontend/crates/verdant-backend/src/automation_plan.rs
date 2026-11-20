//! Compile persisted musical automation into the source sample clock once.
//! Playback and offline export consume this same schedule; neither schedules
//! changes by wall time or polls the UI. Linear musical ramps are split at
//! tempo boundaries so a tempo change does not change their tick-space shape.

use verdant_audio::automation_clock::{AutomationSchedule, ChannelAutomation, PluginAutomation, SampleCurve, SampleInterpolation, SamplePoint};
use verdant_domain::automation::{AutomationDocument, AutomationError, AutomationLane, AutomationOwner, AutomationTarget, Interpolation, interpolate_value};
use verdant_domain::{Project, TempoMap};

pub fn compile_automation(project: &Project, sample_rate: u32) -> Result<AutomationSchedule, AutomationError> {
    if sample_rate == 0 { return Err(AutomationError::Invalid("sample rate must be positive".into())); }
    let document = AutomationDocument::from_project(project)?;
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).map_err(|error| AutomationError::Invalid(error.to_string()))?;
    let mut schedule = AutomationSchedule { tracks: vec![ChannelAutomation::default(); project.tracks.len()], master: ChannelAutomation::default() };
    for lane in document.lanes.iter().filter(|lane| lane.enabled && !lane.points.is_empty()) {
        let curve = compile_curve(lane, &tempo, sample_rate)?;
        let channel = match lane.target.owner() {
            AutomationOwner::Track { track_id } => {
                let index = project.tracks.iter().position(|track| track.id == track_id)
                    .ok_or_else(|| AutomationError::Invalid(format!("missing track {track_id}")))?;
                &mut schedule.tracks[index]
            }
            AutomationOwner::Master => &mut schedule.master,
        };
        match &lane.target {
            AutomationTarget::TrackGain { .. } | AutomationTarget::MasterGain => channel.gain = curve,
            AutomationTarget::TrackPan { .. } | AutomationTarget::MasterPan => channel.pan = curve,
            AutomationTarget::PluginParameter { instance_id, parameter_id, .. } => channel.plugins.push(PluginAutomation {
                instance_id: instance_id.clone(), parameter_id: parameter_id.clone(), curve,
            }),
        }
    }
    Ok(schedule)
}

fn compile_curve(lane: &AutomationLane, tempo: &TempoMap, sample_rate: u32) -> Result<SampleCurve, AutomationError> {
    let mut points = Vec::<SamplePoint>::with_capacity(lane.points.len());
    for (index, point) in lane.points.iter().enumerate() {
        let interpolation = match point.interpolation { Interpolation::Step => SampleInterpolation::Step, Interpolation::Linear => SampleInterpolation::Linear };
        push_point(&mut points, sample_at(tempo, point.tick, sample_rate)?, point.value, interpolation);
        if point.interpolation != Interpolation::Linear { continue; }
        let Some(next) = lane.points.get(index + 1) else { continue; };
        let from = tempo.points().partition_point(|boundary| boundary.tick <= point.tick);
        for boundary in tempo.points()[from..].iter().take_while(|boundary| boundary.tick < next.tick) {
            let fraction = (boundary.tick - point.tick) as f64 / (next.tick - point.tick) as f64;
            let value = interpolate_value(point.value, next.value, fraction);
            push_point(&mut points, sample_at(tempo, boundary.tick, sample_rate)?, value, SampleInterpolation::Linear);
        }
    }
    SampleCurve::new(points).map_err(|error| AutomationError::Invalid(error.to_string()))
}

fn sample_at(tempo: &TempoMap, tick: i64, sample_rate: u32) -> Result<u64, AutomationError> {
    let sample = (tempo.seconds_at(tick as f64) * sample_rate as f64).round();
    // Project/sample APIs use signed absolute sample indices. Reject saturation
    // explicitly instead of silently pinning an enormous tick to i64::MAX.
    if !sample.is_finite() || sample < 0.0 || sample >= i64::MAX as f64 {
        return Err(AutomationError::Invalid(format!("tick {tick} is outside the sample clock range")));
    }
    Ok(sample as u64)
}

fn push_point(points: &mut Vec<SamplePoint>, sample: u64, value: f64, interpolation: SampleInterpolation) {
    let point = SamplePoint { sample, value, interpolation };
    // Subsample-dense edits share one audible sample: the later tick wins.
    // Keep authored ticks intact in the document; this is clock quantization.
    if points.last().is_some_and(|previous| previous.sample == sample) {
        *points.last_mut().expect("last sample exists") = point;
    } else { points.push(point); }
}
