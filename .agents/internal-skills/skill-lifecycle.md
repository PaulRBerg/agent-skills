---
name: skill-lifecycle
description: Checklists for creating, renaming, deleting, and shelving catalog and internal skills in this repository.
---

# Skill Lifecycle

Use these checklists to create, rename, delete, or shelve skills owned by this repository. Catalog skills
(`skills/<name>/`) and internal skills (`.agents/internal-skills/<name>.md`) have different rules. Do not mix them.

## Create (Catalog)

1. Scaffold `skills/<name>/` with `SKILL.md` and `agents/openai.yaml`. Set frontmatter per `@skill-authoring`, including
   field order, invocation control, `metadata.install-targets`, and `skill-dependencies`.
2. Do not use the `skill-writing` catalog skill for this. It scaffolds project skills in **other** repos, not this
   repo's own catalog.
3. Add a row to the skills table in `README.md`.
4. Run `just skill-check` (SKILL.md invocation fields match `agents/openai.yaml`, README skill table, and dependencies).
5. Publish via `@publish-skills`.

## Rename (Catalog)

1. `git mv skills/<old> skills/<new>`.
2. Update the frontmatter `name:` field to `<new>`. Otherwise, the publisher's name-equals-directory guard fails the
   plan.
3. `rg` the repo for stale references: `$<old>` invocations, `skill-dependencies` entries naming `<old>`, and prose
   references in other skills or docs. Update every hit.
4. Update the `README.md` row.
5. Run `just skill-check`.
6. When publishing with an explicit commit range, pass **both** `--skill <old> --skill <new>`. The planner then removes
   the old install and adds the new one automatically.

## Delete (Catalog)

1. `git rm -r skills/<name>`.
2. Remove the `README.md` row.
3. `rg` the repo for `skill-dependencies` entries and `$<name>` references in other skills. Update or remove them.
4. Publish. The remove group cleans global installs and the CLI lock.
5. Caveat: a CLI-lock entry whose `source` is not `PaulRBerg/agent-skills` is invisible to the planner. Remove such an
   install manually with `bunx skills remove --global --skill <name> --yes`.

## Shelve (Catalog)

The `shelved` branch is a skills-only catalog. It contains `skills/<name>/` plus a `README.md` table row per skill. Work
on it in a temporary `git worktree`. Never switch the shared checkout.

1. Copy the committed skill verbatim into the worktree's `skills/<name>/`. Rename it only when asked. If renaming it,
   update `name:`.
2. Add the `README.md` table row.
3. Commit and push `shelved`. Do not run Prettier, `just skill-check`, or other checks on shelved skills. They are
   archived as-is.

## Internal Skills (Create / Rename / Delete)

Internal skills are flat `.agents/internal-skills/<name>.md` files with only `name` + `description` frontmatter. See
`@publish-skills` or `@sync-skills` for the exact frontmatter shape. They have no README row or `agents/openai.yaml`.
They are never published.

- **Create**: Add the file. Do not add a `README.md` row or `agents/openai.yaml`.
- **Rename**: `git mv` the file. Update its `name:` frontmatter. Then update every `@<old-name>` reference across
  `AGENTS.md` and other internal skills. Update any `sync-skills` group member list that names the file.
- **Delete**: `git rm` the file. Then remove every `@<name>` reference. Remove any `sync-skills` group member list entry
  that names it.
