---
argument-hint: '<skill-invocation> [--runs <n>]'
disable-model-invocation: true
name: loop-skill
user-invocable: true
description: 'Run another agent skill until its result stabilizes (at most 3 iterations by default), or exactly N times with --runs, then report the net result once.'
---

# Loop Skill

Repeat a safe target workflow until its observable result stabilizes, while preserving one scope and one final net report.

## Arguments

- Target skill invocation: required. Accept a skill name, `$skill-name`, `SKILL.md` path, or unambiguous natural-language invocation with arguments.
- `--runs <n>` / `-n <n>`: optional positive integer. When present, run exactly `n` completed iterations unless the target hits a genuine stop condition. Without it, stop on stability and cap the loop at three iterations.

## Preconditions

- Run only in an execution-capable mode.
- Resolve and read the target skill once. Refuse a missing target, `loop-skill` itself, or a target whose required workflow creates commits/tags/branches/stashes, rewrites history, performs destructive actions, or makes external writes on each iteration.
- Preserve the target's authority, scope, and stop conditions. Looping does not authorize broader edits or repeated side effects.
- When repository state matters, require a Git worktree and snapshot the initial tracked diff, status, and untracked paths/content needed for a final net comparison.

## Workflow

1. Parse the target and mode:
   - explicit mode: exact positive `--runs n`;
   - convergence mode: maximum three iterations.
2. Resolve scope once from the target request. Override only target reporting and persistent-history behavior: do not commit, tag, branch, stash, or emit a full user report between iterations.
3. Before each iteration, snapshot the observable target state. For code workflows, include tracked diff plus untracked file identity/content; for non-file workflows, define an equivalent artifact or finding set.
4. Run the target inline with the same arguments and fixed scope. If it requires user input or hits a stop condition, end the loop and preserve completed work.
5. Compare the post-iteration state and target outcome with the pre-iteration snapshot:
   - In convergence mode, stop as `stable` when the iteration produces no material state change, no new finding/action, and no changed verification outcome.
   - Otherwise continue until stable or three iterations complete.
   - In explicit mode, stability does not shorten the requested count; continue through exactly `n` completed iterations.
6. Verify the final net state with the narrowest project-defined checks that cover the target's changes.

## Report

Produce one final report containing:

- target, arguments, mode, requested cap/count, completed iterations, and stop reason (`stable`, `cap reached`, `exact count completed`, or target blocker);
- net changes relative to the initial snapshot, excluding intermediate work that did not survive;
- exact verification commands and outcomes;
- residual risks or the blocking user decision.

Completion requires a net-state comparison and final verification. Reaching the default cap is not convergence; report it as `cap reached` unless a no-change iteration established stability.
