//! Structured graphical modal dialogs and inline audio-clip inspector controls,
//! with exact numeric prompts alongside the visual controls.

use gpui::{
    Context, Div, Entity, FontWeight, HitboxBehavior, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, DispatchPhase, Global, Role,
    SharedString, Window, canvas, div, fill, prelude::*, px,
};
use verdant_domain::model::{NoteModulationParameter, VibratoPreset};
use verdant_domain::{Command, CommandBatch, Revision, TrackId, Vibrato};

use crate::editor::Editor;
use crate::paint::b;
use crate::theme::{Theme, with_alpha};
use crate::ui_controls::{format_gain_db, format_pan_percent, selectable_button};

pub const NOTE_ROOT_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
pub const DEGREE_LABELS: [&str; 12] = ["1", "♭2", "2", "♭3", "3", "4", "♯4", "5", "♭6", "6", "♭7", "7"];

pub const SCALE_PRESETS: &[(&str, &[u8])] = &[
    ("自然大调 Major", &[0, 2, 4, 5, 7, 9, 11]),
    ("自然小调 Minor", &[0, 2, 3, 5, 7, 8, 10]),
    ("和声小调", &[0, 2, 3, 5, 7, 8, 11]),
    ("旋律小调", &[0, 2, 3, 5, 7, 9, 11]),
    ("大调五声", &[0, 2, 4, 7, 9]),
    ("小调五声", &[0, 3, 5, 7, 10]),
    ("布鲁斯 Blues", &[0, 3, 5, 6, 7, 10]),
    ("都节音阶 (日系)", &[0, 1, 5, 7, 8]),
];

#[derive(Clone, Debug)]
pub enum ModalDialog {
    Scale {
        root: u8,
        intervals: Vec<u8>,
    },
    Segment {
        editing_key: Option<String>,
        track_index: usize,
        start_tick: i64,
        end_tick: i64,
        selected_id: String,
        base: Revision,
    },
    VibratoShape {
        depth_cents: f64,
        rate_hz: f64,
        delay_percent: f64,
        fade_in_percent: f64,
        fade_out_percent: f64,
        phase_deg: f64,
        portamento_curve: f64,
        base: Revision,
    },
    TransformPoints {
        track: TrackId,
        offset_ticks: i64,
        time_scale_percent: f64,
        value_offset: f64,
        value_scale_percent: f64,
        base: Revision,
    },
    AudioClipEdit {
        track: TrackId,
        clip_id: String,
        start_tick: i64,
        offset_frames: u64,
        duration_frames: u64,
        sample_rate: u32,
        gain_db: f64,
        gain_changed: bool,
        pan_percent: f64,
        pan_changed: bool,
        muted: bool,
        base: Revision,
    },
}

/// Horizontal slider with a compiled source transaction and an Adwaita slot.
/// Draft dialogs echo preview values; project/plugin owners may accept only the
/// commit. Both use the same keyboard, assistive and cancellation path.
pub fn horizontal_slider(editor: Entity<Editor>, id: impl Into<SharedString>,
    value: f64, min: f64, max: f64, default_val: Option<f64>, th: Theme,
    on_change: impl Fn(&mut Editor, f64, bool, &mut Context<Editor>) + Clone + 'static) -> impl IntoElement {
    horizontal_slider_with_precision(editor, id, value, min, max, default_val, SliderPrecision::Continuous, th, on_change)
}

/// Precision is part of each host value's representation, independent of its
/// element ID. Match both pointer candidates to the value the draft will echo.
#[derive(Clone, Copy)]
enum SliderPrecision { Continuous, Whole, Half, Twentieth }
impl SliderPrecision {
    fn canonical(self, value: f64) -> f64 {
        match self {
            Self::Continuous => value,
            Self::Whole => value.round(),
            Self::Half => (value * 2.0).round() / 2.0,
            Self::Twentieth => (value * 20.0).round() / 20.0,
        }
    }
    fn step(self, value: f64) -> f64 {
        let quantum = match self {
            Self::Continuous => return value,
            Self::Whole => 1.0,
            Self::Half => 0.5,
            Self::Twentieth => 0.05,
        };
        self.canonical(value).max(quantum)
    }
}

fn horizontal_slider_with_precision(editor: Entity<Editor>, id: impl Into<SharedString>,
    value: f64, min: f64, max: f64, default_val: Option<f64>, precision: SliderPrecision, th: Theme,
    on_change: impl Fn(&mut Editor, f64, bool, &mut Context<Editor>) + Clone + 'static) -> impl IntoElement {
    use crate::proto_scalar::{EventKind, Sample, Spec, scalar};
    let id: SharedString = id.into();
    let label = if id.contains("gain") { "增益" } else if id.contains("pan") { "声像" }
        else if id.contains("time-scale") { "时间缩放" } else if id.contains("offset") { "偏移" }
        else if id.contains("scale") { "数值缩放" } else { "参数值" };
    let mut spec = Spec::slider(value, min, max, label);
    // The source still owns key and assistive actions. Give it steps the
    // draft can represent, including a useful minimum Shift increment.
    spec.step = precision.step(spec.step);
    spec.fine_step = precision.step(spec.fine_step);
    spec.default_value = default_val.unwrap_or(value);
    spec.double_action = if default_val.is_some() { "reset" } else { "none" };
    spec.preview_echo = true;
    let control = scalar(editor, id.clone(), spec,
        move |sample, bounds, initial| {
            let width = (f64::from(bounds.size.width) - 12.0).max(1.0);
            let candidate = min + (sample.position.x - 6.0) / width * (max-min);
            Sample {
                value: precision.canonical(candidate),
                fine: precision.canonical(initial + (candidate-initial)*0.1),
                jump: true,
            }
        },
        move |display, _, _, _| canvas(|_, _, _| {}, move |bounds, _, window, _| {
            let (w, h) = (f64::from(bounds.size.width), f64::from(bounds.size.height));
            let pad = 6.0;
            let track_w = (w - 2.0*pad).max(1.0);
            let fraction = if max > min { ((display.value-min)/(max-min)).clamp(0.0, 1.0) } else { 0.0 };
            let thumb_x = pad + fraction*track_w;
            let o = bounds.origin;
            window.paint_quad(gpui::quad(b(pad, h*0.5-2.5, track_w, 5.0, o), px(2.5), th.lcd, px(1.0), th.separator, gpui::BorderStyle::Solid));
            if min < 0.0 && max > 0.0 {
                let zero = pad + ((-min)/(max-min)).clamp(0.0, 1.0)*track_w;
                let (a, z) = if thumb_x < zero { (thumb_x, zero) } else { (zero, thumb_x) };
                window.paint_quad(fill(b(a, h*0.5-2.0, (z-a).max(1.0), 4.0, o), th.accent));
                window.paint_quad(fill(b(zero-0.5, h*0.5-5.0, 1.0, 10.0, o), th.text_tertiary));
            } else { window.paint_quad(fill(b(pad, h*0.5-2.0, (thumb_x-pad).max(0.0), 4.0, o), th.accent)); }
            window.paint_quad(gpui::quad(b(thumb_x-5.5, h*0.5-5.5, 11.0, 11.0, o), px(5.5),
                if display.active { th.accent } else { th.text }, px(if display.focus_visible { 2.0 } else { 1.0 }), th.accent, gpui::BorderStyle::Solid));
        }).size_full().into_any_element(),
        move |ed, event, _, cx| match event.kind {
            EventKind::Preview | EventKind::Cancel => on_change(ed, event.value, false, cx),
            EventKind::Commit => on_change(ed, event.value, true, cx),
            _ => {},
        });
    div().id(id).h(px(20.0)).w_full().child(control)
}

