use serde_json::{Value, json};
use verdant_backend::adapter::{payload_for, plan_all_desktop, render_digest};
use verdant_backend::{Capabilities, Phrase, PhrasePlan, Request, Route, plan};
use verdant_backend::phonemizer::{self, PhonemizerSettings};
use verdant_domain::{Note, PhonemeTimingSettings, Project, Pronunciation, PronunciationSource, TempoMap};
use verdant_domain::model::TempoPoint;

fn caps() -> Capabilities {
    serde_json::from_value(json!({
        "protocol":"ogd-worker/1", "test_double":true, "routes":["verdant-voice"], "max_phrase_seconds":30,
        "native_score":{"inventory":["a","ma","shi","gu"], "english_vowel_proxies":["a"],
            "continuous_f0_control":true,"sub_syllable_timing":true,"phoneme_duration_control":true,
            "explicit_f0_hz_range":[35,2000],
            "phoneme_frontend":{"schema_version":"verdant-phonemes/2","alphabets":["ipa","xsampa"],"languages":{"zh":"Chinese","ja":"Japanese","en":"English"}},
            "phoneme_timing":{"methods":["vowel-anchor-v1"],"routes":["verdant-voice"],"timing_resolution_seconds":0.005,
                "constrained_nucleus_timing":true}},
        
    })).unwrap()
}

fn cv(start: i64, duration: i64, midi: i32, pinyin: &str) -> Note {
    let mut note = Note::new(start, duration, midi);
    note.lyric = pinyin.into();
    note.pronunciation = Pronunciation { pinyin:pinyin.into(), source:PronunciationSource::Manual, ..Default::default() };
    note
}

fn project(notes: Vec<Note>) -> Project {
    let mut project = Project::new_empty();
    project.tempo_map[0].micro_bpm = 120_000_000;
    project.tracks[0].notes = notes;
    project.tracks[0].refresh_index();
    project.render_preferences.phoneme_timing = Some(PhonemeTimingSettings::vowel_anchor_v1());
    project
}

fn planned(project: &Project, caps: &Capabilities) -> PhrasePlan {
    let result = plan(project, &project.tracks[0].id, Route::VerdantVoice, caps);
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    result
}

fn near(a: f64,b: f64) { assert!((a-b).abs() < 1e-10,"{a:.15} != {b:.15}"); }

fn assert_timing_geometry(project: &Project, plan: &PhrasePlan) {
    let track = &project.tracks[0];
    let tempo = TempoMap::new(&project.tempo_map, project.ppq).unwrap();
    assert!(plan.blocked.is_empty(), "{:?}", plan.blocked);
    assert!(plan.phrases.iter().flat_map(|p| &p.note_ids).eq(track.notes.iter().map(|n| &n.id)));
    for pair in plan.phrases.windows(2) {
        assert!(pair[0].start_seconds + pair[0].seconds <= pair[1].start_seconds + 1e-10);
    }
    for phrase in &plan.phrases {
        assert!(phrase.start_seconds >= 0.0);
        let mut starts = vec![0.0];
        for segment in &phrase.score.segments {
            assert!(segment.duration_seconds > 0.0 && segment.gap_after_seconds >= 0.0);
            starts.push(starts.last().unwrap() + segment.duration_seconds + segment.gap_after_seconds);
        }
        near(*starts.last().unwrap(), phrase.seconds);
        let traces = &phrase.score.phoneme_timing.as_ref().unwrap().notes;
        assert_eq!(traces.len(), phrase.note_ids.len());
        let mut next_segment = 0;
        for (id, trace) in phrase.note_ids.iter().zip(traces) {
            let note = track.note(id).unwrap();
            assert_eq!(trace.note_id, id.as_str());
            near(phrase.start_seconds + trace.note_onset_seconds, tempo.seconds_at(note.start_tick as f64));
            near(phrase.start_seconds + trace.note_end_seconds, tempo.seconds_at(note.end_tick() as f64));
            let first = trace.first_segment_index;
            let end = trace.end_segment_index;
            assert_eq!(first, next_segment);
            assert!(first < end && end <= phrase.score.segments.len());
            assert!(trace.prefix_seconds >= 0.0 && trace.tail_trim_seconds >= 0.0 && trace.nucleus_delay_seconds >= 0.0);
            near(starts[first], trace.note_onset_seconds - trace.prefix_seconds);
            if let Some(nucleus) = trace.nucleus_segment_index {
                assert!((first..end).contains(&nucleus));
                near(starts[nucleus], trace.note_onset_seconds + trace.nucleus_delay_seconds);
            } else {
                near(trace.prefix_seconds, 0.0);
                near(trace.nucleus_delay_seconds, 0.0);
            }
            near(starts[end] - phrase.score.segments[end - 1].gap_after_seconds, trace.note_end_seconds - trace.tail_trim_seconds);
            next_segment = end;
        }
        assert_eq!(next_segment, phrase.score.segments.len());
    }
}

fn assert_anchor_geometry(project: &Project, plan: &PhrasePlan) {
    assert_timing_geometry(project, plan);
    for phrase in &plan.phrases {
        for trace in &phrase.score.phoneme_timing.as_ref().unwrap().notes {
            assert!(trace.nucleus_segment_index.is_some());
            near(trace.nucleus_delay_seconds, 0.0);
        }
    }
}

fn assert_voiced_pitch(phrase: &Phrase, relative_seconds: f64, expected_hz: f64) {
    let control = phrase.control_track.as_ref().unwrap();
    let frame = control.frame_times_seconds.iter()
        .position(|&time| time == relative_seconds).expect("sample on the authored control clock");
    assert!(!control.uv[frame]);
    near(control.f0_hz[frame], expected_hz);
}

#[test]
fn prefixes_use_rests_then_previous_vowel_tail_while_midi_geometry_stays_exact() {
    let p = project(vec![cv(960,960,60,"ma1"),cv(1920,480,64,"shi1"),cv(2880,960,67,"gu1")]);
    let before = p.clone();
    let result = planned(&p,&caps());
    assert_eq!(p,before);
    assert_eq!(result.phrases.len(),1);
    let phrase = &result.phrases[0];
    near(phrase.start_seconds,0.39);
    let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes;
    near(trace[0].prefix_seconds,0.11);
    near(trace[0].tail_trim_seconds,0.055);
    near(trace[1].tail_trim_seconds,0.0);
    near(phrase.score.segments[3].gap_after_seconds,0.14);
    assert_anchor_geometry(&p,&result);
    // The nasal lead before the first MIDI note is genuinely voiced; the
    // authored note and its PIT/vibrato origin have not moved to 0.39 s.
    let control = phrase.control_track.as_ref().unwrap();
    assert!(!control.uv[0]);
    near(control.f0_hz[0],261.626);
}

