//! Native, offline release assembly and bounded recursive release auditing.
mod archive;
mod files;
mod entrypoints;
mod package;
mod policy;
mod source;
mod target;
#[cfg(test)]
mod tests;

use std::{collections::BTreeMap, path::PathBuf};
use anyhow::{Context, Result, bail, ensure};
use serde_json::json;

const HELP: &str = "verdant-release commands:
  runtime-inputs --target linux-x86_64|linux-aarch64|windows-x86_64|macos-aarch64
  select-source --repo DIR --output FILE
  source --repo DIR --source-selection FILE --source-selection-sha256 SHA256 --output FILE [--epoch SECONDS]
  package --repo DIR --binaries DIR --output DIR --source-selection FILE --source-selection-sha256 SHA256
          (--onnxruntime-dir DIR | --onnxruntime-wheel FILE) [--target TARGET] [--version VERSION]
          [--epoch SECONDS]
          [--voice-bundle DIR --artifact-policy FILE --artifact-policy-sha256 SHA256]
  check --check DIR [--expected-source-commit SHA1] [--expected-source-selection-sha256 SHA256]
        [--require-package] [--require-voice]
        [--artifact-policy FILE --artifact-policy-sha256 SHA256] [--write FILE]

Package runs on its target build host and probes the staged app and worker with
--runtime-features. It never downloads inputs. Runtime libraries and their complete
licenses are pinned to ONNX Runtime 1.23.2. Windows uses ZIP; Linux and Apple Silicon
use tar.gz (macOS includes a signed Verdant.app). Existing outputs are never replaced.

select-source hashes exact HEAD blobs of public build/source/documentation inputs,
including plugins/dsh. Staged changes or worktree divergence after Git's path-specific
normalization fail admission. It never visits retired research/python-oracle or
neural/cfinal trees. Review the selected-source JSON and explicitly supply its SHA256
to source/package. SOURCE-SELECTION.json records committed blob IDs, modes and SHA256,
is embedded in the source offer, and is bound by RELEASE-ASSETS.json. For independent
audits, pin it with check --expected-source-selection-sha256 as well as the commit.

The default artifact policy excludes all voice/model/audio data. A voice bundle
requires a separately reviewed, hash-bound verdant-release-artifact-policy/1 with
every runtime role and artifact-specific redistribution grants. Source-use attestation
and project consent are insufficient. check needs that same external policy to admit
any bundled voice data; an embedded policy never authorizes itself. Assembly is not
voice/runtime quality acceptance or authorization to publish.";

pub(crate) struct Args(BTreeMap<String, String>);

impl Args {
    fn parse() -> Result<(String, Self)> {
        let mut values = std::env::args().skip(1);
        let command = values.next().unwrap_or_else(|| "--help".into());
        let mut options = BTreeMap::new();
        while let Some(key) = values.next() {
            ensure!(key.starts_with("--"), "expected an option, got {key}");
            let (key, value) = if let Some((key, value)) = key.split_once('=') {
                (key.to_owned(), value.to_owned())
            } else if matches!(key.as_str(), "--require-package" | "--require-voice") {
                (key, "true".into())
            } else {
                let value = values.next().with_context(|| format!("{key} requires a value"))?;
                ensure!(!value.starts_with("--"), "{key} requires a value");
                (key, value)
            };
            ensure!(options.insert(key.clone(), value).is_none(), "repeated option {key}");
        }
        Ok((command, Self(options)))
    }
    pub(crate) fn take(&mut self, key: &str) -> Option<String> { self.0.remove(key) }
    pub(crate) fn required(&mut self, key: &str) -> Result<String> {
        self.take(key).with_context(|| format!("missing {key}"))
    }
    pub(crate) fn path(&mut self, key: &str) -> Result<PathBuf> { Ok(self.required(key)?.into()) }
    pub(crate) fn flag(&mut self, key: &str) -> Result<bool> {
        match self.take(key).as_deref() {
            None => Ok(false), Some("true") => Ok(true), _ => bail!("{key} takes no value"),
        }
    }
    pub(crate) fn epoch(&mut self) -> Result<u64> {
        self.take("--epoch").or_else(|| std::env::var("SOURCE_DATE_EPOCH").ok())
            .unwrap_or_else(|| "0".into()).parse().context("invalid SOURCE_DATE_EPOCH/--epoch")
    }
    pub(crate) fn finish(self) -> Result<()> {
        ensure!(self.0.is_empty(), "unknown options: {:?}", self.0.keys().collect::<Vec<_>>());
        Ok(())
    }
}

fn run() -> Result<()> {
    let (command, mut args) = Args::parse()?;
    let report = match command.as_str() {
        "--help" | "-h" | "help" => { println!("{HELP}"); return Ok(()); }
        "--version" => { println!("verdant-release {}", env!("CARGO_PKG_VERSION")); return Ok(()); }
        "--runtime-features" => json!({"schema":"verdant-runtime-features/1","role":"release",
            "version":env!("CARGO_PKG_VERSION"),"production_python_required":false,
            "voice_package":{"name":"verdant-voice","version":env!("CARGO_PKG_VERSION")}}),
        "runtime-inputs" => {
            let target = target::Target::parse(&args.required("--target")?)?;
            args.finish()?;
            serde_json::to_value(target.runtime())?
        }
        "select-source" => {
            let root = args.path("--repo")?;
            let output = args.path("--output")?;
            args.finish()?;
            let selection = source::select(&root)?;
            files::write_new_json(&output, &selection)?;
            json!({"status":"selected","source_selection":output,"sha256":files::digest(&output)?,
                "source_commit":selection.source_commit,"file_count":selection.files.len()})
        }
        "source" => {
            let root = args.path("--repo")?;
            let selection = args.path("--source-selection")?;
            let sha = args.required("--source-selection-sha256")?;
            let output = args.path("--output")?;
            let epoch = args.epoch()?;
            args.finish()?;
            let selected = source::load(&root, &selection, &sha)?;
            source::create(&root, &selection, &sha, &selected, &output, epoch)?;
            json!({"status":"archived","archive":output,"sha256":files::digest(&output)?,
                "source_selection_sha256":sha,"source_commit":selected.source_commit})
        }
        "package" => package::run(args)?,
        "check" => {
            let root = args.path("--check")?;
            let expected_commit = args.take("--expected-source-commit");
            let expected_selection = args.take("--expected-source-selection-sha256");
            let require_package = args.flag("--require-package")?;
            let require_voice = args.flag("--require-voice")?;
            let policy = policy::external(&mut args)?;
            let output = args.take("--write").map(PathBuf::from);
            args.finish()?;
            let report = archive::check(&root, policy.as_ref(), expected_commit.as_deref(), expected_selection.as_deref(), require_package || require_voice, require_voice)?;
            if let Some(path) = output { files::write_new_json(&path, &report)?; }
            report
        }
        _ => bail!("unknown command {command}; use --help"),
    };
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", json!({"status":"rejected","policy":"RELEASE.md","error":format!("{error:#}")}));
        std::process::exit(2);
    }
}
