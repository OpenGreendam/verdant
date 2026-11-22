//! Native studio operations shared by the command line and NDJSON worker.
use std::{collections::BTreeMap, io::Write, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use verdant_domain::{CommandBatch, Project, ProjectStore};
use verdant_plugins::{builtin_dsp as dsp, factory_presets as factory};
use verdant_service::{studio::{self, PluginOwner, StudioEdit}, audio_edit::{self, AudioEdit}};
use crate::{Outcome, CliError, invalid, internal, project::{Params, EditParams}};

#[path="studio_analysis.rs"] mod native_analysis;

pub const FAMILIES:&[&str]=&["plugin","mixer","preset","clip","analysis"];
pub const OPERATIONS:&[&str]=&[
    "plugin.list","plugin.describe","plugin.inspect","plugin.add","plugin.remove","plugin.reorder",
    "plugin.set","plugin.reset","plugin.bypass","plugin.sidechain","plugin.edit",
    "mixer.inspect","mixer.set","mixer.apply","mixer.automate","mixer.remove-automation",
    "preset.list","preset.show","preset.apply","preset.import","preset.export","preset.inspect-ffp","preset.import-ffp",
    "clip.inspect","clip.edit","clip.split","clip.split-range","clip.trim","clip.trim-to-range","clip.slip",
    "clip.move","clip.duplicate","clip.delete","clip.delete-range","clip.copy","clip.copy-range","clip.paste",
    "clip.set-properties","clip.join","clip.crossfade","clip.split-silence","clip.split-zero-crossing",
    "clip.normalize","clip.bounce","analysis.audio","analysis.eq-fit","analysis.eq-match","analysis.reverb-ir",
];

pub fn dispatch(op:&str,params:Value,_progress:&mut dyn FnMut(Value))->Option<Outcome>{
    OPERATIONS.contains(&op).then(||run(op,params))
}
fn run(op:&str,params:Value)->Outcome{
    match op{
        "plugin.list"=>{Params::new(params)?.finish()?;Ok(json!({"devices":devices()}))}
        "plugin.describe"=>describe(Params::new(params)?),
        "plugin.inspect"=>inspect(Params::new(params)?,false),
        "mixer.inspect"=>inspect(Params::new(params)?,true),
        "preset.list"=>preset_list(Params::new(params)?),
        "preset.show"=>{let mut p=Params::new(params)?;let id:String=p.required("id")?;p.finish()?;
            Ok(json!(factory::factory_preset(&id).ok_or_else(||invalid("unknown factory preset ID"))?))}
        "preset.export"=>preset_export(Params::new(params)?),
        "preset.inspect-ffp"=>inspect_ffp(Params::new(params)?),
        "analysis.audio"=>analyze(Params::new(params)?),
        "analysis.eq-fit"|"analysis.eq-match"|"analysis.reverb-ir"=>native_analysis::run(op,Params::new(params)?),
        "clip.inspect"=>crate::integrations::dispatch("media.inspect",params).expect("existing media.inspect dispatch"),
        _=>mutate(op,params),
    }
}
fn devices()->Vec<Value>{
    dsp::EFFECT_IDS.iter().copied().chain([dsp::SYNTH_ID,dsp::ONE_SYNTH_ID]).map(|id|json!({
        "id":id,"name":dsp::display_name(id).or_else(||dsp::instrument_name(id)),
        "kind":if dsp::is_builtin_instrument(id){"instrument"}else{"effect"},"native_editor":true,
        "parameter_count":factory::parameter_descriptors(id).unwrap().len(),
        "factory_preset_count":factory::factory_presets(id).len()})).collect()
}
fn describe(mut p:Params)->Outcome{
    let device:String=p.required("device")?;p.finish()?;
    let descriptors=factory::parameter_descriptors(&device).ok_or_else(||invalid("unknown native device"))?;
    Ok(json!({"device":device,"parameters":descriptors.into_iter().map(|d|{
        let mut v=json!(d);v["automatable"]=json!(factory::parameter_is_automatable(&device,&d.id));v
    }).collect::<Vec<_>>(),"state_units":"native","factory_presets":factory::factory_presets(&device).len()}))
}
fn read_selection(p:&mut Params)->Result<Project,CliError>{
    let path:PathBuf=p.required("path")?;
    let recovery:Option<String>=p.optional("recovery")?;let creator:Option<String>=p.optional("allowed_creator")?;
    crate::project::read_snapshot(&path,recovery.as_deref(),creator.as_deref())
}
fn inspect(mut p:Params,mixer:bool)->Outcome{
    let project=read_selection(&mut p)?;
    if mixer{
        p.finish()?;
        return Ok(json!({"project_id":project.project_id,"revision":project.revision,
            "tracks":project.tracks.iter().map(|t|Ok(json!({"track_id":t.id,"header":t.header(),
                "inserts":studio::plugins(&project,&PluginOwner::Track{track_id:t.id.clone()}).map_err(invalid)?,
                "instrument":studio::plugins(&project,&PluginOwner::Instrument{track_id:t.id.clone()}).map_err(invalid)?}))).collect::<Result<Vec<Value>,CliError>>()?,
            "master":verdant_service::project_audio::MasterBusSettings::from_project(&project),
            "master_inserts":studio::plugins(&project,&PluginOwner::Master).map_err(invalid)?,
            "automation":verdant_domain::automation::AutomationDocument::from_project(&project).map_err(invalid)?}));
    }
    let owner:PluginOwner=p.required("owner")?;let id:Option<String>=p.optional("instance_id")?;p.finish()?;
    let specs=studio::plugins(&project,&owner).map_err(invalid)?;
    if id.as_ref().is_some_and(|id|!specs.iter().any(|s|&s.instance_id==id)){return Err(invalid("unknown plugin instance ID"));}
    let specs=specs.into_iter().filter(|s|id.as_ref().is_none_or(|id|&s.instance_id==id)).map(|spec|{
        let parameters=if spec.format=="builtin"&&factory::parameter_descriptors(&spec.id).is_some(){
            Some(studio::native_parameters(&spec).map_err(invalid)?)
        }else{None};
        Ok(json!({"spec":spec,"parameters":parameters}))
    }).collect::<Result<Vec<Value>,CliError>>()?;
    Ok(json!({"project_id":project.project_id,"revision":project.revision,"owner":owner,"plugins":specs}))
}

fn preset_list(mut p:Params)->Outcome{
    let device:Option<String>=p.optional("device")?;let category:Option<String>=p.optional("category")?;
    let query:Option<String>=p.optional("query")?;let tags:Vec<String>=p.optional("tags")?.unwrap_or_default();p.finish()?;
    if device.as_ref().is_some_and(|d|factory::parameter_descriptors(d).is_none()){return Err(invalid("unknown device"));}
    let query=query.unwrap_or_default().to_lowercase();
    let presets=factory::all_factory_presets().iter().filter(|p|device.as_ref().is_none_or(|d|d==&p.device)
        &&category.as_ref().is_none_or(|c|c==&p.category)&&tags.iter().all(|t|p.tags.contains(t))
        &&format!("{} {} {} {}",p.name,p.category,p.description,p.tags.join(" ")).to_lowercase().contains(&query))
        .map(|p|json!({"id":p.id,"device":p.device,"name":p.name,"category":p.category,"description":p.description,"author":p.author,"tags":p.tags})).collect::<Vec<_>>();
    Ok(json!({"schema":factory::LIBRARY_SCHEMA,"version":factory::LIBRARY_VERSION,"count":presets.len(),"presets":presets}))
}

#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile{
    format:String,version:u32,device:String,name:String,tags:Vec<String>,parameters:BTreeMap<String,f64>,
    #[serde(default)]favorite:bool,#[serde(default)]category:String,#[serde(default)]author:String,
    #[serde(default)]description:String,#[serde(default)]origin_factory_id:Option<String>,
}
fn validate_preset(p:&PresetFile)->Result<(),CliError>{
    if p.format!="verdant-device-preset"||p.version!=1{return Err(invalid("unsupported native preset format/version"));}
    if p.name.trim().is_empty()||p.name.chars().count()>120||p.tags.len()>16
        ||p.tags.iter().any(|t|t.trim().is_empty()||t.chars().count()>40)
        ||p.category.chars().count()>80||p.author.chars().count()>120||p.description.chars().count()>2000
        ||p.origin_factory_id.as_ref().is_some_and(|s|s.chars().count()>160){return Err(invalid("preset metadata exceeds native limits"));}
    factory::complete_parameters(&p.device,&p.parameters).map_err(invalid)?;Ok(())
}
fn preset_export(mut p:Params)->Outcome{
    let output:PathBuf=p.required("output")?;let id:Option<String>=p.optional("id")?;
    let name:Option<String>=p.optional("name")?;let dry_run:bool=p.optional("dry_run")?.unwrap_or(false);
    let preset=if let Some(id)=id{
        let f=factory::factory_preset(&id).ok_or_else(||invalid("unknown factory preset"))?;
        PresetFile{format:"verdant-device-preset".into(),version:1,device:f.device,name:name.unwrap_or(f.name),tags:f.tags,parameters:f.parameters,
            favorite:false,category:f.category,author:f.author,description:f.description,origin_factory_id:Some(f.id)}
    }else{
        let project=read_selection(&mut p)?;let owner:PluginOwner=p.required("owner")?;let id:String=p.required("instance_id")?;
        let s=studio::plugin(&project,&owner,&id).map_err(invalid)?;
        if s.format!="builtin"{return Err(invalid("native presets require a native device"));}
        let values=studio::native_parameters(&s).map_err(invalid)?;
        PresetFile{format:"verdant-device-preset".into(),version:1,device:s.id.clone(),name:name.unwrap_or(s.name),tags:vec![],
            parameters:factory::complete_parameters(&s.id,&values).map_err(invalid)?,favorite:false,category:"User".into(),author:"".into(),description:"".into(),origin_factory_id:None}
    };
    p.finish()?;validate_preset(&preset)?;crate::project::require_fresh(&output)?;
    let bytes=serde_json::to_vec_pretty(&preset).map_err(internal)?;
    if !dry_run {write_fresh(&output,&bytes)?;}
    Ok(json!({"output":output,"dry_run":dry_run,"device":preset.device,"parameters":preset.parameters.len(),"bytes":bytes.len()}))
}
fn write_fresh(path:&Path,bytes:&[u8])->Result<(),CliError>{
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut staged=tempfile::NamedTempFile::new_in(parent).map_err(|e|CliError::new("IO_ERROR",e.to_string()))?;
    staged.write_all(bytes).and_then(|()|staged.as_file().sync_all()).map_err(|e|CliError::new("IO_ERROR",e.to_string()))?;
    staged.persist_noclobber(path).map_err(|e|CliError::new(if e.error.kind()==std::io::ErrorKind::AlreadyExists{"OUTPUT_EXISTS"}else{"IO_ERROR"},e.error.to_string()))?;
    Ok(())
}

