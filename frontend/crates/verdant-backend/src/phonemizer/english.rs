//! Rule-based English letter-to-sound conversion for the automatic frontend.
//!
//! No pronunciation dictionary ships with this repository, so this module
//! converts spelling with an explicit, ordered letter-to-sound rule table plus
//! a finite exception lexicon for frequent irregular words. It never claims
//! lexical stress, morphological parsing or accent-specific allophones: the
//! stress pattern is a documented default, and every reading records whether
//! it came from the exception lexicon or from the rules.
//!
//! Output is the existing ARPABET spelling system: nuclei carry a stress
//! digit and `.` separates syllables. The canonical IPA conversion reuses
//! `crate::phonemes`, so the acoustic inventory is unchanged.

use std::sync::LazyLock;

/// One ARPABET phone and its syllable stress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Phone {
    pub symbol: &'static str,
    /// 0 = unstressed, 1 = primary stress.
    pub stress: u8,
}

/// A converted English word.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct EnglishReading {
    /// ARPABET tokens; nuclei carry a stress digit and `.` separates syllables.
    pub tokens: Vec<String>,
    /// "exception-lexicon" or "letter-to-sound-rules".
    pub basis: &'static str,
    /// The exception-lexicon entry that produced this reading, if any.
    pub exception: Option<&'static str>,
}

/// One context element of a rule.
#[derive(Clone, Debug)]
enum Item {
    Literal(char),
    Start,
    End,
    Vowel,
    Consonant,
    Any,
    /// One or more vowels.
    VowelRun,
    /// Zero or more consonants.
    ConsonantRun,
    /// Alternation of item sequences.
    Group(Vec<Vec<Item>>),
    /// One character that is not the given class (`C`, `V` or a literal).
    Not(char),
}

fn is_vowel(ch: char) -> bool {
    "aeiouy".contains(ch)
}

fn is_consonant(ch: char) -> bool {
    ch.is_ascii_alphabetic() && !is_vowel(ch)
}

fn parse_items(chars: &[char], index: &mut usize, grouped: bool) -> Vec<Item> {
    let mut items = Vec::new();
    while *index < chars.len() {
        let ch = chars[*index];
        match ch {
            ')' | '|' if grouped => break,
            '(' => {
                *index += 1;
                let mut alternatives = Vec::new();
                loop {
                    alternatives.push(parse_items(chars, index, true));
                    if *index < chars.len() && chars[*index] == '|' {
                        *index += 1;
                        continue;
                    }
                    break;
                }
                if *index < chars.len() && chars[*index] == ')' {
                    *index += 1;
                }
                items.push(Item::Group(alternatives));
            }
            '!' => {
                *index += 1;
                let inner = chars.get(*index).copied().unwrap_or('C');
                *index += 1;
                items.push(Item::Not(inner));
            }
            '^' => {
                *index += 1;
                items.push(Item::Start);
            }
            '$' => {
                *index += 1;
                items.push(Item::End);
            }
            'V' => {
                *index += 1;
                items.push(Item::Vowel);
            }
            'C' => {
                *index += 1;
                items.push(Item::Consonant);
            }
            'X' => {
                *index += 1;
                items.push(Item::Any);
            }
            '#' => {
                *index += 1;
                items.push(Item::VowelRun);
            }
            '%' => {
                *index += 1;
                items.push(Item::ConsonantRun);
            }
            ' ' => {
                *index += 1;
            }
            _ => {
                *index += 1;
                items.push(Item::Literal(ch));
            }
        }
    }
    items
}

fn parse_pattern(pattern: &str) -> Vec<Item> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0;
    parse_items(&chars, &mut index, false)
}

fn match_items(items: &[Item], text: &[char], pos: usize) -> Option<usize> {
    let Some((first, rest)) = items.split_first() else { return Some(pos) };
    let next = |end| match_items(rest, text, end);
    match first {
        Item::Start => (pos == 0).then(|| next(pos)).flatten(),
        Item::End => (pos == text.len()).then(|| next(pos)).flatten(),
        Item::VowelRun | Item::ConsonantRun => {
            let vowel = matches!(first, Item::VowelRun);
            let mut end = pos;
            while text.get(end).copied().is_some_and(|ch| if vowel { is_vowel(ch) } else { is_consonant(ch) }) { end += 1; }
            let start = pos + usize::from(vowel);
            (start..=end).find_map(next)
        }
        Item::Group(alternatives) => alternatives.iter()
            .find_map(|alternative| match_items(alternative, text, pos).and_then(next)),
        Item::Not(inner) => {
            let Some(ch) = text.get(pos).copied() else { return next(pos); };
            let excluded = match inner { 'C' => is_consonant(ch), 'V' => is_vowel(ch), literal => ch == *literal };
            (!excluded).then(|| next(pos + 1)).flatten()
        }
        _ => {
            let ch = text.get(pos).copied()?;
            let matches = match first {
                Item::Literal(expected) => ch == *expected,
                Item::Vowel => is_vowel(ch), Item::Consonant => is_consonant(ch), Item::Any => true,
                _ => unreachable!(),
            };
            matches.then(|| next(pos + 1)).flatten()
        }
    }
}

/// A left context must consume the tail of the text before the grapheme.
fn matches_left(items: &[Item], text: &[char]) -> bool {
    if items.is_empty() {
        return true;
    }
    (0..=text.len()).any(|start| match_items(items, text, start) == Some(text.len()))
}

