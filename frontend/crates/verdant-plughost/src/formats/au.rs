//! macOS AUv2 stereo effects through the system AudioToolbox API. No SDK
//! binaries are redistributed. Execution and Cocoa UI acceptance require a
//! native macOS main thread; cross-target checking alone is not AU evidence.

#![cfg(all(feature = "au", target_os = "macos"))]

use super::{EventTx, HostedPlugin, InstDesc, ScanOutcome};
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView};
use objc2_foundation::{NSBundle, NSSize, NSString, NSURL};
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::ffi::{c_void, CString};
use std::path::Path;
use verdant_plugins::error::PluginError;
use verdant_plugins::proto::{AutomationParam, ParamDesc, ParamEvent, ParamTarget,
    ScannedPlugin, MAX_PARAM_EVENTS};

type AudioUnit = *mut c_void;
type AudioComponent = *mut c_void;
type Cf = *const c_void;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct ComponentDesc { kind: u32, subtype: u32, manufacturer: u32, flags: u32, mask: u32 }
#[repr(C)]
struct TimeStamp {
    sample_time: f64, host_time: u64, rate_scalar: f64, word_clock_time: u64,
    smpte_time: [u8; 24], flags: u32, reserved: u32,
}
#[repr(C)]
struct AudioBuffer { channels: u32, bytes: u32, data: *mut c_void }
#[repr(C)]
struct BufferList { count: u32, buffers: [AudioBuffer; 2] }
#[repr(C)]
struct StreamFormat {
    rate: f64, format: u32, flags: u32, bytes_per_packet: u32,
    frames_per_packet: u32, bytes_per_frame: u32, channels: u32, bits: u32, reserved: u32,
}
#[repr(C)]
struct ParameterInfo {
    name: [u8; 52], unit_name: Cf, clump: u32, cf_name: Cf, unit: u32,
    min: f32, max: f32, default: f32, flags: u32,
}
/// AudioUnitParameterEvent with its 16-byte immediate/ramp union. All members
/// are 32-bit; immediate stores offset/value in the first two words. The exact
/// layout is the system AUv2 ABI, not an AudioUnitSetParameter control edit.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ScheduledParameter {
    parameter: u32,
    scope: u32,
    element: u32,
    event_type: u32,
    values: [u32; 4],
}
impl ScheduledParameter {
    fn immediate(parameter: u32, offset: u32, value: f32) -> Self {
        Self { parameter, scope: GLOBAL, element: 0, event_type: 1,
            values: [offset, value.to_bits(), 0, 0] }
    }
}

#[repr(C)]
struct RenderCallback {
    function: extern "C" fn(*mut c_void, *mut u32, *const TimeStamp, u32, u32, *mut BufferList) -> i32,
    context: *mut c_void,
}

