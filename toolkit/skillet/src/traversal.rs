use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
};

use ignore::{DirEntry, WalkBuilder};
use serde::Serialize;

use crate::error::Error;

const BROAD_EXCLUDED_NAMES: &[&str] =
    &[".git", ".next", ".venv", "build", "coverage", "dist", "node_modules", "out", "target", "vendor"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RootMode {
    Explicit,
    Broad,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootRequest {
    pub path: PathBuf,
    pub mode: RootMode,
    pub include_catalog_sources: bool,
}

impl RootRequest {
    pub fn explicit(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), mode: RootMode::Explicit, include_catalog_sources: true }
    }

    pub fn broad(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), mode: RootMode::Broad, include_catalog_sources: false }
    }

    pub fn broad_including_catalog_sources(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), mode: RootMode::Broad, include_catalog_sources: true }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ScanRoot {
    pub exposure_path: PathBuf,
    pub resolved_path: PathBuf,
    pub mode: RootMode,
    pub include_catalog_sources: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExposureScope {
    Direct,
    Catalog,
    Agents,
    Claude,
    Codex,
    Broad,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SkillExposure {
    pub root: PathBuf,
    pub exposure_path: PathBuf,
    pub resolved_path: PathBuf,
    pub scope: ExposureScope,
    pub directory_symlink_target: Option<PathBuf>,
}

impl SkillExposure {
    pub fn exposure_directory(&self) -> &Path {
        self.exposure_path.parent().expect("a discovered SKILL.md always has a parent")
    }

    pub fn resolved_directory(&self) -> &Path {
        self.resolved_path.parent().expect("a discovered SKILL.md always has a parent")
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Discovery {
    pub roots: Vec<ScanRoot>,
    pub skills: Vec<SkillExposure>,
}

pub fn discover(requests: &[RootRequest]) -> Result<Discovery, Error> {
    let mut roots = Vec::new();
    for request in requests {
        roots.push(normalize_root(request)?);
    }
    roots.sort();
    roots.dedup();

    let mut skills = BTreeMap::new();
    for root in &roots {
        discover_root(root, &mut skills)?;
    }
    Ok(Discovery { roots, skills: skills.into_values().collect() })
}

fn normalize_root(request: &RootRequest) -> Result<ScanRoot, Error> {
    let absolute = if request.path.is_absolute() {
        request.path.clone()
    } else {
        env::current_dir()
            .map_err(|error| Error::io("resolve current directory for", &request.path, error))?
            .join(&request.path)
    };
    let exposure_path = lexical_normalize(&absolute);
    let metadata = match fs::metadata(&exposure_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::RootMissing(exposure_path));
        }
        Err(error) => return Err(Error::io("inspect", &exposure_path, error)),
    };
    if !metadata.is_dir() {
        return Err(Error::RootNotDirectory(exposure_path));
    }
    let resolved_path =
        fs::canonicalize(&exposure_path).map_err(|error| Error::io("resolve", &exposure_path, error))?;
    Ok(ScanRoot {
        exposure_path,
        resolved_path,
        mode: request.mode,
        include_catalog_sources: request.include_catalog_sources,
    })
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn discover_root(root: &ScanRoot, skills: &mut BTreeMap<PathBuf, SkillExposure>) -> Result<(), Error> {
    // A directly requested skill remains explicit even when a parent ignore file excludes it.
    add_candidate(root, &root.exposure_path.join("SKILL.md"), skills)?;

    let mut builder = WalkBuilder::new(&root.exposure_path);
    builder
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(false)
        .sort_by_file_path(|left, right| left.cmp(right));
    if root.mode == RootMode::Explicit {
        builder.max_depth(Some(4));
    } else {
        let scan_root = root.exposure_path.clone();
        let excluded_roots = broad_excluded_roots(root.include_catalog_sources);
        builder.filter_entry(move |entry| broad_entry_allowed(entry, &scan_root, &excluded_roots));
    }

    for entry in builder.build() {
        let entry =
            entry.map_err(|error| Error::Traversal { path: root.exposure_path.clone(), message: error.to_string() })?;
        let path = entry.path();
        if path == root.exposure_path {
            continue;
        }

        if entry.file_type().is_some_and(|file_type| file_type.is_symlink()) && recognized_skill_directory(root, path) {
            add_candidate(root, &path.join("SKILL.md"), skills)?;
            continue;
        }
        if entry.file_type().is_some_and(|file_type| file_type.is_file()) &&
            path.file_name() == Some(OsStr::new("SKILL.md")) &&
            (root.mode == RootMode::Broad || recognized_skill_file(root, path))
        {
            add_candidate(root, path, skills)?;
        }
    }
    Ok(())
}

fn add_candidate(root: &ScanRoot, path: &Path, skills: &mut BTreeMap<PathBuf, SkillExposure>) -> Result<(), Error> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io("inspect", path, error)),
    };
    if !metadata.is_file() {
        return Ok(());
    }
    let exposure_path = lexical_normalize(path);
    let resolved_path =
        fs::canonicalize(&exposure_path).map_err(|error| Error::io("resolve", &exposure_path, error))?;
    let directory = exposure_path.parent().expect("a SKILL.md candidate always has a parent");
    let directory_symlink_target = match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Some(fs::read_link(directory).map_err(|error| Error::io("read symlink", directory, error))?)
        }
        Ok(_) => None,
        Err(error) => return Err(Error::io("inspect", directory, error)),
    };
    let scope = classify_scope(root, &exposure_path);
    skills.entry(exposure_path.clone()).or_insert(SkillExposure {
        root: root.exposure_path.clone(),
        exposure_path,
        resolved_path,
        scope,
        directory_symlink_target,
    });
    Ok(())
}

