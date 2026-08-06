# ai-coord

Local coordination for parallel Codex and Claude Code agents, shipped as one Rust binary.

`ai-coord` replaces scattered hook scripts and multi-command conflict checks with a three-command lifecycle:

```sh
ai-coord start 'update importer' 'src/importer'
ai-coord wait
ai-coord done
```

The coordinator is cooperative rather than an OS lock. It uses a private local SQLite ledger and fails closed when it
cannot establish complete provider coverage. Unattributed relevant dirt settles for at most ~90 seconds, then work may
proceed with a stale-dirt advisory and a captured baseline.

## Installation

Requirements: Rust (the repository pins its development toolchain in `rust-toolchain.toml`) and Cargo. The dashboard
additionally requires Bun. Automatic Codex hook trust requires Codex CLI 0.146.0 or newer; compatible later versions are
accepted only when the required app-server protocol and trust semantics still validate.

```sh
cargo install --locked --git 'https://github.com/PaulRBerg/ai-coord.git' ai-coord
ai-coord link all
ai-coord check
```

From a source checkout, `just install-cli` builds and installs the release binary, then links the host hooks.

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

Acquire exact file scopes before editing:

```sh
ai-coord start 'regenerate 2025 tax year' \
  'accounting/txs/incomes/2025.tsv' \
  'accounting/reports/2025/tax-summary.md'
```

Positional paths are exact leaves. Claim a directory prefix only when the work really spans an unknown set of files,
using a repeatable `--recursive` option:

```sh
ai-coord start --recursive 'accounting/reports/2025' 'regenerate all 2025 reports'
```

An existing directory passed positionally is rejected with exit 64 before the ledger is opened; re-run it with
`--recursive` or replace it with the actual files. Existing regular files, literal symlink leaves, and nonexistent
planned files are valid positional scopes. Existing files and symlinks are rejected for `--recursive`; a nonexistent
path is accepted there as an explicitly planned subtree. Scopes remain repository-relative and literal. Globs,
non-printable paths, normalized scopes over 120 characters, and paths outside the repository are rejected.

With no paths, `start` records the label as pathless, non-exclusive intent. Intent advertises planned work but owns no
edit scope.

`start` emits one tab-separated result:

| Result                     | Exit | Meaning                                                           |
| -------------------------- | ---: | ----------------------------------------------------------------- |
| `READY`                    |    0 | The claim is active; editing may begin.                           |
| `INTENT`                   |    0 | A pathless, non-exclusive label was recorded.                     |
| `BLOCKED`                  |    3 | The work is queued behind an active or earlier overlapping claim. |
| `UNKNOWN coverage`         |    2 | Provider coverage is incomplete; work was not granted.            |
| `UNKNOWN dirty-settling:…` |    2 | Relevant unattributed dirt is settling; wait and retry.           |
| `ACTIVE`                   |    3 | A requested active-scope expansion failed; the old scope remains. |

Re-running `start` atomically replaces the session's full desired scope. Narrowing an active claim takes effect
immediately and wakes queued sessions that no longer overlap. Expanding or moving an active claim succeeds only when
coverage is complete, relevant dirt is safe, and no active or queued claim intersects the newly requested area;
otherwise `ACTIVE update-…` leaves the old label, paths, age, baselines, and residual ownership unchanged.

Blocked work retains its paths. Narrowing a queued claim preserves its original queue age; expanding or moving it, or
turning pathless intent into a scoped claim, receives a new age so stale broad requests cannot reserve unrelated work.
Waiting therefore needs no repeated session or path arguments:

```sh
ai-coord wait        # waits up to 300 seconds
ai-coord wait -t 60  # explicit timeout, capped at one hour
```

Editing requires `ai-coord start` to return `READY`. `wait` checks the SQLite generation counter each second and
performs full inventory, Git, and arbitration refreshes only when coordination state changes or every 20 seconds as a
fallback. `MESSAGE`, `NOTE`, `RELEASED`, and `TIMEOUT` are non-readiness wakes with exit 3; `UNKNOWN` exits 2. After any
such wake, inspect the reported state and re-arm as needed. `done` idempotently releases active, queued, or intent-only
work and notifies overlapping queued holders that their claim may now be ready.

FIFO applies among intersecting queued scopes; disjoint queued work can proceed independently. A newly blocked claim
reports only the paths that actually overlap. Holder messages do the same and explicitly suggest narrowing when a
recursive holder is blocking a more targeted request; blocked recursive callers receive a matching stderr hint.

