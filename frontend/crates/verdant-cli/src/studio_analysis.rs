//! Read-only offline analyses. Each result includes native parameter values
//! that can be applied through the revision-checked plugin.set operation.
use super::*;
use std::time::Instant;
use verdant_media::{AssetPolicy, DecodedAudio, MediaImporter};
use verdant_plugins::telemetry::{TelemetryCapture, FFT_SIZE};

fn import(source:&Path)->Result<DecodedAudio,CliError>{
    let policy=AssetPolicy::for_file(source).map_err(crate::integrations::media_error)?;
    Ok(MediaImporter::new(policy,Default::default()).import_file(source).map_err(crate::integrations::media_error)?.audio)
}
fn values(parameters:&[(String,f64)])->BTreeMap<String,f64>{parameters.iter().cloned().collect()}
fn fingerprint(source:&Path,audio:&DecodedAudio)->Value{
    json!({"source":source,"sha256":audio.sha256,"sample_rate":audio.sample_rate,
        "frames":audio.frames(),"channels":audio.channels})
}

pub(super) fn run(op:&str,mut p:Params)->Outcome{
    match op{
        "analysis.eq-fit"=>{
            let points:Vec<(f64,f64)>=p.required("points")?;
            let max_bands:usize=p.optional("max_bands")?.unwrap_or(24);
            let rate:f64=p.optional("sample_rate")?.unwrap_or(48_000.);
            p.finish()?;
            let fit=dsp::eq_fit_curve_report(&points,max_bands,rate).map_err(invalid)?;
            Ok(json!({"schema":"verdant-native-analysis/1","device":dsp::EQ_ID,"sample_rate":rate,
                "values":values(&fit.parameters),"fit":fit,
                "method":"bounded native bell and shelf response fit; apply values with plugin.set"}))
        }
        "analysis.eq-match"=>{
            let source:PathBuf=p.required("source")?;let reference:PathBuf=p.required("reference")?;
            let mut options=dsp::EqMatchOptions::default();
            if let Some(v)=p.optional("max_bands")?{options.max_bands=v;}
            if let Some(v)=p.optional("sample_rate")?{options.sample_rate=v;}
            if let Some(v)=p.optional("min_frequency")?{options.min_frequency=v;}
            if let Some(v)=p.optional("max_frequency")?{options.max_frequency=v;}
            if let Some(v)=p.optional("amount")?{options.amount=v;}
            if let Some(v)=p.optional("smoothing_octaves")?{options.smoothing_octaves=v;}
            if let Some(v)=p.optional("remove_level_offset")?{options.remove_level_offset=v;}
            let windows:usize=p.optional("max_windows")?.unwrap_or(64);
            let start:f64=p.optional("start_seconds")?.unwrap_or(0.);
            let duration:Option<f64>=p.optional("duration_seconds")?;p.finish()?;
            if !(1..=256).contains(&windows)||!start.is_finite()||start<0.||
                duration.is_some_and(|v|!v.is_finite()||v<=0.){
                return Err(invalid("analysis requires max_windows 1..256, start_seconds >= 0 and a positive optional duration_seconds"));
            }
            let audio=import(&source)?;let reference_audio=import(&reference)?;
            let(a,a_window)=spectrum(&audio,windows,start,duration)?;
            let(b,b_window)=spectrum(&reference_audio,windows,start,duration)?;
            let fit=dsp::eq_match_spectra_with_options(&a,&b,options).map_err(invalid)?;
            Ok(json!({"schema":"verdant-native-analysis/1","device":dsp::EQ_ID,
                "source":fingerprint(&source,&audio),"reference":fingerprint(&reference,&reference_audio),
                "source_window":a_window,"reference_window":b_window,"options":options,
                "values":values(&fit.parameters),"fit":fit,
                "method":"evenly sampled 2048-frame Hann windows; average stereo power across windows; bounded native response fit"}))
        }
        "analysis.reverb-ir"=>{
            let source:PathBuf=p.required("source")?;let requested:Option<usize>=p.optional("channel")?;p.finish()?;
            let audio=import(&source)?;let channels=usize::from(audio.channels);
            let channel=if let Some(channel)=requested{
                if channel>=channels{return Err(invalid("IR channel is outside the source channel count"));}
                channel
            }else{
                (0..channels).max_by(|&a,&b|{
                    let power=|channel:usize|audio.samples.chunks_exact(channels).map(|frame|f64::from(frame[channel]).powi(2)).sum::<f64>();
                    power(a).total_cmp(&power(b))
                }).unwrap_or(0)
            };
            let mono=audio.samples.chunks_exact(channels).map(|f|f[channel]).collect::<Vec<_>>();
            let fit=dsp::fit_reverb_ir_report(&mono,f64::from(audio.sample_rate)).map_err(invalid)?;
            Ok(json!({"schema":"verdant-native-analysis/1","device":dsp::REVERB_ID,
                "source":fingerprint(&source,&audio),"channel":channel,
                "values":values(&fit.parameters),"fit":fit,
                "method":"noise-aware decay and spectral parameter estimation for the native algorithmic reverb; apply values with plugin.set"}))
        }
        _=>Err(invalid("unknown native analysis")),
    }
}

