//! Retained native document slot inside a compiled ScrollSurface and scope.
use std::rc::Rc;
use gpui::{prelude::*, App, Context, ElementId, Entity, FocusHandle, Focusable, IntoElement, Subscription, Window};
use serde_json::Value;
use verdant_scroll_region::{ScrollRegion, ScrollRegionHandle};
use verdant_scroll_region::native::gpui_native_owner::{
    attach_scroll_surface, use_builtin_scroll_surface, NativeScrollLease, NativeSnapshot,
};
/// Retain this alongside a native ScrollHandle or virtual list's base_handle.
/// Its identity changes only when that physical surface is replaced.
pub(crate) use verdant_scroll_region::native::gpui_native_owner::NativeScrollTarget as NativeScroll;

pub(crate) type Draw = Rc<dyn Fn(&ScrollRegionHandle, &FocusHandle, &mut Window, &mut App) -> gpui::AnyElement>;
#[derive(IntoElement)]
pub(crate) struct Region { id: ElementId, props: Value, draw: Draw, native_scroll: Option<NativeScroll> }
pub(crate) fn region(id: impl Into<ElementId>, props: Value, draw: Draw) -> Region { Region { id: id.into(), props, draw, native_scroll: None } }
/// The closure rebuilds native document content on every retained-view draw;
/// the compiled surface owns scrolling and focus policy around it.
pub(crate) fn content(id: impl Into<ElementId>, label: impl Into<String>,
    draw: impl Fn(&mut Window, &mut App) -> gpui::AnyElement + 'static) -> Region {
    region(id, serde_json::json!({"label": label.into()}), Rc::new(move |_, _, window, cx| draw(window, cx)))
}
type NativeDraw = Rc<dyn Fn(&gpui::ScrollHandle, &mut Window, &mut App) -> gpui::AnyElement>;
#[derive(IntoElement)]
pub(crate) struct NativeContent { id: ElementId, label: String, axes: &'static str, tab_stop: bool, draw: NativeDraw }
/// Preserve native layout/overflow while exposing its exact physical scroll
/// handle to the compiled surface. Content is rebuilt, never replayed once.
pub(crate) fn native_content(id: impl Into<ElementId>, label: impl Into<String>,
    draw: impl Fn(&gpui::ScrollHandle, &mut Window, &mut App) -> gpui::AnyElement + 'static) -> NativeContent {
    NativeContent { id: id.into(), label: label.into(), axes: "vertical", tab_stop: true, draw: Rc::new(draw) }
}
impl NativeContent {
    pub(crate) fn tab_stop(mut self, tab_stop: bool) -> Self { self.tab_stop = tab_stop; self }
    pub(crate) fn axes(mut self, axes: &'static str) -> Self {
        assert!(matches!(axes, "vertical" | "horizontal" | "both")); self.axes = axes; self
    }
}
struct NativeContentState { handle: gpui::ScrollHandle, target: NativeScroll }
impl RenderOnce for NativeContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A separate scope keeps physical target state distinct from Region's
        // logical component state even when several lists share a parent.
        let state = window.with_element_namespace(self.id.clone(), |window| {
            window.use_keyed_state("physical-scroll-target", cx, |_, _| {
                let handle = gpui::ScrollHandle::default();
                NativeContentState { target: NativeScroll::new(handle.clone()), handle }
            })
        });
        let physical = state.read(cx); let handle = physical.handle.clone(); let target = physical.target.clone();
        content(self.id, self.label, move |window, cx| (self.draw)(&handle, window, cx))
            .axes(self.axes).tab_stop(self.tab_stop).native_scroll(target)
    }
}
impl Region {
    /// Compound collections/forms already provide their own keyboard target.
    pub(crate) fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.props["tabStop"] = tab_stop.into(); self
    }
    /// Native content retains virtualization, layout and item-reveal mechanics;
    /// all ScrollSurface requests and facts address this exact same handle.
    pub(crate) fn native_scroll(mut self, target: NativeScroll) -> Self {
        self.native_scroll = Some(target); self
    }
    pub(crate) fn axes(mut self, axes: &'static str) -> Self {
        assert!(matches!(axes, "vertical" | "horizontal" | "both"));
        self.props["axes"] = axes.into(); self
    }
}
struct Drawing { handle: ScrollRegionHandle, focus: FocusHandle, draw: Draw }
impl Render for Drawing {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement { crate::proto_parent::slot(&self.handle.owner, (self.draw)(&self.handle, &self.focus, window, cx)) }
}
struct Mount { root: Entity<ScrollRegion>, drawing: Entity<Drawing>, props: Value,
    native_scroll: Option<NativeScroll>, scroll_lease: Option<NativeScrollLease>, _changes: Subscription }
