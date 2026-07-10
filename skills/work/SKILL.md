---
argument-hint: <task>
disable-model-invocation: true
name: work
user-invocable: true
description: 'Orchestrate end-to-end implementation: execute directly or delegate independent tracks, integrate, and verify the requested outcome.'
---

# Work

Deliver the requested implementation with the smallest coordination structure that gets to a verified result.

## Contract

- Treat `$ARGUMENTS` and the conversation as the scope and success criteria. Ask only when a missing decision changes scope, safety, implementation, or verification.
- Preserve unrelated work and repository conventions. Do not add features, cleanup, abstractions, or compatibility layers outside the request.
- Use subagents only when the environment supports them and at least two bounded tracks can proceed independently with distinct ownership and acceptance criteria. File count alone does not justify delegation.
- Keep integration decisions with the coordinating agent. Verify the combined result, not merely each track in isolation.

## Workflow

1. Inspect the relevant code, tests, repository instructions, and task-runner recipes. State material assumptions and define the narrowest evidence that will prove completion.
2. Choose execution shape:
   - Work directly when the task is sequential, tightly coupled, small enough to keep coherent, or likely to create overlapping edits.
   - Delegate independent research, implementation, or test tracks when concurrency materially helps. Give each agent a disjoint scope, explicit deliverable, and acceptance criterion; do not delegate the final integration judgment.
3. Implement the minimum in-scope change. Incorporate agent results only after reviewing their evidence and reconciling contracts, naming, and shared state.
4. Run targeted formatting, tests, types, builds, or smoke checks that exercise the changed behavior. Broaden validation only for shared contracts or cross-package effects.
5. Use `code-polish` only when the result would materially benefit from a separate simplification or risk-profiled review, or when the user requested it. Do not make polishing a completion ritual.
6. Report the outcome, files or behavior changed, exact validation results, and residual risks. Stop when the requested state is implemented and checkably verified.

## Stop Conditions

Stop for a destructive or irreversible action, a material scope expansion, an unrequested public-contract break, or input only the user can provide. A failed delegated track is not itself a blocker: reclaim it or continue directly when safe.

## Completion

Complete only when the requested behavior is integrated, the chosen success criteria pass, and the final report states the outcome, validation evidence, and residual risks.
