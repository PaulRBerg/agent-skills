from __future__ import annotations

import json
from dataclasses import dataclass
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


def test_cli_status_labels_planning_sessions_and_delegate_counts(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = Coordinator(Store(tmp_path / "state.db"), StaticInventory())
    identity = Identity("codex", "planning-session")
    monkeypatch.setattr(cli_module, "_coordinator", lambda: coordinator)
    monkeypatch.setenv("AI_COORD_CLIENT", identity.client)
    monkeypatch.setenv("AI_COORD_SESSION_ID", identity.session_id)
    monkeypatch.chdir(git_repo)
    coordinator.ingest_hook(
        identity.client,
        {
            "session_id": identity.session_id,
            "cwd": str(git_repo),
            "hook_event_name": "SessionStart",
            "permission_mode": "plan",
        },
    )
    coordinator.store.update_delegate(identity, "child-1", "explorer", "active")
    runner = CliRunner()

    human = runner.invoke(cli_module.cli, ["status"])

    assert human.exit_code == 0
    assert "planning delegates=1" in human.output

    machine = runner.invoke(cli_module.cli, ["status", "--json"])
    payload = json.loads(machine.output)
    assert machine.exit_code == 0
    assert payload["schema_version"] == 1
    assert payload["sessions"][0]["permission_mode"] == "plan"
    assert payload["sessions"][0]["delegate_count"] == 1


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


def test_cli_name_validation_uniqueness_and_inbox_snapshot(
    tmp_path: Path,
    git_repo: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    coordinator = Coordinator(Store(tmp_path / "state.db"), StaticInventory())
    monkeypatch.setattr(cli_module, "_coordinator", lambda: coordinator)
    monkeypatch.chdir(git_repo)
    runner = CliRunner()
    monkeypatch.setenv("AI_COORD_CLIENT", "codex")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "sender-session")

    named = runner.invoke(cli_module.cli, ["name", "  🦊   Fox One  "])
    assert named.exit_code == 0
    assert named.output == "NAMED\t🦊 Fox One\n"
    generation = coordinator.store.generation()
    repeated = runner.invoke(cli_module.cli, ["name", "🦊 Fox One"])
    assert repeated.exit_code == 0
    assert coordinator.store.generation() == generation
    invalid = runner.invoke(cli_module.cli, ["name", "no emoji"])
    assert invalid.exit_code == 64
    assert invalid.output == "error: callsign must contain at least one emoji\n"

    monkeypatch.setenv("AI_COORD_CLIENT", "claude")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "recipient-session")
    assert runner.invoke(cli_module.cli, ["name", "🐙 Octo Two"]).exit_code == 0
    duplicate = runner.invoke(cli_module.cli, ["name", "🦊 fox one"])
    assert duplicate.exit_code == 64
    assert duplicate.output == "error: callsign is already in use\n"

    monkeypatch.setenv("AI_COORD_CLIENT", "codex")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "sender-session")
    sent = runner.invoke(cli_module.cli, ["msg", "🐙 Octo Two", "snapshot me"])
    assert sent.exit_code == 0
    assert runner.invoke(cli_module.cli, ["name", "🦝 New Fox"]).exit_code == 0
    monkeypatch.setenv("AI_COORD_CLIENT", "claude")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "recipient-session")

    inbox = runner.invoke(cli_module.cli, ["inbox"])

    assert inbox.exit_code == 0
    assert "\t🦊 Fox One\tsnapshot me\n" in inbox.output
    assert "New Fox" not in inbox.output


