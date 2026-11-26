//! Native Android smoke application for GPUI.
//!
//! Build it as a shared library for an Android target and package it with
//! `crates/gpui_android/script/build-apk.sh`.

#[cfg(target_os = "android")]
use gpui::{Context, Window, WindowOptions, div, prelude::*, px, rgb};

#[cfg(target_os = "android")]
struct AndroidExample {
    tap_count: usize,
}

#[cfg(target_os = "android")]
impl Render for AndroidExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let insets = window.insets().safe_area;
        div()
            .size_full()
            .pt(insets.top)
            .pb(insets.bottom)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_6()
            .bg(rgb(0x101622))
            .text_color(rgb(0xf5f7ff))
            .child(div().text_3xl().child("GPUI on Android"))
            .child(
                div()
                    .id("tap-counter")
                    .px_6()
                    .py_3()
                    .rounded_lg()
                    .bg(rgb(0x246bfd))
                    .active(|style| style.bg(rgb(0x1a4fc0)))
                    .text_xl()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.tap_count += 1;
                        cx.notify();
                    }))
                    .child(format!("Taps: {}", self.tap_count)),
            )
            .child(div().max_w(px(320.)).text_center().child("Rust app, NativeActivity loop, GPUI rendering through wgpu"))
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: gpui_android::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("gpui"));
    gpui_android::application(app).run(|cx| {
        if let Err(error) = cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| AndroidExample { tap_count: 0 })) {
            log::error!("failed to open the window: {error:#}");
        }
    });
}
