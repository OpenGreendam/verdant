use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use android_activity::{
    AndroidApp,
    input::{InputEvent, KeyAction, Keycode, MotionAction, MotionEvent},
};
use gpui::{
    AnyWindowHandle, Bounds, Capslock, Decorations, DevicePixels, DispatchEventResult, DisplayId, Edges, GpuSpecs, Modifiers, Pixels,
    PlatformAtlas, PlatformDisplay, PlatformInput, PlatformInputHandler, PlatformWindow, Point, PromptButton, PromptLevel,
    RequestFrameOptions, Scene, Size, TouchEvent, TouchId, TouchPhase, WindowAppearance, WindowBackgroundAppearance, WindowBounds,
    WindowControlArea, WindowControls, WindowInsets, WindowVisibility, px,
};
use gpui_wgpu::{GpuContext, WgpuRenderer, WgpuSurfaceConfig, wgpu};
use ndk::native_window::NativeWindow;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, WindowHandle};

/// Android's baseline density: one density-independent pixel is one physical pixel at
/// 160 dpi.
const BASELINE_DENSITY: f32 = 160.;

/// The activity's native window, in the form the wgpu renderer accepts.
#[derive(Clone, Debug)]
pub(crate) struct NativeSurface(pub(crate) NativeWindow);

impl HasWindowHandle for NativeSurface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for NativeSurface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: the Android display handle carries no data, so it cannot dangle.
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Android(raw_window_handle::AndroidDisplayHandle::new())) })
    }
}

impl NativeSurface {
    fn size(&self) -> Size<DevicePixels> {
        Size { width: DevicePixels(self.0.width().max(1)), height: DevicePixels(self.0.height().max(1)) }
    }
}

pub(crate) fn scale_factor(app: &AndroidApp) -> f32 {
    app.config().density().map_or(1., |density| density as f32 / BASELINE_DENSITY)
}

#[derive(Debug)]
pub(crate) struct AndroidDisplay {
    pub(crate) size: Cell<Size<Pixels>>,
}

impl PlatformDisplay for AndroidDisplay {
    fn id(&self) -> DisplayId {
        DisplayId::new(0)
    }

    fn uuid(&self) -> anyhow::Result<uuid::Uuid> {
        Ok(uuid::Uuid::from_u128(0x67_70_75_69_5f_61_6e_64_72_6f_69_64))
    }

    fn bounds(&self) -> Bounds<Pixels> {
        Bounds { origin: Point::default(), size: self.size.get() }
    }

    fn refresh_interval(&self) -> Option<std::time::Duration> {
        None
    }
}

#[derive(Default)]
struct Callbacks {
    request_frame: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    input: Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>,
    active_status_change: Option<Box<dyn FnMut(bool)>>,
    visibility_change: Option<Box<dyn FnMut(WindowVisibility)>>,
    resize: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    should_close: Option<Box<dyn FnMut() -> bool>>,
    close: Option<Box<dyn FnOnce()>>,
    appearance_changed: Option<Box<dyn FnMut()>>,
    insets_changed: Option<Box<dyn FnMut(WindowInsets)>>,
    back: Option<Box<dyn FnMut()>>,
}

/// Runs a stored callback without holding the `RefCell` borrow, so the callback may
/// re-enter the window (GPUI often does, e.g. drawing from inside an input dispatch).
macro_rules! with_callback {
    ($state:expr, $field:ident, |$callback:ident| $body:expr) => {{
        let taken = $state.callbacks.borrow_mut().$field.take();
        taken.map(|mut $callback| {
            let result = $body;
            let mut callbacks = $state.callbacks.borrow_mut();
            if callbacks.$field.is_none() {
                callbacks.$field = Some($callback);
            }
            result
        })
    }};
}

/// State shared by the platform's event loop and the GPUI-facing window object.
pub(crate) struct AndroidWindowState {
    app: AndroidApp,
    gpu_context: GpuContext,
    renderer: RefCell<WgpuRenderer>,
    surface: RefCell<Option<NativeSurface>>,
    presentation_pending: Cell<bool>,
    display: Rc<AndroidDisplay>,
    size: Cell<Size<Pixels>>,
    scale_factor: Cell<f32>,
    insets: RefCell<WindowInsets>,
    visibility: Cell<WindowVisibility>,
    is_active: Cell<bool>,
    back_enabled: Cell<bool>,
    title: RefCell<String>,
    input_handler: RefCell<Option<PlatformInputHandler>>,
    last_touch_position: Cell<Point<Pixels>>,
    /// Android reuses pointer ids as soon as a finger lifts; GPUI requires ids that are
    /// never reused, so each contact gets a fresh one.
    next_touch_id: Cell<u64>,
    active_touches: RefCell<Vec<(i32, TouchId)>>,
    callbacks: RefCell<Callbacks>,
}

