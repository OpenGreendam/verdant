//! Corresponding source is selected from Git's tracked public inputs, never by
//! walking retired research trees. Source bytes are exact committed Git blobs;
//! both staged changes and normalized worktree divergence reject selection.
use std::{collections::{BTreeMap, BTreeSet}, fs::OpenOptions, io::{BufRead, BufReader, Read, Seek, SeekFrom, Write}, path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio}};
use anyhow::{Context, Result, ensure};
use flate2::{Compression, GzBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::{archive, files::{self, Entry}, policy};

pub(crate) const SCHEMA: &str = "verdant-selected-source/1";
pub(crate) const REQUIRED: &[&str] = &[
    "LICENSE", "NOTICE.md", "README.md", "RELEASE.md", "SPEC.md", "TODO.md",
    "frontend/Cargo.toml", "frontend/Cargo.lock", "frontend/rust-toolchain.toml", "frontend/Dockerfile",
    "frontend/crates/verdant-release/Cargo.toml", "frontend/crates/verdant-release/src/main.rs",
    "frontend/crates/verdant-cli/Cargo.toml", "frontend/crates/verdant-cli/src/lib.rs",
    "frontend/crates/verdant-launcher/Cargo.toml", "frontend/crates/verdant-launcher/src/lib.rs",
    "frontend/crates/verdant-launcher/src/main.rs", "frontend/crates/verdant-agent-skills/Cargo.toml",
    "frontend/crates/verdant-agent-skills/src/lib.rs", "frontend/skills/manifest.json",
    "frontend/crates/verdant-voice/Cargo.toml",
    "backend/worker-request.schema.json", "plugins/dsh/package.json", "plugins/dsh/src/index.ts",
    "plugins/dsh/tsconfig.json", "plugins/dsh/cordis.patch.yml",
];
const SCOPES: &[&str] = &["frontend", "backend", "tools", "spec", "docs", ".github", "runtime-assets", "plugins/dsh",
    "LICENSE", "NOTICE.md", "README.md", "RELEASE.md", "SPEC.md", "DEV_PLAN.md", "TODO.md",
    "CONTRIBUTING.md", "SYNTHESIS-FIDELITY.md", ".gitignore", ".gitattributes"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectedFile { pub path: String, pub bytes: u64, pub sha256: String, pub mode: u32, pub git_blob: String }
impl SelectedFile {
    pub(crate) fn matches(&self, entry: &Entry) -> bool {
        self.path == entry.path && self.bytes == entry.bytes && self.sha256 == entry.sha256 && self.mode == entry.mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    pub schema: String,
    pub source_commit: String,
    pub files: Vec<SelectedFile>,
    pub exclusions: BTreeMap<String, u64>,
}

fn scope(path: &str) -> bool {
    matches!(path, "LICENSE" | "NOTICE.md" | "README.md" | "RELEASE.md" | "SPEC.md" | "DEV_PLAN.md" | "TODO.md"
        | "CONTRIBUTING.md" | "SYNTHESIS-FIDELITY.md" | ".gitignore" | ".gitattributes")
        || ["frontend/", "backend/", "tools/", "spec/", "docs/", ".github/", "runtime-assets/", "plugins/dsh/"].iter().any(|prefix| path.starts_with(prefix))
}

fn excluded(path: &str) -> Option<&'static str> {
    if !scope(path) { return Some("outside current public corresponding-source scope"); }
    if matches!(path,
        "docs/archive/research/licenses/vocoder/NOTICE.txt"
        | "docs/archive/research/licenses/vocoder/NOTICE.zh-CN.txt"
        | "docs/archive/research/licenses/vocoder/STATEMENTS.txt"
    ) { return None; }
    if path.split('/').any(|part| matches!(part, "target" | "node_modules" | "dist" | "__pycache__" | "validation"
        | "validation-final" | "validation-pinned" | "evidence" | "outputs" | "models" | "deps"
        | "logs" | "wheelhouse" | "venvs" | "runtime-cache" | "runtime-view" | "runs" | "research"))
        || path.starts_with("plugins/dsh/data/") {
        return Some("retired research, execution evidence, cache or dependency output");
    }
    if policy::forbidden_name(path, true) { return Some("restricted artifact type or identity"); }
    if path.ends_with(".csv") && !(path.starts_with("frontend/crates/") && path.contains("/src/")) {
        return Some("numeric tables outside authored Rust source");
    }
    if path.ends_with(".vsqx") && !path.starts_with("frontend/crates/verdant-exchange/tests/fixtures/") {
        return Some("score inputs outside admitted source fixtures");
    }
    if path.ends_with(".log") || path.ends_with(".png") { return Some("execution log or documentation capture"); }
    None
}

fn source_kind(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    matches!(name, "Dockerfile" | "LICENSE" | "NOTICE" | "COPYING" | ".gitignore" | ".gitattributes" | ".dockerignore")
        || name.starts_with("LICENSE-")
        || Path::new(path).extension().and_then(|s| s.to_str()).is_some_and(|ext| matches!(ext,
            "rs" | "toml" | "lock" | "json" | "md" | "txt" | "html" | "css" | "js" | "mjs" | "ts"
            | "py" | "sh" | "ps1" | "cmd" | "bat" | "sha256" | "verp" | "ufdata" | "vsqx" | "desktop"
            | "xml" | "yml" | "yaml" | "svg" | "csv" | "map" | "c" | "cc" | "cpp" | "h" | "hpp" | "m" | "mm"))
}

fn git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>> {
    let result = Command::new("git").arg("-C").arg(root).args(arguments).output()?;
    ensure!(result.status.success(), "cannot enumerate tracked source: {}", String::from_utf8_lossy(&result.stderr));
    Ok(result.stdout)
}

pub(crate) fn select(root: &Path) -> Result<Selection> {
    let root = files::absolute(root)?;
    let source_commit = String::from_utf8(git(&root, &["rev-parse", "HEAD"])?)?.trim().to_owned();
    ensure!(source_commit.len() == 40 && source_commit.bytes().all(|c| c.is_ascii_hexdigit()), "invalid source commit identity");
    let mut tree_args = vec!["ls-tree", "-r", "-z", "--full-tree", "HEAD", "--"];
    tree_args.extend_from_slice(SCOPES);
    let tracked = git(&root, &tree_args)?;
    let mut files = Vec::new();
    let mut exclusions = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut blobs = Blobs::open(&root)?;
    for member in tracked.split(|b| *b == 0).filter(|member| !member.is_empty()) {
        let member = std::str::from_utf8(member).context("source paths must be UTF-8")?;
        let (metadata, path) = member.split_once('\t').context("invalid tracked source entry")?;
        // The namespace decision precedes all filesystem access, including stat.
        if let Some(reason) = excluded(path) {
            *exclusions.entry(reason.to_owned()).or_insert(0) += 1;
            continue;
        }
        files::portable_path(path)?;
        ensure!(seen.insert(path.to_owned()), "duplicate source entry: {path}");
        ensure!(source_kind(path), "unclassified public source input: {path}");
        let metadata: Vec<_> = metadata.split(' ').collect();
        ensure!(metadata.len() == 3 && metadata[1] == "blob" && matches!(metadata[0], "100644" | "100755"),
            "source inputs must be regular committed files: {path}");
        files::regular(&root.join(path))?;
        let (bytes, sha256) = blobs.read(metadata[2], |reader, _| files::hash_reader(&mut reader.take(u64::MAX)))?;
        files.push(SelectedFile { path: path.into(), bytes, sha256, mode: if metadata[0] == "100755" { 0o755 } else { 0o644 }, git_blob: metadata[2].into() });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let selection = Selection { schema: SCHEMA.into(), source_commit, files, exclusions };
    validate(&selection)?;
    clean_checkout(&root, &selection)?;
    Ok(selection)
}

fn clean_checkout(root: &Path, selection: &Selection) -> Result<()> {
    let mut untracked = vec!["ls-files", "--others", "--exclude-standard", "-z", "--"];
    untracked.extend_from_slice(SCOPES);
    for path in git(root, &untracked)?.split(|b| *b == 0).filter(|path| !path.is_empty()) {
        let path = std::str::from_utf8(path)?;
        ensure!(excluded(path).is_some(), "untracked public source must be committed before source admission: {path}");
    }
    // hash-object applies the exact path's Git clean/EOL rules, so legitimate
    // Windows checkout CRLF does not change the committed source identity.
    let mut paths = tempfile::tempfile()?;
    for file in &selection.files { writeln!(paths, "{}", file.path)?; }
    paths.seek(SeekFrom::Start(0))?;
    let output = Command::new("git").arg("-C").arg(root).args(["hash-object", "--stdin-paths"])
        .stdin(paths).output()?;
    ensure!(output.status.success(), "cannot bind working-tree bytes to committed source");
    let hashes = std::str::from_utf8(&output.stdout)?.lines().collect::<Vec<_>>();
    ensure!(hashes.len() == selection.files.len(), "incomplete current source readback");
    for (file, hash) in selection.files.iter().zip(hashes) {
        ensure!(file.git_blob == hash, "selected source differs from its committed blob: {}", file.path);
        #[cfg(unix)]
        ensure!(files::file_mode(&root.join(&file.path))? == file.mode, "selected source executable mode changed: {}", file.path);
    }
    // This also rejects tracked additions/removals in the selected namespace;
    // source-excluded artifacts remain outside the admission/read boundary.
    let mut args = vec!["diff-index", "--cached", "--quiet", "HEAD", "--"];
    args.extend_from_slice(SCOPES);
    ensure!(Command::new("git").arg("-C").arg(root).args(args).status()?.success(), "source namespace has staged membership changes");
    Ok(())
}

pub(crate) struct Blobs { child: Child, input: Option<ChildStdin>, output: BufReader<ChildStdout> }
impl Blobs {
    pub(crate) fn open(root: &Path) -> Result<Self> {
        let mut child = Command::new("git").arg("-C").arg(root).args(["cat-file", "--batch"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()?;
        let input = child.stdin.take().context("Git blob input is missing")?;
        let output = child.stdout.take().context("Git blob output is missing")?;
        Ok(Self { child, input: Some(input), output: BufReader::new(output) })
    }
    pub(crate) fn read<T>(&mut self, oid: &str, consume: impl FnOnce(&mut dyn Read, u64) -> Result<T>) -> Result<T> {
        ensure!(oid.len() == 40 && oid.bytes().all(|c| c.is_ascii_hexdigit()), "invalid selected Git blob identity");
        let input = self.input.as_mut().context("Git blob input is closed")?;
        writeln!(input, "{oid}")?; input.flush()?;
        let mut header = String::new(); self.output.read_line(&mut header)?;
        let fields = header.split_whitespace().collect::<Vec<_>>();
        ensure!(fields.len() == 3 && fields[0] == oid && fields[1] == "blob", "selected committed blob is unavailable");
        let size: u64 = fields[2].parse()?;
        ensure!(size <= 256 * 1024 * 1024, "committed public source blob exceeds verification limit");
        let mut body = Read::by_ref(&mut self.output).take(size);
        let result = consume(&mut body, size)?;
        ensure!(body.limit() == 0, "incomplete committed source blob readback");
        let mut newline = [0u8; 1]; self.output.read_exact(&mut newline)?;
        ensure!(newline == [b'\n'], "invalid Git blob framing");
        Ok(result)
    }
    pub(crate) fn copy(&mut self, file: &SelectedFile, destination: &Path) -> Result<()> {
        std::fs::create_dir_all(destination.parent().context("source resource destination parent is missing")?)?;
        let mut output = OpenOptions::new().write(true).create_new(true).open(destination)?;
        self.read(&file.git_blob, |reader, size| {
            ensure!(size == file.bytes, "committed source size changed");
            let mut reader = Hashed { inner: reader, hash: Sha256::new(), bytes: 0 };
            std::io::copy(&mut reader, &mut output)?;
            ensure!(reader.bytes == file.bytes && hex::encode(reader.hash.finalize()) == file.sha256, "committed source resource digest mismatch");
            Ok(())
        })?;
        output.sync_all()?;
        files::set_mode(destination, 0o644)
    }
}
impl Drop for Blobs {
    fn drop(&mut self) { self.input.take(); let _ = self.child.kill(); let _ = self.child.wait(); }
}

pub(crate) fn validate(selection: &Selection) -> Result<()> {
    ensure!(selection.schema == SCHEMA && selection.source_commit.len() == 40
        && selection.source_commit.bytes().all(|c| c.is_ascii_hexdigit()), "invalid selected-source identity");
    let mut paths = BTreeSet::new();
    for file in &selection.files {
        files::portable_path(&file.path)?;
        ensure!(excluded(&file.path).is_none() && source_kind(&file.path), "unapproved corresponding-source input: {}", file.path);
        ensure!(files::valid_sha(&file.sha256) && file.git_blob.len() == 40 && file.git_blob.bytes().all(|c| c.is_ascii_hexdigit()) && matches!(file.mode, 0o644 | 0o755)
            && paths.insert(file.path.as_str()), "invalid or repeated selected-source entry");
    }
    for path in REQUIRED { ensure!(paths.contains(path), "corresponding source omits required input: {path}"); }
    Ok(())
}

pub(crate) fn load(root: &Path, path: &Path, sha256: &str) -> Result<Selection> {
    let selection: Selection = files::hash_bound(path, sha256)?;
    validate(&selection)?;
    ensure!(selection == select(root)?, "selected-source membership, modes, ancestry or bytes changed; review a fresh selection");
    Ok(selection)
}

struct Hashed<R> { inner: R, hash: Sha256, bytes: u64 }
impl<R: Read> Read for Hashed<R> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(bytes)?;
        self.hash.update(&bytes[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
}

pub(crate) fn append<W: Write>(archive: &mut tar::Builder<W>, root: &Path, file: &Entry, prefix: &str, epoch: u64) -> Result<()> {
    let mut reader = Hashed { inner: files::regular(&root.join(&file.path))?, hash: Sha256::new(), bytes: 0 };
    let mut header = tar::Header::new_gnu();
    header.set_size(file.bytes); header.set_mode(file.mode); header.set_uid(0); header.set_gid(0); header.set_mtime(epoch);
    header.set_entry_type(tar::EntryType::Regular); header.set_cksum();
    archive.append_data(&mut header, format!("{prefix}/{}", file.path), &mut reader)?;
    let mut extra = [0u8; 1];
    ensure!(reader.read(&mut extra)? == 0 && reader.bytes == file.bytes && hex::encode(reader.hash.finalize()) == file.sha256,
        "source/package bytes changed during archive assembly: {}", file.path);
    Ok(())
}

pub(crate) fn append_bytes<W: Write>(archive: &mut tar::Builder<W>, name: &str, bytes: &[u8], epoch: u64) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64); header.set_mode(0o644); header.set_uid(0); header.set_gid(0); header.set_mtime(epoch);
    header.set_entry_type(tar::EntryType::Regular); header.set_cksum();
    archive.append_data(&mut header, name, bytes)?;
    Ok(())
}

pub(crate) fn create(root: &Path, selection_path: &Path, selection_sha256: &str, selected: &Selection, destination: &Path, epoch: u64) -> Result<()> {
    let parent = destination.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut prepared = tempfile::NamedTempFile::new_in(parent)?;
    {
        let compressed = GzBuilder::new().mtime(u32::try_from(epoch).context("epoch exceeds gzip timestamp range")?).write(prepared.as_file_mut(), Compression::default());
        let mut archive = tar::Builder::new(compressed);
        let mut blobs = Blobs::open(root)?;
        for file in &selected.files {
            blobs.read(&file.git_blob, |reader, size| {
                ensure!(size == file.bytes, "committed source size changed");
                let mut reader = Hashed { inner: reader, hash: Sha256::new(), bytes: 0 };
                let mut header = tar::Header::new_gnu();
                header.set_size(file.bytes); header.set_mode(file.mode); header.set_uid(0); header.set_gid(0); header.set_mtime(epoch);
                header.set_entry_type(tar::EntryType::Regular); header.set_cksum();
                archive.append_data(&mut header, format!("verdant-source/{}", file.path), &mut reader)?;
                ensure!(reader.bytes == file.bytes && hex::encode(reader.hash.finalize()) == file.sha256, "committed source blob differs from reviewed selection");
                Ok(())
            })?;
        }
        let mut bytes = Vec::new(); files::regular(selection_path)?.take(files::MAX_JSON + 1).read_to_end(&mut bytes)?;
        ensure!(hex::encode(Sha256::digest(&bytes)) == selection_sha256, "selected-source bytes changed during archiving");
        let included: Selection = files::parse_json(&bytes)?;
        ensure!(&included == selected, "source selection changed during archiving");
        append_bytes(&mut archive, "verdant-source/SOURCE-SELECTION.json", &bytes, epoch)?;
        archive.into_inner()?.finish()?;
    }
    prepared.as_file().sync_all()?;
    archive::check_source_file(prepared.path())?;
    prepared.persist_noclobber(destination).map_err(|error| error.error)?;
    files::set_mode(destination, 0o644)?;
    Ok(())
}
