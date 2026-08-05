---
argument-hint: "[task]"
compatibility:
  Requires Plan mode in Claude Code with Git, /bin/bash, Python 3, and an authenticated Codex CLI with dangerous bypass
  support, or Plan mode in Codex CLI with native subagents enabled.
disable-model-invocation: true
metadata:
  install-targets: claude-code codex
name: codex-handoff
user-invocable: true
description:
  Orchestrate read-only planning research and one to eight Codex agents to implement approved plans from Claude Code or
  Codex CLI.
---

# Codex Handoff

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Follow the shared contract below, select exactly one host adapter, and use that adapter for every host-specific action.

## Host Selection

Inspect the callable orchestration tools, not environment variables, process ancestry, or a user-supplied host name:

- When `spawn_agent`, `wait_agent`, `send_message`, and `followup_task` are available, read
  `references/codex-cli-host.md` completely and use only that adapter.
- Otherwise, when Claude Code's Agent and Bash tools are available, read `references/claude-code-host.md` completely and
  use only that adapter.
- If neither surface is available, stop with a compatibility error. Native Codex multi-agent support is mandatory on the
  Codex host; never fall back to a nested Codex CLI process.

Never load both adapters or combine their launch, progress, retry, permission, or result-transport mechanics. The
selected adapter may specialize host mechanics and manifest configuration, but it cannot weaken this shared contract.

## Contract

- Run only after the user explicitly invokes this skill in Plan mode. If Plan mode is not active, ask the user to switch
  and stop.
- The parent agent owns decisions, the final implementation plan, and orchestration. For complex tasks, delegate
  investigation to read-only Codex research agents before writing the plan.
- Research agents gather evidence and report findings. They never edit files, make design decisions, or return plans of
  their own.
- Each implementation agent implements its assigned part of the approved plan. It may inspect, edit, and validate, but
  must not redesign the solution or return another plan.
- Use the smallest effective implementation team. One implementation agent remains valid; use additional agents only
  when decomposition materially improves latency, correctness, or verification. Never exceed eight implementation agents
  across the handoff.
- Size every implementation brief before finalizing the team: estimate its wall-clock agent time, and split any brief
  likely to exceed roughly 25-30 minutes into parallel disjoint scopes or dependency waves, adding an integration agent
  when needed, instead of one monolithic agent. Steering, follow-up, and interrupt churn on an oversized agent costs
  more than decomposition.
- Use at most three research agents with stable IDs `R1` through `R3`. Count them separately from the eight
  implementation agents.
- Keep the parent agent's implementation work to orchestration, integrity checks, failure handling, and the conditional
  polish passes.
- Treat the approved outcome, not the initial agent manifest or its write scopes, as the authorization boundary. When
  implementation reveals a related in-repository fix or evidence change required to achieve that outcome, the parent is
  fully authorized to extend the handoff and launch follow-on implementation agents for the newly discovered scope
  without asking the user again. The worker that discovered the need must still stop at its assigned scope and return
  evidence; the parent owns the scope expansion, repository coordination, and delegation.

Use `$ARGUMENTS` as the task when present; otherwise use the active user request.

## Research Phase

Trigger research when scope is uncertain, the task crosses multiple or unfamiliar subsystems, or the plan depends on
evidence that would be materially slower for the parent to gather serially. Zero research agents remains the default.
The parent alone decides from the task and repository evidence whether research runs; never ask the user to opt in or
name agents. Either launch the research wave immediately or proceed straight to planning.

When triggered, assign up to three agents stable IDs `R1` through `R3` and launch them immediately during Plan mode
through the selected adapter's read-only mechanism. Give each agent a self-contained prompt containing:

- the open questions and exact investigation scope;
- relevant repository constraints and known concurrent-work boundaries;
- a strict read-only authority boundary;
- the stopping rule that it must return evidence rather than a plan or design; and
- exact result fields: `status`, `findings`, `open_questions`, `evidence`, and `blockers`.

When every required research agent settles, fold its findings and evidence into the implementation plan. Surface open
questions or blockers through the host's user-question mechanism only when they change scope or approach. Do not
reconcile the working tree because research agents change nothing; any reported edit is a contract violation.

## Plan Phase

Produce a decision-complete plan with this section and the selected adapter's exact manifest table:

