//! Analytic Fourier tables, six-point periodic interpolation and centered FIR.
//! This is the production Rosenberg/impulse implementation, never a sinusoid
//! stand-in. One bounded phase clock spans every voiced frame in the run.

use anyhow::{ensure, Result};
use num_complex::Complex64;
use rustfft::FftPlanner;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::sync::Arc;

pub const OPEN_FRACTION: f64 = 0.45;
pub const CLOSE_FRACTION: f64 = 0.20;
pub const OVERSAMPLE: usize = 8;
pub const PASS_FRACTION: f64 = 0.45;
pub const STOP_FRACTION: f64 = 0.48;
pub const MAX_HARMONICS: usize = 4096;
const TABLE_SAMPLES_PER_HARMONIC: usize = 64;
const MAX_CACHED_TABLES: usize = 8;

fn complex_integral(rate: f64, length: f64) -> Complex64 {
    let sinc_argument = rate * length / (2.0*PI);
    let argument = PI*sinc_argument;
    let sinc = if argument == 0.0 { 1.0 } else { argument.sin()/argument };
    length * Complex64::from_polar(1.0, 0.5*rate*length) * sinc
}

pub fn rosenberg_coefficients(harmonic_count: usize) -> Result<Vec<Complex64>> {
    ensure!(harmonic_count <= MAX_HARMONICS, "Rosenberg harmonic count exceeds bounded source limit");
    let mut result = Vec::with_capacity(harmonic_count);
    for harmonic in 1..=harmonic_count {
        let omega = 2.0*PI*harmonic as f64;
        let sine_integral = |rate:f64, length:f64| {
            (complex_integral(rate-omega, length)-complex_integral(-rate-omega, length))
                / Complex64::new(0.0, 2.0)
        };
        let opening = PI/(2.0*OPEN_FRACTION)*sine_integral(PI/OPEN_FRACTION, OPEN_FRACTION);
        let closing = -PI/(2.0*CLOSE_FRACTION)
            * Complex64::from_polar(1.0, -omega*OPEN_FRACTION)
            * sine_integral(PI/(2.0*CLOSE_FRACTION), CLOSE_FRACTION);
        result.push(opening+closing);
    }
    Ok(result)
}

#[inline]
pub fn harmonic_weight(frequency: f64, sample_rate: f64) -> f64 {
    let fraction = ((frequency/sample_rate-PASS_FRACTION)/(STOP_FRACTION-PASS_FRACTION)).clamp(0.0, 1.0);
    0.5+0.5*(PI*fraction).cos()
}

pub fn source_coefficients(frequency: f64, count: usize, rate: f64,
    source: &str, brightness: f64) -> Result<Vec<Complex64>> {
    ensure!(matches!(source, "rosenberg" | "impulse"), "known periodic source required");
    let mut values = if source == "rosenberg" { rosenberg_coefficients(count)? }
        else { vec![Complex64::new(frequency/rate, 0.0); count] };
    for (index, value) in values.iter_mut().enumerate() {
        let hz = (index+1) as f64*frequency;
        *value *= harmonic_weight(hz, rate);
        if brightness != 0.0 {
            let position = ((hz.max(1000.0)/1000.0).log2()/2.5).clamp(0.0, 1.0);
            *value *= 10.0_f64.powf(brightness*position*position*(3.0-2.0*position)/20.0);
        }
    }
    Ok(values)
}

struct CoefficientCache(VecDeque<(usize, Arc<Vec<Complex64>>)>);
impl CoefficientCache {
    fn get(&mut self, count: usize) -> Result<Arc<Vec<Complex64>>> {
        if let Some(index) = self.0.iter().position(|entry| entry.0 == count) {
            let entry = self.0.remove(index).unwrap();
            let result = Arc::clone(&entry.1);
            self.0.push_front(entry);
            return Ok(result);
        }
        let value = Arc::new(rosenberg_coefficients(count)?);
        self.0.push_front((count, Arc::clone(&value)));
        if self.0.len() > 32 { self.0.pop_back(); }
        Ok(value)
    }
}

