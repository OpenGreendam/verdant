//! Shared-memory audio ring between the Verdant process and one
//! `verdant-plughost` child, plus the two wake channels.
//!
//! Layout (all atomics naturally aligned; every field is `u32`/`u64` so both
//! processes agree on the wire format):
//!
//! ```text
//! offset 0          RingHeader (64 bytes, cache-line aligned)
//! offset 64         in_slot[0..64] then out_slot[0..64]
//! input stride      align64(24 + 8*max_frames + 16*MAX_PARAM_EVENTS)
//! output stride     align64(24 + 8*max_frames)
//! slot payload      frames/sequence/event_count + L/R + native parameter events
//! ```
//!
//! Audio and parameter events share one slot claim and publication. Events
//! carry quantum-relative sample offsets and are never sent over control IPC.
//!
//! Protocol — `in_seq`/`out_seq` count completed blocks (0 at init):
//! - RT writer: fill `in_slot[n % RING_SLOTS]`, publish `in_seq`, wake host.
//! - Host: drain complete input quanta in order, publish corresponding output.
//! - RT reader consumes output 37 quanta later. The delay covers an entire
//!   8192-frame device callback at >=44.1 kHz, source prefetch, and partial DSP
//!   alignment at the fixed 48 kHz processing rate. A callback never
//!   depends on DSP for audio it submitted during that same callback.
//!
//! Each slot has an atomic ownership word and an exact sequence tag. A
//! producer/consumer attempts one compare-exchange and skips on contention;
//! neither side ever copies a payload another side can mutate. A stale slot
//! is discarded, never mistaken for the requested block after slot wrap.

use crate::error::{PluginError, Result};
use crate::proto::{validate_event_quanta, AutomationError, ParamEvent, MAX_PARAM_EVENTS};
use std::io;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// `"VRPL"` — sanity check when a child maps a descriptor it was handed.
pub const MAGIC: u32 = 0x5652_504C;
pub const RING_VERSION: u32 = 5;
/// Hard cap matching `verdant_audio::device::MAX_BLOCK` — oversized blocks are
/// chunked by the caller before they ever reach the ring.
pub const MAX_BLOCK_FRAMES: usize = 8192;
/// Native DSP quantum, independent of physical device callback size.
pub const DSP_QUANTUM_FRAMES: usize = 256;
/// Bounded input/output history, large enough for whole engine callback bursts.
pub const RING_SLOTS: usize = 64;
/// Covers 8192 device frames at >=44.1 kHz, the output adapter's 128-frame
/// source prefetch/16-frame priming, and partial 256-frame DSP alignment.
pub const TRANSPORT_LATENCY_FRAMES: usize = 37 * DSP_QUANTUM_FRAMES;
pub const TRANSPORT_QUANTA: u64 = (TRANSPORT_LATENCY_FRAMES / DSP_QUANTUM_FRAMES) as u64;

pub const STATUS_OK: u32 = 0;
/// Plugin holds a stale sample rate after a device change; self-bypassed.
pub const STATUS_STALE_RATE: u32 = 1;
/// Host process died or was declared dead by the supervisor.
pub const STATUS_CRASHED: u32 = 2;
/// Slot kept as a missing-plugin placeholder (never instantiated).
pub const STATUS_MISSING: u32 = 3;

#[repr(C, align(64))]
pub struct RingHeader {
    pub magic: AtomicU32,
    pub version: AtomicU32,
    pub max_frames: AtomicU32,
    pub status: AtomicU32,
    /// Plugin-reported latency in samples (host-written).
    pub plugin_latency: AtomicU32,
    /// Host's own pid, for diagnostics (host-written).
    pub host_pid: AtomicU32,
    /// App sets → host exits its loop even if pipes linger.
    pub shutdown: AtomicU32,
    pub _pad: AtomicU32,
    /// Number of input blocks published.
    pub in_seq: AtomicU64,
    /// Number of output blocks published.
    pub out_seq: AtomicU64,
}

const HEADER_SIZE: usize = 64;

/// One audio block: frame count + deinterleaved L/R planes.
/// `data` is `l[0..max] ++ r[0..max]` — a single copy per plane.
#[repr(C)]
struct Slot {
    state: AtomicU32,
    frames: std::cell::UnsafeCell<u32>,
    seq: std::cell::UnsafeCell<u64>,
    event_count: std::cell::UnsafeCell<u32>,
    /// Input slots only: 1 when the sidechain planes carry a key signal.
    sidechain: std::cell::UnsafeCell<u32>,
    // L/R planes follow; input slots add sidechain L/R planes and then
    // WireParamEvent storage.
}

const SLOT_HEADER_SIZE: usize = 24;

/// Explicit C layout: tuple representation is never part of the wire ABI.
#[repr(C)]
#[derive(Clone, Copy)]
struct WireParamEvent {
    native_id: u32,
    sample_offset: u32,
    value: f64,
}

/// Input slots carry four planes (main L/R, sidechain L/R); output two.
fn planes(input: bool) -> usize { if input { 4 } else { 2 } }

fn event_offset(max_frames: usize) -> usize {
    SLOT_HEADER_SIZE + planes(true) * max_frames * 4
}

fn slot_stride(max_frames: usize, input: bool) -> usize {
    let size = if input { event_offset(max_frames) + MAX_PARAM_EVENTS * std::mem::size_of::<WireParamEvent>() }
        else { SLOT_HEADER_SIZE + planes(false) * max_frames * 4 };
    size.div_ceil(64) * 64
}

/// Borrowed sidechain (key) planes for one input publication.
pub type Sidechain<'a> = Option<(&'a [f32], &'a [f32])>;

fn output_base(max_frames: usize) -> usize {
    HEADER_SIZE + RING_SLOTS * slot_stride(max_frames, true)
}

fn ring_len(max_frames: usize) -> usize {
    output_base(max_frames) + RING_SLOTS * slot_stride(max_frames, false)
}

/// Mapped ring shared with one host process. `Send + Sync` is sound here:
/// atomics transfer exclusive slot ownership before any non-atomic payload
/// access. Failed claims leave the payload untouched.
pub struct Ring {
    ptr: *mut u8,
    len: usize,
    imp: imp::RingImp,
}

unsafe impl Send for Ring {}
unsafe impl Sync for Ring {}

