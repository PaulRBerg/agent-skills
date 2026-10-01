use std::{sync::mpsc, thread};

use tempfile::{TempDir, tempdir};

use super::*;
use crate::{
    domain::{Client, ProcessFingerprint, ScopeKind, SessionState, WorkState},
    state::{BaselineRow, RecommendationAction, SessionUpdate, WorkClaimUpdate, WorkUpdate},
};

struct Fixture {
    _temporary: TempDir,
    store: Store,
    sender: Identity,
    recipient: Identity,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempdir().unwrap();
        let mut store = Store::open(temporary.path().join("state.db")).unwrap();
        let sender = identity("sender");
        let recipient = identity("recipient");
        register(&mut store, &sender);
        register(&mut store, &recipient);
        store.set_session_callsign(&sender, "Source").unwrap();
        store.set_session_callsign(&recipient, "Target").unwrap();
        Self { _temporary: temporary, store, sender, recipient }
    }

    fn input(&self) -> RecommendationSend {
        RecommendationSend {
            sender: observation(&self.store, &self.sender),
            recipient: observation(&self.store, &self.recipient),
            repo_root: "/repo".to_owned(),
            scopes: vec![scope("src/legacy.rs", ScopeKind::Exact)],
            action: RecommendationAction::Defer,
            reason: "Evidence ".repeat(60).trim().to_owned(),
            replacement: "Replace the legacy input and verify the retained requirement.".to_owned(),
            current: 10.0,
        }
    }

    fn send(&mut self) -> RecommendationRow {
        let input = self.input();
        self.store.with_work_transaction(|tx| tx.send_recommendation(&input)).unwrap().recommendation
    }

    fn respond(&mut self, id: &str, decision: RecommendationDecision, reason: &str) -> RecommendationMutation {
        let observations = [observation(&self.store, &self.sender), observation(&self.store, &self.recipient)];
        self.store
            .with_work_transaction(|tx| {
                tx.respond_recommendation(&self.recipient, id, decision, reason, &observations, 20.0)
            })
            .unwrap()
    }
}

fn identity(name: &str) -> Identity {
    Identity { client: Client::Codex, session_id: name.to_owned() }
}

fn scope(path: &str, kind: ScopeKind) -> Scope {
    Scope { path: path.to_owned(), kind }
}

fn register(store: &mut Store, identity: &Identity) {
    store
        .upsert_session(&SessionUpdate {
            identity: identity.clone(),
            cwd: "/repo".to_owned(),
            repo_root: Some("/repo".to_owned()),
            state: SessionState::Idle,
            source: "test".to_owned(),
            name: None,
            waiting_for: None,
            permission_mode: None,
            update_permission_mode: false,
            coordination_waived: None,
            fingerprint: Some(ProcessFingerprint { pid: 42, start_token: Some("start".to_owned()) }),
            transcript_path: None,
            started_at: Some(1.0),
            current: 1.0,
        })
        .unwrap();
    store.with_work_transaction(|tx| tx.save_work(&work_update(identity))).unwrap();
}

fn work_update(identity: &Identity) -> WorkUpdate {
    WorkUpdate {
        identity: identity.clone(),
        label: "Submitted work".to_owned(),
        state: WorkState::Queued,
        blocked_reason: Some("holder".to_owned()),
        claims: vec![WorkClaimUpdate {
            repo_root: "/repo".to_owned(),
            blocked_reason: Some("holder".to_owned()),
            scopes: vec![scope("src", ScopeKind::Recursive)],
            baselines: Some(vec![BaselineRow { path: "src/legacy.rs".to_owned(), oid: "old-oid".to_owned() }]),
        }],
        submitted_at: Some(1.0),
        updated_at: 1.0,
        expected_revision: None,
    }
}

fn observation(store: &Store, identity: &Identity) -> RecommendationObservation {
    RecommendationObservation::new(
        &store.session(identity).unwrap().unwrap(),
        &store.work(identity).unwrap().unwrap(),
        ProcessLiveness::Alive,
    )
}

