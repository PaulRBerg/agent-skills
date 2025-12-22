#!/usr/bin/env python3
"""
ai-notify CLI - Command-line interface for managing ai-notify.
"""

import json
import os
import subprocess
import sys
from datetime import datetime, timedelta
from pathlib import Path

import click
from loguru import logger
from tabulate import tabulate

from ai_notify.codex_config import set_codex_notify
from ai_notify.claude_hooks import ensure_claude_hooks
from ai_notify.config_loader import ConfigLoader, DEFAULT_EXPORT_DIR
from ai_notify.database import SessionTracker
from ai_notify.events import (
    handle_ask_user_question,
    handle_codex_notify,
    handle_notification,
    handle_permission,
    handle_stop,
    handle_user_prompt,
)
from ai_notify.integrations import inspect_claude_hooks, inspect_codex_notify
from ai_notify.notifier import MacNotifier
from ai_notify.utils import setup_logging, read_stdin_json, validate_input


def path_with_tilde(path: Path) -> str:
    """Convert path to string, replacing home directory with ~."""
    home = str(Path.home())
    path_str = str(path)
    return "~" + path_str[len(home) :] if path_str.startswith(home) else path_str


@click.group()
@click.version_option(version="1.0.0", prog_name="ai-notify")
def cli():
    """ai-notify - Notification hook for Claude Code and Codex CLI."""
    setup_logging()


@cli.group()
def config():
    """Manage ai-notify configuration."""
    pass


@config.command("show")
@click.option(
    "--path", type=click.Path(exists=True, path_type=Path), help="Custom config file path"
)
def config_show(path):
    """Show current configuration."""
    try:
        loader = ConfigLoader(path)
        cfg = loader.load()

        # Build configuration table
        config_data = [
            ["App Bundle", cfg.notification.app_bundle],
            ["Exclude Patterns", ", ".join(cfg.notification.exclude_patterns) or "None"],
            ["Notification Mode", cfg.notification.mode.value],
            ["Notification Sound", cfg.notification.sound],
            ["Notification Threshold", f"{cfg.notification.threshold_seconds}s"],
            ["Database Path", path_with_tilde(cfg.database.path)],
            ["Retention Days", f"{cfg.cleanup.retention_days} days"],
            ["Auto-cleanup Enabled", "Yes" if cfg.cleanup.auto_cleanup_enabled else "No"],
            ["Export Before Cleanup", "Yes" if cfg.cleanup.export_before_cleanup else "No"],
            ["Log Level", cfg.logging.level],
            ["Log Path", path_with_tilde(cfg.logging.path)],
        ]

        click.echo("\n" + click.style("Current Configuration:", bold=True))
        click.echo(tabulate(config_data, headers=["Setting", "Value"], tablefmt="simple"))
        click.echo(
            click.style("\nHint: macOS sounds are located at /System/Library/Sounds", dim=True)
        )
        click.echo(f"\nConfig file: {path_with_tilde(loader.config_path)}")

        if not loader.config_path.exists():
            click.echo(
                click.style(
                    "\nNote: Using default configuration (no config file found)", fg="yellow"
                )
            )

    except Exception as e:
        click.echo(click.style(f"Error loading configuration: {e}", fg="red"), err=True)
        sys.exit(1)


@config.command("edit")
@click.option("--path", type=click.Path(path_type=Path), help="Custom config file path")
def config_edit(path):
    """Edit configuration file in $EDITOR."""
    try:
        loader = ConfigLoader(path)
        config_path = loader.config_path

        # Create config file if it doesn't exist
        if not config_path.exists():
            click.echo(f"Creating new config file at {config_path}...")
            loader.save(loader.load())

        # Get editor from environment
        editor = os.getenv("EDITOR", "vi")

        # Open editor
        click.echo(f"Opening {config_path} in {editor}...")
        subprocess.run([editor, str(config_path)])

        # Validate after editing
        try:
            loader.load()
            click.echo(click.style("✓ Configuration is valid", fg="green"))
        except Exception as e:
            click.echo(click.style(f"⚠ Warning: Configuration validation failed: {e}", fg="yellow"))

    except Exception as e:
        click.echo(click.style(f"Error editing configuration: {e}", fg="red"), err=True)
        sys.exit(1)


@config.command("reset")
@click.option("--path", type=click.Path(path_type=Path), help="Custom config file path")
@click.confirmation_option(prompt="Are you sure you want to reset configuration to defaults?")
def config_reset(path):
    """Reset configuration to defaults."""
    try:
        loader = ConfigLoader(path)
        loader.reset_to_defaults()
        click.echo(
            click.style(f"✓ Configuration reset to defaults: {loader.config_path}", fg="green")
        )
    except Exception as e:
        click.echo(click.style(f"Error resetting configuration: {e}", fg="red"), err=True)
        sys.exit(1)


