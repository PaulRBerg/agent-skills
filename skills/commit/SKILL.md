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

**Message format** — use the `## message format` value from the helper output.

- If `conventional`: read [references/conventional-prefix-format.md](references/conventional-prefix-format.md).
- If `natural`: read [references/natural-language-format.md](references/natural-language-format.md).

Read only the selected format reference before composing the message.

**Unrelated hunks** — ignore pre-existing changes when determining type/scope/description. Without a baseline exclusion,
unrelated changes in the same file are included in the commit scope but should not influence the message.

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

**Agent-Session attribution** — run once, independent of the diff:

```bash
ai-coord trailer
```

When the CLI exists and this exits `0`, append its `Agent-Session: <client>/<id>` output alongside any `Closes #N`
trailers. When the CLI is missing or the command exits nonzero, skip silently — the catalog skill must keep working on
machines without it. This makes committed changes attributable to an agent session (`git log` shows which session
authored what).

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
  pre-commit hook activity unless a command failed.
- If the helper reports that the default index remains locked, wait and retry the same helper command; never delete the
  lock. If it reports that a commit was created but shared-index reconciliation failed, stop and report that commit ID;
  never retry the commit.
- **Pre-commit hook failure:** `Failed to get staged files!` and a bare `"lint-staged" exited with code 1` do not by
  themselves prove contention. Retry as contention only when the same output explicitly names an index lock or the
  helper reports its lock refusal. Otherwise inspect the named hook output or lint-staged debug trace. Retry with
  `--no-verify` only when that evidence plus the prepared diff conclusively shows an unrelated pre-existing failure. A
  generic failure, repo-wide check, or uncertain ownership is not enough. Never bypass a failure caused by or plausibly
  affected by the intended paths; fix it or surface it. When bypassing, keep the existing one-line disclosure that the
  unrelated hook failure was skipped.
- **Signing failure (signer unreachable):** if `git commit` fails _after_ the pre-commit/commit-msg hooks already
  passed, with an error naming the configured signer rather than the content or a hook (e.g. `1Password`,
  `failed to fill whole buffer`, `ssh-agent`, `gpg failed to sign the data`, `no such identity`) — retry once, same
  command, with `--no-gpg-sign` appended. Interactive/hardware signers (1Password, YubiKey, etc.) can be unreachable
  when unattended, and the user has authorized landing unsigned commits in that case rather than blocking. Only retry on
  a genuine signer error at the signing step, never speculatively, and never edit repo/global git config
  (`commit.gpgsign`, `gpg.format`, etc.) — the bypass is per-commit only. Disclose with one line:
  `Commit created unsigned — signer unavailable ("<short error>")`. In default mode, append `--no-gpg-sign` to the
  `commit-paths.sh commit` command; keep direct Git flags for `--all` and `--staged`.
  - **Session memo:** once a genuine signer error has triggered the fallback in this session, treat the signer as
    unavailable for the rest of it: later commits may append `--no-gpg-sign` on the first attempt instead of re-failing.
    Still per-commit only — never touch git config. Replace the per-commit disclosure with a single line in the
    session's final receipt: `N commits created unsigned — signer unavailable ("<short error>")`.

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
