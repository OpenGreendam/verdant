//! These are live EditorSession behaviors, not a second handwritten reducer
//! compared with a generated enum. Every drag runs the production generated
//! handler and explicit bindings used by the app.

use verdant_domain::{Command, CommandBatch, Edge, Note, Project, ProjectStore};
use verdant_session::{CancelReason, EditorSession, Effect, Hit, InputEvent, Modifiers, MusicPoint, PointerId, Preview};

fn setup() -> (ProjectStore, EditorSession) {
    let mut project = Project::new_empty();
    let track = project.tracks[0].id.clone();
    for (id, tick, pitch) in [("a", 960, 60), ("b", 1440, 64)] {
        let mut note = Note::new(tick, 480, pitch);
        note.id = id.into();
        project.tracks[0].notes.push(note);
    }
    (ProjectStore::new(project), EditorSession::new(track))
}

fn down(hit: Hit, mods: Modifiers) -> InputEvent {
    InputEvent::PointerDown { hit, music: MusicPoint { tick: 1000.0, pitch: 60.0 }, screen: (100.0, 100.0), mods, click_count: 1 }
}
fn motion(tick: f64, pitch: f64, screen: (f64, f64)) -> InputEvent {
    InputEvent::PointerMove { music: MusicPoint { tick, pitch }, screen, mods: Modifiers::default() }
}
fn feed(store: &ProjectStore, session: &mut EditorSession, event: InputEvent) -> Vec<Effect> {
    session.handle(&store.snapshot(), store.revision(), event)
}
fn commit(store: &mut ProjectStore, session: &mut EditorSession, effects: Vec<Effect>) -> usize {
    let mut count = 0;
    for effect in effects {
        if let Effect::Commit { base, batch, select_after } = effect {
            assert_eq!(batch.commands.len(), 1);
            assert!(store.apply(base, batch).unwrap().is_some());
            if let Some(ids) = select_after { session.selection = ids.into_iter().collect(); }
            count += 1;
        }
    }
    count
}

#[test]
fn live_move_handles_selected_unselected_and_group_as_one_undo() {
    for selected in [false, true] {
        let (mut store, mut s) = setup();
        if selected { s.select_all(store.project()); }
        let base = store.revision();
        assert_eq!(feed(&store, &mut s, down(Hit::Body("a".into()), Modifiers::default())), vec![Effect::CapturePointer]);
        feed(&store, &mut s, motion(1240.0, 62.0, (200.0, 60.0)));
        assert_eq!(store.revision(), base);
        assert_eq!(store.undo_depth(), 0);
        let effects = feed(&store, &mut s, InputEvent::PointerUp);
        assert_eq!(effects.iter().filter(|e| matches!(e, Effect::ReleasePointer)).count(), 1);
        assert_eq!(commit(&mut store, &mut s, effects), 1);
        let notes = &store.project().tracks[0].notes;
        assert_eq!((notes[0].start_tick, notes[0].midi_note), (1200, 62));
        assert_eq!((notes[1].start_tick, notes[1].midi_note), if selected { (1680, 66) } else { (1440, 64) });
        assert_eq!(store.undo_depth(), 1);
        assert!(s.gesture.is_none() && !s.pointer_captured && s.pointer_owner().is_none());
        store.undo().unwrap();
        assert_eq!(store.project().tracks[0].notes[0].start_tick, 960);
    }
}

#[test]
fn live_move_updates_after_returning_inside_the_start_threshold() {
    let (store, mut s) = setup();
    feed(&store, &mut s, down(Hit::Body("a".into()), Modifiers::default()));
    feed(&store, &mut s, motion(1240.0, 62.0, (200.0, 60.0)));
    assert_eq!(s.gesture_name(), "dragging");
    feed(&store, &mut s, motion(1000.0, 60.0, (101.0, 100.0)));
    assert!(matches!(s.gesture.as_ref().unwrap().preview, Preview::Move { delta_tick: 0, delta_semitones: 0, .. }));
    let effects = feed(&store, &mut s, InputEvent::PointerUp);
    assert!(!effects.iter().any(|e| matches!(e, Effect::Commit { .. })));
}

#[test]
fn live_duplicate_modifier_commits_new_ids_only_on_success() {
    let (mut store, mut s) = setup();
    let alt = Modifiers { alt: true, ..Default::default() };
    for cancel in [true, false] {
        let digest = store.content_digest();
        feed(&store, &mut s, down(Hit::Body("a".into()), alt));
        feed(&store, &mut s, motion(1240.0, 62.0, (200.0, 60.0)));
        assert!(matches!(s.gesture.as_ref().unwrap().preview, Preview::Move { duplicate: true, .. }));
        assert_eq!(store.content_digest(), digest);
        let effects = feed(&store, &mut s, if cancel { InputEvent::Cancel(CancelReason::PointerCancel) } else { InputEvent::PointerUp });
        assert_eq!(commit(&mut store, &mut s, effects), usize::from(!cancel));
        assert!(!s.pointer_captured);
    }
    assert_eq!(store.project().note_count(), 3);
    assert_eq!(store.undo_depth(), 1);
    assert_eq!(s.selection.len(), 1);
    assert!(!s.selection.contains(&"a".into()));
}

