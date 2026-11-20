//! Bounded H2/H3 correction from the complete existing harmonic spectrum.
//! Unknown IDs, compound rimes, UV, moving controls and segment edges abstain.
use anyhow::{ensure, Result};
use num_complex::Complex64;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::f64::consts::PI;
use super::excitation::{source_coefficients, STOP_FRACTION};
use super::phonetic_source::native_phone_key;

fn rise(value: f64, lower: f64, upper: f64) -> f64 {
    let x = ((value-lower)/(upper-lower)).clamp(0.0, 1.0);
    0.5-0.5*(PI*x).cos()
}

fn monophthong(key: Option<&str>) -> bool {
    matches!(key, Some("zh:a" | "zh:e" | "zh:i" | "zh:o" | "zh:u" | "zh:v"
        | "en:aa" | "en:ae" | "en:ah" | "en:ao" | "en:eh" | "en:er" | "en:ih"
        | "en:iy" | "en:uh" | "en:uw" | "en:ax" | "en:axr" | "en:ix" | "en:ux"))
}

fn partial_magnitudes(lpc: &[f64], frequency: f64, count: usize, rate: f64,
    source: &str, brightness: f64) -> Result<Vec<f64>> {
    let coefficients = source_coefficients(frequency, count, rate, source, brightness)?;
    let mut result = Vec::with_capacity(count);
    for (index, coefficient) in coefficients.into_iter().enumerate() {
        let omega = 2.0*PI*(index+1) as f64*frequency/rate;
        let mut denominator = Complex64::new(0.0, 0.0);
        for (j, &value) in lpc.iter().enumerate() {
            denominator += Complex64::from_polar(1.0, -omega*j as f64)*value;
        }
        let magnitude = (coefficient/denominator).norm();
        ensure!(magnitude.is_finite(), "nonfinite harmonic-balance prediction");
        result.push(magnitude);
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub fn vowel_harmonic_gains(lpc: &[f64], lsf: &[f64], order: usize,
    f0: &[f64], model_f0: &[f64], voiced: &[bool], ids: &[i64], segments: &[i64],
    sample_rate: u32, hop: usize, source: &str, brightness_db: &[f64],
    maximum_gain_db: f64) -> Result<(Vec<[f64; 2]>, Value)> {
    let frames = f0.len();
    ensure!(frames > 0 && order >= 2 && lsf.len() == frames*order
        && lpc.len() == frames*(order+1) && model_f0.len() == frames
        && voiced.len() == frames && ids.len() == frames && segments.len() == frames,
        "aligned LSF/LPC, pitch, voicing and integer identities required");
    ensure!(sample_rate > 0 && hop > 0, "positive integer sample rate and hop required");
    ensure!(lpc.iter().chain(lsf).chain(f0).chain(model_f0).all(|v| v.is_finite())
        && model_f0.iter().all(|v| *v >= 0.0)
        && f0.iter().zip(voiced).all(|(&f,&v)| !v || f > 0.0),
        "finite controls, positive voiced pitch and nonnegative reference pitch required");
    ensure!(maximum_gain_db.is_finite() && (0.0..=18.0).contains(&maximum_gain_db),
        "maximum_gain_db must be in 0..18");
    ensure!(matches!(source, "rosenberg" | "impulse"), "known periodic source required");
    ensure!((brightness_db.len() == 1 || brightness_db.len() == frames)
        && brightness_db.iter().all(|v| v.is_finite() && (0.0..=12.0).contains(v)),
        "source brightness must be finite 0..12 dB");
    let rate = f64::from(sample_rate);
    let dt = hop as f64/rate;
    let mut gate = vec![0.0; frames];
    let mut speed = vec![0.0; frames.saturating_sub(1)];
    let mut cents = vec![0.0; frames.saturating_sub(1)];
    for i in 0..frames-1 {
        let mut squared_motion = (0..order).map(|j| {
            let delta = lsf[(i+1)*order+j]-lsf[i*order+j];
            delta*delta
        });
        let power = super::numpy_math::pairwise_sum_iter(&mut squared_motion,order);
        speed[i] = (power/order as f64).sqrt()*rate/(2.0*PI);
        if voiced[i] && voiced[i+1] { cents[i] = (1200.0*(f0[i+1]/f0[i]).log2()).abs(); }
    }
    let mut edges = vec![0];
    for i in 1..frames { if segments[i] != segments[i-1] { edges.push(i); } }
    edges.push(frames);
    for pair in edges.windows(2) {
        let first = pair[0];
        let last = pair[1];
        ensure!(ids[first..last].iter().all(|&id| id == ids[first]),
            "one segment must retain one phone identity");
        for i in first..last {
            let reference_valid = model_f0[i] >= 35.0 && model_f0[i] < 0.45*rate;
            let usable = voiced[i] && monophthong(native_phone_key(ids[i]))
                && reference_valid && f0[i] < 0.45*rate/3.0;
            let ratio = if reference_valid { f0[i]/model_f0[i] } else { 0.0 };
            let spectral_speed = (if i > 0 {speed[i-1]} else {0.0})
                .max(if i+1 < frames {speed[i]} else {0.0});
            let contour_speed = (if i > 0 {cents[i-1]} else {0.0})
                .max(if i+1 < frames {cents[i]} else {0.0});
            let inside = ((i-first) as f64+0.5).min((last-i) as f64-0.5)*dt;
            gate[i] = (if usable {1.0} else {0.0}) * rise(ratio, 1.6, 2.0)
                * (1.0-rise(spectral_speed, 2.0, 12.0))
                * (1.0-rise(contour_speed, 5.0, 30.0)) * rise(inside, 0.030, 0.060);
        }
    }
    let mut gains = vec![[0.0;2]; frames];
    let mut bounds = vec![[0.0;2]; frames];
    let mut share_range: Option<[f64;2]> = None;
    for i in 0..frames {
        if gate[i] <= 0.0 {continue;}
        let count = 3.max((STOP_FRACTION*rate/f0[i]).floor() as usize);
        let brightness = brightness_db[if brightness_db.len() == 1 {0} else {i}];
        let row = &lpc[i*(order+1)..(i+1)*(order+1)];
        let actual = partial_magnitudes(row, f0[i], count, rate, source, brightness)?;
        let reference = partial_magnitudes(row, model_f0[i], 3, rate, source, brightness)?;
        let total: f64 = actual.iter().map(|v| v*v).sum();
        if total <= f64::MIN_POSITIVE || actual[0] <= 1e-20 || reference[0] <= 1e-20 {
            gate[i] = 0.0;
            continue;
        }
        let share = actual[0].powi(2)/total;
        share_range = Some(share_range.map_or([share,share], |[min,max]| [min.min(share),max.max(share)]));
        gate[i] *= rise(share, 0.97, 0.995);
        for (column, upper) in [-12.0,-18.0].into_iter().enumerate() {
            let actual_db = 20.0*(actual[column+1]/actual[0]).max(1e-30).log10();
            let reference_db = 20.0*(reference[column+1]/reference[0]).max(1e-30).log10();
            let requested = (0.5*(reference_db-actual_db)).max(0.0);
            let cap = maximum_gain_db.min(upper-actual_db).max(0.0);
            bounds[i][column] = requested.min(cap)*gate[i];
            gains[i][column] = bounds[i][column];
        }
    }
    // SciPy gaussian_filter1d sigma=.010/dt, order=0, truncate=4, constant0.
    // Smooth each segment independently and cap by its own original allowance.
    let sigma = 0.010/dt;
    let radius = (4.0*sigma+0.5).floor() as usize;
    let kernel_length = radius.checked_mul(2).and_then(|n|n.checked_add(1))
        .ok_or_else(||anyhow::anyhow!("harmonic smoothing kernel size overflow"))?;
    let mut weights: Vec<f64> = (0..kernel_length).map(|i| {
        let x = i as f64-radius as f64;
        (-0.5/(sigma*sigma)*(x*x)).exp()
    }).collect();
    let sum = super::numpy_math::pairwise_sum(&weights);
    for weight in &mut weights {*weight /= sum;}
    for pair in edges.windows(2) {
        let first = pair[0];
        let last = pair[1];
        for i in first..last {
            for column in 0..2 {
                let mut value = 0.0;
                for (j,&weight) in weights.iter().enumerate() {
                    let query = i as i64+j as i64-radius as i64;
                    if query >= first as i64 && query < last as i64 {
                        value += weight*bounds[query as usize][column];
                    }
                }
                gains[i][column] = value.min(bounds[i][column]);
            }
        }
    }
    ensure!(gains.iter().flatten().all(|v| v.is_finite() && *v >= 0.0 && *v <= maximum_gain_db+1e-10),
        "invalid bounded harmonic gains");
    let mut keys = BTreeSet::new();
    let mut active_frames = 0;
    for (i,row) in gains.iter().enumerate() {
        if row.iter().any(|v| *v > 1e-8) {
            active_frames += 1;
            if let Some(key) = native_phone_key(ids[i]) {keys.insert(key);}
        }
    }
    let ranges:Vec<[f64;2]> = (0..2).map(|column| [
        gains.iter().map(|row|row[column]).fold(f64::INFINITY,f64::min),
        gains.iter().map(|row|row[column]).fold(f64::NEG_INFINITY,f64::max)]).collect();
    Ok((gains,json!({
        "algorithm":"bounded-existing-partials-v1",
        "status":"explicit engineering singing candidate; not recovered original source",
        "maximum_gain_db":maximum_gain_db,"adjusted_harmonics":[2,3],
        "reference_policy":"half positive model/current relative-partial dB difference, used only as an upper reference",
        "relative_partial_upper_db":[-12.0,-18.0],"authored_to_model_f0_gate":[1.6,2.0],
        "predicted_h1_power_share_gate":[0.97,0.995],"minimum_model_reference_hz":35.0,
        "h1_prediction_harmonic_band_hz":[0.0,STOP_FRACTION*rate],
        "protected_segment_edge_seconds":0.030,"full_boundary_admission_seconds":0.060,
        "lsf_rms_motion_gate_hz_per_frame":[2.0,12.0],"f0_motion_gate_cents_per_frame":[5.0,30.0],
        "gain_smoothing_sigma_seconds":0.010,"frame_seconds":dt,
        "active_frames":active_frames,"total_frames":frames,
        "active_native_keys":keys,"gain_db_range_by_harmonic":ranges,
        "evaluated_prediction_h1_share_range":share_range,
        "noise_gain_changed":false,"f0_changed":false,"lsf_changed":false,
        "harmonic_phase_changed":false,"original_pulse_table_used":false,"human_quality_score":null
    })))
}
