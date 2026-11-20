//! Callback-level tests driven without a device (SPEC §10, T13 partial).

use std::sync::Arc;
use verdant_audio::engine::{Clip, Timeline, Transport, channel};

fn ramp(n: usize, v: f32) -> Arc<[f32]> {
    vec![v; n].into()
}

#[test]
fn plays_clips_at_their_timeline_positions() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(
        48_000,
        vec![
            Clip { start: 100, samples: ramp(50, 0.5), gain: 1.0, track: 0 },
            Clip { start: 120, samples: ramp(10, 0.25), gain: 1.0, track: 0 },
        ],
    ));
    let mut out = vec![0.0; 256 * 2];
    // Not playing yet: silence, clock does not move.
    rt.process(&mut out, 2);
    assert_eq!(ctl.position(), 0);
    assert!(out.iter().all(|&s| s == 0.0));
    ctl.play();
    rt.process(&mut out, 2);
    assert_eq!(ctl.position(), 150, "holds at the end of material");
    assert_eq!(ctl.transport(), Transport::Paused);
    assert_eq!(out[99 * 2], 0.0);
    assert_eq!(out[100 * 2], 0.5);
    assert_eq!(out[100 * 2 + 1], 0.5, "mono duplicated to both channels");
    assert_eq!(out[125 * 2], 0.75, "overlapping clips sum");
    assert_eq!(out[150 * 2], 0.0);
}

#[test]
fn seeks_coalesce_and_stop_is_never_lost() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: ramp(100_000, 0.1), gain: 1.0, track: 0 }]));
    ctl.play();
    for s in [10, 20, 30_000] {
        ctl.seek(s);
    }
    let mut out = vec![0.0; 64];
    rt.process(&mut out, 1);
    assert_eq!(ctl.position(), 30_064);
    ctl.stop();
    rt.process(&mut out, 1);
    assert_eq!(ctl.position(), 0);
    assert_eq!(ctl.transport(), Transport::Stopped);
}

#[test]
fn timeline_swap_crossfades_and_retires_off_thread() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: ramp(10_000, 1.0), gain: 1.0, track: 0 }]));
    ctl.play();
    let mut out = vec![0.0; 128];
    rt.process(&mut out, 1);
    let old = ctl.shared.position.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(old, 128);
    // Swap to a silent timeline mid-play: output ramps down, no click.
    ctl.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: ramp(10_000, 0.0), gain: 1.0, track: 0 }]));
    rt.process(&mut out, 1);
    assert!(out[0] > 0.95 && out[127] < out[0] && out[127] > 0.4, "first half of crossfade");
    rt.process(&mut out, 1);
    assert!(out[127] < 0.01, "crossfade complete after 256 frames");
    // The replaced timeline comes back for dropping outside the callback.
    ctl.maintain();
    assert_eq!(ctl.shared.swaps.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[test]
fn oversized_block_is_counted_not_allocated() {
    let (ctl, mut rt) = channel(48_000, 64);
    ctl.play();
    let mut out = vec![1.0; 1000];
    rt.process(&mut out, 1);
    assert!(out.iter().all(|&s| s == 0.0));
    assert_eq!(ctl.shared.underruns.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[test]
fn silent_clock_advances_in_real_time() {
    let (mut ctl, info) = verdant_audio::open_silent(48_000, "test".into());
    assert!(matches!(info.kind, verdant_audio::OutputKind::SilentClock { .. }));
    ctl.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: ramp(480_000, 0.0), gain: 1.0, track: 0 }]));
    ctl.play();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let pos = ctl.position();
    assert!((9_600..=24_000).contains(&pos), "≈0.3 s at 48 kHz, got {pos}");
}

#[test]
fn losing_the_binding_mid_play_stops_instead_of_running_on_silence() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: ramp(100_000, 0.2), gain: 1.0, track: 0 }]));
    ctl.play();
    let mut out = vec![0.0; 256];
    rt.process(&mut out, 1);
    assert_eq!(ctl.transport(), Transport::Playing);
    ctl.set_timeline(Timeline::new(48_000, vec![]));
    rt.process(&mut out, 1);
    rt.process(&mut out, 1);
    assert_ne!(ctl.transport(), Transport::Playing);
}

use verdant_audio::engine::{ChannelSettings, GainProcessor};

#[test]
fn tracks_mix_with_fader_balance_mute_solo_and_meters() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(
        48_000,
        vec![
            Clip { start: 0, samples: ramp(1000, 0.4), gain: 1.0, track: 0 },
            Clip { start: 0, samples: ramp(1000, 0.2), gain: 1.0, track: 1 },
        ],
    ));
    ctl.play();
    let mut out = vec![0.0; 64 * 2];
    rt.process(&mut out, 2);
    assert!((out[0] - 0.6).abs() < 1e-6 && (out[1] - 0.6).abs() < 1e-6, "sum at unity, centre");
    // Track 1 hard left: right side loses it.
    ctl.set_channel(1, ChannelSettings { gain: 1.0, pan: -1.0, mute: false, solo: false });
    rt.process(&mut out, 2);
    assert!((out[0] - 0.6).abs() < 1e-6 && (out[1] - 0.4).abs() < 1e-6);
    // Solo track 1: track 0 drops out.
    ctl.set_channel(1, ChannelSettings { gain: 0.5, pan: 0.0, mute: false, solo: true });
    rt.process(&mut out, 2);
    assert!((out[0] - 0.1).abs() < 1e-6);
    // Mute the soloed track too: silence.
    ctl.set_channel(1, ChannelSettings { gain: 0.5, pan: 0.0, mute: true, solo: true });
    rt.process(&mut out, 2);
    assert_eq!(out[0], 0.0);
    let (tracks, master) = ctl.take_meters(2);
    assert!(tracks[0].0 > 0.39 && tracks[1].0 > 0.0 && master.0 > 0.5);
    let (again, _) = ctl.take_meters(2);
    assert_eq!(again[0].0, 0.0, "meters reset on read");
}

