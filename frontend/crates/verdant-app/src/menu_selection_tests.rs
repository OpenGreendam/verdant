//! Generated Action -> generated Collection -> actual editor acceptance.
use super::*;
use gpui::{AppContext, Entity, Render, Subscription, TestAppContext, WindowHandle, size};
use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{
    all_owners as action_owners, resolved as action_resolved, replace_props, dispatch_accessibility_action, drive_owners,
}};
use verdant_collection_control::{CollectionControlHandle, native::gpui_native_owner::{all_owners, resolved}};

struct Harness { editor: Entity<Editor>, parts: [String; 2], clock: bool, _changed: Subscription }
impl Render for Harness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clock = self.clock;
        self.editor.update(cx, |ed, cx| {
            let menu = match ed.menu.clone() {
                Some(menu) => context_menu_layer(ed, &menu, window, cx).into_any_element(),
                None => div().into_any_element(),
            };
            if clock { div().size_full().child(menu).child(ed.playback.driver.clone()).into_any_element() }
            else { menu }
        })
    }
}

fn items(ed: &Editor) -> Vec<MenuItem> {
    vec![
        MenuItem::boolean_command("Menu mute", CommandId::ToggleMute, CommandArg::Track(ed.session.track.clone()), false),
        MenuItem::boolean("Menu follow", MenuBoolean::FollowPlayhead, false),
        MenuItem::boolean("Menu export click", MenuBoolean::ExportClick, false),
        MenuItem::action("Menu pencil", MenuAction::SetTool(crate::editor::Tool::Pencil)).checked(ed.tool == crate::editor::Tool::Pencil),
        MenuItem::boolean("Disabled follow", MenuBoolean::FollowPlayhead, false).enabled(false),
    ]
}

fn fixture(cx: &mut TestAppContext) -> WindowHandle<Harness> {
    let window = cx.open_window(size(px(600.), px(500.)), |window, cx| {
        let mut project = verdant_domain::Project::new_empty();
        let track = project.tracks[0].id.clone();
        let mut parts = verdant_domain::parts::Parts::default();
        let first = parts.create_empty(&project, &track, 0, 960, "First".into()).unwrap();
        let second = parts.create_empty(&project, &track, 1920, 960, "Second".into()).unwrap();
        project.extensions.insert(verdant_domain::parts::KEY.into(), serde_json::to_value(parts).unwrap());
        let editor = cx.new(|cx| Editor::for_native_test(project, window, cx));
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        editor.update(cx, |ed, cx| {
            ed.session.follow_playhead = false;
            ed.show_menu(gpui::point(px(80.), px(40.)), items(ed), window, cx);
        });
        Harness { editor, parts: [first, second], clock: false, _changed: changed }
    });
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    window
}

fn reopen(window: WindowHandle<Harness>, cx: &mut TestAppContext) {
    window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
        ed.show_menu(gpui::point(px(80.), px(40.)), items(ed), window, cx);
    })).unwrap();
    crate::native_test_support::frames(window, cx);
}

fn collection(window: &Window) -> CollectionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|name| resolved(name).text() == "上下文菜单")
    }).expect("menu is backed by the compiled Collection");
    CollectionControlHandle { owner }
}

fn row(window: &Window, label: &str) -> ActionControlHandle {
    let owner = action_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name").is_some_and(|name| action_resolved(name).text() == label)
    }).expect("menu row is backed by the compiled Action");
    ActionControlHandle { owner }
}

#[gpui::test]
fn checked_menu_preserves_child_proposals_and_closes_duplicate_events_once(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |_, window, cx| {
        let row = row(window, "Menu mute");
        let mut props = row.owner.borrow().props.clone(); props["selected"] = serde_json::json!(true);
        replace_props(&row.owner, props); drive_owners(window, cx);
        dispatch_accessibility_action(&row.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let ed = host.editor.read(cx);
        assert!(!ed.active_track().unwrap().muted, "source false survives both native bridges");
        assert_eq!(ed.revision().get(), 0);
        assert!(ed.menu.is_none());
    }).unwrap();
    reopen(window, cx);
    window.update(cx, |_, window, cx| {
        let row = row(window, "Menu mute");
        dispatch_accessibility_action(&row.owner, "activate", None);
        dispatch_accessibility_action(&row.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let ed = host.editor.read(cx);
        assert!(ed.active_track().unwrap().muted);
        assert_eq!(ed.revision().get(), 1);
        assert!(ed.menu.is_none());
    }).unwrap();
}

#[gpui::test]
fn menu_keyboard_uses_source_checkbox_facts_and_radio_choices_remain_commands(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |_, window, cx| {
        collection(window).call("setItems", vec![serde_json::json!([4, 3, 3, 5, 0])], window, cx).unwrap();
    }).unwrap();
    cx.simulate_keystrokes(window.into(), "enter");
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let ed = host.editor.read(cx);
        assert!(!ed.active_track().unwrap().muted, "source checked fact proposes false on Enter");
        assert_eq!(ed.revision().get(), 0);
        assert!(ed.menu.is_none());
    }).unwrap();
    for _ in 0..2 {
        reopen(window, cx);
        window.update(cx, |_, window, _| {
            assert_eq!(row(window, "Menu pencil").owner.borrow().props["role"], serde_json::json!("menuitemradio"));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "down down down enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert_eq!(ed.tool, crate::editor::Tool::Pencil);
            assert_eq!(ed.revision().get(), 0);
            assert!(ed.menu.is_none());
        }).unwrap();
    }
}

