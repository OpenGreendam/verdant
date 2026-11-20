use std::{collections::BTreeMap, ffi::OsString, path::{Path, PathBuf}};

use crate::{Error, Result, files, platform, require};

pub(crate) enum Action {
    Help,
    Version,
    Convert { source: PathBuf, destination: PathBuf, token: PathBuf, model: PathBuf },
    Install { input: PathBuf, registry: PathBuf, set_default: bool, policy: Option<ReviewedPolicy> },
    List { registry: PathBuf },
    Remove { registry: PathBuf, reference: String },
    SetDefault { registry: PathBuf, reference: String },
    ImportOriginals { registry: PathBuf, token: PathBuf, model: PathBuf },
}

pub(crate) struct ReviewedPolicy {
    pub path: PathBuf,
    pub sha256: String,
}

fn required(options: &mut BTreeMap<String, OsString>, name: &str) -> Result<PathBuf> {
    options.remove(name).map(PathBuf::from).ok_or_else(|| Error::Invalid(format!("--{name} is required")))
}

pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Action> {
    let mut args = args.into_iter().peekable();
    let command = args.next().unwrap_or_default().into_string().map_err(|_| Error::Invalid("invalid voice command".into()))?;
    let allowed: &[&str] = match command.as_str() {
        "" | "--help" | "-h" | "help" => {
            require(args.next().is_none(), "help takes no options")?;
            return Ok(Action::Help);
        }
        "--version" | "-V" => {
            require(args.next().is_none(), "version takes no options")?;
            return Ok(Action::Version);
        }
        "install" => &["registry", "set-default", "artifact-policy", "artifact-policy-sha256"],
        "list" | "remove" | "set-default" => &["registry"],
        "import-originals" => &["registry", "token", "model"],
        "convert" => &["source", "destination", "source-token", "source-model"],
        _ => return Err(Error::Invalid(format!("unknown voice command: {command}"))),
    };

    let mut options = BTreeMap::new();
    let mut positional = None;
    while let Some(arg) = args.next() {
        if let Some(text) = arg.to_str() {
            if matches!(text, "--help" | "-h") { return Ok(Action::Help); }
            if let Some(option) = text.strip_prefix("--") {
                let (name, inline) = option.split_once('=').map_or((option, None), |(key, value)| (key, Some(value)));
                require(allowed.contains(&name), format!("unknown {command} option: --{name}"))?;
                let value = if name == "set-default" {
                    require(inline.is_none(), "--set-default takes no value")?;
                    OsString::new()
                } else {
                    let inline = inline.map(OsString::from);
                    let value = inline.or_else(|| args.next_if(|arg| arg.to_str().is_none_or(|text| !text.starts_with("--"))))
                        .ok_or_else(|| Error::Invalid(format!("--{name} needs a value")))?;
                    require(!value.is_empty(), format!("--{name} cannot be empty"))?;
                    value
                };
                require(options.insert(name.to_owned(), value).is_none(), format!("duplicate option: --{name}"))?;
                continue;
            }
            require(!text.starts_with('-'), format!("unknown {command} option: {text}"))?;
        }
        require(positional.replace(arg).is_none(), format!("{command} accepts one positional argument"))?;
    }
    let needs_positional = matches!(command.as_str(), "install" | "remove" | "set-default");
    require(needs_positional == positional.is_some(), if needs_positional {
        format!("{command} requires {}", if command == "install" { "an input package" } else { "an installed reference" })
    } else { format!("{command} takes no positional arguments") })?;

    if command == "convert" {
        return Ok(Action::Convert {
            source: required(&mut options, "source")?, destination: required(&mut options, "destination")?,
            token: required(&mut options, "source-token")?, model: required(&mut options, "source-model")?,
        });
    }
    let registry = match options.remove("registry") {
        Some(value) => value.into(),
        None => platform::default_registry()?,
    };
    match command.as_str() {
        "install" => {
            let policy = match (options.remove("artifact-policy"), options.remove("artifact-policy-sha256")) {
                (Some(path), Some(sha256)) => {
                    let sha256 = sha256.into_string().map_err(|_| Error::Invalid("artifact policy digest must be UTF-8".into()))?;
                    require(verdant_voice_format::valid_sha256(&sha256), "artifact policy digest must be lowercase SHA-256")?;
                    Some(ReviewedPolicy { path: path.into(), sha256 })
                }
                (None, None) => None,
                _ => return Err(Error::Invalid("use --artifact-policy and --artifact-policy-sha256 together".into())),
            };
            let input: PathBuf = positional.unwrap().into();
            require(!input.as_os_str().is_empty(), "package input path cannot be empty")?;
            Ok(Action::Install { input, registry, set_default: options.contains_key("set-default"), policy })
        }
        "list" => Ok(Action::List { registry }),
        "remove" | "set-default" => {
            let reference = positional.unwrap().into_string().map_err(|_| Error::Invalid("reference must be UTF-8".into()))?;
            require(!reference.trim().is_empty() && !reference.chars().any(char::is_control), "invalid voice reference")?;
            if command == "remove" { Ok(Action::Remove { registry, reference }) }
            else { Ok(Action::SetDefault { registry, reference }) }
        }
        "import-originals" => Ok(Action::ImportOriginals { registry, token: required(&mut options, "token")?, model: required(&mut options, "model")? }),
        _ => unreachable!("validated command"),
    }
}

pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    let path = std::path::absolute(platform::expand_user(path)?)?;
    files::path_text(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Action> { parse(args.iter().copied().map(OsString::from)) }

    #[test]
    fn install_accepts_package_and_explicit_default_only() {
        let Action::Install { input, registry, set_default, policy } = parse_args(&["install", "voice.zip", "--registry=voices.json", "--set-default"]).unwrap()
            else { panic!("install action"); };
        assert_eq!(input, Path::new("voice.zip"));
        assert_eq!(registry, Path::new("voices.json"));
        assert!(set_default);
        assert!(policy.is_none());
        assert!(parse_args(&["install", "voice.zip", "--worker", "worker"]).is_err());
        assert!(parse_args(&["install", "--set-default"]).is_err());
    }

    #[test]
    fn install_policy_requires_external_file_and_reviewed_digest() {
        let prefix = ["install", "voice.zip", "--registry", "voices.json"];
        let digest = "a".repeat(64);
        let parse_with = |extra: &[&str]| parse(prefix.into_iter().chain(extra.iter().copied()).map(OsString::from));
        assert!(parse_with(&["--artifact-policy", "policy.json"]).is_err());
        assert!(parse_with(&["--artifact-policy-sha256", &digest]).is_err());
        assert!(parse_with(&["--artifact-policy", "policy.json", "--artifact-policy-sha256", "invalid"]).is_err());
        let Action::Install { policy: Some(policy), .. } = parse_with(&["--artifact-policy", "policy.json", "--artifact-policy-sha256", &digest]).unwrap()
            else { panic!("reviewed policy option"); };
        assert_eq!(policy.sha256, digest);
        assert_eq!(policy.path, Path::new("policy.json"));
    }

    #[test]
    fn management_commands_and_original_import_have_distinct_inputs() {
        assert!(matches!(parse_args(&["list", "--registry", "voices.json"]).unwrap(), Action::List { .. }));
        assert!(matches!(parse_args(&["remove", "ref", "--registry", "voices.json"]).unwrap(), Action::Remove { .. }));
        assert!(matches!(parse_args(&["set-default", "ref", "--registry", "voices.json"]).unwrap(), Action::SetDefault { .. }));
        assert!(matches!(parse_args(&["import-originals", "--token", "voice.tkn", "--model", "voice.bin", "--registry", "voices.json"]).unwrap(), Action::ImportOriginals { .. }));
        assert!(parse_args(&["import-originals", "--token", "voice.tkn", "--set-default"]).is_err());
        assert!(parse_args(&["import-originals", "--token", "voice.tkn", "--registry", "voices.json"]).is_err());
    }

    #[test]
    fn convert_requires_token_and_model_verification_inputs() {
        assert!(matches!(parse_args(&["convert", "--source", "legacy", "--destination", "new", "--source-token", "token", "--source-model", "model"]).unwrap(), Action::Convert { .. }));
        assert!(parse_args(&["convert", "--source", "legacy", "--destination", "new"]).is_err());
    }
}
