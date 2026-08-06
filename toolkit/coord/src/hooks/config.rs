//! Source-preserving installation and inspection of the hook entries we own.

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use serde_json::{Map, Value, json};
use tempfile::NamedTempFile;
use thiserror::Error;

use super::{
    jsonc::{JsoncDocument, JsoncError, ObjectNode},
    specs::{Client, HookSpec, hook_specs},
};

#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("unsupported alternate Codex hooks path; Codex uses {0}")]
    AlternateCodexPath(PathBuf),
    #[error("could not parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("{0} must contain a JSON object")]
    RootNotObject(PathBuf),
    #[error("hooks field must be an object; pass --force to replace it")]
    HooksNotObject,
    #[error("hooks.{0} must be a list; pass --force to replace it")]
    EventNotArray(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Jsonc(#[from] JsoncError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkResult {
    pub(crate) path: PathBuf,
    pub(crate) changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HooksCheck {
    pub(crate) client: Client,
    pub(crate) path: PathBuf,
    pub(crate) ok: bool,
    pub(crate) missing: Vec<String>,
    pub(crate) error: Option<String>,
}

pub(crate) fn default_hook_path(client: Client) -> PathBuf {
    match client {
        Client::Codex => config_root("CODEX_HOME", ".codex").join("hooks.json"),
        Client::Claude => config_root("CLAUDE_CONFIG_DIR", ".claude").join("settings.json"),
    }
}

/// Return the authoritative file to modify. Claude supports its modular JSONC
/// source; Codex has exactly one active hook source and does not accept overrides.
pub(crate) fn link_path(client: Client, requested: Option<&Path>) -> Result<PathBuf, ConfigError> {
    match (client, requested) {
        (Client::Codex, Some(_)) => Err(ConfigError::AlternateCodexPath(default_hook_path(Client::Codex))),
        (_, Some(path)) => Ok(path.to_path_buf()),
        (Client::Claude, None) => Ok(claude_link_path(default_hook_path(Client::Claude))),
        (Client::Codex, None) => Ok(default_hook_path(Client::Codex)),
    }
}

pub(crate) fn link_default_hooks(
    client: Client,
    requested: Option<&Path>,
    dry_run: bool,
    force: bool,
) -> Result<LinkResult, ConfigError> {
    let path = link_path(client, requested)?;
    link_hooks(client, &path, dry_run, force)
}

/// Install exactly one complete canonical set, preserving all unrelated source.
pub(crate) fn link_hooks(client: Client, path: &Path, dry_run: bool, force: bool) -> Result<LinkResult, ConfigError> {
    let mut document = read_document(path)?;
    let root = document.root.object().ok_or_else(|| ConfigError::RootNotObject(path.to_path_buf()))?;
    let original = document.text.clone();

    if document.member(root, "hooks").is_none() {
        document = document.insert_member(root, "hooks", &json!({}))?;
    }
    let hooks_member = hooks_member(&document);
    if hooks_member.value.object().is_none() {
        if !force {
            return Err(ConfigError::HooksNotObject);
        }
        document = document.replace_value(&hooks_member.value, &json!({}))?;
    }

    document = remove_stale_owned_commands(document, client, hook_specs(client))?;
    for spec in hook_specs(client) {
        let hooks = hooks_object(&document);
        let event = document.member(hooks, spec.event).cloned();
        if event.as_ref().is_some_and(|event| spec_present(&event.value.value(), spec)) {
            continue;
        }
        match event {
            None => document = document.insert_member(hooks, spec.event, &Value::Array(vec![group(spec)]))?,
            Some(event) => {
                if event.value.array().is_none() {
                    if !force {
                        return Err(ConfigError::EventNotArray(spec.event.to_owned()));
                    }
                    document = document.replace_value(&event.value, &Value::Array(Vec::new()))?;
                }
                let hooks = hooks_object(&document);
                let event = document.member(hooks, spec.event).expect("event was inserted or replaced");
                let array = event.value.array().expect("event is an array");
                document = document.append_element(array, &group(spec))?;
            }
        }
    }

    let changed = document.text != original;
    if changed && !dry_run {
        write_config(path, &document.text)?;
    }
    Ok(LinkResult { path: path.to_path_buf(), changed })
}

pub(crate) fn inspect_hooks(client: Client, path: &Path) -> HooksCheck {
    let document = match read_document(path) {
        Ok(document) => document,
        Err(ConfigError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return HooksCheck {
                client,
                path: path.to_path_buf(),
                ok: false,
                missing: hook_specs(client).iter().map(|spec| spec.event.to_owned()).collect(),
                error: None,
            };
        }
        Err(error) => {
            return HooksCheck {
                client,
                path: path.to_path_buf(),
                ok: false,
                missing: Vec::new(),
                error: Some(error.to_string()),
            };
        }
    };
    let Some(root) = document.root.object() else {
        return HooksCheck {
            client,
            path: path.to_path_buf(),
            ok: false,
            missing: Vec::new(),
            error: Some("root is not an object".to_owned()),
        };
    };
    let Some(hooks) = document.member(root, "hooks").and_then(|member| member.value.object()) else {
        return HooksCheck {
            client,
            path: path.to_path_buf(),
            ok: false,
            missing: Vec::new(),
            error: Some("hooks field is not an object".to_owned()),
        };
    };
    let missing: Vec<_> = hook_specs(client)
        .iter()
        .filter(|spec| {
            !document.member(hooks, spec.event).is_some_and(|event| spec_present(&event.value.value(), spec))
        })
        .map(|spec| spec.event.to_owned())
        .collect();
    HooksCheck { client, path: path.to_path_buf(), ok: missing.is_empty(), missing, error: None }
}

fn config_root(variable: &str, fallback: &str) -> PathBuf {
    let root = env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join(fallback));
    expand_tilde(root)
}

fn claude_link_path(runtime: PathBuf) -> PathBuf {
    let modular = runtime.parent().expect("settings file has a parent").join("settings/hooks.jsonc");
    if modular.exists() { modular } else { runtime }
}

fn expand_tilde(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if text == "~" || text.starts_with("~/") {
        if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
            return PathBuf::from(home).join(text.trim_start_matches("~/"));
        }
    }
    path
}

