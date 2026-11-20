use super::{AppearancePreference, PridePreference, Theme};
use crate::files::Prefs;
use chrono::NaiveDate;
use gpui::{Rgba, WindowAppearance};

fn luminance(color: Rgba) -> f64 {
    let linear = |component: f32| {
        let c = f64::from(component);
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn every_paint_token_resolves_in_both_appearances() {
    for theme in [Theme::dark(), Theme::light()] {
        // Exhaustive on purpose: a new paint token must also be checked here.
        let Theme {
            dark: _, pride: _, window, surface, headerbar, view, card, popover, shade, hover, checked,
            control, control_pressed, separator,
            text, text_secondary, text_tertiary, accent, accent_text, selection,
            success, error, warning, roll_white, roll_black, grid_bar, grid_beat,
            grid_sub, key_white, key_black, playhead, lcd, meter_low, meter_high,
        } = theme;
        for color in [
            window, surface, headerbar, view, card, popover, shade, hover, checked, control, control_pressed, separator,
            text, text_secondary, text_tertiary, accent, accent_text, selection,
            success, error, warning, roll_white, roll_black, grid_bar, grid_beat,
            grid_sub, key_white, key_black, playhead, lcd, meter_low, meter_high,
        ] {
            for channel in [color.r, color.g, color.b, color.a] {
                assert!(channel.is_finite() && (0.0..=1.0).contains(&channel));
            }
            assert!(color.a > 0.0, "unresolved transparent token in {theme:?}");
        }
        assert_eq!(meter_low, success);
        assert_eq!(meter_high, warning);
        for background in [window, surface, headerbar, view, card, popover, control, control_pressed, lcd, roll_white, roll_black] {
            for foreground in [text, text_secondary, text_tertiary] {
                assert!(contrast(foreground, background) >= 4.5,
                    "unreadable foreground {foreground:?} on {background:?}");
            }
        }
        assert!(contrast(accent_text, accent) >= 4.5);
        // Checked toggles always carry primary text.
        assert!(contrast(text, checked) >= 4.5, "unreadable checked toggle");
        for status in [success, warning, error] {
            for background in [surface, headerbar, card] {
                assert!(contrast(status, background) >= 4.5, "unreadable status {status:?} on {background:?}");
            }
        }
        assert!(contrast(separator, surface) >= 3.0);
        for background in [roll_white, roll_black] {
            assert!(contrast(playhead, background) >= 3.0);
        }
        let tracks: Vec<_> = verdant_domain::TRACK_COLORS.iter().map(|name| theme.track(Some(name))).collect();
        for (i, color) in tracks.iter().enumerate() {
            assert!(contrast(theme.on_track(), *color) >= 4.5, "unreadable track label: {color:?}");
            for other in &tracks[..i] {
                let distance = ((color.r - other.r).powi(2) + (color.g - other.g).powi(2) + (color.b - other.b).powi(2)).sqrt();
                assert!(distance >= 0.12, "track swatches are too similar: {color:?}, {other:?}");
            }
        }
        assert_eq!(theme.track(None), theme.track(Some("graphite")));
        assert_eq!(theme.track(Some("unknown")), theme.track(Some("graphite")));
    }
}

#[test]
fn persisted_preference_survives_disk_reload() {
    // Use the same flattened Prefs JSON format as save_prefs/load_prefs, without
    // changing process-wide environment variables or a real user's state dir.
    let dir = std::env::temp_dir().join(format!("verdant-appearance-{}", verdant_domain::NoteId::generate()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("prefs.json");
    let mut prefs = Prefs::default();
    prefs.extra.insert("unrelated_setting".into(), serde_json::json!({"keep": true}));
    for preference in [AppearancePreference::Dark, AppearancePreference::Light, AppearancePreference::System] {
        assert!(preference.store(&mut prefs));
        assert!(!preference.store(&mut prefs));
        std::fs::write(&path, serde_json::to_vec_pretty(&prefs).unwrap()).unwrap();
        let restored: Prefs = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(AppearancePreference::from_prefs(&restored), preference);
        assert_eq!(restored.extra["unrelated_setting"], serde_json::json!({"keep": true}));
        assert_eq!(Theme::resolve(WindowAppearance::Dark, None, &restored), Theme::resolve(WindowAppearance::Dark, None, &prefs));
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn toggle_changes_tokens_without_mutating_the_document() {
    let document = verdant_domain::ProjectStore::new(verdant_domain::Project::new_empty());
    let before = document.snapshot();
    let state = (document.revision(), document.content_digest(), document.undo_depth(), document.redo_depth(), document.is_dirty());
    let mut prefs = Prefs::default();
    PridePreference::Off.store(&mut prefs);
    let mut session_override = Some(true);
    let dark = Theme::resolve(WindowAppearance::Light, session_override, &prefs);
    // This is the actual transition called by Editor::toggle_appearance. It can
    // only borrow preferences and the CLI override, never the musical document.
    let light = Theme::toggle(WindowAppearance::Light, &mut session_override, &mut prefs);
    assert_eq!(session_override, None);
    assert_eq!(AppearancePreference::from_prefs(&prefs), AppearancePreference::Light);
    assert!(!light.dark);
    assert_ne!(dark.surface, light.surface);
    assert_ne!(dark.text, light.text);
    // libadwaita keeps one accent blue in both appearances; what must hold is
    // that it stays readable on each.
    assert_eq!(dark.accent, light.accent);
    assert!(contrast(dark.accent_text, dark.accent) >= 4.5 && contrast(light.accent_text, light.accent) >= 4.5);
    assert_ne!(dark.headerbar, light.headerbar);
    for name in verdant_domain::TRACK_COLORS {
        assert_ne!(dark.track(Some(name)), light.track(Some(name)));
    }
    assert_eq!(Theme::resolve(WindowAppearance::Dark, None, &prefs), light);
    assert_eq!(Theme::toggle(WindowAppearance::Light, &mut session_override, &mut prefs), dark);
    assert!(std::sync::Arc::ptr_eq(&before, &document.snapshot()));
    assert_eq!(state, (document.revision(), document.content_digest(), document.undo_depth(), document.redo_depth(), document.is_dirty()));
}

#[test]
fn appearance_precedence_preserves_session_override_and_system_fallback() {
    let mut prefs = Prefs::default();
    PridePreference::Off.store(&mut prefs);
    assert_eq!(AppearancePreference::from_prefs(&prefs), AppearancePreference::System);
    for platform in [WindowAppearance::Light, WindowAppearance::VibrantLight] {
        assert_eq!(Theme::resolve(platform, None, &prefs), Theme::light());
    }
    for platform in [WindowAppearance::Dark, WindowAppearance::VibrantDark] {
        assert_eq!(Theme::resolve(platform, None, &prefs), Theme::dark());
    }
    AppearancePreference::Dark.store(&mut prefs);
    assert_eq!(Theme::resolve(WindowAppearance::Light, None, &prefs), Theme::dark());
    assert_eq!(Theme::resolve(WindowAppearance::Dark, Some(false), &prefs), Theme::light());
    assert_eq!(AppearancePreference::from_prefs(&prefs), AppearancePreference::Dark);
    AppearancePreference::Light.store(&mut prefs);
    assert_eq!(Theme::resolve(WindowAppearance::Dark, None, &prefs), Theme::light());
    assert_eq!(Theme::resolve(WindowAppearance::Light, Some(true), &prefs), Theme::dark());
    prefs.extra.insert("appearance".into(), serde_json::json!("future-setting"));
    assert_eq!(Theme::resolve(WindowAppearance::Light, None, &prefs), Theme::light());
    assert_eq!(Theme::resolve(WindowAppearance::Dark, None, &prefs), Theme::dark());
}

#[test]
fn pride_calendar_uses_only_the_three_local_dates_in_every_year() {
    for year in [2024, 2026] {
        let mut date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let mut active = Vec::new();
        loop {
            if PridePreference::Auto.active_on(date) {
                active.push(date);
            }
            assert!(PridePreference::On.active_on(date));
            assert!(!PridePreference::Off.active_on(date));
            if date == NaiveDate::from_ymd_opt(year, 12, 31).unwrap() { break; }
            date = date.succ_opt().unwrap();
        }
        assert_eq!(active, [(5, 17), (6, 28), (10, 11)]
            .map(|(month, day)| NaiveDate::from_ymd_opt(year, month, day).unwrap()));
    }
}

#[test]
fn pride_preference_is_independent_of_appearance_and_session_override() {
    for platform in [WindowAppearance::Light, WindowAppearance::VibrantLight, WindowAppearance::Dark, WindowAppearance::VibrantDark] {
        for appearance in [AppearancePreference::System, AppearancePreference::Dark, AppearancePreference::Light] {
            for session in [None, Some(true), Some(false)] {
                for (month, day, automatic) in [(5, 17, true), (6, 28, true), (10, 11, true), (11, 19, false)] {
                    let date = NaiveDate::from_ymd_opt(2026, month, day).unwrap();
                    let mut prefs = Prefs::default();
                    appearance.store(&mut prefs);
                    PridePreference::Off.store(&mut prefs);
                    let baseline = Theme::resolve_on_date(platform, session, &prefs, date);
                    for (preference, expected) in [(PridePreference::Auto, automatic), (PridePreference::On, true), (PridePreference::Off, false)] {
                        preference.store(&mut prefs);
                        assert_eq!(Theme::resolve_on_date(platform, session, &prefs, date), baseline.with_pride(expected));
                    }
                }
            }
        }
    }
}

#[test]
fn pride_reading_and_calendar_resolution_never_write_preferences() {
    let date = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
    for value in [None, Some(serde_json::json!("future-setting")), Some(serde_json::json!(17))] {
        let mut prefs = Prefs::default();
        prefs.extra.insert("unrelated_setting".into(), serde_json::json!({"keep": true}));
        if let Some(value) = value { prefs.extra.insert("pride_theme".into(), value); }
        let before = serde_json::to_value(&prefs).unwrap();
        assert_eq!(PridePreference::from_prefs(&prefs), PridePreference::Auto);
        assert!(Theme::resolve_on_date(WindowAppearance::Dark, None, &prefs, date).pride);
        assert_eq!(serde_json::to_value(&prefs).unwrap(), before);
        for preference in [PridePreference::On, PridePreference::Off, PridePreference::Auto] {
            assert!(preference.store(&mut prefs));
            assert!(!preference.store(&mut prefs));
            let restored: Prefs = serde_json::from_value(serde_json::to_value(&prefs).unwrap()).unwrap();
            assert_eq!(PridePreference::from_prefs(&restored), preference);
            let mut expected = before.clone();
            expected["pride_theme"] = serde_json::json!(match preference {
                PridePreference::Auto => "auto", PridePreference::On => "on", PridePreference::Off => "off",
            });
            assert_eq!(serde_json::to_value(&restored).unwrap(), expected);
        }
    }
}

#[test]
fn pride_accent_has_accessible_contrast_and_disabling_restores_every_token() {
    for baseline in [Theme::dark(), Theme::light()] {
        let active = baseline.with_pride(true);
        assert!(active.pride);
        assert!(contrast(active.accent_text, active.accent) >= 4.5);
        for background in [active.window, active.surface, active.control, active.control_pressed, active.lcd, active.roll_white, active.roll_black] {
            assert!(contrast(active.accent, background) >= 3.0, "unreadable focus accent on {background:?}");
            let selection = Rgba {
                r: active.selection.r * active.selection.a + background.r * (1.0 - active.selection.a),
                g: active.selection.g * active.selection.a + background.g * (1.0 - active.selection.a),
                b: active.selection.b * active.selection.a + background.b * (1.0 - active.selection.a),
                a: 1.0,
            };
            assert!(contrast(active.text, selection) >= 4.5);
        }
        // Everything except the explicit accent/selection overlay stays neutral,
        // including status, meters, playhead and the separate track palette.
        assert_eq!(Theme {
            pride: false, accent: baseline.accent, accent_text: baseline.accent_text,
            selection: baseline.selection, ..active
        }, baseline);
        for name in verdant_domain::TRACK_COLORS {
            assert_eq!(active.track(Some(name)), baseline.track(Some(name)));
        }
        assert_eq!(active.with_pride(false), baseline);
        assert_eq!(active.with_pride(false).with_pride(true), active);
    }
}

#[test]
fn pride_idle_date_transition_restores_baseline_without_persisting_auto() {
    let prefs = Prefs::default();
    let before = serde_json::to_value(&prefs).unwrap();
    let eve = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
    let event = eve.succ_opt().unwrap();
    let after = event.succ_opt().unwrap();
    for platform in [WindowAppearance::Dark, WindowAppearance::Light] {
        let baseline = Theme::resolve_on_date(platform, None, &prefs, eve);
        let active = baseline.with_pride(PridePreference::from_prefs(&prefs).active_on(event));
        assert_eq!(active, Theme::resolve_on_date(platform, None, &prefs, event));
        assert!(active.pride);
        assert_eq!(active.with_pride(PridePreference::from_prefs(&prefs).active_on(after)), baseline);
    }
    assert_eq!(serde_json::to_value(&prefs).unwrap(), before);
    assert!(!prefs.extra.contains_key("pride_theme"));
}
