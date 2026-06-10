---
name: sync-code-skills
description: Review and synchronize the shared blocks of the code-polish, code-review, and code-simplify skills; align shared wording, fix divergences, preserve skill-specific content.
---

# Sync Code Skills

Review the three `code-*` skills, detect drift in the shared blocks, and patch only divergences that are not skill-specific.

Work only in these files:

- `skills/code-polish/SKILL.md`
- `skills/code-review/SKILL.md`
- `skills/code-review/references/profiles/*.md`
- `skills/code-simplify/SKILL.md`

## Workflow

1. Read the three `SKILL.md` files and diff the shared blocks across them.
2. Treat these as in scope (byte-identical across all three unless noted):
   - the `## Scope Resolution` section
   - the `## Verification` section (full section in `code-review` and `code-simplify`; `code-polish` instead embeds the three-bullet check list plus the sentence "Name every skipped check and why." in its workflow step "4) Final Verification")
   - the `### Residual Risks` subsection of each `## Report`
   - the Report framing sentence under each `## Report` heading ("Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders.")
   - the `## Stop Conditions` intro line ("Stop and ask for direction when:")
   - the Workflow closing sentence ("Produce the Report section below.") — present in every skill's final workflow step
   - the paths bullet in `## Arguments` ("- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2.")
   - flag-bullet shape in `## Arguments`: one imperative effect sentence; repeatable flags end with "Repeatable."; the last bullet is `- Default: <behavior sentence>.`
   - `argument-hint` shape in frontmatter: `[paths]` first, then flags A→Z
   - profile trigger sentences: `Load when the diff touches <X>.` — exception: `naming.md` keeps its sequencing trigger ("Load last, after correctness and security checks; ...")
3. Treat these as out of scope unless multiple files already carry the same concept and only wording drifted:
   - per-skill workflow bodies, Operating Rules, Core Review Checks, Profile Dispatch, Severity Model, Evidence Rules, Simplification Heuristics, Anti-Patterns, Running Sub-Skills, Stop Conditions bullets, and the completion gate
   - profile checks, severities, per-profile Evidence Expectations, and naming's Guardrail
   - frontmatter `description` fields
4. When a shared block has diverged, normalize all skills to one phrasing. Reuse the clearest wording already present. Keep `code-review` as the tiebreaker only when phrasings are equally clear. Do not introduce new policy unless it is necessary to remove an actual ambiguity.
5. Prefer minimal patches. Do not rewrite whole sections just to make them look symmetrical if the remaining differences are skill-specific.
6. If no drift exists, make no edits and report that the shared blocks are already aligned.

## Verification

After editing, run from the repo root:

```bash
just mdformat-write
just mdformat-check
bash skills/code-review/scripts/validate-references.sh
```

Then confirm byte-identity of the `## Scope Resolution` section. The extraction stops before the next `## ` heading, which legitimately differs per file:

```bash
for s in code-simplify code-polish; do
  diff <(awk '/^## Scope Resolution$/{f=1;print;next} f&&/^## /{exit} f' skills/code-review/SKILL.md) \
       <(awk '/^## Scope Resolution$/{f=1;print;next} f&&/^## /{exit} f' "skills/$s/SKILL.md") \
    && echo "OK: $s Scope Resolution matches code-review"
done
```

Repeat the same extraction diff with `/^## Verification$/` (compare `code-review` against `code-simplify` only) and with `/^### Residual Risks$/` (all three; the stop pattern `/^## /` does not match `### ` headings, so the extraction ends at `## Stop Conditions`).

Re-read the touched sections and confirm the three skills now match on shared wording and still differ only where the workflows require it.
