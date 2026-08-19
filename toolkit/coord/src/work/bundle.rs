//! Claim-vector arbitration over provider, process, and Git evidence.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};

use super::{
    RepoEvidence, WorkCoordinator, blockers, evidence_for, existing_claim_scopes, expansion_blockers,
    foreign_residuals, gather_evidence, merge_baselines, output_path, partition_dirty, path_text, request_paths,
    request_work_overlap, same_claim_vector, same_work_vectors, sorted, unattributed_dirty, validate_claim_vector,
    work_in_repo, work_paths, work_vector_covers_requests, work_vectors_overlap, write_baselines,
};
use crate::{
    domain::{Identity, InventoryResult, Outcome, OutcomeKind, Scope, WorkState, client_name, sanitize},
    error::{AppError, Result},
    host::{WorkClaimRequest, overlapping_paths, relevant_dirty},
    state::{
        BaselineRow, DirtObservationRow, ResidualOwnerRow, WorkClaimRow, WorkClaimUpdate, WorkRow, WorkTransaction,
        WorkUpdate,
    },
};

const MAX_MESSAGE_CHARS: usize = 240;
const MAX_BUNDLE_CONFLICT_PATHS: usize = 32;

#[derive(Clone, Copy)]
enum Submission {
    Direct,
    Draft(i64),
}

enum ArbitrationStep {
    Complete(Outcome),
    Prepare(BTreeMap<String, Vec<String>>),
}

#[derive(Clone, Debug, Default)]
struct ClaimEvaluation {
    reason: Option<String>,
    fresh: Vec<String>,
    advisory: Vec<String>,
    contenders: Vec<WorkRow>,
    residuals: Vec<ResidualOwnerRow>,
}

