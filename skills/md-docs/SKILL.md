---
argument-hint: <update-readme|update-agents|init-readme|init-agents> [path] [--root-only] [--preserve] [--minimal] [--thorough] [--dry-run]
disable-model-invocation: false
name: md-docs
user-invocable: true
description: This skill should be used ONLY when the user asks to update or initialize README.md, CLAUDE.md, or AGENTS.md. Trigger phrases include "update README", "init README", "update context files", "update CLAUDE.md/AGENTS.md". Do NOT activate for any other Markdown file updates.
---

# Markdown Documentation Management

## Overview

Manage project documentation for Claude Code workflows including README.md and agent context files (AGENTS.md / CLAUDE.md). This skill enforces a strict audience split: **README.md is for humans**, **AGENTS.md is for agents and developers running commands**. Use this skill when initializing new projects, updating existing documentation, or ensuring context files accurately reflect current code.

By default the skill operates **recursively** across the whole repository, not just the root. `update-*` workflows find and refresh every existing README.md / AGENTS.md in the tree; `init-*` workflows create files at the repo root and at each package root (a directory holding a manifest or workspace member) that lacks one. Each file is scoped to its own directory subtree and the nearest enclosing manifest. Pass `--root-only` to restrict any workflow to the repository root, or pass a `path` argument to limit the sweep to one subtree.

The skill emphasizes verification and validation over blind generation — analyze the actual codebase structure, file contents, and patterns before creating or updating documentation. All generated content should be terse, imperative, and expert-to-expert rather than verbose or tutorial-style.

## Audience Split

This is the central rule for everything this skill produces.

| File                                  | Audience                                     | Contains                                                                                                                                                                    | Excludes                                                                                                             |
| ------------------------------------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `README.md`                           | Humans browsing the repo (GitHub, npm, etc.) | Description, badges, links to docs/site/demo, references, papers, related work, acknowledgments, license, contributing pointer                                              | Any CLI commands, `just` recipes, package scripts, build/test/lint workflows, project structure trees, API reference |
| `AGENTS.md` (and `CLAUDE.md` symlink) | AI agents and developers working in the repo | Stack, commands (install, dev, build, test, lint), `just` recipes, `package.json` scripts, `Makefile` targets, code style, architecture, conventions, contribution workflow | Marketing copy, badges, external links unrelated to development                                                      |

When in doubt, ask: *would a human reading this on GitHub care, or only a developer/agent running commands?* If the latter, it goes in AGENTS.md.

## Workflow Selection

Pick the workflow that matches the user's intent:

| Trigger                                                               | Workflow             | Reference                     |
| --------------------------------------------------------------------- | -------------------- | ----------------------------- |
| "update README" / "refresh README" (file already exists)              | Update README        | `references/update-readme.md` |
| "init README" / "create README" / "new README" (no file or `--force`) | Initialize README    | `references/init-readme.md`   |
| "update CLAUDE.md" / "update AGENTS.md" / "update context files"      | Update Context Files | `references/update-agents.md` |
| "init AGENTS.md" / "create CLAUDE.md" / "init context"                | Initialize Context   | `references/init-agents.md`   |

Selection rules:

- If the target file already exists and the user says "update" / "refresh" / "fix", route to an `update-*` workflow.
- If the target file is missing or the user says "create" / "init" / "new", route to an `init-*` workflow.
- For ambiguous requests, enumerate the target files first (see Recursive Discovery) and confirm with the user.
- If the user invokes the skill with no arguments, default to listing the files present across the tree and proposing a workflow rather than guessing.
- Multiple workflows in one request (e.g. "update README and AGENTS.md") are fine — run them sequentially in the order the user listed them, reporting each result independently.
- Each selected workflow applies to every target file the discovery step finds, unless `--root-only` or a narrowing `path` argument restricts the sweep.

## CONTRIBUTING.md Policy

This skill does **not** maintain `CONTRIBUTING.md`. If the workflow detects a `CONTRIBUTING.md` alongside any target file (repo root or any package root):

