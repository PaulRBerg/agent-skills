"""SQLite persistence for coordination state."""

from __future__ import annotations

import contextlib
import hashlib
import os
import sqlite3
import time
from collections.abc import Iterator, Sequence
from pathlib import Path
from typing import Any

from ai_coord.identity import Identity, ProcessReference
from ai_coord.util import new_id, now_ts, private_state_dir, sanitize

SCHEMA_VERSION = 3
CODEX_IDLE_TTL = 4 * 60 * 60
CODEX_ORPHAN_GRACE = 30 * 60
MESSAGE_TTL = 48 * 60 * 60
NOTE_TTL = 7 * 24 * 60 * 60
MAX_INBOX_MESSAGES = 50

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


class Store:
    """One local SQLite ledger."""

    def __init__(self, path: Path | None = None) -> None:
        self.path = path or private_state_dir() / "state.db"
        self.path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.path.parent.chmod(0o700)
        previous_umask = os.umask(0o077)
        try:
            self.connection = sqlite3.connect(
                self.path,
                timeout=5,
                isolation_level=None,
            )
        finally:
            os.umask(previous_umask)
        self.connection.row_factory = sqlite3.Row
        self.connection.execute("PRAGMA foreign_keys = ON")
        self.connection.execute("PRAGMA busy_timeout = 250")
        self._enable_wal()
        self.connection.execute("PRAGMA busy_timeout = 5000")
        self.connection.execute("PRAGMA synchronous = NORMAL")
        self._migrate()
        self.path.chmod(0o600)

    def close(self) -> None:
        self.connection.close()

    def _enable_wal(self) -> None:
        deadline = time.monotonic() + 2
        while True:
            try:
                mode = str(self.connection.execute("PRAGMA journal_mode = WAL").fetchone()[0])
                if mode.lower() != "wal":
                    raise RuntimeError(f"could not enable SQLite WAL mode: {mode}")
                return
            except sqlite3.OperationalError as error:
                if "locked" not in str(error).lower() or time.monotonic() >= deadline:
                    raise
                time.sleep(0.01)

    def _migrate(self) -> None:
        current = int(self.connection.execute("PRAGMA user_version").fetchone()[0])
        if current > SCHEMA_VERSION:
            raise RuntimeError(
                f"state schema {current} is newer than supported schema {SCHEMA_VERSION}"
            )
        if current == SCHEMA_VERSION:
            return
        with self.transaction() as connection:
            current = int(connection.execute("PRAGMA user_version").fetchone()[0])
            if current > SCHEMA_VERSION:
                raise RuntimeError(
                    f"state schema {current} is newer than supported schema {SCHEMA_VERSION}"
                )
            if current == 0:
                for statement in _SCHEMA_STATEMENTS:
                    connection.execute(statement)
                connection.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")
            if current == 1:
                connection.execute("ALTER TABLE messages ADD COLUMN notified_at REAL")
                connection.execute("PRAGMA user_version = 2")
                current = 2
            if current == 2:
                connection.execute("ALTER TABLE sessions ADD COLUMN process_started_at REAL")
                connection.execute("PRAGMA user_version = 3")

    @contextlib.contextmanager
    def transaction(self) -> Iterator[sqlite3.Connection]:
        self.connection.execute("BEGIN IMMEDIATE")
        try:
            yield self.connection
        except BaseException:
            self.connection.rollback()
            raise
        else:
            self.connection.commit()

    def prune(
        self,
        current: float | None = None,
        dead_codex_sessions: Sequence[Identity] = (),
    ) -> None:
        timestamp = now_ts() if current is None else current
        dead_keys = {
            (identity.client, identity.session_id)
            for identity in dead_codex_sessions
            if identity.client == "codex"
        }
        with self.transaction() as connection:
            connection.execute(
                "DELETE FROM messages WHERE created_at < ?", (timestamp - MESSAGE_TTL,)
            )
            connection.execute("DELETE FROM notes WHERE created_at < ?", (timestamp - NOTE_TTL,))
            stale = connection.execute(
                """
                SELECT client, session_id FROM sessions
                WHERE client = 'codex' AND state = 'idle' AND last_seen < ?
                """,
                (timestamp - CODEX_IDLE_TTL,),
            ).fetchall()
            stale_keys = {(str(row["client"]), str(row["session_id"])) for row in stale} | dead_keys
            removed = False
            for client, session_id in stale_keys:
                session = connection.execute(
                    """
                    SELECT 1 FROM sessions
                    WHERE client = ? AND session_id = ? AND last_seen < ?
                    """,
                    (client, session_id, timestamp - CODEX_ORPHAN_GRACE),
                ).fetchone()
                if (client, session_id) in dead_keys and session is None:
                    continue
                connection.execute(
                    "DELETE FROM claims WHERE client = ? AND session_id = ?",
                    (client, session_id),
                )
                connection.execute(
                    "DELETE FROM delegates WHERE parent_client = ? AND parent_session_id = ?",
                    (client, session_id),
                )
                connection.execute(
                    "DELETE FROM sessions WHERE client = ? AND session_id = ?",
                    (client, session_id),
                )
                removed = True
            if removed:
                self._bump_generation(connection)

    def upsert_session(
        self,
        identity: Identity,
        *,
        cwd: str,
        repo_root: str | None,
        state: str,
        source: str,
        name: str | None = None,
        label: str | None = None,
        waiting_for: str | None = None,
        pid: int | None = None,
        process_started_at: float | None = None,
        started_at: float | None = None,
        current: float | None = None,
    ) -> None:
        timestamp = now_ts() if current is None else current
        with self.transaction() as connection:
            connection.execute(
                """
                INSERT INTO sessions(
                    client, session_id, cwd, repo_root, state, name, label, waiting_for,
                    pid, process_started_at, source, started_at, last_seen
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(client, session_id) DO UPDATE SET
                    cwd = excluded.cwd,
                    repo_root = excluded.repo_root,
                    state = excluded.state,
                    name = COALESCE(excluded.name, sessions.name),
                    label = COALESCE(excluded.label, sessions.label),
                    waiting_for = excluded.waiting_for,
                    pid = CASE
                        WHEN excluded.pid IS NULL THEN sessions.pid ELSE excluded.pid
                    END,
                    process_started_at = CASE
                        WHEN excluded.pid IS NULL THEN sessions.process_started_at
                        ELSE excluded.process_started_at
                    END,
                    source = excluded.source,
                    last_seen = excluded.last_seen
                """,
                (
                    identity.client,
                    identity.session_id,
                    cwd,
                    repo_root,
                    state,
                    name,
                    label,
                    waiting_for,
                    pid,
                    process_started_at,
                    source,
                    timestamp if started_at is None else started_at,
                    timestamp,
                ),
            )

    def set_session_label(self, identity: Identity, label: str | None) -> None:
        with self.transaction() as connection:
            connection.execute(
                "UPDATE sessions SET label = ? WHERE client = ? AND session_id = ?",
                (label, identity.client, identity.session_id),
            )

    def end_session(self, identity: Identity) -> None:
        with self.transaction() as connection:
            connection.execute(
                "DELETE FROM claims WHERE client = ? AND session_id = ?",
                (identity.client, identity.session_id),
            )
            connection.execute(
                "DELETE FROM delegates WHERE parent_client = ? AND parent_session_id = ?",
                (identity.client, identity.session_id),
            )
            connection.execute(
                "DELETE FROM sessions WHERE client = ? AND session_id = ?",
                (identity.client, identity.session_id),
            )
            self._bump_generation(connection)

    def replace_claude_sessions(self, rows: Sequence[dict[str, Any]], current: float) -> None:
        live_ids = {str(row["session_id"]) for row in rows}
        with self.transaction() as connection:
            existing = connection.execute(
                "SELECT session_id FROM sessions WHERE client = 'claude'"
            ).fetchall()
            for row in rows:
                connection.execute(
                    """
                    INSERT INTO sessions(
                        client, session_id, cwd, repo_root, state, name, label, waiting_for,
                        pid, process_started_at, source, started_at, last_seen
                    ) VALUES ('claude', ?, ?, ?, ?, ?, NULL, ?, ?, ?, 'observer', ?, ?)
                    ON CONFLICT(client, session_id) DO UPDATE SET
                        cwd = excluded.cwd,
                        repo_root = excluded.repo_root,
                        state = excluded.state,
                        name = excluded.name,
                        waiting_for = excluded.waiting_for,
                        pid = excluded.pid,
                        process_started_at = excluded.process_started_at,
                        source = excluded.source,
                        last_seen = excluded.last_seen
                    """,
                    (
                        row["session_id"],
                        row["cwd"],
                        row.get("repo_root"),
                        row["state"],
                        row.get("name"),
                        row.get("waiting_for"),
                        row.get("pid"),
                        row.get("process_started_at"),
                        row.get("started_at", current),
                        current,
                    ),
                )
            removed = False
            for row in existing:
                session_id = str(row["session_id"])
                if session_id not in live_ids:
                    connection.execute(
                        "DELETE FROM claims WHERE client = 'claude' AND session_id = ?",
                        (session_id,),
                    )
                    connection.execute(
                        "DELETE FROM sessions WHERE client = 'claude' AND session_id = ?",
                        (session_id,),
                    )
                    removed = True
            if removed:
                self._bump_generation(connection)

    def sessions(self) -> list[dict[str, Any]]:
        rows = self.connection.execute(
            "SELECT * FROM sessions ORDER BY client, started_at, session_id"
        ).fetchall()
        return [dict(row) for row in rows]

    def session(self, identity: Identity) -> dict[str, Any] | None:
        row = self.connection.execute(
            "SELECT * FROM sessions WHERE client = ? AND session_id = ?",
            (identity.client, identity.session_id),
        ).fetchone()
        return dict(row) if row else None

    def identities_for_processes(self, references: Sequence[ProcessReference]) -> list[Identity]:
        if not references:
            return []
        exact = tuple(
            (reference.pid, reference.started_at)
            for reference in references
            if reference.started_at is not None
        )
        rows: list[sqlite3.Row] = []
        if exact:
            predicate = " OR ".join("(pid = ? AND process_started_at = ?)" for _ in exact)
            rows = self.connection.execute(
                f"SELECT client, session_id FROM sessions WHERE {predicate}",
                tuple(value for reference in exact for value in reference),
            ).fetchall()
        if not rows:
            pids = tuple(dict.fromkeys(reference.pid for reference in references))
            placeholders = ",".join("?" for _ in pids)
            rows = self.connection.execute(
                f"""
                SELECT client, session_id FROM sessions
                WHERE process_started_at IS NULL AND pid IN ({placeholders})
                """,
                pids,
            ).fetchall()
        return [Identity(str(row["client"]), str(row["session_id"])) for row in rows]

    def claim(self, identity: Identity) -> dict[str, Any] | None:
        row = self.connection.execute(
            "SELECT * FROM claims WHERE client = ? AND session_id = ?",
            (identity.client, identity.session_id),
        ).fetchone()
        return self._claim_with_paths(row) if row else None

    def claims(self, repo_root: str | None = None) -> list[dict[str, Any]]:
        if repo_root is None:
            rows = self.connection.execute(
                "SELECT * FROM claims ORDER BY created_at, id"
            ).fetchall()
        else:
            rows = self.connection.execute(
                "SELECT * FROM claims WHERE repo_root = ? ORDER BY created_at, id", (repo_root,)
            ).fetchall()
        return [self._claim_with_paths(row) for row in rows]

    def _claim_with_paths(self, row: sqlite3.Row) -> dict[str, Any]:
        result = dict(row)
        paths = self.connection.execute(
            "SELECT path FROM claim_paths WHERE claim_id = ? ORDER BY path", (row["id"],)
        ).fetchall()
        result["paths"] = tuple(str(path["path"]) for path in paths)
        return result

    def save_claim(
        self,
        connection: sqlite3.Connection,
        identity: Identity,
        *,
        repo_root: str,
        label: str,
        state: str,
        paths: tuple[str, ...],
        blocked_reason: str | None,
        created_at: float,
        updated_at: float,
    ) -> int:
        connection.execute(
            """
            INSERT INTO claims(
                client, session_id, repo_root, label, state, blocked_reason, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(client, session_id) DO UPDATE SET
                repo_root = excluded.repo_root,
                label = excluded.label,
                state = excluded.state,
                blocked_reason = excluded.blocked_reason,
                updated_at = excluded.updated_at
            """,
            (
                identity.client,
                identity.session_id,
                repo_root,
                label,
                state,
                blocked_reason,
                created_at,
                updated_at,
            ),
        )
        claim_id = int(
            connection.execute(
                "SELECT id FROM claims WHERE client = ? AND session_id = ?",
                (identity.client, identity.session_id),
            ).fetchone()[0]
        )
        connection.execute("DELETE FROM claim_paths WHERE claim_id = ?", (claim_id,))
        connection.executemany(
            "INSERT INTO claim_paths(claim_id, path) VALUES (?, ?)",
            [(claim_id, path) for path in paths],
        )
        connection.execute(
            "UPDATE sessions SET label = ? WHERE client = ? AND session_id = ?",
            (label, identity.client, identity.session_id),
        )
        self._bump_generation(connection)
        return claim_id

    def delete_claim(self, identity: Identity) -> bool:
        with self.transaction() as connection:
            cursor = connection.execute(
                "DELETE FROM claims WHERE client = ? AND session_id = ?",
                (identity.client, identity.session_id),
            )
            connection.execute(
                "UPDATE sessions SET label = NULL WHERE client = ? AND session_id = ?",
                (identity.client, identity.session_id),
            )
            self._bump_generation(connection)
            return cursor.rowcount > 0

    def add_message(
        self,
        connection: sqlite3.Connection,
        sender: Identity,
        recipient: Identity,
        text: str,
        repo_root: str | None,
        current: float,
    ) -> str:
        message_id = new_id()
        connection.execute(
            """
            INSERT INTO messages(
                id, sender_client, sender_session_id, recipient_client,
                recipient_session_id, repo_root, text, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                message_id,
                sender.client,
                sender.session_id,
                recipient.client,
                recipient.session_id,
                repo_root,
                text,
                current,
            ),
        )
        connection.execute(
            """
            DELETE FROM messages WHERE id IN (
                SELECT id FROM messages
                WHERE recipient_client = ? AND recipient_session_id = ?
                ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?
            )
            """,
            (recipient.client, recipient.session_id, MAX_INBOX_MESSAGES),
        )
        self._bump_generation(connection)
        return message_id

    def send_message(
        self,
        sender: Identity,
        recipients: Sequence[Identity],
        text: str,
        repo_root: str | None,
        current: float | None = None,
    ) -> list[str]:
        timestamp = now_ts() if current is None else current
        with self.transaction() as connection:
            return [
                self.add_message(connection, sender, recipient, text, repo_root, timestamp)
                for recipient in recipients
            ]

    def inbox(self, identity: Identity, pending_only: bool = False) -> list[dict[str, Any]]:
        predicate = "AND acknowledged_at IS NULL" if pending_only else ""
        rows = self.connection.execute(
            f"""
            SELECT * FROM messages
            WHERE recipient_client = ? AND recipient_session_id = ? {predicate}
            ORDER BY created_at, id
            """,
            (identity.client, identity.session_id),
        ).fetchall()
        return [dict(row) for row in rows]

    def mark_unnotified(self, identity: Identity) -> int:
        """Mark unread messages as notified without waking coordination waiters."""
        with self.transaction() as connection:
            rows = connection.execute(
                """
                UPDATE messages SET notified_at = ?
                WHERE recipient_client = ? AND recipient_session_id = ?
                  AND acknowledged_at IS NULL AND notified_at IS NULL
                RETURNING id
                """,
                (now_ts(), identity.client, identity.session_id),
            ).fetchall()
            return len(rows)

    def acknowledge(self, identity: Identity, message_id: str | None = None) -> int:
        timestamp = now_ts()
        with self.transaction() as connection:
            if message_id is None:
                cursor = connection.execute(
                    """
                    UPDATE messages SET acknowledged_at = ?
                    WHERE recipient_client = ? AND recipient_session_id = ?
                      AND acknowledged_at IS NULL
                    """,
                    (timestamp, identity.client, identity.session_id),
                )
            else:
                cursor = connection.execute(
                    """
                    UPDATE messages SET acknowledged_at = ?
                    WHERE id = ? AND recipient_client = ? AND recipient_session_id = ?
                      AND acknowledged_at IS NULL
                    """,
                    (timestamp, message_id, identity.client, identity.session_id),
                )
            if cursor.rowcount:
                self._bump_generation(connection)
            return cursor.rowcount

    def add_note(self, identity: Identity, repo_root: str, text: str) -> str:
        note_id = new_id()
        with self.transaction() as connection:
            connection.execute(
                """
                INSERT INTO notes(
                    id, repo_root, author_client, author_session_id, text, created_at
                ) VALUES (?, ?, ?, ?, ?, ?)
                """,
                (note_id, repo_root, identity.client, identity.session_id, text, now_ts()),
            )
            self._bump_generation(connection)
        return note_id

    def notes(self, repo_root: str, since: float | None = None) -> list[dict[str, Any]]:
        if since is None:
            rows = self.connection.execute(
                """
                SELECT * FROM notes WHERE repo_root = ? AND resolved_at IS NULL
                ORDER BY created_at, id
                """,
                (repo_root,),
            ).fetchall()
        else:
            rows = self.connection.execute(
                """
                SELECT * FROM notes
                WHERE repo_root = ? AND resolved_at IS NULL AND created_at > ?
                ORDER BY created_at, id
                """,
                (repo_root, since),
            ).fetchall()
        return [dict(row) for row in rows]

    def resolve_note(self, repo_root: str, note_id: str) -> bool:
        with self.transaction() as connection:
            cursor = connection.execute(
                """
                UPDATE notes SET resolved_at = ?
                WHERE repo_root = ? AND id = ? AND resolved_at IS NULL
                """,
                (now_ts(), repo_root, note_id),
            )
            if cursor.rowcount:
                self._bump_generation(connection)
            return cursor.rowcount > 0

    def update_delegate(
        self, parent: Identity, agent_id: str, agent_type: str | None, state: str
    ) -> None:
        with self.transaction() as connection:
            if state == "ended":
                connection.execute(
                    """
                    DELETE FROM delegates
                    WHERE parent_client = ? AND parent_session_id = ? AND agent_id = ?
                    """,
                    (parent.client, parent.session_id, agent_id),
                )
            else:
                connection.execute(
                    """
                    INSERT INTO delegates(
                        parent_client, parent_session_id, agent_id, agent_type, state, last_seen
                    ) VALUES (?, ?, ?, ?, ?, ?)
                    ON CONFLICT(parent_client, parent_session_id, agent_id) DO UPDATE SET
                        agent_type = excluded.agent_type,
                        state = excluded.state,
                        last_seen = excluded.last_seen
                    """,
                    (parent.client, parent.session_id, agent_id, agent_type, state, now_ts()),
                )

    def delegates(self) -> list[dict[str, Any]]:
        return [dict(row) for row in self.connection.execute("SELECT * FROM delegates")]

    def hook_success(self, client: str, event: str) -> None:
        with self.transaction() as connection:
            connection.execute(
                """
                INSERT INTO hook_health(client, event, last_success_at)
                VALUES (?, ?, ?)
                ON CONFLICT(client, event) DO UPDATE SET
                    last_error_code = NULL,
                    last_error_at = NULL,
                    last_success_at = excluded.last_success_at
                """,
                (client, event, now_ts()),
            )

    def hook_error(self, client: str, event: str, code: str) -> None:
        with self.transaction() as connection:
            connection.execute(
                """
                INSERT INTO hook_health(client, event, last_error_code, last_error_at)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(client, event) DO UPDATE SET
                    last_error_code = excluded.last_error_code,
                    last_error_at = excluded.last_error_at
                """,
                (client, event, sanitize(code, 80), now_ts()),
            )

    def hook_health(self) -> list[dict[str, Any]]:
        return [dict(row) for row in self.connection.execute("SELECT * FROM hook_health")]

    def imported(self, source_path: str, content: bytes) -> bool:
        digest = hashlib.sha256(content).hexdigest()
        row = self.connection.execute(
            "SELECT 1 FROM imports WHERE source_path = ? AND content_hash = ?",
            (source_path, digest),
        ).fetchone()
        return row is not None

    def mark_imported(self, source_path: str, content: bytes) -> None:
        digest = hashlib.sha256(content).hexdigest()
        with self.transaction() as connection:
            connection.execute(
                """
                INSERT OR IGNORE INTO imports(source_path, content_hash, imported_at)
                VALUES (?, ?, ?)
                """,
                (source_path, digest, now_ts()),
            )

    def generation(self) -> int:
        return int(
            self.connection.execute(
                "SELECT value FROM metadata WHERE key = 'generation'"
            ).fetchone()[0]
        )

    @staticmethod
    def _bump_generation(connection: sqlite3.Connection) -> None:
        connection.execute("UPDATE metadata SET value = value + 1 WHERE key = 'generation'")
