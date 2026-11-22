//! Allocation-free RT bridge. A bounded pipeline covers the maximum complete
//! callback burst and partial-quantum alignment. Only the
//! separately owned offline handle may wait for audio.

use crate::shm::{Ring, Wake, DSP_QUANTUM_FRAMES, MAX_BLOCK_FRAMES,
    TRANSPORT_LATENCY_FRAMES, STATUS_OK};
use crate::proto::{validate_event_quanta, ParamEvent, MAX_PARAM_EVENTS};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct RemoteState {
    pub bypass: AtomicBool,
    transport_frames: AtomicU32,
    pub xruns: AtomicU64,
    /// Latched for the UI/control thread; callback never sends a channel event.
    pub automation_fault: AtomicBool,
    fault_kind: AtomicU32,
    fault_sequence: AtomicU64,
    fault_input_sequence: AtomicU64,
    fault_output_sequence: AtomicU64,
    fault_event_count: AtomicU64,
    fault_status: AtomicU32,
}
/// Entire physical callbacks may run before the child process gets CPU.
/// Account for a partial native quantum retained from the previous callback.
pub fn transport_latency_for_source_burst(source_frames:u32)->Option<u32>{
    if source_frames==0{return None;}
    let quantum=DSP_QUANTUM_FRAMES as u32;
    let frames=source_frames.checked_add(quantum-1)?.div_ceil(quantum).checked_mul(quantum)?;
    (frames<=TRANSPORT_LATENCY_FRAMES as u32).then_some(frames)
}

impl RemoteState {
    pub fn transport_latency_samples(&self)->u32{
        let frames=self.transport_frames.load(Ordering::Acquire);
        if frames==0{TRANSPORT_LATENCY_FRAMES as u32}else{frames}
    }
    /// Called only behind the engine's device-budget barrier, before matching
    /// PDC is installed. Host ownership, DSP state and ring sequences survive.
    pub fn configure_callback_source_frames(&self,frames:u32)->bool{
        let Some(latency)=transport_latency_for_source_burst(frames)else{return false;};
        self.transport_frames.store(latency,Ordering::Release);true
    }
    // One callback owns the writer. Publish the kind last so the control
    // thread sees an immutable, coherent first-failure snapshot.
    fn record_fault(&self,kind:u32,sequence:u64,ring:&Ring,event_count:usize) {
        if self.fault_kind.load(Ordering::Relaxed)!=0{return;}
        self.fault_sequence.store(sequence,Ordering::Relaxed);
        self.fault_input_sequence.store(ring.in_seq(),Ordering::Relaxed);
        self.fault_output_sequence.store(ring.out_seq(),Ordering::Relaxed);
        self.fault_event_count.store(event_count as u64,Ordering::Relaxed);
        self.fault_status.store(ring.status(),Ordering::Relaxed);
        self.fault_kind.store(kind,Ordering::Release);
    }
    /// Control-thread diagnostic; formatting never occurs in the callback.
    pub fn fault_detail(&self)->Option<String>{
        let kind=self.fault_kind.load(Ordering::Acquire);
        let reason=match kind{1=>"invalid callback or event admission",2=>"host unavailable",
            3=>"native input queue unavailable",_=>return None};
        Some(format!("{reason}; quantum={}, input={}, output={}, events={}, host_status={}",
            self.fault_sequence.load(Ordering::Relaxed),self.fault_input_sequence.load(Ordering::Relaxed),
            self.fault_output_sequence.load(Ordering::Relaxed),self.fault_event_count.load(Ordering::Relaxed),
            self.fault_status.load(Ordering::Relaxed)))
    }
}

/// The engine retires processors on its control thread. The handle retains
/// the map/process owner while the callback holds these RT endpoints.
pub struct RemoteInsert {
    ring: Arc<Ring>,
    in_w: Wake,
    // Retain the readable endpoint without polling/draining it in RT.
    _out_r: Wake,
    state: Arc<RemoteState>,
    name: String,
    seq: u64,
    transport_quanta: u64,
    warm_until: u64,
    at: usize,
    input: [Vec<f32>; 2],
    key: [Vec<f32>; 2],
    keyed: bool,
    output: [Vec<f32>; 2],
    output_ready: bool,
    events: Vec<ParamEvent>,
    event_count: usize,
}

