use std::borrow::Cow;
use std::collections::BTreeMap;

use verdant_domain::{Note, Pronunciation, PronunciationSource, Track};

use super::{AUTOMATIC, ENGLISH_G2P, EXPLICIT_ONLY, JAPANESE_KANA, MANDARIN, PhonemizerSettings, PhraseEntry,
            ReadingDiagnostic, ReadingOrigin, ResolvedNote, ResolvedTrack};
use crate::reading::{ReadingError, pronunciation_shape};

#[path = "polyphone.rs"]
pub(super) mod polyphone;

pub(super) fn resolve(track: &Track, settings: &PhonemizerSettings) -> ResolvedTrack {
    let mut out = ResolvedTrack::default();
    if track.muted { return out; }
    let selected = match settings.validate_structure().and_then(|()| settings.selected_for_track(track)) {
        Ok(selected) => selected,
        Err(error) => {
            out.diagnostics.push(ReadingDiagnostic {
                code: "E_PHONEMIZER_CONFIG", note_id: None,
                message: "当前发音器、轨道语言或用户字典设置无效，无法解析歌词".into(), detail: error, candidates: Vec::new(),
            });
            return out;
        }
    };
    let phrases = phrase_readings(track, settings);
    let mut intervals: Vec<_> = settings.segments.iter().filter(|(_, entry)| entry.track == track.id).collect();
    intervals.sort_by_key(|(_, entry)| entry.start_tick);
    let covering: Vec<_> = track.notes.iter().map(|note| {
        let index = intervals.partition_point(|(_, entry)| entry.start_tick <= note.start_tick);
        index.checked_sub(1).and_then(|i| intervals.get(i).copied()).filter(|(_, entry)| note.start_tick < entry.end_tick)
    }).collect();
    let selections: Vec<_> = covering.iter().map(|entry| entry.map_or(selected, |(_, entry)| entry.selected.as_str())).collect();
    let segments: Vec<_> = covering.iter().map(|entry| entry.map(|(key, _)| key.as_str())).collect();
    let evidence: Vec<_> = track.notes.iter().map(|note| {
        super::automatic::script(&note.lyric).or_else(||
            (crate::pinyin::parse_token(&note.lyric).is_ok() && !super::english::is_common_english_word(&note.lyric)).then_some("zh"))
    }).collect();
    let mut source_anchor = None;
    for (index, note) in track.notes.iter().enumerate() {
        if note.is_muted() { source_anchor = None; continue; }
        let resolved = if note.is_continuation() {
            continuation(note, index.checked_sub(1).map(|previous| &track.notes[previous]), source_anchor, &out)
        } else {
            let local = index.checked_sub(1).filter(|&i| !track.notes[i].is_muted()
                    && segments[i] == segments[index] && track.notes[i].end_tick() == note.start_tick)
                .and_then(|i| evidence[i]).or_else(|| track.notes.get(index + 1)
                    .filter(|next| !next.is_muted() && segments[index] == segments[index + 1]
                        && note.end_tick() == next.start_tick).and_then(|_| evidence[index + 1]));
            ordinary(note, phrases.get(index).copied().flatten(), settings, selections[index], local,
                mandarin_choice(track, index, settings, &selections, &segments, &phrases))
        };
        match resolved {
            Ok(reading) => {
                if !note.is_continuation() { source_anchor = Some(note); }
                out.notes.insert(note.id.clone(), reading);
            }
            Err(error) => { source_anchor = None; out.diagnostics.push(error); }
        }
    }
    // A standalone sokuon has no vowel. Consult the following fully resolved
    // reading, so explicit notes, dictionaries and selected regions retain
    // their precedence over any inference from its displayed lyric.
    for (index, note) in track.notes.iter().enumerate() {
        if note.lyric != "っ" || note.is_muted() || out.notes.contains_key(&note.id)
            || !matches!(selections[index], AUTOMATIC | JAPANESE_KANA)
            || matches!(note.pronunciation.source, PronunciationSource::Manual | PronunciationSource::Imported)
            || phrases.get(index).is_some_and(|entry| entry.is_some())
            || settings.dictionary.lyrics.contains_key(&note.lyric)
            || note.pronunciation.language.as_deref().is_some_and(|language| language != "ja") {
            continue;
        }
        let Some(next) = track.notes.get(index + 1) else { continue; };
        if next.is_muted() || next.is_continuation() || note.duration_tick <= 0 || next.duration_tick <= 0
            || note.end_tick() != next.start_tick || segments[index] != segments[index + 1] { continue; }
        let Some(next_reading) = out.notes.get(&next.id) else { continue; };
        let Ok(next_phones) = crate::reading::resolve_pronunciation(&next_reading.pronunciation) else { continue; };
        if next_phones.language != "ja" { continue; }
        let Some(onset) = next_phones.canonical.first() else { continue; };
        let Some(geminate) = crate::kana::geminate_onset(&onset.symbol) else { continue; };
        let reading = Pronunciation {
            input_text: Some(note.lyric.clone()), language: Some("ja".into()), alphabet: Some("ipa".into()),
            phonemes: vec![geminate], allow_approximation: true, source: PronunciationSource::ExplicitMap,
            frontend_profile: "kana-geminate-onset-v1".into(),
            reading_provenance: Some(serde_json::json!({"schema_version":"verdant-kana-geminate/1",
                "following_note":next.id, "onset":onset.symbol,
                "basis":"The next touching Japanese reading supplies a geminate consonant; no vowel or glottal stop is inferred."})),
            ..Default::default()
        };
        if let Ok(resolved) = checked(note, Cow::Owned(reading), ReadingOrigin::Builtin) {
            out.notes.insert(note.id.clone(), resolved);
            out.diagnostics.retain(|diagnostic| diagnostic.note_id.as_ref() != Some(&note.id));
        }
    }
    out
}

