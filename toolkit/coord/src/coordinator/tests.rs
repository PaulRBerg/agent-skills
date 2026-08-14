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

use super::{Clock, Coordinator, inventory::StaticInventory, normalize_callsign};
use crate::{
    domain::{
        Client, Identity, OutcomeKind, ProcessFingerprint, ProcessLiveness, ProcessProbe, SessionState, WorkState,
    },
    error::Result,
    state::{BaselineRow, SessionUpdate, Store},
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

fn repo() -> TempDir {
    let temp = TempDir::new().unwrap();
    assert!(Command::new("git").args(["init", "-q"]).current_dir(temp.path()).status().unwrap().success());
    temp
}

fn two_repos() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().unwrap();
    let first = temp.path().join("a");
    let second = temp.path().join("b");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    for root in [&first, &second] {
        assert!(Command::new("git").args(["init", "-q"]).current_dir(root).status().unwrap().success());
    }
    let first = fs::canonicalize(first).unwrap();
    let second = fs::canonicalize(second).unwrap();
    assert!(first.to_string_lossy() < second.to_string_lossy());
    (temp, first, second)
}

fn identity(id: &str) -> Identity {
    Identity { client: Client::Codex, session_id: id.to_owned() }
}

#[test]
fn callsigns_reject_terminal_control_characters() {
    assert!(normalize_callsign("🚀 trusted\u{1b}[2J").is_err());
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
            started_at: Some(current),
            current,
        })
        .unwrap();
}

fn coordinator(store: Store, probe: Arc<FakeProbe>, refreshes: Arc<AtomicUsize>) -> Coordinator {
    Coordinator::with_components(
        store,
        Box::new(StaticInventory { complete: true, refreshes }),
        probe,
        Arc::new(FakeClock::new(100.0)),
    )
}

