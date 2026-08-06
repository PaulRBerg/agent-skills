//! Atomic work arbitration over provider, process, and Git evidence.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{Identity, InventoryResult, Outcome, OutcomeKind, Scope, ScopeKind, WorkState},
    error::{AppError, Result},
    host::{
        UNHASHABLE_BLOB_HASH, any_overlap, git_blob_hash, git_dirty_paths, normalize_scopes, overlapping_paths,
        overlaps_outside_coverage, relevant_dirty, scopes_cover, scopes_overlap,
    },
    state::{BaselineRow, DirtObservationRow, ResidualOwnerRow, Store, WorkRow, WorkUpdate},
};

pub(crate) const DIRT_HOLD_SECONDS: f64 = 90.0;
const MAX_MESSAGE_CHARS: usize = 240;

pub(crate) struct WorkCoordinator<'a> {
    pub(crate) store: &'a mut Store,
}

impl WorkCoordinator<'_> {
    pub(crate) fn start_direct(
        &mut self,
        identity: &Identity,
        root: &Path,
        label: &str,
        scopes: Vec<Scope>,
        inventory: &InventoryResult,
        current: f64,
    ) -> Result<Outcome> {
        let existing = self.store.work(identity)?;
        if existing.as_ref().is_some_and(|work| work.state == WorkState::Draft) {
            return Err(AppError::operational(
                "a draft exists; update it with ai-coord draft, then submit it with ai-coord start --draft",
            ));
        }
        self.submit(identity, root, label, scopes, inventory, existing, None, current)
    }

    pub(crate) fn promote_draft(
        &mut self,
        identity: &Identity,
        root: &Path,
        draft: WorkRow,
        inventory: &InventoryResult,
        current: f64,
    ) -> Result<Outcome> {
        if draft.state != WorkState::Draft {
            return Err(AppError::operational("no draft work for this session"));
        }
        let revision = draft.revision;
        let label = draft.label.clone();
        self.submit(identity, root, &label, draft.scopes.clone(), inventory, Some(draft), Some(revision), current)
    }

    #[allow(clippy::too_many_arguments)]
    fn submit(
        &mut self,
        identity: &Identity,
        root: &Path,
        label: &str,
        scopes: Vec<Scope>,
        inventory: &InventoryResult,
        existing: Option<WorkRow>,
        draft_revision: Option<i64>,
        current: f64,
    ) -> Result<Outcome> {
        if scopes.is_empty() {
            return Err(AppError::usage("at least one scope is required"));
        }
        let repo_root = path_text(root)?;
        if let Some(active) = existing.as_ref().filter(|work| work.state == WorkState::Active) {
            return self.update_active(identity, root, &repo_root, label, scopes, inventory, active.clone(), current);
        }

        let (dirty, observations) = observe_git_dirt(self.store, root, current)?;
        let relevant = relevant_dirty(&scopes, &dirty);
        let benign = benign_dirt_scopes(root);
        let existing_scopes = existing.as_ref().map(|work| work.scopes.as_slice()).unwrap_or_default();
        let preserved_submission = if draft_revision.is_some() {
            None
        } else {
            existing
                .as_ref()
                .filter(|work| work.state == WorkState::Queued && scopes_cover(&work.scopes, &scopes))
                .and_then(|work| work.submitted_at)
        };

        let mut advisory = Vec::new();
        let outcome = self.store.with_work_transaction(|transaction| {
            let current_work = transaction.work(identity)?;
            match draft_revision {
                Some(revision) => {
                    let Some(work) = current_work
                        .as_ref()
                        .filter(|work| work.state == WorkState::Draft && work.revision == revision)
                    else {
                        return Err(AppError::retry("draft changed during promotion"));
                    };
                    if work.repo_root != repo_root {
                        return Err(AppError::retry("draft repository changed during promotion"));
                    }
                }
                None => {
                    if current_work.as_ref().is_some_and(|work| work.state == WorkState::Draft) {
                        return Err(AppError::operational(
                            "a draft exists; update it with ai-coord draft, then submit it with ai-coord start --draft",
                        ));
                    }
                    if current_work.as_ref().map(|work| work.revision) != existing.as_ref().map(|work| work.revision) {
                        return Err(AppError::retry("work item changed during arbitration"));
                    }
                }
            }

            let submitted_at = match preserved_submission {
                Some(submitted_at) => submitted_at,
                None => transaction.next_submission_time(current)?,
            };
            let work = transaction.works(&repo_root)?;
            let residuals = transaction.residual_owners(&repo_root)?;
            let active = blockers(&work, identity, &scopes, WorkState::Active, None);
            let earlier = blockers(&work, identity, &scopes, WorkState::Queued, Some(submitted_at));
            let unattributed = unattributed_dirty(&relevant, &work);
            let (fresh, stale) = partition_dirty(&unattributed, &observations, &residuals, &benign, identity, current);
            advisory = stale;
            let (state, blocked_reason, decision) = if !inventory.complete {
                (WorkState::Queued, Some("coverage".to_owned()), Outcome::new(OutcomeKind::Unknown, 2, "coverage"))
            } else if !fresh.is_empty() {
                (
                    WorkState::Queued,
                    Some("dirty".to_owned()),
                    Outcome::new(OutcomeKind::Unknown, 2, format!("dirty-settling:{}", fresh.join(","))),
                )
            } else if !active.is_empty() || !earlier.is_empty() {
                let contenders = if active.is_empty() { &earlier } else { &active };
                let reason = if active.is_empty() { "waiter" } else { "overlap" };
                (WorkState::Queued, Some(reason.to_owned()), blocked_outcome(&scopes, contenders, transaction)?)
            } else {
                let detail =
                    if advisory.is_empty() { String::new() } else { format!("stale-dirt:{}", advisory.join(",")) };
                (WorkState::Active, None, Outcome::new(OutcomeKind::Ready, 0, detail).with_paths(scope_paths(&scopes)))
            };

            let should_notify = existing.as_ref().is_none_or(|work| {
                work.blocked_reason.as_deref() != Some("overlap") || !same_scopes(existing_scopes, &scopes)
            });
            let expected_revision = current_work.as_ref().map(|work| work.revision);
            transaction.save_work(&WorkUpdate {
                identity: identity.clone(),
                repo_root: repo_root.clone(),
                label: label.to_owned(),
                state,
                blocked_reason,
                scopes: scopes.clone(),
                baselines: None,
                residual_paths: Vec::new(),
                draft_created_at: current_work.as_ref().and_then(|work| work.draft_created_at),
                submitted_at: Some(submitted_at),
                updated_at: current,
                expected_revision,
            })?;
            if should_notify {
                for holder in &active {
                    transaction.send_message(
                        identity,
                        &holder.identity,
                        &blocked_message(label, &scopes, holder),
                        Some(&repo_root),
                        current,
                    )?;
                }
            }
            Ok(decision)
        })?;
        if outcome.kind == OutcomeKind::Ready && !advisory.is_empty() {
            let baselines = write_baselines(root, &advisory);
            self.store.replace_baselines(identity, &baselines)?;
        }
        Ok(outcome)
    }

    #[allow(clippy::too_many_arguments)]
    fn update_active(
        &mut self,
        identity: &Identity,
        root: &Path,
        repo_root: &str,
        label: &str,
        scopes: Vec<Scope>,
        inventory: &InventoryResult,
        existing: WorkRow,
        current: f64,
    ) -> Result<Outcome> {
        if same_scopes(&existing.scopes, &scopes) {
            if existing.label != label {
                self.store.save_work(&WorkUpdate {
                    identity: identity.clone(),
                    repo_root: repo_root.to_owned(),
                    label: label.to_owned(),
                    state: WorkState::Active,
                    blocked_reason: None,
                    scopes: scopes.clone(),
                    baselines: None,
                    residual_paths: Vec::new(),
                    draft_created_at: existing.draft_created_at,
                    submitted_at: existing.submitted_at,
                    updated_at: current,
                    expected_revision: Some(existing.revision),
                })?;
            }
            return Ok(Outcome::new(OutcomeKind::Ready, 0, "").with_paths(scope_paths(&scopes)));
        }

        let (dirty, observations) = observe_git_dirt(self.store, root, current)?;
        let narrowing = scopes_cover(&existing.scopes, &scopes);
        let relevant = relevant_dirty(&scopes, &dirty);
        let benign = benign_dirt_scopes(root);
        let mut advisory = Vec::new();
        let outcome = self.store.with_work_transaction(|transaction| {
            let current_work = transaction.work(identity)?;
            let Some(current_work) = current_work.filter(|work| {
                work.state == WorkState::Active &&
                    work.revision == existing.revision &&
                    same_scopes(&work.scopes, &existing.scopes)
            }) else {
                return Err(AppError::retry("active work changed during scope update"));
            };

            let work = transaction.works(repo_root)?;
            let residuals = transaction.residual_owners(repo_root)?;
            let active = expansion_blockers(&work, identity, &scopes, &existing.scopes, WorkState::Active);
            let queued = expansion_blockers(&work, identity, &scopes, &existing.scopes, WorkState::Queued);
            let unattributed = unattributed_dirty(&relevant, &work);
            let (fresh, stale) = partition_dirty(&unattributed, &observations, &residuals, &benign, identity, current);
            advisory = stale;

            if !narrowing && !inventory.complete {
                return Ok(Outcome::new(OutcomeKind::Active, 3, "update-unknown:coverage")
                    .with_paths(scope_paths(&existing.scopes)));
            }
            if !narrowing && !fresh.is_empty() {
                return Ok(Outcome::new(
                    OutcomeKind::Active,
                    3,
                    format!("update-unknown:dirty-settling:{}", fresh.join(",")),
                )
                .with_paths(scope_paths(&existing.scopes)));
            }
            if !narrowing && (!active.is_empty() || !queued.is_empty()) {
                let mut contenders = active;
                contenders.extend(queued);
                let mut decision = blocked_outcome(&scopes, &contenders, transaction)?;
                decision.kind = OutcomeKind::Active;
                decision.detail = format!("update-blocked:{}", decision.holders.join(","));
                decision.paths = scope_paths(&existing.scopes);
                return Ok(decision);
            }

            let released = relevant_dirty(&existing.scopes, &dirty)
                .into_iter()
                .filter(|path| relevant_dirty(&scopes, std::slice::from_ref(path)).is_empty())
                .collect::<Vec<_>>();
            let mut baselines = transaction
                .baselines(identity)?
                .into_iter()
                .filter(|row| !relevant_dirty(&scopes, std::slice::from_ref(&row.path)).is_empty())
                .collect::<Vec<_>>();
            merge_baselines(&mut baselines, write_baselines(root, &advisory));
            let waiters = work
                .iter()
                .filter(|work| {
                    work.state == WorkState::Queued &&
                        any_overlap(&existing.scopes, &work.scopes) &&
                        !any_overlap(&scopes, &work.scopes)
                })
                .cloned()
                .collect::<Vec<_>>();
            transaction.save_work(&WorkUpdate {
                identity: identity.clone(),
                repo_root: repo_root.to_owned(),
                label: label.to_owned(),
                state: WorkState::Active,
                blocked_reason: None,
                scopes: scopes.clone(),
                baselines: Some(baselines),
                residual_paths: released,
                draft_created_at: current_work.draft_created_at,
                submitted_at: current_work.submitted_at,
                updated_at: current,
                expected_revision: Some(current_work.revision),
            })?;
            let message = sanitize(
                &format!("Narrowed work '{}'; your queued work may now be ready.", existing.label),
                MAX_MESSAGE_CHARS,
            );
            for waiter in waiters {
                transaction.send_message(identity, &waiter.identity, &message, Some(repo_root), current)?;
            }
            Ok(Outcome::new(OutcomeKind::Ready, 0, "").with_paths(scope_paths(&scopes)))
        })?;
        Ok(outcome)
    }
}

