//! Exact tick ↔ time ↔ sample mapping (SPEC §4.2).
//!
//! Piecewise-constant tempo is integrated segment by segment. Sample boundaries
//! are always derived from absolute times (`round(t * sr)`) so per-note rounding
//! never accumulates drift.

use crate::model::{PPQ, TempoPoint};

#[derive(Clone, Debug, PartialEq)]
pub struct TempoMap {
    /// Sorted by tick, first point at tick 0, all `micro_bpm > 0`.
    points: Vec<TempoPoint>,
    ppq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TempoError {
    #[error("tempo map is empty")]
    Empty,
    #[error("tempo map must start at tick 0")]
    NotAnchored,
    #[error("tempo ticks must strictly increase")]
    NotIncreasing,
    #[error("tempo must be positive")]
    NonPositive,
    #[error("tempo event tick must be non-negative")]
    NegativeTick,
    #[error("the tempo event at tick zero cannot be removed")]
    AnchorRequired,
    #[error("ppq must be positive")]
    BadPpq,
}

impl TempoMap {
    pub fn new(points: &[TempoPoint], ppq: i64) -> Result<Self, TempoError> {
        if ppq <= 0 {
            return Err(TempoError::BadPpq);
        }
        let first = points.first().ok_or(TempoError::Empty)?;
        if first.tick != 0 {
            return Err(TempoError::NotAnchored);
        }
        for w in points.windows(2) {
            if w[1].tick <= w[0].tick {
                return Err(TempoError::NotIncreasing);
            }
        }
        if points.iter().any(|p| p.micro_bpm == 0) {
            return Err(TempoError::NonPositive);
        }
        Ok(TempoMap { points: points.to_vec(), ppq })
    }

    pub fn constant(bpm: f64) -> Self {
        TempoMap { points: vec![TempoPoint { tick: 0, micro_bpm: (bpm * 1e6).round() as u64, unknown: Default::default() }], ppq: PPQ }
    }

    /// Validated, ordered tempo boundaries for compiling tick-based curves.
    pub fn points(&self) -> &[TempoPoint] {
        &self.points
    }

    pub fn into_points(self) -> Vec<TempoPoint> {
        self.points
    }

    /// Insert, replace the BPM of, or delete one boundary. Existing event
    /// extension fields survive edits; unrelated imported boundaries are exact.
    /// A missing deletion is a no-op, and tick zero always remains anchored.
    pub fn set_event(&mut self, tick: i64, micro_bpm: Option<u64>) -> Result<(), TempoError> {
        if tick < 0 { return Err(TempoError::NegativeTick); }
        if micro_bpm == Some(0) { return Err(TempoError::NonPositive); }
        if tick == 0 && micro_bpm.is_none() { return Err(TempoError::AnchorRequired); }
        match (self.points.binary_search_by_key(&tick, |point| point.tick), micro_bpm) {
            (Ok(index), Some(bpm)) => self.points[index].micro_bpm = bpm,
            (Ok(index), None) => { self.points.remove(index); }
            (Err(index), Some(bpm)) => self.points.insert(index, TempoPoint {
                tick, micro_bpm: bpm, unknown: Default::default(),
            }),
            (Err(_), None) => {},
        }
        Ok(())
    }

    fn seconds_per_tick(&self, micro_bpm: u64) -> f64 {
        60.0 * 1e6 / (self.ppq as f64 * micro_bpm as f64)
    }

    /// Seconds from tick 0 to `tick` (negative ticks extrapolate the first tempo).
    pub fn seconds_at(&self, tick: f64) -> f64 {
        let mut seconds = 0.0;
        for (i, p) in self.points.iter().enumerate() {
            let seg_end = self.points.get(i + 1).map(|n| n.tick as f64).unwrap_or(f64::INFINITY);
            let start = p.tick as f64;
            if tick <= seg_end || i + 1 == self.points.len() {
                return seconds + (tick - start) * self.seconds_per_tick(p.micro_bpm);
            }
            seconds += (seg_end - start) * self.seconds_per_tick(p.micro_bpm);
        }
        unreachable!("tempo map has at least one point")
    }

