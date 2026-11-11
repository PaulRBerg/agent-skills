# Changelog

## [1.1.0] - 2026-11-11

### Added

- Add usage-error hints that name a misplaced flag's subcommands, map bare option names to flags, and print the intended subcommand's help ([`c8474da`](https://github.com/PaulRBerg/agent-skills/commit/c8474da))

## [1.0.6] - 2026-11-06

### Changed

- Resolve each distinct Claude working directory once per inventory response ([`8cd819c`](https://github.com/PaulRBerg/agent-skills/commit/8cd819c))

### Fixed

- Fix unchanged queued rechecks triggering repeated provider and dashboard refreshes through ledger generation updates ([`8cd819c`](https://github.com/PaulRBerg/agent-skills/commit/8cd819c))
- Fix blocked stdin delivery bypassing inspection command deadlines ([`cb9909e`](https://github.com/PaulRBerg/agent-skills/commit/cb9909e))
- Fix failed or timed-out inspection commands leaving descendants running with inherited output pipes ([`cb9909e`](https://github.com/PaulRBerg/agent-skills/commit/cb9909e))

## [1.0.5] - 2026-11-06

### Changed

- Reduce dirty-settling provider refreshes to the remaining hold time, capped at 20 seconds, while preserving prompt wakes and fresh queued-grant checks ([`5d70af8`](https://github.com/PaulRBerg/agent-skills/commit/5d70af8))
- Deduplicate Git status paths in linear expected time while preserving their order ([`5d70af8`](https://github.com/PaulRBerg/agent-skills/commit/5d70af8))

### Fixed

- Fix repeated overlap-holder messages when another claim in the same bundle is blocked by settling dirt ([`5d70af8`](https://github.com/PaulRBerg/agent-skills/commit/5d70af8))

## [1.0.4] - 2026-11-06

### Changed

- Reduce post-tool Git scans to claimed and written paths, share one inspection, and disable optional index writes ([`7fed9d8`](https://github.com/PaulRBerg/agent-skills/commit/7fed9d8))

### Fixed

- Fix directories and special files forcing per-file Git hash retries across an entire batch ([`7fed9d8`](https://github.com/PaulRBerg/agent-skills/commit/7fed9d8))

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

[1.1.0]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.1.0
[1.0.6]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.6
[1.0.5]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.5
[1.0.4]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.4
[1.0.3]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.3
[1.0.2]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.2
[1.0.1]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.1
[1.0.0]: https://github.com/PaulRBerg/agent-skills/releases/tag/ai-coord-v1.0.0