impl Editor {
    pub fn open_scale_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.end_lyric(true, None, window, cx);
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        self.modal_dialog = Some(ModalDialog::Scale {
            root: self.integrations.scale_root,
            intervals: self.integrations.scale_intervals.clone(),
        });
        cx.notify();
    }

    pub fn open_segment_dialog(&mut self, editing_key: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        self.end_lyric(true, None, window, cx);
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        let settings = verdant_backend::phonemizer::settings_from_project(self.project())?;
        let ppq = self.project().ppq.max(1);
        let (track_index, start_tick, end_tick, selected_id) = if let Some(key) = &editing_key {
            let entry = settings.segments.get(key).ok_or("分段映射已不存在")?;
            let idx = self.track_index(&entry.track).unwrap_or(0);
            (idx, entry.start_tick, entry.end_tick, entry.selected.clone())
        } else {
            let idx = self.track_index(&self.session.track).unwrap_or(0);
            let notes = self.selected_notes();
            let (s, e) = if let (Some(first), Some(last)) = (notes.first(), notes.last()) {
                (first.start_tick, last.end_tick())
            } else {
                let cur = self.session.snap.snap(self.tempo().tick_at(self.playhead_seconds())).max(0);
                (cur, cur + ppq * 16)
            };
            (idx, s, e.max(s + ppq), "mandarin-dictionary-v1".into())
        };
        self.modal_dialog = Some(ModalDialog::Segment {
            editing_key,
            track_index,
            start_tick,
            end_tick,
            selected_id,
            base: self.revision(),
        });
        cx.notify();
        Ok(())
    }

    pub fn open_vibrato_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let notes = self.selected_notes();
        let Some(first) = notes.first() else {
            self.notice("请先在钢琴卷帘中选择至少一个音符");
            cx.notify();
            return;
        };
        self.end_lyric(true, None, window, cx);
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        let v = first.vibrato.clone().unwrap_or(Vibrato::DEFAULT);
        self.modal_dialog = Some(ModalDialog::VibratoShape {
            depth_cents: v.depth_cents as f64,
            rate_hz: v.rate_millihertz as f64 / 1000.0,
            delay_percent: v.delay_permille as f64 / 10.0,
            fade_in_percent: v.fade_in_permille() as f64 / 10.0,
            fade_out_percent: v.fade_out_permille() as f64 / 10.0,
            phase_deg: v.phase_millidegrees() as f64 / 1000.0,
            portamento_curve: first.portamento_curve_percent() as f64,
            base: self.revision(),
        });
        cx.notify();
    }

    pub fn open_transform_points_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.point_selection.is_empty() {
            self.notice("请先在参数泳道框选控制点（或点击菜单选择控制点）");
            cx.notify();
            return;
        }
        self.end_lyric(true, None, window, cx);
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        self.modal_dialog = Some(ModalDialog::TransformPoints {
            track: self.session.track.clone(),
            offset_ticks: 0,
            time_scale_percent: 100.0,
            value_offset: 0.0,
            value_scale_percent: 100.0,
            base: self.revision(),
        });
        cx.notify();
    }

    pub fn open_audio_clip_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let clip = self.selected_audio_clip()?;
        self.end_lyric(true, None, window, cx);
        self.cancel_edits(verdant_session::CancelReason::Escape, window, cx);
        let rate = clip.source.sample_rate.max(1);
        let gain_db = if clip.gain <= 1e-5 { -96.0 } else { (20.0 * (clip.gain as f64).log10()).clamp(-96.0, 12.0) };
        self.modal_dialog = Some(ModalDialog::AudioClipEdit {
            track: clip.track,
            clip_id: clip.id.to_string(),
            start_tick: clip.start_tick,
            offset_frames: clip.source_offset_frames,
            duration_frames: clip.duration_frames,
            sample_rate: rate,
            gain_db,
            gain_changed: false,
            pan_percent: (clip.pan as f64 * 100.0).clamp(-100.0, 100.0),
            pan_changed: false,
            muted: clip.muted,
            base: self.revision(),
        });
        cx.notify();
        Ok(())
    }

    fn prompt_exact_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        use crate::editor::{PromptPurpose, integrations::IntegrationPrompt};
        let dialog = self.modal_dialog.as_ref().ok_or("编辑窗口已关闭")?;
        let (title, purpose, text, base, track) = match dialog {
            ModalDialog::TransformPoints { track, offset_ticks, time_scale_percent, value_offset, value_scale_percent, base } => (
                "精确变换：平移 tick 整数, 时间缩放 %, 数值偏移, 数值缩放 %（不限滑块范围）",
                PromptPurpose::Points(track.clone()),
                format!("{offset_ticks}, {time_scale_percent}, {value_offset}, {value_scale_percent}"),
                *base,
                track.clone(),
            ),
            ModalDialog::AudioClipEdit { track, clip_id, start_tick, offset_frames, duration_frames, gain_db, gain_changed, pan_percent, pan_changed, muted, base, .. } => {
                let clips = verdant_media::read_clips(self.project().track(track).ok_or("轨道不存在")?).map_err(|e| e.to_string())?;
                let clip = clips.iter().find(|c| c.id.to_string() == *clip_id).ok_or("音频片段不存在")?;
                let gain = if *gain_changed { clip_dialog_gain(*gain_db) } else { clip.gain };
                let pan = if *pan_changed { (*pan_percent as f32 / 100.0).clamp(-1.0, 1.0) } else { clip.pan };
                (
                    "精确片段：起点 tick, 源起点帧, 时长帧, 线性增益, 声像 (-1..1), 静音 (0/1)",
                    PromptPurpose::Integration(IntegrationPrompt::Clip(track.clone(), clip_id.clone())),
                    format!("{start_tick}, {offset_frames}, {duration_frames}, {gain}, {pan}, {}", u8::from(*muted)),
                    *base,
                    track.clone(),
                )
            }
            _ => return Err("此窗口没有精确数值输入".into()),
        };
        if base != self.revision() || track != self.session.track {
            return Err("工程或轨道已变更，请重新打开编辑窗口".into());
        }
        // Switching input modes carries the draft and its original revision; it commits nothing.
        self.modal_dialog = None;
        self.prompt_text(title, purpose, &text, window, cx);
        if let Some(prompt) = &mut self.text_prompt {
            prompt.base = base;
        }
        Ok(())
    }

    pub fn close_modal_dialog(&mut self, confirm: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.modal_dialog.take() else { return };
        if !confirm { cx.notify(); return; }
        let result = self.apply_modal_dialog(&dialog, cx);
        if let Err(err) = result {
            self.notice(err);
            self.modal_dialog = Some(dialog);
            window.focus(&self.focus_dialog, cx);
        }
        cx.notify();
    }

    fn apply_modal_dialog(&mut self, dialog: &ModalDialog, cx: &mut Context<Self>) -> Result<(), String> {
        match dialog {
            ModalDialog::Scale { root, intervals } => {
                let mut sorted = intervals.clone();
                sorted.retain(|&d| d < 12);
                sorted.sort_unstable();
                sorted.dedup();
                self.integrations.scale_root = (*root).min(11);
                self.integrations.scale_intervals = sorted;
                Ok(())
            }
            ModalDialog::Segment { editing_key, track_index, start_tick, end_tick, selected_id, base } => {
                if *base != self.revision() {
                    return Err("工程已变更，请重新打开分段对话框".into());
                }
                if *start_tick < 0 || *end_tick <= *start_tick {
                    return Err("结束位置必须晚于起始位置，且不能小于 0".into());
                }
                let track = self.project().tracks.get(*track_index)
                    .map(|t| t.id.clone())
                    .ok_or("所选轨道不存在")?;
                let mut settings = verdant_backend::phonemizer::settings_from_project(self.project())?;
                let mut unknown = Default::default();
                let key = match editing_key {
                    Some(old) => {
                        if let Some(prev) = settings.segments.remove(old) {
                            unknown = prev.unknown;
                        }
                        old.clone()
                    }
                    None => verdant_domain::NoteId::generate().to_string(),
                };
                settings.segments.insert(key, verdant_backend::phonemizer::SegmentEntry {
                    track,
                    start_tick: *start_tick,
                    end_tick: *end_tick,
                    selected: selected_id.clone(),
                    unknown,
                });
                let value = verdant_backend::phonemizer::settings_to_extension(&settings)?;
                if !self.commit(*base, CommandBatch::single("设置读音器分段映射", Command::SetProjectExtension {
                    key: verdant_backend::phonemizer::EXTENSION_KEY.into(),
                    value: Some(value),
                }), cx) {
                    return Err("分段映射未提交".into());
                }
                Ok(())
            }
            ModalDialog::VibratoShape {
                depth_cents, rate_hz, delay_percent, fade_in_percent, fade_out_percent, phase_deg, portamento_curve, base,
            } => {
                if *base != self.revision() {
                    return Err("工程已变更，请重新打开颤音设置".into());
                }
                let mut notes = self.selected_notes();
                if notes.is_empty() {
                    return Err("当前没有选中音符".into());
                }
                for note in &mut notes {
                    let mut vib = note.vibrato.clone().unwrap_or(Vibrato::DEFAULT);
                    vib.depth_cents = depth_cents.round().clamp(0.0, 200.0) as u32;
                    vib.rate_millihertz = (rate_hz * 1000.0).round().clamp(1000.0, 12_000.0) as u32;
                    vib.delay_permille = (delay_percent * 10.0).round().clamp(0.0, 1000.0) as u32;
                    vib.set_fades_phase(
                        (fade_in_percent * 10.0).round().clamp(0.0, 1000.0) as u32,
                        (fade_out_percent * 10.0).round().clamp(0.0, 1000.0) as u32,
                        (phase_deg * 1000.0).round().clamp(0.0, 360_000.0) as i64,
                    );
                    note.vibrato = if vib.depth_cents == 0 { None } else { Some(vib) };
                    note.set_modulation_value(NoteModulationParameter::PortamentoCurve, *portamento_curve)
                        .map_err(String::from)?;
                }
                self.commit(*base, CommandBatch::single("编辑颤音包络与滑音曲率", Command::ReplaceNotes {
                    track: self.session.track.clone(),
                    notes,
                }), cx);
                Ok(())
            }
            ModalDialog::TransformPoints { track, offset_ticks, time_scale_percent, value_offset, value_scale_percent, base } => {
                if *base != self.revision() || *track != self.session.track {
                    return Err("工程已变更，请重新打开控制点变换".into());
                }
                self.transform_selected_points(
                    *offset_ticks,
                    *time_scale_percent / 100.0,
                    *value_offset,
                    *value_scale_percent / 100.0,
                    cx,
                )
            }
            ModalDialog::AudioClipEdit {
                track, clip_id, start_tick, offset_frames, duration_frames, gain_db, gain_changed, pan_percent, pan_changed, muted, base, ..
            } => {
                if *base != self.revision() || *track != self.session.track {
                    return Err("工程已变更，请重新打开音频片段设置".into());
                }
                let mut clips = verdant_media::read_clips(self.project().track(track).ok_or("轨道不存在")?)
                    .map_err(|e| e.to_string())?;
                let clip = clips.iter_mut().find(|c| c.id.to_string() == *clip_id).ok_or("音频片段不存在")?;
                let placement = if *start_tick == clip.start_tick { clip.tick() } else { (*start_tick).max(0) as f64 };
                *clip = clip.trim_source(*offset_frames, *duration_frames, placement, &self.tempo()).map_err(|e| e.to_string())?;
                // Display conversions must never rewrite untouched source values.
                if *gain_changed {
                    clip.gain = clip_dialog_gain(*gain_db);
                }
                if *pan_changed {
                    clip.pan = (*pan_percent as f32 / 100.0).clamp(-1.0, 1.0);
                }
                clip.muted = *muted;
                clip.validate().map_err(|e| e.to_string())?;
                let value = verdant_media::clips_value(track.clone(), &clips).map_err(|e| e.to_string())?;
                if !self.commit(*base, CommandBatch::single("编辑音频片段", Command::SetTrackData {
                    track: track.clone(),
                    key: verdant_media::AUDIO_CLIPS_KEY.into(),
                    value,
                }), cx) {
                    return Err("音频片段修改未提交".into());
                }
                self.rebuild_timeline(cx);
                Ok(())
            }
        }
    }
}

