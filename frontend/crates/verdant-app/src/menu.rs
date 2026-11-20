//! GPUI popup menus with native event dispatch, focus and AccessKit menu roles.
//! The File menu opens editable projects and supported score formats, reuses the
//! ordinary export track/metadata prompts, and reads recent files from prefs.
//! The pinned Linux SDK has no OS context-popup API; these are GPUI widgets.
//! Track headers share one menu for rename, ordering and duplication. Boundary
//! moves and capacity-limited actions remain visible but cannot be activated.
use gpui::{Context, FocusHandle, IntoElement, KeyDownEvent, Pixels, Point, SharedString, Window, div, prelude::*, px};
use crate::editor::{Editor, MenuAction};
use crate::theme::mix;
use std::path::{Path, PathBuf};
use verdant_exchange::Format;

/// File entries carry their format through to the existing import/export flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileAction {
    Open,
    Import(Format),
    Export(Format),
    Recent(PathBuf),
}

#[derive(Clone, Debug)]
pub enum MenuCommand {
    Editor(MenuAction),
    File(FileAction),
    Submenu(Vec<MenuItem>),
}

pub const SCORE_FORMATS: [(Format, &str); 7] = [
    (Format::Ust, "UST"), (Format::Ustx, "USTX"), (Format::Midi, "MIDI"),
    (Format::Ufdata, "UFDATA"),
    (Format::Svp, "SVP"), (Format::Vsqx, "VSQX"), (Format::Vpr, "VPR"),
];

/// The menu and open-file path share the established UST importer and exchange
/// parsers. This preserves UST encoding/reading admission and per-format PPQ.
pub(crate) fn import_score_bytes(format: Format, bytes: &[u8], source_name: Option<String>) -> Result<(verdant_domain::Project, serde_json::Value), String> {
    if let Some(format) = verdant_import::ImportFormat::from_extension(format.extension()) {
        let options = verdant_import::ImportOptions { source_name, ..Default::default() };
        let imported = verdant_import::import(format, bytes, &options).map_err(|e| e.to_string())?;
        Ok((imported.project, serde_json::to_value(imported.report).map_err(|e| e.to_string())?))
    } else {
        let options = verdant_exchange::ImportOptions { source_name, ..Default::default() };
        let imported = verdant_exchange::import(format, bytes, &options).map_err(|e| e.to_string())?;
        Ok((imported.project, serde_json::to_value(imported.report).map_err(|e| e.to_string())?))
    }
}

/// Resolve before opening a file, so an arbitrary suffix never reaches the
/// native-project decoder and produces a misleading JSON error.
pub(crate) fn file_format(path: &Path) -> Result<Option<Format>, String> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or_default();
    verdant_domain::io::reject_retired_project_format(extension).map_err(|error| error.to_string())?;
    if let Some(format) = Format::from_extension(extension) { return Ok(Some(format)); }
    if verdant_domain::io::has_project_extension(path) { return Ok(None); }
    Err(format!("不支持的文件扩展名：{}；请选择 VERP、UFDATA、UST、USTX、MID / MIDI、SVP、VSQX 或 VPR", path.display()))
}

pub(crate) fn recent_project_paths(prefs: &crate::files::Prefs) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    prefs.extra.get("recent_projects").and_then(|v| v.as_array()).into_iter().flatten()
        .filter_map(|v| v.as_str()).map(PathBuf::from)
        .filter(|p| crate::files::is_native_project_path(p) && seen.insert(p.clone())).take(20).collect()
}

pub(crate) fn remember_recent_file(prefs: &mut crate::files::Prefs, path: &Path) {
    if !crate::files::is_native_project_path(path) { return; }
    let mut paths = recent_project_paths(prefs);
    // Match the existing preference representation, including native paths
    // which cannot be serialized as UTF-8 by serde's Path implementation.
    let path = PathBuf::from(path.to_string_lossy().as_ref());
    paths.retain(|p| p != &path);
    paths.insert(0, path);
    paths.truncate(20);
    prefs.extra.insert("recent_projects".into(), serde_json::json!(paths));
}