fn update_current(store: &mut Store, identity: &Identity, mutate: impl FnOnce(&mut WorkUpdate)) {
    let mut update = work_update(identity);
    update.expected_revision = Some(store.work(identity).unwrap().unwrap().revision);
    mutate(&mut update);
    store.with_work_transaction(|tx| tx.save_work(&update)).unwrap();
}

#[test]
fn durable_history_and_callsign_snapshots_survive_session_deletion() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    fixture.store.set_session_callsign(&fixture.sender, "RenamedSource").unwrap();
    fixture.store.set_session_callsign(&fixture.recipient, "RenamedTarget").unwrap();
    fixture.store.end_session(&fixture.sender).unwrap();
    fixture.store.refresh_recommendations(12.0).unwrap();
    let pointers = fixture.store.inbox(&fixture.recipient, false).unwrap();
    assert_eq!(pointers.len(), 2);
    assert!(pointers.iter().all(|message| message.sender_callsign.as_deref() == Some("Source")));
    assert!(pointers.iter().all(|message| message.recipient_callsign.as_deref() == Some("Target")));
    assert!(pointers.iter().all(|message| !message.text.contains("Evidence")));
    fixture.store.end_session(&fixture.recipient).unwrap();
    let historical = fixture.store.recommendation(&fixture.recipient, &record.id).unwrap().unwrap();
    assert_eq!(historical.state, RecommendationState::Stale);
    assert_eq!(historical.sender, record.sender);
    assert_eq!(historical.recipient, record.recipient);
    assert_eq!(historical.reason, record.reason);
    assert!(historical.reason.len() > 240);
    assert!(historical.invalidation_reason.unwrap().contains("verify retained requirements"));
}

#[test]
fn rearming_promotion_and_operational_updates_preserve_pending_context() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    for owner in [fixture.sender.clone(), fixture.recipient.clone()] {
        update_current(&mut fixture.store, &owner, |update| {
            update.label = "  Submitted   work  ".to_owned();
            update.state = WorkState::Active;
            update.blocked_reason = None;
            update.claims[0].blocked_reason = None;
            update.claims[0].baselines = Some(vec![]);
            update.updated_at = 50.0;
        });
    }
    assert_eq!(fixture.store.refresh_recommendations(60.0).unwrap(), 0);
    assert_eq!(
        fixture.store.recommendation(&fixture.sender, &record.id).unwrap().unwrap().state,
        RecommendationState::Pending
    );
    assert_eq!(fixture.store.all_messages().unwrap().len(), 1);
}

#[test]
fn material_changes_at_either_endpoint_stale_pending_including_other_bundle_claims() {
    for sender_changed in [true, false] {
        for label_changed in [true, false] {
            let mut fixture = Fixture::new();
            let record = fixture.send();
            let owner = if sender_changed { fixture.sender.clone() } else { fixture.recipient.clone() };
            update_current(&mut fixture.store, &owner, |update| {
                if label_changed {
                    update.label = "Different work".to_owned();
                } else {
                    update.claims.push(WorkClaimUpdate {
                        repo_root: "/other".to_owned(),
                        blocked_reason: None,
                        scopes: vec![scope("other", ScopeKind::Exact)],
                        baselines: None,
                    });
                }
            });
            assert_eq!(fixture.store.refresh_recommendations(12.0).unwrap(), 1);
            assert_eq!(
                fixture.store.recommendation(&fixture.sender, &record.id).unwrap().unwrap().state,
                RecommendationState::Stale
            );
            assert_eq!(fixture.store.refresh_recommendations(13.0).unwrap(), 0);
            assert_eq!(fixture.store.all_messages().unwrap().len(), 3);
        }
    }
}

