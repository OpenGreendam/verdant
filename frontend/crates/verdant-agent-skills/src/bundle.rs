use std::{collections::{BTreeMap, BTreeSet}, path::Path};
use sha2::{Digest, Sha256};
use crate::{BUNDLE_SCHEMA, Manifest, MAX_FILE_BYTES, MAX_MANIFEST_BYTES,
    RECEIPT_FILE, Result, SkillFile, files, integrity, invalid};

pub fn sha256(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 || name.starts_with('-') || name.ends_with('-')
        || name.contains("--") || !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(invalid("skill name must be 1–64 lowercase ASCII letters, digits or single hyphens"));
    }
    Ok(())
}

pub(crate) fn validate_path(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 1024 || path.split('/').count() > 32 {
        return Err(invalid("skill file path is empty or too long"));
    }
    for part in path.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit());
        if part.is_empty() || matches!(part, "." | "..") || part.ends_with(['.', ' ']) || reserved
            || part.chars().any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
            || part.eq_ignore_ascii_case(RECEIPT_FILE)
        {
            return Err(invalid(format!("unsafe or nonportable skill file path: {path}")));
        }
    }
    Ok(())
}

pub(crate) fn validate_files(entries: &[SkillFile]) -> Result<()> {
    if entries.is_empty() || entries.len() > 512 { return Err(invalid("skill requires 1–512 files")); }
    let mut seen = BTreeSet::new();
    for file in entries {
        validate_path(&file.path)?;
        if !seen.insert(file.path.to_lowercase()) { return Err(invalid("duplicate or case-colliding skill file path")); }
        if file.sha256.len() != 64 || !file.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(invalid(format!("invalid SHA-256 for {}", file.path)));
        }
    }
    if !entries.iter().any(|file| file.path == "SKILL.md" && !file.executable) {
        return Err(invalid("every skill requires a nonexecutable SKILL.md"));
    }
    // A file cannot also be the parent directory of another file.
    for path in &seen {
        for (offset, _) in path.match_indices('/') {
            if seen.contains(&path[..offset]) { return Err(invalid("skill file/directory path collision")); }
        }
    }
    Ok(())
}

pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    if bytes.len() > MAX_MANIFEST_BYTES { return Err(invalid("skill manifest exceeds 4 MiB")); }
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|error| invalid(format!("invalid skill manifest: {error}")))?;
    if manifest.schema != BUNDLE_SCHEMA { return Err(invalid("unsupported skill bundle schema")); }
    semver::Version::parse(&manifest.version).map_err(|_| invalid("skill bundle version must be semantic versioning"))?;
    if manifest.skills.is_empty() || manifest.skills.len() > 1024 { return Err(invalid("bundle requires 1–1024 skills")); }
    let mut names = BTreeSet::new();
    let mut count = 0;
    for skill in &manifest.skills {
        validate_name(&skill.name)?;
        if !names.insert(&skill.name) { return Err(invalid("duplicate skill name")); }
        if skill.description.trim().is_empty() || skill.description.len() > 4096 {
            return Err(invalid(format!("invalid skill description: {}", skill.name)));
        }
        validate_files(&skill.files)?;
        count += skill.files.len();
        if count > 10000 { return Err(invalid("skill bundle exceeds 10000 files")); }
    }
    Ok(manifest)
}

/// Immutable, fully verified in-memory payload; installation cannot copy bytes
/// changed on disk after validation.
pub struct Bundle {
    manifest: Manifest,
    manifest_sha256: String,
    content: BTreeMap<String, Vec<u8>>,
}
impl Bundle {
    pub fn load(root: &Path) -> Result<Self> {
        let root = files::absolute(root)?;
        files::directory(&root, false)?;
        let manifest = files::read_regular(&root.join("manifest.json"), MAX_MANIFEST_BYTES)?;
        let bundle = Self::from_reader(&manifest, |path| files::read_regular(&root.join(path), MAX_FILE_BYTES))?;
        for skill in &bundle.manifest.skills {
            files::verify_tree(&root.join(&skill.name), &skill.files, false)?;
        }
        Ok(bundle)
    }

    /// Also supports package validators reading indexed archive members.
    pub fn from_reader(manifest: &[u8], mut read: impl FnMut(&str) -> Result<Vec<u8>>) -> Result<Self> {
        let parsed = parse_manifest(manifest)?;
        let mut content = BTreeMap::new();
        let mut total = 0usize;
        for skill in &parsed.skills {
            for file in &skill.files {
                let path = format!("{}/{}", skill.name, file.path);
                let bytes = read(&path)?;
                total = total.checked_add(bytes.len()).ok_or_else(|| invalid("skill bundle size overflow"))?;
                if bytes.len() > MAX_FILE_BYTES || total > 128 * 1024 * 1024 {
                    return Err(invalid("skill bundle exceeds size limit"));
                }
                if sha256(&bytes) != file.sha256 { return Err(integrity(format!("skill file checksum mismatch: {path}"))); }
                if file.path == "SKILL.md" {
                    let text = std::str::from_utf8(&bytes).map_err(|_| invalid(format!("{path} must be UTF-8")))?;
                    let frontmatter = text.strip_prefix("---\n").and_then(|text| text.split_once("\n---").map(|part| part.0))
                        .ok_or_else(|| invalid(format!("{path} requires YAML frontmatter and LF newlines")))?;
                    let name = frontmatter.lines().find_map(|line| line.strip_prefix("name:").map(str::trim));
                    if name.map(|name| name.trim_matches(['\'', '"'])) != Some(skill.name.as_str()) {
                        return Err(invalid(format!("{path} frontmatter name does not match manifest")));
                    }
                }
                content.insert(path, bytes);
            }
        }
        Ok(Self { manifest: parsed, manifest_sha256: sha256(manifest), content })
    }

    pub fn manifest(&self) -> &Manifest { &self.manifest }
    pub fn manifest_sha256(&self) -> &str { &self.manifest_sha256 }
    pub fn read(&self, name: &str, path: &str) -> Result<&[u8]> {
        self.content.get(&format!("{name}/{path}")).map(Vec::as_slice)
            .ok_or_else(|| invalid(format!("unknown bundled skill file: {name}/{path}")))
    }
}
