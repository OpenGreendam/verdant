//! A durable single-writer project with incremental transaction recovery.
//!
//! `<document-name>.journal` lives beside the document. Its first frame is a
//! full checkpoint (including undo/redo); later frames are small committed
//! transactions. `autosave()` atomically compacts that file only when the live
//! revision advanced. Pointer previews and no-op/rejected edits write nothing.
//! `save()` writes the user's document before replacing the recovery checkpoint;
//! reopening also recognizes a crash between those two durable writes.
//! The desktop asks about `pending_recovery()` before continuing: accepting
//! keeps the verified state and its history; discarding checkpoints only the
//! last saved document. Neither choice writes the user's project file.

use std::fs::{File, OpenOptions};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::command::CommandBatch;
use crate::ids::Revision;
use crate::io::{IoError, LoadReport, load_project, require_project_path, save_project_atomic, save_project_new};
use crate::journal::{self, Checkpoint, DocumentStamp, Journal, JournalError};
use crate::model::Project;
use crate::store::{Commit, DomainError, ProjectStore};

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("{0}")]
    Io(#[from] IoError),
    #[error("{0}")]
    Domain(#[from] DomainError),
    #[error("{0}")]
    Journal(#[from] JournalError),
    #[error("document or recovery data already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("project is already open for writing: {0}")]
    Busy(PathBuf),
}

/// The UI presents the saved/recoverable revisions and recovery timestamp.
/// Recovery changes memory and the recovery log, never the last saved document.
#[derive(Debug, Default, Clone)]
pub struct RecoveryReport {
    pub load: LoadReport,
    pub used_autosave: bool,
    pub replayed_commits: usize,
    pub torn_tail_bytes: u64,
    pub saved_revision: u64,
    pub recovered_revision: u64,
    pub recovered_at_ms: i64,
    /// The document was saved but the process stopped before checkpointing its
    /// new saved-file identity. Its content matched a verified journal state.
    pub completed_interrupted_save: bool,
}

impl RecoveryReport {
    pub fn recovered_anything(&self) -> bool {
        self.used_autosave || self.replayed_commits > 0 || self.torn_tail_bytes > 0 || self.completed_interrupted_save
    }
}

/// Append the suffix instead of replacing the document extension, keeping the
/// recovery log beside its complete native filename.
pub fn journal_path(project_path: &Path) -> PathBuf {
    let mut name = project_path.as_os_str().to_os_string();
    name.push(".journal");
    PathBuf::from(name)
}

/// Stable OS advisory-lock inode. Never unlink this sidecar: another process
/// may already have it open. Closing the lease releases the lock after a crash.
pub fn lock_path(project_path: &Path) -> PathBuf {
    let mut name = project_path.as_os_str().to_os_string();
    name.push(".lock");
    PathBuf::from(name)
}

fn resolved_path(path: &Path) -> Result<PathBuf, PersistenceError> {
    require_project_path(path)?;
    if path.try_exists().map_err(|source| IoError::Io { path: path.into(), source })? {
        let canonical = path.canonicalize().map_err(|source| IoError::Io { path: path.into(), source })?;
        require_project_path(&canonical)?;
        return Ok(canonical);
    }
    let name = path.file_name().ok_or_else(|| IoError::Document("project destination needs a filename".into()))?;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let parent = if parent.is_dir() {
        parent.canonicalize().map_err(|source| IoError::Io { path: parent.into(), source })?
    } else {
        // The domain's atomic writer also creates missing parent directories.
        std::fs::create_dir_all(parent).map_err(|source| IoError::Io { path: parent.into(), source })?;
        parent.canonicalize().map_err(|source| IoError::Io { path: parent.into(), source })?
    };
    Ok(parent.join(name))
}

#[derive(Debug)]
struct ProjectLease {
    _file: File,
}

impl ProjectLease {
    fn acquire(path: &Path) -> Result<Self, PersistenceError> {
        #[cfg(unix)]
        if let Ok(metadata) = path.metadata() {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() > 1 {
                return Err(IoError::Document("a recoverable project writer requires a document without hard-link aliases".into()).into());
            }
        }
        let sidecar = lock_path(path);
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&sidecar).map_err(|source| IoError::Io { path: sidecar.clone(), source })?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(std::fs::TryLockError::WouldBlock) => Err(PersistenceError::Busy(path.into())),
            Err(std::fs::TryLockError::Error(source)) => Err(IoError::Io { path: sidecar, source }.into()),
        }
    }
}

