//! Common Hepburn/Kunrei spellings, used only with Japanese language context.
//!
//! The table is finite and the parser never guesses: an unsupported spelling
//! is an error, not an approximation. Automatic routing consults a narrow
//! spelling heuristic; explicit language selection remains authoritative.
const ROWS: &[(&str, &str)] = &[
    ("a i u e o", "あ い う え お"),
    ("ka ki ku ke ko", "か き く け こ"), ("ga gi gu ge go", "が ぎ ぐ げ ご"),
    ("sa shi su se so", "さ し す せ そ"), ("za ji zu ze zo", "ざ じ ず ぜ ぞ"),
    ("ta chi tsu te to", "た ち つ て と"), ("da di du de do", "だ ぢ づ で ど"),
    ("na ni nu ne no", "な に ぬ ね の"), ("ha hi fu he ho", "は ひ ふ へ ほ"),
    ("ba bi bu be bo", "ば び ぶ べ ぼ"), ("pa pi pu pe po", "ぱ ぴ ぷ ぺ ぽ"),
    ("ma mi mu me mo", "ま み む め も"), ("ya yu yo", "や ゆ よ"),
    ("ra ri ru re ro", "ら り る れ ろ"), ("wa wo", "わ を"),
    ("kya kyu kyo", "きゃ きゅ きょ"), ("gya gyu gyo", "ぎゃ ぎゅ ぎょ"),
    ("sha shu sho", "しゃ しゅ しょ"), ("sya syu syo", "しゃ しゅ しょ"),
    ("ja ju jo", "じゃ じゅ じょ"), ("jya jyu jyo", "じゃ じゅ じょ"),
    ("cha chu cho", "ちゃ ちゅ ちょ"), ("tya tyu tyo", "ちゃ ちゅ ちょ"),
    ("nya nyu nyo", "にゃ にゅ にょ"), ("hya hyu hyo", "ひゃ ひゅ ひょ"),
    ("bya byu byo", "びゃ びゅ びょ"), ("pya pyu pyo", "ぴゃ ぴゅ ぴょ"),
    ("mya myu myo", "みゃ みゅ みょ"), ("rya ryu ryo", "りゃ りゅ りょ"),
    ("si ti tu hu zi", "し ち つ ふ じ"),
    ("fa fi fe fo fyu", "ふぁ ふぃ ふぇ ふぉ ふゅ"),
    ("va vi vu ve vo", "ゔぁ ゔぃ ゔ ゔぇ ゔぉ"),
    ("she che je", "しぇ ちぇ じぇ"), ("wi we", "うぃ うぇ"),
];

/// Distinctive romaji spellings used only after whole-token pinyin and known
/// English words have been excluded. This is a finite heuristic, not a language
/// detector: unmarked ambiguous words need a neighbour or an explicit hint.
const JAPANESE_ONLY_SPELLINGS: &[&str] = &[
    "n'", "kk", "tsu", "ku", "ja", "ju", "jo", "jya", "jyu", "jyo",
    "sha", "shu", "sho", "sya", "syu", "syo", "chu", "cho", "che",
    "tya", "tyu", "tyo", "nya", "nyu", "nyo", "hya", "hyu", "hyo",
    "bya", "byu", "byo", "pya", "pyu", "pyo", "mya", "myu", "myo",
    "rya", "ryu", "ryo", "gya", "gyu", "gyo", "kya", "kyu", "kyo",
    "je", "vu",
];

fn parse(text: &str) -> Result<(String, usize, bool), String> {
    let text = text.to_ascii_lowercase();
    if !text.is_ascii() { return Err("罗马字读音只接受拉丁字母".into()); }
    let mut rest = text.as_str();
    let mut out = String::new();
    let mut count = 0;
    let mut distinctive = false;
    while !rest.is_empty() {
        count += 1;
        if rest.starts_with("n'") {
            out.push('ん');
            distinctive = true;
            rest = &rest[2..];
            continue;
        }
        let bytes = rest.as_bytes();
        if bytes[0] == b'n' && (bytes.len() == 1 || !b"aeiouyn".contains(&bytes[1])) {
            out.push('ん');
            rest = &rest[1..];
            continue;
        }
        if bytes.len() >= 2 && bytes[0] == bytes[1] && b"kstpc".contains(&bytes[0]) {
            out.push('っ');
            distinctive |= bytes[0] == b'k';
            rest = &rest[1..];
            continue;
        }
        let mut best: Option<(&str, &str)> = None;
        for &(roman, kana) in ROWS {
            for (spelling, mora) in roman.split_whitespace().zip(kana.split_whitespace()) {
                if rest.starts_with(spelling) && best.is_none_or(|(old, _)| spelling.len() > old.len()) {
                    best = Some((spelling, mora));
                }
            }
        }
        let Some((spelling, mora)) = best else { return Err(format!("「{rest}」不是支持的日语罗马字音拍；请填写假名或明确音素")); };
        out.push_str(mora);
        distinctive |= JAPANESE_ONLY_SPELLINGS.contains(&spelling);
        rest = &rest[spelling.len()..];
    }
    Ok((out, count, distinctive))
}

/// The finite kana spelling of a romanized Japanese word.
pub(super) fn kana(text: &str) -> Result<String, String> {
    parse(text).map(|(kana, _, _)| kana)
}

/// Require several complete morae and a distinctive spelling before automatic
/// routing considers Japanese without kana context.
pub(super) fn is_unmistakable(text: &str) -> bool {
    parse(text).is_ok_and(|(_, count, distinctive)| count > 1 && distinctive)
}
