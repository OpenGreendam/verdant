//! Application implementations of declaration-generated command and numeric
//! targets. Source declarations define controls; domain behavior belongs here.
use std::cell::RefCell;
use std::ops::Range;

use gpui::{AppContext, Bounds, Context, IntoElement, Render, Role, SharedString, UniformListScrollHandle, WeakEntity, Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, size, uniform_list};
use verdant_domain::{Command, CommandBatch, Vibrato};
use verdant_domain::model::{NoteModulationParameter, VibratoPreset};
use crate::editor::{Editor, LaneTool, MenuAction, Tool, WorkflowAction};
use crate::generated::controls::{
    self, CommandArg, CommandId, CommandTarget, NumericId, NumericPhase, NumericTarget, SelectId,
    SelectTarget,
};

fn modulation_parameter(id: NumericId) -> Option<NoteModulationParameter> {
    use NoteModulationParameter as P;
    use NumericId as N;
    match id {
        N::VibratoDelayPercent => Some(P::VibratoStart),
        N::VibratoDepthCents => Some(P::VibratoDepth),
        N::VibratoRateHz => Some(P::VibratoRate),
        N::VibratoPhaseDegrees => Some(P::VibratoPhase),
        N::PortamentoMs => Some(P::PortamentoLength),
        N::PortamentoCurvePercent => Some(P::PortamentoCurve),
        _ => None,
    }
}

fn modulation_batch(track: verdant_domain::TrackId, mut notes: Vec<verdant_domain::Note>, parameter: NoteModulationParameter, value: f64, label: &str) -> Result<CommandBatch, &'static str> {
    for note in &mut notes { note.set_modulation_value(parameter, value)?; }
    match parameter {
        NoteModulationParameter::PortamentoCurve => Ok(CommandBatch::single(label, Command::ReplaceNotes { track, notes })),
        NoteModulationParameter::PortamentoLength => Ok(CommandBatch::single(label, Command::SetPortamento {
            track, note_ids: notes.into_iter().map(|note| note.id).collect(), portamento_ms: Some(value.round() as u32),
        })),
        _ => Ok(CommandBatch { label: label.into(), commands: notes.into_iter().map(|note| Command::SetVibrato {
            track: track.clone(), note_ids: vec![note.id], vibrato: note.vibrato,
        }).collect() }),
    }
}

fn vibrato_preset_batch(track: verdant_domain::TrackId, notes: Vec<verdant_domain::Note>, preset: VibratoPreset) -> CommandBatch {
    let commands = notes.into_iter().map(|note| {
        let mut vibrato = note.vibrato.unwrap_or(Vibrato::DEFAULT);
        preset.apply(&mut vibrato);
        Command::SetVibrato { track: track.clone(), note_ids: vec![note.id], vibrato: Some(vibrato) }
    }).collect();
    CommandBatch { label: preset.label().into(), commands }
}

impl Editor {
    pub fn apply_vibrato_preset(&mut self, preset: VibratoPreset, cx: &mut Context<Self>) {
        let notes = self.selected_notes();
        if notes.is_empty() { return; }
        self.commit(self.revision(), vibrato_preset_batch(self.session.track.clone(), notes, preset), cx);
    }
}

gpui::actions!(verdant, [ShowHistory]);

thread_local! {
    static HISTORY_WINDOW: RefCell<Option<WindowHandle<HistoryPanel>>> = const { RefCell::new(None) };
}

struct HistoryPanel {
    editor: WeakEntity<Editor>,
    scroll: UniformListScrollHandle,
    selected: usize,
    displayed: Option<(String, verdant_domain::Revision)>,
}

impl Editor {
    /// The history window follows this editor's current project. Reopening it
    /// activates the existing window, like the shortcuts and credits windows.
    pub fn show_history_panel(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let editor = cx.weak_entity();
        // open_window draws immediately, and HistoryPanel reads its editor during that draw.
        cx.defer(move |cx| {
            let Some(owner) = editor.upgrade() else { return; };
            if let Some(handle) = HISTORY_WINDOW.with(|open| *open.borrow()) {
                if handle.update(cx, |_, window, _| window.activate_window()).is_ok() {
                    return;
                }
            }
            let bounds = Bounds::centered(None, size(px(440.0), px(560.0)), cx);
            match cx.open_window(
                WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default() },
                |window, cx| {
                    window.set_window_title("撤销历史 — Verdant");
                    cx.new(|cx| {
                        cx.observe(&owner, |_, _, cx| cx.notify()).detach();
                        HistoryPanel { editor, scroll: UniformListScrollHandle::new(), selected: 0, displayed: None }
                    })
                },
            ) {
                Ok(handle) => HISTORY_WINDOW.with(|open| *open.borrow_mut() = Some(handle)),
                Err(error) => owner.update(cx, |ed, cx| {
                    ed.notice(format!("无法打开撤销历史：{error}"));
                    cx.notify();
                }),
            }
        });
    }

    pub(crate) fn jump_history(&mut self, project_id: &str, base: verdant_domain::Revision, position: usize, cx: &mut Context<Self>) {
        if self.project().project_id != project_id || self.revision() != base {
            self.notice("历史记录已变化，请重新选择目标"); cx.notify(); return;
        }
        if self.lyric.is_some() {
            self.notice("请先完成歌词编辑，再跳转历史"); cx.notify(); return;
        }
        let current = self.store().undo_depth();
        if position > current + self.store().redo_depth() {
            self.notice("历史目标已不在当前分支中"); cx.notify(); return;
        }
        // Keep the existing native-plugin capture and complete editor/cache
        // lifecycle of Undo/Redo. Every traversed batch remains one step.
        while self.store().undo_depth() != position {
            let before = self.store().undo_depth();
            let revision = self.revision();
            let log_len = self.store().log().len();
            let backwards = position < before;
            if backwards { self.undo(cx); } else { self.redo(cx); }
            if self.revision() == revision { break; }
            let expected = if backwards { before - 1 } else { before + 1 };
            let kind = if backwards { verdant_domain::CommitKind::Undo } else { verdant_domain::CommitKind::Redo };
            if self.store().undo_depth() != expected || self.store().log().len() != log_len + 1
                || self.store().log().last().map(|commit| commit.kind) != Some(kind) {
                // A pending native-plugin edit can create a new branch during
                // capture. Do not reinterpret the stale target on that branch.
                self.notice("插件编辑已更新历史，请重新选择目标");
                break;
            }
        }
        cx.notify();
    }
}

