//! Typed native plugin and mixer edits shared by headless clients.
//! Every plan is side-effect free and becomes one domain journal transaction.
use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use verdant_domain::{Command, CommandBatch, Project, ProjectStore, TrackId};
use verdant_domain::automation::{AutomationDocument, AutomationLane, AutomationOwner, AutomationTarget, AUTOMATION_KEY, MASTER_INSERTS_KEY};
use verdant_plugins::{InsertSpec, builtin_dsp as dsp, factory_presets as factory};
use crate::project_audio::{MasterBusSettings, MASTER_BUS_KEY, SIDECHAIN_KEY};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
pub enum PluginOwner { Track { track_id: TrackId }, Master, Instrument { track_id: TrackId } }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixerPatch {
    pub gain_db: Option<f64>,
    pub pan: Option<f64>,
    pub muted: Option<bool>,
    pub solo: Option<bool>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag="operation", rename_all="snake_case", deny_unknown_fields)]
pub enum StudioEdit {
    Batch { edits: Vec<StudioEdit> },
    Add { owner: PluginOwner, device: String, index: Option<usize>, preset_id: Option<String> },
    Remove { owner: PluginOwner, instance_id: String },
    Reorder { owner: PluginOwner, instance_ids: Vec<String> },
    Set { owner: PluginOwner, instance_id: String, values: BTreeMap<String, f64> },
    Reset { owner: PluginOwner, instance_id: String },
    Bypass { owner: PluginOwner, instance_id: String, bypass: bool },
    Sidechain { owner: PluginOwner, instance_id: String, source_track: Option<TrackId> },
    Preset { owner: PluginOwner, instance_id: String, preset_id: String },
    State { owner: PluginOwner, instance_id: String, device: String, values: BTreeMap<String, f64> },
    Mixer { owner: PluginOwner, settings: MixerPatch },
    Automation { lane: AutomationLane },
    RemoveAutomation { target: AutomationTarget },
}

#[derive(Clone, Debug, Serialize)]
pub struct StudioPlan { pub batch: CommandBatch, pub result: Value }

/// Reading never invents IDs or drops malformed records.
pub fn plugins(project: &Project, owner: &PluginOwner) -> Result<Vec<InsertSpec>, String> {
    let value = match owner {
        PluginOwner::Master => project.extensions.get(MASTER_INSERTS_KEY),
        PluginOwner::Track { track_id } => project.track(track_id).ok_or("unknown track")?.unknown.get("inserts"),
        PluginOwner::Instrument { track_id } => {
            let value = project.track(track_id).ok_or("unknown track")?.unknown.get("instrument");
            return value.map(|v| serde_json::from_value(v.clone()).map(|s|vec![s]).map_err(|e|format!("malformed instrument: {e}"))).unwrap_or(Ok(vec![]));
        }
    };
    let specs: Vec<InsertSpec> = value.map(|v| serde_json::from_value(v.clone()).map_err(|e|format!("malformed insert chain: {e}"))).unwrap_or(Ok(vec![]))?;
    let mut ids = BTreeSet::new();
    if specs.iter().any(|s| s.instance_id.is_empty() || !ids.insert(&s.instance_id)) {
        return Err("insert chain requires unique stable instance IDs; migrate the project first".into());
    }
    Ok(specs)
}

pub fn plugin(project:&Project, owner:&PluginOwner, id:&str) -> Result<InsertSpec,String> {
    plugins(project,owner)?.into_iter().find(|s|s.instance_id==id).ok_or_else(||"unknown plugin instance ID".into())
}

fn chain_command(owner:&PluginOwner, specs:&[InsertSpec]) -> Result<Command,String> {
    let values=specs.iter().map(serde_json::to_value).collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(match owner {
        PluginOwner::Master => Command::SetProjectExtension { key:MASTER_INSERTS_KEY.into(), value:(!values.is_empty()).then_some(json!(values)) },
        PluginOwner::Track { track_id } => Command::SetTrackInserts { track:track_id.clone(), inserts:values },
        PluginOwner::Instrument { track_id } => {
            if values.len()>1 { return Err("an instrument slot holds one device".into()); }
            Command::SetTrackInstrument { track:track_id.clone(), instrument:values.into_iter().next() }
        }
    })
}
fn automation_owner(owner:&PluginOwner)->AutomationOwner {
    match owner { PluginOwner::Master=>AutomationOwner::Master,
        PluginOwner::Track{track_id}|PluginOwner::Instrument{track_id}=>AutomationOwner::Track{track_id:track_id.clone()} }
}
fn native(spec:&InsertSpec)->Result<(),String>{
    if spec.format!="builtin" || factory::parameter_descriptors(&spec.id).is_none() {
        return Err("this parameter operation requires a native Verdant device; external opaque state is preserved".into());
    } Ok(())
}

