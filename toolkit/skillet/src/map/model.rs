use std::path::PathBuf;

use serde::Serialize;

use crate::traversal::{ExposureScope, RootMode};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MapReport {
    pub schema_version: u8,
    pub roots: Vec<RootRecord>,
    pub skills: Vec<SkillRecord>,
    pub edges: Vec<EvidenceRecord>,
    pub duplicates: Vec<DuplicateRecord>,
    pub unresolved: Vec<EvidenceRecord>,
    pub counts: Counts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio: Option<PortfolioRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<SkippedRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RootRecord {
    pub exposure_path: PathBuf,
    pub resolved_path: PathBuf,
    pub mode: RootMode,
    pub include_catalog_sources: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillLocation {
    Repository,
    User,
    ScannedRoot,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillKind {
    Catalog,
    Install,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SkillRecord {
    pub name: String,
    pub directory_name: String,
    pub exposure_path: PathBuf,
    pub resolved_path: PathBuf,
    pub directory: PathBuf,
    pub resolved_directory: PathBuf,
    pub scope: ExposureScope,
    pub location: SkillLocation,
    pub kind: SkillKind,
    pub clients: Vec<String>,
    pub is_symlink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symlink_target: Option<PathBuf>,
    pub skill_sha256: String,
    pub tree_sha256: String,
    pub skill_dependencies: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeType {
    Dependency,
    ExternalReference,
    UnresolvedLikeReference,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provenance {
    Declared,
    Inferred,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EvidenceRecord {
    #[serde(rename = "type")]
    pub edge_type: EdgeType,
    pub provenance: Provenance,
    pub identifier: String,
    pub source: Option<String>,
    pub target: String,
    pub path: PathBuf,
    pub line: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_repository: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DuplicateRecord {
    #[serde(rename = "type")]
    pub duplicate_type: DuplicateType,
    pub name: String,
    pub exposure_paths: Vec<PathBuf>,
    pub resolved_directories: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DuplicateType {
    DuplicateInstall,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Counts {
    pub skills: usize,
    pub edges: usize,
    pub declared_dependencies: usize,
    pub inferred_dependencies: usize,
    pub external_references: usize,
    pub duplicates: usize,
    pub unresolved: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortfolioRecord {
    pub requested_path: PathBuf,
    pub repository_root: PathBuf,
    pub user_roots: Vec<UserRootRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserRootRecord {
    pub path: PathBuf,
    pub client: String,
    pub present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SkippedRecord {
    pub directories: Vec<String>,
    pub files: Vec<String>,
    pub macos_protected_home_paths: Vec<String>,
    pub always_ignored_home_paths: Vec<String>,
    pub broad_scan_cache_paths: Vec<String>,
    pub catalog_sources: Vec<String>,
}
