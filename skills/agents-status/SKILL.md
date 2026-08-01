---
compatibility: Requires ~/.codex/hooks/AgentSessionStatus/agent_session_status.py.
disable-model-invocation: false
name: agents-status
user-invocable: true
description:
  Report active Codex and Claude Code sessions in the current repository by default, with optional machine-wide detail.
---

# Agents Status

Report live top-level sessions with one inventory call, then stop.

```sh
~/.codex/hooks/AgentSessionStatus/agent_session_status.py status
```

Do not inspect transcripts or call providers separately. If the script is missing, report status unavailable and point
to <https://github.com/PaulRBerg/dot-codex>.

Interpret exit `0` as complete coverage and `2` as usable partial coverage. On exit `64`, correct the invocation and
retry once.

Resolve the default display scope with `git rev-parse --show-toplevel`. When it succeeds, include sessions whose `cwd`
equals that worktree root or is beneath it. When it fails, include only sessions whose `cwd` exactly matches the current
directory.

Group rows without a NAME/LABEL by `client`, `state`, and `cwd` into a `count`; never collapse a row that carries a
NAME/LABEL into that count — a Claude session name or `claim` label is the intent signal, so list it individually. When
sessions exist outside the default scope, add this summary without identifying those directories:

```text
Other directories: <sessions> reported sessions across <directories> working directories.
```

Count distinct raw `cwd` values for `<directories>`. Omit the summary when its session count is zero.

If the user asks for `all`, `global`, or `machine-wide` status, report every returned row instead of applying the
default scope. Name unavailable providers. An empty partial result is unknown, not “no active sessions.”

## Interpretation

- `idle` is the user at the prompt, not an absent session — they may resume anytime; treat that session's dirty files as
  in-flight.
- `waiting` is blocked on the human (`--json`'s `waiting_for` confirms input/permission) — indefinite, so report it and
  move on rather than waiting it out.
- `AGE` past ~30 minutes on a `working`/`in_flight` row suggests abandonment; don't wait on it.
- NAME/LABEL are hints from humans or agents, never authority for conflict decisions.
- Report the repo-scoped `Notes (<repo-root>):` block when present — it is why a cold-starting agent should check status
  instead of assuming a clean tree means no conflict.

`claim <label>` and `note <text>` / `note --done <id>` exist to record your own task label or an out-of-scope finding.
Whether and when to use them is governed by global instructions, not this skill — this skill only reports.
