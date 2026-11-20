//! Runs the domain/session layer of `spec/examples/contract-traces.json`
//! (SPEC §17 "纯 reducer" tier). Each case executed here is reported by ID;
//! GPUI-harness and platform IME evidence are separate tiers.

use serde_json::Value;
use verdant_domain::model::{Note, Project};
use verdant_domain::{Command, NoteId, ProjectStore, Revision, TrackId};
use verdant_session::{CancelReason, EditorSession, Effect, Hit, InputEvent, LyricOutcome, LyricSession, Modifiers, MusicPoint};

const TRACES: &str = include_str!("../../../../spec/examples/contract-traces.json");

fn case(id: &str) -> Value {
    let v: Value = serde_json::from_str(TRACES).unwrap();
    v["cases"].as_array().unwrap().iter().find(|c| c["id"] == id).cloned().unwrap()
}

fn store_from(initial: &Value) -> (ProjectStore, TrackId) {
    let mut p = Project::new_empty();
    p.revision = initial["revision"].as_u64().unwrap();
    let track = p.tracks[0].id.clone();
    for n in initial["notes"].as_array().unwrap() {
        let mut note =
            Note::new(n["start_tick"].as_i64().unwrap(), n["duration_tick"].as_i64().unwrap(), n["midi_note"].as_i64().unwrap() as i32);
        note.id = NoteId::from(n["id"].as_str().unwrap());
        p.tracks[0].notes.push(note);
    }
    (ProjectStore::new(p), track)
}

/// Feed trace events through session → store, returning committed commands.
fn run(
    store: &mut ProjectStore,
    session: &mut EditorSession,
    events: &[Value],
    observe: &mut dyn FnMut(&ProjectStore, &EditorSession),
) -> Vec<Command> {
    let mut commands = Vec::new();
    for e in events {
        let music = || MusicPoint { tick: e["music_tick"].as_f64().unwrap(), pitch: e["pitch"].as_f64().unwrap() };
        let screen = || (e["screen"][0].as_f64().unwrap(), e["screen"][1].as_f64().unwrap());
        let ev = match e["type"].as_str().unwrap() {
            "PointerDown" => InputEvent::PointerDown {
                hit: Hit::Body(e["note_id"].as_str().unwrap().into()),
                music: music(),
                screen: screen(),
                mods: Modifiers::default(),
                click_count: 1,
            },
            "PointerMove" => InputEvent::PointerMove { music: music(), screen: screen(), mods: Modifiers::default() },
            "PointerUp" => InputEvent::PointerUp,
            "Cancel" => InputEvent::Cancel(match e["reason"].as_str().unwrap() {
                "escape" => CancelReason::Escape,
                "focus_lost" => CancelReason::FocusLost,
                _ => CancelReason::PointerCancel,
            }),
            other => panic!("unsupported event {other}"),
        };
        let project = store.snapshot();
        for fx in session.handle(&project, store.revision(), ev) {
            if let Effect::Commit { base, batch, .. } = fx {
                let commit = store.apply(base, batch).unwrap();
                if let Some(c) = commit {
                    commands.extend(c.commands);
                }
            }
        }
        observe(store, session);
    }
    commands
}

