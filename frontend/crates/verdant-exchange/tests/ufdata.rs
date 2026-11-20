use std::collections::BTreeMap;

use serde_json::{Value, json};
use verdant_domain::editing::{Interpolation, interpolate};
use verdant_domain::model::{ExpressionPoint, GainPoint, PitchPoint, TempoPoint, TimeSignature};
use verdant_domain::{Note, PointId, Project, Track};
use verdant_exchange::{self as exchange, Disposition, ExportOptions, Format, ImportOptions, Limits};

const MULTITRACK: &[u8] = include_bytes!("fixtures/ufdata/multitrack-controls.ufdata");
const OFFICIAL: &[u8] = include_bytes!("fixtures/ufdata/official-standard-midi.ufdata");

fn imported(value: &Value) -> exchange::Result<exchange::ImportResult> {
    exchange::import(Format::Ufdata, &serde_json::to_vec(value).unwrap(), &ImportOptions::default())
}

fn exported(project: &Project) -> (Value, exchange::ExchangeReport) {
    let result = exchange::export(Format::Ufdata, project, &ExportOptions::default()).unwrap();
    (serde_json::from_slice(&result.bytes).unwrap(), result.report)
}

fn base() -> Value {
    json!({"formatVersion":1,"project":{"name":"diagnostic","tracks":[{"name":"voice","notes":[
        {"key":60,"tickOn":100,"tickOff":200,"lyric":"a"},
        {"key":64,"tickOn":200,"tickOff":300,"lyric":"i"}
    ]}],"tempos":[{"tickPosition":0,"bpm":120}],"timeSignatures":[{"measurePosition":0,"numerator":4,"denominator":4}],"measurePrefix":0}})
}

fn native() -> Project {
    let mut project = Project::new_empty();
    project.voice_refs.clear();
    project.tracks.clear();
    let mut track = Track::new("Native", "teal");
    track.voice_ref = None;
    let mut note = Note::new(0, 960, 60);
    note.lyric = "a".into();
    note.portamento_ms = Some(0);
    track.notes.push(note);
    track.refresh_index();
    project.tracks.push(track);
    project
}

fn point(tick: i64, millicents: i64) -> PitchPoint {
    PitchPoint { id: PointId::generate(), tick, millicents, unknown: BTreeMap::new() }
}

fn offset(note: &Note, tick: f64) -> f64 {
    interpolate(&note.curves.pitch_millicent_points, tick, 0.0, note.curves.interpolation("PIT"), |p| p.tick, |p| p.millicents as f64)
}

fn authored(project: &Project) -> Value {
    json!({"ppq":project.ppq,"tempos":project.tempo_map.iter().map(|p|(p.tick,p.micro_bpm)).collect::<Vec<_>>(),
        "meters":project.time_signatures.iter().map(|p|(p.tick,p.numerator,p.denominator)).collect::<Vec<_>>(),
        "tracks":project.tracks.iter().map(|t|json!({"name":t.name,"notes":t.notes.iter().map(|n|json!({
            "start":n.start_tick,"duration":n.duration_tick,"key":n.midi_note,"lyric":n.lyric,
            "phonemes":n.pronunciation.phonemes,"alphabet":n.pronunciation.alphabet,
            "pitch":n.curves.pitch_millicent_points.iter().map(|p|(p.tick,p.millicents)).collect::<Vec<_>>(),
            "interpolation":n.curves.interpolation("PIT")
        })).collect::<Vec<_>>()})).collect::<Vec<_>>()})
}

