//! Native editor integrations. Every authored edit enters the ordinary document journal.
use super::*;
#[path="audio_edit_ui.rs"]
pub mod audio_edit_ui;
use verdant_domain::automation::{AutomationDocument,AutomationLane,AutomationOwner,AutomationPoint,AutomationTarget,Interpolation as AutomationInterpolation};
use verdant_media::{AssetPolicy,AudioClip,ImportLimits,MediaImporter};
use verdant_midi::{CaptureOrigin,MidiCapture,MidiInputService,MidiPort,MidiTake};

#[derive(Clone,Debug)]
pub enum IntegrationAction {
    AudioEdit(audio_edit_ui::AudioUiAction),Menu,Singers,ChooseSinger(String),InstallVoice,Phonemizers,ChoosePhonemizer(String),ChooseSingerPhonemizer(String,Option<String>),Segments,AddSegment,EditSegment(String),RemoveSegment(String),Dictionary,AddDictionaryEntry,EditDictionaryEntry(String,bool),RemoveDictionaryEntry(String,bool),ImportDictionary,ExportDictionary,
    MonitorPorts,Monitor(MidiPort),StopMonitoring,
    MidiPorts,MidiChannels(MidiPort),Record(MidiPort,Option<u8>),FinishRecording,CancelRecording,ImportAudio,AuthorizeAudio,Clips,SelectClip(TrackId,String),EditClip,DuplicateClip,DeleteClip,RelinkClip,
    Automation,EditAutomation(AutomationTarget),ToggleAutomation(AutomationTarget),RemoveAutomation(AutomationTarget),ScoreExport(verdant_exchange::Format),ExportScoreTrack(verdant_exchange::Format,usize),VocaloidMetadata,
    Scale,OtherTracks,ThreeLanes,AuditionSelection,Report,
}
#[derive(Clone,Debug)]
pub enum IntegrationPrompt { AudioEdit(audio_edit_ui::AudioPrompt),InstallVoice(super::voice_ui::VoiceImportDraft),Dictionary(Option<(String,bool)>),Segment(Option<String>),Clip(TrackId,String),Automation(AutomationTarget),Vocaloid,Scale }

