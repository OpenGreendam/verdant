//! Native generated Action events through the real master-insert transaction.
//! Missing-plugin fixtures retain identity and opaque state without a child host.
use super::*;
use gpui::{AppContext, Entity, Subscription, TestAppContext, Window};
use verdant_action_control::{ActionControlHandle, native::gpui_native_owner::{
    all_owners, resolved, replace_props, dispatch_accessibility_action, drive_owners,
}};

struct Harness { editor: Entity<Editor>, key: &'static str, _changed: Subscription }
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key = self.key;
        self.editor.update(cx, |ed, cx| {
            let spec = ed.master_insert_specs().first().cloned().expect("master insert fixture");
            crate::mixer::master_insert_bypass_button(ed, 0, &spec, key, cx)
                .w(px(220.)).h(px(40.)).into_any_element()
        })
    }
}

fn fixture(key: &'static str, cx: &mut TestAppContext) -> gpui::WindowHandle<Harness> {
    cx.open_window(size(px(300.), px(160.)), |window, cx| {
        let editor = cx.new(|cx| {
            let mut project = verdant_domain::Project::new_empty();
            let mut spec = InsertSpec::new("fixture-missing", "rcip-bypass", "RCIP insert", "");
            spec.instance_id = "rcip-original-instance".into();
            spec.state_b64 = "cHJlc2VydmVkLXN0YXRl".into();
            spec.extra.insert("future-field".into(), serde_json::json!({"keep": true}));
            project.extensions.insert(verdant_domain::automation::MASTER_INSERTS_KEY.into(),
                serde_json::Value::Array(encoded(&[spec.clone()])));
            let mut ed = Editor::for_native_test(project, window, cx);
            ed.insert_runtime.callback_budget = ed.audio.callback_budget();
            ed.insert_runtime.tracks = ed.project().tracks.iter().map(|track| AppliedTrack {
                id: track.id.clone(), specs: vec![], live_states: vec![], processors: vec![], padding: 0,
            }).collect();
            for track in &ed.insert_runtime.tracks { ed.inserts.insert(track.id.clone(), vec![]); }
            ed.insert_runtime.master = AppliedMaster {
                specs: vec![spec.clone()], live_states: vec![None], processors: vec![None],
            };
            ed.master_inserts = vec![InsertSlot::Missing { spec, reason: "test fixture".into() }];
            ed
        });
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        Harness { editor, key, _changed: changed }
    })
}

fn source(window: &Window) -> ActionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name")
                .is_some_and(|name| resolved(name).text() == "主总线插入 1 旁通")
    }).expect("production master bypass uses generated ActionControl");
    ActionControlHandle { owner }
}

#[gpui::test]
fn master_bypass_surfaces_preserve_source_value_and_commit_duplicate_proposals_once(cx: &mut TestAppContext) {
    for key in ["master-insert-bypass", "master-param-bypass"] {
        let window = fixture(key, cx);
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        crate::native_test_support::frames(window, cx);
        let original = window.update(cx, |host, window, cx| {
            let original = host.editor.read(cx).master_insert_specs()[0].clone();
            let source = source(window);
            let mut props = source.owner.borrow().props.clone();
            props["selected"] = serde_json::json!(true);
            replace_props(&source.owner, props); drive_owners(window, cx);
            // Source false disagrees with a host-side toggle of accepted false.
            dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
            original
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            assert_eq!(host.editor.read(cx).master_insert_specs()[0], original);
            assert_eq!(host.editor.read(cx).revision().get(), 0, "same value cannot flush or write history");
            let source = source(window);
            let mut props = source.owner.borrow().props.clone();
            props["selected"] = serde_json::json!(false);
            replace_props(&source.owner, props); drive_owners(window, cx);
            dispatch_accessibility_action(&source.owner, "activate", None);
            dispatch_accessibility_action(&source.owner, "activate", None);
            drive_owners(window, cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let mut expected = original.clone(); expected.bypass = true;
            assert_eq!(host.editor.read(cx).master_insert_specs()[0], expected,
                "selected value reaches persisted slot without losing state or unknown fields");
            assert_eq!(host.editor.read(cx).revision().get(), 1, "duplicate source true is one transaction");
        }).unwrap();
        window.update(cx, |_, window, _| window.remove_window()).unwrap(); cx.run_until_parked();
    }
}

#[gpui::test]
fn master_bypass_rejects_retired_document_replaced_slot_and_newly_busy_chain(cx: &mut TestAppContext) {
    for key in ["master-insert-bypass", "master-param-bypass"] {
        for scenario in 0..3 {
            let window = fixture(key, cx);
            window.update(cx, |_, window, _| window.activate_window()).unwrap();
            crate::native_test_support::frames(window, cx);
            let (revision, expected) = window.update(cx, |host, window, cx| {
                let source = source(window);
                host.editor.update(cx, |ed, cx| match scenario {
                    0 => ed.document_generation += 1,
                    1 => {
                        let mut specs = ed.master_insert_specs();
                        specs[0].instance_id = "replacement-in-the-same-slot".into();
                        assert!(ed.commit_owned_inserts(&InsertOwner::Master, specs, "replace fixture", cx));
                    },
                    _ => ed.inserts_loading = true,
                });
                let revision = host.editor.read(cx).revision().get();
                let expected = host.editor.read(cx).master_insert_specs();
                // Deliver before redraw retires the old native source owner.
                dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
                (revision, expected)
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, _, cx| {
                assert_eq!(host.editor.read(cx).master_insert_specs(), expected, "scenario {scenario} cannot write");
                assert_eq!(host.editor.read(cx).revision().get(), revision);
            }).unwrap();
            window.update(cx, |_, window, _| window.remove_window()).unwrap(); cx.run_until_parked();
        }
    }
}

struct TrackHarness { editor: Entity<Editor>, track: TrackId, _changed: Subscription }
impl Render for TrackHarness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        self.editor.update(cx, |ed, cx| crate::devices::device_view(ed, &editor, window, cx))
    }
}

