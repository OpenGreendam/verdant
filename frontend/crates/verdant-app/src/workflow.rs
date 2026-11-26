//! Ordinary editor intents share the document reducer and durable journal.
use super::*;
use verdant_domain::editing::{self, Interpolation};
use verdant_domain::parts::Parts;
use verdant_session::{PointerId, TextEditState};

#[derive(Clone, Debug)]
pub enum WorkflowAction {
    New, Recent, Recovery, Revert, OpenRecent(PathBuf),
    RenameTrack(TrackId), MoveTrack(TrackId,isize), DuplicateTrack(TrackId),
    EditTempo(i64), DeleteTempo(i64), EditMeter(i64), DeleteMeter(i64),
    Split(i64), Join, Quantize, CloseGaps, RemoveOverlaps, MuteNotes,
    BatchLyrics(bool), ShiftLyrics(bool), ContinueSyllable,
    VibratoShape, SelectPoints, ClearPoints, TransformPoints, Interpolation(Interpolation),
    CreatePart, SelectPart(String), RenamePart, MovePart, TrimPart, SplitPart(i64), JoinPart, DissolvePart,
    DuplicatePart(bool), UnlinkPart, MutePart, DeletePart,
    GoToBar, CreatePartAt(TrackId,i64),
    RepeatSelection, ToggleLoop, LoopStart(i64), LoopEnd(i64), ToggleMetronome, CountIn(u32), ToggleExportClick,
    ExportSettings, ExportSetting(crate::export::ExportSetting), Integration(super::integrations::IntegrationAction),
}
#[derive(Clone, Debug)]
pub enum PromptPurpose {
    Integration(super::integrations::IntegrationPrompt),
    RenameTrack(TrackId),
    Lyrics(bool),
    Tempo(i64),
    Meter(i64),
    GoToBar,
    Vibrato,
    Points(TrackId),
    RenamePart(String),
    MovePart(String),
    TrimPart(String),
    NumericValue(crate::generated::controls::NumericId, crate::generated::controls::CommandArg),
}
#[derive(Clone, Debug)]
pub struct TextPrompt { pub instance:String, pub title:String, pub purpose:PromptPurpose, pub state:TextEditState, pub base:Revision }
#[derive(Clone, Debug)]
pub struct NoteCreation { pub pointer:PointerId, pub base:Revision, pub origin_tick:i64, pub note:Note }
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlaybackSettings { pub loop_start:i64, pub loop_end:i64, pub looping:bool, pub metronome:bool, pub count_in_bars:u32, pub export_click:bool }

/// Playback click boundaries share the ruler's meter map and the engine's
/// absolute sample clock. Meter changes start a new accented measure.
pub(crate) fn metronome(project:&Project,tempo:&TempoMap,rate:u32,end_tick:i64)->Result<verdant_audio::ClickTrack,String>{
    let meter=verdant_domain::MeterMap::new(&project.time_signatures,project.ppq)?;
    let length=u64::try_from(tempo.sample_at(end_tick,rate)).map_err(|_|"节拍器范围不能早于工程起点")?;
    let mut events=Vec::new();
    for(tick,bar,beat)in meter.grid(0,end_tick,project.ppq.saturating_mul(4)){
        if !beat{continue;}
        if events.len()>=4_000_000{return Err("节拍器事件超过工程范围预算".into());}
        let sample=u64::try_from(tempo.sample_at(tick,rate)).map_err(|_|"节拍器事件早于工程起点")?;
        if sample<length{events.push(verdant_audio::ClickEvent{sample,accent:bar});}
    }
    verdant_audio::ClickTrack::new(rate,events,length)
}

/// Count-in is N complete bars of the meter and tempo at the playback/record
/// entry point, even when starting in the middle of a bar. It is an overlay,
/// never a source clip, and cannot become part of an offline mix implicitly.
pub(crate) fn count_in_track(project:&Project,tempo:&TempoMap,rate:u32,tick:i64,bars:u32)->Result<verdant_audio::ClickTrack,String>{
    if bars>8{return Err("预备拍范围为 0–8 小节".into());}
    let meter=verdant_domain::MeterMap::new(&project.time_signatures,project.ppq)?;
    let signature=meter.signature_at(tick);
    let beats=u64::from(signature.numerator)*u64::from(bars);
    let frames_per_beat=60.*f64::from(rate)/tempo.bpm_at(tick)*4./f64::from(signature.denominator);
    let frames=frames_per_beat*beats as f64;
    if !frames.is_finite()||frames>f64::from(rate)*120.{return Err("预备拍超过 120 秒；请减少小节数或提高速度".into());}
    let length=frames.round()as u64;
    let events=(0..beats).map(|beat|verdant_audio::ClickEvent{sample:(beat as f64*frames_per_beat).round()as u64,accent:beat%u64::from(signature.numerator)==0}).collect();
    verdant_audio::ClickTrack::new(rate,events,length)
}

#[cfg(test)]
#[path="loop_click_tests.rs"]
mod loop_click_tests;

/// Apply at the offline export boundary, independently of the live click
/// switch. Count-in remains a transport overlay and is never exported.
pub(crate) fn export_click(timeline:&mut Timeline,project:&Project,tempo:&TempoMap,rate:u32,end_tick:i64)->Result<(),String>{
    let settings:PlaybackSettings=project.extensions.get("transport").map(|value|serde_json::from_value(value.clone())).transpose().map_err(|e|format!("播放设置无效：{e}"))?.unwrap_or_default();
    timeline.metronome=if settings.export_click{Some(metronome(project,tempo,rate,end_tick)?)}else{None};
    Ok(())
}

