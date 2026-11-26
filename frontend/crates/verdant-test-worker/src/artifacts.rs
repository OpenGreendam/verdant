use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::RenderError;
use crate::synthesis::{Pitch, Segment};

pub(crate) fn wav(path: &Path, rate: u32, samples: &[f64]) -> Result<(), RenderError> {
    let data_size = u32::try_from(samples.len() * 4).map_err(|error| error.to_string())?;
    let mut file = BufWriter::new(File::create(path)?);
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_size).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&3u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * 4).to_le_bytes())?;
    file.write_all(&4u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    for sample in samples {
        file.write_all(&(*sample as f32).to_le_bytes())?;
    }
    file.flush()?;
    Ok(())
}

fn npy(mut writer: impl Write, values: &[f64]) -> Result<(), RenderError> {
    let mut header = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': ({},), }}", values.len());
    // The retained Python helper adds a full block when already aligned.
    let padding = 64 - (10 + header.len() + 1) % 64;
    header.extend(std::iter::repeat_n(' ', padding));
    header.push('\n');
    writer.write_all(b"\x93NUMPY\x01\x00")?;
    writer.write_all(&(header.len() as u16).to_le_bytes())?;
    writer.write_all(header.as_bytes())?;
    for value in values {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

pub(crate) fn f0_npz(path: &Path, segments: &[Segment], track: Option<&Value>) -> Result<(), RenderError> {
    const HOP: f64 = 0.005;
    let mut pitch = track.map(Pitch::new);
    let mut times = Vec::new();
    let mut f0 = Vec::new();
    let mut start = 0.0;
    for segment in segments {
        let end = start + segment.duration;
        let next = end + segment.gap;
        let end_frame = (next / HOP).round() as usize;
        for frame in times.len()..end_frame {
            let time = (frame as f64 + 0.5) * HOP;
            times.push(time);
            f0.push(if time < end { pitch.as_mut().map_or(segment.f0, |pitch| pitch.at(time)) } else { 0.0 });
        }
        start = next;
    }
    let mut zip = ZipWriter::new(BufWriter::new(File::create(path)?));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).unix_permissions(0o600);
    for (name, values) in [("f0_hz.npy", f0.as_slice()), ("frame_times_seconds.npy", times.as_slice()), ("frame_period_seconds.npy", &[HOP])] {
        zip.start_file(name, options).map_err(|error| error.to_string())?;
        npy(&mut zip, values)?;
    }
    zip.finish().map_err(|error| error.to_string())?.flush()?;
    Ok(())
}

pub(crate) fn json_file(path: &Path, value: &Value, pretty: bool) -> Result<(), RenderError> {
    let mut file = BufWriter::new(File::create(path)?);
    if pretty {
        serde_json::to_writer_pretty(&mut file, value)
    } else {
        serde_json::to_writer(&mut file, value)
    }.map_err(|error| error.to_string())?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

pub(crate) fn manifest(workspace: &Path, folder: &Path, corrupt: bool) -> Result<Vec<Value>, RenderError> {
    let mut paths = Vec::new();
    collect(folder, &mut paths)?;
    paths.sort_unstable();
    let mut artifacts = Vec::with_capacity(paths.len());
    let mut buffer = [0u8; 16 * 1024];
    for path in paths {
        let mut file = File::open(&path)?;
        let mut digest = Sha256::new();
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        let sha256 = if corrupt && path.extension().is_some_and(|extension| extension == "wav") {
            "0".repeat(64)
        } else {
            hex::encode(digest.finalize())
        };
        let relative = path.strip_prefix(workspace).map_err(|error| error.to_string())?;
        // Artifact paths on the worker protocol use '/' on every host.
        #[cfg(windows)]
        let relative = relative.to_string_lossy().replace('\\', "/");
        artifacts.push(json!({
            "path": relative,
            "bytes": file.metadata()?.len(),
            "sha256": sha256,
        }));
    }
    Ok(artifacts)
}

fn collect(folder: &Path, paths: &mut Vec<PathBuf>) -> Result<(), RenderError> {
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(&path, paths)?;
        } else if path.is_file() && path.extension().is_some_and(|extension| extension == "wav" || extension == "npz" || extension == "json") {
            paths.push(path);
        }
    }
    Ok(())
}
