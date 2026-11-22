#!/usr/bin/env python3
"""Generate versioned, standalone Agent Skills from Verdant's executable schema."""
import argparse, hashlib, json, pathlib, subprocess, sys, tomllib

PURPOSES = {
"capabilities":"discover installed operations, formats and protocols", "schema":"inspect an operation's exact admitted fields and types",
"doctor":"diagnose synthesis and rendering dependencies", "project.new":"create a native project at an explicit output path",
"project.inspect":"read project identity, revision, tracks, notes and recovery state", "project.validate":"validate a native project",
"project.apply":"commit note, track and project edits atomically", "project.save-as":"copy a native project with history and verified media",
"project.recover":"apply an explicitly selected saved or recovered branch", "project.migrate":"migrate a project with a stated loss policy",
"project.pack":"pack a portable project with its verified media", "project.unpack":"unpack a bounded portable archive to a fresh destination",
"history.list":"inspect undo and redo history", "history.undo":"undo one project transaction", "history.redo":"redo one project transaction",
"import":"import a supported singing project or MIDI file", "export.project":"export an interchange project with a loss report",
"render.plan":"preflight mix, track, stem or range export", "render":"render a project through the synthesis and mixing pipeline",
"export.audio":"export a mix, track, stems or range with effects and automation",
"job.list":"find persisted rendering jobs","job.inspect":"inspect a rendering job","job.cancel":"cancel an identified rendering job",
"job.retry":"retry a failed or cancelled rendering job","job.verify":"verify rendered job artifacts",
"cache.list":"inspect the rendering cache","cache.verify":"verify cached artifact fingerprints","cache.clean":"remove selected cache entries",
"voice.list":"discover installed voices","voice.verify":"verify a voice package with the synthesis worker",
"voice.convert":"convert authorized original voice inputs into a package","voice.install":"install a verified voice package",
"voice.remove":"remove an identified voice package","voice.set-default":"select the default voice",
"voice.import-originals":"import authorized original voice token and model files",
"batch":"execute sequential independent requests with explicit error handling",
"media.import":"admit an audio source as a non-destructive timeline clip","media.relink":"relink a clip to an explicit audio source",
"media.inspect":"inspect imported audio sources and stable clip IDs","media.verify":"verify saved source fingerprints",
"effect.scan":"scan explicit external plugin files in isolated hosts","effect.catalog":"read the external and native plugin catalog",
"effect.status":"inspect saved plugin instances and availability",
"plugin.list":"discover every native effect and instrument","plugin.describe":"discover native parameter IDs, units, ranges and automation eligibility",
"plugin.inspect":"read a native plugin instance's saved parameters","plugin.add":"insert a native effect or instrument",
"plugin.remove":"remove a plugin and its dependent automation","plugin.reorder":"reorder inserts by stable instance IDs",
"plugin.set":"write a validated set of native parameters atomically","plugin.reset":"restore one plugin's complete defaults",
"plugin.bypass":"set a plugin's bypass state","plugin.sidechain":"route or clear an effect's external sidechain",
"plugin.edit":"commit a typed atomic batch of plugin operations",
"mixer.inspect":"inspect channels, master settings and automation","mixer.set":"set track or master gain, pan and channel properties",
"mixer.apply":"commit several mixer and plugin edits in one transaction","mixer.automate":"author a validated automation lane",
"mixer.remove-automation":"remove an identified automation lane",
"preset.list":"search the original factory library","preset.show":"read a factory preset's metadata and complete native state",
"preset.apply":"apply a factory preset to its matching plugin","preset.import":"load a validated native vpreset",
"preset.export":"save a native preset to a fresh file","preset.inspect-ffp":"inspect verified FabFilter mappings and unsupported fields",
"preset.import-ffp":"import verified FabFilter mappings with an explicit loss policy",
"clip.inspect":"inspect stable audio clip IDs and source metadata","clip.edit":"commit a typed atomic batch of clip operations",
"clip.split":"split selected clips at a tick while preserving source frames and fades",
"clip.split-range":"split clips at both edges of a selected time range","clip.trim":"set non-destructive source-frame trim bounds",
"clip.trim-to-range":"keep selected audio inside a time range","clip.slip":"shift source content without moving timeline positions",
"clip.move":"move clips together in time and across tracks","clip.duplicate":"duplicate clips at a chosen tick and track",
"clip.delete":"delete clips with an explicit ripple choice","clip.delete-range":"remove a selected range with an explicit ripple choice",
"clip.copy":"obtain a JSON audio clipboard","clip.copy-range":"copy selected audio inside a time range",
"clip.paste":"paste an explicit JSON audio clipboard","clip.set-properties":"edit clip gain, pan, fades, mute, reverse or playback rate",
"clip.join":"join compatible adjacent source slices","clip.crossfade":"apply matched fades to overlapping clips",
"clip.split-silence":"split clips at measured silent regions","clip.split-zero-crossing":"split near a measured zero crossing",
"clip.normalize":"measure audio and set non-destructive peak normalization","clip.bounce":"sum clip processing into a fresh float WAV",
"analysis.audio":"measure sample peak, RMS, DC offset and clipped samples",
"analysis.eq-fit":"fit a measured or drawn target response with native EQ bands",
"analysis.eq-match":"measure source and reference audio spectra and produce a native EQ match",
"analysis.reverb-ir":"estimate native algorithmic reverb settings from a measured impulse response",
"skill.list":"list bundled offline Agent Skills","skill.show":"inspect a bundled Agent Skill",
"skill.install":"install selected bundled skills for an agent","skill.uninstall":"remove only unmodified Verdant-managed skills",
}
SPECIAL={
"project.new":"Use output alone for a valid initialized native document, then use revision-checked project.apply for edits. An optional project payload must be a complete validated native project record, not a partial patch. Inspect an existing project for its full structure; preserve unknown data.",
"plugin.add":'Effects use owner {"kind":"track","track_id":"..."} or {"kind":"master"}. Synth/One use {"kind":"instrument","track_id":"..."}. Retain the returned instance_id for later edits.',
"plugin.set":"Read plugin.describe. Values use native units, not guessed normalized controls. One invalid value rejects the whole update.",
"plugin.reorder":"Supply every current instance_id exactly once. Device IDs identify types; instance IDs identify saved slots.",
"plugin.sidechain":"source_track is a stable track ID. Omitting it clears the sidechain. Inspect the destination first.",
"plugin.edit":"edit is tagged with operation. operation=batch takes edits and commits one project revision.",
"mixer.apply":"edits contains typed StudioEdit objects. This operation is rollback-atomic; the separate batch operation is sequential.",
"mixer.set":"gain_db uses dB (-96..6), pan uses -1..1. Master supports gain/pan/mute; track also supports solo/name.",
"mixer.automate":"Ticks strictly increase. Use step interpolation for stepped controls; control-only parameters cannot be automated. Gain lanes use millidecibels, pan lanes use permille. Plugin lanes use native parameter units.",
"preset.apply":"Find an ID for the exact device using preset.list/show. Loading restores complete state, without inheriting the prior sound's hidden settings.",
"preset.import-ffp":"Inspect first. strict rejects incomplete mappings. loss_policy=allow requires the user's acceptance of the reported omissions. Mapping success does not imply identical proprietary DSP.",
"preset.export":"output must be fresh. Choose a factory id or project path+owner+instance_id. Native vpreset v1 is shared with the GUI.",
"clip.edit":"edit is tagged with operation. operation=batch takes edits. Planning/dry_run never publishes bounce assets.",
"clip.trim":"source_offset_frames and duration_frames count decoded source frames. Omitted start_tick preserves timeline start. Source trimming remains restorable.",
"clip.slip":"delta_frames counts decoded source frames; the musical start remains unchanged.",
"clip.move":"delta_ticks may be fractional. track_offset is signed and refers to visible track order.",
"clip.delete":"Use ripple only when closing the removed interval is intended.",
"clip.delete-range":"Name affected tracks explicitly. Use ripple only when closing the interval is intended.",
"clip.copy":"Pass the returned clipboard object unchanged to clip.paste. Copy creates no project revision.",
"clip.copy-range":"Pass the returned clipboard object unchanged to clip.paste. Copy creates no project revision.",
"clip.paste":"Use the actual clipboard from copy/copy-range, not IDs guessed from another document.",
"clip.set-properties":"properties supports name, gain_db, pan, muted, reversed, playback_rate, preserve_pitch, fade_in_ms, fade_out_ms, fade_shape. Set preserve_pitch with playback_rate for duration changes that retain pitch.",
"clip.join":"Join requires compatible adjacent slices of the same source/edit state. Use bounce to mix different sources.",
"clip.crossfade":"The two clip references must overlap. Inspect their actual positions.",
"clip.split-silence":"This reads actual PCM; supply explicitly authorized asset_paths or asset_roots for absolute source references.",
"clip.split-zero-crossing":"This reads actual PCM in window_ms and selects a measured nearby boundary.",
"clip.normalize":"target_db is a sample-peak target, not an integrated-loudness promise.",
"clip.bounce":"The CLI uses output for the fresh WAV; nested AudioEdit uses path for that asset. The outer path always selects the project. replace=true replaces selected clips. Bounce includes clip processing; export.audio includes track/master inserts.",
"analysis.audio":"Sample peak is not reconstructed true peak. RMS is not LUFS. Analyze the actual final output for delivery measurements.",
"analysis.eq-fit":"points are [frequency_Hz, gain_dB] pairs. max_bands is 1..24; use the intended processing sample_rate. This is read-only: inspect fit.rms_error_db/max_error_db, then apply returned values to an EQ instance with revision-checked plugin.set. A finite band fit does not exactly reproduce every drawn curve.",
"analysis.eq-match":"source/reference are explicit audio paths. The analysis averages real stereo power in bounded Hann windows; anti-phase stereo does not cancel. Choose sample_rate for the destination EQ, amount, smoothing_octaves and a valid min_frequency/max_frequency range. By default broad level differences are removed. Inspect fit errors and source fingerprints, then apply returned values with plugin.set. It does not promise identical timbre.",
"analysis.reverb-ir":"source must be an actual measured IR, not a song or test sweep. channel is zero-based; omitted selects the higher-energy channel. Inspect decay confidence/noise/model errors before applying values to the exact reverb device through plugin.set. This estimates algorithmic reverb parameters, not convolution; source PCM is not embedded into the plugin.",
"render.plan":"Use the exact intended export parameters. Absolute imported audio paths require explicit asset_roots; project_root belongs to the synthesis worker and does not authorize audio assets. workspace stores job receipts, render cache and plugin cache; it grants no input-file access. duration_seconds describes the output range; processing_timeline_seconds includes its preroll from project zero. Explicit ranges exclude later tails; a full export includes bounded track/master tails.",
"export.audio":"Preflight with render.plan using the same asset_roots, effects, scope and range. Absolute imported source paths require explicit asset_roots; project_root is only for the synthesis worker. workspace selects the job-receipt and render/plugin-cache directory; it is not audio authorization. Select Mix, ActiveTrack or Stems and effects explicitly. Range ticks follow the tempo map. Verify frames/sample_rate and artifacts in the completed result; keep the fresh report beside the WAV.",
"render":"Preflight with render.plan using the same asset_roots, effects, scope and range. project_root is a synthesis-worker setting; it is not imported-audio authorization. Use asset_roots for explicitly authorized external audio. Verify completed frames/sample_rate and every output artifact.",
"batch":"At most 256 complete requests, no nested batches. Earlier commits remain if a later item fails. Use typed edit batches when one transaction is required.",
"skill.install":"Use name or all, agent=codex|claude|generic, scope=user|project, and dest/project_root as needed. Hash validation and edited/unrelated-file protection must remain enabled.",
"skill.uninstall":"Use the same installation destination. Only receipt-owned unmodified payloads can be removed.",
}
def query(binary,op,params=None):
    request={"protocol":"verdant-cli/1","request_id":"skill-build-"+op,"op":op,"params":params or {}}
    run=subprocess.run([str(binary),"request","--json"],input=json.dumps(request),text=True,capture_output=True,timeout=60,check=True)
    response=json.loads(run.stdout)
    if not response.get("ok"):raise RuntimeError(response)
    return response["result"]
