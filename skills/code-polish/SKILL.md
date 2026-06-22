---
argument-hint: '[paths] [--simplify] [--review] [--with-profile <name>] [--skip-profile <name>]'
disable-model-invocation: true
name: code-polish
user-invocable: true
description: 'Use to polish recently changed code: simplify for readability/maintainability and run a risk-profiled review that autonomously applies fixes. Default runs both passes; pass --simplify or --review for one. Covers code/PR review, audits, bug/security checks, reviewing diffs or changes, cleanup, refactoring, and reducing complexity.'
---

# Code Polish

## Objective

Polish recently changed code in one run: simplify for readability and maintainability, then run an exhaustive risk-profiled review that applies fixes autonomously. Resolve scope once, prioritize correctness/security/data-integrity over style, and produce one user-facing report. Run either pass alone with `--simplify` or `--review`.

## Modes

Passes are selected by flags and default to running both, in this fixed order:

- `--simplify` only: run the **Simplify Pass**, then verify and report.
- `--review` only: run the **Review Pass** (always applies fixes), then verify and report.
- neither flag, or both flags: run the **Simplify Pass**, then the **Review Pass**, over one resolved scope.

When both run, the review pass sees the simplified code: reuse file contents already loaded during simplify and only re-read files the simplify pass changed.

## Arguments

- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.
- `--simplify`: Run the simplify pass only.
- `--review`: Run the review pass only. The review pass always applies fixes autonomously; there is no report-and-wait mode.
- `--with-profile <name>`: Force an optional review profile by stem or filename (e.g. `--with-profile shell`). Repeatable. Review pass only.
- `--skip-profile <name>`: Skip an optional review profile by stem or filename (e.g. `--skip-profile naming`). Repeatable. If both `--with-profile` and `--skip-profile` name the same profile, skip wins. Review pass only.
- Extra cleanup instructions (e.g. "split `_lib.ts` into smaller files"): executed during the simplify pass.
- Default: run both passes — simplify, then review-with-autofix — over the resolved scope.

## Scope Resolution

Resolve scope once, then treat the result as fixed for the rest of the run.

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git repository.
2. If the request names targets — file paths/patterns, a commit/range, a natural-language subset (e.g. "the parser changes"), or a `resolved-scope` fenced block with one repo-relative path per line — scope is exactly those targets. Map natural-language subsets to concrete paths before continuing.
3. Otherwise, scope is **only** session-modified files: files created or edited earlier in this session. Do not include other uncommitted changes.
4. If there are no session-modified files, or earlier conversation history is not visible in this context, fall back to all uncommitted files. Get tracked and untracked paths in a single command, then combine and de-duplicate:
   - `git --no-pager diff --name-only --diff-filter=ACMR; git --no-pager ls-files --others --exclude-standard`
5. Exclude generated, vendored, bulk, and low-signal files from manual simplify/review unless explicitly requested: lockfiles, minified bundles, build outputs, generated outputs, vendored code, and large data snapshots. When excluded files are relevant to correctness, emit an optional fenced code block tagged `excluded-scope`, one repo-relative path or glob per line, and cover them through verification or invariant checks.
6. If scope resolves to zero files, report that and stop.
7. Emit the scope as a fenced code block tagged `resolved-scope`, one repo-relative path per line. The block is authoritative: do not re-run scope commands or revisit exclusions afterward.

## Workflow

### 1) Resolve Scope

- Apply the Scope Resolution section and emit the `resolved-scope` block.
- Emit `excluded-scope` only when generated, vendored, bulk, or low-signal files are intentionally excluded but still relevant to verification.
- Determine the active passes from the flags (default: both).

### 2) Simplify Pass

Skip unless the simplify pass is active. Otherwise apply the Simplify Pass section to the resolved scope, plus any extra cleanup instructions.

### 3) Review Pass

Skip unless the review pass is active. Otherwise apply the Review Pass section to the resolved scope (and any `excluded-scope` via verification). Build findings internally and apply fixes autonomously in severity order — no separate pre-fix report.

### 4) Verify

Run the Verification section once, after the active passes, over the final touched scope.

### 5) Report

Produce the Report section. Include only the subsections for passes that ran.

## Simplify Pass

Apply high-confidence simplifications that materially improve comprehension or reduce defect risk while preserving behavior, public contracts, and side effects. Default to no edit unless the change is high-confidence; skip no-op passes and report that the code is already clear rather than churning it.

### Operating Rules

