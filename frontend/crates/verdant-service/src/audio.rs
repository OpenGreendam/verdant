//! Offline export uses the actual audio mixer, its automation clock and its
//! output rate converter. Native inserts have fresh, export-only host owners.
//! Every output scope passes its selected sources through the master chain;
//! disabling effects bypasses both track and master inserts.
use std::{fs::{File,OpenOptions},io::{BufWriter,Write},path::{Path,PathBuf},sync::{Arc,Mutex}};
use verdant_domain::{NoteId,Project,TempoMap,TrackId};
use verdant_plugins::{InsertHandle,PluginClient};
use crate::project_audio::{self,PhraseSource as Source};

const RATE:u32=48_000;
const BLOCK:usize=verdant_plugins::shm::DSP_QUANTUM_FRAMES;
/// Enabled insert tails are bounded independently for each source track,
/// followed by the same bound for an enabled master chain.
pub const INSERT_TAIL_SECONDS:u32=10;
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub enum WavEncoding{#[default]Float32,Pcm16,Pcm24}
impl WavEncoding{fn bytes(self)->u16{match self{Self::Pcm16=>2,Self::Pcm24=>3,Self::Float32=>4}}}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub enum AudioScope{#[default]Mix,ActiveTrack,Stems}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
pub enum ExportRange{#[default]Full,Loop,Selection}
#[derive(Clone,Debug,serde::Serialize,serde::Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct ExportOptions{pub sample_rate:u32,pub channels:u16,pub encoding:WavEncoding,pub scope:AudioScope,pub range:ExportRange,pub effects:bool}
impl Default for ExportOptions{fn default()->Self{Self{sample_rate:RATE,channels:2,encoding:WavEncoding::Float32,scope:AudioScope::Mix,range:ExportRange::Full,effects:true}}}
/// Snapshot-specific export inputs. Explicit range endpoints are project ticks;
/// a full export leaves `range_ticks` absent and includes the bounded tails.
pub struct RenderSettings {
    pub options: ExportOptions,
    pub active_track: TrackId,
    pub range_ticks: Option<(i64, i64)>,
    pub media_policy: verdant_media::AssetPolicy,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RenderedAudio {
    /// Frames in the longest output, at the requested file sample rate.
    pub frames: u64,
    /// Actual published paths, including the generated names for stems.
    pub paths: Vec<PathBuf>,
}

/// This private processor is constructed and driven exclusively on the export
/// worker. Its synchronous host call never enters the device callback graph.
struct OfflineInsert{handle:InsertHandle,error:Arc<Mutex<Option<String>>>,latency:u32}
impl verdant_audio::Processor for OfflineInsert{
    fn name(&self)->&str{&self.handle.spec().name}
    fn process(&mut self,left:&mut[f32],right:&mut[f32]){let _=self.process_automated(left,right,&[]);}
    fn process_automated(&mut self,left:&mut[f32],right:&mut[f32],events:&[(u32,u32,f64)])->bool{self.run(left,right,None,events)}
    fn process_keyed(&mut self,left:&mut[f32],right:&mut[f32],key:(&[f32],&[f32]),events:&[(u32,u32,f64)])->bool{self.run(left,right,Some(key),events)}
}
impl OfflineInsert{
    fn run(&mut self,left:&mut[f32],right:&mut[f32],key:Option<(&[f32],&[f32])>,events:&[(u32,u32,f64)])->bool{
        let result=self.handle.process_offline_keyed(left,right,key,events).map_err(|e|e.to_string()).and_then(|()|{
            if self.handle.plugin_latency_samples()!=self.latency{return Err("插件在导出期间改变延迟，请重新导出".into());}
            if left.iter().chain(right.iter()).any(|v|!v.is_finite()){return Err("插件输出含非有限 PCM".into());}Ok(())
        });
        if let Err(error)=result{let mut first=self.error.lock().unwrap_or_else(|p|p.into_inner());if first.is_none(){*first=Some(format!("插件 {}：{error}",self.handle.spec().name));}left.fill(0.);right.fill(0.);false}else{true}
    }
}
struct TailGate{position:u64,end:u64}
impl verdant_audio::Processor for TailGate{
    fn name(&self)->&str{"Export tail boundary"}
    fn process(&mut self,left:&mut[f32],right:&mut[f32]){let keep=self.end.saturating_sub(self.position).min(left.len()as u64)as usize;left[keep..].fill(0.);right[keep..].fill(0.);self.position=self.position.saturating_add(left.len()as u64);}
}

/// Render verified phrase/media sources through the same engine used by
/// playback. The caller supplies already committed project and plugin state.
/// Cancellation is checked between render blocks and before publication; no
/// file is published for a cancelled render. `overwrite=false` atomically
/// rejects an existing destination, including one created during rendering.
pub fn render_outputs(
    path: &Path,
    project: &Project,
    sources: Vec<Source>,
    plugins: Option<&PluginClient>,
    settings: &RenderSettings,
    overwrite: bool,
    cancelled: &dyn Fn() -> bool,
) -> Result<RenderedAudio, String> {
    check_cancelled(cancelled)?;
    if ![44_100,48_000,88_200,96_000,192_000].contains(&settings.options.sample_rate)||!matches!(settings.options.channels,1|2){return Err("不支持的 WAV 采样率或声道数".into());}
    let prepared=project_audio::prepare_timeline(project,&sources,settings.media_policy.clone(),RATE,Default::default(),None)?;
    check_cancelled(cancelled)?;
    if prepared.timeline.length==0{return Err("当前工程没有可导出的已校验音频".into());}
    let targets=match settings.options.scope{AudioScope::Mix=>vec![None],AudioScope::ActiveTrack=>vec![Some(settings.active_track.clone())],AudioScope::Stems=>project.tracks.iter().map(|t|Some(t.id.clone())).collect()};
    let mut outputs=Vec::new();let mut frames=0;
    for target in targets{
        check_cancelled(cancelled)?;
        let destination=if settings.options.scope==AudioScope::Stems{
            let index=project.tracks.iter().position(|t|Some(&t.id)==target.as_ref()).ok_or("分轨目标不存在")?;
            let stem=path.file_stem().unwrap_or_default().to_string_lossy();let track=&project.tracks[index];let name=track.name.chars().map(|c|if c.is_alphanumeric()||c=='-'||c=='_'{c}else{'_'}).take(80).collect::<String>();path.with_file_name(format!("{stem}-{:02}-{name}.wav",index+1))
        }else{path.to_path_buf()};
        let(wav,count)=render_one(&destination,project,prepared.timeline.clone(),plugins,settings,target.as_ref(),cancelled)?;frames=frames.max(count);outputs.push(wav);
    }
    for output in &mut outputs {
        check_cancelled(cancelled)?;
        output.finish()?;
    }
    // Flush and sync every temporary before accepting the final cancellation
    // check. Publication then commits the completed output set without another
    // cancellation point between individual stems.
    check_cancelled(cancelled)?;
    let mut paths=Vec::with_capacity(outputs.len());
    for output in outputs {
        let destination=output.destination.clone();
        output.publish(overwrite)?;
        paths.push(destination);
    }
    Ok(RenderedAudio{frames,paths})
}
fn check_cancelled(cancelled:&dyn Fn()->bool)->Result<(),String>{
    if cancelled(){Err("音频导出已取消".into())}else{Ok(())}
}

fn render_one(path:&Path,project:&Project,mut timeline:verdant_audio::Timeline,plugins:Option<&PluginClient>,settings:&RenderSettings,target:Option<&TrackId>,cancelled:&dyn Fn()->bool)->Result<(AtomicWav,u64),String>{
    use verdant_audio::engine::ChainInput;
    let document=verdant_domain::automation::AutomationDocument::from_project(project).map_err(|e|e.to_string())?;
    let transport_tempo=project_audio::transport_tempo_curve(project,RATE)?;
    let errors=Arc::new(Mutex::new(None));let mut chains=Vec::with_capacity(project.tracks.len());let mut schedules=Vec::with_capacity(project.tracks.len());let mut latencies=Vec::with_capacity(project.tracks.len());let mut ends=Vec::with_capacity(project.tracks.len());
    let target_slot=target.map(|id|project.tracks.iter().position(|t|&t.id==id).ok_or("导出轨道不存在")).transpose()?;
    // A stem keeps its sidechain sources' material: they are muted below but
    // still key the target's inserts.
    if let Some(slot)=target_slot{
        let keys:std::collections::BTreeSet<usize>=verdant_plugins::project::read_inserts(&project.tracks[slot].unknown).iter().filter_map(project_audio::sidechain_source).filter_map(|id|project.tracks.iter().position(|t|t.id.as_str()==id)).chain([slot]).collect();
        timeline.clips.retain(|c|keys.contains(&(c.track as usize)));timeline.stereo_clips.retain(|c|keys.contains(&(c.track as usize)));
    }
    let mut sidechains=Vec::new();
    for(index,track)in project.tracks.iter().enumerate(){
        let end=if target_slot.is_some_and(|slot|slot!=index){0}else{timeline.clips.iter().filter(|c|c.track as usize==index).map(|c|c.start+c.samples.len()as u64).chain(timeline.stereo_clips.iter().filter(|c|c.track as usize==index).map(|c|c.start+(c.samples.len()/2)as u64)).max().unwrap_or(0)};
        let mut chain:Vec<ChainInput>=Vec::new();let mut native=Vec::new();let mut latency=0u32;let mut matched=std::collections::BTreeSet::new();
        if target_slot.is_none_or(|slot|slot==index)&&settings.options.effects{
            for spec in verdant_plugins::project::read_inserts(&track.unknown){
                if spec.bypass{matched.insert(spec.instance_id.clone());continue;}
                let client=plugins.ok_or_else(||format!("轨道 {} 需要插件 {}，但宿主不可用",track.name,spec.name))?;
                let handle=client.offline_insert(spec.clone()).map_err(|e|format!("插件 {}：{e}",spec.name))?;
                let schedule=project_audio::plugin_schedule(&timeline.automation.tracks[index],&document,&verdant_domain::automation::AutomationOwner::Track{track_id:track.id.clone()},&spec.instance_id,&handle,&transport_tempo,latency,None)?;
                sidechains.extend(project_audio::sidechain_route(project,index,&spec,chain.len(),latency));
                matched.insert(spec.instance_id.clone());let own=handle.plugin_latency_samples();latency=latency.checked_add(own).ok_or("插件延迟超出范围")?;
                native.push(schedule);chain.push(ChainInput::New(Box::new(OfflineInsert{handle,error:errors.clone(),latency:own})));
            }
            if timeline.automation.tracks[index].plugins.iter().any(|lane|!matched.contains(&lane.instance_id)){return Err(format!("轨道 {} 的参数自动化缺少已安装插件",track.name));}
        }
        let tail=if end>0&&!chain.is_empty(){u64::from(RATE)*u64::from(INSERT_TAIL_SECONDS)}else{0};
        ends.push(end.checked_add(tail).ok_or("尾音范围溢出")?);latencies.push(latency);chains.push(chain);schedules.push(native);
    }
    let maximum=latencies.iter().copied().max().unwrap_or(0);
    let mut master_chain:verdant_audio::engine::Chain=Vec::new();let mut master_native=Vec::new();let mut master_latency=0u32;let mut matched=std::collections::BTreeSet::new();
    if settings.options.effects{
        for spec in project_audio::master_insert_specs(project){
            if spec.bypass{matched.insert(spec.instance_id.clone());continue;}
            let client=plugins.ok_or_else(||format!("主总线需要插件 {}，但宿主不可用",spec.name))?;
            let handle=client.offline_insert(spec.clone()).map_err(|error|format!("主总线插件 {}：{error}",spec.name))?;
            let input_latency=maximum.checked_add(master_latency).ok_or("主总线插件节拍延迟超出范围")?;
            let schedule=project_audio::plugin_schedule(&timeline.automation.master,&document,&verdant_domain::automation::AutomationOwner::Master,&spec.instance_id,&handle,&transport_tempo,input_latency,None)?;
            matched.insert(spec.instance_id.clone());let own=handle.plugin_latency_samples();master_latency=master_latency.checked_add(own).ok_or("主总线插件延迟超出范围")?;
            master_native.push(schedule);master_chain.push(Box::new(OfflineInsert{handle,error:errors.clone(),latency:own}));
        }
        if timeline.automation.master.plugins.iter().any(|lane|!matched.contains(&lane.instance_id)){return Err("主总线的参数自动化缺少已安装插件".into());}
    }
    let source_length=ends.iter().copied().max().unwrap_or(0);if source_length==0{return Err("所选轨道没有可导出的音频".into());}
    let master_tail=if master_chain.is_empty(){0}else{u64::from(RATE)*u64::from(INSERT_TAIL_SECONDS)};
    let length=source_length.checked_add(master_tail).ok_or("主总线尾音范围溢出")?;
    let latency=maximum.checked_add(master_latency).ok_or("总插件延迟超出范围")?;
    for(index,chain)in chains.iter_mut().enumerate(){
        let padding=maximum-latencies[index];if padding>0{chain.push(ChainInput::New(Box::new(verdant_plugins::DelayPad::new(padding))));schedules[index].push(None);}
        if !chain.is_empty(){chain.push(ChainInput::New(Box::new(TailGate{position:0,end:ends[index].checked_add(u64::from(maximum)).ok_or("轨道尾音时钟溢出")?})));schedules[index].push(None);}
    }
    let processed_end=length.checked_add(u64::from(latency)).ok_or("导出时钟溢出")?;
    if !master_chain.is_empty(){master_chain.push(Box::new(TailGate{position:0,end:processed_end}));master_native.push(None);}
    timeline.length=processed_end;timeline.native_automation=schedules;timeline.master_native_automation=master_native;timeline.loop_range=None;timeline.metronome=None;timeline.sidechains=sidechains;
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).map_err(|e|e.to_string())?;
    let(start,end)=if let Some((a,z))=settings.range_ticks{let start=tempo.sample_at(a,RATE).max(0)as u64;let end=tempo.sample_at(z,RATE).max(0)as u64;if end<=start{return Err("导出时间范围为空".into());}(start,end)}else{(0,length)};
    // An explicit range may include silence after the last source. Effects
    // still stop at each track's and the master's documented tail boundary.
    timeline.length=timeline.length.max(end.checked_add(u64::from(latency)).ok_or("导出时间范围溢出")?);
    let rate=settings.options.sample_rate;let output_frames=((end-start)as f64*rate as f64/RATE as f64).round()as u64;
    let skip=(start.checked_add(u64::from(latency)).ok_or("导出起点溢出")?as f64*rate as f64/RATE as f64).round()as u64;
    let(mut control,rt)=verdant_audio::engine::channel(RATE,BLOCK);let any_solo=target.is_none()&&project.tracks.iter().any(|t|t.solo);
    for(index,track)in project.tracks.iter().enumerate(){let mut channel=project_audio::channel_settings(&track.header(),any_solo);if let Some(slot)=target_slot{channel.solo=false;if slot!=index{channel.mute=true;}}control.set_channel(index,channel);}
    if target_slot.is_none() { control.set_master(project_audio::MasterBusSettings::from_project(project).channel_settings()); }
    control.remap_chains(chains).map_err(|_|"无法提交离线插件链")?;if !control.set_master_chain(master_chain){return Err("无法提交离线主总线插件链".into());}control.set_timeline(timeline);control.play();
    let mut renderer=verdant_audio::CallbackRenderer::new(rt,RATE);renderer.set_device_rate(rate).map_err(|e|e.to_string())?;
    let mut wav=AtomicWav::new(path,output_frames,&settings.options)?;let channels=settings.options.channels as usize;let mut samples=vec![0f32;BLOCK*channels];let mut encoded=vec![0u8;BLOCK*channels*4];
    let mut position=0;let total=skip.checked_add(output_frames).ok_or("导出时长溢出")?;
    while position<total{
        check_cancelled(cancelled)?;
        let count=(total-position).min(BLOCK as u64)as usize;renderer.process_f32(&mut samples[..count*channels],channels).map_err(|e|e.to_string())?;
        control.maintain();if let Some(error)=errors.lock().unwrap_or_else(|p|p.into_inner()).as_ref(){return Err(error.clone());}if control.shared.automation_fault.load(std::sync::atomic::Ordering::Acquire){return Err("原生插件自动化拒绝处理，未发布 WAV".into());}
        let from=skip.saturating_sub(position).min(count as u64)as usize;if from<count{
            let selected=&samples[from*channels..count*channels];if selected.iter().any(|sample|!sample.is_finite()){return Err("混音输出含非有限 PCM".into());}
            let bytes=encode(selected,settings.options.encoding,&mut encoded);wav.write(&encoded[..bytes])?;
        }position+=count as u64;
    }
    Ok((wav,output_frames))
}
fn encode(samples:&[f32],encoding:WavEncoding,out:&mut[u8])->usize{
    let width=encoding.bytes()as usize;
    for(sample,bytes)in samples.iter().zip(out.chunks_exact_mut(width)){match encoding{
        WavEncoding::Float32=>bytes.copy_from_slice(&sample.to_le_bytes()),
        WavEncoding::Pcm16=>bytes.copy_from_slice(&((*sample as f64*32768.).round().clamp(-32768.,32767.)as i16).to_le_bytes()),
        WavEncoding::Pcm24=>bytes.copy_from_slice(&((*sample as f64*8_388_608.).round().clamp(-8_388_608.,8_388_607.)as i32).to_le_bytes()[..3]),
    }}samples.len()*width
}
struct AtomicWav{destination:PathBuf,temporary:PathBuf,writer:Option<BufWriter<File>>,pad:bool}
impl AtomicWav{
    fn new(destination:&Path,frames:u64,options:&ExportOptions)->Result<Self,String>{
        let directory=destination.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));let name=destination.file_name().ok_or("导出路径缺少文件名")?.to_string_lossy();let temporary=directory.join(format!(".{name}.{}.tmp",NoteId::generate()));
        let file=OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e|e.to_string())?;
        let width=options.encoding.bytes();let align=width*options.channels;let bytes=frames.checked_mul(u64::from(align)).ok_or("WAV 大小溢出")?;let pad=bytes%2!=0;let floating=options.encoding==WavEncoding::Float32;let base=36+if floating{12}else{0};let rf64=bytes+base+u64::from(pad)>u32::MAX as u64;
        let mut out=Self{destination:destination.into(),temporary,writer:Some(BufWriter::new(file)),pad};
        out.write(if rf64{b"RF64"}else{b"RIFF"})?;out.write(&(if rf64{u32::MAX}else{(bytes+base+u64::from(pad))as u32}).to_le_bytes())?;out.write(b"WAVE")?;
        if rf64{out.write(b"ds64")?;out.write(&28u32.to_le_bytes())?;out.write(&(bytes+base+36+u64::from(pad)).to_le_bytes())?;out.write(&bytes.to_le_bytes())?;out.write(&frames.to_le_bytes())?;out.write(&0u32.to_le_bytes())?;}
        out.write(b"fmt ")?;out.write(&16u32.to_le_bytes())?;out.write(&(if floating{3u16}else{1u16}).to_le_bytes())?;out.write(&options.channels.to_le_bytes())?;out.write(&options.sample_rate.to_le_bytes())?;out.write(&(options.sample_rate*u32::from(align)).to_le_bytes())?;out.write(&align.to_le_bytes())?;out.write(&(width*8).to_le_bytes())?;
        if floating{out.write(b"fact")?;out.write(&4u32.to_le_bytes())?;out.write(&(frames.min(u32::MAX as u64)as u32).to_le_bytes())?;}
        out.write(b"data")?;out.write(&(if rf64{u32::MAX}else{bytes as u32}).to_le_bytes())?;Ok(out)
    }
    fn write(&mut self,bytes:&[u8])->Result<(),String>{self.writer.as_mut().expect("unpublished WAV").write_all(bytes).map_err(|e|e.to_string())}
    fn finish(&mut self)->Result<(),String>{
        if self.pad{self.write(&[0])?;}
        let mut writer=self.writer.take().expect("unpublished WAV");
        writer.flush().map_err(|e|e.to_string())?;
        writer.get_ref().sync_all().map_err(|e|e.to_string())?;
        Ok(())
    }
    fn publish(self,overwrite:bool)->Result<(),String>{
        let result=if overwrite{
            std::fs::rename(&self.temporary,&self.destination)
        }else{
            std::fs::hard_link(&self.temporary,&self.destination)
        };
        result.map_err(|e|format!("{}：{e}",self.destination.display()))
    }
}
impl Drop for AtomicWav{fn drop(&mut self){drop(self.writer.take());let _=std::fs::remove_file(&self.temporary);}}
#[cfg(test)]#[path="audio_tests.rs"]mod tests;
