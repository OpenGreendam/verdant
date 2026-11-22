//! End-to-end checks using the actual isolated child, shared-memory audio and
//! JSON control pipe. Build verdant-plughost first and set VERDANT_PLUGIN_HOST.

#![cfg(feature = "client")]

use std::f64::consts::TAU;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};
use verdant_plugins::builtin_dsp::{self as dsp, TableEffect};
use verdant_plugins::{InsertSpec, PluginClient, PluginClientOpts};

struct CaptureAllocator;
thread_local! {
    static CAPTURING: Cell<bool> = const { Cell::new(false) };
    static MEMORY_OPERATIONS: Cell<usize> = const { Cell::new(0) };
}
fn memory_operation() {
    if CAPTURING.try_with(Cell::get).unwrap_or(false) {
        let _ = MEMORY_OPERATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CaptureAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { memory_operation(); unsafe { System.alloc(layout) } }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) { memory_operation(); unsafe { System.dealloc(pointer, layout) } }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 { memory_operation(); unsafe { System.realloc(pointer, layout, size) } }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { memory_operation(); unsafe { System.alloc_zeroed(layout) } }
}
#[global_allocator]
static ALLOCATOR: CaptureAllocator = CaptureAllocator;

#[test]
fn signal_capture_wraps_without_heap_operations() {
    let mut capture = verdant_plugins::telemetry::TelemetryCapture::default();
    let input = [0.25; 256];
    let output = [-0.125; 256];
    MEMORY_OPERATIONS.set(0);
    CAPTURING.set(true);
    for _ in 0..64 { capture.capture(&input, &input, &output, &output).unwrap(); }
    let rejected = capture.capture(&[f32::NAN], &[0.0], &[0.0], &[0.0]);
    CAPTURING.set(false);
    assert_eq!(MEMORY_OPERATIONS.get(), 0, "audio capture must neither allocate nor free, including wrap and rejection");
    assert!(rejected.is_err());
    assert_eq!(capture.frame().frames_total, 16_384);
}

fn client() -> Arc<PluginClient> {
    let host = std::path::PathBuf::from(std::env::var_os("VERDANT_PLUGIN_HOST")
        .expect("VERDANT_PLUGIN_HOST must name the freshly built verdant-plughost"));
    PluginClient::with_plughost(PluginClientOpts {
        plughost_path: Some(host.clone()),
        cache_dir: std::env::temp_dir().join(format!("verdant-signal-{}", uuid::Uuid::new_v4())),
        sample_rate: 48_000.0, max_frames: 8192,
    }, host).unwrap()
}

#[test]
#[ignore = "requires a freshly built verdant-plughost in VERDANT_PLUGIN_HOST"]
fn deliberate_host_retirement_is_silent_but_explicit_kill_reports_a_crash() {
    let client = client();
    // Drop joins both observers, so after each retirement there can be no
    // later event from that instance. Exercise live undo-style retirement
    // and the independently owned hosts used by export.
    for _ in 0..4 {
        let live = client.insert(PluginClient::builtin_gain_spec()).unwrap();
        assert!(matches!(live.status(), verdant_plugins::InsertStatus::Running));
        client.remove(live.key());
        drop(live);
        let offline = client.offline_insert(PluginClient::builtin_gain_spec()).unwrap();
        assert!(matches!(offline.status(), verdant_plugins::InsertStatus::Running));
        drop(offline);
        while let Some(event) = client.try_recv_event() {
            assert!(!matches!(&event, verdant_plugins::ClientEvent::Crashed { .. }),
                "normal host retirement must not publish a crash: {event:?}");
        }
    }

    let live = client.insert(PluginClient::builtin_gain_spec()).unwrap();
    let key = live.key();
    live.kill_host();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(verdant_plugins::ClientEvent::Crashed { insert, detail }) = client.try_recv_event() {
            assert_eq!(insert, key, "a retired instance must not generate a delayed fault");
            assert!(!detail.is_empty());
            break;
        }
        assert!(Instant::now() < deadline, "an unexpected host exit must remain observable");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(live.status(), verdant_plugins::InsertStatus::Crashed { .. }));
    client.remove(key);
    drop(live);
    while let Some(event) = client.try_recv_event() {
        assert!(!matches!(&event, verdant_plugins::ClientEvent::Crashed { .. }),
            "one real failure must not produce duplicate crash events: {event:?}");
    }
}

