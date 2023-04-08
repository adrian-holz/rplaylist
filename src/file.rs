use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::LibError;
use crate::playlist::{Playlist, Song};

pub fn make_playlist_from_path(path: &PathBuf) -> Result<Playlist, Box<dyn Error>> {
    let songs = load_songs_from_directory(path)?;

    let mut p = Playlist::new();
    for song in songs {
        if let Err(e) = p.add_song(song) {
            eprintln!("{}", e);
        }
    }
    Ok(p)
}

pub fn load_songs_from_directory(path: &PathBuf) -> Result<Vec<Song>, Box<dyn Error>> {
    if path.is_file() {
        Ok(vec![Song::new(path.clone())])
    } else if path.is_dir() {
        let mut songs = vec![];

        let paths = path.read_dir()?;
        for path in paths {
            let p = path?.path();
            if p.is_file() {
                songs.push(Song::new(p))
            }
        }

        Ok(songs)
    } else {
        Err(Box::new(LibError::new(String::from("Expected file or directory"))))
    }
}

pub fn save_playlist(playlist: &Playlist, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let playlist = serde_json::to_string(playlist)?;

    let mut output = File::create(path)?;
    write!(output, "{}", playlist)?;

    Ok(())
}

pub fn load_playlist(path: &PathBuf) -> Result<Playlist, Box<dyn Error>> {
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
}