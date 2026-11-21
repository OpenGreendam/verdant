//! ScoreAdapter: project snapshot → `native_score` phrases, with preflight
//! diagnostics located on notes *before* anything is sent (SPEC §12, T10).
//!
//! What this adapter does **not** do: invent pinyin, phonemes, durations or
//! context features the worker has not documented. Two segment forms are
//! emitted, both grounded in the recovery archive:
//! * **english-proxy** (`backend/validation-final`): explicit
//!   `vowel ∈ english_vowel_proxies`, `context_mode = "english-proxy"`,
//!   per-segment `f0_hz`.
//! * **pinyin** (`crate::pinyin`): explicit `context` objects + `midi_note`,
//!   the recovered `make-pinyin-score` initial/final contract. Legacy routes
//!   retain the 0.15–30 s syllable domain. Verdant ONNX admits shorter notes
//!   when every segment fits the worker-declared minimum.

use std::sync::Arc;

use serde::Serialize;
use verdant_domain::timing::{TempoMap, midi_to_hz, note_name};
use verdant_domain::{Note, NoteId, Project, PronunciationSource, TrackId};

use crate::f0::{ControlTrack, control_track, has_automatic_portamento, snap_seconds_bound};
use crate::pinyin;
use crate::phonemes;
use crate::wire::{Capabilities, NativeScore, Payload, Route, Segment};

pub const MIN_SEGMENT_SECONDS: f64 = 0.025;
/// Only explicitly capable Verdant ONNX workers admit one 5 ms phone frame.
pub fn minimum_segment_seconds(route: Route, caps: &Capabilities) -> f64 {
    if route == Route::VerdantVoice && caps.native_score.as_ref().is_some_and(|ns| ns.compact_phone_states) {
        0.005
    } else {
        MIN_SEGMENT_SECONDS
    }
}

/// Host render range, separate from the bank's frozen training F0 contract.
/// Old workers and legacy routes retain their declared native range.
pub fn authored_f0_hz_range(route: Route, caps: &Capabilities) -> [f64; 2] {
    if route == Route::VerdantVoice {
        if let Some(declaration) = caps.other.get("voice_package").filter(|value|
            value.get("authored_f0_policy").and_then(serde_json::Value::as_str)
                == Some("native-target-reference-v1")) {
            if let Some(range) = declaration.get("authored_f0_hz_range")
                .and_then(|value| serde_json::from_value::<[f64; 2]>(value.clone()).ok())
                .filter(|range| range.iter().all(|value| value.is_finite())
                    && range[0] >= 35.0 && range[0] <= range[1] && range[1] <= 2000.0) {
                return range;
            }
        }
    }
    caps.native_score.as_ref().and_then(|native| native.explicit_f0_hz_range)
        .unwrap_or([35.0, 2000.0])
}

const DURATION_ROUNDOFF_SECONDS: f64 = 1e-12;
/// Rests at least this long start a new phrase (an explainable boundary).
pub const PHRASE_BREAK_SECONDS: f64 = 0.6;
/// Worker `score` limit: `score requires 1..512 explicit segments`.
pub const MAX_SCORE_SEGMENTS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub note_id: Option<NoteId>,
    /// Short, user-facing Chinese message.
    pub message: String,
    /// Technical detail for the collapsible diagnostics area.
    pub detail: String,
}

impl Diagnostic {
    fn error(code: &'static str, note: Option<&Note>, message: impl Into<String>, detail: impl Into<String>) -> Self {
        Diagnostic { code, severity: Severity::Error, note_id: note.map(|n| n.id.clone()), message: message.into(), detail: detail.into() }
    }
    fn warning(code: &'static str, note: Option<&Note>, message: impl Into<String>, detail: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Warning,
            note_id: note.map(|n| n.id.clone()),
            message: message.into(),
            detail: detail.into(),
        }
    }
}

/// How one note will be sung, as the inspector should show it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum NoteSupport {
    /// Renderable: explicit vowel proxy.
    VowelProxy {
        vowel: String,
    },
    /// Renderable: recovered `make-pinyin-score` inventory syllable
    /// (parsed token: `syllable`, `tone`, `initial`, `final_`).
    Pinyin(pinyin::PinyinToken),
    Phonemes(phonemes::ResolvedPronunciation),
    Unsupported,
}

/// Whether the worker renders sub-syllable (initial/final) timing, per the
/// recovered helper contract. Explicit user overrides additionally require
/// `native_score.phoneme_duration_control`; otherwise this split is read-only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum SubSyllableTiming {
    /// `native_score.inventory` is non-empty (or the worker declares
    /// `sub_syllable_timing: true`): syllables render as timed
    /// initial + final segments.
    Supported,
    /// Worker can only take one segment duration per syllable.
    SyllableTotal(&'static str),
}

