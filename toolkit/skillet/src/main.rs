use std::process::ExitCode;

fn main() -> ExitCode {
    match ai_skillet::run_from(std::env::args_os()) {
        Ok(outcome) => ExitCode::from(outcome.exit_code()),
        Err(error) => {
            error.print();
            ExitCode::from(error.exit_code())
        }
    }
}