impl Ring {
    /// Create + initialize a fresh ring (app side).
    pub fn create(max_frames: usize) -> Result<Self> {
        if max_frames == 0 || max_frames > MAX_BLOCK_FRAMES {
            return Err(PluginError::Shm(format!(
                "max_frames {max_frames} out of range 1..={MAX_BLOCK_FRAMES}"
            )));
        }
        let ring = imp::create_ring(max_frames)?;
        ring.init(max_frames);
        Ok(ring)
    }

    /// Initialize the shared header; caller must own the (fresh) mapping.
    fn init(&self, max_frames: usize) {
        let h = self.header();
        h.magic.store(MAGIC, Ordering::Relaxed);
        h.version.store(RING_VERSION, Ordering::Relaxed);
        h.max_frames.store(max_frames as u32, Ordering::Relaxed);
        h.status.store(STATUS_OK, Ordering::Relaxed);
        h.plugin_latency.store(0, Ordering::Relaxed);
        h.host_pid.store(0, Ordering::Relaxed);
        h.shutdown.store(0, Ordering::Relaxed);
        h.in_seq.store(0, Ordering::Relaxed);
        h.out_seq.store(0, Ordering::Relaxed);
        for base in [HEADER_SIZE, output_base(max_frames)] {
            for index in 0..RING_SLOTS {
                unsafe {
                    let ptr = self.slot_ptr(base, index) as *mut Slot;
                    ptr.write(Slot {
                        state: AtomicU32::new(0),
                        frames: std::cell::UnsafeCell::new(0),
                        seq: std::cell::UnsafeCell::new(0),
                        event_count: std::cell::UnsafeCell::new(0),
                        sidechain: std::cell::UnsafeCell::new(0),
                    });
                }
            }
        }
    }

    /// Attach to a ring created by the parent (child side).
    pub fn attach(conn: &ShmConn) -> Result<Self> {
        let (ptr, len, imp) = imp::map_existing(conn)?;
        let ring = Self { ptr, len, imp };
        let h = ring.header();
        if h.magic.load(Ordering::Acquire) != MAGIC
            || h.version.load(Ordering::Acquire) != RING_VERSION
        {
            return Err(PluginError::Shm("bad ring magic/version".into()));
        }
        let max = h.max_frames.load(Ordering::Relaxed) as usize;
        if max == 0 || max > MAX_BLOCK_FRAMES || ring_len(max) != len {
            return Err(PluginError::Shm("ring size mismatch".into()));
        }
        Ok(ring)
    }

    #[inline]
    fn header(&self) -> &RingHeader {
        // SAFETY: layout fixed by construction; mapping outlives `self`.
        unsafe { &*(self.ptr as *const RingHeader) }
    }

    pub fn max_frames(&self) -> usize {
        self.header().max_frames.load(Ordering::Relaxed) as usize
    }

    #[inline]
    pub fn status(&self) -> u32 {
        self.header().status.load(Ordering::Acquire)
    }
    #[inline]
    pub fn set_status(&self, v: u32) {
        self.header().status.store(v, Ordering::Release);
    }
    #[inline]
    pub fn plugin_latency(&self) -> u32 {
        self.header().plugin_latency.load(Ordering::Acquire)
    }
    #[inline]
    pub fn set_plugin_latency(&self, v: u32) {
        self.header().plugin_latency.store(v, Ordering::Release);
    }
    #[inline]
    pub fn set_host_pid(&self, v: u32) {
        self.header().host_pid.store(v, Ordering::Release);
    }
    #[inline]
    pub fn request_shutdown(&self) {
        self.header().shutdown.store(1, Ordering::Release);
    }
    #[inline]
    pub fn shutdown_requested(&self) -> bool {
        self.header().shutdown.load(Ordering::Acquire) != 0
    }
    #[inline]
    pub fn in_seq(&self) -> u64 {
        self.header().in_seq.load(Ordering::Acquire)
    }
    #[inline]
    pub fn out_seq(&self) -> u64 {
        self.header().out_seq.load(Ordering::Acquire)
    }

    #[inline]
    fn slot_ptr(&self, base: usize, index: usize) -> *mut u8 {
        // SAFETY: callers reduce the sequence modulo RING_SLOTS.
        unsafe { self.ptr.add(base + index * slot_stride(self.max_frames(), base == HEADER_SIZE)) }
    }

    fn try_write(&self, base: usize, seq: u64, l: &[f32], r: &[f32], sidechain: Sidechain<'_>, events: &[ParamEvent], source_offset: u32) -> bool {
        let max = self.max_frames();
        if l.len() != r.len() || l.is_empty() || l.len() > max || events.len() > MAX_PARAM_EVENTS
            || (base != HEADER_SIZE && sidechain.is_some())
            || sidechain.is_some_and(|(sl, sr)| sl.len() != l.len() || sr.len() != l.len()) {
            return false;
        }
        unsafe {
            let ptr = self.slot_ptr(base, (seq % RING_SLOTS as u64) as usize);
            let slot = &*(ptr as *const Slot);
            let old = slot.state.load(Ordering::Relaxed);
            // 0 empty, 1 producer-owned, 2 ready, 3 consumer-owned.
            if !matches!(old, 0 | 2)
                || slot.state.compare_exchange(old, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
                return false;
            }
            let data = ptr.add(SLOT_HEADER_SIZE) as *mut f32;
            std::ptr::copy_nonoverlapping(l.as_ptr(), data, l.len());
            std::ptr::copy_nonoverlapping(r.as_ptr(), data.add(max), r.len());
            if let Some((sl, sr)) = sidechain {
                std::ptr::copy_nonoverlapping(sl.as_ptr(), data.add(2 * max), sl.len());
                std::ptr::copy_nonoverlapping(sr.as_ptr(), data.add(3 * max), sr.len());
            }
            *slot.sidechain.get() = sidechain.is_some() as u32;
            if !events.is_empty() {
                let wire = ptr.add(event_offset(max)) as *mut WireParamEvent;
                for (index, &(native_id, sample_offset, value)) in events.iter().enumerate() {
                    wire.add(index).write(WireParamEvent { native_id, sample_offset: sample_offset - source_offset, value });
                }
            }
            *slot.frames.get() = l.len() as u32;
            *slot.seq.get() = seq;
            *slot.event_count.get() = events.len() as u32;
            slot.state.store(2, Ordering::Release);
        }
        true
    }

