//! Six-band dynamics with complementary, zero-latency crossovers.
//!
//! Every split stores its low band and passes `input - low` to the next
//! split. With zero range and gain the bands sum to the original sample,
//! including transients. Detector filters have separate state, so an
//! external key never enters the audible signal. This is an independent
//! implementation, not a model of a proprietary multiband processor.

use super::filters::{db_to_lin, lin_to_db, one_pole_coefficient, time_coefficient, Biquad, DelayLine, Rbj, rbj, response_db, Coefficients, IDENTITY};
use super::spectral::{Complex,PhaseFir};
use super::quality::Oversampler;
use super::{Spec, TableEffect};

pub const MULTIBAND_BANDS: usize = 6;
pub const MULTIBAND_MODES: [&str; 2] = ["压缩", "扩展"];
const GLOBALS: usize = 9;
const STRIDE: usize = 10;
pub const MULTIBAND_EXPERT_START: usize = 71;
pub const MULTIBAND_EXPERT_STRIDE: usize = 11;
pub const MULTIBAND_PHASES: [&str;3] = ["动态相位","最小相位","线性相位"];

macro_rules! band_specs {
    ($($n:literal),+) => { &[
        Spec::new("band_count", "频段数", 1.0, 6.0, 6.0, "").steps(5),
        Spec::new("xover1", "分频 1", 20.0, 200.0, 120.0, "Hz").log(),
        Spec::new("xover2", "分频 2", 201.0, 800.0, 500.0, "Hz").log(),
        Spec::new("xover3", "分频 3", 801.0, 3000.0, 2000.0, "Hz").log(),
        Spec::new("xover4", "分频 4", 3001.0, 8000.0, 6000.0, "Hz").log(),
        Spec::new("xover5", "分频 5", 8001.0, 20000.0, 12000.0, "Hz").log(),
        Spec::new("mix", "干湿比", 0.0, 100.0, 100.0, "%"),
        Spec::new("output", "输出", -24.0, 24.0, 0.0, "dB"),
        Spec::new("sc_listen", "监听侧链", 0.0, 1.0, 0.0, "").steps(1),
        $(
            Spec::new(concat!("b", stringify!($n), ".enabled"), concat!("频段 ", stringify!($n), " 启用"), 0.0, 1.0, 1.0, "").steps(1),
            Spec::new(concat!("b", stringify!($n), ".threshold"), "阈值", -72.0, 0.0, -24.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".range"), "动态范围", -48.0, 24.0, 0.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".ratio"), "比率", 1.0, 20.0, 3.0, ":1").log(),
            Spec::new(concat!("b", stringify!($n), ".attack"), "启动", 0.1, 200.0, 10.0, "ms").log(),
            Spec::new(concat!("b", stringify!($n), ".release"), "释放", 5.0, 2000.0, 120.0, "ms").log(),
            Spec::new(concat!("b", stringify!($n), ".knee"), "拐点", 0.0, 24.0, 6.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".gain"), "频段增益", -24.0, 24.0, 0.0, "dB"),
            Spec::new(concat!("b", stringify!($n), ".solo"), "独奏", 0.0, 1.0, 0.0, "").steps(1),
            Spec::new(concat!("b", stringify!($n), ".mode"), "模式", 0.0, 1.0, 0.0, "").steps(1),
        )+
        Spec::new("phase","相位模式",0.0,2.0,0.0,"").steps(2).fixed(),
        Spec::new("layout","频段布局",0.0,1.0,0.0,"").steps(1),
        $(
            Spec::new(concat!("b",stringify!($n),".low"),"频段下限",20.0,20000.0,100.0,"Hz").log(),
            Spec::new(concat!("b",stringify!($n),".high"),"频段上限",20.0,20000.0,4000.0,"Hz").log(),
            Spec::new(concat!("b",stringify!($n),".slope"),"频段斜率",6.0,48.0,24.0,"dB/oct").steps(7),
            Spec::new(concat!("b",stringify!($n),".sc_free"),"自由侧链频带",0.0,1.0,0.0,"").steps(1),
            Spec::new(concat!("b",stringify!($n),".sc_low"),"侧链下限",20.0,20000.0,20.0,"Hz").log(),
            Spec::new(concat!("b",stringify!($n),".sc_high"),"侧链上限",20.0,20000.0,20000.0,"Hz").log(),
            Spec::new(concat!("b",stringify!($n),".sc_source"),"侧链来源",0.0,2.0,0.0,"").steps(2),
            Spec::new(concat!("b",stringify!($n),".link"),"立体声联动",0.0,100.0,100.0,"%"),
            Spec::new(concat!("b",stringify!($n),".place"),"处理声道",0.0,4.0,0.0,"").steps(4),
            Spec::new(concat!("b",stringify!($n),".mute"),"静音频段",0.0,1.0,0.0,"").steps(1),
            Spec::new(concat!("b",stringify!($n),".lookahead"),"频段前视",0.0,20.0,0.0,"ms").fixed(),
        )+
        Spec::new("oversampling","增益级过采样",0.0,2.0,0.0,"").steps(2).fixed(),
    ]};
}

