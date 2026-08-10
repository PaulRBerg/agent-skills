use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "ai-commit", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Capture an immutable transaction without changing the shared index.
    Prepare(PrepareArgs),
    /// Commit a previously prepared transaction.
    Commit(CommitArgs),
    /// Push the current named branch without integrating remote changes.
    Push,
    /// Show a transaction or retained receipt.
    Show(TransactionArgs),
    /// Discard a prepared transaction.
    Discard(TransactionArgs),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum DiffMode {
    #[default]
    Summary,
    Full,
}

#[derive(Debug, Args)]
pub struct PrepareArgs {
    /// Snapshot all worktree and index changes.
    #[arg(long, conflicts_with = "staged")]
    pub all: bool,

    /// Snapshot the current index exactly.
    #[arg(long, conflicts_with = "all")]
    pub staged: bool,

    /// Force Natural Language Format.
    #[arg(long, conflicts_with = "conventional")]
    pub natural: bool,

    /// Force Conventional Prefix Format.
    #[arg(long, conflicts_with = "natural")]
    pub conventional: bool,

    /// Include only a summary or the complete prepared diff.
    #[arg(long, value_enum, default_value_t)]
    pub diff: DiffMode,

    /// Apply only baseline-to-worktree changes for path.
    #[arg(long = "exclude-baseline", value_name = "PATH=OID")]
    pub exclude_baselines: Vec<String>,

    /// Do not discover stale-dirt baselines from ai-coord.
    #[arg(long)]
    pub no_auto_baseline: bool,

    /// Emit stable tab-separated records.
    #[arg(long)]
    pub porcelain: bool,

    /// Explicit intended paths. Place them after `--`.
    #[arg(last = true)]
    pub paths: Vec<String>,
}

#[derive(Debug, Args)]
pub struct CommitArgs {
    pub transaction_id: String,

    /// Commit message paragraph; use literal newlines and repeat for a body.
    #[arg(short = 'm', long = "message", required = true, allow_hyphen_values = true)]
    pub messages: Vec<String>,

    /// Push after creating or recovering the commit.
    #[arg(long)]
    pub push: bool,

    /// Bypass pre-commit and commit-msg hooks.
    #[arg(long)]
    pub no_verify: bool,

    /// Disable commit signing for this transaction attempt.
    #[arg(long)]
    pub no_gpg_sign: bool,
}

#[derive(Debug, Args)]
pub struct TransactionArgs {
    pub transaction_id: String,
}
