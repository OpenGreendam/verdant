//! Native VST3 lifecycle, buses, controller/state streams and IPlugView
//! embedding via vst3-host 0.11. Isolation is provided by verdant-plughost.

use super::{EventTx, HostedPlugin, InstDesc, ScanOutcome};
use std::path::Path;
use verdant_plugins::error::PluginError;
use verdant_plugins::proto::{AutomationParam, ParamDesc, ParamEvent, ParamTarget,
    ScannedPlugin, MAX_PARAM_EVENTS};
use vst3_host::{BusAudioBuffers, Vst3Host};
use vst3_host::plugin::{ParameterEditKind, ProcessMode};

pub struct Vst3Plugin {
    // Release the instance and its editor before the outer host/context.
    plugin: vst3_host::Plugin,
    host: Vst3Host,
    path: String,
    id: String,
    bufs: BusAudioBuffers,
    max_frames: usize,
    offline: bool,
    editor: Option<crate::window::EditorWindow>,
    editor_title: Option<String>,
    editor_size: (u32, u32),
    events: EventTx,
}
impl Vst3Plugin {
    fn refresh_buffers(&mut self) -> Result<(), String> {
        self.bufs = make_buffers(&self.plugin, self.max_frames)?;
        Ok(())
    }
    fn flush_parameters(&mut self) -> Result<(), String> {
        // vst3-host 0.11.1 queues controller edits until process_bus_audio.
        // Keep the validated bus/channel shape but empty every channel: its
        // frame_count uses Vec lengths, not block_size. For zero samples the
        // wrapper supplies zero native buses and null audio pointers, drains
        // the parameter queues, and leaves the transport position unchanged.
        for bus in self.bufs.inputs.iter_mut().chain(&mut self.bufs.outputs) {
            for channel in &mut bus.channels { channel.clear(); }
        }
        self.bufs.block_size = 0;
        self.plugin.process_bus_audio(&mut self.bufs).map_err(|e| format!("VST3 parameter flush: {e}"))
    }
    fn reload(&mut self) -> Result<(), String> {
        let state = self.state()?;
        let title = self.editor_title.clone();
        self.close_editor();
        self.plugin.stop_processing().map_err(|e| e.to_string())?;
        let mut replacement = self.host.load_plugin_class(&self.path, &self.id).map_err(|e| e.to_string())?;
        replacement.load_state(&state).map_err(|e| e.to_string())?;
        if self.offline { replacement.set_process_mode(ProcessMode::Offline).map_err(|e| e.to_string())?; }
        replacement.start_processing().map_err(|e| e.to_string())?;
        self.plugin = replacement;
        self.refresh_buffers()?;
        if let Some(title) = title { self.open_editor(&title)?; }
        Ok(())
    }
}
fn make_buffers(plugin: &vst3_host::Plugin, max: usize) -> Result<BusAudioBuffers, String> {
    let layout = plugin.audio_bus_layout().map_err(|e| e.to_string())?;
    for bus in [layout.inputs.first(), layout.outputs.first()] {
        if !bus.is_some_and(|b| b.active && b.channel_count == 2) {
            return Err("VST3 insert requires active stereo main input and output buses".into());
        }
    }
    plugin.create_bus_audio_buffers(max).map_err(|e| e.to_string())
}
impl HostedPlugin for Vst3Plugin {
    fn process(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32]) -> Result<(), String> {
        let n = il.len();
        if n == 0 || n > self.max_frames || ir.len() != n || ol.len() != n || orr.len() != n {
            return Err("invalid VST3 stereo block".into());
        }
        // The SDK wrapper derives frame count from Vec lengths, not block_size.
        // Resize within the reserved capacity, including short final export blocks.
        for bus in self.bufs.inputs.iter_mut().chain(&mut self.bufs.outputs) {
            for ch in &mut bus.channels { ch.resize(n, 0.0); ch.fill(0.0); }
        }
        self.bufs.inputs[0].channels[0].copy_from_slice(il);
        self.bufs.inputs[0].channels[1].copy_from_slice(ir);
        self.bufs.block_size = n;
        self.plugin.process_bus_audio(&mut self.bufs).map_err(|e| format!("VST3 process: {e}"))?;
        ol.copy_from_slice(&self.bufs.outputs[0].channels[0]);
        orr.copy_from_slice(&self.bufs.outputs[0].channels[1]);
        Ok(())
    }
    fn process_events(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32], events: &[ParamEvent]) -> Result<(), String> {
        if events.len() > MAX_PARAM_EVENTS || il.is_empty() || il.len() > self.max_frames
            || ir.len() != il.len() || ol.len() != il.len() || orr.len() != il.len() {
            return Err("invalid VST3 native automation block".into());
        }
        let mut previous = 0;
        for &(_, offset, value) in events {
            if offset as usize >= il.len() || offset < previous || !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err("invalid VST3 native parameter event".into());
            }
            previous = offset;
        }
        // The pinned wrapper has a 4096-point pending queue. This exclusively
        // owned host drains it after every process; immediate control edits
        // already flush it. GUI edits use its separate component-handler queue.
        // No point above the native queue capacity is ever submitted to its
        // otherwise-successful overflow/drop path.
        for &(id, offset, value) in events {
            self.plugin.set_parameter_at(id, value, offset as i32).map_err(|e| e.to_string())?;
        }
        self.process(il, ir, ol, orr)
    }

    fn automation_params(&mut self) -> Result<Vec<AutomationParam>, String> {
        self.plugin.get_parameters().map_err(|e| e.to_string())?.into_iter()
            .filter(|p| p.can_automate && !p.is_read_only)
            .map(|p| {
                let desc = ParamDesc { id: p.id.to_string(), name: p.name, value: p.value,
                    min: p.min, max: p.max, unit: p.unit, step_count: p.step_count as i64,
                    default_value: None };
                let target = ParamTarget::from_desc(p.id, &desc).map_err(|e| e.to_string())?;
                Ok(AutomationParam { id: desc.id, target })
            }).collect()
    }

    fn latency(&mut self) -> u64 { self.plugin.latency_samples() as u64 }
    fn params(&mut self) -> Result<Vec<ParamDesc>, String> {
        self.plugin.get_parameters().map(|params| params.into_iter().map(|p| ParamDesc {
            id: p.id.to_string(), name: p.name, value: p.value,
            min: p.min, max: p.max, unit: p.unit, step_count: p.step_count as i64,
            default_value: None,
        }).collect()).map_err(|e| e.to_string())
    }
    fn set_param(&mut self, id: &str, value: f64) -> Result<(), String> {
        let id = id.parse::<u32>().map_err(|_| "invalid VST3 parameter id")?;
        self.plugin.set_parameter(id, value).map_err(|e| e.to_string())?;
        self.flush_parameters()
    }
    fn state(&mut self) -> Result<Vec<u8>, String> {
        self.flush_parameters()?;
        self.plugin.save_state().map_err(|e| e.to_string())
    }
    fn set_state(&mut self, data: &[u8]) -> Result<(), String> {
        self.plugin.load_state(data).map_err(|e| e.to_string())?;
        self.service()?;
        self.flush_parameters()
    }
    fn set_offline(&mut self, on: bool) -> Result<(), String> {
        self.plugin.stop_processing().map_err(|e| e.to_string())?;
        self.plugin.set_process_mode(if on { ProcessMode::Offline } else { ProcessMode::Realtime })
            .map_err(|e| e.to_string())?;
        self.plugin.start_processing().map_err(|e| e.to_string())?;
        self.offline = on;
        self.refresh_buffers()
    }
    fn has_editor(&self) -> bool { self.plugin.has_editor() }
    fn open_editor(&mut self, title: &str) -> Result<(), String> {
        if self.editor.is_some() { return Ok(()); }
        if !self.plugin.has_editor() { return Err("VST3 plugin has no editor".into()); }
        let mut win = crate::window::EditorWindow::open(title)?;
        let size = self.plugin.get_editor_size().map_err(|e| e.to_string())?;
        if size.0 <= 0 || size.1 <= 0 { return Err("VST3 editor size invalid".into()); }
        win.resize(size.0 as u32, size.1 as u32);
        self.editor_size = (size.0 as u32, size.1 as u32);
        self.editor = Some(win);
        if let Err(e) = self.plugin.open_editor(self.editor.as_ref().unwrap().vst3_window_handle()) {
            // attached may have partly succeeded; detach before dropping parent.
            let _ = self.plugin.close_editor();
            self.editor = None;
            return Err(e.to_string());
        }
        self.editor_title = Some(title.into());
        Ok(())
    }
    fn close_editor(&mut self) {
        let _ = self.plugin.close_editor();
        self.editor = None;
        self.editor_title = None;
    }
    fn editor_open(&self) -> bool { self.editor.is_some() && self.plugin.is_editor_open() }
    fn reconfigure(&mut self, rate: f64, max: u32) -> Result<(), String> {
        if rate != verdant_plugins::SAMPLE_RATE || max as usize != self.max_frames {
            return Err("VST3 insert processing contract is fixed".into());
        }
        self.plugin.stop_processing().map_err(|e| e.to_string())?;
        self.plugin.reconfigure(rate, max as usize).map_err(|e| e.to_string())?;
        self.plugin.start_processing().map_err(|e| e.to_string())?;
        self.refresh_buffers()
    }
    fn service(&mut self) -> Result<(), String> {
        self.plugin.service_run_loop();
        let flags = self.plugin.service_host_requests().map_err(|e| e.to_string())?;
        if flags.reload_component() { self.reload()?; }
        else if flags.io_changed() { self.refresh_buffers()?; }
        if flags.param_values_changed() || flags.param_titles_changed() {
            let _ = self.events.send(("params_changed".into(), Default::default()));
        }
        for edit in self.plugin.take_parameter_edits() {
            let mut data = serde_json::Map::new();
            data.insert("param_id".into(), edit.id.to_string().into());
            if let Some(value) = edit.value { data.insert("value".into(), value.into()); }
            let kind = match edit.kind {
                ParameterEditKind::ValueChange => "param_changed",
                ParameterEditKind::BeginGesture => "param_begin",
                ParameterEditKind::EndGesture => "param_end",
            };
            let _ = self.events.send((kind.into(), data));
        }
        if let Some(win) = self.editor.as_mut() {
            if !win.pump() || !self.plugin.is_editor_open() { self.close_editor(); return Ok(()); }
            if let Some((w, h)) = self.plugin.take_editor_resize_request() {
                self.editor_size = (w.max(1) as u32, h.max(1) as u32);
                win.resize(self.editor_size.0, self.editor_size.1);
            } else if win.size() != self.editor_size {
                if self.plugin.attached_editor_can_resize() == Some(true) {
                    let (w, h) = win.size();
                    let (w, h) = self.plugin.resize_editor(w as i32, h as i32).map_err(|e| e.to_string())?;
                    self.editor_size = (w.max(1) as u32, h.max(1) as u32);
                }
                win.resize(self.editor_size.0, self.editor_size.1);
            }
        }
        Ok(())
    }
}
impl Drop for Vst3Plugin {
    fn drop(&mut self) { self.close_editor(); let _ = self.plugin.stop_processing(); }
}

