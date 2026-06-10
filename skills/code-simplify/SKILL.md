---
argument-hint: '[paths] [--no-report] [--no-verify]'
disable-model-invocation: true
name: code-simplify
user-invocable: true
description: This skill should be used when the user asks to "simplify code", "clean up code", "refactor for clarity", "reduce complexity", "improve readability", "make this easier to maintain", or asks to simplify recently modified code.
---

# Code Simplify

## Objective

Simplify code while preserving behavior, public contracts, and side effects. Favor explicit code and local clarity over clever or compressed constructs.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--no-report`: Skip the full user-facing report and return terse working notes for the caller.
- `--no-verify`: Skip verification because a parent orchestrator will verify the final result separately.
- Default: verify touched behavior and present the full report.

## Scope Resolution

Resolve scope once, then treat the result as fixed for the rest of the run.

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git repository.
2. If the request names targets — file paths/patterns, a commit/range, a natural-language subset (e.g. "the parser changes"), or a `resolved-scope` fenced block with one repo-relative path per line — scope is exactly those targets. Map natural-language subsets to concrete paths before continuing.
3. Otherwise, scope is **only** session-modified files: files created or edited earlier in this session. Do not include other uncommitted changes.
4. If there are no session-modified files, or earlier conversation history is not visible in this context, fall back to all uncommitted files, running each command once:
   - tracked: `git diff --name-only --diff-filter=ACMR`
   - untracked: `git ls-files --others --exclude-standard`
   - combine both lists and de-duplicate.
5. Exclude generated/low-signal files unless explicitly requested: lockfiles, minified bundles, build outputs, vendored code.
6. If scope resolves to zero files, report that and stop.
7. Emit the scope as a fenced code block tagged `resolved-scope`, one repo-relative path per line. The block is authoritative: do not re-run scope commands or revisit exclusions afterward.

## Operating Rules

- Preserve runtime behavior exactly. Keep inputs, outputs, side effects, and error behavior stable.
- Prefer project conventions over personal preferences. Infer conventions from existing code, linters, formatters, and tests.
- Make small, reversible edits. Avoid broad rewrites when targeted simplifications solve the problem.
- Call out uncertainty immediately when behavior may change.

## Workflow

### 1) Determine Scope

- Apply the Scope Resolution section.

### 2) Build a Behavior Baseline

- Read surrounding context, not only changed lines.
- Identify invariants that must not change:
  - function signatures and exported APIs
  - state transitions and side effects
  - persistence/network behavior
  - user-facing messages and error semantics where externally relied on
- Note available verification commands (lint, tests, typecheck).

### 3) Apply Simplification Passes (in this order)

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

### 4) Enforce Safety Constraints

- Do not convert sync APIs to async (or reverse) unless explicitly requested.
- Do not alter error propagation strategy unless behavior remains equivalent and verified.
- Do not remove logging, telemetry, guards, or retries that encode operational intent.
- Do not collapse domain-specific steps into generic helpers that hide intent.

### 5) Verify

Skip when `--no-verify` is set. Otherwise verify per the Verification section below.

### 6) Report

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

## Verification

Run the narrowest checks that validate touched behavior:

- formatter/lint on touched files
- targeted tests for touched modules
- typecheck when relevant

Run broader checks only when risk warrants it. Name every skipped check and why.

## Report

Skip when `--no-report` is set; return terse working notes instead: touched scope, key simplifications, residual risks.

Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Files and regions changed.

### Simplifications

One sentence per meaningful change, focused on the readability or maintainability gain. Confirm behavior-preservation assumptions explicitly.

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Plain language — expand or gloss domain-specific terms. Include questions that need a user decision, phrased directly. Write `None.` when there are none.

## Stop Conditions

Stop and ask for direction when:

- simplification requires changing public API/contracts.
- behavior parity cannot be confidently verified.
- the code appears intentionally complex due to domain constraints.
- the requested scope implies a larger redesign rather than simplification.
