---
name: skill-lifecycle
description: Checklists for creating, renaming, and deleting catalog and internal skills in this repository.
---

# Skill Lifecycle

Checklists for the three lifecycle operations on skills owned by this repository: create, rename, delete. Catalog skills
(`skills/<name>/`) and internal skills (`.agents/internal-skills/<name>.md`) have different rules — do not mix them.

## Create (Catalog)

1. Scaffold `skills/<name>/` with `SKILL.md` and `agents/openai.yaml`. Set frontmatter per `@skill-authoring` (field
   order, invocation control, `metadata.install-targets`, `skill-dependencies`).
2. Do not use the `skill-writing` catalog skill for this: it scaffolds project skills in **other** repos, not this
   repo's own catalog.
3. Add a row to the skills table in `README.md`.
4. Run `ai-skillet doctor --root 'skills/<name>'`, `just skill-invocation-check` (SKILL.md invocation fields match
   `agents/openai.yaml`), and `just readme-skills-check`.
5. Publish via `@publish-skills`.

## Rename (Catalog)

1. `git mv skills/<old> skills/<new>`.
2. Update the frontmatter `name:` field to `<new>`. The publisher's name-equals-directory guard fails the plan
   otherwise.
3. `rg` the repo for stale references: `$<old>` invocations, `skill-dependencies` entries naming `<old>`, and prose
   references in other skills or docs. Update every hit.
4. Update the `README.md` row.
5. Run `ai-skillet doctor --root 'skills/<new>'`, `just skill-invocation-check`, `just skill-dependencies-check`, and
   `just readme-skills-check`.
6. When publishing with an explicit commit range, pass **both** `--skill <old> --skill <new>`; the planner then removes
   the old install and adds the new one automatically.

## Delete (Catalog)

1. `git rm -r skills/<name>`.
2. Remove the `README.md` row.
3. `rg` the repo for `skill-dependencies` entries and `$<name>` references in other skills; update or remove them.
4. Publish. The remove group cleans up global installs and the CLI lock.
5. Caveat: a CLI-lock entry whose `source` is not `PaulRBerg/agent-skills` is invisible to the planner. Remove such an
   install manually with `bunx skills remove --global --skill <name> --yes`.

## Internal Skills (Create / Rename / Delete)

Internal skills are flat `.agents/internal-skills/<name>.md` files with `name` + `description` frontmatter only (see
`@publish-skills` or `@sync-skills` for the exact frontmatter shape). They have no README row, no `agents/openai.yaml`,
and are never published.

- **Create**: add the file; do not add a `README.md` row or `agents/openai.yaml`.
- **Rename**: `git mv` the file, update its `name:` frontmatter, then update every `@<old-name>` reference across
  `AGENTS.md` and other internal skills, and any `sync-skills` group member list that names the file.
- **Delete**: `git rm` the file, then remove every `@<name>` reference and any `sync-skills` group member list entry
  that names it.
