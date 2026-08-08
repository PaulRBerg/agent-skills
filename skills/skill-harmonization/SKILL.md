---
argument-hint: "[task]"
compatibility: Requires Git, ripgrep (rg), uv, and an installed skill-map skill with --portfolio-root support.
disable-model-invocation: true
name: skill-harmonization
user-invocable: true
description: Harmonize a repository's skill portfolio across catalog and user-installed skills.
---

# Skill Harmonization

If these instructions are already present from a slash or dollar invocation, follow them directly; do not invoke this
skill again through a skill tool.

Treat the current Git repository and relevant user-installed skills as one repository-centered portfolio. Inventory it
mechanically, distinguish defects from judgment calls and deliberate relationships, then report, plan, or implement
according to the invoking task's authority.

Success means every repository skill was considered against relevant repository and user candidates, every conclusion
has path and client evidence, observations remain separate from inference, and the selected mode ends with appropriate
findings, recommendations, validation, and deliberate no-change conclusions.

## Input and Prerequisites

`[task]` is optional free-form guidance. Use it to narrow the question, desired outcome, or authorized implementation
scope. If it is absent, infer intent from the surrounding request; an invocation without write intent is read-only.

1. Require `git` on `PATH`, then resolve the current repository with `git rev-parse --show-toplevel`. Do not read Git
   history. If Git is unavailable, stop with:

   ```text
   skill-harmonization: Git is required and was not found on PATH.
   ```

   If repository resolution fails, stop with:

   ```text
   skill-harmonization: run from inside a Git repository.
   ```

2. Resolve the installed `skill-map` directory from the host's disclosed skill inventory or the standard user skill
   roots. Do not search the rest of the home directory. Verify its helper advertises `--portfolio-root`. If the skill or
   extended interface is unavailable, stop with:

   ```text
   skill-harmonization: installed skill-map with --portfolio-root support is required; install or publish the current skill-map skill, then retry.
   ```

3. Require `rg` and `uv` before running helpers. Name the missing command and stop if either is unavailable.

## Authority and Mode

Derive behavior from the user's task and the host's authority rules; this skill does not introduce its own phase gate.

- For inspection, audit, review, diagnosis, or an invocation with no write intent, investigate and return an assessment.
  Do not edit.
- For a planning request, return a decision-complete plan grounded in the inventory and static evidence. Do not
  implement it.
- For an implementation request, apply the authorized changes, validate them, and report the resulting portfolio.
- Do not force a report-first step, saved report, plan approval, or implementation phase. Ask only when an unresolved
  choice changes scope, safety, or intended outcome.
- Preserve normal confirmation boundaries for renames, removals, destructive actions, and global skills whose canonical
  source ownership remains unknown.

## Build the Portfolio

Run the helper path resolved from the installed `skill-map`:

```sh
uv run <resolved-skill-map-helper> --portfolio-root <repo-root> --format json
```

Require valid JSON and retain its repository root, present and missing user roots, lexical exposures, resolved targets,
locations, kinds, clients, symlink identity, hashes, references, and duplicate records. Do not substitute a duplicated
inventory helper or broaden the roots.

When `skill-doctor` is installed, optionally run its helper in JSON mode against the repository and present user skill
roots. Consume its metadata and doc-link findings as additional evidence; its absence is not a blocker and its warnings
are not conclusions by themselves.

## Evidence Boundary

Use only static repository evidence needed to understand skills and their workflows:

- discovered `SKILL.md` files and their skill-local scripts, references, agents metadata, examples, and assets;
- applicable `AGENTS.md` or `CLAUDE.md`, repository-facing documentation, and repo-private agent runbooks;
- automation directly referenced by those artifacts, including install, generation, validation, sync, and publication
  helpers.

Do not inspect transcripts, Git history, TODO files, caches, agent state, or unrelated source code. Never use absence of
references as evidence that a skill is unused. Treat inventory edges as leads; open only allowed evidence that bears on
a candidate relationship.