/// Inline Inspector card shown when an audio accompaniment/reference clip is selected.
pub fn audio_clip_inspector_section(ed: &Editor, entity: Entity<Editor>, cx: &mut Context<Editor>) -> Option<Div> {
    let (track_id, clip_id) = ed.integrations.selected_clip.as_ref()?;
    let track = ed.project().track(track_id)?;
    let clips = verdant_media::read_clips(track).ok()?;
    let clip = clips.iter().find(|c| c.id.to_string() == *clip_id)?.clone();
    let th = ed.theme;
    let rate = clip.source.sample_rate.max(1);
    let offset_s = clip.source_offset_frames as f64 / rate as f64;
    let duration_s = clip.duration_seconds();
    let gain_db = if clip.gain <= 1e-5 { -96.0 } else { (20.0 * (clip.gain as f64).log10()).clamp(-96.0, 12.0) };
    let pan_pct = (clip.pan as f64 * 100.0).clamp(-100.0, 100.0);
    let pos = ed.meter().position_at(clip.start_tick);
    let file_name = std::path::Path::new(&clip.source.path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (t_gain, id_gain) = (track_id.clone(), clip_id.clone());
    let (t_pan, id_pan) = (track_id.clone(), clip_id.clone());
    let (t_mute, id_mute, was_muted) = (track_id.clone(), clip_id.clone(), clip.muted);

    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .px(px(12.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(th.shade)
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(th.text_secondary).child("音频片段 (伴奏 / 参考音)"))
            .child(div().text_xs().truncate().text_color(th.text).child(format!("♫ {file_name}")))
            .child(
                div()
                    .text_xs()
                    .text_color(th.text_secondary)
                    .child(format!("起点 {}:{} · 偏移 {:.2}s · 时长 {:.2}s", pos.bar, pos.beat, offset_s, duration_s)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(th.text_secondary).child("片段增益"))
                    .child(format_gain_db(gain_db)),
            )
            .child(horizontal_slider(
                entity.clone(),
                "clip-inspector-gain",
                gain_db,
                -48.0,
                12.0,
                Some(0.0),
                th,
                move |ed, db, commit, cx| {
                    if !commit { return; }
                    if let Some(tr) = ed.project().track(&t_gain) {
                        if let Ok(mut list) = verdant_media::read_clips(tr) {
                            if let Some(c) = list.iter_mut().find(|c| c.id.to_string() == id_gain) {
                                c.gain = if db <= -47.9 { 0.0 } else { 10f32.powf(db as f32 / 20.0) };
                                if let Ok(val) = verdant_media::clips_value(t_gain.clone(), &list) {
                                    ed.commit(ed.revision(), CommandBatch::single("音频片段增益", Command::SetTrackData {
                                        track: t_gain.clone(), key: verdant_media::AUDIO_CLIPS_KEY.into(), value: val,
                                    }), cx);
                                    ed.rebuild_timeline(cx);
                                }
                            }
                        }
                    }
                },
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(th.text_secondary).child("片段声像"))
                    .child(format_pan_percent(pan_pct)),
            )
            .child(horizontal_slider(
                entity,
                "clip-inspector-pan",
                pan_pct,
                -100.0,
                100.0,
                Some(0.0),
                th,
                move |ed, pct, commit, cx| {
                    if !commit { return; }
                    if let Some(tr) = ed.project().track(&t_pan) {
                        if let Ok(mut list) = verdant_media::read_clips(tr) {
                            if let Some(c) = list.iter_mut().find(|c| c.id.to_string() == id_pan) {
                                c.pan = (pct as f32 / 100.0).clamp(-1.0, 1.0);
                                if let Ok(val) = verdant_media::clips_value(t_pan.clone(), &list) {
                                    ed.commit(ed.revision(), CommandBatch::single("音频片段声像", Command::SetTrackData {
                                        track: t_pan.clone(), key: verdant_media::AUDIO_CLIPS_KEY.into(), value: val,
                                    }), cx);
                                    ed.rebuild_timeline(cx);
                                }
                            }
                        }
                    }
                },
            ))
            .child(
                div()
                    .flex()
                    .gap(px(5.0))
                    .child(
                        selectable_button("clip-mute-btn", if was_muted { "已静音" } else { "静音片段" }, was_muted, th)
                            .flex_1()
                            .on_click(cx.listener(move |ed, _, _, cx| {
                                if let Some(tr) = ed.project().track(&t_mute) {
                                    if let Ok(mut list) = verdant_media::read_clips(tr) {
                                        if let Some(c) = list.iter_mut().find(|c| c.id.to_string() == id_mute) {
                                            c.muted = !was_muted;
                                            if let Ok(val) = verdant_media::clips_value(t_mute.clone(), &list) {
                                                ed.commit(ed.revision(), CommandBatch::single("音频片段静音", Command::SetTrackData {
                                                    track: t_mute.clone(), key: verdant_media::AUDIO_CLIPS_KEY.into(), value: val,
                                                }), cx);
                                                ed.rebuild_timeline(cx);
                                            }
                                        }
                                    }
                                }
                            })),
                    )
                    .child(
                        selectable_button("clip-detail-modal", "精确时间裁剪…", false, th)
                            .flex_1()
                            .on_click(cx.listener(|ed, _, window, cx| {
                                if let Err(e) = ed.open_audio_clip_dialog(window, cx) {
                                    ed.notice(e);
                                    cx.notify();
                                }
                            })),
                    ),
            ),
    )
}

fn modal_revision(ed: &Editor) -> String {
    format!("{}:{}:{:?}", ed.document_generation, ed.revision().get(), ed.modal_dialog)
}

pub fn modal_dialog_layer(ed: &Editor, entity: &Entity<Editor>, _cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let dialog = ed.modal_dialog.as_ref()?;
    let label = match dialog {
        ModalDialog::Scale { .. } => "音阶与调式高亮设置",
        ModalDialog::Segment { editing_key, .. } => if editing_key.is_some() { "编辑多语言读音器分段" } else { "新建多语言读音器分段" },
        ModalDialog::VibratoShape { .. } => "颤音包络与滑音曲率",
        ModalDialog::TransformPoints { .. } => "参数控制点批量变换",
        ModalDialog::AudioClipEdit { .. } => "音频片段精确位置与裁剪",
    };
    let revision = modal_revision(ed);
    let props = serde_json::json!({"label":label, "role":"dialog", "open":true, "revision":revision,
        "choices":2, "defaultChoice":1, "cancelChoice":0});
    let draw_editor = entity.clone(); let basis = revision.clone();
    let draw = std::rc::Rc::new(move |handle: &verdant_overlay_control::OverlayControlHandle, focus: &gpui::FocusHandle, window: &mut Window, cx: &mut gpui::App| {
        let binding = window.use_keyed_state("modal-dialog-native-scroll", cx, |_, _| {
            let scroll = gpui::ScrollHandle::new();
            let target = crate::proto_scroll::NativeScroll::new(scroll.clone()); (scroll, target)
        });
        let (scroll, target) = binding.read(cx).clone();
        let editor = draw_editor.clone(); let focus = focus.clone(); let handle = handle.clone(); let basis = basis.clone();
        crate::proto_scroll::content("compiled-modal-dialog-scroll", "对话框内容", move |_, cx| {
            editor.update(cx, |ed, cx| {
                ed.focus_dialog = focus.clone();
                native_modal_dialog_layer(ed, &cx.entity(), handle.clone(), basis.clone(), &scroll, cx).unwrap_or_else(|| div().into_any_element())
            })
        }).tab_stop(false).native_scroll(target).into_any_element()
    });
    let editor = entity.downgrade();
    let response = std::rc::Rc::new(move |index: usize, window: &mut Window, cx: &mut gpui::App| {
        let _ = editor.update(cx, |ed, cx| {
            if modal_revision(ed) == revision { ed.close_modal_dialog(index == 1, window, cx); }
        });
    });
    Some(crate::proto_overlay::overlay("compiled-modal-dialog", props, draw, response).into_any_element())
}

fn native_modal_dialog_layer(ed: &Editor, entity: &Entity<Editor>, handle: verdant_overlay_control::OverlayControlHandle,
    revision: String, scroll: &gpui::ScrollHandle, cx: &mut Context<Editor>) -> Option<gpui::AnyElement> {
    let dialog = ed.modal_dialog.clone()?;
    let th = ed.theme;
    let (title, body): (&str, Div) = match dialog {
        ModalDialog::Scale { root, intervals } => ("音阶与调式高亮设置", render_scale_body(ed, root, &intervals, cx)),
        ModalDialog::Segment { editing_key, track_index, start_tick, end_tick, selected_id, .. } => (
            if editing_key.is_some() { "编辑多语言读音器分段" } else { "新建多语言读音器分段" },
            render_segment_body(ed, track_index, start_tick, end_tick, &selected_id, cx),
        ),
        ModalDialog::VibratoShape {
            depth_cents, rate_hz, delay_percent, fade_in_percent, fade_out_percent, phase_deg, portamento_curve, ..
        } => (
            "颤音包络与滑音曲率",
            render_vibrato_body(entity.clone(), depth_cents, rate_hz, delay_percent, fade_in_percent, fade_out_percent, phase_deg, portamento_curve, th, cx),
        ),
        ModalDialog::TransformPoints { offset_ticks, time_scale_percent, value_offset, value_scale_percent, .. } => (
            "参数控制点批量变换",
            render_transform_body(entity.clone(), offset_ticks, time_scale_percent, value_offset, value_scale_percent, th, cx),
        ),
        ModalDialog::AudioClipEdit { start_tick, offset_frames, duration_frames, sample_rate, gain_db, pan_percent, muted, .. } => (
            "音频片段精确位置与裁剪",
            render_clip_body(ed, entity.clone(), start_tick, offset_frames, duration_frames, sample_rate, gain_db, pan_percent, muted, cx),
        ),
    };

    let keyboard = handle.clone(); let close = handle.clone(); let cancel = handle.clone();
    let close_revision = revision.clone(); let cancel_revision = revision.clone();
    Some(
        div()
            .id("modal-dialog-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(with_alpha(th.window, 0.78))
            .occlude()
            .child(
                div()
                    .id("modal-dialog")
                    .key_context("ModalDialog")
                    .w(px(560.0))
                    .max_h(px(680.0))
                    .overflow_y_scroll()
                    .track_scroll(scroll)
                    .p(px(18.0))
                    .rounded(px(12.0))
                    .bg(th.popover)
                    .border_1()
                    .border_color(th.shade)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .on_key_down(move |event, window, cx| crate::proto_overlay::key(&keyboard, event, window, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(th.shade)
                            .pb(px(10.0))
                            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
                            .child(
                                selectable_button("modal-close-x", "✕", false, th)
                                    .on_click(move |_, w, cx| crate::proto_overlay::choose(&close, 0, &close_revision, w, cx)),
                            ),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.0))
                            .border_t_1()
                            .border_color(th.shade)
                            .pt(px(10.0))
                            .child(
                                selectable_button("modal-cancel", "取消 (Esc)", false, th)
                                    .on_click(move |_, w, cx| crate::proto_overlay::choose(&cancel, 0, &cancel_revision, w, cx)),
                            )
                            .child(
                                selectable_button("modal-confirm", "应用 (Enter)", true, th)
                                    .on_click(move |_, w, cx| crate::proto_overlay::choose(&handle, 1, &revision, w, cx)),
                            ),
                    ),
            )
            .into_any_element(),
    )
}

fn render_scale_body(ed: &Editor, root: u8, intervals: &[u8], cx: &mut Context<Editor>) -> Div {
    let th = ed.theme;
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().text_xs().text_color(th.text_secondary).child("选择主音（Root）与调式预设，或单独开关 12 个半音音级。卷帘与琴键将高亮音阶内音高。"))
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("主音 (Root)"))
        .child(
            div().flex().flex_wrap().gap(px(4.0)).children((0u8..12).map(|r| {
                selectable_button(SharedString::from(format!("scale-root-{r}")), NOTE_ROOT_NAMES[r as usize], root == r, th)
                    .on_click(cx.listener(move |ed, _, _, cx| {
                        if let Some(ModalDialog::Scale { root: cur, .. }) = &mut ed.modal_dialog {
                            *cur = r;
                            cx.notify();
                        }
                    }))
            })),
        )
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("调式预设"))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(5.0))
                .child(
                    selectable_button("scale-preset-off", "关闭音阶高亮", intervals.is_empty(), th).on_click(cx.listener(
                        |ed, _, _, cx| {
                            if let Some(ModalDialog::Scale { intervals: iv, .. }) = &mut ed.modal_dialog {
                                iv.clear();
                                cx.notify();
                            }
                        },
                    )),
                )
                .children(SCALE_PRESETS.iter().enumerate().map(|(i, &(name, degs))| {
                    let active = intervals == degs;
                    selectable_button(SharedString::from(format!("scale-preset-{i}")), name, active, th).on_click(cx.listener(
                        move |ed, _, _, cx| {
                            if let Some(ModalDialog::Scale { intervals: iv, .. }) = &mut ed.modal_dialog {
                                *iv = degs.to_vec();
                                cx.notify();
                            }
                        },
                    ))
                })),
        )
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("音级自定义开关"))
        .child(
            div().flex().flex_wrap().gap(px(4.0)).children((0u8..12).map(|deg| {
                let on = intervals.contains(&deg);
                let pitch_name = NOTE_ROOT_NAMES[((root + deg) % 12) as usize];
                selectable_button(
                    SharedString::from(format!("scale-deg-{deg}")),
                    format!("{} ({pitch_name})", DEGREE_LABELS[deg as usize]),
                    on,
                    th,
                )
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if let Some(ModalDialog::Scale { intervals: iv, .. }) = &mut ed.modal_dialog {
                        if let Some(pos) = iv.iter().position(|&x| x == deg) {
                            iv.remove(pos);
                        } else {
                            iv.push(deg);
                            iv.sort_unstable();
                        }
                        cx.notify();
                    }
                }))
            })),
        )
}

