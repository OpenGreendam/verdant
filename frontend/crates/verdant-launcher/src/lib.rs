//! GUI discovery and launching shared by the thin CLI and package-root launcher.
//! No graphical toolkit or synthesis engine is loaded by this crate.
use std::{
    env, ffi::{OsStr, OsString}, fs::{self, File, OpenOptions},
    io, path::{Path, PathBuf}, process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const HELP: &str = "Usage: verdant gui [--wait | --check] [--gui-binary PATH] [--json] [--] [GUI OPTIONS]\n\
Open the graphical editor explicitly. By default this command returns after launch.\n\
  --wait          Wait for the editor to exit; return its exit status\n\
  --check         Report the resolved editor and resource paths without opening it\n\
  --gui-binary    Explicit editor executable (otherwise use the same installation)\n\
GUI OPTIONS include PROJECT.verp, --import FILE, --silent-audio, --appearance light|dark,\n\
--worker-binary PATH, --plughost PATH, --workspace DIR, and --voice-package DIR.\n\
Put launcher options before GUI arguments. GUI logs are written separately from CLI output.\n";

#[derive(Debug, Clone)]
pub struct LaunchPlan {
    pub executable: PathBuf,
    pub environment: Vec<(String, PathBuf)>,
}

#[derive(Debug)]
pub struct Launched {
    pub pid: u32,
    pub log: PathBuf,
    /// Present when --wait was requested or the editor exited during startup.
    pub exit_code: Option<i32>,
}

fn program(name: &str) -> OsString { format!("{name}{}", env::consts::EXE_SUFFIX).into() }

pub fn resolve(override_path: Option<&Path>) -> io::Result<LaunchPlan> {
    let executable = env::current_exe()?;
    let environment_override = env::var_os("VERDANT_GUI_BINARY").map(PathBuf::from);
    resolve_from(&executable, override_path.or(environment_override.as_deref()))
}

pub fn resolve_from(caller: &Path, override_path: Option<&Path>) -> io::Result<LaunchPlan> {
    let directory = caller.parent().ok_or_else(|| io::Error::other("launcher executable has no parent"))?;
    let candidates = if let Some(path) = override_path {
        vec![path.to_owned()]
    } else {
        let name = program("verdant-gui");
        vec![
            directory.join(&name),
            directory.join("../libexec/verdant").join(&name),
            directory.join("bin").join(&name),
            directory.join("libexec/verdant").join(&name),
            directory.join("Verdant.app/Contents/MacOS").join(&name),
        ]
    };
    let executable = candidates.iter().find(|path| path.is_file())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound,
            format!("The graphical editor was not found in this Verdant installation. Expected {}. Install the complete native package or set VERDANT_GUI_BINARY.",
                candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "))))?
        .canonicalize()?;
    if caller.canonicalize().ok().as_ref() == Some(&executable) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "GUI executable cannot be the calling launcher"));
    }
    let parent = executable.parent().ok_or_else(|| io::Error::other("GUI executable has no parent"))?;
    let mut environment = Vec::new();
    let resources = [
        parent.join("../Resources/share/verdant"), parent.join("../share/verdant"),
        parent.join("../../share/verdant"),
    ];
    if let Some(path) = resources.iter().find(|path| path.is_dir()) {
        environment.push(("VERDANT_DATA_DIR".into(), path.canonicalize()?));
    }
    for (variable, name) in [("VERDANT_WORKER_BINARY", "verdant-worker"), ("VERDANT_PLUGIN_HOST", "verdant-plughost")] {
        let name = program(name);
        let paths = [parent.join(&name), parent.join("../../bin").join(&name), parent.join("../libexec/verdant").join(&name)];
        if let Some(path) = paths.iter().find(|path| path.is_file()) {
            environment.push((variable.into(), path.canonicalize()?));
        }
    }
    let libraries = [
        parent.join("onnxruntime.dll"),
        parent.join("../Frameworks/libonnxruntime.1.23.2.dylib"),
        parent.join("../lib/libonnxruntime.so.1.23.2"),
        parent.join("../../lib/libonnxruntime.so.1.23.2"),
    ];
    if let Some(path) = libraries.iter().find(|path| path.is_file()) {
        environment.push(("VERDANT_ONNXRUNTIME_LIBRARY".into(), path.canonicalize()?));
    }
    Ok(LaunchPlan { executable, environment })
}

fn log_directory() -> PathBuf {
    if let Some(path) = env::var_os("VERDANT_LOG_DIR").filter(|p| !p.is_empty()) { return path.into(); }
    #[cfg(windows)]
    if let Some(path) = env::var_os("LOCALAPPDATA") { return PathBuf::from(path).join("Verdant/logs"); }
    #[cfg(target_os = "macos")]
    if let Some(path) = env::var_os("HOME") { return PathBuf::from(path).join("Library/Logs/Verdant"); }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        if let Some(path) = env::var_os("XDG_STATE_HOME") { return PathBuf::from(path).join("verdant/logs"); }
        if let Some(path) = env::var_os("HOME") { return PathBuf::from(path).join(".local/state/verdant/logs"); }
    }
    env::temp_dir().join("verdant-logs")
}

