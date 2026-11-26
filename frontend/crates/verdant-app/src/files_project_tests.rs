use super::*;
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectPersistence};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("verdant-project-entry-{}", Project::new_empty().project_id));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

fn score() -> Project {
    let mut project = Project::new_empty();
    project.ppq = 480;
    project.voice_refs.clear();
    project.tracks[0].voice_ref = None;
    let mut note = Note::new(240, 480, 64);
    note.lyric = "a".into();
    project.tracks[0].notes.push(note);
    project
}

fn opened(result: Result<StartupOpen, String>) -> StartupDocument {
    match result.unwrap() { StartupOpen::Opened(document) => document, StartupOpen::Newer { .. } => panic!("unexpected newer creator") }
}

#[test]
fn save_as_suggestions_use_verp() {
    assert_eq!(project_suggested_name(None), "project.verp");
    for name in ["song.verp", "song.VERP"] {
        assert_eq!(project_suggested_name(Some(Path::new(name))), "song.verp");
    }
    assert_eq!(native_save_path(Path::new("song")).unwrap(), Path::new("song.verp"));
    assert_eq!(native_save_path(Path::new("song.VERP")).unwrap(), Path::new("song.VERP"));
    assert!(native_save_path(Path::new("song.ust")).is_err());
}

#[test]
fn startup_reopens_native_untitled_recovery_and_creates_fresh_verp() {
    let temp = Temp::new();
    let working = temp.0.join("untitled/project.verp");
    let mut store = ProjectPersistence::create(&working, score()).unwrap();
    let track = &store.project().tracks[0];
    let batch = CommandBatch::single("move", Command::MoveNotes {
        track:track.id.clone(), note_ids:vec![track.notes[0].id.clone()], delta_tick:120, delta_semitones:0,
    });
    store.commit(store.revision(), batch).unwrap();
    store.autosave().unwrap();
    let original = std::fs::read(&working).unwrap();
    let expected = store.project_for_save();
    drop(store);
    let opened = opened(open_startup_document(None, &temp.0, &[], None));
    assert_eq!(opened.persistence.project_path(), working.canonicalize().unwrap());
    assert_eq!(opened.persistence.project_for_save(), expected);
    assert!(opened.recovery.recovered_anything());
    assert!(opened.persistence.pending_recovery().is_some());
    assert!(!opened.new_demo);
    assert!(opened.project_path.is_none());
    assert_eq!(std::fs::read(&working).unwrap(), original);
    let fresh = Temp::new();
    let opened = match open_startup_document(None, &fresh.0, &[], None).unwrap() { StartupOpen::Opened(document) => document, _ => panic!("fresh document") };
    assert_eq!(opened.persistence.project_path().canonicalize().unwrap(), fresh.0.join("untitled/project.verp").canonicalize().unwrap());
    assert!(opened.project_path.is_none());
    assert!(opened.new_demo);
    assert_eq!(creator_version(opened.persistence.project()).as_deref(), Some(CREATOR_VERSION));
}

