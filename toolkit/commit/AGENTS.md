# ai-commit

`ai-commit` is a Rust workspace member. It shells out to native `git`; do not introduce libgit2 or another repository
model.

## Architecture

- `src/cli.rs` defines the public command line; `src/error.rs` and `src/main.rs` map failures to stable exit classes.
- `src/prepare.rs` resolves intended paths, constructs immutable trees in alternate indexes, and records transactions.
- `src/validation.rs` reapplies immutable prepared deltas, materializes candidates, and runs frozen validation with
  drift detection for both `validate` and `commit`.
- `src/commit.rs` runs configured validation without the shared index lock, then locks the shared index, re-verifies
  the branch head, runs hooks/signing, CAS-updates refs, and reconciles the shared index.
- `src/push.rs` implements fetch-first, no-integration pushes.
- `src/state.rs` owns atomic journal records, receipts, retention, and transaction refs.
- `src/git.rs` is the only subprocess boundary for Git operations.

## Invariants

- `prepare` must never mutate the worktree, shared index, branch ref, or user configuration.
- Automatic stale-dirt baselines are advisory: malformed output or an unavailable/failing `ai-coord` must not fail
  preparation, explicit exclusions win by path, and staged capture never consults ambient coordination state. An
  automatic baseline whose path is in neither HEAD nor the worktree is skipped and disclosed; any other automatic
  baseline failure names its ai-coord origin and the `--no-auto-baseline`/`--exclude-baseline` overrides.
- Prepared objects remain pinned until a terminal receipt expires or a prepared transaction is discarded.
- A commit is built from the prepared tree, with only clean current-HEAD movement and hook-staged changes admitted.
- When an intended prepared path differs from the physical worktree, verification hooks run against a temporary
  materialization of the complete prepared index, created outside the repository under ai-commit's state directory,
  with ignored local directories and matching initialized submodules projected in so installed tooling resolves without
  reaching the physical worktree. Those hooks may edit the message but must not modify tracked content; stat-only
  rewrites of identical bytes are not drift.
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
- Never remove an index lock that this transaction did not create; recovery may reclaim only a lock whose contents equal
  the transaction's persisted token. Hold the owned lock through verification hooks, ref CAS, and index
  reconciliation. Configured `[validation]` commands run without it; after re-acquiring the lock, `commit` requires the
  same branch, an idle repository, and a HEAD equal to the validated candidate's parent, rebuilding and revalidating
  onto a moved head at most three times before a retryable exit.
- Receipt cleanup deletes an expired transaction's lock file only after its journal, while holding that lock.
- A post-ref-update failure must remain replayable without creating a second commit.
- Tests isolate repositories, remotes, `HOME`, configuration, and state in temporary directories.

## Validation

From the monorepo root, run the narrowest relevant `cargo test -p ai-commit` filter first, then `just rust-check` for
the aggregate Rust gate. `just install-cli` installs every workspace binary under `~/.local`; run it only when the task
requires refreshing the installed CLIs.

## CLI reference

`ai-commit` separates commit analysis from mutation. `prepare` captures an immutable Git tree without changing the
shared index; `commit` later applies that exact snapshot to the current branch under Git's index lock, runs normal hooks
and signing against an isolated index, and reconciles only shared-index entries that have not changed in the meantime.

```console
$ ai-commit prepare -- src/main.rs
PREPARED 0123456789abcdef
...
$ ai-commit validate 0123456789abcdef
VALIDATED 0123456789abcdef 0123456789abcdef0123456789abcdef01234567
$ ai-commit commit 0123456789abcdef -m "feat: add safe transactions"
COMMITTED 0123456789abcdef 89abcdef0123
```

`validate` is optional. It lets an agent inspect the exact candidate and run configured repository validation before
composing the commit message; `commit` always validates the candidate again before hooks, signing, and commit creation.

### Installation

Requires Git, Cargo, and the rolling Rust nightly toolchain:

```sh
cargo install --git https://github.com/PaulRBerg/agent-toolkit ai-commit --locked --root "$HOME/.local"
```

For local development, install the current checkout instead:

```sh
cargo install --path . --locked --force --root "$HOME/.local"
```

