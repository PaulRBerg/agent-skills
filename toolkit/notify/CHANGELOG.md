# Changelog

## [1.0.0] - 2025-01-22

### Added

#### Core Features

- Initial standalone release of `ai-notify`
- Desktop notification system for Claude Code via macOS terminal-notifier
- Smart notification filtering (duration threshold-based, default 10s)
- SQLite session tracking with auto-incrementing job numbers
- YAML-based configuration with Pydantic validation at `~/.config/ai-notify/config.yaml`
- Auto-cleanup running every 24 hours with configurable retention (default 30 days)
- JSON export of sessions before cleanup for backup

#### Event Handlers

- `user-prompt-submit` handler: Tracks new prompt submissions with session ID, prompt text, and working directory
- `stop` handler: Marks sessions complete, calculates duration, sends notifications if threshold exceeded
- `notification` handler: Suppresses "waiting for input" notifications
- `permission-request` handler: Sends immediate notifications for permission requests with tool/command details

#### Configuration

- `config show`: Display current configuration in formatted table
- `config edit`: Open config in $EDITOR with validation
- `config reset`: Reset configuration to defaults with confirmation
- Notification threshold, sound, and app bundle focus customization
- Configurable database path and log settings
- Retention days, auto-cleanup toggle, and export options

#### Database & Storage

- Sessions table storing session_id, prompt, cwd, job_number, timestamps, duration
- Database trigger for auto-incrementing job numbers per session
- Indexed queries on session_id and created_at for performance
- Cleanup with optional JSON export and VACUUM for space reclamation
- Cleanup statistics (rows deleted and space freed)

#### CLI Commands

- `ai-notify test`: Send test notification for validation
- `ai-notify cleanup`: Manual data cleanup with options:
  - `--days`: Custom retention period
  - `--dry-run`: Preview cleanup without deletion
  - `--no-export`: Skip data export before cleanup
- `ai-notify event <handler>`: Event handler subcommands for Claude Code hooks
- `--version`: Display version information

#### Notification Features

- Job completion notifications showing project name, job number, and duration
- Permission request notifications with request details
- Custom Claude icon support in notifications
- Configurable macOS notification sounds
- Click-to-focus: Clicking notification focuses configured app bundle
- Platform detection with helpful error messages for non-macOS systems
- Graceful fallback when terminal-notifier is unavailable

#### Utilities

- Human-readable duration formatting (e.g., "6m53s", "1h1m")
- File-based logging with rotation (10MB max, 5 files retained)
- JSON input validation for Claude Code hook data
- Project name extraction from current working directory
- Session ID validation and sanitization

[1.0.0]: https://github.com/PaulRBerg/ai-notify/releases/tag/v1.0.0
