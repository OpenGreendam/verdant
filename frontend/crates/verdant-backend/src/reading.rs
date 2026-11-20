//! Editor reading inputs compile to the same canonical phonetic project form.
use verdant_domain::{Pronunciation, PronunciationSource};

pub fn parse_phonetic_input(input: &str) -> Result<Option<(String, Pronunciation)>, String> {
    parse_phonetic_input_impl(input, None)
}

/// Parse a selected singer's declared alphabet. UTAU aliases use `|` between
/// array elements in lyric annotations, retaining spaces inside VCV aliases:
/// `花/[ja/utau:- か|a な]`. The singer's map is required, never guessed.
/// Compiled readings store IPA so existing worker capabilities and duration
/// validation apply unchanged; the authored alphabet is retained as provenance.
pub fn parse_phonetic_input_with_alphabet(input: &str, declaration: &crate::phonemes::AlphabetDeclaration) -> Result<Option<(String, Pronunciation)>, String> {
    parse_phonetic_input_impl(input, Some(declaration))
}

fn parse_phonetic_input_impl(input: &str, declaration: Option<&crate::phonemes::AlphabetDeclaration>) -> Result<Option<(String, Pronunciation)>, String> {
    if let Some((lyric, annotation)) = input.trim().rsplit_once("/[") {
        if let Some(body) = annotation.strip_suffix(']') {
            if let Some((header, reading)) = body.split_once(':') {
                if let Some((language, alphabet)) = header.trim().split_once('/') {
                    if matches!(language, "ja" | "ja~" | "ja!") && matches!(alphabet, "kana" | "kana-common") {
                        let pronunciation = compile_kana(reading.trim(), if alphabet == "kana-common" { "common" } else { "explicit" }, language != "ja!")?;
                        return Ok(Some((lyric.trim().into(), pronunciation)));
                    }
                    let declared: crate::phonemes::PhoneticAlphabet = alphabet.parse()?;
                    if let Some(profile) = declaration {
                        if profile.alphabet != declared { return Err("歌词音标声明与所选歌手的输入音标体系不一致".into()); }
                    }
                    if matches!(declared, crate::phonemes::PhoneticAlphabet::Arpabet | crate::phonemes::PhoneticAlphabet::Utau) {
                        let default_profile = crate::phonemes::AlphabetDeclaration::new(alphabet)?;
                        let profile = declaration.unwrap_or(&default_profile);
                        let tokens: Vec<String> = if declared == crate::phonemes::PhoneticAlphabet::Utau {
                            reading.split('|').map(|alias| alias.trim().to_string()).collect()
                        } else { reading.split_whitespace().map(str::to_string).collect() };
                        let canonical = crate::phonemes::parse_declared_alphabet(&tokens, language.trim_end_matches(['~', '!']), profile, None)?;
                        let ipa = crate::phonemes::format_canonical(&canonical, "ipa")?;
                        let compiled = format!("{lyric}/[{language}/ipa:{ipa}]");
                        let (lyric, mut pronunciation) = verdant_domain::parse_phonetic_lyric(&compiled)?
                            .ok_or_else(|| "缺少明确的音标输入".to_string())?;
                        pronunciation.input_text = Some(reading.trim().into());
                        pronunciation.reading_provenance = Some(serde_json::json!({
                            "schema_version": "verdant-alphabet/1", "alphabet": alphabet,
                            "input_symbols": tokens, "canonical": canonical,
                        }));
                        return Ok(Some((lyric, pronunciation)));
                    }
                }
            }
        }
    }
    let parsed = verdant_domain::parse_phonetic_lyric(input)?;
    if let (Some(profile), Some((_, pronunciation))) = (declaration, &parsed) {
        if let Some(alphabet) = pronunciation.alphabet.as_deref() {
            if profile.alphabet != alphabet.parse()? { return Err("歌词音标声明与所选歌手的输入音标体系不一致".into()); }
            crate::phonemes::parse_declared_alphabet(&pronunciation.phonemes, pronunciation.language.as_deref().unwrap_or(""), profile, pronunciation.stress)?;
        }
    }
    Ok(parsed)
}

