use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use anyhow::{Result, anyhow};
use futures::channel::oneshot;
use gpui::{
    Action, ActivityGuard, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle, DummyKeyboardMapper, ForegroundExecutor,
    GestureTuning, Keymap, Menu, MenuItem, PathPromptOptions, Platform, PlatformDisplay, PlatformGestures, PlatformInfo,
    PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem, PlatformWindow, ScrollPhysics, Task, ThermalState,
    WindowAppearance, WindowKind, WindowParams, popup::PopupNotSupportedError,
};
use gpui_wgpu::{CosmicTextSystem, GpuContext};
use ndk::native_window::NativeWindow;

use super::{
    dispatcher::{AndroidDispatcher, TaskQueue, run_main_thread_tasks},
    window::{AndroidDisplay, AndroidWindow, AndroidWindowState, appearance},
};

/// How long the event loop sleeps between frame requests while a window is visible.
/// GPUI skips frames that have nothing to draw.
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

/// Android's system UI font; `CosmicTextSystem` falls back to it for missing glyphs.
const SYSTEM_FONT: &str = "Roboto";

struct AndroidGestures;

impl PlatformGestures for AndroidGestures {
    fn tuning(&self) -> GestureTuning {
        GestureTuning { scroll_physics: ScrollPhysics::android(), ..GestureTuning::default() }
    }
}

struct AndroidKeyboardLayout;

impl PlatformKeyboardLayout for AndroidKeyboardLayout {
    fn id(&self) -> &str {
        "android"
    }

    fn name(&self) -> &str {
        "Android"
    }
}

#[derive(Default)]
struct PlatformCallbacks {
    quit: Option<Box<dyn FnMut() -> bool>>,
    open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    reopen: Option<Box<dyn FnMut()>>,
    memory_warning: Option<Box<dyn FnMut()>>,
    app_menu_action: Option<Box<dyn FnMut(&dyn Action)>>,
    will_open_app_menu: Option<Box<dyn FnMut()>>,
    validate_app_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    keyboard_layout_change: Option<Box<dyn FnMut()>>,
    thermal_state_change: Option<Box<dyn FnMut()>>,
}

/// The GPUI platform for an Android `NativeActivity`.
///
/// Create it inside `android_main` and run the application there: that thread becomes
/// GPUI's main thread, and [`Platform::run`] drives the activity's event loop. Windows
/// can only be opened from the `run` callback onwards, once the activity has a native
/// window; an Android activity has exactly one.
pub struct AndroidPlatform {
    app: AndroidApp,
    dispatcher: Arc<AndroidDispatcher>,
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    main_queue: Arc<TaskQueue>,
    text_system: Arc<dyn PlatformTextSystem>,
    gpu_context: GpuContext,
    display: Rc<AndroidDisplay>,
    native_window: RefCell<Option<NativeWindow>>,
    window: RefCell<Option<(AnyWindowHandle, Rc<AndroidWindowState>)>>,
    quit_requested: Cell<bool>,
    callbacks: RefCell<PlatformCallbacks>,
    gestures: Rc<AndroidGestures>,
}

impl AndroidPlatform {
    pub fn new(app: AndroidApp) -> Self {
        let (dispatcher, main_queue) = AndroidDispatcher::new(app.create_waker());
        let dispatcher = Arc::new(dispatcher);
        Self {
            dispatcher: dispatcher.clone(),
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            main_queue,
            text_system: Arc::new(CosmicTextSystem::new(SYSTEM_FONT)),
            gpu_context: Rc::new(RefCell::new(None)),
            display: Rc::new(AndroidDisplay { size: Cell::default() }),
            native_window: RefCell::default(),
            window: RefCell::default(),
            quit_requested: Cell::new(false),
            callbacks: RefCell::default(),
            gestures: Rc::new(AndroidGestures),
            app,
        }
    }

    /// The activity this platform runs in, for application code that needs JNI or
    /// Android paths.
    pub fn android_app(&self) -> &AndroidApp {
        &self.app
    }

    fn window_state(&self) -> Option<Rc<AndroidWindowState>> {
        self.window.borrow().as_ref().map(|(_, state)| state.clone())
    }

