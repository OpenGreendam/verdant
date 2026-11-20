use std::{collections::BTreeMap, fs::{self, File, Metadata}, io::{Read, Write}, path::{Path, PathBuf}};
use verdant_domain::{Project, TempoMap};
use crate::{AudioClip, AudioSource, DecodedAudio, ImportLimits, MediaError, PreparedClip, read_clips};
use crate::decode::decode_owned;

/// Authorization belongs to the caller (native file chooser/project asset
/// directory). Loading a project never automatically authorizes its paths.
#[derive(Clone, Debug)]
pub struct AssetPolicy { roots: Vec<PathBuf>, files: Vec<PathBuf>, project_dir: Option<PathBuf> }

impl AssetPolicy {
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> Result<Self, MediaError> {
        let mut canonical = Vec::new();
        for root in roots {
            let root = fs::canonicalize(root)?;
            if !root.is_dir() { return Err(MediaError::InvalidPath); }
            if !canonical.contains(&root) { canonical.push(root); }
        }
        Ok(Self { roots: canonical, files: Vec::new(), project_dir: None })
    }
    pub fn for_file(path: &Path) -> Result<Self, MediaError> {
        let mut policy = Self { roots: Vec::new(), files: Vec::new(), project_dir: None };
        policy.allow_file(path)?;
        Ok(policy)
    }
    /// Bind relative references to the selected native document's directory.
    /// Absolute sources retain the caller's explicit authorization policy.
    pub fn with_project_path(mut self, project_path: &Path) -> Result<Self, MediaError> {
        let parent = project_path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let parent = fs::canonicalize(parent)?;
        if !parent.is_dir() { return Err(MediaError::InvalidPath); }
        self.project_dir = Some(parent);
        Ok(self)
    }
    /// Add one explicitly selected user asset, without authorizing its siblings.
    pub fn allow_file(&mut self, path: &Path) -> Result<(), MediaError> {
        let path = fs::canonicalize(path)?;
        if !fs::metadata(&path)?.is_file() { return Err(MediaError::InvalidPath); }
        if !self.files.contains(&path) { self.files.push(path); }
        Ok(())
    }
    fn resolve(&self, path: &Path) -> Result<PathBuf, MediaError> {
        if !path.is_absolute() {
            let text = path.to_str().ok_or(MediaError::NonUnicodePath)?;
            if !crate::clip::portable_path(text) { return Err(MediaError::InvalidPath); }
            let root = self.project_dir.as_ref().ok_or(MediaError::UnauthorizedPath)?;
            let mut resolved = root.clone();
            for component in text.split('/') {
                resolved.push(component);
                if fs::symlink_metadata(&resolved)?.file_type().is_symlink() { return Err(MediaError::InvalidPath); }
            }
            let canonical = fs::canonicalize(&resolved)?;
            if canonical != resolved || !canonical.starts_with(root) { return Err(MediaError::UnauthorizedPath); }
            return Ok(canonical);
        }
        let canonical = fs::canonicalize(path)?;
        if !self.files.contains(&canonical) && !self.roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(MediaError::UnauthorizedPath);
        }
        Ok(canonical)
    }

    /// Open one authorized regular file with an ancestor-safe fd walk. The
    /// caller must bind bytes to its saved digest before using or publishing
    /// them; this method does not decode or allocate an audio-sized snapshot.
    pub fn open_file(&self, path: &Path) -> Result<File, MediaError> {
        let canonical = self.resolve(path)?;
        self.open_resolved(path, &canonical).map(|(file, _)| file)
    }

    fn open_resolved(&self, path: &Path, canonical: &Path) -> Result<(File, Metadata), MediaError> {
        let before = fs::metadata(canonical)?;
        if !before.is_file() { return Err(MediaError::InvalidPath); }
        let file = open_readonly(canonical)?;
        let opened = file.metadata()?;
        if !same_file(&before, &opened) || self.resolve(path)? != canonical {
            return Err(MediaError::SourceChanged);
        }
        Ok((file, opened))
    }
}

#[derive(Clone, Debug)]
pub struct ImportedAudio { pub source: AudioSource, pub audio: DecodedAudio }

pub struct MediaImporter { policy: AssetPolicy, limits: ImportLimits }

