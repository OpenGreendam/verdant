//! Piano-roll coordinate system (SPEC §5).
//!
//! `x = (tick − start_tick) · px_per_tick`, `y = (top_pitch − pitch) · row_height`
//! in logical pixels, computed in f64. A note with MIDI number `p` occupies the
//! continuous pitch band `[p, p+1)`. Zooming never touches project data.

use verdant_domain::Note;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicPoint {
    pub tick: f64,
    /// Continuous pitch; `floor()` is the MIDI row.
    pub pitch: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Validated document ticks per quarter note; coordinates keep source resolution.
    pub quarter_ticks: i64,
    pub start_tick: f64,
    pub px_per_tick: f64,
    pub top_pitch: f64,
    pub row_height: f64,
    pub width: f64,
    pub height: f64,
}

pub const MIN_PX_PER_QUARTER: f64 = 3.84;
pub const MAX_PX_PER_QUARTER: f64 = 1920.0;
pub const MIN_ROW: f64 = 6.0;
pub const MAX_ROW: f64 = 64.0;
/// Minimum width of the painted piano-roll note body in logical pixels.
pub const MIN_NOTE_BODY_PX: f64 = 2.0;

impl Default for Viewport {
    fn default() -> Self {
        Viewport { quarter_ticks: 960, start_tick: 0.0, px_per_tick: 0.12, top_pitch: 84.0, row_height: 18.0, width: 1200.0, height: 600.0 }
    }
}

impl Viewport {
    /// Change document resolution without changing the displayed musical span.
    pub fn set_quarter_ticks(&mut self, quarter_ticks: i64) {
        assert!(quarter_ticks > 0, "viewport requires validated positive PPQ");
        let ratio = quarter_ticks as f64 / self.quarter_ticks as f64;
        self.start_tick *= ratio;
        self.px_per_tick /= ratio;
        self.quarter_ticks = quarter_ticks;
    }

    fn horizontal_limits(&self) -> (f64, f64) {
        let quarter = self.quarter_ticks as f64;
        (MIN_PX_PER_QUARTER / quarter, MAX_PX_PER_QUARTER / quarter)
    }

    /// Frame musical material with fixed pixel margins at any source PPQ.
    pub fn fit_ticks(&mut self, start: f64, end: f64, leading: f64, trailing: f64) {
        let (min, max) = self.horizontal_limits();
        let span = (end - start).max(self.quarter_ticks as f64);
        self.px_per_tick = ((self.width - leading - trailing).max(1.0) / span).clamp(min, max);
        self.start_tick = start - leading / self.px_per_tick;
    }

    pub fn tick_to_x(&self, tick: f64) -> f64 {
        (tick - self.start_tick) * self.px_per_tick
    }
    pub fn x_to_tick(&self, x: f64) -> f64 {
        self.start_tick + x / self.px_per_tick
    }
    pub fn pitch_to_y(&self, pitch: f64) -> f64 {
        (self.top_pitch - pitch) * self.row_height
    }
    pub fn y_to_pitch(&self, y: f64) -> f64 {
        self.top_pitch - y / self.row_height
    }
    pub fn to_music(&self, x: f64, y: f64) -> MusicPoint {
        MusicPoint { tick: self.x_to_tick(x), pitch: self.y_to_pitch(y) }
    }

    pub fn note_rect(&self, n: &Note) -> Rect {
        let x = self.tick_to_x(n.start_tick as f64);
        let y = self.pitch_to_y(n.midi_note as f64 + 1.0);
        Rect { x, y, w: n.duration_tick as f64 * self.px_per_tick, h: self.row_height }
    }

    #[inline]
    pub fn note_body_rect(&self, note: &Note) -> Rect {
        let mut rect = self.note_rect(note);
        rect.w = rect.w.max(MIN_NOTE_BODY_PX);
        rect
    }

    pub fn visible_ticks(&self) -> (f64, f64) {
        (self.start_tick, self.x_to_tick(self.width))
    }

    /// Inclusive MIDI rows intersecting the viewport.
    pub fn visible_rows(&self) -> (i32, i32) {
        let hi = self.top_pitch.floor() as i32;
        let lo = self.y_to_pitch(self.height).floor() as i32;
        (lo.max(0), hi.min(127))
    }

    /// Mouse-anchored horizontal zoom: the tick under `anchor_x` is invariant.
    pub fn zoom_x_at(&mut self, anchor_x: f64, factor: f64) {
        let anchor_tick = self.x_to_tick(anchor_x);
        let (min, max) = self.horizontal_limits();
        self.px_per_tick = (self.px_per_tick * factor).clamp(min, max);
        self.start_tick = anchor_tick - anchor_x / self.px_per_tick;
        self.clamp();
    }

    /// Mouse-anchored vertical zoom: the pitch under `anchor_y` is invariant.
    pub fn zoom_y_at(&mut self, anchor_y: f64, factor: f64) {
        let anchor_pitch = self.y_to_pitch(anchor_y);
        self.row_height = (self.row_height * factor).clamp(MIN_ROW, MAX_ROW);
        self.top_pitch = anchor_pitch + anchor_y / self.row_height;
        self.clamp();
    }

    pub fn pan_px(&mut self, dx: f64, dy: f64) {
        self.start_tick += dx / self.px_per_tick;
        self.top_pitch -= dy / self.row_height;
        self.clamp();
    }

    fn clamp(&mut self) {
        // Allow a little negative tick space so the first note is not glued to
        // the edge, and keep at least one octave of the MIDI range in view.
        let min_start = -(48.0 / self.px_per_tick);
        if self.start_tick < min_start {
            self.start_tick = min_start;
        }
        let rows_visible = self.height / self.row_height;
        self.top_pitch = self.top_pitch.clamp(rows_visible.min(128.0), 128.0);
    }

    /// Scroll so `n` is fully visible with a margin (keyboard access, T11).
    pub fn reveal(&mut self, n: &Note) {
        let r = self.note_rect(n);
        let margin = 48.0;
        if r.x < margin {
            self.start_tick = n.start_tick as f64 - margin / self.px_per_tick;
        } else if r.right() > self.width - margin {
            let w = r.w.min(self.width - 2.0 * margin);
            self.start_tick = n.start_tick as f64 + w / self.px_per_tick - (self.width - margin) / self.px_per_tick;
        }
        if r.y < 0.0 || r.bottom() > self.height {
            self.top_pitch = n.midi_note as f64 + 0.5 + (self.height / self.row_height) / 2.0;
        }
        self.clamp();
    }
}

/// Grid snapping. `grid_ticks == 0` disables snapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snap {
    pub grid_ticks: i64,
}

impl Snap {
    pub fn snap(&self, tick: f64) -> i64 {
        if self.grid_ticks <= 0 {
            return tick.round() as i64;
        }
        let g = self.grid_ticks as f64;
        ((tick / g).round() * g) as i64
    }

    pub fn floor(&self, tick: f64) -> i64 {
        if self.grid_ticks <= 0 {
            return tick.floor() as i64;
        }
        let g = self.grid_ticks as f64;
        ((tick / g).floor() * g) as i64
    }
}