#[test]
#[ignore = "requires a freshly built verdant-plughost in VERDANT_PLUGIN_HOST"]
fn isolated_host_signal_reports_real_pcm_and_stops_when_audio_stops() {
    let client = client();
    let mut gain = client.offline_insert(PluginClient::builtin_gain_spec()).unwrap();
    let untouched = client.offline_insert(PluginClient::builtin_gain_spec()).unwrap();
    assert!(gain.cached_telemetry().is_none());
    let empty = untouched.refresh_telemetry().unwrap();
    assert_eq!(empty.frames_total, 0);
    assert_eq!(empty.output_peak, [0.0; 2]);
    assert!(empty.waveform_output.is_empty());

    gain.set_param("gain_db", -6.020599913279624).unwrap();
    let mut left: Vec<f32> = (0..4096).map(|i| (0.5 * (TAU * i as f64 / 16.0).sin()) as f32).collect();
    let mut right: Vec<f32> = left.iter().map(|sample| -sample).collect();
    gain.process_offline(&mut left, &mut right).unwrap();
    let snapshot = gain.refresh_telemetry().unwrap();
    assert_eq!(snapshot.frames_total, 4096);
    assert_eq!(snapshot.window_frames, 2048);
    for channel in 0..2 {
        assert!((snapshot.input_peak[channel] - 0.5).abs() < 1e-6);
        assert!((snapshot.output_peak[channel] - 0.25).abs() < 1e-6);
        assert!((snapshot.input_rms[channel] - 0.5 / 2.0f32.sqrt()).abs() < 1e-6);
        assert!((snapshot.output_rms[channel] - 0.25 / 2.0f32.sqrt()).abs() < 1e-6);
    }
    assert!((snapshot.spectrum_input_db[128] + 6.0206).abs() < 0.001);
    assert!((snapshot.spectrum_output_db[128] + 12.0412).abs() < 0.001);
    assert_eq!(snapshot.spectrum_frequencies[128], 3000.0);
    assert!(Arc::ptr_eq(&snapshot, &gain.cached_telemetry().unwrap()));
    assert!(Arc::ptr_eq(&snapshot, &gain.refresh_telemetry().unwrap()), "immediate refresh uses the bounded-rate cache");

    std::thread::sleep(Duration::from_millis(110));
    let stopped = gain.refresh_telemetry().unwrap();
    assert_eq!(stopped.captured_at, snapshot.captured_at, "no PCM must not advance the measurement time");
    assert_eq!(untouched.refresh_telemetry().unwrap().frames_total, 0, "signal cannot leak between isolated instances");

    gain.process_offline(&mut [0.0; 2048], &mut [0.0; 2048]).unwrap();
    std::thread::sleep(Duration::from_millis(110));
    let silence = gain.refresh_telemetry().unwrap();
    assert_eq!(silence.frames_total, 6144);
    assert_eq!(silence.input_peak, [0.0; 2]);
    assert_eq!(silence.output_rms, [0.0; 2]);
    assert!(silence.spectrum_output_db.iter().all(|&db| db == -120.0));
}

#[test]
#[ignore = "requires a freshly built verdant-plughost in VERDANT_PLUGIN_HOST"]
fn isolated_delay_transport_events_reach_exact_samples_without_changing_saved_tempo() {
    let client = client();
    let mut host = client.offline_insert(InsertSpec::new("builtin", dsp::DELAY_ID, "Verdant Delay", "")).unwrap();
    let mut expected = dsp::DelayProcessor::new(48_000.0);
    for (id, value) in [("sync", 1.0), ("mix", 100.0), ("feedback", 0.0),
        ("low_cut", 20.0), ("high_cut", 20_000.0)] {
        host.set_param(id, value).unwrap();
        expected.set(id, value).unwrap();
    }
    assert!(host.automation_params().unwrap().iter()
        .all(|parameter| parameter.target.native_id() != dsp::TRANSPORT_TEMPO_EVENT));
    let saved = host.get_state_b64().unwrap();
    let input: Vec<f32> = (0..48_000).map(|i| (0.25 * (TAU * 260.0 * i as f64 / 48_000.0).sin()) as f32).collect();
    let (mut left, mut right) = (input.clone(), input.clone());
    let (mut expected_l, mut expected_r) = (vec![0.0; input.len()], vec![0.0; input.len()]);
    let boundary = 12_345;
    expected.set_tempo(180.0);
    expected.process(&input[..boundary], &input[..boundary], None,
        &mut expected_l[..boundary], &mut expected_r[..boundary]);
    expected.set_tempo(90.0);
    expected.process(&input[boundary..], &input[boundary..], None,
        &mut expected_l[boundary..], &mut expected_r[boundary..]);
    host.process_offline_events(&mut left, &mut right, &[
        (dsp::TRANSPORT_TEMPO_EVENT, 0, 180.0),
        (dsp::TRANSPORT_TEMPO_EVENT, boundary as u32, 90.0),
    ]).unwrap();
    assert_eq!(left, expected_l);
    assert_eq!(right, expected_r);
    assert!(left.iter().any(|sample| sample.abs() > 0.1));
    assert_eq!(host.get_state_b64().unwrap(), saved);
    assert_eq!(host.refresh_telemetry().unwrap().frames_total, input.len() as u64);
}