/// A right context must consume a prefix of the text after the grapheme.
fn matches_right(items: &[Item], text: &[char]) -> bool {
    if items.is_empty() {
        return true;
    }
    match_items(items, text, 0).is_some()
}

/// One ordered letter-to-sound rule.
struct Rule {
    graphemes: &'static str,
    left: &'static str,
    right: &'static str,
    phones: &'static [&'static str],
}

struct ParsedRule {
    letters: Vec<char>,
    left: Vec<Item>,
    right: Vec<Item>,
    phones: &'static [&'static str],
}

/// Longest matching grapheme wins; equal lengths keep the table order.
fn rules() -> &'static [ParsedRule] {
    static RULES: LazyLock<Vec<ParsedRule>> = LazyLock::new(|| {
        RAW_RULES.iter().map(|rule| ParsedRule {
            letters: rule.graphemes.chars().collect(),
            left: parse_pattern(rule.left),
            right: parse_pattern(rule.right),
            phones: rule.phones,
        }).collect()
    });
    &RULES
}

/// The rule table. Context notation: `^` word start, `$` word end, `V` one
/// vowel, `C` one consonant, `X` one letter, `#` one or more vowels, `%` zero
/// or more consonants, `(a|b)` alternation and `!` negation.
const RAW_RULES: &[Rule] = &[
    // Consonant graphemes.
    Rule { graphemes: "tch", left: "", right: "", phones: &["CH"] },
    Rule { graphemes: "sch", left: "", right: "", phones: &["S", "K"] },
    Rule { graphemes: "ch", left: "", right: "", phones: &["CH"] },
    Rule { graphemes: "sh", left: "", right: "", phones: &["SH"] },
    Rule { graphemes: "th", left: "", right: "", phones: &["TH"] },
    Rule { graphemes: "ph", left: "", right: "", phones: &["F"] },
    Rule { graphemes: "wh", left: "", right: "", phones: &["W"] },
    Rule { graphemes: "gh", left: "V", right: "$", phones: &[] },
    Rule { graphemes: "gh", left: "", right: "", phones: &["G"] },
    Rule { graphemes: "ng", left: "", right: "", phones: &["NG"] },
    Rule { graphemes: "nk", left: "", right: "", phones: &["NG", "K"] },
    Rule { graphemes: "qu", left: "", right: "", phones: &["K", "W"] },
    Rule { graphemes: "ck", left: "", right: "", phones: &["K"] },
    Rule { graphemes: "gn", left: "", right: "", phones: &["N"] },
    Rule { graphemes: "kn", left: "^", right: "", phones: &["N"] },
    Rule { graphemes: "wr", left: "^", right: "", phones: &["R"] },
    Rule { graphemes: "mb", left: "", right: "$", phones: &["M"] },
    Rule { graphemes: "mn", left: "", right: "$", phones: &["M"] },
    Rule { graphemes: "ps", left: "^", right: "", phones: &["S"] },
    Rule { graphemes: "rh", left: "", right: "", phones: &["R"] },
    Rule { graphemes: "gue", left: "", right: "", phones: &["G", "W"] },
    Rule { graphemes: "ss", left: "", right: "", phones: &["S"] },
    Rule { graphemes: "ll", left: "", right: "", phones: &["L"] },
    Rule { graphemes: "tt", left: "", right: "", phones: &["T"] },
    Rule { graphemes: "ff", left: "", right: "", phones: &["F"] },
    Rule { graphemes: "pp", left: "", right: "", phones: &["P"] },
    Rule { graphemes: "bb", left: "", right: "", phones: &["B"] },
    Rule { graphemes: "dd", left: "", right: "", phones: &["D"] },
    Rule { graphemes: "gg", left: "", right: "", phones: &["G"] },
    Rule { graphemes: "mm", left: "", right: "", phones: &["M"] },
    Rule { graphemes: "nn", left: "", right: "", phones: &["N"] },
    Rule { graphemes: "rr", left: "", right: "", phones: &["R"] },
    Rule { graphemes: "cc", left: "", right: "", phones: &["K"] },

    // Vowel-r clusters.
    Rule { graphemes: "ear", left: "", right: "!V", phones: &["IH", "R"] },
    Rule { graphemes: "air", left: "", right: "!V", phones: &["EH", "R"] },
    Rule { graphemes: "eer", left: "", right: "!V", phones: &["IH", "R"] },
    Rule { graphemes: "oor", left: "", right: "!V", phones: &["AO", "R"] },
    Rule { graphemes: "our", left: "", right: "!V", phones: &["AO", "R"] },
    Rule { graphemes: "ure", left: "", right: "$", phones: &["UH", "R"] },
    Rule { graphemes: "ar", left: "", right: "!V", phones: &["AA", "R"] },
    Rule { graphemes: "er", left: "", right: "!V", phones: &["ER"] },
    Rule { graphemes: "ir", left: "", right: "!V", phones: &["ER"] },
    Rule { graphemes: "ur", left: "", right: "!V", phones: &["ER"] },
    Rule { graphemes: "yr", left: "", right: "!V", phones: &["ER"] },
    Rule { graphemes: "or", left: "", right: "!V", phones: &["AO", "R"] },

    // Vowel teams.
    Rule { graphemes: "ough", left: "", right: "", phones: &["AH", "F"] },
    Rule { graphemes: "augh", left: "", right: "", phones: &["AO"] },
    Rule { graphemes: "eigh", left: "", right: "", phones: &["EY"] },
    Rule { graphemes: "igh", left: "", right: "", phones: &["AY"] },
    Rule { graphemes: "tion", left: "", right: "", phones: &["SH", "AH0", "N"] },
    Rule { graphemes: "nsion", left: "", right: "", phones: &["N", "SH", "AH0", "N"] },
    Rule { graphemes: "ssion", left: "", right: "", phones: &["SH", "AH0", "N"] },
    Rule { graphemes: "sion", left: "", right: "", phones: &["ZH", "AH0", "N"] },
    Rule { graphemes: "cious", left: "", right: "", phones: &["SH", "AH0", "S"] },
    Rule { graphemes: "tious", left: "", right: "", phones: &["SH", "AH0", "S"] },
    Rule { graphemes: "cial", left: "", right: "", phones: &["SH", "AH0", "L"] },
    Rule { graphemes: "tial", left: "", right: "", phones: &["SH", "AH0", "L"] },
    Rule { graphemes: "ture", left: "", right: "", phones: &["CH", "ER0"] },
    Rule { graphemes: "ia", left: "", right: "!V", phones: &["Y", "AH"] },
    Rule { graphemes: "io", left: "", right: "!V", phones: &["Y", "OW"] },
    Rule { graphemes: "ua", left: "", right: "!V", phones: &["W", "AH"] },
    Rule { graphemes: "uo", left: "", right: "!V", phones: &["W", "OW"] },
    Rule { graphemes: "ee", left: "", right: "", phones: &["IY"] },
    Rule { graphemes: "ea", left: "", right: "!V", phones: &["IY"] },
    Rule { graphemes: "ie", left: "", right: "!V", phones: &["IY"] },
    Rule { graphemes: "ei", left: "", right: "", phones: &["EY"] },
    Rule { graphemes: "ey", left: "", right: "", phones: &["EY"] },
    Rule { graphemes: "oa", left: "", right: "", phones: &["OW"] },
    Rule { graphemes: "oe", left: "", right: "", phones: &["OW"] },
    Rule { graphemes: "oo", left: "", right: "", phones: &["UW"] },
    Rule { graphemes: "ou", left: "", right: "", phones: &["AW"] },
    Rule { graphemes: "ow", left: "", right: "!V", phones: &["AW"] },
    Rule { graphemes: "ai", left: "", right: "", phones: &["EY"] },
    Rule { graphemes: "ay", left: "", right: "", phones: &["EY"] },
    Rule { graphemes: "au", left: "", right: "", phones: &["AO"] },
    Rule { graphemes: "aw", left: "", right: "", phones: &["AO"] },
    Rule { graphemes: "oi", left: "", right: "", phones: &["OY"] },
    Rule { graphemes: "oy", left: "", right: "", phones: &["OY"] },
    Rule { graphemes: "ue", left: "", right: "", phones: &["UW"] },
    Rule { graphemes: "ui", left: "", right: "", phones: &["UW"] },
    Rule { graphemes: "ew", left: "", right: "", phones: &["UW"] },
    Rule { graphemes: "eu", left: "", right: "", phones: &["UW"] },
    Rule { graphemes: "dge", left: "", right: "", phones: &["JH"] },

    // Silent and single letters.
    Rule { graphemes: "es", left: "(s|z|ch|sh)", right: "$", phones: &["IH0", "Z"] },
    Rule { graphemes: "es", left: "", right: "$", phones: &["S"] },
    Rule { graphemes: "ed", left: "(t|k|p|s|f|ch|sh|th)", right: "$", phones: &["T"] },
    Rule { graphemes: "ed", left: "(d|t)", right: "$", phones: &["IH0", "D"] },
    Rule { graphemes: "ed", left: "", right: "$", phones: &["D"] },
    Rule { graphemes: "le", left: "C", right: "$", phones: &["AH0", "L"] },
    Rule { graphemes: "c", left: "", right: "(e|i|y)", phones: &["S"] },
    Rule { graphemes: "c", left: "", right: "", phones: &["K"] },
    Rule { graphemes: "g", left: "", right: "(e|i|y)", phones: &["JH"] },
    Rule { graphemes: "g", left: "", right: "", phones: &["G"] },
    Rule { graphemes: "s", left: "V", right: "", phones: &["Z"] },
    Rule { graphemes: "s", left: "(b|d|g|j|l|m|n|r|v|w|z)", right: "$", phones: &["Z"] },
    Rule { graphemes: "s", left: "", right: "", phones: &["S"] },
    Rule { graphemes: "x", left: "", right: "", phones: &["K", "S"] },
    Rule { graphemes: "j", left: "", right: "", phones: &["JH"] },
    Rule { graphemes: "v", left: "", right: "", phones: &["V"] },
    Rule { graphemes: "z", left: "", right: "", phones: &["Z"] },
    Rule { graphemes: "b", left: "", right: "", phones: &["B"] },
    Rule { graphemes: "d", left: "", right: "", phones: &["D"] },
    Rule { graphemes: "f", left: "", right: "", phones: &["F"] },
    Rule { graphemes: "h", left: "", right: "", phones: &["HH"] },
    Rule { graphemes: "k", left: "", right: "", phones: &["K"] },
    Rule { graphemes: "l", left: "", right: "", phones: &["L"] },
    Rule { graphemes: "m", left: "", right: "", phones: &["M"] },
    Rule { graphemes: "n", left: "", right: "", phones: &["N"] },
    Rule { graphemes: "p", left: "", right: "", phones: &["P"] },
    Rule { graphemes: "q", left: "", right: "", phones: &["K"] },
    Rule { graphemes: "r", left: "", right: "", phones: &["R"] },
    Rule { graphemes: "t", left: "", right: "", phones: &["T"] },
    Rule { graphemes: "w", left: "", right: "", phones: &["W"] },
    Rule { graphemes: "y", left: "^", right: "V", phones: &["Y"] },
    Rule { graphemes: "y", left: "C", right: "$", phones: &["IY"] },
    Rule { graphemes: "y", left: "", right: "", phones: &["IH"] },
    Rule { graphemes: "a", left: "", right: "", phones: &["AE"] },
    Rule { graphemes: "e", left: "", right: "", phones: &["EH"] },
    Rule { graphemes: "i", left: "", right: "", phones: &["IH"] },
    Rule { graphemes: "o", left: "", right: "$", phones: &["OW"] },
    Rule { graphemes: "o", left: "", right: "", phones: &["AA"] },
    Rule { graphemes: "u", left: "", right: "", phones: &["AH"] },
];

