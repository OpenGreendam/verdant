use super::*;
use verdant_backend::phonemizer::{PhonemizerSettings, resolve_track};
use verdant_session::{LyricOutcome, LyricSession, TextEditState};

fn track(lyrics: &[&str]) -> Track {
    let mut track = Track::new("歌词操作", "teal");
    track.notes = lyrics.iter().enumerate().map(|(index, lyric)| {
        let mut note = Note::new(index as i64 * 480, 480, 60 + index as i32 * 2);
        note.lyric = (*lyric).into();
        note
    }).collect();
    track.refresh_index();
    track
}

fn manual(pinyin: &str) -> Pronunciation {
    Pronunciation { pinyin: pinyin.into(), source: PronunciationSource::Manual, frontend_profile: PINYIN_PROFILE.into(), ..Default::default() }
}

fn lyrics(notes: &[Note]) -> Vec<&str> {
    notes.iter().map(|note| note.lyric.as_str()).collect()
}

#[test]
fn mixed_sentence_separators_fill_one_syllable_per_selected_note_in_order() {
    let source = track(&["外", "旧", "旧", "旧", "旧", "旧", "外"]);
    let selected = &source.notes[1..6];
    let tokens = batch_lyric_tokens(" 春， 风；又:绿！江？\n", false).unwrap();
    let changed = distribute_lyrics(selected, &source.notes, &tokens, &[]).unwrap();
    assert_eq!(lyrics(&changed), ["春", "风", "又", "绿", "江"]);
    for (before, after) in selected.iter().zip(&changed) {
        assert_eq!(before.id, after.id);
        assert_eq!(before.start_tick, after.start_tick);
        assert_eq!(before.midi_note, after.midi_note);
        assert_eq!(before.curves, after.curves);
    }
    assert_eq!(lyrics(&source.notes), ["外", "旧", "旧", "旧", "旧", "旧", "外"]);
    assert!(distribute_lyrics(&selected[..2], &source.notes, &tokens, &[]).is_err());
}

#[test]
fn character_fill_keeps_graphemes_and_explicit_readings_whole() {
    let text = "绿，e\u{301} 👩\u{200d}👧；女/nu:3 hello/[en:h ə . ˈl oʊ]";
    let tokens = batch_lyric_tokens(text, true).unwrap();
    assert_eq!(tokens, ["绿", "e\u{301}", "👩\u{200d}👧", "女/nu:3", "hello/[en:h ə . ˈl oʊ]"]);
    let source = track(&["", "", "", "", ""]);
    let changed = distribute_lyrics(&source.notes, &source.notes, &tokens, &[]).unwrap();
    assert_eq!(lyrics(&changed), ["绿", "e\u{301}", "👩\u{200d}👧", "女", "hello"]);
    assert_eq!(changed[3].pronunciation.pinyin, "nu:3");
    assert_eq!(changed[4].pronunciation.language.as_deref(), Some("en"));
    assert_eq!(changed[4].pronunciation.source, PronunciationSource::Manual);
    assert!(batch_lyric_tokens("hello/[en:h ə", false).is_err());
}

#[test]
fn batch_reentering_the_same_lyric_preserves_authored_pronunciation_and_timing() {
    let mut source = track(&["重"]);
    source.notes[0].pronunciation = manual("chong2");
    source.notes[0].pronunciation.phoneme_durations_permille = Some(vec![250, 750]);
    source.notes[0].pronunciation.unknown.insert("authored".into(), true.into());
    let tokens = batch_lyric_tokens("重。", false).unwrap();
    let changed = distribute_lyrics(&source.notes, &source.notes, &tokens, &[]).unwrap();
    assert_eq!(changed, source.notes);
}