    /// Inverse of [`seconds_at`](Self::seconds_at).
    pub fn tick_at(&self, seconds: f64) -> f64 {
        let mut acc = 0.0;
        for (i, p) in self.points.iter().enumerate() {
            let spt = self.seconds_per_tick(p.micro_bpm);
            let next = self.points.get(i + 1);
            let seg_seconds = next.map(|n| (n.tick - p.tick) as f64 * spt).unwrap_or(f64::INFINITY);
            if seconds <= acc + seg_seconds || next.is_none() {
                return p.tick as f64 + (seconds - acc) / spt;
            }
            acc += seg_seconds;
        }
        unreachable!("tempo map has at least one point")
    }

    /// Absolute sample index of a tick at `sample_rate`.
    pub fn sample_at(&self, tick: i64, sample_rate: u32) -> i64 {
        (self.seconds_at(tick as f64) * sample_rate as f64).round() as i64
    }

    pub fn bpm_at(&self, tick: i64) -> f64 {
        let idx = self.points.partition_point(|p| p.tick <= tick).saturating_sub(1);
        self.points[idx].micro_bpm as f64 / 1e6
    }
}


/// A meter change starts a new measure, including an imported change in the
/// middle of the preceding measure. All UI positions use this same policy.
#[derive(Clone, Debug)]
pub struct MeterMap { segments: Vec<MeterSegment> }
#[derive(Clone, Debug)]
struct MeterSegment { signature: crate::model::TimeSignature, first_bar: i64, beat: i64, bar: i64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicalPosition { pub bar: i64, pub beat: i64, pub subdivision: i64, pub tick: i64 }

impl MeterMap {
    pub fn new(signatures: &[crate::model::TimeSignature], ppq: i64) -> Result<Self, String> {
        if ppq <= 0 || signatures.first().is_none_or(|s| s.tick != 0) { return Err("meter map must start at tick zero with positive PPQ".into()); }
        let mut segments: Vec<MeterSegment> = Vec::with_capacity(signatures.len());
        for signature in signatures {
            if signature.numerator == 0 || signature.numerator > 64 || !signature.denominator.is_power_of_two()
                || signature.denominator > 64 || ppq.checked_mul(4).is_none_or(|q| q % i64::from(signature.denominator) != 0) {
                return Err("meter requires numerator 1..64 and a representable power-of-two denominator 1..64".into());
            }
            let beat = ppq * 4 / i64::from(signature.denominator);
            let first_bar = if let Some(previous) = segments.last() {
                if signature.tick <= previous.signature.tick { return Err("meter ticks must strictly increase".into()); }
                previous.first_bar + ((signature.tick - previous.signature.tick) / previous.bar + i64::from((signature.tick - previous.signature.tick) % previous.bar != 0))
            } else { 0 };
            segments.push(MeterSegment { signature: signature.clone(), first_bar, beat, bar: beat * i64::from(signature.numerator) });
        }
        Ok(Self { segments })
    }
    fn segment(&self, tick: i64) -> &MeterSegment {
        &self.segments[self.segments.partition_point(|s| s.signature.tick <= tick).saturating_sub(1)]
    }
    pub fn signature_at(&self, tick: i64) -> &crate::model::TimeSignature { &self.segment(tick).signature }
    pub fn beat_ticks_at(&self, tick: i64) -> i64 { self.segment(tick).beat }
    pub fn bar_ticks_at(&self, tick: i64) -> i64 { self.segment(tick).bar }
    /// Start tick of a one-based measure number, as displayed by the ruler and
    /// LCD. A meter event starts a new measure even if it truncates the old one.
    /// Invalid measure numbers and positions outside the tick range have no tick.
    pub fn tick_at_bar(&self, bar: i64) -> Option<i64> {
        let index = bar.checked_sub(1).filter(|index| *index >= 0)?;
        let segment = &self.segments[self.segments.partition_point(|s| s.first_bar <= index).saturating_sub(1)];
        segment.signature.tick.checked_add(index.checked_sub(segment.first_bar)?.checked_mul(segment.bar)?)
    }
    pub fn bar_start(&self, tick: i64) -> i64 {
        let s = self.segment(tick.max(0)); s.signature.tick + (tick.max(0) - s.signature.tick) / s.bar * s.bar
    }
    pub fn next_bar(&self, tick: i64) -> i64 {
        let tick = tick.max(0); let s = self.segment(tick);
        let candidate = self.bar_start(tick).saturating_add(s.bar);
        self.segments.iter().find(|s| s.signature.tick > tick).map_or(candidate, |s| candidate.min(s.signature.tick))
    }
    pub fn previous_bar(&self, tick: i64) -> i64 { self.bar_start(tick.saturating_sub(1).max(0)) }
    pub fn bars(&self, from: i64, to: i64) -> impl Iterator<Item = i64> + '_ {
        let from=from.max(0);let start=self.bar_start(from);let mut next=if start<from{self.next_bar(from)}else{start};
        std::iter::from_fn(move||{if next>=to{return None;}let current=next;let following=self.next_bar(current);if following<=current{next=i64::MAX;}else{next=following;}Some(current)})
    }
    pub fn position_at(&self, tick: i64) -> MusicalPosition {
        let tick = tick.max(0); let s = self.segment(tick); let relative = tick - s.signature.tick;
        let within = relative % s.bar; let subdivision = (s.beat / 4).max(1);
        MusicalPosition { bar: s.first_bar + relative / s.bar + 1, beat: within / s.beat + 1,
            subdivision: (within % s.beat) / subdivision + 1, tick: within % subdivision + 1 }
    }
    /// Visible musical grid with bounded work: subdivisions, beats and bars
    /// merge at meter boundaries instead of assuming an absolute 4/4 origin.
    pub fn grid(&self, from: i64, to: i64, subdivision: i64) -> impl Iterator<Item = (i64, bool, bool)> + '_ {
        let mut next = from.max(0); let step = subdivision.max(1);
        std::iter::from_fn(move || {
            if next >= to { return None; }
            let s = self.segment(next); let local = next - s.signature.tick;
            let candidate = s.signature.tick.saturating_add((local / step + i64::from(local % step != 0)).saturating_mul(step));
            let beat = s.signature.tick.saturating_add((local / s.beat + i64::from(local % s.beat != 0)).saturating_mul(s.beat));
            let boundary = self.segments.iter().find(|s| s.signature.tick > next).map(|s| s.signature.tick).unwrap_or(i64::MAX);
            let tick = candidate.min(beat).min(boundary);
            if tick >= to || tick == i64::MAX { return None; }
            next = tick.saturating_add(1);
            let active = self.segment(tick); let offset = tick - active.signature.tick;
            Some((tick, offset % active.bar == 0, offset % active.beat == 0))
        })
    }
}

