//! Transport and real-time mixing (SPEC §10).
//!
//! Signal flow per block:
//!
//! ```text
//! clips ─▶ track bus ─▶ inserts ─▶ fader · balance · mute/solo ─▶ sum ─▶ master inserts ─▶ master fader · balance ─▶ soft clip ─▶ device
//!                                              ▲ track meters                                                        ▲ master meter
//! ```
//!
//! The callback reads only preallocated, immutable PCM and atomics; it never
//! allocates, locks, logs, touches files or waits. Timeline and insert-chain
//! swaps travel as `Box`es through wait-free SPSC rings; replaced objects go
//! back through a second ring and are dropped on a non-real-time thread.
//! Play / pause / seek / stop and every mixer control are atomics, so a fader
//! move is heard on the next block without re-rendering anything.
//! Click tracks are prepared from absolute beat-frame boundaries. Count-in
//! PCM uses a finite transport overlay, so the timeline clock remains at its
//! entry position until the exact final pre-roll frame has been emitted.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};

pub const NO_SEEK: u64 = u64::MAX;
/// Mixer channels available to the timeline (track slot = index here).
pub const MAX_TRACKS: usize = 32;
/// Retained-processor source coordinate for the master insert chain. This is
/// not a track slot: clips and `set_chain` still admit only `0..MAX_TRACKS`.
pub const MASTER_BUS: usize = MAX_TRACKS;
const MIXER_BUSES: usize = MAX_TRACKS + 1;
pub const CROSSFADE_FRAMES: u32 = 256;
const INTENT_MASK: u64 = 3;
const INTENT_PAUSED: u64 = 0;
const INTENT_PLAYING: u64 = 1;
const INTENT_ENDED: u64 = 2;
const INTENT_STOPPED: u64 = 3;

/// Already-resampled mono PCM placed on the timeline (device sample units).
#[derive(Clone, Debug)]
pub struct Clip {
    pub start: u64,
    pub samples: Arc<[f32]>,
    pub gain: f32,
    /// Mixer channel this clip plays through.
    pub track: u16,
}

/// Interleaved stereo source PCM; per-clip pan is a unity-at-centre balance.
#[derive(Clone,Debug)]
pub struct StereoClip{pub start:u64,pub samples:Arc<[f32]>,pub gain:f32,pub pan:f32,pub track:u16}
#[derive(Clone,Copy,Debug)]
pub struct ClickEvent{pub sample:u64,pub accent:bool}
#[derive(Clone,Debug)]
pub struct ClickTrack{pub events:Vec<ClickEvent>,pub strong:Arc<[f32]>,pub weak:Arc<[f32]>,pub length:u64}
impl ClickTrack{
    /// Prepare on the control thread. Every event is an absolute device-frame
    /// boundary; no callback-size or repeatedly rounded beat duration is used.
    pub fn new(sample_rate:u32,events:Vec<ClickEvent>,length:u64)->Result<Self,String>{
        if sample_rate==0{return Err("节拍器采样率须为正数".into());}
        if events.len()>4_000_000||events.windows(2).any(|w|w[0].sample>w[1].sample)||events.iter().any(|e|e.sample>=length){return Err("节拍器事件须按时间排序且位于播放范围内".into());}
        let samples=|accent|{
            let length=(f64::from(sample_rate)*0.025).round()as usize;
            (0..length).map(|i|{let t=i as f64/f64::from(sample_rate);let hz=if accent{2200.}else{1350.};(0.16*(-t*180.).exp()*(t*hz*std::f64::consts::TAU).sin())as f32}).collect::<Vec<_>>().into()
        };
        Ok(Self{events,strong:samples(true),weak:samples(false),length})
    }
    /// A finite pre-roll, deliberately separate from timeline clips. Its last
    /// frame is truncated at the exact count-in boundary, even during a click.
    pub fn count_in_pcm(&self)->Result<Arc<[f32]>,String>{
        let length=usize::try_from(self.length).map_err(|_|"预备拍长度超出平台范围")?;
        let mut pcm=vec![0.;length];
        for event in &self.events{
            let start=usize::try_from(event.sample).map_err(|_|"预备拍事件超出平台范围")?;
            let click=if event.accent{&self.strong}else{&self.weak};
            if start<length{for(output,value)in pcm[start..].iter_mut().zip(click.iter()){*output+=value;}}
        }
        Ok(pcm.into())
    }
    fn mix(&self,pos:u64,left:&mut[f32],right:&mut[f32]){
        let longest=self.strong.len().max(self.weak.len())as u64;let end=pos.saturating_add(left.len()as u64);
        let first=self.events.partition_point(|event|event.sample.saturating_add(longest)<=pos);
        for event in &self.events[first..]{if event.sample>=end{break;}let pcm=if event.accent{&self.strong}else{&self.weak};let from=pos.max(event.sample);let to=end.min(event.sample.saturating_add(pcm.len()as u64));for sample in from..to{let output=(sample-pos)as usize;let value=pcm[(sample-event.sample)as usize];left[output]+=value;right[output]+=value;}}
    }
}

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub sample_rate: u32,
    /// Sorted by `start`.
    pub clips: Vec<Clip>,
    pub stereo_clips: Vec<StereoClip>,
    pub length: u64,
    pub automation: crate::automation_clock::AutomationSchedule,
    /// Mixer channel → processor slot → admitted native parameter curves.
    pub native_automation: Vec<Vec<Option<crate::automation_clock::NativePluginSchedule>>>,
    /// Master processor slot → admitted native parameter curves.
    pub master_native_automation: Vec<Option<crate::automation_clock::NativePluginSchedule>>,
    pub loop_range: Option<(u64,u64)>,
    pub metronome: Option<ClickTrack>,
    /// Sidechain routes: a track's pre-insert, pre-fader signal keys one
    /// processor in another track's chain.
    pub sidechains: Vec<SidechainRoute>,
}

/// One sidechain connection. `delay` aligns the key with the destination
/// processor's input: the summed latency of the processors before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidechainRoute { pub source: u16, pub track: u16, pub processor: u16, pub delay: u32 }

/// Callback-owned delay line and output for one route; allocated on the
/// control side together with its timeline.
struct KeyLine { route: SidechainRoute, line_l: Vec<f32>, line_r: Vec<f32>, head: usize, out_l: Vec<f32>, out_r: Vec<f32> }

impl KeyLine {
    fn new(route: SidechainRoute, block: usize) -> Self {
        let n = route.delay as usize;
        Self { route, line_l: vec![0.0; n], line_r: vec![0.0; n], head: 0, out_l: vec![0.0; block], out_r: vec![0.0; block] }
    }
    /// Delay `frames` of the source signal into `out_*`.
    fn feed(&mut self, left: &[f32], right: &[f32]) {
        let n = left.len();
        if self.line_l.is_empty() {
            self.out_l[..n].copy_from_slice(left);
            self.out_r[..n].copy_from_slice(right);
            return;
        }
        for i in 0..n {
            self.out_l[i] = std::mem::replace(&mut self.line_l[self.head], left[i]);
            self.out_r[i] = std::mem::replace(&mut self.line_r[self.head], right[i]);
            self.head += 1;
            if self.head == self.line_l.len() { self.head = 0; }
        }
    }
}

struct TimelineSwap {
    revision: u64,
    timeline: Timeline,
    keys: Vec<KeyLine>,
}

impl std::ops::Deref for TimelineSwap {
    type Target = Timeline;
    fn deref(&self) -> &Timeline { &self.timeline }
}