#[test]
fn t01_group_move_one_transaction() {
    let c = case("T01");
    let (mut store, track) = store_from(&c["initial"]);
    let mut session = EditorSession::new(track.clone());
    for id in c["initial"]["selection"].as_array().unwrap() {
        session.selection.insert(id.as_str().unwrap().into());
    }
    let a = &c["assertions"];
    let during = a["revision_during_moves"].as_u64().unwrap();
    let mut revisions_during_moves = Vec::new();
    let events = c["events"].as_array().unwrap();
    let commands = run(&mut store, &mut session, events, &mut |s, sess| {
        if sess.gesture.is_some() {
            revisions_during_moves.push(s.revision().get());
        }
    });
    assert!(revisions_during_moves.iter().all(|&r| r == during), "preview must not bump revision");
    assert_eq!(store.revision().get(), a["final_revision"].as_u64().unwrap());
    assert_eq!(commands.len() as u64, a["domain_command_count"].as_u64().unwrap());
    let expected = &a["domain_command"];
    match &commands[0] {
        Command::MoveNotes { note_ids, delta_tick, delta_semitones, .. } => {
            assert_eq!(expected["name"], "MoveNotes");
            let ids: Vec<&str> = note_ids.iter().map(|i| i.as_str()).collect();
            assert_eq!(serde_json::json!(ids), expected["note_ids"]);
            assert_eq!(*delta_tick, expected["delta_tick"].as_i64().unwrap());
            assert_eq!(*delta_semitones as i64, expected["delta_semitones"].as_i64().unwrap());
        }
        other => panic!("unexpected {other:?}"),
    }
    let t = store.project().track(&track).unwrap();
    for n in a["final_notes"].as_array().unwrap() {
        let got = t.note(&n["id"].as_str().unwrap().into()).unwrap();
        assert_eq!(got.start_tick, n["start_tick"].as_i64().unwrap());
        assert_eq!(got.duration_tick, n["duration_tick"].as_i64().unwrap());
        assert_eq!(got.midi_note as i64, n["midi_note"].as_i64().unwrap());
    }
    assert_eq!(store.undo_depth() as u64, a["undo_depth"].as_u64().unwrap());
    assert_eq!(session.gesture_name(), a["gesture"].as_str().unwrap());
    assert_eq!(session.pointer_captured, a["pointer_captured"].as_bool().unwrap());
}

#[test]
fn t02_escape_rolls_back_preview() {
    let c = case("T02");
    let (mut store, track) = store_from(&c["initial"]);
    let digest = store.content_digest();
    let mut session = EditorSession::new(track);
    for id in c["initial"]["selection"].as_array().unwrap() {
        session.selection.insert(id.as_str().unwrap().into());
    }
    let commands = run(&mut store, &mut session, c["events"].as_array().unwrap(), &mut |_, _| {});
    let a = &c["assertions"];
    assert_eq!(store.revision().get(), a["final_revision"].as_u64().unwrap());
    assert_eq!(commands.len() as u64, a["domain_command_count"].as_u64().unwrap());
    assert!(a["notes_unchanged"].as_bool().unwrap() && store.content_digest() == digest);
    assert_eq!(session.gesture_name(), a["gesture"].as_str().unwrap());
    assert_eq!(session.pointer_captured, a["pointer_captured"].as_bool().unwrap());
    assert_eq!(session.autoscroll_active, a["autoscroll_active"].as_bool().unwrap());
}

#[test]
fn t02_focus_loss_variant() {
    // Same contract with reason=focus_lost / pointer_cancel.
    for reason in [CancelReason::FocusLost, CancelReason::PointerCancel] {
        let c = case("T02");
        let (store, track) = store_from(&c["initial"]);
        let mut session = EditorSession::new(track);
        session.selection.insert("n1".into());
        let p = store.snapshot();
        let r = store.revision();
        session.handle(
            &p,
            r,
            InputEvent::PointerDown {
                hit: Hit::Body("n1".into()),
                music: MusicPoint { tick: 1000.0, pitch: 60.0 },
                screen: (100.0, 100.0),
                mods: Modifiers::default(),
                click_count: 1,
            },
        );
        session.handle(
            &p,
            r,
            InputEvent::PointerMove { music: MusicPoint { tick: 1240.0, pitch: 62.0 }, screen: (220.0, 60.0), mods: Modifiers::default() },
        );
        let fx = session.handle(&p, r, InputEvent::Cancel(reason));
        assert!(fx.iter().all(|f| !matches!(f, Effect::Commit { .. })));
        assert!(fx.contains(&Effect::ReleasePointer));
        assert!(!session.pointer_captured && session.gesture.is_none());
    }
}

