use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

use rusqlite::Connection;
use tempfile::tempdir;

use crate::domain::{
    Client, FindingKind, FindingState, Identity, ProcessFingerprint, Scope, ScopeKind, SessionState, WorkState,
};

use super::{
    BaselineRow, EndedObservation, FindingAdd, FindingCounts, FindingPathObservation, FindingResolution,
    MAX_INBOX_MESSAGES, ProviderCacheRow, SCHEMA_VERSION, SessionUpdate, Store, WorkClaimUpdate, WorkUpdate,
};

fn identity(client: Client, session_id: &str) -> Identity {
    Identity { client, session_id: session_id.to_owned() }
}

fn session_update(identity: &Identity, current: f64) -> SessionUpdate {
    SessionUpdate {
        identity: identity.clone(),
        cwd: "/repo".to_owned(),
        repo_root: Some("/repo".to_owned()),
        state: SessionState::Working,
        source: "test".to_owned(),
        name: None,
        waiting_for: None,
        permission_mode: None,
        update_permission_mode: false,
        coordination_waived: None,
        fingerprint: Some(ProcessFingerprint { pid: 42, start_token: Some("boot:42".to_owned()) }),
        started_at: None,
        current,
    }
}

fn work_update(identity: &Identity) -> WorkUpdate {
    WorkUpdate {
        identity: identity.clone(),
        label: "state work".to_owned(),
        state: WorkState::Active,
        blocked_reason: None,
        claims: vec![WorkClaimUpdate {
            repo_root: "/repo".to_owned(),
            blocked_reason: None,
            scopes: vec![Scope { path: "src/state".to_owned(), kind: ScopeKind::Recursive }],
            baselines: Some(vec![BaselineRow { path: "src/state/mod.rs".to_owned(), oid: "old-oid".to_owned() }]),
            residual_paths: Vec::new(),
        }],
        draft_created_at: None,
        submitted_at: Some(1.0),
        updated_at: 1.0,
        expected_revision: None,
    }
}

fn work_claim(repo_root: &str, path: &str, oid: &str) -> WorkClaimUpdate {
    WorkClaimUpdate {
        repo_root: repo_root.to_owned(),
        blocked_reason: None,
        scopes: vec![Scope { path: path.to_owned(), kind: ScopeKind::Exact }],
        baselines: Some(vec![BaselineRow { path: path.to_owned(), oid: oid.to_owned() }]),
        residual_paths: Vec::new(),
    }
}

fn save_work(store: &mut Store, update: &WorkUpdate) -> crate::error::Result<i64> {
    store.with_work_transaction(|transaction| transaction.save_work(update))
}

