//! Independent Python/Rust contract fixture: notation, IDs, state contexts,
//! exact duration budgets, and rejection of unsupported distinctions.
use serde_json::Value;
use verdant_backend::{kana, phonemes, pinyin};

fn tokens(value: &Value) -> Vec<String> {
    if let Some(s) = value.as_str() { s.split_whitespace().map(str::to_string).collect() }
    else { value.as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect() }
}
fn resolve(request: &Value) -> Result<phonemes::ResolvedPronunciation, String> {
    phonemes::resolve(&tokens(&request["tokens"]), request["language"].as_str().unwrap(),
        request["alphabet"].as_str().unwrap_or("ipa"), request["allow_approximation"].as_bool().unwrap_or(false),
        request["stress"].as_u64().map(|s| s as u8), request["tone"].as_u64().unwrap_or(1) as u8)
}
fn fixture() -> Value { serde_json::from_str(include_str!("fixtures/phonemes_v2.json")).unwrap() }

#[test]
fn explicit_phoneme_resolution_and_timing_match_python() {
    let fixture = fixture();
    assert_eq!(fixture["schema_version"], "verdant-phoneme-parity/2");
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let resolved = resolve(&case["request"]).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(serde_json::to_value(&resolved.canonical).unwrap(), case["canonical"], "{name}: canonical");
        let mut actual = serde_json::to_value(&resolved.phones).unwrap();
        for phone in actual.as_array_mut().unwrap() {
            let context = phone["context"].as_object_mut().unwrap();
            context.remove("label"); context.remove("assumptions");
            context.entry("anchor_questions").or_insert(serde_json::json!([]));
        }
        assert_eq!(actual, case["resolved"], "{name}: resolved contexts");
        for timing in case["duration_cases"].as_array().unwrap() {
            let weights: Option<Vec<u32>> = timing["weights"].as_array().map(|a| a.iter().map(|w| w.as_u64().unwrap() as u32).collect());
            let actual = phonemes::allocate_durations(&resolved.phones, timing["total"].as_f64().unwrap(), weights.as_deref());
            if timing.get("error").is_some() || timing.get("expected_error").is_some() {
                assert!(actual.is_err(), "{name}: expected timing error {timing}");
            } else {
                let actual = actual.unwrap_or_else(|e| panic!("{name}: {timing}: {e}"));
                let expected = timing["expected"].as_array().unwrap();
                assert_eq!(actual.len(), expected.len());
                for (a, e) in actual.iter().zip(expected) { assert!((a - e.as_f64().unwrap()).abs() < 1e-12, "{name}: {timing}"); }
            }
        }
    }
}

#[test]
fn pinyin_unicode_aliases_and_errors_match_python() {
    let fixture = fixture();
    for case in fixture["pinyin"].as_array().unwrap() {
        let raw = case["token"].as_str().unwrap();
        let result = pinyin::parse_token(raw).unwrap_or_else(|e| panic!("{raw}: {:?}", e));
        assert_eq!(serde_json::json!([result.syllable, result.tone, result.initial, result.final_]), case["expected"], "{raw}");
    }
    for request in fixture["resolve_errors"].as_array().unwrap() { assert!(resolve(request).is_err(), "{request}"); }
    for raw in fixture["pinyin_errors"].as_array().unwrap() { assert!(pinyin::parse_token(raw.as_str().unwrap()).is_err(), "{raw}"); }
}

#[test]
fn japanese_special_mora_and_contextual_realization_match_python_at_three_rates() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/japanese_prosody_v1.json")).unwrap();
    assert_eq!(fixture["schema_version"], "verdant-japanese-prosody-parity/1");
    for case in fixture["cases"].as_array().unwrap() {
        let parsed = kana::kana_to_ipa(case["text"].as_str().unwrap(), case["long_vowels"].as_str().unwrap()).unwrap();
        let mora_weights: Vec<u32> = parsed.moras.iter().map(|mora| mora.mora_count).collect();
        let expected = &case["expected"];
        assert_eq!(parsed.canonical_ipa, expected["canonical_ipa"].as_str().unwrap());
        assert_eq!(serde_json::to_value(&mora_weights).unwrap(), expected["mora_weights"]);
        let mut reading = phonemes::resolve(&parsed.tokens, "ja", "ipa", true, None, 1).unwrap();
        let provenance = serde_json::json!({"schema_version":"verdant-kana/1", "reading":parsed});
        phonemes::apply_kana_realization(&mut reading, Some(&provenance));
        assert_eq!(serde_json::to_value(reading.phones.iter().map(|phone| &phone.native_key).collect::<Vec<_>>()).unwrap(), expected["native_keys"], "{case}");
        assert_eq!(serde_json::to_value(reading.phones.iter().map(|phone| &phone.realization).collect::<Vec<_>>()).unwrap(), expected["realizations"], "{case}");
        let total = mora_weights.iter().sum::<u32>() as f64 * case["mora_seconds"].as_f64().unwrap();
        let actual = phonemes::allocate_for_note(&reading, total, None, Some(&mora_weights)).unwrap();
        for (a, e) in actual.iter().zip(expected["duration_seconds"].as_array().unwrap()) {
            assert!((a - e.as_f64().unwrap()).abs() < 1e-12, "{case}");
        }
        assert_eq!(actual.len(), expected["duration_seconds"].as_array().unwrap().len());
        assert!(actual.iter().all(|&seconds| seconds >= 0.025 - 1e-12));
        assert!((actual.iter().sum::<f64>() - total).abs() < 1e-12);
    }
}
