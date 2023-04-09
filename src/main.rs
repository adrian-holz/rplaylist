use std::process::ExitCode;

use clap::Parser;

use rplaylist::config::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Disable for debugging to show exact panic message
    if true {
        if let Err(e) = rplaylist::run(cli) {
            eprintln!("{e}");
            return ExitCode::from(1)
        }
    } else {
        rplaylist::run(cli).expect("The heck?");
    }
    ExitCode::from(0)
}
