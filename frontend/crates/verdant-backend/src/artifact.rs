//! Artifact verification (SPEC §11, T12): a completed job's files become
//! playable only after the path stays inside the worker workspace, byte size
//! and SHA-256 match what the worker published, and the WAV decodes to finite
//! samples. Partial or cancelled outputs never reach this point.

use std::fs;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::post::TimeMap;
use crate::scheduler::{F0Kind, F0Track, VerifiedAudio, VerifiedF0Track};
use crate::wire::{Artifact, Route};

/// Which artifact is the audible result of a route.
pub fn playable_suffix(route: Route) -> &'static str {
    playable_suffix_for(route, false)
}

/// The requested stage is part of the job, never inferred from returned files.
pub fn playable_suffix_for(route: Route, _wideband: bool) -> &'static str {
    match route {
        Route::VerdantVoice => "/verdant-voice.wav",
        Route::C0 => "/native.wav",
    }
}

pub fn resolve_inside(workspace: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("artifact path {rel:?} is not a plain workspace-relative path"));
    }
    let root = workspace.canonicalize().map_err(|e| format!("workspace {}: {e}", workspace.display()))?;
    let full = root.join(p).canonicalize().map_err(|e| format!("{rel}: {e}"))?;
    if !full.starts_with(&root) {
        return Err(format!("artifact {rel:?} escapes the workspace"));
    }
    Ok(full)
}

pub fn verify_file(workspace: &Path, a: &Artifact) -> Result<(PathBuf, Vec<u8>), String> {
    let path = resolve_inside(workspace, &a.path)?;
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", a.path))?;
    if bytes.len() as u64 != a.bytes {
        return Err(format!("{}: {} bytes on disk, worker published {}", a.path, bytes.len(), a.bytes));
    }
    let digest = hex::encode(Sha256::digest(&bytes));
    if !digest.eq_ignore_ascii_case(&a.sha256) {
        return Err(format!("{}: SHA-256 mismatch", a.path));
    }
    Ok((path, bytes))
}

pub fn verify_and_decode(workspace: &Path, route: Route, artifacts: &[Artifact]) -> Result<VerifiedAudio, String> {
    verify_and_decode_with_options(workspace, route, false, artifacts)
}

pub fn verify_and_decode_with_options(workspace: &Path, route: Route, wideband: bool, artifacts: &[Artifact]) -> Result<VerifiedAudio, String> {
    if wideband && route == Route::C0 { return Err("wideband output is retired".into()); }
    let suffix = playable_suffix(route);
    let wavs: Vec<&Artifact> = artifacts.iter().filter(|a| a.path.ends_with(suffix)).collect();
    let [wav] = wavs.as_slice() else {
        return Err(format!("expected exactly one *{suffix} artifact, found {}", wavs.len()));
    };
    let (path, bytes) = verify_file(workspace, wav)?;
    let (sample_rate, samples) = decode_wav(&bytes)?;
    let mut f0_tracks = Vec::new();
    let mut time_map = None;
    let mut pitch = artifacts.iter().filter(|a| a.path.ends_with("/native.npz"));
    if let Some(artifact) = pitch.next() {
        if pitch.next().is_some() {
            return Err("expected at most one native.npz artifact".into());
        }
        let (source, bytes) = verify_file(workspace, artifact)?;
        let decoded = read_pitch_from_npz(&bytes)?;
        if decoded.tracks.iter().any(|track| track.kind != F0Kind::NativeTarget) {
            return Err(format!("{}: F0 fields disagree with the declared artifact source", artifact.path));
        }
        time_map = decoded.time_map;
        f0_tracks.extend(decoded.tracks.into_iter().map(|track| VerifiedF0Track {
            track, source: source.clone(), sha256: artifact.sha256.clone(),
        }));
    }
    Ok(VerifiedAudio { path, sha256: wav.sha256.clone(), sample_rate, samples: samples.into(), f0_tracks, time_map })
}

