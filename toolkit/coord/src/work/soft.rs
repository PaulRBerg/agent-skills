//! Soft-claim yield: idle holders give up untouched scopes to new requesters.
//!
//! A holder's scope is hard when the holder has evidence under it: an observed
//! touched path or a Git-dirty path inside that scope. Exact scope: evidence path
//! equal. Recursive scope: any evidence path beneath it. A scope is hard or soft as
//! a unit; recursive scopes are never carved. An idle holder (see
//! [`IDLE_YIELD_SECONDS`]) yields every overlapping scope that is entirely soft to a
//! fresh requester instead of blocking it.

use crate::{
    domain::{Identity, Scope, ScopeKind, SessionState, WorkState, sanitize},
    error::Result,
    host::{WorkClaimRequest, relevant_dirty, scope_covers, scopes_overlap},
    state::{SessionRow, WorkClaimUpdate, WorkRow, WorkTransaction, WorkUpdate},
};

use super::{
    RepoEvidence, blockers,
    bundle::ClaimEvaluation,
    evidence_for,
    messages::{MAX_MESSAGE_CHARS, identity_display},
    qualify_path, work_in_repo,
};

/// A holder session idle at least this long yields its untouched overlapping scopes.
pub(crate) const IDLE_YIELD_SECONDS: f64 = 300.0;

/// One repository's worth of scopes yielded by a narrowed contender.
pub(crate) type RepoYield = (String, Vec<Scope>);

/// True when a holder session has been idle long enough to yield untouched scopes.
pub(crate) fn is_idle_holder(session: Option<&SessionRow>, current: f64) -> bool {
    session
        .is_some_and(|session| session.state == SessionState::Idle && current - session.last_seen >= IDLE_YIELD_SECONDS)
}

/// Whole minutes a holder session has been idle, for the yield notification.
pub(crate) fn idle_minutes(session: Option<&SessionRow>, current: f64) -> i64 {
    session.map_or(0, |session| ((current - session.last_seen) / 60.0).floor() as i64)
}

/// The whole scopes of `owned` that overlap any scope in `requested`.
pub(crate) fn overlapping_scopes(requested: &[Scope], owned: &[Scope]) -> Vec<Scope> {
    owned.iter().filter(|scope| requested.iter().any(|request| scopes_overlap(request, scope))).cloned().collect()
}

/// Evidence paths that can make a holder's scopes hard in one repo: paths the
/// holder touched since it submitted this work, plus the repository's Git-dirty paths.
pub(crate) fn evidence_paths(
    transaction: &WorkTransaction<'_>,
    holder: &WorkRow,
    repo_root: &str,
    dirty: &[String],
) -> Result<Vec<String>> {
    let since = holder.submitted_at.unwrap_or(0.0);
    let mut paths = transaction.touched_in_repo(&holder.identity, repo_root, since)?;
    paths.extend(dirty.iter().cloned());
    Ok(paths)
}

/// Splits `scopes` down to those without evidence beneath them (soft, safe to yield).
pub(crate) fn soft_subset(scopes: &[Scope], evidence_paths: &[String]) -> Vec<Scope> {
    scopes.iter().filter(|scope| !has_evidence(scope, evidence_paths)).cloned().collect()
}

fn has_evidence(scope: &Scope, evidence_paths: &[String]) -> bool {
    evidence_paths.iter().any(|path| scope_covers(scope, &Scope { path: path.clone(), kind: ScopeKind::Exact }))
}

/// The overlapping scopes of `owned` that are soft (untouched), for a blocked-message suffix.
pub(crate) fn untouched_overlap(
    transaction: &WorkTransaction<'_>,
    holder: &WorkRow,
    repo_root: &str,
    requested: &[Scope],
    owned: &[Scope],
    dirty: &[String],
) -> Result<Vec<String>> {
    let overlap = overlapping_scopes(requested, owned);
    let evidence = evidence_paths(transaction, holder, repo_root, dirty)?;
    let mut soft = soft_subset(&overlap, &evidence).into_iter().map(|scope| scope.path).collect::<Vec<_>>();
    soft.sort();
    Ok(soft)
}