#[test]
fn accepted_narrowing_and_withdrawal_preserve_the_recipient_decision() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    let accepted = fixture.respond(&record.id, RecommendationDecision::Accepted, "Retain parser checks").recommendation;
    update_current(&mut fixture.store, &fixture.recipient, |update| {
        update.claims[0].scopes = vec![scope("src/parser.rs", ScopeKind::Exact)]
    });
    assert_eq!(fixture.store.refresh_recommendations(25.0).unwrap(), 0);
    assert!(fixture.store.pending_recommendations(&fixture.recipient, None, false).unwrap().is_empty());
    let withdrawn = fixture
        .store
        .with_work_transaction(|tx| {
            tx.withdraw_recommendation(&fixture.sender, &record.id, "Replacement changed", 30.0)
        })
        .unwrap();
    assert_eq!(withdrawn.outcome, RecommendationOutcome::Withdrawn);
    assert_eq!(withdrawn.recommendation.decision, accepted.decision);
    assert_eq!(withdrawn.recommendation.decision_reason, accepted.decision_reason);
    assert_eq!(withdrawn.recommendation.decision_at, accepted.decision_at);
    assert_eq!(fixture.store.inbox(&fixture.recipient, false).unwrap().len(), 2);
}

#[test]
fn accepted_recipient_relabel_still_receives_the_withdrawal_notice() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    fixture.respond(&record.id, RecommendationDecision::Accepted, "Retain parser checks");
    update_current(&mut fixture.store, &fixture.recipient, |update| {
        update.label = "Retain parser validation".to_owned();
        update.claims[0].scopes = vec![scope("src/parser.rs", ScopeKind::Exact)];
    });
    let before = fixture.store.inbox(&fixture.recipient, false).unwrap().len();
    fixture
        .store
        .with_work_transaction(|tx| {
            tx.withdraw_recommendation(&fixture.sender, &record.id, "Replacement changed", 30.0)
        })
        .unwrap();
    let inbox = fixture.store.inbox(&fixture.recipient, false).unwrap();
    assert_eq!(inbox.len(), before + 1);
    assert!(inbox.iter().any(|message| message.text.contains(&format!("Recommendation {} withdrawn", record.id))));
}

#[test]
fn accepted_recipient_completion_remains_history_without_notifying_new_work() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    fixture.respond(&record.id, RecommendationDecision::Accepted, "Proceed");
    fixture.store.with_work_transaction(|tx| tx.delete_work(&fixture.recipient)).unwrap();
    assert_eq!(fixture.store.refresh_recommendations(25.0).unwrap(), 0);
    fixture.store.with_work_transaction(|tx| tx.save_work(&work_update(&fixture.recipient))).unwrap();
    assert_eq!(fixture.store.refresh_recommendations(26.0).unwrap(), 0);
    assert!(fixture.store.pending_recommendations(&fixture.recipient, None, false).unwrap().is_empty());
    let before = fixture.store.inbox(&fixture.recipient, false).unwrap().len();
    fixture.store.end_session(&fixture.sender).unwrap();
    fixture.store.refresh_recommendations(30.0).unwrap();
    let historical = fixture.store.recommendation(&fixture.recipient, &record.id).unwrap().unwrap();
    assert_eq!(historical.state, RecommendationState::Stale);
    assert_eq!(historical.decision, Some(RecommendationDecision::Accepted));
    assert_eq!(fixture.store.inbox(&fixture.recipient, false).unwrap().len(), before);
}

