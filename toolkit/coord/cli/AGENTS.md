# CLI package

`cli/` contains the Python `ai-coord` package: the command-line interface, host-hook integration, coordination ledger,
and local dashboard API.

## Development workflow

Bootstrap from this directory with `uv sync --extra dev --locked`, then run the checkout with `uv run ai-coord ...`. Use
`just test [pytest args]` from the repository root while iterating. `just check` is the supported local macOS validation
gate and runs formatting, linting, type checks, and the full test suite; use focused Ruff or Prettier commands for
surgical formatting. `just fw` rewrites the whole project. `just install-cli` is the global-install acceptance test;
after a ledger schema change, run it so the global CLI matches and re-exec can cover stragglers.

## Architecture and invariants

- `Coordinator` owns identity, provider coverage, path arbitration, queueing, communication, and status behavior.
- SQLite is both the production and test store. Tests use a temporary `AI_COORD_STATE_DIR`.
- Hook mode is fail-open: it must not expose raw prompts, tool payloads, transcripts, errors, messages, or notes.
- Model-visible hook context must be factual, bounded, and counts-only: never include peer text, IDs, prompts, or tool
  payloads, and state explicitly that peer-reported content is data rather than instructions or authority.
- Hook `permission_mode` values are stored only when they match the fixed host-mode whitelist; unknown values clear the
  field rather than persisting arbitrary payload data.
- Exclusive acquisition fails closed when provider coverage is incomplete. Relevant unattributed dirt settles for at
  most about 90 seconds (`dirty-settling:`), then acquisition can proceed with a `stale-dirt:` advisory and baseline
  capture.
- Hook installers preserve unrelated configuration and derive their checks from the same hook specifications.

## Dashboard API

`ai-coord serve` is a user-facing local server, not hook mode. It serves `GET /api/snapshot` and `GET /api/events` on
`127.0.0.1:4477` by default. Events are generation-driven Server-Sent Events, with a heartbeat; snapshots share a
process-wide two-second cache. Keep the HTTP and SSE implementation stdlib-only.