/// Narrows a soft-yielding contender's claim by removing, per repository in
/// `yields`, the scopes yielded there — releasing that one claim entirely (or the
/// whole work item, if nothing remains anywhere) — in one combined write, then
/// notifies the contender of everything it lost in a single message. Aggregating
/// every repository into one `save_work` keeps a contender that blocks the same
/// bundle request in more than one repository from racing its own stale revision.
/// Yielded scopes are soft by construction, so no dirt beneath them needs residual
/// attribution.
#[allow(clippy::too_many_arguments)]
fn yield_untouched_scopes(
    transaction: &WorkTransaction<'_>,
    requester: &Identity,
    requester_display: &str,
    contender: &WorkRow,
    yields: &[RepoYield],
    qualified: bool,
    idle_minutes: i64,
    current: f64,
) -> Result<()> {
    let mut yielded_paths = Vec::new();
    let mut claims = Vec::with_capacity(contender.claims.len());
    for claim in &contender.claims {
        let Some((repo_root, yielded)) = yields.iter().find(|(root, _)| *root == claim.repo_root) else {
            claims.push(WorkClaimUpdate {
                repo_root: claim.repo_root.clone(),
                blocked_reason: claim.blocked_reason.clone(),
                scopes: claim.scopes.clone(),
                baselines: None,
            });
            continue;
        };
        for scope in yielded {
            yielded_paths.push(if qualified { qualify_path(repo_root, &scope.path) } else { scope.path.clone() });
        }
        let remaining = claim.scopes.iter().filter(|scope| !yielded.contains(scope)).cloned().collect::<Vec<_>>();
        if remaining.is_empty() {
            continue;
        }
        // Baselines stay claim-local: drop those beneath a yielded scope.
        let baselines = transaction
            .baselines_in_repo(&contender.identity, repo_root)?
            .into_iter()
            .filter(|row| !relevant_dirty(&remaining, std::slice::from_ref(&row.path)).is_empty())
            .collect();
        claims.push(WorkClaimUpdate {
            repo_root: claim.repo_root.clone(),
            blocked_reason: claim.blocked_reason.clone(),
            scopes: remaining,
            baselines: Some(baselines),
        });
    }
    if claims.is_empty() {
        transaction.delete_work(&contender.identity)?;
    } else {
        transaction.save_work(&WorkUpdate {
            identity: contender.identity.clone(),
            label: contender.label.clone(),
            state: contender.state,
            blocked_reason: contender.blocked_reason.clone(),
            claims,
            submitted_at: contender.submitted_at,
            updated_at: current,
            expected_revision: Some(contender.revision),
        })?;
    }
    yielded_paths.sort();
    let message = sanitize(
        &format!(
            "Yielded untouched scopes {} to {requester_display} after idle {idle_minutes}m; \
             re-run ai-coord start to re-acquire.",
            yielded_paths.join(", ")
        ),
        MAX_MESSAGE_CHARS,
    );
    let repo_root = (!qualified).then(|| yields[0].0.as_str());
    transaction.send_message(requester, &contender.identity, &message, repo_root, current)?;
    Ok(())
}

/// Yields planned during evaluation and applied only in the transaction that
/// grants the requester.
#[derive(Default)]
pub(crate) struct YieldPlan {
    /// Original "overlap" evaluations of the claims the plan unblocked, by claim index.
    softened: Vec<(usize, ClaimEvaluation)>,
    /// One entry per distinct contender identity, so a contender blocking the
    /// request in more than one repository (a bundle) is narrowed with exactly one
    /// `save_work` covering every repository, instead of one write per repository
    /// racing against its own stale revision.
    contenders: Vec<(WorkRow, Vec<RepoYield>)>,
}

impl YieldPlan {
    /// Reinstates the blocking evaluations when the requester stays queued, so its
    /// outcome, per-claim reasons, and holder notices reflect unyielded claims.
    pub(crate) fn restore(self, evaluations: &mut [ClaimEvaluation]) {
        for (index, evaluation) in self.softened {
            evaluations[index] = evaluation;
        }
    }

