---
argument-hint: "<repo-path> <repo-path> [more-repos...]"
disable-model-invocation: true
metadata:
  install-targets: claude-code codex
name: repo-harmonization
skill-dependencies:
  - codex-handoff
  - commit
description:
  Audit multiple interdependent repositories for alignment, then plan and apply surgical fixes for drift and
  duplication.
---

# Repo Harmonization

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Turn a list of interdependent repositories into verified alignment findings, user-approved decisions, and surgically
implemented fixes.

## Arguments

`$ARGUMENTS` is a whitespace-separated list of repository paths. Tilde expansion is allowed.

- Resolve every supplied path before investigating its contents.
- Require at least two paths.
- Require each resolved path to exist and be a Git repository.
- Stop with a clear message identifying any missing, non-Git, or insufficient repository input.
- Do not infer additional repositories from links, remotes, or installed copies; the supplied list defines the audit
  boundary.

## Contract

- Research before any edit and keep research strictly read-only; parallelize across subagents when the host supports
  them, otherwise investigate serially.
- Verify every alignment or duplication claim mechanically with a diff, content hash, regeneration round-trip, or
  symlink resolution; never rely on prose inspection alone, and attach file:line evidence to every finding.
- Classify each finding exactly once as confirmed drift to fix, true duplication that is a single-source candidate, or
  deliberate or necessary duplication to document without changing.
- Send judgment calls about deduplications to implement, competing workflows to retire, trim depth, and publish versus
  hold to the user before planning; never expand beyond the approved decisions.
- Treat the approved alignment outcome and resolved judgment calls, not the initial file manifest, as the implementation
  authorization boundary. When implementation discovers a related in-repository prerequisite needed to carry out those
  decisions, the orchestrator may extend the manifest, acquire coordination for the new scope, and delegate the smallest
  sufficient follow-on fix without asking again. Workers still stop at their assigned write scopes and return the
  evidence to the orchestrator.
- Prefer surgical fixes to restructuring; add no include pipeline, templating layer, shared reference, or other
  machinery unless it removes more complexity than it adds.
- Respect every repository's generation pipelines and hooks; edit canonical sources only, regenerate artifacts through
  build-only non-committing paths, and let designed commit hooks run at commit time.
- Keep implementation agents from committing or pushing; the orchestrator commits per repository, scopes commits to task
  files, and honors the host's coordination and shared-worktree rules.

## Intake

- Resolve the supplied repository paths to stable absolute paths and retain their original user-facing forms for the
  report.
- Confirm the paths are distinct repositories rather than multiple spellings of the same resolved directory.
- Record the checked repository root for each path so later evidence is unambiguous.
- Snapshot `git status --short` in every repository before research.
- Treat pre-existing dirt as other agents' in-flight work: record it as a preservation boundary, never an audit target.
- Record the snapshot before running any command that might regenerate, install, or otherwise modify a worktree.
- Record each repository's current branch and remote configuration only when those facts affect an install, publish, or
  cross-repository reference.
- Build a first-pass link map before making conclusions.
- Treat the link map as an inventory, not proof that two files should share one source.
- Map symlinks that point between the listed repositories, including their resolved targets.
- Distinguish an intentional external symlink from a broken one by resolving it from its containing repository.
- Map files generated from another listed repository and identify the stated source when available.
- Record the generator command or manifest entry that establishes each generated relationship.
- Map install and publish flows that move artifacts, skills, packages, or configuration between the repositories.
- Note whether each flow changes files, creates external state, or merely validates a source artifact.
- Search AI-context files for references to the other listed repositories, including `AGENTS.md`, `CLAUDE.md`, README
  guidance, skills, and local instructions.
- Capture the concrete referenced path, command, or repository identifier rather than only its surrounding prose.
- Record unknown origins as unknown rather than assuming that path similarity establishes a relationship.

## Research

- Partition the repositories or their independent subsystems across no more than three read-only investigators.
- If the host cannot run investigators, perform the same partitions serially without relaxing the evidence standard.
- Give each investigator a bounded path scope and require evidence, not proposed edits.
- Give every investigator the same repository boundary and pre-existing-dirt list.
- Map canonical-versus-generated relationships and the exact build pipelines that produce generated artifacts.
- Read build configuration and generator inputs before assigning canonical ownership to a file.
- Identify duplicated and near-duplicated instruction, documentation, and script content; cite both sides precisely.
- Test suspected duplication with normalized diffs or content hashes appropriate to the artifact format.
- Use byte comparison for exact copies and show the normalization used for near-duplicate claims.
- Identify drift, including stale paths, version mismatches, contradictory rules, and unpublished changes.
- Prove suspected drift against the relevant canonical source, live path, version source, or reproducible command.
- Attribute a stale reference to the referencing file and its target, not merely to a repository-wide search result.
- Identify competing workflows that accomplish the same job and describe their inputs, outputs, and side effects.
- Include the documented trigger and the actual canonical source each competing workflow consumes.
- Locate references to every other listed repository and determine whether each reference remains valid.
- Check local references independently from remote or published references when both forms exist.
- Keep generated artifacts distinct from their sources when grouping results.
- Fold all investigator results into one evidence-backed picture, preserving the provenance of every claim.
- Resolve disagreements by rerunning the mechanical check, not by selecting the more persuasive prose description.

