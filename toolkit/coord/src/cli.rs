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
    /// Use `recommend send` when a planned replacement may make a holder's work obsolete.
    Start(StartArgs),
    /// Coordinate one atomic work item spanning multiple Git repositories.
    Bundle(BundleArgs),
    /// Return when queued work is ready or another wake event occurs.
    ///
    /// Messages, unknown coverage, work release, and timeout are
    /// non-readiness wake events.
    Wait(WaitArgs),
    /// Release this session's draft, active, or queued work.
    ///
    /// Run from a worktree claimed by this session. For a multi-repository bundle,
    /// this releases every claim atomically.
    Done(DoneArgs),
    /// Print Git blob baselines for this session's active work.
    Baseline,
    /// Print repository-relative paths written by this session's observed tool calls.
    Touched,
    /// Show sessions, work, provider coverage, and repository findings.
    Status(StatusArgs),
    /// Serve the local dashboard HTTP interface.
    Serve(ServeArgs),
    /// Send bounded peer data to one session or current-repository peers.
    ///
    /// TARGET=repo selects live peers in the current Git worktree.
    Msg(MessageArgs),
    /// List or acknowledge recipient-only messages.
    Inbox(InboxArgs),
    /// Send, inspect, and decide cooperative in-flight work recommendations.
    Recommend(RecommendArgs),
    /// Record and manage durable repository findings.
    Finding(FindingArgs),
    /// Print the current agent-session Git trailer.
    Trailer,
    /// Consume one host lifecycle hook payload from standard input.
    #[command(hide = true)]
    Hook(HookArgs),
    /// Wake a Claude session when queued coordination state changes.
    #[command(hide = true)]
    Waker(WakerArgs),
    /// Run one previously claimed findings triage batch.
    #[command(hide = true)]
    TriageWorker(TriageWorkerArgs),
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
    /// Store this draft under a portable name instead of this session.
    #[arg(long, value_name = "NAME")]
    pub(crate) name: Option<String>,

    /// Explicitly remember a directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "DIR")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    pub(crate) label: String,

    #[arg(value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct StartArgs {
    /// Submit the stored draft for normal arbitration. Bare `--draft` submits this
    /// session's unnamed draft; `--draft NAME` submits the named draft.
    #[arg(
        long,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "",
        conflicts_with_all = ["recursive_paths", "label", "paths"]
    )]
    pub(crate) draft: Option<String>,

    /// Explicitly reserve a directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "DIR", conflicts_with = "draft")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    #[arg(required_unless_present = "draft", conflicts_with = "draft")]
    pub(crate) label: Option<String>,

    #[arg(value_name = "PATH", conflicts_with = "draft")]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct BundleArgs {
    #[command(subcommand)]
    pub(crate) command: BundleCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum BundleCommand {
    /// Store an atomic multi-repository draft without reserving it.
    Draft(BundleDraftArgs),
    /// Acquire or submit an atomic multi-repository work item.
    Start(BundleStartArgs),
}

#[derive(Debug, Args)]
pub(crate) struct BundleDraftArgs {
    /// Store this draft under a portable name instead of this session.
    #[arg(long, value_name = "NAME")]
    pub(crate) name: Option<String>,

    /// Explicitly remember an absolute directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "ABSOLUTE_DIR")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    pub(crate) label: String,

    #[arg(value_name = "ABSOLUTE_PATH")]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct BundleStartArgs {
    /// Submit the stored repository bundle draft for arbitration. Bare `--draft`
    /// submits this session's unnamed draft; `--draft NAME` submits the named draft.
    #[arg(
        long,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "",
        conflicts_with_all = ["recursive_paths", "label", "paths"]
    )]
    pub(crate) draft: Option<String>,

    /// Explicitly reserve an absolute directory prefix; repeat for multiple directories.
    #[arg(long = "recursive", value_name = "ABSOLUTE_DIR", conflicts_with = "draft")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    #[arg(required_unless_present = "draft", conflicts_with = "draft")]
    pub(crate) label: Option<String>,

    #[arg(value_name = "ABSOLUTE_PATH", conflicts_with = "draft")]
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
pub(crate) struct DoneArgs {}

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
pub(crate) struct RecommendArgs {
    #[command(subcommand)]
    pub(crate) command: RecommendCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum RecommendCommand {
    /// Propose that one peer defer or omit a covered part of its submitted work.
    Send(RecommendSendArgs),
    /// List incoming pending recommendations in this worktree, or sent/history records.
    List(RecommendListArgs),
    /// Show one endpoint-authorized recommendation from any directory.
    Show(RecommendShowArgs),
    /// Accept or reject an incoming recommendation with a recorded reason.
    Respond(RecommendRespondArgs),
    /// Withdraw a sent recommendation with a recorded reason.
    Withdraw(RecommendWithdrawArgs),
}

#[derive(Debug, Args)]
pub(crate) struct RecommendSendArgs {
    pub(crate) target: String,

    #[arg(long, value_enum)]
    pub(crate) action: RecommendationActionArg,