/// Capability-driven phoneme-duration support for the UI. An explicit
/// `native_score.sub_syllable_timing` declaration wins; absent, a non-empty
/// recovered `inventory` implies it.
pub fn sub_syllable_timing(caps: &Capabilities) -> SubSyllableTiming {
    let Some(ns) = caps.native_score.as_ref() else {
        return SubSyllableTiming::SyllableTotal("后端未声明 native_score 能力");
    };
    match ns.sub_syllable_timing {
        Some(true) => SubSyllableTiming::Supported,
        Some(false) => SubSyllableTiming::SyllableTotal("后端声明 sub_syllable_timing=false：每音节只有一个时长"),
        None if !ns.inventory.is_empty() => SubSyllableTiming::Supported,
        None => SubSyllableTiming::SyllableTotal("native_score.inventory 为空：无恢复的音节清单"),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Phrase {
    pub track: TrackId,
    pub route: Route,
    pub index: usize,
    pub note_ids: Vec<NoteId>,
    pub start_tick: i64,
    pub end_tick: i64,
    /// Absolute timeline position of the phrase's first sample.
    pub start_seconds: f64,
    pub seconds: f64,
    /// Immutable phrase body shared with the scheduler and wire requests.
    pub score: Arc<NativeScore>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_music: Option<crate::wire::VoiceMusic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wideband: Option<crate::wire::Wideband>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocal_tract: Option<crate::wire::VocalTract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_dynamics: Option<crate::wire::ModelDynamics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consonant_dynamics: Option<crate::wire::ConsonantDynamics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<Arc<crate::expression::ExpressionTrack>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_transients: Option<crate::wire::ConsonantDynamics>,
    /// Present when the route takes a control track and a note in the
    /// phrase carries PIT / vibrato / explicit or automatic portamento.
    pub control_track: Option<Arc<ControlTrack>>,
}

/// A phrase that will not be sent because one of its notes has an error.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BlockedPhrase {
    pub note_ids: Vec<NoteId>,
    pub start_tick: i64,
    pub end_tick: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PhrasePlan {
    pub track: TrackId,
    pub route: Route,
    pub phrases: Vec<Phrase>,
    pub blocked: Vec<BlockedPhrase>,
    pub diagnostics: Vec<Diagnostic>,
}

impl PhrasePlan {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
    pub fn errors_for(&self, id: &NoteId) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(move |d| d.note_id.as_ref() == Some(id))
    }
}

/// Strip a trailing tone digit and normalise `v` → `ü`.
pub fn normalize_pinyin(p: &str) -> String {
    p.trim().to_ascii_lowercase().trim_end_matches(|c: char| ('1'..='5').contains(&c)).replace('v', "ü")
}

pub fn note_support(n: &Note, caps: &Capabilities) -> Result<NoteSupport, Diagnostic> {
    let Some(ns) = caps.native_score.as_ref() else {
        return Err(Diagnostic::error("E_RENDER_CAPABILITY", Some(n), "后端未声明 native_score 能力", "capabilities.native_score missing"));
    };
    if let Some(alphabet) = n.pronunciation.alphabet.as_deref() {
        let language = n.pronunciation.language.as_deref().unwrap_or("");
        let parts = crate::phonemizer::automatic::parts(&n.pronunciation)
            .map_err(|reason| Diagnostic::error("E_UNSUPPORTED_PHONEME", Some(n), reason.clone(), reason))?;
        let available = ns.phoneme_frontend.as_ref().is_some_and(|p|
            p.schema_version == phonemes::SCHEMA && p.alphabets.iter().any(|a| a == alphabet)
                && parts.as_ref().map_or_else(|| p.languages.contains_key(language), |parts|
                    parts.iter().all(|part| p.languages.contains_key(part.reading.language.as_deref().unwrap_or("zh")))));
        if !available {
            return Err(Diagnostic::error("E_RENDER_CAPABILITY", Some(n),
                format!("后端未声明 {language}/{alphabet} 音素接口；请更新配套引擎"), "phoneme_frontend capability/schema mismatch"));
        }
        if n.pronunciation.source == PronunciationSource::Unset {
            return Err(Diagnostic::error("E_PRONUNCIATION_UNSET", Some(n), "请先确认显式音素输入", "phonetic input has no source"));
        }
        return crate::reading::resolve_pronunciation(&n.pronunciation)
            .map(|mut reading| {
                phonemes::apply_kana_realization(&mut reading, n.pronunciation.reading_provenance.as_ref());
                NoteSupport::Phonemes(reading)
            })
            .map_err(|reason| Diagnostic::error("E_UNSUPPORTED_PHONEME", Some(n), reason.clone(), reason));
    }
    if n.pronunciation.source == PronunciationSource::Unset || n.pronunciation.pinyin.trim().is_empty() {
        let readings = verdant_domain::hanzi_readings(&n.lyric);
        let candidates = if readings.is_empty() { "字典没有唯一可用读音".into() } else { format!("字典候选读音：{}", readings.join(" / ")) };
        return Err(Diagnostic::error(
            if readings.len() > 1 { "E_PRONUNCIATION_AMBIGUOUS" } else { "E_PRONUNCIATION_UNSET" },
            Some(n),
            format!("「{}」缺少已确认的读音；{candidates}。请使用 汉字/pinyin 语法显式填写，如 上/shang4（一音符一音节）", n.lyric),
            "Dictionary spelling is separate from recovered acoustic support; ambiguous readings require an explicit choice",
        ));
    }
    let raw = n.pronunciation.pinyin.trim().to_lowercase();
    let has_tone = raw.chars().last().is_some_and(|c| c.is_ascii_digit());
    // Only the explicitly selected legacy diagnostic profile uses English proxies.
    // Dictionary mappings always require Chinese contexts, even if a stored
    // profile was inherited from a formerly authored proxy.
    if !has_tone && n.pronunciation.source != PronunciationSource::ExplicitMap
        && n.pronunciation.frontend_profile == "english-vowel-proxy-native-0.2" {
        let py = normalize_pinyin(&raw);
        if ns.english_vowel_proxies.iter().any(|v| *v == py) {
            return Ok(NoteSupport::VowelProxy { vowel: py });
        }
    }
    match pinyin::parse_token(&raw) {
        Ok(t) => {
            if !pinyin::in_inventory(&t.syllable, &ns.inventory) || ns.sub_syllable_timing == Some(false) {
                return Err(Diagnostic::error("E_RENDER_CAPABILITY", Some(n),
                    format!("后端未声明音节「{}」及其子音节时序支持", t.syllable),
                    "recovered frontend context requires worker inventory and sub_syllable_timing"));
            }
            Ok(NoteSupport::Pinyin(t))
        }
        Err(reason) => Err(Diagnostic::error(
            if ns.inventory.iter().any(|s| s == &raw) { "E_RENDER_CAPABILITY" } else { "E_UNSUPPORTED_SYLLABLE" },
            Some(n), format!("音节「{raw}」暂不能使用中文声学路径：{}", reason.0),
            reason.0,
        )),
    }
}

/// How a renderable note emits its segment(s), decided by `note_support`.
enum NoteRender {
    Continuation,
    Proxy(String),
    Pinyin(pinyin::PinyinToken),
    Phonemes(phonemes::ResolvedPronunciation),
}

/// Resolve the track override or project default without falling back from an
/// explicit unsupported route. The reserved original still needs catalog admission.
pub fn track_route(project: &Project, track: &verdant_domain::Track) -> Result<Route, String> {
    let route = track.render_route.as_deref().unwrap_or(&project.render_preferences.route);
    match route {
        "C0" if track.voice_ref.as_deref() == Some(crate::voices::DESKTOP_ORIGINAL_REFERENCE) => Ok(Route::C0),
        "C0" | "C1" | "A" => Err(format!("渲染路线 {route} 已停用；请选择 verdant-voice")),
        _ => Route::parse(route).ok_or_else(|| format!("不支持的渲染路线 {route:?}；请选择 verdant-voice")),
    }
}

/// Route label for the editor. Render preflight validates the stored selection.
pub fn desktop_route(project: &Project, track: &verdant_domain::Track, caps: Option<&Capabilities>) -> Route {
    if track_route(project, track) == Ok(Route::C0) && caps.is_some_and(|caps|
        caps.supports_route(Route::C0) && matches!(caps.voice_for_track(project, track),
            Ok(Some((_, voice))) if crate::voices::is_desktop_original(voice))) {
        Route::C0
    } else {
        Route::VerdantVoice
    }
}

pub fn is_desktop_voice(voice: &crate::wire::VoiceEntry) -> bool {
    let Some(manifest) = voice.files.get("verdant_voice_manifest") else { return false; };
    voice.available && !voice.cache_identity.is_empty() && manifest.available && manifest.sha256.is_some()
        && voice.voice_package.as_ref().is_some_and(|package|package.name=="verdant-voice"
            && crate::voices::valid_voice_version(&package.version))
        && crate::voices::PACKAGE_FILES.iter()
            .all(|(role,_)|voice.files.get(*role).is_some_and(|file|file.available&&file.sha256.is_some()))
        && voice.files.get("voice_source_admission").is_some_and(|receipt|
            receipt.available && receipt.sha256 == manifest.sha256)
}

pub fn desktop_voice<'a>(project: &Project, track: &verdant_domain::Track, caps: &'a Capabilities)
    -> Result<Option<(&'a str, &'a crate::wire::VoiceEntry)>, String>
{
    if track_route(project, track) == Ok(Route::C0) {
        return caps.voice_for_track(project, track);
    }
    let Some(catalog) = caps.voices.as_ref().filter(|catalog| catalog.version == 1) else {
        return caps.voice_for_track(project, track);
    };
    if track.unknown.contains_key("voice_selection") && track.voice_ref.is_none() {
        return Err("轨道的歌手来源绑定缺少稳定引用；请恢复已校验的歌手来源".into());
    }
    if let Some(reference) = track.voice_ref.as_deref() {
        let explicit_voice_package = reference.starts_with("verdant-voice-")
            || track.unknown.contains_key("voice_selection")
            || reference != "voice-sn";
        if explicit_voice_package {
            let selected = caps.voice_for_track(project, track)?;
            return selected.filter(|(_, voice)| is_desktop_voice(voice)).map(Some)
                .ok_or_else(|| "所选 verdant-voice 声库尚未通过来源校验".into());
        }
    }
    let selected = catalog.default_voice.as_deref().and_then(|id| catalog.voices.get_key_value(id))
        .filter(|(reference, voice)| is_desktop_voice(voice)&&crate::voices::voice_reference_matches_package(reference,voice))
        .or_else(|| catalog.voices.iter().find(|(reference, voice)| is_desktop_voice(voice)&&crate::voices::voice_reference_matches_package(reference,voice)));
    Ok(selected.map(|(id, voice)| (id.as_str(), voice)))
}

