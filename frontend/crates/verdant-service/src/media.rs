//! Project-relative media authorization shared by desktop and headless callers.
//! Persisted references stay relative; only the file-open policy carries the
//! selected document's directory. MediaImporter verifies bytes and audio shape
//! against AudioSource before either consumer can use the decoded PCM.

use std::{collections::BTreeMap, ffi::OsString, fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Component, Path, PathBuf}};

use verdant_domain::{Project, ProjectStore, store::MetaPatch};
use verdant_media::{AUDIO_CLIPS_KEY, AssetPolicy, AudioSource, ImportLimits, MediaError, MediaImporter};

pub fn project_policy(project_path: &Path, policy: AssetPolicy) -> Result<AssetPolicy, MediaError> {
    policy.with_project_path(project_path)
}

/// Copy one authorized source to private staging. The native importer binds the
/// exact encoded bytes and decoded shape to the saved source identity. Nothing
/// written here should be published until this function returns successfully.
pub fn copy_source(source: &AudioSource, importer: &MediaImporter, writer: &mut impl Write, limit: u64) -> Result<u64, MediaError> {
    if source.byte_len > limit { return Err(MediaError::Limit("portable source byte limit")); }
    if importer.limits().max_source_bytes > limit { return Err(MediaError::Invalid("portable importer exceeds byte limit")); }
    importer.copy_source(source, writer)
}

#[derive(Debug, Default, serde::Serialize)]
pub struct AssetCopyReport {
    pub same_base: bool,
    pub relative_assets: usize,
    pub copied_assets: usize,
    pub reused_assets: usize,
    pub byte_len: u64,
    pub dry_run: bool,
}

struct CopyItem { source: AudioSource, existing: bool }
struct CopyPlan {
    items: Vec<CopyItem>,
    destination: Option<TargetDirectory>,
    source: MediaImporter,
    output_path: PathBuf,
    report: AssetCopyReport,
}

/// Owned media metadata for a background Save As. Construction never decodes
/// or copies media; copy/preflight perform the actual byte admission later.
pub struct ProjectAssetCopy {
    source_path: PathBuf,
    output_path: PathBuf,
    sources: BTreeMap<String, AudioSource>,
    same_base: bool,
}

impl ProjectAssetCopy {
    pub fn new(store: &ProjectStore, source_path: &Path, output_path: &Path) -> Result<Self, MediaError> {
        Self::capture(source_path, output_path, || retained_sources(store))
    }

    /// Capture only the exported current document, excluding media reachable
    /// solely through history that the recipient will not receive.
    pub fn current(project: &Project, source_path: &Path, output_path: &Path) -> Result<Self, MediaError> {
        Self::capture(source_path, output_path, || {
            let mut sources = BTreeMap::new();
            for track in &project.tracks {
                for clip in verdant_media::read_clips(track)? { retain_source(&mut sources, clip.source)?; }
            }
            Ok(sources)
        })
    }