#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {
    fn AudioComponentFindNext(previous: AudioComponent, desc: *const ComponentDesc) -> AudioComponent;
    fn AudioComponentCopyName(component: AudioComponent, out: *mut Cf) -> i32;
    fn AudioComponentInstanceNew(component: AudioComponent, out: *mut AudioUnit) -> i32;
    fn AudioComponentInstanceDispose(unit: AudioUnit) -> i32;
    fn AudioUnitInitialize(unit: AudioUnit) -> i32;
    fn AudioUnitUninitialize(unit: AudioUnit) -> i32;
    fn AudioUnitRender(unit: AudioUnit, flags: *mut u32, timestamp: *const TimeStamp,
        bus: u32, frames: u32, buffers: *mut BufferList) -> i32;
    fn AudioUnitSetProperty(unit: AudioUnit, property: u32, scope: u32, element: u32,
        data: *const c_void, bytes: u32) -> i32;
    fn AudioUnitGetProperty(unit: AudioUnit, property: u32, scope: u32, element: u32,
        data: *mut c_void, bytes: *mut u32) -> i32;
    fn AudioUnitGetPropertyInfo(unit: AudioUnit, property: u32, scope: u32, element: u32,
        bytes: *mut u32, writable: *mut u8) -> i32;
    fn AudioUnitGetParameter(unit: AudioUnit, id: u32, scope: u32, element: u32, out: *mut f32) -> i32;
    fn AudioUnitSetParameter(unit: AudioUnit, id: u32, scope: u32, element: u32, value: f32, offset: u32) -> i32;
    fn AudioUnitScheduleParameters(unit: AudioUnit, events: *const ScheduledParameter, count: u32) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: Cf);
    fn CFGetTypeID(object: Cf) -> usize;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFPropertyListCreateWithData(allocator: Cf, data: Cf, options: usize, format: *mut isize, error: *mut Cf) -> Cf;
    fn CFPropertyListCreateData(allocator: Cf, plist: Cf, format: isize, options: usize, error: *mut Cf) -> Cf;
    fn CFDataCreate(allocator: Cf, bytes: *const u8, count: isize) -> Cf;
    fn CFDataGetLength(data: Cf) -> isize;
    fn CFDataGetBytePtr(data: Cf) -> *const u8;
    fn CFDictionaryGetValue(dict: Cf, key: Cf) -> Cf;
    fn CFArrayGetCount(array: Cf) -> isize;
    fn CFArrayGetValueAtIndex(array: Cf, index: isize) -> Cf;
    fn CFStringCreateWithCString(allocator: Cf, string: *const i8, encoding: u32) -> Cf;
    fn CFStringGetLength(string: Cf) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(string: Cf, out: *mut i8, bytes: isize, encoding: u32) -> u8;
    fn CFNumberGetValue(number: Cf, kind: isize, out: *mut c_void) -> u8;
}
const GLOBAL: u32 = 0;
const INPUT: u32 = 1;
const OUTPUT: u32 = 2;
const CLASS_INFO: u32 = 0;
const PARAMETER_LIST: u32 = 3;
const PARAMETER_INFO: u32 = 4;
const STREAM_FORMAT: u32 = 8;
const LATENCY: u32 = 12;
const MAXIMUM_FRAMES: u32 = 14;
const SET_RENDER_CALLBACK: u32 = 23;
const COCOA_UI: u32 = 31;
const OFFLINE_RENDER: u32 = 37;
const UTF8: u32 = 0x0800_0100;
const EFFECT: u32 = u32::from_be_bytes(*b"aufx");