fn ffp_report(source:&Path,device:&str)->Result<(Value,BTreeMap<String,f64>),CliError>{
    let metadata=std::fs::metadata(source).map_err(invalid)?;
    if !metadata.is_file()||metadata.len()>1_048_576{return Err(invalid("FFP input requires a regular file of at most 1 MiB"));}
    let bytes=std::fs::read(source).map_err(invalid)?;
    let report=verdant_plugins::preset_interop::inspect_fabfilter_preset(&bytes,device).map_err(invalid)?;
    let value=json!({"product":report.product,"format_version":report.format_version,"complete":report.is_complete(),
        "warnings":report.warnings,"unsupported":report.unsupported,"values":report.values});
    Ok((value,report.values.into_iter().collect()))
}
fn inspect_ffp(mut p:Params)->Outcome{
    let source:PathBuf=p.required("source")?;let device:String=p.required("device")?;p.finish()?;
    ffp_report(&source,&device).map(|(report,_)|report)
}

fn mutate(op:&str,value:Value)->Outcome{
    let mut p=Params::new(value)?;let edit=EditParams::take(&mut p)?;
    let mut roots:Vec<PathBuf>=vec![];let mut paths:Vec<PathBuf>=vec![];
    let mut ffp=Value::Null;
    let audio=op.starts_with("clip.");
    let operation=if audio{
        roots=p.optional("asset_roots")?.unwrap_or_default();paths=p.optional("asset_paths")?.unwrap_or_default();
        if op=="clip.edit"{p.required::<Value>("edit")?}else{collect_operation(&mut p,op,clip_fields(op))?}
    }else if op=="plugin.edit"{p.required::<Value>("edit")?}
    else if op=="mixer.apply"{json!({"operation":"batch","edits":p.required::<Vec<Value>>("edits")?})}
    else if op=="preset.import"{
        let source:PathBuf=p.required("source")?;let owner:PluginOwner=p.required("owner")?;let instance_id:String=p.required("instance_id")?;
        let preset:PresetFile=serde_json::from_value(crate::read_json_file(source.to_str().ok_or_else(||invalid("preset path is not Unicode"))?)?).map_err(invalid)?;validate_preset(&preset)?;
        json!({"operation":"state","owner":owner,"instance_id":instance_id,"device":preset.device,"values":preset.parameters})
    }else if op=="preset.import-ffp"{
        let source:PathBuf=p.required("source")?;let device:String=p.required("device")?;
        let owner:PluginOwner=p.required("owner")?;let instance_id:String=p.required("instance_id")?;
        let loss_policy:String=p.optional("loss_policy")?.unwrap_or_else(||"strict".into());
        if loss_policy!="strict"&&loss_policy!="allow"{return Err(invalid("loss_policy must be strict or allow"));}
        let(report,values)=ffp_report(&source,&device)?;
        if values.is_empty()||(!report["complete"].as_bool().unwrap_or(false)&&loss_policy!="allow"){
            return Err(CliError::new("UNSUPPORTED","FFP translation is incomplete; inspect the report before explicitly allowing partial import").with_details(report));
        }
        ffp=report;json!({"operation":"state","owner":owner,"instance_id":instance_id,"device":device,"values":values})
    }else{collect_operation(&mut p,op,studio_fields(op))?};
    p.finish()?;
    let input=if audio{Operation::Audio(serde_json::from_value(operation).map_err(invalid)?)}else{Operation::Studio(serde_json::from_value(operation).map_err(invalid)?)};
    let mut policy=verdant_media::AssetPolicy::new(roots).map_err(crate::integrations::media_error)?.with_project_path(&edit.path).map_err(crate::integrations::media_error)?;
    for path in paths{policy.allow_file(&path).map_err(crate::integrations::media_error)?;}
    let build=|project:&Project|->Result<Plan,CliError>{
        match &input{
            Operation::Audio(op)=>{let plan=audio_edit::plan_audio_edit_with_media(project,op,policy.clone()).map_err(invalid)?;
                Ok(Plan{batch:plan.batch.clone(),result:json!({"selection":plan.selection,"clipboard":plan.clipboard,"pending_assets":plan.pending_assets}),assets:Some(plan)})}
            Operation::Studio(op)=>{let plan=studio::plan(project,op).map_err(invalid)?;Ok(Plan{batch:plan.batch,result:plan.result,assets:None})}
        }
    };
    if edit.dry_run{
        let(mut preview,_)=edit.read()?;let plan=build(preview.project())?;
        let commit=preview.apply(preview.revision(),plan.batch).map_err(invalid)?;
        preview.validate_document().map_err(invalid)?;
        return Ok(json!({"path":edit.path,"dry_run":true,"state":crate::project::store_json(&preview),"commit":commit.as_ref().map(|c|json!({"revision":c.revision.get(),"label":c.label})),"result":plan.result,"translation":ffp}));
    }
    let(mut document,report)=edit.open()?;let plan=build(document.store().project())?;
    let mut preview=ProjectStore::new(document.store().project().clone());
    preview.apply(preview.revision(),plan.batch.clone()).map_err(invalid)?;preview.validate_document().map_err(invalid)?;
    let created=if let Some(assets)=&plan.assets{
        let mut staged=audio_edit::stage_audio_assets(assets).map_err(|e|CliError::new("IO_ERROR",e))?;
        match staged.publish(){
            Ok(paths)=>paths,
            Err(error)=>return Err(CliError::new("IO_ERROR",error).with_details(json!({
                "created_assets":staged.published_paths(),"project_commit":"not_started",
                "asset_publication":"incomplete","recovery":"The project is unchanged. Inspect the listed completed audio files before choosing fresh output paths for a retry."
            }))),
        }
    }else{vec![]};
    let commit=match document.commit(document.revision(),plan.batch){
        Ok(commit)=>commit,
        Err(error)=>return Err(publication_error(error,&created,document.revision().get(),false)),
    };
    if commit.is_some()||report.recovered_revision>report.saved_revision{
        if let Err(error)=document.save(){return Err(publication_error(error,&created,document.revision().get(),true));}
    }
    Ok(json!({"path":edit.path,"dry_run":false,"state":crate::project::store_json(document.store()),
        "commit":commit.as_ref().map(|c|json!({"revision":c.revision.get(),"label":c.label})),"result":plan.result,"created_assets":created,"translation":ffp}))
}
fn publication_error(error:verdant_domain::PersistenceError,created:&[PathBuf],revision:u64,committed:bool)->CliError{
    let mut failure=crate::project::persistence_error(error);
    if !created.is_empty(){
        failure=failure.with_details(json!({"created_assets":created,"revision":revision,
            "project_commit":if committed{"confirmed_in_journal"}else{"not_confirmed"},
            "asset_publication":"retained",
            "recovery":"Inspect the saved and recovered project revisions before retrying. Published audio is retained because a journal record may refer to it; do not overwrite or delete it while resolving recovery."}));
    }
    failure
}
enum Operation{Audio(AudioEdit),Studio(StudioEdit)}
struct Plan{batch:CommandBatch,result:Value,assets:Option<audio_edit::AudioEditPlan>}
fn collect_operation(p:&mut Params,op:&str,fields:&[&str])->Result<Value,CliError>{
    let operation=match op{
        "preset.apply"=>"preset","mixer.set"=>"mixer","mixer.automate"=>"automation","mixer.remove-automation"=>"remove_automation",
        _=>op.split_once('.').unwrap().1,
    }.replace('-', "_");
    let mut out=serde_json::Map::from_iter([("operation".into(),json!(operation))]);
    for &field in fields{if let Some(v)=p.optional::<Value>(field)?{out.insert(if op=="clip.bounce"&&field=="output"{"path".into()}else{field.into()},v);}}
    Ok(Value::Object(out))
}
fn studio_fields(op:&str)->&'static[&'static str]{
    match op{
        "plugin.add"=>&["owner","device","index","preset_id"],"plugin.remove"|"plugin.reset"=>&["owner","instance_id"],
        "plugin.reorder"=>&["owner","instance_ids"],"plugin.set"=>&["owner","instance_id","values"],
        "plugin.bypass"=>&["owner","instance_id","bypass"],"plugin.sidechain"=>&["owner","instance_id","source_track"],
        "preset.apply"=>&["owner","instance_id","preset_id"],"mixer.set"=>&["owner","settings"],
        "mixer.automate"=>&["lane"],"mixer.remove-automation"=>&["target"],_=>&[],
    }
}
fn clip_fields(op:&str)->&'static[&'static str]{
    match op{
        "clip.split"=>&["clips","tick"],"clip.split-range"|"clip.copy-range"=>&["tracks","start_tick","end_tick"],
        "clip.trim"=>&["clip","source_offset_frames","duration_frames","start_tick"],"clip.trim-to-range"=>&["clips","start_tick","end_tick"],
        "clip.slip"=>&["clips","delta_frames"],"clip.move"=>&["clips","delta_ticks","track_offset"],
        "clip.duplicate"=>&["clips","at_tick","target_track"],"clip.delete"=>&["clips","ripple"],
        "clip.delete-range"=>&["tracks","start_tick","end_tick","ripple"],"clip.copy"|"clip.join"=>&["clips"],
        "clip.paste"=>&["clipboard","at_tick","target_track"],"clip.set-properties"=>&["clips","properties"],
        "clip.crossfade"=>&["left","right","shape"],"clip.split-silence"=>&["clips","threshold_db","min_silence_ms"],
        "clip.split-zero-crossing"=>&["clip","tick","window_ms"],"clip.normalize"=>&["clips","target_db","independent"],
        "clip.bounce"=>&["clips","output","sample_rate","replace"],_=>&[],
    }
}
fn analyze(mut p:Params)->Outcome{
    let source:PathBuf=p.required("source")?;p.finish()?;
    let policy=verdant_media::AssetPolicy::for_file(&source).map_err(crate::integrations::media_error)?;
    let imported=verdant_media::MediaImporter::new(policy,Default::default()).import_file(&source).map_err(crate::integrations::media_error)?;
    let audio=imported.audio;let count=usize::from(audio.channels);
    let channels=(0..count).map(|ch|{
        let mut peak=0f64;let mut sum=0f64;let mut dc=0f64;let mut clipped=0u64;
        for frame in audio.samples.chunks_exact(count){let x=f64::from(frame[ch]);peak=peak.max(x.abs());sum+=x*x;dc+=x;if x.abs()>=1.{clipped+=1;}}
        let n=audio.frames()as f64;let rms=(sum/n).sqrt();
        json!({"channel":ch,"sample_peak":peak,"sample_peak_dbfs":if peak>0.{Some(20.*peak.log10())}else{None},
            "rms":rms,"rms_dbfs":if rms>0.{Some(20.*rms.log10())}else{None},"dc_offset":dc/n,"clipped_samples":clipped})
    }).collect::<Vec<_>>();
    Ok(json!({"schema":"verdant-audio-analysis/1","source":source,"sha256":audio.sha256,"sample_rate":audio.sample_rate,
        "frames":audio.frames(),"duration_seconds":audio.duration_seconds(),"channel_count":count,"channels":channels,"nonfinite_samples":0}))
}