### Commands

```text
ai-commit prepare [--all|--staged] [--natural|--conventional]
                  [--diff summary|full] [--exclude-baseline path=oid]...
                  [--no-auto-baseline]
                  [--porcelain] -- [paths...]
ai-commit validate <transaction-id>
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
skips discovery because it captures the index exactly. An automatic baseline whose path exists in neither HEAD nor
the worktree has nothing to exclude; it is skipped and disclosed as `AUTO_BASELINE_SKIPPED<tab>path<tab>oid` (ordinary
output: `skipped auto baselines`). Any other automatic baseline failure stops preparation with a message naming the
ai-coord record and suggesting `--no-auto-baseline` or an explicit `--exclude-baseline` override.

Before verification hooks run, `commit` compares the transaction's intended paths in the prepared index with the
physical shared worktree. Unrelated dirty paths do not affect hook execution. When every intended path matches and no
`[validation]` command is configured, hooks retain their normal behavior: tracked changes they stage can enter the
commit, and newly added paths are reported as `HOOK_ADDED`. A configured `[validation]` command always selects
snapshot-check hook mode, even when every intended path matches the physical worktree, because that mode's temporary
materialization is also validation's candidate worktree. When an intended path differs, `pre-commit`,
`prepare-commit-msg`, and `commit-msg` instead run from a temporary materialization of the complete prepared index
under ai-commit's state directory (`<state>/tmp/ai-commit-snapshot-*/worktree`), outside the repository, so upward
configuration and `node_modules` lookups never climb into the physical worktree. They receive the existing alternate
`GIT_INDEX_FILE`, `GIT_WORK_TREE` pointing to that materialization, `AI_COMMIT_HOOK_MODE=snapshot-check`, and
`AI_COMMIT_ORIGINAL_WORKTREE` pointing to the canonical physical repository root. Ignored local directories whose parents
exist in the prepared tree (for example `node_modules` or a virtual environment), including ignored symlinks to
directories, are projected one level deep: each is a real directory whose entries link to their physical paths,
`@scope` directories are expanded one more level, and symlinks are recreated so targets inside the repository (such as
workspace links like `node_modules/@scope/pkg -> ../../packages/pkg`) resolve inside the materialization while targets
outside it resolve to the same physical location. An initialized submodule whose checked-out HEAD equals the
candidate's gitlink commit has its entries (not its `.git`) projected the same way; other submodules stay empty. Hooks
therefore need no snapshot-specific branches. Absolute paths recorded inside dependency files, such as editable-install
`.pth` entries, and packages resolved through their physical real paths still refer to the physical worktree. The
materialization is skipped when no executable verification hook file would run (none is installed, or only
`pre-commit`/`commit-msg` exist and `--no-verify` is given) and no `[validation]` command is configured.

Snapshot-check hooks may edit the commit message, but any tracked-content or prepared-index change stops the commit
with `snapshot-check hook modified prepared content`, lists the affected paths, and leaves the transaction prepared for
explicit recovery. Drift is judged by content and mode after a stat refresh, so rewriting identical bytes (`touch`,
`sed -i`) is not drift. Ordinary hook failures remain retryable. Temporary hook state is removed on success or failure;
materializations abandoned by interrupted processes are removed by a later snapshot once their owner lock is free and
they are at least five minutes old. `post-commit` always runs through the physical worktree without snapshot-check
markers and, as with `git commit`, with `GIT_INDEX_FILE` naming the real shared index. This isolates conventional
relative Git and worktree operations; it does not constrain hooks that deliberately perform external side effects.

State defaults to `$XDG_STATE_HOME/ai-commit` or `~/.local/state/ai-commit`; `AI_COMMIT_STATE_DIR` overrides it.
Message format configuration is repository-local at `<git-root>/.agents/commit.toml`:

```toml
[message]
format = "natural"

