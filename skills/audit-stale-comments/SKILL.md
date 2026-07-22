---
agent: Plan
argument-hint: "[path ...]"
context: fork
name: audit-stale-comments
user-invocable: true
description:
  Audit JavaScript, TypeScript, and Go comments for verified stale, orphaned, misleading, or redundant claims.
---

# Audit Stale Comments

Audit source comments against the code they describe. Report only confirmed mismatches and noise; do not edit files
unless the user separately asks for fixes.

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

## Verification

Aggregate and deduplicate candidate findings, then reopen every reported location. Confirm its current line number and
the implementation evidence that makes the comment wrong or unnecessary. Drop findings that cannot be verified. A clean
result is valid and should not be padded with low-confidence observations.

## Report

Return:

```markdown
### 🔎 Stale-comments audit — <finding count or clean>

| Scope   | Files analyzed | Confirmed | Not reviewed    |
| ------- | -------------- | --------- | --------------- |
| <paths> | <count>        | <count>   | <count or none> |

### Findings

#### STALE

- path/to/file.ts:42 — <comment claim, contradictory evidence, and why it matters>

#### ORPHANED

#### MISLEADING

#### REDUNDANT

### ⚠️ Not reviewed

- <path and reason>
```

Omit empty category and not-reviewed sections. When no findings remain and every file was reviewed, lead with
`### ✅ Clean — no confirmed stale, orphaned, misleading, or redundant comments`. If files were not reviewed, lead with
`### ⚠️ Review incomplete — no confirmed findings in reviewed scope` instead. When fixes were requested and applied,
lead with `### ✅ Comments fixed — <count>`, then add `### 📦 Changed` and `### 🧪 Verification`; do not collapse a
write receipt into `Clean`. Keep classifier tokens, paths, line numbers, directives, and quoted comment text exact and
undecorated. Completion requires the exact scope, analyzed-file count, confirmed findings with current lines and
evidence, and any files that could not be reviewed.