impl AndroidWindowState {
    fn logical_size(&self, size: Size<DevicePixels>) -> Size<Pixels> {
        let scale = self.scale_factor.get();
        Size { width: px(size.width.0 as f32 / scale), height: px(size.height.0 as f32 / scale) }
    }

    /// The activity has a new native window, after `InitWindow`.
    pub(crate) fn surface_created(&self, native_window: NativeWindow) {
        let surface = NativeSurface(native_window);
        let size = surface.size();
        let instance = self.gpu_context.borrow().as_ref().map(|context| context.instance.clone());
        let Some(instance) = instance else {
            log::error!("cannot restore the Android surface: the GPU context is gone");
            return;
        };
        // replace_surface applies its mode without querying the new surface. FIFO is always supported.
        let config = WgpuSurfaceConfig { size, transparent: false, preferred_present_mode: Some(wgpu::PresentMode::Fifo) };
        let replaced = self.renderer.borrow_mut().replace_surface(&surface, config, &instance);
        match replaced {
            Ok(()) => {
                *self.surface.borrow_mut() = Some(surface);
                self.presentation_pending.set(true);
                self.configuration_changed();
                self.set_visibility(WindowVisibility::Visible);
            }
            Err(error) => log::error!("failed to attach the Android surface: {error:#}"),
        }
    }

    /// The native window is about to be destroyed, after `TerminateWindow`.
    pub(crate) fn surface_destroyed(&self) {
        self.renderer.borrow_mut().unconfigure_surface();
        self.surface.borrow_mut().take();
        self.set_visibility(WindowVisibility::Hidden);
    }

    /// Re-reads size, density and insets after a resize, rotation or configuration change.
    pub(crate) fn configuration_changed(&self) {
        let scale = scale_factor(&self.app);
        self.scale_factor.set(scale);
        let device_size = self.surface.borrow().as_ref().map(NativeSurface::size);
        if let Some(device_size) = device_size {
            self.renderer.borrow_mut().update_drawable_size(device_size);
            let size = self.logical_size(device_size);
            let changed = size != self.size.get();
            self.size.set(size);
            self.display.size.set(size);
            if changed {
                with_callback!(self, resize, |callback| callback(size, scale));
            }
        }
        self.insets_changed();
        with_callback!(self, appearance_changed, |callback| callback());
    }

    pub(crate) fn insets_changed(&self) {
        let Some(device_size) = self.surface.borrow().as_ref().map(NativeSurface::size) else {
            return;
        };
        let scale = self.scale_factor.get();
        let content = self.app.content_rect();
        let edge = |value: i32| px(value.max(0) as f32 / scale);
        let insets = WindowInsets {
            safe_area: Edges {
                top: edge(content.top),
                left: edge(content.left),
                bottom: edge(device_size.height.0 - content.bottom),
                right: edge(device_size.width.0 - content.right),
            },
            ..Default::default()
        };
        if *self.insets.borrow() != insets {
            *self.insets.borrow_mut() = insets.clone();
            with_callback!(self, insets_changed, |callback| callback(insets));
        }
    }

    pub(crate) fn set_active(&self, active: bool) {
        if self.is_active.replace(active) != active {
            with_callback!(self, active_status_change, |callback| callback(active));
        }
    }

    pub(crate) fn set_visibility(&self, visibility: WindowVisibility) {
        if self.visibility.replace(visibility) != visibility {
            with_callback!(self, visibility_change, |callback| callback(visibility));
        }
    }

    pub(crate) fn is_drawable(&self) -> bool {
        self.surface.borrow().is_some() && self.visibility.get() == WindowVisibility::Visible
    }

    pub(crate) fn redraw_needed(&self) {
        self.presentation_pending.set(true);
        self.request_frame();
    }

    pub(crate) fn request_frame(&self) {
        if !self.is_drawable() {
            return;
        }
        let force_render = self.renderer.borrow_mut().needs_redraw();
        if force_render {
            self.presentation_pending.set(true);
        }
        let require_presentation = self.presentation_pending.get();
        with_callback!(self, request_frame, |callback| callback(RequestFrameOptions {
            require_presentation,
            force_render,
            ..Default::default()
        }));
    }

    fn dispatch(&self, input: PlatformInput) -> Option<DispatchEventResult> {
        with_callback!(self, input, |callback| callback(input))
    }

    /// Returns whether the event was consumed. Unconsumed events go back to the system,
    /// so e.g. an unclaimed back press still leaves the activity.
    pub(crate) fn handle_input(&self, event: &InputEvent<'_>) -> bool {
        match event {
            InputEvent::MotionEvent(motion) => {
                self.handle_motion(motion);
                true
            }
            InputEvent::KeyEvent(key) if key.key_code() == Keycode::Back && self.back_enabled.get() => {
                if key.action() == KeyAction::Up {
                    with_callback!(self, back, |callback| callback());
                }
                true
            }
            _ => false,
        }
    }

