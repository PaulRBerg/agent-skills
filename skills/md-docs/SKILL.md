---
argument-hint: <update-readme|update-agents|init-readme|init-agents> [--preserve] [--minimal] [--thorough] [--dry-run]
disable-model-invocation: false
name: md-docs
user-invocable: true
description: This skill should be used ONLY when the user asks to update or initialize README.md, CLAUDE.md, or AGENTS.md. Trigger phrases include "update README", "init README", "update context files", "update CLAUDE.md/AGENTS.md". Do NOT activate for any other Markdown file updates.
---

# Markdown Documentation Management

## Overview

Manage project documentation for Claude Code workflows including README.md and agent context files (AGENTS.md / CLAUDE.md). This skill enforces a strict audience split: **README.md is for humans**, **AGENTS.md is for agents and developers running commands**. Use this skill when initializing new projects, updating existing documentation, or ensuring context files accurately reflect current code.

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
- For ambiguous requests, list available files first (see Prerequisites) and confirm with the user.
- If the user invokes the skill with no arguments, default to listing the files present and proposing a workflow rather than guessing.
- Multiple workflows in one request (e.g. "update README and AGENTS.md") are fine — run them sequentially in the order the user listed them, reporting each result independently.

## CONTRIBUTING.md Policy

This skill does **not** maintain `CONTRIBUTING.md`. If the workflow detects a `CONTRIBUTING.md` at the repo root:

1. Stop before writing anything else for that workflow.
2. Recommend the user merge its contents into `AGENTS.md` (since AGENTS.md now owns the development workflow, branch conventions, review process, and tooling references).
3. Suggest deleting `CONTRIBUTING.md` after the merge so the agent context file is the single source of truth.
4. Do not auto-merge or auto-delete; the user performs the merge.

Report the recommendation in the standard summary format and continue with whichever README/AGENTS workflow the user requested, ignoring the `CONTRIBUTING.md` file.

## Prerequisites

Before using any documentation workflow, verify basic project structure:

```bash
git rev-parse --git-dir
```

Ensure the output confirms you are in a git repository. If not initialized, documentation workflows may still proceed but git-specific features will be skipped.

For update workflows, verify target files exist:

```bash
ls -la CLAUDE.md AGENTS.md README.md CONTRIBUTING.md
```

Check which files are present before attempting updates. Missing files will show errors, which helps identify what needs initialization. If `CONTRIBUTING.md` shows up, apply the policy above before continuing.

For monorepos, also confirm the working directory:

```bash
git rev-parse --show-toplevel
```

All workflows operate on files at the repo root — never on nested README/AGENTS files inside subpackages. To document a specific package, the user must `cd` into that package's directory first.

## Common Arguments

These flags are interpreted consistently across workflows. Each reference describes their per-workflow effects in detail; see `references/common-patterns.md` for shared parsing conventions.

- `--dry-run`: Preview the changes that would be applied without writing files. Always supported.
- `--preserve`: Keep existing user-authored content; only fix verifiable inaccuracies. Used by `update-*` workflows.
- `--minimal`: Generate or verify the smallest useful output (top-level structure only).
- `--thorough` (alias `--full`): Perform deep analysis or generate comprehensive content. Slowest mode.
- `--force`: Override safety checks (e.g. overwrite existing target without prompting). Used by `init-*` workflows.

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
- For `init-*` workflows: refuse to overwrite an existing target unless `--force` is set or the user confirms via `AskUserQuestion`.
- Operate only at the repository root (`git rev-parse --show-toplevel`). For monorepo subpackages, the user must `cd` into that package's directory before invoking the skill.
- If `CONTRIBUTING.md` exists, do not edit it; surface the merge-into-AGENTS recommendation and continue.

## Update Context Files

When to use: user asks to update CLAUDE.md or AGENTS.md so they match the actual codebase. Trigger phrases include "update CLAUDE.md", "update AGENTS.md", "update context files", "fix context", "refresh context".

`CLAUDE.md` is a symlink to `AGENTS.md` and is not processed separately.

AGENTS.md owns: stack, all CLI commands (install, dev, build, test, lint, deploy), `just` recipes, `package.json` scripts, `Makefile` targets, code style, architecture, conventions, and contribution workflow.

Inputs: existing `AGENTS.md` (required), package manifests, lock files, scripts, `justfile`, `Makefile`. Outputs: rewritten `AGENTS.md`, refreshed `CLAUDE.md` symlink.