fn render_segment_body(
    ed: &Editor,
    track_index: usize,
    start_tick: i64,
    end_tick: i64,
    selected_id: &str,
    cx: &mut Context<Editor>,
) -> Div {
    let th = ed.theme;
    let meter = ed.meter();
    let bar_ticks = meter.bar_ticks_at(start_tick).max(1);
    let beat_ticks = ed.project().ppq.max(1);
    let s_pos = meter.position_at(start_tick);
    let e_pos = meter.position_at(end_tick);

    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().text_xs().text_color(th.text_secondary).child("为同一轨道的指定区间分配独立读音器（如主歌日语假名、副歌普通话字典），无需手填 tick 与 ID。"))
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("目标轨道"))
        .child(
            div().flex().flex_wrap().gap(px(5.0)).children(ed.project().tracks.iter().enumerate().map(|(i, tr)| {
                selectable_button(
                    SharedString::from(format!("seg-track-{i}")),
                    format!("{} · {}", i + 1, tr.name),
                    track_index == i,
                    th,
                )
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if let Some(ModalDialog::Segment { track_index: ti, .. }) = &mut ed.modal_dialog {
                        *ti = i;
                        cx.notify();
                    }
                }))
            })),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("区间范围"))
                .child(
                    div()
                        .flex()
                        .gap(px(4.0))
                        .child(selectable_button("seg-from-selection", "设为选中音符区间", false, th).on_click(cx.listener(
                            |ed, _, _, cx| {
                                let notes = ed.selected_notes();
                                if let (Some(f), Some(l)) = (notes.first(), notes.last()) {
                                    let (s, e) = (f.start_tick, l.end_tick());
                                    if let Some(ModalDialog::Segment { start_tick: st, end_tick: et, .. }) = &mut ed.modal_dialog {
                                        *st = s;
                                        *et = e;
                                        cx.notify();
                                    }
                                } else {
                                    ed.notice("当前未选中音符");
                                    cx.notify();
                                }
                            },
                        )))
                        .child(selectable_button("seg-from-loop", "设为循环区间", false, th).on_click(cx.listener(|ed, _, _, cx| {
                            let pb = ed.playback_settings();
                            if pb.loop_end > pb.loop_start {
                                if let Some(ModalDialog::Segment { start_tick: st, end_tick: et, .. }) = &mut ed.modal_dialog {
                                    *st = pb.loop_start;
                                    *et = pb.loop_end;
                                    cx.notify();
                                }
                            } else {
                                ed.notice("尚未设置循环区间");
                                cx.notify();
                            }
                        }))),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_xs()
                .child(div().w(px(68.0)).text_color(th.text_secondary).child("起始位置"))
                .child(div().flex_1().child(format!("第 {} 小节 第 {} 拍 ({start_tick} tick)", s_pos.bar, s_pos.beat)))
                .children([("-1小节", -bar_ticks), ("-1拍", -beat_ticks), ("+1拍", beat_ticks), ("+1小节", bar_ticks)].into_iter().map(
                    |(lbl, delta)| {
                        selectable_button(SharedString::from(format!("seg-s-{lbl}")), lbl, false, th).on_click(cx.listener(
                            move |ed, _, _, cx| {
                                if let Some(ModalDialog::Segment { start_tick: st, end_tick: et, .. }) = &mut ed.modal_dialog {
                                    *st = (*st + delta).max(0);
                                    if *et <= *st {
                                        *et = *st + beat_ticks;
                                    }
                                    cx.notify();
                                }
                            },
                        ))
                    },
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_xs()
                .child(div().w(px(68.0)).text_color(th.text_secondary).child("结束位置"))
                .child(div().flex_1().child(format!("第 {} 小节 第 {} 拍 ({end_tick} tick)", e_pos.bar, e_pos.beat)))
                .children([("-1小节", -bar_ticks), ("-1拍", -beat_ticks), ("+1拍", beat_ticks), ("+1小节", bar_ticks)].into_iter().map(
                    |(lbl, delta)| {
                        selectable_button(SharedString::from(format!("seg-e-{lbl}")), lbl, false, th).on_click(cx.listener(
                            move |ed, _, _, cx| {
                                if let Some(ModalDialog::Segment { start_tick: st, end_tick: et, .. }) = &mut ed.modal_dialog {
                                    *et = (*et + delta).max(*st + 1);
                                    cx.notify();
                                }
                            },
                        ))
                    },
                )),
        )
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("区间读音器"))
        .child(
            div().flex().flex_col().gap(px(5.0)).children(verdant_backend::phonemizer::capabilities().iter().map(|cap| {
                let id_str = cap.id.to_string();
                let active = selected_id == cap.id;
                selectable_button(
                    SharedString::from(format!("seg-phonemizer-{}", cap.id)),
                    format!("{} ({})", cap.name, cap.id),
                    active,
                    th,
                )
                .justify_start()
                .on_click(cx.listener(move |ed, _, _, cx| {
                    if let Some(ModalDialog::Segment { selected_id: sel, .. }) = &mut ed.modal_dialog {
                        *sel = id_str.clone();
                        cx.notify();
                    }
                }))
            })),
        )
}