#[test]
fn explicit_phone_budget_stays_on_the_consonant_and_disabling_is_exact() {
    let mut first = cv(960,960,60,"ma1");
    first.pronunciation.phoneme_durations_permille = Some(vec![400,600]);
    let mut p = project(vec![first,cv(1920,960,60,"gu1")]);
    p.render_preferences.phoneme_timing = None;
    let old = planned(&p,&caps());
    let old_payload = payload_for(&old.phrases[0],Route::VerdantVoice,10008);
    assert!(serde_json::to_value(&old_payload).unwrap()["score"].get("phoneme_timing").is_none());
    p.render_preferences.phoneme_timing = Some(PhonemeTimingSettings::vowel_anchor_v1());
    let new = planned(&p,&caps());
    near(new.phrases[0].score.segments[0].duration_seconds,0.2);
    near(new.phrases[0].score.segments[2].duration_seconds,0.11);
    assert_anchor_geometry(&p,&new);
    assert_ne!(render_digest(&old_payload,Route::VerdantVoice,&caps()),render_digest(&payload_for(&new.phrases[0],Route::VerdantVoice,10008),Route::VerdantVoice,&caps()));
    p.render_preferences.phoneme_timing = None;
    let restored = planned(&p,&caps());
    assert_eq!(payload_for(&restored.phrases[0],Route::VerdantVoice,10008),old_payload);
}

#[test]
fn score_start_keeps_the_weighted_consonant_and_reports_the_actual_vowel_onset() {
    for start in [0, 48] {
        let mut note = cv(start, 960, 69, "ma1");
        note.pronunciation.phoneme_durations_permille = Some(vec![400, 600]);
        let p = project(vec![note]);
        let result = planned(&p, &caps());
        assert_timing_geometry(&p, &result);
        let phrase = &result.phrases[0];
        let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes[0];
        let segments = &phrase.score.segments;
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].context.as_ref().unwrap().features["C_SM"], "m");
        assert_eq!(segments[1].context.as_ref().unwrap().features["C_YM"], "a");
        assert_eq!(trace.nucleus_segment_index, Some(1));
        near(segments[0].duration_seconds, 0.2);
        near(trace.prefix_seconds + trace.nucleus_delay_seconds, 0.2);
        near(trace.tail_trim_seconds, 0.0);
        assert!(trace.nucleus_delay_seconds > 0.0);
        near(trace.prefix_seconds, start as f64 / 1920.0);
        assert_voiced_pitch(phrase, trace.note_onset_seconds, 440.0);
    }
}

#[test]
fn a_minimum_length_predecessor_keeps_its_vowel_and_the_following_note_keeps_its_pitch() {
    let mut a = cv(960, 48, 60, "a");
    a.pronunciation.frontend_profile = "english-vowel-proxy-native-0.2".into();
    let mut ma = cv(1008, 960, 64, "ma1");
    ma.portamento_ms = Some(0);
    let p = project(vec![a, ma]);
    let result = planned(&p, &caps());
    assert_timing_geometry(&p, &result);
    assert_eq!(result.phrases.len(), 1);
    let phrase = &result.phrases[0];
    let traces = &phrase.score.phoneme_timing.as_ref().unwrap().notes;
    let segments = &phrase.score.segments;
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].vowel.as_deref(), Some("a"));
    assert_eq!(segments[1].context.as_ref().unwrap().features["C_SM"], "m");
    assert_eq!(segments[2].context.as_ref().unwrap().features["C_YM"], "a");
    assert_eq!(traces[0].nucleus_segment_index, Some(0));
    near(segments[0].duration_seconds, 0.025);
    near(traces[0].tail_trim_seconds, 0.0);
    assert_eq!(traces[1].nucleus_segment_index, Some(2));
    near(traces[1].prefix_seconds, 0.0);
    near(traces[1].nucleus_delay_seconds, segments[1].duration_seconds);
    assert!(segments.iter().all(|s| s.duration_seconds >= 0.025 - 1e-10));
    assert_voiced_pitch(phrase, traces[0].note_onset_seconds, 261.626);
    assert_voiced_pitch(phrase, traces[1].note_onset_seconds, 329.628);
}

#[test]
fn a_short_coda_retains_its_budget_when_the_following_consonant_needs_a_lead() {
    let mut at = cv(960, 192, 60, "");
    at.pronunciation = Pronunciation { language: Some("en".into()), alphabet: Some("ipa".into()),
        phonemes: vec!["æ".into(), "t".into()], phoneme_durations_permille: Some(vec![500, 500]),
        source: PronunciationSource::Manual, ..Default::default() };
    let mut ma = cv(1152, 960, 64, "ma1");
    ma.pronunciation.phoneme_durations_permille = Some(vec![300, 700]);
    let p = project(vec![at, ma]);
    for (compact, floor) in [(false, 0.025), (true, 0.005)] {
        let mut c = caps();
        c.native_score.as_mut().unwrap().compact_phone_states = compact;
        let result = planned(&p, &c);
        assert_timing_geometry(&p, &result);
        assert_eq!(result.phrases.len(), 1);
        let phrase = &result.phrases[0];
        let segments = &phrase.score.segments;
        let traces = &phrase.score.phoneme_timing.as_ref().unwrap().notes;
        assert_eq!(segments.len(), 4);
        assert_eq!(traces[0].nucleus_segment_index, Some(0));
        assert_eq!(traces[1].nucleus_segment_index, Some(3));
        assert_eq!(segments[1].phoneme.as_ref().unwrap().native_key, "en:t");
        near(segments[0].duration_seconds, floor);
        near(segments[1].duration_seconds, 0.05);
        near(segments[2].duration_seconds, 0.15);
        near(traces[0].tail_trim_seconds, 0.05 - floor);
        near(traces[1].prefix_seconds, traces[0].tail_trim_seconds);
        near(traces[1].nucleus_delay_seconds, 0.15 - traces[1].prefix_seconds);
    }
}

