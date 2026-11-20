//! CLAP effects via clack 0.2. All instance/main-thread work and serialized
//! processing run on the host's main thread, outside the application's RT
//! callback. Native GUI parents outlive gui.destroy, including error paths.

use super::{EventTx, HostedPlugin, InstDesc, ScanOutcome};
use clack_extensions::audio_ports::{AudioPortInfoBuffer, PluginAudioPorts};
use clack_extensions::gui::{GuiApiType, GuiConfiguration, GuiSize, HostGui, HostGuiImpl, PluginGui};
use clack_extensions::latency::{HostLatency, HostLatencyImpl, PluginLatency};
use clack_extensions::log::{HostLog, HostLogImpl, LogSeverity};
use clack_extensions::params::{HostParams, HostParamsImplMainThread, HostParamsImplShared,
    ParamClearFlags, ParamInfoBuffer, ParamInfoFlags, ParamRescanFlags, PluginParams};
use clack_extensions::state::{HostState, HostStateImpl, PluginState};
use clack_extensions::render::{PluginRender, RenderMode};
use clack_extensions::timer::{HostTimer, HostTimerImpl, PluginTimer, TimerId};
#[cfg(unix)]
use clack_extensions::posix_fd::{FdFlags, HostPosixFd, HostPosixFdImpl, PluginPosixFd};
use clack_host::events::event_types::ParamValueEvent;
use clack_host::events::io::{EventBuffer, InputEvents, OutputEvents};
use clack_host::events::Pckn;
use clack_host::prelude::*;
use clack_host::process::audio_buffers::{AudioPortBuffer, AudioPortBufferType, AudioPorts, InputChannel};
use clack_host::process::{PluginAudioConfiguration, PluginAudioProcessor};
use std::ffi::CString;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use verdant_plugins::error::PluginError;
use verdant_plugins::proto::{AutomationParam, ParamDesc, ParamEvent, ParamTarget,
    ScannedPlugin, MAX_PARAM_EVENTS};

#[derive(Default)]
struct Exts {
    params: OnceLock<Option<PluginParams>>,
    state: OnceLock<Option<PluginState>>,
    latency: OnceLock<Option<PluginLatency>>,
    gui: OnceLock<Option<PluginGui>>,
    ports: OnceLock<Option<PluginAudioPorts>>,
}

#[derive(Default)]
struct Signals {
    restart: AtomicBool,
    callback: AtomicBool,
    flush: AtomicBool,
    resize: AtomicU64,
    visible: AtomicI32,
    closed: AtomicBool,
}

