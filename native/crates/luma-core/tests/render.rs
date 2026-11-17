use luma_core::domain::{Purpose, SourceKind};
use luma_core::render::{
    ColorFilterRequest, ColorTags, DolbyRenderFilter, FilterContentRequest, InputMap, LogFilter,
    ToneMapFilter, VppFilter, filter_content, input_map, software_color_filters, source_tags,
    target_tags,
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

fn empty_content() -> FilterContentRequest<'static> {
    FilterContentRequest {
        log: None,
        tone_map: None,
        dolby_render: None,
        drop_dynamic_metadata: false,
        vpp: None,
    }
}

#[test]
fn the_log_chain_prefers_the_source_tags_over_the_curve_defaults() {
    let mut request = empty_content();
    request.log = Some(LogFilter {
        file: "REDMI_K90_MiLog_to_Rec2100_HLG.cube",
        matrix: "2020_ncl",
        range: "full",
        source_space: Some("bt709"),
        source_range: Some("tv"),
    });

    let log = filter_content(&request).log.expect("log chain");
    assert!(log.contains("zscale=matrixin=709:rangein=limited:range=full"));
    assert!(log.contains("lut3d=file=REDMI_K90_MiLog_to_Rec2100_HLG.cube:interp=tetrahedral"));
}

#[test]
fn an_untagged_log_source_falls_back_to_the_curve_matrix_and_range() {
    let mut request = empty_content();
    request.log = Some(LogFilter {
        file: "Samsung_Log_to_Rec2100_HLG.cube",
        matrix: "2020_ncl",
        range: "limited",
        source_space: None,
        source_range: None,
    });

    let log = filter_content(&request).log.expect("log chain");
    assert!(log.contains("zscale=matrixin=2020_ncl:rangein=limited:range=full"));
}

#[test]
fn tone_mapping_drops_the_metadata_it_invalidates() {
    let mut request = empty_content();
    request.tone_map = Some(ToneMapFilter {
        range: "pc",
        apply_dolby_vision: true,
    });

    let tone = filter_content(&request).tone_map.expect("tone map");
    assert!(tone.starts_with("libplacebo=format=yuv444p16le:colorspace=bt709"));
    assert!(tone.contains("range=pc:tonemapping=bt.2390:apply_dolbyvision=1"));
    assert!(tone.ends_with("sidedata=delete:type=DYNAMIC_HDR_PLUS"));
}

#[test]
fn dolby_vision_keeps_hlg_or_switches_to_pq() {
    let mut request = empty_content();
    request.dolby_render = Some(DolbyRenderFilter {
        kind: SourceKind::Hlg,
        range: "tv",
    });
    let hlg = filter_content(&request).dolby_render.expect("hlg");
    assert!(hlg.contains("color_trc=arib-std-b67"));

    request.dolby_render = Some(DolbyRenderFilter {
        kind: SourceKind::Pq,
        range: "pc",
    });
    let pq = filter_content(&request).dolby_render.expect("pq");
    assert!(pq.contains("color_trc=smpte2084:range=pc"));
}

#[test]
fn the_qsv_surface_converts_to_rec709_only_for_sdr() {
    let mut request = empty_content();
    request.vpp = Some(VppFilter {
        depth: 10,
        out_range: "tv",
        sdr: false,
    });
    assert_eq!(
        filter_content(&request).vpp.expect("vpp"),
        "vpp_qsv=format=p010le:out_range=tv"
    );

    request.vpp = Some(VppFilter {
        depth: 8,
        out_range: "pc",
        sdr: true,
    });
    let vpp = filter_content(&request).vpp.expect("vpp");
    assert!(vpp.starts_with("vpp_qsv=format=nv12:out_range=pc"));
    assert!(vpp.contains("out_color_transfer=bt709"));
}

#[test]
fn dropped_metadata_is_only_emitted_when_a_conversion_invalidates_it() {
    let mut request = empty_content();
    assert!(filter_content(&request).dropped_metadata.is_none());

    request.drop_dynamic_metadata = true;
    assert!(filter_content(&request).dropped_metadata.is_some());
}

#[test]
fn an_untagged_source_keeps_what_it_declared_and_fills_the_rest() {
    let declared = ColorTags {
        space: Some("smpte170m".to_owned()),
        ..ColorTags::default()
    };

    let tags = source_tags(SourceKind::Sdr, false, false, &declared, "limited");
    assert_eq!(tags.primaries.as_deref(), Some("bt709"));
    assert_eq!(tags.transfer.as_deref(), Some("bt709"));
    // A declared matrix is a fact about the file, not a gap to fill.
    assert_eq!(tags.space.as_deref(), Some("smpte170m"));
    assert_eq!(tags.range.as_deref(), Some("tv"));
}

#[test]
fn an_hdr_interpretation_decides_the_transfer() {
    let declared = ColorTags {
        transfer: Some("bt709".to_owned()),
        ..ColorTags::default()
    };

    let tags = source_tags(SourceKind::Pq, false, false, &declared, "full");
    assert_eq!(tags.transfer.as_deref(), Some("smpte2084"));
    assert_eq!(tags.primaries.as_deref(), Some("bt2020"));
    assert_eq!(tags.range.as_deref(), Some("pc"));
}

#[test]
fn the_output_tags_follow_the_target_not_the_source() {
    let source = source_tags(
        SourceKind::Pq,
        false,
        false,
        &ColorTags::default(),
        "limited",
    );

    let sdr = target_tags(SourceKind::Pq, Output::Sdr, &source, "limited");
    assert_eq!(sdr.primaries.as_deref(), Some("bt709"));
    assert_eq!(sdr.transfer.as_deref(), Some("bt709"));

    let hdr = target_tags(SourceKind::Pq, Output::Hdr, &source, "limited");
    assert_eq!(hdr.transfer.as_deref(), Some("smpte2084"));

    let developed = target_tags(SourceKind::Log, Output::Hdr, &ColorTags::default(), "full");
    assert_eq!(developed.transfer.as_deref(), Some("arib-std-b67"));
    assert_eq!(developed.range.as_deref(), Some("pc"));
}

#[test]
fn a_rgb_output_names_the_matrix_it_becomes() {
    let source = source_tags(
        SourceKind::Sdr,
        true,
        false,
        &ColorTags::default(),
        "limited",
    );
    assert_eq!(source.space.as_deref(), Some("gbr"));

    let sdr = target_tags(SourceKind::Sdr, Output::Sdr, &source, "limited");
    assert_eq!(sdr.space.as_deref(), Some("bt709"));

    let hdr = target_tags(SourceKind::Sdr, Output::Hdr, &source, "limited");
    assert_eq!(hdr.space.as_deref(), Some("bt2020nc"));
}

#[test]
fn a_profile_five_stream_keeps_its_own_surface_name() {
    let tags = source_tags(
        SourceKind::Pq,
        false,
        true,
        &ColorTags::default(),
        "limited",
    );
    assert_eq!(tags.space.as_deref(), Some("ipt-c2"));

    // The RGB rule is the last word, exactly as the host ordered the two overrides.
    let rgb = source_tags(SourceKind::Pq, true, true, &ColorTags::default(), "limited");
    assert_eq!(rgb.space.as_deref(), Some("gbr"));
}
