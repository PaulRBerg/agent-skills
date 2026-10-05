---
argument-hint:
  <polish|create|maintain> [path] [target ...] [--root-only] [--preserve] [--minimal] [--thorough|--full] [--dry-run]
  [--force]
compatibility:
  Requires curl and a writable user cache directory; network populates or refreshes the GPT-6.1 Sol and Claude Opus 5.5
  prompting guides.
name: agents-brain
skill-dependencies:
  - skill-writing
description:
  "Create or polish README.md, AGENTS.md/CLAUDE.md, skills, and repo context; continuously maintain repo-local skills
  when task evidence warrants deleting obsolete skills, merging overlapping skills, or creating a reusable skill."
---

# Agents Brain

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Create or polish repo-local context as one coherent system: human-facing README.md files, agent-facing AGENTS.md files
(with companion CLAUDE.md symlinks only for pre-native Claude Code, per Claude Code Compatibility below), existing
project-installed skills under `.agents/skills`, eligible source-catalog skills under `skills/<name>/`, and context docs
— any other Markdown files, under any name or directory, whose content is durable guidance for agents or humans, such as
conventions, command catalogs, data-format rules, workflow runbooks, and reference material. Maintain repo-owned skills
as task work reveals obsolete workflows, meaningful overlap, or recurring procedures worth capturing.

Success means every selected target is grounded in repository evidence, respects its audience and scope, spends agent
context only on guidance that changes behavior, and passes the narrowest repository-defined validation. Stop after
reporting completed or planned changes, validation, and any blockers.

## Model and Context Optimization

Optimize skills and other agent-facing context for GPT-6.1 Sol and Claude Opus 5.5 while preserving README.md as clear
human-facing documentation. Before complex, long-running, multi-tool, or orchestration-heavy context work, resolve
`scripts/fetch-guidance.sh` relative to this skill directory, run it once for `gpt-6.1-sol` and once for
`claude-opus-5-5`, and read both returned files completely. The helper retrieves the official
[GPT-6.1 Sol prompting guidance](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices)
and
[Claude Opus 5.5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)
because their recommendations may evolve. OpenAI publishes Sol guidance in the shared GPT-6 guide; evaluate its
family-wide prompting recommendations on GPT-6.1 Sol. Simple context work does not require either guide.

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

Choose one primary workflow and read its reference. Read `references/maintain-skills.md` additionally only for
task-driven skill maintenance or when adding its standing instruction to repository context.

For skill creation, first inspect applicable repository instructions. When they define a source catalog and lifecycle,
stop and follow that repository-owned workflow. Use `skill-writing` only when no catalog-specific workflow exists.

| User intent                                                                          | Workflow                                      | Reference                                           |
| ------------------------------------------------------------------------------------ | --------------------------------------------- | --------------------------------------------------- |
| Update, refresh, sync, prune, polish, repair, or fix context                         | `polish`                                      | `references/polish.md`                              |
| Create, initialize, generate, or regenerate context files                            | `create`                                      | `references/create-docs.md`                         |
| Audit, check, review, inspect, or suggest changes without edits                      | `polish` in `--dry-run` mode                  | `references/polish.md`                              |
| Maintain or evolve repo-local skills from task evidence; review lifecycle candidates | `maintain` (read-only for review-only intent) | `references/maintain-skills.md`                     |
| Create or scaffold a skill                                                           | Stop                                          | Use repository catalog lifecycle or `skill-writing` |
| Install, discover, or manage skills outside task-driven repo maintenance             | Stop                                          | Use a dedicated skill-management workflow           |

If the intent is unclear, select `polish` in `--dry-run` mode and report the smallest useful planned change set.

## Authority

- Apply explicit user instructions and established authorization before this skill's defaults. Do not ask again for an
  unchanged decision; preserve host restrictions and required destructive-action approval.
- Explicit create, update, polish, repair, fix, maintain, or equivalent intent authorizes in-scope local writes.
  Inspection-only intent, Plan Mode, and `--dry-run` do not. Under `maintain`, carry out evidence-backed removal,
  merging, and creation when the request or standing repository instructions authorize those lifecycle actions.
