//! Appearance. Neutral surfaces in the platform's light or dark appearance,
//! one control accent, and track colours (Logic / GarageBand colour regions by
//! track). Optional Pride chrome adds a thin rainbow and an accessible accent.
//! No gradients, glows or tinted "status boxes": state is carried by words,
//! weight and outline first.
//!
//! Teal is the first track colour as a nod to 绿坝娘; it is a track colour,
//! not the colour of the application.
//!
//! `appearance` in the user's prefs.json stores `system`, `dark`, or `light`.
//! A command-line override lasts for the session; an explicit appearance action
//! replaces it. Appearance changes only preferences and repaint state, never the
//! project, revision, undo history, or render cache.

use chrono::{Datelike, Local, NaiveDate};
use gpui::{Rgba, WindowAppearance, rgb, rgba};

/// User preference, independent of both the OS and a session-only CLI override.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearancePreference {
    #[default]
    System,
    Dark,
    Light,
}

impl AppearancePreference {
    pub fn from_prefs(prefs: &crate::files::Prefs) -> Self {
        match prefs.extra.get("appearance").and_then(serde_json::Value::as_str) {
            Some("dark") => Self::Dark,
            Some("light") => Self::Light,
            _ => Self::System,
        }
    }

    /// Returns whether the durable preference changed. Other preferences survive.
    pub fn store(self, prefs: &mut crate::files::Prefs) -> bool {
        let value = serde_json::Value::String(match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }.into());
        if prefs.extra.get("appearance") == Some(&value) {
            return false;
        }
        prefs.extra.insert("appearance".into(), value);
        true
    }
}

/// Independent of light/dark appearance; only explicit choices are persisted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PridePreference {
    #[default]
    Auto,
    On,
    Off,
}

impl PridePreference {
    pub fn from_prefs(prefs: &crate::files::Prefs) -> Self {
        match prefs.extra.get("pride_theme").and_then(serde_json::Value::as_str) {
            Some("on") => Self::On,
            Some("off") => Self::Off,
            _ => Self::Auto,
        }
    }

    pub fn store(self, prefs: &mut crate::files::Prefs) -> bool {
        let value = serde_json::Value::String(match self {
            Self::Auto => "auto",
            Self::On => "on",
            Self::Off => "off",
        }.into());
        if prefs.extra.get("pride_theme") == Some(&value) {
            return false;
        }
        prefs.extra.insert("pride_theme".into(), value);
        true
    }

