---
compatibility: Requires ~/.codex/hooks/AgentSessionStatus/agent_session_status.py.
disable-model-invocation: false
name: agents-status
user-invocable: true
description: Report currently active Codex and Claude Code sessions with a compact live runtime inventory.
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

Group rows by `client`, `state`, and `cwd`; report a `count` instead of session IDs or duplicate rows. Name unavailable
providers. An empty partial result is unknown, not “no active sessions.”
