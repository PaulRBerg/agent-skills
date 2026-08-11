# Changelog

## 0.1.0 - Unreleased

- Add immutable prepare/commit transactions for shared Git working trees.
- Add safe upstream-aware push, transaction inspection, and discard workflows.
- Support transactional first commits on named unborn branches.
- Automatically exclude `ai-coord` stale-dirt baselines during preparation, with explicit precedence and an opt-out.
- Run verification hooks against a temporary prepared-index worktree when intended paths differ from the physical
  worktree, while preserving normal hook mutation behavior for matching transactions.
