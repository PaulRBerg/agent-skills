---
argument-hint: '[--all] [--staged] [--deep] [--natural] [--push] [--close <issue_numbers>]'
name: commit
user-invocable: true
description: 'Use only when explicitly invoked for Git commit workflows: stage intended changes, craft Conventional Prefix Format messages by default, Natural Language messages with --natural or configured repos, commit, and optionally --all, --staged, --deep, --close, or --push.'
---

# Git Commit

Create atomic commits by staging the right files, analyzing the staged diff, composing a commit message, and optionally pushing.

## Workflow

### 1) Parse arguments

Arguments: `$ARGUMENTS`

- Flags:
  - `--all` commit all changes
  - `--staged` commit exactly the current index; do not auto-stage or unstage (conflicts with `--all`)
  - `--deep` deep analysis with the active session model, breaking changes, concise body
  - `--natural` force Natural Language Format
  - `--push` push after commit
  - `--close <issue_numbers>` append `Closes #N` trailers for listed issues (comma/space-separated)
- Value arguments:
  - Conventional Prefix Format: type keyword overrides inferred type
  - Natural Language Format: leading verb/category keyword overrides inferred verb
  - Quoted text overrides inferred description or subject

Pass `--natural` through to the prepare helper when requested. The helper resolves the message format from the target repository cwd. Never `cd` into the skill directory.

### 2) Prepare staged diff

Run the portable helper from the target repository cwd. Never `cd` into the skill directory, and never use dynamic `!` shell injection.

Resolve `<skill-dir>` from the loaded `SKILL.md` path:

```bash
bash "<skill-dir>/scripts/prepare-commit.sh" [--all] [--staged] [--natural] [--diff summary|full] -- [session_modified_paths...]
```

Use `--diff summary` by default. Use `--diff full` only when the intent is ambiguous or `--deep` was requested.

The helper performs Git preflight checks, stages `--all` or the session-modified paths (or, with `--staged`, leaves the current index untouched), unstages unrelated pre-staged paths, rejects empty staged diffs, and prints the message format, branch, staged name-status, shortstat, and optional full diff. If it fails, stop with its error and a concise suggested fix.

The helper also acquires a repo+branch commit lock before staging. If it fails because another agent is already committing for this repo and branch, stop and tell the user that committing is blocked by another active commit attempt. Do not describe internal lock paths, tokens, retries, or metadata.

On success, save the `## commit lock token` value from the helper output. Treat it as internal. Release it exactly once with:

```bash
bash "<skill-dir>/scripts/commit-lock.sh" release "<commit-lock-token>"
```

Run the release command after the commit workflow completes, including after optional push. If any post-prepare step fails or you stop before committing, release the lock before reporting the failure.

- If `--all`:
  - Include all tracked, untracked, modified, deleted, and already staged changes
- If `--staged`:
  - Commit exactly what is already staged; pass no session paths. The helper neither stages nor unstages. Conflicts with `--all`.
- Otherwise (atomic commits):
  - Session-modified files = files edited in this session
  - Pass every session-modified path after `--`
  - The helper stages only those paths and unstages unrelated pre-staged paths
- **Unrelated changes**: session-modified files may contain pre-existing uncommitted changes (hunks not from this session). Include the entire file—partial staging is impractical. Never revert, discard, or `git checkout` unrelated changes.

### 3) Analyze + compose message

Read the helper output and produce the commit message in a single pass.

**Message format** — use the `## message format` value from the helper output.

- If `conventional`: read [references/conventional-prefix-format.md](references/conventional-prefix-format.md).
- If `natural`: read [references/natural-language-format.md](references/natural-language-format.md).

Read only the selected format reference before composing the message.

**Unrelated hunks** — ignore pre-existing changes when determining type/scope/description. If unrelated changes are in the same file as session changes, they are included in the commit scope but should not influence the message.

**Issue linking** — scan the chat transcript for GitHub issue references (e.g. `#123`, `owner/repo#123`, issue URLs) that the current changes resolve. For each match, append a `Closes #N` trailer. Skip issues merely mentioned in passing; include only ones the commit actually closes.

**If `--deep`:**

- Deep semantic analysis; detect breaking changes
- Infer Conventional Prefix Format scope or Natural Language context from code structure even when path isn't clear
- Follow the selected reference's body and breaking-change rules

**If `--close`:**

- Append a `Closes #N` line for each issue number provided
- Multiple issues: one `Closes #N` per line in the body/trailer
- Merge with transcript-scanned issues; de-duplicate

### 4) Commit

- Use `git commit -m "subject"` (add `-m "body"` only if body is non-empty)
- Output exactly: commit hash, subject, and `N files changed` summary. Nothing else.
- Do not report branch ahead/behind counts, unpushed commits, push availability, unrelated tree state, staging steps, or pre-commit hook activity unless a command failed.
- If failed: show error + suggest fix
- **Pre-commit hook failure:** if the hook fails on unrelated/pre-existing changes (not this session's changes), retry automatically with `git commit --no-verify` — do not ask. Report the bypass in one line, noting the failure was unrelated to the staged changes. Never bypass hooks for failures caused by the session's own changes; fix those or surface the error instead.
- Keep the commit lock held through hook retries, then release it before reporting the final result or final failure.

### 5) Push (if `--push`)

- If upstream exists: `git push`
- If no upstream: `git push -u origin HEAD`
- If failed: show error + suggest fix (pull/rebase first, set upstream, check auth)
- Release the commit lock after push succeeds or fails, before the final user-facing response.