impl MediaImporter {
    pub fn new(policy: AssetPolicy, limits: ImportLimits) -> Self { Self { policy, limits } }
    pub fn limits(&self) -> ImportLimits { self.limits }
    /// Only call after an explicit native file selection/relink decision.
    pub fn allow_file(&mut self, path: &Path) -> Result<(), MediaError> { self.policy.allow_file(path) }

    /// Snapshot an authorized regular file once; hash and decode those exact
    /// bytes. Comparing opened-file identity prevents a replaced final path
    /// from being read under the authorization of a different file.
    pub fn import_file(&self, path: &Path) -> Result<ImportedAudio, MediaError> {
        self.import_with(path, |_| Ok(()))
    }

    /// Copy encoded source bytes to a private staging writer and verify them
    /// through the same decoder used by playback. Publish only after success:
    /// neither an arbitrary binary nor a changed audio format is an asset.
    pub fn copy_source(&self, source: &AudioSource, writer: &mut impl Write) -> Result<u64, MediaError> {
        source.validate()?;
        let imported = self.import_with(Path::new(&source.path), |bytes| {
            if bytes.len() as u64 != source.byte_len { return Err(MediaError::SourceChanged); }
            writer.write_all(bytes)?;
            Ok(())
        })?;
        source.matches(&imported.audio)?;
        Ok(imported.source.byte_len)
    }

    /// Verify an owned archive member without reopening it or copying the
    /// encoded snapshot. Source identity includes decoded rate and shape.
    pub fn verify_bytes(&self, source: &AudioSource, bytes: Vec<u8>) -> Result<(), MediaError> {
        source.validate()?;
        if bytes.len() as u64 != source.byte_len { return Err(MediaError::SourceChanged); }
        let extension = Path::new(&source.path).extension().and_then(|extension| extension.to_str());
        let audio = decode_owned(bytes, extension, self.limits)?;
        source.matches(&audio)
    }