impl Editor {
    pub fn text_state(&self)->Option<&TextEditState>{self.text_prompt.as_ref().map(|p|&p.state).or_else(||self.lyric.as_ref().map(|l|&l.state))}
    pub fn text_state_mut(&mut self)->Option<&mut TextEditState>{if let Some(p)=&mut self.text_prompt{Some(&mut p.state)}else{self.lyric.as_mut().map(|l|&mut l.state)}}
    pub fn prompt_text(&mut self,title:impl Into<String>,purpose:PromptPurpose,current:&str,window:&mut Window,cx:&mut Context<Self>){
        self.end_lyric(true,None,window,cx);if self.lyric.is_some(){return;}self.cancel_edits(verdant_session::CancelReason::Escape,window,cx);
        self.text_prompt=Some(TextPrompt{instance:NoteId::generate().to_string(),title:title.into(),purpose,state:TextEditState::new(current),base:self.revision()});window.focus(&self.focus_lyric,cx);cx.notify();
    }
    pub fn prompt_numeric_value(
        &mut self,
        id: crate::generated::controls::NumericId,
        arg: crate::generated::controls::CommandArg,
        current: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let spec = crate::generated::controls::numeric(id);
        let label = crate::ui_controls::numeric_label_zh(id);
        self.prompt_text(
            format!("输入 {label} ({} .. {} {})", spec.min, spec.max, spec.unit),
            PromptPurpose::NumericValue(id, arg),
            &format!("{:.*}", spec.decimals, current),
            window,
            cx,
        );
    }
    pub fn finish_prompt(&mut self,confirm:bool,window:&mut Window,cx:&mut Context<Self>){
        let Some(prompt)=self.text_prompt.take()else{return;};
        if !confirm{window.focus(&self.focus_roll,cx);cx.notify();return;}
        if prompt.state.is_composing(){self.text_prompt=Some(prompt);return;}
        // The singer prompt supplies the frozen export, with live document
        // side effects separately checked after the file is written.
        let frozen_export=matches!(&prompt.purpose,PromptPurpose::Integration(super::integrations::IntegrationPrompt::VsqxExportSinger(_)));
        if prompt.base!=self.revision()&&!frozen_export{self.notice("工程已改变，请重新打开编辑窗口");window.focus(&self.focus_roll,cx);cx.notify();return;}
        let result=self.apply_prompt(&prompt,cx);
        if let Err(error)=result{self.notice(error);self.text_prompt=Some(prompt);}else{window.focus(&self.focus_roll,cx);}cx.notify();
    }
    fn apply_prompt(&mut self,prompt:&TextPrompt,cx:&mut Context<Self>)->Result<(),String>{
        let text=prompt.state.text.trim();
        match &prompt.purpose{
            PromptPurpose::Integration(purpose)=>self.apply_integration_prompt(purpose,text,cx)?,
            PromptPurpose::RenameTrack(id)=>{if text.is_empty(){return Err("轨道名称不能为空".into());}self.edit_header(id,"重命名轨道",cx,|h|h.name=text.into());}
            PromptPurpose::Lyrics(character)=>self.paste_lyrics(text,*character,cx)?,
            PromptPurpose::Tempo(tick)=>{let bpm=parse_values(text,1)?[0];if !(1.0..=1000.0).contains(&bpm){return Err("速度范围为 1–1000 BPM".into());}self.commit(self.revision(),CommandBatch::single("编辑速度事件",Command::SetTempoEvent{tick:*tick,micro_bpm:Some((bpm*1e6).round()as u64)}),cx);}
            PromptPurpose::Meter(tick)=>{let values=parse_values(&text.replace('/',","),2)?;if values.iter().any(|v|v.fract()!=0.||*v<1.||*v>64.){return Err("拍号需要 1–64 的整数".into());}let numerator=values[0]as u32;let denominator=values[1]as u32;let mut signatures=self.project().time_signatures.clone();if let Some(signature)=signatures.iter_mut().find(|s|s.tick==*tick){signature.numerator=numerator;signature.denominator=denominator;}else{signatures.push(verdant_domain::model::TimeSignature{tick:*tick,numerator,denominator,unknown:Default::default()});signatures.sort_by_key(|s|s.tick);}verdant_domain::MeterMap::new(&signatures,self.project().ppq)?;if !self.commit(self.revision(),CommandBatch::single("编辑拍号事件",Command::SetTimeSignatureEvent{tick:*tick,numerator:Some(numerator),denominator}),cx){return Err("拍号未提交".into());}}
            PromptPurpose::Vibrato=>{let v=parse_values(text,3)?;if !(0.0..=100.0).contains(&v[0])||!(0.0..=100.0).contains(&v[1]){return Err("淡入淡出范围为 0–100%".into());}self.adjust_vibrato(|shape|shape.set_fades_phase((v[0]*10.).round()as u32,(v[1]*10.).round()as u32,(v[2]*1000.).round()as i64),cx);}
            PromptPurpose::GoToBar=>{let target = crate::clock::bar_target_tick(&self.meter(), text)?; self.seek_tick(target as f64);}
            PromptPurpose::Points(track)=>{
                if *track!=self.session.track{return Err("轨道已切换，请重新打开控制点变换".into());}
                let fields=exact_numeric_fields(text,4)?;
                let offset=fields[0].parse::<i64>().map_err(|_|"平移 tick 须为 i64 范围内的整数")?;
                let scales=parse_values(&fields[1..].join(","),3)?;
                self.transform_selected_points(offset,scales[0]/100.,scales[1],scales[2]/100.,cx)?;
            }
            PromptPurpose::RenamePart(id)=>{if text.is_empty(){return Err("音符组名称不能为空".into());}self.edit_part(id,"音符组名称",cx,|p|{p.name=text.into();Ok(())})?;}
            PromptPurpose::MovePart(id)=>{let v=parse_values(text,1)?;self.edit_part(id,"移动音符组",cx,|p|{p.start_tick=v[0].round()as i64;Ok(())})?;}
            PromptPurpose::TrimPart(id)=>{let v=parse_values(text,2)?;self.edit_part(id,"裁剪音符组",cx,|p|{p.source_offset_tick=v[0].round()as i64;p.duration_tick=v[1].round()as i64;Ok(())})?;}
            PromptPurpose::NumericValue(id, arg) => {
                use crate::generated::controls::{NumericPhase, NumericTarget};
                let v = parse_values(text, 1)?[0];
                let spec = crate::generated::controls::numeric(*id);
                if !(spec.min..=spec.max).contains(&v) {
                    return Err(format!("数值须在 {} 到 {} {} 之间", spec.min, spec.max, spec.unit));
                }
                self.set_numeric(*id, arg.clone(), v, NumericPhase::Preview, cx);
                self.set_numeric(*id, arg.clone(), v, NumericPhase::Commit, cx);
            }
        }Ok(())
    }
    pub fn bar_nudge_delta(&self,forward:bool)->i64{let tick=self.selected_notes().first().map(|n|n.start_tick).unwrap_or(0);let meter=self.meter();let start=meter.bar_start(tick);if forward{meter.next_bar(start)-start}else{meter.previous_bar(start)-start}}
    pub fn tempo_event_tick(&self)->i64{let tick=self.tempo().tick_at(self.playhead_seconds()).round()as i64;self.project().tempo_map.iter().rev().find(|p|p.tick<=tick).map(|p|p.tick).unwrap_or(0)}
    pub fn playback_settings(&self)->PlaybackSettings{self.project().extensions.get("transport").and_then(|v|serde_json::from_value(v.clone()).ok()).unwrap_or_default()}
    pub fn edit_playback(&mut self,label:&str,cx:&mut Context<Self>,f:impl FnOnce(&mut PlaybackSettings)){
        let mut settings=self.playback_settings();f(&mut settings);
        if settings.loop_start<0||settings.loop_end<0||(settings.looping&&settings.loop_end<=settings.loop_start)||settings.count_in_bars>8{self.notice("循环范围须为非负且终点晚于起点；预备拍为 0–8 小节");cx.notify();return;}
        self.commit(self.revision(),CommandBatch::single(label,Command::SetProjectExtension{key:"transport".into(),value:Some(serde_json::to_value(settings).expect("transport settings"))}),cx);
    }
    pub fn run_workflow(&mut self,action:WorkflowAction,window:&mut Window,cx:&mut Context<Self>){
        use WorkflowAction as A;
        let result:Result<(),String>=(||{
            match action{
                A::New=>self.new_document(window,cx)?,A::Recent=>self.recent_menu(false,window,cx),A::Recovery=>self.recent_menu(true,window,cx),A::Revert=>self.revert_document(window,cx)?,A::OpenRecent(path)=>self.open_project_path(path,window,cx),
                A::RenameTrack(id)=>{let name=self.project().track(&id).ok_or("轨道已不存在")?.name.clone();self.prompt_text("轨道名称",PromptPurpose::RenameTrack(id),&name,window,cx);}
                A::MoveTrack(id,direction)=>{let index=self.track_index(&id).ok_or("轨道已不存在")?;let destination=(index as isize+direction).clamp(0,self.project().tracks.len()as isize-1)as usize;self.commit(self.revision(),CommandBatch::single("调整轨道顺序",Command::MoveTrack{track:id,index:destination}),cx);}
                A::DuplicateTrack(id)=>self.duplicate_track(&id,cx)?,
                A::EditTempo(tick)=>{let bpm=self.tempo().bpm_at(tick);self.prompt_text(format!("tick {tick} 的速度（BPM）"),PromptPurpose::Tempo(tick),&format!("{bpm}"),window,cx);}
                A::DeleteTempo(tick)=>{self.commit(self.revision(),CommandBatch::single("删除速度事件",Command::SetTempoEvent{tick,micro_bpm:None}),cx);}
                A::EditMeter(tick)=>{let m=self.meter();let signature=m.signature_at(tick);self.prompt_text(format!("tick {tick} 的拍号（例如 3/4）"),PromptPurpose::Meter(tick),&format!("{}/{}",signature.numerator,signature.denominator),window,cx);}
                A::DeleteMeter(tick)=>{self.commit(self.revision(),CommandBatch::single("删除拍号事件",Command::SetTimeSignatureEvent{tick,numerator:None,denominator:4}),cx);}
                A::Split(tick)=>{let(batch,ids)=editing::split(self.project(),&self.session.track,&self.session.selection_in_order(self.project()),tick)?;if self.commit(self.revision(),batch,cx){self.session.selection=ids.into_iter().collect();}}
                A::Join=>{let(batch,id)=editing::join(self.project(),&self.session.track,&self.session.selection_in_order(self.project()))?;if self.commit(self.revision(),batch,cx){self.session.selection=[id].into();}}
                A::Quantize=>{let batch=editing::quantize(self.project(),&self.session.track,&self.session.selection_in_order(self.project()),self.session.snap.grid_ticks)?;self.commit(self.revision(),batch,cx);}
                A::CloseGaps|A::RemoveOverlaps=>{let batch=editing::cleanup(self.project(),&self.session.track,&self.session.selection_in_order(self.project()),self.session.snap.grid_ticks,matches!(action,A::RemoveOverlaps))?;self.commit(self.revision(),batch,cx);}
                A::MuteNotes=>{let mut notes=self.selected_notes();let muted=notes.iter().any(|n|!n.is_muted());for n in &mut notes{n.set_muted(muted);}self.replace_selected_notes(if muted{"静音音符"}else{"取消音符静音"},notes,cx);}
                A::BatchLyrics(character)=>self.prompt_text(if character{"逐字填入歌词；空格和标点分隔，读音注释保留"}else{"歌词按空格和标点分配；用 + 延续前一音节"},PromptPurpose::Lyrics(character),"",window,cx),
                A::ShiftLyrics(right)=>self.shift_lyrics(right,cx)?,A::ContinueSyllable=>self.continue_syllable(cx)?,
                A::VibratoShape=>{self.open_vibrato_dialog(window,cx);}
                A::SelectPoints=>{self.point_selection=self.selected_notes().iter().flat_map(|n|self.param.point_samples(n).into_iter().map(|(id,_,_)|(n.id.clone(),id))).collect();cx.notify();}
                A::ClearPoints=>{self.point_selection.clear();cx.notify();}
                A::TransformPoints=>self.open_transform_points_dialog(window,cx),
                A::Interpolation(mode)=>{let mut notes=self.selected_notes();for n in &mut notes{n.curves.set_interpolation(self.param.short(),mode);}self.replace_selected_notes("曲线插值",notes,cx);}
                A::CreatePartAt(track,tick)=>crate::arrange::create_part_at(self,&track,tick as f64,window,cx),
                A::GoToBar=>self.prompt_text("跳转到小节（从 1 开始）",PromptPurpose::GoToBar,"",window,cx),
                A::CreatePart=>{let mut parts=Parts::load(self.project())?;let name=parts.next_name("音符组");let id=parts.create(self.project(),&self.session.track,&self.session.selection_in_order(self.project()),name)?;if self.commit(self.revision(),CommandBatch::single("创建声部音符组",parts.command()),cx){self.selected_part=Some(id);}}
                A::SelectPart(id)=>{let parts=Parts::load(self.project())?;let p=parts.instance(&id).ok_or("音符组已不存在")?;if !self.select_track(p.track.clone(),cx){return Err("先完成当前歌词编辑".into());}self.session.selection=p.bindings.values().filter(|id|self.project().track(&p.track).is_some_and(|t|t.note(id).is_some())).cloned().collect();self.selected_part=Some(id);self.sync_index();cx.notify();}
                A::RenamePart|A::MovePart|A::TrimPart=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;let p=Parts::load(self.project())?.instance(&id).cloned().ok_or("音符组已不存在")?;let(title,purpose,current)=match action{A::RenamePart=>("音符组名称",PromptPurpose::RenamePart(id),p.name),A::MovePart=>("音符组开始 tick",PromptPurpose::MovePart(id),p.start_tick.to_string()),_=>("源偏移 tick、音符组长度 tick",PromptPurpose::TrimPart(id),format!("{}, {}",p.source_offset_tick,p.duration_tick))};self.prompt_text(title,purpose,&current,window,cx);}
                A::DuplicatePart(linked)=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;let mut parts=Parts::load(self.project())?;let p=parts.instance(&id).ok_or("音符组已不存在")?;let at=p.start_tick+p.duration_tick;let id=parts.duplicate(&id,at,linked)?;if self.commit(self.revision(),CommandBatch::single(if linked{"链接重复音符组"}else{"独立复制音符组"},parts.command()),cx){self.selected_part=Some(id);}}
                A::UnlinkPart=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;let mut parts=Parts::load(self.project())?;parts.unlink(&id)?;self.commit(self.revision(),CommandBatch::single("取消音符组链接",parts.command()),cx);}
                A::SplitPart(tick)=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;let mut parts=Parts::load(self.project())?;parts.split(&id,tick)?;self.commit(self.revision(),CommandBatch::single("分割音符组",parts.command()),cx);}
                A::JoinPart=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;let mut parts=Parts::load(self.project())?;let p=parts.instance(&id).ok_or("音符组已不存在")?;let next=parts.instances.iter().find(|n|n.id!=id&&n.track==p.track&&n.start_tick==p.start_tick+p.duration_tick).map(|p|p.id.clone()).ok_or("右侧没有相邻音符组")?;parts.join(&id,&next)?;self.commit(self.revision(),CommandBatch::single("连接音符组",parts.command()),cx);}
                A::MutePart=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;self.edit_part(&id,"音符组静音",cx,|p|{p.muted=!p.muted;Ok(())})?;}
                A::DissolvePart => {
                    let id = self.selected_part.as_deref().ok_or("先选择音符组")?;
                    let mut parts = Parts::load(self.project())?;
                    parts.instance(id).filter(|part| part.track == self.session.track).ok_or("先选择当前轨道的音符组")?;
                    let (track, notes) = parts.dissolve(self.project(), id)?;
                    let commands = vec![parts.command(), Command::InsertNotes { track, notes }];
                    if self.commit(self.revision(), CommandBatch { label: "解散音符组".into(), commands }, cx) {
                        self.selected_part = None;
                    }
                }
                                A::DeletePart=>{let id=self.selected_part.clone().ok_or("先选择音符组")?;let mut parts=Parts::load(self.project())?;parts.instances.retain(|p|p.id!=id);self.commit(self.revision(),CommandBatch::single("删除音符组",parts.command()),cx);self.selected_part=None;}
                A::RepeatSelection=>{let notes=self.selected_notes();let start=notes.iter().map(|n|n.start_tick).min().ok_or("先选择音符")?;let end=notes.iter().map(|n|n.end_tick()).max().expect("selection");self.edit_playback("循环选择范围",cx,|s|{s.loop_start=start;s.loop_end=end;s.looping=true;});}
                A::ToggleLoop=>{let anchor=self.playback_settings().loop_start.max(0);let length=self.meter().bar_ticks_at(anchor);self.edit_playback("循环播放",cx,|s|{if s.loop_end<=s.loop_start{s.loop_end=s.loop_start+length;}s.looping=!s.looping;});}
                A::LoopStart(tick)=>self.edit_playback("循环起点",cx,|s|{s.loop_start=tick;if s.loop_end<=tick{s.loop_end=tick+1;}}),A::LoopEnd(tick)=>self.edit_playback("循环终点",cx,|s|s.loop_end=tick),
                A::ToggleMetronome=>self.edit_playback("节拍器",cx,|s|s.metronome=!s.metronome),A::CountIn(bars)=>self.edit_playback("预备拍",cx,|s|s.count_in_bars=bars),
                A::ToggleExportClick=>self.edit_playback("导出节拍器",cx,|s|s.export_click=!s.export_click),
                A::ExportSettings=>self.export_settings_menu(window,cx),
                A::ExportSetting(setting)=>self.apply_export_setting(setting,window,cx),
                A::Integration(action)=>self.run_integration(action,window,cx)?,
            }Ok(())
        })();if let Err(error)=result{self.notice(error);cx.notify();}
    }
    fn replace_selected_notes(&mut self,label:&str,notes:Vec<Note>,cx:&mut Context<Self>){if notes.is_empty(){self.notice("先选择音符");cx.notify();return;}self.commit(self.revision(),CommandBatch::single(label,Command::ReplaceNotes{track:self.session.track.clone(),notes}),cx);}
    pub(crate) fn edit_part(&mut self,id:&str,label:&str,cx:&mut Context<Self>,f:impl FnOnce(&mut verdant_domain::parts::PartInstance)->Result<(),String>)->Result<(),String>{let mut parts=Parts::load(self.project())?;f(parts.instances.iter_mut().find(|p|p.id==id).ok_or("音符组已不存在")?)?;parts.validate(self.project())?;if self.commit(self.revision(),CommandBatch::single(label,parts.command()),cx){Ok(())}else{Err("音符组修改未提交".into())}}
    fn duplicate_track(&mut self,id:&TrackId,cx:&mut Context<Self>)->Result<(),String>{
        if self.project().tracks.len()>=verdant_audio::MAX_TRACKS{return Err("已达到轨道数上限".into());}let index=self.track_index(id).ok_or("轨道已不存在")?;
        let mut track=self.project().track(id).cloned().ok_or("轨道已不存在")?;let new_id=TrackId::generate();track.id=new_id.clone();track.name=format!("{} 副本",track.name);track.solo=false;
        let mut mapping=HashMap::new();for n in &mut track.notes{let old=n.id.clone();n.regenerate_ids();mapping.insert(old,n.id.clone());}track.refresh_index();
        let mut plugin_ids=HashMap::new();
        if let Some(inserts)=track.unknown.get_mut("inserts").and_then(|v|v.as_array_mut()){for insert in inserts{if let Some(map)=insert.as_object_mut(){let old=map.get("instance_id").and_then(|v|v.as_str()).unwrap_or_default().to_string();let new=NoteId::generate().to_string();if !old.is_empty(){plugin_ids.insert(old,new.clone());}map.insert("instance_id".into(),new.into());}}}
        let clips=verdant_media::read_clips(self.project().track(id).ok_or("轨道不存在")?).map_err(|e|e.to_string())?.into_iter().map(|old|{let mut clip=old.duplicate_at(old.tick()).map_err(|e|e.to_string())?;clip.track=new_id.clone();Ok(clip)}).collect::<Result<Vec<_>,String>>()?;
        if let Some(value)=verdant_media::clips_value(new_id.clone(),&clips).map_err(|e|e.to_string())?{track.unknown.insert(verdant_media::AUDIO_CLIPS_KEY.into(),value);}
        let mut automation=verdant_domain::automation::AutomationDocument::from_project(self.project()).map_err(|e|e.to_string())?;
        let copied=automation.lanes.iter().filter_map(|lane|{use verdant_domain::automation::{AutomationTarget as T,AutomationOwner as O};let mut copy=lane.clone();copy.target=match &lane.target{T::TrackGain{track_id}if track_id==id=>T::TrackGain{track_id:new_id.clone()},T::TrackPan{track_id}if track_id==id=>T::TrackPan{track_id:new_id.clone()},T::PluginParameter{owner:O::Track{track_id},instance_id,parameter_id}if track_id==id=>T::PluginParameter{owner:O::Track{track_id:new_id.clone()},instance_id:plugin_ids.get(instance_id)?.clone(),parameter_id:parameter_id.clone()},_=>return None};Some(copy)}).collect::<Vec<_>>();
        for lane in copied{automation.set_lane(lane);}
        let mut parts=Parts::load(self.project())?;let original=parts.instances.iter().filter(|p|&p.track==id).cloned().collect::<Vec<_>>();
        for mut part in original{let mut content=parts.contents.get(&part.content_id).cloned().ok_or("音符组内容不存在")?;content.id=NoteId::generate().to_string();part.id=NoteId::generate().to_string();part.content_id=content.id.clone();part.track=new_id.clone();for note in part.bindings.values_mut(){*note=mapping.get(note).cloned().unwrap_or_else(NoteId::generate);}parts.contents.insert(content.id.clone(),content);parts.instances.push(part);}
        let mut commands=vec![Command::AddTrack{index:index+1,track},Command::SetProjectExtension{key:verdant_domain::automation::AUTOMATION_KEY.into(),value:automation.extension_value().map_err(|e|e.to_string())?}];if !parts.instances.is_empty(){commands.push(parts.command());}
        if self.commit(self.revision(),CommandBatch{label:"复制轨道".into(),commands},cx){self.select_track(new_id,cx);}Ok(())
    }
    pub fn lyric_commands(&self,track:&TrackId,note:&Note,text:&str)->Result<Vec<Command>,String>{
        let proxies=self.scheduler.capabilities.as_ref().and_then(|c|c.native_score.as_ref()).map(|n|n.english_vowel_proxies.as_slice()).unwrap_or_default();
        let changed=lyric_note(note,text,proxies)?;
        Ok(vec![Command::ReplaceNotes{track:track.clone(),notes:vec![changed]}])
    }
    fn paste_lyrics(&mut self,text:&str,character:bool,cx:&mut Context<Self>)->Result<(),String>{
        let tokens=batch_lyric_tokens(text,character)?;
        let track=self.active_track().ok_or("轨道不存在")?;
        let proxies=self.scheduler.capabilities.as_ref().and_then(|c|c.native_score.as_ref()).map(|n|n.english_vowel_proxies.as_slice()).unwrap_or_default();
        let notes=distribute_lyrics(&self.selected_notes(),&track.notes,&tokens,proxies)?;
        if !self.commit(self.revision(),CommandBatch::single("批量填写歌词",Command::ReplaceNotes{track:self.session.track.clone(),notes}),cx){return Err("歌词修改未提交".into());}Ok(())
    }
    fn shift_lyrics(&mut self,right:bool,cx:&mut Context<Self>)->Result<(),String>{
        let notes=shifted_lyrics(&self.selected_notes(),&self.active_track().ok_or("轨道不存在")?.notes,right)?;
        if !self.commit(self.revision(),CommandBatch::single("循环移位歌词",Command::ReplaceNotes{track:self.session.track.clone(),notes}),cx){return Err("歌词修改未提交".into());}Ok(())
    }
    fn continue_syllable(&mut self,cx:&mut Context<Self>)->Result<(),String>{
        let notes=continued_syllable(&self.selected_notes(),&self.active_track().ok_or("轨道不存在")?.notes)?;
        if !self.commit(self.revision(),CommandBatch::single("跨音高延续音节",Command::ReplaceNotes{track:self.session.track.clone(),notes}),cx){return Err("歌词修改未提交".into());}Ok(())
    }
    pub(crate) fn transform_selected_points(&mut self,offset:i64,time_scale:f64,value_offset:f64,value_scale:f64,cx:&mut Context<Self>)->Result<(),String>{
        if !time_scale.is_finite()||time_scale<=0.||!value_scale.is_finite(){return Err("时间缩放须为正数，数值缩放须有限".into());}
        let selected=&self.point_selection;let track=self.active_track().ok_or("轨道不存在")?;let origin=track.notes.iter().flat_map(|n|self.param.point_samples(n).into_iter().filter(|(id,_,_)|selected.contains(&(n.id.clone(),id.clone()))).map(|(_,tick,_)|n.start_tick+tick)).min().ok_or("先选择曲线控制点")?;
        if track.notes.iter().any(|source|self.region_lane_disabled(source,self.param)&&self.param.point_samples(source).into_iter().any(|(id,_,_)|selected.contains(&(source.id.clone(),id)))){return Err("先启用此区域曲线，再变换控制点；所有点保持未修改".into());}
        let mut notes=Vec::new();for source in &track.notes{let mut note=source.clone();let mut changed=false;
            macro_rules! transform{($points:expr,$field:ident,$units:expr,$min:expr,$max:expr)=>{for p in $points{if !selected.contains(&(note.id.clone(),p.id.clone())){continue;}let tick=origin as f64+(source.start_tick+p.tick-origin)as f64*time_scale+offset as f64-source.start_tick as f64;let value=p.$field as f64/$units*value_scale+value_offset;if tick<0.||tick>source.duration_tick as f64||value<$min||value>$max{return Err("变换超出音符边界或参数范围；所有点保持未修改".into());}p.tick=tick.round()as i64;p.$field=(value*$units).round()as _;changed=true;}}}
            match self.param{Param::Pit=>transform!(&mut note.curves.pitch_millicent_points,millicents,1000.,-2400.,2400.),Param::Dyn=>transform!(&mut note.curves.gain_millidecibel_points,millidecibels,1000.,-60.,12.),p if p.is_expression()=>{if let Some(points)=note.curves.expression_points.get_mut(p.short()){transform!(points,value,1.,0.,127.);}},_=>return Err("此参数不是控制点曲线".into())}
            if changed{let points=self.param.point_samples(&note);let mut ticks=std::collections::BTreeSet::new();if points.iter().any(|(_,tick,_)|!ticks.insert(tick)){return Err("变换会合并控制点，未提交".into());}note.curves.pitch_millicent_points.sort_by_key(|p|p.tick);note.curves.gain_millidecibel_points.sort_by_key(|p|p.tick);for points in note.curves.expression_points.values_mut(){points.sort_by_key(|p|p.tick);}notes.push(note);}}
        self.replace_selected_notes("变换曲线点集",notes,cx);Ok(())
    }
    pub fn remember_project(&mut self,path:&Path){crate::menu::remember_recent_file(&mut self.prefs,path);self.prefs_dirty=true;}
    fn recent_menu(&mut self,recovery:bool,window:&mut Window,cx:&mut Context<Self>){let mut paths=crate::menu::recent_project_paths(&self.prefs);if recovery{if let Ok(entries)=std::fs::read_dir(crate::files::state_dir().join("documents")){for entry in entries.flatten(){let path=entry.path();if crate::files::is_native_project_path(&path){paths.push(path);}}}paths.retain(|p|verdant_domain::persistence::journal_path(p).is_file());}let mut unique=std::collections::HashSet::new();paths.retain(|p|unique.insert(p.clone()));let mut items=paths.into_iter().map(|p|crate::menu::MenuItem::action(p.display().to_string(),MenuAction::Workflow(WorkflowAction::OpenRecent(p)))).collect::<Vec<_>>();if items.is_empty(){items.push(crate::menu::MenuItem::disabled(if recovery{"没有可恢复的工程"}else{"尚无最近工程"}));}self.show_menu(window.mouse_position(),items,window,cx);}
    fn new_document(&mut self,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{if !self.commit_pending_lyric(cx)||!self.flush_insert_states(cx){return Err("当前编辑尚未提交".into());}self.persistence.autosave().map_err(|e|e.to_string())?;let mut project=super::voice_ui::new_project();crate::files::stamp_creator(&mut project);let path=crate::files::state_dir().join("documents").join(format!("untitled-{}.verp",project.project_id));let persistence=ProjectPersistence::create(&path,project).map_err(|e|e.to_string())?;self.replace_project_opened(persistence,Default::default(),window,cx);self.project_path=None;self.remember_project(&path);Ok(())}
    fn revert_document(&mut self,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        if !self.commit_pending_lyric(cx)||!self.flush_insert_states(cx){return Err("当前编辑尚未提交".into());}
        let source=self.project_path.clone().ok_or("未命名工程尚无手动保存版本")?;
        let current_path=self.persistence.project_path().to_path_buf();let project_id=self.project().project_id.clone();let revision=self.revision();
        let archive=crate::files::state_dir().join("documents").join(format!("before-revert-{}.verp",NoteId::generate()));
        let archive_assets=verdant_service::media::ProjectAssetCopy::new(self.store(),&current_path,&archive).map_err(|error|error.to_string())?;
        let working=crate::files::state_dir().join("documents").join(format!("reverted-{}.verp",NoteId::generate()));
        let source_for_job=source.clone();let working_for_job=working.clone();let executor=cx.background_executor().clone();
        self.integrations.media_file_jobs+=1;
        self.notice("正在核对还原版本与历史副本的音频素材…");
        cx.spawn_in(window,async move|this,cx|{
            let prepared=executor.spawn(async move{
                archive_assets.copy().map_err(|error|error.to_string())?;
                let project=ProjectPersistence::read_saved(&source_for_job).map_err(|error|error.to_string())?;
                let store=verdant_domain::ProjectStore::new(project);
                verdant_service::media::copy_project_assets(&store,&source_for_job,&working_for_job).map_err(|error|error.to_string())?;
                ProjectPersistence::create_from_store(&working_for_job,store).map_err(|error|error.to_string())
            }).await;
            let _=cx.update(|window,cx|this.update(cx,|ed,cx|{
                ed.integrations.media_file_jobs=ed.integrations.media_file_jobs.saturating_sub(1);
                if ed.persistence.project_path()!=current_path.as_path()||ed.project().project_id!=project_id||ed.revision()!=revision{
                    if prepared.is_ok(){ed.notice(format!("工程已改变，未应用还原；已准备的还原副本保留于 {}",working.display()));}
                    else{ed.notice("工程已改变，本次还原未完成；当前工程保持未替换");}cx.notify();return;
                }
                let result=(||->Result<(),String>{
                    let persistence=prepared?;
                    ed.persistence.fork_with_history(&archive).map_err(|error|error.to_string())?;
                    ed.replace_project_opened(persistence,Default::default(),window,cx);ed.project_path=Some(source);ed.remember_project(&archive);
                    ed.notice(format!("已还原手动保存版本；还原前内容及历史保留于 {}",archive.display()));Ok(())
                })();
                if let Err(error)=result{ed.notice(format!("还原失败：{error}"));}cx.notify();
            }));
        }).detach();
        Ok(())
    }
}
pub(super) fn parse_values(text:&str,count:usize)->Result<Vec<f64>,String>{let values=text.split(|c:char|c==','||c=='，'||c.is_whitespace()).filter(|s|!s.is_empty()).map(|s|s.parse::<f64>().map_err(|_|format!("无法读取数值 {s}"))).collect::<Result<Vec<_>,_>>()?;if values.len()!=count||values.iter().any(|v|!v.is_finite()){return Err(format!("需要 {count} 个有限数值"));}Ok(values)}

