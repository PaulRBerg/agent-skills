from __future__ import annotations

from ai_coord.providers import normalize_claude_sessions


def test_normalize_claude_sessions_reports_dropped_and_unknown() -> None:
    rows, dropped = normalize_claude_sessions(
        [
            {
                "sessionId": "one",
                "cwd": "/tmp",
                "state": "busy",
                "startedAt": 1_700_000_000_000,
                "pid": 42,
                "name": "worker",
            },
            {
                "id": "two",
                "cwd": "/tmp",
                "status": "novel",
                "startedAt": "2026-08-02T00:00:00Z",
            },
            {"id": "nan", "cwd": "/tmp", "status": "working", "startedAt": float("nan")},
            {"bad": True},
            {"id": "done", "cwd": "/tmp", "status": "completed", "startedAt": 1},
        ]
    )
    assert dropped == 2
    assert [row["state"] for row in rows] == ["working", "unknown"]
    assert rows[0]["pid"] == 42