impl HistoryPanel {
    fn rows(&self, range: Range<usize>, source: &verdant_collection_control::CollectionControlHandle,
        revision: &str, cx: &mut Context<Self>) -> Vec<gpui::Stateful<gpui::Div>> {
        let Some(owner) = self.editor.upgrade() else { return Vec::new(); };
        let editor = owner.read(cx);
        let history = editor.store().history();
        let th = editor.theme;
        let cursor = source.state("cursor").ok().as_ref().and_then(|value| crate::proto_controls::source_index(Some(value))).unwrap_or(self.selected);
        range.filter(|position| *position <= history.entries.len()).map(|position| {
            let label = if position == 0 { "初始状态".to_string() } else { history.entries[position - 1].label.to_string() };
            let current = position == history.position;
            let future = position > history.position;
            let click = source.clone(); let click_revision = revision.to_string();
            let accessible = source.clone(); let accessible_revision = revision.to_string();
            div().id(SharedString::from(format!("history-{position}"))).role(Role::ListBoxOption)
                .aria_label(format!("{position} · {label}{}", if current { " · 当前状态" } else if future { " · 可重做" } else { "" }))
                .aria_selected(current).aria_position_in_set(position + 1).aria_size_of_set(history.entries.len() + 1)
                .when(position == cursor, |row| row.aria_active_descendant())
                .h(px(34.0)).px(px(12.0)).flex().items_center().gap(px(8.0))
                .bg(if current { th.selection } else if position == cursor { th.control } else { th.surface })
                .text_color(if future { th.text_secondary } else { th.text }).hover(|row| row.bg(th.control_pressed))
                .child(div().w(px(24.0)).flex_none().text_xs().text_color(th.text_secondary).child(position.to_string()))
                .child(div().flex_1().truncate().text_sm().child(label))
                .child(div().text_xs().child(if current { "当前" } else if future { "重做" } else { "" }))
                .on_click(move |_, window, cx| {
                    crate::proto_collection::call(&click, "focusSelf", vec![], window, cx);
                    crate::proto_collection::call(&click, "choose", vec![position.into(), false.into(), click_revision.clone().into()], window, cx);
                    window.prevent_default(); cx.stop_propagation();
                })
                .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                    crate::proto_collection::call(&accessible, "focusSelf", vec![], window, cx);
                    crate::proto_collection::call(&accessible, "choose", vec![position.into(), false.into(), accessible_revision.clone().into()], window, cx);
                })
        }).collect()
    }
}

