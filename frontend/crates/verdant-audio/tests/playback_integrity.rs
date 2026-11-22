//! The prepared-PCM path must not change pronunciation when device callbacks
//! divide a phrase differently. These tests drive the actual callback body.
use std::f64::consts::TAU;
use std::sync::Arc;
use verdant_audio::MAX_BLOCK;
use verdant_audio::engine::channel;
use verdant_audio::{CallbackRenderer, Clip, Timeline};

fn controlled_replay(samples: Arc<[f32]>, rate: u32) -> (verdant_audio::Control, CallbackRenderer) {
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples, gain: 1.0, track: 0 }]));
    control.play();
    let mut callback = CallbackRenderer::new(rt, 48_000);
    callback.set_device_rate(rate).unwrap();
    (control, callback)
}

fn speech_band_signal() -> Arc<[f32]> {
    (0..48_000)
        .map(|i| {
            [220.0, 660.0, 1320.0, 2420.0, 3960.0]
                .into_iter()
                .enumerate()
                .map(|(j, hz)| (TAU * hz * i as f64 / 48_000.0).sin() * (0.11 - j as f64 * 0.01))
                .sum::<f64>() as f32
        })
        .collect::<Vec<_>>()
        .into()
}

fn replay(samples: Arc<[f32]>, rate: u32, blocks: &[usize]) -> Vec<f32> {
    let output_frames = (samples.len() as f64 * rate as f64 / 48_000.0).round() as usize;
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples, gain: 1.0, track: 0 }]));
    control.play();
    let mut callback = CallbackRenderer::new(rt, 48_000);
    callback.set_device_rate(rate).unwrap();
    let mut output = vec![0.0; output_frames];
    let (mut at, mut block) = (0, 0);
    while at < output.len() {
        let end = (at + blocks[block % blocks.len()]).min(output.len());
        callback.process_f32(&mut output[at..end], 1).unwrap();
        at = end;
        block += 1;
    }
    output
}

#[test]
fn equal_rate_voice_band_pcm_is_bit_exact_for_every_callback_partition() {
    let source = speech_band_signal();
    for pattern in [&[1, 127, 128, 255, 256, 1024, 8192][..], &[1024][..], &[8192][..]] {
        assert_eq!(replay(source.clone(), 48_000, pattern), &*source, "{pattern:?}");
    }
}

#[test]
fn device_rate_conversion_is_bit_exact_across_callback_partitions() {
    let source = speech_band_signal();
    for rate in [44_100, 96_000] {
        let reference = replay(source.clone(), rate, &[8192]);
        for pattern in [&[1, 127, 128, 255, 256, 1024][..], &[1024][..]] {
            assert_eq!(replay(source.clone(), rate, pattern), reference, "{rate} {pattern:?}");
        }
    }
}

#[test]
fn device_rate_conversion_keeps_vowel_band_amplitudes_and_pitch() {
    for rate in [44_100, 96_000] {
        let output = replay(speech_band_signal(), rate, &[1024]);
        // 0.1..0.9 s is an integer number of periods for all probe harmonics;
        // the startup/end filters therefore cannot mask a frequency shift.
        let from = rate as usize / 10;
        let to = rate as usize * 9 / 10;
        for (j, hz) in [220.0, 660.0, 1320.0, 2420.0, 3960.0].into_iter().enumerate() {
            let (mut re, mut im) = (0.0, 0.0);
            for (i, sample) in output[from..to].iter().enumerate() {
                let phase = TAU * hz * (from + i) as f64 / rate as f64;
                re += *sample as f64 * phase.cos();
                im += *sample as f64 * phase.sin();
            }
            let amplitude = 2.0 * re.hypot(im) / (to - from) as f64;
            let expected = 0.11 - j as f64 * 0.01;
            assert!(
                (amplitude / expected - 1.0).abs() < 0.005,
                "{rate} Hz output, {hz} Hz probe: amplitude {amplitude}, expected {expected}"
            );
        }
    }
}

#[test]
fn resampled_seek_and_stop_do_not_leak_prefetched_pcm_or_filter_history() {
    let source: Arc<[f32]> = (0..24_000).map(|i| if i < 12_000 { 0.25 } else { -0.25 }).collect::<Vec<_>>().into();
    for rate in [44_100, 96_000] {
        let (control, mut callback) = controlled_replay(source.clone(), rate);
        callback.process_f32(&mut [0.0; 1], 1).unwrap(); // Leaves unread positive PCM in the rate adapter.
        control.seek(12_000);
        let mut sought = [0.0; 256];
        callback.process_f32(&mut sought, 1).unwrap();
        let (fresh_control, mut fresh) = controlled_replay(source.clone(), rate);
        fresh_control.seek(12_000);
        let mut expected = [0.0; 256];
        fresh.process_f32(&mut expected, 1).unwrap();
        assert_eq!(sought, expected, "seek at {rate} Hz must start a clean filter neighbourhood");
        assert!(sought[32..].iter().all(|v| *v < -0.24));

        control.stop();
        let mut stopped = [1.0; 64];
        callback.process_f32(&mut stopped, 1).unwrap();
        assert!(stopped.iter().all(|v| *v == 0.0));
        assert_eq!(control.position(), 0);
        control.play();
        let (_, mut fresh_start) = controlled_replay(source.clone(), rate);
        callback.process_f32(&mut stopped, 1).unwrap();
        let mut start = [0.0; 64];
        fresh_start.process_f32(&mut start, 1).unwrap();
        assert_eq!(stopped, start, "stop/play must not retain the previous seek's PCM");
        control.seek(12_000);
        callback.process_f32(&mut stopped, 1).unwrap();
        control.stop();
        control.play(); // Coalesced before the next callback must still rewind.
        callback.process_f32(&mut stopped, 1).unwrap();
        assert_eq!(stopped, start, "play cancelled a still-pending stop");
    }
}

