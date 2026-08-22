---
argument-hint:
  "[--skill NAME] [--root PATH | --portfolio-root PATH] [--format text|json|dot] [--include-catalog-sources]
  [--include-self] [--include-snippets] [--show-skipped]"
compatibility: Requires ai-skillet 0.1.0+.
coordination: exempt
name: skill-map
description:
  "Use to find agent skill installs, repository skill portfolios, duplicate skills, cross-dependencies, invocations, and
  cross-references across the local machine."
---

# Skill Map

This skill is coordination-exempt: skip the ai-coord gate for its declared work.

Find skill installs and references across local files without scanning macOS protected home paths or obvious transcript,
cache, dependency, and backup noise.

## Arguments

- `--root PATH`: Scan this root. Repeatable. Default: `~`.
- `--portfolio-root PATH`: Resolve `PATH` to its Git root, then scan that repository plus existing `~/.agents/skills`
  and `~/.claude/skills`. Mutually exclusive with `--root`.
- `--skill NAME`: Restrict the report to one skill name. Repeatable.
- `--format text|json|dot`: Select report format. Default: `text`.
- `--include-catalog-sources`: Include known local source checkouts such as `~/projects/agent-skills`,
  `~/sablier/sablier-skills`, and `~/sablier/agent-skills` during broad scans. Explicit `--root` values inside those
  trees are always scanned.
- `--include-self`: Include self-references in dependency output.
- `--include-snippets`: Include matched reference text when exact lines materially improve the result. Default output
  omits snippets for concise, high-signal reports.
- `--show-skipped`: Include ignored path summaries in text or JSON output.

## Workflow

1. Require `ai-skillet` 0.1.0 or newer on `PATH`, then run:

   ```sh
   ai-skillet map "$ARGUMENTS"
   ```

2. Use `--format json` when another command or agent will consume the result.

3. Use `--portfolio-root <repo> --format json` when comparing repository skills with user-installed Codex and Claude
   Code exposures. Do not add broader home roots.

4. Use `--format dot` when the user asks for a graph, Graphviz input, or dependency visualization.

5. Use `--include-snippets` when exact matching lines materially improve the result, including when the user asks to see
   them.

6. Read [references/ignore-policy.md](references/ignore-policy.md) only when explaining, auditing, or changing the
   ignore policy.

## User-Facing Output

Keep JSON and DOT byte-valid and undecorated. For human output, lead with
`### 🗺 Skill Map — <skills> skills · <duplicates> duplicates · <unresolved> unresolved` and a compact
skills/dependencies/duplicates/unresolved summary table. Always state the effective roots and material exclusions; for
the default broad scan, explicitly say that standard agent homes and catalog source checkouts were excluded. Make a
missing explicit `--skill` filter a visible `⚠️ Not found` result rather than a clean-looking empty map. Use section
labels sparingly and keep snippets, local paths, exact edges, commands, and diagnostics undecorated.

## Output Semantics

`ai-skillet map` emits schema version 1. Text is human-readable; JSON is structured for consumers; DOT is Graphviz
input.

- Every edge includes `type`, `provenance`, `identifier`, `source`, `target`, `path`, and `line`. Dependency evidence
  uses `provenance: declared` or `inferred`; declared and inferred evidence remain independent records even when they
  describe the same source and target. Declared identifiers preserve their bare or `ORG/REPO#SKILL` form.
- `external-reference` records references outside a discovered skill, and `unresolved-like-reference` records explicit
  `$kebab-name` or `/kebab-name` tokens that did not match a discovered skill.
- `counts` separates declared dependencies, inferred dependencies, external references, duplicates, and unresolved
  references. A missing `--skill` filter writes a warning to stderr and returns an empty filtered report.
- Skill filters match declaration sources and target skill names, including the name after `#` for external identifiers.
  Invalid declaration fields stop the mapper with a path-specific error.
- Every skill record includes `skill_sha256` and `tree_sha256`. The tree hash covers sorted relative paths, entry types,
  regular-file executable bits, streamed file bytes, and un-followed symlink targets.

Portfolio JSON additionally includes:

- `portfolio.repository_root` and present/missing user roots, including the client exposed by each root.
- Per-skill lexical `exposure_path` beside resolved `realpath`; `directory_name`; repository/user `location`;
  install/catalog `kind`; and applicable `clients`.
- `is_symlink` and the lexical `symlink_target` for recognized skill-directory symlinks. Exposures that resolve to one
  real directory remain separate skill entries, while duplicate installs still require distinct real directories. The
  automatically selected user roots behave like explicit roots: the broad-home ignore policy does not suppress an
  explicit `~/.agents/skills` or `~/.claude/skills` root. Portfolio traversal follows symlinks only when the symlink is
  a direct child of a recognized repository or user skill root; other repository symlinks remain untraversed.

Use `--show-skipped` to include configured ignored directories, files, protected home paths, caches, and catalog-source
exclusions in text or JSON output.

## Related Skills

- `skill-map` only locates and cross-references skills; it does not validate them. To audit a catalog or installed root
  for metadata and doc-link issues, use the `skill-doctor` skill when it is installed.
- To evaluate the mapped repository-centered portfolio for conflicts, drift, consolidation, or missing workflows, use
  the manually invoked `skill-harmonization` skill.

## Guard Rails

- Do not search transcript or backup directories manually after `ai-skillet map` excludes them unless the user
  explicitly requests transcript/history analysis.
- Do not broaden home-directory scans into macOS protected paths such as `~/Library` or `~/.Trash`; pass narrower
  explicit roots instead when a broad scan needs more coverage.
- Treat local skill catalog source checkouts as false positives during broad machine scans; pass them explicitly as
  `--root` when auditing catalog contents.
- Keep reports high-signal: include snippets only when exact evidence materially helps or the user explicitly requests
  it. Before copying output into a public or third-party artifact, perform an external-disclosure review.
- Prefer maintaining ignore rules in ai-skillet and documenting the rationale in `references/ignore-policy.md` instead
  of ad hoc shell filters.
