//! macOS (Cocoa) parent window for embedded plugin editors.
//! CLAP cocoa wants an `NSView*`; AU CocoaUI wants one too.
//!
//! AppKit requires UI work on the main thread — the session worker *is* the
//! main thread, so plain calls are legal here. The session drains AppKit's
//! event queue even when the plugin owns a floating window.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSBackingStoreType, NSView, NSWindow, NSWindowDelegate, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSDate, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

/// Delegate state: a plain close flag flipped by `windowShouldClose:`.
struct CloseFlag {
    dead: std::cell::Cell<bool>,
}

pub fn initialize() -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("AppKit requires the main thread")?;
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Regular);
    app.finishLaunching();
    Ok(())
}

pub fn pump_events() -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("AppKit events require the main thread")?;
    objc2::rc::autoreleasepool(|_| {
        let app = NSApplication::sharedApplication(mtm);
        loop {
            let event = unsafe {
                app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    objc2_app_kit::NSEventMask::Any,
                    Some(&NSDate::distantPast()),
                    objc2_foundation::NSDefaultRunLoopMode,
                    true,
                )
            };
            match event {
                Some(event) => app.sendEvent(&event),
                None => break,
            }
        }
        app.updateWindows();
    });
    Ok(())
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "VerdantWindowDelegate"]
    #[ivars = CloseFlag]
    #[thread_kind = MainThreadOnly]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl NSWindowDelegate for Delegate {
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _sender: &NSWindow) -> bool {
            self.ivars().dead.set(true);
            // Detach the plugin view before destroying its native parent.
            false
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(CloseFlag {
            dead: std::cell::Cell::new(false),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub struct CocoaWindow {
    win: Retained<NSWindow>,
    view: Retained<NSView>,
    delegate: Retained<Delegate>,
}

impl CocoaWindow {
    pub fn open(title: &str) -> Result<Self, String> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Err("cocoa window must be created on the main thread".into());
        };
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Regular);

        let rect = NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(480.0, 320.0));
        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Resizable;
        let win = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        win.setTitle(&NSString::from_str(title));
        unsafe { win.setReleasedWhenClosed(false); }
        let delegate = Delegate::new(mtm);
        win.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        let view = win
            .contentView()
            .ok_or("no content view")?;
        win.makeKeyAndOrderFront(None);
        Ok(Self {
            win,
            view,
            delegate,
        })
    }

    pub fn content_view(&self) -> *mut std::ffi::c_void {
        objc2::rc::Retained::as_ptr(&self.view) as *mut std::ffi::c_void
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.win.setContentSize(NSSize::new(width.max(1) as f64, height.max(1) as f64));
    }

    pub fn size(&self) -> (u32, u32) {
        let size = self.view.frame().size;
        (size.width.max(1.0) as u32, size.height.max(1.0) as u32)
    }
    pub fn show(&mut self, visible: bool) {
        if visible { self.win.makeKeyAndOrderFront(None); }
        else { self.win.orderOut(None); }
    }

    /// Native events have already been dispatched by the session pump.
    pub fn pump(&mut self) -> bool {
        !self.delegate.ivars().dead.get()
    }
}

impl Drop for CocoaWindow {
    fn drop(&mut self) {
        self.win.setDelegate(None);
        self.win.close();
    }
}