/// Integer document coordinates must bypass f64 to retain every tick and frame.
pub(super) fn exact_numeric_fields(text:&str,count:usize)->Result<Vec<&str>,String>{
    let fields=text.split(|c:char|c==','||c=='，'||c.is_whitespace()).filter(|s|!s.is_empty()).collect::<Vec<_>>();
    if fields.len()!=count{return Err(format!("需要 {count} 个数值"));}
    Ok(fields)
}

/// Sentence punctuation separates notes; phonetic annotations retain every symbol.
/// Apostrophes, hyphens and the colon in pinyin `nu:3` remain part of a word.
pub(super) fn lyric_tokens(text:&str)->Result<Vec<String>,String>{
    let mut tokens=Vec::new();let mut token=String::new();let mut annotation=false;
    for character in text.chars(){match character{'['=>{if annotation{return Err("读音方括号不能嵌套".into());}annotation=true;token.push(character);},']'=>{if !annotation{return Err("读音方括号不成对".into());}annotation=false;token.push(character);},c if !annotation&&(c.is_whitespace()||"，。！？、；：,.!?;…—「」『』“”\"（）()《》〈〉".contains(c)||(c==':'&&!pinyin_colon(&token)))=>{if !token.is_empty(){tokens.push(std::mem::take(&mut token));}},c=>token.push(c)}}
    if annotation{return Err("读音方括号未闭合".into());}if !token.is_empty(){tokens.push(token);}Ok(tokens)
}

