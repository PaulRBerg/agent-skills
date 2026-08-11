# ai-coord

`ai-coord` is advisory coordination infrastructure for parallel Codex and Claude Code agents. It is cooperative, not a
security boundary or an OS file lock.

## Package boundaries

- [`src/`](src/) is the single Rust crate for the CLI, hook integration, provider inventory, SQLite ledger, coordination
  runtime, and local dashboard API.
- [`../apps/coord-dashboard/`](../apps/coord-dashboard/AGENTS.md) is the independent Bun-managed Vite and React
  dashboard for the live coordination state.

## Shared workflow

Run shared tasks from the monorepo root `justfile`:

- `cargo test -p ai-coord --locked` runs package tests; `just rust-check` runs the complete Rust workspace gate.
- `just install-cli` installs all four workspace binaries and does not link hooks.
- `just coord-dashboard-check` and `just coord-dashboard-dev` delegate to the dashboard package.

Use package selection when isolating a Rust failure: `cargo test -p ai-coord --locked` and
`cargo clippy -p ai-coord --all-targets --locked -- --deny warnings` are the focused checks.

Keep modules below 1000 lines and test modules below 2000 lines.

## Compatibility and breaking changes

This package favors one clean current implementation. Unless a task explicitly requests compatibility, replace obsolete
behavior in one change and remove its production paths, tests, fixtures, and documentation. Do not add schema migration
ladders, old-format importers, deprecated CLI aliases, dual reads or writes, retired protocol parsers, or transitional
hook recognition by default. Rejecting an incompatible persisted version with an actionable error is required safety
behavior, not backward compatibility.

Schema v13 is the Rust implementation's clean break. It never migrates or imports an older ledger. Session liveness is
based on kernel-backed process fingerprints on macOS and Linux: a confirmed dead or replaced process is removed without
an age grace period, while unknown liveness fails closed and never deletes the record.

Before work that can invalidate live chats, their ledger, hooks, or coordination CLI, require the user to close other
agents and explicitly authorize the break, then implement it from one fresh session. Use an isolated
`AI_COORD_STATE_DIR` for development and validation. Never silently reset a ledger or globally install, relink, or run
incompatible source against live state. Live hook replacement must finish before removing any one-time transitional
recognizer; ledger replacement and global rollout remain separate explicitly authorized actions.

## Agent-facing protocol

Treat the one-sentence stderr guidance printed for every `start`, `wait`, and `done` outcome as the authoritative next
step while preserving their stdout TSV as a machine interface. Only `READY` grants editing; release completed work with
`done`. Preserve `stale-dirt` hunks byte-for-byte. `ai-coord baseline` is a stable machine contract consisting of one
normalized repository-relative `path<TAB>oid` record per line, or empty output when no baselines exist.

`ai-coord touched` is a best-effort cross-check of normalized repository-relative paths observed in this session's
file-mutating post-tool payloads. Its stable output is one path per line, with a leading `!TRUNCATED` record when its
1,000-path cap dropped older records; an empty complete set exits successfully with no output. It stores no payload
content. Status schema v5 exposes required session `coordination_waived` booleans and nonzero repository task-handoff
counts as `{repo_root, count}` records; guidance stays
here while README remains human-facing tool documentation.

## Upstream documentation

- Codex hooks: <https://developers.openai.com/codex/hooks>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>

Codex hook, app-server, and hook-trust changes require `$agents-docs` and verification against the current official
Codex hooks and app-server documentation before implementation. Never derive or persist hook hashes manually; obtain and
verify them through the supported app-server protocol for the exact owned hook definitions.
