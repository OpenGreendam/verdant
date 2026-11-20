use std::{collections::BTreeMap, fs::{self, File, Metadata}, io::{Read, Write}, path::{Component, Path, PathBuf}};
use sha2::{Digest, Sha256};
use crate::{Error, Result, platform, require};

pub(crate) fn path_text(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| Error::Invalid("voice installation paths must be valid UTF-8".into()))
}

pub(crate) fn present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn regular(path: &Path) -> Result<Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    require(metadata.is_file() && !metadata.file_type().is_symlink() && !platform::reparse(&metadata),
        format!("expected regular nonsymlink file: {}", path.display()))?;
    Ok(metadata)
}

pub(crate) fn directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    require(metadata.is_dir() && !metadata.file_type().is_symlink() && !platform::reparse(&metadata),
        format!("expected regular nonsymlink directory: {}", path.display()))
}

pub(crate) fn same_file(a: &Metadata, b: &Metadata) -> bool {
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

/// Open each component relative to a still-owned directory fd. A stale
/// canonical path cannot redirect a source read through a replaced ancestor.
#[cfg(unix)]
fn open_absolute(path: &Path) -> Result<File> {
    use std::{ffi::CString, fs::OpenOptions, os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt}}}};
    let root = fs::metadata("/")?;
    let mut parent = OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC).open("/")?;
    let opened_root = parent.metadata()?;
    require(opened_root.is_dir() && root.dev() == opened_root.dev() && root.ino() == opened_root.ino(), "file root changed during open")?;
    let mut parts = path.strip_prefix("/").map_err(|_| Error::Invalid("file path must be absolute".into()))?.components().peekable();
    while let Some(part) = parts.next() {
        let Component::Normal(name) = part else { return Err(Error::Invalid("file path contains noncanonical components".into())); };
        let name = CString::new(name.as_bytes()).map_err(|_| Error::Invalid("file path contains NUL".into()))?;
        let last = parts.peek().is_none();
        let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC
            | if last { libc::O_NONBLOCK } else { libc::O_DIRECTORY };
        // SAFETY: parent owns its fd and name is NUL terminated; no file is created.
        let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
        // SAFETY: openat returned a fresh fd, transferred immediately to RAII.
        let file = unsafe { File::from_raw_fd(fd) };
        if last { return Ok(file); }
        parent = file;
    }
    Err(Error::Invalid("expected a file path".into()))
}

#[cfg(windows)]
fn open_absolute(path: &Path) -> Result<File> {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE};
    // Keep every parent open without FILE_SHARE_DELETE until the leaf is open.
    // This prevents a junction/ancestor rename from redirecting the final open.
    let mut parents = Vec::new();
    let mut current = PathBuf::new();
    let mut parts = path.components().peekable();
    while let Some(part) = parts.next() {
        current.push(part);
        if matches!(part, Component::Prefix(_)) { continue; }
        let last = parts.peek().is_none();
        let file = OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | if last { 0 } else { FILE_FLAG_BACKUP_SEMANTICS })
            .open(&current)?;
        let metadata = file.metadata()?;
        require(!platform::reparse(&metadata) && if last { metadata.is_file() } else { metadata.is_dir() },
            "file path contains a reparse point or unexpected file type")?;
        if last { return Ok(file); }
        parents.push(file);
    }
    Err(Error::Invalid("expected an absolute file path".into()))
}

#[cfg(not(any(unix, windows)))]
fn open_absolute(_: &Path) -> Result<File> { Err(Error::Invalid("unsupported installation platform".into())) }

pub(crate) fn open_regular(path: &Path) -> Result<File> {
    let before = regular(path)?;
    let parent = fs::canonicalize(path.parent().ok_or_else(|| Error::Invalid("file needs a parent directory".into()))?)?;
    let canonical = parent.join(path.file_name().ok_or_else(|| Error::Invalid("file needs a name".into()))?);
    let file = open_absolute(&canonical)?;
    let opened = file.metadata()?;
    require(same_file(&before, &opened) && !platform::reparse(&opened), format!("file changed during open: {}", path.display()))?;
    require(matches_path(&file, path)?, format!("file identity changed during open: {}", path.display()))?;
    Ok(file)
}

