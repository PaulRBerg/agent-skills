# Common Patterns

Shared conventions and patterns used across all documentation workflows.

## Audience Split

The skill enforces a strict split between `README.md` and `AGENTS.md`. Every workflow must respect it.

| File                                  | Audience                                     | Contains                                                                                                                                                                    | Excludes                                                                                                             |
| ------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `README.md`                           | Humans browsing the repo (GitHub, npm, etc.) | Description, badges, links to docs/site/demo, references, papers, related work, acknowledgments, license, contributing pointer                                              | Any CLI commands, `just` recipes, package scripts, build/test/lint workflows, project structure trees, API reference |
| `AGENTS.md` (and `CLAUDE.md` symlink) | AI agents and developers working in the repo | Stack, commands (install, dev, build, test, lint), `just` recipes, `package.json` scripts, `Makefile` targets, code style, architecture, conventions, contribution workflow | Marketing copy, badges, external links unrelated to development                                                      |

When in doubt, ask: *would a human reading this on GitHub care, or only a developer/agent running commands?* If the latter, it goes in AGENTS.md.

The README is allowed to mention its sibling AGENTS.md and link to it; that's the only legitimate way for a reader to reach development commands from the README.

## Recursive Operation

Workflows act on every relevant file in the repository by default — not just the root. The skill's `SKILL.md` documents the canonical discovery commands; this section captures the conventions every workflow must honor.

**Scope controls.**

- Default: recurse over the whole repo from `git rev-parse --show-toplevel`.
- `--root-only`: collapse to the repo root (pre-recursion behavior).
- `path` argument: confine the sweep to one subtree.

**Exclusions (always).** During discovery and creation, skip:

- VCS / dependency / build output dirs: `.git`, `node_modules`, `vendor`, `.venv`, `target`, `dist`, `build`, `out`, `.next`, `coverage`.
- Anything ignored by git (use `git ls-files --others --exclude-standard` and `git check-ignore`).
- Hidden dot-directories, unless the directory holds a manifest.

**`update-*` targets** — every existing `README.md` / `AGENTS.md` in scope:

```bash
git ls-files --cached --others --exclude-standard -- '**/README.md' 'README.md' '**/AGENTS.md' 'AGENTS.md'
```

**`init-*` targets** — package roots (repo root + directories holding a manifest such as `package.json`, `Cargo.toml`, `pyproject.toml`, `setup.py`, `go.mod`, `foundry.toml`, `Gemfile`, `composer.json`) that lack the file. Never create files in arbitrary leaf directories.

**Per-file scoping.** Each target is independent:

- Read metadata from the nearest enclosing manifest (own directory, else walk up).
- A nested `README.md` links to its sibling `AGENTS.md`, never the root one.
- Each `AGENTS.md` gets a sibling `CLAUDE.md` symlink in the same directory.
- `CONTRIBUTING.md` is detected per directory and merged into the sibling `AGENTS.md`.

`CLAUDE.md` is a symlink to its sibling `AGENTS.md` and is never enumerated or written on its own.

## CONTRIBUTING.md Policy

This skill does not maintain `CONTRIBUTING.md`. The sibling AGENTS.md owns the contribution workflow.

When any workflow detects `CONTRIBUTING.md` next to a target (repo root or any package root):

1. Surface a `⚠ CONTRIBUTING.md detected` advisory in the final report, scoped to that directory.
2. Recommend merging its contents into the sibling AGENTS.md (under a `Contribution Workflow` section).
3. Suggest deleting `CONTRIBUTING.md` after the merge so AGENTS.md is the single source of truth.
4. Never auto-merge or auto-delete; the user performs the merge.
5. Do not edit `CONTRIBUTING.md` in any workflow.

If a generated AGENTS.md is missing a `Contribution Workflow` section while `CONTRIBUTING.md` exists, leave a stub section so the user has a clear destination for the merge.

## Argument Parsing

Standard arguments supported across workflows:

- `path` (positional): Confine the recursive sweep to one directory subtree
- `--root-only`: Disable recursion; act on the repo root only
- `--dry-run`: Preview changes without writing files
- `--preserve`: Maintain existing structure, only fix inaccuracies
- `--minimal`: Generate minimal documentation
- `--thorough` / `--full`: Generate comprehensive documentation
- `--force`: Override safety checks (applies to every file in the sweep)

Parse arguments from user input and set appropriate flags for workflow execution.

## Overwrite Safety

Rely on git for recovery. Do not create `*.backup` files when overwriting `README.md` or `AGENTS.md`. `CLAUDE.md` is a symlink to its sibling `AGENTS.md` and is not written separately. If the working tree has uncommitted changes to a target file, surface that before overwriting. When a recursive sweep would create or rewrite more than a handful of files, list the planned targets and confirm once for the batch before writing.

