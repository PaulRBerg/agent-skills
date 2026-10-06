# Native Codex Adapter

Use this route when Codex orchestrates Codex workers. Use the shared contract in `SKILL.md` for planning, authority,
prompts, reconciliation, and completion.

Native multi-agent support is required. Stop with a compatibility blocker if the orchestration tools become unavailable.
Never invoke `codex exec` or replace this native route with a nested Codex process. Explicit Claude workers use their
own adapter.

## Native Agent Configuration

When the user has not specified a model preference, use these tiers for research and implementation:

| Work                                                                              | Model         | Effort             |
| --------------------------------------------------------------------------------- | ------------- | ------------------ |
| Bounded research or routine implementation                                        | `gpt-6-luna`  | `high`             |
| Involved research or implementation                                               | `gpt-6.1-sol` | `medium` or `high` |
| Semantic or cross-cutting implementation                                          | `gpt-6.1-sol` | `xhigh`            |
| Hardest implementation: interacting invariants or difficult algorithmic reasoning | `gpt-6-astra` | `xhigh`            |

Under this default selection, Astra at `xhigh` is the ceiling and Astra is implementation-only. Under that selection,
research agents use Luna or Sol. Research gathers evidence. The parent synthesizes it. Never select `low`, `ultra`, or
`max`. Keep the highest-tier agent's scope minimal and move deferrable validation to the validation owner.

Spawn every research or implementation worker with a self-contained prompt and `fork_turns: "none"`. This avoids copying
the parent conversation and permits explicit `model` and `reasoning_effort` selection. Use a stable lowercase task name
derived from its manifest ID and scope. Preserve the visible `R1` or `A1` ID in the prompt and report.

Never exceed the active-agent concurrency limit reported by the harness. Reserve one slot for the parent. When other
active workers are reported, account for them. Split a wider manifest into dependency-preserving waves. The shared
eight-agent limit counts total implementation agents, not concurrent width. With no reported concurrency cap, launch one
worker at a time.

Codex subagents inherit the parent sandbox and approval policy. Research stays under the parent's read-only controls.
Implementation cannot bypass the permissions selected for the current parent turn. For every research-only handoff, the
shared prompt's strict no-edit boundary is mandatory. Treat any reported research edit as a contract violation.

## Research Mechanics

For each selected research agent, call `spawn_agent` with `fork_turns: "none"`, the selected model and effort, and the
shared self-contained research prompt. Start all agents that fit the current concurrency allowance without waiting
between launches. Place any remainder in a later research wave.

Wait for native results with `wait_agent`. Incorporate the returned findings into the parent plan or research-only
response per the shared Research Phase. Do not create progress, result, stderr, sentinel, or watcher artifacts. Treat
any reported edit as a contract violation.

## Plan Manifest

Use this exact host-specific table inside the shared `## Orchestration` plan section:

```markdown
| Agent | Wave | Depends on | Scope              | Model                                    | Effort                  | Implementation brief                                   | Completion evidence                 |
| ----- | ---- | ---------- | ------------------ | ---------------------------------------- | ----------------------- | ------------------------------------------------------ | ----------------------------------- |
| `A1`  | `1`  | `none`     | `<files/behavior>` | `<gpt-6-luna\|gpt-6.1-sol\|gpt-6-astra>` | `<medium\|high\|xhigh>` | `<outcome, edits, constraints, and stopping criteria>` | `<commands and observable results>` |
```

Use the native configuration table above for every manifest row unless the user's explicit preference overrides its
model selection. Do not add artificial timeout budgets. The harness owns native agent lifetime and waiting.

## Execution Mechanics

The ai-coord session that performs writes owns the claim. Native Codex subagents inherit the parent session identity, so
the parent owns the coordination claim for every delegated write scope. When the guard recognizes the delegate, it
rejects `ai-coord draft`, `ai-coord start`, `ai-coord bundle draft`, `ai-coord bundle start`, `ai-coord wait`, or
`ai-coord done` with exit 64 and
`lifecycle commands are not allowed from a delegate of <client>/<session>; the parent's claim covers this work`. Every
worker prompt must forbid those lifecycle commands and permit only `ai-coord status`, `ai-coord touched`,
`ai-coord inbox`, `ai-coord msg`, and `ai-coord finding`. Include this fact in every worker prompt so the parent's claim
is treated as authorization rather than a conflict. Unrelated claims on the exact assigned scope can still block work.

