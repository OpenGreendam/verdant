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
    /// Pane surfaces (sidebars): inspector, track headers, mixer.
    pub surface: Rgba,
    /// Toolbar / title bar.
    pub headerbar: Rgba,
    /// Content views: timeline fields, lists, the LCD.
    pub view: Rgba,
    /// Grouped rows on a sidebar (preference-group cards).
    pub card: Rgba,
    /// Menus, popovers and dialogs.
    pub popover: Rgba,
    /// Quiet hairline between panes (decorative; `separator` is the
    /// 3:1 outline for boundaries a user must see).
    pub shade: Rgba,
    /// Hover fill for flat buttons and menu rows.
    pub hover: Rgba,
    /// A toggle that is on (GtkToggleButton:checked); carries primary text.
    pub checked: Rgba,
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

    /// GNOME HIG / libadwaita dark palette (window, view, header bar, card,
    /// popover), with every text ramp kept at WCAG AA on every surface.
    pub fn dark() -> Self {
        let success = rgb(0x8ff0a4);
        let warning = rgb(0xf8e45c);
        Theme {
            dark: true,
            pride: false,
            window: rgb(0x222226),
            surface: rgb(0x2e2e32),
            headerbar: rgb(0x2e2e32),
            view: rgb(0x1d1d20),
            card: rgb(0x38383c),
            popover: rgb(0x36363a),
            shade: rgb(0x18181b),
            hover: rgba(0xffffff17),
            checked: rgb(0x6a6a6e),
            control: rgb(0x434347),
            control_pressed: rgb(0x4e4e52),
            separator: rgb(0x808086),
            text: rgb(0xffffff),
            text_secondary: rgb(0xc8c8cc),
            text_tertiary: rgb(0xc0c0c5),
            accent: rgb(0x1c71d8),
            accent_text: rgb(0xffffff),
            selection: rgba(0x3584e440),
            success,
            error: rgb(0xff7b63),
            warning,
            roll_white: rgb(0x1d1d20),
            roll_black: rgb(0x19191c),
            grid_bar: rgb(0x48484d),
            grid_beat: rgb(0x323236),
            grid_sub: rgb(0x26262a),
            key_white: rgb(0xdedee2),
            key_black: rgb(0x1d1d20),
            playhead: rgb(0xffffff),
            lcd: rgb(0x1a1a1d),
            meter_low: success,
            meter_high: warning,
        }
    }

    /// GNOME HIG / libadwaita light palette.
    pub fn light() -> Self {
        let success = rgb(0x16733f);
        let warning = rgb(0x8a6102);
        Theme {
            dark: false,
            pride: false,
            window: rgb(0xfafafb),
            surface: rgb(0xebebed),
            headerbar: rgb(0xffffff),
            view: rgb(0xffffff),
            card: rgb(0xffffff),
            popover: rgb(0xffffff),
            shade: rgb(0xd6d6d9),
            hover: rgba(0x00000612),
            checked: rgb(0xb4b4b7),
            control: rgb(0xdcdcde),
            control_pressed: rgb(0xcdcdcf),
            separator: rgb(0x7f7f85),
            text: rgb(0x2e2e32),
            // Every text ramp stays above 4.5 on the darkest light
            // background (control_pressed).
            text_secondary: rgb(0x4f4f54),
            text_tertiary: rgb(0x56565b),
            accent: rgb(0x1c71d8),
            accent_text: rgb(0xffffff),
            selection: rgba(0x3584e433),
            success,
            error: rgb(0xc01c28),
            warning,
            roll_white: rgb(0xffffff),
            roll_black: rgb(0xf4f4f6),
            grid_bar: rgb(0xb8b8bd),
            grid_beat: rgb(0xdedee2),
            grid_sub: rgb(0xededf0),
            key_white: rgb(0xffffff),
            key_black: rgb(0x2e2e32),
            playhead: rgb(0x2e2e32),
            lcd: rgb(0xe4e4e7),
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
                .border_1().border_color(if active { theme.accent } else { theme.control })
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
                .border_1().border_color(if active { theme.accent } else { theme.control })
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
    // GNOME's own UI faces first, so Verdant matches libadwaita apps.
    for f in ["Adwaita Sans", "Cantarell"] {
        if has(f) {
            return f.into();
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
