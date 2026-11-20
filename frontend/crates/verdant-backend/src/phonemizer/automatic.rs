//! Automatic text routing. Explicit note and dictionary readings are handled
//! before this module. Parts retain their individual languages and readings;
//! the adapter divides the actual note duration equally, without permille rounding.
use serde::{Deserialize, Serialize};
use verdant_domain::{Pronunciation, PronunciationSource};
use super::{AUTOMATIC, ENGLISH_G2P, JAPANESE_KANA, MANDARIN};

pub(crate) const PARTS_SCHEMA: &str = "verdant-lyric-parts/1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Part {
    pub text: String,
    pub reading: Pronunciation,
}

pub(super) fn script(text: &str) -> Option<&'static str> {
    if text.chars().any(|c| matches!(c as u32, 0x3040..=0x30ff | 0xff66..=0xff9f)) { Some("ja") }
    else if text.chars().any(|c| matches!(c as u32, 0x3400..=0x9fff | 0x20000..=0x323af)) { Some("zh") }
    else { None }
}

fn pinyin(text: &str) -> Result<Pronunciation, String> {
    let token = crate::pinyin::parse_token(text).map_err(|e| e.0)?;
    Ok(Pronunciation {
        pinyin: token.label_token, input_text: Some(text.into()), source: PronunciationSource::ExplicitMap,
        frontend_profile: "direct-pinyin-v1".into(),
        reading_provenance: Some(serde_json::json!({"schema_version":"verdant-direct-pinyin/1", "input":text,
            "tone_policy":"Written tone is preserved; unmarked input uses the existing parser's tone-1 context. Authored note pitch is unchanged."})),
        ..Default::default()
    })
}

fn english(text: &str) -> Result<Pronunciation, String> {
    let converted = super::english::reading(text)?;
    let declaration = crate::phonemes::AlphabetDeclaration::new("arpabet")?;
    let canonical = crate::phonemes::parse_declared_alphabet(&converted.tokens, "en", &declaration, None)?;
    let ipa = crate::phonemes::format_canonical(&canonical, "ipa")?;
    Ok(Pronunciation {
        input_text: Some(text.into()), language: Some("en".into()), alphabet: Some("ipa".into()),
        phonemes: ipa.split_whitespace().map(str::to_string).collect(),
        source: PronunciationSource::ExplicitMap, frontend_profile: "english-g2p-v1".into(),
        // Unstressed rhotic schwa uses the inventory's documented er approximation.
        allow_approximation: true,
        reading_provenance: Some(serde_json::json!({"schema_version":"verdant-english-g2p/1", "basis":converted.basis,
            "exception":converted.exception, "arpabet":converted.tokens,
            "limitations":"Finite spelling rules and irregular-word lexicon; inferred stress and lexical pronunciation may need an explicit override."})),
        ..Default::default()
    })
}

fn japanese(text: &str) -> Result<Pronunciation, String> {
    // These imported aliases have an established meaning in the owner's score.
    // They are finite conventions, not a guessed general UTAU voicebank map.
    let alias = match text {
        "t" => Some("t"), "k" => Some("k"), "sh" => Some("ɕ"), "V" => Some("v"),
        _ => None,
    };
    if let Some(phone) = alias {
        return Ok(Pronunciation { input_text: Some(text.into()), language: Some("ja".into()),
            alphabet: Some("ipa".into()), phonemes: vec![phone.into()], allow_approximation: true,
            source: PronunciationSource::ExplicitMap, frontend_profile: "imported-japanese-alias-v1".into(),
            reading_provenance: Some(serde_json::json!({"schema_version":"verdant-imported-alias/1","alias":text})),
            ..Default::default() });
    }
    let kana = if text == "す2" { "す".into() }
        else if text == "に3" { "に".into() }
        else if script(text) == Some("ja") { text.into() }
        else { super::romaji::kana(text)? };
    let mut reading = crate::reading::compile_kana(&kana, "explicit", true)?;
    if kana != text {
        reading.input_text = Some(text.into());
        reading.reading_provenance.as_mut().unwrap()["authored_text"] = text.into();
    }
    reading.source = PronunciationSource::ExplicitMap;
    Ok(reading)
}

