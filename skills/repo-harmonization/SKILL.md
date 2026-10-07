---
argument-hint: "<repo-path> <repo-path> [more-repos...]"
compatibility: Requires git and Bun. just is optional for listing justfile recipes.
disable-model-invocation: true
metadata:
  install-targets: claude-code codex
name: repo-harmonization
skill-dependencies:
  - commit
  - orchestration
description:
  Audit two or more interdependent repositories for drift, duplication, and practices one repository uses well and
  another lacks. Verify every finding mechanically, decide with the user, then apply surgical alignment fixes and
  adapted transfers per repository.
---

# Repo Harmonization

If a slash or dollar invocation already added these instructions to the conversation, follow them directly. In that
case, do not invoke this skill again through a skill tool.

Use a list of interdependent repositories to produce verified findings, user-approved decisions, and surgical changes.
The audit covers two kinds of work:

- Alignment: drift, stale cross-references, duplicated content, and competing workflows among the repositories.
- Transfer: agent guidance, workflows, and dependencies that one repository uses well and another would benefit from.
  Adapt each transfer to the recipient's conventions.

## Arguments

`$ARGUMENTS` is a whitespace-separated list of repository paths. Tilde expansion is allowed.

- Require at least two paths, each an existing Git repository root, resolving to distinct repositories. The inventory
  helper enforces these rules and exits 2 with the offending inputs. In that case, stop and relay its message.
- The user's request, not `$ARGUMENTS`, may narrow the focus to alignment or to transfers. Without such a request, do
  both. Pass only repository paths to the inventory helper.
- Do not infer additional repositories from links, remotes, or installed copies. The supplied list defines the audit
  boundary.

## Contract

- Research before any edit and keep research strictly read-only. When the host supports subagents, parallelize across
  them. Otherwise, investigate serially.
- Verify every alignment or duplication claim mechanically with a diff, content hash, regeneration round-trip, or
  symlink resolution. Never rely on prose inspection alone. Attach file:line evidence to every finding.
- Treat every repository as both donor and recipient. Evaluate each transfer candidate per direction: donor, practice,
  recipient.
- A transfer unit is one practice: a guidance rule or project skill, a workflow (task recipe, script, CI job, hook,
  toolchain pin, lint, format, type-check, test, or release configuration), or a dependency together with the workflow
  or code that uses it. Never transfer a dependency the recipient would not use.
- Attach evidence to every transfer candidate. Include donor file:line showing the practice and that it is exercised
  (called by a task, CI, a hook, or code). Also include recipient search commands proving the gap, including functional
  equivalents under other names, and recipient evidence of the need the practice addresses.
- Classify each material finding exactly once, with the classes in
  [Consolidate and Evaluate](#consolidate-and-evaluate).
- Adapt rather than copy. Rewrite each transfer for the recipient's language, runtime versions, package manager, task
  runner, naming, file layout, formatting, and guidance voice. Transferred guidance names only commands, paths, and
  tools that exist in the recipient once the plan is applied.
- Never carry donor secrets, credentials, personal identifiers, hostnames, addresses, or project-specific names and
  assumptions into a recipient.
- Prefer surgical fixes to restructuring. Unless it removes more complexity than it adds, introduce no include pipeline,
  templating layer, shared reference, or other machinery.
- Prefer the smallest transfer that delivers the benefit: a task recipe over a new script, a config file over a wrapper,
  a short rule over a copied document. Shared packages, template repositories, sync pipelines, and submodules are
  judgment calls.
- Send every judgment call to the user before planning. Never expand beyond the approved decisions.
- Treat the approved outcome and resolved judgment calls, not the initial file manifest, as the implementation
  authorization boundary. When implementation discovers a related in-repository prerequisite needed to carry out those
  decisions, the orchestrator may extend the manifest, acquire coordination for the new scope, and delegate the smallest
  sufficient follow-on fix without asking again. Examples are a dev dependency, lockfile update, config stub, or ignore
  entry. Workers still stop at their assigned write scopes and return the evidence to the orchestrator.
- Respect every repository's generation pipelines and hooks. Edit canonical sources only. Regenerate artifacts through
  build-only non-committing paths. Let designed commit hooks run at commit time.
- Keep implementation agents from committing or pushing. The orchestrator commits per repository, scopes commits to task
  files, and honors the host's coordination and shared-worktree rules.
- Report progress at phase changes with measured counts (repositories inventoried, and findings found, verified,
  classified, and implemented) rather than elapsed time.

## Intake

Run the inventory helper once with the supplied paths, resolving the script relative to this skill directory. Run it
before any command that might regenerate, install, or otherwise modify a worktree:

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

- Use each `root` as the checked repository root in later evidence. Keep each `input` form for the report.
- Treat each `dirty` list as the pre-research `git status` snapshot. Treat that pre-existing dirt as other agents'
  in-flight work: a preservation boundary, never an audit target, transfer source, or transfer target.
- Record each repository's current branch and remote configuration only when those facts affect an install, publish, or
  cross-repository reference.
- Treat the file lists as a starting inventory. Guidance covers `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`,
  `CONTRIBUTING.md`, Copilot and Cursor rules, `.mcp.json`, and entry points under `.agents`, `.claude`, `.codex`,
  `.cursor`, `.gemini`, and `.windsurf`. Workflow covers task runners, CI, hooks, toolchain pins, and tool configs. Skim
  each repository root and CI directory for surfaces it missed, and read manifests it lists without parsing, such as
  `requirements*.txt`, `foundry.toml`, `.gitmodules`, `Gemfile`, and `composer.json`.
- Read each repository's README and top-level guidance to record its profile: purpose (library, app, CLI, service,
  catalog), languages, runtime and toolchain versions, package manager, task runner, CI provider, and configured agent
  clients. Fit decisions depend on this profile.