/// Decode RIFF/WAVE PCM 16/24/32-bit or IEEE float32, mixed to mono.
pub fn decode_wav(b: &[u8]) -> Result<(u32, Vec<f32>), String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut pos = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= b.len() {
        let tag = &b[pos..pos + 4];
        let size = u32::from_le_bytes(b[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = pos + 8;
        let end = body.checked_add(size).ok_or("chunk size overflow")?;
        if end > b.len() {
            return Err("truncated chunk".into());
        }
        match tag {
            b"fmt " if size >= 16 => {
                let r16 = |o: usize| u16::from_le_bytes(b[body + o..body + o + 2].try_into().unwrap());
                let mut format = r16(0);
                if format == 0xFFFE && size >= 26 {
                    format = r16(24); // subformat GUID's first two bytes
                }
                let rate = u32::from_le_bytes(b[body + 4..body + 8].try_into().unwrap());
                fmt = Some((format, r16(2), rate, r16(14)));
            }
            b"data" => data = Some(&b[body..end]),
            _ => {}
        }
        pos = end + (size & 1);
    }
    let (format, channels, rate, bits) = fmt.ok_or("missing fmt chunk")?;
    let data = data.ok_or("missing data chunk")?;
    if channels == 0 || !(8_000..=192_000).contains(&rate) {
        return Err(format!("unsupported channels={channels} rate={rate}"));
    }
    let width = (bits as usize).div_ceil(8);
    let frame = width * channels as usize;
    if frame == 0 || data.len() % frame != 0 {
        return Err("data length is not a whole number of frames".into());
    }
    let read = |s: &[u8]| -> Result<f32, String> {
        Ok(match (format, bits) {
            (1, 16) => i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0,
            (1, 24) => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8_388_608.0,
            (1, 32) => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0,
            (3, 32) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            _ => return Err(format!("unsupported WAV format {format}/{bits}-bit")),
        })
    };
    let mut out = Vec::with_capacity(data.len() / frame);
    for f in data.chunks_exact(frame) {
        let mut acc = 0.0f32;
        for c in f.chunks_exact(width) {
            acc += read(c)?;
        }
        let v = acc / channels as f32;
        if !v.is_finite() {
            return Err("non-finite sample".into());
        }
        out.push(v);
    }
    if out.is_empty() {
        return Err("no audio frames".into());
    }
    Ok((rate, out))
}

/// Minimal NPZ reader: zip local headers (stored or deflate) holding `.npy`
/// arrays. Retains actual times and separates targets from output estimates.
/// Both float32 neural arrays and float64 native arrays are supported.
pub struct DecodedPitchArtifact {
    pub tracks: Vec<F0Track>,
    pub time_map: Option<TimeMap>,
}

pub fn read_pitch_from_npz(b: &[u8]) -> Result<DecodedPitchArtifact, String> {
    let mut arrays = std::collections::HashMap::new();
    let mut pos = 0;
    while pos + 30 <= b.len() && b[pos..pos + 4] == [0x50, 0x4b, 0x03, 0x04] {
        let r16 = |o: usize| u16::from_le_bytes(b[pos + o..pos + o + 2].try_into().unwrap()) as usize;
        let r32 = |o: usize| u32::from_le_bytes(b[pos + o..pos + o + 4].try_into().unwrap()) as usize;
        let flags = r16(6);
        let method = r16(8);
        let mut csize = r32(18);
        let name_len = r16(26);
        let extra_len = r16(28);
        if flags & 0x08 != 0 {
            return Err("zip data descriptors not supported".into());
        }
        let name_start = pos + 30;
        let data_start = name_start + name_len + extra_len;
        if data_start > b.len() {
            return Err("truncated zip header".into());
        }
        if csize == 0xFFFF_FFFF {
            // numpy writes ZIP64 entries: sizes live in extra field 0x0001
            // as (uncompressed u64, compressed u64).
            let extra = &b[name_start + name_len..data_start];
            let mut e = 0;
            while e + 4 <= extra.len() {
                let id = u16::from_le_bytes([extra[e], extra[e + 1]]);
                let len = u16::from_le_bytes([extra[e + 2], extra[e + 3]]) as usize;
                if id == 0x0001 && len >= 16 && e + 4 + 16 <= extra.len() {
                    csize = u64::from_le_bytes(extra[e + 12..e + 20].try_into().unwrap()) as usize;
                }
                e += 4 + len;
            }
            if csize == 0xFFFF_FFFF {
                return Err("zip64 entry without size field".into());
            }
        }
        let data_end = data_start.checked_add(csize).filter(|&e| e <= b.len()).ok_or("truncated zip entry")?;
        let name = String::from_utf8_lossy(&b[name_start..name_start + name_len]).to_string();
        let raw = &b[data_start..data_end];
        if matches!(name.as_str(), "f0_hz.npy" | "f0_target_hz.npy" | "f0_measured_hz.npy" | "frame_times_seconds.npy"
            | "time_map_render_seconds.npy" | "time_map_authored_seconds.npy" | "frame_period_seconds.npy") {
            let bytes = match method {
                0 => raw.to_vec(),
                8 => miniz_oxide::inflate::decompress_to_vec(raw).map_err(|e| format!("inflate {name}: {e:?}"))?,
                m => return Err(format!("zip method {m} not supported")),
            };
            if arrays.insert(name.clone(), read_npy_float_1d(&bytes)?).is_some() {
                return Err(format!("duplicate NPZ array {name}"));
            }
        }
        pos = data_end;
    }
    let times = arrays.remove("frame_times_seconds.npy").ok_or("frame_times_seconds missing")?;
    if times.is_empty() || times.iter().any(|t| !t.is_finite() || *t < 0.0) {
        return Err("frame times must contain finite nonnegative values".into());
    }
    let explicit_period=arrays.remove("frame_period_seconds.npy");
    let hop=if times.len()>1 { times[1]-times[0] } else {
        let period=explicit_period.as_ref().filter(|values|values.len()==1)
            .ok_or("one pitch frame requires an explicit frame_period_seconds")?;
        period[0]
    };
    if explicit_period.as_ref().is_some_and(|values|values.len()!=1 || !values[0].is_finite() || (values[0]-hop).abs()>1e-8_f64.max(hop*1e-6)) {
        return Err("pitch frame period differs from its time axis".into());
    }
    if !(hop.is_finite() && hop > 0.0) || times.windows(2).any(|p| p[1] <= p[0] || ((p[1] - p[0]) - hop).abs() > 1e-8_f64.max(hop * 1e-6)) {
        return Err("bad frame hop".into());
    }
    let mut tracks = Vec::new();
    for (name, kind) in [("f0_hz.npy", F0Kind::NativeTarget), ("f0_target_hz.npy", F0Kind::NeuralSupplied), ("f0_measured_hz.npy", F0Kind::NeuralEstimate)] {
        if let Some(f0) = arrays.remove(name) {
            if f0.len() != times.len() || f0.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > crate::f0::MAX_CONTROL_F0_HZ) {
                return Err(format!("{name}: invalid F0 values or f0/time length mismatch"));
            }
            tracks.push(F0Track { kind, first_frame_seconds: times[0], hop_seconds: hop,
                                  hz: f0.iter().map(|&v| v as f32).collect::<Vec<_>>().into() });
        }
    }
    if tracks.is_empty() {
        return Err("no supported F0 array".into());
    }
    let time_map = match (arrays.remove("time_map_render_seconds.npy"), arrays.remove("time_map_authored_seconds.npy")) {
        (None, None) => None,
        (Some(render), Some(authored)) => Some(TimeMap::new(render, authored)?),
        _ => return Err("time mapping requires both render and authored arrays".into()),
    };
    Ok(DecodedPitchArtifact { tracks, time_map })
}

