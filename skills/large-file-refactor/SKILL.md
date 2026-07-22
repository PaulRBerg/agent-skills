---
argument-hint: "[path] [--include-generated]"
disable-model-invocation: true
name: large-file-refactor
user-invocable: true
description:
  Discover large source-file refactor candidates and propose cohesion- and risk-driven split plans using available
  semantic tooling.
---

# Large File Refactor

Use LOC thresholds to discover candidates, then decide whether a split is justified by cohesion, coupling, ownership,
and change risk. Test files use a relaxed 2000 LOC discovery threshold.

## Arguments

- `path`: Optional file or directory to scan. Default: current working directory.
- `--include-generated`: Include generated, vendored, dependency, and build-output paths that are skipped by default.

## Workflow

1. Resolve the skill directory, then run the helper from that directory:

   ```sh
   uv run scripts/large-file-refactor.py "$ARGUMENTS"
   ```

2. If no arguments were provided, run the default scan:

   ```sh
   uv run scripts/large-file-refactor.py
   ```

3. Preserve the helper's Markdown table as the exhaustive report. Do not omit matching rows, even when the refactor plan
   only covers a subset.

4. If the helper reports no threshold matches, stop after the report. A match is a candidate, not proof that the file
   should be split.

5. Draft a refactor plan for the 3 largest files only, unless the user explicitly requested another count.

6. For each candidate, rank split value by mixed responsibilities, change frequency/risk, coupling, and testability. Use
   whichever semantic symbol/reference tooling is available; prefer Serena when installed:

   - Inspect symbol overviews, references, imports, and relevant history.
   - Use the evidence to choose extraction boundaries, target module names, migration order, and test coverage.

7. Do not implement the refactor unless the user separately asks for execution.

## Refactor Plan Format

For each selected file, include:

- Current role: the file's apparent responsibility and why line count is a symptom.
- Semantic pass: the exact symbol/reference/history inspection to run before moving code.
- Split proposal: 2-5 target modules or files with responsibilities.
- Migration order: small, reviewable steps that preserve public behavior.
- Verification: narrow tests, type checks, builds, or smoke checks that prove the split.

Lead each plan with one explicit verdict: `### ✂ Split justified`, `### 🧱 Keep intact`, or
`### ⚠️ Generated — change the generator`. Use a compact evidence table for repeated criteria. When proposing a split,
show the source and target modules as a tree only when it clarifies ownership.

If a selected file is generated or vendored because `--include-generated` was used, plan against the generator, schema,
or upstream source instead of hand-splitting generated output.

## Guard Rails

- Treat the table as source of truth for size ranking only; rank refactor priority separately.
- Keep the plan cohesive; do not split solely to reduce line count.
- Prefer existing project module boundaries and naming conventions.
- Call out when the helper used its portable LOC estimate instead of `tokei`.

## Completion

Complete with the exhaustive threshold report plus evidence-ranked plans only for candidates whose cohesion or change
risk justifies a split. Lead with `### 🔎 Large-file scan — <candidate count>`, state when a large file should remain
intact and why, and surface the portable-LOC fallback as `⚠️ Approximate counts` when used. Keep the helper's exhaustive
table, paths, LOC values, and commands exact and undecorated.