@cli.command()
def test():
    """Test notification system."""
    try:
        click.echo("Sending test notification...")

        notifier = MacNotifier()
        notifier.notify_job_done(
            project_name="ai-notify-test", job_number=99, duration_str="1m 23s"
        )

        click.echo(click.style("✓ Test notification sent successfully", fg="green"))
        click.echo("\nIf you didn't see a notification, check:")
        click.echo("  1. System notification permissions")
        click.echo("  2. Do Not Disturb mode")
        click.echo("  3. Log file for errors")

    except Exception as e:
        click.echo(click.style(f"✗ Test notification failed: {e}", fg="red"), err=True)
        logger.exception("Test notification failed")
        sys.exit(1)


@cli.group("codex", invoke_without_command=True)
@click.option("--stdin", "use_stdin", is_flag=True, help="Read JSON payload from stdin")
@click.argument("payload", required=False)
@click.pass_context
def codex(ctx, use_stdin, payload):
    """Codex CLI notify handler."""
    if ctx.invoked_subcommand is not None:
        return

    try:
        if use_stdin:
            payload = sys.stdin.read()

        if not payload:
            raise click.UsageError("Missing JSON payload (use --stdin or pass as argument)")

        data = json.loads(payload)
        validate_input(data)
        handle_codex_notify(data)
        sys.exit(0)

    except Exception as e:
        logger.error(f"Codex notify handler failed: {e}")
        sys.exit(1)


@cli.group()
def link():
    """Link ai-notify to supported CLIs."""
    pass


@link.command("claude")
@click.option(
    "--path",
    type=click.Path(path_type=Path),
    default=Path.home() / ".claude" / "hooks" / "hooks.json",
    show_default=True,
    help="Path to Claude Code hooks.json",
)
@click.option("--force", is_flag=True, help="Overwrite existing hook commands")
@click.option("--dry-run", is_flag=True, help="Show changes without writing")
def link_claude(path: Path, force: bool, dry_run: bool):
    """Install ai-notify hooks for Claude Code."""
    try:
        result = ensure_claude_hooks(path, force=force, dry_run=dry_run)
        if result.changed:
            click.echo(f"Updated hooks in {path_with_tilde(result.path)}")
        else:
            click.echo(f"Hooks already set in {path_with_tilde(result.path)}")
        if result.skipped:
            click.echo("Skipped existing hooks:")
            for event, command in result.skipped.items():
                click.echo(f"  - {event}: {command}")
        if result.errors:
            click.echo("Errors:")
            for error in result.errors:
                click.echo(f"  - {error}")
    except Exception as e:
        logger.error(f"Claude hook install failed: {e}")
        sys.exit(1)


@link.command("codex")
@click.option(
    "--path",
    type=click.Path(path_type=Path),
    default=Path.home() / ".codex" / "config.toml",
    show_default=True,
    help="Path to Codex CLI config.toml",
)
@click.option("--profile", help="Codex profile name (e.g. quiet)")
def link_codex(path: Path, profile: str | None):
    """Update Codex CLI notify command to use ai-notify."""
    try:
        update = set_codex_notify(path, ["ai-notify", "codex"], profile=profile)
        target = f"profile '{profile}'" if profile else "root config"
        if update.changed:
            click.echo(f"Updated {target} notify in {path_with_tilde(update.path)}")
        else:
            click.echo(f"{target} notify already set in {path_with_tilde(update.path)}")
    except Exception as e:
        logger.error(f"Codex config update failed: {e}")
        sys.exit(1)


@cli.command()
def check():
    """Check Claude Code hooks and Codex CLI notify integration."""
    claude_report = inspect_claude_hooks(Path.home() / ".claude", Path.cwd())
    codex_report = inspect_codex_notify(Path.home() / ".codex")

    click.echo("\nIntegration status:")

    if claude_report.status == "ok":
        claude_status = click.style("OK", fg="green")
    elif claude_report.status == "partial":
        claude_status = click.style("PARTIAL", fg="yellow")
    else:
        claude_status = click.style("MISSING", fg="red")

    click.echo(f"Claude Code hooks: {claude_status}")
    if claude_report.path:
        click.echo(f"  Config: {path_with_tilde(claude_report.path)}")
    if claude_report.missing_events:
        click.echo(f"  Missing events: {', '.join(claude_report.missing_events)}")
    if claude_report.errors:
        click.echo("  Errors:")
        for path, error in claude_report.errors.items():
            click.echo(f"    - {path_with_tilde(path)}: {error}")

    if codex_report.status == "ok":
        codex_status = click.style("OK", fg="green")
    elif codex_report.status == "partial":
        codex_status = click.style("PARTIAL", fg="yellow")
    elif codex_report.status == "error":
        codex_status = click.style("ERROR", fg="red")
    else:
        codex_status = click.style("MISSING", fg="red")

    click.echo(f"Codex CLI notify: {codex_status}")
    if codex_report.path:
        click.echo(f"  Config: {path_with_tilde(codex_report.path)}")
    if codex_report.notify is not None:
        click.echo(f"  notify: {codex_report.notify}")
    if codex_report.error:
        click.echo(f"  Error: {codex_report.error}")