    pub fn active_on(self, date: NaiveDate) -> bool {
        match self {
            Self::Auto => matches!((date.month(), date.day()), (5, 17) | (6, 28) | (10, 11)),
            Self::On => true,
            Self::Off => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub pride: bool,
    /// Window background (behind panes).
    pub window: Rgba,
    /// Pane surfaces: control bar, inspector, track headers, mixer.
    pub surface: Rgba,
    /// Raised controls (segmented buttons, slots).
    pub control: Rgba,
    pub control_pressed: Rgba,
    /// Border / hairline separators between panes.
    pub separator: Rgba,
    /// Primary foreground.
    pub text: Rgba,
    /// Muted foreground, also suitable for small labels.
    pub text_secondary: Rgba,
    /// Quiet foreground; retains readable contrast on panes and controls.
    pub text_tertiary: Rgba,
    pub accent: Rgba,
    pub accent_text: Rgba,
    pub selection: Rgba,
    pub success: Rgba,
    pub error: Rgba,
    pub warning: Rgba,
    /// Piano-roll field.
    pub roll_white: Rgba,
    pub roll_black: Rgba,
    pub grid_bar: Rgba,
    pub grid_beat: Rgba,
    pub grid_sub: Rgba,
    pub key_white: Rgba,
    pub key_black: Rgba,
    pub playhead: Rgba,
    /// LCD readout surface in the control bar.
    pub lcd: Rgba,
    pub meter_low: Rgba,
    pub meter_high: Rgba,
}

impl Theme {
    /// Resolve every paint token with CLI > saved preference > OS precedence.
    pub fn resolve(platform: WindowAppearance, session_override: Option<bool>, prefs: &crate::files::Prefs) -> Self {
        Self::resolve_on_date(platform, session_override, prefs, Local::now().date_naive())
    }

    /// Explicit local calendar input keeps calendar and precedence tests deterministic.
    pub fn resolve_on_date(platform: WindowAppearance, session_override: Option<bool>, prefs: &crate::files::Prefs, date: NaiveDate) -> Self {
        Self::resolve_appearance(platform, session_override, prefs)
            .with_pride(PridePreference::from_prefs(prefs).active_on(date))
    }

    pub(crate) fn resolve_appearance(platform: WindowAppearance, session_override: Option<bool>, prefs: &crate::files::Prefs) -> Self {
        match session_override {
            Some(true) => Self::dark(),
            Some(false) => Self::light(),
            None => match AppearancePreference::from_prefs(prefs) {
                AppearancePreference::Dark => Self::dark(),
                AppearancePreference::Light => Self::light(),
                AppearancePreference::System => Self::for_appearance(platform),
            },
        }
    }

    /// Always start from neutral tokens, so disabling restores the exact baseline.
    pub fn with_pride(self, active: bool) -> Self {
        if self.pride == active {
            return self;
        }
        let mut theme = if active { self } else if self.dark { Self::dark() } else { Self::light() };
        theme.pride = active;
        if active {
            theme.accent = rgb(if self.dark { 0xa78bfa } else { 0x6d28d9 });
            theme.accent_text = rgb(if self.dark { 0x1c1c1e } else { 0xffffff });
            theme.selection = rgba(if self.dark { 0xa78bfa40 } else { 0x6d28d933 });
        }
        theme
    }

    /// The same preference-only transition used by the live editor action.
    pub fn toggle(platform: WindowAppearance, session_override: &mut Option<bool>, prefs: &mut crate::files::Prefs) -> Self {
        let current = Self::resolve(platform, *session_override, prefs);
        let preference = if current.dark { AppearancePreference::Light } else { AppearancePreference::Dark };
        preference.store(prefs);
        *session_override = None;
        Self::resolve(platform, None, prefs)
    }

    pub fn for_appearance(a: WindowAppearance) -> Self {
        match a {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
            _ => Self::dark(),
        }
    }

    pub fn dark() -> Self {
        let success = rgb(0x59d987);
        let warning = rgb(0xff9f0a);
        Theme {
            dark: true,
            pride: false,
            window: rgb(0x1c1c1e),
            surface: rgb(0x262628),
            control: rgb(0x3a3a3c),
            control_pressed: rgb(0x4c4c50),
            separator: rgb(0x787880),
            text: rgb(0xebebf0),
            text_secondary: rgb(0xc3c3ca),
            text_tertiary: rgb(0xbdbdc6),
            accent: rgb(0x076bd0),
            accent_text: rgb(0xffffff),
            selection: rgba(0x0a84ff40),
            success,
            error: rgb(0xff7268),
            warning,
            roll_white: rgb(0x2a2a2d),
            roll_black: rgb(0x232326),
            grid_bar: rgb(0x4b4b50),
            grid_beat: rgb(0x38383c),
            grid_sub: rgb(0x303033),
            key_white: rgb(0xd9d9de),
            key_black: rgb(0x1a1a1c),
            playhead: rgb(0xf2f2f7),
            lcd: rgb(0x141416),
            meter_low: success,
            meter_high: warning,
        }
    }

    pub fn light() -> Self {
        let success = rgb(0x18733b);
        let warning = rgb(0xc93400);
        Theme {
            dark: false,
            pride: false,
            window: rgb(0xe8e8ed),
            surface: rgb(0xf5f5f7),
            control: rgb(0xffffff),
            control_pressed: rgb(0xd8d8de),
            separator: rgb(0x898990),
            text: rgb(0x1d1d1f),
            // Keep every text ramp above the 4.5 contrast floor on the
            // darkest light background (control_pressed 0xd8d8de).
            text_secondary: rgb(0x575760),
            text_tertiary: rgb(0x5e5e66),
            accent: rgb(0x0065cf),
            accent_text: rgb(0xffffff),
            selection: rgba(0x007aff33),
            success,
            error: rgb(0xd70015),
            warning,
            roll_white: rgb(0xffffff),
            roll_black: rgb(0xf2f2f5),
            grid_bar: rgb(0xb4b4bb),
            grid_beat: rgb(0xdcdce1),
            grid_sub: rgb(0xececf0),
            key_white: rgb(0xffffff),
            key_black: rgb(0x2c2c2e),
            playhead: rgb(0x1d1d1f),
            lcd: rgb(0xe0e0e5),
            meter_low: success,
            meter_high: warning,
        }
    }

    /// Track colour by palette name (see `verdant_domain::TRACK_COLORS`).
    pub fn track(&self, name: Option<&str>) -> Rgba {
        let (d, l) = match name.unwrap_or("graphite") {
            "teal" => (0x45b5a5, 0x35a99a),
            "blue" => (0x73a7ef, 0x6aa0e8),
            "orange" => (0xe89a45, 0xeba456),
            "purple" => (0xb69ae9, 0xb394e3),
            "rose" => (0xe88da6, 0xe4829b),
            "yellow" => (0xd8bd4a, 0xd2b747),
            _ => (0xaaaab2, 0xa3a3ac),
        };
        rgb(if self.dark { d } else { l })
    }

    /// Text that sits on a track-coloured fill.
    pub fn on_track(&self) -> Rgba {
        rgb(0x111113)
    }
}

impl crate::editor::Editor {
    pub fn set_pride_preference(&mut self, preference: PridePreference, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.prefs_dirty |= preference.store(&mut self.prefs);
        self.theme = Theme::resolve(window.appearance(), self.appearance_override, &self.prefs);
        cx.notify();
    }

    /// Consumer entry point for the appearance menu. The existing slow pump
    /// durably writes prefs.json; notify repaints immediately, without a restart.
    pub fn set_appearance_preference(&mut self, preference: AppearancePreference, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.prefs_dirty |= preference.store(&mut self.prefs);
        self.appearance_override = None;
        self.theme = Theme::resolve(window.appearance(), None, &self.prefs);
        cx.notify();
    }

    /// Switch to the opposite of the currently displayed appearance, including
    /// when that appearance came from the OS or the CLI.
    pub fn toggle_appearance(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.theme = Theme::toggle(window.appearance(), &mut self.appearance_override, &mut self.prefs);
        self.prefs_dirty = true;
        cx.notify();
    }
}

/// Compact preference selector for an editor settings surface. Every choice
/// uses the same live action; a CLI override is shown until the user chooses.
pub fn appearance_controls(editor: &crate::editor::Editor, cx: &mut gpui::Context<crate::editor::Editor>) -> impl gpui::IntoElement {
    use gpui::{div, prelude::*, px};
    let theme = editor.theme;
    let selected = match editor.appearance_override {
        Some(true) => AppearancePreference::Dark,
        Some(false) => AppearancePreference::Light,
        None => AppearancePreference::from_prefs(&editor.prefs),
    };
    div().id("appearance-preference").flex().items_center().gap(px(4.0))
        .text_xs().text_color(theme.text)
        .child(div().mr(px(4.0)).text_color(theme.text_secondary).child("外观"))
        .children([
            ("appearance-system", "随系统", AppearancePreference::System),
            ("appearance-light", "浅色", AppearancePreference::Light),
            ("appearance-dark", "深色", AppearancePreference::Dark),
        ].into_iter().map(|(id, label, preference)| {
            let active = selected == preference;
            div().id(id).role(gpui::Role::Button).aria_label(format!("外观：{label}"))
                .aria_selected(active).cursor_pointer().h(px(crate::ui_controls::BAR_BTN_H)).px(px(8.0)).flex().items_center().justify_center().rounded(px(5.0))
                .bg(if active { theme.accent } else { theme.control })
                .text_color(if active { theme.accent_text } else { theme.text })
                .border_1().border_color(if active { theme.accent } else { theme.separator })
                .on_click(cx.listener(move |ed, _, window, cx| ed.set_appearance_preference(preference, window, cx)))
                .child(label)
        }))
}

/// Voluntary Pride selection lives only in the existing appearance dock.
pub fn pride_controls(editor: &crate::editor::Editor, cx: &mut gpui::Context<crate::editor::Editor>) -> impl gpui::IntoElement {
    use gpui::{div, prelude::*, px};
    let theme = editor.theme;
    let selected = PridePreference::from_prefs(&editor.prefs);
    div().flex().flex_col().gap(px(4.0)).text_xs().text_color(theme.text)
        .child(div().text_color(theme.text_secondary).child("Pride 彩虹主题"))
        .child(div().flex().items_center().gap(px(4.0)).children([
            ("pride-auto", "自动", PridePreference::Auto),
            ("pride-on", "开", PridePreference::On),
            ("pride-off", "关", PridePreference::Off),
        ].into_iter().map(|(id, label, preference)| {
            let active = selected == preference;
            div().id(id).role(gpui::Role::Button).aria_label(format!("Pride 彩虹主题：{label}"))
                .aria_selected(active).cursor_pointer().h(px(crate::ui_controls::BAR_BTN_H)).px(px(8.0))
                .flex().items_center().justify_center().rounded(px(5.0))
                .bg(if active { theme.accent } else { theme.control })
                .text_color(if active { theme.accent_text } else { theme.text })
                .border_1().border_color(if active { theme.accent } else { theme.separator })
                .hover(move |s| s.bg(if active { theme.accent } else { theme.control_pressed }))
                .on_click(cx.listener(move |ed, _, window, cx| ed.set_pride_preference(preference, window, cx)))
                .child(label)
        })))
        .child(div().text_color(theme.text_secondary).child("自动：5 月 17 日、6 月 28 日、10 月 11 日"))
}

pub fn with_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a, ..c }
}

