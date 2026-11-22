//! Real GPUI/editor fixtures with an isolated durable project. No worker,
//! plugin process, preference persistence or hardware audio is started.

use std::path::PathBuf;
use gpui::{AppContext, Render, TestAppContext, WindowHandle};
use verdant_domain::{Project, ProjectPersistence, Track, TrackId, TRACK_COLORS};

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
