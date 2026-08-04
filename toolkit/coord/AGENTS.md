# ai-coord

`ai-coord` is advisory coordination infrastructure for parallel Codex and Claude Code agents. It is cooperative, not a
security boundary or an OS file lock.

## Packages

- [`cli/`](cli/AGENTS.md) is the Python CLI, hook integration, SQLite ledger, and local dashboard API.
- [`dashboard/`](dashboard/AGENTS.md) is the Bun-managed Vite and React dashboard for the live coordination state.

## Shared workflow

Run repository-wide tasks from the root `justfile`:

- `just check` runs the supported local validation gate; `just full-check` and `just full-write` run checks or fixes
  without tests.
- `just test` runs the CLI test suite, and `just install-cli` performs the global-install acceptance flow.
- `just prettier-check` and `just prettier-write` check or format Markdown, JSON, and dashboard source files.
- `just dev` starts the local API and dashboard development servers together.

Keep modules below 1000 lines and test modules below 2000 lines.

## Upstream documentation

- Codex hooks: <https://developers.openai.com/codex/config-advanced#hooks>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>
