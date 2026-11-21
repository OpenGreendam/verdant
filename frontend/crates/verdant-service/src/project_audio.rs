//! One source-clock preparation path for playback and offline export, including
//! native automation on track inserts and on the summed master insert chain.
use std::{collections::HashMap,sync::Arc};
use verdant_audio::{ChannelSettings,Clip,StereoClip,Timeline};
use verdant_backend::{VerifiedAudio,wire::PhonemeTimingTrace};
use verdant_domain::{Note,NoteId,Project,TempoMap,TrackId};
use verdant_media::{AssetPolicy,ImportLimits,MediaImporter,WaveformCache,WaveformPreview};

#[derive(Clone)]
pub struct PhraseSource {
    pub track:TrackId,pub start_seconds:f64,pub note_ids:Vec<NoteId>,pub phoneme_timing:Option<PhonemeTimingTrace>,pub audio:VerifiedAudio,
    /// Present for live, digest-bound playback sources. Exports prepare their own PCM.
    pub playback_key: Option<(usize, String)>,
}

#[derive(PartialEq, Eq, Hash)]
struct PhrasePcmKey {
    track: TrackId,
    slot: usize,
    digest: String,
    rate: u32,
    start_seconds: u64,
}

#[derive(Clone)]
struct CachedPhrasePcm {
    original: Arc<[f32]>,
    prepared: Arc<[f32]>,
}

/// Derived PCM for the last published timeline, scoped to its project revision.
#[derive(Default)]
pub struct PhrasePcmCache(HashMap<PhrasePcmKey, CachedPhrasePcm>);

impl PhrasePcmCache {
    pub fn for_sources(&self, sources: &[PhraseSource], rate: u32) -> Self {
        Self(sources.iter().filter_map(|source| {
            let key = source.pcm_key(rate)?;
            self.0.get(&key).map(|pcm| (key, pcm.clone()))
        }).collect())
    }
}

impl PhraseSource {
    fn pcm_key(&self, rate: u32) -> Option<PhrasePcmKey> {
        let (slot, digest) = self.playback_key.as_ref()?;
        Some(PhrasePcmKey { track: self.track.clone(), slot: *slot, digest: digest.clone(), rate, start_seconds: self.start_seconds.to_bits() })
    }
}

/// Waveform previews of verified media sources. The caller owns the cache and
/// the collected previews, and publishes them only after preparation succeeds.
pub struct MediaPreparation<'a> { pub cache:&'a mut WaveformCache,pub previews:&'a mut HashMap<String,Arc<WaveformPreview>> }
pub struct PreparedTimeline {
    pub timeline:Timeline,pub phrase_pcm:PhrasePcmCache,pub phrase_count:usize,pub media_count:usize,
    /// Instrument tracks that rendered at least one note.
    pub instrument_count:usize,
}
pub fn prepare_timeline(project:&Project,sources:&[PhraseSource],policy:AssetPolicy,rate:u32,mut cache:PhrasePcmCache,media:Option<&mut MediaPreparation>)->Result<PreparedTimeline,String>{
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).map_err(|e|e.to_string())?;
    let mut clips=Vec::with_capacity(sources.len());
    for source in sources {
        let index=project.tracks.iter().position(|t|t.id==source.track).ok_or("渲染结果所属轨道已不存在")?;
        if !source.start_seconds.is_finite()||source.start_seconds<0.{return Err("渲染结果位置无效".into());}
        let key = source.pcm_key(rate);
        let cached = key.as_ref().and_then(|key| cache.0.get(key))
            .filter(|pcm| Arc::ptr_eq(&pcm.original, &source.audio.samples));
        let samples = if let Some(pcm) = cached {
            pcm.prepared.clone()
        } else {
            let track = &project.tracks[index];
            let notes = source.note_ids.iter().map(|id| track.note(id).ok_or("渲染结果所属音符已不存在")).collect::<Result<Vec<_>, _>>()?;
            let prepared = prepare_phrase_pcm(&source.audio, &notes, &tempo, source.start_seconds, source.phoneme_timing.as_ref(), rate)?;
            if let Some(key) = key {
                cache.0.insert(key, CachedPhrasePcm { original: source.audio.samples.clone(), prepared: prepared.clone() });
            }
            prepared
        };
        clips.push(Clip{start:(source.start_seconds*rate as f64).round()as u64,samples,gain:0.9,track:index as u16});
    }
    let phrase_count=clips.len();let importer=MediaImporter::new(policy,ImportLimits::default());
    let prepared_media=match media{
        Some(preparation)=>importer.prepare_project_clips_with(project,&tempo,rate,|clip,audio|{
            let frames=audio.frames();let start=clip.source_offset_frames.min(frames);let end=clip.source_offset_frames.saturating_add(clip.duration_frames).min(frames);
            if end<=start{return;}
            let bins=((end-start)as usize).div_ceil(1024).clamp(1,4096);
            if let Ok(preview)=preparation.cache.preview(audio,start..end,bins){preparation.previews.insert(clip.id.to_string(),preview);}
        }).map_err(|e|e.to_string())?,
        None=>importer.prepare_project_clips(project,&tempo,rate).map_err(|e|e.to_string())?,
    };
    let media_count=prepared_media.len();let mut stereo=Vec::new();
    let mut instrument_count=0;
    for(index,track)in project.tracks.iter().enumerate(){
        if let Some(clip)=render_instrument(track,index,&tempo,rate)?{stereo.push(clip);instrument_count+=1;}
    }
    for clip in prepared_media{if clip.channels==1{clips.push(Clip{start:clip.start,samples:clip.samples,gain:clip.gain,track:clip.track});}else{stereo.push(StereoClip{start:clip.start,samples:clip.samples,gain:clip.gain,pan:clip.pan,track:clip.track});}}
    let mut timeline=Timeline::with_stereo(rate,clips,stereo);timeline.automation=verdant_backend::automation_plan::compile_automation(project,rate).map_err(|e|e.to_string())?;
    Ok(PreparedTimeline{timeline,phrase_pcm:cache,phrase_count,media_count,instrument_count})
}

