//! Declared phonetic alphabets compile to finite canonical IPA before the
//! existing acoustic mappings. Standard X-SAMPA and explicit VOCALOID dialects
//! use the packaged inventory; ARPABET uses broad CMU phone values, and UTAU
//! requires a singer-provided alias map. Conversion never guesses missing
//! phones: inspect `AlphabetConversion::losses` or consume its strict result.
//! The dictionary and timing policy share the archived oracle's JSON assets;
//! no Python code executes in this frontend.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use crate::wire::ScoreContext;

pub const SCHEMA: &str = "verdant-phonemes/2";
pub const PROFILE: &str = "ipa-xsampa-native-0.6";

/// A spelling system, separate from the voice's acoustic phone support.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhoneticAlphabet { Ipa, Xsampa, Arpabet, Utau, VocaloidXsampa }

impl std::str::FromStr for PhoneticAlphabet {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ipa" => Ok(Self::Ipa),
            "xsampa" => Ok(Self::Xsampa),
            "arpabet" => Ok(Self::Arpabet),
            "utau" => Ok(Self::Utau),
            "vocaloid-xsampa" => Ok(Self::VocaloidXsampa),
            _ => Err(format!("未知音标体系「{value}」；请选择 ipa、xsampa、arpabet、utau 或 vocaloid-xsampa")),
        }
    }
}

/// Singer-specific UTAU aliases must be supplied explicitly. A CV/VC/VCV
/// alias can map to several canonical IPA units; spaces inside an alias are
/// significant, so callers pass aliases as individual array elements.
#[derive(Clone, Debug)]
pub struct AlphabetDeclaration {
    pub alphabet: PhoneticAlphabet,
    pub utau_aliases: BTreeMap<String, Vec<String>>,
}

impl AlphabetDeclaration {
    pub fn new(name: &str) -> Result<Self, String> {
        Ok(Self { alphabet: name.parse()?, utau_aliases: BTreeMap::new() })
    }

    pub fn utau(aliases: BTreeMap<String, Vec<String>>) -> Result<Self, String> {
        let declaration = Self { alphabet: PhoneticAlphabet::Utau, utau_aliases: aliases };
        declaration.symbols("ja")?;
        Ok(declaration)
    }

    /// Finite spellings and their exact canonical values. ARPABET uses the
    /// CMU broad-phone convention (IY=/i/, UW=/u/, R=/ɹ/); it never invents
    /// vowel length. AH0 and ER0 denote /ə/ and /ɚ/. Other zero suffixes
    /// normalize to the unmarked phone, like accepted IPA spelling aliases.
    pub fn symbols(&self, language: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
        let inv = inventory();
        let mut symbols = BTreeMap::new();
        match self.alphabet {
            PhoneticAlphabet::Ipa | PhoneticAlphabet::Xsampa => {
                for (ipa, unit) in &inv.units {
                    symbols.insert(if self.alphabet == PhoneticAlphabet::Ipa { ipa.clone() } else { unit.xsampa.clone() }, vec![ipa.clone()]);
                }
                let aliases = if self.alphabet == PhoneticAlphabet::Ipa { &inv.ipa_aliases } else { &inv.xsampa_aliases };
                for (alias, ipa) in aliases { symbols.insert(alias.clone(), vec![ipa.clone()]); }
            }
            PhoneticAlphabet::Arpabet => {
                for &(spelling, ipa) in ARPABET {
                    symbols.insert(spelling.into(), vec![ipa.into()]);
                    if inv.units[ipa].role == "nucleus" {
                        let unstressed = match spelling { "AH" => "ə", "ER" => "ɚ", _ => ipa };
                        symbols.insert(format!("{spelling}0"), vec![unstressed.into()]);
                        symbols.insert(format!("{spelling}1"), vec!["ˈ".into(), ipa.into()]);
                        symbols.insert(format!("{spelling}2"), vec!["ˌ".into(), ipa.into()]);
                    }
                }
            }
            PhoneticAlphabet::VocaloidXsampa => {
                symbols.clone_from(&inv.languages.get(language)
                    .ok_or_else(|| format!("尚未提供语言 {language} 的 VOCALOID 方言"))?.vocaloid_import);
            }
            PhoneticAlphabet::Utau => {
                if self.utau_aliases.is_empty() { return Err("UTAU 音标体系需要所选歌手明确的别名到 IPA 映射".into()); }
                for (alias, ipa) in &self.utau_aliases {
                    if alias.trim() != alias || alias.is_empty() || alias == "." || alias.contains('|') || ipa.is_empty()
                        || ipa.iter().any(|symbol| !inv.units.contains_key(symbol)) {
                        return Err(format!("UTAU 别名「{alias}」必须对应一个或多个明确的规范 IPA 单元"));
                    }
                }
                symbols.clone_from(&self.utau_aliases);
            }
        }
        Ok(symbols)
    }
}

