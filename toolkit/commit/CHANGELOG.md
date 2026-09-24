# Changelog

## 1.0.0 - Unreleased

- Run snapshot-check hooks and configured validation from a materialization under ai-commit's state directory instead of
  the repository's Git directory, so upward config and `node_modules` lookups no longer reach the physical worktree;
  materializations left by interrupted runs are removed automatically, and leftover `.git/ai-commit-hook-*`
  directories from earlier versions can be deleted manually.
- Project ignored directories one level deep and recreate their symlinks, so workspace links such as
  `node_modules/@scope/pkg -> ../../packages/pkg` resolve to the prepared candidate; also project ignored symlinks to
  directories and initialized submodules whose checked-out HEAD matches the candidate's gitlink.
- Stop rejecting hooks and validators that rewrite identical bytes (`touch`, `sed -i`) as modifying prepared content.
- Skip building the materialization when no verification hook file or validator will run.
- Release Git's shared index lock while configured `[validation]` commands run; `commit` re-acquires it, re-verifies the
  branch head, and revalidates onto a moved head up to three times before a retryable exit.
- Skip and disclose (`AUTO_BASELINE_SKIPPED`) automatic `ai-coord` baselines whose path is in neither HEAD nor the
  worktree; other automatic baseline failures now name their ai-coord origin and suggest `--no-auto-baseline` or an
  explicit `--exclude-baseline` override.
- Stage `prepare` path lists through one `git update-index --stdin` and one `git add --pathspec-from-file` process
  instead of 32-path batches.
- Remove an expired transaction's lock file with its receipt, and stop counting lock files toward the receipt-cleanup
  scan limit, so cleanup keeps working after many transactions.
- Run `post-commit` with `GIT_INDEX_FILE` set to the real shared index instead of the temporary commit index.
- Add immutable prepare/commit transactions for shared Git working trees.
- Add safe upstream-aware push, transaction inspection, and discard workflows.
- Support transactional first commits on named unborn branches.
- Automatically exclude `ai-coord` stale-dirt baselines during preparation, with explicit precedence and an opt-out.
- Run verification hooks against a temporary prepared-index worktree when intended paths differ from the physical
  worktree, while preserving normal hook mutation behavior for matching transactions.
- Add an optional repository-local argv-only prepared-tree validation command, frozen at preparation and run directly
  before verification hooks (including with `--no-verify`), with isolated Git state, retryable failure, and rejection
  of validator content drift. Ignored local dependency and evidence directories remain resolvable from the
  materialization. Existing repositories and journals without the option retain their behavior.
- Print 12-character commit OID abbreviations in receipts and retryable diagnostics; `show` and the journal keep full
  OIDs.
- Condense the printed message-format rules and cap the `--diff full` display at 400 lines per file (disclosed via
  `DIFF_TRUNCATED`), omitting binary patch payloads.
- Project ignored local directories into snapshot-check hook materializations (not only configured-validation
  materializations), so repository hooks resolve `node_modules` and similar tooling without special-casing
  `AI_COMMIT_HOOK_MODE`.
- Fix `push` fast-forwarding the wrong ref: refuse before fetching or pushing when the current branch's upstream
  branch name differs from the local branch name, naming both refs and pointing at an explicit `git push`.
- Batch the worktree-comparison `git add -A` invocation with the crate's existing path batch size instead of passing
  every intended path in one command, avoiding an `E2BIG` failure on transactions with very many paths.
- Decode the `--diff full` display diff with lossy UTF-8 instead of failing preparation when tracked content is not
  valid UTF-8; the prepared tree, name-status, shortstat, and path records remain strict and complete.
- Fix `DIFF_TRUNCATED` path tracking in the `--diff full` display diff: a hunk's added-line content beginning with
  `++ b/` could be mistaken for the next file's `+++ b/<path>` header, and Git-quoted headers (paths containing `"`,
  `\`, or control characters) were not unquoted.
- Fix a `git check-ignore --stdin` deadlock when resolving ignored local directories: stdin is now written on a
  separate thread while stdout is read concurrently, instead of writing all of stdin before reading any output.
