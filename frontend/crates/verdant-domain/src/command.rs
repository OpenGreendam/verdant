//! Domain commands (SPEC §5.2, §6). Every user intent becomes one
//! [`CommandBatch`]; the store applies a batch atomically or not at all.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{NoteId, TrackId};
use crate::model::{ExpressionPoint, GainPoint, Note, PitchPoint, Pronunciation, Track, TrackHeader, Vibrato};

pub const MIDI_MIN: i32 = 0;
pub const MIDI_MAX: i32 = 127;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Start,
    End,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum Command {
    /// Group move with one shared delta (never per-note rounding).
    MoveNotes {
        track: TrackId,
        note_ids: Vec<NoteId>,
        delta_tick: i64,
        delta_semitones: i32,
    },
    /// Head/tail resize; minimum length 1 tick, never flips.
    ResizeNotes {
        track: TrackId,
        note_ids: Vec<NoteId>,
        edge: Edge,
        delta_tick: i64,
    },
    /// Alt-drag copy. New IDs are allocated by the gesture at commit time so a
    /// cancelled copy leaves nothing behind (T03).
    DuplicateNotes {
        track: TrackId,
        source_ids: Vec<NoteId>,
        new_ids: Vec<NoteId>,
        delta_tick: i64,
        delta_semitones: i32,
    },
    InsertNotes {
        track: TrackId,
        notes: Vec<Note>,
    },
    DeleteNotes {
        track: TrackId,
        note_ids: Vec<NoteId>,
    },
    SetLyric {
        track: TrackId,
        note_id: NoteId,
        text: String,
    },
    SetPronunciation {
        track: TrackId,
        note_id: NoteId,
        pronunciation: Pronunciation,
    },
    SetPitch {
        track: TrackId,
        note_id: NoteId,
        midi_note: i32,
        detune_millicents: i32,
    },
    SetTiming {
        track: TrackId,
        note_id: NoteId,
        start_tick: i64,
        duration_tick: i64,
    },
    /// Replace existing notes without changing their identities or discarding
    /// pronunciation/curve/extension data. Used by atomic structural edits.
    ReplaceNotes { track: TrackId, notes: Vec<Note> },
    SetTempoEvent { tick: i64, micro_bpm: Option<u64> },
    SetTimeSignatureEvent { tick: i64, numerator: Option<u32>, denominator: u32 },
    MoveTrack { track: TrackId, index: usize },
    SetTrackData { track: TrackId, key: String, value: Option<Value> },
    SetProjectExtension { key: String, value: Option<Value> },
    /// Legacy serialized journal command: replace the complete tempo map.
    /// New editor controls use SetTempoEvent and preserve other events.
    SetTempo {
        micro_bpm: u64,
    },
    SetRoute {
        route: String,
    },
    /// Update the audible synthesis profile as one undoable document edit.
    SetSynthesis {
        settings: Option<crate::SynthesisSettings>,
    },
    /// Change only C1 post-processing; Native settings and seeds are untouched.
    SetVocoderHighband {
        settings: Option<crate::VocoderHighbandSettings>,
    },
    /// Transform the vocal-tract envelope without changing note pitch or seeds.
    SetVocalTract {
        settings: Option<crate::VocalTractSettings>,
    },
    /// Change vowel dynamics without enabling synthesis settings or changing seeds.
    SetModelDynamics {
        settings: Option<crate::ModelDynamicsSettings>,
    },
    SetPhonemeTiming {
        settings: Option<crate::PhonemeTimingSettings>,
    },
    SetConsonantDynamics {
        settings: Option<crate::ConsonantDynamicsSettings>,
    },
    SetOutputTransients {
        settings: Option<crate::OutputTransientsSettings>,
    },
    AddTrack {
        index: usize,
        track: Track,
    },
    RemoveTrack {
        track: TrackId,
    },
    /// Replace a track's header (name, colour, fader, pan, mute, solo, route).
    SetTrackHeader {
        track: TrackId,
        header: TrackHeader,
    },
    /// Replace one note's pitch-bend curve (note-relative ticks, millicents).
    SetPitchCurve {
        track: TrackId,
        note_id: NoteId,
        points: Vec<PitchPoint>,
    },
    /// Replace one note's dynamics curve (note-relative ticks, millidecibels).
    SetGainCurve {
        track: TrackId,
        note_id: NoteId,
        points: Vec<GainPoint>,
    },
    SetExpressionCurve {
        track: TrackId,
        note_id: NoteId,
        parameter: String,
        points: Vec<ExpressionPoint>,
    },
    SetVibrato {
        track: TrackId,
        note_ids: Vec<NoteId>,
        vibrato: Option<Vibrato>,
    },
    SetPortamento {
        track: TrackId,
        note_ids: Vec<NoteId>,
        portamento_ms: Option<u32>,
    },
    /// Replace a track's plugin-insert list (`track.unknown["inserts"]`,
    /// PLUGIN-HOST.md). Elements keep the documented
    /// {"format","id","name","version","state_b64","bypass"} shape plus unknown
    /// keys verbatim; an empty list removes the key. One undoable transaction.
    SetTrackInserts {
        track: TrackId,
        inserts: Vec<Value>,
    },
    /// Make a track an instrument track (`Some` plug-in spec, stored in
    /// `track.unknown["instrument"]`) or a vocal track again (`None`). An
    /// instrument track plays its notes through the instrument instead of a
    /// singing voice. One undoable transaction.
    SetTrackInstrument {
        track: TrackId,
        instrument: Option<Value>,
    },
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Command::MoveNotes { .. } => "MoveNotes",
            Command::ResizeNotes { .. } => "ResizeNotes",
            Command::DuplicateNotes { .. } => "DuplicateNotes",
            Command::InsertNotes { .. } => "InsertNotes",
            Command::DeleteNotes { .. } => "DeleteNotes",
            Command::SetLyric { .. } => "SetLyric",
            Command::SetPronunciation { .. } => "SetPronunciation",
            Command::SetPitch { .. } => "SetPitch",
            Command::SetTiming { .. } => "SetTiming",
            Command::ReplaceNotes { .. } => "ReplaceNotes",
            Command::SetTempoEvent { .. } => "SetTempoEvent",
            Command::SetTimeSignatureEvent { .. } => "SetTimeSignatureEvent",
            Command::MoveTrack { .. } => "MoveTrack",
            Command::SetTrackData { .. } => "SetTrackData",
            Command::SetProjectExtension { .. } => "SetProjectExtension",
            Command::SetTempo { .. } => "SetTempo",
            Command::SetRoute { .. } => "SetRoute",
            Command::SetSynthesis { .. } => "SetSynthesis",
            Command::SetVocoderHighband { .. } => "SetVocoderHighband",
            Command::SetVocalTract { .. } => "SetVocalTract",
            Command::SetModelDynamics { .. } => "SetModelDynamics",
            Command::SetPhonemeTiming { .. } => "SetPhonemeTiming",
            Command::SetConsonantDynamics { .. } => "SetConsonantDynamics",
            Command::SetOutputTransients { .. } => "SetOutputTransients",
            Command::AddTrack { .. } => "AddTrack",
            Command::RemoveTrack { .. } => "RemoveTrack",
            Command::SetTrackHeader { .. } => "SetTrackHeader",
            Command::SetPitchCurve { .. } => "SetPitchCurve",
            Command::SetGainCurve { .. } => "SetGainCurve",
            Command::SetExpressionCurve { .. } => "SetExpressionCurve",
            Command::SetVibrato { .. } => "SetVibrato",
            Command::SetPortamento { .. } => "SetPortamento",
            Command::SetTrackInserts { .. } => "SetTrackInserts",
            Command::SetTrackInstrument { .. } => "SetTrackInstrument",
        }
    }
}