#[test]
fn new_store_has_exact_v15_schema_and_runtime_pragmas() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("private/state.db");
    let store = Store::open(&path).unwrap();

    let version: i64 = store.connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    let foreign_keys: i64 = store.connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    let journal_mode: String = store.connection.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
    let synchronous: i64 = store.connection.pragma_query_value(None, "synchronous", |row| row.get(0)).unwrap();
    let session_columns = table_columns(&store.connection, "sessions");
    let work_columns = table_columns(&store.connection, "work_items");
    let claim_columns = table_columns(&store.connection, "work_claims");
    let scope_columns = table_columns(&store.connection, "work_scopes");
    let baseline_columns = table_columns(&store.connection, "work_baselines");
    let tables = table_names(&store.connection);

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(foreign_keys, 1);
    assert_eq!(journal_mode, "wal");
    assert_eq!(synchronous, 1);
    assert!(session_columns.is_superset(&HashSet::from([
        "callsign_key".to_owned(),
        "process_start_token".to_owned(),
        "coordination_waived".to_owned(),
        "revision".to_owned(),
    ])));
    assert_eq!(
        work_columns,
        HashSet::from([
            "id".to_owned(),
            "client".to_owned(),
            "session_id".to_owned(),
            "label".to_owned(),
            "state".to_owned(),
            "blocked_reason".to_owned(),
            "draft_created_at".to_owned(),
            "submitted_at".to_owned(),
            "updated_at".to_owned(),
            "revision".to_owned(),
        ])
    );
    assert_eq!(
        claim_columns,
        HashSet::from(["id".to_owned(), "work_id".to_owned(), "repo_root".to_owned(), "blocked_reason".to_owned(),])
    );
    assert_eq!(scope_columns, HashSet::from(["claim_id".to_owned(), "path".to_owned(), "kind".to_owned()]));
    assert_eq!(baseline_columns, HashSet::from(["claim_id".to_owned(), "path".to_owned(), "oid".to_owned()]));
    assert_eq!(
        foreign_key_targets(&store.connection, "work_items"),
        HashSet::from([
            ("client".to_owned(), "sessions".to_owned(), "client".to_owned(), "CASCADE".to_owned()),
            ("session_id".to_owned(), "sessions".to_owned(), "session_id".to_owned(), "CASCADE".to_owned(),),
        ])
    );
    assert_eq!(
        foreign_key_targets(&store.connection, "work_claims"),
        HashSet::from([("work_id".to_owned(), "work_items".to_owned(), "id".to_owned(), "CASCADE".to_owned())])
    );
    assert_eq!(
        foreign_key_targets(&store.connection, "work_scopes"),
        HashSet::from([("claim_id".to_owned(), "work_claims".to_owned(), "id".to_owned(), "CASCADE".to_owned())])
    );
    assert_eq!(
        foreign_key_targets(&store.connection, "work_baselines"),
        HashSet::from([("claim_id".to_owned(), "work_claims".to_owned(), "id".to_owned(), "CASCADE".to_owned())])
    );
    assert!(tables.contains("work_items"));
    assert!(tables.contains("work_claims"));
    assert!(tables.contains("work_scopes"));
    assert!(tables.contains("work_baselines"));
    assert!(tables.contains("current_turns"));
    assert!(tables.contains("findings"));
    assert!(tables.contains("finding_paths"));
    assert!(tables.contains("finding_observations"));
    assert!(tables.contains("finding_sightings"));
    assert!(tables.contains("finding_events"));
    assert!(tables.contains("triage_runs"));
    assert!(tables.contains("finding_claims"));
    assert!(!tables.contains("notes"));
    assert!(!tables.contains("claims"));
    assert!(!tables.contains("claim_paths"));
    assert!(!tables.contains("claim_baselines"));

    let sighting_columns = store
        .connection
        .prepare("PRAGMA table_info(finding_sightings)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<HashSet<_>>>()
        .unwrap();
    assert!(sighting_columns.contains("surfaced_at"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(path.parent().unwrap().metadata().unwrap().permissions().mode() & 0o777, 0o700);
    }
}

#[cfg(unix)]
#[test]
fn opening_store_does_not_change_an_existing_shared_directory() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let temporary = tempdir().unwrap();
    let shared = temporary.path().join("shared");
    fs::create_dir(&shared).unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
    let database = shared.join("state.db");

    Store::open(&database).unwrap();

    assert_eq!(shared.metadata().unwrap().permissions().mode() & 0o777, 0o755);
    assert_eq!(database.metadata().unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn incompatible_schema_is_rejected_without_schema_or_journal_mutation() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("state.db");
    let connection = Connection::open(&path).unwrap();
    connection.execute("CREATE TABLE sentinel(value TEXT NOT NULL)", []).unwrap();
    connection.execute("INSERT INTO sentinel VALUES ('preserved')", []).unwrap();
    connection.pragma_update(None, "user_version", 14).unwrap();
    drop(connection);

    let error = Store::open(&path).err().unwrap();
    assert_eq!(
        error.to_string(),
        format!(
            "state schema 14 is incompatible with required schema 15 at {}; \
             close all agents and explicitly replace the ledger before retrying",
            path.display()
        )
    );

    let connection = Connection::open(path).unwrap();
    assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), 14);
    assert_eq!(
        connection.query_row("SELECT value FROM sentinel", [], |row| row.get::<_, String>(0)).unwrap(),
        "preserved"
    );
    assert_eq!(connection.pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0)).unwrap(), "delete");
}

#[test]
fn initialization_is_concurrency_safe() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("state.db");
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            thread::spawn(move || {
                barrier.wait();
                Store::open(path).unwrap();
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(Store::open(path).unwrap().generation().unwrap(), 0);
}

#[test]
fn prompt_waiver_updates_are_atomic_and_observers_preserve_them() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    let mut prompt = session_update(&owner, 1.0);
    prompt.coordination_waived = Some(true);
    store.upsert_session(&prompt).unwrap();
    assert!(store.session(&owner).unwrap().unwrap().coordination_waived);

    let generation = store.generation().unwrap();
    let observer = session_update(&owner, 2.0);
    store.upsert_session(&observer).unwrap();
    assert!(store.session(&owner).unwrap().unwrap().coordination_waived);
    assert_eq!(store.generation().unwrap(), generation);

    let mut untagged = session_update(&owner, 3.0);
    untagged.coordination_waived = Some(false);
    store.upsert_session(&untagged).unwrap();
    assert!(!store.session(&owner).unwrap().unwrap().coordination_waived);
    assert_eq!(store.generation().unwrap(), generation + 1);
}

