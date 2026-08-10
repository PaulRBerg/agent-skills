mod infer;
mod model;
mod render;
mod roots;

use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{
    catalog::{Catalog, Skill, SkillHashes},
    cli::MapArgs,
    dependency::SkillName,
    error::Error,
    frontmatter::InstallTargets,
    traversal::ExposureScope,
};

use infer::{InferenceOptions, effective_name};
use model::{
    Counts, DuplicateRecord, DuplicateType, EdgeType, MapReport, PortfolioRecord, Provenance, RootRecord,
    SCHEMA_VERSION, SkillKind, SkillLocation, SkillRecord, SkippedRecord,
};

pub use model::{EvidenceRecord, MapReport as Report};

pub fn run(args: MapArgs) -> Result<(), Error> {
    let selected = selected_names(&args.skill)?;
    let resolved = roots::resolve(&args)?;
    let catalog = Catalog::load(&resolved.requests)?;
    reject_unmappable_diagnostics(&catalog)?;

    let discovered_names: BTreeSet<_> = catalog.skills.iter().map(effective_name).collect();
    let missing: Vec<_> = selected.difference(&discovered_names).cloned().collect();
    if !missing.is_empty() {
        let mut stderr = io::stderr().lock();
        writeln!(stderr, "ai-skillet: warning: no discovered skill named {}", missing.join(", "))
            .map_err(|error| Error::io("write warning to", Path::new("stderr"), error))?;
    }

    let inference = infer::collect(
        &catalog,
        InferenceOptions {
            selected: &selected,
            include_self: args.include_self,
            include_snippets: args.include_snippets,
        },
    )?;
    let duplicates = duplicate_records(&catalog, &selected);
    let skills = skill_records(&catalog, resolved.portfolio.as_ref(), &selected)?;
    let roots = catalog
        .roots
        .iter()
        .map(|root| RootRecord {
            exposure_path: root.exposure_path.clone(),
            resolved_path: root.resolved_path.clone(),
            mode: root.mode,
            include_catalog_sources: root.include_catalog_sources,
        })
        .collect();
    let counts = Counts {
        skills: skills.len(),
        edges: inference.edges.len(),
        declared_dependencies: inference
            .edges
            .iter()
            .filter(|edge| edge.edge_type == EdgeType::Dependency && edge.provenance == Provenance::Declared)
            .count(),
        inferred_dependencies: inference
            .edges
            .iter()
            .filter(|edge| edge.edge_type == EdgeType::Dependency && edge.provenance == Provenance::Inferred)
            .count(),
        external_references: inference
            .edges
            .iter()
            .filter(|edge| edge.edge_type == EdgeType::ExternalReference)
            .count(),
        duplicates: duplicates.len(),
        unresolved: inference.unresolved.len(),
    };
    let report = MapReport {
        schema_version: SCHEMA_VERSION,
        roots,
        skills,
        edges: inference.edges,
        duplicates,
        unresolved: inference.unresolved,
        counts,
        portfolio: resolved.portfolio,
        skipped: args.show_skipped.then(skipped_record),
    };
    let output = render::render(&report, args.format)?;
    io::stdout()
        .lock()
        .write_all(output.as_bytes())
        .map_err(|error| Error::io("write map to", Path::new("stdout"), error))?;
    Ok(())
}

fn selected_names(values: &[String]) -> Result<BTreeSet<String>, Error> {
    let mut selected = BTreeSet::new();
    let mut invalid = BTreeSet::new();
    for value in values {
        if SkillName::parse(value).is_ok() {
            selected.insert(value.clone());
        } else {
            invalid.insert(value.clone());
        }
    }
    if invalid.is_empty() { Ok(selected) } else { Err(Error::InvalidSkillFilter(invalid.into_iter().collect())) }
}

