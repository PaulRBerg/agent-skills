---
argument-hint: '[--all]'
context: fork
disable-model-invocation: false
name: code-polish
user-invocable: true
description: This skill should be used when the user asks to "polish code", "simplify and review", "clean up and review code", "full code polish", "simplify then review", "refactor and review", "simplify and fix", "clean up and fix", or wants a combined simplification and review workflow on recently changed code.
---

# Code Polish

## Overview

Combined simplification and review pipeline for recently changed code. Simplify for readability and maintainability, then review for correctness, security, and quality, auto-applying all fixes. Run as a single pass with shared scope determination and one verification phase at the end.

## Scope Resolution

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git repository.
2. If `$ARGUMENTS` includes `--all`, scope is `git diff --name-only --diff-filter=ACMR`.
3. Otherwise, if user provides file paths/patterns or a commit/range, scope is exactly those targets.
4. Otherwise, scope is session-modified files.
5. Exclude generated/low-signal files unless requested: lockfiles, minified bundles, build outputs, vendored code.
6. If scope resolves to zero files, report and stop. Do not widen scope silently.

## Workflow

### 1) Determine Scope

- Resolve target files using the "Scope Resolution" section above.
- Read all in-scope files and surrounding context once. This context serves both the simplification and review phases.

### 2) Build Baseline

- Identify invariants that must not change:
  - function signatures and exported APIs
  - state transitions and side effects
  - persistence/network behavior
  - user-facing messages and error semantics
- Note available verification commands (lint, tests, typecheck).
- Assess risk level based on change scope: authentication, authorization, payments, persistence, external APIs, crypto.

### 3) Simplify

Apply simplification passes in order:

1. **Control flow**: Flatten deep nesting with guard clauses and early returns. Replace nested ternaries.
2. **Naming and intent**: Rename ambiguous identifiers. Separate mixed concerns into helpers with intent-revealing names.
3. **Duplication**: Remove obvious duplication. Abstract only when 2+ call sites benefit.
4. **Data shaping**: Break dense transform chains into named intermediate steps. Keep hot-path performance stable.
5. **Type clarity**: Tighten type annotations where they improve readability without broad churn.

Safety constraints during simplification:

- Do not convert sync APIs to async (or reverse) unless explicitly requested.
- Do not alter error propagation strategy.
- Do not remove logging, telemetry, guards, or retries.
- Do not collapse domain-specific steps into generic helpers that hide intent.

Do NOT run verification after this phase. Defer all verification to Step 5.

### 4) Review and Fix

Review the code (including simplifications just applied) using the checklist below. Auto-apply all fixes in severity order: CRITICAL → HIGH → MEDIUM → LOW. Do not wait for user confirmation.

**Severity classification**:

- **CRITICAL**: Security vulnerabilities enabling unauthorized access, data exfiltration, or code execution. Data loss scenarios. Production outage risks. Breaking API changes without versioning.
- **HIGH**: Logic errors in core functionality. Performance degradation (N+1 queries, missing indexes). Error handling gaps causing cascading failures. Race conditions. Missing input validation on external data.
- **MEDIUM**: Maintainability issues (tight coupling, god objects, SRP violations). Missing validation on internal boundaries. Code duplication suggesting abstraction. Missing transaction boundaries.
- **LOW**: Style inconsistencies not enforced by linters. Documentation gaps. Minor naming improvements.

**Review focus areas**:

- **Security**: secrets in code, injection vulnerabilities, auth/authz checks, crypto usage.
- **Logic**: null/undefined handling, boundary conditions, error paths, loop termination.
- **Performance**: algorithmic complexity, resource cleanup, caching opportunities.
- **Maintainability**: coupling, SRP, magic numbers, actionable error messages.

For domain-specific review depth, consult:

- **`references/security.md`** — OWASP, auth, crypto, input validation
- **`references/typescript-react.md`** — Frontend, Node.js, React, TypeScript patterns
- **`references/python.md`** — Type hints, async, exceptions, common bugs
- **`references/shell.md`** — Quoting, error handling, portability, injection
- **`references/smart-contracts.md`** — Solidity, Solana, economic attacks
- **`references/configuration.md`** — Limits, timeouts, environment-specific validation
- **`references/data-formats.md`** — CSV, JSON, parsing safety, encoding
- **`references/naming.md`** — Language-specific naming conventions and anti-patterns

### 5) Verify and Report

Run verification once covering all changes (simplifications + review fixes):

- Formatter/lint on touched files.
- Targeted tests related to touched modules.
- Typecheck when relevant.
- If fast targeted checks pass, run broader checks only when risk warrants it.
- If checks cannot run, state what was skipped and why.

Report:

1. **Scope**: Files and functions touched.
2. **Simplifications**: Key changes with concise rationale.
3. **Review findings**: Grouped by severity (only findings that resulted in fixes).
4. **Fixes applied**: Concrete list with file references.
5. **Verification**: Commands run and outcomes.
6. **Residual risks**: Assumptions or items needing manual review.

## Stop Conditions

Stop and ask for direction when:

- Simplification requires changing public API/contracts.
- Behavior parity cannot be confidently verified.
- Code appears intentionally complex due to domain constraints.
- Scope implies a larger redesign rather than polish.
- A CRITICAL security finding requires architectural changes beyond auto-fix scope.