    fn handle_motion(&self, motion: &MotionEvent<'_>) {
        let scale = self.scale_factor.get();
        let position_of = |index: usize| {
            let pointer = motion.pointer_at_index(index);
            Point::new(px(pointer.x() / scale), px(pointer.y() / scale))
        };
        match motion.action() {
            MotionAction::Down | MotionAction::PointerDown => {
                let index = motion.pointer_index();
                let pointer_id = motion.pointer_at_index(index).pointer_id();
                let id = TouchId(self.next_touch_id.get());
                self.next_touch_id.set(id.0 + 1);
                self.active_touches.borrow_mut().push((pointer_id, id));
                self.send_touch(id, TouchPhase::Started, position_of(index), motion, index);
            }
            MotionAction::Move => {
                for index in 0..motion.pointer_count() {
                    if let Some(id) = self.touch_id(motion.pointer_at_index(index).pointer_id()) {
                        self.send_touch(id, TouchPhase::Moved, position_of(index), motion, index);
                    }
                }
            }
            MotionAction::Up | MotionAction::PointerUp => {
                let index = motion.pointer_index();
                if let Some(id) = self.take_touch(motion.pointer_at_index(index).pointer_id()) {
                    self.send_touch(id, TouchPhase::Ended, position_of(index), motion, index);
                }
            }
            MotionAction::Cancel => {
                for index in 0..motion.pointer_count() {
                    if let Some(id) = self.take_touch(motion.pointer_at_index(index).pointer_id()) {
                        self.send_touch(id, TouchPhase::Cancelled, position_of(index), motion, index);
                    }
                }
            }
            _ => {}
        }
    }

    fn touch_id(&self, pointer_id: i32) -> Option<TouchId> {
        self.active_touches.borrow().iter().find(|(pointer, _)| *pointer == pointer_id).map(|(_, id)| *id)
    }

    fn take_touch(&self, pointer_id: i32) -> Option<TouchId> {
        let mut touches = self.active_touches.borrow_mut();
        let index = touches.iter().position(|(pointer, _)| *pointer == pointer_id)?;
        Some(touches.remove(index).1)
    }

    fn send_touch(&self, id: TouchId, phase: TouchPhase, position: Point<Pixels>, motion: &MotionEvent<'_>, index: usize) {
        self.last_touch_position.set(position);
        let pressure = motion.pointer_at_index(index).pressure();
        self.dispatch(PlatformInput::Touch(TouchEvent {
            id,
            phase,
            position,
            predicted_position: None,
            force: (pressure > 0.).then(|| pressure.min(1.)),
        }));
    }

    pub(crate) fn should_close(&self) -> bool {
        with_callback!(self, should_close, |callback| callback()).unwrap_or(true)
    }

    pub(crate) fn close(&self) {
        let close = self.callbacks.borrow_mut().close.take();
        if let Some(close) = close {
            close();
        }
    }
}

pub(crate) struct AndroidWindow {
    state: Rc<AndroidWindowState>,
}

impl AndroidWindow {
    pub(crate) fn new(
        _handle: AnyWindowHandle,
        app: AndroidApp,
        native_window: NativeWindow,
        gpu_context: GpuContext,
        display: Rc<AndroidDisplay>,
    ) -> anyhow::Result<(Self, Rc<AndroidWindowState>)> {
        let surface = NativeSurface(native_window);
        let device_size = surface.size();
        let config = WgpuSurfaceConfig { size: device_size, transparent: false, preferred_present_mode: Some(wgpu::PresentMode::Mailbox) };
        let renderer = WgpuRenderer::new(gpu_context.clone(), &surface, config, None)?;
        let scale = scale_factor(&app);
        let size = Size { width: px(device_size.width.0 as f32 / scale), height: px(device_size.height.0 as f32 / scale) };
        display.size.set(size);
        let state = Rc::new(AndroidWindowState {
            app,
            gpu_context,
            renderer: RefCell::new(renderer),
            surface: RefCell::new(Some(surface)),
            presentation_pending: Cell::new(false),
            display,
            size: Cell::new(size),
            scale_factor: Cell::new(scale),
            insets: RefCell::default(),
            visibility: Cell::new(WindowVisibility::Visible),
            is_active: Cell::new(true),
            back_enabled: Cell::new(false),
            title: RefCell::default(),
            input_handler: RefCell::default(),
            last_touch_position: Cell::default(),
            next_touch_id: Cell::new(1),
            active_touches: RefCell::default(),
            callbacks: RefCell::default(),
        });
        state.insets_changed();
        Ok((Self { state: state.clone() }, state))
    }
}

