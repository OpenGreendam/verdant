//! Source expression units are based on OpenUtau's UCurve, UstNote,
//! UVibrato and RenderPhrase contracts; links and sampling limits are in README.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use serde_json::Value;
use verdant_domain::{Note, PointId, TempoMap, PPQ};
use verdant_domain::model::{GainPoint, PitchPoint};
use crate::{FieldDisposition as D, ImportError, ImportNotice, ImportReport, Result, field};

pub(crate) const SAMPLE_TICKS: i64 = 5;
#[derive(Default)]
pub(crate) struct NoteControl {
    pub pitch: Option<Vec<BendPoint>>,
    pub snap_first: bool,
    pub vibrato: Option<SourceVibrato>,
    pub intensity_percent: Option<f64>,
    pub tuning_cents: f64,
}
pub(crate) struct BendPoint { pub ms: f64, pub cents: f64, pub shape: Shape }
#[derive(Clone, Copy)]
pub(crate) enum Shape { Linear, InOut, In, Out }
impl Shape {
    pub fn parse(s: &str) -> Option<Self> {
        match s { "l" | "s" => Some(Self::Linear), "io" | "" => Some(Self::InOut), "i" | "j" => Some(Self::In), "o" | "r" => Some(Self::Out), _ => None }
    }
    fn fraction(self, x: f64) -> f64 {
        match self { Self::Linear => x, Self::InOut => (1.0 - (x * PI).cos()) / 2.0,
            Self::In => 1.0 - (x * PI / 2.0).cos(), Self::Out => (x * PI / 2.0).sin() }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct SourceVibrato {
    pub length: f64, pub period_ms: f64, pub depth_cents: f64,
    pub fade_in: f64, pub fade_out: f64, pub shift: f64, pub drift: f64, pub volume_link: f64,
}
impl SourceVibrato {
    pub fn validate(self, path: &str) -> Result<Self> {
        let valid = (0.0..=100.0).contains(&self.length) && (5.0..=500.0).contains(&self.period_ms)
            && (0.0..=200.0).contains(&self.depth_cents) && (0.0..=100.0).contains(&self.fade_in)
            && (0.0..=100.0).contains(&self.fade_out) && self.fade_in + self.fade_out <= 100.0
            && (0.0..=100.0).contains(&self.shift) && (-100.0..=100.0).contains(&self.drift)
            && (-100.0..=100.0).contains(&self.volume_link);
        if !valid { return Err(ImportError::at(path, "vibrato parameters are outside the source expression ranges")); }
        Ok(self)
    }
    fn evaluate(self, fraction: f64, duration_ms: f64) -> (f64, f64) {
        if self.length == 0.0 { return (0.0, 0.0); }
        let start = 1.0 - self.length / 100.0;
        if fraction < start { return (0.0, 0.0); }
        let fade_in = self.length * self.fade_in / 10000.0;
        let fade_out = self.length * self.fade_out / 10000.0;
        let envelope = if fade_in > 0.0 && fraction < start + fade_in { (fraction - start) / fade_in }
            else if fade_out > 0.0 && fraction > 1.0 - fade_out { (1.0 - fraction) / fade_out } else { 1.0 };
        let cycles = (fraction - start) * duration_ms / self.period_ms;
        let cents = ((2.0 * PI * (cycles + self.shift / 100.0)).sin() * self.depth_cents
            + self.depth_cents / 100.0 * self.drift) * envelope;
        let volume_shift = self.shift / 100.0 + if self.volume_link < 0.0 { 0.5 } else { 0.0 };
        let reduction = (-(2.0 * PI * (cycles + volume_shift)).sin() / 2.0 + 0.3)
            * self.volume_link.abs() / 100.0 * envelope;
        (cents, 20.0 * (1.0 - reduction).log10())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurveKind { Pitch, Dynamics }
pub(crate) struct PartCurve {
    pub part: usize,
    pub start: i64,
    pub kind: CurveKind,
    pub xs: Vec<i64>,
    pub ys: Vec<f64>,
    pub default: f64,
}
impl PartCurve {
    fn sample(&self, tick: f64) -> f64 {
        let i = self.xs.partition_point(|x| (*x as f64) < tick);
        if i < self.xs.len() && self.xs[i] as f64 == tick { return self.ys[i]; }
        if i == 0 || i == self.xs.len() { return self.default; }
        let f = (tick - self.xs[i - 1] as f64) / (self.xs[i] as f64 - self.xs[i - 1] as f64);
        (self.ys[i - 1] + (self.ys[i] - self.ys[i - 1]) * f).round_ties_even()
    }
}

pub(crate) fn track_gain(db: f64, path: &str) -> Result<i32> {
    let millidb = (db * 1000.0).round();
    if !millidb.is_finite() || !(-96000.0..=12000.0).contains(&millidb) {
        return Err(ImportError::at(path, "track gain exceeds Verdant's -96..12 dB range"));
    }
    Ok(millidb as i32)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply(note: &mut Note, control: &NoteControl, curves: &[PartCurve], source_start: i64,
    part: usize, ppq: i64, tempo: &TempoMap, previous: Option<(i64, i32, f64)>,
    id: &str, report: &mut ImportReport) -> Result<()> {
    let end = note.start_tick.checked_add(note.duration_tick).ok_or_else(|| ImportError::at(id, "destination note end overflows ticks"))?;
    let begin_seconds = tempo.seconds_at(note.start_tick as f64);
    let duration_ms = (tempo.seconds_at(end as f64) - begin_seconds) * 1000.0;
    let relevant = curves.iter().filter(|c| c.part == part);
    let active_vibrato = control.vibrato.filter(|v| v.length > 0.0);
    if control.pitch.is_none() && active_vibrato.is_none() && control.intensity_percent.is_none()
        && control.tuning_cents == 0.0 && relevant.clone().next().is_none() { return Ok(()); }
    let intensity_db = match control.intensity_percent {
        Some(p) if p > 0.0 => 20.0 * (p / 100.0).log10(),
        Some(_) => return Err(ImportError::at(id, "zero intensity cannot be represented by Verdant's finite gain curve; explicit rest decision required")),
        None => 0.0,
    };
    let mut bend = Vec::new();
    let mut sample_ticks = BTreeSet::from([0, note.duration_tick]);
    if note.duration_tick / SAMPLE_TICKS > 1_000_000 {
        return Err(ImportError::at(id, "expressive note requires more than one million control samples; split it at authored boundaries"));
    }
    let mut tick = SAMPLE_TICKS;
    while tick < note.duration_tick { sample_ticks.insert(tick); tick = tick.saturating_add(SAMPLE_TICKS); }
    if let Some(points) = &control.pitch {
        for (i, p) in points.iter().enumerate() {
            let x = tempo.tick_at(begin_seconds + p.ms / 1000.0) - note.start_tick as f64;
            let y = if i == 0 && control.snap_first {
                previous.map_or(0.0, |(_, midi, tuning)| (midi - note.midi_note) as f64 * 100.0 + tuning - control.tuning_cents)
            } else { p.cents };
            if !x.is_finite() { return Err(ImportError::at(id, "pitch point time is not representable")); }
            if (0.0..=note.duration_tick as f64).contains(&x) { sample_ticks.insert(x.round() as i64); }
            bend.push((x, y, p.shape));
        }
        if bend.first().is_some_and(|p| p.0 < 0.0) || bend.last().is_some_and(|p| p.0 > note.duration_tick as f64) {
            report.notices.push(ImportNotice { code: "pitch_outside_note".into(), location: id.into(),
                message: "Pitch within the source note is translated. Preutter/neighbor regions outside its notated boundaries remain in the retained source curve and are not executed.".into() });
        }
    }
    for curve in relevant.clone() {
        for x in &curve.xs {
            let relative = ((*x as f64 + curve.start as f64 - source_start as f64) * PPQ as f64 / ppq as f64).round();
            if (0.0..=note.duration_tick as f64).contains(&relative) {
                let t = relative as i64;
                sample_ticks.insert(t);
                // UCurve returns the descriptor default immediately outside
                // its support; explicit neighbors prevent held endpoint tails.
                if t > 0 { sample_ticks.insert(t - 1); }
                if t < note.duration_tick { sample_ticks.insert(t + 1); }
            }
        }
    }
    for point in tempo.points() {
        let relative = point.tick - note.start_tick;
        if relative > 0 && relative < note.duration_tick { sample_ticks.insert(relative); }
    }
    let mut pitches = Vec::new();
    let mut gains = Vec::new();
    for t in sample_ticks {
        let mut cents = control.tuning_cents + sample_bend(&bend, t as f64);
        let (vibrato, volume) = active_vibrato.map_or((0.0, 0.0), |v| v.evaluate(t as f64 / note.duration_tick as f64, duration_ms));
        cents += vibrato;
        let mut db = intensity_db + volume;
        for curve in relevant.clone() {
            let source_tick = source_start as f64 - curve.start as f64 + t as f64 * ppq as f64 / PPQ as f64;
            match curve.kind { CurveKind::Pitch => cents += curve.sample(source_tick), CurveKind::Dynamics => db += curve.sample(source_tick) / 10.0 }
        }
        let pitch = (cents * 1000.0).round();
        let gain = (db * 1000.0).round();
        if !pitch.is_finite() || !(-2400000.0..=2400000.0).contains(&pitch) {
            return Err(ImportError::at(id, "combined source pitch exceeds Verdant's +/-2400 cent curve range"));
        }
        if !gain.is_finite() || !(-60000.0..=12000.0).contains(&gain) {
            return Err(ImportError::at(id, "combined source dynamics exceeds Verdant's -60..12 dB note curve range"));
        }
        push_pitch(&mut pitches, PitchPoint { id: PointId(format!("{}:pitch:{t}", note.id)), tick: t,
            millicents: pitch as i64, unknown: BTreeMap::new() });
        push_gain(&mut gains, GainPoint { id: PointId(format!("{}:gain:{t}", note.id)), tick: t,
            millidecibels: gain as i64, unknown: BTreeMap::new() });
    }
    if pitches.iter().any(|p| p.millicents != 0) { note.curves.pitch_millicent_points = pitches; }
    if gains.iter().any(|p| p.millidecibels != 0) { note.curves.gain_millidecibel_points = gains; }
    report.notices.push(ImportNotice { code: "expression_sampling".into(), location: id.into(),
        message: format!("Source bends, vibrato and dynamics translated into editable domain controls at <= {SAMPLE_TICKS} project ticks; curve knots rounded to nearest tick, values to millicents/millidecibels. Raw source parameters retained; continuous renderer equivalence is not claimed.") });
    Ok(())
}
fn sample_bend(points: &[(f64, f64, Shape)], tick: f64) -> f64 {
    let Some(first) = points.first() else { return 0.0 };
    if tick < first.0 { return first.1; }
    let i = points.partition_point(|p| p.0 <= tick);
    if i == points.len() { return points[i - 1].1; }
    let a = points[i - 1]; let b = points[i];
    a.1 + (b.1 - a.1) * a.2.fraction((tick - a.0) / (b.0 - a.0))
}
fn push_pitch(points: &mut Vec<PitchPoint>, point: PitchPoint) {
    let n = points.len();
    if n >= 2 && points[n - 2].millicents == point.millicents && points[n - 1].millicents == point.millicents { points.pop(); }
    points.push(point);
}
fn push_gain(points: &mut Vec<GainPoint>, point: GainPoint) {
    let n = points.len();
    if n >= 2 && points[n - 2].millidecibels == point.millidecibels && points[n - 1].millidecibels == point.millidecibels { points.pop(); }
    points.push(point);
}

pub(crate) fn record_control(fields: &mut Vec<crate::FieldReport>, path: &str, raw: Value, translated: bool, target: &str) {
    field(fields, path, raw, if translated { D::Translated } else { D::RetainedUnsupported },
        translated.then_some(target), if translated { "Source expression retained and translated to domain controls; see expression_sampling notice for quantization." }
        else { "Unsupported expression semantics retained exactly, without substitution." });
}