pub fn positionals(op:&str)->Option<&'static[&'static str]>{
    OPERATIONS.contains(&op).then_some(match op{
        "plugin.list"|"preset.list"=>&[],"plugin.describe"=>&["device"],"preset.show"=>&["id"],
        "preset.inspect-ffp"|"analysis.audio"|"analysis.reverb-ir"=>&["source"],"analysis.eq-fit"=>&[],"analysis.eq-match"=>&["source","reference"],"preset.export"=>&["output"],_=>&["path"],
    })
}
pub fn schema(op:&str)->Option<Value>{
    if !OPERATIONS.contains(&op){return None;}
    let readonly=op.starts_with("analysis.")||matches!(op,"plugin.list"|"plugin.describe"|"plugin.inspect"|"mixer.inspect"|"preset.list"|"preset.show"|"preset.export"|"preset.inspect-ffp"|"clip.inspect"|"analysis.audio");
    let required:&[&str]=match op{
        "plugin.list"|"preset.list"=>&[],"plugin.describe"=>&["device"],"preset.show"=>&["id"],
        "plugin.inspect"=>&["path","owner"],"mixer.inspect"|"clip.inspect"=>&["path"],
        "preset.export"=>&["output"],"preset.inspect-ffp"=>&["source","device"],"analysis.audio"|"analysis.reverb-ir"=>&["source"],
        "analysis.eq-fit"=>&["points"],"analysis.eq-match"=>&["source","reference"],
        "plugin.edit"|"clip.edit"=>&["path","expect_revision","edit"],"mixer.apply"=>&["path","expect_revision","edits"],
        "preset.import"=>&["path","expect_revision","source","owner","instance_id"],
        "preset.import-ffp"=>&["path","expect_revision","source","device","owner","instance_id"],
        _=>&["path","expect_revision"],
    };
    let mut required=required.to_vec();
    if !readonly {
        let optional=["index","preset_id","start_tick","track_offset","target_track","ripple","independent","replace","shape","source_track"];
        for &field in studio_fields(op).iter().chain(clip_fields(op)) {
            let required_preset=op=="preset.apply"&&field=="preset_id";
            let required_range_start=field=="start_tick"&&op!="clip.trim";
            let required_paste_target=op=="clip.paste"&&field=="target_track";
            if (!optional.contains(&field)||required_preset||required_range_start||required_paste_target)&&!required.contains(&field){required.push(field);}
        }
    }
    let mut keys=required.clone();
    if !readonly{keys.extend(["recovery","allowed_creator","expect_project_id","dry_run"]);}
    match op{
        "plugin.inspect"=>keys.extend(["instance_id","recovery","allowed_creator"]),
        "mixer.inspect"=>keys.extend(["recovery","allowed_creator"]),
        "clip.inspect"=>keys.extend(["track","clip_id","recovery","allowed_creator"]),
        "preset.list"=>keys.extend(["device","category","query","tags"]),
        "preset.export"=>keys.extend(["id","name","path","owner","instance_id","dry_run","recovery","allowed_creator"]),
        "preset.import-ffp"=>keys.push("loss_policy"),
        "analysis.eq-fit"=>keys.extend(["max_bands","sample_rate"]),
        "analysis.eq-match"=>keys.extend(["max_bands","sample_rate","min_frequency","max_frequency","amount","smoothing_octaves",
            "remove_level_offset","max_windows","start_seconds","duration_seconds"]),
        "analysis.reverb-ir"=>keys.push("channel"),
        _=>{}
    }
    keys.extend(studio_fields(op));keys.extend(clip_fields(op));
    if op.starts_with("clip.")&&op!="clip.inspect"{keys.extend(["asset_roots","asset_paths"]);}
    keys.sort();keys.dedup();
    let mut properties=keys.iter().map(|k|(k.to_string(),field_schema(k))).collect::<serde_json::Map<_,_>>();
    if op=="plugin.edit"{properties.insert("edit".into(),edit_schema(false));}
    if op=="clip.edit"{properties.insert("edit".into(),edit_schema(true));}
    if op=="mixer.apply"{properties.insert("edits".into(),json!({"type":"array","minItems":1,"maxItems":1024,"items":edit_schema(false)}));}
    Some(json!({"op":op,"params":{"type":"object","additionalProperties":false,"required":required,"properties":properties},
        "effect":if readonly{"read_or_export"}else{"one_atomic_project_transaction"},"native_units":true}))
}
fn field_schema(key:&str)->Value{
    match key{
        "points"=>json!({"type":"array","minItems":2,"maxItems":65536,"items":{"type":"array","minItems":2,"maxItems":2,
            "prefixItems":[{"type":"number","minimum":20,"maximum":20000},{"type":"number"}]},
            "description":"Frequency (Hz), response (dB) pairs."}),
        "max_bands"=>json!({"type":"integer","minimum":1,"maximum":24}),
        "max_windows"=>json!({"type":"integer","minimum":1,"maximum":256}),
        "channel"=>json!({"type":"integer","minimum":0,"description":"Zero-based source channel; by default analyze the higher-energy channel."}),
        "min_frequency"|"max_frequency"=>json!({"type":"number","minimum":20,"maximum":20000}),
        "amount"=>json!({"type":"number","minimum":0,"maximum":1}),
        "smoothing_octaves"=>json!({"type":"number","minimum":0,"maximum":2}),
        "remove_level_offset"=>json!({"type":"boolean"}),
        "start_seconds"=>json!({"type":"number","minimum":0}),
        "duration_seconds"=>json!({"type":"number","exclusiveMinimum":0}),
        "expect_revision"|"source_offset_frames"|"duration_frames"|"delta_frames"|"track_offset"|"index"|"sample_rate"=>json!({"type":"integer"}),
        "tick"|"start_tick"|"end_tick"|"delta_ticks"|"at_tick"|"threshold_db"|"min_silence_ms"|"window_ms"|"target_db"=>json!({"type":"number"}),
        "dry_run"|"bypass"|"ripple"|"replace"|"independent"=>json!({"type":"boolean"}),
        "values"=>json!({"type":"object","additionalProperties":{"type":"number"}}),
        "owner"=>json!({"type":"object","oneOf":[
            {"type":"object","additionalProperties":false,"required":["kind"],"properties":{"kind":{"const":"master"}}},
            {"type":"object","additionalProperties":false,"required":["kind","track_id"],"properties":{"kind":{"enum":["track","instrument"]},"track_id":{"type":"string"}}}]}),
        "settings"=>json!({"type":"object","additionalProperties":false,"properties":{
            "gain_db":{"type":"number","minimum":-96,"maximum":6},"pan":{"type":"number","minimum":-1,"maximum":1},
            "muted":{"type":"boolean"},"solo":{"type":"boolean"},"name":{"type":"string"}}}),
        "properties"=>json!({"type":"object","additionalProperties":false,"properties":{
            "name":{"type":"string","maxLength":512},"gain_db":{"type":"number","minimum":-120,"maximum":36},"pan":{"type":"number","minimum":-1,"maximum":1},"muted":{"type":"boolean"},
            "reversed":{"type":"boolean"},"playback_rate":{"type":"number","minimum":0.0625,"maximum":16},"preserve_pitch":{"type":"boolean"},
            "fade_in_ms":{"type":"number","minimum":0},"fade_out_ms":{"type":"number","minimum":0},"fade_shape":{"enum":["linear","equal_power"]}}}),
        "clip"|"left"|"right"=>json!({"type":"object","additionalProperties":false,"required":["track","id"],
            "properties":{"track":{"type":"string"},"id":{"type":"string","format":"uuid"}}}),
        "lane"=>json!({"type":"object","required":["target","points"],"properties":{
            "target":automation_target_schema(),"enabled":{"type":"boolean"},"points":{"type":"array","items":{
                "type":"object","required":["tick","value"],"properties":{"tick":{"type":"integer","minimum":0},
                "value":{"type":"number"},"interpolation":{"enum":["step","linear"]}}}}}}),
        "target"=>automation_target_schema(),
        "clipboard"=>json!({"type":"object","required":["schema","clips"],"additionalProperties":false,
            "properties":{"schema":{"const":"verdant-audio-clipboard/1"},"clips":{"type":"array","minItems":1,"items":{
                "type":"object","required":["clip","track_offset","offset_seconds"],"additionalProperties":false,
                "properties":{"clip":{"type":"object","description":"Copy the full source clip record returned by clip.copy or clip.copy-range without dropping unknown metadata."},
                    "track_offset":{"type":"integer","minimum":0},"offset_seconds":{"type":"number","minimum":0}}}}},
            "description":"Use the clipboard returned by clip.copy or clip.copy-range. Source frames, fades and fingerprints must be retained."}),
        "shape"=>json!({"enum":["linear","equal_power"]}),
        "edit"=>json!({"type":"object"}),
        "clips"=>json!({"type":"array","minItems":1,"items":field_schema("clip")}),
        "edits"=>json!({"type":"array","items":{"type":"object"}}),
        "tags"|"instance_ids"|"tracks"|"asset_roots"|"asset_paths"=>json!({"type":"array","items":{"type":"string"}}),
        "recovery"=>json!({"enum":["saved","recovered"]}),"loss_policy"=>json!({"enum":["strict","allow"]}),
        _=>json!({"type":"string"}),
    }
}


