use serde_json::json;
use verdant_backend::adapter::{payload_for, render_digest};
use verdant_backend::{Capabilities, PhrasePlan, Route, plan};
use verdant_domain::{Note, PhonemeTimingSettings, Project, Pronunciation, PronunciationSource, TempoMap};
use verdant_domain::model::TempoPoint;

fn caps() -> Capabilities {
    serde_json::from_value(json!({
        "protocol":"ogd-worker/1", "test_double":true, "routes":["verdant-voice"], "max_phrase_seconds":30,
        "native_score":{"inventory":["a","ma","shi","gu"], "english_vowel_proxies":["a"],
            "continuous_f0_control":true,"sub_syllable_timing":true,"phoneme_duration_control":true,
            "explicit_f0_hz_range":[35,2000],
            "phoneme_frontend":{"schema_version":"verdant-phonemes/2","alphabets":["ipa","xsampa"],"languages":{"zh":"Chinese","ja":"Japanese","en":"English"}},
            "phoneme_timing":{"methods":["vowel-anchor-v1"],"routes":["verdant-voice"],"timing_resolution_seconds":0.005}},
        
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

fn assert_anchor_geometry(project: &Project, plan: &PhrasePlan) {
    let tempo = TempoMap::new(&project.tempo_map,project.ppq).unwrap();
    for phrase in &plan.phrases {
        let mut starts = vec![0.0];
        for segment in &phrase.score.segments {
            starts.push(starts.last().unwrap()+segment.duration_seconds+segment.gap_after_seconds);
        }
        near(*starts.last().unwrap(),phrase.seconds);
        for trace in &phrase.score.phoneme_timing.as_ref().unwrap().notes {
            let note = project.tracks[0].notes.iter().find(|note| note.id.as_str() == trace.note_id).unwrap();
            near(phrase.start_seconds+trace.note_onset_seconds,tempo.seconds_at(note.start_tick as f64));
            near(phrase.start_seconds+trace.note_end_seconds,tempo.seconds_at(note.end_tick() as f64));
            near(starts[trace.nucleus_segment_index],trace.note_onset_seconds);
            near(starts[trace.nucleus_segment_index]-starts[trace.first_segment_index],trace.prefix_seconds);
            let end = trace.end_segment_index;
            near(starts[end]-phrase.score.segments[end-1].gap_after_seconds,trace.note_end_seconds-trace.tail_trim_seconds);
        }
    }
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
fn insufficient_preroll_or_vowel_budget_is_reported_without_moving_notes() {
    let p = project(vec![cv(0,960,60,"ma1")]);
    let result = plan(&p,&p.tracks[0].id,Route::VerdantVoice,&caps());
    assert!(result.phrases.is_empty());
    assert!(result.diagnostics.iter().any(|diagnostic|diagnostic.code=="E_PHONEME_TIMING_PREROLL"));
    let mut a = cv(960,48,60,"a");
    a.pronunciation.frontend_profile = "english-vowel-proxy-native-0.2".into();
    let p = project(vec![a,cv(1008,960,64,"ma1")]);
    let result = plan(&p,&p.tracks[0].id,Route::VerdantVoice,&caps());
    assert!(result.phrases.is_empty());
    assert!(result.diagnostics.iter().any(|diagnostic|diagnostic.code=="E_PHONEME_TIMING_BUDGET"));
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
    p.tracks[0].notes[0].pronunciation.phonemes=vec!["s".into()];
    let blocked=plan(&p,&p.tracks[0].id,Route::VerdantVoice,&caps());
    assert!(blocked.phrases.is_empty());
    assert!(blocked.diagnostics.iter().any(|diagnostic|diagnostic.code=="E_PHONEME_TIMING_NUCLEUS"));
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
