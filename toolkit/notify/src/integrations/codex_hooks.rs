use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value, json};

use crate::{
    atomic_write,
    error::{AppError, Result},
};

use super::codex::{CODEX_HOOK_EVENTS, CodexHooksReport};

/// One passive native Codex hook installed by ai-notify.
#[derive(Clone, Copy, Debug)]
pub struct CodexHookSpec {
    pub event: &'static str,
    pub matcher: Option<&'static str>,
    pub timeout: u8,
}

pub const CODEX_HOOK_SPECS: &[CodexHookSpec] = &[
    CodexHookSpec { event: "UserPromptSubmit", matcher: None, timeout: 5 },
    CodexHookSpec { event: "Stop", matcher: None, timeout: 5 },
    CodexHookSpec { event: "PermissionRequest", matcher: None, timeout: 5 },
    CodexHookSpec { event: "PreToolUse", matcher: Some("^request_user_input$"), timeout: 5 },
    CodexHookSpec { event: "Interrupt", matcher: None, timeout: 1 },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexHooksUpdate {
    pub path: PathBuf,
    pub added: Vec<String>,
    pub consolidated: Vec<String>,
}

/// Add missing coverage, consolidating identical owned handlers without changing Codex trust settings.
pub fn ensure_codex_hooks(path: &Path, dry_run: bool) -> Result<CodexHooksUpdate> {
    let mut data = load_hooks(path)?;
    let root = data.as_object_mut().expect("load_hooks validates root");
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().expect("load_hooks validates hooks");
    let mut added = Vec::new();
    let mut consolidated = Vec::new();
    for spec in CODEX_HOOK_SPECS {
        let groups = hooks.entry(spec.event).or_insert_with(|| Value::Array(Vec::new()));
        if spec_present(groups, spec) {
            continue;
        }
        let existing = groups
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|group| group.get("hooks").and_then(Value::as_array).into_iter().flatten())
            .filter(|hook| runs_codex_hook(hook))
            .cloned()
            .collect::<Vec<_>>();
        if existing.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(AppError::operational(format!(
                "{}: conflicting ai-notify definitions for {}; make their command and options identical before rerunning link codex-hooks",
                path.display(),
                spec.event
            )));
        }
        let handler = existing
            .first()
            .cloned()
            .unwrap_or_else(|| json!({"type": "command", "command": "ai-notify event codex", "timeout": spec.timeout}));
        let mut group = json!({"hooks": [handler]});
        if let Some(matcher) = spec.matcher {
            group["matcher"] = Value::String(matcher.to_owned());
        }
        let groups = groups.as_array_mut().expect("load_hooks validates event groups");
        if existing.is_empty() {
            added.push(spec.event.to_owned());
        } else {
            for group in groups.iter_mut() {
                if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                    handlers.retain(|hook| !runs_codex_hook(hook));
                }
            }
            consolidated.push(spec.event.to_owned());
        }
        groups.push(group);
    }
    if (!added.is_empty() || !consolidated.is_empty()) && !dry_run {
        atomic_write::replace(path, serde_json::to_string_pretty(&data)? + "\n")?;
    }
    Ok(CodexHooksUpdate { path: path.to_path_buf(), added, consolidated })
}

pub(super) fn inspect_codex_hooks(config_root: &Path) -> CodexHooksReport {
    let path = config_root.join("hooks.json");
    let (installed, error) = match load_hooks(&path) {
        Ok(data) => (
            CODEX_HOOK_SPECS
                .iter()
                .filter(|spec| {
                    data.get("hooks")
                        .and_then(|hooks| hooks.get(spec.event))
                        .is_some_and(|groups| spec_present(groups, spec))
                })
                .map(|spec| spec.event.to_owned())
                .collect::<Vec<_>>(),
            None,
        ),
        Err(error) => (Vec::new(), Some(error.message)),
    };
    let coverage = |optional: bool, present: bool| {
        CODEX_HOOK_SPECS
            .iter()
            .filter(|spec| CODEX_HOOK_EVENTS.contains(&spec.event) != optional)
            .filter(|spec| installed.iter().any(|event| event == spec.event) == present)
            .map(|spec| spec.event.to_owned())
            .collect()
    };
    CodexHooksReport {
        path,
        installed_events: coverage(false, true),
        missing_events: coverage(false, false),
        optional_installed_events: coverage(true, true),
        optional_missing_events: coverage(true, false),
        error,
    }
}

fn spec_present(groups: &Value, spec: &CodexHookSpec) -> bool {
    groups.as_array().into_iter().flatten().any(|group| {
        let matcher = group.get("matcher").and_then(Value::as_str);
        let matcher_covers = match spec.event {
            // Codex ignores matchers for these lifecycle events.
            "UserPromptSubmit" | "Stop" | "Interrupt" => true,
            "PermissionRequest" => all_tools(matcher),
            "PreToolUse" => {
                all_tools(matcher) || matches!(matcher, Some("request_user_input" | "^request_user_input$"))
            }
            _ => false,
        };
        matcher_covers && group.get("hooks").and_then(Value::as_array).into_iter().flatten().any(runs_codex_hook)
    })
}

fn runs_codex_hook(hook: &Value) -> bool {
    hook.get("type").and_then(Value::as_str) == Some("command") &&
        hook.get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| crate::cli::codex_hook_argv_runs(&command.split_whitespace().collect::<Vec<_>>()))
}

fn all_tools(matcher: Option<&str>) -> bool {
    matches!(matcher, None | Some("" | "*" | ".*" | "^.*$"))
}

fn load_hooks(path: &Path) -> Result<Value> {
    let data = match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str::<Value>(&text)
            .map_err(|error| AppError::operational(format!("failed to parse {}: {error}", path.display())))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(error) => return Err(error.into()),
    };
    let invalid = |detail: &str| AppError::operational(format!("{}: {detail}", path.display()));
    let root = data.as_object().ok_or_else(|| invalid("must contain a JSON object at the root"))?;
    if let Some(hooks) = root.get("hooks") {
        let hooks = hooks.as_object().ok_or_else(|| invalid("hooks field must be an object"))?;
        for (event, groups) in hooks {
            let groups = groups.as_array().ok_or_else(|| invalid(&format!("hooks.{event} must be an array")))?;
            for group in groups {
                let group =
                    group.as_object().ok_or_else(|| invalid(&format!("hooks.{event} groups must be objects")))?;
                if group.get("matcher").is_some_and(|value| !value.is_null() && !value.is_string()) {
                    return Err(invalid(&format!("hooks.{event} matcher must be a string or null")));
                }
                if let Some(handlers) = group.get("hooks") {
                    let handlers = handlers
                        .as_array()
                        .ok_or_else(|| invalid(&format!("hooks.{event} group hooks must be an array")))?;
                    if handlers.iter().any(|handler| !handler.is_object()) {
                        return Err(invalid(&format!("hooks.{event} handlers must be objects")));
                    }
                }
            }
        }
    }
    Ok(data)
}