fn automation_target_schema()->Value{
    json!({"type":"object","oneOf":[
        {"type":"object","required":["kind","track_id"],"additionalProperties":false,
            "properties":{"kind":{"enum":["track_gain","track_pan"]},"track_id":{"type":"string"}}},
        {"type":"object","required":["kind"],"additionalProperties":false,
            "properties":{"kind":{"enum":["master_gain","master_pan"]}}},
        {"type":"object","required":["kind","owner","instance_id","parameter_id"],"additionalProperties":false,
            "properties":{"kind":{"const":"plugin_parameter"},"instance_id":{"type":"string"},"parameter_id":{"type":"string"},
                "owner":{"type":"object","oneOf":[
                    {"type":"object","required":["kind"],"additionalProperties":false,"properties":{"kind":{"const":"master"}}},
                    {"type":"object","required":["kind","track_id"],"additionalProperties":false,
                        "properties":{"kind":{"const":"track"},"track_id":{"type":"string"}}}
                ]}}}
        ],"description":"Gain automation uses millidecibels; pan uses permille. Plugin values use the native descriptor units. Instrument parameters are edited through plugin.set; insert automation targets track/master effects."})
}

fn edit_schema(audio:bool)->Value{
    let operations=OPERATIONS.iter().filter(|op|if audio{op.starts_with("clip.")&&!clip_fields(op).is_empty()}else{!studio_fields(op).is_empty()});
    let mut variants=operations.map(|op|{
        let fields=if audio{clip_fields(op)}else{studio_fields(op)};
        let operation=match *op{"preset.apply"=>"preset","mixer.set"=>"mixer","mixer.automate"=>"automation",
            "mixer.remove-automation"=>"remove_automation",_=>op.split_once('.').unwrap().1}.replace('-',"_");
        let optional=["index","preset_id","start_tick","track_offset","target_track","ripple","independent","replace","shape","source_track"];
        let mut required=vec!["operation"];
        let mut properties=serde_json::Map::from_iter([("operation".into(),json!({"const":operation}))]);
        for &field in fields{
            let key=if *op=="clip.bounce"&&field=="output"{"path"}else{field};
            if !optional.contains(&field)||(*op=="preset.apply"&&field=="preset_id")||(field=="start_tick"&&*op!="clip.trim")||(*op=="clip.paste"&&field=="target_track"){required.push(key);}
            properties.insert(key.into(),field_schema(field));
        }
        json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
    }).collect::<Vec<_>>();
    variants.push(json!({"type":"object","required":["operation","edits"],"additionalProperties":false,
        "properties":{"operation":{"const":"batch"},"edits":{"type":"array","minItems":1,"items":{"type":"object"}}}}));
    json!({"type":"object","oneOf":variants,"description":"Tagged operation. Nested batches use the same union and commit atomically."})
}
