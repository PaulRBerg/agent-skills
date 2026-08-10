//! Library entry points for the `ai-skillet` command-line interface.

pub mod catalog;
pub mod cli;
pub mod dependency;
pub mod diagnostic;
pub mod doctor;
pub mod error;
mod exclusions;
pub mod frontmatter;
pub mod hash;
pub mod map;
pub mod traversal;

use std::ffi::OsString;

use clap::Parser;
use cli::{Cli, Command};
use error::{Error, InvocationError};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RunOutcome {
    exit_code: u8,
}

impl RunOutcome {
    pub const SUCCESS: Self = Self { exit_code: 0 };

    pub const fn with_exit_code(exit_code: u8) -> Self {
        Self { exit_code }
    }

    pub const fn exit_code(self) -> u8 {
        self.exit_code
    }
}

/// Execute a parsed command synchronously.
pub fn run(cli: Cli) -> Result<RunOutcome, Error> {
    match cli.command {
        Command::Map(args) => map::run(args).map(|()| RunOutcome::SUCCESS),
        Command::Doctor(args) => doctor::run(args),
    }
}

/// Parse and execute a command from an argument iterator.
pub fn run_from<I, T>(arguments: I) -> Result<RunOutcome, InvocationError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(arguments).map_err(InvocationError::Arguments)?;
    run(cli).map_err(InvocationError::Operational)
}
