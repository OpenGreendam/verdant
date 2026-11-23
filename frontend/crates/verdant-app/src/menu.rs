//! GPUI popup menus with native event dispatch, focus and AccessKit menu roles.
//! The File menu opens editable projects and supported score formats, reuses the
//! ordinary export track/metadata prompts, and reads recent files from prefs.
//! The pinned Linux SDK has no OS context-popup API; these are GPUI widgets.
//! Track headers share one menu for rename, ordering and duplication. Boundary
//! moves and capacity-limited actions remain visible but cannot be activated.
use gpui::{Context, FocusHandle, IntoElement, KeyDownEvent, Pixels, Point, SharedString, Window, div, prelude::*, px};
use crate::editor::{Editor, MenuAction};
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

/// The header bar's primary menu (GNOME "open-menu"): common file actions,
/// the five menus as sections, playback options, then help. On phones it
/// also carries the toolbar controls that do not fit.
pub(crate) fn main_menu_items(class: crate::ui_controls::WidthClass, bottom: crate::editor::Bottom, count_in: u32) -> Vec<MenuItem> {
    use crate::editor::{Bottom as B, TopMenu as T, WorkflowAction as W};
    use crate::generated::controls::CommandId as C;
    use crate::ui_controls::WidthClass as W2;
    let open = |label: &'static str, menu| MenuItem::action(label, MenuAction::OpenMenu(menu)).with_hint("›");
    let mut items = vec![
        MenuItem::action("新建工程", MenuAction::Workflow(W::New)).with_hint("Ctrl+N"),
        MenuItem::file("打开工程…", FileAction::Open).with_hint("Ctrl+O"),
        MenuItem::action("保存", MenuAction::Save).with_hint("Ctrl+S"),
        MenuItem::action("另存为…", MenuAction::SaveAs).with_hint("Ctrl+Shift+S"),
        MenuItem::separator(),
        open("文件", T::File), open("编辑", T::Edit), open("轨道", T::Track),
        open("声库与词", T::VoiceLyrics), open("视图", T::ViewTransport),
        MenuItem::separator(),
        MenuItem::submenu("预备拍", [(0, "无"), (1, "1 小节"), (2, "2 小节"), (4, "4 小节"), (8, "8 小节")].into_iter()
            .map(|(bars, label)| MenuItem::action(label, MenuAction::Workflow(W::CountIn(bars))).checked(count_in == bars)).collect()),
        MenuItem::action("试听曲…", MenuAction::Run(C::AuditionProjects)),
    ];
    if class <= W2::Phone {
        items.extend([
            MenuItem::separator(),
            MenuItem::action("钢琴卷帘", MenuAction::ShowBottom(B::Editor)).with_hint("E").checked(bottom == B::Editor),
            MenuItem::action("混音器", MenuAction::ShowBottom(B::Mixer)).with_hint("X").checked(bottom == B::Mixer),
            MenuItem::action("音符表", MenuAction::ShowBottom(B::Table)).with_hint("T").checked(bottom == B::Table),
            MenuItem::action("效果器链", MenuAction::ShowBottom(B::Devices)).checked(bottom == B::Devices),
            MenuItem::separator(),
            MenuItem::action("撤销", MenuAction::Run(C::Undo)).with_hint("Ctrl+Z"),
            MenuItem::action("重做", MenuAction::Run(C::Redo)).with_hint("Ctrl+Shift+Z"),
            MenuItem::action("回到开头", MenuAction::Run(C::GoToStart)),
            MenuItem::action("立即渲染", MenuAction::Run(C::RenderNow)),
        ]);
    }
    items.extend([
        MenuItem::separator(),
        MenuItem::action("键盘快捷键", MenuAction::ShowShortcutsWindow).with_hint("F1"),
        MenuItem::action("开源软件与研究引用", MenuAction::ShowDisclosuresWindow).with_hint("Shift+F1"),
    ]);
    items
}