pub static MULTIBAND_SPECS: &[Spec] = band_specs!(1, 2, 3, 4, 5, 6);

/// Effective crossover frequencies; used by both DSP and the native graph.
/// Their non-overlapping parameter ranges preserve their displayed order.
pub fn multiband_crossovers(values: &[f64], sample_rate: f64) -> [f64; 5] {
    std::array::from_fn(|i| values.get(i + 1).copied().unwrap_or(MULTIBAND_SPECS[i + 1].default).min(sample_rate * 0.45))
}

/// Effective free or adjacent band bounds; inverted free bounds are sorted.
pub fn multiband_band_range(v:&[f64],band:usize,fs:f64)->(f64,f64) {
    if v.get(70).copied().unwrap_or(0.0)>=0.5 {
        let o=MULTIBAND_EXPERT_START+band*MULTIBAND_EXPERT_STRIDE;
        let a=v[o].min(v[o+1]).min(fs*0.49);let b=v[o].max(v[o+1]).min(fs*0.49);
        (a.max(1.0),b.max(a+0.01))
    } else {
        let cross=multiband_crossovers(v,fs);let count=v[0].round().clamp(1.0,6.0) as usize;
        (if band==0{0.0}else{cross[band-1]},if band+1>=count{fs*0.5}else{cross[band]})
    }
}
fn range_coefficients(low:f64,high:f64,slope:f64,fs:f64)->[Coefficients;8] {
    let order=(slope/6.0).round().clamp(1.0,8.0) as usize;
    let mut result=[IDENTITY;8];
    for (edge,kind) in [(low,Rbj::HighPass),(high,Rbj::LowPass)] {
        let offset=if matches!(kind,Rbj::HighPass){0}else{4};
        if edge<=1.0||edge>=fs*0.499 {continue;}
        let first=order%2;
        if first==1 {
            let a=(std::f64::consts::PI*edge/fs).tan();let d=1.0+a;
            result[offset]=if matches!(kind,Rbj::HighPass){[1.0/d,-1.0/d,0.0,(a-1.0)/d,0.0]}else{[a/d,a/d,0.0,(a-1.0)/d,0.0]};
        }
        for i in 0..order/2 {
            let q=1.0/(2.0*(std::f64::consts::PI*(2*i+1) as f64/(2*order) as f64).sin());
            result[offset+first+i]=rbj(kind,edge,q,0.0,fs);
        }
    }
    result
}
/// Sample the cascade using one shared sin/cos pair and a magnitude product.
/// This is algebraically the same response as summing per-section dB, without
/// repeating eight trigonometric evaluations and logarithm/exponential pairs.
fn range_gain(coefficients:&[Coefficients;8],frequency:f64,fs:f64)->f64{
    let w=std::f64::consts::TAU*frequency.clamp(1.0,fs*0.5)/fs;
    let (sin,cos)=w.sin_cos();let cos2=2.0*cos*cos-1.0;let sin2=2.0*sin*cos;
    let mut gain=1.0;
    for &[b0,b1,b2,a1,a2] in coefficients{
        if [b0,b1,b2,a1,a2]==IDENTITY{continue;}
        let nr=b0+b1*cos+b2*cos2;let ni=b1*sin+b2*sin2;
        let dr=1.0+a1*cos+a2*cos2;let di=a1*sin+a2*sin2;
        gain*=((nr*nr+ni*ni).sqrt()/(dr*dr+di*di).sqrt().max(1e-12)).max(1e-12);
    }
    gain
}
fn run_range(filters:&mut [Biquad;8],mut x:f64)->f64 {for f in filters{x=f.run(x);}x}
fn place_pair(x:[f64;2],place:usize)->[f64;2] {
    match place {1=>[(x[0]+x[1])*0.5;2],2=>[(x[0]-x[1])*0.5;2],3=>[x[0];2],4=>[x[1];2],_=>x}
}
fn place_delta(x:[f64;2],place:usize)->[f64;2] {
    match place {1=>[x[0];2],2=>[x[0],-x[0]],3=>[x[0],0.0],4=>[0.0,x[1]],_=>x}
}

