# Update Project Skills Workflow

Workflow for verifying and fixing the project-scoped agent skills installed under a repo's `.agents/skills/` directories. It runs on **every** `.agents/skills/<name>/SKILL.md` physically inside the current git repo, at any depth; each skill is verified against its own project root. The repo is the source of truth for facts; the skill is the source of truth for its own voice and structure.

## Workflow

### Step 1: Parse Arguments

Supported arguments:

- `skill-name ...` (positional): restrict the run to the named skills, matched against the skill directory name (`.agents/skills/<name>/`). A name matching several nested locations selects all of them. Zero matches → report `✗ <name> not found` and list the discovered skill names.
- `--dry-run`: show every planned change without writing files.

Mode behavior:

- If `--dry-run`: emit a `## Planned Changes` preview (Step 10) and write nothing.
- Otherwise: apply fixes in place; rely on git for recovery (no `*.backup` files).
- Unrecognized flags: fall back to default mode and add a one-line `⚠` note to the final report.

### Step 2: Guard Rails

Run before anything else. Any abort stops the whole workflow — there is no fallback directory.

```sh
cwd="$(pwd -P)"
case "$cwd" in
  /) printf 'abort: refusing to run at the filesystem root\n' >&2; exit 1 ;;
  "$HOME/.agents"|"$HOME/.agents/"*|"$HOME/.claude"|"$HOME/.claude/"*)
    printf 'abort: refusing to run under ~/.agents or ~/.claude\n' >&2; exit 1 ;;
esac
repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  printf 'abort: not inside a git repository\n' >&2; exit 1; }
case "$repo_root" in
  /|"$HOME") printf 'abort: unsupported repo root: %s\n' "$repo_root" >&2; exit 1 ;;
  "$HOME/.agents"|"$HOME/.agents/"*|"$HOME/.claude"|"$HOME/.claude/"*)
    printf 'abort: repo root is under ~/.agents or ~/.claude\n' >&2; exit 1 ;;
esac
```

### Step 3: Enumerate Targets

```sh
fd --glob --full-path --hidden --no-ignore --follow --type f \
   --exclude .git --exclude .claude --exclude node_modules --exclude vendor \
   --exclude dist --exclude build --exclude out --exclude target \
   --exclude .next --exclude .venv --exclude coverage \
   '**/.agents/skills/*/SKILL.md' "$repo_root" \
| while IFS= read -r p; do
    dir="$(cd "${p%/SKILL.md}" 2>/dev/null && pwd -P)" || continue
    case "$dir" in
      "$HOME/.agents/"*|"$HOME/.claude/"*) continue ;;
      "$repo_root"/*) printf '%s/SKILL.md\n' "$dir" ;;
    esac
  done \
| awk '!seen[$0]++'
```

- `--no-ignore`: repos often gitignore `.agents/`.
- `--hidden` + `--follow`: reach the dot-dir and traverse `.agents/skills` dirs symlinked across packages.
- `--exclude .claude`: `.claude/skills` dirs are never enumerated.
- The realpath stage drops anything resolving under `~/.agents` or `~/.claude` (skill dir symlinked to a global install → `⊘` skip) or escaping the repo; the `awk` stage dedupes aliases of the same physical directory.

Name filter (when positional names were given):

```sh
... | grep -E '/(name1|name2)/SKILL\.md$'
```

Duplicate names in different `.agents/skills` locations are independent targets; disambiguate them in the report by their location path.

Zero results overall → `⊘ No project skills found under .agents/skills in this repo.` and stop successfully. This is the expected outcome inside a skills-catalog repo, whose `skills/` tree never matches the glob.

If `fd` is unavailable, fall back to `find` and pipe it through the same realpath/dedupe stage:

```sh
find -L "$repo_root" -type f -path '*/.agents/skills/*/SKILL.md' \
  -not -path '*/.git/*' -not -path '*/.claude/*' -not -path '*/node_modules/*' \
  -not -path '*/vendor/*' -not -path '*/dist/*' -not -path '*/build/*' \
  -not -path '*/out/*' -not -path '*/target/*' -not -path '*/.next/*' \
  -not -path '*/.venv/*' -not -path '*/coverage/*'
```

### Step 4: Resolve Project Root

Each discovered skill belongs to the package that installed it:

```sh
project_root="${skill_md%/.agents/skills/*}"
```

Verification order for every claim: the skill dir itself (bundled files) → `project_root` → `$repo_root` (repo-wide claims only: default branch, CI files, root configs). Never verify a nested package's skill against another package's manifests.

Command sources are the `justfile` and `package.json` scripts at `project_root`, plus lock files for package-manager detection (`pnpm-lock.yaml` → pnpm, `yarn.lock` → yarn, `bun.lock`/`bun.lockb` → bun, `package-lock.json` → npm).

### Step 5: Skill-Internal Integrity

Before verifying claims against the repo, verify the skill against itself.

**Frontmatter.** The YAML frontmatter must parse and declare `name` and `description`:

- `name` ≠ directory name → fix `name` to the directory name.
- Unparseable YAML → attempt only mechanical, unambiguous repairs (unclosed quote, missing `---` terminator). If still unparseable → `✗ skipped: unparseable frontmatter`, and make no content edits to that skill.
- `description` drifted from what the body actually does → conservative factual fix; keep the original phrasing wherever it is still true.

**Bundled-file references.** Every `references/`, `scripts/`, `assets/`, `examples/` path mentioned in SKILL.md must exist relative to the skill dir:

```sh
rg -o '\b(references|scripts|assets|examples)/[A-Za-z0-9][A-Za-z0-9._/-]*' "$skill_dir/SKILL.md" \
| sort -u \
| while IFS= read -r rel; do
    test -e "$skill_dir/$rel" || printf 'missing: %s\n' "$rel"
  done
```

