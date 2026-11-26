//! The toolbar doubles as the window title bar ("unified title bar").
//!
//! - macOS: the native title bar is transparent; the traffic lights sit inside
//!   the toolbar and the app drives dragging and double-click zoom.
//! - Windows: the caption is hidden; Verdant draws minimise / maximise / close
//!   and marks them, and the empty toolbar areas, as native hit-test regions
//!   (so snap layouts and system drag still work).
//! - Linux: client-side decorations are requested. When the compositor grants
//!   them, Verdant draws the buttons the window manager supports and frames the
//!   window like a libadwaita one: 12 px rounded corners, a 1 px border and a
//!   soft shadow in a transparent margin that also holds the resize zones.
//!   Maximised, fullscreen and tiled edges are square and unpadded. Otherwise
//!   the system title bar stays.
//! - Windows 11 rounds the window itself; Verdant asks DWM for the rounded
//!   preference explicitly so the custom caption keeps it. macOS rounds titled
//!   windows natively.
//!
//! `VERDANT_WINDOW_DECORATIONS=server` keeps the system title bar everywhere.
use gpui::{
    AnyElement, Bounds, Context, CursorStyle, Decorations, DispatchPhase, HitboxBehavior, IntoElement, MouseButton,
    MouseDownEvent, Pixels, Point, ResizeEdge, SharedString, Tiling, TitlebarOptions, Window, WindowControlArea,
    WindowOptions, canvas, div, point, prelude::*, px, svg,
};

use crate::assets::icon;
use crate::editor::Editor;

/// Room for the macOS traffic lights at the start of the toolbar.
pub const TRAFFIC_LIGHT_PADDING: f32 = 78.0;
/// Width of the edge band, inside the window content, that resizes a
/// client-decorated window (in addition to the shadow margin).
const RESIZE_BAND: f32 = 5.0;
/// libadwaita's window corner radius.
pub const CORNER_RADIUS: f32 = 12.0;
/// Transparent margin around a floating client-decorated window for its shadow.
const SHADOW_MARGIN: f32 = 12.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChromeStyle {
    /// The system draws the title bar.
    #[default]
    Native,
    Mac,
    Windows,
    LinuxClient,
}

/// What the toolbar needs to know to act as the title bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChromeState {
    pub style: ChromeStyle,
    pub maximized: bool,
    pub fullscreen: bool,
    pub can_minimize: bool,
    pub can_maximize: bool,
    pub tiling: Tiling,
}

impl ChromeState {
    pub fn read(window: &Window) -> Self {
        let style = if !custom_chrome_requested() {
            ChromeStyle::Native
        } else if cfg!(target_os = "macos") {
            ChromeStyle::Mac
        } else if cfg!(target_os = "windows") {
            ChromeStyle::Windows
        } else {
            match window.window_decorations() {
                Decorations::Client { .. } => ChromeStyle::LinuxClient,
                Decorations::Server => ChromeStyle::Native,
            }
        };
        let tiling = match window.window_decorations() {
            Decorations::Client { tiling } => tiling,
            Decorations::Server => Tiling::default(),
        };
        let controls = window.window_controls();
        let resizable = window.is_resizable();
        Self {
            style,
            maximized: window.is_maximized(),
            fullscreen: window.is_fullscreen(),
            can_minimize: window.is_minimizable() && (style != ChromeStyle::LinuxClient || controls.minimize),
            can_maximize: resizable && (style != ChromeStyle::LinuxClient || controls.maximize),
            tiling,
        }
    }

    /// Verdant draws minimise / maximise / close itself.
    pub fn draws_caption_buttons(&self) -> bool {
        matches!(self.style, ChromeStyle::Windows | ChromeStyle::LinuxClient) && !self.fullscreen
    }

    /// Leading space kept free for the macOS traffic lights.
    pub fn leading_inset(&self) -> f32 {
        if self.style == ChromeStyle::Mac && !self.fullscreen { TRAFFIC_LIGHT_PADDING } else { 0.0 }
    }
}

fn custom_chrome_requested() -> bool {
    !matches!(std::env::var("VERDANT_WINDOW_DECORATIONS").as_deref(), Ok("server" | "native" | "system"))
}

/// Configure the main editor window for the unified title bar.
pub fn apply(options: &mut WindowOptions, toolbar_height: f32) {
    if !custom_chrome_requested() {
        return;
    }
    options.titlebar = Some(TitlebarOptions {
        title: Some("Verdant 翠绿".into()),
        appears_transparent: true,
        // Centre the 12 px traffic lights in the toolbar.
        traffic_light_position: Some(point(px(14.0), px((toolbar_height - 12.0) / 2.0))),
    });
    options.app_owns_titlebar_drag = true;
    if cfg!(any(target_os = "linux", target_os = "freebsd")) {
        options.window_decorations = Some(gpui::WindowDecorations::Client);
    }
}

