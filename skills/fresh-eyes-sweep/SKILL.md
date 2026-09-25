---
argument-hint: "[paths] [--max-runtime DURATION]"
compatibility: Requires Git and local command and edit access.
disable-model-invocation: true
name: fresh-eyes-sweep
skill-dependencies:
  - codebase-design
  - code-polish
  - commit
description:
  Audit an entire repository with fresh eyes for correctness errors, bugs, omissions, duplication, inconsistencies,
  stale or duplicate tests, stale comments, and other evidenced mistakes; fix every safe issue and verify the result.
---

# Fresh Eyes Sweep

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Inspect the requested Git scope for evidenced mistakes, fix every safe issue, and account for every mapped file. A
verified no-op requires that coverage and a full pass that finds nothing new. Leave sound work unchanged; edits are not
required to demonstrate a successful sweep.

`--max-runtime DURATION` is optional: it is a positive integer followed by `m` or `h`, such as `45m` or `3h`. Reject an
invalid duration, unknown option, or ambiguous positional input. When a deadline is supplied, calculate it before
auditing and reserve the final 15% for aggregate validation and reporting, clamped to 5–30 minutes and never exceeding
the total runtime. At that window, settle in-flight slices and do not start new fixes; report an incomplete sweep with
its ledger rather than overrunning the deadline.

## Overnight Autonomy

At invocation, read the environment's local time. If it is strictly after 22:00 or strictly before 08:00, treat the
entire run as autonomous even if it later crosses a boundary. Exactly 22:00 and 08:00 are outside this window.

During an autonomous overnight run:

- Do not ask the user any questions or pause for clarification, selection, or approval. This does not broaden the
  skill's authority: leave destructive, disclosure, purchase, public-contract, and other approval-dependent actions
  undone.
- Use the smallest safe reversible interpretation and continue all independent work. Put every ambiguity, blocked issue,
  and approval-dependent choice on an overnight backlog instead of interrupting the run.
- Present the backlog at the end with each item's evidence, safe disposition, impact, and decision needed. Phrase the
  entries as findings, not questions. Omit the section when the backlog is empty.

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

`excluded` requires an agent-written reason. Unknown paths or invalid batches fail without a partial update. Concurrent
`mark` calls serialize on a sidecar `<scratch.json>.lock`, so parallel subagents may mark their own paths.

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
   partition can preserve a material invariant and the user must choose a narrower outcome. During an autonomous
   overnight run, record that choice in the overnight backlog and complete everything that remains independently safe.
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

- When the host supports model selection, choose reviewer and fixer models deliberately for the task; otherwise use the
  host default.
- Announce the planned fan-out in one line before launching: agent count and the model of each group.
- Cap concurrent reviewers at 4 unless the user raises it.
- Record each spawned task ID in the coordinator's slice plan so a later stop request resolves against real IDs. The
  ledger's `reason` records exclusions only.
- Give writing agents stable IDs, dependency waves, exact non-overlapping write scopes, repository constraints, and
  required completion evidence. In every slice brief, completion evidence must include every discovered strict static
  gate — typecheck, lint, and format/import order — applicable to the languages in the slice's write scope, scoped as
  narrowly as the tool permits. A gate that only runs repository-wide either runs once at slice settlement when cheap or
  is explicitly deferred to aggregate validation in the agent's result. Assign shared manifests, lockfiles, exports, and
  integration files to one sequential owner.
- Reconcile every wave before starting dependents. Use a fresh-context verifier after each nontrivial wave.
- Subagents and workers never commit. The coordinating session commits settled slices serially as checkpoint commits, so
  only one process touches the Git index.
- On lint-staged or other hook failures during a checkpoint commit, follow `$commit`'s failure-recovery guidance rather
  than diagnosing index contention here.

## Inspect and Fix

Work through coherent slices so implementation, callers, tests, configuration, and documentation stay visible together.
For each slice, reason from first principles: identify the intended outcome and required behavior from the user's
request and repository evidence. Treat the current implementation as something to justify, not as a requirement.
Interrogate it in this order:

1. What is unnecessary, overly complicated, or based on weak assumptions? Challenge those assumptions against evidence.
2. What can be deleted entirely while preserving required behavior and contracts? Check consumers and invariants before
   concluding that a piece is unnecessary.
3. After removing unnecessary pieces, what remaining logic, interfaces, or workflow can be simplified?

Prefer deleting over simplifying, simplifying over optimizing, and optimizing over automating. Apply confirmed, safe
improvements within the requested scope; this ordering does not justify dropping requirements or automating needless
work.

Trace important control, data, concurrency, and error paths end to end. Hunt for concrete bugs, omissions, invalid
assumptions, unhandled edges, security/reliability failures, inconsistencies, duplication, dead code, stale docs, and
needless complexity. Also inspect evidenced problems in performance, dependencies, data formats and extensions,
configuration, observability, accessibility, agent context, naming, and directory structure. Style preferences and
unverified hunches are not findings.

