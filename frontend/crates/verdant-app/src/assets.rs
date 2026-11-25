//! Embedded assets: the Lucide icons Verdant uses (ISC, see
//! `assets/icons/LICENSE-lucide.txt`). Compiled in so the binary has no
//! runtime asset lookup.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[$((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")))),*];
    };
}

icons!(
    "play",
    "pause",
    "square",
    "minus",
    "menu",
    "git-merge",
    "instrument",
    "preset-library",
    "lock",
    "lock-open",
    "reset",
    "window-minimize",
    "window-maximize",
    "window-restore",
    "window-close",
    "copy",
    "skip-back",
    "repeat",
    "undo-2",
    "redo-2",
    "panel-left",
    "panel-right",
    "piano",
    "sliders-vertical",
    "sliders-horizontal",
    "plus",
    "trash-2",
    "mouse-pointer-2",
    "pencil",
    "eraser",
    "keyboard",
    "audio-waveform",
    "chevron-down",
    "chevron-right",
    "mic-vocal",
    "music-2",
    "sparkles",
    "languages",
    "cpu",
    "history",
    "circle-dot",
    "bell",
    "maximize-2",
    "layers",
    "activity",
    "columns-3",
    "volume-2",
    "volume-x",
    "globe",
    "x",
    "circle-alert",
    "spline",
);

/// Fonts compiled into the binary (SIL OFL 1.1, see
/// `assets/fonts/LICENSE-OFL.txt`): Inter for Latin UI text, as GNOME's
/// Adwaita Sans is built from it, and a Noto Sans CJK SC subset so Chinese
/// and Japanese text renders from a well-hinted outline face on every
/// platform instead of whatever bitmap-era fallback the system offers.
pub const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/Inter-Regular.otf"),
    include_bytes!("../assets/fonts/Inter-Medium.otf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.otf"),
    include_bytes!("../assets/fonts/Inter-Bold.otf"),
    include_bytes!("../assets/fonts/NotoSansCJKsc-Regular.otf"),
    include_bytes!("../assets/fonts/NotoSansCJKsc-Bold.otf"),
];

/// Register the embedded fonts with the text system.
pub fn register_fonts(cx: &mut gpui::App) {
    let fonts = FONTS.iter().map(|bytes| Cow::Borrowed(*bytes)).collect();
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        eprintln!("verdant: embedded fonts unavailable: {error}");
    }
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS.iter().find(|(p, _)| *p == path).map(|(_, b)| Cow::Borrowed(*b)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICONS.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| SharedString::from(*p)).collect())
    }
}

pub fn icon(name: &'static str) -> SharedString {
    SharedString::from(format!("icons/{name}.svg"))
}