impl HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let surface = self.state.surface.borrow();
        let raw = surface.as_ref().ok_or(HandleError::Unavailable)?.window_handle()?.as_raw();
        // SAFETY: the handle stays valid until `TerminateWindow`, after which the
        // platform no longer hands it out.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: the Android display handle carries no data, so it cannot dangle.
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Android(raw_window_handle::AndroidDisplayHandle::new())) })
    }
}

impl PlatformWindow for AndroidWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        Bounds { origin: Point::default(), size: self.state.size.get() }
    }

    fn is_maximized(&self) -> bool {
        true
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Fullscreen(self.bounds())
    }

    fn content_size(&self) -> Size<Pixels> {
        self.state.size.get()
    }

    // The activity's size is decided by the system (split screen, freeform windows).
    fn resize(&mut self, _size: Size<Pixels>) {}

    fn scale_factor(&self) -> f32 {
        self.state.scale_factor.get()
    }

    fn appearance(&self) -> WindowAppearance {
        appearance(&self.state.app)
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.state.display.clone())
    }

    fn mouse_position(&self) -> Point<Pixels> {
        self.state.last_touch_position.get()
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers::default()
    }

    fn capslock(&self) -> Capslock {
        Capslock::default()
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        *self.state.input_handler.borrow_mut() = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.state.input_handler.borrow_mut().take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        None
    }

    fn activate(&self) {}

    fn is_active(&self) -> bool {
        self.state.is_active.get()
    }

    fn visibility(&self) -> WindowVisibility {
        self.state.visibility.get()
    }

    fn is_hovered(&self) -> bool {
        false
    }

    fn background_appearance(&self) -> WindowBackgroundAppearance {
        WindowBackgroundAppearance::Opaque
    }

    fn set_title(&mut self, title: &str) {
        *self.state.title.borrow_mut() = title.to_owned();
    }

    fn get_title(&self) -> String {
        self.state.title.borrow().clone()
    }

    fn set_background_appearance(&self, _background_appearance: WindowBackgroundAppearance) {}

    fn minimize(&self) {}

    fn zoom(&self) {}

    fn toggle_fullscreen(&self) {}

    fn is_fullscreen(&self) -> bool {
        true
    }

    fn frame_waker(&self) -> Option<Rc<dyn Fn()>> {
        let waker = self.state.app.create_waker();
        Some(Rc::new(move || waker.wake()))
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.state.callbacks.borrow_mut().request_frame = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        self.state.callbacks.borrow_mut().input = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.state.callbacks.borrow_mut().active_status_change = Some(callback);
    }

    fn on_visibility_change(&self, callback: Box<dyn FnMut(WindowVisibility)>) {
        self.state.callbacks.borrow_mut().visibility_change = Some(callback);
    }

    fn on_hover_status_change(&self, _callback: Box<dyn FnMut(bool)>) {}

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        self.state.callbacks.borrow_mut().resize = Some(callback);
    }

    fn on_moved(&self, _callback: Box<dyn FnMut()>) {}

    fn on_display_changed(&self, _callback: Box<dyn FnMut()>) {}

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.state.callbacks.borrow_mut().should_close = Some(callback);
    }

    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {}

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.state.callbacks.borrow_mut().close = Some(callback);
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.state.callbacks.borrow_mut().appearance_changed = Some(callback);
    }

    fn draw(&self, scene: &Scene) {
        if self.state.surface.borrow().is_some() && self.state.renderer.borrow_mut().draw(scene) {
            self.state.presentation_pending.set(false);
        }
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.state.renderer.borrow().sprite_atlas().clone()
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        false
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.state.renderer.borrow().gpu_specs()
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}

    fn window_decorations(&self) -> Decorations {
        Decorations::Server
    }

    fn window_controls(&self) -> WindowControls {
        WindowControls { fullscreen: false, maximize: false, minimize: false, window_menu: false }
    }

    fn insets(&self) -> WindowInsets {
        self.state.insets.borrow().clone()
    }

    fn on_insets_changed(&self, callback: Box<dyn FnMut(WindowInsets)>) {
        self.state.callbacks.borrow_mut().insets_changed = Some(callback);
    }

    fn set_back_handler(&self, callback: Box<dyn FnMut()>) {
        self.state.callbacks.borrow_mut().back = Some(callback);
    }

    fn set_back_enabled(&self, enabled: bool) {
        self.state.back_enabled.set(enabled);
    }

    fn show_soft_keyboard(&self) {
        self.state.app.show_soft_input(true);
    }

    fn hide_soft_keyboard(&self) {
        self.state.app.hide_soft_input(false);
    }
}

pub(crate) fn appearance(app: &AndroidApp) -> WindowAppearance {
    match app.config().ui_mode_night() {
        ndk::configuration::UiModeNight::Yes => WindowAppearance::Dark,
        _ => WindowAppearance::Light,
    }
}