fn pinyin_colon(token:&str)->bool{
    let suffix=token.rsplit('/').next().unwrap_or_default();
    ["u","nu","lu","ju","qu","xu"].iter().any(|prefix|suffix.eq_ignore_ascii_case(prefix))
}

fn batch_lyric_tokens(text:&str,character:bool)->Result<Vec<String>,String>{
    let tokens=lyric_tokens(text)?;
    if !character{return Ok(tokens);}
    use unicode_segmentation::UnicodeSegmentation;
    let mut characters=Vec::new();
    for token in tokens{
        // An authored reading belongs to its whole lyric, even in character mode.
        if token.contains('/') {characters.push(token);}
        else {characters.extend(token.graphemes(true).map(str::to_string));}
    }
    Ok(characters)
}

/// Shared by inline and batch edits: unchanged spellings retain authored
/// readings, and explicit phonetic annotations retain timing/provenance.
fn lyric_note(note:&Note,text:&str,english_vowel_proxies:&[String])->Result<Note,String>{
    let phonetic=verdant_backend::reading::parse_phonetic_input(text)?;
    let (lyric, pinyin) = if let Some((lyric, _)) = &phonetic {
        (lyric.clone(), None)
    } else if text.contains('/') {
        parse_lyric_input(text)
    } else {
        (text.trim().to_owned(), None)
    };
    let profile=if pinyin.as_ref().is_some_and(|py|english_vowel_proxies.contains(&normalize_pinyin(py))){VOWEL_PROFILE}else{PINYIN_PROFILE};
    let pronunciation=if let Some((_,mut reading))=phonetic{let old=&note.pronunciation;if old.language==reading.language&&old.alphabet==reading.alphabet&&old.phonemes==reading.phonemes&&old.allow_approximation==reading.allow_approximation{reading.phoneme_durations_permille=old.phoneme_durations_permille.clone();reading.stress=old.stress;if reading.reading_provenance.is_none(){reading.reading_provenance=old.reading_provenance.clone();reading.mora_weights=old.mora_weights.clone();reading.frontend_profile=old.frontend_profile.clone();reading.input_text=old.input_text.clone();}}reading.unknown=old.unknown.clone();Some(reading)}else{verdant_domain::pronunciation_for_lyric(&note.pronunciation,&note.lyric,&lyric,pinyin.as_deref().map(|py|(py,profile)))};
    let mut changed=note.clone();changed.lyric=lyric;changed.set_continuation(matches!(changed.lyric.trim(),"+"|"-"));if let Some(pronunciation)=pronunciation{changed.pronunciation=pronunciation;}
    Ok(changed)
}

