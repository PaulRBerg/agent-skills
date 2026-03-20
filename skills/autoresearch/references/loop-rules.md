# Loop Rules Reference

## Core Principles

1. **Autonomy**: Never ask "should I continue?" or "is this a good stopping point?". The user expects you to run indefinitely until manually interrupted. They may be asleep, away, or working on something else.

2. **Primary metric is king**: The single primary metric (defined in `autoresearch.md`) determines keep/discard. Secondary metrics are logged for context but rarely affect the decision.

3. **Simplicity criterion**: All else being equal, simpler is better.
   - Removing code for equal or better performance → always keep.
   - Ugly complexity for a tiny gain → probably discard.
   - A 0.001 improvement that adds 20 lines of hacky code? Not worth it.
   - A 0.001 improvement from deleting code? Definitely keep.

## Keep vs Discard Decisions

| Situation | Decision | Rationale |
|-----------|----------|-----------|
| Metric improved | `keep` | Primary metric is king |
| Metric equal | `discard` | No improvement = no reason to keep complexity |
| Metric slightly worse but code much simpler | `discard` (usually) | Simplicity matters but metric still wins |
| Metric improved but confidence <1.0x | Re-run to confirm | May be noise, not real improvement |
| Crash (OOM, bug) | `crash` + revert | Log it and move on |
| Checks failed (tests, types) | `checks_failed` + revert | Correctness is a hard constraint |
| Metric improved but VRAM/memory exploded | Use judgment | Some increase is acceptable for meaningful gains |

## Confidence Score Interpretation

After 3+ experiments, the confidence score compares the best improvement to the session noise floor using Median Absolute Deviation (MAD).

| Score | Meaning | Action |
|-------|---------|--------|
| >= 2.0x | Improvement is likely real | Keep with confidence |
| 1.0-2.0x | Above noise but marginal | Keep, but note it's marginal |
| < 1.0x | Within noise | Re-run the same experiment to confirm before keeping |
| null | Insufficient data (<3 runs) | Keep/discard based on metric alone |

The score is advisory — it never auto-discards. Use it to decide whether to re-run for confirmation.

## Crash Handling

- **Trivial fix** (typo, missing import, wrong variable name): Fix and re-run. Count as the same experiment.
- **Fundamental issue** (OOM, incompatible architecture, dependency missing): Log as `crash`, revert, move on.
- **Repeated crashes on same idea**: The idea is probably broken. Log it in "What's Been Tried" and try something different.
- **Don't over-invest**: More than 2-3 attempts to fix the same crash? Give up and move on.

## When You're Stuck

Getting stuck is normal. The easy wins get found first; the remaining improvements require deeper insight.

1. **Re-read the source files**. You may have missed something on the first pass.
2. **Study the profiling data**. If the benchmark outputs timing breakdowns, analyze where time is actually spent.
3. **Reason about fundamentals**. What is the CPU/GPU/runtime actually doing? Where are the bottlenecks? What does the memory access pattern look like?
4. **Review the git log**. What combinations haven't been tried? What near-misses could be combined?
5. **Check `autoresearch.ideas.md`**. Are there complex ideas you deferred earlier?
6. **Try radical changes**. If incremental tweaks aren't working, consider architectural changes.
7. **Read related code**. Look at dependencies, similar implementations, papers referenced in comments.

Do NOT just try random variations. The best experiments come from understanding, not from brute force.

## Don't Thrash

If you've reverted the same category of change 3+ times, stop trying that approach. Signs of thrashing:

- Toggling the same parameter back and forth.
- Making the same architectural change with minor variations.
- Repeatedly hitting the same OOM/crash.

When thrashing, step back and try something structurally different.

## Resume Protocol

When resuming from a context reset or new conversation:

1. Read `autoresearch.md` — this has the full session context.
2. Read `autoresearch.jsonl` — reconstruct run count, best metric, what's been tried.
3. Read `git log --oneline -20` — see recent commits and their status.
4. Check `autoresearch.ideas.md` — prune stale entries, queue promising ones.
5. Do NOT re-run the baseline. Continue from the current best.
6. Update `autoresearch.md` "What's Been Tried" if the previous agent left it stale.

## Ideas Backlog Management

`autoresearch.ideas.md` is an append-only scratchpad for promising but complex ideas.

- **When to add**: You discover an optimization that needs more than a simple code edit (multi-step refactor, needs profiling data first, requires understanding a dependency).
- **When to prune**: On resume, delete entries that have already been tried or are no longer relevant.
- **When to delete the file**: All ideas exhausted. Write a final summary to `autoresearch.md` instead.

## Session File Protection

These files must NEVER be reverted during a discard/crash:

- `autoresearch.jsonl` — append-only experiment log
- `autoresearch.md` — session document
- `autoresearch.sh` — benchmark script
- `autoresearch.checks.sh` — correctness checks
- `autoresearch.ideas.md` — ideas backlog

Always stage these files before running `git checkout -- .` to revert.

## Experiment Pacing

- **One focused change per experiment**. Don't combine unrelated changes — if the result improves, you won't know which change helped.
- **Exception**: If two changes are tightly coupled (e.g. changing model width requires adjusting learning rate), combine them.
- **Batch reverts are OK**: If 3 consecutive experiments all discard, that's fine. Each was a valid hypothesis that didn't pan out.
