//! GPUI popup menus with native event dispatch, focus and AccessKit menu roles.
//! The File menu opens editable projects and supported score formats, reuses the
//! ordinary export track/metadata prompts, and reads recent files from prefs.
//! The pinned Linux SDK has no OS context-popup API; these are GPUI widgets.
//! Track headers share one menu for rename, ordering and duplication. Boundary
//! moves and capacity-limited actions remain visible but cannot be activated.
use gpui::{App, AnyElement, Bounds, Context, Element, FocusHandle, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, Point, SharedString, Window, div, point, prelude::*, px};
use std::{cell::{Cell, RefCell}, rc::Rc};
use crate::editor::{Editor, MenuAction};
use crate::generated::controls::{CommandArg, CommandId};
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
    Boolean(MenuBoolean),
}

/// Accepted Boolean facts and their existing domain or UI write boundaries.
/// Targets carry stable identities; a checked row never rebuilds a toggle.
#[derive(Clone, Debug)]
pub enum MenuBoolean {
    Command(crate::generated::controls::CommandId, crate::generated::controls::CommandArg),
    FollowPlayhead,
    ExportClick,
    PartMuted(String),
    DeviceParameter { target: crate::devices::DeviceTarget, slot: usize, key: verdant_plugins::InsertKey, parameter: String },
}

impl MenuBoolean {
    fn current(&self, ed: &Editor) -> Option<bool> {
        match self {
            Self::Command(command, arg) => ed.source_command_selection(*command, arg),
            Self::FollowPlayhead => Some(ed.session.follow_playhead),
            Self::ExportClick => Some(ed.playback_settings().export_click),
            Self::PartMuted(id) => verdant_domain::parts::Parts::load(ed.project()).ok()?.instance(id).map(|part| part.muted),
            Self::DeviceParameter { target, slot, key, parameter } => {
                if ed.inserts_loading || ed.plugin_busy.contains(key) { return None; }
                ed.current_device_parameters(target, *slot, *key)?.iter().find(|desc| &desc.id == parameter)
                    .filter(|desc| desc.value.is_finite()).map(|desc| desc.value >= 0.5)
            },
        }
    }

    fn accept(&self, ed: &mut Editor, selected: bool, cx: &mut Context<Editor>) {
        if self.current(ed).is_none_or(|current| current == selected) { return; }
        match self {
            Self::Command(command, arg) => ed.accept_source_command_selection(*command, arg, selected, cx),
            Self::FollowPlayhead => crate::clock::set_follow(ed, selected, cx),
            Self::ExportClick => ed.edit_playback("导出节拍器", cx, |settings| settings.export_click = selected),
            Self::PartMuted(id) => {
                if let Err(error) = ed.edit_part(id, "音符组静音", cx, |part| { part.muted = selected; Ok(()) }) { ed.notice(error); }
            },
            Self::DeviceParameter { target, slot, parameter, .. } => {
                ed.set_device_params(target.clone(), *slot, vec![(parameter.clone(), if selected { 1.0 } else { 0.0 })], cx);
            },
        }
        cx.notify();
    }
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
        MenuItem::action("导入 / 导出报告…", MenuAction::Workflow(W::Integration(I::Report))),
    ]
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: SharedString,
    pub hint: SharedString,
    pub command: Option<MenuCommand>,
    pub enabled: bool,
    /// Accepted selection shown in the check column. Ordinary checked actions
    /// are choices; Boolean commands expose a checkbox and a live accepted value.
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
    pub fn boolean(label: impl Into<SharedString>, target: MenuBoolean, checked: bool) -> Self {
        Self::new(label, Some(MenuCommand::Boolean(target))).checked(checked)
    }
    pub fn boolean_command(label: impl Into<SharedString>, command: crate::generated::controls::CommandId,
        arg: crate::generated::controls::CommandArg, checked: bool) -> Self {
        Self::boolean(label, MenuBoolean::Command(command, arg), checked)
    }
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
    fn is_submenu(&self, ed: &Editor) -> bool {
        use crate::editor::{WorkflowAction as W, integrations::IntegrationAction as I};
        match &self.command {
            Some(MenuCommand::Submenu(_) | MenuCommand::Editor(MenuAction::OpenMenu(_) | MenuAction::ChooseExample | MenuAction::Run(CommandId::AuditionProjects))) => true,
            Some(MenuCommand::Editor(MenuAction::Workflow(W::Recent | W::Recovery | W::ExportSettings))) => true,
            Some(MenuCommand::Editor(MenuAction::Workflow(W::Integration(
                I::Singers | I::Phonemizers | I::Segments | I::Dictionary | I::MonitorPorts | I::MidiPorts | I::MidiChannels(_) | I::Clips | I::Automation,
            )))) => true,
            Some(MenuCommand::File(FileAction::Export(Format::Ust)) | MenuCommand::Editor(MenuAction::Workflow(W::Integration(I::ScoreExport(Format::Ust))))) => ed.project().tracks.len() > 1,
            _ => false,
        }
    }
    fn row_height(&self) -> f32 { if self.is_separator() { 9.0 } else if self.header { 22.0 } else { 26.0 } }
    fn state(&self, ed: &Editor) -> (bool, Option<bool>) {
        if let Some(MenuCommand::Boolean(target)) = &self.command {
            let current = target.current(ed);
            (self.enabled && current.is_some(), current.or(self.checked))
        } else { (self.enabled, self.checked) }
    }
}
#[derive(Clone)]
enum MenuAnchor {
    Point(Point<Pixels>),
    Bounds(Bounds<Pixels>),
    Control { owner: verdant_action_control::native::gpui_native_owner::OwnerWeak, accepted: Bounds<Pixels> },
    Item { viewport: gpui::ScrollHandle, index: usize },
}

impl OpenMenu {
    /// The address of the Action this menu drops down from, if any.
    pub(crate) fn anchor_owner(&self) -> Option<usize> {
        match &self.origin {
            MenuAnchor::Control { owner, .. } => Some(owner.as_ptr() as *const () as usize),
            _ => None,
        }
    }
}

impl std::fmt::Debug for MenuAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Point(point) => f.debug_tuple("Point").field(point).finish(),
            Self::Bounds(bounds) => f.debug_tuple("Bounds").field(bounds).finish(),
            Self::Control { accepted, .. } => f.debug_tuple("Control").field(accepted).finish(),
            Self::Item { index, .. } => f.debug_tuple("Item").field(index).finish(),
        }
    }
}

