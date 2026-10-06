# Claude CLI Adapter

Use this route when Codex or another harness orchestrates and the user explicitly selects Claude workers or a Claude
model. Run authenticated Claude CLI workers through the host's shell execution tool. Do not send Claude model names to
Codex's `spawn_agent` or substitute Codex workers.

The [Claude headless CLI documentation](https://code.claude.com/docs/en/headless) defines print mode, structured output,
permission controls, and session continuation. Check `claude --version`, `claude --help`, and `jq --version` before
launch. Require support for `--permission-prompts`, `--json-schema`, and the other flags below. If a prerequisite fails,
report the incompatibility without changing worker family.

## Configuration and Plan Manifest

Honor the user's exact model or custom agent choice. Otherwise, use `sonnet` for bounded research and routine
implementation. Use `opus` for involved research or implementation with interacting invariants. Let the configured
Claude effort apply unless the user specifies an effort. For a custom agent, verify `--agent <name>` support and
availability. Add that option to the launch without overriding its model unless the user requested a model too.

Use this table in the shared `## Orchestration` plan section:

```markdown
| Agent | Wave | Depends on | Scope              | Model                             | Implementation brief                        | Completion evidence                 |
| ----- | ---- | ---------- | ------------------ | --------------------------------- | ------------------------------------------- | ----------------------------------- |
| `A1`  | `1`  | `none`     | `<files/behavior>` | `<sonnet\|opus\|user preference>` | `<outcome, constraints, stopping criteria>` | `<commands and observable results>` |
```

## Permissions and Coordination

The host's permissions and Plan Mode restrictions remain binding. Do not use another CLI to bypass them. For research,
expose only read tools as described below. For implementation, the template permits local shell commands and file edits,
subject to configured deny rules. The shared prompt limits those permissions to the assigned scope and prohibits
commits, external writes, deployment, and further delegation. Do not add a permission bypass to recover a denied action.

Acquire the full manifest scope in the parent and require `READY` before implementation. Pass the verified parent
identity through `AI_COORD_CLIENT=<parent-client>` and `AI_COORD_SESSION_ID=<parent-session-id>`. Obtain that identity
from `ai-coord status --json`, not the new Claude session ID. Preserve it on continuation. Research needs no write
claim. If required coordination cannot identify the parent, report that prerequisite. Do not fabricate a Claude or Codex
identity.

Every worker prompt must prohibit coordination lifecycle commands. The parent's claim authorizes the assigned writes. On
a scope warning, the worker stops writes and reports to the parent. It never repairs or replaces the parent's claim. For
blocked parent claims, run `ai-coord wait` through the host command tool and follow shared wake handling.

## Launch and Wait

Create a separate temporary artifact directory for each worker. Record its paths in the parent before launch. Run from
the target repository. Supply the self-contained shared prompt through a quoted heredoc. Resolve the schema relative to
this skill, not the target repository.

Use this implementation template in a dedicated shell execution call. Substitute shell-quoted literal paths and values:

```sh
printf '%s\n' "$$" > '<agent-dir>/pid'
exec env AI_COORD_CLIENT='<parent-client>' AI_COORD_SESSION_ID='<parent-session-id>' \
  claude --print --model '<agent-model>' \
  --permission-mode acceptEdits --permission-prompts none \
  --tools 'Read,Glob,Grep,Edit,Write,Bash' --allowedTools 'Read,Glob,Grep,Edit,Write,Bash' \
  --strict-mcp-config --mcp-config '{"mcpServers":{}}' \
  --output-format json --json-schema "$(cat '<skill-dir>/references/result.schema.json')" \
  > '<agent-dir>/result.json' 2> '<agent-dir>/stderr.log' <<'AGENT_PROMPT'
<self-contained implementation prompt>
AGENT_PROMPT
```

For research, use `--permission-mode plan`, `--tools 'Read,Glob,Grep'`, and `--allowedTools 'Read,Glob,Grep'`. Select
`references/research-result.schema.json` and the shared read-only research prompt. Keep the empty MCP configuration.
These tool restrictions exclude shell execution, edits, and nested agents. If research needs an excluded tool, the
parent gathers that evidence under its own authority.

Preserve configured instructions and hooks. Do not add `--bare`, which changes context and authentication behavior. CLI
sessions can write their own session state. Keep that distinct from research edits to the task repository.

Start independent workers without waiting for earlier workers to finish. Retain each execution session handle and PID.
Use the host's continuation or wait tool to await process exit. A yielded command or quiet output is still running. Do
not create a second watcher or infer progress percentages. Keep user updates brief and evidence-based.

If the user cancels, send SIGTERM to the recorded worker PID, await exit, and verify it stopped. Never leave launched
workers running after the orchestration ends. Apply a hard runtime limit only when the user or finalized plan requires
one, and terminate that worker when it expires.

## Results and Continuation

After process exit, inspect the exit code, stderr, and JSON envelope. Require a successful result subtype, no error
flag, and `structured_output` with every field required by the selected schema. Exit code zero alone does not prove the
task completed. A structured `status: blocked` remains a task blocker. Inspect permission denials when work is
incomplete.

Use `jq '.structured_output' '<agent-dir>/result.json'` to read the worker's result. Read its session ID with
`jq -r '.session_id' '<agent-dir>/result.json'`. Do not treat missing or malformed fields as success.

Retain the envelope's exact `session_id`. On an evidenced transport or infrastructure failure, inspect partial edits and
use the shared one allowed continuation. Add `--resume '<session-id>'` to the same launch configuration with new
artifact paths and a short verify-and-continue prompt. Never use `--continue`, which can select another concurrent
session. If no session ID is recoverable, report the worker blocked instead of launching a duplicate against partial
work.

Reconcile structured results under the shared contract. Report the route as Claude CLI workers, including selected
models, status, changed files, exact verification, and material permission or runtime limits. Use
`### ✅ Orchestration completed` or `### ⛔ Orchestration blocked`. Do not expose raw result envelopes.