fn reject_unmappable_diagnostics(catalog: &Catalog) -> Result<(), Error> {
    let fatal = catalog.diagnostics.iter().find(|diagnostic| {
        diagnostic.code.starts_with("FRONTMATTER_") ||
            matches!(
                diagnostic.code.as_str(),
                "SKILL_DEPENDENCIES_EMPTY" |
                    "SKILL_DEPENDENCIES_NOT_ARRAY" |
                    "SKILL_DEPENDENCY_INVALID" |
                    "SKILL_DEPENDENCY_NOT_STRING"
            )
    });
    let Some(diagnostic) = fatal else {
        return Ok(());
    };
    let location = diagnostic.line.map(|line| format!(":{line}")).unwrap_or_default();
    Err(Error::MapData(format!(
        "cannot map {}{}: {} ({})",
        diagnostic.path.display(),
        location,
        diagnostic.message,
        diagnostic.code
    )))
}

fn skill_records(
    catalog: &Catalog,
    portfolio: Option<&PortfolioRecord>,
    selected: &BTreeSet<String>,
) -> Result<Vec<SkillRecord>, Error> {
    let mut hash_cache: BTreeMap<PathBuf, SkillHashes> = BTreeMap::new();
    let mut records = Vec::new();
    for skill in &catalog.skills {
        let name = effective_name(skill);
        if !selected.is_empty() && !selected.contains(&name) {
            continue;
        }
        let key = skill.resolved_skill_path().to_path_buf();
        let hashes = match hash_cache.get(&key) {
            Some(hashes) => hashes.clone(),
            None => {
                let hashes = skill.hashes()?;
                hash_cache.insert(key, hashes.clone());
                hashes
            }
        };
        let location = skill_location(skill, portfolio);
        let kind = skill_kind(skill, location);
        records.push(SkillRecord {
            name,
            directory_name: skill.directory_name.clone(),
            exposure_path: skill.skill_path().to_path_buf(),
            resolved_path: skill.resolved_skill_path().to_path_buf(),
            directory: skill.exposure.exposure_directory().to_path_buf(),
            resolved_directory: skill.exposure.resolved_directory().to_path_buf(),
            scope: skill.exposure.scope,
            location,
            kind,
            clients: clients(skill, kind),
            is_symlink: skill.exposure.directory_symlink_target.is_some(),
            symlink_target: skill.exposure.directory_symlink_target.clone(),
            skill_sha256: hashes.skill_sha256,
            tree_sha256: hashes.tree_sha256,
            skill_dependencies: skill
                .dependencies
                .iter()
                .map(|dependency| dependency.identifier.as_identifier())
                .collect(),
        });
    }
    records.sort_by(|left, right| (&left.name, &left.exposure_path).cmp(&(&right.name, &right.exposure_path)));
    Ok(records)
}

fn skill_location(skill: &Skill, portfolio: Option<&PortfolioRecord>) -> SkillLocation {
    let Some(portfolio) = portfolio else {
        return SkillLocation::ScannedRoot;
    };
    if portfolio.user_roots.iter().filter(|root| root.present).any(|root| skill.skill_path().starts_with(&root.path)) {
        SkillLocation::User
    } else {
        SkillLocation::Repository
    }
}

fn skill_kind(skill: &Skill, location: SkillLocation) -> SkillKind {
    if location == SkillLocation::User ||
        matches!(skill.exposure.scope, ExposureScope::Agents | ExposureScope::Claude | ExposureScope::Codex)
    {
        SkillKind::Install
    } else {
        SkillKind::Catalog
    }
}

fn clients(skill: &Skill, kind: SkillKind) -> Vec<String> {
    if kind == SkillKind::Install {
        return match skill.exposure.scope {
            ExposureScope::Claude => vec!["claude-code".to_owned()],
            ExposureScope::Agents | ExposureScope::Codex => vec!["codex".to_owned()],
            _ => vec!["claude-code".to_owned(), "codex".to_owned()],
        };
    }
    match skill
        .frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.install_targets.as_ref())
        .and_then(|targets| targets.value)
    {
        Some(InstallTargets::ClaudeCode) => vec!["claude-code".to_owned()],
        Some(InstallTargets::Codex) => vec!["codex".to_owned()],
        Some(InstallTargets::ClaudeCodeAndCodex) | None => {
            vec!["claude-code".to_owned(), "codex".to_owned()]
        }
    }
}