pub(crate) fn file_menu_items(prefs: &crate::files::Prefs) -> Vec<MenuItem> {
    use crate::editor::WorkflowAction as W;
    use crate::editor::integrations::IntegrationAction as I;
    let recent = recent_project_paths(prefs);
    let recent_items = if recent.is_empty() {
        vec![MenuItem::disabled("尚无最近文件")]
    } else {
        recent.into_iter().map(|path| {
            // Name first; the folder is the hint, so long paths stay readable.
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string());
            let folder = path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            MenuItem::file(name, FileAction::Recent(path)).with_hint(folder)
        }).collect()
    };
    vec![
        MenuItem::action("新建工程", MenuAction::Workflow(W::New)).with_hint("Ctrl+N"),
        MenuItem::file("打开工程（.verp）…", FileAction::Open).with_hint("Ctrl+O"),
        MenuItem::submenu("最近打开", recent_items),
        MenuItem::action("内建示例 / 试听工程…", MenuAction::ChooseExample),
        MenuItem::separator(),
        MenuItem::action("保存", MenuAction::Save).with_hint("Ctrl+S"),
        MenuItem::action("另存为…", MenuAction::SaveAs).with_hint("Ctrl+Shift+S"),
        MenuItem::action("还原到手动保存版本", MenuAction::Workflow(W::Revert)),
        MenuItem::action("恢复工程…", MenuAction::Workflow(W::Recovery)).with_hint("Ctrl+Alt+O"),
        MenuItem::separator(),
        MenuItem::submenu("导入", SCORE_FORMATS.into_iter().map(|(format, name)|
            MenuItem::file(format!("从 {name} 导入…"), FileAction::Import(format))).collect()),
        MenuItem::action("导入音频伴奏 / 参考音…", MenuAction::Workflow(W::Integration(I::ImportAudio))),
        MenuItem::separator(),
        MenuItem::submenu("导出", SCORE_FORMATS.into_iter().map(|(format, name)|
            MenuItem::file(format!("导出成 {name}…"), FileAction::Export(format))).collect()),
        MenuItem::action("导出 WAV（含插件）…", MenuAction::Workflow(W::ExportSettings)).with_hint("Ctrl+Alt+E"),
        MenuItem::action("导出 Verdant 工程副本 (.verp)…", MenuAction::ExportProject).with_hint("Ctrl+Shift+E"),
        MenuItem::action("Vocaloid 歌手元数据…", MenuAction::Workflow(W::Integration(I::VocaloidMetadata))),
        MenuItem::action("导入 / 导出报告…", MenuAction::Workflow(W::Integration(I::Report))),
    ]
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: SharedString,
    pub hint: SharedString,
    pub command: Option<MenuCommand>,
    pub enabled: bool,
    /// A toggle: shows a check mark column; the label stays the same.
    pub checked: Option<bool>,
    /// A non-interactive section title.
    pub header: bool,
}
impl MenuItem {
    fn new(label: impl Into<SharedString>, command: Option<MenuCommand>) -> Self {
        let enabled = command.is_some();
        Self { label: label.into(), hint: "".into(), command, enabled, checked: None, header: false }
    }
    pub fn action(label: impl Into<SharedString>, command: MenuAction) -> Self { Self::new(label, Some(MenuCommand::Editor(command))) }
    pub fn file(label: impl Into<SharedString>, command: FileAction) -> Self { Self::new(label, Some(MenuCommand::File(command))) }
    pub fn submenu(label: impl Into<SharedString>, items: Vec<MenuItem>) -> Self {
        // Stay openable even when every child is disabled, so users can see what exists.
        let enabled = !items.is_empty();
        Self { hint: "›".into(), ..Self::new(label, Some(MenuCommand::Submenu(items))) }.enabled(enabled)
    }
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self { self.hint = hint.into(); self }
    pub fn enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
    pub fn checked(mut self, on: bool) -> Self { self.checked = Some(on); self }
    pub fn disabled(label: impl Into<SharedString>) -> Self { Self::new(label, None) }
    pub fn header(label: impl Into<SharedString>) -> Self { Self { header: true, ..Self::new(label, None) } }
    pub fn separator() -> Self { Self::disabled("") }
    pub fn is_separator(&self) -> bool { self.label.is_empty() }
    fn row_height(&self) -> f32 { if self.is_separator() { 9.0 } else if self.header { 22.0 } else { 26.0 } }
}
#[derive(Clone)]
pub struct OpenMenu {
    /// A dismissed menu cannot lend its callbacks to a later opening at the
    /// same anchor, even when its content and document revision are identical.
    instance: String,
    pub window: gpui::AnyWindowHandle,
    pub anchor: Point<Pixels>,
    pub items: Vec<MenuItem>,
    pub selected: usize,
    pub previous_focus: Option<FocusHandle>,
    expanded: Option<usize>,
    scroll: gpui::ScrollHandle,
    native_scroll: crate::proto_scroll::NativeScroll,
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
        let Some(menu) = self.menu.as_mut().filter(|menu| menu.window == window.window_handle()) else { return; };
        let Some((_, _, item)) = menu.rows().nth(row).filter(|(_, _, item)| item.enabled) else { return; };
        match &item.command {
            Some(MenuCommand::Submenu(_)) => {},
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
        let previous_focus = self.menu.as_ref().filter(|m| m.window == window.window_handle())
            .and_then(|m| m.previous_focus.clone()).or_else(|| window.focused(cx));
        let selected = items.iter().position(|i| i.enabled && i.command.is_some()).unwrap_or(0);
        let scroll = gpui::ScrollHandle::new();
        let native_scroll = crate::proto_scroll::NativeScroll::new(scroll.clone());
        self.menu = Some(OpenMenu { instance: verdant_domain::TrackId::generate().to_string(), window: window.window_handle(), anchor, items, selected, previous_focus, expanded: None, scroll, native_scroll });
        window.focus(&self.focus_menu, cx);
        cx.notify();
    }

