//! Measured insert I/O for native editors. Capture is fixed-size, allocation
//! free and lock free; serialization and spectral analysis are control work.
//!
//! Meters describe the most recent `window_frames` samples, at most 2048.
//! These are sample peaks and unweighted RMS, not true-peak or loudness meters.
//! Waveforms retain stereo extrema rather than cancelling opposite channels;
//! spectra average channel power for the same reason.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;
use std::time::Instant;

pub const FFT_SIZE: usize = 2048;
pub const WAVEFORM_POINTS: usize = 128;
pub const WAVEFORM_STRIDE: usize = 64;
pub const SPECTRUM_FLOOR_DB: f32 = -120.0;
const CHANNELS: usize = 4;
const FRAME_BYTES: usize = CHANNELS * std::mem::size_of::<f32>();
const MAX_ENCODED_BYTES: usize = (FFT_SIZE * FRAME_BYTES).div_ceil(3) * 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaveformPoint {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum TelemetryError {
    #[error("invalid telemetry sample rate")]
    SampleRate,
    #[error("invalid telemetry dimensions or counters")]
    Dimensions,
    #[error("invalid telemetry base64 or sample byte count")]
    Encoding,
    #[error("non-finite telemetry sample or invalid waveform extrema")]
    NonFinite,
}

/// Bounded wire snapshot. The PCM is little-endian interleaved IL, IR, OL, OR,
/// oldest first. It never carries more than `FFT_SIZE` frames. Decoding checks
/// all dimensions before allocating the sample array or running the FFT.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryFrame {
    pub sample_rate: f64,
    pub frames_total: u64,
    pub window_frames: usize,
    pub samples_b64: String,
    pub waveform_input: Vec<WaveformPoint>,
    pub waveform_output: Vec<WaveformPoint>,
}

/// A client-side, immutable signal snapshot. `captured_at` is the time the
/// client first received these processed frames, not a synthesized animation
/// clock. An unchanged `frames_total` keeps the previous snapshot timestamp.
#[derive(Clone, Debug)]
pub struct TelemetrySnapshot {
    pub sample_rate: f64,
    pub frames_total: u64,
    pub window_frames: usize,
    pub input_peak: [f32; 2],
    pub output_peak: [f32; 2],
    pub input_rms: [f32; 2],
    pub output_rms: [f32; 2],
    pub waveform_input: Vec<WaveformPoint>,
    pub waveform_output: Vec<WaveformPoint>,
    /// Linear FFT bins, including DC and Nyquist; Hz.
    pub spectrum_frequencies: Vec<f32>,
    /// Hann-windowed, coherent-amplitude-corrected stereo power average; dBFS.
    pub spectrum_input_db: Vec<f32>,
    pub spectrum_output_db: Vec<f32>,
    pub captured_at: Instant,
}

/// Owned by a single host session. `capture` does no allocation, locking,
/// serialization, system calls, or FFT work. All buffers have fixed capacity.
pub struct TelemetryCapture {
    sample_rate: f64,
    samples: [[f32; FFT_SIZE]; CHANNELS],
    head: usize,
    count: usize,
    frames_total: u64,
    waveform: [[WaveformPoint; WAVEFORM_POINTS]; 2],
    wave_head: usize,
    wave_count: usize,
    partial: [WaveformPoint; 2],
    partial_count: usize,
}

impl Default for TelemetryCapture {
    fn default() -> Self {
        Self::new(crate::SAMPLE_RATE).expect("fixed host sample rate is valid")
    }
}

impl TelemetryCapture {
    pub fn new(sample_rate: f64) -> Result<Self, TelemetryError> {
        if !valid_sample_rate(sample_rate) { return Err(TelemetryError::SampleRate); }
        Ok(Self {
            sample_rate,
            samples: [[0.0; FFT_SIZE]; CHANNELS],
            head: 0, count: 0, frames_total: 0,
            waveform: [[WaveformPoint::default(); WAVEFORM_POINTS]; 2],
            wave_head: 0, wave_count: 0,
            partial: [WaveformPoint::default(); 2], partial_count: 0,
        })
    }

