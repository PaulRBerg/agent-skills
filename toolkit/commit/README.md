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
cargo install --git https://github.com/PaulRBerg/agent-toolkit ai-commit --locked --root "$HOME/.local"
```

For local development, install the current checkout instead:

```sh
cargo install --path . --locked --force --root "$HOME/.local"
```

## Commands

```text
ai-commit prepare [--all|--staged] [--natural|--conventional]
                  [--diff summary|full] [--exclude-baseline path=oid]...
                  [--no-auto-baseline]
                  [--porcelain] -- [paths...]
ai-commit commit <transaction-id> -m <message>... [--push]
                  [--no-verify] [--no-gpg-sign]
ai-commit push
ai-commit show <transaction-id>
ai-commit discard <transaction-id>
```

Each `-m` value is a literal paragraph; repeated values are separated by one blank line. For a multi-line paragraph,
pass real line breaks within that argument. The two-character text `\\n` is rejected so an accidentally escaped list
does not become a malformed commit message:

```sh
ai-commit commit <transaction-id> -m 'docs: record constraints' -m '- describe the evidence contract
- document the approval gate'
```

Preparation rejects repository operation states and detached HEADs. Named unborn branches are supported: preparation
uses Git's empty tree and commit creates a transactional parentless root commit. Default mode requires explicit paths;
`--all` captures the complete worktree/index result, while `--staged` copies the current index exactly. A successful
transaction is pinned under `refs/ai-commit/transactions/<id>` and remains retryable until committed or discarded.
Prepared journals do not age out; terminal receipts and their refs are retained for seven days. When available,
`ai-coord trailer` contributes one validated `Agent-Session:` line to the preparation evidence.
In default and `--all` modes, `prepare` also asks `ai-coord baseline` for stale-dirt baselines and excludes the
pre-existing portions of those files automatically. Explicit `--exclude-baseline` values take precedence for the same
path. Use `--no-auto-baseline` to disable ambient discovery while retaining explicit exclusions; `--staged` always
skips discovery because it captures the index exactly.

State defaults to `$XDG_STATE_HOME/ai-commit` or `~/.local/state/ai-commit`; `AI_COMMIT_STATE_DIR` overrides it.
Message format configuration is repository-local at `<git-root>/.agents/commit.toml`:

```toml
[message]
format = "natural"
```

`format` must be `"natural"` or `"conventional"`. An absent file defaults to conventional format; an invalid file is a
usage error. Explicit `--natural` or `--conventional` always wins for that preparation.

`prepare --porcelain` emits stable TSV records. Tabs, newlines, carriage returns, and backslashes inside fields are
backslash-escaped. Outcome records use `PREPARED`, `COMMITTED`, `PUSHED`, `PUSHED_NEW`, `BEHIND`, `HOOK_ADDED`, and
`DISCARDED`. Each automatically applied exclusion is disclosed as `AUTO_BASELINE<tab>path<tab>oid`; the ordinary
output lists the same pairs under `auto-applied baselines`.

Exit status `0` means success or an idempotent replay, `2` means invalid invocation or configuration, and `3` means the
repository was left safe but needs a retry or reconciliation. Other Git, hook, signing, and push failures return `1`.
Pushes always fetch and compare first; they never pull, merge, or rebase.

## Development

The crate targets macOS and Linux with the rolling Rust nightly toolchain.

From the monorepo root, run `cargo test -p ai-commit --locked` for package tests or `just rust-check` for the complete
Rust workspace gate.

Licensed under MIT.
