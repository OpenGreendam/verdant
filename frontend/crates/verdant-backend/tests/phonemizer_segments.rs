//! Segmented phonemizer mapping: one track assigns different finite frontends
//! to disjoint tick ranges, without a model or worker process. The segments
//! persist through the ordinary extension command and project serialization,
//! stay opt-in (removing them restores the single selected frontend), and keep
//! located diagnostics for notes outside every segment.
use std::collections::BTreeMap;

use verdant_backend::phonemizer::{
    self, EXTENSION_KEY, JAPANESE_KANA, MANDARIN,
    PhonemizerSettings, ReadingOrigin, ResolvedTrack, SegmentEntry,
};
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectStore, Track};

/// One track with a hanzi section (0–1920), a kana section (1920–3840) and one
/// hanzi note past both sections. The section boundary sits exactly on a note
/// start so `[start, end)` half-open semantics are exercised.
fn segmented_track() -> Track {
    let mut track = Track::new("分段映射", "teal");
    for (index, lyric) in ["海", "妈", "か", "き", "海"].iter().enumerate() {
        let mut note = Note::new(index as i64 * 960, 960, 60);
        note.lyric = (*lyric).into();
        track.notes.push(note);
    }
    track.refresh_index();
    track
}

fn segment_settings(track: &Track) -> PhonemizerSettings {
    let mut settings = PhonemizerSettings::default();
    settings.selected = JAPANESE_KANA.into();
    settings.segments = BTreeMap::from([
        ("hanzi-opening".into(), SegmentEntry {
            track: track.id.clone(), start_tick: 0, end_tick: 1920,
            selected: MANDARIN.into(), unknown: Default::default(),
        }),
        ("kana-middle".into(), SegmentEntry {
            track: track.id.clone(), start_tick: 1920, end_tick: 3840,
            selected: JAPANESE_KANA.into(), unknown: Default::default(),
        }),
    ]);
    settings
}

fn save_settings(store: &mut ProjectStore, settings: &PhonemizerSettings) {
    store.apply(store.revision(), CommandBatch::single("读音器分段映射", Command::SetProjectExtension {
        key: EXTENSION_KEY.into(), value: Some(phonemizer::settings_to_extension(settings).unwrap()),
    })).unwrap();
}

fn resolve(project: &Project, index: usize) -> ResolvedTrack {
    let settings = phonemizer::settings_from_project(project).unwrap();
    phonemizer::resolve_track(&project.tracks[index], &settings)
}