#[test]
fn official_utaformatix_fixture_imports_and_exports_real_editable_fields() {
    assert_eq!(Format::from_extension(".UFDATA"), Some(Format::Ufdata));
    assert_eq!(Format::Ufdata.extension(), "ufdata");
    assert_eq!(serde_json::to_value(Format::Ufdata).unwrap(), "ufdata");
    let first = exchange::import(Format::Ufdata, OFFICIAL, &ImportOptions::default()).unwrap();
    assert_eq!(first.report.source_ppq, 480);
    assert_eq!(first.project.ppq, 480);
    assert_eq!(
        first.project.tracks[0].notes.iter().map(|n| (n.midi_note, n.start_tick, n.end_tick(), n.lyric.as_str())).collect::<Vec<_>>(),
        [(60, 0, 480, "do"), (62, 480, 960, "re"), (64, 960, 1440, "mi"), (65, 1440, 1920, "fa")]
    );
    assert_eq!(first.project.tempo_map.iter().map(|t| (t.tick, t.micro_bpm)).collect::<Vec<_>>(), [(0, 120_000_000), (960, 140_000_000)]);
    assert!(first.project.voice_refs.is_empty());
    assert!(first.project.tracks[0].voice_ref.is_none());
    let (out, _) = exported(&first.project);
    assert_eq!(out["formatVersion"], 1);
    assert_eq!(out["project"]["name"], "sample.mid");
    assert_eq!(out["project"]["tracks"][0]["notes"][3]["tickOff"], 1920);
    let again = imported(&out).unwrap();
    assert_eq!(authored(&first.project), authored(&again.project));
}

#[test]
fn multitrack_decimal_tempos_meter_changes_prefix_and_empty_lane_roundtrip() {
    let source: Value = serde_json::from_slice(MULTITRACK).unwrap();
    let first = exchange::import(Format::Ufdata, MULTITRACK, &ImportOptions::default()).unwrap();
    let p = &first.project;
    assert_eq!(p.tracks.len(), 3);
    assert!(p.tracks[2].notes.is_empty());
    assert_eq!(
        p.time_signatures.iter().map(|m| (m.tick, m.numerator, m.denominator)).collect::<Vec<_>>(),
        [(0, 3, 4), (2880, 5, 8), (5280, 4, 4)]
    );
    assert_eq!(
        p.tempo_map.iter().map(|t| (t.tick, t.micro_bpm)).collect::<Vec<_>>(),
        [(0, 120_500_000), (720, 90_250_000), (3600, 133_333_333)]
    );
    // The prefix was removed by the source-format parser before writing ufdata.
    assert_eq!(p.tracks[0].notes[0].start_tick, 0);
    assert_eq!(p.extensions["exchange_ufdata"]["measurePrefix"], 2);
    let time = verdant_domain::TempoMap::new(&p.tempo_map, p.ppq).unwrap();
    let expected = 1.5 * 60.0 / 120.5 + 3.0 * 60.0 / 90.25;
    assert!((time.seconds_at(2160.0) - expected).abs() < 1e-12);
    assert_eq!(p.tracks[0].notes[0].pronunciation.phonemes, ["  a   i  "]);
    assert_eq!(p.tracks[0].notes[0].pronunciation.alphabet.as_deref(), Some("ufdata-source-phoneme"));
    assert!(p.tracks[0].notes[0].pronunciation.language.is_none());
    assert!(first.report.issues.iter().any(|i| i.path.ends_with("/phoneme") && i.detail.contains("no alphabet")));
    assert_eq!(offset(&p.tracks[0].notes[0], 0.0), 25_000.0);
    assert_eq!(offset(&p.tracks[0].notes[0], 300.0), 50_000.0);
    assert_eq!(offset(&p.tracks[0].notes[0], 420.0), 0.0);
    assert_eq!(offset(&p.tracks[1].notes[0], 180.0), 12_500.0);
    assert_eq!(offset(&p.tracks[1].notes[1], 0.0), -25_000.0);
    // Native save/reopen rebuilds derived indexes; authored content and source
    // curve preservation must survive, without comparing generated point IDs.
    let reopened = exchange::parse_project_json(&serde_json::to_vec(p).unwrap(), &Limits::default()).unwrap();
    assert_eq!(authored(p), authored(&reopened));
    let (out, report) = exported(&reopened);
    assert_eq!(out["project"]["measurePrefix"], 2);
    for ti in 0..3 {
        assert_eq!(out["project"]["tracks"][ti]["pitch"], source["project"]["tracks"][ti]["pitch"]);
    }
    assert_eq!(out["vendorRoot"], source["vendorRoot"]);
    assert_eq!(out["project"]["vendorProject"], source["project"]["vendorProject"]);
    assert_eq!(out["project"]["tracks"][0]["vendorTrack"], source["project"]["tracks"][0]["vendorTrack"]);
    assert_eq!(out["project"]["tracks"][0]["notes"][0]["phoneme"], "  a   i  ");
    assert_eq!(out["project"]["tracks"][0]["notes"][0]["vendorNote"], json!({"kept":7}));
    assert_eq!(out["project"]["timeSignatures"][0]["vendorMeter"], "first");
    assert_eq!(out["project"]["tempos"][0]["vendorTempo"], "first");
    assert!(report.issues.iter().any(|i| i.disposition == Disposition::RetainedUnsupported));
    assert_eq!(authored(p), authored(&imported(&out).unwrap().project));
}