impl WorkCoordinator<'_> {
    /// Behavior-compatible adapter for the ordinary single-repository path.
    pub(crate) fn start_direct(
        &mut self,
        identity: &Identity,
        root: &Path,
        label: &str,
        scopes: Vec<Scope>,
        inventory: &InventoryResult,
        current: f64,
    ) -> Result<Outcome> {
        self.start_claims(
            identity,
            label,
            vec![WorkClaimRequest { repo_root: root.to_owned(), scopes }],
            inventory,
            current,
        )
    }

    /// Submit one sorted, nonempty logical claim vector.
    pub(crate) fn start_claims(
        &mut self,
        identity: &Identity,
        label: &str,
        claims: Vec<WorkClaimRequest>,
        inventory: &InventoryResult,
        current: f64,
    ) -> Result<Outcome> {
        self.submit(identity, label, claims, inventory, Submission::Direct, current)
    }

    pub(crate) fn promote_draft(
        &mut self,
        identity: &Identity,
        draft: WorkRow,
        inventory: &InventoryResult,
        current: f64,
    ) -> Result<Outcome> {
        if draft.state != WorkState::Draft {
            return Err(AppError::operational("no draft work for this session"));
        }
        let claims = draft
            .claims
            .iter()
            .map(|claim| WorkClaimRequest { repo_root: PathBuf::from(&claim.repo_root), scopes: claim.scopes.clone() })
            .collect();
        self.submit(identity, &draft.label, claims, inventory, Submission::Draft(draft.revision), current)
    }

    fn submit(
        &mut self,
        identity: &Identity,
        label: &str,
        claims: Vec<WorkClaimRequest>,
        inventory: &InventoryResult,
        submission: Submission,
        current: f64,
    ) -> Result<Outcome> {
        validate_claim_vector(&claims)?;
        let existing = self.store.work(identity)?;
        match submission {
            Submission::Direct if existing.as_ref().is_some_and(|work| work.state == WorkState::Draft) => {
                return Err(AppError::operational(
                    "a draft exists; update it with ai-coord draft, then submit it with ai-coord start --draft",
                ));
            }
            Submission::Draft(revision)
                if !existing
                    .as_ref()
                    .is_some_and(|work| work.state == WorkState::Draft && work.revision == revision) =>
            {
                return Err(AppError::retry("draft changed during promotion"));
            }
            _ => {}
        }

        if let Some(active) = existing.as_ref().filter(|work| work.state == WorkState::Active) {
            return self.update_active(identity, label, claims, inventory, active, current);
        }

        let evidence = gather_evidence(&claims, existing.as_ref(), claims.len() > 1)?;
        let mut submitted_at = existing
            .as_ref()
            .filter(|work| {
                matches!(submission, Submission::Direct) &&
                    work.state == WorkState::Queued &&
                    work_vector_covers_requests(work, &claims)
            })
            .and_then(|work| work.submitted_at);
        let mut attempted_baselines = HashMap::<String, HashSet<String>>::new();
        let mut prepared_baselines = HashMap::<String, Vec<BaselineRow>>::new();
        loop {
            let step = self.store.with_work_transaction(|transaction| {
                let current_work = transaction.work(identity)?;
                verify_submission(current_work.as_ref(), existing.as_ref(), submission)?;
                let submitted_at = match submitted_at {
                    Some(submitted_at) => submitted_at,
                    None => {
                        let allocated = transaction.next_submission_time(current)?;
                        submitted_at = Some(allocated);
                        allocated
                    }
                };
                let work = transaction.works()?;
                let observations = refresh_observations(transaction, &evidence, current)?;
                let residuals = read_residuals(transaction, &claims)?;
                let evaluations = evaluate_claims(
                    &claims,
                    inventory.complete,
                    &evidence,
                    &observations,
                    &residuals,
                    &work,
                    identity,
                    submitted_at,
                    current,
                    None,
                );
                let blocked_reason = evaluations.iter().find_map(|evaluation| evaluation.reason.clone());
                let state = if blocked_reason.is_some() { WorkState::Queued } else { WorkState::Active };
                if state == WorkState::Active {
                    let missing = missing_advisory_baselines(&claims, &evaluations, &attempted_baselines);
                    if !missing.is_empty() {
                        return Ok(ArbitrationStep::Prepare(missing));
                    }
                }
                let decision = if state == WorkState::Active {
                    ready_outcome(&claims, &evaluations)
                } else {
                    blocked_outcome(&claims, &evaluations, transaction, None)?
                };
                let updates = claim_updates(
                    transaction,
                    identity,
                    &claims,
                    &evaluations,
                    &prepared_baselines,
                    state == WorkState::Active,
                )?;
                let should_notify = current_work.as_ref().is_none_or(|work| {
                    work.blocked_reason.as_deref() != Some("overlap") || !same_claim_vector(work, &claims)
                });
                transaction.save_work(&WorkUpdate {
                    identity: identity.clone(),
                    label: label.to_owned(),
                    state,
                    blocked_reason,
                    claims: updates,
                    draft_created_at: current_work.as_ref().and_then(|work| work.draft_created_at),
                    submitted_at: Some(submitted_at),
                    updated_at: current,
                    expected_revision: current_work.as_ref().map(|work| work.revision),
                })?;
                if should_notify {
                    notify_contenders(transaction, identity, label, &claims, &evaluations, current)?;
                }
                Ok(ArbitrationStep::Complete(decision))
            })?;
            match step {
                ArbitrationStep::Complete(outcome) => return Ok(outcome),
                ArbitrationStep::Prepare(missing) => {
                    prepare_baselines(missing, &mut attempted_baselines, &mut prepared_baselines);
                }
            }
        }
    }

    fn update_active(
        &mut self,
        identity: &Identity,
        label: &str,
        claims: Vec<WorkClaimRequest>,
        inventory: &InventoryResult,
        existing: &WorkRow,
        current: f64,
    ) -> Result<Outcome> {
        let same = same_claim_vector(existing, &claims);
        let qualified = claims.len() > 1 || existing.claims.len() > 1;
        if same {
            return self.store.with_work_transaction(|transaction| {
                let current_work = transaction.work(identity)?;
                verify_active(current_work.as_ref(), existing)?;
                if existing.label != label {
                    transaction.save_work(&WorkUpdate {
                        identity: identity.clone(),
                        label: label.to_owned(),
                        state: WorkState::Active,
                        blocked_reason: None,
                        claims: claims
                            .iter()
                            .map(|claim| WorkClaimUpdate {
                                repo_root: path_text(&claim.repo_root).expect("validated claim root"),
                                blocked_reason: None,
                                scopes: claim.scopes.clone(),
                                baselines: None,
                                residual_paths: Vec::new(),
                            })
                            .collect(),
                        draft_created_at: existing.draft_created_at,
                        submitted_at: existing.submitted_at,
                        updated_at: current,
                        expected_revision: Some(existing.revision),
                    })?;
                }
                Ok(Outcome::new(OutcomeKind::Ready, 0, "").with_paths(request_paths(&claims, qualified)))
            });
        }

        let narrowing = work_vector_covers_requests(existing, &claims);
        let evidence = gather_evidence(&claims, Some(existing), qualified)?;
        let mut attempted_baselines = HashMap::<String, HashSet<String>>::new();
        let mut prepared_baselines = HashMap::<String, Vec<BaselineRow>>::new();
        loop {
            let step = self.store.with_work_transaction(|transaction| {
                let current_work = transaction.work(identity)?;
                verify_active(current_work.as_ref(), existing)?;
                let work = transaction.works()?;
                let observations = refresh_observations(transaction, &evidence, current)?;
                let residuals =
                    read_residuals_for_roots(transaction, evidence.iter().map(|item| item.repo_root.as_str()))?;
                let existing_scopes = existing_claim_scopes(existing);
                let evaluations = evaluate_claims(
                    &claims,
                    inventory.complete,
                    &evidence,
                    &observations,
                    &residuals,
                    &work,
                    identity,
                    existing.submitted_at.unwrap_or(current),
                    current,
                    (!narrowing).then_some(&existing_scopes),
                );

                if let Some(inspection) = evidence.iter().find(|item| item.inspection.is_some()) {
                    return Ok(ArbitrationStep::Complete(
                        Outcome::new(
                            OutcomeKind::Active,
                            3,
                            format!("update-unknown:inspection:{}", inspection.repo_root),
                        )
                        .with_paths(work_paths(existing, qualified)),
                    ));
                }
                if !narrowing && evaluations.iter().any(|evaluation| evaluation.reason.is_some()) {
                    return blocked_outcome(&claims, &evaluations, transaction, Some((existing, qualified)))
                        .map(ArbitrationStep::Complete);
                }

                let successful_evaluations = if narrowing {
                    advisory_evaluations(&claims, &evidence, &observations, &residuals, &work, identity, current)
                } else {
                    evaluations
                };
                let missing = missing_advisory_baselines(&claims, &successful_evaluations, &attempted_baselines);
                if !missing.is_empty() {
                    return Ok(ArbitrationStep::Prepare(missing));
                }
                let released = released_paths(existing, &claims, &evidence);
                for (repo_root, paths) in &released {
                    transaction.record_residual_owners(repo_root, paths, identity, current)?;
                }
                let waiters = newly_unblocked_waiters(existing, &claims, &work, identity);
                let updates =
                    claim_updates(transaction, identity, &claims, &successful_evaluations, &prepared_baselines, true)?;
                transaction.save_work(&WorkUpdate {
                    identity: identity.clone(),
                    label: label.to_owned(),
                    state: WorkState::Active,
                    blocked_reason: None,
                    claims: updates,
                    draft_created_at: existing.draft_created_at,
                    submitted_at: existing.submitted_at,
                    updated_at: current,
                    expected_revision: Some(existing.revision),
                })?;
                notify_waiters(transaction, identity, existing, &waiters, qualified, current)?;
                Ok(ArbitrationStep::Complete(ready_outcome(&claims, &successful_evaluations)))
            })?;
            match step {
                ArbitrationStep::Complete(outcome) => return Ok(outcome),
                ArbitrationStep::Prepare(missing) => {
                    prepare_baselines(missing, &mut attempted_baselines, &mut prepared_baselines);
                }
            }
        }
    }
}

