---
argument-hint: "[paths] [--max-runtime DURATION]"
compatibility: Requires Git and local command and edit access.
disable-model-invocation: true
name: fresh-eyes-sweep
user-invocable: true
description:
  Audit an entire repository with fresh eyes for correctness errors, bugs, omissions, duplication, inconsistencies, and
  other evidenced mistakes; fix every safe issue and verify the result.
---

# Fresh Eyes Sweep

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Inspect the requested Git scope for evidenced mistakes, fix every safe issue, and continue until every mapped file is
accounted for and a full pass finds nothing new. A verified no-op is valid only after that coverage.

`--max-runtime DURATION` is optional: it is a positive integer followed by `m` or `h`, such as `45m` or `3h`. Reject an
invalid duration, unknown option, or ambiguous positional input. When a deadline is supplied, calculate it before
auditing and reserve the final 15% for aggregate validation and reporting, clamped to 5–30 minutes and never exceeding
the total runtime. At that window, settle in-flight slices and do not start new fixes; report an incomplete sweep with
its ledger rather than overrunning the deadline.

## Ledger Interface

Resolve `scripts/sweep-ledger.py` from this `SKILL.md`. Create the scratch ledger outside the repository:

```sh
uv run "<skill-dir>/scripts/sweep-ledger.py" init \
  --root <repo> --ledger <scratch.json> [<path>...]
```

With no paths, `init` maps the whole repository. With paths, it maps exactly those Git scopes. It records every tracked
and non-ignored untracked file plus each path's pre-existing worktree status. The helper does not classify generated,
vendored, binary, safe, important, or defective files.

Record an agent decision atomically only after inspecting or otherwise accounting for the path:

```sh
uv run "<skill-dir>/scripts/sweep-ledger.py" mark \
  --ledger <scratch.json> --status <pending|inspected|fixed|reported|excluded> \
  --path <path> [--path <path>...] [--reason <text>]
```

`excluded` requires an agent-written reason. Unknown paths or invalid batches fail without a partial update.

```sh
uv run "<skill-dir>/scripts/sweep-ledger.py" pending --ledger <scratch.json> [--limit <n>]
uv run "<skill-dir>/scripts/sweep-ledger.py" summary --ledger <scratch.json>
```

`pending` returns the next unaccounted paths in stable order. `summary` returns exact status counts, pre-existing edit
count, completeness, percentage inputs, and a ten-cell bar. Use those facts directly; never estimate progress or
reimplement ledger arithmetic.

## Setup

1. Require Git and read applicable repository instructions. Record the worktree root, starting commit, starting status,
   resolved scope, and any deadline and validation window.
2. Initialize the ledger for the requested scope. The agent may additionally inspect shared configuration and
   instructions needed to understand that scope; do not silently widen the ledger. If `init` maps more than roughly
   2,000 files and the user gave no `[paths]`, partition the mapped ledger into bounded, system-aware directory or
   subsystem slices and continue without asking solely because of file count. Keep the complete requested scope in the
   ledger and preserve cross-slice invariants through the system map and aggregate validation. When a supplied deadline
   cannot cover every slice, stop at its validation window and report the resumable frontier; ask only when no safe
   partition can preserve a material invariant and the user must choose a narrower outcome.
3. Classify generated, vendored, minified, binary, and bulk-data artifacts. Validate them through their generator,
   schema, or invariants when line-by-line review is inappropriate, then mark them with the agent's reason.
4. Build a compact system map: executable entry points, workspace or package dependency directions, public interfaces,
   generators and derived artifacts, external and persisted-data seams, and the owner of each material invariant. Trace
   the highest-risk workflows end to end before choosing slices.
5. Inspect recent history and diffs, especially the newest changes, to find affected callers, dependencies, tests,
   configuration, and docs. Rank slices and fixes by evidenced impact: correctness, data loss, security, and externally
   exposed personal-data or disclosure risk first; then reliability, maintainability, measured performance, and
   developer experience. Treat recency as one prioritization signal, never as a substitute for coverage.
6. Discover build, test, lint, typecheck, format, and codegen checks.
7. Establish a baseline for every safe, relevant check before the first fix. If it is red, prioritize reproducible
   failures before discretionary work; defer failures that need an unclear or prohibited action while continuing with
   independently verifiable work.
8. Preserve every pre-existing edit recorded by the ledger. Do not revert, absorb, commit, or report it as a finding.

After mapping, report `### 🔎 Sweep mapped — <files> files · <slices> slices · ledger <scratch.json>`. Slice count is an
agent organization choice; file count comes from the ledger.

The ledger outlives the session. A later session resumes the same sweep by pointing at the same ledger path instead of
re-running `init`: `pending` defines the frontier, and already-accounted paths are not reinspected. Carry the ledger
path into every progress update, and name it again when reporting an incomplete sweep, so the user can hand it to the
next session.

## Subagents

- Reviewers are read-only and default to model `sonnet`; agents that apply fixes default to model `opus`. Never spawn a
  subagent that implicitly inherits the session model — always set it explicitly.
