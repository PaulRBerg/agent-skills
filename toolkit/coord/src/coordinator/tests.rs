use std::{
    collections::HashMap,
    path::Path,
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
    domain::{Client, Identity, OutcomeKind, ProcessFingerprint, ProcessLiveness, ProcessProbe, SessionState},
    error::Result,
    state::{SessionUpdate, Store},
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
    coordinator.done_for(&holder).unwrap();
    assert_eq!(coordinator.start_for(waiter, "waiter", &scope, &[], repo.path()).unwrap().kind, OutcomeKind::Ready);
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
