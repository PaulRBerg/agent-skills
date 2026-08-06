use std::{
    collections::HashSet,
    sync::{Arc, Barrier},
    thread,
};

use rusqlite::Connection;
use tempfile::tempdir;

use crate::domain::{ClaimState, Client, Identity, ProcessFingerprint, Scope, SessionState};

use super::{
    BaselineRow, ClaimUpdate, EndedObservation, MAX_INBOX_MESSAGES, NOTE_TTL, ProviderCacheRow, SCHEMA_VERSION,
    SessionUpdate, Store,
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
        label: None,
        waiting_for: None,
        permission_mode: None,
        update_permission_mode: false,
        fingerprint: Some(ProcessFingerprint { pid: 42, start_token: Some("boot:42".to_owned()) }),
        started_at: None,
        current,
    }
}

fn claim_update(identity: &Identity) -> ClaimUpdate {
    ClaimUpdate {
        identity: identity.clone(),
        repo_root: "/repo".to_owned(),
        label: "state work".to_owned(),
        state: ClaimState::Active,
        blocked_reason: None,
        scopes: vec![Scope { path: "src/state".to_owned(), recursive: true }],
        baselines: Some(vec![BaselineRow { path: "src/state/mod.rs".to_owned(), oid: "old-oid".to_owned() }]),
        residual_paths: Vec::new(),
        created_at: 1.0,
        updated_at: 1.0,
    }
}

#[test]
fn new_store_has_exact_v9_schema_and_runtime_pragmas() {
    let temporary = tempdir().unwrap();
    let path = temporary.path().join("private/state.db");
    let store = Store::open(&path).unwrap();

    let version: i64 = store.connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    let foreign_keys: i64 = store.connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    let journal_mode: String = store.connection.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
    let synchronous: i64 = store.connection.pragma_query_value(None, "synchronous", |row| row.get(0)).unwrap();
    let session_columns = table_columns(&store.connection, "sessions");
    let path_columns = table_columns(&store.connection, "claim_paths");

    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(foreign_keys, 1);
    assert_eq!(journal_mode, "wal");
    assert_eq!(synchronous, 1);
    assert!(session_columns.is_superset(&HashSet::from([
        "callsign_key".to_owned(),
        "process_start_token".to_owned(),
        "revision".to_owned(),
    ])));
    assert!(path_columns.contains("recursive"));

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
    connection.pragma_update(None, "user_version", 8).unwrap();
    drop(connection);

    let error = Store::open(&path).err().unwrap();
    assert_eq!(
        error.to_string(),
        format!(
            "state schema 8 is incompatible with required schema 9 at {}; \
             close all agents and explicitly replace the ledger before retrying",
            path.display()
        )
    );

    let connection = Connection::open(path).unwrap();
    assert_eq!(connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).unwrap(), 8);
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
fn callsign_claims_are_machine_unique_and_idempotent() {
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
    store.save_claim(&claim_update(&owner)).unwrap();
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
    assert!(store.claim(&owner).unwrap().is_some());

    let current = EndedObservation {
        identity: owner.clone(),
        expected_fingerprint: second.fingerprint,
        expected_revision: second.revision,
    };
    assert_eq!(store.reconcile_ended(&[current]).unwrap(), 1);
    assert!(store.session(&owner).unwrap().is_none());
    assert!(store.claim(&owner).unwrap().is_none());
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
    store.save_claim(&claim_update(&stale)).unwrap();
    store.update_delegate(&stale, "child", Some("explorer"), "active", 1.0).unwrap();

    let mut replacement = session_update(&fresh, 2.0);
    replacement.fingerprint = Some(ProcessFingerprint { pid: 42, start_token: Some("boot:42".to_owned()) });
    store.upsert_session_superseding(&replacement).unwrap();

    assert!(store.session(&stale).unwrap().is_none());
    assert!(store.claim(&stale).unwrap().is_none());
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
fn claim_replacement_is_atomic_across_scopes_baselines_and_residuals() {
    let temporary = tempdir().unwrap();
    let mut store = Store::open(temporary.path().join("state.db")).unwrap();
    let owner = identity(Client::Codex, "owner");
    store.upsert_session(&session_update(&owner, 0.0)).unwrap();
    store.observe_dirt("/repo", &[("docs/readme.md".to_owned(), "dirty".to_owned())], 1.0).unwrap();
    let original = claim_update(&owner);
    store.save_claim(&original).unwrap();

    let mut invalid = original.clone();
    invalid.label = "must roll back".to_owned();
    invalid.scopes.push(invalid.scopes[0].clone());
    invalid.baselines = Some(vec![BaselineRow { path: "replacement".to_owned(), oid: "new".to_owned() }]);
    invalid.residual_paths = vec!["docs/readme.md".to_owned()];
    assert!(store.save_claim(&invalid).is_err());
    assert_eq!(store.claim(&owner).unwrap().unwrap().label, original.label);
    assert_eq!(store.baselines(&owner).unwrap(), original.baselines.clone().unwrap());
    assert!(store.residual_owners("/repo").unwrap().is_empty());

    let replacement = ClaimUpdate {
        label: "narrow".to_owned(),
        scopes: vec![Scope { path: "src/state/store.rs".to_owned(), recursive: false }],
        baselines: Some(vec![BaselineRow { path: "src/state/store.rs".to_owned(), oid: "new".to_owned() }]),
        residual_paths: vec!["docs/readme.md".to_owned()],
        created_at: 2.0,
        updated_at: 2.0,
        ..original
    };
    store.save_claim(&replacement).unwrap();
    assert_eq!(store.claim(&owner).unwrap().unwrap().scopes, replacement.scopes);
    assert_eq!(store.baselines(&owner).unwrap(), replacement.baselines.unwrap());
    assert_eq!(store.residual_owners("/repo").unwrap()[0].identity, owner);
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
