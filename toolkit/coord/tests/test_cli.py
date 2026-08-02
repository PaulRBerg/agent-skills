from __future__ import annotations

import json
from pathlib import Path

import pytest
from click.testing import CliRunner

import ai_coord.cli as cli_module
from ai_coord.coordinator import Coordinator
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


def test_link_cli_dry_run(tmp_path: Path) -> None:
    path = tmp_path / "hooks.json"
    result = CliRunner().invoke(
        cli_module.cli,
        ["link", "codex", "--path", str(path), "--dry-run"],
    )
    assert result.exit_code == 0
    assert result.output.startswith("WOULD_UPDATE\tcodex")
    assert not path.exists()