#[test]
fn absolute_null_before_first_node_and_note_key_boundaries_use_hold_not_linear() {
    let mut source = base();
    source["project"]["tracks"][0]["pitch"] = json!({"ticks":[50,150,250,300],"values":[60.5,null,65.0,null],"isAbsolute":true});
    let result = imported(&source).unwrap();
    let notes = &result.project.tracks[0].notes;
    assert_eq!(offset(&notes[0], 0.0), 50_000.0);
    assert_eq!(offset(&notes[0], 49.9), 50_000.0);
    assert_eq!(offset(&notes[0], 50.0), 0.0);
    assert_eq!(offset(&notes[1], 0.0), 0.0);
    assert_eq!(offset(&notes[1], 49.9), 0.0);
    assert_eq!(offset(&notes[1], 50.0), 100_000.0);
    source["project"]["tracks"][0]["pitch"] = json!({"ticks":[150],"values":[60.5],"isAbsolute":true});
    let result = imported(&source).unwrap();
    let notes = &result.project.tracks[0].notes;
    assert_eq!(offset(&notes[0], 0.0), 0.0);
    assert_eq!(offset(&notes[0], 50.0), 50_000.0);
    assert_eq!(offset(&notes[1], 0.0), -350_000.0);
    source["project"]["tracks"][0]["pitch"] = json!({"ticks":[150,250],"values":[0.5,0.0],"isAbsolute":false});
    let result = imported(&source).unwrap();
    let notes = &result.project.tracks[0].notes;
    assert_eq!(offset(&notes[0], 0.0), 0.0);
    assert_eq!(offset(&notes[1], 0.0), 50_000.0);
    assert_eq!(offset(&notes[1], 50.0), 0.0);
}

#[test]
fn retained_pitch_survives_lyric_edits_but_never_overrides_authored_pitch_or_timing() {
    let source: Value = serde_json::from_slice(MULTITRACK).unwrap();
    let mut p = imported(&source).unwrap().project;
    p.unknown.insert("title".into(), "edited title".into());
    p.tracks[0].notes[0].lyric = "edited lyric".into();
    let (out, _) = exported(&p);
    assert_eq!(out["project"]["name"], "edited title");
    assert_eq!(out["project"]["tracks"][0]["notes"][0]["lyric"], "edited lyric");
    assert_eq!(out["project"]["tracks"][0]["pitch"], source["project"]["tracks"][0]["pitch"]);
    let note = &mut p.tracks[0].notes[0];
    note.start_tick += 24;
    note.midi_note = 61;
    note.curves.pitch_millicent_points[0].millicents = 175_000;
    note.pronunciation.phonemes = vec!["n".into(), "i".into()];
    note.pronunciation.alphabet = Some("ipa".into());
    note.pronunciation.language = Some("zh".into());
    p.tracks[0].refresh_index();
    let (out, report) = exported(&p);
    let track = &out["project"]["tracks"][0];
    assert_eq!(track["notes"][0]["tickOn"], 24);
    assert_eq!(track["notes"][0]["key"], 61);
    assert_eq!(track["notes"][0]["phoneme"], "n i");
    assert_ne!(track["pitch"], source["project"]["tracks"][0]["pitch"]);
    let ticks = track["pitch"]["ticks"].as_array().unwrap();
    let index = ticks.iter().position(|t| t == 24).unwrap();
    assert_eq!(track["pitch"]["values"][index].as_f64(), Some(62.75));
    assert!(!ticks.contains(&json!(2300)));
    assert!(report.issues.iter().any(|i| i.path.ends_with("/pitch") && i.disposition == Disposition::Unsupported));
    assert!(report.issues.iter().any(|i| i.path.ends_with("/pronunciation") && i.detail.contains("alphabet")));
    let again = imported(&out).unwrap();
    assert_eq!(offset(&again.project.tracks[0].notes[0], 0.0), 175_000.0);
}

