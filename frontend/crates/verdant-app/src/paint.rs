//! Small painting helpers shared by the custom canvases.

use gpui::{App, Bounds, PathBuilder, Pixels, Point, Rgba, SharedString, TextAlign, TextRun, Window, point, px, size};

/// Text draws collected while an entity is borrowed, painted afterwards.
pub type Texts = Vec<(String, Point<Pixels>, f32, Rgba)>;

pub fn text(window: &mut Window, cx: &mut App, s: &str, origin: Point<Pixels>, size_px: f32, color: Rgba) -> Pixels {
    if s.is_empty() {
        return px(0.0);
    }
    let style = window.text_style();
    let run =
        TextRun { len: s.len(), font: style.font(), color: color.into(), background_color: None, underline: None, strikethrough: None };
    let line = window.text_system().shape_line(SharedString::from(s.to_string()), px(size_px), &[run], None);
    let w = line.width;
    let _ = line.paint(origin, px(size_px * 1.25), TextAlign::Left, None, window, cx);
    w
}

pub fn flush_texts(window: &mut Window, cx: &mut App, bounds: Bounds<Pixels>, texts: Texts) {
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        for (s, p, sz, c) in texts {
            text(window, cx, &s, p, sz, c);
        }
    });
}

/// Bounds from canvas-local f64 coordinates.
pub fn b(x: f64, y: f64, w: f64, h: f64, origin: Point<Pixels>) -> Bounds<Pixels> {
    Bounds::new(point(origin.x + px(x as f32), origin.y + px(y as f32)), size(px(w.max(0.0) as f32), px(h.max(0.0) as f32)))
}

pub fn p(x: f64, y: f64, origin: Point<Pixels>) -> Point<Pixels> {
    point(origin.x + px(x as f32), origin.y + px(y as f32))
}

/// Stroke a polyline given as canvas-local points; `None` breaks the line.
pub fn polyline(window: &mut Window, origin: Point<Pixels>, pts: impl IntoIterator<Item = Option<(f64, f64)>>, width: f32, color: Rgba) {
    let mut path = PathBuilder::stroke(px(width));
    let mut open = false;
    let mut segments = 0;
    for q in pts {
        match q {
            Some((x, y)) => {
                if open {
                    path.line_to(p(x, y, origin));
                    segments += 1;
                } else {
                    path.move_to(p(x, y, origin));
                    open = true;
                }
            }
            None => open = false,
        }
    }
    if segments > 0 {
        if let Ok(built) = path.build() {
            window.paint_path(built, color);
        }
    }
}

pub fn is_black_key(midi: i32) -> bool {
    matches!(midi.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}