#[test]
fn t03_duplicate_drag_cancel_leaves_nothing() {
    let (mut store, track) = store_from(&case("T01")["initial"]);
    let mut session = EditorSession::new(track.clone());
    session.selection.insert("n1".into());
    let p = store.snapshot();
    let r = store.revision();
    let alt = Modifiers { alt: true, ..Default::default() };
    session.handle(
        &p,
        r,
        InputEvent::PointerDown {
            hit: Hit::Body("n1".into()),
            music: MusicPoint { tick: 1000.0, pitch: 60.0 },
            screen: (100.0, 100.0),
            mods: alt,
            click_count: 1,
        },
    );
    session.handle(&p, r, InputEvent::PointerMove { music: MusicPoint { tick: 2000.0, pitch: 60.0 }, screen: (300.0, 100.0), mods: alt });
    let fx = session.handle(&p, r, InputEvent::Cancel(CancelReason::Escape));
    assert!(fx.iter().all(|f| !matches!(f, Effect::Commit { .. })));
    assert_eq!(store.project().note_count(), 2);
    assert_eq!(store.redo_depth(), 0);
    // And the successful variant commits exactly one DuplicateNotes.
    session.handle(
        &p,
        r,
        InputEvent::PointerDown {
            hit: Hit::Body("n1".into()),
            music: MusicPoint { tick: 1000.0, pitch: 60.0 },
            screen: (100.0, 100.0),
            mods: alt,
            click_count: 1,
        },
    );
    session.handle(&p, r, InputEvent::PointerMove { music: MusicPoint { tick: 2000.0, pitch: 60.0 }, screen: (300.0, 100.0), mods: alt });
    for fx in session.handle(&p, r, InputEvent::PointerUp) {
        if let Effect::Commit { base, batch, .. } = fx {
            store.apply(base, batch).unwrap();
        }
    }
    assert_eq!(store.project().note_count(), 3);
    assert_eq!(store.undo_depth(), 1);
}

#[test]
fn t04_resize_past_head_clamps_without_flip() {
    let (mut store, track) = store_from(&case("T01")["initial"]);
    let mut session = EditorSession::new(track.clone());
    session.selection.insert("n1".into());
    let p = store.snapshot();
    let r = store.revision();
    session.handle(
        &p,
        r,
        InputEvent::PointerDown {
            hit: Hit::Handle("n1".into(), verdant_domain::Edge::End),
            music: MusicPoint { tick: 1440.0, pitch: 60.0 },
            screen: (200.0, 100.0),
            mods: Modifiers::default(),
            click_count: 1,
        },
    );
    session.handle(
        &p,
        r,
        InputEvent::PointerMove { music: MusicPoint { tick: 0.0, pitch: 60.0 }, screen: (0.0, 100.0), mods: Modifiers::default() },
    );
    let preview = session.preview_note(p.track(&track).unwrap().note(&"n1".into()).unwrap()).unwrap();
    assert!(preview.duration_tick >= 1 && preview.start_tick == 960);
    for fx in session.handle(&p, r, InputEvent::PointerUp) {
        if let Effect::Commit { base, batch, .. } = fx {
            store.apply(base, batch).unwrap();
        }
    }
    let n = store.project().track(&track).unwrap().note(&"n1".into()).unwrap().clone();
    assert_eq!((n.start_tick, n.duration_tick), (960, 1));
}

#[test]
fn t05_ime_marked_text_is_not_a_project_edit() {
    let c = case("T05");
    let mut p = Project::new_empty();
    p.revision = c["initial"]["revision"].as_u64().unwrap();
    let track = p.tracks[0].id.clone();
    let mut n = Note::new(0, 480, 60);
    n.id = "n1".into();
    p.tracks[0].notes.push(n);
    let mut store = ProjectStore::new(p);
    let undo_before = store.undo_depth();
    let mut lyric = LyricSession::new(track.clone(), "n1".into(), "");
    let mut transport_commands = 0;
    let mut domain_commands = Vec::new();
    for e in c["events"].as_array().unwrap() {
        match e["type"].as_str().unwrap() {
            "ImeBegin" => {}
            "ImeReplaceAndMark" => {
                let sel = e["selection_utf16"].as_array().unwrap();
                lyric.state.replace_and_mark_text_in_range(
                    None,
                    e["text"].as_str().unwrap(),
                    Some(sel[0].as_u64().unwrap() as usize..sel[1].as_u64().unwrap() as usize),
                );
                assert_eq!(store.revision().get(), 7, "marked text must not commit");
            }
            "Key" => {
                // Routing rule: while composing, the IME owns Space/Enter/Escape.
                let ime_owns = lyric.state.is_composing() && e["ime_consumes"].as_bool().unwrap();
                if !ime_owns && e["key"] == "Space" {
                    transport_commands += 1;
                }
            }
            "ImeCommit" => {
                if let LyricOutcome::Commit(text) = lyric.on_ime_commit(None, e["text"].as_str().unwrap()) {
                    let cmd = Command::SetLyric { track: track.clone(), note_id: "n1".into(), text };
                    let commit = store.apply(store.revision(), verdant_domain::CommandBatch::single("歌词", cmd)).unwrap().unwrap();
                    domain_commands.extend(commit.commands);
                }
            }
            other => panic!("unsupported {other}"),
        }
    }
    let a = &c["assertions"];
    assert_eq!(store.revision().get(), a["final_revision"].as_u64().unwrap());
    assert_eq!(transport_commands, a["transport_command_count"].as_u64().unwrap());
    assert_eq!((store.undo_depth() - undo_before) as u64, a["undo_depth_delta"].as_u64().unwrap());
    assert_eq!(domain_commands.len(), 1);
    assert_eq!(serde_json::to_value(&domain_commands[0]).unwrap()["text"], a["domain_commands"][0]["text"]);
    assert!(a["marked_range"].is_null() && lyric.state.marked.is_none());
}

