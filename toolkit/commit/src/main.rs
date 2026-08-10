use clap::Parser;

fn main() {
    let cli = ai_commit::cli::Cli::parse();
    if let Err(error) = ai_commit::execute(cli) {
        if !error.message.is_empty() {
            eprintln!("{}", error.message);
        }
        std::process::exit(error.kind.code());
    }
}
