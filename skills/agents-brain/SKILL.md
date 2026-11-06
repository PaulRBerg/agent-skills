---
argument-hint:
  maintain [path] [target ...] [--root-only] [--preserve] [--minimal] [--thorough|--full] [--dry-run] [--force] |
  create-skill [skill-name] | create-agents-md [path] [--root-only] [--dry-run] [--force]
compatibility:
  Requires curl and a writable user cache directory. Network access populates or refreshes the official Anthropic and
  OpenAI prompting guides and the agentskills.io specification.
name: agents-brain
description:
  Maintain README.md, AGENTS.md/CLAUDE.md, skills, and repo context. Create, scaffold, or initialize AGENTS.md files and
  project-local agent skills. Use task evidence to delete obsolete skills, merge overlapping skills, and capture
  recurring workflows.
---

# Agents Brain

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly.
In that case, do not invoke this skill again through a skill tool.

Apply explicit user instructions and established authorization before this skill's defaults. Do not ask again for an
unchanged decision. Preserve host restrictions and required destructive-action approval.

Read this file and each reference in separate reads, each sized to the whole file. A truncated read drops rules
silently.

## Routes

- `maintain [path] [target ...] [flags]`: Maintain context and apply task-backed skill lifecycle changes. Use this route
  by default for context maintenance or inspection. Read [references/maintain.md](references/maintain.md).
- `create-skill [skill-name]`: Create a project-local skill from an explicit request. Read
  [references/create-skill.md](references/create-skill.md). Apply its repository catalog guard before scaffolding.
  Explicit creation does not require recurring-use evidence.
- `create-agents-md [path] [--root-only] [--dry-run] [--force]`: Initialize AGENTS.md files for a scope from an explicit
  request. Create the scope-root file and judge each subtree for a nested file. Read
  [references/create-agents-md.md](references/create-agents-md.md).

Choose the route before discovery or writes. All routes use the prompting and STE requirements below. The maintenance
sections govern `maintain`. Each creation reference owns its route's scope, workflow, validation, and report.

## Maintenance Scope

Maintain repo-local context as one coherent system. It includes these targets:

- Human-facing README.md files.
- Agent-facing AGENTS.md files, with companion CLAUDE.md symlinks only for pre-native Claude Code. See Claude Code
  Compatibility below.
- Existing project-installed skills under `.agents/skills` and eligible source-catalog skills under `skills/<name>/`.
- Context docs: other Markdown files, under any name or directory, that provide durable guidance for agents or humans.
  Examples include conventions, command catalogs, data-format rules, workflow runbooks, and reference material.

Maintain repo-owned skills as task work reveals obsolete workflows, meaningful overlap, or recurring procedures worth
capturing.

Success means every selected target meets these criteria:

- Repository evidence supports its content.
- It respects its audience and scope.
- It spends agent context only on guidance that changes behavior.
- It passes the narrowest repository-defined validation.

Stop after reporting completed or planned changes, validation, and any blockers.

## Prompting and Context Optimization

Write skills and other agent-facing context for current Claude and GPT agent models, not for one model. Do not name a
model, a model version, or a model-specific default in that prose. Content that selects, configures, or attributes a
model is the exception, for example routing tables, `model:` fields, and API parameters. Preserve README.md as clear
human-facing documentation.

Read the prompting guides before `create-skill`, `create-agents-md`, `--force` regeneration, and `maintain` runs that
create or rewrite skill instructions, prompt text, or a whole AGENTS.md section. Do not run the helper or read either
guide for factual corrections, README.md edits, single rule additions or removals, link fixes, moves, or deletions. For
those runs, report `guides: skipped (<reason>)`.

Before a run that needs the guides, resolve `scripts/fetch-guidance.sh` relative to this skill directory. Run it once
for `anthropic` and once for `openai`. The two runs are independent, so start both in the same turn. Read each returned
file in its own read, sized to the whole file. The Anthropic guide exceeds 1,000 lines. List its `##` headings first.
Read every section that applies to all current models completely. Read a section that covers one capability, one model,
or migration only when the task targets it. Read the OpenAI guide completely.

The helper retrieves Anthropic's
[prompting best practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices)
and OpenAI's [latest-model guide](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices)
because their recommendations may change. The Anthropic guide covers all current Claude models and links each per-model
prompting page. The OpenAI guide covers the current GPT model family. Its prompting recommendations come from tests on
one model in that family.

