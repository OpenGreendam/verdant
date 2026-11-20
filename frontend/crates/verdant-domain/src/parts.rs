//! Authored parts keep stable content/instance identities. Track.notes is the
//! concrete projection used by every existing renderer, index and clipboard.
//! The reducer updates content and every linked projection in one transaction.
use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use crate::{Command, Note, NoteId, PointId, Project, TrackId};
use crate::store::{MetaPatch, NotePatch};
pub const KEY: &str = "parts";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Parts { pub contents: BTreeMap<String, PartContent>, pub instances: Vec<PartInstance> }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartContent { pub id: String, pub notes: Vec<Note> }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartInstance {
    pub id: String, pub content_id: String, pub name: String, pub track: TrackId,
    pub start_tick: i64, pub duration_tick: i64, pub source_offset_tick: i64,
    pub muted: bool,
    /// Content-note id → this instance's stable concrete note id.
    pub bindings: BTreeMap<NoteId, NoteId>,
}
impl Parts {
    pub fn load(project:&Project)->Result<Self,String>{project.extensions.get(KEY).map(|v|serde_json::from_value(v.clone()).map_err(|e|format!("invalid parts: {e}"))).unwrap_or_else(||Ok(Self::default()))}
    pub fn command(self)->Command { Command::SetProjectExtension{key:KEY.into(),value:Some(serde_json::to_value(self).expect("serializable parts"))} }
    pub fn instance(&self,id:&str)->Option<&PartInstance>{self.instances.iter().find(|p|p.id==id)}
    pub fn create(&mut self,project:&Project,track:&TrackId,ids:&[NoteId],name:String)->Result<String,String>{
        let t=project.track(track).ok_or("track no longer exists")?;let ids:BTreeSet<_>=ids.iter().collect();
        let selected:Vec<_>=t.notes.iter().filter(|n|ids.contains(&n.id)).collect();
        let start=selected.iter().map(|n|n.start_tick).min().ok_or("select notes for a part")?;
        let end=selected.iter().map(|n|n.end_tick()).max().expect("selected notes");
        if self.instances.iter().any(|p|p.track==*track&&p.bindings.values().any(|id|ids.contains(id))){return Err("selected notes already belong to a part".into());}
        let content_id=NoteId::generate().to_string();let id=NoteId::generate().to_string();
        let notes=selected.iter().map(|n|{let mut n=(**n).clone();n.start_tick-=start;n}).collect();
        let bindings=selected.iter().map(|n|(n.id.clone(),n.id.clone())).collect();
        self.contents.insert(content_id.clone(),PartContent{id:content_id.clone(),notes});
        self.instances.push(PartInstance{id:id.clone(),content_id,name,track:track.clone(),start_tick:start,duration_tick:end-start,source_offset_tick:0,muted:false,bindings});Ok(id)
    }
    /// An empty part over `[start, start+duration)`. Notes later inserted
    /// wholly inside it join its content (see `synchronize`).
    pub fn create_empty(&mut self,project:&Project,track:&TrackId,start:i64,duration:i64,name:String)->Result<String,String>{
        project.track(track).ok_or("track no longer exists")?;
        if start<0||duration<1{return Err("part needs a positive length".into());}
        if self.instances.iter().any(|p|p.track==*track&&p.start_tick<start+duration&&start<p.start_tick+p.duration_tick){return Err("parts on one track cannot overlap".into());}
        let content_id=NoteId::generate().to_string();let id=NoteId::generate().to_string();
        self.contents.insert(content_id.clone(),PartContent{id:content_id.clone(),notes:Vec::new()});
        self.instances.push(PartInstance{id:id.clone(),content_id,name,track:track.clone(),start_tick:start,duration_tick:duration,source_offset_tick:0,muted:false,bindings:BTreeMap::new()});Ok(id)
    }
    /// The first "片段 N" name not used by any part.
    pub fn next_name(&self,prefix:&str)->String{
        (1..).map(|n|format!("{prefix} {n}")).find(|name|!self.instances.iter().any(|p|&p.name==name)).expect("unbounded")
    }
    pub fn duplicate(&mut self,id:&str,at:i64,linked:bool)->Result<String,String>{
        let mut copy=self.instance(id).cloned().ok_or("part no longer exists")?;copy.id=NoteId::generate().to_string();copy.start_tick=at.max(0);
        copy.name=format!("{} 副本",copy.name);
        if !linked{let mut content=self.contents.get(&copy.content_id).cloned().ok_or("part content missing")?;content.id=NoteId::generate().to_string();copy.content_id=content.id.clone();self.contents.insert(content.id.clone(),content);}
        copy.bindings.values_mut().for_each(|id|*id=NoteId::generate());let new=copy.id.clone();self.instances.push(copy);Ok(new)
    }
    pub fn unlink(&mut self,id:&str)->Result<(),String>{
        let index=self.instances.iter().position(|p|p.id==id).ok_or("part no longer exists")?;
        let mut content=self.contents.get(&self.instances[index].content_id).cloned().ok_or("part content missing")?;
        content.id=NoteId::generate().to_string();self.instances[index].content_id=content.id.clone();self.contents.insert(content.id.clone(),content);Ok(())
    }
    pub fn split(&mut self,id:&str,tick:i64)->Result<String,String>{
        let index=self.instances.iter().position(|p|p.id==id).ok_or("part no longer exists")?;let original=self.instances[index].clone();
        let cut=tick-original.start_tick;if cut<=0||cut>=original.duration_tick{return Err("split position must be inside the part".into());}
        let mut right=original.clone();right.id=NoteId::generate().to_string();right.start_tick=tick;right.duration_tick-=cut;right.source_offset_tick+=cut;
        right.bindings.values_mut().for_each(|id|*id=NoteId::generate());self.instances[index].duration_tick=cut;
        let id=right.id.clone();self.instances.push(right);Ok(id)
    }
    pub fn join(&mut self,left:&str,right:&str)->Result<(),String>{
        let a=self.instance(left).cloned().ok_or("part no longer exists")?;let b=self.instance(right).cloned().ok_or("part no longer exists")?;
        if a.content_id!=b.content_id||a.track!=b.track||a.muted!=b.muted||a.start_tick+a.duration_tick!=b.start_tick||a.source_offset_tick+a.duration_tick!=b.source_offset_tick{return Err("join requires adjacent instances of the same content, in source order".into());}
        self.instances.iter_mut().find(|p|p.id==left).expect("left").duration_tick+=b.duration_tick;self.instances.retain(|p|p.id!=right);Ok(())
    }
    pub fn validate(&self,project:&Project)->Result<(),String>{
        let mut ids=BTreeSet::new();let mut bindings=BTreeSet::new();
        for (id,content) in &self.contents{if id!=&content.id||id.is_empty(){return Err("invalid part content identity".into());}let mut note_ids=BTreeSet::new();for n in &content.notes{if !note_ids.insert(&n.id)||n.start_tick<0||n.duration_tick<1{return Err("invalid part source note".into());}crate::editing::validate_note(n)?;}}
        for p in &self.instances{
            if !ids.insert(&p.id)||p.id.is_empty()||project.track(&p.track).is_none()||!self.contents.contains_key(&p.content_id)||p.start_tick<0||p.duration_tick<1||p.source_offset_tick<0||p.start_tick.checked_add(p.duration_tick).is_none()||p.source_offset_tick.checked_add(p.duration_tick).is_none(){return Err("invalid part instance identity, track or bounds".into());}
            for id in p.bindings.values(){if !bindings.insert(id){return Err("one concrete note cannot belong to two part instances".into());}}
        }Ok(())
    }
}

