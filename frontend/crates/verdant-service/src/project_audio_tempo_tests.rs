//! Transport tests use the actual curve compiler, delay DSP and latency pad.
//! The ignored binding check additionally starts real isolated plugin hosts.
use super::*;
use verdant_audio::automation_clock::{NativeParameterLane,NativePluginSchedule,SampleCurve};
use verdant_audio::Processor;
use verdant_plugins::builtin_dsp::{self as dsp,TableEffect};

fn project() -> Project {
    let mut project=Project::new_empty();
    project.tempo_map=vec![
        verdant_domain::model::TempoPoint{tick:0,micro_bpm:120_000_000,unknown:Default::default()},
        verdant_domain::model::TempoPoint{tick:project.ppq,micro_bpm:60_000_000,unknown:Default::default()},
        verdant_domain::model::TempoPoint{tick:project.ppq*2,micro_bpm:400_000_000,unknown:Default::default()},
    ];
    project
}

fn schedule(curve:SampleCurve) -> NativePluginSchedule {
    NativePluginSchedule{instance_id:"transport-test".into(),parameters:vec![NativeParameterLane{
        native_id:dsp::TRANSPORT_TEMPO_EVENT,fallback:curve.value_at(0,120.),curve,
    }]}
}

#[test]
fn transport_preserves_high_bpm_and_only_restricts_enabled_sync() {
    let mut project=project();
    let curve=transport_tempo_curve(&project,48_000).unwrap();
    assert_eq!(curve.value(72_000),Some(400.));
    assert_eq!(admit_transport_tempo(&curve,true),Ok(true));
    project.tempo_map[0].micro_bpm=10_000_000;
    let slow=transport_tempo_curve(&project,48_000).unwrap();
    assert_eq!(slow.value(0),Some(10.));
    assert!(admit_transport_tempo(&slow,true).unwrap_err().contains("20–1000"));
    assert_eq!(admit_transport_tempo(&slow,false),Ok(false));
    // No Delay means no extra BPM admission restriction on normal playback.
    let mut timeline=Timeline::new(48_000,Vec::new());
    timeline.automation=verdant_backend::automation_plan::compile_automation(&project,48_000).unwrap();
    bind_native_automation(&project,&mut timeline,&NativeBindings{tracks:vec![vec![]],master:vec![]}).unwrap();
}

#[test]
fn delayed_boundaries_reconstruct_at_exact_samples_and_across_loop_wraps() {
    let curve=transport_tempo_curve(&project(),48_000).unwrap();
    for delay in [0,123,9472,160_001] {
        let shifted=delayed_transport_curve(&curve,delay,None).unwrap();
        for sample in [0,23_999,24_000,71_999,72_000,90_000] {
            assert_eq!(shifted.value(sample+u64::from(delay)),curve.value(sample));
        }
        let linear_schedule=schedule(shifted);
        let mut events=[(0,0,0.);4];
        let count=linear_schedule.write_events(23_998+u64::from(delay),8,&mut events).unwrap();
        assert_eq!(&events[..count],&[(dsp::TRANSPORT_TEMPO_EVENT,0,120.),(dsp::TRANSPORT_TEMPO_EVENT,2,60.)]);
        assert_eq!(linear_schedule.write_events(72_000+u64::from(delay),1,&mut events).unwrap(),1);
        assert_eq!(events[0],(dsp::TRANSPORT_TEMPO_EVENT,0,400.));

        let (start,end)=(10_000,80_000);
        let length=end-start;
        let wrapped=delayed_transport_curve(&curve,delay,Some((start,end))).unwrap();
        for position in (start..end).step_by(97).chain([start,end-1,11_472,19_472,33_472]) {
            let source=start+(position-start+length-u64::from(delay)%length)%length;
            assert_eq!(wrapped.value(position),curve.value(source),"loop position {position}, delay {delay}");
        }
        // Every random access, including a loop restart, emits the translated
        // value at offset zero; no callback-local event history is required.
        let loop_schedule=schedule(wrapped.clone());
        for position in [end-1,start,start+10_000,start] {
            loop_schedule.write_events(position,1,&mut events).unwrap();
            assert_eq!(events[0],(dsp::TRANSPORT_TEMPO_EVENT,0,wrapped.value(position).unwrap()));
        }
    }
}

fn padded(input:&[f32],frames:u32)->Vec<f32> {
    let mut output=vec![0.;input.len()+frames as usize];
    output[..input.len()].copy_from_slice(input);
    let mut right=output.clone();
    verdant_plugins::DelayPad::new(frames).process(&mut output,&mut right);
    assert_eq!(output,right);
    output
}

fn delayed_audio(input:&[f32],curve:SampleCurve)->Vec<f32> {
    let mut delay=dsp::DelayProcessor::new(48_000.);
    for (id,value) in [("sync",1.),("mix",100.),("feedback",0.),("low_cut",20.),("high_cut",20_000.)] {
        delay.set(id,value).unwrap();
    }
    let schedule=schedule(curve);
    let mut output=vec![0.;input.len()];let mut right=output.clone();
    let mut events=[(0,0,0.);8];
    for start in (0..input.len()).step_by(257) {
        let end=(start+257).min(input.len());
        let count=schedule.write_events(start as u64,end-start,&mut events).unwrap();
        let mut from=start;
        for (index,&(_,offset,bpm)) in events[..count].iter().enumerate() {
            let at=start+offset as usize;
            if from<at { delay.process(&input[from..at],&input[from..at],None,&mut output[from..at],&mut right[from..at]); }
            delay.set_tempo(bpm);
            from=at;
            if index+1==count { delay.process(&input[from..end],&input[from..end],None,&mut output[from..end],&mut right[from..end]); }
        }
    }
    assert_eq!(output,right);
    output
}

