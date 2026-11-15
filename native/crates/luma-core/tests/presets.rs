//! Preset table tests, run against the shipped `Assets/recommended-presets.json`
//! so a registry change and an asset change cannot drift apart.

use luma_core::domain::{Codec, Purpose, Variant};
use luma_core::presets::{AudioDefault, PresetRegistry, SampleRate};

const ASSET: &str = include_str!("../../../../Assets/recommended-presets.json");

fn registry() -> PresetRegistry {
    PresetRegistry::parse(ASSET).expect("shipped preset tables parse")
}

#[test]
fn qsv_share_baseline_is_preserved() {
    let registry = registry();
    let preset = registry
        .video(Purpose::Share, Codec::Hevc, Variant::Standard)
        .expect("share HEVC");
    assert_eq!(preset.selector, "share/hevc");
    assert_eq!(preset.encoder, "hevc_qsv");
    assert_eq!(preset.preset, "4");
    assert_eq!(preset.quality, 24);
    assert_eq!(preset.async_depth, Some(4));

    let avc = registry
        .video(Purpose::Share, Codec::Avc, Variant::Standard)
        .expect("share AVC");
    assert_eq!(avc.quality, 22);
    let av1 = registry
        .video(Purpose::Share, Codec::Av1, Variant::Standard)
        .expect("share AV1");
    assert_eq!(av1.quality, 26);
}

#[test]
fn software_archive_keeps_precision_and_options() {
    let registry = registry();
    let hevc = registry
        .video(Purpose::Archive, Codec::Hevc, Variant::Standard)
        .expect("archive HEVC");
    assert_eq!(hevc.encoder, "libx265");
    assert_eq!(hevc.declared_pixel_format.as_deref(), Some("yuv420p10le"));
    assert_eq!(
        hevc.parameters
            .get("rdoq-level")
            .and_then(|value| value.as_i64()),
        Some(2)
    );

    let av1 = registry
        .video(Purpose::Archive, Codec::Av1, Variant::Standard)
        .expect("archive AV1");
    assert_eq!(av1.encoder, "libsvtav1");
    assert_eq!(av1.tune, Some(0));
    assert_eq!(av1.lookahead, Some(0));
}

#[test]
fn fast_variant_is_limited_to_software_archive_av1() {
    let registry = registry();
    let fast = registry
        .video(Purpose::Archive, Codec::Av1, Variant::Fast)
        .expect("standby profile");
    assert_eq!(fast.selector, "archive_fast_av1");
    assert_eq!(fast.preset, "6");
    assert_eq!(fast.lookahead, Some(4));

    assert_eq!(
        registry
            .video(Purpose::Share, Codec::Av1, Variant::Fast)
            .expect_err("fast is archive-only")
            .code(),
        "preset.variant"
    );
    assert_eq!(
        registry
            .video(Purpose::Archive, Codec::Hevc, Variant::Fast)
            .expect_err("fast is AV1-only")
            .code(),
        "preset.variant"
    );
}

#[test]
fn audio_defaults_follow_purpose_and_codec() {
    let registry = registry();
    assert_eq!(
        registry
            .audio(Purpose::Archive, Codec::Hevc)
            .expect("archive"),
        AudioDefault::Copy
    );
    assert_eq!(
        registry
            .audio(Purpose::Share, Codec::Hevc)
            .expect("share HEVC"),
        AudioDefault::Encode("aac".to_owned())
    );
    assert_eq!(
        registry
            .audio(Purpose::Share, Codec::Av1)
            .expect("share AV1"),
        AudioDefault::Encode("opus".to_owned())
    );
}

#[test]
fn lossy_audio_presets_expose_their_measured_values() {
    let registry = registry();
    let aac = registry.audio_preset("aac").expect("aac preset");
    assert_eq!(aac.encoder, "aac");
    assert_eq!(aac.bitrate_kbps, 256);
    assert_eq!(aac.channels, 2);
    assert_eq!(aac.sample_rate, SampleRate::Source);

    let opus = registry.audio_preset("opus").expect("opus preset");
    assert_eq!(opus.bitrate_kbps, 192);
    assert_eq!(opus.channels, 2);
    assert_eq!(opus.sample_rate, SampleRate::Fixed(48_000));
    assert_eq!(opus.vbr.as_deref(), Some("on"));
    assert_eq!(opus.compression_level, Some(10));

    assert_eq!(
        registry
            .audio_preset("vorbis")
            .expect_err("unknown preset")
            .code(),
        "preset.unknown"
    );
}

#[test]
fn catalog_lists_every_selectable_preset_once() {
    let catalog = registry().catalog();
    let selectors: Vec<&str> = catalog
        .iter()
        .map(|preset| preset.selector.as_str())
        .collect();
    assert_eq!(
        selectors,
        vec![
            "share/avc",
            "share/hevc",
            "share/av1",
            "archive/avc",
            "archive/hevc",
            "archive/av1",
            "archive_fast_av1",
        ]
    );
}

#[test]
fn malformed_documents_are_rejected_with_a_stable_code() {
    assert_eq!(
        PresetRegistry::parse("{}").expect_err("no tables").code(),
        "preset.unknown"
    );
    assert_eq!(
        PresetRegistry::parse("not json")
            .expect_err("not json")
            .code(),
        "preset.invalid"
    );
}
