---
argument-hint: "[--root PATH ...] [--format text|json] [--fix-safe] [--dependencies-only]"
compatibility: Requires ai-skillet 0.1.0+.
disable-model-invocation: false
name: skill-doctor
user-invocable: true
description:
  "Use to audit Agent Skills catalogs or installed skill roots for metadata and doc-link issues; optionally apply
  conservative --fix-safe repairs."
---

# Skill Doctor

Audit local Agent Skills catalogs and installed skill roots, then apply only narrow metadata repairs when requested.

## Arguments

- `--root PATH`: Scan this catalog or installed skill root. Repeatable. Default: current working directory.
- `--format text|json`: Select report format. Default: `text`.
- `--fix-safe`: Create missing `agents/openai.yaml` files or update mismatched `policy.allow_implicit_invocation`.
- `--dependencies-only`: Report only malformed or unresolved `skill-dependencies` declarations. Do not combine with
  `--fix-safe`.

## Workflow

1. Require `ai-skillet` 0.1.0 or newer on `PATH`, then run:

   ```sh
   ai-skillet doctor "$ARGUMENTS"
   ```

2. Use JSON when another command or agent will consume the result:

   ```sh
   ai-skillet doctor --root . --format json
   ```

3. Run safe fixes only after reading the findings:

   ```sh
   ai-skillet doctor --root . --fix-safe
   ```

4. Re-run without `--fix-safe` after any manual edits.

For a dependency-only catalog gate, run:

```sh
ai-skillet doctor --root . --dependencies-only
```

## Findings

- Treat `error` findings as catalog defects that should block publishing or syncing.
- Treat `warning` findings as review-required catalog hygiene issues.
- JSON output uses schema version 1 with structured roots, counts, findings, and safe-fix records. Each finding carries
  its code, severity, path, line when known, fixability, and message.
- Prompt-hygiene warnings are advisory and never auto-fix: stale model pins, oversized unconditional Markdown
  references, conflicting requirement/prohibition language, and missing completion evidence.
- Coordination-exemption errors are report-only: `coordination: exempt` must be paired with the catalog's canonical
  `coordination-exempt` body sentence, and neither side is repaired by `--fix-safe`.
- Dependency errors reject non-array, empty, non-string, duplicate, malformed, incorrectly ordered, self-referential, or
  unresolved local declarations. External `ORG/REPO#SKILL` identifiers are shape-checked without network access.
- Metadata, OpenAI policy, coordination, resource, README, prompt-hygiene, and CLI-version checks remain available
  outside `--dependencies-only`.
- Use `path` and `line` from JSON output for precise follow-up edits.

## Safe Fix Policy

`--fix-safe` may only:

- Create a missing `agents/openai.yaml` with `policy.allow_implicit_invocation` derived from `SKILL.md`.
- Update an existing `allow_implicit_invocation` boolean when it disagrees with `disable-model-invocation`.

Each permitted repair is staged and atomically renamed into place, preserving the target's permissions for updates. A
failed safe fix exits 3 without partially rewriting its target.

Do not use `ai-skillet doctor` to rewrite frontmatter order, descriptions, README rows, `references/version.txt`, or
relative links. Make those edits manually and verify with a fresh audit.

## Related Skills

- `skill-doctor` only audits the roots you pass; it does not search for them. To locate skill installs, duplicates, and
  cross-references across the machine, use `ai-skillet map`.

## Exit Codes

- `0`: Clean, or all requested safe fixes succeeded and no findings remain.
- `1`: The audit completed and findings remain; report them as review work, not as an operational crash.
- `2`: Invalid arguments or unreadable environment.
- `3`: A requested safe fix failed.

## User-Facing Output

Keep `--format json` byte-valid and undecorated. For human output, lead with `### 🩺 Skill Doctor — ✅ clean`,
`### 🩺 Skill Doctor — ⚠️ review required`, or `### 🩺 Skill Doctor — ⛔ blocked` for exit 2/3, then show roots and
error/warning/fix counts in a compact table. List safe fixes separately from remaining findings. For review-required or
blocked outcomes, end with the smallest manual next action; for a clean result, stop after the summary. Keep paths, line
numbers, codes, raw findings, commands, and diagnostics exact.
