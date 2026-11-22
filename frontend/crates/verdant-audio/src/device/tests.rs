//! Device-state tests use the production worker/ownership/recovery path with a
//! manual backend. They do not enumerate or open physical audio hardware.

use super::*;
use crate::engine::{ChannelSettings, Clip, Processor, Timeline};
use std::sync::atomic::AtomicUsize;

#[derive(Default)]
struct Harness {
    live: AtomicUsize,
    max_live: AtomicUsize,
    builds: AtomicUsize,
    fail_prepare: AtomicUsize,
    fail_build: AtomicUsize,
    fail_play: AtomicUsize,
    rate_override: AtomicUsize,
    buffer_override: AtomicUsize,
    callback_override: AtomicUsize,
    signals: Mutex<Vec<Arc<FaultSignal>>>,
    samples: Mutex<Vec<(f32, f32)>>,
    drop_threads: Mutex<Vec<String>>,
}

struct ManualBackend(Arc<Harness>);

fn fail_next(count: &AtomicUsize) -> bool {
    count.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_sub(1)).is_ok()
}

impl Backend for ManualBackend {
    fn devices(&self) -> Result<Vec<AudioDeviceInfo>, DeviceError> {
        Ok(["first", "second"].into_iter().map(|id| AudioDeviceInfo {
            id: Some(id.into()), name: "reported device description".into(), is_default: id == "first",
            sample_rates: vec![44_100, 48_000], channels: vec![2], formats: vec![SampleFmt::F32, SampleFmt::I16],
        }).collect())
    }

    fn prepare(&self, selection: &DeviceSelection, rate: u32) -> Result<PreparedOutput, DeviceError> {
        if fail_next(&self.0.fail_prepare) {
            return Err(DeviceError::new(DeviceErrorKind::NoDevice, "test device removed"));
        }
        let id = match selection {
            DeviceSelection::Default => "first".to_string(),
            DeviceSelection::Device(id) if id == "first" || id == "second" => id.clone(),
            _ => return Err(DeviceError::new(DeviceErrorKind::DeviceNotFound, "unknown test device")),
        };
        let numeric = id == "second";
        let harness = self.0.clone();
        let device_rate = match harness.rate_override.load(Ordering::Acquire) { 0 => if numeric { 44_100 } else { 48_000 }, rate => rate as u32 };
        let buffer_frames = match harness.buffer_override.load(Ordering::Acquire) { 0 => 64, frames => frames as u32 };
        Ok(PreparedOutput {
            info: OutputInfo {
                kind: OutputKind::Device { name: "reported device description".into(), channels: 2, format: if numeric { SampleFmt::I16 } else { SampleFmt::F32 } },
                sample_rate: rate, device_sample_rate: device_rate, device_id: Some(id), buffer_frames: Some(buffer_frames),
            },
            build: Box::new(move |lease, fault| {
                if fail_next(&harness.fail_build) {
                    return Err(DeviceError::new(DeviceErrorKind::Open, "test stream construction failed"));
                }
                let live = harness.live.fetch_add(1, Ordering::AcqRel) + 1;
                harness.max_live.fetch_max(live, Ordering::Relaxed);
                harness.builds.fetch_add(1, Ordering::Relaxed);
                harness.signals.lock().unwrap().push(fault);
                Ok(Box::new(ManualDriver { lease, harness, numeric }))
            }),
        })
    }
}

struct ManualDriver {
    lease: CallbackLease,
    harness: Arc<Harness>,
    numeric: bool,
}

impl Driver for ManualDriver {
    fn play(&mut self) -> Result<(), DeviceError> {
        if fail_next(&self.harness.fail_play) {
            return Err(DeviceError::new(DeviceErrorKind::Play, "test stream start failed"));
        }
        // Deterministic callback: no timer and no hardware. The different-rate
        // device takes the exact numeric callback path used by cpal.
        let frames = match self.harness.callback_override.load(Ordering::Acquire) { 0 => 64, frames => frames };
        let sample = if self.numeric {
            let mut out = vec![0i16; frames * 2];
            self.lease.numeric(&mut out, 2);
            (out[0] as f32 / 32768.0, out[1] as f32 / 32768.0)
        } else {
            let mut out = vec![0.0; frames * 2];
            self.lease.f32(&mut out, 2);
            (out[0], out[1])
        };
        self.harness.samples.lock().unwrap().push(sample);
        Ok(())
    }
}

