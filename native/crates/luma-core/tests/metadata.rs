use serde_json::json;

use luma_core::metadata::parse;

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
