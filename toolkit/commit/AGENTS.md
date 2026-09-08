# Development Instructions

`ai-commit` is a Rust workspace member. It shells out to native `git`; do not introduce libgit2 or another repository
model.

## Architecture

- `src/cli.rs` defines the public command line; `src/error.rs` and `src/main.rs` map failures to stable exit classes.
- `src/prepare.rs` resolves intended paths, constructs immutable trees in alternate indexes, and records transactions.
- `src/validation.rs` reapplies immutable prepared deltas, materializes candidates, and runs frozen validation with
  drift detection for both `validate` and `commit`.
- `src/commit.rs` locks the shared index, runs hooks/signing, CAS-updates refs, and reconciles the shared index.
- `src/push.rs` implements fetch-first, no-integration pushes.
- `src/state.rs` owns atomic journal records, receipts, retention, and transaction refs.
- `src/git.rs` is the only subprocess boundary for Git operations.

## Invariants

- `prepare` must never mutate the worktree, shared index, branch ref, or user configuration.
- Automatic stale-dirt baselines are advisory: malformed output or an unavailable/failing `ai-coord` must not fail
  preparation, explicit exclusions win by path, and staged capture never consults ambient coordination state.
- Prepared objects remain pinned until a terminal receipt expires or a prepared transaction is discarded.
- A commit is built from the prepared tree, with only clean current-HEAD movement and hook-staged changes admitted.
- When an intended prepared path differs from the physical worktree, verification hooks run against a temporary
  materialization of the complete prepared index. Those hooks may edit the message but must not modify tracked content.
- An optional repository validation argv is frozen into the prepared transaction and always runs directly against a
  complete materialization of that prepared candidate before verification hooks, including with `--no-verify`. It
  receives an isolated Git environment; ignored local directories remain available for read-only dependency and
  evidence resolution when their parents exist in the candidate tree. Failure or detected tracked/index drift leaves
  the transaction prepared and retryable.
- Standalone validation takes only the transaction lock, runs no hooks or commit mutations, reports skipped validation
  explicitly when no command was frozen, and never lets preflight success bypass commit-time validation.
- Preserve the validation target's journal and refs, including expired receipts. Pending commits require same-ID
  commit recovery; preflight must not validate or replace them.
- Normal verification hooks retain their existing physical-worktree behavior, and `post-commit` always runs from the
  physical worktree without the snapshot-check environment.
- Never remove an index lock that this process did not create. Hold the owned lock through ref CAS and index
  reconciliation.
- A post-ref-update failure must remain replayable without creating a second commit.
- Tests isolate repositories, remotes, `HOME`, configuration, and state in temporary directories.

## Validation

From the monorepo root, run the narrowest relevant `cargo test -p ai-commit` filter first, then `just rust-check` for
the aggregate Rust gate. `just install-cli` installs every workspace binary under `~/.local`; run it only when the task
requires refreshing the installed CLIs.
