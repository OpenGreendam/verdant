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
//! * **Crash-safe writes.** Artifacts are written to `*.tmp.*` siblings,
//!   fsynced, then renamed into place; the manifest is committed last, so a
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

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::artifact::{decode_wav, read_pitch_from_npz};
use crate::scheduler::{CacheEntry, F0Kind, VerifiedAudio, VerifiedF0Track};
use crate::wire::Route;

const MANIFEST: &str = "manifest.json";
const WAV_NAME: &str = "native.wav";
const NPZ_NAME: &str = "native.npz";
const MANIFEST_VERSION: u32 = 3;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    /// Open (creating) the cache directory. Stale `*.tmp.*` staging files
    /// from interrupted writes are removed; entries without a manifest are
    /// inert and simply missed by `entries`/`restore`.
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        for dir in fs::read_dir(&root)? {
            let dir = dir?;
            if !dir.file_type()?.is_dir() {
                continue;
            }
            for item in fs::read_dir(dir.path())? {
                let item = item?;
                if item.file_name().to_string_lossy().contains(".tmp.") {
                    let _ = fs::remove_file(item.path());
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

    fn staging(&self, dir: &Path, name: &str) -> PathBuf {
        dir.join(format!(".{name}.tmp.{}.{}", std::process::id(), TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)))
    }

    /// Write `bytes` to `staging`, fsync, then rename onto `final` (atomic
    /// replace of a single file).
    fn write_atomic(&self, dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
        let tmp = self.staging(dir, name);
        fs::write(&tmp, bytes)?;
        fs::File::open(&tmp)?.sync_all()?;
        fs::rename(&tmp, dir.join(name))?;
        Ok(())
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        fs::File::open(dir)?.sync_all()
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
        self.write_atomic(&dir, WAV_NAME, &wav)?;
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
            self.write_atomic(&dir, name, &npz)?;
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
        self.write_atomic(&dir, MANIFEST, &body)?;
        self.sync_dir(&dir)?;
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
