"""Command-line adapter for ai-coord."""

from __future__ import annotations

import json
import os
import re
import sys
from dataclasses import asdict
from pathlib import Path

import click

from ai_coord import __version__
from ai_coord.coordinator import Coordinator, snapshot_json
from ai_coord.integrations import default_hook_path, default_link_path, inspect_hooks, link_hooks
from ai_coord.migration import migrate_legacy
from ai_coord.store import SCHEMA_VERSION, Store
from ai_coord.util import age_label, private_state_dir

_NEWER_SCHEMA_ERROR = re.compile(r"^state schema (\d+) is newer than supported schema \d+$")


def _coordinator() -> Coordinator:
    return Coordinator(Store())


def _reexec_argv(error: Exception, state_dir: Path | None = None) -> list[str] | None:
    """Return a compatible runner command for a newer state schema, if available."""
    if os.environ.get("AI_COORD_REEXEC") or not isinstance(error, RuntimeError):
        return None
    match = _NEWER_SCHEMA_ERROR.fullmatch(str(error))
    if match is None:
        return None
    try:
        runner = json.loads(((state_dir or private_state_dir()) / "runner.json").read_text())
        schema = runner["schema"]
        argv = runner["argv"]
    except (OSError, KeyError, TypeError, ValueError):
        return None
    if (
        not isinstance(schema, int)
        or isinstance(schema, bool)
        or schema < int(match.group(1))
        or not isinstance(argv, list)
        or not argv
        or not all(isinstance(argument, str) for argument in argv)
    ):
        return None
    executable = Path(argv[0])
    if not executable.is_file() or not os.access(executable, os.X_OK):
        return None
    return argv


def _fail(error: Exception, code: int = 1) -> None:
    runner_argv = _reexec_argv(error)
    if runner_argv is not None:
        environment = os.environ.copy()
        environment["AI_COORD_REEXEC"] = "1"
        os.execve(runner_argv[0], [*runner_argv, *sys.argv[1:]], environment)
    click.echo(f"error: {error}", err=True)
    raise click.exceptions.Exit(code)


@click.group()
@click.version_option(version=__version__, prog_name="ai-coord")
def cli() -> None:
    """Coordinate parallel Codex and Claude Code agents."""


