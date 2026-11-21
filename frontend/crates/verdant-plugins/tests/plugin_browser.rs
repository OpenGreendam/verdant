use verdant_plugins::insert::{parameter_editor_default, parameter_editor_value, PluginBrowser};
use verdant_plugins::{ParamDesc, PluginClient, PluginClientOpts, PluginDesc, PluginFormat};

fn plugin(format: PluginFormat, id: &str, name: &str, vendor: &str) -> PluginDesc {
    PluginDesc { format, id: id.into(), name: name.into(), vendor: vendor.into(),
        version: "1.2".into(), path: format!("/effects/{name}"), has_editor: false,
        features: vec!["audio-effect".into()] }
}

#[test]
fn browser_preserves_host_rows_and_inserts_stable_identity_at_requested_slot() {
    // A multi-plugin bundle and a format-local id collision must retain all rows.
    let catalog = vec![plugin(PluginFormat::Clap, "shared", "Warm Room", "North"),
        plugin(PluginFormat::Clap, "second", "Bright Room", "North"),
        plugin(PluginFormat::Vst3, "shared", "Limiter", "South")];
    let mut browser = PluginBrowser::new(catalog.clone());
    let rows = browser.entries();
    assert_eq!(rows.len(), catalog.len());
    for (row, reported) in rows.iter().zip(&catalog) {
        assert_eq!((row.format, &row.id, &row.name, &row.vendor, &row.version, &row.path),
            (reported.format, &reported.id, &reported.name, &reported.vendor, &reported.version, &reported.path));
    }
    browser.query = "  NORTH ".into();
    assert_eq!(browser.entries().len(), 2);
    browser.format = Some(PluginFormat::Vst3);
    assert!(browser.entries().is_empty());
    browser.query = "limiter".into();
    assert_eq!(browser.entries()[0].id, "shared");

    let mut specs = vec![];
    let first = browser.insert(&mut specs, 0, "clap", "shared").unwrap();
    let second = browser.insert(&mut specs, 0, "vst3", "shared").unwrap();
    assert_ne!(first, second);
    assert!(!first.is_empty());
    assert_eq!(specs[0].format, "vst3");
    assert_eq!(specs[1].instance_id, first);
    let saved = serde_json::to_vec(&specs).unwrap();
    let restored: Vec<verdant_plugins::InsertSpec> = serde_json::from_slice(&saved).unwrap();
    assert_eq!(restored[0].instance_id, second);
    assert_eq!(restored[1].instance_id, first);
    let before = specs.clone();
    assert!(browser.insert(&mut specs, 3, "clap", "shared").is_err());
    assert!(browser.insert(&mut specs, 0, "au", "shared").is_err());
    assert_eq!(specs, before);
    browser.refresh(vec![]);
    browser.query.clear();
    browser.format = None;
    assert!(browser.entries().is_empty(), "a removed host row must disappear");
}

#[test]
fn generic_editor_uses_native_nonzero_range_and_discrete_steps() {
    let desc = ParamDesc { id: "mode".into(), name: "Mode".into(), value: -2.5,
        min: -7.5, max: 12.5, step_count: 4, default_value: Some(-2.5), unit: String::new() };
    assert_eq!(parameter_editor_value(&desc, -100.0).unwrap(), -7.5);
    assert_eq!(parameter_editor_value(&desc, 100.0).unwrap(), 12.5);
    assert_eq!(parameter_editor_value(&desc, 1.9).unwrap(), 2.5);
    assert_eq!(parameter_editor_value(&desc, -4.9).unwrap(), -2.5);
    let continuous = ParamDesc { step_count: 0, ..desc.clone() };
    assert_eq!(parameter_editor_value(&continuous, 1.9).unwrap(), 1.9);
    let fixed = ParamDesc { min: 3.0, max: 3.0, value: 3.0, ..desc.clone() };
    assert_eq!(parameter_editor_value(&fixed, 100.0).unwrap(), 3.0);
    assert!(parameter_editor_value(&desc, f64::NAN).is_err());
    assert!(parameter_editor_value(&desc, f64::INFINITY).is_err());
    assert!(parameter_editor_value(&ParamDesc { min: 20.0, ..desc.clone() }, 1.0).is_err());
    assert!(parameter_editor_value(&ParamDesc { step_count: -1, ..desc }, 1.0).is_err());
}