#[test]
fn callsign_reservations_are_machine_unique_and_idempotent() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("state.db");
    Store::open(&path).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles = [Client::Codex, Client::Claude]
        .into_iter()
        .enumerate()
        .map(|(index, client)| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                let identity = identity(client, &format!("session-{index}"));
                store.upsert_session(&session_update(&identity, index as f64)).unwrap();
                barrier.wait();
                store.set_session_callsign(&identity, "✈️ Night Owl").is_ok()
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| **outcome).count(), 1);

    let mut store = Store::open(path).unwrap();
    let owner = store.sessions().unwrap().into_iter().find(|session| session.callsign.is_some()).unwrap();
    let generation = store.generation().unwrap();
    store.set_session_callsign(&owner.identity, "✈️ Night Owl").unwrap();
    assert_eq!(store.generation().unwrap(), generation);
}

#[test]
fn callsign_keys_use_nfc_and_full_unicode_casefold() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let first = identity(Client::Codex, "first");
    let second = identity(Client::Claude, "second");
    store.upsert_session(&session_update(&first, 1.0)).unwrap();
    store.upsert_session(&session_update(&second, 1.0)).unwrap();

    store.set_session_callsign(&first, "🚀 Café Straße").unwrap();
    let error = store.set_session_callsign(&second, "🚀 Cafe\u{301} STRASSE").unwrap_err();

    assert_eq!(error.to_string(), "callsign is already in use");
}

#[test]
fn stale_ended_observation_cannot_remove_a_refreshed_session() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    let first = store.upsert_session(&session_update(&owner, 1.0)).unwrap();
    save_work(&mut store, &work_update(&owner)).unwrap();
    store.update_delegate(&owner, "child", Some("explorer"), "active", 1.0).unwrap();
    let generation = store.generation().unwrap();

    let second = store.upsert_session(&session_update(&owner, 2.0)).unwrap();
    assert_eq!(second.revision, first.revision + 1);
    let stale = EndedObservation {
        identity: owner.clone(),
        expected_fingerprint: first.fingerprint,
        expected_revision: first.revision,
    };
    assert_eq!(store.reconcile_ended(&[stale]).unwrap(), 0);
    assert!(store.session(&owner).unwrap().is_some());
    assert!(store.work(&owner).unwrap().is_some());

    let current = EndedObservation {
        identity: owner.clone(),
        expected_fingerprint: second.fingerprint,
        expected_revision: second.revision,
    };
    assert_eq!(store.reconcile_ended(&[current]).unwrap(), 1);
    assert!(store.session(&owner).unwrap().is_none());
    assert!(store.work(&owner).unwrap().is_none());
    assert!(store.delegates().unwrap().is_empty());
    assert_eq!(store.generation().unwrap(), generation + 1);
}

#[test]
fn reconcile_requires_the_exact_fingerprint_even_at_the_same_revision() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    let row = store.upsert_session(&session_update(&owner, 1.0)).unwrap();
    let mismatched = EndedObservation {
        identity: owner.clone(),
        expected_fingerprint: Some(ProcessFingerprint { pid: 42, start_token: Some("reused-pid".to_owned()) }),
        expected_revision: row.revision,
    };
    assert_eq!(store.reconcile_ended(&[mismatched]).unwrap(), 0);
    assert!(store.session(&owner).unwrap().is_some());
}

#[test]
fn new_identity_on_the_same_strong_client_process_supersedes_stale_top_level_state() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let stale = identity(Client::Codex, "stale");
    let fresh = identity(Client::Codex, "fresh");
    store.upsert_session(&session_update(&stale, 1.0)).unwrap();
    save_work(&mut store, &work_update(&stale)).unwrap();
    store.update_delegate(&stale, "child", Some("explorer"), "active", 1.0).unwrap();

    let mut replacement = session_update(&fresh, 2.0);
    replacement.fingerprint = Some(ProcessFingerprint { pid: 42, start_token: Some("boot:42".to_owned()) });
    store.upsert_session_superseding(&replacement).unwrap();

    assert!(store.session(&stale).unwrap().is_none());
    assert!(store.work(&stale).unwrap().is_none());
    assert!(store.delegates().unwrap().is_empty());
    assert!(store.session(&fresh).unwrap().is_some());
}

#[test]
fn pruning_expires_messages_but_never_findings_or_sessions() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let sender = identity(Client::Codex, "sender");
    let recipient = identity(Client::Claude, "recipient");
    store.upsert_session(&session_update(&sender, 0.0)).unwrap();
    store.send_message(&sender, std::slice::from_ref(&recipient), "old", None, 0.0).unwrap();
    store
        .add_finding(&FindingAdd {
            repo_root: "/repo".into(),
            summary: "old finding".into(),
            normalized_summary: "old finding".into(),
            kind: None,
            paths: vec![],
            head_oid: None,
            observations: vec![],
            author: sender.clone(),
            turn_id: Some("turn".into()),
            current: 0.0,
        })
        .unwrap();

    store.prune(super::store::MESSAGE_TTL + 1.0).unwrap();

    assert!(store.inbox(&recipient, false).unwrap().is_empty());
    assert_eq!(store.findings("/repo", None, true, f64::MAX).unwrap().len(), 1);
    assert!(store.session(&sender).unwrap().is_some());
}