pub fn plan_desktop(project: &Project, track_id: &TrackId, caps: &Capabilities) -> PhrasePlan {
    plan_with_voice_policy(project, track_id, None, caps, true)
}

pub fn plan_all_desktop(project: &Project, first: &TrackId, caps: &Capabilities) -> Vec<PhrasePlan> {
    let mut order: Vec<_> = project.tracks.iter().collect();
    order.sort_by_key(|track| &track.id != first);
    order.into_iter().map(|track| plan_desktop(project, &track.id, caps)).collect()
}

/// Plans for every track, the `first` track (the one being edited) first so
/// the scheduler renders what the user is looking at before the rest.
pub fn plan_all(project: &Project, first: &TrackId, caps: &Capabilities) -> Vec<PhrasePlan> {
    let mut order: Vec<&verdant_domain::Track> = project.tracks.iter().collect();
    order.sort_by_key(|t| &t.id != first);
    order.into_iter().map(|t| plan_with_voice_policy(project, &t.id, None, caps, false)).collect()
}

/// Preflight + phrase split. Never sends anything. Track-level problems
/// (route, model files, tempo) block the whole track; a note-level error
/// blocks only the phrase containing that note, which stays visible as a
/// located diagnostic and a blocked span — nothing is silently dropped.
pub fn plan(project: &Project, track_id: &TrackId, route: Route, caps: &Capabilities) -> PhrasePlan {
    plan_with_voice_policy(project, track_id, Some(route), caps, false)
}

