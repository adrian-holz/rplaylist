use std::fmt;
use std::fmt::Formatter;
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
        Playlist {
            config: PlaylistConfig::new(),
            songs: vec![],
        }
    }
    pub fn song(&self, index: usize) -> Option<&Song> {
        self.songs.get(index)
    }
    pub fn song_mut(&mut self, index: usize) -> Option<&mut Song> {
        self.songs.get_mut(index)
    }
    pub fn song_count(&self) -> usize {
        self.songs.len()
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

impl fmt::Display for Playlist {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "  Settings:")?;
        write!(f, "\n{}", self.config)?;
        write!(f, "\n  Songs:")?;
        for s in self.songs.iter() {
            write!(f, "\n{}", s)?
        }
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
        Song {
            path,
            config: SongConfig::new(),
        }
    }
}

impl fmt::Display for Song {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(s) = self.path.file_name() {
            if let Some(s) = s.to_str() {
                return write!(f, "{:}", s);
            }
        }
        // If we can't print only the file name, just print everything
        write!(f, "{:}", self.path.display())
    }
}

#[derive(Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct SongConfig {
    pub volume: f32,
}

impl SongConfig {
    pub fn new() -> SongConfig {
        SongConfig { volume: 1.0 }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize)]
pub struct PlaylistConfig {
    pub volume: f32,
    pub random: RandomMode,
}

impl PlaylistConfig {
    pub fn new() -> PlaylistConfig {
        PlaylistConfig {
            volume: 1.0,
            random: RandomMode::Off,
        }
    }
}

impl fmt::Display for PlaylistConfig {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Amplify: {}; Random mode: {}", self.volume, self.random)
    }
}