struct OwnedCf(Cf);
impl Drop for OwnedCf {
    fn drop(&mut self) { if !self.0.is_null() { unsafe { CFRelease(self.0); } } }
}
fn status(code: i32, operation: &str) -> Result<(), String> {
    if code == 0 { Ok(()) } else { Err(format!("{operation}: OSStatus {code}")) }
}
fn cf_key(key: &str) -> OwnedCf {
    let c = CString::new(key).expect("constant CF dictionary key");
    OwnedCf(unsafe { CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), UTF8) })
}
fn cf_string(value: Cf) -> String {
    if value.is_null() || unsafe { CFGetTypeID(value) != CFStringGetTypeID() } { return String::new(); }
    let cap = unsafe { CFStringGetMaximumSizeForEncoding(CFStringGetLength(value), UTF8) };
    if !(0..=8_388_608).contains(&cap) { return String::new(); }
    let mut bytes = vec![0u8; cap as usize + 1];
    if unsafe { CFStringGetCString(value, bytes.as_mut_ptr().cast(), bytes.len() as isize, UTF8) } == 0 {
        return String::new();
    }
    let end = bytes.iter().position(|&v| v == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
fn dict_value(dict: Cf, key: &str) -> Cf {
    if dict.is_null() || unsafe { CFGetTypeID(dict) != CFDictionaryGetTypeID() } { return std::ptr::null(); }
    let key = cf_key(key);
    unsafe { CFDictionaryGetValue(dict, key.0) }
}
fn read_plist(bytes: &[u8]) -> Result<OwnedCf, String> {
    let data = OwnedCf(unsafe { CFDataCreate(std::ptr::null(), bytes.as_ptr(), bytes.len() as isize) });
    if data.0.is_null() { return Err("CFData allocation failed".into()); }
    let plist = OwnedCf(unsafe { CFPropertyListCreateWithData(std::ptr::null(), data.0, 0, std::ptr::null_mut(), std::ptr::null_mut()) });
    if plist.0.is_null() { return Err("invalid property list".into()); }
    Ok(plist)
}
fn parse_fourcc(s: &str) -> Result<u32, String> {
    if let Some(s) = s.strip_prefix("0x") { return u32::from_str_radix(s, 16).map_err(|e| e.to_string()); }
    if s.len() == 4 { return Ok(u32::from_be_bytes(s.as_bytes().try_into().unwrap())); }
    s.parse().map_err(|_| format!("invalid AU fourcc {s:?}"))
}
fn fourcc(value: u32) -> String { format!("0x{value:08x}") }

struct RenderInput { l: Vec<f32>, r: Vec<f32>, frames: usize }
extern "C" fn pull_input(context: *mut c_void, _: *mut u32, _: *const TimeStamp,
    _: u32, frames: u32, data: *mut BufferList) -> i32 {
    if context.is_null() || data.is_null() { return -50; }
    unsafe {
        let input = &*(context as *const RenderInput);
        let list = &mut *data;
        let n = frames as usize;
        if n > input.frames || list.count != 2 { return -10874; }
        for (buffer, samples) in list.buffers.iter_mut().zip([&input.l, &input.r]) {
            if buffer.data.is_null() {
                buffer.data = samples.as_ptr() as *mut c_void;
            } else {
                if buffer.bytes < frames * 4 { return -50; }
                std::ptr::copy_nonoverlapping(samples.as_ptr(), buffer.data.cast::<f32>(), n);
            }
            buffer.channels = 1;
            buffer.bytes = frames * 4;
        }
        0
    }
}

pub struct AuPlugin {
    unit: AudioUnit,
    initialized: bool,
    max_frames: usize,
    input: Box<UnsafeCell<RenderInput>>,
    sample_time: u64,
    editor: Option<crate::window::EditorWindow>,
    view: Option<Retained<NSView>>,
    factory: Option<Retained<AnyObject>>,
    bundle: Option<Retained<NSBundle>>,
    param_values: BTreeMap<String, f64>,
    scheduled: Vec<ScheduledParameter>,
    events: EventTx,
}
impl AuPlugin {
    fn configure(&mut self) -> Result<(), String> {
        let format = StreamFormat {
            rate: verdant_plugins::SAMPLE_RATE, format: u32::from_be_bytes(*b"lpcm"),
            flags: 1 | 8 | 32, bytes_per_packet: 4, frames_per_packet: 1,
            bytes_per_frame: 4, channels: 2, bits: 32, reserved: 0,
        };
        unsafe {
            for scope in [INPUT, OUTPUT] {
                status(AudioUnitSetProperty(self.unit, STREAM_FORMAT, scope, 0,
                    (&format as *const StreamFormat).cast(), std::mem::size_of::<StreamFormat>() as u32), "AU stereo format")?;
            }
            let max = self.max_frames as u32;
            status(AudioUnitSetProperty(self.unit, MAXIMUM_FRAMES, GLOBAL, 0,
                (&max as *const u32).cast(), 4), "AU maximum frames")?;
            let callback = RenderCallback { function: pull_input, context: self.input.get().cast() };
            status(AudioUnitSetProperty(self.unit, SET_RENDER_CALLBACK, INPUT, 0,
                (&callback as *const RenderCallback).cast(), std::mem::size_of::<RenderCallback>() as u32), "AU render callback")?;
            status(AudioUnitInitialize(self.unit), "AudioUnitInitialize")?;
        }
        self.initialized = true;
        Ok(())
    }
    fn property_size(&self, property: u32) -> Result<u32, String> {
        let mut size = 0;
        let mut writable = 0;
        status(unsafe { AudioUnitGetPropertyInfo(self.unit, property, GLOBAL, 0, &mut size, &mut writable) }, "AU property info")?;
        Ok(size)
    }
    fn cocoa_info(&self) -> Result<Vec<OwnedCf>, String> {
        let mut size = self.property_size(COCOA_UI)?;
        let word = std::mem::size_of::<Cf>();
        if (size as usize) < 2 * word || size as usize % word != 0 {
            return Err("invalid AU Cocoa view descriptor".into());
        }
        let mut refs: Vec<Cf> = vec![std::ptr::null(); size as usize / word];
        status(unsafe { AudioUnitGetProperty(self.unit, COCOA_UI, GLOBAL, 0, refs.as_mut_ptr().cast(), &mut size) }, "AU Cocoa view info")?;
        let refs: Vec<_> = refs.into_iter().map(OwnedCf).collect();
        if refs[0].0.is_null() || refs[1].0.is_null() { return Err("empty AU Cocoa view descriptor".into()); }
        Ok(refs)
    }
}
impl HostedPlugin for AuPlugin {
    fn process(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32]) -> Result<(), String> {
        let n = il.len();
        if !self.initialized || n == 0 || n > self.max_frames || ir.len() != n || ol.len() != n || orr.len() != n {
            return Err("invalid/inactive AU stereo block".into());
        }
        unsafe {
            let input = &mut *self.input.get();
            input.l[..n].copy_from_slice(il);
            input.r[..n].copy_from_slice(ir);
            input.frames = n;
        }
        let mut list = BufferList { count: 2, buffers: [
            AudioBuffer { channels: 1, bytes: n as u32 * 4, data: ol.as_mut_ptr().cast() },
            AudioBuffer { channels: 1, bytes: n as u32 * 4, data: orr.as_mut_ptr().cast() },
        ] };
        let stamp = TimeStamp { sample_time: self.sample_time as f64, host_time: 0,
            rate_scalar: 0.0, word_clock_time: 0, smpte_time: [0; 24], flags: 1, reserved: 0 };
        let mut flags = 0;
        status(unsafe { AudioUnitRender(self.unit, &mut flags, &stamp, 0, n as u32, &mut list) }, "AudioUnitRender")?;
        // AU may substitute its own output buffers. Copy them while the unit
        // still owns valid render data, before the next AudioUnitRender call.
        if list.count != 2 { return Err("AU changed stereo output layout".into()); }
        for (buffer, output) in list.buffers.iter().zip([ol, orr]) {
            if buffer.data.is_null() || buffer.bytes < n as u32 * 4 || buffer.channels != 1 {
                return Err("AU output buffer invalid".into());
            }
            if buffer.data != output.as_mut_ptr().cast() {
                unsafe { std::ptr::copy(buffer.data.cast::<f32>(), output.as_mut_ptr(), n); }
            }
        }
        self.sample_time = self.sample_time.wrapping_add(n as u64);
        Ok(())
    }
    fn process_events(&mut self, il: &[f32], ir: &[f32], ol: &mut [f32], orr: &mut [f32], events: &[ParamEvent]) -> Result<(), String> {
        if events.len() > MAX_PARAM_EVENTS || !self.initialized || il.is_empty()
            || il.len() > self.max_frames || ir.len() != il.len() || ol.len() != il.len() || orr.len() != il.len() {
            return Err("invalid AU native automation block".into());
        }
        let mut previous = 0;
        for &(_, offset, value) in events {
            if offset as usize >= il.len() || offset < previous || !value.is_finite() || !(value as f32).is_finite() {
                return Err("invalid AU native parameter event".into());
            }
            previous = offset;
        }
        for (slot, &(id, offset, value)) in self.scheduled.iter_mut().zip(events) {
            *slot = ScheduledParameter::immediate(id, offset, value as f32);
        }
        if !events.is_empty() {
            // Schedule relative to the immediately following native render.
            // Unsupported native scheduling returns an error and aborts this
            // host/export; it never becomes successful unautomated audio.
            status(unsafe { AudioUnitScheduleParameters(self.unit, self.scheduled.as_ptr(), events.len() as u32) },
                "AudioUnitScheduleParameters")?;
        }
        self.process(il, ir, ol, orr)
    }

    fn automation_params(&mut self) -> Result<Vec<AutomationParam>, String> {
        let mut result = Vec::new();
        for mut desc in self.params()? {
            let id: u32 = desc.id.parse().map_err(|_| "invalid AU parameter id")?;
            let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
            let mut bytes = std::mem::size_of::<ParameterInfo>() as u32;
            status(unsafe { AudioUnitGetProperty(self.unit, PARAMETER_INFO, GLOBAL, id,
                (&mut info as *mut ParameterInfo).cast(), &mut bytes) }, "AU automation metadata")?;
            if info.flags & (1 << 4) != 0 && !info.cf_name.is_null() { unsafe { CFRelease(info.cf_name); } }
            // kAudioUnitParameterFlag_IsWritable / NonRealTime. Global element
            // zero is the existing public parameter identity for this host.
            if info.flags & (1 << 31) == 0 || info.flags & (1 << 24) != 0 { continue; }
            if matches!(info.unit, 1 | 2) { // Indexed / Boolean native units.
                let range = desc.max - desc.min;
                if !range.is_finite() || range < 0.0 || range >= i64::MAX as f64
                    || desc.min.fract() != 0.0 || desc.max.fract() != 0.0 {
                    return Err("invalid AU discrete parameter bounds".into());
                }
                desc.step_count = range as i64;
            }
            let target = ParamTarget::from_desc(id, &desc).map_err(|e| e.to_string())?;
            result.push(AutomationParam { id: desc.id, target });
        }
        Ok(result)
    }

    fn latency(&mut self) -> u64 {
        let mut seconds = 0.0f64;
        let mut size = 8;
        if unsafe { AudioUnitGetProperty(self.unit, LATENCY, GLOBAL, 0,
            (&mut seconds as *mut f64).cast(), &mut size) } == 0 && seconds.is_finite() && seconds >= 0.0 {
            (seconds * verdant_plugins::SAMPLE_RATE).round() as u64
        } else { 0 }
    }
    fn params(&mut self) -> Result<Vec<ParamDesc>, String> {
        let mut size = self.property_size(PARAMETER_LIST)?;
        if size == 0 { return Ok(vec![]); }
        if size % 4 != 0 { return Err("AU parameter list length invalid".into()); }
        let mut ids = vec![0u32; size as usize / 4];
        status(unsafe { AudioUnitGetProperty(self.unit, PARAMETER_LIST, GLOBAL, 0,
            ids.as_mut_ptr().cast(), &mut size) }, "AU parameter list")?;
        ids.truncate(size as usize / 4);
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
            let mut bytes = std::mem::size_of::<ParameterInfo>() as u32;
            status(unsafe { AudioUnitGetProperty(self.unit, PARAMETER_INFO, GLOBAL, id,
                (&mut info as *mut ParameterInfo).cast(), &mut bytes) }, "AU parameter metadata")?;
            let mut value = 0.0f32;
            let get = status(unsafe { AudioUnitGetParameter(self.unit, id, GLOBAL, 0, &mut value) }, "AU parameter value");
            let name = if info.flags & (1 << 27) != 0 && !info.cf_name.is_null() { cf_string(info.cf_name) }
                else {
                    let end = info.name.iter().position(|&b| b == 0).unwrap_or(info.name.len());
                    String::from_utf8_lossy(&info.name[..end]).into_owned()
                };
            // CFNameRelease explicitly transfers the parameter name to us.
            if info.flags & (1 << 4) != 0 && !info.cf_name.is_null() { unsafe { CFRelease(info.cf_name); } }
            get?;
            result.push(ParamDesc { id: id.to_string(), name, value: value as f64,
                min: info.min as f64, max: info.max as f64, unit: String::new(), step_count: 0,
                default_value: Some(info.default as f64) });
        }
        Ok(result)
    }
    fn set_param(&mut self, id: &str, value: f64) -> Result<(), String> {
        let parameter = self.params()?.into_iter().find(|p| p.id == id).ok_or("unknown AU parameter")?;
        if !value.is_finite() || !(parameter.min..=parameter.max).contains(&value) { return Err("AU parameter out of range".into()); }
        let id: u32 = id.parse().map_err(|_| "invalid AU parameter id")?;
        status(unsafe { AudioUnitSetParameter(self.unit, id, GLOBAL, 0, value as f32, 0) }, "AudioUnitSetParameter")
    }
    fn state(&mut self) -> Result<Vec<u8>, String> {
        let mut value = std::ptr::null();
        let mut bytes = std::mem::size_of::<Cf>() as u32;
        status(unsafe { AudioUnitGetProperty(self.unit, CLASS_INFO, GLOBAL, 0,
            (&mut value as *mut Cf).cast(), &mut bytes) }, "AU ClassInfo")?;
        let value = OwnedCf(value);
        if value.0.is_null() { return Err("AU ClassInfo unavailable".into()); }
        let data = OwnedCf(unsafe { CFPropertyListCreateData(std::ptr::null(), value.0, 200, 0, std::ptr::null_mut()) });
        if data.0.is_null() { return Err("AU ClassInfo serialization failed".into()); }
        let n = unsafe { CFDataGetLength(data.0) };
        if n < 0 { return Err("AU ClassInfo data length invalid".into()); }
        Ok(unsafe { std::slice::from_raw_parts(CFDataGetBytePtr(data.0), n as usize).to_vec() })
    }
    fn set_state(&mut self, data: &[u8]) -> Result<(), String> {
        let plist = read_plist(data)?;
        status(unsafe { AudioUnitSetProperty(self.unit, CLASS_INFO, GLOBAL, 0,
            (&plist.0 as *const Cf).cast(), std::mem::size_of::<Cf>() as u32) }, "AU ClassInfo restore")
    }
    fn has_editor(&self) -> bool { self.property_size(COCOA_UI).is_ok_and(|s| s as usize >= 2 * std::mem::size_of::<Cf>()) }
    fn open_editor(&mut self, title: &str) -> Result<(), String> {
        if self.editor.is_some() { return Ok(()); }
        let info = self.cocoa_info()?;
        let url = unsafe { &*(info[0].0 as *const NSURL) };
        let bundle = NSBundle::bundleWithURL(url).ok_or("AU view bundle unavailable")?;
        if !unsafe { bundle.load() } { return Err("AU view bundle failed to load".into()); }
        let name = NSString::from_str(&cf_string(info[1].0));
        let class = bundle.classNamed(&name).ok_or("AU Cocoa factory class unavailable")?;
        let factory: Option<Retained<AnyObject>> = unsafe { msg_send![class, new] };
        let factory = factory.ok_or("AU Cocoa factory initialization failed")?;
        // AUCocoaUIBase defines uiViewForAudioUnit:withSize:, not a special
        // NSView initializer. Retain its autoreleased view explicitly.
        let view: Option<Retained<NSView>> = unsafe { msg_send![&*factory,
            uiViewForAudioUnit: self.unit, withSize: NSSize::new(480.0, 320.0)] };
        let view = view.ok_or("AU Cocoa factory returned no view")?;
        let size = view.frame().size;
        let mut window = crate::window::EditorWindow::open(title)?;
        window.resize(size.width.max(1.0) as u32, size.height.max(1.0) as u32);
        let crate::window::NativeHandle::Cocoa(parent) = window.native_handle() else { return Err("AU editor needs Cocoa".into()); };
        let parent = unsafe { &*(parent as *const NSView) };
        view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
        parent.addSubview(&view);
        self.view = Some(view);
        self.factory = Some(factory);
        self.bundle = Some(bundle);
        self.editor = Some(window);
        Ok(())
    }
    fn close_editor(&mut self) {
        if let Some(view) = self.view.take() { view.removeFromSuperview(); }
        self.factory = None;
        self.editor = None;
        self.bundle = None;
    }
    fn editor_open(&self) -> bool { self.editor.is_some() }
    fn reconfigure(&mut self, rate: f64, max: u32) -> Result<(), String> {
        if rate != verdant_plugins::SAMPLE_RATE || max as usize != self.max_frames { return Err("AU processing contract is fixed".into()); }
        if self.initialized {
            status(unsafe { AudioUnitUninitialize(self.unit) }, "AudioUnitUninitialize")?;
            self.initialized = false;
        }
        self.configure()
    }
    fn set_offline(&mut self, on: bool) -> Result<(), String> {
        let mut bytes = 0;
        let mut writable = 0;
        let supported = unsafe { AudioUnitGetPropertyInfo(self.unit, OFFLINE_RENDER, GLOBAL, 0, &mut bytes, &mut writable) } == 0;
        // This optional hint does not determine whether an AU can render PCM
        // synchronously. Units without it use their normal native DSP.
        if !supported { return Ok(()); }
        if writable == 0 || bytes != 4 { return Err("AU offline property is not writable UInt32".into()); }
        if self.initialized {
            status(unsafe { AudioUnitUninitialize(self.unit) }, "AudioUnitUninitialize")?;
            self.initialized = false;
        }
        let value = u32::from(on);
        status(unsafe { AudioUnitSetProperty(self.unit, OFFLINE_RENDER, GLOBAL, 0,
            (&value as *const u32).cast(), 4) }, "AU offline render")?;
        self.configure()
    }
    fn service(&mut self) -> Result<(), String> {
        if let Some(window) = self.editor.as_mut() {
            if !window.pump() { self.close_editor(); }
            else if let Some(view) = self.view.as_ref() {
                let (w, h) = window.size();
                let size = view.frame().size;
                // A custom AU view may request a different size directly.
                if size.width as u32 != w || size.height as u32 != h {
                    window.resize(size.width.max(1.0) as u32, size.height.max(1.0) as u32);
                }
            }
        }
        for parameter in self.params()? {
            let old = self.param_values.insert(parameter.id.clone(), parameter.value);
            if old.is_some_and(|old| old != parameter.value) {
                let mut data = serde_json::Map::new();
                data.insert("param_id".into(), parameter.id.into());
                data.insert("value".into(), parameter.value.into());
                let _ = self.events.send(("param_changed".into(), data));
            }
        }
        Ok(())
    }
}
impl Drop for AuPlugin {
    fn drop(&mut self) {
        self.close_editor();
        if !self.unit.is_null() { unsafe {
            if self.initialized { AudioUnitUninitialize(self.unit); }
            AudioComponentInstanceDispose(self.unit);
        } }
    }
}

