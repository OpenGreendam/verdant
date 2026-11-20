use super::{Key, Locale, Surface};
use crate::files::Prefs;

#[test]
fn every_catalogue_key_has_both_translations() {
    for &key in Key::ALL {
        for locale in Locale::ALL {
            assert!(!locale.text(key).trim().is_empty(), "missing {} translation for {key:?}", locale.code());
        }
    }
}

#[test]
fn locale_switch_changes_resolved_strings_and_survives_preferences_roundtrip() {
    let mut prefs: Prefs = serde_json::from_value(serde_json::json!({
        "device": "selected audio device",
        "audio_export_options": { "sample_rate": 48000 },
        "future_preference": [1, 2, 3]
    })).unwrap();
    let original = prefs.extra.clone();
    assert_eq!(Locale::from_preferences(&prefs).text(Key::Roll), "钢琴卷帘");
    Locale::En.write_preference(&mut prefs);
    let encoded = serde_json::to_vec(&prefs).unwrap();
    let mut reopened: Prefs = serde_json::from_slice(&encoded).unwrap();
    let locale = Locale::from_preferences(&reopened);
    assert_eq!(locale, Locale::En);
    assert_eq!(locale.text(Key::Roll), "Piano roll");
    assert_eq!(locale.text(Key::DisclosuresTitle), "Open-source software and research references");
    assert_eq!(reopened.device.as_deref(), Some("selected audio device"));
    for (key, value) in original { assert_eq!(reopened.extra.get(&key), Some(&value)); }
    Locale::ZhCn.write_preference(&mut reopened);
    assert_eq!(Locale::from_preferences(&reopened).text(Key::Roll), "钢琴卷帘");
}

#[test]
fn every_major_surface_has_localized_accessible_name_and_role() {
    for surface in Surface::ALL {
        for locale in Locale::ALL {
            let semantics = surface.semantics(locale);
            assert!(!semantics.label.trim().is_empty(), "unnamed {surface:?} in {}", locale.code());
            assert!(!semantics.description.trim().is_empty(), "undescribed {surface:?} in {}", locale.code());
            assert_ne!(surface.semantics(Locale::ZhCn).label, surface.semantics(Locale::En).label);
        }
    }
    assert_eq!(Surface::Roll.semantics(Locale::En).role, gpui::Role::Group);
    assert_eq!(Surface::Lanes.semantics(Locale::En).role, gpui::Role::Group);
    assert_eq!(Surface::Mixer.semantics(Locale::En).role, gpui::Role::Group);
    assert_eq!(Surface::Dialog.semantics(Locale::En).role, gpui::Role::Dialog);
}

#[test]
fn older_and_unknown_locale_preferences_keep_the_chinese_default() {
    for value in [serde_json::Value::Null, serde_json::json!("future-locale"), serde_json::json!(42)] {
        let prefs: Prefs = serde_json::from_value(serde_json::json!({"ui_locale": value})).unwrap();
        assert_eq!(Locale::from_preferences(&prefs), Locale::ZhCn);
    }
}
