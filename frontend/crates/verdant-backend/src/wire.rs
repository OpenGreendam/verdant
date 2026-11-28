//! `ogd-worker/1` wire types, transcribed from `backend/worker.py` and
//! `backend/worker-request.schema.json`. These are *not* domain types; the
//! adapter converts field by field (SPEC §12).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const PROTOCOL: &str = "ogd-worker/1";
pub const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Route {
    #[serde(rename = "verdant-voice")]
    VerdantVoice,
    /// Reserved for the explicitly unlocked, hash-verified original source pair.
    C0,
}

impl Route {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::VerdantVoice => "verdant-voice",
            Self::C0 => "原版声库",
        }
    }

    /// Stable route identifier used by existing projects and the worker wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Route::VerdantVoice => "verdant-voice",
            Route::C0 => "C0",
        }
    }
    /// Parse normal render selections. The original-source exception requires
    /// an explicit reserved singer and is resolved by the project adapter.
    pub fn parse(s: &str) -> Option<Route> {
        match s {
            "verdant-voice" => Some(Route::VerdantVoice),
            _ => None,
        }
    }
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Capabilities,
    Render,
    Cancel,
    Shutdown,
    #[serde(rename = "admit_voice")]
    AdmitVoice,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Request {
    pub protocol: &'static str,
    pub request_id: String,
    pub op: Op,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<Route>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<crate::voices::VoiceAssets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_evidence: Option<crate::voices::SourceEvidence>,
}

impl Request {
    pub fn capabilities(request_id: impl Into<String>) -> Self {
        Request {
            protocol: PROTOCOL,
            request_id: request_id.into(),
            op: Op::Capabilities,
            voice_ref: None, reference: None, display_name: None, assets: None, source_evidence: None,
            job_id: None,
            revision: None,
            route: None,
            payload: None,
        }
    }
    pub fn render(request_id: impl Into<String>, job_id: impl Into<String>, revision: u64, route: Route, payload: Payload) -> Self {
        Request {
            protocol: PROTOCOL,
            request_id: request_id.into(),
            op: Op::Render,
            voice_ref: None, reference: None, display_name: None, assets: None, source_evidence: None,
            job_id: Some(job_id.into()),
            revision: Some(revision),
            route: Some(route),
            payload: Some(payload),
        }
    }
    pub fn cancel(request_id: impl Into<String>, job_id: impl Into<String>) -> Self {
        Request {
            protocol: PROTOCOL,
            request_id: request_id.into(),
            op: Op::Cancel,
            voice_ref: None, reference: None, display_name: None, assets: None, source_evidence: None,
            job_id: Some(job_id.into()),
            revision: None,
            route: None,
            payload: None,
        }
    }
    pub fn shutdown(request_id: impl Into<String>) -> Self {
        Request {
            protocol: PROTOCOL,
            request_id: request_id.into(),
            op: Op::Shutdown,
            voice_ref: None, reference: None, display_name: None, assets: None, source_evidence: None,
            job_id: None,
            revision: None,
            route: None,
            payload: None,
        }
    }

    pub fn admit_voice(request_id: impl Into<String>, reference: String, display_name: String, assets: crate::voices::VoiceAssets) -> Self {
        let mut request = Self::capabilities(request_id);
        request.op = Op::AdmitVoice;
        request.reference = Some(reference); request.display_name = Some(display_name); request.assets = Some(assets);
        request
    }
    pub fn admit_voice_package(request_id: impl Into<String>, reference: String, display_name: String,
        assets: crate::voices::VoiceAssets, source_evidence: crate::voices::SourceEvidence) -> Self {
        let mut request = Self::admit_voice(request_id, reference, display_name, assets);
        request.source_evidence = Some(source_evidence);
        request
    }

