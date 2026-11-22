//! The same bounded voice engine serves score rendering and native MIDI monitoring.
use super::{SynthParams,SynthNote,SYNTH_SPECS,SYNTH_ID,ONE_SYNTH_ID,instrument_specs,db_to_lin};
use super::synth::{osc,Svf};
use super::modulation::{Modulator,denormalize};
use super::quality::Oversampler;
use crate::proto::{self,ParamDesc,ParamEvent};
use std::f64::consts::{TAU,PI};

/// A native editor control change, distinct from sample automation. The low
/// bits are the stable visible index returned by instrument_specs().
pub const SYNTH_CONTROL_EVENT:u32=1<<28;
/// Native editor auditions have separate ownership from external MIDI.
/// OR NOTE onto a channel-zero proto::note_on/off id.
pub const SYNTH_PREVIEW_NOTE:u32=1<<27;
pub const SYNTH_PREVIEW_RELEASE:u32=(1<<27)|(1<<26);
pub const SYNTH_PREVIEW_PANIC:u32=(1<<27)|(1<<25);
/// Clear measurements while leaving the performance and audio clocks intact.
/// This private event is admitted only with value zero.
pub const SYNTH_RESET_METERS:u32=1<<26;

struct Voice {
 active:bool,preview:bool,key:u8,channel:u8,pitch:f64,velocity:f64,poly_pressure:f64,age:u64,held:bool,sustained:bool,
 release_age:u64,release_level:f64,previous_pitch:f64,serial:u64,
 phase:[[[f64;2];4];8],filters:[[Svf;2];4],noise:[u64;2],quality:[Oversampler;2],
}
impl Voice {
 fn new()->Self {Self{active:false,preview:false,key:0,channel:0,pitch:60.,velocity:0.,poly_pressure:0.,age:0,held:false,sustained:false,
 release_age:0,release_level:0.,previous_pitch:60.,serial:0,phase:[[[0.;2];4];8],
 filters:[[Svf::default();2];4],noise:[1,2],quality:std::array::from_fn(|_|Oversampler::new())}}
 fn level(&self,c:&Config,rate:f64)->f64 {
  let t=self.age as f64/rate;
  if self.held||self.sustained {if t<c.adsr[0]{t/c.adsr[0]}else if t<c.adsr[0]+c.adsr[1]{1.-(1.-c.adsr[2])*(t-c.adsr[0])/c.adsr[1]}else{c.adsr[2]}}
  else{self.release_level*(1.-self.release_age as f64/(c.adsr[3]*rate)).max(0.)}
 }
 fn off(&mut self,c:&Config,rate:f64){self.release_level=self.level(c,rate);self.held=false;self.sustained=false;self.release_age=0;}
}
struct Config {
 waves:[usize;4],semis:[f64;4],mix:[f64;4],unison:usize,detune:f64,spread:f64,noise:f64,width:f64,
 adsr:[f64;4],lfo:[f64;3],pulse:f64,glide:f64,keytrack:f64,velocity:f64,
 cuts:[f64;4],q:[f64;4],shapes:[usize;4],enabled:[bool;4],pan:[f64;4],parallel:bool,character:usize,model:usize,
 env:f64,gain:f64,drive:f64,quality:usize,mpe:bool,bend:f64,mpe_range:f64,
}
impl Config {
 fn from(p:&SynthParams)->Self {Self{
 waves:["osc1","osc2","osc3","osc4"].map(|id|p.get(id).round()as usize),
 semis:[0.,p.get("osc2_semi"),p.get("osc3_semi"),p.get("osc4_semi")],
 mix:[1.-p.get("osc2_mix")*0.005,p.get("osc2_mix")*0.01,p.get("osc3_mix")*0.01,p.get("osc4_mix")*0.01],
 unison:p.get("unison").round().clamp(1.,8.)as usize,detune:p.get("detune"),spread:p.get("unison_detune"),
 noise:p.get("noise")*0.01,width:p.get("width")*0.01,
 adsr:[p.get("attack")*0.001,p.get("decay")*0.001,p.get("sustain")*0.01,p.get("release")*0.001],
 lfo:[p.get("lfo_rate"),p.get("lfo_pitch"),p.get("lfo_filter")],pulse:p.get("pulse_width")*0.01,glide:p.get("portamento")*0.001,
 keytrack:p.get("keytrack")*0.01,velocity:p.get("velocity")*0.01,
 cuts:["cutoff","cutoff2","cutoff3","cutoff4"].map(|id|p.get(id)),
 q:["resonance","resonance2","resonance3","resonance4"].map(|id|p.get(id)),
 shapes:["filter_shape","filter2_shape","filter3_shape","filter4_shape"].map(|id|p.get(id).round()as usize),
 enabled:[true,p.get("filter2_enabled")>=0.5,p.get("filter3_enabled")>=0.5,p.get("filter4_enabled")>=0.5],
 pan:["filter1_pan","filter2_pan","filter3_pan","filter4_pan"].map(|id|p.get(id)*0.01),
 parallel:p.get("filter_routing")>=0.5,character:p.get("filter_character")as usize,model:p.get("osc_model")as usize,
 env:p.get("env_amount")*0.04,gain:db_to_lin(p.get("gain")),drive:db_to_lin(p.get("drive")),quality:p.get("high_quality")as usize,
 mpe:p.get("mpe")>=0.5,bend:p.get("bend_range"),mpe_range:p.get("mpe_range"),
 }}
}
struct SynthFx {
 delay:[Vec<f64>;2],chorus:[Vec<f64>;2],reverb:[Vec<f64>;4],head:usize,rev_head:[usize;4],
 damp:[f64;4],allpass:[[f64;6];2],phase:f64,feedback:[f64;2],env:f64,
}
impl SynthFx {
 fn new(rate:f64)->Self {Self{delay:std::array::from_fn(|_|vec![0.;(rate*25.)as usize+1]),
 chorus:std::array::from_fn(|_|vec![0.;(rate*0.1)as usize+1]),
 reverb:[0.0297,0.0371,0.0411,0.0437].map(|t|vec![0.;(rate*t)as usize+1]),head:0,rev_head:[0;4],
 damp:[0.;4],allpass:[[0.;6];2],phase:0.,feedback:[0.;2],env:0.}}
 fn reset(&mut self){for x in &mut self.delay{x.fill(0.);}for x in &mut self.chorus{x.fill(0.);}for x in &mut self.reverb{x.fill(0.);}self.allpass=[[0.;6];2];self.env=0.;}
 fn read(line:&[f64],head:usize,delay:f64)->f64{let p=(head as f64-delay).rem_euclid(line.len()as f64);let i=p as usize;line[i]*(1.-p.fract())+line[(i+1)%line.len()]*p.fract()}
 fn run(&mut self,mut x:[f64;2],p:&SynthParams,rate:f64,tempo:f64)->[f64;2] {
  // These fixed indices are appended after the immutable original synth table.
  let v=&p.values[65..];
  if v[0]>0.{let d=1.+v[0]*0.5;for y in &mut x{let driven=*y*d;let wet=match v[1]as u8{1=>driven/(1.+driven.abs()),2=>(driven*PI).sin(),3=>driven.clamp(-1.,1.),_=>driven.tanh()};*y+=v[0]*0.01*(wet-*y);}}
  self.phase=(self.phase+1./rate).rem_euclid(10000.);
  for ch in 0..2 {
   let cpos=self.head%self.chorus[ch].len();self.chorus[ch][cpos]=x[ch];
   if v[2]>0.{let delay=(12.+v[4]*(TAU*self.phase*v[3]+ch as f64*1.7).sin())*0.001*rate;
    let wet=Self::read(&self.chorus[ch],cpos,delay.max(1.));x[ch]+=v[2]*0.005*(wet-x[ch]);}
   if v[5]>0.{
    let wet=if v[8]>=0.5{Self::read(&self.chorus[ch],cpos,(1.5+(TAU*self.phase*v[6]).sin())*0.001*rate)}
    else{let hz=700.*2f64.powf((TAU*self.phase*v[6]+ch as f64*0.7).sin()*2.);
     let g=(PI*hz/rate).tan();let a=(1.-g)/(1.+g);let mut y=x[ch]+self.feedback[ch]*v[7]*0.01;
     for stage in &mut self.allpass[ch]{let out=-a*y+*stage;*stage=y+a*out;y=out;}y};
    self.feedback[ch]=wet.clamp(-4.,4.);x[ch]+=v[5]*0.005*(wet-x[ch]);
   }
   if v[9]>0.{let sync=v[12]as usize;let delay=if sync>0{60./tempo*[0.,0.0625,0.125,0.25,0.5,1.,2.,4.,8.][sync.min(8)]}else{v[10]*0.001};
    let pos=self.head%self.delay[ch].len();let wet=Self::read(&self.delay[ch],pos,(delay*rate).min(self.delay[ch].len()as f64-1.));
    self.delay[ch][pos]=x[ch]+wet*v[11]*0.01;x[ch]+=wet*v[9]*0.01;
   }
  }
  if v[13]>0.{let mut wet=[0.;4];for i in 0..4{let delayed=self.reverb[i][self.rev_head[i]];self.damp[i]+= (0.02+v[15]*0.009)*(delayed-self.damp[i]);wet[i]=self.damp[i];}
   let sum=wet.iter().sum::<f64>()*0.5;for i in 0..4{let feedback=10f64.powf(-3.*self.reverb[i].len()as f64/(rate*v[14].max(0.1)));
    self.reverb[i][self.rev_head[i]]=(x[i%2]*0.5+feedback*(sum-wet[i])).clamp(-8.,8.);self.rev_head[i]=(self.rev_head[i]+1)%self.reverb[i].len();}
   x[0]+=(wet[0]+wet[2])*v[13]*0.005;x[1]+=(wet[1]+wet[3])*v[13]*0.005;
  }
  if v[16]>0.{let peak=x[0].abs().max(x[1].abs());let tau=if peak>self.env{0.005}else{0.1};self.env=peak+(self.env-peak)*(-1./(rate*tau)).exp();
   let db=20.*self.env.max(1e-12).log10();let reduction=(db-v[17]).max(0.)*(1.-1./v[18]);let gain=db_to_lin(-reduction*v[16]*0.01);x[0]*=gain;x[1]*=gain;
  }
  self.head=self.head.wrapping_add(1);x
 }
}

