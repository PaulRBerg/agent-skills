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

Resolve the objective, primary metric and direction, benchmark and correctness commands, allowed/off-limits paths,
run/runtime/command/cost/regression limits, convergence window, and reporting cadence before the baseline. Infer safe
facts from the request and repository; ask only when a missing choice changes the experiment.

Defaults: 20 runs, two hours wall time, 10 minutes per benchmark, five minutes per correctness check, no new paid API
spend, and convergence after five consecutive valid runs without a new retained best. Explicit `--max-runs` and
`--max-runtime` values are hard limits.

## Isolation

Prefer a dedicated branch in a separate Git worktree. Record its starting commit, path, initial status, allowed paths,
and session files in `autoresearch.md`. If isolation is unavailable, require a clean worktree or explicit authorization
to share it.

Never run repository-wide clean, checkout, stash, or reset commands. Revert only paths changed by the current experiment
from its recorded pre-run state, and remove only newly created in-scope paths. Preserve unrelated files and all session
evidence.

## Session Module

Create `autoresearch.md`, deterministic `autoresearch.sh`, optional `autoresearch.checks.sh`, append-only
`autoresearch.jsonl`, and optional `autoresearch.ideas.md`. Resolve the module from this `SKILL.md` and initialize the
JSONL before the baseline:

```sh
uv run "<skill-dir>/scripts/autoresearch-session.py" init \
  --file autoresearch.jsonl --metric <name> --direction <higher|lower> \
  --max-runs <n> --max-runtime-seconds <seconds> \
  --max-cost <amount> --convergence-runs <n>
```

The first config record declares `direction`. Record each completed attempt only after the agent assigns its status:

```sh
uv run "<skill-dir>/scripts/autoresearch-session.py" record \
  --file autoresearch.jsonl --metric <number> \
  --status <keep|discard|crash|checks_failed> \
  [--commit <id>] [--description <text>] \
  [--elapsed-seconds <n>] [--estimated-cost <amount>]
```

Zero and negative metrics are valid values. The agent owns `keep` versus `discard`; the module validates records and
uses the declared direction. When the primary metric changes, pass `--metric-name <new>` and
`--direction <higher|lower>` on the first new record. The module appends a new segment config.

Use `status --format json` for best/delta/MAD/confidence, counts, convergence, budgets, and exact progress rendering:

```sh
uv run "<skill-dir>/scripts/autoresearch-session.py" status --file autoresearch.jsonl
```

`scripts/confidence.sh [jsonl]` and `scripts/summary.sh [jsonl]` remain compatibility adapters. Malformed records or
violated invariants fail; noisy, equivalent, or agent-discarded results are reported facts, not helper failures.

## Experiment Loop

1. Inspect all in-scope source plus relevant tests or profiles. Create isolation and session files, then record an
   unchanged baseline.
2. Before each run, snapshot allowed paths. Choose one focused hypothesis, implement it, and run the benchmark within
   its timeout.
3. Parse the declared metric. Missing metrics, crashes, timeouts, and failed correctness checks cannot be improvements.
4. Run correctness checks for every candidate the agent might retain.
5. Use the session status plus repeated measurements to judge noise or equivalence. Keep only a verified improvement
   within all hard constraints; prefer simpler code when results are equivalent. Otherwise perform the scoped revert.
6. Append the agent-assigned record and update `autoresearch.md` when evidence changes the retained best or rules out an
   approach.
7. Incorporate user steering between completed runs. Stop at the first hard limit, user interruption, satisfied target,
   or helper-reported convergence.
8. Read `references/loop-rules.md` only for ambiguous keep/discard judgment, noise handling, backlog maintenance, or
   thrash recovery.

## Progress and Completion

Send sparse updates at the baseline, every five settled runs or material best change, and the final stop. Render the
module's exact bar, counts, metrics, budgets, and convergence facts; never infer progress from time or activity. Include
the next agent-chosen hypothesis without recording it as settled work.

Finish with `### 🏁 Autoresearch complete — <stop reason>`, baseline/best/delta/confidence, status counts, kept-file
tree, exact checks, worktree/branch, and remaining cleanup or integration. Keep `METRIC` lines, JSONL, commands, and
diagnostics undecorated. A resource limit is not convergence.