    /// Rejects mismatched or non-finite blocks atomically. A zero-length block
    /// is a no-op, so stopping playback cannot advance the measurement clock.
    pub fn capture(&mut self, input_l: &[f32], input_r: &[f32],
        output_l: &[f32], output_r: &[f32]) -> Result<(), TelemetryError> {
        let planes = [input_l, input_r, output_l, output_r];
        if planes.iter().any(|plane| plane.len() != input_l.len()) {
            return Err(TelemetryError::Dimensions);
        }
        if planes.iter().any(|plane| plane.iter().any(|value| !value.is_finite())) {
            return Err(TelemetryError::NonFinite);
        }
        for i in 0..input_l.len() {
            for (channel, plane) in planes.iter().enumerate() {
                self.samples[channel][self.head] = plane[i];
            }
            self.head = (self.head + 1) % FFT_SIZE;
            self.count = (self.count + 1).min(FFT_SIZE);
            for pair in 0..2 {
                let left = planes[pair * 2][i];
                let right = planes[pair * 2 + 1][i];
                let point = WaveformPoint { min: left.min(right), max: left.max(right) };
                if self.partial_count == 0 { self.partial[pair] = point; }
                else {
                    self.partial[pair].min = self.partial[pair].min.min(point.min);
                    self.partial[pair].max = self.partial[pair].max.max(point.max);
                }
            }
            self.partial_count += 1;
            if self.partial_count == WAVEFORM_STRIDE {
                for pair in 0..2 { self.waveform[pair][self.wave_head] = self.partial[pair]; }
                self.wave_head = (self.wave_head + 1) % WAVEFORM_POINTS;
                self.wave_count = (self.wave_count + 1).min(WAVEFORM_POINTS);
                self.partial_count = 0;
            }
        }
        self.frames_total = self.frames_total.saturating_add(input_l.len() as u64);
        Ok(())
    }

    /// Control-side only: copies the latest bounded history for IPC. Capture
    /// and snapshot run on the host's single session thread, so no lock or
    /// shared mutable buffer crosses the audio/control boundary.
    pub fn frame(&self) -> TelemetryFrame {
        let mut bytes = Vec::with_capacity(self.count * FRAME_BYTES);
        let first = (self.head + FFT_SIZE - self.count) % FFT_SIZE;
        for index in 0..self.count {
            let at = (first + index) % FFT_SIZE;
            for channel in 0..CHANNELS { bytes.extend_from_slice(&self.samples[channel][at].to_le_bytes()); }
        }
        let waveform = |pair: usize| {
            let completed = self.wave_count.min(WAVEFORM_POINTS - usize::from(self.partial_count > 0));
            let first = (self.wave_head + WAVEFORM_POINTS - completed) % WAVEFORM_POINTS;
            let mut points = Vec::with_capacity(completed + usize::from(self.partial_count > 0));
            for index in 0..completed { points.push(self.waveform[pair][(first + index) % WAVEFORM_POINTS]); }
            if self.partial_count > 0 { points.push(self.partial[pair]); }
            points
        };
        TelemetryFrame {
            sample_rate: self.sample_rate, frames_total: self.frames_total, window_frames: self.count,
            samples_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
            waveform_input: waveform(0), waveform_output: waveform(1),
        }
    }
}

