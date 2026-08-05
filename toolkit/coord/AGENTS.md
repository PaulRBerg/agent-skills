# ai-coord

`ai-coord` is advisory coordination infrastructure for parallel Codex and Claude Code agents. It is cooperative, not a
security boundary or an OS file lock.

## Packages

- [`cli/`](cli/AGENTS.md) is the Python CLI, hook integration, SQLite ledger, and local dashboard API.
- [`dashboard/`](dashboard/AGENTS.md) is the Bun-managed Vite and React dashboard for the live coordination state.

## Shared workflow

Run repository-wide tasks from the root `justfile`:

- `just check` runs the supported local validation gate; `just full-check` and `just full-write` run checks or fixes
  without tests.
- `just test` runs the CLI test suite, and `just install-cli` performs the global-install acceptance flow.
- `just prettier-check` and `just prettier-write` check or format Markdown, JSON, and dashboard source files.
- `just dev` starts the local API and dashboard development servers together.

Keep modules below 1000 lines and test modules below 2000 lines.

## Compatibility and breaking changes

This pre-1.0 repository favors one clean current implementation. Unless a task explicitly requests compatibility,
replace obsolete behavior in one change and remove its production paths, tests, fixtures, and documentation. Do not add
schema migration ladders, old-format importers, deprecated CLI aliases, dual reads or writes, retired protocol parsers,
or transitional hook recognition by default. Rejecting an incompatible persisted version with an actionable error is
required safety behavior, not backward compatibility.

Before work that can invalidate live chats, their ledger, hooks, or coordination CLI, require the user to close other
agents and explicitly authorize the break, then implement it from one fresh session. Use an isolated
`AI_COORD_STATE_DIR` for development and validation. Never silently reset a ledger or globally install, relink, or run
incompatible source against live state. Live hook replacement must finish before removing any one-time transitional
recognizer; ledger replacement and global rollout remain separate explicitly authorized actions.

## Upstream documentation

- Codex hooks: <https://developers.openai.com/codex/hooks>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>

Codex hook, app-server, and hook-trust changes require `$agents-docs` and verification against the current official
Codex hooks and app-server documentation before implementation. Never derive or persist hook hashes manually; obtain and
verify them through the supported app-server protocol for the exact owned hook definitions.