#[test]
fn phrase_splits_preserve_global_phone_boundaries_and_portamento_context() {
    let p = project(vec![cv(960,960,60,"ma1"),cv(1920,960,64,"gu1"),cv(2880,960,67,"shi1")]);
    let large = planned(&p,&caps());
    let mut short_caps = caps();
    short_caps.max_phrase_seconds = 1.0;
    let split = planned(&p,&short_caps);
    assert!(split.phrases.len()>1);
    assert_anchor_geometry(&p,&split);
    for pair in split.phrases.windows(2) { near(pair[0].start_seconds+pair[0].seconds,pair[1].start_seconds); }
    let geometry = |plan:&PhrasePlan| plan.phrases.iter().flat_map(|phrase| {
        phrase.score.phoneme_timing.as_ref().unwrap().notes.iter().map(move |note|
            (note.note_id.clone(),phrase.start_seconds+note.note_onset_seconds-note.prefix_seconds,
             phrase.start_seconds+note.note_end_seconds-note.tail_trim_seconds))
    }).collect::<Vec<_>>();
    for (a,b) in geometry(&large).into_iter().zip(geometry(&split)) { assert_eq!(a.0,b.0);near(a.1,b.1);near(a.2,b.2); }
    // Compare at an exact shared absolute boundary inside the cross-job glide.
    let boundary = 1.5;
    for plan in [&large,&split] {
        let phrase = plan.phrases.iter().find(|phrase|phrase.start_seconds<=boundary && boundary<phrase.start_seconds+phrase.seconds).unwrap();
        let control = phrase.control_track.as_ref().unwrap();
        let index = control.frame_times_seconds.iter().position(|t|(*t-(boundary-phrase.start_seconds)).abs()<1e-12).unwrap();
        let cents = verdant_backend::f0::note_cents_at_with_next(&p.tracks[0].notes[2],Some(&p.tracks[0].notes[1]),None,&TempoMap::constant(120.0),boundary).unwrap();
        assert!((control.f0_hz[index]-verdant_backend::f0::cents_to_hz(cents)).abs()<0.000501);
    }
}

#[test]
fn tempo_change_is_integrated_before_anchors_and_does_not_round_back_to_ticks() {
    let mut p = project(vec![cv(960,960,60,"ma1"),cv(1920,960,64,"gu1")]);
    p.tempo_map.push(TempoPoint {tick:1440,micro_bpm:183_000_000,unknown:Default::default()});
    let result = planned(&p,&caps());
    assert_anchor_geometry(&p,&result);
    let prefix = result.phrases[0].score.phoneme_timing.as_ref().unwrap().notes[0].prefix_seconds;
    let expected_duration: f64 = 0.25+30.0/183.0;
    near(prefix,(expected_duration*0.22*1e6).round()/1e6);
    near(result.phrases[0].start_seconds,0.5-prefix);
    assert_ne!(result.phrases[0].start_seconds,0.39);
}

#[test]
fn old_workers_keep_exact_on_beat_phrases_and_block_only_the_group_needing_extended_nuclei() {
    let mut modern = continuous_caps();
    modern.native_score.as_mut().unwrap().compact_phone_states = true;
    let mut wire = serde_json::to_value(&modern).unwrap();
    let timing = wire["native_score"]["phoneme_timing"].as_object_mut().unwrap();
    assert_eq!(timing.remove("constrained_nucleus_timing"), Some(json!(true)));
    let legacy: Capabilities = serde_json::from_value(wire).unwrap();
    assert!(!legacy.native_score.as_ref().unwrap().phoneme_timing.as_ref().unwrap().constrained_nucleus_timing);

    let mut stable = cv(1000, 500, 60, "ma1");
    stable.pronunciation.phoneme_durations_permille = Some(vec![250, 750]);
    // This 5 ms vowel keeps its one native frame. The following 20 ms CV
    // must put its entire 8 ms consonant after the beat.
    let vowel = cv(3000, 5, 60, "a1");
    let mut delayed = cv(3005, 20, 60, "ma1");
    delayed.pronunciation.phoneme_durations_permille = Some(vec![400, 600]);
    let mut consonant = cv(3025, 50, 60, "s");
    consonant.pronunciation = Pronunciation { language: Some("en".into()), alphabet: Some("ipa".into()),
        phonemes: vec!["s".into()], source: PronunciationSource::Manual, ..Default::default() };
    let mut p = project(vec![stable, vowel, delayed, consonant]);
    p.ppq = 1000;
    p.tempo_map[0].micro_bpm = 60_000_000;
    let track = &p.tracks[0];
    let current = planned(&p, &modern);
    let [ordinary, mixed] = current.phrases.as_slice() else { panic!("expected two acoustic groups") };
    assert_timing_geometry(&p, &current);
    let traces = serde_json::to_value(&mixed.score.phoneme_timing.as_ref().unwrap().notes).unwrap();
    assert_eq!(traces[0]["nucleus_segment_index"], 0);
    assert!(traces[0].get("nucleus_delay_seconds").is_none());
    assert_eq!(traces[1]["nucleus_segment_index"], 2);
    near(traces[1]["nucleus_delay_seconds"].as_f64().unwrap(), 0.008);
    assert_eq!(traces[2].get("nucleus_segment_index"), Some(&Value::Null));

    let old = plan(&p, &track.id, Route::VerdantVoice, &legacy);
    let [preserved] = old.phrases.as_slice() else { panic!("legacy worker should retain the on-beat group") };
    let [blocked] = old.blocked.as_slice() else { panic!("only the mixed acoustic group needs the extension") };
    assert_eq!(blocked.note_ids, mixed.note_ids);
    assert_eq!((blocked.start_tick, blocked.end_tick), (3000, 3075));
    let errors: Vec<_> = old.diagnostics.iter().filter(|d| d.code == "E_PHONEME_TIMING_CAPABILITY").collect();
    assert_eq!(errors.len(), 2);
    for (error, note) in errors.iter().zip(&track.notes[2..]) {
        assert_eq!(error.note_id.as_ref(), Some(&note.id));
        assert!(error.message.contains("新版合成后端"));
        assert!(error.detail.contains("constrained_nucleus_timing"));
    }
    let payload = payload_for(preserved, old.route, 10008);
    assert_eq!(payload, payload_for(ordinary, current.route, 10008));
    let trace = serde_json::to_value(&payload).unwrap();
    assert_eq!(trace["score"]["phoneme_timing"]["notes"], json!([{
        "note_id":track.notes[0].id, "note_onset_seconds":0.125, "note_end_seconds":0.625,
        "first_segment_index":0, "nucleus_segment_index":1, "end_segment_index":2,
        "prefix_seconds":0.125, "tail_trim_seconds":0.0
    }]));
}