/// An empty stretch of the toolbar that behaves like a title bar: drag to
/// move, double-click to maximise / zoom, right-click for the window menu.
/// The press is recorded here; [`track_title_drag`] on the window root turns
/// the first movement into a window move (the pointer may already have left
/// this narrow area by then).
pub fn drag_region(id: impl Into<SharedString>, chrome: ChromeState, cx: &mut Context<Editor>) -> gpui::Stateful<gpui::Div> {
    let region = div().id(gpui::ElementId::Name(id.into())).flex_1().min_w(px(8.0)).h_full();
    if chrome.style == ChromeStyle::Native {
        return region;
    }
    let region = region.window_control_area(WindowControlArea::Drag);
    if chrome.style == ChromeStyle::Windows {
        return region;
    }
    region
        .on_mouse_down(MouseButton::Left, cx.listener(|ed, ev: &MouseDownEvent, window, _| {
            if ev.click_count >= 2 {
                ed.title_drag = false;
                if cfg!(target_os = "macos") { window.titlebar_double_click() } else { window.zoom_window() }
            } else {
                ed.title_drag = true;
            }
        }))
        .when(chrome.style == ChromeStyle::LinuxClient, |d| d.on_mouse_down(MouseButton::Right, |ev, window, _| {
            window.show_window_menu(ev.position);
        }))
}

/// Root-level listeners that finish a title-bar press started in a drag region.
pub fn track_title_drag(root: gpui::Stateful<gpui::Div>, cx: &mut Context<Editor>) -> gpui::Stateful<gpui::Div> {
    root.on_mouse_move(cx.listener(|ed, ev: &gpui::MouseMoveEvent, window, _| {
        if !ed.title_drag { return; }
        ed.title_drag = false;
        if ev.pressed_button == Some(MouseButton::Left) { window.start_window_move(); }
    }))
    .on_mouse_up(MouseButton::Left, cx.listener(|ed, _, _, _| ed.title_drag = false))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Caption { Minimize, Maximize, Close }

/// Minimise / maximise (restore) / close, full toolbar height, at the far end.
pub fn caption_buttons(chrome: ChromeState, height: f32, th: crate::theme::Theme, cx: &mut Context<Editor>) -> Option<AnyElement> {
    if !chrome.draws_caption_buttons() {
        return None;
    }
    let windows = chrome.style == ChromeStyle::Windows;
    let mut row = div().id("window-controls").flex().flex_none().h(px(height)).ml(px(4.0))
        // Windows must receive the non-client mouse-down to arm the caption button.
        .when(!windows, |d| d.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()));
    for caption in [Caption::Minimize, Caption::Maximize, Caption::Close] {
        let enabled = match caption {
            Caption::Minimize => chrome.can_minimize,
            Caption::Maximize => chrome.can_maximize,
            Caption::Close => true,
        };
        if !enabled && !windows { continue; }
        let (id, name, label, area) = match caption {
            Caption::Minimize => ("window-minimize", if windows { "minus" } else { "window-minimize" }, "最小化", WindowControlArea::Min),
            Caption::Maximize if chrome.maximized => ("window-maximize", if windows { "copy" } else { "window-restore" }, "还原", WindowControlArea::Max),
            Caption::Maximize => ("window-maximize", if windows { "square" } else { "window-maximize" }, "最大化", WindowControlArea::Max),
            Caption::Close => ("window-close", if windows { "x" } else { "window-close" }, "关闭", WindowControlArea::Close),
        };
        let close = caption == Caption::Close;
        let fg = if enabled { th.text } else { th.text_tertiary };
        let mut button = if windows {
            // Windows keeps its own caption layout: 46 px cells, red close.
            let hover = if close { gpui::rgb(0xc42b1c) } else { th.control_pressed };
            crate::proto_controls::action(id, label).disabled(!enabled)
                .w(px(46.0)).h_full().flex().items_center().justify_center().occlude()
                .child(svg().path(icon(name)).size(px(if caption == Caption::Maximize { 12.0 } else { 15.0 })).text_color(fg))
                .when(enabled, |b| b.hover(move |s| s.bg(hover)))
        } else {
            // GNOME: a 24 px round button with a faint fill, darker on hover
            // and pressed, inside a larger hit area.
            let base = crate::theme::with_alpha(th.text, 0.10);
            let (hover, pressed) = (crate::theme::with_alpha(th.text, 0.16), crate::theme::with_alpha(th.text, 0.26));
            crate::proto_controls::action(id, label).disabled(!enabled)
                .w(px(34.0)).h_full().flex().items_center().justify_center().occlude().cursor_pointer().group(id)
                .child(div().id(SharedString::from(format!("{id}-disc"))).size(px(24.0)).rounded(px(12.0)).flex().items_center().justify_center().bg(base)
                    .group_hover(id, move |s| s.bg(hover)).group_active(id, move |s| s.bg(pressed))
                    .child(svg().path(icon(name)).size(px(16.0)).text_color(fg)))
        };
        if windows {
            // Native hit-testing performs the action (and shows snap layouts).
            if enabled {
                button = button.window_control_area(area);
            }
        }
        // Windows retains non-client mouse hit testing (including snap layouts).
        // GPUI keyboard and assistive routes, and every non-Windows click, pass
        // through the compiled action before this native window command.
        button = button.on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, window, cx| {
            if windows && event.is_mouse() { return; }
            match caption {
                Caption::Minimize => window.minimize_window(),
                Caption::Maximize => window.zoom_window(),
                Caption::Close => ed.close_window(window, cx),
            }
        }));
        row = row.child(button);
    }
    Some(row.into_any_element())
}