/// `F0 = 440 · 2^((midi − 69 + cents/100) / 12)` (SPEC §4.2).
pub fn midi_to_hz(midi_note: i32, millicents: i64) -> f64 {
    let semis = midi_note as f64 - 69.0 + millicents as f64 / 100_000.0;
    440.0 * (semis / 12.0).exp2()
}

pub fn note_name(midi_note: i32) -> String {
    const NAMES: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
    let octave = midi_note.div_euclid(12) - 1;
    format!("{}{}", NAMES[midi_note.rem_euclid(12) as usize], octave)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_tempo_round_trip() {
        let map = TempoMap::constant(120.0);
        assert!((map.seconds_at(960.0) - 0.5).abs() < 1e-12);
        assert!((map.tick_at(0.5) - 960.0).abs() < 1e-9);
        assert_eq!(map.sample_at(960, 16000), 8000);
    }

    #[test]
    fn piecewise_tempo_integrates() {
        let map =
            TempoMap::new(&[TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: Default::default() }, TempoPoint { tick: 960, micro_bpm: 60_000_000, unknown: Default::default() }], 960).unwrap();
        // 1 beat at 120 = 0.5 s, then 1 beat at 60 = 1 s.
        assert!((map.seconds_at(1920.0) - 1.5).abs() < 1e-12);
        assert!((map.tick_at(1.5) - 1920.0).abs() < 1e-9);
        assert_eq!(map.bpm_at(1000), 60.0);
    }

    #[test]
    fn a4_is_440() {
        assert!((midi_to_hz(69, 0) - 440.0).abs() < 1e-9);
        assert!((midi_to_hz(60, 0) - 261.6255653).abs() < 1e-6);
        assert!((midi_to_hz(68, 100_000) - 440.0).abs() < 1e-9);
        assert_eq!(note_name(60), "C4");
    }

    #[test]
    fn rejects_bad_maps() {
        assert_eq!(TempoMap::new(&[], 960), Err(TempoError::Empty));
        assert_eq!(TempoMap::new(&[TempoPoint { tick: 5, micro_bpm: 1, unknown: Default::default() }], 960), Err(TempoError::NotAnchored));
    }
}
