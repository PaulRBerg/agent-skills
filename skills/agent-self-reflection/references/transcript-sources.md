# Transcript Sources

Use this reference to locate and sample Codex and Claude Code transcripts for the current working directory. Treat every transcript as sensitive plaintext.

## Project Path

Resolve the project path once:

```sh
project_path="$(pwd -P)"
printf '%s\n' "$project_path"
```

For Claude Code, encode the absolute path by replacing `/` with `-`:

```sh
claude_project_dir="$HOME/.claude/projects/$(printf '%s' "$project_path" | tr '/' '-')"
printf '%s\n' "$claude_project_dir"
```

Example: `/Users/prb/projects/prb-math` maps to `~/.claude/projects/-Users-prb-projects-prb-math`.

## Claude Code

Claude Code project transcripts live under the encoded project directory:

```sh
test -d "$claude_project_dir" && rg --files "$claude_project_dir" || true
```

Expect JSONL-style chat logs and related metadata. Prefer newest files first. Filter by task keywords before reading full bodies when many transcripts exist.

Useful first pass:

```sh
test -d "$claude_project_dir" &&
  rg -n -i -e '<keyword-1>|<keyword-2>' "$claude_project_dir" || true
```

Replace the placeholder keywords with task terms. Use `rg -F` for literal strings containing shell-sensitive characters.

## Codex

Codex stores transcripts under `CODEX_HOME`, defaulting to `~/.codex`.

Documented locations:

- Active transcripts: `~/.codex/sessions`
- Archived transcripts: `~/.codex/archived_sessions`
- Prompt history: `~/.codex/history.jsonl`
- Recent-session index: `~/.codex/session_index.jsonl`

Active session files are commonly date-sharded:

```text
~/.codex/sessions/YYYY/MM/DD/rollout-YYYY-MM-DDTHH-MM-SS-<uuid>.jsonl
```

Each transcript is newline-delimited JSON:

```json
{"timestamp":"...","type":"...","payload":{}}
```

Common record types:

- `session_meta`: session id, cwd, CLI version, model provider, git metadata.
- `turn_context`: cwd, workspace roots, date/timezone, approval policy, sandbox policy, model, effort, summary.
- `event_msg`: visible user/agent events, token counts, task start.
- `response_item`: model messages, function calls, function outputs, reasoning summaries or encrypted reasoning blobs.

Use the index for titles and recency:

```sh
test -f "$HOME/.codex/session_index.jsonl" &&
  tail -n 200 "$HOME/.codex/session_index.jsonl"
```

Then find transcript files that mention the current project path:

```sh
for dir in "$HOME/.codex/sessions" "$HOME/.codex/archived_sessions"; do
  test -d "$dir" && rg -l -F "$project_path" "$dir"
done
```

If exact-path matching misses relevant sessions, search for repo basename and task keywords:

```sh
project_name="${project_path##*/}"
for dir in "$HOME/.codex/sessions" "$HOME/.codex/archived_sessions"; do
  test -d "$dir" && rg -l -i -e "$project_name|<keyword-1>|<keyword-2>" "$dir"
done
```

Prefer JSON-aware inspection when `jq` is available:

```sh
jq -r 'select(.type=="session_meta" or .type=="turn_context") | .payload' < transcript.jsonl
```

## Sampling

Use this order unless the user asks for exhaustive analysis:

1. Project-matching metadata from Codex and Claude Code.
2. Recent sessions with task-keyword overlap.
3. Sessions where the user corrected the agent or where tool calls failed.
4. Comparable successful sessions.
5. Archived sessions only if active sessions do not provide enough signal.

Good evidence includes a repeated correction, a failed command followed by the same failed approach, an AGENTS.md violation, an unrelated edit, or a verification gap that survived to the final answer.

Weak evidence includes one ambiguous failure, generic assistant apology text, or a transcript that only shares a broad keyword with the task.

## Privacy

- Summarize instead of quoting.
- Redact secrets, API keys, private keys, mnemonics, tokens, private wallet addresses, emails, and customer/user data.
- Do not write transcript snippets into `AGENTS.md`, `README.md`, skills, commit messages, or issue bodies.
- Keep raw transcript paths in the report only when they help the user audit the conclusion.