impl RenderOnce for Region {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let parent = crate::proto_parent::current(); let initial_parent = parent.clone();
        let props = self.props.clone(); let draw = self.draw.clone(); let native_scroll = self.native_scroll.clone();
        let mount = window.use_keyed_state(self.id, cx, move |window, cx| {
            let root = ScrollRegion::create(props.clone(), initial_parent, window, cx).unwrap_or_else(|error| panic!("compiled scroll region: {error:?}"));
            let handle = root.read(cx).handle(); let focus = root.read(cx).focus_handle(cx);
            let scroll_lease = native_scroll.clone().map(|target|
                attach_scroll_surface(&Rc::downgrade(&handle.owner), target, window)
                    .unwrap_or_else(|error| panic!("compiled native scroll binding: {error}")));
            let drawing = cx.new(|_| Drawing { handle, focus, draw });
            root.update(cx, |root, cx| root.set_slot(vec![NativeSnapshot::native_view(drawing.clone().into())], cx))
                .unwrap_or_else(|error| panic!("compiled scroll slot: {error:?}"));
            let changes = cx.observe(&root, |mount: &mut Mount, _, cx| {
                let failures = mount.root.read(cx).handle().take_failures();
                if !failures.is_empty() { eprintln!("compiled scroll failed: {failures:?}"); }
                cx.notify();
            });
            Mount { root, drawing, props, native_scroll, scroll_lease, _changes: changes }
        });
        mount.update(cx, |mount, cx| {
            let same_target = match (&mount.native_scroll, &self.native_scroll) {
                (Some(before), Some(after)) => before.same_target(after),
                (None, None) => true,
                _ => false,
            };
            if !same_target || (self.native_scroll.is_some() &&
                !mount.scroll_lease.as_ref().is_some_and(NativeScrollLease::is_current)) {
                // Release the old physical lease before acquiring the next;
                // its queued end movement cannot leak into the replacement.
                mount.scroll_lease = None;
                let owner = Rc::downgrade(&mount.root.read(cx).handle().owner);
                mount.scroll_lease = self.native_scroll.clone().map(|target|
                    attach_scroll_surface(&owner, target, window)
                        .unwrap_or_else(|error| panic!("compiled native scroll binding: {error}")));
                if self.native_scroll.is_none() {
                    use_builtin_scroll_surface(&owner)
                        .unwrap_or_else(|error| panic!("compiled native scroll reset: {error}"));
                }
                mount.native_scroll = self.native_scroll;
            }
            if crate::proto_parent::changed(&mount.root.read(cx).handle().owner, &parent) {
                mount.root.update(cx, |root, cx| root.set_parent(parent, cx)).unwrap_or_else(|error| panic!("compiled container parent: {error:?}"));
            }
            if mount.props != self.props {
                mount.root.update(cx, |root, cx| root.set_props(self.props.clone(), window, cx)).unwrap_or_else(|error| panic!("compiled scroll props: {error:?}"));
                mount.props = self.props;
            }
            mount.drawing.update(cx, |drawing, cx| { drawing.draw = self.draw; cx.notify(); });
            mount.root.clone()
        })
    }
}
