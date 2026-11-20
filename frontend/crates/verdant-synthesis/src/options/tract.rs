use anyhow::{ensure, Context, Result};
use rustfft::{num_complex::Complex, FftPlanner};
use serde_json::{json, Value};
use std::f64::consts::{LN_10, PI};
use crate::core::NumericArray;
use crate::dsp::lpc::{lpc_to_lsf, lsf_to_lpc};
use super::common::{bounded_number, exact_object, floating_dtype, interp, median, range, validate_lsf, validate_numeric, Pchip};

const BILINEAR: &str = "bilinear-v1";
const ANCHORED: &str = "anchored-warp-v1";

#[derive(Clone, Debug)]
pub struct VocalTractConfiguration {
    pub method: String,
    pub ratio: f64,
    pub anchor_ratios: Option<[f64; 2]>,
}
impl VocalTractConfiguration {
    fn neutral_anchors(&self) -> bool { self.anchor_ratios.is_none_or(|a| a == [1., 1.]) }
    fn neutral(&self) -> bool { self.ratio == 1. && self.neutral_anchors() }
    fn json(&self) -> Value {
        let mut value = json!({"method": self.method, "ratio": self.ratio});
        if let Some(anchors) = self.anchor_ratios { value["anchor_ratios"] = json!(anchors); }
        value
    }
}

pub fn validate_vocal_tract(value: &Value) -> Result<Option<VocalTractConfiguration>> {
    if value.is_null() { return Ok(None); }
    let method = value["method"].as_str().context("vocal_tract requires a supported method")?;
    let fields: &[&str] = match method {
        BILINEAR => &["method", "ratio"],
        ANCHORED => &["method", "ratio", "anchor_ratios"],
        _ => anyhow::bail!("unsupported vocal_tract method"),
    };
    exact_object(value, fields, "vocal_tract")?;
    let ratio = bounded_number(&value["ratio"], 0.8, 1.25, "vocal_tract ratio")?;
    let anchors = if method == ANCHORED {
        let values = value["anchor_ratios"].as_array().context("vocal_tract anchor_ratios requires two finite ratios")?;
        ensure!(values.len() == 2, "vocal_tract anchor_ratios requires two finite ratios");
        Some([bounded_number(&values[0], 0.8, 1.25, "vocal_tract anchor ratio")?,
            bounded_number(&values[1], 0.8, 1.25, "vocal_tract anchor ratio")?])
    } else { None };
    Ok(Some(VocalTractConfiguration { method: method.into(), ratio, anchor_ratios: anchors }))
}

pub fn vocal_tract_capabilities() -> Value {
    json!({"methods": [BILINEAR, ANCHORED], "ratio_range": [0.8, 1.25], "default_ratio": 1.,
        "default_enabled": false, "anchor_ratio_range": [0.8, 1.25], "anchor_fractions": [0.1, 0.3],
        "default_anchor_ratios": [1., 1.], "anchor_method": ANCHORED,
        "scope": "explicit spectral-envelope frequency transform; score F0 and timing unchanged",
        "ratio_meaning": "bilinear base-map low-frequency slope, not a uniform formant multiplier or a speaker identity",
        "anchor_meaning": "residual output/input knot-frequency ratios after the bilinear base map; shared by every frame"})
}
fn rate(sample_rate: f64) -> Result<()> {
    ensure!(sample_rate.is_finite() && sample_rate > 0., "sample_rate must be finite and positive");
    Ok(())
}
fn angular(omega: f64, ratio: f64) -> f64 {
    if omega == 0. { 0. } else if omega == PI { PI }
    else { 2.*(ratio*(omega/2.).sin()).atan2((omega/2.).cos()) }
}
fn interpolator(config: &VocalTractConfiguration) -> Result<Option<Pchip>> {
    if config.neutral_anchors() { return Ok(None); }
    let anchors = config.anchor_ratios.context("active anchors missing")?;
    Ok(Some(Pchip::new(&[0., 0.1, 0.3, 1.], &[0., 0.1*anchors[0], 0.3*anchors[1], 1.])?))
}
fn forward(omega: f64, config: &VocalTractConfiguration, map: Option<&Pchip>) -> f64 {
    let base = angular(omega, config.ratio);
    if omega == 0. || omega == PI { return omega; }
    match map { None => base, Some(map) => PI*map.evaluate(base/PI) }
}
fn inverse(omega: f64, config: &VocalTractConfiguration, map: Option<&Pchip>) -> f64 {
    let Some(map) = map else { return angular(omega, 1./config.ratio); };
    let target = omega/PI;
    if target == 0. || target == 1. { return omega; }
    let (mut lower, mut upper) = (0., 1.);
    for _ in 0..48 {
        let middle = (lower+upper)/2.;
        if map.evaluate(middle) < target { lower = middle; } else { upper = middle; }
    }
    angular(PI*((lower+upper)/2.), 1./config.ratio)
}