const ARPABET: &[(&str, &str)] = &[
    ("AA", "ɑ"), ("AE", "æ"), ("AH", "ʌ"), ("AO", "ɔ"), ("AW", "aʊ"), ("AY", "aɪ"),
    ("B", "b"), ("CH", "t͡ʃ"), ("D", "d"), ("DH", "ð"), ("EH", "ɛ"), ("ER", "ɝ"),
    ("EY", "eɪ"), ("F", "f"), ("G", "ɡ"), ("HH", "h"), ("IH", "ɪ"), ("IY", "i"),
    ("JH", "d͡ʒ"), ("K", "k"), ("L", "l"), ("M", "m"), ("N", "n"), ("NG", "ŋ"),
    ("OW", "oʊ"), ("OY", "ɔɪ"), ("P", "p"), ("R", "ɹ"), ("S", "s"), ("SH", "ʃ"),
    ("T", "t"), ("TH", "θ"), ("UH", "ʊ"), ("UW", "u"), ("V", "v"), ("W", "w"),
    ("Y", "j"), ("Z", "z"), ("ZH", "ʒ"),
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AlphabetLoss {
    pub source_index: usize,
    pub input_symbol: String,
    pub canonical_symbol: std::option::Option<std::string::String>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AlphabetConversion {
    /// Every source element is retained, even when it has no known meaning.
    pub input_symbols: Vec<String>,
    /// Available output in order. Never use this partial output as a reading
    /// without checking losses; use into_lossless_tokens for authoring.
    pub output_symbols: Vec<String>,
    /// Original source index for each output token, including expanded aliases.
    pub output_source_indices: Vec<usize>,
    pub losses: Vec<AlphabetLoss>,
}

impl AlphabetConversion {
    pub fn into_lossless_tokens(self) -> Result<Vec<String>, Vec<AlphabetLoss>> {
        if self.losses.is_empty() { Ok(self.output_symbols) } else { Err(self.losses) }
    }
}

/// Convert phonetic values, preserving case, length, boundaries and stress.
/// Equivalent spelling aliases normalize deterministically. Unmapped source
/// or target units produce an indexed loss for every unmapped canonical unit
/// or unknown source symbol (including its original source element);
/// unknown units also block matching across them. Multi-unit singer aliases
/// are matched longest first. No acoustic substitutions take place here.
pub fn convert_alphabet(tokens: &[String], language: &str, source: &AlphabetDeclaration, target: &AlphabetDeclaration) -> Result<AlphabetConversion, String> {
    let mut source_symbols = source.symbols(language)?;
    let mut target_symbols = target.symbols(language)?;
    // Canonical spellings win over alternative spellings on export.
    if target.alphabet == PhoneticAlphabet::Ipa {
        target_symbols.retain(|spelling, units| units.len() == 1 && spelling == &units[0]);
    } else if target.alphabet == PhoneticAlphabet::Xsampa {
        target_symbols.retain(|spelling, units| units.len() == 1 && inventory().units[&units[0]].xsampa == *spelling);
    }
    for symbols in [&mut source_symbols, &mut target_symbols] { symbols.insert(".".into(), vec![".".into()]); }
    let markers = |alphabet| match alphabet {
        PhoneticAlphabet::Ipa => Some([("ˈ", "ˈ"), ("ˌ", "ˌ")]),
        PhoneticAlphabet::Xsampa => Some([( "\"", "ˈ"), ("%", "ˌ")]),
        _ => None,
    };
    for (declaration, symbols) in [(source, &mut source_symbols), (target, &mut target_symbols)] {
        if let Some(marks) = markers(declaration.alphabet) {
            for (spelling, canonical) in marks { symbols.insert(spelling.into(), vec![canonical.into()]); }
        }
    }
    let mut result = AlphabetConversion { input_symbols: tokens.to_vec(), output_symbols: Vec::new(), output_source_indices: Vec::new(), losses: Vec::new() };
    let mut atoms: Vec<(usize, Option<String>)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if let Some(units) = source_symbols.get(token) {
            atoms.extend(units.iter().cloned().map(|unit| (index, Some(unit))));
        } else {
            let marked = markers(source.alphabet).and_then(|marks| marks.into_iter().find_map(|(mark, canonical)| {
                token.strip_prefix(mark).and_then(|phone| source_symbols.get(phone).map(|units| (canonical, units)))
            }));
            if let Some((mark, units)) = marked {
                atoms.push((index, Some(mark.into())));
                atoms.extend(units.iter().cloned().map(|unit| (index, Some(unit))));
            } else {
                atoms.push((index, None));
            }
        }
    }
    // A vowel suffix describes its explicitly delimited syllable, including
    // an onset before that vowel. Do not create a spurious onset-only group
    // in `HH AH0 . L OW1`. Without a boundary after an earlier nucleus, keep
    // the existing explicit frontend rule; lexical syllabification is not G2P.
    if source.alphabet == PhoneticAlphabet::Arpabet {
        for index in 0..atoms.len() {
            if matches!(atoms[index].1.as_deref(), Some("ˈ" | "ˌ")) {
                let start = atoms[..index].iter().rposition(|(_, atom)| atom.is_none() || atom.as_deref() == Some(".")).map_or(0, |boundary| boundary + 1);
                if atoms[start..index].iter().all(|(_, atom)| atom.as_deref().is_some_and(|symbol| inventory().units.get(symbol).is_some_and(|unit| unit.role != "nucleus"))) {
                    let marker = atoms.remove(index);
                    atoms.insert(start, marker);
                }
            }
        }
    }
    // ARPABET encodes stress on the nucleus rather than the onset. Move
    // only within the same explicit group, preserving the marker's source
    // identity so a group without a mapped nucleus reports its loss.
    if target.alphabet == PhoneticAlphabet::Arpabet {
        for index in 0..atoms.len() {
            if matches!(atoms[index].1.as_deref(), Some("ˈ" | "ˌ")) {
                let nucleus = atoms[index + 1..].iter()
                    .take_while(|(_, atom)| atom.as_deref().is_some_and(|symbol| !matches!(symbol, "." | "ˈ" | "ˌ")))
                    .position(|(_, atom)| atom.as_deref().is_some_and(|symbol| inventory().units.get(symbol).is_some_and(|unit| unit.role == "nucleus")));
                if let Some(relative) = nucleus {
                    let marker = atoms.remove(index);
                    atoms.insert(index + relative, marker);
                }
            }
        }
    }
    let mut offset = 0;
    while offset < atoms.len() {
        let (source_index, canonical) = &atoms[offset];
        let candidate = target_symbols.iter().filter(|(_, units)| {
            offset + units.len() <= atoms.len() && units.iter().zip(&atoms[offset..]).all(|(unit, (_, atom))| atom.as_ref() == Some(unit))
        }).max_by(|(left, a), (right, b)| a.len().cmp(&b.len()).then_with(|| right.cmp(left)));
        if let Some((spelling, units)) = candidate {
            result.output_symbols.push(spelling.clone());
            result.output_source_indices.push(*source_index);
            offset += units.len();
        } else {
            result.losses.push(AlphabetLoss {
                source_index: *source_index, input_symbol: tokens[*source_index].clone(), canonical_symbol: canonical.clone(),
                reason: if canonical.is_some() { "目标音标体系没有该符号的无损映射" } else { "源音标体系没有该符号的明确映射" }.into(),
            });
            offset += 1;
        }
    }
    Ok(result)
}

/// Validate a declared spelling and compile it to the existing canonical
/// frontend. Language/voice acoustic support remains resolve's responsibility.
pub fn parse_declared_alphabet(tokens: &[String], language: &str, declaration: &AlphabetDeclaration, stress: Option<u8>) -> Result<Vec<CanonicalUnit>, String> {
    let ipa = AlphabetDeclaration::new("ipa")?;
    let conversion = convert_alphabet(tokens, language, declaration, &ipa)?;
    if !conversion.losses.is_empty() {
        return Err(conversion.losses.iter().map(|loss| format!("音素 {}「{}」：{}", loss.source_index + 1, loss.input_symbol, loss.reason)).collect::<Vec<_>>().join("；"));
    }
    let mut canonical = canonicalize(&conversion.output_symbols, language, "ipa", stress)?;
    for unit in &mut canonical {
        unit.source_index = conversion.output_source_indices[unit.source_index];
        unit.input_symbol.clone_from(&tokens[unit.source_index]);
    }
    Ok(canonical)
}

#[derive(Deserialize)]
struct Inventory {
    schema_version: String,
    supported_alphabets: Vec<String>,
    units: BTreeMap<String, Unit>,
    languages: BTreeMap<String, Language>,
    native_phones: BTreeMap<String, NativePhone>,
    ipa_aliases: BTreeMap<String, String>,
    xsampa_aliases: BTreeMap<String, String>,
    duration_policy: DurationPolicy,
    japanese_kana: KanaRealizationPolicy,
}
#[derive(Deserialize)]
struct Unit { xsampa: String, length: u32, role: String }
#[derive(Deserialize)]
struct Language {
    mappings: BTreeMap<String, Mapping>,
    vocaloid_import: BTreeMap<String, Vec<String>>,
}
#[derive(Deserialize)]
struct Mapping { phones: Vec<String>, support: String, approximation_reason: Option<String> }
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NativePhone {
    pub native_language: String,
    pub native_phone: String,
    pub phone_id: u32,
    pub context_field: String,
    pub role: String,
    pub voiced: bool,
    #[serde(default)]
    pub anchor_questions: Vec<String>,
}
#[derive(Deserialize)]
struct DurationPolicy { min_phone_seconds: f64, consonant_seconds: f64, consonant_share: f64, max_total_seconds: f64 }
#[derive(Deserialize)]
struct KanaRealizationPolicy { moraic_nasal_allophones: BTreeMap<String, String> }
fn inventory() -> &'static Inventory {
    static VALUE: LazyLock<Inventory> = LazyLock::new(|| {
        let data: Inventory = serde_json::from_str(include_str!("data/phoneme_inventory.json"))
            .expect("packaged phoneme inventory must be valid");
        assert_eq!(data.schema_version, SCHEMA);
        data
    });
    &VALUE
}

