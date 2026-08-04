"""SQLite persistence for coordination state."""

from __future__ import annotations

import contextlib
import hashlib
import json
import os
import sqlite3
import sys
import tempfile
import time
from collections.abc import Iterator, Sequence
from pathlib import Path
from typing import Any

from ai_coord.identity import Identity, ProcessReference
from ai_coord.schema import SCHEMA_VERSION, migrate
from ai_coord.util import callsign_key, new_id, now_ts, private_state_dir, sanitize

CODEX_IDLE_TTL = 4 * 60 * 60
CODEX_ORPHAN_GRACE = 30 * 60
MESSAGE_TTL = 48 * 60 * 60
NOTE_TTL = 7 * 24 * 60 * 60
MAX_INBOX_MESSAGES = 50
MAX_ERROR_CODE_CHARS = 80


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
        # Keep SQLite's own wait short so _enable_wal's bounded retry loop arbitrates contention.
        self.connection.execute("PRAGMA busy_timeout = 250")
        self._enable_wal()
        self.connection.execute("PRAGMA busy_timeout = 5000")
        self.connection.execute("PRAGMA synchronous = NORMAL")
        try:
            self._migrate()
        except BaseException:
            # Release the database so a caller re-execing a compatible runner starts clean.
            self.connection.close()
            raise
        self.path.chmod(0o600)
        self._write_runner()

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
        migrate(self.connection)

    def _write_runner(self) -> None:
        runner_path = self.path.parent / "runner.json"
        desired = {
            "schema": SCHEMA_VERSION,
            "argv": [str(Path(sys.executable).resolve()), "-m", "ai_coord"],
        }
        try:
            existing = json.loads(runner_path.read_text())
            if existing == desired:
                return
            existing_schema = existing.get("schema") if isinstance(existing, dict) else 0
            if not isinstance(existing_schema, int) or isinstance(existing_schema, bool):
                existing_schema = 0
            if existing_schema > SCHEMA_VERSION:
                return
        except (OSError, ValueError, TypeError):
            pass

        temporary_path: Path | None = None
        try:
            with tempfile.NamedTemporaryFile(
                "w",
                dir=runner_path.parent,
                prefix=".runner.",
                suffix=".tmp",
                encoding="utf-8",
                delete=False,
            ) as temporary:
                temporary.write(json.dumps(desired))
                temporary_path = Path(temporary.name)
            temporary_path.replace(runner_path)
        except Exception:  # noqa: BLE001 - runner metadata is best effort
            if temporary_path is not None:
                with contextlib.suppress(OSError):
                    temporary_path.unlink()

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
                past_orphan_grace = (
                    connection.execute(
                        """
                        SELECT 1 FROM sessions
                        WHERE client = ? AND session_id = ? AND last_seen < ?
                        """,
                        (client, session_id, timestamp - CODEX_ORPHAN_GRACE),
                    ).fetchone()
                    is not None
                )
                if (client, session_id) in dead_keys and not past_orphan_grace:
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

    def set_session_callsign(self, identity: Identity, callsign: str) -> None:
        """Set a machine-wide unique callsign for one registered session."""
        desired_key = callsign_key(callsign)
        with self.transaction() as connection:
            current = connection.execute(
                "SELECT callsign FROM sessions WHERE client = ? AND session_id = ?",
                (identity.client, identity.session_id),
            ).fetchone()
            if current is None:
                raise RuntimeError("session is not registered")
            if current["callsign"] == callsign:
                return
            candidates = connection.execute(
                """
                SELECT callsign FROM sessions
                WHERE callsign IS NOT NULL AND NOT (client = ? AND session_id = ?)
                """,
                (identity.client, identity.session_id),
            ).fetchall()
            if any(callsign_key(str(row["callsign"])) == desired_key for row in candidates):
                raise ValueError("callsign is already in use")
            connection.execute(
                "UPDATE sessions SET callsign = ? WHERE client = ? AND session_id = ?",
                (callsign, identity.client, identity.session_id),
            )
            self._bump_generation(connection)

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

    def observe_dirt(
        self,
        repo_root: str,
        blob_hashes: dict[str, str],
        *,
        current: float | None = None,
    ) -> dict[str, dict[str, Any]]:
        timestamp = now_ts() if current is None else current
        with self.transaction() as connection:
            if blob_hashes:
                placeholders = ",".join("?" for _ in blob_hashes)
                connection.execute(
                    f"""
                    DELETE FROM dirt_observations
                    WHERE repo_root = ? AND path NOT IN ({placeholders})
                    """,
                    (repo_root, *blob_hashes.keys()),
                )
            else:
                connection.execute(
                    "DELETE FROM dirt_observations WHERE repo_root = ?", (repo_root,)
                )
            connection.executemany(
                """
                INSERT INTO dirt_observations(
                    repo_root, path, blob_hash, first_seen, last_seen
                ) VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(repo_root, path) DO UPDATE SET
                    blob_hash = excluded.blob_hash,
                    first_seen = CASE
                        WHEN dirt_observations.blob_hash = excluded.blob_hash
                        THEN dirt_observations.first_seen
                        ELSE excluded.first_seen
                    END,
                    last_seen = excluded.last_seen
                """,
                [
                    (repo_root, path, blob_hash, timestamp, timestamp)
                    for path, blob_hash in blob_hashes.items()
                ],
            )
            rows = connection.execute(
                """
                SELECT * FROM dirt_observations
                WHERE repo_root = ? ORDER BY path
                """,
                (repo_root,),
            ).fetchall()
        return {str(row["path"]): dict(row) for row in rows}

    def dirt_observations(self, repo_root: str) -> list[dict[str, Any]]:
        rows = self.connection.execute(
            """
            SELECT * FROM dirt_observations
            WHERE repo_root = ? ORDER BY path
            """,
            (repo_root,),
        ).fetchall()
        return [dict(row) for row in rows]

    def residual_owners(self, repo_root: str) -> dict[str, dict[str, Any]]:
        rows = self.connection.execute(
            """
            SELECT * FROM residual_owners
            WHERE repo_root = ? ORDER BY path
            """,
            (repo_root,),
        ).fetchall()
        return {str(row["path"]): dict(row) for row in rows}

    def record_residual_owners(
        self,
        repo_root: str,
        paths: tuple[str, ...],
        identity: Identity,
        *,
        current: float | None = None,
    ) -> None:
        if not paths:
            return
        timestamp = now_ts() if current is None else current
        with self.transaction() as connection:
            connection.executemany(
                """
                INSERT INTO residual_owners(
                    repo_root, path, client, session_id, released_at
                ) VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(repo_root, path) DO UPDATE SET
                    client = excluded.client,
                    session_id = excluded.session_id,
                    released_at = excluded.released_at
                """,
                [
                    (repo_root, path, identity.client, identity.session_id, timestamp)
                    for path in paths
                ],
            )

    def replace_baselines(self, identity: Identity, baselines: dict[str, str]) -> None:
        with self.transaction() as connection:
            row = connection.execute(
                """
                SELECT id FROM claims
                WHERE client = ? AND session_id = ? AND state = 'active'
                """,
                (identity.client, identity.session_id),
            ).fetchone()
            if row is None:
                return
            claim_id = int(row["id"])
            connection.execute("DELETE FROM claim_baselines WHERE claim_id = ?", (claim_id,))
            connection.executemany(
                "INSERT INTO claim_baselines(claim_id, path, oid) VALUES (?, ?, ?)",
                [(claim_id, path, oid) for path, oid in baselines.items()],
            )

    def baselines(self, identity: Identity) -> list[dict[str, str]]:
        rows = self.connection.execute(
            """
            SELECT claim_baselines.path, claim_baselines.oid
            FROM claim_baselines
            JOIN claims ON claims.id = claim_baselines.claim_id
            WHERE claims.client = ? AND claims.session_id = ? AND claims.state = 'active'
            ORDER BY claim_baselines.path
            """,
            (identity.client, identity.session_id),
        ).fetchall()
        return [dict(row) for row in rows]

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
        sender_row = connection.execute(
            "SELECT callsign FROM sessions WHERE client = ? AND session_id = ?",
            (sender.client, sender.session_id),
        ).fetchone()
        recipient_row = connection.execute(
            "SELECT callsign FROM sessions WHERE client = ? AND session_id = ?",
            (recipient.client, recipient.session_id),
        ).fetchone()
        connection.execute(
            """
            INSERT INTO messages(
                id, sender_client, sender_session_id, sender_callsign, recipient_client,
                recipient_session_id, recipient_callsign, repo_root, text, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                message_id,
                sender.client,
                sender.session_id,
                sender_row["callsign"] if sender_row else None,
                recipient.client,
                recipient.session_id,
                recipient_row["callsign"] if recipient_row else None,
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

    def all_messages(self) -> list[dict[str, Any]]:
        """Return every message in chronological order for local status consumers."""
        rows = self.connection.execute(
            """
            SELECT
                id, sender_client, sender_session_id, sender_callsign, recipient_client,
                recipient_session_id, recipient_callsign, repo_root, text, created_at,
                acknowledged_at
            FROM messages
            ORDER BY created_at, id
            """
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
                (client, event, sanitize(code, MAX_ERROR_CODE_CHARS), now_ts()),
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
