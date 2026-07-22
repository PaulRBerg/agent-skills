---
argument-hint: <goal> [--max-runs N] [--max-runtime DURATION]
disable-model-invocation: false
name: autoresearch
user-invocable: true
description:
  Use for autoresearch or "optimize X overnight/in a loop"; sets up bounded iterative trials for a measurable
  optimization target.
---

# Autoresearch

Run isolated experiments, measure them consistently, keep verified improvements, and stop on explicit resource or
convergence limits.

## Session Contract

Resolve these before the baseline. Infer them from the request and repository when safe; ask only for a missing choice
that changes the experiment:

- objective and primary metric, including direction;
- benchmark command and correctness checks;
- files allowed to change and paths that are off limits;
- maximum runs, wall-clock runtime, per-command timeout, paid-service cost, and acceptable regression budgets;
- convergence rule and any user-requested reporting cadence.

Defaults when the user gives none: 20 runs, two hours wall time, 10 minutes per benchmark, five minutes per correctness
check, no new paid API spend, and convergence after five consecutive valid runs without a new best result. An explicit
`--max-runs N` is exact unless another hard limit is reached first; `--max-runtime DURATION` (for example `90m` or `2h`)
overrides the wall-clock limit the same way.

## Isolation

Prefer a dedicated branch in a separate Git worktree so experiments cannot overwrite unrelated files. Record the
starting commit, worktree path, initial status, allowed paths, and session-file paths in `autoresearch.md`. If isolation
is unavailable, require a clean worktree or explicit authorization to share it.

Never use broad cleanup commands such as `git clean -fd`, `git checkout -- .`, or a hard reset. Revert only the paths
changed by the current experiment, using the recorded pre-experiment state; remove only newly created in-scope files
identified by that snapshot. Preserve unrelated tracked and untracked files.

## Session Files

Create these inside the experiment worktree:

- `autoresearch.md`: objective, metrics, limits, commands, scope, off-limits paths, baseline, best result, and concise
  tried/learned notes.
- `autoresearch.sh`: deterministic benchmark that emits `METRIC name=value` lines.
- `autoresearch.checks.sh`: correctness checks, only when correctness constraints require it.
- `autoresearch.jsonl`: append-only run evidence. Each run record needs a numeric `metric` (primary metric value; `0`
  for crashes), a `status` of `keep`, `discard`, `crash`, or `checks_failed`, and an integer `segment` that increments
  when the primary metric changes; lines without `status` are treated as config and skipped.
- `autoresearch.ideas.md`: optional backlog for deferred hypotheses.

Use `set -euo pipefail` in shell helpers. For noisy fast benchmarks, report a median from repeated samples. Keep
correctness-check time outside the primary metric. The bundled helpers `scripts/confidence.sh [jsonl-path]` (MAD-based
confidence for the current segment) and `scripts/summary.sh [jsonl-path]` (session dashboard) read these records.

## Workflow

1. Inspect every in-scope source and the relevant tests or profiling data. Create the isolated worktree/branch and
   session files, then record a no-change baseline.
2. For each run, snapshot the allowed paths, choose one focused hypothesis, implement it, and execute the benchmark
   within the per-command timeout.
3. Parse the declared primary metric. A missing metric, crash, timeout, or failed correctness check is not an
   improvement.
4. Run correctness checks for every benchmark candidate that would otherwise be kept.
5. Compare against the best valid result:
   - Keep a result only when the primary metric improves and every hard constraint passes. Re-run marginal/noisy wins
     before accepting them.
   - Prefer simpler code when results are equivalent; otherwise revert the current experiment's paths only.
6. Append one JSONL record with run number, commit or snapshot ID, `metric`, `status`, `segment`, elapsed time,
   estimated paid cost, description, and confidence. Update `autoresearch.md` when a result changes the best value or
   rules out an approach.
7. Between completed run cycles, incorporate user steering immediately. Do not wait for the entire session when the user
   changes scope, limits, or priorities.
8. Stop at the first hard limit, user interruption, satisfied target, or convergence condition. Read
   [references/loop-rules.md](references/loop-rules.md) only for ambiguous keep/discard calls, noise handling, backlog
   maintenance, or thrash recovery.

## Progress and Completion

For long runs, send sparse updates at the baseline, every five completed runs or major best-result change, and final
stop. Ground every claim in the current session's logs: current/best metric, runs completed, elapsed time, cost used,
and next hypothesis. Render each update as `### 🧪 Autoresearch [<10 cells>] <completed>/<max> runs`, followed by a
compact metric/budget table and one `Next hypothesis: <text>` line. Filled `█` and empty `░` cells represent completed
runs only: use `floor(10 * completed / max + 0.5)` filled cells and fill the remainder with empty cells. Omit the bar
when the run limit is unknown. Never infer progress from elapsed time or activity.

Finish with `### 🏁 Autoresearch complete — <stop reason>`, a baseline/best/delta/confidence table; counts for kept,
discarded, crashed, and checks-failed runs; a tree of kept files; exact checks; worktree/branch location; and any
cleanup or integration action the user still owns. Keep `METRIC` lines, JSONL fields/status values, commands, and
diagnostics undecorated. Do not claim convergence when the session merely hit a resource limit.
