from __future__ import annotations

import json
from pathlib import Path

import pytest
from click.testing import CliRunner

import ai_coord.cli as cli_module
import ai_coord.coordinator as coordinator_module
from ai_coord.coordinator import Coordinator
from ai_coord.identity import Identity
from ai_coord.providers import StaticInventory
from ai_coord.store import Store


def test_cli_start_status_done(
    tmp_path: Path, git_repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    coordinator = Coordinator(Store(tmp_path / "state.db"), StaticInventory())
    monkeypatch.setattr(cli_module, "_coordinator", lambda: coordinator)
    monkeypatch.setenv("AI_COORD_CLIENT", "codex")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "cli-session")
    monkeypatch.chdir(git_repo)
    runner = CliRunner()

    start = runner.invoke(cli_module.cli, ["start", "cli work", "src"])
    assert start.exit_code == 0
    assert start.output == "READY\tsrc\n"

    status = runner.invoke(cli_module.cli, ["status", "--json"])
    assert status.exit_code == 0
    payload = json.loads(status.output)
    assert payload["schema_version"] == 1
    assert payload["claims"][0]["state"] == "active"

    done = runner.invoke(cli_module.cli, ["done"])
    assert done.exit_code == 0
    assert done.output == "DONE\treleased\n"


def test_cli_usage_and_trailer(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    coordinator = Coordinator(Store(tmp_path / "state.db"), StaticInventory())
    monkeypatch.setattr(cli_module, "_coordinator", lambda: coordinator)
    monkeypatch.setenv("AI_COORD_CLIENT", "claude")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "session-123")
    runner = CliRunner()
    trailer = runner.invoke(cli_module.cli, ["trailer"])
    assert trailer.exit_code == 0
    assert trailer.output == "Agent-Session: claude/session-123\n"
    note = runner.invoke(cli_module.cli, ["note"])
    assert note.exit_code == 64
    assert "provide note text" in note.output


def test_hook_cli_is_fail_open(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("AI_COORD_STATE_DIR", str(tmp_path / "state"))
    runner = CliRunner()
    malformed = runner.invoke(cli_module.cli, ["hook", "codex"], input="not json")
    assert malformed.exit_code == 0
    assert malformed.output == ""
    stop = runner.invoke(
        cli_module.cli,
        ["hook", "codex"],
        input=json.dumps({"hook_event_name": "Stop"}),
    )
    assert stop.exit_code == 0
    assert stop.output == "{}\n"


def test_waker_cli_is_silent_unless_a_queued_claim_wakes(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = Coordinator(Store(tmp_path / "state.db"), StaticInventory())
    monkeypatch.setattr(cli_module, "_coordinator", lambda: coordinator)
    runner = CliRunner()
    malformed = runner.invoke(cli_module.cli, ["waker", "claude"], input="not json")
    assert malformed.exit_code == 0
    assert malformed.output == ""

    monkeypatch.setenv("AI_COORD_CLIENT", "claude")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "active-session")
    assert coordinator.start("active", ("docs",), cwd=git_repo).kind == "READY"
    payload = {"hook_event_name": "PostToolUse", "session_id": "active-session"}
    inactive = runner.invoke(cli_module.cli, ["waker", "claude"], input=json.dumps(payload))
    assert inactive.exit_code == 0
    assert inactive.output == ""

    monkeypatch.setenv("AI_COORD_SESSION_ID", "holder-session")
    assert coordinator.start("holder", ("src",), cwd=git_repo).kind == "READY"
    monkeypatch.setenv("AI_COORD_SESSION_ID", "waiter-session")
    assert coordinator.start("waiter", ("src/app.py",), cwd=git_repo).kind == "BLOCKED"
    current = 0.0
    released = False

    def monotonic() -> float:
        return current

    def sleep(seconds: float) -> None:
        nonlocal current, released
        current += seconds
        if not released:
            coordinator.store.delete_claim(Identity("claude", "holder-session"))
            released = True

    monkeypatch.setattr(coordinator_module.time, "monotonic", monotonic)
    monkeypatch.setattr(coordinator_module.time, "sleep", sleep)
    payload["session_id"] = "waiter-session"

    promoted = runner.invoke(cli_module.cli, ["waker", "claude"], input=json.dumps(payload))

    assert promoted.exit_code == 2
    assert promoted.output == (
        "ai-coord: READY — re-run 'ai-coord start <label> <paths>' "
        "to confirm ownership before editing.\n"
    )


def test_link_cli_dry_run(tmp_path: Path) -> None:
    path = tmp_path / "hooks.json"
    result = CliRunner().invoke(
        cli_module.cli,
        ["link", "codex", "--path", str(path), "--dry-run"],
    )
    assert result.exit_code == 0
    assert result.output.startswith("WOULD_UPDATE\tcodex")
    assert not path.exists()