    pub fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.as_ref().is_some_and(|menu| menu.window != window.window_handle()) { return; }
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

    pub fn open_top_menu(&mut self, menu: crate::editor::TopMenu, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        use crate::editor::TopMenu as T;
        match menu {
            T::File => self.show_menu(anchor, file_menu_items(&self.prefs), window, cx),
            T::Edit => self.edit_menu(anchor, window, cx),
            T::Track => self.track_part_menu(anchor, window, cx),
            T::VoiceLyrics => self.voice_lyrics_menu(anchor, window, cx),
            T::ViewTransport => self.view_transport_menu(anchor, window, cx),
        }
    }

    /// One menu for narrow windows: the five toolbar menus plus the toolbar
    /// controls that were folded away, so nothing becomes unreachable.
    pub fn main_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let items = main_menu_items(self.width_class, self.bottom, self.playback_settings().count_in_bars);
        self.show_menu(anchor, items, window, cx);
    }

    pub fn edit_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.edit_menu_items();
        self.show_menu(anchor, items, window, cx);
    }

    pub(crate) fn edit_menu_items(&self) -> Vec<MenuItem> {
        use crate::editor::WorkflowAction as A;
        let tick = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
        let has_sel = !self.session.selection.is_empty();
        vec![
            MenuItem::action("撤销", MenuAction::Undo).with_hint("Ctrl+Z").enabled(self.store().undo_depth() > 0),
            MenuItem::action("重做", MenuAction::Redo).with_hint("Ctrl+Shift+Z").enabled(self.store().redo_depth() > 0),
            MenuItem::action("撤销历史…", MenuAction::ShowHistoryWindow).with_hint("Ctrl+Shift+H"),
            MenuItem::separator(),
            MenuItem::action("剪切", MenuAction::Cut).with_hint("Ctrl+X").enabled(has_sel),
            MenuItem::action("复制", MenuAction::Copy).with_hint("Ctrl+C").enabled(has_sel),
            MenuItem::action("粘贴", MenuAction::Paste).with_hint("Ctrl+V").enabled(!self.note_clipboard.is_empty()),
            MenuItem::action("重复选区", MenuAction::Duplicate).with_hint("Ctrl+D").enabled(has_sel),
            MenuItem::action("删除", MenuAction::DeleteSelection).with_hint("Del").enabled(has_sel),
            MenuItem::action("全选", MenuAction::SelectAll).with_hint("Ctrl+A"),
            MenuItem::separator(),
            MenuItem::header("音符"),
            MenuItem::action("在播放头分割", MenuAction::Workflow(A::Split(tick))).with_hint("Ctrl+Shift+J").enabled(has_sel),
            MenuItem::action("连接相邻音符", MenuAction::Workflow(A::Join)).with_hint("Ctrl+J").enabled(has_sel),
            MenuItem::action("量化到网格", MenuAction::Workflow(A::Quantize)).with_hint("Q").enabled(has_sel),
            MenuItem::action("闭合一格内间隙", MenuAction::Workflow(A::CloseGaps)).with_hint("Alt+Q").enabled(has_sel),
            MenuItem::action("消除重叠", MenuAction::Workflow(A::RemoveOverlaps)).with_hint("Shift+Q").enabled(has_sel),
            MenuItem::action("静音 / 取消静音", MenuAction::Workflow(A::MuteNotes)).enabled(has_sel),
            MenuItem::separator(),
            MenuItem::submenu("参数曲线", vec![
                MenuItem::action("选择当前音符全部曲线点", MenuAction::Workflow(A::SelectPoints)).enabled(has_sel),
                MenuItem::action("控制点批量变换…", MenuAction::Workflow(A::TransformPoints)),
                MenuItem::action("复制参数曲线", MenuAction::CopyCurve).enabled(has_sel),
                MenuItem::action("在播放头粘贴参数曲线", MenuAction::PasteCurveAt(tick)).enabled(self.curve_clipboard.is_some()),
            ]),
        ]
    }

    pub fn track_part_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.track_part_menu_items();
        self.show_menu(anchor, items, window, cx);
    }

    /// Track, then its automation, audio clips and parts. The long, rarely
    /// combined groups live in submenus so the menu fits a laptop screen.
    pub(crate) fn track_part_menu_items(&self) -> Vec<MenuItem> {
        use crate::editor::WorkflowAction as A;
        use crate::editor::integrations::IntegrationAction as I;
        let track = self.session.track.clone();
        let tick = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
        let auto_on = self.automation_ui.expanded_tracks.contains(&track);
        let clip = self.integrations.selected_clip.is_some();
        let part = self.selected_part.is_some();
        let mut items = track_menu_items(self.project(), &track);
        items.extend([
            MenuItem::separator(),
            MenuItem::header("自动化"),
            MenuItem::action("当前轨道自动化泳道", MenuAction::ToggleTrackAutomation(track)).with_hint("A").checked(auto_on),
            MenuItem::action("主总线自动化泳道", MenuAction::ToggleMasterAutomation).checked(self.automation_ui.show_master),
            MenuItem::separator(),
            MenuItem::submenu("片段 (Part)", vec![
                MenuItem::action("从选中音符创建片段", MenuAction::Workflow(A::CreatePart)).enabled(!self.session.selection.is_empty()),
                MenuItem::action("片段名称…", MenuAction::Workflow(A::RenamePart)).enabled(part),
                MenuItem::action("在播放头分割片段", MenuAction::Workflow(A::SplitPart(tick))).enabled(part),
                MenuItem::action("连接右侧片段", MenuAction::Workflow(A::JoinPart)).enabled(part),
                MenuItem::action("链接复制片段", MenuAction::Workflow(A::DuplicatePart(true))).enabled(part),
                MenuItem::action("独立复制片段", MenuAction::Workflow(A::DuplicatePart(false))).enabled(part),
                MenuItem::action("取消片段链接", MenuAction::Workflow(A::UnlinkPart)).enabled(part),
                MenuItem::action("删除片段", MenuAction::Workflow(A::DeletePart)).enabled(part),
            ]),
            MenuItem::submenu("音频片段", vec![
                MenuItem::action("导入音频伴奏 / 参考音…", MenuAction::Workflow(A::Integration(I::ImportAudio))),
                MenuItem::action("音频片段列表…", MenuAction::Workflow(A::Integration(I::Clips))),
                MenuItem::action("精确编辑所选音频片段…", MenuAction::Workflow(A::Integration(I::EditClip))).enabled(clip),
                MenuItem::action("重新定位丢失音源…", MenuAction::Workflow(A::Integration(I::RelinkClip))).enabled(clip),
                MenuItem::action("授权重新打开已保存音频…", MenuAction::Workflow(A::Integration(I::AuthorizeAudio))),
            ]),
        ]);
        if let Ok(parts) = verdant_domain::parts::Parts::load(self.project()) {
            if !parts.instances.is_empty() {
                items.push(MenuItem::submenu("定位片段", parts.instances.into_iter()
                    .map(|p| MenuItem::action(p.name, MenuAction::Workflow(A::SelectPart(p.id)))).collect()));
            }
        }
        items
    }

    pub fn voice_lyrics_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.voice_lyrics_menu_items();
        self.show_menu(anchor, items, window, cx);
    }

    pub(crate) fn voice_lyrics_menu_items(&self) -> Vec<MenuItem> {
        use crate::editor::WorkflowAction as A;
        use crate::editor::integrations::IntegrationAction as I;
        let has_sel = !self.session.selection.is_empty();
        let several = self.session.selection.len() >= 2;
        vec![
            MenuItem::header("声库"),
            MenuItem::action("选择当前轨道歌手…", MenuAction::Workflow(A::Integration(I::Singers))),
            MenuItem::action("导入 verdant-voice 声库…", MenuAction::Workflow(A::Integration(I::InstallVoice))),
            MenuItem::separator(),
            MenuItem::header("读音"),
            MenuItem::action("读音器与默认语言…", MenuAction::Workflow(A::Integration(I::Phonemizers))),
            MenuItem::submenu("多语言分段", vec![
                MenuItem::action("添加分段映射…", MenuAction::Workflow(A::Integration(I::AddSegment))),
                MenuItem::action("管理分段列表…", MenuAction::Workflow(A::Integration(I::Segments))),
            ]),
            MenuItem::submenu("用户读音词典", vec![
                MenuItem::action("编辑工程词典…", MenuAction::Workflow(A::Integration(I::Dictionary))),
                MenuItem::action("导入词典文件…", MenuAction::Workflow(A::Integration(I::ImportDictionary))),
                MenuItem::action("导出词典文件…", MenuAction::Workflow(A::Integration(I::ExportDictionary))),
            ]),
            MenuItem::separator(),
            MenuItem::header("歌词"),
            MenuItem::action("批量填写歌词（按词 / 空格）…", MenuAction::Workflow(A::BatchLyrics(false))).with_hint("Ctrl+L").enabled(has_sel),
            MenuItem::action("逐字填写歌词（每字一音）…", MenuAction::Workflow(A::BatchLyrics(true))).enabled(has_sel),
            MenuItem::action("歌词循环左移", MenuAction::Workflow(A::ShiftLyrics(false))).enabled(several),
            MenuItem::action("歌词循环右移", MenuAction::Workflow(A::ShiftLyrics(true))).enabled(several),
            MenuItem::action("设为同音节延音 (+)", MenuAction::Workflow(A::ContinueSyllable)).enabled(several),
            MenuItem::separator(),
            MenuItem::action("颤音包络与滑音曲率…", MenuAction::Workflow(A::VibratoShape)).enabled(has_sel),
        ]
    }

    pub fn view_transport_menu(&mut self, anchor: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.view_transport_menu_items();
        self.show_menu(anchor, items, window, cx);
    }

    pub(crate) fn view_transport_menu_items(&self) -> Vec<MenuItem> {
        use crate::editor::WorkflowAction as A;
        use crate::editor::integrations::IntegrationAction as I;
        let tick = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
        let pb = self.playback_settings();
        let recording = self.integrations.recording.is_some();
        vec![
            MenuItem::header("显示"),
            MenuItem::action("音素时序泳道", MenuAction::TogglePhonemeLane).checked(self.layout.show_phoneme_lane),
            MenuItem::action("BPM 速度泳道", MenuAction::ToggleTempoLane).checked(self.layout.show_tempo_lane),
            MenuItem::action("添加参数泳道", MenuAction::Workflow(A::Integration(I::ThreeLanes))),
            MenuItem::action("叠加其他轨道重影音符", MenuAction::Workflow(A::Integration(I::OtherTracks))).checked(self.integrations.other_tracks),
            MenuItem::action("音阶与调式高亮…", MenuAction::Workflow(A::Integration(I::Scale))),
            MenuItem::separator(),
            MenuItem::header("走带"),
            MenuItem::action("循环播放", MenuAction::Workflow(A::ToggleLoop)).checked(pb.looping),
            MenuItem::action("以选中音符设为循环范围", MenuAction::Workflow(A::RepeatSelection)).enabled(!self.session.selection.is_empty()),
            MenuItem::action("节拍器", MenuAction::Workflow(A::ToggleMetronome)).checked(pb.metronome),
            MenuItem::action("离线导出包含节拍器", MenuAction::Workflow(A::ToggleExportClick)).checked(pb.export_click),
            MenuItem::action("当前位置速度 (BPM)…", MenuAction::Workflow(A::EditTempo(tick))),
            MenuItem::action("当前位置拍号…", MenuAction::Workflow(A::EditMeter(tick))),
            MenuItem::separator(),
            MenuItem::submenu("MIDI 录音", vec![
                MenuItem::action("选择输入设备并开始录音…", MenuAction::Workflow(A::Integration(I::MidiPorts))).enabled(!recording),
                MenuItem::action("完成并写入录音", MenuAction::Workflow(A::Integration(I::FinishRecording))).enabled(recording),
                MenuItem::action("取消录音", MenuAction::Workflow(A::Integration(I::CancelRecording))).enabled(recording),
            ]),
            MenuItem::separator(),
            MenuItem::header("帮助"),
            MenuItem::action("键盘快捷键…", MenuAction::ShowShortcutsWindow).with_hint("F1"),
            MenuItem::action("开源软件与研究引用…", MenuAction::ShowDisclosuresWindow).with_hint("Shift+F1"),
        ]
    }
}