#[test]
fn native_startup_checks_content_and_preserves_builtin_and_external_destinations() {
    let temp = Temp::new();
    let ordinary = temp.0.join("ordinary.json");
    std::fs::write(&ordinary, br#"{"version":1,"project":{"tracks":[]}}"#).unwrap();
    assert!(open_startup_document(Some(&ordinary), &temp.0, &[], None).is_err());
    assert!(!verdant_domain::persistence::journal_path(&ordinary).exists());
    assert!(!temp.0.join("untitled").exists());
    let documents = temp.0.join("documents");
    for extension in ["verp", "VERP"] {
        let project = crate::demos::project(crate::demos::HEXIE_EXAMPLE_ID).unwrap();
        let path = documents.join(format!("example-{}-{}.{extension}", crate::demos::HEXIE_EXAMPLE_ID, project.project_id));
        let store = ProjectPersistence::create(&path, project).unwrap();
        drop(store);
        let mut document = opened(open_startup_document(Some(&path), &temp.0, &[], None));
        assert!(!document.new_demo);
        if extension == "verp" { assert!(document.project_path.is_none()); }
        let external = temp.0.join("external").join(path.file_name().unwrap());
        document.persistence.save_as(&external).unwrap();
        let expected = document.persistence.project_for_save();
        drop(document);
        let reopened = opened(open_startup_document(Some(&external), &temp.0, &[], None));
        assert_eq!(reopened.project_path.as_deref(), Some(external.as_path()));
        assert_eq!(reopened.persistence.project_for_save(), expected);
    }
}

#[gpui::test]
fn file_open_migrates_the_branch_chosen_in_recovery(cx: &mut gpui::TestAppContext) {
    use crate::native_test_support::wait_for_transport_ready;

    struct Harness(gpui::Entity<Editor>);
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { div() }
    }

    let temp = Temp::new();
    let window = cx.open_window(gpui::size(px(800.), px(600.)), |window, cx| {
        Harness(cx.new(|cx| Editor::for_native_test(score(), window, cx)))
    });
    let editor = window.update(cx, |harness, window, cx| {
        window.activate_window();
        // The service-free constructor leaves insert restoration to the test.
        harness.0.update(cx, |ed, cx| ed.restore_inserts(cx));
        harness.0.clone()
    }).unwrap();
    for (answer, expected_tick, expected_undo_depth) in [("恢复", 360, 2), ("放弃", 240, 1)] {
        let path = temp.0.join(format!("{expected_tick}.verp"));
        let mut saved = ProjectPersistence::create(&path, score()).unwrap();
        let saved_revision = saved.revision();
        let saved_bytes = std::fs::read(&path).unwrap();
        let track = &saved.project().tracks[0];
        let edit = CommandBatch::single("move", Command::MoveNotes {
            track: track.id.clone(), note_ids: vec![track.notes[0].id.clone()], delta_tick: 120, delta_semitones: 0,
        });
        assert!(saved.commit(saved_revision, edit).unwrap().is_some());
        let recovered_revision = saved.revision();
        let saved_path = saved.project_path().to_path_buf();
        let project_id = saved.project().project_id.clone();
        drop(saved);

        wait_for_transport_ready(window, cx, &editor);
        window.update(cx, |harness, window, cx| harness.0.update(cx, |ed, cx| {
            assert_ne!(ed.persistence.project_path(), saved_path.as_path());
            open_project_dialog(ed, window, cx);
        })).unwrap();
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|_| Some(vec![path.clone()]));
        cx.run_until_parked();
        window.update(cx, |harness, _, cx| harness.0.update(cx, |ed, _| {
            assert_eq!(ed.persistence.project_path(), saved_path.as_path(), "chooser did not open {}: {:?}",
                saved_path.display(), ed.notices.iter().map(|notice| notice.text.as_str()).collect::<Vec<_>>());
            assert_eq!(ed.project().project_id, project_id);
            assert_eq!(ed.revision(), recovered_revision);
            assert_eq!(ed.project().tracks[0].notes[0].start_tick, 360);
            assert!(ed.persistence.pending_recovery().is_some());
            assert!(ed.project().render_preferences.phoneme_timing.is_none());
            assert_eq!(ed.store().undo_depth(), 1);
        })).unwrap();

        assert!(cx.has_pending_prompt());
        cx.simulate_prompt_answer(answer);
        cx.run_until_parked();
        wait_for_transport_ready(window, cx, &editor);
        window.update(cx, |harness, _, cx| harness.0.update(cx, |ed, cx| {
            let selected_revision = if answer == "恢复" { recovered_revision } else { saved_revision };
            assert_eq!(ed.revision(), selected_revision.next().unwrap());
            assert!(ed.persistence.pending_recovery().is_none());
            assert_eq!(ed.project().tracks[0].notes[0].start_tick, expected_tick);
            assert_eq!(ed.project().render_preferences.phoneme_timing,
                Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1()));
            assert_eq!(ed.store().undo_depth(), expected_undo_depth);
            assert_eq!(ed.persistence.project_path(), saved_path.as_path());
            assert_eq!(ed.project_path.as_deref(), Some(saved_path.as_path()));

            let migrated_revision = ed.revision();
            ed.undo(cx);
            assert_eq!(ed.revision(), migrated_revision.next().unwrap());
            assert!(ed.project().render_preferences.phoneme_timing.is_none());
            assert_eq!(ed.project().tracks[0].notes[0].start_tick, expected_tick);
            assert_eq!(ed.store().undo_depth(), expected_undo_depth - 1);
        })).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), saved_bytes);
    }
}