impl MenuAnchor {
    fn control_bounds(&self, window: &Window) -> Option<Bounds<Pixels>> {
        match self {
            Self::Bounds(target) => Some(*target),
            Self::Control { owner, accepted } => Some(owner.upgrade().and_then(|owner| {
                let owner = owner.borrow();
                owner.bounds.filter(|_| owner.present && owner.window_id == Some(window.window_handle().window_id()))
            }).unwrap_or(*accepted)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OpenMenu {
    /// A dismissed menu cannot lend its callbacks to a later opening at the
    /// same anchor, even when its content and document revision are identical.
    instance: String,
    pub window: gpui::AnyWindowHandle,
    pub items: Vec<MenuItem>,
    pub selected: usize,
    pub previous_focus: Option<FocusHandle>,
    expanded: Option<usize>,
    branch_revision: u64,
    child: Option<Box<OpenMenu>>,
    origin: MenuAnchor,
    viewport: gpui::ScrollHandle,
    scroll_engine: gpui::Entity<gpui::ScrollHandle>,
    focus_pending: bool,
    label: SharedString,
}

impl OpenMenu {
    fn panel(&self, path: &[usize]) -> Option<&Self> {
        if let Some((index, rest)) = path.split_first() {
            if self.expanded != Some(*index) { return None; }
            self.child.as_deref()?.panel(rest)
        } else { Some(self) }
    }

    fn panel_mut(&mut self, path: &[usize]) -> Option<&mut Self> {
        if let Some((index, rest)) = path.split_first() {
            if self.expanded != Some(*index) { return None; }
            self.child.as_deref_mut()?.panel_mut(rest)
        } else { Some(self) }
    }

    fn contains(&self, position: Point<Pixels>) -> bool {
        self.viewport.bounds().contains(&position) || self.child.as_ref().is_some_and(|child| child.contains(position))
    }

    fn new(origin: MenuAnchor, items: Vec<MenuItem>, window: &Window, label: SharedString, cx: &mut Context<Editor>) -> Self {
        let selected = items.iter().position(|item| item.enabled && item.command.is_some()).unwrap_or(0);
        let viewport = gpui::ScrollHandle::new();
        let scroll_engine = cx.new(|_| viewport.clone());
        Self { instance: verdant_domain::TrackId::generate().to_string(), window: window.window_handle(), origin,
            items, selected, previous_focus: None, expanded: None, branch_revision: 0, child: None, viewport, scroll_engine, focus_pending: true, label }
    }

    fn open_child(&mut self, row: usize, child: OpenMenu) {
        self.branch_revision += 1;
        self.selected = row;
        self.expanded = Some(row);
        self.child = Some(Box::new(child));
    }

    fn collapse(&mut self) {
        if self.child.take().is_some() { self.branch_revision += 1; }
        self.expanded = None;
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

    fn activate_menu_item(&mut self, path: &[usize], row: usize, selected: Option<bool>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.menu.as_ref().filter(|menu| menu.window == window.window_handle()) else { return; };
        let Some(menu) = root.panel(path) else { return; };
        let Some(item) = menu.items.get(row) else { return; };
        let (enabled, checked) = item.state(self);
        if !enabled { return; }
        let submenu = item.is_submenu(self);
        let command = item.command.clone();
        let refresh = matches!(&command, Some(MenuCommand::Editor(MenuAction::Workflow(crate::editor::WorkflowAction::ExportSetting(_)))));
        let choice = checked.is_none() && selected.is_none() || checked.is_some() && selected == Some(true);
        let Some(command) = command.filter(|command| match command {
            MenuCommand::Boolean(_) => selected.is_some(),
            MenuCommand::Editor(_) | MenuCommand::File(_) => choice,
            MenuCommand::Submenu(_) => false,
        }) else { return; };
        let Some(mut retained) = self.dismiss_menu(window, cx) else { return; };
        match command {
            MenuCommand::Boolean(target) => target.accept(self, selected.expect("accepted Boolean proposal"), cx),
            MenuCommand::Editor(command) => self.run_menu_action(command, window, cx),
            MenuCommand::File(command) => self.run_file_action(command, window, cx),
            MenuCommand::Submenu(_) => unreachable!("submenu is an expansion"),
        }
        // Some application commands produce their choices lazily (MIDI ports,
        // singers, workflow menus). They join the same cascade as static items.
        // Closing before dispatch preserves ordinary leaf focus/domain guards.
        if self.menu.as_ref().is_some_and(|menu| menu.window == retained.window) {
            let mut opened = self.menu.take().expect("newly opened menu");
            let parent = retained.panel_mut(path).expect("retained invoking panel");
            if refresh {
                parent.selected = row;
                parent.items = opened.items;
                parent.collapse();
                parent.focus_pending = true;
            } else {
                opened.origin = MenuAnchor::Item { viewport: parent.viewport.clone(), index: row };
                opened.label = parent.items[row].label.clone();
                opened.previous_focus = None;
                parent.open_child(row, opened);
            }
            self.menu = Some(retained);
            cx.notify();
        } else if submenu && self.menu.is_none() {
            if let Some(panel) = retained.panel_mut(path) { panel.focus_pending = true; }
            self.menu = Some(retained);
            cx.notify();
        }
    }

    fn expand_menu_item(&mut self, path: &[usize], index: Option<usize>, cursor: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.menu.as_ref().and_then(|root| root.panel(path)) else { return; };
        let child = index.and_then(|index| {
            let item = panel.items.get(index)?;
            if !item.state(self).0 { return None; }
            let items = match &item.command {
                Some(MenuCommand::Submenu(items)) => items.clone(),
                Some(MenuCommand::Editor(MenuAction::OpenMenu(menu))) => match menu {
                    crate::editor::TopMenu::File => file_menu_items(&self.prefs),
                    crate::editor::TopMenu::Edit => self.edit_menu_items(),
                    crate::editor::TopMenu::Track => self.track_part_menu_items(),
                    crate::editor::TopMenu::VoiceLyrics => self.voice_lyrics_menu_items(),
                    crate::editor::TopMenu::ViewTransport => self.view_transport_menu_items(),
                },
                _ => return None,
            };
            Some(OpenMenu::new(MenuAnchor::Item { viewport: panel.viewport.clone(), index }, items, window, item.label.clone(), cx))
        });
        if child.is_none() && let Some(index) = index {
            self.activate_menu_item(path, index, None, window, cx);
            return;
        }
        let panel = self.menu.as_mut().and_then(|root| root.panel_mut(path)).expect("admitted menu panel");
        if let Some(child) = child {
            panel.open_child(index.expect("submenu index"), child);
        } else if index.is_none() {
            if let Some(cursor) = cursor { panel.selected = cursor; }
            panel.collapse();
            panel.focus_pending = true;
        }
        cx.notify();
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
        let origin = crate::proto_controls::activation_origin(window).and_then(|owner| {
            let accepted = owner.borrow().bounds?;
            Some(MenuAnchor::Control { owner: Rc::downgrade(&owner), accepted })
        }).unwrap_or(MenuAnchor::Point(anchor));
        self.show_menu_at(origin, items, window, cx);
    }

    pub(crate) fn show_dropdown(&mut self, bounds: Bounds<Pixels>, items: Vec<MenuItem>, window: &mut Window, cx: &mut Context<Self>) {
        self.show_menu_at(MenuAnchor::Bounds(bounds), items, window, cx);
    }

    fn show_menu_at(&mut self, origin: MenuAnchor, items: Vec<MenuItem>, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        let previous_focus = self.menu.as_ref().filter(|m| m.window == window.window_handle())
            .and_then(|m| m.previous_focus.clone()).or_else(|| window.focused(cx));
        let mut menu = OpenMenu::new(origin, items, window, "上下文菜单".into(), cx);
        menu.previous_focus = previous_focus;
        self.menu = Some(menu);
        window.focus(&self.focus_menu, cx);
        cx.notify();
    }

    pub fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_menu(window, cx);
    }

    fn dismiss_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<OpenMenu> {
        if self.menu.as_ref().is_some_and(|menu| menu.window != window.window_handle()) { return None; }
        let menu = self.menu.take();
        if let Some(menu) = &menu { window.focus(menu.previous_focus.as_ref().unwrap_or(&self.focus_roll), cx); }
        cx.notify();
        menu
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
        use crate::editor::integrations::IntegrationAction as I;
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
            MenuItem::action("试听所选音高", MenuAction::Workflow(A::Integration(I::AuditionSelection))).enabled(has_sel),
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
            MenuItem::boolean_command("当前轨道自动化泳道", CommandId::ToggleTrackAutomation, CommandArg::Track(track), auto_on).with_hint("A"),
            MenuItem::boolean_command("主总线自动化泳道", CommandId::ToggleMasterAutomation, CommandArg::None, self.automation_ui.show_master),
            MenuItem::action("自动化曲线", MenuAction::Workflow(A::Integration(I::Automation))),
            MenuItem::separator(),
            MenuItem::submenu("音符组", vec![
                MenuItem::action("从选中音符创建音符组", MenuAction::Workflow(A::CreatePart)).with_hint("Ctrl+G").enabled(!self.session.selection.is_empty()),
                MenuItem::action("音符组名称…", MenuAction::Workflow(A::RenamePart)).enabled(part),
                MenuItem::action("精确位置…", MenuAction::Workflow(A::MovePart)).enabled(part),
                MenuItem::action("精确裁剪…", MenuAction::Workflow(A::TrimPart)).enabled(part),
                MenuItem::action("音符组静音 / 取消静音", MenuAction::Workflow(A::MutePart)).enabled(part),
                MenuItem::action("在播放头分割音符组", MenuAction::Workflow(A::SplitPart(tick))).enabled(part),
                MenuItem::action("连接右侧音符组", MenuAction::Workflow(A::JoinPart)).enabled(part),
                MenuItem::action("链接复制音符组", MenuAction::Workflow(A::DuplicatePart(true))).enabled(part),
                MenuItem::action("独立复制音符组", MenuAction::Workflow(A::DuplicatePart(false))).enabled(part),
                MenuItem::action("取消音符组链接", MenuAction::Workflow(A::UnlinkPart)).enabled(part),
                MenuItem::action("解散音符组（保留音符）", MenuAction::Workflow(A::DissolvePart)).with_hint("Ctrl+Shift+G").enabled(part),
                MenuItem::action("删除音符组", MenuAction::Workflow(A::DeletePart)).enabled(part),
            ]),
            MenuItem::submenu("音频片段", vec![
                MenuItem::action("导入音频伴奏 / 参考音…", MenuAction::Workflow(A::Integration(I::ImportAudio))),
                MenuItem::action("音频片段列表…", MenuAction::Workflow(A::Integration(I::Clips))),
                MenuItem::action("精确编辑所选音频片段…", MenuAction::Workflow(A::Integration(I::EditClip))).enabled(clip),
                MenuItem::action("复制所选音频片段", MenuAction::Workflow(A::Integration(I::DuplicateClip))).enabled(clip),
                MenuItem::action("删除所选音频片段", MenuAction::Workflow(A::Integration(I::DeleteClip))).enabled(clip),
                MenuItem::action("重新定位丢失音源…", MenuAction::Workflow(A::Integration(I::RelinkClip))).enabled(clip),
                MenuItem::action("授权重新打开已保存音频…", MenuAction::Workflow(A::Integration(I::AuthorizeAudio))),
            ]),
        ]);
        if let Ok(parts) = verdant_domain::parts::Parts::load(self.project()) {
            if !parts.instances.is_empty() {
                items.push(MenuItem::submenu("定位音符组", parts.instances.into_iter()
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
            MenuItem::boolean_command("音素时序泳道", CommandId::TogglePhonemeLane, CommandArg::None, self.layout.show_phoneme_lane),
            MenuItem::boolean_command("BPM 速度泳道", CommandId::ToggleTempoLane, CommandArg::None, self.layout.show_tempo_lane),
            MenuItem::action("添加参数泳道", MenuAction::Workflow(A::Integration(I::ThreeLanes))),
            MenuItem::boolean_command("叠加其他轨道重影音符", CommandId::ToggleOtherTracks, CommandArg::None, self.integrations.other_tracks),
            MenuItem::action("音阶与调式高亮…", MenuAction::Workflow(A::Integration(I::Scale))),
            MenuItem::action("触控布局（平板）", MenuAction::ToggleTouchLayout).checked(self.touch_ui),
            MenuItem::separator(),
            MenuItem::header("走带"),
            MenuItem::boolean_command("循环播放", CommandId::ToggleLoop, CommandArg::None, pb.looping),
            MenuItem::action("以选中音符设为循环范围", MenuAction::Workflow(A::RepeatSelection)).enabled(!self.session.selection.is_empty()),
            MenuItem::action("在播放头设循环起点", MenuAction::Workflow(A::LoopStart(tick))).with_hint("Ctrl+K ["),
            MenuItem::action("在播放头设循环终点", MenuAction::Workflow(A::LoopEnd(tick))).with_hint("Ctrl+K ]"),
            MenuItem::boolean_command("节拍器", CommandId::ToggleMetronome, CommandArg::None, pb.metronome),
            MenuItem::action("当前位置速度 (BPM)…", MenuAction::Workflow(A::EditTempo(tick))),
            MenuItem::action("当前位置拍号…", MenuAction::Workflow(A::EditMeter(tick))),
            MenuItem::separator(),
            MenuItem::submenu("MIDI 输入", vec![
                MenuItem::action("实时监听…", MenuAction::Workflow(A::Integration(I::MonitorPorts))),
                MenuItem::action("选择输入设备并开始录音…", MenuAction::Workflow(A::Integration(I::MidiPorts))).enabled(!recording),
                MenuItem::action("完成并写入录音", MenuAction::Workflow(A::Integration(I::FinishRecording))).enabled(recording),
                MenuItem::action("取消录音", MenuAction::Workflow(A::Integration(I::CancelRecording))).enabled(recording),
            ]),
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
        MenuItem::boolean_command("静音", CommandId::ToggleMute, CommandArg::Track(track.clone()), state.is_some_and(|t| t.muted)).enabled(exists),
        MenuItem::boolean_command("独奏", CommandId::ToggleSolo, CommandArg::Track(track.clone()), state.is_some_and(|t| t.solo)).enabled(exists),
        MenuItem::action("切换轨道颜色", MenuAction::CycleColor(track.clone())).enabled(exists),
        MenuItem::action(if state.is_some_and(|t| t.is_instrument()) { "转为人声轨" } else { "转为乐器轨" },
            MenuAction::MakeInstrumentTrack { track: track.clone(), instrument: !state.is_some_and(|t| t.is_instrument()) }).enabled(exists),
        MenuItem::separator(),
        MenuItem::action("新建人声轨", MenuAction::AddTrack).enabled(can_add),
        MenuItem::action("新建乐器轨", MenuAction::AddInstrumentTrack).enabled(can_add),
        MenuItem::action("删除轨道", MenuAction::DeleteTrack(track.clone())).enabled(exists && project.tracks.len() > 1),
    ]
}

fn collection_revision(ed: &Editor, menu: &OpenMenu, branch_revision: u64) -> String {
    let states = menu.items.iter().map(|item| item.state(ed)).collect::<Vec<_>>();
    format!("{}:{}:{}:{branch_revision}:{:?}:{states:?}", menu.instance, ed.document_generation, ed.revision().get(), menu.items)
}

pub fn context_menu_layer(ed: &Editor, menu: &OpenMenu, window: &mut Window, cx: &mut Context<Editor>) -> impl IntoElement {
    let mut layer = div().absolute().inset_0();
    let mut branch = Some(menu);
    let mut path = Vec::new();
    let mut parent_source = None;
    let leaf_source: CollectionSource = Rc::new(RefCell::new(None));
    while let Some(panel) = branch {
        let (element, source) = menu_panel(ed, menu, panel, path.clone(), parent_source, leaf_source.clone(), window, cx);
        layer = layer.child(gpui::deferred(element).with_priority(path.len()));
        parent_source = Some(source);
        branch = panel.child.as_deref();
        if let Some(index) = panel.expanded { path.push(index); }
    }
    layer
}

type CollectionSource = Rc<RefCell<Option<(verdant_collection_control::CollectionControlHandle, crate::proto_scroll::EngineBinding)>>>;

fn collection_index(handle: &verdant_collection_control::CollectionControlHandle, state: &str) -> Option<usize> {
    handle.state(state).ok().and_then(|value| crate::proto_controls::source_index(Some(&value)))
}

fn menu_panel(ed: &Editor, root: &OpenMenu, menu: &OpenMenu, path: Vec<usize>, parent: Option<CollectionSource>,
    leaf_source: CollectionSource, _window: &mut Window, cx: &mut Context<Editor>) -> (impl IntoElement, CollectionSource) {
    let revision = collection_revision(ed, menu, menu.branch_revision);
    let viewport = menu.scroll_engine.clone();
    let items = menu.items.iter().map(|item| {
        let (enabled, checked) = item.state(ed);
        if !enabled || item.command.is_none() { 0 }
        else if item.is_submenu(ed) { 2 }
        else if let Some(checked) = checked {
            (if matches!(&item.command, Some(MenuCommand::Boolean(_))) { 3 } else { 5 }) + u8::from(checked)
        } else { 1 }
    }).collect();
    let expanded = menu.expanded.map_or(-1, |index| index as i64);
    let props = serde_json::json!({"mode":"menu", "label":menu.label.as_ref(), "cursor":menu.selected,
        "expanded": expanded, "revision":revision, "autoFocus":true, "disabled":false});
    let draw_editor = cx.entity(); let snapshot = menu.clone(); let basis = revision.clone();
    let source: CollectionSource = if menu.child.is_none() { leaf_source.clone() } else { Rc::new(RefCell::new(None)) };
    let source_draw = source.clone();
    let source_sink = source.clone();
    let draw_path = path.clone();
    let outside = path.is_empty().then(|| root.clone());
    let id: SharedString = if path.is_empty() { "compiled-context-menu".into() } else { format!("compiled-submenu-{}", menu.instance).into() };
    let draw = std::rc::Rc::new(move |handle: &verdant_collection_control::CollectionControlHandle, focus: &gpui::FocusHandle, engine: Option<crate::proto_scroll::EngineBinding>, window: &mut Window, cx: &mut gpui::App| {
        let engine = engine.expect("menu viewport binding");
        *source_draw.borrow_mut() = Some((handle.clone(), engine.clone()));
        draw_editor.update(cx, |ed, cx| {
            if let Some(panel) = ed.menu.as_mut().and_then(|root| root.panel_mut(&draw_path)).filter(|panel| panel.instance == snapshot.instance) {
                if draw_path.is_empty() { ed.focus_menu = focus.clone(); }
                if std::mem::take(&mut panel.focus_pending) {
                    let editor = cx.weak_entity();
                    let path = draw_path.clone();
                    let instance = snapshot.instance.clone();
                    let focus = focus.clone();
                    let source = handle.clone();
                    window.on_next_frame(move |window, cx| {
                        let current = editor.upgrade().is_some_and(|editor| {
                            editor.read(cx).menu.as_ref().filter(|root| root.window == window.window_handle())
                                .and_then(|root| root.panel(&path)).is_some_and(|panel| panel.instance == instance && panel.child.is_none())
                        });
                        if current {
                            crate::proto_collection::call(&source, "focusSelf", vec![], window, cx);
                            window.focus(&focus, cx);
                        }
                    });
                }
            }
            native_context_menu_layer(ed, &snapshot, &draw_path, outside.as_ref(), handle.clone(), &leaf_source, basis.clone(), engine, window, cx).into_any_element()
        })
    });
    let editor = cx.entity().downgrade();
    let sink = std::rc::Rc::new(move |name: &str, payload: &serde_json::Value, window: &mut Window, cx: &mut gpui::App| {
        let _ = editor.update(cx, |ed, cx| {
            let Some(menu) = ed.menu.as_ref().filter(|menu| menu.window == window.window_handle()).and_then(|root| root.panel(&path)) else { return };
            let revision = collection_revision(ed, menu, menu.branch_revision);
            if payload.get("revision").and_then(serde_json::Value::as_str) != Some(revision.as_str()) { return; }
            let index = crate::proto_controls::source_integer(payload.get("index")).unwrap_or(-1);
            match name {
                "close" => ed.close_menu(window, cx),
                "activate" if index >= 0 => ed.activate_menu_item(&path, index as usize, None, window, cx),
                "selection" if index >= 0 => {
                    if let Some(selected) = payload.get("selected").and_then(serde_json::Value::as_bool) {
                        ed.activate_menu_item(&path, index as usize, Some(selected), window, cx);
                    }
                },
                "select" if index >= 0 => {
                    if let Some(menu) = ed.menu.as_mut().and_then(|root| root.panel_mut(&path)) { menu.selected = index as usize; }
                    if let Some((handle, engine)) = source_sink.borrow().clone() {
                        crate::proto_collection::reveal_item(handle, index as usize, revision, engine, false, window);
                    }
                    cx.notify();
                },
                "expand" => {
                    let binding = source_sink.borrow();
                    let Some((handle, _)) = binding.as_ref() else { return; };
                    let expanded = usize::try_from(index).ok();
                    if collection_index(handle, "expanded") == expanded {
                        // Input can advance again before delivery. Left restores the invoking
                        // row, whereas a pointer crossing onto a leaf selects its destination.
                        let cursor = collection_index(handle, "cursor");
                        ed.expand_menu_item(&path, expanded, cursor, window, cx);
                    }
                },
                _ => {},
            }
        });
    });
    let collection = crate::proto_collection::collection(id, props, items, draw, sink)
        .scroll_engine(crate::proto_scroll::Engine::Native(viewport))
        .unhandled_key(Rc::new(move |event, window, cx| {
            if event.keystroke.key == "left" && let Some(parent) = &parent
                && let Some((source, _)) = parent.borrow().as_ref() {
                crate::proto_collection::key(source, event, window, cx);
            }
        }));
    (collection, source)
}

fn native_context_menu_layer(ed: &Editor, menu: &OpenMenu, path: &[usize], outside: Option<&OpenMenu>, handle: verdant_collection_control::CollectionControlHandle,
    leaf_source: &CollectionSource, revision: String, engine: crate::proto_scroll::EngineBinding, window: &Window, cx: &mut Context<Editor>) -> impl IntoElement {

    let th = ed.theme;
    let win = window.viewport_size();
    let cascade = matches!(menu.origin, MenuAnchor::Item { .. }) || menu.items.iter().any(|item| item.is_submenu(ed));
    let available_width = if cascade { win.width * 0.6 } else { win.width };
    let width = px(328.0).min((available_width - px(8.0)).max(px(0.0)));
    let mut available_height = win.height - px(8.0);
    if let Some(button) = menu.origin.control_bounds(window) {
        let (above, below) = dropdown_room(button, win.height);
        available_height = available_height.min(above.max(below));
    }
    let height = px(menu.items.iter().map(MenuItem::row_height).sum::<f32>() + 14.0).min(available_height).max(px(0.0));
    // Reserve a check column only in menus that contain toggles.
    let checks = menu.items.iter().any(|item| item.checked.is_some());
    let selected = collection_index(&handle, "cursor").unwrap_or(menu.selected);
    // Native motion may hit the previous layout after the editor has already
    // accepted a sibling expansion. Only the pointer that moved on that layout
    // may reconcile its navigation result across this one kind of revision change.
    let hover = {
        let editor = cx.weak_entity();
        let path = path.to_vec();
        let painted = menu.branch_revision;
        let basis = revision.clone();
        let handle = handle.clone();
        Rc::new(move |row: usize, enabled: bool, submenu: bool, window: &mut Window, cx: &mut App| {
            let open = collection_index(&handle, "expanded");
            let target = (enabled && submenu).then_some(row);
            if open != target && let Some(index) = target.or(open) {
                crate::proto_collection::call(&handle, "choose", vec![index.into(), false.into(), basis.clone().into()], window, cx);
            }
            if enabled && !submenu && collection_index(&handle, "cursor") != Some(row) {
                crate::proto_collection::call(&handle, "selectIndex", vec![row.into(), false.into(), basis.clone().into()], window, cx);
            }
            let _ = editor.update(cx, |ed, cx| {
                let Some(panel) = ed.menu.as_ref().filter(|root| root.window == window.window_handle()).and_then(|root| root.panel(&path)) else { return };
                if panel.branch_revision == painted || collection_revision(ed, panel, painted) != basis { return; }
                {
                    let owner = handle.owner.borrow();
                    if !owner.present || !owner.mounted || owner.disposed || owner.window_id != Some(window.window_handle().window_id())
                        || owner.props.get("revision").and_then(serde_json::Value::as_str) != Some(basis.as_str()) { return; }
                }
                let cursor = collection_index(&handle, "cursor");
                if collection_index(&handle, "expanded") != target || (enabled && cursor != Some(row)) { return; }
                if panel.expanded != target {
                    ed.expand_menu_item(&path, target, cursor, window, cx);
                } else if let Some(cursor) = cursor && panel.selected != cursor {
                    ed.menu.as_mut().and_then(|root| root.panel_mut(&path)).expect("admitted panel").selected = cursor;
                    cx.notify();
                }
            });
        })
    };
    let list = div().id("context-menu")
        .key_context("ContextMenu").occlude()
        .w_full().max_h(height).overflow_y_scroll().track_scroll(&engine.handle(cx))
        .on_scroll_wheel(move |_,window,cx| engine.notify_input(window,cx))
        .p(px(6.0)).rounded(px(12.0)).bg(th.popover).border_1().border_color(th.shade).shadow_lg()
        .text_sm().text_color(th.text)
        .children(menu.items.iter().enumerate().map(|(i, item)| {
            if item.is_separator() {
                return div().h(px(9.0)).mx(px(8.0)).flex().items_center()
                    .child(div().w_full().h(px(1.0)).bg(th.shade)).into_any_element();
            }
            if item.header {
                return div().h(px(22.0)).mx(px(4.0)).px(px(10.0)).when(checks, |d| d.pl(px(28.0)))
                    .flex().items_end().pb(px(3.0)).text_xs().font_weight(gpui::FontWeight::SEMIBOLD).text_color(th.text_secondary)
                    .child(item.label.clone()).into_any_element();
            }
            let (enabled, checked) = item.state(ed);
            let expanded = menu.expanded == Some(i);
            let submenu = expanded || item.is_submenu(ed);
            let child_focus = expanded.then(|| (leaf_source.clone(), cx.weak_entity(), path.to_vec()));
            let choose = handle.clone(); let choose_revision = revision.clone();
            let hover = hover.clone();
            let role = if matches!(&item.command, Some(MenuCommand::Boolean(_))) { gpui::Role::MenuItemCheckBox }
                else if checked.is_some() { gpui::Role::MenuItemRadio } else { gpui::Role::MenuItem };
            crate::proto_controls::action(SharedString::from(format!("menu-item-{i}")), item.label.clone()).role(role)
                .authority(format!("{revision}:{i}"))
                .tab_index(-1)
                .disabled(!enabled)
                .aria_label(item.label.clone())
                .when_some(checked, |d, on| d.aria_selected(on))
                .when(submenu, |action| action.aria_expanded(expanded))
                .when(i == selected && enabled, |d| d.aria_active_descendant().bg(th.hover))
                .h(px(26.0)).px(px(10.0)).flex().items_center().rounded(px(6.0))
                .when(enabled, |d| d.active(move |s| s.bg(crate::ui_controls::pressed_fill(th, th.hover))))
                .text_color(if enabled { th.text } else { th.text_tertiary })
                .when(checks, |d| d.child(div().w(px(18.0)).flex_none().text_color(th.accent).child(if checked == Some(true) { "✓" } else { "" })))
                .child(div().flex_1().truncate().child(item.label.clone()))
                .when(submenu || !item.hint.is_empty(), |d| d.child(div().ml(px(12.0)).flex_none().text_xs().text_color(th.text_secondary)
                    .child(if submenu && item.hint.is_empty() { "›".into() } else { item.hint.clone() })))
                .on_mouse_move(move |_, window, cx| hover(i, enabled, submenu, window, cx))
                .on_click(move |event, window, cx| {
                    if submenu && event.is_mouse() && collection_index(&choose, "expanded") == Some(i) {
                        if let Some((leaf, editor, path)) = &child_focus && let Some(editor) = editor.upgrade() {
                            let ed = editor.read(cx);
                            let current = ed.menu.as_ref().filter(|root| root.window == window.window_handle()).and_then(|root| root.panel(path));
                            if current.is_some_and(|panel| collection_revision(ed, panel, panel.branch_revision) == choose_revision)
                                && let Some((child, _)) = leaf.borrow().as_ref() {
                                crate::proto_collection::call(child, "focusSelf", vec![], window, cx);
                            }
                        }
                    } else {
                        let (method, value) = if checked.is_some() { ("chooseSelection", event.selected()) } else { ("choose", false) };
                        crate::proto_collection::call(&choose, method, vec![i.into(), value.into(), choose_revision.clone().into()], window, cx);
                    }
                    cx.stop_propagation();
                }).into_any_element()
        }))
        .when_some(outside.cloned(), |list, root| list.on_mouse_down_out(move |event, window, cx| {
            if !root.contains(event.position) { crate::proto_collection::call(&handle, "dismiss", vec![], window, cx); }
        }));
    let (popover, border) = (th.popover, th.shade);
    let arrow = Rc::new(Cell::new(None::<(f64, bool)>));
    let paint_arrow = arrow.clone();
    let content = div().absolute().left_0().top_0().w(width).h(height)
        .child(list)
        // The GtkPopover tail, pointing at the anchor.
        .when(matches!(menu.origin, MenuAnchor::Point(_)), |d| d.child(gpui::canvas(|_, _, _| (), move |bounds, (), window, _| {
            let Some((ax, below)) = paint_arrow.get() else { return; };
            let o = if below { bounds.origin } else { bounds.bottom_left() };
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
        }).absolute().inset_0()));
    MenuPosition { content: content.into_any_element(), origin: menu.origin.clone(), arrow }
}

// Each panel prepaints after its parent, with a separate scroll viewport. This
// reads the actual row layout and offset from this frame, including resize and
// scroll, without using a row's mouse coordinates or a cached popup point.
struct MenuPosition {
    content: AnyElement,
    origin: MenuAnchor,
    arrow: Rc<Cell<Option<(f64, bool)>>>,
}

impl Element for MenuPosition {
    type RequestLayoutState = ();
    type PrepaintState = bool;

    fn id(&self) -> Option<gpui::ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.content.request_layout(window, cx), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) -> bool {
        let win = window.viewport_size();
        let place = match &self.origin {
            MenuAnchor::Point(anchor) => popover_placement(*anchor, (bounds.size.width.as_f32(), bounds.size.height.as_f32()), (win.width.as_f32(), win.height.as_f32())),
            MenuAnchor::Bounds(_) | MenuAnchor::Control { .. } => attached_placement(
                self.origin.control_bounds(window).expect("dropdown control bounds"), bounds.size, win, false),
            MenuAnchor::Item { viewport, index } => {
                let Some(mut row) = viewport.bounds_for_item(*index) else { return false; };
                row.origin = row.origin + viewport.offset();
                let parent = viewport.bounds();
                row.origin.y = row.top().max(parent.top()).min(parent.bottom());
                attached_placement(row, bounds.size, win, true)
            }
        };
        self.arrow.set(place.arrow);
        let offset = point(px(place.x), px(place.y)) - bounds.origin;
        window.with_element_offset(offset, |window| self.content.prepaint(window, cx));
        true
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), drawn: &mut bool, window: &mut Window, cx: &mut App) {
        if *drawn { self.content.paint(window, cx); }
    }
}

const MENU_MARGIN: f32 = 4.0;

fn dropdown_room(target: Bounds<Pixels>, height: Pixels) -> (Pixels, Pixels) {
    (target.top() - px(MENU_MARGIN), height - px(MENU_MARGIN) - target.bottom())
}

fn attached_placement(target: Bounds<Pixels>, size: gpui::Size<Pixels>, viewport: gpui::Size<Pixels>, cascade: bool) -> Placement {
    let margin = px(MENU_MARGIN);
    let (x, y) = if cascade {
        let right = viewport.width - margin - target.right();
        let left = target.left() - margin;
        let x = if right >= size.width || right >= left { target.right() } else { target.left() - size.width };
        (x, target.top())
    } else {
        let (above, below) = dropdown_room(target, viewport.height);
        let y = if below >= size.height || below >= above { target.bottom() } else { target.top() - size.height };
        (target.left(), y)
    };
    Placement {
        x: x.min(viewport.width - size.width - margin).max(margin).as_f32(),
        y: y.min(viewport.height - size.height - margin).max(margin).as_f32(),
        arrow: None,
    }
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

#[cfg(test)]
#[path = "menu_selection_tests.rs"]
mod selection_tests;

#[cfg(test)]
mod native_menu_tests {
    use super::*;
    use gpui::{AppContext,Entity,MouseButton,MouseDownEvent,MouseUpEvent,PlatformInput,Render,TestAppContext,WindowHandle,size};
    use verdant_collection_control::{CollectionControlHandle,native::gpui_native_owner::{all_owners,resolved}};
    struct Harness { editor:Entity<Editor> }
    impl Render for Harness {
        fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
            self.editor.update(cx,|ed,cx| {
                div().size_full()
                    .child(crate::proto_controls::action("dropdown", "Dropdown trigger").absolute().left(px(175.)).top(px(50.)).w(px(100.)).h(px(30.))
                        .on_click(cx.listener(|ed, event: &crate::proto_controls::Activation, window, cx| ed.show_menu(event.position(), items(), window, cx)))
                        .on_context_menu({ let editor = cx.entity(); move |position, window, cx| {
                            editor.update(cx, |ed, cx| ed.show_menu(position, items(), window, cx));
                        }}))
                    .when_some(ed.menu.clone(), |d, menu| d.child(context_menu_layer(ed, &menu, window, cx)))
            })
        }
    }
    fn items()->Vec<MenuItem> {vec![
        MenuItem::submenu("Group",vec![MenuItem::disabled("Unavailable"),MenuItem::submenu("Child",vec![MenuItem::action("Pencil",MenuAction::SetTool(crate::editor::Tool::Pencil))])]),
        MenuItem::disabled("Stopped"),
        MenuItem::submenu("Later",vec![MenuItem::action("Pointer",MenuAction::SetTool(crate::editor::Tool::Pointer))]),
    ]}
    fn open(cx:&mut TestAppContext)->WindowHandle<Harness> {
        let window=cx.open_window(size(px(800.),px(500.)),|window,cx| {
            let editor=cx.new(|cx|Editor::for_native_test(crate::native_test_support::instrument_project(),window,cx));
            cx.observe(&editor,|_,_,cx|cx.notify()).detach();
            editor.update(cx,|ed,cx|ed.show_menu(gpui::point(px(80.),px(40.)),items(),window,cx));Harness{editor}
        });
        window.update(cx,|_,window,_|window.activate_window()).unwrap();crate::native_test_support::frames(window,cx);window
    }
    fn source(window:&Window)->CollectionControlHandle {
        named_source(window, "上下文菜单")
    }
    fn named_source(window:&Window, label: &str)->CollectionControlHandle {
        let mut matching = all_owners().into_iter().filter(|owner| {
            let owner = owner.borrow();
            owner.present && owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                && owner.props.get("mode").and_then(serde_json::Value::as_str) == Some("menu")
                && owner.modules.accessible_text.get("name").is_some_and(|value| resolved(value).text() == label)
        });
        let owner = matching.next().unwrap_or_else(|| panic!("no mounted menu collection named {label}"));
        assert!(matching.next().is_none(), "more than one mounted menu collection named {label}");
        CollectionControlHandle { owner }
    }
    fn revision(source:&CollectionControlHandle)->String {source.owner.borrow().props["revision"].as_str().unwrap().into()}
    fn click(window: WindowHandle<Harness>, cx: &mut TestAppContext, position: Point<Pixels>, button: MouseButton) {
        window.update(cx, |_, window, cx| {
            window.dispatch_event(PlatformInput::MouseDown(MouseDownEvent { position, button, click_count: 1, ..Default::default() }), cx);
            window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { position, button, click_count: 1, ..Default::default() }), cx);
        }).unwrap();
        crate::native_test_support::frames(window, cx);
    }

    fn row_geometry(menu: &OpenMenu, index: usize) -> Bounds<Pixels> {
        let mut row = menu.viewport.bounds_for_item(index).unwrap();
        row.origin = row.origin + menu.viewport.offset();
        row
    }

    fn hover_menu(cx: &mut TestAppContext) -> (WindowHandle<Harness>, [Point<Pixels>; 4]) {
        use crate::editor::Tool;
        let window = open(cx);
        window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
            let tool = |name, value| MenuItem::action(name, MenuAction::SetTool(value));
            ed.show_menu(point(px(80.), px(40.)), vec![
                MenuItem::submenu("First", vec![tool("Pencil", Tool::Pencil)]),
                MenuItem::submenu("Second", vec![tool("Pointer", Tool::Pointer), tool("Eraser", Tool::Eraser)]),
                tool("Pitch", Tool::Pitch), tool("Unavailable", Tool::Pencil).enabled(false),
            ], window, cx);
        })).unwrap();
        crate::native_test_support::frames(window, cx);
        let rows = window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            std::array::from_fn(|index| row_geometry(menu, index).center())
        }).unwrap();
        (window, rows)
    }

    fn move_pointer(window: WindowHandle<Harness>, cx: &mut TestAppContext, positions: &[Point<Pixels>]) {
        window.update(cx, |_, window, cx| {
            for &position in positions {
                window.dispatch_event(PlatformInput::MouseMove(gpui::MouseMoveEvent { position, ..Default::default() }), cx);
            }
        }).unwrap();
        crate::native_test_support::frames(window, cx);
    }

    #[gpui::test]
    fn native_menu_hover_switches_siblings_and_click_keeps_child_selection(cx: &mut TestAppContext) {
        let (window, [first, second, _, _]) = hover_menu(cx);
        move_pointer(window, cx, &[first]);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0]).is_some())).unwrap();
        move_pointer(window, cx, &[second]);
        window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            assert!(menu.panel(&[0]).is_none());
            assert!(menu.panel(&[1]).is_some());
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "down");
        crate::native_test_support::frames(window, cx);
        click(window, cx, second, MouseButton::Left);
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.is_none());
            assert_eq!(ed.tool, crate::editor::Tool::Eraser);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_hover_leaf_closes_child_and_enter_activates_destination(cx: &mut TestAppContext) {
        let (window, [first, _, leaf, _]) = hover_menu(cx);
        move_pointer(window, cx, &[first]);
        move_pointer(window, cx, &[leaf]);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().child.is_none())).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.is_none());
            assert_eq!(ed.tool, crate::editor::Tool::Pitch);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_hover_open_child_remains_clickable_after_clicking_parent(cx: &mut TestAppContext) {
        let (window, [_, second, _, _]) = hover_menu(cx);
        move_pointer(window, cx, &[second]);
        click(window, cx, second, MouseButton::Left);
        let eraser = window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            row_geometry(menu.panel(&[1]).expect("Second remains open"), 1).center()
        }).unwrap();
        move_pointer(window, cx, &[eraser]);
        click(window, cx, eraser, MouseButton::Left);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.is_none());
            assert_eq!(ed.tool, crate::editor::Tool::Eraser);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_hover_disabled_closes_child_and_keeps_parent_keyboard_navigation(cx: &mut TestAppContext) {
        let (window, [_, second, _, disabled]) = hover_menu(cx);
        move_pointer(window, cx, &[second]);
        move_pointer(window, cx, &[disabled]);
        click(window, cx, disabled, MouseButton::Left);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.as_ref().unwrap().child.is_none());
            assert_eq!(ed.tool, crate::editor::Tool::Pointer);
        }).unwrap();
        for key in ["enter", "left", "right"] {
            cx.simulate_keystrokes(window.into(), key);
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, _, cx| {
                assert_eq!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[1]).is_some(), key != "left");
            }).unwrap();
        }
    }

    #[gpui::test]
    fn native_menu_queued_hover_uses_the_last_row_before_repaint(cx: &mut TestAppContext) {
        let (window, [first, second, leaf, _]) = hover_menu(cx);
        move_pointer(window, cx, &[first]);
        move_pointer(window, cx, &[second, leaf]);
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.is_none());
            assert_eq!(ed.tool, crate::editor::Tool::Pitch);
        }).unwrap();
    }

    fn move_without_repaint(window: WindowHandle<Harness>, cx: &mut TestAppContext, position: Point<Pixels>) {
        window.update(cx, |_, window, cx| {
            window.dispatch_event(PlatformInput::MouseMove(gpui::MouseMoveEvent { position, ..Default::default() }), cx);
        }).unwrap();
        cx.run_until_parked();
    }

    fn open_sibling_before_repaint(cx: &mut TestAppContext) -> (WindowHandle<Harness>, [Point<Pixels>; 4]) {
        let (window, rows) = hover_menu(cx);
        move_pointer(window, cx, &[rows[0]]);
        move_without_repaint(window, cx, rows[1]);
        window.update(cx, |host, _, cx| {
            let child = host.editor.read(cx).menu.as_ref().unwrap().panel(&[1]).expect("Second's expansion was delivered");
            assert_eq!(child.viewport.bounds().size.width, px(0.), "Second has not been laid out yet");
        }).unwrap();
        (window, rows)
    }

    #[gpui::test]
    fn native_menu_between_frames_hover_selects_leaf_after_delivered_sibling_expansion(cx: &mut TestAppContext) {
        let (window, [_, _, leaf, _]) = open_sibling_before_repaint(cx);
        move_without_repaint(window, cx, leaf);
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().child.is_none())).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.is_none());
            assert_eq!(ed.tool, crate::editor::Tool::Pitch);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_between_frames_disabled_hover_closes_child_and_keeps_invoking_row(cx: &mut TestAppContext) {
        let (window, [_, _, _, disabled]) = open_sibling_before_repaint(cx);
        move_without_repaint(window, cx, disabled);
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().child.is_none())).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.as_ref().unwrap().panel(&[1]).is_some());
            assert_eq!(ed.tool, crate::editor::Tool::Pointer);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_between_frames_click_cannot_activate_a_stale_leaf(cx: &mut TestAppContext) {
        let (window, [first, _, leaf, _]) = open_sibling_before_repaint(cx);
        // Return to the painted row. Its retired buttons must not regain
        // command authority just because that same submenu is open again.
        move_without_repaint(window, cx, first);
        click(window, cx, leaf, MouseButton::Left);
        window.update(cx, |host, _, cx| {
            let ed = host.editor.read(cx);
            assert!(ed.menu.as_ref().unwrap().panel(&[0]).is_some());
            assert_eq!(ed.tool, crate::editor::Tool::Pointer);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_between_frames_hover_cannot_expand_replacement(cx: &mut TestAppContext) {
        let (window, [_, second, _, _]) = open_sibling_before_repaint(cx);
        window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
            let items = ed.menu.as_ref().unwrap().items.clone();
            ed.show_menu(point(px(80.), px(40.)), items, window, cx);
        })).unwrap();
        move_without_repaint(window, cx, second);
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0]).is_some())).unwrap();
    }

    #[gpui::test]
    fn native_menu_queued_hover_cannot_expand_a_replacement_at_the_same_anchor(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| {
            let position = row_geometry(host.editor.read(cx).menu.as_ref().unwrap(), 2).center();
            window.dispatch_event(PlatformInput::MouseMove(gpui::MouseMoveEvent { position, ..Default::default() }), cx);
            host.editor.update(cx, |ed, cx| ed.show_menu(point(px(80.), px(40.)), items(), window, cx));
        }).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().child.is_none())).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0]).is_some())).unwrap();
    }

    #[gpui::test]
    fn native_menu_submenu_topology_retires_rows_and_close_restores_its_parent(cx:&mut TestAppContext) {
        let window=open(cx);
        let before=window.update(cx,|_,window,cx| {let source=source(window);let before=revision(&source);
            source.call("key",vec!["ArrowRight".into(),false.into(),false.into()],window,cx).unwrap();before}).unwrap();
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            assert_eq!(menu.expanded,Some(0));
            let child = menu.child.as_ref().unwrap();
            let row = row_geometry(menu, 0);
            assert_eq!(child.viewport.bounds().top(), row.top());
            assert_eq!(child.viewport.bounds().left(), row.right());
            assert_eq!(named_source(window, "Group").state("focused").unwrap(), serde_json::json!(true));
            let source=source(window);let now=revision(&source);assert_ne!(now,before,"expanded row topology changes authority");
            assert_eq!(source.call("choose",vec![2.into(),false.into(),before.clone().into()],window,cx).unwrap(),serde_json::json!(false));
            let cursor = named_source(window, "Group").state("cursor").unwrap();
            assert_eq!(crate::proto_controls::source_index(Some(&cursor)), Some(1));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "right");
        crate::native_test_support::frames(window,cx);
        window.update(cx, |host, window, cx| {
            assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0, 1]).is_some());
            assert_eq!(named_source(window, "Child").state("focused").unwrap(), serde_json::json!(true));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "left");
        crate::native_test_support::frames(window,cx);
        window.update(cx, |host, window, cx| {
            assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0]).is_some());
            assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0, 1]).is_none());
            assert_eq!(named_source(window, "Group").state("focused").unwrap(), serde_json::json!(true));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "left");
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx| {let ed=host.editor.read(cx);let menu=ed.menu.as_ref().unwrap();assert_eq!(menu.expanded,None);assert_eq!(menu.selected,0);
            assert_eq!(source(window).state("focused").unwrap(), serde_json::json!(true));}).unwrap();
    }

    #[gpui::test]
    fn native_menu_pointer_replaces_siblings_and_dismisses_only_outside_the_branch(cx: &mut TestAppContext) {
        let window = open(cx);
        let (first, second) = window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            (row_geometry(menu, 0).center(), row_geometry(menu, 2).center())
        }).unwrap();
        click(window, cx, first, MouseButton::Left);
        let disabled = window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            row_geometry(menu.panel(&[0]).unwrap(), 0).center()
        }).unwrap();
        click(window, cx, disabled, MouseButton::Left);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.as_ref().unwrap().panel(&[0]).is_some())).unwrap();
        click(window, cx, second, MouseButton::Left);
        window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            assert!(menu.panel(&[0]).is_none());
            assert!(menu.panel(&[2]).is_some());
        }).unwrap();
        click(window, cx, point(px(10.), px(450.)), MouseButton::Left);
        window.update(cx, |host, _, cx| assert!(host.editor.read(cx).menu.is_none())).unwrap();
    }

    #[gpui::test]
    fn native_menu_dropdown_uses_button_bounds_and_context_uses_click_point(cx: &mut TestAppContext) {
        let window = open(cx);
        cx.simulate_keystrokes(window.into(), "escape");
        crate::native_test_support::frames(window, cx);
        let position = point(px(260.), px(60.));
        click(window, cx, position, MouseButton::Left);
        window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            assert_eq!(menu.viewport.bounds().origin, point(px(175.), px(80.)));
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "escape");
        crate::native_test_support::frames(window, cx);
        click(window, cx, position, MouseButton::Right);
        window.update(cx, |host, _, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            assert!(matches!(menu.origin, MenuAnchor::Point(anchor) if anchor == position));
            assert_eq!(menu.viewport.bounds().top(), px(70.));
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_cascade_uses_its_scrolled_row_and_flips_at_the_right_edge(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
            let mut rows = (0..32).map(|index| MenuItem::file(format!("Row {index}"), FileAction::Open)).collect::<Vec<_>>();
            rows.push(MenuItem::submenu("Scrolled", vec![MenuItem::disabled("Child")]));
            ed.show_menu(point(px(780.), px(60.)), rows, window, cx);
        })).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "end");
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "right");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let menu = host.editor.read(cx).menu.as_ref().unwrap();
            assert!(menu.viewport.offset().y < px(0.));
            let row = row_geometry(menu, 32);
            let child = menu.panel(&[32]).unwrap().viewport.bounds();
            assert_eq!(child.right(), row.left());
            assert_eq!(child.top(), row.top().min(window.viewport_size().height - px(4.) - child.size.height));
            assert!(child.bottom() <= window.viewport_size().height);
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_workflow_opens_a_child_and_refreshes_its_settings_in_place(cx: &mut TestAppContext) {
        let window = open(cx);
        window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
            ed.export_options.sample_rate = 96_000;
            let action = MenuAction::Workflow(crate::editor::WorkflowAction::ExportSettings);
            ed.show_menu(point(px(80.), px(40.)), vec![MenuItem::action("Export settings", action)], window, cx);
        })).unwrap();
        crate::native_test_support::frames(window, cx);
        cx.simulate_keystrokes(window.into(), "right");
        crate::native_test_support::frames(window, cx);
        let instance = window.update(cx, |host, _, cx| {
            host.editor.read(cx).menu.as_ref().unwrap().panel(&[0]).unwrap().instance.clone()
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let editor = host.editor.read(cx);
            let child = editor.menu.as_ref().unwrap().panel(&[0]).unwrap();
            assert_eq!(child.instance, instance);
            assert!(child.child.is_none());
            assert_eq!(editor.export_options.sample_rate, 44_100);
            assert_eq!(named_source(window, "Export settings").state("focused").unwrap(), serde_json::json!(true));
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_arrow_selection_reveals_the_selected_row_on_its_actual_engine(cx: &mut TestAppContext) {
        let window = open(cx);
        let retained_owner = window.update(cx, |_, window, _| {
            let source = source(window);
            assert_eq!(source.state("focused").unwrap(), serde_json::json!(true), "the first menu receives source-owned focus");
            source.owner
        }).unwrap();
        window.update(cx, |host, window, cx| host.editor.update(cx, |ed, cx| {
            let rows = (0..40).map(|index| MenuItem::file(format!("Row {index}"), FileAction::Open)).collect();
            ed.show_menu(gpui::point(px(80.0), px(40.0)), rows, window, cx);
        })).unwrap();
        crate::native_test_support::frames(window, cx);
        window.update(cx, |host, window, cx| {
            let source = source(window);
            assert!(std::rc::Rc::ptr_eq(&retained_owner, &source.owner), "replacement keeps the already-mounted collection");
            assert!(host.editor.read(cx).focus_menu.is_focused(window), "show_menu focuses the mounted collection handle");
            assert_eq!(source.state("focused").unwrap(), serde_json::json!(true), "replacement retains physical collection focus before arrow input");
        }).unwrap();
        for _ in 0..24 {
            cx.simulate_keystrokes(window.into(), "down");
            crate::native_test_support::frames(window, cx);
        }
        window.update(cx, |host, window, cx| {
            assert_eq!(host.editor.read(cx).menu.as_ref().unwrap().selected, 24);
            let source = source(window);
            let owner = source.owner.borrow();
            assert_ne!(owner.modules.scroll.external_engine, 0);
            let scroll = owner.modules.scroll.handle.as_ref().expect("menu leases its physical viewport");
            assert!(scroll.offset().y < px(-100.0), "arrow selection reveals a row below the menu viewport");
            let row = scroll.bounds_for_item(24).expect("selected physical menu row");
            let viewport = scroll.bounds();
            assert!(row.top() + scroll.offset().y >= viewport.top() - px(1.0));
            assert!(row.bottom() + scroll.offset().y <= viewport.bottom() + px(1.0));
        }).unwrap();
    }

    #[gpui::test]
    fn native_menu_reopening_same_content_and_anchor_rejects_the_previous_instance(cx:&mut TestAppContext) {
        let window=open(cx);let before=window.update(cx,|_,window,_|revision(&source(window))).unwrap();
        window.update(cx,|host,window,cx|host.editor.update(cx,|ed,cx| {ed.close_menu(window,cx);ed.show_menu(gpui::point(px(80.),px(40.)),items(),window,cx);})).unwrap();
        crate::native_test_support::frames(window,cx);
        window.update(cx,|host,window,cx| {let source=source(window);assert_ne!(revision(&source),before);
            assert_eq!(source.call("choose",vec![0.into(),false.into(),before.into()],window,cx).unwrap(),serde_json::json!(false));
            assert_eq!(host.editor.read(cx).menu.as_ref().unwrap().expanded,None);}).unwrap();
    }
}
