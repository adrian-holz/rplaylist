use std::fs::File;
use std::io::BufReader;

use rodio::decoder::DecoderError;
use rodio::{Decoder, Sink};

use crate::playlist::{PlaylistConfig, SongConfig};
use crate::LibError;

pub fn play(
    file: File, sink: &Sink, song_config: &SongConfig, global_config: &PlaylistConfig,
) -> Result<(), LibError> {
    let buf = BufReader::new(file);

    let source = Decoder::new(buf);

    let source = match source {
        Ok(s) => s,
        Err(DecoderError::UnrecognizedFormat) => {
            return Err(LibError::new(String::from(
                "Unrecognized Format, skipping.",
            )));
        }
        Err(e) => {
            return Err(LibError::new(format!("Unknown Error: {e}, skipping.")));
        }
    };

    config_sink(sink, song_config, global_config);
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

///Can we decode this file? Does not necessarily mean we can play it to the end.
pub fn valid_audio_file(file: File) -> bool {
    let buf = BufReader::new(file);
    let source = Decoder::new(buf);

    source.is_ok()
}

pub fn config_sink(sink: &Sink, song_config: &SongConfig, global_config: &PlaylistConfig) {
    sink.set_volume(song_config.volume * global_config.volume);
}
