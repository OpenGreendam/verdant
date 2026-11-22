//! The driver budget and the graph using it are one generation. These tests
//! exercise real callback/resampler paths without requiring an audio device.
use std::sync::{Arc,atomic::{AtomicU64,Ordering}};
use verdant_audio::{CallbackRenderer,Clip,Timeline,Processor};
use verdant_audio::engine::channel;
use verdant_audio::callback::source_callback_frame_bound;

struct Counter(Arc<AtomicU64>);
impl Processor for Counter {
    fn name(&self)->&str{"counter"}
    fn process(&mut self,left:&mut[f32],_:&mut[f32]){self.0.fetch_add(left.len()as u64,Ordering::Relaxed);}
}
struct Gain(f32);
impl Processor for Gain{
    fn name(&self)->&str{"gain"}
    fn process(&mut self,left:&mut[f32],right:&mut[f32]){for v in left.iter_mut().chain(right){*v*=self.0;}}
}
fn material()->Timeline{
    Timeline::new(48_000,vec![Clip{start:0,samples:vec![0.1;200_000].into(),gain:1.,track:0}])
}
#[test]
fn device_switch_waits_for_matching_timeline_and_preserves_headless_clients(){
    let(mut control,mut rt)=channel(48_000,8192);
    let calls=Arc::new(AtomicU64::new(0));
    control.require_callback_budget();
    let first=control.shared.publish_callback_budget(624);
    assert_eq!(control.shared.publish_callback_budget(624),first);
    assert!(control.set_chain(0,vec![Box::new(Counter(calls.clone()))]));
    control.set_timeline_for_callback_budget(material(),first);control.play();
    let mut out=[0.;64];rt.process(&mut out,2);
    assert_eq!(control.position(),32);assert_eq!(calls.load(Ordering::Relaxed),32);
    assert!(out.iter().all(|x|(*x-0.1).abs()<1e-6));

    let second=control.shared.publish_callback_budget(1168);
    assert_ne!(first,second);
    for _ in 0..3{rt.process(&mut out,2);assert_eq!(out,[0.;64]);}
    assert_eq!(control.position(),32);assert_eq!(calls.load(Ordering::Relaxed),32);
    control.set_timeline_for_callback_budget(material(),first);
    rt.process(&mut out,2);assert_eq!(out,[0.;64]);assert_eq!(control.position(),32);
    assert!(control.set_chain(0,vec![Box::new(Gain(0.5))]));
    control.set_timeline_for_callback_budget(material(),second);
    rt.process(&mut out,2);
    assert_eq!(control.position(),64);assert!(out.iter().all(|x|(*x-0.05).abs()<1e-6));

    // Returning to the same frame bound still needs the new generation.
    let third=control.shared.publish_callback_budget(624);
    assert_ne!(third,first);
    control.set_timeline_for_callback_budget(material(),first);rt.process(&mut out,2);
    assert_eq!(out,[0.;64]);assert_eq!(control.position(),64);
    control.set_timeline_for_callback_budget(material(),third);rt.process(&mut out,2);
    assert_eq!(control.position(),96);

    let(mut offline,mut rt)=channel(48_000,8192);
    offline.shared.publish_callback_budget(624);
    offline.set_timeline(material());offline.play();rt.process(&mut out,2);
    assert_eq!(offline.position(),32);assert!(out.iter().any(|x|*x!=0.));
}
#[test]
fn negotiated_bound_rejects_entire_callbacks_before_pcm_or_clock_advance(){
    for adapted in [false,true]{
        let(mut control,rt)=channel(48_000,8192);
        control.set_timeline(material());control.play();
        let mut renderer=CallbackRenderer::new(rt,48_000);
        if adapted{renderer.set_device_rate(44_100).unwrap();}
        renderer.set_callback_frame_bound(64).unwrap();
        assert!(renderer.set_callback_frame_bound(0).is_err());
        assert!(renderer.set_callback_frame_bound(8193).is_err());
        let mut out=[1.;130];
        let error=renderer.process_f32(&mut out,2).unwrap_err();
        assert_eq!((error.frames,error.max_frames),(65,64));
        assert_eq!(out,[0.;130]);assert_eq!(control.position(),0);
        let mut numeric=[1i16;130];
        assert_eq!(renderer.process(&mut numeric,2).unwrap_err().max_frames,64);
        assert_eq!(numeric,[0;130]);assert_eq!(control.position(),0);
        renderer.process_f32(&mut out[..128],2).unwrap();
        assert!(control.position()>0);
    }
}
#[test]
fn conservative_source_burst_covers_real_resampler_prefetch_and_rate_changes(){
    let(mut control,rt)=channel(48_000,8192);
    let frames=Arc::new(AtomicU64::new(0));
    assert!(control.set_chain(0,vec![Box::new(Counter(frames.clone()))]));
    control.set_timeline(material());control.play();
    let mut renderer=CallbackRenderer::new(rt,48_000);
    for rate in [44_100,48_000,96_000,192_000,44_100]{
        renderer.set_device_rate(rate).unwrap();
        for bound in [64,480,1024,8192]{
            renderer.set_callback_frame_bound(bound).unwrap();
            for n in [1,bound,bound-1,17]{
                let before=frames.load(Ordering::Relaxed);
                let mut output=vec![0.;n*2];renderer.process_f32(&mut output,2).unwrap();
                let consumed=(frames.load(Ordering::Relaxed)-before)as usize;
                assert!(consumed<=source_callback_frame_bound(48_000,rate,n),
                    "rate={rate} frames={n} consumed={consumed}");
                assert!(output.iter().all(|x|x.is_finite()));
            }
        }
    }
}
#[test]
fn resampler_holds_prefetch_and_phase_under_an_obsolete_budget(){
    let(mut control,rt)=channel(48_000,8192);
    control.require_callback_budget();
    let first=control.shared.publish_callback_budget(702);
    control.set_timeline_for_callback_budget(material(),first);control.play();
    let mut renderer=CallbackRenderer::new(rt,48_000);
    renderer.set_device_rate(44_100).unwrap();
    renderer.set_callback_frame_bound(512).unwrap();
    let mut output=[0.;1024];renderer.process_f32(&mut output,2).unwrap();
    let position=control.position();
    let second=control.shared.publish_callback_budget(1280);
    for _ in 0..5{
        output.fill(1.);renderer.process_f32(&mut output,2).unwrap();
        assert_eq!(output,[0.;1024]);assert_eq!(control.position(),position);
    }
    control.set_timeline_for_callback_budget(material(),first);
    renderer.process_f32(&mut output,2).unwrap();
    assert_eq!(control.position(),position);assert_eq!(output,[0.;1024]);
    control.set_timeline_for_callback_budget(material(),second);
    renderer.process_f32(&mut output,2).unwrap();
    assert!(control.position()>position);assert!(output.iter().any(|x|*x!=0.));
}
