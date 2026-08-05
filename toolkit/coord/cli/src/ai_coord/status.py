"""Pure status snapshot serialization and human-readable rendering."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from ai_coord.identity import Identity
from ai_coord.util import age_label, now_ts


@dataclass(frozen=True, slots=True)
class StatusSnapshot:
    complete: bool
    scope: dict[str, Any]
    self_identity: Identity | None
    providers: tuple[dict[str, Any], ...]
    sessions: tuple[dict[str, Any], ...]
    claims: tuple[dict[str, Any], ...]
    notes: tuple[dict[str, Any], ...]
    delegates: tuple[dict[str, Any], ...]
    outside_scope: dict[str, int]
    schema_version: int = 1

    def as_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "complete": self.complete,
            "scope": self.scope,
            "self": (
                {
                    "client": self.self_identity.client,
                    "session_id": self.self_identity.session_id,
                }
                if self.self_identity
                else None
            ),
            "providers": list(self.providers),
            "sessions": list(self.sessions),
            "claims": list(self.claims),
            "notes": list(self.notes),
            "delegates": list(self.delegates),
            "outside_scope": self.outside_scope,
        }


def render_status(snapshot: StatusSnapshot) -> str:
    lines = ["CLIENT\tSTATE\tAGE\tCALLSIGN\tNAME/LABEL\tSESSION\tCWD\tDETAIL"]
    anonymous: dict[tuple[str, str, str], list[dict[str, Any]]] = {}
    named: list[dict[str, Any]] = []
    for row in snapshot.sessions:
        if (
            row.get("callsign")
            or row.get("name")
            or row.get("label")
            or row.get("permission_mode") == "plan"
            or row.get("delegate_count")
        ):
            named.append(row)
        else:
            key = (str(row["client"]), str(row["state"]), str(row["cwd"]))
            anonymous.setdefault(key, []).append(row)
    for row in named:
        lines.append(_session_line(row))
    for rows in anonymous.values():
        if len(rows) == 1:
            lines.append(_session_line(rows[0]))
        else:
            row = dict(rows[0])
            row["session_id"] = f"count={len(rows)}"
            lines.append(_session_line(row))
    coverage = "; ".join(
        f"{provider['client']}={_coverage_label(provider)}" for provider in snapshot.providers
    )
    lines.append(f"Coverage: {coverage}")
    if snapshot.outside_scope["sessions"]:
        lines.append(
            f"Other directories: {snapshot.outside_scope['sessions']} reported sessions across "
            f"{snapshot.outside_scope['directories']} working directories."
        )
    if snapshot.notes:
        machine_wide = snapshot.scope["kind"] == "machine"
        note_scope = "machine-wide" if machine_wide else snapshot.scope.get("repo_root", "")
        lines.append(f"Notes ({note_scope}):")
        for note in snapshot.notes:
            prefix = f"{note['repo_root']}  " if machine_wide else ""
            lines.append(
                f"{prefix}{note['id']}  {age_label(float(note['created_at']))}  {note['text']}"
            )
        lines.append("(note --done <id> closes a note)")
    states = {str(row["state"]) for row in snapshot.sessions}
    partial = not snapshot.complete or any(
        not provider["enabled"] or not provider["ok"] or provider["dropped"]
        for provider in snapshot.providers
    )
    stale = any(
        row["last_seen"] < now_ts() - 1800 and row["state"] in {"working", "in_flight"}
        for row in snapshot.sessions
    )
    legends = (
        ("Idle: user prompt; dirt may remain in flight (Codex ~4h).", "idle" in states),
        ("Waiting: host/human wait; claim=queued means coordination queue.", "waiting" in states),
        ("Working/in_flight older than ~30m: likely stale.", stale),
        (
            "Names/labels: hints; only 'ai-coord start' returning READY grants an edit scope.",
            True,
        ),
        ("Partial coverage: sessions may be missing; absence does not mean no conflicts.", partial),
    )
    lines.extend(line for line, present in legends if present)
    return "\n".join(lines)


def _coverage_label(provider: dict[str, Any]) -> str:
    if not provider["enabled"]:
        return "disabled"
    return "ok" if provider["ok"] and not provider["dropped"] else "partial"


def _session_line(row: dict[str, Any]) -> str:
    detail: list[str] = []
    if row.get("permission_mode") == "plan":
        detail.append("planning")
    if row.get("delegate_count"):
        detail.append(f"delegates={row['delegate_count']}")
    if row.get("claim_state") == "queued":
        detail.append("claim=queued")
    if row.get("waiting_for"):
        detail.append(f"waiting={row['waiting_for']}")
    if row.get("paths"):
        detail.append(f"paths={','.join(row['paths'])}")
    name = row.get("label") or row.get("name") or ""
    return "\t".join(
        (
            str(row["client"]),
            str(row["state"]),
            age_label(float(row["last_seen"])),
            str(row.get("callsign") or ""),
            str(name),
            str(row["session_id"]),
            str(row["cwd"]),
            " ".join(detail),
        )
    )


def snapshot_json(snapshot: StatusSnapshot) -> str:
    return json.dumps(snapshot.as_dict(), indent=2, sort_keys=True)