Recognised flags: `--dry-run`, `--preserve`, `--thorough`, `--minimal`.

See [references/update-agents.md](references/update-agents.md).

## Update README

When to use: user asks to update or refresh an existing README.md. Trigger phrases include "update README", "refresh README", "fix README", "regenerate README".

If `README.md` does not exist, route to **Initialize README** instead (or, with `--force`, allow update-readme to create it).

README owns: description, badges, links (homepage, docs site, demo, package registry), references, related work, acknowledgments, license, contributing pointer. It does **not** contain CLI commands, `just` recipes, scripts, or project structure trees — those live in AGENTS.md, and the README links to AGENTS.md for them.

Inputs: existing `README.md` at the repo root (`git rev-parse --show-toplevel`); package manifests for name/version/description/license/homepage URL; git remote for repository URL. Outputs: rewritten `README.md`.

Recognised flags: `--dry-run`, `--preserve`, `--minimal`, `--thorough` (alias `--full`).

See [references/update-readme.md](references/update-readme.md).

## Initialize README

When to use: user asks to create a new README.md from scratch in a repository that lacks one. Trigger phrases include "init README", "create README", "new README", "generate a README".

Refuses to overwrite an existing `README.md` without `--force` or explicit confirmation via `AskUserQuestion`. Supports two operating modes:

- **Automatic inference**: derive content entirely from project analysis.
- **Guided**: focus content around a user-provided description (e.g., "TypeScript library for parsing dates with zero deps").

Same audience rules as Update README: humans only, no CLI.

Inputs: codebase analysis (language, framework, LICENSE, homepage URL, citations or papers in repo), optional user-provided description. Outputs: new `README.md` at repo root.

Recognised flags: `--dry-run`, `--minimal`, `--full`, `--force`.

See [references/init-readme.md](references/init-readme.md).

## Initialize Context

When to use: user asks to create a new AGENTS.md (and CLAUDE.md symlink) from scratch in a repository that lacks context documentation. Trigger phrases include "init AGENTS.md", "create CLAUDE.md", "init context", "new context file", "generate AGENTS.md".

Like Initialize README, supports automatic inference and guided mode (e.g., "Foundry smart contract project with security-first mindset"). On completion, always creates the `CLAUDE.md` symlink via `ln -sf AGENTS.md CLAUDE.md`.

Generated AGENTS.md must include a Commands section that consolidates every CLI invocation a developer or agent will need: install, dev, build, test, lint, format, deploy, plus all `just` recipes, npm/pnpm/yarn/bun scripts, and Makefile targets discovered in the repo.

Inputs: codebase analysis (stack, scripts, `justfile`, `Makefile`, architecture hints, existing `package.json`, `README.md`, language-specific manifests), optional user-provided description. Outputs: new `AGENTS.md`, `CLAUDE.md` symlink.

Recognised flags: `--dry-run`, `--minimal`, `--full`, `--force`.

See [references/init-agents.md](references/init-agents.md).

## Reporting

Every workflow ends with a short summary. Use these conventions across all workflows:

- `✓` for successful operations: `✓ Updated AGENTS.md` followed by indented bullet points listing concrete changes.
- `⊘` for skipped optional files.
- `⚠` for advisory notices (e.g. CONTRIBUTING.md merge recommendation).
- `✗` for failures: `✗ Failed to write README.md` with a one-line cause.

Indent change details under each line so the user can scan a single file's deltas without re-reading the header. For `--dry-run`, prefix the report with a "Planned Changes" header and include the diff or proposed-content preview rather than a confirmation. Refer to `references/common-patterns.md` for full report templates.

## Additional Resources

For detailed workflows, examples, and implementation guidance, refer to these reference documents:

- **`references/common-patterns.md`** — Audience split, argument parsing, writing style, report formatting, file detection, metadata extraction, CONTRIBUTING.md merge recommendation
- **`references/update-agents.md`** — Complete context file update workflow including verification strategies, command discovery, and discrepancy detection
- **`references/update-readme.md`** — Complete README update workflow for human-aimed content
- **`references/init-readme.md`** — Complete README initialization workflow for human-aimed content
- **`references/init-agents.md`** — Complete context initialization workflow including language-specific templates and commands consolidation

These references provide implementation details, code examples, and troubleshooting guidance for each workflow type.
