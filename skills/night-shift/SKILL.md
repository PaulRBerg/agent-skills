---
argument-hint: "[path] [--max-runtime DURATION]"
compatibility: Requires Git and local command and edit access. Goal mode and subagents are optional.
disable-model-invocation: true
name: night-shift
user-invocable: true
description: Autonomous overnight codebase improvement with bounded runtime, evidence-gated changes, and verification.
---

# Night Shift

Improve a codebase autonomously within a fixed deadline, keeping only safe changes supported by concrete evidence.

A valid run ends with verified improvements or a verified no-op. Never invoke an ask-user mechanism or end a user-facing
message with a question. Convert every unresolved decision into a deferred-work entry with its evidence, risk, and
required choice while continuing any independent safe work.

## Arguments

- `path`: Optional absolute or relative path. Default to the current Git worktree root. Resolve it canonically and
  accept it only when it is inside that worktree.
- `--max-runtime DURATION`: Optional positive integer followed by `m` or `h`, such as `45m` or `3h`. Default to `2h`.

Treat an invalid path, duration, unknown option, or ambiguous positional input as a blocker. Report it and stop.

Calculate the deadline before auditing. Reserve the final 15% for aggregate validation and reporting, clamped to 5–30
minutes and never exceeding the total runtime.

## Session Contract

When the client supports persistent goals, create one containing the resolved outcome, deadline, safety limits, and
verification bar. Keep working inline when goal mode is unavailable.

Work in the current worktree and preserve changes that this run does not own. Local, reversible edits and
non-destructive validation are authorized. Do not change public contracts, commit, push, sync, deploy, release, write to
external systems, incur paid-service costs, change credentials or permissions, perform destructive migrations, or expand
scope.

Stop when the deadline arrives, no safe candidates remain, or every remaining candidate requires a prohibited or unclear
decision. Enter final validation when the reserved window begins even if a change wave is unfinished.

## Ground the Run

1. Read the applicable repository instructions. Record the worktree root, starting commit, starting status, resolved
   scope, deadline, and validation window. A dirty worktree is not a blocker; distinguish and preserve pre-existing
   changes.
2. Discover repository-defined build, test, lint, typecheck, format, and task-runner commands. Prefer the repository's
   documented commands and inspect wrappers before running commands with unclear side effects.
3. Establish baseline results for every safe, relevant check. If the baseline is red, prioritize reproducible failures
   before discretionary improvements. When a failure needs an unclear or prohibited action, defer it and continue only
   with independently verifiable work.

Completion of this phase requires a recorded baseline and an ownership boundary that prevents this run from overwriting
unrelated edits.

## Audit and Rank

Audit the resolved scope using these lenses:

- correctness, security, reliability, error handling, and concurrency;
- duplication, inconsistent patterns, deep-module and interface quality, and testability;
- performance, dependencies, data formats and file extensions, configuration, and observability;
- tests, accessibility, documentation, agent context, naming, and directory structure.

Treat source files over 1000 lines and test files over 2000 lines as discovery candidates only. Split a file only when
cohesion, coupling, change risk, or testability establishes a better seam; line count alone is not evidence.

Rank candidates by impact, evidence, regression risk, isolation, and verification cost:

- Apply reversible, behavior-preserving local improvements backed by concrete evidence.
- Apply dependency or framework updates, data-format or extension changes, renames, and reorganizations only when all
  affected consumers are controlled, the migration is atomic, compatibility is demonstrable, and repository checks can
  prove the result.
- Defer every candidate prohibited by the Session Contract and any work whose intended behavior is unclear.

Do not retain a performance change without a recorded baseline metric and repeatable benchmark.

## Coordinate Work

Use the smallest effective team when subagents are available, with at most five concurrent children. Otherwise execute
the same workflow inline.

- Parallelize independent read-only audit lenses.
- Give writing agents stable IDs, dependency waves, exact non-overlapping write scopes, repository constraints, and
  required completion evidence.
- Assign shared manifests, lockfiles, exports, and integration files to one sequential owner.
- Tell every worker that it shares the worktree, must preserve others' edits, and must not broaden scope.
- Reconcile each wave before starting dependents. Use a fresh-context verifier after every nontrivial wave.

## Execute and Verify

Implement focused candidates in dependency waves. Require targeted correctness checks for every kept change and broaden
validation only when a shared contract requires it.

If an experiment fails its evidence bar, revert only that experiment's attributable edits. Never use repository-wide
clean, checkout, or reset commands.

After each nontrivial wave, run `$code-polish` over the exact union of files changed by that wave when it is available.
Otherwise apply the same fixed-scope contract inline: simplify only where comprehension or defect risk measurably
improves, review by severity, fix evidenced defects, and rerun the narrowest proving checks.

During the reserved window, stop new implementation, reconcile all owned edits, and run the aggregate format, lint,
type, test, build, and invariant checks justified by the final changed-file union. Compare final results with the
recorded baseline. Ground every kept change and progress claim in tool evidence from this run.

For long runs, send sparse phase updates only after evidence-backed transitions: `🌙 Grounded`, `🔎 Audit complete`,
`🛠 Wave <N> verified`, and `⏳ Entering final validation`. Do not use a percentage bar because candidate discovery has
no stable denominator.

## Final Report

Lead with `### 🌙 Night Shift complete — <elapsed> · <stop reason>`. Compare checks in a compact
`Check | Baseline | Final` table, then report:

- elapsed time and stop reason;
- baseline and final checks, including failures and skipped checks;
- changed files and verified fixes, with before/after metrics where applicable;
- subagent assignments and results;
- reverted experiments and why their evidence failed;
- deferred decisions, each with evidence, risk, and the decision required;
- residual risks and the next proving check;
- that this run made no commit, push, or sync.

Group retained work under `✅ Kept`, failed experiments under `↩ Reverted`, and unresolved work under `⚠️ Deferred`,
always with text and counts. Keep commands, diagnostics, paths, and the no-commit/push/sync statement undecorated. A
verified no-op is complete. Write the report as a self-contained re-grounding for a user who did not observe the run.
