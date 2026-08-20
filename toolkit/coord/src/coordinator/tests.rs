use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tempfile::TempDir;

use super::{
    Clock, Coordinator,
    inventory::{InventoryObservation, ProviderInventory, StaticInventory},
    normalize_callsign,
};
use crate::{
    domain::{
        Client, Identity, InventoryResult, OutcomeKind, ProcessFingerprint, ProcessLiveness, ProcessProbe, Scope,
        ScopeKind, SessionState, WorkState,
    },
    error::Result,
    host::{WorkClaimRequest, git_blob_hashes, git_dirty_paths},
    state::{SessionUpdate, Store, WorkClaimUpdate, WorkUpdate},
    work::WorkCoordinator,
};

#[derive(Default)]
struct FakeProbe {
    states: Mutex<HashMap<u32, ProcessLiveness>>,
}

impl FakeProbe {
    fn set(&self, pid: u32, state: ProcessLiveness) {
        self.states.lock().unwrap().insert(pid, state);
    }
}

impl ProcessProbe for FakeProbe {
    fn fingerprint(&self, pid: u32) -> Result<ProcessFingerprint> {
        Ok(ProcessFingerprint { pid, start_token: Some(format!("token-{pid}")) })
    }
    fn liveness(&self, fingerprint: &ProcessFingerprint) -> ProcessLiveness {
        self.states.lock().unwrap().get(&fingerprint.pid).copied().unwrap_or(ProcessLiveness::Unknown)
    }
}

#[derive(Default)]
struct FakeClock {
    value: Mutex<f64>,
}
impl FakeClock {
    fn new(value: f64) -> Self {
        Self { value: Mutex::new(value) }
    }
}
impl Clock for FakeClock {
    fn wall(&self) -> f64 {
        *self.value.lock().unwrap()
    }
    fn monotonic(&self) -> f64 {
        self.wall()
    }
    fn sleep(&self, duration: Duration) {
        *self.value.lock().unwrap() += duration.as_secs_f64();
    }
}

enum InventoryMutation {
    DeleteOnce,
    ReplaceOnce { label: String, repo_root: String, path: String, submitted_at: f64 },
    ReviseEveryTime,
}

struct MutatingInventory {
    identity: Identity,
    mutation: InventoryMutation,
    refreshes: usize,
}

impl ProviderInventory for MutatingInventory {
    fn cache_key(&self) -> &str {
        "mutating"
    }

    fn refresh(&mut self, store: &Store, _probe: &dyn ProcessProbe) -> Result<InventoryObservation> {
        let mutate = self.refreshes == 0 || matches!(self.mutation, InventoryMutation::ReviseEveryTime);
        self.refreshes += 1;
        if mutate {
            let mut writer = Store::open(store.path())?;
            match &self.mutation {
                InventoryMutation::DeleteOnce => {
                    writer.with_work_transaction(|transaction| transaction.delete_work(&self.identity).map(|_| ()))?;
                }
                InventoryMutation::ReplaceOnce { label, repo_root, path, submitted_at } => {
                    let current = writer.work(&self.identity)?.expect("queued work to replace");
                    writer.with_work_transaction(|transaction| {
                        assert!(transaction.delete_work(&self.identity)?);
                        transaction.save_work(&WorkUpdate {
                            identity: self.identity.clone(),
                            label: label.clone(),
                            state: WorkState::Queued,
                            blocked_reason: Some("coverage".to_owned()),
                            claims: vec![WorkClaimUpdate {
                                repo_root: repo_root.clone(),
                                blocked_reason: Some("coverage".to_owned()),
                                scopes: vec![Scope { path: path.clone(), kind: ScopeKind::Exact }],
                                baselines: None,
                                residual_paths: Vec::new(),
                            }],
                            draft_created_at: current.draft_created_at,
                            submitted_at: Some(*submitted_at),
                            updated_at: 200.0,
                            expected_revision: None,
                        })?;
                        Ok(())
                    })?;
                }
                InventoryMutation::ReviseEveryTime => {
                    let current = writer.work(&self.identity)?.expect("queued work to revise");
                    writer.with_work_transaction(|transaction| {
                        transaction.save_work(&WorkUpdate {
                            identity: self.identity.clone(),
                            label: current.label.clone(),
                            state: current.state,
                            blocked_reason: current.blocked_reason.clone(),
                            claims: current
                                .claims
                                .iter()
                                .map(|claim| WorkClaimUpdate {
                                    repo_root: claim.repo_root.clone(),
                                    blocked_reason: claim.blocked_reason.clone(),
                                    scopes: claim.scopes.clone(),
                                    baselines: None,
                                    residual_paths: Vec::new(),
                                })
                                .collect(),
                            draft_created_at: current.draft_created_at,
                            submitted_at: current.submitted_at,
                            updated_at: 200.0 + self.refreshes as f64,
                            expected_revision: Some(current.revision),
                        })?;
                        Ok(())
                    })?;
                }
            }
        }
        Ok(InventoryObservation {
            result: InventoryResult { complete: true, providers: Vec::new() },
            claude_sessions: Vec::new(),
            claude_authoritative: false,
        })
    }
}

