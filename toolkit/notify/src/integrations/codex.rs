use std::{
    fs,
    path::{Path, PathBuf},
};

use toml_edit::{Array, DocumentMut, Item, Value};

use crate::{
    atomic_write,
    error::{AppError, Result},
};

use super::IntegrationStatus;

/// The callback command installed in Codex's root `notify` key.
pub const CODEX_NOTIFY_COMMAND: &[&str] = &["ai-notify", "codex"];

/// The result of changing a Codex notify setting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexNotifyUpdate {
    pub path: PathBuf,
    pub changed: bool,
    pub profile: Option<String>,
    pub conflict: bool,
    /// The previous root `notify` value, rendered as TOML for useful CLI reporting.
    pub previous_notify: Option<String>,
}

/// The effective Codex notify setting after applying a selected profile overlay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexNotifyReport {
    pub status: IntegrationStatus,
    pub path: Option<PathBuf>,
    /// The effective value, rendered as TOML.
    pub notify: Option<String>,
    pub error: Option<String>,
    pub profile: Option<String>,
    pub paths: Vec<PathBuf>,
}

impl CodexNotifyReport {
    /// Whether parsing or profile resolution failed and a CLI must exit non-zero.
    pub fn has_error(&self) -> bool {
        self.status == IntegrationStatus::Error
    }
}

/// Validate a Codex profile name, preventing profile paths from escaping the config directory.
pub fn validate_codex_profile_name(profile: &str) -> Result<&str> {
    if !profile.is_empty() && profile.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')) {
        Ok(profile)
    } else {
        Err(AppError::usage("Codex profile names may contain only letters, numbers, hyphens, and underscores"))
    }
}

/// Resolve a base config file to the selected sibling profile config file.
pub fn resolve_codex_config_path(config_path: &Path, profile: Option<&str>) -> Result<PathBuf> {
    match profile {
        None => Ok(config_path.to_path_buf()),
        Some(profile) => {
            validate_codex_profile_name(profile)?;
            Ok(config_path.with_file_name(format!("{profile}.config.toml")))
        }
    }
}

/// Set Codex's exact root `notify` array, preserving unrelated TOML syntax and comments.
pub fn set_codex_notify(
    config_path: &Path,
    command: &[&str],
    profile: Option<&str>,
    force: bool,
) -> Result<CodexNotifyUpdate> {
    let target_path = resolve_codex_config_path(config_path, profile)?;
    let mut document = load_document(&target_path)?;
    let previous = document.get("notify").map(render_item);

    if let Some(item) = document.get("notify") {
        if notify_equals(item, command) {
            return Ok(CodexNotifyUpdate {
                path: target_path,
                changed: false,
                profile: profile.map(ToOwned::to_owned),
                conflict: false,
                previous_notify: previous,
            });
        }
        if !force {
            return Ok(CodexNotifyUpdate {
                path: target_path,
                changed: false,
                profile: profile.map(ToOwned::to_owned),
                conflict: true,
                previous_notify: previous,
            });
        }
    }

    // Replacing the value would otherwise drop an inline comment attached to the old value.
    let decor = document.get("notify").and_then(Item::as_value).map(|value| value.decor().clone());
    let mut replacement = command_array(command);
    if let Some(decor) = decor {
        *replacement.decor_mut() = decor;
    }
    document["notify"] = Item::Value(replacement);
    atomic_write::replace(&target_path, document.to_string())?;
    Ok(CodexNotifyUpdate {
        path: target_path,
        changed: true,
        profile: profile.map(ToOwned::to_owned),
        conflict: previous.is_some(),
        previous_notify: previous,
    })
}

/// Inspect Codex's base config and, when requested, its mandatory profile overlay.
pub fn inspect_codex_notify(config_root: &Path, profile: Option<&str>) -> CodexNotifyReport {
    let base_path = config_root.join("config.toml");
    let profile_path = match resolve_codex_config_path(&base_path, profile) {
        Ok(path) => path,
        Err(error) => {
            return CodexNotifyReport {
                status: IntegrationStatus::Error,
                path: None,
                notify: None,
                error: Some(error.message),
                profile: profile.map(ToOwned::to_owned),
                paths: Vec::new(),
            };
        }
    };
    let layer_paths =
        if profile.is_some() { vec![base_path.clone(), profile_path.clone()] } else { vec![base_path.clone()] };
    let mut loaded_paths = Vec::new();
    let mut notify = None;
    let mut notify_path = None;

    if profile.is_some() && !profile_path.exists() {
        return CodexNotifyReport {
            status: IntegrationStatus::Error,
            path: Some(profile_path.clone()),
            notify: None,
            error: Some(format!("Profile config not found: {}", profile_path.display())),
            profile: profile.map(ToOwned::to_owned),
            paths: if base_path.exists() { vec![base_path] } else { Vec::new() },
        };
    }

    for path in layer_paths {
        if !path.exists() {
            continue;
        }
        let document = match load_document(&path) {
            Ok(document) => document,
            Err(error) => {
                loaded_paths.push(path.clone());
                return CodexNotifyReport {
                    status: IntegrationStatus::Error,
                    path: Some(path),
                    notify: None,
                    error: Some(error.message),
                    profile: profile.map(ToOwned::to_owned),
                    paths: loaded_paths,
                };
            }
        };
        loaded_paths.push(path.clone());
        if let Some(item) = document.get("notify") {
            notify = Some(render_item(item));
            notify_path = Some(path);
        }
    }

    let status = match notify.as_deref() {
        None => IntegrationStatus::Missing,
        Some(value) if notify_uses_ai_notify(value) => IntegrationStatus::Ok,
        Some(_) => IntegrationStatus::Partial,
    };
    CodexNotifyReport {
        status,
        path: notify_path.or_else(|| loaded_paths.last().cloned()),
        notify,
        error: None,
        profile: profile.map(ToOwned::to_owned),
        paths: loaded_paths,
    }
}