    fn try_read(&self, base: usize, seq: u64, l: &mut [f32], r: &mut [f32],
        sidechain: Option<(&mut [f32], &mut [f32])>, events: &mut [ParamEvent]) -> std::result::Result<(usize, usize, bool), AutomationError> {
        unsafe {
            let ptr = self.slot_ptr(base, (seq % RING_SLOTS as u64) as usize);
            let slot = &*(ptr as *const Slot);
            if slot.state.compare_exchange(2, 3, Ordering::Acquire, Ordering::Relaxed).is_err() {
                return Ok((0, 0, false));
            }
            let tag = *slot.seq.get();
            let n = *slot.frames.get() as usize;
            if tag != seq || n == 0 || n > self.max_frames() || l.len() < n || r.len() < n {
                slot.state.store(if tag > seq { 2 } else { 0 }, Ordering::Release);
                return Ok((0, 0, false));
            }
            let count = *slot.event_count.get() as usize;
            if count > MAX_PARAM_EVENTS || (base != HEADER_SIZE && count != 0) {
                slot.state.store(0, Ordering::Release);
                return Err(AutomationError::EventCapacity);
            }
            if count > events.len() {
                // Preserve ownership of the complete publication for a caller
                // with sufficient storage. Never consume a truncated event list.
                slot.state.store(2, Ordering::Release);
                return Err(AutomationError::EventCapacity);
            }
            let wire = ptr.add(event_offset(self.max_frames())) as *const WireParamEvent;
            let mut previous = 0;
            for (index, event) in events[..count].iter_mut().enumerate() {
                let value = wire.add(index).read();
                let error = if value.sample_offset as usize >= n { Some(AutomationError::InvalidOffset) }
                    else if value.sample_offset < previous { Some(AutomationError::UnsortedEvents) }
                    else if !value.value.is_finite() { Some(AutomationError::ValueOutOfRange) }
                    else { None };
                if let Some(error) = error {
                    slot.state.store(0, Ordering::Release);
                    return Err(error);
                }
                previous = value.sample_offset;
                *event = (value.native_id, value.sample_offset, value.value);
            }
            let data = ptr.add(SLOT_HEADER_SIZE) as *const f32;
            std::ptr::copy_nonoverlapping(data, l.as_mut_ptr(), n);
            std::ptr::copy_nonoverlapping(data.add(self.max_frames()), r.as_mut_ptr(), n);
            let keyed = base == HEADER_SIZE && *slot.sidechain.get() != 0;
            if let Some((sl, sr)) = sidechain {
                if keyed && sl.len() >= n && sr.len() >= n {
                    std::ptr::copy_nonoverlapping(data.add(2 * self.max_frames()), sl.as_mut_ptr(), n);
                    std::ptr::copy_nonoverlapping(data.add(3 * self.max_frames()), sr.as_mut_ptr(), n);
                } else {
                    sl.fill(0.0);
                    sr.fill(0.0);
                }
            }
            slot.state.store(0, Ordering::Release);
            Ok((n, count, keyed))
        }
    }

    /// Allocation-free, one-attempt RT publication. Contention means xrun.
    pub fn try_write_input(&self, seq: u64, l: &[f32], r: &[f32]) -> bool {
        self.try_write_input_prevalidated(seq, l, r, &[])
    }

    /// Publish audio plus its native events as one ownership transaction.
    /// Every event must fit this quantum. Invalid input publishes nothing.
    pub fn try_write_input_events(&self, seq: u64, l: &[f32], r: &[f32],
        events: &[ParamEvent]) -> std::result::Result<(), AutomationError> {
        validate_event_quanta(events, l.len(), self.max_frames(), 0, 0)?;
        if self.try_write_input_prevalidated(seq, l, r, events) { Ok(()) }
        else { Err(AutomationError::TransportUnavailable) }
    }

    /// Internal callers have already validated every quantum in their call.
    pub(crate) fn try_write_input_prevalidated(&self, seq: u64, l: &[f32], r: &[f32], events: &[ParamEvent]) -> bool {
        self.try_write_input_rebased(seq, l, r, events, 0)
    }

    /// Rebase caller-owned events directly into the wire slot. Full quanta and
    /// export never need an intermediate parameter-event copy.
    pub(crate) fn try_write_input_rebased(&self, seq: u64, l: &[f32], r: &[f32], events: &[ParamEvent], source_offset: u32) -> bool {
        self.try_write_input_keyed(seq, l, r, None, events, source_offset)
    }

    /// As `try_write_input_rebased`, plus the sidechain key planes.
    pub(crate) fn try_write_input_keyed(&self, seq: u64, l: &[f32], r: &[f32], sidechain: Sidechain<'_>,
        events: &[ParamEvent], source_offset: u32) -> bool {
        // Preserve unread input during a full callback burst. A rejected timeline
        // sequence is not a publication; a later admitted sequence may resume it.
        if self.in_seq().saturating_sub(self.out_seq()) >= RING_SLOTS as u64 { return false; }
        if !self.try_write(HEADER_SIZE, seq, l, r, sidechain, events, source_offset) { return false; }
        self.header().in_seq.store(seq + 1, Ordering::Release);
        true
    }

    pub fn write_input(&self, seq: u64, l: &[f32], r: &[f32]) -> Result<usize> {
        if self.try_write_input(seq, l, r) { Ok(l.len()) }
        else { Err(PluginError::Shm("input slot unavailable or invalid block".into())) }
    }

    pub fn read_output(&self, seq: u64, l: &mut [f32], r: &mut [f32]) -> Result<bool> {
        Ok(self.try_read_output(seq, l, r))
    }

    pub fn try_read_output(&self, seq: u64, l: &mut [f32], r: &mut [f32]) -> bool {
        if self.out_seq() <= seq { return false; }
        let Ok((n, _, _)) = self.try_read(output_base(self.max_frames()), seq, l, r, None, &mut []) else { return false; };
        if n == 0 { return false; }
        l[n..].fill(0.0);
        r[n..].fill(0.0);
        true
    }

    pub fn read_input(&self, seq: u64, l: &mut [f32], r: &mut [f32]) -> Result<usize> {
        self.read_input_events(seq, l, r, &mut []).map(|(frames, _)| frames)
    }

    /// Host-side bounded read. An undersized event buffer is an error and
    /// leaves the full publication available, rather than losing its suffix.
    pub fn read_input_events(&self, seq: u64, l: &mut [f32], r: &mut [f32],
        events: &mut [ParamEvent]) -> Result<(usize, usize)> {
        self.read_input_keyed(seq, l, r, None, events).map(|(n, count, _)| (n, count))
    }