#[test]
fn ufdata_menu_and_cli_entry_save_reopen_and_export_native_verp() {
    let temp = Temp::new();
    let input = temp.0.join("source.ufdata");
    let source = verdant_exchange::export(verdant_exchange::Format::Ufdata, &score(), &Default::default()).unwrap();
    std::fs::write(&input, &source.bytes).unwrap();
    let format = crate::menu::file_format(&input).unwrap().unwrap();
    let (mut imported, report, working) = import_project_document(&input, &temp.0.join("documents")).unwrap();
    let (second, _, second_working) = import_project_document(&input, &temp.0.join("documents")).unwrap();
    assert_ne!(working, second_working);
    assert_eq!(second.project().tracks[0].notes[0].midi_note, 64);
    assert_eq!(working.extension().unwrap(), "verp");
    assert_eq!(report["format"], "ufdata");
    let saved = temp.0.join("edited.verp");
    imported.save_as(&saved).unwrap();
    let expected = imported.project_for_save();
    drop(imported);
    let (reopened, _) = ProjectPersistence::open(&saved).unwrap();
    assert_eq!(reopened.project_for_save(), expected);
    reopened.export_project(&temp.0.join("native-copy.verp")).unwrap();
    assert_eq!(ProjectPersistence::read_saved(&temp.0.join("native-copy.verp")).unwrap(), expected);
    let exported = verdant_exchange::export(format, reopened.project(), &Default::default()).unwrap();
    let again = verdant_exchange::import(format, &exported.bytes, &Default::default()).unwrap();
    let note = &again.project.tracks[0].notes[0];
    assert_eq!((note.start_tick, note.duration_tick, note.midi_note), (240, 480, 64));
    assert!(open_startup_document(Some(&input), &temp.0.join("cli-state"), &[], None).is_err());
    assert_eq!(creator_version(reopened.project()).as_deref(), Some(CREATOR_VERSION));
    assert_eq!(std::fs::read(input).unwrap(), source.bytes);
}

const VSQX_FIXTURE: &[u8] = include_bytes!("../../verdant-exchange/tests/fixtures/score.vsqx");
// The public fixture's singer identity, supplied as user input in these tests.
const VSQX_SINGER: &str = "BNGW7FG7E5TRSNC3 | KAITO_V3_English | 1 | 0";

struct VsqxExportHarness(gpui::Entity<Editor>);

impl Render for VsqxExportHarness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let prompt = self.0.update(cx, |ed, cx| crate::lyric_view::prompt_overlay(ed, &cx.entity(), cx));
        div().size_full().children(prompt)
    }
}

fn vsqx_editor(project: Project, cx: &mut gpui::TestAppContext) -> gpui::WindowHandle<VsqxExportHarness> {
    let window = cx.open_window(gpui::size(px(800.), px(600.)), |window, cx| {
        let editor = cx.new(|cx| Editor::for_native_test(project, window, cx));
        cx.observe(&editor, |_: &mut VsqxExportHarness, _, cx| cx.notify()).detach();
        VsqxExportHarness(editor)
    });
    let editor = window.update(cx, |harness, window, cx| {
        window.activate_window();
        harness.0.update(cx, |ed, cx| ed.restore_inserts(cx));
        harness.0.clone()
    }).unwrap();
    crate::native_test_support::wait_for_transport_ready(window, cx, &editor);
    window
}