```markdown
## Codex Handoff

- Research: `<none | R1..Rn — key findings used>`
- Strategy: `<sequential|parallel|hybrid>`
- Agents: `<1-8>` — `<why this is the smallest effective count>`
- Validation owner: `<agent-id|parent>` — `<aggregate checks it runs once>`

<host-adapter manifest table>

- Code polish: `<required|not required>` — `<reason>`
- Agent-context polish: `<required|not required>` — `<reason>`
```

Choose the execution shape from repository evidence and the approved work:

- Use sequential agents when one agent depends on another, write scopes overlap, or a later agent owns integration or
  aggregate validation.
- Use parallel agents only for independent work with explicitly disjoint write scopes. Agents may inspect shared
  context, but must not write outside their assigned scope.
- Use hybrid execution for dependency-ordered waves: run independent agents within a wave in parallel, reconcile the
  entire wave, then start its dependents.

A wave finishes with its slowest agent. Keep the highest-tier agent's scope minimal and move deferrable validation to
the validation owner. If parallel work does not collectively prove the overall plan, reserve a later sequential agent
for integration and aggregate validation. Use stable agent IDs and explicit dependencies across the whole handoff.

Assign aggregate validation to exactly one owner. Package-wide or repository-wide checks run once: by the integration
agent when one exists, otherwise by the parent during post-wave reconciliation. Every other agent runs only the
narrowest checks that prove its own edits, such as file-scoped formatting, lint, or typecheck plus targeted tests.
Duplicate aggregate runs across a wave are wasted wall-clock time, not extra assurance.

Require `$code-polish` for nonlocal invariants, concurrency or state machines, migrations or parsing, auth or security,
retry or error semantics, and public API or data-contract changes. File count alone is not a trigger.

Require `$agents-brain polish` when approved work changes a target supported by its polish workflow: README.md,
AGENTS.md or CLAUDE.md, a durable context doc, or an existing project-installed skill under `.agents/skills`. Source
catalog skills under `skills/` remain outside that workflow. Mark both passes required when both trigger rules apply;
mark neither when neither applies.

Do not launch implementation agents until the user approves the plan and the host leaves Plan mode. Read-only research
is the only pre-approval exception.

## Implementation Prompt Contract

Build a self-contained, outcome-first prompt for every implementation agent. Include:

1. The approved overall outcome plus the agent's implementation brief, dependencies, and completion evidence.
2. Its exact write scope, relevant repository constraints, known dirty-work boundaries, and prerequisite agent results.
3. Its validation assignment: scoped checks it must run and, unless it owns validation, aggregate checks it must not run
   because the validation owner runs them once.
4. A soft time budget matching its manifest sizing, with the instruction to return `blocked` with partial evidence
   rather than grinding past it.
5. This authority boundary: inspect, edit only within the assigned scope, and validate locally; do not commit, push,
   deploy, make external writes, or broaden scope, even when repository or host instructions favor committing finished
   work promptly. Committing stays with the parent after reconciliation.
6. The selected adapter's delegation and coordination context, including why the parent session and disjoint siblings
   are not conflicting work and what unrelated exact-scope claim would justify returning `blocked`.
7. This stopping rule: implement the approved plan exactly; if it is infeasible or requires redesign, return `blocked`
   with evidence instead of proposing a replacement plan.
8. A requirement to return every result field: `status` (`completed` or `blocked`), `summary`, `changed_files` listing
   only files actually touched, `verification` listing every command and outcome, `residual_risks`, and `blockers`.

Add the selected adapter's command, permission, transport, and host-tool constraints without restating this contract.

## Execution and Reconciliation

Launch agents through the selected adapter in the approved strategy and dependency waves. Do not add agents or change
models, efforts, scopes, or validation ownership merely because a worker is slow or quiet. Do revise the manifest and
launch a narrowly scoped follow-on agent when completed work discovers an unplanned prerequisite covered by the approved
outcome. Preserve stable IDs, dependency order, the eight-agent handoff limit, and one aggregate-validation owner;
include follow-on agents in the final counts and report.

For each completed agent:

- require every shared result field and treat `changed_files` as its authoritative post-pass scope;
- confirm reported files exist or were intentionally deleted, stay within scope, and carry verification evidence
  matching the assignment; and
- pass relevant completed results to dependent agents.

