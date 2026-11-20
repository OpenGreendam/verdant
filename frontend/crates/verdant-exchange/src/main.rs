use std::error::Error;
use std::ffi::OsString;
use std::fs::{self,File,OpenOptions};
use std::io::{self,Read,Write};
use std::path::PathBuf;
use verdant_domain::io::{project_output_path, reject_retired_project_format, require_project_path};
use verdant_exchange::{self as exchange,ExportOptions,Format,ImportOptions,Limits,UstEncoding,VocaloidVoice};
const USAGE:&str="verdant-exchange import SOURCE --output PROJECT.verp [--format midi|svp|vsqx|vpr|ufdata] [--report REPORT.json]\nverdant-exchange export PROJECT.verp --output SCORE --format midi|ust|ustx|svp|vsqx|vpr|ufdata [options]\n\nOptions:\n  --options JSON             ExportOptions JSON sidecar\n  --track INDEX              Select one zero-based track (required for multitrack UST)\n  --midi-ppq PPQ             Exact destination SMF resolution, 1..32767\n  --cp932                    Encode UST using CP932 (unencodable text is an error)\n  --discard-extensions       Disable compatible source-extension roundtrip\n  --voice-comp-id ID         Actual licensed VOCALOID voice compID (all four voice fields required)\n  --voice-name NAME          Actual destination voice name\n  --voice-language ID        Actual destination voice language ID\n  --voice-program NUMBER     Actual destination voice program\n\nOutputs are created exclusively and never overwrite existing files. The JSON report\nrecords field losses and numerical conversions. UST/USTX import: use verdant-import.\nNative projects use .verp. An extensionless import destination gains .verp.\nNo UTAU sample engine, VOCALOID or Synthesizer V synthesis is invoked.";
fn invalid(message:impl Into<String>)->io::Error{io::Error::new(io::ErrorKind::InvalidInput,message.into())}
fn utf8(value:OsString)->io::Result<String>{value.into_string().map_err(|_|invalid("option must be UTF-8"))}
fn format(text: &str) -> Result<Format, Box<dyn Error>> {
    reject_retired_project_format(text)?;
    Ok(Format::from_extension(text).ok_or_else(|| invalid("unknown format"))?)
}
struct Args{import:bool,source:PathBuf,output:PathBuf,report:PathBuf,format:Format,options:Option<PathBuf>,track:Option<usize>,midi_ppq:Option<u16>,cp932:bool,discard:bool,voice:Option<VocaloidVoice>}
fn arguments()->Result<Option<Args>,Box<dyn Error>>{
    let mut it=std::env::args_os().skip(1);let Some(command)=it.next()else{return Err(invalid("command required; use --help").into());};let command=utf8(command)?;
    if matches!(command.as_str(),"--help"|"-h"){println!("{USAGE}");return Ok(None);}let import=match command.as_str(){"import"=>true,"export"=>false,_=>return Err(invalid("command must be import or export").into())};
    let(mut source,mut output,mut report,mut selected_format,mut options)=(None,None,None,None,None);let(mut track,mut midi_ppq)=(None,None);let(mut cp932,mut discard)=(false,false);
    let(mut comp_id,mut voice_name,mut language,mut program)=(None,None,None,None);
    while let Some(arg)=it.next(){let text=arg.to_str().ok_or_else(||invalid("argument must be UTF-8"))?;if matches!(text,"--help"|"-h"){println!("{USAGE}");return Ok(None);}let mut next=||it.next().ok_or_else(||invalid(format!("missing value for {text}")));
        match text{"--output"=>output=Some(PathBuf::from(next()?)),"--report"=>report=Some(PathBuf::from(next()?)),"--format"=>selected_format=Some(format(&utf8(next()?)?)?),"--options"=>options=Some(PathBuf::from(next()?)),"--track"=>track=Some(utf8(next()?)?.parse::<usize>()?),"--midi-ppq"=>midi_ppq=Some(utf8(next()?)?.parse::<u16>()?),"--cp932"=>cp932=true,"--discard-extensions"=>discard=true,
            "--voice-comp-id"=>comp_id=Some(utf8(next()?)?),"--voice-name"=>voice_name=Some(utf8(next()?)?),"--voice-language"=>language=Some(utf8(next()?)?.parse::<u8>()?),"--voice-program"=>program=Some(utf8(next()?)?.parse::<u8>()?),
            _ if text.starts_with('-')=>return Err(invalid(format!("unknown option {text}")).into()),_=>{if source.replace(PathBuf::from(arg)).is_some(){return Err(invalid("expected one source file").into());}}
        }
    }
    let voice=match(comp_id,voice_name,language,program){(None,None,None,None)=>None,(Some(comp_id),Some(name),Some(language_id),Some(program))=>Some(VocaloidVoice{comp_id,name,language_id,program}),_=>return Err(invalid("provide all four VOCALOID voice fields").into())};
    if import&&(options.is_some()||track.is_some()||midi_ppq.is_some()||cp932||voice.is_some()){return Err(invalid("export options are not import options").into());}
    let source=source.ok_or_else(||invalid("source path is required"))?;let output=output.ok_or_else(||invalid("--output is required"))?;
    let output = if import {
        reject_retired_project_format(source.extension().and_then(|s| s.to_str()).unwrap_or_default())?;
        project_output_path(&output)?
    } else {
        require_project_path(&source)?;
        reject_retired_project_format(output.extension().and_then(|s| s.to_str()).unwrap_or_default())?;
        output
    };
    let selected_format=selected_format.or_else(||(if import{source.extension()}else{output.extension()}).and_then(|s|s.to_str()).and_then(Format::from_extension)).ok_or_else(||invalid("specify --format or a recognized score extension"))?;
    if import&&matches!(selected_format,Format::Ust|Format::Ustx){return Err(invalid("UST/USTX import belongs to verdant-import").into());}
    let report=report.unwrap_or_else(||output.with_extension("exchange.json"));Ok(Some(Args{import,source,output,report,format:selected_format,options,track,midi_ppq,cp932,discard,voice}))
}
fn read_bounded(path:&PathBuf,limit:usize)->io::Result<Vec<u8>>{let file=File::open(path)?;let mut bytes=Vec::new();file.take((limit as u64).saturating_add(1)).read_to_end(&mut bytes)?;if bytes.len()>limit{return Err(invalid("input exceeds byte limit"));}Ok(bytes)}
fn main(){if let Err(error)=run(){eprintln!("verdant-exchange: {error}");std::process::exit(2);}}
fn run()->Result<(),Box<dyn Error>>{
    let Some(args)=arguments()?else{return Ok(());};let paths=[args.output.clone(),args.report.clone()];let mut created:Vec<(PathBuf,File)>=Vec::new();
    let result=(||->Result<(),Box<dyn Error>>{
        // Reserve every destination before reading inputs, including options.
        // create_new rejects existing files, symlinks, aliases and races.
        for path in &paths{if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()){fs::create_dir_all(parent)?;}let mut options=OpenOptions::new();options.write(true).create_new(true);#[cfg(unix)]{use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}created.push((path.clone(),options.open(path)?));}
        let mut options=match &args.options{Some(path)=>exchange::parse_export_options_json(&read_bounded(path,1024*1024)?)?,None=>ExportOptions::default()};
        if let Some(track)=args.track{options.track=Some(track);}if let Some(ppq)=args.midi_ppq{options.midi_ppq=Some(ppq);}if args.cp932{options.ust_encoding=UstEncoding::Cp932;}if args.discard{options.preserve_extensions=false;}if let Some(voice)=&args.voice{options.vocaloid_voice=Some(voice.clone());}
        let(source_bytes,report,losses)=if args.import{let limits=Limits::default();let raw=read_bounded(&args.source,limits.max_file_bytes)?;let imported=exchange::import(args.format,&raw,&ImportOptions{source_name:args.source.file_name().and_then(|s|s.to_str()).map(str::to_string),preserve_extensions:!args.discard,limits})?;let losses=imported.report.has_losses();(serde_json::to_vec_pretty(&imported.project)?,serde_json::to_vec_pretty(&imported.report)?,losses)}else{let raw=read_bounded(&args.source,options.limits.max_file_bytes)?;let project=exchange::parse_project_json(&raw,&options.limits)?;let exported=exchange::export(args.format,&project,&options)?;let losses=exported.report.has_losses();(exported.bytes,serde_json::to_vec_pretty(&exported.report)?,losses)};
        for((_,file),bytes)in created.iter_mut().zip([source_bytes,report]){file.write_all(&bytes)?;file.sync_all()?;}
        println!("{}",serde_json::json!({"output":args.output,"report":args.report,"has_losses":losses,"format":args.format}));Ok(())
    })();
    if result.is_err(){for(path,file)in created{drop(file);let _=fs::remove_file(path);}}result
}
