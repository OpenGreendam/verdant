//! Compiled Proto UI roll root and its native pitch-stroke binding.

use std::{cell::Cell, rc::Rc};

use gpui::{App, AppContext, AsyncApp, Context, Entity, IntoElement, Render, WeakEntity, Window};
use verdant_pitch_gesture_surface::{
    PitchGestureSurface,
    native::{
        gpui_native_move::{
            MoveGestureBinding, MoveGestureCancelContext, MoveGestureCancelReason,
            MoveGestureInput, MoveGestureLease, MoveGestureSample, attach_move_gesture,
        },
        gpui_native_owner::NativeSnapshot,
    },
};
use verdant_session::PointerId;

use crate::editor::{Editor, Tool};

pub(crate) struct PitchMove {
    lease: Rc<MoveGestureLease>,
    started: Rc<Cell<Option<PointerId>>>,
    generation: Rc<Cell<u64>>,
    window: gpui::AnyWindowHandle,
}

impl PitchMove {
    pub(crate) fn cancel(&self, window: &mut Window, cx: &mut App) {
        // The caller already owns the editor and clears its preview itself.
        self.started.set(None);
        self.lease.cancel(MoveGestureCancelReason::HostCancel, window, cx);
    }

    pub(crate) fn cancel_deferred(&self, cx: &mut App) {
        self.started.set(None);
        let generation = self.generation.clone();
        let expected = generation.get();
        let lease = self.lease.clone();
        let handle = self.window;
        cx.defer(move |cx| {
            if generation.get() == expected {
                let _ = handle.update(cx, |_, window, cx| lease.cancel(MoveGestureCancelReason::HostCancel, window, cx));
            }
        });
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
    let started = Rc::new(Cell::new(None));
    let generation = Rc::new(Cell::new(0));
    let target = root.read(cx).handle().owner();
    let binding = RollMove { editor, app: cx.to_async(), started: started.clone(), generation: generation.clone() };
    let lease = attach_move_gesture(target, Box::new(binding))
        .unwrap_or_else(|error| panic!("compiled pitch surface cannot attach its pointer binding: {error}"));
    (root, PitchMove { lease: Rc::new(lease), started, generation, window: window.window_handle() })
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
    started: Rc<Cell<Option<PointerId>>>,
    generation: Rc<Cell<u64>>,
}

fn sample_pointer(input: MoveGestureInput) -> Option<PointerId> {
    // This GPUI version exposes one legacy touch stream, not contact IDs.
    // Do not silently turn touch/unknown input into a mouse gesture.
    match input {
        MoveGestureInput::Mouse => Some(PointerId::Mouse),
        MoveGestureInput::Touch => Some(PointerId::Touch(0)),
        MoveGestureInput::Pen | MoveGestureInput::Unknown => None,
    }
}

impl RollMove {
    fn sample(editor: &mut Editor, sample: &MoveGestureSample) {
        let x = f64::from(sample.position.x);
        let y = f64::from(sample.position.y);
        let viewport = editor.session.viewport;
        let Some(pointer) = sample_pointer(sample.input) else { return };
        editor.roll_pitch_sample(
            pointer,
            viewport.x_to_tick(x),
            viewport.y_to_pitch(y),
            (x, y),
            sample.modifiers.shift,
        );
    }

    fn cancel_preview(editor: &WeakEntity<Editor>, pointer: PointerId, cx: &mut App) {
        let _ = editor.update(cx, |editor, cx| {
            if editor.lane_stroke.as_ref().is_some_and(|stroke| stroke.roll && stroke.pointer == pointer) {
                editor.lane_stroke = None;
                cx.notify();
            }
        });
    }
}

impl MoveGestureBinding for RollMove {
    fn should_start(&self, sample: &MoveGestureSample, _: &mut Window, cx: &mut App) -> bool {
        sample_pointer(sample.input).is_some() && self.editor.upgrade().is_some_and(|entity| {
            let editor = entity.read(cx);
            editor.tool == Tool::Pitch && editor.lane_stroke.is_none()
                && editor.session.pointer_owner().is_none() && editor.note_creation.is_none()
                && editor.device_drag.is_none() && editor.mixer_drag.is_none() && editor.menu.is_none()
                && editor.text_prompt.is_none() && editor.modal_dialog.is_none()
        })
    }

    fn on_start(&self, sample: &MoveGestureSample, window: &mut Window, cx: &mut App) {
        let Some(pointer) = sample_pointer(sample.input) else { return };
        let started = self.editor.update(cx, |editor, cx| {
            crate::roll::begin_pitch_stroke(
                editor, pointer, sample.position.x, sample.position.y, window, cx,
            )
        }).unwrap_or(false);
        // Beginning may finish a lyric transaction and queue cleanup of an
        // older gesture. This accepted session must supersede that cleanup.
        self.generation.set(self.generation.get().wrapping_add(1));
        self.started.set(started.then_some(pointer));
    }

    fn on_move(&self, sample: &MoveGestureSample, _: &mut Window, cx: &mut App) {
        if self.started.get().is_some() && self.started.get() == sample_pointer(sample.input) {
            let _ = self.editor.update(cx, |editor, cx| {
                Self::sample(editor, sample);
                cx.notify();
            });
        }
    }

    fn on_end(&self, sample: &MoveGestureSample, _: &mut Window, cx: &mut App) {
        let Some(pointer) = sample_pointer(sample.input) else { return };
        if self.started.get() == Some(pointer) {
            self.started.set(None);
            let _ = self.editor.update(cx, |editor, cx| {
                if editor.lane_stroke.as_ref().is_some_and(|stroke| stroke.roll && stroke.pointer == pointer) {
                    Self::sample(editor, sample);
                    editor.commit_roll_pitch(cx);
                }
            });
        }
    }

    fn on_cancel(&self, _: MoveGestureCancelReason, context: Option<MoveGestureCancelContext<'_>>) {
        let Some(pointer) = self.started.replace(None) else { return };
        if let Some(context) = context {
            Self::cancel_preview(&self.editor, pointer, context.cx);
        } else {
            let editor = self.editor.clone();
            let generation = self.generation.clone();
            let expected = generation.get();
            self.app.spawn(async move |cx| {
                if generation.get() == expected {
                    let _ = cx.update(|cx| Self::cancel_preview(&editor, pointer, cx));
                }
            }).detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_pitch_binding_preserves_input_identity() {
        assert_eq!(sample_pointer(MoveGestureInput::Mouse), Some(PointerId::Mouse));
        assert_eq!(sample_pointer(MoveGestureInput::Touch), Some(PointerId::Touch(0)));
        assert_eq!(sample_pointer(MoveGestureInput::Unknown), None);
        assert_eq!(sample_pointer(MoveGestureInput::Pen), None);
    }
}
