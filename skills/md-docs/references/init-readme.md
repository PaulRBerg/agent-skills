# Init README Reference

Generate project-specific README.md files from scratch with content derived from project analysis or a user-provided description.

## Overview

The init-readme workflow creates tailored README.md files for new projects or repositories lacking documentation. It operates in two modes: automatic inference (derives content from project files) or guided mode (uses user description to focus content).

Unlike `update-readme`, this workflow:

- Refuses to overwrite an existing README unless `--force` is passed
- Accepts a freeform description argument for guided content generation
- Assumes no prior README content to preserve

If README.md already exists and the user wants to refresh it, suggest `/md-docs:update-readme --preserve` instead.

## Guiding Principles

**README files are for humans, not documentation dumps.**

- **Balanced, not bloated**: Aim for 200-400 lines for most projects
- **Show, don't tell**: Prefer concise code examples over lengthy prose
- **Every section must add value**: Skip sections that would be empty or trivial
- **Readability first**: Clear headings, proper spacing, visual hierarchy
- **Respect the reader's time**: What it does, how to install, how to use — in under 30 seconds of scanning

**Target length by mode**:

- `--minimal`: 100-200 lines
- Default: 200-400 lines
- `--full`: 400-600 lines (only if the project genuinely needs it)

## Workflow Steps

### 1. Parse Arguments

Determine operating mode and flags:

- **Automatic inference mode**: No description argument — derive content from project analysis
- **Guided mode**: Description argument provided — focus content around the user's intent
- `--dry-run`: Preview content without writing
- `--minimal`: Title, description, installation, usage only
- `--full`: All applicable sections
- `--force`: Overwrite existing README.md without prompting

Guided mode examples:

- `/md-docs:init-readme TypeScript library for parsing dates with zero deps`
- `/md-docs:init-readme Foundry lending protocol with audit-ready docs`
- `/md-docs:init-readme Next.js app for tracking crypto portfolio`

### 2. Validate Prerequisites

**CHECK repository state:**

- Run `git rev-parse --show-toplevel` to confirm we're in a git repository
- IF not a git repo: ERROR "Must be run from within a git repository. Initialize with 'git init' first."
- Store the repository root path

**Scope**: Operate only on `README.md` at the repository root. For package-specific READMEs in a monorepo, the user should `cd` to that package directory first.

### 3. Check Existing README

Check for existing file:

```bash
test -f README.md && echo "exists" || echo "missing"
```

If existing file found AND `--force` not set:

- Read the existing file (for reporting size/sections)
- Use `AskUserQuestion` with options:
  - **Overwrite**: Replace existing file completely
  - **Abort**: Cancel operation (suggest `/md-docs:update-readme --preserve` as an alternative)
- Wait for user response before proceeding

If `--force` is set, proceed without prompting but create a backup:

```bash
cp README.md README.md.backup
```

### 4. Gather Codebase Intelligence

**Language/Stack Detection:**

- `package.json` → Node.js/TypeScript/JavaScript project
- `Cargo.toml` → Rust project
- `pyproject.toml` or `setup.py` → Python project
- `foundry.toml` → Solidity/Foundry smart contract project
- `go.mod` → Go project
- `Gemfile` → Ruby project
- `composer.json` → PHP project

**Extract from detected metadata files:**

- Project name
- Version number
- Description
- License
- Dependencies (production and dev)
- Scripts/commands (build, test, lint, deploy)
- Repository URL (from git remote or package file)

**Discover project structure:**

```bash
fd -t d -d 2 2>/dev/null | sort || find . -type d -maxdepth 2 | sort
```

**Find key files:**

- LICENSE or LICENSE.md
- CONTRIBUTING.md
- CHANGELOG.md
- CODE_OF_CONDUCT.md
- .github/workflows/\*.yml (CI/CD)
- examples/ or example/ directory
- docs/ or doc/ directory

**Analyze entry points (language-specific):**

- JavaScript/TypeScript: `index.ts`, `index.js`, `src/index.ts`, `main.ts`
- Rust: `src/main.rs`, `src/lib.rs`
- Python: `__main__.py`, `main.py`, `src/__init__.py`
- Solidity: `src/*.sol` contracts
- Go: `main.go`, `cmd/*/main.go`

Detect package manager from lock files:

- `package-lock.json` → npm
- `pnpm-lock.yaml` → pnpm
- `yarn.lock` → yarn
- `bun.lockb` or `bun.lock` → bun
- `Cargo.lock` → cargo
- `requirements.txt` or `poetry.lock` → pip/poetry

### 5. Determine Project Type

Classify the project to drive section selection:

- **Library**: No main executable; exports modules/functions for external consumption
- **Application**: Has main entry point; runnable program
- **Smart Contract**: Solidity/Foundry/Hardhat project
- **CLI Tool**: Exports a binary via `bin` field or equivalent
- **Monorepo**: Multiple workspace packages (`workspaces`, `pnpm-workspace.yaml`, Cargo workspaces, etc.)

### 6. Generate README Content

#### Guided Mode (Description Provided)

Analyze the description for:

- **Keywords** — "security", "testing", "monorepo", "contracts", "zero deps"
- **Constraints** — "strict", "functional", "minimal", "fast", "audit-ready"
- **Tools** — "Foundry", "Next.js", "React", "Viem", "Effect-TS"
- **Priorities** — What matters most to the user

Emphasize sections matching intent. Examples:

- "security-first Foundry project" → emphasize testing, audit notes, Slither setup, safe patterns
- "zero-dep TypeScript library" → emphasize bundle size, API reference, tree-shaking
- "Next.js portfolio tracker" → emphasize features, configuration, deployment

