---
argument-hint: "[repo-path ...] [--dry-run]"
disable-model-invocation: true
name: agents-system
description:
  Redesign the assigned repository or repositories for agent understanding, control, and cumulative learning. Use when
  asked to make a system agent-intuitive, agent-ergonomic, or agent-accretive, then revise its design documents and
  plans.
---

# Agents System

Treat the assigned repositories as one connected system. Design for an agent that must understand the domain, choose the
correct action, observe its effects, recover from failure, and leave useful knowledge for the next task.

In this skill, **the system** means the repository or repositories the agent is tasked to work on. **Agent** means the
coding agent operating those repositories. The project does not need to contain AI software.

Use these outcomes to judge improvements:

- **Agent-intuitive:** An agent can discover the system's purpose, vocabulary, structure, constraints, and authoritative
  evidence without guessing.
- **Agent-ergonomic:** An agent can inspect state, make a bounded change, verify the result, and recover with few steps
  and clear feedback.
- **Agent-accretive:** Completed work leaves verified knowledge and reusable capabilities that reduce the cost of later
  work. Obsolete knowledge is corrected or retired.

Optimize for correct results and low total resource cost across repeated tasks. Include context, tool calls, execution
time, and future maintenance. Preserve the system's product goals, human usability, and required contracts.

## Scope and Authority

User instructions take precedence over this skill's defaults. Apply established authorization without asking again.

- Use explicit repository paths when supplied. Otherwise, use the repositories assigned in the conversation, falling
  back to the current repository.
- Resolve paths and read each repository's applicable instructions. Treat unassigned dependencies as external systems.
  Record their contracts without adding their repositories to the write scope.
- By default, revise design documents, architecture decisions, implementation plans, and the context links needed to
  reach them. Record proposed code, tooling, schema, and infrastructure changes in those plans.
- Implement runtime changes only when the user's request or standing instructions also authorize them. Keep the
  documentation work complete and distinguish proposed behavior from implemented behavior.
- For an assessment-only request, Plan Mode, or `--dry-run`, inspect and propose changes without editing repository
  files. Honor protected documents, generated-file ownership, coordination rules, and approval boundaries.

State the repository boundary, material assumptions, intended artifacts, and validation target before edits. Resolve
discoverable facts yourself. Ask only when a missing user decision changes the outcome or prevents safe progress.

## 1. Establish the System Model

Inspect relevant sources before proposing a design. Start with repository entry points, design documents, active plans,
manifests, task runners, representative code, and tests. Exclude dependencies, generated output, and caches from broad
searches. Follow authoritative links and actual execution paths instead of reading every file indiscriminately.

Build one compact model that connects:

| Concern   | Evidence to capture                                                                                    |
| --------- | ------------------------------------------------------------------------------------------------------ |
| Purpose   | User outcomes, domain concepts, actors, and constraints that define correctness.                       |
| Structure | Repository and module responsibilities, interfaces, dependency direction, and ownership of invariants. |
| Behavior  | Entry points, data and control flows, state transitions, persistence, and failure paths.               |
| Operation | Available actions, prerequisites, side effects, status sources, validation, and recovery.              |
| Knowledge | Authoritative documents, decisions, plans, generated copies, and how agents reach and update them.     |

For multiple repositories, trace producer and consumer relationships across repository boundaries. Include shared
contracts, version assumptions, release ordering, and the checks that prove compatibility when relevant.

Connect domain intent to workflows, interfaces, implementation, and verification. Explain what each abstraction hides,
what it promises, and how it composes with adjacent abstractions. Represent real cycles and shared state explicitly. Do
not force the system into a hierarchy that its behavior does not support.

Distinguish observed behavior, documented intent, and proposed behavior. Attach paths, symbols, or command evidence to
material claims. Mark unresolved contradictions and unknowns explicitly.

## 2. Walk Through Agent Work

Choose representative tasks from the actual system. Cover a routine change, a failure and recovery path, and
continuation by another agent when applicable. Trace each task through this loop:

1. Find the goal, current state, applicable constraints, and authoritative source.
2. Locate the smallest responsible interface and determine the change's effects on other parts of the system.
3. Select an action whose inputs, side effects, and authorization are clear.
4. Observe the result and run the check that establishes success.
5. Recover from failure or partial completion.
6. Preserve the decision or reusable procedure that the next agent would otherwise rediscover.

Record concrete friction: a hidden dependency, conflicting source, ambiguous name, opaque state, manual coordination,
missing check, or repeated investigation. Identify the affected task and evidence for each finding. Do not infer a
problem from file count, unfamiliar technology, or personal style alone.

Use the walkthroughs to evaluate the whole system. A locally convenient interface can still increase downstream
coupling, hide errors, or make recovery harder.