    /// Host-side read including the sidechain planes. Unkeyed publications
    /// zero the supplied sidechain buffers; the flag reports which it was.
    pub fn read_input_keyed(&self, seq: u64, l: &mut [f32], r: &mut [f32],
        sidechain: Option<(&mut [f32], &mut [f32])>, events: &mut [ParamEvent]) -> Result<(usize, usize, bool)> {
        if self.in_seq() <= seq { return Ok((0, 0, false)); }
        Ok(self.try_read(HEADER_SIZE, seq, l, r, sidechain, events)?)
    }

    pub fn write_output(&self, seq: u64, l: &[f32], r: &[f32]) -> Result<()> {
        if !self.try_write(output_base(self.max_frames()), seq, l, r, None, &[], 0) {
            return Err(PluginError::Shm("output slot unavailable or invalid block".into()));
        }
        self.header().out_seq.store(seq + 1, Ordering::Release);
        Ok(())
    }
}

impl Drop for Ring {
    fn drop(&mut self) { imp::unmap(self.ptr, self.len, self.imp); }
}

/// How the child attaches to the ring — platform encoding.
#[derive(Clone, Debug)]
pub enum ShmConn {
    /// Unix: inherited fd number (dup2'd in pre_exec).
    Fd(i32),
    /// Windows: `CreateFileMapping` object name (opened by name).
    Name(String),
}

/// One end of a wake channel. On Unix a pipe fd; on Windows an auto-reset
/// event HANDLE. Writer is always non-blocking: a full pipe means the peer
/// is already awake (or dead) — the signal is level-triggered by the seq
/// counters, the byte is just a doorbell.
pub struct Wake {
    imp: imp::WakeImp,
}

unsafe impl Send for Wake {}
unsafe impl Sync for Wake {}

/// How the child connects its ends.
#[derive(Clone, Debug)]
pub enum WakeConn {
    Fd(i32),
    Name(String),
}

impl Wake {
    /// Signal the peer. Never blocks; never fails hard — the seq counter is
    /// the source of truth and a missed doorbell self-heals on the next poll.
    #[inline]
    pub fn signal(&self) {
        self.imp.signal()
    }

    /// Blocking wait for a doorbell (host-side, control side only — never RT).
    pub fn wait(&self) -> io::Result<()> {
        self.imp.wait()
    }

    /// Bounded wait. `Ok(true)` = signaled, `Ok(false)` = timed out.
    pub fn wait_timeout(&self, d: Duration) -> io::Result<bool> {
        self.imp.wait_timeout(d)
    }

    /// Wait until `deadline`, draining stale doorbells. Returns early on
    /// `cond` becoming true so callers poll the seq counter, not the pipe.
    pub fn wait_until(&self, deadline: Instant, cond: impl Fn() -> bool) -> bool {
        loop {
            if cond() {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return cond();
            }
            match self.wait_timeout(deadline - now) {
                Ok(true) => self.drain(),
                Ok(false) | Err(_) => return cond(),
            }
        }
    }

    /// Non-blocking drain of pending doorbells.
    pub fn drain(&self) {
        self.imp.drain()
    }

    /// Duplicate this end into a second owned descriptor/handle. Used to give
    /// `RemoteInsert` its own end while `InsertHandle` keeps one.
    pub fn dup(&self) -> Result<Wake> {
        self.imp.dup().map(|imp| Wake { imp })
    }

    /// Create a connected (reader, writer) pair: a nonblocking pipe on Unix,
    /// a duplicated auto-reset event on Windows. Both ends are owned.
    pub fn pair() -> Result<(Wake, Wake)> {
        imp::wake_pair()
    }
}

/// Wait until any of `wakes` is signaled or `timeout` elapses.
/// Returns the index of the first signaled wake, or `None` on timeout.
/// Both ends must be `wait`-capable ends (read ends on Unix).
pub fn wait_any(wakes: &[&Wake], timeout: Duration) -> Option<usize> {
    imp::wait_any(wakes, timeout)
}

/// A complete channel as the app sees it: the ring plus both wake ends the
/// app keeps, and the connection tokens the child consumes.
pub struct Channel {
    pub ring: Ring,
    /// App → host doorbell (RemoteInsert writes).
    pub app_in_w: Wake,
    /// Host → app doorbell (RemoteInsert/InsertHandle read).
    pub app_out_r: Wake,
    /// Tokens passed to the child (`--shm=`, `--in-wake=`, `--out-wake=`).
    pub child: ChildConn,
    /// Parent-side raw endpoints consumed by `insert::spawn` to wire the
    /// child's inherited descriptors (Unix `(shm_fd, in_r, out_w)`; `()` on
    /// Windows where children open objects by name).
    pub(crate) parent_fds: imp::ParentFds,
    /// Keep child wake endpoints owned until spawn has inherited them.
    #[cfg(unix)]
    _child_ends: (Wake, Wake),
}

pub struct ChildConn {
    pub shm: ShmConn,
    pub in_wake: WakeConn,
    pub out_wake: WakeConn,
}

impl Channel {
    pub fn create(max_frames: usize) -> Result<Self> {
        if max_frames == 0 || max_frames > MAX_BLOCK_FRAMES {
            return Err(PluginError::Shm("invalid channel capacity".into()));
        }
        channel(max_frames)
    }

    /// Serialize child tokens for `verdant-plughost host --channel=…`.
    /// Format: platform-tagged strings — `fd:N,N,N` or `name:S,S,S`.
    pub fn child_arg(&self) -> String {
        match (&self.child.shm, &self.child.in_wake, &self.child.out_wake) {
            (ShmConn::Fd(a), WakeConn::Fd(b), WakeConn::Fd(c)) => {
                format!("fd:{a},{b},{c}")
            }
            (ShmConn::Name(a), WakeConn::Name(b), WakeConn::Name(c)) => {
                format!("name:{a},{b},{c}")
            }
            _ => "invalid".into(),
        }
    }

