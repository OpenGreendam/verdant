use super::{modulation_batch, modulation_parameter, vibrato_preset_batch};
use crate::generated::controls::{self, CommandArg, NumericId as N};
use verdant_backend::f0::{ControlTrack, control_track};
use verdant_domain::model::{NoteModulationParameter as P, VibratoPreset};
use verdant_domain::{Command, CommandBatch, Note, Project, ProjectStore, TempoMap, Vibrato};

fn frame_cents(control: &ControlTrack, seconds: f64) -> f64 {
    let i = control.frame_times_seconds.iter().position(|t| (*t - seconds).abs() < 1e-10).unwrap();
    assert!(!control.uv[i]);
    6900.0 + 1200.0 * (control.f0_hz[i] / 440.0).log2()
}

fn close(actual: f64, expected: f64) {
    // f0_hz is quantized to three decimals by the worker protocol; recovering
    // cents from a rounded Hz moves the value by up to ~0.0033 cents.
    assert!((actual - expected).abs() < 1e-2, "{actual} != {expected}");
}

#[test]
fn numeric_vibrato_and_portamento_compile_at_authored_frames() {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 60_000_000;
    let track = project.tracks[0].id.clone();
    let a = Note::new(0, 960, 60);
    let b = Note::new(960, 1920, 72);
    let target = b.id.clone();
    project.tracks[0].notes = vec![a, b];
    let mut store = ProjectStore::new(project);
    for (numeric, value) in [(N::VibratoDelayPercent, 50.0), (N::VibratoDepthCents, 40.0),
        (N::VibratoRateHz, 5.0), (N::VibratoPhaseDegrees, 90.0), (N::PortamentoMs, 200.0),
        (N::PortamentoCurvePercent, 100.0)] {
        let parameter = modulation_parameter(numeric).unwrap();
        let notes = store.project().tracks[0].notes.iter().filter(|n| n.id == target).cloned().collect();
        store.apply(store.revision(), modulation_batch(track.clone(), notes, parameter, value, "数值编辑").unwrap()).unwrap().unwrap();
    }
    let notes: Vec<_> = store.project().tracks[0].notes.iter().collect();
    let control = control_track(&notes, &TempoMap::constant(60.0), 0.0, 3.0);
    close(frame_cents(&control, 1.0), 6000.0);
    // At 50 ms of a 200 ms glide, +100% bends x=.25 to .4375.
    let x: f64 = 0.25 + 0.25 * 0.75;
    close(frame_cents(&control, 1.05), 6000.0 + 1200.0 * x * x * (3.0 - 2.0 * x));
    close(frame_cents(&control, 1.20), 7200.0);
    close(frame_cents(&control, 1.995), 7200.0);
    close(frame_cents(&control, 2.0), 7200.0); // onset ramp starts at zero
    close(frame_cents(&control, 2.20), 7240.0); // 5 Hz, phase 90°, full ramp
    close(frame_cents(&control, 2.30), 7160.0);
    assert_eq!(control.f0_hz.last(), Some(&0.0));
    assert_eq!(control.uv.last(), Some(&true));
}

#[test]
fn modulation_numeric_ranges_reject_without_mutation_or_history() {
    let mut project = Project::new_empty();
    let track = project.tracks[0].id.clone();
    let mut note = Note::new(0, 960, 60);
    note.vibrato = Some(Vibrato::DEFAULT);
    project.tracks[0].notes.push(note.clone());
    let store = ProjectStore::new(project);
    for numeric in [N::VibratoDelayPercent, N::VibratoDepthCents, N::VibratoRateHz,
        N::VibratoPhaseDegrees, N::PortamentoMs, N::PortamentoCurvePercent] {
        let parameter = modulation_parameter(numeric).unwrap();
        let (min, max) = parameter.range();
        let spec = controls::numeric(numeric);
        assert_eq!((spec.min, spec.max), (min, max));
        assert_eq!(spec.arg, CommandArg::None.kind());
        for value in [min - 0.01, max + 0.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut edited = note.clone();
            assert!(edited.set_modulation_value(parameter, value).is_err());
            assert_eq!(edited, note);
            assert!(modulation_batch(track.clone(), vec![note.clone()], parameter, value, "无效编辑").is_err());
        }
    }
    assert_eq!(store.project().tracks[0].notes, vec![note]);
    assert_eq!(store.undo_depth(), 0);
}

