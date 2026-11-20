use std::collections::BTreeMap;
use std::io::{Cursor,Read,Write};
use serde_json::{Value,json};
use verdant_domain::{Note,Project,Track,PointId};
use verdant_domain::model::{PitchPoint,TempoPoint};
use verdant_exchange::{self as exchange,Disposition,ExportOptions,Format,ImportOptions,UstEncoding,VocaloidVoice};
use zip::{ZipArchive,ZipWriter,write::SimpleFileOptions};
fn project()->Project{let mut p=Project::new_empty();p.voice_refs.clear();p.tracks[0].voice_ref=None;let mut n=Note::new(240,480,60);n.lyric="a".into();n.portamento_ms=Some(0);p.tracks[0].notes=vec![n];p.tracks[0].refresh_index();p}
fn smf(ppq:u16,events:&[u8])->Vec<u8>{let mut b=b"MThd\0\0\0\x06\0\0\0\x01".to_vec();b.extend_from_slice(&ppq.to_be_bytes());b.extend_from_slice(b"MTrk");b.extend_from_slice(&(events.len()as u32).to_be_bytes());b.extend_from_slice(events);b}
fn archive(sequence:&Value,extra:&[(&str,&[u8])])->Vec<u8>{let mut w=ZipWriter::new(Cursor::new(Vec::new()));w.start_file("Project/sequence.json",SimpleFileOptions::default()).unwrap();w.write_all(&serde_json::to_vec(sequence).unwrap()).unwrap();for(name,bytes)in extra{w.start_file(*name,SimpleFileOptions::default()).unwrap();w.write_all(bytes).unwrap();}w.finish().unwrap().into_inner()}
fn sequence(bytes:&[u8])->Value{let mut z=ZipArchive::new(Cursor::new(bytes)).unwrap();let mut b=Vec::new();z.by_name("Project/sequence.json").unwrap().read_to_end(&mut b).unwrap();serde_json::from_slice(&b).unwrap()}
#[test]
fn smf_exact_common_meter_scale_and_track_roundtrip(){
    let events=[0,0xff,0x58,4,3,5,37,9,0,0xff,0x51,3,7,0xa1,0x20,1,0x90,60,70,0,0xff,5,1,b'a',1,0xff,6,1,b'M',2,0x80,60,9,0,0xff,0x2f,0];
    let source=smf(3,&events);let first=exchange::import(Format::Midi,&source,&ImportOptions::default()).unwrap();assert_eq!(first.report.source_ppq,3);assert_eq!(first.project.ppq,24);assert_eq!(first.project.tracks.len(),1);
    let n=&first.project.tracks[0].notes[0];assert_eq!((n.start_tick,n.duration_tick),(8,24));assert_eq!(n.lyric,"a");assert_eq!(n.unknown["midi_release_velocity"],9);assert!(first.report.issues.iter().any(|i|i.path=="/ppq"&&i.disposition==Disposition::Metadata));
    let out=exchange::export(Format::Midi,&first.project,&ExportOptions{midi_ppq:Some(3),..Default::default()}).unwrap();let smf=midly::Smf::parse(&out.bytes).unwrap();assert_eq!(smf.tracks.len(),1);
    assert!(smf.tracks[0].iter().any(|e|matches!(e.kind,midly::TrackEventKind::Meta(midly::MetaMessage::TimeSignature(3,5,37,9)))));
    assert!(smf.tracks[0].iter().any(|e|matches!(e.kind,midly::TrackEventKind::Meta(midly::MetaMessage::Marker(b"M")))));
    let again=exchange::import(Format::Midi,&out.bytes,&ImportOptions::default()).unwrap();assert_eq!(again.project.ppq,24);assert_eq!(again.project.tracks[0].notes[0].start_tick,8);assert_eq!(again.project.tracks.len(),1);
}
#[test]
fn smf_rejects_ambiguous_pairing_duplicate_maps_and_truncated_chunks(){
    let overlap=smf(480,&[0,0x90,60,64,1,0x90,60,64,1,0x80,60,0,0,0xff,0x2f,0]);assert!(exchange::import(Format::Midi,&overlap,&ImportOptions::default()).is_err());
    let duplicate=smf(480,&[0,0xff,0x51,3,7,0xa1,0x20,0,0xff,0x51,3,7,0xa1,0x20,0,0xff,0x2f,0]);assert!(exchange::import(Format::Midi,&duplicate,&ImportOptions::default()).is_err());
    let mut truncated=smf(480,&[0,0xff,0x2f,0]);truncated.pop();assert!(exchange::import(Format::Midi,&truncated,&ImportOptions::default()).is_err());
    let mut p=project();let mut second=Note::new(480,480,64);second.detune_millicents=50000;p.tracks[0].notes.push(second);assert!(exchange::export(Format::Midi,&p,&ExportOptions::default()).unwrap_err().to_string().contains("channel-wide"));
}
#[test]
fn svp_blicks_edits_and_shared_library_identity_are_real_fields(){
    const B:i64=705600000;let id="b058d231-929d-4c6b-a77a-8cc836711b41";
    let group=json!({"uuid":id,"name":"shared","notes":[{"onset":1,"duration":B-1,"pitch":60,"lyrics":"原词","phonemes":"a","vendorNote":7}],"parameters":{"pitchDelta":{"mode":"linear","points":[0,0,B,100]}},"vendorGroup":{"x":true}});
    let source=json!({"version":100,"time":{"tempo":[{"position":0,"bpm":120,"label":"kept"}],"meter":[{"index":0,"numerator":4,"denominator":4}]},"tracks":[{"name":"lead","mainGroup":{"uuid":"79078b24-9cf8-4683-bddd-eddd40da737e","name":"main","notes":[],"parameters":{}},"mainRef":{"blickOffset":0,"pitchOffset":0},"groups":[{"groupID":id,"blickOffset":0,"pitchOffset":0,"vendorRef":1},{"groupID":id,"blickOffset":2*B,"pitchOffset":0,"vendorRef":2}]}],"library":[group],"vendorRoot":"kept"});
    let mut result=exchange::import(Format::Svp,&serde_json::to_vec(&source).unwrap(),&ImportOptions::default()).unwrap();assert_eq!(result.project.ppq,B);assert_eq!(result.project.tracks[0].notes[0].start_tick,1);assert_eq!(result.project.tracks[0].notes[1].start_tick,2*B+1);
    let parts=verdant_domain::parts::Parts::load(&result.project).unwrap();let refs:Vec<_>=parts.instances.iter().filter(|p|p.name=="shared").collect();assert_eq!(refs.len(),2);assert_eq!(refs[0].content_id,refs[1].content_id);
    let out=exchange::export(Format::Svp,&result.project,&ExportOptions::default()).unwrap();let value:Value=serde_json::from_slice(out.bytes.strip_suffix(&[0]).unwrap()).unwrap();assert_eq!(value["library"].as_array().unwrap().len(),1);assert_eq!(value["library"][0]["uuid"],id);assert_eq!(value["tracks"][0]["groups"][0]["groupID"],id);assert_eq!(value["tracks"][0]["groups"][1]["groupID"],id);assert_eq!(value["vendorRoot"],"kept");assert_eq!(value["time"]["tempo"][0]["label"],"kept");assert_eq!(value["library"][0]["vendorGroup"]["x"],true);
    result.project.tracks[0].notes[0].lyric="edited".into();let out=exchange::export(Format::Svp,&result.project,&ExportOptions::default()).unwrap();let again=exchange::import(Format::Svp,&out.bytes,&ImportOptions::default()).unwrap();assert_eq!(again.project.tracks[0].notes[0].lyric,"edited");assert_eq!(again.project.tracks[0].notes[0].start_tick,1);
}
#[test]
fn vsqx3_and_vsqx4_preserve_namespace_voice_controller_tail_and_editable_score(){
    for bytes in [include_bytes!("fixtures/score.vsqx").as_slice(),include_bytes!("fixtures/score-v3.vsqx").as_slice()]{let mut first=exchange::import(Format::Vsqx,bytes,&ImportOptions::default()).unwrap();assert_eq!(first.project.ppq,960);assert_eq!(first.project.tracks[0].notes[0].start_tick,3000);assert_eq!(first.project.tracks[0].notes[0].duration_tick,480);assert_eq!(first.project.tracks[0].notes[0].pronunciation.phonemes,["a","I"]);assert_eq!(first.project.time_signatures[1].tick,5760);
        first.project.tracks[0].notes[0].lyric="new <&\"詞>".into();let out=exchange::export(Format::Vsqx,&first.project,&ExportOptions::default()).unwrap();let text=std::str::from_utf8(&out.bytes).unwrap();let doc=roxmltree::Document::parse(text).unwrap();assert_eq!(doc.root_element().tag_name().namespace(),if text.contains("<vsq4"){Some("http://www.yamaha.co.jp/vocaloid/schema/vsq4/")}else{Some("http://www.yamaha.co.jp/vocaloid/schema/vsq3/")});assert!(doc.descendants().any(|n|n.is_element()&&n.tag_name().namespace()==Some("urn:verdant:test")&&n.tag_name().name()=="note"));assert!(text.contains("BNGW7FG7E5TRSNC3"));assert!(text.contains(">900<"));
        let again=exchange::import(Format::Vsqx,&out.bytes,&ImportOptions::default()).unwrap();assert_eq!(again.project.tracks[0].notes[0].lyric,"new <&\"詞>");assert_eq!(again.project.tracks[0].unknown["vsqx_track_number"],5);assert_eq!(again.project.tracks[0].notes[0].unknown["midi_velocity"],91);assert_eq!(again.project.time_signatures,first.project.time_signatures);
    }
}
#[test]
fn vsqx_requires_actual_voice_metadata_and_rejects_dtd_namespace_and_duplicate_fields(){
    let p=project();let err=exchange::export(Format::Vsqx,&p,&ExportOptions::default()).unwrap_err();assert!(err.to_string().contains("actual vocaloid_voice"));
    // Only public fixture identifiers are used; these are never runtime defaults.
    // 3c1u/vsqx @337d7da358b65f4cc0d85a3b9b88c29cc3de6f4b src/test/v4.vsqx.
    let voice=VocaloidVoice{comp_id:"BNGW7FG7E5TRSNC3".into(),name:"KAITO_V3_English".into(),language_id:1,program:0};let out=exchange::export(Format::Vsqx,&p,&ExportOptions{vocaloid_voice:Some(voice),..Default::default()}).unwrap();let imported=exchange::import(Format::Vsqx,&out.bytes,&ImportOptions::default()).unwrap();assert_eq!(imported.project.tracks[0].notes[0].midi_note,60);
    let source=std::str::from_utf8(include_bytes!("fixtures/score.vsqx")).unwrap();let dtd=source.replace("<vsq4 ","<!DOCTYPE vsq4 [<!ENTITY x 'bad'>]><vsq4 ");assert!(exchange::import(Format::Vsqx,dtd.as_bytes(),&ImportOptions::default()).is_err());let wrong=source.replace("http://www.yamaha.co.jp/vocaloid/schema/vsq4/","urn:wrong");assert!(exchange::import(Format::Vsqx,wrong.as_bytes(),&ImportOptions::default()).is_err());let duplicate=source.replace("<resolution>960</resolution>","<resolution>960</resolution><resolution>480</resolution>");assert!(exchange::import(Format::Vsqx,duplicate.as_bytes(),&ImportOptions::default()).is_err());
}
#[test]
fn vpr_real_archive_roundtrip_keeps_480_ticks_unknown_fields_and_members(){
    let source=json!({"version":{"major":5,"minor":0,"revision":0},"masterTrack":{"tempo":{"global":{"isEnabled":false,"value":12000},"events":[{"pos":0,"value":12345}]},"timeSig":{"events":[{"bar":0,"numer":3,"denom":4}]}},"tracks":[{"type":0,"name":"score","parts":[{"pos":17,"duration":960,"name":"part","voice":{"compID":"BNGW7FG7E5TRSNC3","langID":1},"notes":[{"pos":1,"duration":479,"number":62,"velocity":0.5,"lyric":"original","phoneme":"a I","customNote":{"retained":true}}],"controllers":[{"name":"pitchBend","events":[{"pos":0,"value":0},{"pos":900,"value":1000}]}],"customPart":8}]}],"customRoot":9});
    let bytes=archive(&source,&[("Project/vendor.txt",b"retained\0bytes")]);let mut first=exchange::import(Format::Vpr,&bytes,&ImportOptions::default()).unwrap();assert_eq!(first.project.ppq,480);assert_eq!(first.project.tracks[0].notes[0].start_tick,18);first.project.tracks[0].notes[0].lyric="edited".into();let out=exchange::export(Format::Vpr,&first.project,&ExportOptions::default()).unwrap();let data=sequence(&out.bytes);assert_eq!(data["tracks"][0]["parts"][0]["notes"][0]["lyric"],"edited");assert_eq!(data["tracks"][0]["parts"][0]["notes"][0]["velocity"],0.5);assert_eq!(data["tracks"][0]["parts"][0]["notes"][0]["customNote"]["retained"],true);assert_eq!(data["tracks"][0]["parts"][0]["controllers"][0]["events"][1]["pos"],900);assert_eq!(data["customRoot"],9);
    let mut z=ZipArchive::new(Cursor::new(&out.bytes)).unwrap();let mut kept=Vec::new();z.by_name("Project/vendor.txt").unwrap().read_to_end(&mut kept).unwrap();assert_eq!(kept,b"retained\0bytes");let again=exchange::import(Format::Vpr,&out.bytes,&ImportOptions::default()).unwrap();assert_eq!(again.project.ppq,480);assert_eq!(again.project.tracks[0].notes[0].start_tick,18);
}