    /// Child side: parse `--channel=` argument.
    pub fn parse_child_arg(arg: &str) -> Result<(ShmConn, WakeConn, WakeConn)> {
        if let Some(rest) = arg.strip_prefix("fd:") {
            let mut it = rest.split(',').map(|s| s.parse::<i32>());
            let a = it.next().and_then(std::result::Result::ok);
            let b = it.next().and_then(std::result::Result::ok);
            let c = it.next().and_then(std::result::Result::ok);
            match (a, b, c) {
                (Some(a), Some(b), Some(c)) if a >= 0 && b >= 0 && c >= 0 && it.next().is_none() => {
                    return Ok((ShmConn::Fd(a), WakeConn::Fd(b), WakeConn::Fd(c)));
                }
                _ => {}
            }
        } else if let Some(rest) = arg.strip_prefix("name:") {
            let mut it = rest.splitn(3, ',');
            if let (Some(a), Some(b), Some(c)) = (it.next(), it.next(), it.next()) {
                return Ok((
                    ShmConn::Name(a.into()),
                    WakeConn::Name(b.into()),
                    WakeConn::Name(c.into()),
                ));
            }
        }
        Err(PluginError::Shm(format!("bad channel arg: {arg}")))
    }

    /// Child side: attach to parent's ring and open wake ends.
    /// `in_wake` = read end, `out_wake` = write end.
    pub fn connect(
        shm: &ShmConn,
        in_wake: &WakeConn,
        out_wake: &WakeConn,
    ) -> Result<(Ring, Wake, Wake)> {
        let ring = Ring::attach(shm)?;
        #[cfg(unix)]
        if let ShmConn::Fd(fd) = shm {
            // attach duplicates; the inherited token is consumed by connect.
            unsafe { libc::close(*fd); }
        }
        let in_r = imp::wake_connect_in(in_wake)?;
        let out_w = imp::wake_connect_out(out_wake)?;
        Ok((ring, in_r, out_w))
    }
}

#[cfg(unix)]
mod imp {
    use super::*;

    pub type RingImp = libc::c_int; // owned fd, closed on drop
    pub type WakeImp = FdWake;
    pub type ParentFds = (libc::c_int, libc::c_int, libc::c_int);

    pub fn unmap(ptr: *mut u8, len: usize, fd: RingImp) {
        unsafe { libc::munmap(ptr.cast(), len); libc::close(fd); }
    }

    pub fn create_ring(max_frames: usize) -> Result<Ring> {
        let (ptr, imp) = map_new(ring_len(max_frames))?;
        Ok(Ring { ptr, len: ring_len(max_frames), imp })
    }

    pub fn map_new(len: usize) -> Result<(*mut u8, RingImp)> {
        let name = format!("/verdant-ph-{}", uuid::Uuid::new_v4().as_simple());
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            let fd = libc::shm_open(
                cname.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC,
                0o600,
            );
            if fd < 0 {
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            // Unlink now: mapping + fd survive; the child inherits the fd so
            // there is no name race and nothing to clean up later.
            libc::shm_unlink(cname.as_ptr());
            if libc::ftruncate(fd, len as libc::off_t) != 0 {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(PluginError::Shm(e.to_string()));
            }
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            if ptr == libc::MAP_FAILED {
                let e = io::Error::last_os_error();
                libc::close(fd);
                return Err(PluginError::Shm(e.to_string()));
            }
            Ok((ptr as *mut u8, fd))
        }
    }