fn repos(count: usize) -> (TempDir, Vec<PathBuf>) {
    let temp = TempDir::new().unwrap();
    let mut roots = Vec::new();
    for index in 0..count {
        let root = temp.path().join(format!("repo-{index}"));
        fs::create_dir_all(&root).unwrap();
        assert!(Command::new("git").args(["init", "-q"]).current_dir(&root).status().unwrap().success());
        roots.push(fs::canonicalize(root).unwrap());
    }
    roots.sort();
    (temp, roots)
}

fn identity(id: &str) -> Identity {
    Identity { client: Client::Codex, session_id: id.to_owned() }
}

fn add_session(store: &mut Store, identity: &Identity, root: &Path, pid: u32, current: f64) {
    store
        .upsert_session(&SessionUpdate {
            identity: identity.clone(),
            cwd: root.to_string_lossy().into_owned(),
            repo_root: Some(root.to_string_lossy().into_owned()),
            state: SessionState::Working,
            source: "test".to_owned(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(ProcessFingerprint { pid, start_token: Some(format!("token-{pid}")) }),
            transcript_path: None,
            started_at: Some(current),
            current,
        })
        .unwrap();
}

fn coordinator(store: Store, probe: Arc<FakeProbe>, refreshes: Arc<AtomicUsize>) -> Coordinator {
    coordinator_with_coverage(store, probe, refreshes, true)
}

fn coordinator_with_coverage(
    store: Store,
    probe: Arc<FakeProbe>,
    refreshes: Arc<AtomicUsize>,
    complete: bool,
) -> Coordinator {
    Coordinator::with_components(
        store,
        Box::new(StaticInventory { complete, refreshes }),
        probe,
        Arc::new(FakeClock::new(100.0)),
    )
}

fn fixture(count: usize, sessions: &[(&Identity, usize, u32)]) -> (TempDir, Vec<PathBuf>, Coordinator) {
    let (temp, roots) = repos(count);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    for (identity, root, pid) in sessions {
        add_session(&mut store, identity, &roots[*root], *pid, 1.0);
        probe.set(*pid, ProcessLiveness::Alive);
    }
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    (temp, roots, coordinator)
}

fn files(roots: &[PathBuf], names: &[&str]) -> Vec<PathBuf> {
    roots.iter().zip(names).map(|(root, name)| root.join(name)).collect()
}

#[test]
fn callsigns_reject_terminal_control_characters() {
    assert!(normalize_callsign("🚀 trusted\u{1b}[2J").is_err());
}

#[test]
fn clean_two_root_bundle_is_one_active_item_with_sorted_claims() {
    let owner = identity("owner");
    let (_temp, roots, coordinator) = fixture(2, &[(&owner, 0, 10)]);
    let requested = files(&roots, &["src/lib.rs", "README.md"]);

    let outcome = coordinator.start_bundle_for(owner.clone(), "bundle", &requested, &[], &roots[0]).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Ready);
    assert_eq!(outcome.paths, requested.iter().map(|path| path.to_string_lossy().into_owned()).collect::<Vec<_>>());
    let work = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_eq!(work.state, WorkState::Active);
    assert_eq!(
        work.claims.iter().map(|claim| claim.repo_root.as_str()).collect::<Vec<_>>(),
        roots.iter().map(|root| root.to_str().unwrap()).collect::<Vec<_>>()
    );
}

