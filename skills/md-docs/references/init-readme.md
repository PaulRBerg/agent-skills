# Init README Reference

Generate human-aimed `README.md` files from scratch with content derived from project metadata or a user-provided description. README contains description, badges, links, references, license, contributing pointer — never CLI commands, `just` recipes, scripts, or project structure trees. Those live in the sibling `AGENTS.md`.

## Overview

The init-readme workflow creates tailored README.md files for new projects or repositories lacking documentation. By default it creates one in **each package root** (the repo root plus every manifest-bearing directory) that lacks a README; use `--root-only` for just the repo root, or a `path` argument to limit the sweep. It operates in two modes: automatic inference (derives content from project files) or guided mode (uses user description to focus content).

Unlike `update-readme`, this workflow:

- Refuses to overwrite an existing README unless `--force` is passed
- Accepts a freeform description argument for guided content generation
- Assumes no prior README content to preserve

If README.md already exists and the user wants to refresh it, suggest `/md-docs:update-readme --preserve` instead.

## Guiding Principles

**README.md is for humans browsing the repo on GitHub or a package registry.**

- **Generic information only** — what the project is, where to learn more, how it relates to other work.
- **No CLI commands** — install, build, test, lint, dev, deploy, scripts, `just` recipes all belong in `AGENTS.md`.
- **No project structure trees** — that's a developer concern; AGENTS.md owns it.
- **No API reference** — link to the dedicated docs site or AGENTS.md instead.
- **Show, don't tell** — short, direct prose; if a code snippet is critical, link to a file in `examples/` rather than embedding it.
- **Respect the reader's time** — they want to know what it is, whether it's relevant, and where to go next.

**Target length by mode**:

- `--minimal`: 40-80 lines
- Default: 80-150 lines
- `--full`: 150-250 lines (only if the project has many references or related projects worth listing)

## Workflow Steps

### 1. Parse Arguments

Determine operating mode and flags:

- **Automatic inference mode**: No description argument — derive content from project metadata
- **Guided mode**: Description argument provided — focus content around the user's intent (applies to the root README; nested package READMEs are inferred from their own manifests)
- `path`: Confine the sweep to one directory subtree
- `--root-only`: Create only the repo-root README (disables recursion)
- `--dry-run`: Preview content without writing
- `--minimal`: Title, badges, description, license, AGENTS.md pointer
- `--full`: All applicable sections (About, References, Related Projects, Acknowledgments)
- `--force`: Overwrite existing README.md files without prompting

Guided mode examples:

- `/md-docs:init-readme TypeScript library for parsing dates with zero deps`
- `/md-docs:init-readme Foundry lending protocol with audit-ready docs`
- `/md-docs:init-readme Next.js app for tracking crypto portfolio`

### 2. Validate Prerequisites

**CHECK repository state:**

- Run `git rev-parse --show-toplevel` to confirm we're in a git repository
- IF not a git repo: ERROR "Must be run from within a git repository. Initialize with 'git init' first."
- Store the repository root path

**Scope**: Enumerate package roots — the repo root plus every directory holding a manifest (`package.json`, `Cargo.toml`, `pyproject.toml`, `setup.py`, `go.mod`, `foundry.toml`, `Gemfile`, `composer.json`), minus excluded dirs (`.git`, `node_modules`, `vendor`, `.venv`, `target`, `dist`, `build`, `out`, `.next`, `coverage`, gitignored, hidden-without-manifest). See Recursive Discovery in `SKILL.md` / `references/common-patterns.md`. `--root-only` keeps just the repo root; a `path` argument confines the sweep. Create a README only in package roots that lack one. Run steps 3 onward **per package root**, scoping metadata to that directory's nearest manifest.

**CHECK for CONTRIBUTING.md (per package root):**

```bash
test -f "$dir/CONTRIBUTING.md" && echo "found" || echo "absent"
```

If found, append a `⚠ CONTRIBUTING.md detected` advisory for that directory recommending a merge into the sibling AGENTS.md and deleting the file. Do not touch `CONTRIBUTING.md`.

### 3. Check Existing README (per package root)

```bash
test -f "$dir/README.md" && echo "exists" || echo "missing"
```

