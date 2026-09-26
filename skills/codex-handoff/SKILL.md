---
argument-hint: "[task]"
compatibility:
  The Claude Code host requires Git, /bin/bash, Python 3, and an authenticated Codex CLI with dangerous bypass support;
  the Codex CLI host requires native subagents.
metadata:
  install-targets: claude-code codex
name: codex-handoff
skill-dependencies:
  - agents-brain
  - code-polish
  - commit
description:
  Orchestrate read-only Codex research in any mode, or one to eight Codex agents to implement approved plans from Claude
  Code or Codex CLI.
---

# Codex Handoff

Codex-handoff orchestrates read-only investigation or implementation within the current session after explicit plan
approval. Task-handoff instead writes a decision-complete file for a fresh, separate session; use it when work continues
later or elsewhere, and use an in-session handoff skill to implement an approved plan now.

If these instructions are already present in the conversation from a slash or dollar invocation, follow them directly;
do not invoke this skill again through a skill tool.

Follow the shared contract below, select exactly one host adapter, and use it for every host-specific action.

## Host Selection

Inspect the callable orchestration tools, not environment variables, process ancestry, or a user-supplied host name:

- `spawn_agent`, `wait_agent`, `send_message`, `followup_task` available: read `references/codex-cli-host.md`
  completely.
- Otherwise, Claude Code's Agent and Bash tools available: read `references/claude-code-host.md` completely.
- Neither: stop with a compatibility error. Native Codex multi-agent support is mandatory on the Codex host; never fall
  back to a nested Codex CLI process.

Use exactly one adapter for every host-specific action; never load both or combine their launch, progress, retry,
permission, or result-transport mechanics. The adapter may specialize host mechanics and manifest configuration but
cannot weaken this shared contract.

## Contract

- Run only after explicit invocation. Research-only = requested outcome is findings/evidence/assessment only, no repo
  changes or plan requested. All handoffs may run in any host mode; implementation handoffs must pass through the Plan
  Phase and receive explicit user approval before launch.
- The parent owns decisions, the final plan, and orchestration. Delegate investigation to read-only research agents only
  when task and repository evidence make it useful before planning.
- Research agents gather evidence and report findings only — never edit files, make design decisions, or return plans.
- Implementation agents implement their assigned part of the approved plan: inspect, edit, validate, never redesign or
  return another plan.
- Use the smallest effective implementation team (one agent is valid); add agents only when decomposition materially
  improves latency, correctness, or verification. Never exceed eight implementation agents total.
- Size every brief before finalizing the team: estimate its wall-clock time and split any brief likely to exceed roughly
  25-30 minutes into parallel disjoint scopes or dependency waves — add an integration agent if needed — instead of one
  monolithic agent.
- Use at most three research agents, stable IDs `R1`-`R3`, counted separately from the eight implementation agents.
- Keep the parent's own implementation work to orchestration, integrity checks, failure handling, and conditional polish
  passes.
- Treat an explicit user model preference (e.g. GPT-6 Luna) as an orchestration constraint on every research and
  implementation agent unless scoped narrower; don't substitute the adapter's usual Luna/Sol/Astra selection. If the
  host can't launch that model, report the incompatibility and ask before falling back.
- Treat the approved outcome — not the initial manifest or its write scopes — as the authorization boundary: when
  implementation reveals a related in-repository fix or evidence change required for that outcome, the parent may extend
  the handoff and launch follow-on agents for the newly discovered scope without asking again. The worker that
  discovered the need still stops at its assigned scope and returns evidence; the parent owns scope expansion,
  repository coordination, and delegation.
- Size verification to the requested outcome. Never add validation machinery (gates, manifests, checkpoints, hash pins,
  journals, receipts) unless the approved plan explicitly calls for it. An explicit user request to hurry or wrap up
  overrides optional repeat checks and required polish passes: commit the validated work and report what was skipped or
  left unverified.

