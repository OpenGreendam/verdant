//! Actual public executables; every mutation is confined to temporary directories.
use std::{fs, io::{Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}, process::{Command, Output, Stdio}, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, thread, time::{Duration, Instant}};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn cli() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_verdant")) }
fn legacy() -> PathBuf { PathBuf::from(env!("CARGO_BIN_EXE_verdant-cli")) }

fn run(command: &mut Command, input: &[u8]) -> (bool, String, String) {
    let mut stdin = tempfile::tempfile().unwrap();
    stdin.write_all(input).unwrap(); stdin.seek(SeekFrom::Start(0)).unwrap();
    let mut stdout = tempfile::tempfile().unwrap();
    let mut stderr = tempfile::tempfile().unwrap();
    let mut child = command.stdin(stdin).stdout(stdout.try_clone().unwrap()).stderr(stderr.try_clone().unwrap()).spawn().unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() { break status; }
        if start.elapsed() > Duration::from_secs(15) {
            let _ = child.kill(); let _ = child.wait();
            panic!("public command did not terminate: {command:?}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    stdout.seek(SeekFrom::Start(0)).unwrap(); stderr.seek(SeekFrom::Start(0)).unwrap();
    let mut out = String::new(); let mut err = String::new();
    stdout.take(2 * 1024 * 1024).read_to_string(&mut out).unwrap();
    stderr.take(2 * 1024 * 1024).read_to_string(&mut err).unwrap();
    (status.success(), out, err)
}

fn result(command: &mut Command) -> Value {
    let (ok, out, err) = run(command, b"");
    assert!(ok, "{command:?}: {out}\n{err}");
    let response: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(response["ok"], true, "{response}");
    response["result"].clone()
}

fn bundle(root: &Path) -> PathBuf {
    let root = root.join("offline bundle 音楽");
    fs::create_dir_all(root.join("verdant-test")).unwrap();
    let content = "---\nname: verdant-test\ndescription: Exercise isolated agent skills.\n---\nRun verdant capabilities --json.\n";
    fs::write(root.join("verdant-test/SKILL.md"), content).unwrap();
    fs::write(root.join("manifest.json"), serde_json::to_vec_pretty(&json!({
        "schema":"verdant-agent-skills/1","version":"0.1.0","skills":[{
            "name":"verdant-test","description":"Exercise isolated agent skills.",
            "files":[{"path":"SKILL.md","sha256":hex::encode(Sha256::digest(content.as_bytes()))}]
        }]
    })).unwrap()).unwrap();
    root
}

#[test]
fn both_default_entrypoints_exit_with_help_without_gui() {
    let temp = tempfile::tempdir().unwrap();
    for path in [cli(), legacy()] {
        let (ok, out, err) = run(Command::new(&path).current_dir(temp.path())
            .env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY")
            .env("VERDANT_GUI_BINARY", temp.path().join("must-not-execute")), b"");
        assert!(ok, "{err}");
        assert!(out.contains("Usage: verdant ") && out.contains("without arguments prints this help"));
        let (ok, out, err) = run(Command::new(&path).arg("--runtime-features"), b"");
        assert!(ok, "{err}");
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["role"], "cli");
        assert_eq!(value["headless"], true);
        assert_eq!(value["default_command"], "help");
        assert_eq!(value["workflow_worker"]["protocol"], "verdant-cli/1");
    }
}

#[test]
fn offline_skills_install_idempotently_and_preserve_user_edits() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let bundle = bundle(&root);
    let destination = root.join("project 空 格/.agents/skills");
    let listed = result(Command::new(cli()).args(["skill", "list", "--bundle"]).arg(&bundle).arg("--json"));
    assert_eq!(listed["skills"][0]["name"], "verdant-test");
    let shown = result(Command::new(cli()).args(["skill", "show", "verdant-test", "--bundle"]).arg(&bundle).arg("--json"));
    assert!(shown["skills"][0]["content"].as_str().unwrap().contains("capabilities --json"));
    let mut install = Command::new(cli());
    install.args(["skill", "install", "--all", "--bundle"]).arg(&bundle).arg("--dest").arg(&destination).arg("--json");
    assert_eq!(result(&mut install)["skills"][0]["status"], "installed");
    assert_eq!(result(&mut install)["skills"][0]["status"], "unchanged");
    let skill = destination.join("verdant-test/SKILL.md");
    let original = fs::read(&skill).unwrap();
    fs::write(&skill, "local instructions").unwrap();
    let (ok, out, _) = run(&mut install, b"");
    assert!(!ok);
    assert_eq!(serde_json::from_str::<Value>(&out).unwrap()["error"]["code"], "CONFLICT");
    assert_eq!(fs::read_to_string(&skill).unwrap(), "local instructions");
    fs::write(&skill, original).unwrap();
    let removed = result(Command::new(cli()).args(["skill", "uninstall", "--all", "--dest"]).arg(&destination).arg("--json"));
    assert_eq!(removed["skills"][0]["status"], "removed");
    assert!(fs::read_dir(destination).unwrap().next().is_none());
}

