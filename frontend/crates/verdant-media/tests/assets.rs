mod common;
use std::{fs, path::PathBuf};
use verdant_domain::{Project, TempoMap, Track};
use verdant_media::{AssetPolicy, AudioClip, ImportLimits, MediaError, MediaImporter, write_clips};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("verdant-media-owned-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap(); Self(root)
    }
}
impl Drop for Scratch { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

#[test]
fn reload_verifies_bytes_and_exact_file_authorization_excludes_siblings() {
    let scratch = Scratch::new();
    let selected = scratch.0.join("selected.wav"); let sibling = scratch.0.join("sibling.wav");
    fs::write(&selected, common::pcm16(&[1, 2, 3], 1, 48_000)).unwrap();
    fs::write(&sibling, common::pcm16(&[4, 5, 6], 1, 48_000)).unwrap();
    let importer = MediaImporter::new(AssetPolicy::for_file(&selected).unwrap(), ImportLimits::default());
    let imported = importer.import_file(&selected).unwrap();
    assert!(matches!(importer.import_file(&sibling), Err(MediaError::UnauthorizedPath)));
    assert_eq!(importer.load_source(&imported.source).unwrap().samples.as_ref(), imported.audio.samples.as_ref());
    fs::write(&selected, common::pcm16(&[7, 8, 9], 1, 48_000)).unwrap();
    assert!(matches!(importer.load_source(&imported.source), Err(MediaError::SourceChanged)));
}

#[cfg(unix)]
#[test]
fn symlink_escape_from_authorized_root_is_rejected_before_source_read() {
    let scratch = Scratch::new(); let allowed = scratch.0.join("allowed"); fs::create_dir(&allowed).unwrap();
    let outside = scratch.0.join("outside.wav"); fs::write(&outside, common::pcm16(&[1, 2], 1, 48_000)).unwrap();
    let link = allowed.join("escape.wav"); std::os::unix::fs::symlink(&outside, &link).unwrap();
    let importer = MediaImporter::new(AssetPolicy::new([allowed]).unwrap(), ImportLimits::default());
    assert!(matches!(importer.import_file(&link), Err(MediaError::UnauthorizedPath)));
}

#[test]
fn prepared_project_reuses_sources_and_does_not_authorize_paths_from_the_document() {
    let scratch = Scratch::new(); let path = scratch.0.join("audio.wav");
    fs::write(&path, common::pcm16(&[10; 16], 1, 48_000)).unwrap();
    let importer = MediaImporter::new(AssetPolicy::for_file(&path).unwrap(), ImportLimits::default());
    let imported = importer.import_file(&path).unwrap();
    let mut track = Track::new("Audio", "blue");
    let first = AudioClip::new(imported.source.clone(), track.id.clone(), 0);
    let mut second = first.clone(); second.id = uuid::Uuid::new_v4(); second.start_tick = 960;
    write_clips(&mut track, &[first, second]).unwrap();
    let mut project = Project::new_empty(); project.tracks = vec![track];
    let prepared = importer.prepare_project_clips(&project, &TempoMap::constant(120.0), 48_000).unwrap();
    assert_eq!(prepared.len(), 2); assert_eq!(prepared[1].start, 24_000);
    assert!(std::sync::Arc::ptr_eq(&prepared[0].samples, &prepared[1].samples));
    let denied = MediaImporter::new(AssetPolicy::new(Vec::<PathBuf>::new()).unwrap(), ImportLimits::default());
    assert!(matches!(denied.prepare_project_clips(&project, &TempoMap::constant(120.0), 48_000), Err(MediaError::UnauthorizedPath)));
}
