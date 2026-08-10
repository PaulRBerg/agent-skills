use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsStr,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Component, Path, PathBuf},
    sync::LazyLock,
};

use ignore::{DirEntry, WalkBuilder};
use regex::bytes::Regex;

use crate::{
    catalog::{Catalog, Skill},
    dependency::DependencyIdentifier,
    error::Error,
    traversal::RootMode,
};

use super::model::{EdgeType, EvidenceRecord, Provenance};

const EXCLUDED_DIRECTORY_NAMES: &[&str] =
    &[".git", ".next", ".venv", "build", "coverage", "dist", "node_modules", "out", "target", "vendor"];
const MATCH_OVERLAP_BYTES: usize = 256;
const READ_CHUNK_BYTES: usize = 64 * 1024;
const MAX_SNIPPET_BYTES: usize = 4 * 1024;

static TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[$/]([a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)\b").expect("token pattern is valid"));
static PROSE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)[ \t]+skill\b").expect("prose pattern is valid")
});
static SKILL_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\.agents/|\.claude/|\.codex/)?skills/([a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)\b")
        .expect("skill path pattern is valid")
});
static SIBLING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.\./([a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?)/SKILL\.md\b").expect("sibling pattern is valid")
});

pub struct InferenceOptions<'a> {
    pub selected: &'a BTreeSet<String>,
    pub include_self: bool,
    pub include_snippets: bool,
}

pub struct Inference {
    pub edges: Vec<EvidenceRecord>,
    pub unresolved: Vec<EvidenceRecord>,
}

pub fn collect(catalog: &Catalog, options: InferenceOptions<'_>) -> Result<Inference, Error> {
    let known_names: BTreeSet<String> = catalog.skills.iter().map(effective_name).collect::<BTreeSet<_>>();
    let mut edges = BTreeSet::new();
    let mut unresolved = BTreeSet::new();

    collect_declared(catalog, &options, &mut edges)?;
    let roots = reference_roots(catalog);
    for root in roots {
        scan_root(
            &root.path,
            root.mode,
            root.include_catalog_sources,
            catalog,
            &known_names,
            &options,
            &mut edges,
            &mut unresolved,
        )?;
    }

    Ok(Inference { edges: edges.into_iter().collect(), unresolved: unresolved.into_iter().collect() })
}

fn collect_declared(
    catalog: &Catalog,
    options: &InferenceOptions<'_>,
    edges: &mut BTreeSet<EvidenceRecord>,
) -> Result<(), Error> {
    for skill in &catalog.skills {
        let source = effective_name(skill);
        for dependency in &skill.dependencies {
            let target_name = dependency.identifier.target_name().as_str();
            if !options.selected.is_empty() &&
                !options.selected.contains(&source) &&
                !options.selected.contains(target_name)
            {
                continue;
            }
            if source == target_name && !options.include_self {
                continue;
            }
            let (target, target_repository) = match &dependency.identifier {
                DependencyIdentifier::Internal(name) => (name.to_string(), None),
                DependencyIdentifier::External(external) => {
                    (dependency.identifier.as_identifier(), Some(format!("{}/{}", external.owner, external.repository)))
                }
            };
            edges.insert(EvidenceRecord {
                edge_type: EdgeType::Dependency,
                provenance: Provenance::Declared,
                identifier: dependency.identifier.as_identifier(),
                source: Some(source.clone()),
                target,
                path: skill.skill_path().to_path_buf(),
                line: dependency.line,
                snippet: if options.include_snippets {
                    Some(read_line(skill.skill_path(), dependency.line)?)
                } else {
                    None
                },
                target_repository,
            });
        }
    }
    Ok(())
}

#[derive(Clone)]
struct ReferenceRoot {
    path: PathBuf,
    mode: RootMode,
    include_catalog_sources: bool,
}