impl Drop for ManualDriver {
    fn drop(&mut self) {
        self.harness.live.fetch_sub(1, Ordering::AcqRel);
        self.harness.drop_threads.lock().unwrap().push(std::thread::current().name().unwrap_or("unnamed").into());
    }
}

fn open() -> (OutputEngine, Control, Arc<Harness>) {
    let harness = Arc::new(Harness::default());
    let (engine, control) = OutputEngine::open_backend(DeviceSelection::Default, 48_000, "test silent clock".into(),
        Box::new(ManualBackend(harness.clone())), [Duration::from_millis(2); 3]);
    (engine, control, harness)
}

fn barrier(engine: &OutputEngine) {
    // Enumeration is queued after selection; its reply proves the preceding
    // selection completed, without requiring a timing guess.
    assert_eq!(engine.devices().unwrap().len(), 2);
}

fn await_events(engine: &OutputEngine, predicate: impl Fn(&DeviceEvent) -> bool) -> Vec<DeviceEvent> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut events = Vec::new();
    loop {
        events.extend(engine.drain_events());
        if events.iter().any(&predicate) { return events; }
        assert!(Instant::now() < deadline, "missing expected device event; status {:?}, events {events:?}", engine.device_status());
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn enumerates_ids_and_rejects_an_absent_choice_without_replacing_the_clock() {
    let (engine, control, harness) = open();
    let devices = engine.devices().unwrap();
    assert_eq!(devices[0].name, "reported device description");
    assert_eq!(devices[0].name, devices[1].name);
    assert_ne!(devices[0].id, devices[1].id, "duplicate names still have distinct selectors");
    engine.drain_events();
    engine.select(DeviceSelection::Device("missing".into()));
    barrier(&engine);
    assert_eq!(harness.builds.load(Ordering::Relaxed), 1);
    assert_eq!(harness.live.load(Ordering::Relaxed), 1);
    assert_eq!(control.position(), 0);
    assert!(matches!(engine.device_status(), DeviceStatus::Running(_)));
    assert!(engine.drain_events().iter().any(|e| matches!(e, DeviceEvent::Rejected(DeviceError { kind: DeviceErrorKind::DeviceNotFound, .. }))));
    engine.shutdown();
    assert_eq!(harness.live.load(Ordering::Relaxed), 0);
}

struct InsertProbe { calls: Arc<AtomicUsize>, drops: Arc<AtomicUsize> }
impl Processor for InsertProbe {
    fn name(&self) -> &str { "device-state probe" }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.calls.fetch_add(1, Ordering::Relaxed);
        for sample in left.iter_mut().chain(right.iter_mut()) { *sample *= 0.5; }
    }
}
impl Drop for InsertProbe {
    fn drop(&mut self) { self.drops.fetch_add(1, Ordering::Relaxed); }
}

