use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    domain::{Client, Identity, Outcome, OutcomeKind, ProcessLiveness, Scope, ScopeKind, WorkState, sanitize},
    error::{AppError, ErrorKind, Result},
    host::{
        any_overlap, git_blob_hashes, git_dirty_paths, git_root, normalize_work_claim_bundle, normalize_work_scopes,
        process_sweep, relevant_dirty,
    },
    state::{BaselineRow, EndedObservation, Store, TouchedPaths, WorkClaimUpdate, WorkRow},
    work::WorkCoordinator,
};

use super::{Coordinator, FULL_REFRESH_SECONDS, MAX_LABEL_CHARS, path_text, resolved};

struct ReleaseClaimPlan {
    repo_root: String,
    hashes: Vec<(String, String)>,
    residual_paths: Vec<String>,
    inspected: bool,
}

struct ReleasePlan {
    work: WorkRow,
    claims: Vec<ReleaseClaimPlan>,
}

impl Coordinator {
    pub(crate) fn start(&self, label: &str, files: &[PathBuf], recursive: &[PathBuf], cwd: &Path) -> Result<Outcome> {
        let identity = self.required_identity()?;
        self.start_for(identity, label, files, recursive, cwd)
    }

    pub(crate) fn start_for(
        &self,
        identity: Identity,
        label: &str,
        files: &[PathBuf],
        recursive: &[PathBuf],
        cwd: &Path,
    ) -> Result<Outcome> {
        let cwd = resolved(cwd);
        let root = git_root(&cwd).ok_or_else(|| AppError::operational("start requires a Git worktree"))?;
        let label = normalized_label(label)?;
        let scopes = normalize_work_scopes(files, recursive, &cwd, &root)?;
        if scopes.is_empty() {
            return Err(AppError::usage("at least one scope is required"));
        }
        let mut store = self.store()?;
        require_ordinary_item(store.work(&identity)?.as_ref(), &root, "start")?;
        self.ensure_session(&mut store, &identity, &cwd, Some(&root))?;
        store.set_coordination_waived(&identity, false)?;
        let inventory = self.refresh_inventory(&mut store, false)?;
        WorkCoordinator { store: &mut store }.start_direct(
            &identity,
            &root,
            &label,
            scopes,
            &inventory,
            self.clock.wall(),
        )
    }

    pub(crate) fn start_bundle(&self, label: &str, files: &[PathBuf], recursive: &[PathBuf]) -> Result<Outcome> {
        let identity = self.required_identity()?;
        self.start_bundle_for(identity, label, files, recursive, &std::env::current_dir()?)
    }

    pub(crate) fn start_bundle_for(
        &self,
        identity: Identity,
        label: &str,
        files: &[PathBuf],
        recursive: &[PathBuf],
        cwd: &Path,
    ) -> Result<Outcome> {
        let cwd = resolved(cwd);
        let label = normalized_label(label)?;
        let claims = normalize_work_claim_bundle(files, recursive)?;
        let mut store = self.store()?;
        let session_root = git_root(&cwd);
        self.ensure_session(&mut store, &identity, &cwd, session_root.as_deref())?;
        store.set_coordination_waived(&identity, false)?;
        let inventory = self.refresh_inventory(&mut store, false)?;
        WorkCoordinator { store: &mut store }.start_claims(&identity, &label, claims, &inventory, self.clock.wall())
    }

    pub(crate) fn draft(&self, label: &str, files: &[PathBuf], recursive: &[PathBuf], cwd: &Path) -> Result<Outcome> {
        let identity = self.required_identity()?;
        self.draft_for(identity, label, files, recursive, cwd)
    }

