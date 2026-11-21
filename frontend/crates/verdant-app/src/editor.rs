//! The root view state. Owns the single-writer `ProjectStore`, the
//! `EditorSession`, the render scheduler + worker connection and the audio
//! control handle, and wires them together. GPUI rendering lives in
//! `view.rs` / `roll.rs`; this file is behaviour.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{App, AppContext, Context, FocusHandle, Window};
use verdant_audio::device::{DeviceEvent, DeviceSelection, OutputEngine};
use verdant_audio::{Clip, Control, OutputInfo, Timeline, Transport};
use verdant_backend::adapter::{self, normalize_pinyin};
use verdant_backend::cache::ArtifactCache;
use verdant_backend::params::{self, Param};
use verdant_backend::{Action, PhrasePlan, RenderScheduler, Route, WorkerClient, WorkerConfig, WorkerEvent, WorkerMessage};
use verdant_domain::model::{Note, Project, Track, TrackHeader};
use verdant_domain::persistence::ProjectPersistence;
use verdant_domain::{
    Command, CommandBatch, NoteId, ProjectStore, Pronunciation, PronunciationSource, Revision, TRACK_COLORS, TempoMap, TrackId,
    Vibrato,
};
use verdant_session::{EditorSession, Effect, LyricOutcome, LyricSession, Viewport, parse_lyric_input};

use crate::theme::Theme;

#[path = "plugin_ui.rs"]
mod plugin_ui;
#[path = "phonemes.rs"]
mod phonemes;
#[path = "curves.rs"]
mod curves;
#[path = "project_audio.rs"]
pub(crate) mod project_audio;
#[path = "integrations.rs"]
pub(crate) mod integrations;
#[path = "voice_ui.rs"]
pub(crate) mod voice_ui;
#[path = "workflow.rs"]
pub(crate) mod workflow;
pub use workflow::{WorkflowAction, PromptPurpose, TextPrompt};

/// Editing tool (Logic's Tool menu / Cubase's tool buttons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pointer,
    Pencil,
    Eraser,
    Pitch,
}

/// What the lower pane shows (Logic: E toggles the editor, X the mixer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bottom {
    Editor,
    Mixer,
    /// Keyboard note table (TODO §3/§5): the AccessKit-bearing surface.
    Table,
    /// The selected track's effect chain as device cards.
    Devices,
    Hidden,
}

/// How pointer strokes in a parameter lane become points (TODO §3 curve
/// tools): freehand pencil, straight line between endpoints, smooth (binomial
/// filtered), eraser. `Paste` applies the curve clipboard; `line`/`smooth`
/// are stroke modes, `copy`/`paste` are actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LaneTool {
    #[default]
    Pencil,
    Line,
    Smooth,
    Eraser,
    Select,
}

/// Curve clipboard payload. Separate note-relative segments preserve held
/// endpoints, gaps and each note's implicit zero PIT / unity DYN (zero dB).
#[derive(Clone, Debug)]
pub struct CurveClipboard {
    pub param: Param,
    pub ticks: i64,
    pub segments: Vec<CurveSegment>,
}

#[derive(Clone, Debug)]
pub struct CurveSegment {
    /// Offset from the clipboard origin to this source note.
    pub start_tick: i64,
    pub ticks: i64,
    /// Note-relative explicit points in cents / dB; empty means the default.
    pub samples: Vec<(i64, f64)>,
}

/// Context-menu commands; `menu.rs` renders entries, `Editor::run_menu_action`
/// executes them. All of them are real project or session edits.
#[derive(Clone, Debug)]
pub enum MenuAction {
    Undo,
    Redo,
    Save,
    SaveAs,
    OpenProject,
    ChooseExample,
    OpenBuiltinExample(String),
    ExampleCredits,
    OpenAudition(PathBuf),
    ExportProject,
    Cut,
    Copy,
    /// Paste the note clipboard at the given tick (menu position or playhead).
    Paste,
    PasteAtTick(i64),
    DeleteSelection,
    SelectAll,
    Duplicate,
    EditLyric(NoteId),
    AddNoteAt(i64, i32),
    SetTool(Tool),
    SetLaneTool(LaneTool),
    CopyCurve,
    PasteCurveAt(i64),
    /// Append a master-bus insert: `format` is the plugin format tag.
    AddMasterInsert { format: String, id: String },
    AddTrack,
    DeleteTrack(TrackId),
    MuteTrack(TrackId),
    SoloTrack(TrackId),
    CycleColor(TrackId),
    SeekTick(i64),
    /// Insert-slot actions; `slot` = index inside the track's insert list.
    AddInsert { track: TrackId, format: String, id: String },
    BypassInsert { track: TrackId, slot: usize },
    RemoveInsert { track: TrackId, slot: usize },
    OpenInsertEditor { track: TrackId, slot: usize },
    /// Route another track's signal to an insert's sidechain (`None` = off).
    SetSidechain { track: TrackId, slot: usize, source: Option<TrackId> },
    AddInstrumentTrack,
    /// Make a vocal track an instrument track or back.
    MakeInstrumentTrack { track: TrackId, instrument: bool },
    SetInstrumentPreset { track: TrackId, preset: String },
    /// Set parameters of a track insert as one undo step.
    SetInsertParams { track: TrackId, slot: usize, values: Vec<(String, f64)> },
    RescanPlugins,
    /// Select an audio device by name (empty = system default, `None` text →
    /// silent clock is never offered here; `--silent-audio` starts it).
    ChooseDevice(String),
    ReopenDevice,
    SetGridTicks { snap: bool, ticks: i64 },
    ToggleTrackAutomation(TrackId),
    ToggleMasterAutomation,
    SelectAutomationTarget { track: Option<TrackId>, target: verdant_domain::automation::AutomationTarget },
    AutomationPointInterp { target: verdant_domain::automation::AutomationTarget, tick: i64, interpolation: verdant_domain::automation::Interpolation },
    DeleteAutomationPoint { target: verdant_domain::automation::AutomationTarget, tick: i64 },
    TogglePhonemeLane,
    ToggleTempoLane,
    SetTrackLanguage { track: TrackId, language: Option<String> },
    GeneratedSelect {
        id: crate::generated::controls::SelectId,
        arg: crate::generated::controls::CommandArg,
        option: &'static str,
    },
    ShowHistoryWindow,
    ShowShortcutsWindow,
    ShowDisclosuresWindow,
    /// Open one of the toolbar menus (from the folded menu in narrow windows).
    OpenMenu(TopMenu),
    /// Show the left inspector (the selection's properties live there).
    ShowProperties,
    ToggleFollow,
    ShowBottom(Bottom),
    /// Run a toolbar command that is folded away in narrow windows.
    Run(crate::generated::controls::CommandId),
    Workflow(WorkflowAction),
}

/// The toolbar's menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopMenu { File, Edit, Track, VoiceLyrics, ViewTransport }

/// A pencil stroke in a parameter lane, not yet committed.
#[derive(Clone, Debug)]
pub struct LaneStroke {
    pub param: Param,
    pub base: Revision,
    pub pointer: verdant_session::PointerId,
    pub tool: LaneTool,
    pub note: Option<NoteId>,
    pub point: Option<verdant_domain::PointId>,
    pub roll: bool,
    pub origin_screen: (f64, f64),
    pub axis: Option<bool>,
    /// (absolute tick, value) samples in input order.
    pub samples: Vec<(f64, f64)>,
    pub erase: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixerControl {
    Fader,
    Pan,
}

/// A fader / pan drag in the mixer: previewed on the audio atomics, one
/// transaction on release.
#[derive(Clone, Debug)]
pub struct MixerDrag {
    pub control: MixerControl,
    pub track: TrackId,
    pub header: TrackHeader,
    pub base: Revision,
    /// Pointer coordinate and control value (0…1) at the press.
    pub origin_pos: f64,
    pub origin_value: f64,
}

#[derive(Clone, Debug)]
pub struct MasterMixerDrag {
    pub control: MixerControl,
    pub settings: crate::mix::MasterBusSettings,
    pub base: Revision,
    pub origin_pos: f64,
    pub origin_value: f64,
}

pub const REPLAN_DEBOUNCE: Duration = Duration::from_millis(180);
pub const VOWEL_PROFILE: &str = "english-vowel-proxy-native-0.2";
pub const PINYIN_PROFILE: &str = verdant_domain::HANZI_PINYIN_PROFILE;

#[derive(Clone, Debug, PartialEq)]
pub enum WorkerStatus {
    NotStarted,
    Starting,
    Ready { session_id: String },
    Exited(String),
    SpawnFailed(String),
}

pub struct Notice {
    pub at: Instant,
    pub text: String,
}

/// Startup/shutdown inputs gathered once in `main`.
#[derive(Clone, Debug)]
pub struct EditorOpts {
    /// Explicit project path (may not exist yet).
    pub project_path: Option<PathBuf>,
    pub audition_projects: Vec<crate::auditions::AuditionProject>,
    /// Persisted preferences (window bounds, recent dirs, device).
    pub prefs: crate::files::Prefs,
    /// Plugin host binary path (packaging: VERDANT_PLUGIN_HOST).
    pub plughost_path: Option<PathBuf>,
}

pub struct Editor {
    /// The only writable store. A document is opened only after recovery and
    /// its durable journal have been admitted successfully.
    pub persistence: ProjectPersistence,
    pub project_path: Option<PathBuf>,
    pub audition_projects: Vec<crate::auditions::AuditionProject>,
    pub session: EditorSession,
    pub lyric: Option<LyricSession>,
    pub focus_roll: FocusHandle,
    pub focus_lyric: FocusHandle,
    pub focus_table: FocusHandle,
    pub focus_menu: FocusHandle,

    pub worker_cfg: WorkerConfig,
    pub worker: Option<WorkerClient>,
    pub worker_gen: u64,
    pub worker_status: WorkerStatus,
    pub worker_workspace: Option<PathBuf>,
    pub scheduler: RenderScheduler,
    pub plans: Vec<PhrasePlan>,
    scheduled_phonemes: phonemes::ScheduledPhonemes,
    pub phrase_slots: HashMap<(TrackId, usize), usize>,
    pub render_audio: HashMap<(TrackId, usize), verdant_backend::VerifiedAudio>,
    pub arrange_cache: crate::arrange::ArrangeCache,
    pub auto_render: bool,
    pub render_armed: bool,
    replan_gen: u64,
    pub malformed: Vec<String>,

    pub audio: Control,
    /// Owns the device/silent-clock driver threads; `audio` stays valid
    /// across device reselection (same Control/RtState).
    pub engine: OutputEngine,
    /// Negotiated mixer/timeline rate for renders (fixed 48k for the engine
    /// lifetime; `device_sample_rate` is only a display fact).
    pub audio_info: OutputInfo,
    phrase_pcm: project_audio::PhrasePcmCache,
    timeline_gen: u64,
    pub timeline_phrases: usize,
    /// Instrument tracks with rendered notes in the current timeline.
    pub timeline_instruments: usize,
    cache_restore_gen: u64,
    cache_restoring: bool,

    pub notices: VecDeque<Notice>,
    pub show_f0: bool,
    /// Last shaped lyric line and its bounds, for IME candidate placement.
    pub lyric_line: Option<gpui::ShapedLine>,
    pub lyric_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    /// Fit the viewport to the material on the first laid-out frame.
    pub needs_fit: bool,
    /// Audio clock sampled once per frame so every canvas draws the same playhead.
    pub frame_playhead: f64,
    /// Smooths the buffer-stepped audio clock into a steady playhead.
    pub playhead_smoother: crate::clock::PlayheadSmoother,
    pub ui_font: gpui::SharedString,
    /// Bounded note lookup index for the active track (rebuilt per revision).
    pub index: crate::index::NoteIndex,
    pub language_segments: crate::index::LanguageSegments,
    active_param_data: std::cell::OnceCell<[bool; params::ALL.len()]>,
    phone_budgets: std::cell::RefCell<HashMap<NoteId, Result<verdant_backend::phoneme_timing::PhonemeBudget, String>>>,
    resolved_readings: std::cell::RefCell<Option<verdant_backend::phonemizer::ResolvedTrack>>,
    pub roll_pitch: crate::roll::RenderedPitchCache,
    pub ready_notes: HashSet<NoteId>,
    pub error_notes: HashSet<NoteId>,
    pub playback: crate::clock::PlaybackViews,
    pub static_view: gpui::Entity<crate::view::StaticEditor>,
    pub(crate) chrome: crate::view::ChromeViews,
    pub(crate) compiled_roll: gpui::Entity<verdant_pitch_gesture_surface::PitchGestureSurface>,
    pub(crate) compiled_pitch_move: crate::proto_roll::PitchMove,

    pub theme: Theme,
    pub tool: Tool,
    /// Active pointer-mode for curve lanes (pencil/line/smooth/eraser).
    pub lane_tool: LaneTool,
    pub bottom: Bottom,
    pub show_inspector: bool,
    /// Current window-width class (updated every render).
    pub width_class: crate::ui_controls::WidthClass,
    /// The side panel open as a drawer while panels are not docked.
    pub drawer: Option<crate::ui_controls::Drawer>,
    /// Touch input has been used: toolbar controls grow to touch size.
    pub touch_ui: bool,
    /// How the toolbar acts as the window title bar (updated every render).
    pub window_chrome: crate::window_chrome::ChromeState,
    /// A press on an empty toolbar area that becomes a window move on drag.
    pub title_drag: bool,
    /// A knob or curve handle being dragged in the device view.
    pub device_drag: Option<crate::devices::DeviceDrag>,
    /// The EQ band shown in the device view's band strip.
    pub device_band: usize,
    /// The saved insert state each cached parameter list was read against.
    pub device_param_state: HashMap<verdant_plugins::InsertKey, String>,
    /// Curve/handle animations of device editors after parameter changes.
    pub device_anim: HashMap<verdant_plugins::InsertKey, crate::devices::DeviceAnim>,
    pub param: Param,
    /// PIT lane vertical range, ± cents (VOCALOID PBS-like zoom).
    pub pit_range_cents: f64,
    pub lane_stroke: Option<LaneStroke>,
    pub mixer_drag: Option<MixerDrag>,
    /// Decayed peak meters per track and the master (UI-side ballistics).
    pub meters: Vec<(f32, f32)>,
    pub master_meter: (f32, f32),
    /// Tracks area horizontal view (independent of the editor's zoom).
    pub arrange: Viewport,
    pub arrange_track_height: f64,
    pub needs_arrange_fit: bool,
    pub focus_arrange: FocusHandle,
    /// `Some(true)` dark, `Some(false)` light, `None` follow the system.
    pub appearance_override: Option<bool>,

