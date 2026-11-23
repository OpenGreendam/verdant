//! One executor for Verdant-owned native pointer captures.
//!
//! A hitbox alone is insufficient ownership: a cancelled drag may queue a
//! release and the next drag can capture the same hitbox before it runs. The
//! generation identifies the particular acquisition, so late cleanup cannot
//! release its replacement. GPUI may independently clear capture on MouseUp;
//! that is harmless and does not replace cancellation cleanup.

use std::{cell::Cell, collections::HashMap, rc::Rc};
use gpui::{AnyWindowHandle, App, AppContext, AsyncApp, HitboxId, SharedString, Window, WindowId};

/// Verdant cancels an edit when its pointer leaves the application window.
/// Wayland's implicit grab can keep delivering out-of-window coordinates until
/// release without a MouseExited event. Check every move and terminal sample,
/// in window coordinates: leaving an individual control still permits a drag.
pub(crate) fn contains_window_point(window: &Window, (x, y): (f64, f64)) -> bool {
    let viewport = window.viewport_size();
    x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0
        && x < f64::from(viewport.width) && y < f64::from(viewport.height)
}

/// Identity of one actually rendered native input subtree. It survives
/// redraws, but a detach/remount always gets a different identity.
#[derive(Clone)]
pub(crate) struct SurfaceToken(Rc<Cell<bool>>);

impl SurfaceToken {
    pub(crate) fn is_live(&self) -> bool { self.0.get() }
    pub(crate) fn same(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) }
    #[cfg(test)]
    pub(crate) fn for_test() -> Self { Self(Rc::new(Cell::new(true))) }
}

struct SurfaceLifetime {
    token: SurfaceToken,
    app: AsyncApp,
    detached: Option<Box<dyn FnOnce(SurfaceToken, &mut App)>>,
}

impl Drop for SurfaceLifetime {
    fn drop(&mut self) {
        self.token.0.set(false);
        let token = self.token.clone();
        if let Some(detached) = self.detached.take() {
            self.app.spawn(async move |cx| { cx.update(|cx| detached(token, cx)); }).detach();
        }
    }
}

/// GPUI drops keyed state when a subtree is no longer rendered. Mark it dead
/// synchronously; release editor ownership after draw, where entity mutation is
/// legal. The callback matches this token before cancelling a replacement drag.
pub(crate) fn track_surface(key: SharedString, window: &mut Window, cx: &mut App,
    detached: impl FnOnce(SurfaceToken, &mut App) + 'static) -> SurfaceToken {
    let state = window.use_keyed_state(key, cx, |_, cx| SurfaceLifetime {
        token: SurfaceToken(Rc::new(Cell::new(true))), app: cx.to_async(), detached: Some(Box::new(detached)),
    });
    state.read(cx).token.clone()
}

#[derive(Default)]
struct Captures {
    next: u64,
    active: HashMap<WindowId, u64>,
}
impl gpui::Global for Captures {}

pub(crate) struct OwnedPointerCapture {
    window: AnyWindowHandle,
    hitbox: HitboxId,
    generation: u64,
    app: AsyncApp,
    active: bool,
}

impl OwnedPointerCapture {
    pub(crate) fn window(&self) -> AnyWindowHandle { self.window }
    pub(crate) fn is_current(&self, window: &Window, cx: &App) -> bool {
        self.active && self.window == window.window_handle() && window.captured_hitbox() == Some(self.hitbox)
            && cx.try_global::<Captures>().is_some_and(|captures|
                captures.active.get(&self.window.window_id()) == Some(&self.generation))
    }

    pub(crate) fn acquire(hitbox: HitboxId, window: &mut Window, cx: &mut App) -> Self {
        if cx.try_global::<Captures>().is_none() { cx.set_global(Captures::default()); }
        let handle = window.window_handle();
        let captures = cx.global_mut::<Captures>();
        captures.next = captures.next.checked_add(1).expect("pointer capture generation exhausted");
        let generation = captures.next;
        captures.active.insert(handle.window_id(), generation);
        window.capture_pointer(hitbox);
        Self { window: handle, hitbox, generation, app: cx.to_async(), active: true }
    }

    pub(crate) fn release(mut self, window: &mut Window, cx: &mut App) {
        if self.window.window_id() != window.window_handle().window_id() {
            self.release_deferred(cx);
            return;
        }
        self.active = false;
        let owns = cx.try_global::<Captures>()
            .is_some_and(|captures| captures.active.get(&self.window.window_id()) == Some(&self.generation));
        if owns {
            cx.global_mut::<Captures>().active.remove(&self.window.window_id());
            if window.captured_hitbox() == Some(self.hitbox) { window.release_pointer(); }
        }
    }

