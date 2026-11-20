//! Explicit diagnostic Mandarin contexts, bounded by recovered question evidence.
//! Spelling rules and legal combinations use the canonical source inventory.
use std::collections::BTreeMap;
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use crate::wire::ScoreContext;

#[derive(Deserialize)]
struct Inventory {
    syllables: BTreeMap<String, [String; 2]>,
    aliases: BTreeMap<String, String>,
    tone_marks: BTreeMap<String, (String, u8)>,
    combining_tones: BTreeMap<String, u8>,
    voiced_initials: Vec<String>,
    unsupported_dictionary_syllables: BTreeMap<String, String>,
}
fn inventory() -> &'static Inventory {
    static VALUE: LazyLock<Inventory> = LazyLock::new(|| serde_json::from_str(include_str!("data/pinyin_inventory.json"))
        .expect("packaged pinyin inventory must be valid"));
    &VALUE
}
pub fn recovered_syllables() -> Vec<String> { inventory().syllables.keys().cloned().collect() }
pub const MIN_SYLLABLE_SECONDS: f64 = 0.15;
pub const MAX_SYLLABLE_SECONDS: f64 = 30.0;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PinyinToken {
    pub label_token: String,
    pub syllable: String,
    pub tone: u8,
    pub initial: String,
    pub final_: String,
}

/// Error describes the limitation of this recovered renderer, not absence
/// of the phone from the original model. No English stand-ins are implied.
#[derive(Clone, Debug, PartialEq)]
pub struct PinyinError(pub String);

pub fn parse_token(raw: &str) -> Result<PinyinToken, PinyinError> {
    let inv = inventory();
    // Finite supported orthography: precomposed marks and decomposed marks
    // share the JSON table. Decomposed u + diaeresis is the same ü spelling.
    let text = raw.trim().to_lowercase().replace("u\u{308}", "ü");
    let digit = text.chars().last().filter(char::is_ascii_digit);
    if digit.is_some_and(|c| c > '5') { return Err(PinyinError("声调数字只能为 0–5".into())); }
    let body = if digit.is_some() { &text[..text.len()-1] } else { text.as_str() };
    let mut letters = String::new();
    let mut marks = Vec::new();
    for ch in body.chars() {
        let key = ch.to_string();
        if let Some((base, tone)) = inv.tone_marks.get(&key) { letters.push_str(base); marks.push(*tone); }
        else if let Some(tone) = inv.combining_tones.get(&key) { marks.push(*tone); }
        else { letters.push(ch); }
    }
    if marks.len() > 1 { return Err(PinyinError("一个拼音音节只能有一个声调符号".into())); }
    let numeric = digit.map(|c| if c == '5' { 0 } else { c as u8 - b'0' });
    if marks.first().is_some_and(|&m| numeric.is_some_and(|n| n != m)) {
        return Err(PinyinError("声调符号与声调数字不一致".into()));
    }
    let tone = numeric.or_else(|| marks.first().copied()).unwrap_or(1);
    let letters = letters.replace("u:", "v").replace('ü', "v");
    if letters.is_empty() || !letters.bytes().all(|c| c.is_ascii_lowercase()) {
        return Err(PinyinError("需要单个拼音音节，可带声调符号或数字 0–5".into()));
    }
    let syllable = inv.aliases.get(&letters).ok_or_else(|| PinyinError(
        inv.unsupported_dictionary_syllables.get(&letters).cloned().unwrap_or_else(|| {
            if letters != "er" && letters.strip_suffix('r').is_some_and(|base| inv.aliases.contains_key(base)) {
                "儿化韵的映射尚未验证；er 是独立音节，不能自动替代儿化后缀".into()
            } else { format!("「{letters}」不在明确支持的普通话声韵组合表中") }
        })
    ))?;
    let [initial, final_] = &inv.syllables[syllable];
    Ok(PinyinToken { label_token: format!("{syllable}{tone}"), syllable: syllable.clone(), tone, initial: initial.clone(), final_: final_.clone() })
}

pub fn in_inventory(syllable: &str, inventory: &[String]) -> bool {
    inventory.iter().any(|s| s == syllable)
}

