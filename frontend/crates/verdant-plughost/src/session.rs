//! `host` mode session: runs on the main thread so plugin "main thread"
//! callbacks (CLAP gui, AU CocoaUI) land correctly.
//!
//! Loop: drain old audio wakes → check published input → wait if idle →
//! 1. audio: `while in_seq > done { read → process → write → signal }`
//! 2. control: drain request queue, dispatch, reply
//! 3. service: editor event pumps, plugin periodic work, latency refresh.

use crate::control::{self, ControlPipe};
use crate::formats::{self, HostedPlugin};
use crate::protocol_output::ProtocolOutput;
use std::sync::atomic::Ordering;
use std::time::Duration;
use verdant_plugins::error::{PluginError, Result};
use verdant_plugins::proto::{self, AutomationError, AutomationParam, HostMessage, HostRequest,
    ParamEvent, MAX_PARAM_EVENTS};
use verdant_plugins::shm::{wait_any, Channel, Ring};

/// Maximum audio backlog the worker will process in one poll pass before
/// re-checking control (defends against a parent flooding `in_seq`).
const MAX_BLOCKS_PER_PASS: u64 = 64;
/// How often the worker wakes up even when fully idle, for plugin servicing
/// (editor event pumps, tail/latency refreshes).
const IDLE_PERIOD: Duration = Duration::from_millis(25);

/// Entry point for `verdant-plughost host --channel <arg>` — the arg encodes
/// `fd:9,10,11` (unix) or `name:a,b,c` (windows), matching `Channel::child_arg`.
pub fn run_host(channel_arg: &str, output: ProtocolOutput) -> Result<()> {
    crate::window::initialize().map_err(PluginError::Host)?;
    let (shm_conn, in_w, out_w) = Channel::parse_child_arg(channel_arg)?;
    let (ring, audio_in, audio_out) = Channel::connect(&shm_conn, &in_w, &out_w)?;
    ring.set_host_pid(std::process::id());

    let ctrl = control::spawn(output);

    let (ev_tx, ev_rx) = make_event_channel();
    let mut session = Session {
        ring,
        audio_in,
        audio_out,
        ctrl,
        plugin: None,
        done: 0,
        in_l: vec![0.0; 1],
        in_r: vec![0.0; 1],
        key_l: vec![0.0; 1],
        key_r: vec![0.0; 1],
        out_l: vec![0.0; 1],
        out_r: vec![0.0; 1],
        parameter_events: vec![(0, 0, 0.0); MAX_PARAM_EVENTS],
        automation: Vec::new(),
        editor_title: None,
        ev_tx,
        ev_rx,
    };
    session.run()
}

fn make_event_channel() -> (formats::EventTx, std::sync::mpsc::Receiver<(
    String,
    serde_json::Map<String, serde_json::Value>,
)>) {
    std::sync::mpsc::channel()
}

struct Session {
    ring: Ring,
    /// App→host doorbell (read end).
    audio_in: verdant_plugins::shm::Wake,
    /// Host→app doorbell (write end).
    audio_out: verdant_plugins::shm::Wake,
    ctrl: ControlPipe,
    /// The instantiated plugin; `None` = passthrough until `instantiate`.
    plugin: Option<Box<dyn HostedPlugin>>,
    /// Next block index to produce.
    done: u64,
    /// Preallocated block buffers, grown once to ring capacity.
    in_l: Vec<f32>,
    in_r: Vec<f32>,
    /// Sidechain key planes of the current input block.
    key_l: Vec<f32>,
    key_r: Vec<f32>,
    out_l: Vec<f32>,
    out_r: Vec<f32>,
    /// Native events are copied only from a claimed slot into reused storage.
    parameter_events: Vec<ParamEvent>,
    /// Control-admitted targets, sorted by numeric id for bounded DSP validation.
    automation: Vec<AutomationParam>,
    /// Whether an editor window is currently open (for service pumping).
    editor_title: Option<String>,
    /// Format-side async events forwarded to the client.
    ev_tx: formats::EventTx,
    ev_rx: std::sync::mpsc::Receiver<(
        String,
        serde_json::Map<String, serde_json::Value>,
    )>,
}