#[test]
fn t07_zoom_keeps_anchor_tick() {
    let mut s = EditorSession::new(TrackId::from("t"));
    let before = s.viewport;
    for (x, f) in [(333.0, 1.25), (12.0, 0.5), (900.0, 3.0), (640.0, 0.9)] {
        let anchor = s.viewport.x_to_tick(x);
        s.viewport.zoom_x_at(x, f);
        assert!((s.viewport.x_to_tick(x) - anchor).abs() < 1e-6, "anchor drift");
    }
    let y = 200.0;
    let anchor = s.viewport.y_to_pitch(y);
    s.viewport.zoom_y_at(y, 1.5);
    assert!((s.viewport.y_to_pitch(y) - anchor).abs() < 1e-9);
    assert_ne!(before, s.viewport);
}

#[test]
fn t11_keyboard_reaches_offscreen_notes() {
    let (store, track) = store_from(&case("T01")["initial"]);
    let mut s = EditorSession::new(track);
    s.viewport.start_tick = 50_000.0; // both notes far off-screen
    let p = store.snapshot();
    let id = s.step_selection(&p, true).unwrap();
    assert_eq!(id.as_str(), "n1");
    let r = s.viewport.note_rect(p.tracks[0].note(&id).unwrap());
    assert!(r.x >= 0.0 && r.right() <= s.viewport.width);
    assert_eq!(s.step_selection(&p, true).unwrap().as_str(), "n2");
}

#[test]
fn stale_release_after_concurrent_edit_does_not_commit() {
    let (mut store, track) = store_from(&case("T01")["initial"]);
    let mut session = EditorSession::new(track.clone());
    session.selection.insert("n1".into());
    let p = store.snapshot();
    let r = store.revision();
    session.handle(
        &p,
        r,
        InputEvent::PointerDown {
            hit: Hit::Body("n1".into()),
            music: MusicPoint { tick: 1000.0, pitch: 60.0 },
            screen: (100.0, 100.0),
            mods: Modifiers::default(),
            click_count: 1,
        },
    );
    session.handle(
        &p,
        r,
        InputEvent::PointerMove { music: MusicPoint { tick: 1500.0, pitch: 60.0 }, screen: (200.0, 100.0), mods: Modifiers::default() },
    );
    // Another transaction lands mid-drag.
    store
        .apply(r, verdant_domain::CommandBatch::single("x", Command::SetLyric { track, note_id: "n2".into(), text: "坝".into() }))
        .unwrap();
    let fx = session.on_project_changed(&store.snapshot(), store.revision());
    assert!(fx.iter().any(|f| matches!(f, Effect::Notice(_))));
    assert!(session.gesture.is_none() && !session.pointer_captured);
    assert_eq!(store.revision(), Revision::new(8).unwrap());
}

