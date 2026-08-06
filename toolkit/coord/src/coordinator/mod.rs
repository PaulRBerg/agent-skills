mod inventory;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

use crate::{
    domain::{
        Client, Identity, InventoryResult, Outcome, OutcomeKind, OutsideScopeV2, ProcessLiveness, ProcessProbe,
        ProviderReport, Scope, ScopeKind, SessionState, SnapshotDelegateV2, SnapshotNoteV2, SnapshotScopeKindV2,
        SnapshotScopeV2, SnapshotSessionV2, SnapshotV2, SnapshotWorkV2, WorkState,
    },
    error::{AppError, Result},
    host::{
        INVENTORY_CACHE_SECONDS, NativeProcessProbe, any_overlap, from_environment, git_blob_hash, git_dirty_paths,
        git_root, host_process_reference, identity_key, normalize_work_scopes, process_ancestors, process_sweep,
        relevant_dirty,
    },
    server::{SnapshotMessageV1, SnapshotSource},
    state::{BaselineRow, EndedObservation, MessageRow, ProviderCacheRow, SessionRow, SessionUpdate, Store, WorkRow},
    work::WorkCoordinator,
};

#[cfg(test)]
pub(crate) use inventory::InventoryObservation;
pub(crate) use inventory::{HostInventory, ProviderInventory};

const FULL_REFRESH_SECONDS: f64 = 20.0;
const MAX_CALLSIGN_CODEPOINTS: usize = 40;
const MAX_LABEL_CHARS: usize = 80;
const MAX_MESSAGE_CHARS: usize = 240;

pub(crate) trait Clock: Send + Sync {
    fn wall(&self) -> f64;
    fn monotonic(&self) -> f64;
    fn sleep(&self, duration: Duration);
}

struct SystemClock {
    epoch: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self { epoch: Instant::now() }
    }
}

