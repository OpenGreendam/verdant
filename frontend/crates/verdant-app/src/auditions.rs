//! User-provided listening projects, loaded before GPUI starts. The catalogue
//! contains titles and relative project paths, never bundled third-party scores.

use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AuditionProject {
    pub title: String,
    pub path: PathBuf,
}

#[derive(Deserialize)]
struct Catalog {
    schema: String,
    projects: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    title: String,
    path: String,
}

pub fn load_catalog(directory: &Path) -> Result<Vec<AuditionProject>, String> {
    let root = directory.canonicalize().map_err(|e| format!("{}: {e}", directory.display()))?;
    let bytes = std::fs::read(root.join("manifest.json")).map_err(|e| e.to_string())?;
    let catalog: Catalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if catalog.schema != "verdant-audition-catalog/1" || catalog.projects.len() > 32 {
        return Err("expected verdant-audition-catalog/1 with at most 32 projects".into());
    }
    catalog
        .projects
        .into_iter()
        .map(|entry| {
            let relative = Path::new(&entry.path);
            let extension = relative.extension().and_then(|value| value.to_str()).unwrap_or_default();
            verdant_domain::io::reject_retired_project_format(extension).map_err(|error| error.to_string())?;
            if entry.title.trim().is_empty()
                || entry.title.chars().count() > 120
                || relative.as_os_str().is_empty()
                || relative.components().any(|c| !matches!(c, Component::Normal(_)))
                || !verdant_domain::io::has_project_extension(relative)
            {
                return Err(format!("invalid audition entry {:?}", entry.title));
            }
            let path = root.join(relative).canonicalize().map_err(|e| format!("{}: {e}", entry.path))?;
            if !path.starts_with(&root) || !path.is_file() {
                return Err(format!("audition project {} must remain inside its directory", entry.path));
            }
            Ok(AuditionProject { title: entry.title, path })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_keeps_titles_and_refuses_parent_paths() {
        let root = std::env::temp_dir().join(format!("verdant-auditions-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("song.verp"), "{}").unwrap();
        let write = |path: &str| {
            std::fs::write(
                root.join("manifest.json"),
                serde_json::to_vec(&serde_json::json!({
                    "schema":"verdant-audition-catalog/1", "projects":[{"title":"热异常 · 中文已填词片段", "path":path}]
                }))
                .unwrap(),
            )
            .unwrap()
        };
        write("song.verp");
        let entries = load_catalog(&root).unwrap();
        assert_eq!(entries[0].title, "热异常 · 中文已填词片段");
        assert_eq!(entries[0].path, root.canonicalize().unwrap().join("song.verp"));
        write("../song.verp");
        assert!(load_catalog(&root).is_err());
        write("missing.verp");
        assert!(load_catalog(&root).is_err());
        for path in ["old.json", "old.verdant.JSON", "old.GDMProj"] {
            write(path);
            assert_eq!(load_catalog(&root).unwrap_err(), verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
