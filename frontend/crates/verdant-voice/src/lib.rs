//! Local voice packages and atomic registry management.

mod args;
mod files;
mod installation;
mod platform;
mod strict_json;

pub mod extract;

use std::ffi::OsString;
use serde_json::Value;

pub const HELP: &str = include_str!("help.txt");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition { Ok(()) } else { Err(Error::Invalid(message.into())) }
}

/// Execute a voice command. Arguments exclude the program name.
/// Returns the operation result as JSON.
pub fn execute(args: impl IntoIterator<Item = OsString>) -> Result<Value> {
    match args::parse(args)? {
        args::Action::Help => Ok(serde_json::json!({"schema":"verdant-voice-help/1", "help":HELP})),
        args::Action::Version => Ok(serde_json::json!({"name":"verdant-voice", "version":env!("CARGO_PKG_VERSION")})),
        args::Action::Convert { source, destination, token, model } => verdant_synthesis::model::convert_voice_package(
            &args::absolute(&source)?, &args::absolute(&destination)?, &args::absolute(&token)?, &args::absolute(&model)?)
            .map_err(|error| Error::Invalid(format!("voice package conversion failed: {error:#}"))),
        args::Action::Install { input, registry, set_default, policy } => installation::install(&input, &registry, set_default, policy.as_ref()),
        args::Action::List { registry } => installation::list(&registry),
        args::Action::Remove { registry, reference } => installation::remove(&registry, &reference),
        args::Action::SetDefault { registry, reference } => installation::set_default(&registry, &reference),
        args::Action::ImportOriginals { registry, token, model } => installation::import_originals(&registry, &token, &model),
    }
}