#[test]
fn retries_deduplicate_without_notifications_and_conflicting_decisions_do_not_overwrite() {
    let mut fixture = Fixture::new();
    let input = fixture.input();
    let record = fixture.send();
    fixture.store.set_session_callsign(&fixture.sender, "RenamedSource").unwrap();
    let duplicate = fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).unwrap();
    assert_eq!(duplicate.outcome, RecommendationOutcome::Existing);
    assert_eq!(duplicate.recommendation.id, record.id);
    assert_eq!(fixture.store.all_messages().unwrap().len(), 1);
    fixture.respond(&record.id, RecommendationDecision::Accepted, "Agreed");
    assert_eq!(
        fixture.respond(&record.id, RecommendationDecision::Accepted, "Agreed").outcome,
        RecommendationOutcome::Accepted
    );
    assert_eq!(
        fixture.respond(&record.id, RecommendationDecision::Accepted, "Different reason").outcome,
        RecommendationOutcome::Conflict
    );
    assert_eq!(
        fixture.respond(&record.id, RecommendationDecision::Rejected, "Agreed").outcome,
        RecommendationOutcome::Conflict
    );
    assert_eq!(fixture.store.all_messages().unwrap().len(), 2);
    update_current(&mut fixture.store, &fixture.sender, |update| update.label = "Changed source".to_owned());
    assert_eq!(
        fixture.respond(&record.id, RecommendationDecision::Accepted, "Agreed").outcome,
        RecommendationOutcome::Stale
    );
}

#[test]
fn rejection_and_withdrawal_retries_are_idempotent_but_not_interchangeable() {
    for reject in [true, false] {
        let mut fixture = Fixture::new();
        let record = fixture.send();
        if reject {
            for _ in 0..2 {
                assert_eq!(
                    fixture.respond(&record.id, RecommendationDecision::Rejected, "Authority conflict").outcome,
                    RecommendationOutcome::Rejected
                );
            }
            assert_eq!(
                fixture
                    .store
                    .with_work_transaction(|tx| tx.withdraw_recommendation(&fixture.sender, &record.id, "Cancel", 30.0))
                    .unwrap()
                    .outcome,
                RecommendationOutcome::Conflict
            );
        } else {
            for _ in 0..2 {
                assert_eq!(
                    fixture
                        .store
                        .with_work_transaction(|tx| tx.withdraw_recommendation(
                            &fixture.sender,
                            &record.id,
                            "Cancel",
                            30.0
                        ))
                        .unwrap()
                        .outcome,
                    RecommendationOutcome::Withdrawn
                );
            }
            assert_eq!(
                fixture
                    .store
                    .with_work_transaction(|tx| tx.withdraw_recommendation(
                        &fixture.sender,
                        &record.id,
                        "Different reason",
                        30.0
                    ))
                    .unwrap()
                    .outcome,
                RecommendationOutcome::Conflict
            );
            assert_eq!(
                fixture.respond(&record.id, RecommendationDecision::Accepted, "Agree").outcome,
                RecommendationOutcome::Conflict
            );
        }
        assert_eq!(fixture.store.all_messages().unwrap().len(), 2);
    }
}

#[test]
fn records_and_mutations_are_endpoint_restricted() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    let stranger = identity("stranger");
    assert!(fixture.store.recommendation(&stranger, &record.id).unwrap().is_none());
    assert!(fixture.store.recommendations(&stranger, "/repo", false, true).unwrap().is_empty());
    assert!(fixture.store.recommendations(&fixture.sender, "/other", true, true).unwrap().is_empty());
    assert_eq!(fixture.store.recommendations(&fixture.sender, "/repo", true, false).unwrap().len(), 1);
    assert!(
        fixture
            .store
            .with_work_transaction(|tx| tx.respond_recommendation(
                &fixture.sender,
                &record.id,
                RecommendationDecision::Rejected,
                "No",
                &[],
                20.0
            ))
            .is_err()
    );
    assert!(
        fixture
            .store
            .with_work_transaction(|tx| tx.withdraw_recommendation(&fixture.recipient, &record.id, "No", 20.0))
            .is_err()
    );
    assert!(
        fixture
            .store
            .with_work_transaction(|tx| tx.withdraw_recommendation(&stranger, &record.id, "No", 20.0))
            .is_err()
    );
}

