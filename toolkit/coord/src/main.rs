use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ai_coord::run().await
}
