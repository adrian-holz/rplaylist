use std::{error::Error, fmt, fs};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use rand::{Rng, thread_rng};
use rand::seq::SliceRandom;

use crate::config::{Cli, EditConfig, PlayConfig, RandomMode};
use crate::config::Commands::{Edit, Play};
use crate::playlist::{Playlist, PlaylistConfig, Song};

pub mod config;
mod audio;
mod playlist;

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
            let mut p = load_playlist(path).unwrap_or_else(|_| Playlist::new());
            edit_playlist(&mut p, c)?;
            save_playlist(&p, path)?;
            Ok(())
        }
    }
}

fn edit_playlist(p: &mut Playlist, c: EditConfig) -> Result<(), Box<dyn Error>> {
    if let Some(f) = c.file {
        add_file_to_playlist(p, &PathBuf::from(f))?;
    }
    if let Some(a) = c.amplify {
        p.config.amplify = a;
    }
    if let Some(r) = c.random {
        p.config.random = r;
    }
    Ok(())
}

fn play(c: &PlayConfig) -> Result<(), Box<dyn Error>> {
    let p = &PathBuf::from(&c.file);
    let mut p = if c.playlist {
        load_playlist(p)?
    } else {
        make_playlist_from_path(p)?
    };
    if let Some(a) = c.amplify {
        p.config.amplify = a;
    }
    if p.songs().len() == 0 {
        return Err(Box::new(LibError::new(String::from("Playlist is empty"))));
    }

    if !c.repeat {
        play_playlist(&p)
    } else {
        loop {
            if p.config.random == RandomMode::True {
                play_true_random(&p)?;
            } else {
                play_playlist(&p)?;
            }
        }
    }
}

fn play_playlist(playlist: &Playlist) -> Result<(), Box<dyn Error>> {
    let songs = playlist.songs();

    let mut order: Vec<usize> = (0..songs.len()).collect();

    match playlist.config.random {
        RandomMode::Off => (),
        _ => order.shuffle(&mut thread_rng()),
    }

    for song_index in order {
        play_song(&songs[song_index], &playlist.config)?;
    }

    Ok(())
}

fn play_true_random(p: &Playlist) -> Result<(), Box<dyn Error>> {
    let idx = thread_rng().gen_range(0..p.songs().len());
    play_song(&p.songs()[idx], &p.config)
}

fn play_song(song: &Song, c: &PlaylistConfig) -> Result<(), Box<dyn Error>> {
    println!("Now playing: {}", song);
    let file = File::open(&song.path)?;
    audio::play(file, &song.config, c)
}

fn add_file_to_playlist(playlist: &mut Playlist, file: &PathBuf) -> Result<(), Box<dyn Error>> {
    let p = make_playlist_from_path(file)?;
    for s in p.songs() {
        if let Err(e) = playlist.add_song(s.clone()) {
            eprintln!("{}", e);
        }
    }
    Ok(())
}

fn make_playlist_from_path(path: &PathBuf) -> Result<Playlist, Box<dyn Error>> {
    if path.is_file() {
        let mut p = Playlist::new();
        p.add_song(Song::new(path.clone())).expect("Can always add a Song to an empty playlist");
        Ok(p)
    } else if path.is_dir() {
        let mut playlist = Playlist::new();

        let paths = path.read_dir()?;
        for path in paths {
            let p = path?.path();
            if p.is_file() {
                if let Err(e) = playlist.add_song(Song::new(p)) {
                    eprintln!("{}", e);
                }
            }
        }

        Ok(playlist)
    } else {
        Err(Box::new(LibError::new(String::from("Expected file or directory"))))
    }
}

fn save_playlist(playlist: &Playlist, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let playlist = serde_json::to_string(playlist)?;

    let mut output = File::create(path)?;
    write!(output, "{}", playlist)?;

    Ok(())
}

fn load_playlist(path: &PathBuf) -> Result<Playlist, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;
    let p: Playlist = serde_json::from_str(data.as_str())?;
    Ok(p)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_deserialize_empty_list() {
        let path = &PathBuf::from("test_data/empty.playlist");
        let p = load_playlist(path).expect("Loading playlist from test_data/ should work");
        assert_eq!(p, Playlist::new());
    }

    #[test]
    fn valid_de_serialize_empty_list() {
        let path = &PathBuf::from("test.playlist");
        let p1 = Playlist::new();
        save_playlist(&p1, path).expect("Saving in working directory should work");
        let p2 = load_playlist(path).expect("Loading saved playlist should work");
        assert_eq!(p1, p2);
    }

    #[test]
    fn edit_no_change() {
        let c = EditConfig { amplify: None, file: None, random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        edit_playlist(&mut p1, c).expect("Editing should give no error");

        assert_eq!(p1, Playlist::new())
    }

    #[test]
    fn valid_edit_amplify() {
        let c = EditConfig { amplify: Some(10.0), file: None, random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        edit_playlist(&mut p1, c).expect("Editing should give no error");

        let mut p2 = Playlist::new();
        p2.config.amplify = 10.0;
        assert_eq!(p1, p2)
    }

    #[test]
    fn valid_edit_add_file() {
        let c = EditConfig { amplify: None, file: Some(String::from("test_data/test.mp3")), random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        edit_playlist(&mut p1, c).expect("Editing should give no error");

        let mut p2 = Playlist::new();
        p2.add_song(Song::new(PathBuf::from("test_data/test.mp3"))).expect("Can always add a Song to an empty playlist");
        assert_eq!(p1, p2)
    }

    #[test]
    fn invalid_edit_add_file() -> Result<(), &'static str> {
        let c = EditConfig { amplify: None, file: Some(String::from("invalid.mp3")), random: None, playlist: String::from("") };

        let mut p1 = Playlist::new();
        match edit_playlist(&mut p1, c) {
            Err(_) => Ok(()),
            Ok(_) => Err("Invalid file should give error.")
        }
    }
}
