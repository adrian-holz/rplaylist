use std::{error::Error, fmt, thread};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rand::Rng;
use rand::seq::SliceRandom;
use rodio::{OutputStream, Sink};

use crate::config::{Cli, EditConfig, PlayConfig, RandomMode};
use crate::config::Commands::{Display, Edit, Play};
use crate::controls::PlayState;
use crate::playlist::{Playlist, PlaylistConfig, Song};

pub mod config;
mod audio;
mod playlist;
mod controls;
mod file;

#[derive(Debug)]
struct HandledError {}

impl Error for HandledError {}

impl fmt::Display for HandledError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error was handled")
    }
}

#[derive(Debug)]
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
        Play(c) => play(&c),
        Edit(c) => {
            let path = &PathBuf::from(&c.playlist);
            let mut p = file::load_playlist(path).unwrap_or_else(|_| Playlist::new());
            edit_playlist(&mut p, c)?;
            file::save_playlist(&p, path)?;
            Ok(())
        }
        Display(c) => {
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
    let path = PathBuf::from(&c.file);
    let mut p = if c.playlist {
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

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Arc::new(Sink::try_new(&stream_handle)?);
    let sink_ctrl = Arc::clone(&sink);

    let p = Arc::new(Mutex::new(PlayState { path, playlist: p, song_idx: 0 }));
    let p_ctrl = Arc::clone(&p);

    thread::spawn(move || controls::song_controls(&sink_ctrl, &p_ctrl));

    if !c.repeat {
        play_playlist(&p, &sink)
    } else {
        loop {
            if p.lock().unwrap().playlist.config.random == RandomMode::True {
                play_true_random(&p, &sink)?;
            } else {
                play_playlist(&p, &sink)?;
            }
        }
    }
}

fn play_playlist(state: &Mutex<PlayState>, sink: &Sink) -> Result<(), Box<dyn Error>> {
    let p = &state.lock().unwrap().playlist;

    let mut order: Vec<usize> = (0..p.song_count()).collect();

    match state.lock().unwrap().playlist.config.random {
        RandomMode::Off => (),
        _ => order.shuffle(&mut rand::thread_rng()),
    };

    for song_index in order {
        let song;
        let config;
        {
            let mut state = state.lock().unwrap();
            state.song_idx = song_index;
            song = p.song(song_index).unwrap();
            config = state.playlist.config.clone();
        }
        play_song(song, sink, &config)?;
    }

    Ok(())
}

fn play_true_random(state: &Mutex<PlayState>, sink: &Sink) -> Result<(), Box<dyn Error>> {
    let song;
    let config;
    {
        let mut state = state.lock().unwrap();
        let idx = rand::thread_rng().gen_range(0..state.playlist.song_count());
        state.song_idx = idx;
        song = state.playlist.song(idx).unwrap().clone();
        config = state.playlist.config.clone();
    }
    play_song(&song, sink, &config)
}

fn play_song(song: &Song, sink: &Sink, c: &PlaylistConfig) -> Result<(), Box<dyn Error>> {
    println!("Now playing: {}", song);
    let file = File::open(&song.path)?;
    audio::play(file, sink, &song.config, c)?;
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
