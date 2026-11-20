use verdant_import::{EventKind, FieldDisposition, ImportFormat, ImportOptions, LyricMode, UstEncoding, import};

fn ust(events: &str) -> String {
    format!("[#VERSION]\nUST Version1.2\n[#SETTING]\nTempo=120\nTracks=1\n{events}[#TRACKEND]\n")
}
fn ust_note(section: usize, lyric: &str, length: i64, pitch: i32, extra: &str) -> String {
    format!("[#{section:04}]\nLength={length}\nLyric={lyric}\nNoteNum={pitch}\n{extra}")
}
fn ustx(notes: &str, extra: &str) -> String {
    format!("ustx_version: '0.7'\nresolution: 480\nbpm: 120\ntracks: [{{track_name: vocal}}]\nvoice_parts:\n- track_no: 0\n  position: 0\n  notes:\n{notes}{extra}")
}
fn one_note(extra: &str) -> String {
    format!("  - position: 0\n    duration: 480\n    tone: 60\n    lyric: ma\n{extra}")
}
fn parse_ustx(raw: &str) -> verdant_import::ImportResult {
    import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions::default()).unwrap()
}

#[test]
fn yaml_duplicates_are_rejected_before_any_mapping_is_overwritten() {
    for raw in [
        "ustx_version: '0.7'\nbpm: 120\nbpm: 91.5\n",
        "ustx_version: '0.7'\ntracks: [{singer: a, singer: b}]\n",
        "ustx_version: '0.7'\nexpressions: {dyn: {min: -240, min: 0}}\n",
        "ustx_version: '0.7'\nvalue: &v {a: 1, a: 2}\ncopy: *v\n",
    ] {
        let error = import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions::default()).unwrap_err().to_string();
        assert!(error.contains("duplicate"), "{error}");
    }
    for raw in ["x: {<<: {a: 1}}", "x: .nan", "x: .inf", "x: !unsafe value", "x: 1\n---\ny: 2"] {
        assert!(import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions::default()).is_err());
    }
}

#[test]
fn real_tempo_map_wins_over_stale_top_level_bpm_and_integrates_changes() {
    let raw = ustx(&one_note(""), "tempos: [{position: 0, bpm: 91.5}, {position: 240, bpm: 183}]\n");
    let result = parse_ustx(&raw);
    assert_eq!(result.project.tempo_map.iter().map(|t| (t.tick, t.micro_bpm)).collect::<Vec<_>>(), [(0, 91_500_000), (480, 183_000_000)]);
    assert!((result.report.duration_seconds - (30.0 / 91.5 + 30.0 / 183.0)).abs() < 1e-12);
    assert!(result.report.notices.iter().any(|n| n.code == "obsolete_top_level_bpm"));
    assert_eq!(result.project.tracks[0].notes[0].duration_tick, 960);
    let invalid_compatibility_bpm = raw.replacen("bpm: 120", "bpm: obsolete", 1);
    assert_eq!(parse_ustx(&invalid_compatibility_bpm).project.tempo_map[0].micro_bpm, 91_500_000);
}

#[test]
fn resolution_meters_parts_tracks_and_mixer_values_keep_source_meaning() {
    let raw = "ustx_version: '0.7'
resolution: 96
bpm: 129
time_signatures:
- {bar_position: 0, beat_per_bar: 3, beat_unit: 8}
- {bar_position: 2, beat_per_bar: 5, beat_unit: 4}
tracks:
- {track_name: empty}
- {track_name: harmony, volume: -6, pan: -25, mute: true, solo: true, singer: private-bank}
voice_parts:
- track_no: 1
  position: 96
  duration: 96
  notes: [{position: 0, duration: 48, tone: 92, lyric: la}]
";
    let result = parse_ustx(raw);
    assert_eq!(result.project.tracks.len(), 2);
    let track = &result.project.tracks[1];
    assert_eq!((track.notes[0].start_tick, track.notes[0].duration_tick, track.notes[0].midi_note), (960, 480, 92));
    assert_eq!((track.gain_millidecibels, track.pan_permille, track.muted, track.solo), (-6000, -250, true, true));
    assert_eq!(result.project.time_signatures[1].tick, 2880);
    assert_eq!((result.project.time_signatures[1].numerator, result.project.time_signatures[1].denominator), (5, 4));
    assert!(result.report.unsupported_fields().any(|f| f.location == "/tracks/1/singer" && f.source_value == "private-bank"));
    let selected = import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions { tracks: vec![1], ..Default::default() }).unwrap();
    assert_eq!(selected.project.tracks.len(), 1);
    assert_eq!(selected.report.tracks[0].source_track_index, 1);
    assert_eq!(selected.report.events.iter().find(|e| e.kind == EventKind::Note).unwrap().source_id, "t1/p0/n0");
}