fn periodic_table(frequency: f64, rate: u32, source: &str, brightness: f64,
    planner: &mut FftPlanner<f64>, cache: &mut CoefficientCache,
    spectrum: &mut Vec<Complex64>, scratch: &mut Vec<Complex64>,
    mut table: Vec<f64>) -> Result<Vec<f64>> {
    let count = (STOP_FRACTION*f64::from(rate)/frequency).floor() as usize;
    let minimum = 1024.max(TABLE_SAMPLES_PER_HARMONIC*(count+1));
    let size = minimum.next_power_of_two();
    spectrum.resize(size, Complex64::new(0.0, 0.0));
    spectrum.fill(Complex64::new(0.0, 0.0));
    let coefficients = if source == "rosenberg" { Some(cache.get(count)?) } else { None };
    for harmonic in 1..=count {
        let hz = harmonic as f64*frequency;
        let mut value = coefficients.as_ref().map_or(
            Complex64::new(frequency/f64::from(rate), 0.0), |values| values[harmonic-1]);
        value *= harmonic_weight(hz, f64::from(rate));
        if brightness != 0.0 {
            let position = ((hz.max(1000.0)/1000.0).log2()/2.5).clamp(0.0, 1.0);
            let smooth = position*position*(3.0-2.0*position);
            value *= 10.0_f64.powf(brightness*smooth/20.0);
        }
        // Same explicit scaling as NumPy irfft: coefficients are multiplied
        // by table size before the transform, then the inverse divides by N.
        spectrum[harmonic] = size as f64*value;
        spectrum[size-harmonic] = spectrum[harmonic].conj();
    }
    let fft = planner.plan_fft_inverse(size);
    let needed = fft.get_inplace_scratch_len();
    if scratch.len() < needed {scratch.resize(needed, Complex64::new(0.0, 0.0));}
    fft.process_with_scratch(spectrum, &mut scratch[..needed]);
    table.resize(size, 0.0);
    for (value, bin) in table.iter_mut().zip(spectrum) {*value = bin.re/size as f64;}
    ensure!(table.iter().all(|v| v.is_finite()), "nonfinite periodic table");
    Ok(table)
}

#[inline]
fn periodic_interpolate(table: &[f64], phase: f64) -> f64 {
    let position = phase.rem_euclid(1.0)*table.len() as f64;
    let left = position.floor() as i64;
    let fraction = position-left as f64;
    let mut result = 0.0;
    for node in -2..=3 {
        let mut weight = 1.0;
        for other in -2..=3 {
            if node != other { weight *= (fraction-other as f64)/(node-other) as f64; }
        }
        let index = (left+node).rem_euclid(table.len() as i64) as usize;
        result += weight*table[index];
    }
    result
}

pub fn reconstruction_filter(oversample: usize) -> Result<Vec<f64>> {
    ensure!((2..=16).contains(&oversample), "source oversample must be an integer in 2..16");
    crate::signal::firwin(128*oversample+1, &[0.94/oversample as f64], 2.0, true, 8.6)
}

