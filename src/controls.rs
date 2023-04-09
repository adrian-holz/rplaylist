use std::{io, thread};
use std::error::Error;
use std::io::Stdout;
use std::path::PathBuf;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use crossterm::{ExecutableCommand, style::Print, terminal};
use crossterm::cursor::MoveToColumn;
use crossterm::event::{Event, KeyCode, KeyEvent, read};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use rodio::Sink;

use crate::{audio, file};
use crate::playlist::Playlist;

pub struct PlayState {
    pub save_path: Option<PathBuf>,
    pub playlist: Playlist,
    pub song_idx: usize,
    stopping: bool,
    pub control_error: bool,
}

impl PlayState {
    pub fn new(save_path: Option<PathBuf>, playlist: Playlist) -> Self {
        PlayState { save_path, playlist, song_idx: 0, stopping: false, control_error: false }
    }
    pub fn stopped(&self) -> bool {
        self.stopping
    }
}

pub enum ControlMessage {
    StreamDone,
    StreamUpdate,
    InputEvent(Event),
    StreamError(String),
}

pub fn start(sink: &Arc<Sink>, state: &Arc<Mutex<PlayState>>) -> (JoinHandle<()>, Sender<ControlMessage>) {
    let sink2 = sink.clone();
    let state2 = state.clone();
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        run(&sink2, &state2, rx);
    });

    let sink2 = sink.clone();
    let state2 = state.clone();
    let tx2 = tx.clone();
    thread::spawn(move || {
        read_keys(tx2);
        abort_playback(&sink2, &state2);
    });

    (handle, tx)
}

///Error occurred, stop program
fn abort_playback(sink: &Sink, state: &Mutex<PlayState>) {
    {
        state.lock().unwrap().control_error = true;
    }
    stop_playback(sink, state);
}

/// Stop program for whatever reason
fn stop_playback(sink: &Sink, state: &Mutex<PlayState>) {
    let mut state = state.lock().unwrap();
    state.stopping = true;
    sink.skip_one(); // We know that there is always only one sound queued
}

fn run(sink: &Sink, state: &Mutex<PlayState>, rx: Receiver<ControlMessage>) {
    //setting up stdout and going into raw mode
    let mut stdout = io::stdout();
    if let Err(e) = terminal::enable_raw_mode() {
        eprintln!("Error enabling raw mode: {e}");
        abort_playback(sink, state);
        return;
    }

    let result = control_loop(sink, state, rx, &mut stdout);

    terminal::disable_raw_mode().unwrap();
    stdout
        .execute(Print("\n")).unwrap()
        .execute(MoveToColumn(0)).unwrap();

    if let Err(e) = result {
        abort_playback(sink, state);
        eprintln!("Unexpected error: {e}")
    }
}

fn control_loop(sink: &Sink, state: &Mutex<PlayState>, rx: Receiver<ControlMessage>, stdout: &mut Stdout) -> Result<(), Box<dyn Error>> {
    print_help(stdout)?;

    for c in rx.into_iter() {
        match c {
            ControlMessage::StreamDone => break,
            ControlMessage::InputEvent(e) => {
                if let Event::Key(event) = e {
                    eval_key(sink, state, stdout, event)?
                }
            }
            ControlMessage::StreamUpdate => {
                let state = state.lock().unwrap();
                let index = state.song_idx;
                display_text(format!("Now playing {}", state.playlist.song(index).unwrap()).as_str(), stdout)?;
            }
            ControlMessage::StreamError(e) => { display_error(e.as_str(), stdout)?; }
        }
    }
    Ok(())
}

fn eval_key(sink: &Sink, state: &Mutex<PlayState>, stdout: &mut Stdout, event: KeyEvent) -> Result<(), Box<dyn Error>> {
    match event.code {
        KeyCode::Char('q') => stop_playback(sink, state),
        KeyCode::Char('h') => { print_help(stdout)?; }
        KeyCode::Char(' ') => if sink.is_paused() { sink.play() } else { sink.pause() },
        KeyCode::Up => {
            adjust_volume(sink, &mut state.lock().unwrap(), stdout, true)?;
        }
        KeyCode::Down => {
            adjust_volume(sink, &mut state.lock().unwrap(), stdout, false)?;
        }
        KeyCode::Char('i') => {
            display_text(format!("{}", state.lock().unwrap().song_idx).as_str(), stdout)?;
        }
        KeyCode::Char('s') => save(state, stdout)?,
        _ => (),
    }

    Ok(())
}

fn save(state: &Mutex<PlayState>, stdout: &mut Stdout) -> Result<(), Box<dyn Error>> {
    let state = state.lock().unwrap();
    if let Some(path) = &state.save_path {
        match file::save_playlist(&state.playlist, path) {
            Err(e) => {
                display_error(format!("Unable to save to {:?}, error: {}", &state.save_path.clone().unwrap(), e).as_str(), stdout)?;
            }
            Ok(_) => {
                display_text(format!("Successfully saved to {:?}", &state.save_path.clone().unwrap()).as_str(), stdout)?;
            }
        }
    } else {
        display_error("Unable to save: Direct play mode.", stdout)?;
    }
    Ok(())
}


///Printing help message
fn print_help(stdout: &mut Stdout) -> crossterm::Result<&mut Stdout> {
    display_text("Exit: q, Help: h, Play/Pause: space, Volume: up/down, Save: s, Debug info: i", stdout)
}

fn display_text<'a>(text: &str, stdout: &'a mut Stdout) -> crossterm::Result<&'a mut Stdout> {
    stdout
        .execute(Print("\n"))?
        .execute(MoveToColumn(0))?
        .execute(Print(text))
}

fn display_error<'a>(text: &str, stdout: &'a mut Stdout) -> crossterm::Result<&'a mut Stdout> {
    stdout
        .execute(SetForegroundColor(Color::Red))?
        .execute(Print("\n"))?
        .execute(MoveToColumn(0))?
        .execute(Print(text))?
        .execute(ResetColor)
}

///Not up means down
fn adjust_volume(sink: &Sink, state: &mut PlayState, stdout: &mut Stdout, up: bool) -> Result<(), Box<dyn Error>> {
    let song = state.playlist.song_mut(state.song_idx).unwrap();
    song.config.volume = calc_new_volume(song.config.volume, up);
    display_text(format!("Volume {:.0}%", song.config.volume * 100.0).as_str(), stdout)?;

    let song = state.playlist.song(state.song_idx).unwrap();
    audio::config_sink(sink, &song.config, &state.playlist.config);
    Ok(())
}

fn calc_new_volume(mut vol: f32, up: bool) -> f32 {
    let ratio = 0.1;
    let min_vol = 0.05;
    let max_vol = 3.0;
    if up {
        vol /= 1.0 - ratio;
        if vol > max_vol { vol = max_vol }
    } else {
        vol *= 1.0 - ratio;
        if vol < min_vol { vol = min_vol }
    }
    vol
}

fn read_keys(rx: Sender<ControlMessage>) {
    loop {
        match read() {
            Ok(e) => rx.send(ControlMessage::InputEvent(e)).unwrap(),
            Err(e) => {
                eprintln!("Error reading input: {e}");
                return;
            }
        }
    }
}