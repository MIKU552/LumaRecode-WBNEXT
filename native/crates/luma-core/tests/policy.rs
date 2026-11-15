//! Rule-level tests for the output policy.
//!
//! These cases mirror the Windows baseline table in `Core/OutputPolicy.cs`;
//! when a rule changes, both implementations must change together.

use luma_core::domain::{
    AudioChoice, Codec, ColorRange, ContainerChoice, DepthChoice, DynamicHdrMode, EncodeOptions,
    LogProfile, Output, Purpose, RangeChoice, ResolvedFormat, Sampling, SamplingChoice, SourceKind,
    Transform, Variant,
};
use luma_core::policy;
use luma_core::source::{
    AudioStream, DolbyVisionConfig, DynamicHdrInfo, PictureFormat, SourceFormat,
};

fn picture(pixel: &str) -> PictureFormat {
    PictureFormat {
        pixel_format: pixel.to_owned(),
        ..PictureFormat::default()
    }
}

fn source(pixel: &str) -> SourceFormat {
    SourceFormat {
        picture: picture(pixel),
        kind: SourceKind::Sdr,
        profile: None,
        raw: false,
        raw_decoder_supported: true,
        raw_decoder_version_supported: true,
        display_matrix: false,
        dynamic_hdr: DynamicHdrInfo::default(),
        audio: Vec::new(),
    }
}

fn ten_bit_source(pixel: &str) -> SourceFormat {
    let mut source = source(pixel);
    source.picture.components = vec![10, 10, 10];
    source
}

fn track(codec: &str, sample_format: &str, depth: i32, channels: i32, layout: &str) -> AudioStream {
    AudioStream {
        codec_name: codec.to_owned(),
        profile: String::new(),
        sample_format: sample_format.to_owned(),
        bits_per_raw_sample: depth,
        bits_per_sample: 0,
        channels,
        channel_layout: Some(layout.to_owned()),
    }
}

fn archive_hevc() -> EncodeOptions {
    EncodeOptions {
        purpose: Purpose::Archive,
        codec: Codec::Hevc,
        ..EncodeOptions::default()
    }
}

#[test]
fn sampling_is_read_from_pixel_format_and_layout_flags() {
    assert_eq!(policy::source_sampling(&source("nv12")), Sampling::Yuv420);
    assert_eq!(policy::source_sampling(&source("p010le")), Sampling::Yuv420);
    assert_eq!(
        policy::source_sampling(&source("yuv422p10le")),
        Sampling::Yuv422
    );
    assert_eq!(
        policy::source_sampling(&source("yuv444p12le")),
        Sampling::Yuv444
    );
    assert_eq!(policy::source_sampling(&source("rgb24")), Sampling::Yuv444);
    assert_eq!(policy::source_sampling(&source("gray10le")), Sampling::Gray);

    // An explicit chroma shift wins over name guessing, including 4:4:0.
    let mut shifted = source("mystery420");
    shifted.picture.log2_chroma_w = Some(0);
    shifted.picture.log2_chroma_h = Some(1);
    assert_eq!(policy::source_sampling(&shifted), Sampling::Yuv440);

    // Bayer is not chroma subsampling; it must not be treated as 4:2:0.
    let mut bayer = source("bayer_rggb16le");
    bayer.picture.bayer = true;
    assert_eq!(policy::source_sampling(&bayer), Sampling::Unknown);
}

#[test]
fn depth_ignores_alpha_and_prefers_declared_components() {
    let mut rgba = source("rgba64le");
    rgba.picture.alpha = true;
    rgba.picture.components = vec![16, 16, 16, 16];
    assert_eq!(policy::source_depth(&rgba), 16);

    let ten = ten_bit_source("p010le");
    assert_eq!(policy::source_depth(&ten), 10);

    // Name-only descriptors fall back to the pixel format suffix.
    let mut unnamed = source("yuv420p12le");
    unnamed.picture.bits_per_sample = 0;
    assert_eq!(policy::source_depth(&unnamed), 12);

    let mut eight = source("yuv420p");
    eight.picture.components = vec![8, 8, 8];
    assert_eq!(policy::source_depth(&eight), 8);
}

