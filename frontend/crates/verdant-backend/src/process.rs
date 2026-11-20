//! The Rust worker process: `verdant-worker …` over stdin/stdout, one JSON per
//! line. stdout is protocol only; stderr is kept as a bounded tail for
//! diagnostics and never parsed for state (SPEC §12).

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::wire::{ParseError, Request, WorkerEvent, parse_event};

/// Longest stdout line accepted from the worker (completed events list
/// every artifact; they are small, this is a defensive bound).
pub const MAX_EVENT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct WorkerConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub workspace: PathBuf,
    pub label: String,
}

impl WorkerConfig {
    /// `verdant-worker --workspace <ws> [extra…]`.
    /// The caller resolves the owned executable; no interpreter or script is used.
    pub fn native_worker(program: impl Into<PathBuf>, workspace: impl Into<PathBuf>, extra: &[&str]) -> Self {
        let program = program.into();
        let workspace = workspace.into();
        let mut args = vec!["--workspace".into(), workspace.display().to_string()];
        args.extend(extra.iter().map(|s| s.to_string()));
        WorkerConfig { label: program.display().to_string(), program, args, workspace }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkerMessage {
    Event(WorkerEvent),
    /// A stdout line that is not a valid protocol record (shown, not obeyed).
    Malformed {
        line: String,
        error: String,
    },
    Exited {
        status: String,
    },
}

pub struct WorkerClient {
    pub generation: u64,
    pub config: WorkerConfig,
    child: Arc<Mutex<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    pub stderr_tail: Arc<Mutex<VecDeque<String>>>,
}

impl WorkerClient {
    /// Spawn the worker; messages arrive tagged with `generation` so late
    /// output from a previous process can be discarded by the receiver.
    pub fn spawn(config: WorkerConfig, generation: u64) -> std::io::Result<(Self, async_channel::Receiver<(u64, WorkerMessage)>)> {
        std::fs::create_dir_all(&config.workspace)?;
        let mut child =
            Command::new(&config.program).args(&config.args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let stdin = child.stdin.take();
        let child = Arc::new(Mutex::new(child));
        let (tx, rx) = async_channel::unbounded();
        let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));

        let tail = stderr_tail.clone();
        std::thread::Builder::new().name("ogd-stderr".into()).spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut t = tail.lock().unwrap();
                if t.len() >= 200 {
                    t.pop_front();
                }
                t.push_back(line);
            }
        })?;

        let reaper = child.clone();
        std::thread::Builder::new().name("ogd-stdout".into()).spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                let n = match (&mut reader).take(MAX_EVENT_BYTES as u64 + 1).read_until(b'\n', &mut buf) {
                    Ok(n) => n,
                    Err(e) => {
                        let _ = tx.send_blocking((generation, WorkerMessage::Malformed { line: String::new(), error: e.to_string() }));
                        break;
                    }
                };
                if n == 0 {
                    break;
                }
                if buf.len() > MAX_EVENT_BYTES {
                    let _ = tx.send_blocking((
                        generation,
                        WorkerMessage::Malformed { line: "<oversized>".into(), error: "event line exceeds 4 MiB".into() },
                    ));
                    break;
                }
                let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                if line.is_empty() {
                    continue;
                }
                let msg = match parse_event(&line) {
                    Ok(ev) => WorkerMessage::Event(ev),
                    Err(e @ (ParseError::Json(_) | ParseError::Protocol(_) | ParseError::Field { .. })) => {
                        WorkerMessage::Malformed { line, error: e.to_string() }
                    }
                };
                if tx.send_blocking((generation, msg)).is_err() {
                    break;
                }
            }
            // The supervisor can still kill a child that closed stdout and
            // has yet to exit. Do not hold the child lock while waiting.
            let status = loop {
                let status = reaper.lock().unwrap().try_wait();
                match status {
                    Ok(Some(status)) => break status.to_string(),
                    Err(error) => break error.to_string(),
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                }
            };
            let _ = tx.send_blocking((generation, WorkerMessage::Exited { status }));
        })?;

        Ok((WorkerClient { generation, config, child, stdin: Mutex::new(stdin), stderr_tail }, rx))
    }

    pub fn send(&self, request: &Request) -> Result<(), String> {
        let line = request.to_line()?;
        let mut guard = self.stdin.lock().unwrap();
        let stdin = guard.as_mut().ok_or("worker stdin closed")?;
        stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()).map_err(|e| e.to_string())
    }

    /// Close stdin (the worker treats EOF as shutdown and cleans up jobs).
    pub fn close(&self) {
        self.stdin.lock().unwrap().take();
    }

    pub fn kill(&self) {
        let _ = self.child.lock().unwrap().kill();
    }

    pub fn pid(&self) -> u32 {
        self.child.lock().unwrap().id()
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        self.close();
    }
}