/// One user intent = one batch = one revision = one undo entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandBatch {
    pub label: String,
    pub commands: Vec<Command>,
}

impl CommandBatch {
    pub fn single(label: impl Into<String>, command: Command) -> Self {
        CommandBatch { label: label.into(), commands: vec![command] }
    }
}

/// A committed edit on the current history branch. Its original revision is
/// stable across undo/redo, including edits with identical labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryItem<'a> {
    pub revision: crate::Revision,
    pub label: &'a str,
}

/// Transaction order on the surviving branch, including the redo tail.
/// Position zero is the document before the first edit; position `n` is the
/// state after `entries[n - 1]`. Navigation never creates another edit entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History<'a> {
    pub entries: Vec<HistoryItem<'a>>,
    pub position: usize,
}

impl crate::ProjectStore {
    /// Reconstruct the branch from the durable commit log, rather than listing
    /// undo/redo operations as if they were new user edits. Checkpoints retain
    /// this log, so labels and positions also survive journal compaction.
    pub fn history(&self) -> History<'_> {
        use crate::CommitKind;
        let mut entries = Vec::with_capacity(self.undo_depth() + self.redo_depth());
        let mut position = 0;
        for commit in self.log() {
            match commit.kind {
                CommitKind::Edit => {
                    entries.truncate(position);
                    entries.push(HistoryItem { revision: commit.revision, label: &commit.label });
                    position += 1;
                }
                CommitKind::Undo => position -= 1,
                CommitKind::Redo => position += 1,
            }
        }
        History { entries, position }
    }

    pub(crate) fn history_jump_steps(
        &self,
        base: crate::Revision,
        position: usize,
    ) -> Result<(crate::CommitKind, usize), crate::DomainError> {
        if base != self.revision() {
            return Err(crate::DomainError::StaleBase { base, current: self.revision() });
        }
        let current = self.undo_depth();
        if position > current + self.redo_depth() {
            return Err(crate::DomainError::InvalidCommand("history position is outside the current branch".into()));
        }
        if position < current {
            Ok((crate::CommitKind::Undo, current - position))
        } else {
            Ok((crate::CommitKind::Redo, position - current))
        }
    }

    /// Navigate to a position in [`Self::history`] using the original patches.
    /// Each traversed transaction keeps its own monotonically increasing
    /// revision. A stale/out-of-range target changes nothing. If a replay
    /// fails, preceding successful steps remain applied and can be navigated
    /// normally. Durable editors must use `ProjectPersistence::jump_to_history`.
    pub fn jump_to_history(
        &mut self,
        base: crate::Revision,
        position: usize,
    ) -> Result<Vec<crate::Commit>, crate::DomainError> {
        let (kind, steps) = self.history_jump_steps(base, position)?;
        let mut commits = Vec::with_capacity(steps);
        for _ in 0..steps {
            let commit = match kind {
                crate::CommitKind::Undo => self.undo()?,
                _ => self.redo()?,
            };
            commits.push(commit.expect("validated history position has a transaction"));
        }
        Ok(commits)
    }
}

