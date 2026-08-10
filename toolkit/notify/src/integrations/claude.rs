use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value, json};

use crate::{
    atomic_write,
    error::{AppError, Result},
};

use super::IntegrationStatus;

/// One Claude Code hook installed by ai-notify.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HookSpec {
    pub event: &'static str,
    pub command: &'static str,
    pub matcher: Option<&'static str>,
}

/// The single source of truth for installed Claude Code hooks.
pub const HOOK_SPECS: &[HookSpec] = &[
    HookSpec { event: "UserPromptSubmit", command: "ai-notify event user-prompt-submit", matcher: None },
    HookSpec { event: "Stop", command: "ai-notify event stop", matcher: None },
    HookSpec { event: "StopFailure", command: "ai-notify event stop-failure", matcher: None },
    HookSpec { event: "Notification", command: "ai-notify event notification", matcher: None },
    HookSpec { event: "PermissionRequest", command: "ai-notify event permission-request", matcher: None },
    HookSpec { event: "PreToolUse", command: "ai-notify event ask-user-question", matcher: Some("AskUserQuestion") },
];

/// The outcome of ensuring an individual Claude Code settings file has our hooks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeHooksUpdate {
    pub path: PathBuf,
    pub changed: bool,
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub skipped: BTreeMap<String, String>,
}

/// Aggregate state across Claude Code's active global and project settings files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeHooksReport {
    pub status: IntegrationStatus,
    pub paths: Vec<PathBuf>,
    pub missing_events: Vec<String>,
    pub errors: BTreeMap<PathBuf, String>,
    pub ignored_paths: Vec<PathBuf>,
}

impl ClaudeHooksReport {
    /// The first settings file that contributes an ai-notify command.
    pub fn path(&self) -> Option<&Path> {
        self.paths.first().map(PathBuf::as_path)
    }

    /// A CLI should use this to turn malformed inspected files into a non-zero result.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Add missing ai-notify Claude Code hooks while preserving all unrelated JSON data.
pub fn ensure_claude_hooks(path: &Path, force: bool, dry_run: bool) -> Result<ClaudeHooksUpdate> {
    let mut data = load_settings(path)?;
    let root = data.as_object_mut().expect("load_settings validates object roots");
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| AppError::integration(format!("{}: hooks field must be an object", path.display())))?;

    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut skipped = BTreeMap::new();

    for spec in HOOK_SPECS {
        match hooks.get_mut(spec.event) {
            Some(existing @ Value::Array(_)) => {
                if !command_present(existing, spec.command) {
                    existing.as_array_mut().expect("matched an array").push(build_group(*spec));
                    added.push(spec.event.to_owned());
                }
            }
            Some(existing) if command_present(existing, spec.command) => {
                *existing = Value::Array(vec![build_group(*spec)]);
                updated.push(spec.event.to_owned());
            }
            Some(existing) if force => {
                *existing = Value::Array(vec![build_group(*spec)]);
                updated.push(spec.event.to_owned());
            }
            Some(existing) => {
                skipped.insert(spec.event.to_owned(), summarize_hook(existing));
            }
            None => {
                hooks.insert(spec.event.to_owned(), Value::Array(vec![build_group(*spec)]));
                added.push(spec.event.to_owned());
            }
        }
    }

    let changed = !added.is_empty() || !updated.is_empty();
    if changed && !dry_run {
        let rendered = serde_json::to_string_pretty(&data)
            .map_err(|error| AppError::integration(format!("failed to render {}: {error}", path.display())))? +
            "\n";
        atomic_write::replace(path, rendered)?;
    }

    Ok(ClaudeHooksUpdate { path: path.to_path_buf(), changed, added, updated, skipped })
}

/// Inspect only Claude Code's documented global and project settings locations.
pub fn inspect_claude_hooks(config_root: &Path, project_root: &Path) -> ClaudeHooksReport {
    let active_paths = [
        config_root.join("settings.json"),
        project_root.join(".claude/settings.json"),
        project_root.join(".claude/settings.local.json"),
    ];
    let ignored_paths = [config_root.join("hooks/hooks.json"), config_root.join("settings.local.json")]
        .into_iter()
        .filter(|path| path.exists())
        .collect();

    let mut commands_by_event: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut errors = BTreeMap::new();
    let mut paths = Vec::new();

    for path in active_paths {
        if !path.exists() {
            continue;
        }

        let data = match load_settings(&path) {
            Ok(data) => data,
            Err(error) => {
                errors.insert(path, error.message);
                continue;
            }
        };
        let Some(hooks) = data.get("hooks") else {
            continue;
        };
        let Some(hooks) = hooks.as_object() else {
            errors.insert(path, "hooks field must be an object".to_owned());
            continue;
        };

        let mut path_has_command = false;
        for (event, value) in hooks {
            let commands = commands_by_event.entry(event.clone()).or_default();
            commands.extend(iter_hook_commands(value));
        }
        for spec in HOOK_SPECS {
            if hooks.get(spec.event).is_some_and(|value| has_ai_notify_event_command(value, spec.command)) {
                path_has_command = true;
            }
        }
        if path_has_command {
            paths.push(path);
        }
    }

    let missing_events = HOOK_SPECS
        .iter()
        .filter(|spec| {
            !commands_by_event.get(spec.event).is_some_and(|commands| {
                commands
                    .iter()
                    .any(|command| command.contains("ai-notify") && command.contains(&event_subcommand(spec.command)))
            })
        })
        .map(|spec| spec.event.to_owned())
        .collect::<Vec<_>>();
    let status = if missing_events.is_empty() {
        IntegrationStatus::Ok
    } else if paths.is_empty() {
        IntegrationStatus::Missing
    } else {
        IntegrationStatus::Partial
    };

    ClaudeHooksReport { status, paths, missing_events, errors, ignored_paths }
}

