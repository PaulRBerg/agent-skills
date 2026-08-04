# ai-coord

Local coordination for parallel Codex and Claude Code agents.

`ai-coord` replaces scattered hook scripts and multi-command conflict checks with a three-command lifecycle:

```sh
ai-coord start 'update importer' 'src/importer'
ai-coord wait
ai-coord done
```

## Repository layout

```
.
├── cli/        Python `ai-coord` CLI, hooks, ledger, and local dashboard API
├── dashboard/  Bun-managed Vite and React live coordination dashboard
└── justfile    Shared development recipes
```

The coordinator is cooperative rather than an OS lock. It uses a private local SQLite ledger and fails closed when it
cannot establish complete provider coverage. Unattributed relevant dirt settles for at most ~90 seconds, then work may
proceed with a stale-dirt advisory and a captured baseline.

## Installation

Requirements: Python 3.12+ and [uv](https://docs.astral.sh/uv/). Automatic Codex hook trust requires Codex CLI 0.146.0
or newer; compatible later versions are accepted only when the required app-server protocol and trust semantics still
validate.

```sh
uv tool install 'git+https://github.com/PaulRBerg/ai-coord.git'
ai-coord link all
ai-coord check
```

`link` merges owned hooks into `~/.codex/hooks.json` and `~/.claude/settings.json`. It preserves unrelated settings and
hook commands. Successful Codex links also automatically trust only the exact `ai-coord` hook definitions they own; they
never use a broad trust bypass or manually derived hash. `--dry-run` is fully read-only, including no Codex app-server
call, and reports `trust=skipped`. Codex `--path` accepts only the active `$CODEX_HOME/hooks.json`, which prevents an
invocation from trusting hooks in another configuration; Claude `--path` can target one non-default settings file.
Output retains its TSV columns and adds `trust=updated`, `trust=unchanged`, or `trust=skipped`. `CODEX_HOME` and
`CLAUDE_CONFIG_DIR` override the corresponding default configuration roots.

When a Claude configuration uses the modular source `~/.claude/settings/hooks.jsonc`, `link` updates that file instead
of the generated `settings.json`. Run the configuration repository's normal settings merge afterward so Claude Code
receives the regenerated output.

If Codex cannot inspect or update that narrow trust record, `link codex` fails. `link all` stops before linking Claude;
the already-written Codex hook file is intentionally not rolled back.

## Coordination workflow

Acquire literal file or directory scopes before editing:

```sh
ai-coord start 'regenerate 2025 tax year' \
  'accounting/txs/incomes' \
  'accounting/reports/2025'
```

Scopes are repository-relative prefixes. `.` covers the worktree. Globs, non-printable paths, normalized scopes over 120
characters, and paths outside the repository are rejected so overlap checks stay exact.

`start` emits one tab-separated result:

| Result                     | Exit | Meaning                                                           |
| -------------------------- | ---: | ----------------------------------------------------------------- |
| `READY`                    |    0 | The claim is active; editing may begin.                           |
| `INTENT`                   |    0 | A pathless, non-exclusive label was recorded.                     |
| `BLOCKED`                  |    3 | The work is queued behind an active or earlier overlapping claim. |
| `UNKNOWN coverage`         |    2 | Provider coverage is incomplete; work was not granted.            |
| `UNKNOWN dirty-settling:…` |    2 | Relevant unattributed dirt is settling; wait and retry.           |
| `ACTIVE`                   |    3 | This session already owns a different scope; release it first.    |

Blocked work retains its paths and queue position. Waiting therefore needs no repeated session or path arguments:

```sh
ai-coord wait        # waits up to 300 seconds
ai-coord wait -t 60  # explicit timeout, capped at one hour
```

Only `READY` authorizes editing. `wait` checks the SQLite generation counter each second and performs full inventory,
Git, and arbitration refreshes only when coordination state changes or every 20 seconds as a fallback. A new message or
repository note wakes `wait` with exit 3 so the caller can inspect it and re-arm. `done` idempotently releases active,
queued, or intent-only work and directly notifies overlapping queued holders that their claim may now be ready.

FIFO applies among intersecting queued scopes; disjoint queued work can proceed independently. A newly blocked claim
also sends a bounded system message to its current holders.

In Claude Code, a blocked `ai-coord start` launches a background waker that wakes the session when its claim is
promoted, a message or note arrives, the claim is released, coverage becomes unknown, or the waker times out. The wake
reminder always requires re-running `start` before editing. Repeated `start` calls may launch multiple independent
wakers for the same session; each exits on the first terminal outcome. Codex sessions use `ai-coord wait` in the
foreground.

Sessions whose hooks report plan mode are labeled `planning` in `status` and the dashboard, so peers can distinguish
planning presence from active implementation work.

When `READY` includes `stale-dirt:<paths>`, preserve those pre-existing hunks byte-for-byte. Run `ai-coord baseline` to
print their blob OIDs and pass affected paths to the commit skill's baseline exclusion. A session that finishes with
uncommitted dirt retains residual ownership and can reclaim it immediately.

Repositories may list harness churn in a tracked `.ai-coord.toml`:

```toml
[dirt]
benign = ["config.toml"]
```

Benign prefixes never hold. The CLI writes `runner.json` in the state directory; an older CLI encountering a newer state
schema re-execs the newer runner automatically.

## Inventory and communication

```sh
ai-coord status              # current Git worktree
ai-coord status --all        # machine-wide
ai-coord status --json       # versioned JSON schema
ai-coord name '👩‍💻 Baroness Byte'
ai-coord msg '019fbf24' 'Changes are committed; your path is clear.'
ai-coord inbox
ai-coord inbox --ack '<message-id>'
ai-coord note 'Verified stale importer assumption.'
ai-coord note --done '<note-id>'
```

`status` exits 0 for complete coverage, 2 for usable partial coverage, and 1 on error. Its plain-text output ends with a
contextual legend; `--json` remains the versioned JSON schema.

Callsigns are machine-wide unique while their top-level session remains in the ledger. They must contain a letter or
number and an emoji, are capped at 40 Unicode code points, and are normalized for whitespace, case-insensitive
uniqueness, and equivalent emoji presentation. Naming is optional: immutable session IDs remain the identity and
fallback everywhere.

Message targets resolve an exact `client/session` or session ID first, then an exact callsign, a unique ID prefix of at
least four characters, or a unique callsign/label/provider-name substring. `repo` expands to the currently live peers in
the Git worktree. Messages are private to their recipients and snapshot both endpoint callsigns when sent, so later
renames do not rewrite history; notes are durable, repo-scoped findings visible to future sessions.

`ai-coord trailer` prints the current Git attribution line:

```text
Agent-Session: codex/019fc27b-b4fb-7322-b65c-ed2471a6fce9
```

## Hooks and health

Lifecycle and nudge hooks invoke `ai-coord hook codex` or `ai-coord hook claude`. Session-start and prompt hooks give
unnamed top-level sessions a bounded static reminder to choose a funny emoji callsign; prompt hooks combine it with the
capped presence count and stop reminding immediately after naming. Claude's `PostToolBatch` hook and Codex's
`PostToolUse` hook inject a counts-only reminder once when unread peer messages arrive; peer text remains available only
through `ai-coord inbox`. Stop and session-end hooks update or release the corresponding session; subagent hooks add
read-only parent/child topology. Claude's `ExitPlanMode` hook records only the approved plan's first H1 as a pathless
intent label, while its filtered `ai-coord waker claude` hook handles blocked starts in the background.

Hook mode is fail-open. Malformed payloads and storage errors never block the host and never expose raw data on stdout.
`ai-coord check` reports hook-health codes and exits 2 for a usable but degraded installation.

### Subagents

Both hosts fire `SubagentStart` and `SubagentStop` with the parent `session_id`. `ai-coord` records delegates under that
parent; it never creates child sessions or claims, and child tool calls refresh the parent session. Coordination is
therefore session-scoped: the parent's claim covers all delegated work. Subagents must never run lifecycle commands
(`start`, `wait`, or `done`) themselves because their inherited identity would make those commands act as the parent.

## Legacy migration

Install and test the new CLI before switching hooks:

```sh
ai-coord migrate legacy --dry-run
ai-coord migrate legacy
```

The importer reads the former `$CODEX_HOME/.tmp/agent-session-status` registry, claims, messages, and notes (defaulting
to `~/.codex`). Imports are content-hash idempotent. Literal claims retain their state; legacy glob claims become
conservative queued blockers until released or expired. The importer never removes its source.

## Storage and privacy

State lives at `$XDG_STATE_HOME/ai-coord/state.db`, defaulting to `~/.local/state/ai-coord/state.db`. Set
`AI_COORD_STATE_DIR` to isolate tests or an alternate installation. The directory is mode `0700` and the database is
mode `0600`; SQLite uses WAL, foreign keys, and atomic immediate transactions. The internal schema is currently v6;
opening an older ledger upgrades it one way, while the public `status --json` schema remains v1.

The ledger stores bounded session metadata, callsigns, labels, literal scopes, messages, and notes. It never stores
prompt bodies, plan bodies beyond a sanitized H1, assistant output, transcript contents, or arbitrary hook payloads.

Messages expire after 48 hours and are capped at 50 per inbox; notes expire after seven days. Session processes are
identified by PID and creation time when available, so PID reuse cannot attach ancestry or orphan cleanup to a newer
process. Codex sessions whose exact recorded process is confirmed gone expire after a 30-minute grace period; records
migrated without creation times retain conservative PID-only matching. Other idle Codex sessions expire after four
hours.

## Development

```sh
cd cli
uv sync --extra dev --locked
uv run ai-coord --help
cd ..
just check
just install-cli
```

Validation runs locally on macOS. `just check` runs formatting, linting, type checks, and the full test suite. The root
`justfile` also provides `just full-check`, `just full-write`, `just test`, `just prettier-check`, and
`just prettier-write`.

## Dashboard

The dashboard shows the machine-wide live coordination snapshot: sessions and claims grouped by repository, plus
messages and notes. Run both local servers from the repository root:

```sh
just dev
```

Or run the API and Vite server separately:

```sh
cd cli && uv run ai-coord serve
cd dashboard && bun run dev
```

`ai-coord serve` listens on `127.0.0.1:4477` by default. Vite proxies `/api` requests to that address, so the dashboard
development server can use its own origin.
