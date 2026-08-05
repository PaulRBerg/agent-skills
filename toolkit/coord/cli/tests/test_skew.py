from __future__ import annotations

import json
import os
import sqlite3
import sys
from pathlib import Path

import pytest
from click.testing import CliRunner

import ai_coord.cli as cli_module
from ai_coord.schema import SchemaVersionError
from ai_coord.store import SCHEMA_VERSION, Store


def test_store_writes_runner_sidecar(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    state_dir = tmp_path / "state"
    monkeypatch.setenv("AI_COORD_STATE_DIR", str(state_dir))

    Store().close()

    assert json.loads((state_dir / "runner.json").read_text()) == {
        "schema": SCHEMA_VERSION,
        "argv": [os.path.abspath(sys.executable), "-m", "ai_coord"],
    }


def test_newer_state_schema_raises_structured_error(tmp_path: Path) -> None:
    path = tmp_path / "state.db"
    Store(path).close()
    connection = sqlite3.connect(path)
    connection.execute(f"PRAGMA user_version = {SCHEMA_VERSION + 1}")
    connection.commit()
    connection.close()

    with pytest.raises(SchemaVersionError) as raised:
        Store(path)

    assert (raised.value.found, raised.value.required, raised.value.path) == (
        SCHEMA_VERSION + 1,
        SCHEMA_VERSION,
        path,
    )


def test_store_does_not_downgrade_runner_sidecar(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    state_dir = tmp_path / "state"
    state_dir.mkdir()
    runner_path = state_dir / "runner.json"
    runner = {"schema": SCHEMA_VERSION + 1, "argv": ["/newer/runner"]}
    runner_path.write_text(json.dumps(runner))
    monkeypatch.setenv("AI_COORD_STATE_DIR", str(state_dir))

    Store().close()

    assert json.loads(runner_path.read_text()) == runner


@pytest.mark.parametrize(
    ("guard", "schema", "executable", "expected"),
    [
        (None, SCHEMA_VERSION, sys.executable, True),
        (None, SCHEMA_VERSION - 1, sys.executable, False),
        (None, SCHEMA_VERSION, "/missing/runner", False),
        ("1", SCHEMA_VERSION, sys.executable, False),
    ],
)
def test_reexec_argv_requires_guardless_compatible_runner(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    guard: str | None,
    schema: int,
    executable: str,
    expected: bool,
) -> None:
    if guard is None:
        monkeypatch.delenv("AI_COORD_REEXEC", raising=False)
    else:
        monkeypatch.setenv("AI_COORD_REEXEC", guard)
    argv = [executable, "-m", "ai_coord"]
    (tmp_path / "runner.json").write_text(json.dumps({"schema": schema, "argv": argv}))
    error = SchemaVersionError(
        SCHEMA_VERSION,
        SCHEMA_VERSION - 1,
        tmp_path / "state.db",
    )

    result = cli_module._reexec_argv(error, tmp_path)

    assert (result == argv) is expected


def test_reexec_does_not_run_for_an_older_ledger(tmp_path: Path) -> None:
    argv = [sys.executable, "-m", "ai_coord"]
    (tmp_path / "runner.json").write_text(json.dumps({"schema": SCHEMA_VERSION, "argv": argv}))
    error = SchemaVersionError(
        SCHEMA_VERSION - 1,
        SCHEMA_VERSION,
        tmp_path / "state.db",
    )

    assert cli_module._reexec_argv(error, tmp_path) is None


def test_malformed_runner_sidecar_surfaces_plain_error(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    state_dir = tmp_path / "state"
    state_dir.mkdir()
    (state_dir / "runner.json").write_text("not json")
    monkeypatch.setenv("AI_COORD_STATE_DIR", str(state_dir))
    monkeypatch.delenv("AI_COORD_REEXEC", raising=False)
    error = SchemaVersionError(
        SCHEMA_VERSION,
        SCHEMA_VERSION - 1,
        state_dir / "state.db",
    )
    monkeypatch.setattr(cli_module, "_coordinator", lambda: (_ for _ in ()).throw(error))

    result = CliRunner().invoke(cli_module.cli, ["done"])

    assert result.exit_code == 1
    assert result.output == f"error: {error}\n"