pub fn warp_frequencies(frequencies: &[f64], sample_rate: f64, configuration: &Value) -> Result<Vec<f64>> {
    rate(sample_rate)?;
    let config = validate_vocal_tract(configuration)?.context("frequency warp requires vocal_tract configuration")?;
    ensure!(frequencies.iter().all(|&value| value.is_finite() && value >= 0. && value <= sample_rate/2.),
        "spectral frequencies must be finite and in 0..Nyquist");
    if config.neutral() { return Ok(frequencies.to_vec()); }
    let map = interpolator(&config)?;
    Ok(frequencies.iter().map(|&frequency| {
        if frequency == sample_rate/2. { sample_rate/2. }
        else { forward(frequency*(2.*PI/sample_rate), &config, map.as_ref())*(sample_rate/(2.*PI)) }
    }).collect())
}

pub fn warp_log_envelope(source: &NumericArray, frequencies: &[f64], sample_rate: f64,
    configuration: &Value) -> Result<NumericArray> {
    rate(sample_rate)?;
    let config = validate_vocal_tract(configuration)?.context("envelope warp requires vocal_tract configuration")?;
    validate_numeric(source)?;
    ensure!(frequencies.len() >= 2 && frequencies[0] == 0. && frequencies[frequencies.len()-1] == sample_rate/2. &&
        frequencies.iter().all(|v| v.is_finite()) && frequencies.windows(2).all(|pair| pair[1] > pair[0]) &&
        !source.values.is_empty() && !source.shape.is_empty() && source.shape.last() == Some(&frequencies.len()) && floating_dtype(&source.dtype),
        "log envelope requires finite floating frames on a complete increasing frequency grid");
    if config.neutral() { return Ok(source.clone()); }
    let map = interpolator(&config)?;
    let query: Vec<f64> = frequencies.iter().map(|&frequency|
        inverse(frequency*(2.*PI/sample_rate), &config, map.as_ref())*(sample_rate/(2.*PI))).collect();
    let mut values = Vec::with_capacity(source.values.len());
    for frame in source.values.chunks_exact(frequencies.len()) {
        values.extend(query.iter().map(|&point| interp(point, frequencies, frame)));
    }
    NumericArray::new(source.shape.clone(), "float64", values)
}

struct EnvelopeFit {
    lpc: NumericArray,
    lsf: Vec<f64>,
    gains: Vec<f64>,
    report: Value,
}
fn ar_fit(correlation: &[f64], output: &mut [f64], scratch: &mut [f64]) -> Result<()> {
    output.fill(0.);
    output[0] = 1.;
    let mut error = correlation[0];
    for order in 1..output.len() {
        let sum = correlation[order]+(1..order).map(|i| output[i]*correlation[order-i]).sum::<f64>();
        let reflection = -sum/error;
        ensure!(reflection.is_finite() && reflection.abs() < 1., "warped AR fit is not Schur stable");
        scratch[..order].copy_from_slice(&output[..order]);
        for i in 1..order { output[i] = scratch[i]+reflection*scratch[order-i]; }
        output[order] = reflection;
        error *= 1.-reflection*reflection;
        ensure!(error.is_finite() && error > 0., "AR fit has nonpositive prediction-error variance");
    }
    Ok(())
}