#[test]
fn cp932_ust_decodes_strictly_and_preserves_source_lyric_and_identity() {
    let raw = ust(&ust_note(0, "あ", 480, 60, ""));
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(&raw);
    assert!(!had_errors);
    let result = import(ImportFormat::Ust, &encoded, &ImportOptions { ust_encoding: UstEncoding::Cp932, ..Default::default() }).unwrap();
    assert_eq!(result.report.source.encoding, "cp932");
    assert_eq!(result.project.tracks[0].notes[0].lyric, "あ");
    assert!(result.project.tracks[0].notes[0].pronunciation.pinyin.is_empty());
    assert_eq!(result.report.events[0].source_duration_tick, 480);
    assert_eq!(result.report.events[0].project_duration_tick, 960);
    assert!(!result.report.source_file_copied);
    let mut malformed = encoded.into_owned(); malformed.push(0x81);
    assert!(import(ImportFormat::Ust, &malformed, &ImportOptions { ust_encoding: UstEncoding::Cp932, ..Default::default() }).is_err());
}

#[test]
fn ust_rejects_ambiguous_sections_and_control_syntax() {
    let note = ust_note(0, "ma", 480, 60, "");
    for raw in [
        ust(&(note.clone() + "Lyric=la\n")),
        ust(&(note.clone() + &note)),
        ust(&ust_note(1, "ma", 480, 60, "")),
        ust(&(note.clone() + "[#INSERT]\nLength=10\n")),
        ust(&(note.clone() + "not a key\n")),
        ust(&note).replace("Tracks=1", "Tracks=2"),
        ust(&note).replace("Tracks=1", "Tracks=1\nResolution=960"),
    ] { assert!(import(ImportFormat::Ust, raw.as_bytes(), &ImportOptions::default()).is_err(), "{raw}"); }
}

#[test]
fn ust_per_event_tempo_changes_and_intensity_are_active_domain_controls() {
    let raw = ust(&(ust_note(0, "ma", 480, 60, "Tempo=183\nIntensity=50\n") + &ust_note(1, "la", 480, 61, "Tempo=91.5\n")));
    let result = import(ImportFormat::Ust, raw.as_bytes(), &ImportOptions::default()).unwrap();
    assert_eq!(result.project.tempo_map.iter().map(|t| (t.tick, t.micro_bpm)).collect::<Vec<_>>(), [(0, 183_000_000), (960, 91_500_000)]);
    assert_eq!(result.project.tracks[0].notes[0].curves.gain_millidecibel_points[0].millidecibels, -6021);
    assert_eq!(result.project.tracks[0].notes[1].midi_note, 61);
}