struct Shared { exts: Arc<Exts>, signals: Arc<Signals>, events: EventTx }
impl<'a> SharedHandler<'a> for Shared {
    fn initializing(&self, p: InitializingPluginHandle<'a>) {
        let _ = self.exts.params.set(p.get_extension());
        let _ = self.exts.state.set(p.get_extension());
        let _ = self.exts.latency.set(p.get_extension());
        let _ = self.exts.gui.set(p.get_extension());
        let _ = self.exts.ports.set(p.get_extension());
    }
    fn request_restart(&self) { self.signals.restart.store(true, Ordering::Release); }
    fn request_process(&self) { self.signals.flush.store(true, Ordering::Release); }
    fn request_callback(&self) { self.signals.callback.store(true, Ordering::Release); }
}
impl HostLogImpl for Shared {
    fn log(&self, severity: LogSeverity, message: &str) {
        let mut data = serde_json::Map::new();
        data.insert("severity".into(), format!("{severity:?}").into());
        data.insert("message".into(), message.into());
        let _ = self.events.send(("log".into(), data));
    }
}
impl HostParamsImplShared for Shared {
    fn request_flush(&self) { self.signals.flush.store(true, Ordering::Release); }
}
impl HostGuiImpl for Shared {
    fn resize_hints_changed(&self) {}
    fn request_resize(&self, size: GuiSize) -> Result<(), HostError> {
        if size.width == 0 || size.height == 0 || size.width > 16384 || size.height > 16384 {
            return Err(HostError::Message("invalid editor size"));
        }
        self.signals.resize.store(size.pack_to_u64(), Ordering::Release);
        Ok(())
    }
    fn request_show(&self) -> Result<(), HostError> {
        self.signals.visible.store(1, Ordering::Release);
        Ok(())
    }
    fn request_hide(&self) -> Result<(), HostError> {
        self.signals.visible.store(-1, Ordering::Release);
        Ok(())
    }
    fn closed(&self, _was_destroyed: bool) { self.signals.closed.store(true, Ordering::Release); }
}
struct Mt<'a> {
    shared: &'a Shared,
    timers: RefCell<BTreeMap<TimerId, (Duration, Instant)>>,
    next_timer: Cell<u32>,
    #[cfg(unix)]
    fds: RefCell<BTreeMap<i32, FdFlags>>,
}
impl<'a> MainThreadHandler<'a> for Mt<'a> {}
impl HostParamsImplMainThread for Mt<'_> {
    fn rescan(&self, _flags: ParamRescanFlags) {
        let _ = self.shared.events.send(("params_changed".into(), Default::default()));
    }
    fn clear(&self, _id: ClapId, _flags: ParamClearFlags) {}
}
impl HostStateImpl for Mt<'_> {
    fn mark_dirty(&self) { let _ = self.shared.events.send(("state_dirty".into(), Default::default())); }
}
impl HostLatencyImpl for Mt<'_> {
    fn changed(&self) {} // Session refreshes latency after servicing the plugin.
}
impl HostTimerImpl for Mt<'_> {
    fn register_timer(&self, ms: u32) -> Result<TimerId, HostError> {
        let id = self.next_timer.get();
        let next = id.checked_add(1).ok_or(HostError::Message("timer ids exhausted"))?;
        self.next_timer.set(next);
        let period = Duration::from_millis(ms.max(1) as u64);
        self.timers.borrow_mut().insert(TimerId(id), (period, Instant::now() + period));
        Ok(TimerId(id))
    }
    fn unregister_timer(&self, id: TimerId) -> Result<(), HostError> {
        self.timers.borrow_mut().remove(&id).ok_or(HostError::Message("unknown timer"))?;
        Ok(())
    }
}
#[cfg(unix)]
impl HostPosixFdImpl for Mt<'_> {
    fn register_fd(&self, fd: i32, flags: FdFlags) -> Result<(), HostError> {
        if fd < 0 || self.fds.borrow().contains_key(&fd) { return Err(HostError::Message("invalid or duplicate fd")); }
        self.fds.borrow_mut().insert(fd, flags);
        Ok(())
    }
    fn modify_fd(&self, fd: i32, flags: FdFlags) -> Result<(), HostError> {
        *self.fds.borrow_mut().get_mut(&fd).ok_or(HostError::Message("unknown fd"))? = flags;
        Ok(())
    }
    fn unregister_fd(&self, fd: i32) -> Result<(), HostError> {
        self.fds.borrow_mut().remove(&fd).ok_or(HostError::Message("unknown fd"))?;
        Ok(())
    }
}
struct ClapHost;
impl HostHandlers for ClapHost {
    type Shared<'a> = Shared;
    type MainThread<'a> = Mt<'a>;
    type AudioProcessor<'a> = ();
    fn declare_extensions(b: &mut HostExtensions<Self>, _: &Shared) {
        b.register::<HostParams>().register::<HostState>().register::<HostLatency>()
            .register::<HostGui>().register::<HostLog>().register::<HostTimer>();
        #[cfg(unix)]
        b.register::<HostPosixFd>();
    }
}