fn submit_vsqx_options(
    window: gpui::WindowHandle<VsqxExportHarness>,
    preserve: bool,
    track: Option<String>,
    cx: &mut gpui::TestAppContext,
) {
    use crate::native_test_support::frames;
    use verdant_action_control::native::gpui_native_owner::{all_owners, dispatch_accessibility_action, drive_owners, resolved};

    window.update(cx, |harness, window, cx| harness.0.update(cx, |ed, cx| {
        export_score_dialog(ed, verdant_exchange::Format::Vsqx, window, cx);
    })).unwrap();
    let dialog = cx.update(|cx| cx.windows().into_iter().find_map(|w| w.downcast::<ScoreExportDialog>())).expect("VSQX options window");
    dialog.update(cx, |_, window, _| window.activate_window()).unwrap();
    frames(dialog, cx);
    let activate = |label: &str, cx: &mut gpui::TestAppContext| {
        cx.update_window(dialog.into(), |_, window, cx| {
            let owner = all_owners().into_iter().find(|owner| {
                let owner = owner.borrow();
                owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                    && owner.modules.accessible_text.get("name").is_some_and(|name| resolved(name).text().contains(label))
            }).unwrap_or_else(|| panic!("compiled VSQX option {label}"));
            dispatch_accessibility_action(&owner, "activate", None);
            drive_owners(window, cx);
        }).unwrap();
    };
    if !preserve {
        activate("保留来源扩展", cx);
        frames(dialog, cx);
    }
    if let Some(track) = track {
        activate(&track, cx);
        frames(dialog, cx);
    }
    activate("选择保存位置", cx);
    cx.run_until_parked();
    assert!(dialog.update(cx, |_, _, _| ()).is_err(), "accepted options close the form");
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    frames(window, cx);
}

fn has_vsqx_prompt(window: gpui::WindowHandle<VsqxExportHarness>, cx: &mut gpui::TestAppContext) -> bool {
    window.update(cx, |harness, _, cx| harness.0.read(cx).text_prompt.as_ref().is_some_and(|prompt| {
        matches!(&prompt.purpose, crate::editor::PromptPurpose::Integration(crate::editor::integrations::IntegrationPrompt::VsqxExportSinger(_)))
    })).unwrap()
}

