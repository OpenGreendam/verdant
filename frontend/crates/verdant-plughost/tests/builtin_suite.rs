//! Exercise the public host-format boundary with the real built-in processors.
//! These tests require neither a third-party plugin nor a graphical display;
//! they do not stand in for child-process transport or native-window acceptance.

use std::collections::BTreeSet;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::mpsc;
use verdant_plughost::formats::{self, HostedPlugin};
use verdant_plugins::builtin_dsp as dsp;
use verdant_plugins::{ParamDesc, SAMPLE_RATE};

thread_local! {
    static MEASURING_AUDIO: Cell<bool> = const { Cell::new(false) };
    static AUDIO_ALLOCATIONS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

struct AudioAllocator;

unsafe impl GlobalAlloc for AudioAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if MEASURING_AUDIO.try_with(Cell::get).unwrap_or(false) {
            let _ = AUDIO_ALLOCATIONS.try_with(|counter| { let (allocations, frees) = counter.get(); counter.set((allocations + 1, frees)); });
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if MEASURING_AUDIO.try_with(Cell::get).unwrap_or(false) {
            let _ = AUDIO_ALLOCATIONS.try_with(|counter| { let (allocations, frees) = counter.get(); counter.set((allocations, frees + 1)); });
        }
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: AudioAllocator = AudioAllocator;

fn measured_audio<T>(call: impl FnOnce() -> T) -> (T, (usize, usize)) {
    struct EndMeasurement;
    impl Drop for EndMeasurement {
        fn drop(&mut self) { MEASURING_AUDIO.with(|active| active.set(false)); }
    }
    AUDIO_ALLOCATIONS.with(|counter| counter.set((0, 0)));
    MEASURING_AUDIO.with(|active| active.set(true));
    let guard = EndMeasurement;
    let result = call();
    drop(guard);
    (result, AUDIO_ALLOCATIONS.with(Cell::get))
}

fn hosted(id: &str) -> Box<dyn HostedPlugin> {
    let (events, _receiver) = mpsc::channel();
    let (plugin, descriptor) =
        formats::instantiate("builtin", id, "", SAMPLE_RATE, 256, events).unwrap();
    assert_eq!(descriptor.id, id);
    assert_eq!(Some(descriptor.name.as_str()), dsp::display_name(id));
    plugin
}

fn parameter(plugin: &mut dyn HostedPlugin, id: &str) -> ParamDesc {
    plugin.params().unwrap().into_iter().find(|parameter| parameter.id == id).unwrap()
}

fn process(plugin: &mut dyn HostedPlugin, input: &[f32], key: Option<&[f32]>) -> Vec<f32> {
    let mut output_l = vec![0.0; input.len()];
    let mut output_r = output_l.clone();
    // Use the same quantum as a host session, including a possible short tail.
    for start in (0..input.len()).step_by(256) {
        let end = (start + 256).min(input.len());
        match key {
            Some(key) => plugin.process_keyed(
                &input[start..end], &input[start..end], (&key[start..end], &key[start..end]),
                &mut output_l[start..end], &mut output_r[start..end], &[],
            ),
            None => plugin.process(
                &input[start..end], &input[start..end], &mut output_l[start..end], &mut output_r[start..end],
            ),
        }.unwrap();
    }
    assert!(output_l.iter().chain(&output_r).all(|sample| sample.is_finite()));
    output_l
}

#[test]
fn complete_catalog_instantiates_and_restores_real_parameter_state() {
    let catalog = formats::builtin::effect_descriptors();
    let expected: BTreeSet<_> = dsp::EFFECT_IDS.into_iter().collect();
    let actual: BTreeSet<_> = catalog.iter().map(|descriptor| descriptor.id.as_str()).collect();
    assert_eq!(actual, expected);
    assert_eq!(catalog.len(), expected.len(), "duplicate browser entries");

    for descriptor in catalog {
        let mut plugin = hosted(&descriptor.id);
        // Built-ins are edited by the app's native surface. The child must
        // not fabricate an external GUI capability to obtain that surface.
        assert!(!descriptor.has_editor);
        assert!(!plugin.has_editor());
        assert_eq!(dsp::preferred_editor("builtin", &descriptor.id, false), dsp::EditorKind::Builtin);
        assert_eq!(dsp::preferred_editor("clap", &descriptor.id, false), dsp::EditorKind::Parameters);

        let defaults = plugin.params().unwrap();
        let ids: BTreeSet<_> = defaults.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids.len(), defaults.len(), "duplicate parameters for {}", descriptor.id);
        let changed = defaults.iter().find(|p| p.max > p.min).unwrap();
        let value = if changed.value == changed.min { changed.max } else { changed.min };
        plugin.set_param(&changed.id, value).unwrap();
        let snapshot = plugin.state().unwrap();
        let mut restored = hosted(&descriptor.id);
        restored.set_state(&snapshot).unwrap();
        assert_eq!(restored.params().unwrap(), plugin.params().unwrap(), "{}", descriptor.id);

        restored.set_offline(true).unwrap();
        restored.reconfigure(SAMPLE_RATE, 256).unwrap();
        let input: Vec<f32> = (0..513).map(|n| (n as f32 * 0.17).sin() * 0.1).collect();
        let output = process(&mut *restored, &input, None);
        assert_eq!(output.len(), 513, "short tail remains a real block");
    }
}

#[test]
fn all_effects_reject_invalid_state_and_event_batches_before_mutation() {
    for id in dsp::EFFECT_IDS {
        let mut plugin = hosted(id);
        let before = plugin.params().unwrap();
        let first = &before[0];
        let valid = if first.value == first.min { first.max } else { first.min };
        let bad_state = serde_json::json!({(first.id.clone()): valid, "unknown.parameter": 1.0});
        assert!(plugin.set_state(&serde_json::to_vec(&bad_state).unwrap()).is_err(), "{id}");
        assert_eq!(plugin.params().unwrap(), before, "rejected state mutated {id}");

        for invalid in [(u32::MAX, 4, 0.0), (0, 4, first.max + 1.0), (0, 4, f64::NAN), (0, 8, valid)] {
            let mut output_l = [7.0; 8];
            let mut output_r = [8.0; 8];
            assert!(plugin.process_events(&[0.1; 8], &[-0.2; 8], &mut output_l, &mut output_r,
                &[(0, 0, valid), invalid]).is_err(), "{id}");
            assert_eq!(plugin.params().unwrap(), before, "rejected events mutated {id}");
            assert_eq!(output_l, [7.0; 8]);
            assert_eq!(output_r, [8.0; 8]);
        }
        let mut output_l = [7.0; 8];
        let mut output_r = [8.0; 8];
        assert!(plugin.process_keyed(&[0.1; 8], &[-0.2; 8], (&[0.5; 7], &[0.5; 8]),
            &mut output_l, &mut output_r, &[]).is_err(), "{id}");
        assert_eq!(output_l, [7.0; 8]);
        assert_eq!(output_r, [8.0; 8]);
    }
}

#[test]
fn new_effects_apply_admitted_automation_at_exact_offsets() {
    for id in [dsp::MULTIBAND_ID, dsp::FILTER_ID, dsp::SIMPLON_ID, dsp::MICRO_ID] {
        let mut plugin = hosted(id);
        plugin.set_param("mix", 0.0).unwrap();
        let target = plugin.automation_params().unwrap().into_iter()
            .find(|parameter| parameter.id == "output").unwrap().target;
        let events = [target.event(2, -12.0).unwrap(), target.event(5, 6.0).unwrap(),
            target.event(5, -6.0).unwrap(), target.event(7, 0.0).unwrap()];
        let input_l = [0.125; 8];
        let input_r = [-0.25; 8];
        let mut output_l = [0.0; 8];
        let mut output_r = [0.0; 8];
        plugin.process_events(&input_l, &input_r, &mut output_l, &mut output_r, &events).unwrap();
        for sample in 0..8 {
            let db = if sample < 2 { 0.0 } else if sample < 5 { -12.0 } else if sample < 7 { -6.0 } else { 0.0 };
            let gain = dsp::db_to_lin(db);
            assert_eq!(output_l[sample], (input_l[sample] as f64 * gain) as f32, "{id}, sample {sample}");
            assert_eq!(output_r[sample], (input_r[sample] as f64 * gain) as f32, "{id}, sample {sample}");
        }
        assert_eq!(parameter(&mut *plugin, "output").value, 0.0);
    }
}

#[test]
fn new_processors_receive_external_keys_without_leaking_key_audio() {
    let input: Vec<f32> = (0..12_037).map(|n| (std::f64::consts::TAU * 4000.0 * n as f64 / SAMPLE_RATE).sin() as f32 * 0.01).collect();
    let key = vec![0.8; input.len()];
    let energy = |output: &[f32]| output[6000..].iter().map(|x| (*x as f64).powi(2)).sum::<f64>();
    for id in [dsp::FILTER_ID, dsp::SIMPLON_ID, dsp::MICRO_ID] {
        let mut own_key = hosted(id);
        let mut external_key = hosted(id);
        for plugin in [&mut own_key, &mut external_key] {
            plugin.set_param("env_depth", 48.0).unwrap();
            plugin.set_param("env_attack", 0.1).unwrap();
            plugin.set_param(if id == dsp::MICRO_ID { "freq" } else { "f1.freq" }, 500.0).unwrap();
            if id == dsp::SIMPLON_ID { plugin.set_param("f2.enabled", 0.0).unwrap(); }
        }
        let baseline = process(&mut *own_key, &input, None);
        let modulated = process(&mut *external_key, &input, Some(&key));
        assert!(energy(&modulated) > energy(&baseline) * 8.0, "external key must open {id}'s filter");
        let mut silent = hosted(id);
        silent.set_param("env_depth", 48.0).unwrap();
        assert!(process(&mut *silent, &vec![0.0; input.len()], Some(&key)).iter().all(|x| *x == 0.0), "{id} leaked its key");
    }

    let mut multiband = hosted(dsp::MULTIBAND_ID);
    for (id, value) in [("band_count", 1.0), ("b1.range", -24.0), ("b1.threshold", -24.0),
        ("b1.ratio", 20.0), ("b1.attack", 0.1)] {
        multiband.set_param(id, value).unwrap();
    }
    let compressed = process(&mut *multiband, &input, Some(&key));
    assert!(energy(&compressed) < energy(&input) * 0.02, "multiband must follow the external detector");
    let mut silent = hosted(dsp::MULTIBAND_ID);
    silent.set_param("b1.range", -24.0).unwrap();
    assert!(process(&mut *silent, &vec![0.0; input.len()], Some(&key)).iter().all(|x| *x == 0.0));
}

#[test]
fn legacy_eq_state_and_extended_band_state_share_the_stable_adapter() {
    let mut eq = hosted(dsp::EQ_ID);
    eq.set_state(br#"{"b1_on":0,"b4_gain":3,"b6_freq":12000,"output":-2}"#).unwrap();
    assert_eq!(parameter(&mut *eq, "b4_gain").value, 3.0);
    assert_eq!(parameter(&mut *eq, "b24_on").value, 0.0);
    assert_eq!(parameter(&mut *eq, "b24_dynamic").value, 0.0);
    for (id, value) in [("b24_on", 1.0), ("b24_freq", 6000.0), ("b24_gain", -3.0),
        ("b24_dynamic", 1.0), ("b24_dyn_range", -4.0), ("solo", 24.0)] {
        eq.set_param(id, value).unwrap();
    }
    let automation = eq.automation_params().unwrap();
    assert_eq!(automation.iter().find(|p| p.id == "output").unwrap().target.native_id(), 24);
    assert_eq!(automation.iter().find(|p| p.id == "b4_gain").unwrap().target.native_id(), 14);
    let mut restored = hosted(dsp::EQ_ID);
    restored.set_state(&eq.state().unwrap()).unwrap();
    assert_eq!(restored.params().unwrap(), eq.params().unwrap());
}

#[test]
fn lookahead_remains_control_only_and_reports_real_processing_latency() {
    for id in [dsp::COMPRESSOR_ID, dsp::GATE_ID, dsp::DEESSER_ID, dsp::LIMITER_ID] {
        let mut plugin = hosted(id);
        assert!(!plugin.automation_params().unwrap().iter().any(|p| p.id == "lookahead"), "{id}");
        plugin.set_param("lookahead", 5.0).unwrap();
        assert_eq!(plugin.latency(), 240, "{id}");
        let state = plugin.state().unwrap();
        let mut restored = hosted(id);
        restored.set_state(&state).unwrap();
        assert_eq!(restored.latency(), 240, "{id}");
    }
}

#[test]
fn legacy_effect_parameter_positions_and_partial_states_remain_compatible() {
    let legacy: &[(&str, &[&str])] = &[
        (dsp::COMPRESSOR_ID, &["threshold", "ratio", "attack", "release", "knee", "makeup", "mix", "style", "lookahead", "range", "sc_hpf", "sc_listen"]),
        (dsp::GATE_ID, &["threshold", "range", "ratio", "attack", "hold", "release", "lookahead", "sc_hpf", "sc_lpf", "sc_listen", "mix"]),
        (dsp::DEESSER_ID, &["threshold", "range", "freq", "mode", "release", "sc_listen"]),
        (dsp::LIMITER_ID, &["gain", "ceiling", "release", "lookahead", "link"]),
        (dsp::REVERB_ID, &["predelay", "decay", "size", "damping", "diffusion", "width", "low_cut", "high_cut", "mix", "modulation"]),
        (dsp::DELAY_ID, &["time", "offset", "feedback", "pingpong", "low_cut", "high_cut", "drive", "width", "mix"]),
        (dsp::SATURATION_ID, &["drive", "style", "tone", "mix", "output"]),
    ];
    for &(id, ids) in legacy {
        let mut plugin = hosted(id);
        let defaults = plugin.params().unwrap();
        let automation = plugin.automation_params().unwrap();
        let mut state = serde_json::Map::new();
        for (index, &name) in ids.iter().enumerate() {
            let parameter = &defaults[index];
            assert_eq!(parameter.id, name, "legacy ABI changed for {id}");
            if let Some(target) = automation.iter().find(|target| target.id == name) {
                assert_eq!(target.target.native_id() as usize, index, "legacy automation moved for {id}/{name}");
            }
            state.insert(name.into(), serde_json::json!(parameter.default_value.unwrap()));
        }
        plugin.set_state(&serde_json::to_vec(&state).unwrap()).unwrap();
        assert_eq!(plugin.params().unwrap(), defaults, "old state must preserve neutral extension defaults for {id}");
    }
    let mut gate = hosted(dsp::GATE_ID);
    assert_eq!(parameter(&mut *gate, "hysteresis").value, 0.0);
    let mut delay = hosted(dsp::DELAY_ID);
    assert_eq!(parameter(&mut *delay, "sync").value, 0.0);
    assert_eq!(parameter(&mut *delay, "tap15_on").value, 0.0);
    let mut saturation = hosted(dsp::SATURATION_ID);
    assert_eq!(parameter(&mut *saturation, "band_count").value, 1.0);
}

#[test]
fn admitted_dsp_and_parameter_automation_allocate_nothing_during_audio() {
    let input = [0.125; 256];
    let key = [0.8; 256];
    for id in dsp::EFFECT_IDS {
        let mut plugin = hosted(id);
        if id == dsp::EQ_ID {
            for band in 1..=24 {
                plugin.set_param(&format!("b{band}_on"), 1.0).unwrap();
                plugin.set_param(&format!("b{band}_dynamic"), 1.0).unwrap();
            }
            plugin.set_param("auto_gain", 1.0).unwrap();
        } else if id == dsp::SATURATION_ID {
            for (name, value) in [("band_count", 6.0), ("feedback", 50.0), ("mod_depth", 6.0), ("dynamics", 50.0)] {
                plugin.set_param(name, value).unwrap();
            }
        } else if id == dsp::DELAY_ID {
            for tap in 1..=15 { plugin.set_param(&format!("tap{tap}_on"), 1.0).unwrap(); }
            for (name, value) in [("time", 5.0), ("diffusion", 50.0), ("pitch", 7.0), ("lofi", 40.0)] { plugin.set_param(name, value).unwrap(); }
        }
        let targets = plugin.automation_params().unwrap();
        let target = targets.iter().find(|p| p.id == "output")
            .or_else(|| targets.iter().find(|p| p.id == "mix"))
            .or_else(|| targets.iter().find(|p| p.id == "gain")).unwrap().target;
        let events = [target.event(0, target.min()).unwrap(), target.event(128, target.default_value()).unwrap()];
        let (mut output_l, mut output_r) = ([0.0; 256], [0.0; 256]);
        let (result, counts) = measured_audio(|| {
            for _ in 0..4 {
                plugin.process_keyed(&input, &input, (&key, &key), &mut output_l, &mut output_r, &events)?;
            }
            Ok::<_, String>(())
        });
        result.unwrap();
        assert_eq!(counts, (0, 0), "{id}: audio processing allocated/freed heap storage");
        assert!(output_l.iter().chain(&output_r).all(|sample| sample.is_finite()), "{id}");
    }
}
