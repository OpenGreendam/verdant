//! Editable built-in examples. Each explicit selection creates a new working
//! document; existing projects, recovery data and local audition files are read
//! through their ordinary paths. No voicebank or rendered audio is bundled here.

use std::path::{Path, PathBuf};
use verdant_domain::ProjectPersistence;
use verdant_domain::model::Project;
#[cfg(test)]
use verdant_domain::{Pronunciation, PronunciationSource, Vibrato};
#[cfg(test)]
use verdant_domain::model::{Note, Track};
#[cfg(test)]
use crate::editor::{self, PINYIN_PROFILE};

pub const EXAMPLE_EXTENSION: &str = "verdant_builtin_example";
pub const HEXIE_EXAMPLE_ID: &str = "hexie-title-excerpt";
/// Startup opens the 河蟹你全家 excerpt.
pub const DEFAULT_EXAMPLE_ID: &str = HEXIE_EXAMPLE_ID;

#[derive(Clone, Copy)]
pub struct BuiltinExample {
    pub id: &'static str,
    pub title: &'static str,
    pub attribution: &'static str,
    pub source_url: Option<&'static str>,
    pub reference_url: Option<&'static str>,
    pub material_scope: &'static str,
    build: fn() -> Result<Project, String>,
}

pub const EXAMPLES: &[BuiltinExample] = &[
    BuiltinExample {
        id: HEXIE_EXAMPLE_ID,
        title: "河蟹你全家 · 短句节选",
        attribution: "原曲《ルカルカ★ナイトフィーバー》/ samfree（SAM）；中文填词：荼荼丸",
        source_url: Some("https://www.bilibili.com/video/BV1xx411c7BF/"),
        reference_url: Some("https://elielin.hatenablog.com/entry/20090624/p1"),
        material_scope: "原版短句的近似音高与节拍整理，非官方谱；仅含8字短节选，不含原录音/伴奏/全曲谱。原作品权利保留，软件代码许可不扩及引用歌曲。",
        build: || verdant_domain::io::parse_project(include_str!("../assets/examples/hexie-title-excerpt.verp"))
            .map(|(project, _)| project).map_err(|error| error.to_string()),
    },
];

pub fn find(id: &str) -> Option<&'static BuiltinExample> {
    EXAMPLES.iter().find(|example| example.id == id)
}

fn fresh_copy(mut project: Project, example: &BuiltinExample) -> Project {
    // Keep native track/note IDs and all authored fields so internal references,
    // explicit phones, curves and future extensions retain their relationships.
    // The new document identity isolates saves, history and rendering caches.
    project.project_id = Project::new_empty().project_id;
    project.revision = 0;
    crate::files::stamp_creator(&mut project);
    let credit = project.extensions.entry(EXAMPLE_EXTENSION).or_insert_with(|| serde_json::json!({}));
    if !credit.is_object() { *credit = serde_json::json!({}); }
    credit.as_object_mut().unwrap().extend(serde_json::json!({
        "schema": "verdant-builtin-example/1", "id": example.id, "title": example.title,
        "attribution": example.attribution, "source_url": example.source_url,
        "reference_url": example.reference_url, "material_scope": example.material_scope,
    }).as_object().unwrap().clone());
    for track in &mut project.tracks { track.refresh_index(); }
    project
}

pub fn project(id: &str) -> Result<Project, String> {
    let example = find(id).ok_or_else(|| format!("未知内建示例：{id}"))?;
    Ok(fresh_copy((example.build)()?, example))
}

pub fn credit_text(project: &Project) -> Option<String> {
    let credit = project.extensions.get(EXAMPLE_EXTENSION)?;
    if credit["schema"] != "verdant-builtin-example/1" { return None; }
    Some(["title", "attribution", "material_scope", "source_url", "reference_url"].into_iter()
        .filter_map(|key| credit[key].as_str()).collect::<Vec<_>>().join("\n\n"))
}

pub fn default_project() -> Project {
    project(DEFAULT_EXAMPLE_ID).expect("the default built-in example is registered")
}

pub fn create_working_copy(id: &str, documents: &Path) -> Result<ProjectPersistence, String> {
    let project = project(id)?;
    let path = documents.join(format!("example-{id}-{}.verp", project.project_id));
    ProjectPersistence::create(&path, project).map_err(|error| error.to_string())
}