#[test]
fn findings_deduplicate_exact_open_records_and_preserve_terminal_recurrence() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let author = identity(Client::Codex, "author");
    let mut input = FindingAdd {
        repo_root: "/repo".into(),
        summary: "same finding".into(),
        normalized_summary: "same finding".into(),
        kind: Some(FindingKind::Bug),
        paths: vec!["docs/a.md".into(), "src/a.rs".into()],
        head_oid: Some("head-one".into()),
        observations: vec![FindingPathObservation {
            path: "src/a.rs".into(),
            content_sha256: Some("content-one".into()),
        }],
        author: author.clone(),
        turn_id: Some("turn-one".into()),
        current: 1.0,
    };
    let first = store.add_finding(&input).unwrap();
    assert!(!first.deduplicated);
    input.kind = Some(FindingKind::Docs);
    input.turn_id = Some("turn-two".into());
    input.current = 2.0;
    let duplicate = store.add_finding(&input).unwrap();
    assert!(duplicate.deduplicated);
    assert_eq!(duplicate.finding.id, first.finding.id);
    assert_eq!(duplicate.finding.kind, Some(FindingKind::Bug));
    assert_eq!(duplicate.finding.sighting_count, 2);

    store
        .resolve_finding(
            "/repo",
            &first.finding.id,
            &FindingResolution {
                state: FindingState::Fixed,
                commit_oid: Some("abcdef0".into()),
                canonical_id: None,
                actor: author.clone(),
                current: 3.0,
            },
        )
        .unwrap();
    input.current = 4.0;
    let recurrence = store.add_finding(&input).unwrap();
    assert!(!recurrence.deduplicated);
    assert_ne!(recurrence.finding.id, first.finding.id);
}

#[test]
fn current_turn_inherits_missing_sighting_ids_and_surfaces_duplicate_sightings_together() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let author = identity(Client::Claude, "author");
    store.upsert_session(&session_update(&author, 1.0)).unwrap();
    let turn_id = store.begin_turn(&author, None, 2.0).unwrap();
    assert!(turn_id.starts_with("local-"));
    let input = FindingAdd {
        repo_root: "/repo".into(),
        summary: "same finding".into(),
        normalized_summary: "same finding".into(),
        kind: Some(FindingKind::Bug),
        paths: vec!["src/lib.rs".into()],
        head_oid: None,
        observations: vec![],
        author: author.clone(),
        turn_id: None,
        current: 3.0,
    };
    let first = store.add_finding(&input).unwrap();
    let duplicate = store.add_finding(&input).unwrap();
    assert!(duplicate.deduplicated);
    assert_eq!(store.current_turn_findings(&author).unwrap().len(), 1);
    assert_eq!(store.current_turn_findings(&author).unwrap()[0].id, first.finding.id);
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM finding_sightings WHERE turn_id = ?1 AND surfaced_at IS NULL",
                [&turn_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        2
    );
    assert_eq!(store.mark_current_turn_findings_surfaced(&author, 4.0).unwrap(), 2);
    assert!(store.current_turn_findings(&author).unwrap().is_empty());
}

#[test]
fn explicit_sighting_turn_id_overrides_the_persisted_current_turn() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let author = identity(Client::Codex, "author");
    store.upsert_session(&session_update(&author, 1.0)).unwrap();
    store.begin_turn(&author, Some("provider-turn"), 2.0).unwrap();
    store
        .add_finding(&FindingAdd {
            repo_root: "/repo".into(),
            summary: "different turn".into(),
            normalized_summary: "different turn".into(),
            kind: None,
            paths: vec![],
            head_oid: None,
            observations: vec![],
            author: author.clone(),
            turn_id: Some("explicit-turn".into()),
            current: 3.0,
        })
        .unwrap();
    assert!(store.current_turn_findings(&author).unwrap().is_empty());
}

#[test]
fn finding_creation_does_not_create_a_wait_wake_message() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let author = identity(Client::Codex, "author");
    store
        .add_finding(&FindingAdd {
            repo_root: "/repo".into(),
            summary: "durable only".into(),
            normalized_summary: "durable only".into(),
            kind: None,
            paths: vec![],
            head_oid: None,
            observations: vec![],
            author,
            turn_id: None,
            current: 1.0,
        })
        .unwrap();
    assert_eq!(store.connection.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
}