#[test]
fn reselect_preserves_the_transport_timeline_mixer_and_insert_instance_across_rates() {
    let (engine, mut control, harness) = open();
    let calls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.4; 48_000].into(), gain: 1.0, track: 0 }]));
    control.set_channel(0, ChannelSettings { gain: 0.5, pan: -1.0, mute: false, solo: false });
    assert!(control.set_chain(0, vec![Box::new(InsertProbe { calls: calls.clone(), drops: drops.clone() })]));
    control.seek(1000);
    control.play();
    engine.select(DeviceSelection::Device("first".into()));
    barrier(&engine);
    assert_eq!(control.position(), 1064);
    assert_eq!(control.transport(), Transport::Playing);
    assert_eq!(*harness.samples.lock().unwrap().last().unwrap(), (0.1, 0.0));
    engine.select(DeviceSelection::Device("second".into()));
    barrier(&engine);
    assert!(control.position() > 1064, "continues the same timeline at its fixed processing rate");
    assert_eq!(control.shared.sample_rate.load(Ordering::Acquire), 48_000);
    assert_eq!(engine.output_info().sample_rate, 48_000);
    assert_eq!(engine.output_info().device_sample_rate, 44_100);
    assert_eq!(engine.output_info().device_id.as_deref(), Some("second"));
    assert!(calls.load(Ordering::Relaxed) >= 2);
    assert_eq!(drops.load(Ordering::Relaxed), 0, "insert state is not recreated on reselect");
    assert_eq!(harness.max_live.load(Ordering::Relaxed), 1);
    engine.shutdown();
    assert_eq!(drops.load(Ordering::Relaxed), 1);
    assert!(harness.drop_threads.lock().unwrap().iter().all(|name| name == "verdant-audio-device"));
}

#[test]
fn stream_loss_retries_and_ignores_errors_from_a_retired_stream() {
    let (engine, _control, harness) = open();
    engine.drain_events();
    let stale = harness.signals.lock().unwrap()[0].clone();
    stale.fatal.store(1, Ordering::Release);
    let events = await_events(&engine, |e| matches!(e, DeviceEvent::Recovered(_)));
    assert!(events.iter().any(|e| matches!(e, DeviceEvent::DeviceLost { .. })));
    assert_eq!(harness.builds.load(Ordering::Relaxed), 2);
    assert_eq!(harness.max_live.load(Ordering::Relaxed), 1);
    stale.fatal.store(1, Ordering::Release);
    std::thread::sleep(POLL * 3);
    barrier(&engine);
    assert_eq!(harness.builds.load(Ordering::Relaxed), 2, "retired generations cannot kill a newer binding");
}

#[test]
fn exhausted_recovery_is_typed_silent_fallback_and_can_be_reselected() {
    let (engine, _control, harness) = open();
    engine.drain_events();
    harness.fail_prepare.store(3, Ordering::Release);
    harness.signals.lock().unwrap()[0].fatal.store(1, Ordering::Release);
    await_events(&engine, |e| matches!(e, DeviceEvent::RecoveryFailed { .. }));
    // The event precedes the fallback's completion; the queued enumeration
    // waits for that completion on the same worker.
    barrier(&engine);
    assert!(matches!(engine.output_info().kind, OutputKind::SilentClock { .. }));
    assert_eq!(harness.fail_prepare.load(Ordering::Acquire), 0, "exactly three recovery attempts");
    engine.select(DeviceSelection::Default);
    barrier(&engine);
    assert!(matches!(engine.output_info().kind, OutputKind::Device { .. }));
    assert_eq!(harness.max_live.load(Ordering::Relaxed), 1);
}

#[test]
fn build_and_play_failures_return_the_renderer_for_fallback_and_later_reselect() {
    for failure in ["build", "play"] {
        let (engine, mut control, harness) = open();
        control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.2; 48_000].into(), gain: 1.0, track: 0 }]));
        control.seek(500);
        if failure == "build" { harness.fail_build.store(1, Ordering::Release); }
        else { harness.fail_play.store(1, Ordering::Release); }
        engine.select(DeviceSelection::Default);
        barrier(&engine);
        assert!(matches!(engine.output_info().kind, OutputKind::SilentClock { .. }));
        engine.select(DeviceSelection::Default);
        barrier(&engine);
        assert_eq!(control.position(), 500, "paused seek survives failure, fallback and reselect");
        assert_eq!(control.transport(), Transport::Paused);
        assert!(matches!(engine.output_info().kind, OutputKind::Device { .. }));
        assert_eq!(harness.max_live.load(Ordering::Relaxed), 1);
    }
}

