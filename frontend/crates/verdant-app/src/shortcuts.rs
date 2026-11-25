//! The live keyboard registry and its discoverable, platform-aware shortcut window.
//! Application rows are installed action bindings. Text rows describe the
//! mounted compiled TextField/TextControl, which owns its own native input.

use std::cell::RefCell;

use gpui::{
    Action, App, AppContext, Bounds, Context, FontWeight, IntoElement, KeyBinding, KeyBindingContextPredicate,
    Render, Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, size,
};

use crate::actions::*;
use crate::editor::{Editor, WorkflowAction};
use crate::theme::Theme;

/// Commands whose editor intents already exist but have no unit GPUI action.
/// The exhaustive resolver below keeps shortcuts on the normal journaled edit path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, gpui::Action)]
#[action(namespace = verdant, no_json)]
pub enum WorkflowShortcut {
    CloseGaps, RemoveOverlaps, CharacterLyrics, LyricsLeft, LyricsRight, ContinueSyllable, VibratoShape,
    SelectPoints, ClearPoints, TransformPoints, LinearPoints, HoldPoints, CosinePoints, CubicPoints,
    CreatePart, RenamePart, MovePart, TrimPart, SplitPart, JoinPart, DuplicatePart, LinkedPart, UnlinkPart, MutePart, DeletePart,
    RenameTrack, MoveTrackUp, MoveTrackDown, DuplicateTrack, EditTempo, DeleteTempo, EditMeter, DeleteMeter, LoopStart, LoopEnd,
    NoCountIn, OneBarCountIn, TwoBarCountIn, ExportSettings,
    Integrations, Singers, Phonemizers, Dictionary, AddDictionaryEntry, ImportDictionary, ExportDictionary,
    MidiPorts, FinishRecording, CancelRecording, ImportAudio, AuthorizeAudio, Clips, EditClip, DuplicateClip, DeleteClip, RelinkClip,
    Automation, ExportMidi, ExportUst, ExportUstx, ExportSvp, ExportVsqx, ExportVpr, VocaloidMetadata,
    Scale, OtherTracks, ThreeLanes, AuditionSelection, ExchangeReport,
}