A package root whose README already exists is skipped (it's an `update-readme` job, not init). If existing file found AND `--force` not set:

- Read the existing file (for reporting size/sections)
- Use `AskUserQuestion` with options:
  - **Overwrite**: Replace existing file completely
  - **Skip**: Leave it and continue to the next package root
  - **Abort**: Cancel operation (suggest `/md-docs:update-readme --preserve` as an alternative)
- For a multi-target sweep, ask once for the batch rather than per file. Wait for user response before proceeding.

If `--force` is set, proceed without prompting. Rely on git to restore prior content if needed.

### 4. Gather Project Metadata

Read only the metadata needed for human-facing content. Do **not** scan scripts, lockfiles, or build configs.

**Language/Stack Detection (for badges only):**

- `package.json` → Node.js / TypeScript / JavaScript
- `Cargo.toml` → Rust
- `pyproject.toml` or `setup.py` → Python
- `foundry.toml` → Solidity / Foundry
- `go.mod` → Go
- `Gemfile` → Ruby
- `composer.json` → PHP

**Extract from detected metadata files:**

- Project name
- Version (only if shown in a version badge)
- Description / summary
- License identifier
- Homepage / documentation URL
- Repository URL (also check `git remote get-url origin`)
- Author / maintainer (for acknowledgments, optional)
- Keywords (helps categorize, optional)

**Find human-facing key files:**

- `LICENSE` or `LICENSE.md`
- `CITATION.cff`, `CITATIONS.bib`, `papers/` directory → drives References section
- `CHANGELOG.md` → link from README
- `CODE_OF_CONDUCT.md` → link from README
- `.github/FUNDING.yml` → drives Sponsors / Funding mention
- `examples/`, `demo/`, `docs/` directories → link to them
- `assets/`, `screenshots/`, `media/` directories → drive a banner or screenshot section

**Discover documentation site:**

- `package.json` `homepage` field
- `Cargo.toml` `documentation` field
- A `docs/` directory with a published site (look for `mkdocs.yml`, `docusaurus.config.js`, `vitepress.config.js`, `nextra.config.js`)
- Mention of GitHub Pages in CI (`.github/workflows/*.yml` deploying to `gh-pages`)

Do not run package managers, build tools, or scripts at any point.

### 5. Determine Project Type

Project type is informational only — used to tailor description tone and badge selection. Section list is the same across types because README is now a landing page, not a manual.

- **Library**: exports modules / functions for external consumption
- **Application**: runnable program / service
- **Smart Contract**: Solidity / Foundry / Hardhat project
- **CLI Tool**: ships a binary
- **Monorepo**: multiple workspace packages

### 6. Generate README Content

#### Guided Mode (Description Provided)

Analyze the description for:

- **Keywords** — "security", "testing", "monorepo", "contracts", "zero deps"
- **Constraints** — "strict", "functional", "minimal", "fast", "audit-ready"
- **Tools** — "Foundry", "Next.js", "React", "Viem", "Effect-TS"
- **Priorities** — what matters most to the user

Use this to refine the description sentence and to pick relevant badges. Do not introduce technical sections (install, scripts, etc.) regardless of what the description says — those go in AGENTS.md.

#### Automatic Inference Mode (No Description)

Derive sections and emphasis entirely from step 4 metadata:

- Primary language/framework → badge selection
- Repository description / metadata → README description
- Detected docs site → Links section content
- Detected CITATION/papers → References section

#### Section List (All Project Types)

01. Title + Badges
02. Description
03. Optional banner / screenshot (if assets exist)
04. Links
05. About / Background (if `--full` or guided mode supplies enough material)
06. References (if applicable)
07. Related Projects (if `--full` and detected)
08. Acknowledgments (only if explicitly requested or maintainer info present and meaningful)
09. Contributing (pointer to AGENTS.md)
10. License

#### Writing Style Requirements

- **Plain prose** — short, direct, descriptive. Not imperative ("Install with..."), not marketing.
- **Expert-to-expert** — assume readers know their ecosystem.
- **Minimal markdown**:
  - `##` for main sections, `###` for subsections
  - Bullet points for link lists and references
  - Inline code for project / library names
  - **Bold** sparingly

#### Content Templates

Follow the section templates, formatting rules, and examples documented in `references/update-readme.md`, specifically:

- Title + Badges format and badge selection
- Description length (1-3 sentences)
- Links bullet list format
- References bullet list format
- License sentence format
- No emoji decoration on section headers
- No code blocks showing CLI commands

### 7. Compose Final README

BUILD complete markdown content using the standard structure:

```markdown
# {project-name}

{badges row}

{description paragraph}

{optional banner/screenshot}

## Links

- [Documentation]({url})
- [Package]({url})
- [Changelog](CHANGELOG.md)
- [Discussions]({url})

{about/background section if applicable}

{references section if applicable}

{related projects section if applicable}

{acknowledgments section if applicable}

## Contributing

Contributions are welcome. See [`AGENTS.md`](AGENTS.md) for the development workflow, commands, and conventions.

## License

{license information}
```

**Formatting rules:**

- Plain `##` headings; no emoji decoration

- Code blocks rare and never CLI commands

- Tables only for things like a feature comparison with related projects

- Blank lines between sections

- Line length ~100-120 chars in paragraphs

- Admonitions sparingly:

  ```markdown
  > [!NOTE]
  > Helpful context

  > [!WARNING]
  > Breaking changes or critical notices
  ```

### 8. Write README

#### Location

`$dir/README.md` — the package root being processed (repo root for the top-level file).

#### Write Operation

- For `--dry-run`: Display generated content without writing
- Otherwise: Use the Write tool to create `$dir/README.md` with the composed content

#### Confirmation

Success:

- Display file path
- Show first 10 lines as preview
- Success message: `✓ Created $dir/README.md`

Failure:

- Check permissions: `ls -la "$dir/README.md"`
- Check disk space: `df -h .`
- Suggest specific fix
- DO NOT retry automatically

Loop to the next package root; emit the grouped summary after the last one.

### 9. Display Summary

For recursive runs, repeat this block per package root under a `### {path}` sub-header and end with a tally (`Created N files, skipped M`). Single-target template:

```
✓ Created README.md

**Mode**: {minimal/default/full}
**Project Type**: {Library/Application/Smart Contract/etc.}

**Sections Generated:**
- Title + Badges
- Description
- Links
- Contributing pointer (→ AGENTS.md)
- License
- {others}

{IF CONTRIBUTING.md detected:}
⚠ CONTRIBUTING.md detected
  - Recommend merging its contents into AGENTS.md (which owns the development workflow).
  - Delete CONTRIBUTING.md after the merge.

**Next Steps:**
1. Review README.md for accuracy
2. Ensure development commands and workflow live in AGENTS.md
3. Commit changes: `git add README.md && git commit -m "docs: add README"`
```

## Usage Examples

**Automatic inference (no description, recursive):**

```bash
/md-docs:init-readme
```

Analyzes each package root lacking a README and creates a complete human-aimed README in each.

**Root README only:**

```bash
/md-docs:init-readme --root-only
```

**Limit to one package subtree:**

```bash
/md-docs:init-readme packages/core
```

**Guided mode with description:**

```bash
/md-docs:init-readme TypeScript library for parsing dates with zero deps
```

Refines the description and badge selection around zero-dependency emphasis.

**Minimal README (fast):**

```bash
/md-docs:init-readme --minimal
```

Title, badges, description, license, AGENTS.md pointer only.

**Full README:**

```bash
/md-docs:init-readme --full
```

Includes About, References, Related Projects, Acknowledgments where applicable.

**Dry-run preview:**

```bash
/md-docs:init-readme --dry-run
```

Preview generated content without writing to disk.

**Force overwrite existing README:**

```bash
/md-docs:init-readme --force
```

Replace existing `README.md` without prompting. Rely on git to restore prior content if needed.

## Key Characteristics

**Audience-strict**: README never contains CLI commands, scripts, or developer workflows. All such content is the responsibility of AGENTS.md, and the README links to it.

**Language-agnostic**: Works with Node.js, Rust, Python, Solidity, Go, Ruby, PHP, and other common stacks (only for badge selection and metadata extraction).

**Safe by default**: Refuses to overwrite existing README.md without `--force` or explicit user confirmation.

**Guided or automatic**: Accepts a freeform description for focused content, or infers everything from project metadata.

**Idempotent**: Running multiple times with the same inputs produces consistent results.

**No git operations**: Only creates README.md; never auto-commits.

**Monorepo handling**: Creates a README in each package root that lacks one (repo root plus manifest-bearing directories). Each is scoped to its own manifest and links to its sibling `AGENTS.md`. Use `--root-only` for just the repo root, or a `path` argument to confine the sweep.

**CONTRIBUTING.md handling**: Detected but never edited. Recommendation surfaced to merge into AGENTS.md and delete.

**Related**: For updating an existing README while preserving custom sections, use `/md-docs:update-readme --preserve`. For initializing AGENTS.md/CLAUDE.md context files (where the development commands live), use `/md-docs:init-agents`.