pub fn master_insert_specs(project:&Project)->Vec<verdant_plugins::InsertSpec>{
    project.extensions.get(verdant_domain::automation::MASTER_INSERTS_KEY).and_then(serde_json::Value::as_array)
        .map(|specs|specs.iter().filter_map(|spec|serde_json::from_value(spec.clone()).ok()).collect()).unwrap_or_default()
}

/// Compile persisted native values against the actual hosted parameter descriptor.
pub fn plugin_schedule(channel:&verdant_audio::automation_clock::ChannelAutomation,document:&verdant_domain::automation::AutomationDocument,owner:&verdant_domain::automation::AutomationOwner,instance_id:&str,handle:&verdant_plugins::InsertHandle)->Result<Option<verdant_audio::automation_clock::NativePluginSchedule>,String>{
    use verdant_audio::automation_clock::{NativeParameterLane,NativePluginSchedule};
    use verdant_domain::automation::AutomationTarget;
    let mut schedule=NativePluginSchedule{instance_id:instance_id.into(),parameters:Vec::new()};
    for lane in channel.plugins.iter().filter(|lane|lane.instance_id==instance_id){
        let target=handle.compile_parameter(&lane.parameter_id).map_err(|e|e.to_string())?;
        let authored=document.lanes.iter().find(|authored|matches!(&authored.target,AutomationTarget::PluginParameter{owner:authored_owner,instance_id:instance,parameter_id}if authored_owner==owner&&instance==instance_id&&parameter_id==&lane.parameter_id)).ok_or("自动化原始曲线不存在")?;
        authored.validate_parameter_range(target.min(),target.max()).map_err(|e|e.to_string())?;authored.validate_parameter_steps(target.step_count()).map_err(|e|e.to_string())?;for point in &authored.points{target.event(0,point.value).map_err(|e|e.to_string())?;}
        schedule.parameters.push(NativeParameterLane{native_id:target.native_id(),fallback:target.default_value(),curve:lane.curve.clone()});
    }
    if schedule.parameters.is_empty(){return Ok(None);}schedule.validate_density(256,4096).map_err(|e|e.to_string())?;Ok(Some(schedule))
}

static INSTRUMENT_HOST:std::sync::Mutex<Option<Arc<verdant_plugins::PluginClient>>>=std::sync::Mutex::new(None);

/// The plug-in host instrument tracks render third-party instruments with.
pub fn set_instrument_host(client:Option<Arc<verdant_plugins::PluginClient>>){*INSTRUMENT_HOST.lock().unwrap_or_else(|p|p.into_inner())=client;}
fn instrument_host()->Option<Arc<verdant_plugins::PluginClient>>{INSTRUMENT_HOST.lock().unwrap_or_else(|p|p.into_inner()).clone()}

/// Seconds rendered after the last note-off so instrument releases ring out.
const INSTRUMENT_TAIL_SECONDS:f64=2.0;