/// The recovered `_context()` helper: an explicit context object replacing
/// `vowel`/`context_mode`. Field names and values are verbatim.
fn context(initial: &str, final_: &str, tone: u8, is_initial: bool) -> ScoreContext {
    let label = format!(
        "diagnostic_chinese_{}",
        if is_initial { format!("initial_{initial}") } else { format!("final_{final_}") }
    );
    let mut features = BTreeMap::new();
    let name = if is_initial { initial } else { final_ };
    let phone = crate::phonemes::native_phone(&format!("zh:{name}")).expect("pinyin inventory must name a native phone");
    features.insert("C_Phone".into(), phone.phone_id.to_string());
    features.insert("C_Tone".to_string(), tone.to_string());
    let mut answers = BTreeMap::new();
    answers.insert("C_EN".to_string(), false);
    answers.insert("C_SM".to_string(), is_initial);
    answers.insert("C_SilSp".to_string(), false);
    let anchor_questions;
    if is_initial {
        features.insert("C_SM".to_string(), initial.to_string());
        anchor_questions = phone.anchor_questions.clone();
    } else {
        features.insert("C_YM".to_string(), final_.to_string());
        if !initial.is_empty() {
            features.insert("L_SM".to_string(), initial.to_string());
            answers.insert("L_SM".to_string(), true);
        }
        anchor_questions = phone.anchor_questions.clone();
    }
    ScoreContext {
        label,
        features,
        answers,
        anchor_questions,
        assumptions: vec![
            "Explicit user pinyin inventory; current-phone IDs, groups and tone unions follow the recovered original binary profile."
                .to_string(),
            "Orthographic mapping, including ii/iii for apical vowels, is the standard-pinyin frontend policy; the original text dictionary is not recovered.".to_string(),
            "Original neighboring-phone iterators, prosodic hierarchy and coarticulation are unrecovered.".to_string(),
            "Tone is a context input only; note-specified F0 does not reproduce lexical tone contours.".to_string(),
            "Initial and final are rendered independently with separate state sequences; original boundary rules are not claimed."
                .to_string(),
        ],
        ..Default::default()
    }
}

/// `initial_seconds = min(.11, .22 * syllable_seconds)`; 0 for bare finals.
pub fn initial_seconds(initial: &str, syllable_seconds: f64) -> f64 {
    if initial.is_empty() { 0.0 } else { (0.22 * syllable_seconds).min(0.11) }
}

/// Automatic native split, with room for both phones in a short sung note.
/// The caller must admit the total duration before using this split.
pub(crate) fn bounded_initial_seconds(token: &PinyinToken, total: f64, minimum: f64) -> f64 {
    if token.initial.is_empty() {
        return 0.0;
    }
    let preferred = (initial_seconds(&token.initial, total) * 1e6).round() / 1e6;
    let standard = crate::adapter::MIN_SEGMENT_SECONDS;
    if total >= 2.0 * standard - 1e-12 {
        preferred.clamp(standard, (total - standard).max(standard))
    } else {
        preferred.clamp(minimum, (total - minimum).max(minimum))
    }
}

/// One emitted initial segment (recovered fields, verbatim strings).
pub struct InitialSpec {
    pub label: String,
    pub context: ScoreContext,
    pub voiced: bool,
    /// Present on every initial segment (helper emits it; preserved for the
    /// segment reports the renderer embeds).
    pub excitation_note: &'static str,
}

pub const INITIAL_EXCITATION_NOTE: &str =
    "noise for obstruent initial; no recovered burst/closure or mixed-excitation timing";

/// The initial segment spec for one token, or `None` for bare finals.
pub fn initial_spec(token: &PinyinToken, _syllable_seconds: f64) -> Option<InitialSpec> {
    if token.initial.is_empty() {
        return None;
    }
    Some(InitialSpec {
        label: format!("{}: initial {}", token.label_token, token.initial),
        context: context(&token.initial, &token.final_, token.tone, true),
        voiced: inventory().voiced_initials.contains(&token.initial),
        excitation_note: INITIAL_EXCITATION_NOTE,
    })
}

/// The final segment's label and context for one token (always present).
/// Its duration is `syllable_seconds - initial_seconds(...)`, computed by
/// the caller so both halves round consistently.
pub fn final_spec(token: &PinyinToken, _syllable_seconds: f64) -> (String, ScoreContext) {
    (
        format!("{}: final {}", token.label_token, token.final_),
        context(&token.initial, &token.final_, token.tone, false),
    )
}