#[test]
fn incomplete_coverage_queues_every_bundle_claim() {
    let owner = identity("owner");
    let (temp, roots) = repos(2);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    add_session(&mut store, &owner, &roots[0], 11, 1.0);
    probe.set(11, ProcessLiveness::Alive);
    let coordinator = coordinator_with_coverage(store, probe, Arc::new(AtomicUsize::new(0)), false);

    let outcome = coordinator
        .start_bundle_for(owner.clone(), "bundle", &files(&roots, &["a.rs", "b.rs"]), &[], &roots[0])
        .unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Unknown);
    assert_eq!(outcome.detail, "coverage");
    let work = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_eq!(work.state, WorkState::Queued);
    assert!(work.claims.iter().all(|claim| claim.blocked_reason.as_deref() == Some("coverage")));
}

#[test]
fn fresh_dirt_in_one_repository_queues_the_whole_bundle() {
    let owner = identity("owner");
    let (_temp, roots, coordinator) = fixture(2, &[(&owner, 0, 12)]);
    fs::write(roots[1].join("dirty.rs"), "dirty\n").unwrap();

    let outcome = coordinator
        .start_bundle_for(owner.clone(), "bundle", &files(&roots, &["clean.rs", "dirty.rs"]), &[], &roots[0])
        .unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Unknown);
    assert!(outcome.detail.starts_with("dirty-settling:"));
    let work = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_eq!(work.state, WorkState::Queued);
    assert_eq!(work.claim(roots[0].to_str().unwrap()).unwrap().blocked_reason, None);
    assert_eq!(work.claim(roots[1].to_str().unwrap()).unwrap().blocked_reason.as_deref(), Some("dirty"));
}

#[test]
fn foreign_residual_in_one_repository_queues_the_whole_bundle() {
    let owner = identity("owner");
    let foreign = identity("foreign");
    let (_temp, roots, coordinator) = fixture(2, &[(&owner, 0, 13)]);
    fs::write(roots[1].join("residual.rs"), "dirty\n").unwrap();
    let dirty = git_dirty_paths(&roots[1]).unwrap();
    let hashes = git_blob_hashes(&roots[1], &dirty, false);
    let mut store = coordinator.store().unwrap();
    store.observe_dirt(roots[1].to_str().unwrap(), &hashes, 0.0).unwrap();
    store
        .with_work_transaction(|transaction| {
            transaction.record_residual_owners(roots[1].to_str().unwrap(), &["residual.rs".to_owned()], &foreign, 1.0)
        })
        .unwrap();
    drop(store);

    let outcome = coordinator
        .start_bundle_for(owner.clone(), "bundle", &files(&roots, &["clean.rs", "residual.rs"]), &[], &roots[0])
        .unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Blocked);
    let work = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_eq!(work.state, WorkState::Queued);
    assert_eq!(work.claim(roots[1].to_str().unwrap()).unwrap().blocked_reason.as_deref(), Some("residual"));
}

#[test]
fn failed_git_inspection_identifies_the_claim_and_queues_the_bundle() {
    let owner = identity("owner");
    let (temp, roots, coordinator) = fixture(1, &[(&owner, 0, 14)]);
    let missing = temp.path().join("missing-repo");
    let mut claims = vec![
        WorkClaimRequest {
            repo_root: roots[0].clone(),
            scopes: vec![Scope { path: "clean.rs".to_owned(), kind: ScopeKind::Exact }],
        },
        WorkClaimRequest {
            repo_root: missing.clone(),
            scopes: vec![Scope { path: "unknown.rs".to_owned(), kind: ScopeKind::Exact }],
        },
    ];
    claims.sort_by(|left, right| left.repo_root.cmp(&right.repo_root));
    let inventory = InventoryResult { complete: true, providers: Vec::new() };
    let mut store = coordinator.store().unwrap();

    let outcome =
        WorkCoordinator { store: &mut store }.start_claims(&owner, "bundle", claims, &inventory, 100.0).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Unknown);
    assert_eq!(outcome.detail, format!("inspection:{}", missing.display()));
    let work = store.work(&owner).unwrap().unwrap();
    assert_eq!(work.state, WorkState::Queued);
    assert_eq!(work.claim(missing.to_str().unwrap()).unwrap().blocked_reason.as_deref(), Some("inspection"));
}

