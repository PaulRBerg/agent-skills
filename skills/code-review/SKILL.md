---
argument-hint: '[paths] [--fix] [--deep] [--with-profile <name>] [--skip-profile <name>]'
disable-model-invocation: true
name: code-review
user-invocable: true
description: Use for code/PR review, audits, bug/security checks, reviewing diffs or changes, and finding issues in code.
---

# Code Review

## Objective

Find high-impact defects in changed code with evidence. Prioritize correctness, security, data integrity, shell/config safety, and regressions over style nits.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--fix`: Build findings internally, apply suggested fixes in severity order, then produce one final report and verify per the Verification section. Do not emit a separate pre-fix report.
- `--deep`: Run an exhaustive pass. Read every non-skipped profile triggered by the touched domains, include naming/intent review unless skipped, and broaden verification when risk warrants it.
- `--with-profile <name>`: Force an optional domain profile by stem or filename (e.g. `--with-profile shell`). Repeatable.
- `--skip-profile <name>`: Skip an optional domain profile by stem or filename (e.g. `--skip-profile naming`). Repeatable. If both `--with-profile` and `--skip-profile` name the same profile, skip wins.
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
5. Exclude generated, vendored, bulk, and low-signal files from manual review unless explicitly requested: lockfiles, minified bundles, build outputs, generated outputs, vendored code, and large data snapshots. When excluded files are relevant to correctness, emit an optional fenced code block tagged `excluded-scope`, one repo-relative path or glob per line, and cover them through verification or invariant checks.
6. If scope resolves to zero files, report that and stop.
7. Emit the scope as a fenced code block tagged `resolved-scope`, one repo-relative path per line. The block is authoritative: do not re-run scope commands or revisit exclusions afterward.

## Workflow

### 1) Resolve Scope

- Apply the Scope Resolution section, then read diffs plus minimal surrounding context.
- Treat any `excluded-scope` block as outside manual review but inside verification planning.

### 2) Apply Checks

- Classify files by changed behavior and touched risk surfaces.
- Apply the core checks plus matching profiles per Profile Dispatch, honoring `--with-profile` and `--skip-profile`.
- For generated, vendored, bulk, or low-signal files, review the generator, schema, contract, or invariant that produces or constrains them instead of hand-reviewing every generated row or file.

### 3) Build Findings

- Generate findings with location, impact, evidence, confidence, and a concrete fix.
- Assign severity with the Severity Model.

### 4) Fix or Wait

- Default: report findings and wait for confirmation before editing.
- With `--fix`: apply all suggested fixes in severity order (`CRITICAL -> HIGH -> MEDIUM -> LOW`), then verify per the Verification section. The first user-facing findings report is the final report after fixes.

### 5) Report

Produce the Report section below.

## Review Lens

Treat the user's request as the boundary for judging the diff.

- Surface hidden assumptions: if behavior intent is ambiguous or multiple interpretations would lead to different fixes, stop or record the assumption as residual risk instead of guessing.
- Prefer the smallest defensible fix. Flag single-use abstractions, speculative configurability, extra features, and broad rewrites when they increase risk or review burden.
- Treat unrelated churn as suspicious: adjacent refactors, formatting-only edits, deleted pre-existing dead code, or style conversions need direct traceability to the request.
- Verify goal fit: changed behavior should have concrete success criteria and a narrow check. Bugs need reproducing tests when practical; validation changes need invalid-input coverage; refactors need before/after safety checks.

## Core Review Checks

Apply on every run.

- `CORE-001` Behavior regression (`HIGH`): changed branch/state transition alters external behavior.
- `CORE-002` Error-path safety (`HIGH`): failures can cascade, crash, or return unsafe defaults.
- `CORE-003` Boundary handling (`HIGH`): null/empty/overflow/edge inputs are not handled.
- `CORE-004` Resource hygiene (`MEDIUM`): leaked timers/listeners/handles/connections.
- `CORE-005` Complexity hotspot (`MEDIUM`): change introduces avoidable coupling or hidden side effects.
- `CORE-006` Test gap (`MEDIUM`): changed behavior has no targeted test coverage.
- `CORE-007` Over-scoped change (`MEDIUM`): changed lines do not trace directly to the user's request or verified cleanup caused by the change.
- `CORE-008` Speculative complexity (`MEDIUM`): new abstraction, configurability, flexibility, or impossible-case handling adds code without proven need.
- `CORE-009` Weak success criteria (`MEDIUM`): implementation lacks a clear verification target for the behavior it claims to change.

## Profile Dispatch

Default profile dispatch is risk-triggered and capped. Read only profiles that materially improve defect discovery for the touched risk surfaces; file extension alone is not enough when core checks cover the change. Select at most three auto-selected profiles per pass unless `--deep` is set. Read each selected profile once, in full; every profile fits in a single read, so never page through or re-read one.

Honor `--skip-profile` exclusions first. Add `--with-profile` profiles after exclusions; user-forced profiles may exceed the auto-selection cap. Under `--deep`, read every non-skipped profile triggered by the touched domains and include `references/profiles/naming.md` unless skipped.

- `references/profiles/security.md`: auth, external input, secrets, crypto, public network surfaces, unsafe parsing.
- `references/profiles/configuration.md`: env/config, timeouts, retries, pools, limits, resource tuning, rollout controls.
- `references/profiles/go.md`: Go services, CLIs, concurrency, context propagation, error handling, modules, or tests.
- `references/profiles/typescript-react.md`: TypeScript/JavaScript/React/Node behavior changes where type, render, runtime, package, or async semantics matter.
- `references/profiles/python.md`: Python services, scripts, async workloads, packaging, data processing, or IO-heavy changes.
- `references/profiles/shell.md`: shell scripts, CI command blocks, deployment scripts, installer commands, or command quoting.
- `references/profiles/data-formats.md`: CSV/JSON/YAML/binary ingestion/export/parsing, schemas, generated data, migrations, or fixture updates.
- `references/profiles/naming.md`: naming/intent clarity. Do not run by default; run only with `--deep`, `--with-profile naming`, or explicit naming/intent review instructions.

## Generated and Bulk Files

- Exclude generated, vendored, bulk, and low-signal files from manual review unless the user explicitly asks to inspect them.
- Prefer reviewing the source that creates or constrains them: generator code, schemas, migrations, templates, lockfile update intent, fixture contracts, or serialization/deserialization paths.
- Validate affected outputs with invariant checks such as regeneration diffs, schema validation, parser round trips, row counts, checksums, targeted fixture tests, or package-manager lockfile checks.
- Mention excluded files in Scope or Verification when they affect confidence.

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
- For scope or simplicity findings, cite the changed line and the requested behavior it does not serve.
- Mention unrelated dead code as residual risk; do not suggest deleting it unless the user asked for cleanup.
- Keep style-only issues at LOW unless they create operational risk.

## Verification

Run the narrowest checks that validate touched behavior:

- formatter/lint on touched files
- targeted tests for touched modules
- typecheck when relevant
- invariant checks for any relevant `excluded-scope` outputs

Run broader checks only when risk warrants it, especially under `--deep` or when fixes touch shared contracts. Name every skipped check and why.

## Report

Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Reviewed files and any `excluded-scope` entries with the validation strategy used for them.

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
- multiple plausible interpretations would produce materially different findings or fixes.
- required validation tooling is unavailable and risk is high.