#[test]
fn source_decimal_pitch_precision_is_retained_until_curve_changes() {
    let mut source = base();
    source["project"]["tracks"][0]["pitch"] = json!({"ticks":[0],"values":[60.123456789],"isAbsolute":true});
    let first = imported(&source).unwrap();
    assert!(first.report.issues.iter().any(|i| i.disposition == Disposition::Quantized));
    assert_eq!(offset(&first.project.tracks[0].notes[0], 0.0), 12_346.0);
    assert_eq!(exported(&first.project).0["project"]["tracks"][0]["pitch"], source["project"]["tracks"][0]["pitch"]);
}

#[test]
fn selection_and_disabled_retention_keep_notes_maps_and_explicit_loss_reports() {
    let p = exchange::import(Format::Ufdata, MULTITRACK, &ImportOptions::default()).unwrap().project;
    let selected = exchange::export(Format::Ufdata, &p, &ExportOptions { track: Some(2), ..Default::default() }).unwrap();
    let value: Value = serde_json::from_slice(&selected.bytes).unwrap();
    assert_eq!(value["project"]["tracks"].as_array().unwrap().len(), 1);
    assert_eq!(value["project"]["tracks"][0]["name"], "Empty");
    assert_eq!(value["project"]["tempos"].as_array().unwrap().len(), 3);
    let out = exchange::export(Format::Ufdata, &p, &ExportOptions { preserve_extensions: false, ..Default::default() }).unwrap();
    let value: Value = serde_json::from_slice(&out.bytes).unwrap();
    assert!(value.get("vendorRoot").is_none());
    assert!(value["project"].get("vendorProject").is_none());
    assert!(value["project"]["tracks"][0].get("vendorTrack").is_none());
    assert!(value["project"]["tracks"][0]["notes"][0].get("vendorNote").is_none());
    assert!(value["project"]["tempos"][0].get("vendorTempo").is_none());
    assert_eq!(value["project"]["measurePrefix"], 0);
    assert_eq!(value["project"]["tracks"][0]["notes"][0]["phoneme"], "  a   i  ");
    assert!(out.report.issues.iter().any(|i| i.disposition == Disposition::Unsupported && i.path.contains("exchange_ufdata")));
    let no_extensions =
        exchange::import(Format::Ufdata, MULTITRACK, &ImportOptions { preserve_extensions: false, ..Default::default() }).unwrap();
    assert!(!no_extensions.project.extensions.contains_key("exchange_ufdata"));
    assert!(no_extensions.report.has_losses());
    assert!(!no_extensions.report.issues.iter().any(|i| i.disposition == Disposition::RetainedUnsupported));
}