fn verify_submission(current: Option<&WorkRow>, expected: Option<&WorkRow>, submission: Submission) -> Result<()> {
    match submission {
        Submission::Draft(revision) => {
            if !current.is_some_and(|work| work.state == WorkState::Draft && work.revision == revision) {
                return Err(AppError::retry("draft changed during promotion"));
            }
        }
        Submission::Direct => {
            if current.is_some_and(|work| work.state == WorkState::Draft) {
                return Err(AppError::operational(
                    "a draft exists; update it with ai-coord draft, then submit it with ai-coord start --draft",
                ));
            }
            if current.map(|work| work.revision) != expected.map(|work| work.revision) {
                return Err(AppError::retry("work item changed during arbitration"));
            }
        }
    }
    Ok(())
}

fn verify_active(current: Option<&WorkRow>, expected: &WorkRow) -> Result<()> {
    if !current.is_some_and(|work| {
        work.state == WorkState::Active && work.revision == expected.revision && same_work_vectors(work, expected)
    }) {
        return Err(AppError::retry("active work changed during scope update"));
    }
    Ok(())
}

fn refresh_observations(
    transaction: &WorkTransaction<'_>,
    evidence: &[RepoEvidence],
    current: f64,
) -> Result<HashMap<String, Vec<DirtObservationRow>>> {
    evidence
        .iter()
        .filter(|item| item.inspection.is_none())
        .map(|item| {
            transaction
                .observe_dirt_subset(&item.repo_root, &item.dirty, &item.hashes, current)
                .map(|observations| (item.repo_root.clone(), observations))
        })
        .collect()
}

