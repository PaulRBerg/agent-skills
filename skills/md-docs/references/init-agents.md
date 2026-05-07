# Init AGENTS.md Reference

Generate a project-specific `AGENTS.md` from scratch with content derived from project analysis or a user-provided description. AGENTS.md is the single source of truth for technical content: stack, commands, recipes, build/test/lint workflows, code style, architecture, conventions, and the contribution workflow. The README links here for any of it.

## Overview

The init-agents workflow creates tailored AGENTS.md files that provide project-specific instructions for AI agents and developers working in the repo. It operates in two modes: automatic inference (derives context from project files) or guided mode (uses user description to focus content).

## Workflow Steps

### 1. Check Arguments

Determine operating mode and flags:

- **Automatic inference mode**: No description provided — derive context from project analysis
- **Guided mode**: Description provided — use it to focus content generation
- `--dry-run`: Preview generated content without writing
- `--minimal`: Create minimal context file (Stack, Commands, Style)
- `--full`: Create comprehensive context file with all relevant sections
- `--force`: Overwrite existing AGENTS.md without prompting

Guided mode examples:

- `/md-docs:init-agents TypeScript monorepo with strict type safety and functional patterns`
- `/md-docs:init-agents Foundry smart contract project with security-first mindset`

### 2. Check Existing AGENTS.md

```bash
test -f AGENTS.md && echo "exists" || echo "none"
```

If existing file found:

- Read the existing file
- Use `AskUserQuestion` with options:
  - **Overwrite**: Replace existing file completely
  - **Merge**: Append new context to existing file
  - **Abort**: Cancel operation
- Wait for user response before proceeding

### 2a. Detect CONTRIBUTING.md

```bash
test -f CONTRIBUTING.md && echo "found" || echo "absent"
```

If found, append a `⚠ CONTRIBUTING.md detected` advisory to the final report. AGENTS.md is the home for the contribution workflow, so recommend the user merge `CONTRIBUTING.md` into the new AGENTS.md (under a `Contribution Workflow` section) and delete the file. Do not auto-merge or auto-delete; leave a stub section in the generated AGENTS.md so the user has a clear destination.

### 3. Gather Project Context

Read available project files (skip if missing):

- `package.json` — stack, scripts, dependencies, engines
- `pyproject.toml`, `Cargo.toml`, `go.mod`, `Gemfile`, `composer.json` — language-specific context
- `foundry.toml`, `hardhat.config.{js,ts}` — smart contract context
- `justfile` — recipe inventory (always read in full)
- `Makefile`, `Taskfile.yml`, `mise.toml` — task runner alternatives
- `.lintstagedrc*`, `.eslintrc*`, `biome.json`, `.prettierrc*`, `.editorconfig`, `ruff.toml`, `clippy.toml` — style configuration
- `tsconfig.json`, `pyproject.toml [tool.*]`, `Cargo.toml [profile.*]` — strictness / build profile
- `README.md` — project overview, purpose
- `.gitignore` — exclusion patterns
- `fd -t f -d 2` or `ls -la` — quick directory layout

Detect package manager from lock files:

- `pnpm-lock.yaml` → pnpm
- `yarn.lock` → yarn
- `bun.lockb` or `bun.lock` → bun
- `package-lock.json` → npm
- `Cargo.lock` → cargo
- `requirements.txt`, `poetry.lock`, `uv.lock`, `pdm.lock` → pip / poetry / uv / pdm
- `go.sum` → go modules
- `Gemfile.lock` → bundler
- `composer.lock` → composer

Analyze to understand:

- Primary language / framework
- Project type (library, app, contracts, monorepo, CLI)
- Build / test / lint tools
- Architecture hints

### 4. Generate AGENTS.md Content

#### Writing Style Requirements

- **Terse and direct** — no fluff, straight to point
- **Expert-to-expert** — assume high competency
- **Imperative mood** — commands ("Use", "Follow", "Avoid")
- **Active voice** — "Run tests before committing", not "Tests should be run"
- **Minimal markdown**:
  - `##` for major sections
  - `###` for subsections
  - Bullet points or tables for lists of commands
  - Inline code for technical terms
  - **Bold** for emphasis

#### Required Sections

AGENTS.md owns the technical surface area. These sections must exist (create them; populate from analysis):

- **Stack** — languages, frameworks, primary tools, language versions
- **Commands** — exhaustive list of every CLI command a developer or agent runs. Pulled from `justfile`, `package.json` scripts, `Makefile`, `Taskfile.yml`. Each entry has a one-line description.
- **Architecture** / **Project Structure** — top-level directories and what they hold
- **Code Style** — naming, formatting, patterns (referencing config files where they exist)
- **Conventions** — anything project-specific (commits, PR titles, file organization)
- **Contribution Workflow** — branch naming, default branch, PR process, review expectations. Stub if `CONTRIBUTING.md` is awaiting merge.

`--minimal` may compress these into Stack, Commands, Style only — but Commands is mandatory in every mode.

`--full` adds:

- **Testing** — runners, conventions, fixtures
- **Constraints** — security, performance, compliance
- **Dependencies & Integrations** — notable third-party services
- **Workflows** — common multi-step procedures (release, deploy, migration)

#### Commands Section Template

Always render Commands as a table or grouped list. Examples:

```markdown
## Commands

- `just` — list recipes
- `just build` — compile all packages
- `just test` — run vitest across the workspace
- `just lint` — run BiomeJS
- `just sync` — install skills and commit installed copies
- `pnpm install` — install dependencies
- `pnpm typecheck` — run `tsc --noEmit`
```

Or as a table when there are 8+ entries:

```markdown
## Commands

| Command | Description |
| --- | --- |
| `just build` | Compile all packages |
| `just test` | Run vitest across the workspace |
| ... | ... |
```

Group by source if both `just` and `package.json` scripts exist:

```markdown
## Commands

### Just recipes

- `just build` — compile all packages
- `just test` — run tests

### Package scripts

- `pnpm typecheck` — run `tsc --noEmit`
- `pnpm format` — run BiomeJS formatter
```

Never document install / build / test / lint commands inline elsewhere in the file; link back to this section if needed.

#### Content Patterns Example

```markdown
## Stack

- TypeScript 5.x with strict mode
- pnpm workspaces
- Vitest for testing
- BiomeJS for linting

## Commands

- `just` — list recipes
- `just build` — compile all packages
- `just test` — run vitest across the workspace
- `just lint` — run BiomeJS
- `pnpm install` — install dependencies
- `pnpm typecheck` — run `tsc --noEmit`

## Code Style

- Functional patterns over classes
- Explicit return types on exported functions
- No `any` — use `unknown` and type guards
- Prefer `const` and immutability

## Contribution Workflow

- Default branch: `main`
- Branch naming: `feat/<short>`, `fix/<short>`, `chore/<short>`
- Run `just lint && just test` before opening a PR
- One reviewer required
```

#### Guided Mode (Arguments Provided)

Analyze arguments for:

- **Keywords** — "security", "testing", "monorepo", "contracts"
- **Constraints** — "strict", "functional", "minimal", "fast"
- **Tools** — "Foundry", "Next.js", "React", "Viem"
- **Priorities** — what matters most to the user

Generate sections matching intent. Example: "security-first Foundry project" emphasizes:

- Security tools (Slither, fuzz tests) in Commands
- Audit requirements in Constraints
- Safe patterns in Code Style
- Test coverage requirements in Testing

#### Automatic Inference Mode (No Arguments)

Derive context entirely from STEP 3 analysis:

- Primary language / framework → Stack section
- Detected task runners → Commands section (full inventory)
- Project type → Architecture section
- Detected style configs → Code Style section
- Branching from git remote → Contribution Workflow section

Infer priorities from project signals:

- `foundry.toml` or security deps → security focus
- Extensive test setup → testing emphasis
- Multiple packages → monorepo patterns
- Strict TypeScript config → type safety focus

### 5. Write AGENTS.md

#### Location

`./AGENTS.md` (root)

#### Write Operation

**Overwrite or no existing file**:

- Write complete new AGENTS.md with generated content

**Merge mode**:

- Read existing content
- Append separator: `\n---\n\n# Auto-generated Context\n\n`
- Append generated content
- Write combined content

#### Confirmation

Success:

- Display file path
- Show first 10 lines as preview
- Success message: `✓ Created AGENTS.md at ./AGENTS.md`

Failure:

- Check permissions
- Suggest specific fix
- DO NOT retry automatically

#### Create CLAUDE.md Symlink

After writing AGENTS.md, create a symlink for Claude Code compatibility:

```bash
ln -sf AGENTS.md CLAUDE.md
```

- Creates relative symlink `CLAUDE.md → AGENTS.md`
- Use `-f` to overwrite existing symlink
- Confirm symlink creation: `✓ Created CLAUDE.md symlink`

### 6. Optional Project Import Suggestions

Check for commonly useful files:

- `README.md` exists → suggest adding `@README.md` for project overview
- `package.json` exists → suggest adding `@package.json` for available scripts (only if Commands section relies on its content)

Format suggestion:

```
💡 Tip: Consider importing project files into AGENTS.md:

@README.md         # Project overview
@package.json      # Available scripts

Add these lines to AGENTS.md to auto-load context.
```

## Key Principles

- **AGENTS.md owns the technical surface** — README never duplicates Commands, Style, or Architecture; it links here.
- **Case-by-case content** — adapt to user's description; the required sections above are the spine, everything else is optional.
- **Commands are exhaustive** — every recipe / script / target appears in Commands with a one-line description.
- **Import awareness** — AGENTS.md can use `@path/to/file` syntax to import other files.
- **Hierarchy context** — project AGENTS.md supplements (not replaces) user / enterprise AGENTS.md.
- **CONTRIBUTING.md handling** — never edit it; recommend merging its content into the AGENTS.md `Contribution Workflow` section and deleting it.

## Related Resources

- Memory docs: https://docs.anthropic.com/en/docs/claude-code/memory
