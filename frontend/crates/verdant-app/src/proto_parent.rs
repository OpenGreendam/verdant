//! Explicit logical composition for compiled roots inside native slots.
//!
//! `NativeSnapshot::native_view` preserves the host view, but deliberately
//! cannot infer a Proto parent from arbitrary GPUI content. The slot owner
//! supplies that identity for layout/prepaint/paint; each compiled child binds
//! it before rendering. This carries topology only, never input policy.
use std::{cell::RefCell, rc::Rc};
use gpui::{AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, Window};
use verdant_action_control::native::gpui_native_owner::{OwnerRef, OwnerWeak};

thread_local! { static CURRENT: RefCell<Option<OwnerWeak>> = const { RefCell::new(None) }; }

pub(crate) fn current() -> Option<OwnerWeak> { CURRENT.with(|current| current.borrow().clone()).filter(|parent| parent.strong_count() > 0) }
pub(crate) fn changed(owner: &OwnerRef, parent: &Option<OwnerWeak>) -> bool {
    !owner.borrow().parent.ptr_eq(&parent.clone().unwrap_or_default())
}
fn within<T>(parent: &OwnerWeak, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<OwnerWeak>);
    impl Drop for Restore { fn drop(&mut self) { CURRENT.with(|current| *current.borrow_mut() = self.0.take()); } }
    let previous = CURRENT.with(|current| current.replace(Some(parent.clone())));
    let _restore = Restore(previous);
    run()
}

pub(crate) fn slot(owner: &OwnerRef, element: AnyElement) -> AnyElement {
    Slot { parent: Rc::downgrade(owner), element }.into_any_element()
}
struct Slot { parent: OwnerWeak, element: AnyElement }
impl Element for Slot {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }
    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (within(&self.parent, || self.element.request_layout(window, cx)), ())
    }
    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) {
        within(&self.parent, || self.element.prepaint(window, cx));
    }
    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), _: &mut (), window: &mut Window, cx: &mut App) {
        within(&self.parent, || self.element.paint(window, cx));
    }
}