/// All allocations happen in new(); events and process are bounded and allocation-free.
pub struct RealtimeSynth {
 pub telemetry:std::sync::Arc<crate::telemetry::LiveTelemetry>,capture:crate::telemetry::TelemetryCapture,
 pub params:SynthParams,sample_rate:f64,voices:Vec<Voice>,config:Config,modulator:Modulator,
 descs:Vec<ParamDesc>,targets:Vec<usize>,base:Vec<f64>,values:Vec<f64>,control_specs:Vec<super::Spec>,
 bend:[f64;16],pressure:[f64;16],timbre:[f64;16],sustain:[bool;16],
 serial:u64,clock:u64,previous_pitch:f64,fx:SynthFx,
 held:[[f64;128];16],held_order:[[u64;128];16],held_pitch:Vec<f64>,physical:[u64;32],preview_physical:[u64;2],preview_held:[f64;128],preview_pitch:[f64;128],preview_order:[u64;128],arp_active:bool,latch_mode:u8,arp_until:f64,arp_release:f64,arp_step:usize,arp_voice:Option<(bool,u8,u8)>,tempo:f64,
}
impl RealtimeSynth {
 pub fn new(params:&SynthParams,sample_rate:f64)->Self{Self::for_instrument(SYNTH_ID,params,sample_rate)}
 pub fn for_instrument(id:&str,params:&SynthParams,sample_rate:f64)->Self{
  let mut params=params.clone();if id==ONE_SYNTH_ID{params=SynthParams::from_json_for(id,Some(&params.to_json()));}
  let specs=instrument_specs(id);let targets:Vec<usize>=specs.iter().filter_map(|s|SYNTH_SPECS.iter().position(|p|p.id==s.id)).collect();
  let base:Vec<f64>=targets.iter().map(|&i|params.values[i]).collect();let base_specs=specs[..targets.len()].to_vec();
  let descs=super::table_descs(&base_specs,&base);let values=params.values.clone();let config=Config::from(&params);
  let mut modulator=Modulator::new(sample_rate);modulator.params=params.modulation.clone();
  let tempo=params.get("arp_tempo");let arp_active=params.get("arp_on")>=0.5;let latch_mode=params.get("arp_latch")as u8;
  Self{telemetry:std::sync::Arc::new(crate::telemetry::LiveTelemetry::new()),capture:crate::telemetry::TelemetryCapture::new(sample_rate).expect("validated synth rate"),params,sample_rate,voices:(0..64).map(|_|Voice::new()).collect(),config,modulator,descs,targets,base,values,control_specs:specs,
   bend:[0.;16],pressure:[0.;16],timbre:[0.;16],sustain:[false;16],serial:0,clock:0,previous_pitch:60.,
   fx:SynthFx::new(sample_rate),held:[[0.;128];16],held_order:[[0;128];16],held_pitch:vec![0.0;2048],physical:[0;32],preview_physical:[0;2],preview_held:[0.0;128],preview_pitch:[0.0;128],preview_order:[0;128],arp_active,latch_mode,arp_until:0.,arp_release:0.,arp_step:0,arp_voice:None,tempo}
 }
 pub fn set_tempo(&mut self,bpm:f64){if bpm.is_finite()&&(20.0..=1000.0).contains(&bpm){self.tempo=bpm;self.modulator.tempo=bpm;}}
 pub fn reset(&mut self){
  for voice in &mut self.voices{voice.active=false;for quality in &mut voice.quality{quality.reset();}}
  self.sustain=[false;16];self.bend=[0.0;16];self.pressure=[0.0;16];self.timbre=[0.0;16];self.held=[[0.;128];16];self.held_order=[[0;128];16];self.held_pitch.fill(0.0);self.physical=[0;32];self.preview_physical=[0;2];self.preview_held=[0.0;128];self.preview_pitch=[0.0;128];self.preview_order=[0;128];self.arp_active=self.params.get("arp_on")>=0.5;self.latch_mode=self.params.get("arp_latch")as u8;
  self.arp_voice=None;self.arp_until=0.0;self.arp_release=0.0;self.arp_step=0;self.clock=0;self.serial=0;self.previous_pitch=60.0;
  let modulation=self.modulator.params.clone();self.modulator=Modulator::new(self.sample_rate);self.modulator.params=modulation;self.modulator.tempo=self.tempo;self.fx.reset();
 }
 fn start_note(&mut self,channel:u8,key:u8,pitch:f64,velocity:f64,preview:bool){
  let limit=self.params.get("voices").round().clamp(1.,64.)as usize;
  let index=self.voices[..limit].iter().position(|v|!v.active).unwrap_or_else(||self.voices[..limit].iter().enumerate().min_by_key(|(_,v)|v.serial).unwrap().0);
  self.serial=self.serial.wrapping_add(1);let v=&mut self.voices[index];
  v.active=true;v.preview=preview;v.key=key;v.channel=channel;v.pitch=pitch;v.previous_pitch=self.previous_pitch;self.previous_pitch=pitch;v.velocity=velocity;v.poly_pressure=0.;v.age=0;v.held=true;v.sustained=false;v.release_age=0;v.serial=self.serial;
  v.filters=[[Svf::default();2];4];let seed=(pitch*7.31+self.clock as f64*1e-5).fract();
  for u in 0..8{for o in 0..4{for c in 0..2{let offset=u as f64*0.61803398875;v.phase[u][o][c]=if o==0{offset.fract()}else{(seed+offset+c as f64*0.37+(o-1)as f64*0.271).fract()};}}}
  v.noise=[self.clock^pitch.to_bits()^0x9e3779b97f4a7c15,self.clock^pitch.to_bits()^0x517cc1b727220a95];
  for quality in &mut v.quality{quality.set_quality(self.config.quality);quality.reset();}
  self.modulator.note_on_channel(channel,key,velocity);
 }
 fn release_modulator_key(&mut self,channel:u8,key:u8){
  if !self.voices.iter().any(|voice|voice.active&&voice.held&&voice.channel==channel&&voice.key==key){
   self.modulator.note_off_channel(channel,key);
  }
 }
 fn any_held(&self)->bool{
  self.held.iter().any(|row|row.iter().any(|value|*value>0.0))||self.preview_held.iter().any(|value|*value>0.0)
 }
 fn stop_arp_voice(&mut self){
  if let Some((preview,channel,key))=self.arp_voice.take(){
   for voice in &mut self.voices{if voice.active&&voice.preview==preview&&voice.channel==channel&&voice.key==key&&voice.held{voice.off(&self.config,self.sample_rate);}}
   self.release_modulator_key(channel,key);
  }
 }
 fn clear_channel(&mut self,channel:u8,immediate:bool){
  let channel_index=channel as usize;self.held[channel_index]=[0.0;128];self.held_order[channel_index]=[0;128];
  self.physical[channel_index*2]=0;self.physical[channel_index*2+1]=0;self.sustain[channel_index]=false;
  if self.arp_voice.is_some_and(|(preview,ch,_)|!preview&&ch==channel){self.arp_voice=None;}
  for voice in &mut self.voices{if !voice.preview&&voice.channel==channel&&voice.active{
   if immediate{voice.active=false;}else if voice.held||voice.sustained{voice.off(&self.config,self.sample_rate);}
  }}
  for key in 0..128{self.release_modulator_key(channel,key);}
 }
 pub fn note_on(&mut self,channel:u8,key:u8,pitch:f64,velocity:f64){
  if channel>15||key>127||!pitch.is_finite()||!velocity.is_finite(){return;}
  if velocity<=0.0{self.note_off(channel,key);return;}
  let arpeggiated=self.params.get("arp_on")>=0.5;
  let latch=self.params.get("arp_latch")as u8;
  // Hold replaces the latched chord only after every physical key is up.
  // Add mode retains it. Generated arpeggio voices never count as keys.
  if arpeggiated&&latch==1&&self.physical.iter().all(|&bits|bits==0){
   self.held=[[0.0;128];16];self.held_order=[[0;128];16];if self.arp_voice.is_some_and(|(preview,_,_)|!preview){self.stop_arp_voice();}
  }
  let empty=!self.any_held();
  let note=channel as usize*128+key as usize;self.physical[note/64]|=1u64<<(note%64);self.held_pitch[note]=pitch;
  self.held[channel as usize][key as usize]=velocity.clamp(0.0,1.0);self.serial=self.serial.wrapping_add(1);self.held_order[channel as usize][key as usize]=self.serial;
  if !arpeggiated{self.start_note(channel,key,pitch,velocity,false);}
  else if empty{self.arp_until=self.clock as f64;self.arp_step=0;}
 }
 pub fn note_off(&mut self,channel:u8,key:u8){
  if channel>15||key>127{return;}
  let note=channel as usize*128+key as usize;self.physical[note/64]&=!(1u64<<(note%64));
  let arpeggiated=self.params.get("arp_on")>=0.5;let latch=self.params.get("arp_latch")as u8;
  if !arpeggiated||latch==0{self.held[channel as usize][key as usize]=0.0;}
  if !arpeggiated||latch==0{
   for voice in &mut self.voices{if voice.active&&!voice.preview&&voice.channel==channel&&voice.key==key&&voice.held{
    if self.sustain[channel as usize]{voice.sustained=true;voice.held=false;}else{voice.off(&self.config,self.sample_rate);}
   }}
   self.release_modulator_key(channel,key);
   if arpeggiated&&!self.any_held(){self.stop_arp_voice();}
  }
 }
 /// Screen/computer-keyboard notes are isolated from external MIDI owners.
 fn preview_note_on(&mut self,key:u8,velocity:f64){
  if velocity<=0.0{self.preview_note_off(key);return;}
  let arpeggiated=self.params.get("arp_on")>=0.5;
  if arpeggiated&&self.params.get("arp_latch")==1.0&&self.preview_physical.iter().all(|&bits|bits==0){
   self.preview_held=[0.0;128];self.preview_order=[0;128];
   if self.arp_voice.is_some_and(|(preview,_,_)|preview){self.stop_arp_voice();}
  }
  let empty=!self.any_held();let note=key as usize;
  self.preview_physical[note/64]|=1u64<<(note%64);self.preview_held[note]=velocity;
  self.preview_pitch[note]=key as f64;self.serial=self.serial.wrapping_add(1);self.preview_order[note]=self.serial;
  if arpeggiated{if empty{self.arp_until=self.clock as f64;self.arp_step=0;}}
  else{self.start_note(0,key,key as f64,velocity,true);}
 }
 fn preview_note_off(&mut self,key:u8){
  let note=key as usize;self.preview_physical[note/64]&=!(1u64<<(note%64));
  let arpeggiated=self.params.get("arp_on")>=0.5;
  if !arpeggiated||self.params.get("arp_latch")==0.0{
   self.preview_held[note]=0.0;
   for voice in &mut self.voices{if voice.active&&voice.preview&&voice.key==key&&voice.held{voice.off(&self.config,self.sample_rate);}}
   self.release_modulator_key(0,key);
   if arpeggiated&&!self.any_held(){self.stop_arp_voice();}
  }
 }
 fn release_preview(&mut self,immediate:bool){
  self.preview_physical=[0;2];self.preview_held=[0.0;128];self.preview_order=[0;128];
  let was_arp=self.arp_voice.is_some_and(|(preview,_,_)|preview);
  if was_arp{self.stop_arp_voice();}
  for voice in &mut self.voices{if voice.active&&voice.preview{
   if immediate{voice.active=false;}else if voice.held||voice.sustained{voice.off(&self.config,self.sample_rate);}
  }}
  for key in 0..128{self.release_modulator_key(0,key);}
  if was_arp&&self.any_held(){self.arp_until=self.clock as f64;}
 }
 fn valid_preview_event(id:u32,value:f64)->bool{
  if id==SYNTH_PREVIEW_RELEASE||id==SYNTH_PREVIEW_PANIC{return value==0.0;}
  id&SYNTH_PREVIEW_NOTE!=0&&id&0xf0000==0&&proto::valid_note_event(id&!SYNTH_PREVIEW_NOTE,value)
 }
 /// End a score/transport while allowing oscillator releases and FX tails.
 pub fn release_all(&mut self){
  self.held=[[0.0;128];16];self.physical=[0;32];self.preview_physical=[0;2];self.preview_held=[0.0;128];self.sustain=[false;16];self.arp_voice=None;
  for voice in &mut self.voices{if voice.active&&(voice.held||voice.sustained){voice.off(&self.config,self.sample_rate);}}
  self.modulator.note_off();
 }
 fn control_index(&self,id:u32,value:f64)->Option<usize>{
  let control=id&SYNTH_CONTROL_EVENT!=0;
  let index=(id&!SYNTH_CONTROL_EVENT)as usize;
  let spec=self.control_specs.get(index)?;
  if !spec.accepts(value)||(!control&&!spec.automatable){return None;}
  if spec.id.ends_with("_target")&&!self.control_specs.get(value as usize).is_some_and(|target|(value as usize)<self.targets.len()&&target.automatable){return None;}
  Some(index)
 }
 /// Admission checks never construct error strings or modify DSP state.
 pub fn valid_event(&self,id:u32,value:f64)->bool{
  (id==SYNTH_RESET_METERS&&value==0.0)||Self::valid_preview_event(id,value)||self.control_index(id,value).is_some()||proto::valid_note_event(id,value)||proto::decode_midi(id,value).is_some()
   ||(id==super::TRANSPORT_TEMPO_EVENT&&value.is_finite()&&(20.0..=1000.0).contains(&value))
 }
 fn apply_control(&mut self,index:usize,value:f64){
  if let Some(&native)=self.targets.get(index){
   self.values[native]=value;self.base[index]=value;self.descs[index].value=value;self.params.values[native]=value;
   self.config=Config::from(&self.params);
   if self.control_specs[index].id=="high_quality"{
    for voice in &mut self.voices{for quality in &mut voice.quality{quality.set_quality(self.config.quality);}}
   }
  }else{
   let modulation=index-self.targets.len();
   self.modulator.params.values[modulation]=value;self.params.modulation.values[modulation]=value;
  }
 }
 pub fn event(&mut self,id:u32,value:f64)->bool{
  if id==SYNTH_RESET_METERS{
   if value!=0.0{return false;}
   self.capture.reset_meters();
   // A manual reset must be visible immediately even if the 30 Hz regular
   // publication interval has not elapsed or no new frames arrive.
   self.telemetry.publish(&self.capture);return true;
  }
  if Self::valid_preview_event(id,value){
   if id==SYNTH_PREVIEW_RELEASE||id==SYNTH_PREVIEW_PANIC{self.release_preview(id==SYNTH_PREVIEW_PANIC);}
   else{match proto::decode_note(id&!SYNTH_PREVIEW_NOTE,value).unwrap(){
    proto::NoteEvent::On{key,velocity}=>self.preview_note_on(key,velocity),proto::NoteEvent::Off{key}=>self.preview_note_off(key)
   }}return true;
  }
  if let Some(index)=self.control_index(id,value){self.apply_control(index,value);return true;}
  if id==super::TRANSPORT_TEMPO_EVENT{
   if !value.is_finite()||!(20.0..=1000.0).contains(&value){return false;}self.set_tempo(value);return true;
  }
  if proto::valid_note_event(id,value){let channel=((id>>16)&15)as u8;match proto::decode_note(id,value).unwrap(){
   proto::NoteEvent::On{key,velocity}=>self.note_on(channel,key,key as f64,velocity),proto::NoteEvent::Off{key}=>self.note_off(channel,key)}return true;}
  let Some(event)=proto::decode_midi(id,value)else{return false};
  match event{
   proto::MidiEvent::Control{channel,controller,value}=>{
    let mut writes=[(0,0.);8];let n=self.modulator.control(channel,controller,value,&mut writes);
    for &(target,normalized)in &writes[..n]{if let(Some(&i),Some(d))=(self.targets.get(target),self.descs.get(target)){if SYNTH_SPECS[i].automatable{
     let value=denormalize(d,normalized);self.values[i]=value;self.base[target]=value;
    }}}
    match controller{64=>{self.sustain[channel as usize]=value>=0.5;if value<0.5{for v in &mut self.voices{if !v.preview&&v.channel==channel&&v.sustained{v.off(&self.config,self.sample_rate);}}}},
     74=>self.timbre[channel as usize]=value,120=>self.clear_channel(channel,true),
     121=>{self.modulator.cc=[0.0;128];self.modulator.bend=0.0;self.modulator.pressure=0.0;self.modulator.timbre=0.0;self.bend[channel as usize]=0.;self.pressure[channel as usize]=0.;self.timbre[channel as usize]=0.;self.sustain[channel as usize]=false;for v in &mut self.voices{if !v.preview&&v.channel==channel&&v.sustained{v.off(&self.config,self.sample_rate);}}},
     123=>self.clear_channel(channel,false),_=>{}}
   },
   proto::MidiEvent::PitchBend{channel,value}=>{self.bend[channel as usize]=value;self.modulator.bend=value;},
   proto::MidiEvent::Pressure{channel,value}=>{self.pressure[channel as usize]=value;self.modulator.pressure=value;},
   proto::MidiEvent::PolyPressure{channel,key,value}=>{for v in &mut self.voices{if v.active&&!v.preview&&v.channel==channel&&v.key==key{v.poly_pressure=value;}}}
  }true
 }
 fn sync_arp_mode(&mut self){
  let enabled=self.params.get("arp_on")>=0.5;let latch=self.params.get("arp_latch")as u8;
  if enabled!=self.arp_active{
   self.stop_arp_voice();
   for voice in &mut self.voices{if voice.active&&(voice.held||voice.sustained){voice.off(&self.config,self.sample_rate);}}
   self.modulator.note_off();self.arp_until=self.clock as f64;self.arp_step=0;self.arp_active=enabled;
   if !enabled{
    for channel in 0..16{for key in 0..128{
     let note=channel*128+key;let physical=self.physical[note/64]&(1u64<<(note%64))!=0;
     if physical&&self.held[channel][key]>0.0{self.start_note(channel as u8,key as u8,self.held_pitch[note],self.held[channel][key],false);}
     else{self.held[channel][key]=0.0;}
    }}
    for key in 0..128{
     let physical=self.preview_physical[key/64]&(1u64<<(key%64))!=0;
     if physical&&self.preview_held[key]>0.0{self.start_note(0,key as u8,self.preview_pitch[key],self.preview_held[key],true);}
     else{self.preview_held[key]=0.0;}
    }
   }
  }
  if latch==0&&self.latch_mode!=0{
   for channel in 0..16{for key in 0..128{let note=channel*128+key;if self.physical[note/64]&(1u64<<(note%64))==0{self.held[channel][key]=0.0;}}}
   for key in 0..128{if self.preview_physical[key/64]&(1u64<<(key%64))==0{self.preview_held[key]=0.0;}}
   if !self.any_held(){self.stop_arp_voice();}
  }
  self.latch_mode=latch;
 }
 fn arpeggiate(&mut self){
  self.sync_arp_mode();if !self.arp_active{return;}
  if self.clock as f64>=self.arp_release{self.stop_arp_voice();}
  if (self.clock as f64)<self.arp_until{return;}
  let mut notes=[(0u8,0u8,0f64,0u64,false);2176];let mut n=0;
  for ch in 0..16{for key in 0..128{if self.held[ch][key]>0.{notes[n]=(ch as u8,key as u8,self.held[ch][key],self.held_order[ch][key],false);n+=1;}}}
  for key in 0..128{if self.preview_held[key]>0.0{notes[n]=(0,key as u8,self.preview_held[key],self.preview_order[key],true);n+=1;}}
  if n==0{return;}
  // Fixed slice heapsort is allocation-free and preserves deterministic order.
  let order=self.params.get("arp_order")as usize;
  notes[..n].sort_unstable_by_key(|v|if order==3{v.3}else{v.1 as u64});
  let octaves=self.params.get("arp_octaves")as usize;let total=n*octaves.max(1);
  let idx=match order{1=>total-1-self.arp_step%total,2=>{let period=(total*2).saturating_sub(2).max(1);let step=self.arp_step%period;if step<total{step}else{period-step}},
   4=>(self.arp_step.wrapping_mul(1103515245).wrapping_add(12345))%total,_=>self.arp_step%total};
  let(ch,key,velocity,_,preview)=notes[idx%n];
  let pitch=(if preview{self.preview_pitch[key as usize]}else{self.held_pitch[ch as usize*128+key as usize]}+12.0*(idx/n)as f64).clamp(0.0,127.0);let pitch_key=pitch.round()as u8;
  self.stop_arp_voice();self.start_note(ch,pitch_key,pitch,velocity,preview);self.arp_voice=Some((preview,ch,pitch_key));
  let sync=self.params.get("arp_sync")as usize;
  let seconds=if sync>0{60./self.tempo*[0.,0.0625,0.125,0.25,0.5,1.,2.,4.,8.][sync.min(8)]}else{1./self.params.get("arp_rate")};
  let swing=1.+self.params.get("arp_groove")*0.01*if self.arp_step%2==0{1.}else{-1.};
  let duration=seconds*self.sample_rate*swing;self.arp_until=self.arp_until.max(self.clock as f64)+duration;self.arp_release=self.clock as f64+duration*self.params.get("arp_gate")*0.01;self.arp_step=self.arp_step.wrapping_add(1);
 }
 pub fn process(&mut self,left:&mut[f32],right:&mut[f32]){
  assert_eq!(left.len(),right.len());
  for frame in 0..left.len(){
   if self.clock%32==0{
    self.params.values.copy_from_slice(&self.values);
    let source=self.modulator.sources(32,0.);
    if self.modulator.params.active(){let mut writes=[(0,0.);8];let n=self.modulator.apply(&source,&self.base,&self.descs,&mut writes);
     for &(target,value)in &writes[..n]{if let Some(&i)=self.targets.get(target){if SYNTH_SPECS[i].automatable{self.params.values[i]=value;}}}}
    self.config=Config::from(&self.params);
   }
   self.arpeggiate();let c=&self.config;let mut sum=[0.;2];
   for v in &mut self.voices{
    if !v.active{continue;}let env=v.level(c,self.sample_rate);
    if !v.held&&!v.sustained&&env<=0.{v.active=false;continue;}
    let t=v.age as f64/self.sample_rate;let lfo=(TAU*c.lfo[0]*t).sin();
    let channel=v.channel as usize;let pitchbend=if c.mpe&&channel>0{self.bend[channel]*c.mpe_range+self.bend[0]*c.bend}else{self.bend[channel]*c.bend};
    let glide=if c.glide>0.{(v.previous_pitch-v.pitch)*(-t/c.glide).exp()}else{0.};
    let base=440.*2f64.powf((v.pitch-69.+pitchbend+glide+lfo*c.lfo[1])/12.);
    let tracked=2f64.powf((v.pitch-60.)*c.keytrack/12.);let expression=if c.mpe{2f64.powf(self.timbre[channel]*2.)}else{1.};
    let cut_env=2f64.powf(env*c.env+lfo*c.lfo[2]/12.)*tracked*expression;
    for channel_out in 0..2{
     let sign=if channel_out==0{1.}else{-1.};let mut raw=0.;
     for u in 0..c.unison{let spread=if c.unison==1{0.}else{u as f64*2./(c.unison-1)as f64-1.};
      for o in 0..4{if c.mix[o]==0.{continue;}
       let cents=if o==0{sign*c.detune*c.width*0.5}else{-sign*c.detune};
       let frequency=base*2f64.powf((c.semis[o]*100.+cents+spread*c.spread)/1200.);let dt=frequency/self.sample_rate;
       let phase=v.phase[u][o][channel_out];let sample=osc(c.waves[o],phase,dt,c.pulse);
       let modeled=match c.model{1=>(sample*1.3).tanh()/1.3f64.tanh(),2=>sample+0.035*(phase*TAU*3.).sin(),3=>osc(c.waves[o],(phase*2.).fract(),dt*2.,c.pulse),4=>sample*(0.8+0.2*(phase*TAU).cos()),_=>sample};
       raw+=modeled*c.mix[o]/c.unison as f64;v.phase[u][o][channel_out]=(phase+dt).fract();
      }
     }
     let ns=&mut v.noise[channel_out];*ns^=*ns<<13;*ns^=*ns>>7;*ns^=*ns<<17;let noise=(*ns>>11)as f64/(1u64<<53)as f64*2.-1.;
     raw=(raw+noise*c.noise)/(1.+c.mix[2]+c.mix[3]+c.noise*0.5);
     let filters=&mut v.filters;let factor=1usize<<c.quality.min(2);
     let filtered=v.quality[channel_out].run(raw,|input|{
      let mut driven=if c.drive>1.{(input*c.drive).tanh()/c.drive.tanh()}else{input};
      driven=match c.character{1=>(driven*1.15).tanh(),2=>driven+(driven*2.).sin()*0.06,3=>driven/(1.+driven.abs()*0.25),4=>driven.clamp(-0.75,0.75),5=>(driven*1.5+0.08).tanh()-0.08f64.tanh(),_=>driven};
      let(mut y,mut count)=(if c.parallel{0.}else{driven},0.0f64);
      for f in 0..4{if !c.enabled[f]{continue;}let input=if c.parallel{driven}else{y};
       let cut=c.cuts[f]*if f==0{cut_env}else{tracked};
       let wet=filters[f][channel_out].run(input,cut,c.q[f],self.sample_rate*factor as f64,c.shapes[f]);
       let pan_gain=if channel_out==0{1.-c.pan[f].max(0.)}else{1.+c.pan[f].min(0.)};
       if c.parallel{y+=wet*pan_gain;}else{y=wet*pan_gain;}count+=1.;
      }if c.parallel{y/count.max(1.)}else{y}
     });
     let velocity=1.-c.velocity+c.velocity*v.velocity;
     sum[channel_out]+=filtered*env*velocity*c.gain*(1.+(self.pressure[channel]+if c.mpe&&channel>0{self.pressure[0]}else{0.}+v.poly_pressure)*0.25);
    }
    v.age+=1;if !v.held&&!v.sustained{v.release_age+=1;}
   }
   let out=self.fx.run(sum,&self.params,self.sample_rate,self.tempo);
   left[frame]=if out[0].is_finite(){out[0]as f32}else{0.};right[frame]=if out[1].is_finite(){out[1]as f32}else{0.};self.clock+=1;
  }
  if self.capture.capture(left,right,left,right).is_ok(){self.telemetry.publish_if_due(&self.capture);}
 }
 pub fn process_events(&mut self,l:&mut[f32],r:&mut[f32],events:&[ParamEvent])->bool{
  if l.len()!=r.len()||events.len()>proto::MAX_PARAM_EVENTS||events.windows(2).any(|w|w[0].1>w[1].1)||events.iter().any(|&(id,o,v)|o as usize>=l.len()||!self.valid_event(id,v)){return false;}
  let(mut at,mut event)=(0,0);while at<l.len(){while event<events.len()&&events[event].1 as usize==at{self.event(events[event].0,events[event].2);event+=1;}
   let end=events.get(event).map_or(l.len(),|e|e.1 as usize);self.process(&mut l[at..end],&mut r[at..end]);at=end;}true
 }
}
#[cfg(feature="client")]
impl verdant_audio::Processor for RealtimeSynth{
 fn name(&self)->&str{"Verdant Synth"}
 fn reset(&mut self){RealtimeSynth::reset(self);}
 fn process(&mut self,left:&mut[f32],right:&mut[f32]){RealtimeSynth::process(self,left,right);}
 fn process_automated(&mut self,left:&mut[f32],right:&mut[f32],events:&[ParamEvent])->bool{self.process_events(left,right,events)}
}