#[test]
fn segmented_track_resolves_both_sections_and_removal_restores_the_single_frontend() {
    let mut project = Project::new_empty();
    project.tracks[0] = segmented_track();
    let track_id = project.tracks[0].id.clone();
    let notes = project.tracks[0].notes.clone();
    let mut store = ProjectStore::new(project);
    let settings = segment_settings(&store.project().tracks[0]);
    save_settings(&mut store, &settings);
    assert_eq!(store.undo_depth(), 1);
    // Resolve after a genuine project serialization, not a private draft map.
    let restored: Project = serde_json::from_str(&serde_json::to_string(store.project()).unwrap()).unwrap();
    assert_eq!(phonemizer::settings_from_project(&restored).unwrap(), settings);

    let segmented = resolve(&restored, 0);
    // Every covered note resolves; the only diagnostic is the uncovered tail.
    assert_eq!(segmented.diagnostics.len(), 1);
    assert_eq!(segmented.diagnostics[0].code, "E_PRONUNCIATION_UNSET");
    assert_eq!(segmented.diagnostics[0].note_id, Some(notes[4].id.clone()));
    for (index, pinyin) in [(0, "hai3"), (1, "ma1")] {
        let resolved = &segmented.notes[&notes[index].id];
        assert_eq!(resolved.pronunciation.pinyin, pinyin);
        assert_eq!(resolved.pronunciation.input_text.as_deref(), Some(notes[index].lyric.as_str()));
        assert_eq!(resolved.origin, ReadingOrigin::Builtin);
    }
    for (index, phonemes) in [(2, ["k", "a"]), (3, ["kʲ", "i"])] {
        let resolved = &segmented.notes[&notes[index].id];
        assert_eq!(resolved.pronunciation.language.as_deref(), Some("ja"));
        assert!(resolved.pronunciation.pinyin.is_empty());
        assert_eq!(resolved.pronunciation.phonemes, phonemes);
        assert_eq!(resolved.origin, ReadingOrigin::Builtin);
    }
    assert!(!segmented.notes.contains_key(&notes[4].id));

    // Removing every segment restores the previous single-frontend behaviour:
    // the hanzi notes error again under the kana default, the kana notes stay.
    let mut removed = settings.clone();
    removed.segments.clear();
    save_settings(&mut store, &removed);
    let single = resolve(store.project(), 0);
    assert!(!single.notes.contains_key(&notes[0].id));
    assert!(!single.notes.contains_key(&notes[1].id));
    assert_eq!(single.notes[&notes[2].id].pronunciation.phonemes, ["k", "a"]);
    assert_eq!(single.notes[&notes[3].id].pronunciation.phonemes, ["kʲ", "i"]);
    let errors: Vec<_> = single.diagnostics.iter().filter(|d| d.code == "E_PRONUNCIATION_UNSET").collect();
    assert_eq!(errors.len(), 3);
    assert_eq!(errors[0].note_id, Some(notes[0].id.clone()));
    assert_eq!(errors[1].note_id, Some(notes[1].id.clone()));
    assert_eq!(errors[2].note_id, Some(notes[4].id.clone()));
    assert!(errors.iter().all(|d| d.candidates.is_empty()));
    assert!(errors[0].detail.contains("海"));

    // One undo step brings the segmented mapping back unchanged.
    store.undo().unwrap();
    assert_eq!(resolve(store.project(), 0), segmented);
    assert_eq!(store.project().tracks[0].notes, notes);
    assert_eq!(store.project().tracks[0].id, track_id);
}

#[test]
fn note_outside_every_segment_keeps_its_located_diagnostic() {
    let mut project = Project::new_empty();
    project.tracks[0] = segmented_track();
    let notes = project.tracks[0].notes.clone();
    let mut settings = segment_settings(&project.tracks[0]);
    // Leave two hanzi notes outside the Mandarin segment. The kana default
    // cannot resolve those lyrics, but still resolves the kana section.
    settings.segments.get_mut("hanzi-opening").unwrap().end_tick = 960;
    project.extensions.insert(EXTENSION_KEY.into(), phonemizer::settings_to_extension(&settings).unwrap());
    let restored: Project = serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
    let result = resolve(&restored, 0);
    assert_eq!(result.notes.len(), 3);
    assert_eq!(result.notes[&notes[0].id].pronunciation.pinyin, "hai3");
    assert_eq!(result.notes[&notes[2].id].pronunciation.phonemes, ["k", "a"]);
    assert_eq!(result.notes[&notes[3].id].pronunciation.phonemes, ["kʲ", "i"]);
    // Unsupported uncovered lyrics retain located diagnostics: 妈 at the
    // exclusive segment end and 海 beyond both ranges.
    let uncovered = [(&notes[1], "妈"), (&notes[4], "海")];
    assert_eq!(result.diagnostics.len(), uncovered.len());
    for (diagnostic, (note, lyric)) in result.diagnostics.iter().zip(uncovered) {
        assert_eq!(diagnostic.code, "E_PRONUNCIATION_UNSET");
        assert_eq!(diagnostic.note_id, Some(note.id.clone()));
        assert!(diagnostic.detail.contains(lyric));
        assert!(diagnostic.candidates.is_empty());
        assert!(!result.notes.contains_key(&note.id));
    }
    assert_eq!(restored.tracks[0].notes, notes);
}
