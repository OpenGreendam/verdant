//! EditorSession owns viewport, selection and uncommitted gesture data.
//! Gesture handlers choose every gesture transition and ordered effect;
//! `bindings` supplies the named domain operations. Preview never changes the
//! project. A release can emit one batch against the captured base revision.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use verdant_domain::command::resized;
use verdant_domain::{Command, CommandBatch, Edge, Note, NoteId, Project, Revision, TrackId};

use crate::hit::Hit;
use crate::viewport::{MusicPoint, Snap, Viewport};

pub const DRAG_THRESHOLD_PX: f64 = 3.0;
pub const TOUCH_DRAG_THRESHOLD_PX: f64 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifiers {
    pub shift: bool,
    /// Ctrl on Linux/Windows, Cmd on macOS.
    pub secondary: bool,
    pub alt: bool,
}

/// Native hosts with contact IDs use `handle_pointer`; the existing `handle`
/// entry point maps mouse to Mouse and a legacy touch stream to Touch(0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerId { Mouse, Touch(u64), Pen(u64) }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason { Escape, FocusLost, PointerCancel, ProjectChanged }

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    PointerDown { hit: Hit, music: MusicPoint, screen: (f64, f64), mods: Modifiers, click_count: u32 },
    PointerMove { music: MusicPoint, screen: (f64, f64), mods: Modifiers },
    PointerUp,
    Cancel(CancelReason),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Commit { base: Revision, batch: CommandBatch, select_after: Option<Vec<NoteId>> },
    CapturePointer,
    ReleasePointer,
    BeginLyricEdit(NoteId),
    Notice(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GestureKind {
    Move { duplicate: bool },
    Resize { edge: Edge },
    RubberBand { additive: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Armed, Dragging }

#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    None,
    Move { delta_tick: i64, delta_semitones: i32, duplicate: bool },
    Resize { edge: Edge, delta_tick: i64 },
    RubberBand { from: MusicPoint, to: MusicPoint, hits: BTreeSet<NoteId> },
}

impl Preview {
    /// Project a participating note. Gesture ownership is checked by the caller.
    pub fn project_note(&self, note: &Note) -> Option<Note> {
        let (start, duration, pitch) = match *self {
            Self::Move { delta_tick, delta_semitones, .. } => (note.start_tick + delta_tick, note.duration_tick, note.midi_note + delta_semitones),
            Self::Resize { edge, delta_tick } => {
                let (start, duration) = resized(note, edge, delta_tick);
                (start, duration, note.midi_note)
            }
            _ => return None,
        };
        let mut projected = note.clone();
        projected.start_tick = start;
        projected.duration_tick = duration;
        projected.midi_note = pitch;
        Some(projected)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gesture {
    pub kind: GestureKind,
    pub phase: Phase,
    pub track: TrackId,
    pub base_revision: Revision,
    pub origin_screen: (f64, f64),
    pub origin: MusicPoint,
    pub anchor: Option<NoteId>,
    /// Group in time order.
    pub ids: Vec<NoteId>,
    pub preview: Preview,
    pub click_selects_only_anchor: bool,
}

#[derive(Clone, Debug)]
pub struct EditorSession {
    pub track: TrackId,
    pub viewport: Viewport,
    pub snap: Snap,
    pub selection: BTreeSet<NoteId>,
    pub gesture: Option<Gesture>,
    pub pointer_captured: bool,
    pub autoscroll_active: bool,
    pub follow_playhead: bool,
    pub default_note_ticks: i64,
    pub notice: Option<String>,
    /// Legacy host input source; capture latches the source/contact identity.
    pub touch: bool,
    pointer_owner: Option<PointerId>,
}

impl EditorSession {
    pub fn new(track: TrackId) -> Self {
        Self {
            track, viewport: Viewport::default(), snap: Snap { grid_ticks: 240 },
            selection: BTreeSet::new(), gesture: None, pointer_captured: false,
            autoscroll_active: false, follow_playhead: true, default_note_ticks: 480,
            notice: None, touch: false, pointer_owner: None,
        }
    }

    pub fn gesture_name(&self) -> &'static str {
        match &self.gesture {
            None => "idle",
            Some(g) if g.phase == Phase::Armed => "armed",
            Some(_) => "dragging",
        }
    }

    pub fn pointer_owner(&self) -> Option<PointerId> { self.pointer_owner }

    pub(crate) fn capture_pointer(&mut self, pointer: PointerId, out: &mut Vec<Effect>) {
        if self.pointer_owner.is_none() {
            self.pointer_owner = Some(pointer);
            self.pointer_captured = true;
            out.push(Effect::CapturePointer);
        }
    }

    pub(crate) fn release_pointer(&mut self, out: &mut Vec<Effect>) {
        self.pointer_owner = None;
        if std::mem::take(&mut self.pointer_captured) { out.push(Effect::ReleasePointer); }
    }

    pub fn selection_in_order(&self, project: &Project) -> Vec<NoteId> {
        project.track(&self.track)
            .map(|t| t.notes.iter().filter(|n| self.selection.contains(&n.id)).map(|n| n.id.clone()).collect())
            .unwrap_or_default()
    }

    pub fn on_project_changed(&mut self, project: &Project, revision: Revision) -> Vec<Effect> {
        if let Some(t) = project.track(&self.track) { self.selection.retain(|id| t.note(id).is_some()); }
        if self.gesture.as_ref().is_some_and(|g| g.base_revision != revision) {
            let mut effects = self.handle(project, revision, InputEvent::Cancel(CancelReason::ProjectChanged));
            effects.push(Effect::Notice("工程已被其他操作修改，拖拽已取消（未覆盖新内容）".into()));
            effects
        } else { Vec::new() }
    }

    /// Existing app API. Hosts supporting multiple contacts should supply a
    /// stable identity via `handle_pointer`; an unrelated source cannot move,
    /// end or replace an already captured gesture.
    pub fn handle(&mut self, project: &Project, revision: Revision, event: InputEvent) -> Vec<Effect> {
        let pointer = if self.touch { PointerId::Touch(0) } else { PointerId::Mouse };
        self.handle_pointer(project, revision, pointer, event)
    }

    pub fn handle_pointer(&mut self, project: &Project, revision: Revision, pointer: PointerId, event: InputEvent) -> Vec<Effect> {
        let global_cancel = matches!(event, InputEvent::Cancel(CancelReason::Escape | CancelReason::FocusLost | CancelReason::ProjectChanged));
        if self.pointer_owner.is_some_and(|owner| owner != pointer) && !global_cancel { return Vec::new(); }
        if matches!(event, InputEvent::PointerDown { .. }) && self.gesture.is_some() { return Vec::new(); }
        if let InputEvent::PointerDown { music, screen, .. } | InputEvent::PointerMove { music, screen, .. } = &event {
            if !music.tick.is_finite() || !music.pitch.is_finite() || !screen.0.is_finite() || !screen.1.is_finite() { return Vec::new(); }
        }
        // Double-click text editing/insertion is a separate host primitive;
        // the four drag handlers manage their own starts and transitions.
        if let InputEvent::PointerDown { hit, music, click_count, .. } = &event {
            if project.track(&self.track).is_none() { return Vec::new(); }
            if *click_count >= 2 {
                match hit {
                    Hit::Background => return self.insert_at(revision, *music),
                    Hit::Body(id) if project.track(&self.track).is_some_and(|t| t.note(id).is_some()) => {
                        self.selection = [id.clone()].into();
                        return vec![Effect::BeginLyricEdit(id.clone())];
                    }
                    _ => {}
                }
            }
        }
        crate::bindings::dispatch(self, project, revision, &event, pointer)
    }

    pub(crate) fn extend_selection_to(&mut self, project: &Project, id: &NoteId) {
        let Some(track) = project.track(&self.track) else { return; };
        let Some(target) = track.note_index(id) else { return; };
        let existing: Vec<usize> = track.notes.iter().enumerate().filter(|(_, n)| self.selection.contains(&n.id)).map(|(i, _)| i).collect();
        let (lo, hi) = match (existing.first(), existing.last()) {
            (Some(&a), Some(&b)) => (a.min(target), b.max(target)),
            _ => (target, target),
        };
        for n in &track.notes[lo..=hi] { self.selection.insert(n.id.clone()); }
    }

    fn insert_at(&mut self, revision: Revision, music: MusicPoint) -> Vec<Effect> {
        let start = self.snap.floor(music.tick).max(0);
        let pitch = (music.pitch.floor() as i32).clamp(0, 127);
        let note = Note::new(start, self.default_note_ticks, pitch);
        let id = note.id.clone();
        vec![
            Effect::Commit {
                base: revision,
                batch: CommandBatch::single("插入音符", Command::InsertNotes { track: self.track.clone(), notes: vec![note] }),
                select_after: Some(vec![id.clone()]),
            },
            Effect::BeginLyricEdit(id),
        ]
    }

    /// Committed note projected with the live preview; project truth stays
    /// immutable until the generated handler emits a domain commit effect.
    pub fn preview_note(&self, n: &Note) -> Option<Note> {
        let g = self.gesture.as_ref()?;
        if !g.ids.contains(&n.id) { return None; }
        g.preview.project_note(n)
    }

    // Keyboard commands retain the existing app session API.
    pub fn delete_selection(&mut self, project: &Project, revision: Revision) -> Vec<Effect> {
        let ids = self.selection_in_order(project);
        if ids.is_empty() { return Vec::new(); }
        vec![Effect::Commit {
            base: revision,
            batch: CommandBatch::single(format!("删除 {} 个音符", ids.len()), Command::DeleteNotes { track: self.track.clone(), note_ids: ids }),
            select_after: Some(Vec::new()),
        }]
    }

    pub fn nudge_selection(&mut self, project: &Project, revision: Revision, dt: i64, ds: i32) -> Vec<Effect> {
        let ids = self.selection_in_order(project);
        if ids.is_empty() || self.gesture.is_some() { return Vec::new(); }
        vec![Effect::Commit {
            base: revision,
            batch: CommandBatch::single(format!("移动 {} 个音符", ids.len()), Command::MoveNotes { track: self.track.clone(), note_ids: ids, delta_tick: dt, delta_semitones: ds }),
            select_after: None,
        }]
    }

    pub fn step_selection(&mut self, project: &Project, forward: bool) -> Option<NoteId> {
        let track = project.track(&self.track)?;
        if track.notes.is_empty() { return None; }
        let current = track.notes.iter().position(|n| self.selection.contains(&n.id));
        let idx = match (current, forward) {
            (None, true) => 0,
            (None, false) => track.notes.len() - 1,
            (Some(i), true) => (i + 1).min(track.notes.len() - 1),
            (Some(i), false) => i.saturating_sub(1),
        };
        let note = &track.notes[idx];
        self.selection = [note.id.clone()].into();
        self.viewport.reveal(note);
        Some(note.id.clone())
    }

    pub fn select_all(&mut self, project: &Project) {
        if let Some(t) = project.track(&self.track) { self.selection = t.notes.iter().map(|n| n.id.clone()).collect(); }
    }
}