impl TelemetryFrame {
    /// Run on a background/control thread, never while rendering UI or PCM.
    pub fn analyze(self, captured_at: Instant) -> Result<TelemetrySnapshot, TelemetryError> {
        if !valid_sample_rate(self.sample_rate) { return Err(TelemetryError::SampleRate); }
        if self.window_frames > FFT_SIZE || self.window_frames as u64 > self.frames_total
            || self.window_frames != self.frames_total.min(FFT_SIZE as u64) as usize
            || self.samples_b64.len() > MAX_ENCODED_BYTES
            || self.waveform_input.len() > WAVEFORM_POINTS
            || self.waveform_input.len() != self.waveform_output.len()
            || self.waveform_input.len() != self.frames_total.div_ceil(WAVEFORM_STRIDE as u64)
                .min(WAVEFORM_POINTS as u64) as usize {
            return Err(TelemetryError::Dimensions);
        }
        if self.waveform_input.iter().chain(&self.waveform_output)
            .any(|point| !point.min.is_finite() || !point.max.is_finite() || point.min > point.max) {
            return Err(TelemetryError::NonFinite);
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(&self.samples_b64)
            .map_err(|_| TelemetryError::Encoding)?;
        if bytes.len() != self.window_frames * FRAME_BYTES { return Err(TelemetryError::Encoding); }
        let mut samples = vec![[0.0; CHANNELS]; self.window_frames];
        let mut peak = [0.0f32; CHANNELS];
        let mut squares = [0.0f64; CHANNELS];
        for (frame, encoded) in samples.iter_mut().zip(bytes.chunks_exact(FRAME_BYTES)) {
            for channel in 0..CHANNELS {
                let start = channel * 4;
                let sample = f32::from_le_bytes(encoded[start..start + 4].try_into().unwrap());
                if !sample.is_finite() { return Err(TelemetryError::NonFinite); }
                frame[channel] = sample;
                peak[channel] = peak[channel].max(sample.abs());
                squares[channel] += (sample as f64).powi(2);
            }
        }
        let rms = squares.map(|sum| (sum / self.window_frames.max(1) as f64).sqrt() as f32);
        let [spectrum_input_db, spectrum_output_db] = spectrum(&samples);
        Ok(TelemetrySnapshot {
            sample_rate: self.sample_rate, frames_total: self.frames_total, window_frames: self.window_frames,
            input_peak: [peak[0], peak[1]], output_peak: [peak[2], peak[3]],
            input_rms: [rms[0], rms[1]], output_rms: [rms[2], rms[3]],
            waveform_input: self.waveform_input, waveform_output: self.waveform_output,
            spectrum_frequencies: (0..=FFT_SIZE / 2).map(|bin| (bin as f64 * self.sample_rate / FFT_SIZE as f64) as f32).collect(),
            spectrum_input_db, spectrum_output_db,
            captured_at,
        })
    }
}

fn valid_sample_rate(sample_rate: f64) -> bool {
    sample_rate.is_finite() && (8_000.0..=384_000.0).contains(&sample_rate)
}

#[derive(Clone, Copy, Default)]
struct Complex { re: f64, im: f64 }

fn spectrum(samples: &[[f32; CHANNELS]]) -> [Vec<f32>; 2] {
    let mut powers = [[0.0f64; FFT_SIZE / 2 + 1]; 2];
    if !samples.is_empty() {
        let mut window = [0.0; FFT_SIZE];
        for (index, coefficient) in window.iter_mut().take(samples.len()).enumerate() {
            *coefficient = if samples.len() < 3 { 1.0 }
                else { 0.5 - 0.5 * (TAU * index as f64 / samples.len() as f64).cos() };
        }
        let window_sum = window.iter().sum::<f64>();
        for channel in 0..CHANNELS {
            let mut data = [Complex::default(); FFT_SIZE];
            for (index, frame) in samples.iter().enumerate() {
                data[index].re = frame[channel] as f64 * window[index];
            }
            fft(&mut data);
            for (bin, power) in powers[channel / 2].iter_mut().enumerate() {
                let factor = if bin == 0 || bin == FFT_SIZE / 2 { 1.0 } else { 2.0 } / window_sum;
                *power += (data[bin].re.powi(2) + data[bin].im.powi(2)) * factor.powi(2) * 0.5;
            }
        }
    }
    powers.map(|bins| bins.into_iter().map(|power|
        (10.0 * power.max(1e-12).log10()).max(SPECTRUM_FLOOR_DB as f64) as f32).collect())
}

/// Iterative radix-two FFT. Called four times per requested control snapshot;
/// no spectral work runs in `TelemetryCapture::capture` or the host audio loop.
fn fft(data: &mut [Complex; FFT_SIZE]) {
    let mut reversed = 0;
    for index in 1..FFT_SIZE {
        let mut bit = FFT_SIZE >> 1;
        while reversed & bit != 0 { reversed ^= bit; bit >>= 1; }
        reversed ^= bit;
        if index < reversed { data.swap(index, reversed); }
    }
    let mut width = 2;
    while width <= FFT_SIZE {
        let angle = -TAU / width as f64;
        let root = Complex { re: angle.cos(), im: angle.sin() };
        for start in (0..FFT_SIZE).step_by(width) {
            let mut rotation = Complex { re: 1.0, im: 0.0 };
            for offset in 0..width / 2 {
                let even = data[start + offset];
                let odd = data[start + offset + width / 2];
                let turned = Complex { re: rotation.re * odd.re - rotation.im * odd.im,
                    im: rotation.re * odd.im + rotation.im * odd.re };
                data[start + offset] = Complex { re: even.re + turned.re, im: even.im + turned.im };
                data[start + offset + width / 2] = Complex { re: even.re - turned.re, im: even.im - turned.im };
                rotation = Complex { re: rotation.re * root.re - rotation.im * root.im,
                    im: rotation.re * root.im + rotation.im * root.re };
            }
        }
        width *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(capture: &TelemetryCapture) -> TelemetrySnapshot {
        // Exercise the same serialization and strict decoding as IPC.
        let json = serde_json::to_vec(&capture.frame()).unwrap();
        assert!(json.len() < 60_000);
        serde_json::from_slice::<TelemetryFrame>(&json).unwrap().analyze(Instant::now()).unwrap()
    }

    #[test]
    fn no_audio_and_silence_report_no_signal() {
        let mut capture = TelemetryCapture::default();
        for processed in [false, true] {
            if processed { capture.capture(&[0.0; 256], &[0.0; 256], &[0.0; 256], &[0.0; 256]).unwrap(); }
            let snapshot = measured(&capture);
            assert_eq!(snapshot.frames_total, if processed { 256 } else { 0 });
            assert_eq!(snapshot.input_peak, [0.0; 2]);
            assert_eq!(snapshot.output_rms, [0.0; 2]);
            assert!(snapshot.spectrum_input_db.iter().chain(&snapshot.spectrum_output_db)
                .all(|&db| db == SPECTRUM_FLOOR_DB));
            assert!(snapshot.waveform_input.iter().all(|point| *point == WaveformPoint::default()));
        }
    }

    #[test]
    fn sine_peak_rms_frequency_and_output_gain_are_measured() {
        for sample_rate in [8_000.0, 44_100.0, 48_000.0, 192_000.0] {
            let mut capture = TelemetryCapture::new(sample_rate).unwrap();
            let bin = 128;
            let input: Vec<f32> = (0..FFT_SIZE).map(|i|
                (0.5 * (TAU * bin as f64 * i as f64 / FFT_SIZE as f64).sin()) as f32).collect();
            let opposite: Vec<_> = input.iter().map(|value| -value).collect();
            let output: Vec<_> = input.iter().map(|value| value * 0.5).collect();
            capture.capture(&input, &opposite, &output, &output).unwrap();
            let snapshot = measured(&capture);
            assert!((snapshot.input_peak[0] - 0.5).abs() < 1e-6);
            assert!((snapshot.input_rms[0] - 0.5 / 2.0f32.sqrt()).abs() < 1e-6);
            assert!((snapshot.output_rms[0] / snapshot.input_rms[0] - 0.5).abs() < 1e-6);
            let peak_bin = snapshot.spectrum_input_db.iter().enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b)).unwrap().0;
            assert_eq!(peak_bin, bin);
            assert!((snapshot.spectrum_frequencies[bin] as f64 - sample_rate / 16.0).abs() < 1e-5);
            assert!((snapshot.spectrum_input_db[bin] + 6.0206).abs() < 0.001);
            assert!((snapshot.spectrum_output_db[bin] + 12.0412).abs() < 0.001);
            assert!(snapshot.waveform_input.iter().all(|point| point.min < -0.49 && point.max > 0.49));
        }
    }

