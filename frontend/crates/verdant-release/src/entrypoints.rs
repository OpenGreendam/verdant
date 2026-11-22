//! Real staged-entry checks run from a different working directory before publication.
use std::{fs, path::{Path, PathBuf}, process::Command, time::Duration};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use crate::{files, package::run_bounded, target::Target};

fn command(executable: &Path, directory: &Path) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(directory);
    for key in ["VERDANT_GUI_BINARY", "VERDANT_DATA_DIR", "VERDANT_SKILLS_DIR", "DISPLAY", "WAYLAND_DISPLAY"] {
        command.env_remove(key);
    }
    command
}
fn response(command: &mut Command) -> Result<Value> {
    let text = run_bounded(command, Duration::from_secs(30))?;
    let response: Value = files::parse_json(text.as_bytes())?;
    ensure!(response["protocol"] == "verdant-cli/1" && response["ok"] == true, "staged CLI request failed: {response}");
    response.get("result").cloned().context("staged CLI response omits result")
}
fn path(value: &Value) -> Result<PathBuf> {
    PathBuf::from(value.as_str().context("CLI path is not a string")?).canonicalize().context("CLI path is unavailable")
}

pub(crate) fn verify(stage: &Path, target: Target, version: &str, skills: usize) -> Result<()> {
    let probe = tempfile::Builder::new().prefix("verdant unrelated cwd 空 格 ").tempdir()?;
    let cwd = probe.path().canonicalize()?;
    let cli = stage.join(target.program_path("verdant"));
    let gui = stage.join(target.program_path("verdant-gui")).canonicalize()?;
    for name in ["verdant", "verdant-cli"] {
        let executable = stage.join(target.program_path(name));
        // A missing override makes an accidentally restored GUI default fail
        // without ever opening the real editor on the packaging host.
        let text = run_bounded(command(&executable, &cwd).env("VERDANT_GUI_BINARY", cwd.join("must-not-launch")),
            Duration::from_secs(3))?;
        ensure!(text.contains("Usage: verdant ") && text.contains("without arguments prints this help"), "default command does not exit with help");
        let resolved = response(command(&executable, &cwd).args(["gui", "--check", "--json"]))?;
        ensure!(resolved["launched"] == false && path(&resolved["executable"])? == gui, "GUI command does not resolve this package");
    }
    let launcher = stage.join(target.program_path("verdant-launcher"));
    let resolved = run_bounded(command(&launcher, &cwd).arg("--check"), Duration::from_secs(5))?;
    ensure!(PathBuf::from(resolved.trim()).canonicalize()? == gui, "root launcher does not resolve this package");
    let listed = response(command(&cli, &cwd).args(["skill", "list", "--json"]))?;
    ensure!(listed["version"] == version && listed["skills"].as_array().is_some_and(|items| items.len() == skills),
        "packaged CLI cannot read its complete offline Agent Skills bundle");
    let destination = cwd.join("installed skills 音楽");
    let mut install = command(&cli, &cwd);
    install.args(["skill", "install", "--all", "--dest"]).arg(&destination).arg("--json");
    let installed = response(&mut install)?;
    ensure!(installed["skills"].as_array().is_some_and(|items| items.len() == skills && items.iter().all(|item| item["status"] == "installed")),
        "offline skill install did not install the full bundle");
    let installed = response(&mut install)?;
    ensure!(installed["skills"].as_array().is_some_and(|items| items.len() == skills && items.iter().all(|item| item["status"] == "unchanged")),
        "offline skill installation is not idempotent");
    let removed = response(command(&cli, &cwd).args(["skill", "uninstall", "--all", "--dest"]).arg(&destination).arg("--json"))?;
    ensure!(removed["skills"].as_array().is_some_and(|items| items.len() == skills && items.iter().all(|item| item["status"] == "removed"))
        && fs::read_dir(&destination)?.next().is_none(), "offline skill uninstall did not remove only the managed install");
    Ok(())
}
