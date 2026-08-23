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
    #[serde(default)]
    validation: Option<ValidationConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageConfig {
    format: MessageFormat,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationConfig {
    command: Vec<String>,
}

#[derive(Debug)]
pub struct RepositoryConfig {
    pub message_format: MessageFormat,
    pub validation_command: Option<Vec<String>>,
}

pub fn load(repository_root: &Path, message_override: Option<MessageFormat>) -> Result<RepositoryConfig> {
    let path = repository_root.join(".agents/commit.toml");
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(RepositoryConfig {
                message_format: message_override.unwrap_or(MessageFormat::Conventional),
                validation_command: None,
            });
        }
        Err(error) => {
            return Err(AppError::usage(format!("cannot read config {}: {error}", path.display())));
        }
    };
    let config: Config = match toml::from_str(&source) {
        Ok(config) => config,
        Err(_) if message_override.is_some() && !declares_validation(&source) => {
            return Ok(RepositoryConfig {
                message_format: message_override.expect("checked message override"),
                validation_command: None,
            });
        }
        Err(error) => {
            return Err(AppError::usage(format!("invalid config {}: {error}", path.display())));
        }
    };
    let validation_command =
        config.validation.map(|validation| validate_command(&path, validation.command)).transpose()?;
    Ok(RepositoryConfig { message_format: message_override.unwrap_or(config.message.format), validation_command })
}

fn declares_validation(source: &str) -> bool {
    source.lines().any(|line| {
        let content = line.split_once('#').map_or(line, |(content, _)| content);
        let compact = content.chars().filter(|character| !character.is_whitespace()).collect::<String>();
        let key = compact.trim_start_matches('[');
        ["validation", "\"validation\"", "'validation'"].iter().any(|prefix| {
            key.strip_prefix(prefix).is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '=', ']']))
        })
    })
}

fn validate_command(path: &Path, command: Vec<String>) -> Result<Vec<String>> {
    if command.is_empty() {
        return Err(AppError::usage(format!(
            "invalid config {}: validation.command must contain at least one argv element",
            path.display()
        )));
    }
    if command.iter().any(String::is_empty) {
        return Err(AppError::usage(format!(
            "invalid config {}: validation.command argv elements may not be empty",
            path.display()
        )));
    }
    if command.iter().any(|argument| argument.contains('\0')) {
        return Err(AppError::usage(format!(
            "invalid config {}: validation.command argv elements may not contain NUL bytes",
            path.display()
        )));
    }
    Ok(command)
}
