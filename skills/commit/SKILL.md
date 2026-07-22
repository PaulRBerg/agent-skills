---
argument-hint: "[--all] [--staged] [--natural] [--push] [--close <issue_numbers>]"
disable-model-invocation: false
effort: medium
name: commit
user-invocable: true
description:
  "Commit staged or intended changes: craft a Conventional Prefix or Natural Language message, then commit — with --all,
  --staged, --close, or --push."
---

# Git Commit

Create atomic commits by staging the right files, analyzing the staged diff, composing a commit message, and optionally
pushing.

## Workflow

### 1) Parse arguments

Arguments: `$ARGUMENTS`

- Flags:
  - `--all` commit all changes
  - `--staged` commit exactly the current index; do not auto-stage or unstage (conflicts with `--all`)
  - `--natural` force Natural Language Format
  - `--push` push after commit
  - `--close <issue_numbers>` append `Closes #N` trailers for listed issues (comma/space-separated)
- Value arguments:
  - Conventional Prefix Format: type keyword overrides inferred type
  - Natural Language Format: leading verb/category keyword overrides inferred verb
  - Quoted text overrides inferred description or subject

Pass `--natural` through to the prepare helper when requested. The helper resolves the message format from the target
repository cwd.

### 2) Prepare staged diff

Run the portable helper from the target repository cwd. Never `cd` into the skill directory, and never use dynamic `!`
shell injection.

Resolve `<skill-dir>` from the loaded `SKILL.md` path:

```bash
bash "<skill-dir>/scripts/prepare-commit.sh" [--all] [--staged] [--natural] [--diff summary|full] -- [session_modified_paths...]
```

Use `--diff summary` by default. Use `--diff full` only when the intent is ambiguous.

The helper performs Git preflight checks, rejects empty change sets, and prints the message format, branch, name-status,
shortstat, and optional full diff. It is safe to run alongside other agents committing in the same working tree: `--all`
and `--staged` stage or read the index directly (index-trusting by design); the default mode never stages, unstages, or
otherwise touches the index — it marks new session files with `git add -N` (intent-to-add, no content staged) so diffs
can see them, then diffs the session paths against `HEAD` in the working tree. If it fails, stop with its error and a
concise suggested fix.

- If `--all`:
  - Include all tracked, untracked, modified, deleted, and already staged changes — this also sweeps in any other
    agent's in-flight work by design; see the step 4 output contract for the resulting flag.
- If `--staged`:
  - Commit exactly what is already staged; pass no session paths. The helper neither stages nor unstages. Conflicts with
    `--all`. The index may hold another agent's staged files; committing it verbatim is the user's explicit choice.
- Otherwise (atomic commits):
  - Session-modified files = files edited in this session
  - Pass every session-modified path after `--`
  - The helper never stages, unstages, or touches paths another agent has staged; it prints a `## commit pathspec` list
    of the resolved session paths to commit with in step 4
  - **Renames**: pass both the old and new path of a renamed file as session paths, so the pathspec commit in step 4
    captures both sides
- **Unrelated changes**: session-modified files may contain pre-existing uncommitted changes (hunks not from this
  session). Include the entire file—partial staging is impractical. Never revert, discard, or `git checkout` unrelated
  changes.

### 3) Analyze + compose message

Read the helper output and produce the commit message in a single pass.

**Message format** — use the `## message format` value from the helper output.

- If `conventional`: read [references/conventional-prefix-format.md](references/conventional-prefix-format.md).
- If `natural`: read [references/natural-language-format.md](references/natural-language-format.md).

Read only the selected format reference before composing the message.

**Unrelated hunks** — ignore pre-existing changes when determining type/scope/description. If unrelated changes are in
the same file as session changes, they are included in the commit scope but should not influence the message.

**Issue linking** — scan the chat transcript for GitHub issue references (e.g. `#123`, `owner/repo#123`, issue URLs)
that the current changes resolve. For each match, append a `Closes #N` trailer. Skip issues merely mentioned in passing;
include only ones the commit actually closes.

**Analysis** — perform semantic analysis of the staged diff:

- Detect breaking changes
- Infer Conventional Prefix Format scope or Natural Language context from code structure even when the path isn't clear
- Follow the selected reference's body and breaking-change rules

**If `--close`:**

- Append a `Closes #N` line for each issue number provided
- Multiple issues: one `Closes #N` per line in the body/trailer
- Merge with transcript-scanned issues; de-duplicate

### 4) Commit

- Default mode (no `--all`/`--staged`): commit with an explicit pathspec —
  `git commit -m "subject" [-m "body"] -- <paths from the "## commit pathspec" section>`. A pathspec commit builds the
  commit from `HEAD` plus the working-tree content of exactly those paths, so concurrent staging by other agents cannot
  leak into the commit.
- `--all` / `--staged`: commit the prepared index as-is with `git commit -m "subject"` (add `-m "body"` only if body is
  non-empty).
- Output exactly: commit hash, subject, and `N files changed` summary. In `--all` mode, if the committed set plausibly
  includes files not modified in this session, also print one line listing those files so the user can catch an
  accidental sweep of another agent's work. Nothing else. This exact receipt is intentionally plain: do not add emoji,
  headings, trees, or labels.
- Do not report branch ahead/behind counts, unpushed commits, push availability, unrelated tree state, staging steps, or
  pre-commit hook activity unless a command failed.
- If failed: show error + suggest fix. If `git commit` itself fails on an index.lock error, wait a moment and retry;
  never delete the lock file.
- **Pre-commit hook failure:** retry automatically with `git commit --no-verify` only when the hook output identifies
  the failing check/path and the staged diff plus session scope conclusively show it is unrelated pre-existing work. A
  generic failure, repo-wide check, or uncertain ownership is not enough evidence — with parallel agents, the common
  cause of an unrelated repo-wide hook failure is another agent's in-flight work, but the same evidence bar applies.
  Never bypass a failure caused by or plausibly affected by the staged changes; fix it or surface the error. When
  bypassing, keep the existing one-line disclosure that the unrelated hook failure was skipped.
- **Signing failure (signer unreachable):** if `git commit` fails _after_ the pre-commit/commit-msg hooks already
  passed, with an error naming the configured signer rather than the content or a hook (e.g. `1Password`,
  `failed to fill whole buffer`, `ssh-agent`, `gpg failed to sign the data`, `no such identity`) — retry once, same
  command, with `--no-gpg-sign` appended. Interactive/hardware signers (1Password, YubiKey, etc.) can be unreachable
  when unattended, and the user has authorized landing unsigned commits in that case rather than blocking. Only retry on
  a genuine signer error at the signing step, never speculatively, and never edit repo/global git config
  (`commit.gpgsign`, `gpg.format`, etc.) — the bypass is per-commit only. Disclose with one line:
  `Commit created unsigned — signer unavailable ("<short error>")`.

### 5) Push (if `--push`)

- If upstream exists: `git push`
- If no upstream: `git push -u origin HEAD`
- If rejected as non-fast-forward: retry `git push` once (push races between agents are routine). If still rejected, run
  `git pull --rebase` only when `git status --porcelain` is clean; otherwise stop and report. Never use `--autostash` in
  a shared tree — it can stash another agent's uncommitted work and conflict on pop.
- If failed for another reason: show error + suggest fix (set upstream, check auth)

## Completion

Completion evidence is the created commit hash, subject, and changed-file count; with `--push`, also require the
successful remote update. A hook bypass is complete only with the one-line unrelated-failure disclosure; a signing
bypass is complete only with the one-line unsigned-commit disclosure.
