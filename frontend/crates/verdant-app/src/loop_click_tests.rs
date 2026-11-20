use super::*;
use verdant_audio::engine::channel;
use verdant_domain::model::{TempoPoint, TimeSignature};

const RATE:u32=48_000;

fn signature(tick:i64,numerator:u32,denominator:u32)->TimeSignature{
    TimeSignature{tick,numerator,denominator,unknown:Default::default()}
}

#[test]
fn click_boundaries_and_accents_follow_three_four_and_tempo_changes(){
    let mut project=Project::new_empty();
    project.time_signatures=vec![signature(0,4,4),signature(3840,3,4)];
    project.tempo_map=vec![TempoPoint{tick:0,micro_bpm:120_000_000,unknown:Default::default()},TempoPoint{tick:4800,micro_bpm:60_000_000,unknown:Default::default()}];
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();
    let clicks=metronome(&project,&tempo,RATE,9600).unwrap();
    let boundaries=clicks.events.iter().map(|event|(event.sample,event.accent)).collect::<Vec<_>>();
    assert_eq!(boundaries,vec![(0,true),(24_000,false),(48_000,false),(72_000,false),(96_000,true),(120_000,false),(168_000,false),(216_000,true),(264_000,false),(312_000,false)]);
    assert_eq!(clicks.length,360_000);
    // The event at 216000 is beat 1 of the second 3/4 measure, rather than
    // beat 4 of an absolute four-quarter-note grid.
    assert_ne!(&*clicks.strong,&*clicks.weak);
}

#[test]
fn count_in_delays_first_source_frame_by_exactly_two_three_four_bars(){
    let mut project=Project::new_empty();
    project.time_signatures=vec![signature(0,3,4)];
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();
    let clicks=count_in_track(&project,&tempo,RATE,0,2).unwrap();
    assert_eq!(clicks.length,144_000);
    assert_eq!(clicks.events.iter().filter(|event|event.accent).map(|event|event.sample).collect::<Vec<_>>(),vec![0,72_000]);
    let pre_roll=clicks.count_in_pcm().unwrap();
    assert_eq!(pre_roll.len(),144_000);
    let(mut control,mut rt)=channel(RATE,257);
    control.set_timeline(Timeline::new(RATE,vec![Clip{start:0,samples:vec![0.5;512].into(),gain:1.,track:0}]));
    assert!(control.play_with_count_in(pre_roll.clone()));
    let mut output=vec![0.;257*2];
    let mut rendered=0usize;
    while rendered<pre_roll.len()+512{
        let frames=257.min(pre_roll.len()+512-rendered);
        rt.process(&mut output[..frames*2],2);
        for(frame,stereo)in output[..frames*2].chunks_exact(2).enumerate(){
            let at=rendered+frame;
            let expected=if at<pre_roll.len(){pre_roll[at]}else{0.5};
            assert_eq!(stereo,[expected,expected],"device frame {at}");
        }
        rendered+=frames;
        assert_eq!(control.position(),rendered.saturating_sub(pre_roll.len())as u64);
        control.maintain();
    }
}

#[test]
fn count_in_rounds_absolute_boundaries_without_per_beat_drift(){
    let mut project=Project::new_empty();
    project.time_signatures=vec![signature(0,3,4)];
    project.tempo_map[0].micro_bpm=123_456_789;
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();
    let clicks=count_in_track(&project,&tempo,RATE,0,8).unwrap();
    for(beat,event)in clicks.events.iter().enumerate(){
        assert_eq!(event.sample,tempo.sample_at(beat as i64*project.ppq,RATE)as u64);
    }
    assert_eq!(clicks.length,tempo.sample_at(24*project.ppq,RATE)as u64);
}

#[test]
fn exported_mix_excludes_live_click_unless_explicitly_enabled(){
    let mut project=Project::new_empty();
    project.time_signatures=vec![signature(0,3,4)];
    let tempo=TempoMap::new(&project.tempo_map,project.ppq).unwrap();
    let source=vec![0.25;256];
    let mut timeline=Timeline::new(RATE,vec![Clip{start:0,samples:source.clone().into(),gain:1.,track:0}]);
    // A cloned live timeline must also be stripped at the export boundary.
    timeline.metronome=Some(metronome(&project,&tempo,RATE,2880).unwrap());
    export_click(&mut timeline,&project,&tempo,RATE,2880).unwrap();
    assert!(timeline.metronome.is_none());
    let(mut control,mut rt)=channel(RATE,256);
    control.set_timeline(timeline.clone());control.play();
    let mut output=vec![0.;256*2];rt.process(&mut output,2);
    assert_eq!(output,source.iter().flat_map(|sample|[*sample,*sample]).collect::<Vec<_>>());
    let settings=PlaybackSettings{metronome:true,export_click:true,..Default::default()};
    project.extensions.insert("transport".into(),serde_json::to_value(settings).unwrap());
    export_click(&mut timeline,&project,&tempo,RATE,2880).unwrap();
    assert!(timeline.metronome.is_some());
    let(mut control,mut rt)=channel(RATE,256);
    control.set_timeline(timeline);control.play();
    let mut clicked=vec![0.;256*2];rt.process(&mut clicked,2);
    assert_ne!(clicked,output);
}
