//! Master inserts share the track processor/event seam without using a track slot.
use std::{alloc::{GlobalAlloc, Layout, System}, cell::Cell, sync::{Arc, atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering}}};
use verdant_audio::{Processor, Timeline, Clip, ChannelSettings};
use verdant_audio::engine::{channel, ChainInput, GainProcessor, MASTER_BUS, MAX_TRACKS};
use verdant_audio::automation_clock::{NativeParameterLane, NativePluginSchedule, SampleCurve, SampleInterpolation, SamplePoint};

struct CallbackAllocator;
thread_local! {
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
    static MEMORY_OPERATIONS: Cell<usize> = const { Cell::new(0) };
}
fn memory_operation() {
    if IN_CALLBACK.try_with(Cell::get).unwrap_or(false) {
        let _ = MEMORY_OPERATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CallbackAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { memory_operation(); unsafe { System.alloc(layout) } }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) { memory_operation(); unsafe { System.dealloc(pointer, layout) } }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 { memory_operation(); unsafe { System.realloc(pointer, layout, size) } }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { memory_operation(); unsafe { System.alloc_zeroed(layout) } }
}
#[global_allocator]
static ALLOCATOR: CallbackAllocator = CallbackAllocator;

fn process(realtime: &mut verdant_audio::engine::RtState, output: &mut [f32]) {
    MEMORY_OPERATIONS.set(0);
    IN_CALLBACK.set(true);
    realtime.process(output, 2);
    IN_CALLBACK.set(false);
    assert_eq!(MEMORY_OPERATIONS.get(), 0, "callback must neither allocate nor free, including chain swaps");
}

#[derive(Default)]
struct Observed {
    calls: AtomicUsize,
    count: AtomicUsize,
    offsets: [AtomicU32; 16],
    values: [AtomicU64; 16],
}

/// A nonlinear insert makes processing each track independently observably
/// different from processing the sum. Recording uses only fixed-size atomics.
struct SumShaper { gain: f32, observed: Arc<Observed> }
impl Processor for SumShaper {
    fn name(&self) -> &str { "sum shaper" }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for sample in left.iter_mut().chain(right) { *sample = *sample * *sample * self.gain; }
    }
    fn process_automated(&mut self, left: &mut [f32], right: &mut [f32], events: &[(u32, u32, f64)]) -> bool {
        self.observed.calls.fetch_add(1, Ordering::Relaxed);
        for &(id, offset, value) in events {
            assert_eq!(id, 7);
            let index = self.observed.count.fetch_add(1, Ordering::Relaxed);
            self.observed.offsets[index].store(offset, Ordering::Relaxed);
            self.observed.values[index].store(value.to_bits(), Ordering::Relaxed);
        }
        let mut next = 0;
        for index in 0..left.len() {
            while let Some(&(_, offset, value)) = events.get(next).filter(|event| event.1 as usize == index) {
                self.gain = value as f32;
                next += 1;
            }
            left[index] = left[index] * left[index] * self.gain;
            right[index] = right[index] * right[index] * self.gain;
        }
        true
    }
}

