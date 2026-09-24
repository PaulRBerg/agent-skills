---
argument-hint: "<repo-path> <repo-path> [more-repos...]"
compatibility: Requires git and Bun; just is optional for listing justfile recipes.
disable-model-invocation: true
metadata:
  install-targets: claude-code codex
name: repo-cross-pollination
skill-dependencies:
  - codex-handoff
  - commit
description:
  Compare agent guidance, workflows, and dependencies across repositories, then adopt the practices each recipient would
  benefit from.
---

# Repo Cross-Pollination

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Turn a list of repositories into evidence-backed transfer candidates: agent guidance, workflows, and dependencies that
one repository uses well and another would benefit from. Then implement the user-approved transfers, adapted to each
recipient's conventions.

This skill carries practices between repositories. Aligning drifted copies, stale cross-references, or duplicated
content among interdependent repositories is `$repo-harmonization` work; when the request is about that, say so and
stop.

## Arguments

`$ARGUMENTS` is a whitespace-separated list of repository paths. Tilde expansion is allowed.

- Require at least two paths, each an existing Git repository root, resolving to distinct repositories. The inventory
  helper enforces these rules and exits 2 with the offending inputs; stop and relay its message.
- Do not infer additional repositories from links, remotes, or installed copies; the supplied list defines the boundary.

## Contract

- Research before any edit and keep research read-only; parallelize across subagents when the host supports them,
  otherwise investigate serially.
- Treat every repository as both donor and recipient. Evaluate each candidate per direction: donor, practice, recipient.
- A transfer unit is one practice: a guidance rule or project skill, a workflow (task recipe, script, CI job, hook,
  toolchain pin, lint, format, type-check, test, or release configuration), or a dependency together with the workflow
  or code that uses it. Never transfer a dependency the recipient would not use.
- Attach evidence to every candidate: donor file:line showing the practice and that it is exercised (called by a task,
  CI, a hook, or code); recipient search commands proving the gap, including functional equivalents under other names;
  and recipient evidence of the need the practice addresses.
- Adapt rather than copy. Rewrite each transfer for the recipient's language, runtime versions, package manager, task
  runner, naming, file layout, formatting, and guidance voice. Transferred guidance names only commands, paths, and
  tools that exist in the recipient once the plan is applied.
- Classify each candidate exactly once: recommended transfer, judgment call, or rejected with a reason.
- Never carry donor secrets, credentials, personal identifiers, hostnames, addresses, or project-specific names and
  assumptions into a recipient.
- Prefer the smallest transfer that delivers the benefit: a task recipe over a new script, a config file over a wrapper,
  a short rule over a copied document. Shared packages, template repositories, sync pipelines, and submodules are
  judgment calls.
- Treat the approved transfers, not the initial file manifest, as the implementation authorization boundary. The
  orchestrator may add technical prerequisites those transfers need (a dev dependency, lockfile update, config stub, or
  ignore entry), acquire coordination for the new scope, and delegate the smallest sufficient fix without asking again.
  Workers stop at their assigned write scopes and return evidence to the orchestrator.
- Respect every recipient's generation pipelines and hooks; edit canonical sources only and let designed commit hooks
  run at commit time.
- Keep implementation agents from committing or pushing; the orchestrator commits per repository, scopes commits to task
  files, and honors the host's coordination and shared-worktree rules.
- Report progress at phase changes with measured counts (repositories inventoried, candidates found, verified,
  classified, implemented) rather than elapsed time.

## Intake

Run the inventory helper once with the supplied paths, resolving the script relative to this skill directory:

```bash
bun run "<skill-dir>/scripts/inventory.ts" <repo-path> <repo-path> [more-repos...]
```

It prints one JSON document (`schemaVersion: 1`) to stdout, exits 2 on invalid input, and has no other side effects:

