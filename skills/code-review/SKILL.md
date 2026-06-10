---
argument-hint: '[paths] [--fix] [--skip-profile <name>]'
disable-model-invocation: true
name: code-review
user-invocable: true
description: This skill should be used when the user asks to "review code", "review PR", "code review", "audit code", "check for bugs", "security review", "review my changes", "find issues in this code", "review the diff", or asks for pull request review or code audit.
---

# Code Review

## Objective

Find high-impact defects in changed code with evidence. Prioritize security, correctness, and regressions over style nits.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--fix`: Apply all suggested fixes in severity order after reporting findings, then verify per the Verification section.
- `--skip-profile <name>`: Skip an optional domain profile by stem or filename (e.g. `--skip-profile naming`). Repeatable.
- Default: report findings and wait for confirmation before editing.

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

- Apply the Scope Resolution section, then read diffs plus minimal surrounding context.

### 2) Apply Checks

- Classify files by domain and risk.
- Apply the core checks plus matching profiles per Profile Dispatch, honoring `--skip-profile` exclusions.

### 3) Build Findings

- Generate findings with location, impact, evidence, confidence, and a concrete fix.
- Assign severity with the Severity Model.

### 4) Fix or Wait

- Default: report findings and wait for confirmation before editing.
- With `--fix`: apply all suggested fixes in severity order (`CRITICAL -> HIGH -> MEDIUM -> LOW`), then verify per the Verification section.

### 5) Report

Produce the Report section below.

## Core Review Checks

Apply on every run.

- `CORE-001` Behavior regression (`HIGH`): changed branch/state transition alters external behavior.
- `CORE-002` Error-path safety (`HIGH`): failures can cascade, crash, or return unsafe defaults.
- `CORE-003` Boundary handling (`HIGH`): null/empty/overflow/edge inputs are not handled.
- `CORE-004` Resource hygiene (`MEDIUM`): leaked timers/listeners/handles/connections.
- `CORE-005` Complexity hotspot (`MEDIUM`): change introduces avoidable coupling or hidden side effects.
- `CORE-006` Test gap (`MEDIUM`): changed behavior has no targeted test coverage.

## Profile Dispatch

Select at most three profiles per pass — the highest-risk matches for the touched files — unless the user requests a deep audit. Read each selected profile once, in full; every profile fits in a single read, so never page through or re-read one. Honor `--skip-profile` exclusions.

- `references/profiles/security.md`: auth, external input, secrets, crypto, public network surfaces, unsafe parsing.
- `references/profiles/configuration.md`: env/config, timeouts, retries, pools, limits, resource tuning, rollout controls.
- `references/profiles/typescript-react.md`: TypeScript/JavaScript/React/Node files.
- `references/profiles/python.md`: Python services, scripts, async workloads.
- `references/profiles/shell.md`: shell scripts, CI command blocks, deployment scripts.
- `references/profiles/smart-contracts.md`: Solidity/Solana/on-chain protocol code.
- `references/profiles/data-formats.md`: CSV/JSON/YAML/binary ingestion/export/parsing.
- `references/profiles/naming.md`: naming/intent clarity. Optional; skippable via `--skip-profile naming`.

## Severity Model

- **CRITICAL**: exploitable security flaw, data loss path, or outage risk on critical paths.
- **HIGH**: logic defect or performance failure that can break core behavior.
- **MEDIUM**: maintainability/reliability issue likely to cause near-term defects.
- **LOW**: localized clarity/style/documentation improvements.

## Evidence Rules

- Tie every finding to concrete code evidence at real, verified locations; never fabricate file paths or line numbers.
- Show the input or state that triggers the failure and the changed lines or missing guards that cause it.
- State blast radius and failure mode succinctly.
- Merge duplicate findings.
- Prefer targeted fixes over broad rewrites.
- Keep style-only issues at LOW unless they create operational risk.

## Verification

Run the narrowest checks that validate touched behavior:

- formatter/lint on touched files
- targeted tests for touched modules
- typecheck when relevant

Run broader checks only when risk warrants it. Name every skipped check and why.

## Report

Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Reviewed files and any excluded patterns.

### Findings

Order by severity: `CRITICAL -> HIGH -> MEDIUM -> LOW`. For each finding:

- `[SEVERITY] Title — path/to/file.ext:line`
- Impact: concrete user/system impact.
- Evidence: exact code behavior or diff evidence.
- Fix: smallest practical remediation.
- Confidence: `high | medium | low`.

### Suggested Fixes

Only without `--fix`.

### Applied Fixes

Only with `--fix`. List each change with file references.

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Plain language — expand or gloss domain-specific terms. Include questions that need a user decision, phrased directly. Write `None.` when there are none.

## Stop Conditions

Stop and ask for direction when:

- fixes require API/contract redesign.
- behavior intent is too ambiguous to classify severity.
- required validation tooling is unavailable and risk is high.
