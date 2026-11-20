//! Per-project recovery journal. The first checksummed JSONL frame is a full
//! domain checkpoint (project, undo/redo, saved-file identity); subsequent
//! frames contain only committed transactions. A frame ends at its newline.
//! An unterminated final frame is an interrupted append; any malformed or
//! checksum-invalid *complete* frame is corruption, including the last one.
//! Transaction checksums form a chain and each transaction repeats the project
//! identity and preceding revision, so records cannot migrate between logs.

use std::fs::{File, OpenOptions};
#[cfg(windows)]
use std::io::SeekFrom;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::digest::sha256_hex;
use crate::ids::Revision;
use crate::io::{IoError, atomic_create, atomic_replace};
use crate::store::{Commit, HistoryEntry, StoreCheckpoint, StoreCheckpointRef, refresh_commit_indexes, refresh_entry_indexes};

const JOURNAL_VERSION: u32 = 1;

impl crate::ProjectPersistence {
    /// Jump to a position in `store().history()`, persisting every inverse or
    /// forward transaction before publishing it. No synthetic edit is added;
    /// subsequent ordinary edits still truncate the redo branch. The base
    /// revision rejects a click from a panel whose branch has since changed.
    ///
    /// On a storage failure, earlier steps remain durable. Consumers should
    /// refresh from the actual revision even when this returns an error.
    pub fn jump_to_history(
        &mut self,
        base: Revision,
        position: usize,
    ) -> Result<Vec<Commit>, crate::PersistenceError> {
        let (kind, steps) = self.store().history_jump_steps(base, position)?;
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

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("I/O error on journal {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{0}")]
    Storage(#[from] IoError),
    #[error("journal belongs to project {journal} but this document is {project}")]
    IdentityMismatch { project: String, journal: String },
    #[error("journal base {journal_base} does not match document {project_base}")]
    StaleBase { project_base: Revision, journal_base: Revision },
    #[error("saved project identity does not match the recovery journal: {0}")]
    StaleIdentity(String),
    #[error("journal corrupt at byte {offset}: {reason}")]
    Corrupt { offset: u64, reason: String },
    #[error("journal write failed ({write}); rollback failed ({rollback}); reopen {path} before editing")]
    WriteUncertain { path: PathBuf, write: std::io::Error, rollback: std::io::Error },
    #[error("journal {0} needs reopening after an incomplete write")]
    Poisoned(PathBuf),
    #[error("JSON error in journal: {0}")]
    Json(#[from] serde_json::Error),
}

/// Identity of the user's last saved document; distinct from an unsaved
/// recovery checkpoint. Revision alone cannot distinguish divergent copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DocumentStamp {
    pub project_id: String,
    pub revision: Revision,
    pub digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Checkpoint<S = StoreCheckpoint> {
    pub format: String,
    pub version: u32,
    pub saved: DocumentStamp,
    pub saved_at_ms: i64,
    pub checkpoint_at_ms: i64,
    pub store: S,
}

impl<'a> Checkpoint<StoreCheckpointRef<'a>> {
    pub fn new(store: StoreCheckpointRef<'a>, saved: DocumentStamp, saved_at_ms: i64, checkpoint_at_ms: i64) -> Self {
        Self { format: "verdant-journal".into(), version: JOURNAL_VERSION, saved, saved_at_ms, checkpoint_at_ms, store }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JournalRecord {
    pub project_id: String,
    pub base_revision: Revision,
    pub previous: String,
    pub entry: HistoryEntry,
    pub commit: Commit,
    pub committed_at_ms: i64,
}

#[derive(Serialize)]
struct BorrowedRecord<'a> {
    project_id: &'a str,
    base_revision: Revision,
    previous: &'a str,
    entry: &'a HistoryEntry,
    commit: &'a Commit,
    committed_at_ms: i64,
}

#[derive(Serialize, Deserialize)]
struct Frame {
    sha256: String,
    // Hash the exact payload bytes, preserving unknown-object key order. This
    // does not rely on serde_json's optional map-ordering feature.
    payload: String,
}

fn encode(value: &impl Serialize) -> Result<(Vec<u8>, String), JournalError> {
    let payload = serde_json::to_string(value)?;
    let hash = sha256_hex(payload.as_bytes());
    let mut bytes = serde_json::to_vec(&Frame { sha256: hash.clone(), payload })?;
    bytes.push(b'\n');
    Ok((bytes, hash))
}

fn decode(line: &[u8], offset: u64) -> Result<Frame, JournalError> {
    let frame: Frame = serde_json::from_slice(line).map_err(|error| corrupt(offset, error.to_string()))?;
    if sha256_hex(frame.payload.as_bytes()) != frame.sha256 {
        return Err(corrupt(offset, "checksum mismatch"));
    }
    Ok(frame)
}

fn corrupt(offset: u64, reason: impl Into<String>) -> JournalError {
    JournalError::Corrupt { offset, reason: reason.into() }
}

pub(crate) struct Recovery {
    pub checkpoint: Checkpoint,
    pub records: Vec<(u64, JournalRecord)>,
    pub position: JournalPosition,
}

pub(crate) struct JournalPosition {
    pub complete_len: u64,
    pub file_len: u64,
    pub last_hash: String,
    pub last_revision: Revision,
}

/// Read without changing the recovery file. Repair is deferred until every
/// transaction has also passed the reducer, preserving evidence on failures.
pub(crate) fn read(path: &Path, project_id: &str) -> Result<Option<Recovery>, JournalError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(JournalError::Io { path: path.into(), source }),
    };
    let io = |source| JournalError::Io { path: path.into(), source };
    let file_len = file.metadata().map_err(io)?.len();
    // A read-only inspector can run while the lease holder appends. Bind this
    // snapshot to the opened inode's initial prefix, including an incomplete
    // tail, instead of reading newly appended frames beyond `file_len`.
    let mut reader = BufReader::new(file.take(file_len));
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).map_err(io)?;
    if line.last() != Some(&b'\n') {
        return Err(corrupt(0, "checkpoint is empty or interrupted"));
    }
    let frame = decode(&line, 0)?;
    let checkpoint: Checkpoint = serde_json::from_str(&frame.payload).map_err(|error| corrupt(0, error.to_string()))?;
    if checkpoint.format != "verdant-journal" || checkpoint.version != JOURNAL_VERSION {
        return Err(corrupt(0, "unsupported journal format/version"));
    }
    let checkpoint_id = &checkpoint.store.project.project_id;
    if checkpoint_id != project_id || checkpoint.saved.project_id != project_id {
        return Err(JournalError::IdentityMismatch { project: project_id.into(), journal: checkpoint_id.clone() });
    }
    let mut revision = Revision::new(checkpoint.store.project.revision).ok_or_else(|| corrupt(0, "invalid checkpoint revision"))?;
    if checkpoint.saved.revision > revision {
        return Err(corrupt(0, "saved revision is newer than checkpoint"));
    }
    let mut offset = line.len() as u64;
    let mut hash = frame.sha256;
    let mut records = Vec::new();
    loop {
        line.clear();
        let count = reader.read_until(b'\n', &mut line).map_err(io)?;
        if count == 0 || line.last() != Some(&b'\n') {
            break;
        }
        let frame = decode(&line, offset)?;
        let mut record: JournalRecord = serde_json::from_str(&frame.payload).map_err(|error| corrupt(offset, error.to_string()))?;
        if record.project_id != project_id {
            return Err(JournalError::IdentityMismatch { project: project_id.into(), journal: record.project_id });
        }
        if record.base_revision != revision || Some(record.commit.revision) != revision.next() || record.previous != hash {
            return Err(corrupt(offset, "transaction revision or preceding-checksum mismatch"));
        }
        refresh_entry_indexes(&mut record.entry);
        refresh_commit_indexes(&mut record.commit);
        revision = record.commit.revision;
        hash = frame.sha256;
        records.push((offset, record));
        offset += count as u64;
    }
    Ok(Some(Recovery {
        checkpoint,
        records,
        position: JournalPosition { complete_len: offset, file_len, last_hash: hash, last_revision: revision },
    }))
}