fn lyric_position(note:&Note,ordered:&[Note])->Result<usize,String>{
    // Track mutations maintain (start_tick, id) order; selected_notes uses the
    // same order, so batch edits can cache anchors without rescanning chains.
    ordered.binary_search_by(|n|(n.start_tick,&n.id).cmp(&(note.start_tick,&note.id))).map_err(|_|"歌词目标音符已不存在".into())
}

fn touching_lyrics(left:&Note,right:&Note)->bool{
    !left.is_muted()&&!right.is_muted()&&left.duration_tick>0&&right.duration_tick>0&&left.end_tick()==right.start_tick
}

/// Resolve the actual preceding authored chain, including edits earlier in this
/// batch. Never jump over a rest, an unselected onset, or a muted note.
fn preceding_syllable<'a>(note:&Note,ordered:&'a [Note],edits:&'a std::collections::BTreeMap<NoteId,Note>,anchors:&mut std::collections::BTreeMap<NoteId,usize>)->Result<&'a Note,String>{
    let mut index=lyric_position(note,ordered)?;let mut next=note;
    loop{
        index=index.checked_sub(1).ok_or("延音前缺少同轨道的音节")?;
        let original=&ordered[index];let previous=edits.get(&original.id).unwrap_or(original);
        if !touching_lyrics(previous,next){return Err("延音须紧邻前一音符，不能跨过休止、重叠或静音".into());}
        let anchor_index=if !previous.is_continuation(){Some(index)}else{anchors.get(&previous.id).copied()};
        if let Some(anchor_index)=anchor_index{
            let anchor=&ordered[anchor_index];let anchor=edits.get(&anchor.id).unwrap_or(anchor);
            if anchor.lyric.trim().is_empty(){return Err("延音前缺少已填写的音节".into());}
            anchors.insert(note.id.clone(),anchor_index);return Ok(anchor);
        }
        next=previous;
    }
}

