//! Editor boundary gestures exercise the real allocation, transaction store
//! and score adapter. Capability declarations are fixtures, not voice models.
use serde_json::json;
use verdant_backend::phoneme_timing::{drag_boundary, note_budget};
use verdant_backend::reading::parse_phonetic_input;
use verdant_backend::{Capabilities, Route, plan};
use verdant_domain::{Note, Project, ProjectStore, Pronunciation, PronunciationSource, TempoMap};

fn capabilities() -> Capabilities {
    serde_json::from_value(json!({
        "protocol": "ogd-worker/1", "test_double": true, "routes":["verdant-voice"],
        "max_phrase_seconds": 30,
        "native_score": {
            "inventory": verdant_backend::pinyin::recovered_syllables(),
            "sub_syllable_timing": true, "phoneme_duration_control": true,
            "phoneme_frontend": {
                "schema_version": "verdant-phonemes/2", "alphabets": ["ipa", "xsampa"],
                "languages": {"en": "English"}
            }
        }
    })).unwrap()
}

fn store(note: Note) -> ProjectStore {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 120_000_000;
    project.tracks[0].notes = vec![note];
    project.tracks[0].refresh_index();
    ProjectStore::new(project)
}

fn scheduled_durations(project: &Project, caps: &Capabilities) -> Vec<f64> {
    let plan = plan(project, &project.tracks[0].id, Route::VerdantVoice, caps);
    assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
    assert_eq!(plan.phrases.len(), 1);
    plan.phrases[0].score.segments.iter().map(|segment| segment.duration_seconds).collect()
}

#[test]
fn boundary_drag_preserves_total_and_minimum_and_rejects_impossible_drag_atomically() {
    let mut note = Note::new(960, 960, 60);
    note.lyric = "妈".into();
    note.pronunciation = Pronunciation {
        pinyin: "ma1".into(), source: PronunciationSource::Manual, ..Default::default()
    };
    let mut store = store(note);
    let caps = capabilities();
    let original = store.snapshot();
    let track = original.tracks[0].id.clone();
    let id = original.tracks[0].notes[0].id.clone();
    let before_total: f64 = scheduled_durations(&original, &caps).iter().sum();
    let batch = drag_boundary(store.project(), &track, &id, &caps, 1, 0.025).unwrap();
    assert!(store.apply(store.revision(), batch).unwrap().is_some());
    let durations = scheduled_durations(store.project(), &caps);
    assert!((durations.iter().sum::<f64>() - before_total).abs() < 1e-12);
    assert!((durations[0] - 0.025).abs() < 1e-12);
    assert!(durations.iter().all(|&duration| duration >= 0.025 - 1e-12));
    assert_eq!(store.undo_depth(), 1, "one release must be one undo step");
    let edited = store.project().track(&track).unwrap().note(&id).unwrap();
    let initial = original.track(&track).unwrap().note(&id).unwrap();
    assert_eq!((edited.start_tick, edited.duration_tick, edited.midi_note, &edited.lyric),
        (initial.start_tick, initial.duration_tick, initial.midi_note, &initial.lyric));

    let snapshot = store.snapshot();
    let revision = store.revision();
    for position in [0.024, -1.0, before_total + 1.0, f64::NAN, f64::INFINITY] {
        assert!(drag_boundary(store.project(), &track, &id, &caps, 1, position).is_err());
        assert_eq!(store.project(), snapshot.as_ref());
        assert_eq!(store.revision(), revision);
        assert_eq!(store.undo_depth(), 1);
    }
    store.undo().unwrap();
    assert_eq!(store.project(), original.as_ref());
}

#[test]
fn explicit_boundary_drag_exchanges_only_its_adjacent_pair_in_the_rendered_score() {
    let (lyric, mut pronunciation) = parse_phonetic_input(r"red/[en/xsampa:r\ E d]").unwrap().unwrap();
    pronunciation.phoneme_durations_permille = Some(vec![200, 500, 300]);
    let mut note = Note::new(960, 960, 60);
    note.lyric = lyric;
    note.pronunciation = pronunciation;
    let mut store = store(note);
    let caps = capabilities();
    let track = store.project().tracks[0].id.clone();
    let id = store.project().tracks[0].notes[0].id.clone();
    let before = scheduled_durations(store.project(), &caps);
    assert_eq!(before.len(), 3);
    let total: f64 = before.iter().sum();
    let batch = drag_boundary(store.project(), &track, &id, &caps, 2, total - 0.025).unwrap();
    store.apply(store.revision(), batch).unwrap();
    let after = scheduled_durations(store.project(), &caps);
    assert_eq!(after[0], before[0]);
    assert!((after.iter().sum::<f64>() - total).abs() < 1e-12);
    assert!((after[2] - 0.025).abs() < 1e-12);
    assert!(after.iter().all(|&duration| duration >= 0.025 - 1e-12));
    let snapshot = store.snapshot();
    // This crosses the preceding boundary, even though it is inside the note.
    assert!(drag_boundary(store.project(), &track, &id, &caps, 2, before[0]).is_err());
    assert_eq!(store.project(), snapshot.as_ref());
}

#[test]
fn unavailable_duration_capability_leaves_the_resolved_panel_read_only() {
    let mut note = Note::new(960, 960, 60);
    note.pronunciation = Pronunciation {
        pinyin: "ma1".into(), source: PronunciationSource::Manual, ..Default::default()
    };
    let store = store(note);
    let mut caps = capabilities();
    caps.native_score.as_mut().unwrap().phoneme_duration_control = false;
    let project = store.project();
    let track = &project.tracks[0];
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    let budget = note_budget(&track.notes[0], &tempo, &caps, Route::VerdantVoice).unwrap();
    assert_eq!(budget.labels.len(), 2);
    assert!(!budget.editable);
    assert!(drag_boundary(project, &track.id, &track.notes[0].id, &caps, 1, 0.1).is_err());
    assert_eq!(store.undo_depth(), 0);
}
