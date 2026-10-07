# Transcript Sources

Before opening transcript bodies, use the bundled miner to rank Codex and Claude Code sessions. Include the current
project and any other local project materially relevant to the user's task. Reading relevant other-project sessions is
authorized. Treat the miner's output as heuristic discovery data, not evidence.

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

Run one unarchived-session pass with 3–6 task keywords. Include the current project and every task-relevant local
project as a repeated `--project` argument:

```sh
uv run "$transcript_miner" \
  --project "$project_path" \
  --keyword '<keyword-1>' \
  --keyword '<synonym-a>|<synonym-b>' \
  --since 60d \
  --excerpts \
  --max-sessions 8 \
  --format json
```

`--keyword` accepts `|`-separated OR-groups (e.g. `--keyword 'miner|mining|transcript-miner'`) to encode synonyms as one
group instead of separate flags.

For an explicitly relevant former project root, add `--historical-project '<former-path>'`. This repeatable argument
permits absent directories. Existing paths must be directories. The helper canonicalizes these roots and applies the
same native ownership and lineage checks. It does not infer aliases or change historical ownership to a current root.
`--project` still requires an existing directory and defaults to the current directory when omitted, even with
`--historical-project`. Repeated canonical roots appear once. Other retrieval bounds remain unchanged.

Include another project without requesting permission when task context, an explicit project or path reference, a shared
change or workflow, or session metadata establishes relevance. Never infer relevance from a shared basename or keyword
alone.

The helper returns project coverage, ranked candidate sessions, task themes, correction and failure signals,
verification signals, tool-call counts, and `privacy_gaps` categories. It always redacts common secret-like values. The
helper emits transcript excerpts only with `--excerpts`. With that flag, each candidate has up to 3 redacted
`{channel, text}` entries, each truncated to 240 characters. These entries come from the first user message plus up to 2
keyword-matching messages, preferring user over assistant.

Scores and counts select candidates only. Validate every reported finding against the relevant transcript body.
`keyword_hits` counts eligible `(message, keyword)` pairs keyed by the full OR-group keyword string, not repeated
substring occurrences.

With `--since <YYYY-MM-DD|Nd>`, the report gains a top-level `since` object (`value`, `cutoff`, `codex_dirs_pruned`,
`codex_files_pruned`, `claude_files_pruned`). Without the flag, the object is `null`. Each candidate has a `modified`
ISO mtime. Sessions modified within 7 days score higher than those within 30 days, which score higher than older
sessions. This score is another ranking signal, not evidence.

Source ownership is structural and precedes relevance scoring:

- For Codex, the helper uses `session_meta.payload.cwd`. When that field is absent, the helper accepts sampled
  `turn_context.cwd` values only when all resolve under one requested project.
- For Claude, the helper reconciles the encoded project directory, top-level transcript `cwd`, and
  `history.jsonl.project` when a history record exists. The helper excludes conflicts rather than guessing.
- A cwd equal to or below multiple requested roots belongs to the longest, most-specific root. A transcript is emitted
  at most once, and project strings in messages, context, tool inputs, or tool outputs never establish ownership.

By default, the helper excludes the live `CODEX_THREAD_ID` or `CLAUDE_CODE_SESSION_ID` transcript. Use
`--include-current` only when diagnosing the miner or intentionally inspecting the active session.

Each candidate exposes `session_kind` (`primary`, `subagent`, `guardian`, or `unknown`) and `parent_session_id` (string
or `null`). The helpers derive these fields from native metadata, independently of project ownership:

- Codex `session_meta.payload.source.subagent.other: guardian` identifies a guardian approval session. The miner
  excludes these sessions after ownership checks and before relevance sampling or ranking, including with
  `--include-current`.
- A Codex subagent source, `thread_source: subagent`, or explicit `parent_thread_id` identifies a subagent. The helpers
  expose `parent_thread_id` as `parent_session_id` when present.
- Codex `thread_source: user` identifies a primary session. Sources `cli` and `vscode` also identify primary sessions
  when `thread_source` is absent and no child metadata is present.
- Claude `isSidechain: true` identifies a subagent. An explicit `false` identifies a primary session when no sampled
  record has `true`. Claude message `parentUuid` never establishes a parent session, so the parent ID remains `null`.
- Missing or unrecognized metadata yields `unknown`. The miner retains these sessions and ordinary subagents. Neither an
  unknown kind nor a missing parent ID proves independence. Verify independence before counting separate occurrences.

These rules reflect observed native transcript metadata. They do not establish a version-independent format guarantee. A
parent and its children provide one source of recurrence evidence. Guardian sessions can repeat parent history and must
not add independent occurrences.

Candidate signals use separate channels. `user` is actual task text, preferring Claude history `display`. `assistant` is
plain assistant message text. Injected AGENTS, skill, environment, permission, collaboration, abort, and command
envelopes are ignored `context`. `tool` contributes names plus structured error status or nonzero exit codes only.

Serialized tool inputs and raw outputs never contribute keywords or behavioral regex signals. The helper deduplicates
identical eligible messages within each channel.

Each project coverage record retains `codex_candidates`, `claude_candidates`, and `selected_sessions`.
`codex_candidates` and `claude_candidates` mean structurally owned sessions, including sessions that did not meet the
keyword relevance requirement, after current-session and guardian exclusions. Additional fields make selection and
exclusions auditable:

- `codex_scanned`, `claude_scanned`, `structurally_matched`, and `relevance_matched` describe source coverage.
- `current_sessions_excluded`, `guardian_sessions_excluded`, `content_only_project_mentions_ignored`, and
  `ambiguous_ownership_excluded` count distinct session files, not occurrences. Guardian exclusion takes precedence when
  a session also matches the live session ID.
