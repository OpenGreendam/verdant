//! Hardware-free numeric-format, channel and sample-rate boundary tests.

use std::f64::consts::PI;
use verdant_audio::convert::{DeviceSample, interleave, to};
use verdant_audio::callback::CallbackBufferError;
use verdant_audio::{DeviceErrorKind, MAX_BLOCK};
#[cfg(feature = "audio-device")]
use verdant_audio::device::SampleFmt;
use verdant_audio::engine::{GainProcessor, channel};
use verdant_audio::{CallbackRenderer, Clip, Timeline, Transport};

#[test]
fn every_signed_integer_format_saturates_and_centers_silence() {
    macro_rules! check {
        ($($t:ty),+) => {$ (
            assert_eq!(to::<$t>(-2.0), <$t>::MIN);
            assert_eq!(to::<$t>(-1.0), <$t>::MIN);
            assert_eq!(to::<$t>(0.0), 0);
            assert_eq!(to::<$t>(f32::NAN), 0);
            assert_eq!(to::<$t>(1.0), <$t>::MAX);
            assert_eq!(to::<$t>(2.0), <$t>::MAX);
            assert_eq!(to::<$t>(f32::NEG_INFINITY), <$t>::MIN);
            assert_eq!(to::<$t>(f32::INFINITY), <$t>::MAX);
            assert_eq!(to::<$t>(0.5), (<$t>::MAX / 2) + 1);
            assert_eq!(to::<$t>(-0.5), <$t>::MIN / 2);
        )+};
    }
    check!(i8, i16, i32, i64);
}

#[test]
fn every_unsigned_integer_format_has_midpoint_silence_and_both_full_scale_endpoints() {
    macro_rules! check {
        ($($t:ty),+) => {$ (
            let midpoint = (<$t>::MAX / 2) + 1;
            assert_eq!(to::<$t>(-2.0), 0);
            assert_eq!(to::<$t>(-1.0), 0);
            assert_eq!(to::<$t>(0.0), midpoint);
            assert_eq!(to::<$t>(f32::NAN), midpoint);
            assert_eq!(to::<$t>(1.0), <$t>::MAX);
            assert_eq!(to::<$t>(2.0), <$t>::MAX);
            assert_eq!(to::<$t>(f32::NEG_INFINITY), 0);
            assert_eq!(to::<$t>(f32::INFINITY), <$t>::MAX);
            assert_eq!(to::<$t>(-0.5), midpoint / 2);
            assert_eq!(to::<$t>(0.5), midpoint + midpoint / 2);
        )+};
    }
    check!(u8, u16, u32, u64);
}

#[test]
fn packed_24_bit_and_float_formats_cover_full_scale_without_invalid_samples() {
    #[cfg(feature = "audio-device")]
    {
        for (input, signed, unsigned) in [
            (-2.0, -8_388_608, 0), (-1.0, -8_388_608, 0), (-0.5, -4_194_304, 4_194_304),
            (0.0, 0, 8_388_608), (0.5, 4_194_304, 12_582_912), (1.0, 8_388_607, 16_777_215),
            (2.0, 8_388_607, 16_777_215), (f32::NAN, 0, 8_388_608),
        ] {
            assert_eq!(to::<cpal::I24>(input).inner(), signed);
            assert_eq!(to::<cpal::U24>(input).inner(), unsigned);
        }
    }
    for value in [-1.0, -0.25, 0.0, 0.25, 1.0] {
        assert_eq!(to::<f32>(value), value);
        assert_eq!(to::<f64>(value), value as f64);
    }
    assert_eq!(to::<f64>(f32::NAN), 0.0);
    assert_eq!(to::<f32>(f32::INFINITY), 1.0);
    assert_eq!(to::<f64>(f32::NEG_INFINITY), -1.0);
}

#[test]
fn mono_stereo_surround_and_partial_frames_overwrite_every_output_slot() {
    let stereo = [0.5, -0.25, -1.0, 1.0];
    let mut mono = [99i16; 2];
    interleave(&stereo, &mut mono, 1);
    assert_eq!(mono, [4096, 0]);
    let mut surround = [99u16; 9];
    interleave(&stereo, &mut surround, 4);
    assert_eq!(surround, [49152, 24576, 32768, 32768, 0, 65535, 32768, 32768, 32768]);
    let mut invalid_channels = [99u8; 3];
    interleave(&stereo, &mut invalid_channels, 0);
    assert_eq!(invalid_channels, [128; 3]);
}

#[test]
#[cfg(feature = "audio-device")]
fn sample_format_admission_covers_all_numeric_pcm_and_rejects_dsd() {
    use cpal::SampleFormat as C;
    for format in [C::I8, C::I16, C::I24, C::I32, C::I64, C::U8, C::U16, C::U24, C::U32, C::U64, C::F32, C::F64] {
        assert!(SampleFmt::from_cpal(format).is_some(), "{format:?}");
    }
    for format in [C::DsdU8, C::DsdU16, C::DsdU32] {
        assert_eq!(SampleFmt::from_cpal(format), None);
    }
}