    #[test]
    fn ring_wrap_partitioning_and_instances_are_independent() {
        let samples: Vec<f32> = (0..12_345).map(|i| (i % 997) as f32 / 997.0 - 0.5).collect();
        let mut whole = TelemetryCapture::default();
        let mut chunks = TelemetryCapture::default();
        let untouched = TelemetryCapture::default();
        whole.capture(&samples, &samples, &samples, &samples).unwrap();
        for part in samples.chunks(173) { chunks.capture(part, part, part, part).unwrap(); }
        assert_eq!(serde_json::to_value(whole.frame()).unwrap(), serde_json::to_value(chunks.frame()).unwrap());
        let snapshot = measured(&whole);
        assert_eq!(snapshot.frames_total, samples.len() as u64);
        assert_eq!(snapshot.window_frames, FFT_SIZE);
        assert_eq!(snapshot.waveform_input.len(), WAVEFORM_POINTS);
        let last = &samples[samples.len() - FFT_SIZE..];
        let expected = (last.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / FFT_SIZE as f64).sqrt() as f32;
        assert!((snapshot.input_rms[0] - expected).abs() < 1e-7);
        assert_eq!(measured(&untouched).frames_total, 0);
        assert_eq!(measured(&untouched).input_peak, [0.0; 2]);
    }

    #[test]
    fn invalid_capture_is_atomic_and_malformed_wire_is_rejected() {
        let mut capture = TelemetryCapture::default();
        capture.capture(&[0.2; 4], &[0.2; 4], &[0.1; 4], &[0.1; 4]).unwrap();
        let before = serde_json::to_value(capture.frame()).unwrap();
        assert_eq!(capture.capture(&[0.0], &[], &[0.0], &[0.0]), Err(TelemetryError::Dimensions));
        assert_eq!(capture.capture(&[f32::NAN], &[0.0], &[0.0], &[0.0]), Err(TelemetryError::NonFinite));
        assert_eq!(serde_json::to_value(capture.frame()).unwrap(), before);
        let mut frame = capture.frame();
        frame.window_frames = usize::MAX;
        assert!(frame.analyze(Instant::now()).is_err());
        let mut frame = capture.frame();
        frame.samples_b64 = "A".repeat(MAX_ENCODED_BYTES + 1);
        assert!(frame.analyze(Instant::now()).is_err());
        let mut frame = capture.frame();
        frame.samples_b64 = base64::engine::general_purpose::STANDARD.encode([f32::NAN.to_le_bytes(); 16].concat());
        assert!(matches!(frame.analyze(Instant::now()), Err(TelemetryError::NonFinite)));
        let mut frame = capture.frame();
        frame.waveform_input[0].min = f32::INFINITY;
        assert!(frame.analyze(Instant::now()).is_err());
        assert!(TelemetryCapture::new(f64::NAN).is_err());
    }
}
