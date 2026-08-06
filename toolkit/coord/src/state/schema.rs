use std::path::Path;

use rusqlite::{Connection, TransactionBehavior};

use crate::error::{AppError, Result};

pub(crate) const SCHEMA_VERSION: i64 = 10;

const STATEMENTS: &[&str] = &[
    "CREATE TABLE sessions (
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        cwd TEXT NOT NULL,
        repo_root TEXT,
        state TEXT NOT NULL,
        callsign TEXT,
        callsign_key TEXT UNIQUE,
        name TEXT,
        waiting_for TEXT,
        permission_mode TEXT,
        pid INTEGER,
        process_start_token TEXT,
        source TEXT NOT NULL,
        started_at REAL NOT NULL,
        last_seen REAL NOT NULL,
        revision INTEGER NOT NULL,
        PRIMARY KEY (client, session_id),
        CHECK (process_start_token IS NULL OR pid IS NOT NULL)
    )",
    "CREATE TABLE work_items (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        repo_root TEXT NOT NULL,
        label TEXT NOT NULL,
        state TEXT NOT NULL CHECK (state IN ('draft', 'queued', 'active')),
        blocked_reason TEXT,
        draft_created_at REAL,
        submitted_at REAL,
        updated_at REAL NOT NULL,
        revision INTEGER NOT NULL CHECK (revision > 0),
        UNIQUE (client, session_id),
        FOREIGN KEY (client, session_id)
            REFERENCES sessions(client, session_id) ON DELETE CASCADE,
        CHECK (
            (state = 'draft' AND draft_created_at IS NOT NULL
                AND submitted_at IS NULL AND blocked_reason IS NULL)
            OR (state = 'queued' AND submitted_at IS NOT NULL
                AND blocked_reason IS NOT NULL)
            OR (state = 'active' AND submitted_at IS NOT NULL
                AND blocked_reason IS NULL)
        )
    )",
    "CREATE TABLE work_scopes (
        work_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
        path TEXT NOT NULL,
        kind TEXT NOT NULL CHECK (kind IN ('exact', 'recursive')),
        PRIMARY KEY (work_id, path)
    )",
    "CREATE TABLE work_baselines (
        work_id INTEGER NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
        path TEXT NOT NULL,
        oid TEXT NOT NULL,
        PRIMARY KEY (work_id, path)
    )",
    "CREATE TABLE dirt_observations (
        repo_root TEXT NOT NULL,
        path TEXT NOT NULL,
        blob_hash TEXT NOT NULL,
        first_seen REAL NOT NULL,
        last_seen REAL NOT NULL,
        PRIMARY KEY (repo_root, path)
    )",
    "CREATE TABLE residual_owners (
        repo_root TEXT NOT NULL,
        path TEXT NOT NULL,
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        released_at REAL NOT NULL,
        PRIMARY KEY (repo_root, path),
        FOREIGN KEY (repo_root, path)
            REFERENCES dirt_observations(repo_root, path) ON DELETE CASCADE
    )",
    "CREATE TABLE messages (
        id TEXT PRIMARY KEY,
        sender_client TEXT NOT NULL,
        sender_session_id TEXT NOT NULL,
        sender_callsign TEXT,
        recipient_client TEXT NOT NULL,
        recipient_session_id TEXT NOT NULL,
        recipient_callsign TEXT,
        repo_root TEXT,
        text TEXT NOT NULL,
        created_at REAL NOT NULL,
        acknowledged_at REAL,
        notified_at REAL
    )",
    "CREATE INDEX messages_recipient_idx
        ON messages(recipient_client, recipient_session_id, created_at)",
    "CREATE TABLE notes (
        id TEXT PRIMARY KEY,
        repo_root TEXT NOT NULL,
        author_client TEXT,
        author_session_id TEXT,
        text TEXT NOT NULL,
        created_at REAL NOT NULL,
        resolved_at REAL,
        CHECK ((author_client IS NULL) = (author_session_id IS NULL))
    )",
    "CREATE INDEX notes_repo_idx ON notes(repo_root, created_at)",
    "CREATE TABLE delegates (
        parent_client TEXT NOT NULL,
        parent_session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        agent_type TEXT,
        state TEXT NOT NULL,
        last_seen REAL NOT NULL,
        PRIMARY KEY (parent_client, parent_session_id, agent_id),
        FOREIGN KEY (parent_client, parent_session_id)
            REFERENCES sessions(client, session_id) ON DELETE CASCADE
    )",
    "CREATE TABLE hook_health (
        client TEXT NOT NULL,
        event TEXT NOT NULL,
        last_error_code TEXT,
        last_error_at REAL,
        last_success_at REAL,
        PRIMARY KEY (client, event)
    )",
    "CREATE TABLE provider_cache (
        context_key TEXT NOT NULL,
        client TEXT NOT NULL CHECK (client IN ('codex', 'claude')),
        refreshed_at REAL NOT NULL,
        ok INTEGER NOT NULL CHECK (ok IN (0, 1)),
        source TEXT NOT NULL,
        enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
        dropped INTEGER NOT NULL CHECK (dropped >= 0),
        PRIMARY KEY (context_key, client)
    )",
    "CREATE TABLE metadata (
        key TEXT PRIMARY KEY,
        value INTEGER NOT NULL
    )",
    "INSERT INTO metadata(key, value) VALUES ('generation', 0)",
    "INSERT INTO metadata(key, value) VALUES ('submission_clock_micros', 0)",
];

pub(super) fn initialize(connection: &mut Connection, path: &Path) -> Result<()> {
    let current = user_version(connection)?;
    if current == SCHEMA_VERSION {
        return Ok(());
    }
    if current != 0 {
        return Err(version_error(current, path));
    }

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = user_version(&transaction)?;
    if current == 0 {
        for statement in STATEMENTS {
            transaction.execute_batch(statement)?;
        }
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    } else if current != SCHEMA_VERSION {
        return Err(version_error(current, path));
    }
    transaction.commit()?;
    Ok(())
}

fn user_version(connection: &Connection) -> Result<i64> {
    Ok(connection.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn version_error(found: i64, path: &Path) -> AppError {
    AppError::operational(format!(
        "state schema {found} is incompatible with required schema {SCHEMA_VERSION} at {}; \
         close all agents and explicitly replace the ledger before retrying",
        path.display()
    ))
}
