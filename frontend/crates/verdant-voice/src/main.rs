use std::io::{self, Write};

fn main() {
    let result = verdant_voice::execute(std::env::args_os().skip(1));
    let outcome = match result {
        Ok(value) => {
            let mut output = io::stdout().lock();
            if let Some(help) = value.get("help").and_then(serde_json::Value::as_str) {
                output.write_all(help.as_bytes())
            } else {
                serde_json::to_writer_pretty(&mut output, &value).map_err(io::Error::other)
                    .and_then(|_| output.write_all(b"\n"))
            }
        }
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "verdant-voice command failed: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = outcome {
        let _ = writeln!(io::stderr().lock(), "verdant-voice output failed: {error}");
        std::process::exit(1);
    }
}
