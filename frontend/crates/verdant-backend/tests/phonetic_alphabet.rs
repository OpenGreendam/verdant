//! Alphabet conversion is independent of acoustic approximation. These tests
//! exercise the packaged finite inventory and the actual lyric reading path.
use std::collections::BTreeMap;
use verdant_backend::phonemes::{
    AlphabetDeclaration, PhoneticAlphabet, convert_alphabet, parse_declared_alphabet, resolve,
};
use verdant_backend::reading::{parse_phonetic_input, parse_phonetic_input_with_alphabet, validate_pronunciation};

fn words(text: &str) -> Vec<String> { text.split_whitespace().map(str::to_owned).collect() }

fn singer() -> AlphabetDeclaration {
    AlphabetDeclaration::utau(BTreeMap::from([
        ("- か".into(), words("k a")),
        ("a な".into(), words("n a")),
        ("あ".into(), words("a")),
        ("ん".into(), words("ɴ")),
    ])).unwrap()
}

#[test]
fn every_mapped_symbol_roundtrips_through_each_available_alphabet() {
    let profiles = [
        AlphabetDeclaration::new("ipa").unwrap(),
        AlphabetDeclaration::new("xsampa").unwrap(),
        AlphabetDeclaration::new("arpabet").unwrap(),
        AlphabetDeclaration::new("vocaloid-xsampa").unwrap(),
        singer(),
    ];
    let ipa = &profiles[0];
    for language in ["zh", "en", "ja"] {
        for source in &profiles {
            for (spelling, expected) in source.symbols(language).unwrap() {
                let tokens = vec![spelling.clone()];
                let canonical = convert_alphabet(&tokens, language, source, ipa).unwrap();
                assert!(canonical.losses.is_empty(), "{language}/{spelling}: {:?}", canonical.losses);
                assert_eq!(canonical.output_symbols, expected, "{language}/{spelling}");
                for target in &profiles {
                    let converted = convert_alphabet(&tokens, language, source, target).unwrap();
                    if converted.losses.is_empty() {
                        let back = convert_alphabet(&converted.output_symbols, language, target, ipa).unwrap();
                        assert!(back.losses.is_empty(), "{language}/{spelling}");
                        assert_eq!(back.output_symbols, expected, "{language}/{spelling} via {:?}", target.alphabet);
                    }
                }
            }
        }
    }
}

