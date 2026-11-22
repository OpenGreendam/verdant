//! Fixed-capacity modulation and MIDI-learn shared by native processors.
//! Source values are normalized; target scaling respects frequency/time units.
use super::Spec;
use crate::proto::ParamDesc;
pub const MOD_SLOTS: usize = 8;
pub const MOD_COUNT: usize = 164;
pub const MOD_SOURCE_LABELS: [&str;15] = ["关闭","LFO 1","LFO 2","包络跟随","触发包络","CC / 调制轮","力度","键盘跟踪","压力","音色","X","Y","随机","弯音","步进"];
pub const MOD_SPECS: &[Spec] = &[
    Spec::new("mod1_source", "调制 1 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod1_target", "调制 1 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod1_amount", "调制 1 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod1_curve", "调制 1 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod2_source", "调制 2 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod2_target", "调制 2 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod2_amount", "调制 2 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod2_curve", "调制 2 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod3_source", "调制 3 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod3_target", "调制 3 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod3_amount", "调制 3 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod3_curve", "调制 3 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod4_source", "调制 4 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod4_target", "调制 4 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod4_amount", "调制 4 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod4_curve", "调制 4 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod5_source", "调制 5 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod5_target", "调制 5 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod5_amount", "调制 5 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod5_curve", "调制 5 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod6_source", "调制 6 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod6_target", "调制 6 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod6_amount", "调制 6 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod6_curve", "调制 6 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod7_source", "调制 7 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod7_target", "调制 7 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod7_amount", "调制 7 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod7_curve", "调制 7 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mod8_source", "调制 8 · 来源", 0.0, 14.0, 0.0, "").steps(14),
    Spec::new("mod8_target", "调制 8 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("mod8_amount", "调制 8 · 深度", -100.0, 100.0, 0.0, "%"),
    Spec::new("mod8_curve", "调制 8 · 曲线", -1.0, 1.0, 0.0, ""),
    Spec::new("mlfo1_rate", "LFO 1 · 速率", 0.01, 40.0, 1.0, "Hz"),
    Spec::new("mlfo1_shape", "LFO 1 · 形状", 0.0, 5.0, 0.0, "").steps(5),
    Spec::new("mlfo1_sync", "LFO 1 · 同步", 0.0, 8.0, 0.0, "").steps(8),
    Spec::new("mlfo2_rate", "LFO 2 · 速率", 0.01, 40.0, 1.0, "Hz"),
    Spec::new("mlfo2_shape", "LFO 2 · 形状", 0.0, 5.0, 0.0, "").steps(5),
    Spec::new("mlfo2_sync", "LFO 2 · 同步", 0.0, 8.0, 0.0, "").steps(8),
    Spec::new("menv_attack", "调制包络 · 起音", 0.1, 10000.0, 10.0, "ms"),
    Spec::new("menv_decay", "调制包络 · 衰减", 0.1, 10000.0, 250.0, "ms"),
    Spec::new("menv_sustain", "调制包络 · 延音", 0.0, 100.0, 70.0, "%"),
    Spec::new("menv_release", "调制包络 · 释音", 0.1, 10000.0, 500.0, "ms"),
    Spec::new("mfollower_attack", "包络跟随 · 起音", 0.1, 2000.0, 10.0, "ms"),
    Spec::new("mfollower_release", "包络跟随 · 释音", 0.1, 10000.0, 250.0, "ms"),
    Spec::new("mxy_x", "XY · X", -100.0, 100.0, 0.0, "%"),
    Spec::new("mxy_y", "XY · Y", -100.0, 100.0, 0.0, "%"),
    Spec::new("mseq_rate", "步进 · 速率", 0.01, 40.0, 2.0, "Hz"),
    Spec::new("mseq_step1", "步进 1", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step2", "步进 2", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step3", "步进 3", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step4", "步进 4", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step5", "步进 5", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step6", "步进 6", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step7", "步进 7", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step8", "步进 8", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step9", "步进 9", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step10", "步进 10", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step11", "步进 11", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step12", "步进 12", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step13", "步进 13", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step14", "步进 14", -100.0, 100.0, -100.0, "%"),
    Spec::new("mseq_step15", "步进 15", -100.0, 100.0, 100.0, "%"),
    Spec::new("mseq_step16", "步进 16", -100.0, 100.0, -100.0, "%"),
    Spec::new("midi1_cc", "MIDI 1 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi1_target", "MIDI 1 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi1_channel", "MIDI 1 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi2_cc", "MIDI 2 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi2_target", "MIDI 2 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi2_channel", "MIDI 2 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi3_cc", "MIDI 3 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi3_target", "MIDI 3 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi3_channel", "MIDI 3 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi4_cc", "MIDI 4 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi4_target", "MIDI 4 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi4_channel", "MIDI 4 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi5_cc", "MIDI 5 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi5_target", "MIDI 5 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi5_channel", "MIDI 5 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi6_cc", "MIDI 6 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi6_target", "MIDI 6 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi6_channel", "MIDI 6 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi7_cc", "MIDI 7 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi7_target", "MIDI 7 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi7_channel", "MIDI 7 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("midi8_cc", "MIDI 8 · CC", -1.0, 127.0, -1.0, "").steps(128),
    Spec::new("midi8_target", "MIDI 8 · 目标", 0.0, 4095.0, 0.0, "").steps(4095),
    Spec::new("midi8_channel", "MIDI 8 · 通道", 0.0, 16.0, 0.0, "").steps(16),
    Spec::new("mseq_length", "步进 · 长度", 1.0, 16.0, 16.0, "").steps(15),
    Spec::new("mseq_sync", "步进 · 节拍", 0.0, 13.0, 0.0, "").steps(13),
    Spec::new("mseq_offset", "步进 · 同步倍率", 0.5, 2.0, 1.0, ""),
    Spec::new("mseq_balance", "步进 · 时间平衡", -0.99, 0.99, 0.0, ""),
    Spec::new("mseq_glide", "步进 · 全局滑移", -1.0, 1.0, 0.0, ""),
    Spec::new("mseq_phase", "步进 · 相位", 0.0, 100.0, 0.0, "%"),
    Spec::new("mseq_retrigger", "步进 · MIDI 重触发", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("mseq_snap", "步进 · 半音吸附", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_level", "步进 · 来源电平", -2.0, 2.0, 1.0, ""),
    Spec::new("mseq_glide1", "步进 1 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide2", "步进 2 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide3", "步进 3 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide4", "步进 4 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide5", "步进 5 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide6", "步进 6 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide7", "步进 7 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide8", "步进 8 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide9", "步进 9 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide10", "步进 10 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide11", "步进 11 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide12", "步进 12 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide13", "步进 13 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide14", "步进 14 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide15", "步进 15 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_glide16", "步进 16 · 滑移", 0.0, 1.0, 0.0, ""),
    Spec::new("mseq_curve1", "步进 1 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve2", "步进 2 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve3", "步进 3 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve4", "步进 4 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve5", "步进 5 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve6", "步进 6 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve7", "步进 7 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve8", "步进 8 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve9", "步进 9 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve10", "步进 10 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve11", "步进 11 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve12", "步进 12 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve13", "步进 13 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve14", "步进 14 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve15", "步进 15 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_curve16", "步进 16 · 曲线", 0.0, 3.0, 0.0, "").steps(3),
    Spec::new("mseq_random1", "步进 1 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random2", "步进 2 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random3", "步进 3 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random4", "步进 4 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random5", "步进 5 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random6", "步进 6 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random7", "步进 7 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random8", "步进 8 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random9", "步进 9 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random10", "步进 10 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random11", "步进 11 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random12", "步进 12 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random13", "步进 13 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random14", "步进 14 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random15", "步进 15 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mseq_random16", "步进 16 · 随机", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mlfo1_phase", "LFO 1 · 相位", 0.0, 100.0, 0.0, "%"),
    Spec::new("mlfo1_balance", "LFO 1 · 时间平衡", -0.99, 0.99, 0.0, ""),
    Spec::new("mlfo1_offset", "LFO 1 · 同步倍率", 0.5, 2.0, 1.0, ""),
    Spec::new("mlfo1_retrigger", "LFO 1 · MIDI 重触发", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("mlfo1_level", "LFO 1 · 来源电平", -2.0, 2.0, 1.0, ""),
    Spec::new("mlfo2_phase", "LFO 2 · 相位", 0.0, 100.0, 0.0, "%"),
    Spec::new("mlfo2_balance", "LFO 2 · 时间平衡", -0.99, 0.99, 0.0, ""),
    Spec::new("mlfo2_offset", "LFO 2 · 同步倍率", 0.5, 2.0, 1.0, ""),
    Spec::new("mlfo2_retrigger", "LFO 2 · MIDI 重触发", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("mlfo2_level", "LFO 2 · 来源电平", -2.0, 2.0, 1.0, ""),
    Spec::new("menv_trigger", "调制包络 · 触发来源", 0.0, 2.0, 0.0, "").steps(2),
    Spec::new("menv_threshold", "调制包络 · 音频阈值", -96.0, 0.0, -24.0, "dB"),
    Spec::new("menv_hysteresis", "调制包络 · 迟滞", 0.0, 24.0, 3.0, "dB"),
    Spec::new("menv_range", "调制包络 · 输出范围", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("menv_level", "调制包络 · 来源电平", -2.0, 2.0, 1.0, ""),
    Spec::new("mfollower_mode", "包络跟随 · 模式", 0.0, 1.0, 0.0, "").steps(1),
    Spec::new("mfollower_gain", "包络跟随 · 输入增益", -36.0, 36.0, 0.0, "dB"),
    Spec::new("mfollower_level", "包络跟随 · 来源电平", -2.0, 2.0, 1.0, ""),
    Spec::new("mmidi_cc", "MIDI 来源 · CC", 0.0, 127.0, 1.0, "").steps(127),
    Spec::new("mmidi_channel", "MIDI 来源 · 通道", 0.0, 16.0, 0.0, "").steps(16),
];
pub fn modulation_parameter(id: &str) -> Option<Spec> { MOD_SPECS.iter().find(|s|s.id==id).copied() }
#[derive(Clone,Debug,PartialEq)]
pub struct ModParams { pub values: [f64; MOD_COUNT] }
impl Default for ModParams {
 fn default()->Self { Self { values:std::array::from_fn(|i|MOD_SPECS[i].default) } }
}
impl ModParams {
 pub fn get(&self,id:&str)->f64 { MOD_SPECS.iter().position(|s|s.id==id).map_or(0.,|i|self.values[i]) }
 pub fn set(&mut self,id:&str,value:f64)->Result<(),String> {
  let i=MOD_SPECS.iter().position(|s|s.id==id).ok_or("unknown modulation parameter")?;
  MOD_SPECS[i].check(value)?; self.values[i]=value; Ok(())
 }
 pub fn active(&self)->bool { (0..MOD_SLOTS).any(|i|self.values[i*4]>=1.&&self.values[i*4+2]!=0.) }
 pub fn descs(&self,targets:usize)->Vec<ParamDesc> {
  let mut out=super::table_descs(MOD_SPECS,&self.values);
  for d in &mut out {if d.id.ends_with("_target"){d.max=targets.saturating_sub(1)as f64;d.step_count=d.max as i64;}}
  out
 }
}
pub struct Modulator {
 pub params:ModParams,
 phase:[f64;2],hold_lfo:[f64;2],seq:f64,seq_step:usize,seq_random:[f64;16],random:u64,random_value:f64,held:[u64;32],released:[u64;32],pedal:[bool;16],audio_gate:bool,last_input:f64,
 follower:f64,env:f64,release_start:f64,stage:u8,
 pub cc:[f64;128],pub velocity:f64,pub key:f64,pub pressure:f64,pub timbre:f64,pub bend:f64,
 pub tempo:f64, sample_rate:f64,
}
impl Modulator {
 pub fn new(sample_rate:f64)->Self { Self{params:ModParams::default(),phase:[0.;2],hold_lfo:[0.;2],seq:0.,seq_step:usize::MAX,seq_random:[0.;16],held:[0;32],released:[0;32],pedal:[false;16],audio_gate:false,last_input:0.,random:0x9e3779b97f4a7c15,
 random_value:0.,follower:0.,env:0.,release_start:0.,stage:0,cc:[0.;128],velocity:0.,key:0.,pressure:0.,timbre:0.,bend:0.,tempo:120.,sample_rate} }
 pub fn set_sample_rate(&mut self,rate:f64){self.sample_rate=rate;}
 pub fn note_on(&mut self,key:u8,velocity:f64){self.note_on_channel(0,key,velocity);}
 pub fn note_on_channel(&mut self,channel:u8,key:u8,velocity:f64){
  let was_held=self.held.iter().any(|v|*v!=0);let bit=(channel.min(15)as usize)*128+key.min(127)as usize;self.held[bit/64]|=1u64<<(bit%64);self.released[bit/64]&=!(1u64<<(bit%64));
  self.key=key as f64/127.;self.velocity=velocity;
  if self.params.values[154]!=1.{self.env=0.;self.stage=1;}
  for l in 0..2{let mode=self.params.values[147+l*5]as u8;if mode==1||(mode==2&&!was_held){self.phase[l]=0.;}}
  let mode=self.params.values[93]as u8;if mode==1||(mode==2&&!was_held){self.seq=0.;self.seq_step=usize::MAX;}
 }
 pub fn note_off_channel(&mut self,channel:u8,key:u8){
  let bit=(channel.min(15)as usize)*128+key.min(127)as usize;if self.pedal[channel.min(15)as usize]{self.released[bit/64]|=1u64<<(bit%64);}else{self.held[bit/64]&=!(1u64<<(bit%64));}
  if !self.held.iter().any(|v|*v!=0)&&!(self.params.values[154]!=0.&&self.audio_gate){self.release_start=self.env;self.stage=4;}
 }
 pub fn note_off(&mut self){self.held=[0;32];self.released=[0;32];self.pedal=[false;16];self.release_start=self.env;self.stage=4;}
 /// CC mappings write base parameters, not transient modulation offsets.
 pub fn control(&mut self,channel:u8,cc:u8,value:f64,writes:&mut[(usize,f64);MOD_SLOTS])->usize {
  if channel>15||cc>127||!value.is_finite(){return 0;}
  if self.params.values[163]==0.||self.params.values[163]as u8==channel+1{self.cc[cc as usize]=value.clamp(0.,1.);} if cc==74{self.timbre=value;}
  let first=channel as usize*2;
  match cc{
   64=>{self.pedal[channel as usize]=value>=0.5;if value<0.5{for index in first..first+2{self.held[index]&=!self.released[index];self.released[index]=0;}}},
   120=>{self.held[first..first+2].fill(0);self.released[first..first+2].fill(0);if !self.held.iter().any(|v|*v!=0){self.env=0.;self.stage=0;}},
   123=>{if self.pedal[channel as usize]{for index in first..first+2{self.released[index]=self.held[index];}}else{self.held[first..first+2].fill(0);}},
   121=>{self.cc=[0.;128];self.bend=0.;self.pressure=0.;self.timbre=0.;self.pedal[channel as usize]=false;
    for index in first..first+2{self.held[index]&=!self.released[index];self.released[index]=0;}},
   _=>{}
  }
  if matches!(cc,64|121|123)&&!self.held.iter().any(|v|*v!=0)&&!(self.params.values[154]!=0.&&self.audio_gate){self.release_start=self.env;self.stage=4;}
  let mut n=0; for slot in 0..8 {let i=63+slot*3;let ch=self.params.values[i+2]as u8;
   if self.params.values[i]as i16==cc as i16&&(ch==0||ch==channel+1){writes[n]=(self.params.values[i+1]as usize,value);n+=1;}
  } n
 }
 pub fn sources(&mut self,frames:usize,input:f64)->[f64;15] {
  let p=&self.params.values;let seconds=frames as f64/self.sample_rate;
  let mut src=[0.;15];
  for l in 0..2 {let i=32+l*3;let e=144+l*5;let sync=p[i+2].round()as usize;
   let hz=if sync==0 {p[i]}else{self.tempo/60./[0.,0.0625,0.125,0.25,0.5,1.,2.,4.,8.][sync.min(8)]/p[e+2]};
   let advance=self.phase[l]+seconds*hz;
   for _ in 0..(advance.floor()as usize).min(65536){self.hold_lfo[l]=next_random(&mut self.random);}
   self.phase[l]=advance.fract();
   let x=balanced((self.phase[l]+p[e]*0.01).fract(),p[e+1]);
   src[l+1]=(match p[i+1].round()as u8{1=>1.-4.*(x-0.5).abs(),2=>2.*x-1.,3=>1.-2.*x,4=>if x<0.5{1.}else{-1.},5=>self.hold_lfo[l],_=>(std::f64::consts::TAU*x).sin()})*p[e+4];
  }
  let input=input.abs().max(0.);
  let mode=p[154]as u8;let threshold=10f64.powf(p[155]/20.);
  let gate=if self.audio_gate{input>=threshold*10f64.powf(-p[156]/20.)}else{input>=threshold};
  if mode!=0&&gate&&!self.audio_gate{self.env=0.;self.stage=1;}
  if mode!=0&&!gate&&self.audio_gate&&(mode==1||!self.held.iter().any(|v|*v!=0)){self.release_start=self.env;self.stage=4;}
  self.audio_gate=gate;
  let level=input*10f64.powf(p[160]/20.);
  let target=if p[159]==1.{((level-self.last_input)*16.).max(0.)}else{level}.clamp(0.,1.);self.last_input=level;
  let tau=if target>self.follower{p[42]}else{p[43]}*0.001;
  self.follower=target+(self.follower-target)*(-seconds/tau.max(0.0001)).exp();
  match self.stage {1=>{self.env+=seconds/(p[38]*0.001);if self.env>=1.{self.env=1.;self.stage=2;}},
   2=>{let sustain=p[40]*0.01;self.env-=seconds*(1.-sustain)/(p[39]*0.001);if self.env<=sustain{self.env=sustain;self.stage=3;}},
   3=>self.env=p[40]*0.01,4=>{self.env=(self.env-self.release_start*seconds/(p[41]*0.001)).max(0.);if self.env==0.{self.stage=0;}},_=>{}}
  let length=p[87].round()as usize;let sync=p[88].round()as usize;
  let hz=if sync==0{p[46]/length as f64}else{self.tempo/60./[0.,0.0625,0.125,0.25,0.5,1.,2.,4.,8.,16.,32.,64.,0.3333333333333333,0.75][sync.min(13)]/p[89]};
  self.seq=(self.seq+seconds*hz).fract();
  let position=balanced((self.seq+p[92]*0.01).fract(),p[90])*length as f64;
  let step=(position.floor()as usize).min(length-1);let next=(step+1)%length;
  if step!=self.seq_step{self.seq_step=step;self.random_value=next_random(&mut self.random);self.seq_random[step]=self.random_value;}
  let value=|i:usize|if p[128+i]==1.{self.seq_random[i]}else{p[47+i]*0.01};
  let glide=(p[91]+p[96+step]).clamp(0.,1.);let mut t=if glide==0.{0.}else{((position.fract()-(1.-glide))/glide).clamp(0.,1.)};
  t=match p[112+step]as u8{1=>t*t,2=>t.sqrt(),3=>0.5-0.5*(t*std::f64::consts::PI).cos(),_=>t};
  let sequence=value(step)+(value(next)-value(step))*t;
  src[3]=self.follower*p[161];src[4]=(self.env-if p[157]==1.{p[40]*0.01}else{0.})*p[158];
  src[5]=self.cc[p[162].round()as usize];src[6]=self.velocity;src[7]=self.key;
  src[8]=self.pressure;src[9]=self.timbre;src[10]=p[44]*0.01;src[11]=p[45]*0.01;src[12]=self.random_value;src[13]=self.bend;
  src[14]=(if p[94]==1.{(sequence*12.).round()/12.}else{sequence})*p[95];src
 }
 pub fn apply(&self,sources:&[f64;15],base:&[f64],descs:&[ParamDesc],writes:&mut[(usize,f64);MOD_SLOTS])->usize {
  let mut n=0;
  for slot in 0..MOD_SLOTS {let i=slot*4;let source=self.params.values[i].round()as usize;if source==0{continue;}
   let target=self.params.values[i+1].round()as usize;let Some(d)=descs.get(target)else{continue};let Some(&initial)=base.get(target)else{continue};
   let found=(0..n).find(|&i|writes[i].0==target);let original=found.map_or(initial,|i|writes[i].1);
   let v=sources[source.min(14)];let curve=self.params.values[i+3];let shaped=v.signum()*v.abs().powf(2f64.powf(curve*3.));
   let normalized=(normalize(d,original)+shaped*self.params.values[i+2]*0.01).clamp(0.,1.);let value=denormalize(d,normalized);
   if let Some(i)=found{writes[i].1=value;}else{writes[n]=(target,value);n+=1;}
  } n
 }
}
fn next_random(state:&mut u64)->f64{*state^=*state<<13;*state^=*state>>7;*state^=*state<<17;(*state>>11)as f64/(1u64<<53)as f64*2.-1.}
fn balanced(phase:f64,balance:f64)->f64{let middle=(0.5+balance*0.49).clamp(0.005,0.995);if phase<middle{phase/middle*0.5}else{0.5+(phase-middle)/(1.-middle)*0.5}}
pub fn normalize(d:&ParamDesc,value:f64)->f64 {
 if d.min>0.&&(d.unit=="Hz"||d.unit=="ms"||d.unit=="s"){(value/d.min).ln()/(d.max/d.min).ln()}
 else {(value-d.min)/(d.max-d.min).max(f64::EPSILON)}
}
pub fn denormalize(d:&ParamDesc,value:f64)->f64 {
 let v=if d.min>0.&&(d.unit=="Hz"||d.unit=="ms"||d.unit=="s"){d.min*(d.max/d.min).powf(value)}
 else {d.min+(d.max-d.min)*value};
 if d.step_count>0 {let step=(d.max-d.min)/d.step_count as f64; (d.min+((v-d.min)/step).round()*step).clamp(d.min,d.max)}else{v.clamp(d.min,d.max)}
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn matrix_sums_routes_and_cc_respects_channel(){
 let d=ParamDesc{id:"cutoff".into(),name:"Frequency".into(),min:20.,max:20000.,value:200.,unit:"Hz".into(),step_count:0,default_value:Some(200.)};
 let mut m=Modulator::new(48000.);m.params.set("mod1_source",10.).unwrap();m.params.set("mod1_amount",20.).unwrap();m.params.set("mxy_x",100.).unwrap();
 let s=m.sources(32,0.);let mut writes=[(0,0.);8];assert_eq!(m.apply(&s,&[200.],&[d.clone()],&mut writes),1);assert!((writes[0].1-796.214341).abs()<0.001);
 m.params.set("midi1_cc",74.).unwrap();m.params.set("midi1_channel",2.).unwrap();assert_eq!(m.control(0,74,0.5,&mut writes),0);assert_eq!(m.control(1,74,0.5,&mut writes),1);
 }
 #[test]fn sources_follow_audio_trigger_and_sequence(){let mut m=Modulator::new(48000.);m.note_on(60,0.8);let a=m.sources(480,0.5);assert!(a[3]>0.3&&a[4]>0.9);assert_eq!(a[6],0.8);m.note_off();assert!(m.sources(48000,0.)[4]<0.001);}
}

#[cfg(test)]
mod advanced_tests {
 use super::*;
 #[test]
 fn held_random_lfos_follow_their_own_frequency_and_legato_tracks_channels(){
  let mut m=Modulator::new(48000.);
  m.params.set("mlfo1_shape",5.).unwrap();m.params.set("mlfo2_shape",5.).unwrap();
  m.params.set("mlfo1_rate",1.).unwrap();m.params.set("mlfo2_rate",2.).unwrap();
  let v=m.sources(36000,0.);assert_eq!(v[1],0.);assert_ne!(v[2],0.);
  let v=m.sources(12000,0.);assert_ne!(v[1],0.);
  m.params.set("mlfo1_shape",2.).unwrap();m.params.set("mlfo1_retrigger",2.).unwrap();
  m.note_on_channel(1,60,0.8);assert!((m.sources(12000,0.)[1]+0.5).abs()<1e-10);
  m.note_on_channel(2,60,0.6);assert!(m.sources(12000,0.)[1].abs()<1e-10);
  m.note_off_channel(1,60);m.note_on_channel(2,64,0.6);assert!((m.sources(12000,0.)[1]-0.5).abs()<1e-10);
  m.note_off_channel(2,60);m.note_off_channel(2,64);m.note_on_channel(3,60,0.7);
  assert!((m.sources(12000,0.)[1]+0.5).abs()<1e-10);
 }
 #[test]
 fn editable_sequence_interpolates_shapes_snaps_and_follows_tempo(){
  let mut m=Modulator::new(48000.);
  m.params.set("mseq_length",2.).unwrap();m.params.set("mseq_glide1",1.).unwrap();
  let a=m.sources(6000,0.)[14];assert!((a-0.5).abs()<1e-10);
  m.params.set("mseq_curve1",1.).unwrap();assert!((m.sources(0,0.)[14]-0.875).abs()<1e-10);
  m.params.set("mseq_snap",1.).unwrap();assert!((m.sources(0,0.)[14]*12.).fract().abs()<1e-10);
  m.params.set("mseq_sync",5.).unwrap();m.params.set("mseq_glide", -1.).unwrap();
  m.params.set("mseq_retrigger",1.).unwrap();m.note_on(60,1.);
  m.tempo=120.;assert_eq!(m.sources(12000,0.)[14],-1.);
  m.note_on(60,1.);m.tempo=60.;assert_eq!(m.sources(12000,0.)[14],1.);
 }
 #[test]
 fn audio_threshold_envelope_neutral_sustain_and_midi_source_selection_work(){
  let mut m=Modulator::new(48000.);
  m.params.set("menv_trigger",1.).unwrap();m.params.set("menv_range",1.).unwrap();
  assert!(m.sources(480,0.5)[4]>0.29);
  assert!(m.sources(12000,0.5)[4].abs()<1e-9);
  assert!(m.sources(24000,0.)[4]< -0.69);
  m.params.set("mmidi_cc",74.).unwrap();m.params.set("mmidi_channel",2.).unwrap();
  let mut writes=[(0,0.);8];m.control(0,74,0.3,&mut writes);assert_eq!(m.sources(0,0.)[5],0.);
  m.control(1,74,0.6,&mut writes);assert_eq!(m.sources(0,0.)[5],0.6);
 }
}