- State assumptions before editing. If multiple interpretations would change the simplification or verification strategy, present them and stop for direction.
- Preserve runtime behavior exactly: inputs, outputs, side effects, and error behavior stay stable. Identify invariants first — function signatures and exported APIs, state transitions and side effects, persistence/network behavior, and user-facing messages or error semantics that are externally relied on.
- Prefer project conventions over personal preferences. Infer them from existing code, linters, formatters, and tests.
- Make small, reversible edits. Every changed line should trace to the user's request, requested cleanup, or cleanup caused by your own edits.
- Write the minimum code that solves the requested problem. Do not add features, single-use abstractions, speculative flexibility, or configurability the user did not request.
- Clean up only your own mess: remove imports, variables, functions, and files made unused by your changes; mention pre-existing dead code in Residual Risks instead of deleting it.
- Run naming-only refactors only when they create a concrete clarity or safety gain and can be safely verified. Never reshape APIs solely for taste.
- For generated, vendored, bulk, or low-signal files, simplify the generator, schema, or contract when possible and validate outputs with invariant checks instead of hand-editing every generated row or file.
- Call out uncertainty immediately when behavior may change.

### Apply Simplification Passes

Apply the full checklist in this order:

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

### Safety Constraints

- Do not convert sync APIs to async (or reverse) unless explicitly requested.
- Do not alter error propagation strategy unless behavior remains equivalent and verified.
- Do not remove logging, telemetry, guards, or retries that encode operational intent.
- Do not collapse domain-specific steps into generic helpers that hide intent.

### Simplification Heuristics

- Prefer explicit local variables over nested inline expressions when it reduces cognitive load.
- Prefer one clear branch per condition over compact but ambiguous condition trees.
- Keep function length manageable, but do not split purely for line count.
- Keep comments that explain intent, invariants, or non-obvious constraints; remove comments that restate obvious code behavior.
- Optimize for the next maintainer's comprehension time, not minimum character count.

### Anti-Patterns

- Do not perform speculative architecture rewrites or introduce framework-wide patterns while simplifying a small local change.
- Do not replace understandable duplication with opaque utility layers.
- Do not bundle unrelated cleanups into one patch.
- Do not add error handling for impossible scenarios.
- Do not preserve code volume for its own sake; if a simpler equivalent approach exists, use it or explain why it does not satisfy the request.

## Review Pass

Find high-impact defects in changed code with evidence, then apply fixes autonomously. Prioritize correctness, security, data integrity, shell/config safety, and regressions over style nits. There is no dry-run: build findings internally, apply fixes in severity order (`CRITICAL → HIGH → MEDIUM → LOW`), and exercise judgment — smallest defensible fix, skip or down-rank low-value churn, and when behavior intent is ambiguous record a residual risk instead of guessing.

### Review Lens

Treat the user's request as the boundary for judging the diff.

- Surface hidden assumptions: if behavior intent is ambiguous or multiple interpretations would lead to different fixes, stop or record the assumption as residual risk instead of guessing.
- Prefer the smallest defensible fix. Flag single-use abstractions, speculative configurability, extra features, and broad rewrites when they increase risk or review burden.
- Treat unrelated churn as suspicious: adjacent refactors, formatting-only edits, deleted pre-existing dead code, or style conversions need direct traceability to the request.
- Verify goal fit: changed behavior should have concrete success criteria and a narrow check. Bugs need reproducing tests when practical; validation changes need invalid-input coverage; refactors need before/after safety checks.

### Core Review Checks

Apply on every review.

- `CORE-001` Behavior regression (`HIGH`): changed branch/state transition alters external behavior.
- `CORE-002` Error-path safety (`HIGH`): failures can cascade, crash, or return unsafe defaults.
- `CORE-003` Boundary handling (`HIGH`): null/empty/overflow/edge inputs are not handled.
- `CORE-004` Resource hygiene (`MEDIUM`): leaked timers/listeners/handles/connections.
- `CORE-005` Complexity hotspot (`MEDIUM`): change introduces avoidable coupling or hidden side effects.
- `CORE-006` Test gap (`MEDIUM`): changed behavior has no targeted test coverage.
- `CORE-007` Over-scoped change (`MEDIUM`): changed lines do not trace directly to the user's request or verified cleanup caused by the change.
- `CORE-008` Speculative complexity (`MEDIUM`): new abstraction, configurability, flexibility, or impossible-case handling adds code without proven need.
- `CORE-009` Weak success criteria (`MEDIUM`): implementation lacks a clear verification target for the behavior it claims to change.

### Profile Dispatch

Profile dispatch is risk-triggered and exhaustive: select every profile whose risk surface the diff touches, not by file extension alone when core checks already cover the change.

