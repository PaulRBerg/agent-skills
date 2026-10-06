# Codex CLI Adapter

Use this route when Claude Code explicitly selects Codex workers, or another harness selects Codex workers by default or
user choice. Default Claude Code workers use the native Claude adapter. Apply the shared contract in `SKILL.md`
throughout this route.

On other harnesses, map Bash background tasks to the host's shell execution and task-continuation tools. Use the
foreground watcher path when Monitor is unavailable. Preserve runner arguments, artifact handling, and settlement rules.

This adapter requires Git, `/bin/bash`, Python 3, and an authenticated Codex CLI with dangerous bypass support. Claude
Code 2.1.98+ is recommended for live progress through the Monitor tool. Stop with a compatibility error when a required
runner prerequisite is unavailable.

## Research Mechanics

For each research agent selected by the shared contract, resolve `../scripts/run-codex-agent.sh` relative to this file
and use the implementation launch template with `--read-only`. Give every agent separate `<agent-id>.progress.jsonl`,
`<agent-id>.result.json`, and `<agent-id>.stderr.log` artifacts. Start all selected agents as background Bash tasks
(`run_in_background: true`) in the same turn. Then watch the wave through the implementation watcher and Monitor flow
below. The runner-enforced read-only sandbox permits this launch in any host mode.

Give each research agent a self-contained prompt containing the open questions, exact investigation scope, read-only
boundary, relevant repository constraints, and stopping rule from the shared prompt contract. Require every field in
`research-result.schema.json`. Prohibit plans, design decisions, and edits.

When the user has not explicitly included research agents in a model preference, select research configuration from
these tiers:

| Investigation                          | Model         | Effort   | Baseline timeout |
| -------------------------------------- | ------------- | -------- | ---------------- |
| Bounded, routine survey                | `gpt-6-luna`  | `high`   | 10 minutes       |
| Involved survey across unfamiliar code | `gpt-6.1-sol` | `medium` | 15 minutes       |

Under this default selection, use Luna for bounded surveys and Sol for involved ones. Under that selection, Astra is
implementation-only. Research gathers evidence. The parent synthesizes it. Never select `low`, `ultra`, or `max`.

Research should normally use shorter budgets than implementation. Keep the baseline between 10 and 15 minutes unless
repository evidence says otherwise.

When the research wave settles, parse each result against `research-result.schema.json`. After that wave settles, read
each result's stderr artifact for failure forensics. Then return the findings to the shared Research Phase for the plan
or research-only response. Do not reconcile the working tree.

## Plan Manifest and Configuration

Use this exact host-specific table inside the shared `## Orchestration` plan section:

```markdown
| Agent | Wave | Depends on | Scope              | Model                                    | Effort                  | Timeout             | Implementation brief                                   | Completion evidence                 |
| ----- | ---- | ---------- | ------------------ | ---------------------------------------- | ----------------------- | ------------------- | ------------------------------------------------------ | ----------------------------------- |
| `A1`  | `1`  | `none`     | `<files/behavior>` | `<gpt-6-luna\|gpt-6.1-sol\|gpt-6-astra>` | `<medium\|high\|xhigh>` | `<minutes> minutes` | `<outcome, edits, constraints, and stopping criteria>` | `<commands and observable results>` |
```

When the user has not specified a model preference, select implementation configuration from these tiers:

| Work                                                                              | Model         | Effort             | Baseline timeout |
| --------------------------------------------------------------------------------- | ------------- | ------------------ | ---------------- |
| Bounded, routine implementation                                                   | `gpt-6-luna`  | `high`             | 10 minutes       |
| Everyday or involved implementation                                               | `gpt-6.1-sol` | `medium` or `high` | 20 minutes       |
| Semantic or cross-cutting implementation                                          | `gpt-6.1-sol` | `xhigh`            | 40 minutes       |
| Hardest implementation: interacting invariants or difficult algorithmic reasoning | `gpt-6-astra` | `xhigh`            | 40 minutes       |