    fn capture(source_path: &Path, output_path: &Path, collect: impl FnOnce() -> Result<BTreeMap<String, AudioSource>, MediaError>) -> Result<Self, MediaError> {
        let source_base = fs::canonicalize(project_parent(source_path))?;
        let same_base = match fs::canonicalize(project_parent(output_path)) {
            Ok(base) => base == source_base,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        let sources = if same_base { BTreeMap::new() } else { collect()? };
        Ok(Self { source_path: source_path.into(), output_path: output_path.into(), sources, same_base })
    }

    pub fn requires_copy(&self) -> bool { !self.same_base && !self.sources.is_empty() }

    pub fn preflight(self) -> Result<AssetCopyReport, MediaError> {
        let mut plan = plan_copy(self)?;
        for item in &plan.items {
            if !item.existing { plan.source.load_source(&item.source)?; }
        }
        plan.report.dry_run = true;
        Ok(plan.report)
    }

    pub fn copy(self) -> Result<AssetCopyReport, MediaError> { execute_copy(self) }
}

/// Read-only Save As admission, including every relative audio source retained
/// by undo and redo. No output directory, staging file or document is created.
pub fn preflight_project_assets(store: &ProjectStore, source_path: &Path, output_path: &Path) -> Result<AssetCopyReport, MediaError> {
    ProjectAssetCopy::new(store, source_path, output_path)?.preflight()
}

/// Preserve relative references across Save As without editing any project or
/// history entry. Verify all existing targets and stage every required source
/// before publishing an asset. Each publication is atomic and refuses clobber;
/// a raced existing file is accepted only after full native identity checking.
/// A later document-publication failure may leave verified, unreferenced assets.
pub fn copy_project_assets(store: &ProjectStore, source_path: &Path, output_path: &Path) -> Result<AssetCopyReport, MediaError> {
    ProjectAssetCopy::new(store, source_path, output_path)?.copy()
}

fn execute_copy(request: ProjectAssetCopy) -> Result<AssetCopyReport, MediaError> {
    let mut plan = plan_copy(request)?;
    if plan.report.same_base || plan.items.iter().all(|item| item.existing) { return Ok(plan.report); }
    let root = match plan.destination.take() {
        Some(root) => root,
        None => {
            fs::create_dir_all(project_parent(&plan.output_path))?;
            TargetDirectory::open_root(project_parent(&plan.output_path))?
        }
    };
    root.check_identity()?;
    let mut staging = StagingArea::new(&root)?;
    let mut staged = Vec::new();
    for item in &plan.items {
        if item.existing { continue; }
        let mut file = staging.new_file()?;
        copy_source(&item.source, &plan.source, &mut file.file, ImportLimits::default().max_source_bytes)?;
        file.file.sync_all()?;
        staged.push((&item.source, file));
    }
    root.check_identity()?;
    let verifier = MediaImporter::new(AssetPolicy::new([])?, ImportLimits::default());
    for (source, file) in &staged {
        let parent = asset_parent(&root, &source.path, true)?.ok_or(MediaError::InvalidPath)?;
        let name = Path::new(&source.path).file_name().ok_or(MediaError::InvalidPath)?;
        parent.check_identity()?;
        match publish_asset(&staging.directory, &file.name, &parent, name) {
            Ok(()) => { plan.report.copied_assets += 1; }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = parent.open_file(name)?.ok_or(MediaError::SourceChanged)?;
                verify_file(&verifier, source, existing)?;
                plan.report.reused_assets += 1;
            }
            Err(error) => return Err(error.into()),
        }
        parent.check_identity()?;
        parent.sync()?;
    }
    root.check_identity()?;
    drop(staged);
    drop(staging);
    root.sync()?;
    Ok(plan.report)
}