#[test]
fn generic_editor_default_is_separate_from_current_value() {
    let mut reported = serde_json::json!({ "id": "mode", "name": "Mode", "value": 12.5,
        "min": -7.5, "max": 12.5, "step_count": 4, "default": -2.5 });
    let desc: ParamDesc = serde_json::from_value(reported.clone()).unwrap();
    assert_eq!(desc.value, 12.5);
    assert_eq!(parameter_editor_default(&desc).unwrap(), -2.5);
    reported.as_object_mut().unwrap().remove("default");
    let desc: ParamDesc = serde_json::from_value(reported).unwrap();
    assert!(parameter_editor_default(&desc).is_err(), "an omitted default must not invent zero or use the current value");
}

/// Run after the parent builds the real host, with VERDANT_PLUGIN_HOST and
/// VERDANT_TEST_PLUGIN pointing at a native CLAP/VST3/AUv2 effect bundle.
/// No fake host process or fabricated protocol reply is used by this test.
#[test]
#[ignore = "requires a built verdant-plughost and a real native effect bundle"]
fn browser_matches_native_host_report_and_inserted_handle_identity() {
    let host = std::path::PathBuf::from(std::env::var_os("VERDANT_PLUGIN_HOST").expect("VERDANT_PLUGIN_HOST"));
    let module = std::path::PathBuf::from(std::env::var_os("VERDANT_TEST_PLUGIN").expect("VERDANT_TEST_PLUGIN"));
    let cache = std::env::temp_dir().join(format!("verdant-browser-{}", uuid::Uuid::new_v4()));
    let client = PluginClient::with_plughost(PluginClientOpts { plughost_path: Some(host.clone()),
        cache_dir: cache, sample_rate: 48_000.0, max_frames: 8192 }, host).unwrap();
    let report = client.scan_file(&module).unwrap();
    assert_eq!(report.error, None);
    assert!(!report.plugins.is_empty());
    let mut browser = PluginBrowser::new(client.catalog());
    // The catalog includes each of Verdant's built-in effects exactly once.
    let builtin_ids: Vec<&str> = std::iter::once(verdant_plugins::BUILTIN_GAIN_ID)
        .chain(verdant_plugins::builtin_dsp::EFFECT_IDS).collect();
    assert_eq!(browser.entries().len(), report.plugins.len() + builtin_ids.len());
    for id in builtin_ids {
        assert_eq!(browser.entries().iter().filter(|row| row.format == PluginFormat::Builtin
            && row.id == id).count(), 1, "expected exactly one built-in effect {id}");
    }
    for reported in &report.plugins {
        let rows = browser.entries();
        let row = rows.iter().find(|row| row.id == reported.id && row.format.tag() == reported.format).unwrap();
        assert_eq!((&row.name, &row.vendor, &row.version, row.has_editor, &row.features),
            (&reported.name, &reported.vendor, &reported.version, reported.has_editor, &reported.features));
    }
    let desc = &report.plugins[0];
    browser.format = Some(PluginFormat::from_tag(&desc.format));
    let mut specs = vec![];
    let instance = browser.insert(&mut specs, 0, &desc.format, &desc.id).unwrap();
    let handle = client.insert(specs[0].clone()).unwrap();
    assert_eq!(handle.spec().instance_id, instance);
    let key = handle.key();
    let descriptors = handle.params().unwrap();
    assert!(!descriptors.is_empty(), "select a native effect with generic parameters for this acceptance check");
    for param in descriptors {
        assert!(param.default_value.is_some(), "native host omitted the declared default for {}", param.id);
        let default = parameter_editor_default(&param).unwrap();
        assert!((param.min..=param.max).contains(&default));
        assert!(parameter_editor_value(&param, param.min - 1.0).unwrap() >= param.min);
        assert!(parameter_editor_value(&param, param.max + 1.0).unwrap() <= param.max);
        if param.step_count > 0 && param.max > param.min {
            let value = parameter_editor_value(&param, param.min + (param.max - param.min) * 0.37).unwrap();
            let step = (value - param.min) / (param.max - param.min) * param.step_count as f64;
            assert!((step - step.round()).abs() <= 8.0 * f64::EPSILON * step.abs().max(1.0));
        }
    }
    client.remove(key);
    drop(handle);
    let restored = client.restore(&specs[0]);
    match restored {
        verdant_plugins::InsertSlot::Live(handle) => {
            assert_eq!(handle.spec().instance_id, instance);
            assert_ne!(handle.key(), key);
            client.remove(handle.key());
        }
        verdant_plugins::InsertSlot::Missing { reason, .. } => panic!("native restore failed: {reason}"),
    }
}
