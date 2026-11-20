//! Immutable automation compiled on the control thread, evaluated against the
//! source sample clock by playback and export. Evaluation allocates nothing,
//! performs no I/O and carries no transport history: a seek or loop restart
//! reconstructs the exact value at its absolute project sample.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleInterpolation { Step, Linear }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SamplePoint {
    pub sample: u64,
    pub value: f64,
    pub interpolation: SampleInterpolation,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SampleCurve { points: Vec<SamplePoint> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleError(pub &'static str);
impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.0) }
}
impl std::error::Error for ScheduleError {}

impl SampleCurve {
    pub fn new(points: Vec<SamplePoint>) -> Result<Self, ScheduleError> {
        if points.iter().any(|p| !p.value.is_finite()) {
            return Err(ScheduleError("automation values must be finite"));
        }
        if points.windows(2).any(|pair| pair[0].sample >= pair[1].sample) {
            return Err(ScheduleError("automation sample coordinates must strictly increase"));
        }
        Ok(Self { points })
    }

    pub fn points(&self) -> &[SamplePoint] { &self.points }
    pub fn is_empty(&self) -> bool { self.points.is_empty() }

    pub fn value(&self, sample: u64) -> Option<f64> {
        self.value_at_index(sample, self.points.partition_point(|point| point.sample <= sample))
    }

    pub fn value_at(&self, sample: u64, fallback: f64) -> f64 { self.value(sample).unwrap_or(fallback) }

    fn value_at_index(&self, sample: u64, upper: usize) -> Option<f64> {
        let left = self.points.get(upper.checked_sub(1)?)?;
        let Some(right) = self.points.get(upper) else { return Some(left.value); };
        if left.interpolation == SampleInterpolation::Step { return Some(left.value); }
        let fraction = (sample - left.sample) as f64 / (right.sample - left.sample) as f64;
        Some(interpolate_value(left.value, right.value, fraction))
    }

    pub fn next_boundary_after(&self, sample: u64) -> Option<u64> {
        self.points.get(self.points.partition_point(|point| point.sample <= sample)).map(|point| point.sample)
    }

    /// Includes a value at offset zero, even after a discontinuous seek/loop.
    /// Linear spans generate exact native values per sample; constant spans
    /// jump directly to the next authored point. A caller can feed these into
    /// bounded native-event storage while splitting the processing quantum.
    pub fn events(&self, start: u64, frames: usize, fallback: f64) -> ParameterEvents<'_> {
        ParameterEvents { cursor: CurveCursor::new(self, start), origin: start, next: start,
            remaining: frames, fallback, previous: None }
    }

    pub fn values(&self, start: u64, frames: usize, fallback: f64) -> SampleValues<'_> {
        SampleValues { cursor: CurveCursor::new(self, start), position: start, remaining: frames, fallback }
    }
}

struct CurveCursor<'a> { curve: &'a SampleCurve, upper: usize }
impl<'a> CurveCursor<'a> {
    fn new(curve: &'a SampleCurve, start: u64) -> Self {
        Self { curve, upper: curve.points.partition_point(|point| point.sample <= start) }
    }
    fn value(&mut self, sample: u64) -> Option<f64> {
        while self.curve.points.get(self.upper).is_some_and(|point| point.sample <= sample) { self.upper += 1; }
        self.curve.value_at_index(sample, self.upper)
    }
    fn next_change(&self, sample: u64) -> Option<u64> {
        let right = self.curve.points.get(self.upper)?;
        let linear = self.upper.checked_sub(1).and_then(|i| self.curve.points.get(i))
            .is_some_and(|left| left.interpolation == SampleInterpolation::Linear && left.value != right.value);
        if linear { sample.checked_add(1) } else { Some(right.sample) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterEvent { pub offset: usize, pub value: f64 }

pub struct ParameterEvents<'a> {
    cursor: CurveCursor<'a>,
    origin: u64,
    next: u64,
    remaining: usize,
    fallback: f64,
    previous: Option<f64>,
}

impl Iterator for ParameterEvents<'_> {
    type Item = ParameterEvent;
    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining != 0 {
            let sample = self.next;
            let value = self.cursor.value(sample).unwrap_or(self.fallback);
            let advance = self.cursor.next_change(sample).and_then(|next| usize::try_from(next - sample).ok())
                .unwrap_or(self.remaining).min(self.remaining).max(1);
            self.remaining -= advance;
            self.next = sample.saturating_add(advance as u64);
            if self.previous.is_none_or(|previous| previous != value) {
                self.previous = Some(value);
                return Some(ParameterEvent { offset: (sample - self.origin) as usize, value });
            }
        }
        None
    }
}

