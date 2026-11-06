//! Repeated-recheck cost and noise: the full-refresh cadence of a dirty-settling
//! wait, prompt wakes while it settles, and repeated contender notifications.

use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tempfile::TempDir;

use super::{
    Clock, Coordinator, FakeClock, FakeProbe, InventoryObservation, ProviderInventory, add_session, files, identity,
    repos,
};
use crate::{
    domain::{Identity, InventoryResult, OutcomeKind, ProcessLiveness, ProcessProbe, Scope, ScopeKind, WorkState},
    error::Result,
    state::{Store, WorkClaimUpdate, WorkUpdate},
    work::DIRT_HOLD_SECONDS,
};

type Action = Box<dyn FnOnce() + Send>;

/// A fake clock that runs scripted actions once its time reaches their trigger,
/// modeling concurrent ledger or process changes while a wait sleeps.
struct ScriptedClock {
    inner: FakeClock,
    actions: Mutex<Vec<(f64, Action)>>,
}

impl ScriptedClock {
    fn new(value: f64) -> Self {
        Self { inner: FakeClock::new(value), actions: Mutex::new(Vec::new()) }
    }

    fn at(&self, time: f64, action: impl FnOnce() + Send + 'static) {
        self.actions.lock().unwrap().push((time, Box::new(action)));
    }

    fn set(&self, value: f64) {
        *self.inner.value.lock().unwrap() = value;
    }
}

impl Clock for ScriptedClock {
    fn wall(&self) -> f64 {
        self.inner.wall()
    }
    fn monotonic(&self) -> f64 {
        self.inner.wall()
    }
    fn sleep(&self, duration: Duration) {
        self.inner.sleep(duration);
        let now = self.inner.wall();
        let due = {
            let mut actions = self.actions.lock().unwrap();
            let (due, pending): (Vec<_>, Vec<_>) = actions.drain(..).partition(|(at, _)| *at <= now);
            *actions = pending;
            due
        };
        for (_, action) in due {
            action();
        }
    }
}

/// Records the monotonic time of every fresh provider refresh. Coverage can be
/// flipped while a wait is in flight.
struct RecordingInventory {
    clock: Arc<ScriptedClock>,
    complete: Arc<AtomicBool>,
    refreshed_at: Arc<Mutex<Vec<f64>>>,
}

impl ProviderInventory for RecordingInventory {
    fn cache_key(&self) -> &str {
        "recording"
    }

    fn refresh(&mut self, _store: &Store, _probe: &dyn ProcessProbe) -> Result<InventoryObservation> {
        self.refreshed_at.lock().unwrap().push(self.clock.monotonic());
        Ok(InventoryObservation {
            result: InventoryResult { complete: self.complete.load(Ordering::SeqCst), providers: Vec::new() },
            claude_sessions: Vec::new(),
            claude_authoritative: false,
        })
    }
}

struct Fixture {
    _temp: TempDir,
    roots: Vec<PathBuf>,
    coordinator: Coordinator,
    clock: Arc<ScriptedClock>,
    probe: Arc<FakeProbe>,
    complete: Arc<AtomicBool>,
    refreshed_at: Arc<Mutex<Vec<f64>>>,
}

impl Fixture {
    /// Refresh times observed after the first `skip` refreshes.
    fn refreshes_after(&self, skip: usize) -> Vec<f64> {
        self.refreshed_at.lock().unwrap()[skip..].to_vec()
    }

    fn store(&self) -> Store {
        self.coordinator.store().unwrap()
    }
}

fn fixture(count: usize, sessions: &[(&Identity, usize, u32)]) -> Fixture {
    let (temp, roots) = repos(count);
    let mut store = Store::open(temp.path().join("state.db")).unwrap();
    let probe = Arc::new(FakeProbe::default());
    for (identity, root, pid) in sessions {
        add_session(&mut store, identity, &roots[*root], *pid, 1.0);
        probe.set(*pid, ProcessLiveness::Alive);
    }
    let clock = Arc::new(ScriptedClock::new(100.0));
    let complete = Arc::new(AtomicBool::new(true));
    let refreshed_at = Arc::new(Mutex::new(Vec::new()));
    let coordinator = Coordinator::with_components(
        store,
        Box::new(RecordingInventory {
            clock: Arc::clone(&clock),
            complete: Arc::clone(&complete),
            refreshed_at: Arc::clone(&refreshed_at),
        }),
        Arc::clone(&probe) as Arc<dyn ProcessProbe>,
        Arc::clone(&clock) as Arc<dyn Clock>,
    );
    Fixture { _temp: temp, roots, coordinator, clock, probe, complete, refreshed_at }
}

