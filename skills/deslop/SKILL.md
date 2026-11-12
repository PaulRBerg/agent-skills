---
argument-hint: "[path ...] [--dry-run]"
disable-model-invocation: true
name: deslop
description:
  Remove unnecessary code, files, and prose, consolidate related logic and guidance, and retire obsolete behavior with
  evidence. Use when asked to deslop or clean up AI-generated material while preserving required behavior and contracts.
---

# Deslop

Remove material that creates work without serving a supported purpose. Consolidate concepts that belong together. Leave
the selected system easier to understand, operate, and maintain.

**Slop** is unnecessary, redundant, speculative, or stale material with a demonstrated cost: misleading guidance,
competing authority, avoidable indirection, repeated investigation, or maintenance without useful behavior. AI origin,
age, size, file count, and personal taste alone do not establish that cost. Preserve sound design and useful
documentation regardless of who created them.

## Scope and Authority

User instructions take precedence over this skill's defaults. Apply existing authorization without asking again.

- Use explicit paths or assigned repositories when supplied. Otherwise, use the current repository.
- Read applicable instructions and identify protected files, generated sources, ownership, and required coordination.
- Keep unassigned dependencies outside the write scope. Inspect their contracts only as needed.
- Treat cleanup invocation as authorization for local reversible edits, including evidenced obsolete source and
  documentation removal. Honor existing destructive-action and external-write boundaries.
- For an assessment-only request, Plan Mode, or `--dry-run`, inspect and report without editing files.
- For documentation or design cleanup, complete the authorized document changes. Mark runtime changes as proposed.
- If an action lacks authority, prepare its concrete candidate and evidence before asking for that action. Continue
  independent authorized work while it waits.

Before edits, state the selected boundary, material assumptions, and verification target. Resolve discoverable facts
yourself. Do not add a default approval checkpoint after planning.

## 1. Understand the Selected System

Inspect authors' sources with bounded searches. Exclude dependency, build, cache, vendor, and generated directories from
broad discovery. Follow relevant consumers and execution paths beyond the first search hit.

Build a compact system map from entry points, code, tests, documentation, agent guidance, manifests, scripts, and
configuration:

| Concern   | Establish                                                                                     |
| --------- | --------------------------------------------------------------------------------------------- |
| Purpose   | User outcomes, domain vocabulary, constraints, and required behavior.                         |
| Ownership | Responsible modules, interfaces, dependencies, and owners of invariants.                      |
| Behavior  | Entry points, data flow, state, persistence, side effects, and failure paths.                 |
| Operation | Available actions, current-state sources, verification, interruption, and recovery.           |
| Knowledge | Authoritative guidance, generated copies, audiences, load scope, and update responsibilities. |

Distinguish observed behavior, documented intent, and proposed changes. Attach paths, symbols, or command evidence to
material claims. For connected repositories, identify producer and consumer contracts, versions, and ordering.

For a whole-repository request, account for selected areas as reviewed or excluded with a reason. Use chat by default.
Do not create a custom ledger, persistent report, or helper artifact merely to document routine progress.

Judge the system through these outcomes:

- **Intuitive:** Discover its purpose, vocabulary, constraints, and authority without guessing.
- **Ergonomic:** Find current state, take a bounded action, observe success, and recover from failure.
- **Accretive:** Leave verified durable knowledge or useful capabilities for later work. Correct or retire obsolete
  knowledge.

Preserve product goals and human usability. Optimize total cost across repeated tasks, including future maintenance. Do
not optimize token, line, or file counts at the expense of correctness.

Walk a routine change, a failure and recovery path, and continuation by another worker when applicable:

1. Locate the goal, state, constraints, and authoritative source.
2. Find the smallest responsible interface.
3. Identify the action, its effects, and its authority.
4. Check the result against required behavior.
5. Recover from failure or partial completion.
6. Identify the useful durable fact that later work would otherwise rediscover.

Record concrete friction against these tasks. Examples include conflicting authority, hidden dependencies, opaque state,
ambiguous names, and repeated manual investigation. Do not infer a defect from unfamiliar structure alone.