fn fit(source: &NumericArray, config: &VocalTractConfiguration, nfft: usize, validate_input: bool) -> Result<EnvelopeFit> {
    validate_numeric(source)?;
    ensure!(source.shape.len() == 2 && source.shape[0] > 0 && source.shape[1] >= 3 &&
        (source.shape[1]-1)%2 == 0 && floating_dtype(&source.dtype), "finite monic even-order LPC frames required");
    let width = source.shape[1];
    let order = width-1;
    ensure!(nfft.is_power_of_two() && nfft >= 512.max(4*width), "nfft must be a sufficiently large power of two");
    let frames = source.shape[0];
    if validate_input {
        for row in source.values.chunks_exact(width) {
            ensure!(row[0] == 1., "finite monic even-order LPC frames required");
            lpc_to_lsf(row)?;
        }
    }
    if config.neutral() {
        return Ok(EnvelopeFit { lpc: source.clone(), lsf: Vec::new(), gains: vec![1.; frames],
            report: json!({"nfft": nfft, "maximum_frame_rms_envelope_error_db": 0.,
                "maximum_absolute_envelope_error_db": 0., "gain_included_in_error": true}) });
    }
    let bins = nfft/2+1;
    let omega: Vec<f64> = (0..bins).map(|i| if i+1 == bins { PI } else { i as f64*(PI/(bins-1) as f64) }).collect();
    let map = interpolator(config)?;
    let query: Vec<f64> = omega.iter().map(|&angle| inverse(angle, config, map.as_ref())).collect();
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(nfft);
    let ifft = planner.plan_fft_inverse(nfft);
    let scratch_len = fft.get_inplace_scratch_len().max(ifft.get_inplace_scratch_len());
    let mut fft_scratch = vec![Complex::new(0., 0.); scratch_len];
    let mut spectrum = vec![Complex::new(0., 0.); nfft];
    let mut source_log = vec![0.; bins];
    let mut target_log = vec![0.; bins];
    let mut correlation = vec![0.; width];
    let mut coefficients = vec![0.; width];
    let mut ar_scratch = vec![0.; width];
    let mut values = Vec::with_capacity(source.values.len());
    let mut lsf = Vec::with_capacity(frames*order);
    let mut gains = Vec::with_capacity(frames);
    let mut rms_errors = Vec::with_capacity(frames);
    let mut maximum_error = 0f64;
    for row in source.values.chunks_exact(width) {
        spectrum.fill(Complex::new(0., 0.));
        for (destination, &value) in spectrum.iter_mut().zip(row) { destination.re = value; }
        fft.process_with_scratch(&mut spectrum, &mut fft_scratch);
        for (value, bin) in source_log.iter_mut().zip(&spectrum) {
            let denominator = bin.norm_sqr();
            ensure!(denominator.is_finite() && denominator > 0., "nonpositive LPC envelope denominator");
            *value = -denominator.ln();
        }
        for i in 0..bins {
            target_log[i] = interp(query[i], &omega, &source_log);
            let power = target_log[i].exp();
            ensure!(power.is_finite() && power > 0., "nonfinite warped LPC power");
            spectrum[i] = Complex::new(power, 0.);
        }
        for i in bins..nfft { spectrum[i] = spectrum[nfft-i]; }
        ifft.process_with_scratch(&mut spectrum, &mut fft_scratch);
        let scale = spectrum[0].re/nfft as f64;
        ensure!(scale.is_finite() && scale > 0., "nonpositive warped autocorrelation scale");
        for i in 0..width { correlation[i] = (spectrum[i].re/nfft as f64)/scale; }
        ar_fit(&correlation, &mut coefficients, &mut ar_scratch)?;
        let variance = (1.+coefficients[1..].iter().zip(&correlation[1..]).map(|(&a, &b)| a*b).sum::<f64>())*scale;
        ensure!(variance.is_finite() && variance > 0., "AR fit has nonpositive prediction-error variance");
        lsf.extend(lpc_to_lsf(&coefficients)?);
        gains.push(variance.sqrt());
        values.extend_from_slice(&coefficients);
        spectrum.fill(Complex::new(0., 0.));
        for (destination, &value) in spectrum.iter_mut().zip(&coefficients) { destination.re = value; }
        fft.process_with_scratch(&mut spectrum, &mut fft_scratch);
        let mut sum_squared = 0.;
        let log_variance = variance.ln();
        for i in 0..bins {
            let fitted = log_variance-2.*spectrum[i].norm().ln();
            let delta = (10./LN_10)*(fitted-target_log[i]);
            ensure!(delta.is_finite(), "nonfinite fitted envelope diagnostics");
            sum_squared += delta*delta;
            maximum_error = maximum_error.max(delta.abs());
        }
        rms_errors.push((sum_squared/bins as f64).sqrt());
    }
    let report = json!({"nfft": nfft, "maximum_frame_rms_envelope_error_db": range(&rms_errors)[1],
        "median_frame_rms_envelope_error_db": median(&rms_errors),
        "maximum_absolute_envelope_error_db": maximum_error, "gain_included_in_error": true});
    Ok(EnvelopeFit { lpc: NumericArray::new(source.shape.clone(), "float64", values)?, lsf, gains, report })
}