pub(crate) fn audition_menu_items(projects: &[crate::auditions::AuditionProject]) -> Vec<MenuItem> {
    let mut items = Vec::new();
    if !projects.is_empty() {
        items.push(MenuItem::header("本地试听工程"));
        items.extend(projects.iter().map(|entry|
            MenuItem::action(entry.title.clone(), MenuAction::OpenAudition(entry.path.clone()))));
        items.push(MenuItem::separator());
    }
    items.push(MenuItem::header("内建示例 · 打开可编辑副本"));
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
    let state = project.track(track);
    vec![
        MenuItem::action("重命名轨道…", MenuAction::Workflow(A::RenameTrack(track.clone()))).enabled(exists),
        MenuItem::action("轨道上移", MenuAction::Workflow(A::MoveTrack(track.clone(), -1))).enabled(index.is_some_and(|i| i > 0)),
        MenuItem::action("轨道下移", MenuAction::Workflow(A::MoveTrack(track.clone(), 1))).enabled(index.is_some_and(|i| i + 1 < project.tracks.len())),
        MenuItem::action("复制轨道", MenuAction::Workflow(A::DuplicateTrack(track.clone()))).enabled(exists && can_add),
        MenuItem::separator(),
        MenuItem::action("静音", MenuAction::MuteTrack(track.clone())).enabled(exists).checked(state.is_some_and(|t| t.muted)),
        MenuItem::action("独奏", MenuAction::SoloTrack(track.clone())).enabled(exists).checked(state.is_some_and(|t| t.solo)),
        MenuItem::action("切换轨道颜色", MenuAction::CycleColor(track.clone())).enabled(exists),
        MenuItem::action(if state.is_some_and(|t| t.is_instrument()) { "转为人声轨" } else { "转为乐器轨" },
            MenuAction::MakeInstrumentTrack { track: track.clone(), instrument: !state.is_some_and(|t| t.is_instrument()) }).enabled(exists),
        MenuItem::separator(),
        MenuItem::action("新建人声轨", MenuAction::AddTrack).enabled(can_add),
        MenuItem::action("新建乐器轨", MenuAction::AddInstrumentTrack).enabled(can_add),
        MenuItem::action("删除轨道", MenuAction::DeleteTrack(track.clone())).enabled(exists && project.tracks.len() > 1),
    ]
}