impl Timeline {
    pub fn new(sample_rate:u32,clips:Vec<Clip>)->Self{Self::with_stereo(sample_rate,clips,Vec::new())}
    pub fn with_stereo(sample_rate:u32,mut clips:Vec<Clip>,mut stereo_clips:Vec<StereoClip>)->Self{
        clips.retain(|c|(c.track as usize)<MAX_TRACKS);stereo_clips.retain(|c|(c.track as usize)<MAX_TRACKS&&c.samples.len()%2==0);
        clips.sort_by_key(|c|c.start);stereo_clips.sort_by_key(|c|c.start);
        let length=clips.iter().map(|c|c.start.saturating_add(c.samples.len()as u64)).chain(stereo_clips.iter().map(|c|c.start.saturating_add((c.samples.len()/2)as u64))).max().unwrap_or(0);
        Self{sample_rate,clips,stereo_clips,length,automation:Default::default(),native_automation:Vec::new(),master_native_automation:Vec::new(),loop_range:None,metronome:None,sidechains:Vec::new()}
    }
    fn mix_track(&self,track:Option<u16>,pos:u64,out:&mut[f32])->bool{
        let end=pos.saturating_add(out.len()as u64);let mut any=false;
        for c in &self.clips{if c.start>=end{break;}if track.is_some_and(|t|t!=c.track){continue;}let c_end=c.start.saturating_add(c.samples.len()as u64);if c_end<=pos{continue;}let from=pos.max(c.start);let to=end.min(c_end);for(d,s)in out[(from-pos)as usize..(to-pos)as usize].iter_mut().zip(&c.samples[(from-c.start)as usize..(to-c.start)as usize]){*d+=s*c.gain;}any=true;}any
    }
    fn mix_stereo(&self,track:u16,pos:u64,left:&mut[f32],right:&mut[f32])->bool{
        let mut any=self.mix_track(Some(track),pos,left);right.copy_from_slice(left);let end=pos.saturating_add(left.len()as u64);
        for c in &self.stereo_clips{if c.start>=end{break;}if c.track!=track{continue;}let c_end=c.start.saturating_add((c.samples.len()/2)as u64);if c_end<=pos{continue;}let from=pos.max(c.start);let to=end.min(c_end);let(gl,gr)=crate::automation_clock::balance_gains(c.gain,c.pan);for frame in from..to{let source=(frame-c.start)as usize*2;let output=(frame-pos)as usize;left[output]+=c.samples[source]*gl;right[output]+=c.samples[source+1]*gr;}any=true;}any
    }
    fn used_tracks(&self)->usize{self.clips.iter().map(|c|c.track as usize+1).chain(self.stereo_clips.iter().map(|c|c.track as usize+1)).max().unwrap_or(0).max(self.automation.tracks.len())}
}

/// A real-time audio processor in a track or master insert chain. Implementations
/// must not allocate, lock or block. This is the seam where hosted plugins
/// (CLAP / VST3, out of process — see `frontend/docs/PLUGIN-HOST.md`) attach.
pub trait Processor: Send {
    fn name(&self) -> &str;
    fn process(&mut self, left: &mut [f32], right: &mut [f32]);
    /// Native parameter events are precompiled and bounded on the control side.
    /// An unsupported processor rejects the entire nonempty event call.
    fn process_automated(&mut self,left:&mut[f32],right:&mut[f32],events:&[(u32,u32,f64)])->bool{
        if !events.is_empty(){return false;}self.process(left,right);true
    }
    /// Process with a sidechain key signal aligned to `left`/`right`.
    /// Processors without a key input ignore it.
    fn process_keyed(&mut self,left:&mut[f32],right:&mut[f32],_key:(&[f32],&[f32]),events:&[(u32,u32,f64)])->bool{
        self.process_automated(left,right,events)
    }
}

/// Built-in: fixed gain (used by tests; also a minimal reference processor).
pub struct GainProcessor(pub f32);

impl Processor for GainProcessor {
    fn name(&self) -> &str {
        "Gain"
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for s in l.iter_mut().chain(r.iter_mut()) {
            *s *= self.0;
        }
    }
}

pub type Chain = Vec<Box<dyn Processor>>;

fn process_chain<'k>(chain: &mut Chain, schedules: &[Option<crate::automation_clock::NativePluginSchedule>],
    position: u64, left: &mut [f32], right: &mut [f32], events: &mut [(u32, u32, f64)],
    key: &dyn Fn(usize) -> Option<(&'k [f32], &'k [f32])>) -> bool
{
    for (index, processor) in chain.iter_mut().enumerate() {
        let keyed = key(index);
        if let Some(schedule) = schedules.get(index).and_then(Option::as_ref) {
            // Hosted publication uses a fixed 256-frame quantum. Both track
            // and master inserts reuse the engine's preallocated event buffer.
            let mut offset = 0;
            while offset < left.len() {
                let end = (offset + 256).min(left.len());
                let Ok(count) = schedule.write_events(position + offset as u64, end - offset, events) else { return false };
                let ok = match keyed {
                    Some((kl, kr)) => processor.process_keyed(&mut left[offset..end], &mut right[offset..end], (&kl[offset..end], &kr[offset..end]), &events[..count]),
                    None => processor.process_automated(&mut left[offset..end], &mut right[offset..end], &events[..count]),
                };
                if !ok { return false; }
                offset = end;
            }
        } else if let Some(key) = keyed {
            if !processor.process_keyed(left, right, key, &[]) { return false; }
        } else { processor.process(left, right); }
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Transport {
    Stopped = 0,
    Playing = 1,
    Paused = 2,
    DeviceError = 3,
}

impl Transport {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Transport::Playing,
            2 => Transport::Paused,
            3 => Transport::DeviceError,
            _ => Transport::Stopped,
        }
    }
}

fn f32_atomic(v: f32) -> AtomicU32 {
    AtomicU32::new(v.to_bits())
}

/// One mixer channel's controls and meters.
pub struct ChannelCtl {
    pub gain: AtomicU32,
    /// Balance −1 … 1.
    pub pan: AtomicU32,
    pub mute: AtomicBool,
    pub solo: AtomicBool,
    /// Peak |sample| since the UI last read it (f32 bits; non-negative floats
    /// order like their bit patterns, so `fetch_max` is exact).
    pub peak_l: AtomicU32,
    pub peak_r: AtomicU32,
}

impl Default for ChannelCtl {
    fn default() -> Self {
        ChannelCtl {
            gain: f32_atomic(1.0),
            pan: f32_atomic(0.0),
            mute: AtomicBool::new(false),
            solo: AtomicBool::new(false),
            peak_l: AtomicU32::new(0),
            peak_r: AtomicU32::new(0),
        }
    }
}

/// Lock-free state shared between control and callback.
pub struct Shared {
    pub position: AtomicU64,
    // Kind and revision are published together. Explicit pause and automatic
    // EOF differ: the latter must drain already prepared resampler frames.
    transport_intent: AtomicU64,
    pub stop: AtomicBool,
    pub seek: AtomicU64,
    /// Start of the active loop, or project frame zero. Published off the
    /// callback too so a stopped LCD need not wait for another audio block.
    return_position: AtomicU64,
    pub state: AtomicU8,
    pub underruns: AtomicU64,
    pub callbacks: AtomicU64,
    pub max_callback_ns: AtomicU64,
    pub timeline_len: AtomicU64,
    pub sample_rate: AtomicU64,
    pub swaps: AtomicU64,
    pub automation_fault: AtomicBool,
    timeline_revision: AtomicU64,
    pub channels: Vec<ChannelCtl>,
    pub master: ChannelCtl,
}