pub(crate) fn file_menu_items(prefs: &crate::files::Prefs) -> Vec<MenuItem> {
    use crate::editor::WorkflowAction as W;
    use crate::editor::integrations::IntegrationAction as I;
    let mut items = vec![
        MenuItem::action("新建工程", MenuAction::Workflow(W::New)).with_hint("Ctrl+N"),
        MenuItem::action("内建示例 / 试听工程…", MenuAction::ChooseExample),
        MenuItem::file("打开工程（.verp）…", FileAction::Open).with_hint("Ctrl+O"),
        MenuItem::action("保存", MenuAction::Save).with_hint("Ctrl+S"),
        MenuItem::action("另存为…", MenuAction::SaveAs).with_hint("Ctrl+Shift+S"),
        MenuItem::separator(),
    ];
    items.push(MenuItem::submenu("导入", SCORE_FORMATS.into_iter().map(|(format, name)|
        MenuItem::file(format!("从 {name} 导入…"), FileAction::Import(format))).collect()));
    items.push(MenuItem::action("导入音频伴奏 / 参考音…", MenuAction::Workflow(W::Integration(I::ImportAudio))));
    items.push(MenuItem::action("导入用户词典文件…", MenuAction::Workflow(W::Integration(I::ImportDictionary))));
    items.push(MenuItem::separator());
    items.push(MenuItem::submenu("导出", SCORE_FORMATS.into_iter().map(|(format, name)|
        MenuItem::file(format!("导出成 {name}…"), FileAction::Export(format))).collect()));
    items.extend([
        MenuItem::action("导出用户词典文件…", MenuAction::Workflow(W::Integration(I::ExportDictionary))),
        MenuItem::action("Vocaloid 歌手元数据…", MenuAction::Workflow(W::Integration(I::VocaloidMetadata))),
        MenuItem::action("查看导入 / 导出报告…", MenuAction::Workflow(W::Integration(I::Report))),
        MenuItem::action("导出 Verdant 工程 (.verp)…", MenuAction::ExportProject).with_hint("Ctrl+Shift+E"),
        MenuItem::action("导出 WAV（含插件）…", MenuAction::Workflow(W::ExportSettings)).with_hint("Ctrl+Alt+E"),
        MenuItem::separator(),
        MenuItem::disabled("最近打开的工程（.verp）"),
    ]);
    let recent = recent_project_paths(prefs);
    if recent.is_empty() { items.push(MenuItem::disabled("尚无最近文件")); }
    for path in recent { items.push(MenuItem::file(path.display().to_string(), FileAction::Recent(path))); }
    items.extend([
        MenuItem::separator(),
        MenuItem::action("恢复工程…", MenuAction::Workflow(W::Recovery)).with_hint("Ctrl+Alt+O"),
        MenuItem::action("还原到手动保存版本", MenuAction::Workflow(W::Revert)),
    ]);
    items
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: SharedString,
    pub hint: SharedString,
    pub command: Option<MenuCommand>,
    pub enabled: bool,
}
impl MenuItem {
    pub fn action(label: impl Into<SharedString>, command: MenuAction) -> Self {
        Self { label: label.into(), hint: "".into(), command: Some(MenuCommand::Editor(command)), enabled: true }
    }
    pub fn file(label: impl Into<SharedString>, command: FileAction) -> Self {
        Self { label: label.into(), hint: "".into(), command: Some(MenuCommand::File(command)), enabled: true }
    }
    pub fn submenu(label: impl Into<SharedString>, items: Vec<MenuItem>) -> Self {
        Self { label: label.into(), hint: "›".into(), command: Some(MenuCommand::Submenu(items)), enabled: true }
    }
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self { self.hint = hint.into(); self }
    pub fn enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
    pub fn disabled(label: impl Into<SharedString>) -> Self { Self { label: label.into(), hint: "".into(), command: None, enabled: false } }
    pub fn separator() -> Self { Self::disabled("") }
}
#[derive(Clone, Debug)]
pub struct OpenMenu {
    pub anchor: Point<Pixels>,
    pub items: Vec<MenuItem>,
    pub selected: usize,
    pub previous_focus: Option<FocusHandle>,
    expanded: Option<usize>,
}