fn read_residuals(
    transaction: &WorkTransaction<'_>,
    claims: &[WorkClaimRequest],
) -> Result<HashMap<String, Vec<ResidualOwnerRow>>> {
    read_residuals_for_roots(transaction, claims.iter().map(|claim| claim.repo_root.to_str().expect("validated root")))
}

fn read_residuals_for_roots<'a>(
    transaction: &WorkTransaction<'_>,
    roots: impl IntoIterator<Item = &'a str>,
) -> Result<HashMap<String, Vec<ResidualOwnerRow>>> {
    roots.into_iter().map(|root| transaction.residual_owners(root).map(|rows| (root.to_owned(), rows))).collect()
}

#[allow(clippy::too_many_arguments)]
fn evaluate_claims(
    claims: &[WorkClaimRequest],
    coverage_complete: bool,
    evidence: &[RepoEvidence],
    observations: &HashMap<String, Vec<DirtObservationRow>>,
    residuals: &HashMap<String, Vec<ResidualOwnerRow>>,
    work: &[WorkRow],
    identity: &Identity,
    submitted_at: f64,
    current: f64,
    expansion_from: Option<&HashMap<String, Vec<Scope>>>,
) -> Vec<ClaimEvaluation> {
    claims
        .iter()
        .map(|claim| {
            let repo_root = claim.repo_root.to_str().expect("validated root");
            if !coverage_complete {
                return ClaimEvaluation { reason: Some("coverage".to_owned()), ..ClaimEvaluation::default() };
            }
            let evidence = evidence_for(evidence, repo_root);
            if evidence.inspection.is_some() {
                return ClaimEvaluation { reason: Some("inspection".to_owned()), ..ClaimEvaluation::default() };
            }
            let repo_work = work_in_repo(work, repo_root);
            let relevant = relevant_dirty(&claim.scopes, &evidence.dirty);
            let unattributed = unattributed_dirty(&relevant, &repo_work, repo_root);
            let repo_residuals = residuals.get(repo_root).map(Vec::as_slice).unwrap_or_default();
            let repo_observations = observations.get(repo_root).map(Vec::as_slice).unwrap_or_default();
            let (fresh, advisory) =
                partition_dirty(&unattributed, repo_observations, repo_residuals, &evidence.benign, identity, current);
            if !fresh.is_empty() {
                return ClaimEvaluation {
                    reason: Some("dirty".to_owned()),
                    fresh,
                    advisory,
                    ..ClaimEvaluation::default()
                };
            }
            let foreign = foreign_residuals(&unattributed, repo_residuals, &evidence.benign, identity);
            if !foreign.is_empty() {
                return ClaimEvaluation {
                    reason: Some("residual".to_owned()),
                    advisory,
                    residuals: foreign,
                    ..ClaimEvaluation::default()
                };
            }
            if let Some(existing) = expansion_from {
                let covered = existing.get(repo_root).map(Vec::as_slice).unwrap_or_default();
                let contenders = expansion_blockers(&repo_work, identity, repo_root, &claim.scopes, covered);
                if !contenders.is_empty() {
                    return ClaimEvaluation {
                        reason: Some("overlap".to_owned()),
                        advisory,
                        contenders,
                        ..ClaimEvaluation::default()
                    };
                }
            } else {
                let active = blockers(&repo_work, identity, repo_root, &claim.scopes, WorkState::Active, None);
                if !active.is_empty() {
                    return ClaimEvaluation {
                        reason: Some("overlap".to_owned()),
                        advisory,
                        contenders: active,
                        ..ClaimEvaluation::default()
                    };
                }
                let earlier =
                    blockers(&repo_work, identity, repo_root, &claim.scopes, WorkState::Queued, Some(submitted_at));
                if !earlier.is_empty() {
                    return ClaimEvaluation {
                        reason: Some("waiter".to_owned()),
                        advisory,
                        contenders: earlier,
                        ..ClaimEvaluation::default()
                    };
                }
            }
            ClaimEvaluation { advisory, ..ClaimEvaluation::default() }
        })
        .collect()
}

