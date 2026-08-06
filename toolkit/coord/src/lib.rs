mod claim;
mod cli;
mod coordinator;
mod domain;
mod error;
mod hooks;
mod host;
mod server;
mod state;
mod status;

use std::process::ExitCode;

use clap::Parser;

use crate::{cli::Cli, error::AppError};

pub async fn run() -> ExitCode {
    match execute(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !error.message.is_empty() {
                eprintln!("error: {}", error.message);
            }
            ExitCode::from(error.kind.code())
        }
    }
}

async fn execute(_cli: Cli) -> error::Result<()> {
    Err(AppError::operational("Rust command dispatch is not implemented yet"))
}
