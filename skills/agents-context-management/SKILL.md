---
argument-hint:
  <polish|create> [path] [target ...] [--root-only] [--preserve] [--minimal] [--thorough|--full] [--dry-run] [--force]
disable-model-invocation: true
name: agents-context-management
user-invocable: true
description:
  "Create or polish repo agent context: README.md, AGENTS.md/CLAUDE.md, installed project skills, and other Markdown
  context docs."
---

# Agents Context Management

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Create or polish repo-local context as one coherent system: human-facing README.md files, agent-facing AGENTS.md files
with companion CLAUDE.md symlinks, existing project-installed skills under `.agents/skills`, and context docs — any
other Markdown files, under any name or directory, whose content is durable guidance for agents or humans, such as
conventions, command catalogs, data-format rules, workflow runbooks, and reference material.

Success means every selected target is grounded in repository evidence, respects its audience and scope, and passes the
narrowest repository-defined validation. Stop after reporting completed or planned changes, validation, and any
blockers.

## Model Optimization

Optimize skills and other agent-facing context for GPT-5.6 and Claude Fable 5 while preserving README.md as clear
human-facing documentation. The summaries below are reminders, not substitutes for the live guides. Read both guides
before complex, long-running, multi-tool, or orchestration-heavy context work because their recommendations may evolve.

- [GPT-5.6 prompting guidance](https://developers.openai.com/api/docs/guides/prompt-guidance-gpt-5p6): Prefer lean,
  outcome-first prompts that specify the goal, success and stopping criteria, constraints, evidence, permission
  boundaries, tool routing, output shape, and validation. Remove redundant scaffolding and evaluate changes on
  representative tasks.
- [Claude Fable 5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5):
  Use concise instructions that explain intent and boundaries; avoid over-prescription and scope creep; tune effort
  deliberately; ground progress claims in tool evidence; and make long-run verification and scaffolding explicit when
  needed.

## Choose the representation

- Preserve goals, decision rules, authority, exceptions, and semantic success criteria as prose.
- When existing scripts, recipes, schemas, or configuration already enforce mechanics, document only routing, inputs,
  side effects, outputs, failure signals, and completion evidence.
- Documentation-only authority does not permit creating or changing helpers or schemas. When none exists, retain
  accurate prose and report the extraction opportunity instead of deleting guidance or expanding scope.

## Choose a Workflow

Choose exactly one workflow and read only its reference.

| User intent                                                     | Workflow                     | Reference                                 |
| --------------------------------------------------------------- | ---------------------------- | ----------------------------------------- |
| Update, refresh, sync, prune, polish, repair, or fix context    | `polish`                     | `references/brain-polish.md`              |
| Create, initialize, generate, or regenerate context files       | `create`                     | `references/create-docs.md`               |
| Audit, check, review, inspect, or suggest changes without edits | `polish` in `--dry-run` mode | `references/brain-polish.md`              |
| Create or scaffold a skill                                      | Stop                         | Refer to `skills/create-skill`            |
| Install, discover, remove, or rename a skill                    | Stop                         | Use a dedicated skill-management workflow |

If the intent is unclear, select `polish` in `--dry-run` mode and report the smallest useful planned change set.

## Authority

- Explicit create, update, polish, repair, fix, or equivalent intent authorizes in-scope local writes. Inspection-only
  intent and `--dry-run` do not.
- Require explicit confirmation before deleting README.md, AGENTS.md, CLAUDE.md, or context-doc targets. `--force`
  authorizes documented overwrites, not deletions.
- Treat a broad write request as authorization for the requested scope. Otherwise, preview a change set larger than a
  handful of files and stop before writing.
- Do not expand from documentation work into source changes, skill creation, or external writes.

## Arguments

- `path`: Optional repo-relative subtree. Restrict documentation, package-root, project-skill, and context-doc discovery
  to that subtree.
- `target ...`: Optional filters during `polish`: existing `.agents/skills/<name>/` skill names, or repo-relative
  Markdown paths selecting specific context docs.
- `--root-only`: Select only root README.md, AGENTS.md, and CLAUDE.md targets. Exclude project skills and context docs
  unless explicitly selected by `target`.
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

Snapshot `git status --short` before broad edits. Preserve unrelated pre-existing changes and re-check expected paths
after generators or broad commands.

## Shared Constraints

Stay inside the resolved repository and preserve unrelated changes. The selected workflow reference is authoritative for
README.md, AGENTS.md, CLAUDE.md, and project-skill behavior; do not repeat or broaden its file-specific rules here.

## Discovery and Tool Routing

Use git-aware discovery, canonicalize every candidate beneath `repo_root`, and exclude VCS, dependency, environment, and
build outputs. Deliberately include ignored `.agents/skills/*/SKILL.md` only when project skills are selected. Prefer
`fd`, fall back once on suspiciously narrow results, and synthesize independent repository evidence before writing.

Discover context docs by following Markdown links from README.md, AGENTS.md, CLAUDE.md, and SKILL.md files, then by
scanning remaining tracked Markdown whose content qualifies. Classify by content, never by file name or location.
Exclude changelogs, licenses, legal and policy notices, generated or vendored documentation, and prose that is product
content rather than guidance. When classification is uncertain, leave the file out of scope and report it as a
candidate.

## Completion and Report

After writes, run repository-defined Markdown formatting or checks when present. If skill frontmatter or
`agents/openai.yaml` changed in a catalog, run its invocation metadata check. Verify changed CLAUDE.md symlinks resolve
to sibling AGENTS.md. In `--dry-run`, report commands that would depend on planned files instead of running them.

Lead with `### ✅ Context updated` only after writes and required validation pass,
`### ⚠️ Context updated — validation failed` when files were written but required checks fail,
`### 🔎 Context preview — no files written` in dry-run mode, or `### ⛔ Context blocked — no files written` for a
pre-write stop. Then report only:

1. `🧭 Mode and scope`: workflow, dry-run status, target counts, and relative paths in a compact table.
2. `📦 Changes`: completed or planned changes grouped by directory; use a tree when it makes path ownership clearer.
3. `🧪 Validation`: exact commands, result, and any justified skip in a table.
4. `⚠️ Blockers and risks`: conflicts, advisories, and unrecognized flags; omit when empty.

Keep paths, commands, guard-rail errors, symlink targets, and user-authored content exact and undecorated. Omit empty
detail and stop once the selected targets meet the completion bar.

## References

- `polish`: read `references/brain-polish.md`.
- `create`: read `references/create-docs.md`.
