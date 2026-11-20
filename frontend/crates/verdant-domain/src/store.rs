//! `ProjectStore`: the single writer of project facts (SPEC §2, §6).
//!
//! * A batch applies atomically against an explicit base revision.
//! * Each effective batch yields exactly one new revision and one undo entry.
//! * A batch with no effective change creates neither.
//! * Undo/redo run through the same reducer and produce *new* monotonically
//!   increasing revisions; revisions never move backwards.
//! * Snapshots are immutable `Arc<Project>`s that render jobs and the UI may
//!   hold while editing continues.

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::command::{Command, CommandBatch, MIDI_MAX, MIDI_MIN, clamp_group_move, normalize_points, resized, scale_curve_ticks};
use crate::ids::{NoteId, Revision, TrackId};
use crate::model::{Note, Project, ProjectDocument, TempoPoint, Track, TrackHeader};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("batch was prepared against {base} but the project is at {current}")]
    StaleBase { base: Revision, current: Revision },
    #[error("unknown track {0}")]
    UnknownTrack(TrackId),
    #[error("unknown note {0}")]
    UnknownNote(NoteId),
    #[error("note id {0} already exists")]
    DuplicateNoteId(NoteId),
    #[error("invalid note {id}: {reason}")]
    InvalidNote { id: NoteId, reason: String },
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("revision space exhausted")]
    RevisionOverflow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NotePatch {
    pub track: TrackId,
    pub id: NoteId,
    pub before: Option<Note>,
    pub after: Option<Note>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MetaPatch {
    Tempo {
        before: Vec<TempoPoint>,
        after: Vec<TempoPoint>,
    },
    Meter { before: Vec<crate::model::TimeSignature>, after: Vec<crate::model::TimeSignature> },
    ProjectExtension { key: String, before: Option<serde_json::Value>, after: Option<serde_json::Value> },
    Route {
        before: String,
        after: String,
    },
    Synthesis {
        before: Option<crate::SynthesisSettings>,
        after: Option<crate::SynthesisSettings>,
    },
    VocoderHighband {
        before: Option<crate::VocoderHighbandSettings>,
        after: Option<crate::VocoderHighbandSettings>,
    },
    VocalTract {
        before: Option<crate::VocalTractSettings>,
        after: Option<crate::VocalTractSettings>,
    },
    ModelDynamics {
        before: Option<crate::ModelDynamicsSettings>,
        after: Option<crate::ModelDynamicsSettings>,
    },
    PhonemeTiming {
        before: Option<crate::PhonemeTimingSettings>,
        after: Option<crate::PhonemeTimingSettings>,
    },
    ConsonantDynamics {
        before: Option<crate::ConsonantDynamicsSettings>,
        after: Option<crate::ConsonantDynamicsSettings>,
    },
    OutputTransients {
        before: Option<crate::OutputTransientsSettings>,
        after: Option<crate::OutputTransientsSettings>,
    },
    TrackHeader {
        track: TrackId,
        before: TrackHeader,
        after: TrackHeader,
    },
    /// A whole track appears (`before: None`) or disappears (`after: None`).
    TrackSet {
        index: usize,
        before: Option<Track>,
        after: Option<Track>,
    },
    /// One extension key on a track's unknown map changed (`None` = absent).
    /// Generic so documented plugin-insert objects and future unknown keys
    /// preserve their JSON values on round-trip (SPEC §4.3).
    TrackUnknownKey {
        track: TrackId,
        key: String,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub label: String,
    pub notes: Vec<NotePatch>,
    pub meta: Vec<MetaPatch>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitKind {
    Edit,
    Undo,
    Redo,
}

/// What a successful commit changed; consumers (render scheduler, canvas
/// cache) invalidate by `tick_range` and `changed_notes`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub revision: Revision,
    pub kind: CommitKind,
    pub label: String,
    pub commands: Vec<Command>,
    pub changed_notes: Vec<NoteId>,
    pub tick_range: Option<(i64, i64)>,
    pub meta_changed: bool,
}

#[derive(Debug)]
pub struct ProjectStore {
    project: Arc<Project>,
    revision: Revision,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    saved_digest: Option<String>,
    digest_cache: std::cell::RefCell<Option<(Revision, String)>>,
    /// Every commit, in order. Used by contract traces ("exactly one
    /// command") and by the history rail in the UI.
    log: Vec<Commit>,
}

/// Recovery snapshot. Private patches and history stay inside the domain;
/// callers never reconstruct them to save or recover a project.
#[derive(Debug, Deserialize)]
pub(crate) struct StoreCheckpoint {
    pub project: Project,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    log: Vec<Commit>,
}

#[derive(Serialize)]
pub(crate) struct StoreCheckpointRef<'a> {
    pub project: ProjectDocument<'a>,
    undo: &'a [HistoryEntry],
    redo: &'a [HistoryEntry],
    log: &'a [Commit],
}

impl ProjectStore {
    pub fn new(mut project: Project) -> Self {
        for t in &mut project.tracks {
            t.sort_notes();
        }
        let revision = Revision::new(project.revision).unwrap_or(Revision::new(0).unwrap());
        let mut store = ProjectStore {
            project: Arc::new(project),
            revision,
            undo: Vec::new(),
            redo: Vec::new(),
            saved_digest: None,
            digest_cache: Default::default(),
            log: Vec::new(),
        };
        store.saved_digest = Some(store.content_digest());
        store
    }

    pub fn snapshot(&self) -> Arc<Project> {
        self.project.clone()
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Validate an entire loaded snapshot without applying a synthetic edit,
    /// allocating note patches or changing its revision/history.
    pub fn validate_document(&self) -> Result<(), DomainError> {
        let project = &self.project;
        if project.format != crate::model::FORMAT || !project.version.starts_with("0.")
            || project.project_id.is_empty() || Revision::new(project.revision).is_none()
            || !(1..=Revision::MAX as i64 / 4).contains(&project.ppq) {
            return Err(DomainError::InvalidCommand("invalid native format, version, identity, revision or PPQ".into()));
        }
        let preferences = &project.render_preferences;
        for result in [
            preferences.synthesis.as_ref().map(|settings| settings.validate()),
            preferences.vocoder_highband.as_ref().map(|settings| settings.validate()),
            preferences.vocal_tract.as_ref().map(|settings| settings.validate()),
            preferences.model_dynamics.as_ref().map(|settings| settings.validate()),
            preferences.phoneme_timing.as_ref().map(|settings| settings.validate()),
            preferences.consonant_dynamics.as_ref().map(|settings| settings.validate()),
            preferences.output_transients.as_ref().map(|settings| settings.validate()),
        ].into_iter().flatten() {
            result.map_err(DomainError::InvalidCommand)?;
        }
        validate(&self.project, &[], &[], true)?;
        crate::parts::Parts::load(&self.project).and_then(|parts| parts.validate(&self.project)).map_err(DomainError::InvalidCommand)
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    pub fn log(&self) -> &[Commit] {
        &self.log
    }

    /// Patches still reachable by undo or redo on the surviving branch.
    /// Consumers collecting portable asset references can inspect both sides
    /// without cloning checkpoints or performing synthetic history edits.
    pub fn retained_history(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.undo.iter().chain(&self.redo)
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }

    pub(crate) fn checkpoint(&self) -> StoreCheckpointRef<'_> {
        StoreCheckpointRef {
            project: self.project.document(self.revision.get(), self.project.status.as_deref()),
            undo: &self.undo,
            redo: &self.redo,
            log: &self.log,
        }
    }

    pub(crate) fn from_checkpoint(checkpoint: StoreCheckpoint) -> Self {
        let mut store = Self::new(checkpoint.project);
        store.undo = checkpoint.undo;
        store.redo = checkpoint.redo;
        store.log = checkpoint.log;
        for entry in store.undo.iter_mut().chain(&mut store.redo) {
            refresh_entry_indexes(entry);
        }
        for commit in &mut store.log {
            refresh_commit_indexes(commit);
        }
        store
    }

    pub(crate) fn set_saved_digest(&mut self, digest: String) {
        self.saved_digest = Some(digest);
    }

    /// Content digest of the musical document (excludes the stored revision
    /// and status). Dirty = digest differs from the last saved digest.
    pub fn content_digest(&self) -> String {
        if let Some((rev, d)) = self.digest_cache.borrow().as_ref() {
            if *rev == self.revision {
                return d.clone();
            }
        }
        let d = crate::digest::project_content_digest(&self.project);
        *self.digest_cache.borrow_mut() = Some((self.revision, d.clone()));
        d
    }

    pub fn is_dirty(&self) -> bool {
        self.saved_digest.as_deref() != Some(self.content_digest().as_str())
    }

    /// Project ready to be written: the live revision is stamped in.
    pub fn project_for_save(&self) -> Project {
        let mut p = (*self.project).clone();
        p.revision = self.revision.get();
        p
    }

    pub fn mark_saved(&mut self) {
        self.saved_digest = Some(self.content_digest());
    }

    /// Apply a batch prepared against `base`. Returns `Ok(None)` for a no-op
    /// (no revision, no undo entry).
    pub fn apply(&mut self, base: Revision, batch: CommandBatch) -> Result<Option<Commit>, DomainError> {
        self.apply_with(base, batch, |_, _| Ok::<_, DomainError>(()))
    }

    /// Build and validate a candidate, persist its transaction, then publish
    /// it. A failed write leaves revision, snapshots and both stacks intact.
    pub(crate) fn apply_with<E: From<DomainError>>(
        &mut self,
        base: Revision,
        batch: CommandBatch,
        persist: impl FnOnce(&HistoryEntry, &Commit) -> Result<(), E>,
    ) -> Result<Option<Commit>, E> {
        if base != self.revision {
            return Err(DomainError::StaleBase { base, current: self.revision }.into());
        }
        let next_rev = self.revision.next().ok_or(DomainError::RevisionOverflow)?;
        let mut work = (*self.project).clone();
        let mut notes = Vec::new();
        let mut meta = Vec::new();
        for command in &batch.commands {
            apply_command(&mut work, command, &mut notes, &mut meta)?;
        }
        crate::parts::synchronize(&self.project, &mut work, &batch.commands, &mut notes, &mut meta).map_err(DomainError::InvalidCommand)?;
        for track in &mut work.tracks {
            track.sort_notes();
        }
        let (notes, meta) = normalize(&self.project, &work, notes, meta);
        if notes.is_empty() && meta.is_empty() {
            return Ok(None);
        }
        validate(&work, &notes, &meta, false)?;
        let entry = HistoryEntry { label: batch.label.clone(), notes, meta };
        let commit = make_commit(next_rev, CommitKind::Edit, batch.label, batch.commands, &entry);
        persist(&entry, &commit)?;
        self.project = Arc::new(work);
        self.revision = next_rev;
        self.undo.push(entry);
        self.redo.clear();
        self.log.push(commit.clone());
        Ok(Some(commit))
    }

    pub fn undo(&mut self) -> Result<Option<Commit>, DomainError> {
        self.undo_with(|_, _| Ok::<_, DomainError>(()))
    }

    pub(crate) fn undo_with<E: From<DomainError>>(
        &mut self,
        persist: impl FnOnce(&HistoryEntry, &Commit) -> Result<(), E>,
    ) -> Result<Option<Commit>, E> {
        let Some(entry) = self.undo.pop() else { return Ok(None) };
        match self.replay_with(&entry, CommitKind::Undo, persist) {
            Ok(commit) => {
                self.redo.push(entry);
                Ok(Some(commit))
            }
            Err(error) => {
                self.undo.push(entry);
                Err(error)
            }
        }
    }

    pub fn redo(&mut self) -> Result<Option<Commit>, DomainError> {
        self.redo_with(|_, _| Ok::<_, DomainError>(()))
    }

    pub(crate) fn redo_with<E: From<DomainError>>(
        &mut self,
        persist: impl FnOnce(&HistoryEntry, &Commit) -> Result<(), E>,
    ) -> Result<Option<Commit>, E> {
        let Some(entry) = self.redo.pop() else { return Ok(None) };
        match self.replay_with(&entry, CommitKind::Redo, persist) {
            Ok(commit) => {
                self.undo.push(entry);
                Ok(Some(commit))
            }
            Err(error) => {
                self.redo.push(entry);
                Err(error)
            }
        }
    }

    /// Re-run the reducer, compare the recorded patches/commit, then publish.
    /// Recovery cannot manufacture a missing undo item or bypass validation.
    pub(crate) fn restore(&mut self, entry: &HistoryEntry, recorded: &Commit) -> Result<(), DomainError> {
        let verify = |actual_entry: &HistoryEntry, actual_commit: &Commit| {
            if actual_entry != entry || actual_commit != recorded {
                Err(DomainError::InvalidCommand("journal transaction differs from reducer result".into()))
            } else {
                Ok(())
            }
        };
        let result = match recorded.kind {
            CommitKind::Edit => self.apply_with(
                self.revision,
                CommandBatch { label: entry.label.clone(), commands: recorded.commands.clone() },
                verify,
            )?,
            CommitKind::Undo => self.undo_with(verify)?,
            CommitKind::Redo => self.redo_with(verify)?,
        };
        result.ok_or_else(|| DomainError::InvalidCommand("journal record does not change the project".into()))?;
        Ok(())
    }

    fn replay_with<E: From<DomainError>>(
        &mut self,
        entry: &HistoryEntry,
        kind: CommitKind,
        persist: impl FnOnce(&HistoryEntry, &Commit) -> Result<(), E>,
    ) -> Result<Commit, E> {
        let next_rev = self.revision.next().ok_or(DomainError::RevisionOverflow)?;
        let backwards = kind == CommitKind::Undo;
        let mut work = (*self.project).clone();
        let mut touched = BTreeSet::new();
        for p in &entry.notes {
            let target = if backwards { &p.before } else { &p.after };
            let track = work.track_mut(&p.track).ok_or_else(|| DomainError::UnknownTrack(p.track.clone()))?;
            if let Some(i) = track.note_index(&p.id) {
                track.notes.remove(i);
            }
            if let Some(n) = target {
                track.notes.push(n.clone());
            }
            touched.insert(p.track.clone());
        }
        for m in &entry.meta {
            match m {
                MetaPatch::Tempo { before, after } => work.tempo_map = if backwards { before.clone() } else { after.clone() },
                MetaPatch::Meter { before, after } => work.time_signatures = if backwards { before.clone() } else { after.clone() },
                MetaPatch::ProjectExtension { key, before, after } => {
                    match if backwards { before } else { after } {
                        Some(value) => { work.extensions.insert(key.clone(), value.clone()); }
                        None => { work.extensions.remove(key); }
                    }
                }
                MetaPatch::Route { before, after } => {
                    work.render_preferences.route = if backwards { before.clone() } else { after.clone() }
                }
                MetaPatch::Synthesis { before, after } => {
                    work.render_preferences.synthesis = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::VocoderHighband { before, after } => {
                    work.render_preferences.vocoder_highband = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::VocalTract { before, after } => {
                    work.render_preferences.vocal_tract = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::ModelDynamics { before, after } => {
                    work.render_preferences.model_dynamics = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::PhonemeTiming { before, after } => {
                    work.render_preferences.phoneme_timing = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::ConsonantDynamics { before, after } => {
                    work.render_preferences.consonant_dynamics = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::OutputTransients { before, after } => {
                    work.render_preferences.output_transients = if backwards { before.clone() } else { after.clone() };
                }
                MetaPatch::TrackHeader { track, before, after } => {
                    let t = work.track_mut(track).ok_or_else(|| DomainError::UnknownTrack(track.clone()))?;
                    t.set_header(if backwards { before } else { after });
                }
                MetaPatch::TrackUnknownKey { track, key, before, after } => {
                    let t = work.track_mut(track).ok_or_else(|| DomainError::UnknownTrack(track.clone()))?;
                    let v = if backwards { before } else { after };
                    match v {
                        Some(v) => {
                            t.unknown.insert(key.clone(), v.clone());
                        }
                        None => {
                            t.unknown.remove(key);
                        }
                    }
                }
                MetaPatch::TrackSet { .. } => {}
            }
        }
        // Track existence changes replay in reverse order when undoing.
        let sets: Vec<&MetaPatch> = entry.meta.iter().filter(|m| matches!(m, MetaPatch::TrackSet { .. })).collect();
        let ordered: Vec<&MetaPatch> = if backwards { sets.into_iter().rev().collect() } else { sets };
        for m in ordered {
            if let MetaPatch::TrackSet { index, before, after } = m {
                let (from, to) = if backwards { (after, before) } else { (before, after) };
                if let Some(t) = from {
                    work.tracks.retain(|x| x.id != t.id);
                }
                if let Some(t) = to {
                    let i = (*index).min(work.tracks.len());
                    work.tracks.insert(i, t.clone());
                }
            }
        }
        for t in work.tracks.iter_mut().filter(|t| touched.contains(&t.id)) {
            t.sort_notes();
        }
        let label = if backwards {
            format!("撤销 {}", entry.label)
        } else if kind == CommitKind::Redo {
            format!("重做 {}", entry.label)
        } else {
            entry.label.clone()
        };
        let commit = make_commit(next_rev, kind, label, Vec::new(), entry);
        persist(entry, &commit)?;
        self.project = Arc::new(work);
        self.revision = next_rev;
        self.log.push(commit.clone());
        Ok(commit)
    }
}

pub(crate) fn refresh_entry_indexes(entry: &mut HistoryEntry) {
    for patch in &mut entry.meta {
        if let MetaPatch::TrackSet { before, after, .. } = patch {
            for track in before.iter_mut().chain(after) {
                track.refresh_index();
            }
        }
    }
}

pub(crate) fn refresh_commit_indexes(commit: &mut Commit) {
    for command in &mut commit.commands {
        if let Command::AddTrack { track, .. } = command {
            track.refresh_index();
        }
    }
}

fn make_commit(revision: Revision, kind: CommitKind, label: String, commands: Vec<Command>, e: &HistoryEntry) -> Commit {
    let mut range: Option<(i64, i64)> = None;
    for p in &e.notes {
        for n in [&p.before, &p.after].into_iter().flatten() {
            range = Some(match range {
                None => (n.start_tick, n.end_tick()),
                Some((a, b)) => (a.min(n.start_tick), b.max(n.end_tick())),
            });
        }
    }
    Commit {
        revision,
        kind,
        label,
        commands,
        changed_notes: e.notes.iter().map(|p| p.id.clone()).collect(),
        tick_range: range,
        meta_changed: !e.meta.is_empty(),
    }
}

/// Merge multiple patches for the same note (first `before`, last `after`)
/// and drop patches whose before == after.
fn normalize(before_project: &Project, after_project: &Project, notes: Vec<NotePatch>, meta: Vec<MetaPatch>) -> (Vec<NotePatch>, Vec<MetaPatch>) {
    // A track added/removed/replaced within the batch owns its complete final
    // contents. Do not also journal note/header patches for that same track:
    // their intermediate objects may no longer exist when undoing the batch.
    let mut set_ids = BTreeSet::new();
    for patch in &meta {
        if let MetaPatch::TrackSet { before, after, .. } = patch {
            for track in before.iter().chain(after) {
                set_ids.insert(track.id.clone());
            }
        }
    }
    let touched_set_ids = set_ids.clone();
    set_ids.retain(|id| {
        let before = before_project.tracks.iter().enumerate().find(|(_, track)| &track.id == id);
        let after = after_project.tracks.iter().enumerate().find(|(_, track)| &track.id == id);
        before != after
    });
    let mut merged: Vec<NotePatch> = Vec::new();
    for p in notes {
        if touched_set_ids.contains(&p.track) {
            continue;
        }
        if let Some(m) = merged.iter_mut().find(|m| m.id == p.id && m.track == p.track) {
            m.after = p.after;
        } else {
            merged.push(p);
        }
    }
    merged.retain(|p| p.before != p.after);
    let mut merged_meta: Vec<MetaPatch> = Vec::new();
    for patch in meta {
        match patch {
            MetaPatch::TrackSet { .. } => {},
            MetaPatch::TrackHeader { ref track, .. } | MetaPatch::TrackUnknownKey { ref track, .. }
                if touched_set_ids.contains(track) => {},
            MetaPatch::Tempo { before, after } => {
                if let Some(MetaPatch::Tempo { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::Tempo { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::Tempo { before, after });
                }
            },
            MetaPatch::Meter { before, after } => {
                if let Some(MetaPatch::Meter { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::Meter { .. })) { *previous = after; }
                else { merged_meta.push(MetaPatch::Meter { before, after }); }
            },
            MetaPatch::ProjectExtension { key, before, after } => {
                if let Some(MetaPatch::ProjectExtension { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::ProjectExtension { key: k, .. } if k == &key)) { *previous = after; }
                else { merged_meta.push(MetaPatch::ProjectExtension { key, before, after }); }
            },
            MetaPatch::Route { before, after } => {
                if let Some(MetaPatch::Route { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::Route { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::Route { before, after });
                }
            },
            MetaPatch::Synthesis { before, after } => {
                if let Some(MetaPatch::Synthesis { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::Synthesis { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::Synthesis { before, after });
                }
            }
            MetaPatch::VocoderHighband { before, after } => {
                if let Some(MetaPatch::VocoderHighband { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::VocoderHighband { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::VocoderHighband { before, after });
                }
            }
            MetaPatch::VocalTract { before, after } => {
                if let Some(MetaPatch::VocalTract { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::VocalTract { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::VocalTract { before, after });
                }
            }
            MetaPatch::ModelDynamics { before, after } => {
                if let Some(MetaPatch::ModelDynamics { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::ModelDynamics { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::ModelDynamics { before, after });
                }
            }
            MetaPatch::PhonemeTiming { before, after } => {
                if let Some(MetaPatch::PhonemeTiming { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::PhonemeTiming { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::PhonemeTiming { before, after });
                }
            }
            MetaPatch::ConsonantDynamics { before, after } => {
                if let Some(MetaPatch::ConsonantDynamics { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::ConsonantDynamics { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::ConsonantDynamics { before, after });
                }
            }
            MetaPatch::OutputTransients { before, after } => {
                if let Some(MetaPatch::OutputTransients { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::OutputTransients { .. })) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::OutputTransients { before, after });
                }
            }
            MetaPatch::TrackHeader { track, before, after } => {
                if let Some(MetaPatch::TrackHeader { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::TrackHeader { track: t, .. } if *t == track)) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::TrackHeader { track, before, after });
                }
            },
            MetaPatch::TrackUnknownKey { track, key, before, after } => {
                if let Some(MetaPatch::TrackUnknownKey { after: previous, .. }) = merged_meta.iter_mut().find(|m| matches!(m, MetaPatch::TrackUnknownKey { track: t, key: k, .. } if *t == track && *k == key)) {
                    *previous = after;
                } else {
                    merged_meta.push(MetaPatch::TrackUnknownKey { track, key, before, after });
                }
            },
        }
    }
    // Remove in descending index order, insert in ascending order; reversing
    // this list restores both track contents and ordering exactly.
    for (index, track) in before_project.tracks.iter().enumerate().rev().filter(|(_, t)| set_ids.contains(&t.id)) {
        merged_meta.push(MetaPatch::TrackSet { index, before: Some(track.clone()), after: None });
    }
    for (index, track) in after_project.tracks.iter().enumerate().filter(|(_, t)| set_ids.contains(&t.id)) {
        let mut track = track.clone();
        track.refresh_index();
        merged_meta.push(MetaPatch::TrackSet { index, before: None, after: Some(track) });
    }
    let meta = merged_meta
        .into_iter()
        .filter(|m| match m {
            MetaPatch::Tempo { before, after } => before != after,
            MetaPatch::Meter { before, after } => before != after,
            MetaPatch::ProjectExtension { before, after, .. } => before != after,
            MetaPatch::Route { before, after } => before != after,
            MetaPatch::Synthesis { before, after } => before != after,
            MetaPatch::VocoderHighband { before, after } => before != after,
            MetaPatch::VocalTract { before, after } => before != after,
            MetaPatch::ModelDynamics { before, after } => before != after,
            MetaPatch::PhonemeTiming { before, after } => before != after,
            MetaPatch::ConsonantDynamics { before, after } => before != after,
            MetaPatch::OutputTransients { before, after } => before != after,
            MetaPatch::TrackHeader { before, after, .. } => before != after,
            MetaPatch::TrackUnknownKey { before, after, .. } => before != after,
            MetaPatch::TrackSet { before, after, .. } => before != after,
        })
        .collect();
    (merged, meta)
}

fn validate(p: &Project, patches: &[NotePatch], meta: &[MetaPatch], all_notes: bool) -> Result<(), DomainError> {
    if p.tracks.is_empty() {
        return Err(DomainError::InvalidCommand("a project keeps at least one track".into()));
    }
    crate::automation::AutomationDocument::from_project(p).map_err(|e| DomainError::InvalidCommand(e.to_string()))?;
    crate::TempoMap::new(&p.tempo_map, p.ppq).map_err(|e| DomainError::InvalidCommand(e.to_string()))?;
    crate::MeterMap::new(&p.time_signatures, p.ppq).map_err(DomainError::InvalidCommand)?;
    let mut track_ids = HashSet::new();
    for t in &p.tracks {
        if t.id.as_str().is_empty() || !track_ids.insert(&t.id) {
            return Err(DomainError::InvalidCommand(format!("duplicate track id {}", t.id)));
        }
        if !(-1000..=1000).contains(&t.pan_permille) || !(-96_000..=12_000).contains(&t.gain_millidecibels) {
            return Err(DomainError::InvalidCommand(format!("track {} mix values out of range", t.id)));
        }
    }
    let mut seen = HashSet::new();
    for t in &p.tracks {
        for n in &t.notes {
            if n.id.as_str().is_empty() || !seen.insert(&n.id) {
                return Err(DomainError::DuplicateNoteId(n.id.clone()));
            }
        }
    }
    let changed_notes = patches.iter().filter_map(|patch| patch.after.as_ref());
    let added_track_notes = meta.iter().filter_map(|patch| match patch {
        MetaPatch::TrackSet { after: Some(track), .. } => Some(track.notes.iter()),
        _ => None,
    }).flatten();
    let loaded_notes = all_notes.then_some(p).into_iter().flat_map(|p| p.tracks.iter()).flat_map(|t| &t.notes);
    for n in changed_notes.chain(added_track_notes).chain(loaded_notes) {
            let bad = |reason: &str| DomainError::InvalidNote { id: n.id.clone(), reason: reason.into() };
            crate::editing::validate_note(n).map_err(|reason| DomainError::InvalidNote { id: n.id.clone(), reason })?;
            if n.start_tick < 0 {
                return Err(bad("start_tick < 0"));
            }
            if n.duration_tick < 1 {
                return Err(bad("duration_tick < 1"));
            }
            if n.start_tick.checked_add(n.duration_tick).is_none_or(|end| end > Revision::MAX as i64) {
                return Err(bad("note end exceeds the exact native tick range"));
            }
            if !(MIDI_MIN..=MIDI_MAX).contains(&n.midi_note) {
                return Err(bad("midi_note outside 0..=127"));
            }
            if n.vibrato.as_ref().is_some_and(|v| !v.is_valid()) {
                return Err(bad("vibrato out of range"));
            }
            if n.portamento_ms.is_some_and(|m| m > 2000) {
                return Err(bad("portamento longer than 2 s"));
            }
            if let Some(durations) = &n.pronunciation.phoneme_durations_permille {
                if (n.pronunciation.alphabet.is_none() && durations.len() != n.pronunciation.phonemes.len())
                    || durations.is_empty() || durations.len() > 512
                    || durations.iter().any(|duration| *duration == 0)
                    || durations.iter().map(|duration| u64::from(*duration)).sum::<u64>() != 1000
                {
                    return Err(bad("phoneme durations must be positive, sum to 1000, and match the resolved segment count at render preflight"));
                }
            }
            if n.pronunciation.stress.is_some_and(|stress| stress > 2) {
                return Err(bad("phonetic stress must be 0, 1 or 2"));
            }
            if let Some(weights) = &n.pronunciation.mora_weights {
                if weights.is_empty() || weights.len() > 512 || weights.contains(&0)
                    || n.pronunciation.language.as_deref() != Some("ja") || n.pronunciation.alphabet.is_none() {
                    return Err(bad("mora weights require explicit Japanese phonetic input and positive group weights"));
                }
            }
            let in_note = |t: i64| (0..=n.duration_tick).contains(&t);
            if !n.curves.pitch_millicent_points.iter().all(|q| in_note(q.tick) && q.millicents.unsigned_abs() <= 2_400_000)
                || !n.curves.gain_millidecibel_points.iter().all(|q| in_note(q.tick) && (-60_000..=12_000).contains(&q.millidecibels))
                || n.curves.pitch_millicent_points.windows(2).any(|points| points[0].tick >= points[1].tick)
                || n.curves.gain_millidecibel_points.windows(2).any(|points| points[0].tick >= points[1].tick)
            {
                return Err(bad("curve points require ordered ticks inside the note and valid values"));
            }
            for (parameter, points) in &n.curves.expression_points {
                if crate::model::expression_default(parameter).is_none() || points.len() > 4096
                    || points.iter().any(|q| !in_note(q.tick) || q.value > 127)
                    || points.windows(2).any(|p| p[0].tick >= p[1].tick) {
                    return Err(bad("expression curve requires a known controller, ordered note-relative ticks and values 0..127"));
                }
            }
    }
    Ok(())
}

fn track_of<'a>(p: &'a mut Project, id: &TrackId) -> Result<&'a mut crate::model::Track, DomainError> {
    p.track_mut(id).ok_or_else(|| DomainError::UnknownTrack(id.clone()))
}

fn note_mut<'a>(t: &'a mut crate::model::Track, id: &NoteId) -> Result<&'a mut Note, DomainError> {
    t.notes.iter_mut().find(|n| &n.id == id).ok_or_else(|| DomainError::UnknownNote(id.clone()))
}

fn edit_note(
    p: &mut Project,
    track: &TrackId,
    id: &NoteId,
    out: &mut Vec<NotePatch>,
    f: impl FnOnce(&mut Note),
) -> Result<(), DomainError> {
    let t = track_of(p, track)?;
    let n = note_mut(t, id)?;
    let before = n.clone();
    f(n);
    out.push(NotePatch { track: track.clone(), id: id.clone(), before: Some(before), after: Some(n.clone()) });
    Ok(())
}

fn apply_command(p: &mut Project, command: &Command, out: &mut Vec<NotePatch>, meta: &mut Vec<MetaPatch>) -> Result<(), DomainError> {
    match command {
        Command::MoveNotes { track, note_ids, delta_tick, delta_semitones } => {
            let t = track_of(p, track)?;
            let group: Vec<&Note> =
                note_ids.iter().map(|id| t.note(id).ok_or_else(|| DomainError::UnknownNote(id.clone()))).collect::<Result<_, _>>()?;
            let (dt, ds) = clamp_group_move(group.iter().copied(), *delta_tick, *delta_semitones);
            if let Some(note) = group.iter().find(|note| note.start_tick.checked_add(dt)
                .and_then(|start| start.checked_add(note.duration_tick)).is_none_or(|end| end > Revision::MAX as i64)) {
                return Err(DomainError::InvalidNote { id: note.id.clone(), reason: "group move exceeds the exact native tick range".into() });
            }
            for id in note_ids {
                edit_note(p, track, id, out, |n| {
                    n.start_tick += dt;
                    n.midi_note += ds;
                })?;
            }
        }
        Command::ResizeNotes { track, note_ids, edge, delta_tick } => {
            for id in note_ids {
                let note = p.track(track).ok_or_else(|| DomainError::UnknownTrack(track.clone()))?
                    .note(id).ok_or_else(|| DomainError::UnknownNote(id.clone()))?;
                let addition = match edge {
                    crate::Edge::Start => note.start_tick.checked_add(*delta_tick),
                    crate::Edge::End => note.duration_tick.checked_add(*delta_tick),
                };
                if addition.is_none() {
                    return Err(DomainError::InvalidNote { id: id.clone(), reason: "resize arithmetic exceeds the native tick range".into() });
                }
                let (s, d) = resized(note, *edge, *delta_tick);
                if s.checked_add(d).is_none_or(|end| end > Revision::MAX as i64) {
                    return Err(DomainError::InvalidNote { id: id.clone(), reason: "resize exceeds the exact native tick range".into() });
                }
                edit_note(p, track, id, out, |n| {
                    let old = n.duration_tick;
                    n.start_tick = s;
                    n.duration_tick = d;
                    scale_curve_ticks(n, old);
                })?;
            }
        }
        Command::DuplicateNotes { track, source_ids, new_ids, delta_tick, delta_semitones } => {
            if source_ids.len() != new_ids.len() {
                return Err(DomainError::InvalidCommand("source_ids and new_ids differ in length".into()));
            }
            let t = track_of(p, track)?;
            let group: Vec<&Note> =
                source_ids.iter().map(|id| t.note(id).ok_or_else(|| DomainError::UnknownNote(id.clone()))).collect::<Result<_, _>>()?;
            let (dt, ds) = clamp_group_move(group.iter().copied(), *delta_tick, *delta_semitones);
            if let Some(note) = group.iter().find(|note| note.start_tick.checked_add(dt)
                .and_then(|start| start.checked_add(note.duration_tick)).is_none_or(|end| end > Revision::MAX as i64)) {
                return Err(DomainError::InvalidNote { id: note.id.clone(), reason: "duplicate move exceeds the exact native tick range".into() });
            }
            let copies: Vec<Note> = group
                .iter()
                .zip(new_ids)
                .map(|(src, new_id)| {
                    let mut n = (*src).clone();
                    n.id = new_id.clone();
                    n.start_tick += dt;
                    n.midi_note += ds;
                    n
                })
                .collect();
            for n in copies {
                if t.note(&n.id).is_some() {
                    return Err(DomainError::DuplicateNoteId(n.id));
                }
                out.push(NotePatch { track: track.clone(), id: n.id.clone(), before: None, after: Some(n.clone()) });
                t.notes.push(n);
            }
        }
        Command::InsertNotes { track, notes } => {
            let t = track_of(p, track)?;
            for n in notes {
                if t.note(&n.id).is_some() {
                    return Err(DomainError::DuplicateNoteId(n.id.clone()));
                }
                out.push(NotePatch { track: track.clone(), id: n.id.clone(), before: None, after: Some(n.clone()) });
                t.notes.push(n.clone());
            }
        }
        Command::DeleteNotes { track, note_ids } => {
            let t = track_of(p, track)?;
            for id in note_ids {
                let i = t.note_index(id).ok_or_else(|| DomainError::UnknownNote(id.clone()))?;
                let n = t.notes.remove(i);
                out.push(NotePatch { track: track.clone(), id: id.clone(), before: Some(n), after: None });
            }
        }
        Command::SetLyric { track, note_id, text } => {
            edit_note(p, track, note_id, out, |n| n.lyric = text.clone())?;
        }
        Command::SetPronunciation { track, note_id, pronunciation } => {
            edit_note(p, track, note_id, out, |n| n.pronunciation = pronunciation.clone())?;
        }
        Command::SetPitch { track, note_id, midi_note, detune_millicents } => {
            edit_note(p, track, note_id, out, |n| {
                n.midi_note = *midi_note;
                n.detune_millicents = *detune_millicents;
            })?;
        }
        Command::SetTiming { track, note_id, start_tick, duration_tick } => {
            edit_note(p, track, note_id, out, |n| {
                let old_duration = n.duration_tick;
                n.start_tick = *start_tick;
                n.duration_tick = *duration_tick;
                scale_curve_ticks(n, old_duration);
            })?;
        }
        Command::ReplaceNotes { track, notes } => {
            for note in notes { edit_note(p, track, &note.id, out, |n| *n = note.clone())?; }
        }
        Command::SetTempoEvent { tick, micro_bpm } => {
            let mut tempo = crate::TempoMap::new(&p.tempo_map, p.ppq).map_err(|e| DomainError::InvalidCommand(e.to_string()))?;
            tempo.set_event(*tick, *micro_bpm).map_err(|e| DomainError::InvalidCommand(e.to_string()))?;
            let after = tempo.into_points();
            let before = std::mem::replace(&mut p.tempo_map, after.clone());
            meta.push(MetaPatch::Tempo { before, after });
        }
        Command::SetTimeSignatureEvent { tick, numerator, denominator } => {
            if *tick < 0 || (*tick == 0 && numerator.is_none()) { return Err(DomainError::InvalidCommand("invalid meter event".into())); }
            let before = p.time_signatures.clone();
            if let Some(numerator) = numerator {
                if let Some(point) = p.time_signatures.iter_mut().find(|p| p.tick == *tick) { point.numerator = *numerator; point.denominator = *denominator; }
                else { p.time_signatures.push(crate::model::TimeSignature { tick: *tick, numerator: *numerator, denominator: *denominator, unknown: Default::default() }); }
            } else { p.time_signatures.retain(|p| p.tick != *tick); }
            p.time_signatures.sort_by_key(|p| p.tick); meta.push(MetaPatch::Meter { before, after: p.time_signatures.clone() });
        }
        Command::MoveTrack { track, index } => {
            let previous = p.tracks.iter().position(|t| &t.id == track).ok_or_else(|| DomainError::UnknownTrack(track.clone()))?;
            let moved = p.tracks.remove(previous); let destination = (*index).min(p.tracks.len()); p.tracks.insert(destination, moved.clone());
            meta.push(MetaPatch::TrackSet { index: previous, before: Some(moved.clone()), after: None });
            meta.push(MetaPatch::TrackSet { index: destination, before: None, after: Some(moved) });
        }
        Command::SetTrackData { track, key, value } => {
            if key.is_empty() { return Err(DomainError::InvalidCommand("empty track extension key".into())); }
            let t = track_of(p, track)?; let before = t.unknown.get(key).cloned();
            if let Some(value) = value { t.unknown.insert(key.clone(), value.clone()); } else { t.unknown.remove(key); }
            meta.push(MetaPatch::TrackUnknownKey { track: track.clone(), key: key.clone(), before, after: value.clone() });
        }
        Command::SetProjectExtension { key, value } => {
            if key.is_empty() { return Err(DomainError::InvalidCommand("empty project extension key".into())); }
            let before = p.extensions.get(key).cloned();
            if let Some(value) = value { p.extensions.insert(key.clone(), value.clone()); } else { p.extensions.remove(key); }
            meta.push(MetaPatch::ProjectExtension { key: key.clone(), before, after: value.clone() });
        }
        Command::SetTempo { micro_bpm } => {
            if *micro_bpm == 0 || *micro_bpm > 1_000_000_000 {
                return Err(DomainError::InvalidCommand("tempo out of range".into()));
            }
            let before = p.tempo_map.clone();
            let mut point = before.iter().find(|point| point.tick == 0).cloned().unwrap_or(TempoPoint {
                tick: 0, micro_bpm: *micro_bpm, unknown: Default::default(),
            });
            point.micro_bpm = *micro_bpm;
            p.tempo_map = vec![point];
            meta.push(MetaPatch::Tempo { before, after: p.tempo_map.clone() });
        }
        Command::SetRoute { route } => {
            let before = p.render_preferences.route.clone();
            p.render_preferences.route = route.clone();
            meta.push(MetaPatch::Route { before, after: route.clone() });
        }
        Command::SetSynthesis { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.synthesis.clone();
            p.render_preferences.synthesis = settings.clone();
            meta.push(MetaPatch::Synthesis { before, after: settings.clone() });
        }
        Command::SetVocoderHighband { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.vocoder_highband.clone();
            p.render_preferences.vocoder_highband = settings.clone();
            meta.push(MetaPatch::VocoderHighband { before, after: settings.clone() });
        }
        Command::SetVocalTract { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.vocal_tract.clone();
            p.render_preferences.vocal_tract = settings.clone();
            meta.push(MetaPatch::VocalTract { before, after: settings.clone() });
        }
        Command::SetModelDynamics { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.model_dynamics.clone();
            p.render_preferences.model_dynamics = settings.clone();
            meta.push(MetaPatch::ModelDynamics { before, after: settings.clone() });
        }
        Command::SetPhonemeTiming { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.phoneme_timing.clone();
            p.render_preferences.phoneme_timing = settings.clone();
            meta.push(MetaPatch::PhonemeTiming { before, after: settings.clone() });
        }
        Command::SetConsonantDynamics { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.consonant_dynamics.clone();
            p.render_preferences.consonant_dynamics = settings.clone();
            meta.push(MetaPatch::ConsonantDynamics { before, after: settings.clone() });
        }
        Command::SetOutputTransients { settings } => {
            if let Some(settings) = settings { settings.validate().map_err(DomainError::InvalidCommand)?; }
            let before = p.render_preferences.output_transients.clone();
            p.render_preferences.output_transients = settings.clone();
            meta.push(MetaPatch::OutputTransients { before, after: settings.clone() });
        }
        Command::AddTrack { index, track } => {
            if p.track(&track.id).is_some() {
                return Err(DomainError::InvalidCommand(format!("track {} already exists", track.id)));
            }
            let i = (*index).min(p.tracks.len());
            let mut t = track.clone();
            t.sort_notes();
            p.tracks.insert(i, t.clone());
            meta.push(MetaPatch::TrackSet { index: i, before: None, after: Some(t) });
        }
        Command::RemoveTrack { track } => {
            let mut automation=crate::automation::AutomationDocument::from_project(p).map_err(|e|DomainError::InvalidCommand(e.to_string()))?;
            automation.remove_track(track);
            let key=crate::automation::AUTOMATION_KEY.to_string();let before=p.extensions.get(&key).cloned();
            let after=automation.extension_value().map_err(|e|DomainError::InvalidCommand(e.to_string()))?;
            if before!=after{if let Some(value)=&after{p.extensions.insert(key.clone(),value.clone());}else{p.extensions.remove(&key);}meta.push(MetaPatch::ProjectExtension{key,before,after});}
            let i = p.tracks.iter().position(|t| &t.id == track).ok_or_else(|| DomainError::UnknownTrack(track.clone()))?;
            let t = p.tracks.remove(i);
            meta.push(MetaPatch::TrackSet { index: i, before: Some(t), after: None });
        }
        Command::SetTrackHeader { track, header } => {
            let t = track_of(p, track)?;
            let before = t.header();
            t.set_header(header);
            meta.push(MetaPatch::TrackHeader { track: track.clone(), before, after: header.clone() });
        }
        Command::SetPitchCurve { track, note_id, points } => {
            let pts = normalize_points(points.clone(), |q| q.tick);
            edit_note(p, track, note_id, out, |n| n.curves.pitch_millicent_points = pts)?;
        }
        Command::SetGainCurve { track, note_id, points } => {
            let pts = normalize_points(points.clone(), |q| q.tick);
            edit_note(p, track, note_id, out, |n| n.curves.gain_millidecibel_points = pts)?;
        }
        Command::SetExpressionCurve { track, note_id, parameter, points } => {
            let pts = normalize_points(points.clone(), |q| q.tick);
            edit_note(p, track, note_id, out, |n| {
                if pts.is_empty() { n.curves.expression_points.remove(parameter); }
                else { n.curves.expression_points.insert(parameter.clone(), pts); }
            })?;
        }
        Command::SetVibrato { track, note_ids, vibrato } => {
            for id in note_ids {
                edit_note(p, track, id, out, |n| n.vibrato = vibrato.clone())?;
            }
        }
        Command::SetPortamento { track, note_ids, portamento_ms } => {
            for id in note_ids {
                edit_note(p, track, id, out, |n| n.portamento_ms = *portamento_ms)?;
            }
        }
        Command::SetTrackInserts { track, inserts } => {
            let mut automation=crate::automation::AutomationDocument::from_project(p).map_err(|e|DomainError::InvalidCommand(e.to_string()))?;
            automation.lanes.retain(|lane|!matches!(&lane.target,crate::automation::AutomationTarget::PluginParameter{owner:crate::automation::AutomationOwner::Track{track_id},instance_id,..}if track_id==track&&!inserts.iter().any(|insert|insert.get("instance_id").and_then(serde_json::Value::as_str)==Some(instance_id.as_str()))));
            let key=crate::automation::AUTOMATION_KEY.to_string();let before=p.extensions.get(&key).cloned();let after=automation.extension_value().map_err(|e|DomainError::InvalidCommand(e.to_string()))?;
            if before!=after{if let Some(value)=&after{p.extensions.insert(key.clone(),value.clone());}else{p.extensions.remove(&key);}meta.push(MetaPatch::ProjectExtension{key,before,after});}
            let t = track_of(p, track)?;
            let before = t.unknown.get("inserts").cloned();
            let after = if inserts.is_empty() { None } else { Some(serde_json::Value::Array(inserts.clone())) };
            match &after {
                Some(v) => {
                    t.unknown.insert("inserts".into(), v.clone());
                }
                None => {
                    t.unknown.remove("inserts");
                }
            }
            meta.push(MetaPatch::TrackUnknownKey { track: track.clone(), key: "inserts".into(), before, after });
        }
    }
    Ok(())
}

#[cfg(test)]
mod durability_tests {
    use super::*;

    #[test]
    fn movement_overflow_rejects_the_entire_batch_without_publishing_its_prefix() {
        for operation in 0..4 {
            let mut project = Project::new_empty();
            let track = project.tracks[0].id.clone();
            let mut note = Note::new(Revision::MAX as i64 - 480, 480, 60);
            note.id = "boundary-note".into();
            let id = note.id.clone();
            project.tracks[0].notes.push(note);
            let mut store = ProjectStore::new(project);
            let before = store.snapshot();
            let command = match operation {
                0 => Command::MoveNotes { track:track.clone(),note_ids:vec![id.clone()],delta_tick:1,delta_semitones:0 },
                1 => Command::DuplicateNotes { track:track.clone(),source_ids:vec![id.clone()],new_ids:vec!["copy".into()],delta_tick:1,delta_semitones:0 },
                _ => Command::ResizeNotes { track:track.clone(),note_ids:vec![id.clone()],edge:crate::Edge::End,
                    delta_tick:if operation == 2 { 1 } else { i64::MAX } },
            };
            let error = store.apply(store.revision(), CommandBatch { label:"bounded batch".into(),commands:vec![
                Command::SetLyric { track,note_id:id,text:"uncommitted prefix".into() },command,
            ] }).unwrap_err();
            assert!(matches!(error, DomainError::InvalidNote { .. }));
            assert!(Arc::ptr_eq(&before, &store.snapshot()));
            assert_eq!(store.revision().get(), 0);
            assert_eq!(store.undo_depth(), 0);
            assert!(store.log().is_empty());
        }
    }

    fn fixture() -> (ProjectStore, TrackId) {
        let mut project = Project::new_empty();
        let track = project.tracks[0].id.clone();
        let mut note = Note::new(960, 480, 60);
        note.id = "n1".into();
        project.tracks[0].notes.push(note);
        (ProjectStore::new(project), track)
    }

    fn moving(track: &TrackId) -> CommandBatch {
        CommandBatch::single("move", Command::MoveNotes {
            track: track.clone(), note_ids: vec!["n1".into()], delta_tick: 240, delta_semitones: 0,
        })
    }

    fn reject(_: &HistoryEntry, _: &Commit) -> Result<(), DomainError> {
        Err(DomainError::InvalidCommand("injected durability failure".into()))
    }

    #[test]
    fn failed_durability_preserves_edit_undo_and_redo_state() {
        let (mut store, track) = fixture();
        let snapshot = store.snapshot();
        assert!(store.apply_with(store.revision(), moving(&track), reject).is_err());
        assert!(Arc::ptr_eq(&snapshot, &store.snapshot()));
        assert_eq!(store.revision().get(), 0);
        assert_eq!(store.undo_depth(), 0);
        assert_eq!(store.redo_depth(), 0);
        assert!(store.log().is_empty());
        assert!(!store.is_dirty());

        store.apply(store.revision(), moving(&track)).unwrap();
        let snapshot = store.snapshot();
        assert!(store.undo_with(reject).is_err());
        assert!(Arc::ptr_eq(&snapshot, &store.snapshot()));
        assert_eq!(store.revision().get(), 1);
        assert_eq!(store.undo_depth(), 1);
        assert_eq!(store.redo_depth(), 0);
        assert_eq!(store.log().len(), 1);

        store.undo().unwrap();
        let snapshot = store.snapshot();
        assert!(store.redo_with(reject).is_err());
        assert!(Arc::ptr_eq(&snapshot, &store.snapshot()));
        assert_eq!(store.revision().get(), 2);
        assert_eq!(store.undo_depth(), 0);
        assert_eq!(store.redo_depth(), 1);
        assert_eq!(store.log().len(), 2);
    }

    #[test]
    fn exhausted_revision_does_not_pop_the_undo_entry() {
        let (store, track) = fixture();
        let mut project = store.project_for_save();
        project.revision = Revision::MAX - 1;
        let mut store = ProjectStore::new(project);
        store.apply(store.revision(), moving(&track)).unwrap();
        assert_eq!(store.undo(), Err(DomainError::RevisionOverflow));
        assert_eq!(store.undo_depth(), 1);
        assert_eq!(store.revision().get(), Revision::MAX);
    }

    #[test]
    fn add_edit_remove_track_inside_one_batch_is_not_a_commit() {
        let (mut store, _) = fixture();
        let track = Track::new("temporary", "blue");
        let id = track.id.clone();
        let mut note = Note::new(0, 480, 60);
        note.id = "temporary-note".into();
        let result = store.apply(store.revision(), CommandBatch {
            label: "cancelled transaction".into(),
            commands: vec![
                Command::AddTrack { index: 1, track },
                Command::InsertNotes { track: id.clone(), notes: vec![note] },
                Command::RemoveTrack { track: id },
            ],
        }).unwrap();
        assert!(result.is_none());
        assert_eq!(store.revision().get(), 0);
        assert!(store.log().is_empty());
        assert_eq!(store.undo_depth(), 0);
    }

    #[test]
    fn invalid_notes_in_added_tracks_are_refused() {
        let (mut store, _) = fixture();
        let mut track = Track::new("invalid", "blue");
        track.notes.push(Note::new(-1, 480, 60));
        assert!(matches!(store.apply(store.revision(), CommandBatch::single("invalid", Command::AddTrack { index: 1, track })), Err(DomainError::InvalidNote { .. })));
        assert_eq!(store.revision().get(), 0);
        assert_eq!(store.project().tracks.len(), 1);
    }
}