fn render_vibrato_body(
    entity: Entity<Editor>,
    depth_cents: f64,
    rate_hz: f64,
    delay_percent: f64,
    fade_in_percent: f64,
    fade_out_percent: f64,
    phase_deg: f64,
    portamento_curve: f64,
    th: Theme,
    cx: &mut Context<Editor>,
) -> Div {
    let slider_row = |key: &'static str, label: &'static str, readout: String, val: f64, min: f64, max: f64, def: f64, precision: SliderPrecision,
                      setter: fn(&mut ModalDialog, f64)| {
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(th.text_secondary).child(label))
                    .child(readout),
            )
            .child(horizontal_slider_with_precision(entity.clone(), key, val, min, max, Some(def), precision, th, move |ed, v, _, cx| {
                if let Some(d) = &mut ed.modal_dialog {
                    setter(d, v);
                    cx.notify();
                }
            }))
    };

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(div().text_xs().text_color(th.text_secondary).child("快速预设："))
                .children(VibratoPreset::ALL.into_iter().map(|preset| {
                    selectable_button(SharedString::from(format!("vib-preset-{preset:?}")), preset.label(), false, th).on_click(
                        cx.listener(move |ed, _, _, cx| {
                            let mut v = Vibrato::DEFAULT;
                            preset.apply(&mut v);
                            if let Some(ModalDialog::VibratoShape {
                                depth_cents: d, rate_hz: r, delay_percent: dl, fade_in_percent: fi, fade_out_percent: fo, phase_deg: ph, ..
                            }) = &mut ed.modal_dialog
                            {
                                *d = v.depth_cents as f64;
                                *r = v.rate_millihertz as f64 / 1000.0;
                                *dl = v.delay_permille as f64 / 10.0;
                                *fi = v.fade_in_permille() as f64 / 10.0;
                                *fo = v.fade_out_permille() as f64 / 10.0;
                                *ph = v.phase_millidegrees() as f64 / 1000.0;
                                cx.notify();
                            }
                        }),
                    )
                }))
                .child(selectable_button("vib-preset-off", "关闭颤音", depth_cents == 0.0, th).on_click(cx.listener(|ed, _, _, cx| {
                    if let Some(ModalDialog::VibratoShape { depth_cents: d, .. }) = &mut ed.modal_dialog {
                        *d = 0.0;
                        cx.notify();
                    }
                }))),
        )
        .child(slider_row("vib-depth", "颤音深度", format!("{depth_cents:.0} ¢"), depth_cents, 0.0, 200.0, 35.0, SliderPrecision::Whole, |d, v| {
            if let ModalDialog::VibratoShape { depth_cents, .. } = d { *depth_cents = v.round(); }
        }))
        .child(slider_row("vib-rate", "颤音速率", format!("{rate_hz:.2} Hz"), rate_hz, 1.0, 12.0, 5.5, SliderPrecision::Twentieth, |d, v| {
            if let ModalDialog::VibratoShape { rate_hz, .. } = d { *rate_hz = (v * 20.0).round() / 20.0; }
        }))
        .child(slider_row("vib-delay", "起始延迟 (音符前段平直占比)", format!("{delay_percent:.0}%"), delay_percent, 0.0, 100.0, 35.0, SliderPrecision::Whole, |d, v| {
            if let ModalDialog::VibratoShape { delay_percent, .. } = d { *delay_percent = v.round(); }
        }))
        .child(slider_row("vib-fadein", "包络淡入", format!("{fade_in_percent:.0}%"), fade_in_percent, 0.0, 100.0, 15.0, SliderPrecision::Whole, |d, v| {
            if let ModalDialog::VibratoShape { fade_in_percent, .. } = d { *fade_in_percent = v.round(); }
        }))
        .child(slider_row("vib-fadeout", "包络淡出", format!("{fade_out_percent:.0}%"), fade_out_percent, 0.0, 100.0, 15.0, SliderPrecision::Whole, |d, v| {
            if let ModalDialog::VibratoShape { fade_out_percent, .. } = d { *fade_out_percent = v.round(); }
        }))
        .child(slider_row("vib-phase", "起始相位", format!("{phase_deg:.0}°"), phase_deg, 0.0, 360.0, 0.0, SliderPrecision::Whole, |d, v| {
            if let ModalDialog::VibratoShape { phase_deg, .. } = d { *phase_deg = v.round(); }
        }))
        .child(slider_row("vib-porta-curve", "入音滑音曲率 (S型 / 急弯)", format!("{portamento_curve:+.0}%"), portamento_curve, -100.0, 100.0, 0.0, SliderPrecision::Whole, |d, v| {
            if let ModalDialog::VibratoShape { portamento_curve, .. } = d { *portamento_curve = v.round(); }
        }))
}