fn reference_roots(catalog: &Catalog) -> Vec<ReferenceRoot> {
    let mut roots: BTreeMap<PathBuf, ReferenceRoot> = catalog
        .roots
        .iter()
        .map(|root| {
            (
                root.exposure_path.clone(),
                ReferenceRoot {
                    path: root.exposure_path.clone(),
                    mode: root.mode,
                    include_catalog_sources: root.include_catalog_sources,
                },
            )
        })
        .collect();
    let covered: Vec<_> = catalog.roots.iter().map(|root| &root.resolved_path).collect();
    for skill in &catalog.skills {
        let directory = skill.exposure.resolved_directory();
        if covered.iter().any(|root| directory.starts_with(root)) {
            continue;
        }
        roots.entry(directory.to_path_buf()).or_insert_with(|| ReferenceRoot {
            path: directory.to_path_buf(),
            mode: RootMode::Explicit,
            include_catalog_sources: true,
        });
    }
    roots.into_values().collect()
}

#[allow(clippy::too_many_arguments)]
fn scan_root(
    root: &Path,
    mode: RootMode,
    include_catalog_sources: bool,
    catalog: &Catalog,
    known_names: &BTreeSet<String>,
    options: &InferenceOptions<'_>,
    edges: &mut BTreeSet<EvidenceRecord>,
    unresolved: &mut BTreeSet<EvidenceRecord>,
) -> Result<(), Error> {
    let mut builder = WalkBuilder::new(root);
    let scan_root = root.to_path_buf();
    builder
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(false)
        .sort_by_file_path(|left, right| left.cmp(right))
        .filter_entry(move |entry| reference_entry_allowed(entry, &scan_root, mode, include_catalog_sources));

    for entry in builder.build() {
        let entry = entry.map_err(|error| Error::Traversal { path: root.to_path_buf(), message: error.to_string() })?;
        if !entry.file_type().is_some_and(|file_type| file_type.is_file()) {
            continue;
        }
        scan_file(entry.path(), catalog, known_names, options, edges, unresolved)?;
    }
    Ok(())
}