fn observe_git_dirt(store: &mut Store, root: &Path, current: f64) -> Result<(Vec<String>, Vec<DirtObservationRow>)> {
    let dirty = git_dirty_paths(root)?;
    let hashes = dirty.iter().map(|path| (path.clone(), git_blob_hash(root, path, false))).collect::<Vec<_>>();
    let observations = store.observe_dirt(&path_text(root)?, &hashes, current)?;
    Ok((dirty, observations))
}

fn blockers(
    work: &[WorkRow],
    identity: &Identity,
    scopes: &[Scope],
    state: WorkState,
    before: Option<f64>,
) -> Vec<WorkRow> {
    work.iter()
        .filter(|work| {
            work.state == state &&
                work.identity != *identity &&
                before.is_none_or(|submitted| work.submitted_at.is_some_and(|age| age < submitted)) &&
                any_overlap(scopes, &work.scopes)
        })
        .cloned()
        .collect()
}

fn expansion_blockers(
    work: &[WorkRow],
    identity: &Identity,
    requested: &[Scope],
    existing: &[Scope],
    state: WorkState,
) -> Vec<WorkRow> {
    work.iter()
        .filter(|work| {
            work.state == state &&
                work.identity != *identity &&
                !overlaps_outside_coverage(requested, &work.scopes, existing).is_empty()
        })
        .cloned()
        .collect()
}