Use `$ARGUMENTS` as the task when present; otherwise use the active user request. A task naming another skill follows
Companion Skills.

## Companion Skills

A task that names another skill alongside this handoff — for example,
`run $fresh-eyes-sweep and delegate the implementation` — is a composition: the companion skill defines the work; this
handoff owns delegation mechanics.

- Load the companion skill and run its discovery, judgment, and planning phases in the parent. Route read-only discovery
  through this skill's research agents when materially faster, then fold the companion's method, findings, and
  constraints into the handoff plan. A companion missing from the host's skill list may be installed but hidden by
  `disable-model-invocation: true`: read its `SKILL.md` directly from the host skill root
  (`~/.claude/skills/<name>/SKILL.md` in Claude Code, `~/.agents/skills/<name>/SKILL.md` in Codex) before concluding it
  is absent.
- When the companion's discovery is itself the bulk of the work — an audit or sweep over a whole repository or large
  file set — the parent maps the scope and slices it instead of reading it inline. After plan approval, each
  implementation agent audits and fixes its own slice under the companion's rules, inlined in its brief.
- This contract overrides the companion's overlapping plan approval, agent limits and stable IDs, single validation
  owner, result fields, failure classification, commit ownership, and completion reporting, even when the companion
  prescribes its own subagent, validation, or commit mechanics. Its user-decision gates still bind; this skill's plan
  approval satisfies coincident gates, and a plan-only companion implements only when the combined invocation requested
  implementation and the user approved the plan.
- Agents cannot load skills. Never brief one to "use skill X" by name; inline the specific companion instructions,
  conventions, or excerpts it needs.
- A companion requirement to run `$code-polish` or `$agents-brain polish` marks that pass required in the Plan Phase; it
  still runs once, per Completion. Companion commit instructions never add commits.
- Satisfy both contracts at Completion: produce the companion's required report artifacts (ledgers, tables, verdicts)
  alongside the selected adapter's completion report.

## Research Phase

For a research-only task, launch one to three research agents and stop after returning the consolidated investigation;
never enter the Plan Phase or launch implementation agents. For an implementation handoff, trigger research when scope
is uncertain, the task crosses multiple or unfamiliar subsystems, or gathering the needed evidence serially would be
materially slower for the parent. Zero research agents is the default for implementation handoffs. The parent alone
decides the research count from task and repository evidence; never ask the user to opt in or name agents. Either launch
the research wave immediately or proceed straight to planning.

When triggered, assign up to three agents stable IDs `R1`-`R3` and launch them immediately through the selected
adapter's read-only mechanism. Give each agent a self-contained prompt containing:

- the open questions and exact investigation scope;
- relevant repository constraints and known concurrent-work boundaries;
- a strict read-only authority boundary;
- the stopping rule that it must return evidence rather than a plan or design;
- its time budget, with the instruction to use it: an early `blocked` return citing only time is not a valid stop; and
- exact result fields: `status`, `findings`, `open_questions`, `evidence`, and `blockers`.

A research agent that returns `blocked` citing only its time budget while most of that budget is unused and no concrete
obstacle is named has not settled its scope, and neither has one that returns `completed` while its evidence still lists
uninspected scope paths: continue the same agent once through the adapter's same-agent mechanism with the uncovered
files and the remaining budget. This continuation is not a new research agent.

When every required research agent settles, fold its findings and evidence into the implementation plan or the
research-only response. Surface open questions or blockers through the host's user-question mechanism only when they
change scope or approach. Do not reconcile the working tree — research agents change nothing; any reported edit is a
contract violation.

For a research-only task, synthesize the evidence and finish with `### 🔎 Research handoff — <completed|blocked>`, the
agent count, findings, evidence, open questions, and blockers. This replaces the Plan Phase and the selected adapter's
implementation completion report. If the investigation shows that changes are needed, report them as findings and stop —
do not produce an implementation plan or begin edits.

## Plan Phase

Enter this phase only for an implementation handoff.