fn projection(source:&Note,part:&PartInstance)->Option<Note>{
    let from=part.source_offset_tick.max(source.start_tick);let to=(part.source_offset_tick+part.duration_tick).min(source.end_tick());if to<=from{return None;}
    let id=part.bindings.get(&source.id).cloned().unwrap_or_else(||NoteId(format!("part:{}:{}",part.id,source.id)));
    let mut n=crate::editing::crop_note(source,from-source.start_tick,to-source.start_tick,id);
    n.start_tick=part.start_tick+from-part.source_offset_tick;if part.muted{n.set_muted(true);}
    // Source point IDs survive moves and linked edits. Crop endpoints use
    // stable edge IDs rather than allocating during journal replay.
    for p in &mut n.curves.pitch_millicent_points{let absolute=p.tick+from-source.start_tick;let source_id=source.curves.pitch_millicent_points.iter().find(|q|q.tick==absolute).map(|q|q.id.to_string()).unwrap_or_else(||format!("edge:{absolute}"));p.id=PointId(format!("{}:PIT:{source_id}",n.id));}
    for p in &mut n.curves.gain_millidecibel_points{let absolute=p.tick+from-source.start_tick;let source_id=source.curves.gain_millidecibel_points.iter().find(|q|q.tick==absolute).map(|q|q.id.to_string()).unwrap_or_else(||format!("edge:{absolute}"));p.id=PointId(format!("{}:DYN:{source_id}",n.id));}
    for (key,points) in &mut n.curves.expression_points{for p in points{let absolute=p.tick+from-source.start_tick;let source_id=source.curves.expression(key).iter().find(|q|q.tick==absolute).map(|q|q.id.to_string()).unwrap_or_else(||format!("edge:{absolute}"));p.id=PointId(format!("{}:{key}:{source_id}",n.id));}}
    Some(n)
}