#[test]
fn rests_gaps_and_same_pitch_continuations_preserve_boundaries_and_ids() {
    let notes = "  - {position: 0, duration: 240, tone: 60, lyric: ma}
  - {position: 240, duration: 240, tone: 60, lyric: '+'}
  - {position: 480, duration: 120, tone: 60, lyric: ma}
  - {position: 720, duration: 120, tone: 60, lyric: R}
";
    let result = parse_ustx(&ustx(notes, ""));
    let notes = &result.project.tracks[0].notes;
    assert_eq!(notes.len(), 3);
    assert_eq!((notes[0].start_tick, notes[0].duration_tick), (0, 480));
    assert_eq!((notes[1].start_tick, notes[1].duration_tick), (480, 480));
    assert_eq!((notes[2].start_tick, notes[2].duration_tick), (960, 240));
    assert!(!notes[0].is_continuation());
    assert!(notes[1].is_continuation());
    assert!(!notes[2].is_continuation());
    let continuation = result.report.events.iter().find(|e| e.kind == EventKind::Continuation).unwrap();
    assert_eq!(continuation.project_note_id.as_ref(), Some(&notes[1].id));
    assert_eq!(continuation.continuation_of_project_note_id.as_ref(), Some(&notes[0].id));
    assert_eq!(continuation.source_lyric, "+");
    assert_eq!(notes[0].unknown["source_note_ids"], serde_json::json!(["t0/p0/n0"]));
    assert_eq!(notes[1].unknown["source_note_ids"], serde_json::json!(["t0/p0/n1"]));
    assert_eq!(result.report.tracks[0].source_event_count, 4);
    assert_eq!(result.report.tracks[0].continuation_count, 1);
    assert!(result.report.events.iter().any(|e| e.kind == EventKind::Rest && e.source_start_tick == 720));
    assert!(result.report.events.iter().any(|e| e.kind == EventKind::Gap && e.source_start_tick == 600 && e.source_duration_tick == 120));
    for notes in [
        "  - {position: 0, duration: 240, tone: 60, lyric: '+'}\n",
        "  - {position: 0, duration: 240, tone: 60, lyric: R}\n  - {position: 240, duration: 240, tone: 60, lyric: '+'}\n",
        "  - {position: 0, duration: 240, tone: 60, lyric: ma}\n  - {position: 241, duration: 240, tone: 61, lyric: '+'}\n",
    ] { assert!(import(ImportFormat::Ustx, ustx(notes, "").as_bytes(), &ImportOptions::default()).is_err()); }
}

#[test]
fn ustx_pitch_dyn_and_rich_vibrato_use_source_units_and_keep_voicebank_fields() {
    let note = one_note("    pitch: {snap_first: false, data: [{x: 0, y: 10, shape: l}, {x: 500, y: 10, shape: l}]}\n    vibrato: {length: 100, period: 200, depth: 25, in: 0, out: 0, shift: 25, drift: 0, vol_link: 100}\n    phoneme_overrides: [{index: 2, phoneme: bank-specific, offset: -12}]\n");
    let raw = ustx(&note, "  curves:\n  - {abbr: pitd, xs: [0, 480], ys: [100, 100]}\n  - {abbr: dyn, xs: [0, 480], ys: [-60, -60]}\n");
    let result = parse_ustx(&raw);
    let note = &result.project.tracks[0].notes[0];
    // +10 source note units = +100 cents; PITD +100 = +100 cents;
    // quarter-cycle vibrato starts at +25 cents. No base MIDI rewrite.
    assert_eq!(note.midi_note, 60);
    assert_eq!(note.curves.pitch_millicent_points[0].millicents, 225_000);
    // DYN -60 = -6 dB, linked vibrato gain is 1.2 at this phase.
    assert_eq!(note.curves.gain_millidecibel_points[0].millidecibels, -4416);
    assert!(note.curves.pitch_millicent_points.iter().any(|p| p.millicents < 200_000));
    assert_eq!(note.portamento_ms, Some(0));
    let raw_override = result.report.unsupported_fields().find(|f| f.location.ends_with("/phoneme_overrides")).unwrap();
    assert_eq!(raw_override.source_value[0]["index"], 2);
    assert_eq!(raw_override.source_value[0]["phoneme"], "bank-specific");
    assert_eq!(result.report.events[0].source_reading_evidence, verdant_import::SourceReadingEvidence::VoicebankPhonemeAlias);
    assert_eq!(result.report.events[0].reading, verdant_import::ReadingDisposition::Unresolved);
    assert!(note.pronunciation.phonemes.is_empty());
    assert_eq!(result.project.extensions["score_import"]["source"]["sha256"], result.report.source.sha256);
    let roundtrip = serde_json::to_string(&result.project).unwrap();
    let (loaded, _) = verdant_domain::io::parse_project(&roundtrip).unwrap();
    assert_eq!(loaded.tracks[0].notes[0].curves, note.curves);
    assert_eq!(loaded.extensions["score_import"], result.project.extensions["score_import"]);
}

#[test]
fn unsupported_pitch_shapes_are_retained_without_substituting_a_curve() {
    let raw = ustx(&one_note("    pitch: {snap_first: false, data: [{x: 0, y: 10, shape: sp}, {x: 100, y: 0, shape: sp}]}\n"), "wave_parts: [{file_path: private.wav, track_no: 0, position: 0}]\n");
    let result = parse_ustx(&raw);
    assert!(result.project.tracks[0].notes[0].curves.pitch_millicent_points.is_empty());
    assert!(result.report.fields.iter().any(|f| f.location.ends_with("/pitch") && f.disposition == FieldDisposition::RetainedUnsupported));
    assert!(result.report.unsupported_fields().any(|f| f.location == "/wave_parts"));
}