#[test]
#[ignore = "requires a freshly built verdant-plughost in VERDANT_PLUGIN_HOST"]
fn native_midi_loudness_and_meter_reset_cross_the_real_control_protocol() {
    let client=client();
    let mut fx=client.offline_insert(InsertSpec::new("builtin",dsp::MICRO_ID,"Micro","")).unwrap();
    let descs=fx.params().unwrap();let output=descs.iter().position(|d|d.id=="output").unwrap();
    for(id,v)in[("mix",0.),("midi1_cc",74.),("midi1_target",output as f64)]{fx.set_param(id,v).unwrap();}
    fx.midi_events(&[(verdant_plugins::proto::midi_control(1,74,0,0.5).0,0.5)]).unwrap();
    let descriptor=fx.params().unwrap().into_iter().find(|d|d.id=="output").unwrap();
    assert_eq!(descriptor.value,(descriptor.min+descriptor.max)*0.5);
    for block in 0..80 {
        let mut l:[f32;256]=std::array::from_fn(|i|(0.1*(TAU*1000.*(block*256+i)as f64/48000.).sin())as f32);let mut r=l;
        fx.process_offline(&mut l,&mut r).unwrap();
    }
    let before=fx.refresh_telemetry().unwrap();
    assert!(before.loudness.unwrap().integrated_lufs>-100.);
    fx.reset_meters().unwrap();
    let reset=fx.refresh_telemetry().unwrap();
    assert_eq!(reset.frames_total,before.frames_total,"meter reset does not fabricate new processed PCM");
    assert_eq!(reset.loudness.unwrap().integrated_lufs,-120.);
    assert!(reset.gain_reduction_db.is_empty());
    let cc=verdant_plugins::proto::midi_control(1,74,0,0.25).0;
    fx.enqueue_midi(&[(cc,0.25),(cc,0.75)]).unwrap();
    // No background task runs: state capture itself must deliver queued MIDI.
    let saved=fx.get_state_b64().unwrap();
    let mapped=fx.params().unwrap().into_iter().find(|d|d.id=="output").unwrap();
    assert_eq!(mapped.value,mapped.min+(mapped.max-mapped.min)*0.75);
    assert!(fx.enqueue_midi(&[(cc,0.25),(cc,f64::NAN)]).is_err());
    assert_eq!(fx.get_state_b64().unwrap(),saved);
    fx.stop_midi();fx.flush_midi().unwrap();
    let mut l=[0.1;256];let mut r=l;
    fx.process_offline_events(&mut l,&mut r,&[verdant_plugins::proto::midi_control(1,74,10,0.25)]).unwrap();
    assert!(l[9]>l[10],"sample-offset MIDI reaches the native DSP");
}

#[test]
#[ignore = "requires a freshly built verdant-plughost in VERDANT_PLUGIN_HOST"]
fn limiter_meter_pause_and_reset_control_the_displayed_integrated_history(){
    let client=client();
    let mut limiter=client.offline_insert(InsertSpec::new("builtin",dsp::LIMITER_ID,"Limiter","")).unwrap();
    assert!(!limiter.automation_params().unwrap().iter().any(|p|p.id=="meter_pause"));
    let render=|limiter:&mut verdant_plugins::InsertHandle,amplitude:f64|{
        for block in 0..188{
            let mut l:[f32;256]=std::array::from_fn(|i|(amplitude*(TAU*1000.*(block*256+i)as f64/48000.).sin())as f32);
            let mut r=l;limiter.process_offline(&mut l,&mut r).unwrap();
        }
    };
    render(&mut limiter,0.1);
    let initial=limiter.refresh_telemetry().unwrap();
    let integrated=initial.loudness.unwrap().integrated_lufs;assert!(integrated>-100.);
    limiter.set_param("meter_pause",1.).unwrap();render(&mut limiter,0.01);
    std::thread::sleep(Duration::from_millis(110));
    let paused=limiter.refresh_telemetry().unwrap();
    assert_eq!(paused.loudness.unwrap().integrated_lufs,integrated);
    assert!(paused.frames_total>initial.frames_total,"paused measurement does not freeze processed audio");
    limiter.set_param("meter_reset",1.).unwrap();
    std::thread::sleep(Duration::from_millis(110));
    let reset=limiter.refresh_telemetry().unwrap();
    assert_eq!(reset.loudness.unwrap().integrated_lufs,-120.);
    assert_eq!(reset.frames_total,paused.frames_total,"reset is visible while stopped");
    assert_eq!(reset.captured_at,paused.captured_at,"parameter reset does not refresh the PCM timestamp");
    limiter.set_param("meter_pause",0.).unwrap();render(&mut limiter,0.1);
    std::thread::sleep(Duration::from_millis(110));
    let before_rpc=limiter.refresh_telemetry().unwrap();
    assert!(before_rpc.loudness.unwrap().integrated_lufs>-100.);
    limiter.reset_meters().unwrap();
    let after_rpc=limiter.refresh_telemetry().unwrap();
    assert_eq!(after_rpc.loudness.unwrap().integrated_lufs,-120.);
    assert_eq!(after_rpc.frames_total,before_rpc.frames_total);
    assert_eq!(after_rpc.captured_at,before_rpc.captured_at,"RPC reset does not refresh the PCM timestamp");
}