pub fn instantiate(path: &str, id: &str, rate: f64, max: u32, events: EventTx)
    -> Result<(Box<dyn HostedPlugin>, InstDesc), PluginError> {
    let mut host = Vst3Host::builder().sample_rate(rate).block_size(max as usize)
        .input_channels(2).output_channels(2).build().map_err(|e| PluginError::Host(e.to_string()))?;
    let mut plugin = if id.is_empty() { host.load_plugin(path) } else { host.load_plugin_class(path, id) }
        .map_err(|e| PluginError::Host(format!("VST3 load {path}: {e}")))?;
    plugin.start_processing().map_err(|e| PluginError::Host(e.to_string()))?;
    let info = plugin.info().clone();
    let bufs = make_buffers(&plugin, max as usize).map_err(PluginError::Host)?;
    Ok((Box::new(Vst3Plugin {
        plugin, host, path: path.into(), id: info.uid.clone(), bufs,
        max_frames: max as usize, offline: false, editor: None, editor_title: None,
        editor_size: (0, 0), events,
    }), InstDesc { id: info.uid, name: info.name }))
}

pub fn scan(path: &Path) -> Result<ScanOutcome, PluginError> {
    // The dependency instantiates during deep introspection. This only runs
    // in the disposable timeout-guarded scanner, never in the app process.
    let info = vst3_host::get_detailed_plugin_info(path).map_err(|e| PluginError::Host(format!("VST3 scan: {e}")))?;
    let plugins = info.classes.into_iter().filter(|c| c.category == "Audio Module Class").map(|c| ScannedPlugin {
        format: "vst3".into(), id: c.class_id, name: c.name, vendor: info.factory.vendor.clone(),
        version: c.version, url: info.factory.url.clone(), features: vec![c.category],
        has_editor: false, // Per-class editor capability is queried after load.
    }).collect();
    Ok(ScanOutcome { format: "vst3", plugins })
}