#[test]
fn named_vibrato_preset_is_one_undo_and_preserves_other_note_controls() {
    for preset in VibratoPreset::ALL {
        let mut project = Project::new_empty();
        let track = project.tracks[0].id.clone();
        for (start, pitch) in [(0, 60), (960, 64)] {
            let mut note = Note::new(start, 960, pitch);
            note.portamento_ms = Some(175);
            note.set_modulation_value(P::PortamentoCurve, -40.0).unwrap();
            note.unknown.insert("future_note_data".into(), serde_json::json!({"keep": true}));
            let mut vibrato = Vibrato::DEFAULT;
            vibrato.unknown.insert("future_shape_data".into(), serde_json::json!("keep"));
            note.vibrato = Some(vibrato);
            project.tracks[0].notes.push(note);
        }
        let before = project.tracks[0].notes.clone();
        let mut store = ProjectStore::new(project);
        store.apply(store.revision(), vibrato_preset_batch(track, before.clone(), preset)).unwrap().unwrap();
        assert_eq!(store.undo_depth(), 1);
        assert_eq!(store.undo_label(), Some(preset.label()));
        for (note, original) in store.project().tracks[0].notes.iter().zip(&before) {
            let (depth, rate, delay) = match preset {
                VibratoPreset::Gentle => (20, 5000, 500),
                VibratoPreset::Natural => (35, 5500, 350),
                VibratoPreset::Wide => (70, 6000, 250),
            };
            let vibrato = note.vibrato.as_ref().unwrap();
            assert_eq!((vibrato.depth_cents, vibrato.rate_millihertz, vibrato.delay_permille), (depth, rate, delay));
            assert_eq!((vibrato.fade_in_permille(), vibrato.fade_out_permille(), vibrato.phase_millidegrees()), (150, 150, 0));
            let mut restored = note.clone();
            restored.vibrato = original.vibrato.clone();
            assert_eq!(restored, *original);
            assert_eq!(note.portamento_ms, Some(175));
            assert_eq!(note.portamento_curve_percent(), -40);
            assert_eq!(note.vibrato.as_ref().unwrap().unknown["future_shape_data"], "keep");
        }
        store.undo().unwrap().unwrap();
        assert_eq!(store.project().tracks[0].notes, before);
        store.redo().unwrap().unwrap();
        assert_eq!(store.undo_depth(), 1);
    }
}

#[test]
fn portamento_curvature_is_monotone_and_zero_preserves_existing_progress() {
    let mut note = Note::new(0, 960, 60);
    for curve in [-100.0, -40.0, 0.0, 40.0, 100.0] {
        note.set_modulation_value(P::PortamentoCurve, curve).unwrap();
        assert_eq!(note.portamento_curve_position(0.0), 0.0);
        assert_eq!(note.portamento_curve_position(1.0), 1.0);
        let mut previous = 0.0;
        for step in 0..=100 {
            let x = step as f64 / 100.0;
            let y = note.portamento_curve_position(x);
            assert!(y >= previous && y <= 1.0);
            if curve == 0.0 { assert_eq!(y, x); }
            previous = y;
        }
    }
}

#[test]
fn invalid_persisted_portamento_curve_rejects_the_whole_edit() {
    let mut project = Project::new_empty();
    let track = project.tracks[0].id.clone();
    let a = Note::new(0, 960, 60);
    let b = Note::new(960, 960, 64);
    project.tracks[0].notes = vec![a.clone(), b.clone()];
    let mut store = ProjectStore::new(project);
    let before = store.snapshot();
    for invalid in [serde_json::json!(-101), serde_json::json!(101), serde_json::json!(1.5),
        serde_json::json!("20"), serde_json::Value::Null] {
        let mut changed_a = a.clone();
        changed_a.set_modulation_value(P::VibratoDepth, 70.0).unwrap();
        let mut changed_b = b.clone();
        changed_b.unknown.insert("portamento_curve_percent".into(), invalid);
        let batch = CommandBatch::single("无效曲率", Command::ReplaceNotes { track: track.clone(), notes: vec![changed_a, changed_b] });
        assert!(store.apply(store.revision(), batch).is_err());
        assert_eq!(store.project().tracks[0].notes, before.tracks[0].notes);
        assert_eq!(store.undo_depth(), 0);
        assert_eq!(store.revision().get(), 0);
    }
}
