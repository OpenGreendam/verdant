//! Bounded, offline tonal-response fitting. All candidates are evaluated with
//! the same biquad coefficients as the audio processor; no fitting runs on its
//! real-time thread.
use super::*;

const PROBES: usize = 256;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EqFitReport {
    pub parameters: Vec<(String, f64)>,
    pub bands_used: usize,
    /// Unweighted RMS and worst absolute error on a logarithmic frequency grid.
    pub rms_error_db: f64,
    pub max_error_db: f64,
    /// Broadband reference/source offset removed by EQ Match.
    pub level_offset_db: f64,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct EqMatchOptions {
    pub max_bands: usize,
    pub sample_rate: f64,
    pub min_frequency: f64,
    pub max_frequency: f64,
    /// 0..1. Applied after level matching and smoothing.
    pub amount: f64,
    /// Full width at half maximum of a Gaussian in octaves; zero disables it.
    pub smoothing_octaves: f64,
    pub remove_level_offset: bool,
}
impl Default for EqMatchOptions {
    fn default() -> Self {
        Self { max_bands: EQ_BANDS, sample_rate: 48_000., min_frequency: FREQ_MIN,
            max_frequency: FREQ_MAX, amount: 1., smoothing_octaves: 1./6., remove_level_offset: true }
    }
}

#[derive(Clone, Copy)]
struct Probe { frequency: f64, cosine: [f64; 2], sine: [f64; 2] }
impl Probe {
    fn new(frequency: f64, sample_rate: f64) -> Self {
        let w = std::f64::consts::TAU * frequency / sample_rate;
        Self { frequency, cosine: [w.cos(), (2.*w).cos()], sine: [w.sin(), (2.*w).sin()] }
    }
    fn response(self, c: Coefficients) -> f64 {
        let nr = c[0] + c[1]*self.cosine[0] + c[2]*self.cosine[1];
        let ni = c[1]*self.sine[0] + c[2]*self.sine[1];
        let dr = 1. + c[3]*self.cosine[0] + c[4]*self.cosine[1];
        let di = c[3]*self.sine[0] + c[4]*self.sine[1];
        10.*((nr*nr+ni*ni).max(1e-30)/(dr*dr+di*di).max(1e-30)).log10()
    }
}
fn checked_points(points: &[(f64, f64)]) -> Result<Vec<(f64, f64)>, String> {
    if !(2..=65536).contains(&points.len()) ||
        points.iter().any(|&(f,g)| !f.is_finite() || !g.is_finite() || g.abs()>600. || !(FREQ_MIN..=FREQ_MAX).contains(&f)) {
        return Err("EQ analysis requires 2..65536 finite frequency/level points within 20–20000 Hz".into());
    }
    let mut points = points.to_vec();
    points.sort_by(|a,b| a.0.total_cmp(&b.0));
    // Equal-frequency bins describe one observation, not a zero-width interval.
    let mut unique: Vec<(f64, f64)> = Vec::with_capacity(points.len());
    let mut at = 0;
    while at < points.len() {
        let start = at; let f = points[at].0; let mut sum = 0.;
        while at < points.len() && points[at].0 == f { sum += points[at].1; at += 1; }
        unique.push((f, sum/(at-start) as f64));
    }
    if unique.len() < 2 { return Err("EQ analysis requires distinct frequencies".into()); }
    Ok(unique)
}
fn interpolate(points: &[(f64,f64)], f: f64) -> f64 {
    let at = points.partition_point(|p|p.0 < f);
    if at == 0 { points[0].1 } else if at == points.len() { points[at-1].1 }
    else { let (a,b) = (points[at-1],points[at]); a.1+(b.1-a.1)*(f/a.0).ln()/(b.0/a.0).ln() }
}
fn probes(sample_rate: f64) -> Result<Vec<Probe>, String> {
    if !sample_rate.is_finite() || !(8000. ..=384000.).contains(&sample_rate) {
        return Err("EQ analysis sample rate must be 8–384 kHz".into());
    }
    let upper = FREQ_MAX.min(sample_rate*0.475);
    Ok((0..PROBES).map(|i|Probe::new(FREQ_MIN*(upper/FREQ_MIN).powf(i as f64/(PROBES-1) as f64),sample_rate)).collect())
}
fn curve(band: &Band, probes: &[Probe], fs: f64) -> Vec<f64> {
    let c = band.sections(fs).0[0];
    probes.iter().map(|p|p.response(c)).collect()
}
fn squared_error(response: &[f64], target: &[f64]) -> f64 {
    response.iter().zip(target).map(|(a,b)|(a-b).powi(2)).sum()
}
fn optimize(mut band: Band, target: &[f64], probes: &[Probe], fs: f64, broad: bool) -> (Band, Vec<f64>) {
    let mut response = curve(&band,probes,fs);
    let mut score = squared_error(&response,target);
    let mut steps = if broad { [0.65,0.65,2.] } else { [0.16,0.22,0.6] };
    for pass in 0..12 {
        let mut changed = false;
        for axis in 0..3 {
            let old = band;
            for direction in [-1.,1.] {
                let mut candidate = old;
                match axis {
                    0 => candidate.freq = (old.freq*2_f64.powf(direction*steps[0])).clamp(FREQ_MIN,probes.last().unwrap().frequency),
                    1 => candidate.q = (old.q*2_f64.powf(direction*steps[1])).clamp(if old.kind==BandKind::Bell {0.1}else{0.3},if old.kind==BandKind::Bell {18.}else{1.25}),
                    _ => candidate.gain_db = (old.gain_db+direction*steps[2]).clamp(-GAIN_RANGE,GAIN_RANGE),
                }
                let trial = curve(&candidate,probes,fs);
                let error = squared_error(&trial,target);
                if error + 1e-12 < score { band=candidate; response=trial; score=error; changed=true; }
            }
        }
        if !changed || pass%3==2 { for step in &mut steps { *step *= 0.55; } }
    }
    (band,response)
}
fn refine(bands: &mut [Band], responses: &mut [Vec<f64>], residual: &mut [f64], probes: &[Probe], fs: f64, passes: usize) {
    for _ in 0..passes {
        let before = residual.iter().map(|v|v*v).sum::<f64>();
        for i in 0..bands.len() {
            let target: Vec<_> = residual.iter().zip(&responses[i]).map(|(r,c)|r+c).collect();
            let (band,response) = optimize(bands[i],&target,probes,fs,false);
            for (r,(target,value)) in residual.iter_mut().zip(target.iter().zip(&response)) { *r=target-value; }
            bands[i]=band; responses[i]=response;
        }
        if before-residual.iter().map(|v|v*v).sum::<f64>() < 1e-7 { break; }
    }
}
fn fit(target: &[f64], probes: &[Probe], max_bands: usize, fs: f64) -> EqFitReport {
    let mut residual=target.to_vec(); let mut bands=Vec::new(); let mut responses=Vec::new();
    for _ in 0..max_bands.clamp(1,EQ_BANDS) {
        let peak=(0..residual.len()).max_by(|&a,&b|residual[a].abs().total_cmp(&residual[b].abs())).unwrap();
        let gain=residual[peak].clamp(-GAIN_RANGE,GAIN_RANGE);
        let rms=(residual.iter().map(|v|v*v).sum::<f64>()/residual.len() as f64).sqrt();
        if gain.abs()<0.18 || (rms<0.055 && gain.abs()<0.4) { break; }
        let mut left=peak;let mut right=peak;
        while left>0 && residual[left-1]*gain>gain*gain*0.5 {left-=1;}
        while right+1<residual.len() && residual[right+1]*gain>gain*gain*0.5 {right+=1;}
        let center=probes[peak].frequency;
        let width=(probes[right].frequency-probes[left].frequency).max(center*0.05);
        let seed=Band{kind:BandKind::Bell,on:true,freq:center,gain_db:gain,q:(center/width).clamp(0.1,18.),slope:0,place:Placement::Stereo};
        let mut candidates=vec![seed];
        for (kind,edge,indices) in [
            (BandKind::LowShelf,0,(0..residual.len()).collect::<Vec<_>>()),
            (BandKind::HighShelf,residual.len()-1,(0..residual.len()).rev().collect::<Vec<_>>())
        ] {
            let edge_gain=residual[edge].clamp(-GAIN_RANGE,GAIN_RANGE);
            if edge_gain.abs()<0.2 {continue}
            let half=indices.into_iter().find(|&i| residual[i]*edge_gain<=edge_gain*edge_gain*0.5).unwrap_or(probes.len()/2);
            candidates.push(Band{kind,freq:probes[half].frequency,gain_db:edge_gain,q:0.707,..seed});
        }
        let best=candidates.into_iter().map(|c|optimize(c,&residual,probes,fs,true))
            .min_by(|a,b|squared_error(&a.1,&residual).total_cmp(&squared_error(&b.1,&residual))).unwrap();
        let old=residual.iter().map(|r|r*r).sum::<f64>();
        if old-squared_error(&best.1,&residual)<0.001 {break}
        for (r,c) in residual.iter_mut().zip(&best.1) {*r-=c;}
        bands.push(best.0);responses.push(best.1);
        // Revisit every earlier band: overlapping filters cannot be fitted
        // independently by subtracting a single peak once.
        refine(&mut bands,&mut responses,&mut residual,probes,fs,2);
    }
    refine(&mut bands,&mut responses,&mut residual,probes,fs,8);
    let mut parameters=std::collections::BTreeMap::new();
    for i in 0..EQ_BANDS {
        for field in ["on","slope_continuous","dynamic","spectral","brickwall"] {parameters.insert(eq_param_id(i,field),0.);}
    }
    parameters.insert("solo".into(),0.);parameters.insert("auto_gain".into(),0.);parameters.insert("output".into(),0.);
    for (i,band) in bands.iter().enumerate() {
        for (field,value) in [("on",1.),("shape",band.kind.index() as f64),("freq",band.freq),("gain",band.gain_db),("q",band.q),("place",0.)] {
            parameters.insert(eq_param_id(i,field),value);
        }
    }
    EqFitReport{parameters:parameters.into_iter().collect(),bands_used:bands.len(),
        rms_error_db:(residual.iter().map(|r|r*r).sum::<f64>()/residual.len() as f64).sqrt(),
        max_error_db:residual.iter().map(|r|r.abs()).fold(0.,f64::max),level_offset_db:0.}
}

/// Fit a sketched response with jointly refined bells and shelves. The report
/// describes the static biquad response at the requested processing sample rate.
pub fn eq_fit_curve_report(points:&[(f64,f64)],max_bands:usize,sample_rate:f64)->Result<EqFitReport,String> {
    if points.iter().any(|p|!(-96. ..=96.).contains(&p.1)) {return Err("EQ curve gain must be within −96…96 dB".into())}
    let points=checked_points(points)?;let probes=probes(sample_rate)?;
    let target:Vec<_>=probes.iter().map(|p|interpolate(&points,p.frequency).clamp(-GAIN_RANGE,GAIN_RANGE)).collect();
    Ok(fit(&target,&probes,max_bands,sample_rate))
}
pub fn eq_fit_curve(points:&[(f64,f64)],max_bands:usize)->Result<Vec<(String,f64)>,String> {
    Ok(eq_fit_curve_report(points,max_bands,48000.)?.parameters)
}

/// Match only measured overlapping frequencies, with optional level removal,
/// octave smoothing and strength. Restricted edges taper to zero over 1/4 octave.
pub fn eq_match_spectra_with_options(source:&[(f64,f64)],reference:&[(f64,f64)],options:EqMatchOptions)->Result<EqFitReport,String> {
    if !(1..=EQ_BANDS).contains(&options.max_bands) || !options.min_frequency.is_finite() || !options.max_frequency.is_finite() ||
        !(FREQ_MIN..=FREQ_MAX).contains(&options.min_frequency) ||
        !(FREQ_MIN..=FREQ_MAX).contains(&options.max_frequency) || options.min_frequency>=options.max_frequency ||
        !options.amount.is_finite() || !(0. ..=1.).contains(&options.amount) ||
        !options.smoothing_octaves.is_finite() || !(0. ..=2.).contains(&options.smoothing_octaves) {
        return Err("Match requires 1–24 bands, an ordered 20–20000 Hz range, 0–100% amount and 0–2 octave smoothing".into())
    }
    let source=checked_points(source)?;let reference=checked_points(reference)?;let probes=probes(options.sample_rate)?;
    let low=options.min_frequency.max(source[0].0).max(reference[0].0);
    let high=options.max_frequency.min(source.last().unwrap().0).min(reference.last().unwrap().0).min(probes.last().unwrap().frequency);
    if high/low<1.05 {return Err("Source and reference have no usable overlapping frequency range".into())}
    let raw:Vec<_>=probes.iter().map(|p|(interpolate(&reference,p.frequency)-interpolate(&source,p.frequency)).clamp(-48.,48.)).collect();
    let inside:Vec<_>=probes.iter().enumerate().filter(|(_,p)|p.frequency>=low&&p.frequency<=high).map(|(i,_)|i).collect();
    let offset=if options.remove_level_offset {inside.iter().map(|&i|raw[i]).sum::<f64>()/inside.len().max(1) as f64}else{0.};
    let sigma=options.smoothing_octaves/2.35482;
    let mut target=vec![0.;probes.len()];
    for &i in &inside {
        let value=if sigma<1e-5 {raw[i]} else {
            let (mut sum,mut weights)=(0.,0.);
            for &j in &inside {
                let distance=(probes[j].frequency/probes[i].frequency).log2()/sigma;
                if distance.abs()<=3. {let w=(-0.5*distance*distance).exp();sum+=raw[j]*w;weights+=w;}
            }
            sum/weights.max(1e-12)
        };
        let f=probes[i].frequency;
        let left=if low>FREQ_MIN*1.001 {(f/low).log2().clamp(0.,0.25)/0.25}else{1.};
        let right=if high<probes.last().unwrap().frequency*0.999 {(high/f).log2().clamp(0.,0.25)/0.25}else{1.};
        target[i]=((value-offset)*options.amount).clamp(-GAIN_RANGE,GAIN_RANGE)*left.min(right);
    }
    let mut report=fit(&target,&probes,options.max_bands,options.sample_rate);
    report.level_offset_db=offset;Ok(report)
}
pub fn eq_match_spectra(source:&[(f64,f64)],reference:&[(f64,f64)],max_bands:usize)->Result<Vec<(String,f64)>,String> {
    Ok(eq_match_spectra_with_options(source,reference,EqMatchOptions{max_bands,..Default::default()})?.parameters)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Band> {
        [(BandKind::LowShelf,105.,-4.5,0.707),(BandKind::Bell,330.,-5.5,1.6),
         (BandKind::Bell,1850.,3.8,0.85),(BandKind::HighShelf,7200.,4.2,0.707)]
            .into_iter().map(|(kind,freq,gain_db,q)|Band{kind,on:true,freq,gain_db,q,slope:0,place:Placement::Stereo}).collect()
    }
    fn target_points(bands:&[Band],fs:f64)->Vec<(f64,f64)> {
        (0..513).map(|i|{let f=20.*1000_f64.powf(i as f64/512.);
            (f,bands.iter().map(|b|band_response_db(b,f,fs)).sum())}).collect()
    }
    fn parameters(report:&EqFitReport)->EqParams {
        let mut p=EqParams::default();
        for (id,value) in &report.parameters {p.set(id,*value).unwrap();}
        p
    }
    #[test]
    fn eq_joint_fit_reproduces_shelves_and_overlapping_bells_in_real_pcm() {
        let fs=48000.;let target=fixture();let points=target_points(&target,fs);
        let report=eq_fit_curve_report(&points,6,fs).unwrap();
        eprintln!("EQ joint fit: {} bands RMS {:.5} dB max {:.5} dB",report.bands_used,report.rms_error_db,report.max_error_db);
        assert!(report.rms_error_db<0.35,"{report:?}");
        assert!(report.max_error_db<1.1,"{report:?}");
        let fitted=parameters(&report);
        assert!(fitted.bands.iter().any(|b|b.on&&matches!(b.kind,BandKind::LowShelf|BandKind::HighShelf)));
        let mut errors=Vec::new();
        for freq in [47.,83.,151.,301.,617.,1187.,2377.,4751.,9511.,16001.] {
            let input:Vec<f32>=(0..24000).map(|i|(0.01*(std::f64::consts::TAU*freq*i as f64/fs).sin()) as f32).collect();
            let mut reference=EqProcessor::new(fs);
            for b in &mut reference.params.bands {b.on=false;}
            reference.params.bands[..target.len()].copy_from_slice(&target);reference.refresh();
            let mut processor=EqProcessor::new(fs);processor.params=fitted.clone();processor.refresh();
            let (mut want,mut right,mut got)=(vec![0.;input.len()],vec![0.;input.len()],vec![0.;input.len()]);
            reference.process(&input,&input,&mut want,&mut right);
            processor.process(&input,&input,&mut got,&mut right);
            let power=|x:&[f32]|x[12000..].iter().map(|&v|(v as f64).powi(2)).sum::<f64>();
            errors.push((10.*(power(&got)/power(&want)).log10()).abs());
        }
        eprintln!("EQ held-out PCM errors: {errors:?}");
        assert!(errors.iter().all(|e|*e<0.9));
    }
    #[test]
    fn eq_match_range_amount_and_level_removal_are_measurable() {
        let source:Vec<_>=(0..513).map(|i|(20.*1000_f64.powf(i as f64/512.),-30.)).collect();
        let bell=Band{kind:BandKind::Bell,on:true,freq:1400.,gain_db:8.,q:1.4,slope:0,place:Placement::Stereo};
        let reference:Vec<_>=source.iter().map(|&(f,db)|(f,db+7.+band_response_db(&bell,f,48000.))).collect();
        let options=EqMatchOptions{max_bands:10,min_frequency:200.,max_frequency:6000.,amount:0.5,smoothing_octaves:0.25,..Default::default()};
        let report=eq_match_spectra_with_options(&source,&reference,options).unwrap();
        let p=parameters(&report);
        eprintln!("EQ match: RMS {} dB, removed {} dB, 1400 Hz {}, 40 Hz {}, 16000 Hz {}",report.rms_error_db,report.level_offset_db,p.response_db(1400.,48000.),p.response_db(40.,48000.),p.response_db(16000.,48000.));
        assert!((7. ..10.).contains(&report.level_offset_db));
        assert!(p.response_db(1400.,48000.)>1.5);
        assert!(p.response_db(40.,48000.).abs()<0.5);
        assert!(p.response_db(16000.,48000.).abs()<0.5);
        let none=eq_match_spectra_with_options(&source,&reference,EqMatchOptions{amount:0.,..options}).unwrap();
        assert_eq!(none.bands_used,0);
        assert!(eq_match_spectra_with_options(&source,&reference,EqMatchOptions{min_frequency:7000.,max_frequency:6000.,..options}).is_err());
        assert!(eq_match_spectra_with_options(&[(100.,0.),(100.,1.)],&reference,options).is_err());
    }
    #[test]
    fn eq_fitting_uses_the_processing_sample_rate() {
        for fs in [44100.,96000.] {
            let b=Band{kind:BandKind::HighShelf,on:true,freq:15000.,gain_db:7.,q:0.707,slope:0,place:Placement::Stereo};
            let report=eq_fit_curve_report(&target_points(&[b],fs),2,fs).unwrap();
            assert!(report.rms_error_db<0.12,"{fs}: {report:?}");
            assert!(report.max_error_db<0.4,"{fs}: {report:?}");
        }
    }
}
