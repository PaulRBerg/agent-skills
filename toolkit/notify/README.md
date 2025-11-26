# ai-notify

Desktop notification system for Claude Code that intelligently tracks session activity and sends macOS notifications for
key events.

## Features

- **Smart Notifications**: Only notifies for jobs exceeding a configurable duration threshold (default: 10s)
- **Prompt Filtering**: Exclude specific prompt patterns (e.g., slash commands like `/commit`) from notifications
- **Session Tracking**: SQLite database tracks prompts, durations, and job numbers
- **Auto-cleanup**: Automatic data cleanup with optional export before deletion
- **Event Handlers**: CLI subcommands for Claude Code hook integration
- **Configuration**: YAML-based configuration with sensible defaults
- **Rich Notifications**: Custom Claude icon, configurable sounds, and click-to-focus terminal support

## Installation

### Prerequisites

- **macOS only** (terminal-notifier is macOS-specific)
- Python 3.12+
- [uv](https://github.com/astral-sh/uv) package manager
- [terminal-notifier](https://github.com/julienXX/terminal-notifier): Install with `brew install terminal-notifier`

### Install from source

For end users (installs globally for Claude Code hooks):

```bash
# Clone the repository
git clone https://github.com/PaulRBerg/ai-notify.git
cd ai-notify

# Install globally with uv
uv tool install .

# Verify installation
ai-notify --version
```

**Note**: Claude Code hooks require global installation via `uv tool install .` to make the `ai-notify` command
available system-wide.

### Updating an existing installation

If you already have `ai-notify` installed and need to update to a newer version:

```bash
# Pull latest changes
git pull

# Reinstall with rebuild
uv tool install --reinstall-package ai-notify .
```

The `--reinstall-package` flag forces `uv` to rebuild the package from source instead of using a cached version.

## Configuration

ai-notify uses YAML configuration stored at `~/.config/ai-notify/config.yaml`.

### View current configuration

```bash
ai-notify config show
```

### Edit configuration

```bash
ai-notify config edit
```

### Reset to defaults

```bash
ai-notify config reset
```

### Configuration options

```yaml
cleanup:
  auto_cleanup_enabled: true # Enable automatic cleanup of old data
  export_before_cleanup: true # Export data before cleanup
  retention_days: 30 # Number of days to retain session data (older data will be auto-cleaned)

database:
  path: ~/.config/ai-notify/ai-notify.db # Path to SQLite database file

logging:
  level: INFO # Log level (DEBUG, INFO, WARNING, ERROR, CRITICAL)
  path: ~/.config/ai-notify/ai-notify.log # Path to log file

notification:
  app_bundle: dev.warp.Warp-Stable # Application bundle ID to focus on notification click
  mode: all # Notification mode: 'all' (default), 'permission_only', or 'disabled'
  sound: default # Notification sound to use
  threshold_seconds: 10 # Minimum job duration in seconds to trigger notification (0 = notify all)
  exclude_patterns: # List of prompt prefixes to exclude from notifications (case-sensitive)
    - /commit
    - /update-pr
    - /fix-issue
```

**Prompt Pattern Filtering**

The `exclude_patterns` configuration allows you to filter out notifications for specific prompts:

- **Prefix matching**: Patterns match prompts that _start with_ the pattern (case-sensitive)
- **Common use case**: Exclude slash commands like `/commit`, `/update-pr`, etc.
- **Examples**:
  - Pattern `/commit` matches: `/commit`, `/commit --all`, `/commit -m "message"`
  - Pattern `/commit` does NOT match: `Commit changes`, `run /commit`, `/Commit` (different case)
- **Default**: Empty list (no filtering)

## Usage

### Test notifications

```bash
ai-notify test
```

### Clean up old data

```bash
# Preview cleanup (dry run)
ai-notify cleanup --dry-run

# Clean up with confirmation
ai-notify cleanup

# Clean up with custom retention
ai-notify cleanup --days 60

# Clean up without export
ai-notify cleanup --no-export
```

### Event handlers (Claude Code integration)

The following commands are designed to be called from Claude Code hooks:

```bash
# Track user prompt submission
ai-notify event user-prompt-submit < event_data.json

# Handle stop event (sends notifications)
ai-notify event stop < event_data.json

# Handle notification event
ai-notify event notification < event_data.json

# Handle permission request
ai-notify event permission-request < event_data.json
```

## Claude Code Hook Integration

To integrate ai-notify with Claude Code, update your `~/.claude/hooks/hooks.json`.

For more information about Claude Code hooks, see the
[official documentation](https://docs.claude.com/en/docs/claude-code/hooks).

```json
{
  "hooks": {
    "UserPromptSubmit": {
      "command": "ai-notify event user-prompt-submit"
    },
    "Stop": {
      "command": "ai-notify event stop"
    },
    "Notification": {
      "command": "ai-notify event notification"
    },
    "PermissionRequest": {
      "command": "ai-notify event permission-request"
    }
  }
}
```

## How It Works

1. **UserPromptSubmit**: When you submit a prompt to Claude Code, ai-notify tracks it in the database
2. **Stop**: When Claude finishes (or you stop it), ai-notify:
   - Calculates the duration
   - Checks if duration >= threshold
   - Checks if prompt starts with any excluded pattern
   - Sends notification only if both checks pass (by default, whether the job took longer than >10 seconds)
   - Optionally runs auto-cleanup (every 24 hours)
3. **Notification**: Suppresses "waiting for input" notifications (the Stop handler will send job completion)
4. **PermissionRequest**: Sends immediate notification when Claude requests permissions

## File Structure

```
~/.config/ai-notify/
├── config.yaml             # User configuration
├── ai-notify.db            # SQLite session database
├── ai-notify.log           # Application logs
└── exports/                # JSON exports before cleanup
```

## Documentation

- [Claude Code](https://code.claude.com/docs/en/overview)
- [Hooks](https://docs.claude.com/en/docs/claude-code/hooks)

## License

MIT License - see [LICENSE.md](LICENSE.md)