#[derive(Debug)]
pub(crate) struct Journal {
    path: PathBuf,
    file: File,
    project_id: String,
    revision: Revision,
    last_hash: String,
    len: u64,
    poisoned: bool,
}

impl Journal {
    /// Atomically replace the entire checkpoint+log. The previous file remains
    /// recoverable until the replacement is durable; no truncate-before-save.
    pub fn checkpoint(path: &Path, checkpoint: &Checkpoint<StoreCheckpointRef<'_>>) -> Result<Self, JournalError> {
        Self::write_checkpoint(path, checkpoint, false)
    }

    pub fn checkpoint_new(path: &Path, checkpoint: &Checkpoint<StoreCheckpointRef<'_>>) -> Result<Self, JournalError> {
        Self::write_checkpoint(path, checkpoint, true)
    }

    fn write_checkpoint(path: &Path, checkpoint: &Checkpoint<StoreCheckpointRef<'_>>, new_only: bool) -> Result<Self, JournalError> {
        let revision = Revision::new(checkpoint.store.project.revision).ok_or_else(|| corrupt(0, "invalid checkpoint revision"))?;
        let payload = serde_json::to_string(checkpoint)?;
        let hash = sha256_hex(payload.as_bytes());
        let frame = Frame { sha256: hash, payload };
        let io_error = |source| IoError::Io { path: path.into(), source };
        let mut len = 0;
        let write = |file: &mut File| {
            let mut writer = BufWriter::new(&mut *file);
            serde_json::to_writer(&mut writer, &frame)?;
            writer.write_all(b"\n").and_then(|()| writer.flush()).map_err(io_error)?;
            drop(writer);
            len = file.stream_position().map_err(io_error)?;
            Ok(())
        };
        if new_only { atomic_create(path, write)?; } else { atomic_replace(path, write)?; }
        Self::open_writer(path, &checkpoint.saved.project_id, revision, frame.sha256, len)
    }