#[test]
fn a_weighted_lead_filling_the_opening_rest_keeps_its_legacy_trace() {
    // 42.5% of 720 ticks is exactly 306 ticks. Subtracting the absolute
    // times in seconds makes the weighted prefix one ULP longer than the rest.
    let mut note = cv(306, 720, 60, "ma1");
    note.pronunciation.phoneme_durations_permille = Some(vec![425, 575]);
    let p = project(vec![note]);
    for supported in [false, true] {
        let mut c = caps();
        c.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().constrained_nucleus_timing = supported;
        let result = planned(&p, &c);
        let phrase = &result.phrases[0];
        assert_eq!(phrase.start_seconds, 0.0);
        let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes[0];
        assert_eq!(trace.nucleus_segment_index, Some(1));
        assert_eq!(trace.prefix_seconds, phrase.score.segments[0].duration_seconds);
        assert!(serde_json::to_value(trace).unwrap().get("nucleus_delay_seconds").is_none());
    }
}

#[test]
fn unknown_method_missing_support_or_wrong_clock_blocks_only_opted_in_projects() {
    for case in 0..6 {
        let mut p = project(vec![cv(960,960,60,"ma1")]);
        let mut caps = caps();
        match case {
            0 => caps.native_score.as_mut().unwrap().phoneme_timing=None,
            1 => caps.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().routes=vec!["C0".into()],
            2 => caps.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().timing_resolution_seconds=Some(0.01),
            3 => p.render_preferences.phoneme_timing.as_mut().unwrap().method="unrecognized".into(),
            4 => caps.native_score.as_mut().unwrap().continuous_f0_control=false,
            _ => caps.native_score.as_mut().unwrap().continuous_f0_control=false,
        }
        let result=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&caps);
        assert!(result.phrases.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic|diagnostic.code=="E_PHONEME_TIMING_CAPABILITY"));
        p.render_preferences.phoneme_timing=None;
        planned(&p,&caps);
    }
}

#[test]
fn ipa_coda_budget_is_preserved_and_consonant_only_notes_have_no_invented_nucleus() {
    let mut word=cv(960,960,60,"");
    word.pronunciation=Pronunciation {
        language:Some("en".into()),alphabet:Some("ipa".into()),phonemes:vec!["k".into(),"æ".into(),"t".into()],
        source:PronunciationSource::Manual,..Default::default()
    };
    let mut p=project(vec![word]);
    p.render_preferences.phoneme_timing=None;
    let before=planned(&p,&caps());
    let before=&before.phrases[0].score.segments;
    p.render_preferences.phoneme_timing=Some(PhonemeTimingSettings::vowel_anchor_v1());
    let after=planned(&p,&caps());
    let segments=&after.phrases[0].score.segments;
    assert_eq!(segments.len(),3);
    near(segments[0].duration_seconds,before[0].duration_seconds);
    near(segments[2].duration_seconds,before[2].duration_seconds);
    assert!(segments[1].duration_seconds>before[1].duration_seconds);
    assert_anchor_geometry(&p,&after);
    let reading = &mut p.tracks[0].notes[0].pronunciation;
    reading.phonemes = vec!["s".into(), "t".into()];
    reading.phoneme_durations_permille = Some(vec![350, 650]);
    let consonants = planned(&p, &caps());
    assert_timing_geometry(&p, &consonants);
    let phrase = &consonants.phrases[0];
    let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes[0];
    assert_eq!(trace.nucleus_segment_index, None);
    near(trace.tail_trim_seconds, 0.0);
    assert_eq!(phrase.score.segments.len(), 2);
    for (segment, (native, duration)) in phrase.score.segments.iter().zip([("en:s", 0.175), ("en:t", 0.325)]) {
        assert_eq!(segment.phoneme.as_ref().unwrap().native_key, native);
        assert!(!segment.voiced);
        near(segment.duration_seconds, duration);
    }
}