impl Clock for SystemClock {
    fn wall(&self) -> f64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64()
    }
    fn monotonic(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

/// Deep application service. It retains only thread-safe adapters and a ledger
/// path; every operation opens a short-lived SQLite connection.
pub(crate) struct Coordinator {
    store_path: PathBuf,
    inventory: Mutex<Box<dyn ProviderInventory>>,
    probe: Arc<dyn ProcessProbe>,
    clock: Arc<dyn Clock>,
}

impl Coordinator {
    pub(crate) fn open_default() -> Result<Self> {
        let store = Store::open_default()?;
        Ok(Self::new(store))
    }

    pub(crate) fn new(store: Store) -> Self {
        Self::with_components(
            store,
            Box::new(HostInventory::discover()),
            Arc::new(NativeProcessProbe::new()),
            Arc::new(SystemClock::default()),
        )
    }

    pub(crate) fn with_components(
        store: Store,
        inventory: Box<dyn ProviderInventory>,
        probe: Arc<dyn ProcessProbe>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { store_path: store.path().to_owned(), inventory: Mutex::new(inventory), probe, clock }
    }

    pub(crate) fn store(&self) -> Result<Store> {
        Store::open(&self.store_path)
    }

    pub(crate) fn now(&self) -> f64 {
        self.clock.wall()
    }

    pub(crate) fn identity(&self, required: bool) -> Result<Option<Identity>> {
        if let Some(identity) = from_environment() {
            return Ok(Some(identity));
        }
        let references =
            process_ancestors(None).into_iter().filter(|reference| reference.start_token.is_some()).collect::<Vec<_>>();
        let candidates = self.store()?.identities_for_exact_processes(&references)?;
        let unique = candidates.into_iter().collect::<HashSet<_>>();
        if unique.len() == 1 {
            return Ok(unique.into_iter().next());
        }
        if required {
            Err(AppError::operational("could not resolve a unique Codex or Claude session identity"))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn start(&self, label: &str, files: &[PathBuf], recursive: &[PathBuf], cwd: &Path) -> Result<Outcome> {
        let identity = self.identity(true)?.expect("required identity");
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
        let label = sanitize(label, MAX_LABEL_CHARS);
        if label.is_empty() {
            return Err(AppError::usage("label must contain printable text"));
        }
        let scopes = normalize_work_scopes(files, recursive, &cwd, &root)?;
        if scopes.is_empty() {
            return Err(AppError::usage("at least one scope is required"));
        }
        let mut store = self.store()?;
        if let Some(existing) = store.work(&identity)? {
            if existing.state == WorkState::Draft {
                return Err(AppError::operational(
                    "a draft exists; update it with ai-coord draft, then submit it with ai-coord start --draft",
                ));
            }
            if existing.repo_root != path_text(&root)? {
                return Ok(Outcome::new(OutcomeKind::Active, 3, "active work belongs to another repository")
                    .with_paths(existing.scopes.into_iter().map(|scope| scope.path).collect()));
            }
        }
        self.ensure_session(&mut store, &identity, &cwd, Some(&root))?;
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

    pub(crate) fn draft(&self, label: &str, files: &[PathBuf], recursive: &[PathBuf], cwd: &Path) -> Result<Outcome> {
        let identity = self.identity(true)?.expect("required identity");
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
        let label = sanitize(label, MAX_LABEL_CHARS);
        if label.is_empty() {
            return Err(AppError::usage("label must contain printable text"));
        }
        let scopes = normalize_work_scopes(files, recursive, &cwd, &root)?;
        if scopes.is_empty() {
            return Err(AppError::usage("at least one scope is required"));
        }
        let mut store = self.store()?;
        if store.work(&identity)?.is_some_and(|work| work.state != WorkState::Draft) {
            return Err(AppError::operational("queued or active work exists; run ai-coord done before drafting"));
        }
        self.ensure_session(&mut store, &identity, &cwd, Some(&root))?;
        store.save_draft(&identity, &path_text(&root)?, &label, &scopes, self.clock.wall())?;
        Ok(Outcome::new(OutcomeKind::Draft, 0, scopes.len().to_string()))
    }

    pub(crate) fn promote_draft(&self, cwd: &Path) -> Result<Outcome> {
        let identity = self.identity(true)?.expect("required identity");
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
        if draft.repo_root != path_text(&root)? {
            return Err(AppError::operational("draft belongs to another repository"));
        }
        revalidate_draft_scopes(&draft.scopes, &root)?;
        self.ensure_session(&mut store, identity, &cwd, Some(&root))?;
        let inventory = self.refresh_inventory(&mut store, false)?;
        WorkCoordinator { store: &mut store }.promote_draft(identity, &root, draft, &inventory, self.clock.wall())
    }

    pub(crate) fn wait(&self, timeout_seconds: u64, poll_seconds: f64) -> Result<Outcome> {
        let identity = self.identity(true)?.expect("required identity");
        self.wait_for(&identity, timeout_seconds, poll_seconds, false)
    }

    pub(crate) fn wait_for(
        &self,
        identity: &Identity,
        timeout_seconds: u64,
        poll_seconds: f64,
        released_if_missing: bool,
    ) -> Result<Outcome> {
        if !(1..=3600).contains(&timeout_seconds) {
            return Err(AppError::usage("timeout must be between 1 and 3600 seconds"));
        }
        let started = self.clock.monotonic();
        let note_baseline = self.clock.wall();
        let mut last_generation = None;
        let mut last_full_check = None;
        loop {
            let mut store = self.store()?;
            let process_complete = self.reconcile_processes(&mut store)?.is_empty();
            let Some(work) = store.work(identity)? else {
                return if released_if_missing {
                    Ok(Outcome::new(OutcomeKind::Released, 3, ""))
                } else {
                    Err(AppError::operational("no active or queued work for this session"))
                };
            };
            if work.state == WorkState::Active {
                return Ok(Outcome::new(OutcomeKind::Ready, 0, "")
                    .with_paths(work.scopes.into_iter().map(|scope| scope.path).collect()));
            }
            if work.state == WorkState::Draft {
                return Err(AppError::operational(
                    "draft work must be submitted with ai-coord start --draft before waiting",
                ));
            }
            let pending = store.inbox(identity, true)?;
            if !pending.is_empty() {
                return Ok(Outcome::new(OutcomeKind::Message, 3, pending.len().to_string()));
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
                let root = PathBuf::from(&work.repo_root);
                let outcome = WorkCoordinator { store: &mut store }.start_direct(
                    identity,
                    &root,
                    &work.label,
                    work.scopes.clone(),
                    &inventory,
                    self.clock.wall(),
                )?;
                last_full_check = Some(self.clock.monotonic());
                last_generation = Some(store.generation()?);
                if outcome.code == 0 || (outcome.code == 2 && !outcome.detail.starts_with("dirty-settling:")) {
                    return Ok(outcome);
                }
            }
            let notes = store.notes(&work.repo_root, Some(note_baseline))?;
            if !notes.is_empty() {
                return Ok(Outcome::new(OutcomeKind::Note, 3, notes.len().to_string()));
            }
            let elapsed = self.clock.monotonic() - started;
            if elapsed >= timeout_seconds as f64 {
                return Ok(Outcome::new(OutcomeKind::Timeout, 3, timeout_seconds.to_string()));
            }
            self.clock.sleep(Duration::from_secs_f64(poll_seconds.min(timeout_seconds as f64 - elapsed).max(0.001)));
        }
    }

    pub(crate) fn done(&self) -> Result<Outcome> {
        let identity = self.identity(true)?.expect("required identity");
        self.done_for(&identity)
    }

    pub(crate) fn done_for(&self, identity: &Identity) -> Result<Outcome> {
        let mut store = self.store()?;
        let work = store.work(identity)?;
        let mut waiters = Vec::new();
        if let Some(work) = work.as_ref().filter(|work| work.state == WorkState::Active && !work.scopes.is_empty()) {
            let root = PathBuf::from(&work.repo_root);
            let dirty = git_dirty_paths(&root).unwrap_or_default();
            let hashes = dirty.iter().map(|path| (path.clone(), git_blob_hash(&root, path, false))).collect::<Vec<_>>();
            let _ = store.observe_dirt(&work.repo_root, &hashes, self.clock.wall());
            let residual = relevant_dirty(&work.scopes, &dirty);
            store.record_residual_owners(&work.repo_root, &residual, identity, self.clock.wall())?;
            waiters = store
                .works(Some(&work.repo_root))?
                .into_iter()
                .filter(|candidate| {
                    candidate.state == WorkState::Queued && any_overlap(&work.scopes, &candidate.scopes)
                })
                .map(|work| work.identity)
                .collect();
        }
        let removed = store.delete_work(identity)?;
        if removed &&
            !waiters.is_empty() &&
            let Some(work) = work
        {
            let text = format!("Released work '{}'; your queued work may now be ready.", work.label);
            store.send_message(identity, &waiters, &text, Some(&work.repo_root), self.clock.wall())?;
        }
        Ok(Outcome::new(OutcomeKind::Done, 0, if removed { "released" } else { "already clear" }))
    }

    pub(crate) fn baselines(&self) -> Result<Vec<BaselineRow>> {
        let identity = self.identity(true)?.expect("required identity");
        let store = self.store()?;
        Ok(if store.work(&identity)?.is_some_and(|work| work.state == WorkState::Active) {
            store.baselines(&identity)?
        } else {
            Vec::new()
        })
    }

    pub(crate) fn snapshot(&self, machine_wide: bool, cwd: &Path, allow_cached: bool) -> Result<SnapshotV2> {
        let mut store = self.store()?;
        let inventory = self.refresh_inventory(&mut store, allow_cached)?;
        let self_identity = self.identity(false)?;
        let cwd = resolved(cwd);
        let root = git_root(&cwd);
        let sessions = store.sessions()?;
        let work = store.works(None)?;
        let roots = sessions
            .iter()
            .filter_map(|row| row.repo_root.clone())
            .chain(work.iter().map(|work| work.repo_root.clone()))
            .collect::<HashSet<_>>();
        let observation_roots = if machine_wide {
            roots.clone()
        } else {
            root.as_ref().and_then(|path| path.to_str()).map(str::to_owned).into_iter().collect()
        };
        for value in observation_roots {
            let root = PathBuf::from(&value);
            if let Ok(dirty) = git_dirty_paths(&root) {
                let hashes =
                    dirty.iter().map(|path| (path.clone(), git_blob_hash(&root, path, false))).collect::<Vec<_>>();
                let _ = store.observe_dirt(&value, &hashes, self.clock.wall());
            }
        }
        build_snapshot(&store, inventory, self_identity, machine_wide, &cwd, root.as_deref(), sessions, work, roots)
    }

    pub(crate) fn name(&self, callsign: &str, cwd: &Path) -> Result<String> {
        let identity = self.identity(true)?.expect("required identity");
        let callsign = normalize_callsign(callsign)?;
        let cwd = resolved(cwd);
        let root = git_root(&cwd).ok_or_else(|| AppError::operational("name requires a Git worktree"))?;
        let mut store = self.store()?;
        self.ensure_session(&mut store, &identity, &cwd, Some(&root))?;
        store.set_session_callsign(&identity, &callsign)?;
        Ok(callsign)
    }

    pub(crate) fn send(&self, target: &str, text: &str, cwd: &Path) -> Result<(Vec<String>, usize)> {
        let sender = self.identity(true)?.expect("required identity");
        let text = sanitize(text, MAX_MESSAGE_CHARS);
        if text.is_empty() {
            return Err(AppError::usage("message must contain printable text"));
        }
        let mut store = self.store()?;
        let _ = self.refresh_inventory(&mut store, true)?;
        let root = git_root(&resolved(cwd));
        let recipients = resolve_targets(target, &store.sessions()?, &store.works(None)?, root.as_deref(), &sender)?;
        let ids = store.send_message(
            &sender,
            &recipients,
            &text,
            root.as_ref().and_then(|path| path.to_str()),
            self.clock.wall(),
        )?;
        let count = recipients.len();
        Ok((ids, count))
    }

    pub(crate) fn inbox(&self, pending_only: bool) -> Result<Vec<MessageRow>> {
        let identity = self.identity(true)?.expect("required identity");
        self.store()?.inbox(&identity, pending_only)
    }

    pub(crate) fn acknowledge(&self, message_id: Option<&str>) -> Result<usize> {
        let identity = self.identity(true)?.expect("required identity");
        self.store()?.acknowledge(&identity, message_id, self.clock.wall())
    }

    pub(crate) fn add_note(&self, text: &str, cwd: &Path) -> Result<String> {
        let identity = self.identity(true)?.expect("required identity");
        let root = git_root(&resolved(cwd)).ok_or_else(|| AppError::operational("note requires a Git worktree"))?;
        let text = sanitize(text, MAX_MESSAGE_CHARS);
        if text.is_empty() {
            return Err(AppError::usage("note must contain printable text"));
        }
        self.store()?.add_note(&identity, &path_text(&root)?, &text, self.clock.wall())
    }

    pub(crate) fn resolve_note(&self, note_id: &str, cwd: &Path) -> Result<bool> {
        let root = git_root(&resolved(cwd)).ok_or_else(|| AppError::operational("note requires a Git worktree"))?;
        self.store()?.resolve_note(&path_text(&root)?, note_id, self.clock.wall())
    }

    pub(crate) fn trailer(&self) -> Result<String> {
        Ok(format!("Agent-Session: {}", identity_key(&self.identity(true)?.expect("required identity"))))
    }

    pub(crate) fn generation_with_reconcile(&self) -> Result<u64> {
        let mut store = self.store()?;
        self.reconcile_processes(&mut store)?;
        u64::try_from(store.generation()?).map_err(|_| AppError::operational("negative ledger generation"))
    }

    fn ensure_session(&self, store: &mut Store, identity: &Identity, cwd: &Path, root: Option<&Path>) -> Result<()> {
        let existing = store.session(identity)?;
        let fingerprint = existing
            .as_ref()
            .and_then(|row| row.fingerprint.clone())
            .filter(|value| value.start_token.is_some())
            .or_else(|| host_process_reference(identity.client, None).ok().flatten());
        let update = SessionUpdate {
            identity: identity.clone(),
            cwd: path_text(cwd)?,
            repo_root: root.map(path_text).transpose()?,
            state: existing.as_ref().map(|row| row.state).unwrap_or(SessionState::Working),
            source: existing.as_ref().map(|row| row.source.clone()).unwrap_or_else(|| "cli".to_owned()),
            name: existing.as_ref().and_then(|row| row.name.clone()),
            waiting_for: existing.as_ref().and_then(|row| row.waiting_for.clone()),
            permission_mode: None,
            update_permission_mode: false,
            fingerprint,
            started_at: existing.as_ref().map(|row| row.started_at),
            current: self.clock.wall(),
        };
        store.upsert_session(&update)?;
        Ok(())
    }

    fn reconcile_processes(&self, store: &mut Store) -> Result<HashSet<Client>> {
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
        store.reconcile_ended(&dead)?;
        Ok(observations
            .iter()
            .filter(|observation| observation.liveness == ProcessLiveness::Unknown)
            .map(|observation| observation.identity.client)
            .collect())
    }

    fn refresh_inventory(&self, store: &mut Store, allow_cached: bool) -> Result<InventoryResult> {
        store.prune(self.clock.wall())?;
        let mut unknown_clients = self.reconcile_processes(store)?;
        let mut inventory = self.inventory.lock().expect("inventory lock poisoned");
        let cache_key = inventory.cache_key().to_owned();
        let mut result = if allow_cached { cached_inventory(store, &cache_key, self.clock.wall())? } else { None };
        if result.is_none() {
            let observation = inventory.refresh(store, self.probe.as_ref())?;
            if observation.claude_authoritative {
                let updates = observation
                    .claude_sessions
                    .into_iter()
                    .map(|row| SessionUpdate {
                        identity: row.identity,
                        cwd: path_text(&row.cwd).unwrap_or_else(|_| row.cwd.to_string_lossy().into_owned()),
                        repo_root: row.repo_root.as_ref().map(|path| path.to_string_lossy().into_owned()),
                        state: row.state,
                        source: "observer".to_owned(),
                        name: row.name,
                        waiting_for: row.waiting_for,
                        permission_mode: None,
                        update_permission_mode: false,
                        fingerprint: row.fingerprint,
                        started_at: Some(row.started_at),
                        current: self.clock.wall(),
                    })
                    .collect::<Vec<_>>();
                store.replace_claude_sessions(&updates)?;
                unknown_clients = self.reconcile_processes(store)?;
            }
            if observation.result.complete {
                let rows = observation
                    .result
                    .providers
                    .iter()
                    .map(|report| ProviderCacheRow {
                        context_key: cache_key.clone(),
                        client: report.client,
                        refreshed_at: self.clock.wall(),
                        ok: report.ok,
                        source: report.source.clone(),
                        enabled: report.enabled,
                        dropped: report.dropped,
                    })
                    .collect::<Vec<_>>();
                store.replace_provider_cache(&cache_key, &rows, self.clock.wall())?;
            } else {
                store.clear_provider_cache()?;
            }
            result = Some(observation.result);
        }
        let mut result = result.expect("inventory result assigned");
        if !unknown_clients.is_empty() {
            result.complete = false;
            for report in &mut result.providers {
                if unknown_clients.contains(&report.client) && report.enabled && report.ok {
                    report.ok = false;
                    report.error = Some("process liveness unknown".to_owned());
                }
            }
        }
        Ok(result)
    }
}

impl SnapshotSource for Coordinator {
    fn snapshot(&self) -> Result<SnapshotV2> {
        self.snapshot(true, &std::env::current_dir()?, true)
    }
    fn messages(&self) -> Result<Vec<SnapshotMessageV1>> {
        Ok(self.store()?.all_messages()?.into_iter().map(message_snapshot).collect())
    }
    fn generation(&self) -> Result<u64> {
        self.generation_with_reconcile()
    }
}

#[allow(clippy::too_many_arguments)]
fn build_snapshot(
    store: &Store,
    inventory: InventoryResult,
    self_identity: Option<Identity>,
    machine: bool,
    cwd: &Path,
    root: Option<&Path>,
    sessions: Vec<SessionRow>,
    work: Vec<WorkRow>,
    roots: HashSet<String>,
) -> Result<SnapshotV2> {
    let work_by_identity = work.iter().map(|work| (work.identity.clone(), work)).collect::<HashMap<_, _>>();
    let delegates = store.delegates()?;
    let delegate_counts = delegates.iter().fold(HashMap::<Identity, usize>::new(), |mut counts, row| {
        *counts.entry(row.parent.clone()).or_default() += 1;
        counts
    });
    let root_text = root.map(path_text).transpose()?;
    let mut scoped = Vec::new();
    let mut outside = Vec::new();
    for row in sessions {
        let session_work = work_by_identity.get(&row.identity).copied();
        let in_scope = machine || root_text.as_ref().is_some_and(|root| row.repo_root.as_ref() == Some(root));
        let snapshot = SnapshotSessionV2 {
            identity: row.identity.clone(),
            cwd: row.cwd,
            repo_root: row.repo_root,
            state: if session_work.is_some_and(|work| work.state == WorkState::Queued) {
                SessionState::Waiting
            } else {
                row.state
            },
            callsign: row.callsign,
            name: row.name,
            waiting_for: row.waiting_for,
            permission_mode: row.permission_mode,
            delegate_count: delegate_counts.get(&row.identity).copied().filter(|count| *count > 0),
            pid: row.fingerprint.map(|value| value.pid),
            source: row.source,
            started_at: row.started_at,
            last_seen: row.last_seen,
        };
        (if in_scope { &mut scoped } else { &mut outside }).push(snapshot);
    }
    let scoped_work = work
        .into_iter()
        .filter(|work| machine || root_text.as_ref() == Some(&work.repo_root))
        .map(|work| {
            let draft = work.state == WorkState::Draft;
            SnapshotWorkV2 {
                id: work.id,
                identity: work.identity,
                repo_root: work.repo_root,
                label: work.label,
                state: work.state,
                blocked_reason: work.blocked_reason,
                scope_count: draft.then_some(work.scopes.len()),
                scopes: (!draft).then_some(work.scopes),
                draft_created_at: work.draft_created_at,
                submitted_at: work.submitted_at,
                updated_at: work.updated_at,
            }
        })
        .collect();
    let notes = if machine {
        roots
            .into_iter()
            .map(|root| store.notes(&root, None))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect()
    } else {
        root_text.as_ref().map(|root| store.notes(root, None)).transpose()?.unwrap_or_default()
    };
    let parent_scope = scoped.iter().map(|row| row.identity.clone()).collect::<HashSet<_>>();
    let delegates = delegates
        .into_iter()
        .filter(|row| machine || parent_scope.contains(&row.parent))
        .map(|row| SnapshotDelegateV2 {
            parent_client: row.parent.client,
            parent_session_id: row.parent.session_id,
            agent_id: row.agent_id,
            agent_type: row.agent_type,
            state: row.state,
            last_seen: row.last_seen,
        })
        .collect();
    let outside_directories = outside.iter().map(|row| row.cwd.clone()).collect::<HashSet<_>>().len();
    Ok(SnapshotV2 {
        schema_version: 2,
        complete: inventory.complete,
        scope: if machine {
            SnapshotScopeV2 { kind: SnapshotScopeKindV2::Machine, repo_root: None }
        } else {
            SnapshotScopeV2 {
                kind: if root.is_some() { SnapshotScopeKindV2::Repo } else { SnapshotScopeKindV2::Cwd },
                repo_root: Some(root_text.unwrap_or_else(|| cwd.to_string_lossy().into_owned())),
            }
        },
        self_identity,
        providers: inventory.providers,
        sessions: scoped,
        work: scoped_work,
        notes: notes
            .into_iter()
            .map(|row| SnapshotNoteV2 {
                id: row.id,
                repo_root: row.repo_root,
                author_client: row.author.as_ref().map(|identity| identity.client),
                author_session_id: row.author.map(|identity| identity.session_id),
                text: row.text,
                created_at: row.created_at,
                resolved_at: row.resolved_at,
            })
            .collect(),
        delegates,
        outside_scope: OutsideScopeV2 { sessions: outside.len(), directories: outside_directories },
    })
}

fn cached_inventory(store: &Store, key: &str, current: f64) -> Result<Option<InventoryResult>> {
    let rows = store.provider_cache(key)?;
    if rows.len() != 2 {
        return Ok(None);
    }
    let refreshed = rows[0].refreshed_at;
    if rows.iter().any(|row| row.refreshed_at != refreshed) ||
        current - refreshed < 0.0 ||
        current - refreshed >= INVENTORY_CACHE_SECONDS
    {
        return Ok(None);
    }
    let providers = rows
        .into_iter()
        .map(|row| ProviderReport {
            client: row.client,
            ok: row.ok,
            source: row.source,
            enabled: row.enabled,
            dropped: row.dropped,
            error: None,
        })
        .collect::<Vec<_>>();
    let complete = providers.iter().all(|row| !row.enabled || (row.ok && row.dropped == 0));
    Ok(complete.then_some(InventoryResult { complete, providers }))
}

fn resolve_targets(
    target: &str,
    sessions: &[SessionRow],
    work: &[WorkRow],
    root: Option<&Path>,
    sender: &Identity,
) -> Result<Vec<Identity>> {
    if target == "repo" {
        let root = root.ok_or_else(|| AppError::operational("repo target requires a Git worktree"))?;
        let root = path_text(root)?;
        return Ok(sessions
            .iter()
            .filter(|row| row.repo_root.as_deref() == Some(&root) && row.identity != *sender)
            .map(|row| row.identity.clone())
            .collect());
    }
    let key = callsign_key(target);
    let exact = sessions
        .iter()
        .filter(|row| target == row.identity.session_id || target == identity_key(&row.identity))
        .collect::<Vec<_>>();
    let callsign = sessions
        .iter()
        .filter(|row| row.callsign.as_ref().is_some_and(|value| callsign_key(value) == key))
        .collect::<Vec<_>>();
    let prefix = if target.chars().count() >= 4 {
        sessions.iter().filter(|row| row.identity.session_id.starts_with(target)).collect()
    } else {
        Vec::new()
    };
    let work_labels = work.iter().map(|work| (&work.identity, work.label.as_str())).collect::<HashMap<_, _>>();
    let substring = sessions
        .iter()
        .filter(|row| {
            let work_label = work_labels.get(&row.identity).copied();
            let haystack = callsign_key(
                &[row.callsign.as_deref(), work_label, row.name.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            !key.is_empty() && haystack.contains(&key)
        })
        .collect::<Vec<_>>();
    let matches = if !exact.is_empty() {
        exact
    } else if !callsign.is_empty() {
        callsign
    } else if !prefix.is_empty() {
        prefix
    } else {
        substring
    };
    let unique = matches.into_iter().map(|row| row.identity.clone()).collect::<HashSet<_>>();
    if unique.len() != 1 {
        return Err(AppError::operational(format!(
            "message target matched {} sessions; use a unique id prefix",
            unique.len()
        )));
    }
    Ok(unique.into_iter().collect())
}

fn message_snapshot(row: MessageRow) -> SnapshotMessageV1 {
    SnapshotMessageV1 {
        id: row.id,
        sender_client: row.sender.client,
        sender_session_id: row.sender.session_id,
        sender_callsign: row.sender_callsign,
        recipient_client: row.recipient.client,
        recipient_session_id: row.recipient.session_id,
        recipient_callsign: row.recipient_callsign,
        repo_root: row.repo_root,
        text: row.text,
        created_at: row.created_at,
        acknowledged_at: row.acknowledged_at,
    }
}

fn normalize_callsign(text: &str) -> Result<String> {
    let value = text.nfc().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    if value.is_empty() {
        return Err(AppError::usage("callsign must contain text"));
    }
    if value.chars().count() > MAX_CALLSIGN_CODEPOINTS {
        return Err(AppError::usage(format!("callsign exceeds {MAX_CALLSIGN_CODEPOINTS} Unicode code points")));
    }
    if !value.chars().any(char::is_alphanumeric) {
        return Err(AppError::usage("callsign must contain at least one letter or number"));
    }
    if !value.chars().any(|value| matches!(value as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF)) {
        return Err(AppError::usage("callsign must contain at least one emoji"));
    }
    Ok(value)
}
fn callsign_key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .nfc()
        .case_fold()
        .nfc()
        .filter(|value| !matches!(value, '\u{fe0e}' | '\u{fe0f}'))
        .collect()
}
fn sanitize(text: &str, limit: usize) -> String {
    let value = text
        .chars()
        .map(|value| if value.is_control() { ' ' } else { value })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if value.chars().count() <= limit {
        value
    } else {
        let mut result = value.chars().take(limit - 1).collect::<String>();
        result.push('…');
        result
    }
}
fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| AppError::usage("path is not valid UTF-8"))
}

fn revalidate_draft_scopes(scopes: &[Scope], root: &Path) -> Result<()> {
    let files = scopes
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Exact)
        .map(|scope| PathBuf::from(&scope.path))
        .collect::<Vec<_>>();
    let recursive = scopes
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Recursive)
        .map(|scope| PathBuf::from(&scope.path))
        .collect::<Vec<_>>();
    let normalized = normalize_work_scopes(&files, &recursive, root, root)?;
    if normalized.len() != scopes.len() || normalized.iter().any(|scope| !scopes.contains(scope)) {
        return Err(AppError::usage("stored draft scopes no longer normalize to the same paths"));
    }
    Ok(())
}
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

#[cfg(test)]
mod tests;