#[test]
fn sampling_choices_never_increase_chroma_resolution() {
    // 4:2:0 may only be delivered as 4:2:0: widening adds no real chroma detail.
    assert_eq!(
        policy::sampling_choices(&source("yuv420p")),
        vec![Sampling::Yuv420]
    );
    // A 4:2:2 source may only go down to 4:2:0; 4:4:4 would be a fake upgrade.
    assert_eq!(
        policy::sampling_choices(&source("yuv422p")),
        vec![Sampling::Yuv420, Sampling::Yuv422]
    );
    assert_eq!(
        policy::sampling_choices(&source("yuv444p12le")),
        vec![Sampling::Yuv420, Sampling::Yuv422, Sampling::Yuv444]
    );
    assert_eq!(
        policy::sampling_choices(&source("gray10le")),
        vec![Sampling::Gray, Sampling::Yuv420]
    );

    let mut bayer = source("bayer_rggb16le");
    bayer.picture.bayer = true;
    assert!(policy::sampling_choices(&bayer).is_empty());
}

#[test]
fn depth_choices_intersect_source_precision_and_encoder_limits() {
    assert_eq!(
        policy::depth_choices(10, Purpose::Share, Codec::Avc, Output::Sdr),
        vec![8]
    );
    assert_eq!(
        policy::depth_choices(10, Purpose::Archive, Codec::Hevc, Output::Sdr),
        vec![8, 10]
    );
    assert_eq!(
        policy::depth_choices(12, Purpose::Archive, Codec::Hevc, Output::Sdr),
        vec![8, 10, 12]
    );
    // HDR cannot start below 10 bits, so an 8-bit source has no HDR path.
    assert!(policy::depth_choices(8, Purpose::Share, Codec::Hevc, Output::Hdr).is_empty());
}

#[test]
fn default_depth_follows_purpose_then_output() {
    let choices = [8, 10, 12];
    assert_eq!(
        policy::default_depth(&choices, Purpose::Archive, Output::Sdr),
        Some(12)
    );
    assert_eq!(
        policy::default_depth(&choices, Purpose::Share, Output::Hdr),
        Some(10)
    );
    assert_eq!(
        policy::default_depth(&choices, Purpose::Share, Output::Sdr),
        Some(8)
    );
    assert_eq!(
        policy::default_depth(&[], Purpose::Share, Output::Sdr),
        None
    );
}

#[test]
fn default_sampling_prefers_share_420_and_archive_source() {
    assert_eq!(
        policy::default_sampling(&source("yuv444p"), Purpose::Share, Codec::Hevc),
        SamplingChoice::Fixed(Sampling::Yuv420)
    );
    assert_eq!(
        policy::default_sampling(&source("yuv444p"), Purpose::Archive, Codec::Hevc),
        SamplingChoice::Source
    );
    assert_eq!(
        policy::default_sampling(&source("rgb24"), Purpose::Archive, Codec::Hevc),
        SamplingChoice::Fixed(Sampling::Yuv444)
    );
    // Transparent sources cannot keep a layout that would drop alpha silently.
    let mut alpha = source("yuva420p");
    alpha.picture.alpha = true;
    assert_eq!(
        policy::default_sampling(&alpha, Purpose::Archive, Codec::Hevc),
        SamplingChoice::Fixed(Sampling::Yuv420)
    );
}

#[test]
fn source_range_prefers_tags_then_pixel_format_then_camera_profile() {
    let mut tagged = source("yuv420p10le");
    tagged.picture.color_range = Some("pc".to_owned());
    assert_eq!(policy::source_range(&tagged, None), ColorRange::Full);

    let mut limited = source("yuv420p10le");
    limited.picture.color_range = Some("tv".to_owned());
    assert_eq!(policy::source_range(&limited, None), ColorRange::Limited);

    assert_eq!(
        policy::source_range(&source("yuvj420p"), None),
        ColorRange::Full
    );
    assert_eq!(
        policy::source_range(&source("rgb24"), None),
        ColorRange::Full
    );

    let mut milog = source("yuv420p10le");
    milog.profile = Some(LogProfile::MiLog);
    assert_eq!(policy::source_range(&milog, None), ColorRange::Full);

    assert_eq!(
        policy::source_range(&source("yuv420p10le"), Some(ColorRange::Full)),
        ColorRange::Full
    );
}

#[test]
fn flac_accepts_integer_lossless_and_rejects_float() {
    let mut ok = source("yuv420p");
    ok.audio = vec![track("pcm_s24le", "s32", 24, 2, "stereo")];
    let tracks = policy::flac_tracks(&ok).expect("integer PCM is eligible");
    assert_eq!(tracks[0].depth, 24);
    assert_eq!(tracks[0].channels, 2);

    let mut float = source("yuv420p");
    float.audio = vec![track("pcm_f32le", "flt", 0, 2, "stereo")];
    let error = policy::flac_tracks(&float).expect_err("float PCM is not lossless");
    assert_eq!(error.code(), "audio.not_lossless");

    let mut six = source("yuv420p");
    six.audio = vec![track("pcm_s24le", "s32", 24, 6, "5.1")];
    assert!(policy::flac_tracks(&six).is_ok());

    let mut mismatched = source("yuv420p");
    mismatched.audio = vec![track("pcm_s24le", "s32", 24, 6, "6.1")];
    let error = policy::flac_tracks(&mismatched).expect_err("layout must match channel count");
    assert_eq!(error.code(), "audio.channel_layout");

    let mut empty = source("yuv420p");
    empty.audio.clear();
    assert_eq!(
        policy::flac_tracks(&empty).expect_err("no tracks").code(),
        "audio.empty"
    );
}

