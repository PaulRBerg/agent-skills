//! Atomic claim arbitration over provider, process, and Git evidence.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{ClaimState, Identity, InventoryResult, Outcome, OutcomeKind, Scope},
    error::{AppError, Result},
    host::{
        UNHASHABLE_BLOB_HASH, any_overlap, git_blob_hash, git_dirty_paths, normalize_scopes, overlapping_paths,
        overlaps_outside_coverage, relevant_dirty, scopes_cover, scopes_overlap,
    },
    state::{BaselineRow, ClaimRow, ClaimUpdate, DirtObservationRow, ResidualOwnerRow, Store},
};

pub(crate) const DIRT_HOLD_SECONDS: f64 = 90.0;
const MAX_MESSAGE_CHARS: usize = 240;

pub(crate) struct ClaimArbiter<'a> {
    pub(crate) store: &'a mut Store,
}

impl ClaimArbiter<'_> {
    pub(crate) fn start(
        &mut self,
        identity: &Identity,
        root: &Path,
        label: &str,
        scopes: Vec<Scope>,
        inventory: &InventoryResult,
        current: f64,
    ) -> Result<Outcome> {
        let repo_root = path_text(root)?;
        let existing = self.store.claim(identity)?;
        if scopes.is_empty() {
            return self.save_intent(identity, &repo_root, label, existing.as_ref(), current);
        }
        if let Some(active) = existing.as_ref().filter(|claim| claim.state == ClaimState::Active) {
            return self.update_active(identity, root, &repo_root, label, scopes, inventory, active.clone(), current);
        }

        let (dirty, observations) = observe_git_dirt(self.store, root, current)?;
        let relevant = relevant_dirty(&scopes, &dirty);
        let benign = benign_dirt_scopes(root);
        let existing_scopes = existing.as_ref().map(|claim| claim.scopes.as_slice()).unwrap_or_default();
        let created_at = existing
            .as_ref()
            .filter(|claim| claim.state == ClaimState::Queued && scopes_cover(&claim.scopes, &scopes))
            .map_or(current, |claim| claim.created_at);

        let mut advisory = Vec::new();
        let outcome = self.store.with_claim_transaction(|transaction| {
            let claims = transaction.claims(&repo_root)?;
            let residuals = transaction.residual_owners(&repo_root)?;
            let active = blockers(&claims, identity, &scopes, ClaimState::Active, None);
            let earlier = blockers(&claims, identity, &scopes, ClaimState::Queued, Some(created_at));
            let unattributed = unattributed_dirty(&relevant, &claims);
            let (fresh, stale) = partition_dirty(&unattributed, &observations, &residuals, &benign, identity, current);
            advisory = stale;
            let (state, blocked_reason, decision) = if !inventory.complete {
                (ClaimState::Queued, Some("coverage".to_owned()), Outcome::new(OutcomeKind::Unknown, 2, "coverage"))
            } else if !fresh.is_empty() {
                (
                    ClaimState::Queued,
                    Some("dirty".to_owned()),
                    Outcome::new(OutcomeKind::Unknown, 2, format!("dirty-settling:{}", fresh.join(","))),
                )
            } else if !active.is_empty() || !earlier.is_empty() {
                let contenders = if active.is_empty() { &earlier } else { &active };
                let reason = if active.is_empty() { "waiter" } else { "overlap" };
                (ClaimState::Queued, Some(reason.to_owned()), blocked_outcome(&scopes, contenders, transaction)?)
            } else {
                let detail =
                    if advisory.is_empty() { String::new() } else { format!("stale-dirt:{}", advisory.join(",")) };
                (ClaimState::Active, None, Outcome::new(OutcomeKind::Ready, 0, detail).with_paths(scope_paths(&scopes)))
            };

            let should_notify = existing.as_ref().is_none_or(|claim| {
                claim.blocked_reason.as_deref() != Some("overlap") || !same_scopes(existing_scopes, &scopes)
            });
            transaction.save_claim(&ClaimUpdate {
                identity: identity.clone(),
                repo_root: repo_root.clone(),
                label: label.to_owned(),
                state,
                blocked_reason,
                scopes: scopes.clone(),
                baselines: None,
                residual_paths: Vec::new(),
                created_at,
                updated_at: current,
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
        existing: ClaimRow,
        current: f64,
    ) -> Result<Outcome> {
        if same_scopes(&existing.scopes, &scopes) {
            if existing.label != label {
                self.store.save_claim(&ClaimUpdate {
                    identity: identity.clone(),
                    repo_root: repo_root.to_owned(),
                    label: label.to_owned(),
                    state: ClaimState::Active,
                    blocked_reason: None,
                    scopes: scopes.clone(),
                    baselines: None,
                    residual_paths: Vec::new(),
                    created_at: existing.created_at,
                    updated_at: current,
                })?;
            }
            return Ok(Outcome::new(OutcomeKind::Ready, 0, "").with_paths(scope_paths(&scopes)));
        }

        let (dirty, observations) = observe_git_dirt(self.store, root, current)?;
        let narrowing = scopes_cover(&existing.scopes, &scopes);
        let relevant = relevant_dirty(&scopes, &dirty);
        let benign = benign_dirt_scopes(root);
        let mut advisory = Vec::new();
        let outcome = self.store.with_claim_transaction(|transaction| {
            let current_claim = transaction.claim(identity)?;
            let Some(current_claim) = current_claim
                .filter(|claim| claim.state == ClaimState::Active && same_scopes(&claim.scopes, &existing.scopes))
            else {
                return Err(AppError::retry("active claim changed during scope update"));
            };

            let claims = transaction.claims(repo_root)?;
            let residuals = transaction.residual_owners(repo_root)?;
            let active = expansion_blockers(&claims, identity, &scopes, &existing.scopes, ClaimState::Active);
            let queued = expansion_blockers(&claims, identity, &scopes, &existing.scopes, ClaimState::Queued);
            let unattributed = unattributed_dirty(&relevant, &claims);
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
            let waiters = claims
                .iter()
                .filter(|claim| {
                    claim.state == ClaimState::Queued &&
                        any_overlap(&existing.scopes, &claim.scopes) &&
                        !any_overlap(&scopes, &claim.scopes)
                })
                .cloned()
                .collect::<Vec<_>>();
            transaction.save_claim(&ClaimUpdate {
                identity: identity.clone(),
                repo_root: repo_root.to_owned(),
                label: label.to_owned(),
                state: ClaimState::Active,
                blocked_reason: None,
                scopes: scopes.clone(),
                baselines: Some(baselines),
                residual_paths: released,
                created_at: current_claim.created_at,
                updated_at: current,
            })?;
            let message = sanitize(
                &format!("Narrowed claim '{}'; your queued claim may now be ready.", existing.label),
                MAX_MESSAGE_CHARS,
            );
            for waiter in waiters {
                transaction.send_message(identity, &waiter.identity, &message, Some(repo_root), current)?;
            }
            Ok(Outcome::new(OutcomeKind::Ready, 0, "").with_paths(scope_paths(&scopes)))
        })?;
        Ok(outcome)
    }

    fn save_intent(
        &mut self,
        identity: &Identity,
        repo_root: &str,
        label: &str,
        existing: Option<&ClaimRow>,
        current: f64,
    ) -> Result<Outcome> {
        let state = existing.map(|claim| claim.state).unwrap_or(ClaimState::Intent);
        let scopes = existing.map(|claim| claim.scopes.clone()).unwrap_or_default();
        self.store.save_claim(&ClaimUpdate {
            identity: identity.clone(),
            repo_root: repo_root.to_owned(),
            label: label.to_owned(),
            state,
            blocked_reason: existing.and_then(|claim| claim.blocked_reason.clone()),
            scopes: scopes.clone(),
            baselines: None,
            residual_paths: Vec::new(),
            created_at: existing.map(|claim| claim.created_at).unwrap_or(current),
            updated_at: current,
        })?;
        Ok(match state {
            ClaimState::Active => Outcome::new(OutcomeKind::Ready, 0, "").with_paths(scope_paths(&scopes)),
            ClaimState::Queued => {
                Outcome::new(OutcomeKind::Blocked, 3, "intent updated").with_paths(scope_paths(&scopes))
            }
            ClaimState::Intent => Outcome::new(OutcomeKind::Intent, 0, label),
        })
    }
}

fn observe_git_dirt(store: &mut Store, root: &Path, current: f64) -> Result<(Vec<String>, Vec<DirtObservationRow>)> {
    let dirty = git_dirty_paths(root)?;
    let hashes = dirty.iter().map(|path| (path.clone(), git_blob_hash(root, path, false))).collect::<Vec<_>>();
    let observations = store.observe_dirt(&path_text(root)?, &hashes, current)?;
    Ok((dirty, observations))
}

fn blockers(
    claims: &[ClaimRow],
    identity: &Identity,
    scopes: &[Scope],
    state: ClaimState,
    before: Option<f64>,
) -> Vec<ClaimRow> {
    claims
        .iter()
        .filter(|claim| {
            claim.state == state &&
                claim.identity != *identity &&
                before.is_none_or(|created| claim.created_at < created) &&
                any_overlap(scopes, &claim.scopes)
        })
        .cloned()
        .collect()
}

fn expansion_blockers(
    claims: &[ClaimRow],
    identity: &Identity,
    requested: &[Scope],
    existing: &[Scope],
    state: ClaimState,
) -> Vec<ClaimRow> {
    claims
        .iter()
        .filter(|claim| {
            claim.state == state &&
                claim.identity != *identity &&
                !overlaps_outside_coverage(requested, &claim.scopes, existing).is_empty()
        })
        .cloned()
        .collect()
}

fn unattributed_dirty(dirty: &[String], claims: &[ClaimRow]) -> Vec<String> {
    let owned = claims
        .iter()
        .filter(|claim| claim.state == ClaimState::Active)
        .flat_map(|claim| claim.scopes.iter())
        .collect::<Vec<_>>();
    dirty
        .iter()
        .filter(|path| {
            let leaf = Scope { path: (*path).clone(), recursive: false };
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
        let leaf = Scope { path: path.clone(), recursive: false };
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
    blockers: &[ClaimRow],
    transaction: &crate::state::ClaimTransaction<'_>,
) -> Result<Outcome> {
    let holders =
        blockers.iter().map(|claim| identity_display(&claim.identity, transaction)).collect::<Result<Vec<_>>>()?;
    let mut paths = blockers.iter().flat_map(|claim| overlapping_paths(requested, &claim.scopes)).collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let broad_paths = blockers
        .iter()
        .flat_map(|claim| {
            requested
                .iter()
                .filter(|requested| {
                    claim.scopes.iter().any(|owned| {
                        requested.recursive &&
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

fn identity_display(identity: &Identity, transaction: &crate::state::ClaimTransaction<'_>) -> Result<String> {
    if let Some(callsign) = transaction.callsign(identity)? {
        return Ok(callsign);
    }
    let prefix = identity.session_id.chars().take(8).collect::<String>();
    Ok(format!("{}/{prefix}", client_name(identity)))
}

fn blocked_message(label: &str, requested: &[Scope], blocker: &ClaimRow) -> String {
    let overlaps = overlapping_paths(requested, &blocker.scopes);
    let broad = blocker
        .scopes
        .iter()
        .filter(|owned| {
            requested.iter().any(|requested| {
                owned.recursive && (owned.path == "." || requested.path.starts_with(&format!("{}/", owned.path)))
            })
        })
        .map(|scope| scope.path.clone())
        .collect::<Vec<_>>();
    let message = if broad.is_empty() {
        format!("Queued behind your claim: {label}; overlaps: {}.", overlaps.join(", "))
    } else {
        format!(
            "Narrow broad claim {} with ai-coord start if unrelated; queued work '{label}' overlaps: {}.",
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
        .map(|path| Scope { path, recursive: true })
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

    proptest! {
        #[test]
        fn overlap_is_symmetric_for_exact_and_recursive_scopes(
            left in "[a-z]{1,5}(/[a-z]{1,5}){0,2}",
            right in "[a-z]{1,5}(/[a-z]{1,5}){0,2}",
            left_recursive in any::<bool>(),
            right_recursive in any::<bool>(),
        ) {
            let left = Scope { path: left, recursive: left_recursive };
            let right = Scope { path: right, recursive: right_recursive };
            prop_assert_eq!(scopes_overlap(&left, &right), scopes_overlap(&right, &left));
        }
    }

    #[test]
    fn exact_parent_does_not_cover_child_but_recursive_parent_does() {
        let child = Scope { path: "src/lib.rs".to_owned(), recursive: false };
        assert!(!scopes_overlap(&Scope { path: "src".to_owned(), recursive: false }, &child));
        assert!(scopes_overlap(&Scope { path: "src".to_owned(), recursive: true }, &child));
    }
}