fn read_document(path: &Path) -> Result<JsoncDocument, ConfigError> {
    if !path.exists() {
        return JsoncDocument::parse("{}").map_err(ConfigError::from);
    }
    let text = fs::read_to_string(path)?;
    if path.extension().is_none_or(|extension| !extension.eq_ignore_ascii_case("jsonc")) {
        serde_json::from_str::<Value>(&text)
            .map_err(|source| ConfigError::Parse { path: path.to_path_buf(), source: Box::new(source) })?;
    }
    JsoncDocument::parse(text)
        .map_err(|source| ConfigError::Parse { path: path.to_path_buf(), source: Box::new(source) })
}

fn write_config(path: &Path, text: &str) -> Result<(), ConfigError> {
    let target = if path.symlink_metadata().is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        fs::canonicalize(path)?
    } else {
        path.to_path_buf()
    };
    let parent = target.parent().expect("configuration path has a parent");
    fs::create_dir_all(parent)?;
    #[cfg(unix)]
    let mode = target.metadata().map(|metadata| metadata.mode() & 0o7777).unwrap_or(0o600);
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file_mut().sync_all()?;
    #[cfg(unix)]
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(mode))?;
    temporary.persist(&target).map_err(|error| ConfigError::Io(error.error))?;
    Ok(())
}

fn group(spec: &HookSpec) -> Value {
    let mut group = Map::new();
    if let Some(matcher) = spec.matcher {
        group.insert("matcher".to_owned(), Value::String(matcher.to_owned()));
    }
    let mut handler = Map::from_iter([
        ("type".to_owned(), Value::String("command".to_owned())),
        ("command".to_owned(), Value::String(spec.command.to_owned())),
    ]);
    if let Some(timeout) = spec.timeout {
        handler.insert("timeout".to_owned(), Value::from(timeout));
    }
    if let Some(limit) = spec.additional_context_limit {
        handler.insert("additionalContextLimit".to_owned(), Value::from(limit));
    }
    if let Some(filter) = spec.if_filter {
        handler.insert("if".to_owned(), Value::String(filter.to_owned()));
    }
    if let Some(async_) = spec.async_ {
        handler.insert("async".to_owned(), Value::Bool(async_));
    }
    if let Some(rewake) = spec.async_rewake {
        handler.insert("asyncRewake".to_owned(), Value::Bool(rewake));
    }
    group.insert("hooks".to_owned(), Value::Array(vec![Value::Object(handler)]));
    Value::Object(group)
}

