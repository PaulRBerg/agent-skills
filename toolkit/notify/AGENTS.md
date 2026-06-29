# Context

Desktop notification system for Claude Code and Codex CLI with intelligent session tracking. macOS only — notifications
go through `terminal-notifier`.

## Layout

`src/ai_notify/` is the package; `tests/` holds per-feature `test_*.py` files. Non-obvious structure:

- `events/` — one handler module per event, including `codex.py`.
- `helpers/` — `cleanup.py`, `filters.py`.
- `claude_hooks.py` / `codex_config.py` / `integrations.py` — back `link claude`, `link codex`, and `check`.

## Development Workflow

### Setup

```bash
uv sync           # Install dependencies into the project venv
just install-cli  # Install the `ai-notify` CLI globally (alias: ic)
```

### Running

The CLI is available via the `ai-notify` command after installation:

```bash
ai-notify --help
ai-notify config show
ai-notify test
ai-notify event user-prompt-submit  # Event handlers
```

### Testing

```bash
just test  # Run pytest
```

### Quality

```bash
just fc    # Run all checks (full-check)
just fw    # Auto-fix all issues (full-write)
```

### Contributing

Run `just fc` and `just test` before pushing or opening a PR; `just test` wraps `uv run pytest`.

## Key Components

### CLI Structure

The CLI uses Click with command groups:

- **Top-level commands**: `test`, `cleanup`, `check`
- **`codex` group**: notify handler (run via `ai-notify codex`)
- **`link` group**: `claude`, `codex`
- **`config` group**: `show`, `edit`, `reset`
- **`event` group**: `user-prompt-submit`, `stop`, `notification`, `permission-request`, `ask-user-question` (Claude
  Code hooks only)

### Event Handlers

Event handlers are CLI subcommands that read JSON from stdin (Claude Code hook format). Codex CLI notify payloads are
handled by the `ai-notify codex` command.

- `ai-notify event user-prompt-submit`: Tracks new prompts
- `ai-notify event stop`: Marks sessions complete, sends notifications
- `ai-notify event notification`: Suppresses waiting notifications
- `ai-notify event permission-request`: Sends permission request notifications
- `ai-notify event ask-user-question`: Notifies when Claude asks a question (PreToolUse/AskUserQuestion)

### Configuration

- **YAML-based**: `~/.config/ai-notify/config.yaml`
- **Pydantic validation**: Type-safe config models
- **Defaults**: Sensible defaults if config doesn't exist

### Database

- **SQLite**: Session tracking with auto-incrementing job numbers
- **Schema**: sessions table with triggers for job numbering
- **Export**: JSON export functionality for backups

### Notifications

- **terminal-notifier**: macOS-native notification system via subprocess
- **Smart filtering**: Only notifies if duration >= threshold
- **Project names**: Extracts project name from cwd
- **Rich features**: Custom Claude icon, configurable sounds, click-to-focus activation
- **Platform check**: Explicit macOS requirement with helpful error messages

## Guidelines

### Code Style

- Use type hints
- Document functions with docstrings
- Follow ruff formatting rules
- Keep line length to 100 characters

### Testing

- Unit tests for individual components
- Integration tests for workflows
- Use `pytest` fixtures for temporary configs
- Mock external dependencies (subprocess.run for terminal-notifier calls)