/// Mix `c` toward `toward` by `t` (0 = c, 1 = toward).
pub fn mix(c: Rgba, toward: Rgba, t: f32) -> Rgba {
    Rgba { r: c.r + (toward.r - c.r) * t, g: c.g + (toward.g - c.g) * t, b: c.b + (toward.b - c.b) * t, a: c.a }
}

pub fn white() -> Rgba {
    rgb(0xffffff)
}

pub fn black() -> Rgba {
    rgb(0x000000)
}

/// The platform UI face, chosen for a reason rather than by default:
/// SF on macOS, Segoe on Windows, the desktop's fontconfig `sans-serif`
/// on Linux (falling back to a face with CJK coverage if needed).
pub fn system_ui_font(available: &[String]) -> String {
    let has = |n: &str| available.iter().any(|a| a == n);
    if cfg!(target_os = "macos") {
        return ".SystemUIFont".into();
    }
    if cfg!(target_os = "windows") {
        for f in ["Segoe UI Variable Text", "Segoe UI"] {
            if has(f) {
                return f.into();
            }
        }
    }
    if let Ok(out) = std::process::Command::new("fc-match").args(["-f", "%{family[0]}", "sans-serif"]).output() {
        let fam = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !fam.is_empty() && has(&fam) {
            return fam;
        }
    }
    for f in ["Noto Sans CJK SC", "Source Han Sans SC", "WenQuanYi Zen Hei", "DejaVu Sans"] {
        if has(f) {
            return f.into();
        }
    }
    ".SystemUIFont".into()
}