#### Automatic Inference Mode (No Description)

Derive sections and emphasis entirely from step 4 analysis:

- Primary language/framework → code example syntax and install commands
- Project type → section order and presence of API reference
- Detected tooling → scripts table, CI badges
- Dependency signals → feature highlights (e.g., `react` + `next` → Next.js app emphasis)

#### Section Selection by Project Type

**Libraries:**

1. Title + Badges
2. Description
3. Features (if `--full` or useful)
4. Installation
5. Usage (with code examples)
6. API Reference (if `--full`)
7. Contributing
8. License

**Applications:**

1. Title + Badges
2. Description
3. Features
4. Installation
5. Usage/Getting Started
6. Configuration (if config files found)
7. Scripts/Commands
8. Project Structure (if `--full`)
9. Contributing
10. License

**Smart Contracts:**

1. Title + Badges
2. Description
3. Installation (npm + forge install)
4. Usage (Solidity import examples)
5. Functions/API
6. Testing
7. Deployment
8. Contributing
9. License

#### Writing Style Requirements

- **Terse and direct** — no fluff, straight to point
- **Expert-to-expert** — assume high competency
- **Active voice** — "Run tests before committing" not "Tests should be run"
- **Minimal markdown**:
  - `##` for main sections, `###` for subsections
  - Bullet points for lists
  - Inline code for technical terms
  - **Bold** for emphasis

#### Content Templates

Follow the section templates, formatting rules, and examples documented in `references/update-readme.md`, specifically:

- Title + Badges format and badge selection
- Description length (1-3 sentences)
- Installation commands by package manager
- Usage example constraints (5-15 lines, simplest working code)
- Scripts table format
- Project structure depth (2 levels max)
- License format
- Emoji section headers convention (📦 Install, 🚀 Usage, 🤝 Contributing, 📄 License)

### 7. Compose Final README

BUILD complete markdown content using the standard structure:

```markdown
# {project-name}

{badges row}

{description paragraph}

{features section if applicable}

## 📦 Installation

{installation instructions}

## 🚀 Usage

{usage examples with code blocks}

{scripts/commands section if applicable}

{project structure section if --full}

{API reference if --full}

{configuration section if applicable}

## 🤝 Contributing

{contributing guidelines or link to CONTRIBUTING.md}

## 📄 License

{license information}
```

**Formatting rules:**

- Code blocks have language specifiers (```bash, ```typescript, ```solidity)
- Use tables for scripts/commands if there are 5+ items
- Blank lines between sections
- Line length ~100-120 chars in paragraphs
- Admonitions for critical notices:

  ```markdown
  > [!NOTE]
  > Helpful context

  > [!WARNING]
  > Breaking changes or critical notices
  ```

### 8. Write README

#### Location

`./README.md` (repository root)

#### Write Operation

- For `--dry-run`: Display generated content without writing
- Otherwise: Use the Write tool to create README.md with the composed content

#### Confirmation

Success:

- Display file path
- Show first 10 lines as preview
- Success message: `✓ Created README.md at ./README.md`

Failure:

- Check permissions: `ls -la README.md`
- Check disk space: `df -h .`
- Suggest specific fix
- DO NOT retry automatically

### 9. Display Summary

```
✓ Created README.md

**Mode**: {minimal/default/full}
**Project Type**: {Library/Application/Smart Contract/etc.}
**Language/Stack**: {TypeScript/Rust/Solidity/Python/etc.}

**Sections Generated:**
- Title + Badges
- Installation instructions
- Usage examples
- {list other sections}

**Next Steps:**
1. Review README.md for accuracy
2. Customize auto-generated content as needed
3. Commit changes: `git add README.md && git commit -m "docs: add README"`
```

## Usage Examples

**Automatic inference (no description):**

```bash
/md-docs:init-readme
```

Analyzes codebase and creates a complete README from scratch with inferred content.

**Guided mode with description:**

```bash
/md-docs:init-readme TypeScript library for parsing dates with zero deps
```

Focuses content around zero-dependency, API reference, and bundle size.

**Minimal README (fast):**

```bash
/md-docs:init-readme --minimal
```

Title, description, installation, usage only. Completes in ~3 seconds.

**Full README with all sections:**

```bash
/md-docs:init-readme --full
```

Comprehensive README with API reference, project structure, configuration, and deployment sections where applicable.

**Dry-run preview:**

```bash
/md-docs:init-readme --dry-run
```

Preview generated content without writing to disk.

**Force overwrite existing README:**

```bash
/md-docs:init-readme --force
```

Replace existing README.md without prompting (creates a `.backup` first).

## Key Characteristics

**Language-agnostic**: Works with Node.js, Rust, Python, Solidity, Go, Ruby, PHP, and other common stacks.

**Safe by default**: Refuses to overwrite existing README.md without `--force` or explicit user confirmation.

**Guided or automatic**: Accepts a freeform description for focused content, or infers everything from project files.

**Smart defaults**: Automatically detects project type (library vs application vs smart contract) and adjusts sections accordingly.

**Idempotent**: Running multiple times with the same inputs produces consistent results.

**No git operations**: Only creates README.md; never auto-commits. User reviews and commits manually.

**Monorepo handling**: Operates ONLY on README.md at the repository root (via `git rev-parse --show-toplevel`). For package-specific READMEs, `cd` to that package directory first.

**Related**: For updating an existing README while preserving custom sections, use `/md-docs:update-readme --preserve`. For initializing AGENTS.md/CLAUDE.md context files, use `/md-docs:init-agents`.
