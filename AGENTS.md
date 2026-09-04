# Agent Skills

Collection of self-contained agent skills for Claude Code, Codex, and compatible agents. Keep `README.md` minimal and
human-facing; put maintainer and agent guidance here.

## Model Optimization

Optimize every skill and other agent-facing content for GPT-6 Astra and Claude Fable 5.1. The summaries below are
reminders, not substitutes for the live guides. Read both guides before complex, long-running, multi-tool, or
orchestration-heavy work because their recommendations may evolve.

- [GPT-6 Astra prompting guidance](https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra#prompting-best-practices):
  Complete authorized work under stated assumptions; make user-instruction precedence over skills explicit; specify
  writing and delegation preferences; and keep verification proportional to the change.
- [Claude Fable 5.1 prompting guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1):
  Calibrate effort with evals; request progress updates; batch independent tool calls; preserve decisions across
  compaction; verify changing facts; and finish the requested scope with targeted edits and proportionate tests.

## User-Facing Communication

Treat visual structure as information architecture, not decoration.

- Use a small semantic status vocabulary consistently: `🔎` preview/read-only, `⏳` running, `✅` verified success, `⚠️`
  caveat/approval/risk, `⛔` blocked/not written, `❓` unknown, and `↩` reverted/rolled back. When a status icon is
  used, pair it with a status word; never make color or emoji the only carrier of meaning.
- Lead reports with one outcome line. Use at most one non-status domain icon per heading for identity, then add compact
  headings for changed artifacts, verification, and risks only when those sections are useful.
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
  evidence sent to the configured model provider. Prefer efficient, complete, direct evidence; do not require redaction,
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
- `README.md` lists every skill and stays minimal.
- `CLAUDE.md` is a symlink to `AGENTS.md`; do not edit it separately.

## Commands

Run `just` to list every recipe with its description; the `justfile` is authoritative. Notable, non-obvious facts:

- After editing Markdown, run `just prettier-write` then `just prettier-check`, in that order; if `prettier-check`
  fails, fix only the files you changed.
- `package.json` exists only for local formatting and hook wiring; there is no build step.
- Treat Markdown formatting, invocation metadata checks, and skill-specific helper scripts as the verification surface
  unless a task introduces a narrower check.

## Resource-Safe Search

- Scope `fd`, `rg`, `grep`, and similar searches to the narrowest useful root. Exclude known dependency, build, cache,
  generated, and state directories before broadening; plain bounded searches with these tools are appropriate.
- Do not run per-result commands over unknown or high-cardinality sets with `fd -x`/`--exec`, `find -exec`, parallel or
  unbounded `xargs`, or shell loops that launch one tool per path. Prefer native predicates or metadata output; preview
  or sample cardinality, then use a bounded batch when downstream execution is necessary.
- Stream and bound large search results instead of capturing arbitrary full lines, JSON events, or generated-file output
  in memory or repeatedly rescanning it. Long-running helpers must propagate cancellation and terminate or clean up
  child processes.

## Rules

- In this repository, an unqualified request to create, scaffold, or initialize a skill means an installable catalog
  skill under `skills/<name>/`. Never route it to `.agents/skills/` or the `skill-writing` workflow.
- Create a repo-private internal skill under `.agents/internal-skills/<name>.md` only when the user explicitly requests
  an internal skill.
- When asked to edit or remove an installable catalog skill while the current working directory is this repo, modify the
  skill under `skills/` here only, not the installed copy under `~/.agents`.
- Changes here are not live until installed into every target declared by the skill. By default, `publish-skills`
  reconciles all current source-owned drift; an explicit commit range narrows that reconciliation to the affected skill
  names (scope rules: `@publish-skills`).
- At the end of a successfully completed user task that edits installable catalog skills, run the `publish-skills`
  internal skill on the user's behalf. First inspect `ai-coord status --json`: consider every active work row, and do
  not run this step if another agent has a queued claim overlapping any active claim. Commit and push catalog source
  changes under the source-repository claim, then release that claim before acquiring targets: home-directory targets
  sort before this source repository. Otherwise, if the task was super complex or the working tree has ongoing dirty
  changes, recommend `@publish-skills` instead. The agent must make the complexity assessment.
- When creating, renaming, or deleting a catalog or internal skill, follow `@skill-lifecycle`. For catalog creation,
  also follow `@skill-authoring`. `just readme-skills-check` must pass.
- Before creating or editing `SKILL.md` frontmatter, `agents/openai.yaml`, `metadata.install-targets`, or
  `skill-dependencies`, read `@skill-authoring` — it is the authoritative metadata reference; do not guess field
  semantics.
- Internal skills are special repo-private runbooks. Place them under `.agents/internal-skills/<name>.md`, not under
  `skills/`. Do not add them to `README.md`, do not create `agents/openai.yaml`, and do not treat them as installable
  catalog skills.
- After editing skills that must stay aligned, run the `sync-skills` internal skill to check coupled skills and helper
  data.
- Keep skills self-contained. Do not de-duplicate content across skills by extracting shared references or canonical
  files; users install skills individually.
- Keep globally installed skills self-contained. Do not refer to or depend on another repository; put reusable guidance
  directly in the owning skill and discover target-project conventions at runtime. Refer to an external repository only
  when it is genuinely required to perform the skill's task.
- Write skill content for end users and other repos, not for this repo. Skills must not assume this repo's own tooling
  (e.g. `just prettier-write`, `just skill-invocation-check`) is present elsewhere; have skills detect and use whatever
  the target repo provides instead of naming this repo's recipes.
- Resolve `references/`, `scripts/`, `examples/`, and `assets/` paths relative to the owning skill directory.
- Bash scripts must be compatible with Bash v3.2 (`/bin/bash`) because macOS ships that system version and skills may
  run in Bash-based environments.
- Keep generated docs terse, imperative, and expert-to-expert.
- Never leak personal crypto (EVM) addresses in any skill. Use well-known public addresses or the standard Etherscan doc
  example (`0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe`); never a real user, maintainer, or personal wallet address.
  Same for private keys, mnemonics, and API keys — use env-var placeholders (`$ETH_PRIVATE_KEY`), never literal secrets.