#[test]
fn draft_promotion_modes_reject_mismatches_without_mutating_the_draft() {
    let bundled = identity("bundled");
    let ordinary = identity("ordinary");
    let (_temp, roots, coordinator) = fixture(2, &[(&bundled, 0, 20), (&ordinary, 0, 21)]);
    let requested = files(&roots, &["a.rs", "b.rs"]);

    coordinator.draft_bundle_for(bundled.clone(), "bundle draft", &requested, &[], &roots[0]).unwrap();
    let before = coordinator.store().unwrap().work(&bundled).unwrap().unwrap();
    let error = coordinator.promote_draft_for(&bundled, &roots[0]).unwrap_err();
    assert!(error.to_string().contains("bundle start --draft"));
    assert_eq!(coordinator.store().unwrap().work(&bundled).unwrap().unwrap(), before);
    assert_eq!(coordinator.promote_bundle_draft_for(&bundled, &roots[0]).unwrap().kind, OutcomeKind::Ready);

    coordinator.draft_for(ordinary.clone(), "one draft", &[PathBuf::from("one.rs")], &[], &roots[0]).unwrap();
    let before = coordinator.store().unwrap().work(&ordinary).unwrap().unwrap();
    let error = coordinator.promote_bundle_draft_for(&ordinary, &roots[0]).unwrap_err();
    assert!(error.to_string().contains("ai-coord start --draft"));
    assert_eq!(coordinator.store().unwrap().work(&ordinary).unwrap().unwrap(), before);
}

#[test]
fn wait_reevaluates_the_whole_bundle_and_preserves_fifo_age() {
    let holder = identity("holder");
    let early = identity("early");
    let late = identity("late");
    let (_temp, roots, coordinator) = fixture(2, &[(&holder, 1, 30), (&early, 0, 31), (&late, 0, 32)]);
    let scope = [PathBuf::from("shared.rs")];
    coordinator.start_for(holder.clone(), "holder", &scope, &[], &roots[1]).unwrap();
    let requested = files(&roots, &["owned.rs", "shared.rs"]);
    assert_eq!(
        coordinator.start_bundle_for(early.clone(), "early", &requested, &[], &roots[0]).unwrap().kind,
        OutcomeKind::Blocked
    );
    assert_eq!(
        coordinator.start_bundle_for(late.clone(), "late", &requested, &[], &roots[0]).unwrap().kind,
        OutcomeKind::Blocked
    );
    let early_age = coordinator.store().unwrap().work(&early).unwrap().unwrap().submitted_at;
    let late_age = coordinator.store().unwrap().work(&late).unwrap().unwrap().submitted_at;
    assert!(early_age < late_age);

    coordinator.done_for(&holder, &roots[1]).unwrap();
    let mut store = coordinator.store().unwrap();
    store.acknowledge(&early, None, 100.0).unwrap();
    store.acknowledge(&late, None, 100.0).unwrap();
    drop(store);
    assert_eq!(coordinator.wait_for_repo(&early, &roots[0], 1, 0.1, false).unwrap().kind, OutcomeKind::Ready);
    assert_eq!(coordinator.store().unwrap().work(&early).unwrap().unwrap().submitted_at, early_age);
    let blocked = coordinator.start_bundle_for(late.clone(), "late", &requested, &[], &roots[1]).unwrap();
    assert_eq!(blocked.kind, OutcomeKind::Blocked);
    assert_eq!(coordinator.store().unwrap().work(&late).unwrap().unwrap().submitted_at, late_age);
}

