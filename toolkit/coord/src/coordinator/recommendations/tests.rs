use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, atomic::AtomicUsize},
    time::Duration,
};

use tempfile::TempDir;

use super::*;
use crate::{
    coordinator::{Clock, inventory::StaticInventory},
    domain::{Client, ProcessFingerprint, ProcessProbe, Scope, ScopeKind, SessionState, WorkState},
    error::{ErrorKind, Result},
    state::{
        DraftClaimUpdate, DraftOwner, RecommendationOutcome, RecommendationState, SessionUpdate, WorkClaimUpdate,
        WorkUpdate,
    },
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

struct FixedClock;

impl Clock for FixedClock {
    fn wall(&self) -> f64 {
        1_000.0
    }

    fn monotonic(&self) -> f64 {
        1_000.0
    }

    fn sleep(&self, _duration: Duration) {}
}

struct Fixture {
    _temporary: TempDir,
    roots: Vec<PathBuf>,
    coordinator: Coordinator,
    probe: Arc<FakeProbe>,
    sender: Identity,
    recipient: Identity,
}

impl Fixture {
    fn new() -> Self {
        let temporary = TempDir::new().unwrap();
        let roots = (0..2)
            .map(|index| {
                let root = temporary.path().join(format!("repo-{index}"));
                fs::create_dir_all(root.join("src")).unwrap();
                assert!(Command::new("git").args(["init", "-q"]).current_dir(&root).status().unwrap().success());
                fs::canonicalize(root).unwrap()
            })
            .collect::<Vec<_>>();
        let mut store = Store::open(temporary.path().join("state.db")).unwrap();
        let sender = identity("sender");
        let recipient = identity("recipient");
        register(&mut store, &sender, &roots[0], 10, SessionState::Idle);
        register(&mut store, &recipient, &roots[0], 11, SessionState::Idle);
        save_work(
            &mut store,
            &sender,
            "Source replacement",
            WorkState::Queued,
            vec![claim(&roots[0], vec![scope("src/legacy.rs", false)])],
        );
        save_work(
            &mut store,
            &recipient,
            "Recipient polish",
            WorkState::Active,
            vec![claim(&roots[0], vec![scope("src/legacy.rs", false), scope("src/independent.rs", false)])],
        );
        let probe = Arc::new(FakeProbe::default());
        probe.set(10, ProcessLiveness::Alive);
        probe.set(11, ProcessLiveness::Alive);
        let coordinator = Coordinator::with_components(
            store,
            Box::new(StaticInventory { complete: true, refreshes: Arc::new(AtomicUsize::new(0)) }),
            probe.clone(),
            Arc::new(FixedClock),
        );
        Self { _temporary: temporary, roots, coordinator, probe, sender, recipient }
    }

    fn send(&self, reason: &str, replacement: &str) -> RecommendationMutation {
        self.coordinator
            .send_recommendation_for(
                &self.sender,
                &self.recipient.session_id,
                RecommendationAction::Defer,
                &[PathBuf::from("src/legacy.rs")],
                &[],
                reason,
                replacement,
                &self.roots[0],
            )
            .unwrap()
    }
}

fn identity(session_id: &str) -> Identity {
    Identity { client: Client::Codex, session_id: session_id.to_owned() }
}

fn scope(path: &str, recursive: bool) -> Scope {
    Scope { path: path.to_owned(), kind: if recursive { ScopeKind::Recursive } else { ScopeKind::Exact } }
}

fn claim(root: &Path, scopes: Vec<Scope>) -> WorkClaimUpdate {
    WorkClaimUpdate { repo_root: root.to_string_lossy().into_owned(), blocked_reason: None, scopes, baselines: None }
}

fn register(store: &mut Store, identity: &Identity, root: &Path, pid: u32, state: SessionState) {
    store
        .upsert_session(&SessionUpdate {
            identity: identity.clone(),
            cwd: root.to_string_lossy().into_owned(),
            repo_root: Some(root.to_string_lossy().into_owned()),
            state,
            source: "test".to_owned(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(ProcessFingerprint { pid, start_token: Some(format!("token-{pid}")) }),
            transcript_path: None,
            started_at: Some(1.0),
            current: 1.0,
        })
        .unwrap();
}

fn save_work(store: &mut Store, identity: &Identity, label: &str, state: WorkState, claims: Vec<WorkClaimUpdate>) {
    let blocked = (state == WorkState::Queued).then(|| "holder".to_owned());
    let claims = claims
        .into_iter()
        .map(|mut claim| {
            claim.blocked_reason.clone_from(&blocked);
            claim
        })
        .collect();
    store
        .with_work_transaction(|transaction| {
            transaction.save_work(&WorkUpdate {
                identity: identity.clone(),
                label: label.to_owned(),
                state,
                blocked_reason: blocked,
                claims,
                submitted_at: Some(1.0),
                updated_at: 1.0,
                expected_revision: None,
            })?;
            Ok(())
        })
        .unwrap();
}

#[test]
fn send_preserves_long_normalized_evidence_bundle_context_and_ownership() {
    let fixture = Fixture::new();
    let mut store = fixture.coordinator.store().unwrap();
    for owner in [&fixture.sender, &fixture.recipient] {
        let current = store.work(owner).unwrap().unwrap();
        let mut claims = current
            .claims
            .iter()
            .map(|claim| WorkClaimUpdate {
                repo_root: claim.repo_root.clone(),
                blocked_reason: claim.blocked_reason.clone(),
                scopes: claim.scopes.clone(),
                baselines: None,
            })
            .collect::<Vec<_>>();
        if owner == &fixture.recipient {
            claims[0].scopes.push(scope("src/generated", true));
        }
        claims.push(claim(&fixture.roots[1], vec![scope("other.rs", false)]));
        let update = WorkUpdate {
            identity: owner.clone(),
            label: current.label,
            state: current.state,
            blocked_reason: current.blocked_reason,
            claims,
            submitted_at: current.submitted_at,
            updated_at: 2.0,
            expected_revision: Some(current.revision),
        };
        store.with_work_transaction(|transaction| transaction.save_work(&update).map(|_| ())).unwrap();
    }
    let sender_before = store.work(&fixture.sender).unwrap().unwrap();
    let recipient_before = store.work(&fixture.recipient).unwrap().unwrap();
    drop(store);
    let reason = "Detailed evidence ".repeat(40);

    let mutation = fixture
        .coordinator
        .send_recommendation_for(
            &fixture.sender,
            &fixture.recipient.session_id,
            RecommendationAction::Omit,
            &[PathBuf::from("src/legacy.rs")],
            &[PathBuf::from("src/generated")],
            &reason,
            "  Replace\tlegacy\ninputs\u{1b} and verify.  ",
            &fixture.roots[0],
        )
        .unwrap();

    assert_eq!(mutation.outcome, RecommendationOutcome::Recommended);
    assert!(mutation.recommendation.reason.chars().count() > 240);
    assert_eq!(mutation.recommendation.replacement, "Replace legacy inputs and verify.");
    assert_eq!(mutation.recommendation.sender.work.claims.len(), 2);
    assert_eq!(mutation.recommendation.recipient.work.claims.len(), 2);
    assert_eq!(mutation.recommendation.scopes.len(), 2);
    let store = fixture.coordinator.store().unwrap();
    assert_eq!(store.work(&fixture.sender).unwrap().unwrap(), sender_before);
    assert_eq!(store.work(&fixture.recipient).unwrap().unwrap(), recipient_before);
    assert_eq!(
        fixture
            .coordinator
            .list_recommendations_for(&fixture.recipient, false, false, &fixture.roots[0])
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fixture.coordinator.pending_recommendations(&fixture.recipient, Some(&fixture.roots[0]), true).unwrap().len(),
        1
    );
}

#[test]
fn send_rejects_invalid_targets_paths_text_and_scope_counts() {
    let fixture = Fixture::new();
    let send = |target: &str, files: Vec<PathBuf>, reason: &str| {
        fixture.coordinator.send_recommendation_for(
            &fixture.sender,
            target,
            RecommendationAction::Defer,
            &files,
            &[],
            reason,
            "replacement",
            &fixture.roots[0],
        )
    };
    assert_eq!(send("repo", vec![PathBuf::from("src/legacy.rs")], "reason").unwrap_err().kind, ErrorKind::Usage);
    assert_eq!(
        send(&fixture.sender.session_id, vec![PathBuf::from("src/legacy.rs")], "reason").unwrap_err().kind,
        ErrorKind::Usage
    );
    assert_eq!(
        send("missing", vec![PathBuf::from("src/legacy.rs")], "reason").unwrap_err().kind,
        ErrorKind::Operational
    );
    assert_eq!(
        send(&fixture.recipient.session_id, vec![PathBuf::from("../escape.rs")], "reason").unwrap_err().kind,
        ErrorKind::Usage
    );
    assert_eq!(
        send(&fixture.recipient.session_id, vec![PathBuf::from("src/legacy.rs")], " \n\t").unwrap_err().kind,
        ErrorKind::Usage
    );
    let too_long = "x".repeat(2001);
    assert_eq!(
        send(&fixture.recipient.session_id, vec![PathBuf::from("src/legacy.rs")], &too_long).unwrap_err().kind,
        ErrorKind::Usage
    );
    let too_many = (0..51).map(|index| PathBuf::from(format!("src/{index}.rs"))).collect();
    assert_eq!(send(&fixture.recipient.session_id, too_many, "reason").unwrap_err().kind, ErrorKind::Usage);

    let mut store = fixture.coordinator.store().unwrap();
    for (name, pid) in [("match-one", 12), ("match-two", 13)] {
        let duplicate = identity(name);
        register(&mut store, &duplicate, &fixture.roots[0], pid, SessionState::Working);
        save_work(
            &mut store,
            &duplicate,
            "Other recipient",
            WorkState::Active,
            vec![claim(&fixture.roots[0], vec![scope("src/legacy.rs", false)])],
        );
        fixture.probe.set(pid, ProcessLiveness::Alive);
    }
    drop(store);
    assert_eq!(send("match", vec![PathBuf::from("src/legacy.rs")], "reason").unwrap_err().kind, ErrorKind::Operational);
}

#[test]
fn draft_only_and_cross_root_endpoints_do_not_qualify() {
    let fixture = Fixture::new();
    let mut store = fixture.coordinator.store().unwrap();
    store.with_work_transaction(|transaction| transaction.delete_work(&fixture.recipient).map(|_| ())).unwrap();
    store
        .save_draft(
            DraftOwner::Session(fixture.recipient.clone()),
            "Draft only",
            &[DraftClaimUpdate {
                repo_root: fixture.roots[0].to_string_lossy().into_owned(),
                scopes: vec![scope("src/legacy.rs", false)],
            }],
            2.0,
        )
        .unwrap();
    drop(store);
    let error = fixture
        .coordinator
        .send_recommendation_for(
            &fixture.sender,
            &fixture.recipient.session_id,
            RecommendationAction::Defer,
            &[PathBuf::from("src/legacy.rs")],
            &[],
            "reason",
            "replacement",
            &fixture.roots[0],
        )
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Usage);

    let mut store = fixture.coordinator.store().unwrap();
    save_work(
        &mut store,
        &fixture.recipient,
        "Other root",
        WorkState::Active,
        vec![claim(&fixture.roots[1], vec![scope("src/legacy.rs", false)])],
    );
    drop(store);
    let error = fixture
        .coordinator
        .send_recommendation_for(
            &fixture.sender,
            &fixture.recipient.session_id,
            RecommendationAction::Defer,
            &[PathBuf::from("src/legacy.rs")],
            &[],
            "reason",
            "replacement",
            &fixture.roots[0],
        )
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Usage);
}