impl Render for HistoryPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(owner) = self.editor.upgrade() else {
            return div().p(px(16.0)).child("工程窗口已关闭").into_any_element();
        };
        let editor = owner.read(cx);
        let base = editor.revision();
        let project_id = editor.project().project_id.clone();
        let generation = editor.document_generation;
        let count = editor.store().undo_depth() + editor.store().redo_depth();
        let current = editor.store().undo_depth();
        let th = editor.theme;
        let font = editor.ui_font.clone();
        if self.displayed.as_ref().is_none_or(|(id, revision)| id != &project_id || *revision != base) {
            self.selected = current;
            self.displayed = Some((project_id.clone(), base));
        }
        let revision = format!("{project_id}:{generation}:{base}");
        let panel = cx.entity(); let scroll = self.scroll.clone(); let draw_revision = revision.clone();let selected=self.selected;
        let draw: crate::proto_collection::Draw = std::rc::Rc::new(move |source, focus, engine, window, cx| {
            crate::proto_collection::reveal_uniform_item("history-reveal",source,selected,&draw_revision,count+1,&scroll,window,cx);
            let source = source.clone(); let keyboard = source.clone(); let panel = panel.clone(); let revision = draw_revision.clone();
            let engine = engine.expect("history virtual list engine");
            div().id("history-list-content").track_focus(focus).size_full()
                .on_key_down(move |event, window, cx| crate::proto_collection::key(&keyboard, event, window, cx))
                .child(uniform_list("history-rows", count + 1, move |range, _, cx| {
                    panel.update(cx, |view, cx| view.rows(range, &source, &revision, cx))
                }).track_scroll(&scroll).size_full().on_scroll_wheel(move |_, window, cx| engine.notify_input(window, cx)))
                .into_any_element()
        });
        let panel = cx.entity().downgrade(); let basis = revision.clone();
        let sink: crate::proto_collection::Sink = std::rc::Rc::new(move |name, payload, window, cx| {
            if !matches!(name, "select" | "activate" | "close") || payload.get("revision").and_then(serde_json::Value::as_str) != Some(basis.as_str()) { return; }
            let _ = panel.update(cx, |panel, cx| {
                let Some(editor) = panel.editor.upgrade() else { return; };
                let ed = editor.read(cx);
                if ed.project().project_id != project_id || ed.document_generation != generation || ed.revision() != base { return; }
                if name == "close" { window.remove_window(); return; }
                let Some(position) = crate::proto_controls::source_index(payload.get("index")) else { return; };
                if position > count { return; }
                panel.selected = position;
                if name == "activate" { editor.update(cx, |ed, cx| ed.jump_history(&project_id, base, position, cx)); }
                cx.notify();
            });
        });
        let collection = crate::proto_collection::collection("history-list", serde_json::json!({
            "mode":"listbox", "label":"撤销历史", "description":"上下键选择历史状态，Home/End 到开头或末尾，Enter 或空格跳转。点击条目立即跳转。",
            "cursor":self.selected, "revision":revision, "autoFocus":true,
        }), vec![1;count + 1], draw, sink).scroll_engine(crate::proto_scroll::Engine::Uniform(self.scroll.clone()));
        div().id("undo-history").size_full().flex().flex_col().bg(th.surface).text_color(th.text).font_family(font)
            .child(div().p(px(12.0)).text_sm().child(format!("撤销历史 · {current}/{count}")))
            .child(div().flex_1().min_h_0().child(collection))
            .child(div().p(px(12.0)).border_t_1().border_color(th.shade).text_xs().text_color(th.text_secondary)
                .child("上下键选择，Home/End 到两端，Enter 或空格跳转。点击状态立即跳转；新编辑会清除后续历史。"))
            .into_any_element()
    }
}

fn toggle_phoneme_timing(current: Option<&verdant_domain::PhonemeTimingSettings>) -> Option<verdant_domain::PhonemeTimingSettings> {
    if current.is_some() { None } else { Some(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1()) }
}