#[test]
fn independent_review_survives_ack_eviction_and_marks_only_specific_events() {
    let mut fixture = Fixture::new();
    let first = fixture.send();
    fixture.store.acknowledge(&fixture.recipient, None, 12.0).unwrap();
    for _ in 0..51 {
        fixture
            .store
            .send_message(&fixture.sender, std::slice::from_ref(&fixture.recipient), "ordinary", Some("/repo"), 13.0)
            .unwrap();
    }
    assert!(
        !fixture.store.inbox(&fixture.recipient, false).unwrap().iter().any(|message| message.text.contains(&first.id))
    );
    let mut second_input = fixture.input();
    second_input.reason = "New evidence".to_owned();
    let second =
        fixture.store.with_work_transaction(|tx| tx.send_recommendation(&second_input)).unwrap().recommendation;
    assert_eq!(
        fixture.store.mark_recommendations_surfaced(&fixture.recipient, std::slice::from_ref(&first.id), 14.0).unwrap(),
        1
    );
    let unseen = fixture.store.pending_recommendations(&fixture.recipient, None, true).unwrap();
    assert_eq!(unseen.len(), 1);
    assert_eq!(unseen[0].id, second.id);
    assert_eq!(fixture.store.pending_recommendations(&fixture.recipient, None, false).unwrap().len(), 2);
    assert_eq!(fixture.store.mark_recommendations_surfaced(&fixture.sender, &[second.id], 14.0).unwrap(), 0);
}

#[test]
fn scope_text_and_endpoint_validation_preserve_storage() {
    let mut fixture = Fixture::new();
    for variant in 0..8 {
        let mut input = fixture.input();
        match variant {
            0 => input.reason = "x".repeat(2001),
            1 => input.replacement = " ".to_owned(),
            2 => input.reason = "control\ntext".to_owned(),
            3 => input.scopes = (0..51).map(|i| scope(&format!("src/{i}"), ScopeKind::Exact)).collect(),
            4 => input.scopes.clear(),
            5 => input.scopes = vec![scope("unclaimed", ScopeKind::Exact)],
            6 => input.recipient = input.sender.clone(),
            _ => input.repo_root = "/unclaimed".to_owned(),
        }
        assert!(fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).is_err());
    }
    assert!(fixture.store.all_messages().unwrap().is_empty());
    let mut input = fixture.input();
    input.reason = "🦀".repeat(2000);
    input.scopes = (0..50).map(|i| scope(&format!("src/{i}"), ScopeKind::Exact)).collect();
    assert!(fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).is_ok());
}

#[test]
fn unknown_process_observations_refuse_new_sends_and_acceptance_without_erasing_state() {
    let mut fixture = Fixture::new();
    let mut input = fixture.input();
    input.sender.liveness = ProcessLiveness::Unknown;
    assert!(fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).is_err());
    let record = fixture.send();
    let mut observations = [input.sender, input.recipient];
    observations[1].liveness = ProcessLiveness::Unknown;
    assert!(
        fixture
            .store
            .with_work_transaction(|tx| tx.respond_recommendation(
                &fixture.recipient,
                &record.id,
                RecommendationDecision::Accepted,
                "Yes",
                &observations,
                20.0
            ))
            .is_err()
    );
    assert_eq!(fixture.store.refresh_recommendations(21.0).unwrap(), 0);
    assert_eq!(
        fixture.store.recommendation(&fixture.sender, &record.id).unwrap().unwrap().state,
        RecommendationState::Pending
    );
    assert!(fixture.store.session(&fixture.sender).unwrap().is_some());
}

#[test]
fn changed_fingerprint_invalidates_pretransaction_liveness_evidence() {
    let mut fixture = Fixture::new();
    let input = fixture.input();
    fixture
        .store
        .connection
        .execute("UPDATE sessions SET process_start_token = 'replaced' WHERE session_id = 'sender'", [])
        .unwrap();
    let error = fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).unwrap_err();
    assert_eq!(error.kind.code(), 1);
    assert!(fixture.store.all_messages().unwrap().is_empty());
}