#[test]
fn resampled_pause_is_silent_and_resume_continues_without_dropping_prefetch() {
    for rate in [44_100, 96_000] {
        let source = speech_band_signal();
        let (control, mut callback) = controlled_replay(source.clone(), rate);
        let (_, mut uninterrupted) = controlled_replay(source, rate);
        let mut prefix = [0.0; 37];
        callback.process_f32(&mut prefix, 1).unwrap();
        uninterrupted.process_f32(&mut prefix, 1).unwrap();
        let position = control.position();
        control.pause();
        for size in [1, 63, 1024] {
            let mut paused = vec![1.0; size];
            callback.process_f32(&mut paused, 1).unwrap();
            assert!(paused.iter().all(|v| *v == 0.0), "pause leaked PCM at {rate} Hz");
            assert_eq!(control.position(), position);
        }
        control.play();
        let mut resumed = [0.0; 512];
        let mut expected = [0.0; 512];
        callback.process_f32(&mut resumed, 1).unwrap();
        uninterrupted.process_f32(&mut expected, 1).unwrap();
        assert_eq!(resumed, expected, "pause/resume skipped or repeated prepared PCM at {rate} Hz");
    }
}

#[test]
fn resampled_end_of_material_drains_valid_prefetch_before_silence() {
    // The mixer's position reaches EOF while the device still has unread
    // samples. Automatic EOF must not be treated as an explicit pause.
    let source: Arc<[f32]> = vec![0.25; 1_000].into();
    for rate in [44_100, 96_000] {
        let (control, mut callback) = controlled_replay(source.clone(), rate);
        let frames = (1_000.0 * rate as f64 / 48_000.0).round() as usize;
        let mut output = vec![0.0; frames];
        for block in output.chunks_mut(17) {
            callback.process_f32(block, 1).unwrap();
        }
        assert!(output[frames - 32..frames - 8].iter().all(|v| *v > 0.24), "premature EOF truncation at {rate} Hz");
        assert_eq!(control.position(), 1_000);
    }
}

#[test]
fn a_seek_arriving_inside_source_render_waits_for_the_next_device_callback() {
    struct PublishSeek {
        shared: Arc<verdant_audio::engine::Shared>,
        fired: bool,
    }
    impl verdant_audio::engine::Processor for PublishSeek {
        fn name(&self) -> &str {
            "publish one asynchronous seek"
        }
        fn process(&mut self, _: &mut [f32], _: &mut [f32]) {
            if !self.fired {
                self.shared.seek.store(12_000, std::sync::atomic::Ordering::Release);
                self.fired = true;
            }
        }
    }
    let source: Arc<[f32]> = (0..24_000).map(|i| if i < 12_000 { 0.25 } else { -0.25 }).collect::<Vec<_>>().into();
    let (mut control, mut callback) = controlled_replay(source, 44_100);
    assert!(control.set_chain(0, vec![Box::new(PublishSeek { shared: control.shared.clone(), fired: false })]));
    let mut block = [0.0; 1024];
    callback.process_f32(&mut block, 1).unwrap();
    assert!(block[32..].iter().all(|v| *v > 0.24), "seek was consumed halfway through the physical callback");
    callback.process_f32(&mut block, 1).unwrap();
    assert!(block[32..].iter().all(|v| *v < -0.24));
}

#[test]
fn paused_live_performance_is_resampled_without_consuming_timeline_prefetch(){
    struct Tone { phase:f64 }
    impl verdant_audio::Processor for Tone {
        fn name(&self)->&str{"live tone"}
        fn process(&mut self,l:&mut[f32],r:&mut[f32]){
            for(a,b)in l.iter_mut().zip(r){let sample=(TAU*self.phase).sin()as f32*0.1;self.phase=(self.phase+440./48000.).fract();*a=sample;*b=sample;}
        }
    }
    for rate in [44100,48000,96000] {
        let source=speech_band_signal();
        let(mut control,mut callback)=controlled_replay(source.clone(),rate);
        let(_,mut reference)=controlled_replay(source,rate);
        callback.process_f32(&mut[0.;37],1).unwrap();reference.process_f32(&mut[0.;37],1).unwrap();
        control.pause();let position=control.position();
        assert!(control.set_monitor(1,Some(Box::new(Tone{phase:0.}))));
        let mut live=vec![0.;rate as usize];
        for block in live.chunks_mut(173){callback.process_f32(block,1).unwrap();}
        assert_eq!(control.position(),position);
        let(mut re,mut im)=(0.,0.);let from=rate as usize/10;let to=rate as usize*9/10;
        for(i,&sample)in live[from..to].iter().enumerate(){let angle=TAU*440.*(from+i)as f64/rate as f64;re+=sample as f64*angle.cos();im+=sample as f64*angle.sin();}
        let amplitude=2.*re.hypot(im)/(to-from)as f64;
        assert!((amplitude-0.1).abs()<0.001,"live pitch/rate changed at {rate}: {amplitude}");
        control.set_monitor(1,None);control.play();
        let mut resumed=[0.;512];let mut expected=[0.;512];
        callback.process_f32(&mut resumed,1).unwrap();reference.process_f32(&mut expected,1).unwrap();
        assert_eq!(resumed,expected,"paused performance consumed timeline PCM at {rate}");
    }
}