impl WorkflowShortcut {
    fn resolve(self, track: &verdant_domain::TrackId, tick: impl FnOnce() -> i64) -> WorkflowAction {
        use crate::editor::integrations::IntegrationAction as I;
        use verdant_domain::editing::Interpolation;
        use verdant_exchange::Format;
        use WorkflowAction as W;
        use WorkflowShortcut as S;
        match self {
            S::CloseGaps => W::CloseGaps, S::RemoveOverlaps => W::RemoveOverlaps,
            S::CharacterLyrics => W::BatchLyrics(true), S::LyricsLeft => W::ShiftLyrics(false),
            S::LyricsRight => W::ShiftLyrics(true), S::ContinueSyllable => W::ContinueSyllable, S::VibratoShape => W::VibratoShape,
            S::SelectPoints => W::SelectPoints, S::ClearPoints => W::ClearPoints, S::TransformPoints => W::TransformPoints,
            S::LinearPoints => W::Interpolation(Interpolation::Linear), S::HoldPoints => W::Interpolation(Interpolation::Hold),
            S::CosinePoints => W::Interpolation(Interpolation::Cosine), S::CubicPoints => W::Interpolation(Interpolation::Cubic),
            S::CreatePart => W::CreatePart, S::RenamePart => W::RenamePart, S::MovePart => W::MovePart, S::TrimPart => W::TrimPart,
            S::SplitPart => W::SplitPart(tick()), S::JoinPart => W::JoinPart, S::DuplicatePart => W::DuplicatePart(false),
            S::LinkedPart => W::DuplicatePart(true), S::UnlinkPart => W::UnlinkPart, S::MutePart => W::MutePart, S::DeletePart => W::DeletePart,
            S::RenameTrack => W::RenameTrack(track.clone()), S::MoveTrackUp => W::MoveTrack(track.clone(), -1),
            S::MoveTrackDown => W::MoveTrack(track.clone(), 1), S::DuplicateTrack => W::DuplicateTrack(track.clone()),
            S::EditTempo => W::EditTempo(tick()), S::EditMeter => W::EditMeter(tick()),
            S::DeleteTempo => W::DeleteTempo(tick()), S::DeleteMeter => W::DeleteMeter(tick()),
            S::LoopStart => W::LoopStart(tick()), S::LoopEnd => W::LoopEnd(tick()),
            S::NoCountIn => W::CountIn(0), S::OneBarCountIn => W::CountIn(1), S::TwoBarCountIn => W::CountIn(2),
            S::ExportSettings => W::ExportSettings,
            S::Integrations => W::Integration(I::Menu), S::Singers => W::Integration(I::Singers),
            S::Phonemizers => W::Integration(I::Phonemizers),
            S::Dictionary => W::Integration(I::Dictionary), S::AddDictionaryEntry => W::Integration(I::AddDictionaryEntry),
            S::ImportDictionary => W::Integration(I::ImportDictionary), S::ExportDictionary => W::Integration(I::ExportDictionary),
            S::MidiPorts => W::Integration(I::MidiPorts), S::FinishRecording => W::Integration(I::FinishRecording),
            S::CancelRecording => W::Integration(I::CancelRecording), S::ImportAudio => W::Integration(I::ImportAudio),
            S::AuthorizeAudio => W::Integration(I::AuthorizeAudio), S::Clips => W::Integration(I::Clips),
            S::EditClip => W::Integration(I::EditClip), S::DuplicateClip => W::Integration(I::DuplicateClip),
            S::DeleteClip => W::Integration(I::DeleteClip), S::RelinkClip => W::Integration(I::RelinkClip),
            S::Automation => W::Integration(I::Automation), S::ExportMidi => W::Integration(I::ScoreExport(Format::Midi)),
            S::ExportUst => W::Integration(I::ScoreExport(Format::Ust)), S::ExportUstx => W::Integration(I::ScoreExport(Format::Ustx)),
            S::ExportSvp => W::Integration(I::ScoreExport(Format::Svp)), S::ExportVsqx => W::Integration(I::ScoreExport(Format::Vsqx)),
            S::ExportVpr => W::Integration(I::ScoreExport(Format::Vpr)), S::VocaloidMetadata => W::Integration(I::VocaloidMetadata),
            S::Scale => W::Integration(I::Scale), S::OtherTracks => W::Integration(I::OtherTracks),
            S::ThreeLanes => W::Integration(I::ThreeLanes), S::AuditionSelection => W::Integration(I::AuditionSelection),
            S::ExchangeReport => W::Integration(I::Report),
        }
    }
}

pub fn dispatch(ed: &mut Editor, action: &WorkflowShortcut, window: &mut Window, cx: &mut Context<Editor>) {
    let workflow = action.resolve(&ed.session.track, || ed.session.snap.snap(ed.tempo().tick_at(ed.playhead_seconds())).max(0));
    ed.run_workflow(workflow, window, cx);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope { App, Document, Workspace, Arrange, Roll, Notes, Timeline, Escape, Text, Lyric }
impl Scope {
    fn predicate(self) -> &'static str {
        match self {
            Self::App => "Verdant",
            Self::Document => "Verdant && !LyricEditor && !TextPrompt",
            Self::Workspace => "(Arrange || PianoRoll || Mixer || NoteTable) && !LyricEditor && !TextPrompt",
            Self::Arrange => "Arrange && !LyricEditor && !TextPrompt", Self::Roll => "PianoRoll && !LyricEditor && !TextPrompt",
            Self::Notes => "(PianoRoll || NoteTable) && !LyricEditor && !TextPrompt",
            Self::Timeline => "(Arrange || PianoRoll || Mixer) && !LyricEditor && !TextPrompt",
            Self::Escape => "(Arrange || PianoRoll || Mixer || NoteTable || TimelineScrollbar) && !NoteTablePoint && !LyricEditor && !TextPrompt",
            Self::Text => "LyricEditor || TextPrompt", Self::Lyric => "LyricEditor",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::App => "应用", Self::Document => "应用（文本输入外）", Self::Workspace => "工作区", Self::Arrange => "轨道区",
            Self::Roll => "钢琴卷帘", Self::Notes => "钢琴卷帘 / 音符表", Self::Timeline => "轨道区 / 卷帘 / 混音器",
            Self::Escape => "工作区（控制点外）", Self::Text => "歌词 / 文本框", Self::Lyric => "歌词框",
        }
    }
}

