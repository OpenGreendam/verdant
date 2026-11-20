use anyhow::{ensure, Context, Result};
use rustfft::{num_complex::Complex, FftPlanner};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{f64::consts::PI, sync::LazyLock};
use crate::core::NumericArray;
use crate::dsp::lpc::lsf_to_lpc;
use super::common::{bounded_number, exact_object, median, median_in_place, validate_lsf, Pchip};

const PROFILE_BYTES: &[u8] = include_bytes!("phonetic_correction_profiles.json");
const INVENTORY_BYTES: &[u8] = include_bytes!("../../../verdant-backend/src/data/phoneme_inventory.json");
struct Assets { profiles: Value, inventory: Value, profile_sha256: String }
static ASSETS: LazyLock<std::result::Result<Assets, String>> = LazyLock::new(|| {
        let parse = || -> Result<Assets> {
            let profiles: Value = serde_json::from_slice(PROFILE_BYTES)?;
            let inventory: Value = serde_json::from_slice(INVENTORY_BYTES)?;
            ensure!(profiles["schema"] == "verdant-phonetic-correction-profiles/1", "unsupported phonetic correction profiles");
            ensure!(inventory["schema_version"] == "verdant-phonemes/2", "unsupported phoneme inventory");
            Ok(Assets { profiles, inventory, profile_sha256: hex::encode(Sha256::digest(PROFILE_BYTES)) })
        };
        parse().map_err(|error| error.to_string())
});
fn assets() -> Result<&'static Assets> {
    ASSETS.as_ref().map_err(|error| anyhow::anyhow!("{error}"))
}

pub fn validate_phonetic_correction(value: &Value) -> Result<Option<Value>> {
    if value.is_null() { return Ok(None); }
    let object = exact_object(value, &["method", "strength"], "phonetic_correction")?;
    ensure!(object["method"] == "articulatory-v1", "unsupported phonetic correction method");
    let strength = bounded_number(&object["strength"], 0., 1., "phonetic correction strength")?;
    Ok(Some(json!({"method": "articulatory-v1", "strength": strength})))
}
fn base_symbol(symbol: &str) -> &str { symbol.strip_suffix('ː').unwrap_or(symbol) }

fn trace(segment: &Value, actual_key: &Value, inventory: &Value) -> Result<Option<Value>> {
    let Some(trace) = segment.get("phoneme").filter(|value| !value.is_null()) else { return Ok(None); };
    ensure!(trace.is_object(), "phonetic correction requires an object phoneme trace");
    let language = trace["source_language"].as_str().context("phonetic trace requires source_language")?;
    let symbol = trace["canonical_ipa"].as_str().context("phonetic trace requires canonical_ipa")?;
    let key = trace["native_key"].as_str().context("phonetic trace requires native_key")?;
    ensure!(inventory["languages"].get(language).is_some() && inventory["units"].get(symbol).is_some() && actual_key.as_str() == Some(key),
        "phonetic correction trace must match the actual native phone and a known language/IPA unit");
    let mut mapping = inventory["languages"][language]["mappings"].get(symbol)
        .context("phonetic correction target is not mapped for its stated language")?;
    let mut effective = symbol;
    let realization = trace.get("realization").filter(|value| !value.is_null());
    if let Some(realization) = realization {
        ensure!(realization.is_object() && language == "ja" && symbol == "ɴ" &&
            realization["schema_version"] == "verdant-japanese-realization/1" && realization["source_mora"] == "ん" &&
            realization["source_phonological"] == "moraic_nasal", "unsupported or inconsistent phonetic realization trace");
        let target = realization["target_ipa"].as_str().context("phonetic nasal realization requires target_ipa")?;
        let following = realization["following_ipa"].as_str().context("phonetic nasal realization requires following_ipa")?;
        ensure!(inventory["japanese_kana"]["moraic_nasal_allophones"][following] == target,
            "phonetic nasal realization does not match its declared following phone");
        mapping = inventory["languages"][language]["mappings"].get(target)
            .context("phonetic nasal realization is not mapped for its stated language")?;
        effective = target;
    }
    ensure!(mapping["phones"].as_array().is_some_and(|phones| phones.iter().any(|phone| phone == key)),
        "phonetic correction trace's target and native model path disagree");
    Ok(Some(json!({"source_language": language, "canonical_ipa": symbol, "effective_ipa": effective,
        "native_key": key, "support": mapping["support"], "source_index": trace["source_index"],
        "syllable_index": trace["syllable_index"], "realization": realization})))
}
fn matching_unit(left: Option<&Value>, right: Option<&Value>) -> bool {
    let (Some(left), Some(right)) = (left, right) else { return false; };
    ["source_index", "syllable_index"].iter().all(|key| left[*key].as_u64().is_some() && right[*key].as_u64().is_some()) &&
        ["source_language", "canonical_ipa", "source_index", "syllable_index"].iter().all(|key| left[*key] == right[*key])
}
fn recipe_key(trace: Option<&Value>) -> Option<(&str, &str, &str)> {
    let trace = trace?;
    Some((trace["source_language"].as_str()?, base_symbol(trace["effective_ipa"].as_str()?), trace["native_key"].as_str()?))
}
fn recipe<'a>(trace: Option<&Value>, profiles: &'a Value) -> Option<&'a Value> {
    let (language, target, native_key) = recipe_key(trace)?;
    profiles["vowel_recipes"].as_array()?.iter().find(|recipe|
        recipe["language"] == language && recipe["target"] == target && recipe["native_key"] == native_key)
}