1. Do not block other writes; surface the advisory for that directory.
2. Recommend the user merge its contents into the sibling `AGENTS.md` (since AGENTS.md now owns the development workflow, branch conventions, review process, and tooling references).
3. Suggest deleting `CONTRIBUTING.md` after the merge so the agent context file is the single source of truth.
4. Do not auto-merge or auto-delete; the user performs the merge.

Report the recommendation per directory in the standard summary format and continue with whichever README/AGENTS workflow the user requested, ignoring the `CONTRIBUTING.md` file itself.

## Prerequisites

Before using any documentation workflow, verify basic project structure:

```bash
git rev-parse --git-dir
```

Ensure the output confirms you are in a git repository. If not initialized, documentation workflows may still proceed but git-specific features will be skipped.

Resolve the repository root once; all discovery is relative to it (or to a `path` argument, if given):

```bash
git rev-parse --show-toplevel
```

Then enumerate target files (see Recursive Discovery below) before attempting any workflow. If `CONTRIBUTING.md` shows up next to a target, apply the policy above for that directory.

## Recursive Discovery

By default, workflows act on every relevant file in the tree. `--root-only` collapses this to the repo root; a `path` argument scopes it to one subtree.

**Exclusions (always).** Skip these everywhere during discovery and creation:

- VCS and dependency/build output dirs: `.git`, `node_modules`, `vendor`, `.venv`, `target`, `dist`, `build`, `out`, `.next`, `coverage`.
- Anything ignored by git (rely on `--exclude-standard` / `git check-ignore`).
- Hidden dot-directories (`.github`, `.vscode`, `.claude`, …) — **unless** the directory contains a manifest.

**`update-*` discovery** — find existing files to refresh:

```bash
# README.md / AGENTS.md tracked or untracked, respecting .gitignore
git ls-files --cached --others --exclude-standard -- '**/README.md' 'README.md' '**/AGENTS.md' 'AGENTS.md'
```

Drop any path under an excluded dir. `CLAUDE.md` is a symlink to its sibling `AGENTS.md` and is never processed on its own.

**`init-*` discovery** — find package roots that should get a new file:

```bash
# Directories holding a language/tooling manifest = package roots
git ls-files --cached --others --exclude-standard \
  -- '**/package.json' 'package.json' '**/Cargo.toml' 'Cargo.toml' \
     '**/pyproject.toml' 'pyproject.toml' '**/setup.py' 'setup.py' \
     '**/go.mod' 'go.mod' '**/foundry.toml' 'foundry.toml' \
     '**/Gemfile' 'Gemfile' '**/composer.json' 'composer.json'
```

The set of package roots is the repo root plus the unique directories of those manifests (minus exclusions). `init-*` creates the target file only in package roots that lack it; it does not create files in arbitrary leaf directories.

**Per-file scoping.** Treat each target independently:

- Metadata source is the nearest enclosing manifest (the one in its own directory, else walk up to the repo root).
- A nested `README.md` links to its **sibling** `AGENTS.md`, not the root one.
- Each `AGENTS.md` gets a sibling `CLAUDE.md` symlink in the **same** directory (`ln -sf AGENTS.md CLAUDE.md`, run from that directory).
- `CONTRIBUTING.md` is checked per directory and merged into the sibling `AGENTS.md`.

Process files deepest-first or root-first consistently, and report results grouped by path.

## Common Arguments

These flags are interpreted consistently across workflows. Each reference describes their per-workflow effects in detail; see `references/common-patterns.md` for shared parsing conventions.

- `path` (positional): Limit the recursive sweep to this directory subtree instead of the whole repo. Combine with any workflow.
- `--root-only`: Disable recursion; act on the repository root only (the pre-recursion behavior). Always supported.
- `--dry-run`: Preview the changes that would be applied without writing files. Always supported.
- `--preserve`: Keep existing user-authored content; only fix verifiable inaccuracies. Used by `update-*` workflows.
- `--minimal`: Generate or verify the smallest useful output (top-level structure only).
- `--thorough` (alias `--full`): Perform deep analysis or generate comprehensive content. Slowest mode.
- `--force`: Override safety checks (e.g. overwrite existing target without prompting). Used by `init-*` workflows. Applies to every file in the sweep.