## 3. Choose a Coherent Target Design

Resolve findings into one design before editing individual documents. Favor the smallest set of changes that improves
the complete task loop. For a material structural choice, compare the current approach with a simpler alternative.
Record the selected approach, its evidence, and its tradeoff.

Apply these design tests where the evidence makes them relevant:

- **Legibility:** Use stable domain names and a short entry point with links to deeper context. Make authority,
  ownership, and dependency direction explicit.
- **Modularity:** Put each invariant with its responsible implementation. Prefer a small interface that hides useful
  complexity. Add an abstraction only when it reduces what callers must know or change.
- **Control:** Make current state cheap to query. Give operations explicit inputs, outputs, side effects, completion
  signals, and failure behavior. Specify preview, retry, interruption, and recovery semantics where needed.
- **Feedback:** Put checks near the contracts they enforce. Make errors identify the failed condition and the next
  useful action. Prefer mechanical enforcement for invariants that prose repeatedly fails to protect.
- **Accumulated knowledge:** Give durable facts, decisions, procedures, and temporary task state appropriate homes.
  Attach evidence and an update trigger to knowledge that can become stale. Keep the initial context small and load
  detailed guidance when needed.
- **Economy:** Prefer existing commands and artifacts. Remove redundant steps or competing sources when authorized.
  Account for the upkeep of every proposed document, wrapper, check, or abstraction.

For each selected change, identify the friction it removes, the affected interfaces, the expected benefit, and the
verification method. Qualify estimates. Do not invent numerical savings or optimize token count at the expense of
correctness. Leave sound design intact.

## 4. Revise Documents and Plans

Apply the design to the repository artifacts. An assessment or recommendation list alone does not complete an authorized
editing request.

1. Update the existing authoritative design documents in place. If no suitable document exists, create the smallest
   useful one in the repository's established documentation location.
2. Connect the overview to domain concepts, module contracts, operational workflows, and verification. Use a diagram
   only when it clarifies relationships. Keep detailed guidance near its owner.
3. Reconcile every affected design document and active plan within scope. Update terminology, links, diagrams,
   dependencies, and acceptance criteria so they describe the same target system.
4. Separate current behavior from the target design. Mark unimplemented commands, interfaces, and guarantees as
   proposed. Preserve existing decisions and their rationale unless evidence warrants an explicit replacement.
5. Turn implementation gaps into ordered plan steps. For each step, name the owning repository and paths or interfaces,
   prerequisites, required behavior, and acceptance check. Include migration, compatibility, and recovery details when
   existing consumers or persisted state are affected.
6. Give recurring knowledge a canonical home and a maintenance trigger. Link to it from the relevant entry point.
   Preserve deliberate duplication required by independently usable artifacts.

Do not create a parallel documentation system, universal framework, speculative automation, or process that costs more
than the demonstrated problem. Do not place transient findings into always-loaded instructions. Avoid copying facts that
agents can obtain more reliably from a cheap authoritative query.

Keep the work sequential unless the user or applicable instructions authorize delegation. When delegation is authorized,
partition evidence gathering by subsystem and integrate the results into one system model before edits.

Give brief progress updates with findings and the next concrete action. Continue independent work when one decision is
blocked. Do not stop after an inventory, a proposal, or a milestone while authorized edits remain.

## 5. Verify the Result

Repeat the representative walkthroughs using the revised entry points and documents. Check that an agent can locate the
current state, responsible interface, valid next action, success signal, and recovery procedure without hidden context.

- Verify referenced files, symbols, links, and commands against their owners. Inspect command definitions before running
  them. Use safe checks to validate current behavior, and do not execute consequential operations merely to test prose.
- Verify that cross-repository contracts agree on both sides. If a repository is unavailable, state the specific
  contract that remains unverified and its effect on the plan.
- Check that plans respect dependency order and have observable acceptance criteria. Confirm that proposed capabilities
  are not presented as available today.
- Run the repository's formatting and documentation checks on changed files. Run additional tests only when changed
  behavior or repository requirements justify them. Do not invent new validation machinery for a prose-only change.
- Review the final design for added context cost, redundant sources, hidden coupling, and unnecessary abstractions.
  Finish when the selected problems are addressed and the required checks pass. Reopen analysis only for new evidence or
  an unresolved concern.

## Report

Lead with the outcome. Link the changed design documents and plans. Summarize how the connected system becomes easier to
understand, operate, and improve through future work. Include concise decision rationale, exact checks and outcomes, and
material tradeoffs or unresolved decisions.

Distinguish completed documentation changes from planned runtime work. If no change is warranted, report the verified
no-op and its evidence. If required work is blocked, name the exact obstacle and needed input. Do not claim runtime
improvements or measured savings from documentation changes alone.