struct Shortcut {
    keys: &'static str,
    group: &'static str,
    label: &'static str,
    scope: Scope,
    target: ShortcutTarget,
}

#[derive(Clone, Copy)]
enum ShortcutTarget {
    Application(fn() -> Box<dyn Action>),
    CompiledText(TextCommand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextCommand {
    Submit, Advance, Retreat, Cancel, Undo, Redo, Space, Backspace, Delete,
    Left, Right, SelectLeft, SelectRight, Home, End, SelectAll, Paste,
}

macro_rules! shortcut {
    ($group:literal, ($keys:literal, $scope:ident, $label:literal, text::$command:ident)) => {
        Shortcut { keys: $keys, group: $group, label: $label, scope: Scope::$scope,
            target: ShortcutTarget::CompiledText(TextCommand::$command) }
    };
    ($group:literal, ($keys:literal, $scope:ident, $label:literal, $action:expr)) => {
        Shortcut { keys: $keys, group: $group, label: $label, scope: Scope::$scope,
            target: ShortcutTarget::Application(|| Box::new($action)) }
    };
}

macro_rules! shortcuts {
    ($($group:literal => [$($entry:tt),* $(,)?]),* $(,)?) => {
        // static keeps one canonical table: cheat_sheet rows are verified by
        // pointer identity against these exact action/source entries.
        static SHORTCUTS: &[Shortcut] = &[$($(shortcut!($group, $entry)),*),*];
    };
}

// Single keys keep the established workspace layout. Chords use Ctrl/Cmd+K
// for editor workflows, Ctrl/Cmd+E for score exchange; no leader is a command.
shortcuts![
    "应用与工程" => [
    ("secondary-z", Document, "撤销", Undo),
    ("secondary-shift-z", Document, "重做", Redo),
    ("secondary-y", Document, "重做", Redo),
    ("secondary-shift-h", App, "撤销历史…", crate::commands::ShowHistory),
    ("secondary-n", App, "新建工程", NewProject),
    ("secondary-o", App, "打开工程…", OpenProject),
    ("secondary-s", App, "存储", Save),
    ("secondary-shift-s", App, "另存为…", SaveAs),
    ("secondary-shift-o", App, "最近工程…", RecentProjects),
    ("secondary-alt-o", App, "恢复工程…", RecoverProjects),
    ("secondary-k f r", App, "还原到手动保存版本", RevertProject),
    ("secondary-shift-e", App, "导出工程…", ExportProject),
    ("secondary-alt-e", App, "导出 WAV（含插件）…", ExportAudio),
    ("secondary-enter", App, "立即渲染", RenderNow),
    ("secondary-.", App, "取消渲染", CancelRender),
    ("secondary-shift-r", App, "自动渲染开关", ToggleAutoRender),
    ("secondary-alt-r", App, "重启后端", RestartWorker),
    ("f1", App, "键盘快捷键", ShowShortcuts),
    ("secondary-/", App, "键盘快捷键", ShowShortcuts),
    ("shift-f1", App, "开源软件与研究引用…", ShowDisclosures),
    ("secondary-q", App, "退出 Verdant", Quit),
    ],
    "音符与歌词" => [
    ("secondary-shift-p", Workspace, "全部工作流…", WorkflowMenu),
    ("secondary-c", Workspace, "复制音符", CopyNotes),
    ("secondary-x", Workspace, "剪切音符", CutNotes),
    ("secondary-v", Workspace, "粘贴音符", PasteNotes),
    ("secondary-d", Workspace, "复制一份音符", DuplicateSelection),
    ("secondary-a", Notes, "全选音符", SelectAll),
    ("delete", Roll, "删除选中音符", DeleteSelection),
    ("backspace", Roll, "删除选中音符", DeleteSelection),
    ("secondary-shift-j", Workspace, "在播放头分割音符", SplitNotes),
    ("secondary-j", Workspace, "连接音符", JoinNotes),
    ("q", Roll, "量化到当前网格", QuantizeNotes),
    ("alt-q", Roll, "闭合一格内间隙", WorkflowShortcut::CloseGaps),
    ("shift-q", Roll, "消除重叠", WorkflowShortcut::RemoveOverlaps),
    ("secondary-alt-m", Workspace, "音符静音 / 取消", MuteNotes),
    ("secondary-shift-l", Workspace, "批量歌词…", BatchLyrics),
    ("secondary-k l c", Workspace, "逐字歌词…", WorkflowShortcut::CharacterLyrics),
    ("secondary-k l left", Workspace, "歌词循环左移", WorkflowShortcut::LyricsLeft),
    ("secondary-k l right", Workspace, "歌词循环右移", WorkflowShortcut::LyricsRight),
    ("secondary-k l e", Workspace, "同音节延续", WorkflowShortcut::ContinueSyllable),
    ],
    "走带与速度" => [
    ("space", Workspace, "播放 / 暂停", TogglePlay),
    ("shift-space", Workspace, "停止", Stop),
    ("home", Timeline, "回到开头", GoToStart),
    ("enter", Arrange, "回到开头", GoToStart),
    (",", Workspace, "向前一小节", BackBar),
    (".", Workspace, "向后一小节", ForwardBar),
    ("secondary-l", Workspace, "循环所选音符", RepeatSelection),
    ("c", Workspace, "循环播放开关", ToggleLoop),
    ("shift-c", Workspace, "节拍器开关", ToggleMetronome),
    ("secondary-k [", Workspace, "播放头设为循环起点", WorkflowShortcut::LoopStart),
    ("secondary-k ]", Workspace, "播放头设为循环终点", WorkflowShortcut::LoopEnd),
    ("secondary-k c 0", Workspace, "无预备拍", WorkflowShortcut::NoCountIn),
    ("secondary-k c 1", Workspace, "一小节预备拍", WorkflowShortcut::OneBarCountIn),
    ("secondary-k c 2", Workspace, "两小节预备拍", WorkflowShortcut::TwoBarCountIn),
    ("secondary-k t t", Workspace, "编辑当前位置速度…", WorkflowShortcut::EditTempo),
    ("secondary-k t m", Workspace, "编辑当前位置拍号…", WorkflowShortcut::EditMeter),
    ("secondary-k t delete", Workspace, "删除当前位置速度事件", WorkflowShortcut::DeleteTempo),
    ("secondary-k t backspace", Workspace, "删除当前位置拍号事件", WorkflowShortcut::DeleteMeter),
    ("alt-=", Workspace, "速度增加 1 BPM", TempoUp),
    ("alt--", Workspace, "速度减少 1 BPM", TempoDown),
    ],
    "轨道" => [
    ("secondary-alt-n", App, "新建轨道", NewTrack),
    ("secondary-backspace", Document, "删除轨道", DeleteTrack),
    ("up", Arrange, "上一条轨道", PrevTrack),
    ("down", Arrange, "下一条轨道", NextTrack),
    ("m", Workspace, "轨道静音开关", MuteTrack),
    ("s", Workspace, "轨道独奏开关", SoloTrack),
    ("secondary-k t r", Workspace, "重命名轨道…", WorkflowShortcut::RenameTrack),
    ("secondary-k t up", Workspace, "轨道上移", WorkflowShortcut::MoveTrackUp),
    ("secondary-k t down", Workspace, "轨道下移", WorkflowShortcut::MoveTrackDown),
    ("secondary-k t d", Workspace, "复制轨道", WorkflowShortcut::DuplicateTrack),
    ],
    "显示与工具" => [
    ("i", Workspace, "检查器", ToggleInspector),
    ("e", Workspace, "编辑器", ToggleEditor),
    ("x", Workspace, "混音器", ToggleMixer),
    ("t", Workspace, "键盘音符表", ToggleTable),
    ("1", Workspace, "指针工具", ToolPointer),
    ("2", Workspace, "铅笔工具", ToolPencil),
    ("3", Workspace, "橡皮擦工具", ToolEraser),
    ("4", Workspace, "音高曲线工具", ToolPitch),
    ],
    "钢琴卷帘" => [
    ("left", Roll, "上一个音符", SelectPrev),
    ("right", Roll, "下一个音符", SelectNext),
    ("enter", Roll, "编辑歌词", EditLyric),
    ("escape", Escape, "取消操作 / 清除选择", Escape),
    ("secondary-left", Roll, "左移一格", NudgeLeft),
    ("secondary-right", Roll, "右移一格", NudgeRight),
    ("secondary-up", Roll, "升高半音", NudgeUp),
    ("secondary-down", Roll, "降低半音", NudgeDown),
    ("secondary-shift-left", Roll, "左移一小节", NudgeLeftBar),
    ("secondary-shift-right", Roll, "右移一小节", NudgeRightBar),
    ("secondary-shift-up", Roll, "升高八度", NudgeUpOctave),
    ("secondary-shift-down", Roll, "降低八度", NudgeDownOctave),
    ("secondary-=", Roll, "放大时间轴", ZoomIn),
    ("secondary--", Roll, "缩小时间轴", ZoomOut),
    ("f", Roll, "跟随播放头", ToggleFollow),
    ("p", Roll, "显示 F0", ToggleF0),
    ("v", Roll, "颤音开关", ToggleVibrato),
    ("secondary-k v", Roll, "颤音淡入淡出和相位…", WorkflowShortcut::VibratoShape),
    ("alt-a", Roll, "显式元音 a", VowelA),
    ("alt-i", Roll, "显式元音 i", VowelI),
    ("alt-u", Roll, "显式元音 u", VowelU),
    ("alt-e", Roll, "显式元音 e", VowelE),
    ("alt-o", Roll, "显式元音 o", VowelO),
    ("[", Roll, "网格变粗", GridCoarser),
    ("]", Roll, "网格变细", GridFiner),
    ],
    "参数曲线" => [
    ("secondary-k a a", Workspace, "选择当前参数的全部控制点", WorkflowShortcut::SelectPoints),
    ("secondary-k a c", Workspace, "清除控制点选择", WorkflowShortcut::ClearPoints),
    ("secondary-k a t", Workspace, "变换控制点…", WorkflowShortcut::TransformPoints),
    ("secondary-k a l", Workspace, "线性插值", WorkflowShortcut::LinearPoints),
    ("secondary-k a h", Workspace, "保持插值", WorkflowShortcut::HoldPoints),
    ("secondary-k a s", Workspace, "余弦插值", WorkflowShortcut::CosinePoints),
    ("secondary-k a m", Workspace, "三次插值", WorkflowShortcut::CubicPoints),
    ],
    "声部片段" => [
    ("secondary-k p n", Workspace, "从选中音符创建片段", WorkflowShortcut::CreatePart),
    ("secondary-k p r", Workspace, "片段重命名…", WorkflowShortcut::RenamePart),
    ("secondary-k p m", Workspace, "片段开始位置…", WorkflowShortcut::MovePart),
    ("secondary-k p t", Workspace, "裁剪片段…", WorkflowShortcut::TrimPart),
    ("secondary-k p s", Workspace, "在播放头分割片段", WorkflowShortcut::SplitPart),
    ("secondary-k p j", Workspace, "连接右侧片段", WorkflowShortcut::JoinPart),
    ("secondary-k p d", Workspace, "独立复制片段", WorkflowShortcut::DuplicatePart),
    ("secondary-k p l", Workspace, "链接重复片段", WorkflowShortcut::LinkedPart),
    ("secondary-k p u", Workspace, "取消片段链接", WorkflowShortcut::UnlinkPart),
    ("secondary-k p space", Workspace, "片段静音开关", WorkflowShortcut::MutePart),
    ("secondary-k p delete", Workspace, "删除片段", WorkflowShortcut::DeletePart),
    ],
    "歌手与词典" => [
    ("secondary-k i i", Workspace, "集成工作流…", WorkflowShortcut::Integrations),
    ("secondary-k i s", Workspace, "选择轨道歌手…", WorkflowShortcut::Singers),
    ("secondary-k i p", Workspace, "选择读音器…", WorkflowShortcut::Phonemizers),
    ("secondary-k d d", Workspace, "用户读音词典…", WorkflowShortcut::Dictionary),
    ("secondary-k d n", Workspace, "添加词典词条…", WorkflowShortcut::AddDictionaryEntry),
    ("secondary-k d i", Workspace, "导入词典…", WorkflowShortcut::ImportDictionary),
    ("secondary-k d e", Workspace, "导出词典…", WorkflowShortcut::ExportDictionary),
    ],
    "MIDI、音频与自动化" => [
    ("secondary-k r r", Workspace, "MIDI 输入录音…", WorkflowShortcut::MidiPorts),
    ("secondary-k r enter", Workspace, "完成 MIDI 录音", WorkflowShortcut::FinishRecording),
    ("secondary-k r escape", Workspace, "取消 MIDI 录音", WorkflowShortcut::CancelRecording),
    ("secondary-k b i", Workspace, "导入伴奏 / 参考音…", WorkflowShortcut::ImportAudio),
    ("secondary-k b a", Workspace, "授权已保存音频…", WorkflowShortcut::AuthorizeAudio),
    ("secondary-k b b", Workspace, "选择音频片段…", WorkflowShortcut::Clips),
    ("secondary-k b e", Workspace, "编辑音频片段…", WorkflowShortcut::EditClip),
    ("secondary-k b d", Workspace, "复制音频片段", WorkflowShortcut::DuplicateClip),
    ("secondary-k b delete", Workspace, "删除音频片段", WorkflowShortcut::DeleteClip),
    ("secondary-k b r", Workspace, "重新定位音源…", WorkflowShortcut::RelinkClip),
    ("secondary-shift-a", Workspace, "自动化曲线…", WorkflowShortcut::Automation),
    ],
    "导出与交换" => [
    ("secondary-e m", Workspace, "导出 MIDI…", WorkflowShortcut::ExportMidi),
    ("secondary-e u", Workspace, "导出 UST…", WorkflowShortcut::ExportUst),
    ("secondary-e x", Workspace, "导出 USTX…", WorkflowShortcut::ExportUstx),
    ("secondary-e s", Workspace, "导出 SVP…", WorkflowShortcut::ExportSvp),
    ("secondary-e v", Workspace, "导出 VSQX…", WorkflowShortcut::ExportVsqx),
    ("secondary-e p", Workspace, "导出 VPR…", WorkflowShortcut::ExportVpr),
    ("secondary-e o", Workspace, "音频导出选项…", WorkflowShortcut::ExportSettings),
    ("secondary-e a", Workspace, "Vocaloid 歌手元数据…", WorkflowShortcut::VocaloidMetadata),
    ("secondary-e r", Workspace, "导入 / 导出报告…", WorkflowShortcut::ExchangeReport),
    ],
    "辅助显示与试听" => [
    ("secondary-k s s", Workspace, "音阶高亮设置…", WorkflowShortcut::Scale),
    ("secondary-k s o", Workspace, "其他轨道音符叠加", WorkflowShortcut::OtherTracks),
    ("secondary-k s 3", Workspace, "三个参数面板", WorkflowShortcut::ThreeLanes),
    ("secondary-k s a", Workspace, "试听所选音高（参考正弦音）", WorkflowShortcut::AuditionSelection),
    ],
    "文本输入" => [
    ("enter", Text, "提交文本", text::Submit),
    ("tab", Lyric, "提交并编辑下一个音符", text::Advance),
    ("shift-tab", Lyric, "提交并编辑上一个音符", text::Retreat),
    ("escape", Text, "取消文本编辑", text::Cancel),
    ("secondary-z", Text, "撤销文本编辑", text::Undo),
    ("secondary-shift-z", Text, "重做文本输入", text::Redo),
    ("secondary-y", Text, "重做文本输入", text::Redo),
    ("space", Text, "输入空格", text::Space),
    ("backspace", Text, "删除前一个字符", text::Backspace),
    ("delete", Text, "删除后一个字符", text::Delete),
    ("left", Text, "光标左移", text::Left),
    ("right", Text, "光标右移", text::Right),
    ("shift-left", Text, "向左扩选文本", text::SelectLeft),
    ("shift-right", Text, "向右扩选文本", text::SelectRight),
    ("home", Text, "文本开头", text::Home),
    ("end", Text, "文本末尾", text::End),
    ("secondary-a", Text, "全选文本", text::SelectAll),
    ("secondary-v", Text, "粘贴文本", text::Paste),
    ],
];

pub fn bindings(cx: &App) -> Vec<KeyBinding> {
    SHORTCUTS.iter().filter_map(|s| {
        let ShortcutTarget::Application(action) = s.target else { return None; };
        let predicate = KeyBindingContextPredicate::parse(s.scope.predicate()).expect("static shortcut context");
        Some(KeyBinding::load(s.keys, action(), Some(predicate.into()), false, None, cx).expect("static shortcut keys"))
    }).collect()
}

/// Test the displayed intrinsic commands against real native TextControl
/// input, with the application's actual action bindings installed around it.
#[cfg(test)]
pub(crate) fn compiled_text_shortcuts() -> impl Iterator<Item = (TextCommand, &'static str)> {
    SHORTCUTS.iter().filter_map(|shortcut| match shortcut.target {
        ShortcutTarget::CompiledText(command) => Some((command, shortcut.keys)),
        ShortcutTarget::Application(_) => None,
    })
}

struct ShortcutRow {
    shortcut: &'static Shortcut,
    keys: gpui::SharedString,
}

/// The window renders installed application bindings and native text routes
/// from the same registry, preserving both discoverability and ownership.
fn cheat_sheet(mac: bool) -> Vec<ShortcutRow> {
    SHORTCUTS.iter().map(|shortcut| ShortcutRow { shortcut, keys: display_keys(shortcut.keys, mac).into() }).collect()
}

fn display_keys(keys: &str, mac: bool) -> String {
    let mut output = String::with_capacity(keys.len() + 16);
    for (index, mut stroke) in keys.split_whitespace().enumerate() {
        if index > 0 { output.push_str(" → "); }
        loop {
            if let Some(rest) = stroke.strip_prefix("secondary-") {
                output.push_str(if mac { "⌘+" } else { "Ctrl+" }); stroke = rest;
            } else if let Some(rest) = stroke.strip_prefix("shift-") {
                output.push_str("Shift+"); stroke = rest;
            } else if let Some(rest) = stroke.strip_prefix("alt-") {
                output.push_str(if mac { "⌥+" } else { "Alt+" }); stroke = rest;
            } else { break; }
        }
        output.push_str(stroke);
    }
    output
}

thread_local! {
    static OPEN: RefCell<Option<WindowHandle<Shortcuts>>> = const { RefCell::new(None) };
}

pub struct Shortcuts {
    font: gpui::SharedString,
    rows: Vec<ShortcutRow>,
}

pub fn open(cx: &mut App) {
    if let Some(h) = OPEN.with(|o| *o.borrow()) {
        if h.update(cx, |_, window, _| window.activate_window()).is_ok() { return; }
    }
    let font: gpui::SharedString = crate::theme::system_ui_font(&cx.text_system().all_font_names()).into();
    let bounds = Bounds::centered(None, size(px(760.), px(700.)), cx);
    if let Ok(h) = cx.open_window(
        WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default() },
        |window, cx| {
            window.set_window_title("键盘快捷键 — Verdant");
            cx.new(|_| Shortcuts { font, rows: cheat_sheet(cfg!(target_os = "macos")) })
        },
    ) { OPEN.with(|o| *o.borrow_mut() = Some(h)); }
}

impl Render for Shortcuts {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let th = Theme::for_appearance(window.appearance());
        crate::proto_scroll::viewport("shortcuts", "键盘快捷键", "vertical").size_full().bg(th.surface).text_color(th.text)
            .font_family(self.font.clone()).p(px(20.)).flex().flex_col().gap(px(10.))
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("键盘快捷键"))
            .child(div().text_sm().text_color(th.text_secondary)
                .child("箭头 → 表示依次按键；先松开前一组，再按下一键。单字母仅在工作区生效；输入法组词时由文本框接管。"))
            .children(self.rows.iter().enumerate().map(|(index, row)| {
                let heading = (index == 0 || self.rows[index - 1].shortcut.group != row.shortcut.group)
                    .then(|| div().mt(px(14.)).mb(px(6.)).font_weight(FontWeight::SEMIBOLD).child(row.shortcut.group));
                div().flex().flex_col().children(heading).child(
                    div().id(("shortcut", index)).role(gpui::Role::ListItem).flex().gap(px(16.)).text_sm()
                    .child(div().w(px(250.)).flex_none().text_color(th.text_secondary)
                        .child(row.keys.clone()))
                    .child(div().flex().flex_col().gap(px(2.))
                        .child(row.shortcut.label)
                        .child(div().text_xs().text_color(th.text_secondary).child(row.shortcut.scope.label()))))
            }))
    }
}

#[cfg(test)]
#[path = "shortcut_tests.rs"]
mod tests;