/// Render notes through a hosted CLAP/VST3/AU instrument, offline, at the
/// host's rate, resampled to `rate`. Interleaved stereo.
pub fn render_hosted(client:&verdant_plugins::PluginClient,spec:&serde_json::Value,notes:&[verdant_plugins::builtin_dsp::SynthNote],rate:u32)->Result<Vec<f32>,String>{
    use verdant_plugins::proto::{note_off,note_on};
    let insert:verdant_plugins::InsertSpec=serde_json::from_value(spec.clone()).map_err(|e|format!("乐器设置无效：{e}"))?;
    let mut handle=client.offline_insert(insert).map_err(|e|e.to_string())?;
    let host_rate=client.sample_rate();let scale=host_rate/rate as f64;
    // Every on/off at its host-rate frame; offs sort before ons at one frame.
    let mut events:Vec<(u64,bool,u8,f64)>=Vec::with_capacity(notes.len()*2);
    for n in notes{
        let key=n.pitch.round().clamp(0.,127.)as u8;let start=(n.start as f64*scale).round()as u64;let end=((n.start+n.length)as f64*scale).round()as u64;
        events.push((start,true,key,n.velocity));events.push((end.max(start+1),false,key,0.));
    }
    events.sort_by(|a,b|a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let total=events.last().map_or(0,|e|e.0)+(INSTRUMENT_TAIL_SECONDS*host_rate)as u64;
    const BLOCK:usize=4096;
    let (mut out_l,mut out_r)=(Vec::with_capacity(total as usize),Vec::with_capacity(total as usize));
    let (mut l,mut r)=(vec![0f32;BLOCK],vec![0f32;BLOCK]);
    let mut next=0;let mut position=0u64;
    while position<total{
        let n=((total-position)as usize).min(BLOCK);
        l[..n].fill(0.);r[..n].fill(0.);
        let mut block=Vec::new();
        while next<events.len()&&events[next].0<position+n as u64{
            let (frame,on,key,velocity)=events[next];let offset=(frame-position)as u32;
            block.push(if on{note_on(key,offset,velocity)}else{note_off(key,offset)});next+=1;
        }
        handle.process_offline_events(&mut l[..n],&mut r[..n],&block).map_err(|e|e.to_string())?;
        if l[..n].iter().chain(&r[..n]).any(|v|!v.is_finite()){return Err("乐器输出含非有限 PCM".into());}
        out_l.extend_from_slice(&l[..n]);out_r.extend_from_slice(&r[..n]);position+=n as u64;
    }
    if (host_rate-rate as f64).abs()>0.5{out_l=verdant_audio::resample(&out_l,host_rate.round()as u32,rate);out_r=verdant_audio::resample(&out_r,host_rate.round()as u32,rate);}
    Ok(out_l.iter().zip(&out_r).flat_map(|(a,b)|[*a,*b]).collect())
}

/// Render an instrument track's notes through its instrument, as one stereo
/// clip from the first note. Playback and export share this path. Results
/// are cached by content, so editing one track re-renders only that track.
pub fn render_instrument(track:&verdant_domain::Track,index:usize,tempo:&TempoMap,rate:u32)->Result<Option<StereoClip>,String>{
    use verdant_plugins::builtin_dsp::{render,SynthNote,SynthParams,SYNTH_ID};
    let Some(spec)=track.instrument()else{return Ok(None)};
    let id=spec.get("id").and_then(serde_json::Value::as_str).unwrap_or_default();
    let format=spec.get("format").and_then(serde_json::Value::as_str).unwrap_or_default();
    let hosted=!(format=="builtin"&&id==SYNTH_ID);
    let host=if hosted{
        let name=spec.get("name").and_then(serde_json::Value::as_str).unwrap_or(id);
        if !matches!(format,"clap"|"vst3"|"au"){return Err(format!("轨道 {} 的乐器 {name} 格式未知",track.name));}
        Some(instrument_host().ok_or_else(||format!("轨道 {} 的乐器 {name} 需要插件宿主，但宿主不可用",track.name))?)
    }else{None};
    if index>=verdant_audio::MAX_TRACKS{return Ok(None);}
    let params=SynthParams::from_json(spec.get("params"));
    let notes:Vec<SynthNote>=track.notes.iter().filter(|n|!n.is_muted()&&n.duration_tick>0).map(|n|{
        let start=tempo.sample_at(n.start_tick,rate).max(0);let end=tempo.sample_at(n.start_tick+n.duration_tick,rate).max(start+1);
        SynthNote{start:start as u64,length:(end-start)as u64,pitch:n.midi_note as f64+n.detune_millicents as f64/100_000.0,velocity:0.8}
    }).collect();
    let Some(first)=notes.iter().map(|n|n.start).min()else{return Ok(None)};
    let shifted:Vec<SynthNote>=notes.iter().map(|n|SynthNote{start:n.start-first,..*n}).collect();
    let key={use std::hash::{Hash,Hasher};let mut h=std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(spec).unwrap_or_default().hash(&mut h);rate.hash(&mut h);
        for n in &shifted{n.start.hash(&mut h);n.length.hash(&mut h);n.pitch.to_bits().hash(&mut h);}h.finish()};
    static CACHE:std::sync::OnceLock<std::sync::Mutex<HashMap<u64,Arc<[f32]>>>>=std::sync::OnceLock::new();
    let cache=CACHE.get_or_init(Default::default);
    let cached=cache.lock().unwrap_or_else(|p|p.into_inner()).get(&key).cloned();
    let samples=match cached{Some(pcm)=>pcm,None=>{
        let pcm:Arc<[f32]>=match &host{
            Some(client)=>render_hosted(client,spec,&shifted,rate).map_err(|e|format!("轨道 {} 的乐器：{e}",track.name))?.into(),
            None=>render(&params,&shifted,rate as f64).into(),
        };
        let mut map=cache.lock().unwrap_or_else(|p|p.into_inner());if map.len()>=64{map.clear();}map.insert(key,pcm.clone());pcm
    }};
    Ok(Some(StereoClip{start:first,samples,gain:1.0,pan:0.0,track:index as u16}))
}

/// `InsertSpec.extra` key naming the track whose pre-insert, pre-fader
/// signal keys this insert.
pub const SIDECHAIN_KEY:&str="sidechain";
pub fn sidechain_source(spec:&verdant_plugins::InsertSpec)->Option<&str>{spec.extra.get(SIDECHAIN_KEY).and_then(serde_json::Value::as_str).filter(|id|!id.is_empty())}
/// Resolve a keyed insert to an engine route. `delay` is the summed latency
/// of the processors before it, so the key lines up with its audio input.
pub fn sidechain_route(project:&Project,track:usize,spec:&verdant_plugins::InsertSpec,processor:usize,delay:u32)->Option<verdant_audio::engine::SidechainRoute>{
    let source=sidechain_source(spec)?;let source=project.tracks.iter().position(|t|t.id.as_str()==source)?;
    (source!=track&&source<verdant_audio::MAX_TRACKS&&track<verdant_audio::MAX_TRACKS).then_some(verdant_audio::engine::SidechainRoute{source:source as u16,track:track as u16,processor:processor as u16,delay})
}

#[derive(Clone)]
pub struct NativeBinding{pub instance_id:String,pub processor:Option<usize>,pub bypass:bool,pub handle:Option<Arc<verdant_plugins::InsertHandle>>}
pub struct NativeBindings{pub tracks:Vec<Vec<NativeBinding>>,pub master:Vec<NativeBinding>}

fn realtime_insert_latency(binding:&NativeBinding)->Option<u32>{
    if binding.bypass||binding.processor.is_none(){return None;}
    let handle=binding.handle.as_ref()?;
    if handle.bypassed()||!matches!(handle.status(),verdant_plugins::InsertStatus::Running){return None;}
    Some(handle.latency_samples())
}

/// Finish a freshly assembled playback timeline after its loop and click have
/// been installed. Track compensation contributes the longest live track
/// chain once; enabled master inserts contribute their serial latency after
/// that. Include RT bridge latency and keep processing silence until the last
/// source/click frame can leave the master bus. Loop boundaries stay authored.
/// Offline export owns its separate DSP-only drain and must not call this.
pub fn extend_realtime_timeline(timeline:&mut Timeline,bindings:&NativeBindings)->Result<(),String>{
    let tracks=bindings.tracks.iter().map(|track|track.iter().filter_map(realtime_insert_latency).fold(0u32,u32::saturating_add));
    extend_realtime_duration(timeline,tracks,bindings.master.iter().filter_map(realtime_insert_latency))
}

fn extend_realtime_duration(timeline:&mut Timeline,track_latencies:impl IntoIterator<Item=u32>,master_latencies:impl IntoIterator<Item=u32>)->Result<(),String>{
    let mut click_end=0u64;
    if let Some(clicks)=&timeline.metronome{
        for event in &clicks.events{
            let pcm=if event.accent{&clicks.strong}else{&clicks.weak};
            if !pcm.is_empty(){click_end=click_end.max(event.sample.checked_add(pcm.len()as u64).ok_or("节拍器尾音范围溢出")?);}
        }
    }
    // A loop range, missing render or empty PCM must not become playable just
    // because an insert happens to have latency.
    if click_end==0&&!timeline.clips.iter().any(|clip|!clip.samples.is_empty())&&!timeline.stereo_clips.iter().any(|clip|clip.samples.len()>=2){return Ok(());}
    let track_latency=track_latencies.into_iter().take(verdant_audio::MAX_TRACKS).max().unwrap_or(0);
    let latency=master_latencies.into_iter().try_fold(u64::from(track_latency),|total,latency|total.checked_add(u64::from(latency)).ok_or("实时插件延迟超出范围"))?;
    timeline.length=timeline.length.max(click_end).checked_add(latency).ok_or("实时播放尾音范围溢出")?;
    Ok(())
}

fn bind_channel(channel:&verdant_audio::automation_clock::ChannelAutomation,document:&verdant_domain::automation::AutomationDocument,owner:&verdant_domain::automation::AutomationOwner,label:&str,bound:&[NativeBinding])->Result<Vec<Option<verdant_audio::automation_clock::NativePluginSchedule>>,String>{
    let size=bound.iter().filter_map(|binding|binding.processor).max().map_or(0,|index|index+1);let mut processors=vec![None;size];
    for instance in channel.plugins.iter().map(|lane|lane.instance_id.as_str()).collect::<std::collections::BTreeSet<_>>(){
        let binding=bound.iter().find(|binding|binding.instance_id==instance).ok_or_else(||format!("{label}的自动化插件尚未应用"))?;
        let handle=binding.handle.as_ref().ok_or_else(||format!("{label}的自动化插件不可用"))?;
        if !matches!(handle.status(),verdant_plugins::InsertStatus::Running){return Err(format!("{label}的自动化插件已停止运行"));}
        let schedule=plugin_schedule(channel,document,owner,instance,handle)?;
        if !binding.bypass{let processor=binding.processor.ok_or("插件没有音频处理器")?;processors[processor]=schedule;}
    }Ok(processors)
}
pub fn bind_native_automation(project:&Project,timeline:&mut Timeline,bindings:&NativeBindings)->Result<(),String>{
    use verdant_domain::automation::AutomationOwner;
    let document=verdant_domain::automation::AutomationDocument::from_project(project).map_err(|e|e.to_string())?;
    let mut native=Vec::with_capacity(project.tracks.len());
    for(index,track)in project.tracks.iter().enumerate(){
        let bound=bindings.tracks.get(index).map(Vec::as_slice).unwrap_or(&[]);
        native.push(bind_channel(&timeline.automation.tracks[index],&document,&AutomationOwner::Track{track_id:track.id.clone()},&format!("轨道 {} ",track.name),bound)?);
    }
    let master=bind_channel(&timeline.automation.master,&document,&AutomationOwner::Master,"主总线",&bindings.master)?;
    timeline.native_automation=native;timeline.master_native_automation=master;
    timeline.sidechains=realtime_sidechains(project,bindings);Ok(())
}

fn realtime_sidechains(project:&Project,bindings:&NativeBindings)->Vec<verdant_audio::engine::SidechainRoute>{
    let mut routes=Vec::new();
    for(index,track)in project.tracks.iter().enumerate(){
        let bound=bindings.tracks.get(index).map(Vec::as_slice).unwrap_or(&[]);
        for spec in verdant_plugins::project::read_inserts(&track.unknown){
            let Some(binding)=bound.iter().find(|b|b.instance_id==spec.instance_id&&!b.bypass)else{continue};let Some(processor)=binding.processor else{continue};
            let delay=bound.iter().filter(|b|b.processor.is_some_and(|p|p<processor)).filter_map(realtime_insert_latency).fold(0u32,u32::saturating_add);
            routes.extend(sidechain_route(project,index,&spec,processor,delay));
        }
    }
    routes
}

/// Prepare verified phrase PCM for playback and offline export. Apply the
/// current score's DYN envelope once on the verified render-to-score time map,
/// always from the original PCM.
pub fn prepare_phrase_pcm(
    audio: &VerifiedAudio,
    notes: &[&Note],
    tempo: &TempoMap,
    start_seconds: f64,
    phoneme_timing: Option<&PhonemeTimingTrace>,
    sample_rate: u32,
) -> Result<Arc<[f32]>, String> {
    let resampled = (audio.sample_rate != sample_rate)
        .then(|| verdant_audio::resample(&audio.samples, audio.sample_rate, sample_rate));
    let pcm = resampled.as_deref().unwrap_or(&audio.samples);
    let gain = verdant_backend::post::dyn_envelope_for_phrase(
        notes, tempo, start_seconds, sample_rate, pcm.len(), audio.time_map.as_ref(), phoneme_timing,
    )?;
    Ok(match (resampled, gain) {
        (Some(mut pcm), Some(gain)) => {
            for (sample, gain) in pcm.iter_mut().zip(gain) { *sample *= gain; }
            pcm.into()
        }
        (Some(pcm), None) => pcm.into(),
        (None, Some(mut gain)) => {
            for (gain, &sample) in gain.iter_mut().zip(audio.samples.iter()) { *gain = sample * *gain; }
            gain.into()
        }
        (None, None) => audio.samples.clone(),
    })
}

pub const MASTER_BUS_KEY: &str = "master_bus";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MasterBusSettings {
    pub gain_millidecibels: i32,
    pub pan_permille: i32,
    pub muted: bool,
}

impl MasterBusSettings {
    pub fn from_project(project: &Project) -> Self {
        project.extensions.get(MASTER_BUS_KEY)
            .and_then(|value| serde_json::from_value::<Self>(value.clone()).ok())
            .map(Self::clamped)
            .unwrap_or_default()
    }

    pub fn clamped(self) -> Self {
        Self {
            gain_millidecibels: self.gain_millidecibels.clamp(-96_000, 6_000),
            pan_permille: self.pan_permille.clamp(-1_000, 1_000),
            muted: self.muted,
        }
    }

    pub fn is_default(self) -> bool {
        self.gain_millidecibels == 0 && self.pan_permille == 0 && !self.muted
    }

    pub fn extension_value(self) -> Option<serde_json::Value> {
        let clamped = self.clamped();
        (!clamped.is_default()).then(|| serde_json::to_value(clamped).expect("master bus settings"))
    }

    pub fn channel_settings(self) -> ChannelSettings {
        let clamped = self.clamped();
        ChannelSettings {
            gain: if clamped.gain_millidecibels <= -96_000 { 0.0 } else { 10f32.powf(clamped.gain_millidecibels as f32 / 20_000.0) },
            pan: clamped.pan_permille as f32 / 1000.0,
            mute: clamped.muted,
            solo: false,
        }
    }
}

/// Mixer slots follow project order. Even an empty solo track silences other
/// tracks, without changing any persisted mute setting.
pub fn channel_settings(header: &verdant_domain::model::TrackHeader, any_solo: bool) -> ChannelSettings {
    ChannelSettings {
        gain: if header.gain_millidecibels <= -96_000 { 0.0 } else { 10f32.powf(header.gain_millidecibels as f32 / 20_000.0) },
        pan: header.pan_permille as f32 / 1000.0,
        mute: header.muted || (any_solo && !header.solo),
        solo: header.solo,
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use std::sync::atomic::Ordering;
    use verdant_audio::{ClickEvent,ClickTrack,Transport};
    use verdant_audio::engine::channel;
    use verdant_domain::{Note,Track};
    use verdant_plugins::DelayPad;

    const RATE:u32=48_000;

    #[test]
    fn prepared_realtime_playback_drains_compensated_tracks_and_serial_master_delays(){
        let mut project=Project::new_empty();
        project.tracks.push(Track::new("parallel source","blue"));
        let sources=project.tracks.iter_mut().enumerate().map(|(index,track)|{
            let note=Note::new(0,960,60);let note_id=note.id.clone();track.notes.push(note);track.refresh_index();
            let frames=if index==0{4800}else{3101};
            let samples=(0..frames).map(|frame|if frame+1==frames{0.25}else{(frame%13)as f32/128.-0.0625}).collect::<Vec<_>>();
            PhraseSource{track:track.id.clone(),start_seconds:0.,note_ids:vec![note_id],phoneme_timing:None,playback_key:None,audio:VerifiedAudio{
                path:"synthetic-preverified-drain.wav".into(),sha256:format!("owned in-memory drain PCM {index}"),sample_rate:RATE,samples:samples.into(),f0_tracks:Vec::new(),time_map:None,
            }}
        }).collect::<Vec<_>>();
        let policy=AssetPolicy::new(Vec::<std::path::PathBuf>::new()).unwrap();
        let mut prepared=prepare_timeline(&project,&sources,policy,RATE,PhrasePcmCache::default(),None).unwrap();
        assert_eq!((prepared.phrase_count,prepared.media_count),(2,0));
        assert_eq!(prepared.timeline.length,4800,"shared preparation must retain the source-only export boundary");
        let clips=prepared.timeline.clips.clone();
        let track_delays=[DelayPad::new(257),DelayPad::new(1061)];
        let track_latencies=track_delays.each_ref().map(DelayPad::latency_samples);
        let master_delays=[DelayPad::new(verdant_plugins::shm::TRANSPORT_LATENCY_FRAMES as u32+63),DelayPad::new(verdant_plugins::shm::TRANSPORT_LATENCY_FRAMES as u32)];
        let master_latencies=master_delays.each_ref().map(DelayPad::latency_samples);
        let maximum=1061;let delay=maximum as usize+master_latencies.iter().map(|latency|*latency as usize).sum::<usize>();
        let end=4800+delay;
        extend_realtime_duration(&mut prepared.timeline,track_latencies,master_latencies).unwrap();
        assert_eq!(prepared.timeline.length,end as u64);
        let(mut control,mut rt)=channel(RATE,257);
        for(index,processor)in track_delays.into_iter().enumerate(){
            assert!(control.set_chain(index,vec![Box::new(processor),Box::new(DelayPad::new(maximum-track_latencies[index]))]));
        }
        assert!(control.set_master_chain(master_delays.into_iter().map(|processor|Box::new(processor)as Box<dyn verdant_audio::Processor>).collect()));
        control.set_timeline(prepared.timeline);control.play();
        let mut actual=Vec::new();let mut block=[0.;257*2];
        for start in (0..end).step_by(257){
            let frames=257.min(end-start);rt.process(&mut block[..frames*2],2);
            actual.extend_from_slice(&block[..frames*2]);
            assert_eq!(control.position(),(start+frames)as u64);
            assert_eq!(control.transport(),if start+frames==end{Transport::Paused}else{Transport::Playing},"source EOF must not stop the prepared playback before the last delayed sample");
            control.maintain();
        }
        assert_eq!(control.shared.timeline_len.load(Ordering::Acquire),end as u64);
        for(frame,stereo)in actual.chunks_exact(2).enumerate(){
            let expected=frame.checked_sub(delay).map_or(0.,|source|clips.iter().map(|clip|clip.samples.get(source).copied().unwrap_or(0.)*clip.gain).sum::<f32>());
            assert_eq!(stereo,[expected,expected],"delayed device frame {frame}");
        }
        assert_ne!(actual[actual.len()-1],0.,"the final source sample must survive the drain");
        block.fill(1.);rt.process(&mut block,2);assert_eq!(block,[0.;257*2]);assert_eq!(control.position(),end as u64);
    }

    #[test]
    fn realtime_drain_covers_the_final_click_frame_and_preserves_loop_boundaries(){
        let mut timeline=Timeline::with_stereo(RATE,Vec::new(),vec![StereoClip{start:0,samples:vec![0.125;400].into(),gain:1.,pan:0.,track:0}]);
        timeline.loop_range=Some((100,200));
        timeline.metronome=Some(ClickTrack{events:vec![ClickEvent{sample:399,accent:true}],strong:vec![0.125;40].into(),weak:vec![0.0625;20].into(),length:400});
        timeline.length=400;
        extend_realtime_duration(&mut timeline,[17,7],[31,11]).unwrap();
        assert_eq!(timeline.length,439+17+31+11);
        assert_eq!(timeline.loop_range,Some((100,200)));
        assert_eq!(timeline.stereo_clips[0].samples.len(),400);
        assert_eq!(timeline.metronome.as_ref().unwrap().length,400);
        let(mut control,mut rt)=channel(RATE,257);control.set_timeline(timeline);control.seek(100);control.play();
        rt.process(&mut [0.;257*2],2);assert_eq!(control.position(),157);assert_eq!(control.transport(),Transport::Playing);
    }

    #[test]
    fn realtime_drain_does_not_create_material_from_empty_pcm_or_missing_bindings(){
        let mut empty=Timeline::new(RATE,Vec::new());
        extend_realtime_duration(&mut empty,[9472],[9472]).unwrap();assert_eq!(empty.length,0);
        empty.loop_range=Some((100,200));empty.length=200;
        empty.clips.push(Clip{start:100,samples:Vec::new().into(),gain:1.,track:0});
        extend_realtime_duration(&mut empty,[9472],[9472]).unwrap();assert_eq!(empty.length,200);assert_eq!(empty.loop_range,Some((100,200)));
        let unavailable=||NativeBinding{instance_id:"missing".into(),processor:Some(0),bypass:false,handle:None};
        let mut timeline=Timeline::new(RATE,vec![Clip{start:0,samples:vec![0.25;17].into(),gain:1.,track:0}]);
        extend_realtime_timeline(&mut timeline,&NativeBindings{tracks:vec![vec![unavailable()]],master:vec![unavailable()]}).unwrap();
        assert_eq!(timeline.length,17);
    }

    #[test]
    fn realtime_drain_keeps_all_32_track_slots_without_admitting_a_master_track(){
        let mut timeline=Timeline::new(RATE,vec![
            Clip{start:0,samples:vec![0.25;17].into(),gain:1.,track:31},
            Clip{start:0,samples:vec![0.25;1000].into(),gain:1.,track:32},
        ]);
        let mut latencies=[0;verdant_audio::MAX_TRACKS+1];latencies[31]=73;latencies[32]=u32::MAX;
        extend_realtime_duration(&mut timeline,latencies,[9472]).unwrap();
        assert_eq!(timeline.clips.len(),1);assert_eq!(timeline.clips[0].track,31);assert_eq!(timeline.length,17+73+9472);
    }

    #[test]
    fn realtime_drain_rejects_duration_overflow_without_publishing_a_partial_length(){
        let mut timeline=Timeline::new(RATE,vec![Clip{start:0,samples:vec![0.25].into(),gain:1.,track:0}]);timeline.length=u64::MAX;
        assert!(extend_realtime_duration(&mut timeline,[1],[9472]).is_err());assert_eq!(timeline.length,u64::MAX);
    }

    #[test]
    fn instrument_tracks_render_their_notes_into_the_shared_timeline(){
        use verdant_plugins::builtin_dsp::{SynthParams,SYNTH_ID};
        let mut project=Project::new_empty();
        let mut track=Track::new("乐器","#3584e4");
        for (start,pitch) in [(0,60),(960,64)]{track.notes.push(Note::new(start,480,pitch));}
        track.refresh_index();
        project.tracks=vec![track];
        // A vocal track with the same notes yields nothing without renders.
        let prepared=prepare_timeline(&project,&[],AssetPolicy::new(Vec::<std::path::PathBuf>::new()).unwrap(),RATE,PhrasePcmCache::default(),None).unwrap();
        assert_eq!(prepared.instrument_count,0);
        project.tracks[0].unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(),
            serde_json::json!({"format":"builtin","id":SYNTH_ID,"name":"Verdant Synth","params":SynthParams::default().to_json()}));
        let prepared=prepare_timeline(&project,&[],AssetPolicy::new(Vec::<std::path::PathBuf>::new()).unwrap(),RATE,PhrasePcmCache::default(),None).unwrap();
        assert_eq!(prepared.instrument_count,1);
        let clip=&prepared.timeline.stereo_clips[0];
        assert_eq!((clip.start,clip.track),(0,0));
        let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();
        let second=tempo.sample_at(960,RATE)as usize;
        assert!(clip.samples[..200].iter().any(|v|v.abs()>1e-4),"the first note sounds at once");
        assert!(clip.samples[second*2..second*2+400].iter().any(|v|v.abs()>1e-4),"the second note sounds at its tick");
        // An instrument this build cannot play is a located error, not silence.
        project.tracks[0].unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(),serde_json::json!({"format":"vst3","id":"x","name":"Foreign"}));
        assert!(prepare_timeline(&project,&[],AssetPolicy::new(Vec::<std::path::PathBuf>::new()).unwrap(),RATE,PhrasePcmCache::default(),None).err().unwrap().contains("Foreign"));
    }

    #[test]
    #[ignore = "requires a built verdant-plughost and the verdant-test-clap-instrument bundle"]
    fn a_hosted_clap_instrument_plays_track_notes(){
        use verdant_plugins::{PluginClient,PluginClientOpts};
        let host=std::path::PathBuf::from(std::env::var_os("VERDANT_PLUGIN_HOST").expect("VERDANT_PLUGIN_HOST"));
        let bundle=std::path::PathBuf::from(std::env::var_os("VERDANT_TEST_CLAP_INSTRUMENT").expect("VERDANT_TEST_CLAP_INSTRUMENT"));
        let cache=std::env::temp_dir().join(format!("verdant-instrument-{}",uuid::Uuid::new_v4()));
        let client=PluginClient::with_plughost(PluginClientOpts{plughost_path:Some(host.clone()),cache_dir:cache,sample_rate:48_000.,max_frames:8192},host).unwrap();
        let report=client.scan_file(&bundle).unwrap();
        assert_eq!(report.error,None);
        let desc=&report.plugins[0];
        set_instrument_host(Some(client));
        let mut project=Project::new_empty();
        let mut track=Track::new("乐器","#3584e4");
        track.notes.push(Note::new(0,480,69));track.notes.push(Note::new(960,480,81));track.refresh_index();
        track.unknown.insert(verdant_domain::model::INSTRUMENT_KEY.into(),serde_json::json!({"format":"clap","id":desc.id,"name":desc.name,"path":bundle.to_string_lossy()}));
        project.tracks=vec![track];
        let prepared=prepare_timeline(&project,&[],AssetPolicy::new(Vec::<std::path::PathBuf>::new()).unwrap(),RATE,PhrasePcmCache::default(),None).unwrap();
        set_instrument_host(None);
        assert_eq!(prepared.instrument_count,1);
        let pcm=&prepared.timeline.stereo_clips[0].samples;
        let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();
        let (first_end,second)=(tempo.sample_at(480,RATE)as usize,tempo.sample_at(960,RATE)as usize);
        // A440 for the first note: count rising zero crossings over 0.1 s.
        let left:Vec<f32>=pcm.chunks(2).map(|f|f[0]).collect();
        let crossings=left[100..4_900].windows(2).filter(|w|w[0]<0.&&w[1]>=0.).count();
        assert!((crossings as i64-44).abs()<=1,"{crossings} crossings");
        assert!(left[first_end+10..second-10].iter().all(|v|v.abs()<1e-6),"silent between the notes");
        let crossings=left[second+100..second+4_900].windows(2).filter(|w|w[0]<0.&&w[1]>=0.).count();
        assert!((crossings as i64-88).abs()<=1,"A5 for the second note: {crossings}");
    }
}
