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
2. Do not use the project-local scaffolding in `agents-brain create-skill` for this catalog. Its catalog guard routes
   creation here to this repository-owned lifecycle.
3. Add a row to the skills table in `README.md`.
4. For external sources, complete [External Provenance](#external-provenance) in the same change.
5. Run `just skill-check` (SKILL.md invocation fields match `agents/openai.yaml`, README skill table, and dependencies).
6. Publish via `@publish-skills`.

## External Provenance

For an imported or forked whole external skill, add its upstream URL to the README's "Forked Skills" table. Create
`.agents/skill-upstreams/<name>.md` and add its entry to `.agents/skill-upstreams/README.md` in the same change. For
borrowed components or inspiration, add the provenance record and index entry without adding a whole-skill fork row.

Record each source repository and path, its relationship, and the known source revision and last reconciled base. Mark
unknown revisions explicitly. Include upstream-to-local path mappings, local customizations, and the safe merge
boundary. A source pin does not establish a whole-skill merge base. Inspiration and independent rewrites permit guidance
review, not automatic content merges.

Circadian's `jobs/rebase-upstream-skills.sh` discovers names from fresh `origin/main` on each weekly run. It takes the
union of "Forked Skills" rows and active `.agents/skill-upstreams/<name>.md` records, excludes `README.md`, and
deduplicates names. Do not add a per-skill scheduler list or run a live rebase to enroll a skill.

Before committing, verify any whole-skill table row, record, index link, and existing `skills/<name>/SKILL.md` agree.
After pushing, fetch `origin/main` and inspect its table and provenance record with `git show origin/main:<path>`.
Confirm that the name still matches its active skill path. Follow `@rebase-skill` for review and merge limits.
Enrollment does not approve an unknown base or expand a mapping.

## Rename (Catalog)

1. `git mv skills/<old> skills/<new>`.
2. Update the frontmatter `name:` field to `<new>`. Otherwise, the publisher's name-equals-directory guard fails the
   plan.
3. `rg` the repo for stale references: `$<old>` invocations, `skill-dependencies` entries naming `<old>`, and prose
   references in other skills or docs. Update every hit.
4. Update both `README.md` tables where applicable. Rename `.agents/skill-upstreams/<old>.md` when present. Update its
   local mappings and the provenance index entry to `<new>`. Preserve source paths and revision evidence.
5. Run `just skill-check`.
6. When publishing with an explicit commit range, pass **both** `--skill <old> --skill <new>`. The planner then removes
   the old install and adds the new one automatically.

## Delete (Catalog)

1. `git rm -r skills/<name>`.
2. Remove the skill's rows from both `README.md` tables where applicable. Remove its provenance record and index entry.
3. `rg` the repo for `skill-dependencies` entries and `$<name>` references in other skills. Update or remove them.
4. Publish. The remove group cleans global installs and the CLI lock.
5. Caveat: a CLI-lock entry whose `source` is not `PaulRBerg/agent-skills` is invisible to the planner. Remove such an
   install manually with the `skills` CLI version that `scripts/publish-skills.ts` pins in `skillsCli`:
   `bunx skills@<pinned-version> remove --global --skill <name> --yes`. Another version can rewrite the lock in a format
   that the planner rejects.

## Shelve (Catalog)

The `shelved` branch contains archived skills, a `README.md` table, and known provenance under
`.agents/skill-upstreams/`. Work on it in a temporary `git worktree`. Never switch the shared checkout.

1. Copy the committed skill verbatim into the worktree's `skills/<name>/`. Rename it only when asked. If renaming it,
   update `name:`.
2. Add the `README.md` skills row and any "Forked Skills" row. If the skill has a provenance record, copy it into
   `.agents/skill-upstreams/` on `shelved`. Update that branch's provenance index and mark the record as shelved.
3. Commit and push `shelved` before removing the active copy. Preserve archived skill files verbatim. Do not run
   Prettier, `just skill-check`, or other checks on those skill files. Format only changed archive documentation.
4. On `main`, apply the source cleanup from Delete (Catalog). Remove the skill's owned tests, provenance record, and
   index entry. Keep shelved skill provenance only on `shelved`, not in `.agents/skill-upstreams/` on `main`.
5. Run `just skill-check` on `main`. Commit and publish the removal through `@publish-skills`.

## Internal Skills (Create / Rename / Delete)

Internal skills are flat `.agents/internal-skills/<name>.md` files with only `name` + `description` frontmatter. See
`@publish-skills` or `@sync-skills` for the exact frontmatter shape. They have no README row or `agents/openai.yaml`.
They are never published.

- **Create**: Add the file. Do not add a `README.md` row or `agents/openai.yaml`.
- **Rename**: `git mv` the file. Update its `name:` frontmatter. Then update every `@<old-name>` reference across
  `AGENTS.md` and other internal skills. Update any `sync-skills` group member list that names the file.
- **Delete**: `git rm` the file. Then remove every `@<name>` reference. Remove any `sync-skills` group member list entry
  that names it.
