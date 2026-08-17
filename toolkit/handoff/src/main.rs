#![deny(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = match ai_handoff::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = u8::try_from(error.exit_code()).unwrap_or(2);
            let _ = error.print();
            return ExitCode::from(code);
        }
    };

    match ai_handoff::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ai-handoff: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