## 2. Select Evidence-Backed Cleanup

For each candidate, establish required behavior before judging its implementation. Ask what can disappear entirely. Then
simplify what remains. Prefer deletion over simplification when deletion preserves the required contracts.

Identify the demonstrated cost, responsible owner, affected consumers, preservation requirements, and proving check. For
a material structural decision, compare the current design with the simplest viable alternative.

### Logic and Structure

- Remove speculative layers, unused configuration, and one-use abstractions only when they provide no supported value.
- Apply the deletion test to pass-through wrappers. If removal spreads complexity across callers, preserve or improve
  the useful abstraction. If complexity disappears, remove the wrapper.
- Consolidate co-changing cohesive concepts and shared invariants into their existing owner.
- Compare parallel validators, parsers, normalizers, and defaults against their producer and consumer contracts. Similar
  names or comments do not prove equivalent behavior. Check malformed but plausible inputs and boundary cases.
  Centralize a real shared invariant in its existing owner. Preserve useful platform adapters and intentional semantic
  differences.
- Inventory closed membership lists repeated across manifests, dispatch logic, runtime code, and documentation. Use the
  existing owner for shared membership when practical. Preserve consumer-specific capabilities, ports, and validation
  metadata, along with intentionally independent contracts. Do not add a registry or generator framework to replace
  trivial duplication.
- Prefer substantial behavior behind a small interface that callers and tests can use directly.
- Preserve separation for different meanings, owners, users, lifecycles, or real behavior variations.
- Treat large files as discovery candidates only. Split for unrelated responsibilities, coupling, ownership,
  testability, or change risk. Merge fragmented related logic when that gives the better result.
- Avoid replacing scattered fragments with a giant unrelated module or document.

Preserve inputs, outputs, side effects, errors, ordering, security, telemetry, performance-sensitive behavior, and
operational guards. Do not remove boundary validation because happy-path tests pass. Retain compatibility and migration
logic while supported consumers require it. Remove it only with evidence that those requirements have ended.

### Names

Use one canonical domain term per concept and different terms for distinct concepts. Align filenames, exports, and
responsibilities. Rename only when the result removes ambiguity. Avoid synonym churn.

Update all relevant references in the same change. Preserve public names, persisted strings, and external contracts
unless an authorized migration covers their consumers.

### Documents, Comments, and Agent Guidance

- Retain meaningful rationale, constraints, exceptions, recovery steps, exact commands, formats, licenses, and tool
  directives. Treat behavior-bearing comments as code.
- Treat schema descriptions, CLI help, fixture descriptions, and generated documentation as claims about behavior and
  ownership. Trace generated claims back to their producer. Update the authoritative source and regenerate derived
  artifacts.
- Remove generic tutorials, obvious narration, stale plans, reports, placeholders, and redundant instructions only when
  evidence establishes that they are unnecessary.
- Consolidate guidance with the same audience, lifecycle, and authority into its authoritative owner.
- Deduplicate the effective load chain while preserving independently installed or independently loadable artifacts.
  Keep those artifacts self-contained.
- Keep entry points short and detailed conditional guidance near its owner. Merge into the authoritative document
  instead of creating another summary.
- Give durable knowledge an appropriate home and an update trigger. Keep transient task state outside always-loaded
  instructions. Avoid copying facts available from a cheap authoritative query.
- Honor archive policies, historical rationale, and protected user-owned content.
- Retire a document only after preserving valuable content and repairing incoming links and callers.

For documentation-only work, update existing design documents and plans in place. Keep dependency order and observable
acceptance checks in proposed implementation steps. Do not present proposed commands or runtime behavior as available.

Treat runtime prompt wording, ordering, and output schemas as behavior. Preserve repetition justified by evaluations.
Validate edits through the owning evaluation workflow, comparing baseline and candidate on matched inputs and model
settings. Static string checks prove wiring or required text, not answer quality. Do not run paid inference or
consequential evaluation without existing authorization. If evaluation is unavailable, leave the runtime prompt
unchanged and report the exact gap while continuing independent cleanup.