pub fn instantiate(_path: &str, id: &str, rate: f64, max: u32, events: EventTx)
    -> Result<(Box<dyn HostedPlugin>, InstDesc), PluginError> {
    if rate != verdant_plugins::SAMPLE_RATE || max == 0 { return Err(PluginError::Host("AU requires 48 kHz".into())); }
    let parts: Vec<_> = id.split(':').collect();
    if parts.len() != 3 { return Err(PluginError::Host("AU id must be type:subtype:manufacturer".into())); }
    let desc = ComponentDesc {
        kind: parse_fourcc(parts[0]).map_err(PluginError::Host)?,
        subtype: parse_fourcc(parts[1]).map_err(PluginError::Host)?,
        manufacturer: parse_fourcc(parts[2]).map_err(PluginError::Host)?, flags: 0, mask: 0,
    };
    if desc.kind != EFFECT { return Err(PluginError::Host("AU insert requires an aufx effect".into())); }
    let component = unsafe { AudioComponentFindNext(std::ptr::null_mut(), &desc) };
    if component.is_null() { return Err(PluginError::Host(format!("AU component not registered: {id}"))); }
    let mut unit = std::ptr::null_mut();
    status(unsafe { AudioComponentInstanceNew(component, &mut unit) }, "AudioComponentInstanceNew").map_err(PluginError::Host)?;
    if unit.is_null() { return Err(PluginError::Host("AU returned no instance".into())); }
    let mut plugin = AuPlugin {
        unit, initialized: false, max_frames: max as usize,
        input: Box::new(UnsafeCell::new(RenderInput { l: vec![0.0; max as usize], r: vec![0.0; max as usize], frames: 0 })),
        sample_time: 0, editor: None, view: None, factory: None, bundle: None,
        param_values: BTreeMap::new(),
        scheduled: vec![ScheduledParameter::default(); MAX_PARAM_EVENTS], events,
    };
    plugin.configure().map_err(PluginError::Host)?;
    let mut name = std::ptr::null();
    let name = if unsafe { AudioComponentCopyName(component, &mut name) } == 0 {
        let name = OwnedCf(name); cf_string(name.0)
    } else { id.to_owned() };
    Ok((Box::new(plugin), InstDesc { id: id.into(), name }))
}