#[test]
fn desktop_requests_keep_the_moraic_nasal_geminate_and_separate_phrase_on_one_track() {
    let notes = [("か", 0, 384, 60), ("ん", 384, 384, 64), ("か", 768, 480, 65),
        ("っ", 1248, 288, 62), ("か", 1536, 960, 67), ("ま", 4800, 960, 69)];
    let mut p = project(notes.into_iter().map(|(lyric, start, duration, midi)| {
        let mut note = Note::new(start, duration, midi);
        note.lyric = lyric.into();
        note
    }).collect());
    let settings = PhonemizerSettings { selected: phonemizer::JAPANESE_KANA.into(), ..Default::default() };
    p.extensions.insert(phonemizer::EXTENSION_KEY.into(), phonemizer::settings_to_extension(&settings).unwrap());
    let mut c = caps();
    c.other.insert("voice_package".into(), json!({"music":{"methods":["verdant-voice-music-v1"],
        "routes":["verdant-voice"],"sample_rates":[48000],"default_sample_rate":48000}}));
    let before = p.clone();
    let plans = plan_all_desktop(&p, &p.tracks[0].id, &c);
    let [plan] = plans.as_slice() else { panic!("expected the project's single vocal track") };
    assert!(!plan.has_errors(), "{:?}", plan.diagnostics);
    assert_timing_geometry(&p, plan);
    let [mixed, sibling] = plan.phrases.as_slice() else { panic!("the written rest separates two phrases") };
    assert_eq!(mixed.note_ids.len(), 5);
    let request = Request::render("morae", "morae", p.revision, plan.route, payload_for(mixed, plan.route, 10008));
    let wire: Value = serde_json::from_str(&request.to_line().unwrap()).unwrap();
    let score = &wire["payload"]["score"];
    let traces = score["phoneme_timing"]["notes"].as_array().unwrap();
    let segments = score["segments"].as_array().unwrap();
    let phones = [(0, "k", "zh:g", false), (0, "a", "zh:a", true), (1, "ɴ", "en:ng", true),
        (2, "k", "zh:g", false), (2, "a", "zh:a", true), (3, "kː", "zh:g", false),
        (4, "k", "zh:g", false), (4, "a", "zh:a", true)];
    assert_eq!(segments.len(), phones.len());
    for (segment, (owner, ipa, native, voiced)) in segments.iter().zip(phones) {
        let phone = &segment["phoneme"];
        let note = &p.tracks[0].notes[owner];
        assert_eq!(phone["source_note_id"], note.id.as_str());
        assert_eq!(phone["canonical_ipa"], ipa);
        assert_eq!(phone["native_key"], native);
        assert_eq!(segment["voiced"], voiced);
        near(segment["midi_note"].as_f64().unwrap(), f64::from(note.midi_note));
    }
    for (owner, index, duration) in [(1, 2, 0.2), (3, 5, 0.15)] {
        assert_eq!(traces[owner].get("nucleus_segment_index"), Some(&Value::Null));
        assert!(traces[owner].get("nucleus_delay_seconds").is_none());
        near(segments[index]["duration_seconds"].as_f64().unwrap(), duration);
        near(traces[owner]["tail_trim_seconds"].as_f64().unwrap(), 0.0);
    }
    // Each /ka/ starts at score zero or immediately after a complete consonant-only mora.
    for (owner, nucleus) in [(0, 1), (2, 4), (4, 7)] {
        assert_eq!(traces[owner]["nucleus_segment_index"], json!(nucleus));
        assert!(traces[owner]["nucleus_delay_seconds"].as_f64().unwrap() > 0.0);
    }
    assert_voiced_pitch(mixed, 0.3, 329.628);

    let mut isolated = p.clone();
    isolated.tracks[0].notes = vec![p.tracks[0].notes[5].clone()];
    isolated.tracks[0].refresh_index();
    let only = plan_all_desktop(&isolated, &isolated.tracks[0].id, &c);
    assert!(!only[0].has_errors(), "{:?}", only[0].diagnostics);
    assert_anchor_geometry(&isolated, &only[0]);
    let ordinary = &only[0].phrases[0];
    assert_eq!(sibling.note_ids, ordinary.note_ids);
    near(sibling.start_seconds, ordinary.start_seconds);
    near(sibling.seconds, ordinary.seconds);
    assert_eq!(sibling.score.segments, ordinary.score.segments);
    assert_eq!(sibling.score.phoneme_timing, ordinary.score.phoneme_timing);
    assert_eq!(sibling.control_track, ordinary.control_track);
    let ordinary_wire = serde_json::to_value(payload_for(sibling, plan.route, 10008)).unwrap();
    let ordinary_trace = &ordinary_wire["score"]["phoneme_timing"]["notes"][0];
    assert_eq!(ordinary_trace["nucleus_segment_index"], 1);
    assert!(ordinary_trace.get("nucleus_delay_seconds").is_none());
    assert_eq!(p, before);
}

#[test]
fn a_single_note_uses_the_available_lead_within_its_request_limit() {
    let mut c = caps();
    c.max_phrase_seconds = 1.0;
    for duration in [1824, 1920] {
        let mut note = cv(960, duration, 60, "ma1");
        note.pronunciation.phoneme_durations_permille = Some(vec![200, 800]);
        let p = project(vec![note]);
        let result = planned(&p, &c);
        assert_timing_geometry(&p, &result);
        let phrase = &result.phrases[0];
        let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes[0];
        let note_seconds = duration as f64 / 1920.0;
        near(phrase.seconds, 1.0);
        near(trace.prefix_seconds, 1.0 - note_seconds);
        near(trace.tail_trim_seconds, 0.0);
        assert_eq!(trace.nucleus_segment_index, Some(1));
        near(phrase.score.segments[0].duration_seconds, 0.2 * note_seconds);
        near(trace.nucleus_delay_seconds, 0.2 * note_seconds - trace.prefix_seconds);
    }
}

#[test]
fn a_duration_split_keeps_the_root_and_its_continuation_in_one_request() {
    let mut continuation = Note::new(2784, 480, 67);
    continuation.lyric = "+".into();
    let p = project(vec![cv(960, 1344, 60, "a1"), cv(2304, 480, 64, "ma1"), continuation]);
    let mut c = caps();
    c.max_phrase_seconds = 1.0;
    let result = planned(&p, &c);
    assert_anchor_geometry(&p, &result);
    let [first, chain] = result.phrases.as_slice() else { panic!("the three notes exceed one request") };
    assert_eq!(first.note_ids.len(), 1);
    assert_eq!(chain.note_ids.len(), 2);
    assert!(first.seconds <= c.max_phrase_seconds && chain.seconds <= c.max_phrase_seconds);
    let trace = &chain.score.phoneme_timing.as_ref().unwrap().notes;
    assert_eq!(trace[0].nucleus_segment_index, Some(1));
    assert_eq!(trace[1].nucleus_segment_index, Some(2));
    assert_eq!(chain.score.segments.len(), 3);
    assert_eq!(chain.score.segments[2].continuation_of, Some(1));
    assert_eq!(chain.score.segments[2].midi_note, Some(67.0));
}

