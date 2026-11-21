//! Line-delimited JSON control protocol between Verdant (client) and
//! `verdant-plughost` (host). One JSON object per line on each pipe; no binary
//! frames on this channel (audio lives in shared memory).
//!
//! The protocol version is negotiated by `Hello`/`hello_ack`; a host that
//! answers something else is rejected before any audio state exists.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const PROTOCOL_VERSION: u32 = 4;
/// The host announces itself with this string.
pub const HOST_NAME: &str = "verdant-plughost";

/// Shared parameter description. VST3 uses normalized values; CLAP/AU use
/// their declared plain range. This type has no device or client dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDesc {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub step_count: i64,
    /// Native default reported by the host, in this parameter's own units.
    /// Absent when the plugin format or the host does not expose one.
    #[serde(default, rename = "default", skip_serializing_if = "Option::is_none")]
    pub default_value: Option<f64>,
}

/// Prepared native parameter event: `(native_id, sample_offset, native_value)`.
/// Offsets are relative to the beginning of the processing call, not device time.
/// Keep these in caller-owned storage; construction and processing never use IPC.
pub type ParamEvent = (u32, u32, f64);

/// Maximum admitted events in one native processing quantum. Overflow rejects
/// the entire call before any of its audio or events can be published.
pub const MAX_PARAM_EVENTS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AutomationError {
    #[error("invalid or non-finite parameter descriptor")]
    InvalidDescriptor,
    #[error("parameter value is non-finite or outside its native bounds")]
    ValueOutOfRange,
    #[error("parameter value does not select a native discrete step")]
    InvalidStep,
    #[error("parameter events exceed the processing quantum capacity")]
    EventCapacity,
    #[error("parameter event is outside the processing call")]
    InvalidOffset,
    #[error("parameter events must be ordered by sample offset")]
    UnsortedEvents,
    #[error("parameter was not admitted for native automation")]
    UnknownParameter,
    #[error("parameter event transport is unavailable")]
    TransportUnavailable,
}

/// A control-thread resolution of one writable, automatable parameter.
/// Native values are normalized for VST3, and plain for CLAP/AUv2/builtin.
/// Recompile after a parameter rescan or loading a different plugin state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParamTarget {
    native_id: u32,
    min: f64,
    max: f64,
    default_value: f64,
    step_count: i64,
}

impl ParamTarget {
    pub fn from_desc(native_id: u32, desc: &ParamDesc) -> Result<Self, AutomationError> {
        let target = Self { native_id, min: desc.min, max: desc.max,
            default_value: desc.value, step_count: desc.step_count };
        target.validate()?;
        Ok(target)
    }