pub fn native_phone(key: &str) -> Option<&'static NativePhone> { inventory().native_phones.get(key) }

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CanonicalUnit {
    pub symbol: String,
    pub source_index: usize,
    pub input_symbol: String,
    pub syllable_index: usize,
    pub stress: Option<u8>,
    pub length: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedPhone {
    /// Target mapping language, retained even for Japanese cross-language models.
    /// The wire PhonemeTrace publishes it explicitly; the established resolved
    /// phone JSON shape stays shared with the retained Python oracle.
    #[serde(skip)]
    pub source_language: String,
    #[serde(flatten)]
    pub canonical: CanonicalUnit,
    #[serde(flatten)]
    pub native: NativePhone,
    pub unit_index: usize,
    pub native_key: String,
    pub support: String,
    pub approximation_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realization: Option<PhonemeRealization>,
    pub context: ScoreContext,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PhonemeRealization {
    pub schema_version: String,
    pub source_mora: String,
    pub source_phonological: String,
    pub target_ipa: String,
    pub following_ipa: String,
    pub rule: String,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedPronunciation {
    pub language: String,
    pub alphabet: String,
    pub canonical: Vec<CanonicalUnit>,
    pub phones: Vec<ResolvedPhone>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<ResolvedPart>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedPart {
    pub text: String,
    pub language: String,
    pub phone_count: usize,
    pub mora_weights: Option<Vec<u32>>,
}

pub fn canonicalize(tokens: &[String], language: &str, alphabet: &str, stress: Option<u8>) -> Result<Vec<CanonicalUnit>, String> {
    let inv = inventory();
    if stress.is_some_and(|s| s > 2) { return Err("重音只能为 0、1、2".into()); }
    if !inv.supported_alphabets.iter().any(|a| a == alphabet) { return Err(format!("未知音标体系 {alphabet}；请选择 ipa 或 xsampa")); }
    let lang = inv.languages.get(language).ok_or_else(|| format!("尚未提供语言 {language} 的声学映射；可用 zh / en / ja"))?;
    if tokens.is_empty() { return Err("至少需要一个音素".into()); }
    let mut result = Vec::new();
    let mut current_stress = stress;
    let mut syllable = 0;
    let mut awaiting = false;
    for (source_index, raw) in tokens.iter().enumerate() {
        if raw.trim() != raw || raw.split_whitespace().count() != 1 { return Err("每个音素必须是单个非空符号；请用空格分隔".into()); }
        let mut token = raw.as_str();
        if token == "." {
            if result.is_empty() || awaiting { return Err("音节边界必须位于两个音素之间".into()); }
            syllable += 1; current_stress = stress; awaiting = true; continue;
        }
        let first = token.chars().next().unwrap();
        let marker = match (alphabet, first) {
            ("ipa", 'ˈ') | ("xsampa", '"') => Some(1),
            ("ipa", 'ˌ') | ("xsampa", '%') => Some(2),
            _ => None,
        };
        if let Some(mark) = marker {
            if awaiting && source_index > 0 && tokens[source_index - 1] != "." { return Err("两个重音标记之间缺少音素".into()); }
            if !result.is_empty() && !awaiting { syllable += 1; }
            current_stress = Some(mark); awaiting = true;
            token = &token[first.len_utf8()..];
            if token.is_empty() { continue; }
        }
        let symbols = match alphabet {
            "ipa" => {
                let symbol = inv.ipa_aliases.get(token).map(String::as_str).unwrap_or(token);
                inv.units.contains_key(symbol).then(|| vec![symbol.to_string()])
            }
            "xsampa" => inv.xsampa_aliases.get(token).cloned()
                .or_else(|| inv.units.iter().find(|(_, unit)| unit.xsampa == token).map(|(ipa, _)| ipa.clone()))
                .map(|symbol| vec![symbol]),
            "vocaloid-xsampa" => lang.vocaloid_import.get(token).cloned(),
            _ => unreachable!(),
        }.ok_or_else(|| format!("不支持 {alphabet} 符号「{raw}」；大小写、长音和附加符号具有区别"))?;
        for symbol in symbols {
            let length = inv.units[&symbol].length;
            result.push(CanonicalUnit { symbol, source_index, input_symbol: raw.clone(), syllable_index: syllable, stress: current_stress, length });
        }
        awaiting = false;
    }
    if awaiting || result.is_empty() { return Err("末尾的重音或音节标记后缺少音素".into()); }
    Ok(result)
}

pub fn format_canonical(units: &[CanonicalUnit], alphabet: &str) -> Result<String, String> {
    if !matches!(alphabet, "ipa" | "xsampa") { return Err("仅 IPA 和标准 X-SAMPA 支持无损导出".into()); }
    let mut result = Vec::new();
    let mut last_syllable = None;
    let mut last_stress = None;
    for unit in units {
        if last_syllable.is_some_and(|s| s != unit.syllable_index) { result.push(".".into()); last_stress = None; }
        let mut symbol = if alphabet == "ipa" { unit.symbol.clone() } else { inventory().units[&unit.symbol].xsampa.clone() };
        if unit.stress != last_stress {
            let mark = match (alphabet, unit.stress) { ("ipa", Some(1)) => "ˈ", ("ipa", Some(2)) => "ˌ", (_, Some(1)) => "\"", (_, Some(2)) => "%", _ => "" };
            symbol = format!("{mark}{symbol}");
        }
        result.push(symbol); last_syllable = Some(unit.syllable_index); last_stress = unit.stress;
    }
    Ok(result.join(" "))
}

pub fn resolve(tokens: &[String], language: &str, alphabet: &str, allow_approximation: bool, stress: Option<u8>, tone: u8) -> Result<ResolvedPronunciation, String> {
    if tone > 4 { return Err("声调上下文只能为 0–4".into()); }
    let inv = inventory();
    let canonical = canonicalize(tokens, language, alphabet, stress)?;
    let mut phones = Vec::new();
    for (unit_index, unit) in canonical.iter().enumerate() {
        let mapping = inv.languages[language].mappings.get(&unit.symbol)
            .ok_or_else(|| format!("{language} 映射中没有 IPA /{}/；不会静默替换成其他音", unit.symbol))?;
        if mapping.support != "native" && !allow_approximation {
            return Err(format!("/{}/ 需要显式启用跨语言近似映射：{}", unit.symbol, mapping.approximation_reason.as_deref().unwrap_or("无对应原生音素")));
        }
        for native_key in &mapping.phones {
            let native = inv.native_phones[native_key].clone();
            let english = native.native_language == "en";
            let mut features = BTreeMap::from([("C_Phone".into(), native.phone_id.to_string()), (native.context_field.clone(), native.native_phone.clone())]);
            if english {
                features.insert("C_Language".into(), "3".into());
                features.insert("C_Stress".into(), unit.stress.unwrap_or(0).to_string());
            } else { features.insert("C_Tone".into(), tone.to_string()); }
            let mut answers = BTreeMap::from([("C_EN".into(), english), ("C_SM".into(), native.context_field == "C_SM"), ("C_SilSp".into(), false)]);
            for field in ["C_Vowel", "C_Stop", "C_Nasal", "C_Fric"] { answers.insert(field.into(), native.context_field == field); }
            let mut assumptions: Vec<String> = [
                "Current native phone IDs and predicate groups follow the recovered original binary profile.",
                "IPA is canonical; model mappings use broad phonemic correspondence, not a guarantee of exact allophonic realization.",
                "Explicit frontend syllable boundaries determine English onset/coda; original text/prosodic hierarchy is not reconstructed.",
                "English stress defaults to 0 when unspecified; Japanese pitch accent is not inferred.",
            ].into_iter().map(str::to_string).collect();
            if let Some(reason) = &mapping.approximation_reason { assumptions.push(reason.clone()); }
            let context = ScoreContext {
                label: format!("explicit_{}_{}", native.native_language, native.native_phone), features, answers,
                anchor_questions: native.anchor_questions.clone(), assumptions, ..Default::default()
            };
            phones.push(ResolvedPhone { source_language: language.into(), canonical: unit.clone(), native, unit_index, native_key: native_key.clone(), support: mapping.support.clone(), approximation_reason: mapping.approximation_reason.clone(), realization: None, context });
        }
    }
    let ids: Vec<_> = phones.iter().map(|p| p.native.phone_id).collect();
    let mut seen_nucleus = BTreeSet::new();
    let mut nuclei_per_syllable = BTreeMap::<usize, usize>::new();
    let mut languages_per_syllable = BTreeMap::<usize, BTreeSet<String>>::new();
    for unit in &canonical {
        if inv.units[&unit.symbol].role == "nucleus" { *nuclei_per_syllable.entry(unit.syllable_index).or_default() += 1; }
    }
    for phone in &phones { languages_per_syllable.entry(phone.canonical.syllable_index).or_default().insert(phone.native.native_language.clone()); }
    for (index, phone) in phones.iter_mut().enumerate() {
        phone.context.features.insert("L_Phone".into(), if index == 0 { 0 } else { ids[index - 1] }.to_string());
        phone.context.features.insert("R_Phone".into(), ids.get(index + 1).copied().unwrap_or(0).to_string());
        if phone.native.native_language == "en" && nuclei_per_syllable.get(&phone.canonical.syllable_index) == Some(&1)
            && languages_per_syllable.get(&phone.canonical.syllable_index).is_some_and(|set| set.len() == 1 && set.contains("en")) {
            phone.context.features.insert("C_OnsetCoda".into(), u8::from(seen_nucleus.contains(&phone.canonical.syllable_index)).to_string());
        }
        if phone.native.role == "nucleus" { seen_nucleus.insert(phone.canonical.syllable_index); }
        phone.context.assumptions.push("L/R phone IDs use the supplied sequence's immediate adjacency; 0 means absent boundary, not a synthesized silence phone. Original neighboring-node iterators remain unrecovered.".into());
    }
    Ok(ResolvedPronunciation { language: language.into(), alphabet: alphabet.into(), canonical, phones, parts: Vec::new() })
}

/// A stored reading is advisory: current canonical units are authoritative.
pub fn matching_kana_reading(reading: &ResolvedPronunciation, provenance: Option<&serde_json::Value>) -> Option<crate::kana::KanaReading> {
    if reading.language != "ja" || reading.canonical.iter().any(|unit| unit.stress.is_some()) { return None; }
    let provenance = provenance?;
    if provenance["schema_version"] != "verdant-kana/1" { return None; }
    let kana = serde_json::from_value::<crate::kana::KanaReading>(provenance["reading"].clone()).ok()?;
    if crate::kana::kana_to_ipa(&kana.original_input, &kana.long_vowel_policy).ok().as_ref() != Some(&kana)
        || format_canonical(&reading.canonical, "ipa").ok().as_deref() != Some(kana.canonical_ipa.as_str()) { return None; }
    Some(kana)
}

/// Select a conservative kana /N/ allophone without changing canonical IPA.
/// Explicit IPA /ɴ/, edited canonical readings and written word boundaries
/// never acquire an unrequested substitution from old reading provenance.
pub fn apply_kana_realization(reading: &mut ResolvedPronunciation, provenance: Option<&serde_json::Value>) {
    let Some(kana) = matching_kana_reading(reading, provenance) else { return; };
    let allophones = &inventory().japanese_kana.moraic_nasal_allophones;
    for phone in &mut reading.phones {
        let group = phone.canonical.syllable_index;
        let Some(mora) = kana.moras.get(group) else { continue; };
        let Some(following) = kana.moras.get(group + 1) else { continue; };
        if mora.kana != "ん" || mora.phones != ["ɴ"] || mora.word_index != following.word_index { continue; }
        let onset = following.phones[0].strip_suffix('ː').unwrap_or(&following.phones[0]);
        let Some(target) = allophones.get(onset) else { continue; };
        let mut realized = resolve(&[target.clone()], "ja", "ipa", true, None, 1)
            .expect("packaged kana allophone must have an explicit Japanese mapping").phones.remove(0);
        realized.canonical = phone.canonical.clone();
        realized.unit_index = phone.unit_index;
        realized.realization = Some(PhonemeRealization {
            schema_version: "verdant-japanese-realization/1".into(), source_mora: "ん".into(),
            source_phonological: "moraic_nasal".into(), target_ipa: target.clone(), following_ipa: onset.into(),
            rule: "same-written-word nasal place assimilation".into(),
        });
        let reason = format!("Kana ん retains its canonical /ɴ/ transcription; a same-written-word rule selects approximate [{target}] before /{onset}/. This is an explicit contextual realization, not a native Japanese acoustic model.");
        realized.approximation_reason = Some(format!("{reason} {}", realized.approximation_reason.as_deref().unwrap_or("")));
        realized.context.assumptions.push(reason);
        *phone = realized;
    }
    let ids: Vec<_> = reading.phones.iter().map(|phone| phone.native.phone_id).collect();
    for (index, phone) in reading.phones.iter_mut().enumerate() {
        phone.context.features.insert("L_Phone".into(), if index == 0 { 0 } else { ids[index - 1] }.to_string());
        phone.context.features.insert("R_Phone".into(), ids.get(index + 1).copied().unwrap_or(0).to_string());
    }
}

fn consonant_groups(phones: &[ResolvedPhone], indices: &[usize]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for &index in indices {
        let same = groups.last().is_some_and(|members| {
            let first = &phones[members[0]];
            first.unit_index == phones[index].unit_index
                && first.canonical.syllable_index == phones[index].canonical.syllable_index
        });
        if same { groups.last_mut().unwrap().push(index); }
        else { groups.push(vec![index]); }
    }
    groups
}

fn allocate_consonant_budget(phones: &[ResolvedPhone], groups: &[Vec<usize>], durations: &mut [f64], budget: f64, floor: f64) {
    let count = groups.iter().map(Vec::len).sum::<usize>();
    let extra = budget - count as f64 * floor;
    let length_sum = groups.iter().map(|members| phones[members[0]].canonical.length as f64).sum::<f64>();
    for members in groups {
        let phone = &phones[members[0]];
        let group_extra = extra * phone.canonical.length as f64 / length_sum;
        let symbol = phone.canonical.symbol.strip_suffix('ː').unwrap_or(&phone.canonical.symbol);
        // A model /j/ helper is a short palatal gesture, not a second full
        // consonant budget. Gemination must not sustain this helper either.
        let palatal_helper = members.len() > 1 && (symbol.ends_with('ʲ') || symbol == "ɲ")
            && members[1..].iter().all(|&i| phones[i].native_key == "en:y");
        let targets = if palatal_helper { &members[..1] } else { &members[..] };
        for &index in targets { durations[index] += group_extra / targets.len() as f64; }
    }
}

pub fn allocate_durations(phones: &[ResolvedPhone], total: f64, weights: Option<&[u32]>) -> Result<Vec<f64>, String> {
    allocate_durations_at_minimum(phones, total, weights, inventory().duration_policy.min_phone_seconds)
}

fn allocate_durations_at_minimum(phones: &[ResolvedPhone], total: f64, weights: Option<&[u32]>, floor: f64) -> Result<Vec<f64>, String> {
    let policy = &inventory().duration_policy;
    let count = phones.len();
    if !total.is_finite() || count == 0 || total < count as f64 * floor - 1e-12 || total > policy.max_total_seconds {
        return Err(format!("每个实际音素至少需要 {:.0} ms，{} 段总时长须在 {:.3}–30 秒内", floor * 1000.0, count, count as f64 * floor));
    }
    let mut durations = vec![floor; count];
    if let Some(weights) = weights {
        if weights.len() != count || weights.contains(&0) { return Err("时长权重必须与实际展开音素对应且大于零".into()); }
        let sum = weights.iter().map(|&v| v as f64).sum::<f64>();
        for (d, &w) in durations.iter_mut().zip(weights) { *d = total * (w as f64 / sum); }
        if durations.iter().any(|&d| d < floor - 1e-12) { return Err(format!("时长比例使某个音素少于 {:.0} ms", floor * 1000.0)); }
    } else {
        let nuclei: Vec<_> = (0..count).filter(|&i| phones[i].native.role == "nucleus").collect();
        let consonants: Vec<_> = (0..count).filter(|i| !nuclei.contains(i)).collect();
        let groups = consonant_groups(phones, &consonants);
        let length = |i: usize| phones[i].canonical.length as f64;
        if nuclei.is_empty() {
            allocate_consonant_budget(phones, &groups, &mut durations, total, floor);
        } else {
            let sum = groups.iter().map(|members| length(members[0])).sum::<f64>();
            let budget = (policy.consonant_seconds * sum).min(policy.consonant_share * total)
                .max(consonants.len() as f64 * floor).min(total - nuclei.len() as f64 * floor);
            if !consonants.is_empty() { allocate_consonant_budget(phones, &groups, &mut durations, budget, floor); }
            let extra = total - budget - nuclei.len() as f64 * floor;
            let sum = nuclei.iter().map(|&i| length(i)).sum::<f64>();
            for &i in &nuclei { durations[i] += extra * length(i) / sum; }
        }
    }
    durations[count - 1] = total - durations[..count - 1].iter().sum::<f64>();
    Ok(durations)
}

/// Reserve the extra mora in っ or ー for its own consonant or nucleus.
/// Native stop closure/release remains an explicitly approximate acoustic path.
pub fn allocate_mora_durations(phones: &[ResolvedPhone], total: f64, mora_count: u32) -> Result<Vec<f64>, String> {
    allocate_mora_durations_at_minimum(phones, total, mora_count, inventory().duration_policy.min_phone_seconds)
}

fn allocate_mora_durations_at_minimum(phones: &[ResolvedPhone], total: f64, mora_count: u32, minimum: f64) -> Result<Vec<f64>, String> {
    if mora_count == 0 || !total.is_finite() || total <= 0.0 || total > inventory().duration_policy.max_total_seconds {
        return Err("假名音拍数量须为正整数，组时长须为正且不超过 30 秒".into());
    }
    let mut units = BTreeMap::<usize, Vec<usize>>::new();
    for (index, phone) in phones.iter().enumerate() { units.entry(phone.unit_index).or_default().push(index); }
    let mut extensions = Vec::new();
    for members in units.values() {
        let first = &phones[members[0]];
        if first.canonical.length == 0 { return Err("假名音素长度须为正整数".into()); }
        if first.canonical.length > 1 {
            let target = if first.native.role == "nucleus" { *members.last().unwrap() } else { members[0] };
            extensions.push((target, first.canonical.length - 1));
        }
    }
    if 1 + extensions.iter().map(|(_, extra)| *extra).sum::<u32>() != mora_count {
        return Err("假名音拍数量与明确的辅音／元音长音标记不一致".into());
    }
    let mora_seconds = total / mora_count as f64;
    let mut short_phones = phones.to_vec();
    for phone in &mut short_phones { phone.canonical.length = 1; }
    let mut durations = allocate_durations_at_minimum(&short_phones, mora_seconds, None, minimum)?;
    for (target, extra) in extensions { durations[target] += extra as f64 * mora_seconds; }
    let last = durations.len() - 1;
    durations[last] = total - durations[..last].iter().sum::<f64>();
    Ok(durations)
}

/// Preserve kana mora rhythm when several mora groups share a sung note.
/// An explicit native-phone override remains authoritative and exact.
pub fn allocate_for_note(reading: &ResolvedPronunciation, total: f64, weights: Option<&[u32]>, mora_weights: Option<&[u32]>) -> Result<Vec<f64>, String> {
    allocate_for_note_at_minimum(reading, total, weights, mora_weights, inventory().duration_policy.min_phone_seconds)
}

/// Preserve the complete standard allocation whenever it fits. Only a note
/// that cannot use it is reallocated at the route's shorter acoustic floor.
pub(crate) fn allocate_for_note_with_minimum(reading: &ResolvedPronunciation, total: f64, weights: Option<&[u32]>, mora_weights: Option<&[u32]>, minimum: f64) -> Result<Vec<f64>, String> {
    let standard = allocate_for_note(reading, total, weights, mora_weights);
    if standard.is_ok() || minimum >= inventory().duration_policy.min_phone_seconds { return standard; }
    allocate_for_note_at_minimum(reading, total, weights, mora_weights, minimum)
}

fn allocate_for_note_at_minimum(reading: &ResolvedPronunciation, total: f64, weights: Option<&[u32]>, mora_weights: Option<&[u32]>, minimum: f64) -> Result<Vec<f64>, String> {
    if weights.is_none() && !reading.parts.is_empty() {
        let mut durations = Vec::with_capacity(reading.phones.len());
        let mut offset = 0;
        for (index, part) in reading.parts.iter().enumerate() {
            let end = offset + part.phone_count;
            let phones = reading.phones.get(offset..end).ok_or("歌词分部的音素边界无效")?;
            let seconds = total * (index + 1) as f64 / reading.parts.len() as f64 - total * index as f64 / reading.parts.len() as f64;
            durations.extend(allocate_groups(phones, &part.language, seconds, part.mora_weights.as_deref(), minimum)
                .map_err(|error| format!("歌词均分为 {} 部分后，第 {} 部分「{}」无法满足每音素至少 {:.0} ms：{error}", reading.parts.len(), index + 1, part.text, minimum * 1000.0))?);
            offset = end;
        }
        if offset != reading.phones.len() { return Err("歌词分部未覆盖全部音素".into()); }
        return Ok(durations);
    }
    if weights.is_some() || mora_weights.is_none() { return allocate_durations_at_minimum(&reading.phones, total, weights, minimum); }
    allocate_groups(&reading.phones, &reading.language, total, mora_weights, minimum)
}

fn allocate_groups(phones: &[ResolvedPhone], language: &str, total: f64, mora_weights: Option<&[u32]>, minimum: f64) -> Result<Vec<f64>, String> {
    let Some(moras) = mora_weights else { return allocate_durations_at_minimum(phones, total, None, minimum); };
    let first_group = phones.first().map_or(0, |p| p.canonical.syllable_index);
    let groups = phones.last().map(|p| p.canonical.syllable_index + 1 - first_group).unwrap_or(0);
    if language != "ja" || moras.len() != groups || moras.contains(&0) || groups == 0
        || !total.is_finite() || total > inventory().duration_policy.max_total_seconds {
        return Err("假名音拍权重须与明确的日语音素组对应".into());
    }
    let sum = moras.iter().map(|&m| m as f64).sum::<f64>();
    let mut result = Vec::new();
    let mut start = 0;
    for (group, &mora_count) in moras.iter().enumerate() {
        let end = start + phones[start..].iter().take_while(|p| p.canonical.syllable_index == group + first_group).count();
        let group_seconds = if group + 1 == groups { total - result.iter().sum::<f64>() } else { total * mora_count as f64 / sum };
        result.extend(allocate_mora_durations_at_minimum(&phones[start..end], group_seconds, mora_count, minimum)?);
        start = end;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tokens(s: &str) -> Vec<String> { s.split_whitespace().map(str::to_string).collect() }
    #[test]
    fn standard_symbols_do_not_change_with_language_or_import_dialect() {
        let standard = canonicalize(&tokens("e r r\\ M"), "en", "xsampa", None).unwrap();
        assert_eq!(standard.iter().map(|u| u.symbol.as_str()).collect::<Vec<_>>(), ["e", "r", "ɹ", "ɯ"]);
        let imported = canonicalize(&tokens("e"), "en", "vocaloid-xsampa", None).unwrap();
        assert_eq!(imported[0].symbol, "ɛ");
        assert!(resolve(&tokens("r"), "en", "ipa", false, None, 1).is_err());
        assert_eq!(resolve(&tokens("ɹ"), "en", "ipa", false, None, 1).unwrap().phones[0].native.native_phone, "r");
    }
    #[test]
    fn japanese_mapping_is_explicit_and_retains_target() {
        assert!(resolve(&tokens("ɯː ɾ a"), "ja", "ipa", false, None, 1).is_err());
        let r = resolve(&tokens("ɯː ɾ a"), "ja", "ipa", true, None, 1).unwrap();
        assert_eq!(r.phones[0].canonical.symbol, "ɯː");
        assert_eq!(r.phones[0].native_key, "zh:u");
        assert_eq!(r.phones[1].native_key, "en:dx");
        assert!(r.phones.iter().all(|p| p.approximation_reason.is_some()));
    }
    #[test]
    fn stress_case_and_length_survive_standard_roundtrip() {
        let ipa = canonicalize(&tokens("ˈh ɛ l . ˌuː"), "en", "ipa", None).unwrap();
        let xs = format_canonical(&ipa, "xsampa").unwrap();
        let back = canonicalize(&tokens(&xs), "en", "xsampa", None).unwrap();
        assert_eq!(format_canonical(&back, "ipa").unwrap(), "ˈh ɛ l . ˌuː");
        assert!(canonicalize(&tokens("@ ."), "en", "xsampa", None).is_err());
        assert!(canonicalize(&tokens("ˈ ˌa"), "ja", "ipa", None).is_err());
    }
    #[test]
    fn real_segments_preserve_note_budget_and_minima() {
        let r = resolve(&tokens("s t ɹ ɛ ŋ θ s"), "en", "ipa", false, None, 1).unwrap();
        let ds = allocate_durations(&r.phones, 0.5, None).unwrap();
        assert!((ds.iter().sum::<f64>() - 0.5).abs() < 1e-12);
        assert!(ds.iter().all(|&d| d >= 0.025 - 1e-12));
        assert!(ds[3] > ds[2]);
        assert!(allocate_durations(&r.phones, 0.1, None).is_err());
        assert!(allocate_durations(&r.phones, 0.5, Some(&[1, 1, 1, 990, 1, 1, 5])).is_err());
        // Original C_Vowel group includes /r/, but it must not sustain a note.
        assert_eq!(r.phones[2].native.context_field, "C_Vowel");
        assert_eq!(r.phones[2].native.role, "consonant");
    }
}
