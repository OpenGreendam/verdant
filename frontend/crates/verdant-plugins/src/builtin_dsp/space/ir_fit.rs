//! Offline measurements and editable algorithmic approximation of an IR.
//! No sample data is retained by the reverb and no convolution is performed.
use super::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReverbIrBandFit {
    pub frequency: f64,
    pub decay_seconds: Option<f64>,
    pub decay_r_squared: Option<f64>,
    /// Energy relative to a white spectrum, corrected for analysis bandwidth.
    pub relative_level_db: f64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReverbIrFitReport {
    pub parameters: Vec<(String,f64)>,
    pub predelay_ms: f64,
    pub decay_seconds: f64,
    pub decay_r_squared: f64,
    pub decay_span_db: f64,
    pub noise_floor_dbfs: f64,
    pub early_energy_percent: f64,
    pub decay_model_rms_percent: f64,
    pub bands: Vec<ReverbIrBandFit>,
}
#[derive(Clone, Copy, Debug)]
struct DecayMeasurement { rt60: f64, r_squared: f64, span: f64, noise: f64 }

fn energy_blocks(samples:&[f32],fs:f64)->Vec<(f64,usize)> {
    samples.chunks((fs*0.005).round().max(1.) as usize)
        .map(|block|(block.iter().map(|&x|(x as f64).powi(2)).sum(),block.len())).collect()
}
fn tail_noise(blocks:&[(f64,usize)])->f64 {
    if blocks.len()<20 {return 0.}
    let split=blocks.len()*7/10;let middle=blocks.len()*85/100;
    let mean=|slice:&[(f64,usize)]|slice.iter().map(|v|v.0).sum::<f64>()/slice.iter().map(|v|v.1).sum::<usize>().max(1) as f64;
    let a=mean(&blocks[split..middle]);let b=mean(&blocks[middle..]);
    // An exponentially falling tail is signal. Only a stable final plateau is
    // treated as stationary recording noise; a truncated decay is not a floor.
    if b>1e-24 && a/b<1.8 {mean(&blocks[split..])} else {0.}
}
fn measure_decay(samples:&[f32],fs:f64)->Option<DecayMeasurement> {
    let blocks=energy_blocks(samples,fs);if blocks.len()<8{return None}
    let noise=tail_noise(&blocks);
    let maximum=blocks.iter().map(|b|b.0/b.1 as f64).fold(0.,f64::max);
    if maximum<1e-20 || (noise>0. && maximum/noise<32.) {return None}
    let mut integral=vec![0.;blocks.len()];let mut remaining=0.;
    for i in (0..blocks.len()).rev() {
        remaining=(remaining+blocks[i].0-noise*blocks[i].1 as f64).max(0.);
        integral[i]=remaining;
    }
    let total=integral[0];if total<1e-18{return None}
    let (mut n,mut sx,mut sy,mut sxx,mut sxy,mut syy)=(0.,0.,0.,0.,0.,0.);
    let (mut first,mut last)=(0.,0.);let mut frames=0;
    for (i,block) in blocks.iter().enumerate() {
        let db=10.*(integral[i]/total).max(1e-30).log10();
        if (-35. ..=-5.).contains(&db) && (noise==0. || block.0/block.1 as f64>noise*4.) {
            let t=frames as f64/fs;
            if n==0. {first=db} last=db;
            n+=1.;sx+=t;sy+=db;sxx+=t*t;sxy+=t*db;syy+=db*db;
        }
        frames+=block.1;
    }
    let den=n*sxx-sx*sx;let span=first-last;
    if n<6. || span<10. || den<1e-15{return None}
    let slope=(n*sxy-sx*sy)/den;
    let r_squared=((n*sxy-sx*sy).powi(2)/(den*(n*syy-sy*sy)).max(1e-30)).clamp(0.,1.);
    if slope>=-0.1 || r_squared<0.8 {return None}
    Some(DecayMeasurement{rt60:(-60./slope).clamp(0.05,80.),r_squared,span,noise})
}
fn equivalent_noise_bandwidth(coefficients:super::super::filters::Coefficients,frequency:f64,fs:f64)->f64 {
    // Parseval: sum of squared impulse samples equals output/input white-noise
    // power. This remains accurate for 63 Hz even at a 384 kHz sample rate.
    let mut filter=Biquad::default();filter.set(coefficients);
    let n=(24.*fs/frequency).ceil().clamp(64.,262144.) as usize;
    (0..n).map(|i|filter.run(if i==0 {1.}else{0.}).powi(2)).sum::<f64>().max(1e-12)
}
fn decay_score(values:&[f64],bands:&[ReverbIrBandFit],fs:f64)->f64 {
    let (mut sum,mut weight)=(0.,0.);
    for band in bands {
        if let Some(rt)=band.decay_seconds {
            let confidence=band.decay_r_squared.unwrap_or(0.8);
            let predicted=reverb_decay_at_frequency(values,band.frequency,fs);
            sum+=confidence*(predicted/rt).ln().powi(2);weight+=confidence;
        }
    }
    sum/weight.max(1.)
}
fn refine_decay_nodes(values:&mut [f64],bands:&[ReverbIrBandFit],fs:f64) {
    let mut score=decay_score(values,bands,fs);
    let mut step=0.65;
    for pass in 0..18 {
        let mut improved=false;
        for i in 0..bands.len() {
            if bands[i].decay_seconds.is_none(){continue}
            let index=REVERB_DECAY_START+i*5+2;let old=values[index];let mut best=old;
            for direction in [-1.,1.] {
                // Attenuating feedback nodes avoid the stability limiter
                // flattening or shortening unrelated frequency regions.
                values[index]=(old*2_f64.powf(direction*step)).clamp(25.,100.);
                let trial=decay_score(values,bands,fs);
                if trial+1e-12<score {score=trial;best=values[index];improved=true;}
            }
            values[index]=best;
        }
        if !improved || pass%3==2 {step*=0.6;}
    }
}

/// Analyze a mono recording on a control/worker thread. The returned parameters
/// describe an editable FDN approximation and include no proprietary IR assets.
pub fn fit_reverb_ir_report(samples:&[f32],sample_rate:f64)->Result<ReverbIrFitReport,String> {
    if !sample_rate.is_finite() || !(8000. ..=384000.).contains(&sample_rate) ||
        samples.len()<64 || samples.len()>(sample_rate*60.) as usize || samples.iter().any(|x|!x.is_finite()) {
        return Err("IR requires finite mono PCM, 64 samples to 60 seconds, at 8–384 kHz".into())
    }
    let peak=samples.iter().map(|x|x.abs()).fold(0_f32,f32::max);
    if peak<1e-8 {return Err("IR contains no measurable energy".into())}
    let noise=tail_noise(&energy_blocks(samples,sample_rate));
    let threshold=(peak as f64*0.01).max(noise.sqrt()*6.);
    let onset=samples.iter().position(|x|x.abs() as f64>threshold).ok_or("IR does not rise above the measured noise floor")?;
    let peak_at=samples.iter().position(|x|x.abs()==peak).unwrap_or(onset);
    if peak_at>onset+(sample_rate*0.25) as usize {return Err("Input is not a transient impulse response".into())}
    let tail=&samples[onset..];
    let measured=measure_decay(tail,sample_rate).ok_or("IR needs at least 10 dB of reliable decay above its noise floor")?;
    let total=tail.iter().map(|&x|(x as f64).powi(2)).sum::<f64>();
    let corrected_total=(total-measured.noise*tail.len() as f64).max(1e-18);
    let mut bands=Vec::new();
    for frequency in [63.,125.,250.,500.,1000.,2000.,4000.,8000.] {
        if frequency>=sample_rate*0.45 {continue}
        let coefficients=rbj(Rbj::BandPass,frequency,std::f64::consts::SQRT_2,0.,sample_rate);
        let bandwidth=equivalent_noise_bandwidth(coefficients,frequency,sample_rate);
        let mut filter=Biquad::default();filter.set(coefficients);
        let filtered:Vec<f32>=tail.iter().map(|&x|filter.run(x as f64) as f32).collect();
        let decay=measure_decay(&filtered,sample_rate);
        let energy=filtered.iter().map(|&x|(x as f64).powi(2)).sum::<f64>();
        let signal=(energy-measured.noise*bandwidth*filtered.len() as f64).max(1e-24);
        let relative_level_db=(10.*(signal/corrected_total/bandwidth).log10()).clamp(-24.,24.);
        bands.push(ReverbIrBandFit{frequency,decay_seconds:decay.map(|d|d.rt60),
            decay_r_squared:decay.map(|d|d.r_squared),relative_level_db});
    }
    let early_len=(sample_rate*0.080).round().min(tail.len() as f64) as usize;
    let early_energy=tail[..early_len].iter().map(|&x|(x as f64).powi(2)).sum::<f64>();
    let early_fraction=((early_energy-measured.noise*early_len as f64).max(0.)/corrected_total).clamp(0.,1.);
    let exponential_fraction=1.-10_f64.powf(-6.*0.080/measured.rt60);
    let excess=((early_fraction-exponential_fraction)/(1.-exponential_fraction).max(0.05)).clamp(0.,1.);
    let density_start=(sample_rate*0.002).round() as usize;
    let density_slice=&tail[density_start.min(early_len)..early_len];
    let early_rms=(density_slice.iter().map(|&x|(x as f64).powi(2)).sum::<f64>()/density_slice.len().max(1) as f64).sqrt();
    let density=(density_slice.iter().filter(|&&x|x.abs() as f64>early_rms).count() as f64/
        density_slice.len().max(1) as f64/0.31731).clamp(0.,1.);
    let mut values=super::super::table_defaults(REVERB_SPECS);
    let mut edits=std::collections::BTreeMap::new();
    for (id,value) in [
        ("decay",bands.iter().filter_map(|b|b.decay_seconds).fold(measured.rt60,f64::max).clamp(0.2,20.)),
        ("decay_rate",100.),("predelay",(onset as f64/sample_rate*1000.).min(250.)),("predelay_sync",0.),
        ("style",0.),("room",0.),("size",100.),("freeze",0.),("auto_gate",0.),("ducking",0.),
        ("distance",0.),("thickness",0.),("modulation",0.),("decay_low_rate",100.),("decay_high_rate",100.),
        ("damping",20000.),("low_cut",20.),("high_cut",20000.),("post_low_gain",0.),("post_high_gain",0.),
        ("post_mid_gain",0.),("diffusion",density*100.),("early_level",excess.sqrt()*100.)
    ] {edits.insert(id.to_string(),value);}
    for i in 0..8 {edits.insert(format!("decay{}_on",i+1),0.);edits.insert(format!("post{}_on",i+1),0.);}
    for (i,band) in bands.iter().enumerate() {
        if band.decay_seconds.is_none(){continue}
        let shape=if i==0 {1.}else if i+1==bands.len(){2.}else{0.};
        for (field,value) in [("on",1.),("freq",band.frequency),("rate",100.),("q",1.414),("shape",shape)] {
            edits.insert(format!("decay{}_{field}",i+1),value);
        }
    }
    for (id,value) in &edits {
        if let Some(index)=REVERB_SPECS.iter().position(|s|s.id==id.as_str()) {values[index]=*value;}
    }
    refine_decay_nodes(&mut values,&bands,sample_rate);
    for i in 0..bands.len() {edits.insert(format!("decay{}_rate",i+1),values[REVERB_DECAY_START+i*5+2]);}
    let curve:Vec<_>=bands.iter().map(|b|(b.frequency,b.relative_level_db.clamp(-12.,12.))).collect();
    if let Ok(post)=super::super::eq_fit_curve_report(&curve,8,sample_rate) {
        let map:std::collections::BTreeMap<_,_>=post.parameters.into_iter().collect();
        for i in 0..post.bands_used.min(8) {
            let source=|field:&str|map.get(&format!("b{}_{field}",i+1)).copied().unwrap_or(0.);
            let shape=match source("shape").round() as usize {1=>1.,3=>2.,_=>0.};
            for (field,value) in [("on",1.),("freq",source("freq")),("gain",source("gain")),("q",source("q").clamp(0.1,12.)),("shape",shape)] {
                edits.insert(format!("post{}_{field}",i+1),value);
            }
        }
    }
    let valid:Vec<_>=bands.iter().filter_map(|b|b.decay_seconds.map(|rt|(b.frequency,rt))).collect();
    let model_rms=(valid.iter().map(|(f,rt)|(reverb_decay_at_frequency(&values,*f,sample_rate)/rt-1.).powi(2)).sum::<f64>()/
        valid.len().max(1) as f64).sqrt()*100.;
    Ok(ReverbIrFitReport{parameters:edits.into_iter().collect(),predelay_ms:onset as f64/sample_rate*1000.,
        decay_seconds:measured.rt60,decay_r_squared:measured.r_squared,decay_span_db:measured.span,
        noise_floor_dbfs:10.*measured.noise.max(1e-16).log10(),early_energy_percent:early_fraction*100.,
        decay_model_rms_percent:model_rms,bands})
}
pub fn fit_reverb_ir(samples:&[f32],sample_rate:f64)->Result<Vec<(String,f64)>,String> {
    Ok(fit_reverb_ir_report(samples,sample_rate)?.parameters)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn noise(state:&mut u64)->f64 {
        *state=state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*state>>32) as u32 as f64/u32::MAX as f64)*2.-1.
    }
    fn recorded_tail(fs:f64,rt:f64,seconds:f64,noise_amplitude:f64)->Vec<f32> {
        let (mut seed,mut floor)=(17_u64,29_u64);
        (0..(fs*seconds) as usize).map(|i|{
            (0.6*noise(&mut seed)*10_f64.powf(-3.*i as f64/fs/rt)+noise_amplitude*noise(&mut floor)) as f32
        }).collect()
    }
    #[test]
    fn ir_measurement_rejects_tail_noise_without_biasing_rt60() {
        let fs=48000.;let rt=1.2;let mut ir=vec![0.;960];
        ir.extend(recorded_tail(fs,rt,3.5,0.004));
        let report=fit_reverb_ir_report(&ir,fs).unwrap();
        eprintln!("Noisy IR: RT {:.4}s, r2 {:.6}, span {:.2} dB, noise {:.2} dBFS, model {:.3}%",report.decay_seconds,report.decay_r_squared,report.decay_span_db,report.noise_floor_dbfs,report.decay_model_rms_percent);
        assert!((report.decay_seconds/rt-1.).abs()<0.08,"{report:?}");
        assert!((report.predelay_ms-20.).abs()<0.3);
        assert!(report.decay_r_squared>0.97);
        assert!(report.decay_span_db>20.);
        let noise_db=10.*(0.004_f64.powi(2)/3.).log10();
        assert!((report.noise_floor_dbfs-noise_db).abs()<1.5);
    }
    #[test]
    fn white_ir_has_no_artificial_octave_bandwidth_tilt() {
        let fs=48000.;let report=fit_reverb_ir_report(&recorded_tail(fs,0.9,2.4,0.),fs).unwrap();
        let levels:Vec<_>=report.bands.iter().filter(|b|b.frequency>=250.).map(|b|b.relative_level_db).collect();
        eprintln!("White IR bandwidth-normalized levels: {levels:?}");
        assert!(levels.iter().all(|g|g.abs()<1.5),"{levels:?}");
        let mut values=super::super::super::table_defaults(REVERB_SPECS);
        for (id,v) in &report.parameters {values[REVERB_SPECS.iter().position(|s|s.id==id.as_str()).unwrap()]=*v;}
        for f in [250.,500.,1000.,2000.,4000.,8000.] {
            assert!(reverb_post_response_db(&values,f,fs).abs()<2.,"{f}: {}",reverb_post_response_db(&values,f,fs));
        }
    }
    #[test]
    fn ir_fitted_reverb_has_the_measured_decay_in_rendered_pcm() {
        let fs=12000.;let target_rt=0.85;
        let report=fit_reverb_ir_report(&recorded_tail(fs,target_rt,2.5,0.),fs).unwrap();
        let mut processor=ReverbProcessor::new(fs);
        for (id,v) in &report.parameters {processor.set(id,*v).unwrap();}
        processor.set("mix",100.).unwrap();
        let n=(fs*3.) as usize;let mut input=vec![0.;n];input[0]=0.2;
        let (mut left,mut right)=(vec![0.;n],vec![0.;n]);
        processor.process(&input,&input,None,&mut left,&mut right);
        let actual=measure_decay(&left,fs).expect("rendered FDN must have a measurable tail");
        eprintln!("Rendered fitted FDN: target {target_rt}s, measured {}s, r2 {}",actual.rt60,actual.r_squared);
        assert!((actual.rt60/target_rt-1.).abs()<0.22);
        assert!(report.decay_model_rms_percent<15.,"{report:?}");
        assert!(left.iter().all(|x|x.is_finite()&&x.abs()<1.));
    }
    #[test]
    fn ir_early_reflection_density_distinguishes_sparse_arrivals() {
        let fs=24000.;let diffuse=recorded_tail(fs,1.1,2.8,0.);
        let mut sparse=diffuse.clone();
        sparse[..(fs*0.08) as usize].fill(0.);
        for (ms,amplitude) in [(0.,0.8),(9.,0.7),(23.,0.6),(41.,0.55),(67.,0.5)] {sparse[(ms*0.001*fs) as usize]=amplitude;}
        let a=fit_reverb_ir_report(&diffuse,fs).unwrap();
        let b=fit_reverb_ir_report(&sparse,fs).unwrap();
        let parameter=|r:&ReverbIrFitReport,key:&str|r.parameters.iter().find(|(k,_)|k==key).unwrap().1;
        assert!(parameter(&a,"diffusion")>parameter(&b,"diffusion")+50.);
        assert!((0. ..=100.).contains(&b.early_energy_percent));
    }
}