fn track_fixture(cx: &mut TestAppContext) -> gpui::WindowHandle<TrackHarness> {
    cx.open_window(size(px(720.), px(400.)), |window, cx| {
        let editor = cx.new(|cx| {
            let mut project = verdant_domain::Project::new_empty();
            let mut spec = InsertSpec::new("fixture-missing", "rcip-device-power", "RCIP device", "");
            spec.instance_id = "rcip-track-instance".into();
            spec.state_b64 = "cHJlc2VydmVkLXN0YXRl".into();
            spec.extra.insert("future-field".into(), serde_json::json!({"keep": true}));
            verdant_plugins::project::write_inserts(&mut project.tracks[0].unknown, &[spec]);
            let mut ed = Editor::for_native_test(project, window, cx);
            ed.insert_runtime.callback_budget = ed.audio.callback_budget();
            ed.insert_runtime.tracks = ed.project().tracks.iter().map(|track| {
                let specs = verdant_plugins::project::read_inserts(&track.unknown);
                AppliedTrack { id: track.id.clone(), live_states: vec![None; specs.len()],
                    processors: (0..specs.len()).map(|_| None).collect(), specs, padding: 0 }
            }).collect();
            for track in &ed.insert_runtime.tracks {
                ed.inserts.insert(track.id.clone(), track.specs.iter().map(|spec|
                    InsertSlot::Missing { spec: spec.clone(), reason: "test fixture".into() }).collect());
            }
            ed
        });
        let track = editor.read(cx).session.track.clone();
        let changed = cx.observe(&editor, |_, _, cx| cx.notify());
        TrackHarness { editor, track, _changed: changed }
    })
}

fn power_source(window: &Window) -> ActionControlHandle {
    let owner = all_owners().into_iter().find(|owner| {
        let owner = owner.borrow();
        owner.mounted && owner.window_id == Some(window.window_handle().window_id())
            && owner.modules.accessible_text.get("name")
                .is_some_and(|name| resolved(name).text() == "启用效果器")
    }).expect("production device card uses generated ActionControl");
    ActionControlHandle { owner }
}

#[gpui::test]
fn device_power_switch_preserves_source_selection_and_opaque_insert_state(cx: &mut TestAppContext) {
    let window = track_fixture(cx);
    window.update(cx, |_, window, _| window.activate_window()).unwrap();
    crate::native_test_support::frames(window, cx);
    let original = window.update(cx, |host, window, cx| {
        let original = host.editor.read(cx).insert_specs(&host.track)[0].clone();
        let source = power_source(window);
        let mut props = source.owner.borrow().props.clone();
        props["selected"] = serde_json::json!(false);
        replace_props(&source.owner, props); drive_owners(window, cx);
        // The proposal is enabled=true. Re-toggling accepted bypass=false
        // would disable the effect, reversing the source's selected value.
        dispatch_accessibility_action(&source.owner, "activate", None); drive_owners(window, cx);
        original
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, window, cx| {
        assert_eq!(host.editor.read(cx).insert_specs(&host.track)[0], original,
            "source selected=true must keep an already-enabled device enabled");
        assert_eq!(host.editor.read(cx).revision().get(), 0, "same-value power selection cannot write history");
        let source = power_source(window);
        let mut props = source.owner.borrow().props.clone();
        props["selected"] = serde_json::json!(true);
        replace_props(&source.owner, props); drive_owners(window, cx);
        dispatch_accessibility_action(&source.owner, "activate", None);
        dispatch_accessibility_action(&source.owner, "activate", None);
        drive_owners(window, cx);
    }).unwrap();
    crate::native_test_support::frames(window, cx);
    window.update(cx, |host, _, cx| {
        let mut expected = original; expected.bypass = true;
        assert_eq!(host.editor.read(cx).insert_specs(&host.track)[0], expected,
            "source selected=false bypasses once without losing opaque state or unknown fields");
        assert_eq!(host.editor.read(cx).revision().get(), 1);
    }).unwrap();
}