## Consolidate

- Deduplicate overlapping observations without discarding the strongest file:line and mechanical evidence.
- Keep linked but non-identical observations separate when they require different fixes or decisions.
- Apply the three-way classification to every finding: confirmed drift to fix, true duplication as a single-source
  candidate, or deliberate or necessary duplication to document without changing.
- Do not classify an unverified suspicion; return it to research or record it as an open question.
- Regenerate suspected generated-copy drift through its build-only path and byte-verify the result with a diff.
- Preserve the generator's inputs and command output needed to reproduce the verification.
- Resolve symlinks and compare their targets before calling linked content duplicated or divergent.
- Mark duplication protected by design rules, such as self-contained artifacts or independently installed skills, as
  deliberate when the evidence supports that constraint.
- Treat a self-contained installation requirement as a design constraint even when its copies are byte-identical.
- Separate an objectively broken reference from a preference about how much duplicated context to retain.
- Keep a deliberate-no-change list with the mechanical evidence and design reason for each entry.

## Decide

- Present confirmed drift separately from judgment calls; normally offer confirmed drift as an uncontroversial fix.
- Include the affected repositories and the exact mechanical evidence with each proposed fix.
- Present every judgment call as an explicit user question with a recommended option and its tradeoff.
- Keep alternatives mutually exclusive where a choice determines the next plan.
- Ask before choosing which single-source candidates to implement, which competing workflows to retire, how deeply to
  trim duplicated material, or whether to publish now or hold changes.
- State the default preservation option when no simplification is clearly justified.
- Do not write the implementation plan until the user resolves every decision that changes scope or approach.
- Record each user decision verbatim beside the finding it resolves.
- Preserve a decision's condition or exception when it limits an otherwise approved change.
- Carry declined changes into the deliberate-no-change list rather than silently omitting them.

## Plan

- Produce a decision-complete plan that names exact files, canonical sources, and affected generated artifacts.
- Associate each edit with its finding and the user decision that authorized it.
- Order edits so canonical sources change before their generated or installed counterparts.
- Avoid editing installed output by hand unless it is itself a canonical artifact under the repository's rules.
- Name every regeneration command and whether it is build-only, non-committing, or expected to alter files.
- Identify commit-hook side effects and keep them distinct from build and regeneration steps.
- Specify per-repository verification, commit sequencing, publish sequencing, and any dependency between repositories.
- Identify the point at which a downstream repository can safely consume an upstream generated or installed artifact.
- In Claude Code, prefer plan mode.
- Delegate implementation through `$codex-handoff` when available; otherwise use host subagents, otherwise implement
  directly.
- Use disjoint per-repository write scopes for delegated work in every implementation shape.
- Reserve aggregate cross-repository validation for one owner so it runs once after dependent edits settle.
- Keep the plan limited to user-approved decisions. Extend it autonomously for technical prerequisites covered by those
  decisions; report and ask only when a new finding introduces a subjective consolidation choice, changes the repository
  set or intended outcome, or crosses an unapproved destructive, publish, or other external-write boundary.

## Implement and finalize

- Reconcile each implementation-agent result against the visible working tree and its assigned write scope.
- Confirm every reported changed path belongs to the approved scope before treating an agent result as complete.
- Preserve pre-existing dirt and unrelated concurrent changes byte-for-byte.
- Run the narrowest per-repository checks that prove the intended edits.
- Run those checks from the repository whose rules and dependencies they exercise.
- Run cross-repository invariants once, including regeneration idempotence and source-versus-installed diffs where
  applicable.
- Attribute an aggregate failure before acting: a failure outside the approved files is concurrent work, not evidence to
  broaden this change.
- Use `$commit` when available after validation, passing only task files; never bypass it with `git add -A`,
  `git commit -a`, stash, or reset.
- Inspect each scoped commit before creating the next so an upstream commit remains independently reversible.
- Keep commits per repository so their histories, hooks, and publication states remain independently auditable.
- Rerun every required publish or install flow after its source changes.
- Do not publish an unvalidated source or an installed artifact that no longer matches its source.
- If publication is held by user decision, leave the validated source commit ready and report the required release step.

## Report

- Report per-repository commit identifiers and the files each commit contains.
- Report every verification command and its outcome.
- Distinguish passed checks from checks that were intentionally not run and explain the latter.
- List deliberate no-change findings with the reasons they remain duplicated or unchanged.
- State residual risks, including unrun publish flows, unavailable regeneration tooling, and unresolved cross-repository
  references.
- State open questions that were deferred or discovered after the approved decision boundary.
- Keep the report auditable: link every conclusion back to its recorded finding, decision, or command outcome.