After every implementation wave, reconcile all results with the current manifest and visible working tree without
folding in unrelated concurrent changes. When the parent owns validation, run the assigned aggregate checks once during
this reconciliation. Attribute aggregate-check failures before blocking: a failure confined to files outside every
agent's scope is unrelated concurrent work, so confirm the handoff's files still pass and continue. Unexpected
out-of-scope edits, same-wave overlap, or a failure attributable to the handoff are blockers; do not start dependents or
polish, and do not silently take over implementation.

## Failure Classification

- When `status: blocked` identifies a related in-repository fix or evidence change outside the worker's scope that is
  necessary for the approved outcome, treat it as follow-on work rather than a request for fresh authorization. Let
  already-started independent agents finish, gate dependents, extend the manifest with the smallest sufficient scope,
  satisfy repository coordination for that scope, and launch a new or reused implementation agent. Repeat this process
  until the approved outcome is complete or a genuine authorization boundary is reached.
- Ask the user only when continuation would change the approved outcome, require a material redesign or unrelated work,
  or cross an existing confirmation boundary such as destructive action, purchase, deployment, or external write. Never
  silently take over implementation or relaunch solely on a larger model.
- Treat a tool or infrastructure failure as retryable only when adapter-specific evidence supports that classification.
  Inspect partial edits first, then use the adapter's same-agent mechanism for exactly one verify-and-continue attempt.
  This continuation is not a new agent against the eight-agent limit. A second infrastructure failure blocks that agent
  and its dependents.
- Never classify an ordinary timeout, a returned blocker, silence, or task-level validation failure as infrastructure
  failure. Continue only work proven independent.

## Skill Evolution Review

After every required implementation agent has completed successfully and the overall complex task is verified, the
parent agent reviews the user's completed task for skill-evolution opportunities. Do not run or report this review for a
blocked, failed, or partial handoff. The parent agent makes the judgment itself; research and implementation agents
never make the user-facing recommendation.

Recommend skill work only when the completed task exposes a stable, reusable workflow credibly likely to recur. Task
size or difficulty alone does not establish recurrence; reject one-off work, rare contingencies, incidental cleanup, and
patterns whose future value is speculative.

- For a new skill, state whether it belongs in the repository where the work was done because its reuse is
  project-specific or globally in `~/projects/agent-skills` because it is useful across projects.
- For a revision, name every exact existing skill and briefly state why each needs to change.

When a proposal clears this bar, append at most one compact suggestion of no more than two short sentences to the
selected adapter's existing completion report without otherwise changing its format. State the reusable need and the
proposed create or revise target, then offer `$task-handoff` as the next action for capturing a decision-complete
implementation handoff. Leave design choices, file-level changes, acceptance details, and other low-level material to
that future handoff.

Never invoke `$task-handoff`, create a handoff, create a skill, or revise a skill automatically during this review. When
no proposal clears the recurrence bar, remain silent: add no placeholder section and do not report that no skill
opportunity was found.

## Completion

- After every required agent completes, deduplicate the union of reported `changed_files` and confirm the combined
  verification evidence proves the approved plan.
- If any required agent failed, skip every planned polish pass. Otherwise, invoke each required pass once with only its
  applicable paths from that union: `$code-polish` first in its default simplify-then-review mode, then
  `$agents-brain polish` with its eligible context targets. When only one pass is required, invoke only that one. Do not
  seed either pass with paths outside the union or let it broaden beyond its declared workflow authority.
- Reconcile in-scope files actually changed by each polish pass into the final changed-files set and verification. A
  required polish pass that blocks, fails, or writes outside its supported scope blocks later polish and
  cross-repository commits.
- If approved work changes repositories on this machine other than the repository where the handoff began, invoke
  `$commit` from each additional repository after its work, validation, and required polish complete. `$commit` owns
  semantic message composition; its `ai-commit` backend owns deterministic transaction, commit, and push mechanics.
  Scope each invocation to files changed there; do not commit incomplete, blocked, unexpected, or out-of-scope changes.
  Push only when the user explicitly requested it.
- Finish with the selected adapter's completion report. It must include the strategy, wave and agent counts, each
  agent's requested configuration, status, and summary, plus combined changed files, verification, polish when run,
  listing each pass and outcome, automatic cross-repository commit hashes when any, blockers, and residual risks. Write
  `none` for applicable empty values and never expose machine result payloads.