fn plan_with_voice_policy(project: &Project, track_id: &TrackId, requested: Option<Route>, caps: &Capabilities, desktop: bool) -> PhrasePlan {
    let mut diagnostics = Vec::new();
    let mut out = PhrasePlan { track: track_id.clone(), route: Route::VerdantVoice, phrases: Vec::new(), blocked: Vec::new(), diagnostics: Vec::new() };
    let Some(track) = project.track(track_id) else {
        out.diagnostics.push(Diagnostic::error("E_UNKNOWN_TRACK", None, "轨道不存在", track_id.to_string()));
        return out;
    };
    // Instrument tracks play their notes through the instrument, not a voice.
    if track.is_instrument() { return out; }
    let route = match track_route(project, track) {
        Ok(stored) => requested.unwrap_or(stored),
        Err(error) => {
            out.diagnostics.push(Diagnostic::error("E_RENDER_ROUTE", None, error,
                "explicit render route is retired or unknown"));
            return out;
        }
    };
    out.route = route;
    let selected_caps;
    let voice = if desktop { desktop_voice(project, track, caps) } else { caps.voice_for_track(project, track) };
    if route == Route::C0 && (track.voice_ref.as_deref() != Some(crate::voices::DESKTOP_ORIGINAL_REFERENCE)
        || !matches!(&voice,
        Ok(Some((reference, entry))) if *reference == crate::voices::DESKTOP_ORIGINAL_REFERENCE
            && crate::voices::is_desktop_original(entry))) {
        out.diagnostics.push(Diagnostic::error("E_VOICE_SELECTION", None,
            "原版渲染需要明确选择已解锁的原版声库", "C0 requires admitted original token/model fingerprints"));
        return out;
    }
    let caps = match voice {
        Ok(Some((_,voice))) => { selected_caps=caps.for_voice(voice); &selected_caps },
        Ok(None) => caps,
        Err(error) => { out.diagnostics.push(Diagnostic::error("E_VOICE_SELECTION",None,"当前轨道的歌手无法使用",error)); return out; }
    };
    let settings=match crate::phonemizer::settings_from_project(project){Ok(settings)=>settings,Err(error)=>{
        out.diagnostics.push(Diagnostic::error("E_PHONEMIZER_CONFIG",None,"读音配置无效",error));return out;
    }};
    let resolved=crate::phonemizer::resolve_track(track,&settings);
    diagnostics.extend(resolved.diagnostics.into_iter().map(|d|Diagnostic{code:d.code,severity:Severity::Error,note_id:d.note_id,message:d.message,detail:d.detail}));
    let mut resolved_notes = resolved.notes;
    let mut resolved_track: Option<verdant_domain::Track> = None;
    for (index, note) in track.notes.iter().enumerate() {
        if let Some(reading) = resolved_notes.remove(&note.id) {
            if note.pronunciation != reading.pronunciation {
                let owned = resolved_track.get_or_insert_with(|| track.clone());
                owned.notes[index].pronunciation = reading.pronunciation;
            }
        }
    }
    let track=resolved_track.as_ref().unwrap_or(track);
    let mut track_blocked = false;
    if !caps.supports_route(route) {
        let advertised: Vec<&str> = caps.routes.iter().map(|id| {
            Route::parse(id).map(Route::display_name).unwrap_or(id)
        }).collect();
        diagnostics.push(Diagnostic::error(
            "E_RENDER_CAPABILITY",
            None,
            format!("后端未提供路线 {route}"),
            format!("routes={advertised:?}"),
        ));
        track_blocked = true;
    }
    let legacy_processors = route == Route::C0;
    let voice_music = if route == Route::VerdantVoice { caps.default_voice_music() } else { None };
    if let Some(settings) = project.render_preferences.synthesis.as_ref().filter(|_| legacy_processors) {
        let error = settings.validate().err().or_else(|| {
            let ns = caps.native_score.as_ref();
            let declared = ns.and_then(|ns| ns.vowel_balance_db_range);
            if !declared.is_some_and(|r| r[0].is_finite() && r[1].is_finite() && (r[0]..=r[1]).contains(&settings.vowel_balance_db)) {
                return Some("后端未声明所选元音谐波补偿范围".into());
            }
            if settings.wideband.is_some() {
                return Some("原版声库不再提供高频扩展".into());
            }
            if let Some(correction) = &settings.phonetic_correction {
                let supported = ns.and_then(|ns| ns.phonetic_correction.as_ref()).is_some_and(|c| {
                    c.routes.iter().any(|declared| declared == route.as_str())
                        && c.methods.contains(&correction.method)
                        && c.strength_range.is_some_and(|r| r[0].is_finite() && r[1].is_finite()
                            && (r[0]..=r[1]).contains(&correction.strength))
                });
                if !supported { return Some("后端未声明所选音素矫正方式或强度".into()); }
            }
            None
        });
        if let Some(error) = error {
            diagnostics.push(Diagnostic::error("E_SYNTHESIS_CAPABILITY", None,
                format!("当前合成设置无法渲染：{error}"), "Native v10 settings require explicit worker capabilities"));
            track_blocked = true;
        }
    }
    if let Some(tract) = project.render_preferences.vocal_tract.as_ref().filter(|_| legacy_processors) {
        let error = tract.validate().err().or_else(|| {
            let supported = caps.native_score.as_ref().and_then(|ns| ns.vocal_tract.as_ref()).is_some_and(|v| {
                    v.routes.iter().any(|declared| declared == route.as_str())
                        && v.methods.contains(&tract.method)
                        && v.ratio_range.is_some_and(|r| r[0].is_finite() && r[1].is_finite()
                            && (r[0]..=r[1]).contains(&tract.ratio))
                        && tract.anchor_ratios.is_none_or(|anchors| {
                            v.anchor_fractions == Some([0.1, 0.3])
                                && v.anchor_ratio_range.is_some_and(|r| r[0].is_finite() && r[1].is_finite()
                                    && anchors.iter().all(|value| (r[0]..=r[1]).contains(value)))
                        })
                });
            (!supported).then(|| "后端未声明所选声道变换方式、比例或路线".into())
        });
        if let Some(error) = error {
            diagnostics.push(Diagnostic::error("E_SYNTHESIS_CAPABILITY", None,
                format!("当前声道变换无法渲染：{error}"), "Native envelope transformation requires explicit worker capability"));
            track_blocked = true;
        }
    }
    if let Some(dynamics) = project.render_preferences.model_dynamics.as_ref().filter(|_| legacy_processors) {
        let error = dynamics.validate().err().or_else(|| {
            let supported = caps.native_score.as_ref().and_then(|ns| ns.model_dynamics.as_ref()).is_some_and(|d| {
                    d.routes.iter().any(|declared| declared == route.as_str())
                        && d.methods.contains(&dynamics.method)
                        && d.minimum_strength.zip(d.maximum_strength).is_some_and(|(lo, hi)|
                            lo.is_finite() && hi.is_finite() && (lo..=hi).contains(&dynamics.strength))
                        && d.minimum_attenuation_db.zip(d.maximum_attenuation_db).is_some_and(|(lo, hi)|
                            lo.is_finite() && hi.is_finite() && (lo..=hi).contains(&dynamics.max_attenuation_db))
                });
            (!supported).then(|| "后端未声明所选元音动态方式、强度、衰减上限或路线".into())
        });
        if let Some(error) = error {
            diagnostics.push(Diagnostic::error("E_SYNTHESIS_CAPABILITY", None,
                format!("当前元音动态无法渲染：{error}"), "Native vowel dynamics require explicit worker capability"));
            track_blocked = true;
        }
    }
    if let Some(timing) = &project.render_preferences.phoneme_timing {
        let error = timing.validate().err().or_else(|| {
            let supported = caps.native_score.as_ref().is_some_and(|ns| ns.continuous_f0_control)
                && caps.native_score.as_ref().and_then(|ns| ns.phoneme_timing.as_ref()).is_some_and(|t| {
                    t.routes.iter().any(|declared| declared == route.as_str())
                        && t.methods.contains(&timing.method)
                        && t.timing_resolution_seconds == Some(0.005)
                });
            (!supported).then(|| "后端未声明元音锚点记录与 5 ms 时钟校验能力".into())
        });
        if let Some(error) = error {
            diagnostics.push(Diagnostic::error("E_PHONEME_TIMING_CAPABILITY", None,
                format!("当前元音对拍无法渲染：{error}"), "Explicit materialized phoneme timing requires verified worker support"));
            track_blocked = true;
        }
    }
    if let Some(dynamics) = project.render_preferences.consonant_dynamics.as_ref().filter(|_| legacy_processors) {
        let error = dynamics.validate().err().or_else(|| {
            let supported = caps.native_score.as_ref().and_then(|ns| ns.consonant_dynamics.as_ref()).is_some_and(|d| {
                    d.routes.iter().any(|declared| declared == route.as_str())
                        && d.methods.contains(&dynamics.method)
                        && d.minimum_strength.zip(d.maximum_strength).is_some_and(|(lo, hi)|
                            lo.is_finite() && hi.is_finite() && (lo..=hi).contains(&dynamics.strength))
                        && d.minimum_attenuation_db.zip(d.maximum_attenuation_db).is_some_and(|(lo, hi)|
                            lo.is_finite() && hi.is_finite() && (lo..=hi).contains(&dynamics.max_attenuation_db))
                });
            (!supported).then(|| "后端未声明所选辅音动态方式、强度、衰减上限或路线".into())
        });
        if let Some(error) = error {
            diagnostics.push(Diagnostic::error("E_SYNTHESIS_CAPABILITY", None,
                format!("当前辅音动态无法渲染：{error}"), "Native consonant dynamics require explicit worker capability"));
            track_blocked = true;
        }
    }
    let missing = caps.missing_files(route);
    if let Some(settings) = &project.render_preferences.output_transients {
        let supported = settings.validate().is_ok()
            && caps.native_score.as_ref().and_then(|n| n.output_transients.as_ref()).is_some_and(|c|
                c.methods.contains(&settings.method) && c.routes.iter().any(|r| r == route.as_str())
                && c.minimum_strength.zip(c.maximum_strength).is_some_and(|(a,b)| a.is_finite() && b.is_finite() && (a..=b).contains(&settings.strength))
                && c.minimum_attenuation_db.zip(c.maximum_attenuation_db).is_some_and(|(a,b)| a.is_finite() && b.is_finite() && (a..=b).contains(&settings.max_attenuation_db)));
        if !supported {
            diagnostics.push(Diagnostic::error("E_OUTPUT_TRANSIENTS_CAPABILITY", None,
                "后端未声明所选最终辅音约束方式与范围", "final-waveform transient controls require explicit support"));
            track_blocked = true;
        }
    }
    let authored: std::collections::BTreeSet<_> = track.notes.iter().filter(|n|!n.is_muted()).flat_map(|n| n.curves.expression_points.iter())
        .filter(|(key, points)| key.as_str() != "POR" && !points.is_empty()).map(|(key, _)| key.as_str()).collect();
    for key in authored {
        let supported = caps.native_score.as_ref().and_then(|n| n.expression.as_ref())
            .is_some_and(|e| e.methods.iter().any(|m| m == "vocaloid-controls-v1") && e.parameters.iter().any(|p| p == key)
                && e.routes.iter().any(|r| r == route.as_str()));
        if !supported {
            diagnostics.push(Diagnostic::error("E_EXPRESSION_CAPABILITY", None, format!("后端未声明 {key} 的表达合成能力"),
                "authored voice controls cannot be silently discarded"));
            track_blocked = true;
        }
    }
    if !missing.is_empty() {
        diagnostics.push(Diagnostic::error(
            "E_BACKEND_FILES",
            None,
            format!("后端缺少模型文件：{}", missing.join("、")),
            "capabilities.files[*].available=false; obtain them per OpenGreendam/docs reproduction-and-packaging.md",
        ));
        track_blocked = true;
    }
    let tempo = match TempoMap::new(&project.tempo_map, project.ppq) {
        Ok(t) => t,
        Err(e) => {
            diagnostics.push(Diagnostic::error("E_TEMPO", None, "速度表无效", e.to_string()));
            out.diagnostics = diagnostics;
            return out;
        }
    };
    let mut bad: std::collections::HashSet<NoteId> = diagnostics.iter().filter(|d|d.severity==Severity::Error).filter_map(|d|d.note_id.clone()).collect();
    if track.monophonic {
        let at = |n: &Note| {
            let position = verdant_domain::MeterMap::new(&project.time_signatures,project.ppq).expect("validated meter").position_at(n.start_tick);
            format!("「{}」{}.{}",if n.lyric.is_empty(){"·"}else{&n.lyric},position.bar,position.beat)
        };
        for (a, b) in track.overlaps() {
            let (Some(na), Some(nb)) = (track.note(&a), track.note(&b)) else { continue };
            for (x, y) in [(na, nb), (nb, na)] {
                diagnostics.push(Diagnostic::error(
                    "E_OVERLAP",
                    Some(x),
                    format!("{} 与 {} 重叠（单声部轨，不会自动截短）", at(x), at(y)),
                    format!("note {} overlaps note {}", x.id, y.id),
                ));
                bad.insert(x.id.clone());
            }
        }
    }
    let f0_range = authored_f0_hz_range(route, caps);
    let f0_support = crate::params::support(crate::params::Param::Pit, route, Some(caps));
    let max_phrase = if caps.max_phrase_seconds > 0.0 { caps.max_phrase_seconds } else { 30.0 };
    let segment_minimum = minimum_segment_seconds(route, caps);

    // Per-note checks.
    let mut renders: std::collections::HashMap<NoteId, NoteRender> = std::collections::HashMap::new();
    for n in track.notes.iter().filter(|n|!n.is_muted()) {
        let mut fine = true;
        let mut pinyin_token: Option<pinyin::PinyinToken> = None;
        if !n.is_continuation() { match note_support(n, caps) {
            Ok(NoteSupport::VowelProxy { vowel }) => {
                renders.insert(n.id.clone(), NoteRender::Proxy(vowel));
            }
            Ok(NoteSupport::Pinyin(t)) => {
                pinyin_token = Some(t.clone());
                renders.insert(n.id.clone(), NoteRender::Pinyin(t));
            }
            Ok(NoteSupport::Phonemes(p)) => {
                let reasons: std::collections::BTreeSet<_> = p.phones.iter()
                    .filter_map(|phone| phone.approximation_reason.clone()).collect();
                if !reasons.is_empty() {
                    diagnostics.push(Diagnostic::warning("W_PHONEME_APPROXIMATION", Some(n),
                        format!("「{}」使用已选择的跨语言近似音素映射", n.lyric), reasons.into_iter().collect::<Vec<_>>().join("\n")));
                }
                renders.insert(n.id.clone(), NoteRender::Phonemes(p));
            }
            Ok(NoteSupport::Unsupported) => fine = false,
            Err(d) => {
                diagnostics.push(d);
                fine = false;
            }
        }
        }
        if n.is_continuation(){
            let previous=track.notes.iter().take_while(|p|p.id!=n.id).last();
            let usable = previous.is_some_and(|p| !p.is_muted() && p.end_tick()==n.start_tick && !bad.contains(&p.id)
                && match renders.get(&p.id) { Some(NoteRender::Phonemes(reading))=>reading.phones.last().is_some_and(|phone|phone.native.role=="nucleus"),Some(_)=>true,None=>false });
            if !usable{
                diagnostics.push(Diagnostic::error("E_CONTINUATION",Some(n),"延音须紧邻同轨道中已解析的前一音节","muted notes, rests, unresolved notes and track boundaries break continuation"));fine=false;
            }
            renders.insert(n.id.clone(),NoteRender::Continuation);pinyin_token=None;
        }
        if (n.has_f0_modulation()||n.is_continuation()) && !matches!(f0_support, crate::params::Support::Synthesis(_)) {
            let reason = match &f0_support {
                crate::params::Support::Unsupported(r) => *r,
                _ => "",
            };
            diagnostics.push(Diagnostic::error(
                "E_RENDER_CAPABILITY",
                Some(n),
                "当前路线不支持音高弯曲 / 颤音 / 滑音（不会切碎音符代替）",
                reason,
            ));
            fine = false;
        }
        let f0 = midi_to_hz(n.midi_note, n.detune_millicents as i64);
        if !(f0_range[0]..=f0_range[1]).contains(&f0) {
            diagnostics.push(Diagnostic::error(
                "E_PITCH_RANGE",
                Some(n),
                format!("「{}」{}（{f0:.1} Hz）超出 {route} 支持音域 {:.1}–{:.1} Hz",
                    if n.lyric.is_empty() { "·" } else { &n.lyric },
                    note_name(n.midi_note), f0_range[0], f0_range[1]),
                format!("explicit_f0_hz_range={f0_range:?}"),
            ));
            fine = false;
        } else if !(130.0..=700.0).contains(&f0) {
            diagnostics.push(Diagnostic::warning(
                "W_PITCH_UNVERIFIED",
                Some(n),
                format!("音高 {:.0} Hz 在接受范围内，但未经听评", f0),
                "accepted by the worker; outside the conservative range Verdant highlights",
            ));
        }
        let dur = snap_seconds_bound(
            tempo.seconds_at(n.end_tick() as f64) - tempo.seconds_at(n.start_tick as f64),
            max_phrase.min(pinyin::MAX_SYLLABLE_SECONDS),
        );
        if let Some(weights) = n.pronunciation.phoneme_durations_permille.as_ref().filter(|_|!n.is_continuation()) {
            let supported = caps.native_score.as_ref().is_some_and(|ns| ns.phoneme_duration_control);
            let count = match renders.get(&n.id) {
                Some(NoteRender::Phonemes(p)) => p.phones.len(),
                _ if pinyin_token.as_ref().is_some_and(|token| !token.initial.is_empty()) => 2,
                _ => 1,
            };
            let valid = weights.len() == count
                && weights.iter().all(|&weight| weight > 0 && dur * weight as f64 / 1000.0 >= segment_minimum - DURATION_ROUNDOFF_SECONDS)
                && weights.iter().map(|&weight| weight as u64).sum::<u64>() == 1000;
            if !supported || !valid {
                diagnostics.push(Diagnostic::error(
                    if supported { "E_PHONEME_TIMING" } else { "E_RENDER_CAPABILITY" },
                    Some(n),
                    if supported { format!("音素时长比例须与实际展开音素对应，总和 1000，且每段不少于 {:.0} ms", segment_minimum * 1000.0) }
                    else { "后端未声明音素时长编辑能力：只能使用整音节时长和既有前端默认切分".into() },
                    format!("phoneme_duration_control={supported}; segments={count}; weights={weights:?}"),
                ));
                fine = false;
            }
        }
        if let Some(NoteRender::Phonemes(p)) = renders.get(&n.id) {
            if let Err(reason) = crate::reading::allocate_reading(&n.pronunciation, p, dur, segment_minimum) {
                diagnostics.push(Diagnostic::error("E_PHONEME_TIMING", Some(n), reason.clone(), reason));
                fine = false;
            }
        }
        if let Some(token) = pinyin_token.as_ref() {
            let minimum = if route == Route::VerdantVoice {
                segment_minimum * if token.initial.is_empty() { 1.0 } else { 2.0 }
            } else {
                pinyin::MIN_SYLLABLE_SECONDS
            };
            if dur < minimum - DURATION_ROUNDOFF_SECONDS || dur > pinyin::MAX_SYLLABLE_SECONDS {
                diagnostics.push(Diagnostic::error(
                    "E_SYLLABLE_DOMAIN",
                    Some(n),
                    format!("拼音音节时长 {dur:.3}s 超出支持范围 {minimum:.3}–{:.3}s", pinyin::MAX_SYLLABLE_SECONDS),
                    format!("{route} requires syllable_seconds in {minimum:?}..={:?}", pinyin::MAX_SYLLABLE_SECONDS),
                ));
                fine = false;
            }
            let midi = n.midi_note as f64 + n.detune_millicents as f64 / 100_000.0;
            if !(0.0..=127.0).contains(&midi) {
                diagnostics.push(Diagnostic::error(
                    "E_PITCH_RANGE",
                    Some(n),
                    "拼音段音高超出 MIDI 0–127",
                    format!("midi_note={midi}"),
                ));
                fine = false;
            }
        }
        if dur < segment_minimum - DURATION_ROUNDOFF_SECONDS {
            diagnostics.push(Diagnostic::error("E_NOTE_TOO_SHORT", Some(n), format!("音符短于 {:.0} ms", segment_minimum * 1000.0), format!("{dur:.4}s")));
            fine = false;
        }
        if dur > max_phrase {
            diagnostics.push(Diagnostic::error(
                "E_PHRASE_TOO_LONG",
                Some(n),
                format!("单音符超过 {max_phrase} 秒，不会被悄悄裁断"),
                format!("{dur:.3}s"),
            ));
            fine = false;
        }
        if !fine {
            bad.insert(n.id.clone());
        }
    }
    out.diagnostics = diagnostics;
    if track_blocked {
        return out;
    }

    // Group every note into phrases by rests / the per-job limit; phrases
    // with a bad note are blocked as a whole.
    let mut groups: Vec<Vec<&Note>> = Vec::new();
    for n in track.notes.iter().filter(|n|!n.is_muted()) {
        if let Some(g) = groups.last_mut() {
            let prev = g[g.len() - 1];
            let rest = tempo.seconds_at(n.start_tick as f64) - tempo.seconds_at(prev.end_tick() as f64);
            let span = snap_seconds_bound(tempo.seconds_at(n.end_tick() as f64) - tempo.seconds_at(g[0].start_tick as f64), max_phrase);
            if rest < PHRASE_BREAK_SECONDS && span <= max_phrase {
                g.push(n);
                continue;
            }
        }
        groups.push(vec![n]);
    }
    let takes_control = matches!(f0_support, crate::params::Support::Synthesis(_));
    for g in groups {
        let first = g[0];
        let last = g[g.len() - 1];
        if first.is_continuation(){out.diagnostics.push(Diagnostic::error("E_CONTINUATION_PHRASE",Some(first),"延音链超出单次渲染边界；请缩短连续音节","a continuation cannot create a new acoustic onset"));}
        if first.is_continuation() || g.iter().any(|n| bad.contains(&n.id)) {
            out.blocked.push(BlockedPhrase {
                note_ids: g.iter().map(|n| n.id.clone()).collect(),
                start_tick: first.start_tick,
                end_tick: last.end_tick(),
            });
            continue;
        }
        let start_s = tempo.seconds_at(first.start_tick as f64);
        let end_s = tempo.seconds_at(last.end_tick() as f64);
        let phrase_seconds = snap_seconds_bound(end_s - start_s, max_phrase);
        let mut segments:Vec<Segment> = Vec::with_capacity(g.len());
        let mut last_nucleus:Option<(usize,Segment)>=None;
        for (i, n) in g.iter().enumerate() {
            // Absolute-boundary arithmetic: no accumulated per-note rounding.
            let s = tempo.seconds_at(n.start_tick as f64);
            let e = tempo.seconds_at(n.end_tick() as f64);
            let total = snap_seconds_bound(e - s, max_phrase.min(pinyin::MAX_SYLLABLE_SECONDS));
            let gap = g.get(i + 1).map(|m| tempo.seconds_at(m.start_tick as f64) - e).unwrap_or(0.0);
            let gap_after = gap.max(0.0);
            let segment_start=segments.len();
            match &renders[&n.id] {
                NoteRender::Continuation=>{
                    let (anchor,mut nucleus)=last_nucleus.clone().expect("continuation preflight requires preceding nucleus");
                    nucleus.continuation_of=Some(anchor);
                    nucleus.label=format!("{} · sustained vowel",n.lyric);nucleus.duration_seconds=total;nucleus.gap_after_seconds=gap_after;
                    if nucleus.f0_hz.is_some(){nucleus.f0_hz=Some(midi_to_hz(n.midi_note,n.detune_millicents as i64));}
                    if nucleus.midi_note.is_some(){nucleus.midi_note=Some(n.midi_note as f64+n.detune_millicents as f64/100_000.);}
                    segments.push(nucleus);
                }
                NoteRender::Proxy(vowel) => {
                    let f0 = midi_to_hz(n.midi_note, n.detune_millicents as i64);
                    segments.push(Segment {
                        continuation_of: None,
                        label: format!("{} {} via english-proxy", if n.lyric.is_empty() { "_" } else { &n.lyric }, vowel),
                        vowel: Some(vowel.clone()),
                        context_mode: Some("english-proxy".into()),
                        context: None,
                        duration_seconds: total,
                        gap_after_seconds: gap_after,
                        f0_hz: Some((f0 * 1000.0).round() / 1000.0),
                        midi_note: None,
                        voiced: true,
                        excitation_note: None,
                        phoneme: None,
                    });
                }
                NoteRender::Pinyin(token) => {
                    let midi = round_us(n.midi_note as f64 + n.detune_millicents as f64 / 100_000.0);
                    // Existing frontend heuristic: initial = min(0.11, 0.22 * syllable);
                    // Preserve the note's duration; native quantizes cumulative
                    // absolute boundaries once for phones, gaps and control.
                    let init_seconds = if token.initial.is_empty() {
                        0.0
                    } else if let Some(weights) = &n.pronunciation.phoneme_durations_permille {
                        total * weights[0] as f64 / 1000.0
                    } else {
                        pinyin::bounded_initial_seconds(token, total, segment_minimum)
                    };
                    let final_seconds = total - init_seconds;
                    if let Some(init) = pinyin::initial_spec(token, total) {
                        segments.push(Segment {
                        continuation_of: None,
                            label: init.label,
                            vowel: None,
                            context_mode: None,
                            context: Some(init.context),
                            duration_seconds: init_seconds,
                            gap_after_seconds: 0.0,
                            f0_hz: None,
                            midi_note: Some(midi),
                            voiced: init.voiced,
                            excitation_note: Some(init.excitation_note.into()),
                            phoneme: None,
                        });
                    }
                    let (label, ctx) = pinyin::final_spec(token, total);
                    segments.push(Segment {
                        continuation_of: None,
                        label,
                        vowel: None,
                        context_mode: None,
                        context: Some(ctx),
                        duration_seconds: final_seconds,
                        gap_after_seconds: gap_after,
                        f0_hz: None,
                        midi_note: Some(midi),
                        voiced: true,
                        excitation_note: None,
                        phoneme: None,
                    });
                }
                NoteRender::Phonemes(p) => {
                    let midi = round_us(n.midi_note as f64 + n.detune_millicents as f64 / 100_000.0);
                    let durations = crate::reading::allocate_reading(&n.pronunciation, p, total, segment_minimum)
                        .expect("phonetic timing passed preflight");
                    let verified_kana = phonemes::matching_kana_reading(p, n.pronunciation.reading_provenance.as_ref());
                    for (phone_index, (phone, duration_seconds)) in p.phones.iter().zip(durations).enumerate() {
                        let mut context = phone.context.clone();
                        // The renderer binds neighbors across the whole phrase,
                        // including adjacent notes and authored rests.
                        context.features.remove("L_Phone");
                        context.features.remove("R_Phone");
                        context.assumptions.retain(|s| !s.starts_with("L/R phone IDs use"));
                        context.assumptions.push("Phrase renderer supplies score-adjacent neighbors across note boundaries; original iterator traversal is not claimed.".into());
                        let kana_mora = verified_kana.as_ref().and_then(|reading| reading.moras.get(phone.canonical.syllable_index));
                        let kana_input = kana_mora.map(|mora| mora.kana.as_str());
                        if let Some(mora) = kana_mora {
                            context.assumptions.extend(mora.assumptions.iter().cloned());
                        }
                        segments.push(Segment {
                        continuation_of: None,
                            label: format!("{} /{}/ → {}", n.lyric, phone.canonical.symbol, phone.native_key),
                            vowel: None, context_mode: None, context: Some(context), duration_seconds,
                            gap_after_seconds: if phone_index + 1 == p.phones.len() { gap_after } else { 0.0 },
                            f0_hz: None, midi_note: Some(midi), voiced: phone.native.voiced,
                            excitation_note: phone.approximation_reason.clone(),
                            phoneme: Some(crate::wire::PhonemeTrace {
                                canonical_ipa: phone.canonical.symbol.clone(), input_symbol: kana_input.unwrap_or(&phone.canonical.input_symbol).into(),
                                input_alphabet: if kana_input.is_some() { "kana".into() } else { p.alphabet.clone() }, source_language: phone.source_language.clone(),
                                source_note_id: voice_music.as_ref().map(|_| n.id.to_string()),
                                source_index: if kana_input.is_some() { phone.canonical.syllable_index } else { phone.canonical.source_index }, syllable_index: phone.canonical.syllable_index,
                                stress: phone.canonical.stress, length: phone.canonical.length,
                                native_key: phone.native_key.clone(), support: phone.support.clone(),
                                approximation_reason: phone.approximation_reason.clone(),
                                realization: phone.realization.clone(),
                            }),
                        });
                    }
                }
            }
            let nucleus=match &renders[&n.id]{NoteRender::Proxy(_)|NoteRender::Pinyin(_)=>segments.last().cloned().map(|segment|(segments.len()-1,segment)),NoteRender::Phonemes(reading)=>reading.phones.iter().rposition(|phone|phone.native.role=="nucleus").and_then(|index|segments.get(segment_start+index).cloned().map(|segment|(segment_start+index,segment))),NoteRender::Continuation=>last_nucleus.clone()};
            last_nucleus=nucleus;
        }
        // Keep a phrase exactly on the admitted limit on the same wire clock.
        // Only the last interval absorbs <= 1 ps arithmetic residue; interior
        // note/phone boundaries are neither rounded nor redistributed.
        let emitted_seconds = segments.iter().map(|s| s.duration_seconds + s.gap_after_seconds).sum::<f64>();
        if phrase_seconds == max_phrase && (emitted_seconds - phrase_seconds).abs() <= DURATION_ROUNDOFF_SECONDS {
            let (last, prefix) = segments.split_last_mut().expect("a renderable phrase has a segment");
            let prefix_seconds = prefix.iter().map(|s| s.duration_seconds + s.gap_after_seconds).sum::<f64>();
            last.duration_seconds = phrase_seconds - prefix_seconds - last.gap_after_seconds;
        }
        if segments.len() > MAX_SCORE_SEGMENTS {
            out.diagnostics.push(Diagnostic::error(
                "E_PHRASE_TOO_LONG",
                Some(first),
                format!("乐句展开为 {} 段，超过后端 512 段上限", segments.len()),
                format!("score requires 1..={MAX_SCORE_SEGMENTS} explicit segments"),
            ));
            out.blocked.push(BlockedPhrase {
                note_ids: g.iter().map(|n| n.id.clone()).collect(),
                start_tick: first.start_tick,
                end_tick: last.end_tick(),
            });
            continue;
        }
        let control_track =
            (takes_control && (g.iter().any(|n| n.has_f0_modulation()||n.is_continuation()) || has_automatic_portamento(&g, &tempo)))
                .then(|| control_track(&g, &tempo, start_s, phrase_seconds));
        // Validate the transmitted values, including PIT and vibrato peaks.
        if let Some(control) = control_track.as_ref() {
            let mut reported = std::collections::HashSet::new();
            for ((&relative_time, &hz), &uv) in control.frame_times_seconds.iter().zip(&control.f0_hz).zip(&control.uv) {
                if uv || (f0_range[0]..=f0_range[1]).contains(&hz) { continue; }
                // Use the same phrase-relative half-open note membership as
                // control_track, including nonzero origins and exact borders.
                let index = g.partition_point(|n| tempo.seconds_at(n.end_tick() as f64) - start_s <= relative_time);
                let note = g.get(index).copied().unwrap_or(last);
                if reported.insert(note.id.clone()) {
                    out.diagnostics.push(Diagnostic::error(
                        "E_CONTROL_PITCH_RANGE",
                        Some(note),
                        format!("「{}」{} 的音高曲线在 {:.3}s 达到 {:.3} Hz，超出 {route} 支持音域 {:.1}–{:.1} Hz",
                            if note.lyric.is_empty() { "·" } else { &note.lyric },
                            note_name(note.midi_note), start_s + relative_time,
                            hz, f0_range[0], f0_range[1]),
                        format!("note_start_tick={}; control_time_in_phrase_seconds={relative_time:.9}; f0_hz={hz}; explicit_f0_hz_range={f0_range:?}; PIT/vibrato/portamento are checked after composition; no clamping", note.start_tick),
                    ));
                }
            }
            if !reported.is_empty() {
                out.blocked.push(BlockedPhrase {
                    note_ids: g.iter().map(|n| n.id.clone()).collect(),
                    start_tick: first.start_tick,
                    end_tick: last.end_tick(),
                });
                continue;
            }
        }
        out.phrases.push(Phrase {
            track: track_id.clone(),
            route,
            index: out.phrases.len(),
            note_ids: g.iter().map(|n| n.id.clone()).collect(),
            start_tick: first.start_tick,
            end_tick: last.end_tick(),
            start_seconds: start_s,
            seconds: phrase_seconds,
            score: Arc::new(NativeScore {
                title: format!("Verdant {} phrase {}", track.name, out.phrases.len() + 1), segments, original_hierarchy: None,
                synthesis: project.render_preferences.synthesis.as_ref().filter(|_| legacy_processors).map(|settings| crate::wire::NativeSynthesis {
                    vowel_balance_db: settings.vowel_balance_db,
                    phonetic_correction: settings.phonetic_correction.as_ref().map(|c| crate::wire::PhoneticCorrection {
                        method: c.method.clone(), strength: c.strength,
                    }),
                }),
                phoneme_timing: None,
                continuity: None,
            }),
            voice_music: voice_music.clone(),
            wideband: None,
            vocal_tract: project.render_preferences.vocal_tract.as_ref().filter(|_| legacy_processors)
                .map(|tract| crate::wire::VocalTract {
                    method: tract.method.clone(), ratio: tract.ratio, anchor_ratios: tract.anchor_ratios,
                }),
            model_dynamics: project.render_preferences.model_dynamics.as_ref().filter(|_| legacy_processors).map(|d| crate::wire::ModelDynamics {
                method: d.method.clone(), strength: d.strength, max_attenuation_db: d.max_attenuation_db,
            }),
            consonant_dynamics: project.render_preferences.consonant_dynamics.as_ref().filter(|_| legacy_processors).map(|d| crate::wire::ConsonantDynamics {
                method: d.method.clone(), strength: d.strength, max_attenuation_db: d.max_attenuation_db,
            }),
            expression: None,
            output_transients: project.render_preferences.output_transients.as_ref().map(|d| crate::wire::ConsonantDynamics {
                method: d.method.clone(), strength: d.strength, max_attenuation_db: d.max_attenuation_db,
            }),
            control_track: control_track.map(Arc::new),
        });
    }
    if project.render_preferences.phoneme_timing.is_some() {
        crate::phoneme_timing::anchor_plan(&mut out, project, track, &tempo, caps);
    }
    // Earlier v1 workers accepted only 4096 transmitted points. A larger
    // derived-point budget must be declared, not inferred from a route name.
    let expression_point_limit = caps.native_score.as_ref().and_then(|n| n.expression.as_ref())
        .and_then(|expression| expression.maximum_compiled_curve_points).unwrap_or(4096);
    for mut phrase in std::mem::take(&mut out.phrases) {
        match crate::expression::for_phrase(&phrase, track, &tempo, expression_point_limit) {
            Ok(expression) => {
                phrase.expression = expression.map(Arc::new);
                phrase.index = out.phrases.len();
                out.phrases.push(phrase);
            }
            Err(error) => {
                out.diagnostics.push(Diagnostic::error("E_EXPRESSION_POINT_LIMIT", track.note(&error.note_id),
                    format!("{} 曲线加入变速边界后超过当前 {} 个传输点预算；请减少控制点或速度变化", error.parameter,
                        error.maximum_points),
                    format!("note={}; parameter={}; compiled_points_at_least={}; authored limit=4096; tempo boundaries are never dropped",
                        error.note_id, error.parameter, error.point_count)));
                out.blocked.push(BlockedPhrase { note_ids: phrase.note_ids, start_tick: phrase.start_tick, end_tick: phrase.end_tick });
            }
        }
    }
    out
}

