use std::{fmt, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "ai-handoff", version, about = "Create and archive agent task handoffs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create and publish a task handoff.
    Create(CreateArgs),
    /// Archive a completed task handoff.
    Archive(ArchiveArgs),
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Validate placement without reading a draft or writing files.
    #[arg(long)]
    pub check: bool,

    /// Include a Git worktree; repeat for cross-repository handoffs.
    #[arg(long, required = true, value_name = "DIR")]
    pub repo: Vec<PathBuf>,

    /// Launch Codex in this involved repository.
    #[arg(long, value_name = "DIR")]
    pub launch_repo: Option<PathBuf>,

    /// Categorize the requested work.
    #[arg(long, value_enum)]
    pub category: Category,

    /// Describe the task in one concise line.
    #[arg(long)]
    pub task: String,

    /// Read the handoff body from this Markdown draft.
    #[arg(long, required_unless_present = "check", value_name = "BODY.md")]
    pub draft: Option<PathBuf>,

    /// Do not copy and verify the Codex command.
    #[arg(long)]
    pub no_clipboard: bool,

    /// Name the published handoff.
    #[arg(value_name = "FILENAME.md")]
    pub filename: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Category {
    Implementation,
    Investigation,
    Research,
    Audit,
    Operations,
}

impl fmt::Display for Category {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Implementation => "implementation",
            Self::Investigation => "investigation",
            Self::Research => "research",
            Self::Audit => "audit",
            Self::Operations => "operations",
        })
    }
}

#[derive(Debug, Args)]
pub struct ArchiveArgs {
    /// Handoff file to archive.
    pub handoff_path: PathBuf,
}
