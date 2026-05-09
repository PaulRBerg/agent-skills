# Update AGENTS.md Workflow

Workflow for updating `AGENTS.md` to match actual codebase state. `CLAUDE.md` is a symlink to `AGENTS.md` and does not need separate processing.

AGENTS.md is the single source of truth for everything technical: stack, commands, scripts, recipes, build/test/lint workflows, code style, architecture, conventions, and the contribution workflow. The README links here for any of it.

## Workflow

### Step 1: Parse Arguments

Supported flags:

- `--dry-run`: Show what would change without writing files
- `--preserve`: Keep existing content structure, only fix inaccuracies
- `--thorough`: Perform deep analysis of all files (slower but comprehensive)
- `--minimal`: Quick verification focusing on high-level structure only

Mode behavior:

- If `--dry-run`: Show planned changes without writing files
- Otherwise: Apply changes in place (rely on git for recovery; do not create `*.backup` files)
- For `--preserve`: Maintain existing structure when rewriting; for default mode, reorganize for clarity when needed
- For `--thorough`: Also analyze file content patterns, code organization conventions, and dependency relationships
- For `--minimal`: Skip deep file content scans

### Step 1a: Verify Git Repository

Confirm working directory is a git repository:

```bash
git rev-parse --git-dir
```

If not a git repo, warn the user but proceed with limitations (cannot analyze git history or branches).

### Step 1b: Detect CONTRIBUTING.md

```bash
test -f CONTRIBUTING.md && echo "found" || echo "absent"
```

If found, append a `⚠ CONTRIBUTING.md detected` advisory to the final report. AGENTS.md is now the home for the contribution workflow, so recommend the user merge `CONTRIBUTING.md` into AGENTS.md (under a "Contribution Workflow" section) and delete the file. Do not auto-merge or auto-delete.

When updating AGENTS.md in this run, ensure it has a `Contribution Workflow` section if one is missing — but do not lift content from `CONTRIBUTING.md` automatically; just leave a stub so the user has a clear destination for the merge.

### Step 2: Read Existing AGENTS.md & Extract Verifiable Claims

```bash
cat AGENTS.md
```

`CLAUDE.md` is a symlink to `AGENTS.md` and does not need a separate read.

Extract verifiable claims:

- File paths mentioned
- Directory structures described
- Commands referenced (build tools, scripts, package managers, recipes, targets)
- Rules about what to edit/not edit
- Workflow descriptions
- Testing patterns
- Dependencies and integrations
- Branch naming conventions
- PR / review process references

### Step 3: Verify and Fix Claims

Check each claim against actual codebase and auto-fix discrepancies:

**File/Directory claims:**

- Use `ls`, `fd`, or `tree` to verify paths exist
- If path changed: update to new path
- If path deleted: mark section for removal or update

**Command claims:**

- Verify commands exist in `justfile`, `package.json`, `Makefile`, `Taskfile.yml`, `mise.toml`, or scripts directories
- If command syntax changed: update to match actual command
- If command removed: remove the documented entry
- If a documented command is wrong (e.g. `npm test` but project uses pnpm): fix it

**Linting configuration:**

- Locate lint-staged config (`.lintstagedrc.js`, `.lintstagedrc.json`, `lint-staged` in `package.json`)
- Locate other linters / formatters (eslint, biome, prettier, ruff, clippy, golangci-lint, slither)
- Extract lint commands for each file pattern
- If linting instructions don't match: update to match config

**Code structure claims:**

- Read actual files to verify patterns described
- Update outdated patterns to match current code

**Branch / contribution claims:**

- Verify documented branch names against current default branch (`git symbolic-ref refs/remotes/origin/HEAD`)
- Verify PR template paths exist (`.github/PULL_REQUEST_TEMPLATE.md` or `.github/PULL_REQUEST_TEMPLATE/`)
- Verify CLI commands and tooling references match current state

**CLAUDE.md symlink:**

- Confirm `CLAUDE.md` exists as a symlink pointing to `AGENTS.md`
- If missing: create with `ln -sf AGENTS.md CLAUDE.md`

### Step 4: Discover Undocumented Patterns

Scan for patterns not mentioned in AGENTS.md. Because AGENTS.md owns all technical commands, these discoveries are first-class additions, not optional polish.

**Task runners (mandatory in Commands section):**

- `justfile` → list every recipe and what it does
- `package.json` `scripts` → list every script and what it does
- `Makefile` → list every target and what it does
- `Taskfile.yml`, `mise.toml`, `bun.lock` scripts → same

