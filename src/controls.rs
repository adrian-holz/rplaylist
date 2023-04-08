use std::{io, thread};
use std::io::{Stdout, Write};
use std::path::PathBuf;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use rodio::Sink;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use crate::{audio, file};
use crate::playlist::Playlist;

pub struct PlayState {
    pub save_path: Option<PathBuf>,
    pub playlist: Playlist,
    pub song_idx: usize,
    stopping: bool,
}

impl PlayState {
    pub fn new(save_path: Option<PathBuf>, playlist: Playlist) -> Self {
        PlayState { save_path, playlist, song_idx: 0, stopping: false }
    }
    pub fn stopped(&self) -> bool {
        self.stopping
    }
}

pub enum ControlMessage {
    StreamDone,
    StreamUpdate,
    KeyInput(Key),
}

pub fn start(sink: &Arc<Sink>, state: &Arc<Mutex<PlayState>>) -> (JoinHandle<()>, Sender<ControlMessage>) {
    let sink = sink.clone();
    let state = state.clone();
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        run(&sink, &state, rx);
    });

    let tx2 = tx.clone();
    thread::spawn(move || {
        read_keys(tx2);
    });

    (handle, tx)
}


pub fn run(sink: &Sink, state: &Mutex<PlayState>, rx: Receiver<ControlMessage>) {
    //setting up stdout and going into raw mode
    let mut stdout = io::stdout().into_raw_mode().unwrap();

    print_help(&mut stdout).unwrap();

    for c in rx.into_iter() {
        //clearing the screen and going to top left corner
        write!(stdout,
               "{}{}",
               termion::cursor::Goto(1, 1),
               termion::clear::All
        ).unwrap();

        match c {
            ControlMessage::StreamDone => break,
            ControlMessage::KeyInput(k) => eval_keys(sink, state, &mut stdout, k),
            ControlMessage::StreamUpdate => {
                let state = state.lock().unwrap();
                let index = state.song_idx;
                println!("Now playing {}", state.playlist.song(index).unwrap())
            }
        }

        stdout.flush().unwrap();
    }
}

fn eval_keys(sink: &Sink, state: &Mutex<PlayState>, stdout: &mut RawTerminal<Stdout>, key: Key) {
    match key {
        Key::Char('q') => {
            let mut state = state.lock().unwrap();
            state.stopping = true;
            sink.skip_one(); // We know that there is always only one sound queued
        }
        Key::Char('h') => print_help(stdout).unwrap(),
        Key::Char(' ') => if sink.is_paused() { sink.play() } else { sink.pause() },
        Key::Up => adjust_volume(sink, &mut state.lock().unwrap(), true),
        Key::Down => adjust_volume(sink, &mut state.lock().unwrap(), false),
        Key::Char('i') => println!("{}", state.lock().unwrap().song_idx),
        Key::Char('s') => {
            let state = state.lock().unwrap();
            if let Some(path) = &state.save_path {
                if let Err(e) = file::save_playlist(&state.playlist, path) {
                    println!("Unable to save to {:?}, error: {}", &state.save_path, e)
                }
            } else {
                println!("Unable to save: Direct play mode.")
            }
        }
        _ => (),
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

fn read_keys(rx: Sender<ControlMessage>) {
    let stdin = io::stdin();

    //detecting keydown events
    for c in stdin.keys() {
        match c {
            Ok(k) => rx.send(ControlMessage::KeyInput(k)).unwrap(),
            Err(e) => println!("{}", e), // TODO: better error handling
        }
    }
}