impl Shared {
    pub fn new(sample_rate: u32) -> Arc<Self> {
        Arc::new(Shared {
            position: AtomicU64::new(0),
            transport_intent: AtomicU64::new(INTENT_STOPPED),
            stop: AtomicBool::new(false),
            seek: AtomicU64::new(NO_SEEK),
            return_position: AtomicU64::new(0),
            state: AtomicU8::new(Transport::Stopped as u8),
            underruns: AtomicU64::new(0),
            callbacks: AtomicU64::new(0),
            max_callback_ns: AtomicU64::new(0),
            timeline_len: AtomicU64::new(0),
            sample_rate: AtomicU64::new(sample_rate as u64),
            swaps: AtomicU64::new(0),
            automation_fault: AtomicBool::new(false),
            timeline_revision: AtomicU64::new(0),
            channels: (0..MAX_TRACKS).map(|_| ChannelCtl::default()).collect(),
            master: ChannelCtl::default(),
        })
    }

    pub fn transport(&self) -> Transport {
        Transport::from_u8(self.state.load(Ordering::Acquire))
    }

    pub fn playback_requested(&self) -> bool {
        self.transport_intent.load(Ordering::Acquire) & INTENT_MASK == INTENT_PLAYING
    }

    fn request_transport(&self, kind: u64) {
        let _ = self.transport_intent.fetch_update(Ordering::AcqRel, Ordering::Acquire, |previous| {
            Some((previous.wrapping_add(4) & !INTENT_MASK) | kind)
        });
    }
}

/// Insert-chain replacement for one track or `MASTER_BUS`.
pub struct ChainSwap {
    pub track: usize,
    pub chain: Chain,
}

/// One processor in a new topology. Retained coordinates refer to the last
/// successfully submitted topology, including any earlier queued command.
/// Each old processor may occur only once in the complete new layout.
/// `Retained.track == MASTER_BUS` addresses a master processor and may only
/// appear in the master layout passed to `remap_chains_with_master`.
pub enum ChainInput {
    Retained { track: usize, processor: usize },
    New(Box<dyn Processor>),
}

struct ChainRemap {
    layout: Vec<Vec<ChainInput>>,
    whole_chains: [Option<usize>; MIXER_BUSES],
    staged: [Vec<Option<Box<dyn Processor>>>; MIXER_BUSES],
    chains: [Option<Box<ChainSwap>>; MIXER_BUSES],
}

enum ChainChange {
    Single(Option<Box<ChainSwap>>),
    Remap(Box<ChainRemap>),
}

struct ChainCommand {
    revision: u64,
    change: ChainChange,
}

impl ChainRemap {
    fn apply(&mut self, current: &mut [Option<Box<ChainSwap>>]) {
        // Whole unchanged chains keep both their allocation and DSP history.
        for (track, source) in self.whole_chains.iter().enumerate() {
            if let Some(source) = source {
                self.chains[track] = current[*source].take();
                if let Some(chain) = &mut self.chains[track] { chain.track = track; }
            }
        }
        // Partial edits move existing boxes through preallocated Option slots.
        // Unused old processors remain here until control-side reclamation.
        for (track, staged) in self.staged.iter_mut().enumerate() {
            if staged.is_empty() { continue; }
            if let Some(chain) = &mut current[track] {
                for (slot, processor) in staged.iter_mut().zip(chain.chain.drain(..)) {
                    *slot = Some(processor);
                }
            }
        }
        for (track, inputs) in self.layout.iter_mut().enumerate() {
            if self.whole_chains[track].is_some() { continue; }
            let Some(chain) = &mut self.chains[track] else { continue };
            for input in inputs {
                let processor = match std::mem::replace(input, ChainInput::Retained { track: MIXER_BUSES, processor: 0 }) {
                    ChainInput::Retained { track, processor } => self.staged[track][processor].take().expect("validated exclusive chain source"),
                    ChainInput::New(processor) => processor,
                };
                // Capacity was reserved for every target on the control side.
                chain.chain.push(processor);
            }
        }
        for (old, new) in current.iter_mut().zip(&mut self.chains) { std::mem::swap(old, new); }
    }
}

/// Everything the callback owns.
pub struct RtState {
    shared: Arc<Shared>,
    incoming: rtrb::Consumer<Box<TimelineSwap>>,
    retire: rtrb::Producer<Box<TimelineSwap>>,
    chain_in: rtrb::Consumer<Box<ChainCommand>>,
    chain_retire: rtrb::Producer<Box<ChainCommand>>,
    current: Option<Box<TimelineSwap>>,
    fading: Option<(Box<TimelineSwap>, u32)>,
    chains: Vec<Option<Box<ChainSwap>>>,
    bus_l: Vec<Vec<f32>>,
    bus_r: Vec<Vec<f32>>,
    mix_l: Vec<f32>,
    mix_r: Vec<f32>,
    fade_tmp: Vec<f32>,
    fade_tmp_r: Vec<f32>,
    parameter_events: Vec<(u32,u32,f64)>,
    overlay_in:rtrb::Consumer<Box<Overlay>>,
    overlay_retire:rtrb::Producer<Box<Overlay>>,
    overlay:Option<Box<Overlay>>,
    overlay_position:usize,
    playing: bool,
    transport_intent: u64,
    pos: u64,
}

pub struct Control {
    /// Convenience open_output/open_silent handles own their device engine.
    /// Declared first so it is joined before control-ring endpoints disappear.
    #[cfg(feature = "audio-device")]
    pub(crate) output_owner: Option<crate::device::OutputEngine>,
    pub shared: Arc<Shared>,
    outgoing: rtrb::Producer<Box<TimelineSwap>>,
    retired: rtrb::Consumer<Box<TimelineSwap>>,
    chain_out: rtrb::Producer<Box<ChainCommand>>,
    chain_retired: rtrb::Consumer<Box<ChainCommand>>,
    chain_lengths: [usize; MIXER_BUSES],
    chain_revision: u64,
    retired_chain_revision: u64,
    timeline_revision: u64,
    pending: Option<Box<TimelineSwap>>,
    overlay_out:rtrb::Producer<Box<Overlay>>,
    overlay_retired:rtrb::Consumer<Box<Overlay>>,
    block: usize,
}

struct Overlay{samples:Arc<[f32]>,block_transport:bool}

pub fn channel(sample_rate: u32, max_block: usize) -> (Control, RtState) {
    let shared = Shared::new(sample_rate);
    let (outgoing, incoming) = rtrb::RingBuffer::new(4);
    let (retire, retired) = rtrb::RingBuffer::new(8);
    let (chain_out, chain_in) = rtrb::RingBuffer::new(MAX_TRACKS);
    let (chain_retire, chain_retired) = rtrb::RingBuffer::new(MAX_TRACKS * 2);
    let (overlay_out,overlay_in)=rtrb::RingBuffer::new(4);
    let (overlay_retire,overlay_retired)=rtrb::RingBuffer::new(8);
    let block = max_block.max(64);
    (
        Control {
            #[cfg(feature = "audio-device")]
            output_owner: None,
            shared: shared.clone(), outgoing, retired, chain_out, chain_retired,
            chain_lengths: [0; MIXER_BUSES], chain_revision: 0, retired_chain_revision: 0, timeline_revision: 0, pending: None, overlay_out, overlay_retired, block },
        RtState {
            shared,
            incoming,
            retire,
            chain_in,
            chain_retire,
            current: None,
            fading: None,
            chains: (0..MIXER_BUSES).map(|_| None).collect(),
            // Preallocated once, sized for the largest block the device may ask.
            bus_l: (0..MAX_TRACKS).map(|_| vec![0.0; block]).collect(),
            bus_r: (0..MAX_TRACKS).map(|_| vec![0.0; block]).collect(),
            mix_l: vec![0.0; block],
            mix_r: vec![0.0; block],
            fade_tmp: vec![0.0; CROSSFADE_FRAMES as usize],
            fade_tmp_r: vec![0.0; CROSSFADE_FRAMES as usize],
            parameter_events: vec![(0,0,0.);4096],
            overlay_in,overlay_retire,overlay:None,overlay_position:0,
            playing: false,
            transport_intent: INTENT_STOPPED,
            pos: 0,
        },
    )
}