#[test]
fn wait_retries_a_replaced_snapshot_and_preserves_the_replacement() {
    let owner = identity("owner");
    let (temp, roots) = repos(1);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    add_session(&mut store, &owner, &roots[0], 33, 1.0);
    probe.set(33, ProcessLiveness::Alive);
    let queued = coordinator_with_coverage(store, probe.clone(), Arc::new(AtomicUsize::new(0)), false);
    assert_eq!(
        queued.start_for(owner.clone(), "old", &[PathBuf::from("old.rs")], &[], &roots[0]).unwrap().kind,
        OutcomeKind::Unknown
    );
    let original_id = queued.store().unwrap().work(&owner).unwrap().unwrap().id;
    let clock = Arc::new(FakeClock::new(100.0));
    let coordinator = Coordinator::with_components(
        queued.store().unwrap(),
        Box::new(MutatingInventory {
            identity: owner.clone(),
            mutation: InventoryMutation::ReplaceOnce {
                label: "replacement".to_owned(),
                repo_root: roots[0].to_string_lossy().into_owned(),
                path: "replacement.rs".to_owned(),
                submitted_at: 77.0,
            },
            refreshes: 0,
        }),
        probe,
        clock.clone(),
    );

    let outcome = coordinator.wait_for_repo(&owner, &roots[0], 1, 0.1, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Ready);
    assert_eq!(outcome.paths, ["replacement.rs"]);
    let work = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_ne!(work.id, original_id);
    assert_eq!(work.label, "replacement");
    assert_eq!(work.submitted_at, Some(77.0));
    assert_eq!(work.claims[0].scopes[0].path, "replacement.rs");
    assert!(clock.monotonic() >= 100.1, "a stale snapshot retry must poll instead of busy-looping");
}

#[test]
fn wait_returns_released_when_replacement_drops_the_observed_repository_claim() {
    let owner = identity("owner");
    let (temp, roots) = repos(2);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    add_session(&mut store, &owner, &roots[0], 38, 1.0);
    probe.set(38, ProcessLiveness::Alive);
    let queued = coordinator_with_coverage(store, probe.clone(), Arc::new(AtomicUsize::new(0)), false);
    assert_eq!(
        queued.start_for(owner.clone(), "old", &[PathBuf::from("old.rs")], &[], &roots[0]).unwrap().kind,
        OutcomeKind::Unknown
    );
    let initial_mismatch = queued.wait_for_repo(&owner, &roots[1], 1, 0.1, false).unwrap_err();
    assert!(initial_mismatch.to_string().contains("current repository is not claimed"));
    let original_id = queued.store().unwrap().work(&owner).unwrap().unwrap().id;
    let coordinator = Coordinator::with_components(
        queued.store().unwrap(),
        Box::new(MutatingInventory {
            identity: owner.clone(),
            mutation: InventoryMutation::ReplaceOnce {
                label: "other repo".to_owned(),
                repo_root: roots[1].to_string_lossy().into_owned(),
                path: "other.rs".to_owned(),
                submitted_at: 88.0,
            },
            refreshes: 0,
        }),
        probe,
        Arc::new(FakeClock::new(100.0)),
    );

    let outcome = coordinator.wait_for_repo(&owner, &roots[0], 1, 0.1, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Released);
    assert_eq!(outcome.code, 3);
    assert!(outcome.detail.is_empty());
    let replacement = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_ne!(replacement.id, original_id);
    assert_eq!(replacement.label, "other repo");
    assert_eq!(replacement.submitted_at, Some(88.0));
    assert_eq!(replacement.claims[0].repo_root, roots[1].to_string_lossy());
    assert_eq!(replacement.claims[0].scopes[0].path, "other.rs");
}

#[test]
fn wait_returns_released_when_done_wins_arbitration_without_recreating_work() {
    let owner = identity("owner");
    let (temp, roots) = repos(1);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    add_session(&mut store, &owner, &roots[0], 34, 1.0);
    probe.set(34, ProcessLiveness::Alive);
    let queued = coordinator_with_coverage(store, probe.clone(), Arc::new(AtomicUsize::new(0)), false);
    assert_eq!(
        queued.start_for(owner.clone(), "queued", &[PathBuf::from("queued.rs")], &[], &roots[0]).unwrap().kind,
        OutcomeKind::Unknown
    );
    let coordinator = Coordinator::with_components(
        queued.store().unwrap(),
        Box::new(MutatingInventory { identity: owner.clone(), mutation: InventoryMutation::DeleteOnce, refreshes: 0 }),
        probe,
        Arc::new(FakeClock::new(100.0)),
    );

    let outcome = coordinator.wait_for_repo(&owner, &roots[0], 1, 0.1, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Released);
    assert_eq!(outcome.code, 3);
    assert!(coordinator.store().unwrap().work(&owner).unwrap().is_none());
}

