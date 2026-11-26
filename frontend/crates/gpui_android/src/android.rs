mod dispatcher;
mod platform;
mod window;

pub use android_activity::AndroidApp;
pub use platform::AndroidPlatform;

use std::rc::Rc;

/// Creates the GPUI application for the activity passed to `android_main`.
///
/// Call it on the `android_main` thread: that thread becomes GPUI's main thread and
/// `Application::run` drives the activity's event loop until it is destroyed.
pub fn application(app: AndroidApp) -> gpui::Application {
    gpui::Application::with_platform(Rc::new(AndroidPlatform::new(app)))
}
