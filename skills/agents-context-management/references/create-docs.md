# Create Docs Workflow

Create missing README.md and AGENTS.md context from repository evidence. Regenerate existing targets only with `--force`
or an equally explicit overwrite instruction. Never create skills.

Success means each selected package root has the requested human and agent context, every created AGENTS.md has a safe
companion CLAUDE.md symlink where possible, and generated claims pass repository-defined validation.

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
blocked. Without overwrite authority, skip existing README.md and AGENTS.md files and report them; do not silently route
them through `polish`.

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
- Include non-obvious architecture, style, naming, generated-file, ownership, safety, privacy, credential, deployment,
  financial, data-handling, and review constraints supported by evidence.
- Exclude generic tool tutorials, long directory trees, and package-script inventories that add no preference or
  warning.

Parent files hold shared defaults; nested files contain only local deltas.

## Create CLAUDE.md Symlinks

For each created AGENTS.md, create a sibling compatibility symlink:

```sh
(cd "$dir" && ln -sf AGENTS.md CLAUDE.md)
```

Write only when CLAUDE.md is missing or already a symlink. A regular CLAUDE.md blocks only that symlink target; leave it
untouched and report the conflict.

## Handle CONTRIBUTING.md

Never edit CONTRIBUTING.md. If it exists next to a target, put only stable, relevant contribution guidance in AGENTS.md
and advise the user to merge any remaining useful instructions manually before deleting CONTRIBUTING.md.

## Finish

Run the completion checks and use the report contract from SKILL.md. In dry-run mode, show selected paths and concise
section-level previews or diffs. Stop after the requested files are created or regenerated and validated; do not polish
unrelated existing context.