#[test]
fn changing_continuous_pitch_is_sampled_with_limits_and_not_relabelled_as_hold_nodes() {
    for mode in [Interpolation::Linear, Interpolation::Cosine, Interpolation::Cubic] {
        let mut p = native();
        p.tracks[0].notes[0].curves.pitch_millicent_points = vec![point(0, 0), point(960, 100_000)];
        p.tracks[0].notes[0].curves.set_interpolation("PIT", mode);
        let (out, report) = exported(&p);
        let pitch = &out["project"]["tracks"][0]["pitch"];
        let index = pitch["ticks"].as_array().unwrap().iter().position(|t| t == 240).unwrap();
        assert!((pitch["values"][index].as_f64().unwrap() - 60.5).abs() < 1e-12);
        assert_eq!(pitch["ticks"].as_array().unwrap().last(), Some(&json!(480)));
        assert!(pitch["values"].as_array().unwrap().last().unwrap().is_null());
        assert!(report.issues.iter().any(|i| i.disposition == Disposition::Quantized && i.detail.contains("sampled")));
        let again = imported(&out).unwrap();
        let note = &again.project.tracks[0].notes[0];
        assert_eq!(note.curves.interpolation("PIT"), Interpolation::Hold);
        assert_eq!(offset(note, 240.0), 50_000.0);
        let mut options = ExportOptions::default();
        options.limits.max_events = 100;
        assert!(exchange::export(Format::Ufdata, &p, &options).unwrap_err().to_string().contains("sampling limit"));
    }
}

#[test]
fn sparse_hold_and_detune_export_without_continuous_curve_sampling() {
    let mut p = native();
    let n = &mut p.tracks[0].notes[0];
    n.detune_millicents = 25_000;
    n.curves.pitch_millicent_points = vec![point(0, 0), point(240, 50_000), point(720, -25_000), point(960, -25_000)];
    n.curves.set_interpolation("PIT", Interpolation::Hold);
    let (out, report) = exported(&p);
    assert_eq!(out["project"]["tracks"][0]["pitch"], json!({"ticks":[0,120,360],"values":[60.25,60.75,null],"isAbsolute":true}));
    assert!(!report.issues.iter().any(|i| i.disposition == Disposition::Quantized));
    let again = imported(&out).unwrap();
    let n = &again.project.tracks[0].notes[0];
    assert_eq!(offset(n, 0.0), 25_000.0);
    assert_eq!(offset(n, 200.0), 75_000.0);
    assert_eq!(offset(n, 400.0), 0.0);
}

#[test]
fn timing_rounding_is_reported_and_collapsed_notes_colliding_tempos_and_offbar_meters_fail() {
    let mut p = native();
    p.tracks[0].notes[0].start_tick = 1;
    p.tracks[0].notes[0].duration_tick = 5;
    p.tracks[0].refresh_index();
    let (out, report) = exported(&p);
    assert_eq!(out["project"]["tracks"][0]["notes"][0]["tickOn"], 1);
    assert_eq!(out["project"]["tracks"][0]["notes"][0]["tickOff"], 3);
    assert!(report.issues.iter().any(|i| i.path.ends_with("/tickOn") && i.disposition == Disposition::Quantized));
    p.tracks[0].notes[0].duration_tick = 1;
    assert!(exchange::export(Format::Ufdata, &p, &ExportOptions::default()).unwrap_err().to_string().contains("collapses"));
    let mut p = native();
    p.tempo_map.extend([
        TempoPoint { tick: 1, micro_bpm: 100_000_000, unknown: BTreeMap::new() },
        TempoPoint { tick: 2, micro_bpm: 110_000_000, unknown: BTreeMap::new() },
    ]);
    assert!(exchange::export(Format::Ufdata, &p, &ExportOptions::default()).unwrap_err().to_string().contains("collide"));
    let mut p = native();
    p.time_signatures.push(TimeSignature { tick: 480, numerator: 3, denominator: 4, unknown: BTreeMap::new() });
    assert!(exchange::export(Format::Ufdata, &p, &ExportOptions::default()).unwrap_err().to_string().contains("bar boundaries"));
}