pub struct SampleValues<'a> { cursor: CurveCursor<'a>, position: u64, remaining: usize, fallback: f64 }
impl Iterator for SampleValues<'_> {
    type Item = f64;
    fn next(&mut self) -> Option<f64> {
        if self.remaining == 0 { return None; }
        let value = self.cursor.value(self.position).unwrap_or(self.fallback);
        self.remaining -= 1;
        self.position = self.position.saturating_add(1);
        Some(value)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.remaining, Some(self.remaining)) }
}
impl ExactSizeIterator for SampleValues<'_> {}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginAutomation { pub instance_id: String, pub parameter_id: String, pub curve: SampleCurve }

/// One format-native parameter, resolved and range-checked on the control
/// thread. The fallback is the parameter's saved/current native value, not an
/// assumed normalized value or an arbitrary zero.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeParameterLane { pub native_id: u32, pub fallback: f64, pub curve: SampleCurve }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativePluginSchedule { pub instance_id: String, pub parameters: Vec<NativeParameterLane> }

impl NativePluginSchedule {
    /// Control-thread admission of every possible quantum, including one that
    /// begins at an arbitrary seek position. Event-count slopes change only at
    /// a knot or when a knot enters/leaves the quantum; checking those discrete
    /// boundaries avoids scanning the duration of a long project.
    pub fn validate_density(&self, quantum_frames: usize, max_events: usize) -> Result<(), ScheduleError> {
        if quantum_frames == 0 || quantum_frames > u32::MAX as usize || max_events == 0 {
            return Err(ScheduleError("plugin automation requires positive bounded quantum/event capacity"));
        }
        let mut ids = Vec::with_capacity(self.parameters.len());
        let capacity = self.parameters.iter().fold(1_usize, |count, parameter| count.saturating_add(parameter.curve.points().len().saturating_mul(5)));
        let mut starts = Vec::with_capacity(capacity);
        starts.push(0);
        let quantum = quantum_frames as u64;
        for parameter in &self.parameters {
            if !parameter.fallback.is_finite() { return Err(ScheduleError("plugin automation fallback must be finite")); }
            ids.push(parameter.native_id);
            for point in parameter.curve.points() {
                starts.extend([point.sample, point.sample.saturating_sub(1), point.sample.saturating_add(1), point.sample.saturating_sub(quantum),
                    point.sample.saturating_sub(quantum - 1)]);
            }
        }
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScheduleError("duplicate native plugin parameter automation"));
        }
        starts.sort_unstable();
        starts.dedup();
        for start in starts {
            let mut count = 0_usize;
            for parameter in &self.parameters {
                count = count.saturating_add(event_upper_bound(&parameter.curve, start, quantum_frames, parameter.fallback));
                if count > max_events { return Err(ScheduleError("plugin automation exceeds native quantum event capacity")); }
            }
        }
        Ok(())
    }

    /// Fill caller-owned storage before publishing a native processing block.
    /// On an error no count is returned: partial storage must never be sent.
    /// Sorting in place uses no temporary allocation. The caller chooses a
    /// bounded quantum that passed validate_density before audio activation.
    pub fn write_events(&self, start: u64, frames: usize, out: &mut [(u32, u32, f64)]) -> Result<usize, ScheduleError> {
        if frames > u32::MAX as usize || start.checked_add(frames as u64).is_none() {
            return Err(ScheduleError("plugin automation range exceeds event offset clock"));
        }
        let mut count = 0;
        for parameter in &self.parameters {
            for event in parameter.curve.events(start, frames, parameter.fallback) {
                let slot = out.get_mut(count).ok_or(ScheduleError("plugin automation exceeds supplied event storage"))?;
                *slot = (parameter.native_id, event.offset as u32, event.value);
                count += 1;
            }
        }
        out[..count].sort_unstable_by_key(|event| (event.1, event.0));
        Ok(count)
    }
}