#[test]
fn unknown_liveness_fails_closed_without_deleting_ownership_and_dead_is_reconciled() {
    let fixture = Fixture::new();
    fixture.probe.set(11, ProcessLiveness::Unknown);
    let error = fixture
        .coordinator
        .send_recommendation_for(
            &fixture.sender,
            &fixture.recipient.session_id,
            RecommendationAction::Defer,
            &[PathBuf::from("src/legacy.rs")],
            &[],
            "reason",
            "replacement",
            &fixture.roots[0],
        )
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Operational);
    let store = fixture.coordinator.store().unwrap();
    assert!(store.session(&fixture.recipient).unwrap().is_some());
    assert!(store.work(&fixture.recipient).unwrap().is_some());
    drop(store);

    fixture.probe.set(11, ProcessLiveness::Dead);
    assert!(
        fixture
            .coordinator
            .send_recommendation_for(
                &fixture.sender,
                &fixture.recipient.session_id,
                RecommendationAction::Defer,
                &[PathBuf::from("src/legacy.rs")],
                &[],
                "reason",
                "replacement",
                &fixture.roots[0],
            )
            .is_err()
    );
    let store = fixture.coordinator.store().unwrap();
    assert!(store.session(&fixture.recipient).unwrap().is_none());
    assert!(store.work(&fixture.recipient).unwrap().is_none());
}

