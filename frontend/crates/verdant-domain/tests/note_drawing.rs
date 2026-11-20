use verdant_domain::editing::{finish_note_draw, preview_note_draw};
use verdant_domain::{Note, Project, ProjectStore};

#[test]
fn horizontal_draw_commits_one_note_in_one_revision_and_one_undo_step() {
    for endpoint in [480, 1920] {
        let mut store = ProjectStore::new(Project::new_empty());
        let track = store.project().tracks[0].id.clone();
        let base = store.revision();
        let before = store.content_digest();
        let mut preview = Note::new(960, 0, 64);
        let id = preview.id.clone();

        for tick in [1200, 1440, endpoint] {
            preview_note_draw(&mut preview, 960, tick, 240);
            assert_eq!(store.revision(), base, "pointer moves must stay provisional");
            assert!(store.project().tracks[0].notes.is_empty());
        }

        let batch = finish_note_draw(track.clone(), Some(preview)).unwrap();
        assert_eq!(batch.commands.len(), 1);
        store.apply(base, batch).unwrap().unwrap();
        assert_eq!(store.revision(), base.next().unwrap());
        assert_eq!(store.undo_depth(), 1);
        let notes = &store.project().track(&track).unwrap().notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, id);
        assert_eq!(notes[0].start_tick, endpoint.min(960));
        assert_eq!(notes[0].end_tick(), endpoint.max(960));
        assert_eq!(notes[0].midi_note, 64);
        store.undo().unwrap().unwrap();
        assert_eq!(store.content_digest(), before);
    }
}

#[test]
fn zero_length_draw_and_return_to_anchor_commit_nothing() {
    let store = ProjectStore::new(Project::new_empty());
    let track = store.project().tracks[0].id.clone();
    let base = store.revision();
    let mut preview = Note::new(960, 0, 60);
    assert!(finish_note_draw(track.clone(), Some(preview.clone())).is_none());
    preview_note_draw(&mut preview, 960, 1440, 240);
    preview_note_draw(&mut preview, 960, 960, 240);
    assert!(finish_note_draw(track, Some(preview)).is_none());
    assert_eq!(store.revision(), base);
    assert_eq!(store.undo_depth(), 0);
    assert!(store.project().tracks[0].notes.is_empty());
}

#[test]
fn escape_discards_a_nonempty_preview_without_a_revision() {
    let store = ProjectStore::new(Project::new_empty());
    let track = store.project().tracks[0].id.clone();
    let base = store.revision();
    let before = store.content_digest();
    let mut note = Note::new(960, 0, 60);
    preview_note_draw(&mut note, 960, 1920, 240);
    assert_eq!(note.duration_tick, 960);
    let mut creation = Some(note);
    // Escape's cancel_edits clears the owned preview before a later release.
    drop(creation.take());
    assert!(finish_note_draw(track, creation).is_none());
    assert_eq!(store.revision(), base);
    assert_eq!(store.content_digest(), before);
    assert_eq!(store.undo_depth(), 0);
}

#[test]
fn drawing_honors_grid_minimum_and_alt_tick_resolution() {
    let mut note = Note::new(960, 0, 60);
    preview_note_draw(&mut note, 960, 961, 240);
    assert_eq!((note.start_tick, note.duration_tick), (960, 240));
    preview_note_draw(&mut note, 960, 961, 1);
    assert_eq!((note.start_tick, note.duration_tick), (960, 1));
    preview_note_draw(&mut note, 0, -240, 240);
    assert_eq!((note.start_tick, note.duration_tick), (0, 0));
}