- candidate `ownership` records `matched_via`, canonical `cwd`, and assigned `project`.
- candidate `signal_channels` records eligible user and assistant message counts, ignored context, and structured tool
  failures.

If a specific reported incident is missing, use Exact-Incident Fallback before widening the time window. Otherwise, make
one pass with broader or OR-grouped keywords. If still weak, retry once with `--since` removed. Add `--include-archived`
only for the final bounded fallback. Empty output after those passes is a coverage gap, not proof that no relevant
behavior exists.

## Inspect Candidates

Use the bundled inspector to get a bounded, redacted digest of a candidate transcript before opening its raw body:

```sh
uv run "$skill_dir/scripts/transcript-inspect.py" <transcript-path>... \
  --keyword '<keyword-1>' \
  --max-entries 120 \
  --format text
```

For each file, the inspector emits a header with source, session ID, `session_kind`, `parent_session_id`, cwd, timestamp
range, per-channel totals, and sampled flag. Explicit guardian paths remain inspectable. The inspector emits bounded
entries with absolute record line numbers. Within the sampled records and output limit, these entries contain
non-context user messages, qualifying assistant messages, and tool failures. Redaction is always on. Entry text is
capped at 240 characters.

Digests are redacted and bounded. Inspect them before reading raw bodies. Read raw bodies only when the digest is
insufficient. Each entry's line number lets you retrieve the exact underlying record when needed:

```sh
sed -n '<line+1>p' <transcript-path> | jq
```

## Exact-Incident Fallback

When a specific reported incident is absent from ranked candidates or a sampled digest, make one search without body
sampling. Keep the same project, time, and archive bounds. Search a short, distinctive incident phrase or a small set of
close variants. Return matching filenames first, for example with `rg -l`, rather than raw JSONL records.

For Claude, search the exact encoded project directory. For Codex, first select files whose source-native metadata
establishes project ownership. Verify ownership before inspecting each match. Do not search unrelated projects or treat
the miner's ranked session limit as complete coverage.

Inspect each matching file with the inspector first. If its sampled digest omits the match, decode only the matching
records and the minimum adjacent context. Redact secrets before emitting text. When a parent identifies a relevant child
transcript, inspect that exact child file and verify its cwd and parent linkage. Count inspected child bodies against
the body limit. A parent and its child do not establish independent recurrence.

## Source Layouts

Claude Code uses `CLAUDE_CONFIG_DIR` when set and otherwise defaults to `~/.claude`. Project transcripts normally live
under:

```text
<claude-config>/projects/<absolute-path-with-nonalphanumerics-replaced-by-dashes>/
```

The helper parses `<claude-config>/history.jsonl` once, indexing source-native `project`, `sessionId`, and user-authored
`display` fields. History relevance preselects sessions before the helper samples their transcript bodies. Thus, an
older relevant session remains discoverable behind any number of newer irrelevant files. For sessions without history
records, the helper uses bounded head/tail transcript sampling. The helper also checks the legacy slash-only project
encoding and excludes directory, cwd, or history disagreements.

Codex uses `CODEX_HOME`, defaulting to `~/.codex`:

- Unarchived transcripts: `sessions/`
- Archived transcripts: `archived_sessions/`
- Recent-session index: `session_index.jsonl`

Session JSONL commonly contains `session_meta`, `turn_context`, `event_msg`, and `response_item` records. Prefer
JSON-aware inspection of the smallest relevant record range to keep retrieval bounded and focused on useful evidence.

## Manual Fallback

Use this only when the helper is missing or fails. Preserve the same relevant-project and retrieval bounds, secret
handling, and external-disclosure boundary.

1. For Claude Code, compute the encoded directory from the exact absolute project path and inspect newest JSONL files
   there.
2. For Codex, search unarchived transcript metadata for the exact absolute project path. Search archives only after the
   unarchived pass is insufficient.
3. If exact matching is suspiciously empty, use the repository basename only to identify candidates. Then reject every
   candidate whose source-native metadata or cwd does not resolve to the current or a task-relevant project. Never use a
   content occurrence as ownership evidence.
4. Filter candidates with the task keywords before opening bodies. Inspect at most five unless evidence conflicts or the
   user requested exhaustive coverage.

Prefer `rg`, `fd`, `jq`, and structured parsing. If a command returns partial output or errors, try one equivalent
scoped command before reporting the gap. Never compensate by searching unrelated project history.

Transcript JSONL embeds tool output as JSON strings, so quotes inside that content appear escaped in raw text
(`\"key\":\"value\"`). When grepping raw transcript files, allow optional backslashes in the pattern (e.g. `\\?"`) or
decode lines with `jq` before matching. A pattern written for decoded JSON will silently miss raw-text matches. This
caveat mainly matters when bypassing the inspector and grepping raw bodies directly. The inspector's own digest output
is plain decoded text.

## Secret Handling and External Disclosure

- Use direct transcript evidence when it materially strengthens an internal report. Keep excerpts bounded and relevant.
- Always redact credentials such as API keys, private keys, mnemonics, tokens, and passwords. Never expose personal
  wallet addresses. Before public or third-party disclosure, also remove emails, unrelated personal or customer data,
  unsuitable private paths or repository names, and unrelated transcript material.
- Write transcript content to durable repository artifacts only when the task authorizes it and the evidence materially
  belongs there. Perform an external-disclosure review before posting, publishing, uploading, or otherwise sending the
  artifact outside the agent workspace.
- Include raw transcript paths in the report only when they materially help the user audit a finding.