fn read_npy_float_1d(b: &[u8]) -> Result<Vec<f64>, String> {
    if b.len() < 10 || &b[0..6] != b"\x93NUMPY" {
        return Err("not an npy array".into());
    }
    let (header_len, start): (usize, usize) = match b[6] {
        1 => (u16::from_le_bytes([b[8], b[9]]) as usize, 10),
        2 | 3 if b.len() >= 12 => (u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize, 12),
        v => return Err(format!("npy version {v}")),
    };
    let end = start.checked_add(header_len).filter(|&end| end <= b.len()).ok_or("truncated npy header")?;
    let header = std::str::from_utf8(&b[start..end]).map_err(|_| "npy header not UTF-8")?;
    let width = if header.contains("'descr': '<f8'") { 8 } else if header.contains("'descr': '<f4'") { 4 } else { 0 };
    if width == 0 || !header.contains("'fortran_order': False") {
        return Err(format!("unsupported npy header {header}"));
    }
    let shape = header.split("'shape': (").nth(1).and_then(|s| s.split(')').next()).ok_or("npy shape missing")?;
    let dimensions: Vec<&str> = shape.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    if dimensions.len() != 1 || !shape.contains(',') {
        return Err("expected a one-dimensional npy array".into());
    }
    let count = dimensions[0].parse::<usize>().map_err(|_| "invalid npy shape")?;
    let data = &b[end..];
    if count.checked_mul(width) != Some(data.len()) {
        return Err("npy data length differs from declared shape".into());
    }
    Ok(if width == 8 {
        data.chunks_exact(8).map(|c| f64::from_le_bytes(c.try_into().unwrap())).collect()
    } else {
        data.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap()) as f64).collect()
    })
}