/// Mixer settings for one channel, as the UI holds them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelSettings {
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
}

impl Control {
    pub fn play(&self) {
        // Do not cancel an unconsumed Stop: stop followed immediately by play
        // still restarts at the loop/project start on the next callback.
        self.shared.request_transport(INTENT_PLAYING);
    }
    pub fn audition(&mut self,samples:Arc<[f32]>)->bool{self.queue_overlay(samples,false)}
    pub fn play_with_count_in(&mut self,samples:Arc<[f32]>)->bool{if !self.queue_overlay(samples,true){return false;}self.play();true}
    fn queue_overlay(&mut self,samples:Arc<[f32]>,block_transport:bool)->bool{
        self.maintain();if samples.is_empty()||samples.iter().any(|s|!s.is_finite())||self.overlay_out.is_full(){return false;}
        self.overlay_out.push(Box::new(Overlay{samples,block_transport})).is_ok()
    }
    pub fn pause(&self) {
        self.shared.request_transport(INTENT_PAUSED);
    }
    /// Stop returns to the active loop start, otherwise project frame zero.
    /// A later seek may choose a different stopped position; an earlier,
    /// unconsumed seek must not undo this return.
    pub fn stop(&self) {
        self.shared.seek.store(NO_SEEK, Ordering::Release);
        self.shared.request_transport(INTENT_STOPPED);
        self.shared.stop.store(true, Ordering::Release);
    }
    pub fn seek(&self, frame: u64) {
        let was_playing = self.shared.transport_intent.load(Ordering::Acquire) & INTENT_MASK == INTENT_PLAYING;
        self.shared.seek.store(frame, Ordering::Release);
        let _ = self.shared.transport_intent.fetch_update(Ordering::AcqRel, Ordering::Acquire, |previous| {
            let kind = previous & INTENT_MASK;
            // EOF racing this seek must not cancel a playing seek, while an
            // explicit concurrent Pause remains a Pause.
            let kind = if was_playing && kind == INTENT_ENDED { INTENT_PLAYING }
                else if kind == INTENT_STOPPED { INTENT_PAUSED } else { kind };
            Some((previous.wrapping_add(4) & !INTENT_MASK) | kind)
        });
    }
    /// Logical position includes a pending seek/stop, so UI commands update
    /// the playhead immediately even when playback is paused. The callback
    /// consumes the same absolute source frame at its next boundary.
    pub fn position(&self) -> u64 {
        let seek = self.shared.seek.load(Ordering::Acquire);
        if seek != NO_SEEK { return seek; }
        if self.shared.stop.load(Ordering::Acquire) {
            return self.shared.return_position.load(Ordering::Acquire);
        }
        self.shared.position.load(Ordering::Acquire)
    }
    pub fn transport(&self) -> Transport {
        self.shared.transport()
    }