pub(crate) fn synchronize(before:&Project,project:&mut Project,commands:&[Command],out:&mut Vec<NotePatch>,meta:&mut Vec<MetaPatch>)->Result<(),String>{
    let old=Parts::load(before)?;let mut parts=Parts::load(project)?;
    parts.instances.retain(|part|project.track(&part.track).is_some());
    if old.instances.is_empty()&&parts.instances.is_empty(){return Ok(());}
    let explicit=commands.iter().any(|c|matches!(c,Command::SetProjectExtension{key,..}if key==KEY));
    if !explicit{
        let mut updates:BTreeMap<(String,NoteId),Option<Note>>=BTreeMap::new();
        for part in &old.instances{
            if project.track(&part.track).is_none(){continue;}
            let Some(content)=old.contents.get(&part.content_id)else{return Err("missing part source".into())};
            for source in &content.notes{
                let Some(previous)=projection(source,part)else{continue;};
                let current=project.track(&part.track).and_then(|t|t.note(&previous.id));
                if current==Some(&previous){continue;}
                let changed=current.map(|current|{
                    let mut note=source.clone();
                    note.start_tick=source.start_tick+(current.start_tick-previous.start_tick);
                    note.duration_tick=source.duration_tick+(current.duration_tick-previous.duration_tick);
                    note.midi_note=current.midi_note;note.detune_millicents=current.detune_millicents;note.lyric=current.lyric.clone();note.pronunciation=current.pronunciation.clone();note.vibrato=current.vibrato.clone();note.portamento_ms=current.portamento_ms;
                    if current.unknown!=previous.unknown{note.unknown=current.unknown.clone();if part.muted&&source.is_muted()==previous.is_muted(){note.set_muted(source.is_muted());}}
                    if current.curves!=previous.curves{
                        // Merge an edited visible slice into the source. Points
                        // outside the trim survive; stable interior IDs are
                        // restored from the previous projection when possible.
                        let shift=part.source_offset_tick+current.start_tick-part.start_tick-source.start_tick;
                        let end=shift+current.duration_tick;
                        let map_id=|id:&PointId,previous_ids:Vec<(PointId,PointId)>|previous_ids.into_iter().find(|(p,_)|p==id).map(|(_,s)|s).unwrap_or_else(||id.clone());
                        if current.curves.pitch_millicent_points!=previous.curves.pitch_millicent_points{let mapping=previous.curves.pitch_millicent_points.iter().zip(&source.curves.pitch_millicent_points).map(|(p,s)|(p.id.clone(),s.id.clone())).collect::<Vec<_>>();note.curves.pitch_millicent_points.retain(|p|p.tick<shift||p.tick>end);note.curves.pitch_millicent_points.extend(current.curves.pitch_millicent_points.iter().cloned().map(|mut p|{p.tick+=shift;p.id=map_id(&p.id,mapping.clone());p}));note.curves.pitch_millicent_points=crate::command::normalize_points(note.curves.pitch_millicent_points,|p|p.tick);}
                        if current.curves.gain_millidecibel_points!=previous.curves.gain_millidecibel_points{note.curves.gain_millidecibel_points.retain(|p|p.tick<shift||p.tick>end);note.curves.gain_millidecibel_points.extend(current.curves.gain_millidecibel_points.iter().cloned().map(|mut p|{p.tick+=shift;p}));note.curves.gain_millidecibel_points=crate::command::normalize_points(note.curves.gain_millidecibel_points,|p|p.tick);}
                        for key in current.curves.expression_points.keys().chain(previous.curves.expression_points.keys()).collect::<BTreeSet<_>>(){if current.curves.expression(key)!=previous.curves.expression(key){let points=note.curves.expression_points.entry(key.clone()).or_default();points.retain(|p|p.tick<shift||p.tick>end);points.extend(current.curves.expression(key).iter().cloned().map(|mut p|{p.tick+=shift;p}));*points=crate::command::normalize_points(std::mem::take(points),|p|p.tick);}}
                        note.curves.unknown=current.curves.unknown.clone();
                    }
                    note
                });
                let key=(part.content_id.clone(),source.id.clone());if updates.get(&key).is_some_and(|v|v!=&changed){return Err("conflicting simultaneous edits to linked part content".into());}updates.insert(key,changed);
            }
        }
        for ((content_id,id),note) in updates{let content=parts.contents.get_mut(&content_id).ok_or("part source missing")?;content.notes.retain(|n|n.id!=id);if let Some(n)=note{content.notes.push(n);}}
        // Newly inserted notes inside a single authored instance join its
        // content. Ambiguous overlaps remain explicit ungrouped notes.
        for patch in out.iter().filter(|p|p.before.is_none()){
            let Some(note)=project.track(&patch.track).and_then(|t|t.note(&patch.id)).cloned()else{continue;};
            if old.instances.iter().any(|p|p.bindings.values().any(|id|id==&note.id)){continue;}
            let candidates:Vec<_>=parts.instances.iter().enumerate().filter(|(_,p)|p.track==patch.track&&note.start_tick>=p.start_tick&&note.end_tick()<=p.start_tick+p.duration_tick).map(|(i,_)|i).collect();
            if candidates.len()!=1{continue;}let p=&mut parts.instances[candidates[0]];let mut source=note.clone();source.start_tick=source.start_tick-p.start_tick+p.source_offset_tick;p.bindings.insert(source.id.clone(),note.id.clone());parts.contents.get_mut(&p.content_id).ok_or("part source missing")?.notes.push(source);
        }
    }
    for content in parts.contents.values_mut(){content.notes.sort_by(|a,b|(a.start_tick,&a.id).cmp(&(b.start_tick,&b.id)));}
    // Deterministic bindings for new content notes in every linked copy.
    for p in &mut parts.instances{if let Some(content)=parts.contents.get(&p.content_id){for n in &content.notes{p.bindings.entry(n.id.clone()).or_insert_with(||NoteId(format!("part:{}:{}",p.id,n.id)));}}}
    parts.validate(project)?;
    let mut owned:BTreeMap<TrackId,BTreeSet<NoteId>>=BTreeMap::new();
    for p in old.instances.iter().chain(&parts.instances){owned.entry(p.track.clone()).or_default().extend(p.bindings.values().cloned());}
    let mut projected:BTreeMap<TrackId,Vec<Note>>=BTreeMap::new();
    for part in &parts.instances{let content=parts.contents.get(&part.content_id).ok_or("part source missing")?;for source in &content.notes{if let Some(note)=projection(source,part){projected.entry(part.track.clone()).or_default().push(note);}}}
    for (track,ids) in owned{
        let Some(t)=project.track_mut(&track)else{continue;};let old_notes:Vec<_>=t.notes.iter().filter(|n|ids.contains(&n.id)).cloned().collect();
        t.notes.retain(|n|!ids.contains(&n.id));let new_notes=projected.remove(&track).unwrap_or_default();
        for n in &old_notes{let after=new_notes.iter().find(|x|x.id==n.id);if after!=Some(n){out.push(NotePatch{track:track.clone(),id:n.id.clone(),before:Some(n.clone()),after:after.cloned()});}}
        for n in &new_notes{if !old_notes.iter().any(|o|o.id==n.id){out.push(NotePatch{track:track.clone(),id:n.id.clone(),before:None,after:Some(n.clone())});}}t.notes.extend(new_notes);
    }
    let before=project.extensions.get(KEY).cloned();let after=Some(serde_json::to_value(parts).map_err(|e|e.to_string())?);
    if before!=after{project.extensions.insert(KEY.into(),after.clone().expect("parts"));meta.push(MetaPatch::ProjectExtension{key:KEY.into(),before,after});}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandBatch, ProjectStore};

    #[test]
    fn empty_part_absorbs_notes_drawn_inside_it() {
        let project=Project::new_empty();let track=project.tracks[0].id.clone();
        let mut store=ProjectStore::new(project);
        let mut parts=Parts::load(&store.snapshot()).unwrap();
        assert_eq!(parts.next_name("片段"),"片段 1");
        let id=parts.create_empty(&store.snapshot(),&track,1920,1920,parts.next_name("片段")).unwrap();
        assert_eq!(parts.next_name("片段"),"片段 2");
        assert!(parts.clone().create_empty(&store.snapshot(),&track,3000,960,"x".into()).is_err(),"overlap rejected");
        store.apply(store.revision(),CommandBatch::single("新建片段",parts.command())).unwrap();
        let note=Note::new(2400,480,60);let note_id=note.id.clone();
        store.apply(store.revision(),CommandBatch::single("画音符",Command::InsertNotes{track:track.clone(),notes:vec![note]})).unwrap();
        let parts=Parts::load(&store.snapshot()).unwrap();let part=parts.instance(&id).unwrap();
        assert!(part.bindings.values().any(|n|n==&note_id));
        assert_eq!(parts.contents[&part.content_id].notes[0].start_tick,480);
    }
}