#[test]
fn repeated_wait_arbitration_retries_respect_the_requested_deadline() {
    let owner = identity("owner");
    let (temp, roots) = repos(1);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    add_session(&mut store, &owner, &roots[0], 35, 1.0);
    probe.set(35, ProcessLiveness::Alive);
    let queued = coordinator_with_coverage(store, probe.clone(), Arc::new(AtomicUsize::new(0)), false);
    assert_eq!(
        queued.start_for(owner.clone(), "queued", &[PathBuf::from("queued.rs")], &[], &roots[0]).unwrap().kind,
        OutcomeKind::Unknown
    );
    let submitted_at = queued.store().unwrap().work(&owner).unwrap().unwrap().submitted_at;
    let clock = Arc::new(FakeClock::new(100.0));
    let coordinator = Coordinator::with_components(
        queued.store().unwrap(),
        Box::new(MutatingInventory {
            identity: owner.clone(),
            mutation: InventoryMutation::ReviseEveryTime,
            refreshes: 0,
        }),
        probe,
        clock.clone(),
    );

    let outcome = coordinator.wait_for_repo(&owner, &roots[0], 1, 0.25, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Timeout);
    assert_eq!(outcome.code, 3);
    assert_eq!(outcome.detail, "1");
    assert_eq!(clock.monotonic(), 101.0);
    assert_eq!(coordinator.store().unwrap().work(&owner).unwrap().unwrap().submitted_at, submitted_at);
}

#[test]
fn pending_message_still_wakes_wait_without_rearbitrating() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let (_temp, roots, coordinator) = fixture(1, &[(&holder, 0, 36), (&waiter, 0, 37)]);
    let scope = [PathBuf::from("shared.rs")];
    coordinator.start_for(holder.clone(), "holder", &scope, &[], &roots[0]).unwrap();
    assert_eq!(
        coordinator.start_for(waiter.clone(), "waiter", &scope, &[], &roots[0]).unwrap().kind,
        OutcomeKind::Blocked
    );
    let before = coordinator.store().unwrap().work(&waiter).unwrap().unwrap();
    coordinator
        .store()
        .unwrap()
        .send_message(&holder, std::slice::from_ref(&waiter), "recheck", roots[0].to_str(), 100.0)
        .unwrap();

    let outcome = coordinator.wait_for_repo(&waiter, &roots[0], 1, 0.1, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Message);
    assert_eq!(outcome.code, 3);
    assert_eq!(outcome.detail, "1");
    assert_eq!(coordinator.store().unwrap().work(&waiter).unwrap().unwrap(), before);
}

#[test]
fn one_repository_work_cannot_bypass_an_earlier_queued_bundle() {
    let holder = identity("holder");
    let bundled = identity("bundled");
    let later = identity("later");
    let (_temp, roots, coordinator) = fixture(2, &[(&holder, 1, 33), (&bundled, 0, 34), (&later, 0, 35)]);
    coordinator.start_for(holder, "holder", &[PathBuf::from("shared.rs")], &[], &roots[1]).unwrap();
    let bundled_paths = files(&roots, &["queued.rs", "shared.rs"]);
    assert_eq!(
        coordinator.start_bundle_for(bundled, "bundled", &bundled_paths, &[], &roots[0]).unwrap().kind,
        OutcomeKind::Blocked
    );

    let outcome = coordinator.start_for(later, "later", &[PathBuf::from("queued.rs")], &[], &roots[0]).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Blocked);
}

#[test]
fn opposite_input_order_competes_as_one_canonical_bundle() {
    let first = identity("first");
    let second = identity("second");
    let (_temp, roots, coordinator) = fixture(2, &[(&first, 0, 40), (&second, 1, 41)]);
    let forward = files(&roots, &["same.rs", "same.rs"]);
    let reverse = vec![roots[1].join("same.rs"), roots[0].join("same.rs")];
    assert_eq!(
        coordinator.start_bundle_for(first, "first", &forward, &[], &roots[0]).unwrap().kind,
        OutcomeKind::Ready
    );
    let blocked = coordinator.start_bundle_for(second.clone(), "second", &reverse, &[], &roots[1]).unwrap();
    assert_eq!(blocked.kind, OutcomeKind::Blocked);
    let work = coordinator.store().unwrap().work(&second).unwrap().unwrap();
    assert_eq!(work.claims[0].repo_root, roots[0].to_string_lossy());
    assert_eq!(work.claims[1].repo_root, roots[1].to_string_lossy());
}

