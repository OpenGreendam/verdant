use std::{ffi::{OsStr, OsString}, fs::{self, File, Metadata}, path::{Path, PathBuf}};
use crate::{Error, Result, files, require};

fn environment(name: &str) -> Option<OsString> { std::env::var_os(name).filter(|value| !value.is_empty()) }

pub(crate) fn expand_user(path: &Path) -> Result<PathBuf> {
    let text = path.as_os_str().to_string_lossy();
    if !text.starts_with('~') { return Ok(path.to_path_buf()); }
    #[cfg(unix)] {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let bytes = path.as_os_str().as_bytes();
        let end = bytes.iter().position(|byte| *byte == b'/').unwrap_or(bytes.len());
        let user = &bytes[1..end];
        let home = if user.is_empty() { environment("HOME").map(PathBuf::from).map(Ok).unwrap_or_else(|| unix_home(None))? }
            else { unix_home(Some(user))? };
        if end == bytes.len() { return Ok(home); }
        return Ok(home.join(OsString::from_vec(bytes[end + 1..].to_vec())));
    }
    #[cfg(windows)] {
        require(text == "~" || text.starts_with("~/") || text.starts_with("~\\"), "cannot expand another user's Windows home directory")?;
        let home = environment("USERPROFILE").or_else(|| {
            let mut drive = environment("HOMEDRIVE")?;
            drive.push(environment("HOMEPATH")?);
            Some(drive)
        }).ok_or_else(|| Error::Invalid("cannot find the desktop user's home directory".into()))?;
        return Ok(if text.len() == 1 { PathBuf::from(home) } else { PathBuf::from(home).join(&text[2..]) });
    }
    #[cfg(not(any(unix, windows)))] { Err(Error::Invalid("unsupported installation platform".into())) }
}

#[cfg(unix)]
fn unix_home(user: Option<&[u8]>) -> Result<PathBuf> {
    use std::{ffi::{CStr, CString}, os::unix::ffi::OsStrExt};
    let name = user.map(CString::new).transpose().map_err(|_| Error::Invalid("home username contains NUL".into()))?;
    let mut buffer = vec![0_u8; 4096];
    loop {
        let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
        let mut found = std::ptr::null_mut();
        // SAFETY: writable buffers and passwd/result pointers remain live for
        // the reentrant libc call; returned strings are copied before reuse.
        let status = unsafe {
            match &name {
                Some(name) => libc::getpwnam_r(name.as_ptr(), entry.as_mut_ptr(), buffer.as_mut_ptr().cast(), buffer.len(), &mut found),
                None => libc::getpwuid_r(libc::geteuid(), entry.as_mut_ptr(), buffer.as_mut_ptr().cast(), buffer.len(), &mut found),
            }
        };
        if status == libc::ERANGE && buffer.len() < 1_048_576 { buffer.resize(buffer.len() * 2, 0); continue; }
        require(status == 0 && !found.is_null(), "cannot find the desktop user's home directory")?;
        // SAFETY: a successful nonnull result initialized entry and pw_dir.
        let entry = unsafe { entry.assume_init() };
        require(!entry.pw_dir.is_null(), "desktop user has no home directory")?;
        let directory = unsafe { CStr::from_ptr(entry.pw_dir) };
        return Ok(PathBuf::from(OsStr::from_bytes(directory.to_bytes())));
    }
}

pub(crate) fn user_directory(kind: &str) -> Result<PathBuf> {
    let key = if kind == "data" { "VERDANT_USER_DATA_DIR" } else { "VERDANT_STATE_DIR" };
    if let Some(path) = environment(key) { return expand_user(Path::new(&path)); }
    #[cfg(windows)] {
        let local = environment("LOCALAPPDATA").map(PathBuf::from).ok_or_else(|| Error::Invalid("LOCALAPPDATA must name an absolute user directory".into()))?;
        require(local.is_absolute(), "LOCALAPPDATA must name an absolute user directory")?;
        return Ok(local.join("Verdant").join(kind));
    }
    #[cfg(target_os = "macos")] { return Ok(expand_user(Path::new("~"))?.join("Library/Application Support/Verdant").join(kind)); }
    #[cfg(all(unix, not(target_os = "macos")))] {
        let root = environment(if kind == "data" { "XDG_DATA_HOME" } else { "XDG_STATE_HOME" })
            .map(PathBuf::from).unwrap_or_else(|| PathBuf::from(if kind == "data" { "~/.local/share" } else { "~/.local/state" }));
        return Ok(expand_user(&root)?.join("verdant"));
    }
    #[cfg(not(any(unix, windows)))] { Err(Error::Invalid("unsupported installation platform".into())) }
}