/// Longest matching grapheme wins; equal lengths keep the table order.
/// Frequent irregular words. The spelling is the exact lowercase word; the
/// phones use the existing ARPABET spelling system.
const EXCEPTIONS: &[(&str, &str)] = &[
    ("can't", "K AE1 N T"),
    ("don't", "D OW1 N T"),
    ("won't", "W OW1 N T"),
    ("i'm", "AY1 M"),
    ("it's", "IH1 T S"),
    ("a", "AH0"),
    ("about", "AH0 B AW1 T"),
    ("add", "AE1 D"),
    ("after", "AE1 F T ER0"),
    ("again", "AH0 G EH1 N"),
    ("air", "EH1 R"),
    ("all", "AO1 L"),
    ("also", "AO1 L S OW0"),
    ("america", "AH0 M EH1 R IH0 K AH0"),
    ("an", "AE0 N"),
    ("and", "AE0 N D"),
    ("animal", "AE1 N AH0 M AH0 L"),
    ("another", "AH0 N AH1 DH ER0"),
    ("answer", "AE1 N S ER0"),
    ("any", "EH1 N IY0"),
    ("are", "AA1 R"),
    ("around", "ER0 AW1 N D"),
    ("as", "AE0 Z"),
    ("ask", "AE1 S K"),
    ("at", "AE0 T"),
    ("away", "AH0 W EY1"),
    ("back", "B AE1 K"),
    ("be", "B IY1"),
    ("bear", "B EH1 R"),
    ("beautiful", "B Y UW1 T AH0 F AH0 L"),
    ("because", "B IH0 K AO1 Z"),
    ("been", "B IH1 N"),
    ("beer", "B IH1 R"),
    ("before", "B IH0 F AO1 R"),
    ("being", "B IY1 IH0 NG"),
    ("below", "B IH0 L OW1"),
    ("between", "B IH0 T W IY1 N"),
    ("big", "B IH1 G"),
    ("black", "B L AE1 K"),
    ("blue", "B L UW1"),
    ("book", "B UH1 K"),
    ("boy", "B OY1"),
    ("bread", "B R EH1 D"),
    ("business", "B IH1 Z N AH0 S"),
    ("but", "B AH1 T"),
    ("by", "B AY1"),
    ("call", "K AO1 L"),
    ("came", "K EY1 M"),
    ("can", "K AE1 N"),
    ("caught", "K AO1 T"),
    ("change", "CH EY1 N JH"),
    ("cheer", "CH IH1 R"),
    ("choice", "CH OY1 S"),
    ("city", "S IH1 T IY0"),
    ("clear", "K L IH1 R"),
    ("color", "K AH1 L ER0"),
    ("come", "K AH1 M"),
    ("could", "K UH1 D"),
    ("country", "K AH1 N T R IY0"),
    ("cross", "K R AO1 S"),
    ("cure", "K Y UH1 R"),
    ("dark", "D AA1 R K"),
    ("daughter", "D AO1 T ER0"),
    ("day", "D EY1"),
    ("dead", "D EH1 D"),
    ("deer", "D IH1 R"),
    ("destroy", "D IH0 S T R OY1"),
    ("did", "D IH1 D"),
    ("different", "D IH1 F R AH0 N T"),
    ("do", "D UW1"),
    ("does", "D AH1 Z"),
    ("doing", "D UW1 IH0 NG"),
    ("door", "D AO1 R"),
    ("down", "D AW1 N"),
    ("dream", "D R IY1 M"),
    ("each", "IY1 CH"),
    ("earth", "ER1 TH"),
    ("eat", "IY1 T"),
    ("enjoy", "IH0 N JH OY1"),
    ("even", "IY1 V AH0 N"),
    ("every", "EH1 V R IY0"),
    ("eye", "AY1"),
    ("family", "F AE1 M AH0 L IY0"),
    ("father", "F AA1 DH ER0"),
    ("find", "F AY1 N"),
    ("finger", "F IH1 NG G ER0"),
    ("fire", "F AY1 R"),
    ("first", "F ER1 S T"),
    ("flower", "F L AW1 ER0"),
    ("follow", "F AA1 L OW0"),
    ("food", "F UW1 D"),
    ("for", "F AO1 R"),
    ("form", "F AO1 R M"),
    ("found", "F AW1 N D"),
    ("four", "F AO1 R"),
    ("friend", "F R EH1 N D"),
    ("from", "F R AH1 M"),
    ("gear", "G IH1 R"),
    ("gee", "JH IY1"),
    ("geese", "G IY1 S"),
    ("gel", "JH EH1 L"),
    ("gender", "JH EH1 N D ER0"),
    ("gene", "JH IY1 N"),
    ("general", "JH EH1 N R AH0 L"),
    ("generous", "JH EH1 N R AH0 S"),
    ("genius", "JH IY1 N IY0 AH0 S"),
    ("geography", "JH IY0 AA1 G R AH0 F IY0"),
    ("geometry", "JH IY0 AA1 M AH0 T R IY0"),
    ("germ", "G ER1 M"),
    ("gesture", "JH EH1 S CH ER0"),
    ("get", "G EH1 T"),
    ("giant", "JH AY1 AH0 N T"),
    ("gift", "G IH1 F T"),
    ("girl", "G ER1 L"),
    ("give", "G IH1 V"),
    ("go", "G OW1"),
    ("going", "G OW1 IH0 NG"),
    ("good", "G UH1 D"),
    ("great", "G R EY1 T"),
    ("green", "G R IY1 N"),
    ("gym", "JH IH1 M"),
    ("had", "HH AE1 D"),
    ("hair", "HH EH1 R"),
    ("hand", "HH AE1 N D"),
    ("happy", "HH AE1 P IY0"),
    ("has", "HH AE1 Z"),
    ("have", "HH AE1 V"),
    ("he", "HH IY1"),
    ("head", "HH EH1 D"),
    ("hear", "HH IH1 R"),
    ("heard", "HH ER1 D"),
    ("heart", "HH AA1 R T"),
    ("hello", "HH AH0 . L OW1"),
    ("help", "HH EH1 L P"),
    ("her", "HH ER1"),
    ("here", "HH IH1 R"),
    ("high", "HH AY1"),
    ("him", "HH IH1 M"),
    ("his", "HH IH1 Z"),
    ("home", "HH OW1 M"),
    ("hour", "AW1 ER0"),
    ("house", "HH AW1 S"),
    ("how", "HH AW1"),
    ("hundred", "HH AH1 N D R AH0 D"),
    ("hunger", "HH AH1 NG G ER0"),
    ("i", "AY1"),
    ("idea", "AY0 D IY1 AH0"),
    ("if", "IH1 F"),
    ("in", "IH0 N"),
    ("into", "IH1 N T UW0"),
    ("is", "IH1 Z"),
    ("it", "IH1 T"),
    ("its", "IH1 T S"),
    ("just", "JH AH1 S T"),
    ("keep", "K IY1 P"),
    ("kind", "K AY1 N D"),
    ("know", "N OW1"),
    ("land", "L AE1 N D"),
    ("large", "L AA1 R JH"),
    ("last", "L AE1 S T"),
    ("laugh", "L AE1 F"),
    ("leap", "L IY1 P"),
    ("learn", "L ER1 N"),
    ("letter", "L EH1 T ER0"),
    ("light", "L AY1 T"),
    ("like", "L AY1 K"),
    ("line", "L AY1 N"),
    ("little", "L IH1 T AH0 L"),
    ("live", "L IH1 V"),
    ("long", "L AO1 NG"),
    ("look", "L UH1 K"),
    ("love", "L AH1 V"),
    ("machine", "M AH0 SH IY1 N"),
    ("made", "M EY1 D"),
    ("make", "M EY1 K"),
    ("man", "M AE1 N"),
    ("many", "M EH1 N IY0"),
    ("may", "M EY1"),
    ("me", "M IY1"),
    ("mean", "M IY1 N"),
    ("men", "M EH1 N"),
    ("money", "M AH1 N IY0"),
    ("moon", "M UW1 N"),
    ("more", "M AO1 R"),
    ("morning", "M AO1 R N IH0 NG"),
    ("most", "M OW1 S T"),
    ("mother", "M AH1 DH ER0"),
    ("move", "M UW1 V"),
    ("much", "M AH1 CH"),
    ("music", "M Y UW1 Z IH0 K"),
    ("must", "M AH1 S T"),
    ("my", "M AY1"),
    ("name", "N EY1 M"),
    ("near", "N IH1 R"),
    ("need", "N IY1 D"),
    ("never", "N EH1 V ER0"),
    ("new", "N UW1"),
    ("night", "N AY1 T"),
    ("no", "N OW1"),
    ("noise", "N OY1 Z"),
    ("not", "N AA1 T"),
    ("now", "N AW1"),
    ("ocean", "OW1 SH AH0 N"),
    ("of", "AH0 V"),
    ("off", "AO1 F"),
    ("oil", "OY1 L"),
    ("old", "OW1 L D"),
    ("on", "AA1 N"),
    ("one", "W AH1 N"),
    ("only", "OW1 N L IY0"),
    ("open", "OW1 P AH0 N"),
    ("or", "AO1 R"),
    ("other", "AH1 DH ER0"),
    ("our", "AW1 R"),
    ("out", "AW1 T"),
    ("over", "OW1 V ER0"),
    ("own", "OW1 N"),
    ("page", "P EY1 JH"),
    ("part", "P AA1 R T"),
    ("pear", "P EH1 R"),
    ("people", "P IY1 P AH0 L"),
    ("picture", "P IH1 K CH ER0"),
    ("place", "P L EY1 S"),
    ("plant", "P L AE1 N T"),
    ("play", "P L EY1"),
    ("plenty", "P L EH1 N T IY0"),
    ("point", "P OY1 N T"),
    ("put", "P UH1 T"),
    ("quiet", "K W AY1 AH0 T"),
    ("read", "R IY1 D"),
    ("ready", "R EH1 D IY0"),
    ("red", "R EH1 D"),
    ("right", "R AY1 T"),
    ("river", "R IH1 V ER0"),
    ("royal", "R OY1 AH0 L"),
    ("said", "S EH1 D"),
    ("same", "S EY1 M"),
    ("say", "S EY1"),
    ("saying", "S EY1 IH0 NG"),
    ("says", "S EH1 Z"),
    ("school", "S K UW1 L"),
    ("science", "S AY1 AH0 N S"),
    ("see", "S IY1"),
    ("sentence", "S EH1 N T AH0 N S"),
    ("set", "S EH1 T"),
    ("she", "SH IY1"),
    ("should", "SH UH1 D"),
    ("show", "SH OW1"),
    ("since", "S IH1 N S"),
    ("sky", "S K AY1"),
    ("slept", "S L EH1 P T"),
    ("small", "S M AO1 L"),
    ("so", "S OW1"),
    ("some", "S AH1 M"),
    ("son", "S AH1 N"),
    ("song", "S AO1 NG"),
    ("sound", "S AW1 N D"),
    ("spell", "S P EH1 L"),
    ("still", "S T IH1 L"),
    ("story", "S T AO1 R IY0"),
    ("study", "S T AH1 D IY0"),
    ("such", "S AH1 CH"),
    ("summer", "S AH1 M ER0"),
    ("sure", "SH UH1 R"),
    ("table", "T EY1 B AH0 L"),
    ("take", "T EY1 K"),
    ("taught", "T AO1 T"),
    ("tell", "T EH1 L"),
    ("than", "DH AE1 N"),
    ("that", "DH AE1 T"),
    ("the", "DH AH0"),
    ("their", "DH EH1 R"),
    ("them", "DH EH1 M"),
    ("then", "DH EH1 N"),
    ("there", "DH EH1 R"),
    ("these", "DH IY1 Z"),
    ("they", "DH EY1"),
    ("thing", "TH IH1 NG"),
    ("think", "TH IH1 NG K"),
    ("this", "DH IH1 S"),
    ("thought", "TH AO1 T"),
    ("three", "TH R IY1"),
    ("through", "TH R UW1"),
    ("time", "T AY1 M"),
    ("to", "T UW1"),
    ("too", "T UW1"),
    ("tree", "T R IY1"),
    ("true", "T R UW1"),
    ("try", "T R AY1"),
    ("turn", "T ER1 N"),
    ("two", "T UW1"),
    ("up", "AH1 P"),
    ("us", "AH1 S"),
    ("use", "Y UW1 Z"),
    ("very", "V EH1 R IY0"),
    ("voice", "V OY1 S"),
    ("walk", "W AO1 K"),
    ("want", "W AA1 N T"),
    ("warm", "W AO1 R M"),
    ("was", "W AH0 Z"),
    ("water", "W AO1 T ER0"),
    ("way", "W EY1"),
    ("we", "W IY1"),
    ("well", "W EH1 L"),
    ("went", "W EH1 N T"),
    ("were", "W ER0"),
    ("what", "W AH1 T"),
    ("when", "W EH1 N"),
    ("where", "W EH1 R"),
    ("white", "W AY1 T"),
    ("who", "HH UW1"),
    ("why", "W AY1"),
    ("will", "W IH1 L"),
    ("winter", "W IH1 N T ER0"),
    ("with", "W IH1 DH"),
    ("woman", "W UH1 M AH0 N"),
    ("women", "W IH1 M IH0 N"),
    ("word", "W ER1 D"),
    ("work", "W ER1 K"),
    ("world", "W ER1 L D"),
    ("would", "W UH1 D"),
    ("write", "R AY1 T"),
    ("year", "Y IH1 R"),
    ("you", "Y UW1"),
    ("young", "Y AH1 NG"),
    ("your", "Y AO1 R"),
];

