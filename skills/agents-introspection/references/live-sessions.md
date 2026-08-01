# Live Agent Sessions

Use the machine-wide runtime inventory for requests about Codex or Claude Code sessions that are executing now. This is
top-level session status: spawned subagents are not separate rows.

## Preferred Command

Run the dot-codex status command from either client:

```sh
~/.codex/hooks/AgentSessionStatus/agent_session_status.py status --json
```

If the script is missing, report Codex coverage as unavailable and point to <https://github.com/PaulRBerg/dot-codex>. Do
not reproduce installation instructions, mine historical transcripts, or infer liveness from transcript recency.

Interpret the exit code before reporting:

| Exit | Meaning                                                                    |
| ---- | -------------------------------------------------------------------------- |
| `0`  | Complete Codex and Claude coverage                                         |
| `2`  | Usable output with incomplete provider coverage                            |
| `64` | Invalid command usage; fix the invocation before making any coverage claim |

## Semantics

The command includes sessions with an in-flight turn, including those waiting for user input, permission, or approval.
It excludes idle and completed chats.

| Client | Reported states         | Source                                  |
| ------ | ----------------------- | --------------------------------------- |
| Codex  | `in_flight`             | dot-codex lifecycle-hook registry       |
| Claude | `working` and `waiting` | Native `claude agents --json` inventory |

Codex hooks cannot reliably distinguish generation from an approval wait, so every live Codex turn is `in_flight`.
Claude `blocked` and `waiting` normalize to `waiting`; idle, done, failed, and stopped rows are omitted.

`complete` is true only when both entries under `providers` have `ok: true`. A provider with `ok: false` makes the
inventory partial even when the other provider returned sessions. Treat `sessions` as usable in partial output, visibly
name unavailable providers, and never turn an incomplete empty result into “no agents are executing.”

Report live results as a compact `client`, `state`, `session`, and `cwd` table. Do not include prompts, titles,
assistant text, transcript paths, or tool data.

## Privacy Boundary

The Codex registry stores only its schema version, session and turn IDs, cwd, state, timestamps, and the owning Codex
PID plus process-start fingerprint. Files live under `~/.codex/.tmp/agent-session-status/` with private directory and
file modes. Dead or PID-reused records do not count as live. Claude data comes directly from its native inventory.