/// Queue `waiter` behind fresh unattributed dirt on `dirty.rs` at t=100, so the
/// hold settles at t=100+DIRT_HOLD_SECONDS.
fn queue_behind_fresh_dirt(fixture: &Fixture, waiter: &Identity, extra: &[&str]) {
    let root = &fixture.roots[0];
    fs::write(root.join("dirty.rs"), "dirty\n").unwrap();
    let mut requested = vec![PathBuf::from("dirty.rs")];
    requested.extend(extra.iter().map(PathBuf::from));
    let outcome = fixture.coordinator.start_for(waiter.clone(), "waiter", &requested, &[], root).unwrap();
    assert_eq!((outcome.kind, outcome.detail.as_str()), (OutcomeKind::Unknown, "dirty-settling:dirty.rs"));
    assert_eq!(outcome.settles_in, Some(DIRT_HOLD_SECONDS as u64));
    assert_eq!(fixture.refreshes_after(0), [100.0]);
}

#[test]
fn settling_wait_refreshes_on_the_settle_schedule_and_grants_exactly_when_the_hold_ends() {
    let waiter = identity("waiter");
    let fixture = fixture(1, &[(&waiter, 0, 300)]);
    queue_behind_fresh_dirt(&fixture, &waiter, &[]);

    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 300, 1.0, false).unwrap();

    assert_eq!((outcome.kind, outcome.detail.as_str()), (OutcomeKind::Ready, "stale-dirt:dirty.rs"));
    assert_eq!(fixture.clock.monotonic(), 100.0 + DIRT_HOLD_SECONDS, "never granted before the hold ends");
    // Six full refreshes over the 90 s hold instead of one per second: the
    // fallback interval while the hold is long, then the exact remainder.
    assert_eq!(fixture.refreshes_after(1), [100.0, 120.0, 140.0, 160.0, 180.0, 190.0]);
    assert_eq!(fixture.store().work(&waiter).unwrap().unwrap().state, WorkState::Active);
}

#[test]
fn ledger_generation_change_refreshes_a_settling_wait_immediately() {
    let waiter = identity("waiter");
    let fixture = fixture(1, &[(&waiter, 0, 301)]);
    queue_behind_fresh_dirt(&fixture, &waiter, &[]);
    let store_path = fixture.store().path().to_owned();
    let root = fixture.roots[0].clone();
    fixture.probe.set(302, ProcessLiveness::Alive);
    // Another session submits unrelated work: a coordination-state change.
    fixture.clock.at(105.0, move || {
        let newcomer = identity("newcomer");
        let mut store = Store::open(store_path).unwrap();
        add_session(&mut store, &newcomer, &root, 302, 105.0);
        store
            .with_work_transaction(|transaction| {
                transaction.save_work(&WorkUpdate {
                    identity: newcomer.clone(),
                    label: "newcomer".to_owned(),
                    state: WorkState::Active,
                    blocked_reason: None,
                    claims: vec![WorkClaimUpdate {
                        repo_root: root.to_string_lossy().into_owned(),
                        blocked_reason: None,
                        scopes: vec![Scope { path: "unrelated.rs".to_owned(), kind: ScopeKind::Exact }],
                        baselines: None,
                    }],
                    submitted_at: Some(105.0),
                    updated_at: 105.0,
                    expected_revision: None,
                })?;
                Ok(())
            })
            .unwrap();
    });

    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 30, 1.0, false).unwrap();

    assert_eq!(outcome.kind, OutcomeKind::Timeout);
    assert_eq!(fixture.refreshes_after(1), [100.0, 105.0, 125.0]);
    assert_eq!(fixture.store().work(&waiter).unwrap().unwrap().state, WorkState::Queued);
}

