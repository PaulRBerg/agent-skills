---
disable-model-invocation: false
name: diagnosing-bugs
user-invocable: true
description: Use when diagnosing bugs, debugging failures, chasing regressions, explaining broken/throwing/failing/slow behavior, or building a repro/root-cause loop before fixing code.
---

# Diagnosing Bugs

Loop-first debugging for hard bugs and performance regressions. Do not theorize until there is a tight signal that can catch the user's exact symptom.

## Available Scripts

- `scripts/hitl-loop.template.sh`: copy and edit only when an unattended repro is impossible and the user must perform manual steps.

## Workflow

### 1. Build the feedback loop

Produce one command that can go red on the bug and green after the fix. Run it at least once before moving on.

Prefer loops in this order:

1. Failing test at the seam that reaches the bug.
2. Curl or HTTP script against a local server.
3. CLI invocation with fixture input and asserted output.
4. Browser automation that checks DOM, console, or network state.
5. Replay of a captured request, event, trace, log, or payload.
6. Throwaway harness that calls the failing path with mocked boundaries.
7. Property, fuzz, stress, or repeat loop for intermittent failures.
8. Bisect or differential loop across commits, versions, datasets, or config.
9. Human-in-the-loop script from `scripts/hitl-loop.template.sh`.

Tighten the loop until it is:

- Red-capable: asserts the specific symptom, not merely "did not crash".
- Deterministic: same verdict each run, or a high enough reproduction rate to debug.
- Fast: seconds when practical.
- Agent-runnable: no human steps except through the HITL template.

If no loop is possible, stop. State what was tried and ask for a captured artifact, repro access, or permission to add temporary instrumentation.

### 2. Reproduce and minimize

Run the loop and confirm it fails for the same symptom the user reported. If it finds a nearby but different failure, fix the loop before continuing.

Minimize one variable at a time: inputs, callers, config, data, timing, mocks, environment, and steps. After every cut, rerun the loop. Stop minimizing when every remaining element is load-bearing.

### 3. Rank hypotheses

Write 3-5 ranked hypotheses before testing any of them. Each hypothesis must include its falsifying prediction:

```text
If <cause> is true, then <probe/change> will make <observable result> happen.
```

Briefly show the ranked list to the user when useful, especially when domain or deployment history could reorder it. If they are unavailable, proceed with the current ranking.

### 4. Probe one prediction at a time

Map every probe to a hypothesis prediction. Change one variable per run.

Prefer:

1. Debugger, REPL, inspector, or query plan.
2. Targeted instrumentation at boundaries that distinguish hypotheses.
3. Tagged debug logs only when better probes are unavailable.

Prefix temporary logs with a unique token such as `[DEBUG-a4f2]` and remove them before completion. For performance bugs, establish a baseline measurement first, then profile or bisect; logging is usually the wrong first tool.

### 5. Fix with a regression check

Before the fix, turn the minimized repro into a failing regression test if a correct seam exists. The seam is correct only if it exercises the real bug pattern as it happens at the call site.

If no correct seam exists, document that as an architecture risk and keep the original feedback loop as the verification command.

Then:

1. Watch the regression test fail, if present.
2. Apply the smallest fix that satisfies the confirmed hypothesis.
3. Watch the regression test pass, if present.
4. Rerun the original unminimized feedback loop.

### 6. Clean up and report

Before declaring done:

- Rerun the original feedback loop and confirm green.
- Rerun the regression test, or state why no correct seam exists.
- Remove all tagged debug instrumentation.
- Delete throwaway harnesses unless the user asked to keep them.
- Report the root cause, the fix, the verification commands, and what would have prevented the bug.