fn same_lyric_reading(a:&Pronunciation,b:&Pronunciation)->bool{
    a.pinyin==b.pinyin&&a.phonemes==b.phonemes&&a.language==b.language&&a.alphabet==b.alphabet
        &&a.allow_approximation==b.allow_approximation&&a.stress==b.stress&&a.mora_weights==b.mora_weights
        &&a.phoneme_durations_permille==b.phoneme_durations_permille
}

fn explicit_lyric_reading(note:&Note)->bool{
    matches!(note.pronunciation.source,PronunciationSource::Manual|PronunciationSource::Imported)
}

fn continuation_note(note:&Note,anchor:&Note)->Result<Note,String>{
    let explicit=explicit_lyric_reading(note);
    if explicit&&!same_lyric_reading(&note.pronunciation,&anchor.pronunciation){
        return Err("后续音符有不同的显式读音；请先移出延音选择或明确修改其读音".into());
    }
    let mut changed=note.clone();changed.lyric="+".into();changed.set_continuation(true);
    // Continuations resolve the anchor at render time. Do not create stale
    // copies of its pronunciation or replay its consonants at the new pitch.
    if !explicit{changed.pronunciation=Pronunciation{unknown:note.pronunciation.unknown.clone(),..Default::default()};}
    Ok(changed)
}