/// Prefixes that carry no stress in the default stress pattern.
const UNSTRESSED_PREFIXES: &[&str] = &[
    "a", "be", "de", "re", "in", "im", "ex", "en", "em", "con", "com", "pre", "pro", "sub",
    "dis", "mis", "ob", "per", "sur", "ad", "at", "as", "ac", "af", "ag", "al", "am", "an",
    "ap", "ar", "au", "aw", "un", "up", "de", "be", "for", "fore", "with", "out", "over",
];

/// Suffixes that carry the primary stress in the default stress pattern.
const STRESSED_SUFFIXES: &[&str] = &[
    "tion", "sion", "cial", "tial", "ious", "eous", "ian", "ial", "ity", "ic", "ical",
    "ate", "ize", "ise", "ify", "ogy", "omy", "ege", "ade", "ute", "ive",
];

fn exceptions() -> &'static std::collections::BTreeMap<&'static str, &'static str> {
    static MAP: LazyLock<std::collections::BTreeMap<&'static str, &'static str>> =
        LazyLock::new(|| EXCEPTIONS.iter().copied().collect());
    &MAP
}

/// Convert one lowercase alphabetic word into ARPABET phones.
fn rule_phones(letters: &[char]) -> Vec<Phone> {
    let long = magic_e(letters);
    let silent = magic_e_silent(letters);
    let mut phones = Vec::new();
    let mut position = 0;
    while position < letters.len() {
        if silent[position] {
            position += 1;
            continue;
        }
        let left = &letters[..position];
        let mut best: Option<(usize, &'static [&'static str])> = None;
        for rule in rules() {
            let length = rule.letters.len();
            if position + length > letters.len() { continue; }
            if !letters[position..position + length].eq(&rule.letters[..]) { continue; }
            if !matches_left(&rule.left, left) { continue; }
            if !matches_right(&rule.right, &letters[position + length..]) { continue; }
            if best.is_none_or(|(current, _)| length > current) {
                best = Some((length, rule.phones));
            }
        }
        let Some((length, symbols)) = best else {
            position += 1;
            continue;
        };
        for symbol in symbols {
            let symbol = if long[position] && length == 1 {
                match letters[position] { 'a' => "EY", 'e' => "IY", 'i' => "AY", 'o' => "OW", 'u' => "UW", _ => *symbol }
            } else { *symbol };
            phones.push(Phone { symbol, stress: 0 });
        }
        position += length;
    }
    phones
}

