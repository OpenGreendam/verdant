use std::{collections::BTreeSet, fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Component, Path, PathBuf}};
use crate::{Result, SkillFile, RECEIPT_FILE, conflict, invalid, io_error};

pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    let joined = if path.is_absolute() { path.to_owned() } else {
        std::env::current_dir().map_err(|e| io_error(path, e))?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {},
            Component::ParentDir => return Err(invalid("skill paths may not contain '..'")),
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))] { metadata.file_type().is_symlink() }
}

pub(crate) fn directory(path: &Path, allow_missing: bool) -> Result<()> {
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::CurDir | Component::ParentDir) { return Err(invalid("unresolved skill directory path")); }
        current.push(part.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) if is_link(&meta) || !meta.is_dir() => return Err(conflict(format!("skill directory traverses a link or nondirectory: {}", current.display()))),
            Ok(_) => {},
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(io_error(&current, error)),
        }
    }
    Ok(())
}

pub(crate) fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if is_link(&meta) => Err(conflict(format!("symbolic links are not managed: {}", path.display()))),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(path, error)),
    }
}

pub(crate) fn read_regular(path: &Path, limit: usize) -> Result<Vec<u8>> {
    directory(path.parent().ok_or_else(|| invalid("file has no parent"))?, false)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if is_link(&metadata) || !metadata.is_file() { return Err(conflict(format!("not a regular file: {}", path.display()))); }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)] {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(|error| io_error(path, error))?;
    let opened = file.metadata().map_err(|error| io_error(path, error))?;
    if is_link(&opened) || !opened.is_file() { return Err(conflict(format!("not a regular file: {}", path.display()))); }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|error| io_error(path, error))?;
    if bytes.len() > limit { return Err(invalid(format!("file exceeds skill size limit: {}", path.display()))); }
    Ok(bytes)
}

pub(crate) fn write_new(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    let parent = path.parent().ok_or_else(|| invalid("file has no parent"))?;
    directory(parent, true)?;
    fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    directory(parent, false)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if executable { 0o755 } else { 0o644 }).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path).map_err(|error| io_error(path, error))?;
    file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|error| io_error(path, error))?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 })).map_err(|error| io_error(path, error))?;
    }
    #[cfg(not(unix))] let _ = executable;
    Ok(())
}

pub(crate) fn verify_tree(root: &Path, files: &[SkillFile], receipt: bool) -> Result<()> {
    directory(root, false)?;
    let mut expected: BTreeSet<String> = files.iter().map(|file| file.path.clone()).collect();
    let mut directories = BTreeSet::new();
    for file in files {
        for (offset, _) in file.path.match_indices('/') { directories.insert(file.path[..offset].to_owned()); }
    }
    if receipt { expected.insert(RECEIPT_FILE.into()); }
    fn visit(root: &Path, relative: &Path, files: &mut BTreeSet<String>, directories: &mut BTreeSet<String>) -> Result<()> {
        let path = root.join(relative);
        for entry in fs::read_dir(&path).map_err(|error| io_error(&path, error))? {
            let entry = entry.map_err(|error| io_error(&path, error))?;
            let next = relative.join(entry.file_name());
            let name = next.to_str().ok_or_else(|| conflict("untracked non-UTF-8 skill path"))?.replace('\\', "/");
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| io_error(&entry.path(), error))?;
            if is_link(&metadata) { return Err(conflict(format!("symbolic link in skill directory: {}", entry.path().display()))); }
            if metadata.is_file() && files.remove(&name) { continue; }
            if metadata.is_dir() && directories.remove(&name) { visit(root, &next, files, directories)?; continue; }
            return Err(conflict(format!("untracked skill path is protected: {}", entry.path().display())));
        }
        Ok(())
    }
    visit(root, Path::new(""), &mut expected, &mut directories)?;
    if !expected.is_empty() || !directories.is_empty() { return Err(conflict(format!("managed skill files are missing: {}", root.display()))); }
    Ok(())
}

pub(crate) fn executable_matches(path: &Path, executable: bool) -> Result<bool> {
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?.permissions().mode();
        Ok((mode & 0o111 != 0) == executable)
    }
    #[cfg(not(unix))] { let _ = (path, executable); Ok(true) }
}

pub(crate) struct Lock { path: PathBuf, file: Option<File> }
impl Lock {
    pub fn acquire(root: &Path) -> Result<Self> {
        directory(root, true)?;
        fs::create_dir_all(root).map_err(|error| io_error(root, error))?;
        directory(root, false)?;
        let path = root.join(".verdant-skills.lock");
        let file = OpenOptions::new().write(true).create_new(true).open(&path)
            .map_err(|error| if error.kind() == std::io::ErrorKind::AlreadyExists {
                conflict(format!("another skill operation or a stale lock exists: {}; do not remove it while an operation is active", path.display()))
            } else { io_error(&path, error) })?;
        Ok(Self { path, file: Some(file) })
    }
}
impl Drop for Lock {
    fn drop(&mut self) { drop(self.file.take()); let _ = fs::remove_file(&self.path); }
}
