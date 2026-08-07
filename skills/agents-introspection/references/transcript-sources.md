# Transcript Sources

Use the bundled miner to rank project-scoped Codex and Claude Code sessions before opening transcript bodies. Treat its
output as heuristic discovery data, not evidence.

## Resolve Local Paths

Resolve paths once and reuse them:

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
```

When the skill host exposes its installation directory directly, resolve `scripts/transcript-miner.py` from that
directory instead of searching installed copies.

## Preferred Helper

Run one unarchived-session pass with 3–6 task keywords:

```sh
uv run "$transcript_miner" \
  --project "$project_path" \
  --keyword '<keyword-1>' \
  --keyword '<keyword-2>' \
  --max-sessions 8 \
  --format json
```

For explicitly named additional projects, repeat `--project` with each absolute path. Never infer a second project from
a shared basename or keyword.

The helper returns project coverage, ranked candidate sessions, task themes, correction and failure signals,
verification signals, tool-call counts, and `privacy_gaps` categories. It redacts common secret-like values and emits no
transcript excerpts. Scores and counts select candidates only; validate every reported finding against the relevant
transcript body.

If the first pass is weak, make one broader-keyword pass. Add `--include-archived` only for the final bounded fallback.
Empty output after those passes is a coverage gap, not proof that no relevant behavior exists.

## Source Layouts

Claude Code uses `CLAUDE_CONFIG_DIR` when set and otherwise defaults to `~/.claude`. Project transcripts normally live
under:

```text
<claude-config>/projects/<absolute-path-with-nonalphanumerics-replaced-by-dashes>/
```

The helper also checks the legacy slash-only encoding. Validate the decoded project path or transcript metadata before
reading a body.

Codex uses `CODEX_HOME`, defaulting to `~/.codex`:

- Unarchived transcripts: `sessions/`
- Archived transcripts: `archived_sessions/`
- Recent-session index: `session_index.jsonl`

Session JSONL commonly contains `session_meta`, `turn_context`, `event_msg`, and `response_item` records. Prefer
JSON-aware inspection of the smallest relevant record range to keep retrieval bounded and high-signal.

## Manual Fallback

Use this only when the helper is missing or fails. Preserve the same project and retrieval bounds, secret handling, and
external-disclosure boundary.

1. For Claude Code, compute the encoded directory from the exact absolute project path and inspect newest JSONL files
   there.
2. For Codex, search unarchived transcript metadata for the exact absolute project path. Search archives only after the
   unarchived pass is insufficient.
3. If exact matching is suspiciously empty, use the repository basename only to identify candidates, then reject every
   candidate whose metadata or cwd does not resolve to an explicitly allowed project.
4. Filter candidates with the task keywords before opening bodies. Inspect at most five unless evidence conflicts or the
   user requested exhaustive coverage.

Prefer `rg`, `fd`, `jq`, and structured parsing. If a command returns partial output or errors, try one equivalent
scoped command before reporting the gap; never compensate by searching unrelated project history.

Transcript JSONL embeds tool output as JSON strings, so quotes inside that content appear escaped in raw text
(`\"key\":\"value\"`). When grepping raw transcript files, allow optional backslashes in the pattern (e.g. `\\?"`) or
decode lines with `jq` before matching; a pattern written for decoded JSON will silently miss raw-text matches.

## Secret Handling and External Disclosure

- Use direct transcript evidence when it materially strengthens an internal report; keep excerpts bounded and relevant.
- Always redact credentials such as API keys, private keys, mnemonics, tokens, and passwords. Never expose personal
  wallet addresses. Before public or third-party disclosure, also remove emails, unrelated personal or customer data,
  unsuitable private paths or repository names, and unrelated transcript material.
- Write transcript content to durable repository artifacts only when the task authorizes it and the evidence materially
  belongs there. Perform an external-disclosure review before posting, publishing, uploading, or otherwise sending the
  artifact outside the agent workspace.
- Include raw transcript paths in the report only when they materially help the user audit a finding.
