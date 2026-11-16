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

#[test]
fn color_tags_follow_the_expected_order() {
    let tags = luma_core::render::ColorTags {
        primaries: Some("bt2020".to_owned()),
        transfer: Some("arib-std-b67".to_owned()),
        space: Some("bt2020nc".to_owned()),
        range: Some("tv".to_owned()),
    };
    let arguments = luma_core::render::color_arguments(&tags, false);

    assert_eq!(
        arguments.join(" "),
        "-color_primaries bt2020 -color_trc arib-std-b67 -colorspace bt2020nc -color_range tv"
    );
}

#[test]
fn unspecified_color_tags_are_omitted() {
    let tags = luma_core::render::ColorTags {
        primaries: Some(String::new()),
        transfer: None,
        space: Some("bt709".to_owned()),
        range: Some("tv".to_owned()),
    };
    let arguments = luma_core::render::color_arguments(&tags, false);

    assert_eq!(arguments.join(" "), "-colorspace bt709 -color_range tv");
}

#[test]
fn dolby_vision_profile_five_suppresses_the_tags() {
    let tags = luma_core::render::ColorTags {
        primaries: Some("bt2020".to_owned()),
        transfer: Some("smpte2084".to_owned()),
        space: Some("bt2020nc".to_owned()),
        range: Some("tv".to_owned()),
    };

    assert!(luma_core::render::color_arguments(&tags, true).is_empty());
}