pub struct Recording {
    capture:MidiCapture,take:Option<MidiTake>,track:TrackId,base:Revision,tempo:TempoMap,origin_micros:u64,origin_tick:i64,
}
pub struct IntegrationState {
    pub timeline_preparation:verdant_service::preparation::TimelinePreparationGate,
    pub audio_edit:audio_edit_ui::AudioEditUiState,
    pub performance:crate::performance::PerformanceState,
    pub asset_policy:AssetPolicy,pub selected_clip:Option<(TrackId,String)>,pub recording:Option<Recording>,pub media_ready:bool,pub media_cache:verdant_media::WaveformCache,pub media_previews:HashMap<String,Arc<verdant_media::WaveformPreview>>,pub media_save_generation:u64,pub media_file_jobs:usize,
    pub report:String,pub other_tracks:bool,/// Parameter lanes shown under the roll, top to bottom; `active_panel` edits.
    pub lanes:Vec<Param>,pub active_panel:usize,pub scale_root:u8,pub scale_intervals:Vec<u8>,
}
impl Default for IntegrationState {
    fn default()->Self{Self{timeline_preparation:Default::default(),audio_edit:Default::default(),performance:Default::default(),asset_policy:AssetPolicy::new(Vec::<PathBuf>::new()).expect("empty asset policy"),selected_clip:None,recording:None,media_ready:false,media_cache:verdant_media::WaveformCache::default(),media_previews:HashMap::new(),media_save_generation:0,media_file_jobs:0,
        report:String::new(),other_tracks:false,lanes:vec![Param::Pit],active_panel:0,scale_root:0,scale_intervals:Vec::new()}}
}
fn action(label:impl Into<gpui::SharedString>,command:IntegrationAction)->crate::menu::MenuItem{
    crate::menu::MenuItem::action(label,MenuAction::Workflow(WorkflowAction::Integration(command)))
}
impl Editor {
    pub fn integrations_menu(&mut self,window:&mut Window,cx:&mut Context<Self>){
        use IntegrationAction as A;
        let items=vec![action("轨道歌手…",A::Singers),action("导入 verdant-voice 声库…",A::InstallVoice),action("读音器…",A::Phonemizers),action("用户读音词典…",A::Dictionary),action("导入词典…",A::ImportDictionary),action("导出词典…",A::ExportDictionary),
            action("MIDI 实时监听…",A::MonitorPorts),action("MIDI 输入录音…",A::MidiPorts),action("完成 MIDI 录音",A::FinishRecording),action("取消 MIDI 录音",A::CancelRecording),
            action("导入音频伴奏 / 参考音…",A::ImportAudio),action("授权重新打开已保存音频…",A::AuthorizeAudio),action("音频片段…",A::Clips),action("编辑音频片段位置 / 裁剪 / 音量…",A::EditClip),action("复制音频片段",A::DuplicateClip),action("删除音频片段",A::DeleteClip),action("重新定位音源…",A::RelinkClip),action("自动化曲线…",A::Automation),
            action("导出 MIDI…",A::ScoreExport(verdant_exchange::Format::Midi)),action("导出 UST…",A::ScoreExport(verdant_exchange::Format::Ust)),action("导出 USTX…",A::ScoreExport(verdant_exchange::Format::Ustx)),action("导出 SVP…",A::ScoreExport(verdant_exchange::Format::Svp)),action("导出 VSQX…",A::ScoreExport(verdant_exchange::Format::Vsqx)),action("导出 VPR…",A::ScoreExport(verdant_exchange::Format::Vpr)),action("导出 UtaFormatix Data (.ufdata)…",A::ScoreExport(verdant_exchange::Format::Ufdata)),action("Vocaloid 歌手元数据…",A::VocaloidMetadata),action("查看导入 / 导出报告…",A::Report),
            action("音阶高亮设置…",A::Scale),action("其他轨道音符叠加开关",A::OtherTracks),action("三个参数面板开关",A::ThreeLanes),action("试听所选音高（参考正弦音）",A::AuditionSelection)];
        self.show_menu(window.mouse_position(),items,window,cx);
    }
    pub fn run_integration(&mut self,command:IntegrationAction,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        use IntegrationAction as A;
        match command{
            A::AudioEdit(action)=>self.run_audio_ui(action,window,cx)?,
            A::Menu=>self.integrations_menu(window,cx),
            A::Singers=>{
                let items=self.singer_menu_items()?;
                self.show_menu(window.mouse_position(),items,window,cx);
            }
            A::ChooseSinger(reference)=>{
                let caps = self.scheduler.capabilities.as_ref().ok_or("backend capabilities unavailable")?;
                let batch = crate::desktop_engine::singer_choice_batch(self.project(), &self.session.track, &reference, caps)?;
                if !self.commit(self.revision(),batch,cx){return Err("歌手选择未提交".into());}
            }
            A::InstallVoice=>self.install_voice_dialog(window,cx)?,
            A::Phonemizers=>self.phonemizer_menu(window,cx)?,
            A::ChoosePhonemizer(selected)=>{let mut settings=verdant_backend::phonemizer::settings_from_project(self.project())?;settings.selected=selected;self.set_phonemizer(settings,cx)?;}
            A::ChooseSingerPhonemizer(reference,selected)=>{let mut settings=verdant_backend::phonemizer::settings_from_project(self.project())?;if let Some(selected)=selected{settings.singer_selections.insert(reference,selected);}else{settings.singer_selections.remove(&reference);}self.set_phonemizer(settings,cx)?;}
            A::Segments=>self.segments_menu(window,cx)?,
            A::AddSegment=>self.open_segment_dialog(None,window,cx)?,
            A::EditSegment(key)=>self.open_segment_dialog(Some(key),window,cx)?,
            A::RemoveSegment(key)=>{let mut settings=verdant_backend::phonemizer::settings_from_project(self.project())?;if settings.segments.remove(&key).is_none(){return Err("分段映射已不存在".into());}self.set_phonemizer(settings,cx)?;}
            A::Dictionary=>self.dictionary_menu(window,cx)?,
            A::AddDictionaryEntry=>self.prompt_integration("工程词典：歌词/读音，以空格分开；如 你/ni3 好/hao3 或 海/[ja/kana:うみ]",IntegrationPrompt::Dictionary(None),"",window,cx),
            A::EditDictionaryEntry(key,phrase)=>{let settings=verdant_backend::phonemizer::settings_from_project(self.project())?;let text=if phrase{let entry=settings.dictionary.phrases.get(&key).ok_or("词组已不存在")?;entry.lyrics.iter().zip(&entry.readings).map(|(lyric,reading)|reading_text(lyric,reading)).collect::<Vec<_>>().join(" ")}else{let entry=settings.dictionary.lyrics.get(&key).ok_or("词条已不存在")?;reading_text(&key,&entry.reading)};self.prompt_integration("编辑词条：歌词/读音，以空格分开；显式音标可保留方括号中的空格",IntegrationPrompt::Dictionary(Some((key,phrase))),&text,window,cx);}
            A::RemoveDictionaryEntry(key,phrase)=>{let mut settings=verdant_backend::phonemizer::settings_from_project(self.project())?;if phrase{settings.dictionary.phrases.remove(&key);}else{settings.dictionary.lyrics.remove(&key);}self.set_phonemizer(settings,cx)?;}
            A::ImportDictionary=>self.dictionary_file_dialog(false,window,cx)?,A::ExportDictionary=>self.dictionary_file_dialog(true,window,cx)?,
            A::MonitorPorts=>{
                let ports=MidiInputService::ports().map_err(|e|e.to_string())?;
                let mut items=ports.into_iter().map(|p|action(p.name.clone(),A::Monitor(p))).collect::<Vec<_>>();
                if items.is_empty(){items.push(crate::menu::MenuItem::disabled("没有可用的 MIDI 输入设备"));}
                items.push(action("停止 MIDI 监听",A::StopMonitoring));self.show_menu(window.mouse_position(),items,window,cx);
            }
            A::Monitor(port)=>self.start_instrument_monitor(&port,cx)?,
            A::StopMonitoring=>self.stop_instrument_monitor(cx),
            A::MidiPorts=>{let ports=MidiInputService::ports().map_err(|e|e.to_string())?;let mut items=ports.into_iter().map(|p|action(format!("{} · 选择通道…",p.name),A::MidiChannels(p))).collect::<Vec<_>>();if items.is_empty(){items.push(crate::menu::MenuItem::disabled("没有可用的 MIDI 输入设备"));}self.show_menu(window.mouse_position(),items,window,cx);}
            A::MidiChannels(port)=>{let mut items=vec![action(format!("{} · 所有通道（默认）",port.name),A::Record(port.clone(),None))];items.extend((0..16).map(|channel|action(format!("{} · 通道 {}",port.name,channel+1),A::Record(port.clone(),Some(channel)))));self.show_menu(window.mouse_position(),items,window,cx);}
            A::Record(port,channel)=>self.start_midi_recording(&port,channel,cx)?,A::FinishRecording=>self.finish_midi_recording(cx)?,A::CancelRecording=>{self.cancel_midi_recording("已取消 MIDI 录音");cx.notify();}
            A::ImportAudio=>self.audio_file_dialog(false,false,window,cx),A::AuthorizeAudio=>self.audio_file_dialog(true,false,window,cx),A::RelinkClip=>{self.selected_audio_clip()?;self.audio_file_dialog(false,true,window,cx);}
            A::Clips=>{let mut items=Vec::new();for track in &self.project().tracks{for clip in verdant_media::read_clips(track).map_err(|e|e.to_string())?{items.push(action(format!("{} · {} · tick {}",track.name,Path::new(&clip.source.path).file_name().unwrap_or_default().to_string_lossy(),clip.start_tick),A::SelectClip(track.id.clone(),clip.id.to_string())));}}if items.is_empty(){items.push(crate::menu::MenuItem::disabled("尚无音频片段"));}self.show_menu(window.mouse_position(),items,window,cx);}
            A::SelectClip(track,id)=>{self.select_track(track.clone(),cx);if self.session.track!=track{return Err("先完成当前歌词编辑".into());}self.integrations.selected_clip=Some((track,id));let clip=self.selected_audio_clip()?;self.select_audio_clip(&clip,false,false);cx.notify();}
            A::EditClip=>self.open_audio_clip_dialog(window,cx)?,
            A::DuplicateClip=>self.run_audio_ui(audio_edit_ui::AudioUiAction::Duplicate,window,cx)?,
            A::DeleteClip=>self.run_audio_ui(audio_edit_ui::AudioUiAction::Delete,window,cx)?,
            A::Automation=>self.automation_menu(window,cx)?,
            A::EditAutomation(target)=>{
                match target.owner() {
                    AutomationOwner::Track { track_id } => {
                        self.automation_ui.selected_targets.insert(track_id.clone(), target);
                        self.automation_ui.expanded_tracks.insert(track_id);
                    }
                    AutomationOwner::Master => {
                        self.automation_ui.master_target = Some(target);
                        self.automation_ui.show_master = true;
                    }
                }
                self.notice("已在上方排列视图展开图形化自动化泳道：点击加点、拖拽改值、右键切换阶梯或删除");
                cx.notify();
            }
            A::ToggleAutomation(target)=>{let mut document=AutomationDocument::from_project(self.project()).map_err(|e|e.to_string())?;let mut lane=document.lane(&target).cloned().ok_or("自动化曲线不存在")?;lane.enabled=!lane.enabled;document.set_lane(lane);self.commit_automation(document,cx)?;}
            A::RemoveAutomation(target)=>{let mut document=AutomationDocument::from_project(self.project()).map_err(|e|e.to_string())?;document.remove_lane(&target);self.commit_automation(document,cx)?;}
            A::ScoreExport(format)=>crate::files::export_score_dialog(self,format,window,cx),
            A::ExportScoreTrack(format,index)=>crate::files::export_score_with(self,format,Some(index),window,cx),
            A::VocaloidMetadata=>{let voice=self.project().extensions.get("vocaloid_voice").map(|v|serde_json::from_value::<verdant_exchange::VocaloidVoice>(v.clone())).transpose().map_err(|e|e.to_string())?;let text=voice.map(|v|format!("{} | {} | {} | {}",v.comp_id,v.name,v.language_id,v.program)).unwrap_or_default();self.prompt_integration("Vocaloid：真实 16 字符 compID | 歌手名称 | 语言 0..127 | 程序 0..127",IntegrationPrompt::Vocaloid,&text,window,cx);}
            A::Scale=>self.open_scale_dialog(window,cx),
            A::OtherTracks=>{self.integrations.other_tracks=!self.integrations.other_tracks;cx.notify();},A::ThreeLanes=>{self.add_lane(None,cx);},
            A::AuditionSelection=>{let pitch=self.selected_notes().first().map(|n|n.midi_note).ok_or("先选择音符")?;self.audition_pitch(pitch);}
            A::Report=>self.show_integration_report(cx),
        }Ok(())
    }
    fn prompt_integration(&mut self,title:impl Into<String>,purpose:IntegrationPrompt,text:&str,window:&mut Window,cx:&mut Context<Self>){self.prompt_text(title,PromptPurpose::Integration(purpose),text,window,cx);}
    pub(crate) fn show_integration_report(&mut self, cx: &mut Context<Self>) {
        // Read-only diagnostics stay in the same window and never borrow the
        // lyric input handle or confirm an active lyric/IME session.
        self.open_right_dock(crate::ui_controls::RightDockTab::RenderInserts, cx);
    }
    pub(super) fn apply_integration_prompt(&mut self,purpose:&IntegrationPrompt,text:&str,cx:&mut Context<Self>)->Result<(),String>{
        match purpose{
            IntegrationPrompt::AudioEdit(prompt)=>self.apply_audio_prompt(prompt,text,cx)?,
            IntegrationPrompt::InstallVoice(draft)=>self.submit_voice_import(draft,text)?,
            IntegrationPrompt::Dictionary(original)=>{
                let(lyrics,mut readings)=dictionary_entry(text)?;let key=lyrics.concat();let mut settings=verdant_backend::phonemizer::settings_from_project(self.project())?;
                let phrase=lyrics.len()>1;let same_entry=original.as_ref().is_some_and(|(old,was_phrase)|old==&key&&*was_phrase==phrase);
                if !same_entry&&(if phrase{settings.dictionary.phrases.contains_key(&key)}else{settings.dictionary.lyrics.contains_key(&key)}){return Err(format!("「{key}」已有词条，请从词典菜单编辑；未覆盖已有读音"));}
                if let Some((old,was_phrase))=original{if !(if *was_phrase{settings.dictionary.phrases.contains_key(old)}else{settings.dictionary.lyrics.contains_key(old)}){return Err("原词条已不存在，请重新打开词典菜单".into());}}
                let mut unknown=Default::default();
                if let Some((old,phrase))=original{if *phrase{if let Some(entry)=settings.dictionary.phrases.remove(old){unknown=entry.unknown;for(reading,previous)in readings.iter_mut().zip(entry.readings){reading.unknown=previous.unknown;}}}else if let Some(entry)=settings.dictionary.lyrics.remove(old){unknown=entry.unknown;readings[0].unknown=entry.reading.unknown;}}
                if lyrics.len()==1{settings.dictionary.lyrics.insert(key,verdant_backend::phonemizer::LyricEntry{reading:readings.remove(0),unknown});}else{settings.dictionary.phrases.insert(key,verdant_backend::phonemizer::PhraseEntry{lyrics,readings,unknown});}
                self.set_phonemizer(settings,cx)?;
            }
            IntegrationPrompt::Segment(original)=>{
                let fields=text.split([',','，']).map(str::trim).collect::<Vec<_>>();
                if fields.len()!=4||fields.iter().any(|field|field.is_empty()){return Err("填写 轨道序号, 起始tick, 结束tick, 读音器id；如 1, 960, 24240, mandarin-dictionary-v1".into());}
                let index=fields[0].parse::<usize>().map_err(|_|"轨道序号需要从 1 开始的整数")?.checked_sub(1).ok_or("轨道序号从 1 开始")?;
                let count=self.project().tracks.len();let track=self.project().tracks.get(index).map(|track|track.id.clone()).ok_or_else(||format!("轨道序号超出范围：本工程共 {count} 轨"))?;
                let start_tick=fields[1].parse::<i64>().map_err(|_|"起始tick 需要整数")?;let end_tick=fields[2].parse::<i64>().map_err(|_|"结束tick 需要整数")?;
                let selected=fields[3];if !verdant_backend::phonemizer::capabilities().iter().any(|p|p.id==selected){return Err(format!("读音器id 未知，可选 {}",verdant_backend::phonemizer::capabilities().iter().map(|p|p.id).collect::<Vec<_>>().join(" / ")));}
                let mut settings=verdant_backend::phonemizer::settings_from_project(self.project())?;let mut unknown=Default::default();
                let key=match original{Some(old)=>{unknown=settings.segments.remove(old).ok_or("原分段映射已不存在，请重新打开分段映射菜单")?.unknown;old.clone()},None=>NoteId::generate().to_string()};
                settings.segments.insert(key,verdant_backend::phonemizer::SegmentEntry{track,start_tick,end_tick,selected:selected.into(),unknown});
                self.set_phonemizer(settings,cx)?;
            }
            IntegrationPrompt::Clip(track,id)=>{
                if *track!=self.session.track{return Err("轨道已切换，请重新打开音频片段设置".into());}
                let fields=super::workflow::exact_numeric_fields(text,6)?;
                let start_tick=fields[0].parse::<i64>().map_err(|_|"起点 tick 须为非负 i64 整数")?;
                if start_tick<0{return Err("起点 tick 须为非负整数".into());}
                let offset_frames=fields[1].parse::<u64>().map_err(|_|"源起点帧须为 u64 非负整数")?;
                let duration_frames=fields[2].parse::<u64>().map_err(|_|"时长帧须为 u64 正整数")?;
                let gain=fields[3].parse::<f32>().map_err(|_|"增益须为有限非负数")?;
                let pan=fields[4].parse::<f32>().map_err(|_|"声像须为 -1..1 的有限数值")?;
                let muted=match fields[5]{"0"=>false,"1"=>true,_=>return Err("静音须为 0 或 1".into())};
                let mut clips=verdant_media::read_clips(self.project().track(track).ok_or("轨道不存在")?).map_err(|e|e.to_string())?;
                let clip=clips.iter_mut().find(|c|c.id.to_string()==*id).ok_or("音频片段已不存在")?;
                *clip=clip.trim_source(offset_frames,duration_frames,start_tick as f64,&self.tempo()).map_err(|e|e.to_string())?;
                clip.gain=gain;clip.pan=pan;clip.muted=muted;
                clip.validate().map_err(|e|e.to_string())?;
                self.commit_audio_clips(track,&clips,"编辑音频片段",cx)?;
                self.rebuild_timeline(cx);
            }
            IntegrationPrompt::Automation(target)=>{let mut document=AutomationDocument::from_project(self.project()).map_err(|e|e.to_string())?;let mut lane=document.lane(target).cloned().unwrap_or_else(||AutomationLane::new(target.clone()));edit_automation_points(&mut lane,text)?;self.validate_plugin_lane(&lane)?;document.set_lane(lane);self.commit_automation(document,cx)?;}
            IntegrationPrompt::Vocaloid=>{let voice=parse_vocaloid_voice(text)?;if !self.commit(self.revision(),CommandBatch::single("Vocaloid 歌手元数据",Command::SetProjectExtension{key:"vocaloid_voice".into(),value:Some(serde_json::to_value(voice).map_err(|e|e.to_string())?)}),cx){return Err("元数据未提交".into());}}
            IntegrationPrompt::Scale=>{let(root,intervals)=text.split_once(';').ok_or("使用 根音;级数 格式")?;let root=root.trim().parse::<u8>().map_err(|_|"根音需要 0..11")?;let mut intervals=intervals.split(',').map(str::trim).filter(|v|!v.is_empty()).map(|v|v.parse::<u8>().map_err(|_|"级数需要 0..11")).collect::<Result<Vec<_>,_>>()?;if root>11||intervals.iter().any(|v|*v>11){return Err("根音与级数需要 0..11".into());}intervals.sort();intervals.dedup();self.integrations.scale_root=root;self.integrations.scale_intervals=intervals;cx.notify();}
        }Ok(())
    }
    fn phonemizer_menu(&mut self,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        use IntegrationAction as A;
        let settings=verdant_backend::phonemizer::settings_from_project(self.project())?;
        let has_segments=self.active_track().is_some_and(|track|settings.segments.values().any(|entry|entry.track==track.id));
        let mut items=vec![crate::menu::MenuItem::header(if has_segments{"工程默认读音器（未设歌手覆盖且不在分段映射区间时使用）"}else{"工程默认读音器（未设歌手覆盖时使用）"})];
        if self.active_track().is_some_and(|track|track.unknown.contains_key("voice_language")){items.push(crate::menu::MenuItem::disabled(if has_segments{"当前轨道已指定语言，除分段映射覆盖的音符外优先使用该语言读音器；清除轨道语言后恢复以下设置"}else{"当前轨道已指定语言，优先使用该语言读音器；清除轨道语言后恢复以下设置"}));}
        for p in verdant_backend::phonemizer::capabilities(){items.push(action(format!("工程 · {}",p.name),A::ChoosePhonemizer(p.id.into())).checked(settings.selected==p.id));}
        if let Some(reference)=self.active_track().and_then(|track|track.voice_ref.as_ref()){
            let selected=settings.singer_selections.get(reference);
            items.push(crate::menu::MenuItem::header(format!("歌手 {reference} · 此工程所有同歌手轨道")));
            items.push(action("歌手 · 跟随工程默认",A::ChooseSingerPhonemizer(reference.clone(),None)).checked(selected.is_none()));
            for p in verdant_backend::phonemizer::capabilities(){items.push(action(format!("歌手 · {}",p.name),A::ChooseSingerPhonemizer(reference.clone(),Some(p.id.into()))).checked(selected.is_some_and(|id|id.as_str()==p.id)));}
        }else{items.push(crate::menu::MenuItem::disabled("先选择轨道歌手，可为该歌手设置读音器"));}
        items.push(action("分段映射…",A::Segments));
        self.show_menu(window.mouse_position(),items,window,cx);Ok(())
    }
    fn dictionary_menu(&mut self,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        let settings=verdant_backend::phonemizer::settings_from_project(self.project())?;let mut items=vec![crate::menu::MenuItem::header("工程词典 · 显式音符优先，其次词组、词条，再使用内置读音"),action("添加歌词 / 词组读音…",IntegrationAction::AddDictionaryEntry)];
        for(key,entry)in &settings.dictionary.lyrics{items.push(action(format!("编辑 · {}",reading_text(key,&entry.reading)),IntegrationAction::EditDictionaryEntry(key.clone(),false)));items.push(action(format!("删除 · {key}"),IntegrationAction::RemoveDictionaryEntry(key.clone(),false)));}
        for(key,entry)in &settings.dictionary.phrases{let text=entry.lyrics.iter().zip(&entry.readings).map(|(lyric,reading)|reading_text(lyric,reading)).collect::<Vec<_>>().join(" ");items.push(action(format!("编辑词组 · {text}"),IntegrationAction::EditDictionaryEntry(key.clone(),true)));items.push(action(format!("删除词组 · {key}"),IntegrationAction::RemoveDictionaryEntry(key.clone(),true)));}
        items.push(action("导入词典文件…",IntegrationAction::ImportDictionary));items.push(action("导出词典文件…",IntegrationAction::ExportDictionary));self.show_menu(window.mouse_position(),items,window,cx);Ok(())
    }
    fn segments_menu(&mut self,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        use IntegrationAction as A;
        let settings=verdant_backend::phonemizer::settings_from_project(self.project())?;let mut items=vec![crate::menu::MenuItem::disabled("分段映射 · 按 tick 区间选择读音器；显式音符与用户词典仍然优先"),action("添加分段映射…",A::AddSegment)];
        for(key,entry)in &settings.segments{let track=self.project().track(&entry.track).map(|t|t.name.as_str()).unwrap_or("已删除轨道");let frontend=verdant_backend::phonemizer::capabilities().iter().find(|p|entry.selected==p.id).map_or(entry.selected.as_str(),|p|p.name);let label=format!("{track} · {}–{} · {frontend}",entry.start_tick,entry.end_tick);items.push(action(format!("编辑 · {label}"),A::EditSegment(key.clone())));items.push(action(format!("删除 · {label}"),A::RemoveSegment(key.clone())));}
        if settings.segments.is_empty(){items.push(crate::menu::MenuItem::disabled("尚无分段映射；未覆盖的 tick 使用轨道 / 歌手 / 工程默认读音器"));}
        self.show_menu(window.mouse_position(),items,window,cx);Ok(())
    }
    fn automation_label(&self,target:&AutomationTarget)->String{
        match target{AutomationTarget::MasterGain=>"主音量（毫分贝）".into(),AutomationTarget::MasterPan=>"主声像（千分位）".into(),AutomationTarget::TrackGain{track_id}=>format!("{} · 音量（毫分贝）",self.project().track(track_id).map(|t|t.name.as_str()).unwrap_or("已删除轨道")),AutomationTarget::TrackPan{track_id}=>format!("{} · 声像（千分位）",self.project().track(track_id).map(|t|t.name.as_str()).unwrap_or("已删除轨道")),AutomationTarget::PluginParameter{owner,instance_id,parameter_id}=>{let name=match owner{AutomationOwner::Track{track_id}=>self.insert_specs(track_id).iter().find(|s|&s.instance_id==instance_id).map(|s|s.name.clone()).unwrap_or_else(||"未载入插件".into()),AutomationOwner::Master=>"主输出插件".into()};format!("{name} · {parameter_id}")}}
    }
    fn set_phonemizer(&mut self,settings:verdant_backend::phonemizer::PhonemizerSettings,cx:&mut Context<Self>)->Result<(),String>{let value=verdant_backend::phonemizer::settings_to_extension(&settings)?;if self.commit(self.revision(),CommandBatch::single("读音器与用户词典",Command::SetProjectExtension{key:verdant_backend::phonemizer::EXTENSION_KEY.into(),value:Some(value)}),cx){Ok(())}else{Err("读音配置未提交".into())}}
    fn dictionary_file_dialog(&mut self,export:bool,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        if export{let settings=verdant_backend::phonemizer::settings_from_project(self.project())?;let bytes=verdant_backend::phonemizer::export_dictionary(&settings.dictionary)?.into_bytes();let rx=cx.prompt_for_new_path(&PathBuf::from("."),Some("verdant-dictionary.json"));cx.spawn(async move|this,cx|{if let Ok(Ok(Some(path)))=rx.await{let result=cx.background_spawn(async move{crate::files::atomic_bytes(&path,&bytes)}).await;let _=this.update(cx,|ed,cx|{match result{Ok(())=>ed.notice("已导出用户读音词典"),Err(error)=>ed.notice(error)}cx.notify();});}}).detach();}
        else{
            let project_id=self.project().project_id.clone();let base=self.revision();
            let rx=cx.prompt_for_paths(gpui::PathPromptOptions{files:true,directories:false,multiple:false,prompt:Some("导入并替换工程用户词典（Verdant JSON，可撤销）".into())});
            cx.spawn_in(window,async move|this,cx|{if let Ok(Ok(Some(paths)))=rx.await{if let Some(path)=paths.first(){
                let path=path.clone();let result=cx.background_spawn(async move{crate::files::read_bounded(&path,16*1024*1024).and_then(|bytes|String::from_utf8(bytes).map_err(|e|e.to_string())).and_then(|text|verdant_backend::phonemizer::import_dictionary(&text))}).await;
                let _=cx.update(|_,cx|this.update(cx,|ed,cx|{
                    let result=result.and_then(|dictionary|{
                        if ed.project().project_id!=project_id||ed.revision()!=base{return Err("工程已切换或修改，未替换词典；请重新导入".into());}
                        let mut settings=verdant_backend::phonemizer::settings_from_project(ed.project())?;settings.dictionary=dictionary;ed.set_phonemizer(settings,cx)
                    });
                    match result{Ok(())=>ed.notice("已替换工程用户词典，可撤销"),Err(error)=>ed.notice(error)}cx.notify();
                }));
            }}}).detach();
        }Ok(())
    }
    fn start_midi_recording(&mut self,port:&MidiPort,channel:Option<u8>,cx:&mut Context<Self>)->Result<(),String>{
        if self.integrations.recording.is_some(){return Err("已有 MIDI 录音进行中".into());}if !self.commit_pending_lyric(cx){return Err("先完成歌词编辑".into());}
        // Count-in bars delay both capture origin and playback by the same
        // pre-roll: events pressed during the clicks land before the entry
        // tick, exactly as if the take had started at the playhead.
        let tick=self.tempo().tick_at(self.playhead_seconds()).round()as i64;let bars=self.playback_settings().count_in_bars;
        let pre_roll=if bars>0{Some(super::workflow::count_in_track(self.project(),&self.tempo(),self.audio_info.sample_rate,tick,bars)?) }else{None};
        self.integrations.performance.capture=None;
        let track=self.session.track.clone();self.prepare_instrument_monitor(&track);
        let capture=MidiInputService::open(port,channel,65_536).map_err(|e|e.to_string())?;
        let origin_micros=capture.timestamp_micros().saturating_add(pre_roll.as_ref().map_or(0,|clicks|(clicks.length as f64/f64::from(self.audio_info.sample_rate)*1e6).round()as u64));
        let take=MidiTake::new(CaptureOrigin{timestamp_micros:origin_micros,timeline_tick:tick},channel).map_err(|e|e.to_string())?;
        self.integrations.recording=Some(Recording{capture,take:Some(take),track:self.session.track.clone(),base:self.revision(),tempo:self.tempo(),origin_micros,origin_tick:tick});
        let channel_label=channel.map(|channel|format!("通道 {}",channel+1)).unwrap_or_else(||"所有通道".into());
        let playing=match pre_roll{Some(clicks)=>self.audio.play_with_count_in(clicks.count_in_pcm()?),None=>{self.audio.play();true}};
        if !playing{self.cancel_midi_recording("预备拍队列忙，未开始录音；请重试");cx.notify();return Err("预备拍队列忙，未开始录音".into());}
        self.notice(format!("正在录制 {} · {}；完成后一次写入，编辑或定位会取消本次录音",port.name,channel_label));cx.notify();Ok(())
    }
    pub fn pump_midi(&mut self,cx:&mut Context<Self>){
        self.pump_instrument_monitor(cx);
        let Some(recording)=&mut self.integrations.recording else{return;};
        let mut events=Vec::new();
        let error=if recording.base!=self.persistence.store().revision(){Some("录音期间工程已改变，本次录音已取消".to_string())}else{
            let mut error=None;for _ in 0..8192{match recording.capture.try_recv(){
                Ok(Some(event))=>{events.push(event);if event.timestamp_micros>=recording.origin_micros{if let Err(reason)=recording.take.as_mut().expect("active take").push(event){error=Some(reason.to_string());break;}}},
                Ok(None)=>break,Err(reason)=>{error=Some(reason.to_string());break;}
            }}error
        };
        if let Some(error)=error{self.cancel_midi_recording(&format!("MIDI 录音拒绝：{error}；未写入部分结果"));self.audio.reset_monitor();cx.notify();}
        else{self.consume_performance_events(&events,cx);}
    }
    pub fn finish_midi_recording(&mut self,cx:&mut Context<Self>)->Result<(),String>{
        let Some(mut recording)=self.integrations.recording.take()else{return Err("当前没有 MIDI 录音".into());};
        self.audio.reset_monitor();
        let tail=recording.capture.stop().map_err(|e|e.to_string())?;let stop=recording.capture.timestamp_micros();let mut take=recording.take.take().expect("active take");for event in tail{if event.timestamp_micros>=recording.origin_micros{take.push(event).map_err(|e|e.to_string())?;}}
        if recording.base!=self.revision(){return Err("工程已改变，录音未提交".into());}let notes=take.finish(stop,&recording.tempo).map_err(|e|e.to_string())?;if notes.is_empty(){self.notice("MIDI 录音没有音符");return Ok(());}
        let batch=verdant_midi::notes_to_batch(recording.track,&notes,"");if !self.commit(recording.base,batch,cx){return Err("MIDI 录音未提交".into());}self.notice(format!("已写入 {} 个 MIDI 音符；请选择读音器或填写歌词",notes.len()));cx.notify();Ok(())
    }
    pub fn cancel_midi_recording(&mut self,message:&str){if self.integrations.recording.take().is_some(){self.notice(message);}}
    pub fn selected_audio_clip(&self)->Result<AudioClip,String>{let(track,id)=self.integrations.selected_clip.as_ref().ok_or("先在音频片段菜单或排列视图选择片段")?;verdant_media::read_clips(self.project().track(track).ok_or("轨道不存在")?).map_err(|e|e.to_string())?.into_iter().find(|clip|clip.id.to_string()==*id).ok_or("音频片段不存在".into())}
    fn commit_audio_clips(&mut self,track:&TrackId,clips:&[AudioClip],label:&str,cx:&mut Context<Self>)->Result<(),String>{let value=verdant_media::clips_value(track.clone(),clips).map_err(|e|e.to_string())?;if self.commit(self.revision(),CommandBatch::single(label,Command::SetTrackData{track:track.clone(),key:verdant_media::AUDIO_CLIPS_KEY.into(),value}),cx){Ok(())}else{Err("音频片段未提交".into())}}
    pub fn audio_file_dialog(&mut self,authorize:bool,relink:bool,window:&mut Window,cx:&mut Context<Self>){
        if !self.commit_pending_lyric(cx){return;}let track=self.session.track.clone();let project_id=self.project().project_id.clone();let tick=self.tempo().tick_at(self.playhead_seconds()).round()as i64;let selected=if relink{self.selected_audio_clip().ok()}else{None};
        let rx=cx.prompt_for_paths(gpui::PathPromptOptions{files:true,directories:false,multiple:authorize,prompt:Some(if authorize{"选择并授权工程引用的音频源"}else if relink{"选择内容指纹相同的新音源位置"}else{"导入 WAV / FLAC / MP3 / Ogg / M4A 音频"}.into())});
        cx.spawn_in(window,async move|this,cx|{if let Ok(Ok(Some(paths)))=rx.await{let _=cx.update(|_,cx|this.update(cx,|ed,cx|{
            if ed.project().project_id!=project_id{ed.notice("工程已切换，未导入音频");return;}
            let mut policy=ed.integrations.asset_policy.clone();for path in &paths{if let Err(error)=policy.allow_file(path){ed.notice(error.to_string());cx.notify();return;}}
            if authorize{ed.integrations.asset_policy=policy;ed.remember_media_authorizations(&paths);ed.rebuild_timeline(cx);return;}
            let Some(path)=paths.first().cloned()else{return;};
            cx.spawn(async move|this,cx|{let read_policy=policy.clone();let result=cx.background_spawn(async move{MediaImporter::new(read_policy,ImportLimits::default()).import_file(&path).map_err(|e|e.to_string())}).await;
                let _=this.update(cx,|ed,cx|{let result=(||->Result<(),String>{if ed.project().project_id!=project_id{return Err("工程已切换，未导入音频".into());}let imported=result?;let mut clips=verdant_media::read_clips(ed.project().track(&track).ok_or("目标轨道已删除")?).map_err(|e|e.to_string())?;
                    let clip=if let Some(previous)=selected{if previous.source.sha256!=imported.source.sha256{return Err("重新定位必须选择相同内容；要替换音源，请新建音频片段".into());}let clip=clips.iter_mut().find(|c|c.id==previous.id).ok_or("原片段已不存在")?;clip.source=imported.source;clip.validate().map_err(|e|e.to_string())?;clip.clone()}else{let clip=AudioClip::new(imported.source,track.clone(),tick);clips.push(clip.clone());clip};
                    ed.commit_audio_clips(&track,&clips,if relink{"重新定位音源"}else{"导入音频片段"},cx)?;ed.integrations.asset_policy=policy;ed.remember_media_authorizations(&[PathBuf::from(&clip.source.path)]);ed.select_audio_clip(&clip,false,false);ed.rebuild_timeline(cx);Ok(())})();if let Err(error)=result{ed.notice(format!("音频导入失败：{error}"));}cx.notify();});}).detach();
        }));}}).detach();
    }
    fn remember_media_authorizations(&mut self,paths:&[PathBuf]){let mut authorized=self.prefs.extra.get("media_authorizations").and_then(|v|v.as_array()).cloned().unwrap_or_default();for path in paths{if let Ok(path)=path.canonicalize(){let value=serde_json::Value::String(path.display().to_string());if !authorized.contains(&value){authorized.push(value);}}}self.prefs.extra.insert("media_authorizations".into(),authorized.into());self.prefs_dirty=true;}
    pub fn restore_media_authorizations(&mut self){if let Some(paths)=self.prefs.extra.get("media_authorizations").and_then(|v|v.as_array()){for path in paths.iter().filter_map(|v|v.as_str()){let _=self.integrations.asset_policy.allow_file(Path::new(path));}}}
    pub fn media_asset_policy(&self)->Result<AssetPolicy,String>{
        verdant_service::media::project_policy(self.persistence.project_path(),self.integrations.asset_policy.clone()).map_err(|error|error.to_string())
    }
    fn commit_automation(&mut self,document:AutomationDocument,cx:&mut Context<Self>)->Result<(),String>{document.validate(self.project()).map_err(|e|e.to_string())?;let value=document.extension_value().map_err(|e|e.to_string())?;if self.commit(self.revision(),CommandBatch::single("编辑自动化曲线",Command::SetProjectExtension{key:verdant_domain::automation::AUTOMATION_KEY.into(),value}),cx){Ok(())}else{Err("自动化未提交".into())}}
    pub fn validate_plugin_lane(&self,lane:&AutomationLane)->Result<(),String>{
        if let AutomationTarget::PluginParameter{owner,instance_id,parameter_id}=&lane.target{
            let handle=match owner{
                AutomationOwner::Track{track_id}=>{let specs=self.insert_specs(track_id);let slot=specs.iter().position(|spec|spec.instance_id==*instance_id).ok_or("自动化对应插件已不存在")?;self.live_insert(track_id,slot)}
                AutomationOwner::Master=>{let specs=self.master_insert_specs();let slot=specs.iter().position(|spec|spec.instance_id==*instance_id).ok_or("自动化对应主总线插件已不存在")?;self.live_master_insert(slot)}
            }.ok_or("插件不可用，保留曲线但无法提交新的参数自动化")?;
            let target=handle.compile_parameter(parameter_id).map_err(|e|e.to_string())?;lane.validate_parameter_range(target.min(),target.max()).map_err(|e|e.to_string())?;lane.validate_parameter_steps(target.step_count()).map_err(|e|e.to_string())?;for point in &lane.points{target.event(0,point.value).map_err(|e|e.to_string())?;}
        }Ok(())
    }
    fn automation_menu(&mut self,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        let track=self.session.track.clone();let mut items=vec![action("轨道音量（毫分贝）…",IntegrationAction::EditAutomation(AutomationTarget::TrackGain{track_id:track.clone()})),action("轨道声像（千分位）…",IntegrationAction::EditAutomation(AutomationTarget::TrackPan{track_id:track.clone()})),action("主音量（毫分贝）…",IntegrationAction::EditAutomation(AutomationTarget::MasterGain)),action("主声像（千分位）…",IntegrationAction::EditAutomation(AutomationTarget::MasterPan))];
        let mut specs=self.insert_specs(&track);let mut changed=false;for spec in &mut specs{if spec.instance_id.is_empty(){spec.instance_id=NoteId::generate().to_string();changed=true;}}
        if changed{if !self.commit(self.revision(),CommandBatch::single("为旧插件分配稳定自动化身份",Command::SetTrackInserts{track:track.clone(),inserts:specs.iter().map(|s|serde_json::to_value(s).expect("insert")).collect()}),cx){return Err("插件身份未写入工程".into());}return Err("插件身份已写入工程；等待链恢复后重新打开自动化菜单".into());}
        for(slot,spec)in specs.iter().enumerate(){if let Some(handle)=self.live_insert(&track,slot){let params=handle.params().map_err(|e|e.to_string())?;for param in params{items.push(action(format!("{} · {} [{}..{}]…",spec.name,param.name,param.min,param.max),IntegrationAction::EditAutomation(AutomationTarget::PluginParameter{owner:AutomationOwner::Track{track_id:track.clone()},instance_id:spec.instance_id.clone(),parameter_id:param.id})));}}}
        let document=AutomationDocument::from_project(self.project()).map_err(|e|e.to_string())?;for lane in document.lanes{let label=self.automation_label(&lane.target);items.push(action(format!("{} · {label}",if lane.enabled{"关闭曲线"}else{"启用曲线"}),IntegrationAction::ToggleAutomation(lane.target.clone())));items.push(action(format!("删除曲线 · {label}"),IntegrationAction::RemoveAutomation(lane.target)));}self.show_menu(window.mouse_position(),items,window,cx);Ok(())
    }
    pub fn recording_seconds(&self)->Option<f64>{self.integrations.recording.as_ref().map(|r|r.tempo.seconds_at(r.origin_tick as f64)+r.capture.timestamp_micros().saturating_sub(r.origin_micros)as f64/1e6)}
    pub fn audition_pitch(&mut self,pitch:i32){let rate=self.audio_info.sample_rate;let length=(f64::from(rate)*0.28)as usize;let frequency=verdant_domain::timing::midi_to_hz(pitch,0);let samples=(0..length).map(|i|{let t=i as f64/f64::from(rate);let envelope=(t/0.01).min(1.)*((0.28-t)/0.035).clamp(0.,1.);(0.14*envelope*(t*frequency*std::f64::consts::TAU).sin())as f32}).collect::<Vec<_>>();if !self.audio.audition(samples.into()){self.notice("参考音队列忙，请稍后重试");}}
}