/// Match left to right, choosing the longest exact phrase at each boundary.
/// All matching keys are prefixes of the same authored text, so descending
/// UTF-8 byte length also orders their actual text length correctly.
fn phrase_readings<'a>(track: &Track, settings: &'a PhonemizerSettings) -> Vec<Option<(&'a str, &'a Pronunciation)>> {
    let mut starts: BTreeMap<&str, Vec<(&str, &PhraseEntry)>> = BTreeMap::new();
    for (key, entry) in &settings.dictionary.phrases {
        starts.entry(entry.lyrics[0].as_str()).or_default().push((key, entry));
    }
    for entries in starts.values_mut() { entries.sort_by(|a, b| b.0.len().cmp(&a.0.len())); }
    let mut readings = Vec::new();
    let mut index = 0;
    while index < track.notes.len() {
        let note = &track.notes[index];
        let matched = starts.get(note.lyric.as_str()).and_then(|entries| entries.iter().find(|(_, entry)| {
            let Some(end) = index.checked_add(entry.lyrics.len()) else { return false; };
            let Some(notes) = track.notes.get(index..end) else { return false; };
            notes.iter().zip(&entry.lyrics).all(|(note, lyric)| {
                !note.is_muted() && !note.is_continuation() && note.duration_tick > 0 && note.lyric == *lyric
            }) && notes.windows(2).all(|pair| pair[0].end_tick() == pair[1].start_tick)
        }));
        if let Some((key, entry)) = matched {
            if readings.is_empty() { readings.resize(track.notes.len(), None); }
            for (offset, reading) in entry.readings.iter().enumerate() { readings[index + offset] = Some((*key, reading)); }
            index += entry.lyrics.len();
        } else { index += 1; }
    }
    readings
}

