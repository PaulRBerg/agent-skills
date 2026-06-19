---
name: sync-skills
description: Review and synchronize coupled skill files; align shared wording and helper contracts, fix drift, preserve skill-specific content.
---

# Sync Skills

Review skill files that intentionally share wording, policies, or helper contracts. Patch only real drift; preserve skill-specific behavior and examples.

## Scope

Default: run every sync group below. If the request names a group, file, or subset, run only that group.

Work only in the files listed for the selected groups.

## Sync Groups

### Code Workflow Skills

Files:

- `skills/code-polish/SKILL.md`
- `skills/code-review/SKILL.md`
- `skills/code-review/references/profiles/*.md`
- `skills/code-simplify/SKILL.md`

Treat these as in scope:

- The `## Scope Resolution` section: byte-identical across all three `SKILL.md` files.
- The `## Verification` section: byte-identical in `code-review` and `code-simplify`.
- `code-polish` embeds the same three-bullet verification check list plus "Name every skipped check and why." in workflow step `4) Final Verification`.
- The `### Residual Risks` subsection of each `## Report`.
- The Report framing sentence under each `## Report` heading: "Use these section headings, in this order. Omit sections that do not apply — do not number them and do not leave gaps or placeholders."
- The `## Stop Conditions` intro line: "Stop and ask for direction when:"
- The workflow closing sentence: "Produce the Report section below."
- The paths bullet in `## Arguments`: "- Paths, patterns, a commit/range, or a scope phrase: used in Scope Resolution step 2."
- Flag-bullet shape in `## Arguments`: one imperative effect sentence; repeatable flags end with "Repeatable."; the last bullet is `- Default: <behavior sentence>.`
- `argument-hint` shape in frontmatter: `[paths]` first, then flags A→Z.
- Profile trigger sentences: `Load when the diff touches <X>.`; exception: `naming.md` keeps its sequencing trigger.

Treat these as out of scope unless multiple files already carry the same concept and only wording drifted:

- Per-skill workflow bodies, Operating Rules, Core Review Checks, Profile Dispatch, Severity Model, Evidence Rules, Simplification Heuristics, Anti-Patterns, Running Sub-Skills, Stop Conditions bullets, and the completion gate.
- Profile checks, severities, per-profile Evidence Expectations, and naming's Guardrail.
- Frontmatter `description` fields.

### Commit Message Format Helpers

Files:

- `skills/commit/scripts/prepare-commit.sh`
- `skills/commit/scripts/select-message-format.sh`

Treat these as in scope:

- The ordered literal entries in each `always_natural_language_repos` array. The variable names may differ, but the repo path list must stay identical.
- The rule that `--natural` forces `natural`.
- The default fallback to `conventional` when the target repo is not in the always-natural list.

Treat these as out of scope unless the request explicitly names them:

- Atomic staging behavior in `prepare-commit.sh`.
- Backwards-compatible argument parsing in `select-message-format.sh`.
- Adding a shared sourced helper file.

## Workflow

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git repository.
2. Resolve selected sync groups once. Do not broaden the group list after reading files unless the user asks.
3. Read the selected files and compare only the in-scope shared blocks or helper contracts.
4. When drift exists, normalize all copies to one phrasing or value set. Reuse the clearest wording already present. Keep `code-review` as the tiebreaker only when phrasings are equally clear.
5. Prefer minimal patches. Do not rewrite whole sections just to make them symmetrical if the remaining differences are skill-specific.
6. If no drift exists, make no edits and report that the selected groups are already aligned.

## Verification

After editing Markdown, run from the repo root:

```bash
just mdformat-write
just mdformat-check
```

For the Code Workflow Skills group, also run:

```bash
bash skills/code-review/scripts/validate-references.sh
```

Confirm byte-identity of the `## Scope Resolution` section:

```bash
bash <<'EOF'
set -euo pipefail
for s in code-simplify code-polish; do
  diff <(awk '/^## Scope Resolution$/{f=1;print;next} f&&/^## /{exit} f' skills/code-review/SKILL.md) \
       <(awk '/^## Scope Resolution$/{f=1;print;next} f&&/^## /{exit} f' "skills/$s/SKILL.md") \
    && echo "OK: $s Scope Resolution matches code-review"
done
EOF
```

Repeat the same extraction diff with `/^## Verification$/` for `code-review` against `code-simplify`, and with `/^### Residual Risks$/` for all three. The residual-risk extraction stops at `## Stop Conditions`.

For the Commit Message Format Helpers group, run:

```bash
bash -n skills/commit/scripts/prepare-commit.sh skills/commit/scripts/select-message-format.sh
bash <<'EOF'
set -euo pipefail
extract_repos() {
  awk '
    /always_natural_language_repos=\(/ { in_list=1; next }
    in_list && /^[[:space:]]*\)/ { exit }
    in_list {
      line=$0
      sub(/^[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*$/, "", line)
      if (line != "") print line
    }
  ' "$1"
}
diff -u \
  <(extract_repos skills/commit/scripts/prepare-commit.sh) \
  <(extract_repos skills/commit/scripts/select-message-format.sh)
EOF
```

Re-read touched sections and confirm selected groups now match on shared wording or helper data and still differ only where their workflows require it.
