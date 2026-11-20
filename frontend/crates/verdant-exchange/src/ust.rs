//! Real UTAU/OpenUtau score exports; import belongs to verdant-import.
//! Source sample controls and licensed synthesizer execution are unsupported.
use std::collections::{BTreeMap, BTreeSet};
use serde_json::{Map,Value,json};
use verdant_domain::{Note,Project,TempoMap};
use crate::{Disposition as D,ExchangeError as E,ExchangeReport,ExportOptions,Result,UstEncoding};
use crate::common::*;
fn field(out:&mut String,key:&str,value:impl ToString)->Result<()> {
    if key.is_empty()||key.contains(['\r','\n','\0','=','[',']']){return Err(E::at("/ust","invalid field name"));}
    let value=value.to_string();if value.contains(['\r','\n','\0']){return Err(E::at(key,"UST scalar contains a newline or NUL"));}out.push_str(key);out.push('=');out.push_str(&value);out.push_str("\r\n");Ok(())
}
fn ledger<'a>(p:&'a Project,format:&str,preserve:bool)->Option<&'a [Value]> {
    if !preserve{return None;}let source=p.extensions.get("score_import")?;
    if source.get("source")?.get("format")?.as_str()?!=format{return None;}source.get("fields")?.as_array().map(Vec::as_slice)
}
fn restore(fields:Option<&[Value]>,prefix:&str,known:&[&str])->Map<String,Value>{
    let mut out=Map::new();if let Some(fields)=fields{for f in fields{let Some(location)=f.get("location").and_then(Value::as_str)else{continue;};let Some(key)=location.strip_prefix(prefix)else{continue;};
        if key.is_empty()||key.contains('/')||known.contains(&key){continue;}if let Some(value)=f.get("source_value"){out.insert(key.into(),value.clone());}
    }}out
}
fn note_extras(p:&Project,n:&Note,format:&str,o:&ExportOptions,known:&[&str])->Map<String,Value>{
    let id=n.unknown.get("source_note_ids").and_then(Value::as_array).and_then(|a|a.first()).and_then(Value::as_str);
    id.map(|id|restore(ledger(p,format,o.preserve_extensions),&format!("{id}/"),known)).unwrap_or_default()
}
fn pitch_points(n:&Note,tempo:&TempoMap,report:&mut ExchangeReport,path:&str)->Result<Vec<(f64,f64)>>{
    let mut points=BTreeMap::new();let detune=i64::from(n.detune_millicents);
    if n.curves.pitch_millicent_points.is_empty(){points.insert(0,detune);points.insert(n.duration_tick,detune);}else{
        for p in &n.curves.pitch_millicent_points{points.insert(p.tick,p.millicents.checked_add(detune).ok_or_else(||E::at(path,"pitch overflow"))?);}
    }
    if n.curves.interpolation("PIT")!=verdant_domain::editing::Interpolation::Linear && n.curves.pitch_millicent_points.len()>1{
        report.issue(format!("{path}/pitch/interpolation"),D::Unsupported,json!(n.curves.interpolation("PIT")),"UTAU pitch export keeps authored nodes with linear interpolation; hold/step shape cannot be represented exactly by these nodes.");
    }
    let start=tempo.seconds_at(n.start_tick as f64);let mut result=Vec::new();
    for(tick,value)in points{let ms=(tempo.seconds_at((n.start_tick+tick)as f64)-start)*1000.0;let y=value as f64/10000.0;
        if !ms.is_finite()||!y.is_finite(){return Err(E::at(path,"pitch time/value overflow"));}result.push((ms,y));}Ok(result)
}
fn unsupported_note(n:&Note,report:&mut ExchangeReport,path:&str){
    if !n.pronunciation.phonemes.is_empty(){report.issue(format!("{path}/pronunciation/phonemes"),D::Unsupported,json!(n.pronunciation.phonemes),"Portable phonetic symbols do not identify UTAU voicebank sample aliases; only original source alias fields can roundtrip.");}
    if n.unknown.contains_key("midi_velocity"){report.issue(format!("{path}/midi_velocity"),D::Unsupported,json!(n.unknown["midi_velocity"]),"UTAU Velocity controls consonant stretching, not MIDI note intensity; no numerical relabeling is performed.");}
    for(key,points)in &n.curves.expression_points{if !points.is_empty(){report.issue(format!("{path}/curves/{key}"),D::Unsupported,json!(points),"Source UTAU sample controls do not have portable VOCALOID timbre semantics.");}}
}
fn report_parts(p:&Project,track:&verdant_domain::Track,report:&mut ExchangeReport,path:&str)->Result<()> {
    let document=verdant_domain::parts::Parts::load(p).map_err(|e|E::at(path,e))?;
    for part in document.instances.iter().filter(|part|part.track==track.id){report.issue(format!("{path}/parts/{}",part.id),D::Unsupported,json!({"id":part.id,"content_id":part.content_id}),"Destination stores independent score events; linked content/reference identity has no native equivalent.");}Ok(())
}
pub(crate) fn export_ust(p:&Project,o:&ExportOptions,report:&mut ExchangeReport)->Result<Vec<u8>> {
    if p.tracks.len()>1 && o.track.is_none(){return Err(E::at("/options/track","UST has one vocal track; select a track explicitly for a multitrack project"));}
    let(ti,track)=selected(p,o).next().ok_or_else(||E::at("/tracks","UST requires one selected track"))?;
    let tempo=TempoMap::new(&p.tempo_map,p.ppq).map_err(|e|E::at("/tempo",e.to_string()))?;
    let mut notes:Vec<_>=track.notes.iter().collect();notes.sort_by_key(|n|(n.start_tick,&n.id));
    if notes.windows(2).any(|w|w[0].end_tick()>w[1].start_tick){return Err(E::at("/notes","UST export requires a monophonic selected track without overlaps"));}
    let mut tempo_ticks=BTreeMap::new();for t in &p.tempo_map{tempo_ticks.insert(scale(t.tick,p.ppq,480,"/tempo")?,t.micro_bpm);}
    for n in &notes{if p.tempo_map.iter().any(|t|t.tick>n.start_tick&&t.tick<n.end_tick()){return Err(E::at("/tempo","UST cannot place a tempo change inside a sounding note without splitting that note; edit the note boundary or export USTX"));}}
    let mut out=String::from("[#VERSION]\r\nUST Version1.2\r\n[#SETTING]\r\n");
    field(&mut out,"Tempo",p.tempo_map[0].micro_bpm as f64/1e6)?;field(&mut out,"Tracks",1)?;field(&mut out,"ProjectName",&track.name)?;field(&mut out,"Mode2","True")?;
    field(&mut out,"TimeSignatures",meters_to_bars(p)?.into_iter().map(|(bar,nu,de)|format!("({nu}/{de}/{bar})")).collect::<Vec<_>>().join(","))?;
    for(k,v)in restore(ledger(p,"ust",o.preserve_extensions),"/#SETTING/",&["Tempo","Tracks","ProjectName","Mode2","Resolution","TicksPerQuarter","TimeSignatures"]){if let Some(v)=v.as_str(){field(&mut out,&k,v)?;}}
    let mut cursor=0i64;let mut index=0usize;
    let write_rest=|from:i64,to:i64,out:&mut String,index:&mut usize|->Result<()> {
        let mut points=vec![from];points.extend(tempo_ticks.range((std::ops::Bound::Excluded(from),std::ops::Bound::Excluded(to))).map(|(t,_)|*t));points.push(to);
        for w in points.windows(2){if w[0]==w[1]{continue;}if *index>=o.limits.max_events{return Err(E::at("/ust","event limit exceeded"));}out.push_str(&format!("[#{:04}]\r\n",*index));*index+=1;field(out,"Length",w[1]-w[0])?;field(out,"Lyric","R")?;field(out,"NoteNum",60)?;if let Some(bpm)=tempo_ticks.get(&w[0]){field(out,"Tempo",*bpm as f64/1e6)?;}}Ok(())
    };
    for(ni,n)in notes.iter().enumerate(){let path=format!("/tracks/{ti}/notes/{ni}");let at=scale(n.start_tick,p.ppq,480,&path)?;let end_tick=scale(n.end_tick(),p.ppq,480,&path)?;
        if at>cursor{write_rest(cursor,at,&mut out,&mut index)?;}if index>=o.limits.max_events{return Err(E::at("/ust","event limit exceeded"));}out.push_str(&format!("[#{index:04}]\r\n"));index+=1;
        field(&mut out,"Length",end_tick-at)?;field(&mut out,"Lyric",&n.lyric)?;field(&mut out,"NoteNum",n.midi_note)?;if let Some(bpm)=tempo_ticks.get(&at){field(&mut out,"Tempo",*bpm as f64/1e6)?;}
        for(k,v)in note_extras(p,n,"ust",o,&["Length","Lyric","NoteNum","Tempo","Delta","Duration","Intensity","mode2_pitch","VBR"]){if let Some(v)=v.as_str(){field(&mut out,&k,v)?;}}
        if n.detune_millicents!=0||!n.curves.pitch_millicent_points.is_empty(){let points=pitch_points(n,&tempo,report,&path)?;
            if points.len()>=2{field(&mut out,"PBS",format!("{};{}",points[0].0,points[0].1))?;field(&mut out,"PBW",points.windows(2).map(|w|(w[1].0-w[0].0).to_string()).collect::<Vec<_>>().join(","))?;field(&mut out,"PBY",points.iter().skip(1).map(|v|v.1.to_string()).collect::<Vec<_>>().join(","))?;field(&mut out,"PBM",vec!["s";points.len()-1].join(","))?;}
        }
        if !n.curves.gain_millidecibel_points.is_empty(){let values:BTreeSet<_>=n.curves.gain_millidecibel_points.iter().map(|p|p.millidecibels).collect();if values.len()==1{let db=*values.first().expect("one gain")as f64/1000.0;let intensity=100.0*10f64.powf(db/20.0);if intensity<=200.0&&intensity.is_finite(){field(&mut out,"Intensity",intensity)?;}else{report.issue(format!("{path}/gain"),D::Unsupported,json!(n.curves.gain_millidecibel_points),"UST Intensity supports 0..200 percent only.");}}else{report.issue(format!("{path}/gain"),D::Unsupported,json!(n.curves.gain_millidecibel_points),"UST Intensity is a single per-note value; varying gain cannot be represented.");}}
        unsupported_note(n,report,&path);cursor=end_tick;
    }
    if let Some(last)=tempo_ticks.keys().next_back().copied(){if last>=cursor && last>0{write_rest(cursor,last.checked_add(1).ok_or_else(||E::at("/ust","tempo tail overflow"))?,&mut out,&mut index)?;}}
    if index==0{write_rest(0,480,&mut out,&mut index)?;}
    out.push_str("[#TRACKEND]\r\n");report_parts(p,track,report,&format!("/tracks/{ti}"))?;
    report.issue("/time_signatures",D::Metadata,serde_json::to_value(&p.time_signatures)?,"Meter bars export through the UTAU TimeSignatures setting; older consumers may ignore this extension.");
    if track.gain_millidecibels!=0||track.pan_permille!=0||track.muted||track.solo{report.issue(format!("/tracks/{ti}/mixer"),D::Unsupported,json!({"gain":track.gain_millidecibels,"pan":track.pan_permille,"muted":track.muted,"solo":track.solo}),"UST does not carry this track mixer state.");}
    report.issue("/synthesis",D::Metadata,"UTAU","Score-only export: UTAU voicebank sample controls and synthesizer execution are unsupported; no sample engine is emulated.");
    match o.ust_encoding{UstEncoding::Utf8=>Ok(out.into_bytes()),UstEncoding::Cp932=>{let(encoded,_,errors)=encoding_rs::SHIFT_JIS.encode(&out);if errors{return Err(E::at("/encoding","UST contains characters unrepresentable in CP932; select UTF-8"));}Ok(encoded.into_owned())}}
}
pub(crate) fn export_ustx(p:&Project,o:&ExportOptions,report:&mut ExchangeReport)->Result<Vec<u8>>{
    let fields=ledger(p,"ustx",o.preserve_extensions);let mut root=restore(fields,"/",&["ustx_version","resolution","bpm","beat_per_bar","beat_unit","tempos","time_signatures","tracks","voice_parts","expressions","name"]);
    root.insert("ustx_version".into(),"0.6".into());root.insert("name".into(),p.unknown.get("title").cloned().unwrap_or_else(||p.project_id.clone().into()));root.insert("resolution".into(),p.ppq.into());
    root.insert("bpm".into(),json!(p.tempo_map[0].micro_bpm as f64/1e6));root.insert("beat_per_bar".into(),p.time_signatures[0].numerator.into());root.insert("beat_unit".into(),p.time_signatures[0].denominator.into());
    root.insert("tempos".into(),p.tempo_map.iter().map(|t|json!({"position":t.tick,"bpm":t.micro_bpm as f64/1e6})).collect::<Vec<_>>().into());
    root.insert("time_signatures".into(),meters_to_bars(p)?.into_iter().map(|(bar,nu,de)|json!({"bar_position":bar,"beat_per_bar":nu,"beat_unit":de})).collect::<Vec<_>>().into());
    let mut expressions=restore(fields,"/expressions/",&["dyn","pitd"]);expressions.insert("dyn".into(),json!({"name":"dynamics","abbr":"dyn","type":"Curve","min":-240,"max":240,"default_value":0}));root.insert("expressions".into(),expressions.into());
    let mut tracks=Vec::new();let mut voice_parts=Vec::new();let tempo=TempoMap::new(&p.tempo_map,p.ppq).map_err(|e|E::at("/tempo",e.to_string()))?;
    for(ti,track)in selected(p,o){let source_index=p.extensions.get("score_import").and_then(|v|v.get("tracks")).and_then(Value::as_array).and_then(|tracks|tracks.iter().find(|t|t.get("project_track_id").and_then(Value::as_str)==Some(track.id.as_str()))).and_then(|t|t.get("source_track_index")).and_then(Value::as_u64).unwrap_or(ti as u64);let mut t=restore(fields,&format!("/tracks/{source_index}/"),&["track_name","volume","pan","mute","solo"]);
        t.insert("track_name".into(),track.name.clone().into());t.insert("volume".into(),json!(f64::from(track.gain_millidecibels)/1000.0));t.insert("pan".into(),json!(f64::from(track.pan_permille)/10.0));t.insert("mute".into(),track.muted.into());t.insert("solo".into(),track.solo.into());let target=tracks.len();tracks.push(Value::Object(t));
        for(pi,part)in parts(p,track)?.into_iter().enumerate(){let path=format!("/tracks/{ti}/parts/{pi}");let mut map=Map::new();map.insert("name".into(),part.name.clone().into());map.insert("track_no".into(),target.into());map.insert("position".into(),part.start.into());map.insert("duration".into(),part.duration.into());let mut notes=Vec::new();let mut gain=BTreeMap::new();
            for(ni,n)in part.notes.iter().enumerate(){let np=format!("{path}/notes/{ni}");let mut v=note_extras(p,n,"ustx",o,&["position","duration","tone","lyric","tuning","pitch","vibrato"]);
                v.insert("position".into(),(n.start_tick-part.start).into());v.insert("duration".into(),n.duration_tick.into());v.insert("tone".into(),n.midi_note.into());v.insert("lyric".into(),n.lyric.clone().into());
                if n.detune_millicents!=0||!n.curves.pitch_millicent_points.is_empty(){let points=pitch_points(n,&tempo,report,&np)?;v.insert("pitch".into(),json!({"snap_first":false,"data":points.into_iter().map(|(x,y)|json!({"x":x,"y":y,"shape":"l"})).collect::<Vec<_>>()}));}
                for point in &n.curves.gain_millidecibel_points{let tick=n.start_tick-part.start+point.tick;let value=(point.millidecibels as f64/100.0).round()as i64;
                    if !(-240..=240).contains(&value){return Err(E::at(&np,"USTX dyn curve exceeds the admitted -24..24 dB range"));}if value*100!=point.millidecibels{report.issue(format!("{np}/gain/{tick}"),D::Quantized,point.millidecibels,"USTX dynamics use integer tenths of a decibel.");}
                    if let Some(old)=gain.insert(tick,value){if old!=value{return Err(E::at(&np,"overlapping notes require incompatible part-level gain nodes"));}}
                }
                if n.curves.interpolation("DYN")!=verdant_domain::editing::Interpolation::Linear&&n.curves.gain_millidecibel_points.len()>1{report.issue(format!("{np}/gain/interpolation"),D::Unsupported,json!(n.curves.interpolation("DYN")),"USTX dynamics curves linearly interpolate exported nodes.");}
                unsupported_note(n,report,&np);notes.push(Value::Object(v));
            }
            map.insert("notes".into(),notes.into());if !gain.is_empty(){let(xs,ys):(Vec<_>,Vec<_>)=gain.into_iter().unzip();map.insert("curves".into(),json!([{"abbr":"dyn","xs":xs,"ys":ys}]));}voice_parts.push(Value::Object(map));
        }report_parts(p,track,report,&format!("/tracks/{ti}"))?;
    }
    root.insert("tracks".into(),tracks.into());root.insert("voice_parts".into(),voice_parts.into());
    report.issue("/synthesis",D::Metadata,"OpenUtau","USTX score-only export does not execute a phonemizer, voicebank samples, resampler or licensed synthesizer. Original sample-control fields can be retained but have no native engine semantics.");
    Ok(serde_yaml::to_string(&root)?.into_bytes())
}