fn remove_stale_owned_commands(
    mut document: JsoncDocument,
    client: Client,
    specs: &[HookSpec],
) -> Result<JsoncDocument, ConfigError> {
    let owned = [format!("ai-coord hook {}", client.name()), format!("ai-coord waker {}", client.name())];
    let mut preserved = std::collections::HashSet::new();
    loop {
        let hooks = hooks_object(&document).clone();
        let mut removed = false;
        'events: for event in &hooks.members {
            let Some(groups) = event.value.array() else {
                continue;
            };
            for (group_index, element) in groups.elements.iter().enumerate() {
                let Some(group) = element.value.object() else {
                    continue;
                };
                let Some(handlers) = document.member(group, "hooks").and_then(|member| member.value.array()) else {
                    continue;
                };
                for (handler_index, handler) in handlers.elements.iter().enumerate() {
                    let handler_value = handler.value.value();
                    let command = handler_value.get("command").and_then(Value::as_str);
                    let Some(command) = command else {
                        continue;
                    };
                    if !owned.iter().any(|owned| command == owned) {
                        continue;
                    }
                    let matching = matching_spec(&event.key, &element.value.value(), &handler_value, specs);
                    if matching.is_some_and(|spec| preserved.insert(spec)) {
                        continue;
                    }
                    document = document.remove_element(handlers, handler_index)?;
                    document = prune_empty_group(document, &event.key, group_index)?;
                    removed = true;
                    break 'events;
                }
            }
        }
        if !removed {
            return Ok(document);
        }
    }
}

fn prune_empty_group(
    mut document: JsoncDocument,
    event_name: &str,
    group_index: usize,
) -> Result<JsoncDocument, ConfigError> {
    let hooks = hooks_object(&document);
    let event_index = hooks.members.iter().position(|member| member.key == event_name).expect("event still exists");
    let event = &hooks.members[event_index];
    let array = event.value.array().expect("event is an array");
    let group = &array.elements[group_index];
    let empty = group
        .value
        .object()
        .and_then(|group| document.member(group, "hooks"))
        .and_then(|member| member.value.array())
        .is_some_and(|handlers| handlers.elements.is_empty());
    if !empty {
        return Ok(document);
    }
    document = document.remove_element(array, group_index)?;
    let hooks = hooks_object(&document);
    let event_index = hooks.members.iter().position(|member| member.key == event_name).expect("event still exists");
    let event = &hooks.members[event_index];
    if event.value.array().is_some_and(|array| array.elements.is_empty()) {
        document = document.remove_member(hooks, event_index)?;
    }
    Ok(document)
}

fn hooks_member(document: &JsoncDocument) -> &super::jsonc::ObjectMember {
    let root = document.root.object().expect("root was checked as an object");
    document.member(root, "hooks").expect("hooks was inserted")
}

fn hooks_object(document: &JsoncDocument) -> &ObjectNode {
    hooks_member(document).value.object().expect("hooks was checked as an object")
}

fn matching_spec<'a>(event: &str, group: &Value, handler: &Value, specs: &'a [HookSpec]) -> Option<&'a HookSpec> {
    specs.iter().find(|spec| spec.event == event && handler_matches(group, handler, spec))
}

fn spec_present(value: &Value, spec: &HookSpec) -> bool {
    value.as_array().is_some_and(|groups| {
        groups.iter().any(|group| {
            group
                .get("hooks")
                .and_then(Value::as_array)
                .is_some_and(|handlers| handlers.iter().any(|handler| handler_matches(group, handler, spec)))
        })
    })
}