fn advisory_evaluations(
    claims: &[WorkClaimRequest],
    evidence: &[RepoEvidence],
    observations: &HashMap<String, Vec<DirtObservationRow>>,
    residuals: &HashMap<String, Vec<ResidualOwnerRow>>,
    work: &[WorkRow],
    identity: &Identity,
    current: f64,
) -> Vec<ClaimEvaluation> {
    claims
        .iter()
        .map(|claim| {
            let repo_root = claim.repo_root.to_str().expect("validated root");
            let evidence = evidence_for(evidence, repo_root);
            let relevant = relevant_dirty(&claim.scopes, &evidence.dirty);
            let unattributed = unattributed_dirty(&relevant, &work_in_repo(work, repo_root), repo_root);
            let (_, advisory) = partition_dirty(
                &unattributed,
                observations.get(repo_root).map(Vec::as_slice).unwrap_or_default(),
                residuals.get(repo_root).map(Vec::as_slice).unwrap_or_default(),
                &evidence.benign,
                identity,
                current,
            );
            ClaimEvaluation { advisory, ..ClaimEvaluation::default() }
        })
        .collect()
}

fn ready_outcome(claims: &[WorkClaimRequest], evaluations: &[ClaimEvaluation]) -> Outcome {
    let qualified = claims.len() > 1;
    let mut advisory = claims
        .iter()
        .zip(evaluations)
        .flat_map(|(claim, evaluation)| evaluation.advisory.iter().map(move |path| output_path(claim, path, qualified)))
        .collect::<Vec<_>>();
    if qualified {
        advisory.sort();
        advisory.dedup();
        advisory.truncate(MAX_BUNDLE_CONFLICT_PATHS);
    }
    let detail = if advisory.is_empty() { String::new() } else { format!("stale-dirt:{}", advisory.join(",")) };
    Outcome::new(OutcomeKind::Ready, 0, detail).with_paths(request_paths(claims, qualified))
}

fn blocked_outcome(
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    transaction: &WorkTransaction<'_>,
    active: Option<(&WorkRow, bool)>,
) -> Result<Outcome> {
    let first = evaluations
        .iter()
        .enumerate()
        .find(|(_, evaluation)| evaluation.reason.is_some())
        .expect("blocked outcome requires a blocker");
    let reason = first.1.reason.as_deref().expect("blocked reason");
    let qualified = active.map_or(claims.len() > 1, |(_, qualified)| qualified);
    let prefix = if active.is_some() { "update-unknown:" } else { "" };
    match reason {
        "coverage" => {
            let kind = if active.is_some() { OutcomeKind::Active } else { OutcomeKind::Unknown };
            let code = if active.is_some() { 3 } else { 2 };
            let mut outcome = Outcome::new(kind, code, format!("{prefix}coverage"));
            if let Some((old, _)) = active {
                outcome.paths = work_paths(old, qualified);
            }
            Ok(outcome)
        }
        "inspection" => {
            let root = claims[first.0].repo_root.to_str().expect("validated root");
            let kind = if active.is_some() { OutcomeKind::Active } else { OutcomeKind::Unknown };
            let code = if active.is_some() { 3 } else { 2 };
            let mut outcome = Outcome::new(kind, code, format!("{prefix}inspection:{root}"));
            if let Some((old, _)) = active {
                outcome.paths = work_paths(old, qualified);
            }
            Ok(outcome)
        }
        "dirty" => {
            let mut paths = claims
                .iter()
                .zip(evaluations)
                .flat_map(|(claim, evaluation)| {
                    evaluation.fresh.iter().map(move |path| output_path(claim, path, qualified))
                })
                .collect::<Vec<_>>();
            if qualified {
                paths.sort();
                paths.dedup();
                paths.truncate(MAX_BUNDLE_CONFLICT_PATHS);
            }
            let kind = if active.is_some() { OutcomeKind::Active } else { OutcomeKind::Unknown };
            let code = if active.is_some() { 3 } else { 2 };
            let mut outcome = Outcome::new(kind, code, format!("{prefix}dirty-settling:{}", paths.join(",")));
            if let Some((old, _)) = active {
                outcome.paths = work_paths(old, qualified);
            }
            Ok(outcome)
        }
        "residual" | "overlap" | "waiter" => {
            let (holders, mut paths, broad_paths) = conflict_detail(claims, evaluations, transaction, qualified)?;
            if qualified {
                paths.truncate(MAX_BUNDLE_CONFLICT_PATHS);
            }
            if let Some((old, _)) = active {
                Ok(Outcome {
                    kind: OutcomeKind::Active,
                    code: 3,
                    detail: format!("update-blocked:{}", holders.join(",")),
                    paths: work_paths(old, qualified),
                    holders,
                    broad_paths,
                })
            } else {
                Ok(Outcome {
                    kind: OutcomeKind::Blocked,
                    code: 3,
                    detail: holders.join(","),
                    paths,
                    holders,
                    broad_paths,
                })
            }
        }
        _ => unreachable!("unknown blocker reason"),
    }
}

