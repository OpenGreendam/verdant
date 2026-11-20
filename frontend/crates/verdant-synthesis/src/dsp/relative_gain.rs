//! Optional bounded median-centered model gain; exact no-op paths preserve
//! every sample bit (including signed zero), and never normalize a phone.
use anyhow::{ensure, Result};
use serde_json::{json, Value};

// Scale before materializing an overflowing difference/product. NumPy uses
// longdouble for this bounded control; exponent decomposition gives the same
// finite saturation behavior for all finite f64 scalars, including 1e308 and
// subnormal strengths, without an intermediate infinity×zero operation.
fn centered_db(value: f64, center: f64, strength: f64) -> f64 {
    let difference = value-center;
    if difference == 0.0 || strength == 0.0 {return 0.0;}
    let (mantissa, exponent) = if difference.is_finite() {
        libm::frexp(difference)
    } else {
        let (fraction, exponent) = libm::frexp(value*0.5-center*0.5);
        (fraction, exponent+1)
    };
    let (scale, scale_exponent) = libm::frexp(strength);
    let factor = 20.0/std::f64::consts::LN_10;
    libm::scalbn(mantissa*scale*factor, exponent+scale_exponent)
}

pub fn apply_relative_model_gain(waveform: &mut [f64], log_gain: &[f64], hop: usize,
    mode: &str, strength: f64, max_db: f64) -> Result<Value> {
    ensure!(matches!(mode, "off" | "relative"), "model gain mode must be off or relative");
    ensure!(hop > 0 && log_gain.len().checked_mul(hop) == Some(waveform.len()),
        "waveform length must equal len(log_gain) * hop for one contiguous run");
    ensure!(strength.is_finite() && strength >= 0.0 && max_db.is_finite() && max_db >= 0.0,
        "strength and max_db must be finite and nonnegative");
    ensure!(waveform.iter().chain(log_gain).all(|v| v.is_finite()),
        "waveform and log_gain must be finite one-dimensional real arrays");
    let mut ordered = log_gain.to_vec();
    ordered.sort_by(f64::total_cmp);
    let center = if ordered.is_empty() {None} else {
        let middle = ordered.len()/2;
        Some(if ordered.len()%2 == 1 {ordered[middle]}
            else {0.5*ordered[middle-1]+0.5*ordered[middle]})
    };
    let range = ordered.first().map(|&min| [min,*ordered.last().unwrap()]);
    let mut metadata = json!({
        "mode":mode,"applied":false,"strength":strength,"max_db":max_db,
        "frames":log_gain.len(),"hop_samples":hop,"samples":waveform.len(),
        "original_log_gain_range":range,"center_log_gain":center,
        "centering":"median over this complete contiguous run",
        "frame_control_db_range":[0.0,0.0],"sample_control_db_range":[0.0,0.0],
        "sample_multiplier_range":[1.0,1.0],"fraction_frames_limited_low":0.0,
        "fraction_frames_limited_high":0.0,
        "interpolation":"linear dB interpolation from native frame centers to sample centers, constant beyond end centers",
        "normalization":"none here; caller may calibrate the whole run once",
        "status":"opt-in relative-envelope ablation; original absolute excitation/filter calibration remains unrecovered"
    });
    if mode == "off" || log_gain.is_empty() || strength == 0.0 || max_db == 0.0 {
        metadata["reason"] = json!("disabled, empty run, zero strength, or zero limit: exact no-op");
        return Ok(metadata);
    }
    let center = center.unwrap();
    let mut low = 0;
    let mut high = 0;
    let frame_db:Vec<f64> = log_gain.iter().map(|&v| {
        let db = centered_db(v, center, strength);
        low += usize::from(db < -max_db);
        high += usize::from(db > max_db);
        db.clamp(-max_db,max_db)
    }).collect();
    metadata["fraction_frames_limited_low"] = json!(low as f64/log_gain.len() as f64);
    metadata["fraction_frames_limited_high"] = json!(high as f64/log_gain.len() as f64);
    metadata["frame_control_db_range"] = json!([
        frame_db.iter().copied().fold(f64::INFINITY,f64::min),
        frame_db.iter().copied().fold(f64::NEG_INFINITY,f64::max)]);
    if frame_db.iter().all(|v| *v == 0.0) {
        metadata["reason"] = json!("constant centered gain: exact no-op");
        return Ok(metadata);
    }
    let mut sample_db_range = [f64::INFINITY,f64::NEG_INFINITY];
    let mut multiplier_range = [f64::INFINITY,f64::NEG_INFINITY];
    // Admission is completed before committing samples so an overflowing
    // optional gain cannot return a partially modified waveform.
    let multiplier_at = |index:usize| {
        let coordinate = ((index as f64+0.5)/hop as f64-0.5)
            .clamp(0.0,(log_gain.len()-1) as f64);
        let left = coordinate.floor() as usize;
        let right = (left+1).min(log_gain.len()-1);
        let fraction = coordinate-left as f64;
        let db = if left == right {frame_db[left]}
            else {frame_db[left]+fraction*(frame_db[right]-frame_db[left])};
        (db,(db*(std::f64::consts::LN_10/20.0)).exp())
    };
    for (index,&sample) in waveform.iter().enumerate() {
        let (db,multiplier) = multiplier_at(index);
        ensure!(multiplier.is_finite() && (sample*multiplier).is_finite(),
            "relative gain would produce non-finite waveform values; reduce max_db or input amplitude");
        sample_db_range[0] = sample_db_range[0].min(db);
        sample_db_range[1] = sample_db_range[1].max(db);
        multiplier_range[0] = multiplier_range[0].min(multiplier);
        multiplier_range[1] = multiplier_range[1].max(multiplier);
    }
    for (index,sample) in waveform.iter_mut().enumerate() {*sample *= multiplier_at(index).1;}
    metadata["applied"] = json!(true);
    metadata["reason"] = json!("bounded median-centered model envelope applied within one contiguous run");
    metadata["sample_control_db_range"] = json!(sample_db_range);
    metadata["sample_multiplier_range"] = json!(multiplier_range);
    Ok(metadata)
}
