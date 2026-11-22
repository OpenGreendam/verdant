//! Application implementations of declaration-generated command and numeric
//! targets. Source declarations define controls; domain behavior belongs here.
use std::cell::RefCell;
use std::ops::Range;

use gpui::{AppContext, Bounds, Context, FocusHandle, IntoElement, Render, Role, SharedString, UniformListScrollHandle, WeakEntity, Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, size, uniform_list};
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
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    native_scroll: crate::proto_scroll::NativeScroll,
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
                        let focus = cx.focus_handle();
                        window.focus(&focus, cx);
                        let scroll = UniformListScrollHandle::new();
                        let native_scroll = crate::proto_scroll::NativeScroll::new(scroll.0.borrow().base_handle.clone());
                        HistoryPanel { editor, focus, scroll, native_scroll, selected: 0, displayed: None }
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
    fn rows(&self, range: Range<usize>, collection: &verdant_collection_control::CollectionControlHandle,
        revision: &str, cx: &mut Context<Self>) -> Vec<crate::proto_controls::Action> {
        let Some(owner) = self.editor.upgrade() else { return Vec::new(); };
        let editor = owner.read(cx);
        let history = editor.store().history();
        let th = editor.theme;
        range.filter_map(|position| {
            let label = if position == 0 { "初始状态".to_string() } else { history.entries.get(position - 1)?.label.to_string() };
            let current = position == history.position;
            let future = position > history.position;
            let collection = collection.clone(); let revision = revision.to_owned();
            Some(crate::proto_controls::action(SharedString::from(format!("history-{position}")),
                format!("{position} · {label}{}", if current { " · 当前状态" } else if future { " · 可重做" } else { "" })).role(Role::ListBoxOption).tab_index(-1)
                .aria_selected(current).aria_position_in_set(position + 1).aria_size_of_set(history.entries.len() + 1)
                .when(position == self.selected, |row| row.aria_active_descendant())
                .h(px(34.0)).px(px(12.0)).flex().items_center().gap(px(8.0))
                .bg(if current { th.selection } else if position == self.selected { th.control } else { th.surface })
                .text_color(if future { th.text_secondary } else { th.text }).hover(|row| row.bg(th.control_pressed))
                .child(div().w(px(24.0)).flex_none().text_xs().text_color(th.text_secondary).child(position.to_string()))
                .child(div().flex_1().truncate().text_sm().child(label))
                .child(div().text_xs().child(if current { "当前" } else if future { "重做" } else { "" }))
                .on_click(move |_, window, cx| crate::proto_collection::call(&collection, "choose",
                    vec![position.into(), false.into(), revision.clone().into()], window, cx)))
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
        let count = editor.store().undo_depth() + editor.store().redo_depth();
        let current = editor.store().undo_depth();
        let th = editor.theme;
        let font = editor.ui_font.clone();
        if self.displayed.as_ref().is_none_or(|(id, revision)| id != &project_id || *revision != base) {
            self.selected = current;
            self.scroll.scroll_to_item(current, gpui::ScrollStrategy::Nearest);
            self.displayed = Some((project_id.clone(), base));
        }
        let panel = cx.entity();
        let revision = format!("{}:{}:{}", editor.document_generation, project_id, base.get());
        let props = serde_json::json!({"mode":"listbox", "label":"撤销历史", "cursor":self.selected,
            "revision":revision, "autoFocus":true});
        let draw_revision = revision.clone();
        let draw: crate::proto_collection::Draw = std::rc::Rc::new(move |collection, focus, window, cx| {
            panel.update(cx, |view, cx| {
                let was_focused = view.focus.is_focused(window); view.focus = focus.clone();
                if was_focused { window.focus(focus, cx); }
                let panel = cx.entity(); let collection = collection.clone(); let revision = draw_revision.clone();
                let scroll = view.scroll.clone();
                crate::proto_scroll::content("compiled-history-scroll", "撤销历史列表", move |_, _| {
                    let panel = panel.clone(); let collection = collection.clone(); let revision = revision.clone();
                    uniform_list("history-rows", count + 1, move |range, _, cx| {
                        panel.update(cx, |view, cx| view.rows(range, &collection, &revision, cx))
                    }).track_scroll(&scroll).size_full().into_any_element()
                }).tab_stop(false).native_scroll(view.native_scroll.clone()).into_any_element()
            })
        });
        let panel = cx.entity().downgrade();
        let sink: crate::proto_collection::Sink = std::rc::Rc::new(move |name, payload, window, cx| {
            if payload.get("revision").and_then(serde_json::Value::as_str) != Some(revision.as_str()) { return; }
            let _ = panel.update(cx, |view, cx| {
                let Some(editor) = view.editor.upgrade() else { return; };
                let ed = editor.read(cx);
                if ed.project().project_id != project_id || ed.revision() != base { return; }
                if name == "close" { window.remove_window(); return; }
                if name != "select" && name != "activate" { return; }
                let Some(position) = crate::proto_controls::source_index(payload.get("index")).filter(|position| *position <= count) else { return; };
                view.selected = position;
                view.scroll.scroll_to_item(position, gpui::ScrollStrategy::Nearest);
                window.focus(&view.focus, cx);
                if name == "activate" { editor.update(cx, |editor, cx| editor.jump_history(&project_id, base, position, cx)); }
                cx.notify();
            });
        });
        div().id("undo-history").size_full().flex().flex_col().bg(th.surface).text_color(th.text).font_family(font)
            .child(div().p(px(12.0)).text_sm().child(format!("撤销历史 · {current}/{count}")))
            .child(div().id("history-list")
                .aria_description("上下键选择历史状态，Home/End 到开头或末尾，Enter 或空格跳转。点击条目立即跳转。")
                .flex_1().min_h_0()
                .child(crate::proto_collection::collection("compiled-history-list", props, vec![1; count + 1], draw, sink)))
            .child(div().p(px(12.0)).border_t_1().border_color(th.shade).text_xs().text_color(th.text_secondary)
                .child("点击状态跳转；撤销后进行新编辑会清除后续历史。"))
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
