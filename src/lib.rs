use std::{error::Error, fmt};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use rand::Rng;
use rand::seq::SliceRandom;
use rodio::{OutputStream, Sink};

use crate::config::{Cli, Command, EditConfig, PlayConfig, RandomMode};
use crate::controls::{ControlMessage, PlayState};
use crate::playlist::Playlist;

pub mod config;
mod audio;
mod playlist;
mod controls;
mod file;

#[derive(Debug)]
///Error was handled, we just need to display it now
pub struct HandledError(pub String);

impl Error for HandledError {}

impl fmt::Display for HandledError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
//TODO: Do we need this?
struct LibError {
    msg: String,
}

impl LibError {
    fn new(msg: String) -> LibError {
        LibError { msg }
    }
}

impl Error for LibError {}

impl fmt::Display for LibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

pub fn run(config: Cli) -> Result<(), Box<dyn Error>> {
    match config.command {
        Command::Play(c) => play(&c),
        Command::Edit(c) => {
            let path = &PathBuf::from(&c.playlist);
            let mut p = file::load_playlist(path).unwrap_or_else(|_| Playlist::new());
            edit_playlist(&mut p, c)?;
            file::save_playlist(&p, path)?;
            Ok(())
        }
        Command::Display(c) => {
            println!("{}", file::load_playlist(&PathBuf::from(&c.playlist))?);
            Ok(())
        }
    }
}

fn edit_playlist(p: &mut Playlist, c: EditConfig) -> Result<(), Box<dyn Error>> {
    if let Some(f) = c.file {
        add_file_to_playlist(p, &PathBuf::from(f))?;
    }
    if let Some(a) = c.volume {
        p.config.volume = a;
    }
    if let Some(r) = c.random {
        p.config.random = r;
    }
    Ok(())
}

fn play(c: &PlayConfig) -> Result<(), Box<dyn Error>> {
    let state = prepare_play(c)?;
    // These need to be created here so they won't be dropped until we are done playing,
    // because Sink does not take ownership.
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let sink = Arc::new(sink);
    let state = Arc::new(Mutex::new(state));

    let (handle, tx) = controls::start(&sink, &state);

    let result = play_playlist(&tx, &state, &sink, c.repeat);

    // Tell the controls we are done and wait for it to clean up.
    tx.send(ControlMessage::StreamDone)?;
    handle.join().unwrap();

    result
}

fn prepare_play(c: &PlayConfig) -> Result<PlayState, Box<dyn Error>> {
    let path = PathBuf::from(&c.file);
    let mut save_path = None;
    let mut p = if c.playlist {
        save_path = Some(path.clone());
        file::load_playlist(&path)?
    } else {
        file::make_playlist_from_path(&path)?
    };
    if let Some(a) = c.volume {
        p.config.volume = a;
    }
    if p.song_count() == 0 {
        return Err(Box::new(LibError::new(String::from("Playlist is empty"))));
    }
    Ok(PlayState::new(save_path, p))
}

fn play_playlist(tx: &Sender<ControlMessage>, state: &Mutex<PlayState>, sink: &Sink, repeat: bool) -> Result<(), Box<dyn Error>> {
    if !repeat {
        play_normal(tx, state, sink)
    } else {
        while !state.lock().unwrap().stopped() {
            if state.lock().unwrap().playlist.config.random == RandomMode::True {
                play_true_random(tx, state, sink)?;
            } else {
                play_normal(tx, state, sink)?;
            }
        }
        Ok(())
    }
}

fn play_normal(tx: &Sender<ControlMessage>, state: &Mutex<PlayState>, sink: &Sink) -> Result<(), Box<dyn Error>> {
    let order = {
        let playlist = &state.lock().unwrap().playlist;
        let mut order: Vec<usize> = (0..playlist.song_count()).collect();

        match playlist.config.random {
            RandomMode::Off => (),
            _ => order.shuffle(&mut rand::thread_rng()),
        };

        order
    };

    for song_index in order {
        if state.lock().unwrap().stopped() {
            break;
        }
        play_song(tx, state, sink, song_index)?;
    }

    Ok(())
}

fn play_true_random(tx: &Sender<ControlMessage>, state: &Mutex<PlayState>, sink: &Sink) -> Result<(), Box<dyn Error>> {
    let index = {
        let state = state.lock().unwrap();
        rand::thread_rng().gen_range(0..state.playlist.song_count())
    };
    play_song(tx, state, sink, index)
}

fn play_song(tx: &Sender<ControlMessage>, state: &Mutex<PlayState>, sink: &Sink, index: usize) -> Result<(), Box<dyn Error>> {
    let song;
    let config;
    {
        let mut state = state.lock().unwrap();
        state.song_idx = index;
        song = state.playlist.song(index).unwrap().clone();
        config = state.playlist.config.clone();
    }
    tx.send(ControlMessage::StreamUpdate).unwrap();
    let file = File::open(&song.path)?;
    if let Err(HandledError(msg)) = audio::play(file, sink, &song.config, &config) {
        tx.send(ControlMessage::StreamError(msg)).unwrap();
    }
    Ok(())
}

fn add_file_to_playlist(playlist: &mut Playlist, file: &PathBuf) -> Result<(), Box<dyn Error>> {
    let songs = file::load_songs_from_directory(file)?;
    for s in songs {
        if let Err(e) = playlist.add_song(s) {
            eprintln!("{}", e);
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use crate::playlist::Song;

    use super::*;

    #[test]
    fn edit_no_change() {
        let c = EditConfig { volume: None, file: None, random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        edit_playlist(&mut p1, c).expect("Editing should give no error");

        assert_eq!(p1, Playlist::new())
    }

    #[test]
    fn valid_edit_amplify() {
        let c = EditConfig { volume: Some(10.0), file: None, random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        edit_playlist(&mut p1, c).expect("Editing should give no error");

        let mut p2 = Playlist::new();
        p2.config.volume = 10.0;
        assert_eq!(p1, p2)
    }

    #[test]
    fn valid_edit_add_file() {
        let c = EditConfig { volume: None, file: Some(String::from("test_data/test.mp3")), random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        edit_playlist(&mut p1, c).expect("Editing should give no error");

        let mut p2 = Playlist::new();
        p2.add_song(Song::new(PathBuf::from("test_data/test.mp3"))).expect("Can always add a Song to an empty playlist");
        assert_eq!(p1, p2)
    }

    #[test]
    fn invalid_edit_add_file() -> Result<(), &'static str> {
        let c = EditConfig { volume: None, file: Some(String::from("invalid.mp3")), random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        match edit_playlist(&mut p1, c) {
            Err(_) => Ok(()),
            Ok(_) => Err("Invalid file should give error.")
        }
    }
}