Build a first-pass link map before making conclusions. Treat the link map as an inventory, not proof that two files
should share one source. Record unknown origins as unknown. Path similarity does not establish a relationship.

- Map symlinks that point between the listed repositories, including their resolved targets. Resolve each symlink from
  its containing repository to distinguish an intentional external symlink from a broken one.
- Map files generated from another listed repository. Identify the stated source when available. Record the generator
  command or manifest entry that establishes each generated relationship.
- Map install and publish flows that move artifacts, skills, packages, or configuration between the repositories. Note
  whether each flow changes files, creates external state, or only validates a source artifact.
- Search AI-context files for references to the other listed repositories, including `AGENTS.md`, `CLAUDE.md`, README
  guidance, skills, and local instructions. Capture the concrete path, command, or repository identifier rather than
  only its surrounding prose.

## Research

- Partition the repositories or their independent subsystems across no more than three read-only investigators. Give
  each a bounded path scope and the same repository boundary, helper JSON path, and dirty lists. Require evidence, not
  proposed edits.
- If the host cannot run investigators, perform the same partitions serially without relaxing the evidence standard.
- Each investigator returns alignment evidence and a practice inventory for its scope. When the task text narrows the
  focus, collect only the matching part.

### Alignment Evidence

- Map canonical-versus-generated relationships and the exact build pipelines that produce generated artifacts. Read
  build configuration and generator inputs before assigning canonical ownership to a file.
- Identify duplicated and near-duplicated instruction, documentation, and script content. Cite both sides precisely.
- Test suspected duplication with normalized diffs or content hashes appropriate to the artifact format. Use byte
  comparison for exact copies and show the normalization used for near-duplicate claims.
- Identify drift, including stale paths, version mismatches, contradictory rules, and unpublished changes. Prove
  suspected drift against the relevant canonical source, live path, version source, or reproducible command.
- Attribute a stale reference to the referencing file and its target, not merely to a repository-wide search result.
- Identify competing workflows that accomplish the same job. Describe their inputs, outputs, side effects, documented
  trigger, and the canonical source each consumes.
- Locate references to every other listed repository and determine whether each reference remains valid. Check local
  references independently from remote or published references when both forms exist.
- Keep generated artifacts distinct from their sources when grouping results.

### Practice Inventory

- Guidance: rules, conventions, commands, project skills, hooks, permissions, and MCP servers, each marked portable or
  project-specific.
- Workflows: tasks with the commands they run and what they check or produce, CI jobs and triggers, hooks, toolchain
  pins, and lint, format, type-check, test, coverage, security, dependency-update, and release tooling.
- Dependencies: the gap-matrix entries relevant to tooling or shared concerns, with how the repository uses each
  (config, task, or import at file:line).

### Synthesis

- Fold all investigator results into one evidence-backed picture, preserving the provenance of every claim. Resolve
  disagreements by rerunning the mechanical check, not by selecting the more persuasive prose description.
