//! Finite project-authored lexical choices, checked against the MIT pinyin
//! dictionary candidates. This is deliberately not general Mandarin NLP.

pub(in crate::phonemizer) const PHRASES: &[(&str, &[&str])] = &[
    ("银行", &["yin2", "hang2"]), ("行业", &["hang2", "ye4"]),
    ("行走", &["xing2", "zou3"]), ("行人", &["xing2", "ren2"]),
    ("重庆", &["chong2", "qing4"]), ("重复", &["chong2", "fu4"]),
    ("重新", &["chong2", "xin1"]), ("重要", &["zhong4", "yao4"]),
    ("重量", &["zhong4", "liang4"]), ("长大", &["zhang3", "da4"]),
    ("成长", &["cheng2", "zhang3"]), ("长城", &["chang2", "cheng2"]),
    ("长江", &["chang2", "jiang1"]), ("校长", &["xiao4", "zhang3"]),
    ("音乐", &["yin1", "yue4"]), ("乐曲", &["yue4", "qu3"]),
    ("快乐", &["kuai4", "le4"]), ("乐观", &["le4", "guan1"]),
    ("爱好", &["ai4", "hao4"]), ("好奇", &["hao4", "qi2"]),
    ("好人", &["hao3", "ren2"]), ("好看", &["hao3", "kan4"]),
    ("中奖", &["zhong4", "jiang3"]), ("中国", &["zhong1", "guo2"]),
    ("中间", &["zhong1", "jian1"]), ("目的", &["mu4", "di4"]),
    ("觉得", &["jue2", "de5"]), ("得到", &["de2", "dao4"]),
    ("干净", &["gan1", "jing4"]), ("干活", &["gan4", "huo2"]),
    ("少年", &["shao4", "nian2"]), ("减少", &["jian3", "shao3"]),
    ("归还", &["gui1", "huan2"]), ("还是", &["hai2", "shi4"]),
    ("首都", &["shou3", "du1"]), ("都是", &["dou1", "shi4"]),
    ("因为", &["yin1", "wei4"]), ("认为", &["ren4", "wei2"]),
    ("放假", &["fang4", "jia4"]), ("假如", &["jia3", "ru2"]),
    ("教书", &["jiao1", "shu1"]), ("教育", &["jiao4", "yu4"]),
    ("有空", &["you3", "kong4"]), ("天空", &["tian1", "kong1"]),
    ("歌曲", &["ge1", "qu3"]), ("弯曲", &["wan1", "qu1"]),
    ("勉强", &["mian3", "qiang3"]), ("强大", &["qiang2", "da4"]),
    ("西藏", &["xi1", "zang4"]), ("收藏", &["shou1", "cang2"]),
    ("传记", &["zhuan4", "ji4"]), ("传说", &["chuan2", "shuo1"]),
    ("子弹", &["zi3", "dan4"]), ("弹琴", &["tan2", "qin2"]),
    ("头发", &["tou2", "fa4"]), ("出发", &["chu1", "fa1"]),
    ("了解", &["liao3", "jie3"]), ("看着", &["kan4", "zhe5"]),
    ("睡觉", &["shui4", "jiao4"]), ("感觉", &["gan3", "jue2"]),
    ("调查", &["diao4", "cha2"]), ("调整", &["tiao2", "zheng3"]),
    ("尽管", &["jin3", "guan3"]), ("尽力", &["jin4", "li4"]),
    ("兴趣", &["xing4", "qu4"]), ("兴起", &["xing1", "qi3"]),
    ("答应", &["da1", "ying4"]), ("应该", &["ying1", "gai1"]),
];

pub(in crate::phonemizer) const PREFERRED: &[(&str, &str)] = &[
    ("行", "xing2"), ("重", "zhong4"), ("长", "chang2"), ("乐", "le4"),
    ("好", "hao3"), ("中", "zhong1"), ("地", "di4"), ("得", "de2"),
    ("的", "de5"), ("和", "he2"), ("干", "gan1"), ("少", "shao3"),
    ("还", "hai2"), ("都", "dou1"), ("为", "wei4"), ("分", "fen1"),
    ("假", "jia3"), ("教", "jiao4"), ("空", "kong1"), ("曲", "qu3"),
    ("强", "qiang2"), ("差", "cha1"), ("藏", "cang2"), ("传", "chuan2"),
    ("弹", "tan2"), ("发", "fa1"), ("血", "xue4"), ("朝", "chao2"),
    ("了", "le5"), ("着", "zhe5"), ("数", "shu4"), ("只", "zhi3"),
    ("种", "zhong3"), ("会", "hui4"), ("兴", "xing1"), ("应", "ying1"),
    ("当", "dang1"), ("更", "geng4"), ("尽", "jin4"), ("觉", "jue2"),
    ("漂", "piao1"), ("落", "luo4"), ("难", "nan2"), ("调", "tiao2"),
];

/// Longest matching word wins; equal lengths use stable lexical table order.
/// Windows are exact one-Hanzi notes, never substrings of unsupported lyrics.
pub(super) fn choose(notes: &[verdant_domain::Note], index: usize, start: usize,
    end: usize, candidates: &[String]) -> Option<(&'static str, &'static str, &'static str)> {
    let mut best = None;
    let mut best_len = 0;
    for &(word, readings) in PHRASES {
        let len = readings.len();
        for offset in 0..len {
            let Some(first) = index.checked_sub(offset) else { continue; };
            if first < start || first + len > end { continue; }
            let matches = notes[first..first + len].iter().zip(word.chars())
                .all(|(note, ch)| note.lyric.starts_with(ch) && note.lyric.chars().count() == 1);
            if !matches { continue; }
            let reading = readings[offset];
            if len > best_len && candidates.iter().any(|candidate| candidate == reading) {
                best = Some((reading, "phrase", word));
                best_len = len;
            }
        }
    }
    best.or_else(|| PREFERRED.iter().find(|(character, reading)|
        *character == notes[index].lyric && candidates.iter().any(|candidate| candidate.as_str() == *reading))
        .map(|&(character, reading)| (reading, "preferred", character)))
}