Treat advice that names a model as measured on that model. Apply it to other models only when the cross-model guidance
supports it. To check such a claim, read the linked per-model page. When the guides give opposite corrections for
different models, state the wanted behavior directly. Do not write a correction to one model's default, such as "be
brief" or "write more".

Accept an integrity-valid `cached` guide for 24 hours. Use `--refresh` for explicitly latest or change-sensitive work,
materially disputed guidance, or conflicts with observed model behavior. Interpret helper diagnostics precisely:

- `cached` reused a guide validated no more than 24 hours ago without network access.
- `revalidated` refreshed validation metadata after a successful conditional `304` response.
- `fetched` atomically replaced the cache with integrity-valid content from the pinned official URL.
- `stale` reused a guide validated no more than seven days ago after live retrieval failed. Proceed only after reading
  it. Disclose the validation timestamp and retrieval failure under open issues and caveats.

Forced refreshes, expired entries, integrity failures, and unexpected redirects fail closed. If either required guide
cannot be returned, stop qualifying work before writing. Do not substitute memory or another source. Never create the
cache in a repository or skill installation.

Keep only content that changes a decision, prevents an evidenced mistake, or supplies a non-discoverable constraint.
State each meaning once at the narrowest reliable load scope, except where independently installed artifacts need to
stay self-contained. Preserve authority, safety, material exceptions, semantic success criteria, and exact
machine-consumed text. Documentation-only authority does not permit changing helpers or schemas. Report an extraction
opportunity instead.

## STE Authoring and Review

Before writing or changing agent-facing prose, read [references/asd-ste100.md](references/asd-ste100.md) completely.
Apply its STE-based profile during writing. Complete its meaning and style review before reporting completion. Preserve
obligation strength, uncertainty, conditions, technical precision, and protected literal content. Honor the target's
audience and any instructions to preserve accurate user-authored prose. The profile does not establish official ASD
dictionary compliance. One complete read of that reference during the current run satisfies every later requirement in
this skill to read it, unless the file changed since.

## Maintain Workflow

Use `$agents-brain maintain` for context maintenance and task-backed skill lifecycle changes. Within the requested
scope, update existing context, create warranted missing context, and apply task-backed skill lifecycle changes.

In every `maintain` run, capture corrections. Scan the session for user corrections, such as a restated instruction, "do
not do X", "always do Y", or a reverted choice. Record each durable correction as a rule in the nearest in-scope
steering file, such as AGENTS.md, or in the owning skill. When a correction is not durable, state the reason in the
report: one-off, already stated, or a project-specific exception.

Read `references/create-docs.md` only when creating or regenerating context. Read `references/maintain-skills.md` only
for skill lifecycle decisions or establishing continuous maintenance. These references support the workflow. They are
not separate modes.

For skill creation, use this skill's `references/create-skill.md` after applying the maintenance evidence gate. Follow
applicable source-catalog lifecycle instructions when defined. Installation and skill management outside this repository
belong to a dedicated workflow. For inspection-only or unclear intent, use `maintain --dry-run` and report the smallest
useful planned change set.

## Maintenance Authority

- Explicit creation, update, repair, fix, maintenance, or equivalent intent authorizes in-scope local writes.
  Inspection-only intent, Plan Mode, and `--dry-run` do not. Carry out evidence-backed removal, merging, and creation
  when the request or standing repository instructions authorize those lifecycle actions.
- Require explicit confirmation before deleting README.md, AGENTS.md, regular CLAUDE.md files, or context-doc targets.
  `--force` authorizes documented overwrites, not deletions. The one standing exception is a CLAUDE.md symlink to a
  sibling AGENTS.md when the installed Claude Code reads AGENTS.md natively (see Claude Code Compatibility): delete it
  without asking.
- Resolve scope from the requested outcome. Preview a large change set, then continue when it is already authorized.
  File count alone is not an approval boundary. Ask only when an unresolved choice changes scope or intended meaning.
- Keep context edits documentation-only. When authorized, skill lifecycle work may change the affected skill's owned
  files and local consumers. The repository boundary never permits external writes.

Complete authorized discovery, edits, and validation before reporting completion. When one target needs input, continue
independent targets. Identify the exact unresolved choice. If a skill rule requires a pause, cite that rule and explain
why existing authorization does not cover the next action.

## Continuous Repository Skill Maintenance

During an authorized implementation task, notice skill impact while reading relevant guidance, after changing the
workflow it describes, and before the final report. Review only skills implicated by that task. When repository
instructions or the user authorize continuous maintenance, act on verified opportunities and complete the resulting
changes. A recommendation alone is not completion. Finish fixed-scope caller workflows before independent maintenance,
except for necessary prerequisites. Read-only tasks remain read-only.

