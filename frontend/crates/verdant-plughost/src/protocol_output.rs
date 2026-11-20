//! Own the inherited stdout transport before loading any plugin code.
//!
//! Plugins may write arbitrary bytes through Rust, C stdio or native handles.
//! Redirect process stdout to stderr once at startup; JSON uses only the
//! private, non-inheritable duplicate below. Never restore stdout while plugin
//! teardown or buffered diagnostic writes could still run.

use std::fs::File;
use std::io;

/// The sole protocol writer, moved into the control writer thread in host mode.
/// Dropping it closes the reserved transport without touching diagnostic stdout.
pub struct ProtocolOutput(File);

impl ProtocolOutput {
    /// Call once, on the main thread before plugin loading or I/O threads start.
    /// The caller must provide separate stdout/protocol and stderr/diagnostic
    /// streams, as the parent scanner and insert client do.
    pub fn capture() -> io::Result<Self> {
        platform::capture().map(Self)
    }

    pub fn write_line(&mut self, message: &impl serde::Serialize) -> io::Result<()> {
        verdant_plugins::proto::write_line(&mut self.0, message)
    }
}

#[cfg(unix)]
mod platform {
    use std::fs::File;
    use std::io;
    use std::os::fd::FromRawFd;

    pub fn capture() -> io::Result<File> {
        // Never reuse a closed standard descriptor (especially stderr) for
        // the protocol owner. CLOEXEC prevents plugin-spawned children from
        // retaining the protocol pipe after the host exits.
        let fd = unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_DUPFD_CLOEXEC, 3) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let output = unsafe { File::from_raw_fd(fd) };
        if unsafe { libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(output)
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_int;
    use std::fs::File;
    use std::io;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const O_WRONLY: c_int = 0x0001;
    const O_NOINHERIT: c_int = 0x0080;
    const O_BINARY: c_int = 0x8000;

    // Console APIs are also valid for redirected pipes. Declare this small
    // boundary here rather than enabling unrelated console support elsewhere.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(which: u32) -> HANDLE;
        fn SetStdHandle(which: u32, handle: HANDLE) -> i32;
    }

    // Use the target's CRT, including its normal static/dynamic link choice.
    // Linking a second CRT explicitly would leave the host's stdout unchanged.
    unsafe extern "C" {
        fn _open_osfhandle(handle: isize, flags: c_int) -> c_int;
        fn _get_osfhandle(fd: c_int) -> isize;
        fn _dup2(source: c_int, destination: c_int) -> c_int;
        fn _close(fd: c_int) -> c_int;
        fn _errno() -> *mut c_int;
    }

    fn std_handle(which: u32) -> io::Result<HANDLE> {
        let handle = unsafe { GetStdHandle(which) };
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else if handle.is_null() {
            Err(io::Error::new(io::ErrorKind::NotConnected, "missing standard output handle"))
        } else {
            Ok(handle)
        }
    }

    fn crt_error(operation: &str) -> io::Error {
        // CRT errno is not a Win32 GetLastError code.
        let errno = unsafe { *_errno() };
        io::Error::other(format!("{operation} failed (CRT errno {errno})"))
    }

    fn duplicate(handle: HANDLE) -> io::Result<OwnedHandle> {
        let process = unsafe { GetCurrentProcess() };
        let mut copy = std::ptr::null_mut();
        if unsafe {
            DuplicateHandle(process, handle, process, &mut copy, 0, 0, DUPLICATE_SAME_ACCESS)
        } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { OwnedHandle::from_raw_handle(copy) })
    }

    struct CrtFd(c_int);

    impl CrtFd {
        fn close(mut self) -> io::Result<()> {
            let fd = std::mem::replace(&mut self.0, -1);
            if unsafe { _close(fd) } != 0 {
                return Err(crt_error("close diagnostic duplicate"));
            }
            Ok(())
        }
    }

    impl Drop for CrtFd {
        fn drop(&mut self) {
            if self.0 >= 0 {
                // On an earlier capture failure, retain that primary error.
                unsafe { _close(self.0) };
            }
        }
    }

    pub fn capture() -> io::Result<File> {
        let stdout = std_handle(STD_OUTPUT_HANDLE)?;
        let stderr = std_handle(STD_ERROR_HANDLE)?;
        // A normally spawned native child starts with these referring to the
        // same handle. Refuse a pre-existing split rather than abandon an
        // unowned protocol handle or close another stream's handle.
        let crt_stdout = unsafe { _get_osfhandle(1) };
        if crt_stdout == -1 || crt_stdout == -2 {
            return Err(crt_error("get CRT stdout handle"));
        }
        if crt_stdout as HANDLE != stdout {
            return Err(io::Error::other("Win32 and CRT stdout differ before protocol capture"));
        }

        // Both clones are non-inheritable; each acquires exactly one owner.
        let output = duplicate(stdout)?;
        let diagnostics = duplicate(stderr)?;
        let fd = unsafe {
            _open_osfhandle(diagnostics.as_raw_handle() as isize, O_WRONLY | O_BINARY | O_NOINHERIT)
        };
        if fd < 0 {
            return Err(crt_error("open diagnostic duplicate"));
        }
        // Successful _open_osfhandle transfers HANDLE ownership to the CRT.
        let _ = diagnostics.into_raw_handle();
        let diagnostics = CrtFd(fd);
        if unsafe { _dup2(diagnostics.0, 1) } != 0 {
            return Err(crt_error("redirect CRT stdout"));
        }
        let redirected = unsafe { _get_osfhandle(1) };
        if redirected == -1 || redirected == -2 {
            return Err(crt_error("get redirected CRT stdout handle"));
        }
        // _dup2 closed the original stdout handle. fd 1 now owns a duplicate
        // of stderr for the process lifetime; native writes and Rust println!
        // must use it too. CRTs loaded by later plugins see this redirected
        // standard handle when initializing their own stdout streams.
        if unsafe { SetStdHandle(STD_OUTPUT_HANDLE, redirected as HANDLE) } == 0 {
            return Err(io::Error::last_os_error());
        }
        diagnostics.close()?;
        Ok(File::from(output))
    }
}