/// Native instruments persist their audible state in params; effects use the
/// host state blob. Keep both GUI and headless clients on that same contract.
fn native_state(spec:&InsertSpec)->Result<String,String>{
    if dsp::is_builtin_instrument(&spec.id) {
        let params=spec.extra.get("params").cloned().unwrap_or(json!({}));
        let map=serde_json::from_value(params).map_err(|e|format!("invalid instrument params: {e}"))?;
        factory::encode_native_state(&map)
    }else{Ok(spec.state_b64.clone())}
}
fn replace_native_state(spec:&mut InsertSpec,state:String)->Result<(),String>{
    if dsp::is_builtin_instrument(&spec.id) {
        spec.extra.insert("params".into(),json!(factory::decode_native_state(&state)?));
        spec.state_b64.clear();
    }else{spec.state_b64=state;}
    Ok(())
}
pub fn native_parameters(spec:&InsertSpec)->Result<BTreeMap<String,f64>,String>{
    native(spec)?;
    let raw=factory::decode_native_state(&native_state(spec)?)?;
    let descriptors=factory::parameter_descriptors(&spec.id).ok_or("unknown native device")?;
    let values=raw.into_iter().filter(|(k,_)|descriptors.iter().any(|d|&d.id==k)).map(|(k,v)|v.as_f64().map(|v|(k.clone(),v)).ok_or_else(||format!("non-numeric native parameter {k}"))).collect::<Result<_,_>>()?;
    factory::complete_parameters(&spec.id,&values)
}