fn plan_copy(request: ProjectAssetCopy) -> Result<CopyPlan, MediaError> {
    let source_base = fs::canonicalize(project_parent(&request.source_path))?;
    let destination = match TargetDirectory::open_root(project_parent(&request.output_path)) {
        Ok(root) => Some(root),
        Err(MediaError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let importer = MediaImporter::new(project_policy(&request.source_path, AssetPolicy::new([])?)?, ImportLimits::default());
    if destination.as_ref().is_some_and(|root| root.path == source_base) {
        return Ok(CopyPlan { items: Vec::new(), destination, source: importer, output_path: request.output_path,
            report: AssetCopyReport { same_base: true, ..Default::default() } });
    }
    if request.same_base { return Err(MediaError::SourceChanged); }
    let verifier = MediaImporter::new(AssetPolicy::new([])?, ImportLimits::default());
    let mut items = Vec::with_capacity(request.sources.len());
    let mut report = AssetCopyReport::default();
    for source in request.sources.into_values() {
        report.byte_len = report.byte_len.checked_add(source.byte_len).ok_or(MediaError::Limit("asset copy byte count"))?;
        let existing = if let Some(root) = &destination {
            if let Some(parent) = asset_parent(root, &source.path, false)? {
                if let Some(file) = parent.open_file(Path::new(&source.path).file_name().ok_or(MediaError::InvalidPath)?)? {
                    verify_file(&verifier, &source, file)?;
                    true
                } else { false }
            } else { false }
        } else { false };
        if existing { report.reused_assets += 1; }
        items.push(CopyItem { source, existing });
    }
    report.relative_assets = items.len();
    Ok(CopyPlan { items, destination, source: importer, output_path: request.output_path, report })
}

fn retained_sources(store: &ProjectStore) -> Result<BTreeMap<String, AudioSource>, MediaError> {
    let mut sources = BTreeMap::<String, AudioSource>::new();
    for track in &store.project().tracks {
        for clip in verdant_media::read_clips(track)? { retain_source(&mut sources, clip.source)?; }
    }
    for entry in store.retained_history() {
        for patch in &entry.meta {
            match patch {
                MetaPatch::TrackSet { before, after, .. } => {
                    for track in before.iter().chain(after) {
                        for clip in verdant_media::read_clips(track)? { retain_source(&mut sources, clip.source)?; }
                    }
                }
                MetaPatch::TrackUnknownKey { track, key, before, after } if key == AUDIO_CLIPS_KEY => {
                    for value in before.iter().chain(after) {
                        for clip in verdant_media::read_clips_value(track, value)? { retain_source(&mut sources, clip.source)?; }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(sources)
}

fn retain_source(sources: &mut BTreeMap<String, AudioSource>, source: AudioSource) -> Result<(), MediaError> {
    if Path::new(&source.path).is_absolute() { return Ok(()); }
    source.validate()?;
    if source.byte_len > ImportLimits::default().max_source_bytes { return Err(MediaError::Limit("source byte limit")); }
    if let Some(existing) = sources.get(&source.path) {
        if existing != &source { return Err(MediaError::SourceChanged); }
    } else {
        sources.insert(source.path.clone(), source);
    }
    Ok(())
}

fn project_parent(path: &Path) -> &Path {
    path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."))
}

fn verify_file(importer: &MediaImporter, source: &AudioSource, mut file: File) -> Result<(), MediaError> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() != source.byte_len { return Err(MediaError::SourceChanged); }
    let capacity = usize::try_from(source.byte_len + 1).map_err(|_| MediaError::Limit("source address space"))?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_| MediaError::Limit("source allocation"))?;
    (&mut file).take(source.byte_len + 1).read_to_end(&mut bytes)?;
    importer.verify_bytes(source, bytes)
}

struct TargetDirectory { file: File, path: PathBuf }

impl TargetDirectory {
    fn open_root(path: &Path) -> Result<Self, MediaError> {
        let path = fs::canonicalize(path)?;
        let file = open_directory(&path)?;
        let root = Self { file, path };
        root.check_identity()?;
        Ok(root)
    }

    fn check_identity(&self) -> Result<(), MediaError> {
        let opened = self.file.metadata()?;
        let named = fs::symlink_metadata(&self.path)?;
        if !named.is_dir() || named.file_type().is_symlink() { return Err(MediaError::SourceChanged); }
        #[cfg(unix)] {
            use std::os::unix::fs::MetadataExt;
            if opened.dev() != named.dev() || opened.ino() != named.ino() { return Err(MediaError::SourceChanged); }
        }
        #[cfg(windows)] {
            check_windows_path(&self.file, &self.path)?;
            if opened.created().ok() != named.created().ok() { return Err(MediaError::SourceChanged); }
        }
        Ok(())
    }

    fn child(&self, name: &std::ffi::OsStr, create: bool) -> Result<Option<Self>, MediaError> {
        let path = self.path.join(name);
        #[cfg(unix)] {
            use std::{ffi::CString, os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt}};
            let name = CString::new(name.as_bytes()).map_err(|_| MediaError::InvalidPath)?;
            let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
            let mut fd = unsafe { libc::openat(self.file.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                if !create { return Ok(None); }
                if unsafe { libc::mkdirat(self.file.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(std::io::Error::last_os_error().into());
                }
                self.sync()?;
                fd = unsafe { libc::openat(self.file.as_raw_fd(), name.as_ptr(), flags) };
            }
            if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
            return Ok(Some(Self { file: unsafe { File::from_raw_fd(fd) }, path }));
        }
        #[cfg(not(unix))] {
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => return Err(MediaError::InvalidPath),
                Ok(_) => {},
                Err(error) if error.kind() == std::io::ErrorKind::NotFound && !create => return Ok(None),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    match fs::create_dir(&path) { Ok(()) => {}, Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}, Err(error) => return Err(error.into()) }
                }
                Err(error) => return Err(error.into()),
            }
            let file = open_directory(&path)?;
            #[cfg(windows)]
            check_windows_path(&file, &path)?;
            Ok(Some(Self { file, path }))
        }
    }

    fn private_child(&self, name: &std::ffi::OsStr) -> Result<Self, MediaError> {
        #[cfg(unix)] {
            use std::{ffi::CString, os::{fd::AsRawFd, unix::ffi::OsStrExt}};
            let name = CString::new(name.as_bytes()).map_err(|_| MediaError::InvalidPath)?;
            if unsafe { libc::mkdirat(self.file.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        #[cfg(not(unix))]
        fs::create_dir(self.path.join(name))?;
        let result = self.child(name, false).and_then(|child| {
            let child = child.ok_or(MediaError::InvalidPath)?;
            #[cfg(unix)] {
                use std::os::unix::fs::MetadataExt;
                let metadata = child.file.metadata()?;
                if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
                    return Err(MediaError::SourceChanged);
                }
            }
            Ok(child)
        });
        if result.is_err() { remove_child(self, name, true); }
        result
    }

    fn open_file(&self, name: &std::ffi::OsStr) -> Result<Option<File>, MediaError> {
        #[cfg(unix)] {
            use std::{ffi::CString, os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt}};
            let name = CString::new(name.as_bytes()).map_err(|_| MediaError::InvalidPath)?;
            let fd = unsafe { libc::openat(self.file.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC) };
            if fd < 0 {
                let error = std::io::Error::last_os_error();
                return if error.kind() == std::io::ErrorKind::NotFound { Ok(None) } else { Err(error.into()) };
            }
            let file = unsafe { File::from_raw_fd(fd) };
            if !file.metadata()?.is_file() { return Err(MediaError::InvalidPath); }
            return Ok(Some(file));
        }
        #[cfg(not(unix))] {
            let path = self.path.join(name);
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(windows)] {
                use std::os::windows::fs::OpenOptionsExt;
                options.custom_flags(0x0020_0000);
            }
            let file = match options.open(&path) { Ok(file) => file, Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None), Err(error) => return Err(error.into()) };
            if !file.metadata()?.is_file() { return Err(MediaError::InvalidPath); }
            #[cfg(windows)]
            check_windows_path(&file, &path)?;
            Ok(Some(file))
        }
    }

    fn sync(&self) -> Result<(), MediaError> {
        #[cfg(unix)]
        self.file.sync_all()?;
        Ok(())
    }
}