If the user passes other flags, fall back to default mode and surface a one-line note about the unrecognized flag in the final report.

## Writing Style

All generated documentation should follow these conventions, regardless of workflow:

- **Terse**: Omit needless words. Lead with the answer or the link.
- **Imperative** (AGENTS.md): Use command form ("Build the project") not descriptive ("The project is built").
- **Plain prose** (README.md): Short, direct descriptions; avoid imperative lecturing — the audience is browsing, not executing.
- **Expert-to-expert**: Skip basic explanations; assume reader competence.
- **Scannable**: Use headings, lists, and code blocks. A reader should find what they need in under 30 seconds.
- **Accurate**: Verify every command, link, and path against the actual codebase before writing.

Avoid tutorial-style prose, redundant context, and filler such as "In order to...". When in doubt, write less. See `references/common-patterns.md` for examples of good vs. bad output.

## Safety Defaults

Behaviors that apply across every workflow:

- Never auto-commit. Workflows touch documentation files only; the user reviews and runs `git add` / `git commit` manually. Rely on git for recovery — do not create `*.backup` files.
- For `init-*` workflows: refuse to overwrite an existing target unless `--force` is set or the user confirms via `AskUserQuestion`. When the sweep touches many files, confirm once for the batch rather than prompting per file.
- Recurse by default but stay inside the repo (`git rev-parse --show-toplevel`); honor the exclusions in Recursive Discovery. Use `--root-only` or a `path` argument to narrow scope. Never write outside the discovered target set.
- When a sweep would create or rewrite more than a handful of files, list the planned targets and get confirmation before writing (treat it like an implicit `--dry-run` preview first).
- If `CONTRIBUTING.md` exists next to any target, do not edit it; surface the merge-into-AGENTS recommendation for that directory and continue.

## Update Context Files

When to use: user asks to update CLAUDE.md or AGENTS.md so they match the actual codebase. Trigger phrases include "update CLAUDE.md", "update AGENTS.md", "update context files", "fix context", "refresh context".

`CLAUDE.md` is a symlink to its sibling `AGENTS.md` and is not processed separately.

Runs on every existing `AGENTS.md` in the tree (see Recursive Discovery); each is scoped to its own directory and nearest manifest. AGENTS.md owns: stack, all CLI commands (install, dev, build, test, lint, deploy), `just` recipes, `package.json` scripts, `Makefile` targets, code style, architecture, conventions, and contribution workflow.

Inputs: each existing `AGENTS.md` (required), the nearest enclosing manifests, lock files, scripts, `justfile`, `Makefile`. Outputs: rewritten `AGENTS.md` files, each with a refreshed sibling `CLAUDE.md` symlink.

Recognised flags: `path`, `--root-only`, `--dry-run`, `--preserve`, `--thorough`, `--minimal`.

See [references/update-agents.md](references/update-agents.md).

## Update README

When to use: user asks to update or refresh an existing README.md. Trigger phrases include "update README", "refresh README", "fix README", "regenerate README".

Runs on every existing `README.md` in the tree (see Recursive Discovery). For any package root that has no `README.md`, route that directory to **Initialize README** instead (or, with `--force`, allow update-readme to create it there).

README owns: description, badges, links (homepage, docs site, demo, package registry), references, related work, acknowledgments, license, contributing pointer. It does **not** contain CLI commands, `just` recipes, scripts, or project structure trees — those live in the sibling AGENTS.md, and the README links to it for them.

Inputs: each existing `README.md`; the nearest enclosing manifests for name/version/description/license/homepage URL; git remote for repository URL. Outputs: rewritten `README.md` files.

Recognised flags: `path`, `--root-only`, `--dry-run`, `--preserve`, `--minimal`, `--thorough` (alias `--full`).