    // ---- TODO follow-up state -------------------------------------------------
    /// Application preferences (loaded in main, flushed by the slow pump).
    pub prefs: crate::files::Prefs,
    pub(crate) prefs_dirty: bool,
    /// Open context menu, if any.
    pub menu: Option<crate::menu::OpenMenu>,
    /// Copied notes for paste (deep copies; ids regenerated on paste).
    pub note_clipboard: Vec<Note>,
    /// Copied lane samples for curve paste.
    pub curve_clipboard: Option<CurveClipboard>,
    /// Keyboard note table cursor (row = position in the track's note order).
    pub table_cursor: usize,
    pub table_anchor: Option<NoteId>,
    pub table_focus: crate::table::TableFocus,
    pub table_scroll: gpui::UniformListScrollHandle,
    pub point_scroll: gpui::UniformListScrollHandle,
    pub numeric_preview: Option<(crate::generated::controls::NumericId, crate::generated::controls::CommandArg, f64)>,
    /// Plugin host client and live insert state per track.
    pub plugins: Option<Arc<verdant_plugins::PluginClient>>,
    pub(crate) insert_runtime: plugin_ui::InsertRuntime,
    pub inserts: HashMap<TrackId, Vec<verdant_plugins::InsertSlot>>,
    pub inserts_loading: bool,
    insert_specs_applied: HashMap<TrackId, Vec<verdant_plugins::InsertSpec>>,
    insert_gen: u64,
    pub plugin_params: HashMap<verdant_plugins::InsertKey, Vec<verdant_plugins::ParamDesc>>,
    pub plugin_busy: HashSet<verdant_plugins::InsertKey>,
    plugin_counters: HashMap<verdant_plugins::InsertKey, (u64, u32)>,
    plugin_reconfigure_pending: bool,
    pub insert_panel: Option<(TrackId, usize)>,
    /// Live master-bus insert slots, in persisted spec order.
    pub master_inserts: Vec<verdant_plugins::InsertSlot>,
    /// Which master slot's parameter panel is open.
    pub master_insert_panel: Option<usize>,
    pub export_busy: bool,
    pub text_prompt: Option<TextPrompt>,
    pub modal_dialog: Option<crate::dialogs::ModalDialog>,
    pub focus_dialog: FocusHandle,
    pub layout: crate::ui_controls::LayoutPrefs,
    pub automation_ui: crate::automation_lane::AutomationUiState,
    pub master_drag: Option<MasterMixerDrag>,
    pub selected_part: std::option::Option<std::string::String>,
    pub point_selection: std::collections::BTreeSet<(NoteId, verdant_domain::PointId)>,
    pub note_creation: Option<workflow::NoteCreation>,
    pub arrange_drag: Option<crate::arrange::RegionDrag>,
    pub export_options: crate::export::ExportOptions,
    pub integrations: integrations::IntegrationState,
    shut_down: bool,
    /// Pending recovery notice text (auto-applied on open; informational).
    pub recovered: Option<String>,
    /// Last time the slow pump ran autosave/prefs flushing.
    last_slow: Instant,
    /// Calendar sampling stays independent of autosave and the frame clock.
    last_pride_check: Instant,
}

impl Editor {
    pub fn new(
        persistence: ProjectPersistence,
        recovery: verdant_domain::RecoveryReport,
        opts: EditorOpts,
        worker_cfg: WorkerConfig,
        (engine, audio): (OutputEngine, Control),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_roll = cx.focus_handle();
        let focus_lyric = cx.focus_handle();
        let focus_table = cx.focus_handle();
        cx.on_blur(&focus_lyric, window, |this: &mut Self, window, cx| {
            this.end_lyric(true, None, window, cx);
        })
        .detach();
        cx.observe_window_activation(window, |this: &mut Self, window, cx| {
            if !window.is_window_active() {
                this.menu = None;
                this.cancel_edits(verdant_session::CancelReason::FocusLost, window, cx);
            }
        })
        .detach();
        let recovered = recovery_text(&recovery);
        for key in recovery.load.preserved_unknown_keys { eprintln!("verdant: preserving unknown field {key}"); }
        let project = persistence.project();
        let project_path = opts.project_path;
        let track = project.tracks.first().map(|t| t.id.clone()).unwrap_or_else(TrackId::generate);
        let mut session = EditorSession::new(track);
        session.viewport.set_quarter_ticks(project.ppq);
        let mut arrange = Viewport { row_height: 64.0, ..Viewport::default() };
        arrange.set_quarter_ticks(project.ppq);
        session.snap.grid_ticks=(project.ppq/4).max(1);
        session.default_note_ticks=(project.ppq/2).max(1);
        if let Some(first) = project.tracks.first().and_then(|t| t.notes.first()) {
            session.viewport.top_pitch = first.midi_note as f64 + 12.0;
        }
        let audio_info = engine.output_info();
        let playback = crate::clock::PlaybackViews::new(cx.weak_entity(), cx);
        let chrome = crate::view::ChromeViews::new(cx.weak_entity(), cx);
        let owner = cx.weak_entity();
        let static_view = cx.new(|cx| crate::view::StaticEditor::new(owner, cx));
        let (compiled_roll, compiled_pitch_move) = crate::proto_roll::mount(cx.weak_entity(), window, cx);
        cx.observe_window_bounds(window, |ed, window, _| {
            crate::files::capture_bounds(&mut ed.prefs, window);
            ed.prefs_dirty = true;
        }).detach();
        // Platform quit is already irrevocable here and GPUI forbids spawning
        // tasks. Normal window/Quit requests must pass shutdown() first.
        cx.on_app_quit(|ed, _| {
            if !ed.shut_down {
                eprintln!("verdant: platform quit bypassed final state capture; flushing only committed state");
                if let Err(e) = ed.persistence.autosave() { eprintln!("verdant: final autosave: {e}"); }
                if let Err(e) = crate::files::save_prefs(&ed.prefs) { eprintln!("verdant: final preferences: {e}"); }
                ed.audio.stop();
            }
            async {}
        }).detach();
        let weak = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            weak.update(cx, |ed, cx| {
                crate::files::capture_bounds(&mut ed.prefs, window);
                ed.shutdown(cx)
            }).unwrap_or(false)
        });
        let layout = crate::ui_controls::LayoutPrefs::from_prefs(&opts.prefs);
        let mut ed = Editor {
            persistence,
            project_path,
            audition_projects: opts.audition_projects,
            session,
            lyric: None,
            focus_roll,
            focus_lyric,
            focus_table,
            focus_menu: cx.focus_handle(),
            worker_cfg,
            worker: None,
            worker_gen: 0,
            worker_status: WorkerStatus::NotStarted,
            worker_workspace: None,
            scheduler: RenderScheduler::new(),
            plans: Vec::new(),
            scheduled_phonemes: Default::default(),
            phrase_slots: HashMap::new(),
            render_audio: HashMap::new(),
            arrange_cache: crate::arrange::ArrangeCache::default(),
            auto_render: true,
            render_armed: true,
            replan_gen: 0,
            malformed: Vec::new(),
            audio,
            engine,
            audio_info,
            phrase_pcm: project_audio::PhrasePcmCache::default(),
            timeline_gen: 0,
            timeline_phrases: 0,
            timeline_instruments: 0,
            cache_restore_gen: 0,
            cache_restoring: false,
            notices: VecDeque::new(),
            show_f0: true,
            lyric_line: None,
            lyric_bounds: None,
            needs_fit: true,
            frame_playhead: 0.0,
            playhead_smoother: Default::default(),
            ui_font: pick_ui_font(cx),
            index: crate::index::NoteIndex::default(),
            roll_pitch: crate::roll::RenderedPitchCache::default(),
            language_segments: crate::index::LanguageSegments::default(),
            active_param_data: Default::default(),
            phone_budgets: Default::default(),
            resolved_readings: Default::default(),
            ready_notes: HashSet::new(),
            error_notes: HashSet::new(),
            playback,
            static_view,
            chrome,
            compiled_roll,
            compiled_pitch_move,
            theme: Theme::resolve(window.appearance(), None, &opts.prefs),
            tool: Tool::Pointer,
            lane_tool: LaneTool::default(),
            bottom: Bottom::Editor,
            show_inspector: true,
            width_class: Default::default(),
            drawer: None,
            touch_ui: false,
            window_chrome: Default::default(),
            title_drag: false,
            device_drag: None,
            device_band: 2,
            device_param_state: HashMap::new(),
            device_anim: HashMap::new(),
            param: Param::Pit,
            pit_range_cents: 200.0,
            lane_stroke: None,
            mixer_drag: None,
            meters: Vec::new(),
            master_meter: (0.0, 0.0),
            arrange,
            arrange_track_height: 64.0,
            needs_arrange_fit: true,
            focus_arrange: cx.focus_handle(),
            appearance_override: None,
            prefs: opts.prefs,
            prefs_dirty: false,
            menu: None,
            note_clipboard: Vec::new(),
            curve_clipboard: None,
            table_cursor: 0,
            table_anchor: None,
            table_focus: crate::table::TableFocus::default(),
            table_scroll: gpui::UniformListScrollHandle::new(),
            point_scroll: gpui::UniformListScrollHandle::new(),
            numeric_preview: None,
            plugins: None,
            insert_runtime: plugin_ui::InsertRuntime::default(),
            inserts: HashMap::new(),
            inserts_loading: false,
            insert_specs_applied: HashMap::new(),
            insert_gen: 0,
            plugin_params: HashMap::new(),
            plugin_busy: HashSet::new(),
            plugin_counters: HashMap::new(),
            plugin_reconfigure_pending: false,
            insert_panel: None,
            master_inserts: Vec::new(),
            master_insert_panel: None,
            export_busy: false,
            text_prompt: None,
            modal_dialog: None,
            focus_dialog: cx.focus_handle(),
            layout,
            automation_ui: crate::automation_lane::AutomationUiState::default(),
            master_drag: None,
            selected_part: None,
            point_selection: Default::default(),
            note_creation: None,
            arrange_drag: None,
            export_options: crate::export::ExportOptions::default(),
            integrations: integrations::IntegrationState::default(),
            shut_down: false,
            recovered,
            last_slow: Instant::now(),
            last_pride_check: Instant::now(),
        };
        if let Some(value)=ed.prefs.extra.get("audio_export_options"){if let Ok(options)=serde_json::from_value(value.clone()){ed.export_options=options;}}
        ed.restore_media_authorizations();
        ed.sync_index();
        ed.sync_arrange_cache(false);
        ed.sync_mixer();
        ed.start_plugins(opts.plughost_path.as_deref(), cx);
        ed.restore_inserts(cx);
        ed.start_worker(cx);
        ed.spawn_slow_pump(cx);
        crate::files::prompt_recovery(&mut ed, window, cx);
        ed
    }

    /// Frame all notes: pitch range with margin, first notes to the right edge.
    pub fn fit_view(&mut self) {
        let project = self.store().snapshot();
        let Some(track) = project.track(&self.session.track) else { return };
        let vp = &mut self.session.viewport;
        let notes = &track.notes;
        let (lo, hi, end) =
            notes.iter().fold((127, 0, 0i64), |(lo, hi, end), n| (lo.min(n.midi_note), hi.max(n.midi_note), end.max(n.end_tick())));
        if notes.is_empty() {
            vp.top_pitch = 79.0;
            vp.row_height = 20.0;
            return;
        }
        let rows = (hi - lo + 1) as f64 + 8.0;
        vp.row_height = (vp.height / rows).clamp(14.0, 30.0);
        vp.top_pitch = hi as f64 + 1.0 + ((vp.height / vp.row_height) - (hi - lo + 1) as f64) / 2.0;
        vp.fit_ticks(0.0, end as f64, 40.0, 80.0);
    }

    pub fn project(&self) -> &Project {
        self.store().project()
    }

    pub fn revision(&self) -> Revision {
        self.store().revision()
    }

    /// Pan and zoom redraw the affected viewport. Score edits still notify the editor.
    pub(crate) fn notify_viewport(&self, space: crate::clock::Space, cx: &mut Context<Self>) {
        match space {
            crate::clock::Space::Editor => {
                self.static_view.update(cx, |_, cx| cx.notify());
                self.playback.lcd.update(cx, |_, cx| cx.notify());
            }
            crate::clock::Space::Arrange => self.chrome.invalidate_arrangement(cx),
        }
    }

    pub fn store(&self) -> &ProjectStore {
        self.persistence.store()
    }

    fn sync_index(&mut self) {
        let project = self.store().snapshot();
        let revision = self.revision();
        self.index.sync(&project, &self.session.track, revision);
        self.language_segments.sync(&project, revision);
        self.active_param_data.take();
        self.phone_budgets.get_mut().clear();
        *self.resolved_readings.get_mut() = None;
    }

    pub(crate) fn active_param_data(&self) -> [bool; params::ALL.len()] {
        *self.active_param_data.get_or_init(|| {
            let Some(track) = self.active_track() else { return [false; params::ALL.len()] };
            params::ALL.map(|param| track.notes.iter().any(|note| match param {
                Param::Pit => !note.curves.pitch_millicent_points.is_empty(),
                Param::Vibrato => note.vibrato.as_ref().is_some_and(|v| v.depth_cents > 0),
                Param::Portamento => note.portamento_ms.is_some_and(|ms| ms > 0),
                Param::Dyn => !note.curves.gain_millidecibel_points.is_empty(),
                other => note.curves.expression_points.get(other.short()).is_some_and(|points| !points.is_empty()),
            }))
        })
    }

    fn sync_arrange_cache(&mut self, planned: bool) {
        let project = self.store().snapshot();
        let revision = self.revision();
        self.arrange_cache.rebuild(&project, revision, planned.then_some(self.plans.as_slice()));
    }

    pub fn meter(&self) -> verdant_domain::MeterMap {
        verdant_domain::MeterMap::new(&self.project().time_signatures, self.project().ppq).expect("admitted project meter map")
    }

    pub fn tempo(&self) -> TempoMap {
        TempoMap::new(&self.project().tempo_map, self.project().ppq).unwrap_or_else(|_| TempoMap::constant(120.0))
    }

    /// Route of the track being edited.
    pub fn route(&self) -> Route {
        self.project().track(&self.session.track).map(|t| adapter::desktop_route(self.project(), t, self.scheduler.capabilities.as_ref())).unwrap_or(Route::VerdantVoice)
    }

    pub fn active_track(&self) -> Option<&Track> {
        self.project().track(&self.session.track)
    }

    pub fn track_index(&self, id: &TrackId) -> Option<usize> {
        self.project().tracks.iter().position(|t| &t.id == id)
    }

    pub fn plan_for(&self, id: &TrackId) -> Option<&PhrasePlan> {
        self.plans.iter().find(|p| &p.track == id)
    }

    pub fn active_plan(&self) -> Option<&PhrasePlan> {
        self.plan_for(&self.session.track)
    }

    /// Frame the whole song in the tracks area.
    pub fn fit_arrange(&mut self) {
        let end=self.project().tracks.iter().flat_map(|t|t.notes.iter()).map(|n|n.end_tick()).max().unwrap_or(0).max(self.arrange_cache.end_tick()).max(self.project().ppq.saturating_mul(4));
        let vp = &mut self.arrange;
        vp.fit_ticks(0.0, end as f64, 20.0, 60.0);
    }

    pub fn notice(&mut self, text: impl Into<String>) {
        self.notices.push_back(Notice { at: Instant::now(), text: text.into() });
        while self.notices.len() > 6 {
            self.notices.pop_front();
        }
    }

    // ---- transactions ------------------------------------------------------------

    pub fn commit(&mut self, base: Revision, batch: CommandBatch, cx: &mut Context<Self>) -> bool {
        let result = self.persistence.commit(base, batch);
        match result {
            Ok(Some(_)) => {
                self.after_change(cx);
                true
            }
            Ok(None) => false,
            Err(e) => {
                self.notice(format!("未提交：{e}"));
                cx.notify();
                false
            }
        }
    }

    /// These controls retain previews outside the session gesture machine.
    pub(crate) fn has_control_drag(&self, cx: &Context<Self>) -> bool {
        self.master_drag.is_some()
            || self.numeric_preview.is_some()
            || self.device_drag.is_some()
            || crate::ui_controls::has_numeric_scrub(cx.entity().entity_id(), cx)
            || crate::roll::has_extra_drag(cx.entity().entity_id(), cx)
    }

    pub(crate) fn cancel_control_drags(&mut self, cx: &mut Context<Self>) {
        self.master_drag = None;
        self.numeric_preview = None;
        self.cancel_device_drag(cx);
        let editor = cx.entity().entity_id();
        crate::ui_controls::cancel_numeric_scrub(editor, cx);
        crate::roll::cancel_extra_drag(editor, cx);
    }

    fn after_change(&mut self, cx: &mut Context<Self>) {
        let p = self.store().snapshot();
        let fx = self.session.on_project_changed(&p, self.revision());
        // Undo/redo can remove the active track just like an explicit delete.
        // Keep a surviving selection; otherwise choose before building caches.
        if p.track(&self.session.track).is_none() {
            let next = p.tracks.first().map(|t| t.id.clone()).unwrap_or_else(|| self.session.track.clone());
            self.change_active_track(next);
        }
        self.sync_index();
        self.sync_arrange_cache(false);
        if self.table_anchor.as_ref().is_some_and(|id| !self.index.contains(id)) {
            self.table_anchor = None;
        }
        if self.lyric.as_ref().is_some_and(|l| l.track != self.session.track || !self.index.contains(&l.note_id)) {
            self.lyric = None;
            self.lyric_line = None;
            self.lyric_bounds = None;
        }
        self.sync_table_cursor();
        self.cancel_control_drags(cx);
        self.note_creation = None;
        self.arrange_drag = None;
        self.lane_stroke = None;
        self.mixer_drag = None;
        for f in fx {
            if let Effect::Notice(n) = f {
                self.notice(n);
            }
        }
        self.prepare_insert_topology(cx);
        self.sync_mixer();
        self.restore_inserts(cx);
        self.schedule_replan(cx);
        cx.notify();
    }

    pub fn apply_effects(&mut self, fx: Vec<Effect>, window: &mut Window, cx: &mut Context<Self>) {
        for f in fx {
            match f {
                Effect::Commit { base, batch, select_after } => {
                    if self.commit(base, batch, cx) {
                        if let Some(sel) = select_after {
                            self.session.selection = sel.into_iter().collect();
                        }
                    }
                }
                Effect::BeginLyricEdit(id) => self.begin_lyric(id, window, cx),
                Effect::Notice(n) => self.notice(n),
                Effect::CapturePointer | Effect::ReleasePointer => {}
            }
        }
        self.sync_table_cursor();
        cx.notify();
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.lyric.is_some() {
            return;
        }
        self.cancel_control_drags(cx);
        self.sync_mixer();
        cx.notify();
        // Admit an earlier native-editor edit before history can replace its
        // host. Compare with its live baseline so undoing initial default-state
        // capture does not immediately recapture that same default forever.
        if !self.flush_insert_edits(cx) { return; }
        let result = self.persistence.undo();
        match result {
            Ok(Some(c)) => {
                self.notice(c.label);
                self.after_change(cx);
            }
            Ok(None) => self.notice("没有可撤销的操作"),
            Err(e) => self.notice(e.to_string()),
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if self.lyric.is_some() {
            return;
        }
        self.cancel_control_drags(cx);
        self.sync_mixer();
        cx.notify();
        // A newly captured edit branches history just like any other edit;
        // unchanged capture leaves the existing redo branch intact.
        if !self.flush_insert_edits(cx) { return; }
        let result = self.persistence.redo();
        match result {
            Ok(Some(c)) => {
                self.notice(c.label);
                self.after_change(cx);
            }
            Ok(None) => self.notice("没有可重做的操作"),
            Err(e) => self.notice(e.to_string()),
        }
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.end_lyric(true, None, window, cx);
        match &self.project_path {
            Some(path) => {
                let path = path.clone();
                self.save_project_as(path, cx);
            }
            // Never persisted before: go through the native save-as dialog.
            None => crate::files::save_as_dialog(self, window, cx),
        }
        cx.notify();
    }

    /// Persist to a `.verp` destination and
    /// update the working copy's home. Plugin insert states are flushed into
    /// the document first so state_b64 survives reopen.
    pub fn save_project_as(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let path = match crate::files::native_save_path(&path) {
            Ok(path) => path,
            Err(error) => { self.notice(format!("保存失败：{error}")); cx.notify(); return; }
        };
        if !self.flush_insert_states(cx) { return; }
        let source_path=self.persistence.project_path().to_path_buf();
        let assets=match verdant_service::media::ProjectAssetCopy::new(self.store(),&source_path,&path){
            Ok(assets)=>assets,
            Err(error)=>{self.notice(format!("保存失败：{error}"));cx.notify();return;}
        };
        self.integrations.media_save_generation=self.integrations.media_save_generation.wrapping_add(1);
        let generation=self.integrations.media_save_generation;
        if !assets.requires_copy(){self.persist_project_as(path,cx);return;}
        let project_id=self.project().project_id.clone();let revision=self.revision();
        self.integrations.media_file_jobs+=1;
        self.notice("正在核对并复制工程相对音频素材…");
        cx.spawn(async move|this,cx|{
            let result=cx.background_spawn(async move{assets.copy().map_err(|error|error.to_string())}).await;
            let _=this.update(cx,|ed,cx|{
                ed.integrations.media_file_jobs=ed.integrations.media_file_jobs.saturating_sub(1);
                if ed.integrations.media_save_generation!=generation{return;}
                if ed.persistence.project_path()!=source_path.as_path()||ed.project().project_id!=project_id||ed.revision()!=revision{
                    ed.notice("工程已改变，本次另存为未写入；请再次保存");cx.notify();return;
                }
                match result{
                    Ok(_)=>ed.persist_project_as(path,cx),
                    Err(error)=>{ed.notice(format!("保存失败：音频素材未能完整迁移：{error}"));cx.notify();}
                }
            });
        }).detach();
        cx.notify();
    }

    fn persist_project_as(&mut self,path:PathBuf,cx:&mut Context<Self>){
        let result = self.persistence.save_as(&path);
        match result {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.remember_project(&path);
                if let Some(dir) = path.parent() {
                    self.prefs.last_project_dir = Some(dir.display().to_string());
                    self.prefs_dirty = true;
                }
                self.notice(format!("已保存 {}", path.display()));
            }
            Err(e) => self.notice(format!("保存失败：{e}")),
        }
        cx.notify();
    }

    /// Open a native document after creator-version admission, before any
    /// target journal is opened.
    pub fn open_project_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.open_project_path_allowed(path, None, window, cx);
    }

    fn open_project_path_allowed(&mut self, path: PathBuf, allowed: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = verdant_domain::io::require_project_path(&path) {
            self.notice(error.to_string());
            cx.notify();
            return;
        }
        if !self.commit_pending_lyric(cx) { return; }
        if !self.flush_insert_states(cx) { return; }
        if let Err(e) = self.persistence.autosave() {
            self.notice(format!("当前工程恢复日志保存失败：{e}")); cx.notify(); return;
        }
        enum Opened {
            New(ProjectPersistence, verdant_domain::RecoveryReport),
            Current(verdant_domain::RecoveryReport),
            NeedsApproval(String),
        }
        let opened = if path.canonicalize().is_ok_and(|canonical| canonical == self.persistence.project_path()) {
            let mut newer = None;
            let result = self.persistence.reopen_if(&mut |project| {
                newer = crate::files::creator_requiring_approval(project, allowed);
                newer.is_none()
            });
            match result {
                Ok(Some(report)) => Ok(Opened::Current(report)),
                Ok(None) => Ok(Opened::NeedsApproval(newer.expect("admission rejected a creator version"))),
                Err(error) => Err(error.to_string()),
            }
        } else {
            crate::files::open_native_project(&path, allowed).map(|outcome| match outcome {
                crate::files::NativeOpen::Opened(persistence, report) => Opened::New(persistence, report),
                crate::files::NativeOpen::Newer(creator) => Opened::NeedsApproval(creator),
            })
        };
        let report = match opened {
            Ok(Opened::New(persistence, report)) => {
                self.persistence = persistence;
                report
            }
            Ok(Opened::Current(report)) => report,
            Ok(Opened::NeedsApproval(creator)) => {
                let detail = crate::files::newer_project_warning(&path, &creator);
                let answer = window.prompt(gpui::PromptLevel::Warning, "工程创建版本较新", Some(&detail), &["仍然打开", "取消"], cx);
                cx.spawn_in(window, async move |this, cx| {
                    let answer = answer.await;
                    let _ = cx.update(|window, cx| this.update(cx, |ed, cx| {
                        if matches!(answer, Ok(0)) { ed.open_project_path_allowed(path, Some(&creator), window, cx); }
                        else { ed.notice("已取消打开；目标工程与恢复日志保持原样"); cx.notify(); }
                    }));
                }).detach();
                return;
            }
            Err(e) => {
                self.notice(format!("无法打开 {}：{e}", path.display()));
                return;
            }
        };
        for k in &report.load.preserved_unknown_keys {
            eprintln!("verdant: preserving unknown field {k}");
        }
        self.finish_project_opened(report, window, cx);
        if let Some(dir) = path.parent() {
            self.prefs.last_project_dir = Some(dir.display().to_string());
            self.prefs_dirty = true;
        }
        self.remember_project(&path);
        self.notice(format!("已打开 {}", path.display()));
        cx.notify();
    }

    /// Open an editable built-in as a separate, recoverable working copy.
    /// Selecting an example never overwrites the current document or turns
    /// its internal working path into the destination for the user's Save.
    fn open_builtin_example(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let result = (|| -> Result<(), String> {
            let example = crate::demos::find(id).ok_or_else(|| format!("未知内建示例：{id}"))?;
            if !self.commit_pending_lyric(cx) || !self.flush_insert_states(cx) {
                return Err("当前编辑尚未提交".into());
            }
            self.persistence.autosave().map_err(|error| error.to_string())?;
            let persistence = crate::demos::create_working_copy(id, &crate::files::state_dir().join("documents"))?;
            let path = persistence.project_path().to_path_buf();
            self.replace_project_opened(persistence, Default::default(), window, cx);
            self.remember_project(&path);
            // Explicit example selection keeps its authored sound.
            self.notice(format!("已打开内建示例：{} · {}；保存时可选择自己的工程路径", example.title, example.attribution));
            Ok(())
        })();
        if let Err(error) = result { self.notice(format!("打开示例失败：{error}")); }
        cx.notify();
    }

    /// Swap in a project whose `ProjectPersistence` is already open (the
    /// journal is replayed and bound — no re-open needed).
    pub(crate) fn replace_project_opened(&mut self, persistence: ProjectPersistence, report: verdant_domain::RecoveryReport, window: &mut Window, cx: &mut Context<Self>) {
        self.persistence = persistence;
        self.finish_project_opened(report, window, cx);
    }

    fn finish_project_opened(&mut self, report: verdant_domain::RecoveryReport, window: &mut Window, cx: &mut Context<Self>) {
        let track = self.persistence.project().tracks.first().map(|t| t.id.clone()).unwrap_or_else(TrackId::generate);
        self.cancel_edits(verdant_session::CancelReason::ProjectChanged, window, cx);
        self.audio.stop();
        self.audio.set_timeline(Timeline::new(self.audio_info.sample_rate, Vec::new()));
        self.timeline_gen += 1;
        self.timeline_phrases = 0;
        self.render_armed = self.auto_render;
        self.scheduler = RenderScheduler::new();
        self.roll_pitch = crate::roll::RenderedPitchCache::default();
        self.plans.clear();
        self.scheduled_phonemes.clear();
        self.phrase_slots.clear();
        self.render_audio.clear();
        self.ready_notes.clear();
        self.error_notes.clear();
        self.phrase_pcm = project_audio::PhrasePcmCache::default();
        self.cache_restore_gen += 1;
        self.cache_restoring = false;
        self.text_prompt = None;
        self.modal_dialog = None;
        self.automation_ui = crate::automation_lane::AutomationUiState::default();
        self.master_drag = None;
        self.selected_part = None;
        self.point_selection.clear();
        self.note_creation = None;
        self.arrange_drag = None;
        self.cancel_midi_recording("工程切换，MIDI 录音已取消");
        self.integrations.selected_clip=None;self.integrations.media_ready=false;self.integrations.media_previews.clear();self.integrations.media_save_generation=self.integrations.media_save_generation.wrapping_add(1);
        self.project_path = crate::demos::save_destination(self.persistence.project(), Some(self.persistence.project_path()),
            &crate::files::state_dir().join("documents"));
        self.lyric = None;
        self.session = EditorSession::new(track);
        let quarter_ticks = self.project().ppq;
        self.session.viewport.set_quarter_ticks(quarter_ticks);
        self.arrange.set_quarter_ticks(quarter_ticks);
        self.session.snap.grid_ticks=(self.project().ppq/4).max(1);
        self.session.default_note_ticks=(self.project().ppq/2).max(1);
        self.needs_fit = true;
        self.needs_arrange_fit = true;
        self.table_cursor = 0;
        self.table_anchor = None;
        self.table_focus = crate::table::TableFocus::default();
        self.recovered = recovery_text(&report);
        self.insert_specs_applied.clear();
        self.insert_panel = None;
        self.master_insert_panel = None;
        self.index = crate::index::NoteIndex::default();
        self.language_segments = crate::index::LanguageSegments::default();
        self.sync_index();
        self.sync_arrange_cache(false);
        self.prepare_insert_topology(cx);
        self.restore_inserts(cx);
        self.sync_mixer();
        self.start_worker(cx);
        window.focus(&self.focus_roll, cx);
        cx.notify();
        crate::files::prompt_recovery(self, window, cx);
    }

    /// File → export a native `.verp` project copy.
    pub fn export_project(&mut self, dest: &Path, cx: &mut Context<Self>) {
        let dest = match crate::files::native_save_path(dest) {
            Ok(path) => path,
            Err(error) => { self.notice(format!("导出失败：{error}")); cx.notify(); return; }
        };
        if !self.flush_insert_states(cx) { return; }
        let source_path=self.persistence.project_path().to_path_buf();
        let assets=match verdant_service::media::ProjectAssetCopy::current(self.project(),&source_path,&dest){
            Ok(assets)=>assets,
            Err(error)=>{self.notice(format!("导出失败：{error}"));cx.notify();return;}
        };
        if assets.requires_copy(){
            let project_id=self.project().project_id.clone();let revision=self.revision();
            self.integrations.media_file_jobs+=1;
            self.notice("正在核对并复制导出工程的相对音频素材…");
            cx.spawn(async move|this,cx|{
                let result=cx.background_spawn(async move{assets.copy().map_err(|error|error.to_string())}).await;
                let _=this.update(cx,|ed,cx|{
                    ed.integrations.media_file_jobs=ed.integrations.media_file_jobs.saturating_sub(1);
                    if ed.persistence.project_path()!=source_path.as_path()||ed.project().project_id!=project_id||ed.revision()!=revision{
                        ed.notice("工程已改变，本次工程副本未导出；请再次导出");cx.notify();return;
                    }
                    match result{
                        Ok(_)=>ed.finish_project_export(&dest,cx),
                        Err(error)=>{ed.notice(format!("导出失败：音频素材未能完整迁移：{error}"));cx.notify();}
                    }
                });
            }).detach();
            cx.notify();return;
        }
        self.finish_project_export(&dest,cx);
    }

    fn finish_project_export(&mut self,dest:&Path,cx:&mut Context<Self>){
        match self.persistence.export_project(&dest) {
            Ok(()) => self.notice(format!("已导出 {}", dest.display())),
            Err(e) => self.notice(format!("导出失败：{e}")),
        }
        cx.notify();
    }

    // ---- note clipboard -----------------------------------------------------

    /// Copy the selection (deep copies; ids regenerate on paste).
    pub fn copy_notes(&mut self, cx: &mut Context<Self>) {
        let sel = self.selected_notes();
        if sel.is_empty() {
            self.notice("先选择音符");
            return;
        }
        self.note_clipboard = sel;
        self.notice(format!("已复制 {} 个音符", self.note_clipboard.len()));
        cx.notify();
    }

    /// Cut = copy + one delete batch (one undo).
    pub fn cut_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copy_notes(cx);
        if self.note_clipboard.is_empty() {
            return;
        }
        let p = self.store().snapshot();
        let fx = self.session.delete_selection(&p, self.revision());
        self.apply_effects(fx, window, cx);
    }

    /// Paste the note clipboard on the active track at `at_tick` (snapped to
    /// grid; ids regenerate — same rule as alt-drag duplicate: a cancelled
    /// paste leaves nothing).
    pub fn paste_notes_at(&mut self, at_tick: i64, cx: &mut Context<Self>) {
        if self.note_clipboard.is_empty() {
            self.notice("剪贴板为空");
            return;
        }
        let track = self.session.track.clone();
        let Some(t) = self.project().track(&track) else { return };
        let base = self.note_clipboard.iter().map(|n| n.start_tick).min().unwrap_or(at_tick);
        let anchor = self.session.snap.floor(at_tick as f64).max(0);
        let dt = anchor - base;
        let mut notes: Vec<Note> = Vec::with_capacity(self.note_clipboard.len());
        let mut new_ids = Vec::with_capacity(self.note_clipboard.len());
        for n in &self.note_clipboard {
            let mut c = n.clone();
            c.id = NoteId::generate();
            c.start_tick = (n.start_tick + dt).max(0);
            new_ids.push(c.id.clone());
            notes.push(c);
        }
        // Reject ids already present (paranoia; generate() should never hit).
        if notes.iter().any(|n| t.note(&n.id).is_some()) {
            self.notice("粘贴失败：id 冲突");
            return;
        }
        if self.commit(
            self.revision(),
            CommandBatch::single(format!("粘贴 {} 个音符", notes.len()), Command::InsertNotes { track, notes }),
            cx,
        ) {
            self.session.selection = new_ids.into_iter().collect();
        }
    }

    /// Duplicate the selection in place (+1 grid step right; one batch).
    pub fn duplicate_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids = self.session.selection_in_order(self.project());
        if ids.is_empty() {
            self.notice("先选择音符");
            return;
        }
        let track = self.session.track.clone();
        let new_ids: Vec<NoteId> = ids.iter().map(|_| NoteId::generate()).collect();
        let dt = self.session.snap.grid_ticks.max(1);
        let cmd = Command::DuplicateNotes { track, source_ids: ids, new_ids: new_ids.clone(), delta_tick: dt, delta_semitones: 0 };
        if self.commit(self.revision(), CommandBatch::single("复制一份", cmd), cx) {
            self.session.selection = new_ids.into_iter().collect();
        }
        let _ = window;
    }

    // ---- curve clipboard ----------------------------------------------------

    /// Copy the selected notes' curves without interpolating across notes.
    pub fn copy_curve(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.active_track().cloned() else { return };
        let param = self.param;
        if param == Param::Vibrato {
            self.notice("VIB 是逐音符深度设置；POR 可复制音符起点值，其余参数可复制曲线");
            return;
        }
        let (lo, hi) = self
            .session
            .selection_in_order(self.project())
            .iter()
            .filter_map(|id| track.note(id))
            .fold((i64::MAX, i64::MIN), |(a, b), n| (a.min(n.start_tick), b.max(n.end_tick())));
        if lo > hi {
            self.notice("先选择含曲线的音符");
            return;
        }
        let mut segments = Vec::new();
        for n in track.notes_in_range(lo, hi).filter(|n| self.session.selection.contains(&n.id)) {
            let samples = param.samples(n);
            segments.push(CurveSegment { start_tick: n.start_tick - lo, ticks: n.duration_tick, samples });
        }
        let ticks = hi - lo;
        let points: usize = segments.iter().map(|s| s.samples.len()).sum();
        let segment_count = segments.len();
        self.curve_clipboard = Some(CurveClipboard { param, ticks, segments });
        self.notice(if param == Param::Portamento { format!("已复制 POR 音符起点值（{segment_count} 个音符）") }
            else { format!("已复制 {} 曲线（{} 点）", param.short(), points) });
        cx.notify();
    }

    /// Paste the curve clipboard at `at_tick`: rebased onto notes it
    /// intersects, replacing their curve points inside the pasted span (one
    /// transaction across all touched notes). Bypassed destination regions
    /// reject the complete paste so their retained values stay intact.
    pub fn paste_curve_at(&mut self, at_tick: i64, cx: &mut Context<Self>) {
        let Some(cb) = self.curve_clipboard.clone() else {
            self.notice("曲线剪贴板为空");
            return;
        };
        if !verdant_backend::params::support(cb.param, self.route(), self.scheduler.capabilities.as_ref()).is_effective() {
            self.notice(format!("当前后端不能执行 {} 参数，未粘贴曲线", cb.param.short()));
            cx.notify();
            return;
        }
        let Some(track) = self.active_track().cloned() else { return };
        let (lo, hi) = (at_tick, at_tick.saturating_add(cb.ticks));
        let mut commands = Vec::new();
        for n in track.notes_in_range(lo, hi) {
            let spans: Vec<_> = cb.segments.iter().filter_map(|segment| {
                let origin = at_tick.saturating_add(segment.start_tick);
                let from = origin.max(n.start_tick);
                let to = origin.saturating_add(segment.ticks).min(n.end_tick());
                (from < to).then_some((segment, origin, from - n.start_tick, to - n.start_tick))
            }).collect();
            if spans.is_empty() {
                continue;
            }
            if self.region_lane_disabled(n, cb.param) {
                self.notice(format!("目标音符已旁路 {} 区域，未粘贴曲线", cb.param.short()));
                cx.notify();
                return;
            }
            if cb.param == Param::Portamento {
                // A destination note has one POR event. Use the first source
                // note overlapping it, including a paste begun mid-note.
                // Imported clipboard vertices after its onset are not events.
                let value = cb.param.interpolate(&spans[0].0.samples, 0.);
                let points = verdant_backend::params::portamento_points_for_value(n, value);
                commands.push(Command::SetExpressionCurve { track: track.id.clone(), note_id: n.id.clone(), parameter: "POR".into(), points });
                continue;
            }
            let original = cb.param.samples(n);
            let mut fresh = BTreeMap::new();
            // Guard untouched ticks on either side, including clipboard gaps.
            // Paste values below take priority if an adjacent segment owns one.
            for &(_, _, from, to) in &spans {
                for tick in [from.checked_sub(1).filter(|t| *t >= 0), to.checked_add(1).filter(|t| *t <= n.duration_tick)].into_iter().flatten() {
                    if !original.iter().any(|p| p.0 == tick) {
                        fresh.insert(tick, cb.param.interpolate(&original, tick as f64));
                    }
                }
            }
            for &(segment, origin, from, to) in &spans {
                let source_tick = |tick| n.start_tick + tick - origin;
                let value = |tick| cb.param.interpolate(&segment.samples, source_tick(tick) as f64);
                fresh.insert(from, value(from));
                for &(tick, value) in &segment.samples {
                    let tick = origin + tick - n.start_tick;
                    if tick > from && tick < to { fresh.insert(tick, value); }
                }
                // Curves are piecewise linear: retain exact vertices and the
                // clipped endpoint, without a lossy fixed-15-tick resampling.
                // Inside one destination note, a source boundary is a step on
                // the integer tick grid rather than a ramp across both notes.
                fresh.insert(to - 1, value(to - 1));
                fresh.insert(to, value(to));
            }
            let keep = |tick: i64| !fresh.contains_key(&tick) && !spans.iter().any(|&(_, _, from, to)| {
                tick >= from && tick <= to
            });
            match cb.param {
                Param::Pit => {
                    let mut pts: Vec<_> = n.curves.pitch_millicent_points.iter().filter(|p| keep(p.tick)).cloned().collect();
                    pts.extend(fresh.iter().map(|(t, v)| verdant_domain::model::PitchPoint {
                        id: verdant_domain::PointId::generate(),
                        tick: *t,
                        millicents: (v * 1000.0).round() as i64,
                        unknown: Default::default(),
                    }));
                    commands.push(Command::SetPitchCurve { track: track.id.clone(), note_id: n.id.clone(), points: pts });
                }
                Param::Dyn => {
                    let mut pts: Vec<_> = n.curves.gain_millidecibel_points.iter().filter(|p| keep(p.tick)).cloned().collect();
                    pts.extend(fresh.iter().map(|(t, v)| verdant_domain::model::GainPoint {
                        id: verdant_domain::PointId::generate(),
                        tick: *t,
                        millidecibels: (v * 1000.0).round() as i64,
                        unknown: Default::default(),
                    }));
                    commands.push(Command::SetGainCurve { track: track.id.clone(), note_id: n.id.clone(), points: pts });
                }
                p if p.is_expression() => {
                    let mut pts: Vec<_> = n.curves.expression(p.short()).iter().filter(|p| keep(p.tick)).cloned().collect();
                    pts.extend(fresh.iter().map(|(t, v)| verdant_domain::model::ExpressionPoint {
                        id: verdant_domain::PointId::generate(), tick: *t,
                        value: v.round().clamp(0., 127.) as u8, unknown: Default::default(),
                    }));
                    commands.push(Command::SetExpressionCurve { track: track.id.clone(), note_id: n.id.clone(), parameter: p.short().into(), points: pts });
                }
                _ => {}
            }
        }
        if commands.is_empty() {
            self.notice("粘贴范围内没有音符");
            return;
        }
        self.commit(self.revision(), CommandBatch { label: format!("粘贴 {} 曲线", cb.param.short()), commands }, cx);
    }

    // ---- context menu dispatch ------------------------------------------------

    /// Every menu entry resolves here; all actions are real callbacks.
    pub fn run_menu_action(&mut self, action: MenuAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            MenuAction::Undo => self.undo(cx),
            MenuAction::Redo => self.redo(cx),
            MenuAction::Save => self.save(window, cx),
            MenuAction::SaveAs => crate::files::save_as_dialog(self, window, cx),
            MenuAction::OpenProject => crate::files::open_project_dialog(self, window, cx),
            MenuAction::ChooseExample => self.audition_picker(window, cx),
            MenuAction::OpenBuiltinExample(id) => self.open_builtin_example(&id, window, cx),
            MenuAction::ExampleCredits => {
                if let Some(detail) = crate::demos::credit_text(self.project()) {
                    let response = window.prompt(gpui::PromptLevel::Info, "示例署名与来源", Some(&detail), &["关闭"], cx);
                    cx.spawn(async move |_, _| { let _ = response.await; }).detach();
                } else { self.notice("当前工程没有内建示例的署名记录"); }
            }
            MenuAction::OpenAudition(path) => {
                if crate::files::is_native_project_path(&path) { self.open_project_path(path, window, cx); }
                else { self.import_project_path(path, window, cx); }
            }
            MenuAction::ExportProject => crate::files::export_dialog(self, window, cx),
            MenuAction::Cut => self.cut_notes(window, cx),
            MenuAction::Copy => self.copy_notes(cx),
            MenuAction::Paste => {
                let t = self.tempo().tick_at(self.playhead_seconds()) as i64;
                self.paste_notes_at(t, cx)
            }
            MenuAction::PasteAtTick(t) => self.paste_notes_at(t, cx),
            MenuAction::DeleteSelection => {
                let p = self.store().snapshot();
                let fx = self.session.delete_selection(&p, self.revision());
                self.apply_effects(fx, window, cx);
            }
            MenuAction::SelectAll => {
                let p = self.store().snapshot();
                self.session.select_all(&p);
                cx.notify();
            }
            MenuAction::Duplicate => self.duplicate_selection(window, cx),
            MenuAction::EditLyric(id) => self.begin_lyric(id, window, cx),
            MenuAction::AddNoteAt(tick, midi) => {
                let n = Note::new(self.session.snap.floor(tick as f64).max(0), self.session.default_note_ticks, midi.clamp(0, 127));
                let id = n.id.clone();
                let track = self.session.track.clone();
                if self.commit(
                    self.revision(),
                    CommandBatch::single("插入音符", Command::InsertNotes { track, notes: vec![n] }),
                    cx,
                ) {
                    self.session.selection = [id.clone()].into();
                    self.begin_lyric(id, window, cx);
                }
            }
            MenuAction::SetTool(tool) => {
                self.tool = tool;
                cx.notify();
            }
            MenuAction::SetLaneTool(tool) => {
                self.lane_tool = tool;
                cx.notify();
            }
            MenuAction::CopyCurve => self.copy_curve(cx),
            MenuAction::PasteCurveAt(t) => self.paste_curve_at(t, cx),
            MenuAction::AddTrack => self.add_track(cx),
            MenuAction::DeleteTrack(id) => self.delete_track(id, cx),
            MenuAction::MuteTrack(id) => self.toggle_mute(&id, cx),
            MenuAction::SoloTrack(id) => self.toggle_solo(&id, cx),
            MenuAction::CycleColor(id) => self.cycle_color(&id, cx),
            MenuAction::SeekTick(t) => {
                self.seek_tick(t as f64);
                cx.notify();
            }
            MenuAction::AddInsert { track, format, id } => self.add_insert(&track, &format, &id, cx),
            MenuAction::BypassInsert { track, slot } => self.toggle_insert_bypass(&track, slot, cx),
            MenuAction::RemoveInsert { track, slot } => self.remove_insert(&track, slot, cx),
            MenuAction::OpenInsertEditor { track, slot } => self.open_insert_editor(&track, slot, cx),
            MenuAction::SetSidechain { track, slot, source } => self.set_insert_sidechain(&track, slot, source, cx),
            MenuAction::SetInsertParams { track, slot, values } => self.set_track_insert_params(track, slot, values, cx),
            MenuAction::AddInstrumentTrack => self.add_instrument_track(cx),
            MenuAction::MakeInstrumentTrack { track, instrument } => self.make_instrument_track(&track, instrument, cx),
            MenuAction::SetInstrumentPreset { track, preset } => self.set_instrument_preset(&track, &preset, cx),
            MenuAction::RescanPlugins => self.rescan_plugins(cx),
            MenuAction::AddMasterInsert { format, id } => self.add_master_insert(&format, &id, cx),
            MenuAction::ChooseDevice(name) => self.choose_device(&name),
            MenuAction::ReopenDevice => { let selected = self.prefs.device.clone().unwrap_or_default(); self.choose_device(&selected); },
            MenuAction::SetGridTicks { snap, ticks } => {
                if snap { self.session.snap.grid_ticks = ticks.max(0); }
                else { self.session.default_note_ticks = ticks.max(1); }
                cx.notify();
            }
            MenuAction::ToggleTrackAutomation(track) => {
                self.automation_ui.toggle_track(&track);
                cx.notify();
            }
            MenuAction::ToggleMasterAutomation => {
                self.automation_ui.show_master = !self.automation_ui.show_master;
                cx.notify();
            }
            MenuAction::SelectAutomationTarget { track, target } => {
                if let Some(tid) = track {
                    self.automation_ui.selected_targets.insert(tid.clone(), target);
                    self.automation_ui.expanded_tracks.insert(tid);
                } else {
                    self.automation_ui.master_target = Some(target);
                    self.automation_ui.show_master = true;
                }
                cx.notify();
            }
            MenuAction::AutomationPointInterp { target, tick, interpolation } => {
                self.set_automation_point_interp(target, tick, interpolation, cx);
            }
            MenuAction::DeleteAutomationPoint { target, tick } => {
                self.delete_automation_point(target, tick, cx);
            }
            MenuAction::TogglePhonemeLane => {
                self.layout.show_phoneme_lane = !self.layout.show_phoneme_lane;
                self.layout.store(&mut self.prefs);
                self.prefs_dirty = true;
                cx.notify();
            }
            MenuAction::ToggleTempoLane => {
                self.layout.show_tempo_lane = !self.layout.show_tempo_lane;
                self.layout.store(&mut self.prefs);
                self.prefs_dirty = true;
                cx.notify();
            }
            MenuAction::SetTrackLanguage { track, language } => {
                if let Some(lang) = language {
                    let result = self
                        .scheduler
                        .capabilities
                        .as_ref()
                        .ok_or_else(|| "后端未就绪".to_string())
                        .and_then(|caps| crate::voice_presets::language_choice_batch(self.project(), &track, Some(&lang), caps));
                    match result {
                        Ok(batch) => {
                            self.commit(self.revision(), batch, cx);
                        }
                        Err(error) => {
                            self.notice(error);
                            cx.notify();
                        }
                    }
                } else {
                    self.commit(
                        self.revision(),
                        CommandBatch::single(
                            "语言跟随读音器",
                            Command::SetTrackData { track, key: "voice_language".into(), value: None },
                        ),
                        cx,
                    );
                }
            }
            MenuAction::ShowHistoryWindow => self.show_history_panel(window, cx),
            MenuAction::ShowShortcutsWindow => crate::shortcuts::open(cx),
            MenuAction::ShowDisclosuresWindow => crate::disclosures::open(cx),
            MenuAction::OpenMenu(menu) => self.open_top_menu(menu, window.mouse_position(), window, cx),
            MenuAction::ShowProperties => {
                if !self.left_panel_visible() { self.toggle_left_panel(cx); }
            }
            MenuAction::ToggleFollow => crate::clock::toggle_follow(self, cx),
            MenuAction::ShowBottom(bottom) => self.show_bottom(bottom, window, cx),
            MenuAction::Run(command) => crate::generated::controls::dispatch(self, command, crate::generated::controls::CommandArg::None, window, cx),
            MenuAction::GeneratedSelect { id, arg, option } => {
                crate::generated::controls::dispatch_select(self, id, arg, option, window, cx);
            }
            MenuAction::Workflow(action) => self.run_workflow(action, window, cx),
        }
    }

    // ---- note-table editing -----------------------------------------------------

    /// Step the selected (or given) notes' pitch by `ds` semitones via one
    /// Command::SetPitch batch — the AccessKit Increment/Decrement action.
    pub fn step_notes_pitch(&mut self, ids: Vec<NoteId>, ds: i32, cx: &mut Context<Self>) {
        let track = self.session.track.clone();
        let Some(t) = self.project().track(&track).cloned() else { return };
        let commands: Vec<Command> = ids
            .iter()
            .filter_map(|id| t.note(id))
            .map(|n| Command::SetPitch {
                track: track.clone(),
                note_id: n.id.clone(),
                midi_note: (n.midi_note + ds).clamp(0, 127),
                detune_millicents: n.detune_millicents,
            })
            .collect();
        if commands.is_empty() {
            return;
        }
        let n = commands.len();
        self.commit(self.revision(), CommandBatch { label: format!("{n} 个音符音高 {ds:+}"), commands }, cx);
    }

    /// Move the table cursor; selection follows the cursor (list semantics).
    pub fn table_step(&mut self, forward: bool, extend: bool, cx: &mut Context<Self>) {
        let next = if forward { self.table_cursor.saturating_add(1) } else { self.table_cursor.saturating_sub(1) };
        self.table_select(next, extend, cx);
    }

    pub fn table_select(&mut self, index: usize, extend: bool, cx: &mut Context<Self>) {
        let project = self.store().snapshot();
        let Some(track) = project.track(&self.session.track).filter(|t| !t.notes.is_empty()) else { return };
        let index = index.min(track.notes.len() - 1);
        let note = &track.notes[index];
        if extend {
            let anchor_id = self.table_anchor.clone().or_else(|| track.notes.get(self.table_cursor).map(|n| n.id.clone())).unwrap_or_else(|| note.id.clone());
            let anchor = self.index.position(&anchor_id).unwrap_or(index);
            self.session.selection = track.notes[anchor.min(index)..=anchor.max(index)].iter().map(|n| n.id.clone()).collect();
            self.table_anchor = Some(anchor_id);
        } else {
            self.session.selection = [note.id.clone()].into();
            self.table_anchor = Some(note.id.clone());
        }
        self.table_cursor = index;
        self.table_scroll.scroll_to_item(index, gpui::ScrollStrategy::Nearest);
        self.session.viewport.reveal(note);
        cx.notify();
    }

    /// Keep `table_cursor` aligned with the single selection.
    fn sync_table_cursor(&mut self) {
        if self.session.selection.len() == 1 {
            if let Some(id) = self.session.selection.iter().next() {
                if let Some(i) = self.index.position(id) {
                    self.table_cursor = i;
                }
            }
        }
        let n = self.active_track().map(|t| t.notes.len()).unwrap_or(0);
        self.table_cursor = self.table_cursor.min(n.saturating_sub(1));
        self.table_scroll.scroll_to_item(self.table_cursor, gpui::ScrollStrategy::Nearest);
    }

    /// Explicit vowel-proxy pronunciation for the selection (one transaction).
    pub fn set_vowel(&mut self, vowel: &str, cx: &mut Context<Self>) {
        let ids = self.session.selection_in_order(self.project());
        if ids.is_empty() {
            self.notice("先选择音符");
            return;
        }
        let track = self.session.track.clone();
        let commands = ids
            .into_iter()
            .map(|id| Command::SetPronunciation {
                track: track.clone(),
                note_id: id,
                pronunciation: Pronunciation {
                    pinyin: vowel.to_string(),
                    source: PronunciationSource::Manual,
                    frontend_profile: VOWEL_PROFILE.into(),
                    ..Default::default()
                },
            })
            .collect::<Vec<_>>();
        let n = commands.len();
        self.commit(self.revision(), CommandBatch { label: format!("{n} 个音符设为元音 {vowel}"), commands }, cx);
    }

    // ---- tracks & mixer ------------------------------------------------------------

    /// Reset note-local interaction state before binding the new track's index.
    /// An empty loaded project keeps an unbound session ID until a track exists.
    fn change_active_track(&mut self, id: TrackId) {
        self.session.track = id;
        self.session.selection.clear();
        self.lyric = None;
        self.lyric_line = None;
        self.lyric_bounds = None;
        self.numeric_preview = None;
        self.lane_stroke = None;
        self.mixer_drag = None;
        self.table_cursor = 0;
        self.table_anchor = None;
        self.table_scroll.scroll_to_item(0, gpui::ScrollStrategy::Nearest);
        self.point_scroll.scroll_to_item(0, gpui::ScrollStrategy::Nearest);
        self.needs_fit = true;
    }

    /// Callers may move focus or act on the target only after admission.
    pub fn select_track(&mut self, id: TrackId, cx: &mut Context<Self>) -> bool {
        if self.project().track(&id).is_none() {
            return false;
        }
        if self.session.track == id {
            return true;
        }
        if self.session.gesture.is_some() {
            return false;
        }
        if !self.commit_pending_lyric(cx) {
            return false;
        }
        self.cancel_device_drag(cx);
        self.change_active_track(id);
        self.sync_index();
        self.sync_mixer();
        self.replan(cx);
        cx.notify();
        true
    }

    pub fn edit_header(&mut self, id: &TrackId, label: impl Into<String>, cx: &mut Context<Self>, f: impl FnOnce(&mut TrackHeader)) {
        let Some(t) = self.project().track(id) else { return };
        let mut h = t.header();
        f(&mut h);
        let cmd = Command::SetTrackHeader { track: id.clone(), header: h };
        if self.commit(self.revision(), CommandBatch::single(label, cmd), cx) {
            self.sync_mixer();
        }
    }

    pub fn toggle_mute(&mut self, id: &TrackId, cx: &mut Context<Self>) {
        let on = self.project().track(id).is_some_and(|t| !t.muted);
        self.edit_header(id, if on { "静音" } else { "取消静音" }, cx, |h| h.muted = on);
    }

    pub fn toggle_solo(&mut self, id: &TrackId, cx: &mut Context<Self>) {
        let on = self.project().track(id).is_some_and(|t| !t.solo);
        self.edit_header(id, if on { "独奏" } else { "取消独奏" }, cx, |h| h.solo = on);
    }

    pub fn cycle_color(&mut self, id: &TrackId, cx: &mut Context<Self>) {
        let cur = self.project().track(id).and_then(|t| t.color.clone()).unwrap_or_default();
        let i = TRACK_COLORS.iter().position(|c| *c == cur).map(|i| (i + 1) % TRACK_COLORS.len()).unwrap_or(0);
        self.edit_header(id, "轨道颜色", cx, |h| h.color = Some(TRACK_COLORS[i].into()));
    }

    pub fn add_track(&mut self, cx: &mut Context<Self>) {
        let n = self.project().tracks.len();
        if n >= verdant_audio::MAX_TRACKS {
            self.notice(format!("最多 {} 条轨道", verdant_audio::MAX_TRACKS));
            return;
        }
        let color = TRACK_COLORS[n % TRACK_COLORS.len()];
        let mut track = Track::new(format!("人声 {}", n + 1), color);
        track.render_route = Some(Route::VerdantVoice.as_str().into());
        let id = track.id.clone();
        let index = self.track_index(&self.session.track).map(|i| i + 1).unwrap_or(n);
        if self.commit(self.revision(), CommandBatch::single("新建轨道", Command::AddTrack { index, track }), cx) {
            self.sync_mixer();
            self.select_track(id, cx);
        }
    }

    pub fn delete_active_track(&mut self, cx: &mut Context<Self>) {
        self.delete_track(self.session.track.clone(), cx);
    }

    fn delete_track(&mut self, id: TrackId, cx: &mut Context<Self>) {
        if self.project().tracks.len() <= 1 {
            self.notice("工程至少保留一条轨道");
            return;
        }
        // Native-editor changes are an earlier edit: capture them before the
        // removal snapshot so the first Undo restores their current state.
        // An unchanged capture adds no history; deletion itself is one batch.
        if !self.flush_insert_states(cx) { return; }
        self.commit(self.revision(), CommandBatch::single("删除轨道", Command::RemoveTrack { track: id }), cx);
    }

    /// Push track and master settings to the audio atomics, applying live
    /// mixer and numeric previews without changing the stored project.
    pub fn sync_mixer(&mut self) {
        let drag = self.mixer_drag.clone();
        let any_solo = self.project().tracks.iter().any(|track| track.solo);
        for (i, t) in self.project().tracks.iter().enumerate() {
            let mut h = match &drag {
                Some(d) if d.track == t.id => d.header.clone(),
                _ => t.header(),
            };
            if let Some((id, crate::generated::controls::CommandArg::Track(track), value)) = &self.numeric_preview {
                if track == &t.id {
                    match id {
                        crate::generated::controls::NumericId::TrackGainDb => h.gain_millidecibels = (value * 1000.0).round() as i32,
                        crate::generated::controls::NumericId::TrackPan => h.pan_permille = (value * 10.0).round() as i32,
                        _ => {},
                    }
                }
            }
            self.audio.set_channel(i, crate::mix::channel_settings(&h, any_solo));
        }
        let mut master = self.master_bus();
        if let Some((id, crate::generated::controls::CommandArg::None, value)) = &self.numeric_preview {
            match id {
                crate::generated::controls::NumericId::MasterGainDb => master.gain_millidecibels = (value * 1000.0).round() as i32,
                crate::generated::controls::NumericId::MasterPan => master.pan_permille = (value * 10.0).round() as i32,
                _ => {},
            }
        }
        self.audio.set_master(master.channel_settings());
    }

    pub fn master_bus(&self) -> crate::mix::MasterBusSettings {
        self.master_drag.as_ref().map(|d| d.settings).unwrap_or_else(|| crate::mix::MasterBusSettings::from_project(self.project()))
    }

    pub fn edit_master_bus(&mut self, label: impl Into<String>, cx: &mut Context<Self>, f: impl FnOnce(&mut crate::mix::MasterBusSettings)) {
        let mut settings = crate::mix::MasterBusSettings::from_project(self.project());
        f(&mut settings);
        let settings = settings.clamped();
        let cmd = Command::SetProjectExtension {
            key: crate::mix::MASTER_BUS_KEY.into(),
            value: settings.extension_value(),
        };
        if self.commit(self.revision(), CommandBatch::single(label, cmd), cx) {
            self.sync_mixer();
        }
    }

    pub fn begin_master_drag(&mut self, control: MixerControl, origin_pos: f64, origin_value: f64) {
        self.master_drag = Some(MasterMixerDrag {
            control,
            settings: crate::mix::MasterBusSettings::from_project(self.project()),
            base: self.revision(),
            origin_pos,
            origin_value,
        });
    }

    pub fn update_master_drag(&mut self, f: impl FnOnce(&mut crate::mix::MasterBusSettings)) {
        if let Some(d) = self.master_drag.as_mut() {
            f(&mut d.settings);
            d.settings = d.settings.clamped();
        }
        self.sync_mixer();
    }

    pub fn end_master_drag(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.master_drag.take() else { return };
        let cmd = Command::SetProjectExtension {
            key: crate::mix::MASTER_BUS_KEY.into(),
            value: d.settings.clamped().extension_value(),
        };
        self.commit(d.base, CommandBatch::single("调整主总线推子 / 声像", cmd), cx);
        self.sync_mixer();
    }

    pub fn begin_mixer_drag(&mut self, control: MixerControl, id: &TrackId, origin_pos: f64, origin_value: f64) {
        if let Some(t) = self.project().track(id) {
            self.mixer_drag =
                Some(MixerDrag { control, track: id.clone(), header: t.header(), base: self.revision(), origin_pos, origin_value });
        }
    }

    pub fn update_mixer_drag(&mut self, f: impl FnOnce(&mut TrackHeader)) {
        if let Some(d) = self.mixer_drag.as_mut() {
            f(&mut d.header);
        }
        self.sync_mixer();
    }

    pub fn end_mixer_drag(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.mixer_drag.take() else { return };
        let label = "调整推子 / 声像";
        let cmd = Command::SetTrackHeader { track: d.track.clone(), header: d.header };
        self.commit(d.base, CommandBatch::single(label, cmd), cx);
        self.sync_mixer();
    }

    /// Read peak meters and apply simple ballistics (instant attack, decay).
    pub fn update_meters(&mut self) {
        let n = self.project().tracks.len();
        let (raw, master) = self.audio.take_meters(n);
        self.meters.resize(n, (0.0, 0.0));
        for (m, r) in self.meters.iter_mut().zip(raw) {
            m.0 = r.0.max(m.0 * 0.86);
            m.1 = r.1.max(m.1 * 0.86);
        }
        self.master_meter = (master.0.max(self.master_meter.0 * 0.86), master.1.max(self.master_meter.1 * 0.86));
    }

    // ---- note parameters --------------------------------------------------------------

    pub fn selected_notes(&self) -> Vec<Note> {
        let mut notes: Vec<_> = self.active_track().map(|t| self.session.selection.iter()
            .filter_map(|id| self.index.note(t, id).cloned()).collect()).unwrap_or_default();
        notes.sort_by(|a, b| (a.start_tick, &a.id).cmp(&(b.start_tick, &b.id)));
        notes
    }

    pub fn set_vibrato(&mut self, v: Option<Vibrato>, cx: &mut Context<Self>) {
        let ids = self.session.selection_in_order(self.project());
        if ids.is_empty() {
            return;
        }
        let label = if v.is_some() { "颤音" } else { "移除颤音" };
        let cmd = Command::SetVibrato { track: self.session.track.clone(), note_ids: ids, vibrato: v };
        self.commit(self.revision(), CommandBatch::single(label, cmd), cx);
    }

    pub fn adjust_vibrato(&mut self, f: impl Fn(&mut Vibrato), cx: &mut Context<Self>) {
        let commands = self.selected_notes().into_iter().map(|n| {
            let mut v = n.vibrato.unwrap_or(Vibrato::DEFAULT);
            f(&mut v);
            v.depth_cents = v.depth_cents.min(200);
            v.rate_millihertz = v.rate_millihertz.clamp(1000, 12000);
            v.delay_permille = v.delay_permille.min(1000);
            Command::SetVibrato { track: self.session.track.clone(), note_ids: vec![n.id], vibrato: Some(v) }
        }).collect::<Vec<_>>();
        if !commands.is_empty() { self.commit(self.revision(), CommandBatch { label: "调整颤音".into(), commands }, cx); }
    }

    pub fn set_portamento(&mut self, ms: Option<u32>, cx: &mut Context<Self>) {
        let ids = self.session.selection_in_order(self.project());
        if ids.is_empty() {
            return;
        }
        let cmd = Command::SetPortamento { track: self.session.track.clone(), note_ids: ids, portamento_ms: ms.map(|m| m.min(2000)) };
        self.commit(self.revision(), CommandBatch::single("滑音", cmd), cx);
    }

    /// Commit a pencil/eraser stroke in a numeric curve lane: for every note
    /// the stroke crosses, replace that note's points inside the stroke span
    /// (one transaction for the whole stroke).
    pub fn commit_lane_stroke(&mut self, cx: &mut Context<Self>) {
        let Some(stroke) = self.lane_stroke.take() else { return };
        if stroke.tool==LaneTool::Select&&stroke.point.is_none(){
            let Some(first)=stroke.samples.first()else{return;};let last=stroke.samples.last().unwrap_or(first);
            let(lo,hi)=(first.0.min(last.0),first.0.max(last.0));let(bottom,top)=(first.1.min(last.1),first.1.max(last.1));
            self.point_selection=self.active_track().into_iter().flat_map(|t|t.notes.iter()).flat_map(|n|stroke.param.point_samples(n).into_iter().filter(move|(_,tick,value)|{let absolute=(n.start_tick+tick)as f64;absolute>=lo&&absolute<=hi&&*value>=bottom&&*value<=top}).map(|(id,_,_)|(n.id.clone(),id))).collect();cx.notify();return;
        }
        if stroke.point.is_some() { self.commit_point_drag(stroke, cx); return; }
        let Some(track) = self.active_track().cloned() else { return };
        if stroke.samples.is_empty() || stroke.base != self.revision() {
            cx.notify();
            return;
        }
        // Line mode reduces the stroke to its endpoints; freehand replaces
        // the samples with a binomial-filtered version (removes hand jitter).
        let mut samples = stroke.samples.clone();
        match stroke.tool {
            LaneTool::Line if samples.len() > 2 => {
                let (a, b) = (samples[0], *samples.last().unwrap());
                samples = vec![a, b];
            }
            LaneTool::Pencil | LaneTool::Smooth if samples.len() > 4 => {
                let mut out = samples.clone();
                for i in 2..samples.len() - 2 {
                    out[i].1 = (samples[i - 2].1 + 4.0 * samples[i - 1].1 + 6.0 * samples[i].1 + 4.0 * samples[i + 1].1 + samples[i + 2].1) / 16.0;
                }
                samples = out;
            }
            _ => {}
        }
        let stroke = LaneStroke { samples, ..stroke };
        let (lo, hi) = stroke.samples.iter().fold((f64::MAX, f64::MIN), |(a, b), s| (a.min(s.0), b.max(s.0)));
        let mut commands = Vec::new();
        if stroke.param == Param::Portamento {
            for note in track.notes_in_range(lo.floor() as i64, hi.ceil() as i64 + 1) {
                let Some(value) = crate::curve_span::note_value(&stroke.samples, note.start_tick, note.duration_tick,
                    Param::Portamento.default_value(), stroke.erase) else { continue; };
                let points = verdant_backend::params::portamento_points_for_value(note, value);
                commands.push(Command::SetExpressionCurve { track: track.id.clone(), note_id: note.id.clone(), parameter: "POR".into(), points });
            }
            if !commands.is_empty() {
                self.commit(stroke.base, CommandBatch { label: if stroke.erase { "重置 POR 音符起点值" } else { "设置 POR 音符起点值" }.into(), commands }, cx);
            }
            cx.notify();
            return;
        }
        if stroke.param == Param::Vibrato {
            for n in track.notes_in_range(lo.floor() as i64, hi.ceil() as i64 + 1) {
                let Some(v) = crate::curve_span::note_value(
                    &stroke.samples, n.start_tick, n.duration_tick, 0.0, stroke.erase,
                ) else {
                    continue;
                };
                let base = n.vibrato.clone().unwrap_or(Vibrato::DEFAULT);
                let vib = (v >= 1.0).then_some(Vibrato { depth_cents: v as u32, ..base });
                commands.push(Command::SetVibrato { track: track.id.clone(), note_ids: vec![n.id.clone()], vibrato: vib });
            }
            if !commands.is_empty() {
                let label = format!("{} {}", if stroke.erase { "擦除" } else { "设置" }, stroke.param.short());
                self.commit(stroke.base, CommandBatch { label, commands }, cx);
            }
            cx.notify();
            return;
        }
        for n in track.notes_in_range(lo.floor() as i64, hi.ceil() as i64 + 1).filter(|n| stroke.note.as_ref().is_none_or(|id| id == &n.id)) {
            let rel_lo = (lo - n.start_tick as f64).max(0.0);
            let rel_hi = (hi - n.start_tick as f64).min(n.duration_tick as f64);
            if rel_hi < rel_lo {
                continue;
            }
            let Some(span) = crate::curve_span::prepare_stroke(&stroke.param.samples(n), &stroke.samples,
                n.start_tick, n.duration_tick, stroke.param.default_value(), stroke.erase) else { continue; };
            let fresh = &span.fresh;
            let keep = |tick: i64| span.keeps(tick);
            match stroke.param {
                Param::Pit => {
                    let mut pts: Vec<_> = n.curves.pitch_millicent_points.iter().filter(|p| keep(p.tick)).cloned().collect();
                    pts.extend(fresh.iter().map(|(t, v)| verdant_domain::model::PitchPoint {
                        id: verdant_domain::PointId::generate(),
                        tick: *t,
                        millicents: (v * 1000.0).round() as i64,
                        unknown: Default::default(),
                    }));
                    commands.push(Command::SetPitchCurve { track: track.id.clone(), note_id: n.id.clone(), points: pts });
                }
                Param::Dyn => {
                    let mut pts: Vec<_> = n.curves.gain_millidecibel_points.iter().filter(|p| keep(p.tick)).cloned().collect();
                    pts.extend(fresh.iter().map(|(t, v)| verdant_domain::model::GainPoint {
                        id: verdant_domain::PointId::generate(),
                        tick: *t,
                        millidecibels: (v * 1000.0).round() as i64,
                        unknown: Default::default(),
                    }));
                    commands.push(Command::SetGainCurve { track: track.id.clone(), note_id: n.id.clone(), points: pts });
                }
                p if p.is_expression() => {
                    let mut pts: Vec<_> = n.curves.expression(p.short()).iter().filter(|p| keep(p.tick)).cloned().collect();
                    pts.extend(fresh.iter().map(|(t, v)| verdant_domain::model::ExpressionPoint {
                        id: verdant_domain::PointId::generate(), tick: *t,
                        value: v.round().clamp(0., 127.) as u8, unknown: Default::default(),
                    }));
                    commands.push(Command::SetExpressionCurve { track: track.id.clone(), note_id: n.id.clone(), parameter: p.short().into(), points: pts });
                }
                _ => {}
            }
        }
        if !commands.is_empty() {
            let label = format!("{} {}", if stroke.erase { "擦除" } else { "绘制" }, stroke.param.short());
            self.commit(stroke.base, CommandBatch { label, commands }, cx);
        }
        cx.notify();
    }

    pub fn nudge_tempo(&mut self, delta_bpm: f64, cx: &mut Context<Self>) {
        let tick = self.tempo_event_tick();
        let bpm = (self.tempo().bpm_at(tick) + delta_bpm).clamp(30.0, 300.0);
        self.commit(
            self.revision(),
            CommandBatch::single(format!("速度 {bpm:.0} BPM"), Command::SetTempoEvent { tick, micro_bpm: Some((bpm * 1e6) as u64) }),
            cx,
        );
    }

    // ---- lyric editing (SPEC §7) ---------------------------------------------

    pub fn lyric_display(&self, id: &NoteId) -> String {
        let Some(n) = self.project().track(&self.session.track).and_then(|t| t.note(id)) else { return String::new() };
        if n.pronunciation.alphabet.is_some() {
            return format!("{}/{}", n.lyric, verdant_domain::pronunciation_display(&n.pronunciation));
        }
        let py = &n.pronunciation.pinyin;
        if !py.is_empty() && normalize_pinyin(py) != normalize_pinyin(&n.lyric) { format!("{}/{}", n.lyric, py) } else { n.lyric.clone() }
    }

    pub fn begin_lyric(&mut self, id: NoteId, window: &mut Window, cx: &mut Context<Self>) {
        if self.lyric.as_ref().is_some_and(|l| l.note_id == id) {
            return;
        }
        self.end_lyric(true, None, window, cx);
        if self.lyric.is_some() {
            return;
        }
        let text = self.lyric_display(&id);
        if let Some(n) = self.project().track(&self.session.track).and_then(|t| t.note(&id)).cloned() {
            self.session.viewport.reveal(&n);
        }
        self.session.selection = [id.clone()].into();
        self.lyric = Some(LyricSession::new(self.session.track.clone(), id, &text));
        window.focus(&self.focus_lyric, cx);
        cx.notify();
    }

    /// Turn a lyric field text into one transaction (lyric + explicit pinyin).
    pub fn commit_lyric_text(&mut self, id: &NoteId, text: &str, cx: &mut Context<Self>) {
        let track = self.session.track.clone();
        self.commit_lyric_on_track(&track, id, text, cx);
    }

    fn commit_lyric_on_track(&mut self, track: &TrackId, id: &NoteId, text: &str, cx: &mut Context<Self>) -> bool {
        let Some(note) = self.project().track(track).and_then(|t| t.note(id)).cloned() else { return false };
        let commands = match self.lyric_commands(track, &note, text) {
            Ok(commands) => commands,
            Err(reason) => { self.notice(reason); cx.notify(); return false; }
        };
        // A normalized no-op is also a successful confirmation. On failure,
        // callers retaining pending text must be able to retry it unchanged.
        match self.persistence.commit(self.revision(), CommandBatch { label: "编辑歌词".into(), commands }) {
            Ok(Some(_)) => { self.after_change(cx); true }
            Ok(None) => true,
            Err(e) => { self.notice(format!("未提交：{e}")); cx.notify(); false }
        }
    }

    pub(crate) fn commit_pending_lyric(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(lyric) = self.lyric.take() else { return true };
        if lyric.state.is_composing() {
            self.lyric = Some(lyric);
            self.notice("请先完成歌词输入法组合，再切换轨道或关闭");
            cx.notify();
            return false;
        }
        if self.project().track(&lyric.track).and_then(|t| t.note(&lyric.note_id)).is_none() {
            self.lyric_line = None;
            self.lyric_bounds = None;
            return true;
        }
        if let LyricOutcome::Commit(text) = lyric.clone().confirm() {
            if !self.commit_lyric_on_track(&lyric.track, &lyric.note_id, &text, cx) {
                self.lyric = Some(lyric);
                return false;
            }
        }
        self.lyric_line = None;
        self.lyric_bounds = None;
        true
    }

    /// Close the lyric editor. `confirm` commits pending text; `advance`
    /// moves on to the next/previous note (Tab / Shift-Tab).
    pub fn end_lyric(&mut self, confirm: bool, advance: Option<bool>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(l) = self.lyric.take() else { return };
        if confirm {
            let exists = self.project().track(&l.track).and_then(|t| t.note(&l.note_id)).is_some();
            let rejected = exists && (l.state.is_composing() || match l.clone().confirm() {
                LyricOutcome::Commit(text) => !self.commit_lyric_on_track(&l.track, &l.note_id, &text, cx),
                _ => false,
            });
            if rejected {
                self.lyric = Some(l);
                window.focus(&self.focus_lyric, cx);
                cx.notify();
                return;
            }
        }
        self.lyric_line = None;
        self.lyric_bounds = None;
        if let Some(forward) = advance {
            let p = self.store().snapshot();
            self.session.selection = [l.note_id.clone()].into();
            if let Some(next) = self.session.step_selection(&p, forward) {
                if next != l.note_id {
                    self.begin_lyric(next, window, cx);
                    return;
                }
            }
        }
        window.focus(&self.focus_roll, cx);
        cx.notify();
    }

    /// IME commit arrives through `replace_text_in_range` (see `lyric_view`).
    pub fn lyric_ime_commit(&mut self, range: Option<std::ops::Range<usize>>, text: &str, cx: &mut Context<Self>) {
        let Some(l) = self.lyric.as_mut() else { return };
        let id = l.note_id.clone();
        if let LyricOutcome::Commit(full) = l.on_ime_commit(range, text) {
            self.commit_lyric_text(&id, &full, cx);
        }
        cx.notify();
    }

    // ---- backend ------------------------------------------------------------------

    pub fn start_worker(&mut self, cx: &mut Context<Self>) {
        if let Some(w) = self.worker.take() {
            w.kill();
        }
        self.worker_gen += 1;
        let generation = self.worker_gen;
        match WorkerClient::spawn(self.worker_cfg.clone(), generation) {
            Ok((client, rx)) => {
                self.worker = Some(client);
                self.worker_status = WorkerStatus::Starting;
                cx.spawn(async move |this, cx| {
                    while let Ok((g, msg)) = rx.recv().await {
                        if this.update(cx, |ed, cx| ed.on_worker_message(g, msg, cx)).is_err() {
                            break;
                        }
                    }
                })
                .detach();
            }
            Err(e) => {
                self.worker_status = WorkerStatus::SpawnFailed(e.to_string());
                self.notice(format!("无法启动后端：{e}"));
            }
        }
    }

    fn send(&mut self, req: verdant_backend::Request) {
        let Some(w) = self.worker.as_ref() else {
            self.notice("后端未连接");
            return;
        };
        if let Err(e) = w.send(&req) {
            self.notice(format!("发送失败：{e}"));
        }
    }

    fn on_worker_message(&mut self, generation: u64, msg: WorkerMessage, cx: &mut Context<Self>) {
        if generation != self.worker_gen {
            return; // late output from a previous worker process
        }
        match msg {
            WorkerMessage::Event(ev) => {
                let mut replan = false;
                match &ev {
                    WorkerEvent::Ready { session_id, workspace } => {
                        self.worker_status = WorkerStatus::Ready { session_id: session_id.clone() };
                        self.worker_workspace = Some(PathBuf::from(workspace));
                        self.send(verdant_backend::Request::capabilities(format!("cap-{generation}")));
                    }
                    WorkerEvent::Capabilities { .. } => replan = true,
                    WorkerEvent::VoiceAdmitted { display_name, reference, .. } => { self.notice(format!("已安装歌手：{display_name}（{reference}）；在歌手菜单中选择后即可合成")); self.send(verdant_backend::Request::capabilities(format!("voices-{}",NoteId::generate()))); },
                    WorkerEvent::Failed { code, message, .. } => self.notice(format!("渲染失败 {code}: {message}")),
                    WorkerEvent::Rejected { code, message, .. } => self.notice(format!("后端拒绝 {code}: {message}")),
                    _ => {}
                }
                let actions = self.scheduler.on_event(ev);
                self.run_actions(actions, cx);
                if replan {
                    self.replan(cx);
                }
                self.pump(cx);
            }
            WorkerMessage::Malformed { line, error } => {
                self.notice(format!("忽略无效后端输出：{error}"));
                self.malformed.push(format!("{error}: {line}"));
            }
            WorkerMessage::Exited { status } => {
                self.scheduler.on_worker_exit(&status);
                self.worker = None;
                self.worker_status = WorkerStatus::Exited(status.clone());
                self.notice(format!("后端进程已退出（{status}），可重启"));
            }
        }
        cx.notify();
    }

    fn run_actions(&mut self, actions: Vec<Action>, cx: &mut Context<Self>) {
        for a in actions {
            match a {
                Action::Send(req) => self.send(req),
                Action::Verify { job_id, route, wideband, artifacts } => {
                    let Some(ws) = self.worker_workspace.clone() else { continue };
                    let worker_generation = self.worker_gen;
                    // Durable-cache storage persists the same CacheEntry the
                    // scheduler will build in on_verified — capture the slot
                    // fields now while the job record is live.
                    let entry_meta = self.scheduler.job(&job_id).map(|j| {
                        let w = self.scheduler.wanted().iter().find(|w| w.digest == j.digest);
                        (
                            j.digest.clone(),
                            j.revision,
                            w.map(|w| w.phrase.track.clone()),
                            w.map(|w| w.phrase.index),
                            w.map(|w| w.phrase.start_seconds).unwrap_or(0.0),
                            w.map(|w| w.phrase.seconds).unwrap_or(0.0),
                        )
                    });
                    cx.spawn(async move |this, cx| {
                        let cache_dir = crate::files::render_cache_dir();
                        let store_job_id = job_id.clone();
                        let (result, stored) = cx
                            .background_spawn(async move {
                                let result = verdant_backend::artifact::verify_and_decode_with_options(&ws, route, wideband, &artifacts);
                                // Persist verified bytes into the durable
                                // digest cache (blocking FS — background).
                                let stored = match &result {
                                    Ok(audio) => match &entry_meta {
                                        Some((digest, revision, track, phrase_index, start_seconds, seconds)) => {
                                            let store_result = ArtifactCache::open(&cache_dir).and_then(|cache| {
                                                cache
                                                    .store(&verdant_backend::scheduler::CacheEntry {
                                                        digest: digest.clone(),
                                                        job_id: store_job_id.clone(),
                                                        revision: *revision,
                                                        route,
                                                        audio: audio.clone(),
                                                        track: track.clone(),
                                                        phrase_index: *phrase_index,
                                                        start_seconds: *start_seconds,
                                                        seconds: *seconds,
                                                        last_wanted_revision: *revision,
                                                    })
                                                    .map_err(|e| std::io::Error::other(e.to_string()))
                                            });
                                            store_result.err().map(|e| e.to_string())
                                        }
                                        None => None,
                                    },
                                    Err(_) => None,
                                };
                                (result, stored)
                            })
                            .await;
                        let _ = this.update(cx, |ed, cx| {
                            if ed.worker_gen != worker_generation { return; }
                            if let Some(e) = stored {
                                ed.notice(format!("缓存写入失败：{e}"));
                            }
                            if let Err(e) = &result {
                                ed.notice(format!("制品校验失败：{e}"));
                            }
                            ed.scheduler.on_verified(&job_id, result);
                            ed.rebuild_timeline(cx);
                            ed.pump(cx);
                            cx.notify();
                        });
                    })
                    .detach();
                }
            }
        }
    }

    pub fn schedule_replan(&mut self, cx: &mut Context<Self>) {
        // An edit withdraws playable bindings immediately. The debounced plan
        // may reuse only digest-matched content once it has been recomputed.
        self.render_armed |= self.auto_render;
        self.timeline_gen += 1;
        self.timeline_phrases = 0;
        self.ready_notes.clear();
        self.render_audio.clear();
        self.phrase_pcm = project_audio::PhrasePcmCache::default();
        self.phrase_slots.clear();
        self.plans.clear();
        self.scheduled_phonemes.clear();
        self.error_notes.clear();
        self.cache_restore_gen += 1;
        self.cache_restoring = false;
        self.scheduler.set_plans(self.revision().get(), &[], self.project().render_preferences.native_seed);
        self.roll_pitch.rebuild(&self.scheduler);
        self.pump(cx);
        self.audio.set_timeline(Timeline::new(self.audio_info.sample_rate, Vec::new()));
        self.replan_gen += 1;
        let g = self.replan_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REPLAN_DEBOUNCE).await;
            let _ = this.update(cx, |ed, cx| {
                if ed.replan_gen == g {
                    ed.replan(cx);
                }
            });
        })
        .detach();
    }

    /// Rehydrate the durable digest cache into the current plan: every wanted
    /// digest that already has verified bytes on disk binds immediately
    /// (digest-matched, so an old revision can never replace current
    /// content). Filesystem I/O runs on the background executor.
    fn restore_cached_renders(&mut self, cx: &mut Context<Self>) {
        let digests: Vec<String> = self
            .scheduler
            .wanted()
            .iter()
            .filter(|w| !self.scheduler.has_digest(&w.digest))
            .map(|w| w.digest.clone())
            .collect();
        if digests.is_empty() {
            self.cache_restoring = false;
            self.pump(cx);
            return;
        }
        self.cache_restoring = true;
        self.cache_restore_gen += 1;
        let generation = self.cache_restore_gen;
        let dir = crate::files::render_cache_dir();
        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move {
                    let cache = match ArtifactCache::open(&dir) {
                        Ok(cache) => cache,
                        Err(e) => return vec![Err(e.to_string())],
                    };
                    let mut out = Vec::new();
                    for d in digests {
                        match cache.restore(&d) {
                            Ok(Some(entry)) => out.push(Ok(entry)),
                            Ok(None) => {}
                            Err(e) => out.push(Err(e.to_string())),
                        }
                    }
                    out
                })
                .await;
            let _ = this.update(cx, |ed, cx| {
                if ed.cache_restore_gen != generation { return; }
                ed.cache_restoring = false;
                let mut n = 0usize;
                for e in entries {
                    match e {
                        Ok(entry) => {
                            ed.scheduler.import_verified(entry);
                            n += 1;
                        }
                        Err(msg) => ed.notice(format!("缓存制品损坏，未播放：{msg}")),
                    }
                }
                if n > 0 {
                    ed.notice(format!("已从缓存恢复 {n} 个乐句"));
                    ed.rebuild_timeline(cx);
                    cx.notify();
                }
                ed.pump(cx);
            });
        })
        .detach();
    }

    pub fn replan(&mut self, cx: &mut Context<Self>) {
        self.phone_budgets.get_mut().clear();
        let Some(caps) = self.scheduler.capabilities.clone() else {
            self.plans.clear();
            self.scheduled_phonemes.clear();
            return;
        };
        let plans = adapter::plan_all_desktop(self.project(), &self.session.track, &caps);
        let project=self.store().snapshot();
        self.scheduler.set_desktop_project_voices(&project);
        self.scheduler.set_plans(self.revision().get(), &plans, project.render_preferences.native_seed);
        self.scheduled_phonemes.rebuild(&plans);
        self.plans = plans;
        self.phrase_slots = self.scheduler.wanted().iter().enumerate().map(|(i, w)| ((w.phrase.track.clone(), w.phrase.index), i)).collect();
        self.sync_arrange_cache(true);
        self.error_notes = self.plans.iter().flat_map(|p| p.diagnostics.iter()).filter(|d| d.severity == verdant_backend::Severity::Error).filter_map(|d| d.note_id.clone()).collect();
        self.restore_cached_renders(cx);
        self.rebuild_timeline(cx);
        cx.notify();
    }

    pub fn render_now(&mut self, cx: &mut Context<Self>) {
        self.scheduler.retry_failed();
        self.render_armed = true;
        if self.worker.is_none() {
            self.start_worker(cx);
            return;
        }
        self.replan(cx);
    }

    /// Send whatever the scheduler wants next. Cancels for superseded jobs are
    /// always sent; new renders only when armed.
    pub fn pump(&mut self, cx: &mut Context<Self>) {
        if self.cache_restoring { return; }
        if !self.render_armed && self.scheduler.active().is_none() {
            return;
        }
        self.scheduler.set_playhead_seconds(self.playhead_seconds());
        let actions = self.scheduler.poll_pending(self.render_armed);
        self.run_actions(actions, cx);
        if self.scheduler.all_ready() && !self.auto_render {
            self.render_armed = false;
        }
    }

    pub fn cancel_active(&mut self, cx: &mut Context<Self>) {
        self.render_armed = false;
        let jobs: Vec<_> = self.scheduler.active_jobs().map(|job| job.job_id.clone()).collect();
        for job in jobs {
            if let Some(req) = self.scheduler.request_cancel(&job) {
                self.send(req);
            }
        }
        cx.notify();
    }

    // ---- audio --------------------------------------------------------------------

    /// Assemble the playable binding into a device-rate timeline off the UI
    /// thread, then hand it to the audio engine.
    pub fn rebuild_timeline(&mut self,cx:&mut Context<Self>){
        self.roll_pitch.rebuild(&self.scheduler);
        self.ready_notes=self.scheduler.playable_phrases().into_iter().flat_map(|(phrase,_)|phrase.note_ids.iter().cloned()).collect();
        self.render_audio=self.scheduler.playable_phrases().into_iter().map(|(phrase,entry)|((phrase.track.clone(),phrase.index),entry.audio.clone())).collect();
        let sources = self.scheduler.playable_phrases().into_iter().map(|(phrase, entry)| project_audio::PhraseSource {
            track: phrase.track.clone(),
            start_seconds: phrase.start_seconds,
            note_ids: phrase.note_ids.clone(),
            phoneme_timing: phrase.score.phoneme_timing.clone(),
            audio: entry.audio.clone(),
            playback_key: Some((phrase.index, entry.digest.clone())),
        }).collect::<Vec<_>>();
        let project=self.store().snapshot();let rate=self.audio_info.sample_rate;let policy=self.media_asset_policy();let settings=self.playback_settings();let bindings=self.native_bindings();
        let planned_end = self.scheduler.wanted().iter().map(|wanted| {
            let phrase = &wanted.phrase;
            ((phrase.start_seconds + phrase.seconds) * f64::from(rate)).ceil() as u64
        }).max().unwrap_or(0);
        let cached = self.phrase_pcm.for_sources(&sources, rate);
        let mut media_cache=std::mem::take(&mut self.integrations.media_cache);let mut media_previews=HashMap::new();
        self.timeline_gen+=1;let generation=self.timeline_gen;
        cx.spawn(async move|this,cx|{
            let result=cx.background_spawn(async move{
                let policy=policy?;
                let mut preparation=project_audio::MediaPreparation{cache:&mut media_cache,previews:&mut media_previews};
                let mut prepared=project_audio::prepare_timeline(&project,&sources,policy,rate,cached,Some(&mut preparation))?;
                drop(preparation);
                // Queued phrases occupy time even before they have verified PCM.
                prepared.timeline.length = prepared.timeline.length.max(planned_end);
                project_audio::bind_native_automation(&project,&mut prepared.timeline,&bindings)?;
                let tempo=TempoMap::new(&project.tempo_map,project.ppq).map_err(|e|e.to_string())?;let meter=verdant_domain::MeterMap::new(&project.time_signatures,project.ppq)?;
                let mut end_tick=tempo.tick_at(prepared.timeline.length as f64/rate as f64).ceil()as i64;
                if settings.looping{
                    if settings.loop_start<0||settings.loop_end<=settings.loop_start{return Err("循环播放范围无效".into());}
                    let range=(tempo.sample_at(settings.loop_start,rate).max(0)as u64,tempo.sample_at(settings.loop_end,rate).max(0)as u64);if range.1<=range.0{return Err("循环短于一个音频帧".into());}
                    prepared.timeline.loop_range=Some(range);prepared.timeline.length=prepared.timeline.length.max(range.1);end_tick=end_tick.max(settings.loop_end);
                }
                if settings.metronome{
                    if end_tick==0{for _ in 0..4{end_tick=meter.next_bar(end_tick);}}
                    let clicks=workflow::metronome(&project,&tempo,rate,end_tick)?;prepared.timeline.length=prepared.timeline.length.max(clicks.length);prepared.timeline.metronome=Some(clicks);
                }
                project_audio::extend_realtime_timeline(&mut prepared.timeline,&bindings)?;
                Ok::<_,String>((prepared,media_previews,media_cache))
            }).await;
            let _=this.update(cx,|ed,cx|{if ed.timeline_gen!=generation{return;}match result{
                Ok((prepared,media_previews,media_cache))=>{ed.integrations.media_cache=media_cache;ed.integrations.media_previews=media_previews;ed.phrase_pcm=prepared.phrase_pcm;ed.timeline_phrases=prepared.phrase_count;ed.timeline_instruments=prepared.instrument_count;ed.integrations.media_ready=prepared.media_count>0;let revision=ed.audio.set_timeline(prepared.timeline);ed.insert_timeline_ready(revision,cx);}
                Err(error)=>{ed.audio.set_timeline(Timeline::new(rate,Vec::new()));ed.timeline_phrases=0;ed.timeline_instruments=0;ed.integrations.media_ready=false;ed.integrations.media_previews.clear();ed.notice(format!("音频时序 / 资产校验失败：{error}"));}
            }cx.notify();});
        }).detach();
    }

    pub fn toggle_play(&mut self, cx: &mut Context<Self>) {
        if self.integrations.recording.is_some(){if let Err(error)=self.finish_midi_recording(cx){self.notice(format!("MIDI 录音未提交：{error}"));}self.audio.pause();cx.notify();return;}
        if !self.insert_transport_ready(cx) { return; }
        match self.audio.transport() {
            Transport::Playing => self.audio.pause(),
            _ => {
                let wanted = self.scheduler.wanted().len();
                if self.timeline_phrases == 0 && self.timeline_instruments == 0 && !self.integrations.media_ready && !self.playback_settings().metronome {
                    // Never substitute another sound for a missing render.
                    let msg = if wanted == 0 {
                        "没有可渲染的乐句（查看诊断）".to_string()
                    } else {
                        format!("尚无可播放的渲染结果：{wanted} 个乐句待渲染")
                    };
                    self.notice(msg);
                } else {
                    if self.timeline_phrases < wanted {
                        self.notice(format!("部分播放：{}/{} 个乐句已就绪，其余静音", self.timeline_phrases, wanted));
                    }
                    let at_end=self.audio.position()>=self.audio.shared.timeline_len.load(std::sync::atomic::Ordering::Acquire);
                    let fresh=self.audio.transport()==Transport::Stopped||at_end;
                    if at_end{self.seek_tick(0.);}
                    let bars = self.playback_settings().count_in_bars;
                    if fresh && bars > 0 {
                        let tempo = self.tempo();
                        let tick = tempo.tick_at(self.playhead_seconds()).round() as i64;
                        match workflow::count_in_track(self.project(), &tempo, self.audio_info.sample_rate, tick, bars)
                            .and_then(|clicks| clicks.count_in_pcm())
                        {
                            Ok(samples) => {
                                if !self.audio.play_with_count_in(samples) { self.notice("预备拍队列忙，请重试"); }
                            }
                            Err(error) => self.notice(error),
                        }
                    } else { self.audio.play(); }
                }
            }
        }
        cx.notify();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.integrations.recording.is_some(){if let Err(error)=self.finish_midi_recording(cx){self.notice(format!("MIDI 录音未提交：{error}"));}}
        self.stop_insert_transport();
        self.audio.stop();
        // A held insert transport must resume at the same loop/project start
        // as the audio engine and the immediately sampled playhead.
        self.seek_insert_transport(self.audio.position());
        self.clock_sample_once();
        cx.notify();
    }

    /// Close from Verdant's own title-bar button: same checks as the system close
    /// and the Quit action.
    pub fn close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        crate::files::capture_bounds(&mut self.prefs, window);
        // The editor window is the app (as with the Quit action); its tool
        // windows belong to it, so closing it ends the session.
        if self.shutdown(cx) { cx.quit(); }
    }

    /// Add a parameter lane (the first parameter not already shown, or
    /// `param`) and make it the active one.
    pub fn add_lane(&mut self, param: Option<Param>, cx: &mut Context<Self>) {
        let lanes = &mut self.integrations.lanes;
        let Some(param) = param.or_else(|| verdant_backend::params::ALL.into_iter().find(|p| !lanes.contains(p))) else { return };
        if !lanes.contains(&param) { lanes.push(param); }
        self.integrations.active_panel = lanes.iter().position(|p| *p == param).unwrap_or(0);
        self.param = param;
        self.point_selection.clear();
        cx.notify();
    }

    /// Remove a lane; one lane always stays.
    pub fn remove_lane(&mut self, slot: usize, cx: &mut Context<Self>) {
        let lanes = &mut self.integrations.lanes;
        if lanes.len() <= 1 || slot >= lanes.len() { return; }
        lanes.remove(slot);
        self.integrations.active_panel = self.integrations.active_panel.min(lanes.len() - 1);
        self.param = lanes[self.integrations.active_panel];
        self.point_selection.clear();
        cx.notify();
    }

    pub fn left_panel_visible(&self) -> bool {
        if self.width_class.docks_panels() { self.show_inspector } else { self.drawer == Some(crate::ui_controls::Drawer::Left) }
    }

    pub fn right_dock_visible(&self) -> bool {
        if self.width_class.docks_panels() { self.layout.show_right_dock } else { self.drawer == Some(crate::ui_controls::Drawer::Right) }
    }

    pub fn toggle_left_panel(&mut self, cx: &mut Context<Self>) {
        use crate::ui_controls::Drawer;
        if self.width_class.docks_panels() {
            self.show_inspector = !self.show_inspector;
        } else {
            self.drawer = (self.drawer != Some(Drawer::Left)).then_some(Drawer::Left);
        }
        cx.notify();
    }

    pub fn toggle_right_dock(&mut self, cx: &mut Context<Self>) {
        use crate::ui_controls::Drawer;
        if self.width_class.docks_panels() {
            self.layout.show_right_dock = !self.layout.show_right_dock;
            self.layout.store(&mut self.prefs);
            self.prefs_dirty = true;
        } else {
            self.drawer = (self.drawer != Some(Drawer::Right)).then_some(Drawer::Right);
        }
        cx.notify();
    }

    /// Show `tab` in the right dock (as a drawer in narrow windows).
    pub fn open_right_dock(&mut self, tab: crate::ui_controls::RightDockTab, cx: &mut Context<Self>) {
        self.layout.right_dock_tab = tab;
        if self.width_class.docks_panels() {
            self.layout.show_right_dock = true;
        } else {
            self.drawer = Some(crate::ui_controls::Drawer::Right);
        }
        self.layout.store(&mut self.prefs);
        self.prefs_dirty = true;
        cx.notify();
    }

    pub fn playhead_seconds(&self) -> f64 {
        if let Some(seconds)=self.recording_seconds(){return seconds;}
        self.audio.position() as f64 / self.audio_info.sample_rate as f64
    }

    pub fn seek_tick(&mut self, tick: f64) {
        self.cancel_midi_recording("播放位置改变，MIDI 录音已取消");
        let s = self.tempo().seconds_at(tick.max(0.0));
        let frame = (s * self.audio_info.sample_rate as f64).round() as u64;
        self.seek_insert_transport(frame);
        self.audio.seek(frame);
        self.clock_sample_once();
    }

    /// Sample the audio atomics once (used after seek/stop so the LCD and
    /// playhead update while stopped).
    fn clock_sample_once(&mut self) {
        self.playhead_smoother.reset();
        self.frame_playhead = self.playhead_seconds();
    }

    /// The native next-frame pump scheduled by `ClockDriver`, before drawing:
    /// meters and playhead sampling. Returns whether the clock driver should
    /// keep animating. `ClockDriver` follows both timeline views after this
    /// pump and invalidates each viewport whose position changed.
    pub fn clock_pump(&mut self, _window: &mut Window, _edcx: &mut Context<Self>) -> bool {
        self.audio.maintain();
        self.update_meters();
        let raw = self.playhead_seconds();
        let smooth = self.audio.transport() == Transport::Playing && self.recording_seconds().is_none();
        self.frame_playhead = self.playhead_smoother.sample(raw, Instant::now(), smooth);
        let playing = self.audio.transport() == Transport::Playing;
        playing || self.meters.iter().any(|m| m.0.max(m.1) > 1e-4) || self.master_meter.0.max(self.master_meter.1) > 1e-4
    }

    /// Non-render upkeep (was `maintain`): notice expiry only; the audio
    /// atomics pumping moved to `clock_pump`.
    pub fn maintain(&mut self, _cx: &mut App) {
        self.audio.maintain();
        let now = Instant::now();
        while self.notices.front().is_some_and(|n| now.duration_since(n.at) > Duration::from_secs(6)) {
            self.notices.pop_front();
        }
    }

    /// Slow timer loop (device events, plugin events, autosave, prefs) —
    /// independent of the frame clock so it also runs while the window is
    /// idle.
    fn spawn_slow_pump(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(300)).await;
                match this.update(cx, |ed, cx| ed.pump_slow(cx)) {
                    Ok(()) => {}
                    Err(_) => break,
                }
            }
        })
        .detach();
    }

    fn pump_slow(&mut self, cx: &mut Context<Self>) {
        if self.last_pride_check.elapsed() >= Duration::from_secs(30) {
            self.last_pride_check = Instant::now();
            let active = crate::theme::PridePreference::from_prefs(&self.prefs)
                .active_on(chrono::Local::now().date_naive());
            if active != self.theme.pride {
                self.theme = self.theme.with_pride(active);
                cx.notify();
            }
        }
        self.pump_midi(cx);
        if self.audio.shared.automation_fault.swap(false,std::sync::atomic::Ordering::AcqRel){self.audio.pause();self.notice("原生插件自动化处理被拒绝；播放已暂停，请检查插件和事件密度");cx.notify();}
        let notices = self.notices.len();
        self.maintain(cx);
        if self.notices.len() != notices { cx.notify(); }
        if self.insert_specs_applied.is_empty() { self.restore_inserts(cx); }
        // Device events from the audio driver thread.
        for ev in self.engine.drain_events() {
            match ev {
                DeviceEvent::DeviceLost { .. } => self.notice("音频设备已断开，正在恢复…"),
                DeviceEvent::Recovered(_) => {
                    self.audio_info = self.engine.output_info();
                    self.notice("音频设备已恢复");
                }
                DeviceEvent::RecoveryFailed { .. } => self.notice("音频设备恢复失败，使用静默时钟"),
                DeviceEvent::OutputChanged(_) => {
                    // Timeline rate is fixed 48k — no rebuild needed; refresh
                    // the display-side info only.
                    self.audio_info = self.engine.output_info();
                }
                DeviceEvent::Rejected(e) => self.notice(format!("设备切换被拒绝：{e}")),
                DeviceEvent::RouteChanged => self.notice("音频系统已切换当前设备的输出路线"),
                DeviceEvent::RealtimeDenied => self.notice("音频设备仍在工作；系统未授予实时线程优先级"),
                DeviceEvent::CallbackError { message } => self.notice(format!("音频回调：{message}")),
            }
            cx.notify();
        }
        // Plugin host events.
        if let Some(client) = self.plugins.clone() {
            while let Some(ev) = client.try_recv_event() {
                match ev {
                    verdant_plugins::ClientEvent::Crashed { detail, .. } => {
                        self.plugin_reconfigure_pending = true;
                        self.notice(format!("插件崩溃已旁通：{detail}"));
                        cx.notify();
                    }
                    verdant_plugins::ClientEvent::ScanFinished => cx.notify(),
                    _ => {}
                }
            }
        }
        // Autosave: compact the journal when dirty (Storage contract:
        // autosave() writes the project+undo/redo snapshot atomically; the
        // journal keeps appending per commit).
        let mut plugin_changed = false;
        for slot in self.inserts.values().flatten().chain(&self.master_inserts) {
            if let verdant_plugins::InsertSlot::Live(handle) = slot {
                let counters = (handle.xruns(), handle.latency_samples());
                match self.plugin_counters.insert(handle.key(), counters) {
                    Some(old) if old != counters => {
                        plugin_changed = true;
                        self.plugin_reconfigure_pending |= old.1 != counters.1;
                    }
                    None => plugin_changed = true,
                    _ => {},
                }
            }
        }
        if plugin_changed { cx.notify(); }
        if self.plugin_reconfigure_pending && !self.inserts_loading && self.plugin_busy.is_empty() {
            self.plugin_reconfigure_pending = false;
            if self.flush_insert_states(cx) {
                self.insert_specs_applied.clear();
                self.restore_inserts(cx);
                // Latency can change without changing state or track padding.
                // An asynchronous chain install rebuilds its matching timeline.
                if !self.inserts_loading { self.rebuild_timeline(cx); }
            }
        }
        if self.last_slow.elapsed() >= Duration::from_secs(45) {
            self.last_slow = Instant::now();
            self.flush_insert_states(cx);
            if let Err(e) = self.persistence.autosave() {
                self.notice(format!("自动保存失败：{e}")); cx.notify();
            }
        }
        // Preferences flush (window bounds are captured in `render`).
        if std::mem::take(&mut self.prefs_dirty) {
            if let Err(e) = crate::files::save_prefs(&self.prefs) {
                self.notice(format!("设置保存失败：{e}")); cx.notify();
            }
        }
    }

    /// Admit a normal close only after pending UI and live insert state have
    /// been captured durably. A rejected close leaves the editor retryable.
    pub fn shutdown(&mut self, cx: &mut Context<Self>) -> bool {
        if self.shut_down { return true; }
        if self.integrations.media_file_jobs!=0 {
            self.notice("工程文件与素材正在保存，请等待完成后重试关闭");cx.notify();return false;
        }
        if !self.plugin_busy.is_empty() || self.inserts_loading {
            self.notice("插件参数或插件链正在提交，请等待完成后重试关闭");
            cx.notify();
            return false;
        }
        if !self.commit_pending_lyric(cx) || !self.flush_insert_states(cx) { return false; }
        if let Err(e) = self.persistence.autosave() {
            self.notice(format!("关闭前保存失败，未关闭：{e}")); cx.notify(); return false;
        }
        if let Err(e) = crate::files::save_prefs(&self.prefs) {
            self.notice(format!("关闭前设置保存失败，未关闭：{e}")); cx.notify(); return false;
        }
        self.shut_down = true;
        self.audio.stop();
        true
    }

    // ---- audio devices (TODO §4: choose / recover / non-f32 display) ----------

    /// Device picker menu items (mixer status area + choose command). Names
    /// come from the live engine enumeration; empty = system default.
    pub fn device_menu_items(&self) -> Vec<crate::menu::MenuItem> {
        use crate::menu::MenuItem;
        let mut items = vec![MenuItem::action("系统默认输出", MenuAction::ChooseDevice(String::new()))];
        match self.engine.devices() {
            Ok(devs) => {
                for d in devs {
                    let label = if d.is_default { format!("{}（默认）", d.name) } else { d.name.clone() };
                    items.push(MenuItem::action(label, MenuAction::ChooseDevice(d.id.unwrap_or(d.name))));
                }
            }
            Err(e) => items.push(MenuItem::disabled(format!("无法枚举设备：{e}"))),
        }
        items.push(MenuItem::separator());
        items.push(MenuItem::action("重新打开设备", MenuAction::ReopenDevice));
        items
    }

    /// Select a device by name ("" = system default). Hot reselection keeps
    /// the same `Control`/timeline/insert chains (audio contract).
    pub fn choose_device(&mut self, name: &str) {
        let sel = if name.is_empty() { DeviceSelection::Default } else { DeviceSelection::Device(name.to_string()) };
        self.engine.select(sel);
        self.prefs.device = (!name.is_empty()).then(|| name.to_string());
        self.prefs_dirty = true;
    }

}

fn recovery_text(report: &verdant_domain::RecoveryReport) -> Option<String> {
    report.recovered_anything().then(|| format!(
        "已恢复自动保存（{}，{} 步编辑）{}{}。最后手动保存的文件未被覆盖。",
        crate::files::relative_time_text(report.recovered_at_ms, chrono::Utc::now().timestamp_millis()), report.replayed_commits,
        if report.torn_tail_bytes > 0 { format!("；丢弃中断尾部 {} 字节", report.torn_tail_bytes) } else { String::new() },
        if report.completed_interrupted_save { "；已完成中断保存的恢复登记" } else { "" },
    ))
}

/// The platform UI face (see `theme::system_ui_font`).
fn pick_ui_font(cx: &mut Context<Editor>) -> gpui::SharedString {
    crate::theme::system_ui_font(&cx.text_system().all_font_names()).into()
}