/// The same finite kana compiler serves explicit editor input and a selected
/// kana phonemizer. Kanji and lexical pronunciation are never inferred.
pub(crate) fn compile_kana(input: &str, long_vowels: &str, allow_approximation: bool) -> Result<Pronunciation, String> {
    let parsed = crate::kana::kana_to_ipa(input, long_vowels)?;
    Ok(Pronunciation {
        input_text: Some(parsed.original_input.clone()),
        language: Some("ja".into()),
        alphabet: Some("ipa".into()),
        phonemes: parsed.tokens.clone(),
        allow_approximation,
        mora_weights: Some(parsed.moras.iter().map(|mora| mora.mora_count).collect()),
        reading_provenance: Some(serde_json::json!({"schema_version":"verdant-kana/1", "reading":parsed})),
        source: PronunciationSource::Manual,
        frontend_profile: "kana-ipa-native-0.6".into(),
        ..Default::default()
    })
}

/// Validate dictionary readings against the existing finite pinyin and native
/// phoneme mappings. Note duration and worker/voice capability remain adapter
/// checks: accepting a spelling never promises acoustic support from a voice.
pub fn validate_pronunciation(reading: &Pronunciation) -> Result<(), String> {
    pronunciation_shape(reading).map(|_| ()).map_err(|error| error.message)
}

/// Compile automatic parts independently so each keeps its acoustic language.
/// Single pinyin readings use the same packaged initial/final IPA table.
pub fn resolve_pronunciation(reading: &Pronunciation) -> Result<crate::phonemes::ResolvedPronunciation, String> {
    if let Some(parts) = crate::phonemizer::automatic::parts(reading)? {
        let mut merged = crate::phonemes::ResolvedPronunciation {
            language: "mixed".into(), alphabet: "ipa".into(), canonical: Vec::new(), phones: Vec::new(), parts: Vec::new(),
        };
        let mut syllable_offset = 0;
        for part in parts {
            let mut resolved = resolve_pronunciation(&part.reading)?;
            merged.parts.push(crate::phonemes::ResolvedPart { text: part.text, language: resolved.language.clone(),
                phone_count: resolved.phones.len(), mora_weights: part.reading.mora_weights.clone() });
            let unit_offset = merged.canonical.len();
            let source_offset = merged.canonical.last().map_or(0, |u| u.source_index + 2);
            let groups = resolved.canonical.last().map_or(0, |u| u.syllable_index + 1);
            for unit in &mut resolved.canonical { unit.syllable_index += syllable_offset; unit.source_index += source_offset; }
            for phone in &mut resolved.phones {
                phone.canonical.syllable_index += syllable_offset;
                phone.canonical.source_index += source_offset;
                phone.unit_index += unit_offset;
            }
            merged.canonical.extend(resolved.canonical);
            merged.phones.extend(resolved.phones);
            syllable_offset += groups;
        }
        return Ok(merged);
    }
    if let Some(alphabet) = &reading.alphabet {
        return crate::phonemes::resolve(&reading.phonemes, reading.language.as_deref().unwrap_or(""), alphabet,
            reading.allow_approximation, reading.stress, 1);
    }
    let token = crate::pinyin::parse_token(&reading.pinyin).map_err(|e| e.0)?;
    static IPA: std::sync::LazyLock<serde_json::Value> = std::sync::LazyLock::new(||
        serde_json::from_str(include_str!("data/phoneme_inventory.json")).expect("packaged IPA inventory"));
    let mut tokens = Vec::new();
    if !token.initial.is_empty() {
        tokens.push(IPA["pinyin_ipa"]["initials"][&token.initial].as_str().ok_or("拼音声母没有 IPA 映射")?.to_string());
    }
    tokens.push(IPA["pinyin_ipa"]["finals"][&token.final_].as_str().ok_or("拼音韵母没有 IPA 映射")?.to_string());
    crate::phonemes::resolve(&tokens, "zh", "ipa", false, None, token.tone)
}