// Bound the ideal linear changes even when floating rounding merges adjacent
// values. Actual event density can only be smaller, never an unobserved peak
// between the control-side candidate windows.
fn event_upper_bound(curve: &SampleCurve, start: u64, frames: usize, fallback: f64) -> usize {
    if frames == 0 { return 0; }
    let last = start.saturating_add(frames as u64 - 1);
    let mut count = 1_usize; // Every processing range reconstructs initial state.
    if let Some(first) = curve.points.first() {
        if first.sample > start && first.sample <= last && first.value != fallback { count += 1; }
    }
    let first_edge = curve.points.partition_point(|point| point.sample <= start).saturating_sub(1);
    for pair in curve.points[first_edge..].windows(2).take_while(|pair| pair[0].sample < last) {
        if pair[0].value == pair[1].value { continue; }
        if pair[0].interpolation == SampleInterpolation::Linear {
            let from = start.saturating_add(1).max(pair[0].sample.saturating_add(1));
            let to = last.min(pair[1].sample);
            if from <= to { count = count.saturating_add((to - from + 1) as usize); }
        } else if pair[1].sample > start && pair[1].sample <= last { count += 1; }
    }
    count
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChannelAutomation {
    /// Stored millidecibels; the ordinary channel gain is the fallback.
    pub gain: SampleCurve,
    /// Stored permille; the ordinary channel pan is the fallback.
    pub pan: SampleCurve,
    pub plugins: Vec<PluginAutomation>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AutomationSchedule {
    /// In current mixer track order. The compiler binds persistent TrackId to
    /// the index once; an index never becomes the persisted target identity.
    pub tracks: Vec<ChannelAutomation>,
    pub master: ChannelAutomation,
}

impl AutomationSchedule {
    pub fn is_empty(&self) -> bool {
        self.master.is_empty() && self.tracks.iter().all(ChannelAutomation::is_empty)
    }
}

impl ChannelAutomation {
    pub fn is_empty(&self) -> bool { self.gain.is_empty() && self.pan.is_empty() && self.plugins.is_empty() }

    pub fn gain_at(&self, sample: u64, fallback_linear: f32) -> f32 {
        self.gain.value(sample).map_or(fallback_linear, millidecibels_to_gain)
    }

    pub fn pan_at(&self, sample: u64, fallback_pan: f32) -> f32 {
        self.pan.value(sample).map_or(fallback_pan, |value| (value / 1000.0) as f32)
    }

    pub fn gains_at(&self, sample: u64, fallback_gain: f32, fallback_pan: f32) -> (f32, f32) {
        balance_gains(self.gain_at(sample, fallback_gain), self.pan_at(sample, fallback_pan))
    }

    /// Both consumers use the established unity-at-centre stereo balance law.
    /// Cursors advance once per knot, avoiding a binary search for each sample.
    pub fn gains(&self, start: u64, frames: usize, fallback_gain: f32, fallback_pan: f32) -> ChannelGains<'_> {
        ChannelGains { gain: CurveCursor::new(&self.gain, start), pan: CurveCursor::new(&self.pan, start),
            position: start, remaining: frames, fallback_gain, fallback_pan }
    }

    pub fn apply_stereo(&self, start: u64, left: &mut [f32], right: &mut [f32], fallback_gain: f32, fallback_pan: f32)
        -> Result<(), ScheduleError> {
        if left.len() != right.len() { return Err(ScheduleError("automation stereo lengths differ")); }
        let frames = left.len();
        for ((left, right), (gl, gr)) in left.iter_mut().zip(right).zip(self.gains(start, frames, fallback_gain, fallback_pan)) {
            *left *= gl;
            *right *= gr;
        }
        Ok(())
    }

    pub fn plugin(&self, instance_id: &str, parameter_id: &str) -> Option<&PluginAutomation> {
        self.plugins.iter().find(|lane| lane.instance_id == instance_id && lane.parameter_id == parameter_id)
    }

    pub fn next_boundary_after(&self, sample: u64) -> Option<u64> {
        self.gain.next_boundary_after(sample).into_iter().chain(self.pan.next_boundary_after(sample))
            .chain(self.plugins.iter().filter_map(|lane| lane.curve.next_boundary_after(sample))).min()
    }
}

pub struct ChannelGains<'a> {
    gain: CurveCursor<'a>, pan: CurveCursor<'a>, position: u64, remaining: usize,
    fallback_gain: f32, fallback_pan: f32,
}
impl Iterator for ChannelGains<'_> {
    type Item = (f32, f32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 { return None; }
        let gain = self.gain.value(self.position).map_or(self.fallback_gain, millidecibels_to_gain);
        let pan = self.pan.value(self.position).map_or(self.fallback_pan, |value| (value / 1000.0) as f32);
        self.remaining -= 1;
        self.position = self.position.saturating_add(1);
        Some(balance_gains(gain, pan))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.remaining, Some(self.remaining)) }
}
impl ExactSizeIterator for ChannelGains<'_> {}

/// Convex interpolation of finite endpoints without overflowing their
/// difference when a native parameter spans both signs.
fn interpolate_value(left: f64, right: f64, fraction: f64) -> f64 {
    if fraction <= 0.0 { return left; }
    if fraction >= 1.0 { return right; }
    if left.is_sign_negative() == right.is_sign_negative() {
        left + (right - left) * fraction
    } else { left * (1.0 - fraction) + right * fraction }
}