pub struct ClapPlugin {
    instance: PluginInstance<ClapHost>,
    exts: Arc<Exts>,
    signals: Arc<Signals>,
    events: EventTx,
    audio: Option<PluginAudioProcessor<ClapHost>>,
    config: PluginAudioConfiguration,
    in_bufs: [Vec<f32>; 2],
    out_bufs: [Vec<f32>; 2],
    in_ports: AudioPorts,
    out_ports: AudioPorts,
    in_events: EventBuffer,
    out_events: EventBuffer,
    editor: Option<crate::window::EditorWindow>,
    gui_created: bool,
    gui_size: (u32, u32),
    steady_time: u64,
}
impl ClapPlugin {
    fn stop(&mut self) {
        if let Some(audio) = self.audio.take() { self.instance.deactivate(audio.into_stopped()); }
    }
    fn activate(&mut self) -> Result<(), String> {
        let stopped = self.instance.activate(|_, _| (), self.config).map_err(|e| format!("activate: {e:?}"))?;
        let mut audio = PluginAudioProcessor::Stopped(stopped);
        if let Err(e) = audio.start_processing() {
            self.instance.deactivate(audio.into_stopped());
            return Err(format!("start processing: {e:?}"));
        }
        self.audio = Some(audio);
        Ok(())
    }
    fn check_ports(&mut self) -> Result<(), String> {
        let ports = self.exts.ports.get().copied().flatten().ok_or("CLAP effect has no audio ports")?;
        let handle = self.instance.plugin_handle();
        for input in [true, false] {
            if ports.count(&handle, input) != 1 { return Err("CLAP insert needs one stereo input and output bus".into()); }
            let mut buf = AudioPortInfoBuffer::new();
            let info = ports.get(&handle, 0, input, &mut buf).ok_or("CLAP port query failed")?;
            if info.channel_count != 2 { return Err("CLAP insert needs stereo ports".into()); }
        }
        Ok(())
    }
    fn destroy_gui(&mut self) {
        if self.gui_created {
            if let Some(gui) = self.exts.gui.get().copied().flatten() {
                let handle = self.instance.plugin_handle();
                let _ = gui.hide(&handle);
                gui.destroy(&handle);
            }
            self.gui_created = false;
        }
        self.editor = None;
        self.signals.closed.store(false, Ordering::Release);
        self.signals.resize.store(0, Ordering::Release);
        self.signals.visible.store(0, Ordering::Release);
    }
    fn emit_params(&mut self) {
        for event in self.out_events.iter() {
            if let Some(p) = event.as_event::<ParamValueEvent>() {
                if let Some(id) = p.param_id() {
                    let mut data = serde_json::Map::new();
                    data.insert("param_id".into(), id.get().to_string().into());
                    data.insert("value".into(), p.value().into());
                    let _ = self.events.send(("param_changed".into(), data));
                }
            }
        }
        self.out_events.clear();
    }
    fn flush(&mut self) {
        if let (Some(params), Some(audio)) = (self.exts.params.get().copied().flatten(), self.audio.as_mut()) {
            let mut handle = audio.plugin_handle();
            params.flush_active(&mut handle, &InputEvents::from_buffer(&self.in_events),
                &mut OutputEvents::from_buffer(&mut self.out_events));
        }
        self.in_events.clear();
        self.emit_params();
    }
    fn service_reactors(&mut self) {
        let now = Instant::now();
        let due = self.instance.access_handler(|h| {
            h.timers.borrow_mut().iter_mut().filter_map(|(id, (period, next))| {
                if *next <= now { *next = now + *period; Some(*id) } else { None }
            }).collect::<Vec<_>>()
        });
        for id in due {
            if self.instance.access_handler(|h| h.timers.borrow().contains_key(&id)) {
                let handle = self.instance.plugin_handle();
                if let Some(timer) = handle.get_extension::<PluginTimer>() { timer.on_timer(&handle, id); }
            }
        }
        #[cfg(unix)]
        {
            let mut poll = self.instance.access_handler(|h| h.fds.borrow().iter().map(|(&fd, flags)| {
                let mut events = 0;
                if flags.contains(FdFlags::READ) { events |= libc::POLLIN; }
                if flags.contains(FdFlags::WRITE) { events |= libc::POLLOUT; }
                libc::pollfd { fd, events, revents: 0 }
            }).collect::<Vec<_>>());
            if unsafe { libc::poll(poll.as_mut_ptr(), poll.len() as libc::nfds_t, 0) } > 0 {
                for fd in poll {
                    if !self.instance.access_handler(|h| h.fds.borrow().contains_key(&fd.fd)) { continue; }
                    let mut flags = FdFlags::empty();
                    if fd.revents & libc::POLLIN != 0 { flags |= FdFlags::READ; }
                    if fd.revents & libc::POLLOUT != 0 { flags |= FdFlags::WRITE; }
                    if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 { flags |= FdFlags::ERROR; }
                    if !flags.is_empty() {
                        let handle = self.instance.plugin_handle();
                        if let Some(ext) = handle.get_extension::<PluginPosixFd>() { ext.on_fd(&handle, fd.fd, flags); }
                    }
                }
            }
        }
    }
}
impl HostedPlugin for ClapPlugin {
    fn process(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32]) -> Result<(), String> {
        let n = il.len();
        if n == 0 || n > self.config.max_frames_count as usize || ir.len() != n || ol.len() != n || orr.len() != n {
            return Err("invalid CLAP stereo block".into());
        }
        let audio = self.audio.as_mut().ok_or("CLAP processor inactive")?.as_started_mut()
            .map_err(|e| format!("CLAP processor stopped: {e:?}"))?;
        self.in_bufs[0][..n].copy_from_slice(il);
        self.in_bufs[1][..n].copy_from_slice(ir);
        for b in &mut self.out_bufs { b[..n].fill(0.0); }
        let inputs = self.in_ports.with_input_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_input_only(self.in_bufs.iter_mut().map(|b| InputChannel::variable(&mut b[..n]))),
        }]);
        let mut outputs = self.out_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only(self.out_bufs.iter_mut().map(|b| &mut b[..n])),
        }]);
        self.out_events.clear();
        audio.process(&inputs, &mut outputs, &InputEvents::from_buffer(&self.in_events),
            &mut OutputEvents::from_buffer(&mut self.out_events), Some(self.steady_time), None)
            .map_err(|e| format!("CLAP process failed: {e:?}"))?;
        self.in_events.clear();
        self.steady_time = self.steady_time.wrapping_add(n as u64);
        ol.copy_from_slice(&self.out_bufs[0][..n]);
        orr.copy_from_slice(&self.out_bufs[1][..n]);
        self.emit_params();
        Ok(())
    }
    fn process_events(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32], events: &[ParamEvent]) -> Result<(), String> {
        if events.len() > MAX_PARAM_EVENTS || !self.in_events.is_empty() {
            return Err("CLAP native parameter queue unavailable or over capacity".into());
        }
        // IDs/ranges/flags were admitted before this call. Validate every native
        // event header before staging any point, then use the preallocated buffer.
        let mut previous = 0;
        for &(id, offset, value) in events {
            if ClapId::from_raw(id).is_none() || offset as usize >= il.len()
                || offset < previous || !value.is_finite() {
                return Err("invalid CLAP native parameter event".into());
            }
            previous = offset;
        }
        for &(id, offset, value) in events {
            let id = ClapId::from_raw(id).ok_or("invalid CLAP parameter id")?;
            self.in_events.push(&ParamValueEvent::new(offset, id, Pckn::match_all(), value));
        }
        self.process(il, ir, ol, orr)
    }

    fn automation_params(&mut self) -> Result<Vec<AutomationParam>, String> {
        let Some(ext) = self.exts.params.get().copied().flatten() else { return Ok(vec![]); };
        let handle = self.instance.plugin_handle();
        let mut result = Vec::new();
        let mut buffer = ParamInfoBuffer::new();
        for index in 0..ext.count(&handle) {
            let info = ext.get_info(&handle, index, &mut buffer).ok_or("CLAP parameter metadata failed")?;
            if !info.flags.contains(ParamInfoFlags::IS_AUTOMATABLE)
                || info.flags.contains(ParamInfoFlags::IS_READONLY) { continue; }
            let range = info.max_value - info.min_value;
            let step_count = if info.flags.contains(ParamInfoFlags::IS_STEPPED) {
                if !range.is_finite() || range < 0.0 || range >= i64::MAX as f64
                    || info.min_value.fract() != 0.0 || info.max_value.fract() != 0.0 {
                    return Err("invalid CLAP stepped parameter bounds".into());
                }
                range as i64
            } else { 0 };
            let desc = ParamDesc { id: info.id.get().to_string(), name: String::new(),
                value: ext.get_value(&handle, info.id).ok_or("CLAP parameter value failed")?,
                min: info.min_value, max: info.max_value, unit: String::new(), step_count,
                default_value: None };
            let target = ParamTarget::from_desc(info.id.get(), &desc).map_err(|e| e.to_string())?;
            result.push(AutomationParam { id: desc.id, target });
        }
        Ok(result)
    }

    fn latency(&mut self) -> u64 {
        self.exts.latency.get().copied().flatten().map(|ext| ext.get(&self.instance.plugin_handle()) as u64).unwrap_or(0)
    }
    fn params(&mut self) -> Result<Vec<ParamDesc>, String> {
        let Some(ext) = self.exts.params.get().copied().flatten() else { return Ok(vec![]); };
        let handle = self.instance.plugin_handle();
        let mut out = Vec::new();
        let mut buf = ParamInfoBuffer::new();
        for i in 0..ext.count(&handle) {
            let info = ext.get_info(&handle, i, &mut buf).ok_or("CLAP parameter metadata failed")?;
            let range = info.max_value - info.min_value;
            let step_count = if info.flags.contains(ParamInfoFlags::IS_STEPPED) {
                if !range.is_finite() || range < 0.0 || range >= i64::MAX as f64
                    || info.min_value.fract() != 0.0 || info.max_value.fract() != 0.0 {
                    return Err("invalid CLAP stepped parameter bounds".into());
                }
                range as i64
            } else { 0 };
            out.push(ParamDesc {
                id: info.id.get().to_string(), name: String::from_utf8_lossy(info.name).into_owned(),
                value: ext.get_value(&handle, info.id).ok_or("CLAP parameter value failed")?,
                min: info.min_value, max: info.max_value, unit: String::new(), step_count,
                default_value: None,
            });
        }
        Ok(out)
    }
    fn set_param(&mut self, id: &str, value: f64) -> Result<(), String> {
        let raw: u32 = id.parse().map_err(|_| "bad CLAP parameter id")?;
        let pid = ClapId::from_raw(raw).ok_or("invalid CLAP parameter id")?;
        let ext = self.exts.params.get().copied().flatten().ok_or("CLAP has no parameters")?;
        let (min, max, flags) = {
            let handle = self.instance.plugin_handle();
            let mut buffer = ParamInfoBuffer::new();
            let mut found = None;
            for index in 0..ext.count(&handle) {
                let info = ext.get_info(&handle, index, &mut buffer).ok_or("CLAP parameter metadata failed")?;
                if info.id == pid {
                    found = Some((info.min_value, info.max_value, info.flags));
                    break;
                }
            }
            found.ok_or("unknown CLAP parameter")?
        };
        if flags.contains(ParamInfoFlags::IS_READONLY) { return Err("CLAP parameter is read-only".into()); }
        if !min.is_finite() || !max.is_finite() || min > max
            || !value.is_finite() || !(min..=max).contains(&value) { return Err("CLAP parameter out of range".into()); }
        self.in_events.clear();
        self.in_events.push(&ParamValueEvent::new(0, pid, Pckn::match_all(), value));
        if flags.contains(ParamInfoFlags::REQUIRES_PROCESS) {
            // This control transaction must also apply while playback is
            // stopped, and be included by an immediately following get_state.
            // Such parameters cannot be delivered by flush while active.
            self.stop();
            {
                let mut handle = self.instance.inactive_plugin_handle().ok_or("CLAP deactivation failed")?;
                ext.flush(&mut handle, &InputEvents::from_buffer(&self.in_events),
                    &mut OutputEvents::from_buffer(&mut self.out_events));
            }
            self.in_events.clear();
            self.emit_params();
            self.check_ports()?;
            self.activate()?;
        } else {
            self.flush();
        }
        Ok(())
    }
    fn state(&mut self) -> Result<Vec<u8>, String> {
        let ext = self.exts.state.get().copied().flatten().ok_or("CLAP has no state extension")?;
        let mut bytes = Vec::new();
        ext.save(&self.instance.plugin_handle(), &mut bytes).map_err(|e| format!("{e:?}"))?;
        Ok(bytes)
    }
    fn set_state(&mut self, bytes: &[u8]) -> Result<(), String> {
        let ext = self.exts.state.get().copied().flatten().ok_or("CLAP has no state extension")?;
        ext.load(&self.instance.plugin_handle(), &mut Cursor::new(bytes)).map_err(|e| format!("{e:?}"))?;
        if self.signals.restart.swap(false, Ordering::AcqRel) { self.stop(); self.check_ports()?; self.activate()?; }
        Ok(())
    }
    fn has_editor(&self) -> bool { self.exts.gui.get().is_some_and(Option::is_some) }
    fn open_editor(&mut self, title: &str) -> Result<(), String> {
        if self.gui_created { return Ok(()); }
        let gui = self.exts.gui.get().copied().flatten().ok_or("CLAP has no native GUI")?;
        #[cfg(target_os = "linux")]
        let api = GuiApiType::X11;
        #[cfg(target_os = "macos")]
        let api = GuiApiType::COCOA;
        #[cfg(target_os = "windows")]
        let api = GuiApiType::WIN32;
        let title = CString::new(title.replace('\0', " ")).map_err(|e| e.to_string())?;
        let handle = self.instance.plugin_handle();
        let embedded = GuiConfiguration { api_type: api, is_floating: false };
        let config = if gui.is_api_supported(&handle, embedded) { embedded }
            else { GuiConfiguration { api_type: api, is_floating: true } };
        if !gui.is_api_supported(&handle, config) { return Err("CLAP native GUI API unsupported".into()); }
        gui.create(&handle, config).map_err(|e| format!("{e:?}"))?;
        self.gui_created = true;
        // Retain the parent before set_parent/show. On either failure destroy
        // the plugin GUI before releasing that parent.
        let result = (|| {
            if config.is_floating {
                gui.suggest_title(&handle, &title);
            } else {
                let mut win = crate::window::EditorWindow::open(&title.to_string_lossy())?;
                let size = gui.get_size(&handle).ok_or("CLAP GUI size unavailable")?;
                win.resize(size.width, size.height);
                self.gui_size = (size.width, size.height);
                self.editor = Some(win);
                unsafe { gui.set_parent(&handle, self.editor.as_ref().unwrap().clap_window()) }
                    .map_err(|e| format!("{e:?}"))?;
            }
            gui.show(&handle).map_err(|e| format!("{e:?}"))
        })();
        if result.is_err() { self.destroy_gui(); }
        result
    }
    fn close_editor(&mut self) { self.destroy_gui(); }
    fn editor_open(&self) -> bool { self.gui_created }
    fn reconfigure(&mut self, rate: f64, max: u32) -> Result<(), String> {
        if rate != verdant_plugins::SAMPLE_RATE || max != self.config.max_frames_count {
            return Err("CLAP insert processing contract is fixed".into());
        }
        self.stop();
        self.check_ports()?;
        self.activate()
    }
    fn set_offline(&mut self, on: bool) -> Result<(), String> {
        self.stop();
        let handle = self.instance.plugin_handle();
        if let Some(render) = handle.get_extension::<PluginRender>() {
            if on && render.has_realtime_requirement(&handle) {
                return Err("CLAP plugin requires realtime hardware; offline export unavailable".into());
            }
            render.set(&handle, if on { RenderMode::Offline } else { RenderMode::Realtime })
                .map_err(|e| format!("CLAP render mode: {e:?}"))?;
        }
        self.activate()
    }
    fn service(&mut self) -> Result<(), String> {
        self.service_reactors();
        if self.signals.callback.swap(false, Ordering::AcqRel) { self.instance.call_on_main_thread_callback(); }
        if self.signals.restart.swap(false, Ordering::AcqRel) { self.stop(); self.check_ports()?; self.activate()?; }
        if self.signals.flush.swap(false, Ordering::AcqRel) { self.flush(); }
        if self.signals.closed.swap(false, Ordering::AcqRel) { self.destroy_gui(); }
        if let Some(win) = self.editor.as_mut() {
            if !win.pump() { self.destroy_gui(); return Ok(()); }
            if let Some(gui) = self.exts.gui.get().copied().flatten() {
                let handle = self.instance.plugin_handle();
                let requested = self.signals.resize.swap(0, Ordering::AcqRel);
                if requested != 0 {
                    let size = GuiSize::unpack_from_u64(requested);
                    win.resize(size.width, size.height);
                    self.gui_size = (size.width, size.height);
                } else if win.size() != self.gui_size {
                    let current = win.size();
                    let size = GuiSize { width: current.0, height: current.1 };
                    if gui.can_resize(&handle) {
                        let size = gui.adjust_size(&handle, size).unwrap_or(size);
                        gui.set_size(&handle, size).map_err(|e| format!("GUI resize: {e:?}"))?;
                        self.gui_size = (size.width, size.height);
                    }
                    win.resize(self.gui_size.0, self.gui_size.1);
                }
                match self.signals.visible.swap(0, Ordering::AcqRel) {
                    1 => { gui.show(&handle).map_err(|e| format!("GUI show: {e:?}"))?; win.show(true); }
                    -1 => { gui.hide(&handle).map_err(|e| format!("GUI hide: {e:?}"))?; win.show(false); }
                    _ => {}
                }
            }
        } else if self.gui_created {
            if let Some(gui) = self.exts.gui.get().copied().flatten() {
                let handle = self.instance.plugin_handle();
                match self.signals.visible.swap(0, Ordering::AcqRel) {
                    1 => gui.show(&handle).map_err(|e| format!("GUI show: {e:?}"))?,
                    -1 => gui.hide(&handle).map_err(|e| format!("GUI hide: {e:?}"))?,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
impl Drop for ClapPlugin {
    fn drop(&mut self) { self.destroy_gui(); self.stop(); }
}

pub fn instantiate(path: &str, id: &str, rate: f64, max: u32, events: EventTx)
    -> Result<(Box<dyn HostedPlugin>, InstDesc), PluginError> {
    let entry = unsafe { PluginEntry::load(path) }.map_err(|e| PluginError::Host(format!("CLAP load: {e:?}")))?;
    let host_info = HostInfo::new("Verdant", "Verdant", "https://github.com/OpenGreendam/verdant", env!("CARGO_PKG_VERSION"))
        .map_err(|e| PluginError::Host(e.to_string()))?;
    let exts = Arc::new(Exts::default());
    let signals = Arc::new(Signals::default());
    let shared = Shared { exts: exts.clone(), signals: signals.clone(), events: events.clone() };
    let id_c = CString::new(id).map_err(|e| PluginError::Host(e.to_string()))?;
    let instance = PluginInstance::<ClapHost>::new(|_| shared, |s| Mt {
        shared: s, timers: RefCell::new(BTreeMap::new()), next_timer: Cell::new(0),
        #[cfg(unix)]
        fds: RefCell::new(BTreeMap::new()),
    }, &entry, &id_c, &host_info)
        .map_err(|e| PluginError::Host(format!("CLAP instantiate {id}: {e:?}")))?;
    let mut plugin = ClapPlugin {
        instance, exts, signals, events, audio: None,
        config: PluginAudioConfiguration { sample_rate: rate, min_frames_count: 1, max_frames_count: max },
        in_bufs: [vec![0.0; max as usize], vec![0.0; max as usize]],
        out_bufs: [vec![0.0; max as usize], vec![0.0; max as usize]],
        in_ports: AudioPorts::with_capacity(2, 1), out_ports: AudioPorts::with_capacity(2, 1),
        in_events: EventBuffer::with_capacity(MAX_PARAM_EVENTS), out_events: EventBuffer::with_capacity(128),
        editor: None, gui_created: false, gui_size: (0, 0), steady_time: 0,
    };
    plugin.check_ports().and_then(|_| plugin.activate()).map_err(PluginError::Host)?;
    Ok((Box::new(plugin), InstDesc { id: id.into(), name: id.into() }))
}

pub fn scan(path: &Path) -> Result<ScanOutcome, PluginError> {
    let entry = unsafe { PluginEntry::load(path) }.map_err(|e| PluginError::Host(format!("CLAP load: {e:?}")))?;
    let factory = entry.get_plugin_factory().ok_or_else(|| PluginError::Host("no CLAP plugin factory".into()))?;
    let plugins = factory.plugin_descriptors().map(|d| ScannedPlugin {
        format: "clap".into(),
        id: d.id().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        name: d.name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        vendor: d.vendor().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        version: d.version().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        url: d.url().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        features: d.features().map(|s| s.to_string_lossy().into_owned()).collect(),
        // Descriptors do not prove a native GUI. Instantiate queries it.
        has_editor: false,
    }).collect();
    Ok(ScanOutcome { format: "clap", plugins })
}