    fn handle_event(&self, event: PollEvent<'_>, on_finish_launching: &mut Option<Box<dyn 'static + FnOnce()>>) {
        let PollEvent::Main(event) = event else {
            return;
        };
        match event {
            MainEvent::InitWindow { .. } => {
                let native_window = self.app.native_window();
                *self.native_window.borrow_mut() = native_window.clone();
                if let (Some(state), Some(native_window)) = (self.window_state(), native_window) {
                    state.surface_created(native_window);
                }
                if self.native_window.borrow().is_some()
                    && let Some(launch) = on_finish_launching.take()
                {
                    launch();
                }
            }
            MainEvent::TerminateWindow { .. } => {
                if let Some(state) = self.window_state() {
                    state.surface_destroyed();
                }
                self.native_window.borrow_mut().take();
            }
            MainEvent::WindowResized { .. } | MainEvent::ConfigChanged { .. } => {
                if let Some(state) = self.window_state() {
                    state.configuration_changed();
                }
            }
            MainEvent::RedrawNeeded { .. } => {
                if let Some(state) = self.window_state() {
                    state.configuration_changed();
                    // Android expects the presentation before the poll callback returns.
                    state.redraw_needed();
                }
            }
            MainEvent::ContentRectChanged { .. } | MainEvent::InsetsChanged { .. } => {
                if let Some(state) = self.window_state() {
                    state.insets_changed();
                }
            }
            MainEvent::GainedFocus => {
                if let Some(state) = self.window_state() {
                    state.set_active(true);
                }
            }
            MainEvent::LostFocus => {
                if let Some(state) = self.window_state() {
                    state.set_active(false);
                }
            }
            MainEvent::LowMemory => {
                let callback = self.callbacks.borrow_mut().memory_warning.take();
                if let Some(mut callback) = callback {
                    callback();
                    self.callbacks.borrow_mut().memory_warning.get_or_insert(callback);
                }
            }
            MainEvent::InputAvailable => self.handle_input(),
            MainEvent::Destroy => self.quit_requested.set(true),
            _ => {}
        }
    }

    fn handle_input(&self) {
        let mut events = match self.app.input_events_iter() {
            Ok(events) => events,
            Err(error) => {
                log::error!("failed to read Android input events: {error}");
                return;
            }
        };
        let state = self.window_state();
        while events.next(|event| match &state {
            Some(state) if state.handle_input(event) => InputStatus::Handled,
            _ => InputStatus::Unhandled,
        }) {}
    }
}

impl Drop for AndroidPlatform {
    fn drop(&mut self) {
        self.dispatcher.shutdown();
    }
}

impl Platform for AndroidPlatform {
    fn platform_info(&self) -> PlatformInfo {
        PlatformInfo { os: gpui::OperatingSystem::Android, target: gpui::PlatformTarget::Native }
    }

    fn background_executor(&self) -> BackgroundExecutor {
        self.background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.foreground_executor.clone()
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.text_system.clone()
    }

    fn gestures(&self) -> Option<Rc<dyn PlatformGestures>> {
        Some(self.gestures.clone())
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        let mut on_finish_launching = Some(on_finish_launching);
        while !self.quit_requested.get() {
            let drawable = self.window_state().is_some_and(|state| state.is_drawable());
            // A prior batch may have left runnable work even with no surface.
            let timeout = if self.main_queue.has_pending() {
                Some(Duration::ZERO)
            } else {
                drawable.then_some(FRAME_INTERVAL)
            };
            self.app.poll_events(timeout, |event| {
                if !self.quit_requested.get() {
                    self.handle_event(event, &mut on_finish_launching);
                }
            });
            if self.quit_requested.get() {
                break;
            }
            run_main_thread_tasks(&self.main_queue, || self.quit_requested.get());
            if self.quit_requested.get() {
                break;
            }
            if let Some(state) = self.window_state() {
                state.request_frame();
            }
        }

        if let Some((_, state)) = self.window.borrow_mut().take() {
            state.close();
        }
        let quit = self.callbacks.borrow_mut().quit.take();
        if let Some(mut quit) = quit {
            quit();
        }
        self.dispatcher.shutdown();
    }

    fn quit(&self) {
        let should_close = self.window_state().is_none_or(|state| state.should_close());
        if should_close {
            self.quit_requested.set(true);
            self.app.create_waker().wake();
        }
    }

    fn restart(&self, _binary_path: Option<PathBuf>, _arguments: Vec<std::ffi::OsString>) {
        log::warn!("restarting is not supported on Android");
    }

    fn activate(&self, _ignoring_other_apps: bool) {}

    fn hide(&self) {}