/// The silent `e` of a magic-e spelling marks the preceding vowel as long.
fn magic_e(letters: &[char]) -> Vec<bool> {
    let mut long = vec![false; letters.len()];
    for index in 0..letters.len() {
        if !is_vowel(letters[index]) { continue; }
        if index + 1 < letters.len() && letters[index + 1] == 'e' && index + 2 == letters.len() {
            long[index] = true;
        }
        if index + 2 < letters.len() && index + 3 == letters.len() && !is_vowel(letters[index + 1])
            && letters[index + 1] != 'e' && letters[index + 2] == 'e' {
            long[index] = true;
        }
    }
    long
}

fn magic_e_silent(letters: &[char]) -> Vec<bool> {
    let mut silent = vec![false; letters.len()];
    let long = magic_e(letters);
    for index in 0..letters.len() {
        if !long[index] { continue; }
        if index + 1 < letters.len() && letters[index + 1] == 'e' && index + 2 == letters.len() {
            silent[index + 1] = true;
        }
        if index + 2 < letters.len() && index + 3 == letters.len() && letters[index + 2] == 'e' {
            silent[index + 2] = true;
        }
    }
    silent
}

/// Group phones into syllables and mark the nuclei.
fn syllabify(phones: &[Phone]) -> Vec<Vec<Phone>> {
    let nuclei: Vec<usize> = phones.iter().enumerate()
        .filter(|(_, phone)| is_nucleus(phone.symbol))
        .map(|(index, _)| index)
        .collect();
    if nuclei.is_empty() {
        return phones.iter().map(|phone| vec![*phone]).collect();
    }
    // The syllable boundary sits before the last consonant of an inter-nuclear
    // cluster, so a single consonant closes the previous syllable.
    let mut starts = vec![0usize];
    for window in nuclei.windows(2) {
        let (first, second) = (window[0], window[1]);
        let cluster = second - first - 1;
        let boundary = if cluster >= 2 { second - 1 } else { second };
        starts.push(boundary);
    }
    let mut syllables = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(phones.len());
        if end > *start {
            syllables.push(phones[*start..end].to_vec());
        }
    }
    syllables
}

