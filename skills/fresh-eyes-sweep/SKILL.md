---
argument-hint: "[paths]"
compatibility: Requires Git and local command and edit access.
disable-model-invocation: true
name: fresh-eyes-sweep
user-invocable: true
description:
  Audit an entire repository with fresh eyes for correctness errors, bugs, omissions, duplication, inconsistencies, and
  other evidenced mistakes; fix every safe issue and verify the result.
---

# Fresh Eyes Sweep

Inspect the requested Git scope for evidenced mistakes, fix every safe issue, and continue until every mapped file is
accounted for and a full pass finds nothing new. A verified no-op is valid only after that coverage.

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

1. Require Git and read applicable repository instructions.
2. Initialize the ledger for the requested scope. The agent may additionally inspect shared configuration and
   instructions needed to understand that scope; do not silently widen the ledger.
3. Classify generated, vendored, minified, binary, and bulk-data artifacts. Validate them through their generator,
   schema, or invariants when line-by-line review is inappropriate, then mark them with the agent's reason.
4. Inspect recent history and diffs, especially the newest changes, to find affected callers, dependencies, tests,
   configuration, and docs. Recency sets order, never coverage.
5. Discover build, test, lint, typecheck, format, and codegen checks.
6. Preserve every pre-existing edit recorded by the ledger. Do not revert, absorb, commit, or report it as a finding.

After mapping, report `### 🔎 Sweep mapped — <files> files · <slices> slices`. Slice count is an agent organization
choice; file count comes from the ledger.

## Inspect and Fix

Work through coherent slices so implementation, callers, tests, configuration, and documentation stay visible together.
Trace important control, data, and error paths end to end. Hunt for concrete bugs, omissions, invalid assumptions,
unhandled edges, security/reliability failures, inconsistencies, duplication, dead code, stale docs, and needless
complexity. Style preferences and unverified hunches are not findings.

Confirm each issue before editing. Fix the smallest root cause when intent is clear and verification is available; add a
focused regression test when useful. Mark `reported` when a safe fix would alter a public contract, intent is ambiguous,
or verification is unavailable. Do not add speculative features, broad refactors, or cosmetic churn.

On long runs, post updates only after coherent slices settle, using the ledger summary's exact bar and counts. The bar
means path accounting, not depth of inspection.

## Verify and Report

Run the narrowest check proving each fix, then aggregate checks scoped to changed files. Reinspect affected paths and
repeat until a pass finds no new evidenced issue. Audit coverage, fixes, and checks against tool output before claiming
completion.

Lead with
`### ✅ Sweep ledger complete — <accounted>/<mapped> files accounted (<inspected> inspected, <excluded> excluded)` only
when helper `complete` is true; otherwise use `### ⛔ Sweep incomplete`. Summarize fixed, reported, excluded, and check
counts, then include fixed artifacts, verification, unresolved findings, and residual risk only when non-empty. Do not
expose the scratch ledger, pre-existing changes, or private/bulk data.

Completion requires every mapped path accounted for, every finding fixed and verified or reported with evidence, and
every relevant check passing or its failure attributed.
