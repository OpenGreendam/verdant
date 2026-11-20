//! Synthetic numeric fixtures only; no voice model or vocoder is required.
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use verdant_backend::artifact::{read_pitch_from_npz, verify_and_decode};
use verdant_backend::cache::ArtifactCache;
use verdant_backend::wire::Artifact;
use verdant_backend::{CacheEntry, F0Kind, Route};
use verdant_domain::digest::sha256_hex;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn npy(values: &[f64], float32: bool) -> Vec<u8> {
    let descriptor = if float32 { "<f4" } else { "<f8" };
    let mut header = format!("{{'descr': '{descriptor}', 'fortran_order': False, 'shape': ({},), }}", values.len());
    header.push_str(&" ".repeat((64 - (10 + header.len() + 1) % 64) % 64));
    header.push('\n');
    let mut bytes = b"\x93NUMPY\x01\x00".to_vec();
    bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
    bytes.extend_from_slice(header.as_bytes());
    for &value in values {
        if float32 {
            bytes.extend_from_slice(&(value as f32).to_le_bytes());
        } else {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

fn npz(arrays: &[(&str, &[f64], bool)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for &(name, values, float32) in arrays {
        zip.start_file(format!("{name}.npy"), SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).unwrap();
        zip.write_all(&npy(values, float32)).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn native_npz(mapping: bool) -> Vec<u8> {
    let mut arrays: Vec<(&str, &[f64], bool)> =
        vec![("f0_hz", &[200., 200., 200.], false), ("frame_times_seconds", &[0.0025, 0.0075, 0.0125], false)];
    if mapping {
        arrays.extend([
            ("time_map_render_seconds", &[0., 0.005, 0.005, 0.015][..], false),
            ("time_map_authored_seconds", &[0., 0.005, 0.006, 0.016][..], false),
        ]);
    }
    npz(&arrays)
}

fn neural_npz() -> Vec<u8> {
    npz(&[
        ("f0_target_hz", &[0., 210., 230.], true),
        ("f0_measured_hz", &[123., 205., 234.], true),
        // Deliberately not the native grid or an inferred 5 ms grid.
        ("frame_times_seconds", &[0.0058, 0.0174, 0.0290], false),
    ])
}

fn wav() -> Vec<u8> {
    let mut b = b"RIFF".to_vec();
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&44100u32.to_le_bytes());
    b.extend_from_slice(&88200u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&[0, 0, 1, 0]);
    b
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!("verdant-f0-sources-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn publish(&self, path: &str, bytes: &[u8]) -> Artifact {
        let output = self.0.join(path);
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(output, bytes).unwrap();
        Artifact { path: path.into(), bytes: bytes.len() as u64, sha256: sha256_hex(bytes) }
    }
    fn artifacts(&self) -> Vec<Artifact> {
        vec![
            self.publish("jobs/test/native.wav", &wav()),
            self.publish("jobs/test/native.npz", &native_npz(true)),
            self.publish("jobs/test/verdant-voice.wav", &wav()),
            self.publish("jobs/test/neural/pc_zero_semitones.npz", &neural_npz()),
        ]
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn float32_neural_target_and_estimate_keep_different_roles_and_values() {
    let decoded = read_pitch_from_npz(&neural_npz()).unwrap();
    assert_eq!(decoded.tracks.len(), 2);
    assert_eq!(decoded.tracks[0].kind, F0Kind::NeuralSupplied);
    assert_eq!(decoded.tracks[1].kind, F0Kind::NeuralEstimate);
    assert_eq!(&*decoded.tracks[0].hz, &[0., 210., 230.]);
    assert_eq!(&*decoded.tracks[1].hz, &[123., 205., 234.]);
    assert!((decoded.tracks[0].first_frame_seconds - 0.0058).abs() < 1e-12);
    assert!((decoded.tracks[0].hop_seconds - 0.0116).abs() < 1e-12);
    assert!(decoded.time_map.is_none());
}

#[test]
fn partial_time_map_or_nonfinite_f0_is_rejected() {
    let partial = npz(&[
        ("f0_hz", &[200., 200.], false),
        ("frame_times_seconds", &[0.0025, 0.0075], false),
        ("time_map_render_seconds", &[0., 0.01], false),
    ]);
    assert!(read_pitch_from_npz(&partial).err().unwrap().contains("both render and authored"));
    let invalid = npz(&[("f0_hz", &[200., f64::NAN], false), ("frame_times_seconds", &[0.0025, 0.0075], false)]);
    assert!(read_pitch_from_npz(&invalid).is_err());
    assert!(read_pitch_from_npz(&native_npz(false)).unwrap().time_map.is_none());
    assert!(read_pitch_from_npz(&native_npz(true)).unwrap().time_map.is_some());
}

#[test]
fn current_routes_read_only_native_target_pitch() {
    let fixture = Fixture::new();
    let artifacts = fixture.artifacts();
    for route in [Route::VerdantVoice, Route::C0] {
        let audio = verify_and_decode(&fixture.0, route, &artifacts).unwrap();
        assert_eq!(audio.f0_tracks.len(), 1);
        assert_eq!(audio.f0_tracks[0].track.kind, F0Kind::NativeTarget);
        assert!(audio.f0_tracks[0].source.ends_with(Path::new("jobs/test/native.npz")));
        assert!(audio.time_map.is_some());
    }
}

#[test]
fn durable_cache_roundtrip_preserves_roles_grids_hashes_and_native_mapping() {
    let fixture = Fixture::new();
    let audio = verify_and_decode(&fixture.0, Route::VerdantVoice, &fixture.artifacts()).unwrap();
    let entry = CacheEntry {
        digest: "a".repeat(64),
        job_id: "test".into(),
        revision: 1,
        route: Route::VerdantVoice,
        audio,
        track: None,
        phrase_index: None,
        start_seconds: 0.,
        seconds: 0.016,
        last_wanted_revision: 1,
    };
    let cache = ArtifactCache::open(fixture.0.join("cache")).unwrap();
    cache.store(&entry).unwrap();
    let restored = cache.restore(&entry.digest).unwrap().unwrap();
    assert_eq!(restored.audio.f0_tracks.len(), 1);
    assert_eq!(format!("{:?}", restored.audio.time_map), format!("{:?}", entry.audio.time_map));
    for (a, b) in restored.audio.f0_tracks.iter().zip(&entry.audio.f0_tracks) {
        assert_eq!(a.track.kind, b.track.kind);
        assert_eq!(a.track.first_frame_seconds, b.track.first_frame_seconds);
        assert_eq!(a.track.hop_seconds, b.track.hop_seconds);
        assert_eq!(a.track.hz, b.track.hz);
        assert_eq!(a.sha256, b.sha256);
    }
    // Storage must not silently bless changed NPZ bytes after verification.
    std::fs::write(&entry.audio.f0_tracks[0].source, b"changed").unwrap();
    assert!(cache.store(&entry).is_err());
}


#[test]
fn high_pitch_npz_preserves_c8_and_enforces_the_host_ceiling() {
    let c8 = 440.0 * 2.0_f64.powf(39.0 / 12.0);
    for float32 in [false, true] {
        for field in ["f0_hz", "f0_target_hz", "f0_measured_hz"] {
            let valid = npz(&[
                (field, &[c8, 5000.0, 0.0], float32),
                ("frame_times_seconds", &[0.0025, 0.0075, 0.0125], false),
            ]);
            let decoded = read_pitch_from_npz(&valid).unwrap();
            assert_eq!(&*decoded.tracks[0].hz, &[c8 as f32, 5000.0, 0.0]);
            let invalid = npz(&[
                (field, &[c8, 5000.001], float32),
                ("frame_times_seconds", &[0.0025, 0.0075], false),
            ]);
            assert!(read_pitch_from_npz(&invalid).is_err());
        }
    }
}

#[test]
fn b6_voice_artifacts_remain_playable_after_verified_cache_roundtrip() {
    let fixture = Fixture::new();
    let b6 = 440.0 * 2.0_f64.powf(26.0 / 12.0);
    let pitch = npz(&[
        ("f0_hz", &[b6, b6, 0.0], false),
        ("frame_times_seconds", &[0.0025, 0.0075, 0.0125], false),
    ]);
    let artifacts = vec![
        fixture.publish("jobs/b6/verdant-voice.wav", &wav()),
        fixture.publish("jobs/b6/native.npz", &pitch),
    ];
    let audio = verify_and_decode(&fixture.0, Route::VerdantVoice, &artifacts).unwrap();
    assert_eq!(&*audio.f0_tracks[0].track.hz, &[b6 as f32, b6 as f32, 0.0]);
    let entry = CacheEntry {
        digest: "b".repeat(64), job_id: "b6".into(), revision: 1, route: Route::VerdantVoice,
        audio, track: None, phrase_index: None, start_seconds: 0.0, seconds: 0.015,
        last_wanted_revision: 1,
    };
    let cache = ArtifactCache::open(fixture.0.join("b6-cache")).unwrap();
    cache.store(&entry).unwrap();
    let restored = cache.restore(&entry.digest).unwrap().unwrap();
    assert_eq!(restored.audio.f0_tracks[0].track.hz, entry.audio.f0_tracks[0].track.hz);
    assert_eq!(restored.audio.sha256, entry.audio.sha256);
}

#[test]
fn one_frame_voice_pitch_requires_and_preserves_its_explicit_clock() {
    let pitch=npz(&[("f0_hz",&[440.0],false),("frame_times_seconds",&[0.0025],false),
        ("frame_period_seconds",&[0.005],false)]);
    let decoded=read_pitch_from_npz(&pitch).unwrap();
    assert_eq!(decoded.tracks[0].hop_seconds,0.005);
    assert_eq!(decoded.tracks[0].first_frame_seconds,0.0025);
    assert_eq!(&*decoded.tracks[0].hz,&[440.0]);
    let missing=npz(&[("f0_hz",&[440.0],false),("frame_times_seconds",&[0.0025],false)]);
    assert!(read_pitch_from_npz(&missing).is_err());
    for period in [0.0,-0.005] {
        let invalid=npz(&[("f0_hz",&[440.0],false),("frame_times_seconds",&[0.0025],false),
            ("frame_period_seconds",&[period],false)]);
        assert!(read_pitch_from_npz(&invalid).is_err());
    }
}

#[test]
fn only_the_canonical_voice_wav_is_playable_and_verified() {
    let fixture=Fixture::new();
    let canonical=fixture.publish("jobs/voice/verdant-voice.wav",&wav());
    let unrelated=fixture.publish("jobs/voice/unrelated.wav",&wav());
    assert!(verify_and_decode(&fixture.0,Route::VerdantVoice,std::slice::from_ref(&unrelated)).is_err());
    let verified=verify_and_decode(&fixture.0,Route::VerdantVoice,&[canonical.clone(),unrelated]).unwrap();
    assert_eq!(verified.sha256,canonical.sha256);
    assert!(verify_and_decode(&fixture.0,Route::VerdantVoice,&[canonical.clone(),canonical.clone()]).is_err());
    let optional=fixture.publish("jobs/voice/verdant-voice-before-music.wav",&wav());
    assert!(verify_and_decode(&fixture.0,Route::VerdantVoice,&[optional]).is_err());
    let mut changed=canonical;changed.sha256="0".repeat(64);
    assert!(verify_and_decode(&fixture.0,Route::VerdantVoice,&[changed]).is_err());
}