    /// Open the verified prefix for append. Truncating an interrupted final
    /// frame happens only after recovery has accepted all complete records.
    pub fn resume(path: &Path, project_id: &str, position: JournalPosition) -> Result<Self, JournalError> {
        let journal = Self::open_writer(
            path,
            project_id,
            position.last_revision,
            position.last_hash,
            position.complete_len,
        )?;
        if position.complete_len != position.file_len {
            journal.file.set_len(position.complete_len).map_err(|source| JournalError::Io { path: path.into(), source })?;
            journal.file.sync_all().map_err(|source| JournalError::Io { path: path.into(), source })?;
        }
        Ok(journal)
    }

    fn open_writer(path: &Path, project_id: &str, revision: Revision, last_hash: String, len: u64) -> Result<Self, JournalError> {
        let io_error = |source| JournalError::Io { path: path.into(), source };
        let mut options = OpenOptions::new();
        options.write(true);
        // Windows append-only access excludes the write-data right needed
        // to truncate a torn or failed frame. The journal has one writer.
        #[cfg(not(windows))]
        options.append(true);
        let file = options.open(path).map_err(io_error)?;
        #[cfg(windows)]
        let file = {
            let mut file = file;
            file.seek(SeekFrom::Start(len)).map_err(io_error)?;
            file
        };
        Ok(Self { path: path.into(), file, project_id: project_id.into(), revision, last_hash, len, poisoned: false })
    }

    /// Write before publishing the store candidate. If a write/flush fails,
    /// roll back the append to the previous complete length and sync it. A
    /// failed rollback poisons this writer until explicit recovery/reopen.
    pub fn append(&mut self, entry: &HistoryEntry, commit: &Commit, committed_at_ms: i64) -> Result<(), JournalError> {
        if self.poisoned {
            return Err(JournalError::Poisoned(self.path.clone()));
        }
        if Some(commit.revision) != self.revision.next() {
            return Err(corrupt(self.len, "writer/store revision mismatch"));
        }
        let disk_len = self.file.metadata().map_err(|source| JournalError::Io { path: self.path.clone(), source })?.len();
        if disk_len != self.len {
            self.poisoned = true;
            return Err(corrupt(self.len, "journal changed outside its writer"));
        }
        let record = BorrowedRecord {
            project_id: &self.project_id,
            base_revision: self.revision,
            previous: &self.last_hash,
            entry,
            commit,
            committed_at_ms,
        };
        let (bytes, hash) = encode(&record)?;
        if let Err(write) = self.file.write_all(&bytes).and_then(|()| self.file.sync_all()) {
            if let Err(rollback) = self.rollback_append() {
                self.poisoned = true;
                return Err(JournalError::WriteUncertain { path: self.path.clone(), write, rollback });
            }
            return Err(JournalError::Io { path: self.path.clone(), source: write });
        }
        self.last_hash = hash;
        self.revision = commit.revision;
        self.len += bytes.len() as u64;
        Ok(())
    }

    fn rollback_append(&mut self) -> std::io::Result<()> {
        self.file.set_len(self.len)?;
        #[cfg(windows)]
        self.file.seek(SeekFrom::Start(self.len))?;
        self.file.sync_all()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