impl CommandTarget for Editor {
    fn run_command(&mut self, command: CommandId, arg: CommandArg, window: &mut Window, cx: &mut Context<Self>) {
        use CommandArg as A;
        use CommandId as C;
        let action = match (command, arg) {
            (C::OpenProject, A::None) => MenuAction::OpenProject,
            (C::AuditionProjects, A::None) => { self.audition_picker(window, cx); return; }
            (C::TogglePhonemeTiming, A::None) => {
                let settings = toggle_phoneme_timing(self.project().render_preferences.phoneme_timing.as_ref());
                self.commit(self.revision(), CommandBatch::single("元音对拍", Command::SetPhonemeTiming { settings }), cx); return;
            }
            (C::ToggleOutputTransients, A::None) => {
                let settings = if self.project().render_preferences.output_transients.is_some() { None }
                    else { Some(verdant_domain::OutputTransientsSettings::phoneme_output_envelope_v1()) };
                self.commit(self.revision(), CommandBatch::single("最终辅音约束", Command::SetOutputTransients { settings }), cx); return;
            }
            (C::Save, A::None) => MenuAction::Save,
            (C::SaveAs, A::None) => MenuAction::SaveAs,
            (C::ExportProject, A::None) => MenuAction::ExportProject,
            (C::ExportAudio, A::None) => { crate::files::export_audio_dialog(self, window, cx); return; }
            (C::Undo, A::None) => MenuAction::Undo,
            (C::Redo, A::None) => MenuAction::Redo,
            (C::Cut, A::None) => MenuAction::Cut,
            (C::Copy, A::None) => MenuAction::Copy,
            (C::Paste, A::None) => MenuAction::Paste,
            (C::DuplicateSelection, A::None) => MenuAction::Duplicate,
            (C::DeleteSelection, A::None) => MenuAction::DeleteSelection,
            (C::SelectAll, A::None) => MenuAction::SelectAll,
            (C::EditLyric, A::Note(id)) => MenuAction::EditLyric(id),
            (C::TogglePlay, A::None) => { self.toggle_play(cx); return; }
            (C::Stop, A::None) => { self.stop(cx); return; }
            (C::GoToStart, A::None) => { self.seek_tick(0.0); cx.notify(); return; }
            (C::ToggleLoop, A::None) => { self.run_workflow(WorkflowAction::ToggleLoop, window, cx); return; }
            (C::ToggleMetronome, A::None) => { self.run_workflow(WorkflowAction::ToggleMetronome, window, cx); return; }
            (C::AddTrack, A::None) => MenuAction::AddTrack,
            (C::ImportAudioClip, A::None) => {
                self.run_workflow(WorkflowAction::Integration(crate::editor::integrations::IntegrationAction::ImportAudio), window, cx);
                return;
            }
            (C::DeleteActiveTrack, A::None) => MenuAction::DeleteTrack(self.session.track.clone()),
            (C::ToggleMute, A::Track(id)) => MenuAction::MuteTrack(id),
            (C::ToggleSolo, A::Track(id)) => MenuAction::SoloTrack(id),
            (C::ToggleTrackAutomation, A::Track(id)) => MenuAction::ToggleTrackAutomation(id),
            (C::ToggleMasterMute, A::None) => {
                self.edit_master_bus("主总线静音", cx, |m| m.muted = !m.muted);
                return;
            }
            (C::ToggleMasterAutomation, A::None) => MenuAction::ToggleMasterAutomation,
            (C::CycleColor, A::Track(id)) => MenuAction::CycleColor(id),
            (C::SetToolPointer, A::None) => MenuAction::SetTool(Tool::Pointer),
            (C::SetToolPencil, A::None) => MenuAction::SetTool(Tool::Pencil),
            (C::SetToolEraser, A::None) => MenuAction::SetTool(Tool::Eraser),
            (C::SetToolPitch, A::None) => MenuAction::SetTool(Tool::Pitch),
            (C::SetLanePencil, A::None) => { self.cancel_edits(verdant_session::CancelReason::Escape, window, cx); self.lane_tool = LaneTool::Pencil; cx.notify(); return; }
            (C::SetLaneLine, A::None) => { self.cancel_edits(verdant_session::CancelReason::Escape, window, cx); self.lane_tool = LaneTool::Line; cx.notify(); return; }
            // Freehand already smooths hand jitter; the old Smooth command selects it.
            (C::SetLaneSmooth, A::None) => { self.cancel_edits(verdant_session::CancelReason::Escape, window, cx); self.lane_tool = LaneTool::Pencil; cx.notify(); return; }
            (C::SetLaneEraser, A::None) => { self.cancel_edits(verdant_session::CancelReason::Escape, window, cx); self.lane_tool = LaneTool::Eraser; cx.notify(); return; }
            (C::SetLaneSelect, A::None) => { self.cancel_edits(verdant_session::CancelReason::Escape, window, cx); self.lane_tool = LaneTool::Select; cx.notify(); return; }
            (C::CopyCurve, A::None) => MenuAction::CopyCurve,
            (C::PasteCurve, A::None) => MenuAction::PasteCurveAt(self.tempo().tick_at(self.playhead_seconds()) as i64),
            (C::OpenTransformPoints, A::None) => { self.open_transform_points_dialog(window, cx); return; }
            (C::OpenScaleDialog, A::None) => { self.open_scale_dialog(window, cx); return; }
            (C::OpenVibratoDialog, A::None) => { self.open_vibrato_dialog(window, cx); return; }
            (C::ToggleLeftPanel, A::None) => { self.toggle_left_panel(cx); return; }
            (C::ToggleRightDock, A::None) => { self.toggle_right_dock(cx); return; }
            (C::TogglePhonemeLane, A::None) => MenuAction::TogglePhonemeLane,
            (C::ToggleTempoLane, A::None) => MenuAction::ToggleTempoLane,
            (C::ToggleOtherTracks, A::None) => { self.integrations.other_tracks = !self.integrations.other_tracks; cx.notify(); return; }
            (C::ToggleThreeLanes, A::None) => { self.add_lane(None, cx); return; }
            (C::ToggleF0Overlay, A::None) => { self.show_f0 = !self.show_f0; cx.notify(); return; }
            (C::ResetPhonemeDurations, A::Note(id)) => { self.reset_phoneme_durations(&id, cx); return; }
            (C::TogglePhonemeApproximation, A::Note(id)) => { self.toggle_phoneme_approximation(&id, cx); return; }
            (C::FitView, A::None) => { self.fit_view(); cx.notify(); return; }
            (C::FitArrange, A::None) => { self.fit_arrange(); cx.notify(); return; }
            (C::StartWorker, A::None) => { self.start_worker(cx); return; }
            (C::RenderNow, A::None) => { self.render_now(cx); return; }
            (C::PluginAdd, A::TrackSlot(id, slot)) => { self.plugin_picker(id, slot as usize, window.mouse_position(), window, cx); return; }
            (C::PluginBypass, A::TrackSlot(track, slot)) => MenuAction::BypassInsert { track, slot: slot as usize, expected_key: None },
            (C::PluginRemove, A::TrackSlot(track, slot)) => MenuAction::RemoveInsert { track, slot: slot as usize, expected_key: None },
            (C::PluginOpenEditor, A::TrackSlot(track, slot)) => MenuAction::OpenInsertEditor { track, slot: slot as usize, expected_key: None },
            (C::ChooseDevice, A::Text(id)) => MenuAction::ChooseDevice(id.to_string()),
            (C::ReopenDevice, A::None) => MenuAction::ReopenDevice,
            (C::OpenShortcuts, A::None) => MenuAction::ShowShortcutsWindow,
            (C::OpenDisclosures, A::None) => MenuAction::ShowDisclosuresWindow,
            _ => { self.notice("命令参数与控件声明不匹配"); cx.notify(); return; }
        };
        self.run_menu_action(action, window, cx);
        cx.notify();
    }
}