**Package manager detection (drives install / dev / build commands):**

- `pnpm-lock.yaml` → pnpm
- `yarn.lock` → yarn
- `bun.lockb` or `bun.lock` → bun
- `package-lock.json` → npm
- `Cargo.lock` → cargo
- `requirements.txt`, `poetry.lock`, `uv.lock`, `pdm.lock` → pip / poetry / uv / pdm
- `go.sum` → go modules
- `Gemfile.lock` → bundler
- `composer.lock` → composer

**Lint configuration:**

- If lint-staged exists but no linting section in AGENTS.md, draft one
- If formatter configs (`.prettierrc*`, `biome.json`, `.editorconfig`) exist but unmentioned, draft a Style section

**Build/test commands:**

- Check for undocumented build, test, deploy, or migration commands

**Architecture / conventions:**

- Project structure (top-level directories, what each holds)
- Code style hints from configs (TypeScript strict mode, ESLint rules, Biome config, Clippy lints)
- Test runner detection (vitest, jest, mocha, pytest, cargo test, go test, forge test)

**Branch / PR conventions:**

- Default branch
- Branch naming convention if discoverable from CI or `.github` configs
- PR template path

If `CONTRIBUTING.md` exists, list its discovered sections in the report so the user knows what to merge in.

### Step 5: Apply Updates

**If --dry-run:**

Show preview of all changes without writing:

```
## Planned Changes

AGENTS.md:
  - Line X: "{old}" → "{new}"
  - Section Y: [REMOVE - path no longer exists]

## Suggested Additions

AGENTS.md: Consider adding section:
[Draft section content]
```

**If NOT --dry-run:**

1. Apply all fixes to `AGENTS.md` directly
2. Ensure `CLAUDE.md` symlink exists: `ln -sf AGENTS.md CLAUDE.md`
3. Optionally show diff: `git diff -- AGENTS.md`
4. Report changes made

### Step 6: Required Sections in AGENTS.md

Every AGENTS.md after this workflow should have these sections (create if missing, update if present):

- **Stack** — languages, frameworks, primary tools.
- **Commands** — exhaustive table or list of every CLI command a developer or agent runs: install, dev, build, test, lint, format, typecheck, deploy. Include `just` recipes, npm/pnpm/yarn/bun scripts, Makefile targets, etc., each with a one-line description.
- **Architecture** / **Project Structure** — top-level directories and their purpose.
- **Code Style** — naming, formatting, patterns (referencing config files where they exist).
- **Conventions** — anything project-specific (commits, PR titles, file organization).
- **Contribution Workflow** — branch naming, default branch, PR process, review expectations. (Stub if `CONTRIBUTING.md` is being merged.)

`--minimal` may compress these into a shorter form (Stack, Commands, Style) but Commands is mandatory in every mode.

### Step 7: Report Summary

**Format with all expected files:**

```
✓ Updated AGENTS.md
  - Refreshed Commands section from justfile (5 recipes) and package.json (8 scripts)
  - Fixed outdated build command: `npm run build` → `pnpm build`
  - Added new /api directory to architecture section
  - Added Contribution Workflow stub

✓ CLAUDE.md symlink verified

⚠ CONTRIBUTING.md detected
  - Recommend merging its sections (Setup, Code Review, Branch Conventions) into AGENTS.md → Contribution Workflow.
  - Delete CONTRIBUTING.md after the merge.
```

**Format when CONTRIBUTING.md absent:**

```
✓ Updated AGENTS.md
  - Refreshed Commands section
  - Fixed outdated build command

✓ CLAUDE.md symlink verified
```

**If no changes needed:**

```
✓ AGENTS.md is up to date
✓ CLAUDE.md symlink verified
```

**For fixes:**

```
## Fixed

✓ AGENTS.md: Updated package manager npm → pnpm
✓ AGENTS.md: Removed outdated `lint:fix` script (no longer in package.json)
✓ AGENTS.md: Added missing `just sync` recipe
```

**For suggestions:**

```
## Suggested Additions

AGENTS.md: Consider adding:

### Style

- TypeScript strict mode enforced via `tsconfig.json`
- Biome formatter (`biome.json`) — run with `pnpm format`

---
```

## Notes

- Focus on factual claims, not stylistic opinions
- Preserve user's writing style when making fixes
- AGENTS.md is the source of truth for commands; do not let documented commands drift from `justfile` / `package.json` / `Makefile`
- Adapt discovery to project type (web, CLI, library, contracts, monorepo)
- Never edit `CONTRIBUTING.md`; only recommend merging it into AGENTS.md