fn scan_file(
    path: &Path,
    catalog: &Catalog,
    known_names: &BTreeSet<String>,
    options: &InferenceOptions<'_>,
    edges: &mut BTreeSet<EvidenceRecord>,
    unresolved: &mut BTreeSet<EvidenceRecord>,
) -> Result<(), Error> {
    let file = File::open(path).map_err(|error| Error::io("read reference file", path, error))?;
    let resolved_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let source = source_for_file(&resolved_path, &catalog.skills);
    let mut reader = BufReader::with_capacity(READ_CHUNK_BYTES, file);
    let mut window = Vec::with_capacity(READ_CHUNK_BYTES + MATCH_OVERLAP_BYTES);
    let mut chunk = vec![0; READ_CHUNK_BYTES];
    let mut window_start_line = 1u64;
    let mut known_evidence = BTreeSet::new();
    let mut unresolved_evidence = BTreeSet::new();
    loop {
        let read = reader.read(&mut chunk).map_err(|error| Error::io("read reference file", path, error))?;
        if read == 0 {
            process_window(
                &window,
                window.len(),
                window_start_line,
                &resolved_path,
                source.as_deref(),
                known_names,
                options,
                &mut known_evidence,
                &mut unresolved_evidence,
                edges,
                unresolved,
            );
            break;
        }
        let previous_len = window.len();
        window.extend_from_slice(&chunk[..read]);
        if let Some(nul) = chunk[..read].iter().position(|byte| *byte == 0) {
            let nul = previous_len + nul;
            let complete_text = window[..nul].iter().rposition(|byte| *byte == b'\n').map_or(0, |index| index + 1);
            process_window(
                &window,
                complete_text,
                window_start_line,
                &resolved_path,
                source.as_deref(),
                known_names,
                options,
                &mut known_evidence,
                &mut unresolved_evidence,
                edges,
                unresolved,
            );
            break;
        }
        let stable_end = window.len().saturating_sub(MATCH_OVERLAP_BYTES);
        process_window(
            &window,
            stable_end,
            window_start_line,
            &resolved_path,
            source.as_deref(),
            known_names,
            options,
            &mut known_evidence,
            &mut unresolved_evidence,
            edges,
            unresolved,
        );
        window_start_line += window[..stable_end].iter().filter(|byte| **byte == b'\n').count() as u64;
        window.drain(..stable_end);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CandidateKind {
    Known,
    Unresolved,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Candidate {
    start: usize,
    end: usize,
    target: String,
    kind: CandidateKind,
}

#[allow(clippy::too_many_arguments)]
fn process_window(
    window: &[u8],
    emit_before: usize,
    window_start_line: u64,
    path: &Path,
    source: Option<&str>,
    known_names: &BTreeSet<String>,
    options: &InferenceOptions<'_>,
    known_evidence: &mut BTreeSet<(String, u64)>,
    unresolved_evidence: &mut BTreeSet<(String, u64)>,
    edges: &mut BTreeSet<EvidenceRecord>,
    unresolved: &mut BTreeSet<EvidenceRecord>,
) {
    let mut candidates = Vec::new();
    for (target, start, end) in explicit_tokens(window) {
        let kind = if known_names.contains(&target) {
            CandidateKind::Known
        } else if target.contains('-') {
            CandidateKind::Unresolved
        } else {
            continue;
        };
        if start < emit_before {
            candidates.push(Candidate { start, end, target, kind });
        }
    }
    collect_capture_targets(&PROSE_RE, window, emit_before, known_names, &mut candidates);
    collect_capture_targets(&SKILL_PATH_RE, window, emit_before, known_names, &mut candidates);
    collect_capture_targets(&SIBLING_RE, window, emit_before, known_names, &mut candidates);
    candidates.sort();

    let mut cursor = 0usize;
    let mut line = window_start_line;
    for candidate in candidates {
        line += window[cursor..candidate.start].iter().filter(|byte| **byte == b'\n').count() as u64;
        cursor = candidate.start;
        let snippet = options
            .include_snippets
            .then(|| String::from_utf8_lossy(&window[candidate.start..candidate.end]).into_owned());
        match candidate.kind {
            CandidateKind::Known => {
                if !known_evidence.insert((candidate.target.clone(), line)) ||
                    (!options.selected.is_empty() &&
                        !options.selected.contains(&candidate.target) &&
                        !source.is_some_and(|source| options.selected.contains(source))) ||
                    (source == Some(candidate.target.as_str()) && !options.include_self)
                {
                    continue;
                }
                edges.insert(EvidenceRecord {
                    edge_type: if source.is_some() { EdgeType::Dependency } else { EdgeType::ExternalReference },
                    provenance: Provenance::Inferred,
                    identifier: candidate.target.clone(),
                    source: source.map(str::to_owned),
                    target: candidate.target,
                    path: path.to_path_buf(),
                    line,
                    snippet,
                    target_repository: None,
                });
            }
            CandidateKind::Unresolved => {
                if !unresolved_evidence.insert((candidate.target.clone(), line)) ||
                    (!options.selected.is_empty() && !options.selected.contains(&candidate.target))
                {
                    continue;
                }
                unresolved.insert(EvidenceRecord {
                    edge_type: EdgeType::UnresolvedLikeReference,
                    provenance: Provenance::Inferred,
                    identifier: candidate.target.clone(),
                    source: source.map(str::to_owned),
                    target: candidate.target,
                    path: path.to_path_buf(),
                    line,
                    snippet,
                    target_repository: None,
                });
            }
        }
    }
}

fn explicit_tokens(line: &[u8]) -> Vec<(String, usize, usize)> {
    let mut targets = BTreeSet::new();
    for captures in TOKEN_RE.captures_iter(line) {
        let Some(whole) = captures.get(0) else {
            continue;
        };
        let marker = line[whole.start()];
        let previous = whole.start().checked_sub(1).map(|index| line[index]);
        let allowed = if marker == b'$' {
            previous.is_none_or(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'_' | b'.' | b'/' | b'-'))
        } else {
            previous.is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'`' | b'\'' | b'"' | b'('))
        };
        if !allowed {
            continue;
        }
        if let Some(name) = captures.get(1) {
            targets.insert((String::from_utf8_lossy(name.as_bytes()).into_owned(), whole.start(), whole.end()));
        }
    }
    targets.into_iter().collect()
}