fn render_transform_body(
    entity: Entity<Editor>,
    offset_ticks: i64,
    time_scale_percent: f64,
    value_offset: f64,
    value_scale_percent: f64,
    th: Theme,
    cx: &mut Context<Editor>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(selectable_button("tf-exact", "精确数值输入…（不限滑块范围）", false, th).on_click(cx.listener(|ed, _, window, cx| {
            if let Err(error) = ed.prompt_exact_dialog(window, cx) {
                ed.notice(error);
                cx.notify();
            }
        })))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(5.0))
                .child(selectable_button("tf-invert", "上下反转 (-100%)", value_scale_percent == -100.0, th).on_click(cx.listener(
                    |ed, _, _, cx| {
                        if let Some(ModalDialog::TransformPoints { value_scale_percent: vs, .. }) = &mut ed.modal_dialog {
                            *vs = -100.0;
                            cx.notify();
                        }
                    },
                )))
                .child(selectable_button("tf-half", "幅度减半 (50%)", value_scale_percent == 50.0, th).on_click(cx.listener(
                    |ed, _, _, cx| {
                        if let Some(ModalDialog::TransformPoints { value_scale_percent: vs, .. }) = &mut ed.modal_dialog {
                            *vs = 50.0;
                            cx.notify();
                        }
                    },
                )))
                .child(selectable_button("tf-boost", "幅度增强 (150%)", value_scale_percent == 150.0, th).on_click(cx.listener(
                    |ed, _, _, cx| {
                        if let Some(ModalDialog::TransformPoints { value_scale_percent: vs, .. }) = &mut ed.modal_dialog {
                            *vs = 150.0;
                            cx.notify();
                        }
                    },
                )))
                .child(selectable_button("tf-reset", "重置为单位变换", false, th).on_click(cx.listener(|ed, _, _, cx| {
                    if let Some(ModalDialog::TransformPoints {
                        offset_ticks: ot, time_scale_percent: ts, value_offset: vo, value_scale_percent: vs, ..
                    }) = &mut ed.modal_dialog
                    {
                        *ot = 0;
                        *ts = 100.0;
                        *vo = 0.0;
                        *vs = 100.0;
                        cx.notify();
                    }
                }))),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child("时间平移 (tick)"))
                .child(format!("{offset_ticks:+} tick")),
        )
        .child(horizontal_slider_with_precision(entity.clone(), "tf-offset-ticks", offset_ticks as f64, -960.0, 960.0, Some(0.0), SliderPrecision::Whole, th, |ed, v, _, cx| {
            if let Some(ModalDialog::TransformPoints { offset_ticks, .. }) = &mut ed.modal_dialog {
                *offset_ticks = v.round() as i64;
                cx.notify();
            }
        }))
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child("时间伸缩 (%)"))
                .child(format!("{time_scale_percent:.0}%")),
        )
        .child(horizontal_slider_with_precision(entity.clone(), "tf-time-scale", time_scale_percent, 25.0, 200.0, Some(100.0), SliderPrecision::Whole, th, |ed, v, _, cx| {
            if let Some(ModalDialog::TransformPoints { time_scale_percent, .. }) = &mut ed.modal_dialog {
                *time_scale_percent = v.round();
                cx.notify();
            }
        }))
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child("数值偏移 (¢ / dB / 表达量)"))
                .child(format!("{value_offset:+.1}")),
        )
        .child(horizontal_slider_with_precision(entity.clone(), "tf-val-offset", value_offset, -100.0, 100.0, Some(0.0), SliderPrecision::Half, th, |ed, v, _, cx| {
            if let Some(ModalDialog::TransformPoints { value_offset, .. }) = &mut ed.modal_dialog {
                *value_offset = (v * 2.0).round() / 2.0;
                cx.notify();
            }
        }))
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child("数值缩放 (%)"))
                .child(format!("{value_scale_percent:.0}%")),
        )
        .child(horizontal_slider_with_precision(entity, "tf-val-scale", value_scale_percent, -200.0, 200.0, Some(100.0), SliderPrecision::Whole, th, |ed, v, _, cx| {
            if let Some(ModalDialog::TransformPoints { value_scale_percent, .. }) = &mut ed.modal_dialog {
                *value_scale_percent = v.round();
                cx.notify();
            }
        }))
}