impl Editor {
    /// Only these commands represent an accepted Boolean value. Mode choices,
    /// playback requests and ordinary actions retain their command semantics.
    pub(crate) fn source_command_selection(&self, command: CommandId, arg: &CommandArg) -> Option<bool> {
        use CommandArg as A;
        use CommandId as C;
        match (command, arg) {
            (C::ToggleMute, A::Track(id)) => self.project().track(id).map(|track| track.muted),
            (C::ToggleSolo, A::Track(id)) => self.project().track(id).map(|track| track.solo),
            (C::ToggleTrackAutomation, A::Track(id)) => self.project().track(id).map(|_| self.automation_ui.expanded_tracks.contains(id)),
            (C::ToggleMasterMute, A::None) => Some(crate::mix::MasterBusSettings::from_project(self.project()).muted),
            (C::ToggleMasterAutomation, A::None) => Some(self.automation_ui.show_master),
            (C::TogglePhonemeApproximation, A::Note(id)) => self.active_track().and_then(|track| self.index.note(track, id))
                .filter(|note| note.pronunciation.alphabet.is_some()).map(|note| note.pronunciation.allow_approximation),
            (C::ToggleLoop, A::None) => Some(self.playback_settings().looping),
            (C::ToggleMetronome, A::None) => Some(self.playback_settings().metronome),
            (C::TogglePhonemeLane, A::None) => Some(self.layout.show_phoneme_lane),
            (C::ToggleTempoLane, A::None) => Some(self.layout.show_tempo_lane),
            (C::ToggleOtherTracks, A::None) => Some(self.integrations.other_tracks),
            (C::ToggleF0Overlay, A::None) => Some(self.show_f0),
            (C::TogglePhonemeTiming, A::None) => Some(self.project().render_preferences.phoneme_timing.is_some()),
            (C::ToggleOutputTransients, A::None) => Some(self.project().render_preferences.output_transients.is_some()),
            _ => None,
        }
    }

    pub(crate) fn accept_source_command_selection(&mut self, command: CommandId, arg: &CommandArg, selected: bool, cx: &mut Context<Self>) {
        use CommandArg as A;
        use CommandId as C;
        if self.source_command_selection(command, arg).is_none_or(|current| current == selected) { return; }
        match (command, arg) {
            (C::ToggleMute, A::Track(id)) => self.edit_header(id, "轨道静音", cx, |header| header.muted = selected),
            (C::ToggleSolo, A::Track(id)) => self.edit_header(id, "轨道独奏", cx, |header| header.solo = selected),
            (C::ToggleTrackAutomation, A::Track(id)) => {
                if selected { self.automation_ui.expanded_tracks.insert(id.clone()); }
                else { self.automation_ui.expanded_tracks.remove(id); }
            },
            (C::ToggleMasterMute, A::None) => self.edit_master_bus("主总线静音", cx, |settings| settings.muted = selected),
            (C::ToggleMasterAutomation, A::None) => self.automation_ui.show_master = selected,
            (C::TogglePhonemeApproximation, A::Note(id)) => self.set_phoneme_approximation(id, selected, cx),
            (C::ToggleLoop, A::None) => {
                let anchor = self.playback_settings().loop_start.max(0);
                let length = self.meter().bar_ticks_at(anchor);
                self.edit_playback("循环播放", cx, |settings| {
                    if settings.loop_end <= settings.loop_start { settings.loop_end = settings.loop_start + length; }
                    settings.looping = selected;
                });
            },
            (C::ToggleMetronome, A::None) => self.edit_playback("节拍器", cx, |settings| settings.metronome = selected),
            (C::TogglePhonemeLane | C::ToggleTempoLane, A::None) => {
                if command == C::TogglePhonemeLane { self.layout.show_phoneme_lane = selected; }
                else { self.layout.show_tempo_lane = selected; }
                self.layout.store(&mut self.prefs);
                self.prefs_dirty = true;
            },
            (C::ToggleOtherTracks, A::None) => self.integrations.other_tracks = selected,
            (C::ToggleF0Overlay, A::None) => self.show_f0 = selected,
            (C::TogglePhonemeTiming, A::None) => {
                let settings = selected.then(verdant_domain::PhonemeTimingSettings::vowel_anchor_v1);
                self.commit(self.revision(), CommandBatch::single("元音对拍", Command::SetPhonemeTiming { settings }), cx);
            },
            (C::ToggleOutputTransients, A::None) => {
                let settings = selected.then(verdant_domain::OutputTransientsSettings::phoneme_output_envelope_v1);
                self.commit(self.revision(), CommandBatch::single("最终辅音约束", Command::SetOutputTransients { settings }), cx);
            },
            _ => return,
        }
        cx.notify();
    }
}

