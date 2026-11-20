use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use verdant_domain::io::{project_output_path, reject_retired_project_format};
use verdant_import::{ImportFormat, ImportOptions, LyricMode, UstEncoding};

const USAGE: &str = "verdant-import SOURCE --output PROJECT.verp [--report REPORT.json] [--catalog manifest.json]
  --format ust|ustx          Otherwise inferred from the source extension
  --track INDEX             Repeat to select tracks; default is all vocal tracks
  --title TEXT              Display title
  --encoding auto|cp932|utf8 UST decoding (USTX is strict UTF-8)
  --pinyin                  Explicitly identify source lyrics as pinyin
  --readings READINGS.json   Inspected domain pronunciation overrides
  --source-sha256 HEX        Require this original-byte SHA-256
  --rest-lyric TEXT          Additional inspected silence marker (repeatable)
  --credit TEXT             Attribution retained in the report (repeatable)
All outputs must be new paths. No source file, voicebank or external tool is executed.
Native projects use .verp. Report defaults to PROJECT.import.json. The optional catalog uses the app's
verdant-audition-catalog/1 schema and references the written project.";

struct Args { source: PathBuf, output: PathBuf, report: PathBuf, catalog: Option<PathBuf>, format: ImportFormat, options: ImportOptions, readings: Option<PathBuf> }
fn invalid(message: impl Into<String>) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message.into()) }
fn main() {
    if let Err(error) = run() { eprintln!("verdant-import: {error}"); std::process::exit(2); }
}
fn run() -> Result<(), Box<dyn Error>> {
    let Some(mut args) = arguments()? else { return Ok(()) };
    let mut targets = vec![args.output.clone(), args.report.clone()];
    if let Some(path) = &args.catalog { targets.push(path.clone()); }
    // Before reading private inputs, refuse every existing destination,
    // including symlinks. create_new below is still the authoritative check.
    for path in &targets {
        match fs::symlink_metadata(path) {
            Ok(_) => return Err(invalid(format!("output already exists: {}", path.display())).into()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {},
            Err(e) => return Err(e.into()),
        }
    }
    if let Some(path) = &args.readings { args.options.readings = verdant_import::parse_readings_json(&fs::read(path)?)?; }
    let raw = fs::read(&args.source)?;
    let imported = verdant_import::import(args.format, &raw, &args.options)?;
    let mut documents = vec![serde_json::to_vec_pretty(&imported.project)?, serde_json::to_vec_pretty(&imported.report)?];
    if let Some(catalog_path) = &args.catalog {
        let parent = absolute(catalog_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")))?;
        let output = absolute(&args.output)?;
        let entry = output.strip_prefix(&parent).unwrap_or(&output).to_str().ok_or_else(|| invalid("catalog project path is not UTF-8"))?;
        documents.push(serde_json::to_vec_pretty(&imported.audition_catalog(entry))?);
    }
    write_new_documents(&targets, &documents)?;
    let summary = serde_json::json!({"project": args.output, "report": args.report, "catalog": args.catalog,
        "source_sha256": imported.report.source.sha256, "tracks": imported.report.tracks,
        "duration_seconds": imported.report.duration_seconds,
        "unsupported_field_count": imported.report.unsupported_fields().count()});
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}
fn absolute(path: &Path) -> io::Result<PathBuf> { Ok(if path.is_absolute() { path.into() } else { std::env::current_dir()?.join(path) }) }
fn write_new_documents(paths: &[PathBuf], documents: &[Vec<u8>]) -> io::Result<()> {
    let mut created: Vec<(&PathBuf, File)> = Vec::new();
    let result = (|| {
        for path in paths {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) { fs::create_dir_all(parent)?; }
            let mut options = OpenOptions::new(); options.write(true).create_new(true);
            #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
            created.push((path, options.open(path)?));
        }
        for ((_, file), document) in created.iter_mut().zip(documents) {
            file.write_all(document)?; file.write_all(b"\n")?; file.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (path, file) in created { drop(file); let _ = fs::remove_file(path); }
    }
    result
}
fn arguments() -> Result<Option<Args>, Box<dyn Error>> {
    let mut iter = std::env::args_os().skip(1);
    let mut source = None;
    let mut output = None;
    let mut report = None;
    let mut catalog = None;
    let mut format = None;
    let mut readings = None;
    let mut options = ImportOptions::default();
    while let Some(arg) = iter.next() {
        let option = arg.to_str().ok_or_else(|| invalid("arguments must be UTF-8"))?;
        if matches!(option, "--help" | "-h") { println!("{USAGE}"); return Ok(None); }
        let mut next = || iter.next().ok_or_else(|| invalid(format!("missing value after {option}")));
        match option {
            "--output" => output = Some(PathBuf::from(next()?)),
            "--report" => report = Some(PathBuf::from(next()?)),
            "--catalog" => catalog = Some(PathBuf::from(next()?)),
            "--readings" => readings = Some(PathBuf::from(next()?)),
            "--title" => options.title = Some(utf8(next()?)?),
            "--source-sha256" => options.expected_source_sha256 = Some(utf8(next()?)?),
            "--credit" => options.credits.push(utf8(next()?)?),
            "--rest-lyric" => { options.extra_rest_lyrics.insert(utf8(next()?)?); },
            "--pinyin" => options.lyric_mode = LyricMode::Pinyin,
            "--track" => options.tracks.push(utf8(next()?)?.parse::<usize>().map_err(|_| invalid("track must be a nonnegative integer"))?),
            "--encoding" => options.ust_encoding = match utf8(next()?)?.as_str() {
                "auto" => UstEncoding::Auto, "cp932" => UstEncoding::Cp932, "utf8" | "utf-8" => UstEncoding::Utf8,
                _ => return Err(invalid("encoding must be auto, cp932 or utf8").into()),
            },
            "--format" => {
                let name = utf8(next()?)?;
                reject_retired_project_format(&name)?;
                format = Some(match name.as_str() {
                    "ust" => ImportFormat::Ust,
                    "ustx" => ImportFormat::Ustx,
                    _ => return Err(invalid("format must be ust or ustx").into()),
                });
            },
            _ if option.starts_with('-') => return Err(invalid(format!("unknown option {option}; use --help")).into()),
            _ => {
                if source.replace(PathBuf::from(arg)).is_some() { return Err(invalid("expected one source file").into()); }
            }
        }
    }
    let source = source.ok_or_else(|| invalid("source file required; use --help"))?;
    let extension = source.extension().and_then(|s| s.to_str()).unwrap_or_default();
    reject_retired_project_format(extension)?;
    let output = output.ok_or_else(|| invalid("--output is required"))?;
    let output = project_output_path(&output)?;
    let report = report.unwrap_or_else(|| output.with_extension("import.json"));
    let format = format.or_else(|| ImportFormat::from_extension(extension))
        .ok_or_else(|| invalid(format!("unsupported source extension: {}; expected .ust or .ustx (use verdant-exchange import for UFDATA, MIDI, SVP, VSQX or VPR), or specify --format", source.display())))?;
    options.source_name = source.file_name().and_then(|s| s.to_str()).map(str::to_string);
    Ok(Some(Args { source, output, report, catalog, format, options, readings }))
}
fn utf8(value: OsString) -> Result<String, io::Error> { value.into_string().map_err(|_| invalid("option value must be UTF-8")) }
