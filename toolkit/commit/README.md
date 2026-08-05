# ai-commit

`ai-commit` separates commit analysis from mutation. `prepare` captures an immutable Git tree without changing the
shared index; `commit` later applies that exact snapshot to the current branch under Git's index lock, runs normal hooks
and signing against an isolated index, and reconciles only shared-index entries that have not changed in the meantime.

```console
$ ai-commit prepare -- src/main.rs
PREPARED 0123456789abcdef
...
$ ai-commit commit 0123456789abcdef -m "feat: add safe transactions"
COMMITTED 0123456789abcdef 89abcdef...
```

## Installation

Requires Git, Cargo, and the rolling Rust nightly toolchain:

```sh
cargo install --git https://github.com/PaulRBerg/ai-commit --locked --root "$HOME/.local"
```

For local development, install the current checkout instead:

```sh
cargo install --path . --locked --force --root "$HOME/.local"
```

## Commands

```text
ai-commit prepare [--all|--staged] [--natural|--conventional]
                  [--diff summary|full] [--exclude-baseline path=oid]...
                  [--porcelain] -- [paths...]
ai-commit commit <transaction-id> -m <message>... [--push]
                  [--no-verify] [--no-gpg-sign]
ai-commit push
ai-commit show <transaction-id>
ai-commit discard <transaction-id>
```

Preparation rejects repository operation states and detached HEADs. Default mode requires explicit paths; `--all`
captures the complete worktree/index result, while `--staged` copies the current index exactly. A successful transaction
is pinned under `refs/ai-commit/transactions/<id>` and remains retryable until committed or discarded.
Prepared journals do not age out; terminal receipts and their refs are retained for seven days. When available,
`ai-coord trailer` contributes one validated `Agent-Session:` line to the preparation evidence.

State defaults to `$XDG_STATE_HOME/ai-commit` or `~/.local/state/ai-commit`; `AI_COMMIT_STATE_DIR` overrides it.
Configuration defaults to `$XDG_CONFIG_HOME/ai-commit/config.toml` and can be overridden with `AI_COMMIT_CONFIG`:

```toml
[message]
natural_repositories = ["~/work/natural-history"]
```

Configured repository paths are matched against the exact canonical physical repository root. Missing configuration is
valid. Explicit `--natural` or `--conventional` always wins.

`prepare --porcelain` emits stable TSV records. Tabs, newlines, carriage returns, and backslashes inside fields are
backslash-escaped. Outcome records use `PREPARED`, `COMMITTED`, `PUSHED`, `PUSHED_NEW`, `BEHIND`, `HOOK_ADDED`, and
`DISCARDED`.

Exit status `0` means success or an idempotent replay, `2` means invalid invocation or configuration, and `3` means the
repository was left safe but needs a retry or reconciliation. Other Git, hook, signing, and push failures return `1`.
Pushes always fetch and compare first; they never pull, merge, or rebase.

## Development

The crate targets macOS and Linux with the rolling Rust nightly toolchain.

```console
just full-check
just test
```

Licensed under MIT.
