# Native Claude Adapter

Use this route when Claude Code orchestrates Claude workers. Use the shared contract in `SKILL.md` for planning,
authority, prompts, reconciliation, and completion.

## Configuration

Before each new brief launches, follow [Jev configuration routing](jev-routing.md). Use these model tiers as candidates
and as the local fallback when Jev cannot provide a sound selection:

| Work                                                                 | Model    |
| -------------------------------------------------------------------- | -------- |
| Bounded research or routine implementation                           | `sonnet` |
| Involved research across unfamiliar subsystems                       | `opus`   |
| Semantic or cross-cutting implementation with interacting invariants | `opus`   |

Use the Agent tool's supported model aliases or the user's exact supported model. The Agent tool has no per-call effort
control. Subagents inherit session effort. If the user names a custom agent, verify its availability and use that type.
Its tools and instructions must support the assigned authority boundary. Report an incompatibility before substituting.
Keep effort fixed to `inherited` during Jev selection. Never pass an effort argument to Agent or create a custom agent
to set it. Preserve a custom agent's host-controlled settings unless the user explicitly overrides them. Record the
selection source in each brief.

Let Claude Code select foreground or background delivery. Only a terminal result, error, or completion notification
settles an agent. A launch acknowledgement, task row, quiet period, or permission prompt does not settle it. Reconcile a
wave only after every required terminal outcome arrives.

## Research

Launch research through Agent with `subagent_type: "Explore"` and the selected explicit model. Use stable IDs `R1` to
`R3`. Start independent agents together, within any host concurrency limit. When Plan Mode prescribes exploration, these
agents provide it. The parent writes the plan and launches no planning agents.

Include all task-relevant repository constraints in each prompt. Explore does not load project `CLAUDE.md`. Add a
thoroughness hint: `medium` for bounded surveys or `very thorough` for multi-subsystem investigations. Require the
shared research result fields. Prohibit edits, design decisions, and plans.

Explore agents are one-shot and return no reusable agent ID. Do not apply implementation continuation to them. Missing
evidence remains an open question or blocker in the consolidated result.

## Plan Manifest

Use this table in the shared `## Orchestration` plan section:

```markdown
| Agent | Wave | Depends on | Scope              | Model                             | Implementation brief                        | Completion evidence                 |
| ----- | ---- | ---------- | ------------------ | --------------------------------- | ------------------------------------------- | ----------------------------------- |
| `A1`  | `1`  | `none`     | `<files/behavior>` | `<sonnet\|opus\|user preference>` | `<outcome, constraints, stopping criteria>` | `<commands and observable results>` |
```

Keep table cells to one line. Put longer briefs under per-agent headings. Do not invent native runtime limits.

## Implementation

The parent acquires and retains the full manifest write-scope claim. Require `READY` before launch. For a blocked claim,
run `ai-coord wait` with Bash `run_in_background: true`. On each wake, follow the shared coordination rules and resubmit
the complete claim.

Claude subagents inherit the parent session identity. The claim authorizes their assigned writes. The delegate guard
cannot distinguish them from the parent, so every brief must prohibit coordination lifecycle commands. Name the parent
session and explain that disjoint siblings are not conflicts. An unrelated claim on the exact assigned files can block
writes. On a scope warning, workers must stop writes and notify the parent instead of repairing claims themselves.

Launch implementation through Agent with `subagent_type: "general-purpose"`, the selected model, and a description such
as `A1 — <scope>`. An explicit compatible custom agent choice replaces `general-purpose`. Start independent workers
together. Start dependents only after reconciling their prerequisites.

Agents receive none of the planning conversation. Supply the complete shared implementation prompt contract. Require the
exact shared result keys, including `changed_files` and `residual_risks`. Retain each returned agent ID for
continuation.

## Progress and Failure Handling

Post one brief research or implementation launch update. Use Claude Code's native progress rendering. Do not build
dashboards or polling loops. A text-only progress report does not prove completion.

An Agent tool error or final message missing required fields can be an infrastructure failure. Inspect partial edits
before continuing. For that failure, use `SendMessage` addressed to the returned agent ID once. Name the partial files
and missing evidence. Preserve prior context. This retry does not count as a new implementation agent.

If no ID was returned, or continuation fails, the agent is blocked. There is one exception: after a harness stall,
verify that the assigned write scope is untouched. Only then may you relaunch once with the same brief and model. That
retry also does not count as a new implementation agent. Ordinary timeouts, task blockers, and validation failures do
not qualify.

## Completion Report

Use `### ✅ Orchestration completed` after all required work is verified, or `### ⛔ Orchestration blocked` otherwise.
Include strategy, worker family, agent and wave counts, and a compact per-agent result table. Follow the shared
completion contract for changed files, exact verification, polish, commits, and issues.