    pub fn validate(&self) -> Result<(), AutomationError> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min > self.max
            || !(self.max - self.min).is_finite() || !self.default_value.is_finite()
            || !(self.min..=self.max).contains(&self.default_value) || self.step_count < 0 {
            return Err(AutomationError::InvalidDescriptor);
        }
        self.event(0, self.default_value)?;
        Ok(())
    }

    pub fn native_id(&self) -> u32 { self.native_id }
    pub fn min(&self) -> f64 { self.min }
    pub fn max(&self) -> f64 { self.max }
    /// Current native value captured during control-side compilation.
    pub fn default_value(&self) -> f64 { self.default_value }
    pub fn step_count(&self) -> i64 { self.step_count }

    pub fn event(&self, sample_offset: u32, value: f64) -> Result<ParamEvent, AutomationError> {
        if !value.is_finite() || !(self.min..=self.max).contains(&value) {
            return Err(AutomationError::ValueOutOfRange);
        }
        if self.step_count > 0 && self.max > self.min {
            let step = (value - self.min) / (self.max - self.min) * self.step_count as f64;
            let tolerance = 8.0 * f64::EPSILON * step.abs().max(1.0);
            if (step - step.round()).abs() > tolerance {
                return Err(AutomationError::InvalidStep);
            }
        }
        Ok((self.native_id, sample_offset, value))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationParam {
    /// Same persistent string id returned by `get_params`.
    pub id: String,
    pub target: ParamTarget,
}

/// Validate the whole call, including events already accumulated in a partial
/// quantum. This bounded pass makes capacity failures atomic with respect to
/// publication. No allocation, sorting, locking or parameter IPC occurs here.
/// Note events travel in the same per-quantum event list as parameter
/// points: the top bit of the id marks a note, bit 8 marks note-on and the
/// low seven bits are the MIDI key; the value is the velocity (0…1). Only
/// instrument instances accept them.
pub const NOTE_EVENT: u32 = 1 << 31;
const NOTE_ON: u32 = 1 << 8;

/// A decoded note event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteEvent { On { key: u8, velocity: f64 }, Off { key: u8 } }

pub fn note_on(key: u8, offset: u32, velocity: f64) -> ParamEvent { (NOTE_EVENT | NOTE_ON | (key & 0x7f) as u32, offset, velocity.clamp(0.0, 1.0)) }
pub fn note_off(key: u8, offset: u32) -> ParamEvent { (NOTE_EVENT | (key & 0x7f) as u32, offset, 0.0) }

/// `None` for a parameter event.
pub fn decode_note(id: u32, value: f64) -> Option<NoteEvent> {
    if id & NOTE_EVENT == 0 { return None; }
    let key = (id & 0x7f) as u8;
    Some(if id & NOTE_ON != 0 { NoteEvent::On { key, velocity: value } } else { NoteEvent::Off { key } })
}

pub(crate) fn validate_event_quanta(events: &[ParamEvent], frames: usize,
    quantum: usize, carry_frames: usize, carry_events: usize) -> Result<(), AutomationError> {
    if quantum == 0 || carry_frames >= quantum || carry_events > MAX_PARAM_EVENTS {
        return Err(AutomationError::InvalidOffset);
    }
    let quanta = frames.saturating_add(carry_frames).div_ceil(quantum).max(1);
    if events.len() > quanta.saturating_mul(MAX_PARAM_EVENTS) {
        return Err(AutomationError::EventCapacity);
    }
    let mut bucket = 0;
    let mut count = carry_events;
    let mut previous = 0;
    for &(.., offset, value) in events {
        let offset = offset as usize;
        if offset >= frames { return Err(AutomationError::InvalidOffset); }
        if offset < previous { return Err(AutomationError::UnsortedEvents); }
        if !value.is_finite() { return Err(AutomationError::ValueOutOfRange); }
        previous = offset;
        let next = (carry_frames + offset) / quantum;
        if next != bucket { bucket = next; count = 0; }
        count += 1;
        if count > MAX_PARAM_EVENTS { return Err(AutomationError::EventCapacity); }
    }
    Ok(())
}

/// App → host requests. `id` is caller-assigned and echoed in the reply.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum HostRequest {
    /// First message on the control pipe; carries nothing else.
    Hello { id: u64 },
    /// Instantiate (or re-instantiate) a plugin.
    Instantiate {
        id: u64,
        format: String,
        /// Plugin id inside the bundle (VST3 class id; empty → first audio class).
        plugin_id: String,
        /// Bundle/file path. Empty for `builtin`.
        path: String,
        sample_rate: f64,
        max_frames: u32,
        /// Optional base64 state blob applied right after instantiation.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        state_b64: String,
    },
    /// Change plugin processing rate; host re-activates on the audio side.
    Reconfigure {
        id: u64,
        sample_rate: f64,
        max_frames: u32,
    },
    /// Select native offline DSP mode on a fresh export-owned instance.
    SetOffline { id: u64, enabled: bool },
    /// Normalized/plain parameter set. Plain for CLAP/AU, normalized 0..1 for VST3.
    SetParam { id: u64, param_id: String, value: f64 },
    GetParams { id: u64 },
    /// Control-only admission of native automation targets; never sent by RT.
    GetAutomationParams { id: u64 },
    /// Bounded measured input/output history; analyzed on a client background
    /// thread. This request never runs in an audio callback.
    GetTelemetry { id: u64 },
    GetState { id: u64 },
    /// Base64 blob previously produced by GetState on the same plugin id+version.
    SetState { id: u64, state_b64: String },
    /// Open the plugin's native editor inside a host-created window.
    OpenEditor { id: u64, title: String },
    CloseEditor { id: u64 },
    /// Polite teardown; host must exit promptly after replying.
    Shutdown { id: u64 },
}

/// Host → app messages. Exactly one `reply` per request `id`; `event`s may
/// arrive at any time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostMessage {
    HelloAck {
        protocol: u32,
        #[serde(default)]
        formats: Vec<String>,
        #[serde(default)]
        scan_supports: bool,
    },
    Reply {
        id: u64,
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(flatten)]
        data: Map<String, Value>,
    },
    Event {
        event: String,
        #[serde(flatten)]
        data: Map<String, Value>,
    },
}