### Tests

Identify the behavior each affected test protects. Remove tautologies that cannot detect a plausible implementation
defect. Remove obsolete implementation-only assertions when no supported requirement depends on them.

Inspect which checks each test runner invokes. If a runner repeats enforcement that its own tests already provide,
consolidate the checks by contract and coverage. For source-text assertions, name the invariant and a plausible failure.
Replace incidental source or command spelling constraints with behavior checks when they prove the same contract and fit
the existing authority. Preserve exact-text contracts, independent layer checks, and intentional validation tiers. Do
not bypass required checks or remove guards to make a suite pass.

Check that successful fixtures can come from the real producer and that mocks do not bypass or copy the invariant under
test. Keep malformed fixtures that verify rejection. Repair weak fixture or contract assertions instead of deleting
useful integration tests.

Before deleting a test, prove that its behavior is obsolete or identify the retained test that covers it. Merge
equivalent cases into the owning suite only while preserving distinct behavioral assertions, input cases, and
diagnosable failures. Use coverage or a focused defect experiment when overlap remains unclear.

Never remove a failing or flaky test merely to make checks pass. Add proportionate regression coverage for meaningful
changed behavior. Do not add tautological content tests for reversible prose edits.

## 3. Prove Removals and Execute

Before deleting source, behavior, or artifacts, inspect their actual consumers as applicable:

- Imports, exports, runtime registration, dynamic discovery, and reflection.
- Build configuration, manifests, task runners, packaging, and test setup.
- Schemas, generators, generated consumers, fixtures, and documentation links.
- Public interfaces, persisted formats, external callers, supported compatibility, and migrations.
- Operational recovery paths and fallback behavior.
- Supported runtime, host, platform, and test roles. Keep differences that serve a real consumer, such as template
  platforms, host-only test support, vocabulary differences between client and server layers, and local adapters.

Classify an artifact by its role before removing it: authoritative input, raw evidence, generated derivative, published
snapshot, fixture, or disposable output. Similar contents or a completed migration do not make provenance, audit, or
recovery records disposable. Follow the owning lifecycle and preserve original evidence. Generated copies can be
intentional when separate consumers require them.

Missing text references alone do not prove deadness. If usage remains unknown, retain the candidate until resolved.
Report the exact uncertainty and the evidence needed to resolve it.

Execute small coherent changes in dependency order. Re-read current files before editing. Preserve unrelated work.
Update owned producers, consumers, links, metadata, examples, and tests together. Change generated artifacts through
their authoritative source and existing generator.

Reuse existing task tooling. Do not add a speculative framework, documentation system, or cleanup process. For repeated
evidenced invariant failures, prefer a proportionate mechanical check over more reminders. Keep the required contract
explicit in guidance even when tooling enforces it.

Keep work sequential unless the user or applicable instructions authorize delegation. If authorized, assign independent
bounded scopes and one owner for aggregate validation. Do not require full companion audits.

Give brief factual progress updates. Continue through authorized edits and checks. Do not stop at an inventory,
proposal, or milestone. Once the fixed scope is complete, do not start another audit without new evidence.

## 4. Verify and Report

Run existing required checks and the narrowest behavior or interface checks that prove the selected changes. Verify
paths, commands, references, and intentional remnants against their owners. Inspect command definitions before executing
them. Do not run consequential operations merely to verify prose.

Repeat representative task loops against the revised system. Compare how a worker finds state, authority, actions,
success signals, recovery, and durable knowledge. Check that local simplification did not move complexity to consumers.

Finish when selected areas are accounted for, authorized cleanup is complete, and required checks pass. If required work
remains blocked, report the exact obstacle and needed input. Tie claims to observed results. Disclose failed, skipped,
and unrun checks. Do not invent savings or impose cleanup quotas.

Lead with the outcome. Summarize changes and reasons, important preserved boundaries, exact checks and outcomes, and
unresolved items. Distinguish document changes from proposed runtime work. Report a verified no-op when no cleanup is
warranted.