- `repos[]`: `id`, `input`, `root`, `head`, `dirty` (porcelain lines), `packageManagers`, `ecosystems` (parsed
  dependency count per ecosystem), `files.guidance`, `files.workflow`, `files.manifest` (tracked paths), `tasks[]`
  (`runner` is `just` or `package.json`, with `file` and `name`), and `notes` for anything it could not parse.
- `dependencyGaps[]`: dependencies present in some but not all repositories sharing that ecosystem, with `presentIn`
  (repo, manifest, section, spec, use count) and `missingIn`. It parses `package.json`, `pyproject.toml` (uv:
  `[project]`, `[dependency-groups]`, `[tool.uv]`), `Cargo.toml`, and `go.mod` (direct `require` and `tool` entries),
  skipping workspace-local packages.
- `taskGaps[]`: task names present in some but not all repositories.
- `summary`: shared and gap counts per ecosystem and for tasks.

Save the JSON under the host's scratch or temporary directory and query it with `jq` rather than reloading it whole.

- Treat each repository's `dirty` list as other agents' in-flight work: a preservation boundary, never a transfer source
  or target.
- Treat the file lists as a starting inventory. Guidance covers `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`,
  `CONTRIBUTING.md`, Copilot and Cursor rules, `.mcp.json`, and entry points under `.agents`, `.claude`, `.codex`,
  `.cursor`, `.gemini`, and `.windsurf`; workflow covers task runners, CI, hooks, toolchain pins, and tool configs. Skim
  each repository root and CI directory for surfaces it missed, and read manifests it lists without parsing, such as
  `requirements*.txt`, `foundry.toml`, `.gitmodules`, `Gemfile`, and `composer.json`.
- Read each repository's README and top-level guidance to record its profile: purpose (library, app, CLI, service,
  catalog), languages, runtime and toolchain versions, package manager, task runner, CI provider, and configured agent
  clients. Fit decisions depend on this profile.

## Research

- Partition the repositories across no more than three read-only investigators. Give each the same repository boundary,
  helper JSON path, and dirty lists, and require evidence, not proposed edits.
- Each investigator returns a practice inventory for its repositories:
  - Guidance: rules, conventions, commands, project skills, hooks, permissions, and MCP servers, each marked portable or
    project-specific.
  - Workflows: tasks with the commands they run and what they check or produce; CI jobs and triggers; hooks; toolchain
    pins; lint, format, type-check, test, coverage, security, dependency-update, and release tooling.
  - Dependencies: the gap-matrix entries relevant to tooling or shared concerns, with how the repository uses each
    (config, task, or import at file:line).
- Build the cross-repository matrix in the orchestrator. For each practice, record which repositories have it, an
  equivalent, or nothing. Establish equivalence by function, not name: `oxlint` versus `eslint`, `just check` versus
  `npm run lint`, `ruff` versus `flake8` plus `black`.
- Verify every recipient gap mechanically by searching for the tool name, config files, commands, and functional
  equivalents; record the commands and results.
- Assess fit per direction: ecosystem and runtime compatibility (`engines`, `requires-python`, `rust-version`, the `go`
  directive, peer constraints), recipient purpose, conflicting tools or documented policy, and maintenance or CI cost.
  Concept-level practices may cross ecosystems (a type-check gate, a scoped-formatting rule); dependencies transfer only
  within an ecosystem or as an approved ecosystem-equivalent tool.
- Skip guidance the recipient already receives from a broader scope, such as global or parent-directory instruction
  files loaded in the agent's context; duplicating it adds context without changing behavior.
- For tooling candidates, preview impact read-only when cheap: run the tool through an ephemeral runner (`bunx`, `npx`,
  `uvx`, `go run`, `cargo` in check mode) without writing repository files, confirm with `git status --short`, and
  attach counts such as violations found and autofixable.
- Check current registry versions and runtime requirements through the recipient's package manager at research time;
  recheck when implementing.

## Evaluate

- Merge candidates for the same practice from multiple donors; pick the strongest donor implementation as the model and
  cite the others.
