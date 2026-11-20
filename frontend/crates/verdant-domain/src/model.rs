//! `greendam-project` v0.1 data model (SPEC §4).
//!
//! Every struct keeps unknown keys in a flattened map so that a file written by
//! a newer tool round-trips without silent loss (SPEC §4.3: "未知允许扩展字段原样保留").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ids::{NoteId, PointId, TrackId};
use crate::interval::IntervalIndex;

pub const FORMAT: &str = "greendam-project";
pub const FORMAT_VERSION: &str = "0.1";
/// Project pulses per quarter note (SPEC §4.1).
pub const PPQ: i64 = 960;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Project {
    pub format: String,
    pub version: String,
    #[serde(default)]
    pub status: Option<String>,
    pub project_id: String,
    /// Revision recorded at the last save. The live revision is owned by
    /// [`crate::store::ProjectStore`].
    pub revision: u64,
    pub ppq: i64,
    pub tempo_map: Vec<TempoPoint>,
    pub time_signatures: Vec<TimeSignature>,
    #[serde(default)]
    pub voice_refs: BTreeMap<String, VoiceRef>,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub render_preferences: RenderPreferences,
    #[serde(default)]
    pub extensions: Map<String, Value>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Serialize)]
#[serde(rename = "Project")]
pub(crate) struct ProjectDocument<'a> {
    format: &'a str,
    version: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<&'a str>,
    project_id: &'a str,
    pub revision: u64,
    ppq: i64,
    tempo_map: &'a [TempoPoint],
    time_signatures: &'a [TimeSignature],
    voice_refs: &'a BTreeMap<String, VoiceRef>,
    tracks: &'a [Track],
    render_preferences: &'a RenderPreferences,
    extensions: &'a Map<String, Value>,
    #[serde(flatten)]
    unknown: &'a Map<String, Value>,
}

impl Project {
    pub(crate) fn document<'a>(&'a self, revision: u64, status: Option<&'a str>) -> ProjectDocument<'a> {
        let Project {
            format, version, status: _, project_id, revision: _, ppq, tempo_map,
            time_signatures, voice_refs, tracks, render_preferences, extensions, unknown,
        } = self;
        ProjectDocument {
            format, version, status, project_id, revision, ppq: *ppq, tempo_map,
            time_signatures, voice_refs, tracks, render_preferences, extensions, unknown,
        }
    }
}

impl Serialize for Project {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.document(self.revision, self.status.as_deref()).serialize(serializer)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempoPoint {
    pub tick: i64,
    /// Beats per minute × 10⁶; strictly positive.
    pub micro_bpm: u64,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub tick: i64,
    pub numerator: u32,
    pub denominator: u32,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceRef {
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vid: Option<u64>,
    /// True only when the historical identity was confirmed by the project
    /// owner (README boundary). sn/VID 10008 = 「童声01」 confirmed 2026-11-13.
    #[serde(default)]
    pub historical_identity_confirmed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_sha256: Option<String>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_ref: Option<String>,
    #[serde(default = "yes")]
    pub monophonic: bool,
    /// Kept sorted by `(start_tick, id)` by every mutation in this crate.
    /// Directly mutated outside a command? Call [`Track::refresh_index`]
    /// afterwards — range queries trust this order and the interval bounds.
    pub notes: Vec<Note>,
    /// Subtree end/pitch bounds maintained by `sort_notes` / `refresh_index`.
    /// Derived state: never serialized, rebuilt on load.
    #[serde(skip)]
    interval_index: IntervalIndex,
    /// Display colour from the fixed track palette (see `TRACK_COLORS`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Channel fader, millidecibels (0 = unity).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub gain_millidecibels: i32,
    /// Constant-power pan, −1000 (left) … 1000 (right).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pan_permille: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub muted: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub solo: bool,
    /// Per-track renderer route; falls back to `render_preferences.route`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_route: Option<String>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

fn yes() -> bool {
    true
}
fn is_zero(v: &i32) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}

/// Track palette, named rather than hex so themes can map each name to a
/// light- and a dark-appearance value.
pub const TRACK_COLORS: &[&str] = &["teal", "blue", "orange", "purple", "rose", "yellow", "graphite"];

/// Everything about a track except its notes: the unit of mixer edits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackHeader {
    pub name: String,
    pub voice_ref: Option<String>,
    pub monophonic: bool,
    pub color: Option<String>,
    pub gain_millidecibels: i32,
    pub pan_permille: i32,
    pub muted: bool,
    pub solo: bool,
    pub render_route: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub start_tick: i64,
    pub duration_tick: i64,
    pub midi_note: i32,
    #[serde(default)]
    pub detune_millicents: i32,
    #[serde(default)]
    pub lyric: String,
    #[serde(default)]
    pub pronunciation: Pronunciation,
    #[serde(default)]
    pub curves: NoteCurves,
    /// Per-note vibrato (an F0 modulation; renderable only where the route
    /// accepts a control track).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato: Option<Vibrato>,
    /// None: bounded automatic connection (80 ms upper limit).
    /// Some(0): disable the incoming connection. Some(ms > 0): authored
    /// glide time into this note from an adjacent previous note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portamento_ms: Option<u32>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vibrato {
    /// Peak deviation in cents (0..=200).
    pub depth_cents: u32,
    /// Rate in millihertz (1000..=12000).
    pub rate_millihertz: u32,
    /// Onset as a fraction of the note length, ‰ (0..=1000).
    pub delay_permille: u32,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

impl Vibrato {
    pub const DEFAULT: Vibrato = Vibrato {
        depth_cents: 35, rate_millihertz: 5500, delay_permille: 350, unknown: BTreeMap::new(),
    };

