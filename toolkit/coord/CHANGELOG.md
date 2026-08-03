# Changelog

## Unreleased

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
