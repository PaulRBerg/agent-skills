# Changelog

## [1.0.3] - 2026-10-28

### Fixed

- Fix an authoritative `claude agents --json` listing deleting live Claude sessions it omits, which failed `start` with `FOREIGN KEY constraint failed` ([`f86ec58`](https://github.com/PaulRBerg/agent-skills/commit/f86ec58))

## [1.0.2] - 2026-10-22

### Fixed

- Fix `start --recursive` over an active exact claim on the same path skipping arbitration and returning READY over peer child claims ([`3f24015`](https://github.com/PaulRBerg/agent-skills/commit/3f24015))
- Fix overlap diagnostics reporting `.` instead of the inner path for a root recursive scope ([`3f24015`](https://github.com/PaulRBerg/agent-skills/commit/3f24015))
- Fix `serve` accepting trailing text after a bracketed IPv6 `Host` header ([`733eadf`](https://github.com/PaulRBerg/agent-skills/commit/733eadf))

## [1.0.1] - 2026-10-22

### Fixed

- Fix Claude provider coverage after Claude Code auto-updates prune the pinned binary ([`dd244b6`](https://github.com/PaulRBerg/agent-skills/commit/dd244b6))
- Fix waker hook health outside Git worktrees ([`6029561`](https://github.com/PaulRBerg/agent-skills/commit/6029561))

## [1.0.0] - 2026-10-14

_First release._

[1.0.3]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.3
[1.0.2]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.2
[1.0.1]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.1
[1.0.0]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.0
