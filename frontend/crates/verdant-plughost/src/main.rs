use std::process::ExitCode;
use verdant_plughost::protocol_output::ProtocolOutput;

fn main() -> ExitCode {
    // Plugin processes must not die on SIGPIPE (control peer closed).
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("verdant-plughost protocol output: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> std::io::Result<ExitCode> {
    let mut args = std::env::args().skip(1);
    let Some(mode) = args.next() else {
        eprintln!("usage: verdant-plughost scan-file <path> | host --channel <fd:a,b,c|name:a,b,c>");
        return Ok(ExitCode::from(2));
    };

    match mode.as_str() {
        "scan-file" => {
            let Some(path) = args.next() else {
                eprintln!("scan-file: missing path");
                return Ok(ExitCode::from(2));
            };
            let mut output = ProtocolOutput::capture()?;
            let (report, code) = match verdant_plughost::scan_file(std::path::Path::new(&path)) {
                Ok(report) => (report, ExitCode::SUCCESS),
                Err(e) => {
                    // Still emit a parseable report so the parent can record
                    // the failure into its blacklist, then exit non-zero.
                    let report = verdant_plugins::proto::ScanFileReport {
                        path: path.clone(),
                        format: "unknown".into(),
                        sha256: String::new(),
                        file_size: 0,
                        plugins: vec![],
                        error: Some(e.to_string()),
                        scan_ms: 0,
                    };
                    (report, ExitCode::from(1))
                }
            };
            output.write_line(&report)?;
            Ok(code)
        }
        "host" => {
            let mut channel_arg = None;
            for a in args.by_ref() {
                if let Some(v) = a.strip_prefix("--channel=") {
                    channel_arg = Some(v.to_string());
                }
            }
            let Some(channel_arg) = channel_arg else {
                eprintln!("host: missing --channel");
                return Ok(ExitCode::from(2));
            };
            let output = ProtocolOutput::capture()?;
            match verdant_plughost::run_host(&channel_arg, output) {
                Ok(()) => Ok(ExitCode::SUCCESS),
                Err(e) => {
                    eprintln!("verdant-plughost host: {e}");
                    Ok(ExitCode::from(1))
                }
            }
        }
        _ => {
            eprintln!("unknown mode: {mode}");
            Ok(ExitCode::from(2))
        }
    }
}