pub fn warp_lpc_envelope(source: &NumericArray, configuration: &Value, nfft: usize)
    -> Result<(NumericArray, Vec<f64>, Value)> {
    let config = validate_vocal_tract(configuration)?.context("LPC warp requires vocal_tract configuration")?;
    let result = fit(source, &config, nfft, true)?;
    Ok((result.lpc, result.gains, result.report))
}

pub fn transform_lsf(source: &NumericArray, configuration: &Value, sample_rate: f64,
    bandwidth_gamma: f64) -> Result<(NumericArray, Option<Value>)> {
    let Some(config) = validate_vocal_tract(configuration)? else { return Ok((source.clone(), None)); };
    rate(sample_rate)?;
    let (frames, order) = validate_lsf(source)?;
    ensure!(bandwidth_gamma.is_finite() && bandwidth_gamma > 0. && bandwidth_gamma <= 1.,
        "bandwidth_gamma must be finite in (0,1]");
    let active = !config.neutral();
    let (output, gains, diagnostic) = if active {
        let mut coefficients = Vec::with_capacity(frames*(order+1));
        let gamma: Vec<f64> = (0..=order).map(|i| bandwidth_gamma.powi(i as i32)).collect();
        for row in source.values.chunks_exact(order) {
            let row = lsf_to_lpc(row)?;
            coefficients.extend(row.iter().zip(&gamma).map(|(&value, &scale)| value*scale));
        }
        let coefficients = NumericArray::new(vec![frames, order+1], "float64", coefficients)?;
        let result = fit(&coefficients, &config, 8192, false)?;
        (NumericArray::new(source.shape.clone(), "float64", result.lsf)?, result.gains, Some(result.report))
    } else { (source.clone(), vec![1.; frames], None) };
    let mut shifts: Vec<f64> = output.values.iter().zip(&source.values).map(|(&left, &right)|
        (left-right)*(sample_rate/(2.*PI))).collect();
    let signed_median = median(&shifts);
    for value in &mut shifts { *value = value.abs(); }
    let mut report = config.json();
    let object = report.as_object_mut().unwrap();
    let attributes = json!({"schema": "verdant-vocal-tract-transform/1", "applied": active,
        "representation": "sampled LPC power envelope; same-order stable AR refit; ordered LSF output",
        "mapping": "omega_out=2*atan2(ratio*sin(omega_in/2),cos(omega_in/2))",
        "ratio_meaning": "derivative at DC; not exact F1/F2/F3 multipliers or an anatomical length estimate",
        "nyquist_fixed": true, "sample_rate": sample_rate, "frames": frames, "order": order,
        "source_bandwidth_gamma": bandwidth_gamma,
        "output_bandwidth_gamma": if active { 1. } else { bandwidth_gamma },
        "bandwidth_expansion_baked_in": active, "fit": diagnostic,
        "prediction_error_amplitude_gain_range": range(&gains),
        "prediction_error_gain_applied_by_this_adapter": false,
        "maximum_lsf_shift_hz": range(&shifts)[1], "median_lsf_shift_hz": signed_median,
        "score_pitch_modified": false, "timing_modified": false, "trained_voice_model_modified": false,
        "historical_voice_identity": null, "exact_reconstructed_formant_map": false,
        "additional_waveform_normalization": false,
        "status": "explicit voice-color transform; listening acceptance is recorded by the voice-bank preset, not inferred by this neutral core"});
    object.extend(attributes.as_object().unwrap().clone());
    if let Some(anchors) = config.anchor_ratios {
        let attributes = json!({"mapping": "omega_out=pi*T(B_ratio(omega_in)/pi); T=PCHIP([0,.1,.3,1],[0,.1*a_low,.3*a_mid,1])",
            "ratio_meaning": "bilinear base-map derivative at DC before the residual map; not exact formant multipliers or an anatomical length estimate",
            "anchor_fractions": [0.1, 0.3], "anchor_domain": "frequency after the bilinear base map, normalized by Nyquist",
            "residual_anchor_input_hz": [0.1*sample_rate/2., 0.3*sample_rate/2.],
            "residual_anchor_output_hz": [0.1*anchors[0]*sample_rate/2., 0.3*anchors[1]*sample_rate/2.],
            "residual_interpolation": "monotone PCHIP; inverse by 48-step bounded bisection",
            "all_frames_share_map": true, "neutral_anchors_use_exact_bilinear_path": true});
        object.extend(attributes.as_object().unwrap().clone());
    }
    Ok((output, Some(report)))
}
