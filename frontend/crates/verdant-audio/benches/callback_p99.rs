//! Bounded, hardware-free execution of the real callback body with 32 active
//! tracks and one insert on every track. Reports p50/p99/max and counts heap
//! operations during callbacks; no source-text/wiring proxy is measured.
//!
//! Run in the development container:
//! cargo bench -p verdant-audio --bench callback_p99 -- --iterations 5000 --frames 256
//!
//! SPEC §18 budget: callback p99 < 25% of the physical block duration. This is
//! a CPU measurement, not hardware latency/underrun acceptance under UI/backend
//! contention. Native output-loss acceptance still requires a physical device.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use verdant_audio::MAX_BLOCK;
use verdant_audio::engine::{GainProcessor, channel};
use verdant_audio::{CallbackRenderer, Clip, Control, MAX_TRACKS, Timeline};

static RECORD: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if RECORD.load(Ordering::Relaxed) { ALLOCATIONS.fetch_add(1, Ordering::Relaxed); }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if RECORD.load(Ordering::Relaxed) { DEALLOCATIONS.fetch_add(1, Ordering::Relaxed); }
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if RECORD.load(Ordering::Relaxed) { ALLOCATIONS.fetch_add(1, Ordering::Relaxed); }
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if RECORD.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn prepared(rate: u32) -> (Control, CallbackRenderer) {
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    let pcm: Arc<[f32]> = (0..65_536).map(|i| (i as f32 * 0.013).sin() * 0.01).collect::<Vec<_>>().into();
    let clips = (0..MAX_TRACKS).map(|track| Clip { start: 0, samples: pcm.clone(), gain: 1.0, track: track as u16 }).collect();
    control.set_timeline(Timeline::new(48_000, clips));
    for track in 0..MAX_TRACKS {
        assert!(control.set_chain(track, vec![Box::new(GainProcessor(0.95))]));
    }
    control.play();
    let mut renderer = CallbackRenderer::new(rt, 48_000);
    renderer.set_device_rate(rate).expect("benchmark rate is admitted");
    (control, renderer)
}

fn measure(name: &str, iterations: usize, frames: usize, device_rate: u32, numeric: bool) -> bool {
    let (control, mut renderer) = prepared(device_rate);
    let mut float = vec![0.0; frames * 2];
    let mut integer = vec![0i16; frames * 2];
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..256 {
        control.seek(0);
        if numeric { renderer.process(black_box(&mut integer), 2).expect("bounded benchmark callback"); }
        else { renderer.process_f32(black_box(&mut float), 2).expect("bounded benchmark callback"); }
    }
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    for _ in 0..iterations {
        // Rewind prepared material outside the measured callback so the test
        // never quietly changes into a benchmark of end-of-timeline silence.
        control.seek(0);
        RECORD.store(true, Ordering::SeqCst);
        let started = Instant::now();
        if numeric { renderer.process(black_box(&mut integer), 2).expect("bounded benchmark callback"); }
        else { renderer.process_f32(black_box(&mut float), 2).expect("bounded benchmark callback"); }
        let elapsed = started.elapsed().as_nanos() as u64;
        RECORD.store(false, Ordering::SeqCst);
        samples.push(elapsed);
    }
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let deallocations = DEALLOCATIONS.load(Ordering::Relaxed);
    let underruns = control.shared.underruns.load(Ordering::Acquire);
    samples.sort_unstable();
    let p50 = samples[iterations / 2];
    let p99 = samples[(iterations * 99).div_ceil(100) - 1];
    let maximum = samples[iterations - 1];
    let budget = frames as u64 * 1_000_000_000 / device_rate as u64 / 4;
    let passed = p99 < budget && allocations == 0 && deallocations == 0 && underruns == 0;
    println!("{{\"case\":\"{name}\",\"tracks\":{MAX_TRACKS},\"iterations\":{iterations},\"frames\":{frames},\"processing_rate\":48000,\"device_rate\":{device_rate},\"p50_ns\":{p50},\"p99_ns\":{p99},\"max_ns\":{maximum},\"budget_ns\":{budget},\"allocations\":{allocations},\"deallocations\":{deallocations},\"underruns\":{underruns},\"passed\":{passed}}}");
    passed
}

fn arguments() -> Result<(usize, usize), String> {
    let mut iterations = 5000;
    let mut frames = 256;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        // Cargo may append --bench to harness=false executables.
        if argument == "--bench" { continue; }
        let target = match argument.as_str() {
            "--iterations" => &mut iterations,
            "--frames" => &mut frames,
            _ => return Err(format!("unknown argument {argument:?}; use --iterations 100..20000 and --frames 64..8192")),
        };
        *target = args.next().ok_or_else(|| format!("missing value for {argument}"))?
            .parse::<usize>().map_err(|e| e.to_string())?;
    }
    if !(100..=20_000).contains(&iterations) || !(64..=MAX_BLOCK).contains(&frames) {
        return Err(format!("bounded limits: iterations 100..20000, frames 64..{MAX_BLOCK}"));
    }
    Ok((iterations, frames))
}

fn main() {
    let (iterations, frames) = arguments().unwrap_or_else(|error| {
        eprintln!("callback_p99: {error}");
        std::process::exit(2);
    });
    let mut passed = measure("f32_48000", iterations, frames, 48_000, false);
    passed &= measure("i16_48000", iterations, frames, 48_000, true);
    passed &= measure("i16_44100", iterations, frames, 44_100, true);
    if !passed { std::process::exit(1); }
}
