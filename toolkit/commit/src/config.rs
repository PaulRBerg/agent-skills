use std::{env, fs, path::PathBuf};

use serde::Deserialize;

use crate::error::{AppError, Result};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    message: MessageConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageConfig {
    #[serde(default)]
    natural_repositories: Vec<String>,
}

pub fn repository_uses_natural_format(repository_root: &std::path::Path) -> Result<bool> {
    let Some(path) = config_path()? else {
        return Ok(false);
    };
    if !path.exists() {
        return Ok(false);
    }
    let source = fs::read_to_string(&path)
        .map_err(|error| AppError::usage(format!("cannot read config {}: {error}", path.display())))?;
    let config: Config = toml::from_str(&source)
        .map_err(|error| AppError::usage(format!("invalid config {}: {error}", path.display())))?;
    let home = env::var_os("HOME").map(PathBuf::from);

    for configured in config.message.natural_repositories {
        if configured.chars().any(char::is_control) {
            return Err(AppError::usage("natural repository paths may not contain control characters"));
        }
        let expanded = if configured == "~" {
            home.clone().ok_or_else(|| AppError::usage("HOME is required to expand '~' in config"))?
        } else if let Some(rest) = configured.strip_prefix("~/") {
            home.clone().ok_or_else(|| AppError::usage("HOME is required to expand '~/' in config"))?.join(rest)
        } else {
            PathBuf::from(configured)
        };
        let Ok(canonical) = expanded.canonicalize() else {
            continue;
        };
        if canonical == repository_root {
            return Ok(true);
        }
    }
    Ok(false)
}

fn config_path() -> Result<Option<PathBuf>> {
    if let Some(value) = env::var_os("AI_COMMIT_CONFIG") {
        if value.is_empty() {
            return Err(AppError::usage("AI_COMMIT_CONFIG may not be empty"));
        }
        return Ok(Some(PathBuf::from(value)));
    }
    if let Some(value) = env::var_os("XDG_CONFIG_HOME") {
        if value.is_empty() {
            return Err(AppError::usage("XDG_CONFIG_HOME may not be empty"));
        }
        return Ok(Some(PathBuf::from(value).join("ai-commit/config.toml")));
    }
    let Some(home) = env::var_os("HOME") else {
        return Ok(None);
    };
    Ok(Some(PathBuf::from(home).join(".config/ai-commit/config.toml")))
}
