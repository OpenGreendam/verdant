//! Win32 parent window for embedded plugin editors (CLAP win32 / VST3 HWND).
//! The session pumps the thread queue for embedded and floating editors.

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{UpdateWindow, HBRUSH, COLOR_WINDOW};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

pub fn pump_events() -> Result<(), String> {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == WM_QUIT { return Err("native editor requested host shutdown".into()); }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

pub struct Win32Window {
    hwnd: HWND,
    dead: Box<std::cell::Cell<bool>>,
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_CLOSE | WM_DESTROY => {
                // Mark closed via GWLP_USERDATA pointer the owner set.
                let flag = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const std::cell::Cell<bool>;
                if !flag.is_null() {
                    (*flag).set(true);
                }
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

impl Win32Window {
    pub fn open(title: &str) -> Result<Self, String> {
        unsafe {
            let class = wide("VerdantPlughostEditor");
            let hinst = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinst,
                hIcon: std::ptr::null_mut(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                // ERROR_CLASS_ALREADY_EXISTS = 1410 — fine on second window.
            }
            let mut this = Self {
                hwnd: std::ptr::null_mut(),
                dead: Box::new(std::cell::Cell::new(false)),
            };
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                wide(title).as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                480,
                320,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinst,
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW failed".into());
            }
            this.hwnd = hwnd;
            // Hand `dead` to the wndproc through GWLP_USERDATA (raw pointer to
            // the boxed flag — the box outlives the window).
            let flag_ptr = &*this.dead as *const std::cell::Cell<bool>;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, flag_ptr as isize);
            let _ = UpdateWindow(hwnd);
            Ok(this)
        }
    }

    pub fn hwnd(&self) -> *mut std::ffi::c_void {
        self.hwnd as *mut std::ffi::c_void
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        unsafe {
            let mut rect = RECT { left: 0, top: 0, right: width.max(1) as i32, bottom: height.max(1) as i32 };
            AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, 0);
            let _ = SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOMOVE | SWP_NOZORDER,
            );
        }
    }

    pub fn size(&self) -> (u32, u32) {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.hwnd, &mut rect); }
        ((rect.right - rect.left).max(1) as u32, (rect.bottom - rect.top).max(1) as u32)
    }
    pub fn show(&mut self, visible: bool) {
        unsafe { ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE }); }
    }

    pub fn pump(&mut self) -> bool {
        !self.dead.get()
    }
}

impl Drop for Win32Window {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_null() {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                DestroyWindow(self.hwnd);
            }
        }
    }
}