#[test]
fn acceptance_requires_live_endpoints_while_rejection_does_not_change_work() {
    let fixture = Fixture::new();
    let first = fixture.send("reason", "replacement").recommendation;
    fixture.probe.set(10, ProcessLiveness::Unknown);
    let error = fixture
        .coordinator
        .respond_recommendation_for(&fixture.recipient, &first.id, RecommendationDecision::Accepted, "adjusting work")
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Operational);
    let before = fixture.coordinator.store().unwrap().work(&fixture.recipient).unwrap().unwrap();
    let rejected = fixture
        .coordinator
        .respond_recommendation_for(
            &fixture.recipient,
            &first.id,
            RecommendationDecision::Rejected,
            "preservation requirement remains",
        )
        .unwrap();
    assert_eq!(rejected.outcome, RecommendationOutcome::Rejected);
    assert_eq!(fixture.coordinator.store().unwrap().work(&fixture.recipient).unwrap().unwrap(), before);

    fixture.probe.set(10, ProcessLiveness::Alive);
    let second = fixture.send("new reason", "new replacement").recommendation;
    let accepted = fixture
        .coordinator
        .respond_recommendation_for(
            &fixture.recipient,
            &second.id,
            RecommendationDecision::Accepted,
            "defer legacy polish and retain verification",
        )
        .unwrap();
    assert_eq!(accepted.outcome, RecommendationOutcome::Accepted);
    assert_eq!(fixture.coordinator.store().unwrap().work(&fixture.recipient).unwrap().unwrap(), before);
}

