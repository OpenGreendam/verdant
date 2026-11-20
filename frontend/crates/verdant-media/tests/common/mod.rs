#![allow(dead_code)]

pub fn wav(format: u16, bits: u16, channels: u16, rate: u32, payload: &[u8]) -> Vec<u8> {
    let align = channels * bits / 8;
    let padding = payload.len() & 1;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36u32 + payload.len() as u32 + padding as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt "); bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&format.to_le_bytes()); bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes()); bytes.extend_from_slice(&(rate * u32::from(align)).to_le_bytes());
    bytes.extend_from_slice(&align.to_le_bytes()); bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data"); bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    if padding != 0 { bytes.push(0); }
    bytes
}

pub fn pcm16(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let payload: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    wav(1, 16, channels, rate, &payload)
}

pub fn float32(samples: &[f32], channels: u16, rate: u32) -> Vec<u8> {
    let payload: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    wav(3, 32, channels, rate, &payload)
}

pub fn source(audio: &verdant_media::DecodedAudio) -> verdant_media::AudioSource {
    verdant_media::AudioSource { path: std::env::temp_dir().join("verdant-media-authorized-source.wav").to_string_lossy().into_owned(), sha256: audio.sha256.clone(),
        byte_len: audio.byte_len, sample_rate: audio.sample_rate, channels: audio.channels, frames: audio.frames() }
}
