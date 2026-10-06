# Maintain Workflow

Maintain context for factual accuracy, useful placement, and lower noise. Preserve accurate user-authored content and
structure. Update existing context by default. Create missing context when requested or when verified guidance has no
appropriate existing home. For context creation, use this skill's `references/create-docs.md`. Apply task-backed skill
lifecycle changes using `references/maintain-skills.md` under the same `maintain` workflow.

Before writing or changing agent-facing prose, read this skill's `references/asd-ste100.md` completely. Apply its
STE-based profile only to prose the task authorizes you to write or change. At completion, compare meaning with the
source or authoritative requirements before checking style. Preserve obligation strength and protected technical
content. Honor instructions to preserve accurate user-authored prose and structure.

Success means each changed claim is verified against the repository. Each instruction lives at the narrowest useful
scope. No unrelated content or user work is disturbed.

## Discover and Inspect

Select existing README.md and AGENTS.md files, sibling CLAUDE.md entries, in-scope context docs, and any in-scope
existing skill targets under `.agents/skills/<name>/` or eligible `skills/<name>/` trees. Apply `path`, `--root-only`,
and `target` filters before reading deeply. Preserve any caller's narrower file scope. Missing context or lifecycle work
outside it requires separate authorization or an applicable standing instruction.

Use the nearest manifests, task runners, lock files, lint and CI configuration, generated-file notices, and relevant
source files to verify claims. Check paths, commands, scripts, recipes, environment variables, ownership rules, default
branches, and local conventions.

Detect CONTRIBUTING.md next to documentation targets. Never edit it. Advise the user when stable agent guidance should
move into sibling AGENTS.md.

Preview unexpectedly large target sets against the requested outcome. Continue within existing authorization. Ask only
before adding outcomes or changing meaning the user has not authorized. File count alone does not require approval.

## Context Economy Audit

Before applying the file-specific decisions below, classify each agent-facing target by how it enters context: always
loaded, inherited through a scope chain, conditional or path-scoped, or independently loaded on demand.

- For every retained block, identify the decision it changes, the mistake it prevents, or the non-discoverable fact it
  supplies. Remove generic defaults, tutorials, history, inventories, stale rationale, and other prose with no durable
  behavioral effect.
- Remove exact and semantic duplication from the same effective load chain. Put shared meaning in the parent. Keep a
  child to its delta or override. Do not deduplicate independently loaded artifacts when that would break
  self-containment.
- Replace equivalent lists of prohibitions with one positive decision rule. Retain rationale only when it changes how a
  rule is interpreted. Retain one minimal example only for an exact requirement or an evidenced failure.
- Route specialized guidance to the deepest existing applicable context or an existing on-demand doc or skill. When no
  suitable target exists, apply the higher creation bar in `references/maintain-skills.md` for skill candidates and the
  placement rules in `references/create-docs.md` for missing context. Preserve explicit creation and move approvals.
- Preserve authority, safety, material exceptions, semantic completion criteria, exact commands and machine-consumed
  text, and clarity. Re-read the effective load chain after pruning to ensure no required constraint is orphaned or
  contradicted.
- In always-loaded or inherited files, keep in-band metadata only when it changes reader behavior. Remove
  maintainer-only identifiers and annotations, or state their meaning for the reader once at the root.

## README.md Decisions

Keep README.md useful to humans browsing the repository, package registry, or project page:

- Preserve an accurate project description, badges, documentation and package links, references, acknowledgments,
  funding, and license information.
- Keep a short contributing pointer to sibling AGENTS.md.
- Keep short operator-run setup instructions only for dotfiles, infrastructure, homelab, personal tooling, or when the
  user explicitly requests them.
- Move developer commands, architecture constraints, review rules, configuration manuals, and contribution workflow into
  AGENTS.md when they provide durable value there.
- Remove directory trees, command inventories, placeholders, and generic explanations that are cheaply discoverable or
  add no decision guidance.

With `--preserve`, retain accurate custom prose and structure. Make the smallest edit that restores truth or correct
placement.

## AGENTS.md Decisions

Keep AGENTS.md terse, imperative, repository-specific, and scoped to its directory tree:

- Preserve commands when their preferred order, runner, side effects, environment, or failure behavior matters.
- Preserve non-obvious architecture, style, naming, review, generated-file, safety, external-disclosure, credential,
  deployment, financial, and recipient-scoped data-handling constraints.
- Preserve speed traps, flaky checks, shell quirks, migration constraints, and external-system notes that prevent
  observed mistakes.
- Keep AGENTS.md a short map that links to deeper context docs, not an encyclopedia. A long AGENTS.md loads on every
  task and crowds out the task and the code. It also makes every rule look equally important, goes stale, and resists
  mechanical checks. Route depth to the linked context docs, so agents start from a small, stable entry point and learn
  where to look next.
- State each obligation as a rule the agent can find. Keep schemas, procedures, and catalogs in linked context docs.
- Make every skill, command, or document that a rule names reachable from where the rule loads. When the harness does
  not list a named skill there, state how to find it.
- Use the same section name for the same role in sibling files.
- When task evidence shows that a decision lives only outside the repository, record it in the applicable context.
  Examples of such places are a chat thread, an external document, and a person's knowledge. An agent can see only what
  the repository contains.
- Remove generic tutorials, historical authoring notes, file inventories, lists of installed skills, and command lists
  with no preference or warning.
