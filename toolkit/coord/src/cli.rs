use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "ai-coord", version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Assign this session an emoji-bearing callsign.
    Name(NameArgs),
    /// Store exact planned scopes without reserving them.
    Draft(DraftArgs),
    /// Return READY after acquiring exact file PATHS, or queue the work.
    ///
    /// Use --draft to submit the stored draft, or pass LABEL and scopes for a
    /// direct submission. Use --recursive DIR for directory-prefix ownership.
    Start(StartArgs),
    /// Return when queued work is ready or another wake event occurs.
    ///
    /// Messages, notes, unknown coverage, work release, and timeout are
    /// non-readiness wake events.
    Wait(WaitArgs),
    /// Release this session's draft, active, or queued work.
    Done,
    /// Print Git blob baselines for this session's active work.
    Baseline,
    /// Show sessions, work, provider coverage, and repository notes.
    Status(StatusArgs),
    /// Serve the local dashboard HTTP interface.
    Serve(ServeArgs),
    /// Send bounded peer data to one session or current-repository peers.
    ///
    /// TARGET=repo selects live peers in the current Git worktree.
    Msg(MessageArgs),
    /// List or acknowledge recipient-only messages.
    Inbox(InboxArgs),
    /// Create or resolve a durable repository note.
    Note(NoteArgs),
    /// Print the current agent-session Git trailer.
    Trailer,
    /// Consume one host lifecycle hook payload from standard input.
    #[command(hide = true)]
    Hook(HookArgs),
    /// Wake a Claude session when queued coordination state changes.
    #[command(hide = true)]
    Waker(WakerArgs),
    /// Install owned lifecycle hooks while preserving unrelated hooks.
    Link(LinkArgs),
    /// Report installation, schema, hook, provider, and hook-health status.
    Check(CheckArgs),
}

#[derive(Debug, Args)]
pub(crate) struct NameArgs {
    pub(crate) callsign: String,
}

#[derive(Debug, Args)]
pub(crate) struct DraftArgs {
    /// Explicitly remember a directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "DIR")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    pub(crate) label: String,

    #[arg(value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct StartArgs {
    /// Submit the stored draft for normal arbitration.
    #[arg(long, conflicts_with_all = ["recursive_paths", "label", "paths"])]
    pub(crate) draft: bool,

    /// Explicitly reserve a directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "DIR", conflicts_with = "draft")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    #[arg(required_unless_present = "draft", conflicts_with = "draft")]
    pub(crate) label: Option<String>,

    #[arg(value_name = "PATH", conflicts_with = "draft")]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct WaitArgs {
    #[arg(
        short = 't',
        long = "timeout-seconds",
        default_value_t = 300,
        value_parser = clap::value_parser!(u64).range(1..=3600)
    )]
    pub(crate) timeout_seconds: u64,
}

#[derive(Debug, Args)]
pub(crate) struct StatusArgs {
    /// Show machine-wide inventory.
    #[arg(long = "all")]
    pub(crate) machine_wide: bool,

    /// Emit the versioned JSON schema.
    #[arg(long = "json")]
    pub(crate) as_json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ServeArgs {
    #[arg(long, default_value = crate::server::DEFAULT_HOST)]
    pub(crate) host: String,

    #[arg(
        long,
        default_value_t = crate::server::DEFAULT_PORT,
        value_parser = clap::value_parser!(u16).range(1..)
    )]
    pub(crate) port: u16,
}

#[derive(Debug, Args)]
pub(crate) struct MessageArgs {
    pub(crate) target: String,
    pub(crate) text: String,
}

#[derive(Debug, Args)]
pub(crate) struct InboxArgs {
    /// Acknowledge one message ID.
    #[arg(long = "ack", value_name = "ID")]
    pub(crate) message_id: Option<String>,

    /// Acknowledge all pending messages.
    #[arg(long = "ack-all")]
    pub(crate) ack_all: bool,
}

#[derive(Debug, Args)]
pub(crate) struct NoteArgs {
    pub(crate) text: Option<String>,

    /// Resolve one repository note.
    #[arg(long = "done", value_name = "ID")]
    pub(crate) note_id: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum HookClient {
    Codex,
    Claude,
}

#[derive(Debug, Args)]
pub(crate) struct HookArgs {
    pub(crate) client: HookClient,
}

#[derive(Debug, Args)]
pub(crate) struct WakerArgs {
    #[arg(value_enum)]
    pub(crate) client: ClaudeClient,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ClaudeClient {
    Claude,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum LinkClient {
    Codex,
    Claude,
    All,
}

#[derive(Debug, Args)]
pub(crate) struct LinkArgs {
    pub(crate) client: LinkClient,

    /// Codex: active hooks file only; Claude: one alternate settings file.
    #[arg(long, value_name = "PATH")]
    pub(crate) path: Option<PathBuf>,

    /// Inspect changes without writing.
    #[arg(long)]
    pub(crate) dry_run: bool,

    /// Replace malformed owned hook containers.
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Debug, Args)]
pub(crate) struct CheckArgs {
    /// Emit machine-readable diagnostics.
    #[arg(long = "json")]
    pub(crate) as_json: bool,
}