#[test]
fn master_insert_shapes_the_sum_automates_at_sample_offsets_and_removes_exactly() {
    let (mut control, mut realtime) = channel(48_000, 512);
    let mut timeline = Timeline::new(48_000, vec![
        Clip { start: 0, samples: vec![0.25; 1024].into(), gain: 1.0, track: 0 },
        Clip { start: 0, samples: vec![0.125; 1024].into(), gain: 1.0, track: (MAX_TRACKS - 1) as u16 },
    ]);
    let curve = SampleCurve::new([(0, 1.0), (4, 0.5), (255, 0.25), (256, 0.5), (259, 1.0)]
        .into_iter().map(|(sample, value)| SamplePoint { sample, value, interpolation: SampleInterpolation::Step }).collect()).unwrap();
    let schedule = NativePluginSchedule { instance_id: "master-shaper".into(), parameters: vec![
        NativeParameterLane { native_id: 7, fallback: 1.0, curve: curve.clone() },
    ] };
    schedule.validate_density(256, 4096).unwrap();
    timeline.master_native_automation = vec![Some(schedule)];
    control.set_timeline(timeline);
    control.set_channel(0, ChannelSettings { gain: 0.5, pan: -1.0, mute: false, solo: false });
    let observed = Arc::new(Observed::default());
    assert!(control.set_master_chain(vec![Box::new(SumShaper { gain: 1.0, observed: observed.clone() })]));
    control.play();
    let mut output = [0.0; 600];
    process(&mut realtime, &mut output);
    for (sample, frame) in output.chunks_exact(2).enumerate() {
        let gain = curve.value_at(sample as u64, 1.0) as f32;
        assert_eq!(frame, &[0.0625 * gain, 0.015625 * gain], "sample {sample}: master must shape the post-fader sum");
    }
    assert_eq!(observed.calls.load(Ordering::Relaxed), 2, "one master chain, split only at the host quantum");
    assert_eq!(observed.count.load(Ordering::Relaxed), 5);
    for (index, (offset, value)) in [(0, 1.0_f64), (4, 0.5), (255, 0.25), (0, 0.5), (3, 1.0)].into_iter().enumerate() {
        assert_eq!(observed.offsets[index].load(Ordering::Relaxed), offset);
        assert_eq!(f64::from_bits(observed.values[index].load(Ordering::Relaxed)), value);
    }
    control.seek(257);
    let mut seeked = [0.0; 16];
    process(&mut realtime, &mut seeked);
    assert_eq!(observed.offsets[5].load(Ordering::Relaxed), 0);
    assert_eq!(observed.offsets[6].load(Ordering::Relaxed), 2);
    assert_eq!(f64::from_bits(observed.values[5].load(Ordering::Relaxed)), 0.5);
    for (sample, frame) in seeked.chunks_exact(2).enumerate() {
        let gain = curve.value_at(257 + sample as u64, 1.0) as f32;
        assert_eq!(frame, &[0.0625 * gain, 0.015625 * gain]);
    }
    assert!(control.set_master_chain(Vec::new()));
    control.seek(0);
    process(&mut realtime, &mut output);
    assert!(output.chunks_exact(2).all(|frame| frame == [0.25, 0.125]), "removal restores every unprocessed sample exactly");
    assert_eq!(observed.calls.load(Ordering::Relaxed), 3);
    assert_eq!(control.shared.channels.len(), MAX_TRACKS);
    assert!(!control.set_chain(MAX_TRACKS, vec![Box::new(GainProcessor(0.0))]), "the master is not a 33rd track");
}

#[test]
fn track_remap_retains_master_state_and_explicit_master_remap_clears_it() {
    let (mut control, mut realtime) = channel(48_000, 64);
    control.set_timeline(Timeline::new(48_000, vec![
        Clip { start: 0, samples: vec![0.25; 128].into(), gain: 1.0, track: 0 },
    ]));
    assert!(control.set_chain(0, vec![Box::new(GainProcessor(0.5))]));
    assert!(control.set_master_chain(vec![Box::new(GainProcessor(0.5))]));
    // Queued remaps address the last submitted topology even before a callback.
    assert!(control.remap_chains(vec![vec![ChainInput::Retained { track: 0, processor: 0 }]]).is_ok());
    control.play();
    let mut output = [0.0; 8];
    process(&mut realtime, &mut output);
    assert_eq!(output, [0.0625; 8]);
    assert!(control.remap_master_chain(vec![ChainInput::Retained { track: MASTER_BUS, processor: 0 }]).is_ok());
    process(&mut realtime, &mut output);
    assert_eq!(output, [0.0625; 8]);
    // Track layouts cannot consume the master's processor or exceed 32 slots.
    assert!(control.remap_chains(vec![vec![ChainInput::Retained { track: MASTER_BUS, processor: 0 }]]).is_err());
    assert!(control.remap_chains((0..=MAX_TRACKS).map(|_| Vec::new()).collect()).is_err());
    assert!(control.remap_master_chain(Vec::new()).is_ok());
    process(&mut realtime, &mut output);
    assert_eq!(output, [0.125; 8], "master edits preserve the track insert");
    let revision = control.remap_chains_with_master(vec![], Vec::new()).ok().unwrap();
    process(&mut realtime, &mut output);
    assert_eq!(output, [0.25; 8]);
    assert!(control.retired_chain_revision() >= revision);
}