    fn import_with(&self, path: &Path, snapshot: impl FnOnce(&[u8]) -> Result<(), MediaError>) -> Result<ImportedAudio, MediaError> {
        self.limits.validate()?;
        let canonical = self.policy.resolve(path)?;
        let (mut file, opened) = self.policy.open_resolved(path, &canonical)?;
        if opened.len() > self.limits.max_source_bytes { return Err(MediaError::Limit("source byte limit")); }
        let length = usize::try_from(opened.len()).map_err(|_| MediaError::Limit("source address space"))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| MediaError::Limit("source allocation"))?;
        let read_limit = self.limits.max_source_bytes.saturating_add(1);
        (&mut file).take(read_limit).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > self.limits.max_source_bytes { return Err(MediaError::Limit("source byte limit")); }
        let after = file.metadata()?;
        if !same_file(&opened, &after) || bytes.len() as u64 != opened.len()
            || self.policy.resolve(path)? != canonical || !same_file(&after, &fs::metadata(&canonical)?) {
            return Err(MediaError::SourceChanged);
        }
        snapshot(&bytes)?;
        let extension = canonical.extension().and_then(|s| s.to_str());
        let audio = decode_owned(bytes, extension, self.limits)?;
        let source = AudioSource { path: canonical.to_str().ok_or(MediaError::NonUnicodePath)?.into(),
            sha256: audio.sha256.clone(), byte_len: audio.byte_len, sample_rate: audio.sample_rate,
            channels: audio.channels, frames: audio.frames() };
        Ok(ImportedAudio { source, audio })
    }

    /// Reopen under the current authorization policy and compare content plus
    /// decoded format. A saved document is not authority to read an arbitrary file.
    pub fn load_source(&self, source: &AudioSource) -> Result<DecodedAudio, MediaError> {
        source.validate()?;
        let imported = self.import_file(Path::new(&source.path))?;
        source.matches(&imported.audio)?;
        Ok(imported.audio)
    }

    /// Prepare all saved clips in track order, sharing each verified decode.
    /// Engine rate is an explicit argument; 48 kHz is the editor's current rate.
    pub fn prepare_project_clips(&self, project: &Project, tempo: &TempoMap, engine_rate: u32) -> Result<Vec<PreparedClip>, MediaError> {
        self.prepare_project_clips_with(project, tempo, engine_rate, |_, _| {})
    }

    /// Same preparation, with a readonly hook that receives every clip and its
    /// verified source. The hook runs off the realtime thread before the caller
    /// publishes anything, and never copies the decoded PCM.
    pub fn prepare_project_clips_with(&self, project: &Project, tempo: &TempoMap, engine_rate: u32, mut on_clip: impl FnMut(&AudioClip, &DecodedAudio)) -> Result<Vec<PreparedClip>, MediaError> {
        self.limits.validate()?;
        if engine_rate == 0 || engine_rate > self.limits.max_sample_rate { return Err(MediaError::Invalid("engine sample rate")); }
        let mut decoded = BTreeMap::<(String, String), DecodedAudio>::new();
        let mut ids = std::collections::BTreeSet::new();
        let mut output = Vec::new();
        let mut bytes = 0usize;
        for (index, track) in project.tracks.iter().enumerate() {
            let clips = read_clips(track)?;
            if !clips.is_empty() && index >= verdant_audio::MAX_TRACKS { return Err(MediaError::Invalid("audio track exceeds mixer channel count")); }
            for clip in clips {
                if !ids.insert(clip.id) { return Err(MediaError::Invalid("duplicate project audio clip ID")); }
                // Authorization is checked even when another clip shares bytes.
                self.policy.resolve(Path::new(&clip.source.path))?;
                let key = (clip.source.path.clone(), clip.source.sha256.clone());
                if !decoded.contains_key(&key) {
                    let estimate = self.limits.pcm_len(clip.source.frames, clip.source.channels)?.checked_mul(4)
                        .ok_or(MediaError::Limit("project PCM address space"))?;
                    if bytes.checked_add(estimate).is_none_or(|n| n > self.limits.max_project_pcm_bytes) {
                        return Err(MediaError::Limit("project PCM byte limit"));
                    }
                    let audio = self.load_source(&clip.source)?;
                    bytes += estimate;
                    if bytes > self.limits.max_project_pcm_bytes { return Err(MediaError::Limit("project PCM byte limit")); }
                    decoded.insert(key.clone(), audio);
                }
                let audio = &decoded[&key];
                on_clip(&clip, audio);
                let frames = (clip.duration_frames as f64 * f64::from(engine_rate) / f64::from(audio.sample_rate)).round() as u64;
                let channels = if audio.channels == 1 && clip.pan != 0.0 { 2 } else { audio.channels };
                let shared_full_source = engine_rate == audio.sample_rate && clip.source_offset_frames == 0
                    && clip.duration_frames == audio.frames() && clip.pan == 0.0
                    && clip.fade_in_frames == 0 && clip.fade_out_frames == 0;
                let estimate = if shared_full_source { 0 } else {
                    self.limits.pcm_len(frames, channels)?.checked_mul(4)
                        .ok_or(MediaError::Limit("project PCM address space"))?
                };
                if bytes.checked_add(estimate).is_none_or(|n| n > self.limits.max_project_pcm_bytes) {
                    return Err(MediaError::Limit("project PCM byte limit"));
                }
                let prepared = crate::clip::prepare_with_limits(&clip, audio, tempo, engine_rate, index as u16, self.limits)?;
                bytes = bytes.checked_add(estimate)
                    .ok_or(MediaError::Limit("project PCM address space"))?;
                if bytes > self.limits.max_project_pcm_bytes { return Err(MediaError::Limit("project PCM byte limit")); }
                output.push(prepared);
            }
        }
        output.sort_by_key(|clip| (clip.start, clip.id));
        Ok(output)
    }
}

fn same_file(a: &Metadata, b: &Metadata) -> bool {
    if !a.is_file() || !b.is_file() || a.len() != b.len() || a.modified().ok() != b.modified().ok() { return false; }
    #[cfg(unix)] {
        use std::os::unix::fs::MetadataExt;
        if a.dev() != b.dev() || a.ino() != b.ino() || a.ctime() != b.ctime() || a.ctime_nsec() != b.ctime_nsec() { return false; }
    }
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        if a.creation_time() != b.creation_time() || a.last_write_time() != b.last_write_time()
            || a.file_attributes() != b.file_attributes() { return false; }
    }
    true
}