fn ordinary(note: &Note, phrase: Option<(&str, &Pronunciation)>, settings: &PhonemizerSettings, selected: &str, local: Option<&str>,
    automatic: Option<(&'static str, &'static str, &'static str)>) -> Result<ResolvedNote, ReadingDiagnostic> {
    if matches!(note.pronunciation.source, PronunciationSource::Manual | PronunciationSource::Imported) {
        return checked(note, Cow::Borrowed(&note.pronunciation), ReadingOrigin::ExplicitNote);
    }
    if let Some((key, pronunciation)) = phrase {
        return checked(note, Cow::Borrowed(pronunciation), ReadingOrigin::UserPhrase { phrase: key.into() });
    }
    if let Some(entry) = settings.dictionary.lyrics.get(&note.lyric) {
        return checked(note, Cow::Borrowed(&entry.reading), ReadingOrigin::UserLyric { lyric: note.lyric.clone() });
    }
    // A segment overrides the inherited frontend for its tick range only; it never overrides explicit readings.
    if existing_builtin(note, selected) {
        return checked(note, Cow::Borrowed(&note.pronunciation), ReadingOrigin::ExistingBuiltin);
    }
    match selected {
        MANDARIN | AUTOMATIC if super::automatic::script(&note.lyric) == Some("zh") && note.lyric.chars().count() == 1
            && note.pronunciation.language.as_deref().is_none_or(|l| l == "zh") => {
            let mut candidates = verdant_domain::hanzi_readings(&note.lyric);
            let primary = if automatic.is_none() && candidates.len() > 1 {
                verdant_domain::hanzi_primary_reading(&note.lyric)
                    .filter(|reading| candidates.contains(reading))
                    .or_else(|| candidates.first().cloned())
            } else { None };
            let automatic = match automatic {
                Some(choice) => Some(choice),
                None => primary.as_deref().map(|reading| (reading, "dictionary_primary", note.lyric.as_str())),
            };
            if let Some((pinyin, basis, key)) = automatic {
                let reading = Pronunciation {
                    input_text: Some(note.lyric.clone()), pinyin: pinyin.into(),
                    source: PronunciationSource::ExplicitMap,
                    frontend_profile: verdant_domain::HANZI_PINYIN_PROFILE.into(),
                    reading_provenance: Some(serde_json::json!({
                        "schema_version": "verdant-mandarin-context/1", "basis": basis, "key": key,
                        "candidates": candidates, "preferred_characters": polyphone::PREFERRED.len(),
                        "lexical_phrases": polyphone::PHRASES.len(),
                    })),
                    ..Default::default()
                };
                return checked(note, Cow::Owned(reading), ReadingOrigin::Builtin);
            }
            if candidates.len() != 1 { return Err(missing(note, candidates, "普通话字典没有可用读音；多字歌词与未知输入不会被猜测")); }
            let reading = Pronunciation {
                input_text: Some(note.lyric.clone()),
                pinyin: candidates.pop().expect("one dictionary reading"),
                source: PronunciationSource::ExplicitMap,
                frontend_profile: verdant_domain::HANZI_PINYIN_PROFILE.into(),
                ..Default::default()
            };
            checked(note, Cow::Owned(reading), ReadingOrigin::Builtin)
        }
        MANDARIN | AUTOMATIC | ENGLISH_G2P | JAPANESE_KANA => {
            let stale = note.pronunciation.source == PronunciationSource::ExplicitMap
                && note.pronunciation.input_text.as_deref().is_some_and(|input| input != note.lyric);
            let hint = (!stale).then_some(note.pronunciation.language.as_deref()).flatten();
            let reading = super::automatic::reading(&note.lyric, selected, hint, local)
                .map_err(|error| diagnostic(note, "E_PRONUNCIATION_UNSET",
                    format!("「{}」无法通过当前发音器转换（日语假名／拼音／英语）：{error}", note.lyric),
                    format!("lyric={:?}; selected={selected:?}; {error}", note.lyric)))?;
            checked(note, Cow::Owned(reading), ReadingOrigin::Builtin)
        }
        EXPLICIT_ONLY => Err(missing(note, verdant_domain::hanzi_readings(&note.lyric), "当前发音器只使用已填写读音和用户字典；请明确选择一个读音")),
        _ => unreachable!("selected frontend validated before resolution"),
    }
}

/// Automatic lexical context ends at every authored segment boundary, even
/// when both segments happen to select Mandarin. Explicit foreign readings
/// also terminate the window. No source note is changed.
fn mandarin_choice(track: &Track, index: usize, settings: &PhonemizerSettings, selections: &[&str], segments: &[Option<&str>],
    phrases: &[Option<(&str, &Pronunciation)>])
    -> Option<(&'static str, &'static str, &'static str)> {
    let note = &track.notes[index];
    let eligible = |other: &Note| !other.is_muted() && !other.is_continuation()
        && other.duration_tick > 0 && other.lyric.chars().count() == 1
        && other.pronunciation.language.as_deref().is_none_or(|language| language == "zh")
        ;
    let eligible_index = |i: usize| {
        let other = &track.notes[i];
        let explicit = matches!(other.pronunciation.source, PronunciationSource::Manual | PronunciationSource::Imported);
        let reading = if explicit { Some(&other.pronunciation) } else {
            phrases.get(i).copied().flatten().map(|(_, reading)| reading)
                .or_else(|| settings.dictionary.lyrics.get(&other.lyric).map(|entry| &entry.reading))
        };
        eligible(other) && matches!(selections[i], MANDARIN | AUTOMATIC) && segments[i] == segments[index]
            && reading.is_none_or(|reading| reading.language.as_deref().is_none_or(|language| language == "zh"))
    };
    if !eligible_index(index) { return None; }
    let candidates = verdant_domain::hanzi_readings(&note.lyric);
    if candidates.len() < 2 { return None; }
    // Tables currently contain two-note words. Bound scanning by the longest
    // lexical entry so work per note stays independent of track length.
    let maximum = polyphone::PHRASES.iter().map(|(_, readings)| readings.len()).max().unwrap_or(1);
    let mut start = index;
    while start > 0 && index - start + 1 < maximum && eligible_index(start - 1)
        && track.notes[start - 1].end_tick() == track.notes[start].start_tick { start -= 1; }
    let mut end = index + 1;
    while end < track.notes.len() && end - index < maximum && eligible_index(end)
        && track.notes[end - 1].end_tick() == track.notes[end].start_tick { end += 1; }
    polyphone::choose(&track.notes, index, start, end, &candidates)
}

fn existing_builtin(note: &Note, selected: &str) -> bool {
    let reading = &note.pronunciation;
    if reading.source != PronunciationSource::ExplicitMap { return false; }
    // Its onset depends on the following note's current reading, so it must be
    // rebuilt after that reading has resolved on every pass.
    if reading.frontend_profile == "kana-geminate-onset-v1" { return false; }
    // Automatic readings cache the authored input, including finite kana.
    // A lyric edit must recompile that input instead of retaining the former
    // phonemes. Explicit Manual/Imported readings were handled above and may
    // deliberately differ from the displayed lyric.
    if matches!(reading.frontend_profile.as_str(), verdant_domain::HANZI_PINYIN_PROFILE | "kana-ipa-native-0.6" | "direct-pinyin-v1" | "english-g2p-v1" | "automatic-parts-v1" | "imported-japanese-alias-v1")
        && reading.input_text.as_deref().is_some_and(|input| input != note.lyric) { return false; }
    match selected {
        MANDARIN => reading.language.as_deref().is_none_or(|language| language == "zh"),
        JAPANESE_KANA => reading.alphabet.is_some() && reading.language.as_deref() == Some("ja"),
        EXPLICIT_ONLY => true,
        ENGLISH_G2P => reading.language.as_deref() == Some("en"),
        AUTOMATIC => true,
        _ => false,
    }
}

fn checked(note: &Note, mut pronunciation: Cow<'_, Pronunciation>, origin: ReadingOrigin) -> Result<ResolvedNote, ReadingDiagnostic> {
    // A phone-boundary edit is authored on the original note even when its
    // reading is supplied by an automatic frontend or user dictionary. Preserve
    // that override, then validate it against the newly expanded phone count;
    // a stale/infeasible override must be a located error, never silently lost.
    if let Some(weights) = &note.pronunciation.phoneme_durations_permille {
        if pronunciation.phoneme_durations_permille.as_ref() != Some(weights) {
            pronunciation.to_mut().phoneme_durations_permille = Some(weights.clone());
        }
    }
    let shape = pronunciation_shape(&pronunciation).map_err(|error| reading_error(note, error))?;
    Ok(ResolvedNote { pronunciation: pronunciation.into_owned(), origin, continuation_of: None, nucleus: shape.nucleus })
}

fn continuation(note: &Note, previous: Option<&Note>, source_anchor: Option<&Note>, out: &ResolvedTrack) -> Result<ResolvedNote, ReadingDiagnostic> {
    let previous = previous.ok_or_else(|| diagnostic(note, "E_READING_CONTINUATION_ANCHOR",
        "延音前缺少同轨道的发声音符", "continuation has no preceding authored note in this track"))?;
    if previous.is_muted() {
        return Err(diagnostic(note, "E_READING_CONTINUATION_MUTED", "延音不能跨过静音音符", "the immediately preceding authored note is muted"));
    }
    if note.duration_tick <= 0 || previous.duration_tick <= 0 || previous.end_tick() != note.start_tick {
        return Err(diagnostic(note, "E_READING_CONTINUATION_BOUNDARY", "延音须与前一音符准确相接，不能跨过休止或重叠",
            format!("previous={} end_tick={}; continuation={} start_tick={}", previous.id, previous.end_tick(), note.id, note.start_tick)));
    }
    let anchor = out.notes.get(&previous.id).ok_or_else(|| diagnostic(note, "E_READING_CONTINUATION_UNRESOLVED",
        "请先解决延音前一音符的读音", format!("preceding note {} has no resolved pronunciation", previous.id)))?;
    if anchor.nucleus.is_none() {
        return Err(diagnostic(note, "E_READING_CONTINUATION_NUCLEUS", "前一读音没有已声明的元音／韵母核，无法延音",
            "the resolved native mapping contains no nucleus; consonants are not guessed to be vowels"));
    }
    // Ordinary editor continuation edits copy the source seed before a user
    // dictionary is applied. Accept that copy as well as the resolved reading;
    // otherwise editing a dictionary would spuriously block an existing sustain.
    if matches!(note.pronunciation.source, PronunciationSource::Manual | PronunciationSource::Imported)
        && !same_reading(&note.pronunciation, &anchor.pronunciation)
        && !source_anchor.is_some_and(|source| same_reading(&note.pronunciation, &source.pronunciation)) {
        return Err(diagnostic(note, "E_READING_CONTINUATION_OVERRIDE", "延音读音与原音符不同；请取消延音或使用原音符的读音",
            "an explicit continuation reading may not replace the sustained anchor with a new onset"));
    }
    Ok(ResolvedNote {
        pronunciation: anchor.pronunciation.clone(), origin: ReadingOrigin::Continuation,
        continuation_of: Some(anchor.continuation_of.as_ref().unwrap_or(&previous.id).clone()), nucleus: anchor.nucleus,
    })
}

fn same_reading(a: &Pronunciation, b: &Pronunciation) -> bool {
    a.pinyin == b.pinyin && a.phonemes == b.phonemes && a.language == b.language && a.alphabet == b.alphabet
        && a.allow_approximation == b.allow_approximation && a.stress == b.stress
        && a.mora_weights == b.mora_weights && a.phoneme_durations_permille == b.phoneme_durations_permille
}

fn reading_error(note: &Note, error: ReadingError) -> ReadingDiagnostic {
    diagnostic(note, error.code, format!("「{}」的读音无法使用：{}", note.lyric, error.message), error.message)
}

fn missing(note: &Note, candidates: Vec<String>, reason: &str) -> ReadingDiagnostic {
    let ambiguous = candidates.len() > 1;
    ReadingDiagnostic {
        code: if ambiguous { "E_PRONUNCIATION_AMBIGUOUS" } else { "E_PRONUNCIATION_UNSET" },
        note_id: Some(note.id.clone()),
        message: if ambiguous { format!("「{}」有多个读音，请明确选择：{}", note.lyric, candidates.join(" / ")) }
            else { format!("「{}」缺少已确认的读音", note.lyric) },
        detail: reason.into(), candidates,
    }
}

fn diagnostic(note: &Note, code: &'static str, message: impl Into<String>, detail: impl Into<String>) -> ReadingDiagnostic {
    ReadingDiagnostic { code, note_id: Some(note.id.clone()), message: message.into(), detail: detail.into(), candidates: Vec::new() }
}