fn collect_capture_targets(
    pattern: &Regex,
    line: &[u8],
    emit_before: usize,
    known_names: &BTreeSet<String>,
    candidates: &mut Vec<Candidate>,
) {
    for captures in pattern.captures_iter(line) {
        let Some(whole) = captures.get(0) else {
            continue;
        };
        if whole.start() >= emit_before {
            continue;
        }
        let Some(name) = captures.get(1) else {
            continue;
        };
        let name = String::from_utf8_lossy(name.as_bytes()).into_owned();
        if known_names.contains(&name) {
            candidates.push(Candidate {
                start: whole.start(),
                end: whole.end(),
                target: name,
                kind: CandidateKind::Known,
            });
        }
    }
}

fn source_for_file(path: &Path, skills: &[Skill]) -> Option<String> {
    skills
        .iter()
        .filter(|skill| path.starts_with(skill.exposure.resolved_directory()))
        .max_by(|left, right| {
            left.exposure
                .resolved_directory()
                .components()
                .count()
                .cmp(&right.exposure.resolved_directory().components().count())
                .then_with(|| left.exposure.cmp(&right.exposure))
        })
        .map(effective_name)
}

pub fn effective_name(skill: &Skill) -> String {
    skill.name.as_ref().map(ToString::to_string).unwrap_or_else(|| skill.directory_name.clone())
}

fn read_line(path: &Path, requested_line: u64) -> Result<String, Error> {
    let file = File::open(path).map_err(|error| Error::io("read dependency snippet", path, error))?;
    let mut reader = BufReader::new(file);
    let mut current_line = 1u64;
    let mut snippet = Vec::new();
    let mut truncated = false;
    let mut saw_requested_bytes = false;
    loop {
        let available = reader.fill_buf().map_err(|error| Error::io("read dependency snippet", path, error))?;
        if available.is_empty() {
            if current_line == requested_line && saw_requested_bytes {
                break;
            }
            return Err(Error::MapData(format!(
                "dependency line {requested_line} is past end of file: {}",
                path.display()
            )));
        }
        let consumed =
            available.iter().position(|byte| *byte == b'\n').map_or_else(|| available.len(), |index| index + 1);
        let complete_line = available[consumed.saturating_sub(1)] == b'\n';
        if current_line == requested_line {
            saw_requested_bytes = true;
            let content_end = consumed - usize::from(complete_line);
            let remaining = MAX_SNIPPET_BYTES.saturating_sub(snippet.len());
            snippet.extend_from_slice(&available[..content_end.min(remaining)]);
            truncated |= content_end > remaining;
        }
        reader.consume(consumed);
        if complete_line {
            if current_line == requested_line {
                break;
            }
            current_line += 1;
        }
    }
    let mut result = String::from_utf8_lossy(&snippet).trim_end_matches('\r').to_owned();
    if truncated {
        result.push('…');
    }
    Ok(result)
}

fn reference_entry_allowed(entry: &DirEntry, scan_root: &Path, mode: RootMode, include_catalog_sources: bool) -> bool {
    let path = entry.path();
    if path == scan_root {
        return true;
    }
    if entry.file_type().is_some_and(|file_type| file_type.is_dir()) &&
        entry.file_name().to_str().is_some_and(|name| EXCLUDED_DIRECTORY_NAMES.contains(&name))
    {
        return false;
    }
    if agent_state_path(path) {
        return false;
    }
    if mode == RootMode::Broad && broad_home_path_is_excluded(path, scan_root, include_catalog_sources) {
        return false;
    }
    true
}

fn broad_home_path_is_excluded(path: &Path, scan_root: &Path, include_catalog_sources: bool) -> bool {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return false;
    };
    let mut roots = vec![
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
        roots.extend(["projects/agent-skills", "sablier/agent-skills", "sablier/sablier-skills"]);
    }
    roots
        .into_iter()
        .map(|relative| home.join(relative))
        .any(|excluded| !scan_root.starts_with(&excluded) && path.starts_with(excluded))
}

fn agent_state_path(path: &Path) -> bool {
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