Continuous means repeated attention during normal task work, not a background monitor, scheduled job, or catalog-wide
audit. Freeze the boundary to the repository where the task was given. Never follow a skill's source or installation
into another repository or a global agent directory. A failed or blocked main task does not prevent an independent,
authorized skill improvement supported by verified evidence.

When asked to establish this behavior for future tasks, use the standing-instruction guidance in
`references/maintain-skills.md` to update or create the applicable AGENTS.md. Loading this skill in one session does not
by itself make future agents run it.

## Maintenance Arguments

- `path`: Optional repo-relative subtree. Restrict documentation, package-root, project-skill, source-catalog skill, and
  context-doc discovery to that subtree.
- `target ...`: Optional filters: skill names from existing `.agents/skills/<name>/` or eligible `skills/<name>/` trees,
  or repo-relative Markdown paths selecting context docs. Without filters, discover context within the selected tree.
  Continuous skill lifecycle review still considers only skills implicated by the current task's evidence.
- `--root-only`: Select only root README.md, AGENTS.md, and CLAUDE.md targets. Exclude project-installed skills,
  source-catalog skills, and context docs unless explicitly selected by `target`.
- `--dry-run`: Report planned writes and concise diffs without changing files.
- `--preserve`: Keep accurate user-authored prose and structure. Fix only drift and obvious noise.
- `--minimal`: Produce the smallest context that still meets the completion bar.
- `--thorough` / `--full`: Perform deeper analysis only where it adds durable, repository-specific context.
- `--force`: Regenerate existing README.md or AGENTS.md targets without prompting. Never applies to skills or deletions.

If `--minimal` and `--thorough` / `--full` are both present, make no writes. Ask the user to choose. Report unrecognized
flags. Continue only when they cannot change scope, safety, or write behavior.

## Repository Guard Rail

For `maintain` and `create-agents-md`, run `scripts/preflight.sh`, resolved relative to this skill directory, from the
invocation working directory before discovery or writes. It prints one `key=value` line per fact:

- `repo_root`: the Git repository root.
- `managed_skill_root`: `<repo_root>/skills` when the repository root is `~/.agents`, `~/.codex`, or `~/.claude`.
  Otherwise empty.
- `claude_version`: the installed Claude Code version. Empty when `claude` is missing.
- `agents_md_native`: `true` when Claude Code 2.1.277 or later is installed. Otherwise `false`.

The helper exits 1 with an `abort:` message on stderr at the filesystem root, outside a Git repository, when the
repository root is `/` or `$HOME`, when the repository is nested under an agent configuration repository, or when the
working directory is inside a managed `skills/` tree. On a nonzero exit, stop and report the message exactly.

When `managed_skill_root` is set, allow README.md, AGENTS.md, and CLAUDE.md work elsewhere in that repository. In that
case, exclude the entire installed `skills/` tree from every workflow. Apply the exclusion before discovery,
canonicalization, or symlink traversal. If `path`, a `target`, or an explicit request would enter that tree, make no
writes there and report that the skill must be edited in its source catalog. `--force` does not override this boundary.

Outside managed agent-config roots, eligible git-tracked `skills/<name>/` source catalogs are in scope for factual
context corrections per `references/maintain.md` or their repository-owned lifecycle. Apply the stricter ownership and
repository boundary in `references/maintain-skills.md` before lifecycle work.

## Claude Code Compatibility

Claude Code v2.1.277 and later read `AGENTS.md` directly whenever no `CLAUDE.md`, `.claude/CLAUDE.md`, or
`CLAUDE.local.md` exists in the working directory or above it, so a CLAUDE.md symlink is no longer needed. Use the
`agents_md_native` value that `scripts/preflight.sh` printed. Run the helper once per run before a workflow touches
CLAUDE.md.

When `agents_md_native=true`:

- Do not create CLAUDE.md symlinks.
- Delete every CLAUDE.md that is a symlink resolving to its sibling AGENTS.md, in the same pass and across the whole
  selected tree, using `git rm` when tracked. Leave regular CLAUDE.md and CLAUDE.local.md files untouched. Report them.
  Any such file at or above the repository root still suppresses direct AGENTS.md loading unless the user sets **Project
  instructions** to `claude-md-and-agents-md` in `/config`.

When `claude` is missing or older, keep the pre-native behavior. In that case, create or refresh a sibling symlink only
where CLAUDE.md is missing or already a symlink. Never delete one under the pre-native behavior.

