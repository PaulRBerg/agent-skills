from __future__ import annotations

import json
from dataclasses import replace

import pytest

import ai_coord.status as status_module
from ai_coord.identity import Identity
from ai_coord.status import StatusSnapshot, render_status, snapshot_json


def _snapshot(**overrides: object) -> StatusSnapshot:
    base = StatusSnapshot(
        complete=True,
        scope={"kind": "repo", "repo_root": "/repo"},
        self_identity=Identity("codex", "self"),
        providers=({"client": "codex", "enabled": True, "ok": True, "dropped": 0},),
        sessions=(),
        claims=(),
        notes=(),
        delegates=(),
        outside_scope={"sessions": 0, "directories": 0},
    )
    return replace(base, **overrides)


def test_status_snapshot_json_keeps_the_public_schema() -> None:
    payload = json.loads(snapshot_json(_snapshot()))

    assert payload == {
        "claims": [],
        "complete": True,
        "delegates": [],
        "notes": [],
        "outside_scope": {"directories": 0, "sessions": 0},
        "providers": [{"client": "codex", "dropped": 0, "enabled": True, "ok": True}],
        "schema_version": 1,
        "scope": {"kind": "repo", "repo_root": "/repo"},
        "self": {"client": "codex", "session_id": "self"},
        "sessions": [],
    }


def test_status_rendering_groups_anonymous_sessions_and_keeps_named_sessions_visible(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(status_module, "now_ts", lambda: 2_000.0)
    anonymous = {
        "client": "codex",
        "state": "working",
        "last_seen": 1_000.0,
        "cwd": "/repo",
        "callsign": None,
        "name": None,
        "label": None,
    }
    sessions = (
        dict(anonymous, session_id="one"),
        dict(anonymous, session_id="two"),
        dict(anonymous, session_id="named", callsign="🦊 Fox", label="exact files"),
    )

    rendered = render_status(_snapshot(sessions=sessions))

    assert "\t🦊 Fox\texact files\tnamed\t/repo\t" in rendered
    assert "\tcount=2\t/repo\t" in rendered
    assert "\tone\t" not in rendered
    assert "\ttwo\t" not in rendered