- Require explicit confirmation before deleting README.md, AGENTS.md, regular CLAUDE.md files, or context-doc targets.
  `--force` authorizes documented overwrites, not deletions. The one standing exception is a CLAUDE.md symlink to a
  sibling AGENTS.md when the installed Claude Code reads AGENTS.md natively (see Claude Code Compatibility): delete it
  without asking.
- Resolve scope from the requested outcome. Preview a large change set, then continue when it is already authorized;
  file count alone is not an approval boundary. Ask only when an unresolved choice changes scope or intended meaning.
- Keep `polish` and `create` documentation-only. Authorized skill lifecycle work runs separately through `maintain`; its
  repository boundary never permits external writes.

Complete authorized discovery, edits, and validation before reporting completion. When one target needs input, continue
independent targets and identify the exact unresolved choice. If a skill rule requires a pause, cite that rule and
explain why existing authorization does not cover the next action.

## Continuous Repository Skill Maintenance

During an authorized implementation task, notice skill impact while reading relevant guidance, after changing the
workflow it describes, and before the final report. Review only skills implicated by that task. When repository
instructions or the user authorize continuous maintenance, run `maintain` for verified opportunities and complete the
resulting changes; a recommendation alone is not completion. Finish fixed-scope workflows before this separate pass,
except for necessary prerequisites. Read-only tasks remain read-only.

Continuous means repeated attention during normal task work, not a background monitor, scheduled job, or catalog-wide
audit. Freeze the boundary to the repository where the task was given; never follow a skill's source or installation
into another repository or a global agent directory. A failed or blocked main task does not prevent an independent,
authorized skill improvement supported by verified evidence.

When asked to establish this behavior for future tasks, use the standing-instruction guidance in
`references/maintain-skills.md` through `polish` for existing AGENTS.md or `create` for a missing one. Loading this
skill in one session does not by itself make future agents run it.

## Arguments

- `path`: Optional repo-relative subtree. Restrict documentation, package-root, project-skill, source-catalog skill, and
  context-doc discovery to that subtree.
- `target ...`: Optional filters during `polish` or `maintain`: skill names from existing `.agents/skills/<name>/` or
  eligible `skills/<name>/` trees; `polish` also accepts repo-relative Markdown paths selecting context docs. `maintain`
  without targets considers only skills implicated by the current task's evidence.
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
`references/polish.md`, or their repository-owned lifecycle through `maintain`. Apply the stricter ownership and
repository boundary in `references/maintain-skills.md` before lifecycle work.

## Claude Code Compatibility

Claude Code v2.1.277 and later read `AGENTS.md` directly whenever no `CLAUDE.md`, `.claude/CLAUDE.md`, or
`CLAUDE.local.md` exists in the working directory or above it, so a CLAUDE.md symlink is no longer needed. Detect the
installed version once per run before a workflow touches CLAUDE.md:

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

For `maintain`, use the task-bounded discovery in `references/maintain-skills.md`. The context-discovery rules below
apply to `polish` and `create`.

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
pre-write stop. Follow with the workflow and scope, material changes, exact validation commands and outcomes, and any
remaining limitation. Use short prose for a small change; add headings or tables only when they organize repeated
information, and a tree only when directory ownership matters. Follow the user's requested report format.

When issues need a separate section, group verified fixes with evidence as `Resolved` and remaining problems as `Open`,
with impact and next action. Omit empty groups and report each item once. Reserve `blocker` for something preventing
required work and `risk` for a specific potential adverse outcome. A workaround leaves the underlying issue open.

Keep paths, commands, guard-rail errors, symlink targets, and user-authored content exact and undecorated. Omit empty
detail and stop once the selected targets meet the completion bar.

## References

- `polish`: read `references/polish.md`.
- `create`: read `references/create-docs.md`.
- `maintain` or establishing continuous repo-local skill maintenance: read `references/maintain-skills.md`.
