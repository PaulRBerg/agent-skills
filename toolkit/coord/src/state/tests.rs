use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

use rusqlite::Connection;
use tempfile::tempdir;

use crate::domain::{Client, Identity, ProcessFingerprint, Scope, ScopeKind, SessionState, WorkState};

use super::{
    BaselineRow, EndedObservation, MAX_INBOX_MESSAGES, NOTE_TTL, ProviderCacheRow, SCHEMA_VERSION, SessionUpdate,
    Store, WorkUpdate,
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
        fingerprint: Some(ProcessFingerprint { pid: 42, start_token: Some("boot:42".to_owned()) }),
        started_at: None,
        current,
    }
}

fn work_update(identity: &Identity) -> WorkUpdate {
    WorkUpdate {
        identity: identity.clone(),
        repo_root: "/repo".to_owned(),
        label: "state work".to_owned(),
        state: WorkState::Active,
        blocked_reason: None,
        scopes: vec![Scope { path: "src/state".to_owned(), kind: ScopeKind::Recursive }],
        baselines: Some(vec![BaselineRow { path: "src/state/mod.rs".to_owned(), oid: "old-oid".to_owned() }]),
        residual_paths: Vec::new(),
        draft_created_at: None,
        submitted_at: Some(1.0),
        updated_at: 1.0,
        expected_revision: None,
    }
}

#[test]
fn new_store_has_exact_v10_schema_and_runtime_pragmas() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("private/state.db");
    let store = Store::open(&path).unwrap();

    let version: i64 = store.connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    let foreign_keys: i64 = store.connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    let journal_mode: String = store.connection.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
    let synchronous: i64 = store.connection.pragma_query_value(None, "synchronous", |row| row.get(0)).unwrap();
    let session_columns = table_columns(&store.connection, "sessions");
    let work_columns = table_columns(&store.connection, "work_items");
    let scope_columns = table_columns(&store.connection, "work_scopes");
    let tables = table_names(&store.connection);

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(foreign_keys, 1);
    assert_eq!(journal_mode, "wal");
    assert_eq!(synchronous, 1);
    assert!(session_columns.is_superset(&HashSet::from([
        "callsign_key".to_owned(),
        "process_start_token".to_owned(),
        "revision".to_owned(),
    ])));
    assert!(work_columns.is_superset(&HashSet::from([
        "draft_created_at".to_owned(),
        "submitted_at".to_owned(),
        "revision".to_owned(),
    ])));
    assert!(scope_columns.contains("kind"));
    assert!(tables.contains("work_items"));
    assert!(tables.contains("work_scopes"));
    assert!(tables.contains("work_baselines"));
    assert!(!tables.contains("claims"));
    assert!(!tables.contains("claim_paths"));
    assert!(!tables.contains("claim_baselines"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(path.parent().unwrap().metadata().unwrap().permissions().mode() & 0o777, 0o700);
    }
}

#[test]
fn incompatible_schema_is_rejected_without_schema_or_journal_mutation() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("state.db");
    let connection = Connection::open(&path).unwrap();
    connection.execute("CREATE TABLE sentinel(value TEXT NOT NULL)", []).unwrap();
    connection.execute("INSERT INTO sentinel VALUES ('preserved')", []).unwrap();
    connection.pragma_update(None, "user_version", 9).unwrap();
    drop(connection);

    let error = Store::open(&path).err().unwrap();
    assert_eq!(
        error.to_string(),
        format!(
            "state schema 9 is incompatible with required schema 10 at {}; \
             close all agents and explicitly replace the ledger before retrying",
            path.display()
        )
    );

    let connection = Connection::open(path).unwrap();
    assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), 9);
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
    store.save_work(&work_update(&owner)).unwrap();
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
    store.save_work(&work_update(&stale)).unwrap();
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
fn pruning_expires_messages_and_notes_but_never_sessions() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let sender = identity(Client::Codex, "sender");
    let recipient = identity(Client::Claude, "recipient");
    store.upsert_session(&session_update(&sender, 0.0)).unwrap();
    store.send_message(&sender, std::slice::from_ref(&recipient), "old", None, 0.0).unwrap();
    store.add_note(&sender, "/repo", "old", 0.0).unwrap();

    store.prune(NOTE_TTL + 1.0).unwrap();

    assert!(store.inbox(&recipient, false).unwrap().is_empty());
    assert!(store.all_notes().unwrap().is_empty());
    assert!(store.session(&sender).unwrap().is_some());
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
fn work_replacement_is_atomic_across_scopes_baselines_and_residuals() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    store.observe_dirt("/repo", &[("docs/readme.md".to_owned(), "dirty".to_owned())], 1.0).unwrap();
    let original = work_update(&owner);
    store.save_work(&original).unwrap();
    let original_row = store.work(&owner).unwrap().unwrap();

    let mut invalid = original.clone();
    invalid.expected_revision = Some(original_row.revision);
    invalid.label = "must roll back".to_owned();
    invalid.scopes.push(invalid.scopes[0].clone());
    invalid.baselines = Some(vec![BaselineRow { path: "replacement".to_owned(), oid: "new".to_owned() }]);
    invalid.residual_paths = vec!["docs/readme.md".to_owned()];
    assert!(store.save_work(&invalid).is_err());
    assert_eq!(store.work(&owner).unwrap().unwrap().label, original.label);
    assert_eq!(store.baselines(&owner).unwrap(), original.baselines.clone().unwrap());
    assert!(store.residual_owners("/repo").unwrap().is_empty());

    let replacement = WorkUpdate {
        label: "narrow".to_owned(),
        scopes: vec![Scope { path: "src/state/store.rs".to_owned(), kind: ScopeKind::Exact }],
        baselines: Some(vec![BaselineRow { path: "src/state/store.rs".to_owned(), oid: "new".to_owned() }]),
        residual_paths: vec!["docs/readme.md".to_owned()],
        submitted_at: Some(2.0),
        updated_at: 2.0,
        expected_revision: Some(original_row.revision),
        ..original
    };
    store.save_work(&replacement).unwrap();
    assert_eq!(store.work(&owner).unwrap().unwrap().scopes, replacement.scopes);
    assert_eq!(store.baselines(&owner).unwrap(), replacement.baselines.unwrap());
    assert_eq!(store.residual_owners("/repo").unwrap()[0].identity, owner);
}