fn render_clip_body(
    ed: &Editor,
    entity: Entity<Editor>,
    start_tick: i64,
    offset_frames: u64,
    duration_frames: u64,
    sample_rate: u32,
    gain_db: f64,
    pan_percent: f64,
    muted: bool,
    cx: &mut Context<Editor>,
) -> Div {
    let th = ed.theme;
    let meter = ed.meter();
    let pos = meter.position_at(start_tick);
    let beat = ed.project().ppq.max(1);
    let bar = meter.bar_ticks_at(start_tick).max(1);
    let offset_seconds = offset_frames as f64 / sample_rate as f64;
    let duration_seconds = duration_frames as f64 / sample_rate as f64;

    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(selectable_button("clip-exact", "精确输入 tick / 帧 / 线性增益…", false, th).on_click(cx.listener(|ed, _, window, cx| {
            if let Err(error) = ed.prompt_exact_dialog(window, cx) {
                ed.notice(error);
                cx.notify();
            }
        })))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_xs()
                .child(div().w(px(90.0)).text_color(th.text_secondary).child("排列起点"))
                .child(div().flex_1().child(format!("第 {} 小节 第 {} 拍 ({start_tick} tick)", pos.bar, pos.beat)))
                .children([("-1小节", -bar), ("-1拍", -beat), ("+1拍", beat), ("+1小节", bar)].into_iter().map(|(lbl, d)| {
                    selectable_button(SharedString::from(format!("clip-st-{lbl}")), lbl, false, th).on_click(cx.listener(
                        move |ed, _, _, cx| {
                            if let Some(ModalDialog::AudioClipEdit { start_tick: st, .. }) = &mut ed.modal_dialog {
                                *st = st.saturating_add(d).max(0);
                                cx.notify();
                            }
                        },
                    ))
                })),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_xs()
                .child(div().w(px(90.0)).text_color(th.text_secondary).child("源裁剪起点"))
                .child(div().flex_1().child(format!("{offset_seconds:.3} 秒 ({offset_frames} 帧)")))
                .children([("-1.0s", -1.0), ("-0.1s", -0.1), ("+0.1s", 0.1), ("+1.0s", 1.0)].into_iter().map(|(lbl, d)| {
                    selectable_button(SharedString::from(format!("clip-off-{lbl}")), lbl, false, th).on_click(cx.listener(
                        move |ed, _, _, cx| {
                            if let Some(ModalDialog::AudioClipEdit { offset_frames, sample_rate, .. }) = &mut ed.modal_dialog {
                                *offset_frames = nudge_clip_frames(*offset_frames, d, *sample_rate);
                                cx.notify();
                            }
                        },
                    ))
                })),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_xs()
                .child(div().w(px(90.0)).text_color(th.text_secondary).child("片段持续时长"))
                .child(div().flex_1().child(format!("{duration_seconds:.3} 秒 ({duration_frames} 帧)")))
                .children([("-1.0s", -1.0), ("-0.1s", -0.1), ("+0.1s", 0.1), ("+1.0s", 1.0)].into_iter().map(|(lbl, d)| {
                    selectable_button(SharedString::from(format!("clip-dur-{lbl}")), lbl, false, th).on_click(cx.listener(
                        move |ed, _, _, cx| {
                            if let Some(ModalDialog::AudioClipEdit { duration_frames, sample_rate, .. }) = &mut ed.modal_dialog {
                                *duration_frames = nudge_clip_frames(*duration_frames, d, *sample_rate).max(1);
                                cx.notify();
                            }
                        },
                    ))
                })),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child("增益"))
                .child(format_gain_db(gain_db)),
        )
        .child(horizontal_slider(entity.clone(), "clip-modal-gain", gain_db, -48.0, 12.0, Some(0.0), th, |ed, v, _, cx| {
            if let Some(ModalDialog::AudioClipEdit { gain_db, gain_changed, .. }) = &mut ed.modal_dialog {
                *gain_db = v;
                *gain_changed = true;
                cx.notify();
            }
        }))
        .child(
            div()
                .flex()
                .justify_between()
                .text_xs()
                .child(div().text_color(th.text_secondary).child("声像"))
                .child(format_pan_percent(pan_percent)),
        )
        .child(horizontal_slider(entity, "clip-modal-pan", pan_percent, -100.0, 100.0, Some(0.0), th, |ed, v, _, cx| {
            if let Some(ModalDialog::AudioClipEdit { pan_percent, pan_changed, .. }) = &mut ed.modal_dialog {
                *pan_percent = v;
                *pan_changed = true;
                cx.notify();
            }
        }))
        .child(selectable_button("clip-modal-mute", if muted { "状态：已静音" } else { "状态：正常发声" }, muted, th).on_click(
            cx.listener(|ed, _, _, cx| {
                if let Some(ModalDialog::AudioClipEdit { muted: m, .. }) = &mut ed.modal_dialog {
                    *m = !*m;
                    cx.notify();
                }
            }),
        ))
}

fn clip_dialog_gain(gain_db: f64) -> f32 {
    if gain_db <= -95.0 { 0.0 } else { 10f32.powf(gain_db as f32 / 20.0) }
}

