# Agent Skills

This repository contains self-contained agent skills for Claude Code, Codex, and compatible agents. Keep `README.md`
minimal and human-facing. Put maintainer and agent guidance here.

## Model Optimization

Optimize every skill and other agent-facing content for GPT-6.1 Sol and Claude Opus 5.5. The summaries below remind you
of the live guidance. They do not replace the guides. Read both guides before complex, long-running, multi-tool, or
orchestration-heavy work. Their recommendations may change.

- [GPT-6.1 Sol prompting guidance](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices)
  (shared GPT-6 guide): Evaluate its family-wide recommendations on Sol. Complete authorized work under stated
  assumptions. State that user instructions take precedence over skills. Specify writing and delegation preferences.
  Keep verification proportional to the change.
- [Claude Opus 5.5 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5):
  Calibrate effort instead of prompting for more thinking. Never ask for reasoning in response text. Name the premature
  stops to avoid and the stops that are wanted. Treat text-only turns as reports, not completion.

  Request brief progress updates. Explore relevant sources before acting. Name concrete patterns to avoid instead of
  giving generic style advice.

## User-Facing Communication

Treat visual structure as information architecture, not decoration.

- Use a small semantic status vocabulary consistently: `🔎` preview/read-only, `⏳` running, `✅` verified success, `⚠️`
  caveat/approval/risk, `⛔` blocked/not written, `❓` unknown, and `↩` reverted/rolled back. When a status icon is
  used, pair it with a status word. Never make color or emoji the only carrier of meaning.
- Lead reports with one outcome line. Use at most one non-status domain icon per heading for identity. Then add compact
  headings for changed artifacts, verification, and issues only when those sections are useful.
- Use `Issues and caveats` for report findings, grouped as `Resolved` (verified fixes) and `Open` (remaining problems,
  limitations, or unverified assumptions). State resolution evidence or the remaining impact and next step. Omit empty
  groups. Report each item once. Reserve `blocker` for something preventing required work and `risk` for a specific
  potential adverse outcome. Put neutral context and agreed decisions under changes or scope.
- Use tables for repeated fields, trees for real hierarchy or file structure, and progress bars only when a measured
  numerator and denominator exist. Never imply progress from elapsed time, activity, or guesswork.
- Keep JSON, JSONL, DOT, CSV/TSV, exact commands, confirmation tokens, identifiers, addresses, hashes, signatures,
  diagnostics, logs, copied prompts, and other machine-consumed or verbatim content undecorated. Keep safety and
  rollback wording direct.
- Decoration belongs to the agent's status wrapper. Do not add emoji or ASCII ornament to source code, generated
  artifacts, product UI copy, GitHub contributions, spreadsheet data, or user-authored content unless the task itself
  calls for it.

## Evidence and Disclosure

- Internal agent communication includes task reports, tool output, current-project transcript inspection, and material
  evidence sent to the configured model provider. Prefer efficient, complete, direct evidence. Do not require redaction,
  local-only handling, or excerpt suppression solely because the provider can see it.
- Before posting, uploading, publishing, emailing, releasing, or otherwise sending data outside the agent workspace,
  perform an external-disclosure review. Remove credentials and secrets, unrelated personal or customer data, unsuitable
  private paths or repository names, and unrelated transcript material.
- This distinction never relaxes secret handling or the no-personal-EVM-address rule under **Rules**.

## Structure

- `skills/<name>/SKILL.md` is the skill entrypoint.
- `skills/<name>/references/` contains skill-local reference docs.
- `skills/<name>/scripts/` contains executable helpers.
- `skills/<name>/agents/openai.yaml` contains Codex-specific metadata, including for skills not installed into Codex.
- `skills/<name>/examples/` contains sample files.
- `skills/<name>/assets/` contains bundled media or other static assets.
- `.agents/internal-skills/<name>.md` contains repo-private internal skills referenced with `@`.
- `tests/<name>/` contains tests for that skill's helpers. `scripts/` contains catalog tooling.
- `toolkit/` is the Rust workspace and Bun apps that own the `ai-*` CLIs the skills and the global coordination workflow
  rely on. Follow `toolkit/AGENTS.md` there.