fn is_nucleus(symbol: &str) -> bool {
    matches!(symbol.trim_end_matches(['0', '1', '2']), "AA" | "AE" | "AH" | "AO" | "AW" | "AY" | "EH" | "ER" | "EY" | "IH" | "IY"
        | "OW" | "OY" | "UH" | "UW")
}

/// The documented default stress pattern: the first syllable, unless an
/// unstressed prefix or a stressed suffix moves it.
fn stress_index(syllables: &[Vec<Phone>], word: &str) -> usize {
    if syllables.len() <= 1 {
        return 0;
    }
    for prefix in UNSTRESSED_PREFIXES {
        if word.len() > prefix.len() && word.starts_with(prefix) {
            return 1.min(syllables.len() - 1);
        }
    }
    for suffix in STRESSED_SUFFIXES {
        if word.len() > suffix.len() && word.ends_with(suffix) {
            return syllables.len().saturating_sub(2);
        }
    }
    0
}

/// Convert one word into ARPABET tokens with stress digits and syllable marks.
fn convert_word(word: &str) -> Result<EnglishReading, String> {
    let lower = word.to_lowercase();
    if let Some((&key, &entry)) = exceptions().get_key_value(lower.as_str()) {
        let mut tokens = Vec::new();
        for phone in entry.split_whitespace() {
            tokens.push(phone.to_string());
        }
        return Ok(EnglishReading { tokens, basis: "exception-lexicon", exception: Some(key) });
    }
    // A contraction's apostrophe is internal punctuation, not a new syllable.
    let letters: Vec<char> = lower.chars().filter(|ch| *ch != '\'').collect();
    if letters.is_empty() {
        return Err(format!("英文读音「{word}」没有可转换的字母"));
    }
    let phones = rule_phones(&letters);
    if phones.is_empty() {
        return Err(format!("英文读音「{word}」没有可转换的音素"));
    }
    let mut syllables = syllabify(&phones);
    let stressed = stress_index(&syllables, &lower);
    // Unstressed syllables use the schwa; the r-coloured vowel keeps its own
    // model, which the inventory documents as the available rhotic path.
    for (index, syllable) in syllables.iter_mut().enumerate() {
        for phone in syllable.iter_mut() {
            if !is_nucleus(phone.symbol) || phone.symbol.ends_with('0') { continue; }
            if index == stressed {
                phone.stress = 1;
            } else {
                phone.symbol = if phone.symbol == "ER" { "ER" } else { "AH" };
                phone.stress = 0;
            }
        }
    }
    let mut tokens = Vec::new();
    for (index, syllable) in syllables.iter().enumerate() {
        if index > 0 {
            tokens.push(".".to_string());
        }
        for phone in syllable {
            let mut token = phone.symbol.to_string();
            if is_nucleus(phone.symbol) && !token.ends_with(['0', '1', '2']) {
                token.push_str(if phone.stress == 1 { "1" } else { "0" });
            }
            tokens.push(token);
        }
    }
    Ok(EnglishReading { tokens, basis: "letter-to-sound-rules", exception: None })
}