/// Author weights remain authoritative. Otherwise each whitespace part receives
/// exactly the same time; its own mora/CV policy and admitted phone floor apply.
pub(crate) fn allocate_reading(reading: &Pronunciation, resolved: &crate::phonemes::ResolvedPronunciation, total: f64, minimum: f64) -> Result<Vec<f64>, String> {
    crate::phonemes::allocate_for_note_with_minimum(resolved, total, reading.phoneme_durations_permille.as_deref(), reading.mora_weights.as_deref(), minimum)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReadingShape {
    pub nucleus: Option<usize>,
}

pub(crate) struct ReadingError {
    pub code: &'static str,
    pub message: String,
}

impl ReadingError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

pub(crate) fn pronunciation_shape(reading: &Pronunciation) -> Result<ReadingShape, ReadingError> {
    if reading.source == PronunciationSource::Unset {
        return Err(ReadingError::new("E_PRONUNCIATION_UNSET", "读音尚未确认；请选择一个读音并记录来源"));
    }
    let (count, nucleus) = if let Some(alphabet) = reading.alphabet.as_deref() {
        let _ = alphabet;
        let resolved = resolve_pronunciation(reading).map_err(|reason| ReadingError::new("E_UNSUPPORTED_PHONEME", reason))?;
        if let Some(weights) = &reading.mora_weights {
            let groups = resolved.canonical.last().map_or(0, |unit| unit.syllable_index + 1);
            if resolved.language != "ja" || weights.len() != groups || weights.contains(&0) {
                return Err(ReadingError::new("E_PHONEME_TIMING", "假名音拍权重须与明确的日语音素组对应且大于零"));
            }
            let mut expected = vec![1_u64; groups];
            for unit in &resolved.canonical {
                expected[unit.syllable_index] += u64::from(unit.length.saturating_sub(1));
            }
            if weights.iter().map(|&weight| u64::from(weight)).ne(expected) {
                return Err(ReadingError::new("E_PHONEME_TIMING", "假名音拍数量与明确的辅音／元音长音标记不一致"));
            }
        }
        (resolved.phones.len(), resolved.phones.iter().rposition(|phone| phone.native.role == "nucleus"))
    } else {
        if reading.language.as_deref().is_some_and(|language| language != "zh") {
            return Err(ReadingError::new("E_UNSUPPORTED_PHONEME", "非普通话读音需要明确的 language/alphabet 和音素，不能改用汉字拼音"));
        }
        if reading.mora_weights.is_some() {
            return Err(ReadingError::new("E_PHONEME_TIMING", "拼音读音不能使用假名音拍权重"));
        }
        // Preserve the old, explicitly authored English diagnostic profile.
        // This table is the existing native phone inventory, not new G2P.
        let legacy_vowel = reading.source != PronunciationSource::ExplicitMap
            && reading.frontend_profile == "english-vowel-proxy-native-0.2"
            && crate::phonemes::native_phone(&format!("en:{}", reading.pinyin.trim().to_ascii_lowercase())).is_some_and(|phone| phone.role == "nucleus");
        if legacy_vowel { (1, Some(0)) } else {
            let token = crate::pinyin::parse_token(&reading.pinyin)
                .map_err(|reason| ReadingError::new("E_UNSUPPORTED_SYLLABLE", reason.0))?;
            let count = if token.initial.is_empty() { 1 } else { 2 };
            (count, Some(count - 1))
        }
    };
    if let Some(weights) = &reading.phoneme_durations_permille {
        if weights.len() != count || weights.contains(&0) || weights.iter().map(|&weight| u64::from(weight)).sum::<u64>() != 1000 {
            return Err(ReadingError::new("E_PHONEME_TIMING", "音素时长比例须与实际展开音素对应、大于零且总和为 1000"));
        }
    }
    Ok(ReadingShape { nucleus })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phonemes::{allocate_for_note, resolve};
    #[test]
    fn kana_input_keeps_canonical_symbols_source_and_mora_timing() {
        let (lyric, pronunciation) = parse_phonetic_input("学校/[ja/kana-common:がっこう]").unwrap().unwrap();
        assert_eq!(lyric, "学校");
        assert_eq!(pronunciation.alphabet.as_deref(), Some("ipa"));
        assert_eq!(pronunciation.mora_weights, Some(vec![1, 3]));
        assert_eq!(pronunciation.phonemes.join(" "), "ɡ a . kː oː");
        let resolved = resolve(&pronunciation.phonemes, "ja", "ipa", true, None, 1).unwrap();
        let ds = allocate_for_note(&resolved, 0.8, None, pronunciation.mora_weights.as_deref()).unwrap();
        assert!((ds[0] + ds[1] - 0.2).abs() < 1e-12);
        assert!((ds[2] + ds[3] - 0.6).abs() < 1e-12);
        let json = serde_json::to_value(&pronunciation).unwrap();
        assert_eq!(json["reading_provenance"]["reading"]["original_input"], "がっこう");
        assert!(parse_phonetic_input("学校/[ja/kana:学校]").is_err());
        // Common long-vowel folding is never inferred from the default entry.
        let (_, explicit) = parse_phonetic_input("学校/[ja/kana:がっこう]").unwrap().unwrap();
        assert_eq!(explicit.mora_weights, Some(vec![1, 2, 1]));
        assert!(explicit.phonemes.iter().any(|p| p == "ɯ"));
    }
}