#[test]
fn finding_candidates_and_lifecycle_transitions_are_bounded_and_explicit() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let author = identity(Client::Claude, "author");
    for index in 0..7 {
        store
            .add_finding(&FindingAdd {
                repo_root: "/repo".into(),
                summary: format!("candidate {index}"),
                normalized_summary: format!("candidate {index}"),
                kind: None,
                paths: vec!["src/shared.rs".into()],
                head_oid: None,
                observations: vec![],
                author: author.clone(),
                turn_id: None,
                current: index as f64,
            })
            .unwrap();
    }
    let added = store
        .add_finding(&FindingAdd {
            repo_root: "/repo".into(),
            summary: "new report".into(),
            normalized_summary: "new report".into(),
            kind: Some(FindingKind::Improvement),
            paths: vec!["src/shared.rs".into()],
            head_oid: None,
            observations: vec![],
            author: author.clone(),
            turn_id: None,
            current: 10.0,
        })
        .unwrap();
    assert_eq!(added.candidates.len(), 5);
    store
        .connection
        .execute(
            "INSERT INTO triage_runs(
                id, repo_root, runner_client, runner_session_id, started_at
             ) VALUES ('run-one', '/repo', 'claude', 'author', 10.0)",
            [],
        )
        .unwrap();
    store
        .connection
        .execute(
            "INSERT INTO finding_claims(finding_id, triage_run_id, claimed_at, lease_expires_at)
             VALUES (?1, 'run-one', 10.0, 20.0)",
            [&added.finding.id],
        )
        .unwrap();
    assert!(store.finding("/repo", &added.finding.id, 15.0).unwrap().unwrap().triaging);
    assert!(!store.finding("/repo", &added.finding.id, 20.0).unwrap().unwrap().triaging);
    assert_eq!(store.finding_counts("/repo", 15.0).unwrap(), FindingCounts { pending: 8, triaging: 1, handed_off: 0 });
    let handed_off = store.handoff_finding("/repo", &added.finding.id, "src/shared.rs", &author, 11.0).unwrap();
    assert_eq!(handed_off.state, FindingState::HandedOff);
    assert_eq!(store.finding_counts("/repo", 15.0).unwrap(), FindingCounts { pending: 7, triaging: 1, handed_off: 1 });
    let resolved = store
        .resolve_finding(
            "/repo",
            &added.finding.id,
            &FindingResolution {
                state: FindingState::Rejected,
                commit_oid: None,
                canonical_id: None,
                actor: author.clone(),
                current: 12.0,
            },
        )
        .unwrap();
    assert_eq!(resolved.terminal_at, Some(12.0));
    let reopened = store.reopen_finding("/repo", &added.finding.id, &author, 13.0).unwrap();
    assert_eq!(reopened.state, FindingState::Pending);
    assert_eq!(reopened.handoff_path, None);
    assert_eq!(reopened.terminal_at, None);
}

#[test]
fn inbox_is_capped_and_callsigns_are_snapshotted() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let sender = identity(Client::Codex, "sender");
    let recipient = identity(Client::Claude, "recipient");
    store.upsert_session(&session_update(&sender, 0.0)).unwrap();
    store.upsert_session(&session_update(&recipient, 0.0)).unwrap();
    store.set_session_callsign(&sender, "🦊 Fox One").unwrap();
    store.set_session_callsign(&recipient, "🐙 Octo Two").unwrap();
    for index in 0..MAX_INBOX_MESSAGES + 5 {
        store
            .send_message(
                &sender,
                std::slice::from_ref(&recipient),
                &format!("message {index}"),
                Some("/repo"),
                index as f64,
            )
            .unwrap();
    }
    store.end_session(&sender).unwrap();
    store.end_session(&recipient).unwrap();

    let inbox = store.inbox(&recipient, false).unwrap();
    assert_eq!(inbox.len(), MAX_INBOX_MESSAGES);
    assert_eq!(inbox[0].text, "message 5");
    assert_eq!(inbox[0].sender_callsign.as_deref(), Some("🦊 Fox One"));
    assert_eq!(inbox[0].recipient_callsign.as_deref(), Some("🐙 Octo Two"));
}