    pub(crate) fn release_deferred(mut self, cx: &mut App) {
        self.active = false;
        let handle = self.window;
        let generation = self.generation;
        cx.defer(move |cx| {
            // A closed window no longer has native capture to release.
            if handle.update(cx, |_, window, cx| self.release(window, cx)).is_err() {
                if let Some(captures) = cx.try_global::<Captures>() {
                    if captures.active.get(&handle.window_id()) == Some(&generation) {
                        cx.global_mut::<Captures>().active.remove(&handle.window_id());
                    }
                }
            }
        });
    }
}

impl Drop for OwnedPointerCapture {
    fn drop(&mut self) {
        if !self.active { return; }
        // Dropping an editor/control is a cancellation too. The async handle
        // avoids borrowing an App during entity destruction; generation and
        // hitbox checks are identical to an explicit release.
        let (window, hitbox, generation) = (self.window, self.hitbox, self.generation);
        self.app.spawn(async move |cx| {
            let _ = cx.update(|cx| {
                let owned = cx.try_global::<Captures>()
                    .is_some_and(|captures| captures.active.get(&window.window_id()) == Some(&generation));
                if !owned { return; }
                cx.global_mut::<Captures>().active.remove(&window.window_id());
                let _ = window.update(cx, |_, window, _| {
                    if window.captured_hitbox() == Some(hitbox) { window.release_pointer(); }
                });
            });
        }).detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use gpui::{Context, HitboxBehavior, IntoElement, Render, TestAppContext, WindowHandle, canvas, prelude::*, px, size};

    struct Harness { hitboxes: Rc<Cell<Option<[HitboxId; 2]>>> }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let hitboxes = self.hitboxes.clone();
            canvas(move |bounds, window, _| {
                let a = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                let b = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                hitboxes.set(Some([a.id, b.id]));
            }, |_, _, _, _| {}).size(px(50.0))
        }
    }

    fn open(cx: &mut TestAppContext) -> (WindowHandle<Harness>, [HitboxId; 2]) {
        let hitboxes = Rc::new(Cell::new(None));
        let observed = hitboxes.clone();
        let window = cx.open_window(size(px(100.0), px(100.0)), |_, _| Harness { hitboxes });
        // Drawing reads the root entity; update the window without also
        // leasing Harness through WindowHandle::update.
        cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx)).unwrap();
        cx.run_until_parked();
        (window, observed.get().expect("real prepaint allocated hitboxes"))
    }

    #[gpui::test]
    fn delayed_cancel_cannot_release_a_new_capture_of_the_same_hitbox(cx: &mut TestAppContext) {
        let (window, [hitbox, _]) = open(cx);
        let current = window.update(cx, |_, window, cx| {
            let old = OwnedPointerCapture::acquire(hitbox, window, cx);
            let current = OwnedPointerCapture::acquire(hitbox, window, cx);
            old.release_deferred(cx);
            current
        }).unwrap();
        cx.run_until_parked();
        window.update(cx, |_, window, cx| {
            assert_eq!(window.captured_hitbox(), Some(current.hitbox));
            current.release(window, cx);
            assert!(window.captured_hitbox().is_none());
        }).unwrap();
    }

    #[gpui::test]
    fn cancellation_releases_before_mouse_up_and_preserves_foreign_capture(cx: &mut TestAppContext) {
        let (window, [a, b]) = open(cx);
        window.update(cx, |_, window, cx| {
            let capture = OwnedPointerCapture::acquire(a, window, cx);
            capture.release(window, cx);
            assert!(window.captured_hitbox().is_none());
            let capture = OwnedPointerCapture::acquire(a, window, cx);
            window.capture_pointer(b);
            capture.release(window, cx);
            assert_eq!(window.captured_hitbox(), Some(b));
            window.release_pointer();
        }).unwrap();
    }

    #[gpui::test]
    fn dropping_an_owner_releases_its_native_capture(cx: &mut TestAppContext) {
        let (window, [hitbox, _]) = open(cx);
        let capture = window.update(cx, |_, window, cx| OwnedPointerCapture::acquire(hitbox, window, cx)).unwrap();
        drop(capture);
        cx.run_until_parked();
        window.update(cx, |_, window, _| assert!(window.captured_hitbox().is_none())).unwrap();
    }
}