/// Convert English text into ARPABET tokens. Words are separated by `.`.
///
/// The text must be Latin script; other scripts are rejected so a Japanese or
/// Chinese lyric is never read with English rules.
pub(super) fn reading(text: &str) -> Result<EnglishReading, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("英文读音为空".into());
    }
    if !trimmed.chars().all(|ch| ch.is_ascii_alphabetic() || ch.is_whitespace() || ch == '\'' || ch == '-') {
        return Err(format!("英文读音「{trimmed}」含有非拉丁字母"));
    }
    let mut tokens = Vec::new();
    let mut basis = "letter-to-sound-rules";
    let mut exception = None;
    for (index, word) in trimmed.split(|ch: char| ch.is_whitespace() || ch == '-').filter(|word| !word.is_empty()).enumerate() {
        let word = word.trim_matches(|ch| ch == '\'' || ch == '-');
        if word.is_empty() { continue; }
        let converted = convert_word(word)?;
        if index > 0 {
            tokens.push(".".to_string());
        }
        if converted.basis == "exception-lexicon" {
            basis = "exception-lexicon";
            exception = converted.exception;
        }
        tokens.extend(converted.tokens);
    }
    if tokens.is_empty() {
        return Err(format!("英文读音「{trimmed}」没有可转换的音素"));
    }
    Ok(EnglishReading { tokens, basis, exception })
}