fn duplicate_records(catalog: &Catalog, selected: &BTreeSet<String>) -> Vec<DuplicateRecord> {
    let mut by_name: BTreeMap<String, Vec<&Skill>> = BTreeMap::new();
    for skill in &catalog.skills {
        by_name.entry(effective_name(skill)).or_default().push(skill);
    }
    let mut duplicates = Vec::new();
    for (name, skills) in by_name {
        if !selected.is_empty() && !selected.contains(&name) {
            continue;
        }
        let resolved_directories: BTreeSet<PathBuf> =
            skills.iter().map(|skill| skill.exposure.resolved_directory().to_path_buf()).collect();
        if resolved_directories.len() <= 1 {
            continue;
        }
        let exposure_paths = skills.iter().map(|skill| skill.skill_path().to_path_buf()).collect();
        duplicates.push(DuplicateRecord {
            duplicate_type: DuplicateType::DuplicateInstall,
            name,
            exposure_paths,
            resolved_directories: resolved_directories.into_iter().collect(),
        });
    }
    duplicates
}

fn skipped_record() -> SkippedRecord {
    let mut directories: Vec<_> =
        [".git", ".next", ".venv", "build", "coverage", "dist", "node_modules", "out", "target", "vendor"]
            .into_iter()
            .map(|name| format!("**/{name}/**"))
            .collect();
    directories.extend(
        [
            "**/.claude/backups/**",
            "**/.claude/debug/**",
            "**/.claude/file-history/**",
            "**/.claude/image-cache/**",
            "**/.claude/logs/**",
            "**/.claude/paste-cache/**",
            "**/.claude/plans/**",
            "**/.claude/projects/**",
            "**/.claude/session-env/**",
            "**/.claude/shell-snapshots/**",
            "**/.claude/statsig/**",
            "**/.claude/tasks/**",
            "**/.claude/todos/**",
            "**/.codex/.tmp/**",
            "**/.codex/archived_sessions/**",
            "**/.codex/backups/**",
            "**/.codex/cache/**",
            "**/.codex/generated_images/**",
            "**/.codex/log/**",
            "**/.codex/logs/**",
            "**/.codex/sessions/**",
            "**/.codex/shell_snapshots/**",
            "**/.codex/sqlite/**",
            "**/.codex/threads/**",
            "**/.codex/tmp/**",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    SkippedRecord {
        directories,
        files: vec![
            "**/.claude/history.jsonl".to_owned(),
            "**/.claude/remote-settings.json".to_owned(),
            "**/.claude/stats-cache.json".to_owned(),
            "**/.codex/history.jsonl".to_owned(),
            "**/.codex/session_index.jsonl".to_owned(),
            "**/.codex/*.sqlite*".to_owned(),
            "**/.codex/*.bak".to_owned(),
        ],
        macos_protected_home_paths: vec!["~/Library".to_owned(), "~/.Trash".to_owned()],
        always_ignored_home_paths: vec![
            "~/.agents".to_owned(),
            "~/.claude".to_owned(),
            "~/.codex".to_owned(),
            "~/.local/state/skills".to_owned(),
        ],
        broad_scan_cache_paths: [
            "~/.cache",
            "~/.npm",
            "~/.rustup",
            "~/.cargo/git",
            "~/.cargo/registry",
            "~/.bun/install/cache",
            "~/.pnpm-store",
            "~/.local/share/uv",
            "~/.local/share/rustup",
            "~/.local/share/cargo/git",
            "~/.local/share/cargo/registry",
            "~/.local/share/bun/install/cache",
            "~/.local/share/pnpm/store",
            "~/go/pkg/mod",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        catalog_sources: vec![
            "~/projects/agent-skills".to_owned(),
            "~/sablier/agent-skills".to_owned(),
            "~/sablier/sablier-skills".to_owned(),
        ],
    }
}