#[test]
fn resolve_rejects_hdr_for_avc() {
    let options = EncodeOptions {
        codec: Codec::Avc,
        output: Output::Hdr,
        ..EncodeOptions::default()
    };
    let error = policy::resolve(&options, &ten_bit_source("p010le")).expect_err("AVC has no HDR");
    assert_eq!(error.code(), "options.codec");
}

#[test]
fn resolve_keeps_archive_range_and_share_limited() {
    let mut full_range = ten_bit_source("p010le");
    full_range.picture.color_range = Some("pc".to_owned());

    let resolved = policy::resolve(&archive_hevc(), &full_range).expect("archive resolves");
    assert_eq!(resolved.format.color_range, ColorRange::Full);
    assert_eq!(resolved.format.bit_depth, 10);
    assert_eq!(resolved.format.sampling, Sampling::Yuv420);

    let share = EncodeOptions::default();
    let resolved = policy::resolve(&share, &full_range).expect("share resolves");
    assert_eq!(resolved.format.color_range, ColorRange::Limited);
    assert_eq!(resolved.format.bit_depth, 8);
}

#[test]
fn resolve_rejects_sampling_above_source_and_software_limits() {
    let mut options = archive_hevc();
    options.sampling = SamplingChoice::Fixed(Sampling::Yuv444);
    assert!(policy::resolve(&options, &source("yuv420p")).is_err());

    // 4:2:2 archive needs software HEVC/AVC; AV1 cannot carry it here.
    let mut av1 = archive_hevc();
    av1.codec = Codec::Av1;
    av1.sampling = SamplingChoice::Fixed(Sampling::Yuv422);
    let error = policy::resolve(&av1, &source("yuv422p")).expect_err("AV1 is 4:2:0 here");
    assert_eq!(error.code(), "policy.sampling_software");

    // A share request may not keep 4:2:2 at all.
    let share = EncodeOptions {
        sampling: SamplingChoice::Fixed(Sampling::Yuv422),
        ..EncodeOptions::default()
    };
    assert_eq!(
        policy::resolve(&share, &source("yuv422p"))
            .expect_err("share stays 420")
            .code(),
        "policy.sampling_software"
    );
}

#[test]
fn resolved_format_names_pixel_formats() {
    let format = ResolvedFormat {
        sampling: Sampling::Yuv420,
        color_range: ColorRange::Limited,
        bit_depth: 10,
    };
    assert_eq!(format.pixel_format(), "yuv420p10le");
    assert_eq!(format.range_tag(), "tv");

    let gray = ResolvedFormat {
        sampling: Sampling::Gray,
        color_range: ColorRange::Full,
        bit_depth: 8,
    };
    assert_eq!(gray.pixel_format(), "gray");
    assert_eq!(gray.range_tag(), "pc");
}

#[test]
fn validate_requires_a_curve_for_log_transforms() {
    let mut log = ten_bit_source("yuv420p10le");
    log.kind = SourceKind::Log;
    log.profile = Some(LogProfile::Pocket3);
    let options = EncodeOptions {
        transform: Transform::LogHdr,
        output: Output::Hdr,
        profile: Some(LogProfile::Pocket3),
        ..EncodeOptions::default()
    };
    assert!(policy::validate(&options, &log).is_ok());

    let missing = EncodeOptions {
        profile: None,
        ..options.clone()
    };
    assert_eq!(
        policy::validate(&missing, &log)
            .expect_err("curve required")
            .code(),
        "transform.profile_required"
    );

    // Log needs the matching target: log-sdr cannot produce an HDR file.
    let mismatched = EncodeOptions {
        profile: Some(LogProfile::Pocket3),
        transform: Transform::LogSdr,
        output: Output::Hdr,
        ..EncodeOptions::default()
    };
    assert_eq!(
        policy::validate(&mismatched, &log)
            .expect_err("target mismatch")
            .code(),
        "transform.target_mismatch"
    );
}

