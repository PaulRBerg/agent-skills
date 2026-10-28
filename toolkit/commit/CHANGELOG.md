# Changelog

## [1.1.1] - 2026-10-28

### Fixed

- Fix false superseded receipts after clean rebases with unrelated edits in the same file ([`f191cf0`](https://github.com/PaulRBerg/agent-skills/commit/f191cf0239767c532674ef48e9f7783df4c5fb96))

## [1.1.0] - 2026-10-24

### Added

- Add `PUSH_SKIPPED <branch> no-remote` for repositories with no branch remote or named remotes, preserving local commit receipts without a pending push ([`71c1b82`](https://github.com/PaulRBerg/agent-skills/commit/71c1b82fa01a850b05cff9a1e1261864b7695265))

## [1.0.3] - 2026-10-22

### Fixed

- Fix `prepare` rejecting absolute paths spelled through a symlinked prefix such as macOS `/tmp` ([`733eadf`](https://github.com/PaulRBerg/agent-skills/commit/733eadf), [`c9e5ed0`](https://github.com/PaulRBerg/agent-skills/commit/c9e5ed0))
- Fix embedded spaces in the superseded-after-rebase error message ([`0718565`](https://github.com/PaulRBerg/agent-skills/commit/0718565))

## [1.0.2] - 2026-10-20

### Fixed

- Fix baseline isolation by bypassing custom and conflict-resolving merge drivers ([`6a72390`](https://github.com/PaulRBerg/agent-skills/commit/6a72390a87a8))

## [1.0.1] - 2026-10-20

### Fixed

- Fix baseline exclusion when unrelated edits share patch context ([`e71e48d`](https://github.com/PaulRBerg/agent-skills/commit/e71e48d8cbb2))

## [1.0.0] - 2026-10-14

_First release._

[1.1.1]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-commit-v1.1.1
[1.1.0]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-commit-v1.1.0
[1.0.3]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-commit-v1.0.3
[1.0.2]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-commit-v1.0.2
[1.0.1]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-commit-v1.0.1
[1.0.0]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-commit-v1.0.0