fn load_settings(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let text = fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&text)
        .map_err(|error| AppError::integration(format!("failed to parse {}: {error}", path.display())))?;
    if !data.is_object() {
        return Err(AppError::integration(format!("{} must contain a JSON object at the root", path.display())));
    }
    Ok(data)
}

fn build_group(spec: HookSpec) -> Value {
    let mut group = Map::new();
    if let Some(matcher) = spec.matcher {
        group.insert("matcher".to_owned(), Value::String(matcher.to_owned()));
    }
    group.insert("hooks".to_owned(), json!([{ "type": "command", "command": spec.command }]));
    Value::Object(group)
}

/// Recursively yield all command strings in Claude Code's accepted nested hook shapes.
pub fn iter_hook_commands(value: &Value) -> Vec<String> {
    let mut commands = Vec::new();
    collect_hook_commands(value, &mut commands);
    commands
}

fn collect_hook_commands(value: &Value, commands: &mut Vec<String>) {
    match value {
        Value::String(command) => commands.push(command.clone()),
        Value::Array(items) => {
            for item in items {
                collect_hook_commands(item, commands);
            }
        }
        Value::Object(object) => {
            if let Some(Value::String(command)) = object.get("command") {
                commands.push(command.clone());
            }
            if let Some(hooks) = object.get("hooks") {
                collect_hook_commands(hooks, commands);
            }
        }
        _ => {}
    }
}

fn command_present(value: &Value, expected: &str) -> bool {
    iter_hook_commands(value).iter().any(|command| command.trim() == expected)
}

fn has_ai_notify_event_command(value: &Value, command: &str) -> bool {
    let subcommand = event_subcommand(command);
    iter_hook_commands(value).iter().any(|candidate| candidate.contains("ai-notify") && candidate.contains(&subcommand))
}

fn event_subcommand(command: &str) -> String {
    command.strip_prefix("ai-notify event ").unwrap_or(command).to_owned()
}

fn summarize_hook(value: &Value) -> String {
    match value {
        Value::String(command) => command.clone(),
        Value::Object(object) => object
            .get("command")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "<object>".to_owned()),
        Value::Array(items) => format!("<list:{}>", items.len()),
        _ => "<unknown>".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn installs_all_hooks_with_the_pre_tool_matcher() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");

        let update = ensure_claude_hooks(&path, false, false).unwrap();
        let settings: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();

        assert_eq!(update.added.len(), HOOK_SPECS.len());
        assert_eq!(settings["hooks"]["PreToolUse"][0]["matcher"], "AskUserQuestion");
        assert_eq!(settings["hooks"]["Stop"][0]["hooks"][0]["type"], "command");
        assert!(fs::read_to_string(path).unwrap().ends_with('\n'));
    }

    #[test]
    fn preserves_list_groups_and_migrates_trimmed_legacy_commands() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(
            &path,
            serde_json::to_string(&json!({
                "model": "opus",
                "hooks": {
                    "Stop": [{"hooks": [{"type": "command", "command": "echo mine"}]}],
                    "Notification": {"command": " ai-notify event notification  "}
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let update = ensure_claude_hooks(&path, false, false).unwrap();
        let settings: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

        assert!(update.added.contains(&"Stop".to_owned()));
        assert!(update.updated.contains(&"Notification".to_owned()));
        assert_eq!(settings["model"], "opus");
        assert_eq!(iter_hook_commands(&settings["hooks"]["Stop"]).len(), 2);
    }

    #[test]
    fn skips_foreign_non_lists_unless_forced_and_never_writes_invalid_json() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, r#"{"hooks":{"Stop":{"command":"echo stop"}}}"#).unwrap();

        let update = ensure_claude_hooks(&path, false, false).unwrap();
        assert_eq!(update.skipped["Stop"], "echo stop");
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap()["hooks"]["Stop"]["command"],
            "echo stop"
        );

        fs::write(&path, "{").unwrap();
        assert!(ensure_claude_hooks(&path, false, false).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "{");
    }

    #[test]
    fn inspector_merges_only_active_locations_and_reports_stale_ones() {
        let directory = tempdir().unwrap();
        let config = directory.path().join("home/.claude");
        let project = directory.path().join("project");
        fs::create_dir_all(config.join("hooks")).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::write(config.join("hooks/hooks.json"), "{}").unwrap();
        fs::write(config.join("settings.local.json"), "{}").unwrap();

        let groups = |specs: &[HookSpec]| {
            let mut hooks = Map::new();
            for spec in specs {
                hooks.insert(spec.event.to_owned(), Value::Array(vec![build_group(*spec)]));
            }
            Value::Object(Map::from_iter([(String::from("hooks"), Value::Object(hooks))]))
        };
        fs::write(config.join("settings.json"), serde_json::to_string(&groups(&HOOK_SPECS[..2])).unwrap()).unwrap();
        fs::write(project.join(".claude/settings.json"), serde_json::to_string(&groups(&HOOK_SPECS[2..4])).unwrap())
            .unwrap();
        fs::write(
            project.join(".claude/settings.local.json"),
            serde_json::to_string(&groups(&HOOK_SPECS[4..])).unwrap(),
        )
        .unwrap();

        let report = inspect_claude_hooks(&config, &project);
        assert_eq!(report.status, IntegrationStatus::Ok);
        assert_eq!(report.ignored_paths.len(), 2);
    }
}