#[test]
fn one_work_retains_stable_repository_claims_and_isolates_children() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    store.observe_dirt("/repo-a", &[("src/a.rs".to_owned(), "dirty-a".to_owned())], 1.0).unwrap();
    store.observe_dirt("/repo-b", &[("src/b.rs".to_owned(), "dirty-b".to_owned())], 1.0).unwrap();

    let mut update = work_update(&owner);
    update.claims = vec![work_claim("/repo-b", "src/b.rs", "b-old"), work_claim("/repo-a", "src/a.rs", "a-old")];
    update.claims[0].blocked_reason = Some("waiting for repo b".to_owned());
    update.claims[0].residual_paths = vec!["src/b.rs".to_owned()];
    update.claims[1].scopes.push(Scope { path: "README.md".to_owned(), kind: ScopeKind::Exact });
    let work_id = save_work(&mut store, &update).unwrap();
    let original = store.work(&owner).unwrap().unwrap();

    assert_eq!(original.id, work_id);
    assert_eq!(
        original.claims.iter().map(|claim| claim.repo_root.as_str()).collect::<Vec<_>>(),
        ["/repo-a", "/repo-b"]
    );
    assert_eq!(original.claim("/repo-b").unwrap().blocked_reason.as_deref(), Some("waiting for repo b"));
    assert_eq!(original.claim("/repo-a").unwrap().scopes[0].path, "README.md");
    assert_eq!(store.works().unwrap(), vec![original.clone()]);
    assert_eq!(store.works_in_repo("/repo-a").unwrap(), vec![original.clone()]);
    assert!(store.works_in_repo("/missing").unwrap().is_empty());
    assert_eq!(store.baselines_in_repo(&owner, "/repo-a").unwrap()[0].oid, "a-old");
    assert_eq!(store.baselines_in_repo(&owner, "/repo-b").unwrap()[0].oid, "b-old");
    assert_eq!(store.residual_owners("/repo-b").unwrap()[0].repo_root, "/repo-b");

    let a_id = original.claim("/repo-a").unwrap().id;
    let b_id = original.claim("/repo-b").unwrap().id;
    update.label = "replacement".to_owned();
    update.expected_revision = Some(original.revision);
    update.updated_at = 2.0;
    update.claims = vec![work_claim("/repo-c", "src/c.rs", "c-new"), work_claim("/repo-b", "src/b2.rs", "b-new")];
    update.claims[1].baselines = None;
    save_work(&mut store, &update).unwrap();

    let replaced = store.work(&owner).unwrap().unwrap();
    assert_eq!(replaced.id, work_id);
    assert_eq!(replaced.revision, original.revision + 1);
    assert_eq!(replaced.claim("/repo-b").unwrap().id, b_id);
    assert_ne!(replaced.claim("/repo-c").unwrap().id, a_id);
    assert!(replaced.claim("/repo-a").is_none());
    assert_eq!(replaced.claim("/repo-b").unwrap().scopes[0].path, "src/b2.rs");
    assert_eq!(store.baselines_in_repo(&owner, "/repo-b").unwrap()[0].oid, "b-old");
    assert_eq!(store.baselines_in_repo(&owner, "/repo-c").unwrap()[0].oid, "c-new");
    let removed_children: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM work_scopes WHERE claim_id = ?1", [a_id], |row| row.get(0))
        .unwrap();
    assert_eq!(removed_children, 0);
}

#[test]
fn work_save_is_atomic_and_cas_rollback_preserves_all_claims() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    let initial_generation = store.generation().unwrap();
    let mut empty = work_update(&owner);
    empty.claims.clear();
    assert_eq!(save_work(&mut store, &empty).unwrap_err().to_string(), "at least one repository claim is required");
    assert!(store.work(&owner).unwrap().is_none());
    assert_eq!(store.generation().unwrap(), initial_generation);
    let mut original = work_update(&owner);
    original.claims.push(work_claim("/repo-b", "src/b.rs", "b-old"));
    save_work(&mut store, &original).unwrap();
    let before = store.work(&owner).unwrap().unwrap();
    let baselines_before = store.baselines_in_repo(&owner, "/repo").unwrap();
    let generation = store.generation().unwrap();

    let mut invalid = original.clone();
    invalid.label = "must roll back".to_owned();
    invalid.expected_revision = Some(before.revision);
    invalid.claims[0].scopes = vec![Scope { path: "replacement.rs".to_owned(), kind: ScopeKind::Exact }];
    invalid.claims[0].baselines =
        Some(vec![BaselineRow { path: "replacement.rs".to_owned(), oid: "replacement".to_owned() }]);
    invalid.claims[0].residual_paths = vec!["not-observed.rs".to_owned()];
    assert!(save_work(&mut store, &invalid).is_err());
    assert_eq!(store.work(&owner).unwrap().unwrap(), before);
    assert_eq!(store.baselines_in_repo(&owner, "/repo").unwrap(), baselines_before);
    assert_eq!(store.generation().unwrap(), generation);

    let mut winner = original.clone();
    winner.label = "winner".to_owned();
    winner.expected_revision = Some(before.revision);
    winner.updated_at = 2.0;
    save_work(&mut store, &winner).unwrap();
    let after_winner = store.work(&owner).unwrap().unwrap();
    let mut stale = original;
    stale.label = "stale loser".to_owned();
    stale.expected_revision = Some(before.revision);
    stale.claims = vec![work_claim("/replacement", "new.rs", "new")];
    assert_eq!(save_work(&mut store, &stale).unwrap_err().to_string(), "work item changed during update");
    assert_eq!(store.work(&owner).unwrap().unwrap(), after_winner);
    assert_eq!(store.generation().unwrap(), generation + 1);
}