- When requested, establish continuous repo-local skill maintenance using the standing instruction in
  `references/maintain-skills.md` from this skill. Preserve an equivalent existing rule without duplicating it.

Move subtree-specific rules to the deepest common ancestor where they apply. Promote duplicated child guidance only when
every affected child shares it. Recommend a missing nested AGENTS.md only for a distinct command, safety rule,
generated-file boundary, ownership rule, data constraint, or review requirement. For authorized creation, use
`references/create-docs.md`.

Never delete an empty or obsolete AGENTS.md automatically. Report it as a deletion candidate, together with any sibling
CLAUDE.md symlink. Require explicit confirmation.

## CLAUDE.md Decisions

Run the version check from Claude Code Compatibility in SKILL.md.

When `agents_md_native=true`, delete every CLAUDE.md symlink in the selected tree that resolves to its sibling
AGENTS.md, without confirmation. Use `git rm` when tracked. In that case, also retire repository checks, hooks, and
instructions that require the symlink. Report any remaining regular CLAUDE.md or CLAUDE.local.md that still suppresses
direct AGENTS.md loading.

Otherwise, create or refresh a sibling symlink only when CLAUDE.md is missing or already a symlink:

```sh
(cd "$dir" && ln -sfn AGENTS.md CLAUDE.md)
```

Before writing, require `test -L "$dir/CLAUDE.md" || test ! -e "$dir/CLAUDE.md"`. A regular CLAUDE.md blocks only that
target. Leave it untouched and report the conflict.

After changing placement or symlinks, rediscover affected targets and confirm no local constraint was orphaned.

## Context Doc Decisions

Maintain selected context docs — conventions, command catalogs, data-format rules, workflow runbooks, and similar
reference material — wherever they live and whatever they are named:

- Treat architecture maps of domains and layering, quality or technical-debt registers, and plan or decision logs as
  context docs when they guide future work.
- Also treat state-surface indexes and knowledge-home maps as context docs. A state-surface index maps each question to
  its cheapest authoritative source. A knowledge-home map maps each kind of learned knowledge to where to record it.
- Verify commands, paths, flags, formats, environment variables, versions, and rules against the repository with the
  same rigor as AGENTS.md.
- When repository instructions assign a document class to a repository-owned lifecycle or workflow, fix only factual
  drift in those docs and report structural or placement changes as recommendations.
- Preserve each doc's audience, depth, structure, and voice. A deep reference stays a deep reference. Do not compress it
  to AGENTS.md terseness or inline it into AGENTS.md.
- Fix broken links between context docs, README.md, AGENTS.md, and skills. Do not move or rename docs.
- Recommend relocating guidance only when it is clearly misplaced, such as stable repo-wide rules living solely in a
  deep doc nothing links to. Perform the move only with explicit confirmation.
- Report an obsolete doc whose central subject no longer exists as a deletion candidate. Never delete it or remove its
  essential content.

## Skill Decisions

Apply factual corrections to these existing skill classes:

- Project-installed skills under `.agents/skills`. A minimal factual fix may touch SKILL.md or its existing bundled
  files.
- Source-catalog skills under `skills/<name>/` when `SKILL.md` is git-tracked, the tree is neither ignored nor
  symlinked, and the repository root is neither a managed agent-config root nor nested under one, as enforced by the
  Repository Guard Rail. Edit only the SKILL.md body and existing bundled Markdown, such as files under `references/`.

For task-backed creation, deletion, merging, or restructuring, follow `references/maintain-skills.md` when authorized by
the user or standing repository instructions. The factual-correction rules below do not limit separately authorized
lifecycle changes. Both belong to this maintenance workflow.

For a project-installed skill:

- Confirm frontmatter parses. Confirm `name` matches the directory. Fix only mechanical, unambiguous drift.
- Classify its declared default write boundary. If it writes no repository files or only repository metadata, add
  `coordination: exempt` when absent. Add the standard body sentence near the top:
  `This skill is coordination-exempt: skip the ai-coord gate for its declared work.` Explicitly authorized escalation
  beyond that declared behavior enters the gate.
- Otherwise, omit `coordination`. When repository evidence establishes that the exemption is unsafe, remove a stale
  `coordination: exempt` field and its matching standard body sentence. Do not invent another `coordination` value. Keep
  frontmatter fields alphabetized, with `description` last.

For a source-catalog skill, treat frontmatter, `agents/openai.yaml`, `metadata.install-targets`, the bundled-file set,
and file structure as report-only. Report drift in those surfaces or in the skill's purpose as recommendations. Never
edit them.

For each selected skill:

- Verify referenced `references/`, `scripts/`, `assets/`, and `examples/` paths relative to the skill directory.
- Read only the bundled files needed to verify paths, commands, flags, environment variables, versions, symbols,
  ownership, and repository conventions.
- Preserve structure and voice. Use the smallest factual edit span.
- Leave third-party behavior and paths outside the repository unchanged unless current repository evidence
  authoritatively establishes the correction.
- Record evidence for an obsolete or redundant skill or a useful merge. Apply authorized lifecycle changes with
  `references/maintain-skills.md`. Otherwise, report the candidate without deleting the skill or removing its essential
  content.

## Finish

Run the completion checks from SKILL.md. Complete the meaning and style review in this skill's
`references/asd-ste100.md`. Review protected content and precision exceptions explicitly. Use the report contract from
SKILL.md.

Finish when selected context is accurate, warranted missing context is created, and authorized task-backed skill changes
are complete and validated. Do not expand into unrelated cleanup.