impl OpenMenu {
    fn rows(&self) -> impl Iterator<Item = (usize, bool, &MenuItem)> {
        self.items.iter().enumerate().flat_map(|(index, item)| {
            let children = match &item.command {
                Some(MenuCommand::Submenu(children)) if self.expanded == Some(index) => children.as_slice(),
                _ => &[],
            };
            std::iter::once((index, false, item)).chain(children.iter().map(move |child| (index, true, child)))
        })
    }
}

impl Editor {
    pub fn file_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_menu(window.mouse_position(), file_menu_items(&self.prefs), window, cx);
    }

    pub fn run_file_action(&mut self, action: FileAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            FileAction::Open => self.file_open_dialog(None, window, cx),
            FileAction::Import(format) => self.file_open_dialog(Some(format), window, cx),
            // Keep UST track choice, export options and real VSQX metadata admission.
            FileAction::Export(format) => crate::files::export_score_dialog(self, format, window, cx),
            FileAction::Recent(path) => self.open_project_path(path, window, cx),
        }
    }

    fn activate_menu_item(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.as_mut() else { return; };
        let Some((index, _, item)) = menu.rows().nth(row).filter(|(_, _, item)| item.enabled) else { return; };
        match &item.command {
            Some(MenuCommand::Submenu(_)) => {
                menu.expanded = (menu.expanded != Some(index)).then_some(index);
                menu.selected = index;
                cx.notify();
            }
            Some(MenuCommand::Editor(command)) => {
                let command = command.clone();
                self.close_menu(window, cx);
                self.run_menu_action(command, window, cx);
            }
            Some(MenuCommand::File(command)) => {
                let command = command.clone();
                self.close_menu(window, cx);
                self.run_file_action(command, window, cx);
            }
            None => {}
        }
    }

    fn file_open_dialog(&mut self, expected: Option<Format>, window: &mut Window, cx: &mut Context<Self>) {
        self.end_lyric(true, None, window, cx);
        let prompt = expected.map(|format| format!("导入 {} 乐谱", format.extension().to_uppercase()))
            .unwrap_or_else(|| "打开 Verdant 工程（.verp）".into());
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some(prompt.into()) });
        cx.spawn_in(window, async move |this, cx| {
            let picked = rx.await.map_err(|e| e.to_string()).and_then(|r| r.map_err(|e| e.to_string()));
            let _ = cx.update(|window, cx| this.update(cx, |ed, cx| {
                let result = (|| -> Result<(), String> {
                    if let Some(mut paths) = picked? {
                        if let Some(path) = paths.pop() {
                            let actual = file_format(&path)?;
                            if let Some(expected) = expected {
                                if actual != Some(expected) { return Err(format!("请选择 .{} 文件；所选文件为 {}", expected.extension(), path.display())); }
                                ed.import_project_path(path, window, cx);
                            } else {
                                ed.open_project_path(path, window, cx);
                            }
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = result { ed.notice(format!("打开失败：{error}")); cx.notify(); }
            }));
        }).detach();
    }


    pub fn show_menu(&mut self, anchor: Point<Pixels>, items: Vec<MenuItem>, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        let previous_focus = self.menu.as_ref().and_then(|m| m.previous_focus.clone()).or_else(|| window.focused(cx));
        let selected = items.iter().position(|i| i.enabled && i.command.is_some()).unwrap_or(0);
        self.menu = Some(OpenMenu { anchor, items, selected, previous_focus, expanded: None });
        window.focus(&self.focus_menu, cx);
        cx.notify();
    }

    pub fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.take() {
            if let Some(focus) = menu.previous_focus { window.focus(&focus, cx); }
            else { window.focus(&self.focus_roll, cx); }
        }
        cx.notify();
    }

    pub fn audition_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = audition_menu_items(&self.audition_projects);
        self.show_menu(window.mouse_position(), items, window, cx);
    }

    pub fn plugin_picker(&mut self, track: verdant_domain::TrackId, slot: usize, _anchor: Point<Pixels>, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_plugin_browser(track, slot, cx);
    }

    pub fn track_menu(&mut self, track: verdant_domain::TrackId, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let items = track_menu_items(self.project(), &track);
        self.show_menu(anchor, items, window, cx);
    }

    pub fn edit_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::editor::WorkflowAction as A;
        let tick = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
        let has_sel = !self.session.selection.is_empty();
        let items = vec![
            MenuItem::action("撤销", MenuAction::Undo).with_hint("Ctrl+Z").enabled(self.store().undo_depth() > 0),
            MenuItem::action("重做", MenuAction::Redo).with_hint("Ctrl+Shift+Z").enabled(self.store().redo_depth() > 0),
            MenuItem::action("打开撤销历史窗口…", MenuAction::ShowHistoryWindow).with_hint("Ctrl+Shift+H"),
            MenuItem::separator(),
            MenuItem::action("剪切音符", MenuAction::Cut).with_hint("Ctrl+X").enabled(has_sel),
            MenuItem::action("复制音符", MenuAction::Copy).with_hint("Ctrl+C").enabled(has_sel),
            MenuItem::action("粘贴音符", MenuAction::Paste).with_hint("Ctrl+V").enabled(!self.note_clipboard.is_empty()),
            MenuItem::action("重复一份选区", MenuAction::Duplicate).with_hint("Ctrl+D").enabled(has_sel),
            MenuItem::action("全选音符", MenuAction::SelectAll).with_hint("Ctrl+A"),
            MenuItem::action("删除选中音符", MenuAction::DeleteSelection).with_hint("Del").enabled(has_sel),
            MenuItem::separator(),
            MenuItem::action("在播放头分割音符", MenuAction::Workflow(A::Split(tick))).with_hint("Ctrl+Shift+J").enabled(has_sel),
            MenuItem::action("连接相邻选中音符", MenuAction::Workflow(A::Join)).with_hint("Ctrl+J").enabled(has_sel),
            MenuItem::action("量化到当前网格", MenuAction::Workflow(A::Quantize)).with_hint("Q").enabled(has_sel),
            MenuItem::action("闭合一格内间隙", MenuAction::Workflow(A::CloseGaps)).with_hint("Alt+Q").enabled(has_sel),
            MenuItem::action("消除音符重叠", MenuAction::Workflow(A::RemoveOverlaps)).with_hint("Shift+Q").enabled(has_sel),
            MenuItem::action("音符静音 / 取消静音", MenuAction::Workflow(A::MuteNotes)).enabled(has_sel),
            MenuItem::separator(),
            MenuItem::action("选择当前音符全部曲线点", MenuAction::Workflow(A::SelectPoints)).enabled(has_sel),
            MenuItem::action("曲线控制点批量变换…", MenuAction::Workflow(A::TransformPoints)),
            MenuItem::action("复制参数曲线", MenuAction::CopyCurve).enabled(has_sel),
            MenuItem::action("在播放头粘贴参数曲线", MenuAction::PasteCurveAt(tick)).enabled(self.curve_clipboard.is_some()),
        ];
        self.show_menu(anchor, items, window, cx);
    }

    pub fn track_part_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::editor::WorkflowAction as A;
        use crate::editor::integrations::IntegrationAction as I;
        let track = self.session.track.clone();
        let tick = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
        let auto_on = self.automation_ui.expanded_tracks.contains(&track);
        let mut items = track_menu_items(self.project(), &track);
        items.extend([
            MenuItem::separator(),
            MenuItem::action(
                if auto_on { "✓ 展开当前轨道自动化泳道" } else { "展开当前轨道自动化泳道" },
                MenuAction::ToggleTrackAutomation(track),
            ).with_hint("A 按钮"),
            MenuItem::action(
                if self.automation_ui.show_master { "✓ 显示主总线自动化泳道" } else { "显示主总线自动化泳道" },
                MenuAction::ToggleMasterAutomation,
            ),
            MenuItem::separator(),
            MenuItem::action("导入音频伴奏 / 参考音…", MenuAction::Workflow(A::Integration(I::ImportAudio))),
            MenuItem::action("音频片段列表…", MenuAction::Workflow(A::Integration(I::Clips))),
            MenuItem::action("精确编辑所选音频片段…", MenuAction::Workflow(A::Integration(I::EditClip)))
                .enabled(self.integrations.selected_clip.is_some()),
            MenuItem::action("重新定位丢失音源…", MenuAction::Workflow(A::Integration(I::RelinkClip)))
                .enabled(self.integrations.selected_clip.is_some()),
            MenuItem::action("授权重新打开已保存音频…", MenuAction::Workflow(A::Integration(I::AuthorizeAudio))),
            MenuItem::separator(),
            MenuItem::action("从选中音符创建片段 (Part)…", MenuAction::Workflow(A::CreatePart)),
            MenuItem::action("在播放头分割片段", MenuAction::Workflow(A::SplitPart(tick))).enabled(self.selected_part.is_some()),
            MenuItem::action("连接右侧片段", MenuAction::Workflow(A::JoinPart)).enabled(self.selected_part.is_some()),
            MenuItem::action("链接重复片段", MenuAction::Workflow(A::DuplicatePart(true))).enabled(self.selected_part.is_some()),
            MenuItem::action("独立复制片段", MenuAction::Workflow(A::DuplicatePart(false))).enabled(self.selected_part.is_some()),
            MenuItem::action("解除片段链接", MenuAction::Workflow(A::UnlinkPart)).enabled(self.selected_part.is_some()),
            MenuItem::action("删除所选片段", MenuAction::Workflow(A::DeletePart)).enabled(self.selected_part.is_some()),
        ]);
        if let Ok(parts) = verdant_domain::parts::Parts::load(self.project()) {
            if !parts.instances.is_empty() {
                items.push(MenuItem::separator());
                for p in parts.instances {
                    items.push(MenuItem::action(format!("定位片段 · {}", p.name), MenuAction::Workflow(A::SelectPart(p.id))));
                }
            }
        }
        self.show_menu(anchor, items, window, cx);
    }

    pub fn voice_lyrics_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::editor::WorkflowAction as A;
        use crate::editor::integrations::IntegrationAction as I;
        let has_sel = !self.session.selection.is_empty();
        let items = vec![
            MenuItem::action("选择当前轨道歌手…", MenuAction::Workflow(A::Integration(I::Singers))),
            MenuItem::action("导入 verdant-voice 声库…", MenuAction::Workflow(A::Integration(I::InstallVoice))),
            MenuItem::separator(),
            MenuItem::action("读音器与默认语言…", MenuAction::Workflow(A::Integration(I::Phonemizers))),
            MenuItem::action("添加多语言区间分段映射…", MenuAction::Workflow(A::Integration(I::AddSegment))),
            MenuItem::action("管理多语言区间分段列表…", MenuAction::Workflow(A::Integration(I::Segments))),
            MenuItem::action("工程用户读音词典…", MenuAction::Workflow(A::Integration(I::Dictionary))),
            MenuItem::action("导入用户词典文件…", MenuAction::Workflow(A::Integration(I::ImportDictionary))),
            MenuItem::action("导出用户词典文件…", MenuAction::Workflow(A::Integration(I::ExportDictionary))),
            MenuItem::separator(),
            MenuItem::action("批量填写歌词（按词/空格分配）…", MenuAction::Workflow(A::BatchLyrics(false))).with_hint("Ctrl+L").enabled(has_sel),
            MenuItem::action("逐字填写歌词（每字一音）…", MenuAction::Workflow(A::BatchLyrics(true))).enabled(has_sel),
            MenuItem::action("歌词循环左移", MenuAction::Workflow(A::ShiftLyrics(false))).enabled(self.session.selection.len() >= 2),
            MenuItem::action("歌词循环右移", MenuAction::Workflow(A::ShiftLyrics(true))).enabled(self.session.selection.len() >= 2),
            MenuItem::action("设为同音节跨音高延音 (+)", MenuAction::Workflow(A::ContinueSyllable)).enabled(self.session.selection.len() >= 2),
            MenuItem::separator(),
            MenuItem::action("颤音包络与滑音曲率设置…", MenuAction::Workflow(A::VibratoShape)).enabled(has_sel),
        ];
        self.show_menu(anchor, items, window, cx);
    }

    pub fn view_transport_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::editor::WorkflowAction as A;
        use crate::editor::integrations::IntegrationAction as I;
        let tick = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
        let pb = self.playback_settings();
        let items = vec![
            MenuItem::action("音阶与调式高亮设置…", MenuAction::Workflow(A::Integration(I::Scale))),
            MenuItem::action(
                if self.integrations.other_tracks { "✓ 叠加显示其他轨道重影音符" } else { "叠加显示其他轨道重影音符" },
                MenuAction::Workflow(A::Integration(I::OtherTracks)),
            ),
            MenuItem::action(
                if self.layout.show_phoneme_lane { "✓ 显示卷帘音素时序泳道" } else { "显示卷帘音素时序泳道" },
                MenuAction::TogglePhonemeLane,
            ),
            MenuItem::action(
                if self.layout.show_tempo_lane { "✓ 显示独立 BPM 速度泳道" } else { "显示独立 BPM 速度泳道" },
                MenuAction::ToggleTempoLane,
            ),
            MenuItem::action(
                if self.integrations.three_lanes { "✓ 并排显示三个参数泳道" } else { "并排显示三个参数泳道" },
                MenuAction::Workflow(A::Integration(I::ThreeLanes)),
            ),
            MenuItem::separator(),
            MenuItem::action("编辑当前位置速度 (BPM)…", MenuAction::Workflow(A::EditTempo(tick))),
            MenuItem::action("编辑当前位置拍号…", MenuAction::Workflow(A::EditMeter(tick))),
            MenuItem::action(if pb.looping { "✓ 循环播放已开启" } else { "开启循环播放" }, MenuAction::Workflow(A::ToggleLoop)),
            MenuItem::action("以选中音符设为循环范围", MenuAction::Workflow(A::RepeatSelection)).enabled(!self.session.selection.is_empty()),
            MenuItem::action(if pb.metronome { "✓ 节拍器已开启" } else { "开启节拍器" }, MenuAction::Workflow(A::ToggleMetronome)),
            MenuItem::action(
                if pb.export_click { "✓ 离线导出包含节拍器" } else { "离线导出包含节拍器" },
                MenuAction::Workflow(A::ToggleExportClick),
            ),
            MenuItem::separator(),
            MenuItem::action("选择 MIDI 输入设备开始录音…", MenuAction::Workflow(A::Integration(I::MidiPorts))),
            MenuItem::action("完成并写入 MIDI 录音", MenuAction::Workflow(A::Integration(I::FinishRecording)))
                .enabled(self.integrations.recording.is_some()),
            MenuItem::action("取消 MIDI 录音", MenuAction::Workflow(A::Integration(I::CancelRecording)))
                .enabled(self.integrations.recording.is_some()),
            MenuItem::separator(),
            MenuItem::action("键盘快捷键总览…", MenuAction::ShowShortcutsWindow).with_hint("F1"),
            MenuItem::action("开源软件与研究引用…", MenuAction::ShowDisclosuresWindow).with_hint("Shift+F1"),
        ];
        self.show_menu(anchor, items, window, cx);
    }
}