impl NumericTarget for Editor {
    fn numeric_value(&self, id: NumericId, arg: CommandArg) -> f64 {
        use NumericId as N;
        if let Some((pid, parg, value)) = &self.numeric_preview {
            if *pid == id && *parg == arg { return *value; }
        }
        if let CommandArg::Track(track) = &arg {
            if let Some(t) = self.project().track(track) {
                return match id { N::TrackGainDb => t.gain_millidecibels as f64 / 1000.0, N::TrackPan => t.pan_permille as f64 / 10.0, _ => controls::numeric(id).default };
            }
        }
        if let CommandArg::Note(note) = &arg {
            if let Some(n) = self.active_track().and_then(|t| self.index.note(t, note)) {
                return match id {
                    N::NoteStartTick => n.start_tick as f64,
                    N::NoteDurationTicks => n.duration_tick as f64,
                    N::NotePitchSemitone => n.midi_note as f64,
                    N::NoteDetuneCents => n.detune_millicents as f64 / 1000.0,
                    N::PhonemeInitialPercent | N::PhonemeFinalPercent => self.phoneme_timing(n).map(|t| {
                        if id == N::PhonemeInitialPercent { if t.labels.len() == 2 { t.weights.first().copied().unwrap_or(0) as f64 / 10.0 } else { 0.0 } }
                        else { t.weights.last().copied().unwrap_or(1000) as f64 / 10.0 }
                    }).unwrap_or(0.0),
                    _ => controls::numeric(id).default,
                };
            }
        }
        if let CommandArg::NotePhone(note, index) = &arg {
            if id == N::PhonemePercent {
                return self.active_track().and_then(|track| self.index.note(track, note))
                    .and_then(|note| self.phoneme_timing(note))
                    .and_then(|timing| timing.weights.get(*index as usize).copied())
                    .map(|weight| weight as f64 / 10.0).unwrap_or(0.0);
            }
        }
        if let Some(parameter) = modulation_parameter(id) {
            return self.session.selection_in_order(self.project()).first()
                .and_then(|id| self.active_track().and_then(|track| self.index.note(track, id)))
                .map_or(controls::numeric(id).default, |note| note.modulation_value(parameter));
        }
        match id {
            N::MasterGainDb => self.master_bus().gain_millidecibels as f64 / 1000.0,
            N::MasterPan => self.master_bus().pan_permille as f64 / 10.0,
            N::TempoBpm => self.tempo().bpm_at(self.tempo_event_tick()),
            N::SnapGridTicks => self.session.snap.grid_ticks as f64,
            N::DefaultNoteTicks => self.session.default_note_ticks as f64,
            _ => controls::numeric(id).default,
        }
    }

    fn set_numeric(&mut self, id: NumericId, arg: CommandArg, value: f64, phase: NumericPhase, cx: &mut Context<Self>) {
        use NumericId as N;
        let spec = controls::numeric(id);
        if !value.is_finite() || spec.arg != arg.kind() {
            if modulation_parameter(id).is_some() { self.numeric_preview = None; }
            self.notice("数值或控件参数无效"); cx.notify(); return;
        }
        if modulation_parameter(id).is_some() && !(spec.min..=spec.max).contains(&value) {
            self.numeric_preview = None;
            self.notice("颤音或滑音数值超出允许范围"); cx.notify(); return;
        }
        let value = value.clamp(spec.min, spec.max);
        if phase == NumericPhase::Preview {
            self.numeric_preview = Some((id, arg, value));
            self.sync_mixer();
            cx.notify(); return;
        }
        self.numeric_preview = None;
        let command = match (id, arg) {
            (N::TrackGainDb, CommandArg::Track(track)) => {
                self.edit_header(&track, "轨道增益", cx, |h| h.gain_millidecibels = (value * 1000.0).round() as i32); return;
            }
            (N::TrackPan, CommandArg::Track(track)) => {
                self.edit_header(&track, "轨道声像", cx, |h| h.pan_permille = (value * 10.0).round() as i32); return;
            }
            (N::MasterGainDb, CommandArg::None) => {
                self.edit_master_bus("主总线增益", cx, |m| m.gain_millidecibels = (value * 1000.0).round() as i32); return;
            }
            (N::MasterPan, CommandArg::None) => {
                self.edit_master_bus("主总线声像", cx, |m| m.pan_permille = (value * 10.0).round() as i32); return;
            }
            (N::TempoBpm, CommandArg::None) => Command::SetTempoEvent { tick: self.tempo_event_tick(), micro_bpm: Some((value * 1e6).round() as u64) },
            (N::SnapGridTicks, CommandArg::None) => { self.session.snap.grid_ticks = value.round() as i64; cx.notify(); return; }
            (N::DefaultNoteTicks, CommandArg::None) => { self.session.default_note_ticks = value.round() as i64; cx.notify(); return; }
            (N::PhonemeInitialPercent | N::PhonemeFinalPercent, CommandArg::Note(note)) => {
                self.set_phoneme_percent(&note, id == N::PhonemeInitialPercent, value, cx); return;
            }
            (N::PhonemePercent, CommandArg::NotePhone(note, index)) => {
                self.set_phone_percent(&note, index as usize, value, cx); return;
            }
            (N::NoteStartTick | N::NoteDurationTicks | N::NotePitchSemitone | N::NoteDetuneCents, CommandArg::Note(note)) => {
                let Some(anchor)=self.active_track().and_then(|t|self.index.note(t,&note)).cloned()else{return;};
                let selected=if self.session.selection.contains(&note){self.selected_notes()}else{vec![anchor.clone()]};
                let(dt,ds)=verdant_domain::command::clamp_group_move(selected.iter(),if id==N::NoteStartTick{value.round()as i64-anchor.start_tick}else{0},if id==N::NotePitchSemitone{value.round()as i32-anchor.midi_note}else{0});
                let commands=selected.into_iter().map(|n|match id{
                    N::NoteStartTick=>Command::SetTiming{track:self.session.track.clone(),note_id:n.id,start_tick:n.start_tick+dt,duration_tick:n.duration_tick},
                    N::NoteDurationTicks=>Command::SetTiming{track:self.session.track.clone(),note_id:n.id,start_tick:n.start_tick,duration_tick:value.round()as i64},
                    N::NotePitchSemitone=>Command::SetPitch{track:self.session.track.clone(),note_id:n.id,midi_note:n.midi_note+ds,detune_millicents:n.detune_millicents},
                    _=>Command::SetPitch{track:self.session.track.clone(),note_id:n.id,midi_note:n.midi_note,detune_millicents:(value*1000.).round()as i32},
                }).collect();
                self.commit(self.revision(),CommandBatch{label:spec.label.into(),commands},cx);return;
            }
            (id, CommandArg::None) if modulation_parameter(id).is_some() => {
                let notes = self.selected_notes();
                if notes.is_empty() { return; }
                match modulation_batch(self.session.track.clone(), notes, modulation_parameter(id).unwrap(), value, spec.label) {
                    Ok(batch) => { self.commit(self.revision(), batch, cx); }
                    Err(message) => { self.notice(message); cx.notify(); }
                }
                return;
            }
            _ => { self.notice("控件参数与数值声明不匹配"); cx.notify(); return; }
        };
        self.commit(self.revision(), CommandBatch::single(spec.label, command), cx);
        self.sync_mixer();
        cx.notify();
    }
}