/// Walk from an identity-checked directory fd. Every component is opened
/// relative to the still-owned parent, so replacing an ancestor by a symlink
/// cannot redirect this open. Nonblocking final open also makes FIFO swaps
/// rejectable by fstat instead of hanging before the regular-file check.
#[cfg(unix)]
fn open_readonly(path: &Path) -> Result<File, MediaError> {
    use std::{ffi::CString, fs::OpenOptions, os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt}}}, path::Component};
    let root_metadata = fs::metadata("/")?;
    let mut parent = OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC).open("/")?;
    let opened_root = parent.metadata()?;
    if !opened_root.is_dir() || root_metadata.dev() != opened_root.dev() || root_metadata.ino() != opened_root.ino() {
        return Err(MediaError::SourceChanged);
    }
    let mut components = path.strip_prefix("/").map_err(|_| MediaError::InvalidPath)?.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else { return Err(MediaError::InvalidPath); };
        let name = CString::new(name.as_bytes()).map_err(|_| MediaError::InvalidPath)?;
        let last = components.peek().is_none();
        let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC
            | if last { libc::O_NONBLOCK } else { libc::O_DIRECTORY };
        // SAFETY: the parent fd is live, name is NUL terminated, flags do not
        // create files. A successful returned fd is immediately RAII-owned.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
        // SAFETY: successful openat returned a fresh, uniquely owned fd.
        let opened = unsafe { File::from_raw_fd(fd) };
        if last {
            if !opened.metadata()?.is_file() { return Err(MediaError::InvalidPath); }
            return Ok(opened);
        }
        parent = opened;
    }
    Err(MediaError::InvalidPath)
}

#[cfg(windows)]
fn open_readonly(path: &Path) -> Result<File, MediaError> {
    use std::{fs::OpenOptions, os::windows::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt}, io::AsRawHandle}};
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, GetFinalPathNameByHandleW};
    let file = OpenOptions::new().read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(MediaError::InvalidPath);
    }
    let capacity = path.as_os_str().encode_wide().count().checked_add(1).ok_or(MediaError::InvalidPath)?;
    let capacity_u32 = u32::try_from(capacity).map_err(|_| MediaError::InvalidPath)?;
    let mut short_name = [0u16; 1024];
    let mut long_name = Vec::new();
    let final_name = if capacity <= short_name.len() { &mut short_name[..capacity] } else {
        long_name.resize(capacity, 0u16);
        long_name.as_mut_slice()
    };
    // The policy supplied a canonical DOS/verbatim path. Verify the actual
    // opened handle before reading, so an ancestor junction/symlink swap cannot
    // borrow the authorization of that earlier pathname.
    let length = unsafe { GetFinalPathNameByHandleW(file.as_raw_handle(), final_name.as_mut_ptr(), capacity_u32, 0) };
    if length == 0 { return Err(std::io::Error::last_os_error().into()); }
    if length as usize >= capacity || !path.as_os_str().encode_wide().eq(final_name[..length as usize].iter().copied()) {
        return Err(MediaError::SourceChanged);
    }
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
fn open_readonly(path: &Path) -> Result<File, MediaError> { Ok(File::open(path)?) }

#[cfg(all(test, unix))]
mod open_tests {
    use super::*;
    #[test]
    fn stale_resolved_ancestor_cannot_redirect_the_fd_walk() {
        let root = std::env::temp_dir().join(format!("verdant-media-open-test-{}", uuid::Uuid::new_v4()));
        struct OwnedScratch(PathBuf);
        impl Drop for OwnedScratch { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
        fs::create_dir(&root).unwrap(); let _owned = OwnedScratch(root.clone());
        let inside = root.join("inside"); let outside = root.join("outside");
        fs::create_dir(&inside).unwrap(); fs::create_dir(&outside).unwrap();
        fs::write(inside.join("audio"), b"inside").unwrap();
        fs::write(outside.join("audio"), b"outside").unwrap();
        let stale = fs::canonicalize(inside.join("audio")).unwrap();
        fs::rename(&inside, root.join("retained")).unwrap();
        std::os::unix::fs::symlink(&outside, &inside).unwrap();
        // This reconstructs the previous resolve-then-open failure on owned
        // fixture bytes; the production walk rejects the swapped ancestor.
        assert_eq!(fs::read(&stale).unwrap(), b"outside");
        for _ in 0..16 { assert!(open_readonly(&stale).is_err()); }
    }
}
