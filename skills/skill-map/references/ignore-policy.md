# Ignore Policy

`skill-map` scans broad local roots, so the default ignore policy removes high-volume and false-positive-heavy
directories before searching references.

## Always Ignored

- VCS and dependency directories: `.git`, `node_modules`, `vendor`, `.venv`, `target`.
- Build outputs: `dist`, `build`, `out`, `.next`, `coverage`.
- Large or binary-ish local state: caches, logs, SQLite state, generated images, and temporary directories under known
  agent homes.
- macOS protected home paths: `~/Library` and `~/.Trash`.
- Agent home install/state roots during broad scans: `~/.agents`, `~/.claude`, `~/.codex`, and `~/.local/state/skills`.
- Known local skill catalog source checkouts during broad scans: `~/projects/agent-skills`, `~/sablier/sablier-skills`,
  and `~/sablier/agent-skills`.

## macOS Protected Paths

Broad home-directory scans ignore `~/Library` and `~/.Trash` because macOS privacy protections can make ripgrep return
`2` after producing partial results. Scoped explicit roots are the reliable way to cover additional local content
without treating protected-path failures as successful scans.

## Agent Home Install Roots

Broad scans ignore the agent home roots themselves — `~/.agents`, `~/.claude`, `~/.codex`, and `~/.local/state/skills` —
not just their state subdirectories. These hold installed skill copies and managed state, so during a default `~` scan
they are noise relative to authored sources and project references. Pass one explicitly as `--root` (e.g.
`--root ~/.agents`) to audit installs there; an explicit root is never self-ignored, matching how `~/Library` and
catalog source checkouts behave.

## Claude Code State

Claude Code documents `~/.claude` as containing authored configuration and application data. Authored skills live under
`.claude/skills/`, but these application-data paths are ignored by default:

- `.claude/projects/`: transcripts, subagent transcripts, spilled tool outputs, and auto memory.
- `.claude/plans/`: plan-mode files.
- `.claude/file-history/`: pre-edit snapshots.
- `.claude/tasks/`, `.claude/debug/`, `.claude/backups/`, `.claude/paste-cache/`, `.claude/image-cache/`,
  `.claude/session-env/`, `.claude/shell-snapshots/`.
- `.claude/history.jsonl`, stats, logs, and legacy state.

## Codex State

Codex documents `CODEX_HOME` as defaulting to `~/.codex`; it stores config, skills, auth, history, logs, caches, and
thread/session state there. Authored skills remain scannable under `.codex/skills/`, but these state paths are ignored
by default:

- `.codex/sessions/`, `.codex/archived_sessions/`, `.codex/threads/`, `.codex/backups/`, and
  `.codex/session_index.jsonl`.
- `.codex/history.jsonl`, logs, SQLite state, cache/tmp directories, shell snapshots, generated images, and backup
  files.

## Name-Based Caution

The helper does not globally ignore every directory named `plans`, `sessions`, or `backups`. Those names are ignored
only under known agent state roots such as `.claude/` and `.codex/`, so project-authored files with those names can
still be scanned.

Known skill catalog source checkouts are different: broad home-directory scans ignore them because installed copies
under `~/.agents`, `~/.claude`, `.agents`, or `.claude` are the actionable skill locations. Explicit `--root` paths
inside a catalog source tree still scan that catalog for repo-local audits.
