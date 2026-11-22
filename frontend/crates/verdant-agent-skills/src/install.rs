use std::{collections::BTreeSet, fs, path::Path};
use serde::{Deserialize, Serialize};
use crate::{Bundle, Result, SkillFile, RECEIPT_FILE, RECEIPT_SCHEMA, MAX_FILE_BYTES, MAX_MANIFEST_BYTES,
    bundle::validate_files, conflict, files, integrity, invalid, io_error, sha256, validate_name};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    name: String,
    version: String,
    bundle_sha256: String,
    files: Vec<SkillFile>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub status: String,
    pub version: Option<String>,
}
struct Change {
    name: String,
    old: Option<Receipt>,
    new: Option<Receipt>,
}

fn inspect(root: &Path, name: &str) -> Result<Receipt> {
    files::directory(root, false)?;
    let receipt_path = root.join(RECEIPT_FILE);
    if !files::exists(&receipt_path)? {
        return Err(conflict(format!("unmanaged skill directory is protected: {}", root.display())));
    }
    let receipt: Receipt = serde_json::from_slice(&files::read_regular(&receipt_path, MAX_MANIFEST_BYTES)?)
        .map_err(|error| conflict(format!("invalid install receipt {}: {error}", receipt_path.display())))?;
    if receipt.schema != RECEIPT_SCHEMA || receipt.name != name {
        return Err(conflict(format!("install receipt identity mismatch: {}", root.display())));
    }
    semver::Version::parse(&receipt.version).map_err(|_| conflict("invalid managed skill version"))?;
    if receipt.bundle_sha256.len() != 64 || !receipt.bundle_sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err(conflict("invalid managed bundle checksum"));
    }
    validate_files(&receipt.files)?;
    files::verify_tree(root, &receipt.files, true)?;
    for file in &receipt.files {
        let path = root.join(&file.path);
        let bytes = files::read_regular(&path, MAX_FILE_BYTES)?;
        if sha256(&bytes) != file.sha256 || !files::executable_matches(&path, file.executable)? {
            return Err(conflict(format!("edited skill file is protected: {}", path.display())));
        }
    }
    Ok(receipt)
}

fn validate_names(names: &[String]) -> Result<()> {
    if names.is_empty() || names.len() > 1024 { return Err(invalid("select 1–1024 skills, or use --all")); }
    let mut seen = BTreeSet::new();
    for name in names {
        validate_name(name)?;
        if !seen.insert(name) { return Err(invalid("duplicate selected skill")); }
    }
    Ok(())
}

impl Bundle {
    pub fn install(&self, destination: &Path, names: &[String], dry_run: bool) -> Result<Vec<Action>> {
        validate_names(names)?;
        let destination = files::absolute(destination)?;
        files::directory(&destination, true)?;
        let selected: Vec<_> = names.iter().map(|name| {
            self.manifest().skills.iter().find(|skill| &skill.name == name)
                .ok_or_else(|| invalid(format!("unknown bundled skill: {name}")))
        }).collect::<Result<_>>()?;
        let _lock = if dry_run { None } else { Some(files::Lock::acquire(&destination)?) };
        let mut changes = Vec::new();
        let mut actions = Vec::new();
        for skill in selected {
            let new = Receipt { schema: RECEIPT_SCHEMA.into(), name: skill.name.clone(),
                version: self.manifest().version.clone(), bundle_sha256: self.manifest_sha256().to_owned(),
                files: skill.files.clone() };
            let target = destination.join(&skill.name);
            let old = if files::exists(&target)? { Some(inspect(&target, &skill.name)?) } else { None };
            let status = if old.as_ref() == Some(&new) { "unchanged" } else if old.is_some() { "updated" } else { "installed" };
            actions.push(Action { name: skill.name.clone(), status: status.into(), version: Some(new.version.clone()) });
            if status != "unchanged" {
                changes.push(Change { name: skill.name.clone(), old, new: Some(new) });
            }
        }
        if dry_run || changes.is_empty() { return Ok(actions); }
        let temp = tempfile::Builder::new().prefix(".verdant-skills-").tempdir_in(&destination)
            .map_err(|error| io_error(&destination, error))?;
        for change in &changes {
            let new = change.new.as_ref().expect("installation has new receipt");
            let root = temp.path().join("new").join(&change.name);
            for file in &new.files {
                files::write_new(&root.join(&file.path), self.read(&change.name, &file.path)?, file.executable)?;
            }
            let bytes = serde_json::to_vec_pretty(new).map_err(|error| integrity(error.to_string()))?;
            files::write_new(&root.join(RECEIPT_FILE), &bytes, false)?;
            if inspect(&root, &change.name)? != *new { return Err(integrity("staged skill changed")); }
        }
        // Once originals can enter this directory, never rely on TempDir's
        // unconditional destructor: failed transactions retain recovery data.
        let workspace = temp.keep();
        commit(&destination, &workspace, &changes)?;
        Ok(actions)
    }
}

