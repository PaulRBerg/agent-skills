//! Atomic work arbitration over provider, process, and Git evidence.

mod bundle;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{Identity, Scope, ScopeKind, WorkState},
    error::{AppError, Result},
    host::{
        UNHASHABLE_BLOB_HASH, WorkClaimRequest, any_overlap, git_blob_hashes, git_dirty_paths, normalize_scopes,
        overlaps_outside_coverage, relevant_dirty, scopes_cover, scopes_overlap,
    },
    state::{BaselineRow, DirtObservationRow, ResidualOwnerRow, Store, WorkRow},
};

pub(crate) const DIRT_HOLD_SECONDS: f64 = 90.0;

pub(crate) struct WorkCoordinator<'a> {
    pub(crate) store: &'a mut Store,
}

#[derive(Clone, Debug)]
struct RepoEvidence {
    repo_root: String,
    dirty: Vec<String>,
    hashes: Vec<(String, String)>,
    benign: Vec<Scope>,
    inspection: Option<String>,
}

fn validate_claim_vector(claims: &[WorkClaimRequest]) -> Result<()> {
    if claims.is_empty() {
        return Err(AppError::usage("at least one repository claim is required"));
    }
    let mut previous: Option<String> = None;
    for claim in claims {
        let repo_root = path_text(&claim.repo_root)?;
        if !claim.repo_root.is_absolute() {
            return Err(AppError::usage(format!("repository root must be absolute: {repo_root}")));
        }
        if claim.scopes.is_empty() {
            return Err(AppError::usage(format!("at least one scope is required for {repo_root}")));
        }
        if previous.as_deref().is_some_and(|root| root >= repo_root.as_str()) {
            return Err(AppError::usage("repository claims must be sorted by distinct root"));
        }
        previous = Some(repo_root);
    }
    Ok(())
}

fn gather_evidence(
    claims: &[WorkClaimRequest],
    existing: Option<&WorkRow>,
    bundle_operation: bool,
) -> Result<Vec<RepoEvidence>> {
    let mut roots = BTreeMap::<String, (PathBuf, Vec<Scope>)>::new();
    for claim in claims {
        let repo_root = path_text(&claim.repo_root)?;
        roots.insert(repo_root, (claim.repo_root.clone(), claim.scopes.clone()));
    }
    if let Some(existing) = existing.filter(|work| work.state == WorkState::Active) {
        for claim in &existing.claims {
            roots
                .entry(claim.repo_root.clone())
                .and_modify(|(_, scopes)| merge_scopes(scopes, &claim.scopes))
                .or_insert_with(|| (PathBuf::from(&claim.repo_root), claim.scopes.clone()));
        }
    }

    roots
        .into_iter()
        .map(|(repo_root, (root, scopes))| match git_dirty_paths(&root) {
            Ok(dirty) => {
                let relevant = relevant_dirty(&scopes, &dirty);
                let hashes = git_blob_hashes(&root, &relevant, false);
                Ok(RepoEvidence { repo_root, dirty, hashes, benign: benign_dirt_scopes(&root), inspection: None })
            }
            Err(error) if bundle_operation => Ok(RepoEvidence {
                repo_root,
                dirty: Vec::new(),
                hashes: Vec::new(),
                benign: benign_dirt_scopes(&root),
                inspection: Some(error.to_string()),
            }),
            Err(error) => Err(error),
        })
        .collect()
}

fn blockers(
    work: &[WorkRow],
    identity: &Identity,
    repo_root: &str,
    scopes: &[Scope],
    state: WorkState,
    before: Option<f64>,
) -> Vec<WorkRow> {
    work.iter()
        .filter(|work| {
            work.state == state &&
                work.identity != *identity &&
                before.is_none_or(|submitted| work.submitted_at.is_some_and(|age| age < submitted)) &&
                work.claim(repo_root).is_some_and(|claim| any_overlap(scopes, &claim.scopes))
        })
        .cloned()
        .collect()
}

fn expansion_blockers(
    work: &[WorkRow],
    identity: &Identity,
    repo_root: &str,
    requested: &[Scope],
    existing: &[Scope],
) -> Vec<WorkRow> {
    work.iter()
        .filter(|work| {
            matches!(work.state, WorkState::Active | WorkState::Queued) &&
                work.identity != *identity &&
                work.claim(repo_root)
                    .is_some_and(|claim| !overlaps_outside_coverage(requested, &claim.scopes, existing).is_empty())
        })
        .cloned()
        .collect()
}

