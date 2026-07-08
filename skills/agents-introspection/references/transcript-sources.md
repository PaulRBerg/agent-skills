# Transcript Sources

Use this reference to locate and sample Codex and Claude Code transcripts for the current working directory. Treat every transcript as sensitive plaintext.

## Local Paths

Resolve local paths once and reuse these variables in later snippets:

```sh
project_path="$(pwd -P)"
home_dir="$(cd ~ && pwd -P)"
claude_config_dir="${CLAUDE_CONFIG_DIR:-$home_dir/.claude}"
codex_home="${CODEX_HOME:-$home_dir/.codex}"
skill_dir="${AGENTS_INTROSPECTION_SKILL_DIR:-}"
if [ -z "$skill_dir" ]; then
  for candidate in "$home_dir/.agents/skills/agents-introspection" "$home_dir/.claude/skills/agents-introspection"; do
    if [ -f "$candidate/scripts/transcript-miner.py" ]; then
      skill_dir="$candidate"
      break
    fi
  done
fi
transcript_miner="$skill_dir/scripts/transcript-miner.py"
test -f "$transcript_miner" || {
  printf '%s\n' "missing agents-introspection transcript miner" >&2
  exit 1
}

printf '%s\n' "$project_path"
printf '%s\n' "$claude_config_dir"
printf '%s\n' "$codex_home"
printf '%s\n' "$transcript_miner"
```

For Claude Code, use `CLAUDE_CONFIG_DIR` when set; otherwise use `~/.claude`.
Encode the absolute path by replacing every non-alphanumeric character with
`-`:

```sh
claude_project_dir="$claude_config_dir/projects/$(printf '%s' "$project_path" | sed 's/[^A-Za-z0-9]/-/g')"
printf '%s\n' "$claude_project_dir"
```

Example: `/Users/prb/projects/prb-math` maps to `~/.claude/projects/-Users-prb-projects-prb-math`.

## Preferred Helper

Run the bundled miner before opening transcript bodies:

```sh
uv run "$transcript_miner" --project "$project_path" --keyword "<keyword>" --format json
```

For explicitly named projects, pass every path and mine only those scopes:

```sh
uv run "$transcript_miner" \
  --project /Users/prb/projects/one \
  --project /Users/prb/projects/two \
  --keyword "<keyword>" \
  --format json
```

Use `--include-archived` only when active Codex sessions do not provide enough signal. The helper reports project coverage, candidate sessions, task themes, correction/failure/verification signals, tool-call counts, and privacy-gap categories. It reads Claude transcripts from `CLAUDE_CONFIG_DIR` when set, falls back to `~/.claude`, and checks the legacy slash-only project key for older local history. It redacts emails, API-key-like strings, private-key-like hex, EVM addresses, transaction hashes, and long secret-like tokens; it does not emit raw transcript excerpts.

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
test -f "$codex_home/session_index.jsonl" &&
  tail -n 200 "$codex_home/session_index.jsonl"
```

Then find transcript files that mention the current project path:

```sh
for dir in "$codex_home/sessions" "$codex_home/archived_sessions"; do
  test -d "$dir" && rg -l -F "$project_path" "$dir"
done
```

If exact-path matching misses relevant sessions, search for repo basename and task keywords:

```sh
project_name="${project_path##*/}"
for dir in "$codex_home/sessions" "$codex_home/archived_sessions"; do
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