@cli.command()
@click.argument("label")
@click.argument("paths", nargs=-1)
def start(label: str, paths: tuple[str, ...]) -> None:
    """Acquire or queue exclusive work for literal repository paths."""
    try:
        coordinator = _coordinator()
        outcome = coordinator.start(label, paths)
        click.echo(outcome.line())
        raise click.exceptions.Exit(outcome.code)
    except click.exceptions.Exit:
        raise
    except ValueError as error:
        _fail(error, 64)
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
@click.option(
    "--timeout-seconds",
    "-t",
    type=click.IntRange(1, 3600),
    default=300,
    show_default=True,
)
def wait(timeout_seconds: int) -> None:
    """Wait for the caller's queued work to become ready."""
    try:
        outcome = _coordinator().wait(timeout_seconds)
        click.echo(outcome.line())
        raise click.exceptions.Exit(outcome.code)
    except click.exceptions.Exit:
        raise
    except ValueError as error:
        _fail(error, 64)
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
def done() -> None:
    """Release the caller's active, queued, or intent work."""
    try:
        outcome = _coordinator().done()
        click.echo(outcome.line())
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
def baseline() -> None:
    """Print Git blob baselines for the caller's active claim."""
    try:
        for row in _coordinator().baselines():
            click.echo(f"{row['path']}\t{row['oid']}")
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
@click.option("--all", "machine_wide", is_flag=True, help="Show machine-wide inventory")
@click.option("--json", "as_json", is_flag=True, help="Emit the versioned JSON schema")
def status(machine_wide: bool, as_json: bool) -> None:
    """Show active sessions, work claims, coverage, and repository notes."""
    try:
        coordinator = _coordinator()
        snapshot = coordinator.snapshot(machine_wide)
        click.echo(snapshot_json(snapshot) if as_json else coordinator.render_status(snapshot))
        if not snapshot.complete:
            raise click.exceptions.Exit(2)
    except click.exceptions.Exit:
        raise
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
@click.argument("target")
@click.argument("text")
def msg(target: str, text: str) -> None:
    """Send a bounded message to one session or the current repository."""
    try:
        ids, recipients = _coordinator().send(target, text)
        click.echo(f"SENT\t{recipients}\t{','.join(ids)}")
    except ValueError as error:
        _fail(error, 64)
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
@click.option("--ack", "message_id", help="Acknowledge one message ID")
@click.option("--ack-all", is_flag=True, help="Acknowledge all pending messages")
def inbox(message_id: str | None, ack_all: bool) -> None:
    """Read or acknowledge recipient-only messages."""
    if message_id and ack_all:
        _fail(ValueError("use only one of --ack or --ack-all"), 64)
    try:
        coordinator = _coordinator()
        if message_id or ack_all:
            count = coordinator.acknowledge(None if ack_all else message_id)
            click.echo(f"ACK\t{count}")
            return
        messages = coordinator.inbox()
        click.echo("ID\tAGE\tFROM\tTEXT")
        for row in messages:
            if row["acknowledged_at"] is not None:
                continue
            click.echo(
                "\t".join(
                    (
                        str(row["id"]),
                        age_label(float(row["created_at"])),
                        f"{row['sender_client']}/{str(row['sender_session_id'])[:8]}",
                        str(row["text"]),
                    )
                )
            )
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
@click.argument("text", required=False)
@click.option("--done", "note_id", help="Resolve one repository note")
def note(text: str | None, note_id: str | None) -> None:
    """Create or resolve a durable repository note."""
    if bool(text) == bool(note_id):
        _fail(ValueError("provide note text or --done ID"), 64)
    try:
        coordinator = _coordinator()
        if note_id:
            if not coordinator.resolve_note(note_id):
                _fail(RuntimeError(f"note not found: {note_id}"))
            click.echo(f"DONE\t{note_id}")
        else:
            assert text is not None
            click.echo(f"NOTE\t{coordinator.add_note(text)}")
    except click.exceptions.Exit:
        raise
    except ValueError as error:
        _fail(error, 64)
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
def trailer() -> None:
    """Print the current agent-session Git trailer."""
    try:
        click.echo(_coordinator().trailer())
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command(hidden=True)
@click.argument("client", type=click.Choice(["codex", "claude"]))
def hook(client: str) -> None:
    """Consume one host hook payload from stdin without blocking the host."""
    output = ""
    event_name = "unknown"
    try:
        payload = json.load(sys.stdin)
        if not isinstance(payload, dict):
            raise TypeError("hook input must be an object")
        raw_event = payload.get("hook_event_name")
        event_name = raw_event if isinstance(raw_event, str) else "unknown"
        output = _coordinator().ingest_hook(client, payload)
    except Exception:  # noqa: BLE001 - hooks must fail open even if state initialization fails
        if client == "codex" and event_name in {"Stop", "SubagentStop"}:
            output = "{}"
    if output:
        click.echo(output)


@cli.command(hidden=True)
@click.argument("client", type=click.Choice(["claude"]))
def waker(client: str) -> None:
    """Wake a Claude session when its queued coordination state changes."""
    try:
        payload = json.load(sys.stdin)
        if not isinstance(payload, dict):
            raise TypeError("waker input must be an object")
        outcome = _coordinator().waker(client, payload)
        if outcome is None:
            return
        click.echo(
            f"ai-coord: {outcome.kind} — re-run 'ai-coord start <label> <paths>' "
            "to confirm ownership before editing.",
            err=True,
        )
        raise click.exceptions.Exit(2)
    except click.exceptions.Exit:
        raise
    except Exception:  # noqa: BLE001 - waker hooks must fail open
        return