#[test]
fn xrun_is_counted_without_rebuilding_a_working_stream() {
    let (engine, control, harness) = open();
    engine.drain_events();
    harness.signals.lock().unwrap()[0].xruns.store(2, Ordering::Release);
    await_events(&engine, |e| matches!(e, DeviceEvent::CallbackError { .. }));
    assert_eq!(control.shared.underruns.load(Ordering::Acquire), 2);
    assert_eq!(harness.builds.load(Ordering::Acquire), 1);
}

#[test]
fn automatic_route_and_realtime_notices_preserve_the_active_stream() {
    let (engine, _control, harness) = open();
    engine.drain_events();
    let signal = harness.signals.lock().unwrap()[0].clone();
    signal.report(cpal::Error::new(cpal::ErrorKind::DeviceChanged));
    signal.report(cpal::Error::new(cpal::ErrorKind::RealtimeDenied));
    let events = await_events(&engine, |event| matches!(event, DeviceEvent::RealtimeDenied));
    assert!(events.iter().any(|event| matches!(event, DeviceEvent::RouteChanged)));
    assert!(!events.iter().any(|event| matches!(event, DeviceEvent::DeviceLost { .. } | DeviceEvent::Recovered(_))));
    assert!(matches!(engine.device_status(), DeviceStatus::Running(_)));
    assert_eq!(harness.builds.load(Ordering::Acquire), 1);
    assert_eq!(harness.live.load(Ordering::Acquire), 1);
}

#[test]
fn selecting_a_device_joins_the_previous_silent_clock_and_shutdown_joins_the_last() {
    let (engine, mut control, harness) = open();
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.0; 480_000].into(), gain: 1.0, track: 0 }]));
    control.play();
    for _ in 0..3 {
        let before = control.shared.callbacks.load(Ordering::Acquire);
        engine.select(DeviceSelection::Silent);
        barrier(&engine);
        engine.select(DeviceSelection::Silent);
        barrier(&engine);
        let deadline = Instant::now() + Duration::from_secs(1);
        while control.shared.callbacks.load(Ordering::Acquire) <= before + 1 {
            assert!(Instant::now() < deadline, "silent clock did not start");
            std::thread::sleep(Duration::from_millis(2));
        }
        engine.select(DeviceSelection::Default);
        barrier(&engine);
        let retired = control.shared.callbacks.load(Ordering::Acquire);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(control.shared.callbacks.load(Ordering::Acquire), retired, "retired silent clock must not keep advancing transport");
    }
    let shared = control.shared.clone();
    engine.shutdown();
    let stopped = shared.callbacks.load(Ordering::Acquire);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(shared.callbacks.load(Ordering::Acquire), stopped);
    assert_eq!(harness.live.load(Ordering::Acquire), 0);
}

#[test]
fn convenience_control_owns_its_silent_clock() {
    let (control, info) = open_silent(8_000, "owned test clock".into());
    assert_eq!(info.sample_rate, 8_000);
    let shared = control.shared.clone();
    drop(control);
    let stopped = shared.callbacks.load(Ordering::Acquire);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(shared.callbacks.load(Ordering::Acquire), stopped);
}

#[test]
fn unsupported_rate_and_buffer_preparation_leave_the_previous_stream_running() {
    let (engine, control, harness) = open();
    engine.drain_events();
    for (rate, frames) in [(24_000, 64), (48_000, MAX_BLOCK + 1)] {
        harness.rate_override.store(rate, Ordering::Release);
        harness.buffer_override.store(frames, Ordering::Release);
        engine.select(DeviceSelection::Device("second".into()));
        barrier(&engine);
        assert!(engine.drain_events().iter().any(|event| matches!(event,
            DeviceEvent::Rejected(DeviceError { kind: DeviceErrorKind::UnsupportedConfig, .. }))));
        assert_eq!(engine.output_info().device_id.as_deref(), Some("first"));
        assert!(matches!(engine.device_status(), DeviceStatus::Running(_)));
        assert_eq!(harness.builds.load(Ordering::Acquire), 1);
        assert_eq!(harness.live.load(Ordering::Acquire), 1);
        assert_eq!(control.position(), 0);
    }
    harness.rate_override.store(0, Ordering::Release);
    harness.buffer_override.store(0, Ordering::Release);
    engine.select(DeviceSelection::Device("second".into()));
    barrier(&engine);
    assert_eq!(engine.output_info().device_id.as_deref(), Some("second"));
    assert_eq!(harness.builds.load(Ordering::Acquire), 2);
    assert_eq!(harness.max_live.load(Ordering::Acquire), 1);
}