When research — delegated or the parent's own — contradicts a fact the user stated explicitly (quantities, which items,
which accounts), ask through the host's user-question mechanism before writing the plan; never widen the plan's default
scope to fit the research.

Produce a decision-complete plan with this section and the selected adapter's exact manifest table:

```markdown
## Codex Handoff

- Research: `<none | R1..Rn — key findings used>`
- Companion skills: `<none | $x — phases the parent runs / what is folded into briefs>`
- Strategy: `<sequential|parallel|hybrid>`
- Agents: `<1-8>` — `<why this is the smallest effective count>`
- Validation owner: `<agent-id|parent>` — `<aggregate checks it runs once>`

<host-adapter manifest table>

- Code polish: `<required|not required>` — `<reason>`
- Agent-context polish: `<required|not required>` — `<reason>`
```

While finalizing the plan, record the union of every manifest write scope with
`ai-coord draft --name <plan-slug> '<label>' '<path>'...`, using `--recursive` only for directory scopes. Derive
`<plan-slug>` from the plan's short identity (for example `debarrel-lib`): lowercase it, replace characters outside
`[A-Za-z0-9._-]` with `-`, and truncate to 40 characters. For two or more Git roots, use
`ai-coord bundle draft --name <plan-slug> '<label>' '<absolute-path>'...`. In the plan's "Wait out conflicting agents"
section, write the exact promote command `ai-coord start --draft <plan-slug>` (or
`ai-coord bundle start --draft <plan-slug>`) and retain the explicit `ai-coord start '<label>' '<path>'...` fallback (or
`ai-coord bundle start '<label>' '<absolute-path>'...`) over the same union. A fresh implementation session must receive
these commands in the plan itself, without reconstructing scopes from prose. Use the fallback only when promotion
reports `no draft named ...`. Named drafts grant no authority and expire after seven days.

Choose the execution shape from repository evidence and the approved work:

- Sequential: one agent depends on another, write scopes overlap, or a later agent owns integration or aggregate
  validation.
- Parallel: independent work only, with explicitly disjoint write scopes. Agents may inspect shared context but must not
  write outside their assigned scope.
- Hybrid: dependency-ordered waves — run independent agents within a wave in parallel, reconcile the entire wave, then
  start its dependents.

A wave finishes with its slowest agent. Keep the highest-tier agent's scope minimal and move deferrable validation to
the validation owner. If parallel work does not collectively prove the overall plan, reserve a later sequential agent
for integration and aggregate validation. Use stable agent IDs and explicit dependencies across the whole handoff.

Assign aggregate validation to exactly one owner: package- or repository-wide checks run once, by the integration agent
when one exists, otherwise by the parent during post-wave reconciliation. Every other agent runs only the narrowest
checks that prove its own edits, such as file-scoped formatting, lint, or typecheck plus targeted tests.

Require `$code-polish` for nonlocal invariants, concurrency or state machines, migrations or parsing, auth or security,
retry or error semantics, and public API or data-contract changes. File count alone is not a trigger.

Require `$agents-brain polish` when approved work changes a target its polish workflow supports: README.md, AGENTS.md or
CLAUDE.md, a durable context doc, an existing project-installed skill under `.agents/skills`, or an existing git-tracked
source-catalog skill under `skills/` where polish is prose-only. Installed copies under managed agent-config roots
remain excluded. Mark both passes required when both trigger rules apply; mark neither when neither applies.

Do not launch implementation agents until the user approves the plan. Read-only research is the only pre-approval
exception.

## Implementation Prompt Contract

Build a self-contained, outcome-first prompt for every implementation agent. Include:

1. The approved overall outcome plus the agent's implementation brief, dependencies, and completion evidence.
2. Its exact write scope, relevant repository constraints, known dirty-work boundaries, and prerequisite agent results.
3. Its validation assignment per the Plan Phase's single validation owner: scoped checks it must run and, unless it owns
   validation, that it must not run aggregate checks. Never brief new validation machinery the approved plan does not
   call for.
