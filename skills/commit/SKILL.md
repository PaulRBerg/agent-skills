---
argument-hint: '[--all] [--deep] [--push] [--close <issue_numbers>]'
name: commit
user-invocable: true
description: 'Use only when explicitly invoked for Git commit workflows: stage intended changes, craft conventional commit messages, commit, and optionally --all, --deep, --close, or --push.'
---

# Git Commit

Create atomic commits by staging the right files, analyzing the staged diff, composing a conventional commit message, and optionally pushing.

## Workflow

### 1) Parse arguments

Arguments: `$ARGUMENTS`

- Flags:
  - `--all` commit all changes
  - `--deep` deep analysis with the active session model, breaking changes, concise body
  - `--push` push after commit
  - `--close <issue_numbers>` append `Closes #N` trailers for listed issues (comma/space-separated)
- Value arguments:
  - Type keyword (any conventional type) overrides inferred type
  - Quoted text overrides inferred description

### 2) Prepare staged diff

Run the portable helper from the target repository cwd. Never `cd` into the skill directory, and never use dynamic `!` shell injection.

For Claude Code:

```bash
bash "${CLAUDE_SKILL_DIR}/scripts/prepare-commit.sh" [--all] [--diff summary|full] -- [session_modified_paths...]
```

For Codex CLI, resolve `<skill-dir>` from the loaded `SKILL.md` path:

```bash
bash "<skill-dir>/scripts/prepare-commit.sh" [--all] [--diff summary|full] -- [session_modified_paths...]
```

Use `--diff summary` when the user supplied a clear subject or description. Use `--diff full` when the intent is ambiguous or `--deep` was requested.

The helper performs Git preflight checks, stages `--all` or the session-modified paths, unstages unrelated pre-staged paths, rejects empty staged diffs, and prints the branch, staged name-status, shortstat, and optional full diff. If it fails, stop with its error and a concise suggested fix.

- If `--all`:
  - Include all tracked, untracked, modified, deleted, and already staged changes
- Otherwise (atomic commits):
  - Session-modified files = files edited in this session
  - Pass every session-modified path after `--`
  - The helper stages only those paths and unstages unrelated pre-staged paths
- **Unrelated changes**: session-modified files may contain pre-existing uncommitted changes (hunks not from this session). Include the entire file—partial staging is impractical. Never revert, discard, or `git checkout` unrelated changes.

### 3) Analyze + compose message

Read the helper output and produce the commit message in a single pass.

**Type inference** — infer the type from the dominant user-visible intent, not the largest file diff or the presence of
dependency/config churn.

Choose the highest-signal behavior that explains why the commit exists:

- If a dependency bump is only the enabler for a migration/refactor/fix, use the migration/refactor/fix type instead of
  `chore(deps)`.
- If changed tooling/scripts/config are required to keep existing behavior working after a code migration, include them in
  the same type as the migration.
- Use `chore` only for maintenance that does not fit a more specific behavioral category.
- Use `chore(deps)` only for dependency-only updates or dependency updates whose main purpose is routine maintenance.

| Behavior                                            | Type          |
| --------------------------------------------------- | ------------- |
| New functionality                                   | `feat`        |
| Bug fix / error handling                            | `fix`         |
| Code migration or API adaptation without new UX/API | `refactor`    |
| Code reorganization, no behavior change             | `refactor`    |
| Documentation                                       | `docs`        |
| Tests                                               | `test`        |
| Build system (webpack, vite, esbuild)               | `build`       |
| CI/CD pipelines                                     | `ci`          |
| Dependency-only maintenance                         | `chore(deps)` |
| Formatting / whitespace only                        | `style`       |
| Performance                                         | `perf`        |
| Reverting previous commit                           | `revert`      |
| AI config (CLAUDE.md, .claude/, .gemini/, .codex/)  | `ai`          |
| Other maintenance                                   | `chore`       |

Explicit type keyword in arguments takes precedence over inference.

**Scope** — infer only when path makes it obvious (lowercase).

**Unrelated hunks** — ignore pre-existing changes when determining type/scope/description. If unrelated changes are in the same file as session changes, they are included in the commit scope but should not influence the message.

**Message format:**

- Subject line (\<= 50 chars): `type(scope): description` or `type: description`
- Imperative mood ("add" not "added"), lowercase, no period
- Describe what the change does, not which files changed
- Body: hyphenated lines for distinct changes; skip for trivial changes

**Issue linking** — scan the chat transcript for GitHub issue references (e.g. `#123`, `owner/repo#123`, issue URLs) that the current changes resolve. For each match, append a `Closes #N` trailer. Skip issues merely mentioned in passing; include only ones the commit actually closes.

**If `--deep`:**

- Deep semantic analysis; detect breaking changes
- Infer scope from code structure even when path isn't clear
- Body: 2-3 hyphenated lines max, focus on WHY
- Breaking change: `BREAKING CHANGE:` + one-line migration note

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

### 5) Push (if `--push`)

- If upstream exists: `git push`
- If no upstream: `git push -u origin HEAD`
- If failed: show error + suggest fix (pull/rebase first, set upstream, check auth)
