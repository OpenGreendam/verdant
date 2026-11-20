//! Output-rate conversion and explicit attenuation-only music headroom.
//! The native 48 kHz decoder owns articulation and the learned audio band.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use crate::signal;

#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub sample_rate: u32,
}

impl Configuration {
    /// Absence is handled by the caller. A present null or malformed object
    /// must never silently select either the old path or the new default.
    pub fn parse(value: &Value) -> Result<Self> {
        let object = value.as_object().context("voice_music must be an object")?;
        ensure!(object.len() == 2 && object.contains_key("method") && object.contains_key("sample_rate"),
            "voice_music requires exactly method and sample_rate");
        ensure!(value["method"] == "verdant-voice-music-v1", "unsupported Verdant ONNX music method");
        let rate = value["sample_rate"].as_u64().context("Verdant ONNX music sample_rate must be an integer")?;
        ensure!(matches!(rate, 44_100 | 48_000), "Verdant ONNX music supports 44100 or 48000 Hz");
        Ok(Self { sample_rate: rate as u32 })
    }
}

/// Map the final rate to the original 200 Hz activity clock using exact integer
/// arithmetic. At 44.1 kHz, boundaries alternate between 220 and 221 samples;
/// rounding each frame to a constant hop would accumulate a duration error.
pub fn enforce_rests(samples: &mut [f64], output_rate: u32, active: &[f64]) -> Result<Value> {
    ensure!(matches!(output_rate, 44_100 | 48_000) && !active.is_empty() &&
        active.iter().all(|&v| v == 0. || v == 1.), "invalid music activity clock");
    let expected = (active.len() as u128 * output_rate as u128).div_ceil(200);
    ensure!(samples.len() as u128 == expected, "music output duration changed");
    let mut removed = 0;
    let mut maximum_removed = 0_f64;
    let mut rest_samples = 0;
    for (sample, value) in samples.iter_mut().enumerate() {
        let frame = (sample as u128 * 200 / output_rate as u128) as usize;
        if active[frame] == 0. {
            rest_samples += 1;
            if *value != 0. { removed += 1; maximum_removed = maximum_removed.max(value.abs()); }
            *value = 0.;
        }
    }
    Ok(json!({"method":"exact original activity at rational output sample times",
        "frame_rate":200,"output_sample_rate":output_rate,"rest_samples":rest_samples,
        "removed_tail_samples":removed,"maximum_removed_tail_amplitude":maximum_removed,
        "note_or_phoneme_duration_changed":false,"new_onset_fades_added":false}))
}

