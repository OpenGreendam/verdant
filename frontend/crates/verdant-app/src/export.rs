//! Desktop controls for the shared headless WAV export service.
use std::path::{Path, PathBuf};
use gpui::{AppContext, Context, Window};
use verdant_backend::Severity;
use verdant_domain::TrackId;
use verdant_service::audio::{RenderSettings, render_outputs};
use verdant_service::project_audio::PhraseSource;
use crate::editor::{Editor, MenuAction, WorkflowAction};

pub use verdant_service::audio::{AudioScope, ExportOptions, ExportRange, WavEncoding};

#[derive(Clone, Debug)]
pub enum ExportSetting {
    Rate(u32), Channels(u16), Encoding(WavEncoding), Scope(AudioScope),
    Range(ExportRange), Effects, Start,
}

impl Editor {
    pub fn export_settings_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use crate::menu::MenuItem as I;
        let item = |label: String, setting| I::action(label, MenuAction::Workflow(WorkflowAction::ExportSetting(setting)));
        let mut items = vec![I::disabled(format!("当前：{} Hz · {} 声道 · {:?} · {:?} · {:?}", self.export_options.sample_rate, self.export_options.channels, self.export_options.encoding, self.export_options.scope, self.export_options.range))];
        for rate in [44_100, 48_000, 88_200, 96_000, 192_000] { items.push(item(format!("采样率 {rate} Hz"), ExportSetting::Rate(rate))); }
        for channels in [1, 2] { items.push(item(format!("{channels} 声道"), ExportSetting::Channels(channels))); }
        for encoding in [WavEncoding::Float32, WavEncoding::Pcm16, WavEncoding::Pcm24] { items.push(item(format!("WAV {encoding:?}"), ExportSetting::Encoding(encoding))); }
        for (scope, label) in [(AudioScope::Mix, "完整混音"), (AudioScope::ActiveTrack, "当前轨道"), (AudioScope::Stems, "逐轨文件（同一工程时钟）")] { items.push(item(label.into(), ExportSetting::Scope(scope))); }
        for (range, label) in [(ExportRange::Full, "工程、各轨与主总线限定尾音"), (ExportRange::Loop, "循环范围"), (ExportRange::Selection, "选中音符的时间范围")] { items.push(item(label.into(), ExportSetting::Range(range))); }
        items.push(item(format!("{}插入效果", if self.export_options.effects { "关闭" } else { "启用" }), ExportSetting::Effects));
        items.push(I::boolean("包含节拍器", crate::menu::MenuBoolean::ExportClick, self.playback_settings().export_click));
        items.push(item("选择路径并导出 WAV…".into(), ExportSetting::Start));
        self.show_menu(window.mouse_position(), items, window, cx);
    }

    pub fn apply_export_setting(&mut self, setting: ExportSetting, window: &mut Window, cx: &mut Context<Self>) {
        match setting {
            ExportSetting::Rate(rate) => self.export_options.sample_rate = rate,
            ExportSetting::Channels(channels) => self.export_options.channels = channels,
            ExportSetting::Encoding(encoding) => self.export_options.encoding = encoding,
            ExportSetting::Scope(scope) => self.export_options.scope = scope,
            ExportSetting::Range(range) => self.export_options.range = range,
            ExportSetting::Effects => self.export_options.effects = !self.export_options.effects,
            ExportSetting::Start => { crate::files::export_audio_dialog(self, window, cx); return; }
        }
        self.prefs.extra.insert("audio_export_options".into(), serde_json::to_value(&self.export_options).expect("export settings"));
        self.prefs_dirty = true;
        self.export_settings_menu(window, cx);
    }

    pub fn export_audio(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.export_busy { self.notice("已有离线导出正在进行"); cx.notify(); return; }
        if path == self.persistence.project_path() || same_file(&path, self.persistence.project_path()) { self.notice("导出路径不能覆盖当前工程"); cx.notify(); return; }
        if !self.commit_pending_lyric(cx) || !self.flush_insert_states(cx) { return; }
        self.replan(cx);
        let relevant = |id: &TrackId| self.export_options.scope != AudioScope::ActiveTrack || id == &self.session.track;
        if self.plans.iter().filter(|p| relevant(&p.track)).flat_map(|p| &p.diagnostics).any(|d| d.severity == Severity::Error) { self.notice("请先解决导出轨道的渲染预检错误"); cx.notify(); return; }
        let wanted = self.scheduler.wanted().iter().filter(|w| relevant(&w.phrase.track)).count();
        let ready = self.scheduler.playable_phrases().into_iter().filter(|(p, _)| relevant(&p.track)).count();
        if ready < wanted { self.notice("导出轨道仍有乐句等待渲染或制品校验"); cx.notify(); return; }
        let range_ticks = match self.export_options.range {
            ExportRange::Full => None,
            ExportRange::Loop => {
                let s = self.playback_settings();
                if s.loop_end <= s.loop_start { self.notice("先设置有效的循环范围"); cx.notify(); return; }
                Some((s.loop_start, s.loop_end))
            }
            ExportRange::Selection => {
                let notes = self.selected_notes();
                let Some(start) = notes.iter().map(|n| n.start_tick).min() else { self.notice("先选择音符以确定导出时间范围"); cx.notify(); return; };
                Some((start, notes.iter().map(|n| n.end_tick()).max().unwrap()))
            }
        };
        let media_policy = match self.media_asset_policy() {
            Ok(policy) => policy,
            Err(error) => { self.notice(format!("离线导出失败：{error}")); cx.notify(); return; }
        };
        let settings = RenderSettings { options: self.export_options.clone(), active_track: self.session.track.clone(), range_ticks, media_policy };
        let project = self.store().snapshot();
        let revision = self.revision();
        let sources = self.scheduler.playable_phrases().into_iter().map(|(phrase, entry)| PhraseSource {
            track: phrase.track.clone(), start_seconds: phrase.start_seconds, note_ids: phrase.note_ids.clone(),
            phoneme_timing: phrase.score.phoneme_timing.clone(), audio: entry.audio.clone(), playback_key: None,
        }).collect::<Vec<_>>();
        let plugins = self.plugins.clone();
        let rate = settings.options.sample_rate;
        self.export_busy = true;
        self.notice(format!("正在导出 {revision} · {rate} Hz · 与播放共用混音和自动化引擎"));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let destination = path.clone();
            let result = cx.background_spawn(async move { render_outputs(&destination, &project, sources, plugins.as_deref(), &settings, true, &|| false) }).await;
            let _ = this.update(cx, |ed, cx| {
                ed.export_busy = false;
                match result {
                    Ok(output) => ed.notice(format!("已导出 {revision} · {:.2} 秒 · {}", output.frames as f64 / rate as f64, path.display())),
                    Err(error) => ed.notice(format!("离线导出失败：{error}")),
                }
                cx.notify();
            });
        }).detach();
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) { (Ok(a), Ok(b)) => a == b, _ => false }
}
