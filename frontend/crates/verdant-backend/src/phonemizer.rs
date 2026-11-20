//! Persisted, finite lyric frontends and user pronunciation dictionaries.
//!
//! Store [`settings_to_extension`] with the domain's `SetProjectExtension`
//! command under [`EXTENSION_KEY`], so selection and dictionary edits use the
//! ordinary save/undo path. Import files with [`import_dictionary`], which
//! rejects duplicate object keys before Serde can discard them. Unknown fields,
//! including fields on each domain `Pronunciation`, survive import/export.
//! `selected` is the project default; `singer_selections` overrides it for all
//! tracks with the same stable `voice_ref`. Removing an override immediately
//! restores inheritance. An explicit `verdant-track-language/1` track preference
//! selects the matching finite frontend ahead of either default; removing that
//! preference restores the singer/project selection. `segments` assigns a finite
//! frontend to a document-tick range of one track, so one track can mix Japanese
//! kana and Chinese hanzi sections. A covering segment selects the frontend ahead
//! of the track language, singer and project default, and still sits behind
//! explicit note, phrase and lyric readings; a stored builtin reading is judged
//! against that segment's frontend. Removing a segment immediately restores the
//! inherited selection. Dictionary readings remain explicit language choices,
//! ahead of stored readings and the selected frontend.
//!
//! Resolution never edits source notes. The adapter materializes [`ResolvedTrack`]
//! on its own track snapshot, retains its located diagnostics, and gives the
//! same snapshot to vowel-anchor scheduling. A continuation's full reading is
//! inspectable, but its `continuation_of` and `nucleus` instruct the adapter to
//! extend the anchor's last native nucleus without replaying an onset.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use verdant_domain::{NoteId, Project, Pronunciation, Track, TrackId};

mod json;
mod resolution;
pub(crate) mod automatic;
mod english;
mod romaji;

pub const EXTENSION_KEY: &str = "verdant.phonemizer";
pub const SCHEMA: &str = "verdant-phonemizer/1";
pub const DICTIONARY_SCHEMA: &str = "verdant-pronunciation-dictionary/1";
pub const MANDARIN: &str = "mandarin-dictionary-v1";
pub const EXPLICIT_ONLY: &str = "explicit-only-v1";
pub const JAPANESE_KANA: &str = "japanese-kana-v1";
pub const ENGLISH_G2P: &str = "english-g2p-v1";
pub const AUTOMATIC: &str = "automatic-text-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PhonemizerCapability {
    pub id: &'static str,
    pub name: &'static str,
    pub languages: &'static [&'static str],
    pub automatic_input: &'static str,
    pub limitations: &'static str,
}

/// Frontend capabilities are separate from the selected voice's acoustic
/// capabilities. Explicit note/dictionary readings always retain their own
/// declared language and approximation permission.
pub fn capabilities() -> &'static [PhonemizerCapability] {
    &[
        PhonemizerCapability {
            id: AUTOMATIC, name: "自动识别中英日歌词", languages: &["zh", "en", "ja"],
            automatic_input: "Hanzi, direct pinyin, kana and English; whitespace parts share the note equally",
            limitations: "Latin ambiguity uses explicit language and local score context. Japanese is an acoustic approximation; English uses finite spelling rules and an irregular-word lexicon.",
        },
        PhonemizerCapability {
            id: ENGLISH_G2P, name: "英语拼写转读音", languages: &["en"],
            automatic_input: "English spelling rules and a finite irregular-word lexicon",
            limitations: "Rule-based pronunciation and stress can differ from lexical usage; explicit readings override them.",
        },
        PhonemizerCapability {
            id: MANDARIN,
            name: "普通话字典与相邻词组",
            languages: &["zh"],
            automatic_input: "Single-character Hanzi dictionary with adjacent lexical matches and deterministic preferred readings",
            limitations: "Finite lexical rules. The acoustic inventory is checked separately, and explicit readings override automatic choices.",
        },
        PhonemizerCapability {
            id: EXPLICIT_ONLY,
            name: "已填写读音和用户字典",
            languages: &["zh", "en", "ja"],
            automatic_input: "None; retain valid stored readings and exact user dictionary entries",
            limitations: "No text G2P. English words, Japanese kanji and unsupported languages need supplied readings and a supported voice mapping.",
        },
        PhonemizerCapability {
            id: JAPANESE_KANA,
            name: "日语假名（现有跨语言近似）",
            languages: &["ja"],
            automatic_input: "Existing finite kana table with explicit written long vowels",
            limitations: "No kanji G2P, particle reading, inferred pitch accent or vowel devoicing. This explicitly selects the existing approximate Japanese acoustic mapping, not a native Japanese voice. Chinese pinyin is never a fallback.",
        },
    ]
}

