pub mod cli;
mod commit;
mod config;
mod error;
mod git;
mod prepare;
mod push;
mod rules;
mod state;
mod transactions;

use cli::{Cli, Command};
use error::Result;
pub use error::{AppError, ErrorKind};
use push::PushOutcome;
use state::Store;

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Prepare(args) => {
            let store = state_store()?;
            prepare::run(args, &store)
        }
        Command::Commit(args) => {
            let store = state_store()?;
            commit::run(args, &store)
        }
        Command::Push => {
            let repository = git::Repository::discover()?;
            match push::execute(&repository)? {
                outcome @ PushOutcome::Behind { .. } => {
                    outcome.print();
                    Err(AppError::retry(""))
                }
                outcome => {
                    outcome.print();
                    Ok(())
                }
            }
        }
        Command::Show(args) => {
            let store = state_store()?;
            transactions::show(&store, &args.transaction_id)
        }
        Command::Discard(args) => {
            let store = state_store()?;
            transactions::discard(&store, &args.transaction_id)
        }
    }
}

fn state_store() -> Result<Store> {
    let store = Store::discover()?;
    store.cleanup_receipts()?;
    Ok(store)
}