impl Editor {
    pub fn current_select_option(&self, id: SelectId, arg: &CommandArg) -> &'static str {
        match (id, arg) {
            (SelectId::AppearanceMode, CommandArg::None) => match self.appearance_override {
                Some(true) => "dark",
                Some(false) => "light",
                None => match crate::theme::AppearancePreference::from_prefs(&self.prefs) {
                    crate::theme::AppearancePreference::System => "system",
                    crate::theme::AppearancePreference::Light => "light",
                    crate::theme::AppearancePreference::Dark => "dark",
                },
            },
            (SelectId::CountInBars, CommandArg::None) => match self.playback_settings().count_in_bars {
                1 => "one",
                2 => "two",
                4 => "four",
                _ => "off",
            },
            (SelectId::PitchRangeCents, CommandArg::None) => {
                if (self.pit_range_cents - 100.0).abs() < 1e-6 {
                    "c100"
                } else if (self.pit_range_cents - 1200.0).abs() < 1e-6 {
                    "c1200"
                } else {
                    "c200"
                }
            }
            _ => controls::select(id).default_option,
        }
    }
}

impl SelectTarget for Editor {
    fn open_select(
        &mut self,
        id: SelectId,
        arg: CommandArg,
        anchor: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let spec = controls::select(id);
        let current = self.current_select_option(id, &arg);
        let items = spec
            .options
            .iter()
            .map(|opt| {
                let selected = opt.id == current;
                let mut item = crate::menu::MenuItem::action(
                    opt.label,
                    MenuAction::GeneratedSelect { id, arg: arg.clone(), option: opt.id },
                ).checked(selected);
                if let Some(hint) = opt.hint {
                    item = item.with_hint(hint);
                }
                item
            })
            .collect();
        self.show_menu(anchor, items, window, cx);
    }

    fn set_select(
        &mut self,
        id: SelectId,
        arg: CommandArg,
        option_id: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match (id, arg, option_id) {
            (SelectId::AppearanceMode, CommandArg::None, opt) => {
                let pref = match opt {
                    "light" => crate::theme::AppearancePreference::Light,
                    "dark" => crate::theme::AppearancePreference::Dark,
                    _ => crate::theme::AppearancePreference::System,
                };
                self.set_appearance_preference(pref, window, cx);
            }
            (SelectId::CountInBars, CommandArg::None, opt) => {
                let bars = match opt {
                    "one" => 1,
                    "two" => 2,
                    "four" => 4,
                    _ => 0,
                };
                self.run_workflow(WorkflowAction::CountIn(bars), window, cx);
            }
            (SelectId::PitchRangeCents, CommandArg::None, opt) => {
                self.pit_range_cents = match opt {
                    "c100" => 100.0,
                    "c1200" => 1200.0,
                    _ => 200.0,
                };
                cx.notify();
            }
            _ => {
                self.notice("选择框参数不匹配");
                cx.notify();
            }
        }
    }
}

#[cfg(test)]
#[path = "vibrato_portamento_tests.rs"]
mod vibrato_portamento_tests;

#[cfg(test)]
mod timing_control_tests {
    use super::toggle_phoneme_timing;
    use verdant_domain::{Command, CommandBatch, ModelDynamicsSettings, PhonemeTimingSettings, Project, ProjectStore, VocalTractSettings};

