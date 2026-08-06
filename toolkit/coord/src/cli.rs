#![allow(dead_code)]

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
    /// Acquire exact file scopes or record pathless intent.
    Start(StartArgs),
    /// Wait for queued work or another coordination event.
    Wait(WaitArgs),
    /// Release this session's active, queued, or intent work.
    Done,
    /// Print Git blob baselines for this session's active claim.
    Baseline,
    /// Show sessions, claims, provider coverage, and repository notes.
    Status(StatusArgs),
    /// Serve the local dashboard HTTP interface.
    Serve(ServeArgs),
    /// Send bounded peer data to one session or repository peers.
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
pub(crate) struct StartArgs {
    /// Explicitly claim a directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "DIR")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    pub(crate) label: String,

    #[arg(value_name = "PATH")]
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
    #[arg(long, default_value = "127.0.0.1")]
    pub(crate) host: String,

    #[arg(long, default_value_t = 4477, value_parser = clap::value_parser!(u16).range(1..))]
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
    #[arg(long = "ack", value_name = "ID", conflicts_with = "ack_all")]
    pub(crate) message_id: Option<String>,

    /// Acknowledge all pending messages.
    #[arg(long = "ack-all", conflicts_with = "message_id")]
    pub(crate) ack_all: bool,
}

#[derive(Debug, Args)]
pub(crate) struct NoteArgs {
    #[arg(required_unless_present = "note_id", conflicts_with = "note_id")]
    pub(crate) text: Option<String>,

    /// Resolve one repository note.
    #[arg(long = "done", value_name = "ID", required_unless_present = "text", conflicts_with = "text")]
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

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum LinkClient {
    Codex,
    Claude,
    All,
}

#[derive(Debug, Args)]
pub(crate) struct LinkArgs {
    pub(crate) client: LinkClient,

    /// Use one alternate Claude settings file.
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
