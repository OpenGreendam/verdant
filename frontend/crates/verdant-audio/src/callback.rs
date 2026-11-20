//! The actual device callback body, reusable by hardware-free tests and the
//! bounded callback benchmark. All memory and rate-conversion coefficients are
//! prepared before a stream starts. The timeline/insert rate stays fixed when
//! physical devices change; only this output boundary changes rate.

use crate::convert::{self, DeviceSample};
use crate::{DeviceError, DeviceErrorKind, MAX_BLOCK};
use crate::engine::RtState;

/// Lower rates exceed the admitted source-frame burst at the device boundary.
pub const MIN_DEVICE_RATE: u32 = 44_100;
const TAPS: usize = 32;
const PHASES: usize = 512;
const SOURCE_BLOCK: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackBufferError {
    pub frames: usize,
    pub max_frames: usize,
}
impl std::fmt::Display for CallbackBufferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "physical callback has {} frames; admitted maximum is {}", self.frames, self.max_frames)
    }
}
impl std::error::Error for CallbackBufferError {}

/// Owns the mixer and preallocated output-format/rate adaptation buffers.
/// A single live callback owns this value; no mutex, shared mutable reference,
/// allocation, device operation or channel send occurs in `process_*`.
pub struct CallbackRenderer {
    rt: RtState,
    sample_rate: u32,
    device_rate: u32,
    scratch: Vec<f32>,
    source: [f32; SOURCE_BLOCK * 2],
    source_at: usize,
    history: [[f32; 2]; TAPS],
    head: usize,
    phase: f64,
    step: f64,
    primed: bool,
    adapted: bool,
    coefficients: Vec<f32>,
}

impl CallbackRenderer {
    pub fn new(rt: RtState, sample_rate: u32) -> Self {
        assert!(sample_rate > 0);
        CallbackRenderer {
            rt,
            sample_rate,
            device_rate: sample_rate,
            scratch: vec![0.0; MAX_BLOCK * 2],
            source: [0.0; SOURCE_BLOCK * 2],
            source_at: SOURCE_BLOCK,
            history: [[0.0; 2]; TAPS],
            head: 0,
            phase: 0.0,
            step: 1.0,
            primed: false,
            adapted: false,
            coefficients: Vec::new(),
        }
    }

    pub fn validate_device_rate(rate: u32) -> Result<(), DeviceError> {
        if rate < MIN_DEVICE_RATE {
            return Err(DeviceError::new(DeviceErrorKind::UnsupportedConfig,
                format!("output rate {rate} Hz is below the admitted minimum {MIN_DEVICE_RATE} Hz")));
        }
        Ok(())
    }

    /// Control/worker thread only, while no callback owns the renderer.
    /// Retains unread PCM, filter history, timeline, mixer and insert state.
    /// Rejection leaves all renderer state untouched.
    pub fn set_device_rate(&mut self, rate: u32) -> Result<(), DeviceError> {
        Self::validate_device_rate(rate)?;
        self.configure_rate(rate);
        Ok(())
    }

    /// An explicitly silent clock follows the source rate; it is not a
    /// physical output admission and must not reinterpret source PCM.
    pub(crate) fn use_source_clock(&mut self) {
        self.configure_rate(self.sample_rate);
    }

    fn configure_rate(&mut self, rate: u32) {
        if rate == self.device_rate {
            return;
        }
        self.device_rate = rate;
        self.step = self.sample_rate as f64 / rate as f64;
        self.adapted |= rate != self.sample_rate;
        if !self.adapted {
            return;
        }
        if self.coefficients.is_empty() {
            self.coefficients.resize(TAPS * PHASES, 0.0);
        }
        let cutoff = (rate as f64 / self.sample_rate as f64).min(1.0) * 0.97;
        let pi = std::f64::consts::PI;
        for p in 0..PHASES {
            let phase = p as f64 / PHASES as f64;
            let coefficients = &mut self.coefficients[p * TAPS..(p + 1) * TAPS];
            let mut sum = 0.0;
            for (i, coefficient) in coefficients.iter_mut().enumerate() {
                let distance = i as f64 - (TAPS / 2) as f64 - phase;
                let argument = pi * distance * cutoff;
                let sinc = if argument.abs() < 1e-12 { 1.0 } else { argument.sin() / argument };
                let window_phase = i as f64 / (TAPS - 1) as f64;
                let window = 0.35875 - 0.48829 * (2.0 * pi * window_phase).cos()
                    + 0.14128 * (4.0 * pi * window_phase).cos() - 0.01168 * (6.0 * pi * window_phase).cos();
                *coefficient = (sinc * window * cutoff) as f32;
                sum += *coefficient as f64;
            }
            for coefficient in coefficients {
                *coefficient = (*coefficient as f64 / sum) as f32;
            }
        }
    }