    /// Propose one exact repository-relative path; repeat for multiple paths.
    #[arg(long = "path", value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    /// Propose one recursive repository-relative directory; repeat as needed.
    #[arg(long = "recursive", value_name = "DIR")]
    pub(crate) recursive_paths: Vec<PathBuf>,

    #[arg(long)]
    pub(crate) reason: String,

    #[arg(long)]
    pub(crate) replacement: String,
}

#[derive(Debug, Args)]
pub(crate) struct RecommendListArgs {
    /// List recommendations sent by this endpoint instead of incoming recommendations.
    #[arg(long)]
    pub(crate) sent: bool,

    /// Include accepted and terminal recommendation history.
    #[arg(long)]
    pub(crate) all: bool,

    /// Emit the recommendation JSON v1 envelope.
    #[arg(long = "json")]
    pub(crate) as_json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct RecommendShowArgs {
    pub(crate) id: String,

    /// Emit the recommendation JSON v1 envelope.
    #[arg(long = "json")]
    pub(crate) as_json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct RecommendRespondArgs {
    pub(crate) id: String,

    #[arg(long, value_enum)]
    pub(crate) decision: RecommendationDecisionArg,

    #[arg(long)]
    pub(crate) reason: String,
}

#[derive(Debug, Args)]
pub(crate) struct RecommendWithdrawArgs {
    pub(crate) id: String,

    #[arg(long)]
    pub(crate) reason: String,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum RecommendationActionArg {
    Defer,
    Omit,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum RecommendationDecisionArg {
    Accepted,
    Rejected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recommend_send_with_exact_and_recursive_scopes() {
        let cli = Cli::try_parse_from([
            "ai-coord",
            "recommend",
            "send",
            "peer",
            "--action",
            "defer",
            "--path",
            "src/legacy.rs",
            "--recursive",
            "src/generated",
            "--reason",
            "replacement makes this obsolete",
            "--replacement",
            "remove the adapter and revalidate callers",
        ])
        .expect("recommend send parses");
        let Command::Recommend(RecommendArgs { command: RecommendCommand::Send(arguments) }) = cli.command else {
            panic!("expected recommend send");
        };
        assert!(matches!(arguments.action, RecommendationActionArg::Defer));
        assert_eq!(arguments.paths, vec![PathBuf::from("src/legacy.rs")]);
        assert_eq!(arguments.recursive_paths, vec![PathBuf::from("src/generated")]);
    }
}

#[derive(Debug, Args)]
pub(crate) struct FindingArgs {
    #[command(subcommand)]
    pub(crate) command: FindingCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum FindingCommand {
    /// Record a new finding or another sighting of an exact open match.
    Add(FindingAddArgs),
    /// List findings in the current repository.
    List(FindingListArgs),
    /// Show one finding.
    Show(FindingShowArgs),
    /// Mark a pending finding as handed off to an owned path.
    Handoff(FindingHandoffArgs),
    /// Resolve a finding into a terminal state.
    Resolve(FindingResolveArgs),
    /// Return a terminal finding to pending.
    Reopen(FindingReopenArgs),
}

#[derive(Debug, Args)]
pub(crate) struct FindingAddArgs {
    #[arg(long, value_enum)]
    pub(crate) kind: Option<FindingKindArg>,

    #[arg(long = "path", value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    pub(crate) text: String,
}

#[derive(Debug, Args)]
pub(crate) struct FindingListArgs {
    #[arg(long, value_enum, conflicts_with = "all")]
    pub(crate) state: Option<FindingStateArg>,

    /// Include terminal findings.
    #[arg(long)]
    pub(crate) all: bool,

    #[arg(long = "json")]
    pub(crate) as_json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct FindingShowArgs {
    pub(crate) id: String,

    #[arg(long = "json")]
    pub(crate) as_json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct FindingHandoffArgs {
    pub(crate) id: String,

    #[arg(long, value_name = "PATH")]
    pub(crate) path: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct FindingResolveArgs {
    pub(crate) id: String,

    #[arg(long = "as", value_enum)]
    pub(crate) resolution: FindingResolutionArg,

    /// Record a commit object ID as resolution evidence.
    #[arg(long, value_name = "OID")]
    pub(crate) commit: Option<String>,

    /// Identify the canonical finding when resolving as duplicate.
    #[arg(long, value_name = "ID")]
    pub(crate) canonical: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct FindingReopenArgs {
    pub(crate) id: String,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum FindingKindArg {
    Bug,
    Docs,
    Improvement,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum FindingStateArg {
    Pending,
    HandedOff,
    Fixed,
    Stale,
    Rejected,
    Duplicate,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum FindingResolutionArg {
    Fixed,
    Stale,
    Rejected,
    Duplicate,
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

#[derive(Debug, Args)]
pub(crate) struct TriageWorkerArgs {
    #[arg(long)]
    pub(crate) run_id: String,

    #[arg(long)]
    pub(crate) repo: PathBuf,
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
