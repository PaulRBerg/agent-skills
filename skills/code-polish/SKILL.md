---
argument-hint: '[paths] [--deep] [--with-profile <name>] [--skip-profile <name>]'
disable-model-invocation: true
name: code-polish
user-invocable: true
description: 'Use for combined simplification and review: polish code, clean up and review/fix, simplify then review, refactor and review, or full code polish on recent changes.'
---

# Code Polish

## Objective

Run a fast combined pipeline on recently changed code: focused simplification for readability and maintainability, then risk-profiled review with fixes applied. Resolve scope once, avoid duplicate broad checks, and produce one user-facing report.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--deep`: Run exhaustive simplify and review passes. Read sibling `code-simplify` and `code-review` skill files before those phases.
- `--with-profile <name>`: Forward unchanged to the review phase. Repeatable.
- `--skip-profile <name>`: Forward unchanged to the review phase. Repeatable.
- Extra cleanup instructions (e.g. "and split `_lib.ts` into smaller files"): execute during the simplify phase.
- Default: run the faster common path on the resolved scope.

## Phase Contracts

Use these embedded contracts for the default path. Read sibling `SKILL.md` files only when `--deep` is set, the user's instructions create ambiguity these contracts do not resolve, or a phase hits a stop condition that requires the full sibling guidance. Sibling paths are `../code-simplify/SKILL.md` and `../code-review/SKILL.md`, relative to this file.

### Focused Simplify Contract

- Use the fixed `resolved-scope` block. Do not broaden, rediscover, or re-emit scope.
- Apply only high-confidence simplifications that materially improve comprehension, reduce defect risk, or remove cleanup created by the current change.
- Preserve behavior, public contracts, side effects, logging, telemetry, retries, and error semantics.
- Do not run naming-only refactors unless `--deep` is set or the user explicitly asked for naming or intent cleanup.
- Do not hand-edit generated, vendored, bulk, or low-signal files. If they must change, edit the generator, schema, or contract and validate the output with invariant checks.
- Skip phase-level verification and the phase-level report; keep terse internal notes for the final report.

### Risk-Profiled Review Contract

- Use the fixed `resolved-scope` block and any `excluded-scope` block. Do not broaden or rediscover scope.
- Build findings internally, apply fixes in severity order, then produce one final report. Do not stop for a separate pre-fix report.
- Apply core correctness, security, data integrity, shell/config safety, regression, and targeted verification checks on every run.
- Select only review profiles triggered by touched risk surfaces, capped at three auto-selected profiles by default. Include `--with-profile` profiles unless excluded by `--skip-profile`.
- Run the naming profile only with `--deep`, `--with-profile naming`, or explicit naming/intent review instructions, unless skipped.
- For generated, vendored, bulk, or low-signal files, review generators, schemas, contracts, and invariants instead of hand-reviewing every generated row or file.

## Scope Resolution

Resolve scope once, then treat the result as fixed for the rest of the run.

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git repository.
2. If the request names targets — file paths/patterns, a commit/range, a natural-language subset (e.g. "the parser changes"), or a `resolved-scope` fenced block with one repo-relative path per line — scope is exactly those targets. Map natural-language subsets to concrete paths before continuing.
3. Otherwise, scope is **only** session-modified files: files created or edited earlier in this session. Do not include other uncommitted changes.
4. If there are no session-modified files, or earlier conversation history is not visible in this context, fall back to all uncommitted files, running each command once:
   - tracked: `git diff --name-only --diff-filter=ACMR`
   - untracked: `git ls-files --others --exclude-standard`
   - combine both lists and de-duplicate.
5. Exclude generated, vendored, bulk, and low-signal files from manual simplify/review unless explicitly requested: lockfiles, minified bundles, build outputs, generated outputs, vendored code, and large data snapshots. When excluded files are relevant to correctness, emit an optional fenced code block tagged `excluded-scope`, one repo-relative path or glob per line, and cover them through verification or invariant checks.
6. If scope resolves to zero files, report that and stop.
7. Emit the scope as a fenced code block tagged `resolved-scope`, one repo-relative path per line. The block is authoritative: do not re-run scope commands or revisit exclusions afterward.

## Workflow

### 1) Resolve Scope

- Apply the Scope Resolution section and emit the `resolved-scope` block.
- Emit `excluded-scope` only when generated, vendored, bulk, or low-signal files are intentionally excluded but still relevant to verification.
- Forward user intent, constraints, and risk preferences to both phases; the scope blocks replace raw scope selectors.

### 2) Simplify Phase

Run the Focused Simplify Contract with:

- the `resolved-scope` block
- any extra cleanup instructions from the user
- `--deep` semantics only when requested

Under `--deep`, read `../code-simplify/SKILL.md` and follow it inline with `--no-verify`, `--no-report`, the `resolved-scope` block, and any extra cleanup instructions. Flags are instructions to interpret, not commands to execute.

### 3) Review Phase

Run the Risk-Profiled Review Contract with:

- the same `resolved-scope` block
- any `excluded-scope` block
- `--fix`
- `--deep` when requested
- any `--with-profile` and `--skip-profile` flags from the user

Under `--deep`, read `../code-review/SKILL.md` and follow it inline with `--fix`, `--deep`, the scope blocks, and any forwarded profile flags. Flags are instructions to interpret, not commands to execute.

### 4) Final Verification

- Reuse the review phase's post-fix verification as final when it covers the final touched scope.
- Otherwise run the narrowest checks that validate touched behavior:
  - formatter/lint on touched files
  - targeted tests for touched modules
  - typecheck when relevant
  - invariant checks for any `excluded-scope` outputs affected by the change
- Name every skipped check and why.

### 5) Report

Produce the Report section below. Both phases must have completed first (see Stop Conditions).

## Report

Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Files and functions touched, final state, and any `excluded-scope` entries with the validation strategy used for them.

### Simplifications

Key changes from the simplify phase, derived from the diff when needed.

### Review Findings and Fixes

Findings and applied fixes from the review phase, ordered `CRITICAL -> HIGH -> MEDIUM -> LOW`.

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Plain language — expand or gloss domain-specific terms. Include questions that need a user decision, phrased directly. Write `None.` when there are none.

## Stop Conditions

Stop and ask for direction when:

- a required sibling skill is missing during `--deep` or an ambiguity path that requires sibling guidance.
- a phase hits one of its own stop conditions.
- the review phase cannot run or cannot complete its fixes.

Completion gate: a polish run is complete only after both phases have run over the resolved scope and the Report above is produced. Never end the run after the simplify phase alone; if the review phase did not run, state explicitly that the polish is incomplete and which phase is missing.