#[test]
fn pending_message_wakes_a_settling_wait_without_a_refresh() {
    let waiter = identity("waiter");
    let peer = identity("peer");
    let fixture = fixture(1, &[(&waiter, 0, 303), (&peer, 0, 304)]);
    queue_behind_fresh_dirt(&fixture, &waiter, &[]);
    let store_path = fixture.store().path().to_owned();
    let root = fixture.roots[0].to_str().unwrap().to_owned();
    let recipient = waiter.clone();
    fixture.clock.at(105.0, move || {
        Store::open(store_path)
            .unwrap()
            .send_message(&identity("peer"), std::slice::from_ref(&recipient), "heads up", Some(&root), 105.0)
            .unwrap();
    });

    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 30, 1.0, false).unwrap();

    assert_eq!((outcome.kind, outcome.detail.as_str()), (OutcomeKind::Message, "1"));
    assert_eq!(fixture.clock.monotonic(), 105.0);
    assert_eq!(fixture.refreshes_after(1), [100.0]);
}

#[test]
fn holder_death_wakes_a_settling_wait_promptly() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let fixture = fixture(1, &[(&holder, 0, 305), (&waiter, 0, 306)]);
    let held = [PathBuf::from("held.rs")];
    assert_eq!(
        fixture.coordinator.start_for(holder.clone(), "holder", &held, &[], &fixture.roots[0]).unwrap().kind,
        OutcomeKind::Ready
    );
    fixture.refreshed_at.lock().unwrap().clear();
    queue_behind_fresh_dirt(&fixture, &waiter, &["held.rs"]);
    let probe = Arc::clone(&fixture.probe);
    fixture.clock.at(105.0, move || probe.set(305, ProcessLiveness::Dead));

    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 30, 1.0, false).unwrap();

    assert_eq!((outcome.kind, outcome.detail.as_str()), (OutcomeKind::Message, "1"));
    assert_eq!(fixture.clock.monotonic(), 105.0);
    assert_eq!(fixture.refreshes_after(1), [100.0]);
    let store = fixture.store();
    assert!(store.work(&holder).unwrap().is_none());
    assert!(store.inbox(&waiter, true).unwrap()[0].text.starts_with("Session ended; released work 'holder'"));
}

#[test]
fn incomplete_coverage_at_the_final_recheck_never_grants() {
    let waiter = identity("waiter");
    let fixture = fixture(1, &[(&waiter, 0, 307)]);
    queue_behind_fresh_dirt(&fixture, &waiter, &[]);
    let complete = Arc::clone(&fixture.complete);
    fixture.clock.at(185.0, move || complete.store(false, Ordering::SeqCst));

    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 300, 1.0, false).unwrap();

    assert_eq!((outcome.kind, outcome.code, outcome.detail.as_str()), (OutcomeKind::Unknown, 2, "coverage"));
    assert_eq!(fixture.clock.monotonic(), 100.0 + DIRT_HOLD_SECONDS);
    assert_eq!(fixture.refreshes_after(1), [100.0, 120.0, 140.0, 160.0, 180.0, 190.0]);
    let work = fixture.store().work(&waiter).unwrap().unwrap();
    assert_eq!((work.state, work.blocked_reason.as_deref()), (WorkState::Queued, Some("coverage")));
}

/// A holder active on `roots[1]/shared.rs` and a bundle waiter whose first claim
/// is blocked by fresh dirt in `roots[0]` while its second overlaps the holder.
fn mixed_blocker_bundle(fixture: &Fixture, holder: &Identity, waiter: &Identity) {
    let shared = [PathBuf::from("shared.rs")];
    assert_eq!(
        fixture.coordinator.start_for(holder.clone(), "holder", &shared, &[], &fixture.roots[1]).unwrap().kind,
        OutcomeKind::Ready
    );
    fixture.refreshed_at.lock().unwrap().clear();
    fs::write(fixture.roots[0].join("dirty.rs"), "dirty\n").unwrap();
    let requested = files(&fixture.roots, &["dirty.rs", "shared.rs"]);
    let outcome =
        fixture.coordinator.start_bundle_for(waiter.clone(), "waiter", &requested, &[], &fixture.roots[0]).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Unknown);
    assert!(outcome.detail.starts_with("dirty-settling:"), "{}", outcome.detail);
    let work = fixture.store().work(waiter).unwrap().unwrap();
    assert_eq!(work.blocked_reason.as_deref(), Some("dirty"));
    assert_eq!(work.claim(fixture.roots[1].to_str().unwrap()).unwrap().blocked_reason.as_deref(), Some("overlap"));
}