/// A real fixed-pitch face for numeric readouts. The generic name
/// "monospace" is not resolved by every text backend; an unresolved family
/// falls back to the proportional UI face and makes running digits jitter.
pub fn system_mono_font(available: &[String]) -> String {
    let has = |n: &str| available.iter().any(|a| a == n);
    let preferred: &[&str] = if cfg!(target_os = "macos") {
        &["SF Mono", "Menlo", "Monaco"]
    } else if cfg!(target_os = "windows") {
        &["Cascadia Mono", "Consolas", "Courier New"]
    } else {
        &[]
    };
    if let Some(f) = preferred.iter().find(|f| has(f)) {
        return (*f).into();
    }
    if !cfg!(any(target_os = "macos", target_os = "windows")) {
        if let Ok(out) = std::process::Command::new("fc-match").args(["-f", "%{family[0]}", "monospace"]).output() {
            let fam = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !fam.is_empty() && has(&fam) {
                return fam;
            }
        }
    }
    for f in ["JetBrains Mono", "DejaVu Sans Mono", "Noto Sans Mono", "Liberation Mono", "Menlo", "Consolas"] {
        if has(f) {
            return f.into();
        }
    }
    "monospace".into()
}

/// Tabular (equal-advance) figures, for faces that default to proportional digits.
pub fn tabular_figures() -> gpui::FontFeatures {
    gpui::FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)]))
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