struct OpenCandidate {
    store: ProjectStore,
    saved: DocumentStamp,
    saved_at_ms: i64,
    checkpoint_revision: Revision,
    position: Option<journal::JournalPosition>,
    report: RecoveryReport,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn stamp(store: &ProjectStore) -> DocumentStamp {
    DocumentStamp { project_id: store.project().project_id.clone(), revision: store.revision(), digest: store.content_digest() }
}

/// The store is readable through `store()`/immutable Deref. No mutable store is
/// exposed: every accepted edit, undo and redo passes through durable append.
#[derive(Debug)]
pub struct ProjectPersistence {
    store: ProjectStore,
    project_path: PathBuf,
    lease: ProjectLease,
    journal: Journal,
    saved: DocumentStamp,
    saved_at_ms: i64,
    checkpoint_revision: Revision,
    pending_recovery: Option<RecoveryReport>,
    // A checkpoint failure after rename may leave the previous writer pointing
    // at an unlinked inode. Require reopening rather than accepting lost edits.
    needs_reopen: bool,
}

impl Deref for ProjectPersistence {
    type Target = ProjectStore;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl ProjectPersistence {
    pub fn store(&self) -> &ProjectStore {
        &self.store
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    /// A verified recovery newer than the last manual save needs an explicit
    /// desktop decision. `open` retains its transaction-replay contract; this
    /// report lets the UI offer the recovered state or the saved document.
    pub fn pending_recovery(&self) -> Option<&RecoveryReport> {
        self.pending_recovery.as_ref()
    }

    /// Keep the already verified recovered project, undo/redo and revisions.
    /// The last saved document is deliberately left untouched and dirty stays
    /// true whenever the recovered musical content differs from that save.
    pub fn accept_recovery(&mut self) -> bool {
        self.pending_recovery.take().is_some()
    }

    /// Explicitly abandon the recovered branch and reopen the last manual
    /// save. Verify that neither the live project nor the saved document has
    /// changed since the offer, so a delayed dialog cannot discard new edits.
    /// Replace the journal atomically before publishing the saved state.
    pub fn discard_recovery(&mut self) -> Result<bool, PersistenceError> {
        self.ensure_writable()?;
        let Some(report) = self.pending_recovery.as_ref() else { return Ok(false) };
        if self.store.revision().get() != report.recovered_revision {
            return Err(IoError::Document("project changed since the recovery offer; reopen before discarding".into()).into());
        }
        let (project, _) = load_project(&self.project_path)?;
        let store = ProjectStore::new(project);
        if stamp(&store) != self.saved {
            return Err(JournalError::StaleIdentity("saved document changed since the recovery offer".into()).into());
        }
        let checkpoint = Checkpoint::new(store.checkpoint(), self.saved.clone(), self.saved_at_ms, now_ms());
        self.needs_reopen = true;
        let journal = Journal::checkpoint(&journal_path(&self.project_path), &checkpoint)?;
        self.journal = journal;
        self.checkpoint_revision = store.revision();
        self.store = store;
        self.pending_recovery = None;
        self.needs_reopen = false;
        Ok(true)
    }

    /// Open a native `.verp` document, verify the saved-file
    /// identity, recover complete transactions, then repair a torn final frame.
    /// A corrupt earlier record or stale/foreign identity returns an error and
    /// leaves both files untouched.
    pub fn open(path: &Path) -> Result<(Self, RecoveryReport), PersistenceError> {
        Self::open_if(path, &mut |_| true).map(|opened| opened.expect("unconditional admission"))
    }

    /// Check the decoded saved document before opening or modifying its recovery journal.
    pub fn open_if(path: &Path, admit: &mut dyn FnMut(&Project) -> bool) -> Result<Option<(Self, RecoveryReport)>, PersistenceError> {
        Self::open_inner(path, admit, &mut |_, _| true)
    }

    /// Read and verify the saved identity, complete journal chain and histories
    /// without creating a lock, opening a writer, repairing a tail or saving.
    pub fn inspect(path: &Path) -> Result<(ProjectStore, RecoveryReport), PersistenceError> {
        require_project_path(path)?;
        let path = path.canonicalize().map_err(|source| IoError::Io { path: path.into(), source })?;
        let candidate = Self::read_candidate(&path, &mut |_| true)?.expect("unconditional admission");
        Ok((candidate.store, candidate.report))
    }

    /// Decide against the verified recovered store while holding the writer
    /// lease, before creating or repairing any journal. Rejected revisions,
    /// versions and recovery choices therefore leave recovery evidence intact.
    pub fn open_admitted(
        path: &Path,
        admit: &mut dyn FnMut(&ProjectStore, &RecoveryReport) -> bool,
    ) -> Result<Option<(Self, RecoveryReport)>, PersistenceError> {
        Self::open_inner(path, &mut |_| true, admit)
    }

    fn open_inner(
        path: &Path,
        saved_admit: &mut dyn FnMut(&Project) -> bool,
        admit: &mut dyn FnMut(&ProjectStore, &RecoveryReport) -> bool,
    ) -> Result<Option<(Self, RecoveryReport)>, PersistenceError> {
        let path = resolved_path(path)?;
        let lease = ProjectLease::acquire(&path)?;
        let Some(candidate) = Self::read_candidate(&path, saved_admit)? else { return Ok(None) };
        if !admit(&candidate.store, &candidate.report) { return Ok(None); }
        let journal = Self::resume_candidate(&path, &candidate.store, &candidate.saved, candidate.saved_at_ms, candidate.position)?;
        let report = candidate.report;
        Ok(Some((Self {
            store: candidate.store,
            project_path: path,
            lease,
            journal,
            saved: candidate.saved,
            saved_at_ms: candidate.saved_at_ms,
            checkpoint_revision: candidate.checkpoint_revision,
            pending_recovery: (report.recovered_revision > report.saved_revision).then(|| report.clone()),
            needs_reopen: false,
        }, report)))
    }

    /// Reload the current binding without releasing its lease. A desktop that
    /// explicitly reopens its current document must use this instead of opening
    /// a competing writer and then dropping the previous one.
    pub fn reopen_if(&mut self, admit: &mut dyn FnMut(&Project) -> bool) -> Result<Option<RecoveryReport>, PersistenceError> {
        let Some(candidate) = Self::read_candidate(&self.project_path, admit)? else { return Ok(None) };
        let journal = Self::resume_candidate(&self.project_path, &candidate.store, &candidate.saved, candidate.saved_at_ms, candidate.position)?;
        self.store = candidate.store;
        self.journal = journal;
        self.saved = candidate.saved;
        self.saved_at_ms = candidate.saved_at_ms;
        self.checkpoint_revision = candidate.checkpoint_revision;
        self.pending_recovery = (candidate.report.recovered_revision > candidate.report.saved_revision).then(|| candidate.report.clone());
        self.needs_reopen = false;
        Ok(Some(candidate.report))
    }

    fn resume_candidate(path: &Path, store: &ProjectStore, saved: &DocumentStamp, saved_at_ms: i64, position: Option<journal::JournalPosition>) -> Result<Journal, PersistenceError> {
        let path = journal_path(path);
        Ok(match position {
            Some(position) => Journal::resume(&path, &saved.project_id, position)?,
            None => Journal::checkpoint(&path, &Checkpoint::new(store.checkpoint(), saved.clone(), saved_at_ms, now_ms()))?,
        })
    }

    fn read_candidate(path: &Path, admit: &mut dyn FnMut(&Project) -> bool) -> Result<Option<OpenCandidate>, PersistenceError> {
        let (project, load) = load_project(path)?;
        if !admit(&project) { return Ok(None); }
        let saved_store = ProjectStore::new(project);
        let saved = stamp(&saved_store);
        let saved_at_ms = path.metadata().and_then(|m| m.modified()).ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64).unwrap_or(0);
        let jpath = journal_path(path);
        let mut report = RecoveryReport { load, saved_revision: saved.revision.get(), ..Default::default() };
        let Some(recovery) = journal::read(&jpath, &saved.project_id)? else {
            report.recovered_revision = saved.revision.get();
            return Ok(Some(OpenCandidate {
                store: saved_store,
                checkpoint_revision: saved.revision,
                saved,
                saved_at_ms,
                position: None,
                report,
            }));
        };
        // Either the saved stamp matches exactly, or this is the small crash
        // window after writing a new document but before marking it saved in
        // the recovery log. In that case its digest must match an actual state
        // on the verified revision chain; revision equality alone is not proof.
        let checkpoint_revision = Revision::new(recovery.checkpoint.store.project.revision)
            .ok_or_else(|| JournalError::Corrupt { offset: 0, reason: "invalid checkpoint revision".into() })?;
        let expected_saved = recovery.checkpoint.saved;
        let mut matched_document = expected_saved == saved;
        let mut store = ProjectStore::from_checkpoint(recovery.checkpoint.store);
        if store.revision() == saved.revision && stamp(&store) == saved {
            matched_document = true;
        }
        report.used_autosave = checkpoint_revision > saved.revision;
        report.recovered_at_ms = recovery.checkpoint.checkpoint_at_ms;
        for (offset, record) in recovery.records {
            store.restore(&record.entry, &record.commit).map_err(|error| JournalError::Corrupt { offset, reason: error.to_string() })?;
            if store.revision() == saved.revision && stamp(&store) == saved {
                matched_document = true;
            }
            report.replayed_commits += 1;
            report.recovered_at_ms = record.committed_at_ms;
        }
        if !matched_document {
            return Err(JournalError::StaleIdentity(format!(
                "document {} at {} does not match the saved stamp or any recoverable state (expected {})",
                saved.project_id, saved.revision, expected_saved.revision,
            )).into());
        }
        report.completed_interrupted_save = expected_saved != saved;
        report.torn_tail_bytes = recovery.position.file_len - recovery.position.complete_len;
        report.recovered_revision = store.revision().get();
        store.set_saved_digest(saved.digest.clone());
        Ok(Some(OpenCandidate {
            store,
            saved,
            saved_at_ms,
            checkpoint_revision,
            position: Some(recovery.position),
            report,
        }))
    }

    /// Establish a new working document and recovery checkpoint. Existing
    /// documents must be opened; explicit Save As may replace a chosen file.
    pub fn create(path: &Path, project: Project) -> Result<Self, PersistenceError> {
        Self::create_from_store(path, ProjectStore::new(project))
    }

    /// Save a new binding of an owned verified snapshot with its complete
    /// undo/redo history. The source document and journal are never touched.
    pub fn create_from_store(path: &Path, mut store: ProjectStore) -> Result<Self, PersistenceError> {
        let path = resolved_path(path)?;
        Self::require_fresh(&path)?;
        let lease = ProjectLease::acquire(&path)?;
        Self::require_fresh(&path)?;
        let recovery_path = journal_path(&path);
        if store.project().project_id.is_empty() || Revision::new(store.project().revision).is_none() {
            return Err(IoError::Document("empty project_id or invalid revision".into()).into());
        }
        save_project_new(&path, &store.project_for_save())?;
        let saved = stamp(&store);
        store.set_saved_digest(saved.digest.clone());
        let saved_at_ms = now_ms();
        let checkpoint = Checkpoint::new(store.checkpoint(), saved.clone(), saved_at_ms, saved_at_ms);
        let journal = Journal::checkpoint_new(&recovery_path, &checkpoint)?;
        Ok(Self {
            checkpoint_revision: store.revision(),
            store,
            project_path: path,
            lease,
            journal,
            saved,
            saved_at_ms,
            pending_recovery: None,
            needs_reopen: false,
        })
    }

    fn ensure_writable(&self) -> Result<(), PersistenceError> {
        if self.needs_reopen {
            return Err(JournalError::Poisoned(self.journal.path().into()).into());
        }
        Ok(())
    }

    pub fn commit(&mut self, base: Revision, batch: CommandBatch) -> Result<Option<Commit>, PersistenceError> {
        self.ensure_writable()?;
        let journal = &mut self.journal;
        self.store.apply_with(base, batch, |entry, commit| Ok(journal.append(entry, commit, now_ms())?))
    }

    pub fn undo(&mut self) -> Result<Option<Commit>, PersistenceError> {
        self.ensure_writable()?;
        let journal = &mut self.journal;
        self.store.undo_with(|entry, commit| Ok(journal.append(entry, commit, now_ms())?))
    }

    pub fn redo(&mut self) -> Result<Option<Commit>, PersistenceError> {
        self.ensure_writable()?;
        let journal = &mut self.journal;
        self.store.redo_with(|entry, commit| Ok(journal.append(entry, commit, now_ms())?))
    }

    fn checkpoint(&mut self) -> Result<(), PersistenceError> {
        let checkpoint = Checkpoint::new(self.store.checkpoint(), self.saved.clone(), self.saved_at_ms, now_ms());
        self.needs_reopen = true;
        let journal = Journal::checkpoint(&journal_path(&self.project_path), &checkpoint)?;
        self.journal = journal;
        self.checkpoint_revision = self.store.revision();
        self.needs_reopen = false;
        Ok(())
    }

    /// Durable user save followed by a recovery checkpoint. Undo/redo history
    /// survives both operations. No journal is truncated before a durable copy
    /// of its entire current project/history exists.
    pub fn save(&mut self) -> Result<(), PersistenceError> {
        self.ensure_writable()?;
        save_project_atomic(&self.project_path, &self.store.project_for_save())?;
        self.saved = stamp(&self.store);
        self.saved_at_ms = now_ms();
        self.store.set_saved_digest(self.saved.digest.clone());
        self.checkpoint()?;
        self.pending_recovery = None;
        Ok(())
    }

    /// Rebind after writing both target document and recovery checkpoint.
    /// A failed Save As leaves the previous binding and history usable.
    pub fn save_as(&mut self, path: &Path) -> Result<(), PersistenceError> {
        self.ensure_writable()?;
        let path = resolved_path(path)?;
        if path == self.project_path {
            return self.save();
        }
        let lease = ProjectLease::acquire(&path)?;
        save_project_atomic(&path, &self.store.project_for_save())?;
        let saved = stamp(&self.store);
        let saved_at_ms = now_ms();
        let checkpoint = Checkpoint::new(self.store.checkpoint(), saved.clone(), saved_at_ms, saved_at_ms);
        let journal = Journal::checkpoint(&journal_path(&path), &checkpoint)?;
        self.store.set_saved_digest(saved.digest.clone());
        self.saved = saved;
        self.saved_at_ms = saved_at_ms;
        self.project_path = path;
        self.lease = lease;
        self.journal = journal;
        self.checkpoint_revision = self.store.revision();
        self.pending_recovery = None;
        Ok(())
    }

    /// Periodic compaction. Every transaction is already durable; a full
    /// checkpoint is written only once per advanced revision, on this explicit
    /// autosave call. The saved document remains untouched and dirty stays true.
    pub fn autosave(&mut self) -> Result<bool, PersistenceError> {
        self.ensure_writable()?;
        if self.checkpoint_revision == self.store.revision() {
            return Ok(false);
        }
        self.checkpoint()?;
        Ok(true)
    }

    /// Preserve the complete current document and undo/redo checkpoint under
    /// an independent recovery path before an explicit revert.
    pub fn fork_with_history(&mut self, path: &Path) -> Result<(), PersistenceError> {
        self.ensure_writable()?;
        let path = resolved_path(path)?;
        Self::require_fresh(&path)?;
        let _lease = ProjectLease::acquire(&path)?;
        Self::require_fresh(&path)?;
        save_project_new(&path, &self.store.project_for_save())?;
        let checkpoint = Checkpoint::new(self.store.checkpoint(), stamp(&self.store), now_ms(), now_ms());
        Journal::checkpoint_new(&journal_path(&path), &checkpoint)?;
        Ok(())
    }

    fn require_fresh(path: &Path) -> Result<(), PersistenceError> {
        for candidate in [path.to_path_buf(), journal_path(path)] {
            // symlink_metadata also rejects a dangling destination symlink.
            match candidate.symlink_metadata() {
                Ok(_) => return Err(PersistenceError::AlreadyExists(candidate)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                Err(source) => return Err(IoError::Io { path: candidate, source }.into()),
            }
        }
        Ok(())
    }

    /// A fresh distribution document, without a recovery sidecar. This is an
    /// export, not a new editing binding; the source store and histories remain
    /// untouched. The destination must use the native `.verp` extension.
    pub fn export_snapshot(path: &Path, project: &Project) -> Result<(), PersistenceError> {
        let path = resolved_path(path)?;
        Self::require_fresh(&path)?;
        let _lease = ProjectLease::acquire(&path)?;
        Self::require_fresh(&path)?;
        Ok(save_project_new(&path, project)?)
    }

    /// Read the user's last manual save without replaying or changing its
    /// journal. The caller binds a fresh working document before swapping UI.
    pub fn read_saved(path: &Path) -> Result<Project, PersistenceError> { Ok(load_project(path)?.0) }

    /// Distribution export does not change the live binding or saved marker.
    /// An extensionless destination gains the native `.verp` extension.
    pub fn export_project(&self, dest: &Path) -> Result<(), PersistenceError> {
        let dest = crate::io::project_output_path(dest)?;
        let dest = resolved_path(&dest)?;
        if dest == self.project_path {
            return Err(IoError::Document("use Save to replace the active project document".into()).into());
        }
        let _lease = ProjectLease::acquire(&dest)?;
        Ok(save_project_atomic(&dest, &self.store.project_for_save())?)
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use std::io::Write;
    use crate::{Command, Note};

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("verdant-snapshot-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn project(&self) -> PathBuf { self.0.join("song.verp") }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn inspecting_a_saved_document_creates_no_journal_or_lease() {
        let sandbox = Sandbox::new();
        let path = sandbox.project();
        let project = Project::new_empty();
        save_project_atomic(&path, &project).unwrap();
        let before = std::fs::read(&path).unwrap();
        let (store, report) = ProjectPersistence::inspect(&path).unwrap();
        assert_eq!(store.project_for_save(), project);
        assert!(!report.recovered_anything());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!journal_path(&path).exists());
        assert!(!lock_path(&path).exists());
        let opened = ProjectPersistence::open_admitted(&path, &mut |_, _| false).unwrap();
        assert!(opened.is_none());
        assert!(!journal_path(&path).exists());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn inspecting_a_torn_recovery_prefix_preserves_bytes_and_full_history() {
        let sandbox = Sandbox::new();
        let path = sandbox.project();
        let mut project = Project::new_empty();
        let track = project.tracks[0].id.clone();
        project.tracks[0].notes.push(Note::new(960, 480, 60));
        let mut document = ProjectPersistence::create(&path, project).unwrap();
        document.commit(document.revision(), CommandBatch::single("tempo", Command::SetTempoEvent {
            tick: 0, micro_bpm: Some(90_000_000),
        })).unwrap();
        let expected = document.project_for_save();
        drop(document);
        let journal = journal_path(&path);
        OpenOptions::new().append(true).open(&journal).unwrap().write_all(b"unfinished").unwrap();
        let before = std::fs::read(&journal).unwrap();
        let (store, report) = ProjectPersistence::inspect(&path).unwrap();
        assert_eq!(store.project_for_save(), expected);
        assert_eq!(store.undo_depth(), 1);
        assert_eq!(report.torn_tail_bytes, 10);
        assert_eq!(std::fs::read(&journal).unwrap(), before);
        let copy = sandbox.0.join("copy.verp");
        drop(ProjectPersistence::create_from_store(&copy, store).unwrap());
        let (mut copy, _) = ProjectPersistence::open(&copy).unwrap();
        assert_eq!(copy.undo().unwrap().unwrap().revision.get(), 2);
        assert_eq!(copy.project().tempo_map[0].micro_bpm, 120_000_000);
        assert!(copy.project().track(&track).is_some());
        assert_eq!(std::fs::read(&journal).unwrap(), before);
    }

    #[test]
    fn writer_lease_excludes_other_writers_and_same_path_save_keeps_it() {
        let sandbox = Sandbox::new();
        let path = sandbox.project();
        let mut document = ProjectPersistence::create(&path, Project::new_empty()).unwrap();
        assert!(matches!(ProjectPersistence::open(&path), Err(PersistenceError::Busy(_))));
        document.save_as(&path).unwrap();
        assert!(matches!(ProjectPersistence::open(&path), Err(PersistenceError::Busy(_))));
        assert!(ProjectPersistence::read_saved(&path).is_ok());
        drop(document);
        assert!(ProjectPersistence::open(&path).is_ok());
    }
}
