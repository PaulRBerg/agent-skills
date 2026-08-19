mod findings;
mod inventory;
mod triage;
mod triage_command;
mod triage_config;
mod triage_paths;
mod triage_prompt;
mod triage_schema;
mod work_lifecycle;

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
        Identity, InventoryResult, OutsideScopeV2, ProcessProbe, ProviderReport, SessionState, SnapshotDelegateV2,
        SnapshotHandoffV4, SnapshotScopeKindV2, SnapshotScopeV2, SnapshotSessionV2, SnapshotV2, SnapshotWorkClaimV2,
        SnapshotWorkV2, WorkState, sanitize,
    },
    error::{AppError, Result},
    host::{
        INVENTORY_CACHE_SECONDS, NativeProcessProbe, from_environment, git_blob_hashes, git_dirty_paths, git_root,
        host_process_reference, identity_key, process_ancestors,
    },
    server::{SnapshotMessageV1, SnapshotSource},
    state::{MessageRow, ProviderCacheRow, SessionRow, SessionUpdate, Store, WorkRow},
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

    /// Resolves the caller's identity, which `identity(true)` guarantees is `Some` on success.
    fn required_identity(&self) -> Result<Identity> {
        Ok(self.identity(true)?.expect("required identity"))
    }

    pub(crate) fn snapshot(&self, machine_wide: bool, cwd: &Path, allow_cached: bool) -> Result<SnapshotV2> {
        let mut store = self.store()?;
        let inventory = self.refresh_inventory(&mut store, allow_cached)?;
        let self_identity = self.identity(false)?;
        let cwd = resolved(cwd);
        let root = git_root(&cwd);
        let sessions = store.sessions()?;
        let work = store.works()?;
        let roots = sessions
            .iter()
            .filter_map(|row| row.repo_root.clone())
            .chain(work.iter().flat_map(|work| work.claims.iter().map(|claim| claim.repo_root.clone())))
            .collect::<HashSet<_>>();
        let observation_roots = if machine_wide {
            roots.clone()
        } else {
            root.as_ref().and_then(|path| path.to_str()).map(str::to_owned).into_iter().collect()
        };
        for value in observation_roots {
            let root = PathBuf::from(&value);
            if let Ok(dirty) = git_dirty_paths(&root) {
                let hashes = git_blob_hashes(&root, &dirty, false);
                let _ = store.observe_dirt(&value, &hashes, self.clock.wall());
            }
        }
        build_snapshot(
            &store,
            inventory,
            self_identity,
            machine_wide,
            &cwd,
            root.as_deref(),
            sessions,
            work,
            self.clock.wall(),
        )
    }

    pub(crate) fn name(&self, callsign: &str, cwd: &Path) -> Result<String> {
        let identity = self.required_identity()?;
        let callsign = normalize_callsign(callsign)?;
        let cwd = resolved(cwd);
        let root = git_root(&cwd).ok_or_else(|| AppError::operational("name requires a Git worktree"))?;
        let mut store = self.store()?;
        self.ensure_session(&mut store, &identity, &cwd, Some(&root))?;
        store.set_session_callsign(&identity, &callsign)?;
        Ok(callsign)
    }

    pub(crate) fn send(&self, target: &str, text: &str, cwd: &Path) -> Result<(Vec<String>, usize)> {
        let sender = self.required_identity()?;
        let text = sanitize(text, MAX_MESSAGE_CHARS);
        if text.is_empty() {
            return Err(AppError::usage("message must contain printable text"));
        }
        let mut store = self.store()?;
        let _ = self.refresh_inventory(&mut store, true)?;
        let root = git_root(&resolved(cwd));
        let recipients = resolve_targets(target, &store.sessions()?, &store.works()?, root.as_deref(), &sender)?;
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
        let identity = self.required_identity()?;
        self.store()?.inbox(&identity, pending_only)
    }

    pub(crate) fn acknowledge(&self, message_id: Option<&str>) -> Result<usize> {
        let identity = self.required_identity()?;
        self.store()?.acknowledge(&identity, message_id, self.clock.wall())
    }

    pub(crate) fn trailer(&self) -> Result<String> {
        Ok(format!("Agent-Session: {}", identity_key(&self.required_identity()?)))
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
            coordination_waived: None,
            fingerprint,
            started_at: existing.as_ref().map(|row| row.started_at),
            current: self.clock.wall(),
        };
        store.upsert_session(&update)?;
        Ok(())
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
                        coordination_waived: None,
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
    current: f64,
) -> Result<SnapshotV2> {
    let handoff_roots = if machine {
        sessions
            .iter()
            .filter_map(|row| row.repo_root.as_ref())
            .chain(work.iter().flat_map(|row| row.claims.iter().map(|claim| &claim.repo_root)))
            .map(PathBuf::from)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    } else {
        root.map(Path::to_path_buf).into_iter().collect()
    };
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
        let matching_work = work_by_identity
            .get(&row.identity)
            .copied()
            .filter(|work| machine || root_text.as_ref().is_some_and(|root| work.claim(root).is_some()));
        let in_scope = machine ||
            root_text.as_ref().is_some_and(|root| row.repo_root.as_ref() == Some(root)) ||
            matching_work.is_some();
        let snapshot = SnapshotSessionV2 {
            identity: row.identity.clone(),
            cwd: row.cwd,
            repo_root: row.repo_root,
            state: if !machine && matching_work.is_some_and(|work| work.state == WorkState::Queued) {
                SessionState::Waiting
            } else {
                row.state
            },
            callsign: row.callsign,
            name: row.name,
            waiting_for: row.waiting_for,
            permission_mode: row.permission_mode,
            coordination_waived: row.coordination_waived,
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
        .filter(|work| machine || root_text.as_ref().is_some_and(|root| work.claim(root).is_some()))
        .map(|work| {
            let draft = work.state == WorkState::Draft;
            let scope_count = work.claims.iter().map(|claim| claim.scopes.len()).sum::<usize>();
            let mut claims = work
                .claims
                .into_iter()
                .map(|claim| SnapshotWorkClaimV2 {
                    repo_root: claim.repo_root,
                    blocked_reason: claim.blocked_reason,
                    scope_count: claim.scopes.len(),
                    scopes: (!draft).then_some(claim.scopes),
                })
                .collect::<Vec<_>>();
            claims.sort_by(|left, right| left.repo_root.cmp(&right.repo_root));
            SnapshotWorkV2 {
                id: work.id,
                identity: work.identity,
                label: work.label,
                state: work.state,
                blocked_reason: work.blocked_reason,
                scope_count,
                claims,
                draft_created_at: work.draft_created_at,
                submitted_at: work.submitted_at,
                updated_at: work.updated_at,
            }
        })
        .collect();
    let findings = if machine {
        store.all_findings(current)?
    } else {
        root_text.as_ref().map(|root| store.findings(root, None, true, current)).transpose()?.unwrap_or_default()
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
    let handoffs = snapshot_handoffs(handoff_roots);
    Ok(SnapshotV2 {
        schema_version: 7,
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
        findings,
        handoffs,
        delegates,
        outside_scope: OutsideScopeV2 { sessions: outside.len(), directories: outside_directories },
    })
}

fn snapshot_handoffs(roots: Vec<PathBuf>) -> Vec<SnapshotHandoffV4> {
    let mut rows = roots
        .into_iter()
        .filter_map(|root| {
            let directory = root.join(".ai/task-handoffs");
            let count = std::fs::read_dir(directory)
                .ok()?
                .filter_map(std::result::Result::ok)
                .filter(|entry| {
                    entry.path().extension().is_some_and(|extension| extension == "md") &&
                        entry.file_type().is_ok_and(|kind| kind.is_file())
                })
                .count();
            if count == 0 {
                None
            } else {
                path_text(&root).ok().map(|repo_root| SnapshotHandoffV4 { repo_root, count })
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.repo_root.cmp(&right.repo_root));
    rows
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

pub(crate) fn normalize_callsign(text: &str) -> Result<String> {
    if text.chars().any(char::is_control) {
        return Err(AppError::usage("callsign must not contain control characters"));
    }
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
fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| AppError::usage("path is not valid UTF-8"))
}

fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

#[cfg(test)]
mod tests;
