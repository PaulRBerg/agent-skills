---
argument-hint:
  <polish|create> [path] [target ...] [--root-only] [--preserve] [--minimal] [--thorough|--full] [--dry-run] [--force]
compatibility:
  Requires curl and a writable user cache directory; network populates or refreshes the GPT-6 Astra and Claude Opus 5.5
  prompting guides.
name: agents-brain
skill-dependencies:
  - skill-writing
description:
  "Create or polish repo agent context: README.md, AGENTS.md/CLAUDE.md, project-installed and source-catalog skills, and
  other Markdown context docs."
---

# Agents Brain

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Create or polish repo-local context as one coherent system: human-facing README.md files, agent-facing AGENTS.md files
(with companion CLAUDE.md symlinks only for pre-native Claude Code, per Claude Code Compatibility below), existing
project-installed skills under `.agents/skills`, eligible source-catalog skills under `skills/<name>/`, and context docs
— any other Markdown files, under any name or directory, whose content is durable guidance for agents or humans, such as
conventions, command catalogs, data-format rules, workflow runbooks, and reference material.

Success means every selected target is grounded in repository evidence, respects its audience and scope, spends agent
context only on guidance that changes behavior, and passes the narrowest repository-defined validation. Stop after
reporting completed or planned changes, validation, and any blockers.

## Model and Context Optimization