fn unattributed_dirty(dirty: &[String], work: &[WorkRow]) -> Vec<String> {
    let owned = work
        .iter()
        .filter(|work| work.state == WorkState::Active)
        .flat_map(|work| work.scopes.iter())
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

fn blocked_outcome(
    requested: &[Scope],
    blockers: &[WorkRow],
    transaction: &crate::state::WorkTransaction<'_>,
) -> Result<Outcome> {
    let holders =
        blockers.iter().map(|work| identity_display(&work.identity, transaction)).collect::<Result<Vec<_>>>()?;
    let mut paths = blockers.iter().flat_map(|work| overlapping_paths(requested, &work.scopes)).collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let broad_paths = blockers
        .iter()
        .flat_map(|work| {
            requested
                .iter()
                .filter(|requested| {
                    work.scopes.iter().any(|owned| {
                        requested.is_recursive() &&
                            (requested.path == "." || owned.path.starts_with(&format!("{}/", requested.path)))
                    })
                })
                .map(|scope| scope.path.clone())
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>();
    Ok(Outcome {
        kind: OutcomeKind::Blocked,
        code: 3,
        detail: holders.join(","),
        paths,
        holders,
        broad_paths: sorted(broad_paths),
    })
}

fn identity_display(identity: &Identity, transaction: &crate::state::WorkTransaction<'_>) -> Result<String> {
    if let Some(callsign) = transaction.callsign(identity)? {
        return Ok(callsign);
    }
    let prefix = identity.session_id.chars().take(8).collect::<String>();
    Ok(format!("{}/{prefix}", client_name(identity)))
}

fn blocked_message(label: &str, requested: &[Scope], blocker: &WorkRow) -> String {
    let overlaps = overlapping_paths(requested, &blocker.scopes);
    let broad = blocker
        .scopes
        .iter()
        .filter(|owned| {
            requested.iter().any(|requested| {
                owned.is_recursive() && (owned.path == "." || requested.path.starts_with(&format!("{}/", owned.path)))
            })
        })
        .map(|scope| scope.path.clone())
        .collect::<Vec<_>>();
    let message = if broad.is_empty() {
        format!("Queued behind your work: {label}; overlaps: {}.", overlaps.join(", "))
    } else {
        format!(
            "Narrow broad work {} with ai-coord start if unrelated; queued work '{label}' overlaps: {}.",
            broad.join(", "),
            overlaps.join(", ")
        )
    };
    sanitize(&message, MAX_MESSAGE_CHARS)
}

fn benign_dirt_scopes(root: &Path) -> Vec<Scope> {
    let Ok(text) = fs::read_to_string(root.join(".ai-coord.toml")) else {
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
    paths
        .iter()
        .filter_map(|path| {
            let oid = git_blob_hash(root, path, true);
            (oid != UNHASHABLE_BLOB_HASH).then(|| BaselineRow { path: path.clone(), oid })
        })
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

fn same_scopes(left: &[Scope], right: &[Scope]) -> bool {
    left.len() == right.len() && left.iter().all(|scope| right.contains(scope))
}

fn scope_paths(scopes: &[Scope]) -> Vec<String> {
    scopes.iter().map(|scope| scope.path.clone()).collect()
}
fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| AppError::usage("path is not valid UTF-8"))
}
fn client_name(identity: &Identity) -> &'static str {
    match identity.client {
        crate::domain::Client::Codex => "codex",
        crate::domain::Client::Claude => "claude",
    }
}
fn sanitize(text: &str, limit: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= limit {
        return collapsed;
    }
    let mut value = collapsed.chars().take(limit.saturating_sub(1)).collect::<String>();
    value.push('…');
    value
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

    fn scope(path: String, recursive: bool) -> Scope {
        Scope { path, kind: if recursive { ScopeKind::Recursive } else { ScopeKind::Exact } }
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
        let child = scope("src/lib.rs".to_owned(), false);
        assert!(!scopes_overlap(&scope("src".to_owned(), false), &child));
        assert!(scopes_overlap(&scope("src".to_owned(), true), &child));
    }
}
