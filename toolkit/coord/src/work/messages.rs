//! Contender-facing message composition for queued, blocked, and soft-yielded work.

use std::collections::HashSet;

use super::{
    RepoEvidence,
    bundle::{ClaimEvaluation, MAX_BUNDLE_CONFLICT_PATHS},
    evidence_for, output_path, soft, sorted,
};
use crate::{
    domain::{Identity, Scope, client_name, sanitize},
    error::Result,
    host::{WorkClaimRequest, overlapping_paths},
    state::{WorkClaimRow, WorkRow, WorkTransaction},
};

pub(crate) const MAX_MESSAGE_CHARS: usize = 240;

pub(crate) fn identity_display(identity: &Identity, transaction: &WorkTransaction<'_>) -> Result<String> {
    if let Some(callsign) = transaction.callsign(identity)? {
        return Ok(callsign);
    }
    let prefix = identity.session_id.chars().take(8).collect::<String>();
    Ok(format!("{}/{prefix}", client_name(identity.client)))
}

pub(crate) fn conflict_detail(
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

/// `untouched` lists this blocker's own overlapping scopes that carry no evidence
/// (soft), even though the request stayed blocked (the blocker was not idle enough,
/// or another overlapping scope was hard).
pub(crate) fn blocked_message(
    label: &str,
    requested: &[Scope],
    blocker: &WorkClaimRow,
    untouched: &[String],
) -> String {
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
    let mut message = if broad.is_empty() {
        format!("Queued behind your work: {label}; overlaps: {}.", overlaps.join(", "))
    } else {
        format!(
            "Narrow broad work {} with ai-coord start if unrelated; queued work '{label}' overlaps: {}.",
            broad.join(", "),
            overlaps.join(", ")
        )
    };
    if !untouched.is_empty() {
        message.push_str(&format!(" untouched: {}.", untouched.join(", ")));
    }
    sanitize(&message, MAX_MESSAGE_CHARS)
}

pub(crate) fn bundle_blocked_message(
    label: &str,
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    blocker: &WorkRow,
    transaction: &WorkTransaction<'_>,
    evidence: &[RepoEvidence],
) -> Result<String> {
    let mut overlaps = Vec::new();
    let mut untouched = Vec::new();
    for (claim, evaluation) in claims.iter().zip(evaluations) {
        if !evaluation.contenders.iter().any(|candidate| candidate.identity == blocker.identity) {
            continue;
        }
        let repo_root = claim.repo_root.to_str().expect("validated root");
        if let Some(owned) = blocker.claim(repo_root) {
            overlaps.extend(
                overlapping_paths(&claim.scopes, &owned.scopes).iter().map(|path| output_path(claim, path, true)),
            );
            let dirty = &evidence_for(evidence, repo_root).dirty;
            let soft_paths =
                soft::untouched_overlap(transaction, blocker, repo_root, &claim.scopes, &owned.scopes, dirty)?;
            untouched.extend(soft_paths.iter().map(|path| output_path(claim, path, true)));
        }
    }
    overlaps.sort();
    overlaps.dedup();
    overlaps.truncate(MAX_BUNDLE_CONFLICT_PATHS);
    untouched.sort();
    untouched.dedup();
    untouched.truncate(MAX_BUNDLE_CONFLICT_PATHS);
    let mut message = format!("Queued behind your work: {label}; overlaps: {}.", overlaps.join(", "));
    if !untouched.is_empty() {
        message.push_str(&format!(" untouched: {}.", untouched.join(", ")));
    }
    Ok(sanitize(&message, MAX_MESSAGE_CHARS))
}

pub(crate) fn notify_contenders(
    transaction: &WorkTransaction<'_>,
    identity: &Identity,
    label: &str,
    claims: &[WorkClaimRequest],
    evaluations: &[ClaimEvaluation],
    evidence: &[RepoEvidence],
    current: f64,
) -> Result<()> {
    let qualified = claims.len() > 1;
    let mut notified = Vec::<Identity>::new();
    for (claim, evaluation) in claims.iter().zip(evaluations) {
        if evaluation.reason.as_deref() != Some("overlap") {
            continue;
        }
        let repo_root = claim.repo_root.to_str().expect("validated root");
        let dirty = &evidence_for(evidence, repo_root).dirty;
        for contender in &evaluation.contenders {
            if notified.contains(&contender.identity) {
                continue;
            }
            let contender_claim = contender.claim(repo_root).expect("repository contender");
            let message = if qualified {
                bundle_blocked_message(label, claims, evaluations, contender, transaction, evidence)?
            } else {
                let untouched = soft::untouched_overlap(
                    transaction,
                    contender,
                    repo_root,
                    &claim.scopes,
                    &contender_claim.scopes,
                    dirty,
                )?;
                blocked_message(label, &claim.scopes, contender_claim, &untouched)
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