fn gain_nodes(note: &Note) -> Vec<(i64, i64)> {
    note.curves.gain_millidecibel_points.iter().map(|p| (p.tick, p.millidecibels)).collect()
}

#[test]
fn vsqx_dynamics_projects_held_gain_and_retains_accent() {
    let sources = [
        (include_str!("fixtures/score-v3.vsqx"), "mCtrl", "posTick", "attr", "DYN"),
        (include_str!("fixtures/score.vsqx"), "cc", "t", "v", "D"),
    ];
    for (source, event, tick, value, id) in sources {
        let nodes = [(120, 0), (240, 64), (480, 127)].map(|(at, raw)|
            format!("<{event}><{tick}>{at}</{tick}><{value} id=\"{id}\">{raw}</{value}></{event}>"));
        let source = source.replacen("<note>", &format!("{}<note>", nodes.concat()), 1);
        let imported = exchange::import(Format::Vsqx, source.as_bytes(), &ImportOptions::default()).unwrap();
        let note = &imported.project.tracks[0].notes[0];
        assert_eq!(gain_nodes(note), [(0, -60_000), (120, 0), (360, 12_000), (480, 12_000)]);
        assert_eq!(note.curves.interpolation("DYN"), verdant_domain::editing::Interpolation::Hold);
        assert_eq!(note.unknown["midi_velocity"], 91);
        assert!(imported.report.issues.iter().any(|issue| issue.disposition == Disposition::RetainedUnsupported
            && (issue.path.ends_with("/nStyle") || issue.path.ends_with("/noteStyle"))));
        let exported = exchange::export(Format::Vsqx, &imported.project, &ExportOptions::default()).unwrap();
        let text = std::str::from_utf8(&exported.bytes).unwrap();
        let parsed = roxmltree::Document::parse(text).unwrap();
        let exported_nodes = parsed.descendants().filter(|node| node.attribute("id") == Some(id))
            .map(|node| (node.parent().unwrap().children().find(|child| child.has_tag_name((node.tag_name().namespace().unwrap(), tick))).unwrap().text().unwrap(), node.text().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(exported_nodes, [("120", "0"), ("240", "64"), ("480", "127")]);
        let again = exchange::import(Format::Vsqx, &exported.bytes, &ImportOptions::default()).unwrap();
        assert_eq!(gain_nodes(&again.project.tracks[0].notes[0]), gain_nodes(note));
    }
}

fn vpr_controller_score() -> Value {
    json!({
        "masterTrack":{"tempo":{"events":[{"pos":0,"value":12000}]},"timeSig":{"events":[{"bar":0,"numer":4,"denom":4}]}},
        "tracks":[{"type":0,"parts":[{"pos":7,"duration":480,"notes":[{"pos":0,"duration":360,"number":60,"lyric":"a","velocity":92,"exp":{"accent":42}}],
            "controllers":[
                {"name":"pitchBend","events":[{"pos":0,"value":1},{"pos":240,"value":4096}]},
                {"name":"pitchBendSens","events":[{"pos":0,"value":2}]},
                {"name":"dyn","events":[{"pos":0,"value":0},{"pos":120,"value":65},{"pos":240,"value":127}]},
                {"name":"breathiness","events":[{"pos":0,"value":18},{"pos":240,"value":48},{"pos":450,"value":21}]},
                {"name":"acc","events":[{"pos":0,"value":60}]},
                {"name":"vel","events":[{"pos":0,"value":80}]}
            ]}]}]
    })
}

#[test]
fn vpr_preserves_unedited_controller_nodes_and_reports_projection_quantization() {
    let source = vpr_controller_score();
    let opts = ImportOptions::default();
    let imported = exchange::import(Format::Vpr, &archive(&source, &[]), &opts).unwrap();
    let note = &imported.project.tracks[0].notes[0];
    assert_eq!(gain_nodes(note), [(0, -60_000), (120, 190), (240, 12_000), (360, 12_000)]);
    let pitch_nodes = |note: &Note| note.curves.pitch_millicent_points.iter().map(|p| (p.tick, p.millicents)).collect::<Vec<_>>();
    let breath_nodes = |note: &Note| note.curves.expression("BRE").iter().map(|p| (p.tick, p.value)).collect::<Vec<_>>();
    assert_eq!(pitch_nodes(note), [(0, 24), (240, 100_000), (360, 100_000)]);
    assert_eq!(breath_nodes(note), [(0, 18), (240, 48), (360, 48)]);
    assert_eq!(note.unknown["midi_velocity"], 92);
    let quantized = |report: &exchange::ExchangeReport| report.issues.iter().filter(|issue| issue.disposition == Disposition::Quantized)
        .map(|issue| issue.path.clone()).collect::<Vec<_>>();
    let expected_quantization = ["/tracks/0/parts/0/pitch/0", "/tracks/0/parts/0/controllers/dyn/120"];
    assert_eq!(quantized(&imported.report), expected_quantization);
    for field in ["/controllers/acc", "/controllers/vel", "/notes/0/exp"] {
        assert!(imported.report.issues.iter().any(|issue| issue.disposition == Disposition::RetainedUnsupported && issue.path.ends_with(field)));
    }
    let exported = exchange::export(Format::Vpr, &imported.project, &ExportOptions::default()).unwrap();
    let output = sequence(&exported.bytes);
    assert_eq!(output["tracks"][0]["parts"][0]["controllers"], source["tracks"][0]["parts"][0]["controllers"]);
    assert_eq!(output["tracks"][0]["parts"][0]["notes"][0]["exp"], json!({"accent":42}));
    assert!(quantized(&exported.report).is_empty());
    let again = exchange::import(Format::Vpr, &exported.bytes, &opts).unwrap();
    let reimported = &again.project.tracks[0].notes[0];
    assert_eq!(gain_nodes(reimported), gain_nodes(note));
    assert_eq!(pitch_nodes(reimported), pitch_nodes(note));
    assert_eq!(breath_nodes(reimported), breath_nodes(note));
    assert_eq!(quantized(&again.report), expected_quantization);
}

#[test]
fn vpr_edited_controllers_quantize_and_keep_unedited_source_lanes() {
    let source = vpr_controller_score();
    let mut imported = exchange::import(Format::Vpr, &archive(&source, &[]), &ImportOptions::default()).unwrap();
    let note = &mut imported.project.tracks[0].notes[0];
    note.curves.pitch_millicent_points[1].millicents = 75_001;
    note.curves.gain_millidecibel_points[1].millidecibels = -30_001;
    let out = exchange::export(Format::Vpr, &imported.project, &ExportOptions::default()).unwrap();
    let data = sequence(&out.bytes);
    let controllers = data["tracks"][0]["parts"][0]["controllers"].as_array().unwrap();
    let controller = |name: &str| controllers.iter().find(|lane| lane["name"] == name).unwrap();
    assert_eq!(controller("breathiness"), &source["tracks"][0]["parts"][0]["controllers"][3]);
    assert_eq!(controller("pitchBend")["events"][1], json!({"pos":240,"value":3072}));
    assert_eq!(controller("pitchBendSens")["events"], json!([{"pos":0,"value":2}]));
    assert_eq!(controller("dyn")["events"][1], json!({"pos":120,"value":32}));
    for path in ["/pitch/240", "/gain/120"] {
        assert!(out.report.issues.iter().any(|issue| issue.disposition == Disposition::Quantized && issue.path.ends_with(path)));
    }
    let again = exchange::import(Format::Vpr, &out.bytes, &ImportOptions::default()).unwrap();
    assert_eq!(gain_nodes(&again.project.tracks[0].notes[0])[1], (120, -30_000));
}

#[test]
fn vpr_dynamics_rejects_values_outside_its_integer_range() {
    for invalid in [json!(-1), json!(128), json!(64.5)] {
        let mut source = vpr_controller_score();
        source["tracks"][0]["parts"][0]["controllers"][2]["events"][0]["value"] = invalid;
        let error = exchange::import(Format::Vpr, &archive(&source, &[]), &ImportOptions::default()).unwrap_err();
        assert!(error.to_string().contains("/controllers/dyn"));
    }
}

#[test]
fn vpr_edited_expression_emits_all_seven_integer_lanes() {
    use verdant_domain::model::ExpressionPoint;

    let lanes = [("BRE", "breathiness"), ("BRI", "brightness"), ("CLE", "clearness"),
        ("GEN", "gender"), ("POR", "portamento"), ("OPE", "opening"), ("GWL", "growl")];
    let mut project = project();
    let note = &mut project.tracks[0].notes[0];
    for (i, (key, _)) in lanes.iter().enumerate() {
        note.curves.expression_points.insert((*key).into(), vec![
            ExpressionPoint { id: PointId::generate(), tick: 0, value: i as u8, unknown: BTreeMap::new() },
            ExpressionPoint { id: PointId::generate(), tick: 120, value: if i == 0 { 200 } else { 80 + i as u8 }, unknown: BTreeMap::new() },
        ]);
        note.curves.set_interpolation(key, verdant_domain::editing::Interpolation::Hold);
    }
    let out = exchange::export(Format::Vpr, &project, &ExportOptions::default()).unwrap();
    let data = sequence(&out.bytes);
    let controllers = data["tracks"][0]["parts"][0]["controllers"].as_array().unwrap();
    for (i, (_, name)) in lanes.into_iter().enumerate() {
        let lane = controllers.iter().find(|lane| lane["name"] == name).unwrap();
        let last = if i == 0 { 127 } else { 80 + i };
        assert_eq!(lane["events"], json!([{"pos":120,"value":i},{"pos":180,"value":last}]));
    }
    assert!(out.report.issues.iter().any(|issue| issue.disposition == Disposition::Quantized
        && issue.path.ends_with("/curves/BRE/180") && issue.value == 200));
}

#[test]
fn vpr_track_selection_preserves_source_only_tracks_order_metadata_and_members(){
    let vocal=|name:&str|json!({"type":0,"name":name,"vendorTrack":{"name":name},"parts":[{"pos":0,"duration":480,"name":"part","notes":[{"pos":0,"duration":480,"number":60,"lyric":name}],"controllers":[]}]});
    let source=json!({
        "masterTrack":{"tempo":{"global":{"isEnabled":true,"value":12000},"vendorTempo":7},"timeSig":{"events":[{"bar":0,"numer":4,"denom":4}]},"samplingRate":44100,"vendorMaster":8},
        "voices":[{"compID":"source-voice","vendorVoice":9}],"vendorRoot":{"kept":true},
        "tracks":[{"type":1,"name":"audio before","parts":[{"file":"never-open.wav","vendorAudio":1}]},vocal("lead"),{"type":99,"name":"unknown middle","payload":[1,2,3]},vocal("harmony"),{"type":1,"name":"audio after","parts":[]}]
    });
    let bytes=archive(&source,&[("Project/vendor.bin",b"retained\0member")]);
    let mut imported=exchange::import(Format::Vpr,&bytes,&ImportOptions::default()).unwrap();
    imported.project.tracks[1].notes[0].lyric="edited harmony".into();
    for(track,indices)in[(None,vec![0,1,2,3,4]),(Some(0),vec![0,1,2,4]),(Some(1),vec![0,2,3,4])]{
        let out=exchange::export(Format::Vpr,&imported.project,&ExportOptions{track,..Default::default()}).unwrap();
        let data=sequence(&out.bytes);let tracks=data["tracks"].as_array().unwrap();
        assert_eq!(tracks.len(),indices.len());
        for(output,index)in tracks.iter().zip(indices){
            assert_eq!(output["name"],source["tracks"][index]["name"]);
            if matches!(index,0|2|4){assert_eq!(output,&source["tracks"][index]);}
            else{
                assert_eq!(output["vendorTrack"],source["tracks"][index]["vendorTrack"]);
                assert_eq!(output["parts"][0]["notes"][0]["lyric"],if index==3{"edited harmony"}else{"lead"});
            }
        }
        for key in ["vendorRoot","voices","masterTrack"]{assert_eq!(data[key],source[key]);}
        let mut zip=ZipArchive::new(Cursor::new(&out.bytes)).unwrap();let mut member=Vec::new();
        zip.by_name("Project/vendor.bin").unwrap().read_to_end(&mut member).unwrap();assert_eq!(member,b"retained\0member");
        let again=exchange::import(Format::Vpr,&out.bytes,&ImportOptions::default()).unwrap();
        assert_eq!(again.project.tracks.len(),if track.is_some(){1}else{2});
    }
    let discarded=exchange::export(Format::Vpr,&imported.project,&ExportOptions{track:Some(1),preserve_extensions:false,..Default::default()}).unwrap();
    let data=sequence(&discarded.bytes);assert_eq!(data["tracks"].as_array().unwrap().len(),1);assert_eq!(data["tracks"][0]["name"],"harmony");assert!(data.get("vendorRoot").is_none());
    assert_eq!(ZipArchive::new(Cursor::new(&discarded.bytes)).unwrap().len(),1);
}
#[test]
fn archive_paths_sizes_and_duplicate_json_are_admitted_before_score_projection(){
    let source=json!({"anything":true});for name in ["../escape","Project/../escape","C:/escape","Project\\escape"]{let bytes=archive(&source,&[(name,b"x")]);assert!(exchange::import(Format::Vpr,&bytes,&ImportOptions::default()).unwrap_err().to_string().contains("archive"));}
    let bytes=archive(&source,&[("Project/data.bin",b"longer than zero")]);let mut options=ImportOptions::default();options.limits.max_archive_bytes=1;assert!(exchange::import(Format::Vpr,&bytes,&options).is_err());
    assert!(exchange::import(Format::Svp,b"{\"version\":100,\"version\":100}",&ImportOptions::default()).unwrap_err().to_string().contains("duplicate"));
    assert!(exchange::parse_export_options_json(b"{\"track\":0,\"track\":1}").unwrap_err().to_string().contains("duplicate"));
}
#[test]
fn ust_track_selection_encoding_tempo_gaps_and_mode2_are_consumed_by_real_importer(){
    let mut p=project();p.tracks[0].notes[0].lyric="か".into();p.tracks[0].notes[0].curves.pitch_millicent_points=vec![PitchPoint{id:PointId::generate(),tick:0,millicents:0,unknown:BTreeMap::new()},PitchPoint{id:PointId::generate(),tick:480,millicents:50000,unknown:BTreeMap::new()}];let mut other=Track::new("other","teal");other.voice_ref=None;other.notes=vec![Note::new(0,480,65)];p.tracks.push(other);
    assert!(exchange::export(Format::Ust,&p,&ExportOptions::default()).unwrap_err().to_string().contains("select a track"));let out=exchange::export(Format::Ust,&p,&ExportOptions{track:Some(0),ust_encoding:UstEncoding::Cp932,..Default::default()}).unwrap();let imported=verdant_import::import(verdant_import::ImportFormat::Ust,&out.bytes,&verdant_import::ImportOptions{ust_encoding:verdant_import::UstEncoding::Cp932,..Default::default()}).unwrap();let n=&imported.project.tracks[0].notes[0];assert_eq!((n.start_tick,n.duration_tick,n.midi_note),(240,480,60));assert_eq!(n.lyric,"か");assert!(!n.curves.pitch_millicent_points.is_empty());
    p.tracks[0].notes[0].lyric="😀".into();assert!(exchange::export(Format::Ust,&p,&ExportOptions{track:Some(0),ust_encoding:UstEncoding::Cp932,..Default::default()}).is_err());
    p.tempo_map.push(TempoPoint{tick:480,micro_bpm:90000000,unknown:BTreeMap::new()});assert!(exchange::export(Format::Ust,&p,&ExportOptions{track:Some(0),..Default::default()}).unwrap_err().to_string().contains("inside a sounding note"));
}
#[test]
fn ustx_preserves_multiple_tracks_maps_parts_and_authored_pitch_in_real_importer(){
    let mut p=project();p.tracks[0].notes[0].detune_millicents=25000;p.tempo_map.push(TempoPoint{tick:480,micro_bpm:90000000,unknown:BTreeMap::new()});let mut second=Track::new("second","teal");second.voice_ref=None;let mut n=Note::new(960,960,67);n.lyric="other".into();second.notes.push(n);p.tracks.push(second);
    let out=exchange::export(Format::Ustx,&p,&ExportOptions::default()).unwrap();let value:Value=serde_yaml::from_slice(&out.bytes).unwrap();assert_eq!(value["voice_parts"].as_array().unwrap().len(),2);assert_eq!(value["tracks"].as_array().unwrap().len(),2);let imported=verdant_import::import(verdant_import::ImportFormat::Ustx,&out.bytes,&verdant_import::ImportOptions::default()).unwrap();assert_eq!(imported.project.tempo_map,p.tempo_map);assert_eq!(imported.project.tracks[0].notes[0].start_tick,240);assert_eq!(imported.project.tracks[1].notes[0].midi_note,67);assert!(imported.project.tracks[0].notes[0].curves.pitch_millicent_points.iter().all(|p|p.millicents==25000));
}
