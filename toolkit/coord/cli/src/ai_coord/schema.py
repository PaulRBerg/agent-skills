"""SQLite schema creation for the current ledger format."""

from __future__ import annotations

import sqlite3
from pathlib import Path

SCHEMA_VERSION = 8


class SchemaVersionError(RuntimeError):
    """The ledger format is incompatible with this ai-coord build."""

    def __init__(self, found: int, required: int, path: Path) -> None:
        self.found = found
        self.required = required
        self.path = path
        super().__init__(
            f"state schema {found} is incompatible with required schema {required} at {path}; "
            "close all agents and explicitly replace the ledger before retrying"
        )


_SCHEMA_STATEMENTS = (
    """
    CREATE TABLE sessions (
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        cwd TEXT NOT NULL,
        repo_root TEXT,
        state TEXT NOT NULL,
        callsign TEXT,
        name TEXT,
        label TEXT,
        waiting_for TEXT,
        permission_mode TEXT,
        pid INTEGER,
        process_started_at REAL,
        source TEXT NOT NULL,
        started_at REAL NOT NULL,
        last_seen REAL NOT NULL,
        PRIMARY KEY (client, session_id)
    )
    """,
    """
    CREATE TABLE claims (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        repo_root TEXT NOT NULL,
        label TEXT NOT NULL,
        state TEXT NOT NULL CHECK (state IN ('intent', 'queued', 'active')),
        blocked_reason TEXT,
        created_at REAL NOT NULL,
        updated_at REAL NOT NULL,
        UNIQUE (client, session_id)
    )
    """,
    """
    CREATE TABLE claim_paths (
        claim_id INTEGER NOT NULL REFERENCES claims(id) ON DELETE CASCADE,
        path TEXT NOT NULL,
        PRIMARY KEY (claim_id, path)
    )
    """,
    """
    CREATE TABLE claim_baselines (
        claim_id INTEGER NOT NULL REFERENCES claims(id) ON DELETE CASCADE,
        path TEXT NOT NULL,
        oid TEXT NOT NULL,
        PRIMARY KEY (claim_id, path)
    )
    """,
    """
    CREATE TABLE dirt_observations (
        repo_root TEXT NOT NULL,
        path TEXT NOT NULL,
        blob_hash TEXT NOT NULL,
        first_seen REAL NOT NULL,
        last_seen REAL NOT NULL,
        PRIMARY KEY (repo_root, path)
    )
    """,
    """
    CREATE TABLE residual_owners (
        repo_root TEXT NOT NULL,
        path TEXT NOT NULL,
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        released_at REAL NOT NULL,
        PRIMARY KEY (repo_root, path),
        FOREIGN KEY (repo_root, path)
            REFERENCES dirt_observations(repo_root, path) ON DELETE CASCADE
    )
    """,
    """
    CREATE TABLE messages (
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
    )
    """,
    """
    CREATE INDEX messages_recipient_idx
        ON messages(recipient_client, recipient_session_id, created_at)
    """,
    """
    CREATE TABLE notes (
        id TEXT PRIMARY KEY,
        repo_root TEXT NOT NULL,
        author_client TEXT,
        author_session_id TEXT,
        text TEXT NOT NULL,
        created_at REAL NOT NULL,
        resolved_at REAL
    )
    """,
    "CREATE INDEX notes_repo_idx ON notes(repo_root, created_at)",
    """
    CREATE TABLE delegates (
        parent_client TEXT NOT NULL,
        parent_session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        agent_type TEXT,
        state TEXT NOT NULL,
        last_seen REAL NOT NULL,
        PRIMARY KEY (parent_client, parent_session_id, agent_id)
    )
    """,
    """
    CREATE TABLE hook_health (
        client TEXT NOT NULL,
        event TEXT NOT NULL,
        last_error_code TEXT,
        last_error_at REAL,
        last_success_at REAL,
        PRIMARY KEY (client, event)
    )
    """,
    """
    CREATE TABLE provider_cache (
        context_key TEXT NOT NULL,
        client TEXT NOT NULL CHECK (client IN ('codex', 'claude')),
        refreshed_at REAL NOT NULL,
        ok INTEGER NOT NULL CHECK (ok IN (0, 1)),
        source TEXT NOT NULL,
        enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
        dropped INTEGER NOT NULL CHECK (dropped >= 0),
        PRIMARY KEY (context_key, client)
    )
    """,
    """
    CREATE TABLE metadata (
        key TEXT PRIMARY KEY,
        value INTEGER NOT NULL
    )
    """,
    "INSERT INTO metadata(key, value) VALUES ('generation', 0)",
)


def initialize(connection: sqlite3.Connection, path: Path) -> None:
    """Create or accept exactly the current schema without upgrading old ledgers."""
    current = int(connection.execute("PRAGMA user_version").fetchone()[0])
    if current == SCHEMA_VERSION:
        return
    if current != 0:
        raise SchemaVersionError(current, SCHEMA_VERSION, path)

    connection.execute("BEGIN IMMEDIATE")
    try:
        current = int(connection.execute("PRAGMA user_version").fetchone()[0])
        if current == 0:
            for statement in _SCHEMA_STATEMENTS:
                connection.execute(statement)
            connection.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")
            current = SCHEMA_VERSION
        if current != SCHEMA_VERSION:
            raise SchemaVersionError(current, SCHEMA_VERSION, path)
    except BaseException:
        connection.rollback()
        raise
    else:
        connection.commit()
