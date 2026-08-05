# Failure Recovery

Retry the existing immutable transaction. Do not run `prepare` again after a commit, hook, signing, lock, receipt, or
reconciliation failure: `ai-commit commit <transaction-id> ...` is idempotent and recovers a commit created before an
interruption without duplicating it.

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
