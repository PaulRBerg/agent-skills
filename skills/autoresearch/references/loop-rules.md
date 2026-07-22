# Loop Rules Reference

Read only when a keep/discard decision is ambiguous, a benchmark is noisy, or the search repeats itself.

## Agent Judgment

- The declared primary metric supplies direction; secondary metrics enforce explicit budgets and explain tradeoffs.
- Keep only candidates that pass correctness checks and hard constraints.
- Re-run gains within the measured noise floor. Module confidence is evidence, not a substitute for measurement.
- Prefer less code for equivalent results. Do not retain complexity for an unconfirmed marginal gain.
- Treat crashes, timeouts, missing metrics, and failed checks as failed experiments. Fix one trivial experiment mistake;
  otherwise record the outcome and revert it.

The session module calculates retained bests, confidence, budget state, and the no-improvement window from validated
records. Do not reproduce those calculations manually or override a reported segment/budget fact.

## Thrash and Noise

Treat three variants of the same mechanism or failure as thrashing: record the lesson and choose a structurally
different hypothesis. When ideas run out, inspect profiles, source, dependencies, or relevant papers before trying
random parameters.

Establish noise from repeated unchanged or best-known runs. Prefer medians for short noisy workloads and keep the
sampling method stable. Do not move goalposts after seeing a result; changing the primary metric requires a new module
segment and baseline.

## Ideas Backlog

Add an item to `autoresearch.ideas.md` when promising work requires profiling, a coupled refactor, or a prerequisite.
Remove tried or invalidated items on resume. The backlog never expands session limits.

## Safe Revert

Before each experiment, record exact tracked and untracked in-scope paths and their state. Restore only those tracked
paths and delete only newly created in-scope files. Never clean, checkout, or reset the repository broadly.

Preserve `autoresearch.md`, `autoresearch.sh`, `autoresearch.checks.sh`, `autoresearch.jsonl`, and
`autoresearch.ideas.md` through experiment reverts.
