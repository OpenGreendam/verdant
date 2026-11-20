mod common;
use verdant_media::{ImportLimits, MediaError, decode_bytes};

#[test]
fn signed_pcm_wav_depths_decode_exact_frame_order_and_channel_order() {
    for bits in [16u16, 24, 32] {
        let scale = 1i64 << (bits - 1);
        let values = [-scale, 0, scale / 2, -scale / 2, scale - 1, scale / 4];
        let mut payload = Vec::new();
        for value in values { payload.extend_from_slice(&(value as i32).to_le_bytes()[..usize::from(bits / 8)]); }
        let bytes = common::wav(1, bits, 2, 44_100, &payload);
        let audio = decode_bytes(&bytes, Some("wav"), ImportLimits::default()).unwrap();
        assert_eq!((audio.sample_rate, audio.channels, audio.frames()), (44_100, 2, 3));
        for (actual, expected) in audio.samples.iter().zip(values) {
            assert!((f64::from(*actual) - expected as f64 / scale as f64).abs() < 1e-7, "PCM{bits}: {actual}");
        }
    }
}

#[test]
fn ieee_float_wav_and_mono_are_preserved_without_peak_normalization() {
    let source = [-0.75, 0.25, 1.25, 0.0];
    let bytes = common::float32(&source, 1, 48_000);
    let audio = decode_bytes(&bytes, Some("wav"), ImportLimits::default()).unwrap();
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.samples.as_ref(), source.as_slice());
    assert_eq!(audio.byte_len, bytes.len() as u64);
    assert_eq!(audio.sha256.len(), 64);
}

#[test]
fn aiff_pcm16_is_decoded_by_the_admitted_container_library() {
    // A complete generated two-frame AIFF, with standard 80-bit 44100 Hz COMM.
    let mut bytes = b"FORM".to_vec(); bytes.extend_from_slice(&50u32.to_be_bytes());
    bytes.extend_from_slice(b"AIFFCOMM"); bytes.extend_from_slice(&18u32.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes()); bytes.extend_from_slice(&2u32.to_be_bytes());
    bytes.extend_from_slice(&16u16.to_be_bytes());
    bytes.extend_from_slice(&[0x40, 0x0e, 0xac, 0x44, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(b"SSND"); bytes.extend_from_slice(&12u32.to_be_bytes());
    bytes.extend_from_slice(&[0; 8]); bytes.extend_from_slice(&1000i16.to_be_bytes()); bytes.extend_from_slice(&(-1000i16).to_be_bytes());
    let audio = decode_bytes(&bytes, Some("aiff"), ImportLimits::default()).unwrap();
    assert_eq!((audio.sample_rate, audio.channels, audio.frames()), (44_100, 1, 2));
    assert_eq!(audio.samples.as_ref(), &[1000.0 / 32768.0, -1000.0 / 32768.0]);
}

#[test]
fn truncated_or_nonfinite_wav_and_resource_excess_are_explicit_errors() {
    let mut bytes = common::pcm16(&[1, 2, 3, 4], 2, 48_000);
    bytes.pop();
    assert!(decode_bytes(&bytes, Some("wav"), ImportLimits::default()).is_err());
    let nonfinite = common::float32(&[f32::NAN, 0.0], 1, 48_000);
    assert!(decode_bytes(&nonfinite, Some("wav"), ImportLimits::default()).is_err());
    let bytes = common::pcm16(&[1, 2, 3, 4], 2, 48_000);
    let limits = ImportLimits { max_source_bytes: bytes.len() as u64 - 1, ..Default::default() };
    assert!(matches!(decode_bytes(&bytes, Some("wav"), limits), Err(MediaError::Limit(_))));
    let six_channel = common::pcm16(&[0; 12], 6, 48_000);
    assert!(matches!(decode_bytes(&six_channel, Some("wav"), ImportLimits::default()), Err(MediaError::Channels(6))));
}
