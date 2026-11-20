use super::*;
use crate::index::NoteIndex;
use verdant_domain::Revision;
use verdant_session::EditorSession;

fn overlay_project(ppq: i64) -> Project {
    let mut project = Project::new_empty();
    project.ppq = ppq;
    project.tracks[0].notes = vec![Note::new(ppq * 3 / 2, ppq, 64)];
    project.tracks[0].refresh_index();
    let mut other = Track::new("和声", "blue");
    other.notes = vec![Note::new(ppq * 3 / 2, ppq, 60)];
    other.refresh_index();
    project.tracks.push(other);
    project
}

fn zoomed_viewport(ppq: i64) -> Viewport {
    let mut vp = Viewport {
        start_tick: 960.0,
        px_per_tick: 0.125,
        top_pitch: 65.5,
        row_height: 20.0,
        width: 600.0,
        height: 180.0,
        ..Viewport::default()
    };
    vp.set_quarter_ticks(ppq);
    vp.zoom_x_at(60.0, 2.0);
    vp
}

#[test]
fn ghost_geometry_follows_ppq_scaled_zoom_and_pan() {
    for ppq in [480, 1920] {
        let project = overlay_project(ppq);
        let active = &project.tracks[0].id;
        let mut vp = zoomed_viewport(ppq);
        let ghosts: Vec<_> = other_track_notes(&project, active, vp, true).collect();
        assert_eq!(ghosts.len(), 1);
        assert_eq!(ghosts[0].0.id, project.tracks[1].id);
        assert_eq!(ghosts[0].0.color.as_deref(), Some("blue"));
        assert_eq!(ghosts[0].1, Rect { x: 60.0, y: 90.0, w: 240.0, h: 20.0 });

        vp.pan_px(24.0, 20.0);
        vp.zoom_y_at(50.0, 2.0);
        let (_, rect) = other_track_notes(&project, active, vp, true).next().unwrap();
        assert!((rect.x - 36.0).abs() < 1e-9);
        assert!((rect.y - 90.0).abs() < 1e-9);
        assert_eq!(rect.w, 240.0);
        assert_eq!(rect.h, 40.0);
    }
}

#[test]
fn ghost_regions_never_hit_or_select_other_tracks() {
    let project = overlay_project(1920);
    let active = &project.tracks[0];
    let vp = zoomed_viewport(project.ppq);
    let (_, ghost) = other_track_notes(&project, &active.id, vp, true).next().unwrap();
    let revision = Revision::new(0).unwrap();
    let mut index = NoteIndex::default();
    index.sync(&project, &active.id, revision);
    for (pointer, handle, outside) in [(PointerId::Mouse, 7.0, 4.0), (PointerId::Touch(0), 22.0, 14.0)] {
        let mut session = EditorSession::new(active.id.clone());
        session.viewport = vp;
        session.selection.insert(active.notes[0].id.clone());
        let (x, y) = (ghost.x + ghost.w / 2.0, ghost.y + ghost.h / 2.0);
        let hit = index.hit_test(active, &vp, &session.selection, x, y, handle, outside);
        assert_eq!(hit, Hit::Background);
        assert_eq!(hit_test_touch(active, &vp, &session.selection, x, y), Hit::Background);
        let down = session.handle_pointer(&project, revision, pointer, InputEvent::PointerDown {
            hit, music: vp.to_music(x, y), screen: (x, y), mods: Modifiers::default(), click_count: 1,
        });
        let up = session.handle_pointer(&project, revision, pointer, InputEvent::PointerUp);
        assert!(session.selection.is_empty());
        assert!(!down.iter().chain(&up).any(|effect| matches!(effect, Effect::Commit { .. } | Effect::BeginLyricEdit(_))));

        // A marquee enclosing only the ghost must also leave selection empty.
        let (x0, y0) = (ghost.x - 2.0, ghost.y - 2.0);
        let (x1, y1) = (ghost.right() + 2.0, ghost.bottom() + 2.0);
        session.handle_pointer(&project, revision, pointer, InputEvent::PointerDown {
            hit: Hit::Background, music: vp.to_music(x0, y0), screen: (x0, y0), mods: Modifiers::default(), click_count: 1,
        });
        session.handle_pointer(&project, revision, pointer, InputEvent::PointerMove {
            music: vp.to_music(x1, y1), screen: (x1, y1), mods: Modifiers::default(),
        });
        session.handle_pointer(&project, revision, pointer, InputEvent::PointerUp);
        assert!(session.selection.is_empty());

        // The same coordinates become editable only after switching tracks.
        assert_eq!(index.hit_test(&project.tracks[1], &vp, &session.selection, x, y, handle, outside),
            Hit::Body(project.tracks[1].notes[0].id.clone()));
    }
}

#[test]
fn ghost_layer_toggle_and_viewport_cull_keep_partial_notes() {
    let mut project = overlay_project(480);
    let ppq = project.ppq;
    let other = &mut project.tracks[1];
    other.monophonic = false;
    other.notes.extend([
        Note::new(0, ppq * 2, 62),           // Crosses the left edge.
        Note::new(ppq * 3 / 2, ppq, 65),    // Crosses the top edge.
        Note::new(ppq * 3 / 2, ppq, 56),    // Crosses the bottom edge.
        Note::new(0, ppq, 60),              // Before the viewport.
        Note::new(ppq * 4, ppq, 60),        // After the viewport.
        Note::new(ppq * 3 / 2, ppq, 66),    // Above the viewport.
        Note::new(ppq * 3 / 2, ppq, 55),    // Below the viewport.
    ]);
    other.refresh_index();
    let active = &project.tracks[0].id;
    let vp = zoomed_viewport(ppq);
    assert_eq!(other_track_notes(&project, active, vp, false).count(), 0);
    let ghosts: Vec<_> = other_track_notes(&project, active, vp, true).collect();
    assert_eq!(ghosts.len(), 4);
    assert!(ghosts.iter().all(|(track, _)| track.id == project.tracks[1].id));
    assert!(ghosts.iter().any(|(_, rect)| rect.x < 0.0 && rect.right() > 0.0));
    assert!(ghosts.iter().any(|(_, rect)| rect.y < 0.0 && rect.bottom() > 0.0));
    assert!(ghosts.iter().any(|(_, rect)| rect.y < vp.height && rect.bottom() > vp.height));
}