#[test]
fn stale_and_conflicting_decisions_remain_typed_mutation_outcomes() {
    let fixture = Fixture::new();
    let decided = fixture.send("decided", "replacement").recommendation;
    fixture
        .coordinator
        .respond_recommendation_for(
            &fixture.recipient,
            &decided.id,
            RecommendationDecision::Rejected,
            "keep the requirement",
        )
        .unwrap();
    fixture.probe.set(10, ProcessLiveness::Unknown);
    let conflict = fixture
        .coordinator
        .respond_recommendation_for(
            &fixture.recipient,
            &decided.id,
            RecommendationDecision::Accepted,
            "different decision",
        )
        .unwrap();
    assert_eq!(conflict.outcome, RecommendationOutcome::Conflict);

    fixture.probe.set(10, ProcessLiveness::Alive);
    let stale = fixture.send("stale", "replacement").recommendation;
    fixture.coordinator.done_for(&fixture.sender, &fixture.roots[0]).unwrap();
    let response = fixture
        .coordinator
        .respond_recommendation_for(
            &fixture.recipient,
            &stale.id,
            RecommendationDecision::Accepted,
            "would have adjusted",
        )
        .unwrap();
    assert_eq!(response.outcome, RecommendationOutcome::Stale);
}

#[test]
fn recommendation_reads_and_retries_reconcile_dead_sources_but_preserve_unknown() {
    let shown_fixture = Fixture::new();
    let shown = shown_fixture.send("shown", "replacement").recommendation;
    shown_fixture
        .coordinator
        .respond_recommendation_for(
            &shown_fixture.recipient,
            &shown.id,
            RecommendationDecision::Accepted,
            "same accepted decision",
        )
        .unwrap();
    let initial_messages = shown_fixture.coordinator.store().unwrap().all_messages().unwrap().len();
    shown_fixture.probe.set(10, ProcessLiveness::Dead);

    let stale = shown_fixture.coordinator.show_recommendation_for(&shown_fixture.recipient, &shown.id).unwrap();
    assert_eq!(stale.state, RecommendationState::Stale);
    assert!(stale.invalidation_reason.as_deref().unwrap().contains("Source work or session changed or ended"));
    let store = shown_fixture.coordinator.store().unwrap();
    assert!(store.session(&shown_fixture.sender).unwrap().is_none());
    assert!(store.work(&shown_fixture.sender).unwrap().is_none());
    let stale_pointer = format!("Recommendation {} stale", shown.id);
    assert_eq!(store.all_messages().unwrap().iter().filter(|message| message.text.contains(&stale_pointer)).count(), 2);
    assert_eq!(store.all_messages().unwrap().len(), initial_messages + 2);

    let retried_fixture = Fixture::new();
    let retried = retried_fixture.send("retried", "replacement").recommendation;
    retried_fixture
        .coordinator
        .respond_recommendation_for(
            &retried_fixture.recipient,
            &retried.id,
            RecommendationDecision::Accepted,
            "same accepted decision",
        )
        .unwrap();
    retried_fixture.probe.set(10, ProcessLiveness::Dead);
    let retry = retried_fixture
        .coordinator
        .respond_recommendation_for(
            &retried_fixture.recipient,
            &retried.id,
            RecommendationDecision::Accepted,
            "same accepted decision",
        )
        .unwrap();
    assert_eq!(retry.outcome, RecommendationOutcome::Stale);
    let store = retried_fixture.coordinator.store().unwrap();
    let stale_pointer = format!("Recommendation {} stale", retried.id);
    assert_eq!(store.all_messages().unwrap().iter().filter(|message| message.text.contains(&stale_pointer)).count(), 2);

    let unknown_fixture = Fixture::new();
    let unknown = unknown_fixture.send("unknown", "replacement").recommendation;
    unknown_fixture
        .coordinator
        .respond_recommendation_for(
            &unknown_fixture.recipient,
            &unknown.id,
            RecommendationDecision::Accepted,
            "same accepted decision",
        )
        .unwrap();
    let initial_messages = unknown_fixture.coordinator.store().unwrap().all_messages().unwrap().len();
    unknown_fixture.probe.set(10, ProcessLiveness::Unknown);
    assert_eq!(
        unknown_fixture.coordinator.show_recommendation_for(&unknown_fixture.recipient, &unknown.id).unwrap().state,
        RecommendationState::Accepted
    );
    let retry = unknown_fixture
        .coordinator
        .respond_recommendation_for(
            &unknown_fixture.recipient,
            &unknown.id,
            RecommendationDecision::Accepted,
            "same accepted decision",
        )
        .unwrap();
    assert_eq!(retry.outcome, RecommendationOutcome::Accepted);
    let store = unknown_fixture.coordinator.store().unwrap();
    assert!(store.session(&unknown_fixture.sender).unwrap().is_some());
    assert!(store.work(&unknown_fixture.sender).unwrap().is_some());
    assert_eq!(store.all_messages().unwrap().len(), initial_messages);
}

