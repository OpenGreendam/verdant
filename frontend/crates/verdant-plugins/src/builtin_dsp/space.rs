//! Space: an eight-line feedback-delay-network reverb and a stereo delay.

use super::filters::{db_to_lin, lin_to_db, one_pole_coefficient, response_db, rbj, time_coefficient, Biquad, DelayLine, Rbj, IDENTITY};
use super::{Spec, TableEffect};
use super::quality::Oversampler;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

// ---------------------------------------------------------------------------
// Reverb
// ---------------------------------------------------------------------------

pub const REVERB_STYLES: [&str; 3] = ["现代", "复古", "板式"];

pub static REVERB_SPECS: &[Spec] = &[
    Spec::new("predelay", "预延迟", 0.0, 250.0, 10.0, "ms"),
    Spec::new("decay", "衰减时间", 0.2, 20.0, 2.0, "s").log(),
    Spec::new("size", "空间大小", 10.0, 100.0, 60.0, "%"),
    Spec::new("damping", "高频阻尼", 1000.0, 20000.0, 8000.0, "Hz").log(),
    Spec::new("diffusion", "扩散", 0.0, 100.0, 75.0, "%"),
    Spec::new("width", "宽度", 0.0, 150.0, 100.0, "%"),
    Spec::new("low_cut", "低切", 20.0, 1000.0, 120.0, "Hz").log(),
    Spec::new("high_cut", "高切", 1000.0, 20000.0, 12000.0, "Hz").log(),
    Spec::new("mix", "干湿比", 0.0, 100.0, 25.0, "%"),
    Spec::new("modulation", "调制", 0.0, 100.0, 30.0, "%"),
    Spec::new("style", "风格", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("decay_rate", "衰减比例", 25.0, 400.0, 100.0, "%"),
    Spec::new("freeze", "冻结", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("ducking", "闪避", 0.0, 48.0, 0.0, "dB"),
    Spec::new("distance", "距离", 0.0, 100.0, 0.0, "%"),
    Spec::new("thickness", "厚度", -100.0, 100.0, 0.0, "%"),
    Spec::new("auto_gate", "自动门限", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("gate_time", "门限保持", 0.0, 2000.0, 250.0, "ms"),
    Spec::new("decay_low_rate", "低频衰减比例", 25.0, 400.0, 100.0, "%"),
    Spec::new("decay_high_rate", "高频衰减比例", 25.0, 400.0, 100.0, "%"),
    Spec::new("decay_low_freq", "低频衰减频率", 20.0, 2000.0, 250.0, "Hz").log(),
    Spec::new("decay_high_freq", "高频衰减频率", 500.0, 20000.0, 6000.0, "Hz").log(),
    Spec::new("post_low_gain", "输出低架", -24.0, 24.0, 0.0, "dB"),
    Spec::new("post_high_gain", "输出高架", -24.0, 24.0, 0.0, "dB"),
    Spec::new("post_mid_freq", "输出峰值频率", 20.0, 20000.0, 1000.0, "Hz").log(),
    Spec::new("post_mid_gain", "输出峰值增益", -24.0, 24.0, 0.0, "dB"),
    Spec::new("post_mid_q", "输出峰值 Q", 0.1, 18.0, 1.0, "").log(),
    Spec::new("decay1_on","衰减节点 1 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay1_freq","衰减节点 1 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay1_rate","衰减节点 1 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay1_q","衰减节点 1 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay1_shape","衰减节点 1 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay2_on","衰减节点 2 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay2_freq","衰减节点 2 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay2_rate","衰减节点 2 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay2_q","衰减节点 2 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay2_shape","衰减节点 2 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay3_on","衰减节点 3 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay3_freq","衰减节点 3 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay3_rate","衰减节点 3 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay3_q","衰减节点 3 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay3_shape","衰减节点 3 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay4_on","衰减节点 4 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay4_freq","衰减节点 4 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay4_rate","衰减节点 4 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay4_q","衰减节点 4 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay4_shape","衰减节点 4 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay5_on","衰减节点 5 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay5_freq","衰减节点 5 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay5_rate","衰减节点 5 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay5_q","衰减节点 5 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay5_shape","衰减节点 5 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay6_on","衰减节点 6 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay6_freq","衰减节点 6 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay6_rate","衰减节点 6 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay6_q","衰减节点 6 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay6_shape","衰减节点 6 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay7_on","衰减节点 7 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay7_freq","衰减节点 7 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay7_rate","衰减节点 7 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay7_q","衰减节点 7 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay7_shape","衰减节点 7 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("decay8_on","衰减节点 8 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("decay8_freq","衰减节点 8 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("decay8_rate","衰减节点 8 衰减比例",25.0,400.0,100.0,"%"),
    Spec::new("decay8_q","衰减节点 8 Q",0.1,12.0,1.0,"").log(),
    Spec::new("decay8_shape","衰减节点 8 形状",0.0,2.0,0.0,"").steps(2),
    Spec::new("post1_on","输出 EQ 节点 1 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post1_freq","输出 EQ 节点 1 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post1_gain","输出 EQ 节点 1 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post1_q","输出 EQ 节点 1 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post1_shape","输出 EQ 节点 1 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post2_on","输出 EQ 节点 2 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post2_freq","输出 EQ 节点 2 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post2_gain","输出 EQ 节点 2 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post2_q","输出 EQ 节点 2 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post2_shape","输出 EQ 节点 2 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post3_on","输出 EQ 节点 3 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post3_freq","输出 EQ 节点 3 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post3_gain","输出 EQ 节点 3 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post3_q","输出 EQ 节点 3 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post3_shape","输出 EQ 节点 3 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post4_on","输出 EQ 节点 4 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post4_freq","输出 EQ 节点 4 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post4_gain","输出 EQ 节点 4 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post4_q","输出 EQ 节点 4 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post4_shape","输出 EQ 节点 4 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post5_on","输出 EQ 节点 5 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post5_freq","输出 EQ 节点 5 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post5_gain","输出 EQ 节点 5 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post5_q","输出 EQ 节点 5 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post5_shape","输出 EQ 节点 5 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post6_on","输出 EQ 节点 6 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post6_freq","输出 EQ 节点 6 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post6_gain","输出 EQ 节点 6 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post6_q","输出 EQ 节点 6 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post6_shape","输出 EQ 节点 6 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post7_on","输出 EQ 节点 7 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post7_freq","输出 EQ 节点 7 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post7_gain","输出 EQ 节点 7 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post7_q","输出 EQ 节点 7 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post7_shape","输出 EQ 节点 7 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("post8_on","输出 EQ 节点 8 启用",0.0,1.0,0.0,"").steps(1),
    Spec::new("post8_freq","输出 EQ 节点 8 频率",20.0,20000.0,1000.0,"Hz").log(),
    Spec::new("post8_gain","输出 EQ 节点 8 增益",-24.0,24.0,0.0,"dB"),
    Spec::new("post8_q","输出 EQ 节点 8 Q",0.1,18.0,1.0,"").log(),
    Spec::new("post8_shape","输出 EQ 节点 8 形状",0.0,4.0,0.0,"").steps(4),
    Spec::new("room","房间模型",0.0,4.0,0.0,""),
    Spec::new("predelay_sync","预延迟节拍",0.0,8.0,0.0,"").steps(8),
    Spec::new("tempo","参考速度",20.0,300.0,120.0,"BPM"),
    Spec::new("gate_sync","门限节拍",0.0,8.0,0.0,"").steps(8),
    Spec::new("gate_threshold","门限阈值",-100.0,-12.0,-60.0,"dB"),
    Spec::new("gate_release","门限释放",0.1,2000.0,5.0,"ms").log(),
    Spec::new("early_level","早期反射",0.0,100.0,0.0,"%"),
    Spec::new("position","声源位置",-100.0,100.0,0.0,"%"),
];

pub const REVERB_NODES:usize=8;
pub const REVERB_DECAY_START:usize=27;
pub const REVERB_POST_START:usize=67;
const REVERB_ADVANCED_START:usize=107;
const LINES: usize = 8;
/// Base line lengths in milliseconds at 100 % size (mutually prime-ish).
const LINE_MS: [f64; LINES] = [29.7, 37.1, 41.1, 43.7, 47.9, 53.3, 59.9, 67.1];
const DIFFUSER_MS: [f64; 4] = [4.77, 3.59, 12.73, 9.30];
const MOD_DEPTH_MS: f64 = 0.6;

/// Feedback gain giving a 60 dB decay over `rt60` seconds for a line of
/// `delay` samples.
pub fn decay_gain(delay: f64, rt60: f64, sample_rate: f64) -> f64 { 10f64.powf(-3.0 * delay / (rt60.max(0.05) * sample_rate)) }

/// Level of the reverb tail `t` seconds after an impulse, in dB, as drawn by
/// the editor (exponential decay reaching −60 dB at the decay time).
pub fn reverb_envelope_db(decay: f64, predelay_ms: f64, t: f64) -> Option<f64> {
    let t = t - predelay_ms * 0.001;
    (t >= 0.0).then(|| -60.0 * t / decay.max(0.05))
}

pub fn reverb_effective_decay(values: &[f64]) -> f64 { values.get(1).copied().unwrap_or(2.0) * values.get(11).copied().unwrap_or(100.0) / 100.0 }

fn reverb_post_coefficients(v: &[f64], fs: f64) -> [super::filters::Coefficients; 5] {
    [if v[6] > 20.5 { rbj(Rbj::HighPass, v[6], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY },
     if v[7] < 19999.0 { rbj(Rbj::LowPass, v[7], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY },
     rbj(Rbj::LowShelf, v[20], 0.707, v[22], fs), rbj(Rbj::HighShelf, v[21], 0.707, v[23], fs),
     rbj(Rbj::Bell, v[24], v[26], v[25], fs)]
}

/// The actual post-EQ response used by the reverb's native curve editor.
pub fn reverb_post_response_db(values: &[f64], freq: f64, sample_rate: f64) -> f64 {
    if values.len() < REVERB_SPECS.len() { return 0.0; }
    reverb_post_coefficients(values, sample_rate).iter().chain(reverb_post_nodes(values,sample_rate).iter()).map(|&c| response_db(c, freq, sample_rate)).sum()
}

/// RT60 model derived from the actual per-line feedback filters. This is a
/// frequency-dependent decay estimate, not a fabricated spectrum trace.
pub fn reverb_decay_at_frequency(v: &[f64], freq: f64, sample_rate: f64) -> f64 {
    if v.len() < REVERB_SPECS.len() { return reverb_effective_decay(v); }
    if v[12] >= 0.5 { return f64::INFINITY; }
    let one_pole = |coefficient: f64| {
        let w = TAU * freq.clamp(1.0, sample_rate * 0.49) / sample_rate;
        let re = 1.0 - coefficient * w.cos(); let im = coefficient * w.sin();
        let scale = (1.0 - coefficient) / (re * re + im * im);
        (re * scale, -im * scale)
    };
    let mul = |a: (f64, f64), b: (f64, f64)| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
    let low = one_pole(one_pole_coefficient(v[20], sample_rate));
    let upper = one_pole(one_pole_coefficient(v[21].max(v[20]), sample_rate));
    let remainder = (1.0 - low.0, -low.1);
    let mid = mul(remainder, upper);
    let high = (remainder.0 - mid.0, remainder.1 - mid.1);
    let damping = if v[3] < 19999.0 { one_pole(one_pole_coefficient(v[3], sample_rate)) } else { (1.0, 0.0) };
    let size = v[2] / 100.0 * match v[10].round() as usize { 1 => 0.92, 2 => 0.73, _ => 1.0 };
    let decay = reverb_effective_decay(v);
    let mut log_gain = 0.0; let mut total_delay = 0.0;
    for line_index in 0..LINES {
        let line=room_line(v,line_index);
        let delay = (line * size * 0.001 * sample_rate).round().max(8.0);
        let gains = [decay_gain(delay, decay * v[18] / 100.0, sample_rate), decay_gain(delay, decay, sample_rate), decay_gain(delay, decay * v[19] / 100.0, sample_rate)];
        let response = mul((low.0 * gains[0] + mid.0 * gains[1] + high.0 * gains[2], low.1 * gains[0] + mid.1 * gains[1] + high.1 * gains[2]), damping);
        let (nodes,scale)=reverb_node_coefficients(v,sample_rate,delay);
        let node_gain=db_to_lin(nodes.iter().map(|&c|response_db(c,freq,sample_rate)).sum())*scale;
        log_gain += (response.0.hypot(response.1)*node_gain).clamp(1e-15, 0.99999999).log10();
        total_delay += delay;
    }
    -3.0 * total_delay / (sample_rate * log_gain)
}

fn room_line(v:&[f64],line:usize)->f64 {
    const ROOMS:[[f64;8];5]=[
        LINE_MS,[13.7,19.3,23.9,31.1,36.7,43.1,47.3,53.9],
        [23.1,29.9,37.7,41.9,53.3,59.3,61.7,67.1],
        [31.3,41.3,43.7,53.9,59.9,61.1,65.9,67.1],
        [17.3,29.3,41.1,47.9,53.3,57.1,61.3,67.1]];
    let room=v.get(REVERB_ADVANCED_START).copied().unwrap_or(0.0).clamp(0.0,4.0);
    let a=room.floor() as usize;let b=(a+1).min(4);let t=room-a as f64;
    ROOMS[a][line]*(1.0-t)+ROOMS[b][line]*t
}
fn reverb_node_coefficients(v:&[f64],fs:f64,delay:f64)->([super::filters::Coefficients;8],f64) {
    let base=decay_gain(delay,reverb_effective_decay(v),fs);
    let mut boost=0.0;
    let coefficients=std::array::from_fn(|i|{
        let o=REVERB_DECAY_START+i*5;
        if v[o]<0.5{return IDENTITY;}
        let rate=v[o+2]*0.01;
        let gain=20.0*(decay_gain(delay,reverb_effective_decay(v)*rate,fs)/base).log10();
        boost+=gain.max(0.0);
        let kind=match v[o+4].round() as usize{1=>Rbj::LowShelf,2=>Rbj::HighShelf,_=>Rbj::Bell};
        rbj(kind,v[o+1],if matches!(kind,Rbj::Bell){v[o+3]}else{0.707},gain,fs)
    });
    // Bound the product of node gains. This retains stable FDN feedback for
    // overlapping positive decay nodes, even during abrupt parameter edits.
    // Legacy low/high decay ratios may already lengthen the feedback tail.
    // Bound against their largest gain, not only the nominal mid-band gain.
    let decay=reverb_effective_decay(v);
    let spectral_max=base.max(decay_gain(delay,decay*v[18]*0.01,fs)).max(decay_gain(delay,decay*v[19]*0.01,fs));
    let worst=spectral_max*db_to_lin(boost);
    let scale=if worst>0.9995{0.9995/worst}else{1.0};
    (coefficients,scale)
}
fn reverb_post_nodes(v:&[f64],fs:f64)->[super::filters::Coefficients;8] {
    std::array::from_fn(|i|{
        let o=REVERB_POST_START+i*5;if v[o]<0.5{return IDENTITY;}
        let kind=match v[o+4].round() as usize{1=>Rbj::LowShelf,2=>Rbj::HighShelf,3=>Rbj::HighPass,4=>Rbj::LowPass,_=>Rbj::Bell};
        rbj(kind,v[o+1],v[o+3],v[o+2],fs)
    })
}
fn beat_ms(index:usize,tempo:f64)->f64 {
    60000.0/tempo*[0.0,0.0625,0.125,0.25,0.5,1.0,2.0,4.0,8.0][index.min(8)]
}
pub fn reverb_predelay_ms(v:&[f64],tempo:Option<f64>)->f64 {
    let sync=v.get(108).copied().unwrap_or(0.0).round() as usize;
    if sync==0{v[0]}else{beat_ms(sync,tempo.unwrap_or(v[109]))}
}

/// Schroeder energy-decay fit, independently implemented. Input is mono PCM;
/// callers decode/downmix files on the control thread. The returned editable
/// algorithmic settings do not preserve or convolve the original recording.
pub fn fit_reverb_ir(samples:&[f32],sample_rate:f64)->Result<Vec<(String,f64)>,String> {
    if !sample_rate.is_finite()||!(8000.0..=384000.0).contains(&sample_rate)||samples.len()<64||samples.len()>(sample_rate*60.0) as usize||samples.iter().any(|x|!x.is_finite()) {
        return Err("IR requires finite mono PCM, 64 samples to 60 seconds, at 8–384 kHz".into());
    }
    let peak=samples.iter().map(|x|x.abs()).fold(0.0_f32,f32::max);
    if peak<1e-8{return Err("IR contains no measurable energy".into());}
    let onset=samples.iter().position(|x|x.abs()>peak*0.01).unwrap_or(0);
    let peak_at=samples.iter().position(|x|x.abs()==peak).unwrap_or(onset);
    if peak_at>onset+(sample_rate*0.25) as usize {return Err("Input is not a transient impulse response".into());}
    fn decay_fit(x:&[f32],fs:f64)->Option<f64> {
        let total=x.iter().map(|&x|(x as f64).powi(2)).sum::<f64>();
        if total<1e-18{return None;}
        let mut remaining=total;let(mut n,mut sx,mut sy,mut sxx,mut sxy)=(0.0,0.0,0.0,0.0,0.0);
        for (i,&v) in x.iter().enumerate() {
            let db=10.0*(remaining/total).max(1e-30).log10();
            if (-35.0..=-5.0).contains(&db) {let t=i as f64/fs;n+=1.0;sx+=t;sy+=db;sxx+=t*t;sxy+=t*db;}
            remaining=(remaining-(v as f64).powi(2)).max(0.0);
        }
        let den=n*sxx-sx*sx;if n<16.0||den<1e-15{return None;}
        let slope=(n*sxy-sx*sy)/den;
        if slope>=-0.1 {None}else{Some((-60.0/slope).clamp(0.05,80.0))}
    }
    let tail=&samples[onset..];
    let decay=decay_fit(tail,sample_rate).ok_or("IR does not contain a measurable decay")?.clamp(0.2,20.0);
    let mut changes=vec![("decay".into(),decay),("decay_rate".into(),100.0),("predelay".into(),(onset as f64/sample_rate*1000.0).min(250.0)),
        ("predelay_sync".into(),0.0),("style".into(),0.0),("room".into(),(decay.log2()+1.0).clamp(0.0,4.0)),("freeze".into(),0.0),
        ("auto_gate".into(),0.0),("decay_low_rate".into(),100.0),("decay_high_rate".into(),100.0),("damping".into(),20000.0)];
    let centers=[63.0,125.0,250.0,500.0,1000.0,2000.0,4000.0,8000.0];
    let total=tail.iter().map(|&v|(v as f64).powi(2)).sum::<f64>();
    for (i,f) in centers.into_iter().enumerate() {
        if f>=sample_rate*0.45 {continue;}
        let mut filter=Biquad::default();filter.set(rbj(Rbj::BandPass,f,0.707,0.0,sample_rate));
        let filtered:Vec<f32>=tail.iter().map(|&x|filter.run(x as f64) as f32).collect();
        let rt=decay_fit(&filtered,sample_rate).unwrap_or(decay);
        let energy=filtered.iter().map(|&x|(x as f64).powi(2)).sum::<f64>();
        let gain=(10.0*(energy/total.max(1e-18)).max(1e-12).log10()+9.0).clamp(-12.0,12.0);
        for(field,v)in[("on",1.0),("freq",f),("rate",(100.0*rt/decay).clamp(25.0,400.0)),("q",0.707),("shape",0.0)] {
            changes.push((format!("decay{}_{field}",i+1),v));
        }
        for(field,v)in[("on",1.0),("freq",f),("gain",gain),("q",0.707),("shape",0.0)] {
            changes.push((format!("post{}_{field}",i+1),v));
        }
    }
    let mut values=std::collections::BTreeMap::new();
    for i in 1..=8 {values.insert(format!("decay{i}_on"),0.0);values.insert(format!("post{i}_on"),0.0);}
    for (id,value) in [("low_cut",20.0),("high_cut",20000.0),("post_low_gain",0.0),("post_high_gain",0.0),("post_mid_gain",0.0)]{values.insert(id.to_string(),value);}
    for (id,value) in changes {values.insert(id,value);}
    Ok(values.into_iter().collect())
}

pub struct ReverbProcessor {
    pub values: Vec<f64>,
    sample_rate: f64,
    predelay: [DelayLine; 2],
    diffusers: [[DelayLine; 4]; 2],
    lines: [DelayLine; LINES],
    damp: [f64; LINES],
    damp_coeff: f64,
    lengths: [f64; LINES],
    gains: [f64; LINES],
    phase: f64,
    post: [[Biquad; 5]; 2],
    spectral_state: [[f64; 2]; LINES],
    spectral_coeff: [f64; 2],
    spectral_gain: [[f64; 3]; LINES],
    duck_envelope: f64,
    gate_remaining: usize,
    gate_gain: f64,
    decay_nodes:[[Biquad;8];LINES],
    decay_scale:[f64;LINES],
    post_nodes:[[Biquad;8];2],
    tempo:Option<f64>,
}

impl ReverbProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let fs = sample_rate.max(8_000.0);
        let samples = |ms: f64| (ms * 0.001 * fs).ceil() as usize + 4;
        let mut p = Self {
            values: super::table_defaults(REVERB_SPECS), sample_rate,
            predelay: [DelayLine::new(samples(24100.0)), DelayLine::new(samples(24100.0))],
            diffusers: std::array::from_fn(|_| DIFFUSER_MS.map(|ms| DelayLine::new(samples(ms)))),
            lines: LINE_MS.map(|ms| DelayLine::new(samples(ms + MOD_DEPTH_MS * 2.0))),
            damp: [0.0; LINES], damp_coeff: 0.0, lengths: [1.0; LINES], gains: [0.0; LINES], phase: 0.0,
            post: Default::default(), spectral_state: [[0.0; 2]; LINES], spectral_coeff: [0.0; 2],
            spectral_gain: [[0.0; 3]; LINES], duck_envelope: 0.0, gate_remaining: 0, gate_gain: 0.0,
            decay_nodes:Default::default(),decay_scale:[1.0;LINES],post_nodes:Default::default(),tempo:None,
        };
        p.refresh();
        p
    }
}

impl TableEffect for ReverbProcessor {
    const SPECS: &'static [Spec] = REVERB_SPECS;
    fn uses_transport_tempo()->bool{true}
    fn set_transport_tempo(&mut self,bpm:f64){if bpm.is_finite()&&(20.0..=1000.0).contains(&bpm){self.tempo=Some(bpm);}}

    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let fs = self.sample_rate;
        let style = self.values[10].round() as usize;
        let size = self.values[2] / 100.0 * match style { 1 => 0.92, 2 => 0.73, _ => 1.0 };
        let decay = reverb_effective_decay(&self.values);
        for i in 0..LINES {
            self.lengths[i] = (room_line(&self.values,i) * size * 0.001 * fs).round().max(8.0);
            self.gains[i] = decay_gain(self.lengths[i], decay, fs);
            self.spectral_gain[i] = [decay_gain(self.lengths[i], decay * self.values[18] / 100.0, fs), self.gains[i], decay_gain(self.lengths[i], decay * self.values[19] / 100.0, fs)];
        }
        for i in 0..LINES {
            let(coeffs,scale)=reverb_node_coefficients(&self.values,fs,self.lengths[i]);self.decay_scale[i]=scale;
            for (f,c) in self.decay_nodes[i].iter_mut().zip(coeffs){f.set(c);}
        }
        let nodes=reverb_post_nodes(&self.values,fs);
        for channel in &mut self.post_nodes{for(f,c)in channel.iter_mut().zip(nodes){f.set(c);}}
        self.spectral_coeff = [one_pole_coefficient(self.values[20], fs), one_pole_coefficient(self.values[21].max(self.values[20]), fs)];
        self.damp_coeff = if self.values[3] < 19_999.0 { one_pole_coefficient(self.values[3], fs) } else { 0.0 };
        let coeffs = reverb_post_coefficients(&self.values, fs);
        for channel in &mut self.post { for (filter, coefficients) in channel.iter_mut().zip(coeffs) { filter.set(coefficients); } }
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); let tempo=self.tempo; *self = Self::new(sample_rate); self.values = values; self.tempo=tempo; self.refresh(); }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let fs = self.sample_rate;
        let predelay = ((reverb_predelay_ms(&self.values,self.tempo) + self.values[14] * 0.3) * 0.001 * fs) as usize;
        let style = self.values[10].round() as usize;
        let frozen = self.values[12] >= 0.5;
        let thickness = self.values[15] / 100.0;
        let diffusion = (self.values[4] / 100.0 * 0.7 + if style == 2 { 0.12 } else { 0.0 } + thickness * 0.08).clamp(0.0, 0.88);
        let width = self.values[5] / 100.0; let wet = self.values[8] / 100.0;
        let depth = if frozen { 0.0 } else { self.values[9] / 100.0 * MOD_DEPTH_MS * 0.001 * fs };
        let step = TAU * if style == 1 { 0.71 } else { 0.37 } / fs;
        let diffuser_len: [usize; 4] = DIFFUSER_MS.map(|ms| ((ms * 0.001 * fs) as usize).max(1));
        let duck_attack = time_coefficient(5.0, fs); let duck_release = time_coefficient(250.0, fs);
        let gate_smoothing = time_coefficient(self.values[112], fs);
        let gate_hold_ms=if self.values[110]>=0.5{beat_ms(self.values[110].round() as usize,self.tempo.unwrap_or(self.values[109]))}else{self.values[17]};
        let gate_hold=(gate_hold_ms*0.001*fs) as usize;let gate_threshold=db_to_lin(self.values[111]);
        let input_scale = 1.0 - self.values[14] / 100.0 * 0.5;
        let input_drive = db_to_lin(thickness.max(0.0) * 12.0);
        for i in 0..in_l.len() {
            let (l, r) = (in_l[i] as f64, in_r[i] as f64);
            let detector = key.map_or(l.abs().max(r.abs()), |(a, b)| a[i].abs().max(b[i].abs()) as f64);
            let coefficient = if detector > self.duck_envelope { duck_attack } else { duck_release };
            self.duck_envelope = detector + coefficient * (self.duck_envelope - detector);
            let duck = db_to_lin(-self.values[13] * ((lin_to_db(self.duck_envelope) + 42.0) / 36.0).clamp(0.0, 1.0));
            if detector > gate_threshold { self.gate_remaining = gate_hold; }
            else { self.gate_remaining = self.gate_remaining.saturating_sub(1); }
            let target_gate = if self.values[16] < 0.5 || self.gate_remaining > 0 || detector > gate_threshold { 1.0 } else { 0.0 };
            self.gate_gain = target_gate + gate_smoothing * (self.gate_gain - target_gate);
            self.predelay[0].push(l); self.predelay[1].push(r);
            let mut x = [self.predelay[0].tap(predelay), self.predelay[1].tap(predelay)];
            for c in 0..2 {
                x[c] *= input_scale;
                if thickness > 0.0 { x[c] = (x[c] * input_drive).tanh() / input_drive.sqrt(); }
                for (k, line) in self.diffusers[c].iter_mut().enumerate() {
                    let delayed = line.tap(diffuser_len[k] - 1);
                    let v = x[c] + diffusion * delayed; line.push(v); x[c] = delayed - diffusion * v;
                }
            }
            self.phase = (self.phase + step).rem_euclid(TAU);
            let mut y = [0.0; LINES];
            for k in 0..LINES {
                let wobble = match k { 1 => self.phase.sin(), 6 => (self.phase * 1.37).cos(), _ => 0.0 } * depth;
                let tap = self.lines[k].tap_frac(self.lengths[k] - 1.0 + wobble);
                if frozen { y[k] = tap; continue; }
                self.damp[k] = tap + self.damp_coeff * (self.damp[k] - tap);
                let signal = self.damp[k];
                self.spectral_state[k][0] = signal + self.spectral_coeff[0] * (self.spectral_state[k][0] - signal);
                let low = self.spectral_state[k][0]; let rest = signal - low;
                self.spectral_state[k][1] = rest + self.spectral_coeff[1] * (self.spectral_state[k][1] - rest);
                let mid = self.spectral_state[k][1]; let high = rest - mid;
                y[k] = low * self.spectral_gain[k][0] + mid * self.spectral_gain[k][1] + high * self.spectral_gain[k][2];
                for filter in &mut self.decay_nodes[k]{y[k]=filter.run(y[k]);}y[k]*=self.decay_scale[k];
                if style == 1 { y[k] = (y[k] * 32768.0).round() / 32768.0; }
            }
            let (wl, wr) = (y[0] - y[2] + y[4] - y[6], y[1] - y[3] + y[5] - y[7]);
            let sum = y.iter().sum::<f64>() * (2.0 / LINES as f64);
            for k in 0..LINES {
                let pan=self.values[114]*0.01;
                let input = if frozen { 0.0 } else { x[k % 2] * 0.35 * (1.0+if k%2==0{-pan}else{pan}).sqrt() };
                self.lines[k].push(y[k] - sum + input);
            }
            let (mid, side) = ((wl + wr) * 0.5, (wl - wr) * 0.5 * width);
            let mut effected = [mid + side, mid - side];
            for c in 0..2 {
                let early=[7.1,13.3,23.9,37.1].into_iter().enumerate().map(|(n,ms)|{
                    let shift=(ms*(0.5+self.values[2]*0.01)*(1.0+if c==0{self.values[114]*0.002}else{-self.values[114]*0.002})*0.001*fs) as usize;
                    self.predelay[(c+n)%2].tap(predelay+shift)*(0.45/(n+1) as f64)
                }).sum::<f64>();
                if !frozen{effected[c]+=early*self.values[113]*0.01;}
                for filter in &mut self.post[c] { effected[c] = filter.run(effected[c]); }
                for filter in &mut self.post_nodes[c] {effected[c]=filter.run(effected[c]);}
            }
            let gate = if self.values[16] >= 0.5 { self.gate_gain } else { 1.0 };
            out_l[i] = (l * (1.0 - wet) + effected[0] * wet * 0.5 * duck * gate) as f32;
            out_r[i] = (r * (1.0 - wet) + effected[1] * wet * 0.5 * duck * gate) as f32;
        }
    }

}

// ---------------------------------------------------------------------------
// Delay
// ---------------------------------------------------------------------------

pub const DELAY_TAPS: usize = 15;
pub const DELAY_TAP_START: usize = 24;
pub const DELAY_TAP_STRIDE: usize = 4;
pub const DELAY_SYNC: [&str; 9] = ["自由", "1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 小节", "2 小节"];
pub static DELAY_SPECS: &[Spec] = &[
    Spec::new("time", "时间", 1.0, 2000.0, 375.0, "ms").log(),
    Spec::new("offset", "左右偏移", -50.0, 50.0, 0.0, "%"),
    Spec::new("feedback", "反馈", 0.0, 95.0, 35.0, "%"),
    Spec::new("pingpong", "乒乓", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("low_cut", "低切", 20.0, 2000.0, 150.0, "Hz").log(),
    Spec::new("high_cut", "高切", 1000.0, 20000.0, 8000.0, "Hz").log(),
    Spec::new("drive", "驱动", 0.0, 24.0, 0.0, "dB"),
    Spec::new("width", "宽度", 0.0, 100.0, 100.0, "%"),
    Spec::new("mix", "干湿比", 0.0, 100.0, 25.0, "%"),
    Spec::new("freeze", "冻结", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("ducking", "闪避", 0.0, 48.0, 0.0, "dB"),
    Spec::new("mod_rate", "调制速率", 0.01, 20.0, 0.5, "Hz").log(),
    Spec::new("mod_depth", "调制深度", 0.0, 20.0, 0.0, "ms"),
    Spec::new("diffusion", "扩散", 0.0, 100.0, 0.0, "%"),
    Spec::new("lofi", "Lo-Fi", 0.0, 100.0, 0.0, "%"),
    Spec::new("dynamics", "动态", -100.0, 100.0, 0.0, "%"),
    Spec::new("pitch", "移调", -12.0, 12.0, 0.0, "st"),
    Spec::new("pitch_mirror", "反向立体声移调", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("main_level", "主抽头电平", -60.0, 12.0, 0.0, "dB"),
    Spec::new("main_pan", "主抽头声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("main_on", "主抽头", 0.0, 1.0, 1.0, "").steps(1),
    Spec::new("cross_feedback", "交叉反馈", 0.0, 95.0, 0.0, "%"),
    Spec::new("sync", "节拍同步", 0.0, 8.0, 0.0, "").steps(8),
    Spec::new("tempo", "参考速度", 20.0, 300.0, 120.0, "BPM"),
    Spec::new("tap1_time", "抽头 1 时间", 0.1, 100.0, 6.25, "%"),
    Spec::new("tap1_level", "抽头 1 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap1_pan", "抽头 1 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap1_on", "抽头 1 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap2_time", "抽头 2 时间", 0.1, 100.0, 12.5, "%"),
    Spec::new("tap2_level", "抽头 2 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap2_pan", "抽头 2 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap2_on", "抽头 2 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap3_time", "抽头 3 时间", 0.1, 100.0, 18.75, "%"),
    Spec::new("tap3_level", "抽头 3 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap3_pan", "抽头 3 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap3_on", "抽头 3 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap4_time", "抽头 4 时间", 0.1, 100.0, 25.0, "%"),
    Spec::new("tap4_level", "抽头 4 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap4_pan", "抽头 4 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap4_on", "抽头 4 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap5_time", "抽头 5 时间", 0.1, 100.0, 31.25, "%"),
    Spec::new("tap5_level", "抽头 5 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap5_pan", "抽头 5 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap5_on", "抽头 5 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap6_time", "抽头 6 时间", 0.1, 100.0, 37.5, "%"),
    Spec::new("tap6_level", "抽头 6 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap6_pan", "抽头 6 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap6_on", "抽头 6 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap7_time", "抽头 7 时间", 0.1, 100.0, 43.75, "%"),
    Spec::new("tap7_level", "抽头 7 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap7_pan", "抽头 7 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap7_on", "抽头 7 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap8_time", "抽头 8 时间", 0.1, 100.0, 50.0, "%"),
    Spec::new("tap8_level", "抽头 8 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap8_pan", "抽头 8 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap8_on", "抽头 8 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap9_time", "抽头 9 时间", 0.1, 100.0, 56.25, "%"),
    Spec::new("tap9_level", "抽头 9 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap9_pan", "抽头 9 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap9_on", "抽头 9 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap10_time", "抽头 10 时间", 0.1, 100.0, 62.5, "%"),
    Spec::new("tap10_level", "抽头 10 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap10_pan", "抽头 10 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap10_on", "抽头 10 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap11_time", "抽头 11 时间", 0.1, 100.0, 68.75, "%"),
    Spec::new("tap11_level", "抽头 11 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap11_pan", "抽头 11 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap11_on", "抽头 11 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap12_time", "抽头 12 时间", 0.1, 100.0, 75.0, "%"),
    Spec::new("tap12_level", "抽头 12 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap12_pan", "抽头 12 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap12_on", "抽头 12 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap13_time", "抽头 13 时间", 0.1, 100.0, 81.25, "%"),
    Spec::new("tap13_level", "抽头 13 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap13_pan", "抽头 13 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap13_on", "抽头 13 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap14_time", "抽头 14 时间", 0.1, 100.0, 87.5, "%"),
    Spec::new("tap14_level", "抽头 14 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap14_pan", "抽头 14 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap14_on", "抽头 14 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("tap15_time", "抽头 15 时间", 0.1, 100.0, 93.75, "%"),
    Spec::new("tap15_level", "抽头 15 电平", -60.0, 12.0, -6.0, "dB"),
    Spec::new("tap15_pan", "抽头 15 声像", -100.0, 100.0, 0.0, "%"),
    Spec::new("tap15_on", "抽头 15 开关", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("time_mode","时间模式",0.0,3.0,0.0,"").steps(3),
    Spec::new("tape_speed","磁带速度",0.125,8.0,1.0,"x").log(),
    Spec::new("stretch_fade","时间切换淡化",5.0,500.0,50.0,"ms").log(),
    Spec::new("routing","延迟路由",0.0,3.0,0.0,"").steps(3),
    Spec::new("sync_feel","节拍类型",0.0,2.0,0.0,"").steps(2),
    Spec::new("filter_routing","滤波器路由",0.0,2.0,0.0,"").steps(2),
    Spec::new("filter1_shape","滤波器 1 形状",0.0,5.0,0.0,"").steps(5),
    Spec::new("filter1_freq","滤波器 1 频率",20.0,20000.0,150.0,"Hz").log(),
    Spec::new("filter1_q","滤波器 1 共振",0.1,18.0,0.707,"").log(),
    Spec::new("filter2_shape","滤波器 2 形状",0.0,5.0,0.0,"").steps(5),
    Spec::new("filter2_freq","滤波器 2 频率",20.0,20000.0,8000.0,"Hz").log(),
    Spec::new("filter2_q","滤波器 2 共振",0.1,18.0,0.707,"").log(),
    Spec::new("feedback_style","反馈染色",0.0,3.0,0.0,"").steps(3),
    Spec::new("feedback_rotate","反馈声场旋转",-180.0,180.0,0.0,"°"),
    Spec::new("feedback_quality","反馈过采样",0.0,5.0,0.0,"").steps(5).fixed(),

 ];

/// Actual base time before per-channel offset and modulation. Host adapters
/// may set the transport tempo via `set_tempo`; saved tempo is the fallback.
pub fn delay_time_ms(values: &[f64], tempo: Option<f64>) -> f64 {
    let sync = values.get(22).copied().unwrap_or(0.0).round() as usize;
    if sync == 0 { return values[0]; }
    // Transport tempo is validated by the host (20..=1000 BPM). Its range
    // is independent of the legacy saved fallback parameter's 20..=300 BPM.
    let bpm = tempo.unwrap_or_else(|| values.get(23).copied().unwrap_or(120.0));
    let beats = [0.0, 0.0625, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0][sync.min(8)];
    60000.0 / bpm * beats * match values.get(88).copied().unwrap_or(0.0).round() as usize {1=>1.5,2=>2.0/3.0,_=>1.0}
}

const DELAY_CAPACITY_SECONDS:f64=54.0;
fn delay_requested_time_ms(values:&[f64],tempo:Option<f64>)->f64{
    let speed=if values.get(84).copied().unwrap_or(0.0).round()==1.0{values.get(85).copied().unwrap_or(1.0)}else{1.0};
    delay_time_ms(values,tempo)/speed
}
/// Settled base time shown by the editor, including tape speed and the
/// available delay-line span. Audio modulation remains a separate motion.
pub fn delay_effective_time_ms(values:&[f64],tempo:Option<f64>,sample_rate:f64)->f64{
    let fs=sample_rate.max(8000.0);let maximum=((DELAY_CAPACITY_SECONDS*fs)as usize+4)as f64*1000.0/fs;
    delay_requested_time_ms(values,tempo).clamp(1000.0/fs,maximum)
}
/// Exact static L/R read-head targets used by the processor. At the capacity
/// limit the two offset channels may saturate independently.
pub fn delay_target_times_ms(values:&[f64],tempo:Option<f64>,sample_rate:f64)->[f64;2]{
    let fs=sample_rate.max(8000.0);let maximum=((DELAY_CAPACITY_SECONDS*fs)as usize+4)as f64*1000.0/fs;
    let base=delay_requested_time_ms(values,tempo);let offset=values.get(1).copied().unwrap_or(0.0)*0.01;
    [base*(1.0+offset),base*(1.0-offset)].map(|time|time.clamp(1000.0/fs,maximum))
}

pub struct DelayProcessor {
    pub values: Vec<f64>, sample_rate: f64, lines: [DelayLine; 2], filters: [[Biquad; 2]; 2],
    times: [f64; 2], drive: f64, started: bool, phase: f64,
    diffuse: [[DelayLine; 2]; 2], pitch_lines: [DelayLine; 2], pitch_phase: [f64; 2],
    envelope: f64, lofi_held: [f64; 2], lofi_clock: usize, tempo: Option<f64>,
    stretch_old:[f64;2], stretch_target:[f64;2], stretch_phase:f64, reverse_phase:[f64;2], tape_memory:[f64;2], feedback_oversampling:[Oversampler;2],

}
impl DelayProcessor {
    pub fn new(sample_rate: f64) -> Self {
        let fs = sample_rate.max(8000.0);
        // Two bars at the minimum tempo, plus the maximum stereo offset.
        let capacity = (DELAY_CAPACITY_SECONDS * fs) as usize + 8;
        let mut p = Self { values: super::table_defaults(DELAY_SPECS), sample_rate,
            lines: [DelayLine::new(capacity), DelayLine::new(capacity)], filters: Default::default(),
            times: [0.0; 2], drive: 1.0, started: false, phase: 0.0,
            diffuse: std::array::from_fn(|_| [DelayLine::new((fs * 0.005) as usize + 2), DelayLine::new((fs * 0.013) as usize + 2)]),
            pitch_lines: [DelayLine::new((fs * 0.05) as usize + 4), DelayLine::new((fs * 0.05) as usize + 4)],
            pitch_phase: [0.0, 0.5], envelope: 0.0, lofi_held: [0.0; 2], lofi_clock: 0, tempo: None, stretch_old:[0.0;2],stretch_target:[0.0;2],stretch_phase:1.0,reverse_phase:[0.0,0.5],tape_memory:[0.0;2], feedback_oversampling:std::array::from_fn(|_|Oversampler::new()) };
        p.refresh(); p.times = p.targets(); p
    }
    pub fn set_tempo(&mut self, bpm: f64) { if bpm.is_finite() && (20.0..=1000.0).contains(&bpm) { self.tempo = Some(bpm); } }
    fn targets(&self) -> [f64; 2] {
        delay_target_times_ms(&self.values,self.tempo,self.sample_rate).map(|time|time*0.001*self.sample_rate)
    }
    pub fn echo_db(&self, n: u32) -> f64 { 20.0 * (self.values[2] / 100.0).max(1e-6).log10() * (n.saturating_sub(1)) as f64 }
    fn shift_pitch(&mut self, channel: usize, x: f64, semitones: f64) -> f64 {
        self.pitch_lines[channel].push(x);
        if semitones.abs() < 0.001 { return x; }
        let window = self.sample_rate * 0.04;
        let ratio = 2.0f64.powf(semitones / 12.0);
        self.pitch_phase[channel] = (self.pitch_phase[channel] + (1.0 - ratio) / window).rem_euclid(1.0);
        let a = self.pitch_phase[channel]; let b = (a + 0.5).fract();
        // Complementary Hann windows mask the delay discontinuities.
        let weight = 0.5 - 0.5 * (TAU * a).cos();
        self.pitch_lines[channel].tap_frac(1.0 + a * window) * weight + self.pitch_lines[channel].tap_frac(1.0 + b * window) * (1.0 - weight)
    }
}
impl TableEffect for DelayProcessor {
    const SPECS: &'static [Spec] = DELAY_SPECS;
    fn uses_transport_tempo() -> bool { true }
    fn set_transport_tempo(&mut self, bpm: f64) { self.set_tempo(bpm); }
    fn values(&self) -> &[f64] { &self.values }
    fn values_mut(&mut self) -> &mut [f64] { &mut self.values }
    fn refresh(&mut self) {
        let fs = self.sample_rate;
        let low = if self.values[4] > 20.5 { rbj(Rbj::HighPass, self.values[4], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY };
        let high = if self.values[5] < 19999.0 { rbj(Rbj::LowPass, self.values[5], FRAC_1_SQRT_2, 0.0, fs) } else { IDENTITY };
        let custom=|shape:f64,freq:f64,q:f64,legacy| {
            let kind=match shape.round() as usize{1=>Rbj::LowPass,2=>Rbj::HighPass,3=>Rbj::BandPass,4=>Rbj::Notch,5=>Rbj::LowPass,_=>return legacy};
            let c=rbj(kind,freq,q,0.0,fs);
            if shape>=4.5{[c[4],c[3],1.0,c[3],c[4]]}else{c}
        };
        let a=custom(self.values[90],self.values[91],self.values[92],low);
        let b=custom(self.values[93],self.values[94],self.values[95],high);
        for channel in &mut self.filters { channel[0].set(a); channel[1].set(b); }
        self.drive = db_to_lin(self.values[6]);
        for channel in &mut self.feedback_oversampling {channel.set_quality(self.values[98].round() as usize);}
    }
    fn set_sample_rate(&mut self, sample_rate: f64) { let values = self.values.clone(); let tempo = self.tempo; *self = Self::new(sample_rate); self.values = values; self.tempo = tempo; self.refresh(); self.times = self.targets(); }
    fn process(&mut self, in_l: &[f32], in_r: &[f32], key: Option<(&[f32], &[f32])>, out_l: &mut [f32], out_r: &mut [f32]) {
        let targets = self.targets();
        if !self.started {self.times=targets;self.stretch_target=targets;self.stretch_old=targets;self.started=true;}
        let mode=self.values[84].round() as usize;
        if mode==2 && targets!=self.stretch_target {
            self.stretch_old=std::array::from_fn(|c|self.stretch_old[c]*(1.0-self.stretch_phase)+self.stretch_target[c]*self.stretch_phase);
            self.stretch_target=targets;self.stretch_phase=0.0;
        }
        let routing=self.values[87].round() as usize;let rotation=self.values[97].to_radians();

        let frozen = self.values[9] >= 0.5;
        let feedback = if frozen { 1.0 } else { self.values[2] / 100.0 };
        let cross = if frozen { 0.0 } else { self.values[21] / 100.0 };
        // The sum of both feedback paths remains below unity.
        let feedback_scale = if frozen { 1.0 } else { 0.95 / (feedback + cross).max(0.95) };
        let pingpong = self.values[3] >= 0.5; let width = self.values[7] / 100.0; let wet = self.values[8] / 100.0;
        let glide = (-1.0 / (0.05 * self.sample_rate)).exp();
        let attack = time_coefficient(5.0, self.sample_rate); let release = time_coefficient(150.0, self.sample_rate);
        let diffusion = self.values[13] / 100.0; let lo_fi = self.values[14] / 100.0;
        let hold = (1.0 + lo_fi * 31.0).round() as usize; let quantize = 2.0f64.powf((16.0 - lo_fi * 12.0).round());
        let dynamics = self.values[15] / 100.0;
        let main_gain = if self.values[20] >= 0.5 { db_to_lin(self.values[18]) } else { 0.0 }; let main_pan = self.values[19] / 100.0;
        for i in 0..in_l.len() {
            let dry = [in_l[i] as f64, in_r[i] as f64];
            let x=if routing==1{[(dry[0]+dry[1])*FRAC_1_SQRT_2,(dry[0]-dry[1])*FRAC_1_SQRT_2]}else{dry};
            let detector = key.map_or(x[0].abs().max(x[1].abs()), |(a, b)| a[i].abs().max(b[i].abs()) as f64);
            let coeff = if detector > self.envelope { attack } else { release };
            self.envelope = detector + coeff * (self.envelope - detector);
            let db = lin_to_db(self.envelope);
            let duck = db_to_lin(-self.values[10] * ((db + 42.0) / 36.0).clamp(0.0, 1.0));
            let dyn_db = if dynamics >= 0.0 { ((-18.0 - db) * dynamics * 0.5).clamp(-24.0, 12.0) } else { ((db + 35.0).min(0.0) * -dynamics * 2.0).max(-80.0) };
            let dynamic_gain = db_to_lin(dyn_db);
            self.phase = (self.phase + TAU * self.values[11] / self.sample_rate).rem_euclid(TAU);
            let mut echo = [0.0; 2];
            for c in 0..2 {
                self.times[c] = if mode==1 { self.times[c]+(targets[c]-self.times[c]).clamp(-0.75,0.75) }
                    else if mode>=2 {targets[c]}else{targets[c]+glide*(self.times[c]-targets[c])};
                let modulation = if frozen { 0.0 } else { self.values[12] * 0.001 * self.sample_rate * (self.phase + c as f64 * 1.1).sin() };
                echo[c] = match mode {
                    2=>self.lines[c].tap_frac(self.stretch_old[c]-1.0+modulation)*(1.0-self.stretch_phase)
                        +self.lines[c].tap_frac(self.stretch_target[c]-1.0+modulation)*self.stretch_phase,
                    3=>{
                        let length=self.times[c].max(8.0);
                        self.reverse_phase[c]=(self.reverse_phase[c]+2.0/length).fract();
                        let a=self.reverse_phase[c];let b=(a+0.5).fract();let weight=0.5-0.5*(TAU*a).cos();
                        self.lines[c].tap_frac(1.0+a*length)*weight+self.lines[c].tap_frac(1.0+b*length)*(1.0-weight)
                    }
                    _=>self.lines[c].tap_frac(self.times[c]-1.0+modulation),
                };
            }
            self.stretch_phase=(self.stretch_phase+1.0/(self.values[86]*0.001*self.sample_rate)).min(1.0);
            let mut fed = echo;
            for c in 0..2 {
                if frozen { continue; }
                let f = match self.values[89].round() as usize {
                    1=>(self.filters[c][0].run(echo[c])+self.filters[c][1].run(echo[c]))*0.5,
                    2=>self.filters[c][c].run(echo[c]),
                    _=>{let t=self.filters[c][0].run(echo[c]);self.filters[c][1].run(t)}
                };
                let style=self.values[96].round() as usize;let drive=self.drive;
                let signal=if style==1 {
                    let a=one_pole_coefficient((4000.0*self.values[85]).clamp(500.0,20000.0),self.sample_rate);
                    self.tape_memory[c]=f+a*(self.tape_memory[c]-f);self.tape_memory[c]
                }else{f};
                let mut value=self.feedback_oversampling[c].run(signal,|f|match style {
                    1=>(f*drive).tanh()/drive,
                    2=>((f*drive+0.1).tanh()-0.1_f64.tanh())/drive,
                    3=>(f*drive).atan()/drive,
                    _=>if drive>1.0{(f*drive).tanh()/drive}else{f},
                });

                if diffusion > 0.0 {
                    let mut diffused = value;
                    for line in &mut self.diffuse[c] { let delayed = line.tap(line.capacity() - 2); let v = diffused + 0.7 * delayed; line.push(v); diffused = delayed - 0.7 * v; }
                    value = value * (1.0 - diffusion) + diffused * diffusion;
                }
                if lo_fi > 0.0 { if self.lofi_clock % hold == 0 { self.lofi_held[c] = (value * quantize).round() / quantize; } value = self.lofi_held[c]; }
                let pitch = if c == 1 && self.values[17] >= 0.5 { -self.values[16] } else { self.values[16] };
                fed[c] = self.shift_pitch(c, value, pitch);
            }
            let mut output = [echo[0] * main_gain * (1.0 - main_pan.max(0.0)), echo[1] * main_gain * (1.0 + main_pan.min(0.0))];
            for t in 0..DELAY_TAPS {
                let p = DELAY_TAP_START + t * DELAY_TAP_STRIDE;
                if self.values[p + 3] < 0.5 { continue; }
                let time = self.values[p] / 100.0; let level = db_to_lin(self.values[p + 1]); let pan = self.values[p + 2] / 100.0;
                output[0] += self.lines[0].tap_frac((self.times[0] * time).max(1.0) - 1.0) * level * (1.0 - pan.max(0.0));
                output[1] += self.lines[1].tap_frac((self.times[1] * time).max(1.0) - 1.0) * level * (1.0 + pan.min(0.0));
            }
            if rotation.abs()>1e-9 {
                fed=[fed[0]*rotation.cos()-fed[1]*rotation.sin(),fed[0]*rotation.sin()+fed[1]*rotation.cos()];
            }
            if routing==2{fed.swap(0,1);}
            if frozen { self.lines[0].push(echo[0]); self.lines[1].push(echo[1]); }
            else if pingpong {
                self.lines[0].push((x[0] + x[1]) * 0.5 * dynamic_gain + (fed[1] * feedback + fed[0] * cross) * feedback_scale);
                self.lines[1].push((fed[0] * feedback + fed[1] * cross) * feedback_scale);
            } else if routing==3 {
                self.lines[0].push(x[0]*dynamic_gain+fed[0]*feedback*feedback_scale);
                self.lines[1].push((x[1]*dynamic_gain+fed[0]+fed[1]*feedback)*feedback_scale);
            } else {
                self.lines[0].push(x[0] * dynamic_gain + (fed[0] * feedback + fed[1] * cross) * feedback_scale);
                self.lines[1].push(x[1] * dynamic_gain + (fed[1] * feedback + fed[0] * cross) * feedback_scale);
            }
            if routing==1{output=[(output[0]+output[1])*FRAC_1_SQRT_2,(output[0]-output[1])*FRAC_1_SQRT_2];}
            let (mid, side) = ((output[0] + output[1]) * 0.5, (output[0] - output[1]) * 0.5 * width);
            out_l[i] = (dry[0] * (1.0 - wet) + (mid + side) * wet * duck) as f32;
            out_r[i] = (dry[1] * (1.0 - wet) + (mid - side) * wet * duck) as f32;
            self.lofi_clock = self.lofi_clock.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_util::*;

    fn impulse_response(fx: &mut impl TableEffect, n: usize) -> (Vec<f32>, Vec<f32>) {
        let mut x = vec![0.0; n];
        x[0] = 1.0;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        fx.process(&x, &x, None, &mut l, &mut r);
        (l, r)
    }

    #[test]
    fn reverb_tail_decays_at_the_set_time_and_is_stereo() {
        for decay in [0.8, 3.0] {
            let mut rv = ReverbProcessor::new(FS);
            for (id, v) in [("decay", decay), ("mix", 100.0), ("predelay", 0.0), ("damping", 20_000.0), ("low_cut", 20.0), ("high_cut", 20_000.0), ("modulation", 0.0)] { rv.set(id, v).unwrap(); }
            let n = (FS * decay * 1.2) as usize;
            let (l, r) = impulse_response(&mut rv, n);
            let window = 2_400;
            let early = rms_db(&l[4_800..4_800 + window]);
            let at = (FS * decay * 0.5) as usize;
            let later = rms_db(&l[at..at + window]);
            // Halfway through the decay time the tail has fallen ~30 dB.
            let fall = early - later;
            let expected = 60.0 * (at - 4_800) as f64 / (FS * decay);
            assert!((fall - expected).abs() < 6.0, "decay {decay}: fell {fall}, expected {expected}");
            assert!(l.iter().zip(&r).any(|(a, b)| (a - b).abs() > 1e-4), "decorrelated channels");
            assert!(l.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn reverb_predelay_holds_the_tail_back_and_dry_passes() {
        let mut rv = ReverbProcessor::new(FS);
        rv.set("predelay", 50.0).unwrap();
        rv.set("mix", 100.0).unwrap();
        let (l, _) = impulse_response(&mut rv, 6_000);
        assert!(l[..2_400].iter().all(|v| v.abs() < 1e-9));
        assert!(peak(&l[2_400..]) > 1e-3);
        let mut dry = ReverbProcessor::new(FS);
        dry.set("mix", 0.0).unwrap();
        let x = sine(440.0, 0.3, 2_000);
        let (mut l, mut r) = (vec![0.0; x.len()], vec![0.0; x.len()]);
        dry.process(&x, &x, None, &mut l, &mut r);
        assert_eq!(l, x);
    }

    #[test]
    fn delay_repeats_at_the_set_time_with_feedback() {
        let mut d = DelayProcessor::new(FS);
        for (id, v) in [("time", 100.0), ("feedback", 50.0), ("mix", 100.0), ("low_cut", 20.0), ("high_cut", 20_000.0)] { d.set(id, v).unwrap(); }
        let (l, _) = impulse_response(&mut d, 20_000);
        let first = l.iter().position(|v| v.abs() > 0.1).unwrap();
        assert!((first as i64 - 4_800).abs() <= 2, "first echo at {first}");
        let second = peak(&l[9_000..10_000]);
        assert!((second as f64 - 0.5).abs() < 0.05, "second echo {second}");
    }

    #[test]
    fn pingpong_alternates_sides() {
        let mut d = DelayProcessor::new(FS);
        for (id, v) in [("time", 50.0), ("feedback", 80.0), ("mix", 100.0), ("pingpong", 1.0), ("low_cut", 20.0), ("high_cut", 20_000.0)] { d.set(id, v).unwrap(); }
        let (l, r) = impulse_response(&mut d, 8_000);
        assert!(peak(&l[2_300..2_500]) > 0.3 && peak(&r[2_300..2_500]) < 0.05);
        assert!(peak(&r[4_700..4_900]) > 0.2 && peak(&l[4_700..4_900]) < 0.05);
    }

    #[test]
    fn reverb_freeze_preserves_the_tank_and_excludes_new_audio() {
        let configure = || { let mut rv = ReverbProcessor::new(FS); rv.set("mix", 100.0).unwrap(); rv.set("modulation", 0.0).unwrap(); rv };
        let mut a = configure(); let mut b = configure();
        impulse_response(&mut a, 24000); impulse_response(&mut b, 24000);
        a.set("freeze", 1.0).unwrap(); b.set("freeze", 1.0).unwrap();
        let incoming = sine(440.0, 1.0, 48000); let silence = vec![0.0; incoming.len()];
        let (mut al, mut ar, mut bl, mut br) = (silence.clone(), silence.clone(), silence.clone(), silence.clone());
        a.process(&incoming, &incoming, None, &mut al, &mut ar); b.process(&silence, &silence, None, &mut bl, &mut br);
        assert_eq!(al, bl); assert_eq!(ar, br);
        assert!((rms_db(&al[4000..12000]) - rms_db(&al[36000..44000])).abs() < 3.0);
        assert!(reverb_decay_at_frequency(&a.values, 1000.0, FS).is_infinite());
    }

    #[test]
    fn reverb_decay_and_post_eq_curves_use_actual_filter_parameters() {
        let mut rv = ReverbProcessor::new(FS); rv.set("damping", 20000.0).unwrap();
        rv.set("decay_rate", 200.0).unwrap();
        assert_eq!(reverb_effective_decay(&rv.values), 4.0);
        for f in [100.0, 1000.0, 10000.0] { assert!((reverb_decay_at_frequency(&rv.values, f, FS) - 4.0).abs() < 0.01); }
        rv.set("decay_low_rate", 200.0).unwrap();
        assert!(reverb_decay_at_frequency(&rv.values, 30.0, FS) > reverb_decay_at_frequency(&rv.values, 10000.0, FS));
        rv.set("low_cut", 20.0).unwrap(); rv.set("high_cut", 20000.0).unwrap(); rv.set("post_mid_gain", 9.0).unwrap();
        assert!((reverb_post_response_db(&rv.values, 1000.0, FS) - 9.0).abs() < 0.01);
    }

    #[test]
    fn reverb_ducking_reduces_wet_output_without_leaking_the_sidechain() {
        let mut plain = ReverbProcessor::new(FS); let mut ducked = ReverbProcessor::new(FS);
        plain.set("mix", 100.0).unwrap(); ducked.set("mix", 100.0).unwrap(); ducked.set("ducking", 24.0).unwrap();
        let mut input = vec![0.0; 24000]; input[0] = 1.0; let key = vec![0.8; input.len()];
        let (mut pl, mut pr, mut dl, mut dr) = (vec![0.0; input.len()], vec![0.0; input.len()], vec![0.0; input.len()], vec![0.0; input.len()]);
        plain.process(&input, &input, None, &mut pl, &mut pr); ducked.process(&input, &input, Some((&key, &key)), &mut dl, &mut dr);
        assert!(rms_db(&pl[6000..]) - rms_db(&dl[6000..]) > 20.0);
        let silence = vec![0.0; 24000]; let mut empty = ReverbProcessor::new(FS); empty.set("ducking", 24.0).unwrap();
        empty.process(&silence, &silence, Some((&key, &key)), &mut dl, &mut dr); assert_eq!(peak(&dl), 0.0);
    }

    #[test]
    fn delay_taps_have_independent_time_level_pan_and_enable() {
        let mut d = DelayProcessor::new(FS);
        for (id, v) in [("time", 100.0), ("feedback", 0.0), ("mix", 100.0), ("main_on", 0.0), ("tap15_on", 1.0), ("tap15_time", 50.0), ("tap15_level", -6.0), ("tap15_pan", -100.0)] { d.set(id, v).unwrap(); }
        let (l, r) = impulse_response(&mut d, 6000);
        assert_eq!(l.iter().position(|v| v.abs() > 0.1), Some(2400));
        assert!((peak(&l) as f64 - db_to_lin(-6.0)).abs() < 1e-6); assert_eq!(peak(&r), 0.0);
        assert_eq!(d.descs()[0].id, "time"); assert_eq!(d.descs()[8].id, "mix");
    }

    #[test]
    fn delay_tempo_sync_tracks_transport_and_freeze_stops_input() {
        let mut d = DelayProcessor::new(FS); d.set("sync", 5.0).unwrap();
        assert_eq!(delay_time_ms(&d.values, None), 500.0); d.set_tempo(60.0);
        assert_eq!(d.targets(), [48000.0; 2]);
        d.set_tempo(1000.0); assert_eq!(d.targets(), [2880.0; 2]);
        assert_eq!(delay_time_ms(&d.values, Some(1000.0)), 60.0);
        d.set("sync", 0.0).unwrap(); d.set("time", 10.0).unwrap(); d.set("mix", 100.0).unwrap(); d.set("freeze", 1.0).unwrap();
        let (l, r) = impulse_response(&mut d, 2400); assert_eq!(peak(&l), 0.0); assert_eq!(peak(&r), 0.0);
    }

    #[test]
    fn creative_delay_controls_stay_bounded_with_feedback() {
        let mut d = DelayProcessor::new(FS);
        for (id, v) in [("time", 25.0), ("feedback", 95.0), ("cross_feedback", 95.0), ("mod_depth", 20.0), ("mod_rate", 20.0), ("diffusion", 100.0), ("lofi", 100.0), ("pitch", 12.0), ("pitch_mirror", 1.0)] { d.set(id, v).unwrap(); }
        let (l, r) = impulse_response(&mut d, 48000);
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0));
    }

    #[test]
    fn editable_reverb_nodes_and_room_interpolation_change_the_tank() {
        let mut p=ReverbProcessor::new(FS);
        for(id,v)in[("damping",20000.0),("decay1_on",1.0),("decay1_freq",1000.0),("decay1_rate",50.0),("post1_on",1.0),("post1_freq",1000.0),("post1_gain",-9.0),("low_cut",20.0),("high_cut",20000.0)]{p.set(id,v).unwrap();}
        assert!(reverb_decay_at_frequency(&p.values,1000.0,FS)<reverb_decay_at_frequency(&p.values,100.0,FS));
        assert!((reverb_post_response_db(&p.values,1000.0,FS)+9.0).abs()<0.01);
        p.set("room",1.0).unwrap();let a=p.lengths;p.set("room",2.0).unwrap();let b=p.lengths;p.set("room",1.5).unwrap();
        for i in 0..8 {assert!((p.lengths[i]-(a[i]+b[i])*0.5).abs()<=1.0);}
        let (l,r)=impulse_response(&mut p,48000);assert!(l.iter().chain(&r).all(|x|x.is_finite()&&x.abs()<2.0));
    }
    #[test]
    fn reverb_ir_fit_recovers_decay_and_rejects_silence_nonfinite_and_sweeps() {
        let fs=48000.0;let decay=1.5;
        let mut ir=vec![0.0;96000];
        for(i,x)in ir.iter_mut().enumerate().skip(480){*x=(((i*1973%997) as f64/498.0)-1.0) as f32*10.0_f32.powf(-3.0*(i-480) as f32/(fs*decay) as f32);}
        let fitted=fit_reverb_ir(&ir,fs).unwrap();
        let value=|id:&str|fitted.iter().find(|p|p.0==id).unwrap().1;
        assert!((value("decay")-decay).abs()<0.06);assert!((value("predelay")-10.0).abs()<0.1);
        let mut p=ReverbProcessor::new(fs);for(id,v)in fitted {p.set(&id,v).unwrap();}
        assert!(fit_reverb_ir(&vec![0.0;1024],fs).is_err());
        assert!(fit_reverb_ir(&vec![f32::NAN;1024],fs).is_err());
        let sweep:Vec<f32>=(0..48000).map(|i|i as f32/48000.0).collect();assert!(fit_reverb_ir(&sweep,fs).is_err());
    }
    #[test]
    fn reverb_beat_predelay_and_gate_follow_private_transport() {
        let mut p=ReverbProcessor::new(FS);p.set("predelay_sync",5.0).unwrap();
        assert_eq!(reverb_predelay_ms(&p.values,None),500.0);
        p.set_transport_tempo(60.0);assert_eq!(reverb_predelay_ms(&p.values,p.tempo),1000.0);
        assert_eq!(p.value("tempo"),120.0);
        p.set("mix",100.0).unwrap();let(l,_)=impulse_response(&mut p,52000);
        assert!(l[..48000].iter().all(|x|*x==0.0));assert!(peak(&l[48000..])>0.001);
    }
    #[test]
    fn creative_delay_tape_stretch_reverse_and_routing_are_partition_invariant() {
        for mode in 1..=3 {
            let configure=||{let mut p=DelayProcessor::new(FS);for(id,v)in[("time",15.0),("mix",100.0),("time_mode",mode as f64),("tape_speed",2.0),("routing",1.0),("feedback_quality",1.0)]{p.set(id,v).unwrap();}p};
            let x=sine(700.0,0.2,6000);let(mut a,mut b)=(configure(),configure());let(mut l,mut r)=(vec![0.0;x.len()],vec![0.0;x.len()]);
            a.process(&x,&x,None,&mut l,&mut r);
            let mut assembled=Vec::new();for chunk in x.chunks(137){let(mut bl,mut br)=(vec![0.0;chunk.len()],vec![0.0;chunk.len()]);b.process(chunk,chunk,None,&mut bl,&mut br);assembled.extend(bl);}
            assert_eq!(l,assembled);assert!(l.iter().all(|x|x.is_finite()&&x.abs()<2.0));assert!(peak(&l)>0.05);
        }
        let mut p=DelayProcessor::new(FS);p.set("sync",5.0).unwrap();p.set("sync_feel",1.0).unwrap();
        assert_eq!(delay_time_ms(&p.values,Some(120.0)),750.0);p.set("sync_feel",2.0).unwrap();
        assert!((delay_time_ms(&p.values,Some(120.0))-1000.0/3.0).abs()<1e-10);
    }

    #[test]
    fn overlapping_long_decay_nodes_remain_stable_with_legacy_spectral_rates(){
        let mut reverb=ReverbProcessor::new(FS);
        for(id,value)in[("decay",0.25),("size",50.0),("mix",100.0),("damping",20000.0),("modulation",0.0),("decay_low_rate",400.0),("decay_high_rate",400.0)]{
            reverb.set(id,value).unwrap();
        }
        for node in 0..8{
            let at=REVERB_DECAY_START+node*5;
            reverb.values[at]=1.0;reverb.values[at+1]=1000.0;reverb.values[at+2]=400.0;reverb.values[at+4]=1.0;
        }
        reverb.refresh();
        let(l,r)=impulse_response(&mut reverb,96000);
        assert!(l.iter().chain(&r).all(|value|value.is_finite()&&value.abs()<2.0));
        assert!(rms_db(&l[72000..])+rms_db(&r[72000..])<rms_db(&l[12000..36000])+rms_db(&r[12000..36000])+3.0);
    }

    #[test]
    fn tape_speed_and_capacity_targets_match_real_echo_arrivals(){
        let mut delay=DelayProcessor::new(FS);
        for(id,value)in[("time",25.0),("time_mode",1.0),("tape_speed",0.5),("offset",25.0),("feedback",0.0),("mix",100.0)]{delay.set(id,value).unwrap();}
        assert_eq!(delay_effective_time_ms(&delay.values,None,FS),50.0);
        assert_eq!(delay_target_times_ms(&delay.values,None,FS),[62.5,37.5]);
        let(l,r)=impulse_response(&mut delay,4000);
        assert_eq!(l.iter().position(|value|value.abs()>0.5),Some(3000));
        assert_eq!(r.iter().position(|value|value.abs()>0.5),Some(1800));
        for(id,value)in[("sync",8.0),("sync_feel",1.0),("tape_speed",0.125)]{delay.set(id,value).unwrap();}
        delay.set_tempo(20.0);
        let actual=delay_target_times_ms(&delay.values,Some(20.0),FS);
        assert!(actual.iter().all(|&time|time>=54000.0&&time<54001.0));
        assert_eq!(delay.targets(),actual.map(|time|time*0.001*FS));
    }

}