impl RemoteInsert {
    pub fn new(ring: Arc<Ring>, in_w: Wake, out_r: Wake, state: Arc<RemoteState>, name: String) -> Self {
        let frames = ring.max_frames();
        assert_eq!(frames, DSP_QUANTUM_FRAMES, "RT ring must use the native DSP quantum");
        let transport_quanta=u64::from(state.transport_latency_samples())/DSP_QUANTUM_FRAMES as u64;
        Self {
            ring, in_w, _out_r: out_r, state, name, seq: 0, transport_quanta,warm_until:transport_quanta,at: 0,
            input: [vec![0.0; frames], vec![0.0; frames]],
            key: [vec![0.0; frames], vec![0.0; frames]],
            keyed: false,
            output: [vec![0.0; frames], vec![0.0; frames]],
            output_ready: true,
            events: vec![(0, 0, 0.0); MAX_PARAM_EVENTS], event_count: 0,
        }
    }

    pub fn state(&self) -> &Arc<RemoteState> { &self.state }
    pub fn transport_latency_samples(&self) -> u32 { self.transport_quanta as u32*DSP_QUANTUM_FRAMES as u32 }
    pub fn latency_samples(&self) -> u32 {
        self.ring.plugin_latency().saturating_add(self.transport_latency_samples())
    }
    pub fn set_bypass(&self, on: bool) { self.state.bypass.store(on, Ordering::Release); }
    pub fn xruns(&self) -> u64 { self.state.xruns.load(Ordering::Relaxed) }
}

impl RemoteInsert {
    /// Process a complete call using caller-owned, precompiled native events.
    /// Admission includes the partial native quantum retained from prior calls.
    /// No quantum is published until this entire event slice has been checked.
    pub fn process_events(&mut self, left: &mut [f32], right: &mut [f32], events: &[ParamEvent]) -> bool {
        self.process_keyed(left, right, None, events)
    }