fn asset_parent(root: &TargetDirectory, relative: &str, create: bool) -> Result<Option<TargetDirectory>, MediaError> {
    let mut components = Path::new(relative).components().peekable();
    let mut parent = None::<TargetDirectory>;
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else { return Err(MediaError::InvalidPath); };
        if components.peek().is_none() {
            return Ok(Some(match parent { Some(parent) => parent, None => TargetDirectory { file: root.file.try_clone()?, path: root.path.clone() } }));
        }
        let Some(child) = parent.as_ref().unwrap_or(root).child(name, create)? else { return Ok(None); };
        parent = Some(child);
    }
    Err(MediaError::InvalidPath)
}

struct StagedFile { name: OsString, file: File }
struct StagingArea<'a> { root: &'a TargetDirectory, name: OsString, directory: TargetDirectory, names: Vec<OsString> }

impl<'a> StagingArea<'a> {
    fn new(root: &'a TargetDirectory) -> Result<Self, MediaError> {
        let name = OsString::from(format!(".verdant-media-{}", uuid::Uuid::new_v4()));
        let directory = root.private_child(&name)?;
        Ok(Self { root, name, directory, names: Vec::new() })
    }

    fn new_file(&mut self) -> Result<StagedFile, MediaError> {
        let name = OsString::from(format!("{}.tmp", uuid::Uuid::new_v4()));
        #[cfg(unix)]
        let file = {
            use std::{ffi::CString, os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt}};
            let name = CString::new(name.as_bytes()).map_err(|_| MediaError::InvalidPath)?;
            let fd = unsafe { libc::openat(self.directory.file.as_raw_fd(), name.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600) };
            if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
            unsafe { File::from_raw_fd(fd) }
        };
        #[cfg(not(unix))]
        let file = {
            let path = self.directory.path.join(&name);
            let file = OpenOptions::new().read(true).write(true).create_new(true).open(&path)?;
            #[cfg(windows)]
            check_windows_path(&file, &path)?;
            file
        };
        self.names.push(name.clone());
        Ok(StagedFile { name, file })
    }
}

