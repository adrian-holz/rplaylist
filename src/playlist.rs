use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::RandomMode;

#[derive(Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub config: PlaylistConfig,
    songs: Vec<Song>,
}

impl Playlist {
    pub fn new() -> Playlist {
        Playlist { config: PlaylistConfig::new(), songs: vec![] }
    }
    pub fn songs(&self) -> &Vec<Song> {
        &self.songs
    }
    pub fn add_song(&mut self, song: Song) -> Result<(), String> {
        for s in self.songs.as_slice() {
            if s.path == song.path {
                return Err(format!("Song already exists: {}", s.path.display()));
            }
        }
        self.songs.push(song);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct Song {
    pub path: PathBuf,
    pub config: SongConfig,
}

impl Song {
    pub fn new(path: PathBuf) -> Song {
        return Song { path, config: SongConfig::new() };
    }
}

impl fmt::Display for Song {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

#[derive(Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct SongConfig {
    pub amplify: f32,
}

impl SongConfig {
    pub fn new() -> SongConfig {
        SongConfig { amplify: 1.0 }
    }
}

#[derive(Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct PlaylistConfig {
    pub amplify: f32,
    pub random: RandomMode,
}

impl PlaylistConfig {
    pub fn new() -> PlaylistConfig {
        PlaylistConfig { amplify: 1.0, random: RandomMode::Off }
    }
}