#[test]
fn two_insert_stages_and_master_match_after_physical_input_latency() {
    let curve=transport_tempo_curve(&project(),48_000).unwrap();
    let input:Vec<f32>=(0..100_000).map(|sample|if sample<85_000 {
        0.2*(std::f64::consts::TAU*173.0*sample as f64/48_000.).sin()as f32
    }else{0.}).collect();
    let bridge=verdant_plugins::shm::TRANSPORT_LATENCY_FRAMES as u32;
    // Gain's RT bridge precedes insert A, A's bridge precedes insert B,
    // then B's bridge plus parallel-track compensation precede the master.
    let gaps=[bridge,bridge,bridge+311];
    let mut reference=input.clone();let mut realtime=input;let mut prefix=0;
    for gap in gaps {
        reference=delayed_audio(&reference,curve.clone());
        realtime=padded(&realtime,gap);prefix+=gap;
        realtime=delayed_audio(&realtime,delayed_transport_curve(&curve,prefix,None).unwrap());
    }
    assert!(reference.iter().any(|sample|sample.abs()>0.1));
    assert!(realtime[..prefix as usize].iter().all(|sample|*sample==0.));
    assert_eq!(&realtime[prefix as usize..],reference.as_slice());
}

#[test]
#[ignore="requires a freshly built verdant-plughost in VERDANT_PLUGIN_HOST"]
fn real_host_bindings_keep_legacy_sidechains_and_tempo_on_physical_slots() {
    let host=std::path::PathBuf::from(std::env::var_os("VERDANT_PLUGIN_HOST").expect("VERDANT_PLUGIN_HOST"));
    let client=verdant_plugins::PluginClient::with_plughost(verdant_plugins::PluginClientOpts{
        plughost_path:Some(host.clone()),cache_dir:std::env::temp_dir().join(format!("verdant-tempo-{}",uuid::Uuid::new_v4())),
        sample_rate:48_000.,max_frames:8192,
    },host).unwrap();
    let mut project=project();project.tracks.push(verdant_domain::Track::new("key source","blue"));
    let mut first=verdant_plugins::InsertSpec::new("builtin",dsp::DELAY_ID,"Delay A","");first.instance_id.clear();
    let mut second=first.clone();second.name="Delay B".into();second.extra.insert(SIDECHAIN_KEY.into(),project.tracks[1].id.to_string().into());
    let mut master=verdant_plugins::InsertSpec::new("builtin",dsp::DELAY_ID,"Master Delay","");
    master.extra.insert(SIDECHAIN_KEY.into(),project.tracks[1].id.to_string().into());
    verdant_plugins::project::write_inserts(&mut project.tracks[0].unknown,&[first.clone(),second.clone()]);
    project.extensions.insert(verdant_domain::automation::MASTER_INSERTS_KEY.into(),serde_json::to_value([master.clone()]).unwrap());
    let a=client.insert(first).unwrap();let b=client.insert(second).unwrap();let m=client.insert(master.clone()).unwrap();
    for handle in [&a,&b,&m] { handle.set_param("sync",1.).unwrap(); }
    let bind=|handle:&Arc<verdant_plugins::InsertHandle>,processor|NativeBinding{instance_id:handle.spec().instance_id.clone(),processor:Some(processor),bypass:false,handle:Some(handle.clone())};
    let bindings=NativeBindings{tracks:vec![vec![bind(&a,0),bind(&b,1)],vec![]],master:vec![bind(&m,0)]};
    let mut timeline=Timeline::new(48_000,Vec::new());timeline.automation=verdant_backend::automation_plan::compile_automation(&project,48_000).unwrap();
    bind_native_automation(&project,&mut timeline,&bindings).unwrap();
    assert_eq!(timeline.sidechains.len(),2);
    assert_eq!((timeline.sidechains[0].source,timeline.sidechains[0].track,timeline.sidechains[0].processor),(1,0,1));
    assert_eq!(timeline.sidechains[0].delay,a.latency_samples());
    assert_eq!((timeline.sidechains[1].source,timeline.sidechains[1].track,timeline.sidechains[1].processor),
        (1,verdant_audio::engine::MASTER_BUS as u16,0));
    assert_eq!(timeline.sidechains[1].delay,a.latency_samples()+b.latency_samples());
    let first_curve=&timeline.native_automation[0][0].as_ref().unwrap().parameters[0].curve;
    let second_curve=&timeline.native_automation[0][1].as_ref().unwrap().parameters[0].curve;
    let master_curve=&timeline.master_native_automation[0].as_ref().unwrap().parameters[0].curve;
    assert_eq!(first_curve.value(24_000),Some(60.));
    assert_eq!(second_curve.value(24_000),Some(120.));
    assert_eq!(second_curve.value(24_000+u64::from(a.latency_samples())),Some(60.));
    let master_prefix=a.latency_samples()+b.latency_samples();
    assert_eq!(master_curve.value(24_000+u64::from(master_prefix)-1),Some(120.));
    assert_eq!(master_curve.value(24_000+u64::from(master_prefix)),Some(60.));
    for handle in [&a,&b,&m] { client.remove(handle.key()); }
}
