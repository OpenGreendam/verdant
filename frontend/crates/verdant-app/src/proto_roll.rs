//! Compiled Proto UI roll root and its native pitch-stroke binding.

use std::{cell::Cell, rc::Rc};

use gpui::{App, AppContext, AsyncApp, Context, Entity, IntoElement, Render, WeakEntity, Window};
use verdant_pitch_gesture_surface::{
    PitchGestureSurface,
    native::{
        gpui_native_move::{
            MoveGestureBinding, MoveGestureCancelContext, MoveGestureCancelReason,
            MoveGestureLease, MoveGestureSample, attach_move_gesture,
        },
        gpui_native_owner::NativeSnapshot,
    },
};
use verdant_session::PointerId;

use crate::editor::{Editor, Tool};

pub(crate) struct PitchMove {
    lease: MoveGestureLease,
    started: Rc<Cell<bool>>,
}

impl PitchMove {
    pub(crate) fn cancel(&self, window: &mut Window, cx: &mut App) {
        // The caller already owns the editor and clears its preview itself.
        self.started.set(false);
        self.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
    }
}

pub(crate) fn mount(
    editor: WeakEntity<Editor>,
    window: &mut Window,
    cx: &mut App,
) -> (Entity<PitchGestureSurface>, PitchMove) {
    let view = cx.new(|_| NativeRoll { editor: editor.clone() });
    let root = PitchGestureSurface::create(serde_json::json!({}), None, window, cx)
        .unwrap_or_else(|error| panic!("compiled pitch surface setup failed: {error:?}"));
    root.update(cx, |root, cx| {
        root.set_slot(vec![NativeSnapshot::native_view(view.into())], cx)
            .unwrap_or_else(|error| panic!("compiled pitch surface rejected its drawing view: {error:?}"));
    });
    let started = Rc::new(Cell::new(false));
    let target = root.read(cx).handle().owner();
    let binding = RollMove { editor, app: cx.to_async(), started: started.clone() };
    let lease = attach_move_gesture(target, Box::new(binding))
        .unwrap_or_else(|error| panic!("compiled pitch surface cannot attach its pointer binding: {error}"));
    (root, PitchMove { lease, started })
}

struct NativeRoll {
    editor: WeakEntity<Editor>,
}

impl Render for NativeRoll {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        match self.editor.upgrade() {
            Some(editor) => crate::roll::piano_roll(editor).into_any_element(),
            None => gpui::div().into_any_element(),
        }
    }
}

struct RollMove {
    editor: WeakEntity<Editor>,
    app: AsyncApp,
    started: Rc<Cell<bool>>,
}

impl RollMove {
    fn sample(editor: &mut Editor, sample: &MoveGestureSample) {
        let x = f64::from(sample.position.x);
        let y = f64::from(sample.position.y);
        let viewport = editor.session.viewport;
        editor.roll_pitch_sample(
            PointerId::Mouse,
            viewport.x_to_tick(x),
            viewport.y_to_pitch(y),
            (x, y),
            sample.modifiers.shift,
        );
    }

    fn cancel_preview(editor: &WeakEntity<Editor>, cx: &mut App) {
        let _ = editor.update(cx, |editor, cx| {
            if editor.lane_stroke.as_ref().is_some_and(|stroke| stroke.roll && stroke.pointer == PointerId::Mouse) {
                editor.lane_stroke = None;
                cx.notify();
            }
        });
    }
}

impl MoveGestureBinding for RollMove {
    fn should_start(&self, _: &MoveGestureSample, _: &mut Window, cx: &mut App) -> bool {
        self.editor.upgrade().is_some_and(|entity| {
            let editor = entity.read(cx);
            editor.tool == Tool::Pitch && editor.lane_stroke.is_none()
                && editor.session.pointer_owner().is_none() && editor.note_creation.is_none()
                && editor.mixer_drag.is_none() && editor.menu.is_none()
                && editor.text_prompt.is_none() && editor.modal_dialog.is_none()
        })
    }

    fn on_start(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        let started = self.editor.update(cx, |editor, cx| {
            crate::roll::begin_pitch_stroke(
                editor, PointerId::Mouse, f64::from(sample.position.x), f64::from(sample.position.y), window, cx,
            )
        }).unwrap_or(false);
        self.started.set(started);
    }

    fn on_move(&self, sample: &MoveGestureSample, _: &mut Window, cx: &mut App) {
        if self.started.get() {
            let _ = self.editor.update(cx, |editor, cx| {
                Self::sample(editor, sample);
                cx.notify();
            });
        }
    }

    fn on_end(&self, sample: &MoveGestureSample, _: &mut Window, cx: &mut App) {
        if self.started.replace(false) {
            let _ = self.editor.update(cx, |editor, cx| {
                if editor.lane_stroke.as_ref().is_some_and(|stroke| stroke.roll && stroke.pointer == PointerId::Mouse) {
                    Self::sample(editor, sample);
                    editor.commit_roll_pitch(cx);
                }
            });
        }
    }

    fn on_cancel(&self, _: MoveGestureCancelReason, context: Option<MoveGestureCancelContext<'_>>) {
        if !self.started.replace(false) { return; }
        if let Some(context) = context {
            Self::cancel_preview(&self.editor, context.cx);
        } else {
            let editor = self.editor.clone();
            let started = self.started.clone();
            self.app.spawn(async move |cx| {
                if !started.get() {
                    let _ = cx.update(|cx| Self::cancel_preview(&editor, cx));
                }
            }).detach();
        }
    }
}