fn load_document(path: &Path) -> Result<DocumentMut> {
    if !path.exists() {
        return Ok(DocumentMut::new());
    }
    let text = fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(DocumentMut::new());
    }
    text.parse::<DocumentMut>()
        .map_err(|error| AppError::integration(format!("failed to parse {}: {error}", path.display())))
}

fn command_array(command: &[&str]) -> Value {
    let mut array = Array::new();
    for part in command {
        array.push(*part);
    }
    Value::Array(array)
}

fn notify_equals(item: &Item, command: &[&str]) -> bool {
    let Some(array) = item.as_value().and_then(Value::as_array) else {
        return false;
    };
    array.len() == command.len() &&
        array.iter().zip(command).all(|(actual, expected)| actual.as_str() == Some(*expected))
}

fn render_item(item: &Item) -> String {
    item.to_string().trim().to_owned()
}

// The historical checker intentionally used a permissive substring check. Retain it for
// configuration written by wrapper scripts that represent notify as a string or TOML array.
fn notify_uses_ai_notify(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("ai-notify") && normalized.contains("codex")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn inserts_root_notify_before_tables_and_preserves_comments() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "# model selection\nmodel = \"gpt-5.6\"\n\n[features]\n# keep\nshell_snapshot = true\n")
            .unwrap();

        let update = set_codex_notify(&path, CODEX_NOTIFY_COMMAND, None, false).unwrap();
        let output = fs::read_to_string(&path).unwrap();

        assert!(update.changed);
        assert!(output.contains("# model selection"));
        assert!(output.contains("# keep"));
        assert!(output.find("notify = [\"ai-notify\", \"codex\"]").unwrap() < output.find("[features]").unwrap());
    }

    #[test]
    fn refuses_conflicts_then_force_replaces_only_root_notify() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "# keep\nnotify = [\"other\"] # old\n\n[features]\nnotify = [\"nested\"]\n").unwrap();

        let conflict = set_codex_notify(&path, CODEX_NOTIFY_COMMAND, None, false).unwrap();
        assert!(conflict.conflict);
        assert!(!conflict.changed);
        let update = set_codex_notify(&path, CODEX_NOTIFY_COMMAND, None, true).unwrap();
        let output = fs::read_to_string(&path).unwrap();
        assert!(update.changed);
        assert!(output.contains("# keep"));
        assert!(output.contains("# old"));
        assert!(output.contains("notify = [\"nested\"]"));
    }

    #[test]
    fn profile_is_a_sibling_and_overlay_can_override_base() {
        let directory = tempdir().unwrap();
        let root = directory.path().join(".codex");
        fs::create_dir(&root).unwrap();
        let base = root.join("config.toml");
        fs::write(&base, "notify = [\"ai-notify\", \"codex\"]\n").unwrap();
        fs::write(root.join("review.config.toml"), "notify = [\"other\"]\n").unwrap();

        let report = inspect_codex_notify(&root, Some("review"));
        assert_eq!(report.status, IntegrationStatus::Partial);
        assert_eq!(report.path, Some(root.join("review.config.toml")));
        assert_eq!(report.paths, vec![base, root.join("review.config.toml")]);
    }

    #[test]
    fn malformed_toml_and_invalid_profiles_are_reported_without_writing() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "notify = [\"unterminated\"\n").unwrap();

        assert!(set_codex_notify(&path, CODEX_NOTIFY_COMMAND, None, true).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "notify = [\"unterminated\"\n");
        let report = inspect_codex_notify(directory.path(), Some("bad.profile"));
        assert!(report.has_error());
        assert!(report.error.unwrap().contains("letters, numbers"));
    }
}
