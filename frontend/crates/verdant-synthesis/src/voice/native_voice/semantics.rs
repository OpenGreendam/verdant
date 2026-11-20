//! Frame semantics from `fullband_runtime.job_semantics` and
//! `fullband_semantics` at a3b0a1a3f14020f349d575f2bc1a66b05d3035ba.
//! Native fallback describes the selected acoustic phone. A validated authored
//! IPA trace overrides it only when its realization uses one native phone.
use std::sync::LazyLock;

use anyhow::{Context, Result, ensure};
use serde_json::Value;

use crate::{Arrays, NativePlan};
use super::super::validate_conditions;
const SEMANTIC_WIDTH: usize = 16;

type Feature = [f32; SEMANTIC_WIDTH];
const VOWEL: usize = 0;
const NASAL: usize = 1;
const APPROXIMANT: usize = 2;
const STOP: usize = 3;
const FRICATIVE: usize = 4;
const AFFRICATE: usize = 5;
const CLOSURE: usize = 6;
const LABIAL: usize = 7;
const ALVEOLAR: usize = 8;
const POSTALVEOLAR: usize = 9;
const PALATAL: usize = 10;
const VELAR: usize = 11;
const GLOTTAL: usize = 12;
const HIGH_VOWEL: usize = 13;
const ROUNDED: usize = 14;
const FRONT_VOWEL: usize = 15;
const VOWELS: &str = "aiueoɑɐæɒɛɜɞɤɚɝəɪɨɯʊʌɔœøʏy";

struct Inventory {
    declaration: Value,
    native: [Option<Feature>; 256],
}

static INVENTORY: LazyLock<std::result::Result<Inventory, String>> = LazyLock::new(|| {
    Inventory::load().map_err(|error| format!("{error:#}"))
});

fn marked(indices: &[usize]) -> Feature {
    let mut feature = [0.; SEMANTIC_WIDTH];
    for &index in indices { feature[index] = 1.; }
    feature
}

fn ipa_features(symbol: &str) -> Result<Feature> {
    ensure!(!symbol.is_empty(), "native semantics require an explicit IPA unit");
    let palatalized = symbol.contains('ʲ');
    let mut chars = symbol.chars().filter(|&c| !matches!(c, 'ː' | '͡' | 'ʰ' | 'ʲ'));
    let mut first = chars.next().context("native semantics require an explicit IPA unit")?;
    // A native compound rime keeps its nucleus, rather than sustaining its glide.
    if "jwɥ".contains(first) && chars.clone().next().is_some_and(|c| VOWELS.contains(c)) {
        first = chars.next().unwrap();
    }
    let second = chars.clone().next();
    let mut feature = if matches!(first, 'ɹ' | 'ɻ') && second == Some('̩') && chars.clone().nth(1).is_none() {
        marked(&[VOWEL, HIGH_VOWEL, if first == 'ɹ' { ALVEOLAR } else { POSTALVEOLAR }])
    } else if VOWELS.contains(first) {
        let mut feature = marked(&[VOWEL]);
        if "iɪɨɯuʊyʏ".contains(first) { feature[HIGH_VOWEL] = 1.; }
        if "uoʊɔɒœøyʏ".contains(first) { feature[ROUNDED] = 1.; }
        if "ieɛɪæœøyʏ".contains(first) { feature[FRONT_VOWEL] = 1.; }
        if chars.clone().any(|c| "nŋɲm".contains(c)) { feature[NASAL] = 1.; }
        feature
    } else if matches!((first, second),
        ('t', Some('s' | 'ɕ' | 'ʃ' | 'ʂ')) | ('d', Some('z' | 'ʑ' | 'ʒ' | 'ʐ')) |
        ('ʈ', Some('ʂ')) | ('ɖ', Some('ʐ'))) {
        let place = if chars.clone().any(|c| "ɕʑ".contains(c)) { PALATAL }
            else if chars.clone().any(|c| "ʃʒʂʐ".contains(c)) { POSTALVEOLAR }
            else { ALVEOLAR };
        marked(&[AFFRICATE, place])
    } else if "mnŋɲɴ".contains(first) {
        let place = match first { 'm' => LABIAL, 'n' => ALVEOLAR, 'ɲ' => PALATAL, _ => VELAR };
        marked(&[NASAL, place])
    } else if "lɫrɹɾɻwjɥ".contains(first) {
        let place = if "wɥ".contains(first) { LABIAL } else if first == 'j' { PALATAL } else { ALVEOLAR };
        let mut feature = marked(&[APPROXIMANT, place]);
        if first == 'ɫ' { feature[VELAR] = 1.; }
        feature
    } else if "pbtdkgɡqʔ".contains(first) {
        let place = if "pb".contains(first) { LABIAL } else if "td".contains(first) { ALVEOLAR }
            else if first == 'ʔ' { GLOTTAL } else { VELAR };
        marked(&[STOP, place])
    } else if "fvɸβszʃʒʂʐɕʑçxhɦθð".contains(first) {
        let place = if "fvɸβ".contains(first) { LABIAL } else if "szθð".contains(first) { ALVEOLAR }
            else if "ʃʒʂʐ".contains(first) { POSTALVEOLAR } else if "ɕʑç".contains(first) { PALATAL }
            else if first == 'x' { VELAR } else { GLOTTAL };
        marked(&[FRICATIVE, place])
    } else {
        anyhow::bail!("unmapped native IPA conditioning unit: {symbol}");
    };
    if palatalized { feature[PALATAL] = 1.; }
    Ok(feature)
}