fn collection_revision(ed: &Editor, menu: &OpenMenu) -> String {
    // Expanded children change the row-to-command mapping. Old virtual row
    // callbacks must therefore expire when the submenu topology changes.
    format!("{}:{}:{}:{:?}:{:?}:{:?}:{:?}", menu.instance, ed.document_generation, ed.revision().get(), menu.window, menu.anchor, menu.expanded, menu.items)
}

pub fn context_menu_layer(ed: &Editor, menu: &OpenMenu, _window: &Window, cx: &mut Context<Editor>) -> impl IntoElement {
    let revision = collection_revision(ed, menu);
    let items = menu.rows().map(|(_, _, item)| if !item.enabled || item.command.is_none() { 0 }
        else if matches!(&item.command, Some(MenuCommand::Submenu(_))) { 2 } else { 1 }).collect();
    let expanded = menu.rows().enumerate().find_map(|(row, (index, child, _))|
        (!child && menu.expanded == Some(index)).then_some(row as i64)).unwrap_or(-1);
    let props = serde_json::json!({"mode":"menu", "label":"上下文菜单", "cursor":menu.selected,
        "expanded": expanded, "revision":revision, "autoFocus":true, "disabled":false});
    let draw_editor = cx.entity(); let snapshot = menu.clone(); let basis = revision.clone();
    let draw = std::rc::Rc::new(move |handle: &verdant_collection_control::CollectionControlHandle, _focus: &gpui::FocusHandle, _window: &mut Window, _cx: &mut gpui::App| {
        let editor = draw_editor.clone(); let snapshot = snapshot.clone(); let basis = basis.clone(); let handle = handle.clone();
        let target = snapshot.native_scroll.clone();
        crate::proto_scroll::content("compiled-context-menu-scroll", "菜单选项", move |window, cx| {
            editor.update(cx, |ed, cx| native_context_menu_layer(ed, &snapshot, handle.clone(), basis.clone(), window, cx).into_any_element())
        }).tab_stop(false).native_scroll(target).into_any_element()
    });
    let editor = cx.entity().downgrade();
    let sink = std::rc::Rc::new(move |name: &str, payload: &serde_json::Value, window: &mut Window, cx: &mut gpui::App| {
        let _ = editor.update(cx, |ed, cx| {
            let Some(menu) = ed.menu.as_ref().filter(|menu| menu.window == window.window_handle()) else { return };
            if payload.get("revision").and_then(serde_json::Value::as_str) != Some(collection_revision(ed, menu).as_str()) { return; }
            let index = crate::proto_controls::source_integer(payload.get("index")).unwrap_or(-1);
            match name {
                "close" => ed.close_menu(window, cx),
                "activate" if index >= 0 => ed.activate_menu_item(index as usize, window, cx),
                "select" if index >= 0 => { if let Some(menu) = ed.menu.as_mut() { menu.selected = index as usize; menu.scroll.scroll_to_item(index as usize); } cx.notify(); },
                "expand" => {
                    if let Some(menu) = ed.menu.as_mut() {
                        let previous_parent = menu.rows().enumerate().find_map(|(row, (source, child, _))|
                            (!child && menu.expanded == Some(source)).then_some(row));
                        let target = if index >= 0 { menu.rows().nth(index as usize).map(|(source, _, _)| source) } else { None };
                        menu.expanded = target;
                        if index >= 0 { menu.selected = index as usize; }
                        else if let Some(parent) = previous_parent { menu.selected = parent; }
                    }
                    cx.notify();
                },
                _ => {},
            }
        });
    });
    crate::proto_collection::collection("compiled-context-menu", props, items, draw, sink)
}