/// One track's document-tick range mapped to a finite lyric frontend. The range
/// includes `start_tick` and excludes `end_tick`, so touching ranges never claim
/// the same note and a note is addressed by its own start tick. `selected` must
/// be one of the identifiers returned by [`capabilities`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SegmentEntry {
    pub track: TrackId,
    /// Inclusive start, exclusive end, document ticks.
    pub start_tick: i64,
    pub end_tick: i64,
    /// Finite frontend id from [`capabilities`].
    pub selected: String,
    #[serde(default, flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhonemizerSettings {
    pub schema_version: String,
    pub selected: String,
    /// Project-local overrides keyed by the singer's stable voice reference.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub singer_selections: BTreeMap<String, String>,
    /// Segmented frontends keyed by the user's stable segment id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub segments: BTreeMap<String, SegmentEntry>,
    #[serde(default)]
    pub dictionary: UserDictionary,
    #[serde(default, flatten)]
    pub unknown: Map<String, Value>,
}

impl Default for PhonemizerSettings {
    fn default() -> Self {
        Self { schema_version: SCHEMA.into(), selected: AUTOMATIC.into(), singer_selections: BTreeMap::new(), segments: BTreeMap::new(), dictionary: UserDictionary::default(), unknown: Map::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserDictionary {
    pub schema_version: String,
    #[serde(default)]
    pub phrases: BTreeMap<String, PhraseEntry>,
    #[serde(default)]
    pub lyrics: BTreeMap<String, LyricEntry>,
    #[serde(default, flatten)]
    pub unknown: Map<String, Value>,
}

impl Default for UserDictionary {
    fn default() -> Self {
        Self { schema_version: DICTIONARY_SCHEMA.into(), phrases: BTreeMap::new(), lyrics: BTreeMap::new(), unknown: Map::new() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhraseEntry {
    /// Exact authored note boundaries. The map key must equal `lyrics.concat()`;
    /// each reading addresses the corresponding complete note lyric.
    pub lyrics: Vec<String>,
    pub readings: Vec<Pronunciation>,
    #[serde(default, flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LyricEntry {
    pub reading: Pronunciation,
    #[serde(default, flatten)]
    pub unknown: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ReadingOrigin {
    ExplicitNote,
    UserPhrase { phrase: String },
    UserLyric { lyric: String },
    ExistingBuiltin,
    Builtin,
    Continuation,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedNote {
    pub pronunciation: Pronunciation,
    pub origin: ReadingOrigin,
    /// Stable original syllable anchor, even through several continuation notes.
    pub continuation_of: Option<NoteId>,
    /// Last native nucleus index in this full reading; never an inferred phone.
    pub nucleus: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReadingDiagnostic {
    pub code: &'static str,
    pub note_id: Option<NoteId>,
    pub message: String,
    pub detail: String,
    /// All original dictionary alternatives, without silently selecting one or
    /// removing alternatives the current acoustic inventory cannot render.
    pub candidates: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ResolvedTrack {
    /// Muted and unresolved notes have no entry. The source track is unchanged.
    pub notes: BTreeMap<NoteId, ResolvedNote>,
    pub diagnostics: Vec<ReadingDiagnostic>,
}

/// An absent extension selects automatic text routing; an explicitly stored
/// frontend retains its selection and language constraints.
/// Readings are validated when used, so malformed project entries remain
/// inspectable as located diagnostics instead of erasing source notes.
pub fn settings_from_project(project: &Project) -> Result<PhonemizerSettings, String> {
    let settings = match project.extensions.get(EXTENSION_KEY) {
        None => PhonemizerSettings::default(),
        Some(value) => PhonemizerSettings::deserialize(value).map_err(|error| format!("invalid phonemizer settings: {error}"))?,
    };
    settings.validate_structure()?;
    Ok(settings)
}

pub fn settings_to_extension(settings: &PhonemizerSettings) -> Result<Value, String> {
    settings.validate()?;
    serde_json::to_value(settings).map_err(|error| error.to_string())
}

/// Import is atomic: an invalid or duplicate-key document returns no dictionary.
/// The caller can replace its draft after success and use the normal domain
/// extension command to persist it. No filesystem or online access occurs.
pub fn import_dictionary(input: &str) -> Result<UserDictionary, String> {
    let value = json::parse_unique(input)?;
    let dictionary: UserDictionary = serde_json::from_value(value).map_err(|error| format!("invalid pronunciation dictionary: {error}"))?;
    dictionary.validate()?;
    Ok(dictionary)
}

pub fn export_dictionary(dictionary: &UserDictionary) -> Result<String, String> {
    dictionary.validate()?;
    serde_json::to_string_pretty(dictionary).map_err(|error| error.to_string())
}

/// Stable precedence: explicit Manual/Imported note, longest exact user phrase,
/// exact user lyric, valid stored builtin, then the selected finite frontend.
/// A covering segment supplies that frontend ahead of the track language,
/// singer and project default, and the stored builtin is judged against it.
/// Phrase windows use adjacent touching authored notes. Mutes, continuation
/// markers, rests, overlaps and track boundaries terminate a window.
pub fn resolve_track(track: &Track, settings: &PhonemizerSettings) -> ResolvedTrack {
    resolution::resolve(track, settings)
}

impl PhonemizerSettings {
    /// Used by the real score adapter through `resolve_track`, without changing
    /// source notes or materializing stale per-track copies of a singer setting.
    pub fn selected_for_track(&self, track: &Track) -> Result<&str, String> {
        if let Some(preference) = track.unknown.get("voice_language") {
            if preference.get("schema").and_then(Value::as_str) != Some("verdant-track-language/1") {
                return Err("unsupported track voice_language schema".into());
            }
            if !matches!(preference.get("support").and_then(Value::as_str), Some("native" | "approximate")) {
                return Err("track voice_language support must be native or approximate".into());
            }
            return match preference.get("language").and_then(Value::as_str) {
                Some("zh") => Ok(MANDARIN),
                Some("en") => Ok(ENGLISH_G2P),
                Some("ja") => Ok(JAPANESE_KANA),
                _ => Err("track voice_language language must be zh, en or ja".into()),
            };
        }
        Ok(track.voice_ref.as_ref().and_then(|reference| self.singer_selections.get(reference))
            .map(String::as_str).unwrap_or(&self.selected))
    }

    /// Frontend for a note on `track` starting at `start_tick`, if a segment covers it.
    /// Validation rejects same-track overlaps, so at most one segment can match.
    pub fn segment_selection(&self, track: &TrackId, start_tick: i64) -> Option<&str> {
        self.segments.values().find(|entry| entry.track == *track && start_tick >= entry.start_tick && start_tick < entry.end_tick)
            .map(|entry| entry.selected.as_str())
    }

    fn validate_structure(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA { return Err(format!("unsupported phonemizer schema {:?}", self.schema_version)); }
        validate_selection(&self.selected)?;
        for (reference, selected) in &self.singer_selections {
            if reference.trim().is_empty() { return Err("singer phonemizer reference must be nonempty".into()); }
            validate_selection(selected)?;
        }
        for (key, entry) in &self.segments { validate_segment(key, entry)?; }
        // Half-open ranges: touching boundaries are legal, any shared tick is not.
        let mut intervals: Vec<_> = self.segments.iter().collect();
        intervals.sort_by(|a, b| a.1.track.cmp(&b.1.track).then(a.1.start_tick.cmp(&b.1.start_tick)).then(a.0.cmp(b.0)));
        for pair in intervals.windows(2) {
            let ((key, entry), (later, next)) = (pair[0], pair[1]);
            if entry.track == next.track && next.start_tick < entry.end_tick {
                return Err(format!("同一轨道的分段 tick 区间不能重叠：{key:?} 与 {later:?}"));
            }
        }
        reject_reserved(&self.unknown, &["schema_version", "selected", "singer_selections", "segments", "dictionary"])?;
        self.dictionary.validate_structure()
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_structure()?;
        self.dictionary.validate_readings()
    }
}

fn validate_selection(selected: &str) -> Result<(), String> {
    if capabilities().iter().any(|capability| capability.id == selected) { Ok(()) }
    else { Err(format!("unsupported phonemizer {selected:?}; choose {MANDARIN}, {EXPLICIT_ONLY}, {JAPANESE_KANA}, {ENGLISH_G2P} or {AUTOMATIC}")) }
}

/// A segment map key is the user's stable segment id, independent of the track
/// it addresses; unknown fields survive exactly like every other stored entry.
fn validate_segment(key: &str, entry: &SegmentEntry) -> Result<(), String> {
    if key.trim().is_empty() { return Err("分段名称必须非空".into()); }
    if entry.track.as_str().trim().is_empty() { return Err(format!("分段 {key:?} 的轨道 id 必须非空")); }
    if entry.start_tick < 0 { return Err(format!("分段 {key:?} 的起始 tick 不能为负数")); }
    if entry.end_tick <= entry.start_tick { return Err(format!("分段 {key:?} 的结束 tick 必须大于起始 tick")); }
    validate_selection(&entry.selected)?;
    reject_reserved(&entry.unknown, &["track", "start_tick", "end_tick", "selected"])
}

impl UserDictionary {
    fn validate_structure(&self) -> Result<(), String> {
        if self.schema_version != DICTIONARY_SCHEMA { return Err(format!("unsupported pronunciation dictionary schema {:?}", self.schema_version)); }
        reject_reserved(&self.unknown, &["schema_version", "phrases", "lyrics"])?;
        for (phrase, entry) in &self.phrases {
            if entry.lyrics.len() < 2 || entry.lyrics.len() != entry.readings.len() {
                return Err(format!("phrase {phrase:?} requires at least two exact note lyrics and one reading per lyric"));
            }
            let mut remaining = phrase.as_str();
            for lyric in &entry.lyrics {
                validate_lyric_key(lyric)?;
                remaining = remaining.strip_prefix(lyric).ok_or_else(|| format!("phrase key {phrase:?} must equal the concatenated note lyrics"))?;
            }
            if !remaining.is_empty() { return Err(format!("phrase key {phrase:?} must equal the concatenated note lyrics")); }
            reject_reserved(&entry.unknown, &["lyrics", "readings"])?;
        }
        for (lyric, entry) in &self.lyrics {
            validate_lyric_key(lyric)?;
            reject_reserved(&entry.unknown, &["reading"])?;
        }
        Ok(())
    }

    fn validate_readings(&self) -> Result<(), String> {
        for (phrase, entry) in &self.phrases {
            for (index, reading) in entry.readings.iter().enumerate() {
                validate_dictionary_reading(reading).map_err(|error| format!("phrase {phrase:?}, note {}: {error}", index + 1))?;
            }
        }
        for (lyric, entry) in &self.lyrics {
            validate_dictionary_reading(&entry.reading).map_err(|error| format!("lyric {lyric:?}: {error}"))?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_structure()?;
        self.validate_readings()
    }
}

fn validate_lyric_key(lyric: &str) -> Result<(), String> {
    if lyric.trim().is_empty() || matches!(lyric.trim(), "+" | "-") {
        return Err("dictionary lyrics must be nonempty exact text, not a continuation marker".into());
    }
    Ok(())
}

fn reject_reserved(unknown: &Map<String, Value>, reserved: &[&str]) -> Result<(), String> {
    if let Some(key) = reserved.iter().find(|key| unknown.contains_key(**key)) {
        return Err(format!("extension field {key:?} duplicates a defined field"));
    }
    Ok(())
}

fn validate_dictionary_reading(reading: &Pronunciation) -> Result<(), String> {
    reject_reserved(&reading.unknown, &[
        "input_text", "pinyin", "phonemes", "language", "alphabet", "allow_approximation", "stress",
        "mora_weights", "reading_provenance", "source", "frontend_profile", "phoneme_durations_permille", "diagnostics",
    ])?;
    crate::reading::validate_pronunciation(reading)
}

#[cfg(test)]
mod tests;
