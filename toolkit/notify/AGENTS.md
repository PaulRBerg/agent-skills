# Context

`ai-notify` is a macOS Python CLI that sends `terminal-notifier` alerts for Claude Code hooks and Codex CLI's `notify`
callback. Keep the notification integration macOS-specific while keeping pure logic and tests platform-independent; CI
runs on Ubuntu with Python 3.13.

## Development Workflow

- Bootstrap the Python 3.12+ environment with `uv sync --extra dev --locked`.
- Run the checkout directly with `uv run ai-notify ...`; use `just install-cli` only when testing the global install.
- After editing CLI code, proactively run `just install-cli` to refresh the globally installed `ai-notify` so it
  reflects the change.
- Prefer the `justfile`: `just test [pytest args]` runs pytest, while `just fc` runs Prettier, Ruff, and Pyright.
- Use `just prettier-check`, `just ruff-check`, or `just pyright-check` when a focused check is sufficient.
- `just fw` rewrites every matching Python, Markdown, and JSON file. Use focused formatter commands for surgical
  changes.

## Architecture and Invariants

- Claude Code commands under `ai-notify event` read hook JSON from stdin. The `ai-notify codex` callback accepts Codex's
  JSON as its final argument or via `--stdin`; it does not create a tracked SQLite session.
- `claude_hooks.HOOK_SPECS` is the source of truth for installed Claude hooks. `integrations.py` derives its required
  event set from that list so `link claude` and `check` stay aligned.
- Preserve unrelated settings and hooks when changing integration writers. `link codex` must continue to refuse a
  different root `notify` value unless forced; profile names resolve to sibling `<profile>.config.toml` files.
- Configuration respects `XDG_CONFIG_HOME` and defaults to `~/.config/ai-notify`. Runtime configuration is cached for
  the life of the process.
- Claude `Stop` defers completion while `background_tasks` or `session_crons` are present. `StopFailure` alerts only in
  `all` mode and bypasses duration and prompt filters. Codex payloads lack duration, so Codex filtering applies only
  notification mode and prompt-prefix exclusions.
- SQLite uses WAL mode with `synchronous=NORMAL`; session data is intentionally transient rather than strictly durable.

## Testing

- Keep feature tests in the corresponding `tests/test_*.py` module and use `just test <path-or-node-id>` for targeted
  verification.
- Isolate configuration and database paths with temporary directories; tests must not write to the user's actual XDG
  configuration directory.
- Mock the macOS platform check, `terminal-notifier` discovery, and subprocess calls. Linux CI must not require the real
  notifier.
- For hook or Codex configuration changes, cover idempotence, preservation of unrelated configuration, and conflict
  behavior.
