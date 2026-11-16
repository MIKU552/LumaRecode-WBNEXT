use serde_json::json;

use luma_core::domain::{Codec, Output, SourceKind};
use luma_core::metadata::{HdrMetadataArguments, HdrMetadataRequest, encoder_metadata, parse};

#[test]
fn a_single_layer_dolby_vision_record_is_read() {
    let side = vec![json!({
        "side_data_type": "DOVI configuration record",
        "dv_profile": 8,
        "dv_bl_signal_compatibility_id": 4,
        "rpu_present_flag": 1,
        "bl_present_flag": 1,
        "el_present_flag": 0
    })];

    let metadata = parse(&side);
    let dolby = metadata.dolby_vision.expect("DOVI record");
    assert_eq!(dolby.profile, 8);
    assert!(dolby.is_supported_layer());
    assert!(metadata.present());
    assert!(!metadata.hdr10_plus);
}

#[test]
fn a_dual_layer_record_is_not_supported() {
    let side = vec![json!({
        "side_data_type": "DOVI configuration record",
        "dv_profile": 7,
        "dv_bl_signal_compatibility_id": 0,
        "rpu_present_flag": 1,
        "bl_present_flag": 1,
        "el_present_flag": 1
    })];

    let dolby = parse(&side).dolby_vision.expect("DOVI record");
    assert!(!dolby.is_supported_layer());
}

#[test]
fn hdr10_plus_is_recognized_by_either_name() {
    let smpte = parse(&[json!({ "side_data_type": "HDR Dynamic Metadata SMPTE2094-40" })]);
    assert!(smpte.hdr10_plus);
    let plus = parse(&[json!({ "side_data_type": "HDR10+ dynamic metadata" })]);
    assert!(plus.hdr10_plus);
    assert!(!parse(&[]).present());
}

fn display() -> serde_json::Value {
    json!({
        "side_data_type": "Mastering display metadata",
        "green_x": "13250/50000",
        "green_y": "34500/50000",
        "blue_x": "7500/50000",
        "blue_y": "3000/50000",
        "red_x": "34000/50000",
        "red_y": "16000/50000",
        "white_point_x": "15635/50000",
        "white_point_y": "16450/50000",
        "max_luminance": "10000000/10000",
        "min_luminance": "50/10000"
    })
}

fn light() -> serde_json::Value {
    json!({
        "side_data_type": "Content light level metadata",
        "max_content": 1000,
        "max_average": 400
    })
}

fn request<'a>(codec: Codec, side: &'a [serde_json::Value]) -> HdrMetadataRequest<'a> {
    HdrMetadataRequest {
        codec,
        output: Output::Hdr,
        source_kind: SourceKind::Pq,
        preserve_dynamic: false,
        dolby_vision_profile: None,
        hdr10_plus: false,
        side_data: side,
    }
}

/// Parameters rendered as `key=value` pairs, in the order the encoder sees them.
fn parameters(arguments: &HdrMetadataArguments) -> String {
    arguments
        .parameters
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(":")
}

#[test]
fn x265_receives_integer_mastering_values() {
    let side = [display(), light()];
    let arguments = encoder_metadata(&request(Codec::Hevc, &side));

    let master =
        "master-display=G(13250,34500)B(7500,3000)R(34000,16000)WP(15635,16450)L(10000000,50)";
    let flags = "hdr10=1:hdr10-opt=1:repeat-headers=1";
    assert_eq!(arguments.appended.len(), 2);
    assert_eq!(arguments.appended[0], master);
    assert_eq!(arguments.appended[1], "max-cll=1000,400");
    assert_eq!(parameters(&arguments), flags);
}

#[test]
fn svt_av1_receives_eight_decimal_mastering_values() {
    let side = [display(), light()];
    let arguments = encoder_metadata(&request(Codec::Av1, &side));

    let master = "mastering-display=G(0.26500000,0.69000000)B(0.15000000,0.06000000)R(0.68000000,0.32000000)WP(0.31270000,0.32900000)L(1000.00000000,0.00500000)";
    assert_eq!(arguments.appended.len(), 2);
    assert_eq!(arguments.appended[0], master);
    assert_eq!(arguments.appended[1], "content-light=1000,400");
    // AV1 has no equivalent of the HEVC repeat-header flags.
    assert!(arguments.parameters.is_empty());
}

#[test]
fn dynamic_metadata_adds_the_rate_control_floor_and_the_payload() {
    let side = [display(), light()];
    let mut metadata = request(Codec::Hevc, &side);
    metadata.preserve_dynamic = true;
    metadata.dolby_vision_profile = Some(8);
    metadata.hdr10_plus = true;

    let expected = "vbv-maxrate=100000:vbv-bufsize=100000:dhdr10-info=hdr10plus.json:hdr10=1:hdr10-opt=1:repeat-headers=1";
    assert_eq!(parameters(&encoder_metadata(&metadata)), expected);
}

#[test]
fn an_sdr_target_keeps_only_the_dynamic_payload() {
    let side = [display(), light()];
    let mut metadata = request(Codec::Hevc, &side);
    metadata.output = Output::Sdr;
    metadata.preserve_dynamic = true;
    metadata.hdr10_plus = true;

    let arguments = encoder_metadata(&metadata);
    assert!(arguments.appended.is_empty());
    assert_eq!(parameters(&arguments), "dhdr10-info=hdr10plus.json");
}

#[test]
fn an_incomplete_display_is_skipped_rather_than_zeroed() {
    let side = [json!({
        "side_data_type": "Mastering display metadata",
        "green_x": "13250/50000"
    })];

    assert!(
        encoder_metadata(&request(Codec::Hevc, &side))
            .appended
            .is_empty()
    );
}

#[test]
fn an_overridden_interpretation_drops_the_static_tokens() {
    let side = [display(), light()];
    let mut metadata = request(Codec::Hevc, &side);
    metadata.source_kind = SourceKind::Log;

    assert!(encoder_metadata(&metadata).appended.is_empty());
}
