use std::{
    env,
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

pub(crate) const EXCLUDED_DIRECTORY_NAMES: &[&str] =
    &[".git", ".next", ".venv", "build", "coverage", "dist", "node_modules", "out", "target", "vendor"];

/// Home paths macOS protects from ordinary traversal (Time Machine exclusions, SIP-adjacent).
pub(crate) const MACOS_PROTECTED_HOME_PATHS: &[&str] = &["Library", ".Trash"];

/// Home paths that always hold agent state rather than user content.
pub(crate) const ALWAYS_IGNORED_HOME_PATHS: &[&str] = &[".agents", ".claude", ".codex", ".local/state/skills"];

/// Home paths that hold package-manager and toolchain caches, irrelevant to a broad skill scan.
pub(crate) const BROAD_SCAN_CACHE_PATHS: &[&str] = &[
    ".cache",
    ".npm",
    ".rustup",
    ".cargo/git",
    ".cargo/registry",
    ".bun/install/cache",
    ".pnpm-store",
    ".local/share/uv",
    ".local/share/rustup",
    ".local/share/cargo/git",
    ".local/share/cargo/registry",
    ".local/share/bun/install/cache",
    ".local/share/pnpm/store",
    "go/pkg/mod",
];

/// Home paths holding source catalogs that a broad scan excludes unless explicitly requested.
pub(crate) const CATALOG_SOURCE_HOME_PATHS: &[&str] =
    &["projects/agent-skills", "sablier/agent-skills", "sablier/sablier-skills"];

pub(crate) const CLAUDE_AGENT_STATE_DIRECTORIES: &[&str] = &[
    "backups",
    "debug",
    "file-history",
    "image-cache",
    "logs",
    "paste-cache",
    "plans",
    "projects",
    "session-env",
    "shell-snapshots",
    "statsig",
    "tasks",
    "todos",
];

pub(crate) const CODEX_AGENT_STATE_DIRECTORIES: &[&str] = &[
    ".tmp",
    "archived_sessions",
    "backups",
    "cache",
    "generated_images",
    "log",
    "logs",
    "sessions",
    "shell_snapshots",
    "sqlite",
    "threads",
    "tmp",
];

pub(crate) const CLAUDE_AGENT_STATE_FILES: &[&str] = &["history.jsonl", "remote-settings.json", "stats-cache.json"];

pub(crate) const CODEX_AGENT_STATE_FILES: &[&str] = &["history.jsonl", "session_index.jsonl"];

/// Codex file-name globs matched by suffix in [`agent_state_path`] rather than by an exact name.
pub(crate) const CODEX_AGENT_STATE_FILE_GLOBS: &[&str] = &["*.sqlite*", "*.bak"];

pub(crate) fn directory_name_is_excluded(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| EXCLUDED_DIRECTORY_NAMES.contains(&name))
}

pub(crate) fn agent_state_path(path: &Path) -> bool {
    let parts: Vec<_> = path.components().map(Component::as_os_str).collect();
    if parts.windows(2).any(|pair| {
        let (Some(first), Some(second)) = (pair[0].to_str(), pair[1].to_str()) else {
            return false;
        };
        (first == ".claude" && CLAUDE_AGENT_STATE_DIRECTORIES.contains(&second)) ||
            (first == ".codex" && CODEX_AGENT_STATE_DIRECTORIES.contains(&second))
    }) {
        return true;
    }
    if parts.windows(2).any(|pair| {
        let (Some(first), Some(second)) = (pair[0].to_str(), pair[1].to_str()) else {
            return false;
        };
        (first == ".claude" && CLAUDE_AGENT_STATE_FILES.contains(&second)) ||
            (first == ".codex" && CODEX_AGENT_STATE_FILES.contains(&second))
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

/// Home-relative roots excluded from a broad scan: macOS-protected paths, always-ignored agent
/// state, package-manager caches, and (unless `include_catalog_sources`) known catalog sources.
pub(crate) fn broad_excluded_roots(include_catalog_sources: bool) -> Vec<PathBuf> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut relative_roots: Vec<&str> = MACOS_PROTECTED_HOME_PATHS
        .iter()
        .chain(ALWAYS_IGNORED_HOME_PATHS)
        .chain(BROAD_SCAN_CACHE_PATHS)
        .copied()
        .collect();
    if !include_catalog_sources {
        relative_roots.extend(CATALOG_SOURCE_HOME_PATHS.iter().copied());
    }
    relative_roots.into_iter().map(|relative| home.join(relative)).collect()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    use super::{EXCLUDED_DIRECTORY_NAMES, agent_state_path, directory_name_is_excluded};

    #[test]
    fn exclusions_cover_dependency_trees_and_agent_state() {
        for name in EXCLUDED_DIRECTORY_NAMES {
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