#[test]
fn caps_count_accepted_as_live_and_expiry_and_retention_use_fake_time() {
    for incoming in [true, false] {
        let mut fixture = Fixture::new();
        for index in 0..50 {
            let mut input = fixture.input();
            input.reason = format!("Reason {index}");
            let record =
                fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).unwrap().recommendation;
            if index == 0 {
                fixture.respond(&record.id, RecommendationDecision::Accepted, "Agree");
            }
        }
        let other = identity("other");
        register(&mut fixture.store, &other);
        let mut input = fixture.input();
        if incoming {
            input.sender = observation(&fixture.store, &other);
        } else {
            input.recipient = observation(&fixture.store, &other);
        }
        let error = fixture.store.with_work_transaction(|tx| tx.send_recommendation(&input)).unwrap_err();
        assert!(error.message.contains(if incoming { "recipient endpoint" } else { "sender endpoint" }));
        assert_eq!(fixture.store.refresh_recommendations(10.0 + RECOMMENDATION_TTL - 0.5).unwrap(), 0);
        fixture.store.prune(10.0 + RECOMMENDATION_TTL).unwrap();
        assert_eq!(fixture.store.recommendations(&fixture.sender, "/repo", true, true).unwrap().len(), 50);
        assert!(
            fixture
                .store
                .recommendations(&fixture.sender, "/repo", true, true)
                .unwrap()
                .iter()
                .all(|record| record.state == RecommendationState::Stale)
        );
        assert_eq!(fixture.store.refresh_recommendations(10.0 + 2.0 * RECOMMENDATION_TTL - 0.5).unwrap(), 0);
        assert_eq!(fixture.store.refresh_recommendations(10.0 + 2.0 * RECOMMENDATION_TTL).unwrap(), 50);
        assert!(fixture.store.recommendations(&fixture.sender, "/repo", true, true).unwrap().is_empty());
    }
}

#[test]
fn rejected_and_withdrawn_retention_starts_at_transition_not_creation() {
    for reject in [true, false] {
        let mut fixture = Fixture::new();
        let record = fixture.send();
        let at = 10.0 + RECOMMENDATION_TTL - 1.0;
        fixture
            .store
            .with_work_transaction(|tx| {
                if reject {
                    tx.respond_recommendation(
                        &fixture.recipient,
                        &record.id,
                        RecommendationDecision::Rejected,
                        "No",
                        &[],
                        at,
                    )
                } else {
                    tx.withdraw_recommendation(&fixture.sender, &record.id, "Cancel", at)
                }
            })
            .unwrap();
        assert_eq!(fixture.store.refresh_recommendations(at + RECOMMENDATION_TTL - 0.5).unwrap(), 0);
        assert!(fixture.store.recommendation(&fixture.sender, &record.id).unwrap().is_some());
        assert_eq!(fixture.store.refresh_recommendations(at + RECOMMENDATION_TTL).unwrap(), 1);
    }
}

/// The winning transaction owns the write lock before the competing operation
/// is released. Independent connections exercise SQLite serialization without sleeps.
fn ordered_race<T: Send + 'static>(
    store: &mut Store,
    first: impl FnOnce(&WorkTransaction<'_>) -> Result<()>,
    second: impl FnOnce(&WorkTransaction<'_>) -> Result<T> + Send + 'static,
) -> Result<T> {
    let mut competitor = Store::open(store.path()).unwrap();
    let (go, wait) = mpsc::channel();
    let handle = thread::spawn(move || {
        wait.recv().unwrap();
        competitor.with_work_transaction(second)
    });
    store
        .with_work_transaction(|tx| {
            go.send(()).unwrap();
            first(tx)
        })
        .unwrap();
    handle.join().unwrap()
}

#[test]
fn send_racing_work_replacement_revalidates_context_inside_transaction() {
    let mut fixture = Fixture::new();
    let input = fixture.input();
    let result = ordered_race(
        &mut fixture.store,
        |tx| {
            tx.delete_work(&fixture.recipient)?;
            tx.save_work(&work_update(&fixture.recipient))?;
            Ok(())
        },
        move |tx| tx.send_recommendation(&input),
    );
    assert_eq!(result.unwrap_err().kind.code(), 1);
    assert!(fixture.store.all_messages().unwrap().is_empty());
    assert!(fixture.store.recommendations(&fixture.sender, "/repo", true, true).unwrap().is_empty());
}

#[test]
fn acceptance_racing_source_change_commits_only_stale_transition_and_pointers() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    let recipient = fixture.recipient.clone();
    let id = record.id.clone();
    let observations = [observation(&fixture.store, &fixture.sender), observation(&fixture.store, &fixture.recipient)];
    let result = ordered_race(
        &mut fixture.store,
        |tx| {
            let mut update = work_update(&fixture.sender);
            update.expected_revision = Some(tx.work(&fixture.sender)?.unwrap().revision);
            update.label = "Changed source".to_owned();
            tx.save_work(&update)?;
            Ok(())
        },
        move |tx| {
            tx.respond_recommendation(&recipient, &id, RecommendationDecision::Accepted, "Agree", &observations, 20.0)
        },
    )
    .unwrap();
    assert_eq!(result.outcome, RecommendationOutcome::Stale);
    assert_eq!(result.recommendation.decision, None);
    assert_eq!(fixture.store.all_messages().unwrap().len(), 3);
}