#[test]
fn timeline_replacement_preserves_insert_gain_pan_mute_and_solo() {
    for (settings, other_solo, expected_left, expected_right) in [
        (ChannelSettings { gain: 1.0, pan: 0.0, mute: true, solo: false }, false, 0.0, 0.0),
        (ChannelSettings { gain: 0.0, pan: 0.0, mute: false, solo: false }, false, 0.0, 0.0),
        (ChannelSettings { gain: 0.5, pan: -1.0, mute: false, solo: false }, false, 0.1, 0.0),
        (ChannelSettings { gain: 1.0, pan: 0.0, mute: false, solo: false }, true, 0.0, 0.0),
    ] {
        let (mut control, mut realtime) = channel(48_000, 1024);
        let timeline = |level| Timeline::new(48_000, vec![
            Clip { start: 0, samples: ramp(1024, level), gain: 1.0, track: 0 },
            Clip { start: 0, samples: ramp(1024, 0.0), gain: 1.0, track: 1 },
        ]);
        control.set_channel(0, settings);
        control.set_channel(1, ChannelSettings { gain: 1.0, pan: 0.0, mute: false, solo: other_solo });
        assert!(control.set_chain(0, vec![Box::new(GainProcessor(0.5))]));
        control.set_timeline(timeline(0.4));
        control.play();
        let mut output = vec![0.0; 128];
        realtime.process(&mut output, 2);
        control.set_timeline(timeline(0.0));
        realtime.process(&mut output, 2);
        for (index, frame) in output.chunks_exact(2).enumerate() {
            let remaining = 1.0 - index as f32 / 256.0;
            assert!((frame[0] - expected_left * remaining).abs() < 1e-6, "{settings:?}, solo={other_solo}");
            assert!((frame[1] - expected_right * remaining).abs() < 1e-6, "{settings:?}, solo={other_solo}");
        }
    }
}

#[test]
fn stateful_insert_receives_silence_through_a_timeline_gap() {
    struct OneFrameDelay(f32, f32);
    impl verdant_audio::engine::Processor for OneFrameDelay {
        fn name(&self) -> &str { "one-frame delay" }
        fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
            for (left, right) in left.iter_mut().zip(right) {
                std::mem::swap(left, &mut self.0);
                std::mem::swap(right, &mut self.1);
            }
        }
    }
    let (mut control, mut realtime) = channel(48_000, 64);
    control.set_timeline(Timeline::new(48_000, vec![
        Clip { start: 0, samples: ramp(1, 0.5), gain: 1.0, track: 0 },
        Clip { start: 8, samples: ramp(1, 0.0), gain: 1.0, track: 0 },
    ]));
    assert!(control.set_chain(0, vec![Box::new(OneFrameDelay(0.0, 0.0))]));
    control.play();
    let mut output = [0.0; 2];
    realtime.process(&mut output, 2);
    assert_eq!(output, [0.0, 0.0]);
    realtime.process(&mut output, 2);
    assert_eq!(output, [0.5, 0.5], "delayed PCM must emerge during the gap");
    realtime.process(&mut output, 2);
    assert_eq!(output, [0.0, 0.0], "the insert state must continue advancing");
}

#[test]
fn insert_chain_swaps_in_without_allocation_and_retires_old() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: ramp(1000, 0.5), gain: 1.0, track: 0 }]));
    ctl.play();
    assert!(ctl.set_chain(0, vec![Box::new(GainProcessor(0.5))]));
    let mut out = vec![0.0; 32 * 2];
    rt.process(&mut out, 2);
    assert!((out[0] - 0.25).abs() < 1e-6);
    assert!(ctl.set_chain(0, Vec::new()));
    rt.process(&mut out, 2);
    assert!((out[0] - 0.5).abs() < 1e-6);
    ctl.maintain();
}

#[test]
fn master_soft_clip_bounds_hot_sums() {
    let (mut ctl, mut rt) = channel(48_000, 4096);
    ctl.set_timeline(Timeline::new(
        48_000,
        vec![
            Clip { start: 0, samples: ramp(100, 0.9), gain: 1.0, track: 0 },
            Clip { start: 0, samples: ramp(100, 0.9), gain: 1.0, track: 1 },
        ],
    ));
    ctl.play();
    let mut out = vec![0.0; 16 * 2];
    rt.process(&mut out, 2);
    assert!(out[0] <= 1.0 && out[0] > 0.9, "{}", out[0]);
}
