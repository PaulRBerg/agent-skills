# Polish Workflow

Update existing context for factual accuracy, useful placement, and lower noise. Do not create README.md, AGENTS.md,
context docs, or skills, and do not broadly restyle accurate user-authored content.

Success means each changed claim is verified against the repository, each instruction lives at the narrowest useful
scope, and no unrelated content or user work is disturbed.

## Discover and Inspect

Select existing README.md and AGENTS.md files, sibling CLAUDE.md entries, in-scope context docs, and any in-scope
project-installed `.agents/skills/<name>/SKILL.md` files. Apply `path`, `--root-only`, and `target` filters before
reading deeply.

Use the nearest manifests, task runners, lock files, lint and CI configuration, generated-file notices, and relevant
source files to verify claims. Check paths, commands, scripts, recipes, environment variables, ownership rules, default
branches, and local conventions.

Detect CONTRIBUTING.md next to documentation targets. Never edit it; advise the user when stable agent guidance should
move into sibling AGENTS.md.

If a non-broad request would touch more than a handful of files, switch to dry-run, show the planned targets, and stop
before writing.

## Context Economy Audit

Before applying the file-specific decisions below, classify each agent-facing target by how it enters context: always
loaded, inherited through a scope chain, conditional or path-scoped, or independently loaded on demand.

- For every retained block, identify the decision it changes, the mistake it prevents, or the non-discoverable fact it
  supplies. Remove generic defaults, tutorials, history, inventories, stale rationale, and other prose with no durable
  behavioral effect.
- Remove exact and semantic duplication from the same effective load chain. Put shared meaning in the parent and keep a
  child to its delta or override; do not deduplicate independently loaded artifacts when that would break
  self-containment.
- Replace equivalent lists of prohibitions with one positive decision rule. Retain rationale only when it changes how a
  rule is interpreted, and retain one minimal example only for an exact requirement or an evidenced failure.
- Route specialized guidance to the deepest existing applicable context or an existing on-demand doc or skill. When no
  suitable target exists, recommend creation through the `create` workflow instead of creating or moving files here.
- Preserve authority, safety, material exceptions, semantic completion criteria, exact commands and machine-consumed
  text, and clarity. Re-read the effective load chain after pruning to ensure no required constraint is orphaned or
  contradicted.

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
- Remove generic tutorials, historical authoring notes, file inventories, lists of installed skills, and command lists
  with no preference or warning.

Move subtree-specific rules to the deepest common ancestor where they apply. Promote duplicated child guidance only when
every affected child shares it. Recommend a missing nested AGENTS.md only for a distinct command, safety rule,
generated-file boundary, ownership rule, data constraint, or review requirement; route actual creation through the
`create` workflow.

Never delete an empty or obsolete AGENTS.md automatically. Report it as a deletion candidate, together with any sibling
CLAUDE.md symlink, and require explicit confirmation.

## CLAUDE.md Decisions

Create or refresh a sibling symlink only when CLAUDE.md is missing or already a symlink:

```sh
(cd "$dir" && ln -sf AGENTS.md CLAUDE.md)
```

Before writing, require `test -L "$dir/CLAUDE.md" || test ! -e "$dir/CLAUDE.md"`. A regular CLAUDE.md blocks only that
target; leave it untouched and report the conflict.

After changing placement or symlinks, rediscover affected targets and confirm no local constraint was orphaned.

## Context Doc Decisions

Polish selected context docs — conventions, command catalogs, data-format rules, workflow runbooks, and similar
reference material — wherever they live and whatever they are named:

- Verify commands, paths, flags, formats, environment variables, versions, and rules against the repository with the
  same rigor as AGENTS.md.
- Preserve each doc's audience, depth, structure, and voice; a deep reference stays a deep reference. Do not compress it
  to AGENTS.md terseness or inline it into AGENTS.md.
- Fix broken links between context docs, README.md, AGENTS.md, and skills. Do not move or rename docs.
- Recommend relocating guidance only when it is clearly misplaced, such as stable repo-wide rules living solely in a
  deep doc nothing links to; perform the move only with explicit confirmation.
- Report an obsolete doc whose central subject no longer exists as a deletion candidate; never delete or hollow it out.

## Project Skill Decisions

Only update existing project-installed skills under `.agents/skills`. A minimal factual fix may touch SKILL.md or its
existing bundled files. Never target catalog `skills/<name>/`, create skill or bundled files, or change a skill's
purpose or structure.

For each selected skill:

- Confirm frontmatter parses and `name` matches the directory. Fix only mechanical, unambiguous drift.
- Classify its declared write boundary. If it can never write repository files — it is read-only or reporting, writes
  only external or out-of-repository state, or writes only repository-local metadata — add `coordination: exempt` when
  absent. Add the standard body sentence near the top: `This skill is coordination-exempt: skip the ai-coord gate (\`git
  status\` / \`ai-coord status\` / \`ai-coord start\`) for this skill's own work.`
- If the skill may write repository files, omit `coordination`; remove a stale `coordination: exempt` field and its
  matching standard body sentence when repository evidence establishes that the exemption is unsafe. Do not invent
  another `coordination` value. Keep frontmatter fields alphabetized, with `description` last.
- Verify referenced `references/`, `scripts/`, `assets/`, and `examples/` paths relative to the skill directory.
- Read only the bundled files needed to verify paths, commands, flags, environment variables, versions, symbols,
  ownership, and repository conventions.
- Preserve structure and voice; use the smallest factual edit span.
- Leave third-party behavior and paths outside the repository unchanged unless current repository evidence
  authoritatively establishes the correction.
- Report an obsolete skill whose central subject no longer exists; do not delete or hollow it out.

## Finish

Run the completion checks and use the report contract from SKILL.md. Stop after the selected existing targets are
accurate and validated; do not create recommended context or perform adjacent cleanup.
