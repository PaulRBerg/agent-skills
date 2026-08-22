---
argument-hint: "[--all] [--staged] [--natural] [--push] [--close <issue_numbers>] [--finding <finding_ids>]"
compatibility: Requires Git and ai-commit with automatic ai-coord stale-dirt baseline discovery on PATH.
effort: medium
name: commit
description:
  "Commit staged or intended changes: compose a Conventional Prefix or Natural Language message, then use ai-commit —
  with --all, --staged, --close, --finding, or --push."
---

# Git Commit

Compose the semantic commit message from immutable evidence. Let `ai-commit` own deterministic preparation, commit,
index reconciliation, and push mechanics.

Run every command from the target repository cwd. Do not surround `ai-commit` with extra `git status`, `git log`, or
`git diff` calls: preparation provides the complete evidence for this workflow. Use `--porcelain` only in deterministic
automation that explicitly parses its stable TSV records; the normal skill flow consumes the human-readable sections.

## 1. Parse Arguments

Arguments: `$ARGUMENTS`

- `ai-commit` selects the message format from repository config; pass `--natural` or `--conventional` only as an
  explicit one-off override, never to reproduce repository policy.
- `--all`: capture all worktree and index changes. This intentionally risks including another agent's work.
- `--staged`: capture exactly the current index; do not add session paths. It conflicts with `--all`.
- `--natural`: force Natural Language Format for this commit.
- `--conventional`: force Conventional Prefix Format for this commit.
- `--push`: request a push after commit. Otherwise push only when standing instructions authorize it.
- `--close <issue_numbers>`: append one `Closes #N` trailer per positive decimal issue number; accept comma- or
  space-separated input.
- `--finding <finding_ids>`: append one `Finding-ID: <id>` trailer per ledger finding this commit fixes; accept comma-
  or space-separated input. Also infer a finding from context when this session's commit fixes a specific ledger
  finding. Never include or resolve findings the commit does not actually fix.
- In Conventional Prefix Format, a positional type keyword overrides the inferred type. In Natural Language Format, a
  positional verb or category keyword overrides the inferred verb. Quoted positional text overrides the inferred
  description or subject.

If the requested operation is only to push a clean branch that is already ahead, skip preparation and run
`ai-commit push`.

## 2. Prepare Once

Run one preparation command:

```bash
ai-commit prepare [--all | --staged] [--natural | --conventional] --diff full \
  [--exclude-baseline '<path>=<oid>']... [-- <session-modified-paths>...]
```

- Default mode requires every path edited in this session. For a rename, include both old and new names, including
  case-only file or directory renames.
- Before default-mode preparation, run `ai-coord touched` when available and reconcile its output against the session
  path list: add missed session-edited paths, but ignore paths this session did not semantically change because touched
  paths are best-effort evidence, not authority. Skip this cross-check silently when the command is unavailable or the
  session is unrecognized.
- `--all` accepts no explicit paths and captures all tracked, untracked, modified, deleted, and staged changes.
- `--staged` accepts neither explicit paths nor baseline exclusions and captures the shared index exactly.
- `ai-commit prepare` automatically applies this session's ai-coord stale-dirt baselines. Auto-applied exclusions appear
  in the preparation evidence and must be disclosed unchanged. Explicit `--exclude-baseline` remains available for
  overrides, and `--no-auto-baseline` disables discovery for deterministic automation. Never revert unrelated changes.

Preparation pins the exact tree and delta under the printed transaction ID without changing the shared index. Keep that
ID. The later commit reuses the transaction instead of recomputing intended content from the mutable worktree or shared
index. It applies that immutable delta to the locked current branch and fails safely if intervening branch movement
conflicts. If preparation fails, stop with its error and the smallest safe correction.

## 3. Analyze and Compose

Analyze the single prepared full diff. Do not prepare again to get different evidence. Oversized per-file sections
arrive cut with a `DIFF_TRUNCATED` disclosure; their name-status and shortstat evidence still governs the message.

- Use the printed message format and message-format rules. `ai-commit` is the source of those rules; do not load a
  separate Conventional or Natural reference.
- Apply positional overrides, detect breaking changes, infer scope or context from the code, and include a body only
  when it adds material rationale.
- Add `Closes #N` trailers from `--close` and from transcript issue references only when this commit actually resolves
  them. De-duplicate issue numbers.
- Add one `Finding-ID: <id>` trailer per finding supplied by `--finding` or inferred from context that this commit
  actually fixes. De-duplicate finding IDs.
- Append the exact `Agent-Session:` line from the preparation trailer section when present. `ai-commit` has already
  validated it; do not synthesize or repair a missing or malformed trailer.

Compose one subject paragraph, an optional body paragraph, and one final trailer paragraph containing all issue and
Agent-Session lines.

`ai-commit` receives every `-m` value verbatim and rejects a literal `\\n`; write real line breaks inside the quoted
argument, as in the step 4 example.

## 4. Commit the Transaction

Run:

```bash
ai-commit commit <transaction-id> -m '<subject>' [-m '<body>'] [-m '<trailers>'] [--push]
```

For example, a two-item body is one `-m` argument containing a physical newline:

```bash
ai-commit commit <transaction-id> -m '<subject>' -m '- first material change
- second material change' [-m '<trailers>'] [--push]
```

Append `--push` when explicitly requested or authorized by standing instructions. The same command handles default,
`--all`, and `--staged` transactions; never stage or commit them with direct Git commands.

Transactions are idempotent. After an interruption, lock race, or retryable exit, retry the same transaction ID and
message arguments; do not prepare a replacement from newer mutable state. A replay recovers or returns the retained
receipt without creating a duplicate commit. Never delete an index lock.

The exact `snapshot-check hook modified prepared content` diagnostic is the one exception: do not retry that
transaction. Follow the discard, owned-content correction, and single reprepare procedure in the recovery reference.

Read [references/failure-recovery.md](references/failure-recovery.md) before adding `--no-verify` or `--no-gpg-sign`.
Those are explicit per-attempt recovery options, not first-attempt defaults.

After `COMMITTED <transaction-id> <commit-oid>` (the OID is a 12-character abbreviation; pass it as-is), resolve every
included finding with `ai-coord finding resolve '<id>' --as fixed --commit '<commit-oid>'` and report the resolved
finding IDs in the receipt summary.

## 5. Interpret the Receipt

Keep the receipt compact and forward its outcome lines without decoration:

- `COMMITTED <transaction-id> <commit-oid>` proves commit creation or idempotent recovery; without push authorization it
  is completion.
- `HOOK_ADDED <path>` identifies content introduced by a hook outside the prepared path set. Disclose every such line.
- `PUSHED <branch>` or `PUSHED_NEW <branch>` proves propagation and completes push-authorized or push-only work.
- `PUSHED <transaction-id> <commit-oid>` is the retained proof returned when an already-pushed transaction is replayed.
- `BEHIND <branch> <count>` is safe noncompletion, never completion: `ai-commit` fetched and refused to integrate or
  push. A preceding `COMMITTED` still proves the local commit; after the user reconciles the branch, replay the same
  transaction command (or rerun `ai-commit push` for push-only work).

Do not report unrelated tree state, ahead/behind counts not emitted by the command, staging narration, or successful
hook activity. Add only a required one-line bypass disclosure from the recovery reference.