- Return an unverified suspicion to research or record it as an open question; do not classify it.
- Recommended transfers: clear benefit, verified gap, compatible ecosystem, no conflict with an existing tool or
  documented policy, and low cost.
- Judgment calls: replacing or retiring an existing recipient tool; introducing a new ecosystem, runtime, task runner,
  service, or required secret; material CI cost; license concerns; policy-changing guidance such as commit, release, or
  review rules; and pre-existing recipient violations surfaced by a new check (fix, baseline, or drop).
- Rejected: ecosystem mismatch, project-specific practice, equivalent already present, low value, or conflict with the
  recipient's documented policy. Keep this list with evidence for the report.

## Decide

- Present a compact table of recommended transfers with donor, recipients, evidence, benefit, and cost; offer them as
  one default-accept batch that the user can trim.
- Present every judgment call as an explicit question with a recommended option and its tradeoff; keep alternatives
  mutually exclusive when the choice determines the plan.
- Do not write the implementation plan until the user resolves every decision that changes scope or approach.
- Record each decision verbatim beside its candidate, preserve its conditions, and move declined candidates to the
  rejected list.

## Plan

- Produce a decision-complete plan per recipient: exact files to create or edit, the donor source each adapts
  (file:line), dependencies with the package-manager command that adds them, verification commands, and the approving
  decision for each change.
- Order each recipient's edits so dependencies precede the configs and tasks that use them, and guidance lands last so
  it documents what exists.
- In Claude Code, prefer plan mode.
- Delegate implementation through `$codex-handoff` when available; otherwise use host subagents, otherwise implement
  directly. Use disjoint per-repository write scopes, and reserve cross-repository checks for one owner.
- Keep the plan limited to approved transfers. Extend it autonomously for technical prerequisites; ask only when a new
  finding introduces a subjective choice, changes the repository set or intended outcome, or crosses an unapproved
  destructive, publish, or other external-write boundary.

## Implement and Finalize

- Add dependencies through the recipient's package manager (for example `bun add -d`, `pnpm add -D`, `uv add --dev`,
  `cargo add`, `go get -tool`) at the latest stable version compatible with its constraints, matching its range and
  pinning style; let the package manager update the lockfile.
- Adapt donor configs by dropping options that reference donor-only paths, names, or features.
- Match the recipient's CI conventions: provider, action pinning style, runners, caching, and least-privilege
  permissions.
- When a guidance file is a symlink or generated artifact, edit its canonical source. Transfer a project skill by
  adapting it into the recipient's skill directory, or through the same install mechanism when the donor installs it
  from a shared catalog.
- Run every new or changed task once in the recipient and confirm it does the job the donor's version does. A
  transferred check passes on the recipient, or the user's decision for pre-existing failures is applied.
- Validate CI and hook changes with an available local linter or dry run, such as `actionlint`; state explicitly when
  only a CI run can prove them.
- Verify transferred guidance: every command, path, and tool it names exists in the recipient.
- Reconcile each implementation-agent result against the working tree and its assigned write scope; preserve
  pre-existing dirt and unrelated concurrent changes byte-for-byte.
- Run the recipient's narrowest checks that prove the edits, from that repository. Attribute a failure before acting:
  rule out this task's changes, formatters, hooks, and generators first, and continue past a failure only when evidence
  shows it is unrelated and the task's own checks pass.
- Use `$commit` when available after validation, passing only task files; never bypass it with `git add -A`,
  `git commit -a`, stash, or reset. Commit per repository and, where practical, per transfer so each stays independently
  revertible.

## Report

- Lead with one outcome line: transfers applied per recipient and any remaining blocker.
- Tabulate applied transfers: recipient, practice, donor source, changed files, and commit identifier.
- Report every verification command and its outcome; distinguish checks not run and explain why, including CI-only
  verification.
- List rejected and declined candidates with their reasons in one compact table.
- State residual risks and open questions discovered after the decision boundary.
- Keep the report auditable: tie every conclusion to its recorded evidence, decision, or command outcome.