/// None selects all recognized managed skill directories, including skills that
/// disappeared from a newer bundle. Named uninstall also works without a bundle.
pub fn uninstall(destination: &Path, names: Option<&[String]>, dry_run: bool) -> Result<Vec<Action>> {
    let destination = files::absolute(destination)?;
    files::directory(&destination, true)?;
    if let Some(names) = names { validate_names(names)?; }
    if !files::exists(&destination)? {
        return Ok(names.unwrap_or(&[]).iter().map(|name| Action {
            name: name.clone(), status: "absent".into(), version: None,
        }).collect());
    }
    let _lock = if dry_run { None } else { Some(files::Lock::acquire(&destination)?) };
    let selected = match names {
        Some(names) => names.to_vec(),
        None => {
            let mut names = Vec::new();
            for entry in fs::read_dir(&destination).map_err(|error| io_error(&destination, error))? {
                let entry = entry.map_err(|error| io_error(&destination, error))?;
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue; };
                if validate_name(&name).is_err() { continue; }
                if !files::exists(&entry.path())? { continue; }
                if entry.file_type().map_err(|error| io_error(&entry.path(), error))?.is_dir()
                    && files::exists(&entry.path().join(RECEIPT_FILE))? { names.push(name); }
            }
            names.sort();
            names
        },
    };
    let mut changes = Vec::new();
    let mut actions = Vec::new();
    for name in selected {
        let target = destination.join(&name);
        let old = if files::exists(&target)? { Some(inspect(&target, &name)?) } else { None };
        actions.push(Action { name: name.clone(), status: if old.is_some() { "removed" } else { "absent" }.into(),
            version: old.as_ref().map(|receipt| receipt.version.clone()) });
        if old.is_some() { changes.push(Change { name, old, new: None }); }
    }
    if dry_run || changes.is_empty() { return Ok(actions); }
    let workspace = tempfile::Builder::new().prefix(".verdant-skills-").tempdir_in(&destination)
        .map_err(|error| io_error(&destination, error))?.keep();
    commit(&destination, &workspace, &changes)?;
    Ok(actions)
}

fn commit(destination: &Path, workspace: &Path, changes: &[Change]) -> Result<()> {
    let backups = workspace.join("old");
    fs::create_dir(&backups).map_err(|error| io_error(&backups, error))?;
    let mut moved_old = Vec::new();
    let mut moved_new = Vec::new();
    let outcome = (|| {
        for (index, change) in changes.iter().enumerate() {
            files::directory(destination, false)?;
            let target = destination.join(&change.name);
            if let Some(old) = &change.old {
                if inspect(&target, &change.name)? != *old { return Err(conflict("skill changed during installation")); }
                fs::rename(&target, backups.join(&change.name)).map_err(|error| io_error(&target, error))?;
                moved_old.push(index);
                if inspect(&backups.join(&change.name), &change.name)? != *old {
                    return Err(conflict("skill changed during installation"));
                }
            } else if files::exists(&target)? {
                return Err(conflict(format!("new skill directory appeared during installation: {}", target.display())));
            }
            if change.new.is_some() {
                fs::rename(workspace.join("new").join(&change.name), &target).map_err(|error| io_error(&target, error))?;
                moved_new.push(index);
            }
        }
        // Check moved originals before deleting anything. A racing edit is
        // restored or preserved in the recovery workspace, never discarded.
        for &index in &moved_old {
            let change = &changes[index];
            if inspect(&backups.join(&change.name), &change.name)? != *change.old.as_ref().unwrap() {
                return Err(conflict("backup changed during skill transaction"));
            }
        }
        Ok(())
    })();
    if let Err(error) = outcome {
        let rollback = rollback(destination, workspace, changes, &moved_old, &moved_new);
        return Err(conflict(format!("{error}; {}",
            match rollback {
                Ok(()) => "previous installation restored".to_owned(),
                Err(reason) => format!("recovery files preserved at {}: {reason}", workspace.display()),
            })));
    }
    // All old files have been verified; new files remain in the destination.
    fs::remove_dir_all(workspace).map_err(|error| io_error(workspace, error))?;
    Ok(())
}