fn create_log(directory: &Path) -> io::Result<(PathBuf, File)> {
    fs::create_dir_all(directory)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    for suffix in 0..8 {
        let path = directory.join(format!("gui-{}-{stamp}-{suffix}.log", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "cannot reserve a unique GUI log"))
}

impl LaunchPlan {
    pub fn command(&self, arguments: &[OsString]) -> Command {
        let mut command = Command::new(&self.executable);
        command.args(arguments);
        for (name, path) in &self.environment {
            if env::var_os(name).is_none() { command.env(name, path); }
        }
        command
    }

    pub fn launch(&self, arguments: &[OsString], wait: bool) -> io::Result<Launched> {
        self.launch_in(arguments, wait, &log_directory())
    }

    fn launch_in(&self, arguments: &[OsString], wait: bool, logs: &Path) -> io::Result<Launched> {
        let (log, output) = create_log(logs)?;
        let mut command = self.command(arguments);
        // A detached GUI must not keep an agent's stdin/stdout pipe alive.
        command.stdin(Stdio::null()).stdout(output.try_clone()?).stderr(output);
        if !wait {
            #[cfg(unix)] { use std::os::unix::process::CommandExt; command.process_group(0); }
            #[cfg(windows)] {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0000_0200 | 0x0800_0000); // new group; no console
            }
        }
        let mut child = command.spawn().map_err(|error|
            io::Error::new(error.kind(), format!("cannot launch {}: {error}; log {}", self.executable.display(), log.display())))?;
        let pid = child.id();
        let status = if wait { Some(child.wait()?) } else {
            // Catch immediate loader/argument failures without waiting for the session.
            std::thread::sleep(Duration::from_millis(75));
            child.try_wait()?
        };
        let exit_code = status.map(|status| status.code().unwrap_or(1));
        Ok(Launched { pid, log, exit_code })
    }
}

pub fn runtime_features() -> String {
    format!("{{\"schema\":\"verdant-runtime-features/1\",\"role\":\"launcher\",\"version\":\"{}\",\"voice_package\":{{\"name\":\"verdant-voice\",\"version\":\"{}\"}},\"gui_executable\":\"verdant-gui\",\"headless_probe\":true}}",
        env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_VERSION"))
}

/// The human-facing launcher reports startup failures in the native desktop when possible.
pub fn show_error(message: &str) {
    eprintln!("Verdant: {message}");
    #[cfg(windows)] {
        #[link(name = "User32")]
        unsafe extern "system" { fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, caption: *const u16, flags: u32) -> i32; }
        let message: Vec<u16> = message.encode_utf16().filter(|c| *c != 0).chain([0]).collect();
        let caption: Vec<u16> = "Verdant".encode_utf16().chain([0]).collect();
        unsafe { MessageBoxW(std::ptr::null_mut(), message.as_ptr(), caption.as_ptr(), 0x10); }
    }
    #[cfg(target_os = "macos")] {
        let _ = Command::new("/usr/bin/osascript").args(["-e",
            "on run argv\n display alert \"Verdant\" message (item 1 of argv) as critical\nend run", "--", message])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    #[cfg(not(any(windows, target_os = "macos")))] {
        if env::var_os("DISPLAY").is_some() || env::var_os("WAYLAND_DISPLAY").is_some() {
            let result = Command::new("zenity").args(["--error", "--no-markup", "--title=Verdant", "--text", message])
                .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
            if result.is_err() {
                let _ = Command::new("kdialog").args(["--title", "Verdant", "--error", message])
                    .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
            }
        }
    }
}

pub fn is_flag(argument: &OsStr, flag: &str) -> bool { argument == OsStr::new(flag) }

#[cfg(test)]
mod tests {
    use super::*;
    fn fake(path: &Path) { fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(path, b"fixture").unwrap(); }

    #[test]
    fn resolves_native_layouts_and_never_falls_back_to_path() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().join("bundle with spaces 音楽"); fs::create_dir_all(&base).unwrap();
        let caller = base.join(program("verdant-launcher")); fake(&caller);
        let gui = base.join("bin").join(program("verdant-gui")); fake(&gui);
        let worker = base.join("bin").join(program("verdant-worker")); fake(&worker);
        assert_eq!(resolve_from(&caller, None).unwrap().executable, gui.canonicalize().unwrap());
        assert!(resolve_from(&caller, Some(&base.join("missing"))).is_err());
        assert!(resolve_from(&caller, Some(&caller)).is_err());
        fs::remove_file(gui).unwrap();
        let gui = base.join("libexec/verdant").join(program("verdant-gui")); fake(&gui);
        assert_eq!(resolve_from(&base.join("bin").join(program("verdant")), None).unwrap().executable, gui.canonicalize().unwrap());
        fs::remove_file(gui).unwrap();
        let gui = base.join("Verdant.app/Contents/MacOS").join(program("verdant-gui")); fake(&gui);
        assert_eq!(resolve_from(&caller, None).unwrap().executable, gui.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn launch_preserves_arguments_and_cwd_with_separate_io() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let gui = root.path().join("gui 空 格");
        fs::write(&gui, "#!/bin/sh\nout=$1\ncwd=$2\nshift 2\nprintf '%s\\n' \"$@\" > \"$out\"\npwd > \"$cwd\"\nprintf 'gui stdout\\n'\nprintf 'gui stderr\\n' >&2\n").unwrap();
        fs::set_permissions(&gui, fs::Permissions::from_mode(0o755)).unwrap();
        let out = root.path().join("args"); let cwd = root.path().join("cwd");
        let plan = LaunchPlan { executable: gui, environment: Vec::new() };
        let launched = plan.launch_in(&[out.clone().into_os_string(), cwd.clone().into_os_string(), "工程 名.verp".into(), "--silent-audio".into()], true, &root.path().join("logs")).unwrap();
        assert_eq!(launched.exit_code, Some(0));
        assert_eq!(fs::read_to_string(out).unwrap(), "工程 名.verp\n--silent-audio\n");
        assert_eq!(PathBuf::from(fs::read_to_string(cwd).unwrap().trim()), env::current_dir().unwrap());
        assert_eq!(fs::read_to_string(launched.log).unwrap(), "gui stdout\ngui stderr\n");
    }
}