fn set_spec(project:&Project,owner:&PluginOwner,id:&str,f:impl FnOnce(&mut InsertSpec)->Result<(),String>)->Result<Command,String>{
    let mut specs=plugins(project,owner)?;
    let spec=specs.iter_mut().find(|s|s.instance_id==id).ok_or("unknown plugin instance ID")?;
    f(spec)?;
    chain_command(owner,&specs)
}
pub fn plan(project:&Project,edit:&StudioEdit)->Result<StudioPlan,String>{
    plan_at(project,edit,0)
}
fn plan_at(project:&Project,edit:&StudioEdit,depth:usize)->Result<StudioPlan,String>{
    if depth>8 {return Err("studio batch nesting exceeds 8".into());}
    let mut commands=Vec::new();
    let result=match edit{
        StudioEdit::Batch{edits}=>{
            if edits.is_empty()||edits.len()>1024{return Err("studio batch requires 1..1024 edits".into());}
            let mut preview=ProjectStore::new(project.clone()); let mut results=Vec::new();
            for edit in edits {
                let plan=plan_at(preview.project(),edit,depth+1)?;
                preview.apply(preview.revision(),plan.batch.clone()).map_err(|e|e.to_string())?;
                preview.validate_document().map_err(|e|e.to_string())?;
                commands.extend(plan.batch.commands); results.push(plan.result);
            } json!({"results":results})
        }
        StudioEdit::Add{owner,device,index,preset_id}=>{
            factory::default_parameters(device)?;
            let instrument=dsp::is_builtin_instrument(device);
            if instrument!=matches!(owner,PluginOwner::Instrument{..}){return Err("instrument devices require an instrument owner; effects require track/master".into());}
            let mut specs=plugins(project,owner)?;
            if instrument&&!specs.is_empty(){return Err("instrument slot is occupied; remove it explicitly before replacing it".into());}
            let index=index.unwrap_or(specs.len()); if index>specs.len(){return Err("insert index exceeds chain length".into());}
            let name=dsp::display_name(device).or_else(||dsp::instrument_name(device)).ok_or("unknown device")?;
            let mut spec=InsertSpec::new("builtin",device,name,"");
            let values=if let Some(id)=preset_id { matching_preset(id,device)? }else{factory::default_parameters(device)?};
            replace_native_state(&mut spec,factory::parameter_state(&values)?)?;
            if let Some(id)=preset_id{spec.extra.insert("factory_preset_id".into(),json!(id));}
            let result=json!({"owner":owner,"instance_id":spec.instance_id,"device":device,"index":index});
            specs.insert(index,spec);commands.push(chain_command(owner,&specs)?);result
        }
        StudioEdit::Remove{owner,instance_id}=>{
            let mut specs=plugins(project,owner)?;let len=specs.len();specs.retain(|s|&s.instance_id!=instance_id);
            if len==specs.len(){return Err("unknown plugin instance ID".into());}
            let mut automation=AutomationDocument::from_project(project).map_err(|e|e.to_string())?;
            automation.remove_plugin(&automation_owner(owner),instance_id);
            commands.push(Command::SetProjectExtension{key:AUTOMATION_KEY.into(),value:automation.extension_value().map_err(|e|e.to_string())?});
            commands.push(chain_command(owner,&specs)?);json!({"removed":instance_id})
        }
        StudioEdit::Reorder{owner,instance_ids}=>{
            let specs=plugins(project,owner)?;
            if instance_ids.len()!=specs.len() || instance_ids.iter().collect::<BTreeSet<_>>().len()!=specs.len() {
                return Err("reorder must contain every instance exactly once".into());
            }
            let reordered=instance_ids.iter().map(|id|specs.iter().find(|s|&s.instance_id==id).cloned().ok_or("unknown plugin instance ID")).collect::<Result<Vec<_>,_>>()?;
            commands.push(chain_command(owner,&reordered)?);json!({"instance_ids":instance_ids})
        }
        StudioEdit::Set{owner,instance_id,values}=>{
            if values.is_empty(){return Err("parameter update is empty".into());}
            commands.push(set_spec(project,owner,instance_id,|s|{native(s)?;let state=factory::patch_native_state(&s.id,&native_state(s)?,values)?;replace_native_state(s,state)?;s.extra.remove("factory_preset_id");Ok(())})?);
            json!({"instance_id":instance_id,"parameters":values})
        }
        StudioEdit::Reset{owner,instance_id}=>{
            commands.push(set_spec(project,owner,instance_id,|s|{native(s)?;let state=factory::parameter_state(&factory::default_parameters(&s.id)?)?;replace_native_state(s,state)?;s.extra.remove("factory_preset_id");Ok(())})?);
            json!({"instance_id":instance_id,"reset":true})
        }
        StudioEdit::Bypass{owner,instance_id,bypass}=>{
            commands.push(set_spec(project,owner,instance_id,|s|{s.bypass=*bypass;Ok(())})?);json!({"instance_id":instance_id,"bypass":bypass})
        }
        StudioEdit::Sidechain{owner,instance_id,source_track}=>{
            if matches!(owner,PluginOwner::Instrument{..}){return Err("instruments do not accept audio sidechains".into());}
            if let Some(id)=source_track{
                project.track(id).ok_or("unknown sidechain source track")?;
                if matches!(owner,PluginOwner::Track{track_id}if track_id==id){return Err("a track cannot sidechain itself".into());}
            }
            commands.push(set_spec(project,owner,instance_id,|s|{
                native(s)?;if !dsp::takes_sidechain(&s.id){return Err("device does not accept sidechain".into());}
                if let Some(id)=source_track{s.extra.insert(SIDECHAIN_KEY.into(),json!(id));}else{s.extra.remove(SIDECHAIN_KEY);}
                Ok(())
            })?);json!({"instance_id":instance_id,"source_track":source_track})
        }
        StudioEdit::Preset{owner,instance_id,preset_id}=>{
            commands.push(set_spec(project,owner,instance_id,|s|{native(s)?;let state=factory::parameter_state(&matching_preset(preset_id,&s.id)?)?;replace_native_state(s,state)?;s.extra.insert("factory_preset_id".into(),json!(preset_id));Ok(())})?);
            json!({"instance_id":instance_id,"preset_id":preset_id})
        }
        StudioEdit::State{owner,instance_id,device,values}=>{
            commands.push(set_spec(project,owner,instance_id,|s|{
                native(s)?;if &s.id!=device{return Err("preset device does not match destination".into());}
                let state=factory::parameter_state(&factory::complete_parameters(device,values)?)?;replace_native_state(s,state)?;s.extra.remove("factory_preset_id");Ok(())
            })?);json!({"instance_id":instance_id,"device":device})
        }
        StudioEdit::Mixer{owner,settings}=>{
            if settings.gain_db.is_some_and(|v|!v.is_finite()||!(-96.0..=6.0).contains(&v)){return Err("gain_db must be within -96..6".into());}
            if settings.pan.is_some_and(|v|!v.is_finite()||!(-1.0..=1.0).contains(&v)){return Err("pan must be within -1..1".into());}
            if settings.name.as_ref().is_some_and(|n|n.trim().is_empty()||n.len()>4096){return Err("name must contain 1..4096 bytes".into());}
            match owner{
                PluginOwner::Master=>{
                    if settings.solo.is_some()||settings.name.is_some(){return Err("master does not accept solo/name".into());}
                    let mut bus=MasterBusSettings::from_project(project);
                    if let Some(v)=settings.gain_db{bus.gain_millidecibels=(v*1000.).round()as i32;}
                    if let Some(v)=settings.pan{bus.pan_permille=(v*1000.).round()as i32;}
                    if let Some(v)=settings.muted{bus.muted=v;}
                    commands.push(Command::SetProjectExtension{key:MASTER_BUS_KEY.into(),value:bus.extension_value()});
                }
                PluginOwner::Track{track_id}=>{
                    let mut header=project.track(track_id).ok_or("unknown mixer track")?.header();
                    if let Some(v)=settings.gain_db{header.gain_millidecibels=(v*1000.).round()as i32;}
                    if let Some(v)=settings.pan{header.pan_permille=(v*1000.).round()as i32;}
                    if let Some(v)=settings.muted{header.muted=v;}
                    if let Some(v)=settings.solo{header.solo=v;}
                    if let Some(v)=&settings.name{header.name=v.clone();}
                    commands.push(Command::SetTrackHeader{track:track_id.clone(),header});
                }
                PluginOwner::Instrument{..}=>return Err("mixer settings address track or master".into()),
            } json!({"owner":owner,"settings":settings})
        }
        StudioEdit::Automation{lane}=>{
            validate_lane(project,lane)?;
            let mut document=AutomationDocument::from_project(project).map_err(|e|e.to_string())?;
            document.set_lane(lane.clone());document.validate(project).map_err(|e|e.to_string())?;
            commands.push(Command::SetProjectExtension{key:AUTOMATION_KEY.into(),value:document.extension_value().map_err(|e|e.to_string())?});
            json!({"target":lane.target,"points":lane.points.len()})
        }
        StudioEdit::RemoveAutomation{target}=>{
            let mut document=AutomationDocument::from_project(project).map_err(|e|e.to_string())?;
            if !document.remove_lane(target){return Err("automation lane does not exist".into());}
            commands.push(Command::SetProjectExtension{key:AUTOMATION_KEY.into(),value:document.extension_value().map_err(|e|e.to_string())?});
            json!({"removed":target})
        }
    };
    let batch=CommandBatch{label:"Edit studio".into(),commands};
    // Reject invalid cross-document references and routing before publication.
    let mut preview=ProjectStore::new(project.clone());
    preview.apply(preview.revision(),batch.clone()).map_err(|e|e.to_string())?;
    preview.validate_document().map_err(|e|e.to_string())?;
    Ok(StudioPlan{batch,result})
}
fn matching_preset(id:&str,device:&str)->Result<BTreeMap<String,f64>,String>{
    let preset=factory::factory_preset(id).ok_or("unknown factory preset ID")?;
    if preset.device!=device{return Err("factory preset device differs from destination".into());}Ok(preset.parameters)
}
fn validate_lane(project:&Project,lane:&AutomationLane)->Result<(),String>{
    if let AutomationTarget::PluginParameter{owner,instance_id,parameter_id}=&lane.target {
        let owner=match owner{AutomationOwner::Master=>PluginOwner::Master,AutomationOwner::Track{track_id}=>PluginOwner::Track{track_id:track_id.clone()}};
        let spec=plugin(project,&owner,instance_id)?;native(&spec)?;
        let descriptors=factory::parameter_descriptors(&spec.id).ok_or("unknown device")?;
        let d=descriptors.iter().find(|d|&d.id==parameter_id).ok_or("unknown automation parameter")?;
        if !factory::parameter_is_automatable(&spec.id,parameter_id){return Err("control-only parameter cannot be automated".into());}
        lane.validate_parameter_steps(d.step_count).map_err(|e|e.to_string())?;
        for p in &lane.points{factory::complete_parameters(&spec.id,&BTreeMap::from([(parameter_id.clone(),p.value)]))?;}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_domain::automation::{AutomationPoint, Interpolation};

    fn fixture()->(Project,PluginOwner,String){
        let project=Project::new_empty();
        let owner=PluginOwner::Track{track_id:project.tracks[0].id.clone()};
        let plan=plan(&project,&StudioEdit::Add{owner:owner.clone(),device:dsp::EQ_ID.into(),index:None,preset_id:None}).unwrap();
        let id=plan.result["instance_id"].as_str().unwrap().to_owned();
        let mut store=ProjectStore::new(project);store.apply(store.revision(),plan.batch).unwrap();
        (store.project().clone(),owner,id)
    }
    #[test]
    fn mixed_invalid_transaction_preserves_all_state_and_one_valid_batch_undoes(){
        let(project,owner,id)=fixture();
        let set=StudioEdit::Set{owner:owner.clone(),instance_id:id.clone(),values:BTreeMap::from([("output".into(),2.)])};
        let bad=StudioEdit::Mixer{owner:owner.clone(),settings:MixerPatch{pan:Some(2.),..Default::default()}};
        let original=project.clone();
        assert!(plan(&project,&StudioEdit::Batch{edits:vec![set.clone(),bad]}).is_err());
        assert_eq!(project,original);
        let good=StudioEdit::Mixer{owner:owner.clone(),settings:MixerPatch{gain_db:Some(-4.5),pan:Some(-0.2),..Default::default()}};
        let plan=plan(&project,&StudioEdit::Batch{edits:vec![set,good]}).unwrap();
        let mut store=ProjectStore::new(project.clone());
        store.apply(store.revision(),plan.batch).unwrap();
        assert_eq!(store.undo_depth(),1);
        assert_eq!(store.project().tracks[0].gain_millidecibels,-4500);
        let spec=plugin(store.project(),&owner,&id).unwrap();
        assert_eq!(factory::decode_native_state(&spec.state_b64).unwrap()["output"],json!(2.));
        store.undo().unwrap();assert_eq!(store.project().tracks,project.tracks);
    }
    #[test]
    fn preset_full_state_replaces_hidden_values_and_refuses_wrong_device(){
        let(project,owner,id)=fixture();
        let preset=factory::factory_presets(dsp::EQ_ID).remove(0);
        let set=StudioEdit::Set{owner:owner.clone(),instance_id:id.clone(),values:BTreeMap::from([("b24_gain".into(),12.),("b24_on".into(),1.)])};
        let load=StudioEdit::Preset{owner:owner.clone(),instance_id:id.clone(),preset_id:preset.id};
        let plan=plan(&project,&StudioEdit::Batch{edits:vec![set,load]}).unwrap();
        let mut store=ProjectStore::new(project);store.apply(store.revision(),plan.batch).unwrap();
        let state=factory::decode_native_state(&plugin(store.project(),&owner,&id).unwrap().state_b64).unwrap();
        assert_eq!(state["b24_gain"],json!(0.));assert_eq!(state["b24_on"],json!(0.));
        assert!(plan_at(store.project(),&StudioEdit::Preset{owner,instance_id:id,preset_id:factory::factory_presets(dsp::COMPRESSOR_ID)[0].id.clone()},0).is_err());
    }
    #[test]
    fn automation_uses_actual_parameter_grids_and_is_removed_with_instance(){
        let(project,owner,id)=fixture();
        let target=AutomationTarget::PluginParameter{owner:automation_owner(&owner),instance_id:id.clone(),parameter_id:"b1_on".into()};
        let mut lane=AutomationLane::new(target);lane.points=vec![AutomationPoint::new(0,0.),AutomationPoint::new(480,1.)];
        assert!(plan(&project,&StudioEdit::Automation{lane:lane.clone()}).is_err());
        lane.points[0].interpolation=Interpolation::Step;
        let valid=plan(&project,&StudioEdit::Automation{lane}).unwrap();
        let mut store=ProjectStore::new(project);store.apply(store.revision(),valid.batch).unwrap();
        let remove=plan(store.project(),&StudioEdit::Remove{owner,instance_id:id}).unwrap();
        store.apply(store.revision(),remove.batch).unwrap();
        assert!(AutomationDocument::from_project(store.project()).unwrap().lanes.is_empty());
        store.undo().unwrap();
        assert_eq!(AutomationDocument::from_project(store.project()).unwrap().lanes.len(),1);
    }

    #[test]
    fn instrument_controls_change_the_pcm_and_preserve_legacy_hidden_fields(){
        let mut project=Project::new_empty();
        project.tracks[0].notes.push(verdant_domain::Note::new(0,480,60));
        let owner=PluginOwner::Instrument{track_id:project.tracks[0].id.clone()};
        let added=plan(&project,&StudioEdit::Add{owner:owner.clone(),device:dsp::ONE_SYNTH_ID.into(),index:None,preset_id:None}).unwrap();
        let id=added.result["instance_id"].as_str().unwrap().to_owned();
        let mut store=ProjectStore::new(project);store.apply(store.revision(),added.batch).unwrap();
        let tempo=verdant_domain::TempoMap::new(&store.project().tempo_map,store.project().ppq).unwrap();
        let before=crate::project_audio::render_instrument(&store.project().tracks[0],0,&tempo,48000).unwrap().unwrap();
        let edited=plan(store.project(),&StudioEdit::Set{owner:owner.clone(),instance_id:id.clone(),values:BTreeMap::from([("gain".into(),-20.)])}).unwrap();
        store.apply(store.revision(),edited.batch).unwrap();
        let spec=plugin(store.project(),&owner,&id).unwrap();
        assert_eq!(spec.extra["params"]["gain"],json!(-20.));
        assert!(spec.state_b64.is_empty());
        let after=crate::project_audio::render_instrument(&store.project().tracks[0],0,&tempo,48000).unwrap().unwrap();
        let power=|samples:&[f32]|samples.iter().map(|v|f64::from(*v).powi(2)).sum::<f64>();
        let ratio=power(&after.samples)/power(&before.samples);
        assert!((ratio-10f64.powf(-12./10.)).abs()<1e-5,"{ratio}");
        // GUI historically writes every Synth field into One's params.
        let mut spec=spec;spec.extra["params"]["osc2_mix"]=json!(0.);
        assert_eq!(native_parameters(&spec).unwrap().len(),187);
        let state=factory::patch_native_state(&spec.id,&native_state(&spec).unwrap(),&BTreeMap::from([("gain".into(),-18.)])).unwrap();
        replace_native_state(&mut spec,state).unwrap();
        assert_eq!(spec.extra["params"]["osc2_mix"],json!(0.));
    }

    #[test]
    fn malformed_chain_is_never_silently_shortened_and_instruments_use_their_own_slot(){
        let(mut project,owner,_)=fixture();
        project.tracks[0].unknown.get_mut("inserts").unwrap().as_array_mut().unwrap().push(json!(42));
        assert!(plugins(&project,&owner).is_err());
        let project=Project::new_empty();let owner=PluginOwner::Instrument{track_id:project.tracks[0].id.clone()};
        let plan=plan(&project,&StudioEdit::Add{owner:owner.clone(),device:dsp::ONE_SYNTH_ID.into(),index:None,preset_id:None}).unwrap();
        let mut store=ProjectStore::new(project);store.apply(store.revision(),plan.batch).unwrap();
        assert_eq!(plugins(store.project(),&owner).unwrap().len(),1);
        assert!(plan_at(store.project(),&StudioEdit::Add{owner,device:dsp::EQ_ID.into(),index:None,preset_id:None},0).is_err());
    }
}