#[test]
fn blocked_active_bundle_update_is_atomic() {
    let owner = identity("owner");
    let contender = identity("contender");
    let (_temp, roots, coordinator) = fixture(2, &[(&owner, 0, 50), (&contender, 1, 51)]);
    let original = files(&roots, &["old-a.rs", "old-b.rs"]);
    coordinator.start_bundle_for(owner.clone(), "owner", &original, &[], &roots[0]).unwrap();
    coordinator.start_for(contender, "contender", &[PathBuf::from("held.rs")], &[], &roots[1]).unwrap();
    let before = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    let expanded = files(&roots, &["new-a.rs", "held.rs"]);
    let outcome = coordinator.start_bundle_for(owner.clone(), "changed", &expanded, &[], &roots[0]).unwrap();
    assert_eq!((outcome.kind, outcome.code), (OutcomeKind::Active, 3));
    assert_eq!(coordinator.store().unwrap().work(&owner).unwrap().unwrap(), before);
}

#[test]
fn bundle_done_records_per_root_residuals_and_wakes_a_waiter_once() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let (_temp, roots, coordinator) = fixture(2, &[(&holder, 0, 60), (&waiter, 1, 61)]);
    let requested = files(&roots, &["tracked.txt", "tracked.txt"]);
    coordinator.start_bundle_for(holder.clone(), "holder", &requested, &[], &roots[0]).unwrap();
    assert_eq!(
        coordinator.start_bundle_for(waiter.clone(), "waiter", &requested, &[], &roots[1]).unwrap().kind,
        OutcomeKind::Blocked
    );
    for root in &roots {
        fs::write(root.join("tracked.txt"), "dirty").unwrap();
    }

    let outcome = coordinator.done_for(&holder, &roots[1]).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Done);
    assert_eq!(outcome.holders.len(), 2);
    let store = coordinator.store().unwrap();
    assert!(store.work(&holder).unwrap().is_none());
    assert_eq!(store.inbox(&waiter, true).unwrap().len(), 1);
    for root in &roots {
        assert_eq!(store.residual_owners(root.to_str().unwrap()).unwrap()[0].path, "tracked.txt");
    }
}

#[test]
fn bundle_done_is_not_stranded_by_one_failed_repository_inspection() {
    let holder = identity("holder");
    let (_temp, roots, coordinator) = fixture(2, &[(&holder, 0, 65)]);
    let requested = files(&roots, &["a.rs", "b.rs"]);
    coordinator.start_bundle_for(holder.clone(), "holder", &requested, &[], &roots[0]).unwrap();
    fs::remove_dir_all(&roots[1]).unwrap();
    assert_eq!(coordinator.done_for(&holder, &roots[0]).unwrap().kind, OutcomeKind::Done);
    assert!(coordinator.store().unwrap().work(&holder).unwrap().is_none());
}

#[test]
fn bundle_done_requires_a_claimed_repository() {
    let holder = identity("holder");
    let (_temp, roots, coordinator) = fixture(3, &[(&holder, 0, 66)]);
    let requested = files(&roots[..2], &["a.rs", "b.rs"]);
    coordinator.start_bundle_for(holder.clone(), "holder", &requested, &[], &roots[0]).unwrap();
    let before = coordinator.store().unwrap().work(&holder).unwrap().unwrap();

    let error = coordinator.done_for(&holder, &roots[2]).unwrap_err();
    assert!(error.to_string().contains("run ai-coord done from a claimed repository"));
    assert_eq!(coordinator.store().unwrap().work(&holder).unwrap().unwrap(), before);
    assert_eq!(coordinator.done_for(&holder, &roots[1]).unwrap().kind, OutcomeKind::Done);
}

