//! The toolbar doubles as the window title bar ("unified title bar").
//!
//! - macOS: the native title bar is transparent; the traffic lights sit inside
//!   the toolbar and the app drives dragging and double-click zoom.
//! - Windows: the caption is hidden; Verdant draws minimise / maximise / close
//!   and marks them, and the empty toolbar areas, as native hit-test regions
//!   (so snap layouts and system drag still work).
//! - Linux: client-side decorations are requested. When the compositor grants
//!   them, Verdant draws the buttons the window manager supports, a 1 px frame
//!   and edge resize zones; otherwise the system title bar stays.
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
/// Width of the edge band that resizes a client-decorated window.
const RESIZE_BAND: f32 = 5.0;

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
    region
        .window_control_area(WindowControlArea::Drag)
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
        // Windows moves through the Drag hit-test area instead.
        if ev.pressed_button == Some(MouseButton::Left) && !cfg!(target_os = "windows") { window.start_window_move(); }
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
        // Keep presses on the buttons from starting a title-bar drag.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
    for caption in [Caption::Minimize, Caption::Maximize, Caption::Close] {
        let enabled = match caption {
            Caption::Minimize => chrome.can_minimize,
            Caption::Maximize => chrome.can_maximize,
            Caption::Close => true,
        };
        if !enabled && !windows { continue; }
        let (id, name, label, area) = match caption {
            Caption::Minimize => ("window-minimize", "minus", "最小化", WindowControlArea::Min),
            Caption::Maximize if chrome.maximized => ("window-maximize", "copy", "还原", WindowControlArea::Max),
            Caption::Maximize => ("window-maximize", "square", "最大化", WindowControlArea::Max),
            Caption::Close => ("window-close", "x", "关闭", WindowControlArea::Close),
        };
        let close = caption == Caption::Close;
        let hover = if close { gpui::rgb(0xc42b1c) } else { th.control_pressed };
        let fg = if enabled { th.text } else { th.text_tertiary };
        let mut button = div().id(id).role(gpui::Role::Button).aria_label(label)
            .w(px(if windows { 46.0 } else { 40.0 })).h_full().flex().items_center().justify_center().occlude()
            .child(svg().path(icon(name)).size(px(if caption == Caption::Maximize { 12.0 } else { 15.0 })).text_color(fg))
            .when(enabled, |b| b.hover(move |s| s.bg(hover)));
        if windows {
            // Native hit-testing performs the action (and shows snap layouts).
            button = button.window_control_area(area);
        } else {
            button = button.on_click(cx.listener(move |ed, _, window, cx| match caption {
                Caption::Minimize => window.minimize_window(),
                Caption::Maximize => window.zoom_window(),
                Caption::Close => ed.close_window(window, cx),
            }));
        }
        row = row.child(button);
    }
    Some(row.into_any_element())
}

fn resize_edge(pos: Point<Pixels>, size: gpui::Size<Pixels>, tiling: Tiling) -> Option<ResizeEdge> {
    let band = px(RESIZE_BAND);
    let corner = px(RESIZE_BAND * 3.0);
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

/// Linux client decorations: a 1 px frame plus resize zones along the window
/// edges, painted above everything without taking hover from the content.
pub fn client_frame(chrome: ChromeState, th: crate::theme::Theme, window: &mut Window) -> Option<AnyElement> {
    if chrome.style != ChromeStyle::LinuxClient {
        window.set_client_inset(px(0.0));
        return None;
    }
    window.set_client_inset(px(0.0));
    let tiling = chrome.tiling;
    let resizable = !chrome.maximized && !chrome.fullscreen;
    Some(div().absolute().inset_0()
        .when(!chrome.maximized, |d| d
            .when(!tiling.top, |d| d.border_t_1()).when(!tiling.bottom, |d| d.border_b_1())
            .when(!tiling.left, |d| d.border_l_1()).when(!tiling.right, |d| d.border_r_1())
            .border_color(th.shade))
        .when(resizable, |d| d.child(canvas(
            |bounds: Bounds<Pixels>, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                let size = window.window_bounds().get_bounds().size;
                if let Some(edge) = resize_edge(window.mouse_position(), size, tiling) {
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
                    if let Some(edge) = resize_edge(ev.position, size, tiling) {
                        window.start_window_resize(edge);
                        cx.stop_propagation();
                    }
                });
            },
        ).size_full()))
        .into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_edges_follow_corners_and_skip_tiled_sides() {
        let size = gpui::size(px(800.0), px(600.0));
        let free = Tiling::default();
        assert_eq!(resize_edge(point(px(2.0), px(300.0)), size, free), Some(ResizeEdge::Left));
        assert_eq!(resize_edge(point(px(798.0), px(2.0)), size, free), Some(ResizeEdge::TopRight));
        assert_eq!(resize_edge(point(px(10.0), px(598.0)), size, free), Some(ResizeEdge::BottomLeft));
        assert_eq!(resize_edge(point(px(400.0), px(300.0)), size, free), None);
        let tiled_left = Tiling { left: true, ..Default::default() };
        assert_eq!(resize_edge(point(px(2.0), px(300.0)), size, tiled_left), None);
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
