//! `verdant-plughost` — one plugin host process per live insert.
//!
//! Two entry modes (see `main`):
//! - `verdant-plughost scan-file <path>` — probe one plugin bundle/file and
//!   print a `ScanFileReport` JSON line. Short-lived, timeout-guarded by the
//!   caller (`verdant-plugins` scan).
//! - `verdant-plughost host --channel fd:9,10,11 | name:s,s,s` — run a hosting
//!   session: line-JSON control on stdin/stdout, audio over shared memory.
//!
//! The session and all plugin lifecycle/editor work run on the process main
//! thread. Pipe I/O uses separate threads; audio is copied over mmap.
//! `ProtocolOutput::capture` must reserve the inherited stdout pipe before any
//! plugin loads. Global stdout then carries diagnostics to stderr, while scan
//! reports and control messages use only that owned protocol handle.

pub mod control;
pub mod formats;
pub mod protocol_output;
pub mod session;
pub mod window;

pub use session::run_host;
pub use formats::scan_file;
