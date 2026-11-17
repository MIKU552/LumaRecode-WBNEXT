use luma_core::domain::Purpose;
use luma_core::render::{
    ColorFilterRequest, ColorTags, InputMap, input_map, software_color_filters,
};

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

#[test]
fn share_encodes_with_global_quality_and_a_low_power_flag() {
    use luma_core::domain::Codec;
    use luma_core::render::{EncoderRequest, encoder_arguments};

    let arguments = encoder_arguments(&EncoderRequest {
        encoder: "hevc_qsv".to_owned(),
        preset: "medium".to_owned(),
        quality: "26".to_owned(),
        share: true,
        codec: Codec::Hevc,
        depth: 10,
        gop: 60,
        auto_low_power: false,
        timing_compatibility: true,
        async_depth: "4".to_owned(),
        pixel_format: "p010le".to_owned(),
    });

    assert_eq!(
        arguments.join(" "),
        "-c:v hevc_qsv -preset medium -global_quality:v 26 -low_power 1 -async_depth 4 -g 60 -bf:v 0 -profile:v main10"
    );
}

#[test]
fn archive_encodes_with_crf_and_the_pixel_format() {
    use luma_core::domain::Codec;
    use luma_core::render::{EncoderRequest, encoder_arguments};

    let arguments = encoder_arguments(&EncoderRequest {
        encoder: "libx265".to_owned(),
        preset: "slow".to_owned(),
        quality: "18".to_owned(),
        share: false,
        codec: Codec::Hevc,
        depth: 10,
        gop: 240,
        auto_low_power: true,
        timing_compatibility: false,
        async_depth: "4".to_owned(),
        pixel_format: "yuv420p10le".to_owned(),
    });

    assert_eq!(
        arguments.join(" "),
        "-c:v libx265 -preset slow -crf 18 -pix_fmt yuv420p10le -g 240"
    );
}

#[test]
fn parameters_keep_order_and_append_bare_tokens() {
    use luma_core::render::parameter_string;

    let values = vec![
        ("pools".to_owned(), "16".to_owned()),
        ("log-level".to_owned(), "warning".to_owned()),
    ];
    let appended = vec!["hdr10=1".to_owned()];

    assert_eq!(
        parameter_string(&values, &appended),
        "pools=16:log-level=warning:hdr10=1"
    );
}

#[test]
fn an_empty_parameter_list_produces_an_empty_string() {
    assert!(luma_core::render::parameter_string(&[], &[]).is_empty());
}

#[test]
fn filters_join_in_order_and_skip_empty_entries() {
    let filters = vec![
        "trim=start_frame=0:end_frame=10".to_owned(),
        String::new(),
        "setpts=PTS-STARTPTS".to_owned(),
    ];

    assert_eq!(
        luma_core::render::filter_chain(&filters),
        "trim=start_frame=0:end_frame=10,setpts=PTS-STARTPTS"
    );
}

#[test]
fn gop_follows_rate_and_purpose() {
    assert_eq!(luma_core::render::gop(30.0, true, false), 60);
    assert_eq!(luma_core::render::gop(120.0, true, false), 240);
    assert_eq!(luma_core::render::gop(30.0, false, true), 60);
    assert_eq!(luma_core::render::gop(30.0, false, false), 240);
}

#[test]
fn an_unknown_rate_falls_back_to_thirty() {
    assert_eq!(luma_core::render::gop(0.0, false, false), 240);
    assert_eq!(luma_core::render::gop(f64::NAN, true, false), 60);
}

fn color_request(purpose: Purpose, depth: u8, format: &str) -> ColorFilterRequest {
    ColorFilterRequest {
        tags: ColorTags {
            primaries: Some("bt709".to_owned()),
            transfer: Some("bt709".to_owned()),
            space: Some("bt709".to_owned()),
            range: Some("tv".to_owned()),
        },
        range: "limited".to_owned(),
        pixel_format: format.to_owned(),
        purpose,
        depth,
        preserve_dynamic: false,
    }
}

#[test]
fn the_software_tail_converts_range_and_writes_tags() {
    let filters = software_color_filters(&color_request(Purpose::Archive, 8, "yuv420p"));

    let expected = "zscale=range=limited:dither=error_diffusion:primaries=bt709:transfer=bt709:matrix=bt709,format=yuv420p,setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited";
    assert_eq!(filters.join(","), expected);
}

#[test]
fn share_hands_the_encoder_a_ten_bit_surface() {
    let filters = software_color_filters(&color_request(Purpose::Share, 10, "yuv420p10le"));

    assert_eq!(filters[1], "format=p010le");
}

#[test]
fn undeclared_tags_are_left_out_and_the_range_word_is_spelled_out() {
    let mut bare = color_request(Purpose::Archive, 8, "yuv420p");
    bare.tags = ColorTags {
        range: Some("pc".to_owned()),
        ..ColorTags::default()
    };

    let filters = software_color_filters(&bare);
    assert_eq!(filters[0], "zscale=range=limited:dither=error_diffusion");
    assert_eq!(filters[2], "setparams=range=full");
}

#[test]
fn preserving_dynamic_metadata_skips_the_conversion() {
    let mut kept = color_request(Purpose::Archive, 10, "yuv420p10le");
    kept.preserve_dynamic = true;

    assert!(software_color_filters(&kept).is_empty());
}