pub fn scan(path: &Path) -> Result<ScanOutcome, PluginError> {
    let bytes = std::fs::read(path.join("Contents/Info.plist"))?;
    let plist = read_plist(&bytes).map_err(PluginError::Host)?;
    let components = dict_value(plist.0, "AudioComponents");
    if components.is_null() || unsafe { CFGetTypeID(components) != CFArrayGetTypeID() } {
        return Err(PluginError::Host("AU bundle has no AudioComponents array".into()));
    }
    let mut plugins = Vec::new();
    for index in 0..unsafe { CFArrayGetCount(components) } {
        let entry = unsafe { CFArrayGetValueAtIndex(components, index) };
        let get = |key| cf_string(dict_value(entry, key));
        let kind = parse_fourcc(&get("type")).map_err(PluginError::Host)?;
        if kind != EFFECT { continue; }
        let subtype = parse_fourcc(&get("subtype")).map_err(PluginError::Host)?;
        let manufacturer = parse_fourcc(&get("manufacturer")).map_err(PluginError::Host)?;
        let version = dict_value(entry, "version");
        let mut number = 0i32;
        let version = if !version.is_null() && unsafe { CFGetTypeID(version) == CFNumberGetTypeID()
            && CFNumberGetValue(version, 3, (&mut number as *mut i32).cast()) != 0 } {
            number.to_string()
        } else { cf_string(version) };
        let id = format!("{}:{}:{}", fourcc(kind), fourcc(subtype), fourcc(manufacturer));
        let name = get("name");
        plugins.push(ScannedPlugin {
            format: "au".into(), id: id.clone(), name: if name.is_empty() { id } else { name },
            vendor: String::new(), version, url: String::new(), features: vec!["audio-effect".into()], has_editor: false,
        });
    }
    Ok(ScanOutcome { format: "au", plugins })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_schedule_uses_auv2_immediate_event_layout() {
        assert_eq!(std::mem::size_of::<ScheduledParameter>(), 32);
        assert_eq!(std::mem::align_of::<ScheduledParameter>(), 4);
        let event = ScheduledParameter::immediate(43, 255, 0.75);
        assert_eq!(event.parameter, 43);
        assert_eq!(event.scope, GLOBAL);
        assert_eq!(event.element, 0);
        assert_eq!(event.event_type, 1);
        assert_eq!(event.values, [255, 0.75f32.to_bits(), 0, 0]);
    }
}
