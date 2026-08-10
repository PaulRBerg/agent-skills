use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "ai-skillet", version, about = "Inspect and maintain agent-skill catalogs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Map skills, dependencies, and installed copies.
    Map(MapArgs),
    /// Diagnose catalog metadata and optionally apply safe repairs.
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
pub struct MapArgs {
    /// Catalog root to scan.
    #[arg(long, value_name = "PATH", conflicts_with = "portfolio_root")]
    pub root: Vec<PathBuf>,

    /// Root containing skill portfolios and client installations.
    #[arg(long, value_name = "PATH", conflicts_with = "root")]
    pub portfolio_root: Option<PathBuf>,

    /// Restrict the map to a skill and its relationships.
    #[arg(long, value_name = "NAME")]
    pub skill: Vec<String>,

    /// Include catalog source files in the report.
    #[arg(long)]
    pub include_catalog_sources: bool,

    /// Include the invoking skill in filtered reports.
    #[arg(long)]
    pub include_self: bool,

    /// Include embedded skill snippets in the report.
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
    /// Catalog root to scan.
    #[arg(long, value_name = "PATH")]
    pub root: Vec<PathBuf>,

    /// Limit diagnostics to declared skill dependencies.
    #[arg(long, conflicts_with = "fix_safe")]
    pub dependencies_only: bool,

    /// Apply narrowly scoped, safe metadata repairs.
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