#[test]
fn dead_holder_is_pruned_before_authorization_and_waiter_is_ready() {
    let repo = repo();
    let mut store = Store::open(repo.path().join("state.db")).unwrap();
    let holder = identity("holder");
    let waiter = identity("waiter");
    add_session(&mut store, &holder, repo.path(), 10, 1.0);
    add_session(&mut store, &waiter, repo.path(), 11, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(10, ProcessLiveness::Alive);
    probe.set(11, ProcessLiveness::Alive);
    let coordinator = coordinator(store, Arc::clone(&probe), Arc::new(AtomicUsize::new(0)));
    let file = ["src/lib.rs".into()];
    assert_eq!(
        coordinator.start_for(holder.clone(), "holder", &file, &[], repo.path()).unwrap().kind,
        OutcomeKind::Ready
    );
    probe.set(10, ProcessLiveness::Dead);
    assert_eq!(coordinator.start_for(waiter, "waiter", &file, &[], repo.path()).unwrap().kind, OutcomeKind::Ready);
    assert!(coordinator.store().unwrap().session(&holder).unwrap().is_none());
}

#[test]
fn process_unknown_fails_closed_without_deleting_the_session() {
    let repo = repo();
    let mut store = Store::open(repo.path().join("state.db")).unwrap();
    let owner = identity("owner");
    add_session(&mut store, &owner, repo.path(), 20, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(20, ProcessLiveness::Unknown);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let outcome = coordinator.start_for(owner.clone(), "work", &["src/lib.rs".into()], &[], repo.path()).unwrap();
    assert_eq!((outcome.kind, outcome.detail.as_str()), (OutcomeKind::Unknown, "coverage"));
    assert!(coordinator.store().unwrap().session(&owner).unwrap().is_some());
}

#[test]
fn provider_cache_never_caches_the_process_sweep() {
    let repo = repo();
    let mut store = Store::open(repo.path().join("state.db")).unwrap();
    let owner = identity("owner");
    add_session(&mut store, &owner, repo.path(), 30, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(30, ProcessLiveness::Alive);
    let refreshes = Arc::new(AtomicUsize::new(0));
    let coordinator = coordinator(store, Arc::clone(&probe), Arc::clone(&refreshes));
    coordinator.snapshot(true, repo.path(), true).unwrap();
    probe.set(30, ProcessLiveness::Dead);
    coordinator.snapshot(true, repo.path(), true).unwrap();
    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    assert!(coordinator.store().unwrap().session(&owner).unwrap().is_none());
}

#[test]
fn queued_reservation_promotes_after_holder_release() {
    let repo = repo();
    let mut store = Store::open(repo.path().join("state.db")).unwrap();
    let holder = identity("holder");
    let waiter = identity("waiter");
    add_session(&mut store, &holder, repo.path(), 40, 1.0);
    add_session(&mut store, &waiter, repo.path(), 41, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(40, ProcessLiveness::Alive);
    probe.set(41, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];
    assert_eq!(
        coordinator.start_for(holder.clone(), "holder", &scope, &[], repo.path()).unwrap().kind,
        OutcomeKind::Ready
    );
    assert_eq!(
        coordinator.start_for(waiter.clone(), "waiter", &scope, &[], repo.path()).unwrap().kind,
        OutcomeKind::Blocked
    );
    coordinator.done_for(&holder, repo.path()).unwrap();
    assert_eq!(coordinator.start_for(waiter, "waiter", &scope, &[], repo.path()).unwrap().kind, OutcomeKind::Ready);
}

#[test]
fn one_identity_retains_two_roots_and_replaces_and_releases_only_the_current_root() {
    let (_temp, first, second) = two_repos();
    let mut store = Store::open(first.parent().unwrap().join("state.db")).unwrap();
    let owner = identity("owner");
    add_session(&mut store, &owner, &first, 70, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(70, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];

    assert_eq!(coordinator.start_for(owner.clone(), "first", &scope, &[], &first).unwrap().kind, OutcomeKind::Ready);
    assert_eq!(coordinator.start_for(owner.clone(), "second", &scope, &[], &second).unwrap().kind, OutcomeKind::Ready);
    assert_eq!(
        coordinator.start_for(owner.clone(), "second updated", &scope, &[], &second).unwrap().kind,
        OutcomeKind::Ready
    );

    let store = coordinator.store().unwrap();
    assert_eq!(store.works_for_identity(&owner).unwrap().len(), 2);
    assert_eq!(store.work_in_repo(&owner, first.to_str().unwrap()).unwrap().unwrap().label, "first");
    assert_eq!(store.work_in_repo(&owner, second.to_str().unwrap()).unwrap().unwrap().label, "second updated");
    drop(store);

    coordinator.done_for(&owner, &second).unwrap();
    let store = coordinator.store().unwrap();
    assert!(store.work_in_repo(&owner, second.to_str().unwrap()).unwrap().is_none());
    assert_eq!(store.work_in_repo(&owner, first.to_str().unwrap()).unwrap().unwrap().label, "first");
}

#[test]
fn wait_promotes_only_the_current_root_and_retains_other_active_work() {
    let (_temp, first, second) = two_repos();
    let mut store = Store::open(first.parent().unwrap().join("state.db")).unwrap();
    let owner = identity("owner");
    let holder = identity("holder");
    add_session(&mut store, &owner, &first, 80, 1.0);
    add_session(&mut store, &holder, &second, 81, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(80, ProcessLiveness::Alive);
    probe.set(81, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];

    assert_eq!(coordinator.start_for(owner.clone(), "first", &scope, &[], &first).unwrap().kind, OutcomeKind::Ready);
    assert_eq!(coordinator.start_for(holder.clone(), "holder", &scope, &[], &second).unwrap().kind, OutcomeKind::Ready);
    assert_eq!(
        coordinator.start_for(owner.clone(), "second", &scope, &[], &second).unwrap().kind,
        OutcomeKind::Blocked
    );
    coordinator.done_for(&holder, &second).unwrap();
    let mut store = coordinator.store().unwrap();
    store.acknowledge(&owner, None, 100.0).unwrap();
    store
        .send_message(&holder, std::slice::from_ref(&owner), "other root", Some(first.to_str().unwrap()), 100.0)
        .unwrap();
    drop(store);

    assert_eq!(coordinator.wait_for_repo(&owner, &second, 1, 0.1, false).unwrap().kind, OutcomeKind::Ready);
    let store = coordinator.store().unwrap();
    assert_eq!(store.work_in_repo(&owner, first.to_str().unwrap()).unwrap().unwrap().state, WorkState::Active);
    assert_eq!(store.work_in_repo(&owner, second.to_str().unwrap()).unwrap().unwrap().state, WorkState::Active);
}

#[test]
fn earlier_direct_start_and_draft_promotion_reject_without_ledger_mutation() {
    let (_temp, first, second) = two_repos();
    let mut store = Store::open(first.parent().unwrap().join("state.db")).unwrap();
    let direct = identity("direct");
    let drafted = identity("drafted");
    add_session(&mut store, &direct, &second, 90, 1.0);
    add_session(&mut store, &drafted, &second, 91, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(90, ProcessLiveness::Alive);
    probe.set(91, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];

    coordinator.start_for(direct.clone(), "later", &scope, &[], &second).unwrap();
    let before = coordinator.store().unwrap().works_for_identity(&direct).unwrap();
    let generation = coordinator.store().unwrap().generation().unwrap();
    let error = coordinator.start_for(direct.clone(), "earlier", &scope, &[], &first).unwrap_err();
    assert!(error.to_string().contains(second.to_str().unwrap()));
    assert_eq!(coordinator.store().unwrap().generation().unwrap(), generation);
    assert_eq!(coordinator.store().unwrap().works_for_identity(&direct).unwrap(), before);

    coordinator.start_for(drafted.clone(), "later", &scope, &[], &second).unwrap();
    coordinator.draft_for(drafted.clone(), "earlier draft", &scope, &[], &first).unwrap();
    let before = coordinator.store().unwrap().works_for_identity(&drafted).unwrap();
    let generation = coordinator.store().unwrap().generation().unwrap();
    let error = coordinator.promote_draft_for(&drafted, &first).unwrap_err();
    assert!(error.to_string().contains(second.to_str().unwrap()));
    assert_eq!(coordinator.store().unwrap().generation().unwrap(), generation);
    assert_eq!(coordinator.store().unwrap().works_for_identity(&drafted).unwrap(), before);
    assert_eq!(
        coordinator.store().unwrap().work_in_repo(&drafted, first.to_str().unwrap()).unwrap().unwrap().state,
        WorkState::Draft
    );
}

#[test]
fn baselines_and_all_root_residuals_are_repository_local_for_equal_relative_paths() {
    let (_temp, first, second) = two_repos();
    let mut store = Store::open(first.parent().unwrap().join("state.db")).unwrap();
    let owner = identity("owner");
    let waiter = identity("waiter");
    add_session(&mut store, &owner, &first, 100, 1.0);
    add_session(&mut store, &waiter, &first, 101, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(100, ProcessLiveness::Alive);
    probe.set(101, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];
    coordinator.start_for(owner.clone(), "first", &scope, &[], &first).unwrap();
    coordinator.start_for(owner.clone(), "second", &scope, &[], &second).unwrap();
    assert_eq!(
        coordinator.start_for(waiter.clone(), "first waiter", &scope, &[], &first).unwrap().kind,
        OutcomeKind::Blocked
    );
    assert_eq!(
        coordinator.start_for(waiter.clone(), "second waiter", &scope, &[], &second).unwrap().kind,
        OutcomeKind::Blocked
    );
    let first_baseline = BaselineRow { path: "src/lib.rs".to_owned(), oid: "first".to_owned() };
    let second_baseline = BaselineRow { path: "src/lib.rs".to_owned(), oid: "second".to_owned() };
    let mut store = coordinator.store().unwrap();
    store.replace_baselines_in_repo(&owner, first.to_str().unwrap(), std::slice::from_ref(&first_baseline)).unwrap();
    store.replace_baselines_in_repo(&owner, second.to_str().unwrap(), std::slice::from_ref(&second_baseline)).unwrap();
    drop(store);
    assert_eq!(coordinator.baselines_for(&owner, &first).unwrap(), [first_baseline]);
    assert_eq!(coordinator.baselines_for(&owner, &second).unwrap(), [second_baseline]);

    for root in [&first, &second] {
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), root.to_string_lossy().as_bytes()).unwrap();
    }
    coordinator.done_all_for(&owner).unwrap();
    let store = coordinator.store().unwrap();
    assert!(store.works_for_identity(&owner).unwrap().is_empty());
    assert_eq!(store.residual_owners(first.to_str().unwrap()).unwrap()[0].path, "src/lib.rs");
    assert_eq!(store.residual_owners(second.to_str().unwrap()).unwrap()[0].path, "src/lib.rs");
    let roots = store
        .inbox(&waiter, true)
        .unwrap()
        .into_iter()
        .filter_map(|message| message.repo_root)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(roots, [first.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()].into());
}

#[test]
fn confirmed_death_releases_and_wakes_waiters_in_every_owned_root_without_residuals() {
    let (_temp, first, second) = two_repos();
    let mut store = Store::open(first.parent().unwrap().join("state.db")).unwrap();
    let holder = identity("holder");
    let waiter = identity("waiter");
    add_session(&mut store, &holder, &first, 110, 1.0);
    add_session(&mut store, &waiter, &first, 111, 1.0);
    let probe = Arc::new(FakeProbe::default());
    probe.set(110, ProcessLiveness::Alive);
    probe.set(111, ProcessLiveness::Alive);
    let coordinator = coordinator(store, Arc::clone(&probe), Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];
    coordinator.start_for(holder.clone(), "first holder", &scope, &[], &first).unwrap();
    coordinator.start_for(holder.clone(), "second holder", &scope, &[], &second).unwrap();
    assert_eq!(
        coordinator.start_for(waiter.clone(), "first waiter", &scope, &[], &first).unwrap().kind,
        OutcomeKind::Blocked
    );
    assert_eq!(
        coordinator.start_for(waiter.clone(), "second waiter", &scope, &[], &second).unwrap().kind,
        OutcomeKind::Blocked
    );

    probe.set(110, ProcessLiveness::Dead);
    coordinator.generation_with_reconcile().unwrap();
    let store = coordinator.store().unwrap();
    assert!(store.works_for_identity(&holder).unwrap().is_empty());
    let roots = store
        .inbox(&waiter, true)
        .unwrap()
        .into_iter()
        .filter_map(|message| message.repo_root)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(roots, [first.to_string_lossy().into_owned(), second.to_string_lossy().into_owned()].into());
    assert!(store.residual_owners(first.to_str().unwrap()).unwrap().is_empty());
    assert!(store.residual_owners(second.to_str().unwrap()).unwrap().is_empty());
}

#[test]
fn blocker_details_prefer_the_holders_callsign() {
    let repo = repo();
    let mut store = Store::open(repo.path().join("state.db")).unwrap();
    let holder = identity("holder");
    let waiter = identity("waiter");
    add_session(&mut store, &holder, repo.path(), 50, 1.0);
    add_session(&mut store, &waiter, repo.path(), 51, 1.0);
    store.set_session_callsign(&holder, "🧱 Brick Boss").unwrap();
    let probe = Arc::new(FakeProbe::default());
    probe.set(50, ProcessLiveness::Alive);
    probe.set(51, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];
    coordinator.start_for(holder, "holder", &scope, &[], repo.path()).unwrap();
    let blocked = coordinator.start_for(waiter, "waiter", &scope, &[], repo.path()).unwrap();
    assert_eq!(blocked.detail, "🧱 Brick Boss");
    assert_eq!(blocked.holders, ["🧱 Brick Boss"]);
}

#[test]
fn draft_and_all_start_paths_clear_waivers_without_releasing_existing_work() {
    let repo = repo();
    let mut store = Store::open(repo.path().join("state.db")).unwrap();
    let owner = identity("owner");
    let empty = identity("empty");
    add_session(&mut store, &owner, repo.path(), 60, 1.0);
    add_session(&mut store, &empty, repo.path(), 61, 1.0);
    store.set_coordination_waived(&owner, true).unwrap();
    store.set_coordination_waived(&empty, true).unwrap();
    let probe = Arc::new(FakeProbe::default());
    probe.set(60, ProcessLiveness::Alive);
    probe.set(61, ProcessLiveness::Alive);
    let coordinator = coordinator(store, probe, Arc::new(AtomicUsize::new(0)));
    let scope = ["src/lib.rs".into()];

    coordinator.draft_for(owner.clone(), "planned", &scope, &[], repo.path()).unwrap();
    let store = coordinator.store().unwrap();
    assert!(!store.session(&owner).unwrap().unwrap().coordination_waived);
    assert_eq!(store.works_for_identity(&owner).unwrap()[0].state, WorkState::Draft);
    drop(store);

    coordinator.store().unwrap().set_coordination_waived(&owner, true).unwrap();
    assert!(coordinator.start_for(owner.clone(), "direct", &scope, &[], repo.path()).is_err());
    let store = coordinator.store().unwrap();
    assert!(!store.session(&owner).unwrap().unwrap().coordination_waived);
    assert_eq!(store.works_for_identity(&owner).unwrap()[0].state, WorkState::Draft);
    drop(store);

    coordinator.store().unwrap().set_coordination_waived(&owner, true).unwrap();
    assert_eq!(coordinator.promote_draft_for(&owner, repo.path()).unwrap().kind, OutcomeKind::Ready);
    let store = coordinator.store().unwrap();
    assert!(!store.session(&owner).unwrap().unwrap().coordination_waived);
    assert_eq!(store.works_for_identity(&owner).unwrap()[0].state, WorkState::Active);
    drop(store);

    coordinator.store().unwrap().set_coordination_waived(&owner, true).unwrap();
    assert!(coordinator.draft_for(owner.clone(), "blocked", &scope, &[], repo.path()).is_err());
    let store = coordinator.store().unwrap();
    assert!(!store.session(&owner).unwrap().unwrap().coordination_waived);
    assert_eq!(store.works_for_identity(&owner).unwrap()[0].state, WorkState::Active);
    drop(store);

    assert!(coordinator.promote_draft_for(&empty, repo.path()).is_err());
    assert!(!coordinator.store().unwrap().session(&empty).unwrap().unwrap().coordination_waived);
}