fn native_context_menu_layer(ed: &Editor, menu: &OpenMenu, handle: verdant_collection_control::CollectionControlHandle,
    revision: String, window: &Window, _cx: &mut Context<Editor>) -> impl IntoElement {

    let th = ed.theme;
    let win = window.viewport_size();
    let width = px(328.0).min(win.width);
    let height = px((menu.rows().map(|(_, _, item)| item.row_height()).sum::<f32>() + 14.0).min(win.height.as_f32() - 8.0).max(0.0));
    // Reserve a check column only in menus that contain toggles.
    let checks = menu.rows().any(|(_, _, item)| item.checked.is_some());
    let place = popover_placement(menu.anchor, (width.as_f32(), height.as_f32()), (win.width.as_f32(), win.height.as_f32()));
    let selected = handle.state("cursor").ok().and_then(|v| crate::proto_controls::source_index(Some(&v))).unwrap_or(menu.selected);
    let key_handle = handle.clone();
    let list = div().id("context-menu")
        .key_context("ContextMenu").occlude()
        .w_full().max_h(height).overflow_y_scroll().track_scroll(&menu.scroll)
        .p(px(6.0)).rounded(px(12.0)).bg(th.popover).border_1().border_color(th.shade).shadow_lg()
        .text_sm().text_color(th.text)
        .on_key_down(move |event, window, cx| crate::proto_collection::key(&key_handle, event, window, cx))
        .children(menu.rows().enumerate().map(|(i, (index, child, item))| {
            if item.is_separator() {
                return div().h(px(9.0)).mx(px(8.0)).flex().items_center()
                    .child(div().w_full().h(px(1.0)).bg(th.shade)).into_any_element();
            }
            if item.header {
                return div().h(px(22.0)).mx(px(4.0)).px(px(10.0)).when(checks, |d| d.pl(px(28.0))).when(child, |d| d.pl(px(if checks { 44.0 } else { 26.0 })))
                    .flex().items_end().pb(px(3.0)).text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(th.text_secondary)
                    .child(item.label.clone()).into_any_element();
            }
            let enabled = item.enabled;
            let submenu = matches!(&item.command, Some(MenuCommand::Submenu(_)));
            let expanded = submenu && menu.expanded == Some(index);
            let choose = handle.clone(); let choose_revision = revision.clone();
            let hover = handle.clone(); let hover_revision = revision.clone();
            crate::proto_controls::action(SharedString::from(format!("menu-item-{i}")), item.label.clone())
                .role(if item.checked.is_some() { gpui::Role::MenuItemCheckBox } else { gpui::Role::MenuItem })
                .disabled(!enabled)
                .tab_index(-1)
                .aria_label(item.label.clone())
                .when_some(item.checked, |d, on| d.aria_selected(on))
                .when(submenu, |d| d.aria_expanded(expanded))
                .when(i == selected && enabled, |d| d.aria_active_descendant().bg(th.hover))
                .h(px(26.0)).px(px(10.0)).flex().items_center().rounded(px(6.0))
                .text_color(if enabled { th.text } else { th.text_tertiary })
                .when(checks, |d| d.child(div().w(px(18.0)).flex_none().text_color(th.accent).child(if item.checked == Some(true) { "✓" } else { "" })))
                .child(div().when(child, |d| d.pl(px(16.0))).flex_1().truncate().child(item.label.clone()))
                .when(!item.hint.is_empty(), |d| d.child(div().ml(px(12.0)).flex_none().text_xs().text_color(th.text_secondary).child(if expanded { "⌄".into() } else { item.hint.clone() })))
                .on_mouse_move(move |_, window, cx| {
                    crate::proto_collection::call(&hover, "selectIndex", vec![i.into(), false.into(), hover_revision.clone().into()], window, cx);
                })
                .on_click(move |_, window, cx| {
                    crate::proto_collection::call(&choose, "choose", vec![i.into(), false.into(), choose_revision.clone().into()], window, cx);
                    cx.stop_propagation();
                }).into_any_element()
        }))
        .on_mouse_down_out(move |_, window, cx| crate::proto_collection::call(&handle, "dismiss", vec![], window, cx));
    let (popover, border) = (th.popover, th.shade);
    div().absolute().left(px(place.x)).top(px(place.y)).w(width)
        .child(list)
        // The GtkPopover tail, pointing at the anchor.
        .when_some(place.arrow, |d, (ax, below)| d.child(gpui::canvas(|_, _, _| (), move |bounds, (), window, _| {
            let o = bounds.origin;
            let tail = |inset: f64, color: gpui::Rgba, window: &mut Window| {
                let (tip, base) = if below { (-ARROW_H + inset, 1.5) } else { (ARROW_H - inset, -1.5) };
                let mut path = gpui::PathBuilder::fill();
                path.move_to(crate::paint::p(ax, tip, o));
                path.line_to(crate::paint::p(ax - ARROW_W / 2.0 + inset, base, o));
                path.line_to(crate::paint::p(ax + ARROW_W / 2.0 - inset, base, o));
                path.close();
                if let Ok(path) = path.build() { window.paint_path(path, color); }
            };
            tail(0.0, border, window);
            tail(1.2, popover, window);
        }).absolute().left_0().w_full().h(px(0.0)).when(below, |d| d.top_0()).when(!below, |d| d.bottom_0())))
}

