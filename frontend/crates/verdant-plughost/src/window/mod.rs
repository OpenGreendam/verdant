//! Native parent windows for embedded plugin editors. Each format maps the
//! `NativeHandle` to its own API (CLAP `clap_window`, VST3 `WindowHandle`,
//! AU NSView).
//!
//! All calls run on the process main thread. The session pumps native events
//! independently of host-created parent windows, including floating editors.

#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "macos")]
mod cocoa;
#[cfg(target_os = "windows")]
mod win32;

/// Platform-native handle handed to the plugin's embedding API.
#[derive(Clone, Copy)]
pub enum NativeHandle {
    /// X11 window id (`clap_window.x11` / `X11EmbedWindowID`).
    X11(std::ffi::c_ulong),
    /// Win32 `HWND`.
    Win32(*mut std::ffi::c_void),
    /// Cocoa `NSView *`.
    Cocoa(*mut std::ffi::c_void),
}

/// Initialize AppKit on the process main thread before loading plugins that
/// may query NSApp during initialization. Other native APIs need no bootstrap.
pub fn initialize() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    cocoa::initialize()?;
    Ok(())
}

pub(crate) fn pump_events() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    cocoa::pump_events()?;
    #[cfg(target_os = "windows")]
    win32::pump_events()?;
    // X11 plugins own their connection/event dispatch. CLAP's registered
    // POSIX descriptors and timers are serviced by the format host.
    Ok(())
}

/// A host-created top-level window that can parent a plugin editor.
pub struct EditorWindow {
    #[cfg(target_os = "linux")]
    imp: x11::X11Window,
    #[cfg(target_os = "macos")]
    imp: cocoa::CocoaWindow,
    #[cfg(target_os = "windows")]
    imp: win32::Win32Window,
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    imp: (),
}

impl EditorWindow {
    /// Create + map a plain top-level window titled `title`.
    pub fn open(title: &str) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            return x11::X11Window::open(title).map(|imp| Self { imp });
        }
        #[cfg(target_os = "macos")]
        {
            return cocoa::CocoaWindow::open(title).map(|imp| Self { imp });
        }
        #[cfg(target_os = "windows")]
        {
            return win32::Win32Window::open(title).map(|imp| Self { imp });
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let _ = title;
            Err("native editor windows unsupported on this platform".into())
        }
    }

    /// The plugin-facing native handle of this window's content area.
    pub fn native_handle(&self) -> NativeHandle {
        #[cfg(target_os = "linux")]
        {
            NativeHandle::X11(self.imp.window_id().into())
        }
        #[cfg(target_os = "macos")]
        {
            NativeHandle::Cocoa(self.imp.content_view())
        }
        #[cfg(target_os = "windows")]
        {
            NativeHandle::Win32(self.imp.hwnd())
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            unreachable!()
        }
    }

    /// Resize the client area to `width`×`height` (best effort).
    pub fn resize(&mut self, width: u32, height: u32) {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        self.imp.resize(width, height);
    }

    pub fn size(&self) -> (u32, u32) {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        { self.imp.size() }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        { (0, 0) }
    }

    pub fn show(&mut self, visible: bool) {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        self.imp.show(visible);
    }

    /// Pump pending window events; `false` = the user closed the window.
    /// Cheap; called from the session's idle service pass.
    pub fn pump(&mut self) -> bool {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            self.imp.pump()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }

    /// CLAP `clap_window` for `set_parent` — feature-gated helper so this
    /// module doesn't depend on clack.
    #[cfg(feature = "clap")]
    pub fn clap_window(&self) -> clack_extensions::gui::Window<'static, 'static> {
        match self.native_handle() {
            NativeHandle::X11(id) => clack_extensions::gui::Window::from_x11_handle(id),
            NativeHandle::Win32(h) => unsafe {
                clack_extensions::gui::Window::from_win32_hwnd(h)
            },
            NativeHandle::Cocoa(v) => unsafe {
                clack_extensions::gui::Window::from_cocoa_nsview(v)
            },
        }
    }

    /// vst3-host `WindowHandle` for `Plugin::open_editor`.
    #[cfg(feature = "vst3")]
    pub fn vst3_window_handle(&self) -> vst3_host::plugin::WindowHandle {
        match self.native_handle() {
            #[cfg(target_os = "linux")]
            NativeHandle::X11(id) => vst3_host::plugin::WindowHandle::from_x11(id as u32),
            _ => unsafe { vst3_host::plugin::WindowHandle::from_raw(match self.native_handle() {
                NativeHandle::Win32(h) => h,
                NativeHandle::Cocoa(v) => v,
                NativeHandle::X11(id) => id as usize as *mut std::ffi::c_void,
            }) },
        }
    }
}
