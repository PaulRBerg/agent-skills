# Failure Recovery

Retry the existing immutable transaction. Do not run `prepare` again after a commit, hook, signing, lock, receipt, or
reconciliation failure: `ai-commit commit <transaction-id> ...` is idempotent and recovers a commit created before an
interruption without duplicating it.

## Prepared Snapshot Drift

The exact diagnostic prefix `snapshot-check hook modified prepared content` is the only exception to immutable retry. An
unchanged retry repeats because a verification hook tried to change the validation-only prepared snapshot.

1. Do not retry the transaction, add `--no-verify`, or make the shared worktree temporarily match the prepared index.
2. Record the repository-relative paths named by the diagnostic, then run `ai-commit discard <transaction-id>`.
3. Apply only the named deterministic formatter or generator change to session-owned content. Preserve every stale-dirt
   baseline byte; do not stage the whole physical file or restore excluded hunks temporarily.
4. Prepare once from the corrected worktree and continue with the new transaction.

If the hook-required change would alter baseline-owned bytes, stop and wait for or contact that baseline's owner instead
of discarding their work.

The legacy `partially staged files are unsafe in the shared worktree` diagnostic on a prepared path is a deterministic
compatibility failure, not index contention. Never respond with `--no-verify`, temporary hunk restoration, or a
sleep/retry loop; surface the incompatible `ai-commit`/hook path and update it before preparing another transaction.

- **Index lock:** wait and retry the same command only when the diagnostic names the default-index lock or `ai-commit`
  reports its lock refusal. Never delete a lock.
- **Hook failure:** a bare lint-staged `Failed to get staged files!` or `"lint-staged" exited with code 1` does not
  prove contention. Inspect the named hook output or lint-staged debug trace. Retry the same transaction with
  `--no-verify` only when that evidence and the immutable prepared diff conclusively prove an unrelated pre-existing
  failure. Never bypass a failure caused by, or plausibly affected by, the prepared paths. The flag bypasses pre-commit
  and commit-msg hooks for that attempt; it does not change repository configuration. After success disclose exactly one
  line: `Commit created with hooks bypassed — unrelated failure ("<short error>")`.
- **Signing failure:** when commit creation fails at signing after hooks passed, and the error names the configured
  signer rather than content or a hook, retry the same transaction once with `--no-gpg-sign`. Examples include an
  unreachable 1Password or YubiKey signer, `failed to fill whole buffer`, `ssh-agent`, `gpg failed to sign the data`, or
  `no such identity`. Never add the flag speculatively and never edit `commit.gpgsign`, `gpg.format`, or other Git
  configuration. After success disclose exactly one line:
  `Commit created unsigned — signer unavailable ("<short error>")`.

Once a genuine signer error establishes that the signer is unavailable for the session, later transactions may use
`--no-gpg-sign` on their first commit attempt. Keep the bypass per transaction and replace repeated disclosures in the
final receipt with: `N commits created unsigned — signer unavailable ("<short error>")`.
