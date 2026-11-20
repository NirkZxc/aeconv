mod cli;
mod codecs;
mod engine;
mod errors;
mod formats;
mod metadata;
mod output;
mod processing;

use clap::Parser;
use cli::{Cli, run};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e.user_message());
        std::process::exit(e.exit_code());
    }
}
