use serde_json::json;
use verdant_domain::command::Edge;
use verdant_domain::model::{ExpressionPoint, expression_default};
use verdant_domain::{Command, CommandBatch, Note, OutputTransientsSettings, Project,
    ProjectPersistence, ProjectStore, VocalTractSettings};

const KEYS: [&str;7] = ["BRE","BRI","CLE","GEN","OPE","GWL","POR"];

fn initial() -> Project {
    let mut p=Project::new_empty();
    let mut n=Note::new(960,960,60);n.id="voice-note".into();
    p.tracks[0].notes.push(n);p.tracks[0].refresh_index();
    p.render_preferences.vocal_tract=Some(VocalTractSettings::bilinear_v1(1.125));
    p
}

fn point(key:&str,tick:i64,value:u8) -> ExpressionPoint {
    ExpressionPoint { id:format!("{key}-{tick}").as_str().into(),tick,value,unknown:Default::default() }
}

fn curve(p:&Project,key:&str,points:Vec<ExpressionPoint>) -> Command {
    Command::SetExpressionCurve {track:p.tracks[0].id.clone(),note_id:p.tracks[0].notes[0].id.clone(),
        parameter:key.into(),points}
}

#[test]
fn all_seven_controls_and_final_transients_survive_journal_undo_redo_save_with_unknown_metadata() {
    let root=std::env::temp_dir().join(format!("verdant-expression-{}",uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();let path=root.join("song.verp");
    let p=initial();let before=p.clone();
    let mut commands=Vec::new();
    for key in KEYS {
        let mut a=point(key,0,expression_default(key).unwrap());
        a.unknown.insert("future_shape".into(),json!({"tension":0.4}));
        commands.push(curve(&p,key,vec![a,point(key,960,127)]));
    }
    let mut transients=OutputTransientsSettings::phoneme_output_envelope_v1();
    transients.unknown.insert("future_annotation".into(),json!("retain"));
    commands.push(Command::SetOutputTransients{settings:Some(transients)});
    let mut document=ProjectPersistence::create(&path,p).unwrap();
    document.commit(document.revision(),CommandBatch{label:"表达与最终辅音".into(),commands}).unwrap().unwrap();
    let edited=document.project().clone();
    assert_eq!(document.undo_depth(),1);
    assert_eq!(edited.render_preferences.vocal_tract,before.render_preferences.vocal_tract);
    assert_eq!(edited.tracks[0].notes[0].start_tick,960);
    assert_eq!(edited.tracks[0].notes[0].duration_tick,960);
    drop(document);
    let (mut document,recovery)=ProjectPersistence::open(&path).unwrap();
    assert_eq!(recovery.replayed_commits,1);
    assert_eq!(document.project(),&edited);
    document.undo().unwrap();
    assert_eq!(document.project().tracks,before.tracks);
    assert_eq!(document.project().render_preferences,before.render_preferences);
    document.redo().unwrap();document.save().unwrap();drop(document);
    let (document,recovery)=ProjectPersistence::open(&path).unwrap();
    assert_eq!(recovery.replayed_commits,0);
    assert_eq!(document.project().tracks,edited.tracks);
    assert_eq!(document.project().render_preferences,edited.render_preferences);
    let (restored,report)=verdant_domain::io::parse_project(&serde_json::to_string(document.project()).unwrap()).unwrap();
    assert_eq!(restored,document.project().clone());
    assert!(report.preserved_unknown_keys.iter().any(|p|p.ends_with("/output_transients/future_annotation")));
    for key in KEYS {assert!(report.preserved_unknown_keys.iter().any(|p|p.ends_with(&format!("/expression_points/{key}/0/future_shape"))));}
    drop(document);std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_expression_or_output_settings_roll_back_the_whole_user_gesture() {
    for case in 0..7 {
        let p=initial();let before=p.clone();
        let mut store=ProjectStore::new(p.clone());let rev=store.revision();
        let invalid=match case {
            0=>curve(&p,"UNKNOWN",vec![point("bad",0,64)]),
            1=>curve(&p,"BRI",vec![point("bad",0,128)]),
            2=>curve(&p,"BRI",vec![point("bad",-1,64)]),
            3=>curve(&p,"BRI",vec![point("bad",961,64)]),
            _=>{
                let mut setting=OutputTransientsSettings::phoneme_output_envelope_v1();
                match case {4=>setting.strength=f64::NAN,5=>setting.max_attenuation_db=12.01,_=>setting.method="unknown".into()}
                Command::SetOutputTransients{settings:Some(setting)}
            }
        };
        let batch=CommandBatch{label:"atomic".into(),commands:vec![curve(&p,"GWL",vec![point("good",0,80)]),invalid]};
        assert!(store.apply(rev,batch).is_err(),"case{case}");
        assert_eq!(store.project(),&before);assert_eq!(store.revision(),rev);assert_eq!(store.undo_depth(),0);
    }
}

#[test]
fn move_resize_and_retime_keep_all_controller_points_attached_and_collisions_keep_last_edit() {
    let mut p=initial();
    for key in KEYS {p.tracks[0].notes[0].curves.expression_points.insert(key.into(),
        vec![point(key,0,10),point(key,1,20),point(key,480,30),point(key,960,40)]);}
    let mut store=ProjectStore::new(p.clone());let track=p.tracks[0].id.clone();let id=p.tracks[0].notes[0].id.clone();
    store.apply(store.revision(),CommandBatch::single("移动",Command::MoveNotes{track:track.clone(),note_ids:vec![id.clone()],delta_tick:240,delta_semitones:0})).unwrap();
    assert_eq!(store.project().tracks[0].notes[0].curves,p.tracks[0].notes[0].curves);
    store.apply(store.revision(),CommandBatch::single("缩短",Command::ResizeNotes{track:track.clone(),note_ids:vec![id.clone()],edge:Edge::End,delta_tick:-480})).unwrap();
    for key in KEYS {let points=store.project().tracks[0].notes[0].curves.expression(key);
        assert_eq!(points.iter().map(|p|(p.tick,p.value)).collect::<Vec<_>>(),vec![(0,20),(240,30),(480,40)]);
        assert_eq!(points[0].id.as_str(),format!("{key}-1").as_str());}
    let resized=store.project().tracks[0].notes[0].clone();
    store.apply(store.revision(),CommandBatch::single("重新定时",Command::SetTiming{track,note_id:id,start_tick:1920,duration_tick:240})).unwrap();
    for key in KEYS {assert_eq!(store.project().tracks[0].notes[0].curves.expression(key).iter().map(|p|p.tick).collect::<Vec<_>>(),vec![0,120,240]);}
    store.undo().unwrap();assert_eq!(store.project().tracks[0].notes[0],resized);
    store.redo().unwrap();assert_eq!(store.project().tracks[0].notes[0].duration_tick,240);
}

#[test]
fn old_documents_omit_new_fields_and_cancelled_curve_gestures_leave_no_new_state() {
    let p=initial();let value=serde_json::to_value(&p).unwrap();
    assert!(value["render_preferences"].get("output_transients").is_none());
    assert!(value["tracks"][0]["notes"][0]["curves"].get("expression_points").is_none());
    let (roundtrip,_)=verdant_domain::io::parse_project(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(p,roundtrip);
    let mut store=ProjectStore::new(p.clone());let rev=store.revision();
    let batch=CommandBatch{label:"取消".into(),commands:vec![curve(&p,"GWL",vec![point("temporary",0,80)]),curve(&p,"GWL",vec![])]};
    assert!(store.apply(rev,batch).unwrap().is_none());assert_eq!(store.project(),&p);assert_eq!(store.undo_depth(),0);
    let mut zero=OutputTransientsSettings::phoneme_output_envelope_v1();zero.strength=0.;
    store.apply(rev,CommandBatch::single("显式旁路",Command::SetOutputTransients{settings:Some(zero)})).unwrap();
    assert_eq!(serde_json::to_value(store.project()).unwrap()["render_preferences"]["output_transients"]["strength"],0.);
}