fn distribute_lyrics(notes:&[Note],track:&[Note],tokens:&[String],proxies:&[String])->Result<Vec<Note>,String>{
    if tokens.is_empty()||notes.is_empty(){return Err("输入歌词并选择目标音符".into());}
    if tokens.len()>notes.len(){return Err(format!("{} 个歌词超过 {} 个选中音符；未截断歌词",tokens.len(),notes.len()));}
    let mut edits=std::collections::BTreeMap::new();let mut anchors=std::collections::BTreeMap::new();
    for(note,token)in notes.iter().zip(tokens){
        let changed=if matches!(token.as_str(),"+"|"-"){
            continuation_note(note,preceding_syllable(note,track,&edits,&mut anchors)?)?
        }else{lyric_note(note,token,proxies)?};
        edits.insert(note.id.clone(),changed);
    }
    Ok(notes.iter().filter_map(|n|edits.remove(&n.id)).collect())
}

fn continued_syllable(notes:&[Note],track:&[Note])->Result<Vec<Note>,String>{
    let Some(anchor)=notes.first().filter(|_|notes.len()>=2)else{return Err("选择音节起点及其后续音符".into());};
    if anchor.lyric.trim().is_empty()||anchor.is_continuation(){return Err("第一个音符须有明确音节".into());}
    let mut previous_index=lyric_position(anchor,track)?;
    let mut changed=Vec::with_capacity(notes.len());changed.push(anchor.clone());
    for pair in notes.windows(2){
        let index=lyric_position(&pair[1],track)?;
        if index!=previous_index+1||!touching_lyrics(&pair[0],&pair[1]){return Err("延音须选择连续相接的非静音音符".into());}
        changed.push(continuation_note(&pair[1],anchor)?);previous_index=index;
    }
    Ok(changed)
}

