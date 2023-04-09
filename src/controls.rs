use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::{io, thread};

use crossterm::cursor::MoveToColumn;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::ClearType;
use crossterm::{style::Print, terminal, ExecutableCommand};
use rodio::Sink;

use crate::playlist::Playlist;
use crate::{audio, file};

pub enum ControlMessage {
    StreamDone,
    StartSong(usize),
    InputEvent(Event),
    StreamError(String),
}

pub struct Playback {
    pub save_path: Option<PathBuf>,
    pub playlist: Playlist,
    stopping: bool,
    pub control_error: bool,
}

impl Playback {
    pub fn new(save_path: Option<PathBuf>, playlist: Playlist) -> Self {
        Playback {
            save_path,
            playlist,
            stopping: false,
            control_error: false,
        }
    }
    pub fn stopped(&self) -> bool {
        self.stopping
    }
}

struct ControlState {
    sink: Arc<Sink>,
    last_out_was_action: bool,
    song_index: usize,
}

impl ControlState {
    fn new(sink: &Arc<Sink>) -> Self {
        Self {
            sink: Arc::clone(sink),
            last_out_was_action: false,
            song_index: 0,
        }
    }
}

pub fn start(
    sink: &Arc<Sink>, playback: &Arc<Mutex<Playback>>,
) -> (JoinHandle<()>, Sender<ControlMessage>) {
    let playback2 = playback.clone();
    let (tx, rx) = mpsc::channel();

    let state = ControlState::new(sink);
    let handle = thread::spawn(move || {
        run(state, &playback2, rx);
    });

    let sink2 = sink.clone();
    let playback2 = playback.clone();
    let tx2 = tx.clone();
    thread::spawn(move || {
        read_keys(tx2);
        abort_playback(&sink2, &playback2);
    });

    (handle, tx)
}

///Error occurred, stop program
fn abort_playback(sink: &Sink, playback: &Mutex<Playback>) {
    {
        playback.lock().unwrap().control_error = true;
    }
    stop_playback(sink, playback);
}

/// Stop program for whatever reason
fn stop_playback(sink: &Sink, state: &Mutex<Playback>) {
    let mut playback = state.lock().unwrap();
    playback.stopping = true;
    sink.clear();
}

fn run(mut state: ControlState, playback: &Mutex<Playback>, rx: Receiver<ControlMessage>) {
    //setting up stdout and going into raw mode
    if let Err(e) = terminal::enable_raw_mode() {
        eprintln!("Error enabling raw mode: {e}");
        abort_playback(&state.sink, playback);
        return;
    }

    let result = control_loop(&mut state, playback, rx);

    terminal::disable_raw_mode().unwrap();
    io::stdout()
        .execute(Print("\n"))
        .unwrap()
        .execute(MoveToColumn(0))
        .unwrap();

    if let Err(e) = result {
        abort_playback(&state.sink, playback);
        eprintln!("Unexpected error: {e}")
    }
}

fn control_loop(
    state: &mut ControlState, playback: &Mutex<Playback>, rx: Receiver<ControlMessage>,
) -> Result<(), Box<dyn Error>> {
    print_help(state)?;
    state.last_out_was_action = false;

    for c in rx.into_iter() {
        match c {
            ControlMessage::StreamDone => break,
            ControlMessage::InputEvent(e) => {
                if let Event::Key(event) = e {
                    eval_key(state, playback, event)?
                }
            }
            ControlMessage::StartSong(index) => {
                let playback = playback.lock().unwrap();
                state.song_index = index;
                display_message(
                    format!("Playing {}", playback.playlist.song(index).unwrap()).as_str(),
                    state,
                )?;
            }
            ControlMessage::StreamError(e) => {
                display_error(e.as_str(), state)?;
            }
        }
    }
    Ok(())
}

fn eval_key(
    state: &mut ControlState, playback: &Mutex<Playback>, event: KeyEvent,
) -> Result<(), Box<dyn Error>> {
    match event.code {
        KeyCode::Char('q') => stop_playback(&state.sink, playback),
        KeyCode::Char('h') => {
            print_help(state)?;
        }
        KeyCode::Char(' ') => toggle_pause(state)?,
        KeyCode::Up => {
            adjust_volume(state, &mut playback.lock().unwrap(), true)?;
        }
        KeyCode::Down => {
            adjust_volume(state, &mut playback.lock().unwrap(), false)?;
        }
        KeyCode::Right => {
            state.sink.clear();
            state.sink.play();
        }
        KeyCode::Char('s') => save(state, playback)?,
        _ => (),
    }

    Ok(())
}

fn print_help(state: &mut ControlState) -> Result<(), io::Error> {
    display_action(
        "Exit: q, Help: h, Play/Pause: space, Volume: \u{2191}/\u{2193}, Next: \u{2192}, Save: s",
        state,
    )
}

fn toggle_pause(state: &mut ControlState) -> Result<(), io::Error> {
    if state.sink.is_paused() {
        state.sink.play();
        display_action("Play", state)
    } else {
        state.sink.pause();
        display_action("Pause", state)
    }
}

fn save(state: &mut ControlState, playback: &Mutex<Playback>) -> Result<(), Box<dyn Error>> {
    let playback = playback.lock().unwrap();
    if let Some(path) = &playback.save_path {
        match file::save_playlist(&playback.playlist, path) {
            Err(e) => {
                display_error(
                    format!("Unable to save to {:?}, error: {}", path, e).as_str(),
                    state,
                )?;
            }
            Ok(_) => {
                display_action(format!("Successfully saved to {:?}", path).as_str(), state)?;
            }
        }
    } else {
        display_error("Unable to save: Direct play mode.", state)?;
    }
    Ok(())
}

///Not up means down
fn adjust_volume(
    state: &mut ControlState, playback: &mut Playback, up: bool,
) -> Result<(), Box<dyn Error>> {
    let song = playback.playlist.song_mut(state.song_index).unwrap();
    song.config.volume = calc_new_volume(song.config.volume, up);
    display_action(
        format!("Volume {:.0}%", song.config.volume * 100.0).as_str(),
        state,
    )?;

    let song = playback.playlist.song(state.song_index).unwrap();
    audio::config_sink(&state.sink, &song.config, &playback.playlist.config);
    Ok(())
}

///Won't be overwritten
fn display_message(text: &str, state: &mut ControlState) -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    if state.last_out_was_action {
        stdout.execute(terminal::Clear(ClearType::CurrentLine))?;
        state.last_out_was_action = false;
    } else {
        stdout.execute(Print("\n"))?;
    }
    stdout.execute(MoveToColumn(0))?.execute(Print(text))?;

    Ok(())
}

///Will be overwritten by next output
fn display_action(text: &str, state: &mut ControlState) -> Result<(), io::Error> {
    display_message(text, state)?;
    state.last_out_was_action = true;
    Ok(())
}

///Error variant for display_error
fn display_error(text: &str, state: &mut ControlState) -> Result<(), io::Error> {
    let mut stdout = io::stdout();
    stdout.execute(SetForegroundColor(Color::DarkRed))?;
    display_message(text, state)?;
    stdout.execute(ResetColor)?;
    Ok(())
}

fn calc_new_volume(mut vol: f32, up: bool) -> f32 {
    let ratio = 0.1;
    let min_vol = 0.05;
    let max_vol = 3.0;
    if up {
        vol /= 1.0 - ratio;
        if vol > max_vol {
            vol = max_vol
        }
    } else {
        vol *= 1.0 - ratio;
        if vol < min_vol {
            vol = min_vol
        }
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
