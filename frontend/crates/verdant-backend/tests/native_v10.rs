//! Original-source output admission.
use verdant_backend::artifact::verify_and_decode_with_options;
use verdant_backend::wire::Artifact;
use verdant_backend::Route;

fn wav(sample: i16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&38_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&44100_u32.to_le_bytes());
    bytes.extend_from_slice(&88200_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&sample.to_le_bytes());
    bytes
}

#[test]
fn original_route_plays_native_output_and_rejects_wideband() {
    let root = std::env::temp_dir().join(format!("verdant-v10-artifacts-{}", std::process::id()));
    std::fs::create_dir_all(root.join("job")).unwrap();
    let mut artifacts = Vec::new();
    for (name, sample) in [("native.wav", 2000), ("wideband.wav", 4000)] {
        let bytes = wav(sample);
        std::fs::write(root.join("job").join(name), &bytes).unwrap();
        artifacts.push(Artifact {
            path: format!("job/{name}"),
            bytes: bytes.len() as u64,
            sha256: verdant_domain::digest::sha256_hex(&bytes),
        });
    }
    let native = verify_and_decode_with_options(&root, Route::C0, false, &artifacts).unwrap();

    assert!(native.path.ends_with("native.wav"));

    assert!(verify_and_decode_with_options(&root, Route::C0, true, &artifacts).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

