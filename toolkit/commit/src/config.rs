use std::{fs, io::ErrorKind, path::Path};

use serde::Deserialize;

use crate::{
    error::{AppError, Result},
    state::MessageFormat,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    message: MessageConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageConfig {
    format: MessageFormat,
}

pub fn message_format(repository_root: &Path) -> Result<MessageFormat> {
    let path = repository_root.join(".agents/commit.toml");
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(MessageFormat::Conventional),
        Err(error) => {
            return Err(AppError::usage(format!("cannot read config {}: {error}", path.display())));
        }
    };
    let config: Config = toml::from_str(&source)
        .map_err(|error| AppError::usage(format!("invalid config {}: {error}", path.display())))?;
    Ok(config.message.format)
}
