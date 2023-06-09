#![deny(clippy::pedantic)]
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::{error::Error, fmt};

use rand::seq::SliceRandom;
use rand::Rng;
use rodio::{OutputStream, Sink};

use crate::config::{Cli, Command, EditCommand, PlayCommand, RandomMode};
use crate::controls::{ControlMessage, Playback};
use crate::playlist::Playlist;

mod audio;
pub mod config;
mod controls;
mod file;
mod playlist;

#[derive(Debug)]
///Error was handled, we just need to display it now.
///May contain an actual error to display in verbose mode.
pub struct LibError(pub String, pub Option<Box<dyn Error>>);

impl LibError {
    fn new(msg: String) -> Self {
        Self(msg, None)
    }
}

impl Error for LibError {}

impl fmt::Display for LibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.1 {
            None => write!(f, "{}", self.0),
            Some(e) => write!(f, "{}: {}", self.0, e),
        }
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn run(config: Cli) -> Result<(), LibError> {
    match config.command {
        Command::Play(c) => play(&c),
        Command::Edit(c) => {
            let path = &PathBuf::from(&c.playlist);
            let p = file::load_playlist(path).unwrap_or_else(|_| Playlist::new());
            let p = edit_playlist(p, c)?;
            file::save_playlist(&p, path)?;
            Ok(())
        }
        Command::Display(c) => {
            println!("{}", file::load_playlist(&PathBuf::from(&c.playlist))?);
            Ok(())
        }
    }
}

fn edit_playlist(mut p: Playlist, c: EditCommand) -> Result<Playlist, LibError> {
    if let Some(f) = c.file {
        add_file_to_playlist(&mut p, Path::new(f.as_str()))?;
    }
    if let Some(a) = c.volume {
        p.config.volume = a;
    }
    if let Some(r) = c.random {
        p.config.random = r;
    }
    if c.validate {
        p = validate_playlist(p);
    }
    Ok(p)
}

fn play(c: &PlayCommand) -> Result<(), LibError> {
    let state = prepare_play(c)?;
    // These need to be created here so they won't be dropped until we are done playing,
    // as Sink does not take ownership.
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(stream) => stream,
        Err(e) => {
            return Err(LibError(
                String::from("Unable to create audio stream"),
                Some(Box::new(e)),
            ));
        }
    };
    let sink = match Sink::try_new(&stream_handle) {
        Ok(s) => s,
        Err(e) => {
            return Err(LibError(
                String::from("Unable to start audio stream"),
                Some(Box::new(e)),
            ));
        }
    };

    let sink = Arc::new(sink);
    let state = Arc::new(Mutex::new(state));

    let (handle, tx) = controls::start(&sink, &state);

    play_playlist(&tx, &state, &sink, c.repeat);

    // Tell the controls we are done and wait for it to clean up.
    let _ = tx.send(ControlMessage::StreamDone);
    let result = handle
        .join()
        .map_err(|_| LibError::new(String::from("Controls crashed")));

    if result.is_ok() && state.lock().unwrap().control_error {
        return Err(LibError::new(String::from("Playback aborted")));
    }

    result
}

fn prepare_play(c: &PlayCommand) -> Result<Playback, LibError> {
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
        return Err(LibError::new(String::from("Playlist is empty")));
    }
    Ok(Playback::new(save_path, p))
}

fn play_playlist(tx: &Sender<ControlMessage>, state: &Mutex<Playback>, sink: &Sink, repeat: bool) {
    if repeat {
        while !state.lock().unwrap().stopped() {
            if state.lock().unwrap().playlist.config.random == RandomMode::True {
                play_true_random(tx, state, sink);
            } else {
                play_normal(tx, state, sink);
            }
        }
    } else {
        play_normal(tx, state, sink);
    }
}

fn play_normal(tx: &Sender<ControlMessage>, state: &Mutex<Playback>, sink: &Sink) {
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
        play_song(tx, state, sink, song_index);
    }
}

fn play_true_random(tx: &Sender<ControlMessage>, state: &Mutex<Playback>, sink: &Sink) {
    let index = {
        let state = state.lock().unwrap();
        rand::thread_rng().gen_range(0..state.playlist.song_count())
    };
    play_song(tx, state, sink, index);
}

