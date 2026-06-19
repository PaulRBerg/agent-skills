---
argument-hint: '[paths] [--deep] [--no-report] [--no-verify]'
disable-model-invocation: true
name: code-simplify
user-invocable: true
description: 'Use for simplifying recently changed code: clean up, refactor for clarity, reduce complexity, improve readability, or maintainability.'
---

# Code Simplify

## Objective

Simplify code while preserving behavior, public contracts, and side effects. Default to a triage-first pass: apply only high-confidence simplifications that materially improve comprehension or reduce defect risk. Use `--deep` for exhaustive cleanup.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--deep`: Run the full simplification checklist, including naming/intent cleanup when safe.
- `--no-report`: Skip the full user-facing report and return terse working notes for the caller.
- `--no-verify`: Skip verification because a parent orchestrator will verify the final result separately.
- Default: perform the faster triage-first pass, verify touched behavior, and present the full report.

## Scope Resolution

Resolve scope once, then treat the result as fixed for the rest of the run.

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git repository.
2. If the request names targets — file paths/patterns, a commit/range, a natural-language subset (e.g. "the parser changes"), or a `resolved-scope` fenced block with one repo-relative path per line — scope is exactly those targets. Map natural-language subsets to concrete paths before continuing.
3. Otherwise, scope is **only** session-modified files: files created or edited earlier in this session. Do not include other uncommitted changes.
4. If there are no session-modified files, or earlier conversation history is not visible in this context, fall back to all uncommitted files, running each command once:
   - tracked: `git diff --name-only --diff-filter=ACMR`
   - untracked: `git ls-files --others --exclude-standard`
   - combine both lists and de-duplicate.
5. Exclude generated, vendored, bulk, and low-signal files from manual simplification unless explicitly requested: lockfiles, minified bundles, build outputs, generated outputs, vendored code, and large data snapshots. When excluded files are relevant to correctness, emit an optional fenced code block tagged `excluded-scope`, one repo-relative path or glob per line, and cover them through verification or invariant checks.
6. If scope resolves to zero files, report that and stop.
7. Emit the scope as a fenced code block tagged `resolved-scope`, one repo-relative path per line. The block is authoritative: do not re-run scope commands or revisit exclusions afterward.

## Operating Rules

- State assumptions before editing. If multiple interpretations would change the simplification or verification strategy, present them and stop for direction.
- Preserve runtime behavior exactly. Keep inputs, outputs, side effects, and error behavior stable.
- Prefer project conventions over personal preferences. Infer conventions from existing code, linters, formatters, and tests.
- Make small, reversible edits. Every changed line should trace to the user's request, requested cleanup, or cleanup caused by your own edits.
- Write the minimum code that solves the requested problem. Do not add features, single-use abstractions, speculative flexibility, or configurability the user did not request.
- Clean up only your own mess: remove imports, variables, functions, and files made unused by your changes; mention pre-existing dead code in Residual Risks instead of deleting it.
- Do not run naming-only refactors unless `--deep` is set or the user explicitly asked for naming or intent cleanup.
- For generated, vendored, bulk, or low-signal files, simplify the generator, schema, or contract when possible and validate outputs with invariant checks instead of hand-editing every generated row or file.
- Call out uncertainty immediately when behavior may change.

## Workflow

### 1) Determine Scope

- Apply the Scope Resolution section.
- Treat any `excluded-scope` block as outside manual simplification but inside verification planning.

### 2) Build a Behavior Baseline

- Read surrounding context, not only changed lines.
- Identify invariants that must not change:
  - function signatures and exported APIs
  - state transitions and side effects
  - persistence/network behavior
  - user-facing messages and error semantics where externally relied on
- Note available verification commands (lint, tests, typecheck).
- Define success criteria before editing. For multi-step work, state a brief plan where each step names its verification check.

### 3) Triage Simplification Opportunities

- Default to no edit unless the simplification is high-confidence and materially improves comprehension, removes current-change cleanup, or reduces defect risk.
- Prefer local, behavior-preserving edits over broad rewrites.
- Skip no-op passes. If the scoped code is already clear enough, report that rather than churning it.
- Do not rename identifiers or split helpers unless `--deep` is set or the user explicitly requested naming/intent cleanup; even then, require a concrete clarity or safety gain. Never reshape APIs solely for taste.

### 4) Apply Simplification Passes

Use the default pass only for opportunities identified in triage. Under `--deep`, apply the full checklist in this order:

1. Control flow:
   - Flatten deep nesting with guard clauses and early returns.
   - Replace nested ternaries with clearer conditionals.
2. Naming and intent:
   - Rename ambiguous identifiers when local context supports safe renaming.
   - Separate mixed concerns into small helpers with intent-revealing names.
3. Duplication:
   - Remove obvious duplication.
   - Abstract only when at least two real call sites benefit and the abstraction reduces cognitive load.
4. Data shaping:
   - Break dense transform chains into named intermediate steps when readability improves.
   - Keep hot-path performance characteristics stable unless improvement is explicit and measured.
5. Type and contract clarity:
   - Add or tighten type annotations when they improve readability and safety without forcing broad churn.
   - Preserve external interfaces unless asked to change them.

### 5) Enforce Safety Constraints

- Do not convert sync APIs to async (or reverse) unless explicitly requested.
- Do not alter error propagation strategy unless behavior remains equivalent and verified.
- Do not remove logging, telemetry, guards, or retries that encode operational intent.
- Do not collapse domain-specific steps into generic helpers that hide intent.

### 6) Verify

Skip when `--no-verify` is set. Otherwise verify per the Verification section below.

### 7) Report

Produce the Report section below.

## Simplification Heuristics

- Prefer explicit local variables over nested inline expressions when it reduces cognitive load.
- Prefer one clear branch per condition over compact but ambiguous condition trees.
- Keep function length manageable, but do not split purely for line count.
- Keep comments that explain intent, invariants, or non-obvious constraints.
- Remove comments that restate obvious code behavior.
- Optimize for the next maintainer's comprehension time, not minimum character count.

## Anti-Patterns

- Do not perform speculative architecture rewrites.
- Do not introduce framework-wide patterns while simplifying a small local change.
- Do not replace understandable duplication with opaque utility layers.
- Do not bundle unrelated cleanups into one patch.
- Do not add error handling for impossible scenarios.
- Do not preserve code volume for its own sake; if a simpler equivalent approach exists, use it or explain why it does not satisfy the request.

## Verification

Run the narrowest checks that validate touched behavior:

- formatter/lint on touched files
- targeted tests for touched modules
- typecheck when relevant
- invariant checks for any relevant `excluded-scope` outputs

Run broader checks only when risk warrants it, especially under `--deep` or when simplification touches shared contracts. Name every skipped check and why.

## Report

Skip when `--no-report` is set; return terse working notes instead: touched scope, key simplifications, residual risks.

Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Files and regions changed, plus any `excluded-scope` entries with the validation strategy used for them.

### Simplifications

One sentence per meaningful change, focused on the readability or maintainability gain. Confirm behavior-preservation assumptions explicitly.

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Plain language — expand or gloss domain-specific terms. Include questions that need a user decision, phrased directly. Write `None.` when there are none.

## Stop Conditions

Stop and ask for direction when:

- simplification requires changing public API/contracts.
- requirements are unclear or competing interpretations would produce materially different edits.
- behavior parity cannot be confidently verified.
- the code appears intentionally complex due to domain constraints.
- the requested scope implies a larger redesign rather than simplification.