fn round_us(s: f64) -> f64 {
    (s * 1e6).round() / 1e6
}

/// The exact payload for one phrase.
pub fn payload_for(phrase: &Phrase, route: Route, native_seed: u32) -> Payload {
    Payload {
        kind: "native_score",
        score: Some(phrase.score.clone()),
        native_seed: Some(native_seed),
        voice_music: if route == Route::VerdantVoice { phrase.voice_music.clone() } else { None },
        wideband: phrase.wideband.clone(),
        vocal_tract: phrase.vocal_tract.clone(),
        model_dynamics: phrase.model_dynamics.clone(),
        consonant_dynamics: phrase.consonant_dynamics.clone(),
        expression: phrase.expression.clone(),
        output_transients: phrase.output_transients.clone(),
        control_track: phrase.control_track.clone(),
    }
}

/// Content key for cache and binding (SPEC §11): canonical wire payload +
/// route + protocol + implementation version + model hashes. UI state never
/// enters; the revision never enters (undo to identical content hits).
pub fn render_digest(payload: &Payload, route: Route, caps: &Capabilities) -> String {
    let files: Vec<_> = caps.files.iter().filter(|(role, _)| role.as_str() != "vocoder").map(|(role, status)| (role, &status.sha256)).collect();
    let key = serde_json::json!({
        "protocol": crate::wire::PROTOCOL,
        "engine": caps.engine,
        "native_version": caps.native_version,
        "route": route.as_str(),
        "files": files,
        "payload": payload,
    });
    verdant_domain::digest::sha256_hex(serde_json::to_string(&key).unwrap().as_bytes())
}