/// Common English words that are also valid Mandarin syllables. They keep the
/// English reading unless the score context explicitly selects Mandarin.
const COMMON_ENGLISH_WORDS: &[&str] = &["sun", "hang", "sang", "sing", "ran", "tan", "pan", "ban", "fan", "gun", "fun", "pin", "bin", "win", "sin", "can", "go", "no", "so", "he", "me", "you", "song"];

// These English words also satisfy the conservative multi-mora romaji test.
const ENGLISH_ROMAJI_LOOKALIKES: &[&str] = &["anyone", "choke", "chore", "choose", "chose", "joke", "june", "shake", "shame", "shape", "share", "shave", "shoe", "shone", "shoo"];

/// True when the spelling is a common English word, so an automatic frontend
/// without a language hint reads it as English rather than as a Mandarin
/// syllable with the same spelling.
pub(super) fn is_common_english_word(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    COMMON_ENGLISH_WORDS.contains(&lower.as_str()) || ENGLISH_ROMAJI_LOOKALIKES.contains(&lower.as_str())
        || exceptions().contains_key(lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(word: &str) -> Vec<String> {
        reading(word).unwrap().tokens
    }

    #[test]
    fn frequent_words_use_the_exception_lexicon() {
        assert_eq!(tokens("the"), ["DH", "AH0"]);
        assert_eq!(tokens("of"), ["AH0", "V"]);
        assert_eq!(tokens("you"), ["Y", "UW1"]);
        assert_eq!(tokens("one"), ["W", "AH1", "N"]);
        assert_eq!(tokens("woman"), ["W", "UH1", "M", "AH0", "N"]);
    }

    #[test]
    fn regular_words_use_the_rule_table() {
        assert_eq!(tokens("cat"), ["K", "AE1", "T"]);
        assert_eq!(tokens("make"), ["M", "EY1", "K"]);
        assert_eq!(tokens("ship"), ["SH", "IH1", "P"]);
        assert_eq!(tokens("thought"), ["TH", "AO1", "T"]);
        assert_eq!(tokens("school"), ["S", "K", "UW1", "L"]);
    }

    #[test]
    fn every_token_is_a_known_arpabet_spelling() {
        for word in ["cat", "make", "school", "thought", "bright", "climb", "box", "city", "iraq"] {
            for token in tokens(word) {
                if token == "." { continue; }
                assert!(crate::phonemes::AlphabetDeclaration::new("arpabet").unwrap()
                    .symbols("en").unwrap().contains_key(&token), "{word}: {token}");
            }
        }
    }

    #[test]
    fn non_latin_text_is_rejected_instead_of_read_as_english() {
        assert!(reading("海").is_err());
        assert!(reading("か").is_err());
        assert!(reading("").is_err());
    }

    #[test]
    fn common_english_words_are_recognized_for_language_tie_breaks() {
        assert!(is_common_english_word("song"));
        assert!(is_common_english_word("you"));
        assert!(!is_common_english_word("ren"));
        assert!(!is_common_english_word("zhe"));
        assert!(!is_common_english_word("mei"));
    }
}
