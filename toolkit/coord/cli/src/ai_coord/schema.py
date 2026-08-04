"""SQLite schema creation and migration ladder."""

from __future__ import annotations

import sqlite3

SCHEMA_VERSION = 4

_SCHEMA_STATEMENTS = (
    """
    CREATE TABLE sessions (
        client TEXT NOT NULL,
        session_id TEXT NOT NULL,
        cwd TEXT NOT NULL,
        repo_root TEXT,
        state TEXT NOT NULL,
        name TEXT,
        label TEXT,
        waiting_for TEXT,
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
        recipient_client TEXT NOT NULL,
        recipient_session_id TEXT NOT NULL,
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
    CREATE TABLE imports (
        source_path TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        imported_at REAL NOT NULL,
        PRIMARY KEY (source_path, content_hash)
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


def migrate(connection: sqlite3.Connection) -> None:
    """Bring one state database to the supported schema version."""
    current = int(connection.execute("PRAGMA user_version").fetchone()[0])
    if current > SCHEMA_VERSION:
        raise RuntimeError(
            f"state schema {current} is newer than supported schema {SCHEMA_VERSION}"
        )
    if current == SCHEMA_VERSION:
        return
    connection.execute("BEGIN IMMEDIATE")
    try:
        current = int(connection.execute("PRAGMA user_version").fetchone()[0])
        if current > SCHEMA_VERSION:
            raise RuntimeError(
                f"state schema {current} is newer than supported schema {SCHEMA_VERSION}"
            )
        if current == 0:
            for statement in _SCHEMA_STATEMENTS:
                connection.execute(statement)
            connection.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")
            current = SCHEMA_VERSION
        if current == 1:
            connection.execute("ALTER TABLE messages ADD COLUMN notified_at REAL")
            connection.execute("PRAGMA user_version = 2")
            current = 2
        if current == 2:
            connection.execute("ALTER TABLE sessions ADD COLUMN process_started_at REAL")
            connection.execute("PRAGMA user_version = 3")
            current = 3
        if current == 3:
            # Frozen v4 snapshot: never share this DDL with the evolving _SCHEMA_STATEMENTS.
            connection.execute(
                """
                CREATE TABLE IF NOT EXISTS claim_baselines (
                    claim_id INTEGER NOT NULL REFERENCES claims(id) ON DELETE CASCADE,
                    path TEXT NOT NULL,
                    oid TEXT NOT NULL,
                    PRIMARY KEY (claim_id, path)
                )
                """
            )
            connection.execute(
                """
                CREATE TABLE IF NOT EXISTS dirt_observations (
                    repo_root TEXT NOT NULL,
                    path TEXT NOT NULL,
                    blob_hash TEXT NOT NULL,
                    first_seen REAL NOT NULL,
                    last_seen REAL NOT NULL,
                    PRIMARY KEY (repo_root, path)
                )
                """
            )
            connection.execute(
                """
                CREATE TABLE IF NOT EXISTS residual_owners (
                    repo_root TEXT NOT NULL,
                    path TEXT NOT NULL,
                    client TEXT NOT NULL,
                    session_id TEXT NOT NULL,
                    released_at REAL NOT NULL,
                    PRIMARY KEY (repo_root, path),
                    FOREIGN KEY (repo_root, path)
                        REFERENCES dirt_observations(repo_root, path) ON DELETE CASCADE
                )
                """
            )
            connection.execute("PRAGMA user_version = 4")
    except BaseException:
        connection.rollback()
        raise
    else:
        connection.commit()