#[test]
fn a_bare_vowel_below_the_native_phone_floor_blocks_only_its_own_phrase() {
    // A 12.5 ms vowel has no preceding consonant that could move into the rest.
    let p = project(vec![cv(0, 960, 60, "ma1"), cv(3840, 24, 64, "a1"), cv(7680, 960, 67, "gu1")]);
    let track = &p.tracks[0];
    let c = caps();
    let plans = plan_all_desktop(&p, &track.id, &c);
    let plan = &plans[0];
    let short = &track.notes[1];
    assert!(plan.errors_for(&short.id).any(|d| d.code == "E_NOTE_TOO_SHORT"));
    let [blocked] = plan.blocked.as_slice() else { panic!("only the 12.5 ms vowel is below the 25 ms floor") };
    assert_eq!(blocked.note_ids.as_slice(), std::slice::from_ref(&short.id));
    assert_eq!((blocked.start_tick, blocked.end_tick), (short.start_tick, short.end_tick()));
    let [opening, ending] = plan.phrases.as_slice() else { panic!("both neighboring phrases must render") };
    assert_eq!(opening.note_ids.as_slice(), std::slice::from_ref(&track.notes[0].id));
    assert_eq!(ending.note_ids.as_slice(), std::slice::from_ref(&track.notes[2].id));
    let opening_trace = &opening.score.phoneme_timing.as_ref().unwrap().notes[0];
    assert_eq!(opening_trace.nucleus_segment_index, Some(1));
    assert!(opening_trace.nucleus_delay_seconds > 0.0);
    let ending_trace = &ending.score.phoneme_timing.as_ref().unwrap().notes[0];
    assert_eq!(ending_trace.nucleus_segment_index, Some(1));
    near(ending_trace.nucleus_delay_seconds, 0.0);

    let mut compact = c;
    compact.native_score.as_mut().unwrap().compact_phone_states = true;
    let all = plan_all_desktop(&p, &track.id, &compact);
    assert!(!all[0].has_errors(), "{:?}", all[0].diagnostics);
    assert_timing_geometry(&p, &all[0]);
    assert_eq!(all[0].phrases.len(), 3);
    let recovered = &all[0].phrases[1];
    assert_eq!(recovered.note_ids.as_slice(), std::slice::from_ref(&short.id));
    assert_eq!(recovered.score.segments.len(), 1);
    near(recovered.score.segments[0].duration_seconds, 0.0125);
}

#[test]
fn a_muted_predecessor_frees_its_time_for_a_lead_but_a_blocked_phrase_is_a_barrier() {
    let mut first = cv(0, 960, 60, "ma1");
    first.set_muted(true);
    let mut p = project(vec![first, cv(960, 960, 69, "ma1")]);
    let mut c = caps();
    c.max_phrase_seconds = 0.7;
    let muted = planned(&p, &c);
    assert!(muted.blocked.is_empty());
    let [phrase] = muted.phrases.as_slice() else { panic!("only the second note should sound") };
    assert_eq!(phrase.note_ids.as_slice(), std::slice::from_ref(&p.tracks[0].notes[1].id));
    let [m, a] = phrase.score.segments.as_slice() else { panic!("expected the native initial and final") };
    assert_eq!(m.context.as_ref().unwrap().features["C_SM"], "m");
    assert_eq!(a.context.as_ref().unwrap().features["C_YM"], "a");
    assert!(m.voiced && a.voiced);
    near(phrase.start_seconds, 0.390);
    near(phrase.start_seconds + m.duration_seconds, 0.500);
    near(phrase.start_seconds + phrase.seconds, 1.000);
    let trace = &phrase.score.phoneme_timing.as_ref().unwrap().notes[0];
    assert_eq!(trace.nucleus_segment_index, Some(1));
    near(trace.prefix_seconds, 0.110);
    near(trace.nucleus_delay_seconds, 0.0);
    assert_voiced_pitch(phrase, 0.0, 440.0);

    // Identical note times, but now the first phrase contains an actual admission error.
    let first = &mut p.tracks[0].notes[0];
    first.set_muted(false);
    first.pronunciation.phoneme_durations_permille = Some(vec![0, 1000]);
    let with_barrier = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &c);
    assert!(with_barrier.errors_for(&p.tracks[0].notes[0].id).any(|d| d.code == "E_PHONEME_TIMING"));
    let [blocked] = with_barrier.blocked.as_slice() else { panic!("the invalid first phrase must stay located") };
    assert_eq!(blocked.note_ids.as_slice(), std::slice::from_ref(&p.tracks[0].notes[0].id));
    let [following] = with_barrier.phrases.as_slice() else { panic!("the second phrase must still render") };
    assert_eq!(following.note_ids, phrase.note_ids);
    near(following.start_seconds, 0.500);
    let trace = &following.score.phoneme_timing.as_ref().unwrap().notes[0];
    near(trace.prefix_seconds, 0.0);
    near(trace.nucleus_delay_seconds, 0.110);
}

#[test]
fn an_illegal_continuation_after_a_coda_keeps_the_valid_thirty_second_source_phrase() {
    let mut at = Note::new(0, 57_600, 64);
    at.lyric = "at".into();
    at.pronunciation = Pronunciation { language: Some("en".into()), alphabet: Some("ipa".into()),
        phonemes: vec!["æ".into(), "t".into()], source: PronunciationSource::Manual, ..Default::default() };
    let mut sustain = Note::new(at.end_tick(), 960, 64);
    sustain.lyric = "+".into();
    let mut p = project(vec![at]);
    let c = caps();
    let standalone = planned(&p, &c);
    let expected = &standalone.phrases[0];
    p.tracks[0].notes.push(sustain.clone());
    p.tracks[0].refresh_index();
    let joined = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &c);
    assert!(joined.errors_for(&sustain.id).any(|d| d.code == "E_CONTINUATION"));
    assert!(joined.errors_for(&p.tracks[0].notes[0].id).next().is_none());
    let [blocked] = joined.blocked.as_slice() else { panic!("only the illegal continuation should be blocked") };
    assert_eq!(blocked.note_ids.as_slice(), std::slice::from_ref(&sustain.id));
    let [actual] = joined.phrases.as_slice() else { panic!("the complete /æt/ source must render") };
    assert_eq!(actual.note_ids, expected.note_ids);
    assert_eq!(actual.score.segments, expected.score.segments);
    assert_eq!(actual.score.phoneme_timing, expected.score.phoneme_timing);
    let [vowel, coda] = actual.score.segments.as_slice() else { panic!("both source phones must remain") };
    assert_eq!(vowel.phoneme.as_ref().unwrap().native_key, "en:ae");
    assert_eq!(coda.phoneme.as_ref().unwrap().native_key, "en:t");
    near(actual.start_seconds, 0.0);
    near(actual.seconds, 30.0);
    near(vowel.duration_seconds + coda.duration_seconds, 30.0);
}