    pub fn map_existing(conn: &ShmConn) -> Result<(*mut u8, usize, RingImp)> {
        let ShmConn::Fd(fd) = conn else {
            return Err(PluginError::Shm("unix ring attach needs an fd".into()));
        };
        let fd = unsafe { libc::fcntl(*fd, libc::F_DUPFD_CLOEXEC, 0) };
        if fd < 0 { return Err(io::Error::last_os_error().into()); }
        unsafe {
            let mut st: libc::stat = std::mem::zeroed();
            if libc::fstat(fd, &mut st) != 0 {
                libc::close(fd);
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            let len = st.st_size as usize;
            if len < HEADER_SIZE || len > ring_len(MAX_BLOCK_FRAMES) {
                libc::close(fd);
                return Err(PluginError::Shm("invalid mapping length".into()));
            }
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            );
            if ptr == libc::MAP_FAILED {
                libc::close(fd);
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            Ok((ptr as *mut u8, len, fd))
        }
    }

    pub struct FdWake {
        fd: libc::c_int,
        owned: bool,
    }

    impl FdWake {
        fn new(fd: libc::c_int, owned: bool) -> Self {
            Self { fd, owned }
        }

        pub fn signal(&self) {
            unsafe {
                let b = 1u8;
                // O_NONBLOCK: EAGAIN (full) is a no-op doorbell; EBADF (peer
                // gone) likewise. Never propagate — seq counters are truth.
                libc::write(self.fd, &b as *const u8 as *const _, 1);
            }
        }

        pub fn wait(&self) -> io::Result<()> {
            self.poll_inner(None).map(|_| ())
        }

        pub fn wait_timeout(&self, d: Duration) -> io::Result<bool> {
            self.poll_inner(Some(d))
        }

        fn poll_inner(&self, d: Option<Duration>) -> io::Result<bool> {
            let mut pfd = libc::pollfd {
                fd: self.fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ms = d.map(|t| t.as_millis().clamp(0, i32::MAX as u128) as i32);
            loop {
                let r = unsafe {
                    libc::poll(&mut pfd, 1, ms.unwrap_or(-1))
                };
                if r < 0 {
                    let e = io::Error::last_os_error();
                    if e.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(e);
                }
                if r == 0 {
                    return Ok(false);
                }
                if pfd.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
                    // Readable or dead peer — either way, re-check seq state.
                    return Ok(true);
                }
            }
        }

        pub fn drain(&self) {
            unsafe {
                let mut buf = [0u8; 64];
                // fd is O_NONBLOCK; read until empty or peer EOF.
                while libc::read(self.fd, buf.as_mut_ptr() as *mut _, buf.len()) > 0 {}
            }
        }
    }

    impl Drop for FdWake {
        fn drop(&mut self) {
            if self.owned {
                unsafe {
                    libc::close(self.fd);
                }
            }
        }
    }

    impl FdWake {
        pub fn dup(&self) -> Result<FdWake> {
            unsafe {
                let t = libc::fcntl(self.fd, libc::F_DUPFD_CLOEXEC, 0);
                if t < 0 {
                    Err(PluginError::Shm(io::Error::last_os_error().to_string()))
                } else {
                    Ok(FdWake { fd: t, owned: true })
                }
            }
        }
    }

    pub fn wake_pair() -> Result<(Wake, Wake)> {
        let (r, w) = pipe_nonblock()?;
        Ok((
            Wake { imp: FdWake::new(r, true) },
            Wake { imp: FdWake::new(w, true) },
        ))
    }

    pub fn wait_any(wakes: &[&Wake], timeout: Duration) -> Option<usize> {
        let mut pfds: Vec<libc::pollfd> = wakes
            .iter()
            .map(|w| libc::pollfd {
                fd: w.imp.fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        let ms = timeout.as_millis().clamp(0, i32::MAX as u128) as i32;
        loop {
            let r = unsafe { libc::poll(pfds.as_mut_ptr(), pfds.len() as libc::nfds_t, ms) };
            if r < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return None;
            }
            if r == 0 {
                return None;
            }
            return pfds
                .iter()
                .position(|p| p.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0);
        }
    }

    fn pipe_nonblock() -> io::Result<(libc::c_int, libc::c_int)> {
        let mut fds = [0; 2];
        unsafe {
            if libc::pipe(fds.as_mut_ptr()) != 0 {
                return Err(io::Error::last_os_error());
            }
            for f in fds {
                let fl = libc::fcntl(f, libc::F_GETFL);
                libc::fcntl(f, libc::F_SETFL, fl | libc::O_NONBLOCK);
                libc::fcntl(f, libc::F_SETFD, libc::FD_CLOEXEC);
            }
        }
        Ok((fds[0], fds[1]))
    }

    pub fn channel(max_frames: usize) -> Result<Channel> {
        let ring = Ring::create(max_frames)?;
        let shm_fd = ring.imp;
        let (in_r, in_w) = wake_pair()?;
        let (out_r, out_w) = wake_pair()?;
        let fds = (shm_fd, in_r.imp.fd, out_w.imp.fd);
        Ok(Channel {
            ring,
            app_in_w: in_w,
            app_out_r: out_r,
            child: ChildConn {
                // Child sees them dup2'd onto fixed descriptors — see insert.rs.
                shm: ShmConn::Fd(9),
                in_wake: WakeConn::Fd(10),
                out_wake: WakeConn::Fd(11),
            },
            parent_fds: fds,
            _child_ends: (in_r, out_w),
        })
    }

    pub fn wake_connect_in(conn: &WakeConn) -> Result<Wake> {
        let WakeConn::Fd(fd) = conn else {
            return Err(PluginError::Shm("unix wake needs fd".into()));
        };
        if unsafe { libc::fcntl(*fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(Wake { imp: FdWake::new(*fd, true) })
    }

    pub fn wake_connect_out(conn: &WakeConn) -> Result<Wake> {
        wake_connect_in(conn)
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, FILE_MAP_ALL_ACCESS, MapViewOfFile, OpenFileMappingW,
        PAGE_READWRITE, UnmapViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_ALL_ACCESS,
    };

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    /// On Windows the mapping handle owns the region; `Ring` closes it.
    pub type RingImp = HANDLE;
    pub type WakeImp = EventWake;
    pub type ParentFds = ();

    pub fn unmap(ptr: *mut u8, _: usize, handle: RingImp) {
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: ptr.cast() });
            CloseHandle(handle);
        }
    }

    pub fn create_ring(max_frames: usize) -> Result<Ring> {
        let (ptr, imp, _name) = map_new(ring_len(max_frames))?;
        Ok(Ring { ptr, len: ring_len(max_frames), imp })
    }

    fn name(tag: &str) -> String {
        format!("Local\\verdant-ph-{tag}-{}", uuid::Uuid::new_v4().as_simple())
    }

    pub fn map_new(len: usize) -> Result<(*mut u8, RingImp, String)> {
        let nm = name("shm");
        let wnm = wide(&nm);
        unsafe {
            let h = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null(),
                PAGE_READWRITE,
                (len >> 32) as u32,
                len as u32,
                wnm.as_ptr(),
            );
            if h.is_null() {
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            let ptr = MapViewOfFile(h, FILE_MAP_ALL_ACCESS, 0, 0, len);
            if ptr.Value.is_null() {
                let e = io::Error::last_os_error();
                CloseHandle(h);
                return Err(PluginError::Shm(e.to_string()));
            }
            Ok((ptr.Value as *mut u8, h, nm))
        }
    }

    pub fn map_existing(conn: &ShmConn) -> Result<(*mut u8, usize, RingImp)> {
        let ShmConn::Name(nm) = conn else {
            return Err(PluginError::Shm("windows ring attach needs a name".into()));
        };
        let wnm = wide(nm);
        unsafe {
            let h = OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, wnm.as_ptr());
            if h.is_null() {
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            // Map header first to learn the real size.
            let hdr = MapViewOfFile(h, FILE_MAP_ALL_ACCESS, 0, 0, HEADER_SIZE);
            if hdr.Value.is_null() {
                CloseHandle(h);
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            let max = (*(hdr.Value as *const RingHeader)).max_frames.load(Ordering::Acquire) as usize;
            UnmapViewOfFile(hdr);
            if max == 0 || max > MAX_BLOCK_FRAMES {
                CloseHandle(h);
                return Err(PluginError::Shm("invalid shared capacity".into()));
            }
            let len = ring_len(max);
            let ptr = MapViewOfFile(h, FILE_MAP_ALL_ACCESS, 0, 0, len);
            if ptr.Value.is_null() {
                CloseHandle(h);
                return Err(PluginError::Shm(io::Error::last_os_error().to_string()));
            }
            Ok((ptr.Value as *mut u8, len, h))
        }
    }

    pub struct EventWake {
        h: HANDLE,
        owned: bool,
    }

    impl EventWake {
        pub fn signal(&self) {
            unsafe {
                SetEvent(self.h);
            }
        }
        fn wait_ms(&self, ms: u32) -> io::Result<bool> {
            unsafe {
                match WaitForSingleObject(self.h, ms) {
                    r if r == windows_sys::Win32::Foundation::WAIT_OBJECT_0 => Ok(true),
                    r if r == windows_sys::Win32::Foundation::WAIT_TIMEOUT => Ok(false),
                    _ => Err(io::Error::last_os_error()),
                }
            }
        }
        pub fn wait(&self) -> io::Result<()> {
            self.wait_ms(windows_sys::Win32::System::Threading::INFINITE).map(|_| ())
        }
        pub fn wait_timeout(&self, d: Duration) -> io::Result<bool> {
            self.wait_ms(d.as_millis().min(u32::MAX as u128 - 1) as u32)
        }
        pub fn drain(&self) {
            // Auto-reset events don't accumulate; nothing to drain.
        }
    }

    impl Drop for EventWake {
        fn drop(&mut self) {
            if self.owned {
                unsafe {
                    CloseHandle(self.h);
                }
            }
        }
    }

    impl EventWake {
        pub fn dup(&self) -> Result<EventWake> {
            use windows_sys::Win32::Foundation::DuplicateHandle;
            use windows_sys::Win32::System::Threading::GetCurrentProcess;
            unsafe {
                let mut out: HANDLE = std::ptr::null_mut();
                if DuplicateHandle(
                    GetCurrentProcess(),
                    self.h,
                    GetCurrentProcess(),
                    &mut out,
                    0,
                    0,
                    windows_sys::Win32::Foundation::DUPLICATE_SAME_ACCESS,
                ) == 0
                {
                    Err(PluginError::Shm(io::Error::last_os_error().to_string()))
                } else {
                    Ok(EventWake { h: out, owned: true })
                }
            }
        }
    }

    pub fn wake_pair() -> Result<(Wake, Wake)> {
        // A Windows event has no distinct read/write ends; duplicate the
        // same handle twice so both halves own a handle (drop-safe).
        let (w, _nm) = new_event("pair")?;
        let r = Wake { imp: EventWake { h: w, owned: true } };
        let w2 = r.dup()?;
        Ok((r, w2))
    }

    pub fn wait_any(wakes: &[&Wake], timeout: Duration) -> Option<usize> {
        use windows_sys::Win32::System::Threading::WaitForMultipleObjects;
        use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
        let handles: Vec<HANDLE> = wakes.iter().map(|w| w.imp.h).collect();
        let ms = timeout.as_millis().min(u32::MAX as u128 - 1) as u32;
        unsafe {
            match WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, ms) {
                r if r >= WAIT_OBJECT_0 && r < WAIT_OBJECT_0 + handles.len() as u32 => {
                    Some((r - WAIT_OBJECT_0) as usize)
                }
                r if r == WAIT_TIMEOUT => None,
                _ => None,
            }
        }
    }

    fn new_event(tag: &str) -> io::Result<(HANDLE, String)> {
        let nm = name(tag);
        let wnm = wide(&nm);
        unsafe {
            let h = CreateEventW(std::ptr::null(), 0, 0, wnm.as_ptr());
            if h.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok((h, nm))
            }
        }
    }

