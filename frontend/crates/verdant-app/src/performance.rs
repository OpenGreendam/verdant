//! Native MIDI performance, bounded audio-queue delivery and device learn.
use std::{sync::Arc,time::{Duration,Instant},hash::{Hash,Hasher}};
use gpui::{Context,AppContext,Window};
use verdant_domain::TrackId;
use verdant_midi::{CapturedEvent,EventKind,MidiCapture,MidiInputService,MidiPort};
use verdant_plugins::{InsertKey,proto};
use verdant_plugins::builtin_dsp::{self as dsp,RealtimeSynth};
use verdant_plugins::telemetry::{LiveTelemetry,TelemetrySnapshot};
use crate::{editor::Editor,devices::DeviceTarget};

struct MidiLearn {target:DeviceTarget,slot:usize,key:InsertKey,mapping:usize,generation:u64}
pub struct PerformanceState {
 pub capture:Option<MidiCapture>, binding:Option<(TrackId,u64,u64)>,
 learn:Option<MidiLearn>,target:Option<DeviceTarget>,preview_keys:[bool;128],
 telemetry:Option<Arc<LiveTelemetry>>,snapshot:Option<Arc<TelemetrySnapshot>>,
 instrument_id:Option<String>,parameters:Option<dsp::SynthParams>,track_index:Option<usize>,tempo:Option<f64>,
 polling:bool,last_poll:Instant,stop_pending:bool,preview_release_pending:bool,meter_reset_pending:bool,meter_reset_epoch:u64,
}
impl Default for PerformanceState {
 fn default()->Self{Self{capture:None,binding:None,learn:None,target:None,preview_keys:[false;128],telemetry:None,snapshot:None,instrument_id:None,parameters:None,track_index:None,tempo:None,polling:false,last_poll:Instant::now(),stop_pending:false,preview_release_pending:false,meter_reset_pending:false,meter_reset_epoch:0}}
}
fn native_event(event:CapturedEvent)->(u32,f64){
 let ch=event.channel;
 let (id,_,value)=match event.kind{
 EventKind::NoteOn{midi_note,velocity}=>proto::note_on_channel(ch,midi_note,0,f64::from(velocity)/127.),
 EventKind::NoteOff{midi_note,..}=>proto::note_off_channel(ch,midi_note,0),
 EventKind::Control{controller,value}=>proto::midi_control(ch,controller,0,f64::from(value)/127.),
 EventKind::PitchBend{value}=>proto::midi_pitch_bend(ch,0,f64::from(value)/if value<0{8192.}else{8191.}),
 EventKind::ChannelPressure{pressure}=>proto::midi_pressure(ch,0,f64::from(pressure)/127.),
 EventKind::PolyPressure{midi_note,pressure}=>proto::midi_poly_pressure(ch,midi_note,0,f64::from(pressure)/127.),
 EventKind::Sustain{down}=>proto::midi_control(ch,64,0,if down{1.}else{0.}),
 EventKind::AllSoundOff=>proto::midi_control(ch,120,0,0.),
 EventKind::AllNotesOff=>proto::midi_control(ch,123,0,0.),
 EventKind::ResetControllers=>proto::midi_control(ch,121,0,0.),
 };(id,value)
}
fn learned_instrument_changes(id:&str,params:&dsp::SynthParams,messages:&[(u32,f64)])->Vec<(String,f64)>{
 let specs=dsp::instrument_specs(id);let base:Vec<_>=specs.iter().filter(|s|dsp::modulation_parameter(s.id).is_none()).copied().collect();
 let values:Vec<_>=base.iter().map(|s|params.get(s.id)).collect();let descs=dsp::table_descs(&base,&values);
 let mut writes:Vec<(String,f64)>=Vec::new();
 for &(event,value)in messages{
  let Some(proto::MidiEvent::Control{channel,controller,value})=proto::decode_midi(event,value)else{continue;};
  for slot in 1..=8{
   let cc=params.get(&format!("midi{slot}_cc"));let ch=params.get(&format!("midi{slot}_channel"));
   if cc!=f64::from(controller)||(ch!=0.&&ch!=f64::from(channel)+1.){continue;}
   let target=params.get(&format!("midi{slot}_target"))as usize;
   let Some(spec)=base.get(target).filter(|s|s.automatable)else{continue;};
   let value=dsp::modulation::denormalize(&descs[target],value);
   if let Some(existing)=writes.iter_mut().find(|(id,_)|id==spec.id){existing.1=value;}
   else{writes.push((spec.id.to_string(),value));}
  }
 }
 writes.retain(|(id,value)|params.get(id)!=*value);writes
}
impl Editor {
 pub(crate) fn prepare_instrument_monitor(&mut self,track:&TrackId)->bool{
  let Some(t)=self.project().track(track)else{return false};
  let Some(params)=crate::instrument::synth_params(t)else{return false};
  let Some(spec)=t.instrument()else{return false};
  let id=spec.get("id").and_then(|v|v.as_str()).unwrap_or(dsp::SYNTH_ID).to_string();
  let mut hash=std::collections::hash_map::DefaultHasher::new();spec.to_string().hash(&mut hash);
  let fingerprint=hash.finish();let generation=self.document_generation;
  let Some(index)=self.track_index(track)else{return false};
  let tempo=self.tempo();let tick=tempo.tick_at(self.playhead_seconds()).round()as i64;
  let bpm=tempo.points().iter().rev().find(|p|p.tick<=tick).map_or(120.,|p|p.micro_bpm as f64/1e6);
  let state=&self.integrations.performance;
  let same_instance=state.binding.as_ref().is_some_and(|(old,g,_)|old==track&&*g==generation)
   &&state.instrument_id.as_deref()==Some(id.as_str());
  if same_instance&&state.binding.as_ref().is_some_and(|(_,_,old)|*old==fingerprint)&&state.track_index==Some(index)&&state.tempo==Some(bpm){return true;}
  if same_instance{
   if state.track_index!=Some(index)&&!self.audio.move_monitor(index){return false;}
   let specs=dsp::instrument_specs(&id);let old=state.parameters.as_ref();
   let mut writes:Vec<(u32,f64)>=specs.iter().enumerate().filter_map(|(i,s)|{
    let value=params.get(s.id);
    (old.is_none_or(|p|p.get(s.id)!=value)).then_some((dsp::SYNTH_CONTROL_EVENT|i as u32,value))
   }).collect();
   if state.tempo!=Some(bpm)&&(20.0..=1000.0).contains(&bpm){writes.push((dsp::TRANSPORT_TEMPO_EVENT,bpm));}
   if !writes.is_empty()&&!self.audio.monitor_events(&writes){return false;}
   let state=&mut self.integrations.performance;state.binding=Some((track.clone(),generation,fingerprint));
   state.parameters=Some(params);state.track_index=Some(index);state.tempo=Some(bpm);return true;
  }
  let mut synth=RealtimeSynth::for_instrument(&id,&params,self.audio_info.sample_rate as f64);synth.set_tempo(bpm);
  let telemetry=synth.telemetry.clone();
  if !self.audio.set_monitor(index,Some(Box::new(synth))){self.notice("实时乐器队列忙，请重试");return false;}
  let state=&mut self.integrations.performance;
  state.binding=Some((track.clone(),generation,fingerprint));state.telemetry=Some(telemetry);state.snapshot=None;state.preview_keys=[false;128];
  state.instrument_id=Some(id);state.parameters=Some(params);state.track_index=Some(index);state.tempo=Some(bpm);state.stop_pending=false;state.preview_release_pending=false;state.meter_reset_pending=false;true
 }
 pub(crate) fn open_device_midi_input(&mut self,target:DeviceTarget,window:&mut Window,cx:&mut Context<Self>){
  self.integrations.performance.target=Some(target);
  if let Err(error)=self.run_integration(crate::editor::integrations::IntegrationAction::MonitorPorts,window,cx){self.notice(error);cx.notify();}
 }
 pub(crate) fn start_instrument_monitor(&mut self,port:&MidiPort,cx:&mut Context<Self>)->Result<(),String>{
  if self.integrations.recording.is_some(){return Err("请先完成或取消当前 MIDI 录音".into());}
  // Retire the previous performance before reopening: a failed port change
  // must not strand keys whose note-offs can no longer arrive.
  if self.integrations.performance.capture.take().is_some(){
   self.clear_instrument_monitor();
   let handles=self.native_effect_midi_handles(None);for handle in &handles{handle.stop_midi();}
   self.flush_effect_midi(handles,cx);
  }
  let capture=MidiInputService::open(port,None,65536).map_err(|e|e.to_string())?;
  let target=self.integrations.performance.target.clone().unwrap_or_else(||DeviceTarget::Track(self.session.track.clone()));
  let instrument=match &target{DeviceTarget::Track(track)=>self.project().track(track).and_then(crate::instrument::synth_params).map(|_|track.clone()),DeviceTarget::Master=>None};
  if let Some(track)=instrument{if !self.prepare_instrument_monitor(&track){return Err("实时乐器尚未准备好，请重试 MIDI 输入".into());}}
  else{self.clear_instrument_monitor();}
  self.integrations.performance.capture=Some(capture);self.integrations.performance.target=Some(target);
  self.notice(format!("MIDI 实时监听：{} · 全部通道",port.name));cx.notify();Ok(())
 }
 fn clear_instrument_monitor(&mut self){
  self.audio.reset_monitor();let pending=!self.audio.set_monitor(0,None);
  let state=&mut self.integrations.performance;state.binding=None;state.parameters=None;state.instrument_id=None;
  state.track_index=None;state.telemetry=None;state.snapshot=None;state.preview_keys=[false;128];state.preview_release_pending=false;state.meter_reset_pending=false;state.stop_pending=pending;
 }
 pub(crate) fn stop_instrument_monitor(&mut self,cx:&mut Context<Self>){
  self.clear_instrument_monitor();let pending=self.integrations.performance.stop_pending;
  self.integrations.performance=PerformanceState::default();self.integrations.performance.stop_pending=pending;
  let handles=self.native_effect_midi_handles(None);for handle in &handles{handle.stop_midi();}
  self.flush_effect_midi(handles,cx);
 }
 pub(crate) fn play_instrument_key(&mut self,track:&TrackId,key:u8,pressed:bool,cx:&mut Context<Self>){
  if key>127{return;}
  if pressed&&!self.prepare_instrument_monitor(track){return;}
  if !self.integrations.performance.binding.as_ref().is_some_and(|(t,g,_)|t==track&&*g==self.document_generation){return;}
  if self.integrations.performance.preview_keys[key as usize]==pressed{return;}
  let(id,_,value)=if pressed{proto::note_on(key,0,0.8)}else{proto::note_off(key,0)};
  let release=self.integrations.performance.preview_release_pending;
  let batch=[(dsp::SYNTH_PREVIEW_RELEASE,0.),(id|dsp::SYNTH_PREVIEW_NOTE,value)];
  if self.audio.monitor_events(if release{&batch}else{&batch[1..]}){
   if release{self.integrations.performance.preview_keys.fill(false);}
   self.integrations.performance.preview_keys[key as usize]=pressed;self.integrations.performance.preview_release_pending=false;
  }else{self.integrations.performance.preview_release_pending=true;self.notice("预览队列忙，正在释放预览音符");}
  cx.notify();
 }
 pub(crate) fn stop_instrument_preview(&mut self,track:&TrackId,cx:&mut Context<Self>){
  if !self.integrations.performance.binding.as_ref().is_some_and(|(owner,g,_)|owner==track&&*g==self.document_generation){return;}
  self.integrations.performance.preview_release_pending=!self.audio.monitor_events(&[(dsp::SYNTH_PREVIEW_RELEASE,0.)]);
  self.integrations.performance.preview_keys=[false;128];cx.notify();
 }
 pub(crate) fn reset_instrument_meters(&mut self,track:&TrackId,cx:&mut Context<Self>){
  let state=&mut self.integrations.performance;
  if !state.binding.as_ref().is_some_and(|(owner,g,_)|owner==track&&*g==self.document_generation){return;}
  state.meter_reset_pending=!self.audio.monitor_events(&[(dsp::SYNTH_RESET_METERS,0.)]);
  state.meter_reset_epoch=state.meter_reset_epoch.wrapping_add(1);
  state.snapshot=None;state.last_poll=Instant::now()-Duration::from_millis(100);cx.notify();
 }
 pub(crate) fn begin_device_midi_learn(&mut self,target:DeviceTarget,slot:usize,key:InsertKey,mapping_slot:usize,cx:&mut Context<Self>){
  if mapping_slot>=8||!self.device_target_matches(&target,slot,key){return;}
  self.integrations.performance.target=Some(target.clone());
  self.integrations.performance.learn=Some(MidiLearn{target,slot,key,mapping:mapping_slot,generation:self.document_generation});
  if self.integrations.performance.capture.is_none()&&self.integrations.recording.is_none(){
   if let Ok(ports)=MidiInputService::ports(){if ports.len()==1{let _=self.start_instrument_monitor(&ports[0],cx);}}
  }
  self.notice("MIDI 学习：移动控制器。可从「MIDI 输入」选择设备；新的 CC 映射可撤销。");cx.notify();
 }
 pub(crate) fn instrument_telemetry(&self,track:&TrackId)->Option<Arc<TelemetrySnapshot>>{
  let state=&self.integrations.performance;
  state.binding.as_ref().filter(|(t,g,_)|t==track&&*g==self.document_generation)?;
  state.snapshot.clone()
 }
 pub(crate) fn pump_instrument_monitor(&mut self,cx:&mut Context<Self>){
  if self.integrations.performance.stop_pending&&self.audio.set_monitor(0,None){self.integrations.performance.stop_pending=false;}
  if let Some((track,generation,_))=self.integrations.performance.binding.clone(){
   if generation!=self.document_generation||self.project().track(&track).and_then(crate::instrument::synth_params).is_none(){self.stop_instrument_monitor(cx);return;}
   self.prepare_instrument_monitor(&track);
  }
  if self.integrations.performance.preview_release_pending&&self.integrations.performance.binding.is_some()
   &&self.audio.monitor_events(&[(dsp::SYNTH_PREVIEW_RELEASE,0.)]){
   self.integrations.performance.preview_release_pending=false;self.integrations.performance.preview_keys=[false;128];
  }
  if self.integrations.performance.meter_reset_pending&&self.integrations.performance.binding.is_some()
   &&self.audio.monitor_events(&[(dsp::SYNTH_RESET_METERS,0.)]){
   self.integrations.performance.meter_reset_pending=false;self.integrations.performance.snapshot=None;
  }
  let mut events=Vec::new();let mut failure=None;
  if let Some(capture)=&mut self.integrations.performance.capture{for _ in 0..8192{
   match capture.try_recv(){Ok(Some(event))=>events.push(event),Ok(None)=>break,Err(error)=>{failure=Some(error.to_string());break;}}
  }}
  if let Some(error)=failure{self.stop_instrument_monitor(cx);self.notice(format!("MIDI 监听已停止：{error}"));cx.notify();}
  else{self.consume_performance_events(&events,cx);}
  let state=&mut self.integrations.performance;
  if state.polling||state.last_poll.elapsed()<Duration::from_millis(100){return;}
  let Some(telemetry)=state.telemetry.clone()else{return};
  state.polling=true;state.last_poll=Instant::now();let binding=state.binding.clone();let meter_epoch=state.meter_reset_epoch;
  cx.spawn(async move|this,cx|{
   let snapshot=cx.background_spawn(async move{telemetry.snapshot()}).await;
   let _=this.update(cx,|ed,cx|{let state=&mut ed.integrations.performance;state.polling=false;
    if state.binding!=binding||state.meter_reset_epoch!=meter_epoch{return;}
    if let Some(snapshot)=snapshot{if state.snapshot.as_ref().is_none_or(|old|old.frames_total!=snapshot.frames_total){
     state.snapshot=Some(Arc::new(snapshot));cx.notify();
    }}
   });
  }).detach();
 }
 pub(crate) fn consume_performance_events(&mut self,events:&[CapturedEvent],cx:&mut Context<Self>){
  if events.is_empty(){return;}
  let messages:Vec<(u32,f64)>=events.iter().copied().map(native_event).collect();
  if self.integrations.performance.binding.as_ref().is_some_and(|(_,g,_)|*g==self.document_generation){
   for &(id,value)in &messages{if !self.audio.monitor_event(id,value){self.notice("MIDI 队列已满，已释放全部发声");break;}}
  }
  if let Some(learn)=self.integrations.performance.learn.take(){
   if learn.generation==self.document_generation&&self.device_target_matches(&learn.target,learn.slot,learn.key){
    if let Some(proto::MidiEvent::Control{channel,controller,..})=messages.iter().find_map(|&(id,v)|proto::decode_midi(id,v).filter(|m|matches!(m,proto::MidiEvent::Control{..}))){
     let n=learn.mapping+1;let writes=vec![(format!("midi{n}_cc"),f64::from(controller)),(format!("midi{n}_channel"),f64::from(channel)+1.)];
     if self.set_device_params(learn.target.clone(),learn.slot,writes,cx){self.notice(format!("已学习 CC {controller} · 通道 {}",channel+1));}
     else{self.integrations.performance.learn=Some(learn);}
    }else{self.integrations.performance.learn=Some(learn);}
   }
  }
  if let Some((track,generation,_))=self.integrations.performance.binding.clone(){
   if generation==self.document_generation{
    if let Some(id)=self.integrations.performance.instrument_id.clone(){
     if let Some(params)=self.project().track(&track).and_then(crate::instrument::synth_params){
      let writes=learned_instrument_changes(&id,&params,&messages);
      if !writes.is_empty(){self.set_instrument_params(&track,writes,cx);}
     }
    }
   }
  }
  let target=self.integrations.performance.target.clone().unwrap_or_else(||DeviceTarget::Track(self.session.track.clone()));
  let handles=self.native_effect_midi_handles(Some(target));
  for handle in &handles{if let Err(error)=handle.enqueue_midi(&messages){handle.stop_midi();self.notice(format!("MIDI 输入已释放：{error}"));}}
  self.flush_effect_midi(handles,cx);
 }
 fn native_effect_midi_handles(&self,target:Option<DeviceTarget>)->Vec<Arc<verdant_plugins::InsertHandle>>{
  let owners:Vec<_>=match target{Some(target)=>vec![target,DeviceTarget::Master],None=>self.project().tracks.iter().map(|t|DeviceTarget::Track(t.id.clone())).chain([DeviceTarget::Master]).collect()};
  let mut handles=Vec::new();
  for owner in owners{for(slot,spec)in self.device_insert_specs(&owner).iter().enumerate(){
   if spec.format=="builtin"&&dsp::EFFECT_IDS.contains(&spec.id.as_str()){if let Some(handle)=self.live_device_insert(&owner,slot){
    if !handles.iter().any(|h:&Arc<verdant_plugins::InsertHandle>|h.key()==handle.key()){handles.push(handle);}
   }}
  }}handles
 }
 fn flush_effect_midi(&mut self,handles:Vec<Arc<verdant_plugins::InsertHandle>>,cx:&mut Context<Self>){
  if handles.is_empty(){return;}let generation=self.document_generation;
  cx.spawn(async move|this,cx|{
   let result=cx.background_spawn(async move{for handle in handles{handle.flush_midi().map_err(|e|e.to_string())?;}Ok::<_,String>(())}).await;
   let _=this.update(cx,|ed,cx|{if ed.document_generation==generation{if let Err(error)=result{ed.notice(format!("MIDI 插件输入：{error}"));cx.notify();}}});
  }).detach();
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn midi_expression_preserves_channel_and_full_bend_range(){
  for value in[-8192,0,8191]{let e=CapturedEvent{timestamp_micros:0,channel:7,kind:EventKind::PitchBend{value}};
   let(id,v)=native_event(e);assert!(matches!(proto::decode_midi(id,v),Some(proto::MidiEvent::PitchBend{channel:7,..})));assert!((-1.0..=1.0).contains(&v));}
  let(id,v)=native_event(CapturedEvent{timestamp_micros:0,channel:2,kind:EventKind::Control{controller:74,value:127}});
  assert_eq!(proto::decode_midi(id,v),Some(proto::MidiEvent::Control{channel:2,controller:74,value:1.}));
 }
}
