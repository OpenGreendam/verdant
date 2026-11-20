//! Finite kana readings compiled into canonical IPA using the shared asset.
//!
//! This is an explicit reading helper, not Japanese text G2P. Kanji, particles,
//! pitch accent and vowel devoicing are not inferred. A separate rendering
//! pass can select conservative nasal place allophones with matching kana
//! provenance; canonical IPA remains unchanged. The resulting IPA still
//! requires the Japanese cross-language acoustic profile.
use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Inventory {
    japanese_kana: KanaInventory,
    units: BTreeMap<String, Unit>,
    languages: BTreeMap<String, Language>,
}

#[derive(Deserialize)]
struct Unit {
    role: String,
}

#[derive(Deserialize)]
struct Language {
    mappings: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct KanaInventory {
    schema_version: String,
    moras: BTreeMap<String, Vec<String>>,
    katakana_to_hiragana: BTreeMap<String, String>,
    unicode_aliases: BTreeMap<String, String>,
    geminate_onsets: Vec<String>,
    common_long_vowels: BTreeMap<String, String>,
}

fn inventory() -> &'static Inventory {
    static VALUE: LazyLock<Inventory> = LazyLock::new(|| {
        let inventory: Inventory = serde_json::from_str(include_str!("data/phoneme_inventory.json"))
            .expect("packaged kana inventory must be valid");
        assert_eq!(inventory.japanese_kana.schema_version, "verdant-kana/1");
        inventory
    });
    &VALUE
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanaMora {
    pub kana: String,
    pub phones: Vec<String>,
    pub mora_count: u32,
    pub word_index: usize,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KanaReading {
    /// IPA units with standalone dots between explicit mora groups.
    pub tokens: Vec<String>,
    pub canonical_ipa: String,
    pub original_input: String,
    pub long_vowel_policy: String,
    pub moras: Vec<KanaMora>,
}

fn normalized_reading(text: &str, data: &KanaInventory) -> String {
    // The finite shared aliases cover decomposed dakuten/handakuten and
    // halfwidth kana without adding a platform normalization dependency.
    let chars: Vec<char> = text.chars().collect();
    let mut normalized = String::new();
    let mut index = 0;
    while index < chars.len() {
        let mut matched = false;
        for width in [2, 1] {
            if index + width > chars.len() {
                continue;
            }
            let spelling: String = chars[index..index + width].iter().collect();
            if let Some(replacement) = data.unicode_aliases.get(&spelling) {
                normalized.push_str(replacement);
                index += width;
                matched = true;
                break;
            }
        }
        if !matched {
            normalized.push(chars[index]);
            index += 1;
        }
    }
    normalized
        .chars()
        .map(|ch| {
            let spelling = ch.to_string();
            data.katakana_to_hiragana.get(&spelling).cloned().unwrap_or(spelling)
        })
        .collect()
}

fn is_word_separator(ch: char) -> bool {
    ch.is_whitespace() || "、。，,.!?！？;；・".contains(ch)
}

pub(crate) fn geminate_onset(onset: &str) -> Option<String> {
    let inv = inventory();
    if !inv.japanese_kana.geminate_onsets.iter().any(|supported| supported == onset) {
        return None;
    }
    let long = format!("{onset}ː");
    inv.languages["ja"].mappings.contains_key(&long).then_some(long)
}

/// Compile an explicit kana reading into IPA and retain its provenance.
///
/// `explicit` honors ー. `common` additionally folds aa/ii/uu/ee/oo/ei/ou,
/// retaining a warning for each heuristic; spaces block such folding.
/// Long stops preserve the request but do not establish a native geminate
/// closure/release model. Use the canonical phoneme API to resolve the result.
pub fn kana_to_ipa(text: &str, long_vowels: &str) -> Result<KanaReading, String> {
    if text.trim().is_empty() {
        return Err("请输入非空假名读音".into());
    }
    if !matches!(long_vowels, "explicit" | "common") {
        return Err("假名长音策略只能为 explicit 或 common".into());
    }
    let inv = inventory();
    let data = &inv.japanese_kana;
    let normalized = normalized_reading(text, data);
    let words: Vec<&str> = normalized.split(is_word_separator).filter(|word| !word.is_empty()).collect();
    let mut moras: Vec<KanaMora> = Vec::new();
    for (word_index, word) in words.iter().enumerate() {
        let chars: Vec<char> = word.chars().collect();
        let mut index = 0;
        let mut geminate = false;
        let word_start = moras.len();
        while index < chars.len() {
            let ch = chars[index];
            if ch == 'っ' {
                if geminate {
                    return Err("连续小写っ需要明确的 IPA 输入".into());
                }
                geminate = true;
                index += 1;
                continue;
            }
            if ch == 'ー' {
                if geminate || moras.len() == word_start {
                    return Err("ー前必须有同一书写词内的元音音拍".into());
                }
                let previous = moras.last_mut().unwrap();
                let vowel = previous.phones.last().unwrap();
                let long_vowel = format!("{vowel}ː");
                if inv.units[vowel].role != "nucleus" || !inv.languages["ja"].mappings.contains_key(&long_vowel) {
                    return Err("前一音素不能再添加当前支持的长元音标记".into());
                }
                *previous.phones.last_mut().unwrap() = long_vowel;
                previous.kana.push(ch);
                previous.mora_count += 1;
                previous.assumptions.push("Written long mark preserved as canonical vowel length and an additional mora.".into());
                index += 1;
                continue;
            }
            let mut matched = None;
            for width in [2, 1] {
                if index + width <= chars.len() {
                    let spelling: String = chars[index..index + width].iter().collect();
                    if let Some(phones) = data.moras.get(&spelling) {
                        matched = Some((width, spelling, phones.clone()));
                        break;
                    }
                }
            }
            let (width, mut spelling, mut phones) =
                matched.ok_or_else(|| format!("「{ch}」不在有限假名读音表中；请提供假名读音或明确 IPA，不会猜测汉字读音"))?;
            let mut assumptions = Vec::new();
            let mut mora_count = 1;
            if geminate {
                let long_onset = geminate_onset(&phones[0])
                    .ok_or("小写っ后必须是已支持的阻碍音，不会静默替换")?;
                phones[0] = long_onset;
                spelling = format!("っ{spelling}");
                mora_count = 2;
                assumptions.push("Small tsu adds one mora and canonical consonant length; whole-phone stretching does not guarantee a correct geminate closure/release.".into());
                geminate = false;
            } else if long_vowels == "common" && phones.len() == 1 && moras.len() > word_start {
                let previous = moras.last_mut().unwrap();
                let pair = format!("{} {}", previous.phones.last().unwrap(), phones[0]);
                if let Some(folded) = data.common_long_vowels.get(&pair) {
                    *previous.phones.last_mut().unwrap() = folded.clone();
                    previous.kana.push_str(&spelling);
                    previous.mora_count += 1;
                    previous.assumptions.push("Author-selected common long-vowel spelling rule applied; lexical exceptions and hidden word boundaries are not inferred.".into());
                    index += width;
                    continue;
                }
            }
            moras.push(KanaMora { kana: spelling, phones, mora_count, word_index, assumptions });
            index += width;
        }
        if geminate {
            return Err("末尾小写っ没有后接辅音，不会猜测表情性喉塞音".into());
        }
    }
    if moras.is_empty() {
        return Err("至少需要一个假名音拍".into());
    }
    let mut tokens = Vec::new();
    for (index, mora) in moras.iter().enumerate() {
        if index > 0 {
            tokens.push(".".to_string());
        }
        tokens.extend(mora.phones.iter().cloned());
    }
    Ok(KanaReading {
        canonical_ipa: tokens.join(" "),
        tokens,
        original_input: text.to_string(),
        long_vowel_policy: long_vowels.to_string(),
        moras,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_rust_shared_kana_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/kana_v1.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap();
            let policy = case["long_vowels"].as_str().unwrap();
            let actual = kana_to_ipa(text, policy).unwrap();
            assert_eq!(serde_json::to_value(actual).unwrap(), case["expected"], "{text} / {policy}");
        }
        for case in fixture["errors"].as_array().unwrap() {
            assert!(kana_to_ipa(case["text"].as_str().unwrap(), case["long_vowels"].as_str().unwrap()).is_err(), "{case}");
        }
    }

    #[test]
    fn all_dictionary_readings_compile_to_advertised_ipa() {
        for (spelling, phones) in &inventory().japanese_kana.moras {
            let result = kana_to_ipa(spelling, "explicit").unwrap();
            assert_eq!(&result.moras[0].phones, phones, "{spelling}");
        }
    }
}
