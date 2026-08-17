#![deny(unsafe_code)]

mod archive;
mod create;
mod error;
mod git;
mod util;

pub mod cli;

use cli::{Cli, Command};
use error::Result;

/// Execute one parsed command.
pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Create(arguments) => create::run(arguments),
        Command::Archive(arguments) => archive::run(arguments),
    }
}