/// Rotate lyric ownership, not pitch or timing. A sustain cut by the cyclic
/// selection boundary gets an onset at its new beginning; adjacent pieces of
/// the same syllable remain linked to a single onset.
fn shifted_lyrics(notes:&[Note],track:&[Note],right:bool)->Result<Vec<Note>,String>{
    if notes.len()<2{return Err("至少选择两个音符".into());}
    let edits=std::collections::BTreeMap::new();let mut cached_anchors=std::collections::BTreeMap::new();
    let anchors=notes.iter().map(|n|if n.is_continuation(){
        let anchor=preceding_syllable(n,track,&edits,&mut cached_anchors)?;
        if explicit_lyric_reading(n)&&!same_lyric_reading(&n.pronunciation,&anchor.pronunciation){return Err("延音有不同的显式读音；请先解决读音冲突".into());}
        Ok(anchor)
    }else{Ok(n)}).collect::<Result<Vec<_>,String>>()?;
    let mut changed=Vec::with_capacity(notes.len());let mut previous_owner:Option<&NoteId>=None;let mut previous_index=None;
    for(index,destination)in notes.iter().enumerate(){
        let source=if right{(index+notes.len()-1)%notes.len()}else{(index+1)%notes.len()};let anchor=anchors[source];
        let position=lyric_position(destination,track)?;
        let continuation=previous_owner==Some(&anchor.id)&&previous_index.is_some_and(|p|p+1==position)
            &&index>0&&touching_lyrics(&notes[index-1],destination);
        let mut note=destination.clone();
        note.lyric=if continuation{"+".into()}else{anchor.lyric.clone()};
        note.pronunciation=if notes[source].is_continuation()&&explicit_lyric_reading(&notes[source]){notes[source].pronunciation.clone()}
            else if continuation{Pronunciation::default()}else{anchor.pronunciation.clone()};
        note.set_continuation(continuation);changed.push(note);previous_owner=Some(&anchor.id);previous_index=Some(position);
    }
    Ok(changed)
}

#[cfg(test)]
#[path="batch_lyrics_tests.rs"]
mod batch_lyrics_tests;