fn low_resonance(values: &[f64], frames: usize, order: usize, sample_rate: u32, gamma: f64) -> Result<Option<Value>> {
    let first = frames/4;
    let last = frames-frames/4;
    let mut median_lsf = Vec::with_capacity(order);
    let mut column = Vec::with_capacity(last-first);
    for i in 0..order {
        column.clear();
        column.extend((first..last).map(|frame| values[frame*order+i]));
        median_lsf.push(median_in_place(&mut column));
    }
    let mut lpc = lsf_to_lpc(&median_lsf)?;
    for (i, coefficient) in lpc.iter_mut().enumerate() { *coefficient *= gamma.powi(i as i32); }
    let mut spectrum = vec![Complex::new(0., 0.); 8192];
    for (sample, coefficient) in spectrum.iter_mut().zip(lpc) { sample.re = coefficient; }
    FftPlanner::<f64>::new().plan_fft_forward(8192).process(&mut spectrum);
    let db: Vec<f64> = spectrum[..4096].iter().map(|value|
        20.*(1./value.norm()).max(1e-12).log10()).collect();
    ensure!(db.iter().all(|value| value.is_finite()), "nonfinite source LPC envelope");
    let nyquist = sample_rate as f64/2.;
    let mut i = 1;
    while i+1 < db.len() {
        if db[i] <= db[i-1] { i += 1; continue; }
        let plateau_first = i;
        let mut plateau_last = i;
        while plateau_last+1 < db.len() && db[plateau_last+1] == db[i] { plateau_last += 1; }
        if plateau_last+1 == db.len() || db[plateau_last+1] >= db[i] { i = plateau_last+1; continue; }
        let peak = (plateau_first+plateau_last)/2;
        let height = db[peak];
        let mut left_min = height;
        for &value in db[..peak].iter().rev() {
            if value > height { break; }
            left_min = left_min.min(value);
        }
        let mut right_min = height;
        for &value in &db[peak+1..] {
            if value > height { break; }
            right_min = right_min.min(value);
        }
        let prominence = height-left_min.max(right_min);
        let frequency = peak as f64*(sample_rate as f64/8192.);
        if prominence >= 3. && frequency > nyquist/64. && frequency < nyquist*0.18 {
            return Ok(Some(json!({"frequency_hz": frequency, "prominence_db": prominence,
                "method": "first low-frequency peak of the actual median LPC envelope, prominence >= 3 dB",
                "scope": "source-relative scale landmark; not a measured Japanese F1 or F2"})));
        }
        i = plateau_last+1;
    }
    Ok(None)
}
fn edge_weight(count: usize, hop: usize, sample_rate: u32, spectral_edge: f64) -> Vec<f64> {
    let duration = count as f64*hop as f64/sample_rate as f64;
    let ramp = spectral_edge.min(duration/4.);
    (0..count).map(|frame| {
        let time = (frame as f64+0.5)*hop as f64/sample_rate as f64;
        let first = ((time/ramp).min(1.)*PI/2.).sin().powi(2);
        let last = (((duration-time)/ramp).min(1.)*PI/2.).sin().powi(2);
        first*last
    }).collect()
}
fn operation(records: &mut [Value], index: usize, operation: Value) -> Result<()> {
    records[index]["operations"].as_array_mut().context("invalid phonetic operations report")?.push(operation);
    Ok(())
}
fn store(value: f64, float32: bool) -> f64 { if float32 { (value as f32) as f64 } else { value } }