fn native_fallback(key: &str) -> Option<&'static str> {
    match key {
        "en:ar" => Some("ɑ"), "en:axr" => Some("ɚ"), "en:er" => Some("ɜ"),
        "en:ir" => Some("ɪ"), "en:ur" => Some("ʊ"), "en:ix" => Some("ɨ"),
        "en:oo" => Some("o"), "en:ox" => Some("ɔ"), "en:ux" => Some("ʊ"),
        "en:rx" => Some("ɹ"), "en:el" => Some("l"), "en:em" => Some("m"),
        "en:en" | "en:nx" => Some("n"), "en:dx" => Some("ɾ"), _ => None,
    }
}

fn phone_id(row: &Value) -> Result<usize> {
    let id = row["phone_id"].as_u64().context("native inventory lacks an integer phone ID")?;
    ensure!(id < 256, "native inventory phone ID exceeds the native ABI");
    Ok(id as usize)
}

impl Inventory {
    fn load() -> Result<Self> {
        // The canonical public inventory preserves the frozen source bytes.
        // serde_json's workspace preserve_order feature retains the original
        // first-declared mapping for each native phone.
        let declaration: Value = serde_json::from_slice(include_bytes!("../../../../verdant-backend/src/data/phoneme_inventory.json"))?;
        ensure!(declaration["schema_version"] == "verdant-phonemes/2", "unsupported native phoneme inventory");
        let phones = declaration["native_phones"].as_object().context("native inventory lacks phones")?;
        let mut choices = [None; 256];
        for language in ["zh", "en"] {
            let mappings = declaration["languages"][language]["mappings"].as_object()
                .context("native inventory lacks language mappings")?;
            for (symbol, mapping) in mappings {
                let mapped = mapping["phones"].as_array().context("native inventory mapping lacks phones")?;
                if mapping["support"] == "native" && mapped.len() == 1 {
                    let key = mapped[0].as_str().context("native inventory key must be a string")?;
                    let id = phone_id(phones.get(key).context("native inventory mapping names an unknown phone")?)?;
                    choices[id].get_or_insert(symbol.as_str());
                }
            }
        }
        let mut native = [None; 256];
        for (key, row) in phones {
            let id = phone_id(row)?;
            ensure!(native[id].is_none(), "native inventory repeats a phone ID");
            let feature = if key.ends_with("cl") { marked(&[CLOSURE]) }
                else if key.ends_with("hsharp") || key.ends_with("pau") { [0.; SEMANTIC_WIDTH] }
                else {
                    let symbol = choices[id].or_else(|| native_fallback(key))
                        .or_else(|| (row["role"] == "nucleus").then_some("ə"))
                        .with_context(|| format!("native conditioning phone has no mapping: {key}"))?;
                    ipa_features(symbol)?
                };
            native[id] = Some(feature);
        }
        Ok(Self { declaration, native })
    }
}

