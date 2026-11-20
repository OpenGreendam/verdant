//! Command registry (SPEC §9). Menus, buttons and keys dispatch the same
//! actions. Keys follow Logic Pro where an equivalent exists (Space, I / E /
//! X, ⌥⌘N, M / S, `,` `.`), Cubase for numbered tools, and the platform
//! modifier (`secondary` = ⌘ on macOS, Ctrl elsewhere) for app commands.
//!
//! Single-letter keys are bound only in the workspace panes. The lyric
//! field is not a descendant of those contexts, so typing never triggers a
//! shortcut, and while an IME is composing it owns Space/Enter/Escape.

use gpui::{App, Menu, MenuItem, actions};

actions!(
    verdant,
    [
        // app
        Undo,
        Redo,
        Save,
        OpenProject,
        SaveAs,
        ExportProject,
        ExportAudio,
        CutNotes,
        CopyNotes,
        PasteNotes,
        DuplicateSelection,
        RenderNow,
        CancelRender,
        ToggleAutoRender,
        ShowShortcuts,
        ShowDisclosures,
        NewTrack,
        DeleteTrack,
        RestartWorker,
        Quit,
        TempoUp,
        TempoDown,
        // workspace
        TogglePlay,
        Stop,
        GoToStart,
        BackBar,
        ForwardBar,
        ToggleInspector,
        ToggleEditor,
        ToggleMixer,
        ToggleTable,
        ToolPointer,
        ToolPencil,
        ToolEraser,
        ToolPitch,
        MuteTrack,
        SoloTrack,
        PrevTrack,
        NextTrack,
        // piano roll
        DeleteSelection,
        SelectPrev,
        SelectNext,
        SelectAll,
        EditLyric,
        Escape,
        NudgeLeft,
        NudgeRight,
        NudgeUp,
        NudgeDown,
        NudgeLeftBar,
        NudgeRightBar,
        NudgeUpOctave,
        NudgeDownOctave,
        ZoomIn,
        ZoomOut,
        ToggleFollow,
        ToggleF0,
        ToggleVibrato,
        VowelA,
        VowelI,
        VowelU,
        VowelE,
        VowelO,
        GridFiner,
        GridCoarser,
        // lyric field
        LyricConfirm,
        LyricNext,
        LyricPrev,
        LyricCancel,
        LyricSpace,
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        Home,
        End,
        SelectAllText,
        PasteText,
        WorkflowMenu,
        NewProject,
        RecentProjects,
        RecoverProjects,
        RevertProject,
        SplitNotes,
        JoinNotes,
        QuantizeNotes,
        MuteNotes,
        BatchLyrics,
        RepeatSelection,
        ToggleLoop,
        ToggleMetronome,
        FileMenu,
    ]
);

/// File-menu entries whose payload (score format or recent path) cannot ride
/// a unit action. They dispatch to the same `menu.rs` flow as the in-window
/// File menu, preserving format admission and the UST export prompts.
#[derive(Clone, Debug, PartialEq, Eq, gpui::Action)]
#[action(namespace = verdant, no_json)]
pub enum FileCommand {
    Import(verdant_exchange::Format),
    Export(verdant_exchange::Format),
    Recent(std::path::PathBuf),
}

/// Native menu selections arrive as app-level actions after the element
/// bubble phase, so the editor is reached through the active window's root.
fn with_editor(cx: &mut App, run: impl FnOnce(&mut crate::editor::Editor, &mut gpui::Window, &mut gpui::Context<crate::editor::Editor>)) {
    let Some(handle) = cx.active_window() else { return };
    let _ = handle.update(cx, |root, window, cx| {
        if let Ok(editor) = root.downcast::<crate::editor::Editor>() {
            editor.update(cx, |ed, cx| run(ed, window, cx));
        }
    });
}

/// Install key bindings and the native File-menu actions. The File menu
/// offers exactly the entries of `Editor::file_menu` (`menu.rs`); these
/// listeners run the same commands from OS menu selections.
pub fn bind(cx: &mut App) {
    let bindings = crate::shortcuts::bindings(cx);
    cx.bind_keys(bindings);
    cx.on_action(|_: &FileMenu, cx| with_editor(cx, |ed, window, cx| ed.file_menu(window, cx)));
    cx.on_action(|action: &FileCommand, cx| {
        let command = match action {
            FileCommand::Import(format) => crate::menu::FileAction::Import(*format),
            FileCommand::Export(format) => crate::menu::FileAction::Export(*format),
            FileCommand::Recent(path) => crate::menu::FileAction::Recent(path.clone()),
        };
        with_editor(cx, |ed, window, cx| ed.run_file_action(command, window, cx));
    });
}

