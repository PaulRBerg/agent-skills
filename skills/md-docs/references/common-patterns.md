# Common Patterns

Shared conventions and patterns used across all documentation workflows.

## Audience Split

The skill enforces a strict split between `README.md` and `AGENTS.md`. Every workflow must respect it.

| File                                  | Audience                                     | Contains                                                                                                                                                                    | Excludes                                                                                                             |
| ------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `README.md`                           | Humans browsing the repo (GitHub, npm, etc.) | Description, badges, links to docs/site/demo, references, papers, related work, acknowledgments, license, contributing pointer                                              | Any CLI commands, `just` recipes, package scripts, build/test/lint workflows, project structure trees, API reference |
| `AGENTS.md` (and `CLAUDE.md` symlink) | AI agents and developers working in the repo | Stack, commands (install, dev, build, test, lint), `just` recipes, `package.json` scripts, `Makefile` targets, code style, architecture, conventions, contribution workflow | Marketing copy, badges, external links unrelated to development                                                      |

When in doubt, ask: *would a human reading this on GitHub care, or only a developer/agent running commands?* If the latter, it goes in AGENTS.md.

The README is allowed to mention AGENTS.md and link to it; that's the only legitimate way for a reader to reach development commands from the README.

## CONTRIBUTING.md Policy

This skill does not maintain `CONTRIBUTING.md`. AGENTS.md owns the contribution workflow.

When any workflow detects `CONTRIBUTING.md` at the repo root:

1. Surface a `⚠ CONTRIBUTING.md detected` advisory in the final report.
2. Recommend merging its contents into AGENTS.md (under a `Contribution Workflow` section).
3. Suggest deleting `CONTRIBUTING.md` after the merge so AGENTS.md is the single source of truth.
4. Never auto-merge or auto-delete; the user performs the merge.
5. Do not edit `CONTRIBUTING.md` in any workflow.

If a generated AGENTS.md is missing a `Contribution Workflow` section while `CONTRIBUTING.md` exists, leave a stub section so the user has a clear destination for the merge.

## Argument Parsing

Standard arguments supported across workflows:

- `--dry-run`: Preview changes without writing files
- `--preserve`: Maintain existing structure, only fix inaccuracies
- `--minimal`: Generate minimal documentation
- `--thorough` / `--full`: Generate comprehensive documentation
- `--force`: Override safety checks

Parse arguments from user input and set appropriate flags for workflow execution.

## Overwrite Safety

Rely on git for recovery. Do not create `*.backup` files when overwriting `README.md` or `AGENTS.md`. `CLAUDE.md` is a symlink to `AGENTS.md` and is not written separately. If the working tree has uncommitted changes to the target file, surface that to the user before overwriting.

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
````

````

### Good — README.md

```markdown
## Links

- [Documentation](https://example.com/docs)
- [Package on npm](https://npmjs.com/package/foo)

## Contributing

Contributions are welcome. See [`AGENTS.md`](AGENTS.md) for the development workflow, commands, and conventions.
````

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
