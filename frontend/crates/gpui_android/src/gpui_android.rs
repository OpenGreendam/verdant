//! GPUI platform backend for Android tablets.
//!
//! The pinned GPUI fork has no Android platform, so Verdant carries one: a
//! `NativeActivity` event loop (android-activity), rendering through `gpui_wgpu`
//! (Vulkan or GLES), and touch input fed to GPUI's portable gesture recognizers.
//! On every other target this crate is empty. See `docs/ANDROID-TABLET.md`.

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub use android::*;
