use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::{
    domain::{FindingKind, FindingState, FindingSummary},
    error::{AppError, Result},
    host::{git_head_oid, git_root, normalize_scopes},
    state::{FindingAdd, FindingAddResult, FindingPathObservation, FindingResolution},
};

use super::{Coordinator, path_text, resolved};

const MAX_FINDING_CHARS: usize = 1_000;

impl Coordinator {
    pub(crate) fn add_finding(
        &self,
        kind: Option<FindingKind>,
        paths: &[PathBuf],
        text: &str,
        cwd: &Path,
    ) -> Result<FindingAddResult> {
        let identity = self.identity(true)?.expect("required identity");
        let root = finding_root(cwd)?;
        let summary = normalize_summary(text)?;
        let paths = normalize_paths(paths, &root)?;
        let observations = paths
            .iter()
            .map(|path| FindingPathObservation { path: path.clone(), content_sha256: content_sha256(&root.join(path)) })
            .collect();
        self.store()?.add_finding(&FindingAdd {
            repo_root: path_text(&root)?,
            normalized_summary: summary.clone(),
            summary,
            kind,
            paths,
            head_oid: git_head_oid(&root),
            observations,
            author: identity,
            turn_id: turn_id(),
            current: self.clock.wall(),
        })
    }

    pub(crate) fn findings(
        &self,
        state: Option<FindingState>,
        include_terminal: bool,
        cwd: &Path,
    ) -> Result<Vec<FindingSummary>> {
        let root = finding_root(cwd)?;
        self.store()?.findings(&path_text(&root)?, state, include_terminal, self.clock.wall())
    }

    pub(crate) fn finding(&self, id: &str, cwd: &Path) -> Result<FindingSummary> {
        let root = finding_root(cwd)?;
        self.store()?
            .finding(&path_text(&root)?, id, self.clock.wall())?
            .ok_or_else(|| AppError::operational(format!("finding not found: {id}")))
    }

    pub(crate) fn handoff_finding(&self, id: &str, path: &Path, cwd: &Path) -> Result<FindingSummary> {
        let identity = self.identity(true)?.expect("required identity");
        let root = finding_root(cwd)?;
        let paths = normalize_paths(&[path.to_owned()], &root)?;
        let path = paths.first().ok_or_else(|| AppError::usage("handoff path is required"))?;
        self.store()?.handoff_finding(&path_text(&root)?, id, path, &identity, self.clock.wall())
    }

    pub(crate) fn resolve_finding(
        &self,
        id: &str,
        state: FindingState,
        commit_oid: Option<&str>,
        canonical_id: Option<&str>,
        cwd: &Path,
    ) -> Result<FindingSummary> {
        let identity = self.identity(true)?.expect("required identity");
        let root = finding_root(cwd)?;
        if let Some(oid) = commit_oid {
            validate_commit_oid(oid)?;
        }
        self.store()?.resolve_finding(
            &path_text(&root)?,
            id,
            &FindingResolution {
                state,
                commit_oid: commit_oid.map(str::to_owned),
                canonical_id: canonical_id.map(str::to_owned),
                actor: identity,
                current: self.clock.wall(),
            },
        )
    }

    pub(crate) fn reopen_finding(&self, id: &str, cwd: &Path) -> Result<FindingSummary> {
        let identity = self.identity(true)?.expect("required identity");
        let root = finding_root(cwd)?;
        self.store()?.reopen_finding(&path_text(&root)?, id, &identity, self.clock.wall())
    }
}

fn normalize_summary(text: &str) -> Result<String> {
    let summary = text.nfc().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    if summary.is_empty() {
        return Err(AppError::usage("finding summary must contain text"));
    }
    if summary.chars().count() > MAX_FINDING_CHARS {
        return Err(AppError::usage(format!("finding summary exceeds {MAX_FINDING_CHARS} Unicode code points")));
    }
    Ok(summary)
}

fn finding_root(cwd: &Path) -> Result<PathBuf> {
    git_root(&resolved(cwd)).ok_or_else(|| AppError::operational("finding requires a Git worktree"))
}

fn normalize_paths(paths: &[PathBuf], root: &Path) -> Result<Vec<String>> {
    let mut normalized = normalize_scopes(paths, root, root)?;
    for path in &normalized {
        let candidate = root.join(path);
        if std::fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            let target = std::fs::canonicalize(&candidate)
                .map_err(|_| AppError::usage(format!("finding path escapes repository: {path}")))?;
            target
                .strip_prefix(root)
                .map_err(|_| AppError::usage(format!("finding path escapes repository: {path}")))?;
        }
    }
    normalized.sort();
    Ok(normalized)
}

fn content_sha256(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Some(digest.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

fn turn_id() -> Option<String> {
    ["AI_COORD_TURN_ID", "CODEX_TURN_ID", "CLAUDE_CODE_TURN_ID"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
}

fn validate_commit_oid(oid: &str) -> Result<()> {
    if !(7..=64).contains(&oid.len()) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::usage("commit evidence must be a 7-64 character hexadecimal object ID"));
    }
    Ok(())
}
