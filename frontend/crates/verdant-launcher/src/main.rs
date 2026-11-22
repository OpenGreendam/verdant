#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
use std::{env, process::ExitCode};
use verdant_launcher::{is_flag, resolve, runtime_features, show_error};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && is_flag(&args[0], "--runtime-features") {
        println!("{}", runtime_features());
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && (is_flag(&args[0], "--help") || is_flag(&args[0], "-h")) {
        println!("Verdant graphical launcher\nUsage: verdant-launcher [PROJECT.verp | GUI OPTIONS]\nUse verdant for command-line help. This launcher opens the graphical editor.");
        return ExitCode::SUCCESS;
    }
    let result = resolve(None).and_then(|plan| {
        if args.len() == 1 && is_flag(&args[0], "--check") {
            println!("{}", plan.executable.display());
            return Ok(None);
        }
        plan.launch(&args, true).map(Some)
    });
    match result {
        Ok(Some(launched)) if launched.exit_code.is_some_and(|code| code != 0) => {
            show_error(&format!("The graphical editor exited unsuccessfully. See {}", launched.log.display()));
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            if args.len() == 1 && is_flag(&args[0], "--check") { eprintln!("Verdant: {error}"); }
            else { show_error(&error.to_string()); }
            ExitCode::FAILURE
        }
    }
}
