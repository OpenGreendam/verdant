//! Project-wide audibility for the realtime mixer. A solo track still mutes
//! other tracks when it is empty or its render has not become playable yet.
//!
//! Mixer slots follow `Project.tracks` order, never TrackId creation order.
//! A MoveTrack commit uses the existing topology hold before `sync_mixer`;
//! phrase clips, imported clips, automation and retained insert chains are
//! rebuilt against the new order before that hold releases playback.
pub use verdant_service::project_audio::{MASTER_BUS_KEY, MasterBusSettings, channel_settings};

#[cfg(test)]
#[path = "track_order_tests.rs"]
mod track_order_tests;

#[cfg(test)]
mod master_bus_tests {
    use super::*;
    use verdant_domain::Project;

    #[test]
    fn default_master_bus_omits_extension_key() {
        let project = Project::new_empty();
        let settings = MasterBusSettings::from_project(&project);
        assert!(settings.is_default());
        assert_eq!(settings.extension_value(), None);
    }

    #[test]
    fn master_bus_clamps_and_roundtrips_through_project_extensions() {
        let mut project = Project::new_empty();
        let raw = MasterBusSettings { gain_millidecibels: 9_500, pan_permille: -1_800, muted: true };
        let val = raw.extension_value().expect("non-default");
        project.extensions.insert(MASTER_BUS_KEY.to_string(), val);

        let loaded = MasterBusSettings::from_project(&project);
        assert_eq!(loaded.gain_millidecibels, 6_000);
        assert_eq!(loaded.pan_permille, -1_000);
        assert!(loaded.muted);

        let ch = loaded.channel_settings();
        assert!(ch.mute);
        assert!((ch.pan + 1.0).abs() < 1e-6);
        assert!((ch.gain - 10f32.powf(6.0 / 20.0)).abs() < 1e-4);
    }
}