    /// As `process_events`, plus a sidechain key aligned with `left`/`right`.
    /// A quantum is keyed when any of its frames arrived with a key.
    pub fn process_keyed(&mut self, left: &mut [f32], right: &mut [f32], key: Option<(&[f32], &[f32])>,
        events: &[ParamEvent]) -> bool {
        let frames = left.len();
        if frames != right.len() || frames > MAX_BLOCK_FRAMES
            || key.is_some_and(|(kl, kr)| kl.len() != frames || kr.len() != frames)
            || validate_event_quanta(events, frames, self.ring.max_frames(), self.at, self.event_count).is_err() {
            self.state.record_fault(1,self.seq,&self.ring,events.len());
            self.state.automation_fault.store(true, Ordering::Release);
            self.state.xruns.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        if frames == 0 { return true; }
        if self.ring.status() != STATUS_OK {
            self.state.record_fault(2,self.seq,&self.ring,events.len());
            self.state.xruns.fetch_add(1, Ordering::Relaxed);
            if !events.is_empty() { self.state.automation_fault.store(true, Ordering::Release); }
            return false;
        }
        let quantum = self.ring.max_frames();
        let transport_quanta=u64::from(self.state.transport_latency_samples())/quantum as u64;
        if transport_quanta!=self.transport_quanta{
            self.transport_quanta=transport_quanta;
            self.warm_until=self.seq.saturating_add(transport_quanta);
            self.output[0].fill(0.);self.output[1].fill(0.);self.output_ready=true;
        }
        let mut offset = 0;
        let mut next_event = 0;
        let mut accepted = true;
        while offset < frames {
            if self.at == 0 && self.seq >= self.warm_until {
                let [ol, orr] = &mut self.output;
                self.output_ready = self.ring.try_read_output(self.seq - self.transport_quanta, ol, orr);
                if !self.output_ready { self.state.xruns.fetch_add(1, Ordering::Relaxed); }
            }
            let n = (quantum - self.at).min(frames - offset);
            let range = self.at..self.at + n;
            let src = offset..offset + n;
            let direct = self.at == 0 && n == quantum;
            let first_event = next_event;
            while next_event < events.len() && (events[next_event].1 as usize) < offset + n {
                if !direct {
                    // Partial calls must retain their events beside the audio.
                    let (native_id, sample_offset, value) = events[next_event];
                    self.events[self.event_count] = (native_id, (self.at + sample_offset as usize - offset) as u32, value);
                    self.event_count += 1;
                }
                next_event += 1;
            }
            self.input[0][range.clone()].copy_from_slice(&left[src.clone()]);
            self.input[1][range.clone()].copy_from_slice(&right[src.clone()]);
            if let Some((kl, kr)) = key {
                self.key[0][range.clone()].copy_from_slice(&kl[src.clone()]);
                self.key[1][range.clone()].copy_from_slice(&kr[src.clone()]);
                self.keyed = true;
            } else if self.keyed {
                self.key[0][range.clone()].fill(0.0);
                self.key[1][range.clone()].fill(0.0);
            }
            if !self.state.bypass.load(Ordering::Acquire) {
                if self.output_ready {
                    left[src.clone()].copy_from_slice(&self.output[0][range.clone()]);
                    right[src].copy_from_slice(&self.output[1][range]);
                } else {
                    // An underrun is a wet-path gap, never an unsolicited
                    // unprocessed signal at a different PDC position.
                    left[src.clone()].fill(0.);right[src].fill(0.);
                }
            }
            // Bypass still advances native DSP and automation time.
            self.at += n;
            offset += n;
            if self.at == quantum {
                let (quantum_events, source_offset) = if direct {
                    (&events[first_event..next_event], (offset - n) as u32)
                } else {
                    (&self.events[..self.event_count], 0)
                };
                let key = self.keyed.then_some((&self.key[0][..], &self.key[1][..]));
                if self.ring.try_write_input_keyed(self.seq, &self.input[0], &self.input[1], key, quantum_events, source_offset) {
                    self.in_w.signal();
                } else {
                    self.state.xruns.fetch_add(1, Ordering::Relaxed);
                    if !quantum_events.is_empty() {
                        self.state.record_fault(3,self.seq,&self.ring,quantum_events.len());
                        self.state.automation_fault.store(true, Ordering::Release);
                        accepted = false;
                    }
                }
                self.seq += 1;
                self.at = 0;
                self.event_count = 0;
                self.keyed = false;
            }
        }
        accepted
    }
}

impl verdant_audio::Processor for RemoteInsert {
    fn name(&self) -> &str { &self.name }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.process_events(left, right, &[]);
    }

    fn process_automated(&mut self, left: &mut [f32], right: &mut [f32], events: &[(u32, u32, f64)]) -> bool {
        self.process_events(left, right, events)
    }

    fn process_keyed(&mut self, left: &mut [f32], right: &mut [f32], key: (&[f32], &[f32]),
        events: &[(u32, u32, f64)]) -> bool {
        RemoteInsert::process_keyed(self, left, right, Some(key), events)
    }
}

/// Preallocated sample delay for compensating shorter insert chains.
pub struct DelayPad {
    frames: usize,
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    head: usize,
}

impl DelayPad {
    pub fn new(frames: u32) -> Self {
        let n = frames as usize;
        Self { frames: n, buf_l: vec![0.0; n.max(1)], buf_r: vec![0.0; n.max(1)], head: 0 }
    }
    pub fn latency_samples(&self) -> u32 { self.frames as u32 }
}