/// Application menus (native menu bar on macOS).
pub fn menus() -> Vec<Menu> {
    let prefs = crate::files::load_prefs();
    let mut file_items = vec![MenuItem::action("新建工程",NewProject),MenuItem::action("最近工程…",RecentProjects),MenuItem::action("恢复工程…",RecoverProjects),MenuItem::action("还原到保存版本",RevertProject),MenuItem::separator(),MenuItem::action("打开…", OpenProject), MenuItem::action("存储", Save),
        MenuItem::action("另存为…", SaveAs), MenuItem::separator()];
    let imports = crate::menu::SCORE_FORMATS.map(|(format, name)|
        MenuItem::action(format!("从 {name} 导入…"), FileCommand::Import(format)));
    let exports = crate::menu::SCORE_FORMATS.map(|(format, name)|
        MenuItem::action(format!("导出成 {name}…"), FileCommand::Export(format)));
    file_items.push(MenuItem::submenu(Menu::new("导入").items(imports)));
    file_items.push(MenuItem::submenu(Menu::new("导出").items(exports)));
    file_items.extend([MenuItem::separator(), MenuItem::action("导出 Verdant 工程 (.verp)…", ExportProject),
        MenuItem::action("导出 WAV（含插件）…", ExportAudio), MenuItem::separator()]);
    let recent = crate::menu::recent_project_paths(&prefs);
    if recent.is_empty() {
        file_items.push(MenuItem::action("尚无最近文件", FileMenu).disabled(true));
    }
    for path in recent {
        file_items.push(MenuItem::action(path.display().to_string(), FileCommand::Recent(path)));
    }
    file_items.extend([MenuItem::separator(), MenuItem::action("完整文件菜单…", FileMenu)]);
    vec![
        Menu {
            name: "Verdant".into(),
            items: vec![MenuItem::action("键盘快捷键", ShowShortcuts), MenuItem::action("开源软件与研究引用…", ShowDisclosures), MenuItem::separator(), MenuItem::action("退出 Verdant", Quit)],
            disabled: false,
        },
        Menu { name: "文件".into(), items: file_items, disabled: false },
        Menu {
            name: "编辑".into(),
            items: vec![
                MenuItem::action("撤销", Undo),
                MenuItem::action("重做", Redo),
                MenuItem::separator(),
                MenuItem::action("剪切音符", CutNotes),
                MenuItem::action("复制音符", CopyNotes),
                MenuItem::action("粘贴音符", PasteNotes),
                MenuItem::action("复制一份", DuplicateSelection),
                MenuItem::action("全选", SelectAll),
                MenuItem::action("删除", DeleteSelection),
                MenuItem::action("分割音符",SplitNotes),MenuItem::action("连接音符",JoinNotes),MenuItem::action("量化音符",QuantizeNotes),MenuItem::action("音符静音",MuteNotes),MenuItem::action("批量歌词…",BatchLyrics),MenuItem::action("全部工作流…",WorkflowMenu),
            ],
            disabled: false,
        },
        Menu {
            name: "轨道".into(),
            items: vec![
                MenuItem::action("新建轨道", NewTrack),
                MenuItem::action("删除轨道", DeleteTrack),
                MenuItem::separator(),
                MenuItem::action("静音", MuteTrack),
                MenuItem::action("独奏", SoloTrack),
            ],
            disabled: false,
        },
        Menu {
            name: "显示".into(),
            items: vec![
                MenuItem::action("检查器", ToggleInspector),
                MenuItem::action("编辑器", ToggleEditor),
                MenuItem::action("混音器", ToggleMixer),
                MenuItem::action("键盘音符表", ToggleTable),
            ],
            disabled: false,
        },
        Menu {
            name: "走带".into(),
            items: vec![
                MenuItem::action("播放 / 暂停", TogglePlay),
                MenuItem::action("停止", Stop),
                MenuItem::action("回到开头", GoToStart),
                MenuItem::action("循环选择",RepeatSelection),MenuItem::action("循环播放",ToggleLoop),MenuItem::action("节拍器",ToggleMetronome),
            ],
            disabled: false,
        },
        Menu {
            name: "渲染".into(),
            items: vec![
                MenuItem::action("立即渲染", RenderNow),
                MenuItem::action("取消当前任务", CancelRender),
                MenuItem::action("自动渲染", ToggleAutoRender),
                MenuItem::separator(),
                MenuItem::action("重启后端", RestartWorker),
            ],
            disabled: false,
        },
    ]
}