fn nudge_clip_frames(frames: u64, seconds: f64, sample_rate: u32) -> u64 {
    let delta = (seconds.abs() * sample_rate as f64).round() as u64;
    if seconds < 0.0 { frames.saturating_sub(delta) } else { frames.saturating_add(delta) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_presets_are_strictly_ascending_valid_pitch_classes() {
        for (name, degrees) in SCALE_PRESETS {
            assert!(!degrees.is_empty(), "{name}");
            assert_eq!(degrees[0], 0, "{name} must contain tonic");
            for pair in degrees.windows(2) {
                assert!(pair[0] < pair[1] && pair[1] < 12, "{name} invalid degrees: {degrees:?}");
            }
        }
    }
}

#[cfg(test)]
mod slider_precision_tests {
    use super::*;
    use gpui::{AppContext, Render, Subscription, TestAppContext, point, size};

    #[derive(Clone, Copy)]
    enum Family { Vibrato, Transform }
    struct Case { id: &'static str, family: Family, units: f64, read: fn(&ModalDialog) -> f64 }
    const CASES: [Case; 11] = [
        Case { id: "vib-depth", family: Family::Vibrato, units: 1.0, read: |d| match d {
            ModalDialog::VibratoShape { depth_cents, .. } => *depth_cents, _ => panic!("vibrato draft"),
        } },
        Case { id: "vib-rate", family: Family::Vibrato, units: 20.0, read: |d| match d {
            ModalDialog::VibratoShape { rate_hz, .. } => *rate_hz, _ => panic!("vibrato draft"),
        } },
        Case { id: "vib-delay", family: Family::Vibrato, units: 1.0, read: |d| match d {
            ModalDialog::VibratoShape { delay_percent, .. } => *delay_percent, _ => panic!("vibrato draft"),
        } },
        Case { id: "vib-fadein", family: Family::Vibrato, units: 1.0, read: |d| match d {
            ModalDialog::VibratoShape { fade_in_percent, .. } => *fade_in_percent, _ => panic!("vibrato draft"),
        } },
        Case { id: "vib-fadeout", family: Family::Vibrato, units: 1.0, read: |d| match d {
            ModalDialog::VibratoShape { fade_out_percent, .. } => *fade_out_percent, _ => panic!("vibrato draft"),
        } },
        Case { id: "vib-phase", family: Family::Vibrato, units: 1.0, read: |d| match d {
            ModalDialog::VibratoShape { phase_deg, .. } => *phase_deg, _ => panic!("vibrato draft"),
        } },
        Case { id: "vib-porta-curve", family: Family::Vibrato, units: 1.0, read: |d| match d {
            ModalDialog::VibratoShape { portamento_curve, .. } => *portamento_curve, _ => panic!("vibrato draft"),
        } },
        Case { id: "tf-offset-ticks", family: Family::Transform, units: 1.0, read: |d| match d {
            ModalDialog::TransformPoints { offset_ticks, .. } => *offset_ticks as f64, _ => panic!("transform draft"),
        } },
        Case { id: "tf-time-scale", family: Family::Transform, units: 1.0, read: |d| match d {
            ModalDialog::TransformPoints { time_scale_percent, .. } => *time_scale_percent, _ => panic!("transform draft"),
        } },
        Case { id: "tf-val-offset", family: Family::Transform, units: 2.0, read: |d| match d {
            ModalDialog::TransformPoints { value_offset, .. } => *value_offset, _ => panic!("transform draft"),
        } },
        Case { id: "tf-val-scale", family: Family::Transform, units: 1.0, read: |d| match d {
            ModalDialog::TransformPoints { value_scale_percent, .. } => *value_scale_percent, _ => panic!("transform draft"),
        } },
    ];
    struct Harness { editor: Entity<Editor>, _changed: Subscription }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.editor.update(cx, |ed, cx| {
                let entity = cx.entity();
                div().size_full().children(modal_dialog_layer(ed, &entity, cx))
            })
        }
    }

    #[gpui::test]
    fn native_modal_sliders_keep_quantized_previews_through_normal_and_fine_repaints(cx: &mut TestAppContext) {
        for case in CASES { for fine in [false, true] {
            let window = cx.open_window(size(px(760.0), px(780.0)), |window, cx| {
                let editor = cx.new(|cx| {
                    let mut ed = Editor::for_native_test(verdant_domain::Project::new_empty(), window, cx);
                    ed.modal_dialog = Some(match case.family {
                        Family::Vibrato => ModalDialog::VibratoShape {
                            depth_cents: 35.0, rate_hz: 5.5, delay_percent: 35.0,
                            fade_in_percent: 15.0, fade_out_percent: 15.0,
                            phase_deg: 0.0, portamento_curve: 0.0, base: ed.revision(),
                        },
                        Family::Transform => ModalDialog::TransformPoints {
                            track: ed.session.track.clone(), offset_ticks: 0,
                            time_scale_percent: 100.0, value_offset: 0.0,
                            value_scale_percent: 100.0, base: ed.revision(),
                        },
                    });
                    ed
                });
                let changed = cx.observe(&editor, |_, _, cx| cx.notify());
                Harness { editor, _changed: changed }
            });
            window.update(cx, |_, window, _| window.activate_window()).unwrap();
            crate::native_test_support::frames(window, cx);
            let (source, min, max, initial) = window.update(cx, |host, window, cx| {
                let owners = verdant_continuous_control::native::gpui_native_owner::all_owners().into_iter()
                    .filter(|owner| {
                        let owner = owner.borrow();
                        owner.window_id == Some(window.window_handle().window_id()) && owner.mounted
                            && owner.declared_events.contains("valueCommit")
                            && owner.props.get("revision").and_then(serde_json::Value::as_str)
                                .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                                .is_some_and(|value| value.get("control").and_then(serde_json::Value::as_str) == Some(case.id))
                    }).collect::<Vec<_>>();
                assert_eq!(owners.len(), 1, "{} must render one actual compiled scalar", case.id);
                let source = verdant_continuous_control::ContinuousControlHandle { owner: owners[0].clone() };
                let (min, max) = { let owner = source.owner.borrow();
                    (owner.props["min"].as_f64().unwrap(), owner.props["max"].as_f64().unwrap()) };
                let initial = (case.read)(host.editor.read(cx).modal_dialog.as_ref().unwrap());
                (source, min, max, initial)
            }).unwrap();
            let bounds = source.owner.borrow().native_hitbox.as_ref().expect("painted dialog slider").bounds;
            assert!(bounds.size.width > px(12.0) && bounds.size.height > px(0.0));
            let start = point(bounds.origin.x + px((6.0 + (initial-min)/(max-min)
                * (f64::from(bounds.size.width)-12.0)) as f32), bounds.center().y);
            cx.update_window(window.into(), |_, window, cx| {
                window.dispatch_event(gpui::PlatformInput::MouseDown(MouseDownEvent {
                    position: start, button: MouseButton::Left, ..Default::default()
                }), cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            let mut last = start;
            let mut expected = initial;
            for fraction in [0.4321, 0.7689] {
                let bounds = source.owner.borrow().native_hitbox.as_ref().expect("same slider after repaint").bounds;
                let width = f64::from(bounds.size.width) - 12.0;
                last = point(bounds.origin.x + px((6.0 + fraction*width) as f32), bounds.center().y);
                let candidate = min + (f64::from(last.x)-f64::from(bounds.origin.x)-6.0)/width*(max-min);
                let requested = if fine { initial + (candidate-initial)*0.1 } else { candidate };
                expected = (requested*case.units).round()/case.units;
                cx.update_window(window.into(), |_, window, cx| {
                    window.dispatch_event(gpui::PlatformInput::MouseMove(MouseMoveEvent {
                        position: last, pressed_button: Some(MouseButton::Left),
                        modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
                    }), cx);
                }).unwrap();
                // Rebuild the complete production dialog from its accepted
                // draft before the next physical sample reaches the source.
                crate::native_test_support::frames(window, cx);
                window.update(cx, |host, window, cx| {
                    let editor = host.editor.read(cx);
                    assert_eq!(source.state("phase").unwrap(), serde_json::json!("dragging"), "{}, fine={fine}", case.id);
                    assert_eq!((case.read)(editor.modal_dialog.as_ref().unwrap()), expected, "{}, fine={fine}", case.id);
                    assert_eq!(source.state("value").unwrap().as_f64(), Some(expected));
                    assert_eq!(editor.revision().get(), 0, "dialog previews modify only the draft");
                    assert!(window.captured_hitbox().is_some());
                }).unwrap();
            }
            cx.update_window(window.into(), |_, window, cx| {
                window.dispatch_event(gpui::PlatformInput::MouseUp(MouseUpEvent {
                    position: last, button: MouseButton::Left,
                    modifiers: gpui::Modifiers { shift: fine, ..Default::default() }, ..Default::default()
                }), cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, window, cx| {
                assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
                assert_eq!((case.read)(host.editor.read(cx).modal_dialog.as_ref().unwrap()), expected);
                assert_eq!(source.state("value").unwrap().as_f64(), Some(expected),
                    "release must retain the accepted draft for the next interaction");
                assert_ne!(expected, initial, "the completed gesture must change this draft");
                assert_eq!(host.editor.read(cx).revision().get(), 0, "Apply remains the project transaction boundary");
                assert!(window.captured_hitbox().is_none());
                assert!(source.take_failures().is_empty());
            }).unwrap();
            assert_eq!(source.state("focused").unwrap(), serde_json::json!(true), "the real pointer gesture focuses its scalar");
            let quantum = 1.0/case.units;
            let fine_step = (((max-min)/1000.0*case.units).round()/case.units).max(quantum);
            let step = (((max-min)/100.0*case.units).round()/case.units).max(quantum);
            for _ in 0..2 {
                let before = expected;
                expected = ((expected+fine_step)*case.units).round()/case.units;
                cx.simulate_keystrokes(window.into(), "shift-right");
                crate::native_test_support::frames(window, cx);
                window.update(cx, |host, _, cx| {
                    let editor = host.editor.read(cx);
                    assert_eq!((case.read)(editor.modal_dialog.as_ref().unwrap()), expected, "{} Shift+Right", case.id);
                    assert!(expected > before, "each fine key must advance by a representable increment");
                    assert_eq!(source.state("value").unwrap().as_f64(), Some(expected));
                    assert_eq!(editor.revision().get(), 0);
                }).unwrap();
            }
            expected = ((expected+step)*case.units).round()/case.units;
            cx.update_window(window.into(), |_, window, cx| {
                use verdant_continuous_control::native::gpui_native_owner::{dispatch_accessibility_action, drive_owners};
                // The bridge reports default prevention, not acceptance.
                // This authored action proposes the value without requesting
                // prevention; the draft/source assertions below prove delivery.
                assert!(!dispatch_accessibility_action(&source.owner, "increment", None));
                drive_owners(window, cx);
            }).unwrap();
            crate::native_test_support::frames(window, cx);
            window.update(cx, |host, _, cx| {
                let editor = host.editor.read(cx);
                assert_eq!((case.read)(editor.modal_dialog.as_ref().unwrap()), expected, "{} assistive Increment", case.id);
                assert_eq!(source.state("value").unwrap().as_f64(), Some(expected));
                assert_eq!(source.state("phase").unwrap(), serde_json::json!("idle"));
                assert_eq!(editor.revision().get(), 0);
                assert!(source.take_failures().is_empty());
            }).unwrap();
        } }
    }
}