#[test]
fn acceptance_and_withdrawal_races_preserve_the_winning_decision_in_both_orders() {
    for withdrawal_first in [true, false] {
        let mut fixture = Fixture::new();
        let record = fixture.send();
        let recipient = fixture.recipient.clone();
        let sender = fixture.sender.clone();
        let id = record.id.clone();
        let observations =
            [observation(&fixture.store, &fixture.sender), observation(&fixture.store, &fixture.recipient)];
        let observations2 = observations.clone();
        let result = ordered_race(
            &mut fixture.store,
            |tx| {
                if withdrawal_first {
                    tx.withdraw_recommendation(&fixture.sender, &record.id, "Cancel", 20.0)?;
                } else {
                    tx.respond_recommendation(
                        &fixture.recipient,
                        &record.id,
                        RecommendationDecision::Accepted,
                        "Agree",
                        &observations,
                        20.0,
                    )?;
                }
                Ok(())
            },
            move |tx| {
                if withdrawal_first {
                    tx.respond_recommendation(
                        &recipient,
                        &id,
                        RecommendationDecision::Accepted,
                        "Agree",
                        &observations2,
                        21.0,
                    )
                } else {
                    tx.withdraw_recommendation(&sender, &id, "Cancel", 21.0)
                }
            },
        )
        .unwrap();
        assert_eq!(
            result.outcome,
            if withdrawal_first { RecommendationOutcome::Conflict } else { RecommendationOutcome::Withdrawn }
        );
        assert_eq!(result.recommendation.state, RecommendationState::Withdrawn);
        assert_eq!(
            result.recommendation.decision,
            if withdrawal_first { None } else { Some(RecommendationDecision::Accepted) }
        );
    }
}

#[test]
fn rollback_cannot_leave_a_decision_or_orphan_pointer() {
    let mut fixture = Fixture::new();
    let input = fixture.input();
    let before = fixture.store.works().unwrap();
    let error = fixture
        .store
        .with_work_transaction(|tx| {
            tx.send_recommendation(&input)?;
            Err::<(), _>(AppError::operational("abort insertion"))
        })
        .unwrap_err();
    assert!(error.message.contains("abort"));
    assert!(fixture.store.all_messages().unwrap().is_empty());
    assert!(fixture.store.recommendations(&fixture.sender, "/repo", true, true).unwrap().is_empty());
    let record = fixture.send();
    let generation = fixture.store.generation().unwrap();
    fixture
        .store
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_pointer BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT, 'pointer failed'); END;",
        )
        .unwrap();
    let observations = [observation(&fixture.store, &fixture.sender), observation(&fixture.store, &fixture.recipient)];
    assert!(
        fixture
            .store
            .with_work_transaction(|tx| tx.respond_recommendation(
                &fixture.recipient,
                &record.id,
                RecommendationDecision::Accepted,
                "Agree",
                &observations,
                20.0
            ))
            .is_err()
    );
    let after = fixture.store.recommendation(&fixture.sender, &record.id).unwrap().unwrap();
    assert_eq!(after.state, RecommendationState::Pending);
    assert_eq!(after.decision, None);
    assert_eq!(fixture.store.all_messages().unwrap().len(), 1);
    assert_eq!(fixture.store.generation().unwrap(), generation);
    assert_eq!(fixture.store.works().unwrap(), before);
}