#[derive(Clone, Copy, Default)]
struct Splitter { memory: [[f64; 2]; 5] }

impl Splitter {
    #[inline]
    fn run(&mut self, input: [f64; 2], coefficients: &[f64; 5], count: usize) -> [[f64; 2]; 6] {
        let mut bands = [[0.0; 2]; 6];
        let mut high = input;
        for band in 0..count - 1 {
            let a = coefficients[band];
            for channel in 0..2 {
                let low = high[channel] + a * (self.memory[band][channel] - high[channel]);
                self.memory[band][channel] = low;
                bands[band][channel] = low;
                high[channel] -= low;
            }
        }
        bands[count - 1] = high;
        bands
    }
}

fn complex_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Exact small-signal magnitude of the same complementary splitter used by
/// the processor. This is a band response, not a fabricated live analyzer.
pub fn multiband_band_response_db(values: &[f64], band: usize, freq: f64, sample_rate: f64) -> f64 {
    let count = values.first().copied().unwrap_or(6.0).round().clamp(1.0, 6.0) as usize;
    if band >= count { return -180.0; }
    if values.get(70).copied().unwrap_or(0.0)>=0.5 || values.get(69).copied().unwrap_or(0.0)>=1.5 {
        let (low,high)=multiband_band_range(values,band,sample_rate);
        let o=MULTIBAND_EXPERT_START+band*MULTIBAND_EXPERT_STRIDE;
        return range_coefficients(low,high,values[o+2],sample_rate).iter().map(|&c|response_db(c,freq,sample_rate)).sum();
    }
    let w = std::f64::consts::TAU * freq.clamp(0.0, sample_rate * 0.5) / sample_rate;
    let mut h = (1.0, 0.0);
    for (index, crossover) in multiband_crossovers(values, sample_rate).into_iter().enumerate().take(count - 1) {
        let a = one_pole_coefficient(crossover, sample_rate);
        let (re, im) = (1.0 - a * w.cos(), a * w.sin());
        let denominator = re * re + im * im;
        let low = ((1.0 - a) * re / denominator, -(1.0 - a) * im / denominator);
        if index == band {
            h = complex_mul(h, low);
            break;
        }
        h = complex_mul(h, (1.0 - low.0, -low.1));
    }
    lin_to_db(h.0.hypot(h.1))
}

fn soft_positive(x: f64, knee: f64) -> f64 {
    if x <= -knee * 0.5 { 0.0 }
    else if knee > 0.0 && x < knee * 0.5 { (x + knee * 0.5).powi(2) / (2.0 * knee) }
    else { x }
}

/// Static signed gain change. Negative range selects downward action and
/// positive range selects upward action; expansion reverses the trigger side.
pub fn multiband_gain_db(level: f64, threshold: f64, range: f64, ratio: f64, knee: f64, expand: bool) -> f64 {
    let amount = if expand { ratio.max(1.0) - 1.0 } else { 1.0 - 1.0 / ratio.max(1.0) };
    let distance = if (range < 0.0) != expand { level - threshold } else { threshold - level };
    let gain = soft_positive(distance, knee.max(0.0)) * amount;
    if range < 0.0 { -gain.min(-range) } else { gain.min(range) }
}

pub struct MultibandProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    split: Splitter,
    detector_split: Splitter,
    coefficients: [f64; 5],
    attack: [f64; 6],
    release: [f64; 6],
    gain_db: [f64; 6],
    detector_coefficient: f64,
    free_audio: [[[Biquad;8];2];6],
    free_key: [[[Biquad;8];2];6],
    phase_filters: [[Biquad;5];2],
    fir: [PhaseFir;6],
    dry: [DelayLine;2],
    key_delay: [[DelayLine;2];6],
    audio_delay: [[DelayLine;2];6],
    detector_stereo: [[f64;2];6],
    gains_stereo: [[f64;2];6],
    max_lookahead:usize,
    band_lookahead:[usize;6],
    phase_latency:usize,
    fir_signature:[f64;32],
    processing_started:bool,
    fir_clock:usize,
    fir_round_robin:usize,
    fir_dirty:[bool;6],
    fir_requested:[[Coefficients;8];6],
    fir_target:[[Coefficients;8];6],
    oversampling:[[Oversampler;2];6],

}

