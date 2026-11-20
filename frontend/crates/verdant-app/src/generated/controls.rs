// Greendam.Controls command, numeric, and select bindings.
// Originally emitted from Verdant's verdant-controls-v0.controls.json declaration.
// This checked-in Rust module is the source for the application controls.

#![allow(dead_code)]
use gpui::{prelude::*, div, px, ClickEvent, Context, Div, ElementId, Hsla, SharedString, Stateful, Window};
use verdant_domain::{NoteId, TrackId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind { None, Track, Note, NotePhone, TrackSlot, Text }
/// Clone across listeners: IDs and SharedString retain their own ownership.
/// TrackSlot = (TrackId, zero-based mixer slot). Text audio-device names use
/// the enumerator's spelling; an empty device name means system default.
/// NotePhone = (NoteId, zero-based expanded native phone index), so a single
/// canonical IPA unit that expands to several model phones stays addressable.
#[derive(Clone, Debug, PartialEq)]
pub enum CommandArg { None, Track(TrackId), Note(NoteId), NotePhone(NoteId, u32), TrackSlot(TrackId, u32), Text(SharedString) }
impl CommandArg {
    pub fn kind(&self) -> ArgKind {
        match self { Self::None => ArgKind::None, Self::Track(_) => ArgKind::Track,
            Self::Note(_) => ArgKind::Note, Self::NotePhone(_, _) => ArgKind::NotePhone,
            Self::TrackSlot(_, _) => ArgKind::TrackSlot, Self::Text(_) => ArgKind::Text }
    }
}
/// Every ordinary command declared by source is a Rust variant. Adding one
/// forces exhaustive app-side target matches to be updated at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandId {
    OpenProject,
    AuditionProjects,
    TogglePhonemeTiming,
    ToggleOutputTransients,
    Save,
    SaveAs,
    ExportProject,
    ExportAudio,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    DuplicateSelection,
    DeleteSelection,
    SelectAll,
    EditLyric,
    TogglePlay,
    Stop,
    GoToStart,
    ToggleLoop,
    ToggleMetronome,
    AddTrack,
    ImportAudioClip,
    DeleteActiveTrack,
    ToggleMute,
    ToggleSolo,
    ToggleTrackAutomation,
    ToggleMasterMute,
    ToggleMasterAutomation,
    CycleColor,
    SetToolPointer,
    SetToolPencil,
    SetToolEraser,
    SetToolPitch,
    SetLanePencil,
    SetLaneLine,
    SetLaneSmooth,
    SetLaneEraser,
    SetLaneSelect,
    CopyCurve,
    PasteCurve,
    OpenTransformPoints,
    OpenScaleDialog,
    OpenVibratoDialog,
    ToggleLeftPanel,
    ToggleRightDock,
    TogglePhonemeLane,
    ToggleTempoLane,
    ToggleOtherTracks,
    ToggleThreeLanes,
    ToggleF0Overlay,
    ResetPhonemeDurations,
    TogglePhonemeApproximation,
    FitView,
    FitArrange,
    StartWorker,
    RenderNow,
    PluginAdd,
    PluginBypass,
    PluginRemove,
    PluginOpenEditor,
    ChooseDevice,
    ReopenDevice,
    OpenShortcuts,
    OpenDisclosures,
}
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec { pub id: CommandId, pub label: &'static str, pub arg: ArgKind }
pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec { id: CommandId::OpenProject, label: "Open…", arg: ArgKind::None },
    CommandSpec { id: CommandId::AuditionProjects, label: "Listening Songs…", arg: ArgKind::None },
    CommandSpec { id: CommandId::TogglePhonemeTiming, label: "Vowel On Beat", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleOutputTransients, label: "Final Consonant Transients", arg: ArgKind::None },
    CommandSpec { id: CommandId::Save, label: "Save", arg: ArgKind::None },
    CommandSpec { id: CommandId::SaveAs, label: "Save As…", arg: ArgKind::None },
    CommandSpec { id: CommandId::ExportProject, label: "Export .verp…", arg: ArgKind::None },
    CommandSpec { id: CommandId::ExportAudio, label: "Export WAV…", arg: ArgKind::None },
    CommandSpec { id: CommandId::Undo, label: "Undo", arg: ArgKind::None },
    CommandSpec { id: CommandId::Redo, label: "Redo", arg: ArgKind::None },
    CommandSpec { id: CommandId::Cut, label: "Cut", arg: ArgKind::None },
    CommandSpec { id: CommandId::Copy, label: "Copy", arg: ArgKind::None },
    CommandSpec { id: CommandId::Paste, label: "Paste", arg: ArgKind::None },
    CommandSpec { id: CommandId::DuplicateSelection, label: "Duplicate", arg: ArgKind::None },
    CommandSpec { id: CommandId::DeleteSelection, label: "Delete", arg: ArgKind::None },
    CommandSpec { id: CommandId::SelectAll, label: "Select All", arg: ArgKind::None },
    CommandSpec { id: CommandId::EditLyric, label: "Edit Lyric…", arg: ArgKind::Note },
    CommandSpec { id: CommandId::TogglePlay, label: "Play/Pause", arg: ArgKind::None },
    CommandSpec { id: CommandId::Stop, label: "Stop", arg: ArgKind::None },
    CommandSpec { id: CommandId::GoToStart, label: "Go To Start", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleLoop, label: "Loop Playback", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleMetronome, label: "Metronome", arg: ArgKind::None },
    CommandSpec { id: CommandId::AddTrack, label: "Add Track", arg: ArgKind::None },
    CommandSpec { id: CommandId::ImportAudioClip, label: "Import Audio…", arg: ArgKind::None },
    CommandSpec { id: CommandId::DeleteActiveTrack, label: "Delete Track", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleMute, label: "Mute", arg: ArgKind::Track },
    CommandSpec { id: CommandId::ToggleSolo, label: "Solo", arg: ArgKind::Track },
    CommandSpec { id: CommandId::ToggleTrackAutomation, label: "Track Automation", arg: ArgKind::Track },
    CommandSpec { id: CommandId::ToggleMasterMute, label: "Master Mute", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleMasterAutomation, label: "Master Automation", arg: ArgKind::None },
    CommandSpec { id: CommandId::CycleColor, label: "Color", arg: ArgKind::Track },
    CommandSpec { id: CommandId::SetToolPointer, label: "Pointer Tool", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetToolPencil, label: "Pencil Tool", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetToolEraser, label: "Eraser Tool", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetToolPitch, label: "Pitch Curve Tool", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetLanePencil, label: "Freehand Curve", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetLaneLine, label: "Line Curve", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetLaneSmooth, label: "Smooth Curve", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetLaneEraser, label: "Erase Curve", arg: ArgKind::None },
    CommandSpec { id: CommandId::SetLaneSelect, label: "Select Points", arg: ArgKind::None },
    CommandSpec { id: CommandId::CopyCurve, label: "Copy Curve", arg: ArgKind::None },
    CommandSpec { id: CommandId::PasteCurve, label: "Paste Curve", arg: ArgKind::None },
    CommandSpec { id: CommandId::OpenTransformPoints, label: "Transform Points…", arg: ArgKind::None },
    CommandSpec { id: CommandId::OpenScaleDialog, label: "Scale Highlight…", arg: ArgKind::None },
    CommandSpec { id: CommandId::OpenVibratoDialog, label: "Vibrato Envelope…", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleLeftPanel, label: "Left Track Panel", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleRightDock, label: "Right Property Dock", arg: ArgKind::None },
    CommandSpec { id: CommandId::TogglePhonemeLane, label: "Phoneme Timing Strip", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleTempoLane, label: "Tempo Strip", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleOtherTracks, label: "Ghost Notes", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleThreeLanes, label: "Three Lanes", arg: ArgKind::None },
    CommandSpec { id: CommandId::ToggleF0Overlay, label: "F0 Overlay", arg: ArgKind::None },
    CommandSpec { id: CommandId::ResetPhonemeDurations, label: "Reset Phoneme Durations", arg: ArgKind::Note },
    CommandSpec { id: CommandId::TogglePhonemeApproximation, label: "Phoneme Approximation", arg: ArgKind::Note },
    CommandSpec { id: CommandId::FitView, label: "Fit Notes", arg: ArgKind::None },
    CommandSpec { id: CommandId::FitArrange, label: "Fit Arrangement", arg: ArgKind::None },
    CommandSpec { id: CommandId::StartWorker, label: "Start Engine", arg: ArgKind::None },
    CommandSpec { id: CommandId::RenderNow, label: "Render", arg: ArgKind::None },
    CommandSpec { id: CommandId::PluginAdd, label: "Add Plugin…", arg: ArgKind::TrackSlot },
    CommandSpec { id: CommandId::PluginBypass, label: "Bypass", arg: ArgKind::TrackSlot },
    CommandSpec { id: CommandId::PluginRemove, label: "Remove Plugin", arg: ArgKind::TrackSlot },
    CommandSpec { id: CommandId::PluginOpenEditor, label: "Plugin Editor", arg: ArgKind::TrackSlot },
    CommandSpec { id: CommandId::ChooseDevice, label: "Audio Device", arg: ArgKind::Text },
    CommandSpec { id: CommandId::ReopenDevice, label: "Reconnect Device", arg: ArgKind::None },
    CommandSpec { id: CommandId::OpenShortcuts, label: "Keyboard Shortcuts…", arg: ArgKind::None },
    CommandSpec { id: CommandId::OpenDisclosures, label: "Open-Source Disclosures…", arg: ArgKind::None },
];
pub const fn command(id: CommandId) -> CommandSpec {
    match id {
        CommandId::OpenProject => COMMANDS[0],
        CommandId::AuditionProjects => COMMANDS[1],
        CommandId::TogglePhonemeTiming => COMMANDS[2],
        CommandId::ToggleOutputTransients => COMMANDS[3],
        CommandId::Save => COMMANDS[4],
        CommandId::SaveAs => COMMANDS[5],
        CommandId::ExportProject => COMMANDS[6],
        CommandId::ExportAudio => COMMANDS[7],
        CommandId::Undo => COMMANDS[8],
        CommandId::Redo => COMMANDS[9],
        CommandId::Cut => COMMANDS[10],
        CommandId::Copy => COMMANDS[11],
        CommandId::Paste => COMMANDS[12],
        CommandId::DuplicateSelection => COMMANDS[13],
        CommandId::DeleteSelection => COMMANDS[14],
        CommandId::SelectAll => COMMANDS[15],
        CommandId::EditLyric => COMMANDS[16],
        CommandId::TogglePlay => COMMANDS[17],
        CommandId::Stop => COMMANDS[18],
        CommandId::GoToStart => COMMANDS[19],
        CommandId::ToggleLoop => COMMANDS[20],
        CommandId::ToggleMetronome => COMMANDS[21],
        CommandId::AddTrack => COMMANDS[22],
        CommandId::ImportAudioClip => COMMANDS[23],
        CommandId::DeleteActiveTrack => COMMANDS[24],
        CommandId::ToggleMute => COMMANDS[25],
        CommandId::ToggleSolo => COMMANDS[26],
        CommandId::ToggleTrackAutomation => COMMANDS[27],
        CommandId::ToggleMasterMute => COMMANDS[28],
        CommandId::ToggleMasterAutomation => COMMANDS[29],
        CommandId::CycleColor => COMMANDS[30],
        CommandId::SetToolPointer => COMMANDS[31],
        CommandId::SetToolPencil => COMMANDS[32],
        CommandId::SetToolEraser => COMMANDS[33],
        CommandId::SetToolPitch => COMMANDS[34],
        CommandId::SetLanePencil => COMMANDS[35],
        CommandId::SetLaneLine => COMMANDS[36],
        CommandId::SetLaneSmooth => COMMANDS[37],
        CommandId::SetLaneEraser => COMMANDS[38],
        CommandId::SetLaneSelect => COMMANDS[39],
        CommandId::CopyCurve => COMMANDS[40],
        CommandId::PasteCurve => COMMANDS[41],
        CommandId::OpenTransformPoints => COMMANDS[42],
        CommandId::OpenScaleDialog => COMMANDS[43],
        CommandId::OpenVibratoDialog => COMMANDS[44],
        CommandId::ToggleLeftPanel => COMMANDS[45],
        CommandId::ToggleRightDock => COMMANDS[46],
        CommandId::TogglePhonemeLane => COMMANDS[47],
        CommandId::ToggleTempoLane => COMMANDS[48],
        CommandId::ToggleOtherTracks => COMMANDS[49],
        CommandId::ToggleThreeLanes => COMMANDS[50],
        CommandId::ToggleF0Overlay => COMMANDS[51],
        CommandId::ResetPhonemeDurations => COMMANDS[52],
        CommandId::TogglePhonemeApproximation => COMMANDS[53],
        CommandId::FitView => COMMANDS[54],
        CommandId::FitArrange => COMMANDS[55],
        CommandId::StartWorker => COMMANDS[56],
        CommandId::RenderNow => COMMANDS[57],
        CommandId::PluginAdd => COMMANDS[58],
        CommandId::PluginBypass => COMMANDS[59],
        CommandId::PluginRemove => COMMANDS[60],
        CommandId::PluginOpenEditor => COMMANDS[61],
        CommandId::ChooseDevice => COMMANDS[62],
        CommandId::ReopenDevice => COMMANDS[63],
        CommandId::OpenShortcuts => COMMANDS[64],
        CommandId::OpenDisclosures => COMMANDS[65],
    }
}
pub trait CommandTarget: Sized + 'static {
    fn run_command(&mut self, cmd: CommandId, arg: CommandArg, window: &mut Window, cx: &mut Context<Self>);
}
pub fn dispatch<T: CommandTarget>(target: &mut T, cmd: CommandId, arg: CommandArg, window: &mut Window, cx: &mut Context<T>) {
    assert_eq!(command(cmd).arg, arg.kind(), "generated command argument mismatch");
    target.run_command(cmd, arg, window, cx);
}
pub fn command_button<T: CommandTarget>(cmd: CommandId, arg: CommandArg, label: impl Into<SharedString>,
    key: impl Into<ElementId>, fg: Hsla, bg: Hsla, hover: Hsla, cx: &mut Context<T>) -> Stateful<Div> {
    assert_eq!(command(cmd).arg, arg.kind(), "generated command argument mismatch");
    let label: SharedString = label.into();
    div().id(key).flex().items_center().justify_center().px(px(6.0)).py(px(3.0))
        .rounded(px(3.0)).text_xs().text_color(fg).bg(bg).hover(move |s| s.bg(hover))
        .child(label)
        .on_click(cx.listener(move |target, _: &ClickEvent, window, cx| dispatch(target, cmd, arg.clone(), window, cx)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericId {
    TrackGainDb,
    TrackPan,
    MasterGainDb,
    MasterPan,
    TempoBpm,
    SnapGridTicks,
    DefaultNoteTicks,
    NoteStartTick,
    NoteDurationTicks,
    NotePitchSemitone,
    NoteDetuneCents,
    VibratoPhaseDegrees,
    PortamentoCurvePercent,
    VibratoRateHz,
    VibratoDepthCents,
    VibratoDelayPercent,
    PortamentoMs,
    PhonemeInitialPercent,
    PhonemeFinalPercent,
    PhonemePercent,
}
/// Units come from source: gain is dB, pan is percent (host converts to
/// domain permille). Snap 0 disables snapping. Preview never mutates the
/// ProjectStore; Commit is one transaction and one undo step.
#[derive(Clone, Copy, Debug)]
pub struct NumericSpec {
    pub id: NumericId, pub label: &'static str, pub arg: ArgKind,
    pub min: f64, pub max: f64, pub step: f64, pub fine_step: f64, pub default: f64,
    pub unit: &'static str, pub decimals: usize,
}
pub const NUMERICS: &[NumericSpec] = &[
    NumericSpec { id: NumericId::TrackGainDb, label: "Gain", arg: ArgKind::Track, min: -95.0, max: 6.0, step: 0.5, fine_step: 0.1, default: 0.0, unit: "dB", decimals: 1 },
    NumericSpec { id: NumericId::TrackPan, label: "Pan", arg: ArgKind::Track, min: -100.0, max: 100.0, step: 1.0, fine_step: 0.5, default: 0.0, unit: "%", decimals: 0 },
    NumericSpec { id: NumericId::MasterGainDb, label: "Master Gain", arg: ArgKind::None, min: -96.0, max: 6.0, step: 0.5, fine_step: 0.1, default: 0.0, unit: "dB", decimals: 1 },
    NumericSpec { id: NumericId::MasterPan, label: "Master Pan", arg: ArgKind::None, min: -100.0, max: 100.0, step: 1.0, fine_step: 0.5, default: 0.0, unit: "%", decimals: 0 },
    NumericSpec { id: NumericId::TempoBpm, label: "Tempo", arg: ArgKind::None, min: 40.0, max: 300.0, step: 1.0, fine_step: 0.1, default: 120.0, unit: "bpm", decimals: 1 },
    NumericSpec { id: NumericId::SnapGridTicks, label: "Snap", arg: ArgKind::None, min: 0.0, max: 1920.0, step: 120.0, fine_step: 60.0, default: 240.0, unit: "tick", decimals: 0 },
    NumericSpec { id: NumericId::DefaultNoteTicks, label: "New Note", arg: ArgKind::None, min: 1.0, max: 1920.0, step: 120.0, fine_step: 30.0, default: 480.0, unit: "tick", decimals: 0 },
    NumericSpec { id: NumericId::NoteStartTick, label: "Start", arg: ArgKind::Note, min: 0.0, max: 4294967296.0, step: 120.0, fine_step: 1.0, default: 0.0, unit: "tick", decimals: 0 },
    NumericSpec { id: NumericId::NoteDurationTicks, label: "Length", arg: ArgKind::Note, min: 1.0, max: 4294967296.0, step: 120.0, fine_step: 1.0, default: 480.0, unit: "tick", decimals: 0 },
    NumericSpec { id: NumericId::NotePitchSemitone, label: "Pitch", arg: ArgKind::Note, min: 0.0, max: 127.0, step: 1.0, fine_step: 1.0, default: 60.0, unit: "midi", decimals: 0 },
    NumericSpec { id: NumericId::NoteDetuneCents, label: "Detune", arg: ArgKind::Note, min: -50.0, max: 50.0, step: 1.0, fine_step: 0.1, default: 0.0, unit: "cent", decimals: 1 },
    NumericSpec { id: NumericId::VibratoPhaseDegrees, label: "Vibrato Phase", arg: ArgKind::None, min: 0.0, max: 360.0, step: 1.0, fine_step: 0.1, default: 0.0, unit: "°", decimals: 1 },
    NumericSpec { id: NumericId::PortamentoCurvePercent, label: "Portamento Curve", arg: ArgKind::None, min: -100.0, max: 100.0, step: 5.0, fine_step: 1.0, default: 0.0, unit: "%", decimals: 0 },
    NumericSpec { id: NumericId::VibratoRateHz, label: "Vibrato Rate", arg: ArgKind::None, min: 1.0, max: 12.0, step: 0.1, fine_step: 0.01, default: 5.5, unit: "Hz", decimals: 2 },
    NumericSpec { id: NumericId::VibratoDepthCents, label: "Vibrato Depth", arg: ArgKind::None, min: 0.0, max: 200.0, step: 1.0, fine_step: 0.5, default: 35.0, unit: "cent", decimals: 1 },
    NumericSpec { id: NumericId::VibratoDelayPercent, label: "Vibrato Delay", arg: ArgKind::None, min: 0.0, max: 100.0, step: 5.0, fine_step: 0.1, default: 35.0, unit: "%", decimals: 1 },
    NumericSpec { id: NumericId::PortamentoMs, label: "Portamento", arg: ArgKind::None, min: 0.0, max: 2000.0, step: 20.0, fine_step: 1.0, default: 0.0, unit: "ms", decimals: 0 },
    NumericSpec { id: NumericId::PhonemeInitialPercent, label: "Initial", arg: ArgKind::Note, min: 0.1, max: 99.9, step: 1.0, fine_step: 0.1, default: 22.0, unit: "%", decimals: 1 },
    NumericSpec { id: NumericId::PhonemeFinalPercent, label: "Final", arg: ArgKind::Note, min: 0.1, max: 100.0, step: 1.0, fine_step: 0.1, default: 78.0, unit: "%", decimals: 1 },
    NumericSpec { id: NumericId::PhonemePercent, label: "Phone", arg: ArgKind::NotePhone, min: 0.1, max: 100.0, step: 1.0, fine_step: 0.1, default: 100.0, unit: "%", decimals: 1 },
];
pub const fn numeric(id: NumericId) -> NumericSpec {
    match id {
        NumericId::TrackGainDb => NUMERICS[0],
        NumericId::TrackPan => NUMERICS[1],
        NumericId::MasterGainDb => NUMERICS[2],
        NumericId::MasterPan => NUMERICS[3],
        NumericId::TempoBpm => NUMERICS[4],
        NumericId::SnapGridTicks => NUMERICS[5],
        NumericId::DefaultNoteTicks => NUMERICS[6],
        NumericId::NoteStartTick => NUMERICS[7],
        NumericId::NoteDurationTicks => NUMERICS[8],
        NumericId::NotePitchSemitone => NUMERICS[9],
        NumericId::NoteDetuneCents => NUMERICS[10],
        NumericId::VibratoPhaseDegrees => NUMERICS[11],
        NumericId::PortamentoCurvePercent => NUMERICS[12],
        NumericId::VibratoRateHz => NUMERICS[13],
        NumericId::VibratoDepthCents => NUMERICS[14],
        NumericId::VibratoDelayPercent => NUMERICS[15],
        NumericId::PortamentoMs => NUMERICS[16],
        NumericId::PhonemeInitialPercent => NUMERICS[17],
        NumericId::PhonemeFinalPercent => NUMERICS[18],
        NumericId::PhonemePercent => NUMERICS[19],
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericPhase { Preview, Commit }
pub trait NumericTarget: Sized + 'static {
    fn numeric_value(&self, id: NumericId, arg: CommandArg) -> f64;
    fn set_numeric(&mut self, id: NumericId, arg: CommandArg, value: f64, phase: NumericPhase, cx: &mut Context<Self>);
}
impl NumericSpec {
    pub fn stepped(self, value: f64, direction: f64, fine: bool) -> Option<f64> {
        if !value.is_finite() || !direction.is_finite() { return None; }
        let step = if fine { self.fine_step } else { self.step };
        let next = (value + direction * step).clamp(self.min, self.max);
        (next != value).then_some(next)
    }
}
/// Ordinary numeric shell: minus/plus steppers commit once per click; Shift
/// uses the source fine step. The render owner supplies its current value;
/// this avoids reading an entity already borrowed for rendering. Continuous
/// canvas/fader drags remain named host primitives using the same target.
pub fn numeric_control<T: NumericTarget>(id: NumericId, arg: CommandArg, value: f64,
    key: impl Into<SharedString>, fg: Hsla, cx: &mut Context<T>) -> Stateful<Div> {
    let key: SharedString = key.into();
    let spec = numeric(id);
    assert_eq!(spec.arg, arg.kind(), "generated numeric argument mismatch");
    let label = format!("{} {:.*} {}", spec.label, spec.decimals, value, spec.unit);
    let button = |suffix: &str, direction: f64, text: &'static str| {
        let arg = arg.clone();
        div().id(SharedString::from(format!("{key}-{suffix}"))).px(px(5.0)).py(px(2.0))
            .child(text).on_click(cx.listener(move |target, event: &ClickEvent, _, cx| {
                let value = target.numeric_value(id, arg.clone());
                if let Some(next) = spec.stepped(value, direction, event.modifiers().shift) {
                    target.set_numeric(id, arg.clone(), next, NumericPhase::Preview, cx);
                    target.set_numeric(id, arg.clone(), next, NumericPhase::Commit, cx);
                }
            }))
    };
    let minus = button("minus", -1.0, "−");
    let plus = button("plus", 1.0, "+");
    div().id(key).flex().items_center().text_xs().text_color(fg)
        .child(minus).child(label).child(plus)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectId {
    AppearanceMode,
    CountInBars,
    PitchRangeCents,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectOptionSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
}
#[derive(Clone, Copy, Debug)]
pub struct SelectSpec {
    pub id: SelectId,
    pub label: &'static str,
    pub arg: ArgKind,
    pub default_option: &'static str,
    pub options: &'static [SelectOptionSpec],
}
impl SelectSpec {
    pub fn valid_option(self, option_id: &str) -> Option<SelectOptionSpec> {
        self.options.iter().copied().find(|o| o.id == option_id)
    }
    pub fn option_or_default(self, option_id: &str) -> SelectOptionSpec {
        self.valid_option(option_id)
            .or_else(|| self.valid_option(self.default_option))
            .expect("compiler validated default option")
    }
}
pub const SELECTS: &[SelectSpec] = &[
    SelectSpec { id: SelectId::AppearanceMode, label: "Appearance", arg: ArgKind::None, default_option: "system", options: &[SelectOptionSpec { id: "system", label: "随系统", hint: None }, SelectOptionSpec { id: "light", label: "浅色", hint: None }, SelectOptionSpec { id: "dark", label: "深色", hint: None }] },
    SelectSpec { id: SelectId::CountInBars, label: "Count-In", arg: ArgKind::None, default_option: "off", options: &[SelectOptionSpec { id: "off", label: "预备关", hint: Some("直接起播") }, SelectOptionSpec { id: "one", label: "预备1小节", hint: Some("1 小节节拍预备") }, SelectOptionSpec { id: "two", label: "预备2小节", hint: Some("2 小节节拍预备") }, SelectOptionSpec { id: "four", label: "预备4小节", hint: Some("4 小节节拍预备") }] },
    SelectSpec { id: SelectId::PitchRangeCents, label: "Pitch Bend Range", arg: ArgKind::None, default_option: "c200", options: &[SelectOptionSpec { id: "c100", label: "±100¢", hint: Some("精细半音视图") }, SelectOptionSpec { id: "c200", label: "±200¢", hint: Some("标准全音视图") }, SelectOptionSpec { id: "c1200", label: "±1200¢", hint: Some("八度大跨度滑音") }] },
];
pub const fn select(id: SelectId) -> SelectSpec {
    match id {
        SelectId::AppearanceMode => SELECTS[0],
        SelectId::CountInBars => SELECTS[1],
        SelectId::PitchRangeCents => SELECTS[2],
    }
}
/// Ordinary select shell: validates option membership and default from source,
/// opens or commits choices through an explicit app-side SelectTarget binding.
pub trait SelectTarget: Sized + 'static {
    fn open_select(&mut self, id: SelectId, arg: CommandArg, anchor: gpui::Point<gpui::Pixels>, window: &mut Window, cx: &mut Context<Self>);
    fn set_select(&mut self, id: SelectId, arg: CommandArg, option_id: &'static str, window: &mut Window, cx: &mut Context<Self>);
}
pub fn dispatch_select<T: SelectTarget>(target: &mut T, id: SelectId, arg: CommandArg, option_id: &'static str, window: &mut Window, cx: &mut Context<T>) {
    let spec = select(id);
    assert_eq!(spec.arg, arg.kind(), "generated select argument mismatch");
    let validated = spec.valid_option(option_id).expect("generated select option must be declared in source");
    target.set_select(id, arg, validated.id, window, cx);
}
pub fn select_control<T: SelectTarget>(id: SelectId, arg: CommandArg, current_option: &str,
    key: impl Into<ElementId>, fg: Hsla, bg: Hsla, hover: Hsla, cx: &mut Context<T>) -> Stateful<Div> {
    let spec = select(id);
    assert_eq!(spec.arg, arg.kind(), "generated select argument mismatch");
    let resolved = spec.option_or_default(current_option);
    div().id(key).flex().items_center().px(px(8.0)).py(px(4.0))
        .rounded(px(5.0)).text_xs().text_color(fg).bg(bg).hover(move |s| s.bg(hover))
        .child(div().flex_1().min_w_0().truncate().child(resolved.label))
        .on_click(cx.listener(move |target, ev: &ClickEvent, window, cx| {
            target.open_select(id, arg.clone(), ev.position(), window, cx);
        }))
}

/// Stable semantic control IDs → source pointers (no source line-number ABI).
pub const SOURCE_MAP: &[(&str, &str)] = &[
    ("command.open_project", "/commands/0"),
    ("command.audition_projects", "/commands/1"),
    ("command.toggle_phoneme_timing", "/commands/2"),
    ("command.toggle_output_transients", "/commands/3"),
    ("command.save", "/commands/4"),
    ("command.save_as", "/commands/5"),
    ("command.export_project", "/commands/6"),
    ("command.export_audio", "/commands/7"),
    ("command.undo", "/commands/8"),
    ("command.redo", "/commands/9"),
    ("command.cut", "/commands/10"),
    ("command.copy", "/commands/11"),
    ("command.paste", "/commands/12"),
    ("command.duplicate_selection", "/commands/13"),
    ("command.delete_selection", "/commands/14"),
    ("command.select_all", "/commands/15"),
    ("command.edit_lyric", "/commands/16"),
    ("command.toggle_play", "/commands/17"),
    ("command.stop", "/commands/18"),
    ("command.go_to_start", "/commands/19"),
    ("command.toggle_loop", "/commands/20"),
    ("command.toggle_metronome", "/commands/21"),
    ("command.add_track", "/commands/22"),
    ("command.import_audio_clip", "/commands/23"),
    ("command.delete_active_track", "/commands/24"),
    ("command.toggle_mute", "/commands/25"),
    ("command.toggle_solo", "/commands/26"),
    ("command.toggle_track_automation", "/commands/27"),
    ("command.toggle_master_mute", "/commands/28"),
    ("command.toggle_master_automation", "/commands/29"),
    ("command.cycle_color", "/commands/30"),
    ("command.set_tool_pointer", "/commands/31"),
    ("command.set_tool_pencil", "/commands/32"),
    ("command.set_tool_eraser", "/commands/33"),
    ("command.set_tool_pitch", "/commands/34"),
    ("command.set_lane_pencil", "/commands/35"),
    ("command.set_lane_line", "/commands/36"),
    ("command.set_lane_smooth", "/commands/37"),
    ("command.set_lane_eraser", "/commands/38"),
    ("command.set_lane_select", "/commands/39"),
    ("command.copy_curve", "/commands/40"),
    ("command.paste_curve", "/commands/41"),
    ("command.open_transform_points", "/commands/42"),
    ("command.open_scale_dialog", "/commands/43"),
    ("command.open_vibrato_dialog", "/commands/44"),
    ("command.toggle_left_panel", "/commands/45"),
    ("command.toggle_right_dock", "/commands/46"),
    ("command.toggle_phoneme_lane", "/commands/47"),
    ("command.toggle_tempo_lane", "/commands/48"),
    ("command.toggle_other_tracks", "/commands/49"),
    ("command.toggle_three_lanes", "/commands/50"),
    ("command.toggle_f0_overlay", "/commands/51"),
    ("command.reset_phoneme_durations", "/commands/52"),
    ("command.toggle_phoneme_approximation", "/commands/53"),
    ("command.fit_view", "/commands/54"),
    ("command.fit_arrange", "/commands/55"),
    ("command.start_worker", "/commands/56"),
    ("command.render_now", "/commands/57"),
    ("command.plugin_add", "/commands/58"),
    ("command.plugin_bypass", "/commands/59"),
    ("command.plugin_remove", "/commands/60"),
    ("command.plugin_open_editor", "/commands/61"),
    ("command.choose_device", "/commands/62"),
    ("command.reopen_device", "/commands/63"),
    ("command.open_shortcuts", "/commands/64"),
    ("command.open_disclosures", "/commands/65"),
    ("numeric.track_gain_db", "/numerics/0"),
    ("numeric.track_pan", "/numerics/1"),
    ("numeric.master_gain_db", "/numerics/2"),
    ("numeric.master_pan", "/numerics/3"),
    ("numeric.tempo_bpm", "/numerics/4"),
    ("numeric.snap_grid_ticks", "/numerics/5"),
    ("numeric.default_note_ticks", "/numerics/6"),
    ("numeric.note_start_tick", "/numerics/7"),
    ("numeric.note_duration_ticks", "/numerics/8"),
    ("numeric.note_pitch_semitone", "/numerics/9"),
    ("numeric.note_detune_cents", "/numerics/10"),
    ("numeric.vibrato_phase_degrees", "/numerics/11"),
    ("numeric.portamento_curve_percent", "/numerics/12"),
    ("numeric.vibrato_rate_hz", "/numerics/13"),
    ("numeric.vibrato_depth_cents", "/numerics/14"),
    ("numeric.vibrato_delay_percent", "/numerics/15"),
    ("numeric.portamento_ms", "/numerics/16"),
    ("numeric.phoneme_initial_percent", "/numerics/17"),
    ("numeric.phoneme_final_percent", "/numerics/18"),
    ("numeric.phoneme_percent", "/numerics/19"),
    ("select.appearance_mode", "/selects/0"),
    ("select.count_in_bars", "/selects/1"),
    ("select.pitch_range_cents", "/selects/2"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_declared_numeric_bounds_and_fine_steps_are_observed() {
        for spec in NUMERICS {
            assert!(spec.stepped(f64::NAN, 1.0, false).is_none());
            assert!(spec.stepped(spec.max, 1.0, false).is_none());
            assert!(spec.stepped(spec.min, -1.0, true).is_none());
            let coarse = spec.stepped(spec.min, 1.0, false).unwrap();
            let fine = spec.stepped(spec.min, 1.0, true).unwrap();
            assert!(fine <= coarse && coarse <= spec.max);
            assert_eq!(numeric(spec.id).min, spec.min);
        }
    }
    #[test]
    fn source_declared_select_defaults_and_options_are_observed() {
        for spec in SELECTS {
            assert!(spec.options.len() >= 2);
            let def = spec.valid_option(spec.default_option).expect("default must exist");
            assert_eq!(spec.option_or_default("__missing__").id, def.id);
        }
    }
}
