# Context

Desktop notification system for Claude Code with intelligent session tracking.

## Project Structure

```
ai-notify/
├── src/ai_notify/          # Main package
│   ├── __init__.py        # Package exports and version
│   ├── assets/            # Static assets
│   │   └── claude-icon.png  # Claude icon for notifications
│   ├── cli.py             # Click-based CLI with event handler subcommands
│   ├── config.py          # Configuration constants and Config class
│   ├── config_loader.py   # YAML config loader with Pydantic validation
│   ├── database.py        # SQLite session tracker
│   ├── notifier.py        # Desktop notification handler
│   └── utils.py           # Utility functions and data models
├── tests/                  # Test suite
│   ├── test_ai_notify.py      # Unit tests
│   └── test_integration.py    # Integration tests
├── pyproject.toml         # Project metadata and dependencies
├── justfile               # Development commands
├── ruff.toml              # Ruff configuration
├── pyrightconfig.json     # Pyright type checking
└── README.md              # User documentation
```

## Development Workflow

### Setup

```bash
just install  # Install dependencies with uv
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

## Key Components

### CLI Structure

The CLI uses Click with command groups:

- **Top-level commands**: `test`, `cleanup`
- **`config` group**: `show`, `edit`, `reset`
- **`event` group**: `user-prompt-submit`, `stop`, `notification`, `permission-request`

### Event Handlers

Event handlers are CLI subcommands that read JSON from stdin (Claude Code hook format):

- `ai-notify event user-prompt-submit`: Tracks new prompts
- `ai-notify event stop`: Marks sessions complete, sends notifications
- `ai-notify event notification`: Suppresses waiting notifications
- `ai-notify event permission-request`: Sends permission request notifications

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