#[test]
fn workflow_worker_and_legacy_serve_share_sequential_ndjson_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let bundle = bundle(&temp.path().canonicalize().unwrap());
    let first = json!({"protocol":"verdant-cli/1","request_id":"first","op":"capabilities","params":{}});
    let second = json!({"protocol":"verdant-cli/1","request_id":"second","op":"skill.list","params":{"bundle":bundle}});
    let input = format!("{first}\n{second}\n");
    for (binary, command) in [(cli(), "worker"), (legacy(), "serve")] {
        let (ok, out, err) = run(Command::new(binary).args([command, "--stdio"]), input.as_bytes());
        assert!(ok, "{err}");
        let events: Vec<Value> = out.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["request_id"], "first");
        assert_eq!(events[1]["request_id"], "second");
        assert!(events.iter().all(|event| event["event"] == "result" && event["ok"] == true && event["sequence"] == 1));
        let operations = events[0]["result"]["operations"].as_array().unwrap();
        for op in ["plugin.list", "mixer.inspect", "preset.list", "clip.split", "skill.install"] {
            assert!(operations.iter().any(|item| item == op), "missing worker operation {op}");
        }
        assert_eq!(events[1]["result"]["skills"][0]["name"], "verdant-test");
    }
}

fn native_fixture(root: &Path) -> PathBuf {
    let source = root.join("gui-fixture.rs");
    fs::write(&source, r#"
use std::{env, fs, io::{self, Write}, path::PathBuf, thread, time::{Duration, Instant}};
fn main() {
    let report = PathBuf::from(env::var_os("VERDANT_FIXTURE_REPORT").unwrap());
    fs::write(report.join("cwd"), env::current_dir().unwrap().to_string_lossy().as_bytes()).unwrap();
    for (index, arg) in env::args_os().skip(1).enumerate() {
        fs::write(report.join(format!("arg{index}")), arg.to_string_lossy().as_bytes()).unwrap();
    }
    println!("fixture GUI stdout");
    eprintln!("fixture GUI stderr");
    io::stdout().flush().unwrap();
    io::stderr().flush().unwrap();
    fs::write(report.join("ready"), b"ready").unwrap();
    let start = Instant::now();
    while !report.join("release").exists() {
        assert!(start.elapsed() < Duration::from_secs(60), "fixture was not released");
        thread::sleep(Duration::from_millis(10));
    }
    fs::write(report.join("done"), b"done").unwrap();
}
"#).unwrap();
    let executable = root.join(format!("GUI fixture 空 格{}", std::env::consts::EXE_SUFFIX));
    let output = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--crate-name", "verdant_gui_fixture", "--edition=2024"]).arg(source).arg("-o").arg(&executable).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    executable
}

struct GuiProbe {
    report: PathBuf,
    output: mpsc::Receiver<std::io::Result<Output>>,
    cancel: Arc<AtomicBool>,
    collector: Option<thread::JoinHandle<()>>,
}
impl GuiProbe {
    fn spawn(command: &mut Command, report: &Path) -> Self {
        let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let (sender, output) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::clone(&cancel);
        let collector = thread::spawn(move || {
            // Keep a cancellation path while the CLI itself is running. Once it
            // exits, wait_with_output must also observe EOF on both real pipes.
            let error = loop {
                if cancelled.load(Ordering::Relaxed) { let _ = child.kill(); break None; }
                match child.try_wait() {
                    Ok(Some(_)) => break None,
                    Ok(None) => thread::sleep(Duration::from_millis(10)),
                    Err(error) => { let _ = child.kill(); break Some(error); },
                }
            };
            let captured = child.wait_with_output();
            let _ = sender.send(match error { Some(error) => Err(error), None => captured });
        });
        Self { report: report.to_owned(), output, cancel, collector: Some(collector) }
    }
    fn ready(&self) {
        let start = Instant::now();
        while !self.report.join("ready").exists() {
            assert!(start.elapsed() < Duration::from_secs(30), "spawned GUI did not initialize");
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn release(&self) { fs::write(self.report.join("release"), b"release").unwrap(); }
    fn finished(&self) {
        let start = Instant::now();
        while !self.report.join("done").exists() {
            assert!(start.elapsed() < Duration::from_secs(10), "released GUI did not finish");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for GuiProbe {
    fn drop(&mut self) {
        // Always release the GUI, including assertion/timeout failures. The
        // collector also kills a CLI still running after the test has failed.
        let _ = fs::write(self.report.join("release"), b"release");
        self.cancel.store(true, Ordering::Relaxed);
        let start = Instant::now();
        if let Some(collector) = self.collector.take() {
            while (!collector.is_finished() || !self.report.join("done").exists())
                && start.elapsed() < Duration::from_secs(10)
            { thread::sleep(Duration::from_millis(10)); }
            if collector.is_finished() { let _ = collector.join(); }
        }
    }
}

#[test]
fn gui_preserves_arguments_cwd_unicode_and_never_inherits_agent_output_pipes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let fixture = native_fixture(&root);
    let cwd = root.join("unrelated cwd 空 格"); fs::create_dir(&cwd).unwrap();
    let logs = root.join("logs");
    let checked = result(Command::new(cli()).args(["gui", "--check", "--json", "--gui-binary"]).arg(&fixture).current_dir(&cwd));
    assert_eq!(PathBuf::from(checked["executable"].as_str().unwrap()).canonicalize().unwrap(), fixture.canonicalize().unwrap());
    assert_eq!(checked["launched"], false);
    for wait in [false, true] {
        let report = root.join(if wait { "report-wait" } else { "report-detached" });
        fs::create_dir(&report).unwrap();
        let mut command = Command::new(cli());
        command.args(["gui", "--json", "--gui-binary"]).arg(&fixture);
        if wait { command.arg("--wait"); }
        command.args(["--", "工程 名.verp", "--appearance", "dark"])
            .current_dir(&cwd).env("VERDANT_FIXTURE_REPORT", &report).env("VERDANT_LOG_DIR", &logs);
        let probe = GuiProbe::spawn(&mut command, &report);
        probe.ready();
        assert!(!report.join("release").exists() && !report.join("done").exists());
        let output = if wait {
            assert!(matches!(probe.output.recv_timeout(Duration::from_millis(250)), Err(mpsc::RecvTimeoutError::Timeout)),
                "--wait returned before the GUI was released");
            probe.release();
            probe.output.recv_timeout(Duration::from_secs(10)).expect("--wait did not return after GUI release").unwrap()
        } else {
            // The GUI cannot finish on its own. Receiving this output proves
            // the CLI exited AND both captured pipes reached EOF while it lives.
            let output = probe.output.recv_timeout(Duration::from_secs(10))
                .expect("CLI waited on the unreleased GUI or inherited GUI pipes").unwrap();
            assert!(!report.join("release").exists() && !report.join("done").exists());
            output
        };
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.stderr.is_empty(), "GUI stderr leaked into CLI output");
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(response["result"]["wait"], wait);
        if wait { assert_eq!(response["result"]["exit_code"], 0); }
        else { assert!(response["result"]["exit_code"].is_null()); }
        let reported_cwd = PathBuf::from(fs::read_to_string(report.join("cwd")).unwrap());
        assert_eq!(reported_cwd.canonicalize().unwrap(), cwd.canonicalize().unwrap());
        for (index, expected) in ["工程 名.verp", "--appearance", "dark"].into_iter().enumerate() {
            assert_eq!(fs::read_to_string(report.join(format!("arg{index}"))).unwrap(), expected);
        }
        let log = PathBuf::from(response["result"]["log"].as_str().unwrap());
        assert!(log.starts_with(&logs));
        let log = fs::read_to_string(log).unwrap();
        assert!(log.contains("fixture GUI stdout") && log.contains("fixture GUI stderr"));
        probe.release();
        probe.finished();
    }
}
