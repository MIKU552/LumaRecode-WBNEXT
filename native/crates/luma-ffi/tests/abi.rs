//! Boundary tests: envelope shape, ownership and failure codes.

use std::ffi::{CStr, CString};

use lumacore::{luma_abi_version, luma_call_json, luma_core_version, luma_string_free};
use serde_json::Value;

fn call(op: &str, request: &str) -> Value {
    let op = CString::new(op).expect("op");
    let request = CString::new(request).expect("request");
    // SAFETY: both inputs outlive the call and are NUL-terminated.
    let response = unsafe { luma_call_json(op.as_ptr(), request.as_ptr()) };
    assert!(!response.is_null(), "allocation must succeed");
    // SAFETY: the library returned an owned NUL-terminated string.
    let text = unsafe { CStr::from_ptr(response) }
        .to_str()
        .expect("UTF-8 envelope")
        .to_owned();
    // SAFETY: single free of the returned pointer.
    unsafe { luma_string_free(response) };
    serde_json::from_str(&text).expect("envelope parses")
}

fn data(op: &str, request: &str) -> Value {
    let envelope = call(op, request);
    assert_eq!(envelope["ok"], Value::Bool(true), "{envelope}");
    envelope["data"].clone()
}

fn error_code(op: &str, request: &str) -> String {
    let envelope = call(op, request);
    assert_eq!(envelope["ok"], Value::Bool(false), "{envelope}");
    envelope["error"]["code"]
        .as_str()
        .expect("code string")
        .to_owned()
}

#[test]
fn version_pointers_are_stable() {
    assert_eq!(luma_abi_version(), 1);
    // SAFETY: static storage documented by the header.
    let version = unsafe { CStr::from_ptr(luma_core_version()) };
    assert_eq!(version.to_str().expect("UTF-8"), "0.1.0");
}

#[test]
fn core_info_lists_every_operation() {
    let info = data("core.info", "{}");
    assert_eq!(info["abi"], Value::from(1));
    let operations = info["operations"].as_array().expect("operation list");
    assert_eq!(operations.len(), 10);
    assert!(operations.iter().any(|value| value == "plan.build"));
}