fn work_in_repo(work: &[WorkRow], repo_root: &str) -> Vec<WorkRow> {
    work.iter().filter(|work| work.claim(repo_root).is_some()).cloned().collect()
}

fn unattributed_dirty(dirty: &[String], work: &[WorkRow], repo_root: &str) -> Vec<String> {
    let owned = work
        .iter()
        .filter(|work| work.state == WorkState::Active)
        .filter_map(|work| work.claim(repo_root))
        .flat_map(|claim| claim.scopes.iter())
        .collect::<Vec<_>>();
    dirty
        .iter()
        .filter(|path| {
            let leaf = Scope { path: (*path).clone(), kind: ScopeKind::Exact };
            !owned.iter().any(|scope| scopes_overlap(scope, &leaf))
        })
        .cloned()
        .collect()
}

fn partition_dirty(
    dirty: &[String],
    observations: &[DirtObservationRow],
    residuals: &[ResidualOwnerRow],
    benign: &[Scope],
    identity: &Identity,
    current: f64,
) -> (Vec<String>, Vec<String>) {
    let mut fresh = Vec::new();
    let mut advisory = Vec::new();
    for path in dirty {
        let leaf = Scope { path: path.clone(), kind: ScopeKind::Exact };
        let benign = benign.iter().any(|scope| scopes_overlap(scope, &leaf));
        let residual_own = residuals.iter().any(|row| row.path == *path && row.identity == *identity);
        let stale = observations
            .iter()
            .find(|row| row.path == *path)
            .is_some_and(|row| current - row.first_seen >= DIRT_HOLD_SECONDS);
        if benign || residual_own || stale {
            advisory.push(path.clone());
        } else {
            fresh.push(path.clone());
        }
    }
    (fresh, advisory)
}

fn foreign_residuals(
    dirty: &[String],
    residuals: &[ResidualOwnerRow],
    benign: &[Scope],
    identity: &Identity,
) -> Vec<ResidualOwnerRow> {
    residuals
        .iter()
        .filter(|row| {
            row.identity != *identity &&
                dirty.contains(&row.path) &&
                !benign
                    .iter()
                    .any(|scope| scopes_overlap(scope, &Scope { path: row.path.clone(), kind: ScopeKind::Exact }))
        })
        .cloned()
        .collect()
}

fn benign_dirt_scopes(root: &Path) -> Vec<Scope> {
    let Ok(text) = fs::read_to_string(root.join(".agents/coord.toml")) else {
        return Vec::new();
    };
    let mut in_dirt = false;
    let mut value = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_dirt = line == "[dirt]";
            continue;
        }
        if in_dirt && line.starts_with("benign") {
            let Some((key, raw_value)) = line.split_once('=') else {
                return Vec::new();
            };
            if key.trim() != "benign" || value.is_some() {
                return Vec::new();
            }
            value = Some(raw_value.split('#').next().unwrap_or("").trim());
        }
    }
    let Some(value) = value else {
        return Vec::new();
    };
    let Ok(serde_json::Value::Array(values)) = serde_json::from_str::<serde_json::Value>(value) else {
        return Vec::new();
    };
    let Some(paths) = values.iter().map(|value| value.as_str().map(PathBuf::from)).collect::<Option<Vec<_>>>() else {
        return Vec::new();
    };
    normalize_scopes(&paths, root, root)
        .unwrap_or_default()
        .into_iter()
        .map(|path| Scope { path, kind: ScopeKind::Recursive })
        .collect()
}

fn write_baselines(root: &Path, paths: &[String]) -> Vec<BaselineRow> {
    git_blob_hashes(root, paths, true)
        .into_iter()
        .filter_map(|(path, oid)| (oid != UNHASHABLE_BLOB_HASH).then_some(BaselineRow { path, oid }))
        .collect()
}

fn merge_baselines(current: &mut Vec<BaselineRow>, additional: Vec<BaselineRow>) {
    for row in additional {
        if let Some(existing) = current.iter_mut().find(|existing| existing.path == row.path) {
            *existing = row;
        } else {
            current.push(row);
        }
    }
}

fn merge_scopes(current: &mut Vec<Scope>, additional: &[Scope]) {
    for scope in additional {
        if !current.contains(scope) {
            current.push(scope.clone());
        }
    }
}

