use std::{ffi::OsString, io::{self, Write}, path::PathBuf};
use serde_json::json;
use crate::{CliError, Mode, invalid, write_response};

pub(crate) fn run(args: &[OsString], writer: &mut impl Write) -> io::Result<u8> {
    let mut wait = false;
    let mut check = false;
    let mut mode = Mode::Text;
    let mut binary = None;
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        if arg == "--" { index += 1; break; }
        if arg == "--help" || arg == "-h" {
            writer.write_all(verdant_launcher::HELP.as_bytes())?;
            return Ok(0);
        } else if arg == "--wait" {
            wait = true;
        } else if arg == "--check" {
            check = true;
        } else if arg == "--json" || arg == "--events" {
            if mode != Mode::Text {
                return write_response(writer, None, Err(invalid("choose only one of --json or --events")), mode);
            }
            mode = if arg == "--json" { Mode::Json } else { Mode::Events };
        } else if arg == "--gui-binary" {
            index += 1;
            let Some(path) = args.get(index) else {
                return write_response(writer, None, Err(invalid("--gui-binary requires a path")), mode);
            };
            if binary.replace(PathBuf::from(path)).is_some() {
                return write_response(writer, None, Err(invalid("--gui-binary may only occur once")), mode);
            }
        } else {
            break;
        }
        index += 1;
    }
    let outcome = (|| {
        if wait && check { return Err(invalid("choose only one of --wait or --check")); }
        let plan = verdant_launcher::resolve(binary.as_deref())
            .map_err(|error| CliError::new("GUI_UNAVAILABLE", error.to_string()))?;
        if check {
            return Ok(json!({"executable":plan.executable,"launched":false,"wait":false}));
        }
        let launched = plan.launch(&args[index..], wait)
            .map_err(|error| CliError::new("IO_ERROR", error.to_string()))?;
        let result = json!({"executable":plan.executable,"launched":true,"pid":launched.pid,
            "wait":wait,"exit_code":launched.exit_code,"log":launched.log});
        if launched.exit_code.is_some_and(|code| code != 0) {
            return Err(CliError::new("GUI_EXITED", "GUI exited unsuccessfully; see its log").with_details(result));
        }
        Ok(result)
    })();
    write_response(writer, None, outcome, mode)
}
