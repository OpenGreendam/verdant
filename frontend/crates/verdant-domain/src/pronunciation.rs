//! Dictionary spelling for one Hanzi per note. This does not establish acoustic
//! support and never modifies the note's pitch or injects a lexical tone curve.
use pinyin::{ToPinyin, ToPinyinMulti};

use crate::{Pronunciation, PronunciationSource};

pub const HANZI_PINYIN_PROFILE: &str = "hanzi-pinyin-data0.11";

/// Explicit phonetic editor syntax, without text-to-phoneme guessing.
/// Examples: `hello/[en:h ə l oʊ]`, `さ/[ja:s a]`,
/// `hello/[en/xsampa:h @ l oU]`. `ja` selects its named cross-language
/// approximation profile; a `~` suffix explicitly permits approximations for
/// other languages. Symbol case and every diacritic are retained verbatim.
pub fn parse_phonetic_lyric(input: &str) -> Result<Option<(String, Pronunciation)>, String> {
    let text = input.trim();
    let Some((lyric, annotated)) = text.rsplit_once("/[") else { return Ok(None) };
    let body = annotated.strip_suffix(']').ok_or_else(|| "音标输入缺少结尾 ]".to_string())?;
    let (header, phones) = body.split_once(':').ok_or_else(|| "格式为 歌词/[语言:IPA] 或 歌词/[语言/xsampa:音素]".to_string())?;
    let (language, alphabet) = header.trim().split_once('/').unwrap_or((header.trim(), "ipa"));
    let explicitly_approximate = language.ends_with('~');
    let explicitly_strict = language.ends_with('!');
    let language = language.strip_suffix('~').or_else(|| language.strip_suffix('!')).unwrap_or(language);
    if !matches!(language, "zh" | "en" | "ja") { return Err("音素映射语言请选择 zh、en 或 ja".into()); }
    if !matches!(alphabet, "ipa" | "xsampa" | "vocaloid-xsampa") { return Err("音标体系请选择 ipa 或 xsampa；VOCALOID 兼容导入须显式填写 vocaloid-xsampa".into()); }
    let phonemes: Vec<String> = phones.split_whitespace().map(str::to_string).collect();
    if phonemes.is_empty() { return Err("音标括号内至少需要一个音素".into()); }
    let lyric = lyric.trim().to_string();
    Ok(Some((lyric.clone(), Pronunciation {
        input_text: Some(lyric), language: Some(language.into()), alphabet: Some(alphabet.into()),
        phonemes, allow_approximation: explicitly_approximate || (language == "ja" && !explicitly_strict),
        source: PronunciationSource::Manual, frontend_profile: "ipa-xsampa-native-0.6".into(),
        ..Default::default()
    })))
}

pub fn pronunciation_display(pronunciation: &Pronunciation) -> String {
    if let Some(alphabet) = pronunciation.alphabet.as_deref() {
        let language = pronunciation.language.as_deref().unwrap_or("?");
        let permission = if pronunciation.allow_approximation && language != "ja" { "~" }
            else if !pronunciation.allow_approximation && language == "ja" { "!" } else { "" };
        let alphabet = if alphabet == "ipa" { String::new() } else { format!("/{alphabet}") };
        format!("[{language}{permission}{alphabet}:{}]", pronunciation.phonemes.join(" "))
    } else { pronunciation.pinyin.clone() }
}

/// All distinct dictionary readings, with a trailing tone number (neutral = 5).
/// Unknown characters and inputs other than exactly one Unicode scalar return
/// no readings. No phrase segmentation or default polyphone choice is made.
pub fn hanzi_readings(text: &str) -> Vec<String> {
    let mut chars = text.chars();
    let Some(ch) = chars.next() else { return Vec::new() };
    if chars.next().is_some() {
        return Vec::new();
    }
    let Some(readings) = ch.to_pinyin_multi() else { return Vec::new() };
    let mut result: Vec<String> = readings.into_iter().map(|reading| {
        let mut text = reading.with_tone_num_end().to_lowercase().replace('ü', "v");
        if !text.ends_with(|c: char| c.is_ascii_digit()) {
            text.push('5');
        }
        text
    }).collect();
    result.sort();
    result.dedup();
    result
}

/// The pinned MIT dictionary's primary reading of exactly one Hanzi. This is
/// a deterministic default, not lexical disambiguation or acoustic admission.
pub fn hanzi_primary_reading(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    if chars.next().is_some() { return None; }
    let mut reading = ch.to_pinyin()?.with_tone_num_end().to_lowercase().replace('ü', "v");
    if !reading.ends_with(|c: char| c.is_ascii_digit()) { reading.push('5'); }
    Some(reading)
}