#[test]
fn lifecycle_refresh_handles_rearm_narrowing_completion_and_idle_yielding() {
    let fixture = Fixture::new();
    let pending = fixture.send("pending", "replacement").recommendation;
    let rearmed = fixture
        .coordinator
        .start_for(
            fixture.recipient.clone(),
            "Recipient polish",
            &[PathBuf::from("src/legacy.rs"), PathBuf::from("src/independent.rs")],
            &[],
            &fixture.roots[0],
        )
        .unwrap();
    assert!(matches!(rearmed.kind, crate::domain::OutcomeKind::Active | crate::domain::OutcomeKind::Ready));
    assert_eq!(
        fixture.coordinator.show_recommendation_for(&fixture.recipient, &pending.id).unwrap().state,
        RecommendationState::Pending
    );

    fixture
        .coordinator
        .start_for(
            fixture.recipient.clone(),
            "Materially different work",
            &[PathBuf::from("src/new.rs")],
            &[],
            &fixture.roots[0],
        )
        .unwrap();
    assert_eq!(
        fixture.coordinator.show_recommendation_for(&fixture.recipient, &pending.id).unwrap().state,
        RecommendationState::Stale
    );

    let accepted_fixture = Fixture::new();
    let accepted = accepted_fixture.send("accepted", "replacement").recommendation;
    accepted_fixture
        .coordinator
        .respond_recommendation_for(
            &accepted_fixture.recipient,
            &accepted.id,
            RecommendationDecision::Accepted,
            "retain independent verification",
        )
        .unwrap();
    accepted_fixture
        .coordinator
        .start_for(
            accepted_fixture.recipient.clone(),
            "Narrowed recipient work",
            &[PathBuf::from("src/independent.rs")],
            &[],
            &accepted_fixture.roots[0],
        )
        .unwrap();
    assert_eq!(
        accepted_fixture.coordinator.show_recommendation_for(&accepted_fixture.recipient, &accepted.id).unwrap().state,
        RecommendationState::Accepted
    );
    accepted_fixture.coordinator.done_for(&accepted_fixture.sender, &accepted_fixture.roots[0]).unwrap();
    assert_eq!(
        accepted_fixture.coordinator.show_recommendation_for(&accepted_fixture.recipient, &accepted.id).unwrap().state,
        RecommendationState::Stale
    );

    let yielded_fixture = Fixture::new();
    let yielded = yielded_fixture.send("yield", "replacement").recommendation;
    let outcome = yielded_fixture
        .coordinator
        .start_for(
            yielded_fixture.sender.clone(),
            "Source replacement",
            &[PathBuf::from("src/legacy.rs")],
            &[],
            &yielded_fixture.roots[0],
        )
        .unwrap();
    assert_eq!(outcome.kind, crate::domain::OutcomeKind::Ready);
    let recipient_work =
        yielded_fixture.coordinator.store().unwrap().work(&yielded_fixture.recipient).unwrap().unwrap();
    assert_eq!(recipient_work.claims[0].scopes, vec![scope("src/independent.rs", false)]);
    assert_eq!(
        yielded_fixture.coordinator.show_recommendation_for(&yielded_fixture.recipient, &yielded.id).unwrap().state,
        RecommendationState::Stale
    );
}