The native thread-ID guard requires an active delegate record. Missing lifecycle records can leave it unable to reject a
child's command. Worker prompts must require stopping writes and notifying the parent on a scope warning. These prompts
must also forbid workers from repairing claims themselves. Before resuming delegated edits, the parent re-acquires the
complete manifest scope union and requires `READY`. A child must not replace the union with its own subset.

Before implementation wave 1, the parent promotes the named draft recorded over the full manifest write-scope union
during the shared Plan Phase: `ai-coord start --draft <plan-slug>` (or `ai-coord bundle start --draft <plan-slug>` for
two or more Git roots). Only when promotion reports `no draft named ...`, use the plan's explicit
`ai-coord start '<label>' '<path>'...` fallback (or `ai-coord bundle start '<label>' '<absolute-path>'...`) over that
union. Require `READY` before launch.

When the claim queues or blocks, run `ai-coord wait` as a foreground command with a command timeout above its `-t` value
(300 seconds by default). In that case, apply the shared wake handling and repeat until `READY`. Never end the turn
between waits.

After finalizing the plan and receiving `READY` from coordination, call `spawn_agent` for each implementation worker
with:

- `fork_turns: "none"`.
- the model and `reasoning_effort` from its manifest row.
- a stable task name and a self-contained prompt satisfying the shared implementation prompt contract.

Start all independent workers that fit the concurrency allowance without waiting between calls. Reconcile the entire
wave before launching dependents. Never spawn more workers merely because a thread is quiet.

While any agent is running, use `wait_agent` with `timeout_ms: 900000`. It returns early for mailbox updates, completed
results, or user steering. Codex's native thread UI is the progress surface. Do not reproduce it with custom dashboards,
polling loops, wrapper artifacts, or synthetic percentages. Ground any concise user update in an actual agent result or
harness state.

Practice wait economy. When `wait_agent` returns without a settled result, an actionable mailbox message, or user
steering, immediately call it again. Before that call, give the permitted fifteen-minute status update only when one is
due. Do not add analysis, extra narration, or `list_agents` round-trips during that idle wakeup. Reserve reasoning and
user-visible status for settlements, steering-worthy evidence, or that one compact update per roughly fifteen minutes of
elapsed wave time. Every idle wakeup otherwise costs a full model turn.

Use `send_message` only to steer a currently running agent when new evidence shows it is off track or missing material
context. Do not use it for routine check-ins, completed agents, or retries.

## Collection and Failure Handling

Read each completed agent's final message. Require every field in the shared result contract. Apply the shared scope,
validation, dependency-gating, and working-tree reconciliation rules before starting the next wave.

A returned `status: blocked` is a plan blocker, not an infrastructure failure. An agent-tool error or a final result
missing required fields is an infrastructure failure only when the harness evidence supports that classification.

For that infrastructure failure, inspect partial edits. Then use exactly one `followup_task` on that same agent. Use a
short verify-and-continue prompt naming the partial files and missing evidence. Do not spawn a replacement agent. A
second infrastructure failure blocks that agent and its dependents.

## Completion Report

Rely on native thread rendering while work runs. At settlement, render `### ✅ Orchestration completed` or
`### ⛔ Orchestration blocked` with the strategy, total agent count, and wave count. Include a compact per-agent table
with model, effort, result, and summary, then `### 📦 Changed`, `### 🧪 Verification`, `### 🧹 Polish` when applicable,
automatic cross-repository commit hashes when any, and `### Issues and caveats` with the shared contract's `Resolved`
and `Open` groups. Omit empty issue groups and the whole section when empty. Write `none` for other applicable empty
values.
