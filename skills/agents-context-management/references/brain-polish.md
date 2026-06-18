# Polish Workflow

Use this workflow to update existing context as one surface: README.md, AGENTS.md with sibling CLAUDE.md symlinks, and existing project-installed `.agents/skills/<name>/SKILL.md` files. Polish means factual repair, noise removal, and better placement. It does not mean creating README.md, AGENTS.md, skills, or broad restyling.

## Inputs

- Existing `README.md` files in scope.
- Existing `AGENTS.md` files in scope.
- Sibling `CLAUDE.md` symlinks in scope.
- Existing repo-local `.agents/skills/<name>/SKILL.md` files and their bundled files.
- Nearest manifests, task runners, lock files, lint configs, CI configs, and relevant source files needed to verify claims.

## Steps

01. Parse `path`, `skill-name ...`, `--root-only`, `--dry-run`, `--preserve`, `--minimal`, and `--thorough` / `--full`.
02. Run the guard rails from `SKILL.md` and snapshot `git status --short`.
03. Discover README.md, AGENTS.md, CLAUDE.md symlinks, and existing project skills. Apply `path`, `--root-only`, and skill-name filters.
04. Detect `CONTRIBUTING.md` next to each README.md or AGENTS.md target. Do not edit it; report a recommendation to merge it into sibling AGENTS.md and delete it after review.
05. Read each target and extract verifiable claims: paths, commands, scripts, recipes, target names, env vars, file ownership, generated-file warnings, default branch, CI paths, and local conventions.
06. Verify claims against the target directory, its nearest enclosing manifest, and repo-wide config where appropriate.
07. Update README.md files for human-facing accuracy.
08. Update and polish AGENTS.md hierarchy and CLAUDE.md symlinks.
09. Update existing project skills with factual corrections only.
10. Run the narrowest formatter/checker and report grouped results.

If a run would rewrite more than a handful of files, first list planned targets and treat the first pass as `--dry-run` unless the user already requested broad writes.

## README.md Rules

README.md is for humans browsing the repo, package registry, or project page.

Keep:

- Project name, short description, and badges.
- Documentation, homepage, demo, package registry, changelog, citation, reference, funding, and license links that exist.
- A short contributing pointer to sibling `AGENTS.md`.
- A short operator-run setup guide only for dotfiles, infra, homelab, personal tooling, or explicit user requests.

Remove or move to AGENTS.md:

- Install, build, test, lint, dev, deploy, and release commands.
- `just` recipes, package scripts, Makefile targets, and CI command inventories.
- Project structure trees.
- API reference and configuration manuals.
- Developer workflow and review rules.

When `--preserve` is present, keep custom human prose such as About, Background, References, Related Projects, Credits, and Acknowledgments unless it is factually wrong. Refresh metadata-driven sections and remove non-exempt technical sections.

Use plain prose. Do not add marketing copy, placeholders, or links that have not been verified.

## AGENTS.md Rules

AGENTS.md is for agents and developers running commands. Keep it terse, imperative, and scoped to its directory tree.

Keep or sharpen:

- Commands with preferred order, side effects, environment needs, or failure guidance.
- Coding preferences, architecture constraints, review standards, naming rules, and generated-file ownership.
- Safety, privacy, credential, financial, deployment, and data-handling rules.
- Repo-specific speed traps, flaky checks, migration constraints, shell quirks, and external system notes.
- Contribution workflow and branch/review conventions when discoverable or already present.

Remove or compress:

- Directory trees and file-by-file inventories that only restate names discoverable with `rg --files`.
- Generic textbook explanations of languages, frameworks, package managers, git, or Markdown.
- Package script or recipe inventories copied verbatim without preference, ordering, side effects, or failure guidance.
- Lists of installed skills or available skills.
- Historical authoring notes, implementation logs, and why-a-file-was-created commentary.

Required sections are not mandatory by title. Prefer a small number of useful sections over boilerplate. Commands can be exhaustive when that prevents drift, but do not duplicate commands across parent and child AGENTS.md files unless both scopes genuinely need them.

## AGENTS.md Placement

Treat each AGENTS.md as scoped instructions for its directory tree.

- Move subtree-specific guidance to the deepest common ancestor where it applies.
- Promote duplicated nested guidance to the nearest shared parent only when it applies across those children.
- Recommend nested AGENTS.md only when the subtree has distinct commands, safety rules, generated files, ownership boundaries, data-handling rules, or review constraints. Do not create it during `polish`; route actual creation through the `create` workflow.
- Delete an AGENTS.md only when no useful guidance remains after pruning. Delete only a sibling CLAUDE.md symlink that resolves to that deleted AGENTS.md.
- Leave regular CLAUDE.md files untouched.
- After moving, adding, or deleting AGENTS.md files or companion symlinks, rediscover targets and confirm no local constraint was orphaned.

Create or refresh a sibling symlink with:

```sh
(cd "$dir" && ln -sf AGENTS.md CLAUDE.md)
```

Before writing `CLAUDE.md`, check `test -L "$dir/CLAUDE.md" || test ! -e "$dir/CLAUDE.md"`. Stop for that file if it is a regular file.

## Project Skill Rules

Only update existing project-installed skills discovered under `.agents/skills`. Never create, install, delete, or rename skills. Never target catalog `skills/<name>/` directories.

For each discovered skill:

1. Resolve its project root:

   ```sh
   project_root="${skill_md%/.agents/skills/*}"
   ```

2. Verify frontmatter parses and `name` matches the skill directory. Fix only mechanical, unambiguous frontmatter drift.

3. Verify every `references/`, `scripts/`, `assets/`, and `examples/` path mentioned in SKILL.md exists relative to the skill directory.

4. Read SKILL.md and referenced files needed for claim verification.

5. Fix only factual drift: paths, commands, flags, env vars, versions, symbols, generated-file ownership, and repo conventions.

6. Preserve the skill's structure and voice. Use the smallest edit span.

7. Leave external URLs, third-party API behavior, and paths outside the repo untouched unless the repo itself owns the claim.

8. Flag obsolete skills when their central subject no longer exists. Suggest shelving or deleting; do not hollow them out.

Bundled-file reference check:

```sh
rg -o '\b(references|scripts|assets|examples)/[A-Za-z0-9][A-Za-z0-9._/-]*' "$skill_dir/SKILL.md" \
| sort -u \
| while IFS= read -r rel; do
    test -e "$skill_dir/$rel" || printf 'missing: %s\n' "$rel"
  done
```

## Verification

Prefer host repo recipes, inspecting unclear recipes first:

```sh
just mdformat-write
just mdformat-check
```

If skill frontmatter or `agents/openai.yaml` changed in a skills catalog, also run:

```sh
just skill-invocation-check
```

If no formatter/checker exists, report the skip. For `--dry-run`, report the commands that would run.

## Report

Use this shape:

```text
### Scope
README.md: <count> (<paths>)
AGENTS.md: <count> (<paths>)
CLAUDE.md symlinks: <count> (<paths>)
Project skills: <count> (<paths or "none">)

### Changes
✓ <path>
  - <concrete change>

⚠ <path>
  - <advisory>

### Verification
✓ just mdformat-write
✓ just mdformat-check

### Residual Risks
None.
```

For `--dry-run`, open with `## Planned Changes` and show target paths plus concise diffs or section-level previews.