#[gpui::test]
fn native_vsqx_export_discarded_voice_table_accepts_retry_and_keeps_selected_track(cx: &mut gpui::TestAppContext) {
    use crate::native_test_support::frames;
    use verdant_exchange::Format;
    use verdant_text_field::{TextFieldHandle, native::gpui_native_owner::all_owners};

    let invalid = "BNGW7FG7E5T SNC3 | KAITO_V3_English | 1 | 0";
    let text_owner = |window: &Window| {
        let owner = all_owners().into_iter().find(|owner| {
            let owner = owner.borrow();
            owner.present && owner.mounted && !owner.disposed
                && owner.window_id == Some(window.window_handle().window_id()) && owner.exposes.contains_key("viewEpoch")
        }).expect("compiled singer text field");
        TextFieldHandle { owner }
    };

    let temp = Temp::new();
    let source = temp.0.join("source.vsqx");
    std::fs::write(&source, VSQX_FIXTURE).unwrap();
    let (imported, _, _) = import_project_document(&source, &temp.0.join("documents")).unwrap();
    let mut project = imported.project_for_save();
    assert!(has_vsqx_voice_table(&project));
    assert!(!project.extensions.contains_key("vocaloid_voice"));
    let mut other = verdant_domain::Track::new("Omitted track", "teal");
    other.notes.push(Note::new(240, 480, 67));
    other.refresh_index();
    project.tracks.push(other);
    let window = vsqx_editor(project, cx);
    submit_vsqx_options(window, false, Some("Lead & harmony".into()), cx);
    assert!(has_vsqx_prompt(window, cx));
    assert!(!cx.did_prompt_for_new_path());

    let (instance, field, epoch) = window.update(cx, |harness, window, cx| {
        let field = text_owner(window);
        let epoch = field.state("viewEpoch").unwrap();
        let instance = harness.0.read(cx).text_prompt.as_ref().unwrap().instance.clone();
        (instance, field, epoch)
    }).unwrap();
    cx.simulate_input(window.into(), invalid);
    cx.simulate_keystrokes(window.into(), "enter");
    frames(window, cx);
    assert!(has_vsqx_prompt(window, cx), "invalid singer data retains the pending export");
    assert!(!cx.did_prompt_for_new_path());
    window.update(cx, |harness, window, cx| {
        let ed = harness.0.read(cx);
        assert!(ed.notices.iter().any(|notice| notice.text.contains("可见 ASCII")));
        let prompt = ed.text_prompt.as_ref().unwrap();
        assert_eq!(prompt.instance, instance);
        assert_eq!(prompt.state.text, invalid);
        assert!(std::rc::Rc::ptr_eq(&text_owner(window).owner, &field.owner));
        assert_eq!(field.state("viewEpoch").unwrap(), epoch);
        assert_eq!(field.state("focused").unwrap(), serde_json::json!(true), "validation keeps keyboard focus in the surviving singer field");
    }).unwrap();

    // An ordinary edit while the singer prompt is open belongs only to the
    // live document. The export continues from the originally chosen score.
    let edited = window.update(cx, |harness, _, cx| harness.0.update(cx, |ed, cx| {
        let track = &ed.project().tracks[0];
        let command = Command::MoveNotes {
            track: track.id.clone(), note_ids: vec![track.notes[0].id.clone()], delta_tick: 0, delta_semitones: 2,
        };
        assert!(ed.commit(ed.revision(), CommandBatch::single("transpose during export", command), cx));
        ed.persistence.project_for_save()
    })).unwrap();
    assert_eq!(edited.tracks[0].notes[0].midi_note, 62);
    frames(window, cx);
    cx.simulate_keystrokes(window.into(), "secondary-a");
    cx.simulate_input(window.into(), VSQX_SINGER);
    window.update(cx, |harness, window, cx| {
        let ed = harness.0.read(cx);
        let prompt = ed.text_prompt.as_ref().unwrap();
        assert_eq!(prompt.instance, instance);
        assert_ne!(prompt.base, ed.revision());
        assert!(std::rc::Rc::ptr_eq(&text_owner(window).owner, &field.owner));
        assert_eq!(field.state("viewEpoch").unwrap(), epoch);
        assert_eq!(field.state("value").unwrap(), serde_json::json!(VSQX_SINGER), "native select-all and typing replace the whole draft");
        assert_eq!(prompt.state.text, VSQX_SINGER);
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "enter");
    cx.run_until_parked();
    assert!(!has_vsqx_prompt(window, cx));
    assert!(cx.did_prompt_for_new_path());
    let output = temp.0.join("export.vsqx");
    cx.simulate_new_path_selection(|_| Some(output.clone()));
    cx.run_until_parked();
    let exported = std::fs::read_to_string(output).unwrap();
    assert!(exported.contains("BNGW7FG7E5TRSNC3"));
    assert!(!exported.contains("source-extension"));
    let roundtrip = verdant_exchange::import(Format::Vsqx, exported.as_bytes(), &Default::default()).unwrap();
    assert_eq!(roundtrip.project.tracks.len(), 1, "only the selected track is written");
    let track = &roundtrip.project.tracks[0];
    assert_eq!(track.name, "Lead & harmony");
    assert_eq!(track.notes[0].midi_note, 60);
    assert_eq!(std::fs::read(source).unwrap(), VSQX_FIXTURE);
    window.update(cx, |harness, _, cx| assert_eq!(harness.0.read(cx).persistence.project_for_save(), edited)).unwrap();
}

#[gpui::test]
fn native_vsqx_export_new_score_remembers_singer_only_after_success_and_reuses_it(cx: &mut gpui::TestAppContext) {
    use crate::native_test_support::frames;

    let temp = Temp::new();
    let window = vsqx_editor(score(), cx);
    let before = window.update(cx, |harness, _, cx| harness.0.read(cx).persistence.project_for_save()).unwrap();
    for (accept_singer, destination) in [(false, None), (true, None), (true, Some(temp.0.join("missing/out.vsqx")))] {
        submit_vsqx_options(window, true, None, cx);
        assert!(has_vsqx_prompt(window, cx), "new scores need an explicit singer even with preservation enabled");
        cx.simulate_input(window.into(), VSQX_SINGER);
        cx.simulate_keystrokes(window.into(), if accept_singer { "enter" } else { "escape" });
        frames(window, cx);
        assert!(!has_vsqx_prompt(window, cx));
        assert_eq!(cx.did_prompt_for_new_path(), accept_singer);
        if accept_singer {
            cx.simulate_new_path_selection(|_| destination);
            cx.run_until_parked();
        }
        window.update(cx, |harness, _, cx| {
            let ed = harness.0.read(cx);
            assert_eq!(ed.persistence.project_for_save(), before);
            assert!(ed.prefs.last_export_dir.is_none());
            assert!(!ed.prefs.extra.contains_key("last_exchange_report"));
        }).unwrap();
    }

    submit_vsqx_options(window, true, None, cx);
    cx.simulate_input(window.into(), VSQX_SINGER);
    cx.simulate_keystrokes(window.into(), "enter");
    cx.run_until_parked();
    let output = temp.0.join("new-score.vsqx");
    cx.simulate_new_path_selection(|_| Some(output.clone()));
    cx.run_until_parked();
    assert!(std::fs::read_to_string(output).unwrap().contains("BNGW7FG7E5TRSNC3"));
    window.update(cx, |harness, _, cx| {
        let ed = harness.0.read(cx);
        assert_eq!(ed.project().extensions["vocaloid_voice"]["comp_id"], "BNGW7FG7E5TRSNC3");
        assert_eq!(ed.store().undo_depth(), 1, "remembering a singer is an undoable edit");
    }).unwrap();

    submit_vsqx_options(window, false, None, cx);
    assert!(!has_vsqx_prompt(window, cx));
    let reused = temp.0.join("reused.vsqx");
    cx.simulate_new_path_selection(|_| Some(reused.clone()));
    cx.run_until_parked();
    assert!(std::fs::read_to_string(reused).unwrap().contains("BNGW7FG7E5TRSNC3"));
    window.update(cx, |harness, _, cx| assert_eq!(harness.0.read(cx).store().undo_depth(), 1)).unwrap();
}

#[gpui::test]
fn native_vsqx_export_uses_retained_source_voice_without_prompt(cx: &mut gpui::TestAppContext) {
    use verdant_exchange::Format;

    let temp = Temp::new();
    let imported = verdant_exchange::import(Format::Vsqx, VSQX_FIXTURE, &Default::default()).unwrap();
    let window = vsqx_editor(imported.project, cx);
    submit_vsqx_options(window, true, None, cx);
    assert!(!has_vsqx_prompt(window, cx));
    assert!(cx.did_prompt_for_new_path());
    let retained = temp.0.join("retained.vsqx");
    cx.simulate_new_path_selection(|_| Some(retained.clone()));
    cx.run_until_parked();
    let retained = std::fs::read_to_string(retained).unwrap();
    assert!(retained.contains("source-extension"));
    assert!(retained.contains("BNGW7FG7E5TRSNC3"));
}

#[gpui::test]
fn native_vsqx_export_finishes_frozen_write_without_touching_replacement_document(cx: &mut gpui::TestAppContext) {
    use verdant_exchange::Format;

    let temp = Temp::new();
    let window = vsqx_editor(score(), cx);
    submit_vsqx_options(window, true, None, cx);
    cx.simulate_input(window.into(), VSQX_SINGER);
    cx.simulate_keystrokes(window.into(), "enter");
    cx.run_until_parked();
    assert!(cx.did_prompt_for_new_path());

    let mut replacement = score();
    replacement.tracks[0].notes[0].midi_note = 72;
    replacement.tracks[0].refresh_index();
    let document = ProjectPersistence::create(&temp.0.join("replacement.verp"), replacement).unwrap();
    let (expected, undo_depth) = window.update(cx, |harness, window, cx| harness.0.update(cx, |ed, cx| {
        ed.replace_project_opened(document, Default::default(), window, cx);
        (ed.persistence.project_for_save(), ed.store().undo_depth())
    })).unwrap();
    let output = temp.0.join("original.vsqx");
    cx.simulate_new_path_selection(|_| Some(output.clone()));
    cx.run_until_parked();
    let exported = verdant_exchange::import(Format::Vsqx, &std::fs::read(output).unwrap(), &Default::default()).unwrap();
    assert_eq!(exported.project.tracks[0].notes[0].midi_note, 64);
    window.update(cx, |harness, _, cx| {
        let ed = harness.0.read(cx);
        assert_eq!(ed.persistence.project_for_save(), expected);
        assert_eq!(ed.store().undo_depth(), undo_depth);
        assert!(ed.prefs.last_export_dir.is_none());
        assert!(!ed.prefs.extra.contains_key("last_exchange_report"));
    }).unwrap();
}

#[test]
fn creator_gate_precedes_journal_creation_and_preserves_creator_on_save() {
    let temp = Temp::new();
    for creator in [None, Some("0.0.1"), Some(CREATOR_VERSION), Some("99999.0.0"), Some("future-version")] {
        let path = temp.0.join(format!("{}.VERP", Project::new_empty().project_id));
        let mut project = score();
        if let Some(creator) = creator { project.extensions.insert(CREATOR_EXTENSION.into(), serde_json::json!(creator)); }
        verdant_domain::io::save_project_atomic(&path, &project).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let needs_confirmation = creator.is_some_and(|value| creator_is_newer(value, CREATOR_VERSION));
        match open_native_project(&path, None).unwrap() {
            NativeOpen::Newer(value) => {
                assert!(needs_confirmation);
                assert_eq!(Some(value.as_str()), creator);
                assert!(!verdant_domain::persistence::journal_path(&path).exists());
                assert!(matches!(open_startup_document(Some(&path), &temp.0, &[], None).unwrap(), StartupOpen::Newer { .. }));
            }
            NativeOpen::Opened(_, _) => assert!(!needs_confirmation),
        }
        let NativeOpen::Opened(mut document, _) = open_native_project(&path, creator).unwrap() else { panic!("explicit creator approval"); };
        let destination = temp.0.join(format!("copy-{}.verp", project.project_id));
        document.save_as(&destination).unwrap();
        assert_eq!(creator_version(document.project()).as_deref(), creator);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    assert!(!creator_is_newer("0.1.0-alpha.1", "0.1.0"));
    assert!(!creator_is_newer("0.1.0+build.2", "0.1.0+build.1"));
    assert!(creator_is_newer("0.10.0", "0.9.0"));
}

#[test]
fn retired_projects_are_rejected_by_open_and_import_without_touching_source() {
    use verdant_domain::io::RETIRED_PROJECT_FORMAT_MESSAGE;
    use verdant_domain::persistence::{journal_path, lock_path};

    let temp = Temp::new();
    let documents = temp.0.join("documents");
    for extension in ["json", "verdant.JSON", "gdmproj", "GDMProj"] {
        let path = temp.0.join(format!("old.{extension}"));
        let bytes = serde_json::to_vec(&score()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let journal = journal_path(&path);
        std::fs::write(&journal, b"existing recovery bytes").unwrap();
        assert_eq!(open_native_project(&path, None).err().unwrap(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(open_startup_document(Some(&path), &temp.0, &[], None).err().unwrap(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(import_project_document(&path, &documents).err().unwrap(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(native_save_path(&path).unwrap_err(), RETIRED_PROJECT_FORMAT_MESSAGE);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read(&journal).unwrap(), b"existing recovery bytes");
        assert!(!lock_path(&path).exists());
        assert!(!documents.exists());
    }
}

#[test]
fn recovery_prompt_uses_relative_time_and_file_name() {
    let now = 1_800_000_000_000;
    assert_eq!(relative_time_text(now - 20_000, now), "刚刚");
    assert_eq!(relative_time_text(now + 5_000, now), "刚刚");
    assert!(relative_time_text(now - 5 * 60_000, now).starts_with("5 分钟前（"));
    assert!(relative_time_text(now - 3 * 3_600_000, now).starts_with("3 小时前（"));
    assert!(relative_time_text(now - 30 * 3_600_000, now).starts_with("昨天 "));
    assert_eq!(relative_time_text(now - 3 * 86_400_000, now).len(), "2027-01-12 08:00".len());
    let report = verdant_domain::RecoveryReport { saved_revision: 4, recovered_revision: 9, recovered_at_ms: now - 120_000, ..Default::default() };
    let detail = recovery_prompt_detail(Path::new("/very/long/folder/that/would/overflow/歌.verp"), &report, now);
    assert!(detail.starts_with("「歌.verp」"), "{detail}");
    assert!(detail.contains("2 分钟前") && detail.contains("5 步编辑"), "{detail}");
    assert!(!detail.contains("/very/long") && !detail.contains("Unix"), "{detail}");
}
