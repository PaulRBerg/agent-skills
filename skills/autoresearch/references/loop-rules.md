# Loop Rules Reference

Read this reference only when a decision is ambiguous, a benchmark is noisy, or the search is repeating itself.

## Keep or Revert

- The declared primary metric decides improvement; secondary metrics enforce explicit budgets and explain tradeoffs.
- Keep only results that pass correctness checks and hard constraints.
- Re-run a gain that is within the measured noise floor. The confidence helper is advisory, not a substitute for
  repeated measurement.
- Prefer less code for statistically equivalent results. Do not retain complexity for an unconfirmed marginal gain.
- Treat crashes, timeouts, missing metrics, and failed checks as failed experiments. Fix a trivial experiment mistake
  once; otherwise log and revert it.

## Convergence and Thrash

Count only valid completed runs toward the no-improvement convergence window. Stop when the configured window closes
without a new best result.

An approach is thrashing when three variants repeat the same mechanism or failure. Record the lesson, stop varying that
mechanism, and choose a structurally different hypothesis. When ideas run out, inspect profiles, source, dependencies,
or relevant papers before trying random parameters.

## Noise

- Establish noise from repeated unchanged or best-known runs when the benchmark is variable.
- Prefer medians for short noisy workloads and keep the sampling method stable across candidates.
- Treat changes below the noise floor as unconfirmed. Re-run within the remaining run, time, and cost budgets.
- Do not move goalposts after seeing a result; changing the primary metric starts a new baseline segment.

## Ideas Backlog

Add an item to `autoresearch.ideas.md` when it is promising but requires profiling, a coupled refactor, or a
prerequisite. On resume, remove items already tried or invalidated. The backlog is not a reason to exceed session
limits.

## Safe Revert

Before each experiment, record the exact tracked and untracked in-scope paths and their state. On failure, restore those
tracked paths from the pre-experiment commit or snapshot and delete only newly created in-scope files from that run.
Never run a repository-wide clean, checkout, or reset.

Session files are evidence and must survive experiment reverts: `autoresearch.md`, `autoresearch.sh`,
`autoresearch.checks.sh`, `autoresearch.jsonl`, and `autoresearch.ideas.md`.