An explicit user model preference replaces this task-complexity model selection. Effort and timeout still follow the
applicable work tier. Never select `low`, `ultra`, or `max`.

Adjust a timeout when repository evidence shows that required validation needs materially more or less time. The timeout
is a kill-switch. It does not set the pace. Codex never sees it, and an early finish costs nothing. Size the timeout
only to bound how long a hung agent can block its wave.

Keep the highest-tier agent's scope minimal and move deferrable validation to the validation owner.

## Execution Mechanics

### Launch

Resolve `../scripts/run-codex-agent.sh` to an absolute path relative to this file. Never search for it in the target
repository. Each invocation is one Codex agent.

Without `--read-only`, the runner deliberately disables Codex approvals and sandboxing. Agents can read, modify, or
delete any files accessible to the host account. An implementation request authorizes this runner mode without a
separate acceptance step, subject to host restrictions and existing confirmation requirements. Keep each agent within
its assigned scope. The runner pins every Codex process to the `default` service tier, overriding inherited fast or
priority selection without changing persisted Codex configuration.

Before implementation wave 1, the parent promotes the named draft recorded over the full manifest write-scope union
during the shared Plan Phase: `ai-coord start --draft <plan-slug>` (or `ai-coord bundle start --draft <plan-slug>` for
two or more Git roots). Only when promotion reports `no draft named ...`, use the plan's explicit
`ai-coord start '<label>' '<path>'...` fallback (or `ai-coord bundle start '<label>' '<absolute-path>'...`) over that
union. Name exact files individually and use `--recursive` only for true subtrees. Require `READY` before launch.

Obtain the parent's verified client and session ID from `ai-coord status --json`. Use that identity in
`--coord-identity`, including in another harness. If required coordination cannot identify the parent, report that
prerequisite. Do not fabricate a Claude or Codex identity.

When the claim queues or blocks, run `ai-coord wait` as a background Bash task (`run_in_background: true`) so its return
wakes the session. In that case, apply the shared wake handling and never end the turn to pause. Hold that claim through
reconciliation, required polish, and commit. The parent claim authorizes each delegate's assigned writes and is not a
conflict.

One work item per session requires the full union at the start. When follow-on work expands the scope, expand it only at
a wave boundary. At that boundary, run `ai-coord done`, then start a fresh item over the enlarged union before launching
the next wave.

For every agent, create separate per-agent artifact paths ending in `<agent-id>.progress.jsonl`,
`<agent-id>.result.json`, and `<agent-id>.stderr.log` under `${TMPDIR:-/tmp}`. Convert its configured whole-minute
timeout to seconds only at the wrapper boundary. Then start the runner from anywhere inside the target Git worktree as a
background Bash task (`run_in_background: true`) with a description like
`Codex A1/3: <scope> (<model>, <effort>, ≤<minutes>m)`:

```bash
bash <skill-dir>/scripts/run-codex-agent.sh \
  --model <agent-model> \
  --effort <agent-effort> \
  --timeout-seconds <agent-minutes-times-60> \
  --coord-identity <parent-client>/<parent-session-id> \
  --progress-file <agent-progress-file> \
  --result-file <agent-result-file> \
  2> <agent-stderr-file> <<'CODEX_PROMPT'
<agent implementation prompt>
CODEX_PROMPT
```

`--result-file` keeps structured JSON out of stdout, and redirecting stderr keeps wrapper diagnostics out of the
background task display. Do not set a Bash-tool timeout. The wrapper's `--timeout-seconds` is the sole timeout
authority. The wrapper always terminates itself.

Start sequential agents only after reconciling their dependencies. Start every agent in a parallel wave in the same
turn. Pass the same `--coord-identity` on every fresh or resumed implementation launch.

Research launches omit it because read-only agents make no writes and remain separately visible.