## Writing Style

### README.md (humans)

- **Plain prose** — short, descriptive sentences. Not imperative, not marketing.
- **Generic information** — what the project is, where to learn more, how it relates to other work.
- **No CLI commands** — anywhere. Link to AGENTS.md instead.
- **Scannable** — headings, bullet lists for links and references.
- **Accurate** — verify all URLs against actual project metadata.

### AGENTS.md (developers and agents)

- **Terse** — omit needless words, lead with the answer.
- **Imperative** — "Build the project", "Run tests before committing".
- **Expert-to-expert** — skip basic explanations.
- **Scannable** — headings, lists, code blocks for commands.
- **Accurate** — verify all commands against `justfile` / `package.json` / `Makefile` before writing.

### Good — AGENTS.md

```markdown
## Commands

- `just build` — compile all packages
- `just test` — run vitest across the workspace
- `just lint` — run BiomeJS
```

### Bad — README.md (CLI commands belong in AGENTS.md)

````markdown
## Installation

```bash
pnpm install
pnpm build
```
````

### Good — README.md

```markdown
## Links

- [Documentation](https://example.com/docs)
- [Package on npm](https://npmjs.com/package/foo)

## Contributing

Contributions are welcome. See [`AGENTS.md`](AGENTS.md) for the development workflow, commands, and conventions.
```

### Bad — AGENTS.md (marketing copy belongs in README)

```markdown
## About

Foo is a fast, ergonomic, zero-dependency library that makes parsing dates a breeze. Loved by 10,000 developers worldwide.
```

## Report Formatting

After completing operations, display a clear summary:

```
✓ Updated AGENTS.md
  - Refreshed Commands section from justfile (5 recipes) and package.json (8 scripts)
  - Fixed outdated build command
  - Added new directory structure

✓ Updated README.md
  - Refreshed badges and links
  - Removed Installation/Usage/Scripts sections (now in AGENTS.md)

⚠ CONTRIBUTING.md detected
  - Recommend merging into AGENTS.md → Contribution Workflow.
  - Delete CONTRIBUTING.md after the merge.

⊘ Section X skipped
  - Reason
```

Use:

- `✓` for successful operations
- `⚠` for advisory notices (CONTRIBUTING.md merge)
- `⊘` for skipped optional sections
- `✗` for failed operations

Include indented details showing specific changes made.

### Recursive runs

Group the summary by file path (relative to the repo root) and close with a tally:

```
### .
✓ Updated AGENTS.md
  - Refreshed Commands from justfile (5 recipes)
✓ Updated README.md
  - Refreshed badges and links

### packages/core
✓ Updated AGENTS.md
  - Added vitest test command
⚠ CONTRIBUTING.md detected → merge into packages/core/AGENTS.md

### packages/cli
⊘ README.md unchanged (already accurate)

Updated 3 files, skipped 1, 1 advisory.
```

For `--dry-run`, list every target path under a `## Planned Changes` header before showing per-file diffs.

## File Detection

Detect project type and structure by checking for characteristic files:

```bash
# Node.js / JavaScript / TypeScript
test -f package.json

# Python
test -f pyproject.toml || test -f setup.py

# Rust
test -f Cargo.toml

# Go
test -f go.mod

# Solidity / Foundry
test -f foundry.toml

# Ruby
test -f Gemfile

# PHP
test -f composer.json
```

Detect task runners (drives the AGENTS.md Commands section):

```bash
test -f justfile
test -f Makefile
test -f Taskfile.yml
test -f mise.toml
```

Use detection results to customize documentation templates.

## Metadata Extraction

Read package configuration files to extract accurate metadata.

For README.md (human-facing fields only — name, description, license, homepage, repository):

```bash
# Node.js
jq '.name, .version, .description, .license, .homepage, .repository' package.json

# Python
grep -E '^(name|version|description|license|homepage)' pyproject.toml

# Rust
grep -E '^(name|version|description|license|homepage|documentation|repository)' Cargo.toml
```

For AGENTS.md (technical fields — engines, scripts, dependencies):

```bash
# Node.js scripts
jq '.scripts | to_entries[] | "\(.key): \(.value)"' package.json

# justfile recipes (just lists them itself)
just --list

# Makefile targets
grep -E '^[a-zA-Z_-]+:' Makefile | sed 's/:.*//'
```

Parse JSON or TOML appropriately to extract values. Never hardcode or guess metadata when it can be read directly from configuration files.

In recursive runs, run detection and extraction per target against the **nearest enclosing manifest** (the one in the target's own directory, else the closest ancestor). A nested package's README/AGENTS reflects that package's manifest, scripts, and task runners — not the root project's. Fall back to the repo-root manifest only when a target directory has none of its own.