fn evidence_for<'a>(evidence: &'a [RepoEvidence], repo_root: &str) -> &'a RepoEvidence {
    evidence.iter().find(|item| item.repo_root == repo_root).expect("evidence covers every claim")
}

fn existing_claim_scopes(work: &WorkRow) -> HashMap<String, Vec<Scope>> {
    work.claims.iter().map(|claim| (claim.repo_root.clone(), claim.scopes.clone())).collect()
}

fn work_vector_covers_requests(work: &WorkRow, requested: &[WorkClaimRequest]) -> bool {
    requested.iter().all(|claim| {
        claim
            .repo_root
            .to_str()
            .is_some_and(|root| work.claim(root).is_some_and(|existing| scopes_cover(&existing.scopes, &claim.scopes)))
    })
}

fn same_claim_vector(work: &WorkRow, requested: &[WorkClaimRequest]) -> bool {
    work.claims.len() == requested.len() &&
        requested.iter().all(|claim| {
            claim.repo_root.to_str().is_some_and(|root| {
                work.claim(root).is_some_and(|existing| same_scopes(&existing.scopes, &claim.scopes))
            })
        })
}

fn same_work_vectors(left: &WorkRow, right: &WorkRow) -> bool {
    left.claims.len() == right.claims.len() &&
        left.claims.iter().all(|claim| {
            right.claim(&claim.repo_root).is_some_and(|other| same_scopes(&claim.scopes, &other.scopes))
        })
}

fn same_scopes(left: &[Scope], right: &[Scope]) -> bool {
    left.len() == right.len() && left.iter().all(|scope| right.contains(scope))
}

fn work_vectors_overlap(left: &WorkRow, right: &WorkRow) -> bool {
    left.claims
        .iter()
        .any(|claim| right.claim(&claim.repo_root).is_some_and(|other| any_overlap(&claim.scopes, &other.scopes)))
}

fn request_work_overlap(requested: &[WorkClaimRequest], work: &WorkRow) -> bool {
    requested.iter().any(|claim| {
        claim
            .repo_root
            .to_str()
            .is_some_and(|root| work.claim(root).is_some_and(|other| any_overlap(&claim.scopes, &other.scopes)))
    })
}

fn request_paths(claims: &[WorkClaimRequest], qualified: bool) -> Vec<String> {
    claims
        .iter()
        .flat_map(|claim| claim.scopes.iter().map(move |scope| output_path(claim, &scope.path, qualified)))
        .collect()
}

fn work_paths(work: &WorkRow, qualified: bool) -> Vec<String> {
    work.claims
        .iter()
        .flat_map(|claim| {
            claim.scopes.iter().map(move |scope| {
                if qualified { qualify_path(&claim.repo_root, &scope.path) } else { scope.path.clone() }
            })
        })
        .collect()
}

fn output_path(claim: &WorkClaimRequest, path: &str, qualified: bool) -> String {
    if qualified { qualify_path(claim.repo_root.to_str().expect("validated root"), path) } else { path.to_owned() }
}

fn qualify_path(repo_root: &str, path: &str) -> String {
    if path == "." {
        repo_root.to_owned()
    } else {
        Path::new(repo_root).join(path).to_str().expect("validated repository path").to_owned()
    }
}

fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| AppError::usage("path is not valid UTF-8"))
}

fn sorted(values: HashSet<String>) -> Vec<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn scope(path: impl Into<String>, recursive: bool) -> Scope {
        Scope { path: path.into(), kind: if recursive { ScopeKind::Recursive } else { ScopeKind::Exact } }
    }

    proptest! {
        #[test]
        fn overlap_is_symmetric_for_exact_and_recursive_scopes(
            left in "[a-z]{1,5}(/[a-z]{1,5}){0,2}",
            right in "[a-z]{1,5}(/[a-z]{1,5}){0,2}",
            left_recursive in any::<bool>(),
            right_recursive in any::<bool>(),
        ) {
            let left = scope(left, left_recursive);
            let right = scope(right, right_recursive);
            prop_assert_eq!(scopes_overlap(&left, &right), scopes_overlap(&right, &left));
        }
    }

    #[test]
    fn exact_parent_does_not_cover_child_but_recursive_parent_does() {
        let child = scope("src/lib.rs", false);
        assert!(!scopes_overlap(&scope("src", false), &child));
        assert!(scopes_overlap(&scope("src", true), &child));
    }
}