This catches both Markdown links and inline-code mentions. File moved within the skill → relink the mention; file gone → `⚠` advisory (do not delete the mention's surrounding section).

**Orphans.** Bundled files that exist on disk but are never mentioned in SKILL.md → `⚠` advisory only; never delete them.

**Global paths.** `~/...` references inside skill docs are legitimate (project skills may point at global tooling); leave them as-is.

### Step 6: Extract Verifiable Claims

Read the skill's `SKILL.md` and every file under its `references/`. Extract claims that the repo can confirm or refute:

- File and directory paths
- Directory trees
- Commands: `just` recipes, `package.json` scripts, raw CLI invocations
- CLI flags of repo-owned scripts and tools
- Environment variables
- Code symbols and APIs quoted in fenced snippets
- Framework and version claims
- Workflow and convention claims: CI files, lint configs, default branch

Out of scope (never verified, never edited): URLs, third-party tool or API behavior, paths outside the repo.

### Step 7: Verify and Fix

Check each claim against the repo and fix discrepancies with the smallest possible edit span.

**File/directory paths:**

```sh
test -e "$project_root/$claimed_path" \
  || fd --hidden --no-ignore --max-results 5 "$(basename "$claimed_path")" "$project_root"
```

- Renamed or moved → update the mention to the new path.
- Deleted with no successor → remove the mention (or the row/bullet containing it).

**Directory trees:** re-list the claimed tree with `ls` or `fd` from `project_root`; correct names and nesting, drop entries that no longer exist. Add new entries only when the tree claims to be exhaustive.

**Commands:**

```sh
(cd "$project_root" && just --summary) 2>/dev/null
jq -r '.scripts | keys[]' "$project_root/package.json" 2>/dev/null
```

- Documented recipe or script renamed → update the invocation.
- Removed → remove the documented entry.
- Wrong runner (e.g. `npm run x` but the lock file says pnpm) → fix it.

**CLI flags** (repo-owned scripts and tools only):

```sh
rg -n -- '--flag-name' "$project_root"
```

Flags of third-party tools are left alone unless the repo wraps or pins them.

**Environment variables:**

```sh
rg -n 'VAR_NAME' "$project_root" --hidden --glob '!.git'
```

Check `.env.example`, CI workflow files, and source reads (`process.env.X`, `os.environ`). Renamed → update; removed everywhere → remove the documented entry.

**Code symbols/APIs in fences:** for snippets that quote repo code:

```sh
rg -n 'symbolName' "$project_root"
```

Renamed symbol → update the fence; removed → rework the example minimally or remove it. Never leave a fence referencing a symbol that no longer exists.

**Framework/version claims:** read the manifests —

```sh
jq -r '.dependencies, .devDependencies | keys[]?' "$project_root/package.json"
```

(or `Cargo.toml`, `pyproject.toml`, `foundry.toml` as appropriate). Fix drifted major-version claims, e.g. "Tailwind v3" when the manifest pins v4.

**Workflow/convention claims:** verify CI files (`.github/workflows/*.yml`), lint and hook configs (lint-staged, husky), and the default branch:

```sh
git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null
```

**Leave alone** (restated): URLs, third-party documentation and API behavior, `~/...` paths and anything outside the repo, and the skill's style, tone, and structure.

### Step 8: Obsolete Skill Handling

A skill is obsolete when its central subject is missing from both `project_root` and `$repo_root`: the contract it wraps was deleted, the service it deploys was removed, the tool it documents is no longer a dependency. In that case:

1. Stop editing that skill — fixing details of a dead subject creates false freshness.
2. Report `⚠ <name> appears obsolete` with one line of evidence (e.g. `contracts/Vesting.sol no longer exists`).
3. Suggest the user shelve or delete the skill.
4. Never delete it or hollow out its content yourself.

Stale ≠ obsolete: a stale skill's subject still exists, and it gets fixes as normal.

### Step 9: Suggested Additions

Scan each `project_root` for new capabilities near a skill's stated scope: new `just` recipes, new `package.json` scripts, new env vars touching its domain.

- Default: report-only, under a `## Suggested Additions` section grouped per skill.
- Write the addition only when the skill's stated scope unambiguously covers it (e.g. the skill says it documents every deploy recipe and a new deploy recipe exists).
- Never invent new skills; never broaden a skill's stated scope.

### Step 10: Apply Updates

**If --dry-run:**

```
## Planned Changes

### packages/contracts/.agents/skills/deploy
- SKILL.md line 12: `just deploy-testnet` → `just deploy --network testnet`
- references/networks.md: remove row for deleted `goerli` config

### .agents/skills/release
- SKILL.md: package manager `npm` → `pnpm` (2 occurrences)

## Suggested Additions

deploy: new recipe `just verify-deployment` is in scope; draft entry below.
```

**If NOT --dry-run:**

1. Apply each fix with the smallest possible edit span; leave everything else byte-identical.
2. Optionally show `git diff -- "$skill_dir"` so the user can review per skill.
3. Never commit; the user reviews and commits.

### Step 11: Format Changed Files

Best-effort, changed files only, using the host repo's own tooling:

1. A `just` recipe matching a known formatter name exists → run it:

   ```sh
   (cd "$repo_root" && just --summary 2>/dev/null) | tr ' ' '\n' \
   | grep -E -m1 -x 'mdformat-write|mdformat|fmt-md|format-md'
   ```

2. Else, a prettier config exists **and** prettier is already a dependency in `package.json` → `nlx prettier --write <changed files>`.

3. Else → `⊘` skip silently.

Never install formatters or add dependencies.

### Step 12: Report Summary

Group results per `.agents/skills` location (path relative to the repo root as the sub-header), indent per-skill deltas, and close with a tally.

**Grouped report:**

```
### packages/contracts/.agents/skills

✓ deploy — updated
  - Fixed recipe rename: `just deploy-testnet` → `just deploy --network testnet`
  - references/networks.md: removed deleted `goerli` row

⚠ vesting — obsolete
  - contracts/Vesting.sol no longer exists; suggest shelving or deleting this skill

### .agents/skills

✓ release — up to date

✗ scratch — skipped
  - Unparseable frontmatter (no unambiguous mechanical repair)

Checked 4 skills: 1 updated, 1 up to date, 1 obsolete, 1 failed.
```

**Zero targets found:**

```
⊘ No project skills found under .agents/skills in this repo.
```

**All clean:**

```
✓ All 3 skills under .agents/skills are up to date.
```

**Dry run:** open with the `## Planned Changes` block from Step 10 and close with the tally phrased prospectively, e.g. `Would update 2 skills; 1 already up to date.`

## Notes

- Facts vs. voice: the repo is the source of truth for facts; the skill is the source of truth for its workflow, voice, and structure.
- Never verify a nested package's skill against another package's manifests; resolve `project_root` per skill.
- Writes are confined to the discovered `.agents/skills` directories — nothing else in the repo, nothing under `$HOME`.
- Never delete a skill or its files; obsolete skills are flagged, not removed.
- Never auto-commit.
