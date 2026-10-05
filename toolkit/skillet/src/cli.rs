use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "ai-skillet", version, about = "Inspect and maintain agent-skill catalogs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

const MAP_AFTER_HELP: &str = "\
Without --root or --portfolio-root, map scans $HOME but skips dependency and build directories, ~/Library and ~/.Trash,
agent homes (~/.agents, ~/.claude, ~/.codex, ~/.local/state/skills), package caches, and known catalog source
checkouts. Pass any of those paths as --root to scan it; --show-skipped lists every exclusion.

Exit status:
  0  success; a --skill filter that matches nothing warns on stderr and prints an empty report
  2  invalid arguments or an unreadable root";

const DOCTOR_AFTER_HELP: &str = "\
Exit status:
  0  clean, or every requested safe fix succeeded and no findings remain
  1  the audit completed and findings remain; treat them as review work, not a crash
  2  invalid arguments or an unreadable environment
  3  a requested safe fix failed without partially rewriting its target";

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Map skills, dependencies, and installed copies.
    #[command(after_help = MAP_AFTER_HELP)]
    Map(MapArgs),
    /// Diagnose catalog metadata and optionally apply safe repairs.
    #[command(after_help = DOCTOR_AFTER_HELP)]
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
pub struct MapArgs {
    /// Root to scan; may be repeated. Defaults to $HOME with broad-scan exclusions.
    #[arg(long, value_name = "PATH", conflicts_with = "portfolio_root")]
    pub root: Vec<PathBuf>,

    /// Scan the Git repository containing PATH plus ~/.agents/skills and ~/.claude/skills.
    #[arg(long, value_name = "PATH", conflicts_with = "root")]
    pub portfolio_root: Option<PathBuf>,

    /// Restrict the map to a skill and its relationships; may be repeated.
    #[arg(long, value_name = "NAME")]
    pub skill: Vec<String>,

    /// Scan known catalog source checkouts during the default $HOME scan.
    #[arg(long)]
    pub include_catalog_sources: bool,

    /// Include self-references in dependency output.
    #[arg(long)]
    pub include_self: bool,

    /// Include the matched reference text in each edge.
    #[arg(long)]
    pub include_snippets: bool,

    /// Include configured ignored files and directories in the report.
    #[arg(long)]
    pub show_skipped: bool,

    /// Output representation.
    #[arg(long, value_enum, default_value_t = MapFormat::Text)]
    pub format: MapFormat,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum MapFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// Structured JSON.
    Json,
    /// Graphviz DOT.
    Dot,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Skill or catalog root to audit; may be repeated. Defaults to the current directory.
    #[arg(long, value_name = "PATH")]
    pub root: Vec<PathBuf>,

    /// Restrict diagnostics and fixes to a skill directory name; may be repeated.
    #[arg(long, value_name = "NAME")]
    pub skill: Vec<String>,

    /// Limit diagnostics to declared skill dependencies.
    #[arg(long, conflicts_with = "fix_safe")]
    pub dependencies_only: bool,

    /// Create a missing agents/openai.yaml or align its policy.allow_implicit_invocation with SKILL.md.
    ///
    /// No other finding is auto-fixed: frontmatter, descriptions, README rows, links, and coordination declarations
    /// stay report-only. Edit those manually, then re-run without --fix-safe.
    #[arg(long, conflicts_with = "dependencies_only")]
    pub fix_safe: bool,

    /// Output representation.
    #[arg(long, value_enum, default_value_t = DoctorFormat::Text)]
    pub format: DoctorFormat,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum DoctorFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// Structured JSON.
    Json,
}
