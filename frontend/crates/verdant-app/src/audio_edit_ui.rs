//! Audio editing controls use the same serializable service operations as the
//! command line. Selection and drag previews are transient; every edit is one
//! ordinary journal transaction and all media analysis runs off the UI thread.
use std::{collections::BTreeSet,path::PathBuf};
use gpui::{Context,IntoElement,SharedString,Window,div,px,prelude::*};
use verdant_domain::{Revision,TrackId};
use verdant_media::{AudioClip,FadeShape};
use verdant_service::audio_edit::{AudioClipboard,AudioClipProperties,AudioClipRef,AudioEdit,AudioEditPlan};
use verdant_session::PointerId;
use crate::editor::{Editor,MenuAction,WorkflowAction};
use super::{IntegrationAction,IntegrationPrompt};

#[derive(Clone,Debug)]
pub struct AudioRange { pub tracks:Vec<TrackId>,pub start_tick:f64,pub end_tick:f64 }
#[derive(Clone,Debug)]
pub struct RangeDrag { pub pointer:PointerId,pub anchor_tick:f64,pub anchor_track:usize }
#[derive(Default)]
pub struct AudioEditUiState {
    pub selected:BTreeSet<AudioClipRef>,pub range:Option<AudioRange>,pub range_tool:bool,
    pub range_drag:Option<RangeDrag>,pub clipboard:Option<AudioClipboard>,pub busy:bool,
}
#[derive(Clone,Copy,Debug)]
pub enum AudioUiAction {
    Menu,SelectTool,RangeTool,Range,Split,SplitRange,TrimRange,Join,Duplicate,
    Copy,Cut,Paste,Delete,RippleDelete,SelectAll,Rename,Gain,Pan,Fades,Speed,TrimSource,Slip,
    ToggleMute,ToggleReverse,TogglePreservePitch,Crossfade,SplitSilence,SplitZeroCrossing,Normalize,Bounce,
}
#[derive(Clone,Debug)]
pub enum AudioPrompt {
    Rename(Vec<AudioClipRef>),Gain(Vec<AudioClipRef>),Pan(Vec<AudioClipRef>),Fades(Vec<AudioClipRef>),
    Speed(Vec<AudioClipRef>),Trim(AudioClipRef),Slip(Vec<AudioClipRef>),Silence(Vec<AudioClipRef>),
    Normalize(Vec<AudioClipRef>),Range,
}
fn item(label:impl Into<SharedString>,action:AudioUiAction)->crate::menu::MenuItem{
    crate::menu::MenuItem::action(label,MenuAction::Workflow(WorkflowAction::Integration(IntegrationAction::AudioEdit(action))))
}
pub fn toolbar(ed:&Editor,cx:&mut Context<Editor>)->impl IntoElement{
    let th=ed.theme;
    let selection=ed.audio_selection();
    let detail=if ed.integrations.audio_edit.busy{"正在准备音频编辑…".into()}
        else if let Some(range)=&ed.integrations.audio_edit.range{
            let tempo=ed.tempo();format!("{:.3}–{:.3} s · {} 轨 · {} 个片段",tempo.seconds_at(range.start_tick),tempo.seconds_at(range.end_tick),range.tracks.len(),selection.len())
        }else if let Ok(clip)=ed.selected_audio_clip(){
            let name=if clip.name.is_empty(){std::path::Path::new(&clip.source.path).file_name().unwrap_or_default().to_string_lossy().into_owned()}else{clip.name.clone()};
            format!("{} · {:.3} s · {:+.1} dB · {:.3}×{}{}",name,clip.duration_seconds(),20.0*clip.gain.max(1e-6).log10(),clip.playback_rate,if clip.preserve_pitch{" · 保持音高"}else{""},if clip.reversed{" · 反向"}else{""})
        }else{"导入音频后可拖动边缘裁剪；Shift 多选，Alt 暂停吸附".into()};
    div().id("audio-edit-toolbar").h(px(34.0)).flex_none().flex().items_center().gap(px(5.0)).px(px(8.0))
        .bg(th.surface).border_b_1().border_color(th.shade).text_sm()
        .children([
            ("选择","音频选择工具 · Shift 点击多选",AudioUiAction::SelectTool,!ed.integrations.audio_edit.range_tool),
            ("范围","拖动选择时间与轨道范围",AudioUiAction::RangeTool,ed.integrations.audio_edit.range_tool),
            ("分割","在播放头或选区边界切分",AudioUiAction::Split,false),
            ("片段 ▾","剪切、淡化、变速与合并",AudioUiAction::Menu,false),
        ].into_iter().map(|(label,description,action,selected)|crate::proto_controls::action(SharedString::from(format!("audio-tool-{label}")),description)
            .authority(format!("{}:{}:{}",ed.project().project_id,ed.document_generation,ed.revision()))
            .when(matches!(action,AudioUiAction::SelectTool|AudioUiAction::RangeTool),|button|button.aria_selected(selected))
            .h(px(26.0)).px(px(9.0)).flex().items_center().rounded(px(6.0)).cursor_pointer()
            .bg(if selected{th.control}else{th.surface}).hover(move|style|style.bg(th.hover))
            .child(label).on_click(cx.listener(move|ed,_,window,cx|{
                crate::proto_editor_surface::focus_commands("audio",window,cx);if let Err(error)=ed.run_audio_ui(action,window,cx){ed.notice(error);cx.notify();}
            }))))
        .child(div().flex_1().min_w_0().truncate().text_xs().text_color(th.text_secondary).child(detail))
}
/// Application command effects admitted by the compiled arrangement surface.
/// Native code does not interpret keys or retire pointer captures here.
pub fn source_command(ed:&mut Editor,command:&str,window:&mut Window,cx:&mut Context<Editor>){
    let action=match command{
        "delete"=>AudioUiAction::Delete,
        "ripple-delete"=>AudioUiAction::RippleDelete,
        "split"=>AudioUiAction::Split,
        "select-all"=>AudioUiAction::SelectAll,
        "join"=>AudioUiAction::Join,
        "range-tool"=>AudioUiAction::RangeTool,
        "select-tool"=>AudioUiAction::SelectTool,
        "clear-range"=>{ed.integrations.audio_edit.range=None;ed.integrations.audio_edit.range_drag=None;ed.arrange_drag=None;AudioUiAction::SelectTool},
        _=>return,
    };
    if let Err(error)=ed.run_audio_ui(action,window,cx){ed.notice(error);cx.notify();}
}

