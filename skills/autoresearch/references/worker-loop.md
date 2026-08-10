# Autoresearch Worker Loop

Read this reference completely only when the parent assigns an implementation batch through codex-handoff. Execute the
approved hypotheses; do not choose the research direction.

## Brief Contract

Require the brief to provide the approved session contract, ordered hypotheses, exact write scope and commands, current
session and retained-best state or first-batch status, and batch stopping criteria. Return `blocked` when any of these
inputs is missing or contradictory rather than inventing a contract.

Execute the supplied hypotheses in order against the current retained best. Make only local implementation adjustments
needed to test an assigned mechanism. If a result invalidates a remaining hypothesis, stop the batch and report that
fact. Record promising new directions in `autoresearch.ideas.md` or the result, but do not execute them until the parent
assigns them.

## Isolation

For the first batch, prefer a dedicated branch in a separate Git worktree. Record its starting commit, path, initial
status, allowed paths, and session files in `autoresearch.md`. If isolation is unavailable, require a clean worktree or
explicit authorization to share it.

Never run repository-wide clean, checkout, stash, or reset commands. Revert only paths changed by the current experiment
from its recorded pre-run state, and remove only newly created in-scope paths. Preserve unrelated files and all session
evidence.

## Session Module

Create `autoresearch.md`, deterministic `autoresearch.sh`, optional `autoresearch.checks.sh`, append-only
`autoresearch.jsonl`, and optional `autoresearch.ideas.md`. Resolve the module from the owning `SKILL.md` and initialize
the JSONL before the baseline:

```sh
uv run "<skill-dir>/scripts/autoresearch-session.py" init \
  --file autoresearch.jsonl --metric <name> --direction <higher|lower> \
  --max-runs <n> --max-runtime-seconds <seconds> \
  --max-cost <amount> --convergence-runs <n>
```

The first config record declares `direction`. Record each completed attempt only after assigning its status:

```sh
uv run "<skill-dir>/scripts/autoresearch-session.py" record \
  --file autoresearch.jsonl --metric <number> \
  --status <keep|discard|crash|checks_failed> \
  [--commit <id>] [--description <text>] \
  [--elapsed-seconds <n>] [--estimated-cost <amount>]
```

Zero and negative metrics are valid. The worker owns mechanical `keep` versus `discard` judgment under the approved
contract; the module validates records and uses the declared direction. When the primary metric changes under a newly
approved contract, pass `--metric-name <new>` and `--direction <higher|lower>` on the first new record. The module
appends a new segment config.

Use `status --format json` for best/delta/MAD/confidence, counts, convergence, budgets, and exact progress rendering:

```sh
uv run "<skill-dir>/scripts/autoresearch-session.py" status --file autoresearch.jsonl
```

`scripts/confidence.sh [jsonl]` and `scripts/summary.sh [jsonl]` remain compatibility adapters. Malformed records or
violated invariants fail; noisy, equivalent, or worker-discarded results are reported facts, not helper failures.

## Batch Loop

1. Inspect all in-scope source plus relevant tests or profiles. For the first batch, create isolation and session files,
   then record an unchanged baseline.
2. Before each hypothesis, snapshot allowed paths, implement the assigned mechanism, and run the benchmark within its
   timeout.
3. Parse the declared metric. Missing metrics, crashes, timeouts, and failed correctness checks cannot be improvements.
4. Run correctness checks for every candidate that might be retained.
5. Use the session status plus repeated measurements to judge noise or equivalence. Keep only a verified improvement
   within all hard constraints; prefer simpler code when results are equivalent. Otherwise perform the scoped revert.
6. Append the assigned record and update `autoresearch.md` when evidence changes the retained best or rules out an
   approach.
7. Stop at the first batch criterion, hard session limit, user interruption, satisfied target, helper-reported
   convergence, or result that invalidates the remaining batch. Do not substitute an unassigned hypothesis.
8. Read `loop-rules.md` only for ambiguous keep/discard judgment, noise handling, backlog maintenance, or thrash
   recovery.

## Batch Result

Run the session module's JSON status after the last settled attempt. Preserve full commands, measurements, diagnostics,
and lessons in the session artifacts. Return codex-handoff's required result fields plus a compact autoresearch receipt:

- each attempted hypothesis in order with its `keep`, `discard`, `crash`, or `checks_failed` status and metric when
  available;
- baseline, retained best, delta, confidence, status counts, budget state, and convergence state;
- the exact batch stop reason and any remaining hypotheses invalidated or not attempted; and
- suggested next directions, without implementing them.

Do not return raw benchmark logs unless they are necessary evidence for a blocker or integrity failure.