impl MultibandProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let mut processor = Self { values: super::table_defaults(MULTIBAND_SPECS), sample_rate,
            split: Splitter::default(), detector_split: Splitter::default(), coefficients: [0.0; 5],
            attack: [0.0; 6], release: [0.0; 6], gain_db: [0.0; 6], detector_coefficient: 0.0,
            free_audio:Default::default(),free_key:Default::default(),phase_filters:Default::default(),
            fir:std::array::from_fn(|_|PhaseFir::new()),
            dry:std::array::from_fn(|_|DelayLine::new((sample_rate*0.025) as usize+2048)),
            key_delay:std::array::from_fn(|_|std::array::from_fn(|_|DelayLine::new((sample_rate*0.025) as usize+2048))),
            audio_delay:std::array::from_fn(|_|std::array::from_fn(|_|DelayLine::new((sample_rate*0.025) as usize+2048))),
            detector_stereo:[[0.0;2];6],gains_stereo:[[0.0;2];6],max_lookahead:0,band_lookahead:[0;6],phase_latency:0,
            fir_signature:[f64::NAN;32],processing_started:false,fir_clock:0,fir_round_robin:0,fir_dirty:[false;6],
            fir_requested:[[IDENTITY;8];6],fir_target:[[IDENTITY;8];6],oversampling:std::array::from_fn(|_|std::array::from_fn(|_|Oversampler::new())) };
        processor.refresh();
        processor
    }
    pub fn band_gain_db(&self) -> [f64; 6] { self.gain_db }
    /// All six designers share one fixed-clock work budget. A changing target
    /// cannot restart an unfinished snapshot; the latest request follows it.
    fn update_firs(&mut self){
        for fir in &mut self.fir{fir.commit_ready();}
        if self.fir_clock%32!=0{return;}
        for offset in 0..6{
            let band=(self.fir_round_robin+offset)%6;
            if self.fir[band].design_idle(){
                if !self.fir_dirty[band]{continue;}
                self.fir_target[band]=self.fir_requested[band];
                self.fir[band].begin_design(0);self.fir_dirty[band]=false;
            }else if !self.fir[band].needs_design_step(){continue;}
            let coefficients=self.fir_target[band];let fs=self.sample_rate;
            self.fir[band].design_step(fs,512,|frequency|{
                let gain=range_gain(&coefficients,frequency,fs);
                [Complex::new(gain,0.0),Complex::default(),Complex::default(),Complex::new(gain,0.0)]
            });
            self.fir[band].commit_ready();self.fir_round_robin=(band+1)%6;break;
        }
    }
}