pub fn millidecibels_to_gain(value: f64) -> f32 { 10_f64.powf(value / 20_000.0) as f32 }
pub fn balance_gains(gain: f32, pan: f32) -> (f32, f32) {
    (gain * (1.0 - pan).min(1.0), gain * (1.0 + pan).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(sample: u64, value: f64, interpolation: SampleInterpolation) -> SamplePoint {
        SamplePoint { sample, value, interpolation }
    }

    #[test]
    fn seek_and_loop_reconstruct_the_same_parameter_state() {
        let curve = SampleCurve::new(vec![point(4, 0.0, SampleInterpolation::Linear), point(8, 1.0, SampleInterpolation::Step)]).unwrap();
        assert_eq!(curve.events(6, 3, -1.0).collect::<Vec<_>>(), vec![
            ParameterEvent { offset: 0, value: 0.5 }, ParameterEvent { offset: 1, value: 0.75 }, ParameterEvent { offset: 2, value: 1.0 }]);
        let first = curve.events(4, 5, -1.0).collect::<Vec<_>>();
        assert_eq!(curve.events(4, 5, -1.0).collect::<Vec<_>>(), first);
        assert_eq!(curve.value_at(2, 7.0), 7.0);
        assert_eq!(curve.value_at(80, 7.0), 1.0);
    }

    #[test]
    fn subranges_and_whole_render_are_sample_identical() {
        let curve = SampleCurve::new(vec![point(2, -6000.0, SampleInterpolation::Linear), point(9, 0.0, SampleInterpolation::Step)]).unwrap();
        let channel = ChannelAutomation { gain: curve, pan: SampleCurve::new(vec![point(0, -1000.0, SampleInterpolation::Linear), point(10, 1000.0, SampleInterpolation::Step)]).unwrap(), plugins: Vec::new() };
        let whole = channel.gains(0, 20, 1.0, 0.0).collect::<Vec<_>>();
        let chunks = channel.gains(0, 7, 1.0, 0.0).chain(channel.gains(7, 6, 1.0, 0.0))
            .chain(channel.gains(13, 7, 1.0, 0.0)).collect::<Vec<_>>();
        assert_eq!(whole, chunks);
        assert_eq!(whole[0], (1.0, 0.0));
        assert_eq!(whole[10], (0.0, 1.0));
    }

    #[test]
    fn held_parameter_emits_only_initial_and_changed_boundaries() {
        let curve = SampleCurve::new(vec![point(10, 2.0, SampleInterpolation::Step), point(100, 4.0, SampleInterpolation::Step)]).unwrap();
        assert_eq!(curve.events(0, 1000, 1.0).collect::<Vec<_>>(), vec![
            ParameterEvent { offset: 0, value: 1.0 }, ParameterEvent { offset: 10, value: 2.0 }, ParameterEvent { offset: 100, value: 4.0 }]);
        assert_eq!(curve.events(100, 0, 1.0).count(), 0);
    }

    #[test]
    fn native_density_checks_seek_windows_and_never_returns_partial_events() {
        let parameters = (0..17).map(|native_id| NativeParameterLane { native_id, fallback: 0.0,
            curve: SampleCurve::new(vec![point(100, 0.0, SampleInterpolation::Linear), point(1000, 1.0, SampleInterpolation::Step)]).unwrap() }).collect();
        let mut schedule = NativePluginSchedule { instance_id: "effect-instance".into(), parameters };
        assert!(schedule.validate_density(256, 4096).is_err());
        schedule.parameters.pop();
        schedule.validate_density(256, 4096).unwrap();
        let mut small = [(0, 0, 0.0); 3];
        assert!(schedule.write_events(200, 256, &mut small).is_err());
        let mut storage = vec![(0, 0, 0.0); 4096];
        let count = schedule.write_events(200, 256, &mut storage).unwrap();
        assert_eq!(count, 4096);
        assert!(storage[..count].windows(2).all(|pair| (pair[0].1, pair[0].0) <= (pair[1].1, pair[1].0)));
        assert_eq!(storage[0].1, 0);
        assert_eq!(storage[count - 1].1, 255);
    }

    #[test]
    fn no_automation_retains_exact_mixer_defaults() {
        let channel = ChannelAutomation::default();
        assert_eq!(channel.gains(123, 3, 0.7, -0.25).collect::<Vec<_>>(), vec![balance_gains(0.7, -0.25); 3]);
    }
}
