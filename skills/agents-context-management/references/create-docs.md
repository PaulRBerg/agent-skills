# Create Docs Workflow

Use this workflow to create missing README.md and AGENTS.md files. It may create sibling CLAUDE.md symlinks for AGENTS.md files. It must not create skills; for that, refer to `skills/create-skill`.

## Inputs

- Repo root.
- Package roots: repo root plus manifest-bearing directories.
- Nearest manifests and metadata files.
- Existing README.md / AGENTS.md files for overwrite checks.
- Optional user-provided description.

## Steps

1. Parse `path`, `--root-only`, `--dry-run`, `--minimal`, `--full` / `--thorough`, `--force`, and optional guided description.
2. Run the guard rails from `SKILL.md` and snapshot `git status --short`.
3. Enumerate package roots. Restrict by `path` or `--root-only`.
4. For each package root, determine which files are missing: README.md, AGENTS.md, and CLAUDE.md symlink.
5. If existing README.md or AGENTS.md files would be overwritten, require `--force` or explicit user confirmation. For multi-target sweeps, confirm once for the batch.
6. Generate README.md for humans and AGENTS.md for agents from the same analysis, keeping the audience split strict.
7. Create sibling CLAUDE.md symlinks for created AGENTS.md files.
8. Recommend nested AGENTS.md / CLAUDE.md pairs when analysis finds distinct subtree rules outside the initial package-root set.
9. Run the narrowest formatter/checker and report grouped results.

## Package Roots

Create files only at package roots: the repo root plus directories containing one of these manifests:

- `package.json`
- `Cargo.toml`
- `pyproject.toml`
- `setup.py`
- `go.mod`
- `foundry.toml`
- `Gemfile`
- `composer.json`

Do not create files in arbitrary leaf directories. Do not create `.agents/skills` directories or skill files.

## README.md Generation

README.md is human-facing. Generate:

- Title and a short description from metadata or the guided description.
- Verified badges and links only.
- Optional references, related projects, citations, funding, changelog, docs, package registry, demo, or license sections when the files or metadata exist.
- A short contributing pointer to sibling `AGENTS.md`.

Do not include install/build/test/lint/dev/deploy commands unless the repo is an operator-run setup repo or the user explicitly asks for a setup guide. Even then, keep the guide short and task-focused; developer workflow commands belong in AGENTS.md.

## AGENTS.md Generation

AGENTS.md is agent-facing. Generate concise, imperative guidance:

- Stack and package manager.
- Commands with preferred runners and any non-obvious order or side effects.
- Code style and architecture constraints that are not obvious from filenames.
- Generated-file warnings and ownership boundaries.
- Safety, secrets, data-handling, deployment, or financial constraints.
- Contribution workflow when discoverable.
- A `CONTRIBUTING.md` merge stub if that file exists next to the target.

Avoid long directory trees, generic tool tutorials, or package-script inventories that add no preference or warning. Create child AGENTS.md files only for real local deltas.

## Nested Context Recommendations

After generating package-root files, scan for subtrees whose rules would be noisy or misleading in the parent:

- Distinct task runners or package managers.
- Generated files with different ownership.
- Separate deployment, data, credential, or safety constraints.
- Different review or testing requirements.
- Distinct app/library/contract boundaries in a monorepo.

If such a subtree lacks AGENTS.md, recommend creating a nested AGENTS.md and sibling CLAUDE.md symlink. Create it during this workflow only when the user requested broad context creation and the scope is clear. Otherwise report the recommendation.

## CLAUDE.md Symlinks

For each created AGENTS.md, create a sibling symlink:

```sh
(cd "$dir" && ln -sf AGENTS.md CLAUDE.md)
```

Before writing, check whether `$dir/CLAUDE.md` exists. If it is a regular file, stop for that target and report `✗`; do not overwrite it.

## CONTRIBUTING.md

Never edit CONTRIBUTING.md. If it exists next to a target:

1. Add or keep a concise Contribution Workflow stub in sibling AGENTS.md.
2. Report an advisory recommending that the user merge CONTRIBUTING.md into AGENTS.md and delete CONTRIBUTING.md after review.

## Verification

Check whether the repo defines Markdown lint/format rules (for example a `just` recipe, npm/package script, `.markdownlint.json`, `.prettierrc`, or lint-staged config). If found, apply them; otherwise report the skip. For `--dry-run`, report the commands that would run.

## Report

Group by relative package root:

```text
### packages/core
✓ Created README.md
  - Human-facing overview and AGENTS.md pointer
✓ Created AGENTS.md
  - Commands, style, and local constraints
✓ Created CLAUDE.md symlink

⚠ CONTRIBUTING.md detected
  - Merge into packages/core/AGENTS.md, then delete CONTRIBUTING.md after review.
```

Close with a tally: `Created 4 files, skipped 1 existing file, 1 advisory.`