#[test]
fn a_chain_rejected_at_513_segments_cannot_borrow_the_preceding_vowels_tail() {
    let mut notes = vec![cv(0, 57_600, 60, "a1"), cv(57_600, 960, 64, "ma1")];
    for _ in 0..510 {
        let mut sustain = Note::new(notes.last().unwrap().end_tick(), 48, 64);
        sustain.lyric = "+".into();
        notes.push(sustain);
    }
    let mut p = project(notes);
    let c = caps();
    let admitted = planned(&p, &c);
    let [before, chain] = admitted.phrases.as_slice() else { panic!("the 512-segment chain fits its own request") };
    assert_eq!(chain.score.segments.len(), 512);
    assert!(chain.seconds < c.max_phrase_seconds);
    near(before.score.phoneme_timing.as_ref().unwrap().notes[0].tail_trim_seconds, 0.110);

    let track = &mut p.tracks[0];
    let mut extra = Note::new(track.notes.last().unwrap().end_tick(), 48, 64);
    extra.lyric = "+".into();
    track.notes.push(extra);
    track.refresh_index();
    let track = &p.tracks[0];
    let over = plan(&p, &track.id, Route::VerdantVoice, &c);
    assert!(over.has_errors() && over.errors_for(&track.notes[0].id).next().is_none());
    let [blocked] = over.blocked.as_slice() else { panic!("only the 513-segment chain should be blocked") };
    assert!(blocked.note_ids.iter().eq(track.notes[1..].iter().map(|note| &note.id)));
    assert!(blocked.note_ids.iter().any(|id| over.errors_for(id).any(|d| d.severity == verdant_backend::Severity::Error)));
    let [survivor] = over.phrases.as_slice() else { panic!("the preceding whole vowel must remain") };
    assert_eq!(survivor.note_ids.as_slice(), std::slice::from_ref(&track.notes[0].id));
    near(survivor.start_seconds, 0.0);
    near(survivor.seconds, 30.0);
    near(survivor.score.phoneme_timing.as_ref().unwrap().notes[0].tail_trim_seconds, 0.0);
    let [vowel] = survivor.score.segments.as_slice() else { panic!("expected one whole vowel") };
    assert_eq!(vowel.context.as_ref().unwrap().features["C_YM"], "a");
    near(vowel.duration_seconds, 30.0);
    near(vowel.gap_after_seconds, 0.0);
}

#[test]
fn following_note_budget_changes_the_previous_phrase_digest_across_a_hard_split() {
    let mut p=project(vec![cv(960,960,60,"ma1"),cv(1920,960,64,"gu1"),cv(2880,960,67,"shi1")]);
    let mut c=caps();c.max_phrase_seconds=0.7;
    let before=planned(&p,&c);
    assert_eq!(before.phrases.len(),3);
    let before_digest=render_digest(&payload_for(&before.phrases[0],Route::VerdantVoice,10008),Route::VerdantVoice,&c);
    p.tracks[0].notes[1].pronunciation.phoneme_durations_permille=Some(vec![400,600]);
    let after=planned(&p,&c);
    assert_eq!(after.phrases.len(),3);
    near(before.phrases[0].seconds-after.phrases[0].seconds,0.09);
    assert_ne!(before_digest,render_digest(&payload_for(&after.phrases[0],Route::VerdantVoice,10008),Route::VerdantVoice,&c));
    assert_anchor_geometry(&p,&after);
}

fn continuous_caps() -> Capabilities {
    let mut c=caps();
    c.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().continuity=Some(
        verdant_backend::wire::ContinuousRunsCaps { method:"continuous-runs-v1".into(),
            max_seconds:120.0,max_segments:2048,max_control_points:32768,minimum_silence_seconds:0.005 });
    c
}

#[test]
fn a_late_origin_120_second_request_meets_the_native_cumulative_duration_and_frame_limits() {
    // At 125 BPM and 960 PPQ, 10 ticks are one 5 ms frame. The large origin
    // makes subtraction of independently converted absolute boundaries inexact.
    let mut start = 2_048_000;
    let mut notes = Vec::new();
    for duration in std::iter::repeat_n(10, 25).chain([59_750, 60_000, 60_000, 60_000]) {
        notes.push(cv(start, duration, 64, "a1"));
        start += duration;
    }
    let mut p = project(notes);
    p.tempo_map[0].micro_bpm = 125_000_000;
    let mut c = continuous_caps();
    c.native_score.as_mut().unwrap().compact_phone_states = true;
    let result = planned(&p, &c);
    assert_anchor_geometry(&p, &result);
    let [phrase] = result.phrases.as_slice() else { panic!("all 29 contiguous vowels fit one continuous run") };
    near(phrase.start_seconds, 1024.0);
    near(phrase.seconds, 120.0);
    let request = Request::render("late-origin", "late-origin", p.revision, result.route,
        payload_for(phrase, result.route, 10008));
    let wire: Value = serde_json::from_str(&request.to_line().unwrap()).unwrap();
    let score = &wire["payload"]["score"];
    assert_eq!(score["continuity"]["method"], "continuous-runs-v1");
    let segments = score["segments"].as_array().unwrap();
    assert_eq!(segments.len(), 29);

    // Binary64 intervals of at least 5 ms are integral in units of 2^-60 s.
    // Sum the actual wire values exactly, as the native synthesis validator
    // does, so repeated rounded additions cannot conceal a 1 ps overrun.
    const UNITS_PER_SECOND: u128 = 1 << 60;
    let mut elapsed_units = 0_u128;
    let mut previous_frame = 0;
    for (segment, note) in segments.iter().zip(&p.tracks[0].notes) {
        let seconds = segment["duration_seconds"].as_f64().unwrap();
        assert!((0.005 - 1e-12..=30.0 + 1e-12).contains(&seconds), "{seconds:.16}");
        near(seconds, note.duration_tick as f64 / 2000.0);
        if let Some(gap) = segment.get("gap_after_seconds") { assert_eq!(gap.as_f64(), Some(0.0)); }
        let units = seconds * UNITS_PER_SECOND as f64;
        assert_eq!(units.fract(), 0.0);
        elapsed_units += units as u128;
        let frame = (elapsed_units * 200 + UNITS_PER_SECOND / 2) / UNITS_PER_SECOND;
        assert_eq!(frame - previous_frame, (note.duration_tick / 10) as u128);
        previous_frame = frame;
    }
    assert_eq!(previous_frame, 24_000);
    let wire_seconds = elapsed_units as f64 / UNITS_PER_SECOND as f64;
    assert!((120.0 - 1e-12..=120.0 + 1e-12).contains(&wire_seconds), "wire duration {wire_seconds:.16} s");
}

