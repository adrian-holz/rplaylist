use std::fmt;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub songs: Vec<Song>,
}

impl Playlist {
    pub fn new() -> Playlist {
        Playlist { songs: vec![] }
    }
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct SongConfig {
    pub amplify: f32,
}

impl SongConfig {
    pub fn new() -> SongConfig {
        SongConfig { amplify: 0.2 }
    }
}