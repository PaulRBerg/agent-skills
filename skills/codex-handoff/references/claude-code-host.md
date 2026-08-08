# Claude Code Host Adapter

Load this adapter only when `SKILL.md` selects the Claude Code orchestration surface. Do not read or apply the Codex CLI
adapter in the same handoff.

This adapter requires Git, `/bin/bash`, Python 3, and an authenticated Codex CLI with dangerous bypass support. Claude
Code 2.1.98 or newer is recommended for live progress through the Monitor tool. Stop with a compatibility error when a
required runner prerequisite is unavailable.

## Research Mechanics

For each research agent selected by the shared contract, resolve `../scripts/run-codex-handoff.sh` relative to this file
and use the implementation launch template with `--read-only`. Give every agent separate `<agent-id>.progress.jsonl`,
`<agent-id>.result.json`, and `<agent-id>.stderr.log` artifacts. Start all selected agents as background Bash tasks
(`run_in_background: true`) in the same turn, then watch the wave through the implementation watcher and Monitor flow
below. The runner-enforced read-only sandbox makes this launch legitimate in any mode, including Plan mode.

Give each research agent a self-contained prompt containing the open questions, exact investigation scope, read-only
boundary, relevant repository constraints, and stopping rule from the shared prompt contract. Require every field in
`research-result.schema.json`; prohibit plans, design decisions, and edits.

When the user has not explicitly included research agents in a model preference, select research configuration from
these tiers:

| Investigation                          | Model           | Effort             | Baseline timeout |
| -------------------------------------- | --------------- | ------------------ | ---------------- |
| Bounded, routine survey                | `gpt-5.6-luna`  | `medium`           | 10 minutes       |
| Involved survey across unfamiliar code | `gpt-5.6-terra` | `medium` or `high` | 15 minutes       |

Use Luna for bounded surveys and Terra for involved ones; research never uses Sol — research gathers evidence, the
parent synthesizes. Never select `low`, `ultra`, or `max`. Research should normally use shorter budgets than
implementation; keep the baseline between 10 and 15 minutes unless repository evidence requires otherwise.

When the research wave settles, parse each result against `research-result.schema.json`, read its stderr artifact for
failure forensics, and return the findings to the shared Research Phase for the plan or research-only response. Do not
reconcile the working tree.

## Plan Manifest and Configuration

Use this exact host-specific table inside the shared `## Codex Handoff` plan section:

```markdown
| Agent | Wave | Depends on | Scope              | Model                                        | Effort                  | Timeout             | Implementation brief                                   | Completion evidence                 |
| ----- | ---- | ---------- | ------------------ | -------------------------------------------- | ----------------------- | ------------------- | ------------------------------------------------------ | ----------------------------------- |
| `A1`  | `1`  | `none`     | `<files/behavior>` | `<gpt-5.6-luna\|gpt-5.6-terra\|gpt-5.6-sol>` | `<medium\|high\|xhigh>` | `<minutes> minutes` | `<outcome, edits, constraints, and stopping criteria>` | `<commands and observable results>` |
```

When the user has not specified a model preference, select implementation configuration from these tiers:

| Work                                     | Model           | Effort             | Baseline timeout |
| ---------------------------------------- | --------------- | ------------------ | ---------------- |
| Bounded, routine implementation          | `gpt-5.6-luna`  | `medium`           | 10 minutes       |
| Everyday or involved implementation      | `gpt-5.6-terra` | `medium` or `high` | 20 minutes       |
| Semantic or cross-cutting implementation | `gpt-5.6-sol`   | `xhigh`            | 40 minutes       |

An explicit user model preference replaces this task-complexity model selection, but effort and timeout still follow the
applicable work tier. Never select `low`, `ultra`, or `max`. Adjust a timeout when repository evidence shows that
required validation needs materially more or less time. The timeout is a kill-switch, not pacing: Codex never sees it
and an early finish costs nothing, so size it only to bound how long a hung agent can block its wave.

Keep the highest-tier agent's scope minimal and move deferrable validation to the validation owner.

## Execution Mechanics

### Launch

Resolve `../scripts/run-codex-handoff.sh` to an absolute path relative to this file; never search for it in the target
repository. Each invocation is one Codex agent.

Without `--read-only`, the runner deliberately disables Codex approvals and sandboxing. Use that mode only after the
user approves the plan and accepts that agents can read, modify, or delete any files accessible to the host account. The
runner pins every Codex process to the `default` service tier, overriding inherited fast or priority selection without
changing persisted Codex configuration.

Before launching agents, do not hold a path-scoped session claim over any path in an agent's write scope. Record
orchestrator intent with a pathless label only; the delegated agents own per-path claims. Tell every agent that the
orchestrating session's presence authorizes its assigned work and is not a conflict.

For every agent, create separate per-agent artifact paths ending in `<agent-id>.progress.jsonl`,
`<agent-id>.result.json`, and `<agent-id>.stderr.log` under `${TMPDIR:-/tmp}`. Convert its approved whole-minute timeout
to seconds only at the wrapper boundary, then start the runner from anywhere inside the target Git worktree as a
background Bash task (`run_in_background: true`) with a description like
`Codex A1/3: <scope> (<model>, <effort>, ≤<minutes>m)`:

```bash
bash <skill-dir>/scripts/run-codex-handoff.sh \
  --model <agent-model> \
  --effort <agent-effort> \
  --timeout-seconds <agent-minutes-times-60> \
  --progress-file <agent-progress-file> \
  --result-file <agent-result-file> \
  2> <agent-stderr-file> <<'CODEX_PROMPT'
<agent implementation prompt>
CODEX_PROMPT
```