- `README.md` lists every skill and stays minimal.
- Claude Code reads `AGENTS.md` directly. Do not add a `CLAUDE.md`.

## Commands

Run `just` to list every recipe with its description. The `justfile` is authoritative. These command details are not
obvious:

- After editing Markdown, run `just prettier-write <changed files>` then `just prettier-check <changed files>`, in that
  order. If `prettier-check` fails, fix only the files you changed.
- The root `package.json` exists only for local formatting, type-checking, and hook wiring. There is no build step.
- CLI changes go live only via `just toolkit::install-cli`, under the install-authorization rules in
  `toolkit/AGENTS.md`.

## Local Verification

All checks run locally. Do not add CI workflows or rely on remote checks for completion.

- During iteration, run focused checks for the changed behavior. Before committing, run the affected owner's complete
  local gate below, including dependent consumers when an interface or shared configuration changes.
- Run the final gate once after completing edits. Repeat only after further changes, a failure, or an unresolved
  concern. Documentation-only changes need formatting and verification of changed commands, paths, and claims, not
  unrelated runtime suites.
- Formatting, pre-commit hooks, successful installation, and live smoke checks do not replace required tests,
  type-checks, linting, or builds. Add regression coverage when changing behavior. Keep tests proportional to the
  change.
- Keep formatters and fixers scoped to files edited in this session. Attribute failures before debugging. If an
  aggregate fails only in untouched concurrent work, verify your own scope and report the limitation.
- Report the exact check commands and outcomes, including failures and skipped checks. Do not claim verification from an
  unrun command or finish while a required check is still running.

| Change scope                               | Required local verification                                                                                              |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Skill metadata, dependencies, README table | Run `just skill-check`. Verify referenced files and documented commands for changed skills.                              |
| Shell helpers or tests                     | Run `just shell-check <changed files>` and the affected shell tests. Use `just bats-test tests/<skill>` for Bats suites. |
| Python helpers or tests                    | Run the affected `tests/<skill>/test*.py` scripts with `uv run`.                                                         |
| Root Bun TypeScript helpers                | `just typescript-check` and the affected tests (`just publish-skills-test` or `just evm-atlas-test`).                    |
| evm-atlas generated inputs or references   | Run `just evm-atlas-check`. Change canonical inputs and regenerate derivatives rather than editing generated files.      |
| Shared catalog tooling or test wiring      | `just test` and `just skill-check`, plus shell linting for changed shell files.                                          |
| Toolkit Rust or Bun applications           | Follow `toolkit/AGENTS.md` and the owning package's validation rules.                                                    |

Skill prose changes also require checking affected reference links, helper usage, and examples. Run helper tests when
the instructions change how a helper is used. Markdown changes always follow the formatting sequence under **Commands**.

## Resource-Safe Search

- Scope `fd`, `rg`, `grep`, and similar searches to the narrowest useful root. Exclude known dependency, build, cache,
  generated, and state directories before broadening. Plain bounded searches with these tools are appropriate.
- Do not run per-result commands over unknown or high-cardinality sets with `fd -x`/`--exec`, `find -exec`, parallel or
  unbounded `xargs`, or shell loops that launch one tool per path. Prefer native predicates or metadata output. Preview
  or sample cardinality. When downstream execution is necessary, use a bounded batch.
- Stream and bound large search results instead of capturing arbitrary full lines, JSON events, or generated-file output
  in memory or repeatedly rescanning it. Long-running helpers must propagate cancellation and terminate or clean up
  child processes.

## Rules

- Before writing or changing agent-facing prose, read
  [the STE authoring profile](skills/skill-writing/references/asd-ste100.md) completely. Apply the profile during
  writing. Complete its meaning and style review before finishing. Preserve obligation strength and protected technical
  content. This review does not establish official ASD dictionary compliance.
