---
argument-hint: "[path] [--include-generated]"
disable-model-invocation: true
name: large-file-refactor
user-invocable: true
description: Scan for source files over 1000 LOC (2000 LOC for test files) and propose Serena-assisted split refactor plans.
---

# Large File Refactor

Scan a codebase for source files over 1000 LOC, report every match, and propose a focused split plan for the largest files. Test files (matched by `test`/`tests`/`spec`/`specs` directories or filename tokens) use a relaxed 2000 LOC threshold instead.

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

3. Preserve the helper's Markdown table as the exhaustive report. Do not omit matching rows, even when the refactor plan only covers a subset.

4. If the helper reports no files over 1000 LOC, stop after the report.

5. Draft a refactor plan for the 3 largest files only, unless the user explicitly requested another count.

6. For each planned split, mention a Serena MCP pass before any code movement:

   - Activate or target the scanned project with Serena.
   - Inspect symbol overviews and references for the large file.
   - Use the Serena findings to choose extraction boundaries, target module names, migration order, and test coverage.

7. Do not implement the refactor unless the user separately asks for execution.

## Refactor Plan Format

For each selected file, include:

- Current role: the file's apparent responsibility and why line count is a symptom.
- Serena MCP pass: the exact semantic inspection to run before moving code.
- Split proposal: 2-5 target modules or files with responsibilities.
- Migration order: small, reviewable steps that preserve public behavior.
- Verification: narrow tests, type checks, builds, or smoke checks that prove the split.

If a selected file is generated or vendored because `--include-generated` was used, plan against the generator, schema, or upstream source instead of hand-splitting generated output.

## Guard Rails

- Treat the table as source of truth for size ranking.
- Keep the plan cohesive; do not split solely to reduce line count.
- Prefer existing project module boundaries and naming conventions.
- Call out when the helper used its portable LOC estimate instead of `tokei`.