impl TableEffect for MultibandProcessor {
    const SPECS: &'static [Spec] = MULTIBAND_SPECS;
    fn gain_reduction_db(&self)->Option<f64>{
        let count=self.values[0].round().clamp(1.0,6.0)as usize;
        Some(self.gain_db[..count].iter().copied().fold(0.0,f64::min))
    }
    fn begin_audio(&mut self){self.processing_started=true;}
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let previous_requested=self.fir_requested;
        for band in &mut self.oversampling {for channel in band {channel.set_quality(self.values[137].round() as usize);}}
        self.coefficients = multiband_crossovers(&self.values, self.sample_rate).map(|freq| one_pole_coefficient(freq, self.sample_rate));
        self.detector_coefficient = time_coefficient(2.0, self.sample_rate);
        for band in 0..6 {
            let offset = GLOBALS + band * STRIDE;
            self.attack[band] = time_coefficient(self.values[offset + 4], self.sample_rate);
            self.release[band] = time_coefficient(self.values[offset + 5], self.sample_rate);
            let e=MULTIBAND_EXPERT_START+band*MULTIBAND_EXPERT_STRIDE;
            self.band_lookahead[band]=(self.values[e+10]*0.001*self.sample_rate).round() as usize;
            let (low,high)=multiband_band_range(&self.values,band,self.sample_rate);
            let coefficients=range_coefficients(low,high,self.values[e+2],self.sample_rate);
            self.fir_requested[band]=coefficients;
            for c in 0..2 {for k in 0..8 {self.free_audio[band][c][k].set(coefficients[k]);}}
            let (kl,kh)=if self.values[e+3]>=0.5 {(self.values[e+4].min(self.values[e+5]),self.values[e+4].max(self.values[e+5]))}else{(low,high)};
            let key_coefficients=range_coefficients(kl,kh,self.values[e+2],self.sample_rate);
            for c in 0..2 {for k in 0..8 {self.free_key[band][c][k].set(key_coefficients[k]);}}
        }
        self.max_lookahead=self.band_lookahead.iter().copied().max().unwrap_or(0);
        self.phase_latency=if self.values[69]>=1.5 {384}else{0};
        for (i,f) in multiband_crossovers(&self.values,self.sample_rate).into_iter().enumerate() {
            let c=rbj(Rbj::LowPass,f,std::f64::consts::FRAC_1_SQRT_2,0.0,self.sample_rate);
            for ch in &mut self.phase_filters {ch[i].set([c[4],c[3],1.0,c[3],c[4]]);}
        }
        let signature=std::array::from_fn(|i|match i {0=>self.values[0],1=>self.values[69],2=>self.values[70],3..=7=>self.values[i-2],8..=25=>{let j=i-8;self.values[MULTIBAND_EXPERT_START+j/3*MULTIBAND_EXPERT_STRIDE+j%3]},_=>0.0});
        if signature!=self.fir_signature{
            if self.phase_latency>0{
                if !self.processing_started||signature[1]!=self.fir_signature[1]{
                    // Static setup and a fixed phase/latency switch publish all
                    // kernels synchronously before the next audio block.
                    for band in 0..6{
                        let coefficients=self.fir_requested[band];let fs=self.sample_rate;
                        self.fir[band].design(fs,0,|f|{let gain=range_gain(&coefficients,f,fs);[gain,0.0,0.0,gain]});
                        self.fir_dirty[band]=false;
                    }
                }else{
                    for band in 0..6{
                        if previous_requested[band]!=self.fir_requested[band]{self.fir_dirty[band]=true;}
                    }
                }
            }else{
                for fir in &mut self.fir{fir.cancel_design();}
                self.fir_dirty=[false;6];
            }
            self.fir_signature=signature;
        }
    }
    fn latency(&self)->u32 {(self.max_lookahead+self.phase_latency+self.oversampling[0][0].latency()) as u32}
    fn set_sample_rate(&mut self,sample_rate:f64) {
        let values=self.values.clone();*self=Self::new(sample_rate);self.values=values;self.refresh();
    }
    fn process(&mut self,in_l:&[f32],in_r:&[f32],key:Option<(&[f32],&[f32])>,out_l:&mut[f32],out_r:&mut[f32]) {
        let count=self.values[0].round().clamp(1.0,6.0) as usize;
        let wet=self.values[6]*0.01;let output=db_to_lin(self.values[7]);let listen=self.values[8]>=0.5;
        let free=self.values[70]>=0.5;let phase=self.values[69].round() as usize;
        let any_solo=(0..count).any(|b|self.values[GLOBALS+b*STRIDE+8]>=0.5);
        if !in_l.is_empty(){self.begin_audio();}
        for i in 0..in_l.len() {
            if phase==2{self.update_firs();}
            let input=[in_l[i] as f64,in_r[i] as f64];let external=key.map_or([0.0;2],|(l,r)|[l[i] as f64,r[i] as f64]);
            let sidechain=if key.is_some(){external}else{input};
            let mut bands=self.split.run(input,&self.coefficients,count);
            // Keep every linear band's history on the same frame clock, even
            // while its band-count slot is hidden or bypassed.
            if phase==2{for (band,fir) in self.fir.iter_mut().enumerate(){bands[band]=fir.run(input);}}
            let keys=self.detector_split.run(sidechain,&self.coefficients,count);
            let mut dry=[0.0;2];
            for c in 0..2 {self.dry[c].push(input[c]);dry[c]=self.dry[c].tap(self.max_lookahead+self.phase_latency+self.oversampling[0][0].latency());}
            let mut result=if listen||any_solo{[0.0;2]}else{dry};
            for b in 0..count {
                let e=MULTIBAND_EXPERT_START+b*MULTIBAND_EXPERT_STRIDE;
                let v=&self.values[GLOBALS+b*STRIDE..][..STRIDE];
                let place=self.values[e+8].round() as usize;
                if phase!=2&&free {bands[b]=[run_range(&mut self.free_audio[b][0],input[0]),run_range(&mut self.free_audio[b][1],input[1])];}
                // Audio and detector paths have separate history; external keys
                // cannot leak into normal output, including overlapping bands.
                let source=match self.values[e+6].round() as usize{1=>input,2=>external,_=>sidechain};
                let custom=free||self.values[e+3]>=0.5||self.values[e+6]>=0.5||phase==2;
                let detector=place_pair(if custom {[run_range(&mut self.free_key[b][0],source[0]),run_range(&mut self.free_key[b][1],source[1])]}else{keys[b]},place);
                let mut delayed_key=[0.0;2];
                for c in 0..2 {
                    self.key_delay[b][c].push(detector[c]);
                    delayed_key[c]=self.key_delay[b][c].tap(self.phase_latency+self.max_lookahead-self.band_lookahead[b]);
                }
                let link=self.values[e+7]*0.01;
                let maximum=delayed_key[0].abs().max(delayed_key[1].abs());
                for c in 0..2 {self.audio_delay[b][c].push(bands[b][c]);bands[b][c]=self.audio_delay[b][c].tap(self.max_lookahead);}
                let selected=place_pair(bands[b],place);
                let mut delta=[0.0;2];
                for c in 0..2 {
                    let level=(delayed_key[c].abs()*(1.0-link)+maximum*link).powi(2);
                    self.detector_stereo[b][c]=level+self.detector_coefficient*(self.detector_stereo[b][c]-level);
                    let target=if v[0]>=0.5{multiband_gain_db(lin_to_db(self.detector_stereo[b][c].sqrt()),v[1],v[2],v[3],v[6],v[9]>=0.5)}else{0.0};
                    let coefficient=if target.abs()>self.gains_stereo[b][c].abs(){self.attack[b]}else{self.release[b]};
                    self.gains_stereo[b][c]=target+coefficient*(self.gains_stereo[b][c]-target);
                    let gain=if self.values[e+9]>=0.5{0.0}else if v[0]>=0.5{db_to_lin(self.gains_stereo[b][c]+v[7])}else{1.0};
                    delta[c]=self.oversampling[b][c].run(if listen{delayed_key[c]}else{selected[c]},|sample|if listen{sample}else{sample*if any_solo{gain}else{gain-1.0}});
                }
                self.gain_db[b]=self.gains_stereo[b][0].min(self.gains_stereo[b][1]);
                if !any_solo||v[8]>=0.5 {
                    let delta=place_delta(delta,place);for c in 0..2{result[c]+=delta[c];}
                }
            }
            for c in 0..2 {
                let mut effected=result[c];
                if phase==1&&!listen {for f in self.phase_filters[c].iter_mut().take(count.saturating_sub(1)){effected=f.run(effected);}}
                let x=if listen{effected}else{dry[c]+wet*(effected-dry[c])};
                if c==0{out_l[i]=(x*output) as f32;}else{out_r[i]=(x*output) as f32;}
            }
            self.fir_clock=self.fir_clock.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn run(p: &mut MultibandProcessor, x: &[f32], key: Option<&[f32]>) -> Vec<f32> {
        let mut l = vec![0.0; x.len()];
        let mut r = l.clone();
        p.process(x, x, key.map(|k| (k, k)), &mut l, &mut r);
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        l
    }

    #[test]
    fn neutral_splits_reconstruct_transients_at_each_sample_rate_and_band_count() {
        let x: Vec<f32> = (0..4096).map(|i| ((i * 1973 % 997) as f32 / 498.0) - 1.0).collect();
        for sr in [8_000.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            for count in 1..=6 {
                let mut p = MultibandProcessor::new(sr);
                p.set("band_count", count as f64).unwrap();
                assert_eq!(run(&mut p, &x, None), x, "{sr} Hz / {count} bands");
            }
        }
    }

    #[test]
    fn sidechain_changes_dynamics_but_does_not_leak_into_output() {
        let x = sine(440.0, 0.1, 48_000);
        let loud_key = sine(2000.0, 0.9, x.len());
        let quiet_key = vec![0.0; x.len()];
        let configure = || {
            let mut p = MultibandProcessor::new(FS);
            for (id, value) in [("band_count", 1.0), ("b1.threshold", -30.0), ("b1.range", -36.0), ("b1.ratio", 10.0)] { p.set(id, value).unwrap(); }
            p
        };
        let a = run(&mut configure(), &x, Some(&loud_key));
        let b = run(&mut configure(), &x, Some(&quiet_key));
        assert!(rms_db(&a[24_000..]) < rms_db(&b[24_000..]) - 15.0);
        assert!(run(&mut configure(), &quiet_key, Some(&loud_key)).iter().all(|x| *x == 0.0));
    }

    #[test]
    fn audition_magnitude_agrees_with_display_and_streaming_is_partition_invariant() {
        let mut p = MultibandProcessor::new(FS);
        p.set("b2.solo", 1.0).unwrap();
        let x = sine(400.0, 0.2, 48_000);
        let expected = multiband_band_response_db(&p.values, 1, 400.0, FS);
        let actual = run(&mut p, &x, None);
        assert!((rms_db(&actual[24_000..]) - rms_db(&x[24_000..]) - expected).abs() < 0.02);
        let mut streaming = MultibandProcessor::new(FS);
        streaming.set("b2.solo", 1.0).unwrap();
        let mut assembled = Vec::new();
        for block in x.chunks(137) { assembled.extend(run(&mut streaming, block, None)); }
        assert_eq!(actual, assembled);
    }

    #[test]
    fn signed_ranges_implement_four_dynamics_quadrants_and_reset_clears_history() {
        assert!(multiband_gain_db(-10.0, -20.0, -24.0, 4.0, 0.0, false) < 0.0);
        assert!(multiband_gain_db(-30.0, -20.0, 12.0, 4.0, 0.0, false) > 0.0);
        assert!(multiband_gain_db(-30.0, -20.0, -24.0, 4.0, 0.0, true) < 0.0);
        assert!(multiband_gain_db(-10.0, -20.0, 12.0, 4.0, 0.0, true) > 0.0);
        let mut p = MultibandProcessor::new(FS);
        p.set("b1.gain", -12.0).unwrap();
        run(&mut p, &sine(80.0, 0.5, 4096), None);
        p.set_sample_rate(96_000.0);
        assert!(run(&mut p, &vec![0.0; 1024], None).iter().all(|x| *x == 0.0));
        assert!(p.set("b1.range", f64::NAN).is_err());
    }

    #[test]
    fn free_bands_and_expert_key_ranges_control_actual_audio() {
        let configure=||{
            let mut p=MultibandProcessor::new(FS);
            for(id,v)in[("band_count",1.0),("layout",1.0),("b1.low",800.0),("b1.high",2000.0),("b1.gain",6.0)]{p.set(id,v).unwrap();}
            p
        };
        let input=sine(1200.0,0.1,24000);let out=run(&mut configure(),&input,None);
        assert!(rms_db(&out[12000..])-rms_db(&input[12000..])>5.0);
        let input=sine(80.0,0.1,24000);let out=run(&mut configure(),&input,None);
        assert!((rms_db(&out[12000..])-rms_db(&input[12000..])).abs()<0.1);
        let mut p=configure();
        for(id,v)in[("b1.gain",0.0),("b1.range",-24.0),("b1.threshold",-35.0),("b1.sc_free",1.0),("b1.sc_low",4000.0),("b1.sc_high",8000.0),("b1.sc_source",2.0)]{p.set(id,v).unwrap();}
        let input=sine(1200.0,0.1,24000);let key=sine(6000.0,0.8,24000);let out=run(&mut p,&input,Some(&key));
        assert!(rms_db(&out[12000..])<rms_db(&input[12000..])-8.0);
    }
    #[test]
    fn linear_phase_lookahead_and_oversampling_preserve_delayed_neutral_input() {
        for phase in [0.0,2.0] {
            let mut p=MultibandProcessor::new(FS);
            p.set("phase",phase).unwrap();p.set("b1.lookahead",5.0).unwrap();p.set("oversampling",2.0).unwrap();
            let delay=p.latency() as usize;
            let x:Vec<f32>=(0..4096).map(|i|((i*1973%997) as f32/498.0)-1.0).collect();
            let y=run(&mut p,&x,None);
            assert!(y[..delay].iter().all(|x|*x==0.0));
            assert_eq!(&y[delay..],&x[..x.len()-delay]);
        }
    }
    #[test]
    fn free_stereo_placement_and_external_key_do_not_leak() {
        let mut p=MultibandProcessor::new(FS);
        for(id,v)in[("band_count",1.0),("layout",1.0),("b1.low",20.0),("b1.high",20000.0),("b1.gain",9.0),("b1.place",2.0)]{p.set(id,v).unwrap();}
        let x=sine(1000.0,0.1,4096);assert_eq!(run(&mut p,&x,None),x,"mono input has no side component");
        let silence=vec![0.0;4096];p.set("b1.sc_source",2.0).unwrap();
        assert_eq!(peak(&run(&mut p,&silence,Some(&x))),0.0);
    }

    fn staged_multiband()->MultibandProcessor{
        let mut p=MultibandProcessor::new(FS);
        p.values[69]=2.0;p.values[70]=1.0;
        for b in 0..6{
            let e=MULTIBAND_EXPERT_START+b*MULTIBAND_EXPERT_STRIDE;
            p.values[e]=80.0+b as f64*200.0;p.values[e+1]=1800.0+b as f64*600.0;
            p.values[GLOBALS+b*STRIDE+7]=1.0;
        }
        p.refresh();p
    }
    fn staged_multiband_render(chunks:&[usize])->(Vec<f32>,MultibandProcessor,usize,usize){
        let mut p=staged_multiband();p.begin_audio();
        let initial=p.fir.iter().map(|f|f.designs_started).sum();
        let steps=p.fir.iter().map(|f|f.work_steps).sum();
        let x:Vec<f32>=(0..18000).map(|i|((i*719%997)as f32/997.0-0.5)*0.2).collect();
        let mut l=vec![0.0;x.len()];let mut r=l.clone();let(mut frame,mut chunk)=(0,0);
        while frame<x.len(){
            if frame<3200&&frame%32==0{
                let before:usize=p.fir.iter().map(|f|f.designs_started).sum();
                let tick=(frame/32)as f64;
                for band in 0..6{
                    let e=MULTIBAND_EXPERT_START+band*MULTIBAND_EXPERT_STRIDE;
                    p.set_index(e,100.0+band as f64*200.0+tick).unwrap();
                    if band<2{p.set_index(e+1,2000.0+band as f64*600.0+tick).unwrap();}
                }
                assert_eq!(p.fir.iter().map(|f|f.designs_started).sum::<usize>(),before);
            }
            let mut end=(frame+chunks[chunk%chunks.len()]).min(x.len());
            if frame<3200{end=end.min((frame/32+1)*32);}
            p.process(&x[frame..end],&x[frame..end],None,&mut l[frame..end],&mut r[frame..end]);
            frame=end;chunk+=1;
        }
        assert_eq!(l,r);(l,p,initial,steps)
    }
    #[test]
    fn linear_multiband_shares_bounded_work_and_preserves_partition_identity(){
        let (whole,p,initial,steps)=staged_multiband_render(&[18000]);
        let (split,other,_,_)=staged_multiband_render(&[1,7,113,256,31]);
        assert_eq!(whole,split);assert_eq!(p.fir_clock,other.fir_clock);assert_eq!(p.fir_clock,18000);
        assert!(p.fir_dirty.iter().all(|v|!*v)&&p.fir.iter().all(PhaseFir::design_idle));
        assert_eq!(p.fir_target,p.fir_requested);
        let designs=p.fir.iter().map(|f|f.designs_started).sum::<usize>()-initial;
        assert!((12..=24).contains(&designs),"{designs} snapshots for 800 writes");
        assert!(p.fir.iter().map(|f|f.work_steps).sum::<usize>()-steps<=18000_usize.div_ceil(32));
        assert!(p.fir.iter().all(|f|f.designs_started==f.designs_committed));
        let mut settled=staged_multiband();settled.values=p.values.clone();settled.refresh();
        let x:Vec<f32>=(0..18000).map(|i|((i*719%997)as f32/997.0-0.5)*0.2).collect();
        let expected=run(&mut settled,&x,None);
        assert_eq!(&whole[14000..],&expected[14000..]);assert_eq!(p.latency(),384);
    }
    #[test]
    fn fixed_multiband_phase_cancels_pending_work_and_matches_declared_latency(){
        let mut p=staged_multiband();p.begin_audio();
        p.set("b1.high",2900.0).unwrap();run(&mut p,&[0.1;32],None);
        assert!(p.fir.iter().any(|f|!f.design_idle()));
        p.set("phase",0.0).unwrap();assert_eq!(p.latency(),0);
        assert!(p.fir.iter().all(PhaseFir::design_idle));
        p.set("phase",2.0).unwrap();assert_eq!(p.latency(),384);
        assert!(p.fir.iter().all(|f|f.design_idle()&&f.latency()==384));
        assert!(p.fir_dirty.iter().all(|v|!*v));assert_eq!(p.fir_clock,32);
    }

}