    #[test]
    fn timing_button_keeps_notes_colour_dynamics_and_seed_and_roundtrips_undo() {
        let mut project = Project::new_empty();
        project.render_preferences.vocal_tract = Some(VocalTractSettings::bilinear_v1(1.125));
        project.render_preferences.model_dynamics = Some(ModelDynamicsSettings::voiced_envelope_v1());
        let original = project.clone();
        let mut store = ProjectStore::new(project);
        let settings = toggle_phoneme_timing(store.project().render_preferences.phoneme_timing.as_ref());
        assert_eq!(settings, Some(PhonemeTimingSettings::vowel_anchor_v1()));
        store.apply(store.revision(), CommandBatch::single("元音对拍", Command::SetPhonemeTiming { settings })).unwrap().unwrap();
        let mut actual = store.project().clone();
        actual.render_preferences.phoneme_timing = None;
        assert_eq!(actual, original);
        assert!(toggle_phoneme_timing(store.project().render_preferences.phoneme_timing.as_ref()).is_none());
        store.undo().unwrap();
        assert_eq!(store.project(), &original);
        store.redo().unwrap();
        assert_eq!(store.project().render_preferences.phoneme_timing, Some(PhonemeTimingSettings::vowel_anchor_v1()));
    }

}

#[cfg(test)]
mod history_native_tests {
    use super::*;
    use gpui::{Entity, TestAppContext};
    use std::rc::Rc;
    use verdant_collection_control::native::gpui_native_owner::{all_owners, resolved};

    fn transport_diagnostics(ed: &mut Editor) -> String {
        format!("revision={}, loading={}, callback_budget={}, retired_chain={}, applied_timeline={}, notices={:?}",
            ed.revision().get(), ed.inserts_loading, ed.audio.callback_budget(),
            ed.audio.retired_chain_revision(), ed.audio.applied_timeline_revision(),
            ed.notices.iter().map(|notice| notice.text.as_str()).collect::<Vec<_>>())
    }

    fn wait_for_transport_ready(window: gpui::WindowHandle<HistoryPanel>, cx: &mut TestAppContext) {
        // SilentClock uses a real callback thread. Rendering additional frames
        // alone cannot acknowledge the exact installed chain/timeline receipts.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            crate::native_test_support::frames(window, cx);
            let (ready, diagnostics) = window.update(cx, |panel, _, cx| {
                panel.editor.upgrade().expect("history editor stays alive").update(cx, |ed, cx| {
                    let ready = ed.native_insert_transport_ready(cx);
                    (ready, transport_diagnostics(ed))
                })
            }).unwrap();
            if ready { return; }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            assert!(!remaining.is_zero(), "history fixture transport did not become ready: {diagnostics}");
            // Yield only between checks of the actual production readiness
            // predicate; neither a fixed frame count nor clearing its guard.
            std::thread::park_timeout(remaining.min(std::time::Duration::from_millis(1)));
        }
    }

    #[gpui::test]
    fn history_window_uses_compiled_selection_before_domain_jump(cx: &mut TestAppContext) {
        let kept = Rc::new(RefCell::new(None::<Entity<Editor>>));
        let retain = kept.clone();
        let window = cx.open_window(size(px(440.0), px(560.0)), move |window, cx| {
            let editor = cx.new(|cx| Editor::for_native_test(verdant_domain::Project::new_empty(), window, cx));
            editor.update(cx, |ed, cx| {
                for name in ["First edit", "Second edit"] {
                    let track = ed.project().tracks[0].id.clone();
                    let mut header = ed.project().tracks[0].header(); header.name = name.into();
                    assert!(ed.commit(ed.revision(), CommandBatch::single(name, Command::SetTrackHeader { track, header }), cx));
                }
            });
            *retain.borrow_mut() = Some(editor.clone());
            HistoryPanel { editor:editor.downgrade(), scroll:UniformListScrollHandle::new(), selected:0, displayed:None }
        });
        window.update(cx, |_, window, _| window.activate_window()).unwrap();
        wait_for_transport_ready(window, cx);
        cx.simulate_keystrokes(window.into(), "up");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |panel, window, cx| {
            let source = all_owners().into_iter().find(|owner| {
                let owner = owner.borrow();
                owner.mounted && owner.window_id == Some(window.window_handle().window_id())
                    && owner.props.get("label").and_then(serde_json::Value::as_str) == Some("撤销历史")
            }).expect("mounted compiled history collection");
            let owner = source.borrow();
            assert_eq!(owner.modules.accessible_text.get("description").map(|value| resolved(value).text()).as_deref(),
                Some("上下键选择历史状态，Home/End 到开头或末尾，Enter 或空格跳转。点击条目立即跳转。"), "the compiled collection owns its accessible keyboard guidance");
            assert_eq!(panel.selected, 1, "one native key traverses exactly one authored row");
            assert_eq!(panel.editor.upgrade().unwrap().read(cx).store().undo_depth(), 2);
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "home");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |panel, _, cx| {
            assert_eq!(panel.selected, 0);
            assert_eq!(panel.editor.upgrade().unwrap().read(cx).store().undo_depth(), 2, "selection alone cannot undo");
        }).unwrap();
        cx.simulate_keystrokes(window.into(), "enter");
        crate::native_test_support::frames(window, cx);
        window.update(cx, |panel, _, cx| {
            let (depth, diagnostics) = panel.editor.upgrade().unwrap().update(cx, |ed, _| {
                (ed.store().undo_depth(), transport_diagnostics(ed))
            });
            assert_eq!(depth, 0, "source activation executes the guarded history transaction: {diagnostics}");
        }).unwrap();
    }
}
