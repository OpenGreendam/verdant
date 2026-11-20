use anyhow::{ensure, Context, Result};
use serde_json::{Map, Value};
use crate::core::{AcousticControlTrack, NumericArray, RunParameters};

pub(super) fn exact_object<'a>(value: &'a Value, fields: &[&str], name: &str) -> Result<&'a Map<String, Value>> {
    let object = value.as_object().with_context(|| format!("{name} requires a configuration object"))?;
    ensure!(object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field)),
        "{name} requires exactly {}", fields.join(", "));
    Ok(object)
}

pub(super) fn bounded_number(value: &Value, minimum: f64, maximum: f64, name: &str) -> Result<f64> {
    let number = value.as_f64().with_context(|| format!("{name} must be a real number"))?;
    ensure!(number.is_finite() && number >= minimum && number <= maximum,
        "{name} must be finite in {minimum}..{maximum}");
    Ok(number)
}

pub(super) fn object_mut<'a>(value: &'a mut Value, name: &str) -> Result<&'a mut Map<String, Value>> {
    value.as_object_mut().with_context(|| format!("{name} must be an object"))
}

pub(super) fn floating_dtype(dtype: &str) -> bool {
    matches!(dtype, "float64" | "float32" | "<f8" | "<f4" | ">f8" | ">f4" | "f8" | "f4")
}

pub(super) fn round_storage(values: &mut [f64], gains: &[f64], dtype: &str) -> Result<()> {
    ensure!(floating_dtype(dtype), "floating storage dtype required");
    ensure!(values.len() == gains.len(), "storage rounding requires aligned sample gains");
    if matches!(dtype, "float32" | "<f4" | ">f4" | "f4") {
        for (value, &gain) in values.iter_mut().zip(gains) {
            if gain != 1. { *value = (*value as f32) as f64; }
        }
    }
    Ok(())
}

pub(super) fn validate_numeric(array: &NumericArray) -> Result<()> {
    let count = array.shape.iter().try_fold(1usize, |a, &b| a.checked_mul(b)).context("array shape overflow")?;
    ensure!(count == array.values.len() && array.values.iter().all(|value| value.is_finite()),
        "array shape/finite-value contract violated");
    Ok(())
}

pub(super) fn array<'a>(run: &'a RunParameters, key: &str) -> Result<&'a NumericArray> {
    let value = run.arrays.get(key).with_context(|| format!("missing run array {key}"))?;
    validate_numeric(value)?;
    ensure!(floating_dtype(&value.dtype), "{key} requires floating storage");
    Ok(value)
}

pub(super) fn sample_grid(run: &RunParameters) -> Result<(u32, usize)> {
    let rate = run.metadata["sample_rate"].as_u64().context("run metadata requires integer sample_rate")?;
    let hop = run.metadata["hop_samples"].as_u64().context("run metadata requires integer hop_samples")?;
    let rate = u32::try_from(rate).context("sample_rate exceeds supported range")?;
    let hop = usize::try_from(hop).context("hop_samples exceeds address space")?;
    ensure!(rate > 0 && hop > 0, "run sample grid must be positive");
    Ok((rate, hop))
}

pub(super) fn validate_controls(controls: &AcousticControlTrack) -> Result<usize> {
    let count = controls.f0_hz.len();
    ensure!(count > 0 && [controls.voiced.len(), controls.phonetic_classes.len(),
        controls.phone_identity.len(), controls.segment_identity.len(), controls.continuity_identity.len()]
        .iter().all(|&size| size == count), "acoustic controls require aligned nonempty one-dimensional cells");
    for i in 0..count {
        let f0 = controls.f0_hz[i];
        ensure!(f0.is_finite() && if controls.voiced[i] { f0 > 0. } else { f0 == 0. },
            "control F0 must be positive when voiced and zero for UV");
        ensure!(matches!(controls.phonetic_classes[i].as_str(), "vowel" | "nasal" | "sonorant" |
            "voiced_fricative" | "sibilant" | "fricative" | "aspiration" | "stop_unaspirated" |
            "stop_aspirated" | "stop_voiced" | "affricate" | "closure" | "silence" | "unknown"),
            "explicit supported phonetic classes required; use unknown when unavailable");
    }
    let period = controls.period_seconds;
    let origin = controls.origin_seconds;
    ensure!(period.is_finite() && period > 0. && origin.is_finite() &&
        (origin + count as f64 * period).is_finite() && origin + period != origin,
        "control clock must have a representable finite positive period and finite origin");
    Ok(count)
}

