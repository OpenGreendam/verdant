//! X11 parent window for embedded plugin editors (CLAP x11, VST3 X11EmbedWindowID).
//! Uses `x11rb` (MIT): connects to `$DISPLAY`, creates a simple top-level
//! window, maps it, and pumps events non-blockingly for close detection.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::wrapper::ConnectionExt as _;

pub struct X11Window {
    conn: RustConnection,
    win: u32,
    wm_delete: u32,
    /// `WM_DELETE_WINDOW` seen → the user asked to close.
    dead: bool,
    size: (u32, u32),
}

impl X11Window {
    pub fn open(title: &str) -> Result<Self, String> {
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| format!("x11 connect: {e}"))?;
        let screen = &conn.setup().roots[screen_num];
        let win = conn.generate_id().map_err(|e| e.to_string())?;
        let title_bytes = title.as_bytes();
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            win,
            screen.root,
            0,
            0,
            480,
            320,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixel(screen.white_pixel)
                .event_mask(EventMask::STRUCTURE_NOTIFY),
        )
        .map_err(|e| e.to_string())?;
        conn.change_property8(
            PropMode::REPLACE,
            win,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title_bytes,
        )
        .map_err(|e| e.to_string())?;

        // Ask the WM for a polite close event instead of killing us.
        let wm_protocols = conn
            .intern_atom(false, b"WM_PROTOCOLS")
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .atom;
        let wm_delete = conn
            .intern_atom(false, b"WM_DELETE_WINDOW")
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .atom;
        conn.change_property32(
            PropMode::REPLACE,
            win,
            wm_protocols,
            AtomEnum::ATOM,
            &[wm_delete],
        )
        .map_err(|e| e.to_string())?;

        conn.map_window(win).map_err(|e| e.to_string())?;
        conn.flush().map_err(|e| e.to_string())?;
        Ok(Self {
            conn,
            win,
            wm_delete,
            dead: false,
            size: (480, 320),
        })
    }

    pub fn window_id(&self) -> u32 {
        self.win
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width.max(1), height.max(1));
        let _ = self.conn.configure_window(
            self.win,
            &ConfigureWindowAux::new().width(width).height(height),
        );
        let _ = self.conn.flush();
    }

    pub fn size(&self) -> (u32, u32) { self.size }
    pub fn show(&mut self, visible: bool) {
        if visible { let _ = self.conn.map_window(self.win); }
        else { let _ = self.conn.unmap_window(self.win); }
        let _ = self.conn.flush();
    }

    /// Pump events; `false` when the window is gone / close requested.
    pub fn pump(&mut self) -> bool {
        if self.dead {
            return false;
        }
        loop {
            match self.conn.poll_for_event() {
                Ok(Some(Event::ClientMessage(ev))) => {
                    if ev.window == self.win && ev.data.as_data32()[0] == self.wm_delete {
                        self.dead = true;
                        return false;
                    }
                }
                Ok(Some(Event::ConfigureNotify(ev))) if ev.window == self.win => {
                    self.size = (ev.width as u32, ev.height as u32);
                }
                Ok(Some(Event::DestroyNotify(ev))) => {
                    if ev.window == self.win {
                        self.dead = true;
                        return false;
                    }
                }
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(_) => {
                    self.dead = true;
                    return false;
                }
            }
        }
        true
    }
}

impl Drop for X11Window {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.win);
        let _ = self.conn.flush();
    }
}