@cli.command()
@click.option("--days", type=int, help="Days of data to retain (default: from config)")
@click.option(
    "--dry-run", is_flag=True, help="Show what would be deleted without actually deleting"
)
@click.option("--no-export", is_flag=True, help="Skip exporting data before cleanup")
def cleanup(days, dry_run, no_export):
    """Clean up old session data."""
    try:
        # Load config
        loader = ConfigLoader()
        cfg = loader.load()
        retention_days = days if days is not None else cfg.cleanup.retention_days
        export_before = not no_export and cfg.cleanup.export_before_cleanup

        # Get tracker
        tracker = SessionTracker()

        if dry_run:
            # Count sessions that would be deleted
            cutoff_timestamp = int((datetime.now() - timedelta(days=retention_days)).timestamp())

            with tracker._get_connection() as conn:
                cursor = conn.execute(
                    "SELECT COUNT(*) FROM sessions WHERE created_at < ?", (cutoff_timestamp,)
                )
                count = cursor.fetchone()[0]

            click.echo(f"\n{click.style('DRY RUN MODE', bold=True)} - No data will be deleted\n")
            click.echo(f"Retention period: {retention_days} days")
            click.echo(f"Sessions to delete: {count}")
            click.echo(f"Export before cleanup: {'Yes' if export_before else 'No'}")
            sys.exit(0)

        # Confirm cleanup
        click.echo(f"\nRetention period: {retention_days} days")
        click.echo(f"Export before cleanup: {'Yes' if export_before else 'No'}")

        if not click.confirm("\nProceed with cleanup?"):
            click.echo("Cleanup cancelled")
            sys.exit(0)

        # Run cleanup
        click.echo("\nRunning cleanup...")
        stats = tracker.cleanup_old_data(retention_days=retention_days, export_before=export_before)

        # Display results
        click.echo(click.style("\n✓ Cleanup complete:", fg="green"))
        results = [
            ["Sessions deleted", stats["rows_deleted"]],
            ["Space freed", f"{stats['space_freed_kb']} KB"],
        ]

        if export_before:
            results.append(["Sessions exported", stats["rows_exported"]])
            if stats["rows_exported"] > 0:
                click.echo(f"\nExported data saved to: {DEFAULT_EXPORT_DIR}")

        click.echo(tabulate(results, tablefmt="simple"))

    except Exception as e:
        click.echo(click.style(f"✗ Cleanup failed: {e}", fg="red"), err=True)
        logger.exception("Cleanup failed")
        sys.exit(1)


# Event handler subcommands
@cli.group()
def event():
    """Event handlers for Claude Code hooks (Codex uses `ai-notify codex`)."""
    pass


@event.command("user-prompt-submit")
def event_user_prompt_submit():
    """Handle UserPromptSubmit event."""
    try:
        # Read and validate input
        data = read_stdin_json()
        validate_input(data)

        # Call handler
        handle_user_prompt(data)
        sys.exit(0)

    except Exception as e:
        logger.error(f"UserPromptSubmit handler failed: {e}")
        sys.exit(1)


@event.command("stop")
def event_stop():
    """Handle Stop event."""
    try:
        # Read and validate input
        data = read_stdin_json()
        validate_input(data)

        # Call handler
        handle_stop(data)
        sys.exit(0)

    except Exception as e:
        logger.error(f"Stop handler failed: {e}")
        sys.exit(1)


@event.command("notification")
def event_notification():
    """Handle Notification event."""
    try:
        # Read and validate input
        data = read_stdin_json()
        validate_input(data)

        # Call handler
        handle_notification(data)
        sys.exit(0)

    except Exception as e:
        logger.error(f"Notification handler failed: {e}")
        sys.exit(1)


@event.command("permission-request")
def event_permission_request():
    """Handle PermissionRequest event."""
    try:
        # Read and validate input
        data = read_stdin_json()
        validate_input(data)

        # Call handler
        handle_permission(data)
        sys.exit(0)

    except Exception as e:
        logger.error(f"PermissionRequest handler failed: {e}")
        sys.exit(1)


@event.command("ask-user-question")
def event_ask_user_question():
    """Handle PreToolUse/AskUserQuestion event."""
    try:
        # Read and validate input
        data = read_stdin_json()
        validate_input(data)

        # Call handler
        handle_ask_user_question(data)
        sys.exit(0)

    except Exception as e:
        logger.error(f"AskUserQuestion handler failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    cli()
