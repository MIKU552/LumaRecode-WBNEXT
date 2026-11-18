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

/// The GOP length the core answers for one request.
fn gop(request: &str) -> Value {
    data("plan.gop", request)["frames"].clone()
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
    assert_eq!(operations.len(), 28);
    assert!(operations.iter().any(|value| value == "plan.build"));
    assert!(operations.iter().any(|value| value == "probe.summarize"));
    assert!(operations.iter().any(|value| value == "capability.backend"));
    assert!(
        operations
            .iter()
            .any(|value| value == "capability.supports")
    );
    assert!(operations.iter().any(|value| value == "cache.plan"));
    assert!(operations.iter().any(|value| value == "plan.input_map"));
    assert!(operations.iter().any(|value| value == "plan.color_args"));
    assert!(operations.iter().any(|value| value == "plan.encoder_args"));
    assert!(
        operations
            .iter()
            .any(|value| value == "plan.parameter_string")
    );
    assert!(operations.iter().any(|value| value == "plan.filter_chain"));
    assert!(operations.iter().any(|value| value == "capability.detect"));
    assert!(
        operations
            .iter()
            .any(|value| value == "capability.parameters")
    );
    assert!(operations.iter().any(|value| value == "plan.gop"));
    assert!(operations.iter().any(|value| value == "plan.hdr_metadata"));
    assert!(operations.iter().any(|value| value == "plan.color_filters"));
    assert!(
        operations
            .iter()
            .any(|value| value == "plan.render_filters")
    );
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
    assert_eq!(
        error_code("plan.gop", r#"{"share":true}"#),
        "ffi.invalid_request"
    );
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

#[test]
fn plan_gop_accepts_every_rate_a_probe_can_report() {
    // JSON hosts write 30.0 as 30, so the request accepts an integer token.
    let stream = r#"{"frame_rate":30,"share":true}"#;
    assert_eq!(gop(stream), Value::from(60));

    // Archive uses the eight-second cadence; motion photos use two.
    let archive = r#"{"frame_rate":29.97,"share":false}"#;
    assert_eq!(gop(archive), Value::from(240));
    let motion = r#"{"frame_rate":30,"share":false,"motion_photo":true}"#;
    assert_eq!(gop(motion), Value::from(60));

    // A half frame rounds away from zero, which is what the managed fallback does.
    let tie = r#"{"frame_rate":30.25,"share":true}"#;
    assert_eq!(gop(tie), Value::from(61));

    // Quick share stops at 240 frames; archive has no upper bound.
    let fast = r#"{"frame_rate":1000,"share":true}"#;
    assert_eq!(gop(fast), Value::from(240));
    let long = r#"{"frame_rate":1000,"share":false}"#;
    assert_eq!(gop(long), Value::from(8000));
}

#[test]
fn plan_hdr_metadata_spells_the_tokens_the_encoder_expects() {
    let side = r#"[{"side_data_type":"Mastering display metadata",
        "green_x":"13250/50000","green_y":"34500/50000",
        "blue_x":"7500/50000","blue_y":"3000/50000",
        "red_x":"34000/50000","red_y":"16000/50000",
        "white_point_x":"15635/50000","white_point_y":"16450/50000",
        "max_luminance":"10000000/10000","min_luminance":"50/10000"},
        {"side_data_type":"Content light level metadata","max_content":1000,"max_average":400}]"#;
    let request =
        format!(r#"{{"codec":"hevc","output":"hdr","source_kind":"pq","side_data":{side}}}"#);
    let arguments = data("plan.hdr_metadata", &request);

    assert_eq!(
        arguments["appended"][0],
        "master-display=G(13250,34500)B(7500,3000)R(34000,16000)WP(15635,16450)L(10000000,50)"
    );
    assert_eq!(arguments["appended"][1], "max-cll=1000,400");
    assert_eq!(arguments["parameters"][0]["key"], "hdr10");
    assert_eq!(arguments["parameters"][0]["value"], "1");
    assert_eq!(
        arguments["parameters"]
            .as_array()
            .expect("parameters")
            .len(),
        3
    );
}

#[test]
fn plan_hdr_metadata_keeps_dynamic_metadata_on_an_sdr_target() {
    let request = r#"{"codec":"hevc","output":"sdr","source_kind":"pq","preserve_dynamic":true,
        "hdr10_plus":true,"side_data":[]}"#;
    let arguments = data("plan.hdr_metadata", request);

    assert_eq!(arguments["appended"], serde_json::json!([]));
    assert_eq!(arguments["parameters"][0]["key"], "dhdr10-info");
    assert_eq!(arguments["parameters"][0]["value"], "hdr10plus.json");
}

#[test]
fn probe_summarize_returns_a_description_the_rules_accept() {
    let document = r#"{"format":{"duration":"61.5"},
        "streams":[{"codec_type":"video","index":0,"pix_fmt":"yuv420p",
        "width":320,"height":240,"color_transfer":"bt709","avg_frame_rate":"30000/1001",
        "bits_per_raw_sample":"8"}]}"#;
    let request = serde_json::json!({ "document": document }).to_string();
    let summary = data("probe.summarize", &request);
    assert_eq!(summary["video_index"], Value::from(0));
    let rate = summary["frame_rate"].as_f64().expect("frame rate");
    assert!((rate - 29.97).abs() < 0.001);
    assert_eq!(summary["source_range"], "limited");
    assert!((summary["duration_seconds"].as_f64().expect("duration") - 61.5).abs() < 0.001);

    // The description is ready for the rules without re-reading the document.
    let validate = serde_json::json!({
        "source": summary["source"].clone(),
        "options": { "purpose": "archive", "codec": "hevc" }
    })
    .to_string();
    let resolved = data("policy.validate", &validate);
    assert_eq!(resolved["sampling"], "420");
    assert_eq!(resolved["source_kind"], "sdr");
}