impl Session {
    fn run(&mut self) -> Result<()> {
        let max = self.ring.max_frames();
        for b in [
            &mut self.in_l,
            &mut self.in_r,
            &mut self.key_l,
            &mut self.key_r,
            &mut self.out_l,
            &mut self.out_r,
        ] {
            b.resize(max, 0.0);
        }

        loop {
            if self.ring.shutdown_requested() {
                let _ = self
                    .ctrl
                    .outgoing
                    .send(control::reply(u64::MAX, true, None, Default::default()));
                return Ok(());
            }
            if self.ctrl.stop.load(Ordering::Acquire) {
                return Ok(()); // parent closed stdin — exit
            }

            // Clear notifications before checking the input predicate. The
            // producer publishes in_seq before signaling: input published
            // during this drain is caught by the predicate; input published
            // after the check leaves a wake for poll. Draining after the last
            // input check could otherwise swallow the next block's only wake.
            self.audio_in.drain();
            let which = if self.ring.in_seq() > self.done {
                Some(0)
            } else {
                // IDLE_PERIOD still bounds editor servicing when fully idle.
                wait_any(&[&self.audio_in, &self.ctrl.cmd_ready], IDLE_PERIOD)
            };

            // Audio first (latency-critical), bounded per pass.
            if which.is_none() || which == Some(0) {
                self.drain_audio()?;
            }

            // Control.
            let mut had_cmd = false;
            while let Ok(req) = self.ctrl.requests.try_recv() {
                had_cmd = true;
                self.dispatch(req);
            }
            if which == Some(1) || had_cmd {
                self.ctrl.cmd_ready.drain();
            }

            // Periodic servicing (editor pump, latency refresh).
            crate::window::pump_events().map_err(PluginError::Host)?;
            if let Some(p) = self.plugin.as_mut() {
                p.service().map_err(PluginError::Host)?;
                let lat = p.latency();
                if lat as u32 != self.ring.plugin_latency() {
                    self.ring.set_plugin_latency(lat as u32);
                    let mut data = serde_json::Map::new();
                    data.insert("latency".into(), lat.into());
                    let _ = self.ctrl.outgoing.send(control::event("latency_changed", data));
                }
                if self.editor_title.is_some() && !p.editor_open() {
                    self.editor_title = None;
                    let _ = self
                        .ctrl
                        .outgoing
                        .send(control::event("editor_closed", Default::default()));
                }
            }
            // Forward format-side async events (param/log/state_dirty…).
            while let Ok((name, d)) = self.ev_rx.try_recv() {
                if name == "params_changed" && !self.automation.is_empty() {
                    if let Some(plugin) = self.plugin.as_mut() {
                        self.automation = admitted_params(plugin.as_mut()).map_err(PluginError::Host)?;
                    }
                }
                let _ = self.ctrl.outgoing.send(control::event(&name, d));
            }
        }
    }

    /// Consume all published input blocks.
    fn drain_audio(&mut self) -> Result<()> {
        let mut produced = 0u64;
        // Bound stale history after a plugin/parent stall, while retaining
        // every quantum in a complete admitted callback burst.
        self.done = self.done.max(self.ring.in_seq().saturating_sub(verdant_plugins::shm::RING_SLOTS as u64));
        loop {
            let seq = self.done;
            if self.ring.in_seq() <= seq || produced >= MAX_BLOCKS_PER_PASS {
                break;
            }
            let (n, event_count, keyed) = self.ring.read_input_keyed(seq, &mut self.in_l, &mut self.in_r,
                Some((&mut self.key_l, &mut self.key_r)), &mut self.parameter_events)?;
            if n == 0 {
                // Published seq but unreadable slot — drop the block rather
                // than stall the pipe.
                self.done = seq + 1;
                produced += 1;
                continue;
            }
            {
                let (il, ir) = (&self.in_l[..n], &self.in_r[..n]);
                let (ol, orr) = (&mut self.out_l[..n], &mut self.out_r[..n]);
                match self.plugin.as_mut() {
                    Some(p) => {
                        let events = &self.parameter_events[..event_count];
                        // Validate the complete quantum before any native API
                        // can queue a point. Metadata discovery never runs here.
                        let instrument = p.is_instrument();
                        for &(native_id, offset, value) in events {
                            if instrument && verdant_plugins::proto::decode_note(native_id, value).is_some() { continue; }
                            let index = self.automation.binary_search_by_key(&native_id, |p| p.target.native_id())
                                .map_err(|_| AutomationError::UnknownParameter)?;
                            self.automation[index].target.event(offset, value)?;
                        }
                        if keyed { p.process_keyed(il, ir, (&self.key_l[..n], &self.key_r[..n]), ol, orr, events) }
                        else { p.process_events(il, ir, ol, orr, events) }.map_err(PluginError::Host)?;
                    }
                    None => {
                        return Err(PluginError::Host("audio submitted before instantiation".into()));
                    }
                }
                if ol.iter().chain(orr.iter()).any(|v| !v.is_finite()) {
                    return Err(PluginError::Host("plugin produced non-finite PCM".into()));
                }
            }
            let _ = self
                .ring
                .write_output(seq, &self.out_l[..n], &self.out_r[..n]);
            self.done = seq + 1;
            produced += 1;
            self.audio_out.signal();
        }
        Ok(())
    }

