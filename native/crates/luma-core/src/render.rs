//! Argument blocks that every host renders the same way.
//!
//! The full FFmpeg command still belongs to the host, which knows device quirks and filter
//! choices. The blocks here are the ones that must not drift between hosts: which streams are
//! mapped, whether metadata and chapters survive, and the flags that keep the process quiet.

/// One input and its stream mapping.
pub struct InputMap<'a> {
    pub input: &'a str,
    pub video_index: i32,
    /// `0:a?` for every audio track, `0:a:0?` for a motion photo's single track.
    pub audio_map: &'a str,
    pub preserve_metadata: bool,
    pub preserve_chapters: bool,
}

/// Arguments from `-i` through the stream-selection flags.
pub fn input_map(request: &InputMap<'_>) -> Vec<String> {
    vec![
        "-i".to_owned(),
        request.input.to_owned(),
        "-map".to_owned(),
        format!("0:{}", request.video_index),
        "-map".to_owned(),
        request.audio_map.to_owned(),
        "-map_metadata".to_owned(),
        if request.preserve_metadata { "0" } else { "-1" }.to_owned(),
        "-map_chapters".to_owned(),
        if request.preserve_chapters { "0" } else { "-1" }.to_owned(),
        "-sn".to_owned(),
        "-dn".to_owned(),
    ]
}
