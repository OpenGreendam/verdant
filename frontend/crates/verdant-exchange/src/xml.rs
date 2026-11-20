//! Namespace-aware VOCALOID 3/4 XML scores. Source extensions are structured
//! XML trees; all musical fields are projected into the editable domain.
use std::collections::{BTreeMap, BTreeSet};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use verdant_domain::{Note, Project};
use verdant_domain::model::TempoPoint;
use crate::{Disposition as D, ExchangeError as E, ExchangeReport, ExportOptions, ImportOptions, Result, VocaloidVoice};
use crate::common::*;
use crate::vocaloid::{self, Controllers};
const KEY: &str = "exchange_vsqx";
const NS3: &str = "http://www.yamaha.co.jp/vocaloid/schema/vsq3/";
const NS4: &str = "http://www.yamaha.co.jp/vocaloid/schema/vsq4/";
#[derive(Clone, Copy)]
struct Schema { v4: bool, ns: &'static str }
impl Schema { fn t(self, three: &'static str, four: &'static str) -> &'static str { if self.v4 { four } else { three } } }

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Attribute { namespace: Option<String>, name: String, value: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum Content { Text(String), Element(Element) }
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Element { namespace: Option<String>, name: String, attributes: Vec<Attribute>, content: Vec<Content> }
impl Element {
    fn new(s: Schema, name: &str) -> Self { Self { namespace: Some(s.ns.into()), name: name.into(), attributes: Vec::new(), content: Vec::new() } }
    fn text(s: Schema, name: &str, value: impl ToString) -> Self { let mut e = Self::new(s, name); e.content.push(Content::Text(value.to_string())); e }
    fn push(&mut self, child: Element) { self.content.push(Content::Element(child)); }
    fn set(&mut self, s: Schema, name: &str, value: impl ToString) {
        let attributes=self.content.iter().find_map(|c|match c{Content::Element(e)if e.namespace.as_deref()==Some(s.ns)&&e.name==name=>Some(e.attributes.clone()),_=>None}).unwrap_or_default();
        self.content.retain(|c| !matches!(c,Content::Element(e) if e.namespace.as_deref()==Some(s.ns) && e.name==name));let mut child=Self::text(s,name,value);child.attributes=attributes;self.push(child);
    }
    fn attr(&mut self, name: &str, value: impl ToString) { self.attributes.retain(|a| a.namespace.is_some() || a.name != name); self.attributes.push(Attribute {namespace:None,name:name.into(),value:value.to_string()}); }
}
fn tree(n: Node<'_, '_>) -> Element {
    Element { namespace: n.tag_name().namespace().map(str::to_string), name: n.tag_name().name().into(),
        attributes: n.attributes().map(|a| Attribute {namespace:a.namespace().map(str::to_string),name:a.name().into(),value:a.value().into()}).collect(),
        content: n.children().filter_map(|c| if c.is_element() { Some(Content::Element(tree(c))) } else if c.is_text() { Some(Content::Text(c.text().unwrap_or("").into())) } else { None }).collect() }
}
fn children<'a,'d>(n: Node<'a,'d>, s: Schema, name: &str) -> Vec<Node<'a,'d>> {
    n.children().filter(|c| c.is_element() && c.tag_name().namespace()==Some(s.ns) && c.tag_name().name()==name).collect()
}
fn one<'a,'d>(n: Node<'a,'d>, s: Schema, name: &str, needed: bool, path: &str) -> Result<Option<Node<'a,'d>>> {
    let matches = children(n,s,name);
    if matches.len()>1 || (needed && matches.is_empty()) { return Err(E::at(format!("{path}/{name}"),"missing or duplicate singular XML field")); }
    Ok(matches.first().copied())
}
fn leaf(n: Node<'_, '_>, path: &str) -> Result<String> {
    if n.children().any(|c|c.is_element()) { return Err(E::at(path,"expected XML scalar text")); }
    Ok(n.children().filter_map(|c|c.text()).collect())
}
fn str_field(n: Node<'_, '_>, s: Schema, name: &str, path: &str) -> Result<String> { leaf(one(n,s,name,true,path)?.expect("required node"),path) }
fn int_field(n: Node<'_, '_>, s: Schema, name: &str, path: &str) -> Result<i64> {
    str_field(n,s,name,path)?.trim().parse().map_err(|_|E::at(format!("{path}/{name}"),"expected integer XML field"))
}
fn extras(n: Node<'_, '_>, s: Schema, known: &[&str], path: &str, report: &mut ExchangeReport) -> Result<Value> {
    let mut e=tree(n);e.content=e.content.into_iter().filter_map(|c|match c{
        Content::Element(mut child) if child.namespace.as_deref()==Some(s.ns)&&known.contains(&child.name.as_str())=>{
            if !child.attributes.is_empty()&&!child.content.iter().any(|c|matches!(c,Content::Element(_))){child.content.clear();Some(Content::Element(child))}else{None}
        },Content::Element(_)=>Some(c),_=>None
    }).collect();
    for c in &e.content { if let Content::Element(child)=c { report.issue(format!("{path}/{}",child.name),D::RetainedUnsupported,serde_json::to_value(child)?,"XML extension retained as a namespace-aware tree without executing synthesis or plugins."); } }
    for a in &e.attributes { report.issue(format!("{path}/@{}",a.name),D::RetainedUnsupported,a.value.clone(),"XML attribute retained."); }
    Ok(serde_json::to_value(e)?)
}
fn element_from(value: Option<&Value>, s: Schema, name: &str) -> Result<Element> {
    let mut e:Element=match value {Some(v)=>serde_json::from_value(v.clone())?,None=>Element::new(s,name)};
    e.namespace=Some(s.ns.into()); e.name=name.into(); Ok(e)
}
fn metadata<'a>(map: &'a Map<String,Value>, key: &str, enabled: bool) -> Option<&'a Value> { if enabled {map.get(key)} else {None} }
fn controller_name(id:&str,v4:bool)->&str { match (id,v4) {
    ("DYN",_)|("D",true)=>"dyn",
    ("PIT",false)|("P",true)=>"pitchBend",("PBS",false)|("S",true)=>"pitchBendSens",("BRE",false)|("B",true)=>"breathiness",
    ("BRI",false)|("R",true)=>"brightness",("CLE",false)|("C",true)=>"clearness",("GEN",false)|("G",true)=>"gender",
    ("POR",false)|("T",true)=>"portamento",("OPE",false)=>"opening",("GWL",false)|("W",true)=>"growl",_=>id }
}
fn controller_id(name:&str,v4:bool)->&str { match (name,v4) {
    ("dyn",false)=>"DYN",("dyn",true)=>"D",
    ("pitchBend",false)=>"PIT",("pitchBend",true)=>"P",("pitchBendSens",false)=>"PBS",("pitchBendSens",true)=>"S",
    ("breathiness",false)=>"BRE",("breathiness",true)=>"B",("brightness",false)=>"BRI",("brightness",true)=>"R",
    ("clearness",false)=>"CLE",("clearness",true)=>"C",("gender",false)=>"GEN",("gender",true)=>"G",
    ("portamento",false)=>"POR",("portamento",true)=>"T",("opening",_)=>"OPE",("growl",false)=>"GWL",("growl",true)=>"W",_=>name }
}
fn voice_fields(n:Node<'_,'_>,s:Schema)->Result<VocaloidVoice>{
    let v=VocaloidVoice {comp_id:str_field(n,s,s.t("compID","id"),"/vVoiceTable")?,name:str_field(n,s,s.t("vVoiceName","name"),"/vVoiceTable")?,
        language_id:bounded(int_field(n,s,s.t("vBS","bs"),"/vVoiceTable")?,0,127,"/vVoiceTable/language")? as u8,
        program:bounded(int_field(n,s,s.t("vPC","pc"),"/vVoiceTable")?,0,127,"/vVoiceTable/program")? as u8}; validate_voice(&v)?; Ok(v)
}
pub(crate) fn validate_voice(v:&VocaloidVoice)->Result<()> {
    if v.comp_id.len()!=16 || !v.comp_id.bytes().all(|b|b.is_ascii_graphic()) || v.name.trim().is_empty() || v.language_id>127 || v.program>127 {
        return Err(E::at("/options/vocaloid_voice","VSQX requires the actual 16-character voice comp_id, nonempty name, and 0..127 language_id/program; obtain these from the licensed destination voice or a real imported voice table"));
    } Ok(())
}

pub(crate) fn import(bytes:&[u8],options:&ImportOptions,report:&mut ExchangeReport)->Result<Project>{
    let text=std::str::from_utf8(bytes).map_err(|_|E::at("/encoding","VSQX must be UTF-8 XML"))?.trim_start_matches('\u{feff}');
    let doc=Document::parse_with_options(text,roxmltree::ParsingOptions{allow_dtd:false,..Default::default()})?;
    if doc.descendants().count()>options.limits.max_events {return Err(E::at("/xml","XML node limit exceeded"));}
    if doc.descendants().any(|n|n.ancestors().take(130).count()>128){return Err(E::at("/xml","XML nesting exceeds 128 levels"));}
    let root=doc.root_element(); let s=match(root.tag_name().name(),root.tag_name().namespace()) {("vsq3",Some(NS3))=>Schema{v4:false,ns:NS3},("vsq4",Some(NS4))=>Schema{v4:true,ns:NS4},_=>return Err(E::at("/xml","expected Yamaha vsq3/vsq4 root and matching namespace"))};
    check_unique_fields(root,s)?;
    let master=one(root,s,"masterTrack",true,"")?.expect("master");
    let ppq=bounded(int_field(master,s,"resolution","/masterTrack")?,1,MAX_TICK/4,"/masterTrack/resolution")?;
    let mut project=source_project(ppq);project.tempo_map.clear();project.time_signatures.clear();report.source_ppq=ppq;
    let pre=bounded(int_field(master,s,"preMeasure","/masterTrack")?,0,127,"/masterTrack/preMeasure")?;
    let mut bars=Vec::new();let mut meter_source=BTreeMap::new();
    for m in children(master,s,"timeSig") {
        let bar=int_field(m,s,s.t("posMes","m"),"/masterTrack/timeSig")?;
        bars.push((bar,bounded(int_field(m,s,s.t("nume","nu"),"/meter")?,1,65535,"/meter")? as u32,bounded(int_field(m,s,s.t("denomi","de"),"/meter")?,1,65536,"/meter")? as u32));
        meter_source.insert(bar,extras(m,s,&[s.t("posMes","m"),s.t("nume","nu"),s.t("denomi","de")],"/masterTrack/timeSig",report)?);
    }
    project.time_signatures=meters_from_bars(bars,ppq)?;
    let meter_bars=meters_to_bars(&project)?;
    for(m,(bar,_,_))in project.time_signatures.iter_mut().zip(meter_bars){if let Some(v)=meter_source.remove(&bar){m.unknown.insert(KEY.into(),v);}}
    for t in children(master,s,"tempo") {
        let tick=bounded(int_field(t,s,s.t("posTick","t"),"/tempo")?,0,MAX_TICK,"/tempo")?;
        let raw=bounded(int_field(t,s,s.t("bpm","v"),"/tempo")?,1,100000,"/tempo")?;
        let mut unknown=BTreeMap::new();unknown.insert(KEY.into(),extras(t,s,&[s.t("posTick","t"),s.t("bpm","v")],"/tempo",report)?);
        project.tempo_map.push(TempoPoint{tick,micro_bpm:raw as u64*10000,unknown});
    }
    default_maps(&mut project);
    let table=one(root,s,"vVoiceTable",true,"")?.expect("voice table");let mut voices=BTreeMap::new();
    for n in children(table,s,"vVoice") {let v=voice_fields(n,s)?;if voices.insert((v.language_id,v.program),v).is_some(){return Err(E::at("/vVoiceTable","duplicate language/program voice identity"));}}
    let mixer=one(root,s,"mixer",true,"")?.expect("mixer");let mut units=BTreeMap::new();
    for u in children(mixer,s,"vsUnit") {let number=int_field(u,s,s.t("vsTrackNo","tNo"),"/mixer")?;if units.insert(number,u).is_some(){return Err(E::at("/mixer","duplicate track unit"));}}
    let tracks=children(root,s,"vsTrack");if tracks.len()>options.limits.max_tracks || tracks.len()>16 {return Err(E::at("/vsTrack","VSQX supports at most 16 vocal tracks"));}
    let mut ids=BTreeSet::new();let mut total=0usize;
    for(ti,t)in tracks.into_iter().enumerate(){
        let path=format!("/vsTrack/{ti}");let number=bounded(int_field(t,s,s.t("vsTrackNo","tNo"),&path)?,0,127,&path)?;
        if !ids.insert(number){return Err(E::at(&path,"duplicate track number"));}
        let mut track=source_track(str_field(t,s,s.t("trackName","name"),&path)?);track.unknown.insert("vsqx_track_number".into(),number.into());
        let unit=units.remove(&number).ok_or_else(||E::at(&path,"missing matching mixer vsUnit"))?;
        track.muted=bounded(int_field(unit,s,s.t("mute","m"),&path)?,0,1,&path)?!=0;track.solo=bounded(int_field(unit,s,s.t("solo","s"),&path)?,0,1,&path)?!=0;
        let volume=int_field(unit,s,"vol",&path)?;track.gain_millidecibels=i32::try_from(volume.checked_mul(10).ok_or_else(||E::at(&path,"mixer gain overflow"))?).map_err(|_|E::at(&path,"mixer gain overflow"))?;
        let pan=bounded(int_field(unit,s,"pan",&path)?,0,127,&path)?;track.pan_permille=(((pan-64)as f64/if pan<64{64.0}else{63.0})*1000.0).round() as i32;
        let extra=extras(t,s,&[s.t("vsTrackNo","tNo"),s.t("trackName","name"),s.t("musicalPart","vsPart")],&path,report)?;
        if options.preserve_extensions{track.unknown.insert(KEY.into(),json!({"extra":extra,"unit":tree(unit),"pan":pan,"editable_pan":track.pan_permille}));}
        project.tracks.push(track);
        for(pi,p)in children(t,s,s.t("musicalPart","vsPart")).into_iter().enumerate(){
            let path=format!("{path}/parts/{pi}");let start=int_field(p,s,s.t("posTick","t"),&path)?;let duration=int_field(p,s,"playTime",&path)?;end(start,duration,&path)?;
            let mut singer_nodes=Vec::new();let mut singers=Vec::new();
            for singer in children(p,s,"singer") {
                let at=bounded(int_field(singer,s,s.t("posTick","t"),&path)?,0,duration,&path)?;
                let bs=bounded(int_field(singer,s,s.t("vBS","bs"),&path)?,0,127,&path)? as u8;let pc=bounded(int_field(singer,s,s.t("vPC","pc"),&path)?,0,127,&path)? as u8;
                if singers.last().is_some_and(|(t,_,_):&(i64,u8,u8)|*t>=at){return Err(E::at(&path,"singer events must strictly increase"));}
                if !voices.contains_key(&(bs,pc)){return Err(E::at(&path,"singer references missing voice table entry"));}
                singers.push((at,bs,pc));singer_nodes.push(tree(singer));
            }
            if singers.first().is_none_or(|v|v.0!=0){return Err(E::at(&path,"part must have an initial singer at tick zero"));}
            let mut notes=Vec::new();
            for(ni,n)in children(p,s,"note").into_iter().enumerate(){
                total=total.checked_add(1).ok_or_else(||E::at(&path,"note count overflow"))?;if total>options.limits.max_notes{return Err(E::at(&path,"note limit exceeded"));}
                let np=format!("{path}/notes/{ni}");let mut note=note(int_field(n,s,s.t("posTick","t"),&np)?,int_field(n,s,s.t("durTick","dur"),&np)?,int_field(n,s,s.t("noteNum","n"),&np)?,str_field(n,s,s.t("lyric","y"),&np)?,&np)?;
                set_velocity(&mut note,int_field(n,s,s.t("velocity","v"),&np)?,&np)?;
                let ph=one(n,s,s.t("phnms","p"),true,&np)?.expect("phoneme");let symbols=leaf(ph,&np)?;
                if let Some(lock)=ph.attribute("lock"){if !matches!(lock,"0"|"1"){return Err(E::at(&np,"phoneme lock must be 0 or 1"));}}
                let singer=singers.iter().take_while(|v|v.0<=note.start_tick).last().expect("initial singer");
                phonemes(&mut note,&symbols,vocaloid::language(i64::from(singer.1)),"vocaloid-xsampa");
                let extra=extras(n,s,&[s.t("posTick","t"),s.t("durTick","dur"),s.t("noteNum","n"),s.t("velocity","v"),s.t("lyric","y"),s.t("phnms","p")],&np,report)?;
                if options.preserve_extensions{note.unknown.insert(KEY.into(),json!({"extra":extra,"phonemes":tree(ph)}));}notes.push(note);
            }
            let mut controls=Controllers::new();let mut source_controls=BTreeMap::<String,Vec<Value>>::new();
            for cc in children(p,s,s.t("mCtrl","cc")){
                let tick=bounded(int_field(cc,s,s.t("posTick","t"),&path)?,0,duration,&path)?;
                let attr=one(cc,s,s.t("attr","v"),true,&path)?.expect("controller attr");let id=attr.attribute("id").filter(|v|!v.is_empty()).ok_or_else(||E::at(&path,"controller missing id"))?;
                let name=controller_name(id,s.v4).to_string();let raw=leaf(attr,&path)?.trim().parse::<i64>().map_err(|_|E::at(&path,"VSQX controller must be an integer"))?;let value=bounded(raw,i64::from(i32::MIN),i64::from(i32::MAX),&path)? as f64;
                let values=controls.entry(name.clone()).or_default();if !value.is_finite() || values.last().is_some_and(|v|v.0>=tick){return Err(E::at(&path,"controller ticks must strictly increase and values be finite"));}values.push((tick,value));
                source_controls.entry(name).or_default().push(json!({"pos":tick,"value":value,"xml":tree(cc)}));
            }
            vocaloid::apply_controllers(&mut notes,&controls,&path,report)?;
            let controllers=source_controls.into_iter().map(|(name,events)|json!({"name":name,"events":events})).collect::<Vec<_>>();
            let extra=extras(p,s,&[s.t("posTick","t"),"playTime",s.t("partName","name"),"singer",s.t("mCtrl","cc"),"note"],&path,report)?;
            let meta=if options.preserve_extensions{json!({"format":"vsqx","extra":extra,"singers":singer_nodes,"controllers":controllers,"controller_ppq":ppq,"curve_snapshot":curve_snapshot(notes.iter(),0)}) .as_object().expect("metadata").clone()}else{Map::new()};
            add_part(&mut project,ti,format!("vsqx:t{number}:p{pi}"),str_field(p,s,s.t("partName","name"),&path)?,start,duration,notes,meta)?;
        }
    }
    if !units.is_empty(){return Err(E::at("/mixer","mixer references nonexistent vocal track"));}
    let root_extra=extras(root,s,&["vVoiceTable","mixer","masterTrack","vsTrack"],"",report)?;
    let master_extra=extras(master,s,&["resolution","preMeasure","timeSig","tempo"],"/masterTrack",report)?;
    if options.preserve_extensions{project.extensions.insert(KEY.into(),json!({"v4":s.v4,"root":root_extra,"master":master_extra,"voices":tree(table),"mixer":tree(mixer),"pre_measure":pre}));}
    report.issue("/masterTrack/preMeasure",D::Metadata,pre,"Source leading measures remain at their exact source ticks on the editable timeline.");
    report.issue("/synthesis",D::Metadata,"VOCALOID","VSQX interchange does not execute or license VOCALOID, voice databases, style plugins or renderers.");Ok(project)
}

pub(crate) fn export(project:&Project,options:&ExportOptions,report:&mut ExchangeReport)->Result<Vec<u8>>{
    let source=if options.preserve_extensions{project.extensions.get(KEY)}else{None};
    let s=if source.and_then(|v|v.get("v4")).and_then(Value::as_bool)==Some(false){Schema{v4:false,ns:NS3}}else{Schema{v4:true,ns:NS4}};
    let mut root=element_from(source.and_then(|v|v.get("root")),s,if s.v4{"vsq4"}else{"vsq3"})?;
    let mut voices=if options.vocaloid_voice.is_none(){source.and_then(|v|v.get("voices")).map(|v|serde_json::from_value::<Element>(v.clone())).transpose()?}else{None};
    if let Some(v)=&options.vocaloid_voice {validate_voice(v)?;let mut table=Element::new(s,"vVoiceTable");let mut voice=Element::new(s,"vVoice");
        for(name,value)in[(s.t("vBS","bs"),v.language_id.to_string()),(s.t("vPC","pc"),v.program.to_string()),(s.t("compID","id"),v.comp_id.clone()),(s.t("vVoiceName","name"),v.name.clone())]{voice.push(Element::text(s,name,value));}
        let mut params=Element::new(s,s.t("vVoiceParam","vPrm"));for key in ["bre","bri","cle","gen","ope"]{params.push(Element::text(s,key,0));}voice.push(params);table.push(voice);voices=Some(table);
    }
    let table=voices.ok_or_else(||E::at("/options/vocaloid_voice","VSQX export needs actual vocaloid_voice metadata or an imported voice table; no licensed compID is invented"))?;
    let mut admitted=BTreeSet::new();let mut first=None;
    for c in &table.content {if let Content::Element(e)=c{if e.name=="vVoice"{let read=|name:&str|->Result<String>{let values:Vec<_>=e.content.iter().filter_map(|c|match c{Content::Element(n)if n.name==name&&n.namespace.as_deref()==Some(s.ns)=>Some(n),_=>None}).collect();if values.len()!=1{return Err(E::at("/vVoiceTable","missing/duplicate voice field"));}Ok(values[0].content.iter().filter_map(|c|if let Content::Text(v)=c{Some(v.as_str())}else{None}).collect())};
        let v=VocaloidVoice{comp_id:read(s.t("compID","id"))?,name:read(s.t("vVoiceName","name"))?,language_id:read(s.t("vBS","bs"))?.parse().map_err(|_|E::at("/vVoiceTable","invalid language"))?,program:read(s.t("vPC","pc"))?.parse().map_err(|_|E::at("/vVoiceTable","invalid program"))?};validate_voice(&v)?;if !admitted.insert((v.language_id,v.program)){return Err(E::at("/vVoiceTable","duplicate voice identity"));}first.get_or_insert((v.language_id,v.program));}}}
    let first=first.ok_or_else(||E::at("/options/vocaloid_voice","imported voice table is empty; supply actual voice metadata"))?;
    root.push(table);
    let tracks:Vec<_>=selected(project,options).collect();if tracks.is_empty()||tracks.len()>16{return Err(E::at("/tracks","VSQX requires one to sixteen vocal tracks"));}
    let mut track_numbers=BTreeMap::new();let mut used_numbers=BTreeSet::new();
    for(i,(ti,track))in tracks.iter().enumerate(){let original=if options.preserve_extensions{track.unknown.get("vsqx_track_number").and_then(Value::as_i64)}else{None};let number=original.unwrap_or(i as i64);bounded(number,0,127,"/tracks/number")?;if !used_numbers.insert(number){return Err(E::at("/tracks/number","duplicate VSQX source track number; remove the duplicated source metadata before exporting a copied track"));}track_numbers.insert(*ti,number);}
    let mut mixer=element_from(source.and_then(|v|v.get("mixer")),s,"mixer")?;mixer.content.retain(|c|!matches!(c,Content::Element(e)if e.name=="vsUnit"&&e.namespace.as_deref()==Some(s.ns)));
    if !mixer.content.iter().any(|c|matches!(c,Content::Element(e)if e.name=="masterUnit")) {let mut unit=Element::new(s,"masterUnit");for(name,value)in[(s.t("outDev","oDev"),0),(s.t("retLevel","rLvl"),0),("vol",0)]{unit.push(Element::text(s,name,value));}mixer.push(unit);}
    for(ti,track)in &tracks{let saved=metadata(&track.unknown,KEY,options.preserve_extensions);let mut unit=element_from(saved.and_then(|v|v.get("unit")),s,"vsUnit")?;unit.set(s,s.t("vsTrackNo","tNo"),track_numbers[ti]);unit.set(s,s.t("mute","m"),i32::from(track.muted));unit.set(s,s.t("solo","s"),i32::from(track.solo));
        let volume=(f64::from(track.gain_millidecibels)/10.0).round();if volume*10.0!=f64::from(track.gain_millidecibels){report.issue(format!("/tracks/{ti}/gain"),D::Quantized,track.gain_millidecibels,"VSQX mixer volume uses hundredths of a decibel.");}unit.set(s,"vol",volume as i64);
        let original=saved.filter(|v|v.get("editable_pan").and_then(Value::as_i64)==Some(i64::from(track.pan_permille))).and_then(|v|v.get("pan")).and_then(Value::as_i64);
        let pan=original.unwrap_or_else(||(64.0+f64::from(track.pan_permille)*if track.pan_permille<0{64.0}else{63.0}/1000.0).round()as i64);
        if original.is_none() && (((pan-64)as f64/if pan<64{64.0}else{63.0})*1000.0).round()as i32!=track.pan_permille{report.issue(format!("/tracks/{ti}/pan"),D::Quantized,track.pan_permille,"VSQX pan uses 128 positions.");}unit.set(s,"pan",pan);
        for(name,value)in[(s.t("inGain","iGin"),0),(s.t("sendLevel","sLvl"),-898),(s.t("sendEnable","sEnable"),0)]{if !unit.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==name)){unit.push(Element::text(s,name,value));}}mixer.push(unit);
    }
    for(name,fields)in[(s.t("seUnit","monoUnit"),vec![(s.t("inGain","iGin"),0),(s.t("sendLevel","sLvl"),-898),(s.t("sendEnable","sEnable"),0),(s.t("mute","m"),0),(s.t("solo","s"),0),("pan",64),("vol",0)]),(s.t("karaokeUnit","stUnit"),vec![(s.t("inGain","iGin"),0),(s.t("sendLevel","sLvl"),-898),(s.t("sendEnable","sEnable"),0),(s.t("mute","m"),0),(s.t("solo","s"),0),("vol",0)])]{if !mixer.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==name)){let mut e=Element::new(s,name);for(k,v)in fields{e.push(Element::text(s,k,v));}mixer.push(e);}}
    root.push(mixer);let mut master=element_from(source.and_then(|v|v.get("master")),s,"masterTrack")?;master.set(s,"resolution",project.ppq);master.set(s,"preMeasure",source.and_then(|v|v.get("pre_measure")).and_then(Value::as_i64).unwrap_or(0));
    for(name,value)in[("seqName",project.project_id.as_str()),("comment","")]{if !master.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==name)){master.push(Element::text(s,name,value));}}
    for(m,(bar,nu,de))in project.time_signatures.iter().zip(meters_to_bars(project)?){let mut e=element_from(if options.preserve_extensions{m.unknown.get(KEY)}else{None},s,"timeSig")?;for(k,v)in[(s.t("posMes","m"),bar),(s.t("nume","nu"),i64::from(nu)),(s.t("denomi","de"),i64::from(de))]{e.set(s,k,v);}master.push(e);}
    for t in &project.tempo_map{let mut e=element_from(if options.preserve_extensions{t.unknown.get(KEY)}else{None},s,"tempo")?;let raw=(t.micro_bpm as f64/10000.0).round()as u64;if raw==0{return Err(E::at("/tempo","VSQX tempo rounded to zero"));}if raw*10000!=t.micro_bpm{report.issue(format!("/tempo/{}",t.tick),D::Quantized,t.micro_bpm,"VSQX tempo resolution is 0.01 BPM.");}e.set(s,s.t("posTick","t"),t.tick);e.set(s,s.t("bpm","v"),raw);master.push(e);}root.push(master);
    for(ti,track)in tracks{let saved=metadata(&track.unknown,KEY,options.preserve_extensions);let mut t=element_from(saved.and_then(|v|v.get("extra")),s,"vsTrack")?;t.set(s,s.t("vsTrackNo","tNo"),track_numbers[&ti]);t.set(s,s.t("trackName","name"),&track.name);
        for(pi,part)in parts(project,track)?.into_iter().enumerate(){let path=format!("/tracks/{ti}/parts/{pi}");let meta=if options.preserve_extensions && part.metadata.get("format").and_then(Value::as_str)==Some("vsqx"){Some(&part.metadata)}else{None};let mut p=element_from(meta.and_then(|v|v.get("extra")),s,s.t("musicalPart","vsPart"))?;
            if !p.content.iter().any(|c|matches!(c,Content::Element(e)if e.name=="comment")){p.push(Element::text(s,"comment",""));}
            p.set(s,s.t("posTick","t"),part.start);p.set(s,"playTime",part.duration);p.set(s,s.t("partName","name"),&part.name);
            if !p.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==s.t("stylePlugin","sPlug"))){let mut plug=Element::new(s,s.t("stylePlugin","sPlug"));for(k,v)in[(s.t("stylePluginID","id"),"ACA9C502-A04B-42b5-B2EB-5CEA36D16FCE"),(s.t("stylePluginName","name"),"VOCALOID2 Compatible Style"),("version","3.0.0.1")]{plug.push(Element::text(s,k,v));}p.push(plug);}
            if !p.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==s.t("partStyle","pStyle"))){let mut style=Element::new(s,s.t("partStyle","pStyle"));let mut attr=Element::text(s,s.t("attr","v"),50);attr.attr("id","accent");style.push(attr);p.push(style);}
            let singers=if options.vocaloid_voice.is_none(){meta.and_then(|v|v.get("singers")).map(|v|serde_json::from_value::<Vec<Element>>(v.clone())).transpose()?}else{None};
            if let Some(singers)=singers{let mut previous=None;for singer in singers{
                let at=scalar(&singer,s,s.t("posTick","t"))?.parse::<i64>().map_err(|_|E::at(&path,"invalid singer tick"))?;
                let bs=scalar(&singer,s,s.t("vBS","bs"))?.parse::<u8>().map_err(|_|E::at(&path,"invalid singer language"))?;
                let pc=scalar(&singer,s,s.t("vPC","pc"))?.parse::<u8>().map_err(|_|E::at(&path,"invalid singer program"))?;
                if at<0||at>part.duration||previous.is_none()&&at!=0||previous.is_some_and(|t|t>=at)||!admitted.contains(&(bs,pc)){return Err(E::at(&path,"invalid singer timing or missing voice table reference"));}previous=Some(at);p.push(singer);
            }}else{let mut singer=Element::new(s,"singer");for(k,v)in[(s.t("posTick","t"),0),(s.t("vBS","bs"),i64::from(first.0)),(s.t("vPC","pc"),i64::from(first.1))]{singer.push(Element::text(s,k,v));}p.push(singer);}
            for cc in vocaloid::exported_controllers(&part,project.ppq,project.ppq,options.preserve_extensions,report,&path)?{let c=object(&cc,&path)?;let name=text(required(c,"name",&path)?,&path)?;for event in array(required(c,"events",&path)?,&path)?{let event=object(event,&path)?;let mut e=element_from(event.get("xml"),s,s.t("mCtrl","cc"))?;e.set(s,s.t("posTick","t"),integer(required(event,"pos",&path)?,&path)?);let mut attr=e.content.iter().find_map(|c|match c{Content::Element(child)if child.name==s.t("attr","v")&&child.namespace.as_deref()==Some(s.ns)=>Some(child.clone()),_=>None}).unwrap_or_else(||Element::new(s,s.t("attr","v")));attr.content=vec![Content::Text(number(required(event,"value",&path)?,&path)?.to_string())];if event.get("xml").is_none(){attr.attr("id",controller_id(name,s.v4));}e.content.retain(|c|!matches!(c,Content::Element(e)if e.name==s.t("attr","v")&&e.namespace.as_deref()==Some(s.ns)));e.push(attr);p.push(e);}}
            for n in &part.notes{let saved=metadata(&n.unknown,KEY,options.preserve_extensions);let mut e=element_from(saved.and_then(|v|v.get("extra")),s,"note")?;for(k,v)in[(s.t("posTick","t"),n.start_tick-part.start),(s.t("durTick","dur"),n.duration_tick),(s.t("noteNum","n"),i64::from(n.midi_note)),(s.t("velocity","v"),i64::from(velocity(n)?))]{e.set(s,k,v);}e.set(s,s.t("lyric","y"),&n.lyric);
                // extras() retains the source phoneme element as an empty shell to keep its
                // attributes; the rebuilt element below is the single phnms/p child, so drop
                // the shell or the exported note would carry duplicate singular fields.
                e.content.retain(|c|!matches!(c,Content::Element(x) if x.name==s.t("phnms","p")&&x.namespace.as_deref()==Some(s.ns)));
                let mut ph=element_from(saved.and_then(|v|v.get("phonemes")),s,s.t("phnms","p"))?;let text=destination_phonemes(n,&["vocaloid-xsampa","xsampa","x-sampa"],report,&path);let old:String=ph.content.iter().filter_map(|c|if let Content::Text(v)=c{Some(v.as_str())}else{None}).collect();if text!=old || ph.attributes.is_empty(){ph.attr("lock",i32::from(!text.is_empty()));}ph.content=vec![Content::Text(text)];e.push(ph);p.push(e);
            }t.push(p);
        }root.push(t);
    }
    for(name,value)in[("vender","Yamaha Corporation"),("version",if s.v4{"4.0.0.3"}else{"3.0.0.0"})]{if !root.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==name)){root.push(Element::text(s,name,value));}}
    for name in [s.t("seTrack","monoTrack"),s.t("karaokeTrack","stTrack")]{if !root.content.iter().any(|c|matches!(c,Content::Element(e)if e.name==name)){root.push(Element::new(s,name));}}
    // XSD sequence order is part of the real format, including retained nodes.
    order(&mut root,s,&["vender","version","vVoiceTable","mixer","masterTrack","vsTrack",s.t("seTrack","monoTrack"),s.t("karaokeTrack","stTrack"),"aux"]);
    order_schema(&mut root,s);
    let mut out=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");let mut remaining=options.limits.max_events;write_element(&root,&mut out,0,&mut remaining)?;Ok(out.into_bytes())
}
fn check_unique_fields(root:Node<'_,'_>,s:Schema)->Result<()>{
    for node in root.descendants().filter(|n|n.is_element()&&n.tag_name().namespace()==Some(s.ns)){
        let names:Vec<&str>=match node.tag_name().name(){
            "vsq3"|"vsq4"=>vec!["vender","version","vVoiceTable","mixer","masterTrack",s.t("seTrack","monoTrack"),s.t("karaokeTrack","stTrack")],
            "masterTrack"=>vec!["seqName","comment","resolution","preMeasure"],
            "mixer"=>vec!["masterUnit",s.t("seUnit","monoUnit"),s.t("karaokeUnit","stUnit")],
            "vsTrack"=>vec![s.t("vsTrackNo","tNo"),s.t("trackName","name"),"comment"],
            "vsUnit"=>vec![s.t("vsTrackNo","tNo"),s.t("inGain","iGin"),s.t("sendLevel","sLvl"),s.t("sendEnable","sEnable"),s.t("mute","m"),s.t("solo","s"),"pan","vol"],
            "musicalPart"|"vsPart"=>vec![s.t("posTick","t"),"playTime",s.t("partName","name"),"comment",s.t("stylePlugin","sPlug"),s.t("partStyle","pStyle"),"plane"],
            "note"=>vec![s.t("posTick","t"),s.t("durTick","dur"),s.t("noteNum","n"),s.t("velocity","v"),s.t("lyric","y"),s.t("phnms","p"),s.t("noteStyle","nStyle")],
            "singer"=>vec![s.t("posTick","t"),s.t("vBS","bs"),s.t("vPC","pc")],
            "vVoice"=>vec![s.t("vBS","bs"),s.t("vPC","pc"),s.t("compID","id"),s.t("vVoiceName","name"),s.t("vVoiceParam","vPrm"),"id2","vPrm2"],
            "timeSig"=>vec![s.t("posMes","m"),s.t("nume","nu"),s.t("denomi","de")],
            "tempo"=>vec![s.t("posTick","t"),s.t("bpm","v")],
            "mCtrl"|"cc"=>vec![s.t("posTick","t"),s.t("attr","v")],_=>Vec::new()
        };
        let mut seen=BTreeSet::new();for child in node.children().filter(|n|n.is_element()&&n.tag_name().namespace()==Some(s.ns)){
            let name=child.tag_name().name();if names.contains(&name)&&!seen.insert(name){return Err(E::at(format!("/xml/{}/{name}",node.tag_name().name()),"duplicate singular XML field"));}
        }
    }Ok(())
}
fn scalar(e:&Element,s:Schema,name:&str)->Result<String>{let values:Vec<_>=e.content.iter().filter_map(|c|match c{Content::Element(n)if n.name==name&&n.namespace.as_deref()==Some(s.ns)=>Some(n),_=>None}).collect();if values.len()!=1{return Err(E::at("/xml","missing/duplicate scalar"));}Ok(values[0].content.iter().filter_map(|c|if let Content::Text(v)=c{Some(v.as_str())}else{None}).collect())}
fn order(e:&mut Element,s:Schema,names:&[&str]){e.content.sort_by_key(|c|match c{Content::Element(e)if e.namespace.as_deref()==Some(s.ns)=>names.iter().position(|n|*n==e.name).unwrap_or(names.len()),_=>names.len()});}
fn order_schema(e:&mut Element,s:Schema){
    let names:Vec<&str>=match e.name.as_str(){
        "masterTrack"=>vec!["seqName","comment","resolution","preMeasure","timeSig","tempo"],
        "mixer"=>vec!["masterUnit","vsUnit",s.t("seUnit","monoUnit"),s.t("karaokeUnit","stUnit")],
        "vsUnit"=>vec![s.t("vsTrackNo","tNo"),s.t("inGain","iGin"),s.t("vstPlugin","plug"),s.t("sendLevel","sLvl"),s.t("sendEnable","sEnable"),s.t("mute","m"),s.t("solo","s"),"pan","vol"],
        "vsTrack"=>vec![s.t("vsTrackNo","tNo"),s.t("trackName","name"),"comment",s.t("musicalPart","vsPart")],
        "musicalPart"|"vsPart"=>vec![s.t("posTick","t"),"playTime",s.t("partName","name"),"comment",s.t("stylePlugin","sPlug"),s.t("partStyle","pStyle"),"singer",s.t("mCtrl","cc"),"note","plane"],
        "note"=>vec![s.t("posTick","t"),s.t("durTick","dur"),s.t("noteNum","n"),s.t("velocity","v"),s.t("lyric","y"),s.t("phnms","p"),s.t("noteStyle","nStyle")],
        "singer"=>vec![s.t("posTick","t"),s.t("vBS","bs"),s.t("vPC","pc")],
        "vVoice"=>vec![s.t("vBS","bs"),s.t("vPC","pc"),s.t("compID","id"),s.t("vVoiceName","name"),s.t("vVoiceParam","vPrm"),"id2","vPrm2"],
        "timeSig"=>vec![s.t("posMes","m"),s.t("nume","nu"),s.t("denomi","de")],
        "tempo"=>vec![s.t("posTick","t"),s.t("bpm","v")],
        "mCtrl"|"cc"=>vec![s.t("posTick","t"),s.t("attr","v")],
        _=>Vec::new()
    };
    if !names.is_empty(){order(e,s,&names);}
    for c in &mut e.content{if let Content::Element(child)=c{if child.namespace.as_deref()==Some(s.ns){order_schema(child,s);}}}
}
fn valid_name(s:&str)->bool{let mut c=s.chars();c.next().is_some_and(|c|c=='_'||c.is_alphabetic())&&c.all(|c|c=='_'||c=='-'||c=='.'||c.is_alphanumeric())}
fn escape(s:&str,out:&mut String)->Result<()>{for c in s.chars(){match c{'&'=>out.push_str("&amp;"),'<'=>out.push_str("&lt;"),'>'=>out.push_str("&gt;"),'"'=>out.push_str("&quot;"),'\''=>out.push_str("&apos;"),'\r'=>out.push_str("&#13;"),'\n'=>out.push_str("&#10;"),'\t'=>out.push_str("&#9;"),c if c=='\u{fffe}'||c=='\u{ffff}'||(c as u32)<0x20=>return Err(E::at("/xml","forbidden XML character")),c=>out.push(c)}}Ok(())}
fn write_element(e:&Element,out:&mut String,depth:usize,budget:&mut usize)->Result<()>{
    if depth>128 || *budget==0 || !valid_name(&e.name){return Err(E::at("/xml","XML depth, node or element-name limit"));}
    *budget-=1;
    out.push('<');out.push_str(&e.name);out.push_str(" xmlns=\"");escape(e.namespace.as_deref().unwrap_or(""),out)?;out.push('"');let mut ids=BTreeSet::new();
    for(i,a)in e.attributes.iter().enumerate(){if !valid_name(&a.name)||a.name=="xmlns"||!ids.insert((&a.namespace,&a.name)){return Err(E::at("/xml","invalid/duplicate XML attribute"));}out.push(' ');if let Some(ns)=&a.namespace{if ns=="http://www.w3.org/XML/1998/namespace"{out.push_str("xml:");}else{out.push_str(&format!("xmlns:a{i}=\""));escape(ns,out)?;out.push_str(&format!("\" a{i}:"));}}out.push_str(&a.name);out.push_str("=\"");escape(&a.value,out)?;out.push('"');}
    out.push('>');for c in &e.content{match c{Content::Text(t)=>escape(t,out)?,Content::Element(child)=>write_element(child,out,depth+1,budget)?}}out.push_str("</");out.push_str(&e.name);out.push('>');Ok(())
}