#[test]
fn rubber_band_selects_intersecting_notes() {
    let (store, track) = store_from(&case("T01")["initial"]);
    let mut s = EditorSession::new(track);
    let p = store.snapshot();
    let r = store.revision();
    s.handle(
        &p,
        r,
        InputEvent::PointerDown {
            hit: Hit::Background,
            music: MusicPoint { tick: 1300.0, pitch: 70.0 },
            screen: (0.0, 0.0),
            mods: Modifiers::default(),
            click_count: 1,
        },
    );
    s.handle(
        &p,
        r,
        InputEvent::PointerMove { music: MusicPoint { tick: 1500.0, pitch: 63.5 }, screen: (50.0, 50.0), mods: Modifiers::default() },
    );
    s.handle(&p, r, InputEvent::PointerUp);
    assert_eq!(s.selection.iter().map(|i| i.as_str()).collect::<Vec<_>>(), vec!["n2"]);
}

#[test]
fn touch_needs_more_travel_and_gets_bigger_handles() {
    let (store, track) = store_from(&case("T01")["initial"]);
    let p = store.snapshot();
    let r = store.revision();
    for (touch, expect_drag) in [(false, true), (true, false)] {
        let mut s = EditorSession::new(track.clone());
        s.touch = touch;
        s.selection.insert("n1".into());
        s.handle(
            &p,
            r,
            InputEvent::PointerDown {
                hit: Hit::Body("n1".into()),
                music: MusicPoint { tick: 1000.0, pitch: 60.0 },
                screen: (100.0, 100.0),
                mods: Modifiers::default(),
                click_count: 1,
            },
        );
        s.handle(
            &p,
            r,
            InputEvent::PointerMove { music: MusicPoint { tick: 1010.0, pitch: 60.0 }, screen: (105.0, 100.0), mods: Modifiers::default() },
        );
        assert_eq!(s.gesture_name() == "dragging", expect_drag, "touch={touch}");
    }
    // A finger 10 px right of a selected note's tail still grabs the handle.
    let s = EditorSession::new(track.clone());
    let mut sel = std::collections::BTreeSet::new();
    sel.insert(NoteId::from("n1"));
    let vp = s.viewport;
    let n = p.tracks[0].note(&"n1".into()).unwrap();
    let rr = vp.note_rect(n);
    let (x, y) = (rr.right() + 6.0, rr.y + rr.h / 2.0);
    assert_eq!(verdant_session::hit_test(&p.tracks[0], &vp, &sel, x, y), Hit::Background);
    assert!(matches!(verdant_session::hit_test_touch(&p.tracks[0], &vp, &sel, x, y), Hit::Handle(_, verdant_domain::Edge::End)));
}

#[test]
fn imported_svp_resolution_keeps_musical_framing_and_zoom_anchor() {
    // The same sustained note and 32-quarter arrangement, as native and SVP scores.
    let scene = |ppq: i64| {
        let mut session = EditorSession::new(TrackId::from("imported"));
        session.viewport.set_quarter_ticks(ppq);
        let note = Note::new(3 * ppq, 5 * ppq, 60);
        session.viewport.fit_ticks(3.0 * ppq as f64, 35.0 * ppq as f64, 40.0, 80.0);
        (session, note)
    };
    let (mut native, native_note) = scene(960);
    let (mut svp, svp_note) = scene(705_600_000);
    let same_note_on_screen = |a: &EditorSession, b: &EditorSession| {
        let a = a.viewport.note_rect(&native_note);
        let b = b.viewport.note_rect(&svp_note);
        assert!((a.x - b.x).abs() < 1e-6 && (a.w - b.w).abs() < 1e-6);
        assert_eq!((a.y, a.h), (b.y, b.h));
    };
    same_note_on_screen(&native, &svp);
    let anchor_x = 540.0;
    let musical_anchor = native.viewport.x_to_tick(anchor_x) / 960.0;
    for factor in [2.25, 0.8] {
        native.viewport.zoom_x_at(anchor_x, factor);
        svp.viewport.zoom_x_at(anchor_x, factor);
        same_note_on_screen(&native, &svp);
        assert!((native.viewport.x_to_tick(anchor_x) / 960.0 - musical_anchor).abs() < 1e-9);
        assert!((svp.viewport.x_to_tick(anchor_x) / 705_600_000.0 - musical_anchor).abs() < 1e-9);
    }
    // Zoom limits must also describe the same music rather than raw tick counts.
    for factor in [1e-12, 1e12] {
        native.viewport.zoom_x_at(anchor_x, factor);
        svp.viewport.zoom_x_at(anchor_x, factor);
        same_note_on_screen(&native, &svp);
    }
}
