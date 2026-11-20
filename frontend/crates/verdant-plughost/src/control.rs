//! Control-plane plumbing: stdin requests → worker, worker replies/events →
//! the reserved protocol output. A `Wake` pair lets the session worker poll
//! commands and the audio doorbell on one `wait_any` call.

use crate::protocol_output::ProtocolOutput;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use verdant_plugins::proto::{self, HostMessage, HostRequest};
use verdant_plugins::shm::Wake;

/// Both halves the session loop needs.
pub struct ControlPipe {
    /// Requests read from stdin.
    pub requests: Receiver<HostRequest>,
    /// Readable whenever `requests` has new items (poll alongside the audio
    /// doorbell).
    pub cmd_ready: Wake,
    /// All outgoing messages (replies + events); drained by the writer thread.
    pub outgoing: Sender<HostMessage>,
    /// Shared shutdown flag between reader/writer threads.
    pub stop: Arc<AtomicBool>,
}

/// Spawn the stdin reader + reserved protocol writer threads.
/// `stdin` EOF → the requests channel disconnects → session exits.
pub fn spawn(mut output: ProtocolOutput) -> ControlPipe {
    let (req_tx, requests) = channel::<HostRequest>();
    let (out_tx, outgoing_rx) = channel::<HostMessage>();
    let (cmd_ready, cmd_poke) = Wake::pair().expect("cmd wake pair");
    let stop = Arc::new(AtomicBool::new(false));
    let stop_r = stop.clone();

    std::thread::Builder::new()
        .name("ph-stdin".into())
        .spawn(move || {
            let stdin = io::stdin();
            let mut lock = stdin.lock();
            loop {
                match proto::read_host_request(&mut lock) {
                    Ok(Some(req)) => {
                        if req_tx.send(req).is_err() {
                            return;
                        }
                        cmd_poke.signal();
                    }
                    Ok(None) | Err(_) => {
                        drop(req_tx);
                        cmd_poke.signal(); // wake worker so it sees EOF
                        stop_r.store(true, Ordering::Release);
                        return;
                    }
                }
            }
        })
        .expect("spawn ph-stdin");

    let stop_w = stop.clone();
    std::thread::Builder::new()
        .name("ph-stdout".into())
        .spawn(move || {
            for msg in outgoing_rx {
                if let Err(error) = output.write_line(&msg) {
                    eprintln!("verdant-plughost control output: {error}");
                    stop_w.store(true, Ordering::Release);
                    return;
                }
            }
        })
        .expect("spawn ph-stdout");

    ControlPipe {
        requests,
        cmd_ready,
        outgoing: out_tx,
        stop,
    }
}

/// Reply helper.
pub fn reply(
    id: u64,
    ok: bool,
    error: Option<String>,
    data: serde_json::Map<String, serde_json::Value>,
) -> HostMessage {
    HostMessage::Reply { id, ok, error, data }
}

/// Event helper.
pub fn event(name: &str, data: serde_json::Map<String, serde_json::Value>) -> HostMessage {
    HostMessage::Event {
        event: name.into(),
        data,
    }
}