#[test]
fn describe_reports_source_facts_and_choices() {
    let source = r#"{"pixel_format":"yuv420p","components":[8,8,8],"kind":"sdr"}"#;
    let described = data(
        "policy.describe",
        &format!(r#"{{"source":{source},"options":{{"purpose":"archive","codec":"hevc"}}}}"#),
    );
    assert_eq!(described["source"]["sampling"], "420");
    assert_eq!(described["source"]["depth"], 8);
    assert_eq!(described["source"]["range"], "limited");
    assert_eq!(
        described["choices"]["samplings"],
        serde_json::json!(["420"])
    );
    assert_eq!(described["choices"]["depths"], serde_json::json!([8]));
    assert_eq!(described["choices"]["depth"], Value::from(8));
    assert_eq!(described["audio"]["flac"], Value::Bool(false));
}

#[test]
fn resolve_returns_a_concrete_format() {
    let source = r#"{"pixel_format":"p010le","components":[10,10,10],"kind":"hlg"}"#;
    let resolved = data(
        "policy.resolve",
        &format!(
            r#"{{"source":{source},"options":{{"purpose":"share","codec":"hevc","output":"hdr","transform":"hdr-sdr","sampling":"auto"}}}}"#
        ),
    );
    // An HDR request keeps 10 bits; the transform only changes the target.
    assert_eq!(resolved["bit_depth"], Value::from(10));
    assert_eq!(resolved["pixel_format"], "yuv420p10le");
    assert_eq!(resolved["range_tag"], "tv");
    assert_eq!(resolved["container"], "mp4");
    assert_eq!(resolved["suffix"], ".mp4");
}

#[test]
fn validate_reports_stable_codes_instead_of_text() {
    let source = r#"{"pixel_format":"yuv420p","components":[8,8,8],"kind":"sdr"}"#;
    assert_eq!(
        error_code(
            "policy.validate",
            &format!(r#"{{"source":{source},"options":{{"output":"hdr"}}}}"#)
        ),
        "policy.hdr_depth"
    );
    assert_eq!(error_code("plan.build", "{}"), "ffi.invalid_request");
    assert_eq!(error_code("nope.op", "{}"), "ffi.unknown_operation");
    assert_eq!(error_code("core.info", "}"), "ffi.invalid_json");
}

#[test]
fn timeline_returns_a_window_with_seek_information() {
    let request = r#"{
        "frames":[{"pts":0,"duration":1000,"key_frame":true},
                  {"pts":1000,"duration":1000,"key_frame":false},
                  {"pts":2000,"duration":1000,"key_frame":true}],
        "time_base":"1/1000",
        "packet_indexed":true,
        "range":{"start":1,"end":3}
    }"#;
    let timeline = data("timeline.resolve", request);
    assert_eq!(timeline["count"], Value::from(3));
    assert_eq!(timeline["window"]["seek_frame"], Value::from(0));
    assert_eq!(timeline["window"]["count"], Value::from(2));
    let duration = timeline["window"]["duration"].as_f64().expect("duration");
    assert!((duration - 2.0).abs() < 1e-9);
}

#[test]
fn presets_resolve_uses_the_shipped_tables() {
    let document = include_str!("../../../../Assets/recommended-presets.json");
    let request = serde_json::json!({
        "document": document,
        "purpose": "archive",
        "codec": "hevc"
    })
    .to_string();
    let preset = data("presets.resolve", &request);
    assert_eq!(preset["encoder"], "libx265");
    assert_eq!(preset["quality"], Value::from(23));
    assert_eq!(preset["audio"]["mode"], "copy");

    let catalog = data(
        "presets.validate",
        &serde_json::json!({ "document": document }).to_string(),
    );
    assert_eq!(catalog["count"], Value::from(7));
}

#[test]
fn capability_selection_and_summary_agree() {
    let devices = r#"[
        {"id":"igpu","backend":"qsv","status":"available","encoders":[
            {"codec":"hevc","max_depth":10,"samplings":["420"],"hdr":true,"verification":"short_task"}]},
        {"id":"cpu","backend":"software","status":"available","encoders":[
            {"codec":"hevc","max_depth":12,"samplings":["420","444"],"hdr":true,"verification":"probed"}]}
    ]"#;
    let selection = data(
        "capability.select",
        &format!(
            r#"{{"devices":{devices},"requirement":{{"purpose":"share","codec":"hevc","output":"sdr","depth":8,"sampling":"420"}}}}"#
        ),
    );
    assert_eq!(selection["backend"], "qsv");
    assert_eq!(selection["fallback"], Value::Bool(false));

    let summary = data(
        "capability.summary",
        &format!(r#"{{"devices":{devices},"codec":"hevc"}}"#),
    );
    assert_eq!(summary["max_depth"], Value::from(12));
    assert_eq!(summary["backends"], serde_json::json!(["qsv", "software"]));
}

#[test]
fn plan_describes_transport_and_steps_without_ffmpeg_flags() {
    let source = r#"{"pixel_format":"p010le","components":[10,10,10],"kind":"hlg"}"#;
    let plan = data(
        "plan.build",
        &format!(
            r#"{{"source":{source},"options":{{"purpose":"share","codec":"hevc","output":"sdr","transform":"hdr-sdr"}},"backend":"qsv","frame_window":true}}"#
        ),
    );
    assert_eq!(plan["decode"], "software");
    assert_eq!(plan["transport"], "system_memory");
    let steps = plan["steps"].as_array().expect("steps");
    let kinds: Vec<&str> = steps
        .iter()
        .map(|step| step["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(
        kinds,
        vec![
            "decode",
            "tone_map",
            "convert_range",
            "encode",
            "mux",
            "verify"
        ]
    );
    assert!(
        plan["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .any(|note| note == "frame_accurate_decode")
    );
    // The plan carries no command line: that belongs to the backend.
    assert!(!plan.to_string().contains("-vf"));
}
