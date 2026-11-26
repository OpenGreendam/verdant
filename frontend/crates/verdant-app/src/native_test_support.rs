//! Real GPUI/editor fixtures with an isolated durable project. No worker,
//! plugin process, preference persistence or hardware audio is started.

use std::path::PathBuf;
use gpui::{AppContext, Entity, Render, TestAppContext, WindowHandle};
use verdant_domain::{Project, ProjectPersistence, Track, TrackId, TRACK_COLORS};

use crate::editor::Editor;

pub(crate) struct TestDocument { pub path: PathBuf }

impl TestDocument {
    pub(crate) fn create(project: Project) -> (Self, ProjectPersistence) {
        let path = std::env::temp_dir().join(format!("verdant-native-test-{}", TrackId::generate()));
        std::fs::create_dir(&path).expect("isolated native test directory");
        let persistence = ProjectPersistence::create(&path.join("project.verp"), project).expect("real durable test store");
        (Self { path }, persistence)
    }
}

impl Drop for TestDocument {
    fn drop(&mut self) {
        // Editor declares this guard last, after persistence/audio. File leases
        // have been dropped before cleanup, including on Windows.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn instrument_project() -> Project {
    let mut project = Project::new_empty();
    let mut track = Track::new("Native fixture", TRACK_COLORS[0]);
    track.unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(),
        crate::instrument::synth_spec(&verdant_plugins::builtin_dsp::SynthParams::default(), "test"));
    project.tracks = vec![track, Track::new("Other track", TRACK_COLORS[1])];
    project
}

pub(crate) fn frames<T: Render + 'static>(window: WindowHandle<T>, cx: &mut TestAppContext) {
    for _ in 0..3 {
        cx.update_window(window.into(), |_, window, cx| {
            // TestPlatform has no automatic display-link loop. Deliver the
            // same callbacks a real frame runs before drawing, including
            // measured source reveal requests and retained focus updates.
            window.simulate_next_frame(cx);
            window.refresh();
            window.draw(cx).clear(cx);
        }).unwrap();
        cx.run_until_parked();
    }
}

pub(crate) fn transport_diagnostics(ed: &mut Editor) -> String {
    format!("revision={}, loading={}, callback_budget={}, retired_chain={}, applied_timeline={}, {}, notices={:?}",
        ed.revision().get(), ed.inserts_loading, ed.audio.callback_budget(),
        ed.audio.retired_chain_revision(), ed.audio.applied_timeline_revision(), ed.native_insert_receipt_diagnostics(),
        ed.notices.iter().map(|notice| notice.text.as_str()).collect::<Vec<_>>())
}

pub(crate) fn wait_for_transport_ready<T: Render + 'static>(
    window: WindowHandle<T>, cx: &mut TestAppContext, editor: &Entity<Editor>,
) {
    // SilentClock uses a real callback thread. Rendering additional frames
    // alone cannot acknowledge the exact installed chain/timeline receipts.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        frames(window, cx);
        let (ready, diagnostics) = window.update(cx, |_, _, cx| {
            editor.update(cx, |ed, cx| {
                let ready = ed.native_insert_transport_ready(cx);
                (ready, transport_diagnostics(ed))
            })
        }).unwrap();
        if ready { return; }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        assert!(!remaining.is_zero(), "fixture transport did not become ready: {diagnostics}");
        // Yield only between checks of the actual production readiness
        // predicate; neither a fixed frame count nor clearing its guard.
        std::thread::park_timeout(remaining.min(std::time::Duration::from_millis(1)));
    }
}