/// Clamp a group move so the whole group stays inside `start ≥ 0` and the MIDI
/// range while keeping its shape (SPEC §5.2). Shared by the gesture preview and
/// the reducer so preview and commit can never disagree.
pub fn clamp_group_move<'a>(notes: impl IntoIterator<Item = &'a Note>, delta_tick: i64, delta_semitones: i32) -> (i64, i32) {
    let mut min_start = i64::MAX;
    let mut min_pitch = i32::MAX;
    let mut max_pitch = i32::MIN;
    for n in notes {
        min_start = min_start.min(n.start_tick);
        min_pitch = min_pitch.min(n.midi_note);
        max_pitch = max_pitch.max(n.midi_note);
    }
    if min_start == i64::MAX {
        return (0, 0);
    }
    let dt = delta_tick.max(-min_start);
    let ds = delta_semitones.clamp(MIDI_MIN - min_pitch, MIDI_MAX - max_pitch);
    (dt, ds)
}

/// Resize one note, preserving the opposite edge, min length 1, start ≥ 0.
pub fn resized(note: &Note, edge: Edge, delta_tick: i64) -> (i64, i64) {
    match edge {
        Edge::End => (note.start_tick, (note.duration_tick + delta_tick).max(1)),
        Edge::Start => {
            let end = note.end_tick();
            let start = (note.start_tick + delta_tick).clamp(0, end - 1);
            (start, end - start)
        }
    }
}

/// Scale note-relative curve ticks when a note's length changes (SPEC §8:
/// attached points stretch proportionally by default).
pub fn scale_curve_ticks(n: &mut Note, old_duration: i64) {
    if old_duration <= 0 || old_duration == n.duration_tick {
        return;
    }
    let f = |t: i64| ((t as i128 * n.duration_tick as i128) / old_duration as i128) as i64;
    let mut pitch = std::mem::take(&mut n.curves.pitch_millicent_points);
    for p in &mut pitch {
        p.tick = f(p.tick);
    }
    n.curves.pitch_millicent_points = normalize_points(pitch, |p| p.tick);
    let mut gain = std::mem::take(&mut n.curves.gain_millidecibel_points);
    for p in &mut gain {
        p.tick = f(p.tick);
    }
    n.curves.gain_millidecibel_points = normalize_points(gain, |p| p.tick);
    for points in n.curves.expression_points.values_mut() {
        let mut scaled = std::mem::take(points);
        for point in &mut scaled { point.tick = f(point.tick); }
        *points = normalize_points(scaled, |p| p.tick);
    }
}

/// Sort by tick; for equal ticks the later point replaces the earlier one
/// (SPEC §8 MVP rule).
pub fn normalize_points<P>(mut pts: Vec<P>, tick: impl Fn(&P) -> i64) -> Vec<P> {
    pts.sort_by_key(|p| tick(p));
    let mut out: Vec<P> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_some_and(|q| tick(q) == tick(&p)) {
            out.pop();
        }
        out.push(p);
    }
    out
}
