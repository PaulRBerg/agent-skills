use std::path::{Path, PathBuf};

use crate::{
    domain::{Identity, ProcessLiveness},
    error::{AppError, Result},
    host::{git_root, normalize_work_scopes},
    state::{
        MAX_RECOMMENDATION_SCOPES, MAX_RECOMMENDATION_TEXT, RecommendationAction, RecommendationDecision,
        RecommendationMutation, RecommendationObservation, RecommendationRow, RecommendationSend, RecommendationState,
        Store,
    },
};

use super::{Coordinator, path_text, resolve_targets, resolved};

impl Coordinator {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_recommendation(
        &self,
        target: &str,
        action: RecommendationAction,
        files: &[PathBuf],
        recursive: &[PathBuf],
        reason: &str,
        replacement: &str,
        cwd: &Path,
    ) -> Result<RecommendationMutation> {
        let sender = self.recommendation_identity()?;
        self.send_recommendation_for(&sender, target, action, files, recursive, reason, replacement, cwd)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_recommendation_for(
        &self,
        sender: &Identity,
        target: &str,
        action: RecommendationAction,
        files: &[PathBuf],
        recursive: &[PathBuf],
        reason: &str,
        replacement: &str,
        cwd: &Path,
    ) -> Result<RecommendationMutation> {
        if target == "repo" {
            return Err(AppError::usage("recommendations require one peer target; 'repo' is not allowed"));
        }
        let cwd = resolved(cwd);
        let root = git_root(&cwd).ok_or_else(|| AppError::operational("recommend send requires a Git worktree"))?;
        let repo_root = path_text(&root)?;
        let scopes = normalize_work_scopes(files, recursive, &cwd, &root)?;
        if scopes.is_empty() || scopes.len() > MAX_RECOMMENDATION_SCOPES {
            return Err(AppError::usage("a recommendation requires 1 to 50 scopes"));
        }
        let reason = normalize_recommendation_text(reason)?;
        let replacement = normalize_recommendation_text(replacement)?;

        let mut store = self.store()?;
        let _ = self.refresh_inventory(&mut store, false)?;
        let sessions = store.sessions()?;
        let works = store.works()?;
        let recipient = resolve_targets(target, &sessions, &works, Some(&root), sender)?
            .into_iter()
            .next()
            .expect("target resolver returns exactly one identity");
        if recipient == *sender {
            return Err(AppError::usage("a recommendation requires a different recipient"));
        }
        let sender_observation = self.recommendation_observation(&store, sender, "sender")?;
        let recipient_observation = self.recommendation_observation(&store, &recipient, "recipient")?;
        require_claim(&sender_observation, &repo_root, "sender")?;
        require_claim(&recipient_observation, &repo_root, "recipient")?;
        let input = RecommendationSend {
            sender: sender_observation,
            recipient: recipient_observation,
            repo_root,
            scopes,
            action,
            reason,
            replacement,
            current: self.clock.wall(),
        };
        store.with_work_transaction(|transaction| transaction.send_recommendation(&input))
    }

    pub(crate) fn list_recommendations(&self, sent: bool, all: bool, cwd: &Path) -> Result<Vec<RecommendationRow>> {
        let identity = self.required_identity()?;
        self.list_recommendations_for(&identity, sent, all, cwd)
    }

    pub(crate) fn list_recommendations_for(
        &self,
        identity: &Identity,
        sent: bool,
        all: bool,
        cwd: &Path,
    ) -> Result<Vec<RecommendationRow>> {
        let root =
            git_root(&resolved(cwd)).ok_or_else(|| AppError::operational("recommend list requires a Git worktree"))?;
        let repo_root = path_text(&root)?;
        let mut store = self.store()?;
        self.refresh_recommendation_context(&mut store)?;
        store.recommendations(identity, &repo_root, sent, all)
    }

    pub(crate) fn show_recommendation(&self, id: &str) -> Result<RecommendationRow> {
        let identity = self.required_identity()?;
        self.show_recommendation_for(&identity, id)
    }

    pub(crate) fn show_recommendation_for(&self, identity: &Identity, id: &str) -> Result<RecommendationRow> {
        let mut store = self.store()?;
        self.refresh_recommendation_context(&mut store)?;
        store
            .recommendation(identity, id)?
            .ok_or_else(|| AppError::operational("recommendation not found for this endpoint"))
    }

    pub(crate) fn respond_recommendation(
        &self,
        id: &str,
        decision: RecommendationDecision,
        reason: &str,
    ) -> Result<RecommendationMutation> {
        let recipient = self.recommendation_identity()?;
        self.respond_recommendation_for(&recipient, id, decision, reason)
    }

    pub(crate) fn respond_recommendation_for(
        &self,
        recipient: &Identity,
        id: &str,
        decision: RecommendationDecision,
        reason: &str,
    ) -> Result<RecommendationMutation> {
        let reason = normalize_recommendation_text(reason)?;
        let mut store = self.store()?;
        self.refresh_recommendation_context(&mut store)?;
        let record = store
            .recommendation(recipient, id)?
            .ok_or_else(|| AppError::operational("recommendation not found for this endpoint"))?;
        let observations =
            if decision == RecommendationDecision::Accepted && record.state == RecommendationState::Pending {
                vec![
                    self.recommendation_observation(&store, &record.sender.identity, "sender")?,
                    self.recommendation_observation(&store, &record.recipient.identity, "recipient")?,
                ]
            } else {
                Vec::new()
            };
        store.with_work_transaction(|transaction| {
            transaction.respond_recommendation(recipient, id, decision, &reason, &observations, self.clock.wall())
        })
    }

    pub(crate) fn withdraw_recommendation(&self, id: &str, reason: &str) -> Result<RecommendationMutation> {
        let sender = self.recommendation_identity()?;
        self.withdraw_recommendation_for(&sender, id, reason)
    }

    pub(crate) fn withdraw_recommendation_for(
        &self,
        sender: &Identity,
        id: &str,
        reason: &str,
    ) -> Result<RecommendationMutation> {
        let reason = normalize_recommendation_text(reason)?;
        let mut store = self.store()?;
        self.refresh_recommendation_context(&mut store)?;
        store.with_work_transaction(|transaction| {
            transaction.withdraw_recommendation(sender, id, &reason, self.clock.wall())
        })
    }

    /// Hook and wait paths use this typed view rather than ordinary pointer messages.
    pub(crate) fn pending_recommendations(
        &self,
        identity: &Identity,
        root: Option<&Path>,
        unsurfaced_only: bool,
    ) -> Result<Vec<RecommendationRow>> {
        let repo_root = root.map(path_text).transpose()?;
        let mut store = self.store()?;
        self.refresh_recommendation_context(&mut store)?;
        store.pending_recommendations(identity, repo_root.as_deref(), unsurfaced_only)
    }

    /// Marks only records represented by a successfully rendered hook checkpoint.
    pub(crate) fn mark_recommendations_surfaced(&self, identity: &Identity, ids: &[String]) -> Result<usize> {
        self.store()?.mark_recommendations_surfaced(identity, ids, self.clock.wall())
    }

    fn recommendation_identity(&self) -> Result<Identity> {
        let identity = self.required_identity()?;
        self.reject_delegate(&identity)?;
        Ok(identity)
    }

    fn refresh_recommendation_context(&self, store: &mut Store) -> Result<()> {
        let _ = self.reconcile_processes(store)?;
        store.refresh_recommendations(self.clock.wall())?;
        Ok(())
    }

    fn recommendation_observation(
        &self,
        store: &Store,
        identity: &Identity,
        endpoint: &str,
    ) -> Result<RecommendationObservation> {
        let session = store
            .session(identity)?
            .ok_or_else(|| AppError::operational(format!("{endpoint} session is not live; refresh and retry")))?;
        let work = store.work(identity)?.ok_or_else(|| {
            AppError::usage(format!("{endpoint} must have submitted queued or active work before recommending"))
        })?;
        let liveness = session
            .fingerprint
            .as_ref()
            .map_or(ProcessLiveness::Unknown, |fingerprint| self.probe.liveness(fingerprint));
        if liveness != ProcessLiveness::Alive {
            return Err(AppError::operational(format!(
                "{endpoint} process liveness is not established; refresh and retry"
            )));
        }
        Ok(RecommendationObservation::new(&session, &work, liveness))
    }
}

fn require_claim(observation: &RecommendationObservation, repo_root: &str, endpoint: &str) -> Result<()> {
    if observation.work.claims.iter().any(|claim| claim.repo_root == repo_root) {
        Ok(())
    } else {
        Err(AppError::usage(format!("{endpoint} must have submitted work in the recommendation repository")))
    }
}

fn normalize_recommendation_text(text: &str) -> Result<String> {
    let normalized = text
        .chars()
        .map(|character| if character.is_control() { ' ' } else { character })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() || normalized.chars().count() > MAX_RECOMMENDATION_TEXT {
        return Err(AppError::usage("recommendation text must contain 1 to 2000 Unicode characters"));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests;