const ARROW_W: f64 = 20.0;
const ARROW_H: f64 = 10.0;

/// Where a popover goes for an anchor point, as GtkPopover does: centred
/// under the anchor with the tail pointing at it, flipped above when there
/// is no room below, kept inside the window. `arrow` is the tail's x inside
/// the popover and whether the popover is below the anchor.
#[derive(Debug, PartialEq)]
pub(crate) struct Placement { pub x: f32, pub y: f32, pub arrow: Option<(f64, bool)> }

pub(crate) fn popover_placement(anchor: Point<Pixels>, (w, h): (f32, f32), (win_w, win_h): (f32, f32)) -> Placement {
    let (ax, ay) = (anchor.x.as_f32(), anchor.y.as_f32());
    let margin = 4.0;
    let gap = ARROW_H as f32;
    let x = (ax - w / 2.0).min(win_w - w - margin).max(margin.min((win_w - w).max(0.0)));
    let below = ay + gap + h <= win_h - margin || ay < win_h / 2.0;
    let y = if below { ay + gap } else { ay - gap - h };
    let y = y.min(win_h - h - margin).max(margin.min((win_h - h).max(0.0)));
    // Only draw the tail where it can touch the popover's straight edge.
    let tail_x = (ax - x) as f64;
    let reaches = tail_x >= 16.0 && tail_x <= w as f64 - 16.0 && (if below { (y - ay - gap).abs() < 0.5 } else { (ay - gap - h - y).abs() < 0.5 });
    Placement { x, y, arrow: reaches.then_some((tail_x, below)) }
}

#[cfg(test)]
#[path = "menu_file_tests.rs"]
mod file_tests;