#[gpui::test]
fn menu_rejects_retired_ui_only_acceptance_and_reconciles_the_next_render(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |host, window, cx| {
        let row = row(window, "Menu follow");
        let mut props = row.owner.borrow().props.clone(); props["selected"] = serde_json::json!(true);
        replace_props(&row.owner, props); drive_owners(window, cx);
        host.editor.update(cx, |ed, _| ed.session.follow_playhead = true);
        dispatch_accessibility_action(&row.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let ed = host.editor.read(cx);
        assert!(ed.session.follow_playhead, "a queued old false cannot overwrite the new accepted true");
        assert_eq!(ed.revision().get(), 0);
        assert!(ed.menu.is_some());
        let row = row(window, "Menu follow");
        assert_eq!(row.owner.borrow().props["selected"], serde_json::json!(true));
        dispatch_accessibility_action(&row.owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let ed = host.editor.read(cx);
        assert!(!ed.session.follow_playhead);
        assert_eq!(ed.revision().get(), 0);
        assert!(ed.menu.is_none());
    }).unwrap();
}

#[gpui::test]
fn menu_follow_starts_the_idle_clock_driver_and_moves_both_viewports(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |host, _, cx| {
        host.clock = true;
        host.editor.update(cx, |ed, _| {
            assert_ne!(ed.audio.transport(), verdant_audio::Transport::Playing);
            ed.session.viewport.start_tick = 12000.0;
            ed.arrange.start_tick = 12000.0;
        });
        cx.notify();
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let ed = host.editor.read(cx);
        assert_eq!(ed.session.viewport.start_tick, 12000.0);
        assert_eq!(ed.arrange.start_tick, 12000.0);
        assert!(!ed.session.follow_playhead);
        dispatch_accessibility_action(&row(window, "Menu follow").owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let ed = host.editor.read(cx);
        assert!(ed.session.follow_playhead);
        assert_ne!(ed.audio.transport(), verdant_audio::Transport::Playing);
        assert!(ed.session.viewport.start_tick < 12000.0, "the idle piano-roll viewport follows after the source menu enables follow");
        assert!(ed.arrange.start_tick < 12000.0, "the idle arrangement viewport follows through the same clock driver");
        assert_eq!(ed.revision().get(), 0);
    }).unwrap();
}

#[gpui::test]
fn menu_part_mute_keeps_its_target_and_cached_plugin_parameters_cannot_enable_a_removed_slot(cx: &mut TestAppContext) {
    let window = fixture(cx);
    window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
        ed.selected_part = Some(host.parts[1].clone());
        ed.show_menu(gpui::point(px(80.), px(40.)), vec![
            MenuItem::boolean("Part mute", MenuBoolean::PartMuted(host.parts[0].clone()), false),
        ], window, cx);
    })).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |_, window, cx| {
        dispatch_accessibility_action(&row(window, "Part mute").owner, "activate", None); drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let parts = verdant_domain::parts::Parts::load(host.editor.read(cx).project()).unwrap();
        assert!(parts.instance(&host.parts[0]).unwrap().muted);
        assert!(!parts.instance(&host.parts[1]).unwrap().muted);
        assert_eq!(host.editor.read(cx).revision().get(), 1);
        host.editor.update(cx, |ed, cx| {
            let key = verdant_plugins::InsertKey(424242);
            ed.plugin_params.insert(key, verdant_plugins::builtin_dsp::EqParams::default().descs());
            ed.show_menu(gpui::point(px(80.), px(40.)), vec![MenuItem::boolean("Removed EQ", MenuBoolean::DeviceParameter {
                target: crate::devices::DeviceTarget::Master, slot: 0, key,
                parameter: verdant_plugins::builtin_dsp::eq_param_id(0, "slope_continuous"),
            }, false)], window, cx);
        });
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        let source = collection(window);
        let revision = source.owner.borrow().props["revision"].clone();
        assert_eq!(source.call("chooseSelection", vec![0.into(), true.into(), revision], window, cx).unwrap(), serde_json::json!(false));
        assert_eq!(row(window, "Removed EQ").owner.borrow().props["disabled"], serde_json::json!(true));
        assert_eq!(host.editor.read(cx).revision().get(), 1);
    }).unwrap();
}