#[test]
fn a_tight_continuous_run_keeps_both_adjacent_minimum_length_cvs() {
    let p = project(vec![cv(960, 96, 60, "ma1"), cv(1056, 96, 64, "ma1")]);
    let mut c = continuous_caps();
    c.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().continuity.as_mut().unwrap().max_seconds = 0.1;
    let result = planned(&p, &c);
    assert_timing_geometry(&p, &result);
    let [phrase] = result.phrases.as_slice() else { panic!("the two 50 ms notes fit one 100 ms request") };
    near(phrase.start_seconds, 0.5);
    near(phrase.seconds, 0.1);
    assert_eq!(phrase.score.continuity.as_ref().unwrap().method, "continuous-runs-v1");
    assert_eq!(phrase.score.segments.len(), 4);
    for segment in &phrase.score.segments { near(segment.duration_seconds, 0.025); }
    for (index, trace) in phrase.score.phoneme_timing.as_ref().unwrap().notes.iter().enumerate() {
        assert_eq!(trace.nucleus_segment_index, Some(index * 2 + 1));
        near(trace.nucleus_delay_seconds, 0.025);
        near(trace.prefix_seconds, 0.0);
        near(trace.tail_trim_seconds, 0.0);
    }
}

#[test]
fn negotiated_continuity_keeps_the_original_nucleus_across_thirty_seconds() {
    let mut notes = vec![cv(960, 960, 60, "ma1")];
    for index in 1..=60 {
        let mut sustain = Note::new(960 + index * 960, 960, 60 + (index % 5) as i32);
        sustain.lyric = "+".into();
        notes.push(sustain);
    }
    let p = project(notes);
    let joined = planned(&p, &continuous_caps());
    assert_anchor_geometry(&p, &joined);
    let [phrase] = joined.phrases.as_slice() else { panic!("the 30.5 s chain fits one negotiated run") };
    near(phrase.seconds, 30.61);
    assert_eq!(phrase.score.continuity.as_ref().unwrap().method, "continuous-runs-v1");
    let segments = &phrase.score.segments;
    assert_eq!(segments.len(), 62);
    assert_eq!(segments[1].context.as_ref().unwrap().features["C_YM"], "a");
    for (segment, note) in segments[2..].iter().zip(&p.tracks[0].notes[1..]) {
        assert_eq!(segment.continuation_of, Some(1));
        assert_eq!(segment.context, segments[1].context);
        assert_eq!(segment.midi_note, Some(f64::from(note.midi_note)));
        near(segment.duration_seconds, 0.5);
    }
    let ordinary = plan(&p, &p.tracks[0].id, Route::VerdantVoice, &caps());
    assert!(ordinary.diagnostics.iter().any(|d| d.code == "E_CONTINUATION_PHRASE"));
    assert!(ordinary.phrases.iter().all(|phrase| phrase.seconds <= 30.0));
}

#[test]
fn a_hundred_seconds_of_legato_is_one_explicit_request_with_one_clock() {
    let p=project((0..200).map(|i|cv(960+i*960,960,60+(i%5) as i32,"ma1")).collect());
    let old=planned(&p,&caps());
    assert!(old.phrases.len()>3);
    let new=planned(&p,&continuous_caps());
    assert_eq!(new.phrases.len(),1);
    let phrase=&new.phrases[0];
    near(phrase.seconds,100.11);
    assert_eq!(phrase.note_ids.len(),200);
    assert_eq!(phrase.score.segments.len(),400);
    assert_eq!(phrase.score.continuity.as_ref().unwrap().method,"continuous-runs-v1");
    assert!(phrase.control_track.as_ref().unwrap().frame_times_seconds.len()>20_000);
    assert_anchor_geometry(&p,&new);
    let request=verdant_backend::Request::render("cont","cont",0,Route::VerdantVoice,payload_for(phrase,Route::VerdantVoice,10008));
    assert!(request.to_line().is_ok());
}

#[test]
fn only_real_rests_split_continuous_jobs_and_tiny_gaps_stay_inside_one_frame_plan() {
    // At120 BPM /960 PPQ: eight ticks are4.167ms, ten ticks5.208ms.
    let p=project(vec![cv(960,960,60,"a1"),cv(1928,960,64,"a1"),cv(2898,960,67,"a1")]);
    let result=planned(&p,&continuous_caps());
    assert_eq!(result.phrases.len(),2);
    assert_eq!(result.phrases[0].note_ids.len(),2);
    let separation=result.phrases[1].start_seconds-result.phrases[0].start_seconds-result.phrases[0].seconds;
    near(separation,10.0/1920.0);
    assert!(separation>=0.005);
    assert_anchor_geometry(&p,&result);
}

#[test]
fn resource_limits_block_the_run_instead_of_cutting_through_voice() {
    let p=project((0..241).map(|i|cv(960+i*960,960,60,"ma1")).collect());
    let c=continuous_caps();
    let blocked=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&c);
    assert!(blocked.phrases.is_empty());
    assert!(blocked.diagnostics.iter().any(|d|d.code=="E_CONTINUOUS_RUN_LIMIT"));
    let p=project((0..6).map(|i|cv(960+i*960,960,60,"ma1")).collect());
    let mut c=continuous_caps();
    c.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().continuity.as_mut().unwrap().max_segments=8;
    let blocked=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&c);
    assert!(blocked.phrases.is_empty());
    assert!(blocked.diagnostics.iter().any(|d|d.code=="E_CONTINUOUS_RUN_LIMIT"));
}

#[test]
fn timing_off_remains_legacy_and_malformed_continuity_capabilities_cannot_silently_cut() {
    let mut p=project((0..80).map(|i|cv(960+i*960,960,60,"ma1")).collect());
    p.render_preferences.phoneme_timing=None;
    assert_eq!(planned(&p,&caps()).phrases,planned(&p,&continuous_caps()).phrases);
    p.render_preferences.phoneme_timing=Some(PhonemeTimingSettings::vowel_anchor_v1());
    let mut c=continuous_caps();
    c.native_score.as_mut().unwrap().phoneme_timing.as_mut().unwrap().continuity.as_mut().unwrap().max_seconds=121.0;
    let result=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&c);
    assert!(result.phrases.is_empty());
    assert!(result.diagnostics.iter().any(|d|d.code=="E_CONTINUITY_CAPABILITY"));
}