- Build the cross-repository matrix in the orchestrator. For each practice, record which repositories have it, an
  equivalent, or nothing. Establish equivalence by function, not name: `oxlint` versus `eslint`, `just check` versus
  `npm run lint`, `ruff` versus `flake8` plus `black`.
- Verify every recipient gap mechanically by searching for the tool name, config files, commands, and functional
  equivalents. Record the commands and results.
- Assess fit per direction: ecosystem and runtime compatibility (`engines`, `requires-python`, `rust-version`, the `go`
  directive, peer constraints), recipient purpose, conflicting tools or documented policy, and maintenance or CI cost.
  Concept-level practices may cross ecosystems (a type-check gate, a scoped-formatting rule). Dependencies transfer only
  within an ecosystem or as an approved ecosystem-equivalent tool.
- Skip guidance the recipient already receives from a broader scope, such as global or parent-directory instruction
  files loaded in the agent's context. Duplicating it adds context without changing behavior.
- For tooling candidates, preview impact read-only when cheap: run the tool through an ephemeral runner (`bunx`, `npx`,
  `uvx`, `go run`, `cargo` in check mode) without writing repository files, confirm with `git status --short`, and
  attach counts such as violations found and autofixable.
- Check current registry versions and runtime requirements through the recipient's package manager at research time.
  Recheck when implementing.

## Consolidate and Evaluate

- Deduplicate overlapping observations without discarding the strongest file:line and mechanical evidence. Keep linked
  but non-identical observations separate when they require different fixes or decisions.
- Merge transfer candidates for the same practice from multiple donors. Pick the strongest donor implementation as the
  model and cite the others.
- Do not classify an unverified suspicion. Return it to research or record it as an open question.
- Regenerate suspected generated-copy drift through its build-only path and byte-verify the result with a diff. Preserve
  the generator inputs and command output needed to reproduce the verification.
- Resolve symlinks and compare their targets before calling linked content duplicated or divergent.
- Separate an objectively broken reference from a preference about how much duplicated context to retain.

Classify each material finding exactly once:

1. Confirmed drift to fix: a mechanically proven mismatch against the canonical source, live path, version source, or
   reproducible command.
2. Single-source candidate: true duplication that one canonical source could replace. The user decides whether to
   implement it.
3. Recommended transfer: clear benefit, verified gap, compatible ecosystem, no conflict with an existing tool or
   documented policy, and low cost.
4. Judgment call: replacing or retiring an existing tool or competing workflow, introducing a new ecosystem, runtime,
   task runner, service, or required secret, material CI cost, license concerns, policy-changing guidance such as
   commit, release, or review rules, pre-existing violations surfaced by a new check (fix, baseline, or drop), trim
   depth for duplicated material, and publish versus hold.
5. Deliberate no-change or rejected: deliberate or necessary duplication, ecosystem mismatch, project-specific practice,
   equivalent already present, low value, or conflict with the recipient's documented policy.

Mark duplication protected by design rules, such as self-contained artifacts or independently installed skills, as
deliberate when the evidence supports that constraint. Treat a self-contained installation requirement as a design
constraint even when its copies are byte-identical. Keep the no-change and rejected list with the mechanical evidence
and reason for each entry.

## Decide

- Apply decisions and delegated judgment already established by the user. Ask only for unresolved choices that
  materially change the outcome. Prepare the evidence and concrete proposal before requesting approval.
- Present confirmed drift separately from judgment calls. Normally offer it as one uncontroversial fix batch. Include
  the affected repositories and the exact mechanical evidence with each fix.
- Present recommended transfers as a compact table with donor, recipients, evidence, benefit, and cost. Offer them as
  one default-accept batch that the user can trim.
- Present every single-source candidate and judgment call as an explicit question with a recommended option and its
  tradeoff. When the choice determines the plan, keep alternatives mutually exclusive.
- State the default preservation option when no simplification is clearly justified.
- Resolve decisions affecting each change before planning it. Continue independent research and already authorized
  changes while another choice is pending. Do not treat a progress report as completion of the requested work.
- Record each user decision verbatim beside the finding it resolves. Preserve a decision's condition or exception when
  it limits an otherwise approved change.
- Carry declined items into the no-change and rejected list rather than silently omitting them.

## Plan