pub fn render(mut samples: Vec<f64>, active: &[f64],
    configuration: Configuration) -> Result<(Vec<f64>, Value)> {
    const NATIVE_RATE: u32 = 48_000;
    ensure!(matches!(configuration.sample_rate, 44_100 | NATIVE_RATE) && !active.is_empty() &&
        active.len().checked_mul(240) == Some(samples.len()) && samples.iter().all(|v| v.is_finite()),
        "native voice music requires a finite native 48 kHz / 5 ms waveform");
    let source_peak = signal::peak(&samples);
    let resampled = signal::resample_poly(&samples, 44_100, NATIVE_RATE as usize, 5.)?;
    let mut reference_peak = source_peak;
    let mut probes = Vec::new();
    for (rate, wave) in [(44_100, resampled.as_slice()), (NATIVE_RATE, samples.as_slice())] {
        let sample_peak = signal::peak(wave);
        let interpolated_peak = signal::peak(&signal::resample_poly(wave, 4, 1, 8.6)?);
        reference_peak = reference_peak.max(sample_peak).max(interpolated_peak);
        probes.push(json!({"sample_rate":rate,"sample_peak":sample_peak,
            "four_times_interpolated_peak":interpolated_peak}));
    }
    let gain = if reference_peak > 0.85 { (0.85 / reference_peak) * (1. - 16. * f64::EPSILON) } else { 1. };
    if configuration.sample_rate == 44_100 { samples = resampled; }
    if gain != 1. { for value in &mut samples { *value *= gain; } }
    let rest = enforce_rests(&mut samples, configuration.sample_rate, active)?;
    ensure!(samples.iter().all(|v| v.is_finite()), "nonfinite native voice music output");
    Ok((samples, json!({"method":"verdant-voice-music-v1","decoder_sample_rate":NATIVE_RATE,
        "output_sample_rate":configuration.sample_rate,"frame_period_seconds":0.005,
        "frame_to_sample_mapping":"floor(sample_index * 200 / output_sample_rate)",
        "sample_count_policy":"ceil(native_samples * output_sample_rate / 48000)",
        "articulation":{"method":"native semantic model","legacy_articulation_applied":false,
            "reason":"native semantic model covers articulation",
            "native_conditions_modified":false,"learned_weights_modified":false,
            "f0_uv_or_duration_modified":false},
        "source_headroom":{"method":"verdant-voice-music-headroom-v1","gain":gain,
            "policy":"one constant attenuation over the complete phrase; never amplify quiet input",
            "reference":"provided native 48 kHz waveform, before output-rate conversion",
            "input_sample_rate":NATIVE_RATE,"input_sample_peak":source_peak,
            "maximum_measured_reference_peak":reference_peak,"reserved_base_ceiling":0.85,
            "output_rate_probes":probes,"same_gain_for_both_output_rates":true,
            "oversampling_factor":4,"oversampling_beta":8.6,"certified_true_peak_meter":false,
            "loudness_target":null,"time_varying_gain":false,"core_archive_modified":false},
        "bandwidth":{"applied":false,"method":"preserved native learned band",
            "legacy_16k_highband_prior_applied":false,"source_sample_rate":NATIVE_RATE,
            "output_sample_rate":configuration.sample_rate,
            "resampling":if configuration.sample_rate == NATIVE_RATE {"none"} else {"signal::resample_poly; 147/160; Kaiser beta 5"}},
        "authored_rests":rest,"original_highband_recovered":false,
        "normalization":"no upward normalization; explicit attenuation-only music headroom",
        "execution_scope":"whole phrase offline; no bounded streaming claim"})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_contract_never_silently_accepts_another_rate_or_field() {
        for rate in [44100, 48000] {
            assert_eq!(Configuration::parse(&json!({"method":"verdant-voice-music-v1","sample_rate":rate})).unwrap().sample_rate, rate);
        }
        for bad in [Value::Null, json!({"method":"verdant-voice-music-v1","sample_rate":16000}),
            json!({"method":"verdant-voice-music-v1","sample_rate":48000.5}),
            json!({"method":"unknown","sample_rate":48000}),
            json!({"method":"verdant-voice-music-v1","sample_rate":48000,"strength":1})] {
            assert!(Configuration::parse(&bad).is_err());
        }
    }

    #[test]
    fn long_44100_activity_clock_has_no_per_frame_rounding_drift() {
        let mut active = vec![1.; 24001];
        for i in (1..active.len()).step_by(2) { active[i] = 0.; }
        let count = (active.len() * 44100).div_ceil(200);
        let mut wave = vec![0.125; count];
        let report = enforce_rests(&mut wave, 44100, &active).unwrap();
        assert_eq!(wave.len(), 5292221);
        assert_eq!(report["removed_tail_samples"], 2640000);
        for (sample, &value) in wave.iter().enumerate() {
            assert_eq!(value, if active[sample * 200 / 44100] == 0. { 0. } else { 0.125 });
        }
        assert!(enforce_rests(&mut wave[..count - 1], 44100, &active).is_err());
    }

    #[test]
    fn native_music_retains_its_clock_rests_and_quiet_level() {
        let input = vec![0.125; 4*240];
        let active = [1.,0.,1.,1.];
        for rate in [44100,48000] {
            let (audio,report) = render(input.clone(),&active,Configuration {sample_rate:rate}).unwrap();
            assert_eq!(audio.len(), (active.len()*rate as usize).div_ceil(200));
            assert_eq!(report["decoder_sample_rate"],48000);
            assert_eq!(report["source_headroom"]["gain"],1.);
            assert_eq!(report["bandwidth"]["applied"],false);
            for (sample,&value) in audio.iter().enumerate() {
                if active[sample*200/rate as usize]==0. {assert_eq!(value,0.);}
            }
            if rate==48000 {assert_eq!(&audio[..240],&input[..240]);}
        }
    }
}