    pub(crate) fn draft_for(
        &self,
        identity: Identity,
        label: &str,
        files: &[PathBuf],
        recursive: &[PathBuf],
        cwd: &Path,
    ) -> Result<Outcome> {
        let cwd = resolved(cwd);
        let root = git_root(&cwd).ok_or_else(|| AppError::operational("draft requires a Git worktree"))?;
        let label = normalized_label(label)?;
        let scopes = normalize_work_scopes(files, recursive, &cwd, &root)?;
        if scopes.is_empty() {
            return Err(AppError::usage("at least one scope is required"));
        }
        let mut store = self.store()?;
        require_ordinary_item(store.work(&identity)?.as_ref(), &root, "draft")?;
        self.ensure_session(&mut store, &identity, &cwd, Some(&root))?;
        store.set_coordination_waived(&identity, false)?;
        store.save_draft(&identity, &label, &[claim_update(path_text(&root)?, scopes.clone())], self.clock.wall())?;
        Ok(Outcome::new(OutcomeKind::Draft, 0, scopes.len().to_string()))
    }

    pub(crate) fn draft_bundle(&self, label: &str, files: &[PathBuf], recursive: &[PathBuf]) -> Result<Outcome> {
        let identity = self.required_identity()?;
        self.draft_bundle_for(identity, label, files, recursive, &std::env::current_dir()?)
    }

    pub(crate) fn draft_bundle_for(
        &self,
        identity: Identity,
        label: &str,
        files: &[PathBuf],
        recursive: &[PathBuf],
        cwd: &Path,
    ) -> Result<Outcome> {
        let cwd = resolved(cwd);
        let label = normalized_label(label)?;
        let claims = normalize_work_claim_bundle(files, recursive)?;
        let updates = claims
            .iter()
            .map(|claim| Ok(claim_update(path_text(&claim.repo_root)?, claim.scopes.clone())))
            .collect::<Result<Vec<_>>>()?;
        let scope_count = updates.iter().map(|claim| claim.scopes.len()).sum::<usize>();
        let mut store = self.store()?;
        let session_root = git_root(&cwd);
        self.ensure_session(&mut store, &identity, &cwd, session_root.as_deref())?;
        store.set_coordination_waived(&identity, false)?;
        store.save_draft(&identity, &label, &updates, self.clock.wall())?;
        Ok(Outcome::new(OutcomeKind::Draft, 0, scope_count.to_string()))
    }

    pub(crate) fn promote_draft(&self, cwd: &Path) -> Result<Outcome> {
        let identity = self.required_identity()?;
        self.promote_draft_for(&identity, cwd)
    }

    pub(crate) fn promote_draft_for(&self, identity: &Identity, cwd: &Path) -> Result<Outcome> {
        let cwd = resolved(cwd);
        let root = git_root(&cwd).ok_or_else(|| AppError::operational("start --draft requires a Git worktree"))?;
        let mut store = self.store()?;
        let draft = store
            .work(identity)?
            .filter(|work| work.state == WorkState::Draft)
            .ok_or_else(|| AppError::operational("no draft work for this session"))?;
        if draft.claims.len() != 1 {
            return Err(AppError::operational(
                "draft is a repository bundle; submit it with ai-coord bundle start --draft",
            ));
        }
        let repo_root = path_text(&root)?;
        if draft.claim(&repo_root).is_none() {
            return Err(AppError::operational(format!(
                "draft belongs to {}; run ai-coord start --draft there or clear it with ai-coord done",
                draft.claims[0].repo_root
            )));
        }
        revalidate_draft(&draft)?;
        self.ensure_session(&mut store, identity, &cwd, Some(&root))?;
        store.set_coordination_waived(identity, false)?;
        let inventory = self.refresh_inventory(&mut store, false)?;
        WorkCoordinator { store: &mut store }.promote_draft(identity, draft, &inventory, self.clock.wall())
    }

    pub(crate) fn promote_bundle_draft(&self, cwd: &Path) -> Result<Outcome> {
        let identity = self.required_identity()?;
        self.promote_bundle_draft_for(&identity, cwd)
    }

    pub(crate) fn promote_bundle_draft_for(&self, identity: &Identity, cwd: &Path) -> Result<Outcome> {
        let cwd = resolved(cwd);
        let mut store = self.store()?;
        let draft = store
            .work(identity)?
            .filter(|work| work.state == WorkState::Draft)
            .ok_or_else(|| AppError::operational("no draft work for this session"))?;
        if draft.claims.len() < 2 {
            return Err(AppError::operational("draft has one repository claim; submit it with ai-coord start --draft"));
        }
        revalidate_draft(&draft)?;
        let session_root = git_root(&cwd);
        self.ensure_session(&mut store, identity, &cwd, session_root.as_deref())?;
        store.set_coordination_waived(identity, false)?;
        let inventory = self.refresh_inventory(&mut store, false)?;
        WorkCoordinator { store: &mut store }.promote_draft(identity, draft, &inventory, self.clock.wall())
    }