4. A soft time budget matching its manifest sizing, with the instruction to return `blocked` with partial evidence
   rather than grinding past it.
5. This authority boundary: inspect, edit only within the assigned scope, and validate locally; never commit, push,
   deploy, make external writes, or broaden scope, even when repository or host instructions favor committing finished
   work promptly. Committing stays with the parent after reconciliation.
6. The selected adapter's delegation and coordination context, including why the parent session and disjoint siblings
   are not conflicting work and what unrelated exact-scope claim would justify returning `blocked`. Delegates must not
   run coordination lifecycle commands: these are rejected with exit 64. Permit only `ai-coord status`,
   `ai-coord touched`, `ai-coord inbox`, `ai-coord msg`, and `ai-coord finding`.
7. This stopping rule: implement the approved plan exactly; if infeasible or requiring redesign, return `blocked` with
   evidence instead of proposing a replacement plan.
8. A requirement to return every result field: `status` (`completed` or `blocked`), `summary`, `changed_files` listing
   only files actually touched, `verification` listing every command and outcome, `residual_risks`, and `blockers`.

Keep each agent prompt as compact as completeness allows: the shared outcome summary plus that agent's own brief, scope,
and constraints — never restate the full plan text per agent.

Add the selected adapter's command, permission, transport, and host-tool constraints without restating this contract.

## Execution and Reconciliation

Before implementation wave 1, the parent promotes the plan's named draft to acquire the full manifest write-scope union.
Use the plan's recorded explicit start fallback only when promotion reports `no draft named ...`; require `READY` before
launching agents. When promotion or start queues or blocks instead, never end the turn to pause: run `ai-coord wait`
through the adapter's wait mechanics. It also returns on non-readiness wake events (message, unknown coverage, work
release, 300-second default timeout); on each wake, handle `MESSAGE` events through `ai-coord inbox`, re-submit the
recorded promote or start command, and diagnose stale blockers.

Launch agents through the selected adapter in the approved strategy and dependency waves. Do not add agents or change
models, efforts, scopes, or validation ownership merely because a worker is slow or quiet. Do revise the manifest and
launch a narrowly scoped follow-on agent when completed work discovers an unplanned prerequisite covered by the approved
outcome. Preserve stable IDs, dependency order, the eight-agent limit, and one aggregate-validation owner; include
follow-on agents in the final counts and report.

For each completed agent: require every shared result field and treat `changed_files` as its authoritative post-pass
scope; confirm reported files exist or were intentionally deleted, stay within scope, and carry verification evidence
matching the assignment; and pass relevant completed results to dependent agents.

After every implementation wave, reconcile all results with the current manifest and visible working tree without
folding in unrelated concurrent changes. When the parent owns validation, run the assigned aggregate checks once during
this reconciliation. Attribute aggregate-check failures before blocking: first rule out effects of the handoff's
changes, formatters, hooks, and generators, including failures in downstream files outside its write scopes. Continue
past a failure only when evidence establishes that it is unrelated and the handoff's own checks still pass. Unexpected
out-of-scope edits, same-wave overlap, or a failure attributable to the handoff are blockers; do not start dependents or
polish, and do not silently take over implementation.

## Failure Classification

- A `status: blocked` result identifying a related in-repository fix or evidence change outside the worker's scope that
  is necessary for the approved outcome is follow-on work under the Contract's scope-expansion authority, not a request
  for fresh authorization: let already-started independent agents finish, gate dependents, extend the manifest with the
  smallest sufficient scope, satisfy repository coordination for that scope, and launch a new or reused implementation
  agent. Repeat until the outcome is complete or a genuine authorization boundary is reached.
- Ask the user only when continuation would change the approved outcome, require a material redesign or unrelated work,
  or cross an existing confirmation boundary such as destructive action, purchase, deployment, or external write. Never
  silently take over implementation or relaunch solely on a larger model.
