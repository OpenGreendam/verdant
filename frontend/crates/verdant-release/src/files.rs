use std::{fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Component, Path, PathBuf}};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub(crate) const MAX_JSON: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub mode: u32,
}

pub(crate) fn valid_sha(sha: &str) -> bool {
    sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn portable_path(path: &str) -> Result<()> {
    ensure!(!path.is_empty() && !path.starts_with('/') && !path.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| {
            let stem = part.split('.').next().unwrap_or("");
            !matches!(part, "" | "." | "..") && !part.ends_with([' ', '.'])
                && !["CON", "PRN", "AUX", "NUL", "CLOCK$", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"].iter().any(|name| stem.eq_ignore_ascii_case(name))
        }), "unsafe member path: {path}");
    Ok(())
}

pub(crate) fn regular(path: &Path) -> Result<File> {
    for ancestor in path.ancestors() {
        if ancestor.as_os_str().is_empty() { continue; }
        ensure!(!fs::symlink_metadata(ancestor).with_context(|| format!("missing input {}", ancestor.display()))?.file_type().is_symlink(),
            "symlink package input: {}", ancestor.display());
    }
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    ensure!(file.metadata()?.is_file(), "input is not a regular file: {}", path.display());
    Ok(file)
}

pub(crate) fn input(root: &Path, relative: &str) -> Result<PathBuf> {
    portable_path(relative)?;
    let path = root.join(relative);
    regular(&path)?;
    Ok(path)
}

pub(crate) fn hash_reader(reader: &mut impl Read) -> Result<(u64, String)> {
    let mut buffer = [0u8; 64 * 1024];
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 { break; }
        bytes = bytes.checked_add(count as u64).context("file size overflow")?;
        hash.update(&buffer[..count]);
    }
    Ok((bytes, hex::encode(hash.finalize())))
}

pub(crate) fn digest(path: &Path) -> Result<String> { Ok(hash_reader(&mut regular(path)?)?.1) }

pub(crate) fn file_mode(path: &Path) -> Result<u32> {
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        Ok(if fs::metadata(path)?.permissions().mode() & 0o111 != 0 { 0o755 } else { 0o644 })
    }
    #[cfg(not(unix))] {
        Ok(if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe")) { 0o755 } else { 0o644 })
    }
}

pub(crate) fn set_mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))] let _ = (path, mode);
    Ok(())
}

pub(crate) fn record(root: &Path, path: &Path) -> Result<Entry> {
    let relative = path.strip_prefix(root)?.to_str().context("non-UTF-8 package path")?.replace('\\', "/");
    portable_path(&relative)?;
    let (bytes, sha256) = hash_reader(&mut regular(path)?)?;
    Ok(Entry { path: relative, bytes, sha256, mode: file_mode(path)? })
}

pub(crate) fn walk(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        ensure!(fs::symlink_metadata(root)?.is_dir(), "not a regular directory: {}", root.display());
        let mut entries = fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let kind = entry.file_type()?;
            if kind.is_file() { files.push(entry.path()); }
            else if kind.is_dir() { visit(&entry.path(), files)?; }
            else { bail!("symlink or special package member: {}", entry.path().display()); }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, &mut files)?;
    Ok(files)
}

pub(crate) fn copy(source: &Path, destination: &Path, executable: bool) -> Result<()> {
    let mut input = regular(source)?;
    let parent = destination.parent().context("output has no parent")?;
    fs::create_dir_all(parent)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(destination)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    set_mode(destination, if executable { 0o755 } else { 0o644 })
}

pub(crate) fn write_new(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) { fs::create_dir_all(parent)?; }
    let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
    output.write_all(bytes)?;
    output.sync_all()?;
    set_mode(path, if executable { 0o755 } else { 0o644 })
}

pub(crate) fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn write_new_json(path: &Path, value: &impl Serialize) -> Result<()> {
    write_new(path, &json_bytes(value)?, false)
}

// Value normally discards duplicate keys. All admission JSON is first parsed
// through this visitor so an auditor and producer cannot interpret different keys.
struct Unique(serde_json::Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("JSON without duplicate keys") }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v).map(|v| Unique(v.into())).ok_or_else(|| E::custom("nonfinite JSON"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
            fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<Unique, E> { Ok(Unique(v.into())) }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Unique, E> { Ok(Unique(serde_json::Value::Null)) }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Unique, E> { self.visit_unit() }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(value)) = seq.next_element()? { values.push(value); }
                Ok(Unique(values.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> std::result::Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if values.insert(key.clone(), value).is_some() { return Err(serde::de::Error::custom(format!("duplicate JSON key {key}"))); }
                }
                Ok(Unique(values.into()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

pub(crate) fn parse_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    ensure!(bytes.len() as u64 <= MAX_JSON, "JSON exceeds admission limit");
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let Unique(value) = Unique::deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(serde_json::from_value(value)?)
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = regular(path)?;
    ensure!(file.metadata()?.len() <= MAX_JSON, "JSON exceeds admission limit: {}", path.display());
    let mut bytes = Vec::new();
    file.take(MAX_JSON + 1).read_to_end(&mut bytes)?;
    parse_json(&bytes)
}

pub(crate) fn hash_bound<T: DeserializeOwned>(path: &Path, expected: &str) -> Result<T> {
    ensure!(valid_sha(expected), "expected a lowercase SHA256");
    let mut file = regular(path)?;
    ensure!(file.metadata()?.len() <= MAX_JSON, "admission manifest exceeds limit");
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(MAX_JSON + 1).read_to_end(&mut bytes)?;
    ensure!(hex::encode(Sha256::digest(&bytes)) == expected, "admission manifest digest mismatch: {}", path.display());
    parse_json(&bytes)
}

pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() { path.to_owned() } else { std::env::current_dir()?.join(path) };
    ensure!(!path.components().any(|part| matches!(part, Component::ParentDir)), "explicit paths must not contain ..");
    Ok(path)
}