pub(crate) fn audition_menu_items(projects: &[crate::auditions::AuditionProject]) -> Vec<MenuItem> {
    let mut items = Vec::new();
    if !projects.is_empty() {
        items.push(MenuItem::disabled("本地试听工程"));
        items.extend(projects.iter().map(|entry|
            MenuItem::action(entry.title.clone(), MenuAction::OpenAudition(entry.path.clone()))));
        items.push(MenuItem::separator());
    }
    items.push(MenuItem::disabled("内建示例 · 打开可编辑副本"));
    items.extend(crate::demos::EXAMPLES.iter().map(|example|
        MenuItem::action(example.title, MenuAction::OpenBuiltinExample(example.id.into()))));
    items.push(MenuItem::separator());
    items.push(MenuItem::action("当前工程的示例署名与来源…", MenuAction::ExampleCredits));
    items.push(MenuItem::action("打开试听工程…", MenuAction::OpenProject));
    items
}

pub(crate) fn track_menu_items(project: &verdant_domain::Project, track: &verdant_domain::TrackId) -> Vec<MenuItem> {
    use crate::editor::WorkflowAction as A;
    let index = project.tracks.iter().position(|candidate| &candidate.id == track);
    let exists = index.is_some();
    let can_add = project.tracks.len() < verdant_audio::MAX_TRACKS;
    vec![
        MenuItem::action("重命名轨道…", MenuAction::Workflow(A::RenameTrack(track.clone()))).enabled(exists),
        MenuItem::action("轨道上移", MenuAction::Workflow(A::MoveTrack(track.clone(), -1))).enabled(index.is_some_and(|i| i > 0)),
        MenuItem::action("轨道下移", MenuAction::Workflow(A::MoveTrack(track.clone(), 1))).enabled(index.is_some_and(|i| i + 1 < project.tracks.len())),
        MenuItem::action("复制轨道", MenuAction::Workflow(A::DuplicateTrack(track.clone()))).enabled(exists && can_add),
        MenuItem::separator(),
        MenuItem::action("静音 / 取消静音", MenuAction::MuteTrack(track.clone())).enabled(exists),
        MenuItem::action("独奏 / 取消独奏", MenuAction::SoloTrack(track.clone())).enabled(exists),
        MenuItem::action("切换轨道颜色", MenuAction::CycleColor(track.clone())).enabled(exists),
        MenuItem::separator(),
        MenuItem::action("新建轨道", MenuAction::AddTrack).enabled(can_add),
        MenuItem::action("删除轨道", MenuAction::DeleteTrack(track.clone())).enabled(exists && project.tracks.len() > 1),
    ]
}