impl verdant_audio::Processor for DelayPad {
    fn name(&self) -> &str { "DelayPad" }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        if self.frames == 0 { return; }
        for i in 0..left.len().min(right.len()) {
            std::mem::swap(&mut left[i], &mut self.buf_l[self.head]);
            std::mem::swap(&mut right[i], &mut self.buf_r[self.head]);
            self.head += 1;
            if self.head == self.frames { self.head = 0; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_audio::Processor;

    fn event_processor() -> (RemoteInsert, Arc<Ring>) {
        let channel = crate::shm::Channel::create(DSP_QUANTUM_FRAMES).unwrap();
        let ring = Arc::new(channel.ring);
        let processor = RemoteInsert::new(ring.clone(), channel.app_in_w, channel.app_out_r,
            Arc::new(RemoteState::default()), "events".into());
        (processor, ring)
    }

    #[test]
    fn event_offsets_follow_audio_through_partial_callbacks() {
        let (mut processor, ring) = event_processor();
        assert!(processor.process_automated(&mut [1.0; 17], &mut [2.0; 17], &[(9, 0, 0.25), (9, 16, 0.5)]));
        assert_eq!(ring.in_seq(), 0);
        let mut l = [3.0; 512];
        let mut r = [4.0; 512];
        assert!(processor.process_automated(&mut l, &mut r,
            &[(9, 0, 0.75), (9, 238, 1.0), (9, 239, 0.0), (9, 511, 0.9)]));
        assert_eq!(ring.in_seq(), 2);
        let mut il = [0.0; DSP_QUANTUM_FRAMES];
        let mut ir = il;
        let mut events = vec![(0, 0, 0.0); MAX_PARAM_EVENTS];
        let (frames, count) = ring.read_input_events(0, &mut il, &mut ir, &mut events).unwrap();
        assert_eq!(frames, DSP_QUANTUM_FRAMES);
        assert_eq!(&events[..count], &[(9, 0, 0.25), (9, 16, 0.5), (9, 17, 0.75), (9, 255, 1.0)]);
        assert_eq!(&il[..17], &[1.0; 17]);
        assert_eq!(&il[17..], &[3.0; 239]);
        let (_, count) = ring.read_input_events(1, &mut il, &mut ir, &mut events).unwrap();
        assert_eq!(&events[..count], &[(9, 0, 0.0)]);
        assert!(processor.process_automated(&mut [5.0; 239], &mut [6.0; 239], &[]));
        let (_, count) = ring.read_input_events(2, &mut il, &mut ir, &mut events).unwrap();
        assert_eq!(&events[..count], &[(9, 16, 0.9)]);
    }

    #[test]
    fn later_quantum_overflow_rejects_before_any_publication() {
        let (mut processor, ring) = event_processor();
        let mut events = vec![(1, 256, 0.5); MAX_PARAM_EVENTS + 1];
        events.insert(0, (1, 0, 0.25));
        let mut l = [1.0; 512];
        let mut r = [2.0; 512];
        assert!(!processor.process_automated(&mut l, &mut r, &events));
        assert_eq!(ring.in_seq(), 0);
        assert_eq!(processor.at, 0);
        assert_eq!(processor.seq, 0);
        assert_eq!(l, [1.0; 512]);
        assert_eq!(r, [2.0; 512]);
        assert!(processor.state.automation_fault.load(Ordering::Acquire));
    }

    #[test]
    fn partial_quantum_overflow_does_not_discard_previously_admitted_events() {
        let (mut processor, ring) = event_processor();
        let events = vec![(1, 0, 0.5); MAX_PARAM_EVENTS];
        assert!(processor.process_automated(&mut [1.0; 64], &mut [1.0; 64], &events));
        assert!(!processor.process_automated(&mut [1.0; 192], &mut [1.0; 192], &[(1, 0, 1.0)]));
        assert_eq!(ring.in_seq(), 0);
        assert_eq!(processor.at, 64);
        assert!(processor.process_automated(&mut [1.0; 192], &mut [1.0; 192], &[]));
        let mut l = [0.0; DSP_QUANTUM_FRAMES];
        let mut r = l;
        let mut received = vec![(0, 0, 0.0); MAX_PARAM_EVENTS];
        assert_eq!(ring.read_input_events(0, &mut l, &mut r, &mut received).unwrap(),
            (DSP_QUANTUM_FRAMES, MAX_PARAM_EVENTS));
        assert_eq!(received, events);
    }

    #[test]
    fn delay_pad_shifts_exactly() {
        let mut pad = DelayPad::new(4);
        let mut l: Vec<f32> = (1..=16).map(|i| i as f32).collect();
        let mut r = l.clone();
        pad.process(&mut l, &mut r);
        assert_eq!(&l[..4], &[0.0; 4]);
        assert_eq!(&l[4..], &(1..=12).map(|i| i as f32).collect::<Vec<_>>()[..]);
    }

    #[test]
    fn delay_pad_zero_is_noop() {
        let mut pad = DelayPad::new(0);
        let mut l = vec![1.0; 8];
        let mut r = vec![2.0; 8];
        pad.process(&mut l, &mut r);
        assert_eq!(l, vec![1.0; 8]);
        assert_eq!(r, vec![2.0; 8]);
    }

    #[test]
    fn callback_bursts_only_use_previously_published_input() {
        let chan = crate::shm::Channel::create(DSP_QUANTUM_FRAMES).unwrap();
        let ring = Arc::new(chan.ring);
        let mut ri = RemoteInsert::new(ring.clone(), chan.app_in_w, chan.app_out_r,
            Arc::new(RemoteState::default()), "t".into());
        let mut l = vec![0.0; MAX_BLOCK_FRAMES];
        let mut r = l.clone();
        let mut il = vec![0.0; DSP_QUANTUM_FRAMES];
        let mut ir = il.clone();
        let sizes = [480, 512, 1024, 278, 279, 441, 557, 8192, 37, 1];
        let mut done = 0;
        let mut frame = 0;
        let mut call = 0;
        while frame < TRANSPORT_LATENCY_FRAMES * 4 {
            let n = sizes[call % sizes.len()];
            for i in 0..n { l[i] = (frame + i + 1) as f32; r[i] = -l[i]; }
            ri.process(&mut l[..n], &mut r[..n]);
            for i in 0..n {
                let expected = if frame + i < TRANSPORT_LATENCY_FRAMES { 0.0 }
                    else { 2.0 * (frame + i + 1 - TRANSPORT_LATENCY_FRAMES) as f32 };
                assert_eq!(l[i], expected, "callback {call}, sample {i}");
                assert_eq!(r[i], -expected);
            }
            // The fake child gets no CPU until the entire callback returns.
            // This catches same-callback dependencies and overwritten bursts.
            while done < ring.in_seq() {
                let n = ring.read_input(done, &mut il, &mut ir).unwrap();
                assert_eq!(n, DSP_QUANTUM_FRAMES);
                for value in il.iter_mut().chain(&mut ir) { *value *= 2.0; }
                ring.write_output(done, &il, &ir).unwrap();
                done += 1;
            }
            frame += n;
            call += 1;
        }
        assert_eq!(ri.latency_samples(), TRANSPORT_LATENCY_FRAMES as u32);
        assert_eq!(ri.xruns(), 0);
    }

    #[test]
    fn negotiated_pipeline_covers_frozen_child_bursts_and_partial_quanta(){
        for (device_frames,device_rate,expected) in [(480,48_000,1024),(1024,48_000,1536),(1024,44_100,1536),(8192,44_100,9472)]{
            let burst=verdant_audio::callback::source_callback_frame_bound(48_000,device_rate,device_frames);
            assert_eq!(transport_latency_for_source_burst(burst as u32),Some(expected));
            let(mut processor,ring)=event_processor();
            assert!(processor.state.configure_callback_source_frames(burst as u32));
            let mut il=[0.;DSP_QUANTUM_FRAMES];let mut ir=il;
            let(mut done,mut frame)=(0,0usize);
            for call in 0..16{
                let n=if call%3==0{burst}else if call%3==1{17}else{burst-1};
                let mut l=(frame..frame+n).map(|i|(i+1)as f32).collect::<Vec<_>>();let mut r=l.iter().map(|x|-x).collect::<Vec<_>>();
                // The child receives no CPU until this entire physical burst
                // has returned, including all engine resampler sub-blocks.
                for (l,r) in l.chunks_mut(128).zip(r.chunks_mut(128)){processor.process(l,r);}
                for i in 0..n{
                    let expected=if frame+i<expected as usize{0.}else{2.*(frame+i+1-expected as usize)as f32};
                    assert_eq!(l[i],expected,"rate={device_rate} burst={burst} call={call} sample={i}");
                    assert_eq!(r[i],-expected);
                }
                while done<ring.in_seq(){
                    ring.read_input(done,&mut il,&mut ir).unwrap();
                    for x in il.iter_mut().chain(&mut ir){*x*=2.;}
                    ring.write_output(done,&il,&ir).unwrap();done+=1;
                }
                frame+=n;
            }
            assert_eq!(processor.xruns(),0);
            assert_eq!(processor.transport_latency_samples(),expected);
        }
        assert_eq!(transport_latency_for_source_burst(0),None);
        assert_eq!(transport_latency_for_source_burst(u32::MAX),None);
    }

    #[test]
    fn retained_processor_rebudgets_without_resetting_host_sequence_or_events(){
        let(mut processor,ring)=event_processor();
        let owner=processor.state.clone();
        let mut il=[0.;DSP_QUANTUM_FRAMES];let mut ir=il;let mut event_buffer=vec![(0,0,0.);MAX_PARAM_EVENTS];
        let(mut done,mut frame)=(0,0usize);let mut received=Vec::new();
        for (stage,burst) in [624,1168,624].into_iter().enumerate(){
            assert!(owner.configure_callback_source_frames(burst));
            let latency=transport_latency_for_source_burst(burst).unwrap()as usize;
            let start=frame;
            for call in 0..24{
                // An odd fragment boundary survives budget handoff.
                let n=if call==23{273}else{256};
                let mut l=(frame..frame+n).map(|i|(i+1)as f32).collect::<Vec<_>>();let mut r=l.clone();
                let event=(7,0,(stage*24+call)as f64);
                assert!(processor.process_events(&mut l,&mut r,&[event]));
                if frame>=start+latency+256{
                    for (i,value) in l.iter().enumerate(){assert_eq!(*value,(frame+i+1-latency)as f32);}
                }
                while done<ring.in_seq(){
                    let(_,count)=ring.read_input_events(done,&mut il,&mut ir,&mut event_buffer).unwrap();
                    for &(id,offset,value) in &event_buffer[..count]{received.push((done*256+u64::from(offset),id,value));}
                    ring.write_output(done,&il,&ir).unwrap();done+=1;
                }
                frame+=n;
            }
            assert!(Arc::ptr_eq(&owner,processor.state()));
        }
        assert!(processor.process_events(&mut [0.;256],&mut [0.;256],&[]));
        while done<ring.in_seq(){
            let(_,count)=ring.read_input_events(done,&mut il,&mut ir,&mut event_buffer).unwrap();
            for &(id,offset,value)in &event_buffer[..count]{received.push((done*256+u64::from(offset),id,value));}
            done+=1;
        }
        assert_eq!(received.len(),72);
        let mut position=0;
        for (index,(sample,id,value))in received.iter().enumerate(){
            assert_eq!((*sample,*id,*value),(position,7,index as f64));
            position+=if index%24==23{273}else{256};
        }
        assert_eq!(processor.xruns(),0);
    }

    #[test]
    fn input_backlog_diagnostic_preserves_the_rejected_event_evidence(){
        let(mut processor,ring)=event_processor();
        for _ in 0..crate::shm::RING_SLOTS{
            assert!(processor.process_events(&mut [0.;256],&mut [0.;256],&[(1,0,0.5)]));
        }
        assert!(!processor.process_events(&mut [0.;256],&mut [0.;256],&[(1,0,0.5)]));
        let detail=processor.state.fault_detail().unwrap();
        assert!(detail.contains("native input queue unavailable"),"{detail}");
        assert!(detail.contains("input=64, output=0, events=1, host_status=0"),"{detail}");
        assert_eq!(ring.in_seq(),64);
        assert!(processor.state.automation_fault.load(Ordering::Acquire));
    }

    #[test]
    fn changed_transport_budget_keeps_short_and_long_chains_phase_aligned(){
        let(mut short,short_ring)=event_processor();
        let(mut long_a,a_ring)=event_processor();let(mut long_b,b_ring)=event_processor();
        let rings=[short_ring,a_ring,b_ring];let mut done=[0u64;3];
        let mut input=[0.;256];let mut right=[0.;256];let mut absolute=0usize;
        for source_burst in [624,1168,624]{
            for processor in [&short,&long_a,&long_b]{assert!(processor.state.configure_callback_source_frames(source_burst));}
            let latency=transport_latency_for_source_burst(source_burst).unwrap()as usize;
            let mut pad=DelayPad::new(latency as u32);
            let stage_start=absolute;
            for _ in 0..24{
                let n=480;
                let source=(absolute..absolute+n).map(|i|(i+1)as f32).collect::<Vec<_>>();
                let(mut sl,mut sr,mut ll,mut lr)=(source.clone(),source.clone(),source.clone(),source);
                short.process(&mut sl,&mut sr);pad.process(&mut sl,&mut sr);
                long_a.process(&mut ll,&mut lr);long_b.process(&mut ll,&mut lr);
                if absolute>=stage_start+latency*2+512{
                    for i in 0..n{
                        let expected=(absolute+i+1-latency*2)as f32;
                        assert_eq!(sl[i],expected);assert_eq!(ll[i],expected);
                        assert_eq!(sr[i],lr[i]);
                    }
                }
                for (ring,done) in rings.iter().zip(&mut done){
                    while *done<ring.in_seq(){
                        ring.read_input(*done,&mut input,&mut right).unwrap();
                        ring.write_output(*done,&input,&right).unwrap();*done+=1;
                    }
                }
                absolute+=n;
            }
        }
        assert_eq!((short.xruns(),long_a.xruns(),long_b.xruns()),(0,0,0));
    }

    #[test]
    fn missing_wet_output_never_leaks_unprocessed_unaligned_dry_audio(){
        let(mut processor,ring)=event_processor();
        assert!(processor.state.configure_callback_source_frames(624));
        let(mut done,mut il,mut ir)=(0,[0.;256],[0.;256]);
        for call in 0..10{
            let(mut l,mut r)=([0.8;256],[0.5;256]);
            assert!(processor.process_events(&mut l,&mut r,&[]));
            if call>=4{assert_eq!(l,[0.2;256]);assert_eq!(r,[0.125;256]);}
            while done<ring.in_seq(){
                ring.read_input(done,&mut il,&mut ir).unwrap();
                for x in il.iter_mut().chain(&mut ir){*x*=0.25;}
                ring.write_output(done,&il,&ir).unwrap();done+=1;
            }
        }
        assert_eq!(processor.xruns(),0);
        // The child now misses two wet deadlines after previously valid PCM.
        // Already published audio keeps its position; later gaps stay silent.
        for call in 0..6{
            let(mut l,mut r)=([0.8;256],[0.5;256]);
            assert!(processor.process_events(&mut l,&mut r,&[]));
            if call<4{assert_eq!(l,[0.2;256]);assert_eq!(r,[0.125;256]);}
            else{assert_eq!(l,[0.;256]);assert_eq!(r,[0.;256]);}
        }
        assert_eq!(processor.xruns(),2);
        assert!(!processor.state.automation_fault.load(Ordering::Acquire));
        processor.set_bypass(true);
        let(mut l,mut r)=([0.8;256],[0.5;256]);
        processor.process(&mut l,&mut r);
        assert_eq!(l,[0.8;256]);assert_eq!(r,[0.5;256]);
    }
}