    pub fn set_channel(&self, track: usize, s: ChannelSettings) {
        let Some(c) = self.shared.channels.get(track) else { return };
        c.gain.store(s.gain.max(0.0).to_bits(), Ordering::Relaxed);
        c.pan.store(s.pan.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
        c.mute.store(s.mute, Ordering::Relaxed);
        c.solo.store(s.solo, Ordering::Relaxed);
    }

    pub fn set_master(&self, s: ChannelSettings) {
        let c = &self.shared.master;
        c.gain.store(s.gain.max(0.0).to_bits(), Ordering::Relaxed);
        c.pan.store(s.pan.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
        c.mute.store(s.mute, Ordering::Relaxed);
    }

    /// Peak (L, R) since the previous call, per channel, then the master.
    pub fn take_meters(&self, tracks: usize) -> (Vec<(f32, f32)>, (f32, f32)) {
        let take =
            |c: &ChannelCtl| (f32::from_bits(c.peak_l.swap(0, Ordering::Relaxed)), f32::from_bits(c.peak_r.swap(0, Ordering::Relaxed)));
        (self.shared.channels.iter().take(tracks).map(take).collect(), take(&self.shared.master))
    }

    /// Hand a new timeline to the callback. If the ring is momentarily full
    /// the newest timeline is kept and retried on the next `maintain()`.
    pub fn set_timeline(&mut self, t: Timeline) -> u64 {
        self.timeline_revision += 1;
        let start = t.loop_range.filter(|(start, end)| start < end).map_or(0, |(start, _)| start);
        self.shared.return_position.store(start, Ordering::Release);
        let keys = t.sidechains.iter().filter(|r| (r.source as usize) < MAX_TRACKS && (r.track as usize) < MAX_TRACKS && r.source != r.track)
            .map(|&r| KeyLine::new(r, self.block)).collect();
        self.pending = Some(Box::new(TimelineSwap { revision: self.timeline_revision, timeline: t, keys }));
        self.maintain();
        self.timeline_revision
    }

    /// Last timeline actually installed by the callback, rather than merely
    /// queued. Used with chain reclamation receipts before resuming a remap.
    pub fn applied_timeline_revision(&self) -> u64 {
        self.shared.timeline_revision.load(Ordering::Acquire)
    }

    /// Replace a track's insert chain (takes effect at the next block).
    pub fn set_chain(&mut self, track: usize, chain: Chain) -> bool {
        if track >= MAX_TRACKS { return false; }
        self.replace_chain(track, chain)
    }

    /// Replace the insert chain applied once to the summed stereo mix, before
    /// master gain/pan and soft clipping. Retired processors are reclaimed by
    /// `maintain`, exactly as for track chains.
    pub fn set_master_chain(&mut self, chain: Chain) -> bool {
        self.replace_chain(MASTER_BUS, chain)
    }

    fn replace_chain(&mut self, track: usize, chain: Chain) -> bool {
        self.maintain();
        if self.chain_out.is_full() { return false; }
        let length = chain.len();
        let revision = self.chain_revision + 1;
        let command = Box::new(ChainCommand { revision,
            change: ChainChange::Single(Some(Box::new(ChainSwap { track, chain }))) });
        if self.chain_out.push(command).is_err() { return false; }
        self.chain_lengths[track] = length;
        self.chain_revision = revision;
        true
    }

    /// Atomically replace the track/processor order at a callback boundary,
    /// preserving the master chain. Empty/omitted tracks clear their previous chain. Latency pads
    /// are ordinary final processors, so they move or change in the same swap.
    /// Invalid or full-queue submissions return their still-owned layout.
    /// Keep removed plugin owners alive until `retired_chain_revision()` has
    /// reached the returned revision; enqueueing alone is not reclamation.
    pub fn remap_chains(&mut self, layout: Vec<Vec<ChainInput>>) -> Result<u64, Vec<Vec<ChainInput>>> {
        let master = (0..self.chain_lengths[MASTER_BUS]).map(|processor|
            ChainInput::Retained { track: MASTER_BUS, processor }).collect();
        self.remap_chains_with_master(layout, master).map_err(|(layout, _)| layout)
    }

    /// Edit master inserts while retaining every track processor and its DSP
    /// history. The returned revision covers control-side reclamation of the
    /// old master topology, even when track edits are already queued.
    pub fn remap_master_chain(&mut self, master: Vec<ChainInput>) -> Result<u64, Vec<ChainInput>> {
        let tracks = (0..MAX_TRACKS).map(|track| (0..self.chain_lengths[track]).map(|processor|
            ChainInput::Retained { track, processor }).collect()).collect();
        self.remap_chains_with_master(tracks, master).map_err(|(_, master)| master)
    }

    /// Replace track and master insert topology in one callback transaction.
    /// The master does not consume one of the 32 track slots. Retained master
    /// coordinates use `MASTER_BUS`; processors cannot move between a track
    /// and the master. Errors return both still-owned input layouts.
    pub fn remap_chains_with_master(&mut self, mut layout: Vec<Vec<ChainInput>>, master: Vec<ChainInput>)
        -> Result<u64, (Vec<Vec<ChainInput>>, Vec<ChainInput>)>
    {
        if layout.len() > MAX_TRACKS { return Err((layout, master)); }
        let tracks = layout.len();
        layout.resize_with(MAX_TRACKS, Vec::new);
        layout.push(master);
        self.submit_chains(layout).map_err(|mut layout| {
            let master = layout.pop().expect("master topology slot");
            layout.truncate(tracks);
            (layout, master)
        })
    }

    fn submit_chains(&mut self, layout: Vec<Vec<ChainInput>>) -> Result<u64, Vec<Vec<ChainInput>>> {
        self.maintain();
        if self.chain_out.is_full() { return Err(layout); }
        let mut seen: [Vec<bool>; MIXER_BUSES] = std::array::from_fn(|_| Vec::new());
        for (target, inputs) in layout.iter().enumerate() {
            for input in inputs {
                if let ChainInput::Retained { track, processor } = input {
                    if *track >= MIXER_BUSES || (*track == MASTER_BUS) != (target == MASTER_BUS)
                        || *processor >= self.chain_lengths[*track] { return Err(layout); }
                    if seen[*track].is_empty() { seen[*track].resize(self.chain_lengths[*track], false); }
                    if std::mem::replace(&mut seen[*track][*processor], true) { return Err(layout); }
                }
            }
        }
        let whole_chains = std::array::from_fn(|track| {
            let inputs = layout.get(track)?;
            let ChainInput::Retained { track: source, .. } = inputs.first()? else { return None };
            (inputs.len() == self.chain_lengths[*source] && inputs.iter().enumerate().all(|(index, input)|
                matches!(input, ChainInput::Retained { track, processor } if track == source && *processor == index)))
                .then_some(*source)
        });
        let staged = std::array::from_fn(|track| {
            if seen[track].is_empty() || whole_chains.contains(&Some(track)) { Vec::new() }
            else { (0..self.chain_lengths[track]).map(|_| None).collect() }
        });
        let chains = std::array::from_fn(|track| {
            let length = layout.get(track).map_or(0, Vec::len);
            (length > 0 && whole_chains[track].is_none()).then(|| Box::new(ChainSwap { track, chain: Vec::with_capacity(length) }))
        });
        let lengths = std::array::from_fn(|track| layout.get(track).map_or(0, Vec::len));
        let revision = self.chain_revision + 1;
        let command = Box::new(ChainCommand { revision,
            change: ChainChange::Remap(Box::new(ChainRemap { layout, whole_chains, staged, chains })) });
        if let Err(rtrb::PushError::Full(command)) = self.chain_out.push(command) {
            let ChainChange::Remap(remap) = command.change else { unreachable!() };
            return Err(remap.layout);
        }
        self.chain_lengths = lengths;
        self.chain_revision = revision;
        Ok(revision)
    }

    /// Reclaim old processors before acknowledging that their host-process
    /// owners may be released. This method is for the control thread only.
    pub fn retired_chain_revision(&mut self) -> u64 {
        self.maintain();
        self.retired_chain_revision
    }

    /// Non-real-time housekeeping: push pending swaps, drop retired objects.
    pub fn maintain(&mut self) {
        while let Ok(old)=self.overlay_retired.pop(){drop(old);}
        while let Ok(old) = self.retired.pop() {
            drop(old);
        }
        while let Ok(old) = self.chain_retired.pop() {
            let revision = old.revision;
            drop(old);
            self.retired_chain_revision = revision;
        }
        if let Some(t) = self.pending.take() {
            if let Err(rtrb::PushError::Full(t)) = self.outgoing.push(t) {
                self.pending = Some(t);
            }
        }
    }
}

#[inline]
fn soft_clip(x: f32) -> f32 {
    if x.is_nan() {
        return 0.0;
    }
    let a = x.abs();
    if a <= 0.9 { x } else { x.signum() * (0.9 + 0.1 * ((a - 0.9) / 0.1).tanh()) }
}

#[inline]
fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

impl RtState {
    /// Apply controls once at the physical callback boundary. An output
    /// resampler may then request several source blocks without consuming a
    /// seek halfway through its existing filter history.
    pub(crate) fn prepare_callback(&mut self) -> bool {
        let shared = &*self.shared;
        // 1. Swaps: newest timeline wins; the previous one fades out.
        // A full retirement ring applies backpressure rather than dropping a
        // timeline (and its potentially large PCM) on this callback thread.
        while !self.retire.is_full() {
            let Ok(next) = self.incoming.pop() else { break };
            let revision = next.revision;
            if let Some(prev) = self.current.replace(next) {
                if let Some((older, _)) = self.fading.replace((prev, CROSSFADE_FRAMES)) {
                    let _ = self.retire.push(older);
                }
            }
            shared.swaps.fetch_add(1, Ordering::Relaxed);
            shared.timeline_revision.store(revision, Ordering::Release);
        }
        while !self.chain_retire.is_full() {
            let Ok(mut command) = self.chain_in.pop() else { break };
            match &mut command.change {
                ChainChange::Single(swap) => {
                    if let Some(next) = swap.take() {
                        *swap = self.chains[next.track].replace(next);
                    }
                }
                ChainChange::Remap(remap) => remap.apply(&mut self.chains),
            }
            // Every command carries its retired allocations back, including
            // an empty first installation, and provides a reclamation receipt.
            let _ = self.chain_retire.push(command);
        }
        let len = self.current.as_ref().map(|t| t.length).unwrap_or(0);
        shared.timeline_len.store(len, Ordering::Relaxed);
        // 2. Transport control.
        let start = self.current.as_ref().and_then(|t| t.loop_range)
            .filter(|(start, end)| start < end).map_or(0, |(start, _)| start);
        shared.return_position.store(start, Ordering::Release);
        let stopped = shared.stop.swap(false, Ordering::AcqRel);
        if stopped {
            self.playing = false;
            self.pos = start;
        }
        let seek = shared.seek.swap(NO_SEEK, Ordering::AcqRel);
        if seek != NO_SEEK {
            self.pos = seek;
        }
        self.transport_intent = shared.transport_intent.load(Ordering::Acquire);
        self.playing = self.transport_intent & INTENT_MASK == INTENT_PLAYING;
        if (stopped||seek!=NO_SEEK)&&!self.overlay_retire.is_full(){if let Some(old)=self.overlay.take(){let _=self.overlay_retire.push(old);}}
        while !self.overlay_retire.is_full(){let Ok(next)=self.overlay_in.pop()else{break;};if let Some(old)=self.overlay.replace(next){let _=self.overlay_retire.push(old);}self.overlay_position=0;}
        stopped || seek != NO_SEEK
    }

    pub(crate) fn explicitly_paused(&self) -> bool {
        matches!(self.transport_intent & INTENT_MASK, INTENT_PAUSED | INTENT_STOPPED)
    }

    /// The audio callback body. `out` is interleaved with `channels` channels.
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        self.prepare_callback();
        self.process_prepared(out, channels);
    }

    /// Render source PCM after controls have been applied for this device
    /// callback. This does not consume another asynchronous transport command.
    pub(crate) fn process_prepared(&mut self,out:&mut[f32],channels:usize){
        let started=std::time::Instant::now();let channels=channels.max(1);let frames=out.len()/channels;out.fill(0.);
        if frames>self.mix_l.len(){self.shared.underruns.fetch_add(1,Ordering::Relaxed);}
        else if self.playing{
            let mut written=0;
            while written<frames&&self.playing{
                if let Some(overlay)=self.overlay.as_ref().filter(|o|o.block_transport&&self.overlay_position<o.samples.len()){
                    let count=(frames-written).min(overlay.samples.len()-self.overlay_position);
                    for(i,frame)in out[written*channels..(written+count)*channels].chunks_exact_mut(channels).enumerate(){let value=overlay.samples[self.overlay_position+i];frame[0]=value;if channels>1{frame[1]=value;}}
                    self.overlay_position+=count;written+=count;continue;
                }
                let length=self.current.as_ref().map_or(0,|t|t.length);
                let loop_range=self.current.as_ref().and_then(|t|t.loop_range).filter(|(a,z)|a<z);
                if let Some((start,end))=loop_range{if self.pos>=end{self.pos=start;}}
                let boundary=loop_range.map_or(length,|(_,end)|end);
                if self.pos>=boundary{self.finish_material();break;}
                let count=(frames-written).min((boundary-self.pos)as usize);
                if !self.render_span(&mut out[written*channels..(written+count)*channels],channels){self.playing=false;break;}
                self.pos+=count as u64;written+=count;
                if let Some((start,end))=loop_range{if self.pos==end{self.pos=start;}}
                else if self.pos>=length{self.pos=length;self.finish_material();}
            }
        }
        if let Some(overlay)=self.overlay.as_ref().filter(|o|!o.block_transport){
            let count=frames.min(overlay.samples.len().saturating_sub(self.overlay_position));for(i,frame)in out.chunks_exact_mut(channels).take(count).enumerate(){let value=overlay.samples[self.overlay_position+i];frame[0]=soft_clip(frame[0]+value);if channels>1{frame[1]=soft_clip(frame[1]+value);}}self.overlay_position+=count;
        }
        if self.overlay.as_ref().is_some_and(|o|self.overlay_position>=o.samples.len())&&!self.overlay_retire.is_full(){if let Some(old)=self.overlay.take(){let _=self.overlay_retire.push(old);}}
        if !self.retire.is_full()&&(matches!(&self.fading,Some((_,0)))||(!self.playing&&self.fading.is_some())){if let Some((old,_))=self.fading.take(){let _=self.retire.push(old);}}
        self.shared.position.store(self.pos,Ordering::Release);
        let state=if self.playing{Transport::Playing}else if self.transport_intent&INTENT_MASK==INTENT_STOPPED{Transport::Stopped}else{Transport::Paused};
        if self.shared.transport()!=Transport::DeviceError{self.shared.state.store(state as u8,Ordering::Release);}
        self.shared.callbacks.fetch_add(1,Ordering::Relaxed);self.shared.max_callback_ns.fetch_max(started.elapsed().as_nanos()as u64,Ordering::Relaxed);
    }
    fn finish_material(&mut self){
        let ended=(self.transport_intent&!INTENT_MASK)|INTENT_ENDED;
        if self.shared.seek.load(Ordering::Acquire)==NO_SEEK&&!self.shared.stop.load(Ordering::Acquire){let _=self.shared.transport_intent.compare_exchange(self.transport_intent,ended,Ordering::AcqRel,Ordering::Acquire);}
        self.playing=false;
    }
    fn render_span(&mut self,out:&mut[f32],channels:usize)->bool{
        let frames=out.len()/channels;let pos=self.pos;
        let(ml,mr)=(&mut self.mix_l[..frames],&mut self.mix_r[..frames]);ml.fill(0.);mr.fill(0.);
        let fading=self.fading.as_ref().filter(|(_,remaining)|*remaining>0);
        let used=self.current.as_ref().map_or(0,|t|t.used_tracks()).max(fading.map_or(0,|(old,_)|old.used_tracks())).min(MAX_TRACKS);
        let any_solo=self.shared.channels.iter().any(|c|c.solo.load(Ordering::Relaxed));
        // Pass 1: every track's source material, so sidechain keys can read
        // any track's pre-insert signal regardless of track order.
        let mut material=[false;MAX_TRACKS];
        for ti in 0..used{
            let(bl,br)=(&mut self.bus_l[ti][..frames],&mut self.bus_r[ti][..frames]);bl.fill(0.);br.fill(0.);
            material[ti]=self.current.as_ref().is_some_and(|t|t.mix_stereo(ti as u16,pos,bl,br));
            if let Some((old,remaining))=fading{let n=(*remaining as usize).min(frames);let(pl,pr)=(&mut self.fade_tmp[..n],&mut self.fade_tmp_r[..n]);pl.fill(0.);pr.fill(0.);material[ti]|=old.mix_stereo(ti as u16,pos,pl,pr);for i in 0..n{let old_gain=(*remaining as usize-i)as f32/CROSSFADE_FRAMES as f32;bl[i]=bl[i]*(1.-old_gain)+pl[i]*old_gain;br[i]=br[i]*(1.-old_gain)+pr[i]*old_gain;}}
        }
        if let Some(current)=self.current.as_mut(){
            for key in &mut current.keys{
                let source=key.route.source as usize;
                if source<used{key.feed(&self.bus_l[source][..frames],&self.bus_r[source][..frames]);}
                else{key.out_l[..frames].fill(0.);key.out_r[..frames].fill(0.);}
            }
        }
        for ti in 0..used{
            let ctl=&self.shared.channels[ti];let(bl,br)=(&mut self.bus_l[ti][..frames],&mut self.bus_r[ti][..frames]);
            if !material[ti]&&self.chains[ti].is_none(){continue;}
            if let Some(chain)=&mut self.chains[ti]{
                let schedules=self.current.as_ref().and_then(|t|t.native_automation.get(ti)).map(Vec::as_slice).unwrap_or(&[]);
                let keys=self.current.as_ref().map(|t|t.keys.as_slice()).unwrap_or(&[]);
                let key=|processor:usize|keys.iter().find(|k|k.route.track as usize==ti&&k.route.processor as usize==processor).map(|k|(&k.out_l[..frames],&k.out_r[..frames]));
                if !process_chain(&mut chain.chain,schedules,pos,bl,br,&mut self.parameter_events,&key){self.shared.automation_fault.store(true,Ordering::Release);return false;}
            }
            let audible=!ctl.mute.load(Ordering::Relaxed)&&(!any_solo||ctl.solo.load(Ordering::Relaxed));
            let gain=f32::from_bits(ctl.gain.load(Ordering::Relaxed));let pan=f32::from_bits(ctl.pan.load(Ordering::Relaxed));
            if let Some(automation)=self.current.as_ref().and_then(|t|t.automation.tracks.get(ti)){
                for(i,(gl,gr))in automation.gains(pos,frames,gain,pan).enumerate(){let(gl,gr)=if audible{(gl,gr)}else{(0.,0.)};bl[i]*=gl;br[i]*=gr;ml[i]+=bl[i];mr[i]+=br[i];}
            }else{let(gl,gr)=if audible{crate::automation_clock::balance_gains(gain,pan)}else{(0.,0.)};for i in 0..frames{bl[i]*=gl;br[i]*=gr;ml[i]+=bl[i];mr[i]+=br[i];}}
            ctl.peak_l.fetch_max(peak(bl).to_bits(),Ordering::Relaxed);ctl.peak_r.fetch_max(peak(br).to_bits(),Ordering::Relaxed);
        }
        if let Some(clicks)=self.current.as_ref().and_then(|t|t.metronome.as_ref()){clicks.mix(pos,ml,mr);}
        if let Some(chain)=&mut self.chains[MASTER_BUS]{
            let schedules=self.current.as_ref().map(|t|t.master_native_automation.as_slice()).unwrap_or(&[]);
            if !process_chain(&mut chain.chain,schedules,pos,ml,mr,&mut self.parameter_events,&|_|None){self.shared.automation_fault.store(true,Ordering::Release);return false;}
        }
        let master_muted=self.shared.master.mute.load(Ordering::Relaxed);
        let gain=f32::from_bits(self.shared.master.gain.load(Ordering::Relaxed));let pan=f32::from_bits(self.shared.master.pan.load(Ordering::Relaxed));
        if master_muted{ml.fill(0.);mr.fill(0.);}
        else if let Some(timeline)=&self.current{for(i,(gl,gr))in timeline.automation.master.gains(pos,frames,gain,pan).enumerate(){ml[i]=soft_clip(ml[i]*gl);mr[i]=soft_clip(mr[i]*gr);}}
        else{let(gl,gr)=crate::automation_clock::balance_gains(gain,pan);for i in 0..frames{ml[i]=soft_clip(ml[i]*gl);mr[i]=soft_clip(mr[i]*gr);}}
        self.shared.master.peak_l.fetch_max(peak(ml).to_bits(),Ordering::Relaxed);self.shared.master.peak_r.fetch_max(peak(mr).to_bits(),Ordering::Relaxed);
        for(i,frame)in out.chunks_exact_mut(channels).enumerate(){if channels==1{frame[0]=0.5*(ml[i]+mr[i]);}else{frame[0]=ml[i];frame[1]=mr[i];}}
        if let Some((_,remaining))=&mut self.fading{*remaining-=(*remaining as usize).min(frames)as u32;}true
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_physical_callback_keeps_one_transport_intent_snapshot() {
        let (mut control, mut rt) = channel(48_000, 64);
        control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; 128].into(), gain: 1.0, track: 0 }]));
        // Reproduce the interleaving that two separate pause/play atomics
        // cannot represent safely: Play arrives after the callback snapshot.
        rt.prepare_callback();
        assert!(rt.explicitly_paused());
        control.play();
        assert!(rt.explicitly_paused());
        let mut out = [1.0; 8];
        rt.process_prepared(&mut out, 1);
        assert_eq!(out, [0.0; 8]);
        rt.prepare_callback();
        assert!(!rt.explicitly_paused());
        control.pause();
        assert!(!rt.explicitly_paused());
        rt.process_prepared(&mut out, 1);
        assert_eq!(out, [0.25; 8]);
        rt.prepare_callback();
        assert!(rt.explicitly_paused());
    }

    #[test]
    fn old_callback_eof_cannot_overwrite_a_new_seek_and_play_intent() {
        let (mut control, mut rt) = channel(48_000, 64);
        control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; 4].into(), gain: 1.0, track: 0 }]));
        control.play();
        rt.prepare_callback();
        control.seek(0);
        control.play();
        rt.process_prepared(&mut [0.0; 4], 1); // EOF belongs to the old intent.
        assert!(control.shared.playback_requested());
        assert!(rt.prepare_callback());
        let mut resumed = [0.0; 4];
        rt.process_prepared(&mut resumed, 1);
        assert_eq!(resumed, [0.25; 4]);
    }

    #[test]
    fn eof_preserves_a_seek_published_between_command_poll_and_intent_snapshot() {
        let (mut control, mut rt) = channel(48_000, 64);
        control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; 4].into(), gain: 1.0, track: 0 }]));
        control.play();
        rt.prepare_callback();
        control.seek(0);
        // Deterministically stage the exact concurrent window: the callback
        // has already polled the seek slot but now sees its newer revision.
        rt.transport_intent = control.shared.transport_intent.load(Ordering::Acquire);
        rt.process_prepared(&mut [0.0; 4], 1);
        assert!(control.shared.playback_requested());
        assert!(rt.prepare_callback());
        let mut resumed = [0.0; 4];
        rt.process_prepared(&mut resumed, 1);
        assert_eq!(resumed, [0.25; 4]);
    }

    #[test]
    fn eof_preserves_stop_then_play_published_after_command_poll() {
        let (mut control, mut rt) = channel(48_000, 64);
        control.set_timeline(Timeline::new(48_000, vec![Clip { start: 0, samples: vec![0.25; 4].into(), gain: 1.0, track: 0 }]));
        control.play();
        rt.prepare_callback();
        control.stop();
        control.play();
        rt.transport_intent = control.shared.transport_intent.load(Ordering::Acquire);
        rt.process_prepared(&mut [0.0; 4], 1);
        assert!(control.shared.playback_requested());
        assert!(rt.prepare_callback());
        let mut restarted = [0.0; 4];
        rt.process_prepared(&mut restarted, 1);
        assert_eq!(restarted, [0.25; 4]);
    }
    use std::sync::atomic::AtomicUsize;

    struct DropProbe {
        in_callback: Arc<AtomicBool>,
        callback_drops: Arc<AtomicUsize>,
        drops: Arc<AtomicUsize>,
    }

    impl Processor for DropProbe {
        fn name(&self) -> &str { "retirement probe" }
        fn process(&mut self, _left: &mut [f32], _right: &mut [f32]) {}
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
            if self.in_callback.load(Ordering::Acquire) {
                self.callback_drops.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    #[test]
    fn a_full_retirement_ring_defers_insert_swaps_instead_of_dropping_on_the_callback() {
        let (mut control, mut rt) = channel(48_000, 64);
        let in_callback = Arc::new(AtomicBool::new(false));
        let callback_drops = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        for _ in 0..MAX_TRACKS * 2 + 3 {
            let chain = vec![Box::new(DropProbe {
                in_callback: in_callback.clone(), callback_drops: callback_drops.clone(), drops: drops.clone(),
            }) as Box<dyn Processor>];
            // Queue without maintain() to exercise the bounded full-ring
            // boundary; ordinary UI calls drain retired objects first.
            control.chain_revision += 1;
            assert!(control.chain_out.push(Box::new(ChainCommand { revision: control.chain_revision,
                change: ChainChange::Single(Some(Box::new(ChainSwap { track: 0, chain }))) })).is_ok());
            in_callback.store(true, Ordering::Release);
            rt.process(&mut [0.0; 2], 2);
            in_callback.store(false, Ordering::Release);
        }
        assert_eq!(callback_drops.load(Ordering::Acquire), 0);
        assert_eq!(drops.load(Ordering::Acquire), 0);
        assert_eq!(rt.chain_in.slots(), 3, "new swaps wait for control-side retirement capacity");
        control.maintain();
        assert_eq!(drops.load(Ordering::Acquire), MAX_TRACKS * 2 - 1);
        in_callback.store(true, Ordering::Release);
        rt.process(&mut [0.0; 2], 2);
        in_callback.store(false, Ordering::Release);
        control.maintain();
        assert_eq!(rt.chain_in.slots(), 0);
        assert_eq!(callback_drops.load(Ordering::Acquire), 0);
        drop(rt);
        assert_eq!(drops.load(Ordering::Acquire), MAX_TRACKS * 2 + 3);
    }

    #[test]
    fn remap_moves_exact_processor_boxes_and_reclaims_removed_ones_before_acknowledging() {
        let (mut control, mut rt) = channel(48_000, 64);
        let in_callback = Arc::new(AtomicBool::new(false));
        let callback_drops = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let probe = || Box::new(DropProbe { in_callback: in_callback.clone(),
            callback_drops: callback_drops.clone(), drops: drops.clone() }) as Box<dyn Processor>;
        assert!(control.set_chain(0, vec![probe(), probe()]));
        assert!(control.set_chain(1, vec![probe()]));
        rt.process(&mut [0.0; 2], 2);
        control.maintain();
        let first = &*rt.chains[0].as_ref().unwrap().chain[0] as *const dyn Processor as *const ();
        let second = &*rt.chains[0].as_ref().unwrap().chain[1] as *const dyn Processor as *const ();
        let receipt = control.remap_chains(vec![vec![], vec![
            ChainInput::Retained { track: 0, processor: 1 },
            ChainInput::New(Box::new(GainProcessor(0.5))),
            ChainInput::Retained { track: 0, processor: 0 },
        ]]).ok().unwrap();
        assert!(control.retired_chain_revision() < receipt);
        in_callback.store(true, Ordering::Release);
        rt.process(&mut [0.0; 2], 2);
        in_callback.store(false, Ordering::Release);
        assert!(rt.chains[0].is_none());
        let chain = &rt.chains[1].as_ref().unwrap().chain;
        assert_eq!(&*chain[0] as *const dyn Processor as *const (), second);
        assert_eq!(&*chain[2] as *const dyn Processor as *const (), first);
        assert_eq!(drops.load(Ordering::Acquire), 0, "host owners still need the control-side receipt");
        assert_eq!(callback_drops.load(Ordering::Acquire), 0);
        assert_eq!(control.retired_chain_revision(), receipt);
        assert_eq!(drops.load(Ordering::Acquire), 1);
        drop(rt);
        assert_eq!(drops.load(Ordering::Acquire), 3);
    }

    #[test]
    fn whole_chain_remap_preserves_allocation_and_stateful_processor_history() {
        struct History(u32);
        impl Processor for History {
            fn name(&self) -> &str { "history" }
            fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
                self.0 += 1;
                left.fill(self.0 as f32);
                right.copy_from_slice(left);
            }
        }
        let (mut control, mut rt) = channel(48_000, 64);
        assert!(control.set_chain(0, vec![Box::new(History(0)), Box::new(GainProcessor(0.5))]));
        rt.process(&mut [0.0; 2], 2);
        let allocation = &**rt.chains[0].as_ref().unwrap() as *const ChainSwap;
        let storage = rt.chains[0].as_ref().unwrap().chain.as_ptr();
        let (mut left, mut right) = ([0.0], [0.0]);
        rt.chains[0].as_mut().unwrap().chain[0].process(&mut left, &mut right);
        assert_eq!(left, [1.0]);
        assert!(control.remap_chains(vec![vec![], vec![
            ChainInput::Retained { track: 0, processor: 0 },
            ChainInput::Retained { track: 0, processor: 1 },
        ]]).is_ok());
        rt.process(&mut [0.0; 2], 2);
        assert_eq!(&**rt.chains[1].as_ref().unwrap() as *const ChainSwap, allocation);
        assert_eq!(rt.chains[1].as_ref().unwrap().chain.as_ptr(), storage);
        rt.chains[1].as_mut().unwrap().chain[0].process(&mut left, &mut right);
        assert_eq!(left, [2.0]);
    }

    #[test]
    fn remap_rejects_aliases_and_returns_owned_layout_when_queue_is_full() {
        let (mut control, mut rt) = channel(48_000, 64);
        assert!(control.set_chain(0, vec![Box::new(GainProcessor(0.5))]));
        assert!(control.remap_chains(vec![vec![
            ChainInput::Retained { track: 0, processor: 0 },
            ChainInput::Retained { track: 0, processor: 0 },
        ]]).is_err());
        assert_eq!(control.chain_lengths[0], 1);
        for _ in 1..MAX_TRACKS {
            assert!(control.remap_chains(vec![vec![ChainInput::Retained { track: 0, processor: 0 }]]).is_ok());
        }
        let revision = control.chain_revision;
        let rejected = control.remap_chains(vec![vec![ChainInput::New(Box::new(GainProcessor(0.25)))]]).err().unwrap();
        assert!(matches!(&rejected[0][0], ChainInput::New(_)));
        assert_eq!(control.chain_revision, revision);
        assert_eq!(control.chain_lengths[0], 1);
        rt.process(&mut [0.0; 2], 2);
        assert_eq!(control.retired_chain_revision(), revision);
        assert!(control.remap_chains(rejected).is_ok());
    }

    #[test]
    fn timeline_receipt_reports_callback_installation_rather_than_enqueueing() {
        let (mut control, mut rt) = channel(48_000, 64);
        let revision = control.set_timeline(Timeline::new(48_000, Vec::new()));
        assert_eq!(control.applied_timeline_revision(), 0);
        rt.process(&mut [0.0; 2], 2);
        assert_eq!(control.applied_timeline_revision(), revision);
    }

    struct KeyProbe { seen: Arc<std::sync::Mutex<Vec<f32>>> }
    impl Processor for KeyProbe {
        fn name(&self) -> &str { "key probe" }
        fn process(&mut self, _l: &mut [f32], _r: &mut [f32]) {}
        fn process_keyed(&mut self, _l: &mut [f32], _r: &mut [f32], key: (&[f32], &[f32]), _events: &[(u32, u32, f64)]) -> bool {
            self.seen.lock().unwrap().extend_from_slice(key.0);
            true
        }
    }

    #[test]
    fn a_sidechain_route_keys_a_later_or_earlier_track_with_its_source_delayed() {
        for (source, dest) in [(1u16, 0u16), (0, 1)] {
            let (mut control, mut rt) = channel(48_000, 64);
            let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
            let mut chain: Chain = vec![Box::new(GainProcessor(1.0)), Box::new(KeyProbe { seen: seen.clone() })];
            chain.shrink_to_fit();
            assert!(control.set_chain(dest as usize, chain));
            let ramp: Arc<[f32]> = (0..32).map(|i| i as f32 + 1.0).collect::<Vec<_>>().into();
            let mut timeline = Timeline::new(48_000, vec![
                Clip { start: 0, samples: ramp, gain: 1.0, track: source },
                Clip { start: 0, samples: vec![0.0; 32].into(), gain: 1.0, track: dest },
            ]);
            timeline.sidechains = vec![SidechainRoute { source, track: dest, processor: 1, delay: 3 }];
            control.set_timeline(timeline);
            control.play();
            rt.process(&mut [0.0; 16], 1);
            rt.process(&mut [0.0; 16], 1);
            let seen = seen.lock().unwrap();
            // The key is the source's pre-fader signal, three samples late.
            assert_eq!(&seen[..6], &[0.0, 0.0, 0.0, 1.0, 2.0, 3.0], "source {source} → {dest}");
            assert_eq!(seen.len(), 32);
        }
    }
}
