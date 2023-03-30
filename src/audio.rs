use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use rodio::{Decoder, OutputStream, Sink, Source};
use rodio::decoder::DecoderError;

use crate::playlist::{PlaylistConfig, SongConfig};

pub fn play(file: File, song_config: &SongConfig, global_config: &PlaylistConfig) -> Result<(), Box<dyn Error>> {
    let buf = BufReader::new(file);

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let source = Decoder::new(buf);

    if let Err(e) = &source {
        if let DecoderError::UnrecognizedFormat = e {
            eprintln!("Unrecognized Format, skipping.");
            return Ok(());
        }
    }

    let source = source?;

    let source = source.amplify(song_config.amplify * global_config.amplify);

    sink.append(source);
    sink.sleep_until_end();

    return Ok(());
}
