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

Create atomic commits by staging the right files, analyzing the staged diff, composing a commit message, and pushing
when explicitly requested or authorized by standing instructions.

## Workflow

Do not run extra Git inspection commands (`git status`, `git log`, or `git diff`) around the helpers; helper output is
the complete evidence for composing the message and receipt.

### 1) Parse arguments

Arguments: `$ARGUMENTS`

- Flags:
  - `--all` commit all changes
  - `--staged` commit exactly the current index; do not auto-stage or unstage (conflicts with `--all`)
  - `--natural` force Natural Language Format
  - `--push` explicitly request a push after commit; otherwise push only when standing instructions authorize it
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
shortstat, and optional full diff. `--all` and `--staged` stage or read the shared index directly (index-trusting by
design). Default mode delegates to `commit-paths.sh preview`, which builds a temporary index from `HEAD` and never
changes the shared index. If preparation fails, stop with its error and a concise suggested fix.

- If `--all`:
  - Include all tracked, untracked, modified, deleted, and already staged changes — this also sweeps in any other
    agent's in-flight work by design; see the step 4 output contract for the resulting flag.
- If `--staged`:
  - Commit exactly what is already staged; pass no session paths. The helper neither stages nor unstages. Conflicts with
    `--all`. The index may hold another agent's staged files; committing it verbatim is the user's explicit choice.
- Otherwise (atomic commits):
  - Session-modified files = files edited in this session
  - Pass every session-modified path after `--`
  - The helper prints the exact old and new file paths under `## commit paths`; pass that list to the commit helper in
    step 4
  - **Renames**: pass both the old and new name as session paths, including for case-only file or directory renames
- **Unrelated changes**: session-modified files may contain pre-existing uncommitted changes (hunks not from this
  session). Preserve `stale-dirt:` baselines as described in step 4. Without a baseline, include the entire file. Never
  revert, discard, or `git checkout` unrelated changes.

### 3) Analyze + compose message

Read the helper output and produce the commit message in a single pass.

**Message format** — use the `## message format` value from the helper output. Its `## message format rules` section
contains the complete selected rules; compose the message from those rules.

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

**Agent-Session attribution** — when the helper output includes an optional `## trailer` section, append its line as a
trailer alongside any `Closes #N` trailers.

### 4) Commit

- Default mode (no `--all`/`--staged`): run the helper from the target repository cwd:

  ```bash
  bash "<skill-dir>/scripts/commit-paths.sh" commit -m "subject" [-m "body"] \
    [--exclude-baseline "<path>=<oid>"]... -- \
    <paths from the "## commit paths" section>
  ```

  When `ai-coord start` returned a `stale-dirt:<paths>` advisory covering an intended path, run `ai-coord baseline` and
  pass `--exclude-baseline "<path>=<oid>"` for each affected intended path. The helper applies only the
  baseline-to-worktree change onto locked `HEAD`; if any patch does not apply cleanly, it aborts the whole commit and
  leaves the worktree and shared index unchanged.

  The helper rejects an inherited `GIT_INDEX_FILE`, waits only on an explicit default-index lock, and holds that lock
  through the commit. It builds the commit from locked `HEAD` in a separate index whose name does not end in `.lock`, so
  normal hooks and signing run without exposing formatter staging to the shared index. After success, it updates only
  the committed path entries in the locked shared index; unrelated staging remains intact.

- `--all` / `--staged`: commit the prepared index as-is with `git commit -m "subject"` (add `-m "body"` only if body is
  non-empty).
- Output exactly: commit hash, subject, and `N files changed` summary. In `--all` mode, if the committed set plausibly
  includes files not modified in this session, also print one line listing those files so the user can catch an
  accidental sweep of another agent's work. Nothing else. This exact receipt is intentionally plain: do not add emoji,
  headings, trees, or labels.
- Do not report branch ahead/behind counts, unpushed commits, push availability, unrelated tree state, staging steps, or
  pre-commit hook activity unless the push workflow requires the user to reconcile a behind branch or a command failed.
- Never delete index locks; wait and retry the same command. Never retry a commit after the helper reports a created
  commit with failed shared-index reconciliation; stop and report the commit ID. On any pre-commit hook or signing
  failure, read [references/failure-recovery.md](references/failure-recovery.md) before deciding on a bypass; bypasses
  always require their one-line disclosure.

### 5) Push when requested or authorized

Run this step when `--push` was supplied or standing instructions authorize automatic pushing for the current
repository. Otherwise stop after the commit receipt.

In default mode, append `--push` to the `commit-paths.sh commit` invocation. In `--all` / `--staged` modes, run
`bash "<skill-dir>/scripts/commit-paths.sh" push` after the direct `git commit`. Report the helper's one-line outcome;
when it reports `behind <n> — push skipped`, explicitly state that the push was skipped for user reconciliation.

## Completion

Completion evidence is the created commit hash, subject, and changed-file count. When the push workflow applies, also
require the push helper's successful outcome line unless the branch is behind; in that case, completion requires its
`behind <n> — push skipped` outcome and explicitly reporting that the push was skipped for user reconciliation. A hook
bypass is complete only with the one-line unrelated-failure disclosure; a signing bypass is complete only with the
one-line unsigned-commit disclosure.