| Touched surface                                                                              | Profile         |
| -------------------------------------------------------------------------------------------- | --------------- |
| auth, external input, secrets, crypto, public network, unsafe parsing                        | `security`      |
| env/config, timeouts, retries, pools, limits, resource tuning, rollout controls              | `configuration` |
| Go services, CLIs, concurrency, context propagation, error handling, modules, tests          | `go`            |
| TypeScript behavior where type, module, package, generated type, or async semantics matter   | `typescript`    |
| Python services, scripts, async workloads, packaging, data processing, IO-heavy changes      | `python`        |
| shell scripts, CI/deploy/installer command blocks, command quoting                           | `shell`         |
| CSV/JSON/YAML/binary ingestion/export/parsing, schemas, generated data, migrations, fixtures | `data-formats`  |
| every review (naming/intent clarity) unless `--skip-profile naming`                          | `naming`        |

Honor `--skip-profile` exclusions first, then add `--with-profile` profiles. Read all selected profiles as **parallel `Read` calls in a single message** — one batch, each file once, never paged or re-read. Profiles live at `references/profiles/<name>.md` relative to this file.

### Generated and Bulk Files

- Exclude generated, vendored, bulk, and low-signal files from manual review unless the user explicitly asks to inspect them.
- Prefer reviewing the source that creates or constrains them: generator code, schemas, migrations, templates, lockfile update intent, fixture contracts, or serialization/deserialization paths.
- Validate affected outputs with invariant checks: regeneration diffs, schema validation, parser round trips, row counts, checksums, targeted fixture tests, or package-manager lockfile checks.
- Mention excluded files in Scope or Verification when they affect confidence.

### Severity Model

- **CRITICAL**: exploitable security flaw, data loss path, or outage risk on critical paths.
- **HIGH**: logic defect or performance failure that can break core behavior.
- **MEDIUM**: maintainability/reliability issue likely to cause near-term defects.
- **LOW**: localized clarity/style/documentation improvements.

### Evidence Rules

- Tie every finding to concrete code evidence at real, verified locations; never fabricate file paths or line numbers.
- Show the input or state that triggers the failure and the changed lines or missing guards that cause it.
- State blast radius and failure mode succinctly. Merge duplicate findings.
- Prefer targeted fixes over broad rewrites.
- For scope or simplicity findings, cite the changed line and the requested behavior it does not serve.
- Mention unrelated dead code as residual risk; do not delete it unless the user asked for cleanup.
- Keep style-only issues at LOW unless they create operational risk.

## Verification

Run the narrowest checks that validate touched behavior:

- formatter/lint on touched files
- targeted tests for touched modules
- typecheck when relevant
- invariant checks for any relevant `excluded-scope` outputs

Issue independent checks as **parallel tool calls in a single message**, preferring the project's task runner and the narrowest per-file/per-workspace scope over repo-wide commands. When scanning beyond the diff (call sites, usages), prefer fast tools (`rg`/`fd`/`ast-grep`) when available and batch independent searches and reads. Run broader checks only when risk warrants it, especially when changes touch shared contracts. Name every skipped check and why.

## Report

Use these section headings, in this order. Include only subsections for passes that ran. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.

### Scope

Files and functions touched, final state, and any `excluded-scope` entries with the validation strategy used for them.

### Simplifications

Only if the simplify pass ran. One sentence per meaningful change, focused on the readability or maintainability gain. Confirm behavior-preservation assumptions explicitly.

### Review Findings and Fixes

Only if the review pass ran. Findings and applied fixes ordered `CRITICAL → HIGH → MEDIUM → LOW`. For each: `[SEVERITY] Title — path/to/file.ext:line`, concrete impact, evidence, the fix applied, and confidence (`high | medium | low`).

### Verification

Commands run and outcomes, including skipped checks.

### Residual Risks

One line per risk: `Assumed <assumption>; if wrong, <what breaks>; check via <command or inspection>.` Plain language — expand or gloss domain-specific terms. Include questions that need a user decision, phrased directly. Write `None.` when there are none.

## Stop Conditions

Stop and ask for direction when:

- a required review profile file (`references/profiles/*.md`) is missing.
- simplification or a fix requires changing public API/contracts or implies a larger redesign rather than a local change.
- behavior parity cannot be confidently verified, or the code appears intentionally complex due to domain constraints.
- behavior intent is too ambiguous to classify severity, or multiple plausible interpretations would produce materially different edits, findings, or fixes.
- required validation tooling is unavailable and risk is high.
