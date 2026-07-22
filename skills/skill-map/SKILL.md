---
argument-hint:
  "[--skill NAME] [--root PATH] [--format text|json|dot] [--include-catalog-sources] [--include-self]
  [--include-snippets] [--show-skipped]"
disable-model-invocation: false
name: skill-map
user-invocable: true
description:
  "Use to find agent skill installs, duplicate skills, cross-dependencies, invocations, and cross-references across the
  local machine."
---

# Skill Map

Find skill installs and references across local files without scanning macOS protected home paths or obvious transcript,
cache, dependency, and backup noise.

## Arguments

- `--root PATH`: Scan this root. Repeatable. Default: `~`.
- `--skill NAME`: Restrict the report to one skill name. Repeatable.
- `--format text|json|dot`: Select report format. Default: `text`.
- `--include-catalog-sources`: Include known local source checkouts such as `~/projects/agent-skills`,
  `~/sablier/sablier-skills`, and `~/sablier/agent-skills` during broad scans. Explicit `--root` values inside those
  trees are always scanned.
- `--include-self`: Include self-references in dependency output.
- `--include-snippets`: Include matched line text. Default output omits snippets to avoid leaking transcript or
  secret-adjacent content.
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

4. Use `--format dot` when the user asks for a graph, Graphviz input, or dependency visualization.

5. Use `--include-snippets` only when the user asks to see exact matching lines.

6. Read [references/ignore-policy.md](references/ignore-policy.md) only when explaining, auditing, or changing the
   ignore policy.

## User-Facing Output

Keep JSON and DOT byte-valid and undecorated. For human output, lead with
`### 🗺 Skill Map — <skills> skills · <duplicates> duplicates · <unresolved> unresolved` and a compact
skills/dependencies/duplicates/unresolved summary table. Always state the effective roots and material exclusions; for
the default broad scan, explicitly say that standard agent homes and catalog source checkouts were excluded. Make a
missing explicit `--skill` filter a visible `⚠️ Not found` result rather than a clean-looking empty map. Use section
labels sparingly and keep snippets, private paths, exact edges, commands, and diagnostics undecorated.

## Output Semantics

- `dependency`: a skill file or support file references another discovered skill.
- `external-reference`: a non-skill file references a discovered skill.
- `duplicate-install`: multiple discovered `SKILL.md` files declare or resolve to the same skill name.
- `unresolved-like-reference`: explicit `$kebab-name` or `/kebab-name` tokens that do not match a discovered skill.

## Related Skills

- `skill-map` only locates and cross-references skills; it does not validate them. To audit a catalog or installed root
  for metadata and doc-link issues, use the `skill-doctor` skill when it is installed.

## Guard Rails

- Do not search transcript or backup directories manually after the helper excludes them unless the user explicitly
  requests transcript/history analysis.
- Do not broaden home-directory scans into macOS protected paths such as `~/Library` or `~/.Trash`; pass narrower
  explicit roots instead when a broad scan needs more coverage.
- Treat local skill catalog source checkouts as false positives during broad machine scans; pass them explicitly as
  `--root` when auditing catalog contents.
- Treat output paths as local private context. Do not paste snippets unless `--include-snippets` was intentionally used.
- Prefer adding ignore rules in the helper and documenting the rationale in `references/ignore-policy.md` instead of ad
  hoc shell filters.
