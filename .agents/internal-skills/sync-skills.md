---
name: sync-skills
description:
  Review and synchronize coupled skill files; align shared wording and helper contracts, fix drift, preserve
  skill-specific content.
---

# Sync Skills

Review skill files that intentionally share wording, policies, or helper contracts. Patch only real drift; preserve
skill-specific behavior and examples.

## Scope

Default: run every sync group below. If the request names a group, file, or subset, run only that group.

Work only in the files listed for the selected groups.

## Sync Groups

### Commit Message Format Helpers

Files:

- `skills/commit/scripts/prepare-commit.sh`
- `skills/commit/scripts/select-message-format.sh`

Treat these as in scope:

- The ordered literal entries in each `always_natural_language_repos` array. The variable names may differ, but the repo
  path list must stay identical.
- The rule that `--natural` forces `natural`.
- The default fallback to `conventional` when the target repo is not in the always-natural list.

Treat these as out of scope unless the request explicitly names them:

- Atomic staging behavior in `prepare-commit.sh`.
- Backwards-compatible argument parsing in `select-message-format.sh`.
- Adding a shared sourced helper file.

## Workflow

1. Verify repository context: `git rev-parse --git-dir`. If this fails, stop and tell the user to run from a git
   repository.
2. Resolve selected sync groups once. Do not broaden the group list after reading files unless the user asks.
3. Read the selected files and compare only the in-scope shared blocks or helper contracts.
4. When drift exists, normalize all copies to one phrasing or value set. Reuse the clearest wording already present.
5. Prefer minimal patches. Do not rewrite whole sections just to make them symmetrical if the remaining differences are
   skill-specific.
6. If no drift exists, make no edits and report that the selected groups are already aligned.

## Verification

After editing Markdown, run from the repo root:

```bash
just prettier-write
just prettier-check
```

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

Re-read touched sections and confirm selected groups now match on shared wording or helper data and still differ only
where their workflows require it.
