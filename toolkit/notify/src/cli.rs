use std::{
    env,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, ExitCode},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use rusqlite::Connection;

use crate::{
    config::{self, AppConfig, ConfigLoader, ConfigSource},
    error::{AppError, Result},
    events,
    integrations::{
        CODEX_NOTIFY_COMMAND, IntegrationStatus, ensure_claude_hooks, inspect_claude_hooks, inspect_codex_notify,
        set_codex_notify,
    },
    logging,
    model::Client,
    notifier::{MacNotifier, NotificationSink, completion_notification},
    state::SessionStore,
};

#[derive(Debug, Parser)]
#[command(name = "ai-notify", version, about = "Notification hook for Claude Code and Codex CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage ai-notify configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Test the notification system.
    Test,
    /// Handle a Codex CLI notification callback.
    Codex {
        /// Read the JSON payload from stdin.
        #[arg(long)]
        stdin: bool,
        /// JSON callback payload (normally supplied by Codex as the final argument).
        payload: Option<String>,
    },
    /// Link ai-notify to a supported CLI.
    Link {
        #[command(subcommand)]
        command: LinkCommand,
    },
    /// Check Claude Code and Codex CLI integrations.
    Check {
        /// Inspect a Codex profile overlay.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Clean up old session data.
    Cleanup {
        /// Days of data to retain (defaults to the configured value).
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        days: Option<u32>,
        /// Report what would be removed without deleting anything.
        #[arg(long)]
        dry_run: bool,
        /// Skip the configured pre-cleanup export.
        #[arg(long)]
        no_export: bool,
    },
    /// Handle a Claude Code hook event.
    Event {
        #[command(subcommand)]
        event: EventCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the effective configuration.
    Show {
        /// Read an existing custom configuration file.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Edit the configuration file with $EDITOR.
    Edit {
        /// Edit a custom configuration file.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Reset the configuration to defaults.
    Reset {
        /// Reset a custom configuration file.
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum LinkCommand {
    /// Install ai-notify hooks in Claude Code settings.
    Claude {
        /// Claude Code settings.json path (defaults to ~/.claude/settings.json).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Replace a conflicting non-list hook entry.
        #[arg(long)]
        force: bool,
        /// Show the update without writing it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Configure Codex CLI's notify callback.
    Codex {
        /// Codex config.toml path (defaults to ~/.codex/config.toml).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Codex profile name.
        #[arg(long)]
        profile: Option<String>,
        /// Replace a different existing notify command.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, Subcommand)]
pub enum EventCommand {
    UserPromptSubmit,
    Stop,
    StopFailure,
    Notification,
    PermissionRequest,
    AskUserQuestion,
}

/// Parse process arguments, run one command, and map errors to the public exit contract.
pub fn entrypoint() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = u8::try_from(error.exit_code()).unwrap_or(2);
            let _ = error.print();
            return ExitCode::from(code);
        }
    };

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::from(error.kind.code())
        }
    }
}

/// Execute a parsed command. Keeping parsing separate makes the Clap surface reusable in tests.
pub fn run(cli: Cli) -> Result<()> {
    let loader = loader_for(&cli.command)?;
    let loaded = loader.load_report();
    let _logging_guard = logging::init_loaded(&loaded)?;
    execute(cli.command, &loader, Arc::clone(&loaded.config))
}

fn loader_for(command: &Command) -> Result<ConfigLoader> {
    let path = match command {
        Command::Config { command: ConfigCommand::Show { path: Some(path) } } => {
            if !path.exists() {
                return Err(AppError::usage(format!("configuration file does not exist: {}", path.display())));
            }
            path.clone()
        }
        Command::Config { command: ConfigCommand::Edit { path: Some(path) } } |
        Command::Config { command: ConfigCommand::Reset { path: Some(path) } } => path.clone(),
        _ => config::config_path(),
    };
    Ok(ConfigLoader::new(path))
}

fn execute(command: Command, loader: &ConfigLoader, config: Arc<AppConfig>) -> Result<()> {
    match command {
        Command::Config { command } => run_config(command, loader, &config),
        Command::Test => test_notification(&config),
        Command::Codex { stdin, payload } => run_codex(stdin, payload, &config),
        Command::Link { command } => run_link(command),
        Command::Check { profile } => check_integrations(profile.as_deref()),
        Command::Cleanup { days, dry_run, no_export } => cleanup(days, dry_run, no_export, &config),
        Command::Event { event } => run_event(event, &config),
    }
}

fn run_config(command: ConfigCommand, loader: &ConfigLoader, config: &AppConfig) -> Result<()> {
    match command {
        ConfigCommand::Show { .. } => {
            println!("Current Configuration:");
            println!("  App Bundle: {}", config.notification.app_bundle);
            println!(
                "  Exclude Patterns: {}",
                if config.notification.exclude_patterns.is_empty() {
                    "None".to_owned()
                } else {
                    config.notification.exclude_patterns.join(", ")
                }
            );
            println!("  Notification Mode: {}", config.notification.mode);
            println!("  Notification Sound: {}", config.notification.sound);
            println!("  Notification Threshold: {}s", config.notification.threshold_seconds);
            println!("  Database Path: {}", display_path(&config.database.path));
            println!("  Retention Days: {} days", config.cleanup.retention_days);
            println!("  Auto-cleanup Enabled: {}", if config.cleanup.auto_cleanup_enabled { "Yes" } else { "No" });
            println!("  Export Before Cleanup: {}", if config.cleanup.export_before_cleanup { "Yes" } else { "No" });
            println!("  Log Level: {}", config.logging.level);
            println!("  Log Path: {}", display_path(&config.logging.path));
            println!("\nConfig file: {}", display_path(loader.path()));
            if loader.load_report().source == ConfigSource::Defaults && !loader.path().exists() {
                println!("\nNote: Using default configuration (no config file found)");
            }
            Ok(())
        }
        ConfigCommand::Edit { .. } => {
            if !loader.path().exists() {
                println!("Creating new config file at {}...", display_path(loader.path()));
                loader.save(config)?;
            }
            let editor = env::var_os("EDITOR").filter(|value| !value.is_empty()).unwrap_or_else(|| "vi".into());
            println!("Opening {} in {}...", display_path(loader.path()), editor.to_string_lossy());
            let result = ProcessCommand::new(&editor).arg(loader.path()).status().map_err(|error| {
                AppError::operational(format!("failed to start editor {}: {error}", editor.to_string_lossy()))
            })?;
            if !result.success() {
                return Err(AppError::operational(format!("editor exited with status {result}")));
            }
            let report = ConfigLoader::new(loader.path()).load_report();
            if let Some(warning) = report.warning {
                println!("Warning: Configuration validation failed: {warning}");
            } else {
                println!("Configuration is valid");
            }
            Ok(())
        }
        ConfigCommand::Reset { .. } => {
            if !confirm("Are you sure you want to reset configuration to defaults?")? {
                return Err(AppError::operational("Aborted"));
            }
            loader.reset_to_defaults()?;
            println!("Configuration reset to defaults: {}", display_path(loader.path()));
            Ok(())
        }
    }
}

fn test_notification(config: &AppConfig) -> Result<()> {
    println!("Sending test notification...");
    let mut notifier = MacNotifier::new(config.clone());
    let notification = completion_notification(
        Client::Claude,
        "ai-notify-test",
        "Improve notification messages",
        "Notification content is clearer and more useful.",
        Some("1m23s"),
    );
    if notifier.deliver(&notification) {
        println!("Test notification sent successfully");
    } else {
        println!("Test notification could not be delivered; terminal-notifier is unavailable on this system");
    }
    Ok(())
}

fn run_codex(use_stdin: bool, payload: Option<String>, config: &AppConfig) -> Result<()> {
    let payload = if use_stdin {
        read_stdin()?
    } else {
        payload.ok_or_else(|| AppError::usage("Missing JSON payload (use --stdin or pass as argument)"))?
    };
    if payload.trim().is_empty() {
        return Err(AppError::usage("Missing JSON payload (use --stdin or pass as argument)"));
    }
    let payload = events::parse_payload(&payload)?;
    let mut notifier = MacNotifier::new(config.clone());
    events::handle_codex_notify(&payload, config, &mut notifier)
}

fn run_link(command: LinkCommand) -> Result<()> {
    match command {
        LinkCommand::Claude { path, force, dry_run } => {
            let path = path.map_or_else(|| home_path(".claude/settings.json"), Ok)?;
            let update = ensure_claude_hooks(&path, force, dry_run)?;
            if update.changed {
                if dry_run {
                    println!("Would update hooks in {}", display_path(&update.path));
                } else {
                    println!("Updated hooks in {}", display_path(&update.path));
                }
            } else {
                println!("Hooks already set in {}", display_path(&update.path));
            }
            if !update.added.is_empty() {
                println!("Added events: {}", update.added.join(", "));
            }
            if !update.updated.is_empty() {
                println!("Updated events: {}", update.updated.join(", "));
            }
            if !update.skipped.is_empty() {
                println!("Skipped existing hooks:");
                for (event, hook) in update.skipped {
                    println!("  - {event}: {hook}");
                }
            }
            let legacy = home_path(".claude/hooks/hooks.json")?;
            if legacy.exists() {
                println!(
                    "Note: {} is no longer read by Claude Code; hooks now live in settings.json",
                    display_path(&legacy)
                );
            }
            Ok(())
        }
        LinkCommand::Codex { path, profile, force } => {
            let path = path.map_or_else(|| home_path(".codex/config.toml"), Ok)?;
            let update = set_codex_notify(&path, CODEX_NOTIFY_COMMAND, profile.as_deref(), force)?;
            let target = profile.as_ref().map_or_else(|| "root config".to_owned(), |name| format!("profile '{name}'"));
            if update.conflict && !force {
                println!("Refusing to replace a different {target} notify command.");
                if let Some(previous) = update.previous_notify {
                    println!("Existing notify: {previous}");
                }
                println!("Re-run with --force to replace it.");
                return Err(AppError::integration("Codex notify conflict"));
            }
            if update.changed {
                println!("Updated {target} notify in {}", display_path(&update.path));
                if let Some(previous) = update.previous_notify {
                    println!("Replaced previous notify: {previous}");
                }
            } else {
                println!("{target} notify already set in {}", display_path(&update.path));
            }
            Ok(())
        }
    }
}

fn check_integrations(profile: Option<&str>) -> Result<()> {
    let claude_root = home_path(".claude")?;
    let codex_root = home_path(".codex")?;
    let project_root = env::current_dir()?;
    let claude = inspect_claude_hooks(&claude_root, &project_root);
    let codex = inspect_codex_notify(&codex_root, profile);

    println!("Integration status:");
    println!("Claude Code hooks: {}", status_label(claude.status));
    if !claude.paths.is_empty() {
        println!("  Contributing configs:");
        for path in &claude.paths {
            println!("    - {}", display_path(path));
        }
    }
    if !claude.missing_events.is_empty() {
        println!("  Missing events: {}", claude.missing_events.join(", "));
    }
    if !claude.errors.is_empty() {
        println!("  Errors:");
        for (path, error) in &claude.errors {
            println!("    - {}: {error}", display_path(path));
        }
    }
    if !claude.ignored_paths.is_empty() {
        println!("  Ignored stale configs:");
        for path in &claude.ignored_paths {
            println!("    - {}", display_path(path));
        }
    }

    let label =
        profile.map_or_else(|| "Codex CLI notify".to_owned(), |name| format!("Codex CLI notify (profile '{name}')"));
    println!("{label}: {}", status_label(codex.status));
    if !codex.paths.is_empty() {
        println!("  Active configs:");
        for path in &codex.paths {
            println!("    - {}", display_path(path));
        }
    }
    if let Some(path) = &codex.path {
        println!("  Effective notify source: {}", display_path(path));
    }
    if let Some(notify) = &codex.notify {
        println!("  notify: {notify}");
    }
    if let Some(error) = &codex.error {
        println!("  Error: {error}");
    }

    if claude.has_errors() {
        return Err(AppError::integration("Claude Code settings contain parse or schema errors"));
    }
    if codex.has_error() {
        return Err(AppError::integration("Codex configuration inspection failed"));
    }
    Ok(())
}

fn cleanup(days: Option<u32>, dry_run: bool, no_export: bool, config: &AppConfig) -> Result<()> {
    let retention_days = days.unwrap_or(config.cleanup.retention_days);
    let export_before = !no_export && config.cleanup.export_before_cleanup;
    let store = SessionStore::new(config);

    if dry_run {
        let count = expired_session_count(store.database_path(), retention_days)?;
        println!("DRY RUN MODE - No data will be deleted");
        println!("Retention period: {retention_days} days");
        println!("Sessions to delete: {count}");
        println!("Export before cleanup: {}", if export_before { "Yes" } else { "No" });
        return Ok(());
    }

    println!("Retention period: {retention_days} days");
    println!("Export before cleanup: {}", if export_before { "Yes" } else { "No" });
    if !confirm("Proceed with cleanup?")? {
        println!("Cleanup cancelled");
        return Ok(());
    }
    println!("Running cleanup...");
    let stats = store.cleanup_old_data(retention_days, export_before);
    println!("Cleanup complete:");
    println!("  Sessions deleted: {}", stats.rows_deleted);
    println!("  Space freed: {} KB", stats.space_freed_kb);
    if export_before {
        println!("  Sessions exported: {}", stats.rows_exported);
        if stats.rows_exported > 0 {
            println!("  Exported data saved to: {}", display_path(&config::export_dir()));
        }
    }
    Ok(())
}

fn run_event(event: EventCommand, config: &AppConfig) -> Result<()> {
    let payload = events::parse_payload(&read_stdin()?)?;
    let state = SessionStore::new(config);
    let mut notifier = MacNotifier::new(config.clone());
    match event {
        EventCommand::UserPromptSubmit => events::handle_user_prompt(&payload, &state),
        EventCommand::Stop => events::handle_stop(&payload, &state, config, &mut notifier),
        EventCommand::StopFailure => events::handle_stop_failure(&payload, &state, config, &mut notifier),
        EventCommand::Notification => events::handle_notification(&payload, &state),
        EventCommand::PermissionRequest => events::handle_permission(&payload, &state, config, &mut notifier),
        EventCommand::AskUserQuestion => events::handle_ask_user_question(&payload, &state, config, &mut notifier),
    }
}

fn read_stdin() -> Result<String> {
    let mut payload = String::new();
    io::stdin().read_to_string(&mut payload)?;
    Ok(payload)
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} [y/N]: ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

fn expired_session_count(path: &Path, retention_days: u32) -> Result<i64> {
    let connection = Connection::open(path)
        .map_err(|error| AppError::operational(format!("cannot open session database {}: {error}", path.display())))?;
    let modifier = format!("-{retention_days} days");
    connection
        .query_row("SELECT COUNT(*) FROM sessions WHERE created_at < datetime('now', ?1)", [&modifier], |row| {
            row.get(0)
        })
        .map_err(Into::into)
}

fn status_label(status: IntegrationStatus) -> &'static str {
    match status {
        IntegrationStatus::Missing => "MISSING",
        IntegrationStatus::Partial => "PARTIAL",
        IntegrationStatus::Ok => "OK",
        IntegrationStatus::Error => "ERROR",
    }
}

fn display_path(path: &Path) -> String {
    env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .and_then(|home| path.strip_prefix(Path::new(&home)).ok())
        .map_or_else(
            || path.display().to_string(),
            |relative| {
                if relative.as_os_str().is_empty() { "~".to_owned() } else { format!("~/{}", relative.display()) }
            },
        )
}

fn home_path(relative: &str) -> Result<PathBuf> {
    env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(|home| PathBuf::from(home).join(relative))
        .ok_or_else(|| AppError::configuration("HOME is not set"))
}