`--result-file` keeps structured JSON out of stdout, and redirecting stderr keeps wrapper diagnostics out of the
background task display. Do not set a Bash-tool timeout; the wrapper's `--timeout-seconds` is the sole timeout authority
and always terminates itself. Start sequential agents only after reconciling their dependencies. Start every agent in a
parallel wave in the same turn.

Add these host constraints to the shared implementation prompt:

- Honor `~/.codex/rules/*.rules`, which the CLI enforces even under the bypass flag. Non-interactive runs reject
  `prompt`-gated commands outright. Skim existing rules and include relevant restrictions in the prompt.
- Baseline command conventions: use `rg`, not `grep` variants; use `uv run python` and `uv add` or `uv run --with`,
  never bare Python or pip; keep Bash-only constructs inside an explicit `bash <<'EOF'` block; avoid recursive removal,
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
`watcher.settlement` records. It owns elapsed time, event counts, last command or file activity, settled percentage, and
the ten-cell bar. Set the Monitor `timeout_ms` above the wave's largest budget plus the 120-second no-sentinel grace. On
each digest or settlement, post one short wave-status block using those exact facts. If Monitor is unavailable, run the
same watcher in a foreground command; do not recreate its loop or arithmetic.

Once Monitor is armed, wait for Monitor events. Do not launch Bash sleeps, tail artifacts, poll result or progress
files, or add any second wait loop. Inspect artifacts only after settlement.

The watcher settles an agent as failed with reason `no-sentinel` once elapsed exceeds its budget plus 120 seconds of
grace. Silence is never evidence of safety buffering or model rerouting. Keep watching until the wrapper sentinel or
approved timeout; never cancel, retry, extend, or downgrade because of silence. Report `no recent activity` during quiet
periods.

### Collect and Reconcile

When a sentinel arrives, read the result artifact and the stderr artifact for the `codex-handoff: elapsed=<seconds>s`
line or failure forensics. Do not read or print background-task output; artifact-mode stdout is intentionally empty.
Parse implementation results against `result.schema.json` before applying the shared reconciliation rules.

Treat timeouts, nonzero runner exits, and watcher `no-sentinel` settlements as failed settlements, not returned plan
blockers. For a `handoff.failed` sentinel with reason `error`, inspect stderr first. When it evidences a transport,
stream, or API death and no Codex-reported task failure, inspect partial edits with `git status` and `git diff`. Extract
the session ID from the progress file's `thread.started` event and perform the shared one allowed same-agent
continuation through `--resume <session-id>` with a fresh budget and a short verify-and-continue prompt naming the
partially edited files. Fall back to one fresh relaunch only when no session ID is recoverable. Returned `blocked`
results and timeouts are never infrastructure failures.

## Status Reporting

These dashboards and the shared completion report are mandatory. Host-rendered background-task and Monitor banners are
transport notifications, not status reports. Do not expose task IDs, raw JSON, sentinels, or monitor payloads.

Use this legend consistently: 🔎 research · 🚀 kickoff · ⏳ running · ✅ completed · ⛔ blocked · ⏱️ timed out · 💥
runner error · 🧹 polish · 🏁 final report. Keep each update to one compact rendered block.

Prefix every wave-scoped kickoff, digest, and completion update with the watcher's exact ten-cell bar, percentage, and
settled counts. Progress means sentinel settlement, including failed sentinels; never infer it from elapsed time, event
count, or activity.

Kickoff, once per wave:

```markdown
### 🚀 Wave 1/2 [░░░░░░░░░░] 0% (0/3 settled) — 3 agents launched

| Agent | Scope               | Model · effort        | Budget | State       |
| ----- | ------------------- | --------------------- | ------ | ----------- |
| A1    | `internal/pricing`  | `gpt-5.6-sol` · high  | ≤30m   | 🚀 launched |
| A2    | `internal/backfill` | `gpt-5.6-sol` · high  | ≤30m   | 🚀 launched |
| A3    | `internal/evidence` | `gpt-5.6-sol` · xhigh | ≤40m   | 🚀 launched |
```

Research waves use 🔎 in their heading and investigation scopes in their rows.

Wave status, on each digest or completion:

```markdown
### ⏳ Wave 1/2 [███░░░░░░░] 33% (1/3 settled) — 15m elapsed

| Agent · model/effort   | Status     | Activity                   |
| ---------------------- | ---------- | -------------------------- |
| A1 · gpt-5.6-sol/high  | ⏳ 15m/20m | ran `cargo test`           |
| A2 · gpt-5.6-sol/high  | ✅ 8m      | done — 3 files, tests pass |
| A3 · gpt-5.6-sol/xhigh | ⏳ 15m/20m | no recent activity         |
```

At full settlement, use the final watcher settlement record. A wave with failures still reaches 100%; its heading and
rows must expose those failures.

## Completion Report

Render `### 🏁 Codex handoff [██████████] 100% (<settled>/<total> settled) — <completed|blocked>`. Include strategy,
agent count, and wave count, then one row per agent with result, requested model and effort, timeout budget versus
actual elapsed, output tokens when available, and summary. For a resumed retry, report its sentinel's output-token total
minus the prior run's total as that attempt's usage.

Follow the table with `### 📦 Changed`, `### 🧪 Verification`, `### 🧹 Polish` when applicable, automatic
cross-repository commit hashes when any, and `### ⚠️ Risks / blockers`; write `none` for applicable empty values. Never
expose result JSON.