Every finding must identify the affected lexical and resolved paths when they differ, applicable clients, and the
mechanical or textual evidence. State observed facts separately from inferred design intent or recommended action.

## Analyze Repository-Centered Relationships

Account for every repository skill. Compare it with other repository skills and user-installed candidates that could
affect repository behavior. Exclude global-to-global issues unless a repository skill, invocation, dependency,
publication flow, or runtime exposure is affected.

Cluster plausible candidates before deeper comparison using:

- declared and directory invocation names;
- direct skill references and referenced automation;
- descriptions and intended outcomes;
- authority and side-effect boundaries;
- runtime or tool dependencies;
- `SKILL.md` and complete-tree hashes.

Do not perform an exhaustive all-pairs body comparison. Equal hashes establish content identity, not design intent;
different hashes establish divergence, not a defect. Resolve symlinks and client exposure before interpreting either.

### Externally Managed User Installations

A user-installed skill can intentionally have an external canonical source rather than a matching directory in the
current repository. When documentation or installation automation for the relevant user root identifies an upstream
source and the inventory resolves the skill for the affected client, treat the reference as a valid externally managed
dependency.

Classify that relationship as deliberate no-change. Do not call it load-bearing drift, recommend vendoring it into the
current catalog, or infer removal risk from this catalog's publication workflow. Report a defect only when the target is
absent for the affected client, broken, or contradicts its declared external source or installation contract.

## Classify Every Material Finding

Classify each material relationship exactly once:

### Verified defects

Use only for mechanically or textually proven problems, such as broken referenced artifacts, source/install tree drift
against an established publication owner, contradictory shared contracts, or same-name runtime collisions at distinct
real locations. Identify the canonical contract or source that proves the defect.

### Judgment candidates

Use for overlap, merge or split, rename, relocation, enhancement, addition, or removal proposals. Explain the observed
relationship, the inference behind the recommendation, affected paths and clients, and the smallest viable action.

- Recommend an addition only when static evidence establishes a conditional workflow with no suitable existing owner.
- Recommend removal only when a skill is broken, obsolete by an evidenced current contract, or demonstrably subsumed by
  another skill. Missing references never satisfies this bar.

### Deliberate no-change relationships

Record relationships that should remain as-is, including same-target aliases, aligned source/publication mirrors,
platform adaptations, externally managed global installations, and duplication required to keep independently installed
skills self-contained. Cite both the mechanical relationship and the design constraint.

Do not convert an uncertain observation into a defect. Return to the bounded evidence or retain it as a judgment
candidate with explicit uncertainty.

## Resolve Ownership Before Writes

Before changing any skill, identify its canonical source from repository instructions, symlink targets, generation or
publication automation, and source/install hashes. Edit the canonical catalog or generator input and run the owning
workflow. Never hand-edit a published global copy when a source catalog or symlink target owns it.

If a global copy has no provable source owner, treat ownership as unknown. Read-only and planning work may report it;
implementation must stop for confirmation before adopting, replacing, renaming, or removing it.

An explicit external source declaration in the relevant user-root documentation or installation automation proves
external ownership. The current repository's lack of a matching catalog source does not make that ownership unknown.

For authorized implementation, preserve unrelated work, follow the host's repository coordination and commit rules, run
the narrowest skill-specific checks, and re-run the portfolio inventory when needed to prove the intended relationship.
Do not expand implementation to unrelated improvements discovered during the audit.

## Completion and Output

Choose the smallest presentation that fits the selected mode; do not force a fixed report template. Every response must
still preserve, as applicable:

- findings with observation and inference separated;
- exact evidence plus affected paths, scopes, and clients;
- recommendations or implemented decisions;
- validation commands and outcomes;
- deliberate no-change conclusions and their reasons;
- unresolved ownership, evidence gaps, or confirmation boundaries.

Read-only work completes when every repository skill is accounted for and the assessment is evidence-backed. Planning
completes when all authorized findings map to exact canonical sources, edits, and validation, with unresolved decisions
made explicit. Implementation completes only when authorized changes and their validation succeed and the final report
faithfully states any skipped or failed checks.
