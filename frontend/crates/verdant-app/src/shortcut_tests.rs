//! Registry-level acceptance: real GPUI key/context parsing, installed action
//! listeners, and the exact row model consumed by the shortcut window.
use super::*;
use gpui::{KeyContext, Keystroke, OperatingSystem};

fn keys(source: &str, mac: bool) -> Vec<Keystroke> {
    let os = if mac { OperatingSystem::Mac } else { OperatingSystem::Linux };
    source.split_whitespace().map(|stroke| {
        Keystroke::parse_with_os(stroke, os)
            .unwrap_or_else(|error| panic!("invalid binding {source}: {error}"))
    }).collect()
}

fn compact(source: &str) -> String { source.chars().filter(|c| !c.is_whitespace()).collect() }

#[test]
fn shortcut_table_is_conflict_free_dispatchable_and_exactly_rendered() {
    // Enter, Space, arrows and clipboard keys intentionally mean different
    // things in mutually exclusive panes. Check actual predicate overlap,
    // rather than rejecting legitimate text-field bindings by key alone.
    let focus_paths = [
        vec!["Verdant"],
        vec!["Verdant", "Arrange"],
        vec!["Verdant", "PianoRoll"],
        vec!["Verdant", "Mixer"],
        vec!["Verdant", "NoteTable"],
        vec!["Verdant", "NoteTable", "NoteTablePoint"],
        vec!["Verdant", "LyricEditor"],
        vec!["Verdant", "TextPrompt"],
        vec!["Verdant", "PianoRoll", "LyricEditor"],
        vec!["Verdant", "PianoRoll", "TextPrompt"],
        vec!["Verdant", "Arrange", "TextPrompt"],
        vec!["Verdant", "NoteTable", "LyricEditor"],
    ];
    let contexts: Vec<Vec<KeyContext>> = focus_paths.iter().map(|path| {
        path.iter().map(|context| KeyContext::parse(context).expect("fixture key context")).collect()
    }).collect();
    let predicates: Vec<_> = SHORTCUTS.iter().map(|s| {
        KeyBindingContextPredicate::parse(s.scope.predicate()).expect("shortcut predicate")
    }).collect();

    for mac in [false, true] {
        let parsed: Vec<_> = SHORTCUTS.iter().map(|s| keys(s.keys, mac)).collect();
        for (i, a) in SHORTCUTS.iter().enumerate() {
            assert!(!parsed[i].is_empty(), "empty shortcut for {}", a.label);
            assert!(contexts.iter().any(|path| predicates[i].depth_of(path).is_some()), "unreachable shortcut: {}", a.label);
            for j in 0..i {
                let b = &SHORTCUTS[j];
                if parsed[i].starts_with(&parsed[j]) || parsed[j].starts_with(&parsed[i]) {
                    for (path, names) in contexts.iter().zip(&focus_paths) {
                        assert!(predicates[i].depth_of(path).is_none() || predicates[j].depth_of(path).is_none(),
                            "duplicate/prefix conflict on {names:?}, mac={mac}: {} ({}) vs {} ({})",
                            a.keys, a.label, b.keys, b.label);
                    }
                }
            }
        }

        // These are the actual rows stored in Shortcuts and rendered without
        // filtering. Identity covers both application and compiled text routes;
        // the displayed platform modifiers and chord delimiters are also exact.
        let rows = cheat_sheet(mac);
        assert_eq!(rows.len(), SHORTCUTS.len());
        for (row, binding) in rows.iter().zip(SHORTCUTS) {
            assert!(std::ptr::eq(row.shortcut, binding));
            assert_eq!(row.keys.as_ref(), display_keys(binding.keys, mac));
            assert!(!row.keys.contains("secondary-"));
            assert!(!binding.label.is_empty());
            assert!(!binding.group.is_empty());
            assert!(!binding.scope.label().is_empty());
        }
    }
    assert_eq!(display_keys("secondary-shift-z", false), "Ctrl+Shift+z");
    assert_eq!(display_keys("secondary-shift-z", true), "⌘+Shift+z");
    assert_eq!(display_keys("secondary-k p n", false), "Ctrl+k → p → n");

    // Every application action must have a listener, not just a declaration that
    // KeyBinding would happily accept. The custom enum has one typed listener
    // and an exhaustive resolver to existing editor intents.
    let view = compact(include_str!("view.rs"));
    let lyric = compact(include_str!("lyric_view.rs"));
    let commands = compact(include_str!("commands.rs"));
    assert!(view.contains("cx.listener(crate::shortcuts::dispatch)") || view.contains("crate::shortcuts::dispatch(ed,"),
        "Editor root must install the WorkflowShortcut dispatcher");
    let track = verdant_domain::TrackId::generate();
    for shortcut in SHORTCUTS {
        let ShortcutDispatch::Application(create) = shortcut.dispatch else {
            assert!(matches!(shortcut.scope, Scope::Text | Scope::Lyric));
            // These physical keys are exercised with this registry installed in
            // proto_text::tests::native_editor_owns_the_registered_text_shortcuts.
            // Source tests separately cover composition and terminal epoch gates.
            assert!(matches!(shortcut.keys, "enter" | "tab" | "shift-tab" | "escape" | "secondary-z"
                | "secondary-shift-z" | "secondary-y" | "space" | "backspace" | "delete" | "left" | "right"
                | "shift-left" | "shift-right" | "home" | "end" | "secondary-a" | "secondary-v"),
                "compiled text route needs a native behavioral case: {}", shortcut.keys);
            continue;
        };
        let action = create();
        if let Some(workflow) = action.as_any().downcast_ref::<WorkflowShortcut>() {
            // Calling the exhaustive resolver also covers every bound enum
            // value, including all concrete score formats and part operations.
            let _ = workflow.resolve(&track, || 960);
        } else {
            let name = action.name().rsplit("::").next().expect("action name");
            let listener_type = format!(":&{name},");
            let qualified = format!(":&crate::actions::{name},");
            let history = format!(":&crate::commands::{name},");
            assert!(view.contains(&listener_type) || view.contains(&qualified)
                || view.contains(&history)
                || lyric.contains(&listener_type) || lyric.contains(&qualified)
                || commands.contains(&listener_type), "binding {} has no dispatcher for {name}", shortcut.keys);
        }
    }
}

#[test]
fn workflow_shortcuts_preserve_part_linking_playhead_and_exchange_format() {
    use crate::editor::integrations::IntegrationAction;
    use verdant_domain::editing::Interpolation;
    let track = verdant_domain::TrackId::generate();
    assert!(matches!(WorkflowShortcut::LinkedPart.resolve(&track, || 1440), WorkflowAction::DuplicatePart(true)));
    assert!(matches!(WorkflowShortcut::DuplicatePart.resolve(&track, || 1440), WorkflowAction::DuplicatePart(false)));
    assert!(matches!(WorkflowShortcut::SplitPart.resolve(&track, || 1440), WorkflowAction::SplitPart(1440)));
    assert!(matches!(WorkflowShortcut::LoopEnd.resolve(&track, || 1440), WorkflowAction::LoopEnd(1440)));
    assert!(matches!(WorkflowShortcut::MoveTrackUp.resolve(&track, || 1440), WorkflowAction::MoveTrack(id, -1) if id == track));
    assert!(matches!(WorkflowShortcut::ExportSvp.resolve(&track, || 1440),
        WorkflowAction::Integration(IntegrationAction::ScoreExport(verdant_exchange::Format::Svp))));
    assert!(matches!(WorkflowShortcut::HoldPoints.resolve(&track, || 1440), WorkflowAction::Interpolation(Interpolation::Hold)));
}
