# Create or Regenerate Context

Use these placement and generation rules within `maintain` when missing context is warranted or regeneration is
requested. Create README.md and AGENTS.md from repository evidence. Regenerate existing targets only with `--force` or
an equally explicit overwrite instruction. Create other context docs only on explicit request. Skill creation follows
`references/maintain-skills.md` from this skill.

Success means each selected package root has the requested human and agent context, CLAUDE.md handling matches the
installed Claude Code (see Claude Code Compatibility in SKILL.md), and generated claims pass repository-defined
validation.

## Select Targets

Package roots are the repository root and directories containing one of these manifests:

- `package.json`
- `Cargo.toml`
- `pyproject.toml`
- `setup.py`
- `go.mod`
- `foundry.toml`
- `Gemfile`
- `composer.json`

Create README.md only at package roots. Create package-root AGENTS.md files there as well. Apply `path` and
`--root-only` before analyzing targets.

Nested AGENTS.md files may also be created when the user explicitly requests broad context creation and a subtree has a
distinct command runner, generated-file boundary, ownership rule, deployment or data constraint, safety requirement, or
review workflow. Otherwise, report the recommendation without writing it. Never create README.md in an arbitrary leaf
directory.

For each selected target, classify README.md, AGENTS.md, and CLAUDE.md as missing, reusable, safely replaceable, or
blocked. Maintain existing README.md and AGENTS.md with targeted edits under `references/maintain.md`; regenerate them
only with overwrite authority. A request restricted to creating missing files leaves existing files untouched.

## Ground the Content

Derive claims from the nearest manifests and metadata, task runners, lock files, CI and lint configuration,
generated-file notices, and relevant source boundaries. Use a user-provided description when present, but verify any
factual claims it adds.

Do not invent project purpose, badges, links, commands, conventions, ownership, or safety rules. When evidence is
missing, narrow the generated document instead of guessing.

## Generate README.md

Keep README.md human-facing:

- Add a title and short factual description.
- Add only verified documentation, homepage, demo, package, changelog, citation, funding, reference, or license links
  that materially help readers.
- Add a short contributing pointer to sibling AGENTS.md.
- Include a short operator-run setup guide only for dotfiles, infrastructure, homelab, personal tooling, or an explicit
  setup request.

Do not add developer command inventories, directory trees, configuration manuals, contribution rules, marketing copy, or
placeholders.

## Generate AGENTS.md

Keep AGENTS.md concise, imperative, and scoped:

- Name the stack and preferred package manager only when useful for choosing commands.
- Include commands whose runner, order, side effects, environment, or failure behavior matters.
- Include non-obvious architecture, style, naming, generated-file, ownership, safety, external-disclosure, credential,
  deployment, financial, recipient-scoped data-handling, and review constraints supported by evidence.
- Exclude generic tool tutorials, long directory trees, and package-script inventories that add no preference or
  warning.
- When the user requests continuous repo-local skill maintenance, include the standing instruction from this skill's
  `references/maintain-skills.md`, adapted to the repository's ownership and lifecycle rules.

Parent files hold shared defaults; nested files contain only local deltas.

## Handle CLAUDE.md

Run the version check from Claude Code Compatibility in SKILL.md. When `agents_md_native=true`, create no CLAUDE.md
symlinks and delete any existing symlink to a sibling AGENTS.md in the selected tree. Otherwise, create a sibling
compatibility symlink for each created AGENTS.md:

```sh
(cd "$dir" && ln -sfn AGENTS.md CLAUDE.md)
```

Write only when CLAUDE.md is missing or already a symlink. A regular CLAUDE.md blocks only that symlink target; leave it
untouched and report the conflict.

## Create Context Docs on Request

Create a Markdown context doc outside the default set — a conventions file, command catalog, data-format reference, or
workflow runbook — only when the user explicitly names its path and purpose. Ground its content in repository evidence
like any other target, keep it scoped to that purpose, and link it from the nearest AGENTS.md or README.md when that
improves discoverability. Do not scan for missing context docs; at most report a recommendation without writing it.

## Handle CONTRIBUTING.md

Never edit CONTRIBUTING.md. If it exists next to a target, put only stable, relevant contribution guidance in AGENTS.md
and advise the user to merge any remaining useful instructions manually before deleting CONTRIBUTING.md.

## Finish

Return to the shared completion checks and report contract in SKILL.md. In dry-run mode, show selected paths and concise
section-level previews or diffs. Keep all creation and regeneration within the selected maintenance scope.