#[allow(clippy::too_many_arguments)]
pub fn bandlimited_excitation(f0: &[f64], voiced: &[bool], sample_rate: u32,
    hop: usize, source: &str, oversample: usize, brightness_db: &[f64],
    harmonic_gains_db: Option<&[[f64; 2]]>) -> Result<(Vec<f64>, Value)> {
    ensure!(!f0.is_empty() && voiced.len() == f0.len(),
        "source requires matching nonempty one-dimensional F0 and voiced arrays");
    ensure!(sample_rate > 0 && hop > 0 && (2..=16).contains(&oversample),
        "source requires positive sample_rate/hop and oversample in 2..16");
    ensure!(matches!(source, "rosenberg" | "impulse"), "source must be rosenberg or impulse");
    ensure!((brightness_db.len() == 1 || brightness_db.len() == f0.len())
        && brightness_db.iter().all(|v| v.is_finite() && (0.0..=12.0).contains(v)),
        "source brightness must be aligned finite values in 0..12 dB");
    let rate = f64::from(sample_rate);
    let cutoff = STOP_FRACTION*rate;
    ensure!(f0.iter().all(|v| v.is_finite()), "F0 must be finite");
    for (&frequency, &active) in f0.iter().zip(voiced) {
        if active {
            ensure!(frequency > 0.0 && frequency < cutoff,
                "voiced F0 must be positive and below source cutoff ({cutoff} Hz)");
            ensure!(frequency >= cutoff/MAX_HARMONICS as f64,
                "voiced F0 requires more than {MAX_HARMONICS} harmonics");
        }
    }
    let partial_gains = if let Some(gains) = harmonic_gains_db {
        ensure!(gains.len() == f0.len() && gains.iter().flatten()
            .all(|v| v.is_finite() && (0.0..=18.0).contains(v)),
            "harmonic gains must be finite [frames, 2] values in 0..18 dB");
        if gains.iter().zip(voiced).any(|(row, &v)| v && (row[0] != 0.0 || row[1] != 0.0)) {
            Some(gains)
        } else { None }
    } else { None };
    let high_hop = hop.checked_mul(oversample).ok_or_else(|| anyhow::anyhow!("high-rate hop overflow"))?;
    let high_count = f0.len().checked_mul(high_hop).ok_or_else(|| anyhow::anyhow!("source duration overflow"))?;
    let mut high = vec![0.0; high_count];
    let mut delta_high = partial_gains.map(|_| vec![0.0; high_count]);
    let local_seconds: Vec<f64> = (0..high_hop).map(|i| i as f64/(rate*oversample as f64)).collect();
    let mut phase = 0.0;
    let mut maximum_table_size = 0;
    let mut planner = FftPlanner::new();
    let mut coefficients = CoefficientCache(VecDeque::new());
    let mut fft_buffer = Vec::new();
    let mut fft_scratch = Vec::new();
    // This cache is bounded independently of run duration and distinct pitches.
    let mut tables: VecDeque<(u64, u64, Vec<f64>)> = VecDeque::new();
    for index in 0..f0.len() {
        if !voiced[index] { continue; }
        let frequency = f0[index];
        let brightness = brightness_db[if brightness_db.len() == 1 {0} else {index}];
        if let Some(position) = tables.iter().position(|entry| entry.0 == frequency.to_bits()
            && entry.1 == brightness.to_bits()) {
            let entry = tables.remove(position).unwrap();
            tables.push_front(entry);
        } else {
            let recycled = if tables.len() == MAX_CACHED_TABLES {
                tables.pop_back().unwrap().2
            } else {Vec::new()};
            let table = periodic_table(frequency, sample_rate, source, brightness,
                &mut planner, &mut coefficients, &mut fft_buffer, &mut fft_scratch, recycled)?;
            tables.push_front((frequency.to_bits(), brightness.to_bits(), table));
        }
        let table = &tables.front().unwrap().2;
        maximum_table_size = maximum_table_size.max(table.len());
        let begin = index*high_hop;
        for local in 0..high_hop {
            high[begin+local] = periodic_interpolate(table, phase+frequency*local_seconds[local]);
        }
        if let (Some(gains), Some(delta)) = (partial_gains, delta_high.as_mut()) {
            let previous = if voiced[index.saturating_sub(1)] { gains[index.saturating_sub(1)] } else { [0.0; 2] };
            let current = gains[index];
            let next = (index+1).min(f0.len()-1);
            let following = if voiced[next] { gains[next] } else { [0.0; 2] };
            let partials = source_coefficients(frequency, 3, rate, source, brightness)?;
            for local in 0..high_hop {
                let fraction = local as f64/high_hop as f64-0.5;
                let theta = 2.0*PI*(phase+frequency*local_seconds[local]);
                let mut sum = Complex64::new(0.0, 0.0);
                for column in 0..2 {
                    let interpolated = current[column] + if fraction < 0.0 {
                        fraction*(current[column]-previous[column])
                    } else { fraction*(following[column]-current[column]) };
                    sum += partials[column+1] * (10.0_f64.powf(interpolated/20.0)-1.0)
                        * Complex64::from_polar(1.0, theta*(column+2) as f64);
                }
                delta[begin+local] = 2.0*sum.re;
            }
        }
        phase = (phase+frequency*hop as f64/rate).rem_euclid(1.0);
    }
    let taps = reconstruction_filter(oversample)?;
    let mut output = crate::signal::resample_poly_with_taps(&high, 1, oversample, &taps)?;
    let mut original_rms = None;
    if let Some(delta) = delta_high {
        original_rms = Some(super::numpy_math::voiced_rms(&output, voiced, hop));
        let changed = crate::signal::resample_poly_with_taps(&delta, 1, oversample, &taps)?;
        ensure!(changed.len() == output.len(), "harmonic delta duration mismatch");
        for (value, delta) in output.iter_mut().zip(changed) { *value += delta; }
    }
    ensure!(f0.len().checked_mul(hop) == Some(output.len()) && output.iter().all(|v| v.is_finite()),
        "invalid band-limited excitation output");
    let brightness_min = brightness_db.iter().copied().fold(f64::INFINITY, f64::min);
    let brightness_max = brightness_db.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut metadata = json!({
        "algorithm":"analytic_fourier_table_6point_fir_v1", "original_pulse_table_used":false,
        "harmonic_model":if source == "rosenberg" {"analytic Rosenberg flow-derivative Fourier coefficients"}
            else {"DC-free flat harmonic comb with coefficient F0/Fs"},
        "dc_coefficient":0.0, "harmonic_pass_hz":PASS_FRACTION*rate, "harmonic_stop_hz":cutoff,
        "harmonic_rolloff":"raised cosine between pass and stop frequencies", "oversample":oversample,
        "phase_policy":"frame-held F0; bounded per-frame integration; initial phase zero at sample 0; freeze in UV; no reset at voiced frame boundaries",
        "table_interpolation":"periodic six-point Lagrange, at least 64 table samples per highest harmonic",
        "maximum_table_samples_used":maximum_table_size,"maximum_cached_tables":MAX_CACHED_TABLES,
        "maximum_harmonics":MAX_HARMONICS,"fir_taps":taps.len(),"fir_cutoff_hz":0.47*rate,
        "fir_kaiser_beta":8.6,"fir_group_delay_output_samples":(taps.len()-1) as f64/(2*oversample) as f64,
        "fir_delay_compensated":true,"fir_edge_support_output_samples":(taps.len()-1)/(2*oversample),
        "uv_boundary_policy":"zero periodic high-rate input in UV; centered FIR may ring into neighboring frames; caller applies explicit UV noise switch after decimation",
        "bandlimit_scope":"finite stationary harmonic series with small interpolation residual; high-rate FIR suppresses modulation aliases; abrupt F0/UV transitions are not stationary periodic signals",
        "source_brightness_db_range":[brightness_min,brightness_max],
        "source_brightness_policy":"Explicit harmonic-amplitude tilt: smooth 0 at 1 kHz to selected gain at 5.657 kHz; phase and bandlimit retained. Not recovered original source."
    });
    if let Some(gains) = partial_gains {
        let ranges: Vec<[f64; 2]> = (0..2).map(|column| {
            let values = gains.iter().zip(voiced).map(|(row, &active)| if active {row[column]} else {0.0});
            [values.clone().fold(f64::INFINITY, f64::min), values.fold(f64::NEG_INFINITY, f64::max)]
        }).collect();
        metadata["harmonic_balance"] = json!("existing H2/H3 complex coefficients amplified before filtering; same phase clock; frame-center linear dB interpolation");
        metadata["harmonic_gain_db_range"] = json!(ranges);
        metadata["periodic_rms_before_harmonic_balance"] = json!(original_rms);
    }
    Ok((output, metadata))
}