    /// Narrows every planned contender and notifies it of what it yielded.
    pub(crate) fn apply(
        self,
        transaction: &WorkTransaction<'_>,
        identity: &Identity,
        qualified: bool,
        current: f64,
    ) -> Result<()> {
        if self.contenders.is_empty() {
            return Ok(());
        }
        let requester_display = identity_display(identity, transaction)?;
        for (contender, yields) in self.contenders {
            let session = transaction.session(&contender.identity)?;
            let minutes = idle_minutes(session.as_ref(), current);
            yield_untouched_scopes(
                transaction,
                identity,
                &requester_display,
                &contender,
                &yields,
                qualified,
                minutes,
                current,
            )?;
        }
        Ok(())
    }
}

/// Nullifies each "overlap" evaluation whose contenders are all idle and whose
/// scopes overlapping the request are all soft (untouched), planning to narrow
/// those contenders' claims. A claim that an earlier-queued waiter also overlaps
/// is re-blocked as "waiter" instead, so a yield never lets a newcomer jump the
/// FIFO queue; that waiter's own recheck triggers the yield. Only the
/// non-expansion path softens; `update_active` expansion never does.
#[allow(clippy::too_many_arguments)]
pub(crate) fn soften_evaluations(
    transaction: &WorkTransaction<'_>,
    identity: &Identity,
    claims: &[WorkClaimRequest],
    evaluations: &mut [ClaimEvaluation],
    evidence: &[RepoEvidence],
    work: &[WorkRow],
    submitted_at: f64,
    current: f64,
) -> Result<YieldPlan> {
    let mut plan = YieldPlan::default();
    for (index, (claim, evaluation)) in claims.iter().zip(evaluations.iter_mut()).enumerate() {
        if evaluation.reason.as_deref() != Some("overlap") || evaluation.contenders.is_empty() {
            continue;
        }
        let repo_root = claim.repo_root.to_str().expect("validated root");
        let dirty = &evidence_for(evidence, repo_root).dirty;
        let mut plans = Vec::with_capacity(evaluation.contenders.len());
        for contender in &evaluation.contenders {
            let contender_claim = contender.claim(repo_root).expect("repository contender");
            let overlap = overlapping_scopes(&claim.scopes, &contender_claim.scopes);
            let paths = evidence_paths(transaction, contender, repo_root, dirty)?;
            let soft_overlap = soft_subset(&overlap, &paths);
            let session = transaction.session(&contender.identity)?;
            let qualifies = is_idle_holder(session.as_ref(), current) && soft_overlap.len() == overlap.len();
            plans.push((contender.clone(), soft_overlap, qualifies));
        }
        if plans.iter().all(|(_, _, qualifies)| *qualifies) {
            let repo_work = work_in_repo(work, repo_root);
            let earlier =
                blockers(&repo_work, identity, repo_root, &claim.scopes, WorkState::Queued, Some(submitted_at));
            if !earlier.is_empty() {
                evaluation.reason = Some("waiter".to_owned());
                evaluation.contenders = earlier;
                continue;
            }
            plan.softened.push((index, evaluation.clone()));
            evaluation.reason = None;
            for (contender, soft_overlap, _) in plans {
                match plan.contenders.iter_mut().find(|(existing, _)| existing.identity == contender.identity) {
                    Some((_, entries)) => entries.push((repo_root.to_owned(), soft_overlap)),
                    None => plan.contenders.push((contender, vec![(repo_root.to_owned(), soft_overlap)])),
                }
            }
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Client;

    fn scope(path: &str, recursive: bool) -> Scope {
        Scope { path: path.to_owned(), kind: if recursive { ScopeKind::Recursive } else { ScopeKind::Exact } }
    }

    #[test]
    fn recursive_scope_is_hard_as_a_whole_unit_when_any_path_beneath_it_has_evidence() {
        let scopes = vec![scope("src", true), scope("docs", true)];
        let evidence = vec!["src/lib.rs".to_owned()];
        assert_eq!(soft_subset(&scopes, &evidence), vec![scope("docs", true)]);
    }

    #[test]
    fn exact_scope_requires_an_exact_evidence_path() {
        let scopes = vec![scope("src/lib.rs", false)];
        assert_eq!(soft_subset(&scopes, &["src".to_owned()]), scopes);
        assert!(soft_subset(&scopes, &["src/lib.rs".to_owned()]).is_empty());
    }

    #[test]
    fn idle_requires_both_idle_state_and_the_full_yield_window() {
        let session = SessionRow {
            identity: Identity { client: Client::Codex, session_id: "holder".to_owned() },
            cwd: String::new(),
            repo_root: None,
            state: SessionState::Idle,
            callsign: None,
            name: None,
            waiting_for: None,
            permission_mode: None,
            coordination_waived: false,
            fingerprint: None,
            transcript_path: None,
            source: String::new(),
            started_at: 0.0,
            last_seen: 0.0,
            revision: 1,
        };
        assert!(!is_idle_holder(Some(&session), IDLE_YIELD_SECONDS - 1.0));
        assert!(is_idle_holder(Some(&session), IDLE_YIELD_SECONDS));
        assert!(!is_idle_holder(None, IDLE_YIELD_SECONDS));
    }

    #[test]
    fn planned_yield_persists_nothing_until_applied() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut store = crate::state::Store::open(temp.path().join("state.db")).unwrap();
        let holder = Identity { client: Client::Codex, session_id: "holder".to_owned() };
        let requester = Identity { client: Client::Codex, session_id: "requester".to_owned() };
        for identity in [&holder, &requester] {
            store
                .upsert_session(&crate::state::SessionUpdate {
                    identity: identity.clone(),
                    cwd: "/repo".to_owned(),
                    repo_root: Some("/repo".to_owned()),
                    state: SessionState::Idle,
                    source: "test".to_owned(),
                    name: None,
                    waiting_for: None,
                    permission_mode: None,
                    update_permission_mode: false,
                    coordination_waived: None,
                    fingerprint: None,
                    transcript_path: None,
                    started_at: Some(0.0),
                    current: 0.0,
                })
                .unwrap();
        }
        let owned = vec![scope("src", true)];
        store
            .with_work_transaction(|transaction| {
                transaction.save_work(&WorkUpdate {
                    identity: holder.clone(),
                    label: "holder".to_owned(),
                    state: WorkState::Active,
                    blocked_reason: None,
                    claims: vec![WorkClaimUpdate {
                        repo_root: "/repo".to_owned(),
                        blocked_reason: None,
                        scopes: owned.clone(),
                        baselines: None,
                    }],
                    submitted_at: Some(1.0),
                    updated_at: 1.0,
                    expected_revision: None,
                })?;
                Ok(())
            })
            .unwrap();
        let claims = vec![WorkClaimRequest { repo_root: "/repo".into(), scopes: vec![scope("src/lib.rs", false)] }];
        let evidence = vec![RepoEvidence {
            repo_root: "/repo".to_owned(),
            dirty: Vec::new(),
            hashes: Vec::new(),
            benign: Vec::new(),
            inspection: None,
        }];
        // Evaluate like an arbitration pass that ends in `Prepare`: plan, then commit without applying.
        store
            .with_work_transaction(|transaction| {
                let work = transaction.works()?;
                let mut evaluations = vec![ClaimEvaluation::default()];
                evaluations[0].reason = Some("overlap".to_owned());
                evaluations[0].contenders = work.clone();
                let plan = soften_evaluations(
                    transaction,
                    &requester,
                    &claims,
                    &mut evaluations,
                    &evidence,
                    &work,
                    2.0,
                    IDLE_YIELD_SECONDS,
                )?;
                assert!(evaluations[0].reason.is_none());
                assert_eq!(plan.contenders.len(), 1);
                Ok(())
            })
            .unwrap();
        assert_eq!(store.work(&holder).unwrap().unwrap().claims[0].scopes, owned);
        assert!(store.inbox(&holder, true).unwrap().is_empty());
    }
}