#[test]
fn nondefault_project_render_preferences_are_reported_without_warning_on_defaults() {
    let mut p = native();
    let (default_file, default_report) = exported(&p);
    assert!(!default_report.has_losses(), "ordinary default score should not receive generic loss warnings");
    assert!(!default_report.issues.iter().any(|i| i.path == "/render_preferences"));
    assert_eq!(imported(&default_file).unwrap().project.render_preferences, p.render_preferences);
    for route in ["C0", "C1"] {
        p.render_preferences.route = route.into();
        let before = serde_json::to_value(&p.render_preferences).unwrap();
        let (out, report) = exported(&p);
        assert!(report.has_losses());
        let issue = report.issues.iter().find(|i| i.path == "/render_preferences").unwrap();
        assert_eq!(issue.disposition, Disposition::Unsupported);
        assert_eq!(issue.value, before);
        assert!(out["project"].get("render_preferences").is_none());
        let again = imported(&out).unwrap();
        assert_eq!(again.project.render_preferences, verdant_domain::model::RenderPreferences::default());
        assert_eq!(again.project.tracks[0].notes[0].midi_note, p.tracks[0].notes[0].midi_note);
        assert_eq!(serde_json::to_value(&p.render_preferences).unwrap(), before, "export must not mutate the source project");
    }
    p.render_preferences = Default::default();
    p.render_preferences.native_seed += 1;
    assert!(exported(&p).1.issues.iter().any(|i| i.path == "/render_preferences"));
}

#[test]
fn unsupported_native_controls_and_phonetic_metadata_are_visible_in_export_report() {
    let mut p = native();
    let t = &mut p.tracks[0];
    t.gain_millidecibels = -6000;
    t.pan_permille = -200;
    t.muted = true;
    t.render_route = Some("verdant-voice".into());
    let n = &mut t.notes[0];
    n.pronunciation.pinyin = "a1".into();
    n.pronunciation.phonemes = vec!["a".into()];
    n.pronunciation.alphabet = Some("ipa".into());
    n.pronunciation.language = Some("zh".into());
    n.curves.gain_millidecibel_points.push(GainPoint { id: PointId::generate(), tick: 0, millidecibels: -1200, unknown: BTreeMap::new() });
    n.curves
        .expression_points
        .insert("BRI".into(), vec![ExpressionPoint { id: PointId::generate(), tick: 0, value: 90, unknown: BTreeMap::new() }]);
    let (out, report) = exported(&p);
    assert!(report.has_losses());
    for suffix in [
        "/gain_millidecibels",
        "/pan_permille",
        "/muted",
        "/render_route",
        "/pronunciation/pinyin",
        "/pronunciation",
        "/curves/DYN",
        "/curves/BRI",
    ] {
        assert!(report.issues.iter().any(|i| i.path.ends_with(suffix) && i.disposition == Disposition::Unsupported), "missing {suffix}");
    }
    assert_eq!(out["project"]["tracks"][0]["notes"][0]["phoneme"], "a");
    assert!(out["project"]["tracks"][0].get("render_route").is_none());
}

#[test]
fn malformed_json_versions_pitch_and_time_maps_are_rejected_without_silent_repairs() {
    let invalid: Vec<(&str, Value)> = vec![
        ("/formatVersion", json!(0)),
        ("/formatVersion", json!(2)),
        ("/formatVersion", json!(1.5)),
        ("/project/measurePrefix", json!(-1)),
        ("/project/name", json!(null)),
        ("/project/tempos", json!([{"tickPosition":1,"bpm":120}])),
        ("/project/tempos", json!([{"tickPosition":0,"bpm":0}])),
        ("/project/tempos", json!([{"tickPosition":0,"bpm":120},{"tickPosition":0,"bpm":130}])),
        ("/project/timeSignatures", json!([{"measurePosition":0,"numerator":4,"denominator":3}])),
        ("/project/timeSignatures", json!([{"measurePosition":1,"numerator":4,"denominator":4}])),
        ("/project/timeSignatures", json!([{"measurePosition":0,"numerator":65,"denominator":4}])),
        ("/project/tracks/0/notes/0/tickOn", json!(-1)),
        ("/project/tracks/0/notes/0/tickOff", json!(100)),
        ("/project/tracks/0/notes/0/tickOff", json!(9_007_199_254_740_992i64)),
        ("/project/tracks/0/notes/0/key", json!(128)),
        ("/project/tracks/0/notes/0/lyric", json!(17)),
        ("/project/tracks/0/pitch", json!({"ticks":[0],"values":[],"isAbsolute":true})),
        ("/project/tracks/0/pitch", json!({"ticks":[0],"values":[null],"isAbsolute":false})),
        ("/project/tracks/0/pitch", json!({"ticks":[0,0],"values":[0.1,0.2]})),
        ("/project/tracks/0/pitch", json!({"ticks":[2,1],"values":[0.1,0.2]})),
        ("/project/tracks/0/pitch", json!({"ticks":[0],"values":[1e100]})),
        ("/project/tracks/0/pitch", json!({"ticks":[0],"values":[0.5],"isAbsolute":"true"})),
    ];
    for (pointer, value) in invalid {
        let mut source = base();
        if pointer.ends_with("/pitch") {
            source["project"]["tracks"][0]["pitch"] = value;
        } else {
            *source.pointer_mut(pointer).unwrap() = value;
        }
        assert!(imported(&source).is_err(), "accepted {source}");
    }
    for bytes in [
        br#"{"formatVersion":1,"formatVersion":1,"project":{}}"#.as_slice(),
        br#"{"formatVersion":1,"project":{"name":"x","name":"y"}}"#.as_slice(),
        br#"{"formatVersion":1,"project":{"name":"x","tracks":[],"tempos":[],"timeSignatures":[],"unknown":{"x":1,"x":2}}}"#.as_slice(),
        br#"{"formatVersion":1,"project":{"name":"x","tempos":[{"tickPosition":0,"bpm":1e400}]}}"#.as_slice(),
        b"{} trailing".as_slice(),
        b"\xff".as_slice(),
    ] {
        assert!(exchange::import(Format::Ufdata, bytes, &ImportOptions::default()).is_err());
    }
}