Snapshot `git status --short` before broad edits. Preserve unrelated pre-existing changes and re-check expected paths
after generators or broad commands.

## Discovery and Tool Routing

For task-driven skill lifecycle review, use the bounded discovery in `references/maintain-skills.md`. For context
maintenance, apply the discovery rules below within the requested scope.

Use git-aware discovery. Canonicalize every candidate beneath `repo_root`. Exclude VCS, dependency, environment, and
build outputs.

Deliberately include ignored `.agents/skills/*/SKILL.md` only when project skills are selected. Discover git-tracked,
non-ignored, non-symlinked `skills/*/SKILL.md` only outside managed agent-config roots and only when source-catalog
skills are selected. Parse each selected skill's YAML frontmatter.

Inspect only a project-installed skill's declared write boundary before deciding whether it qualifies for a coordination
exemption. Prefer `fd`. If results are suspiciously narrow, use an alternative once. Combine independent repository
evidence before writing.

Discover context docs by following Markdown links from README.md, AGENTS.md, CLAUDE.md, and SKILL.md files, then by
scanning remaining tracked Markdown whose content qualifies. Classify by content, never by file name or location.
Exclude changelogs, licenses, legal and policy notices, generated or vendored documentation, and prose that is product
content rather than guidance. When classification is uncertain, leave the file out of scope and report it as a
candidate.

## Completion and Report

After writes, run repository-defined Markdown formatting or checks when present. Complete the meaning and style review
in `references/asd-ste100.md` for every authored or changed agent-facing prose target. Compare existing prose with its
source and new prose with the authoritative requirements. Review protected content and precision exceptions explicitly.

If skill frontmatter or `agents/openai.yaml` changed in a project-installed skill, run its invocation metadata check.
When `agents_md_native=true`, verify that no CLAUDE.md symlink remains. Otherwise, verify that every retained or created
symlink resolves to its sibling AGENTS.md. In `--dry-run`, report commands that would depend on planned files instead of
running them.

Lead with `### ✅ Context updated` only after writes and required validation pass,
`### ⚠️ Context updated — validation failed` when files were written but required checks fail,
`### 🔎 Context preview — no files written` in dry-run mode, or `### ⛔ Context blocked — no files written` for a
pre-write stop. Follow with the scope, material changes, exact validation commands and outcomes, and any remaining
limitation. Use short prose for a small change. Add headings or tables only when they organize repeated information. Add
a tree only when directory ownership matters. Follow the user's requested report format.

When issues need a separate section, group verified fixes with evidence as `Resolved` and remaining problems as `Open`,
with impact and next action. Omit empty groups. Report each item once. Reserve `blocker` for something preventing
required work and `risk` for a specific potential adverse outcome. A workaround leaves the underlying issue open.

An `Open` item may cite only user-owned input, an action outside the repository, or a confirmation boundary (destructive
action, purchase, deployment, or external write). When the task permits writes and an in-repository change resolves an
item, make that change before the report under the standing maintenance authorization. Do not write
`needs owner decision`, `report-only`, or `residual risk` for a routine engineering choice. Decide, make the change, and
state the decision in the report.

Keep paths, commands, guard-rail errors, symlink targets, and user-authored content exact and undecorated. Omit empty
detail. Stop once the selected targets meet the completion bar.

## References

- Maintenance: read [references/maintain.md](references/maintain.md) for the workflow.
- Before writing or changing agent-facing prose: read [references/asd-ste100.md](references/asd-ste100.md) for the
  STE-based profile and completion review.
- Creating or regenerating context: read [references/create-docs.md](references/create-docs.md) for placement and
  generation rules.
- Skill lifecycle changes or establishing continuous maintenance: read
  [references/maintain-skills.md](references/maintain-skills.md).
- Creating a skill: read [references/create-skill.md](references/create-skill.md) for scope, metadata, symlink, and
  validation requirements. Run `scripts/fetch-agentskills-spec.sh [--refresh]` through Bash to obtain the absolute
  cached specification path. The default reuses a valid cache for 24 hours, then revalidates it. The creation reference
  owns refresh, stale-cache, and failure rules.
- Initializing AGENTS.md files: read [references/create-agents-md.md](references/create-agents-md.md) for scope,
  nested-file decisions, and the report.
- Before choosing a new skill's contract or layout: read
  [references/writing-great-skills.md](references/writing-great-skills.md) for authoring, routing, and execution rules.