/// A recovered example working copy still needs a user-chosen Save As path.
/// Credit alone does not make a saved document untitled: the registered example,
/// project identity and actual working-directory path must all agree.
pub fn save_destination(project: &Project, proposed: Option<&Path>, documents: &Path) -> Option<PathBuf> {
    let path = proposed?;
    let example = project.extensions.get(EXAMPLE_EXTENSION)
        .filter(|credit| credit["schema"] == "verdant-builtin-example/1")
        .and_then(|credit| credit["id"].as_str()).and_then(find);
    let internal = example.is_some_and(|example| {
        // Generated document IDs are UUIDs. Never let an imported identity
        // introduce path separators while recognizing an internal filename.
        if project.project_id.is_empty() || !project.project_id.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')) {
            return false;
        }
        // Existing .json working copies and their journals stay recoverable;
        // every newly created copy uses .verp.
        ["verp", "json"].iter().any(|extension| {
            let expected = documents.join(format!("example-{}-{}.{extension}", example.id, project.project_id));
            path == expected.as_path() || match (path.canonicalize(), expected.canonicalize()) {
                (Ok(actual), Ok(expected)) => actual == expected,
                _ => false,
            }
        })
    });
    if internal { None } else { Some(path.to_path_buf()) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupSource { Existing, Audition, Builtin }

/// Preserve the startup precedence, including a missing explicit project path:
/// it starts the built-in exercise rather than importing a local audition.
pub fn startup_source(working_exists: bool, explicit_project: bool, has_auditions: bool) -> StartupSource {
    if working_exists { StartupSource::Existing }
    else if !explicit_project && has_auditions { StartupSource::Audition }
    else { StartupSource::Builtin }
}

/// Test-only two-track fixture for pronunciation-boundary invariants. It is not
#[cfg(test)]
/// shipped as a built-in example.
fn diagnostic_project() -> Project {
    fn sung(start: i64, dur: i64, midi: i32, lyric: &str, py: &str) -> Note {
        let mut n = Note::new(start, dur, midi);
        n.lyric = lyric.into();
        if !py.is_empty() {
            n.pronunciation = Pronunciation {
                pinyin: py.into(),
                source: PronunciationSource::Manual,
                frontend_profile: PINYIN_PROFILE.into(),
                ..Default::default()
            };
        }
        n
    }
    // A pronunciation-boundary fixture: it predates vowel-onset alignment and
    // is checked against capabilities that do not declare it.
    let mut p = Project::new_empty();
    let lead = &mut p.tracks[0];
    lead.name = "读音边界夹具".into();
    let melody: &[(i64, i64, i32, &str, &str)] = &[
        (0, 480, 64, "上", "shang4"),
        (480, 480, 67, "海", "hai3"),
        (960, 960, 69, "好", "hao3"),
        (1920, 480, 67, "上", "shang4"),
        (2400, 480, 64, "海", "hai3"),
        (2880, 960, 62, "好", "hao3"),
        (4800, 480, 60, "上", "shang4"),
        (5280, 480, 62, "海", "hai3"),
        (5760, 480, 64, "好", "hao3"),
        (6240, 1440, 67, "上", "shang4"),
        (8160, 480, 69, "好", "hao3"),
    ];
    for &(start, dur, midi, lyric, py) in melody {
        lead.notes.push(sung(start, dur, midi, lyric, py));
    }
    // Expressive defaults on the lead use the selected route's authored controls.
    lead.notes[2].vibrato = Some(Vibrato::DEFAULT);
    lead.notes[2].portamento_ms = Some(80);
    lead.notes[9].vibrato = Some(Vibrato { depth_cents: 45, rate_millihertz: 5800, delay_permille: 300, unknown: Default::default() });
    let mut harmony = Track::new("和声", "blue");
    harmony.gain_millidecibels = -6000;
    harmony.pan_permille = 300;
    // A pronunciation demo must not layer different syllables over the lead.
    // Make independent notes; only lyric timing is shared, not expressive edits.
    for (lead_note, midi) in lead.notes.iter().take(6).zip([60, 64, 64, 59, 60, 59]) {
        harmony.notes.push(sung(lead_note.start_tick, lead_note.duration_tick, midi,
            &lead_note.lyric, &lead_note.pronunciation.pinyin));
    }
    p.tracks.push(harmony);
    for track in &mut p.tracks {
        track.refresh_index();
    }
    p
}

#[cfg(test)]
mod demo_tests {
    use super::*;

    #[test]
    fn demo_harmony_has_matching_syllable_and_phone_boundaries_with_independent_notes() {
        let project = diagnostic_project();
        let (lead, harmony) = (&project.tracks[0], &project.tracks[1]);
        assert_eq!(harmony.notes.len(), 6);
        assert_eq!(harmony.gain_millidecibels, -6000);
        assert_eq!(harmony.pan_permille, 300);
        assert!(!harmony.muted);
        for ((a, b), midi) in lead.notes.iter().zip(&harmony.notes).zip([60, 64, 64, 59, 60, 59]) {
            assert_eq!((&a.lyric, &a.pronunciation.pinyin, a.start_tick, a.end_tick()),
                (&b.lyric, &b.pronunciation.pinyin, b.start_tick, b.end_tick()));
            assert_ne!(a.id, b.id);
            assert_eq!(b.midi_note, midi);
            assert!(b.vibrato.is_none() && b.portamento_ms.is_none());
        }
        let caps: verdant_backend::wire::Capabilities = serde_json::from_value(serde_json::json!({
            "engine":"synthetic demo-test capabilities", "protocol":"ogd-worker/1", "native_version":"fixture", "test_double":true,
            "routes":["verdant-voice"], "max_active_jobs":1,"max_phrase_seconds":30.0,
            "native_score":{"inventory":verdant_backend::pinyin::recovered_syllables(),
                "english_vowel_proxies":["a","i","u","e","o"],"continuous_f0_control":true,
                "coarticulation_recovered":false,"sub_syllable_timing":true,"phoneme_duration_control":true}
        })).unwrap();
        let a = verdant_backend::plan(&project, &lead.id, verdant_backend::Route::VerdantVoice, &caps);
        let b = verdant_backend::plan(&project, &harmony.id, verdant_backend::Route::VerdantVoice, &caps);
        assert!(!a.has_errors() && !b.has_errors());
        assert_eq!(b.phrases.len(), 1);
        assert_eq!(b.phrases[0].score.segments.len(), 12);
        let (mut a_seconds, mut b_seconds) = (a.phrases[0].start_seconds, b.phrases[0].start_seconds);
        for (a, b) in a.phrases[0].score.segments.iter().zip(&b.phrases[0].score.segments) {
            assert_eq!(a_seconds, b_seconds);
            assert_eq!(a.context, b.context);
            assert_eq!(a.voiced, b.voiced);
            a_seconds += a.duration_seconds;
            b_seconds += b.duration_seconds;
            assert_eq!(a_seconds, b_seconds);
            // The lead continues after a rest; the harmony ends here. Compare
            // phone endpoints, not an unrelated following phrase gap.
            a_seconds += a.gap_after_seconds;
            b_seconds += b.gap_after_seconds;
        }
    }
}


#[cfg(test)]
mod foundation_tests {
    use super::*;
    use serde_json::json;
    use verdant_backend::{Capabilities, Route};
    use verdant_domain::{Command, CommandBatch, TempoMap};
    use verdant_domain::model::{ExpressionPoint, GainPoint, PitchPoint};

    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("verdant-examples-{}", Project::new_empty().project_id));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    fn capabilities() -> Capabilities {
        serde_json::from_value(json!({"protocol":"ogd-worker/1", "test_double":true,
            "routes":["verdant-voice"], "max_phrase_seconds":30,
            "native_score":{"inventory":verdant_backend::pinyin::recovered_syllables(),
                "sub_syllable_timing":true,"phoneme_duration_control":true,"continuous_f0_control":true,
                "explicit_f0_hz_range":[35,2000],
                "expression":{"methods":["vocaloid-controls-v1"],"parameters":["BRE","BRI","CLE","GEN","OPE","GWL"],
                    "routes":["verdant-voice"],"maximum_compiled_curve_points":32768}}})).unwrap()
    }

    #[test]
    fn startup_keeps_saved_explicit_and_audition_precedence() {
        for explicit in [false, true] {
            for auditions in [false, true] {
                assert_eq!(startup_source(true, explicit, auditions), StartupSource::Existing);
                let expected = if !explicit && auditions { StartupSource::Audition } else { StartupSource::Builtin };
                assert_eq!(startup_source(false, explicit, auditions), expected);
            }
        }
        // Reopening a saved example is an ordinary document, never a newly
        // generated demo eligible for an automatic voice-default adjustment.
        assert_eq!(startup_source(true, false, false), StartupSource::Existing);
    }

    #[test]
    fn internal_example_reopen_keeps_save_as_until_external_save() {
        let temp = Temp::new();
        let documents = temp.0.join("documents");
        std::fs::create_dir_all(&documents).unwrap();
        let mut first = create_working_copy(HEXIE_EXAMPLE_ID, &documents).unwrap();
        let working = first.project_path().to_path_buf();
        assert_eq!(save_destination(first.project(), Some(&working), &documents), None);

        let original = first.project().tracks[0].notes[0].clone();
        let mut edited = original.clone();
        edited.midi_note += 1;
        first.commit(first.revision(), CommandBatch { label:"编辑后切换示例".into(), commands:vec![
            Command::ReplaceNotes { track:first.project().tracks[0].id.clone(), notes:vec![edited.clone()] },
        ]}).unwrap();
        first.autosave().unwrap();
        drop(first);

        // Recent/recovery opening and an explicit startup path use the same
        // destination decision as initial selection, after journal replay.
        let (mut reopened, report) = ProjectPersistence::open(&working).unwrap();
        assert!(report.recovered_anything());
        assert!(reopened.accept_recovery());
        assert_eq!(reopened.project().tracks[0].notes[0], edited);
        assert_eq!(save_destination(reopened.project(), Some(&working), &documents), None);
        let alias = documents.join(".").join(working.file_name().unwrap());
        assert_eq!(save_destination(reopened.project(), Some(&alias), &documents), None);
        let credit = reopened.project().extensions[EXAMPLE_EXTENSION].clone();

        // Even the identical basename and retained example metadata must not
        // turn the user's external Save As destination back into an untitled.
        let external_dir = temp.0.join("user-chosen");
        std::fs::create_dir_all(&external_dir).unwrap();
        let external = external_dir.join(working.file_name().unwrap());
        reopened.save_as(&external).unwrap();
        assert_eq!(save_destination(reopened.project(), Some(&external), &documents), Some(external.clone()));
        drop(reopened);
        let (mut saved, report) = ProjectPersistence::open(&external).unwrap();
        assert!(!report.recovered_anything());
        assert_eq!(save_destination(saved.project(), Some(&external), &documents), Some(external));
        assert_eq!(saved.project().extensions[EXAMPLE_EXTENSION], credit);
        assert_eq!(saved.project().tracks[0].notes[0], edited);
        saved.undo().unwrap();
        assert_eq!(saved.project().tracks[0].notes[0], original);
    }

    #[test]
    fn save_destination_requires_both_working_path_and_registered_identity() {
        let temp = Temp::new();
        let documents = temp.0.join("documents");
        std::fs::create_dir_all(&documents).unwrap();
        let working = create_working_copy(HEXIE_EXAMPLE_ID, &documents).unwrap();
        let path = working.project_path();
        let normal = documents.join("my-song.verp");
        assert_eq!(save_destination(working.project(), Some(&normal), &documents), Some(normal));
        let mut ordinary = working.project().clone();
        ordinary.extensions.remove(EXAMPLE_EXTENSION);
        assert_eq!(save_destination(&ordinary, Some(path), &documents), Some(path.to_path_buf()));
        let mut changed_identity = working.project().clone();
        changed_identity.project_id.push_str("-different");
        assert_eq!(save_destination(&changed_identity, Some(path), &documents), Some(path.to_path_buf()));
        changed_identity.project_id = "../external".into();
        assert_eq!(save_destination(&changed_identity, Some(path), &documents), Some(path.to_path_buf()));
        assert_eq!(save_destination(&ordinary, None, &documents), None);
    }

    #[test]
    fn native_short_excerpt_retains_the_provided_score_and_credit() {
        let project = project(HEXIE_EXAMPLE_ID).unwrap();
        assert_eq!(project.ppq, 480);
        assert_eq!(project.tempo_map[0].micro_bpm, 160_000_000);
        assert_eq!((project.time_signatures[0].numerator, project.time_signatures[0].denominator), (4, 4));
        assert_eq!(project.render_preferences.route, "verdant-voice");
        assert_eq!(project.tracks.len(), 1);
        let notes = &project.tracks[0].notes;
        let expected = [
            (0,480,65,"绿","lv4"), (480,480,72,"坝","ba4"),
            (960,480,65,"绿","lv4"), (1440,480,72,"坝","ba4"),
            (1920,240,67,"把","ba3"), (2160,240,68,"你","ni3"),
            (2400,480,70,"萌","meng2"), (2880,480,68,"杀","sha1"),
        ];
        assert_eq!(notes.len(), expected.len());
        for (note, (start, duration, midi, lyric, pinyin)) in notes.iter().zip(expected) {
            assert_eq!((note.start_tick, note.duration_tick, note.midi_note), (start, duration, midi));
            assert_eq!((&*note.lyric, &*note.pronunciation.pinyin), (lyric, pinyin));
            assert_eq!(note.pronunciation.source, PronunciationSource::Manual);
        }
        let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
        assert_eq!(tempo.seconds_at(notes.last().unwrap().end_tick() as f64), 2.625);
        let credit = credit_text(&project).unwrap();
        for text in ["samfree", "荼荼丸", "非官方谱", "仅含8字短节选", "软件代码许可不扩及引用歌曲", "BV1xx411c7BF", "20090624/p1"] {
            assert!(credit.contains(text), "missing credit: {text}");
        }
        assert_eq!(project.extensions[EXAMPLE_EXTENSION]["transcription_source_span_seconds"], json!([49.9,52.65]));
    }

    #[test]
    fn example_menu_uses_builtin_actions_without_displacing_local_projects() {
        use crate::editor::MenuAction;
        use crate::menu::{MenuCommand, audition_menu_items, file_menu_items};
        let local = crate::auditions::AuditionProject { title:EXAMPLES[0].title.into(), path:"/local/edited.json".into() };
        let items = audition_menu_items(&[local.clone()]);
        for example in EXAMPLES {
            assert_eq!(items.iter().filter(|item| matches!(&item.command,
                Some(MenuCommand::Editor(MenuAction::OpenBuiltinExample(id))) if id == example.id)).count(), 1);
        }
        assert!(items.iter().any(|item| matches!(&item.command,
            Some(MenuCommand::Editor(MenuAction::OpenAudition(path))) if path == &local.path)));
        assert!(items.iter().any(|item| matches!(&item.command,
            Some(MenuCommand::Editor(MenuAction::ExampleCredits)))));
        assert!(file_menu_items(&crate::files::Prefs::default()).iter().any(|item| matches!(&item.command,
            Some(MenuCommand::Editor(MenuAction::ChooseExample)))));
        assert!(project("../unregistered-score").is_err());
    }

    #[test]
    fn working_copy_preserves_full_phone_edits_expression_history_and_saved_route() {
        let temp = Temp::new();
        let mut first = create_working_copy(HEXIE_EXAMPLE_ID, &temp.0).unwrap();
        let second = create_working_copy(HEXIE_EXAMPLE_ID, &temp.0).unwrap();
        assert_ne!(first.project().project_id, second.project().project_id);
        assert_ne!(first.project_path(), second.project_path());
        let untouched = std::fs::read(second.project_path()).unwrap();
        let original = first.project().tracks[0].notes[0].clone();
        let mut edited = original.clone();
        edited.pronunciation = Pronunciation {
            input_text:Some("manual IPA edit".into()), pinyin:"ma1".into(), phonemes:vec!["m".into(),"ɑ".into()],
            language:Some("en".into()), alphabet:Some("ipa".into()), stress:Some(1),
            source:PronunciationSource::Manual, frontend_profile:"manual-test-fixture".into(),
            phoneme_durations_permille:Some(vec![250,750]), reading_provenance:Some(json!({"fixture":"manual edit"})),
            unknown:serde_json::from_value(json!({"retained_phone_metadata":{"future":true}})).unwrap(),
            ..Default::default()
        };
        edited.curves.pitch_millicent_points = vec![PitchPoint { id:"edited-pit".into(),tick:0,millicents:12000,unknown:Default::default() }];
        edited.curves.gain_millidecibel_points = vec![GainPoint { id:"edited-dyn".into(),tick:0,millidecibels:-1200,unknown:Default::default() }];
        for parameter in ["BRE","BRI","CLE","GEN","OPE","GWL","POR"] {
            edited.curves.expression_points.insert(parameter.into(), vec![ExpressionPoint {
                id:format!("edited-{parameter}").as_str().into(),tick:0,value:32,unknown:Default::default() }]);
        }
        edited.vibrato = Some(Vibrato::DEFAULT);
        edited.portamento_ms = Some(60);
        edited.unknown.insert("future_note_field".into(), json!({"preserve":true}));
        let track = first.project().tracks[0].id.clone();
        let mut header = first.project().tracks[0].header();
        header.render_route = Some("C1".into());
        first.commit(first.revision(), CommandBatch { label:"示例完整编辑".into(), commands:vec![
            Command::ReplaceNotes { track:track.clone(),notes:vec![edited.clone()] },
            Command::SetTrackHeader { track,header },
        ]}).unwrap();
        first.undo().unwrap();
        assert_eq!(first.project().tracks[0].notes[0], original);
        assert_eq!(first.project().tracks[0].render_route, None);
        first.redo().unwrap();
        first.save().unwrap();
        let saved = serde_json::to_value(first.project_for_save()).unwrap();
        let path = first.project_path().to_path_buf();
        drop(first);
        let (mut reopened, recovery) = ProjectPersistence::open(&path).unwrap();
        assert!(!recovery.recovered_anything());
        assert_eq!(serde_json::to_value(reopened.project_for_save()).unwrap(), saved);
        assert_eq!(reopened.project().tracks[0].notes[0], edited);
        assert_eq!(reopened.project().tracks[0].render_route.as_deref(), Some("C1"));
        let retired = verdant_backend::adapter::plan_desktop(reopened.project(), &reopened.project().tracks[0].id, &capabilities());
        assert!(retired.has_errors(), "an explicit retired route must not fall back to current synthesis");
        let copy = fresh_copy(reopened.project().clone(), find(HEXIE_EXAMPLE_ID).unwrap());
        assert_ne!(copy.project_id, reopened.project().project_id);
        assert_eq!(copy.revision, 0);
        assert_eq!(copy.tracks[0].notes[0], edited);
        assert_eq!(copy.tracks[0].render_route.as_deref(), Some("C1"));
        assert_eq!(credit_text(&copy), credit_text(reopened.project()));
        reopened.undo().unwrap();
        assert_eq!(reopened.project().tracks[0].notes[0], original);
        assert_eq!(std::fs::read(second.project_path()).unwrap(), untouched);
    }

    #[test]
    fn registered_examples_keep_current_route_and_ten_controls_without_admitting_a_bank() {
        let caps = capabilities();
        for example in EXAMPLES {
            let project = project(example.id).unwrap();
            let authored = serde_json::to_value(&project).unwrap();
            for route in [Route::VerdantVoice] {
                for parameter in verdant_backend::params::ALL {
                    assert!(verdant_backend::params::support(parameter, route, Some(&caps)).is_effective());
                }
                for track in &project.tracks {
                    let planned = verdant_backend::plan(&project, &track.id, route, &caps);
                    assert!(!planned.has_errors(), "{} {route:?}: {:?}", example.id, planned.diagnostics);
                    assert!(!planned.phrases.is_empty());
                }
            }
            assert_eq!(serde_json::to_value(&project).unwrap(), authored);
            let missing: Capabilities = serde_json::from_value(json!({"routes":["verdant-voice"],
                "native_score":{"inventory":verdant_backend::pinyin::recovered_syllables()}})).unwrap();
            assert!(verdant_backend::plan(&project, &project.tracks[0].id, Route::VerdantVoice, &missing).has_errors());
            assert!(missing.missing_files(Route::VerdantVoice).contains(&"voice_source_admission"));
        }
    }
}