fn handler_matches(group: &Value, handler: &Value, spec: &HookSpec) -> bool {
    let matcher = group.get("matcher").and_then(Value::as_str);
    if match spec.matcher {
        Some(expected) => matcher != Some(expected),
        None => !matches!(matcher, None | Some("") | Some("*")),
    } {
        return false;
    }
    handler.get("type").and_then(Value::as_str) == Some("command") &&
        handler.get("command").and_then(Value::as_str) == Some(spec.command) &&
        spec.timeout.is_none_or(|timeout| handler.get("timeout").and_then(Value::as_u64) == Some(timeout)) &&
        spec.additional_context_limit
            .is_none_or(|limit| handler.get("additionalContextLimit").and_then(Value::as_u64) == Some(limit)) &&
        spec.if_filter.is_none_or(|filter| handler.get("if").and_then(Value::as_str) == Some(filter)) &&
        spec.async_.is_none_or(|async_| handler.get("async").and_then(Value::as_bool) == Some(async_)) &&
        spec.async_rewake.is_none_or(|rewake| handler.get("asyncRewake").and_then(Value::as_bool) == Some(rewake))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{MetadataExt, symlink},
    };

    use tempfile::tempdir;

    use super::{Client, claude_link_path, inspect_hooks, link_hooks, link_path};

    #[test]
    fn installs_idempotently_without_reformatting_unrelated_jsonc() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("settings.jsonc");
        fs::write(&path, "{\n  // retain this\n  \"unrelated\": true,\n  \"hooks\": {\n    \"Stop\": [{\"hooks\": [{\"type\": \"command\", \"command\": \"other\"}]}],\n  },\n}").unwrap();

        let first = link_hooks(Client::Claude, &path, false, false).unwrap();
        let installed = fs::read_to_string(&path).unwrap();
        let second = link_hooks(Client::Claude, &path, false, false).unwrap();

        assert!(first.changed);
        assert!(!second.changed);
        assert!(installed.contains("// retain this"));
        assert!(installed.contains("\"command\": \"other\""));
        assert!(inspect_hooks(Client::Claude, &path).ok);
    }

    #[test]
    fn removes_stale_owned_handlers_but_not_neighbouring_handlers() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("hooks.json");
        fs::write(&path, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"ai-coord hook codex","timeout":99},{"type":"command","command":"other"}]}],"Other":[{"hooks":[{"type":"command","command":"ai-coord waker codex"}]}]}}"#).unwrap();

        link_hooks(Client::Codex, &path, false, false).unwrap();
        let text = fs::read_to_string(path).unwrap();

        assert!(text.contains("\"command\":\"other\""));
        assert!(!text.contains("ai-coord waker codex"));
        assert!(inspect_hooks(Client::Codex, &directory.path().join("hooks.json")).ok);
    }

    #[test]
    fn dry_run_does_not_write_and_force_replaces_only_owned_containers() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("hooks.json");
        fs::write(&path, r#"{"hooks":"bad","other":7}"#).unwrap();
        assert!(link_hooks(Client::Codex, &path, true, true).unwrap().changed);
        assert_eq!(fs::read_to_string(&path).unwrap(), r#"{"hooks":"bad","other":7}"#);
        link_hooks(Client::Codex, &path, false, true).unwrap();
        assert!(fs::read_to_string(path).unwrap().contains("\"other\":7"));
    }

    #[test]
    fn write_preserves_symlink_and_target_mode() {
        let directory = tempdir().unwrap();
        let target = directory.path().join("target.json");
        let link = directory.path().join("link.json");
        fs::write(&target, "{}").unwrap();
        fs::set_permissions(&target, std::os::unix::fs::PermissionsExt::from_mode(0o640)).unwrap();
        symlink(&target, &link).unwrap();

        link_hooks(Client::Codex, &link, false, false).unwrap();

        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::metadata(target).unwrap().mode() & 0o777, 0o640);
    }

    #[test]
    fn codex_has_no_alternate_source_and_claude_prefers_modular_jsonc() {
        let directory = tempdir().unwrap();
        let alternate = directory.path().join("alternate.json");
        assert!(link_path(Client::Codex, Some(&alternate)).is_err());
        let home = directory.path().join("claude");
        fs::create_dir_all(home.join("settings")).unwrap();
        fs::write(home.join("settings/hooks.jsonc"), "{}").unwrap();
        // Avoid changing process environment in a parallel test: exercise the
        // modular selection from an explicit runtime path.
        assert_eq!(claude_link_path(home.join("settings.json")), home.join("settings/hooks.jsonc"));
        assert_eq!(link_path(Client::Claude, Some(&alternate)).unwrap(), alternate);
    }
}