def write(path,text):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(text.replace("§",chr(96)),encoding="utf-8",newline="\n")
def emit(root,name,description,body,reference,initializer):
    folder=root/name
    if initializer and not folder.exists():
        subprocess.run([sys.executable,str(initializer),name,"--path",str(root),"--resources","references",
            "--interface","display_name="+name.replace("-"," ").title(),
            "--interface","short_description=Atomic Verdant audio and project operation",
            "--interface","default_prompt=Use $"+name+" for the requested Verdant operation."],
            check=True,stdout=subprocess.DEVNULL)
    write(folder/"SKILL.md","---\nname: "+name+"\ndescription: "+json.dumps(description,ensure_ascii=False)+"\n---\n\n"+body.rstrip()+"\n")
    write(folder/"agents/openai.yaml","interface:\n  display_name: "+json.dumps(name.replace("-"," ").title())+
        "\n  short_description: "+json.dumps("Operate Verdant through its typed headless API")+
        "\n  default_prompt: "+json.dumps("Use $"+name+" to complete the requested Verdant task.")+
        "\npolicy:\n  allow_implicit_invocation: true\n")
    if reference is not None:write(folder/"references/operation.json",json.dumps(reference,ensure_ascii=False,sort_keys=True,indent=2)+"\n")
    for directory in sorted((p for p in folder.rglob("*") if p.is_dir()), reverse=True):
        if not any(directory.iterdir()):directory.rmdir()
    files=[{"path":p.relative_to(folder).as_posix(),"sha256":hashlib.sha256(p.read_bytes()).hexdigest()}for p in sorted(folder.rglob("*"))if p.is_file()]
    return {"name":name,"description":description,"files":files}