    pub fn channel_named(max_frames: usize) -> Result<Channel> {
        let len = ring_len(max_frames);
        let (ptr, h, shm_name) = map_new(len)?;
        let ring = Ring { ptr, len, imp: h };
        ring.init(max_frames);
        let (in_h, in_nm) = new_event("in")?;
        let (out_h, out_nm) = new_event("out")?;
        Ok(Channel {
            ring,
            app_in_w: Wake { imp: EventWake { h: in_h, owned: true } },
            app_out_r: Wake { imp: EventWake { h: out_h, owned: true } },
            child: ChildConn {
                shm: ShmConn::Name(shm_name),
                in_wake: WakeConn::Name(in_nm),
                out_wake: WakeConn::Name(out_nm),
            },
            parent_fds: (),
        })
    }

    fn open_event(nm: &str) -> Result<Wake> {
        let wnm = wide(nm);
        unsafe {
            let h = OpenEventW(EVENT_ALL_ACCESS, 0, wnm.as_ptr());
            if h.is_null() {
                Err(PluginError::Shm(io::Error::last_os_error().to_string()))
            } else {
                Ok(Wake {
                    imp: EventWake { h, owned: true },
                })
            }
        }
    }

    pub fn wake_connect_in(conn: &WakeConn) -> Result<Wake> {
        match conn {
            WakeConn::Name(nm) => open_event(nm),
            _ => Err(PluginError::Shm("windows wake needs a name".into())),
        }
    }
    pub fn wake_connect_out(conn: &WakeConn) -> Result<Wake> {
        wake_connect_in(conn)
    }
}

#[cfg(unix)]
pub(crate) fn channel(max_frames: usize) -> Result<Channel> {
    imp::channel(max_frames)
}

