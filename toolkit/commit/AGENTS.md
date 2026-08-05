# Development Instructions

`ai-commit` is a single Rust crate. It shells out to native `git`; do not introduce libgit2 or another repository model.

## Architecture

- `src/cli.rs` defines the public command line; `src/error.rs` and `src/main.rs` map failures to stable exit classes.
- `src/prepare.rs` resolves intended paths, constructs immutable trees in alternate indexes, and records transactions.
- `src/commit.rs` reapplies prepared deltas to locked current HEAD, runs hooks/signing, CAS-updates refs, and reconciles
  the shared index.
- `src/push.rs` implements fetch-first, no-integration pushes.
- `src/state.rs` owns atomic journal records, receipts, retention, and transaction refs.
- `src/git.rs` is the only subprocess boundary for Git operations.

## Invariants

- `prepare` must never mutate the worktree, shared index, branch ref, or user configuration.
- Prepared objects remain pinned until a terminal receipt expires or a prepared transaction is discarded.
- A commit is built from the prepared tree, with only clean current-HEAD movement and hook-staged changes admitted.
- Never remove an index lock that this process did not create. Hold the owned lock through ref CAS and index
  reconciliation.
- A post-ref-update failure must remain replayable without creating a second commit.
- Tests isolate repositories, remotes, `HOME`, configuration, and state in temporary directories.

## Validation

Run `just full-check`, then `just test`. During focused development, use the narrowest relevant `cargo test` filter
before the aggregate commands.
