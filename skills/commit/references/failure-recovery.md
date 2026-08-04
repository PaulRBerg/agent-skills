# Failure Recovery

Read these rules before deciding whether to retry or bypass a failed pre-commit hook or signing attempt.

- **Pre-commit hook failure:** `Failed to get staged files!` and a bare `"lint-staged" exited with code 1` do not by
  themselves prove contention. Retry as contention only when the same output explicitly names an index lock or the
  helper reports its lock refusal. Otherwise inspect the named hook output or lint-staged debug trace. Retry with
  `--no-verify` only when that evidence plus the prepared diff conclusively shows an unrelated pre-existing failure. A
  generic failure, repo-wide check, or uncertain ownership is not enough. Never bypass a failure caused by or plausibly
  affected by the intended paths; fix it or surface it. When bypassing, keep the existing one-line disclosure that the
  unrelated hook failure was skipped.
- **Signing failure (signer unreachable):** if `git commit` fails _after_ the pre-commit/commit-msg hooks already
  passed, with an error naming the configured signer rather than the content or a hook (e.g. `1Password`,
  `failed to fill whole buffer`, `ssh-agent`, `gpg failed to sign the data`, `no such identity`) — retry once, same
  command, with `--no-gpg-sign` appended. Interactive/hardware signers (1Password, YubiKey, etc.) can be unreachable
  when unattended, and the user has authorized landing unsigned commits in that case rather than blocking. Only retry on
  a genuine signer error at the signing step, never speculatively, and never edit repo/global git config
  (`commit.gpgsign`, `gpg.format`, etc.) — the bypass is per-commit only. Disclose with one line:
  `Commit created unsigned — signer unavailable ("<short error>")`. In default mode, append `--no-gpg-sign` to the
  `commit-paths.sh commit` command; keep direct Git flags for `--all` and `--staged`.
  - **Session memo:** once a genuine signer error has triggered the fallback in this session, treat the signer as
    unavailable for the rest of it: later commits may append `--no-gpg-sign` on the first attempt instead of re-failing.
    Still per-commit only — never touch git config. Replace the per-commit disclosure with a single line in the
    session's final receipt: `N commits created unsigned — signer unavailable ("<short error>")`.