pub fn correct_spectrum(source: &NumericArray, segments: &[Value], counts: &[usize], native_keys: &[Value],
    sample_rate: u32, hop: usize, configuration: &Value, gamma: f64) -> Result<(NumericArray, Option<Value>)> {
    let config = validate_phonetic_correction(configuration)?;
    let Some(config) = config.filter(|config| config["strength"].as_f64() != Some(0.)) else { return Ok((source.clone(), None)); };
    let (frames, order) = validate_lsf(source)?;
    ensure!(segments.len() == counts.len() && segments.len() == native_keys.len() && !counts.is_empty() &&
        counts.iter().all(|&count| count > 0), "phonetic correction requires aligned segment identities and positive counts");
    let mut offsets = Vec::with_capacity(counts.len()+1);
    offsets.push(0usize);
    for &count in counts { offsets.push(offsets.last().unwrap().checked_add(count).context("phonetic frame count overflow")?); }
    ensure!(offsets.last() == Some(&frames), "phonetic segment counts do not cover LSFs");
    ensure!(sample_rate > 0 && hop > 0 && gamma.is_finite() && gamma > 0. && gamma <= 1.,
        "invalid phonetic correction sample grid or bandwidth expansion");
    let assets = assets()?;
    let profiles = &assets.profiles;
    let inventory = &assets.inventory;
    let traces: Vec<Option<Value>> = segments.iter().zip(native_keys).map(|(segment, key)| trace(segment, key, inventory)).collect::<Result<_>>()?;
    let mut records: Vec<Value> = traces.iter().enumerate().map(|(i, trace)| json!({
        "run_segment_index": i, "first_frame": offsets[i], "end_frame": offsets[i+1], "trace": trace,
        "status": "unmodified", "operations": [], "reason": if trace.is_none() { "no explicit target IPA trace" }
        else { "no calibrated correction recipe; approximate mapping remains approximate" }})).collect();
    let strength = config["strength"].as_f64().unwrap();
    let float32 = matches!(source.dtype.as_str(), "float32" | "<f4" | ">f4" | "f4");
    let mut corrected = source.clone();
    let nyquist = sample_rate as f64/2.;
    let mut begin = 0;
    while begin < traces.len() {
        let key = recipe_key(traces[begin].as_ref());
        let mut end = begin+1;
        while end < traces.len() && recipe_key(traces[end].as_ref()) == key { end += 1; }
        if let Some(recipe) = recipe(traces[begin].as_ref(), profiles) {
            let first = offsets[begin];
            let last = offsets[end];
            let landmark = low_resonance(&source.values[first*order..last*order], last-first, order, sample_rate, gamma)?;
            if let Some(landmark) = landmark {
                let scale = landmark["frequency_hz"].as_f64().unwrap();
                let anchors = |key: &str| -> Result<Vec<f64>> {
                    let mut values: Vec<f64> = recipe[key].as_array().context("invalid phonetic frequency anchors")?.iter()
                        .map(|value| value.as_f64().context("invalid phonetic frequency anchor").map(|value| value*scale)).collect::<Result<_>>()?;
                    values.push(nyquist);
                    Ok(values)
                };
                let x = anchors("source_scale_multiples")?;
                let y = anchors("target_scale_multiples")?;
                let safe = x[x.len()-2] < nyquist*0.96 && y[y.len()-2] < nyquist*0.96 &&
                    x.windows(2).all(|v| v[1] > v[0]) && y.windows(2).all(|v| v[1] > v[0]);
                if safe {
                    let interpolator = Pchip::new(&x, &y)?;
                    let moved: Vec<usize> = x.iter().zip(&y).enumerate().filter_map(|(i, (x, y))| (x != y).then_some(i)).collect();
                    ensure!(!moved.is_empty() && moved[0] > 0 && moved[moved.len()-1]+1 < x.len(), "phonetic recipe lacks protected outer anchors");
                    let below = x[moved[0]-1];
                    let above = x[moved[moved.len()-1]+1];
                    let weights = edge_weight(last-first, hop, sample_rate, profiles["spectral_edge_seconds"].as_f64().unwrap());
                    let shared: Vec<usize> = (begin..end).collect();
                    for member in begin..end {
                        let mut maximum = 0f64;
                        for frame in offsets[member]..offsets[member+1] {
                            let weight = strength*weights[frame-first];
                            for column in 0..order {
                                let index = frame*order+column;
                                let original = source.values[index];
                                let frequency = original*nyquist/PI;
                                if frequency > below && frequency < above {
                                    let warped = interpolator.evaluate(frequency)*PI/nyquist;
                                    corrected.values[index] = store(corrected.values[index]+(warped-original)*weight, float32);
                                }
                                maximum = maximum.max((corrected.values[index]-original).abs()*nyquist/PI);
                            }
                        }
                        operation(&mut records, member, json!({"id": recipe["id"], "mechanism": recipe["mechanism"],
                            "landmark": landmark, "applied": true, "shared_scale_run_segment_indices": shared,
                            "source_anchor_hz": x, "target_anchor_hz": y, "identity_below_hz": below,
                            "identity_above_hz": above, "strength": strength, "phonetic_intent": recipe["phonetic_intent"],
                            "maximum_actual_lsf_shift_hz": maximum}))?;
                    }
                } else {
                    for record in &mut records[begin..end] { record["reason"] = json!("source-relative frequency anchors cannot fit safely below Nyquist"); }
                }
            } else {
                for record in &mut records[begin..end] { record["reason"] = json!("source envelope has no trustworthy low-resonance scale landmark"); }
            }
        }
        begin = end;
    }
    let palatal = &profiles["palatalization"];
    for i in 0..traces.len().saturating_sub(1) {
        let (Some(base), Some(helper)) = (traces[i].as_ref(), traces[i+1].as_ref()) else { continue; };
        if !matching_unit(Some(base), Some(helper)) || base["source_language"] != palatal["language"] ||
            helper["native_key"] != palatal["helper_native_key"] || base["native_key"] == palatal["helper_native_key"] { continue; }
        let symbol = base_symbol(base["effective_ipa"].as_str().unwrap());
        if !symbol.ends_with(palatal["target_suffix"].as_str().unwrap()) &&
            !palatal["additional_targets"].as_array().unwrap().iter().any(|target| target == symbol) { continue; }
        let mut helper_after = i+2;
        while helper_after < traces.len() && matching_unit(Some(base), traces[helper_after].as_ref()) &&
            traces[helper_after].as_ref().is_some_and(|trace| trace["native_key"] == palatal["helper_native_key"]) { helper_after += 1; }
        let (start, boundary, helper_end) = (offsets[i], offsets[i+1], offsets[helper_after]);
        let mut column_values = Vec::with_capacity(helper_end-boundary);
        let mut anchor = Vec::with_capacity(order);
        for column in 0..order {
            column_values.clear();
            column_values.extend((boundary..helper_end).map(|frame| source.values[frame*order+column]));
            anchor.push(median_in_place(&mut column_values));
        }
        let maximum = (palatal["maximum_transition_seconds"].as_f64().unwrap()*sample_rate as f64/hop as f64).round_ties_even().max(1.) as usize;
        let count = (boundary-start).min(maximum);
        let mut maximum_mix = 0f64;
        for frame in 0..count {
            let weight = (((frame as f64+0.5)/count as f64*PI/2.).sin().powi(2))*strength*palatal["base_mix"].as_f64().unwrap();
            maximum_mix = maximum_mix.max(weight);
            for column in 0..order {
                let index = (boundary-count+frame)*order+column;
                corrected.values[index] = store(corrected.values[index]*(1.-weight)+anchor[column]*weight, float32);
            }
        }
        let helpers: Vec<usize> = (i+1..helper_after).collect();
        operation(&mut records, i, json!({"id": palatal["id"], "mechanism": "adjacent-helper-lsf-interpolation", "applied": true,
            "helper_run_segment_indices": helpers, "helper_native_key": helper["native_key"], "borrowed_frames": count,
            "maximum_mix": maximum_mix, "phonetic_intent": palatal["phonetic_intent"]}))?;
        for helper_index in i+1..helper_after {
            operation(&mut records, helper_index, json!({"id": palatal["id"], "mechanism": "existing-helper-anchor", "applied": false,
                "used_as_reference": true, "helper_lsf_and_duration_changed_by_anchor": false}))?;
        }
        if let Some(Some(following)) = traces.get(helper_after) {
            let symbol = following["canonical_ipa"].as_str().unwrap();
            if following["source_language"] == base["source_language"] && inventory["units"][symbol]["role"] == "nucleus" {
                let count = (offsets[helper_after+1]-helper_end).min(maximum);
                let mut maximum_mix = 0f64;
                for frame in 0..count {
                    let weight = (((frame as f64+0.5)/count as f64*PI/2.).cos().powi(2))*strength*palatal["following_nucleus_mix"].as_f64().unwrap();
                    maximum_mix = maximum_mix.max(weight);
                    for column in 0..order {
                        let index = (helper_end+frame)*order+column;
                        corrected.values[index] = store(corrected.values[index]*(1.-weight)+anchor[column]*weight, float32);
                    }
                }
                operation(&mut records, helper_after, json!({"id": palatal["id"], "mechanism": "palatal-release-lsf-interpolation", "applied": true,
                    "helper_run_segment_indices": helpers, "borrowed_frames": count,
                    "maximum_mix": maximum_mix, "authored_pitch_or_duration_changed": false}))?;
            }
        }
    }
    validate_lsf(&corrected)?;
    let tap = &profiles["tap"];
    let mut taps = Vec::new();
    let mut groups: Vec<Value> = Vec::new();
    for i in 0..traces.len() {
        if let Some(trace) = traces[i].as_ref() {
            if trace["source_language"] == tap["language"] && trace["native_key"] == tap["native_key"] &&
                tap["targets"].as_array().unwrap().iter().any(|target| target == base_symbol(trace["effective_ipa"].as_str().unwrap())) {
                taps.push(i);
                let merge = groups.last().is_some_and(|group| group["run_segment_indices"].as_array().unwrap().last().and_then(Value::as_u64) == i.checked_sub(1).map(|v| v as u64)) &&
                    i > 0 && matching_unit(traces[i-1].as_ref(), Some(trace)) && traces[i-1].as_ref().is_some_and(|left| left["native_key"] == trace["native_key"]);
                if merge {
                    let group = groups.last_mut().unwrap();
                    group["run_segment_indices"].as_array_mut().unwrap().push(json!(i));
                    group["end_frame"] = json!(offsets[i+1]);
                } else { groups.push(json!({"run_segment_indices": [i], "first_frame": offsets[i], "end_frame": offsets[i+1]})); }
            }
        }
        let operations = records[i]["operations"].as_array().unwrap();
        if operations.iter().any(|operation| operation["applied"] == true) {
            records[i]["status"] = json!("adjusted");
            records[i].as_object_mut().unwrap().remove("reason");
        } else if !operations.is_empty() {
            records[i]["status"] = json!("reference_only");
            records[i]["reason"] = json!("existing model segment used as a spectral reference; its LSF and duration are not changed by that operation");
        }
    }
    Ok((corrected, Some(json!({"method": config["method"], "strength": strength,
        "profile_schema": profiles["schema"], "profile_sha256": assets.profile_sha256,
        "status": profiles["status"], "parameter_provenance": profiles["parameter_provenance"],
        "segments": records, "tap_run_segment_indices": taps, "tap_groups": groups,
        "f0_uv_duration_changed": false, "source_observations_and_model_selection_changed": false}))))
}