#[test]
fn numeric_callback_uses_the_real_mixer_insert_and_channel_mapping() {
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.5; 1000].into(), gain: 1.0, track: 0 }]));
    assert!(control.set_chain(0, vec![Box::new(GainProcessor(0.5))]));
    control.play();
    let mut callback = CallbackRenderer::new(rt, 48_000);
    let mut output = [99u16; 4 * 32];
    callback.process(&mut output, 4).unwrap();
    assert_eq!(&output[..4], &[40960, 40960, 32768, 32768]);
    assert_eq!(control.position(), 32);
    assert_eq!(control.transport(), Transport::Playing);
}

#[test]
fn an_oversized_physical_callback_is_rejected_without_advancing_the_mixer() {
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; MAX_BLOCK * 3].into(), gain: 1.0, track: 0 }]));
    control.play();
    let mut callback = CallbackRenderer::new(rt, 48_000);
    let mut output = vec![0.0; (MAX_BLOCK + 17) * 2];
    assert_eq!(callback.process_f32(&mut output, 2), Err(CallbackBufferError { frames: MAX_BLOCK + 17, max_frames: MAX_BLOCK }));
    assert_eq!(control.position(), 0);
    assert!(output.iter().all(|s| *s == 0.0));
    let mut integer = vec![0u16; (MAX_BLOCK + 1) * 2];
    assert_eq!(callback.process(&mut integer, 2), Err(CallbackBufferError { frames: MAX_BLOCK + 1, max_frames: MAX_BLOCK }));
    assert!(integer.iter().all(|s| *s == u16::from_f32(0.0)));
    assert_eq!(control.position(), 0);
    assert_eq!(control.shared.underruns.load(std::sync::atomic::Ordering::Acquire), 0);
    callback.process_f32(&mut output[..MAX_BLOCK * 2], 2).unwrap();
    assert_eq!(control.position(), MAX_BLOCK as u64);
    assert!(output[..MAX_BLOCK * 2].iter().all(|s| *s == 0.25));
}

fn tone(frequency: f64) -> Vec<f32> {
    (0..96_000).map(|i| (2.0 * PI * frequency * i as f64 / 48_000.0).sin() as f32 * 0.5).collect()
}

fn resampled_tone(frequency: f64, device_rate: u32) -> (Vec<f32>, u64) {
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: tone(frequency).into(), gain: 1.0, track: 0 }]));
    control.play();
    let mut callback = CallbackRenderer::new(rt, 48_000);
    callback.set_device_rate(device_rate).unwrap();
    let mut output = vec![0.0; device_rate as usize / 10];
    for block in output.chunks_mut(1024) { callback.process_f32(block, 1).unwrap(); }
    (output, control.position())
}

#[test]
fn different_physical_rates_preserve_pitch_duration_and_fixed_processing_frames() {
    for rate in [44_100, 48_000, 96_000] {
        let (output, position) = resampled_tone(440.0, rate);
        // Source prefetch (128 frames) and filter lookahead are bounded; the
        // timeline itself is still at 48k and advances about 0.1 seconds.
        assert!((4_800..=4_944).contains(&position), "rate {rate}, source position {position}");
        let from = 100;
        let error = output[from..].iter().enumerate().map(|(i, sample)| {
            let expected = (2.0 * PI * 440.0 * (from + i) as f64 / rate as f64).sin() * 0.5;
            (*sample as f64 - expected).powi(2)
        }).sum::<f64>() / (output.len() - from) as f64;
        assert!(error.sqrt() < 0.01, "rate {rate}, RMS error {}", error.sqrt());
    }
}

#[test]
fn admitted_downsampling_attenuates_a_tone_above_the_new_nyquist_frequency() {
    let (output, _) = resampled_tone(23_900.0, 44_100);
    let rms = (output[100..].iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / (output.len() - 100) as f64).sqrt();
    // 44.1k admits a much narrower transition band than the retired 24k case;
    // require attenuation relative to the input tone's 0.354 RMS amplitude.
    assert!(rms < 0.1, "anti-alias filter leaked an out-of-band tone: RMS {rms}");
}

#[test]
fn a_rejected_physical_rate_preserves_the_active_renderer_and_transport() {
    let (mut control, rt) = channel(48_000, MAX_BLOCK);
    control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; 1000].into(), gain: 1.0, track: 0 }]));
    control.play();
    let mut callback = CallbackRenderer::new(rt, 48_000);
    let mut output = [0.0; 64];
    callback.process_f32(&mut output, 2).unwrap();
    let position = control.position();
    for rate in [0, 8_000, 24_000, 44_099] {
        assert_eq!(callback.set_device_rate(rate).unwrap_err().kind, DeviceErrorKind::UnsupportedConfig);
        assert_eq!(control.position(), position);
    }
    callback.process_f32(&mut output, 2).unwrap();
    assert_eq!(control.position(), position + 32);
    assert!(output.iter().all(|s| *s == 0.25));
}

#[test]
fn silence_after_unsigned_conversion_uses_the_device_origin() {
    let (_control, rt) = channel(48_000, MAX_BLOCK);
    let mut callback = CallbackRenderer::new(rt, 48_000);
    let mut output = [0u64; 64];
    callback.process(&mut output, 2).unwrap();
    assert!(output.iter().all(|sample| *sample == u64::from_f32(0.0)));
}
