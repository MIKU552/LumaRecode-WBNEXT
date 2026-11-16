use luma_core::probe::summarize;

const HLG_CLIP: &str = r#"{
  "streams": [
    {
      "codec_type": "video",
      "codec_name": "hevc",
      "pix_fmt": "yuv420p10le",
      "width": 3840,
      "height": 2160,
      "color_range": "tv",
      "color_transfer": "arib-std-b67",
      "bits_per_raw_sample": "10",
      "side_data_list": [
        { "side_data_type": "Display Matrix", "rotation": -90 },
        { "side_data_type": "HDR Dynamic Metadata SMPTE2094-40" }
      ]
    },
    {
      "codec_type": "audio",
      "codec_name": "aac",
      "sample_fmt": "fltp",
      "channels": 2,
      "channel_layout": "stereo"
    }
  ]
}"#;

#[test]
fn reads_video_and_audio_facts() {
    let summary = summarize(HLG_CLIP).unwrap();

    assert_eq!(summary.width, 3840);
    assert_eq!(summary.height, 2160);
    assert_eq!(summary.rotation, -90);
    assert!(summary.hdr10_plus);
    assert_eq!(summary.source.kind.as_deref(), Some("hlg"));
    assert_eq!(summary.source.pixel_format.as_deref(), Some("yuv420p10le"));
    assert_eq!(summary.source.bits_per_raw_sample, Some(10));
    assert_eq!(summary.source.color_range.as_deref(), Some("tv"));
    assert_eq!(summary.source.audio.len(), 1);
    assert_eq!(summary.source.audio[0].codec_name.as_deref(), Some("aac"));
    assert_eq!(summary.source.audio[0].channels, Some(2));
}

#[test]
fn dolby_vision_profile_is_reported() {
    let document = r#"{
      "streams": [
        {
          "codec_type": "video",
          "pix_fmt": "yuv420p10le",
          "side_data_list": [
            { "side_data_type": "DOVI configuration record", "dv_profile": 8 }
          ]
        }
      ]
    }"#;

    assert_eq!(summarize(document).unwrap().dolby_vision_profile, Some(8));
}

#[test]
fn depth_falls_back_to_the_pixel_format_suffix() {
    let document = r#"{
      "streams": [{ "codec_type": "video", "pix_fmt": "yuv422p12le" }]
    }"#;

    let summary = summarize(document).unwrap();
    assert_eq!(summary.source.bits_per_raw_sample, Some(12));
    assert_eq!(summary.rotation, 0);
    assert!(!summary.hdr10_plus);
}

#[test]
fn a_missing_transfer_function_stays_unknown() {
    // ffprobe omits the field entirely when the container carries no tag; the
    // application must ask the user rather than assume SDR.
    let document = r#"{
      "streams": [{ "codec_type": "video", "pix_fmt": "yuv420p" }]
    }"#;

    let summary = summarize(document).unwrap();
    assert_eq!(summary.source.bits_per_raw_sample, Some(8));
    assert_eq!(summary.source.kind.as_deref(), Some("unknown"));
    assert!(summary.source.audio.is_empty());
}

#[test]
fn a_tagged_transfer_function_becomes_sdr() {
    let document = r#"{
      "streams": [{ "codec_type": "video", "pix_fmt": "yuv420p", "color_transfer": "bt709" }]
    }"#;

    assert_eq!(
        summarize(document).unwrap().source.kind.as_deref(),
        Some("sdr")
    );
}

#[test]
fn a_document_without_a_video_stream_is_refused() {
    let document = r#"{ "streams": [{ "codec_type": "audio", "codec_name": "aac" }] }"#;
    assert_eq!(summarize(document).unwrap_err().code(), "probe.no_video");
}

#[test]
fn invalid_json_is_reported_as_a_probe_error() {
    assert_eq!(summarize("{").unwrap_err().code(), "probe.json");
}

#[test]
fn reads_a_document_captured_from_ffprobe() {
    // Captured in CI from the synthetic clip: the container carries no colour tags,
    // so the transfer function is absent rather than "bt709".
    let summary = summarize(include_str!("fixtures/ffprobe-testsrc.json")).unwrap();
    assert_eq!(summary.width, 320);
    assert_eq!(summary.height, 240);
    assert_eq!(summary.source.bits_per_raw_sample, Some(8));
    assert_eq!(summary.source.kind.as_deref(), Some("unknown"));
    assert!(summary.source.audio.is_empty());
    assert!(!summary.hdr10_plus);
    assert_eq!(summary.rotation, 0);
}