fn resize_edge(pos: Point<Pixels>, size: gpui::Size<Pixels>, tiling: Tiling, margin: f32) -> Option<ResizeEdge> {
    let band = px(margin + RESIZE_BAND);
    let corner = px(margin + CORNER_RADIUS.max(RESIZE_BAND * 3.0));
    let (left, right) = (pos.x < band && !tiling.left, pos.x > size.width - band && !tiling.right);
    let (top, bottom) = (pos.y < band && !tiling.top, pos.y > size.height - band && !tiling.bottom);
    let near_left = pos.x < corner && !tiling.left;
    let near_right = pos.x > size.width - corner && !tiling.right;
    let near_top = pos.y < corner && !tiling.top;
    let near_bottom = pos.y > size.height - corner && !tiling.bottom;
    Some(match () {
        _ if (top && near_left) || (left && near_top) => ResizeEdge::TopLeft,
        _ if (top && near_right) || (right && near_top) => ResizeEdge::TopRight,
        _ if (bottom && near_left) || (left && near_bottom) => ResizeEdge::BottomLeft,
        _ if (bottom && near_right) || (right && near_bottom) => ResizeEdge::BottomRight,
        _ if top => ResizeEdge::Top,
        _ if bottom => ResizeEdge::Bottom,
        _ if left => ResizeEdge::Left,
        _ if right => ResizeEdge::Right,
        _ => return None,
    })
}

/// The rounded corners of a floating Linux client-decorated window; square
/// elsewhere, and on tiled, maximised or fullscreen sides. Panes that paint
/// into a window corner apply the matching radius to their own background,
/// because GPUI's content masks are rectangular.
pub fn corner_radii(chrome: ChromeState) -> gpui::Corners<Pixels> {
    let floating = chrome.style == ChromeStyle::LinuxClient && !chrome.maximized && !chrome.fullscreen;
    let tiling = chrome.tiling;
    let radius = |a: bool, b: bool| px(if floating && !a && !b { CORNER_RADIUS } else { 0.0 });
    gpui::Corners {
        top_left: radius(tiling.top, tiling.left),
        top_right: radius(tiling.top, tiling.right),
        bottom_left: radius(tiling.bottom, tiling.left),
        bottom_right: radius(tiling.bottom, tiling.right),
    }
}

