---
argument-hint: '[paths] [--skip-profile <name>]'
disable-model-invocation: true
name: code-polish
user-invocable: true
description: This skill should be used when the user asks to "polish code", "simplify and review", "clean up and review code", "full code polish", "simplify then review", "refactor and review", "simplify and fix", "clean up and fix", or wants a combined simplification and review workflow on recently changed code.
---

# Code Polish

## Objective

Run a combined pipeline on recently changed code: `code-simplify` for readability and maintainability, then `code-review --fix` for correctness, security, and quality with fixes applied. One scope resolution, one user-facing report, no redundant simplify-phase verification.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--skip-profile <name>`: Forward unchanged to `code-review`. Repeatable.
- Extra cleanup instructions (e.g. "and split `_lib.ts` into smaller files"): execute during the simplify phase.
- Default: run the full pipeline on the resolved scope.

## Running Sub-Skills

This skill requires `code-simplify` and `code-review` installed as sibling skills.

- Read the sibling skill file once — `../code-simplify/SKILL.md` or `../code-review/SKILL.md`, relative to this file — and follow its instructions inline as if it were invoked with the stated arguments. Flags such as `--fix` are instructions to interpret, not commands to execute.
- If a sibling `SKILL.md` is missing, stop and report which one.

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

## Workflow

### 1) Resolve Scope

- Apply the Scope Resolution section and emit the `resolved-scope` block.
- Forward user intent, constraints, and risk preferences to both phases; the block replaces raw scope selectors.

### 2) Simplify Phase

Run `code-simplify` (per Running Sub-Skills) with:

- the `resolved-scope` block
- `--no-verify` and `--no-report`
- any extra cleanup instructions from the user

Tell it not to broaden or rediscover scope.

### 3) Review Phase

Run `code-review` (per Running Sub-Skills) with:

- the same `resolved-scope` block
- `--fix`
- any `--skip-profile` flags from the user

Skip the naming profile only when the user asks for a speed-first pass, never by default.

### 4) Final Verification

- Reuse the review phase's post-fix verification as final when it covers the final touched scope.
- Otherwise run the narrowest checks that validate touched behavior:
  - formatter/lint on touched files
  - targeted tests for touched modules
  - typecheck when relevant
- Name every skipped check and why.

### 5) Report

Produce the Report section below. Both phases must have completed first (see Stop Conditions).

## Report

Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Files and functions touched, final state.

### Simplifications

Key changes from the simplify phase, derived from the diff when needed — the phase ran with `--no-report`.

### Review Findings and Fixes

Findings and applied fixes from the review phase, ordered `CRITICAL -> HIGH -> MEDIUM -> LOW`.

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Plain language — expand or gloss domain-specific terms. Include questions that need a user decision, phrased directly. Write `None.` when there are none.

## Stop Conditions

Stop and ask for direction when:

- `code-simplify` or `code-review` is not installed as a sibling skill.
- a sub-skill hits one of its own stop conditions.
- the review phase cannot run or cannot complete its fixes.

Completion gate: a polish run is complete only after both phases have run over the resolved scope and the Report above is produced. Never end the run after the simplify phase alone; if the review phase did not run, state explicitly that the polish is incomplete and which phase is missing.