#[test]
fn draft_replacement_is_whole_work_only_and_rejects_authoritative_work() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    let first_claims = [work_claim("/repo-b", "b.rs", "ignored"), work_claim("/repo-a", "a.rs", "ignored")];
    let first = store.save_draft(&owner, "first", &first_claims, 1.0).unwrap();
    assert_eq!(first.state, WorkState::Draft);
    assert_eq!(first.claims.iter().map(|claim| claim.repo_root.as_str()).collect::<Vec<_>>(), ["/repo-a", "/repo-b"]);
    assert!(store.baselines_in_repo(&owner, "/repo-a").unwrap().is_empty());

    let retained_id = first.claim("/repo-b").unwrap().id;
    let second_claims = [work_claim("/repo-c", "c.rs", "ignored"), work_claim("/repo-b", "b2.rs", "ignored")];
    let second = store.save_draft(&owner, "second", &second_claims, 2.0).unwrap();
    assert_eq!(second.id, first.id);
    assert_eq!(second.revision, first.revision + 1);
    assert_eq!(second.claim("/repo-b").unwrap().id, retained_id);
    assert!(second.claim("/repo-a").is_none());

    let mut queued = work_update(&owner);
    queued.state = WorkState::Queued;
    queued.blocked_reason = Some("blocked".to_owned());
    queued.submitted_at = Some(3.0);
    queued.expected_revision = Some(second.revision);
    save_work(&mut store, &queued).unwrap();
    let before = store.work(&owner).unwrap().unwrap();
    assert_eq!(
        store.save_draft(&owner, "rejected", &first_claims, 4.0).unwrap_err().to_string(),
        "queued or active work exists; run ai-coord done before drafting"
    );
    assert_eq!(store.work(&owner).unwrap().unwrap(), before);
}

#[test]
fn work_generation_advances_once_per_save_or_delete() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    let initial = store.generation().unwrap();
    save_work(&mut store, &work_update(&owner)).unwrap();
    assert_eq!(store.generation().unwrap(), initial + 1);
    let revision = store.work(&owner).unwrap().unwrap().revision;
    let mut replacement = work_update(&owner);
    replacement.expected_revision = Some(revision);
    save_work(&mut store, &replacement).unwrap();
    assert_eq!(store.generation().unwrap(), initial + 2);
    assert!(store.with_work_transaction(|transaction| transaction.delete_work(&owner)).unwrap());
    assert_eq!(store.generation().unwrap(), initial + 3);
    assert!(!store.with_work_transaction(|transaction| transaction.delete_work(&owner)).unwrap());
    assert_eq!(store.generation().unwrap(), initial + 3);
}

#[test]
fn fifo_clock_advances_only_when_submitted_and_breaks_timestamp_ties() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    store.save_draft(&owner, "draft", &[work_claim("/repo", "src/lib.rs", "ignored")], 42.0).unwrap();
    let untouched: i64 = store
        .connection
        .query_row("SELECT value FROM metadata WHERE key = 'submission_clock_micros'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(untouched, 0);

    let first = store.with_work_transaction(|transaction| transaction.next_submission_time(42.0)).unwrap();
    let second = store.with_work_transaction(|transaction| transaction.next_submission_time(42.0)).unwrap();
    assert_eq!(first, 42.0);
    assert!(second > first);
}

#[test]
fn work_schema_constraints_and_cascades_are_enforced() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();

    assert!(
        store
            .connection
            .execute(
                "INSERT INTO work_items(
                    client, session_id, label, state, blocked_reason,
                    draft_created_at, submitted_at, updated_at, revision
                 ) VALUES ('codex', 'missing', 'bad', 'draft', NULL, 1, NULL, 1, 1)",
                [],
            )
            .is_err()
    );
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO work_items(
                    client, session_id, label, state, blocked_reason,
                    draft_created_at, submitted_at, updated_at, revision
                 ) VALUES ('codex', 'owner', 'bad', 'draft', NULL, NULL, 1, 1, 1)",
                [],
            )
            .is_err()
    );

    save_work(&mut store, &work_update(&owner)).unwrap();
    let work = store.work(&owner).unwrap().unwrap();
    let claim_id = work.claim("/repo").unwrap().id;
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO work_items(
                    client, session_id, label, state, blocked_reason,
                    draft_created_at, submitted_at, updated_at, revision
                 ) VALUES ('codex', 'owner', 'duplicate', 'draft', NULL, 2, NULL, 2, 1)",
                [],
            )
            .is_err()
    );
    assert!(
        store
            .connection
            .execute("INSERT INTO work_scopes(claim_id, path, kind) VALUES (?1, 'bad', 'prefix')", [claim_id])
            .is_err()
    );
    assert!(
        store
            .connection
            .execute("INSERT INTO work_claims(work_id, repo_root) VALUES (?1, '/repo')", [work.id])
            .is_err()
    );
    store.update_delegate(&owner, "child", Some("test"), "active", 1.0).unwrap();
    store.end_session(&owner).unwrap();

    for table in ["work_items", "work_claims", "work_scopes", "work_baselines", "delegates"] {
        let count: i64 =
            store.connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0, "{table} did not cascade");
    }
}