pub(crate) fn matches_path(file: &File, path: &Path) -> Result<bool> {
    let metadata = regular(path)?;
    if !same_file(&metadata, &file.metadata()?) { return Ok(false); }
    #[cfg(windows)] {
        let parent = fs::canonicalize(path.parent().ok_or_else(|| Error::Invalid("file needs a parent".into()))?)?;
        let current = open_absolute(&parent.join(path.file_name().ok_or_else(|| Error::Invalid("file needs a name".into()))?))?;
        return platform::windows::same_open_file(file, &current);
    }
    #[cfg(not(windows))] { Ok(true) }
}

pub(crate) fn read(path: &Path) -> Result<Vec<u8>> {
    let mut file = open_regular(path)?;
    let before = file.metadata()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    require(same_file(&before, &file.metadata()?) && matches_path(&file, path)?,
        format!("file changed during read: {}", path.display()))?;
    Ok(bytes)
}

fn hash_and_size(path: &Path, limit: u64) -> Result<(String, u64)> {
    let mut file = open_regular(path)?;
    let before = file.metadata()?;
    require(before.len() <= limit, format!("voice package file exceeds its declared size: {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    let mut reader = (&mut file).take(limit.saturating_add(1));
    let mut size = 0;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 { break; }
        digest.update(&buffer[..count]);
        size += count as u64;
    }
    require(size == before.len() && same_file(&before, &file.metadata()?) && matches_path(&file, path)?,
        format!("file changed during hashing: {}", path.display()))?;
    Ok((hex::encode(digest.finalize()), size))
}

pub(crate) fn digest(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)] {
        use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};
        let resolved = fs::canonicalize(path)?;
        OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC).open(resolved)?.sync_all()?;
    }
    #[cfg(not(unix))] { let _ = path; }
    Ok(())
}

pub(crate) fn sync_file(path: &Path) -> Result<()> {
    #[cfg(windows)] {
        regular(path)?;
        fs::OpenOptions::new().read(true).write(true).open(path)?.sync_all()?;
    }
    #[cfg(not(windows))] { open_regular(path)?.sync_all()?; }
    Ok(())
}

pub(crate) fn atomic_bytes(path: &Path, bytes: &[u8], mode: u32, replace: bool) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::Invalid("atomic file needs a parent".into()))?;
    let resolved_parent = fs::canonicalize(parent)?;
    directory(&resolved_parent)?;
    let mut output = tempfile::Builder::new().prefix(".verdant-voice-write-").tempfile_in(&resolved_parent)?;
    platform::set_mode(output.path(), mode)?;
    output.write_all(bytes)?;
    output.flush()?;
    output.as_file().sync_all()?;
    if replace {
        let temporary = output.into_temp_path();
        platform::replace_file(&temporary, path)?;
    } else {
        output.persist_noclobber(path).map_err(|error| error.error)?;
    }
    sync_directory(parent)
}

pub(crate) struct OwnedDirectory { pub path: PathBuf }

