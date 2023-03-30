use clap::{Args, Parser, Subcommand, ValueEnum};
use clap::builder::PossibleValue;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, PartialEq)]
#[derive(Subcommand)]
pub enum Commands {
    /// Play sound files or playlist
    Play(PlayConfig),
    /// Edit or create a playlist
    Edit(EditConfig),
}

#[derive(Debug, PartialEq)]
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
    pub amplify: Option<f32>,
}


#[derive(Debug, PartialEq)]
#[derive(Args)]
pub struct EditConfig {
    /// Playlist to edit. Will create if not existing.
    pub playlist: String,
    #[arg(long)]
    /// Sound file or directory of sound files to add to playlist.
    pub file: Option<String>,
    #[arg(long)]
    /// Acts multiplicative to the amplification of each song.
    pub amplify: Option<f32>,
    #[arg(long, value_enum)]
    /// Unless songs are repeating 'on' and 'shuffle' act the same.
    pub random: Option<RandomMode>,
}

#[derive(Debug, PartialEq)]
#[derive(Clone)]
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
