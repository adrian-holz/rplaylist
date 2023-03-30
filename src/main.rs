use std::process;

use clap::Parser;

use rplaylist::config::Cli;

fn main() {
    let cli = Cli::parse();

    // Disable for debugging to show exact panic message
    if true {
        if let Err(e) = rplaylist::run(cli) {
            eprintln!("Application error: {e}");
            process::exit(1);
        }
    } else {
        rplaylist::run(cli).expect("The heck?");
    }
}