Optimize skills and other agent-facing context for GPT-6 Astra and Claude Opus 5.5 while preserving README.md as clear
human-facing documentation. Before complex, long-running, multi-tool, or orchestration-heavy context work, resolve
`scripts/fetch-guidance.sh` relative to this skill directory, run it once for `gpt-6-astra` and once for
`claude-opus-5-5`, and read both returned files completely. The helper retrieves the official
[GPT-6 Astra prompting guidance](https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra#prompting-best-practices)
and
[Claude Opus 5.5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)
because their recommendations may evolve. Simple context work does not require either guide.

Accept an integrity-valid `cached` guide for 24 hours. Use `--refresh` for explicitly latest or change-sensitive work,
materially disputed guidance, or conflicts with observed model behavior. Interpret helper diagnostics precisely:

- `cached` reused a guide validated no more than 24 hours ago without network access.
- `revalidated` refreshed validation metadata after a successful conditional `304` response.
- `fetched` atomically replaced the cache with integrity-valid content from the pinned official URL.
- `stale` reused a guide validated no more than seven days ago after live retrieval failed. Proceed only after reading
  it and disclose the validation timestamp and retrieval failure under open issues and caveats.

Forced refreshes, expired entries, integrity failures, and unexpected redirects fail closed. If either required guide
cannot be returned, stop qualifying work before writing instead of substituting memory or another source. Never create
the cache in a repository or skill installation.

Keep only content that changes a decision, prevents an evidenced mistake, or supplies a non-discoverable constraint.
State each meaning once at the narrowest reliable load scope, except where independently installed artifacts need to
stay self-contained. Preserve authority, safety, material exceptions, semantic success criteria, and exact
machine-consumed text. Documentation-only authority does not permit changing helpers or schemas; report an extraction
opportunity instead.

## Choose a Workflow

Choose exactly one workflow and read only its reference.

For skill creation, first inspect applicable repository instructions. When they define a source catalog and lifecycle,
stop and follow that repository-owned workflow. Use `skill-writing` only when no catalog-specific workflow exists.

| User intent                                                     | Workflow                     | Reference                                           |
| --------------------------------------------------------------- | ---------------------------- | --------------------------------------------------- |
| Update, refresh, sync, prune, polish, repair, or fix context    | `polish`                     | `references/polish.md`                              |
| Create, initialize, generate, or regenerate context files       | `create`                     | `references/create-docs.md`                         |
| Audit, check, review, inspect, or suggest changes without edits | `polish` in `--dry-run` mode | `references/polish.md`                              |
| Create or scaffold a skill                                      | Stop                         | Use repository catalog lifecycle or `skill-writing` |
| Install, discover, remove, or rename a skill                    | Stop                         | Use a dedicated skill-management workflow           |

If the intent is unclear, select `polish` in `--dry-run` mode and report the smallest useful planned change set.

## Authority

- Explicit create, update, polish, repair, fix, or equivalent intent authorizes in-scope local writes. Inspection-only
  intent and `--dry-run` do not.
- Require explicit confirmation before deleting README.md, AGENTS.md, regular CLAUDE.md files, or context-doc targets.
  `--force` authorizes documented overwrites, not deletions. The one standing exception is a CLAUDE.md symlink to a
  sibling AGENTS.md when the installed Claude Code reads AGENTS.md natively (see Claude Code Compatibility): delete it
  without asking.
- Treat a broad write request as authorization for the requested scope. Otherwise, preview a change set larger than a
  handful of files and stop before writing.
- Do not expand from documentation work into source changes, skill creation, or external writes.

## Arguments

- `path`: Optional repo-relative subtree. Restrict documentation, package-root, project-skill, source-catalog skill, and
  context-doc discovery to that subtree.
- `target ...`: Optional filters during `polish`: skill names from existing `.agents/skills/<name>/` or eligible
  `skills/<name>/` trees, or repo-relative Markdown paths selecting specific context docs.
- `--root-only`: Select only root README.md, AGENTS.md, and CLAUDE.md targets. Exclude project-installed skills,
  source-catalog skills, and context docs unless explicitly selected by `target`.
- `--dry-run`: Report planned writes and concise diffs without changing files.
- `--preserve`: During `polish`, keep accurate user-authored prose and structure; fix only drift and obvious noise.
- `--minimal`: Produce the smallest context that still meets the completion bar.
- `--thorough` / `--full`: Perform deeper analysis only where it adds durable, repository-specific context.
- `--force`: During `create`, regenerate existing README.md or AGENTS.md targets without prompting. Never applies to
  skills or deletions.

If `--minimal` and `--thorough` / `--full` are both present, make no writes and ask the user to choose. Report
unrecognized flags; continue only when they cannot change scope, safety, or write behavior.

## Repository Guard Rail

Run before discovery or writes:

```sh
cwd="$(pwd -P)"
case "$cwd" in
  /) printf 'abort: refusing to run at the filesystem root\n' >&2; exit 1 ;;
esac
repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  printf 'abort: not inside a git repository\n' >&2; exit 1; }
managed_skill_root=
case "$repo_root" in
  /|"$HOME") printf 'abort: unsupported repo root: %s\n' "$repo_root" >&2; exit 1 ;;
  "$HOME/.agents"|"$HOME/.codex"|"$HOME/.claude") managed_skill_root="$repo_root/skills" ;;
  "$HOME/.agents/"*|"$HOME/.codex/"*|"$HOME/.claude/"*)
    printf 'abort: repo root is nested under an agent configuration repository: %s\n' "$repo_root" >&2; exit 1 ;;
esac
if [ -n "$managed_skill_root" ]; then
  case "$cwd" in
    "$managed_skill_root"|"$managed_skill_root/"*)
      printf 'abort: installed skills must be edited in their source catalog: %s\n' "$cwd" >&2; exit 1 ;;
  esac
fi
```

When `managed_skill_root` is set, allow README.md, AGENTS.md, and CLAUDE.md work elsewhere in that repository, but
exclude the entire installed `skills/` tree from every workflow. Apply the exclusion before discovery, canonicalization,
or symlink traversal. If `path`, a `target`, or an explicit request would enter that tree, make no writes there and
report that the skill must be edited in its source catalog. `--force` does not override this boundary.

Outside managed agent-config roots, eligible git-tracked `skills/<name>/` source catalogs are in scope for `polish` per
`references/polish.md`.

## Claude Code Compatibility

Claude Code v2.1.277 and later read `AGENTS.md` directly whenever no `CLAUDE.md`, `.claude/CLAUDE.md`, or
`CLAUDE.local.md` exists in the working directory or above it, so a CLAUDE.md symlink is no longer needed. Detect the
installed version once per run before either workflow touches CLAUDE.md:

```sh
claude_version=$(claude --version 2>/dev/null | awk '{ print $1; exit }')
agents_md_native=false
if [ -n "$claude_version" ] &&
  [ "$(printf '%s\n' 2.1.277 "$claude_version" | sort -V | head -n 1)" = 2.1.277 ]; then
  agents_md_native=true
fi
```

When `agents_md_native=true`:

- Do not create CLAUDE.md symlinks.
- Delete every CLAUDE.md that is a symlink resolving to its sibling AGENTS.md, in the same pass and across the whole
  selected tree, using `git rm` when tracked. Leave regular CLAUDE.md and CLAUDE.local.md files untouched and report
  them: any such file at or above the repository root still suppresses direct AGENTS.md loading unless the user sets
  **Project instructions** to `claude-md-and-agents-md` in `/config`.

When `claude` is missing or older, keep the pre-native behavior: create or refresh a sibling symlink only where
CLAUDE.md is missing or already a symlink, and never delete one.

Snapshot `git status --short` before broad edits. Preserve unrelated pre-existing changes and re-check expected paths
after generators or broad commands.

## Discovery and Tool Routing

Use git-aware discovery, canonicalize every candidate beneath `repo_root`, and exclude VCS, dependency, environment, and
build outputs. Deliberately include ignored `.agents/skills/*/SKILL.md` only when project skills are selected. Discover
git-tracked, non-ignored, non-symlinked `skills/*/SKILL.md` only outside managed agent-config roots and only when
source-catalog skills are selected. Parse each selected skill's YAML frontmatter. Inspect only a project-installed
skill's declared write boundary before deciding whether it qualifies for a coordination exemption. Prefer `fd`, fall
back once on suspiciously narrow results, and synthesize independent repository evidence before writing.

Discover context docs by following Markdown links from README.md, AGENTS.md, CLAUDE.md, and SKILL.md files, then by
scanning remaining tracked Markdown whose content qualifies. Classify by content, never by file name or location.
Exclude changelogs, licenses, legal and policy notices, generated or vendored documentation, and prose that is product
content rather than guidance. When classification is uncertain, leave the file out of scope and report it as a
candidate.

## Completion and Report

After writes, run repository-defined Markdown formatting or checks when present. If skill frontmatter or
`agents/openai.yaml` changed in a project-installed skill, run its invocation metadata check. Verify that no CLAUDE.md
symlink remains when `agents_md_native=true`, and that every retained or created symlink resolves to its sibling
AGENTS.md otherwise. In `--dry-run`, report commands that would depend on planned files instead of running them.

Lead with `### ✅ Context updated` only after writes and required validation pass,
`### ⚠️ Context updated — validation failed` when files were written but required checks fail,
`### 🔎 Context preview — no files written` in dry-run mode, or `### ⛔ Context blocked — no files written` for a
pre-write stop. Then report only:

1. `🧭 Mode and scope`: workflow, dry-run status, target counts, and relative paths in a compact table.
2. `📦 Changes`: completed or planned changes grouped by directory; use a tree when it makes path ownership clearer.
3. `🧪 Validation`: exact commands, result, and any justified skip in a table.
4. `Issues and caveats`: conflicts, advisories, unrecognized flags, limitations, and unverified assumptions, grouped as
   `Resolved` (verified fixes with evidence) and `Open` (remaining impact and next step). Omit empty groups and the
   whole section when empty. Report each item once; put neutral context and agreed decisions under changes or scope.
   Reserve `blocker` for something preventing required work and `risk` for a specific potential adverse outcome. A
   workaround leaves an item open when the underlying issue still affects the result.

Keep paths, commands, guard-rail errors, symlink targets, and user-authored content exact and undecorated. Omit empty
detail and stop once the selected targets meet the completion bar.

## References

- `polish`: read `references/polish.md`.
- `create`: read `references/create-docs.md`.
