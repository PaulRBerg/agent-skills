# Codex CLI Host Adapter

Load this adapter only when `SKILL.md` selects Codex's native orchestration tools. Do not read or apply the Claude Code
adapter in the same handoff.

Native multi-agent support is required. If the orchestration tools become unavailable, stop with a compatibility
blocker. Never invoke `codex exec` or fall back to any nested CLI process.

## Native Agent Configuration

When the user has not specified a model preference, use these tiers for research and implementation:

| Work                                       | Model           | Effort   |
| ------------------------------------------ | --------------- | -------- |
| Bounded research or routine implementation | `gpt-5.6-terra` | `medium` |
| Involved research or implementation        | `gpt-5.6-terra` | `high`   |
| Semantic or cross-cutting implementation   | `gpt-5.6-sol`   | `xhigh`  |

Under this default selection, Sol at `xhigh` is the ceiling and is implementation-only; research agents always use Terra
— research gathers evidence, the parent synthesizes. Never select `low`, `ultra`, or `max`, and do not choose Luna
unless the user's explicit preference requires it. Keep the highest-tier agent's scope minimal and move deferrable
validation to the validation owner.

Spawn every research or implementation worker with a self-contained prompt and `fork_turns: "none"`. This avoids copying
the parent conversation and permits explicit `model` and `reasoning_effort` selection. Use a stable lowercase task name
derived from its manifest ID and scope, and preserve the visible `R1` or `A1` ID in the prompt and report.

Never exceed the active-agent concurrency limit reported by the harness. Reserve one slot for the parent and account for
other active workers when the harness reports them. Split a wider manifest into dependency-preserving waves; the
eight-agent shared limit is total implementation agents, not concurrent width. When no concurrency cap is reported,
launch one worker at a time.

Codex subagents inherit the parent sandbox and approval policy. Plan-mode research therefore stays under the parent's
read-only controls, and implementation cannot bypass the permissions selected for the approved parent turn.

## Research Mechanics

For each selected research agent, call `spawn_agent` with `fork_turns: "none"`, the selected model and effort, and the
shared self-contained research prompt. Start all agents that fit the current concurrency allowance without waiting
between launches; place any remainder in a later research wave.

Wait for native results with `wait_agent`. Fold the returned findings into the parent plan as required by the shared
Research Phase. Do not create progress, result, stderr, sentinel, or watcher artifacts. Treat any reported edit as a
contract violation.

## Plan Manifest

Use this exact host-specific table inside the shared `## Codex Handoff` plan section:

```markdown
| Agent | Wave | Depends on | Scope              | Model                                        | Effort                  | Implementation brief                                   | Completion evidence                 |
| ----- | ---- | ---------- | ------------------ | -------------------------------------------- | ----------------------- | ------------------------------------------------------ | ----------------------------------- |
| `A1`  | `1`  | `none`     | `<files/behavior>` | `<gpt-5.6-luna\|gpt-5.6-terra\|gpt-5.6-sol>` | `<medium\|high\|xhigh>` | `<outcome, edits, constraints, and stopping criteria>` | `<commands and observable results>` |
```

Use the native configuration table above for every manifest row unless the user's explicit preference overrides its
model selection. Do not add artificial timeout budgets: native agent lifetime and waiting are owned by the harness.

## Execution Mechanics

The parent session owns the coordination claim for every delegated write scope. Native subagents share that session
ownership and must never run `ai-coord start`, `ai-coord wait`, or `ai-coord done`. Include this fact in every worker
prompt so the parent's claim is treated as authorization rather than a conflict; unrelated claims on the exact assigned
scope can still block work.

After plan approval and exit from Plan mode, call `spawn_agent` for each implementation worker with:

- `fork_turns: "none"`;
- the model and `reasoning_effort` from its approved manifest row;
- a stable task name and a self-contained prompt satisfying the shared implementation prompt contract.

Start all independent workers that fit the concurrency allowance without waiting between calls. Reconcile the entire
wave before launching dependents. Never spawn more workers merely because a thread is quiet.

Use `wait_agent` to await mailbox updates and completed results. Codex's native thread UI is the progress surface: do
not reproduce it with custom dashboards, polling loops, wrapper artifacts, or synthetic percentages. Ground any concise
user update in an actual agent result or harness state.

Practice wait economy: when `wait_agent` returns without a settled result or an actionable mailbox message, immediately
call `wait_agent` again with no analysis, narration, or `list_agents` round-trips. Reserve reasoning and user-visible
status for settlements, steering-worthy evidence, or at most one compact update per roughly fifteen minutes of elapsed
wave time; every idle wakeup otherwise costs a full model turn.

Use `send_message` only to steer a currently running agent when new evidence shows it is off track or missing material
context. Do not use it for routine check-ins, completed agents, or retries.

## Collection and Failure Handling

Read each completed agent's final message and require every field in the shared result contract. Apply the shared scope,
validation, dependency-gating, and working-tree reconciliation rules before starting the next wave.

A returned `status: blocked` is a plan blocker, not an infrastructure failure. An agent-tool error or a final result
missing required fields is an infrastructure failure only when the harness evidence supports that classification. After
inspecting partial edits, use exactly one `followup_task` on that same agent with a short verify-and-continue prompt
naming the partial files and missing evidence. Do not spawn a replacement agent. A second infrastructure failure blocks
that agent and its dependents.

## Completion Report

Rely on native thread rendering while work runs. At settlement, render `### 🏁 Codex handoff — <completed|blocked>` with
the strategy, total agent count, and wave count. Include a compact per-agent table with model, effort, result, and
summary, then `### 📦 Changed`, `### 🧪 Verification`, `### 🧹 Polish` when applicable, automatic cross-repository
commit hashes when any, and an always-present `### ⚠️ Risks / blockers`; write `none` when empty.