// Explicit hints and selected frontends win. In automatic mode script and the
// finite imported aliases decide first. A touching kana neighbour keeps an
// isolated romaji vowel Japanese, including English's ambiguous "a" and "I".
// Other known English words stay English unless touching Chinese evidence
// selects pinyin. A kana neighbour can then
// disambiguate valid romaji; otherwise marked/unmarked pinyin wins before the
// narrow multi-mora romaji spelling test. Remaining Latin fragments use the
// existing English spelling rules. Ambiguous Latin needs an explicit hint.
fn single(text: &str, selected: &str, hint: Option<&str>, local: Option<&str>, alias: bool) -> Result<Pronunciation, String> {
    let language = if let Some(hint) = hint { hint }
        else { match selected {
            MANDARIN => "zh", JAPANESE_KANA => "ja", ENGLISH_G2P => "en",
            AUTOMATIC => {
                if let Some(script) = script(text) { script }
                else if alias { "ja" }
                else if local == Some("ja") && matches!(text, "a" | "i" | "u" | "e" | "o" | "A" | "I" | "U" | "E" | "O") { "ja" }
                else if super::english::is_common_english_word(text) && local != Some("zh") { "en" }
                else if local == Some("ja") && super::romaji::kana(text).is_ok() { "ja" }
                else if crate::pinyin::parse_token(text).is_ok() { "zh" }
                else if super::romaji::is_unmistakable(text) { "ja" }
                else { "en" }
            },
            _ => return Err("当前发音器不进行自动文本转换".into()),
        }};
    match language {
        "zh" => {
            let candidates = verdant_domain::hanzi_readings(text);
            if !candidates.is_empty() {
                let preferred = super::resolution::polyphone::PREFERRED.iter()
                    .find(|(character, _)| *character == text).map(|(_, reading)| (*reading).to_string());
                let spelling = preferred.or_else(|| verdant_domain::hanzi_primary_reading(text))
                    .unwrap_or_else(|| candidates[0].clone());
                let mut reading = pinyin(&spelling)?;
                reading.input_text = Some(text.into());
                reading.frontend_profile = verdant_domain::HANZI_PINYIN_PROFILE.into();
                Ok(reading)
            } else { pinyin(text) }
        }
        "ja" => japanese(text),
        "en" => english(text),
        other => Err(format!("没有语言 {other} 的自动歌词转换器")),
    }
}

pub(super) fn reading(text: &str, selected: &str, hint: Option<&str>, local: Option<&str>) -> Result<Pronunciation, String> {
    let words: Vec<_> = text.split_whitespace().collect();
    if words.is_empty() { return Err("歌词为空；请填写歌词或明确静音".into()); }
    let alias = matches!(text.trim(), "i t" | "a t" | "e t" | "e k" | "u sh" | "o V" | "u V" | "i k" | "す2" | "に3");
    if words.len() == 1 { return single(words[0], selected, hint, local, alias); }
    let parts: Vec<Part> = words.into_iter().map(|word| {
        single(word, selected, hint, local, alias).map(|reading| Part { text: word.into(), reading })
    }).collect::<Result<_, _>>()?;
    let mut tokens = Vec::new();
    for part in &parts {
        let resolved = crate::reading::resolve_pronunciation(&part.reading)?;
        if !tokens.is_empty() { tokens.push(".".into()); }
        tokens.extend(crate::phonemes::format_canonical(&resolved.canonical, "ipa")?.split_whitespace().map(str::to_string));
    }
    Ok(Pronunciation {
        input_text: Some(text.into()), language: Some("mixed".into()), alphabet: Some("ipa".into()),
        phonemes: tokens, allow_approximation: true, source: PronunciationSource::ExplicitMap,
        frontend_profile: "automatic-parts-v1".into(),
        reading_provenance: Some(serde_json::json!({"schema_version":PARTS_SCHEMA, "parts":parts})),
        ..Default::default()
    })
}

/// Read only this module's complete part representation. Flat IPA must still
/// match the parts, so editing a reading cannot retain stale part semantics.
pub(crate) fn parts(reading: &Pronunciation) -> Result<Option<Vec<Part>>, String> {
    let Some(value) = &reading.reading_provenance else { return Ok(None); };
    if value["schema_version"] != PARTS_SCHEMA { return Ok(None); }
    let parts: Vec<Part> = serde_json::from_value(value["parts"].clone()).map_err(|e| e.to_string())?;
    if parts.len() < 2 || parts.len() > 512 { return Err("歌词分部数量须在 2–512 内".into()); }
    let mut flat = Vec::new();
    for part in &parts {
        if part.reading.reading_provenance.as_ref().is_some_and(|v| v["schema_version"] == PARTS_SCHEMA) { return Err("歌词分部不能嵌套".into()); }
        crate::reading::validate_pronunciation(&part.reading)?;
        let resolved = crate::reading::resolve_pronunciation(&part.reading)?;
        if !flat.is_empty() { flat.push(".".to_string()); }
        flat.extend(crate::phonemes::format_canonical(&resolved.canonical, "ipa")?.split_whitespace().map(str::to_string));
    }
    if reading.alphabet.as_deref() != Some("ipa") || flat != reading.phonemes {
        return Err("歌词分部与当前音素不一致；请重新生成读音".into());
    }
    Ok(Some(parts))
}