Delegate prompts forbid ai-coord lifecycle commands. Delegates launched with `--coord-identity` share the orchestrating
session's coordination identity. The guard now rejects a delegate's `ai-coord draft`, `ai-coord start`,
`ai-coord bundle draft`, `ai-coord bundle start`, `ai-coord wait`, or `ai-coord done` with exit 64 and
`lifecycle commands are not allowed from a delegate of <client>/<session>; the parent's claim covers this work`. Require
every implementation prompt to state this rule explicitly. Permit only `ai-coord status`, `ai-coord touched`,
`ai-coord inbox`, `ai-coord msg`, and `ai-coord finding` in that prompt. Also state that the parent's claim authorizes
the assigned writes rather than conflicting with them.

Delegate claims and work no longer appear as separate `ai-coord status` work rows. Transient Codex thread inventory may
remain visible while delegates run. Handoff artifacts contain per-delegate progress.

Add these host constraints to the shared implementation prompt:

- Honor `~/.codex/rules/*.rules`, which the CLI enforces even under the bypass flag. Non-interactive runs reject
  `prompt`-gated commands outright. Skim existing rules and include relevant restrictions in the prompt.
- Baseline command conventions: use `rg`, not `grep` variants. Use `uv run python` and `uv add` or `uv run --with`,
  never bare Python or pip. Keep Bash-only constructs inside an explicit `bash <<'EOF'` block. Avoid recursive removal,
  worktree-destroying or history-rewriting Git, secret-reading commands, and package deploy or release scripts.
- Require every field in `result.schema.json`. The wrapper passes that schema to Codex and writes the structured result
  to the selected artifact.

### Watch

Research and implementation waves share this watcher. Read `progress-events.md` for the progress-event, sentinel,
settlement, and quiet/failure contracts.

Resolve `../scripts/watch-codex-wave.sh` relative to this file. Arm one Monitor per wave around one watcher invocation,
passing each agent's stable ID, budget in seconds, and progress path as a repeated triple:

```sh
bash <skill-dir>/scripts/watch-codex-wave.sh \
  --agent A1 <budget-seconds> <A1.progress.jsonl> \
  --agent A2 <budget-seconds> <A2.progress.jsonl>
```

The watcher tolerates delayed file creation and emits stable JSONL `watcher.digest`, `watcher.sentinel`, and
`watcher.settlement` records. It owns elapsed time, event counts, last relevant activity, settled percentage, and the
ten-cell bar. Set the Monitor `timeout_ms` above the wave's largest budget plus the 120-second no-sentinel grace. On
each digest or settlement, post one short wave-status block using those exact facts. If Monitor is unavailable, run the
same watcher in a foreground command. Do not recreate its loop or arithmetic.

Once Monitor is armed, wait for Monitor events. Do not launch Bash sleeps, tail artifacts, poll result or progress
files, or add any second wait loop. Inspect artifacts only after settlement.

The watcher settles an agent as failed with reason `no-sentinel` once elapsed exceeds its budget plus 120 seconds of
grace. Silence is never evidence of safety buffering or model rerouting. Keep watching until the wrapper sentinel or
configured timeout. Never cancel, retry, extend, or downgrade because of silence. Report `no recent activity` during
quiet periods.

### Collect and Reconcile

When a sentinel arrives, read the result artifact and the stderr artifact for the `orchestration: elapsed=<seconds>s`
line or failure forensics. Do not read or print background-task output. Artifact-mode stdout is intentionally empty.
Parse implementation results against `result.schema.json` before applying the shared reconciliation rules.

At each wave boundary, if `ai-coord status` shows that a delegate narrowed the parent's claim, re-run the parent's full
`ai-coord start` before continuing. This recovery applies only if a delegate bypassed the guard, for example with an
unrelated identity. Normally the guard rejects the lifecycle attempt with exit 64 in the delegate's stderr artifact.

