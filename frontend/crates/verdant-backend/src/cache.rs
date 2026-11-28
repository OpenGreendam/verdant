//! Durable, content-digest-keyed render cache (SPEC §11; TODO §4).
//!
//! A [`CacheEntry`] is persisted under `<root>/<digest>/` as the verified
//! artifact bytes plus a small `manifest.json`:
//!
//! ```text
//! <root>/<digest>/native.wav      verified PCM, byte-identical to the worker artifact
//! <root>/<digest>/native.npz      optional native target F0 and time mapping
//! <root>/<digest>/manifest.json   digest, revision, job_id, route, slot,
//!                                 artifact {file, sha256, bytes}
//! ```
//!
//! Guarantees:
//! * **Crash-safe writes.** A per-digest writer lease protects a staging
//!   sibling. Artifacts are fsynced and renamed; the manifest is committed last, so a
//!   torn write yields a missing or stale manifest — a clean miss, never a
//!   half-entry.
//! * **Verified restores.** The manifest is schema-checked and its declared
//!   digest/size/SHA-256 are re-validated against the contained bytes before
//!   anything is decoded. Corruption is a visible [`CacheError::Corrupt`],
//!   never unchecked playback.
//! * **Blocking I/O by caller contract.** Every method performs filesystem
//!   work; callers must invoke them off the GPUI update path and off the
//!   audio callback (the app already runs verification in a background task —
//!   cache calls belong there too).