    fn dispatch(&mut self, req: HostRequest) {
        let id = req_id(&req);
        let mut data = serde_json::Map::new();
        let msg = match req {
            HostRequest::Hello { .. } => HostMessage::HelloAck {
                protocol: proto::PROTOCOL_VERSION,
                formats: formats::supported_formats(),
                scan_supports: true,
            },
            HostRequest::Instantiate {
                format,
                plugin_id,
                path,
                sample_rate,
                max_frames,
                state_b64,
                ..
            } => match formats::instantiate(
                &format,
                &plugin_id,
                &path,
                sample_rate,
                max_frames,
                self.ev_tx.clone(),
            ) {
                Ok((mut plugin, desc)) => {
                    if !state_b64.is_empty() {
                        let bytes = b64d(&state_b64);
                        match bytes.and_then(|b| plugin.set_state(&b).map_err(|e| e.to_string())) {
                            Ok(()) => {}
                            Err(e) => {
                                return self.send(control::reply(
                                    id,
                                    false,
                                    Some(format!("set_state: {e}")),
                                    Default::default(),
                                ));
                            }
                        }
                    }
                    if max_frames as usize != self.ring.max_frames() {
                        return self.send(control::reply(id, false, Some("ring quantum mismatch".into()), data));
                    }
                    data.insert("plugin_id".into(), desc.id.into());
                    data.insert("name".into(), desc.name.into());
                    data.insert("latency".into(), plugin.latency().into());
                    data.insert("has_editor".into(), plugin.has_editor().into());
                    self.ring.set_plugin_latency(plugin.latency() as u32);
                    self.automation.clear();
                    self.plugin = Some(plugin);
                    control::reply(id, true, None, data)
                }
                Err(e) => control::reply(id, false, Some(e.to_string()), Default::default()),
            },
            HostRequest::Reconfigure {
                sample_rate,
                max_frames,
                ..
            } if sample_rate != verdant_plugins::SAMPLE_RATE || max_frames as usize != self.ring.max_frames() => {
                control::reply(id, false, Some("insert rate/block quantum is fixed".into()), data)
            }
            HostRequest::Reconfigure { sample_rate, max_frames, .. } => match self.plugin.as_mut() {
                Some(p) => match p.reconfigure(sample_rate, max_frames) {
                    Ok(()) => {
                        data.insert("latency".into(), p.latency().into());
                        self.ring.set_plugin_latency(p.latency() as u32);
                        control::reply(id, true, None, data)
                    }
                    Err(e) => control::reply(id, false, Some(e.to_string()), Default::default()),
                },
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::SetParam { param_id, value, .. } => match self.plugin.as_mut() {
                Some(_) if !value.is_finite() => control::reply(id, false, Some("parameter must be finite".into()), data),
                Some(p) => match set_control_parameter(p.as_mut(), &param_id, value) {
                    Ok(()) => control::reply(id, true, None, data),
                    Err(e) => control::reply(id, false, Some(e.to_string()), Default::default()),
                },
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::SetOffline { enabled, .. } => match self.plugin.as_mut() {
                Some(p) => match p.set_offline(enabled) {
                    Ok(()) => {
                        self.ring.set_plugin_latency(p.latency() as u32);
                        control::reply(id, true, None, data)
                    }
                    Err(e) => control::reply(id, false, Some(e), data),
                },
                None => control::reply(id, false, Some("no plugin".into()), data),
            },
            HostRequest::GetAutomationParams { .. } => match self.plugin.as_mut() {
                Some(p) => match admitted_params(p.as_mut()) {
                    Ok(params) => {
                        match serde_json::to_value(&params) {
                            Ok(value) => {
                                data.insert("params".into(), value);
                                self.automation = params;
                                control::reply(id, true, None, data)
                            }
                            Err(e) => control::reply(id, false, Some(e.to_string()), data),
                        }
                    }
                    Err(e) => control::reply(id, false, Some(e), data),
                },
                None => control::reply(id, false, Some("no plugin".into()), data),
            },
            HostRequest::GetParams { .. } => match self.plugin.as_mut() {
                Some(p) => match p.params().and_then(|params| {
                    for param in &params {
                        proto::ParamTarget::from_desc(0, param).map_err(|e| e.to_string())?;
                    }
                    Ok(params)
                }) {
                    Ok(params) => {
                    let params: Vec<serde_json::Value> = params
                        .into_iter()
                        .map(|d| serde_json::to_value(d).unwrap_or_default())
                        .collect();
                    data.insert("params".into(), params.into());
                    control::reply(id, true, None, data)
                    }
                    Err(e) => control::reply(id, false, Some(e), data),
                }
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::GetState { .. } => match self.plugin.as_mut() {
                Some(p) => match p.state() {
                    Ok(bytes) => {
                        data.insert("state_b64".into(), b64e(&bytes).into());
                        control::reply(id, true, None, data)
                    }
                    Err(e) => control::reply(id, false, Some(e.to_string()), Default::default()),
                },
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::SetState { state_b64, .. } => match self.plugin.as_mut() {
                Some(p) => match b64d(&state_b64)
                    .and_then(|b| {
                        p.set_state(&b)?;
                        if !self.automation.is_empty() {
                            self.automation = admitted_params(p.as_mut())?;
                        }
                        Ok(())
                    })
                {
                    Ok(()) => control::reply(id, true, None, data),
                    Err(e) => control::reply(id, false, Some(e.to_string()), Default::default()),
                },
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::OpenEditor { title, .. } => match self.plugin.as_mut() {
                Some(p) => match p.open_editor(&title) {
                    Ok(()) => {
                        self.editor_title = Some(title);
                        control::reply(id, true, None, data)
                    }
                    Err(e) => control::reply(id, false, Some(e.to_string()), Default::default()),
                },
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::CloseEditor { .. } => match self.plugin.as_mut() {
                Some(p) => {
                    p.close_editor();
                    self.editor_title = None;
                    control::reply(id, true, None, data)
                }
                None => control::reply(id, false, Some("no plugin".into()), Default::default()),
            },
            HostRequest::Shutdown { .. } => {
                self.send(control::reply(id, true, None, data));
                self.ring.request_shutdown();
                return;
            }
        };
        self.send(msg);
    }

    fn send(&self, msg: HostMessage) {
        let _ = self.ctrl.outgoing.send(msg);
    }
}

fn set_control_parameter(plugin: &mut dyn HostedPlugin, id: &str, value: f64) -> std::result::Result<(), String> {
    let desc = plugin.params()?.into_iter().find(|p| p.id == id).ok_or("unknown parameter")?;
    let target = proto::ParamTarget::from_desc(0, &desc).map_err(|e| e.to_string())?;
    if !value.is_finite() || !(target.min()..=target.max()).contains(&value) {
        return Err("parameter outside finite native bounds".into());
    }
    plugin.set_param(id, value)
}

fn admitted_params(plugin: &mut dyn HostedPlugin) -> std::result::Result<Vec<AutomationParam>, String> {
    let mut params = plugin.automation_params()?;
    for param in &params { param.target.validate().map_err(|e| e.to_string())?; }
    params.sort_unstable_by_key(|param| param.target.native_id());
    if params.windows(2).any(|pair| pair[0].target.native_id() == pair[1].target.native_id()) {
        return Err("duplicate native automation parameter id".into());
    }
    let mut ids: Vec<_> = params.iter().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("duplicate persistent automation parameter id".into());
    }
    Ok(params)
}

fn req_id(req: &HostRequest) -> u64 {
    match req {
        HostRequest::Hello { id }
        | HostRequest::Instantiate { id, .. }
        | HostRequest::Reconfigure { id, .. }
        | HostRequest::SetOffline { id, .. }
        | HostRequest::SetParam { id, .. }
        | HostRequest::GetParams { id }
        | HostRequest::GetAutomationParams { id }
        | HostRequest::GetState { id }
        | HostRequest::SetState { id, .. }
        | HostRequest::OpenEditor { id, .. }
        | HostRequest::CloseEditor { id }
        | HostRequest::Shutdown { id } => *id,
    }
}

fn b64e(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}
fn b64d(s: &str) -> std::result::Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}