- Produce a decision-complete plan per repository. Name the exact files to create or edit, canonical sources, affected
  generated artifacts, the donor source each transfer adapts (file:line), and dependencies with the package-manager
  command that adds them.
- Associate each change with its finding, its verification commands, and the user decision that authorized it.
- Order edits so canonical sources change before their generated or installed counterparts. In each recipient,
  dependencies precede the configs and tasks that use them, and guidance lands last so it documents what exists.
- Avoid editing installed output by hand unless it is itself a canonical artifact under the repository's rules.
- Name every regeneration command and whether it is build-only, non-committing, or expected to alter files. Identify
  commit-hook side effects and keep them distinct from build and regeneration steps.
- Specify commit sequencing, publish sequencing, and any dependency between repositories. Identify the point at which a
  downstream repository can safely consume an upstream generated or installed artifact.
- In Claude Code, prefer plan mode.
- When `$orchestration` is available, delegate implementation through it. Otherwise, use host subagents. If those are
  also unavailable, implement directly.
- Use disjoint per-repository write scopes for delegated work in every implementation shape. Reserve aggregate
  cross-repository validation for one owner so it runs once after dependent edits settle.
- Keep the plan limited to user-approved decisions. Extend it autonomously for technical prerequisites covered by those
  decisions. Report and ask only when a new finding introduces a subjective choice, changes the repository set or
  intended outcome, or crosses an unapproved destructive, publish, or other external-write boundary.

## Implement and Finalize

For transfers:

- Add dependencies through the recipient's package manager (for example `bun add -d`, `pnpm add -D`, `uv add --dev`,
  `cargo add`, `go get -tool`) at the latest stable version compatible with its constraints, matching its range and
  pinning style. Let the package manager update the lockfile.
- Adapt donor configs by dropping options that reference donor-only paths, names, or features.
- Match the recipient's CI conventions: provider, action pinning style, runners, caching, and least-privilege
  permissions.
- When a guidance file is a symlink or generated artifact, edit its canonical source. Transfer a project skill by
  adapting it into the recipient's skill directory, or through the same install mechanism when the donor installs it
  from a shared catalog.
- Run every new or changed task once in the recipient and confirm it does the job the donor's version does. A
  transferred check passes on the recipient, or the user's decision for pre-existing failures is applied.
- Validate CI and hook changes with an available local linter or dry run, such as `actionlint`. When only a CI run can
  prove them, state this explicitly.
- Verify transferred guidance: every command, path, and tool it names exists in the recipient.

For every change:

- Reconcile each implementation-agent result against the visible working tree and its assigned write scope. Confirm
  every reported changed path belongs to the approved scope before treating an agent result as complete.
- Preserve pre-existing dirt and unrelated concurrent changes byte-for-byte.
- Run the narrowest per-repository checks that prove the intended edits. Run them from the repository whose rules and
  dependencies they exercise.
- Run cross-repository invariants once, including regeneration idempotence and source-versus-installed diffs where
  applicable.
- Attribute an aggregate failure before acting: first rule out effects of this task's changes, formatters, hooks, and
  generators, including downstream failures outside the approved files. Continue past a failure only when evidence
  establishes that it is unrelated and the task's own checks still pass.
- When `$commit` is available, use it after validation, passing only task files. Never bypass it with `git add -A`,
  `git commit -a`, stash, or reset.
- Commit per repository so histories, hooks, and publication states remain independently auditable. Where practical,
  commit each transfer separately so it stays independently revertible.
- Inspect each scoped commit before creating the next so an upstream commit remains independently reversible.
- Rerun every required publish or install flow after its source changes. Do not publish an unvalidated source or an
  installed artifact that no longer matches its source.
- If publication is held by user decision, leave the validated source commit ready and report the required release step.

## Report

- Lead with one outcome line: fixes and transfers applied per repository and any remaining blocker.
- Tabulate applied fixes and transfers: repository or recipient, finding or practice, donor source when any, changed
  files, and commit identifier.
- Report every verification command and its outcome. Distinguish passed checks from checks that were intentionally not
  run, and explain why, including CI-only verification.
- List deliberate no-change, rejected, and declined items with their reasons in one compact table.
- State residual risks, including unrun publish flows, unavailable regeneration tooling, and unresolved cross-repository
  references.
- State open questions that were deferred or discovered after the approved decision boundary.
- Keep the report auditable: tie every conclusion to its recorded finding, decision, evidence, or command outcome.
