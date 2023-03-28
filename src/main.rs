use std::{env, process};
use music_player::config::{make_config};

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = make_config(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    if true {
        if let Err(e) = music_player::run(config) {
            eprintln!("Application error: {e}");
            process::exit(1);
        }
    } else {
        music_player::run(config).expect("");
    }
}