impl OwnedDirectory {
    pub(crate) fn temporary(parent: &Path, prefix: &str) -> Result<Self> {
        for _ in 0..16 {
            let path = parent.join(format!("{prefix}{}", uuid::Uuid::new_v4().simple()));
            match platform::create_private_directory(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(Error::Invalid("cannot allocate a unique private temporary directory".into()))
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        // Only this transaction's fresh directory is removed, never a package
        // published at its final versioned location.
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn resolve_missing(path: &Path) -> Result<PathBuf> {
    let mut existing = path;
    let mut missing = Vec::new();
    while !present(existing)? {
        missing.push(existing.file_name().ok_or_else(|| Error::Invalid("destination has no existing parent".into()))?.to_owned());
        existing = existing.parent().ok_or_else(|| Error::Invalid("destination has no existing parent".into()))?;
    }
    let mut result = fs::canonicalize(existing)?;
    for name in missing.into_iter().rev() { result.push(name); }
    Ok(result)
}

pub(crate) fn create_private_parents(path: &Path) -> Result<()> {
    let mut parent = path;
    let mut missing = Vec::new();
    while !present(parent)? {
        missing.push(parent.to_path_buf());
        parent = parent.parent().ok_or_else(|| Error::Invalid("destination has no existing parent".into()))?;
    }
    directory(parent)?;
    for path in missing.into_iter().rev() {
        match platform::create_private_directory(&path) {
            Ok(()) => {},
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => directory(&path)?,
            Err(error) => return Err(error),
        }
        sync_directory(path.parent().unwrap())?;
    }
    Ok(())
}

pub(crate) fn verify_package(root: &Path, expected: &BTreeMap<&str, (u64, &str)>) -> Result<()> {
    directory(root)?;
    let mut members = 0;
    for entry in fs::read_dir(root)? {
        let name = entry?.file_name();
        let name = name.to_str().ok_or_else(|| Error::Invalid("package filenames must be valid UTF-8".into()))?;
        require(expected.contains_key(name), format!("unexpected file in voice package: {name}"))?;
        members += 1;
    }
    require(members == expected.len(), "voice package is missing an expected file")?;
    for (name, (size, digest)) in expected {
        let (actual_digest, actual_size) = hash_and_size(&root.join(name), *size)?;
        require(actual_size == *size && actual_digest == *digest, format!("voice package file size or SHA-256 differs: {name}"))?;
    }
    Ok(())
}

pub(crate) fn copy_bounded(source: &Path, destination: &Path, limit: u64) -> Result<()> {
    let mut source_file = open_regular(source)?;
    let before = source_file.metadata()?;
    require(before.len() <= limit, format!("package member exceeds its size limit: {}", source.display()))?;
    let mut output = fs::OpenOptions::new().write(true).create_new(true).open(destination)?;
    let copied = std::io::copy(&mut (&mut source_file).take(limit + 1), &mut output)?;
    output.flush()?;
    require(copied == before.len() && same_file(&before, &source_file.metadata()?) && matches_path(&source_file, source)?,
        "source file changed while copying")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_installers_conflict_until_the_first_lock_is_closed() {
        let scratch = tempfile::tempdir().unwrap();
        let registry = scratch.path().join("voices.json");
        atomic_bytes(&registry, b"{}\n", 0o600, false).unwrap();
        let first = platform::RegistryLock::acquire(&registry).unwrap();
        assert!(platform::RegistryLock::acquire(&registry).is_err());
        first.unchanged().unwrap();
        drop(first);
        platform::RegistryLock::acquire(&registry).unwrap().unchanged().unwrap();
    }

    #[test]
    fn immutable_directory_publication_cannot_replace_a_competing_destination() {
        let scratch = tempfile::tempdir().unwrap();
        let stage = scratch.path().join("stage"); let destination = scratch.path().join("destination");
        platform::create_private_directory(&stage).unwrap();
        platform::create_private_directory(&destination).unwrap();
        fs::write(stage.join("retained"), b"staged bytes").unwrap();
        assert!(platform::publish_directory(&stage, &destination).is_err());
        assert!(destination.is_dir());
        assert_eq!(fs::read(stage.join("retained")).unwrap(), b"staged bytes");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_leaves_are_not_admitted_as_regular_files() {
        let scratch = tempfile::tempdir().unwrap();
        let regular = scratch.path().join("source"); let link = scratch.path().join("link");
        fs::write(&regular, b"fixture bytes").unwrap();
        std::os::unix::fs::symlink(&regular, &link).unwrap();
        assert!(hash_and_size(&link, 13).is_err());
        assert_eq!(hash_and_size(&regular, 13).unwrap(), (digest(b"fixture bytes"), 13));
    }

    #[test]
    fn package_verification_checks_exact_inventory_size_and_digest() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path();
        let file = root.join("manifest.json");
        let pin = digest(b"exact bytes\r\n");
        let expected = BTreeMap::from([("manifest.json", (13, pin.as_str()))]);
        fs::write(&file, b"exact bytes\r\n").unwrap();
        verify_package(root, &expected).unwrap();
        fs::write(root.join("source.tkn"), b"extra").unwrap();
        assert!(verify_package(root, &expected).is_err());
        fs::remove_file(root.join("source.tkn")).unwrap();
        fs::write(&file, b"Exact bytes\r\n").unwrap();
        assert!(verify_package(root, &expected).is_err());
        fs::write(&file, b"exact bytes\r\n ").unwrap();
        assert!(verify_package(root, &expected).is_err());
    }
}
