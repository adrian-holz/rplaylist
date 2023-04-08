use std::io;
use std::io::{Stdout, Write};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Mutex;

use rodio::Sink;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use crate::{audio, file};
use crate::playlist::Playlist;

pub struct PlayState {
    pub path: PathBuf,
    pub playlist: Playlist,
    pub song_idx: usize,
}

pub fn song_controls(sink: &Sink, p: &Mutex<PlayState>) {
    let stdin = io::stdin();
    //setting up stdout and going into raw mode
    let mut stdout = io::stdout().into_raw_mode().unwrap();

    print_help(&mut stdout).unwrap();

    //detecting keydown events
    for c in stdin.keys() {
        //clearing the screen and going to top left corner
        write!(
            stdout,
            "{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All
        )
            .unwrap();

        match c.unwrap() {
            Key::Char('q') => exit(0), // TODO: clean shutdown
            Key::Char('h') => print_help(&mut stdout).unwrap(),
            Key::Char(' ') => if sink.is_paused() { sink.play() } else { sink.pause() },
            Key::Up => adjust_volume(sink, &mut p.lock().unwrap(), true),
            Key::Down => adjust_volume(sink, &mut p.lock().unwrap(), false),
            Key::Char('i') => println!("{}", p.lock().unwrap().song_idx),
            Key::Char('s') => {
                let state = p.lock().unwrap();
                if let Err(e) = file::save_playlist(&state.playlist, &state.path) {
                    println!("Unable to save to {:?}, error: {}", &state.path, e)
                }
            }
            _ => (),
        }

        stdout.flush().unwrap();
    }
}

///Printing help message, clearing the screen and going to left top corner with the cursor
fn print_help(stdout: &mut RawTerminal<Stdout>) -> io::Result<()> {
    write!(stdout,
           r#"{}{}q to exit, h to print this text, space for play/pause, up/down: volume, s: save, i: debug info{}"#,
           termion::cursor::Goto(1, 1), termion::clear::All, termion::cursor::Goto(1, 2))?;
    stdout.flush()
}

///Not up means down
fn adjust_volume(sink: &Sink, state: &mut PlayState, up: bool) {
    let song = state.playlist.song_mut(state.song_idx).unwrap();
    song.config.volume = calc_new_volume(song.config.volume, up);

    let song = state.playlist.song(state.song_idx).unwrap();
    audio::config_sink(sink, &song.config, &state.playlist.config);
}

fn calc_new_volume(mut vol: f32, up: bool) -> f32 {
    let ratio = 0.1;
    let min_vol = 0.05;
    let max_vol = 3.0;
    println!("old: {}", vol);
    if up {
        vol /= 1.0 - ratio;
        if vol > max_vol { vol = max_vol }
    } else {
        vol *= 1.0 - ratio;
        if vol < min_vol { vol = min_vol }
    }
    println!("new: {}", vol);
    vol
}