@cli.command()
@click.argument("client", type=click.Choice(["codex", "claude", "all"]))
@click.option("--path", type=click.Path(path_type=Path), help="Override one client's config path")
@click.option("--dry-run", is_flag=True, help="Inspect changes without writing")
@click.option("--force", is_flag=True, help="Replace malformed owned hook containers")
def link(client: str, path: Path | None, dry_run: bool, force: bool) -> None:
    """Install ai-coord lifecycle hooks while preserving unrelated hooks."""
    if client == "all" and path is not None:
        _fail(ValueError("--path is available only when linking one client"), 64)
    clients = ("codex", "claude") if client == "all" else (client,)
    try:
        for selected in clients:
            result = link_hooks(
                selected,
                path or default_link_path(selected),
                dry_run=dry_run,
                force=force,
            )
            state = (
                "WOULD_UPDATE"
                if dry_run and result.changed
                else "UPDATED"
                if result.changed
                else "OK"
            )
            click.echo(f"{state}\t{selected}\t{result.path}\tlegacy={result.removed_legacy}")
    except ValueError as error:
        _fail(error, 64)
    except Exception as error:  # noqa: BLE001
        _fail(error)


@cli.command()
@click.option("--json", "as_json", is_flag=True, help="Emit machine-readable diagnostics")
def check(as_json: bool) -> None:
    """Check installation, schema, hooks, providers, and hook health."""
    reports: list[dict[str, object]] = []
    broken = False
    degraded = False
    try:
        store = Store()
        reports.append(
            {
                "component": "state",
                "status": "ok",
                "path": str(store.path),
                "schema_version": SCHEMA_VERSION,
            }
        )
        paths = {selected: default_hook_path(selected) for selected in ("codex", "claude")}
        for selected, path in paths.items():
            report = inspect_hooks(selected, path)
            reports.append({"component": f"hooks:{selected}", **asdict(report)})
            degraded = degraded or not report.ok
        snapshot = Coordinator(store).snapshot(machine_wide=True)
        reports.extend(
            {"component": f"provider:{provider['client']}", **provider}
            for provider in snapshot.providers
        )
        degraded = degraded or not snapshot.complete
        for health in store.hook_health():
            if health["last_error_code"]:
                reports.append({"component": "hook-health", **health})
                degraded = True
    except Exception as error:  # noqa: BLE001
        reports.append({"component": "runtime", "status": "broken", "error": str(error)})
        broken = True
    if as_json:
        click.echo(json.dumps(reports, indent=2, sort_keys=True, default=str))
    else:
        for report in reports:
            status_value = report.get("status") or (
                "ok" if report.get("ok") is True else "degraded"
            )
            detail = report.get("error") or report.get("path") or ""
            click.echo(f"{str(status_value).upper()}\t{report['component']}\t{detail}")
    if broken:
        raise click.exceptions.Exit(1)
    if degraded:
        raise click.exceptions.Exit(2)


@cli.group()
def migrate() -> None:
    """Import state from retired coordination implementations."""


@migrate.command("legacy")
@click.option(
    "--source",
    type=click.Path(path_type=Path),
    default=default_hook_path("codex").parent / ".tmp" / "agent-session-status",
    show_default=True,
)
@click.option("--dry-run", is_flag=True, help="Count valid records without writing")
def migrate_legacy_command(source: Path, dry_run: bool) -> None:
    """Import the legacy AgentSessionStatus JSON registry."""
    if not source.is_dir():
        _fail(ValueError(f"legacy state directory not found: {source}"), 64)
    try:
        report = migrate_legacy(Store(), source, dry_run=dry_run)
        click.echo(json.dumps(report.as_dict(), sort_keys=True))
    except Exception as error:  # noqa: BLE001
        _fail(error)


if __name__ == "__main__":
    cli()
