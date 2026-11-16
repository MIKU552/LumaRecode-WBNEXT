use luma_core::render::{InputMap, input_map};

#[test]
fn keeps_metadata_and_chapters_for_a_plain_conversion() {
    let arguments = input_map(&InputMap {
        input: "clip.mp4",
        video_index: 0,
        audio_map: "0:a?",
        preserve_metadata: true,
        preserve_chapters: true,
    });

    assert_eq!(
        arguments.join(" "),
        "-i clip.mp4 -map 0:0 -map 0:a? -map_metadata 0 -map_chapters 0 -sn -dn"
    );
}

#[test]
fn drops_metadata_and_chapters_for_log_and_frame_ranges() {
    let arguments = input_map(&InputMap {
        input: "clip.mov",
        video_index: 2,
        audio_map: "0:a:0?",
        preserve_metadata: false,
        preserve_chapters: false,
    });

    assert_eq!(
        arguments.join(" "),
        "-i clip.mov -map 0:2 -map 0:a:0? -map_metadata -1 -map_chapters -1 -sn -dn"
    );
}