#[test]
fn cyclic_shifts_wrap_readings_at_both_selection_edges() {
    let mut source = track(&["外", "妈", "拿", "拉", "外"]);
    for (note, reading) in source.notes[1..4].iter_mut().zip(["ma1", "na2", "la3"]) {
        note.pronunciation = manual(reading);
        note.pronunciation.unknown.insert("user_metadata".into(), note.lyric.clone().into());
    }
    let selected = &source.notes[1..4];
    let right = shifted_lyrics(selected, &source.notes, true).unwrap();
    let left = shifted_lyrics(selected, &source.notes, false).unwrap();
    assert_eq!(lyrics(&right), ["拉", "妈", "拿"]);
    assert_eq!(lyrics(&left), ["拿", "拉", "妈"]);
    for (index, changed) in right.iter().enumerate() {
        assert_eq!(changed.pronunciation, selected[(index + 2) % 3].pronunciation);
        assert_eq!(changed.id, selected[index].id);
        assert_eq!(changed.midi_note, selected[index].midi_note);
        assert_eq!(changed.start_tick, selected[index].start_tick);
    }
    assert_eq!(shifted_lyrics(&right, &right, false).unwrap(), selected);
    assert_eq!(source.notes[0].lyric, "外");
    assert_eq!(source.notes[4].lyric, "外");
}

#[test]
fn cyclic_sustain_wrap_has_an_anchor_and_does_not_repeat_an_onset() {
    let mut source = track(&["妈", "", ""]);
    source.notes[0].pronunciation = manual("ma1");
    source.notes = continued_syllable(&source.notes, &source.notes).unwrap();
    for right in [false, true] {
        let shifted = shifted_lyrics(&source.notes, &source.notes, right).unwrap();
        assert_eq!(lyrics(&shifted), ["妈", "+", "+"]);
        assert_eq!(shifted[0].pronunciation, source.notes[0].pronunciation);
        assert!(shifted[1..].iter().all(|n| n.is_continuation() && n.pronunciation == Pronunciation::default()));
        let mut rendered = source.clone();
        rendered.notes = shifted;
        let resolved = resolve_track(&rendered, &PhonemizerSettings::default());
        assert!(resolved.diagnostics.is_empty(), "{:?}", resolved.diagnostics);
        for note in &rendered.notes[1..] {
            assert_eq!(resolved.notes[&note.id].continuation_of.as_ref(), Some(&rendered.notes[0].id));
        }
    }
}

#[test]
fn cyclic_shift_materializes_a_syllable_split_by_the_selection_boundary() {
    let mut source = track(&["妈", "+", "拿"]);
    source.notes[0].pronunciation = manual("ma1");
    source.notes[1].set_continuation(true);
    source.notes[2].pronunciation = manual("na2");
    let shifted = shifted_lyrics(&source.notes, &source.notes, false).unwrap();
    assert_eq!(lyrics(&shifted), ["妈", "拿", "妈"]);
    assert!(shifted.iter().all(|n| !n.is_continuation()));
    assert_eq!(shifted[0].pronunciation, source.notes[0].pronunciation);
    assert_eq!(shifted[2].pronunciation, source.notes[0].pronunciation);
}

#[test]
fn continuation_over_changing_pitches_keeps_one_lyric_and_one_explicit_reading() {
    let mut source = track(&["妈", "", ""]);
    source.notes[0].pronunciation = manual("ma1");
    source.notes[0].pronunciation.phoneme_durations_permille = Some(vec![200, 800]);
    source.notes[0].pronunciation.unknown.insert("authored".into(), true.into());
    let original = source.clone();
    source.notes = continued_syllable(&source.notes, &source.notes).unwrap();
    assert_eq!(lyrics(&source.notes), ["妈", "+", "+"]);
    assert_eq!(source.notes[0], original.notes[0]);
    assert_eq!(source.notes.iter().map(|n| n.midi_note).collect::<Vec<_>>(), [60, 62, 64]);
    assert!(source.notes[1..].iter().all(|n| n.is_continuation() && n.pronunciation == Pronunciation::default()));
    let resolved = resolve_track(&source, &PhonemizerSettings::default());
    assert!(resolved.diagnostics.is_empty(), "{:?}", resolved.diagnostics);
    for note in &source.notes[1..] {
        let reading = &resolved.notes[&note.id];
        assert_eq!(reading.continuation_of.as_ref(), Some(&source.notes[0].id));
        assert!(reading.nucleus.is_some());
        assert_eq!(reading.pronunciation, source.notes[0].pronunciation);
    }
}