- Announce the planned fan-out in one line before launching: agent count and the model of each group.
- Cap concurrent reviewers at 4 unless the user raises it.
- Record each spawned task ID in the ledger reason field for the slice it covers, so a later stop request resolves
  against real IDs instead of guesses.
- Give writing agents stable IDs, dependency waves, exact non-overlapping write scopes, repository constraints, and
  required completion evidence. Assign shared manifests, lockfiles, exports, and integration files to one sequential
  owner.
- Reconcile every wave before starting dependents. Use a fresh-context verifier after each nontrivial wave.
- Subagents and workers never commit. The coordinating session commits settled slices serially as checkpoint commits, so
  only one process touches the Git index.
- A lint-staged `Failed to get staged files!` or bare `"lint-staged" exited with code 1` is not enough to diagnose index
  contention. Retry as contention only when the same output explicitly names an index lock; otherwise inspect the hook
  diagnostics emitted by `ai-commit` or the lint-staged debug trace before deciding whether to fix, report, or use
  `$commit` to apply `ai-commit`'s transaction-aware unrelated-hook recovery.

## Inspect and Fix

Work through coherent slices so implementation, callers, tests, configuration, and documentation stay visible together.
Trace important control, data, concurrency, and error paths end to end. Hunt for concrete bugs, omissions, invalid
assumptions, unhandled edges, security/reliability failures, inconsistencies, duplication, dead code, stale docs, and
needless complexity. Also inspect evidenced problems in performance, dependencies, data formats and extensions,
configuration, observability, accessibility, agent context, naming, and directory structure. Style preferences and
unverified hunches are not findings.

At applicable external and persisted-data seams, inspect validation; domain precision and units; deterministic ordering
and deduplication; idempotency and repeat-run behavior; atomicity and interruption safety; retry and pagination
completeness; bounded concurrency, cancellation, and resource cleanup; and secret, log, path, temporary-file, and
command safety.

Confirm each issue before editing. Fix the smallest root cause when intent is clear and verification is available; add a
focused regression test when useful. Mark `reported` when a safe fix would alter a public contract, intent is ambiguous,
or verification is unavailable. Do not add speculative features, broad refactors, or cosmetic churn.

Treat source files over 1000 lines and test files over 2000 lines as discovery candidates only. Split a file only when
cohesion, coupling, change risk, or testability establishes a better seam; line count alone is not evidence. When a
confirmed structural issue requires interface or seam redesign, use `$codebase-design` when available. Centralize the
invariant in its owning module, apply the deletion test to pass-through modules, introduce a seam only where behavior
actually varies, and keep callers and tests on the resulting interface.

Before changing an interface, persisted format, exported name, or path, enumerate and migrate every producer, consumer,
schema, fixture, generator, export or manifest, script or recipe, check, configuration reference, and document. Search
for the old identifier afterward and account for every intentional remainder. Apply dependency or framework updates,
data-format or extension changes, renames, and reorganizations only when the migration is atomic, compatibility is
demonstrable, and repository checks can prove it. Do not retain a performance change without a recorded baseline metric
and repeatable benchmark.

If an experiment fails its evidence bar, revert only that experiment's attributable edits; never use repository-wide
clean, checkout, or reset commands. After each nontrivial change wave, run `$code-polish` over that wave's exact changed
file union when available. Otherwise apply the same fixed-scope contract inline: simplify only where comprehension or
defect risk measurably improves, review by severity, fix evidenced defects, and rerun the narrowest proving checks.

On long runs, post updates only after coherent slices settle, using the ledger summary's exact bar and counts. The bar
means path accounting, not depth of inspection.

## Verify and Report

Run the narrowest check proving each fix, then aggregate checks scoped to changed files. Reinspect affected paths and
repeat until a pass finds no new evidenced issue. During a supplied deadline's validation window, reconcile owned edits
and run the aggregate format, lint, type, test, build, and invariant checks justified by the final changed-file union.
Compare final results with the recorded baseline. Audit coverage, fixes, and checks against tool output before claiming
completion.

Lead with
`### ✅ Sweep ledger complete — <accounted>/<mapped> files accounted (<inspected> inspected, <excluded> excluded)` only
when helper `complete` is true; otherwise use `### ⛔ Sweep incomplete`. Summarize fixed, reported, excluded, and check
counts. Include a compact `Check | Baseline | Final` table, changed artifacts and verified fixes, and subagent results.
When non-empty, also include reverted experiments with the failed evidence, unresolved findings with their evidence,
risk, and required decision, and residual risk with its next proving check. On `### ⛔ Sweep incomplete`, name the
ledger path so the next session can resume from `pending`. Do not dump the scratch ledger's contents, unrelated
pre-existing changes, or bulk data; include task-relevant evidence when it materially supports the report.

Completion requires every mapped path accounted for, every finding fixed and verified or reported with evidence, and
every relevant check passing or its failure attributed.