#[test]
fn an_unadvertised_oversized_callback_reports_rejection_and_enters_an_explicit_source_clock() {
    let (engine, control, harness) = open();
    engine.drain_events();
    harness.callback_override.store(MAX_BLOCK + 1, Ordering::Release);
    engine.select(DeviceSelection::Device("second".into()));
    let events = await_events(&engine, |event| matches!(event,
        DeviceEvent::OutputChanged(OutputInfo { kind: OutputKind::SilentClock { .. }, .. })));
    assert!(events.iter().any(|event| matches!(event,
        DeviceEvent::Rejected(DeviceError { kind: DeviceErrorKind::UnsupportedConfig, message })
            if message.contains("8193") && message.contains("maximum is 64"))));
    assert!(!events.iter().any(|event| matches!(event, DeviceEvent::DeviceLost { .. } | DeviceEvent::Recovered(_))));
    assert!(matches!(engine.output_info().kind, OutputKind::SilentClock { .. }));
    assert_eq!(engine.output_info().device_sample_rate, 48_000);
    assert_eq!(control.position(), 0);
    assert_eq!(control.shared.underruns.load(Ordering::Acquire), 0);
    assert_eq!(*harness.samples.lock().unwrap().last().unwrap(), (0.0, 0.0));
    assert_eq!(harness.builds.load(Ordering::Acquire), 2);
    assert_eq!(harness.live.load(Ordering::Acquire), 0);
}

#[test]
fn negotiated_small_callback_bound_is_enforced_even_below_global_maximum(){
    let(engine,control,harness)=open();engine.drain_events();
    harness.callback_override.store(65,Ordering::Release);
    engine.select(DeviceSelection::Device("second".into()));
    let events=await_events(&engine,|event|matches!(event,
        DeviceEvent::OutputChanged(OutputInfo{kind:OutputKind::SilentClock{..},..})));
    assert!(events.iter().any(|event|matches!(event,
        DeviceEvent::Rejected(DeviceError{kind:DeviceErrorKind::UnsupportedConfig,message})
            if message.contains("65 frames")&&message.contains("maximum is 64"))));
    assert_eq!(control.position(),0);
    assert_eq!(control.callback_budget()as u32,624);
}

#[test]
fn device_budget_is_published_before_the_first_callback_of_a_new_rate(){
    let(engine,mut control,harness)=open();engine.drain_events();
    control.require_callback_budget();
    let first=control.callback_budget();
    assert_eq!(first as u32,208);
    let timeline=||crate::Timeline::new(48_000,vec![crate::Clip{start:0,
        samples:vec![0.1;10_000].into(),gain:1.,track:0}]);
    control.set_timeline_for_callback_budget(timeline(),first);control.play();
    harness.rate_override.store(44_100,Ordering::Release);
    engine.select(DeviceSelection::Device("second".into()));barrier(&engine);
    let second=control.callback_budget();
    assert_ne!(first,second);
    assert_eq!(second as u32,214);
    assert_eq!(control.position(),0);
    assert_eq!(*harness.samples.lock().unwrap().last().unwrap(),(0.,0.));
    // Same physical budget, fresh driver: its callback can now acknowledge the
    // graph generation prepared in response to the preceding device change.
    control.set_timeline_for_callback_budget(timeline(),second);
    engine.select(DeviceSelection::Device("first".into()));barrier(&engine);
    assert_eq!(control.callback_budget(),second);
    assert!(control.position()>0);
}