#[test]
fn unnumbered_pinyin_is_only_admitted_by_explicit_option_or_override() {
    let raw = ustx(&one_note(""), "");
    let preserved = parse_ustx(&raw);
    assert!(preserved.project.tracks[0].notes[0].pronunciation.pinyin.is_empty());
    let unresolved = preserved.report.unresolved_readings().next().unwrap();
    assert_eq!((&*unresolved.source_id, &*unresolved.source_lyric), ("t0/p0/n0", "ma"));
    assert_eq!(unresolved.source_reading_evidence, verdant_import::SourceReadingEvidence::LyricOnly);
    let explicit = import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions { lyric_mode: LyricMode::Pinyin, ..Default::default() }).unwrap();
    assert_eq!(explicit.project.tracks[0].notes[0].pronunciation.pinyin, "ma");
    assert_eq!(explicit.report.events[0].reading, verdant_import::ReadingDisposition::DeclaredSourcePinyin);
    assert_eq!(explicit.report.unresolved_readings().count(), 0);
    let sidecar = br#"{"by_source_id":{"t0/p0/n0":{"pinyin":"ma3"}}}"#;
    let readings = verdant_import::parse_readings_json(sidecar).unwrap();
    let overridden = import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions { readings, ..Default::default() }).unwrap();
    assert_eq!(overridden.project.tracks[0].notes[0].pronunciation.pinyin, "ma3");
    assert!(verdant_import::parse_readings_json(br#"{"by_lyric":{"a":{"pinyin":"a1"},"a":{"pinyin":"a2"}}}"#).is_err());
}

#[test]
fn invalid_tempo_meter_tick_expression_and_overlap_boundaries_are_errors() {
    let baseline = ustx(&one_note(""), "");
    for raw in [
        baseline.replace("resolution: 480", "resolution: 1000").replace("duration: 480", "duration: 1"),
        baseline.clone() + "tempos: [{position: 1, bpm: 120}]\n",
        baseline.clone() + "tempos: [{position: 0, bpm: 120}, {position: 0, bpm: 129}]\n",
        baseline.clone() + "tempos: [{position: 0, bpm: 0}]\n",
        baseline.clone() + "time_signatures: [{bar_position: 0, beat_per_bar: 4, beat_unit: 3}]\n",
        ustx(&(one_note("") + "  - {position: 100, duration: 100, tone: 61, lyric: la}\n"), ""),
        ustx(&one_note("    vibrato: {length: 100, period: 0, depth: 25}\n"), ""),
        ustx(&one_note("    pitch: {snap_first: false, data: [{x: 0, y: 300, shape: l}]}\n"), ""),
        baseline.replace("duration: 480", "duration: true"),
    ] { assert!(import(ImportFormat::Ustx, raw.as_bytes(), &ImportOptions::default()).is_err(), "{raw}"); }
}

#[test]
fn hashes_and_persistent_ids_are_deterministic_and_admission_is_exact() {
    let raw = ustx(&one_note(""), "");
    let a = parse_ustx(&raw);
    let b = parse_ustx(&raw);
    assert_eq!(serde_json::to_value(&a.project).unwrap(), serde_json::to_value(&b.project).unwrap());
    assert_eq!(a.report.source.sha256.len(), 64);
    let options = ImportOptions { expected_source_sha256: Some(a.report.source.sha256), ..Default::default() };
    assert!(import(ImportFormat::Ustx, raw.as_bytes(), &options).is_ok());
    let changed = raw.replace("tone: 60", "tone: 61");
    assert!(import(ImportFormat::Ustx, changed.as_bytes(), &options).is_err());
}

#[test]
fn ust_mode2_and_vibrato_translate_while_resampler_fields_remain_explicit_losses() {
    let raw = ust(&ust_note(0, "ma", 480, 69,
        "PBS=0;5\nPBW=500\nPBY=5\nPBM=s\nVBR=100,200,25,0,0,25,0\nFlags=g-5\nEnvelope=0,5,35,0,100,100,0\n"));
    let result = import(ImportFormat::Ust, raw.as_bytes(), &ImportOptions::default()).unwrap();
    let note = &result.project.tracks[0].notes[0];
    assert_eq!(note.midi_note, 69);
    assert_eq!(note.curves.pitch_millicent_points[0].millicents, 75_000);
    assert!(result.report.fields.iter().any(|f| f.location.ends_with("/mode2_pitch") && f.disposition == FieldDisposition::Translated));
    assert!(result.report.unsupported_fields().any(|f| f.location.ends_with("/Flags") && f.source_value == "g-5"));
    assert!(result.report.unsupported_fields().any(|f| f.location.ends_with("/Envelope")));
}

#[test]
fn out_of_note_bends_are_reported_with_original_data_and_rest_controls_are_not_claimed() {
    let raw = ustx(&one_note("    pitch: {snap_first: false, data: [{x: -50, y: 10, shape: io}, {x: 50, y: 0, shape: io}]}\n"), "");
    let result = parse_ustx(&raw);
    assert!(result.report.notices.iter().any(|n| n.code == "pitch_outside_note"));
    assert_eq!(result.report.fields.iter().find(|f| f.location.ends_with("/pitch")).unwrap().source_value["data"][0]["x"], -50);
    let rest = raw.replace("lyric: ma", "lyric: R");
    let silent = parse_ustx(&rest);
    assert!(silent.project.tracks[0].notes.is_empty());
    assert!(silent.report.fields.iter().any(|f| f.location.ends_with("/pitch") && f.disposition == FieldDisposition::RetainedUnsupported));
}

#[test]
fn zero_width_bend_knots_keep_the_right_hand_value_and_backward_time_is_rejected() {
    let note = one_note("    pitch: {snap_first: false, data: [{x: 0, y: 0, shape: l}, {x: 0, y: 10, shape: l}, {x: 500, y: 10, shape: l}]}\n");
    let result = parse_ustx(&ustx(&note, ""));
    assert_eq!(result.project.tracks[0].notes[0].curves.pitch_millicent_points[0].millicents, 100_000);
    let backwards = note.replace("x: 500", "x: -1");
    assert!(import(ImportFormat::Ustx, ustx(&backwards, "").as_bytes(), &ImportOptions::default()).is_err());
    let source = ust(&ust_note(0, "ma", 480, 60, "PBS=0;0\nPBW=0,500\nPBY=10,10\nPBM=s,s\n"));
    let imported = import(ImportFormat::Ust, source.as_bytes(), &ImportOptions::default()).unwrap();
    assert_eq!(imported.project.tracks[0].notes[0].curves.pitch_millicent_points[0].millicents, 100_000);
}

#[test]
fn changed_pitch_continuation_retains_each_notes_f0_dyn_identity_and_rejects_a_gap() {
    let notes = "  - position: 0
    duration: 240
    tone: 60
    lyric: ma
    pitch: {snap_first: false, data: [{x: 0, y: 5, shape: l}, {x: 250, y: 5, shape: l}]}
  - position: 240
    duration: 240
    tone: 64
    lyric: '+'
    pitch: {snap_first: false, data: [{x: 0, y: -3, shape: l}, {x: 250, y: -3, shape: l}]}
";
    let curves = "  curves:\n  - {abbr: dyn, xs: [0, 239, 240, 480], ys: [0, 0, -60, -60]}\n";
    let raw = ustx(notes, curves);
    let options = ImportOptions { lyric_mode: LyricMode::Pinyin, ..Default::default() };
    let result = import(ImportFormat::Ustx, raw.as_bytes(), &options).unwrap();
    let imported = &result.project.tracks[0].notes;
    assert_eq!(imported.len(), 2);
    assert_eq!((imported[0].start_tick, imported[0].duration_tick, imported[0].midi_note), (0, 480, 60));
    assert_eq!((imported[1].start_tick, imported[1].duration_tick, imported[1].midi_note), (480, 480, 64));
    assert_eq!(imported[0].curves.pitch_millicent_points[0].millicents, 50_000);
    assert_eq!(imported[1].curves.pitch_millicent_points[0].millicents, -30_000);
    assert_eq!(imported[0].curves.gain_millidecibel_points[0].millidecibels, 0);
    assert_eq!(imported[1].curves.gain_millidecibel_points[0].millidecibels, -6000);
    assert_eq!(imported[1].unknown["continuation"], true);
    assert_eq!(imported[0].pronunciation.pinyin, "ma");
    assert_eq!(imported[1].pronunciation, verdant_domain::Pronunciation::default());
    assert_ne!(imported[0].id, imported[1].id);
    assert_eq!(imported[1].unknown["source_note_ids"], serde_json::json!(["t0/p0/n1"]));
    assert_eq!(result.report.events[1].project_note_id.as_ref(), Some(&imported[1].id));
    assert_eq!(result.report.events[1].continuation_of_project_note_id.as_ref(), Some(&imported[0].id));
    assert_eq!(result.report.events[1].source_midi_note, Some(64));
    assert_eq!(result.report.events[1].source_start_tick, 240);
    assert_eq!(result.report.events[1].source_duration_tick, 240);
    assert_eq!(result.report.events[1].reading, verdant_import::ReadingDisposition::Continuation);
    assert_eq!(result.report.tracks[0].source_event_count, 2);
    assert_eq!(result.report.tracks[0].project_note_count, 2);
    assert_eq!(result.report.tracks[0].continuation_count, 1);
    assert_eq!(result.report.unresolved_readings().count(), 0);
    let gapped = raw.replace("  - position: 240", "  - position: 241");
    let error = import(ImportFormat::Ustx, gapped.as_bytes(), &options).unwrap_err().to_string();
    assert!(error.contains("adjacent"), "{error}");
}

#[test]
fn ust_meter_tuples_use_preceding_bar_lengths_and_retain_the_original_setting() {
    let signatures = "(3/8/0),(5/4/2),(7/8/3),";
    let raw = ust(&ust_note(0, "ma", 4800, 60, ""))
        .replace("Tracks=1", &format!("Tracks=1\nTimeSignatures={signatures}"));
    let result = import(ImportFormat::Ust, raw.as_bytes(), &ImportOptions::default()).unwrap();
    assert_eq!(result.report.source.ppq, 480);
    assert_eq!(result.report.source_time_signatures.iter().map(|s| (s.tick, s.numerator, s.denominator)).collect::<Vec<_>>(),
        [(0, 3, 8), (1440, 5, 4), (3840, 7, 8)]);
    assert_eq!(result.project.time_signatures.iter().map(|s| (s.tick, s.numerator, s.denominator)).collect::<Vec<_>>(),
        [(0, 3, 8), (2880, 5, 4), (7680, 7, 8)]);
    assert_eq!(result.project.tracks[0].notes[0].duration_tick, 9600);
    assert_eq!(result.project.tempo_map[0].micro_bpm, 120_000_000);
    let retained = result.report.fields.iter().find(|f| f.location == "/#SETTING/TimeSignatures").unwrap();
    assert_eq!(retained.source_value, signatures);
    assert_eq!(retained.disposition, FieldDisposition::Translated);
    assert_eq!(retained.target.as_deref(), Some("project.time_signatures"));
    assert!(!result.report.unsupported_fields().any(|f| f.location == "/#SETTING/TimeSignatures"));
}

#[test]
fn ust_meter_tuple_syntax_order_fractional_boundaries_and_overflow_are_errors() {
    for signatures in [
        "", "4/4/0", "(4/4)", "(4/4/0/1)", "(4/4/0),,",
        "(4/4/1)", "(4/4/0),(3/4/0)", "(4/4/0),(3/4/2),(4/4/1)",
        "(4/4/-1)", "(0/4/0)", "(4/3/0)", "(4/0/0)", "(3.5/4/0)",
        "(4294967296/4/0)", "(4/4294967296/0)",
        "(1/4096/0),(4/4/1)", "(4/4/0),(4/4/9223372036854775807)",
    ] {
        let raw = ust(&ust_note(0, "ma", 480, 60, ""))
            .replace("Tracks=1", &format!("Tracks=1\nTimeSignatures={signatures}"));
        let error = import(ImportFormat::Ust, raw.as_bytes(), &ImportOptions::default()).unwrap_err().to_string();
        assert!(error.contains("TimeSignatures"), "{signatures:?}: {error}");
    }
}