    /// Interleaved f32 fast path: no scratch copy or conversion at equal rates.
    /// Reject a whole oversized device callback before publishing source PCM.
    pub fn process_f32(&mut self, out: &mut [f32], channels: usize) -> Result<(), CallbackBufferError> {
        if self.adapted { return self.process(out, channels); }
        if channels == 0 {
            out.fill(0.0);
            return Ok(());
        }
        let frames = out.len().div_ceil(channels);
        if frames > MAX_BLOCK {
            out.fill(0.0);
            return Err(CallbackBufferError { frames, max_frames: MAX_BLOCK });
        }
        self.rt.process(out, channels);
        Ok(())
    }

    /// Numeric output path. f32 remains the mixer's format; wire conversion
    /// uses saturating arithmetic with format-correct unsigned silence.
    pub fn process<T: DeviceSample>(&mut self, out: &mut [T], channels: usize) -> Result<(), CallbackBufferError> {
        if channels == 0 {
            out.fill(T::from_f32(0.0));
            return Ok(());
        }
        let requested = out.len().div_ceil(channels);
        if requested > MAX_BLOCK {
            out.fill(T::from_f32(0.0));
            return Err(CallbackBufferError { frames: requested, max_frames: MAX_BLOCK });
        }
        let frames = out.len() / channels;
        if self.adapted {
            if self.rt.prepare_callback() {
                // A seek/stop invalidates both unread PCM and the old filter
                // neighbourhood. Keeping either can play the previous phone
                // after the transport has moved elsewhere.
                self.source_at = SOURCE_BLOCK;
                self.history.fill([0.0; 2]);
                self.head = 0;
                self.phase = 0.0;
                self.primed = false;
            }
            if self.rt.explicitly_paused() {
                // Hold the conversion clock and unread PCM so resume neither
                // leaks old samples during pause nor skips prefetched samples.
                self.rt.process_prepared(&mut [], 2);
                out.fill(T::from_f32(0.0));
                return Ok(());
            }
            for frame in out.chunks_exact_mut(channels) {
                let [left, right] = self.resampled_frame();
                if channels == 1 {
                    frame[0] = T::from_f32((left + right) * 0.5);
                } else {
                    frame[0] = T::from_f32(left);
                    frame[1] = T::from_f32(right);
                    frame[2..].fill(T::from_f32(0.0));
                }
            }
            out[frames * channels..].fill(T::from_f32(0.0));
        } else {
            let stereo = &mut self.scratch[..frames * 2];
            self.rt.process(stereo, 2);
            convert::interleave(stereo, out, channels);
        }
        Ok(())
    }

    fn next_source(&mut self) -> [f32; 2] {
        if self.source_at == SOURCE_BLOCK {
            self.rt.process_prepared(&mut self.source, 2);
            self.source_at = 0;
        }
        let i = self.source_at * 2;
        self.source_at += 1;
        [self.source[i], self.source[i + 1]]
    }

    fn push_source(&mut self) {
        let next = self.next_source();
        self.history[self.head] = next;
        self.head = (self.head + 1) % TAPS;
    }

    fn resampled_frame(&mut self) -> [f32; 2] {
        if !self.primed {
            for _ in 0..TAPS / 2 {
                self.push_source();
            }
            self.primed = true;
        }
        let p = ((self.phase * PHASES as f64) as usize).min(PHASES - 1);
        let coefficients = &self.coefficients[p * TAPS..(p + 1) * TAPS];
        let mut result = [0.0, 0.0];
        for (i, coefficient) in coefficients.iter().enumerate() {
            let source = self.history[(self.head + i) % TAPS];
            result[0] += source[0] * coefficient;
            result[1] += source[1] * coefficient;
        }
        self.phase += self.step;
        while self.phase >= 1.0 {
            self.push_source();
            self.phase -= 1.0;
        }
        result
    }
}