fn rollback(destination: &Path, workspace: &Path, changes: &[Change], moved_old: &[usize], moved_new: &[usize]) -> Result<()> {
    let discard = workspace.join("discard");
    fs::create_dir_all(&discard).map_err(|error| io_error(&discard, error))?;
    for &index in moved_new.iter().rev() {
        let change = &changes[index];
        let target = destination.join(&change.name);
        // Preserve changed output in the workspace even when restoring old files.
        fs::rename(&target, discard.join(&change.name)).map_err(|error| io_error(&target, error))?;
    }
    for &index in moved_old.iter().rev() {
        let change = &changes[index];
        let target = destination.join(&change.name);
        if files::exists(&target)? { return Err(conflict("rollback destination occupied; originals retained")); }
        fs::rename(workspace.join("old").join(&change.name), &target).map_err(|error| io_error(&target, error))?;
    }
    for &index in moved_new {
        let change = &changes[index];
        if inspect(&discard.join(&change.name), &change.name)? != *change.new.as_ref().unwrap() {
            return Err(conflict("changed newly installed files retained"));
        }
    }
    fs::remove_dir_all(workspace).map_err(|error| io_error(workspace, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Manifest, Skill, BUNDLE_SCHEMA};

    fn bundle(version: &str, content: &str) -> Bundle {
        let document = format!("---\nname: verdant-test\ndescription: Test isolated installation.\n---\n{content}\n");
        let manifest = Manifest { schema: BUNDLE_SCHEMA.into(), version: version.into(), skills: vec![
            Skill { name: "verdant-test".into(), description: "Test isolated installation.".into(),
                files: vec![SkillFile { path: "SKILL.md".into(), sha256: sha256(document.as_bytes()), executable: false }] },
        ] };
        Bundle::from_reader(&serde_json::to_vec(&manifest).unwrap(), |_| Ok(document.as_bytes().to_vec())).unwrap()
    }

    #[test]
    fn install_update_uninstall_are_idempotent_and_dry_run_is_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().canonicalize().unwrap().join("space 中文 skills");
        let names = vec!["verdant-test".to_owned()];
        let first = bundle("0.1.0", "First");
        assert_eq!(first.install(&dest, &names, true).unwrap()[0].status, "installed");
        assert!(!dest.exists());
        assert_eq!(first.install(&dest, &names, false).unwrap()[0].status, "installed");
        assert_eq!(first.install(&dest, &names, false).unwrap()[0].status, "unchanged");
        assert_eq!(bundle("0.2.0", "Second").install(&dest, &names, false).unwrap()[0].status, "updated");
        assert!(fs::read_to_string(dest.join("verdant-test/SKILL.md")).unwrap().contains("Second"));
        assert_eq!(uninstall(&dest, None, true).unwrap()[0].status, "removed");
        assert!(dest.join("verdant-test").exists());
        assert_eq!(uninstall(&dest, None, false).unwrap()[0].status, "removed");
        assert_eq!(uninstall(&dest, Some(&names), false).unwrap()[0].status, "absent");
    }

    #[test]
    fn preserves_local_edits_untracked_files_and_unmanaged_directories() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().canonicalize().unwrap().join("skills");
        let names = vec!["verdant-test".to_owned()];
        let first = bundle("0.1.0", "First");
        first.install(&dest, &names, false).unwrap();
        let skill = dest.join("verdant-test");
        fs::write(skill.join("SKILL.md"), "user changes").unwrap();
        assert_eq!(first.install(&dest, &names, false).unwrap_err().code, "CONFLICT");
        assert_eq!(uninstall(&dest, Some(&names), false).unwrap_err().code, "CONFLICT");
        assert_eq!(fs::read_to_string(skill.join("SKILL.md")).unwrap(), "user changes");
        fs::write(skill.join("SKILL.md"), first.read("verdant-test", "SKILL.md").unwrap()).unwrap();
        fs::write(skill.join("private.txt"), "user data").unwrap();
        assert!(uninstall(&dest, None, false).is_err());
        fs::remove_file(skill.join("private.txt")).unwrap();
        fs::remove_file(skill.join(RECEIPT_FILE)).unwrap();
        assert!(first.install(&dest, &names, false).is_err());
        assert!(skill.join("SKILL.md").exists());
    }

    #[test]
    fn rejects_paths_hashes_and_unknown_selection_before_writes() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = bundle("0.1.0", "First");
        for path in ["../x", "/x", "x\\y", "x/../y", "CON.md", "a:b", "x./y", "x//y", RECEIPT_FILE] {
            assert!(crate::bundle::validate_path(path).is_err(), "{path}");
        }
        assert!(bundle.install(&temp.path().join("missing"), &["unknown".into()], false).is_err());
        assert!(!temp.path().join("missing").exists());
        let manifest = serde_json::to_vec(bundle.manifest()).unwrap();
        assert_eq!(Bundle::from_reader(&manifest, |_| Ok(b"bad".to_vec())).err().unwrap().code, "INTEGRITY_ERROR");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symbolic_links_in_destinations_sources_and_managed_files() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().canonicalize().unwrap().join("outside");
        fs::create_dir(&outside).unwrap();
        let dest = temp.path().canonicalize().unwrap().join("linked");
        symlink(&outside, &dest).unwrap();
        let bundle = bundle("0.1.0", "First");
        let names = vec!["verdant-test".to_owned()];
        assert!(bundle.install(&dest, &names, false).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        let dest = temp.path().canonicalize().unwrap().join("skills");
        bundle.install(&dest, &names, false).unwrap();
        let managed = dest.join("verdant-test/SKILL.md");
        fs::remove_file(&managed).unwrap();
        let private = outside.join("private.txt");
        fs::write(&private, "keep").unwrap();
        symlink(&private, &managed).unwrap();
        assert!(uninstall(&dest, None, false).is_err());
        assert_eq!(fs::read_to_string(&private).unwrap(), "keep");
    }
}
