//! Tracks the one preparation already requested for a timeline generation.
//! A transport waiting for a receipt must not continually supersede slow media
//! jobs. Explicit document changes can still start a newer generation.
#[derive(Clone, Debug, Default)]
pub struct TimelinePreparationGate { in_flight: Option<u64> }
impl TimelinePreparationGate {
    pub fn begin(&mut self, generation:u64) { self.in_flight=Some(generation); }
    pub fn needs_receipt_prepare(&self,generation:u64)->bool { self.in_flight!=Some(generation) }
    /// An obsolete completion cannot clear the newer job's in-flight marker.
    pub fn finish(&mut self,generation:u64)->bool {
        if self.in_flight==Some(generation){self.in_flight=None;true}else{false}
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use std::sync::{Arc,Barrier,mpsc};
    use verdant_domain::{TempoMap,Track};
    use verdant_media::{AudioClip,AudioSource,DecodedAudio,prepare_clip};

    fn slow_source()->(AudioClip,DecodedAudio){
        let samples:Vec<f32>=(0..96_000).flat_map(|i|{
            let sample=(i as f32*std::f32::consts::TAU*220.0/48_000.0).sin()*0.5;[sample,-sample]
        }).collect();
        let audio=DecodedAudio{sample_rate:48_000,channels:2,samples:samples.into(),sha256:"ab".repeat(32),byte_len:1};
        let source=AudioSource{path:std::env::temp_dir().join("timeline-gate.wav").to_string_lossy().into_owned(),sha256:audio.sha256.clone(),byte_len:1,sample_rate:48_000,channels:2,frames:96_000};
        let mut clip=AudioClip::new(source,Track::new("audio","blue").id,0);clip.playback_rate=0.5;clip.preserve_pitch=true;
        (clip,audio)
    }
    #[test]
    fn receipt_polls_do_not_starve_a_real_pitch_preserving_prepare(){
        let (clip,audio)=slow_source();let gate=Arc::new(Barrier::new(2));let work_gate=gate.clone();
        let (sender,receiver)=mpsc::channel();let mut pending=TimelinePreparationGate::default();let mut generation=4;
        pending.begin(generation);let job_generation=generation;
        let job=std::thread::spawn(move||{work_gate.wait();let output=prepare_clip(&clip,&audio,&TempoMap::constant(120.0),48_000,0).unwrap();sender.send((job_generation,output)).unwrap();});
        gate.wait();
        // These are the same requests made by the held transport on each UI
        // pump. Even a delayed delivery must leave the actual job current.
        for _ in 0..100_000{if pending.needs_receipt_prepare(generation){generation+=1;pending.begin(generation);}}
        let (completed,output)=receiver.recv_timeout(std::time::Duration::from_secs(20)).unwrap();
        job.join().unwrap();assert_eq!(generation,4);assert_eq!(completed,generation);assert!(pending.finish(completed));
        assert_eq!(output.frames(),192_000);assert!(output.samples.iter().all(|sample|sample.is_finite()));
    }
    #[test]
    fn an_older_slow_completion_cannot_clear_or_replace_a_new_document_job(){
        let (clip,audio)=slow_source();let (sender,receiver)=mpsc::channel();
        let mut gate=TimelinePreparationGate::default();gate.begin(8);
        let job=std::thread::spawn(move||{let output=prepare_clip(&clip,&audio,&TempoMap::constant(120.0),48_000,0).unwrap();sender.send((8,output.frames())).unwrap();});
        gate.begin(9);
        let (generation,frames)=receiver.recv_timeout(std::time::Duration::from_secs(20)).unwrap();job.join().unwrap();
        assert_eq!(frames,192_000);assert!(!gate.finish(generation));assert!(!gate.needs_receipt_prepare(9));
        assert!(gate.finish(9));assert!(gate.needs_receipt_prepare(9));
    }
}