/// A persisted controller point, in frames relative to its owning note.
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct SynthExpression {pub offset:u64,pub id:u32,pub value:f64}
#[derive(Clone,Debug)]
pub struct SynthPerformanceNote {pub note:SynthNote,pub channel:u8,pub expression:Vec<SynthExpression>}

/// Score playback/export and native MIDI monitoring use this same engine.
/// Equal-frame ordering is tempo, note-off, initial expression, note-on, then
/// in-note expression. Stable note order preserves chord retrigger behavior.
pub fn render_performance(id:&str,params:&SynthParams,notes:&[SynthPerformanceNote],sample_rate:f64,tempos:&[(u64,f64)])->Result<Vec<f32>,String>{
 if !sample_rate.is_finite()||!(8000.0..=384000.0).contains(&sample_rate){return Err("Invalid instrument sample rate".into());}
 if notes.is_empty(){return Ok(Vec::new());}
 if notes.len()>1_048_576{return Err("Instrument note limit exceeded".into());}
 if notes.iter().any(|n|n.channel>15||!n.note.pitch.is_finite()||!n.note.velocity.is_finite()||!(0.0..=1.0).contains(&n.note.velocity)||n.note.length==0
   ||n.expression.iter().any(|e|e.offset>n.note.length||!(proto::decode_midi(e.id,e.value).is_some()||proto::valid_note_event(e.id,e.value))))
 {return Err("Invalid MIDI performance".into());}
 if tempos.windows(2).any(|w|w[0].0>w[1].0)||tempos.iter().any(|(_,bpm)|!bpm.is_finite()||!(20.0..=1000.0).contains(bpm)){
  return Err("Instrument beat sync supports 20–1000 BPM".into());
 }
 let expression_count=notes.iter().try_fold(0usize,|count,n|count.checked_add(n.expression.len())).ok_or("Expression limit exceeded")?;
 if expression_count>1_048_576{return Err("Expression limit exceeded".into());}
 let release=(params.release_seconds()*sample_rate).ceil()as u64;
 let reverb_tail=if params.get("fx_reverb")>0.{params.get("fx_reverb_time")*4.0/3.0}else{0.0};
 let delay_tail=if params.get("fx_delay")>0.{
  let sync=params.get("fx_delay_sync").round() as usize;
  let slowest=tempos.iter().map(|p|p.1).fold(params.get("arp_tempo"),f64::min);
  let time=if sync>0{60.0/slowest*[0.0,0.0625,0.125,0.25,0.5,1.0,2.0,4.0,8.0][sync.min(8)]}else{params.get("fx_delay_time")*0.001};
  let feedback=(params.get("fx_delay_feedback")*0.01).clamp(0.0,0.95);
  time*(if feedback>0.0{(1e-4_f64.ln()/feedback.ln()).ceil()}else{1.0}+1.0)
 }else{0.0};
 let tail=((reverb_tail+delay_tail)*sample_rate).ceil() as u64;
 let end=notes.iter().map(|n|n.note.start.checked_add(n.note.length).and_then(|n|n.checked_add(release))).collect::<Option<Vec<_>>>().ok_or("Instrument duration overflow")?.into_iter().max().unwrap_or(0);
 let frames=usize::try_from(end.checked_add(tail).ok_or("Instrument duration overflow")?).map_err(|_|"Instrument duration overflow")?;
 // kind: 0tempo,1off,2initialexpression,3on,4expression.
 let mut events:Vec<(u64,u8,usize,u32,f64)>=Vec::with_capacity(notes.len()*2+expression_count+tempos.len());
 for(i,n)in notes.iter().enumerate(){
  events.push((n.note.start,3,i,0,0.));events.push((n.note.start+n.note.length,1,i,0,0.));
  for e in &n.expression{events.push((n.note.start+e.offset,if e.offset==0{2}else{4},i,e.id,e.value));}
 }
 let last_note=notes.iter().map(|n|n.note.start+n.note.length).max().unwrap_or(0);
 events.push((last_note,5,0,0,0.0));
 for &(frame,bpm)in tempos{events.push((frame,0,0,super::TRANSPORT_TEMPO_EVENT,bpm));}
 events.sort_by_key(|e|(e.0,e.1,e.2));
 let mut synth=RealtimeSynth::for_instrument(id,params,sample_rate);
 let count=frames.checked_mul(2).ok_or("Instrument duration overflow")?;
 let mut out=Vec::new();out.try_reserve_exact(count).map_err(|_|"Not enough memory to render instrument")?;out.resize(count,0.);
 let(mut l,mut r)=([0.;256],[0.;256]);let(mut at,mut next)=(0usize,0usize);
 while at<frames{
  while next<events.len()&&events[next].0<=at as u64{
   let(_,kind,i,event,value)=events[next];
   if kind==1||kind==3{
    let n=&notes[i];let key=n.note.pitch.round().clamp(0.,127.)as u8;
    if kind==3{synth.note_on(n.channel,key,n.note.pitch,n.note.velocity);}else{synth.note_off(n.channel,key);}
   }else if kind==5{synth.release_all();}else{synth.event(event,value);}next+=1;
  }
  let end=(at+256).min(frames).min(events.get(next).map_or(frames,|e|usize::try_from(e.0).unwrap_or(frames)));
  if end==at{continue;}
  synth.process(&mut l[..end-at],&mut r[..end-at]);for i in 0..end-at{out[(at+i)*2]=l[i];out[(at+i)*2+1]=r[i];}at=end;
 }
 Ok(out)
}
/// Compatibility entry point for score callers without expressive metadata.
pub fn render(params:&SynthParams,notes:&[SynthNote],sample_rate:f64)->Vec<f32>{
 let notes:Vec<_>=notes.iter().map(|&note|SynthPerformanceNote{note,channel:0,expression:Vec::new()}).collect();
 render_performance(SYNTH_ID,params,&notes,sample_rate,&[]).unwrap_or_default()
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn mpe_bend_and_pressure_are_channel_local_and_release_works(){
 let mut p=SynthParams::default();p.set("mpe",1.).unwrap();p.set("osc2_mix",0.).unwrap();p.set("release",10.).unwrap();
 let mut a=RealtimeSynth::new(&p,48000.);a.note_on(1,60,60.,0.8);let mut l=vec![0.;4096];let mut r=l.clone();a.process(&mut l,&mut r);let before=l.clone();
 a.event(proto::midi_pitch_bend(1,0,0.25).0,0.25);a.process(&mut l,&mut r);assert_ne!(l,before);a.note_off(1,60);a.process(&mut l,&mut r);assert!(l[1000..].iter().all(|x|x.abs()<1e-5));
 }
 #[test]fn effects_four_filters_and_arp_each_change_audio(){
 let notes=[SynthNote{start:0,length:6000,pitch:60.,velocity:0.8}];let p=SynthParams::default();let baseline=render(&p,&notes,48000.);
 for(id,value)in[("filter3_enabled",1.),("filter4_enabled",1.),("filter_character",3.),("osc_model",2.),("fx_drive",30.),("fx_chorus",50.),("fx_phaser",50.),("fx_delay",50.),("fx_reverb",50.),("fx_compressor",100.),("arp_on",1.)]{
 let mut p=p.clone();p.set(id,value).unwrap();if id=="fx_delay"{p.set("fx_delay_time",20.).unwrap();}if id=="arp_on"{p.set("arp_sync",1.).unwrap();}
 let audio=render(&p,&notes,48000.);assert!(audio.iter().all(|v|v.is_finite()));assert!(audio.iter().zip(&baseline).any(|(a,b)|(a-b).abs()>1e-5),"{id}");}
 }
 #[test]fn live_blocks_equal_score_render_and_invalid_events_are_atomic(){
 let mut p=SynthParams::default();p.set("release",10.).unwrap();let note=SynthNote{start:64,length:256,pitch:60.,velocity:0.8};let expected=render(&p,&[note],48000.);
 let mut live=RealtimeSynth::new(&p,48000.);let mut l=vec![0.;expected.len()/2];let mut r=l.clone();
 assert!(live.process_events(&mut l,&mut r,&[proto::note_on(60,64,0.8),proto::note_off(60,320)]));
 let actual:Vec<f32>=l.iter().zip(&r).flat_map(|(l,r)|[*l,*r]).collect();assert_eq!(actual,expected);
 let old=l.clone();assert!(!live.process_events(&mut l,&mut r,&[(u32::MAX,0,1.)]));assert_eq!(old,l);
 }
 #[test]fn performance_render_replays_mpe_and_tempo_at_exact_frames(){
  let mut p=SynthParams::default();for(id,value)in[("mpe",1.0),("release",10.0),("osc2_mix",0.0)]{p.set(id,value).unwrap();}
  let bend=proto::midi_pitch_bend(1,0,0.125);let pressure=proto::midi_pressure(1,0,0.7);
  let note=SynthPerformanceNote{note:SynthNote{start:13,length:2000,pitch:60.0,velocity:0.7},channel:1,
   expression:vec![SynthExpression{offset:500,id:bend.0,value:bend.2},SynthExpression{offset:1000,id:pressure.0,value:pressure.2}]};
  let expected=render_performance(SYNTH_ID,&p,&[note.clone()],48000.0,&[(0,120.0),(800,90.0)]).unwrap();
  let mut live=RealtimeSynth::new(&p,48000.0);let frames=expected.len()/2;let(mut l,mut r)=(vec![0.0;frames],vec![0.0;frames]);
  let events=[(super::super::TRANSPORT_TEMPO_EVENT,0,120.0),proto::note_on_channel(1,60,13,0.7),proto::midi_pitch_bend(1,513,0.125),
   (super::super::TRANSPORT_TEMPO_EVENT,800,90.0),proto::midi_pressure(1,1013,0.7),proto::note_off_channel(1,60,2013)];
  assert!(live.process_events(&mut l,&mut r,&events));let actual:Vec<f32>=l.iter().zip(r).flat_map(|(l,r)|[*l,r]).collect();assert_eq!(expected,actual);
  let mut plain=note;plain.expression.clear();let baseline=render_performance(SYNTH_ID,&p,&[plain],48000.0,&[]).unwrap();
  assert_eq!(&expected[..513*2],&baseline[..513*2]);assert_ne!(&expected[600*2..1800*2],&baseline[600*2..1800*2]);
 }
 #[test]fn beat_arpeggiator_uses_host_tempo_without_mutating_saved_fallback(){
  let mut p=SynthParams::default();for(id,value)in[("arp_on",1.0),("arp_sync",3.0),("arp_gate",30.0),("release",10.0)]{p.set(id,value).unwrap();}
  let notes=[SynthPerformanceNote{note:SynthNote{start:0,length:24000,pitch:60.0,velocity:0.8},channel:0,expression:vec![]}];
  let a=render_performance(SYNTH_ID,&p,&notes,48000.0,&[(0,120.0)]).unwrap();
  let b=render_performance(SYNTH_ID,&p,&notes,48000.0,&[(0,60.0),(12000,180.0)]).unwrap();assert_ne!(a,b);assert_eq!(p.get("arp_tempo"),120.0);
 }
 #[test]fn midi_matrix_sources_and_poly_pressure_preserve_velocity_and_reset(){
  let mut synth=RealtimeSynth::new(&SynthParams::default(),48000.0);synth.note_on(2,60,60.0,0.4);
  for e in [proto::midi_pitch_bend(2,0,0.25),proto::midi_pressure(2,0,0.75),proto::midi_control(2,74,0,0.6),proto::midi_poly_pressure(2,60,0,0.9)]{assert!(synth.event(e.0,e.2));}
  assert_eq!(synth.modulator.bend,0.25);assert_eq!(synth.modulator.pressure,0.75);assert_eq!(synth.modulator.timbre,0.6);
  assert_eq!(synth.voices[0].velocity,0.4);assert_eq!(synth.voices[0].poly_pressure,0.9);
  synth.reset();assert_eq!(synth.bend,[0.0;16]);assert_eq!(synth.modulator.pressure,0.0);
 }

 #[test]fn native_controls_preserve_held_voices_and_batches_are_atomic(){
  let mut p=SynthParams::default();p.set("attack",1.0).unwrap();p.set("sustain",100.0).unwrap();p.set("release",20.0).unwrap();
  let mut synth=RealtimeSynth::new(&p,48000.0);synth.note_on(2,64,64.0,0.7);
  let mut l=[0.0;64];let mut r=[0.0;64];synth.process(&mut l,&mut r);
  let index=|id:&str|instrument_specs(SYNTH_ID).iter().position(|s|s.id==id).unwrap()as u32;
  let voice=synth.voices.iter().position(|v|v.active).unwrap();let age=synth.voices[voice].age;let serial=synth.voices[voice].serial;
  let quality=index("high_quality");let gain=index("gain");let original_gain=synth.params.get("gain");
  let before=l;
  assert!(!synth.process_events(&mut l,&mut r,&[(gain,0,-18.0),(quality,32,2.0)]));
  assert_eq!(l,before);assert_eq!(synth.params.get("gain"),original_gain);assert_eq!(synth.voices[voice].age,age);
  assert!(synth.process_events(&mut l,&mut r,&[(SYNTH_CONTROL_EVENT|quality,0,2.0),(gain,32,-18.0)]));
  assert_eq!(synth.voices[voice].serial,serial);assert_eq!(synth.voices[voice].age,age+64);
  assert!(synth.voices[voice].held);assert_eq!(synth.voices[voice].quality[0].factor(),4);
  assert_eq!(synth.params.get("gain"),-18.0);
  let modulation=index("mod1_amount");assert!(synth.event(SYNTH_CONTROL_EVENT|modulation,17.5));
  assert_eq!(synth.modulator.params.get("mod1_amount"),17.5);
  assert!(!synth.event(SYNTH_CONTROL_EVENT|0x0fff_ffff,0.0));
  let mut one=RealtimeSynth::for_instrument(ONE_SYNTH_ID,&p,48000.0);
  let voices=instrument_specs(ONE_SYNTH_ID).iter().position(|s|s.id=="voices").unwrap()as u32;
  assert!(!one.event(SYNTH_CONTROL_EVENT|voices,11.0));
 }

 #[test]fn arp_latch_replaces_or_adds_using_physical_keys_and_channel_clears(){
  for mode in 0..=2{
   let mut params=SynthParams::default();params.set("arp_on",1.0).unwrap();params.set("arp_latch",mode as f64).unwrap();
   let mut synth=RealtimeSynth::new(&params,48000.0);
   synth.note_on(1,60,60.0,0.8);synth.note_on(2,64,64.0,0.7);
   let(mut l,mut r)=([0.0;64],[0.0;64]);synth.process(&mut l,&mut r);
   synth.note_off(1,60);synth.note_off(2,64);
   assert!(synth.physical.iter().all(|bits|*bits==0));
   assert_eq!(synth.held[1][60]>0.0,mode!=0);
   synth.note_on(3,67,67.5,0.6);
   assert_eq!(synth.held[1][60]>0.0,mode==2);assert_eq!(synth.held[2][64]>0.0,mode==2);
   assert_eq!(synth.held[3][67],0.6);
   if mode==1{
    synth.process(&mut l,&mut r);
    let voice=synth.voices.iter().find(|v|v.active&&v.held).unwrap();
    assert_eq!(voice.channel,3);assert_eq!(voice.pitch,67.5);
   }
   synth.note_on(4,72,72.0,0.8);
   assert!(synth.event(proto::midi_control(3,123,0,0.0).0,0.0));
   assert!(synth.held[3].iter().all(|value|*value==0.0));assert_eq!(synth.physical[6],0);assert_eq!(synth.physical[7],0);
   assert!(synth.held[4][72]>0.0);
   assert!(synth.event(proto::midi_control(4,120,0,0.0).0,0.0));
   assert!(synth.held[4].iter().all(|value|*value==0.0));assert_eq!(synth.physical[8],0);assert_eq!(synth.physical[9],0);
   synth.release_all();assert!(synth.physical.iter().all(|bits|*bits==0));
   synth.note_on(0,60,60.0,0.8);synth.reset();assert!(synth.physical.iter().all(|bits|*bits==0));
  }
 }

 #[test]fn enabling_arpeggiator_after_idle_never_catches_up_or_leaves_held_voices(){
  let mut params=SynthParams::default();params.set("release",5.0).unwrap();params.set("arp_sync",0.0).unwrap();params.set("arp_rate",8.0).unwrap();
  let mut synth=RealtimeSynth::new(&params,48000.0);
  let(mut silent_l,mut silent_r)=(vec![0.0;96000],vec![0.0;96000]);synth.process(&mut silent_l,&mut silent_r);
  synth.note_on(1,60,60.25,0.7);let(mut l,mut r)=([0.0;128],[0.0;128]);synth.process(&mut l,&mut r);
  let index=instrument_specs(SYNTH_ID).iter().position(|spec|spec.id=="arp_on").unwrap()as u32;
  let before=synth.serial;assert!(synth.event(SYNTH_CONTROL_EVENT|index,1.0));synth.process(&mut l,&mut r);
  assert_eq!(synth.serial,before+1);assert_eq!(synth.arp_step,1);assert_eq!(synth.voices.iter().filter(|voice|voice.active&&voice.held).count(),1);
  // A missed deadline (for example a live transport reposition) schedules
  // one future step rather than generating one new voice on every sample.
  synth.arp_until=0.0;synth.arp_release=synth.clock as f64+100000.0;let before=synth.serial;
  synth.process(&mut l,&mut r);
  assert_eq!(synth.serial,before+1);assert!(synth.arp_until>synth.clock as f64);
  assert_eq!(synth.voices.iter().filter(|voice|voice.active&&voice.held).count(),1);
  assert!(synth.event(SYNTH_CONTROL_EVENT|index,0.0));synth.process(&mut l,&mut r);
  let voice=synth.voices.iter().find(|voice|voice.active&&voice.held).unwrap();assert_eq!(voice.pitch,60.25);
  synth.note_off(1,60);synth.process(&mut silent_l[..1024],&mut silent_r[..1024]);
  assert!(synth.voices.iter().all(|voice|!voice.active));assert!(synth.arp_voice.is_none());
 }

 #[test]fn control_admission_rejects_fractional_selectors_and_fixed_modulation_targets(){
  let mut synth=RealtimeSynth::new(&SynthParams::default(),48000.0);
  let index=|id:&str|instrument_specs(SYNTH_ID).iter().position(|spec|spec.id==id).unwrap()as u32;
  assert!(!synth.event(SYNTH_CONTROL_EVENT|index("osc1"),0.5));
  assert!(!synth.event(SYNTH_CONTROL_EVENT|index("mod1_target"),index("high_quality")as f64));
  assert!(!synth.event(SYNTH_CONTROL_EVENT|index("mod1_target"),index("mod1_source")as f64));
  let gain=synth.params.get("gain");let(mut l,mut r)=([0.25;32],[0.5;32]);
  assert!(!synth.process_events(&mut l,&mut r,&[(index("gain"),0,-20.0),(SYNTH_CONTROL_EVENT|index("osc1"),16,0.5)]));
  assert_eq!(synth.params.get("gain"),gain);assert_eq!(l,[0.25;32]);assert_eq!(r,[0.5;32]);
  assert!(synth.event(SYNTH_CONTROL_EVENT|index("mod1_target"),index("cutoff")as f64));
 }

 #[test]fn preview_release_does_not_release_external_midi_on_the_same_key(){
  let mut params=SynthParams::default();params.set("release",5.0).unwrap();
  let mut synth=RealtimeSynth::new(&params,48000.0);synth.note_on(0,60,60.0,0.8);
  let on=proto::note_on(60,0,0.7);let off=proto::note_off(60,0);
  assert!(synth.event(SYNTH_PREVIEW_NOTE|on.0,on.2));
  let(mut l,mut r)=([0.0;64],[0.0;64]);synth.process(&mut l,&mut r);
  assert_eq!(synth.voices.iter().filter(|voice|voice.held&&voice.active).count(),2);
  assert!(synth.event(SYNTH_PREVIEW_NOTE|off.0,0.0));
  assert!(synth.voices.iter().any(|voice|!voice.preview&&voice.active&&voice.held));
  assert!(synth.event(SYNTH_PREVIEW_NOTE|on.0,on.2));
  assert!(synth.event(SYNTH_PREVIEW_RELEASE,0.0));
  assert!(synth.preview_held.iter().all(|velocity|*velocity==0.0));assert!(synth.preview_physical.iter().all(|bits|*bits==0));
  assert_eq!(synth.held[0][60],0.8);assert_ne!(synth.physical[0],0);
  assert!(synth.voices.iter().any(|voice|!voice.preview&&voice.active&&voice.held));
  for _ in 0..8{synth.process(&mut l,&mut r);}
  assert!(synth.voices.iter().all(|voice|!voice.preview||!voice.active));
  assert!(synth.voices.iter().any(|voice|!voice.preview&&voice.active&&voice.held));
  synth.note_off(0,60);for _ in 0..8{synth.process(&mut l,&mut r);}assert!(synth.voices.iter().all(|voice|!voice.active));
 }

 #[test]fn preview_latched_arpeggio_stop_preserves_external_owned_chord(){
  let mut params=SynthParams::default();
  for(id,value)in[("arp_on",1.0),("arp_latch",2.0),("arp_sync",0.0),("arp_rate",20.0),("arp_octaves",1.0),("release",5.0)]{params.set(id,value).unwrap();}
  let mut synth=RealtimeSynth::new(&params,48000.0);
  synth.note_on(0,60,60.0,0.8);synth.note_off(0,60);
  let on=proto::note_on(72,0,0.7);let off=proto::note_off(72,0);
  assert!(synth.event(SYNTH_PREVIEW_NOTE|on.0,on.2));assert!(synth.event(SYNTH_PREVIEW_NOTE|off.0,0.0));
  let(mut l,mut r)=(vec![0.0;3000],vec![0.0;3000]);synth.process(&mut l,&mut r);
  assert_eq!(synth.arp_voice,Some((true,0,72)));
  assert!(synth.event(SYNTH_PREVIEW_RELEASE,0.0));
  assert_eq!(synth.held[0][60],0.8);assert!(synth.preview_held.iter().all(|velocity|*velocity==0.0));
  synth.process(&mut l,&mut r);
  assert_eq!(synth.arp_voice,Some((false,0,60)));
  assert!(synth.voices.iter().all(|voice|!voice.preview||!voice.active));
  // A source-specific panic also leaves the external voice and chord intact.
  assert!(synth.event(SYNTH_PREVIEW_NOTE|on.0,on.2));assert!(synth.event(SYNTH_PREVIEW_PANIC,0.0));
  assert_eq!(synth.held[0][60],0.8);assert!(synth.voices.iter().any(|voice|!voice.preview&&voice.active));
  synth.release_all();synth.process(&mut l,&mut r);
  assert!(synth.voices.iter().all(|voice|!voice.active));assert!(!synth.any_held());
 }

 #[test]fn preview_event_admission_is_atomic_and_unconnected_modulators_keep_time(){
  let mut synth=RealtimeSynth::new(&SynthParams::default(),48000.0);
  let on=proto::note_on(60,0,0.7);let(mut l,mut r)=([0.25;64],[0.5;64]);
  assert!(!synth.process_events(&mut l,&mut r,&[(SYNTH_PREVIEW_NOTE|on.0,0,on.2),(SYNTH_PREVIEW_RELEASE,16,1.0)]));
  assert_eq!(l,[0.25;64]);assert!(!synth.any_held());
  assert!(!synth.valid_event(SYNTH_PREVIEW_NOTE|on.0|(1<<16),on.2));
  let before=synth.modulator.sources(0,0.0);
  for _ in 0..8{synth.process(&mut l,&mut r);}
  let after=synth.modulator.sources(0,0.0);
  assert!((after[1]-before[1]).abs()>1e-4);
 }

 #[test]fn synth_and_one_meter_reset_preserves_audio_voices_arp_keys_and_frame_clocks(){
  for id in [SYNTH_ID,ONE_SYNTH_ID]{
   let mut params=SynthParams::default();params.set("sustain",100.0).unwrap();
   let cutoff=instrument_specs(id).iter().position(|spec|spec.id=="cutoff").unwrap();
   for(key,value)in[("mod1_source",1.0),("mod1_target",cutoff as f64),("mod1_amount",15.0)]{params.set(key,value).unwrap();}
   if id==SYNTH_ID{
    for(key,value)in[("arp_on",1.0),("arp_latch",2.0),("arp_sync",0.0),("arp_rate",8.0),("fx_delay",12.0)]{params.set(key,value).unwrap();}
   }
   let mut measured=RealtimeSynth::for_instrument(id,&params,48000.0);
   let mut reference=RealtimeSynth::for_instrument(id,&params,48000.0);
   let preview=proto::note_on(65,0,0.6);
   for synth in [&mut measured,&mut reference]{
    synth.note_on(0,60,60.25,0.8);assert!(synth.event(SYNTH_PREVIEW_NOTE|preview.0,preview.2));
   }
   let(mut al,mut ar,mut bl,mut br)=(vec![0.0;32000],vec![0.0;32000],vec![0.0;32000],vec![0.0;32000]);
   measured.process(&mut al,&mut ar);reference.process(&mut bl,&mut br);assert_eq!(al,bl);assert_eq!(ar,br);
   let before=measured.telemetry.frame().unwrap();assert!(before.loudness.unwrap().integrated_lufs> -100.0);
   let clock=measured.clock;let arp=(measured.arp_voice,measured.arp_step,measured.arp_until,measured.arp_release);
   let physical=measured.physical;let held=measured.held;let preview_keys=measured.preview_physical;let preview_held=measured.preview_held;
   let voices:Vec<_>=measured.voices.iter().map(|voice|(voice.active,voice.preview,voice.age,voice.serial,voice.held,voice.release_age)).collect();
   assert!(measured.event(SYNTH_RESET_METERS,0.0));
   let after=measured.telemetry.frame().unwrap();
   assert_eq!(after.frames_total,before.frames_total);assert_eq!(after.window_frames,before.window_frames);
   assert_eq!(after.samples_b64,before.samples_b64);assert_eq!(after.waveform_input,before.waveform_input);assert_eq!(after.waveform_output,before.waveform_output);
   assert_eq!(after.loudness,Some(super::super::quality::LoudnessReading::default()));
   assert_eq!(measured.clock,clock);assert_eq!((measured.arp_voice,measured.arp_step,measured.arp_until,measured.arp_release),arp);
   assert_eq!(measured.physical,physical);assert_eq!(measured.held,held);assert_eq!(measured.preview_physical,preview_keys);assert_eq!(measured.preview_held,preview_held);
   assert_eq!(measured.voices.iter().map(|voice|(voice.active,voice.preview,voice.age,voice.serial,voice.held,voice.release_age)).collect::<Vec<_>>(),voices);
   // The same admitted event can arrive mid-block. Audio, modulation phase,
   // effects histories and cumulative sample clocks stay sample-identical.
   assert!(measured.process_events(&mut al[..2048],&mut ar[..2048],&[(SYNTH_RESET_METERS,512,0.0)]));
   reference.process(&mut bl[..2048],&mut br[..2048]);
   assert_eq!(&al[..2048],&bl[..2048]);assert_eq!(&ar[..2048],&br[..2048]);assert!(al[..2048].iter().any(|sample|sample.abs()>1e-4));
   assert_eq!(measured.clock,clock+2048);assert_eq!(measured.clock,reference.clock);
   assert_eq!(measured.capture.frame().frames_total,before.frames_total+2048);
  }
 }
 #[test]fn invalid_meter_reset_batches_leave_measurements_and_audio_untouched(){
  let mut synth=RealtimeSynth::new(&SynthParams::default(),48000.0);synth.note_on(0,60,60.0,0.8);
  let(mut l,mut r)=(vec![0.0;24000],vec![0.0;24000]);synth.process(&mut l,&mut r);
  let before=synth.telemetry.frame().unwrap();let clock=synth.clock;
  assert!(!synth.valid_event(SYNTH_RESET_METERS,1.0));assert!(!synth.event(SYNTH_RESET_METERS,f64::NAN));
  let(mut left,mut right)=([0.25;64],[0.5;64]);
  assert!(!synth.process_events(&mut left,&mut right,&[(SYNTH_RESET_METERS,0,0.0),(SYNTH_RESET_METERS,16,1.0)]));
  let after=synth.telemetry.frame().unwrap();assert_eq!(after.frames_total,before.frames_total);assert_eq!(after.loudness,before.loudness);
  assert_eq!(synth.clock,clock);assert_eq!(left,[0.25;64]);assert_eq!(right,[0.5;64]);
 }

}
