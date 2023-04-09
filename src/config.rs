use std::fmt;
use std::fmt::Formatter;

use clap::builder::PossibleValue;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Play sound files or playlist
    Play(PlayConfig),
    /// Edit or create a playlist
    Edit(EditConfig),
    Display(DisplayConfig),
}

#[derive(Args)]
pub struct PlayConfig {
    /// Sound file or directory of sound files
    pub file: String,
    #[arg(short, long)]
    /// Given file is a single playlist
    pub playlist: bool,
    #[arg(long)]
    /// Play songs in a loop
    pub repeat: bool,
    #[arg(long)]
    /// Overwrites playlist config
    pub volume: Option<f32>,
}

#[derive(Args)]
pub struct EditConfig {
    /// Playlist to edit. Will create a new one if not existing.
    pub playlist: String,
    #[arg(long)]
    /// Sound file or directory of sound files to add to playlist.
    pub file: Option<String>,
    #[arg(long)]
    /// Acts multiplicative to the volume of each song.
    pub volume: Option<f32>,
    #[arg(long, value_enum)]
    /// Unless songs are repeating 'on' and 'shuffle' act the same.
    pub random: Option<RandomMode>,
}

#[derive(Args)]
pub struct DisplayConfig {
    pub playlist: String,
}

#[derive(Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub enum RandomMode {
    Off,
    True,
    Shuffle,
}

impl ValueEnum for RandomMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[RandomMode::Off, RandomMode::True, RandomMode::Shuffle]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            RandomMode::Off => "off",
            RandomMode::True => "on",
            RandomMode::Shuffle => "shuffle",
        }))
    }
}

impl fmt::Display for RandomMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            RandomMode::Off => write!(f, "OFF"),
            RandomMode::True => write!(f, "TRUE"),
            RandomMode::Shuffle => write!(f, "SHUFFLE"),
        }
    }
}