In Claude Code, a blocked `ai-coord start` launches a background waker that wakes the session when its claim is
promoted, a message or note arrives, the claim is released, coverage becomes unknown, or the waker times out. A
readiness wake still requires `start` to return `READY`; message and note wakes identify `inbox` or `status` as the
inspection surface and `start` as the ownership recheck. Unknown coverage, timeout, and release state explicitly that no
edit scope is owned. Repeated `start` calls may launch multiple independent wakers for the same session; each exits on
the first terminal outcome. Codex sessions use `ai-coord wait` in the foreground.

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

Benign prefixes never hold.

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

`status` exits 0 for complete coverage, 2 for usable partial coverage, and 1 on error. Its plain-text output marks
queued claims with `claim=queued` and ends with compact, contextual definitions for the states present; `--json` remains
the versioned JSON schema. Status, dashboard snapshots, and message recipient discovery may reuse complete provider
inventory for up to two seconds. `start`, wait promotion, and `check` always probe providers freshly before granting
work or reporting installation health.

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

Lifecycle and nudge hooks invoke `ai-coord hook codex` or `ai-coord hook claude`. Session-start hooks silently register
or refresh idle sessions; Codex limits them to startup, resume, and clear so mid-turn compaction cannot mark working
sessions idle. Prompt hooks inject at most 200 characters of factual state: whether the session is unnamed, plus peer,
queued-claim, and unread-message counts. Naming removes the unnamed fact. Claude's `PostToolBatch` hook and Codex's
`PostToolUse` hook report the unread count once, route inspection to `ai-coord inbox`, and identify message text as
peer-reported data rather than instructions or authority. Peer text, IDs, prompts, and tool payloads are never injected.
Stop and session-end hooks update or release the corresponding session; subagent hooks add read-only parent/child
topology. Claude's `ExitPlanMode` hook records only the approved plan's first H1 as a pathless intent label, while its
filtered `ai-coord waker claude` hook handles blocked starts in the background.

Hook mode is fail-open. Malformed payloads and storage errors never block the host and never expose raw data on stdout.
`ai-coord check` reports hook-health codes and exits 2 for a usable but degraded installation.

### Subagents

Both hosts fire `SubagentStart` and `SubagentStop` with the parent `session_id`. `ai-coord` records delegates under that
parent; it never creates child sessions or claims, and child tool calls refresh the parent session. Coordination is
therefore session-scoped: the parent's claim covers all delegated work. Subagents must never run lifecycle commands
(`start`, `wait`, or `done`) themselves because their inherited identity would make those commands act as the parent.

## Storage and privacy

State lives at `$XDG_STATE_HOME/ai-coord/state.db`, defaulting to `~/.local/state/ai-coord/state.db`. Set
`AI_COORD_STATE_DIR` to isolate tests or an alternate installation. The directory is mode `0700` and the database is
mode `0600`; SQLite uses WAL, foreign keys, and atomic immediate transactions. A fresh database is created directly at
internal schema v9. Any other nonzero schema, including v8, is rejected without migration, import, deletion, or
replacement, while the public `status --json` schema remains v1. Close agents and explicitly choose any backup, removal,
installation, and relinking rollout before retrying with incompatible state.

The ledger stores bounded session metadata, callsigns, labels, literal scopes, messages, notes, and complete provider
health cache rows. Cached provider errors, hook hashes, prompt bodies, plan bodies beyond a sanitized H1, assistant
output, transcript contents, and arbitrary hook payloads are never stored.

Messages expire after 48 hours and are capped at 50 per inbox; notes expire after seven days. On macOS and Linux,
sessions are bound to a kernel-derived process fingerprint containing both PID and process start identity. Normal
`SessionEnd` hooks release immediately; after terminal closure, Ctrl+C, host crash, or another missed hook, the next
fresh coordination probe removes a session as soon as that exact process is confirmed gone. PID reuse is treated as a
different process. An unavailable or ambiguous liveness result fails closed: coverage becomes unknown and the session is
retained. Sessions are never deleted merely because they are old.

## Development

The CLI, hooks, SQLite state, and dashboard API are a single Rust crate; the React dashboard remains a Bun-managed Vite
package. Common source-checkout workflows are:

```sh
cargo test --locked
just check
just install-cli
just dev
```

See [AGENTS.md](AGENTS.md) for architecture, validation, and clean-break rules.

## Dashboard

The dashboard shows the machine-wide live coordination snapshot: sessions and claims grouped by repository, plus
messages and notes. Run both local servers from the repository root:

```sh
just dev
```

Or run the API and Vite server separately:

```sh
ai-coord serve
cd dashboard && bun run dev
```

`ai-coord serve` listens on `127.0.0.1:4477` by default. Vite proxies `/api` requests to that address, so the dashboard
development server can use its own origin.