use std::fs::{self, File, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::artifact::{decode_wav, read_pitch_from_npz};
use crate::scheduler::{CacheEntry, F0Kind, VerifiedAudio, VerifiedF0Track};
use crate::wire::Route;

const MANIFEST: &str = "manifest.json";
const WAV_NAME: &str = "native.wav";
const NPZ_NAME: &str = "native.npz";
const MANIFEST_VERSION: u32 = 3;

// Released readers unlink any immediate child of an entry containing ".tmp.".
// Keep both the stage and the names immediately inside this directory immune.
const STATE: &str = ".verdant-cache-writes";
const STAGE: &str = ".verdant-cache-stage";

fn open_writer_lock(locks: &Path, digest: &std::ffi::OsStr) -> io::Result<File> {
    fs::create_dir_all(locks)?;
    // This inode is permanent. Waiters and recovery always lock the same file.
    File::options().read(true).write(true).create(true).truncate(false).open(locks.join(digest))
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

fn recover_legacy(dir: &Path) -> io::Result<()> {
    for item in fs::read_dir(dir)? {
        let item = item?;
        let name = item.file_name();
        let Some((artifact, counter)) = name.to_str().and_then(|n| n.rsplit_once(".tmp.")) else { continue };
        // A released writer's PID.counter spelling cannot prove abandonment.
        if matches!(artifact, ".native.wav" | ".native.npz" | ".manifest.json")
            && !counter.is_empty() && counter.bytes().all(|c| c.is_ascii_digit())
        {
            let _ = fs::remove_file(item.path());
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("cache I/O: {0}")]
    Io(#[from] io::Error),
    /// Bytes or metadata failed verification; surfaced visibly, never played.
    #[error("corrupt cache entry: {0}")]
    Corrupt(String),
}

#[derive(Serialize, Deserialize)]
struct ManifestArtifact {
    file: String,
    sha256: String,
    bytes: u64,
}

#[derive(Serialize, Deserialize)]
struct ManifestSlot {
    track: String,
    phrase_index: usize,
    start_seconds: f64,
    seconds: f64,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    digest: String,
    job_id: String,
    revision: u64,
    route: String,
    slot: Option<ManifestSlot>,
    artifacts: Vec<ManifestArtifact>,
}

/// The on-disk cache. Construct with [`ArtifactCache::open`].
pub struct ArtifactCache {
    root: PathBuf,
}

impl ArtifactCache {
    /// Recover abandoned writes without waiting for live writers. A directory
    /// without a committed manifest remains a clean miss.
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let locks = root.join(STATE).join("locks");
        for dir in fs::read_dir(&root)? {
            let dir = dir?;
            let name = dir.file_name();
            if name == STATE || !dir.file_type()?.is_dir() {
                continue;
            }
            let dir = dir.path();
            if !locks.join(&name).try_exists()? {
                recover_legacy(&dir)?;
            }
            // Modern entries have one known staging path. Inspect no payloads
            // and acquire no locks for a clean entry.
            let stage = dir.join(STAGE);
            if stage.try_exists()? {
                let lock = match open_writer_lock(&locks, &name) {
                    Err(e) if matches!(e.kind(), io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem) => continue,
                    result => result?,
                };
                match lock.try_lock() {
                    Ok(()) => remove_if_present(&stage)?,
                    Err(TryLockError::WouldBlock) => (),
                    Err(TryLockError::Error(error)) => return Err(error),
                }
            }
        }
        Ok(ArtifactCache { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn entry_dir(&self, digest: &str) -> PathBuf {
        self.root.join(digest)
    }

    /// Digests with a committed manifest (still verified again on restore).
    pub fn entries(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(dirs) = fs::read_dir(&self.root) {
            for dir in dirs.flatten() {
                let name = dir.file_name().to_string_lossy().to_string();
                if dir.path().join(MANIFEST).is_file() {
                    out.push(name);
                }
            }
        }
        out.sort();
        out
    }

    /// Persist a verified entry: copy the artifact bytes into the cache and
    /// commit the manifest last. Safe to call again for the same digest —
    /// artifacts replace atomically and the manifest lands last.
    ///
    /// Blocking I/O: call off the UI/audio threads.
    pub fn store(&self, entry: &CacheEntry) -> Result<(), CacheError> {
        if entry.audio.f0_tracks.iter().any(|track| track.track.kind != F0Kind::NativeTarget) {
            return Err(CacheError::Corrupt("render cache admits only native target F0".into()));
        }
        let dir = self.entry_dir(&entry.digest);
        fs::create_dir_all(&dir)?;
        let mut artifacts = Vec::new();
        let wav = fs::read(&entry.audio.path).map_err(|e| CacheError::Corrupt(format!("{}: {e}", entry.audio.path.display())))?;
        let wav_sha = hex::encode(Sha256::digest(&wav));
        if !wav_sha.eq_ignore_ascii_case(&entry.audio.sha256) {
            return Err(CacheError::Corrupt(format!(
                "verified audio sha256 {} does not match {} on disk",
                entry.audio.sha256, wav_sha
            )));
        }
        let writer = CacheWriter::begin(&self.root, &dir)?;
        writer.write_atomic(WAV_NAME, &wav)?;
        artifacts.push(ManifestArtifact { file: WAV_NAME.into(), sha256: wav_sha, bytes: wav.len() as u64 });
        let mut f0_sources = std::collections::BTreeMap::new();
        for f0 in &entry.audio.f0_tracks {
            if let Some(previous) = f0_sources.insert(f0.track.kind.cache_file(), f0) {
                if previous.sha256 != f0.sha256 || previous.source != f0.source {
                    return Err(CacheError::Corrupt("F0 tracks disagree about their shared source artifact".into()));
                }
            }
        }
        for (name, f0) in f0_sources {
            let npz = fs::read(&f0.source).map_err(|e| CacheError::Corrupt(format!("{}: {e}", f0.source.display())))?;
            let digest = hex::encode(Sha256::digest(&npz));
            if !digest.eq_ignore_ascii_case(&f0.sha256) {
                return Err(CacheError::Corrupt(format!("{}: verified F0 artifact changed before cache storage", f0.source.display())));
            }
            writer.write_atomic(name, &npz)?;
            artifacts.push(ManifestArtifact {
                file: name.into(),
                sha256: digest,
                bytes: npz.len() as u64,
            });
        }
        let slot = entry.track.as_ref().map(|t| ManifestSlot {
            track: t.as_str().to_string(),
            phrase_index: entry.phrase_index.unwrap_or(0),
            start_seconds: entry.start_seconds,
            seconds: entry.seconds,
        });
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            digest: entry.digest.clone(),
            job_id: entry.job_id.clone(),
            revision: entry.revision,
            route: entry.route.as_str().into(),
            slot,
            artifacts,
        };
        let body = serde_json::to_vec(&manifest).map_err(|e| CacheError::Corrupt(e.to_string()))?;
        writer.write_atomic(MANIFEST, &body)?;
        #[cfg(not(windows))]
        fs::File::open(&dir)?.sync_all()?;
        Ok(())
    }

    fn read_manifest(&self, dir: &Path, digest: &str) -> Result<Manifest, CacheError> {
        let body = fs::read(dir.join(MANIFEST)).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => CacheError::Corrupt(format!("{digest}: no manifest (incomplete entry)")),
            _ => e.into(),
        })?;
        let m: Manifest =
            serde_json::from_slice(&body).map_err(|e| CacheError::Corrupt(format!("{digest}: manifest not JSON: {e}")))?;
        if !(1..=MANIFEST_VERSION).contains(&m.version) {
            return Err(CacheError::Corrupt(format!("{digest}: manifest version {}", m.version)));
        }
        if m.digest != digest {
            return Err(CacheError::Corrupt(format!("{digest}: manifest claims {}", m.digest)));
        }
        Ok(m)
    }

    /// Read a cached artifact set for `digest`. `Ok(None)` is a clean miss
    /// (no entry dir / no manifest file is *not* an error when the dir is
    /// simply absent). `Err(Corrupt)` means on-disk data failed validation.
    fn read_artifact(&self, dir: &Path, want: &ManifestArtifact) -> Result<Vec<u8>, CacheError> {
        let bytes = fs::read(dir.join(&want.file))
            .map_err(|e| CacheError::Corrupt(format!("{}: {e}", want.file)))?;
        if bytes.len() as u64 != want.bytes {
            return Err(CacheError::Corrupt(format!(
                "{}: {} bytes on disk, manifest says {}",
                want.file,
                bytes.len(),
                want.bytes
            )));
        }
        let sha = hex::encode(Sha256::digest(&bytes));
        if !sha.eq_ignore_ascii_case(&want.sha256) {
            return Err(CacheError::Corrupt(format!("{}: SHA-256 mismatch", want.file)));
        }
        Ok(bytes)
    }

    /// Restore a verified entry. The manifest *and* every contained artifact
    /// byte are re-validated (size + SHA-256), then the WAV and optional NPZ
    /// are decoded exactly like a fresh [`crate::artifact::verify_and_decode`]
    /// result. Corruption is reported, never played.
    ///
    /// Blocking I/O: call off the UI/audio threads.
    pub fn restore(&self, digest: &str) -> Result<Option<CacheEntry>, CacheError> {
        let dir = self.entry_dir(digest);
        if !dir.is_dir() {
            return Ok(None);
        }
        if !dir.join(MANIFEST).is_file() {
            // Incomplete staging directory: a miss, not corruption.
            return Ok(None);
        }
        let m = self.read_manifest(&dir, digest)?;
        let route = match m.route.as_str() {
            "verdant-voice" => Route::VerdantVoice,
            "C0" if m.version == MANIFEST_VERSION => Route::C0,
            "C0" | "C1" | "A" => return Ok(None),
            _ => return Err(CacheError::Corrupt(format!("{digest}: bad route {}", m.route))),
        };
        let wav_decl = m
            .artifacts
            .iter()
            .find(|a| a.file == WAV_NAME)
            .ok_or_else(|| CacheError::Corrupt(format!("{digest}: manifest lists no {WAV_NAME}")))?;
        let wav_bytes = self.read_artifact(&dir, wav_decl)?;
        let (sample_rate, samples) =
            decode_wav(&wav_bytes).map_err(|e| CacheError::Corrupt(format!("{digest}/{WAV_NAME}: {e}")))?;
        let mut f0_tracks = Vec::new();
        let mut time_map = None;
        {
            let name = NPZ_NAME;
            let declarations: Vec<_> = m.artifacts.iter().filter(|a| a.file == name).collect();
            if declarations.len() > 1 {
                return Err(CacheError::Corrupt(format!("{digest}: repeated {name} artifact")));
            }
            if let Some(decl) = declarations.first() {
                let npz = self.read_artifact(&dir, decl)?;
                let decoded = read_pitch_from_npz(&npz).map_err(|e| CacheError::Corrupt(format!("{digest}/{name}: {e}")))?;
                if decoded.tracks.iter().any(|t| t.kind != F0Kind::NativeTarget) {
                    return Err(CacheError::Corrupt(format!("{digest}/{name}: F0 fields disagree with the cached source")));
                }
                time_map = decoded.time_map;
                f0_tracks.extend(decoded.tracks.into_iter().map(|track| VerifiedF0Track {
                    track, source: dir.join(name), sha256: decl.sha256.clone(),
                }));
            }
        }
        // No undeclared payload may silently ride along as playable.
        for a in &m.artifacts {
            if a.file != WAV_NAME && a.file != NPZ_NAME {
                return Err(CacheError::Corrupt(format!("{digest}: unexpected manifest artifact {}", a.file)));
            }
        }
        let slot = m.slot;
        Ok(Some(CacheEntry {
            digest: m.digest,
            job_id: m.job_id,
            revision: m.revision,
            route,
            audio: VerifiedAudio {
                path: dir.join(WAV_NAME),
                sha256: wav_decl.sha256.clone(),
                sample_rate,
                samples: samples.into(),
                f0_tracks,
                time_map,
            },
            track: slot.as_ref().map(|s| verdant_domain::TrackId::from(s.track.as_str())),
            phrase_index: slot.as_ref().map(|s| s.phrase_index),
            start_seconds: slot.as_ref().map(|s| s.start_seconds).unwrap_or(0.0),
            seconds: slot.as_ref().map(|s| s.seconds).unwrap_or(0.0),
            last_wanted_revision: m.revision,
        }))
    }
}

struct CacheWriter<'a> {
    dir: &'a Path,
    stage: PathBuf,
    _lock: File,
}

impl<'a> CacheWriter<'a> {
    fn begin(root: &Path, dir: &'a Path) -> io::Result<Self> {
        let locks = root.join(STATE).join("locks");
        let digest = dir.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing cache digest"))?;
        let lock = open_writer_lock(&locks, digest)?;
        lock.lock()?;
        let stage = dir.join(STAGE);
        remove_if_present(&stage)?;
        Ok(Self { dir, stage, _lock: lock })
    }

    fn write_atomic(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let mut file = File::create_new(&self.stage)?;
        file.write_all(bytes)?;
        self.publish(file, name)
    }

    fn publish(&self, file: File, name: &str) -> io::Result<()> {
        file.sync_all()?;
        drop(file);
        // Windows publishes with a write-through rename in place of a directory fsync.
        #[cfg(windows)]
        verdant_domain::io::replace_file_write_through(self.dir, &self.stage, &self.dir.join(name)).map_err(|error| match error {
            verdant_domain::io::IoError::Io { source, .. } => source,
            other => io::Error::new(io::ErrorKind::InvalidInput, other),
        })?;
        #[cfg(not(windows))]
        fs::rename(&self.stage, self.dir.join(name))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_open_preserves_live_stages_and_recovers_an_abandoned_writer() {
        let root = std::env::temp_dir().join(format!("verdant-cache-live-stages-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let cache = ArtifactCache::open(&root).unwrap();
        let a = cache.entry_dir("a");
        let b = cache.entry_dir("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        // The two production leases coexist. Pause at the exact interval between
        // create_new and rename, without scheduling or filesystem timing guesses.
        let live = CacheWriter::begin(&root, &a).unwrap();
        let interrupted = CacheWriter::begin(&root, &b).unwrap();
        // One 16 kHz, mono, signed 16-bit PCM frame at +0.25 full scale.
        let wav = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x80\x3e\0\0\0\x7d\0\0\x02\0\x10\0data\x02\0\0\0\0\x20";
        let mut file = File::create_new(&live.stage).unwrap();
        file.write_all(wav).unwrap();
        fs::write(&interrupted.stage, b"partial").unwrap();
        let reader = ArtifactCache::open(&root).unwrap();
        assert!(reader.restore("a").unwrap().is_none());
        assert!(interrupted.stage.is_file());

        // Run the released reader's actual two-level name-based sweep.
        for dir in fs::read_dir(&root).unwrap() {
            let dir = dir.unwrap();
            if dir.file_type().unwrap().is_dir() {
                for item in fs::read_dir(dir.path()).unwrap() {
                    let item = item.unwrap();
                    if item.file_name().to_string_lossy().contains(".tmp.") {
                        let _ = fs::remove_file(item.path());
                    }
                }
            }
        }
        assert!(interrupted.stage.is_file());
        let abandoned = interrupted.stage.clone();
        drop(interrupted); // A dead process releases its OS lock, leaving this same on-disk state.
        let reader = ArtifactCache::open(&root).unwrap();
        assert!(!abandoned.exists());
        assert!(reader.restore("b").unwrap().is_none());

        live.publish(file, WAV_NAME).unwrap();
        assert!(reader.restore("a").unwrap().is_none());
        let sha = hex::encode(Sha256::digest(wav));
        let manifest = serde_json::json!({
            "version": MANIFEST_VERSION, "digest": "a", "job_id": "job-a", "revision": 1,
            "route": "verdant-voice", "slot": null,
            "artifacts": [{"file": WAV_NAME, "sha256": sha, "bytes": wav.len()}]
        });
        live.write_atomic(MANIFEST, &serde_json::to_vec(&manifest).unwrap()).unwrap();
        drop(live);
        let restored = reader.restore("a").unwrap().unwrap();
        assert_eq!(restored.audio.sha256, sha);
        assert_eq!(restored.audio.samples.as_ref(), &[0.25]);
        fs::remove_dir_all(&root).unwrap();
    }
}