#[test]
fn draft_replacement_is_atomic_and_advances_its_revision() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    let first_scopes = vec![Scope { path: "src/lib.rs".to_owned(), kind: ScopeKind::Exact }];
    let first = store.save_draft(&owner, "/repo", "first", &first_scopes, 1.0).unwrap();
    assert_eq!(first.state, WorkState::Draft);
    assert_eq!(first.draft_created_at, Some(1.0));
    assert_eq!(first.submitted_at, None);

    let second_scopes = vec![Scope { path: "src".to_owned(), kind: ScopeKind::Recursive }];
    let second = store.save_draft(&owner, "/repo", "second", &second_scopes, 2.0).unwrap();
    assert_eq!(second.id, first.id);
    assert_eq!(second.revision, first.revision + 1);
    assert_eq!(second.draft_created_at, Some(2.0));
    assert_eq!(second.scopes, second_scopes);

    let duplicate = vec![second.scopes[0].clone(), second.scopes[0].clone()];
    assert!(store.save_draft(&owner, "/repo", "rolled back", &duplicate, 3.0).is_err());
    assert_eq!(store.work(&owner).unwrap().unwrap(), second);
}

#[test]
fn work_revision_compare_and_swap_rejects_stale_writers() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    store.save_work(&work_update(&owner)).unwrap();
    let current = store.work(&owner).unwrap().unwrap();
    let mut update = work_update(&owner);
    update.label = "winner".to_owned();
    update.expected_revision = Some(current.revision);
    store.save_work(&update).unwrap();

    update.label = "stale loser".to_owned();
    assert_eq!(store.save_work(&update).unwrap_err().to_string(), "work item changed during update");
    assert_eq!(store.work(&owner).unwrap().unwrap().label, "winner");
}

#[test]
fn fifo_clock_advances_only_when_submitted_and_breaks_timestamp_ties() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    store
        .save_draft(&owner, "/repo", "draft", &[Scope { path: "src/lib.rs".to_owned(), kind: ScopeKind::Exact }], 42.0)
        .unwrap();
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
fn work_schema_constraints_and_session_cascades_are_enforced() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();

    assert!(
        store
            .connection
            .execute(
                "INSERT INTO work_items(
            client, session_id, repo_root, label, state, blocked_reason,
            draft_created_at, submitted_at, updated_at, revision
         ) VALUES ('codex', 'missing', '/repo', 'bad', 'draft', NULL, 1, NULL, 1, 1)",
                [],
            )
            .is_err()
    );
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO work_items(
            client, session_id, repo_root, label, state, blocked_reason,
            draft_created_at, submitted_at, updated_at, revision
         ) VALUES ('codex', 'owner', '/repo', 'bad', 'draft', NULL, NULL, 1, 1, 1)",
                [],
            )
            .is_err()
    );

    store.save_work(&work_update(&owner)).unwrap();
    let work_id = store.work(&owner).unwrap().unwrap().id;
    assert!(
        store
            .connection
            .execute("INSERT INTO work_scopes(work_id, path, kind) VALUES (?1, 'bad', 'prefix')", [work_id],)
            .is_err()
    );
    store.update_delegate(&owner, "child", Some("test"), "active", 1.0).unwrap();
    store.end_session(&owner).unwrap();

    for table in ["work_items", "work_scopes", "work_baselines", "delegates"] {
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

fn table_columns(connection: &Connection, table: &str) -> HashSet<String> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})")).unwrap();
    statement.query_map([], |row| row.get::<_, String>(1)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}

fn table_names(connection: &Connection) -> HashSet<String> {
    let mut statement =
        connection.prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'").unwrap();
    statement.query_map([], |row| row.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}
