---
context: fork
name: code-polish
user-invocable: true
description: This skill should be used when the user asks to "polish code", "simplify and review", "clean up and review code", "full code polish", "simplify then review", "refactor and review", "simplify and fix", "clean up and fix", or wants a combined simplification and review workflow on recently changed code.
---

# Code Polish

> **File paths**: All `references/` paths in this skill resolve under `~/.agents/skills/code-review/`. Do not look for them in the current working directory.

## Overview

Combined simplification and review pipeline for recently changed code. Simplify for readability and maintainability, then review for correctness, security, and quality, auto-applying all fixes. Run as a single pass with shared scope determination and one verification phase at the end.

## Workflow

### 1) Determine Scope

- Verify repository context: `git rev-parse --git-dir`. If this fails, stop.
- Identify candidate files:
  - If the user provides a file list or path in `$ARGUMENTS`, use it.
  - Otherwise: `git diff --name-only --diff-filter=ACMR`.
- Exclude generated or low-signal files: lockfiles, minified bundles, build outputs, vendored code.
- If no target files found, ask for explicit scope.
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

- **`~/.agents/skills/code-review/references/security.md`** — OWASP, auth, crypto, input validation
- **`~/.agents/skills/code-review/references/typescript-react.md`** — Frontend, Node.js, React, TypeScript patterns
- **`~/.agents/skills/code-review/references/python.md`** — Type hints, async, exceptions, common bugs
- **`~/.agents/skills/code-review/references/shell.md`** — Quoting, error handling, portability, injection
- **`~/.agents/skills/code-review/references/smart-contracts.md`** — Solidity, Solana, economic attacks
- **`~/.agents/skills/code-review/references/configuration.md`** — Limits, timeouts, environment-specific validation
- **`~/.agents/skills/code-review/references/data-formats.md`** — CSV, JSON, parsing safety, encoding
- **`~/.agents/skills/code-review/references/naming.md`** — Language-specific naming conventions and anti-patterns

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
