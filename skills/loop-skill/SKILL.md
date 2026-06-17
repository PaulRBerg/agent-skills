---
argument-hint: '<skill-invocation> [--runs <n>]'
disable-model-invocation: true
name: loop-skill
user-invocable: true
description: 'Explicit-only meta skill for running another agent skill repeatedly, defaulting to 3 iterations, with code-polish after code-changing runs and one final net report.'
---

# Loop Skill

Run another agent skill multiple times in sequence, polishing code-changing iterations and reporting only the final net result.

## Arguments

- Target skill invocation: required. Accept `$skill-name`, `skill-name`, a path to `SKILL.md`, or a natural-language instruction that names the skill and its arguments.
- `--runs <n>` or `-n <n>`: optional positive integer iteration count. Default: 3.
- Additional target arguments: forward unchanged to the target skill on every iteration.

## Preconditions

- Do not run in Plan mode. If the current mode, tools, or user instruction indicate planning-only work, stop and say: `$loop-skill` is execution-only and cannot run in Plan mode. Switch to execution/default mode and invoke it again.
- Require a target skill. If the request does not identify one, stop and ask for the skill invocation to repeat.
- Require `code-polish` to be installed as a sibling skill at `../code-polish/SKILL.md`. If missing, stop and recommend installing it with `bunx skills add` from a catalog that includes `code-polish` (for this catalog: `bunx skills add PaulRBerg/agent-skills`).
- Resolve the target skill before starting:
  - For `$skill-name` or `skill-name`, read `../skill-name/SKILL.md`.
  - For a path, read that `SKILL.md`.
  - For an already-loaded skill block in the conversation, use that block only when it clearly matches the requested target.
  - If the target cannot be resolved, stop and report the missing skill.
- Refuse to target `loop-skill` or `code-polish` directly.
- Refuse targets whose required workflow commits, tags, stashes, force-resets, or otherwise depends on intermediate git history. This loop makes no commits.

## Workflow

### 1. Initialize

1. Parse the run count. Use 3 unless the user explicitly asks for a different positive integer.
2. Read `../code-polish/SKILL.md` once.
3. Read the target skill instructions once.
4. Verify repository context with `git rev-parse --git-dir` when the target or `code-polish` may inspect diffs. If it fails, continue only if the target skill is explicitly non-repo work.
5. Snapshot the initial state for the final net report:
   - `git status --short`
   - `git diff --stat`
   - `git diff --name-only --diff-filter=ACMR`
   - `git ls-files --others --exclude-standard`

### 2. Iterate

For each iteration from 1 through the requested run count:

1. Record the pre-iteration changed-path set:
   - tracked: `git diff --name-only --diff-filter=ACMR`
   - untracked: `git ls-files --others --exclude-standard`
2. Run the target skill inline as if invoked with the user's target arguments.
3. Override the target skill with these loop constraints:
   - Do not create commits, tags, branches, stashes, or other persistent git history.
   - Do not produce a full user-facing report unless the target must stop or ask for direction.
   - Keep scope stable unless the target's own instructions require a narrower discovered scope.
4. If the target skill hits a stop condition, stop the loop and produce the net report for completed work plus the blocking condition.
5. Record the post-target changed-path set and compare it with the pre-iteration set.
6. If the target produced code changes, run `code-polish` inline on only those newly code-changed paths:
   - Pass a `resolved-scope` block with the code-changed paths.
   - Instruct `code-polish` not to broaden or rediscover scope.
   - Preserve all loop constraints, especially no commits.
7. Skip `code-polish` when the target produced no code changes. Also skip when changes are only docs, prose, generated outputs, assets, lockfiles, or vendored files unless the user explicitly asked to polish those files as code.
8. If `code-polish` cannot complete, stop the loop and produce the net report with the incomplete polish called out.

Treat these as code changes unless local context proves otherwise: source files, tests, build scripts, package manifests, app configs, schemas, migrations, and executable scripts.

### 3. Verify

After the final completed iteration, verify the net final state with the narrowest command that proves the touched behavior.

- Prefer project recipes such as `just test`, `just lint`, `just mdformat-check`, or a narrower recipe when present.
- Reuse verification already run by the final `code-polish` pass when it covers the final touched scope.
- Name skipped checks and why.

### 4. Report

Produce one final report only. Base it on the net diff from the initial snapshot to the final state, not on intermediate fixes that were later changed again.

Use these headings in order, omitting sections that do not apply:

### Loop

Target skill, target arguments, requested iterations, completed iterations, and whether the loop stopped early.

### Net Changes

Final files and behaviors changed. Mention intermediate work only when it explains a final decision or residual risk.

### Code Polish

Which iterations ran `code-polish`, which skipped it, and why.

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Write `None.` when there are none.