#[test]
fn live_resize_and_marquee_use_their_generated_effect_paths() {
    let (mut store, mut s) = setup();
    feed(&store, &mut s, down(Hit::Handle("a".into(), Edge::End), Modifiers::default()));
    feed(&store, &mut s, motion(-1000.0, 60.0, (0.0, 100.0)));
    let effects = feed(&store, &mut s, InputEvent::PointerUp);
    assert_eq!(commit(&mut store, &mut s, effects), 1);
    assert_eq!(store.project().tracks[0].notes[0].duration_tick, 1);
    let revision = store.revision();
    feed(&store, &mut s, InputEvent::PointerDown { hit: Hit::Background, music: MusicPoint { tick: 1400.0, pitch: 63.0 }, screen: (0.0, 0.0), mods: Modifiers::default(), click_count: 1 });
    feed(&store, &mut s, motion(2000.0, 66.0, (100.0, 100.0)));
    let effects = feed(&store, &mut s, InputEvent::PointerUp);
    assert_eq!(commit(&mut store, &mut s, effects), 0);
    assert_eq!(s.selection, ["b".into()].into());
    assert_eq!(store.revision(), revision);
    assert_eq!(store.undo_depth(), 1);
}

#[test]
fn every_live_gesture_cancel_releases_once_and_stale_release_never_commits() {
    for hit in [Hit::Body("a".into()), Hit::Handle("a".into(), Edge::Start), Hit::Background] {
        for mods in [Modifiers::default(), Modifiers { alt: true, ..Default::default() }] {
            for reason in [CancelReason::Escape, CancelReason::FocusLost, CancelReason::PointerCancel] {
                let (store, mut s) = setup();
                let digest = store.content_digest();
                feed(&store, &mut s, down(hit.clone(), mods));
                feed(&store, &mut s, motion(1240.0, 62.0, (200.0, 60.0)));
                s.autoscroll_active = true;
                assert_eq!(feed(&store, &mut s, InputEvent::Cancel(reason)), vec![Effect::ReleasePointer]);
                assert!(feed(&store, &mut s, InputEvent::PointerUp).is_empty());
                assert_eq!(store.content_digest(), digest);
                assert!(!s.autoscroll_active && s.pointer_owner().is_none());
            }
            let (mut store, mut s) = setup();
            feed(&store, &mut s, down(hit.clone(), mods));
            feed(&store, &mut s, motion(1240.0, 62.0, (200.0, 60.0)));
            store.apply(store.revision(), CommandBatch::single("other edit", Command::SetLyric { track: s.track.clone(), note_id: "b".into(), text: "坝".into() })).unwrap();
            let effects = feed(&store, &mut s, InputEvent::PointerUp);
            assert!(!effects.iter().any(|e| matches!(e, Effect::Commit { .. })));
            assert!(effects.iter().any(|e| matches!(e, Effect::Notice(_))));
            assert!(!s.pointer_captured);
            assert_eq!(store.undo_depth(), 1);
        }
    }
}

#[test]
fn capture_latches_touch_contact_and_rejects_other_pointer_streams() {
    let (store, mut s) = setup();
    let p = store.snapshot();
    let rev = store.revision();
    assert_eq!(s.handle_pointer(&p, rev, PointerId::Touch(7), down(Hit::Body("a".into()), Modifiers::default())), vec![Effect::CapturePointer]);
    assert!(s.handle_pointer(&p, rev, PointerId::Touch(8), down(Hit::Body("b".into()), Modifiers::default())).is_empty());
    assert!(s.handle_pointer(&p, rev, PointerId::Mouse, InputEvent::PointerUp).is_empty());
    assert!(s.handle_pointer(&p, rev, PointerId::Touch(8), InputEvent::Cancel(CancelReason::PointerCancel)).is_empty());
    s.touch = false; // host's unrelated source flag cannot change a capture.
    s.handle_pointer(&p, rev, PointerId::Touch(7), motion(1240.0, 62.0, (105.0, 100.0)));
    assert_eq!(s.gesture_name(), "armed");
    s.handle_pointer(&p, rev, PointerId::Touch(7), motion(1240.0, 62.0, (109.0, 100.0)));
    assert_eq!(s.gesture_name(), "dragging");
    assert_eq!(s.handle_pointer(&p, rev, PointerId::Mouse, InputEvent::Cancel(CancelReason::FocusLost)), vec![Effect::ReleasePointer]);
}