pub fn context_menu_layer(ed: &Editor, menu: &OpenMenu, window: &Window, cx: &mut Context<Editor>) -> impl IntoElement {
    let th = ed.theme;
    let win = window.viewport_size();
    let width = px(328.0).min(win.width);
    let height = px((menu.rows().count() as f32 * 26.0 + 8.0).min(win.height.as_f32() - 8.0).max(0.0));
    let x = menu.anchor.x.min((win.width - width).max(px(0.0))).max(px(0.0));
    let y = menu.anchor.y.min((win.height - height).max(px(0.0))).max(px(0.0));
    let selected = menu.selected;
    div().id("context-menu").role(gpui::Role::Menu).aria_label("上下文菜单")
        .key_context("ContextMenu").track_focus(&ed.focus_menu).occlude()
        .absolute().left(x).top(y).w(width).max_h(height).overflow_y_scroll()
        .py(px(4.0)).rounded(px(7.0)).bg(th.surface).border_1().border_color(th.separator).shadow_lg()
        .text_sm().text_color(th.text)
        .on_key_down(cx.listener(|ed, ev: &KeyDownEvent, window, cx| {
            let key = ev.keystroke.key.as_str();
            if !matches!(key, "up" | "down" | "home" | "end" | "enter" | "space" | "escape" | "tab" | "left" | "right") { return; }
            if matches!(key, "left" | "right") && !ed.menu.as_ref().is_some_and(|menu|
                menu.expanded.is_some() || menu.rows().nth(menu.selected).is_some_and(|(_, _, item)|
                    matches!(&item.command, Some(MenuCommand::Submenu(_))))) { return; }
            window.prevent_default(); cx.stop_propagation();
            if matches!(key, "escape" | "tab") { ed.close_menu(window, cx); return; }
            if matches!(key, "enter" | "space") {
                if let Some(row) = ed.menu.as_ref().map(|menu| menu.selected) { ed.activate_menu_item(row, window, cx); }
                return;
            }
            let Some(menu) = ed.menu.as_mut() else { return };
            if key == "left" {
                if let Some(index) = menu.expanded.take() { menu.selected = index; cx.notify(); }
                return;
            }
            if key == "right" {
                let submenu = menu.rows().nth(menu.selected).and_then(|(index, _, item)|
                    matches!(&item.command, Some(MenuCommand::Submenu(_))).then_some(index));
                if let Some(index) = submenu {
                    menu.expanded = Some(index);
                    menu.selected = index + 1;
                    cx.notify();
                }
                return;
            }
            let enabled: Vec<usize> = menu.rows().enumerate().filter(|(_, (_, _, item))| item.enabled && item.command.is_some()).map(|(i, _)| i).collect();
            if enabled.is_empty() { return; }
            let current = enabled.iter().position(|i| *i == menu.selected).unwrap_or(0);
            let i = match key { "home" => 0, "end" => enabled.len() - 1, "up" => (current + enabled.len() - 1) % enabled.len(), _ => (current + 1) % enabled.len() };
            menu.selected = enabled[i]; cx.notify();
        }))
        .children(menu.rows().enumerate().map(|(i, (index, child, item))| {
            if item.label.is_empty() { return div().h(px(8.0)).mx(px(8.0)).border_b_1().border_color(th.separator).into_any_element(); }
            let enabled = item.enabled;
            let submenu = matches!(&item.command, Some(MenuCommand::Submenu(_)));
            let expanded = submenu && menu.expanded == Some(index);
            div().id(SharedString::from(format!("menu-item-{i}"))).role(gpui::Role::MenuItem)
                .aria_label(item.label.clone())
                .a11y_synthetic_children(move |builder| {
                    if !enabled { builder.parent_node().set_disabled(); }
                    if submenu { builder.parent_node().set_expanded(expanded); }
                })
                .when(i == selected && enabled, |d| d.aria_active_descendant().bg(mix(th.surface, th.accent, 0.18)))
                .h(px(26.0)).mx(px(4.0)).px(px(10.0)).flex().items_center().rounded(px(4.0))
                .text_color(if enabled { th.text } else { th.text_tertiary })
                .child(div().when(child, |d| d.pl(px(16.0))).flex_1().truncate().child(item.label.clone()))
                .when(!item.hint.is_empty(), |d| d.child(div().ml(px(12.0)).flex_none().text_xs().text_color(th.text_secondary).child(if expanded { "⌄".into() } else { item.hint.clone() })))
                .on_mouse_move(cx.listener(move |ed, _, _, cx| { if enabled { if let Some(m) = ed.menu.as_mut() { if m.selected != i { m.selected = i; cx.notify(); } } } }))
                .on_click(cx.listener(move |ed, _, window, cx| {
                    if enabled { ed.activate_menu_item(i, window, cx); }
                    cx.stop_propagation();
                })).into_any_element()
        }))
        .on_mouse_down_out(cx.listener(|ed, _, window, cx| ed.close_menu(window, cx)))
}

#[cfg(test)]
#[path = "menu_file_tests.rs"]
mod file_tests;