/// `Reply.data` conventions (not separately typed; both sides agree):
/// - `instantiate` ok: `{"plugin_id": "…", "latency": n, "has_editor": bool, "name": "…"}`
/// - `get_params` ok: `{"params": [{"id","name","value","min","max","default","unit","step_count"}]}`
/// - `get_state` ok: `{"state_b64": "…"}`
/// - `get_telemetry` ok: `{"telemetry": TelemetryFrame}` (see `telemetry` module)
/// - `open_editor` ok: `{}` (failure → ok:false with error)
///
/// `Event.event` names: `"param_changed"`, `"latency_changed"`,
/// `"editor_closed"`, `"log"` (plugin-side log routed via host), `"scan_progress"`.

/// Descriptor of one plugin inside a scanned file/bundle. Serialized both in
/// `scan-file` output and the scan cache.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannedPlugin {
    /// `clap` | `vst3` | `au`.
    pub format: String,
    /// Format-specific instantiation id.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub url: String,
    /// Feature strings as reported by the format (e.g. CLAP `features`).
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub has_editor: bool,
}

/// Top-level JSON emitted by `verdant-plughost scan-file <path>` on stdout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFileReport {
    pub path: String,
    /// `clap` | `vst3` | `au` | `builtin` | `unknown`.
    pub format: String,
    pub sha256: String,
    pub file_size: u64,
    /// Present iff probing the bundle succeeded.
    #[serde(default)]
    pub plugins: Vec<ScannedPlugin>,
    /// Present iff probing failed (load error, no plugins, wrong format).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub scan_ms: u64,
}

/// Write one JSON line. Shared by both ends so framing never diverges.
pub fn write_line<W: std::io::Write>(w: &mut W, msg: &impl Serialize) -> std::io::Result<()> {
    let mut buf = serde_json::to_vec(msg)?;
    buf.push(b'\n');
    w.write_all(&buf)?;
    w.flush()
}

/// One message per line; a peer that spews megabyte lines is treated as
/// hostile and disconnected rather than allowed to exhaust memory.
pub const MAX_LINE: usize = 8 * 1024 * 1024;

/// Blocking read of one JSON line → `HostMessage`. Returns `Ok(None)` on clean
/// EOF (peer exited). Lines longer than `MAX_LINE` are an error, not a hang.
pub fn read_host_message<R: std::io::BufRead>(r: &mut R) -> std::io::Result<Option<HostMessage>> {
    read_line_opt(r, MAX_LINE).and_then(|opt| {
        opt.map(|line| serde_json::from_slice(&line).map_err(json_to_io))
            .transpose()
    })
}

/// Blocking read of one JSON line → `HostRequest`.
pub fn read_host_request<R: std::io::BufRead>(r: &mut R) -> std::io::Result<Option<HostRequest>> {
    read_line_opt(r, MAX_LINE).and_then(|opt| {
        opt.map(|line| serde_json::from_slice(&line).map_err(json_to_io))
            .transpose()
    })
}

/// Reads one line without trusting `BufRead::read_line`'s unbounded growth.
pub fn read_line_opt<R: std::io::BufRead>(r: &mut R, cap: usize) -> std::io::Result<Option<Vec<u8>>> {
    let mut buf = Vec::with_capacity(1024);
    loop {
        let avail = r.fill_buf()?;
        if avail.is_empty() {
            return Ok(if buf.is_empty() { None } else { Some(buf) });
        }
        let take = avail
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(avail.len());
        buf.extend_from_slice(&avail[..take]);
        r.consume(take);
        if buf.len() > cap {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "control line exceeds limit",
            ));
        }
        if buf.last() == Some(&b'\n') {
            buf.pop();
            if buf.last() == Some(&b'\r') {
                buf.pop();
            }
            return Ok(Some(buf));
        }
    }
}