At applicable external and persisted-data seams, inspect validation; domain precision and units; deterministic ordering
and deduplication; idempotency and repeat-run behavior; atomicity and interruption safety; retry and pagination
completeness; bounded concurrency, cancellation, and resource cleanup; and secret, log, path, temporary-file, and
command safety.

Confirm each issue before editing. Fix the smallest root cause when intent is clear and verification is available; write
a missing test or fix an in-scope residual risk whenever the fix can be verified, rather than reporting it. Mark
`reported` only for real decisions: intent is ambiguous, a safe fix would change a public contract for consumers outside
the repository, or no verification is available. Give every `reported` finding a recommended fix and its blast radius.
Do not add speculative features, broad refactors, or cosmetic churn.

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

### Tests

Review tests as code under the same first-principles questions, aiming for a smaller suite that catches the same or more
defects. For each test, identify the required behavior it protects, then:

- Delete tests that are stale or unhelpful: they target removed or renamed behavior kept alive only by mocks or
  fixtures; cannot fail (no meaningful assertion, asserting a mock's own return value, tautologies); pin incidental
  implementation details or call sequences no requirement depends on; restate language, framework, or dependency
  behavior; or are skipped or commented out with no live reason.
- Merge tests that effectively prove the same thing: identical paths differing only in inputs become one table-driven or
  parameterized test; a narrower test fully subsumed by another test's assertions goes; duplicates across files collapse
  into the owning suite.

Before deleting or merging, confirm the protected behavior is obsolete or still covered by a named retained test; use
coverage output, or a temporary targeted break of the code, when the overlap is not obvious. A merge keeps every
distinct assertion, input, and diagnosable failure message. Keep regression tests for fixed bugs unless another test
demonstrably covers the same case. Never delete or skip a failing or flaky test to get green: fix the cause or mark it
`reported`. Run the affected suites before and after, and record the test-count delta.

### Comments

Compare every comment with the code, callers, and history it describes. Fix only clear `STALE` (describes behavior the
code no longer has), `ORPHANED` (names a missing symbol, path, flag, or concept), `MISLEADING` (materially suggests
different behavior), or `REDUNDANT` (narrates self-explanatory code without intent, constraint, or context) comments.
Rewrite when the correct claim is proven; otherwise remove. Never change executable code merely to make a comment true,
and leave useful rationale and imperfect-but-accurate wording alone.

Treat behavior-bearing comments as code: compiler and tool directives (`//go:*`, build constraints, cgo preambles,
`go:embed`, `@ts-expect-error`, lint suppressions, coverage pragmas), license headers, public API docs, and concurrency,
ownership, or safety contracts. Edit them only when the tooling semantics are proven and validated by the relevant
tooling; otherwise mark them `reported`.

## Verify and Report

Run the narrowest check proving each fix, including every discovered typecheck, lint, and format/import-order gate
applicable to its changed files, then aggregate checks scoped to changed files. Reinspect affected paths and repeat
until a pass finds no new evidenced issue. Before declaring completion, revisit the first-principles questions against
the result, including the sweep's own additions; passing checks alone does not justify unnecessary complexity. During a
supplied deadline's validation window, reconcile owned edits and run the aggregate format, lint, type, test, build, and
invariant checks justified by the final changed-file union. Compare final results with the recorded baseline. Audit
coverage, fixes, and checks against tool output before claiming completion.

Lead with
`### ✅ Sweep ledger complete — <accounted>/<mapped> files accounted (<inspected> inspected, <excluded> excluded)` only
when helper `complete` is true; otherwise use `### ⛔ Sweep incomplete`. Summarize fixed, reported, excluded, and check
counts, plus deleted and merged tests with the test-count delta and fixed comments when non-zero. Include a compact
`Check | Baseline | Final` table, changed artifacts and verified fixes, and subagent results. When non-empty, also
include reverted experiments with the failed evidence, each `reported` finding with its evidence, recommended fix, and
blast radius, and the overnight backlog when applicable. On `### ⛔ Sweep incomplete`, name the ledger path so the next
session can resume from `pending`. Do not dump the scratch ledger's contents, unrelated pre-existing changes, or bulk
data; include task-relevant evidence when it materially supports the report.

In an interactive (non-overnight) run with any `reported` findings, end with one decision question listing them: fix all
as recommended, pick specific items, or leave them reported. Treat invocation wording that already authorizes fixing
(for example "fix any/all problems" or "I will follow your judgement") as that approval up front and skip the question,
except for destructive actions, external writes, and purchases, which still need explicit confirmation regardless of
invocation wording. On approval, run a fix wave over the approved items under the same sweep rules — confirm, fix,
verify, update the ledger, and rerun `$code-polish` — then report the updated ledger and check results. During an
autonomous overnight run, skip the question and leave `reported` findings in the overnight backlog instead.

Completion requires every mapped path accounted for, every finding fixed and verified, fixed by an approved fix wave, or
reported with evidence and a pending decision, and every relevant check passing or its failure attributed.
