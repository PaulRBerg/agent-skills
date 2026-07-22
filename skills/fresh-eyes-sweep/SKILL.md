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

Relentlessly inspect the repository for anything wrong, as if reading it for the first time, then fix every safe,
evidenced issue. Relentless means the sweep does not wind down early: keep going until every mapped file is accounted
for and a full pass turns up nothing new. A verified no-op is a valid result — but only after that.

## Setup

1. Require a Git repository and read the applicable repository instructions.
2. Default to the whole repository; with path arguments, sweep only those subtrees plus the shared configuration and
   instructions they depend on.
3. Map every tracked and non-ignored file in scope. Classify generated, vendored, minified, binary, and bulk-data
   artifacts; validate those through their generator, schema, or invariants instead of line-by-line reading.
4. Discover the repository's checks: build, test, lint, typecheck, format, and codegen verification.
5. Record existing worktree changes before touching anything. Pre-existing edits stay untouched: never revert, absorb,
   or commit them, and never report them as findings.
6. Create a coverage ledger in a scratch file listing every mapped file with a status: `pending`, `inspected`, `fixed`,
   `reported`, or `excluded` with the reason. Keep it current; never claim coverage the ledger does not show.

## Inspect

Work through the ledger in coherent slices — a package, subsystem, or directory with its tests, configuration, and
documentation together — so cross-file relationships are visible. Delegate independent slices to parallel read-only
subagents when the environment supports them, and keep inspecting while they run; findings return to the main context,
which owns all edits.

Trace important control, data, and error paths end to end rather than skimming file by file. Hunt for evidenced
mistakes: bugs, omissions, invalid assumptions, unhandled edge cases, security and reliability failures, cross-file
inconsistencies, duplication, dead code, misleading or stale documentation, and needless complexity. Style preferences
and unverified hunches are not findings.

## Fix

Confirm each suspected issue with concrete evidence before changing anything. Fix it when intent is clear and the result
can be verified: prefer the smallest root-cause change and add a focused regression test when it locks in the fix.
Report instead of fixing when the change would alter a public contract, when intent is ambiguous, or when the fix cannot
be verified — record the evidence and the blocker. No speculative features, drive-by refactors, or cosmetic churn.

## Verify

Run the narrowest check that proves each fix, then the relevant aggregate checks scoped to changed files. Reinspect
paths affected by fixes and relentlessly repeat the sweep until a pass surfaces no new evidenced issue; finding issues
is a reason to look harder, not to stop. Before reporting, audit every claim — coverage, fixes, and passing checks —
against tool output from this session; anything not verified is reported as unverified.

## Report

Lead with the outcome: what was fixed, what was found but not fixed, or that the sweep was clean. Then give changed
files, the exact validation commands with their results, unresolved findings with evidence and blocker, ledger
exclusions with reasons, and residual risk. Completion requires every mapped file accounted for in the ledger, every
finding fixed and verified or reported with evidence, and every relevant check passing or its failure explained.
