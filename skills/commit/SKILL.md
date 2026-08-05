---
argument-hint: "[--all] [--staged] [--natural] [--push] [--close <issue_numbers>]"
compatibility: Requires Git and ai-commit >=0.1.0 on PATH.
disable-model-invocation: false
effort: medium
name: commit
user-invocable: true
description:
  "Commit staged or intended changes: compose a Conventional Prefix or Natural Language message, then use ai-commit —
  with --all, --staged, --close, or --push."
---

# Git Commit

Compose the semantic commit message from immutable evidence. Let `ai-commit` own deterministic preparation, commit,
index reconciliation, and push mechanics.

Run every command from the target repository cwd. Do not surround `ai-commit` with extra `git status`, `git log`, or
`git diff` calls: preparation provides the complete evidence for this workflow. Use `--porcelain` only in deterministic
automation that explicitly parses its stable TSV records; the normal skill flow consumes the human-readable sections.

## 1. Parse Arguments

Arguments: `$ARGUMENTS`

- `--all`: capture all worktree and index changes. This intentionally risks including another agent's work.
- `--staged`: capture exactly the current index; do not add session paths. It conflicts with `--all`.
- `--natural`: force Natural Language Format.
- `--push`: request a push after commit. Otherwise push only when standing instructions authorize it.
- `--close <issue_numbers>`: append one `Closes #N` trailer per positive decimal issue number; accept comma- or
  space-separated input.
- In Conventional Prefix Format, a positional type keyword overrides the inferred type. In Natural Language Format, a
  positional verb or category keyword overrides the inferred verb. Quoted positional text overrides the inferred
  description or subject.

If the requested operation is only to push a clean branch that is already ahead, skip preparation and run
`ai-commit push`.

## 2. Prepare Once

Run one preparation command:

```bash
ai-commit prepare [--all | --staged] [--natural] --diff full \
  [--exclude-baseline '<path>=<oid>']... [-- <session-modified-paths>...]
```

- Default mode requires every path edited in this session. For a rename, include both old and new names, including
  case-only file or directory renames.
- `--all` accepts no explicit paths and captures all tracked, untracked, modified, deleted, and staged changes.
- `--staged` accepts neither explicit paths nor baseline exclusions and captures the shared index exactly.
- When `ai-coord start` reported `stale-dirt:<paths>`, run `ai-coord baseline` and pass every returned path/blob pair as
  a repeated `--exclude-baseline '<path>=<oid>'`. Without that baseline, preparation includes the whole current file.
  Never revert unrelated changes.

Preparation pins the exact tree and delta under the printed transaction ID without changing the shared index. Keep that
ID. The later commit reuses the transaction instead of recomputing intended content from the mutable worktree or shared
index. It applies that immutable delta to the locked current branch and fails safely if intervening branch movement
conflicts. If preparation fails, stop with its error and the smallest safe correction.

## 3. Analyze and Compose

Analyze the single prepared full diff. Do not prepare again to get different evidence.

- Use the printed message format and message-format rules. `ai-commit` is the source of those rules; do not load a
  separate Conventional or Natural reference.
- Apply positional overrides, detect breaking changes, infer scope or context from the code, and include a body only
  when it adds material rationale.
- Add `Closes #N` trailers from `--close` and from transcript issue references only when this commit actually resolves
  them. De-duplicate issue numbers.
- Append the exact `Agent-Session:` line from the preparation trailer section when present. `ai-commit` has already
  validated it; do not synthesize or repair a missing or malformed trailer.

Compose one subject paragraph, an optional body paragraph, and one final trailer paragraph containing all issue and
Agent-Session lines.

## 4. Commit the Transaction

Run:

```bash
ai-commit commit <transaction-id> -m '<subject>' [-m '<body>'] [-m '<trailers>'] [--push]
```

Append `--push` when explicitly requested or authorized by standing instructions. The same command handles default,
`--all`, and `--staged` transactions; never stage or commit them with direct Git commands.

Transactions are idempotent. After an interruption, lock race, or retryable exit, retry the same transaction ID and
message arguments; do not prepare a replacement from newer mutable state. A replay recovers or returns the retained
receipt without creating a duplicate commit. Never delete an index lock.

Read [references/failure-recovery.md](references/failure-recovery.md) before adding `--no-verify` or `--no-gpg-sign`.
Those are explicit per-attempt recovery options, not first-attempt defaults.

## 5. Interpret the Receipt

Keep the receipt compact and forward its outcome lines without decoration:

- `COMMITTED <transaction-id> <commit-oid>` proves commit creation or idempotent recovery.
- `HOOK_ADDED <path>` identifies content introduced by a hook outside the prepared path set. Disclose every such line.
- `PUSHED <branch>` or `PUSHED_NEW <branch>` proves propagation.
- `PUSHED <transaction-id> <commit-oid>` is the retained proof returned when an already-pushed transaction is replayed.
- `BEHIND <branch> <count>` is safe noncompletion: `ai-commit` fetched and refused to integrate or push. A preceding
  `COMMITTED` line still proves the local commit, but the push workflow is incomplete until the user reconciles the
  branch and the same transaction command is replayed. For push-only work, rerun `ai-commit push` after reconciliation.

Do not report unrelated tree state, ahead/behind counts not emitted by the command, staging narration, or successful
hook activity. Add only a required one-line bypass disclosure from the recovery reference.

## Completion

Without push authorization, completion requires `COMMITTED`. With push authorization, completion also requires `PUSHED`
or `PUSHED_NEW`; `BEHIND` is not completion. A push-only request completes on `PUSHED` or `PUSHED_NEW`.