#[test]
fn validate_rejects_unexplained_transfer_changes() {
    let sdr = source("yuv420p");
    let hdr_without_transform = EncodeOptions {
        output: Output::Hdr,
        ..EncodeOptions::default()
    };
    // An 8-bit source has no HDR path at all, so the precision rule fires first.
    assert_eq!(
        policy::validate(&hdr_without_transform, &sdr)
            .expect_err("HDR needs at least 10 bits")
            .code(),
        "policy.hdr_depth"
    );
    let sdr_to_hdr = EncodeOptions {
        output: Output::Hdr,
        ..archive_hevc()
    };
    assert_eq!(
        policy::validate(&sdr_to_hdr, &ten_bit_source("p010le"))
            .expect_err("SDR to HDR needs a transform")
            .code(),
        "transform.sdr_to_hdr"
    );

    let mut hlg = ten_bit_source("p010le");
    hlg.kind = SourceKind::Hlg;
    let sdr_output = EncodeOptions::default();
    assert_eq!(
        policy::validate(&sdr_output, &hlg)
            .expect_err("HLG to SDR needs tone mapping")
            .code(),
        "transform.hdr_to_sdr_required"
    );
    let tonemap = EncodeOptions {
        transform: Transform::HdrSdr,
        ..EncodeOptions::default()
    };
    assert!(policy::validate(&tonemap, &hlg).is_ok());
}

#[test]
fn dynamic_hdr_preservation_requires_the_archive_combination() {
    let mut hdr10plus = ten_bit_source("p010le");
    hdr10plus.kind = SourceKind::Hlg;
    hdr10plus.dynamic_hdr.hdr10_plus = true;

    let archive = EncodeOptions {
        output: Output::Hdr,
        ..archive_hevc()
    };
    assert!(policy::validate(&archive, &hdr10plus).is_ok());
    assert!(policy::preserve_dynamic_hdr(&archive, &hdr10plus));

    let share = EncodeOptions {
        output: Output::Hdr,
        ..EncodeOptions::default()
    };
    assert_eq!(
        policy::validate(&share, &hdr10plus)
            .expect_err("share cannot preserve")
            .code(),
        "dvhdr.needs_software_archive"
    );

    let av1 = EncodeOptions {
        codec: Codec::Av1,
        output: Output::Hdr,
        ..archive_hevc()
    };
    assert_eq!(
        policy::validate(&av1, &hdr10plus)
            .expect_err("HDR10+ needs HEVC")
            .code(),
        "dvhdr.hdr10plus_codec"
    );

    let standard = EncodeOptions {
        dynamic_hdr: DynamicHdrMode::Standard,
        output: Output::Hdr,
        ..EncodeOptions::default()
    };
    assert!(!policy::preserve_dynamic_hdr(&standard, &hdr10plus));
}

#[test]
fn dolby_vision_layers_outside_the_supported_set_are_rejected() {
    let mut dovi = ten_bit_source("p010le");
    dovi.kind = SourceKind::Pq;
    dovi.dynamic_hdr.dolby_vision = Some(DolbyVisionConfig {
        profile: 8,
        bl_signal_compatibility_id: 1,
        rpu_present_flag: 1,
        bl_present_flag: 1,
        el_present_flag: 1,
    });
    let options = EncodeOptions {
        output: Output::Hdr,
        ..archive_hevc()
    };
    assert_eq!(
        policy::validate(&options, &dovi)
            .expect_err("enhancement layer")
            .code(),
        "dvhdr.unsupported_layer"
    );
}

#[test]
fn flac_container_and_variant_rules_follow_the_request() {
    let mut flac_mp4 = archive_hevc();
    flac_mp4.audio = AudioChoice::Flac;
    flac_mp4.container = ContainerChoice::Mp4;
    let mut audio = source("yuv420p");
    audio.audio = vec![track("pcm_s24le", "s32", 24, 2, "stereo")];
    assert_eq!(
        policy::validate(&flac_mp4, &audio)
            .expect_err("MP4 cannot hold FLAC")
            .code(),
        "options.container"
    );

    let mut fast = archive_hevc();
    fast.variant = Variant::Fast;
    assert_eq!(
        policy::validate(&fast, &source("yuv420p"))
            .expect_err("fast is AV1 only")
            .code(),
        "preset.variant"
    );
}

#[test]
fn depth_choice_outside_the_allowed_set_is_rejected() {
    let options = EncodeOptions {
        bit_depth: DepthChoice::Bits(12),
        ..archive_hevc()
    };
    assert_eq!(
        policy::validate(&options, &source("yuv420p"))
            .expect_err("12-bit source required")
            .code(),
        "policy.depth_limit"
    );

    let mut ranged = archive_hevc();
    ranged.color_range = RangeChoice::Fixed(ColorRange::Full);
    assert!(policy::validate(&ranged, &source("yuv420p")).is_ok());
}