fn recognized_skill_file(root: &ScanRoot, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(&root.exposure_path) else {
        return false;
    };
    let parts: Vec<_> = relative.components().map(Component::as_os_str).collect();
    if parts.as_slice() == [OsStr::new("SKILL.md")] {
        return true;
    }
    if root.exposure_path.file_name() == Some(OsStr::new("skills")) &&
        parts.len() == 2 &&
        parts[1] == OsStr::new("SKILL.md")
    {
        return true;
    }
    (parts.len() == 3 && parts[0] == OsStr::new("skills") && parts[2] == OsStr::new("SKILL.md")) ||
        (parts.len() == 4 &&
            matches!(parts[0].to_str(), Some(".agents" | ".claude" | ".codex")) &&
            parts[1] == OsStr::new("skills") &&
            parts[3] == OsStr::new("SKILL.md"))
}

fn recognized_skill_directory(root: &ScanRoot, path: &Path) -> bool {
    let marker = path.join("SKILL.md");
    recognized_skill_file(root, &marker)
}

fn classify_scope(root: &ScanRoot, path: &Path) -> ExposureScope {
    if path == root.exposure_path.join("SKILL.md") {
        return ExposureScope::Direct;
    }
    let parts: Vec<_> = path.components().map(Component::as_os_str).collect();
    for pair in parts.windows(2) {
        if pair[1] != OsStr::new("skills") {
            continue;
        }
        match pair[0].to_str() {
            Some(".agents") => return ExposureScope::Agents,
            Some(".claude") => return ExposureScope::Claude,
            Some(".codex") => return ExposureScope::Codex,
            _ => {}
        }
    }
    if parts.iter().any(|part| *part == OsStr::new("skills")) { ExposureScope::Catalog } else { ExposureScope::Broad }
}

fn broad_entry_allowed(entry: &DirEntry, scan_root: &Path, excluded_roots: &[PathBuf]) -> bool {
    let path = entry.path();
    if path == scan_root {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    if BROAD_EXCLUDED_NAMES.iter().any(|excluded| name == *excluded) {
        return false;
    }
    if excluded_roots.iter().any(|excluded| !scan_root.starts_with(excluded) && path.starts_with(excluded)) {
        return false;
    }
    if agent_state_path(path) {
        return false;
    }
    true
}

fn broad_excluded_roots(include_catalog_sources: bool) -> Vec<PathBuf> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut relative_roots = vec![
        ".Trash",
        ".agents",
        ".bun/install/cache",
        ".cache",
        ".cargo/git",
        ".cargo/registry",
        ".claude",
        ".codex",
        ".local/share/bun/install/cache",
        ".local/share/cargo/git",
        ".local/share/cargo/registry",
        ".local/share/pnpm/store",
        ".local/share/rustup",
        ".local/share/uv",
        ".local/state/skills",
        ".npm",
        ".pnpm-store",
        ".rustup",
        "Library",
        "go/pkg/mod",
    ];
    if !include_catalog_sources {
        relative_roots.extend(["projects/agent-skills", "sablier/agent-skills", "sablier/sablier-skills"]);
    }
    relative_roots.into_iter().map(|relative| home.join(relative)).collect()
}

fn agent_state_path(path: &Path) -> bool {
    let parts: Vec<_> = path.components().map(Component::as_os_str).collect();
    parts.windows(2).any(|pair| {
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
                        "todos"
                )
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
                        "tmp"
                )
            )
        )
    })
}