fn json_to_io(e: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    fn descriptor() -> ParamDesc {
        ParamDesc { id: "native".into(), name: "Native".into(), value: 0.5,
            min: 0.0, max: 1.0, unit: String::new(), step_count: 0, default_value: None }
    }

    #[test]
    fn native_target_rejects_nonfinite_inverted_and_discrete_values() {
        let desc = descriptor();
        let target = ParamTarget::from_desc(93, &desc).unwrap();
        assert_eq!(target.event(255, 0.0).unwrap(), (93, 255, 0.0));
        assert_eq!(target.event(0, 1.0).unwrap(), (93, 0, 1.0));
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, 1.01] {
            assert_eq!(target.event(0, value), Err(AutomationError::ValueOutOfRange));
        }
        for (min, max, value) in [(f64::NAN, 1.0, 0.5), (0.0, f64::INFINITY, 0.5),
            (1.0, 0.0, 0.5), (0.0, 1.0, f64::NAN), (-f64::MAX, f64::MAX, 0.0)] {
            let desc = ParamDesc { min, max, value, ..descriptor() };
            assert_eq!(ParamTarget::from_desc(0, &desc), Err(AutomationError::InvalidDescriptor));
        }
        let desc = ParamDesc { step_count: 4, ..descriptor() };
        let stepped = ParamTarget::from_desc(7, &desc).unwrap();
        assert!(stepped.event(0, 0.75).is_ok());
        assert_eq!(stepped.event(0, 0.7), Err(AutomationError::InvalidStep));
    }

    #[test]
    fn whole_call_admission_accounts_for_partial_quanta_and_offsets() {
        assert!(validate_event_quanta(&[(3, 0, 0.0), (3, 255, 1.0)], 256, 256, 0, 0).is_ok());
        assert_eq!(validate_event_quanta(&[(3, 256, 0.0)], 256, 256, 0, 0), Err(AutomationError::InvalidOffset));
        assert_eq!(validate_event_quanta(&[(3, 7, 0.0), (3, 6, 1.0)], 256, 256, 0, 0), Err(AutomationError::UnsortedEvents));
        assert_eq!(validate_event_quanta(&[(3, 0, 0.0)], 192, 256, 64, MAX_PARAM_EVENTS), Err(AutomationError::EventCapacity));
        // The full old quantum may complete before the next event arrives.
        assert!(validate_event_quanta(&[(3, 192, 0.0)], 193, 256, 64, MAX_PARAM_EVENTS).is_ok());
    }

    #[test]
    fn automation_target_control_message_roundtrip() {
        let request = HostRequest::GetAutomationParams { id: 91 };
        let mut bytes = Vec::new();
        write_line(&mut bytes, &request).unwrap();
        assert!(matches!(read_host_request(&mut BufReader::new(&bytes[..])).unwrap(),
            Some(HostRequest::GetAutomationParams { id: 91 })));
        let descriptor = AutomationParam { id: "native".into(),
            target: ParamTarget::from_desc(42, &descriptor()).unwrap() };
        let bytes = serde_json::to_vec(&descriptor).unwrap();
        let back: AutomationParam = serde_json::from_slice(&bytes).unwrap();
        back.target.validate().unwrap();
        assert_eq!(back.target.event(9, 0.75).unwrap(), (42, 9, 0.75));
    }

    #[test]
    fn request_roundtrip() {
        let req = HostRequest::Instantiate {
            id: 7,
            format: "clap".into(),
            plugin_id: "org.x.y".into(),
            path: "/p/x.clap".into(),
            sample_rate: 48000.0,
            max_frames: 8192,
            state_b64: "QUJD".into(),
        };
        let mut buf = Vec::new();
        write_line(&mut buf, &req).unwrap();
        let mut r = BufReader::new(&buf[..]);
        let back = read_host_request(&mut r).unwrap().unwrap();
        match back {
            HostRequest::Instantiate { id, plugin_id, state_b64, .. } => {
                assert_eq!(id, 7);
                assert_eq!(plugin_id, "org.x.y");
                assert_eq!(state_b64, "QUJD");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn reply_with_flattened_data() {
        let mut data = Map::new();
        data.insert("latency".into(), Value::from(128));
        data.insert("has_editor".into(), Value::from(true));
        let msg = HostMessage::Reply { id: 3, ok: true, error: None, data };
        let mut buf = Vec::new();
        write_line(&mut buf, &msg).unwrap();
        let mut r = BufReader::new(&buf[..]);
        match read_host_message(&mut r).unwrap().unwrap() {
            HostMessage::Reply { id, ok, data, .. } => {
                assert!(ok && id == 3);
                assert_eq!(data["latency"], Value::from(128));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn capped_line_reader() {
        let long = vec![b'x'; MAX_LINE + 1];
        let mut cur = std::io::Cursor::new(long);
        let mut r = BufReader::new(&mut cur);
        assert!(read_line_opt(&mut r, MAX_LINE).is_err());

        let mut r = BufReader::new(&b"abc\nrest"[..]);
        assert_eq!(read_line_opt(&mut r, MAX_LINE).unwrap().unwrap(), b"abc");
        assert_eq!(read_line_opt(&mut r, MAX_LINE).unwrap().unwrap(), b"rest");
        assert!(read_line_opt(&mut r, MAX_LINE).unwrap().is_none());
    }
}