fn quantile75(values: &mut [f64]) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    let index = (values.len()-1) as f64*0.75;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let fraction = index-lower as f64;
    let difference = values[upper]-values[lower];
    if fraction >= 0.5 { values[upper]-difference*(1.-fraction) }
    else { values[lower]+difference*fraction }
}

/// Returns the one shared sample envelope and mutates only the execution report.
/// A reference supplied by the adapter is the sole input to contact detection.
pub fn correct_waveform(wave: &[f64], f0: &[f64], voiced: &[bool], report: &mut Value,
    sample_rate: u32, hop: usize) -> Result<Vec<f64>> {
    if report.is_null() { return Ok(vec![1.; wave.len()]); }
    ensure!(sample_rate > 0 && hop > 0 && !f0.is_empty() && f0.len() == voiced.len() &&
        f0.len().checked_mul(hop) == Some(wave.len()) && wave.iter().chain(f0).all(|value| value.is_finite()),
        "phonetic contact requires a finite frame-aligned waveform and F0/UV");
    let assets = assets()?;
    let settings = &assets.profiles["tap"];
    let groups = report["tap_groups"].as_array().context("phonetic contact requires tap_groups report")?.clone();
    let strength = bounded_number(&report["strength"], 0., 1., "phonetic contact strength")?;
    let mut gains = vec![1.; wave.len()];
    for group in groups {
        let first = group["first_frame"].as_u64().context("invalid tap first_frame")? as usize;
        let last = group["end_frame"].as_u64().context("invalid tap end_frame")? as usize;
        ensure!(first < last && last <= f0.len(), "tap group lies outside the run");
        let (start, end) = (first*hop, last*hop);
        let x = &wave[start..end];
        let local_f0: Vec<f64> = (first..last).filter(|&i| voiced[i] && f0[i] > 0.).map(|i| f0[i]).collect();
        let period = if local_f0.is_empty() { x.len() as f64 } else { sample_rate as f64/median(&local_f0) };
        ensure!(period.is_finite() && period > 0. && (2.*period).is_finite() &&
            (2.*period).ceil() < usize::MAX as f64, "phonetic contact period is not representable");
        let window = (0.006*sample_rate as f64).round_ties_even().max((2.*period).ceil()) as usize;
        let mut op = json!({"id": settings["id"], "mechanism": "single-post-filter-contact-envelope",
            "shared_run_segment_indices": group["run_segment_indices"], "phonetic_intent": settings["phonetic_intent"],
            "rms_window_seconds": window as f64/sample_rate as f64});
        let duration = x.len() as f64/sample_rate as f64;
        let width = settings["maximum_contact_seconds"].as_f64().unwrap()
            .min(settings["maximum_duration_fraction"].as_f64().unwrap()*duration);
        if window > x.len()/2 || window as f64/sample_rate as f64 > width {
            op["applied"] = json!(false);
            op["reason"] = json!("segment too short or F0 too low to exclude an existing short contact with a two-period check");
        } else {
            // Sliding sum retains all complete windows, including both edges.
            // Neumaier compensation bounds accumulated cancellation error.
            let mut sum = 0.;
            let mut compensation = 0.;
            let add = |sum: &mut f64, compensation: &mut f64, value: f64| {
                let total = *sum+value;
                if sum.abs() >= value.abs() { *compensation += (*sum-total)+value; }
                else { *compensation += (value-total)+*sum; }
                *sum = total;
            };
            for &value in &x[..window] { add(&mut sum, &mut compensation, value*value); }
            let mut energy = Vec::with_capacity(x.len()-window+1);
            energy.push((sum+compensation)/window as f64);
            for i in window..x.len() {
                add(&mut sum, &mut compensation, -(x[i-window]*x[i-window]));
                add(&mut sum, &mut compensation, x[i]*x[i]);
                energy.push(((sum+compensation)/window as f64).max(0.));
            }
            ensure!(energy.iter().all(|value| value.is_finite() && *value >= 0.), "nonfinite contact energy");
            let reference = quantile75(&mut energy);
            let ratio = (energy[0]/reference.max(1e-30)).sqrt();
            op["existing_minimum_rms_ratio"] = json!(ratio);
            op["existing_contact_search"] = json!("all complete two-period windows, including segment edges; compare minimum with upper-quartile energy");
            if ratio < settings["existing_notch_ratio"].as_f64().unwrap() || reference < 1e-20 {
                op["applied"] = json!(false);
                op["reason"] = json!("existing energy dip or negligible signal retained");
            } else {
                let depth = settings["depth_db"].as_f64().unwrap()*strength;
                let mut minimum = 1f64;
                for sample in 0..x.len() {
                    let time = (sample as f64+0.5)/sample_rate as f64;
                    let phase = ((time-duration/2.)/(width/2.)).clamp(-1., 1.);
                    let bump = (phase*PI/2.).cos().powi(2);
                    let gain = 10f64.powf(-depth*bump/20.);
                    gains[start+sample] *= gain;
                    minimum = minimum.min(gain);
                }
                op["applied"] = json!(true);
                op["contact_seconds"] = json!(width);
                op["center_seconds"] = json!(duration/2.);
                op["depth_db"] = json!(depth);
                op["minimum_gain"] = json!(minimum);
                op["total_segment_seconds"] = json!(duration);
            }
        }
        let members = group["run_segment_indices"].as_array().context("tap group requires segment indices")?;
        for member in members {
            let index = member.as_u64().context("invalid tap segment index")? as usize;
            let rows = report["segments"].as_array_mut().context("phonetic report requires segments")?;
            let row = rows.get_mut(index).context("tap segment index lies outside report")?;
            let first = row["first_frame"].as_u64().context("invalid contact segment first_frame")? as usize;
            let last = row["end_frame"].as_u64().context("invalid contact segment end_frame")? as usize;
            ensure!(first < last && last <= f0.len(), "contact segment lies outside the run");
            let applied = gains[first*hop..last*hop].iter().any(|&gain| gain != 1.);
            let mut item = op.clone();
            item["contact_applied_in_group"] = op["applied"].clone();
            item["applied"] = json!(applied);
            row["operations"].as_array_mut().context("contact segment requires operations")?.push(item);
            if applied {
                row["status"] = json!("adjusted");
                row.as_object_mut().unwrap().remove("reason");
            } else if row["status"] != "adjusted" {
                row["reason"] = op.get("reason").cloned().unwrap_or_else(|| json!("single shared contact lies in another segment of this unit"));
            }
        }
    }
    let rows = report["segments"].as_array().context("phonetic report requires segments")?;
    let adjusted = rows.iter().filter(|row| row["status"] == "adjusted").count();
    let unmodified = rows.iter().filter(|row| row["status"] == "unmodified").count();
    let reference = rows.iter().filter(|row| row["status"] == "reference_only").count();
    report["adjusted_segments"] = json!(adjusted);
    report["unmodified_segments"] = json!(unmodified);
    report["reference_only_segments"] = json!(reference);
    let mut hash = Sha256::new();
    for &gain in &gains { hash.update(gain.to_le_bytes()); }
    report["contact_gain_sha256"] = json!(hex::encode(hash.finalize()));
    Ok(gains)
}