fn queued_behind_messages(store: &Store, holder: &Identity) -> Vec<String> {
    store
        .inbox(holder, true)
        .unwrap()
        .into_iter()
        .map(|message| message.text)
        .filter(|text| text.starts_with("Queued behind your work: waiter"))
        .collect()
}

#[test]
fn repeated_rechecks_of_a_mixed_blocker_bundle_notify_the_overlap_holder_once() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let fixture = fixture(2, &[(&holder, 1, 310), (&waiter, 0, 311)]);
    mixed_blocker_bundle(&fixture, &holder, &waiter);
    let messages = queued_behind_messages(&fixture.store(), &holder);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains(&format!("overlaps: {}/shared.rs", fixture.roots[1].display())), "{}", messages[0]);

    // Rechecks while the dirt settles keep the holder's claim overlap-blocked.
    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 45, 1.0, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Timeout);
    assert_eq!(fixture.refreshes_after(1), [100.0, 120.0, 140.0]);
    assert_eq!(queued_behind_messages(&fixture.store(), &holder).len(), 1);

    // Once the dirt settles the item itself becomes overlap-blocked; that
    // transition is not a new overlap for the holder either.
    fixture.clock.set(100.0 + DIRT_HOLD_SECONDS);
    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 1, 1.0, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Timeout);
    let store = fixture.store();
    assert_eq!(store.work(&waiter).unwrap().unwrap().blocked_reason.as_deref(), Some("overlap"));
    assert_eq!(queued_behind_messages(&store, &holder).len(), 1);
}

#[test]
fn changed_scopes_notify_the_overlap_holder_again() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let fixture = fixture(2, &[(&holder, 1, 312), (&waiter, 0, 313)]);
    mixed_blocker_bundle(&fixture, &holder, &waiter);

    let changed = files(&fixture.roots, &["other.rs", "shared.rs"]);
    let outcome =
        fixture.coordinator.start_bundle_for(waiter.clone(), "waiter", &changed, &[], &fixture.roots[0]).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Blocked);
    assert_eq!(queued_behind_messages(&fixture.store(), &holder).len(), 2);

    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 1, 1.0, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Timeout);
    assert_eq!(queued_behind_messages(&fixture.store(), &holder).len(), 2);
}

#[test]
fn a_claim_that_becomes_overlap_blocked_notifies_its_holder_once() {
    let holder = identity("holder");
    let waiter = identity("waiter");
    let fixture = fixture(2, &[(&holder, 1, 314), (&waiter, 0, 315)]);
    let shared = [PathBuf::from("shared.rs")];
    assert_eq!(
        fixture.coordinator.start_for(holder.clone(), "holder", &shared, &[], &fixture.roots[1]).unwrap().kind,
        OutcomeKind::Ready
    );
    fixture.refreshed_at.lock().unwrap().clear();
    fs::write(fixture.roots[0].join("dirty.rs"), "dirty\n").unwrap();
    fixture.complete.store(false, Ordering::SeqCst);
    let requested = files(&fixture.roots, &["dirty.rs", "shared.rs"]);
    let outcome =
        fixture.coordinator.start_bundle_for(waiter.clone(), "waiter", &requested, &[], &fixture.roots[0]).unwrap();
    assert_eq!((outcome.kind, outcome.detail.as_str()), (OutcomeKind::Unknown, "coverage"));
    assert!(queued_behind_messages(&fixture.store(), &holder).is_empty());

    // Coverage returns: the holder's claim is overlap-blocked for the first time.
    fixture.complete.store(true, Ordering::SeqCst);
    let outcome = fixture.coordinator.wait_for_repo(&waiter, &fixture.roots[0], 25, 1.0, false).unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Timeout);
    assert_eq!(fixture.refreshes_after(1), [100.0, 120.0]);
    let store = fixture.store();
    let work = store.work(&waiter).unwrap().unwrap();
    assert_eq!(work.blocked_reason.as_deref(), Some("dirty"));
    assert_eq!(work.claim(fixture.roots[1].to_str().unwrap()).unwrap().blocked_reason.as_deref(), Some("overlap"));
    assert_eq!(queued_behind_messages(&store, &holder).len(), 1);
}
