---
argument-hint: "[path ...]"
name: stale-comments-audit
user-invocable: true
description:
  Audit and fix JavaScript, TypeScript, and Go comments that are verified as stale, orphaned, misleading, or redundant.
---

# Stale Comments Audit

Audit source comments against the code they describe, then fix every confirmed mismatch or source of noise in the same
run. Limit edits to comments; never change executable code merely to make a comment true.

## Scope

Accept zero or more file or directory paths. With no arguments, use the current repository root. Resolve every path from
the current working directory and report any path that cannot be read.

Discover JavaScript, JSX, TypeScript, TSX, and Go source files. By default exclude:

- tests, testdata, fixtures, snapshots, and stories;
- generated, vendor, dependency, coverage, cache, and build-output directories;
- minified JavaScript and generated Go files.

An explicitly named test file or test directory overrides only the test exclusion. Generated, vendored, minified, and
build-output files remain excluded unless the user explicitly requests them.

Use repository-aware discovery when available (`git ls-files` or `rg --files`) and preserve the user's path order. Do
not impose an arbitrary result cap.

## Analysis

Split discovered files into non-overlapping batches of about ten. Delegate independent batches to read-only subagents
when supported; otherwise inspect them directly. Continue useful work while delegated batches run.

For every line, block, and documentation comment, compare the claim with adjacent code, referenced symbols, callers,
configuration, and relevant repository history when current code alone is inconclusive. Classify only clear findings:

- `STALE`: describes behavior the code no longer has;
- `ORPHANED`: references a symbol, path, flag, or concept that no longer exists;
- `MISLEADING`: materially suggests behavior different from the implementation;
- `REDUNDANT`: merely narrates self-explanatory code and carries no intent, constraint, or context.

Do not report style preferences, imperfect wording, useful rationale, or uncertainty as findings.

### Behavior-Bearing Go Comments

Treat these as part of program behavior or required documentation, not ordinary prose:

- compiler and tool directives, including `//go:*`, `//line`, and `//nolint`;
- build constraints and their required placement;
- cgo preambles attached to `import "C"`;
- `go:embed` patterns and the declarations they populate;
- package comments and exported-symbol documentation;
- concurrency, ownership, locking, lifetime, and safety contracts.

Verify these against Go syntax, symbol use, and tooling semantics. Never mark one redundant merely because the adjacent
declaration is obvious.

## Fix and Verify

Aggregate and deduplicate candidate findings, then reopen every candidate location. Confirm its current line number and
the implementation evidence that makes the comment wrong or unnecessary. Drop findings that cannot be verified. A clean
result is valid and should not be padded with low-confidence observations.

Apply the smallest comment-only fix for every confirmed finding:

- `STALE` and `MISLEADING`: rewrite the comment when the intended claim is proven; otherwise remove it when the code
  needs no replacement explanation.
- `ORPHANED`: update the reference only when its replacement is verified; otherwise remove the obsolete reference or
  comment.
- `REDUNDANT`: remove the comment without reformatting unrelated code.

For behavior-bearing Go comments, edit only when the intended compiler, tool, documentation, or concurrency semantics
are proven. If the safe correction is ambiguous, leave the comment unchanged and report it as not fixed. Never delegate
writes; apply all fixes in the main execution context after consolidating the read-only analysis.

Reopen every changed location and confirm the resulting comment agrees with the implementation. Run the narrowest
formatter, lint, typecheck, test, build, or Go tooling that proves the edited files remain valid. Behavior-bearing Go
comment changes require the relevant Go validation; ordinary prose-only changes do not require unrelated broad tests.

## Report

Return:

```markdown
### ✅ Comments fixed — <fixed count>

| Scope   | Files analyzed | Confirmed | Fixed   | Not fixed       | Not reviewed    |
| ------- | -------------- | --------- | ------- | --------------- | --------------- |
| <paths> | <count>        | <count>   | <count> | <count or none> | <count or none> |

### 🔧 Fixes

#### STALE

- path/to/file.ts:42 — <previous claim, contradictory evidence, and applied fix>

#### ORPHANED

#### MISLEADING

#### REDUNDANT

### 🧪 Verification

- `<command>` — <result>

### ⚠️ Not fixed

- path/to/file.go:42 — <classification, evidence, and why a safe correction is ambiguous>

### ⚠️ Not reviewed

- <path and reason>
```

Omit empty category, not-fixed, and not-reviewed sections. When no findings remain and every file was reviewed, lead
with `### ✅ Clean — no confirmed stale, orphaned, misleading, or redundant comments`. If files were not reviewed, lead
with `### ⚠️ Review incomplete — no confirmed findings in reviewed scope` instead. When any confirmed finding could not
be fixed safely, lead with `### ⚠️ Comments partially fixed — <fixed count> fixed, <not-fixed count> not fixed`. Keep
classifier tokens, paths, line numbers, commands, directives, and quoted comment text exact and undecorated. Completion
requires the exact scope, analyzed-file count, confirmed and fixed counts, traceable fixes, validation evidence, and any
files or findings that could not be completed.