#[test]
fn provider_cache_hook_health_and_dirt_observations_round_trip() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let cache = ProviderCacheRow {
        context_key: "ignored input context".to_owned(),
        client: Client::Codex,
        refreshed_at: 0.0,
        ok: true,
        source: "app-server".to_owned(),
        enabled: true,
        dropped: 2,
    };
    store.replace_provider_cache("cwd:/repo", std::slice::from_ref(&cache), 3.0).unwrap();
    let cached = store.provider_cache("cwd:/repo").unwrap();
    assert_eq!(cached[0].context_key, "cwd:/repo");
    assert_eq!(cached[0].refreshed_at, 3.0);
    assert_eq!(cached[0].dropped, 2);

    store.hook_error(Client::Codex, "SessionStart", &"x".repeat(100), 4.0).unwrap();
    let health = store.hook_health().unwrap();
    assert_eq!(health[0].last_error_code.as_ref().unwrap().chars().count(), 80);
    store.hook_success(Client::Codex, "SessionStart", 5.0).unwrap();
    let health = store.hook_health().unwrap();
    assert_eq!(health[0].last_error_code, None);
    assert_eq!(health[0].last_success_at, Some(5.0));

    let first = store.observe_dirt("/repo", &[("a".to_owned(), "one".to_owned())], 1.0).unwrap();
    let stable = store.observe_dirt("/repo", &[("a".to_owned(), "one".to_owned())], 2.0).unwrap();
    let changed = store.observe_dirt("/repo", &[("a".to_owned(), "two".to_owned())], 3.0).unwrap();
    assert_eq!(first[0].first_seen, 1.0);
    assert_eq!(stable[0].first_seen, 1.0);
    assert_eq!(changed[0].first_seen, 3.0);
}

#[test]
fn partial_dirt_observation_retains_omitted_dirty_paths_and_residual_owners() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    let initial = [("relevant.rs".to_owned(), "one".to_owned()), ("omitted.rs".to_owned(), "two".to_owned())];
    store.observe_dirt("/repo", &initial, 1.0).unwrap();
    store
        .with_work_transaction(|transaction| {
            transaction.record_residual_owners("/repo", &["omitted.rs".to_owned()], &owner, 1.5)
        })
        .unwrap();

    let still_dirty = ["relevant.rs".to_owned(), "omitted.rs".to_owned()];
    let observations = store
        .observe_dirt_subset("/repo", &still_dirty, &[("relevant.rs".to_owned(), "updated".to_owned())], 2.0)
        .unwrap();
    let omitted = observations.iter().find(|observation| observation.path == "omitted.rs").unwrap();
    assert_eq!(omitted.first_seen, 1.0);
    assert_eq!(store.residual_owners("/repo").unwrap()[0].identity, owner);

    let pruned = store
        .observe_dirt_subset(
            "/repo",
            &["relevant.rs".to_owned()],
            &[("relevant.rs".to_owned(), "updated".to_owned())],
            3.0,
        )
        .unwrap();
    assert_eq!(pruned.iter().map(|observation| observation.path.as_str()).collect::<Vec<_>>(), ["relevant.rs"]);
    assert!(store.residual_owners("/repo").unwrap().is_empty());
}

fn table_columns(connection: &Connection, table: &str) -> HashSet<String> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})")).unwrap();
    statement.query_map([], |row| row.get::<_, String>(1)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}

fn table_names(connection: &Connection) -> HashSet<String> {
    let mut statement =
        connection.prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'").unwrap();
    statement.query_map([], |row| row.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}

fn foreign_key_targets(connection: &Connection, table: &str) -> HashSet<(String, String, String, String)> {
    let mut statement = connection.prepare(&format!("PRAGMA foreign_key_list({table})")).unwrap();
    statement
        .query_map([], |row| Ok((row.get(3)?, row.get(2)?, row.get(4)?, row.get(6)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