#[test]
fn plan_color_filters_writes_the_software_tail() {
    let request = serde_json::json!({
        "primaries": "bt709",
        "transfer": "bt709",
        "space": "bt709",
        "range": "tv",
        "conversion_range": "limited",
        "pixel_format": "yuv420p",
        "purpose": "archive",
        "depth": 8
    })
    .to_string();
    let filters = data("plan.color_filters", &request);
    let list = filters["filters"].as_array().expect("filters");

    assert_eq!(list.len(), 3);
    assert_eq!(
        list[0],
        "zscale=range=limited:dither=error_diffusion:primaries=bt709:transfer=bt709:matrix=bt709"
    );
    assert_eq!(list[1], "format=yuv420p");
    assert_eq!(
        list[2],
        "setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=limited"
    );
}

#[test]
fn plan_render_filters_returns_only_the_entries_that_apply() {
    let request = serde_json::json!({
        "log": {
            "file": "Samsung_Log_to_Rec2100_HLG.cube",
            "matrix": "2020_ncl",
            "range": "limited",
            "source_space": "bt709"
        },
        "tone_map": {"range": "pc", "apply_dolby_vision": true},
        "drop_dynamic_metadata": true
    })
    .to_string();
    let content = data("plan.render_filters", &request);

    let log = content["log"].as_str().expect("log chain");
    assert!(log.starts_with("zscale=matrixin=709:rangein=limited:range=full"));
    assert!(
        content["tone_map"]
            .as_str()
            .expect("tone map")
            .contains("apply_dolbyvision=1")
    );
    assert!(
        content["dropped_metadata"]
            .as_str()
            .expect("drop")
            .starts_with("sidedata=delete")
    );
    // A request without a VPP section must not invent a hardware entry.
    assert_eq!(content["vpp"], Value::Null);
}

#[test]
fn probe_side_data_feeds_the_metadata_tokens() {
    let document = r#"{"streams":[{"codec_type":"video","index":0,"pix_fmt":"yuv420p10le",
        "width":320,"height":240,"color_transfer":"smpte2084","avg_frame_rate":"30/1",
        "side_data_list":[{"side_data_type":"Mastering display metadata",
        "green_x":"13250/50000","green_y":"34500/50000","blue_x":"7500/50000",
        "blue_y":"3000/50000","red_x":"34000/50000","red_y":"16000/50000",
        "white_point_x":"15635/50000","white_point_y":"16450/50000",
        "max_luminance":"10000000/10000","min_luminance":"50/10000"}]}]}"#;
    let request = serde_json::json!({ "document": document }).to_string();
    let summary = data("probe.summarize", &request);
    assert_eq!(summary["side_data"].as_array().expect("side data").len(), 1);

    // The same side data becomes encoder tokens without reading the probe again.
    let metadata = serde_json::json!({
        "codec": "hevc",
        "output": "hdr",
        "source_kind": "pq",
        "side_data": summary["side_data"].clone()
    })
    .to_string();
    let token = data("plan.hdr_metadata", &metadata)["appended"][0]
        .as_str()
        .expect("token")
        .to_owned();
    assert!(token.starts_with("master-display=G(13250,34500)"));
}

#[test]
fn plan_color_tags_fills_the_source_and_names_the_target() {
    let request = serde_json::json!({
        "kind": "pq",
        "output": "sdr",
        "source_range": "limited",
        "output_range": "limited",
        "space": "bt2020nc"
    })
    .to_string();
    let tags = data("plan.color_tags", &request);

    assert_eq!(tags["source"]["transfer"], "smpte2084");
    assert_eq!(tags["source"]["primaries"], "bt2020");
    assert_eq!(tags["source"]["range"], "tv");
    assert_eq!(tags["target"]["transfer"], "bt709");
    assert_eq!(tags["target"]["range"], "tv");
    // The same spelling the software tail uses for its own output entry.
    assert_eq!(
        tags["source_filter"],
        "setparams=color_primaries=bt2020:color_trc=smpte2084:colorspace=bt2020nc:range=limited"
    );
}

#[test]
fn plan_log_profile_names_the_curve_the_lut_expects() {
    let request = serde_json::json!({ "profile": "milog" }).to_string();
    let profile = data("plan.log_profile", &request);

    assert_eq!(profile["file"], "REDMI_K90_MiLog_to_Rec2100_HLG.cube");
    assert_eq!(profile["matrix"], "2020_ncl");
    // Mi-Log is the one curve the camera writes full range.
    assert_eq!(profile["range"], "full");
}

#[test]
fn presets_audio_names_the_share_encoder() {
    let document = include_str!("../../../../Assets/recommended-presets.json").to_string();
    let request = serde_json::json!({ "document": document, "preset": "aac" }).to_string();
    let preset = data("presets.audio", &request);

    assert_eq!(preset["encoder"], "aac");
    assert_eq!(preset["bitrate_kbps"], 256);
    assert_eq!(preset["channels"], 2);
    // The table follows the source rate for AAC, so there is no fixed rate to pass on.
    assert_eq!(preset["sample_rate"], Value::Null);
}