    pub fn is_valid(&self) -> bool {
        self.depth_cents <= 200 && (1000..=12000).contains(&self.rate_millihertz) && self.delay_permille <= 1000
    }
}

/// Inspector units: percent, cents, Hz, degrees, milliseconds, percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteModulationParameter {
    VibratoStart, VibratoDepth, VibratoRate, VibratoPhase, PortamentoLength, PortamentoCurve,
}

impl NoteModulationParameter {
    pub fn range(self) -> (f64, f64) {
        match self {
            Self::VibratoStart => (0.0, 100.0),
            Self::VibratoDepth => (0.0, 200.0),
            Self::VibratoRate => (1.0, 12.0),
            Self::VibratoPhase => (0.0, 360.0),
            Self::PortamentoLength => (0.0, 2000.0),
            Self::PortamentoCurve => (-100.0, 100.0),
        }
    }
}

/// Musical starting points, independent of the selected singer or route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VibratoPreset { Gentle, Natural, Wide }

impl VibratoPreset {
    pub const ALL: [Self; 3] = [Self::Gentle, Self::Natural, Self::Wide];

    pub fn label(self) -> &'static str {
        match self { Self::Gentle => "轻柔颤音", Self::Natural => "自然颤音", Self::Wide => "宽幅颤音" }
    }

    pub fn apply(self, vibrato: &mut Vibrato) {
        let (depth, rate, start) = match self {
            Self::Gentle => (20, 5000, 500),
            Self::Natural => (35, 5500, 350),
            Self::Wide => (70, 6000, 250),
        };
        vibrato.depth_cents = depth;
        vibrato.rate_millihertz = rate;
        vibrato.delay_permille = start;
        vibrato.set_fades_phase(150, 150, 0);
    }
}

impl Note {
    /// Signed curvature of the incoming glide. Zero preserves its historical
    /// smoothstep; negative values start slowly, positive values start quickly.
    /// Kept in the extension map so existing note literals and old files retain
    /// their representation when this control has never been authored.
    pub fn portamento_curve_percent(&self) -> i32 {
        self.unknown.get("portamento_curve_percent").and_then(Value::as_i64).unwrap_or(0) as i32
    }

    /// Warp a glide's unit interval before its smoothstep interpolation.
    /// For c in [-1, 1], x + c*x*(1-x) is monotone and fixes both endpoints.
    /// An unauthored/zero curve returns x exactly, preserving existing audio.
    pub fn portamento_curve_position(&self, position: f64) -> f64 {
        let x = position.clamp(0.0, 1.0);
        let curve = self.portamento_curve_percent() as f64 / 100.0;
        x + curve * x * (1.0 - x)
    }