/// Frames the editor's root element for the current chrome.
///
/// Linux client decorations get libadwaita's look: the content is clipped to
/// rounded corners with a 1 px border and sits in a transparent margin that
/// carries the shadow and the resize zones. Sides that are tiled, maximised or
/// fullscreen stay square and flush. Other chromes return the root unchanged.
pub fn frame_window(root: gpui::Stateful<gpui::Div>, chrome: ChromeState, th: crate::theme::Theme, window: &mut Window) -> AnyElement {
    if chrome.style == ChromeStyle::Windows {
        prefer_rounded_corners(window);
    }
    if chrome.style != ChromeStyle::LinuxClient {
        window.set_client_inset(px(0.0));
        return root.into_any_element();
    }
    let floating = !chrome.maximized && !chrome.fullscreen;
    let tiling = if floating { chrome.tiling } else { Tiling { top: true, bottom: true, left: true, right: true } };
    let margin = if floating { SHADOW_MARGIN } else { 0.0 };
    window.set_client_inset(px(margin));
    let radii = corner_radii(chrome);
    window.set_client_corner_radii(radii);
    let resizable = floating;
    div().id("verdant-window-frame").size_full().relative()
        .when(!tiling.top, |d| d.pt(px(margin))).when(!tiling.bottom, |d| d.pb(px(margin)))
        .when(!tiling.left, |d| d.pl(px(margin))).when(!tiling.right, |d| d.pr(px(margin)))
        // GPUI clips to rectangles, so the panes that touch a corner round
        // themselves (see `corner_radii`); this layer only adds the shadow
        // behind and the border on top.
        .child(div().size_full().relative()
            .rounded_tl(radii.top_left).rounded_tr(radii.top_right)
            .rounded_bl(radii.bottom_left).rounded_br(radii.bottom_right)
            .when(floating, |d| d.shadow(vec![
                gpui::BoxShadow { color: gpui::hsla(0., 0., 0., 0.28), offset: point(px(0.), px(2.)), blur_radius: px(margin - 2.), spread_radius: px(0.), inset: false },
                gpui::BoxShadow { color: gpui::hsla(0., 0., 0., 0.14), offset: point(px(0.), px(0.)), blur_radius: px(2.), spread_radius: px(1.), inset: false },
            ]))
            .child(root.size_full()
                .rounded_tl(radii.top_left).rounded_tr(radii.top_right)
                .rounded_bl(radii.bottom_left).rounded_br(radii.bottom_right))
            .when(floating, |d| d.child(div().absolute().inset_0().border_1().border_color(th.shade)
                .rounded_tl(radii.top_left).rounded_tr(radii.top_right)
                .rounded_bl(radii.bottom_left).rounded_br(radii.bottom_right))))
        .when(resizable, |d| d.child(canvas(
            |bounds: Bounds<Pixels>, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                let size = window.window_bounds().get_bounds().size;
                if let Some(edge) = resize_edge(window.mouse_position(), size, tiling, margin) {
                    window.set_cursor_style(match edge {
                        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
                        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
                        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
                        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
                    }, &hitbox);
                }
                window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                    if phase != DispatchPhase::Capture || ev.button != MouseButton::Left { return; }
                    let size = window.window_bounds().get_bounds().size;
                    if let Some(edge) = resize_edge(ev.position, size, tiling, margin) {
                        window.start_window_resize(edge);
                        cx.stop_propagation();
                    }
                });
            },
        ).absolute().inset_0()))
        .into_any_element()
}

/// Windows 11 rounds windows that keep their frame; ask for it explicitly so
/// the hidden caption never opts the window out. A no-op before Windows 11.
#[cfg(target_os = "windows")]
fn prefer_rounded_corners(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};
    let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else { return };
    let preference = DWMWCP_ROUND;
    // SAFETY: the handle belongs to this live window and the attribute value
    // is a valid DWM_WINDOW_CORNER_PREFERENCE of the size passed.
    unsafe {
        DwmSetWindowAttribute(handle.hwnd.get() as _, DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&preference as *const i32).cast(), std::mem::size_of::<i32>() as u32);
    }
}

#[cfg(not(target_os = "windows"))]
fn prefer_rounded_corners(_window: &Window) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_edges_follow_corners_and_skip_tiled_sides() {
        let size = gpui::size(px(800.0), px(600.0));
        let free = Tiling::default();
        assert_eq!(resize_edge(point(px(2.0), px(300.0)), size, free, 0.0), Some(ResizeEdge::Left));
        assert_eq!(resize_edge(point(px(798.0), px(2.0)), size, free, 0.0), Some(ResizeEdge::TopRight));
        assert_eq!(resize_edge(point(px(10.0), px(598.0)), size, free, 0.0), Some(ResizeEdge::BottomLeft));
        assert_eq!(resize_edge(point(px(400.0), px(300.0)), size, free, 0.0), None);
        let tiled_left = Tiling { left: true, ..Default::default() };
        assert_eq!(resize_edge(point(px(2.0), px(300.0)), size, tiled_left, 0.0), None);
        // The shadow margin is part of the resize band.
        assert_eq!(resize_edge(point(px(14.0), px(300.0)), size, free, SHADOW_MARGIN), Some(ResizeEdge::Left));
        assert_eq!(resize_edge(point(px(40.0), px(300.0)), size, free, SHADOW_MARGIN), None);
    }

    #[test]
    fn only_drawn_chrome_has_caption_buttons_or_inset() {
        let mut chrome = ChromeState { style: ChromeStyle::Mac, ..Default::default() };
        assert!(!chrome.draws_caption_buttons());
        assert_eq!(chrome.leading_inset(), TRAFFIC_LIGHT_PADDING);
        chrome.fullscreen = true;
        assert_eq!(chrome.leading_inset(), 0.0);
        for style in [ChromeStyle::Windows, ChromeStyle::LinuxClient] {
            let chrome = ChromeState { style, ..Default::default() };
            assert!(chrome.draws_caption_buttons() && chrome.leading_inset() == 0.0);
        }
        assert!(!ChromeState::default().draws_caption_buttons());
    }
}
