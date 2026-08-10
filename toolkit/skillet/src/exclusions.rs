use std::{
    ffi::OsStr,
    path::{Component, Path},
};

const EXCLUDED_DIRECTORY_NAMES: &[&str] =
    &[".git", ".next", ".venv", "build", "coverage", "dist", "node_modules", "out", "target", "vendor"];

pub(crate) fn directory_name_is_excluded(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| EXCLUDED_DIRECTORY_NAMES.contains(&name))
}

pub(crate) fn agent_state_path(path: &Path) -> bool {
    let parts: Vec<_> = path.components().map(Component::as_os_str).collect();
    if parts.windows(2).any(|pair| {
        matches!(
            (pair[0].to_str(), pair[1].to_str()),
            (
                Some(".claude"),
                Some(
                    "backups" |
                        "debug" |
                        "file-history" |
                        "image-cache" |
                        "logs" |
                        "paste-cache" |
                        "plans" |
                        "projects" |
                        "session-env" |
                        "shell-snapshots" |
                        "statsig" |
                        "tasks" |
                        "todos",
                ),
            ) | (
                Some(".codex"),
                Some(
                    ".tmp" |
                        "archived_sessions" |
                        "backups" |
                        "cache" |
                        "generated_images" |
                        "log" |
                        "logs" |
                        "sessions" |
                        "shell_snapshots" |
                        "sqlite" |
                        "threads" |
                        "tmp",
                ),
            )
        )
    }) {
        return true;
    }
    if parts.windows(2).any(|pair| {
        matches!(
            (pair[0].to_str(), pair[1].to_str()),
            (Some(".claude"), Some("history.jsonl" | "remote-settings.json" | "stats-cache.json")) |
                (Some(".codex"), Some("history.jsonl" | "session_index.jsonl"))
        )
    }) {
        return true;
    }
    let in_codex =
        parts.iter().position(|part| *part == OsStr::new(".codex")).is_some_and(|index| index + 1 < parts.len());
    let file = parts.last().and_then(|part| part.to_str()).unwrap_or_default();
    in_codex &&
        (file.ends_with(".sqlite") ||
            file.ends_with(".sqlite-shm") ||
            file.ends_with(".sqlite-wal") ||
            file.ends_with(".bak"))
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    use super::{agent_state_path, directory_name_is_excluded};

    #[test]
    fn exclusions_cover_dependency_trees_and_agent_state() {
        for name in [".git", ".next", ".venv", "build", "coverage", "dist", "node_modules", "out", "target", "vendor"] {
            assert!(directory_name_is_excluded(OsStr::new(name)), "{name}");
        }
        for path in [
            ".claude/logs/session.log",
            ".claude/history.jsonl",
            ".codex/sessions/rollout.jsonl",
            ".codex/session_index.jsonl",
            ".codex/state.sqlite-wal",
            ".codex/config.bak",
        ] {
            assert!(agent_state_path(Path::new(path)), "{path}");
        }
        for path in ["history.jsonl", "workspace/state.sqlite", ".codex", ".claude/skills/example/SKILL.md"] {
            assert!(!agent_state_path(Path::new(path)), "{path}");
        }
    }
}