[validation]
command = ["cargo", "test", "--locked"]
```

`format` must be `"natural"` or `"conventional"`. An absent file defaults to conventional format; an invalid file is a
usage error. Explicit `--natural` or `--conventional` always wins for that preparation.

`validation.command` is optional. When configured, it must be one non-empty argv vector (no shell form, empty argv
elements, or NUL bytes). `prepare` freezes that argv in its journal without running it. Both `validate` and `commit`
execute the frozen command directly from a temporary complete materialization of the exact commit candidate, even
when the physical worktree has changed. `commit` runs it before Git verification hooks, including with `--no-verify`,
without holding Git's shared index lock, so other Git operations in the worktree are not blocked; afterwards it
re-acquires the lock and, if the branch moved meanwhile, re-applies the prepared delta onto the new HEAD and validates
again, exiting `3` after three attempts that each saw movement.
If HEAD advanced cleanly, the candidate includes the immutable prepared delta applied to that observed HEAD. The
validator receives `GIT_DIR` for the physical repository,
`GIT_WORK_TREE` and `GIT_INDEX_FILE` for the materialization, `AI_COMMIT_VALIDATION_MODE=prepared-tree`, and
`AI_COMMIT_ORIGINAL_WORKTREE` for the canonical physical root; inherited conflicting Git and ai-commit hook variables
are cleared or replaced. Ignored local directories and matching initialized submodules are projected into the
materialization as for snapshot-check hooks, so validators can resolve installed tooling and local evidence; projected
entries are links to physical files, so validators must treat them as read-only. A nonzero exit, or a validator that changes tracked worktree or staged/index content, admits no changes
and leaves the transaction prepared for retry. When both happen, both facts are reported. A same-ID retry is suitable
after repairing only a transient dependency or environment failure. A content or configuration repair requires
reviewing the failure, preserving excluded baseline bytes, discarding the confirmed uncommitted preparation, and
preparing the corrected explicit owned scope again.

Successful preflight prints `VALIDATED <transaction-id> <full-candidate-tree-oid>` for configured validation of that
candidate only; hooks, signing, and a later commit can still fail. It runs no hooks, signer, commit,
push, or shared-index mutation, and it does not cache success; branch or HEAD movement invalidates the result, and the
final `commit` still runs validation and all existing checks. Without a frozen command, preflight prints
`VALIDATION_SKIPPED <transaction-id> no-configured-command` and performs no validation. Temporary materialization and
Git environment isolation do not sandbox arbitrary validator side effects or writes through projected ignored
directories. Repositories without `[validation]` retain the existing hook, signing, push, receipt, and
physical-worktree behavior; journals created before this option remain loadable.

`show` appends `pending-commit<TAB><full-oid>` when commit recovery is pending and `pending-commit<TAB>-` otherwise.
Pending state must be recovered by retrying `commit` with the same transaction ID; `validate` does not replace or
discard it.

`prepare --porcelain` emits stable TSV records. Tabs, newlines, carriage returns, and backslashes inside fields are
backslash-escaped. Outcome records use `PREPARED`, `VALIDATED`, `VALIDATION_SKIPPED`, `COMMITTED`, `PUSHED`,
`PUSHED_NEW`, `BEHIND`, `HOOK_ADDED`, and `DISCARDED`. Each automatically applied exclusion is disclosed as
`AUTO_BASELINE<tab>path<tab>oid`; the ordinary output lists the same pairs under `auto-applied baselines`. Each skipped
automatic baseline is disclosed as `AUTO_BASELINE_SKIPPED<tab>path<tab>oid`.

Receipts and retryable diagnostics print a fixed 12-character commit OID abbreviation; `show` and the transaction
journal retain full OIDs. The `--diff full` display diff omits binary patch payloads and caps each file's section at
400 lines, disclosing every cut as `DIFF_TRUNCATED<tab>path<tab>omitted-line-count` (ordinary output:
`DIFF_TRUNCATED path (N more lines)` after the diff). Truncation is display-only: the prepared tree, name-status,
shortstat, and path records stay complete.

Exit status `0` means success or an idempotent replay, `2` means invalid invocation or configuration, and `3` means the
repository was left safe but needs a retry or reconciliation. Other Git, hook, signing, and push failures return `1`.
Pushes always fetch and compare first; they never pull, merge, or rebase.

### Development

The crate targets macOS and Linux with the rolling Rust nightly toolchain.

Licensed under MIT.