- In this repository, an unqualified request to create, scaffold, or initialize a skill means an installable catalog
  skill under `skills/<name>/`. Never route it to `.agents/skills/` or the `skill-writing` workflow.
- Create a repo-private internal skill under `.agents/internal-skills/<name>.md` only when the user explicitly requests
  an internal skill.
- Edit or remove installable catalog skills under `skills/` here, regardless of the session's starting repository.
  Installed copies belong to the publish workflow.
- Changes here are not live until installed into every target declared by the skill. By default, `publish-skills`
  reconciles all current source-owned drift. An explicit commit range or automatic continuous skill maintenance narrows
  reconciliation to the affected skill names. See `@publish-skills` for scope rules.
- After validating edits to installable catalog skills, run the `publish-skills` internal skill on the user's behalf,
  including for independent repairs made during a blocked main task. Complete publication unless a concrete blocker
  prevents it. Task complexity and unrelated dirty files are not reasons to leave validated repairs unpublished.
- Before publication, inspect `ai-coord status --json`. Consider every active work row. Commit and push catalog source
  changes under the source-repository claim. Then release that claim before acquiring targets. Home-directory targets
  sort before this source repository. If another agent has a queued claim overlapping any active claim, resolve or wait
  for that conflict to end before publishing.

  Require `READY` for the complete target claim set in `@publish-skills`. This source-then-target sequence is the
  documented exception to the global rule that multi-root writes take one `ai-coord bundle start`. The target set itself
  still uses one bundle when it spans two or more roots.

- When creating, renaming, deleting, or shelving a catalog or internal skill, follow `@skill-lifecycle`. For catalog
  creation, also follow `@skill-authoring`. `just skill-check` must pass.
- Before creating or editing `SKILL.md` frontmatter, `agents/openai.yaml`, `metadata.install-targets`, or
  `skill-dependencies`, read `@skill-authoring`. It is the authoritative metadata reference. Do not guess field
  semantics.
- Internal skills are special repo-private runbooks. Place them under `.agents/internal-skills/<name>.md`, not under
  `skills/`. Do not add them to `README.md`. Do not create `agents/openai.yaml`. Do not treat them as installable
  catalog skills.
- After editing skills that must stay aligned, run the `sync-skills` internal skill to check coupled skills and helper
  data.
- When an installed CLI outpaces a `cli-*` skill's `references/version.txt`, refresh it with `@refresh-cli-skill`.
- Claude Code slash commands under `~/.claude/commands/<skill>/*.md` are thin wrappers over catalog skills (currently
  `agents-brain` and `yeet`). After changing a skill's workflows, argument forms, context requirements, or reference
  file names, update the matching commands in the same session. Argument hints, descriptions, `## Context` reads, and
  reference paths must match the skill. Every user-facing workflow needs a command. Commit them in `~/.claude`.
- Keep skills self-contained. Do not remove duplicate content across skills by extracting shared references or canonical
  files. Users install skills individually.
- Keep globally installed skills self-contained. Do not refer to or depend on another repository. Put reusable guidance
  directly in the owning skill. Discover target-project conventions at runtime. Refer to an external repository only
  when it is required to perform the skill's task.
- Write skill content for end users and other repos, not for this repo. Skills must not assume this repo's own tooling
  (e.g. `just prettier-write`, `just skill-check`) is present elsewhere. Have skills detect and use whatever the target
  repo provides instead of naming this repo's recipes.
- Resolve `references/`, `scripts/`, `examples/`, and `assets/` paths relative to the owning skill directory.
- Bash scripts must be compatible with Bash v3.2 (`/bin/bash`) because macOS ships that system version and skills may
  run in Bash-based environments.
- Keep generated docs terse, imperative, and expert-to-expert.
- Never leak personal crypto (EVM) addresses in any skill. Use well-known public addresses or the standard Etherscan doc
  example (`0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe`). Never use a real user, maintainer, or personal wallet address.
  For private keys, mnemonics, and API keys, use env-var placeholders (`$ETH_PRIVATE_KEY`). Never use literal secrets.