#[cfg(windows)]
pub(crate) fn channel(max_frames: usize) -> Result<Channel> {
    imp::channel_named(max_frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_layout() {
        assert_eq!(std::mem::size_of::<RingHeader>(), HEADER_SIZE);
        assert_eq!(std::mem::align_of::<RingHeader>(), 64);
        assert_eq!(std::mem::size_of::<Slot>(), SLOT_HEADER_SIZE);
        assert_eq!(std::mem::size_of::<WireParamEvent>(), 16);
        assert_eq!(std::mem::align_of::<WireParamEvent>(), 8);
    }

    #[test]
    fn event_payload_is_atomic_and_never_truncated() {
        let ring = Ring::create(DSP_QUANTUM_FRAMES).unwrap();
        let samples = [0.25; DSP_QUANTUM_FRAMES];
        let events: Vec<_> = (0..MAX_PARAM_EVENTS).map(|id| (id as u32, 255, id as f64)).collect();
        ring.try_write_input_events(0, &samples, &samples, &events).unwrap();
        let mut l = [9.0; DSP_QUANTUM_FRAMES];
        let mut r = l;
        let mut short = [(0, 0, 0.0); 1];
        assert!(matches!(ring.read_input_events(0, &mut l, &mut r, &mut short),
            Err(PluginError::Automation(AutomationError::EventCapacity))));
        assert_eq!(l, [9.0; DSP_QUANTUM_FRAMES]);
        let mut received = vec![(0, 0, 0.0); MAX_PARAM_EVENTS];
        assert_eq!(ring.read_input_events(0, &mut l, &mut r, &mut received).unwrap(),
            (DSP_QUANTUM_FRAMES, MAX_PARAM_EVENTS));
        assert_eq!(received, events);
        assert_eq!(l, samples);
        assert_eq!(r, samples);
        assert_eq!(ring.read_input_events(0, &mut l, &mut r, &mut received).unwrap(), (0, 0));
    }

    #[test]
    fn invalid_event_publication_preserves_sequence_and_audio() {
        let ring = Ring::create(DSP_QUANTUM_FRAMES).unwrap();
        let samples = [1.0; DSP_QUANTUM_FRAMES];
        let oversized = vec![(1, 0, 1.0); MAX_PARAM_EVENTS + 1];
        assert_eq!(ring.try_write_input_events(0, &samples, &samples, &oversized), Err(AutomationError::EventCapacity));
        assert_eq!(ring.try_write_input_events(0, &samples, &samples, &[(1, 256, 1.0)]), Err(AutomationError::InvalidOffset));
        assert_eq!(ring.in_seq(), 0);
        ring.try_write_input_events(0, &samples, &samples, &[(1, 255, 1.0)]).unwrap();
        let slot = unsafe { &*(ring.slot_ptr(HEADER_SIZE, 0) as *const Slot) };
        unsafe { *slot.event_count.get() = (MAX_PARAM_EVENTS + 1) as u32; }
        let mut l = [0.0; DSP_QUANTUM_FRAMES];
        let mut r = l;
        let mut events = vec![(0, 0, 0.0); MAX_PARAM_EVENTS];
        assert!(matches!(ring.read_input_events(0, &mut l, &mut r, &mut events),
            Err(PluginError::Automation(AutomationError::EventCapacity))));
        assert_eq!(l, [0.0; DSP_QUANTUM_FRAMES]);
    }

    #[test]
    fn ring_write_read_roundtrip() {
        let ring = Ring::create(256).unwrap();
        let l: Vec<f32> = (0..256).map(|i| i as f32).collect();
        let r: Vec<f32> = (0..256).map(|i| -(i as f32)).collect();
        ring.write_input(0, &l, &r).unwrap();
        let mut lo = vec![0.0; 256];
        let mut ro = vec![0.0; 256];
        assert_eq!(ring.read_input(0, &mut lo, &mut ro).unwrap(), 256);
        assert_eq!(lo, l);
        assert_eq!(ro, r);
        ring.write_output(0, &l, &r).unwrap();
        let mut ol = vec![0.0; 256];
        let mut orr = vec![0.0; 256];
        assert!(ring.read_output(0, &mut ol, &mut orr).unwrap());
        assert_eq!(ol, l);
        assert_eq!(orr, r);
        // Future block not yet produced.
        assert!(!ring.read_output(1, &mut ol, &mut orr).unwrap());
        assert_eq!(ring.in_seq(), 1);
        assert_eq!(ring.out_seq(), 1);
    }

    #[test]
    fn queued_burst_preserves_every_unread_slot() {
        let ring = Ring::create(64).unwrap();
        for seq in 0..RING_SLOTS as u64 {
            let samples = [seq as f32 + 1.0; 64];
            ring.write_input(seq, &samples, &samples).unwrap();
        }
        assert!(!ring.try_write_input(RING_SLOTS as u64, &[99.0; 64], &[99.0; 64]));
        let resumed = RING_SLOTS as u64 * 4;
        assert!(!ring.try_write_input(resumed, &[99.0; 64], &[99.0; 64]));
        let mut lo = vec![0.0; 64];
        let mut ro = vec![0.0; 64];
        for seq in 0..RING_SLOTS as u64 {
            assert_eq!(ring.read_input(seq, &mut lo, &mut ro).unwrap(), 64);
            assert_eq!(lo, vec![seq as f32 + 1.0; 64]);
            assert_eq!(ro, lo);
            ring.write_output(seq, &lo, &ro).unwrap();
        }
        ring.write_input(resumed, &[99.0; 64], &[99.0; 64]).unwrap();
        assert_eq!(ring.read_input(resumed, &mut lo, &mut ro).unwrap(), 64);
        assert_eq!(lo, vec![99.0; 64]);
        ring.write_output(resumed, &lo, &ro).unwrap();
        ring.write_input(resumed + 1, &[100.0; 64], &[100.0; 64]).unwrap();
        assert_eq!(ring.read_input(resumed + 1, &mut lo, &mut ro).unwrap(), 64);
        assert_eq!(lo, vec![100.0; 64]);
    }

    #[test]
    fn bounds_rejected() {
        let ring = Ring::create(64).unwrap();
        let l = vec![0.0f32; 128];
        assert!(ring.write_input(0, &l, &l).is_err());
        assert!(Ring::create(0).is_err());
        assert!(Ring::create(MAX_BLOCK_FRAMES + 1).is_err());
        // Mismatched L/R lengths rejected.
        let short = vec![0.0f32; 32];
        assert!(ring.write_input(0, &l[..32], &short[..31]).is_err());
    }

    #[test]
    fn attach_validates_magic() {
        let ring = Ring::create(64).unwrap();
        #[cfg(unix)]
        {
            let attached = Ring::attach(&ShmConn::Fd(ring.imp)).unwrap();
            assert_eq!(attached.max_frames(), 64);
            // Corrupt the magic → re-attach must fail.
            ring.header().magic.store(0, Ordering::Relaxed);
            assert!(Ring::attach(&ShmConn::Fd(ring.imp)).is_err());
        }
    }

    #[test]
    fn sidechain_planes_travel_with_their_block_and_unkeyed_blocks_read_silence() {
        let ring = Ring::create(64).unwrap();
        let (l, r) = ([0.25f32; 64], [-0.25f32; 64]);
        let (kl, kr) = ([0.5f32; 64], [0.75f32; 64]);
        assert!(ring.try_write_input_keyed(0, &l, &r, Some((&kl, &kr)), &[], 0));
        assert!(ring.try_write_input_keyed(1, &l, &r, None, &[], 0));
        let (mut il, mut ir, mut ol, mut or) = ([0.0; 64], [0.0; 64], [9.0; 64], [9.0; 64]);
        assert_eq!(ring.read_input_keyed(0, &mut il, &mut ir, Some((&mut ol, &mut or)), &mut []).unwrap(), (64, 0, true));
        assert_eq!((ol, or), (kl, kr));
        assert_eq!(ring.read_input_keyed(1, &mut il, &mut ir, Some((&mut ol, &mut or)), &mut []).unwrap(), (64, 0, false));
        assert_eq!((ol, or), ([0.0; 64], [0.0; 64]));
        // Mismatched key lengths publish nothing.
        assert!(!ring.try_write_input_keyed(2, &l, &r, Some((&kl[..32], &kr)), &[], 0));
    }
}