def test_cli_name_requires_git_worktree(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    coordinator = Coordinator(Store(tmp_path / "state.db"), StaticInventory())
    monkeypatch.setattr(cli_module, "_coordinator", lambda: coordinator)
    monkeypatch.setenv("AI_COORD_CLIENT", "codex")
    monkeypatch.setenv("AI_COORD_SESSION_ID", "session")
    monkeypatch.chdir(tmp_path)

    result = CliRunner().invoke(cli_module.cli, ["name", "🧭 Lost One"])

    assert result.exit_code == 1
    assert result.output == "error: name requires a Git worktree\n"


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
    payload = {"hook_event_name": "PostToolUseFailure", "session_id": "active-session"}
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


def test_link_cli_reports_dry_run_then_update_then_noop(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "codex" / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(path.parent))
    trust_calls: list[Path] = []

    def trust_hook_path(trusted_path: Path) -> str:
        trust_calls.append(trusted_path)
        return "updated" if len(trust_calls) == 1 else "unchanged"

    monkeypatch.setattr(cli_module, "trust_codex_hooks", trust_hook_path)
    runner = CliRunner()
    supplied_path = path.parent / "not-a-directory" / ".." / "hooks.json"

    preview = runner.invoke(
        cli_module.cli, ["link", "codex", "--path", str(supplied_path), "--dry-run"]
    )
    assert preview.exit_code == 0
    assert preview.output == f"WOULD_UPDATE\tcodex\t{path}\tlegacy=0\ttrust=skipped\n"
    assert not path.exists()
    assert trust_calls == []

    applied = runner.invoke(cli_module.cli, ["link", "codex", "--path", str(supplied_path)])
    assert applied.exit_code == 0
    assert applied.output == f"UPDATED\tcodex\t{path}\tlegacy=0\ttrust=updated\n"

    repeated = runner.invoke(cli_module.cli, ["link", "codex", "--path", str(supplied_path)])
    assert repeated.exit_code == 0
    assert repeated.output == f"OK\tcodex\t{path}\tlegacy=0\ttrust=unchanged\n"
    assert trust_calls == [path, path]

    unverified = runner.invoke(
        cli_module.cli, ["link", "codex", "--path", str(supplied_path), "--dry-run"]
    )
    assert unverified.exit_code == 0
    assert unverified.output == f"WOULD_UPDATE\tcodex\t{path}\tlegacy=0\ttrust=skipped\n"
    assert trust_calls == [path, path]


def test_link_codex_path_must_resolve_to_active_hooks_file(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    codex_home = tmp_path / "codex"
    invalid_path = tmp_path / "other" / "hooks.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    monkeypatch.setattr(
        cli_module,
        "trust_codex_hooks",
        lambda _path: pytest.fail("Codex trust must not run for a rejected path"),
    )

    result = CliRunner().invoke(cli_module.cli, ["link", "codex", "--path", str(invalid_path)])

    assert result.exit_code == 64
    assert result.output == (
        f"error: --path for codex must be the active hooks file: {codex_home / 'hooks.json'}\n"
    )
    assert not invalid_path.exists()


def test_link_claude_reports_skipped_trust(tmp_path: Path) -> None:
    path = tmp_path / "claude" / "settings.json"
    runner = CliRunner()

    applied = runner.invoke(cli_module.cli, ["link", "claude", "--path", str(path)])
    assert applied.exit_code == 0
    assert applied.output == f"UPDATED\tclaude\t{path}\tlegacy=0\ttrust=skipped\n"

    repeated = runner.invoke(cli_module.cli, ["link", "claude", "--path", str(path)])
    assert repeated.exit_code == 0
    assert repeated.output == f"OK\tclaude\t{path}\tlegacy=0\ttrust=skipped\n"


def test_link_all_stops_before_claude_when_codex_trust_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    codex_home = tmp_path / "codex"
    claude_home = tmp_path / "claude"
    codex_hooks = codex_home / "hooks.json"
    claude_settings = claude_home / "settings.json"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(claude_home))

    def trust_failure(_path: Path) -> str:
        raise RuntimeError("Codex app-server trust failed")

    monkeypatch.setattr(cli_module, "trust_codex_hooks", trust_failure)

    result = CliRunner().invoke(cli_module.cli, ["link", "all"])

    assert result.exit_code == 1
    assert result.output == "error: Codex app-server trust failed\n"
    assert codex_hooks.exists()
    assert not claude_settings.exists()


def test_check_reports_hook_health_codes_and_exits_degraded(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    @dataclass(frozen=True)
    class TrustCheck:
        ok: bool
        path: Path
        error: str | None
        details: dict[str, str]

    hooks_path = tmp_path / "hooks.json"
    monkeypatch.setenv("AI_COORD_STATE_DIR", str(tmp_path / "state"))
    monkeypatch.setattr(cli_module, "default_hook_path", lambda _client: hooks_path)
    monkeypatch.setattr(
        cli_module,
        "inspect_codex_hook_trust",
        lambda _path: TrustCheck(
            False, hooks_path, "owned hook is untrusted", {"reason": "untrusted"}
        ),
    )
    monkeypatch.setattr(
        cli_module, "Coordinator", lambda store: Coordinator(store, StaticInventory())
    )
    store = Store()
    store.hook_error("codex", "Stop", "boom")
    store.close()
    runner = CliRunner()

    result = runner.invoke(cli_module.cli, ["check"])

    assert result.exit_code == 2
    assert "DEGRADED\thooks:codex" in result.output
    assert "DEGRADED\thooks-trust:codex\towned hook is untrusted\n" in result.output
    assert "DEGRADED\thook-health\tcodex/Stop: boom\n" in result.output

    as_json = runner.invoke(cli_module.cli, ["check", "--json"])

    assert as_json.exit_code == 2
    health = next(
        report for report in json.loads(as_json.output) if report["component"] == "hook-health"
    )
    assert health["client"] == "codex"
    assert health["event"] == "Stop"
    assert health["last_error_code"] == "boom"
    trust = next(
        report
        for report in json.loads(as_json.output)
        if report["component"] == "hooks-trust:codex"
    )
    assert trust["ok"] is False
    assert trust["error"] == "owned hook is untrusted"
    assert trust["details"] == {"reason": "untrusted"}
