# Changelog

## Unreleased

- Cache complete provider inventory for bounded read reuse, probe Codex and Claude concurrently, keep authorization
  refreshes fresh, and lazy-load command-specific modules.
- Add optional emoji callsigns with machine-wide uniqueness, historical message endpoint snapshots, callsign targeting,
  lifecycle nudges, CLI status, and dashboard display support while retaining immutable session-ID fallbacks.
- Keep session-start hooks bookkeeping-only, move callsign and presence context to prompt hooks, and exclude Codex
  compaction starts from lifecycle bookkeeping.
- Add bounded property and state-machine coverage for coordination, hooks, migration, integrations, and pure helpers.
- Reject unterminated JSONC block comments instead of silently accepting a valid prefix.
- Fingerprint agent processes with PID and creation time, and upgrade the internal ledger to schema v3 without changing
  the public status JSON schema.
- Poll the SQLite generation counter for push-driven waits, with a slow full-refresh fallback.
- Add counts-only mid-turn inbox nudges for Codex and Claude Code.
- Wake blocked Claude Code sessions asynchronously and notify overlapping waiters on release.
- Update modular Claude hook sources instead of overwriting generated settings output.
- Upgrade the ledger one way to schema v2 to deduplicate inbox nudges.
- Scope delegates and machine-wide notes correctly, and ignore invalid legacy-pattern claims in FIFO arbitration.
- Reclaim claims from stale Codex sessions after their recorded process exits.
- Allow literal in-repository symlink scopes without dereferencing their targets.
- Preserve literal scope identity and prevent claims from moving across repositories.
- Make concurrent first-run database initialization safe.
- Honor client configuration roots and validate complete hook definitions.
- Replace hook configuration files atomically while preserving symlink targets and permissions.
- Reject malformed lifecycle and legacy records without polluting coordination state.

## 0.1.0

- Initial release.