#[test]
fn continuation_refuses_to_destroy_a_different_explicit_reading_or_cross_a_boundary() {
    let mut source = track(&["妈", "拿", ""]);
    source.notes[0].pronunciation = manual("ma1");
    source.notes[1].pronunciation = manual("na2");
    source.notes[1].pronunciation.source = PronunciationSource::Imported;
    let original = source.clone();
    assert!(continued_syllable(&source.notes, &source.notes).is_err());
    assert_eq!(source, original);
    source.notes[1].pronunciation = Pronunciation::default();
    let skipped = vec![source.notes[0].clone(), source.notes[2].clone()];
    assert!(continued_syllable(&skipped, &source.notes).is_err());
    source.notes[1].start_tick += 1;
    assert!(continued_syllable(&source.notes, &source.notes).is_err());
    source.notes[1].start_tick -= 1;
    source.notes[1].set_muted(true);
    assert!(continued_syllable(&source.notes, &source.notes).is_err());
}

#[test]
fn continuation_preserves_an_existing_compatible_explicit_reading() {
    let mut source = track(&["妈", "妈"]);
    source.notes[0].pronunciation = manual("ma1");
    source.notes[1].pronunciation = manual("ma1");
    source.notes[1].pronunciation.source = PronunciationSource::Imported;
    source.notes[1].pronunciation.unknown.insert("import_id".into(), "reading-42".into());
    let changed = continued_syllable(&source.notes, &source.notes).unwrap();
    assert_eq!(changed[0], source.notes[0]);
    assert_eq!(changed[1].pronunciation, source.notes[1].pronunciation);
    assert!(changed[1].is_continuation());
}

#[test]
fn batch_continuations_use_the_new_anchor_without_copying_its_pronunciation() {
    let source = track(&["旧", "", ""]);
    let tokens = batch_lyric_tokens("妈/ma1 + +", false).unwrap();
    let changed = distribute_lyrics(&source.notes, &source.notes, &tokens, &[]).unwrap();
    assert_eq!(lyrics(&changed), ["妈", "+", "+"]);
    assert_eq!(changed[0].pronunciation.pinyin, "ma1");
    assert!(changed[1..].iter().all(|n| n.pronunciation == Pronunciation::default()));
    let mut gapped = source.clone();
    gapped.notes[1].start_tick += 1;
    assert!(distribute_lyrics(&gapped.notes, &gapped.notes, &tokens, &[]).is_err());
}

#[test]
fn batch_prompt_ime_uses_relative_utf16_selection_and_keeps_graphemes_intact() {
    let mut state = TextEditState::new("绿𝄞");
    state.end();
    state.replace_and_mark_text_in_range(None, "e\u{301}👩\u{200d}👧", Some(2..7));
    assert_eq!(state.marked, Some("绿𝄞".len()..state.text.len()));
    assert_eq!(state.range_to_utf16(&state.selection), 5..10);
    assert_eq!(state.text_for_range_utf16(2..3), ("𝄞".into(), 1..3));
    let composing = state.clone();
    crate::lyric_view::edit_unmarked_text(&mut state, |s| s.backspace());
    crate::lyric_view::edit_unmarked_text(&mut state, |s| s.replace_text_in_range(None, "clipboard"));
    assert_eq!(state, composing);
    state.replace_and_mark_text_in_range(None, "绿坝", Some(2..2));
    assert_eq!(state.range_to_utf16(&state.selection), 5..5);
    state.replace_text_in_range(None, "绿坝");
    assert!(!state.is_composing());
    assert_eq!(state.text, "绿𝄞绿坝");
    crate::lyric_view::edit_unmarked_text(&mut state, |s| s.replace_text_in_range(None, "e\u{301}👩\u{200d}👧"));
    crate::lyric_view::edit_unmarked_text(&mut state, |s| s.backspace());
    assert_eq!(state.text, "绿𝄞绿坝e\u{301}");
    crate::lyric_view::edit_unmarked_text(&mut state, |s| s.backspace());
    assert_eq!(state.text, "绿𝄞绿坝");

    let mut inline = LyricSession::new("track".into(), "note".into(), "");
    inline.state.replace_and_mark_text_in_range(None, "绿𝄞", Some(3..3));
    assert_eq!(inline.on_ime_commit(None, "绿𝄞"), LyricOutcome::Commit("绿𝄞".into()));
    assert_eq!(inline.confirm(), LyricOutcome::None);
    inline.state.replace_and_mark_text_in_range(None, "坝", Some(1..1));
    inline.state.unmark_text();
    assert_eq!(inline.confirm(), LyricOutcome::Commit("绿𝄞坝".into()));
    assert_eq!(inline.confirm(), LyricOutcome::None);
}