#[test]
fn complete_sorted_snapshots_ignore_claim_ids_and_keep_other_work_data_unchanged() {
    let mut fixture = Fixture::new();
    update_current(&mut fixture.store, &fixture.sender, |update| {
        update.claims.push(WorkClaimUpdate {
            repo_root: "/alpha".to_owned(),
            blocked_reason: Some("unrelated".to_owned()),
            scopes: vec![scope("z", ScopeKind::Exact), scope("a", ScopeKind::Recursive)],
            baselines: None,
        });
    });
    let before = fixture.store.works().unwrap();
    let record = fixture.send();
    assert_eq!(record.sender.work.claims[0].repo_root, "/alpha");
    assert_eq!(record.sender.work.claims[0].scopes[0].path, "a");
    fixture.respond(&record.id, RecommendationDecision::Accepted, "Retain essential checks");
    fixture
        .store
        .with_work_transaction(|tx| tx.withdraw_recommendation(&fixture.sender, &record.id, "Withdraw", 30.0))
        .unwrap();
    assert_eq!(fixture.store.works().unwrap(), before);
    assert_eq!(
        fixture
            .store
            .connection
            .query_row("SELECT oid FROM work_baselines LIMIT 1", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "old-oid"
    );
}

#[test]
fn pending_narrowing_or_replacement_removes_actionable_review_even_before_refresh() {
    for replace in [true, false] {
        let mut fixture = Fixture::new();
        let record = fixture.send();
        if replace {
            fixture
                .store
                .with_work_transaction(|tx| {
                    tx.delete_work(&fixture.recipient)?;
                    tx.save_work(&work_update(&fixture.recipient))
                })
                .unwrap();
        } else {
            update_current(&mut fixture.store, &fixture.recipient, |update| {
                update.claims[0].scopes = vec![scope("src/parser.rs", ScopeKind::Exact)]
            });
        }
        assert!(fixture.store.pending_recommendations(&fixture.recipient, None, false).unwrap().is_empty());
        assert_eq!(
            fixture
                .store
                .mark_recommendations_surfaced(&fixture.recipient, std::slice::from_ref(&record.id), 15.0)
                .unwrap(),
            0
        );
        assert_eq!(
            fixture.store.recommendation(&fixture.sender, &record.id).unwrap().unwrap().state,
            RecommendationState::Stale
        );
    }
}

#[test]
fn rollback_of_validity_refresh_keeps_transition_and_pointer_atomic() {
    let mut fixture = Fixture::new();
    let record = fixture.send();
    fixture.store.end_session(&fixture.sender).unwrap();
    fixture
        .store
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_pointer BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT, 'pointer failed'); END;",
        )
        .unwrap();
    assert!(fixture.store.refresh_recommendations(20.0).is_err());
    assert_eq!(
        fixture.store.recommendation(&fixture.recipient, &record.id).unwrap().unwrap().state,
        RecommendationState::Pending
    );
    assert_eq!(fixture.store.all_messages().unwrap().len(), 1);
    fixture.store.connection.execute_batch("DROP TRIGGER fail_pointer;").unwrap();
    assert_eq!(fixture.store.refresh_recommendations(21.0).unwrap(), 1);
    assert_eq!(
        fixture.store.recommendation(&fixture.recipient, &record.id).unwrap().unwrap().state,
        RecommendationState::Stale
    );
    assert_eq!(fixture.store.all_messages().unwrap().len(), 3);
}