fn play_song(tx: &Sender<ControlMessage>, state: &Mutex<Playback>, sink: &Sink, index: usize) {
    let song;
    let config;
    {
        let state = state.lock().unwrap();
        song = state.playlist.song(index).unwrap().clone();
        config = state.playlist.config.clone();
    }
    tx.send(ControlMessage::StartSong(index)).unwrap();

    let file = File::open(&song.path);
    match file {
        Ok(file) => {
            if let Err(LibError(msg, _)) = audio::play(file, sink, &song.config, &config) {
                tx.send(ControlMessage::StreamError(msg)).unwrap();
            }
        }
        Err(_) => tx
            .send(ControlMessage::StreamError(String::from(
                "Unable to open audio file",
            )))
            .unwrap(),
    }
}

fn validate_playlist(mut p: Playlist) -> Playlist {
    p.validate_songs(|song| {
        let file = File::open(&song.path);
        let valid = match file {
            Ok(f) => audio::valid_audio_file(f),
            Err(_) => false,
        };
        if !valid {
            eprintln!("Filtered invalid audio file: {song}");
        }
        valid
    });
    p
}

fn add_file_to_playlist(playlist: &mut Playlist, file: &Path) -> Result<(), LibError> {
    let songs = file::load_songs(file)?;
    for s in songs {
        if let Err(e) = playlist.add_song(s) {
            eprintln!("{e}");
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
        let c = EditCommand {
            volume: None,
            file: None,
            random: None,
            playlist: String::new(),
            validate: false,
        };

        let mut p1 = Playlist::new();
        p1 = edit_playlist(p1, c).expect("Editing should give no error");

        assert_eq!(p1, Playlist::new());
    }

    #[test]
    fn valid_edit_amplify() {
        let c = EditCommand {
            volume: Some(10.0),
            file: None,
            random: None,
            playlist: String::new(),
            validate: false,
        };

        let mut p1 = Playlist::new();
        p1 = edit_playlist(p1, c).expect("Editing should give no error");

        let mut p2 = Playlist::new();
        p2.config.volume = 10.0;
        assert_eq!(p1, p2);
    }

    #[test]
    fn valid_edit_add_file() {
        let c = EditCommand {
            volume: None,
            file: Some(String::from("test_data/test.mp3")),
            random: None,
            playlist: String::new(),
            validate: false,
        };

        let mut p1 = Playlist::new();
        p1 = edit_playlist(p1, c).expect("Editing should give no error");

        let mut p2 = Playlist::new();
        p2.add_song(Song::new(PathBuf::from("test_data/test.mp3")))
            .expect("Can always add a Song to an empty playlist");
        assert_eq!(p1, p2);
    }

    #[test]
    fn invalid_edit_add_file() -> Result<(), &'static str> {
        let c = EditCommand {
            volume: None,
            file: Some(String::from("invalid.mp3")),
            random: None,
            playlist: String::new(),
            validate: false,
        };

        let p1 = Playlist::new();
        match edit_playlist(p1, c) {
            Err(_) => Ok(()),
            Ok(_) => Err("Invalid file should give error."),
        }
    }

    #[test]
    fn filter_invalid_not_existing() {
        let c = EditCommand {
            volume: None,
            file: None,
            random: None,
            playlist: String::new(),
            validate: true,
        };
        let mut p = Playlist::new();
        p.add_song(Song::new(PathBuf::from("file.invalid")))
            .unwrap();
        p = edit_playlist(p, c).expect("Editing should give no error");
        assert_eq!(p.song_count(), 0);
    }

    #[test]
    fn filter_invalid_not_audio() {
        let c = EditCommand {
            volume: None,
            file: None,
            random: None,
            playlist: String::new(),
            validate: true,
        };
        let mut p = Playlist::new();
        p.add_song(Song::new(PathBuf::from("test_data/empty.playlist")))
            .unwrap();
        p = edit_playlist(p, c).expect("Editing should give no error");
        assert_eq!(p.song_count(), 0);
    }

    #[test]
    fn filter_invalid_valid() {
        let c = EditCommand {
            volume: None,
            file: None,
            random: None,
            playlist: String::new(),
            validate: true,
        };
        let mut p = Playlist::new();
        p.add_song(Song::new(PathBuf::from("test_data/test.mp3")))
            .unwrap();
        p = edit_playlist(p, c).expect("Editing should give no error");
        assert_eq!(p.song_count(), 1);
    }
}