impl Editor{
    pub fn audio_selection(&self)->Vec<AudioClipRef>{
        let mut selected=self.integrations.audio_edit.selected.iter().filter(|reference|
            self.project().track(&reference.track).and_then(|track|verdant_media::read_clips(track).ok())
                .is_some_and(|clips|clips.iter().any(|clip|clip.id==reference.id))).cloned().collect::<Vec<_>>();
        if selected.is_empty(){if let Ok(clip)=self.selected_audio_clip(){selected.push(AudioClipRef::from(&clip));}}
        selected
    }
    pub fn select_audio_clip(&mut self,clip:&AudioClip,toggle:bool,preserve_group:bool){
        let reference=AudioClipRef::from(clip);
        let state=&mut self.integrations.audio_edit;state.range=None;
        if toggle{
            if !state.selected.remove(&reference){state.selected.insert(reference.clone());}
        }else if !preserve_group||!state.selected.contains(&reference){state.selected.clear();state.selected.insert(reference.clone());}
        self.integrations.selected_clip=if state.selected.contains(&reference){Some((clip.track.clone(),clip.id.to_string()))}
            else{state.selected.iter().next().map(|reference|(reference.track.clone(),reference.id.to_string()))};
    }
    pub fn select_audio_range(&mut self,range:AudioRange){
        let tempo=self.tempo();let mut selected=BTreeSet::new();
        for track in &self.project().tracks{if range.tracks.contains(&track.id){
            if let Ok(clips)=verdant_media::read_clips(track){for clip in clips{
                if clip.tick()<range.end_tick&&clip.end_tick(&tempo)>range.start_tick{selected.insert(AudioClipRef::from(&clip));}
            }}
        }}
        self.integrations.selected_clip=selected.iter().next().map(|clip|(clip.track.clone(),clip.id.to_string()));
        self.integrations.audio_edit.selected=selected;self.integrations.audio_edit.range=Some(range);
    }
    fn receive_audio_plan(&mut self,plan:AudioEditPlan,base:Revision,cx:&mut Context<Self>)->Result<(),String>{
        if base!=self.revision(){return Err("工程已修改，请重新执行音频编辑".into());}
        let changed=!plan.batch.commands.is_empty();
        if changed&&!self.commit(base,plan.batch,cx){return Err("音频片段编辑未提交".into());}
        if let Some(clipboard)=plan.clipboard{self.integrations.audio_edit.clipboard=Some(clipboard);}
        self.integrations.audio_edit.selected=plan.selection.into_iter().collect();
        self.integrations.selected_clip=self.integrations.audio_edit.selected.iter().next().map(|clip|(clip.track.clone(),clip.id.to_string()));
        if changed{self.integrations.audio_edit.range=None;self.rebuild_timeline(cx);}
        cx.notify();Ok(())
    }
    pub fn apply_audio_edit(&mut self,edit:AudioEdit,cx:&mut Context<Self>)->Result<(),String>{
        if self.integrations.audio_edit.busy{return Err("等待当前音频处理完成".into());}
        let base=self.revision();let plan=verdant_service::audio_edit::plan_audio_edit(self.project(),&edit)?;
        self.receive_audio_plan(plan,base,cx)
    }
    fn start_audio_media_edit(&mut self,edit:AudioEdit,cx:&mut Context<Self>)->Result<(),String>{
        if self.integrations.audio_edit.busy{return Err("已有音频处理正在进行".into());}
        let mut policy=self.media_asset_policy()?;
        // This is the application's own previously verified clipboard. Its
        // file grants survive switching projects, while Paste still verifies
        // every fingerprint again before introducing a source into the target.
        let copied_paths=if let AudioEdit::Paste{clipboard,..}=&edit{
            clipboard.clips.iter().map(|item|PathBuf::from(&item.clip.source.path)).collect::<Vec<_>>()
        }else{Vec::new()};
        for path in &copied_paths{policy.allow_file(path).map_err(|e|e.to_string())?;}
        let project=self.store().snapshot();let base=self.revision();let project_id=self.project().project_id.clone();let document_generation=self.document_generation;
        self.integrations.audio_edit.busy=true;cx.notify();
        cx.spawn(async move|this,cx|{
            let result=cx.background_spawn(async move{
                let plan=verdant_service::audio_edit::plan_audio_edit_with_media(&project,&edit,policy)?;
                let staged=verdant_service::audio_edit::stage_audio_assets(&plan)?;
                Ok::<_,String>((plan,staged))
            }).await;
            let _=this.update(cx,|ed,cx|{
                if ed.document_generation==document_generation{ed.integrations.audio_edit.busy=false;}
                let mut published=Vec::new();
                let outcome=(||->Result<(),String>{
                    let(plan,mut staged)=result?;
                    if ed.document_generation!=document_generation||ed.project().project_id!=project_id||ed.revision()!=base{
                        return Err("工程已修改，后台音频编辑未应用；临时音频已清理".into());
                    }
                    // No await may separate this final revision check from
                    // publication and journal commit. PCM write/hash/fsync
                    // already completed off-thread; only hard links remain.
                    let publication=staged.publish();
                    published=staged.published_paths().to_vec();
                    let paths=publication?;
                    let grants=paths.iter().chain(&copied_paths).cloned().collect::<Vec<_>>();
                    for path in &grants{ed.integrations.asset_policy.allow_file(path).map_err(|e|e.to_string())?;}
                    ed.remember_media_authorizations(&grants);ed.receive_audio_plan(plan,base,cx)
                })();
                match outcome{
                    Ok(())=>ed.notice("音频编辑已完成，可撤销"),
                    Err(error) if published.is_empty()=>ed.notice(error),
                    Err(error)=>ed.notice(format!("{error}；音频文件已保留：{}。请检查工程历史后导入该文件，或另选路径重试。",
                        published.iter().map(|path|path.display().to_string()).collect::<Vec<_>>().join("、"))),
                }
                cx.notify();
            });
        }).detach();Ok(())
    }
    pub fn audio_menu_items(&self)->Vec<crate::menu::MenuItem>{
        use AudioUiAction as A;use crate::menu::MenuItem as I;
        let count=self.audio_selection().len();
        let mut items=vec![I::header(format!("音频片段 · 已选 {count} 个")),
            item("复制 · Ctrl/Cmd+C",A::Copy),item("剪切 · Ctrl/Cmd+X",A::Cut),item("粘贴到播放头 · Ctrl/Cmd+V",A::Paste),item("复制到播放头",A::Duplicate),
            I::separator(),item("在播放头 / 选区两端分割 · Ctrl/Cmd+I",A::Split),item("按选区裁剪",A::TrimRange),item("连接选中相邻片段 · Ctrl/Cmd+J",A::Join),item("交叉渐变（等功率）",A::Crossfade),
            item("选区起止时间…",A::Range),item("无损源范围 / 位置…",A::TrimSource),item("滑移源内容…",A::Slip),I::separator(),
            item("片段名称…",A::Rename),item("增益（dB）…",A::Gain),item("声像…",A::Pan),item("淡入 / 淡出时长…",A::Fades),item("速度 / 时长…",A::Speed),
            item("切换保持音高",A::TogglePreservePitch),item("反向播放",A::ToggleReverse),item("切换静音",A::ToggleMute),I::separator(),
            item("按静音切片…",A::SplitSilence),item("在播放头附近过零点分割",A::SplitZeroCrossing),item("峰值标准化…",A::Normalize),item("合并为新音频文件…",A::Bounce),I::separator(),
            item("删除（保留空隙） · Delete",A::Delete),item("波纹删除（关闭空隙） · Shift+Delete",A::RippleDelete),item("选择全部音频片段",A::SelectAll)];
        if self.integrations.audio_edit.range.is_some(){items.insert(1,item("在选区两端切分",A::SplitRange));}items
    }
    pub fn run_audio_ui(&mut self,action:AudioUiAction,window:&mut Window,cx:&mut Context<Self>)->Result<(),String>{
        use AudioUiAction as A;
        let clips=self.audio_selection();let tick=self.tempo().tick_at(self.playhead_seconds());let range=self.integrations.audio_edit.range.clone();
        let selected=||if clips.is_empty(){Err("先选择音频片段".to_string())}else{Ok(clips.clone())};
        match action{
            A::Menu=>self.show_menu(window.mouse_position(),self.audio_menu_items(),window,cx),
            A::SelectTool=>{self.integrations.audio_edit.range_tool=false;cx.notify();},
            A::RangeTool=>{self.integrations.audio_edit.range_tool=true;cx.notify();},
            A::SelectAll=>{let tempo=self.tempo();let end=self.project().tracks.iter().filter_map(|track|verdant_media::read_clips(track).ok()).flatten().map(|clip|clip.end_tick(&tempo)).fold(0.0_f64,f64::max);self.select_audio_range(AudioRange{tracks:self.project().tracks.iter().map(|track|track.id.clone()).collect(),start_tick:0.0,end_tick:end});cx.notify();},
            A::Range=>{let value=range.map(|range|format!("{:.6}, {:.6}",self.tempo().seconds_at(range.start_tick),self.tempo().seconds_at(range.end_tick))).unwrap_or_else(||format!("{:.6}, {:.6}",self.playhead_seconds(),self.playhead_seconds()+1.0));self.prompt_integration("音频选区：起始秒, 结束秒",IntegrationPrompt::AudioEdit(AudioPrompt::Range),&value,window,cx);},
            A::Split|A::SplitRange=>{let edit=if let Some(range)=range{AudioEdit::SplitRange{tracks:range.tracks,start_tick:range.start_tick,end_tick:range.end_tick}}else{AudioEdit::Split{clips:selected()?,tick}};self.apply_audio_edit(edit,cx)?;},
            A::TrimRange=>{let range=range.ok_or("先用范围工具拖出选区")?;self.apply_audio_edit(AudioEdit::TrimToRange{clips:selected()?,start_tick:range.start_tick,end_tick:range.end_tick},cx)?;},
            A::Copy|A::Cut=>{
                let copy=if let Some(range)=&range{AudioEdit::CopyRange{tracks:range.tracks.clone(),start_tick:range.start_tick,end_tick:range.end_tick}}else{AudioEdit::Copy{clips:selected()?}};
                let edit=if matches!(action,A::Cut){let delete=if let Some(range)=range{AudioEdit::DeleteRange{tracks:range.tracks,start_tick:range.start_tick,end_tick:range.end_tick,ripple:false}}else{AudioEdit::Delete{clips:selected()?,ripple:false}};AudioEdit::Batch{edits:vec![copy,delete]}}else{copy};self.start_audio_media_edit(edit,cx)?;
            }
            A::Paste=>{let clipboard=self.integrations.audio_edit.clipboard.clone().ok_or("音频剪贴板为空")?;self.start_audio_media_edit(AudioEdit::Paste{clipboard,at_tick:tick,target_track:self.session.track.clone()},cx)?;},
            A::Delete|A::RippleDelete=>{let ripple=matches!(action,A::RippleDelete);let edit=if let Some(range)=range{AudioEdit::DeleteRange{tracks:range.tracks,start_tick:range.start_tick,end_tick:range.end_tick,ripple}}else{AudioEdit::Delete{clips:selected()?,ripple}};self.apply_audio_edit(edit,cx)?;},
            A::Duplicate=>self.apply_audio_edit(AudioEdit::Duplicate{clips:selected()?,at_tick:tick,target_track:None},cx)?,
            A::Join=>{
                let mut refs=selected()?;
                if refs.len()==1{let clip=self.selected_audio_clip()?;let mut others=verdant_media::read_clips(self.project().track(&clip.track).ok_or("轨道不存在")?).map_err(|e|e.to_string())?;others.sort_by(|a,b|a.tick().total_cmp(&b.tick()));let right=others.iter().find(|other|other.id!=clip.id&&other.tick()>=clip.tick()).ok_or("右侧没有相邻音频片段")?;refs.push(AudioClipRef::from(right));}
                self.apply_audio_edit(AudioEdit::Join{clips:refs},cx)?;
            }
            A::Crossfade=>{let refs=selected()?;if refs.len()!=2{return Err("Shift 点击选择两个重叠片段".into());}let mut pair=refs.iter().map(|reference|verdant_media::read_clips(self.project().track(&reference.track).ok_or("轨道不存在")?).map_err(|e|e.to_string())?.into_iter().find(|clip|clip.id==reference.id).ok_or("音频片段不存在".into())).collect::<Result<Vec<_>,String>>()?;pair.sort_by(|a,b|a.tick().total_cmp(&b.tick()));self.apply_audio_edit(AudioEdit::Crossfade{left:AudioClipRef::from(&pair[0]),right:AudioClipRef::from(&pair[1]),shape:FadeShape::EqualPower},cx)?;},
            A::ToggleMute|A::ToggleReverse|A::TogglePreservePitch=>{let clip=self.selected_audio_clip()?;let mut properties=AudioClipProperties::default();match action{A::ToggleMute=>properties.muted=Some(!clip.muted),A::ToggleReverse=>properties.reversed=Some(!clip.reversed),_=>properties.preserve_pitch=Some(!clip.preserve_pitch)}self.apply_audio_edit(AudioEdit::SetProperties{clips:selected()?,properties},cx)?;},
            A::SplitZeroCrossing=>{let clip=self.selected_audio_clip()?;self.start_audio_media_edit(AudioEdit::SplitZeroCrossing{clip:AudioClipRef::from(&clip),tick,window_ms:10.0},cx)?;},
            A::Bounce=>{let clips=selected()?;let rate=self.audio_info.sample_rate;let rx=cx.prompt_for_new_path(&PathBuf::from("."),Some("bounce.wav"));cx.spawn(async move|this,cx|{if let Ok(Ok(Some(path)))=rx.await{let _=this.update(cx,|ed,cx|{if let Err(error)=ed.start_audio_media_edit(AudioEdit::Bounce{clips,path:path.to_string_lossy().into_owned(),sample_rate:rate,replace:true},cx){ed.notice(error);cx.notify();}});}}).detach();},
            A::Rename|A::Gain|A::Pan|A::Fades|A::Speed|A::TrimSource|A::Slip|A::SplitSilence|A::Normalize=>{
                let refs=selected()?;let clip=self.selected_audio_clip()?;
                let(title,prompt,value)=match action{
                    A::Rename=>("音频片段名称",AudioPrompt::Rename(refs),clip.name.clone()),
                    A::Gain=>("片段增益（dB，-120 至 +36）",AudioPrompt::Gain(refs),format!("{:.2}",20.0*clip.gain.max(1e-6).log10())),
                    A::Pan=>("片段声像（-1 左，0 中，+1 右）",AudioPrompt::Pan(refs),format!("{:.3}",clip.pan)),
                    A::Fades=>("淡入毫秒, 淡出毫秒",AudioPrompt::Fades(refs),format!("{:.3}, {:.3}",clip.fade_in_frames as f64/clip.source.sample_rate as f64/clip.playback_rate*1000.0,clip.fade_out_frames as f64/clip.source.sample_rate as f64/clip.playback_rate*1000.0)),
                    A::Speed=>("播放速度（0.0625–16×；保持音高由片段菜单切换）",AudioPrompt::Speed(refs),format!("{:.6}",clip.playback_rate)),
                    A::TrimSource=>("源起始帧, 源时长帧, 放置 tick（可恢复隐藏源内容）",AudioPrompt::Trim(AudioClipRef::from(&clip)),format!("{}, {}, {:.6}",clip.source_offset_frames,clip.duration_frames,clip.tick())),
                    A::Slip=>("滑移源内容的帧数（正数向后，负数向前）",AudioPrompt::Slip(refs),"0".into()),
                    A::SplitSilence=>("静音阈值 dB, 最短静音毫秒",AudioPrompt::Silence(refs),"-48, 150".into()),
                    _=>("标准化目标峰值（dBFS，-120 至 0）",AudioPrompt::Normalize(refs),"-1".into()),
                };self.prompt_integration(title,IntegrationPrompt::AudioEdit(prompt),&value,window,cx);
            }
        }Ok(())
    }
    pub fn apply_audio_prompt(&mut self,prompt:&AudioPrompt,text:&str,cx:&mut Context<Self>)->Result<(),String>{
        let fields=|count:usize|->Result<Vec<f64>,String>{let values=text.split([',','，']).map(str::trim).map(|value|value.parse::<f64>().map_err(|_|"需要数值".to_string())).collect::<Result<Vec<_>,_>>()?;if values.len()!=count||values.iter().any(|value|!value.is_finite()){return Err(format!("需要 {count} 个有限数值，以逗号分开"));}Ok(values)};
        let mut properties=AudioClipProperties::default();
        let clips=match prompt{
            AudioPrompt::Range=>{let values=fields(2)?;if values[0]<0.0||values[1]<=values[0]{return Err("选区终点必须大于非负起点".into());}let tracks=self.integrations.audio_edit.range.as_ref().map(|range|range.tracks.clone()).unwrap_or_else(||vec![self.session.track.clone()]);self.select_audio_range(AudioRange{tracks,start_tick:self.tempo().tick_at(values[0]),end_tick:self.tempo().tick_at(values[1])});cx.notify();return Ok(());},
            AudioPrompt::Trim(clip)=>{let parts=text.split([',','，']).map(str::trim).collect::<Vec<_>>();if parts.len()!=3{return Err("填写源起始帧, 源时长帧, 放置 tick".into());}let offset=parts[0].parse::<u64>().map_err(|_|"源起点须为非负整数")?;let duration=parts[1].parse::<u64>().map_err(|_|"源时长须为正整数")?;let tick=parts[2].parse::<f64>().map_err(|_|"放置 tick 无效")?;return self.apply_audio_edit(AudioEdit::Trim{clip:clip.clone(),source_offset_frames:offset,duration_frames:duration,start_tick:Some(tick)},cx);},
            AudioPrompt::Slip(clips)=>{let delta=text.trim().parse::<i64>().map_err(|_|"滑移帧数须为整数")?;return self.apply_audio_edit(AudioEdit::Slip{clips:clips.clone(),delta_frames:delta},cx);},
            AudioPrompt::Silence(clips)=>{let values=fields(2)?;return self.start_audio_media_edit(AudioEdit::SplitSilence{clips:clips.clone(),threshold_db:values[0],min_silence_ms:values[1]},cx);},
            AudioPrompt::Normalize(clips)=>{return self.start_audio_media_edit(AudioEdit::Normalize{clips:clips.clone(),target_db:fields(1)?[0],independent:false},cx);},
            AudioPrompt::Rename(clips)=>{properties.name=Some(text.trim().into());clips},
            AudioPrompt::Gain(clips)=>{properties.gain_db=Some(fields(1)?[0]);clips},
            AudioPrompt::Pan(clips)=>{properties.pan=Some(fields(1)?[0]as f32);clips},
            AudioPrompt::Fades(clips)=>{let values=fields(2)?;properties.fade_in_ms=Some(values[0]);properties.fade_out_ms=Some(values[1]);clips},
            AudioPrompt::Speed(clips)=>{properties.playback_rate=Some(fields(1)?[0]);clips},
        };
        self.apply_audio_edit(AudioEdit::SetProperties{clips:clips.clone(),properties},cx)
    }
}
