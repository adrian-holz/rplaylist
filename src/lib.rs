use std::path::{PathBuf};
use std::{error::Error, fmt, fs};
use std::fs::File;
use std::io::Write;
use crate::config::{CmdConfig, CreateListConfig, PlayFileConfig};
use crate::config::CmdConfig::{AddFile, CreateList, PlayFile, PlayList};
use crate::playlist::{Playlist, Song};

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

pub fn run(config: CmdConfig) -> Result<(), Box<dyn Error>> {
    match config {
        PlayFile(c) => play_playlist(&make_playlist(&c)?),
        PlayList(c) => play_playlist(&load_playlist(&c.playlist)?),
        CreateList(c) => save_playlist(&create_playlist(&c)?, &c.playlist),
        AddFile(c) => {
            let mut p = load_playlist(&c.playlist)?;
            add_file_to_playlist(&mut p, &c.file)?;
            save_playlist(&p, &c.playlist)?;
            Ok(())
        }
        // _ => Err(Box::new(LibError::new(String::from("not implemented"))))
    }
}

fn play_playlist(playlist: &Playlist) -> Result<(), Box<dyn Error>> {
    if playlist.songs.len() == 0 {
        return Err(Box::new(LibError::new(String::from("Playlist is empty"))));
    }
    for song in playlist.songs.as_slice() {
        println!("Now playing {}", song);
        let file = File::open(&song.path)?;
        audio::play(file, &song.config)?;
    }
    Ok(())
}

fn create_playlist(c: &CreateListConfig) -> Result<Playlist, Box<dyn Error>> {
    if let Some(f) = &c.file {
        make_playlist_from_path(f)
    } else {
        Ok(Playlist::new())
    }
}

fn make_playlist(config: &PlayFileConfig) -> Result<Playlist, Box<dyn Error>> {
    make_playlist_from_path(&config.file)
}

fn add_file_to_playlist(playlist: &mut Playlist, file: &PathBuf) -> Result<(), Box<dyn Error>> {
    let mut p = make_playlist_from_path(file)?;
    playlist.songs.append(&mut p.songs);
    Ok(())
}

fn make_playlist_from_path(path: &PathBuf) -> Result<Playlist, Box<dyn Error>> {
    if path.is_file() {
        Ok(Playlist::new())
    } else if path.is_dir() {
        let mut playlist = Playlist::new();

        let paths = path.read_dir()?;
        for path in paths {
            let p = path?.path();
            if p.is_file() {
                playlist.songs.push(Song::new(p));
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
        let path = &PathBuf::from("../test.playlist.json");
        let p1 = Playlist::new();
        save_playlist(&p1, path).expect("Saving in working directory should work");
        let p2 = load_playlist(path).expect("Loading saved playlist should work");
        assert_eq!(p1, p2);
    }
}