impl Drop for StagingArea<'_> {
    fn drop(&mut self) {
        for name in &self.names { remove_child(&self.directory, name, false); }
        remove_child(self.root, &self.name, true);
    }
}

#[cfg(unix)]
fn open_directory(path: &Path) -> Result<File, MediaError> {
    use std::{ffi::CString, os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::OpenOptionsExt}}};
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let mut directory = OpenOptions::new().read(true).custom_flags(flags & !libc::O_RDONLY).open("/")?;
    for component in path.components() {
        let component = match component { Component::RootDir => continue, Component::Normal(name) => name, _ => return Err(MediaError::InvalidPath) };
        let name = CString::new(component.as_bytes()).map_err(|_| MediaError::InvalidPath)?;
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
        directory = unsafe { File::from_raw_fd(fd) };
    }
    Ok(directory)
}

#[cfg(windows)]
fn open_directory(path: &Path) -> Result<File, MediaError> {
    use std::os::windows::fs::OpenOptionsExt;
    let file = OpenOptions::new().read(true).custom_flags(0x0200_0000 | 0x0020_0000).open(path)?;
    check_windows_path(&file, path)?;
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
fn open_directory(path: &Path) -> Result<File, MediaError> { Ok(File::open(path)?) }

#[cfg(unix)]
fn publish_asset(stage: &TargetDirectory, temporary: &std::ffi::OsStr, parent: &TargetDirectory, name: &std::ffi::OsStr) -> std::io::Result<()> {
    use std::{ffi::CString, os::{fd::AsRawFd, unix::ffi::OsStrExt}};
    let temporary = CString::new(temporary.as_bytes()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    let name = CString::new(name.as_bytes()).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    let result = unsafe { libc::linkat(stage.file.as_raw_fd(), temporary.as_ptr(), parent.file.as_raw_fd(), name.as_ptr(), 0) };
    if result != 0 { return Err(std::io::Error::last_os_error()); }
    Ok(())
}

#[cfg(not(unix))]
fn publish_asset(stage: &TargetDirectory, temporary: &std::ffi::OsStr, parent: &TargetDirectory, name: &std::ffi::OsStr) -> std::io::Result<()> {
    fs::hard_link(stage.path.join(temporary), parent.path.join(name))
}

fn remove_child(parent: &TargetDirectory, name: &std::ffi::OsStr, directory: bool) {
    #[cfg(unix)] {
        use std::{ffi::CString, os::{fd::AsRawFd, unix::ffi::OsStrExt}};
        if let Ok(name) = CString::new(name.as_bytes()) {
            unsafe { libc::unlinkat(parent.file.as_raw_fd(), name.as_ptr(), if directory { libc::AT_REMOVEDIR } else { 0 }); }
        }
    }
    #[cfg(not(unix))] {
        let path = parent.path.join(name);
        let _ = if directory { fs::remove_dir(path) } else { fs::remove_file(path) };
    }
}

#[cfg(windows)]
fn check_windows_path(file: &File, path: &Path) -> Result<(), MediaError> {
    use std::os::windows::{ffi::OsStrExt, fs::MetadataExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, GetFinalPathNameByHandleW};
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 { return Err(MediaError::InvalidPath); }
    let capacity = path.as_os_str().encode_wide().count().checked_add(1).ok_or(MediaError::InvalidPath)?;
    let mut short_name = [0u16; 1024];
    let mut long_name = Vec::new();
    let name = if capacity <= short_name.len() { &mut short_name[..capacity] } else {
        long_name.resize(capacity, 0u16);
        long_name.as_mut_slice()
    };
    let length = unsafe { GetFinalPathNameByHandleW(file.as_raw_handle(), name.as_mut_ptr(), u32::try_from(capacity).map_err(|_| MediaError::InvalidPath)?, 0) };
    if length == 0 { return Err(std::io::Error::last_os_error().into()); }
    if length as usize >= capacity || !path.as_os_str().encode_wide().eq(name[..length as usize].iter().copied()) {
        return Err(MediaError::SourceChanged);
    }
    Ok(())
}