Treat timeouts, nonzero runner exits, and watcher `no-sentinel` settlements as failed settlements, not returned plan
blockers. For a `handoff.failed` sentinel with reason `error`, inspect stderr first. When stderr shows a transport,
stream, or API death and no Codex-reported task failure, inspect partial edits with `git status` and `git diff`. For
that infrastructure failure, extract the session ID from the progress file's `thread.started` event. With that session
ID, perform the shared one allowed same-agent continuation through `--resume <session-id>` with a fresh budget. For that
continuation, use a short verify-and-continue prompt naming the partially edited files.

For that infrastructure failure, fall back to one fresh relaunch only when no session ID is recoverable. Returned
`blocked` results and timeouts are never infrastructure failures.

### Commit Delegated Work

Delegated writes are attributed to the parent session, so they cannot create a delegate residual or stale-dirt baseline
for those paths. Reconcile, perform required polish, and commit under the held parent claim. Release the claim only
afterward.

Legacy troubleshooting: validate an unexpected residual session ID against `thread.started` and its dirt blob hashes
against the reconciled diff. Ask before clearing coordination state. If matching auto-baselines exclude verified
delegate edits, re-prepare with `--no-auto-baseline` and disclose it.

## Status Reporting

These dashboards and the shared completion report are mandatory. Host-rendered background-task and Monitor banners are
transport notifications, not status reports. Do not expose task IDs, raw JSON, sentinels, or monitor payloads.

Use this legend consistently: 🔎 research · 🚀 kickoff · ⏳ running · ✅ completed · ⛔ blocked · ⏱️ timed out · 💥
runner error · 🧹 polish · 🏁 final report. Keep each update to one compact rendered block.

Prefix every wave-scoped kickoff, digest, and completion update with the watcher's exact ten-cell bar, percentage, and
settled counts. Progress means sentinel settlement, including failed sentinels. Never infer progress from elapsed time,
event count, or activity.

Kickoff, once per wave:

```markdown
### 🚀 Wave 1/2 [░░░░░░░░░░] 0% (0/3 settled) — 3 agents launched

| Agent | Scope               | Model · effort        | Budget | State       |
| ----- | ------------------- | --------------------- | ------ | ----------- |
| A1    | `internal/pricing`  | `gpt-6.1-sol` · high  | ≤20m   | 🚀 launched |
| A2    | `internal/backfill` | `gpt-6.1-sol` · high  | ≤20m   | 🚀 launched |
| A3    | `internal/evidence` | `gpt-6.1-sol` · xhigh | ≤40m   | 🚀 launched |
```

Research waves use 🔎 in their heading and investigation scopes in their rows.

Wave status, on each digest or completion:

```markdown
### ⏳ Wave 1/2 [███░░░░░░░] 33% (1/3 settled) — 15m elapsed

| Agent · model/effort   | Status     | Activity                   |
| ---------------------- | ---------- | -------------------------- |
| A1 · gpt-6.1-sol/high  | ⏳ 15m/20m | ran `cargo test`           |
| A2 · gpt-6.1-sol/high  | ✅ 8m      | done — 3 files, tests pass |
| A3 · gpt-6.1-sol/xhigh | ⏳ 15m/40m | no recent activity         |
```

At full settlement, use the final watcher settlement record. A wave with failures still reaches 100%. Its heading and
rows must expose those failures.

## Completion Report

Render `### 🏁 Orchestration [██████████] 100% (<settled>/<total> settled) — <completed|blocked>`. Include strategy,
agent count, and wave count, then one row per agent with result, requested model and effort, timeout budget versus
actual elapsed, output tokens when available, and summary. For a resumed retry, report its sentinel's output-token total
minus the prior run's total as that attempt's usage.

Follow the table with `### 📦 Changed`, `### 🧪 Verification`, `### 🧹 Polish` when applicable, automatic
cross-repository commit hashes when any, and `### Issues and caveats` with the shared contract's `Resolved` and `Open`
groups. Omit empty issue groups and the whole section when empty. Write `none` for other applicable empty values. Never
expose result JSON.