def main():
    ap=argparse.ArgumentParser();ap.add_argument("--binary",required=True,type=pathlib.Path)
    ap.add_argument("--output",type=pathlib.Path,default=pathlib.Path(__file__).resolve().parents[1]/"skills")
    ap.add_argument("--initializer",type=pathlib.Path);args=ap.parse_args()
    workspace=pathlib.Path(__file__).resolve().parents[1]/"Cargo.toml"
    version=tomllib.loads(workspace.read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    root=args.output.resolve();root.mkdir(parents=True,exist_ok=True)
    schema=query(args.binary,"schema");caps=query(args.binary,"capabilities");entries=[]
    for operation in schema["operations"]:
        op=operation["op"];name="verdant-"+op.replace(".","-").replace("_","-")
        purpose=PURPOSES.get(op,op.replace("."," ").replace("-"," ")+" in Verdant")
        description="Use when an AI Agent needs to "+purpose+". Performs "+op+" through Verdant's CLI or NDJSON worker."
        required=operation.get("params",{}).get("required",[]);mutation="expect_revision"in required
        body="# "+op+"\n\nComplete one operation: "+purpose+".\n\n## Procedure\n\n"
        body+="1. Locate §verdant§ and run §verdant capabilities --json§ once per session. Bare §verdant§ displays help and exits; GUI startup is explicit with §verdant gui§.\n"
        body+="2. Read §references/operation.json§. Refresh with §verdant schema --op "+op+" --json§ when the runtime differs. Supply only admitted fields, stable IDs and native units.\n"
        if mutation:
            body+="3. Run §project.inspect§ on the chosen document. Resolve tracks/clips/instances from its result. Use the current recoverable revision for §expect_revision§ and bind §expect_project_id§. Honor the user's saved/recovered branch choice; do not silently discard recovery.\n4. Submit the edit. Use §dry_run=true§ when review is needed; re-inspect after another writer changes the revision. Validation precedes one undoable transaction.\n"
        else:
            body+="3. Resolve required paths and IDs from the user task or an inspect/list response. Choose explicit output destinations for file-producing operations. Submit and verify the response.\n"
        body+="\n## Invocation\n\nUse structured stdin with an argument array:\n\n§§§python\nrequest = {\n    \"protocol\": \"verdant-cli/1\",\n    \"request_id\": \"unique-task-operation-id\",\n    \"op\": \""+op+"\",\n    \"params\": params,  # Resolve from the schema and actual project IDs.\n}\nrun = subprocess.run(\n    [verdant_binary, \"request\", \"--json\"],\n    input=json.dumps(request), text=True, capture_output=True, timeout=timeout_seconds,\n)\nresponse = json.loads(run.stdout)\nif not response.get(\"ok\"):\n    raise RuntimeError(response.get(\"error\"))\n§§§\n\n"
        if required:body+="Required fields: "+", ".join("§"+k+"§"for k in required)+". The reference contains optional fields and nested structures.\n\n"
        if op in SPECIAL:body+="## Details\n\n"+SPECIAL[op]+"\n\n"
        if "dry_run" in operation.get("params",{}).get("properties",{}):body+="A dry-run response is a preview: its revision, commit and newly generated IDs are not persisted. Submit the real edit against the original inspected revision and use only the real response's created IDs.\n\n"
        body+="## Verification and recovery\n\nCheck exit status and JSON §ok§. Return actual IDs, revisions, paths and material diagnostics. Read back edited state. Use §history.undo§ for a requested reversal. On §REVISION_CONFLICT§ inspect and rebuild the edit. Preserve the user's open document on writer-lock errors. After a transport timeout inspect before retrying a non-idempotent edit. Missing audio and incomplete mappings are explicit errors; never substitute silence or guessed controls.\n\nThe same request works as one JSON line on §verdant worker --stdio§. Match §request_id§, consume progress through its result record, and close stdin when finished. Requests are sequential. This workflow worker differs from the synthesis executable §verdant-worker§.\n"
        entries.append(emit(root,name,description,body,operation,args.initializer))
    for item in caps["project_operations"]["operations"]:
        op=item["op"];name="verdant-edit-"+op.replace("_","-")
        description="Use to "+op.replace("_"," ")+" in Verdant as one revision-checked, undoable atomic project edit."
        body="# "+op.replace("_"," ")+"\n\nInspect the project and resolve stable track/note IDs. Read §references/operation.json§; refresh §verdant schema --op project.apply --json§ if the runtime differs.\n\nSubmit one completed §project.apply§ request:\n\n§§§json\n"+json.dumps({
            "protocol":"verdant-cli/1","request_id":"unique-edit-id","op":"project.apply","params":{
                "path":"ABSOLUTE_PROJECT_PATH","expect_revision":0,"expect_project_id":"INSPECTED_PROJECT_ID",
                "label":op.replace("_"," "),"operations":[item.get("example",{"op":op})]}},indent=2)+"\n§§§\n\n"
        body+="Replace the example path, ID and revision with inspected values, and add the operation fields from the reference. The example omits task-specific data and must be completed before execution. Native units come from the schema/domain record. Multiple related commands in operations commit as one revision.\n\nCheck JSON §ok§, read back changed state and record the revision. §dry_run=true§ validates without saving. Re-inspect on conflict. Preserve unknown document data and the user's recovery choice. Use §history.undo§ for a requested reversal. The same request works over §verdant worker --stdio§.\n"
        if op in("set_track_data","set_project_extension","set_track_inserts","set_track_instrument"):
            body+="\nPrefer typed §clip.*§, §plugin.*§ and §mixer.*§ operations for native state; they validate media/device invariants. Use this generic operation for explicitly supplied extension data.\n"
        body+="\nUnits: read project.ppq for ticks per quarter note. Tempo uses micro_bpm = BPM * 1000000 (120 BPM = 120000000). Mixer document gain is millidecibels and pan is permille; plugin controls use their native descriptor units. A dry-run creates no persistent revision or IDs: submit with the inspected base revision and use the actual result.\n"
        reference=dict(item);reference["units"]=caps["project_operations"].get("units",{})
        entries.append(emit(root,name,description,body,reference,args.initializer))
    overview="""# Verdant Agent workflow

Use the installed headless §verdant§ command. Bare §verdant§ prints help and exits. Open a desktop with §verdant gui§; packaged users can double-click the root §verdant-launcher§.

## Discover and install

§§§text
verdant capabilities --json
verdant schema --json
verdant skill list --json
verdant skill install --all --agent codex --scope user
§§§

Use §--agent claude§ or §--agent generic§ for those layouts. §--scope project --project-root /absolute/project§ installs for a project; §--dest§ chooses an explicit directory. Read §skill.show§ for one skill. Installation is offline, hash checked and protects edited/unrelated files.

## Work from stable identities

1. Inspect the project; resolve track, clip and plugin instance IDs. Device IDs identify types; instance_id identifies a saved slot.
2. Choose the operation-specific §verdant-*§ skill, inspect its schema and send structured JSON on stdin.
3. Bind mutations to the recoverable revision and project ID. Honor the saved/recovered choice. Keep related edits in one §project.apply§, §plugin.edit§, §mixer.apply§ or §clip.edit§ transaction.
4. Read back. Export through §render.plan§ then §export.audio§; measure the actual file with §analysis.audio§.
5. Report actual outputs and diagnostics. Loading a preset does not prove mix quality; inspect the rendered material.

| Task | Family |
| --- | --- |
| Native project lifecycle, recovery, history | project, history |
| Notes, lyrics, curves and structure | verdant-edit-* via project.apply |
| Audio admission and fingerprints | media |
| Slice, trim, slip, reverse, stretch, fades, bounce | clip |
| Inserts, native controls, bypass and sidechain | plugin |
| Faders, pan, mute, solo and automation | mixer |
| Factory library and native/FFP import/export | preset |
| Mix, stems, ranges, jobs and cache | render, export, job, cache |
| Voices and runtime diagnostics | voice, doctor |
| External plugin discovery | effect |
| Measured file statistics | analysis |
| Offline Agent Skills lifecycle | skill |

## Worker and atomicity

Launch §verdant worker --stdio§. Send complete §verdant-cli/1§ requests as JSON lines, match progress/result by request_id, and close stdin. One request runs at a time. §verdant-worker§ is the distinct synthesis backend.

§batch§ is sequential and preserves prior commits after a later failure. Use typed atomic edit batches for all-or-nothing changes. After a timeout inspect state before retrying. JSON errors are authoritative; resolve missing assets and avoid guessed parameter mappings.
"""
    entries.append(emit(root,"verdant","Use when an AI Agent needs to operate Verdant end to end, choose atomic skills, install the offline bundle, or use the CLI and workflow worker.",overview,None,args.initializer))
    entries.sort(key=lambda e:e["name"])
    write(root/"manifest.json",json.dumps({"schema":"verdant-agent-skills/1","version":version,"skills":entries},ensure_ascii=False,indent=2)+"\n")
    print(json.dumps({"skills":len(entries),"operations":len(schema["operations"]),"domain_operations":len(caps["project_operations"]["operations"]),"output":str(root)}))
if __name__=="__main__":main()