    fn hide_other_apps(&self) {}

    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![self.display.clone()]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.display.clone())
    }

    fn on_displays_changed(&self, _callback: Box<dyn FnMut()>) {}

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.window.borrow().as_ref().map(|(handle, _)| *handle)
    }

    fn open_window(&self, handle: AnyWindowHandle, params: WindowParams) -> Result<Box<dyn PlatformWindow>> {
        match params.kind {
            WindowKind::Normal => {}
            WindowKind::AnchoredPopup(_) => return Err(PopupNotSupportedError.into()),
            WindowKind::PopUp | WindowKind::Floating | WindowKind::Dialog => {
                return Err(anyhow!("an Android activity has a single window; render this inside it instead"));
            }
        }
        if self.window.borrow().is_some() {
            return Err(anyhow!("an Android activity has a single window, and it is already open"));
        }
        let native_window = self
            .native_window
            .borrow()
            .clone()
            .ok_or_else(|| anyhow!("the activity has no native window yet; open windows from the run callback"))?;
        let (window, state) = AndroidWindow::new(handle, self.app.clone(), native_window, self.gpu_context.clone(), self.display.clone())?;
        *self.window.borrow_mut() = Some((handle, state));
        Ok(Box::new(window))
    }

    fn window_appearance(&self) -> WindowAppearance {
        appearance(&self.app)
    }

    fn open_url(&self, url: &str) {
        log::warn!("opening URLs is not implemented on Android yet: {url}");
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.callbacks.borrow_mut().open_urls = Some(callback);
    }

    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Err(anyhow!("URL schemes are declared in the Android manifest")))
    }

    fn prompt_for_paths(&self, _options: PathPromptOptions) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (sender, receiver) = oneshot::channel();
        sender.send(Err(anyhow!("file pickers need the Storage Access Framework, which is not implemented yet"))).ok();
        receiver
    }

    fn prompt_for_new_path(&self, _directory: &Path, _suggested_name: Option<&str>) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (sender, receiver) = oneshot::channel();
        sender.send(Err(anyhow!("file pickers need the Storage Access Framework, which is not implemented yet"))).ok();
        receiver
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }

    fn reveal_path(&self, _path: &Path) {}

    fn open_with_system(&self, _path: &Path) {}

    fn on_quit(&self, callback: Box<dyn FnMut() -> bool>) {
        self.callbacks.borrow_mut().quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.callbacks.borrow_mut().reopen = Some(callback);
    }

    fn on_system_sleep(&self, _callback: Box<dyn FnMut()>) {}

    fn on_system_wake(&self, _callback: Box<dyn FnMut()>) {}

    fn on_memory_warning(&self, callback: Box<dyn FnMut()>) {
        self.callbacks.borrow_mut().memory_warning = Some(callback);
    }

    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}

    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn Action)>) {
        self.callbacks.borrow_mut().app_menu_action = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.callbacks.borrow_mut().will_open_app_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn Action) -> bool>) {
        self.callbacks.borrow_mut().validate_app_menu_command = Some(callback);
    }

    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }

    fn on_thermal_state_change(&self, callback: Box<dyn FnMut()>) {
        self.callbacks.borrow_mut().thermal_state_change = Some(callback);
    }

    fn prevent_idle_sleep(&self, reason: &str) -> Task<Result<ActivityGuard>> {
        Task::ready(Err(anyhow!("keeping the screen on for {reason:?} is not implemented on Android yet")))
    }

    fn compositor_name(&self) -> &'static str {
        "Android"
    }

    fn app_path(&self) -> Result<PathBuf> {
        Ok(std::env::current_exe()?)
    }

    fn path_for_auxiliary_executable(&self, name: &str) -> Result<PathBuf> {
        Err(anyhow!("Android apps cannot run auxiliary executables such as {name:?}"))
    }

    fn set_cursor_style(&self, _style: CursorStyle) {}

    fn hide_cursor_until_mouse_moves(&self) {}

    fn is_cursor_visible(&self) -> bool {
        false
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        None
    }

    fn write_to_clipboard(&self, _item: ClipboardItem) {
        log::warn!("the clipboard is not implemented on Android yet");
    }

    fn write_credentials(&self, _url: &str, _username: &str, _password: &[u8]) -> Task<Result<()>> {
        Task::ready(Err(anyhow!("credential storage is not implemented on Android yet")))
    }

    fn read_credentials(&self, _url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        Task::ready(Ok(None))
    }

    fn delete_credentials(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Err(anyhow!("credential storage is not implemented on Android yet")))
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(AndroidKeyboardLayout)
    }

    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }

    fn on_keyboard_layout_change(&self, callback: Box<dyn FnMut()>) {
        self.callbacks.borrow_mut().keyboard_layout_change = Some(callback);
    }
}