#[test]
fn overlaps_and_resource_limits_fail_and_retained_pitch_is_revalidated_on_export() {
    let mut source = base();
    source["project"]["tracks"][0]["notes"][1]["tickOn"] = 199.into();
    assert!(imported(&source).unwrap_err().to_string().contains("overlapping"));
    let mut p = native();
    let mut note = Note::new(500, 960, 64);
    note.portamento_ms = Some(0);
    p.tracks[0].notes.push(note);
    p.tracks[0].refresh_index();
    assert!(exchange::export(Format::Ufdata, &p, &ExportOptions::default()).unwrap_err().to_string().contains("overlapping"));
    for limits in [
        Limits { max_file_bytes: 10, ..Default::default() },
        Limits { max_tracks: 0, ..Default::default() },
        Limits { max_notes: 1, ..Default::default() },
        Limits { max_events: 1, ..Default::default() },
    ] {
        assert!(exchange::import(Format::Ufdata, OFFICIAL, &ImportOptions { limits, ..Default::default() }).is_err());
    }
    let mut p = exchange::import(Format::Ufdata, MULTITRACK, &ImportOptions::default()).unwrap().project;
    p.tracks[0].unknown["exchange_ufdata"]["pitch"]["values"] = json!([]);
    assert!(exchange::export(Format::Ufdata, &p, &ExportOptions::default()).unwrap_err().to_string().contains("equal lengths"));
}

#[test]
fn empty_defaults_nullable_pitch_and_utf8_bom_are_supported_explicitly() {
    let value =
        json!({"formatVersion":1,"project":{"name":"empty","tracks":[{"name":"Empty","pitch":null}],"tempos":[],"timeSignatures":[]}});
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(serde_json::to_vec(&value).unwrap());
    let first = exchange::import(Format::Ufdata, &bytes, &ImportOptions::default()).unwrap();
    assert!(first.project.tracks[0].notes.is_empty());
    assert_eq!(first.project.tempo_map[0].micro_bpm, 120_000_000);
    assert_eq!(first.project.time_signatures[0].numerator, 4);
    assert_eq!(first.report.issues.iter().filter(|i| i.detail.contains("Empty")).count(), 2);
    let (out, _) = exported(&first.project);
    assert_eq!(out["project"]["tracks"][0]["pitch"]["ticks"], json!([]));
    let no_tracks = json!({"formatVersion":1,"project":{"name":"empty","tempos":[],"timeSignatures":[]}});
    assert!(imported(&no_tracks).unwrap().project.tracks.is_empty());
}