- Treat a tool or infrastructure failure as retryable only when adapter-specific evidence supports that classification.
  Inspect partial edits first, then use the adapter's same-agent mechanism for exactly one verify-and-continue attempt;
  this is not a new agent against the eight-agent limit. A second infrastructure failure blocks that agent and its
  dependents.
- Never classify an ordinary timeout, a returned blocker, silence, or task-level validation failure as infrastructure
  failure. Continue only work proven independent.

## Skill Evolution Review

Keep verified repairs to skills used during the handoff separate from the optional review below. When user or repository
instructions already authorize repairs, the parent owns their completion; subagents report evidence without expanding
their write scopes. One verified occurrence is enough, and a blocked main task does not prevent independent repairs.
Complete the handoff's required work or establish its blocker, then finish independent repairs before the final report
under the applicable maintenance policy. Plan Mode still prohibits edits.

After every required agent succeeds and the task is verified — never for a blocked, failed, or partial handoff — the
parent alone judges skill-evolution opportunities; agents never make the recommendation. Recommend only a stable,
reusable workflow credibly likely to recur; reject one-offs, rare contingencies, incidental cleanup, and speculative
value. For a new skill, state repo-local vs. global `~/projects/agent-skills` placement; for a revision, name the exact
skills and why. When a proposal clears this bar, append at most one compact suggestion (≤2 short sentences) to the
adapter's existing completion report without changing its format, then offer `$task-handoff` as the next action. Never
auto-invoke `$task-handoff` or create/revise anything during this review; when nothing qualifies, stay silent — no
placeholder.

## Completion

- After every required agent completes, deduplicate the union of reported `changed_files` and confirm the combined
  verification evidence proves the approved plan.
- Before the completion report, fix remaining same-pattern sites the approved outcome covers through follow-on agents
  per Failure Classification; never list them as optional or out-of-scope items.
- If any required agent failed, or the user explicitly asked to hurry or wrap up, skip every planned polish pass and
  report the skip. Otherwise, invoke each required pass once with only its applicable paths from that union:
  `$code-polish` first in its default simplify-then-review mode, then `$agents-brain polish` with its eligible context
  targets. Invoke only one when only one is required. Do not seed either pass with paths outside the union or let it
  broaden beyond its declared workflow authority.
- Reconcile in-scope files actually changed by each polish pass into the final changed-files set and verification. A
  required polish pass that blocks, fails, or writes outside its supported scope blocks later polish and
  cross-repository commits.
- If approved work changes repositories on this machine other than the one where the handoff began, invoke `$commit`
  from each additional repository after its work, validation, and required polish complete, scoped to files changed
  there; do not commit incomplete, blocked, unexpected, or out-of-scope changes. Push only when the user explicitly
  requested it.
- When the handoff pushed commits and the repository defines CI workflows, such as `.github/workflows`, watch the pushed
  head's runs before the completion report (`gh run list --commit <sha>`, then `gh run watch <run-id>`, in the
  background when the host supports it). Fix failures attributable to the handoff as follow-on work and report the CI
  outcome. When changed code behaves differently by platform and local checks covered only one, name the unverified
  platforms as a risk.
- Finish with the selected adapter's completion report, including strategy, wave and agent counts, each agent's
  requested configuration, status, and summary, plus combined changed files, verification, polish when run (listing each
  pass and outcome), automatic cross-repository commit hashes when any, and `Issues and caveats` when present. Write
  `none` for other applicable empty values and never expose machine result payloads.
- Group issues and caveats as `Resolved` (verified fixes with evidence) and `Open` (remaining problems, limitations, or
  unverified assumptions, with impact and next step). Omit empty groups and the whole section when empty. Report each
  item once; put neutral context and agreed decisions under changes or scope. Reserve `blocker` for something preventing
  required work and `risk` for a specific potential adverse outcome. A workaround leaves an item open when the
  underlying issue still affects the result.