#[test]
fn unmapped_source_and_target_symbols_report_indices_without_erasing_input() {
    let ipa = AlphabetDeclaration::new("ipa").unwrap();
    let arpabet = AlphabetDeclaration::new("arpabet").unwrap();
    let tokens = words("k ɯː mystery ɾ a");
    let converted = convert_alphabet(&tokens, "ja", &ipa, &arpabet).unwrap();
    assert_eq!(converted.input_symbols, tokens);
    assert_eq!(converted.output_symbols, words("K"));
    assert_eq!(converted.losses.iter().map(|loss| loss.source_index).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(converted.losses[0].canonical_symbol.as_deref(), Some("ɯː"));
    assert_eq!(converted.losses[1].input_symbol, "mystery");
    assert!(converted.losses[1].canonical_symbol.is_none());
    assert!(converted.into_lossless_tokens().is_err());
    assert!(parse_declared_alphabet(&tokens, "ja", &ipa, None).unwrap_err().contains("mystery"));
}

#[test]
fn singer_aliases_expand_and_recombine_without_crossing_an_unknown_symbol() {
    let utau = singer();
    let ipa = AlphabetDeclaration::new("ipa").unwrap();
    let tokens = vec!["- か".into(), "a な".into(), "ん".into()];
    let converted = convert_alphabet(&tokens, "ja", &utau, &ipa).unwrap();
    assert!(converted.losses.is_empty());
    assert_eq!(converted.output_symbols, words("k a n a ɴ"));
    assert_eq!(converted.output_source_indices, [0, 0, 1, 1, 2]);
    let back = convert_alphabet(&converted.output_symbols, "ja", &ipa, &utau).unwrap();
    assert!(back.losses.is_empty());
    assert_eq!(back.output_symbols, tokens);
    let broken = convert_alphabet(&words("k unknown a"), "ja", &ipa, &utau).unwrap();
    assert_eq!(broken.output_symbols, ["あ"]);
    assert_eq!(broken.losses.iter().map(|loss| loss.source_index).collect::<Vec<_>>(), [0, 1]);
    let canonical = parse_declared_alphabet(&tokens, "ja", &utau, None).unwrap();
    assert_eq!(canonical.iter().map(|unit| unit.source_index).collect::<Vec<_>>(), [0, 0, 1, 1, 2]);
    assert_eq!(canonical[1].input_symbol, "- か");
}

#[test]
fn invalid_declarations_and_missing_or_invalid_singer_maps_are_rejected() {
    assert!("IPA-ish".parse::<PhoneticAlphabet>().is_err());
    assert!(AlphabetDeclaration::new("guessed").is_err());
    assert!(parse_phonetic_input("词/[en/guessed:AA]").is_err());
    assert!(parse_phonetic_input("花/[ja/utau:か]").is_err());
    assert!(AlphabetDeclaration::utau(BTreeMap::from([("か".into(), words("not-ipa"))])).is_err());
    assert!(parse_phonetic_input("词/[en/arpabet:AA3]").is_err());
    assert!(parse_phonetic_input("词/[en/arpabet:B1]").is_err());
    let ipa = AlphabetDeclaration::new("ipa").unwrap();
    assert!(parse_phonetic_input_with_alphabet("词/[en/arpabet:AA]", &ipa).is_err());
    assert!(parse_phonetic_input_with_alphabet("词/[en/ipa:unknown]", &ipa).is_err());
}

#[test]
fn arpabet_lyrics_compile_to_the_existing_acoustic_consumer() {
    let (lyric, pronunciation) = parse_phonetic_input("hello/[en/arpabet:HH AH0 . L OW1]").unwrap().unwrap();
    assert_eq!(lyric, "hello");
    assert_eq!(pronunciation.alphabet.as_deref(), Some("ipa"));
    assert_eq!(pronunciation.phonemes, words("h ə . ˈl oʊ"));
    validate_pronunciation(&pronunciation).unwrap();
    let resolved = resolve(&pronunciation.phonemes, "en", "ipa", false, None, 1).unwrap();
    assert_eq!(resolved.canonical.iter().map(|unit| unit.symbol.as_str()).collect::<Vec<_>>(), ["h", "ə", "l", "oʊ"]);
    assert_eq!(resolved.canonical.last().unwrap().stress, Some(1));
    assert_eq!(pronunciation.reading_provenance.as_ref().unwrap()["alphabet"], "arpabet");
    assert!(parse_phonetic_input("词/[en/arpabet:HH missing OW1]").is_err());
}

#[test]
fn stress_moves_between_ipa_onsets_and_arpabet_nuclei_without_an_extra_syllable() {
    let ipa = AlphabetDeclaration::new("ipa").unwrap();
    let arpabet = AlphabetDeclaration::new("arpabet").unwrap();
    let original = words("h ə . ˈl oʊ");
    let converted = convert_alphabet(&original, "en", &ipa, &arpabet).unwrap();
    assert!(converted.losses.is_empty());
    assert_eq!(converted.output_symbols, words("HH AH0 . L OW1"));
    let roundtrip = parse_declared_alphabet(&converted.output_symbols, "en", &arpabet, None).unwrap();
    assert_eq!(verdant_backend::phonemes::format_canonical(&roundtrip, "ipa").unwrap(), original.join(" "));
    assert_eq!(roundtrip[2].syllable_index, 1);
    assert_eq!(roundtrip[2].stress, Some(1));
    assert_eq!(roundtrip[3].syllable_index, 1);
    // Stress with no following nucleus cannot be silently erased.
    let missing = convert_alphabet(&words("ˈh"), "en", &ipa, &arpabet).unwrap();
    assert_eq!(missing.losses.len(), 1);
    assert_eq!(missing.losses[0].source_index, 0);
    assert_eq!(missing.losses[0].canonical_symbol.as_deref(), Some("ˈ"));
}

#[test]
fn singer_lyrics_retain_alias_provenance_and_use_existing_japanese_support() {
    let (lyric, pronunciation) = parse_phonetic_input_with_alphabet("花/[ja/utau:- か|a な]", &singer()).unwrap().unwrap();
    assert_eq!(lyric, "花");
    assert_eq!(pronunciation.phonemes, words("k a n a"));
    assert!(pronunciation.allow_approximation);
    validate_pronunciation(&pronunciation).unwrap();
    assert_eq!(pronunciation.reading_provenance.as_ref().unwrap()["input_symbols"], serde_json::json!(["- か", "a な"]));
}

#[test]
fn explicit_pinyin_and_kana_readings_keep_their_existing_exact_paths() {
    assert!(parse_phonetic_input("上/shang4").unwrap().is_none());
    assert_eq!(verdant_backend::pinyin::parse_token("shang4").unwrap().tone, 4);
    let (_, kana) = parse_phonetic_input("学校/[ja/kana-common:がっこう]").unwrap().unwrap();
    assert_eq!(kana.phonemes, words("ɡ a . kː oː"));
    assert_eq!(kana.mora_weights, Some(vec![1, 3]));
    assert_eq!(kana.reading_provenance.as_ref().unwrap()["schema_version"], "verdant-kana/1");
    let (_, explicit) = parse_phonetic_input("学校/[ja/kana:がっこう]").unwrap().unwrap();
    assert_eq!(explicit.phonemes, words("ɡ a . kː o . ɯ"));
    assert_eq!(explicit.mora_weights, Some(vec![1, 2, 1]));
}