#[test]
fn baseline_selects_each_bundle_claim_and_rejects_an_unclaimed_root() {
    let owner = identity("owner");
    let (_temp, roots, coordinator) = fixture(3, &[(&owner, 0, 70)]);
    for root in &roots[..2] {
        fs::write(root.join("tracked.txt"), "base").unwrap();
        assert!(Command::new("git").args(["add", "tracked.txt"]).current_dir(root).status().unwrap().success());
        assert!(
            Command::new("git")
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-qm", "base"])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        fs::write(root.join("tracked.txt"), "changed").unwrap();
    }
    let mut store = coordinator.store().unwrap();
    for root in &roots[..2] {
        let dirty = git_dirty_paths(root).unwrap();
        store.observe_dirt(root.to_str().unwrap(), &git_blob_hashes(root, &dirty, false), 0.0).unwrap();
    }
    drop(store);
    let requested = files(&roots[..2], &["tracked.txt", "tracked.txt"]);
    assert_eq!(
        coordinator.start_bundle_for(owner.clone(), "bundle", &requested, &[], &roots[0]).unwrap().kind,
        OutcomeKind::Ready
    );
    assert_eq!(coordinator.baselines_for(&owner, &roots[0]).unwrap().len(), 1);
    assert_eq!(coordinator.baselines_for(&owner, &roots[1]).unwrap().len(), 1);
    assert!(coordinator.baselines_for(&owner, &roots[2]).unwrap_err().to_string().contains("does not claim"));
}

#[test]
fn session_end_releases_the_bundle_without_residuals_and_deduplicates_wakeup() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let (_temp, roots, coordinator) = fixture(2, &[(&holder, 0, 80), (&waiter, 0, 81)]);
    let requested = files(&roots, &["same.rs", "same.rs"]);
    coordinator.start_bundle_for(holder.clone(), "holder", &requested, &[], &roots[0]).unwrap();
    coordinator.start_bundle_for(waiter.clone(), "waiter", &requested, &[], &roots[0]).unwrap();
    for root in &roots {
        fs::write(root.join("same.rs"), "dirty").unwrap();
    }

    coordinator.end_session_for(&holder).unwrap();
    let store = coordinator.store().unwrap();
    assert!(store.work(&holder).unwrap().is_none());
    assert_eq!(store.inbox(&waiter, true).unwrap().len(), 1);
    assert!(roots.iter().all(|root| store.residual_owners(root.to_str().unwrap()).unwrap().is_empty()));
}

#[test]
fn repo_filtered_snapshot_includes_the_complete_claim_vector() {
    let owner = identity("owner");
    let (_temp, roots, coordinator) = fixture(2, &[(&owner, 1, 90)]);
    let requested = files(&roots, &["a.rs", "b.rs"]);
    coordinator.start_bundle_for(owner.clone(), "bundle", &requested, &[], &roots[1]).unwrap();

    let snapshot = coordinator.snapshot(false, &roots[0], false).unwrap();
    assert_eq!(snapshot.schema_version, 7);
    assert_eq!(snapshot.work.len(), 1);
    assert_eq!(snapshot.work[0].claims.len(), 2);
    assert!(snapshot.sessions.iter().any(|session| session.identity == owner));
}

#[test]
fn ordinary_one_claim_lifecycle_remains_current_root_scoped() {
    let owner = identity("owner");
    let (_temp, roots, coordinator) = fixture(2, &[(&owner, 0, 100)]);
    coordinator.draft_for(owner.clone(), "draft", &[PathBuf::from("one.rs")], &[], &roots[0]).unwrap();
    assert_eq!(coordinator.promote_draft_for(&owner, &roots[0]).unwrap().kind, OutcomeKind::Ready);
    let work = coordinator.store().unwrap().work(&owner).unwrap().unwrap();
    assert_eq!(work.claims.len(), 1);
    assert_eq!(coordinator.done_for(&owner, &roots[1]).unwrap().detail, "already clear");
    assert!(coordinator.store().unwrap().work(&owner).unwrap().is_some());
    assert_eq!(coordinator.done_for(&owner, &roots[0]).unwrap().detail, "released");
}

#[test]
fn process_inventory_is_cached_but_confirmed_death_still_cleans_bundle_work() {
    let owner = identity("owner");
    let (temp, roots) = repos(2);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    add_session(&mut store, &owner, &roots[0], 110, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(110, ProcessLiveness::Alive);
    let refreshes = Arc::new(AtomicUsize::new(0));
    let coordinator = coordinator(store, Arc::clone(&probe), Arc::clone(&refreshes));
    let requested = files(&roots, &["a.rs", "b.rs"]);
    coordinator.start_bundle_for(owner.clone(), "bundle", &requested, &[], &roots[0]).unwrap();
    coordinator.snapshot(true, &roots[0], true).unwrap();
    probe.set(110, ProcessLiveness::Dead);
    coordinator.snapshot(true, &roots[0], true).unwrap();
    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    assert!(coordinator.store().unwrap().work(&owner).unwrap().is_none());
}