/// An explicit override always wins. An unchanged lyric preserves its reading;
/// replacing the lyric invalidates any old reading, including manual/imported
/// readings, and looks up the replacement without choosing a polyphone.
pub fn pronunciation_for_lyric(
    current: &Pronunciation,
    previous_lyric: &str,
    lyric: &str,
    manual: Option<(&str, &str)>,
) -> Option<Pronunciation> {
    let (pinyin, source, profile) = if let Some((pinyin, profile)) = manual {
        (pinyin.to_string(), PronunciationSource::Manual, profile)
    } else {
        if current.source != PronunciationSource::Unset && previous_lyric == lyric {
            return None;
        }
        let readings = hanzi_readings(lyric);
        if readings.len() != 1 {
            return (current != &Pronunciation::default()).then(Pronunciation::default);
        }
        (readings.into_iter().next().unwrap(), PronunciationSource::ExplicitMap, HANZI_PINYIN_PROFILE)
    };
    let changed = current.pinyin != pinyin || current.alphabet.is_some();
    Some(Pronunciation {
        input_text: Some(lyric.to_string()),
        pinyin,
        source,
        frontend_profile: profile.into(),
        phonemes: if changed { Vec::new() } else { current.phonemes.clone() },
        phoneme_durations_permille: if changed { None } else { current.phoneme_durations_permille.clone() },
        language: None,
        alphabet: None,
        allow_approximation: false,
        stress: None,
        mora_weights: None,
        reading_provenance: None,
        diagnostics: Vec::new(),
        ..current.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, CommandBatch, Note, Project, ProjectStore};

    #[test]
    fn explicit_phonetic_editor_preserves_case_language_and_symbols() {
        let (lyric, reading) = parse_phonetic_lyric(" し/[ja/xsampa:s\\ i] ").unwrap().unwrap();
        assert_eq!(lyric, "し");
        assert_eq!(reading.phonemes, ["s\\", "i"]);
        assert!(reading.allow_approximation);
        assert_eq!(pronunciation_display(&reading), "[ja/xsampa:s\\ i]");
        let mut strict = reading.clone(); strict.allow_approximation = false;
        let text = format!("し/{}", pronunciation_display(&strict));
        assert!(!parse_phonetic_lyric(&text).unwrap().unwrap().1.allow_approximation);
        let (_, en) = parse_phonetic_lyric("word/[en/xsampa:w 3` d]").unwrap().unwrap();
        assert!(!en.allow_approximation);
        assert_eq!(en.language.as_deref(), Some("en"));
        assert!(parse_phonetic_lyric("x/[en:] ").is_err());
        assert!(parse_phonetic_lyric("x/[en:ɹ").is_err());
        let replaced = pronunciation_for_lyric(&reading, "し", "海", None).unwrap();
        assert_eq!(replaced.alphabet, None);
        assert!(!replaced.allow_approximation);
        assert!(replaced.phonemes.is_empty());
        let bytes = serde_json::to_vec(&reading).unwrap();
        assert_eq!(serde_json::from_slice::<Pronunciation>(&bytes).unwrap(), reading);
    }

    #[test]
    fn single_readings_and_normalization() {
        assert_eq!(hanzi_readings("海"), ["hai3"]);
        assert_eq!(hanzi_readings("妈"), ["ma1"]);
        assert!(hanzi_readings("女").contains(&"nv3".to_string()));
        assert!(hanzi_readings("的").contains(&"de5".to_string()));
    }

    #[test]
    fn polyphones_are_not_silently_selected() {
        let readings = hanzi_readings("好");
        assert!(readings.contains(&"hao3".to_string()));
        assert!(readings.contains(&"hao4".to_string()));
        assert_eq!(pronunciation_for_lyric(&Pronunciation::default(), "", "好", None), None);
    }

    #[test]
    fn unknown_or_multiple_characters_are_not_truncated() {
        for text in ["", "海好", "海 ", "a", "🙂", "\u{10ffff}", "海\u{fe00}"] {
            assert!(hanzi_readings(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn manual_override_and_preservation() {
        let auto = pronunciation_for_lyric(&Pronunciation::default(), "", "海", None).unwrap();
        assert_eq!(auto.source, PronunciationSource::ExplicitMap);
        let manual = pronunciation_for_lyric(&auto, "海", "好", Some(("hao4", HANZI_PINYIN_PROFILE))).unwrap();
        assert_eq!(manual.pinyin, "hao4");
        assert_eq!(manual.source, PronunciationSource::Manual);
        assert_eq!(pronunciation_for_lyric(&manual, "好", "好", None), None);
        let replaced = pronunciation_for_lyric(&manual, "好", "海", None).unwrap();
        assert_eq!(replaced.pinyin, "hai3");
        assert_eq!(replaced.source, PronunciationSource::ExplicitMap);
        assert_eq!(replaced.frontend_profile, HANZI_PINYIN_PROFILE);
        let imported = Pronunciation { source: PronunciationSource::Imported, ..manual.clone() };
        assert_eq!(pronunciation_for_lyric(&imported, "好", "好", None), None);
        assert_eq!(pronunciation_for_lyric(&imported, "好", "海", None).unwrap().pinyin, "hai3");
        assert_eq!(pronunciation_for_lyric(&auto, "海", "海", None), None);
        assert_eq!(pronunciation_for_lyric(&auto, "海", "好", None), Some(Pronunciation::default()));
        assert_eq!(pronunciation_for_lyric(&manual, "好", "行", None), Some(Pronunciation::default()));
        assert_eq!(pronunciation_for_lyric(&manual, "好", "🙂", None), Some(Pronunciation::default()));
    }

    #[test]
    fn replacing_legacy_vowel_proxy_uses_new_hanzi_reading() {
        let legacy = Pronunciation {
            pinyin: "i".into(),
            source: PronunciationSource::Manual,
            frontend_profile: "english-vowel-proxy-native-0.2".into(),
            ..Default::default()
        };
        assert_eq!(pronunciation_for_lyric(&legacy, "咿", "咿", None), None);
        let replacement = pronunciation_for_lyric(&legacy, "咿", "海", None).unwrap();
        assert_eq!(replacement.pinyin, "hai3");
        assert_eq!(replacement.source, PronunciationSource::ExplicitMap);
        assert_eq!(replacement.frontend_profile, HANZI_PINYIN_PROFILE);
        let explicit = pronunciation_for_lyric(&legacy, "咿", "海", Some(("hai4", HANZI_PINYIN_PROFILE))).unwrap();
        assert_eq!(explicit.pinyin, "hai4");
        assert_eq!(explicit.source, PronunciationSource::Manual);
    }

    #[test]
    fn lyric_and_dictionary_reading_are_one_undo_without_pitch_changes() {
        let mut project = Project::new_empty();
        let note = Note::new(0, 480, 64);
        let id = note.id.clone();
        let track = project.tracks[0].id.clone();
        project.tracks[0].notes.push(note.clone());
        let mut store = ProjectStore::new(project);
        let pronunciation = pronunciation_for_lyric(&note.pronunciation, "", "海", None).unwrap();
        store.apply(store.revision(), CommandBatch { label: "编辑歌词".into(), commands: vec![
            Command::SetLyric { track: track.clone(), note_id: id.clone(), text: "海".into() },
            Command::SetPronunciation { track: track.clone(), note_id: id.clone(), pronunciation },
        ] }).unwrap();
        assert_eq!(store.undo_depth(), 1);
        let edited = store.project().track(&track).unwrap().note(&id).unwrap();
        assert_eq!(edited.pronunciation.pinyin, "hai3");
        assert_eq!(edited.midi_note, note.midi_note);
        assert_eq!(edited.curves, note.curves);
        let dictionary_note = edited.clone();
        let pronunciation = pronunciation_for_lyric(
            &dictionary_note.pronunciation, "海", "好", Some(("hao4", HANZI_PINYIN_PROFILE)),
        ).unwrap();
        store.apply(store.revision(), CommandBatch { label: "编辑歌词".into(), commands: vec![
            Command::SetLyric { track: track.clone(), note_id: id.clone(), text: "好".into() },
            Command::SetPronunciation { track: track.clone(), note_id: id.clone(), pronunciation },
        ] }).unwrap();
        assert_eq!(store.undo_depth(), 2);
        assert_eq!(store.project().track(&track).unwrap().note(&id).unwrap().pronunciation.source, PronunciationSource::Manual);
        store.undo().unwrap();
        assert_eq!(store.project().track(&track).unwrap().note(&id).unwrap(), &dictionary_note);
        store.undo().unwrap();
        assert_eq!(store.project().track(&track).unwrap().note(&id).unwrap(), &note);
    }
}
