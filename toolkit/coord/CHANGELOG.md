# Changelog

## 0.3.0

- Rewrite the CLI, hook runtime, SQLite ledger, provider inventory, and dashboard API as one Rust binary; retain the
  Bun-managed Vite and React dashboard.
- Break the internal ledger at schema v9, create only the current schema, reject older state without migration or
  import, and remove retired compatibility paths.
- Reconcile session liveness from kernel-backed PID and process-start fingerprints on macOS and Linux. Normal
  `SessionEnd`, terminal closure, Ctrl+C, crashes, and PID reuse no longer leave age-based false presence; ambiguous
  liveness fails closed and never deletes a session.
- Make `start` exact-file-first with explicit repeatable `--recursive` directory scopes, and support atomic active or
  queued scope replacement with narrowing guidance, fair queue-age handling, and fail-closed expansions.
- Cache complete provider inventory for bounded read reuse while keeping authorization refreshes fresh.
- Add optional emoji callsigns with machine-wide uniqueness, historical message endpoint snapshots, callsign targeting,
  lifecycle nudges, CLI status, and dashboard display support while retaining immutable session-ID fallbacks.
- Keep session-start hooks bookkeeping-only, move callsign and presence context to prompt hooks, and exclude Codex
  compaction starts from lifecycle bookkeeping.
- Add bounded property and state-machine coverage for coordination, hooks, integrations, and pure helpers.
- Reject unterminated JSONC block comments instead of silently accepting a valid prefix.
- Preserve the public status JSON v1 schema across the implementation and internal-schema break.
- Poll the SQLite generation counter for push-driven waits, with a slow full-refresh fallback.
- Add counts-only mid-turn inbox nudges for Codex and Claude Code.
- Wake blocked Claude Code sessions asynchronously and notify overlapping waiters on release.
- Update modular Claude hook sources instead of overwriting generated settings output.
- Scope delegates and machine-wide notes correctly.
- Reclaim claims immediately after the recorded host process is confirmed gone.
- Allow literal in-repository symlink scopes without dereferencing their targets.
- Preserve literal scope identity and prevent claims from moving across repositories.
- Make concurrent first-run database initialization safe.
- Honor client configuration roots and validate complete hook definitions.
- Replace hook configuration files atomically while preserving symlink targets and permissions.
- Reject malformed lifecycle records without polluting coordination state.

## 0.1.0

- Initial release.
