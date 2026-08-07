---
argument-hint:
  "[--skill NAME] [--root PATH | --portfolio-root PATH] [--format text|json|dot] [--include-catalog-sources]
  [--include-self] [--include-snippets] [--show-skipped]"
coordination: exempt
disable-model-invocation: false
name: skill-map
user-invocable: true
description:
  "Use to find agent skill installs, repository skill portfolios, duplicate skills, cross-dependencies, invocations, and
  cross-references across the local machine."
---

# Skill Map

This skill is coordination-exempt: skip the ai-coord gate (`git status` / `ai-coord status` / `ai-coord start`) for this
skill's own work.

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

1. Resolve the skill directory, then run the helper from that directory:

   ```sh
   uv run scripts/skill-map.py "$ARGUMENTS"
   ```

2. If no arguments were provided, run the default machine scan:

   ```sh
   uv run scripts/skill-map.py
   ```

3. Use `--format json` when another command or agent will consume the result.

4. Use `--portfolio-root <repo> --format json` when comparing repository skills with user-installed Codex and Claude
   Code exposures. Do not add broader home roots.

5. Use `--format dot` when the user asks for a graph, Graphviz input, or dependency visualization.

6. Use `--include-snippets` when exact matching lines materially improve the result, including when the user asks to see
   them.

7. Read [references/ignore-policy.md](references/ignore-policy.md) only when explaining, auditing, or changing the
   ignore policy.

## User-Facing Output

Keep JSON and DOT byte-valid and undecorated. For human output, lead with
`### 🗺 Skill Map — <skills> skills · <duplicates> duplicates · <unresolved> unresolved` and a compact
skills/dependencies/duplicates/unresolved summary table. Always state the effective roots and material exclusions; for
the default broad scan, explicitly say that standard agent homes and catalog source checkouts were excluded. Make a
missing explicit `--skill` filter a visible `⚠️ Not found` result rather than a clean-looking empty map. Use section
labels sparingly and keep snippets, local paths, exact edges, commands, and diagnostics undecorated.

## Output Semantics

- `dependency`: a skill file or support file references another discovered skill.
- `external-reference`: a non-skill file references a discovered skill.
- `duplicate-install`: multiple discovered `SKILL.md` files declare or resolve to the same skill name.
- `unresolved-like-reference`: explicit `$kebab-name` or `/kebab-name` tokens that do not match a discovered skill.

Portfolio JSON preserves those fields and adds:

- `portfolio.repository_root` and present/missing user roots, including the client exposed by each root.
- Per-skill lexical `exposure_path` beside resolved `realpath`; `directory_name`; repository/user `location`;
  install/catalog `kind`; and applicable `clients`.
- `is_symlink` and the lexical `symlink_target` for recognized skill-directory symlinks. Exposures that resolve to one
  real directory remain separate skill entries, while duplicate installs still require distinct real directories.
- `skill_sha256` for `SKILL.md` bytes and `tree_sha256` for the complete filtered skill tree. Tree hashes cover sorted
  relative paths, entry types, regular-file executable bits, streamed file bytes, and un-followed symlink targets.

The automatically selected user roots behave like explicit roots: the broad-home ignore policy does not suppress an
explicit `~/.agents/skills` or `~/.claude/skills` root. Portfolio traversal follows symlinks only when the symlink is a
direct child of a recognized repository or user skill root; other repository symlinks remain untraversed.

## Related Skills

- `skill-map` only locates and cross-references skills; it does not validate them. To audit a catalog or installed root
  for metadata and doc-link issues, use the `skill-doctor` skill when it is installed.
- To evaluate the mapped repository-centered portfolio for conflicts, drift, consolidation, or missing workflows, use
  the manually invoked `skill-harmonization` skill.

## Guard Rails

- Do not search transcript or backup directories manually after the helper excludes them unless the user explicitly
  requests transcript/history analysis.
- Do not broaden home-directory scans into macOS protected paths such as `~/Library` or `~/.Trash`; pass narrower
  explicit roots instead when a broad scan needs more coverage.
- Treat local skill catalog source checkouts as false positives during broad machine scans; pass them explicitly as
  `--root` when auditing catalog contents.
- Keep reports high-signal: include snippets only when exact evidence materially helps or the user explicitly requests
  it. Before copying output into a public or third-party artifact, perform an external-disclosure review.
- Prefer adding ignore rules in the helper and documenting the rationale in `references/ignore-policy.md` instead of ad
  hoc shell filters.
