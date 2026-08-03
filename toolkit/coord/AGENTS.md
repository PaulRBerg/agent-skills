# Context

`ai-coord` is a Python CLI that coordinates parallel Codex and Claude Code sessions through lifecycle hooks and a local
SQLite ledger. The CLI is advisory coordination infrastructure, not a security boundary or an OS file lock.

## Upstream Documentation

- Codex hooks: <https://developers.openai.com/codex/config-advanced#hooks>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>

## Development Workflow

- Bootstrap with `uv sync --extra dev --locked`.
- Run the checkout with `uv run ai-coord ...`; use `just install-cli` only for the global-install acceptance test.
- Prefer `just test [pytest args]` while iterating on tests.
- After changing the ledger schema, run `just install-cli` so the global CLI matches; re-exec covers stragglers.
- Run `just check` after each coherent edit batch and again immediately before committing; it is the sole supported
  local macOS validation gate.
- Use focused Ruff/Prettier commands for surgical formatting. `just fw` rewrites the entire project.

## Architecture and Invariants

- `Coordinator` owns identity, provider coverage, path arbitration, queueing, communication, and status behavior.
- SQLite is the production store and the test store. Use a temporary `AI_COORD_STATE_DIR` in tests.
- Hook mode is fail-open. It must never expose raw prompts, tool payloads, transcripts, errors, messages, or notes.
- Exclusive acquisition fails closed on incomplete provider coverage; unattributed dirt holds at most ~90 seconds
  (`dirty-settling:`), then proceeds with a `stale-dirt:` advisory and baseline capture.
- Hook installers preserve unrelated configuration and derive their checks from the same hook specifications.
- Keep modules below 1000 lines and test modules below 2000 lines.