    pub fn modulation_value(&self, parameter: NoteModulationParameter) -> f64 {
        use NoteModulationParameter as P;
        let default = Vibrato::DEFAULT;
        let v = self.vibrato.as_ref().unwrap_or(&default);
        match parameter {
            P::VibratoStart => v.delay_permille as f64 / 10.0,
            P::VibratoDepth => self.vibrato.as_ref().map_or(0, |v| v.depth_cents) as f64,
            P::VibratoRate => v.rate_millihertz as f64 / 1000.0,
            P::VibratoPhase => v.phase_millidegrees() as f64 / 1000.0,
            P::PortamentoLength => self.portamento_ms.unwrap_or(80) as f64,
            P::PortamentoCurve => self.portamento_curve_percent() as f64,
        }
    }

    /// Reject before mutation, including NaN/infinity and values outside the
    /// inspector's declared range. Other controls and extension data survive.
    pub fn set_modulation_value(&mut self, parameter: NoteModulationParameter, value: f64) -> Result<(), &'static str> {
        let (min, max) = parameter.range();
        if !value.is_finite() || !(min..=max).contains(&value) { return Err("颤音或滑音数值超出允许范围"); }
        use NoteModulationParameter as P;
        match parameter {
            P::PortamentoLength => self.portamento_ms = Some(value.round() as u32),
            P::PortamentoCurve => {
                if value.round() == 0.0 { self.unknown.remove("portamento_curve_percent"); }
                else { self.unknown.insert("portamento_curve_percent".into(), Value::from(value.round() as i32)); }
            }
            _ => {
                let v = self.vibrato.get_or_insert_with(|| Vibrato::DEFAULT);
                match parameter {
                    P::VibratoStart => v.delay_permille = (value * 10.0).round() as u32,
                    P::VibratoDepth => v.depth_cents = value.round() as u32,
                    P::VibratoRate => v.rate_millihertz = (value * 1000.0).round() as u32,
                    P::VibratoPhase => { v.unknown.insert("phase_millidegrees".into(), Value::from(((value * 1000.0).round() as i64).rem_euclid(360_000))); }
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }

    /// Exclusive end, saturated at the representable tick boundaries.
    pub fn end_tick(&self) -> i64 {
        self.start_tick.saturating_add(self.duration_tick)
    }

    /// A fresh note with an empty, explicitly unset pronunciation.
    pub fn new(start_tick: i64, duration_tick: i64, midi_note: i32) -> Self {
        Note {
            id: NoteId::generate(),
            start_tick,
            duration_tick,
            midi_note,
            detune_millicents: 0,
            lyric: String::new(),
            pronunciation: Pronunciation::default(),
            curves: NoteCurves::default(),
            vibrato: None,
            portamento_ms: None,
            unknown: Map::new(),
        }
    }

    /// True when the note carries any F0 modulation beyond its base pitch.
    pub fn has_f0_modulation(&self) -> bool {
        !self.curves.pitch_millicent_points.is_empty()
            || !self.curves.expression("POR").is_empty()
            || self.vibrato.as_ref().is_some_and(|v| v.depth_cents > 0)
            || self.portamento_ms.is_some_and(|m| m > 0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PronunciationSource {
    /// Nothing has been decided; the renderer must refuse.
    #[default]
    Unset,
    Manual,
    /// A declared finite mapping (recorded in `frontend_profile`).
    ExplicitMap,
    Imported,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pronunciation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_text: Option<String>,
    #[serde(default)]
    pub pinyin: String,
    #[serde(default)]
    pub phonemes: Vec<String>,
    /// Explicit phonetic input. When absent, `phonemes` retains its legacy
    /// pinyin timing-label meaning. Symbols themselves are language invariant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabet: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_approximation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stress: Option<u8>,
    /// Optional Japanese mora weights, parallel to explicit IPA groups. A
    /// kana compiler supplies these; manual native-segment durations override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mora_weights: Option<Vec<u32>>,
    /// Inspectable source-reading compilation; canonical phonemes remain the
    /// authoritative editable form. Unknown provenance keys round-trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_provenance: Option<Value>,
    #[serde(default)]
    pub source: PronunciationSource,
    #[serde(default)]
    pub frontend_profile: String,
    /// Optional durations as ‰ of note length. For explicit phonetic input
    /// these address resolved native segments (one symbol may expand to two).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phoneme_durations_permille: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NoteCurves {
    #[serde(default)]
    pub pitch_millicent_points: Vec<PitchPoint>,
    #[serde(default)]
    pub gain_millidecibel_points: Vec<GainPoint>,
    /// VOCALOID-style 0..127 controls, keyed by BRE/BRI/CLE/GEN/OPE/GWL/POR.
    /// Empty maps retain the exact sound/serialization of older projects.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expression_points: BTreeMap<String, Vec<ExpressionPoint>>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

impl NoteCurves {
    pub fn is_empty(&self) -> bool {
        self.pitch_millicent_points.is_empty() && self.gain_millidecibel_points.is_empty() && self.expression_points.values().all(Vec::is_empty)
    }

    pub fn expression(&self, parameter: &str) -> &[ExpressionPoint] {
        self.expression_points.get(parameter).map(Vec::as_slice).unwrap_or(&[])
    }
}

pub fn expression_default(parameter: &str) -> Option<u8> {
    match parameter { "BRI" | "GEN" | "POR" => Some(64), "OPE" => Some(127), "BRE" | "CLE" | "GWL" => Some(0), _ => None }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionPoint {
    pub id: PointId,
    pub tick: i64,
    pub value: u8,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

/// Note-relative tick, value relative to the note's base pitch (SPEC §8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PitchPoint {
    pub id: PointId,
    pub tick: i64,
    pub millicents: i64,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GainPoint {
    pub id: PointId,
    pub tick: i64,
    pub millidecibels: i64,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderPreferences {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default = "default_route")]
    pub route: String,
    #[serde(default = "default_seed")]
    pub native_seed: u32,
    /// Explicit sound settings; absence preserves legacy rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<crate::SynthesisSettings>,
    /// Independent post-vocoder protection. Does not opt a legacy project
    /// into Native synthesis settings or change its implicit vocoder seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vocoder_highband: Option<crate::VocoderHighbandSettings>,
    /// Independent envelope transform; never changes the authored note pitch
    /// or opts a legacy document into explicit synthesis settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vocal_tract: Option<crate::VocalTractSettings>,
    /// Opt-in vowel dynamics; absence preserves the existing sound and seed policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_dynamics: Option<crate::ModelDynamicsSettings>,
    /// Optional vowel-onset anchoring; older projects retain their exact timing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phoneme_timing: Option<crate::PhonemeTimingSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consonant_dynamics: Option<crate::ConsonantDynamicsSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_transients: Option<crate::OutputTransientsSettings>,
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

fn default_protocol() -> String {
    "ogd-worker/1".into()
}
fn default_route() -> String {
    "verdant-voice".into()
}
fn default_seed() -> u32 {
    10008
}

impl Default for RenderPreferences {
    fn default() -> Self {
        RenderPreferences { protocol: default_protocol(), route: default_route(), native_seed: default_seed(), synthesis: None, vocoder_highband: None, vocal_tract: None, model_dynamics: None, phoneme_timing: None, consonant_dynamics: None, output_transients: None, unknown: Map::new() }
    }
}

impl Project {
    /// A new empty project at 120 BPM, 4/4, with one monophonic track bound to
    /// sn / VID 10008, confirmed by the owner as 「童声01」（楠楠）.
    pub fn new_empty() -> Self {
        let mut voice_refs = BTreeMap::new();
        voice_refs.insert(
            "voice-sn".to_string(),
            VoiceRef {
                name: "sn".into(),
                display_name: "童声01（楠楠）".into(),
                vid: Some(10008),
                historical_identity_confirmed: true,
                token_sha256: None,
                model_sha256: None,
                unknown: Map::new(),
            },
        );
        Project {
            format: FORMAT.into(),
            version: FORMAT_VERSION.into(),
            status: None,
            project_id: uuid::Uuid::new_v4().to_string(),
            revision: 0,
            ppq: PPQ,
            tempo_map: vec![TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: BTreeMap::new() }],
            time_signatures: vec![TimeSignature { tick: 0, numerator: 4, denominator: 4, unknown: BTreeMap::new() }],
            voice_refs,
            tracks: vec![Track::new("主声部", "teal")],
            render_preferences: RenderPreferences::default(),
            extensions: Map::new(),
            unknown: Map::new(),
        }
    }

    pub fn track(&self, id: &TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| &t.id == id)
    }

    pub fn track_mut(&mut self, id: &TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| &t.id == id)
    }

    pub fn note_count(&self) -> usize {
        self.tracks.iter().map(|t| t.notes.len()).sum()
    }
}

impl Track {
    pub fn new(name: impl Into<String>, color: &str) -> Self {
        Track {
            id: TrackId::generate(),
            name: name.into(),
            voice_ref: Some("voice-sn".into()),
            monophonic: true,
            notes: Vec::new(),
            interval_index: IntervalIndex::default(),
            color: Some(color.into()),
            gain_millidecibels: 0,
            pan_permille: 0,
            muted: false,
            solo: false,
            render_route: None,
            unknown: Map::new(),
        }
    }

    pub fn header(&self) -> TrackHeader {
        TrackHeader {
            name: self.name.clone(),
            voice_ref: self.voice_ref.clone(),
            monophonic: self.monophonic,
            color: self.color.clone(),
            gain_millidecibels: self.gain_millidecibels,
            pan_permille: self.pan_permille,
            muted: self.muted,
            solo: self.solo,
            render_route: self.render_route.clone(),
        }
    }

    pub fn set_header(&mut self, h: &TrackHeader) {
        self.name = h.name.clone();
        self.voice_ref = h.voice_ref.clone();
        self.monophonic = h.monophonic;
        self.color = h.color.clone();
        self.gain_millidecibels = h.gain_millidecibels;
        self.pan_permille = h.pan_permille;
        self.muted = h.muted;
        self.solo = h.solo;
        self.render_route = h.render_route.clone();
    }

    pub fn note(&self, id: &NoteId) -> Option<&Note> {
        self.notes.iter().find(|n| &n.id == id)
    }

    pub fn note_index(&self, id: &NoteId) -> Option<usize> {
        self.notes.iter().position(|n| &n.id == id)
    }

    pub(crate) fn sort_notes(&mut self) {
        self.notes.sort_by(|a, b| a.start_tick.cmp(&b.start_tick).then_with(|| a.id.cmp(&b.id)));
        self.interval_index.rebuild(&self.notes);
    }

    /// Re-sort and rebuild the interval bounds after notes were mutated
    /// outside a command (file import, fixtures, generators).
    pub fn refresh_index(&mut self) {
        self.sort_notes();
    }

    /// Notes with `start < to` and `end > from`, in `(start_tick, id)` order.
    /// Subtrees ending at/before `from` or starting at/after `to` are pruned;
    /// an early long overlap does not widen a scan over unrelated short notes.
    /// The iterator is allocation-free and can also traverse in reverse.
    pub fn notes_in_range(&self, from: i64, to: i64) -> impl DoubleEndedIterator<Item = &Note> + Clone {
        self.notes_in_pitch_range(from, to, i32::MIN, i32::MAX)
    }

    /// The same time boundaries as [`Track::notes_in_range`], restricted to
    /// inclusive MIDI rows `low..=high`. Nonintersecting pitch subtrees are
    /// pruned before examining their notes; `low > high` yields no notes.
    pub fn notes_in_pitch_range(&self, from: i64, to: i64, low: i32, high: i32) -> impl DoubleEndedIterator<Item = &Note> + Clone {
        self.interval_index.query(&self.notes, from, to, low, high)
    }

    /// Pairs of time-overlapping notes; a monophonic track renders only when
    /// this is empty (SPEC §5.2: diagnose, never silently truncate).
    pub fn overlaps(&self) -> Vec<(NoteId, NoteId)> {
        let mut out = Vec::new();
        let mut max_end: Option<(i64, &NoteId)> = None;
        for n in self.notes.iter().filter(|n| !n.is_muted()) {
            if let Some((end, id)) = max_end {
                if n.start_tick < end {
                    out.push((id.clone(), n.id.clone()));
                }
            }
            if max_end.is_none_or(|(end, _)| n.end_tick() > end) {
                max_end = Some((n.end_tick(), &n.id));
            }
        }
        out
    }
}