    pub(crate) fn wait(&self, timeout_seconds: u64, poll_seconds: f64) -> Result<Outcome> {
        let identity = self.required_identity()?;
        let cwd = std::env::current_dir()?;
        let root = git_root(&resolved(&cwd)).ok_or_else(|| AppError::operational("wait requires a Git worktree"))?;
        self.wait_for_repo(&identity, &root, timeout_seconds, poll_seconds, false)
    }

    pub(crate) fn wait_for_repo(
        &self,
        identity: &Identity,
        root: &Path,
        timeout_seconds: u64,
        poll_seconds: f64,
        released_if_missing: bool,
    ) -> Result<Outcome> {
        if !(1..=3600).contains(&timeout_seconds) {
            return Err(AppError::usage("timeout must be between 1 and 3600 seconds"));
        }
        let repo_root = path_text(root)?;
        let started = self.clock.monotonic();
        let mut last_generation = None;
        let mut last_full_check = None;
        let mut observed_valid_work = false;
        loop {
            let mut store = self.store()?;
            let process_complete = self.reconcile_processes(&mut store)?.is_empty();
            let Some(work) = store.work(identity)? else {
                return if released_if_missing || observed_valid_work {
                    Ok(Outcome::new(OutcomeKind::Released, 3, ""))
                } else {
                    Err(AppError::operational("no active or queued work for this session"))
                };
            };
            if work.claim(&repo_root).is_none() {
                if observed_valid_work {
                    return Ok(Outcome::new(OutcomeKind::Released, 3, ""));
                }
                return Err(AppError::operational(format!(
                    "current repository is not claimed by work '{}'; use ai-coord done from a claimed repository",
                    work.label
                )));
            }
            observed_valid_work = true;
            let qualified = work.claims.len() > 1;
            if work.state == WorkState::Active {
                return Ok(Outcome::new(OutcomeKind::Ready, 0, "").with_paths(work_paths(&work, qualified)));
            }
            if work.state == WorkState::Draft {
                let command = if qualified { "ai-coord bundle start --draft" } else { "ai-coord start --draft" };
                return Err(AppError::operational(format!(
                    "draft work must be submitted with {command} before waiting"
                )));
            }
            let claimed_roots = work.claims.iter().map(|claim| claim.repo_root.as_str()).collect::<HashSet<_>>();
            let pending = store
                .inbox(identity, true)?
                .into_iter()
                .filter(|message| message.repo_root.as_deref().is_none_or(|root| claimed_roots.contains(root)))
                .collect::<Vec<_>>();
            if !pending.is_empty() {
                return Ok(Outcome::new(OutcomeKind::Message, 3, pending.len().to_string()));
            }

            let elapsed = self.clock.monotonic() - started;
            if elapsed >= timeout_seconds as f64 {
                return Ok(Outcome::new(OutcomeKind::Timeout, 3, timeout_seconds.to_string()));
            }

            let generation = store.generation()?;
            let now = self.clock.monotonic();
            let refresh_seconds =
                if work.blocked_reason.as_deref() == Some("dirty") { 1.0 } else { FULL_REFRESH_SECONDS };
            let due =
                last_full_check.is_none_or(|last| now - last >= refresh_seconds) || last_generation != Some(generation);
            if due {
                let mut inventory = self.refresh_inventory(&mut store, false)?;
                inventory.complete &= process_complete;
                let outcome =
                    WorkCoordinator { store: &mut store }.wait_recheck(identity, &work, &inventory, self.clock.wall());
                let outcome = match outcome {
                    Ok(outcome) => outcome,
                    Err(error) if error.kind == ErrorKind::WaitArbitrationRetry => {
                        let elapsed = self.clock.monotonic() - started;
                        if elapsed < timeout_seconds as f64 {
                            self.clock.sleep(Duration::from_secs_f64(
                                poll_seconds.max(0.001).min(timeout_seconds as f64 - elapsed),
                            ));
                        }
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                last_full_check = Some(self.clock.monotonic());
                last_generation = Some(store.generation()?);
                if outcome.code == 0 || (outcome.code == 2 && !outcome.detail.starts_with("dirty-settling:")) {
                    return Ok(outcome);
                }
            }
            let elapsed = self.clock.monotonic() - started;
            if elapsed >= timeout_seconds as f64 {
                return Ok(Outcome::new(OutcomeKind::Timeout, 3, timeout_seconds.to_string()));
            }
            self.clock.sleep(Duration::from_secs_f64(poll_seconds.max(0.001).min(timeout_seconds as f64 - elapsed)));
        }
    }

    pub(crate) fn done(&self) -> Result<Outcome> {
        let identity = self.required_identity()?;
        let cwd = std::env::current_dir()?;
        let outcome = self.done_for(&identity, &cwd)?;
        if let Ok(cwd) = std::env::current_dir() {
            let _ = self.schedule_findings_triage(&cwd);
        }
        Ok(outcome)
    }

    pub(crate) fn done_for(&self, identity: &Identity, cwd: &Path) -> Result<Outcome> {
        let root = git_root(&resolved(cwd)).ok_or_else(|| AppError::operational("done requires a Git worktree"))?;
        let repo_root = path_text(&root)?;
        let mut store = self.store()?;
        let Some(work) = store.work(identity)? else {
            return Ok(Outcome::new(OutcomeKind::Done, 0, "already clear"));
        };
        if work.claim(&repo_root).is_none() {
            if work.claims.len() == 1 {
                return Ok(Outcome::new(OutcomeKind::Done, 0, "already clear"));
            }
            return Err(AppError::operational(format!(
                "repository bundle does not claim {repo_root}; run ai-coord done from a claimed repository"
            )));
        }
        self.release_work(&mut store, identity, work)
    }

    fn release_work(&self, store: &mut Store, identity: &Identity, work: WorkRow) -> Result<Outcome> {
        let bundle = work.claims.len() > 1;
        let mut claims = Vec::with_capacity(work.claims.len());
        for claim in &work.claims {
            if work.state != WorkState::Active {
                claims.push(ReleaseClaimPlan {
                    repo_root: claim.repo_root.clone(),
                    hashes: Vec::new(),
                    residual_paths: Vec::new(),
                    inspected: false,
                });
                continue;
            }
            let inspection = git_dirty_paths(Path::new(&claim.repo_root));
            let (hashes, residual_paths, inspected) = match inspection {
                Ok(dirty) => (
                    git_blob_hashes(Path::new(&claim.repo_root), &dirty, false),
                    relevant_dirty(&claim.scopes, &dirty),
                    true,
                ),
                Err(_) if bundle => (Vec::new(), Vec::new(), false),
                Err(error) => return Err(error),
            };
            claims.push(ReleaseClaimPlan { repo_root: claim.repo_root.clone(), hashes, residual_paths, inspected });
        }
        let plan = ReleasePlan { work, claims };

        let removed = store.with_work_transaction(|transaction| {
            let current = transaction.work(identity)?;
            if !current
                .as_ref()
                .is_some_and(|current| current.id == plan.work.id && current.revision == plan.work.revision)
            {
                return Err(AppError::retry("work changed during release; retry ai-coord done"));
            }
            let waiters = overlapping_waiters(&transaction.works()?, &plan.work, identity);
            if plan.work.state == WorkState::Active {
                for claim in &plan.claims {
                    if claim.inspected {
                        transaction.observe_dirt(&claim.repo_root, &claim.hashes, self.clock.wall())?;
                        transaction.record_residual_owners(
                            &claim.repo_root,
                            &claim.residual_paths,
                            identity,
                            self.clock.wall(),
                        )?;
                    }
                }
            }
            let removed = transaction.delete_work(identity)?;
            if removed {
                let text = sanitize(
                    &format!("Released work '{}'; your queued work may now be ready.", plan.work.label),
                    super::MAX_MESSAGE_CHARS,
                );
                for (waiter, repo_root) in waiters {
                    transaction.send_message(identity, &waiter, &text, Some(&repo_root), self.clock.wall())?;
                }
            }
            Ok(removed)
        })?;
        let mut outcome = Outcome::new(OutcomeKind::Done, 0, if removed { "released" } else { "already clear" });
        outcome.holders = plan
            .claims
            .iter()
            .flat_map(|claim| {
                claim
                    .residual_paths
                    .iter()
                    .map(|path| if bundle { qualified_path(&claim.repo_root, path) } else { path.clone() })
            })
            .collect();
        Ok(outcome)
    }

    /// End an identity authoritatively and wake each overlapping queued item once.
    pub(crate) fn end_session_for(&self, identity: &Identity) -> Result<()> {
        let mut store = self.store()?;
        let released = store.work(identity)?;
        let all_work = store.works()?;
        let wakeups = released
            .as_ref()
            .filter(|work| work.state != WorkState::Draft)
            .map(|work| overlapping_waiters(&all_work, work, identity))
            .unwrap_or_default();
        store.end_session(identity)?;
        notify_session_release(&mut store, identity, released.as_ref(), wakeups, self.clock.wall())
    }

    pub(crate) fn baselines(&self) -> Result<Vec<BaselineRow>> {
        let identity = self.required_identity()?;
        let cwd = std::env::current_dir()?;
        self.baselines_for(&identity, &cwd)
    }

    pub(crate) fn baselines_for(&self, identity: &Identity, cwd: &Path) -> Result<Vec<BaselineRow>> {
        let root = git_root(&resolved(cwd)).ok_or_else(|| AppError::operational("baseline requires a Git worktree"))?;
        let repo_root = path_text(&root)?;
        let store = self.store()?;
        let Some(work) = store.work(identity)?.filter(|work| work.state == WorkState::Active) else {
            return Ok(Vec::new());
        };
        if work.claim(&repo_root).is_some() {
            return store.baselines_in_repo(identity, &repo_root);
        }
        if work.claims.len() > 1 {
            return Err(AppError::operational(format!(
                "active repository bundle does not claim {repo_root}; run baseline from a claimed repository"
            )));
        }
        Ok(Vec::new())
    }

    pub(crate) fn touched(&self, cwd: &Path) -> Result<TouchedPaths> {
        let identity = self.required_identity()?;
        let root = git_root(cwd).ok_or_else(|| AppError::operational("touched requires a Git worktree"))?;
        self.store()?.touched(&identity, &path_text(&root)?)
    }

    pub(super) fn reconcile_processes(&self, store: &mut Store) -> Result<HashSet<Client>> {
        let sessions = store.sessions()?;
        let observations = process_sweep(
            self.probe.as_ref(),
            sessions.iter().map(|row| (row.identity.clone(), row.fingerprint.clone())),
        );
        let revisions = sessions.iter().map(|row| (row.identity.clone(), row.revision)).collect::<HashMap<_, _>>();
        let dead = observations
            .iter()
            .filter(|observation| observation.liveness == ProcessLiveness::Dead)
            .map(|observation| EndedObservation {
                identity: observation.identity.clone(),
                expected_fingerprint: observation.expected_fingerprint.clone(),
                expected_revision: revisions[&observation.identity],
            })
            .collect::<Vec<_>>();
        let released_work = dead
            .iter()
            .map(|observation| Ok((observation.identity.clone(), store.work(&observation.identity)?)))
            .collect::<Result<Vec<_>>>()?;
        store.reconcile_ended(&dead)?;
        let remaining_work = store.works()?;
        for (identity, released) in released_work {
            if store.session(&identity)?.is_some() {
                continue;
            }
            let wakeups = released
                .as_ref()
                .filter(|work| work.state != WorkState::Draft)
                .map(|work| overlapping_waiters(&remaining_work, work, &identity))
                .unwrap_or_default();
            notify_session_release(store, &identity, released.as_ref(), wakeups, self.clock.wall())?;
        }
        Ok(observations
            .iter()
            .filter(|observation| observation.liveness == ProcessLiveness::Unknown)
            .map(|observation| observation.identity.client)
            .collect())
    }
}

fn normalized_label(label: &str) -> Result<String> {
    let label = sanitize(label, MAX_LABEL_CHARS);
    if label.is_empty() {
        return Err(AppError::usage("label must contain printable text"));
    }
    Ok(label)
}

fn claim_update(repo_root: String, scopes: Vec<Scope>) -> WorkClaimUpdate {
    WorkClaimUpdate { repo_root, blocked_reason: None, scopes, baselines: Some(Vec::new()), residual_paths: Vec::new() }
}

fn require_ordinary_item(existing: Option<&WorkRow>, root: &Path, command: &str) -> Result<()> {
    let Some(existing) = existing else {
        return Ok(());
    };
    let repo_root = path_text(root)?;
    if existing.claims.len() == 1 && existing.claim(&repo_root).is_some() {
        return Ok(());
    }
    Err(AppError::operational(format!(
        "existing work has {} repository claim(s) and cannot be changed by ai-coord {command}; use ai-coord bundle {command} or ai-coord done",
        existing.claims.len()
    )))
}

fn revalidate_draft(draft: &WorkRow) -> Result<()> {
    for claim in &draft.claims {
        let root = Path::new(&claim.repo_root);
        let files = claim
            .scopes
            .iter()
            .filter(|scope| scope.kind == ScopeKind::Exact)
            .map(|scope| PathBuf::from(&scope.path))
            .collect::<Vec<_>>();
        let recursive = claim
            .scopes
            .iter()
            .filter(|scope| scope.kind == ScopeKind::Recursive)
            .map(|scope| PathBuf::from(&scope.path))
            .collect::<Vec<_>>();
        let normalized = normalize_work_scopes(&files, &recursive, root, root)?;
        if normalized.len() != claim.scopes.len() || normalized.iter().any(|scope| !claim.scopes.contains(scope)) {
            return Err(AppError::usage(format!(
                "stored draft scopes for {} no longer normalize to the same paths",
                claim.repo_root
            )));
        }
    }
    Ok(())
}

fn work_paths(work: &WorkRow, qualified: bool) -> Vec<String> {
    work.claims
        .iter()
        .flat_map(|claim| {
            claim.scopes.iter().map(move |scope| {
                if qualified { qualified_path(&claim.repo_root, &scope.path) } else { scope.path.clone() }
            })
        })
        .collect()
}

fn qualified_path(repo_root: &str, path: &str) -> String {
    if path == "." { repo_root.to_owned() } else { Path::new(repo_root).join(path).to_string_lossy().into_owned() }
}

fn overlapping_waiters(work: &[WorkRow], released: &WorkRow, identity: &Identity) -> Vec<(Identity, String)> {
    let mut waiters = work
        .iter()
        .filter(|candidate| candidate.identity != *identity && candidate.state == WorkState::Queued)
        .filter_map(|candidate| {
            released
                .claims
                .iter()
                .find(|claim| {
                    candidate.claim(&claim.repo_root).is_some_and(|other| any_overlap(&claim.scopes, &other.scopes))
                })
                .map(|claim| (candidate.identity.clone(), claim.repo_root.clone()))
        })
        .collect::<Vec<_>>();
    waiters.sort_by(|left, right| {
        crate::domain::client_name(left.0.client)
            .cmp(crate::domain::client_name(right.0.client))
            .then_with(|| left.0.session_id.cmp(&right.0.session_id))
            .then_with(|| left.1.cmp(&right.1))
    });
    waiters.dedup_by(|left, right| left.0 == right.0);
    waiters
}

fn notify_session_release(
    store: &mut Store,
    identity: &Identity,
    released: Option<&WorkRow>,
    wakeups: Vec<(Identity, String)>,
    current: f64,
) -> Result<()> {
    let Some(released) = released else {
        return Ok(());
    };
    let text = sanitize(
        &format!("Session ended; released work '{}'; your queued work may now be ready.", released.label),
        super::MAX_MESSAGE_CHARS,
    );
    for (waiter, repo_root) in wakeups {
        store.send_message(identity, &[waiter], &text, Some(&repo_root), current)?;
    }
    Ok(())
}