fn reading_text(lyric:&str,reading:&Pronunciation)->String{if reading.alphabet.is_some(){format!("{lyric}/{}",verdant_domain::pronunciation_display(reading))}else{format!("{lyric}/{}",reading.pinyin)}}
fn dictionary_entry(text:&str)->Result<(Vec<String>,Vec<Pronunciation>),String>{
    let tokens=super::workflow::lyric_tokens(text)?;if tokens.is_empty(){return Err("至少填写一个 歌词/读音".into());}let mut lyrics=Vec::new();let mut readings=Vec::new();
    for token in tokens{let(lyric,reading)=if let Some(reading)=verdant_backend::reading::parse_phonetic_input(&token)?{reading}else{let(lyric,pinyin)=parse_lyric_input(&token);let pinyin=pinyin.filter(|s|!s.is_empty()).ok_or("词典需要明确读音，如 你/ni3 或 hello/[en:h ə . ˈl oʊ]")?;(lyric,Pronunciation{pinyin,language:Some("zh".into()),source:PronunciationSource::Manual,frontend_profile:PINYIN_PROFILE.into(),..Default::default()})};verdant_backend::reading::validate_pronunciation(&reading)?;lyrics.push(lyric);readings.push(reading);}
    Ok((lyrics,readings))
}
fn edit_automation_points(lane:&mut AutomationLane,text:&str)->Result<(),String>{
    let mut points=Vec::new();let mut previous=None;
    for entry in text.split([';','\n']).map(str::trim).filter(|v|!v.is_empty()){let fields=entry.split(',').map(str::trim).collect::<Vec<_>>();if fields.len()!=3{return Err("每个点使用 tick,数值,linear 或 tick,数值,step；点之间用分号分隔".into());}let tick=fields[0].parse::<i64>().map_err(|_|"tick 需要非负整数")?;let value=fields[1].parse::<f64>().map_err(|_|"自动化数值无效")?;let interpolation=match fields[2]{"linear"=>AutomationInterpolation::Linear,"step"=>AutomationInterpolation::Step,_=>return Err("插值选择 linear 或 step".into())};if tick<0||previous.is_some_and(|p|tick<=p)||!lane.target.accepts_value(value){return Err("tick 须严格递增且不重复，数值须在此目标的范围内".into());}let mut point=lane.points.iter().find(|p|p.tick==tick).cloned().unwrap_or_else(||AutomationPoint::new(tick,value));point.value=value;point.interpolation=interpolation;points.push(point);previous=Some(tick);}
    lane.points=points;Ok(())
}
fn parse_vocaloid_voice(text:&str)->Result<verdant_exchange::VocaloidVoice,String>{
    let fields=text.split('|').map(str::trim).collect::<Vec<_>>();if fields.len()!=4{return Err("使用 compID | 歌手名称 | 语言 | 程序 格式".into());}let language_id=fields[2].parse::<u8>().map_err(|_|"语言需要 0..127")?;let program=fields[3].parse::<u8>().map_err(|_|"程序需要 0..127")?;if fields[0].len()!=16||!fields[0].is_ascii()||fields[0].chars().any(char::is_control)||fields[1].is_empty()||language_id>127||program>127{return Err("compID 须为真实的 16 个 ASCII 字符，名称非空，语言和程序 0..127".into());}Ok(verdant_exchange::VocaloidVoice{comp_id:fields[0].into(),name:fields[1].into(),language_id,program})
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn human_phrase_dictionary_is_resolved_from_the_saved_undoable_document(){
        use verdant_backend::phonemizer::{self,PhonemizerSettings,PhraseEntry};
        let mut project=Project::new_empty();for(index,lyric)in ["重","行"].into_iter().enumerate(){let mut note=Note::new(index as i64*960,960,60);note.lyric=lyric.into();project.tracks[0].notes.push(note);}
        let mut settings=PhonemizerSettings{selected:phonemizer::EXPLICIT_ONLY.into(),..Default::default()};project.extensions.insert(phonemizer::EXTENSION_KEY.into(),phonemizer::settings_to_extension(&settings).unwrap());
        let mut store=ProjectStore::new(project);let(lyrics,readings)=dictionary_entry("重/zhong4 行/hang2").unwrap();settings.dictionary.phrases.insert(lyrics.concat(),PhraseEntry{lyrics,readings,..Default::default()});
        store.apply(store.revision(),CommandBatch::single("用户词组",Command::SetProjectExtension{key:phonemizer::EXTENSION_KEY.into(),value:Some(phonemizer::settings_to_extension(&settings).unwrap())})).unwrap();
        let persisted=serde_json::to_string(store.project()).unwrap();let restored:Project=serde_json::from_str(&persisted).unwrap();let settings=phonemizer::settings_from_project(&restored).unwrap();let resolved=phonemizer::resolve_track(&restored.tracks[0],&settings);
        assert!(resolved.diagnostics.is_empty());assert_eq!(normalize_pinyin(&resolved.notes[&restored.tracks[0].notes[0].id].pronunciation.pinyin),normalize_pinyin("zhong4"));assert_eq!(normalize_pinyin(&resolved.notes[&restored.tracks[0].notes[1].id].pronunciation.pinyin),normalize_pinyin("hang2"));assert_eq!(store.undo_depth(),1);
        store.undo().unwrap();let settings=phonemizer::settings_from_project(store.project()).unwrap();let unresolved=phonemizer::resolve_track(&store.project().tracks[0],&settings);assert!(!unresolved.diagnostics.is_empty());assert!(unresolved.notes.values().all(|note|normalize_pinyin(&note.pronunciation.pinyin)!=normalize_pinyin("zhong4")));
    }
}
