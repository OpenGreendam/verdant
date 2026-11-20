use super::{follow_playhead, lcd_position};
use verdant_audio::engine::{Clip, Timeline, Transport, channel};
use verdant_domain::{TrackId, model::{TempoPoint, TimeSignature}, timing::{MeterMap, TempoMap}};
use verdant_session::{EditorSession, Viewport};

fn signature(tick: i64, numerator: u32, denominator: u32) -> TimeSignature {
    TimeSignature { tick, numerator, denominator, unknown: Default::default() }
}

#[test]
fn transport_pause_stop_and_seek_keep_exact_source_frames() {
    let (mut control, mut rt) = channel(48_000, 64);
    let samples = (0..2048).map(|frame| frame as f32 / 4096.0).collect::<Vec<_>>();
    let mut timeline = Timeline::new(48_000, vec![Clip { start: 0, samples: samples.into(), gain: 1.0, track: 0 }]);
    timeline.loop_range = Some((101, 901));
    control.set_timeline(timeline);
    let mut out = [0.0; 8];
    rt.process(&mut out, 1);

    control.seek(233);
    assert_eq!(control.position(), 233, "stopped seek updates the UI before the next callback");
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 233);
    assert_eq!(control.transport(), Transport::Paused);
    assert_eq!(out, [0.0; 8]);
    control.play();
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 241);
    assert_eq!(out[0], 233.0 / 4096.0, "the first output is the requested source sample");
    assert_eq!(out[7], 240.0 / 4096.0);

    control.pause();
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 241, "pause holds the exact next frame");
    assert_eq!(control.transport(), Transport::Paused);
    assert_eq!(out, [0.0; 8]);
    control.seek(701);
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 701);
    control.play();
    control.seek(711);
    rt.process(&mut out, 1);
    assert_eq!(out[0], 711.0 / 4096.0, "playing seek also starts at the exact sample");
    assert_eq!(control.position(), 719);

    control.seek(777);
    control.stop();
    assert_eq!(control.position(), 101, "stop supersedes an older pending seek");
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 101);
    assert_eq!(control.transport(), Transport::Stopped, "nonzero loop start is still stopped");
    assert_eq!(out, [0.0; 8]);
    control.stop();
    control.play();
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 109, "a quick stop/play restarts from the loop start");
    control.stop();
    control.seek(411);
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 411, "a later explicit seek supersedes the stop destination");
    assert_eq!(control.transport(), Transport::Paused);

    control.set_timeline(Timeline::new(48_000, Vec::new()));
    rt.process(&mut [], 1);
    control.stop();
    assert_eq!(control.position(), 0);
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 0);
    assert_eq!(control.transport(), Transport::Stopped);
    control.pause();
    rt.process(&mut out, 1);
    assert_eq!(control.position(), 0);
    assert_eq!(control.transport(), Transport::Paused, "pause at zero is distinct from stop");
}

#[test]
fn follow_playhead_reengages_both_views_only_when_enabled() {
    let mut session = EditorSession::new(TrackId::from("transport-track"));
    session.viewport = Viewport { width: 100.0, px_per_tick: 1.0, ..Viewport::default() };
    let mut arrange = session.viewport;
    session.follow_playhead = false;
    let (control, mut rt) = channel(48_000, 64);
    control.play();
    rt.process(&mut [], 1);
    assert!(!follow_playhead(&mut session, &mut arrange, 500.0, 0.016));
    assert_eq!((session.viewport.start_tick, arrange.start_tick), (0.0, 0.0));
    control.pause();
    rt.process(&mut [], 1);
    control.play();
    rt.process(&mut [], 1);
    assert!(!follow_playhead(&mut session, &mut arrange, 600.0, 0.016));
    assert!(!session.follow_playhead, "transport transitions must preserve the user's preference");

    session.follow_playhead = true;
    arrange.px_per_tick = 0.5;
    assert!(follow_playhead(&mut session, &mut arrange, 600.0, 0.016));
    assert!((0.0..565.0).contains(&session.viewport.start_tick) && session.viewport.start_tick > 0.0);
    assert!((0.0..session.viewport.start_tick).contains(&arrange.start_tick) && arrange.start_tick > 0.0);
    let before = (session.viewport.start_tick, arrange.start_tick);
    session.autoscroll_active = true;
    assert!(!follow_playhead(&mut session, &mut arrange, 900.0, 0.016));
    assert_eq!((session.viewport.start_tick, arrange.start_tick), before);
    session.autoscroll_active = false;
    for _ in 0..60 {
        follow_playhead(&mut session, &mut arrange, 900.0, 0.016);
    }
    assert_eq!((session.viewport.start_tick, arrange.start_tick), (865.0, 830.0));
    assert!(!follow_playhead(&mut session, &mut arrange, 900.0, 0.016));

    // After manual navigation, both views recover at the same rate regardless
    // of display refresh. A wheel frame must never turn follow off.
    session.viewport.pan_px(160.0, 0.0);
    arrange.pan_px(120.0, 0.0);
    let (roll, tracks) = (session.viewport, arrange);
    follow_playhead(&mut session, &mut arrange, 900.0, 0.032);
    let once = (session.viewport.start_tick, arrange.start_tick);
    (session.viewport, arrange) = (roll, tracks);
    for _ in 0..2 {
        follow_playhead(&mut session, &mut arrange, 900.0, 0.016);
    }
    assert!((once.0 - session.viewport.start_tick).abs() < 1e-9);
    assert!((once.1 - arrange.start_tick).abs() < 1e-9);
}

#[test]
fn lcd_musical_position_tracks_meter_and_tempo_changes() {
    let tempo = TempoMap::new(&[
        TempoPoint { tick: 0, micro_bpm: 120_000_000, unknown: Default::default() },
        TempoPoint { tick: 5760, micro_bpm: 90_000_000, unknown: Default::default() },
    ], 960).unwrap();
    let meter = MeterMap::new(&[signature(0, 3, 4), signature(5760, 7, 8), signature(12480, 5, 4)], 960).unwrap();
    for (tick, musical, signature, time, bpm) in [
        (0, "001:01:01:001", "3/4", "00:00.000", 120.0),
        (5760, "003:01:01:001", "7/8", "00:03.000", 90.0),
        (9600, "004:02:01:001", "7/8", "00:05.667", 90.0),
        (12480, "005:01:01:001", "5/4", "00:07.667", 90.0),
    ] {
        let display = lcd_position(tempo.sample_at(tick, 48_000) as u64, 48_000, &tempo, &meter);
        assert_eq!(display.musical, musical);
        assert_eq!(display.signature, signature);
        assert_eq!(display.time, time);
        assert_eq!(display.bpm, bpm);
    }
}

#[test]
fn lcd_changes_meter_on_the_rounded_seek_frame_and_carries_minutes() {
    let tempo = TempoMap::constant(127.0);
    let meter = MeterMap::new(&[signature(0, 3, 4), signature(2880, 7, 8)], 960).unwrap();
    let frame = tempo.sample_at(2880, 48_000) as u64;
    assert_eq!(frame, 68_031);
    let before = lcd_position(frame - 1, 48_000, &tempo, &meter);
    assert_eq!(before.musical, "001:03:04:240");
    assert_eq!(before.signature, "3/4");
    let boundary = lcd_position(frame, 48_000, &tempo, &meter);
    assert_eq!(boundary.musical, "002:01:01:001");
    assert_eq!(boundary.signature, "7/8");
    assert_eq!(lcd_position(2_879_981, 48_000, &tempo, &meter).time, "01:00.000");
}