    /// One newline-terminated line, refusing anything over 8 MiB.
    pub fn to_line(&self) -> Result<String, String> {
        let mut s = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if s.len() + 1 > MAX_REQUEST_BYTES {
            return Err(format!("request is {} bytes; ogd-worker/1 accepts at most 8 MiB", s.len()));
        }
        s.push('\n');
        Ok(s)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Payload {
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<Arc<NativeScore>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_seed: Option<u32>,
    /// Explicit verdant-voice output profile. Omission preserves native 48 kHz
    /// output; this profile never changes the condition/controller ABI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_music: Option<VoiceMusic>,
    /// Optional Native v10 bandwidth extension before the selected output route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wideband: Option<Wideband>,
    /// Independent envelope transform for an unlocked original source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocal_tract: Option<VocalTract>,
    /// Opt-in voiced-vowel attenuation for an unlocked original source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_dynamics: Option<ModelDynamics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consonant_dynamics: Option<ConsonantDynamics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<Arc<crate::expression::ExpressionTrack>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_transients: Option<ConsonantDynamics>,
    /// Per-frame F0/UV, capability-gated by the phrase planner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_track: Option<Arc<crate::f0::ControlTrack>>,
}

pub const VOICE_MUSIC_METHOD: &str = "verdant-voice-music-v1";
pub const VOICE_MUSIC_SAMPLE_RATES: [u32; 2] = [44_100, 48_000];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceMusic {
    pub method: String,
    pub sample_rate: u32,
}

impl VoiceMusic {
    pub fn validate(&self) -> Result<(), String> {
        if self.method != VOICE_MUSIC_METHOD {
            return Err("Unsupported Verdant ONNX music output method".into());
        }
        if !VOICE_MUSIC_SAMPLE_RATES.contains(&self.sample_rate) {
            return Err("Verdant ONNX music output requires 44100 or 48000 Hz".into());
        }
        Ok(())
    }
}

/// The nested music declaration selects the output clock after native 48 kHz decoding.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VoiceMusicCaps {
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub sample_rates: Vec<u32>,
    #[serde(default)]
    pub default_sample_rate: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeScore {
    pub title: String,
    pub segments: Vec<Segment>,
    /// Explicit original hierarchy annotations, never inferred from note adjacency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hierarchy: Option<OriginalHierarchy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<NativeSynthesis>,
    /// Auditable, materialized note/phone anchors on the phrase-relative clock.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phoneme_timing: Option<PhonemeTimingTrace>,
    /// Explicit capability-gated whole-run execution; legacy requests omit it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuity: Option<ContinuousRuns>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ContinuousRuns {
    pub method: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct PhonemeTimingTrace {
    pub method: String,
    pub notes: Vec<PhonemeTimingNote>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct PhonemeTimingNote {
    pub note_id: String,
    pub note_onset_seconds: f64,
    pub note_end_seconds: f64,
    pub first_segment_index: usize,
    /// First actual native nucleus, or null when every segment is consonantal.
    #[serde(deserialize_with = "Option::deserialize")]
    pub nucleus_segment_index: Option<usize>,
    /// Time from the authored beat to that nucleus when the full lead cannot fit.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub nucleus_delay_seconds: f64,
    /// Exclusive segment index.
    pub end_segment_index: usize,
    /// Actual lead before the authored beat. Consonants can continue after it.
    pub prefix_seconds: f64,
    pub tail_trim_seconds: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeSynthesis {
    pub vowel_balance_db: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonetic_correction: Option<PhoneticCorrection>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PhoneticCorrection {
    pub method: String,
    pub strength: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wideband {
    pub method: String,
    pub presence_db: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VocalTract {
    pub method: String,
    pub ratio: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_ratios: Option<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelDynamics {
    pub method: String,
    pub strength: f64,
    pub max_attenuation_db: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConsonantDynamics {
    pub method: String,
    pub strength: f64,
    pub max_attenuation_db: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OriginalHierarchy {
    pub schema_version: String,
    pub nodes: Vec<OriginalHierarchyNode>,
    pub segment_node_indices: Vec<usize>,
    pub provenance: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OriginalHierarchyNode {
    pub phone: u16,
    pub left_level: i32,
    pub right_level: i32,
}

/// The documented native0.2 segment subset this frontend emits.
///
/// Two verified forms exist (worker validation is duration/gap/f0/voiced
/// only; `render.py::context_for_segment` picks `context` first):
/// * **english-proxy**: `vowel` + `context_mode = "english-proxy"` + `f0_hz`.
/// * **pinyin**: explicit `context` object (recovered `make-pinyin-score`
///   contract) + `midi_note`; `vowel`/`context_mode` are absent.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Segment {
    pub label: String,
    /// Original voiced nucleus index; a chain retains this anchor across distinct note segments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_of: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vowel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_mode: Option<String>,
    /// Explicit recovered context object (pinyin initial/final segments).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ScoreContext>,
    pub duration_seconds: f64,
    #[serde(skip_serializing_if = "is_zero")]
    pub gap_after_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f0_hz: Option<f64>,
    /// Recovered helper pitch field (fractional = note + detune in
    /// semitones); the renderer maps it as `440 * 2**((m - 69) / 12)`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_note: Option<f64>,
    pub voiced: bool,
    /// Preserved helper annotation on obstruent-initial segments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excitation_note: Option<String>,
    /// Target notation and actual model mapping survive in render artifacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phoneme: Option<PhonemeTrace>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PhonemeTrace {
    pub canonical_ipa: String,
    pub input_symbol: String,
    pub input_alphabet: String,
    pub source_language: String,
    /// Authored note ownership for trace-constrained verdant-voice articulation.
    /// This is frontend provenance, not an acoustic condition or identity proof.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_note_id: Option<String>,
    pub source_index: usize,
    pub syllable_index: usize,
    pub stress: Option<u8>,
    pub length: u32,
    pub native_key: String,
    pub support: String,
    pub approximation_reason: Option<String>,
    /// Contextual kana realization; canonical IPA remains the source request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realization: Option<crate::phonemes::PhonemeRealization>,
}

/// Explicit context object, mirroring `render.py::Context` field names.
/// `features` are string-valued, `answers` boolean-valued; unknown questions
/// stay unknown — the renderer completes them diagnostically.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ScoreContext {
    pub label: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub features: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub answers: BTreeMap<String, bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchor_questions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<String>,
    /// Renderer-internal completion state; emitted empty by the helper.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub diagnostic_answers: BTreeMap<String, bool>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

// ---- events ---------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Artifact {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkerEvent {
    Ready {
        session_id: String,
        workspace: String,
    },
    Capabilities {
        request_id: Option<String>,
        capabilities: Box<Capabilities>,
    },
    VoiceAdmitted {
        request_id: Option<String>,
        reference: String,
        display_name: String,
        cache_identity: String,
        source_sha256: BTreeMap<String,String>,
        historical_identity_confirmed: bool,
    },
    Accepted {
        request_id: Option<String>,
        job_id: String,
        revision: u64,
        job_directory: Option<String>,
    },
    Progress {
        job_id: String,
        stage: Option<String>,
        phase: Option<String>,
        fraction: Option<f64>,
    },
    Completed {
        job_id: String,
        revision: u64,
        artifacts: Vec<Artifact>,
        identity_status: Option<String>,
        elapsed_seconds: Option<f64>,
    },
    Failed {
        job_id: String,
        revision: u64,
        code: String,
        message: String,
    },
    Cancelled {
        job_id: String,
        revision: u64,
        partial_outputs_valid: bool,
    },
    Rejected {
        request_id: Option<String>,
        job_id: Option<String>,
        code: String,
        message: String,
    },
    CancelAck {
        request_id: Option<String>,
        job_id: String,
        active_match: bool,
        previous_terminal: Option<String>,
    },
    Shutdown {
        request_id: Option<String>,
    },
    /// Well-formed record with an event name this client does not know.
    Unknown {
        event: String,
        raw: Value,
    },
}

impl WorkerEvent {
    pub fn job_id(&self) -> Option<&str> {
        match self {
            WorkerEvent::Accepted { job_id, .. }
            | WorkerEvent::Progress { job_id, .. }
            | WorkerEvent::Completed { job_id, .. }
            | WorkerEvent::Failed { job_id, .. }
            | WorkerEvent::Cancelled { job_id, .. }
            | WorkerEvent::CancelAck { job_id, .. } => Some(job_id),
            WorkerEvent::Rejected { job_id, .. } => job_id.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("not JSON: {0}")]
    Json(String),
    #[error("wrong protocol {0:?}")]
    Protocol(Option<String>),
    #[error("event {event}: missing or invalid field {field}")]
    Field { event: String, field: &'static str },
}

pub fn parse_event(line: &str) -> Result<WorkerEvent, ParseError> {
    let v: Value = serde_json::from_str(line).map_err(|e| ParseError::Json(e.to_string()))?;
    parse_event_value(v)
}

pub fn parse_event_value(v: Value) -> Result<WorkerEvent, ParseError> {
    let o = v.as_object().ok_or_else(|| ParseError::Json("record is not an object".into()))?;
    let protocol = o.get("protocol").and_then(Value::as_str);
    if protocol != Some(PROTOCOL) {
        return Err(ParseError::Protocol(protocol.map(str::to_string)));
    }
    let event = o.get("event").and_then(Value::as_str).unwrap_or_default().to_string();
    let s = |k: &str| o.get(k).and_then(Value::as_str).map(str::to_string);
    let need_s = |k: &'static str| s(k).ok_or(ParseError::Field { event: event.clone(), field: k });
    let need_u = |k: &'static str| o.get(k).and_then(Value::as_u64).ok_or(ParseError::Field { event: event.clone(), field: k });
    Ok(match event.as_str() {
        "ready" => WorkerEvent::Ready { session_id: need_s("session_id")?, workspace: need_s("workspace")? },
        "capabilities" => {
            let caps = o.get("capabilities").cloned().ok_or(ParseError::Field { event: event.clone(), field: "capabilities" })?;
            let caps: Capabilities =
                serde_json::from_value(caps).map_err(|_| ParseError::Field { event: event.clone(), field: "capabilities" })?;
            WorkerEvent::Capabilities { request_id: s("request_id"), capabilities: Box::new(caps) }
        }
        "accepted" => WorkerEvent::Accepted {
            request_id: s("request_id"),
            job_id: need_s("job_id")?,
            revision: need_u("revision")?,
            job_directory: s("job_directory"),
        },
        "voice_admitted" => WorkerEvent::VoiceAdmitted {
            request_id:s("request_id"),reference:need_s("reference")?,display_name:need_s("display_name")?,cache_identity:need_s("cache_identity")?,
            source_sha256:serde_json::from_value(o.get("source_sha256").cloned().ok_or_else(||ParseError::Field{event:event.clone(),field:"source_sha256"})?).map_err(|_|ParseError::Field{event:event.clone(),field:"source_sha256"})?,
            historical_identity_confirmed:o.get("historical_identity_confirmed").and_then(Value::as_bool).ok_or_else(||ParseError::Field{event:event.clone(),field:"historical_identity_confirmed"})?,
        },
        "progress" => WorkerEvent::Progress {
            job_id: need_s("job_id")?,
            stage: s("stage"),
            phase: s("phase"),
            fraction: o.get("fraction").and_then(Value::as_f64),
        },
        "completed" => {
            let artifacts = o.get("artifacts").cloned().ok_or(ParseError::Field { event: event.clone(), field: "artifacts" })?;
            WorkerEvent::Completed {
                job_id: need_s("job_id")?,
                revision: need_u("revision")?,
                artifacts: serde_json::from_value(artifacts).map_err(|_| ParseError::Field { event: event.clone(), field: "artifacts" })?,
                identity_status: s("identity_status"),
                elapsed_seconds: o.get("elapsed_seconds").and_then(Value::as_f64),
            }
        }
        "failed" => WorkerEvent::Failed {
            job_id: need_s("job_id")?,
            revision: need_u("revision")?,
            code: s("code").unwrap_or_else(|| "RENDER_FAILED".into()),
            message: s("message").unwrap_or_default(),
        },
        "cancelled" => WorkerEvent::Cancelled {
            job_id: need_s("job_id")?,
            revision: need_u("revision")?,
            partial_outputs_valid: o.get("partial_outputs_valid").and_then(Value::as_bool).unwrap_or(false),
        },
        "rejected" => WorkerEvent::Rejected {
            request_id: s("request_id"),
            job_id: s("job_id"),
            code: s("code").unwrap_or_default(),
            message: s("message").unwrap_or_default(),
        },
        "cancel_ack" => WorkerEvent::CancelAck {
            request_id: s("request_id"),
            job_id: need_s("job_id")?,
            active_match: o.get("active_match").and_then(Value::as_bool).unwrap_or(false),
            previous_terminal: s("previous_terminal"),
        },
        "shutdown" => WorkerEvent::Shutdown { request_id: s("request_id") },
        _ => WorkerEvent::Unknown { event, raw: v },
    })
}

// ---- capabilities ------------------------------------------------------------------
// Every field is optional/defaulted: an unknown or missing capability is
// treated as *unsupported* (SPEC §12).

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Capabilities {
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub native_version: Option<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default = "one")]
    pub max_active_jobs: u32,
    #[serde(default)]
    pub max_phrase_seconds: f64,
    #[serde(default)]
    pub dependency_mode: Option<String>,
    #[serde(default)]
    pub native_score: Option<NativeScoreCaps>,
    #[serde(default)]
    pub candidate_voice: Option<CandidateVoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voices: Option<VoiceCatalog>,
    #[serde(default)]
    pub files: BTreeMap<String, FileStatus>,
    /// Set only by the protocol test double so the UI can never present it
    /// as a voice.
    #[serde(default)]
    pub test_double: bool,
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

fn one() -> u32 {
    1
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct NativeScoreCaps {
    #[serde(default)]
    pub inventory: Vec<String>,
    #[serde(default)]
    pub english_vowel_proxies: Vec<String>,
    #[serde(default)]
    pub explicit_f0_hz_range: Option<[f64; 2]>,
    #[serde(default)]
    pub continuous_f0_control: bool,
    #[serde(default)]
    pub coarticulation_recovered: bool,
    #[serde(default)]
    pub vowel_balance_db_range: Option<[f64; 2]>,
    #[serde(default)]
    pub wideband: Option<WidebandCaps>,
    #[serde(default)]
    pub phonetic_correction: Option<PhoneticCorrectionCaps>,
    #[serde(default)]
    pub vocal_tract: Option<VocalTractCaps>,
    #[serde(default)]
    pub model_dynamics: Option<ModelDynamicsCaps>,
    #[serde(default)]
    pub phoneme_timing: Option<PhonemeTimingCaps>,
    #[serde(default)]
    pub consonant_dynamics: Option<ConsonantDynamicsCaps>,
    #[serde(default)]
    pub expression: Option<ExpressionCaps>,
    #[serde(default)]
    pub output_transients: Option<ConsonantDynamicsCaps>,
    /// Worker-declared sub-syllable (initial/final) timing support per the
    /// recovered `make-pinyin-score` contract. `None` means "not declared":
    /// the adapter derives it from a non-empty recovered `inventory`.
    #[serde(default)]
    pub sub_syllable_timing: Option<bool>,
    /// Explicit segment-duration overrides, distinct from a fixed recovered split.
    #[serde(default)]
    pub phoneme_duration_control: bool,
    /// The worker renders one 5 ms acoustic frame per native phone state, so
    /// a phone may be as short as one compact frame instead of the recovered
    /// 25 ms minimum segment. Only the Verdant ONNX route admits it.
    #[serde(default)]
    pub compact_phone_states: bool,
    #[serde(default)]
    pub phoneme_frontend: Option<PhonemeFrontendCaps>,
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ExpressionCaps {
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub maximum_compiled_curve_points: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct WidebandCaps {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub presence_db_range: Option<[f64; 2]>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PhoneticCorrectionCaps {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub strength_range: Option<[f64; 2]>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct VocalTractCaps {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub ratio_range: Option<[f64; 2]>,
    #[serde(default)]
    pub anchor_ratio_range: Option<[f64; 2]>,
    #[serde(default)]
    pub anchor_fractions: Option<[f64; 2]>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ModelDynamicsCaps {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub minimum_strength: Option<f64>,
    #[serde(default)]
    pub maximum_strength: Option<f64>,
    #[serde(default)]
    pub minimum_attenuation_db: Option<f64>,
    #[serde(default)]
    pub maximum_attenuation_db: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PhonemeTimingCaps {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub timing_resolution_seconds: Option<f64>,
    #[serde(default)]
    pub continuity: Option<ContinuousRunsCaps>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ContinuousRunsCaps {
    pub method: String,
    pub max_seconds: f64,
    pub max_segments: usize,
    pub max_control_points: usize,
    pub minimum_silence_seconds: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ConsonantDynamicsCaps {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub minimum_strength: Option<f64>,
    #[serde(default)]
    pub maximum_strength: Option<f64>,
    #[serde(default)]
    pub minimum_attenuation_db: Option<f64>,
    #[serde(default)]
    pub maximum_attenuation_db: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PhonemeFrontendCaps {
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub alphabets: Vec<String>,
    #[serde(default)]
    pub languages: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct CandidateVoice {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub vid: Option<u64>,
    #[serde(default)]
    pub historical_identity_confirmed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct FileStatus {
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct VoiceCatalog {
    pub version: u32,
    #[serde(default)]
    pub default_voice: Option<String>,
    #[serde(default)]
    pub voices: BTreeMap<String, VoiceEntry>,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct VoiceEntry {
    #[serde(default)] pub display_name: String,
    #[serde(default)] pub available: bool,
    #[serde(default)] pub cache_identity: String,
    #[serde(default)] pub source_sha256: BTreeMap<String, String>,
    #[serde(default)] pub languages: BTreeMap<String, String>,
    #[serde(default)] pub historical_identity_confirmed: bool,
    #[serde(default)] pub files: BTreeMap<String, FileStatus>,
    #[serde(default)] pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub voice_package: Option<crate::voices::VoicePackage>,
}

impl Capabilities {
    /// Enable the built-in music output only on an explicit matching worker
    /// declaration. Old/unknown capabilities keep the byte-compatible request
    /// without this field; mapping support alone never opts into processing.
    pub fn default_voice_music(&self) -> Option<VoiceMusic> {
        let declaration: VoiceMusicCaps = serde_json::from_value(self.other.get("voice_package")?.get("music")?.clone()).ok()?;
        if !declaration.routes.iter().any(|route| route == "verdant-voice")
            || !declaration.methods.iter().any(|method| method == VOICE_MUSIC_METHOD)
            || !declaration.sample_rates.contains(&48_000)
            || declaration.default_sample_rate != Some(48_000) {
            return None;
        }
        Some(VoiceMusic { method: VOICE_MUSIC_METHOD.into(), sample_rate: 48_000 })
    }

    /// Resolve only an admitted catalog entry and enforce the document's source pins.
    pub fn voice_for_track<'a>(&'a self, project: &verdant_domain::Project, track: &verdant_domain::Track)
        -> Result<Option<(&'a str, &'a VoiceEntry)>, String> {
        let Some(catalog) = &self.voices else {
            if track.unknown.contains_key("voice_selection"){return Err("轨道已绑定的歌手目录不可用；请恢复已校验的歌手来源".into());}
            if track.voice_ref.as_deref().is_some_and(|reference| reference != "voice-sn") {
                return Err("当前引擎未提供该歌手目录；请安装歌手或选择已配置的默认歌手".into());
            }
            return Ok(None);
        };
        if catalog.version != 1 { return Err("不支持的歌手目录版本".into()); }
        let legacy_cli=match track.voice_ref.as_deref(){
            Some("voice-sn")=>!catalog.voices.contains_key("voice-sn"),
            None=>catalog.default_voice.is_none(),
            _=>false,
        };
        if legacy_cli{
            if track.unknown.contains_key("voice_selection"){return Err("轨道已绑定的歌手尚未安装；请恢复已校验的歌手来源".into());}
            return Ok(None);
        }
        let reference = track.voice_ref.as_deref().or(catalog.default_voice.as_deref()).ok_or("未选择歌手")?;
        let (reference, voice) = catalog.voices.get_key_value(reference).ok_or_else(|| format!("歌手 {reference} 尚未安装"))?;
        if !voice.available || voice.cache_identity.is_empty() { return Err(format!("歌手 {reference} 不可用：{}",voice.error.as_deref().unwrap_or("源文件未通过校验"))); }
        if !crate::voices::voice_reference_matches_package(reference,voice) { return Err("歌手引用与已准入的声库版本不符".into()); }
        if let Some(expected) = project.voice_refs.get(reference) {
            for (role, original, expected) in [("token","sn16k.tkn",expected.token_sha256.as_deref()),("model","sn16k.bin",expected.model_sha256.as_deref())] {
                if let Some(expected) = expected {
                    if !voice.source_sha256.get(original).or_else(||voice.source_sha256.get(role)).is_some_and(|actual|actual.eq_ignore_ascii_case(expected)) {
                        return Err(format!("歌手 {reference} 的 {role} 源指纹与工程不符"));
                    }
                }
            }
            if expected.historical_identity_confirmed && !voice.historical_identity_confirmed { return Err("所选源未建立工程要求的歌手身份".into()); }
        }
        if let Some(pin)=track.unknown.get("voice_selection") {
            if pin.get("schema").and_then(Value::as_str)!=Some("verdant-track-voice/1")
                || pin.get("reference").and_then(Value::as_str)!=Some(reference.as_str())
                || pin.get("cache_identity").and_then(Value::as_str)!=Some(voice.cache_identity.as_str()) {
                return Err("轨道歌手来源已改变；请重新选择并确认已安装的歌手".into());
            }
            let expected:BTreeMap<String,String>=serde_json::from_value(pin.get("source_sha256").cloned().ok_or("轨道歌手缺少源指纹")?).map_err(|e|format!("轨道歌手指纹无效：{e}"))?;
            if expected!=voice.source_sha256{return Err("轨道歌手源文件指纹与已安装源不符".into());}
        }
        Ok(Some((reference,voice)))
    }

    pub fn for_voice(&self, voice: &VoiceEntry) -> Self {
        let mut caps = self.clone(); caps.files = voice.files.clone();
        // Shared runtime fingerprints remain part of each selected bank's
        // admission and cache identity.
        for(key,value)in &self.files{
            if matches!(key.as_str(),"onnxruntime"|"worker_binary")||key.starts_with("synthesis:"){
                caps.files.entry(key.clone()).or_insert_with(||value.clone());
            }
        }
        caps.files.remove("vocoder");
        caps.routes.retain(|route| route == "verdant-voice"
            || route == "C0" && crate::voices::is_desktop_original(voice));
        caps.other.remove("neural");
        caps.other.remove("vocoder");
        let current=voice.voice_package.is_some() || voice.files.contains_key("verdant_voice_manifest");
        if current {
            caps.other.insert("voice_source".into(),Value::String("verdant-voice".into()));
            if let Some(declaration)=caps.other.entry("voice_package".to_owned()).or_insert_with(||Value::Object(Map::new())).as_object_mut() {
                declaration.insert("display_name".into(),voice.voice_package.as_ref()
                    .map_or_else(||Value::String("verdant-voice".into()),|package|format!("{} {}",package.name,package.version).into()));
                declaration.insert("name".into(),"verdant-voice".into());
                declaration.insert("voice_version".into(),voice.voice_package.as_ref().map_or(Value::Null,|package|package.version.clone().into()));
                declaration.insert("sample_rate".into(),48000.into());
                declaration.insert("hop_samples".into(),240.into());
                declaration.insert("authored_f0_hz_range".into(),serde_json::json!([35.,2000.]));
                declaration.insert("authored_f0_policy".into(),"native-target-reference-v1".into());
                declaration.insert("mainline_decoder_candidate".into(),"decoder-48k".into());
            }
        } else if voice.files.contains_key("voice_json") {
            caps.other.insert("voice_source".into(),Value::String("modern".into()));
            caps.other.remove("voice_package");
        } else {
            caps.other.remove("voice_source");
            caps.other.remove("voice_package");
        }
        caps
    }

    pub fn supports_route(&self, r: Route) -> bool {
        self.routes.iter().any(|x| x == r.as_str())
    }

    /// Files for the selected voice source; modern availability includes validation.
    pub fn missing_files(&self, r: Route) -> Vec<&'static str> {
        let need: &[&'static str] = match r {
            Route::VerdantVoice => &["onnxruntime"],
            Route::C0 => &["token", "model"],
        };
        if self.test_double {
            return Vec::new();
        }
        let mut missing:Vec<_>=need.iter().copied().filter(|k| !self.files.get(*k).is_some_and(|f| f.available)).collect();
        if r == Route::VerdantVoice {
            missing.extend(crate::voices::PACKAGE_FILES.iter().map(|(key,_)|*key)
                .filter(|key|!self.files.get(*key).is_some_and(|file|file.available&&file.sha256.is_some())));
            let manifest_hash = self.files.get("verdant_voice_manifest").and_then(|file| file.sha256.as_ref());
            if !self.files.get("voice_source_admission").is_some_and(|receipt|
                receipt.available && manifest_hash.is_some() && receipt.sha256.as_ref() == manifest_hash) {
                missing.push("voice_source_admission");
            }
            let declaration=self.other.get("voice_package");
            if !declaration.is_some_and(|value|value["name"]=="verdant-voice"
                && value["voice_version"].as_str().is_some_and(crate::voices::valid_voice_version)) {
                missing.push("voice_package_version");
            }
        }
        missing
    }
}

#[cfg(test)]
mod voice_tests {
    use super::*;
    use serde_json::json;

    fn capabilities() -> Capabilities {
        serde_json::from_value(json!({
            "native_score":{"explicit_f0_hz_range":[35.,2000.]},
            "voice_package":{"name":"verdant-voice","authored_f0_policy":"native-target-reference-v1"},
            "files":{"onnxruntime":{"available":true,"sha256":"runtime"},
                "worker_binary":{"available":true,"sha256":"worker"}}
        })).unwrap()
    }

    fn canonical_voice() -> VoiceEntry {
        let mut voice = VoiceEntry { available:true, cache_identity:"versioned voice".into(),
            voice_package:Some(crate::voices::VoicePackage { name:"verdant-voice".into(), version:"0.1.0".into() }),
            ..Default::default() };
        for (role, _) in crate::voices::PACKAGE_FILES {
            voice.files.insert((*role).into(), FileStatus { available:true, sha256:Some("a".repeat(64)) });
        }
        voice.files.insert("voice_source_admission".into(), FileStatus { available:true, sha256:Some("a".repeat(64)) });
        voice
    }

    #[test]
    fn canonical_projection_retains_version_and_requires_every_canonical_role() {
        let voice=canonical_voice();let selected=capabilities().for_voice(&voice);
        assert_eq!(selected.other["voice_package"]["display_name"],"verdant-voice 0.1.0");
        assert_eq!(selected.other["voice_package"]["voice_version"],"0.1.0");
        assert_eq!(crate::adapter::authored_f0_hz_range(Route::VerdantVoice,&selected),[35.,2000.]);
        assert!(selected.missing_files(Route::VerdantVoice).is_empty());
        assert!(crate::adapter::is_desktop_voice(&voice));
        for (role,_) in crate::voices::PACKAGE_FILES {
            let mut incomplete=voice.clone();incomplete.files.remove(*role);
            assert!(capabilities().for_voice(&incomplete).missing_files(Route::VerdantVoice).contains(role),"{role}");
        }
        let mut no_version=voice.clone();no_version.voice_package=None;
        assert!(capabilities().for_voice(&no_version).missing_files(Route::VerdantVoice).contains(&"voice_package_version"));
        assert!(!crate::adapter::is_desktop_voice(&no_version));
        let raw=selected.for_voice(&VoiceEntry::default());
        assert!(!raw.other.contains_key("voice_package"));
        assert!(!raw.files.contains_key("verdant_voice_manifest"));
        let mut wrong_receipt = voice;
        wrong_receipt.files.get_mut("voice_source_admission").unwrap().sha256 = Some("b".repeat(64));
        assert!(!crate::adapter::is_desktop_voice(&wrong_receipt));
        assert!(capabilities().for_voice(&wrong_receipt).missing_files(Route::VerdantVoice).contains(&"voice_source_admission"));
    }

    #[test]
    fn canonical_explicit_pin_never_falls_back_when_unavailable_or_relabelled() {
        let mut caps=capabilities();let voice=canonical_voice();let reference="verdant-voice-0.1.0";
        caps.voices=Some(VoiceCatalog{version:1,default_voice:Some("other".into()),
            voices:BTreeMap::from([(reference.into(),voice.clone()),("other".into(),canonical_voice())])});
        let mut project=verdant_domain::Project::new_empty();project.tracks[0].voice_ref=Some(reference.into());
        let track=&project.tracks[0];
        assert_eq!(crate::adapter::desktop_voice(&project,track,&caps).unwrap().unwrap().0,reference);
        caps.voices.as_mut().unwrap().voices.get_mut(reference).unwrap().available=false;
        assert!(crate::adapter::desktop_voice(&project,track,&caps).is_err());
        caps.voices.as_mut().unwrap().voices.insert(reference.into(),voice.clone());
        caps.voices.as_mut().unwrap().voices.get_mut(reference).unwrap().voice_package.as_mut().unwrap().version="0.1.1".into();
        assert!(caps.voice_for_track(&project,track).is_err());
        assert!(crate::adapter::desktop_voice(&project,track,&caps).is_err());
        caps.voices.as_mut().unwrap().voices.remove(reference);
        assert!(crate::adapter::desktop_voice(&project,track,&caps).is_err());
        caps.voices=None;
        assert!(crate::adapter::desktop_voice(&project,track,&caps).is_err());
    }
}
