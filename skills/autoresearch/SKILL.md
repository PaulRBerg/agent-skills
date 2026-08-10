---
argument-hint: <goal> [--max-runs N] [--max-runtime DURATION]
compatibility:
  Requires Plan mode to establish or materially change a session contract and a codex-handoff-compatible host for
  implementation.
name: autoresearch
skill-dependencies:
  - codex-handoff
description:
  Use for autoresearch or "optimize X overnight/in a loop"; plans bounded measurable experiment batches, then delegates
  execution through codex-handoff.
---

# Autoresearch

Use the parent for research decisions and Codex handoff workers for experiment execution. Measure consistently, retain
only verified improvements, and stop on explicit resource or convergence limits.

## Orchestration Contract

Start a new autoresearch session in Plan mode. Also require Plan mode before materially changing an approved session's
objective, primary metric or direction, benchmark or correctness commands, write scope, or hard resource, cost,
regression, or convergence limits. If Plan mode is required but inactive, ask the user to switch and stop. An unchanged
approved contract may resume and receive new hypothesis batches outside Plan mode.

Always invoke `$codex-handoff` for implementation. Never fall back to direct parent implementation or another handoff
mechanism. Follow its host selection, plan manifest, team sizing, validation ownership, reconciliation, failure, and
completion contracts.

The parent owns the session contract, evidence synthesis, hypothesis selection and ordering, and stop decisions. It may
delegate read-only repository investigation when useful, but research workers return evidence rather than hypotheses or
plans. Keep the parent's execution work to orchestration, integrity checks, compact result review, and selection of the
next batch.

Implementation workers execute parent-supplied ordered hypothesis batches. Default to one worker for a sequential search
and pack multiple related hypotheses into one brief up to codex-handoff's sizing limit; never map one worker to each
idea by default. Split only when hypotheses are genuinely independent, dependency waves require it, or one brief would
be oversized. A worker may make local adjustments needed to execute an assigned hypothesis, but it must report rather
than execute a materially different research direction.

## Plan and Session Contract

Resolve the objective, primary metric and direction, benchmark and correctness commands, allowed/off-limits paths,
run/runtime/command/cost/regression limits, convergence window, and reporting cadence before approval. Infer safe facts
from the request and repository; ask only when a missing choice changes the experiment.

Defaults: 20 runs, two hours wall time, 10 minutes per benchmark, five minutes per correctness check, no new paid API
spend, and convergence after five consecutive valid runs without a new retained best. Explicit `--max-runs` and
`--max-runtime` values are hard limits.

The Plan-mode response must include the resolved contract, an evidence-backed ordered initial hypothesis batch with its
completion or early-stop criteria, and codex-handoff's required plan section and manifest. Adding, removing, or
reordering hypotheses inside the approved contract is follow-on planning, not a material contract change.

## Delegated Execution

Resolve `references/worker-loop.md` relative to this `SKILL.md`. Every implementation brief must require the worker to
read it completely and must supply the approved contract, ordered batch, exact paths and commands, current session and
best-result state or first-batch status, and batch stopping criteria. Codex-handoff owns the remaining prompt and result
fields.

The first implementation worker creates the isolation and session artifacts and records the unchanged baseline. Each
worker leaves detailed measurements and logs in those artifacts and returns only the compact batch receipt required by
the worker reference. The parent reconciles that receipt with the session module's JSON status, reads raw benchmark
output only when a decision or integrity check requires it, then selects another batch or stops. Further batches under
the unchanged contract remain follow-on work within the approved outcome.

## Progress and Completion

Use codex-handoff's host-native progress surface. Send parent-authored updates only from settled evidence at the
baseline, completed batch, material best change, blocker, or final stop; do not relay per-run narration. Render the
session module's exact bar, counts, metrics, budgets, and convergence facts, and name the next parent-selected batch
without recording it as settled work.

Finish with `### 🏁 Autoresearch complete — <stop reason>`, baseline/best/delta/confidence, status counts, kept-file
tree, exact checks, worktree/branch, and remaining cleanup or integration. Keep `METRIC` lines, JSONL, commands, and
diagnostics undecorated. A resource limit is not convergence.