pub(crate) fn default_registry() -> Result<PathBuf> {
    if let Some(path) = environment("VERDANT_VOICE_REGISTRY") { return Ok(path.into()); }
    Ok(user_directory("state")?.join("voices.json"))
}

pub(crate) fn reparse(metadata: &Metadata) -> bool {
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))] { let _ = metadata; false }
}

pub(crate) fn file_mode(metadata: &Metadata) -> u32 {
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; metadata.permissions().mode() & 0o7777 }
    #[cfg(not(unix))] { if metadata.permissions().readonly() { 0o444 } else { 0o666 } }
}

pub(crate) fn set_mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(path, fs::Permissions::from_mode(mode))?; }
    #[cfg(not(unix))] { let _ = (path, mode); }
    Ok(())
}

pub(crate) fn require_registry_owner(path: &Path) -> Result<()> {
    let file = files::open_regular(path)?;
    #[cfg(unix)] {
        use std::os::unix::fs::MetadataExt;
        require(file.metadata()?.uid() == unsafe { libc::geteuid() }, "run the installer as the desktop registry owner")?;
    }
    #[cfg(windows)] { windows::require_owner(path)?; }
    let _ = file;
    Ok(())
}

pub(crate) fn create_private_directory(path: &Path) -> Result<()> {
    #[cfg(unix)] {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(path)?;
    }
    #[cfg(windows)] { windows::create_private_directory(path)?; }
    #[cfg(not(any(unix, windows)))] { return Err(Error::Invalid("unsupported installation platform".into())); }
    Ok(())
}

pub(crate) fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(windows)] { windows::move_file(source, destination, true)?; }
    #[cfg(not(windows))] { fs::rename(source, destination)?; }
    Ok(())
}

pub(crate) fn publish_directory(source: &Path, destination: &Path) -> Result<()> {
    // A competing process must never have its new directory replaced, even if
    // it is still empty when this transaction finishes worker admission.
    #[cfg(target_os = "linux")] {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let source = CString::new(source.as_os_str().as_bytes()).map_err(|_| Error::Invalid("directory path contains NUL".into()))?;
        let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| Error::Invalid("directory path contains NUL".into()))?;
        if unsafe { libc::renameat2(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, destination.as_ptr(), libc::RENAME_NOREPLACE) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(target_os = "macos")] {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let source = CString::new(source.as_os_str().as_bytes()).map_err(|_| Error::Invalid("directory path contains NUL".into()))?;
        let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| Error::Invalid("directory path contains NUL".into()))?;
        if unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(windows)] { windows::move_file(source, destination, false)?; }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))] {
        let _ = (source, destination);
        return Err(Error::Invalid("unsupported installation platform".into()));
    }
    Ok(())
}

pub(crate) struct RegistryLock { file: File, path: PathBuf }

impl RegistryLock {
    pub(crate) fn acquire(registry: &Path) -> Result<Self> {
        let name = registry.file_name().ok_or_else(|| Error::Invalid("registry needs a file name".into()))?;
        let mut lock_name = OsString::from("."); lock_name.push(name); lock_name.push(".verdant-voice-install.lock");
        let path = registry.parent().unwrap().join(lock_name);
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK); }
        #[cfg(windows)] { use std::os::windows::fs::OpenOptionsExt; options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT); }
        let file = options.open(&path)?;
        require(files::matches_path(&file, &path)?, "transaction lock must be a regular nonsymlink file")?;
        #[cfg(unix)] {
            use std::os::fd::AsRawFd;
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::WouldBlock { return Err(Error::Invalid("another voice registry transaction is running".into())); }
                return Err(error.into());
            }
        }
        #[cfg(windows)] { windows::lock(&file)?; }
        Ok(Self { file, path })
    }

    pub(crate) fn unchanged(&self) -> Result<()> {
        require(files::matches_path(&self.file, &self.path)?, "transaction lock changed during installation")
    }
}

#[cfg(windows)]
pub(crate) mod windows {
    use super::*;
    use std::{ffi::c_void, os::windows::{ffi::OsStrExt, io::AsRawHandle}, ptr::null_mut};
    use windows_sys::Win32::{Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_LOCK_VIOLATION, GetLastError, HANDLE, LocalFree},
        Security::{EqualSid, GetFileSecurityW, GetSecurityDescriptorOwner, GetTokenInformation, OWNER_SECURITY_INFORMATION, SECURITY_ATTRIBUTES, TOKEN_QUERY, TokenOwner, TokenUser},
        Security::Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        Storage::FileSystem::{CreateDirectoryW, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
        System::{IO::OVERLAPPED, Threading::{GetCurrentProcess, OpenProcessToken}}};

    pub(crate) struct Handle(pub HANDLE);
    impl Drop for Handle { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }

    pub(crate) fn wide(path: &Path) -> Result<Vec<u16>> {
        let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
        require(!value.contains(&0), "Windows path contains NUL")?;
        value.push(0); Ok(value)
    }

    fn token() -> Result<Handle> {
        let mut token = null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        Ok(Handle(token))
    }

    fn token_identity(token: HANDLE, class: i32) -> Result<Vec<usize>> {
        let mut size = 0;
        unsafe { GetTokenInformation(token, class, null_mut(), 0, &mut size); }
        require(unsafe { GetLastError() } == ERROR_INSUFFICIENT_BUFFER && size > 0 && size <= 1_048_576, "cannot inspect desktop user identity")?;
        let mut data = vec![0_usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
        if unsafe { GetTokenInformation(token, class, data.as_mut_ptr().cast(), size, &mut size) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        Ok(data)
    }

    pub(crate) fn require_owner(path: &Path) -> Result<()> {
        let path = wide(path)?;
        let mut size = 0;
        unsafe { GetFileSecurityW(path.as_ptr(), OWNER_SECURITY_INFORMATION, null_mut(), 0, &mut size); }
        require(unsafe { GetLastError() } == ERROR_INSUFFICIENT_BUFFER && size > 0 && size <= 1_048_576, "cannot inspect registry ownership")?;
        let mut descriptor = vec![0_usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
        if unsafe { GetFileSecurityW(path.as_ptr(), OWNER_SECURITY_INFORMATION, descriptor.as_mut_ptr().cast(), size, &mut size) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        let mut owner = null_mut(); let mut defaulted = 0;
        if unsafe { GetSecurityDescriptorOwner(descriptor.as_mut_ptr().cast(), &mut owner, &mut defaulted) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        require(!owner.is_null(), "registry has no file owner")?;
        let token = token()?;
        // Elevated files can belong to the token's actual default-owner group.
        for class in [TokenUser, TokenOwner] {
            let identity = token_identity(token.0, class)?;
            let sid = unsafe { *identity.as_ptr().cast::<*mut c_void>() };
            if !sid.is_null() && unsafe { EqualSid(owner, sid) } != 0 { return Ok(()); }
        }
        Err(Error::Invalid("run the installer as the desktop registry owner".into()))
    }

    struct Descriptor(*mut c_void);
    impl Drop for Descriptor { fn drop(&mut self) { unsafe { LocalFree(self.0); } } }

    fn private_descriptor() -> Result<Descriptor> {
        let token = token()?;
        let identity = token_identity(token.0, TokenUser)?;
        let sid = unsafe { *identity.as_ptr().cast::<*mut c_void>() };
        let mut sid_string = null_mut();
        if unsafe { ConvertSidToStringSidW(sid, &mut sid_string) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        let allocated = Descriptor(sid_string.cast());
        let mut length = 0;
        unsafe { while *sid_string.add(length) != 0 { length += 1; } }
        let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(sid_string, length) }).map_err(|_| Error::Invalid("invalid desktop SID".into()))?;
        drop(allocated);
        let sddl = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;{sid})\0").encode_utf16().collect::<Vec<_>>();
        let mut descriptor = null_mut();
        if unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut descriptor, null_mut()) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        Ok(Descriptor(descriptor))
    }

    pub(crate) fn create_private_directory(path: &Path) -> Result<()> {
        let descriptor = private_descriptor()?;
        let attributes = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: descriptor.0, bInheritHandle: 0 };
        let path = wide(path)?;
        if unsafe { CreateDirectoryW(path.as_ptr(), &attributes) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        Ok(())
    }

    pub(crate) fn move_file(source: &Path, destination: &Path, replace: bool) -> Result<()> {
        let source = wide(source)?; let destination = wide(destination)?;
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), MOVEFILE_WRITE_THROUGH | if replace { MOVEFILE_REPLACE_EXISTING } else { 0 }) } == 0 { return Err(std::io::Error::last_os_error().into()); }
        Ok(())
    }

    pub(crate) fn same_open_file(first: &File, second: &File) -> Result<bool> {
        use windows_sys::Win32::Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle};
        let identity = |file: &File| -> Result<(u32, u32, u32)> {
            let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
            if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 { return Err(std::io::Error::last_os_error().into()); }
            Ok((info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow))
        };
        Ok(identity(first)? == identity(second)?)
    }

    pub(crate) fn lock(file: &File) -> Result<()> {
        let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
        if unsafe { LockFileEx(file.as_raw_handle(), LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0, 1, 0, &mut overlapped) } == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) { return Err(Error::Invalid("another voice registry transaction is running".into())); }
            return Err(error.into());
        }
        Ok(())
    }
}
