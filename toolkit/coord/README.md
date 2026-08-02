# ai-coord

Local coordination for parallel Codex and Claude Code agents.

`ai-coord` replaces scattered hook scripts and multi-command conflict checks with a three-command lifecycle:

```sh
ai-coord start 'update importer' 'src/importer'
ai-coord wait
ai-coord done
```

The coordinator is cooperative rather than an OS lock. It uses a private local SQLite ledger and fails closed when it
cannot establish complete provider coverage or ownership of relevant dirty files.

## Installation

Requirements: Python 3.12+ and [uv](https://docs.astral.sh/uv/).

```sh
uv tool install 'git+https://github.com/PaulRBerg/ai-coord.git'
ai-coord link all
ai-coord check
```

`link` merges owned hooks into `~/.codex/hooks.json` and `~/.claude/settings.json`. It preserves unrelated settings and
hook commands. Use `--dry-run` to preview or `--path` when linking one client to a non-default settings file.

Codex requires interactive review whenever an exact hook definition changes. Open `/hooks` after linking and approve the
`ai-coord hook codex` definition.

## Coordination workflow

Acquire literal file or directory scopes before editing:

```sh
ai-coord start 'regenerate 2025 tax year' \
  'accounting/txs/incomes' \
  'accounting/reports/2025'
```

Scopes are repository-relative prefixes. `.` covers the worktree. Globs and paths outside the repository are rejected so
overlap checks stay exact.

`start` emits one tab-separated result:

| Result             | Exit | Meaning                                                           |
| ------------------ | ---: | ----------------------------------------------------------------- |
| `READY`            |    0 | The claim is active; editing may begin.                           |
| `INTENT`           |    0 | A pathless, non-exclusive label was recorded.                     |
| `BLOCKED`          |    3 | The work is queued behind an active or earlier overlapping claim. |
| `UNKNOWN coverage` |    2 | Provider coverage is incomplete; work was not granted.            |
| `UNKNOWN dirty:…`  |    2 | Relevant dirty files have no attributable live owner.             |
| `ACTIVE`           |    3 | This session already owns a different scope; release it first.    |

Blocked work retains its paths and queue position. Waiting therefore needs no repeated session or path arguments:

```sh
ai-coord wait        # waits up to 300 seconds
ai-coord wait -t 60  # explicit timeout, capped at one hour
```

Only `READY` authorizes editing. A new message or repository note wakes `wait` with exit 3 so the caller can inspect it
and re-arm. `done` idempotently releases active, queued, or intent-only work.

FIFO applies among intersecting queued scopes; disjoint queued work can proceed independently. A newly blocked claim
also sends a bounded system message to its current holders.

## Inventory and communication

```sh
ai-coord status              # current Git worktree
ai-coord status --all        # machine-wide
ai-coord status --json       # versioned JSON schema
ai-coord msg '019fbf24' 'Changes are committed; your path is clear.'
ai-coord inbox
ai-coord inbox --ack '<message-id>'
ai-coord note 'Verified stale importer assumption.'
ai-coord note --done '<note-id>'
```

Message targets accept an exact `client/session`, an exact session ID, a unique ID prefix of at least four characters,
or a unique label/name substring. `repo` expands to the currently live peers in the Git worktree. Messages are private
to their recipients; notes are durable, repo-scoped findings visible to future sessions.

`ai-coord trailer` prints the current Git attribution line:

```text
Agent-Session: codex/019fc27b-b4fb-7322-b65c-ed2471a6fce9
```

## Hooks and health

Both clients invoke one stable command: `ai-coord hook codex` or `ai-coord hook claude`. Prompt hooks update lifecycle
state and return only a capped presence count. Stop and session-end hooks update or release the corresponding session;
subagent hooks add read-only parent/child topology. Claude's `ExitPlanMode` hook records only the approved plan's first
H1 as a pathless intent label.

Hook mode is fail-open. Malformed payloads and storage errors never block the host and never expose raw data on stdout.
`ai-coord check` reports hook-health codes and exits 2 for a usable but degraded installation.

## Legacy migration

Install and test the new CLI before switching hooks:

```sh
ai-coord migrate legacy --dry-run
ai-coord migrate legacy
```

The importer reads the former `~/.codex/.tmp/agent-session-status` registry, claims, messages, and notes. Imports are
content-hash idempotent. Literal claims retain their state; legacy glob claims become conservative queued blockers until
released or expired. The importer never removes its source.

## Storage and privacy

State lives at `$XDG_STATE_HOME/ai-coord/state.db`, defaulting to `~/.local/state/ai-coord/state.db`. Set
`AI_COORD_STATE_DIR` to isolate tests or an alternate installation. The directory is mode `0700` and the database is
mode `0600`; SQLite uses WAL, foreign keys, and atomic immediate transactions.

The ledger stores bounded session metadata, labels, literal scopes, messages, and notes. It never stores prompt bodies,
plan bodies beyond a sanitized H1, assistant output, transcript contents, or arbitrary hook payloads.

Messages expire after 48 hours and are capped at 50 per inbox; notes expire after seven days; idle Codex sessions expire
after four hours.

## Development

```sh
uv sync --extra dev --locked
just test
just full-check
just install-cli
```

CI runs the same checks and tests on Ubuntu with Python 3.13.
