use std::fs::File;
use std::io::BufReader;

use rodio::{Decoder, Sink};
use rodio::decoder::DecoderError;

use crate::HandledError;
use crate::playlist::{PlaylistConfig, SongConfig};

pub fn play(file: File, sink: &Sink, song_config: &SongConfig, global_config: &PlaylistConfig) -> Result<(), HandledError> {
    let buf = BufReader::new(file);

    let source = Decoder::new(buf);


    let source = match source {
        Ok(s) => { s }
        Err(DecoderError::UnrecognizedFormat) => {
            return Err(HandledError::new(String::from("Unrecognized Format, skipping.")));
        }
        Err(e) => {
            eprintln!("Unknown Error: {}, skipping.", e);
            return Ok(());
        }
    };

    config_sink(sink, song_config, global_config);
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

pub fn config_sink(sink: &Sink, song_config: &SongConfig, global_config: &PlaylistConfig) {
    sink.set_volume(song_config.volume * global_config.volume);
}