fn authored_features(trace: &Value, native_key: &str, inventory: &Value) -> Result<Option<Feature>> {
    ensure!(trace.is_object(), "authored phoneme trace must be an object");
    let language = trace["source_language"].as_str().context("authored IPA trace requires source_language")?;
    let symbol = trace["canonical_ipa"].as_str().context("authored IPA trace requires canonical_ipa")?;
    let language_row = inventory["languages"].get(language).context("authored IPA trace has an unknown language")?;
    ensure!(trace["native_key"].as_str() == Some(native_key), "authored IPA trace must match its actual native path");
    let mut effective = symbol;
    if let Some(realization) = trace.get("realization").filter(|value| !value.is_null()) {
        ensure!(realization.is_object() && language == "ja" && symbol == "ɴ" &&
            realization["schema_version"] == "verdant-japanese-realization/1" && realization["source_mora"] == "ん" &&
            realization["source_phonological"] == "moraic_nasal", "unsupported authored realization trace");
        effective = realization["target_ipa"].as_str().context("authored nasal realization requires target_ipa")?;
        let following = realization["following_ipa"].as_str().context("authored nasal realization requires following_ipa")?;
        ensure!(inventory["japanese_kana"]["moraic_nasal_allophones"].get(following).and_then(Value::as_str) == Some(effective),
            "authored nasal realization is inconsistent");
    }
    let mapping = language_row["mappings"].get(effective).context("authored IPA trace and native phone mapping disagree")?;
    let phones = mapping["phones"].as_array().context("authored mapping lacks native phones")?;
    ensure!(phones.iter().any(|phone| phone.as_str() == Some(native_key)),
        "authored IPA trace and native phone mapping disagree");
    if phones.len() == 1 { Ok(Some(ipa_features(effective)?)) } else { Ok(None) }
}

/// Return row-major, exactly binary float32 `[frames, 16]` conditions.
pub(super) fn features(arrays: &Arrays, plan: &NativePlan) -> Result<Vec<f32>> {
    let frames = validate_conditions(arrays)?;
    ensure!(plan.duration_frames == frames, "native semantic plan and conditions have different frame clocks");
    let inventory = INVENTORY.as_ref().map_err(|error| anyhow::anyhow!("{error}"))?;
    let phones = &arrays["phone_ids"].values;
    let active = &arrays["active"].values;
    let mut semantic = vec![0.; frames * SEMANTIC_WIDTH];
    for (frame, row) in semantic.chunks_exact_mut(SEMANTIC_WIDTH).enumerate() {
        if active[frame] == 1. {
            let id = phones[frame * 3 + 1] as usize;
            row.copy_from_slice(inventory.native[id].as_ref().context("unknown active native phone ID")?);
        }
    }
    for run in &plan.runs {
        let run_end = run.start_frame.checked_add(run.frames).context("native semantic run frame overflow")?;
        ensure!(run_end <= frames, "native semantic run extends beyond the conditions");
        let segments = run.metadata["segments"].as_array().context("native job lacks segment traces")?;
        let counts = run.metadata["frame_counts"].as_array().context("native job lacks segment frame counts")?;
        let keys = run.metadata["native_keys"].as_array().context("native job lacks phone keys")?;
        ensure!(segments.len() == counts.len() && segments.len() == keys.len(), "native job segment arrays disagree");
        let mut first = run.start_frame;
        for ((segment, count), key) in segments.iter().zip(counts).zip(keys) {
            let count = usize::try_from(count.as_u64().context("segment frame count must be a nonnegative integer")?)
                .context("segment frame count overflow")?;
            let end = first.checked_add(count).context("native semantic segment frame overflow")?;
            ensure!(end <= run_end, "native semantic segment extends beyond its run");
            let key = key.as_str().context("segment native key must be a string")?;
            let expected = inventory.declaration["native_phones"].get(key).context("trace names an unknown native phone")?;
            let expected_id = phone_id(expected)? as f64;
            ensure!((first..end).all(|frame| phones[frame * 3 + 1] == expected_id),
                "native trace is not aligned to the condition archive");
            if let Some(trace) = segment.get("phoneme").filter(|value| !value.is_null()) {
                if let Some(feature) = authored_features(trace, key, &inventory.declaration)? {
                    for row in semantic[first * SEMANTIC_WIDTH..end * SEMANTIC_WIDTH].chunks_exact_mut(SEMANTIC_WIDTH) {
                        row.copy_from_slice(&feature);
                    }
                }
            }
            first = end;
        }
        ensure!(first == run_end, "segment frame counts do not cover their native run");
    }
    for (row, &on) in semantic.chunks_exact_mut(SEMANTIC_WIDTH).zip(active) {
        if on == 0. { row.fill(0.); }
    }
    Ok(semantic)
}