pub(super) fn validate_lsf(array: &NumericArray) -> Result<(usize, usize)> {
    validate_numeric(array)?;
    ensure!(floating_dtype(&array.dtype) && array.shape.len() == 2 && array.shape[0] > 0 &&
        array.shape[1] >= 2 && array.shape[1] % 2 == 0,
        "finite nonempty floating even-order LSF frames required");
    let order = array.shape[1];
    for row in array.values.chunks_exact(order) {
        ensure!(row[0] > 0. && row[order-1] < std::f64::consts::PI &&
            row.windows(2).all(|pair| pair[0] < pair[1]),
            "LSFs must be strictly increasing in (0,pi)");
    }
    Ok((array.shape[0], order))
}

pub(super) fn median_bounds(values: &[f64]) -> (f64, f64) {
    let mut ordered = values.to_vec();
    median_bounds_in_place(&mut ordered)
}

pub(super) fn median_bounds_in_place(ordered: &mut [f64]) -> (f64, f64) {
    ordered.sort_unstable_by(f64::total_cmp);
    let middle = ordered.len()/2;
    if ordered.len() % 2 == 1 { (ordered[middle], ordered[middle]) }
    else { (ordered[middle-1], ordered[middle]) }
}

pub(super) fn median(values: &[f64]) -> f64 {
    let (lower, upper) = median_bounds(values);
    lower/2. + upper/2.
}

pub(super) fn median_in_place(values: &mut [f64]) -> f64 {
    let (lower, upper) = median_bounds_in_place(values);
    lower/2. + upper/2.
}

pub(super) fn range(values: &[f64]) -> [f64; 2] {
    values.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], &value| [lo.min(value), hi.max(value)])
}

pub(super) fn interp(query: f64, x: &[f64], y: &[f64]) -> f64 {
    if query <= x[0] { return y[0]; }
    if query >= x[x.len()-1] { return y[y.len()-1]; }
    let right = x.partition_point(|&value| value <= query);
    let left = right-1;
    if query == x[left] { return y[left]; }
    let slope = (y[right]-y[left])/(x[right]-x[left]);
    // Same left-endpoint form used by numpy.interp, without fused arithmetic.
    slope*(query-x[left])+y[left]
}

/// Fritsch-Butland derivatives and endpoint limiting used by SciPy PCHIP.
/// Inversion must evaluate this cubic; exchanging x/y knots changes the map.
pub(super) struct Pchip {
    x: Vec<f64>,
    y: Vec<f64>,
    derivative: Vec<f64>,
}
impl Pchip {
    pub(super) fn new(x: &[f64], y: &[f64]) -> Result<Self> {
        ensure!(x.len() >= 2 && x.len() == y.len() && x.iter().chain(y).all(|v| v.is_finite()) &&
            x.windows(2).all(|pair| pair[1] > pair[0]), "PCHIP requires finite increasing aligned knots");
        let h: Vec<f64> = x.windows(2).map(|pair| pair[1]-pair[0]).collect();
        let slopes: Vec<f64> = y.windows(2).zip(&h).map(|(pair, width)| (pair[1]-pair[0])/width).collect();
        let mut derivative = vec![0.; x.len()];
        if x.len() == 2 {
            derivative.fill(slopes[0]);
        } else {
            let endpoint = |h0: f64, h1: f64, m0: f64, m1: f64| {
                let value = ((2.*h0+h1)*m0-h0*m1)/(h0+h1);
                if value.signum() != m0.signum() { 0. }
                else if m0.signum() != m1.signum() && value.abs() > 3.*m0.abs() { 3.*m0 }
                else { value }
            };
            derivative[0] = endpoint(h[0], h[1], slopes[0], slopes[1]);
            let last = x.len()-1;
            derivative[last] = endpoint(h[last-1], h[last-2], slopes[last-1], slopes[last-2]);
            for i in 1..last {
                let left = slopes[i-1];
                let right = slopes[i];
                if left != 0. && right != 0. && left.signum() == right.signum() {
                    let w1 = 2.*h[i]+h[i-1];
                    let w2 = h[i]+2.*h[i-1];
                    derivative[i] = 1. / ((w1/left+w2/right)/(w1+w2));
                }
            }
        }
        Ok(Self { x: x.to_vec(), y: y.to_vec(), derivative })
    }

    pub(super) fn evaluate(&self, query: f64) -> f64 {
        let i = self.x.partition_point(|&value| value <= query).saturating_sub(1).min(self.x.len()-2);
        let width = self.x[i+1]-self.x[i];
        let slope = (self.y[i+1]-self.y[i])/width;
        let t = (self.derivative[i]+self.derivative[i+1]-2.*slope)/width;
        let c1 = (slope-self.derivative[i])/width-t;
        let delta = query-self.x[i];
        ((t/width*delta+c1)*delta+self.derivative[i])*delta+self.y[i]
    }
}