/// The same stereo-power FFT as the native editor, averaged over explicit,
/// evenly spaced source windows. Silence fails instead of yielding a fake fit.
fn spectrum(audio:&DecodedAudio,max_windows:usize,start:f64,duration:Option<f64>)->Result<(Vec<(f64,f64)>,Value),CliError>{
    let rate=f64::from(audio.sample_rate);let total=audio.frames() as usize;
    let from=(start*rate).round().clamp(0.,total as f64) as usize;
    let end=duration.map(|d|((start+d)*rate).round().clamp(0.,total as f64)as usize).unwrap_or(total);
    if end<=from+1{return Err(invalid("analysis window contains fewer than two source frames"));}
    let frames=end-from;let count=frames.div_ceil(FFT_SIZE).min(max_windows).max(1);
    let mut power=vec![0f64;FFT_SIZE/2+1];let mut frequencies=vec![];let mut energy=0f64;
    let channels=usize::from(audio.channels);
    let mut left=[0f32;FFT_SIZE];let mut right=[0f32;FFT_SIZE];
    let mut capture=TelemetryCapture::new(rate).map_err(invalid)?;
    for index in 0..count{
        let offset=from+if count==1{0}else{index*(frames.saturating_sub(FFT_SIZE))/(count-1)};
        let take=(end-offset).min(FFT_SIZE);left.fill(0.);right.fill(0.);
        for i in 0..take{
            left[i]=audio.samples[(offset+i)*channels];
            right[i]=audio.samples[(offset+i)*channels+if channels==2{1}else{0}];
            energy+=f64::from(left[i]).powi(2)+f64::from(right[i]).powi(2);
        }
        capture.capture(&left,&right,&left,&right).map_err(invalid)?;
        let snapshot=capture.frame().analyze(Instant::now()).map_err(invalid)?;
        if frequencies.is_empty(){frequencies=snapshot.spectrum_frequencies;}
        for (sum,db) in power.iter_mut().zip(snapshot.spectrum_input_db){*sum+=10f64.powf(f64::from(db)/10.);}
    }
    if energy<=1e-16{return Err(invalid("analysis window is silent"));}
    let bins=frequencies.into_iter().zip(power).filter_map(|(f,p)|{
        let f=f64::from(f);(20. ..=20_000.).contains(&f).then_some((f,10.*(p/count as f64).max(1e-12).log10()))
    }).collect::<Vec<_>>();
    if bins.len()<2{return Err(invalid("source rate does not provide enough audible spectrum bins"));}
    Ok((bins,json!({"start_frame":from,"end_frame":end,"sample_rate":audio.sample_rate,"windows":count,
        "fft_frames":FFT_SIZE,"stereo_power_average":true})))
}
