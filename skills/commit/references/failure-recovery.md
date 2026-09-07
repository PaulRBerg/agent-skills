# Failure Recovery

Classify the failure before choosing recovery. Retry the existing immutable transaction for interruptions, lock races,
signing failures, receipt or reconciliation failures, and any uncertain commit outcome. An idempotent retry can recover
a commit created before the failure without duplicating it. A known content failure before commit creation instead
requires a corrected preparation; retrying the old snapshot cannot include its repair.

## Content Validation Failure

Use this recovery for deterministic validation failures requiring changes to authorized content, including a dependency
cycle where separately prepared units cannot pass or apply in a valid order. A timeout or transient tool failure alone
does not justify replacing a preparation.

1. Retain the failed transaction ID, its prepared path set, and the diagnostic proving which content needs correction.
   Inspect `ai-commit show <transaction-id>` and the failure receipt. Establish that no commit was created; absent or
   uncertain outcome evidence requires the same-transaction retry path instead. `PREPARED` in `show` alone does not
   prove this: the display may omit a pending commit.
2. Identify the smallest coherent correction and preserve unrelated or baseline-owned content. Existing authorization to
   fix and commit the task covers repairing its failed content and combining interdependent owned changes; a new
   permission request is needed only when the correction itself exceeds that authority.
3. Run `ai-commit discard <transaction-id>` only for the superseded, uncommitted preparation and require `DISCARDED`. If
   it refuses a pending or committed transaction, recover that same ID instead of preparing a replacement. Never rewrite
   a committed transaction or delete its retained receipt. For multiple dependent preparations, account for each ID and
   all intended changes before discarding them.
4. Apply the correction, run the relevant checks, then prepare once from the corrected owned paths. Review the full new
   evidence and compose its message around the final change. Retain the new ID and follow the normal commit workflow.

Do not bypass a validation failure caused by or plausibly affected by the prepared paths. Keep each new preparation
immutable; another evidenced content defect requires another explicit diagnosis, not an automatic reprepare loop.

## Prepared Snapshot Drift

The exact diagnostic prefix `snapshot-check hook modified prepared content` identifies a content failure before commit
creation: a verification hook tried to change the validation-only prepared snapshot.

1. Do not retry the transaction, add `--no-verify`, or make the shared worktree temporarily match the prepared index.
2. Record the repository-relative paths named by the diagnostic, then run `ai-commit discard <transaction-id>` and
   require `DISCARDED`; a pending or committed transaction still requires same-ID recovery.
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