fn conflict_detail(
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    transaction: &WorkTransaction<'_>,
    qualified: bool,
) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    let mut identities = Vec::<Identity>::new();
    let mut paths = Vec::new();
    let mut broad = HashSet::new();
    for (claim, evaluation) in claims.iter().zip(evaluations) {
        for residual in &evaluation.residuals {
            if !identities.contains(&residual.identity) {
                identities.push(residual.identity.clone());
            }
            paths.push(output_path(claim, &residual.path, qualified));
        }
        let repo_root = claim.repo_root.to_str().expect("validated root");
        for contender in &evaluation.contenders {
            if !identities.contains(&contender.identity) {
                identities.push(contender.identity.clone());
            }
            let contender_claim = contender.claim(repo_root).expect("repository contender");
            paths.extend(
                overlapping_paths(&claim.scopes, &contender_claim.scopes)
                    .iter()
                    .map(|path| output_path(claim, path, qualified)),
            );
            for requested in &claim.scopes {
                if requested.is_recursive() &&
                    contender_claim.scopes.iter().any(|owned| {
                        requested.path == "." || owned.path.starts_with(&format!("{}/", requested.path))
                    })
                {
                    broad.insert(output_path(claim, &requested.path, qualified));
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    let holders =
        identities.iter().map(|identity| identity_display(identity, transaction)).collect::<Result<Vec<_>>>()?;
    let mut broad = sorted(broad);
    if qualified {
        broad.truncate(MAX_BUNDLE_CONFLICT_PATHS);
    }
    Ok((holders, paths, broad))
}

fn identity_display(identity: &Identity, transaction: &WorkTransaction<'_>) -> Result<String> {
    if let Some(callsign) = transaction.callsign(identity)? {
        return Ok(callsign);
    }
    let prefix = identity.session_id.chars().take(8).collect::<String>();
    Ok(format!("{}/{prefix}", client_name(identity.client)))
}

fn claim_updates(
    transaction: &WorkTransaction<'_>,
    identity: &Identity,
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    prepared_baselines: &HashMap<String, Vec<BaselineRow>>,
    active: bool,
) -> Result<Vec<WorkClaimUpdate>> {
    claims
        .iter()
        .zip(evaluations)
        .map(|(claim, evaluation)| {
            let repo_root = path_text(&claim.repo_root)?;
            let baselines = if active {
                let mut baselines = transaction
                    .baselines_in_repo(identity, &repo_root)?
                    .into_iter()
                    .filter(|row| !relevant_dirty(&claim.scopes, std::slice::from_ref(&row.path)).is_empty())
                    .collect::<Vec<_>>();
                let advisory = evaluation.advisory.iter().collect::<HashSet<_>>();
                merge_baselines(
                    &mut baselines,
                    prepared_baselines
                        .get(&repo_root)
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                        .iter()
                        .filter(|row| advisory.contains(&row.path))
                        .cloned()
                        .collect(),
                );
                baselines.sort_by(|left, right| left.path.cmp(&right.path));
                Some(baselines)
            } else {
                None
            };
            Ok(WorkClaimUpdate {
                repo_root,
                blocked_reason: evaluation.reason.clone(),
                scopes: claim.scopes.clone(),
                baselines,
                residual_paths: Vec::new(),
            })
        })
        .collect()
}

fn missing_advisory_baselines(
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    attempted: &HashMap<String, HashSet<String>>,
) -> BTreeMap<String, Vec<String>> {
    claims
        .iter()
        .zip(evaluations)
        .filter_map(|(claim, evaluation)| {
            let repo_root = claim.repo_root.to_str().expect("validated root");
            let attempted = attempted.get(repo_root);
            let missing = evaluation
                .advisory
                .iter()
                .filter(|path| !attempted.is_some_and(|attempted| attempted.contains(path.as_str())))
                .cloned()
                .collect::<Vec<_>>();
            (!missing.is_empty()).then(|| (repo_root.to_owned(), missing))
        })
        .collect()
}

fn prepare_baselines(
    missing: BTreeMap<String, Vec<String>>,
    attempted: &mut HashMap<String, HashSet<String>>,
    prepared: &mut HashMap<String, Vec<BaselineRow>>,
) {
    for (repo_root, paths) in missing {
        merge_baselines(prepared.entry(repo_root.clone()).or_default(), write_baselines(Path::new(&repo_root), &paths));
        attempted.entry(repo_root).or_default().extend(paths);
    }
}

fn notify_contenders(
    transaction: &WorkTransaction<'_>,
    identity: &Identity,
    label: &str,
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    current: f64,
) -> Result<()> {
    let qualified = claims.len() > 1;
    let mut notified = Vec::<Identity>::new();
    for (claim, evaluation) in claims.iter().zip(evaluations) {
        if evaluation.reason.as_deref() != Some("overlap") {
            continue;
        }
        let repo_root = claim.repo_root.to_str().expect("validated root");
        for contender in &evaluation.contenders {
            if notified.contains(&contender.identity) {
                continue;
            }
            let message = if qualified {
                bundle_blocked_message(label, claims, evaluations, contender)
            } else {
                blocked_message(label, &claim.scopes, contender.claim(repo_root).expect("repository contender"))
            };
            transaction.send_message(
                identity,
                &contender.identity,
                &message,
                (!qualified).then_some(repo_root),
                current,
            )?;
            notified.push(contender.identity.clone());
        }
    }
    Ok(())
}

fn blocked_message(label: &str, requested: &[Scope], blocker: &WorkClaimRow) -> String {
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

fn bundle_blocked_message(
    label: &str,
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    blocker: &WorkRow,
) -> String {
    let mut overlaps = Vec::new();
    for (claim, evaluation) in claims.iter().zip(evaluations) {
        if !evaluation.contenders.iter().any(|candidate| candidate.identity == blocker.identity) {
            continue;
        }
        let repo_root = claim.repo_root.to_str().expect("validated root");
        if let Some(owned) = blocker.claim(repo_root) {
            overlaps.extend(
                overlapping_paths(&claim.scopes, &owned.scopes).iter().map(|path| output_path(claim, path, true)),
            );
        }
    }
    overlaps.sort();
    overlaps.dedup();
    overlaps.truncate(MAX_BUNDLE_CONFLICT_PATHS);
    sanitize(&format!("Queued behind your work: {label}; overlaps: {}.", overlaps.join(", ")), MAX_MESSAGE_CHARS)
}

fn released_paths(
    existing: &WorkRow,
    claims: &[WorkClaimRequest],
    evidence: &[RepoEvidence],
) -> BTreeMap<String, Vec<String>> {
    existing
        .claims
        .iter()
        .filter_map(|old| {
            let evidence = evidence_for(evidence, &old.repo_root);
            let old_dirty = relevant_dirty(&old.scopes, &evidence.dirty);
            let new_scopes = claims
                .iter()
                .find(|claim| claim.repo_root.to_str() == Some(old.repo_root.as_str()))
                .map(|claim| claim.scopes.as_slice())
                .unwrap_or_default();
            let released = old_dirty
                .into_iter()
                .filter(|path| relevant_dirty(new_scopes, std::slice::from_ref(path)).is_empty())
                .collect::<Vec<_>>();
            (!released.is_empty()).then(|| (old.repo_root.clone(), released))
        })
        .collect()
}

fn newly_unblocked_waiters(
    existing: &WorkRow,
    claims: &[WorkClaimRequest],
    work: &[WorkRow],
    identity: &Identity,
) -> Vec<WorkRow> {
    let mut waiters = work
        .iter()
        .filter(|candidate| {
            candidate.state == WorkState::Queued &&
                candidate.identity != *identity &&
                work_vectors_overlap(existing, candidate) &&
                !request_work_overlap(claims, candidate)
        })
        .cloned()
        .collect::<Vec<_>>();
    waiters.dedup_by(|left, right| left.identity == right.identity);
    waiters
}

fn notify_waiters(
    transaction: &WorkTransaction<'_>,
    identity: &Identity,
    existing: &WorkRow,
    waiters: &[WorkRow],
    qualified: bool,
    current: f64,
) -> Result<()> {
    let message =
        sanitize(&format!("Narrowed work '{}'; your queued work may now be ready.", existing.label), MAX_MESSAGE_CHARS);
    let repo_root = (!qualified && existing.claims.len() == 1).then_some(existing.claims[0].repo_root.as_str());
    for waiter in waiters {
        transaction.send_message(identity, &waiter.identity, &message, repo_root, current)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::domain::{Client, ScopeKind};

    use super::*;

    fn scope(path: impl Into<String>, recursive: bool) -> Scope {
        Scope { path: path.into(), kind: if recursive { ScopeKind::Recursive } else { ScopeKind::Exact } }
    }

    fn claim(root: &str, scopes: Vec<Scope>) -> WorkClaimRequest {
        WorkClaimRequest { repo_root: PathBuf::from(root), scopes }
    }

    fn work(identity: &str, state: WorkState, claims: Vec<WorkClaimRequest>, submitted_at: f64) -> WorkRow {
        WorkRow {
            id: 1,
            identity: Identity { client: Client::Codex, session_id: identity.to_owned() },
            label: identity.to_owned(),
            state,
            blocked_reason: None,
            claims: claims
                .into_iter()
                .enumerate()
                .map(|(id, claim)| WorkClaimRow {
                    id: id as i64,
                    repo_root: claim.repo_root.to_string_lossy().into_owned(),
                    blocked_reason: None,
                    scopes: claim.scopes,
                })
                .collect(),
            draft_created_at: None,
            submitted_at: Some(submitted_at),
            updated_at: submitted_at,
            revision: 1,
        }
    }

    #[test]
    fn claim_vector_coverage_is_repository_local() {
        let old = work(
            "owner",
            WorkState::Queued,
            vec![claim("/a", vec![scope("src", true)]), claim("/b", vec![scope("tests", true)])],
            1.0,
        );
        assert!(work_vector_covers_requests(&old, &[claim("/a", vec![scope("src/lib.rs", false)])]));
        assert!(!work_vector_covers_requests(&old, &[claim("/a", vec![scope("tests/unit.rs", false)])]));
        assert!(!work_vector_covers_requests(&old, &[claim("/c", vec![scope("src/lib.rs", false)])]));
    }

    #[test]
    fn overlap_and_fifo_use_the_contenders_matching_repository_claim() {
        let owner = Identity { client: Client::Codex, session_id: "owner".to_owned() };
        let contender = work(
            "other",
            WorkState::Queued,
            vec![claim("/a", vec![scope("unrelated.rs", false)]), claim("/b", vec![scope("src/lib.rs", false)])],
            1.0,
        );
        assert!(
            blockers(
                std::slice::from_ref(&contender),
                &owner,
                "/a",
                &[scope("src/lib.rs", false)],
                WorkState::Queued,
                Some(2.0),
            )
            .is_empty()
        );
        assert_eq!(
            blockers(
                std::slice::from_ref(&contender),
                &owner,
                "/b",
                &[scope("src/lib.rs", false)],
                WorkState::Queued,
                Some(2.0),
            )
            .len(),
            1
        );
        assert!(
            blockers(&[contender], &owner, "/b", &[scope("src/lib.rs", false)], WorkState::Queued, Some(0.5),)
                .is_empty()
        );
    }

    #[test]
    fn bundle_paths_are_absolute_and_deterministic() {
        let claims =
            vec![claim("/a", vec![scope("z.rs", false), scope("a.rs", false)]), claim("/b", vec![scope("src", true)])];
        assert_eq!(request_paths(&claims, true), ["/a/z.rs", "/a/a.rs", "/b/src"]);
        assert_eq!(super::super::qualify_path("/a", "."), "/a");
    }
}