See [references/update-readme.md](references/update-readme.md).

## Initialize README

When to use: user asks to create new README.md files from scratch in a repository (or package roots) that lack them. Trigger phrases include "init README", "create README", "new README", "generate a README".

Creates a `README.md` in each package root that lacks one (repo root plus manifest-bearing directories; see Recursive Discovery). Refuses to overwrite an existing `README.md` without `--force` or explicit confirmation via `AskUserQuestion`. Supports two operating modes:

- **Automatic inference**: derive content entirely from project analysis.
- **Guided**: focus content around a user-provided description (e.g., "TypeScript library for parsing dates with zero deps"). A guided description applies to the root file; nested package files are inferred from their own manifests.

Same audience rules as Update README: humans only, no CLI.

Inputs: per-package-root codebase analysis (language, framework, LICENSE, homepage URL, citations or papers in repo), optional user-provided description. Outputs: new `README.md` in each targeted package root.

Recognised flags: `path`, `--root-only`, `--dry-run`, `--minimal`, `--full`, `--force`.

See [references/init-readme.md](references/init-readme.md).

## Initialize Context

When to use: user asks to create new AGENTS.md (and CLAUDE.md symlink) files from scratch in a repository (or package roots) that lack context documentation. Trigger phrases include "init AGENTS.md", "create CLAUDE.md", "init context", "new context file", "generate AGENTS.md".

Creates an `AGENTS.md` in each package root that lacks one (see Recursive Discovery). Like Initialize README, supports automatic inference and guided mode (e.g., "Foundry smart contract project with security-first mindset"). For each generated file, always creates the sibling `CLAUDE.md` symlink via `ln -sf AGENTS.md CLAUDE.md` run from that directory.

Each generated AGENTS.md must include a Commands section that consolidates every CLI invocation a developer or agent will need for that package: install, dev, build, test, lint, format, deploy, plus all `just` recipes, npm/pnpm/yarn/bun scripts, and Makefile targets discovered in its directory.

Inputs: per-package-root codebase analysis (stack, scripts, `justfile`, `Makefile`, architecture hints, nearest `package.json`, `README.md`, language-specific manifests), optional user-provided description. Outputs: new `AGENTS.md` files, each with a sibling `CLAUDE.md` symlink.

Recognised flags: `path`, `--root-only`, `--dry-run`, `--minimal`, `--full`, `--force`.

See [references/init-agents.md](references/init-agents.md).

## Reporting

Every workflow ends with a short summary. Use these conventions across all workflows:

- `✓` for successful operations: `✓ Updated AGENTS.md` followed by indented bullet points listing concrete changes.
- `⊘` for skipped optional files.
- `⚠` for advisory notices (e.g. CONTRIBUTING.md merge recommendation).
- `✗` for failures: `✗ Failed to write README.md` with a one-line cause.

Indent change details under each line so the user can scan a single file's deltas without re-reading the header. For recursive runs, group results by file path (use the path relative to the repo root as a sub-header) and end with a one-line tally (e.g. `Updated 4 files, skipped 1, 1 advisory`). For `--dry-run`, prefix the report with a "Planned Changes" header, list every target path, and include the diff or proposed-content preview rather than a confirmation. Refer to `references/common-patterns.md` for full report templates.

## Additional Resources

For detailed workflows, examples, and implementation guidance, refer to these reference documents:

- **`references/common-patterns.md`** — Audience split, argument parsing, writing style, report formatting, file detection, metadata extraction, CONTRIBUTING.md merge recommendation
- **`references/update-agents.md`** — Complete context file update workflow including verification strategies, command discovery, and discrepancy detection
- **`references/update-readme.md`** — Complete README update workflow for human-aimed content
- **`references/init-readme.md`** — Complete README initialization workflow for human-aimed content
- **`references/init-agents.md`** — Complete context initialization workflow including language-specific templates and commands consolidation

These references provide implementation details, code examples, and troubleshooting guidance for each workflow type.
