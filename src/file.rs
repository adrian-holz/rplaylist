use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::playlist::{Playlist, Song};
use crate::LibError;

pub fn make_playlist_from_path(path: &Path) -> Result<Playlist, LibError> {
    let songs = load_songs(path)?;

    let mut p = Playlist::new();
    for song in songs {
        if let Err(e) = p.add_song(song) {
            eprintln!("Error adding song: {e}");
        }
    }
    Ok(p)
}

pub fn load_songs(path: &Path) -> Result<Vec<Song>, LibError> {
    if path.is_file() {
        Ok(vec![Song::new(PathBuf::from(path))])
    } else if path.is_dir() {
        let songs = load_songs_from_directory(path);
        match songs {
            Ok(s) => Ok(s),
            Err(e) => Err(LibError(
                String::from("Unable to read songs from directory"),
                Some(Box::new(e)),
            )),
        }
    } else {
        Err(LibError::new(String::from("Expected file or directory")))
    }
}

fn load_songs_from_directory(path: &Path) -> Result<Vec<Song>, io::Error> {
    let mut songs = vec![];

    let paths = path.read_dir()?;
    for path in paths {
        let p = path?.path();
        if p.is_file() {
            songs.push(Song::new(p));
        }
    }

    Ok(songs)
}

pub fn save_playlist(playlist: &Playlist, path: &PathBuf) -> Result<(), LibError> {
    let playlist = serde_json::to_string(playlist).unwrap();

    File::create(path)
        .and_then(|mut o| write!(o, "{playlist}"))
        .map_err(|e| LibError(String::from("Error writing playlist"), Some(Box::new(e))))
}

pub fn load_playlist(path: &PathBuf) -> Result<Playlist, LibError> {
    let data = fs::read_to_string(path);
    let data = match data {
        Ok(d) => d,
        Err(e) => {
            return Err(LibError(
                String::from("Error reading playlist"),
                Some(Box::new(e)),
            ));
        }
    };

    serde_json::from_str(data.as_str()).map_err(|e| {
        LibError(
            String::from("Error deserializing playlist"),
            Some(Box::new(e)),
        )
    })
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