#[test]
fn selection_modifiers_and_click_release_preserve_existing_semantics() {
    let (store, mut s) = setup();
    s.select_all(store.project());
    feed(&store, &mut s, down(Hit::Body("a".into()), Modifiers::default()));
    feed(&store, &mut s, InputEvent::PointerUp);
    assert_eq!(s.selection, ["a".into()].into());
    let toggle = Modifiers { secondary: true, ..Default::default() };
    assert!(feed(&store, &mut s, down(Hit::Body("a".into()), toggle)).is_empty());
    assert!(s.selection.is_empty() && s.gesture.is_none());
    assert_eq!(feed(&store, &mut s, down(Hit::Body("b".into()), toggle)), vec![Effect::CapturePointer]);
    feed(&store, &mut s, InputEvent::PointerUp);
    feed(&store, &mut s, down(Hit::Body("a".into()), Modifiers { shift: true, ..Default::default() }));
    assert_eq!(s.selection.len(), 2);
    feed(&store, &mut s, InputEvent::Cancel(CancelReason::Escape));
}

#[test]
fn native_terminal_sample_replaces_the_last_move_before_committing() {
    let (mut store, mut session) = setup();
    feed(&store, &mut session, down(Hit::Body("a".into()), Modifiers::default()));
    feed(&store, &mut session, motion(1240.0, 61.0, (200.0, 90.0)));
    let effects = feed(&store, &mut session, InputEvent::PointerUpAt {
        music: MusicPoint { tick: 1480.0, pitch: 63.0 }, screen: (300.0, 70.0), mods: Modifiers::default(),
    });
    assert_eq!(effects.iter().filter(|effect| matches!(effect, Effect::ReleasePointer)).count(), 1);
    assert_eq!(commit(&mut store, &mut session, effects), 1);
    let note = &store.project().tracks[0].notes[0];
    assert_eq!((note.start_tick, note.midi_note), (1440, 63));
    assert_eq!(store.undo_depth(), 1);
}

#[test]
fn coalesced_down_up_samples_can_cross_threshold_without_a_move_event() {
    let (mut store, mut session) = setup();
    feed(&store, &mut session, down(Hit::Body("a".into()), Modifiers::default()));
    let effects = feed(&store, &mut session, InputEvent::PointerUpAt {
        music: MusicPoint { tick: 1240.0, pitch: 62.0 }, screen: (200.0, 80.0), mods: Modifiers::default(),
    });
    assert_eq!(commit(&mut store, &mut session, effects), 1);
    assert_eq!(store.project().tracks[0].notes[0].start_tick, 1200);
    assert!(session.pointer_owner().is_none());
}

#[test]
fn terminal_sample_honors_latest_modifiers_and_rejects_foreign_or_invalid_input() {
    let (mut store, mut session) = setup();
    let project = store.snapshot();
    let revision = store.revision();
    session.handle_pointer(&project, revision, PointerId::Touch(7), down(Hit::Body("a".into()), Modifiers::default()));
    let terminal = InputEvent::PointerUpAt {
        music: MusicPoint { tick: 1480.0, pitch: 64.0 }, screen: (300.0, 70.0),
        mods: Modifiers { shift: true, ..Default::default() },
    };
    assert!(session.handle_pointer(&project, revision, PointerId::Mouse, terminal.clone()).is_empty());
    assert_eq!(session.pointer_owner(), Some(PointerId::Touch(7)));
    let effects = session.handle_pointer(&project, revision, PointerId::Touch(7), terminal);
    assert_eq!(commit(&mut store, &mut session, effects), 1);
    assert_eq!((store.project().tracks[0].notes[0].start_tick, store.project().tracks[0].notes[0].midi_note), (1440, 60));

    feed(&store, &mut session, down(Hit::Body("a".into()), Modifiers::default()));
    feed(&store, &mut session, motion(1700.0, 62.0, (200.0, 80.0)));
    let effects = feed(&store, &mut session, InputEvent::PointerUpAt {
        music: MusicPoint { tick: f64::NAN, pitch: 62.0 }, screen: (300.0, 70.0), mods: Modifiers::default(),
    });
    assert_eq!(effects, [Effect::ReleasePointer]);
    assert!(session.gesture.is_none() && session.pointer_owner().is_none());
    assert_eq!(store.undo_depth(), 1, "invalid terminal position must not commit a stale preview");
}

#[test]
fn project_change_returns_the_native_release_effect_before_any_late_up() {
    let (mut store, mut session) = setup();
    feed(&store, &mut session, down(Hit::Body("a".into()), Modifiers::default()));
    feed(&store, &mut session, motion(1240.0, 62.0, (200.0, 80.0)));
    store.apply(store.revision(), CommandBatch::single("other edit", Command::SetLyric {
        track: session.track.clone(), note_id: "b".into(), text: "坝".into(),
    })).unwrap();
    let effects = session.on_project_changed(store.project(), store.revision());
    assert_eq!(effects.iter().filter(|effect| matches!(effect, Effect::ReleasePointer)).count(), 1);
    assert!(session.pointer_owner().is_none());
    let up = feed(&store, &mut session, InputEvent::PointerUpAt {
        music: MusicPoint { tick: 1480.0, pitch: 62.0 }, screen: (300.0, 70.0), mods: Modifiers::default(),
    });
    assert!(up.is_empty());
    assert_eq!(store.undo_depth(), 1);
}
