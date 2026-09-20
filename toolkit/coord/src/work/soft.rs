//! Soft-claim yield: idle holders give up untouched scopes to new requesters.
//!
//! A holder's scope is hard when the holder has evidence under it: an observed
//! touched path or a Git-dirty path inside that scope. Exact scope: evidence path
//! equal. Recursive scope: any evidence path beneath it. A scope is hard or soft as
//! a unit; recursive scopes are never carved. An idle holder (see
//! [`IDLE_YIELD_SECONDS`]) yields every overlapping scope that is entirely soft to a
//! fresh requester instead of blocking it.

use crate::{
    domain::{Identity, Scope, ScopeKind, SessionState, sanitize},
    error::Result,
    host::{scope_covers, scopes_overlap},
    state::{SessionRow, WorkClaimUpdate, WorkRow, WorkTransaction, WorkUpdate},
};

use super::{messages::MAX_MESSAGE_CHARS, qualify_path};

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
pub(crate) fn yield_untouched_scopes(
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
    let claims = contender
        .claims
        .iter()
        .filter_map(|claim| {
            let Some((repo_root, yielded)) = yields.iter().find(|(root, _)| *root == claim.repo_root) else {
                return Some(WorkClaimUpdate {
                    repo_root: claim.repo_root.clone(),
                    blocked_reason: claim.blocked_reason.clone(),
                    scopes: claim.scopes.clone(),
                    baselines: None,
                    residual_paths: Vec::new(),
                });
            };
            for scope in yielded {
                yielded_paths.push(if qualified { qualify_path(repo_root, &scope.path) } else { scope.path.clone() });
            }
            let remaining = claim.scopes.iter().filter(|scope| !yielded.contains(scope)).cloned().collect::<Vec<_>>();
            if remaining.is_empty() {
                None
            } else {
                Some(WorkClaimUpdate {
                    repo_root: claim.repo_root.clone(),
                    blocked_reason: claim.blocked_reason.clone(),
                    scopes: remaining,
                    baselines: None,
                    residual_paths: Vec::new(),
                })
            }
        })
        .collect::<Vec<_>>();
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
}
