//! Versioned C ABI for the shared core.
//!
//! Ownership rules:
//!
//! - Every `*mut c_char` returned here is owned by the caller and must be
//!   released with [`luma_string_free`].
//! - Input pointers are borrowed for the duration of the call only; the core
//!   never keeps them.
//! - `luma_core_version` returns a pointer to static storage and must *not* be
//!   freed.
//!
//! Every call answers with one JSON envelope:
//!
//! ```json
//! {"ok":true,"data":{...}}
//! {"ok":false,"error":{"code":"policy.hdr_depth","detail":null}}
//! ```
//!
//! A panic never crosses the boundary: it is reported as `ffi.panic` so a host
//! process cannot be aborted by a rule bug.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use luma_core::capability::{Backend, Device, DeviceDto, Requirement, select, summarize};
use luma_core::domain::{Codec, ColorRange, EncodeOptions, OptionsDto, Output, Purpose, Sampling};
use luma_core::error::{CoreError, CoreResult, codes};
use luma_core::plan::{self, PlanRequest};
use luma_core::policy;
use luma_core::presets::PresetRegistry;
use luma_core::source::{SourceDto, SourceFormat};
use luma_core::timeline::{FramePoint, FrameRange, FrameTimeline};
use serde::Deserialize;
use serde_json::{Value, json};

/// ABI revision. Bumped only for incompatible changes to exports or envelopes.
const ABI_VERSION: u32 = luma_core::ABI_VERSION;

/// NUL-terminated crate version in static storage.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

/// Operations understood by [`luma_call_json`].
const OPERATIONS: [&str; 26] = [
    "core.info",
    "policy.describe",
    "policy.resolve",
    "policy.validate",
    "timeline.resolve",
    "presets.resolve",
    "presets.validate",
    "capability.select",
    "capability.summary",
    "plan.build",
    "probe.summarize",
    "capability.backend",
    "capability.supports",
    "cache.plan",
    "capability.detect",
    "capability.parameters",
    "plan.input_map",
    "plan.color_args",
    "plan.encoder_args",
    "plan.parameter_string",
    "plan.filter_chain",
    "plan.gop",
    "plan.hdr_metadata",
    "plan.color_filters",
    "plan.render_filters",
    "plan.color_tags",
];

/// Returns the ABI revision the loaded library implements.
#[unsafe(no_mangle)]
pub extern "C" fn luma_abi_version() -> u32 {
    ABI_VERSION
}

/// Returns a static NUL-terminated core version. Do not free this pointer.
#[unsafe(no_mangle)]
pub extern "C" fn luma_core_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

/// Releases a string returned by [`luma_call_json`].
///
/// # Safety
///
/// `value` must be null or a pointer previously returned by this library and
/// not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn luma_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    // SAFETY: the caller upholds the contract above; the pointer came from
    // CString::into_raw in this module.
    drop(unsafe { CString::from_raw(value) });
}

/// Executes one operation and returns an owned JSON envelope.
///
/// # Safety
///
/// `op` and `request` must be null or point to NUL-terminated UTF-8 strings
/// that stay valid for the duration of the call. The result must be freed with
/// [`luma_string_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn luma_call_json(op: *const c_char, request: *const c_char) -> *mut c_char {
    let envelope = catch_unwind(AssertUnwindSafe(|| {
        let op = match unsafe { read_string(op) } {
            Ok(op) => op,
            Err(error) => return error_envelope(&error),
        };
        let request = match unsafe { read_string(request) } {
            Ok(request) => request,
            Err(error) => return error_envelope(&error),
        };
        match dispatch(&op, &request) {
            Ok(data) => json!({ "ok": true, "data": data }).to_string(),
            Err(error) => error_envelope(&error),
        }
    }))
    .unwrap_or_else(|_| error_envelope(&CoreError::new(codes::FFI_PANIC)));
    into_c_string(envelope)
}

fn error_envelope(error: &CoreError) -> String {
    json!({
        "ok": false,
        "error": { "code": error.code(), "detail": error.detail() }
    })
    .to_string()
}

/// # Safety
///
/// `ptr` is null or a valid NUL-terminated C string.
unsafe fn read_string(ptr: *const c_char) -> CoreResult<String> {
    if ptr.is_null() {
        return Err(CoreError::new(codes::FFI_NULL));
    }
    // SAFETY: guaranteed by the caller of the exported function.
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| CoreError::new(codes::FFI_UTF8))
}

fn into_c_string(text: String) -> *mut c_char {
    // JSON output never contains a raw NUL; a failure here would mean a bug in
    // the encoder, so the caller receives null and reports the missing result.
    CString::new(text)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn dispatch(op: &str, request: &str) -> CoreResult<Value> {
    let payload: Value =
        serde_json::from_str(request).map_err(|_| CoreError::new(codes::FFI_JSON))?;
    match op {
        "core.info" => Ok(core_info()),
        "policy.describe" => policy_describe(&payload),
        "policy.resolve" => policy_resolve(&payload),
        "policy.validate" => policy_validate(&payload),
        "timeline.resolve" => timeline_resolve(&payload),
        "presets.resolve" => presets_resolve(&payload),
        "presets.validate" => presets_validate(&payload),
        "probe.summarize" => probe_summarize(&payload),
        "capability.select" => capability_select(&payload),
        "capability.summary" => capability_summary(&payload),
        "capability.backend" => capability_backend(&payload),
        "capability.supports" => capability_supports(&payload),
        "cache.plan" => cache_plan(&payload),
        "capability.detect" => capability_detect(&payload),
        "capability.parameters" => capability_parameters(&payload),
        "plan.input_map" => plan_input_map(&payload),
        "plan.color_args" => plan_color_args(&payload),
        "plan.encoder_args" => plan_encoder_args(&payload),
        "plan.parameter_string" => plan_parameter_string(&payload),
        "plan.filter_chain" => plan_filter_chain(&payload),
        "plan.gop" => plan_gop(&payload),
        "plan.hdr_metadata" => plan_hdr_metadata(&payload),
        "plan.color_filters" => plan_color_filters(&payload),
        "plan.render_filters" => plan_render_filters(&payload),
        "plan.color_tags" => plan_color_tags(&payload),
        "plan.build" => plan_build(&payload),
        other => Err(CoreError::with_detail(codes::FFI_OP, other)),
    }
}

fn core_info() -> Value {
    json!({
        "abi": ABI_VERSION,
        "core_version": luma_core::CORE_VERSION,
        "target_arch": std::env::consts::ARCH,
        "target_os": std::env::consts::OS,
        "operations": OPERATIONS,
    })
}

fn object_field<'a>(payload: &'a Value, key: &str) -> CoreResult<&'a Value> {
    payload
        .get(key)
        .ok_or_else(|| CoreError::with_detail(codes::FFI_ENVELOPE, key))
}

fn parse_source(payload: &Value) -> CoreResult<SourceFormat> {
    let source: SourceDto = serde_json::from_value(object_field(payload, "source")?.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "source"))?;
    source.into_source()
}

fn parse_options(payload: &Value) -> CoreResult<EncodeOptions> {
    match payload.get("options") {
        None | Some(Value::Null) => Ok(EncodeOptions::default()),
        Some(value) => {
            let dto: OptionsDto = serde_json::from_value(value.clone())
                .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "options"))?;
            dto.into_options()
        }
    }
}

fn range_label(range: ColorRange) -> &'static str {
    range.as_str()
}

fn policy_describe(payload: &Value) -> CoreResult<Value> {
    let source = parse_source(payload)?;
    let options = parse_options(payload)?;
    let samplings = policy::sampling_choices(&source);
    let depths = policy::depth_choices(
        policy::source_depth(&source),
        options.purpose,
        options.codec,
        options.output,
    );
    Ok(json!({
        "source": {
            "sampling": policy::source_sampling(&source).label(),
            "depth": policy::source_depth(&source),
            "range": range_label(policy::source_range(&source, None)),
            "kind": source.effective_kind(options.source_kind).as_str(),
            "dynamic_hdr": source.dynamic_hdr.is_present(),
        },
        "choices": {
            "samplings": samplings.iter().map(|value| value.label()).collect::<Vec<_>>(),
            "depths": depths,
            "sampling": policy::default_sampling(&source, options.purpose, options.codec).label(),
            "depth": policy::default_depth(&depths, options.purpose, options.output),
        },
        "audio": {
            "flac": policy::flac_eligibility(&source).is_ok(),
        },
    }))
}

fn resolved_json(resolved: &luma_core::ResolvedEncode) -> Value {
    json!({
        "sampling": resolved.format.sampling.label(),
        "color_range": resolved.format.color_range.as_str(),
        "bit_depth": resolved.format.bit_depth,
        "pixel_format": resolved.format.pixel_format(),
        "range_tag": resolved.format.range_tag(),
        "source_kind": resolved.source_kind.as_str(),
        "container": resolved.options.container(),
        "suffix": resolved.options.suffix(),
    })
}

fn policy_resolve(payload: &Value) -> CoreResult<Value> {
    let source = parse_source(payload)?;
    let options = parse_options(payload)?;
    let resolved = policy::resolve(&options, &source)?;
    Ok(resolved_json(&resolved))
}

fn policy_validate(payload: &Value) -> CoreResult<Value> {
    let source = parse_source(payload)?;
    let options = parse_options(payload)?;
    let resolved = policy::validate(&options, &source)?;
    Ok(resolved_json(&resolved))
}

#[derive(Deserialize)]
struct FrameDto {
    #[serde(default)]
    pts: i64,
    #[serde(default)]
    duration: i64,
    #[serde(default)]
    key_frame: bool,
}

#[derive(Deserialize)]
struct RangeDto {
    #[serde(default)]
    start: usize,
    #[serde(default)]
    end: Option<usize>,
}

#[derive(Deserialize)]
struct TimelineRequest {
    frames: Vec<FrameDto>,
    /// `numerator/denominator`, as reported by the probe.
    time_base: String,
    #[serde(default)]
    packet_indexed: bool,
    #[serde(default)]
    range: Option<RangeDto>,
}

fn timeline_resolve(payload: &Value) -> CoreResult<Value> {
    let request: TimelineRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "timeline"))?;
    let (numerator, denominator) = request
        .time_base
        .split_once('/')
        .ok_or_else(|| CoreError::with_detail(codes::SOURCE_TIME_BASE, &request.time_base))?;
    let numerator = numerator
        .trim()
        .parse::<i64>()
        .map_err(|_| CoreError::with_detail(codes::SOURCE_TIME_BASE, &request.time_base))?;
    let denominator = denominator
        .trim()
        .parse::<i64>()
        .map_err(|_| CoreError::with_detail(codes::SOURCE_TIME_BASE, &request.time_base))?;
    let frames = request
        .frames
        .into_iter()
        .map(|frame| FramePoint {
            pts: frame.pts,
            duration: frame.duration,
            key_frame: frame.key_frame,
        })
        .collect();
    let timeline = FrameTimeline::new(frames, numerator, denominator, request.packet_indexed)?;
    let window = match request.range {
        None => None,
        Some(range) => Some(timeline.resolve(FrameRange {
            start: range.start,
            end: range.end,
        })?),
    };
    Ok(json!({
        "count": timeline.count(),
        "duration": timeline.duration(),
        "packet_indexed": timeline.is_packet_indexed(),
        "window": window.map(|window| json!({
            "start": window.start,
            "end": window.end,
            "count": window.count(),
            "seek_frame": window.seek_frame,
            "seek_seconds": window.seek_seconds,
            "start_seconds": window.start_seconds,
            "end_seconds": window.end_seconds,
            "duration": window.duration(),
        })),
    }))
}

#[derive(Deserialize)]
struct PresetRequest {
    /// Preset document text; the host owns file access.
    document: String,
    purpose: String,
    codec: String,
    #[serde(default)]
    variant: Option<String>,
}

fn presets_resolve(payload: &Value) -> CoreResult<Value> {
    let request: PresetRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "presets"))?;
    let registry = PresetRegistry::parse(&request.document)?;
    let purpose = Purpose::parse(&request.purpose)?;
    let codec = Codec::parse(&request.codec)?;
    let variant = match request.variant.as_deref() {
        None | Some("") | Some("standard") => luma_core::Variant::Standard,
        Some(value) => luma_core::Variant::parse(value)?,
    };
    let preset = registry.video(purpose, codec, variant)?;
    let audio = registry.audio(purpose, codec)?;
    Ok(json!({
        "selector": preset.selector,
        "encoder": preset.encoder,
        "preset": preset.preset,
        "quality": preset.quality,
        "pixel_format": preset.declared_pixel_format,
        "tune": preset.tune,
        "lookahead": preset.lookahead,
        "async_depth": preset.async_depth,
        "parameters": preset.parameters,
        "audio": match audio {
            luma_core::AudioDefault::Copy => json!({"mode": "copy"}),
            luma_core::AudioDefault::Encode(name) => json!({"mode": "encode", "preset": name}),
        },
    }))
}

#[derive(Deserialize)]
struct BackendRequest {
    encoder: String,
}

fn capability_backend(payload: &Value) -> CoreResult<Value> {
    let request: BackendRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "capability"))?;
    Ok(json!({
        "backend": luma_core::capability::backend_for_encoder(&request.encoder).map(|value| value.as_str()),
    }))
}

#[derive(Deserialize)]
struct SupportRequest {
    backend: String,
    codec: String,
    depth: u8,
    sampling: String,
}

fn capability_supports(payload: &Value) -> CoreResult<Value> {
    let request: SupportRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "capability"))?;
    let backend = luma_core::capability::Backend::parse(&request.backend)
        .ok_or_else(|| CoreError::with_detail(codes::FFI_ENVELOPE, "backend"))?;
    let codec = luma_core::Codec::parse(&request.codec)?;
    Ok(json!({
        "supported": luma_core::capability::supports(backend, codec, request.depth, &request.sampling),
    }))
}

#[derive(Deserialize)]
struct CacheEntryDto {
    key: String,
    age_days: u32,
    bytes: u64,
    #[serde(default)]
    in_use: bool,
}

#[derive(Deserialize)]
struct CachePolicyDto {
    keep_recent: usize,
    max_age_days: u32,
    max_bytes: u64,
}

#[derive(Deserialize)]
struct CacheRequest {
    entries: Vec<CacheEntryDto>,
    #[serde(default)]
    current: String,
    policy: CachePolicyDto,
}

fn cache_plan(payload: &Value) -> CoreResult<Value> {
    let request: CacheRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "cache"))?;
    let entries = request
        .entries
        .into_iter()
        .map(|entry| luma_core::cache::Entry {
            key: entry.key,
            age_days: entry.age_days,
            bytes: entry.bytes,
            in_use: entry.in_use,
        })
        .collect::<Vec<_>>();
    let policy = luma_core::cache::Policy {
        keep_recent: request.policy.keep_recent,
        max_age_days: request.policy.max_age_days,
        max_bytes: request.policy.max_bytes,
    };
    let decisions = luma_core::cache::plan(&entries, &request.current, &policy);
    Ok(json!({
        "decisions": decisions
            .iter()
            .map(|(key, decision)| json!({ "key": key, "decision": decision_name(*decision) }))
            .collect::<Vec<_>>(),
        "kept_bytes": luma_core::cache::kept_bytes(&entries, &decisions),
        "reclaimable_bytes": luma_core::cache::reclaimable_bytes(&entries, &decisions),
    }))
}

fn decision_name(decision: luma_core::cache::Decision) -> &'static str {
    match decision {
        luma_core::cache::Decision::KeepCurrent => "keep_current",
        luma_core::cache::Decision::KeepInUse => "keep_in_use",
        luma_core::cache::Decision::KeepRecent => "keep_recent",
        luma_core::cache::Decision::RemoveTooOld => "remove_too_old",
        luma_core::cache::Decision::RemoveOverCount => "remove_over_count",
        luma_core::cache::Decision::RemoveOverBudget => "remove_over_budget",
    }
}

#[derive(Deserialize)]
struct DetectRequest {
    encoders: Vec<String>,
}

fn capability_detect(payload: &Value) -> CoreResult<Value> {
    let request: DetectRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "capability"))?;
    let statuses = luma_core::providers::detect(&request.encoders);
    Ok(json!({
        "backends": statuses
            .iter()
            .map(|status| json!({
                "backend": status.backend.as_str(),
                "availability": status.availability.as_str(),
                "encoders": &status.encoders,
            }))
            .collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
struct ParametersRequest {
    backend: String,
    codec: String,
}

fn capability_parameters(payload: &Value) -> CoreResult<Value> {
    let request: ParametersRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "capability"))?;
    let backend = luma_core::capability::Backend::parse(&request.backend)
        .ok_or_else(|| CoreError::with_detail(codes::FFI_ENVELOPE, "backend"))?;
    let codec = luma_core::Codec::parse(&request.codec)?;
    let parameters = luma_core::providers::video_parameters(backend, codec)
        .iter()
        .map(|parameter| {
            let (kind, choices) = match parameter.kind {
                luma_core::providers::ParameterKind::Integer => ("integer", Vec::new()),
                luma_core::providers::ParameterKind::Flag => ("flag", Vec::new()),
                luma_core::providers::ParameterKind::Choice(values) => ("choice", values.to_vec()),
            };
            json!({ "name": parameter.name, "kind": kind, "choices": choices })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "parameters": parameters }))
}

#[derive(Deserialize)]
struct InputMapRequest {
    input: String,
    video_index: i32,
    audio_map: String,
    preserve_metadata: bool,
    preserve_chapters: bool,
}

fn plan_input_map(payload: &Value) -> CoreResult<Value> {
    let request: InputMapRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let arguments = luma_core::render::input_map(&luma_core::render::InputMap {
        input: &request.input,
        video_index: request.video_index,
        audio_map: &request.audio_map,
        preserve_metadata: request.preserve_metadata,
        preserve_chapters: request.preserve_chapters,
    });
    Ok(json!({ "arguments": arguments }))
}

#[derive(Deserialize)]
struct ColorRequest {
    #[serde(default)]
    primaries: Option<String>,
    #[serde(default)]
    transfer: Option<String>,
    #[serde(default)]
    space: Option<String>,
    #[serde(default)]
    range: Option<String>,
    #[serde(default)]
    suppress_dv5_tags: bool,
}

fn plan_color_args(payload: &Value) -> CoreResult<Value> {
    let request: ColorRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let arguments = luma_core::render::color_arguments(
        &luma_core::render::ColorTags {
            primaries: request.primaries,
            transfer: request.transfer,
            space: request.space,
            range: request.range,
        },
        request.suppress_dv5_tags,
    );
    Ok(json!({ "arguments": arguments }))
}

#[derive(Deserialize)]
struct EncoderRequestDto {
    encoder: String,
    preset: String,
    quality: String,
    share: bool,
    codec: String,
    depth: u8,
    gop: i64,
    auto_low_power: bool,
    timing_compatibility: bool,
    async_depth: String,
    pixel_format: String,
}

fn plan_encoder_args(payload: &Value) -> CoreResult<Value> {
    let request: EncoderRequestDto = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let codec = luma_core::Codec::parse(&request.codec)?;
    let arguments = luma_core::render::encoder_arguments(&luma_core::render::EncoderRequest {
        encoder: request.encoder,
        preset: request.preset,
        quality: request.quality,
        share: request.share,
        codec,
        depth: request.depth,
        gop: request.gop,
        auto_low_power: request.auto_low_power,
        timing_compatibility: request.timing_compatibility,
        async_depth: request.async_depth,
        pixel_format: request.pixel_format,
    });
    Ok(json!({ "arguments": arguments }))
}

#[derive(Deserialize)]
struct ParameterPairDto {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct ParameterStringRequest {
    values: Vec<ParameterPairDto>,
    #[serde(default)]
    appended: Vec<String>,
}

fn plan_parameter_string(payload: &Value) -> CoreResult<Value> {
    let request: ParameterStringRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let values = request
        .values
        .into_iter()
        .map(|pair| (pair.key, pair.value))
        .collect::<Vec<_>>();
    Ok(json!({
        "text": luma_core::render::parameter_string(&values, &request.appended),
    }))
}

#[derive(Deserialize)]
struct FilterChainRequest {
    filters: Vec<String>,
}

fn plan_filter_chain(payload: &Value) -> CoreResult<Value> {
    let request: FilterChainRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    Ok(json!({ "text": luma_core::render::filter_chain(&request.filters) }))
}

#[derive(Deserialize)]
struct GopRequest {
    /// Nominal frame rate, already resolved from the probe's rational form.
    frame_rate: f64,
    share: bool,
    #[serde(default)]
    motion_photo: bool,
}

fn plan_gop(payload: &Value) -> CoreResult<Value> {
    let request: GopRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    Ok(json!({
        "frames": luma_core::render::gop(request.frame_rate, request.share, request.motion_photo),
    }))
}

#[derive(Deserialize)]
struct LogFilterDto {
    file: String,
    matrix: String,
    range: String,
    #[serde(default)]
    source_space: Option<String>,
    #[serde(default)]
    source_range: Option<String>,
}

#[derive(Deserialize)]
struct ToneMapDto {
    range: String,
    #[serde(default)]
    apply_dolby_vision: bool,
}

#[derive(Deserialize)]
struct DolbyRenderDto {
    kind: String,
    range: String,
}

#[derive(Deserialize)]
struct VppDto {
    depth: u8,
    out_range: String,
    #[serde(default)]
    sdr: bool,
}

#[derive(Deserialize)]
struct ColorTagsDto {
    kind: String,
    output: String,
    #[serde(default)]
    rgb: bool,
    /// Dolby Vision profile 5 carries its own color description.
    #[serde(default)]
    dolby_profile5: bool,
    /// Resolved source range: `limited` or `full`.
    source_range: String,
    /// Resolved output range: `limited` or `full`.
    output_range: String,
    #[serde(default)]
    primaries: Option<String>,
    #[serde(default)]
    transfer: Option<String>,
    #[serde(default)]
    space: Option<String>,
}

fn plan_color_tags(payload: &Value) -> CoreResult<Value> {
    let request: ColorTagsDto = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let kind = luma_core::SourceKind::parse(&request.kind)?;
    let output = Output::parse(&request.output)?;
    let declared = luma_core::render::ColorTags {
        primaries: request.primaries,
        transfer: request.transfer,
        space: request.space,
        range: None,
    };
    let source = luma_core::render::source_tags(
        kind,
        request.rgb,
        request.dolby_profile5,
        &declared,
        &request.source_range,
    );
    let target = luma_core::render::target_tags(kind, output, &source, &request.output_range);
    Ok(json!({
        "source": tag_json(&source),
        "target": tag_json(&target),
    }))
}

fn tag_json(tags: &luma_core::render::ColorTags) -> Value {
    json!({
        "primaries": tags.primaries,
        "transfer": tags.transfer,
        "space": tags.space,
        "range": tags.range,
    })
}

#[derive(Deserialize)]
struct RenderFilterDto {
    #[serde(default)]
    log: Option<LogFilterDto>,
    #[serde(default)]
    tone_map: Option<ToneMapDto>,
    #[serde(default)]
    dolby_render: Option<DolbyRenderDto>,
    #[serde(default)]
    drop_dynamic_metadata: bool,
    #[serde(default)]
    vpp: Option<VppDto>,
}

fn log_filter(dto: &LogFilterDto) -> luma_core::render::LogFilter<'_> {
    luma_core::render::LogFilter {
        file: dto.file.as_str(),
        matrix: dto.matrix.as_str(),
        range: dto.range.as_str(),
        source_space: dto.source_space.as_deref(),
        source_range: dto.source_range.as_deref(),
    }
}

fn tone_map(dto: &ToneMapDto) -> luma_core::render::ToneMapFilter<'_> {
    luma_core::render::ToneMapFilter {
        range: dto.range.as_str(),
        apply_dolby_vision: dto.apply_dolby_vision,
    }
}

fn dolby_render(dto: &DolbyRenderDto) -> CoreResult<luma_core::render::DolbyRenderFilter<'_>> {
    Ok(luma_core::render::DolbyRenderFilter {
        kind: luma_core::SourceKind::parse(&dto.kind)?,
        range: dto.range.as_str(),
    })
}

fn vpp(dto: &VppDto) -> luma_core::render::VppFilter<'_> {
    luma_core::render::VppFilter {
        depth: dto.depth,
        out_range: dto.out_range.as_str(),
        sdr: dto.sdr,
    }
}

fn plan_render_filters(payload: &Value) -> CoreResult<Value> {
    let request: RenderFilterDto = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let dolby = request
        .dolby_render
        .as_ref()
        .map(dolby_render)
        .transpose()?;
    let content = luma_core::render::filter_content(&luma_core::render::FilterContentRequest {
        log: request.log.as_ref().map(log_filter),
        tone_map: request.tone_map.as_ref().map(tone_map),
        dolby_render: dolby,
        drop_dynamic_metadata: request.drop_dynamic_metadata,
        vpp: request.vpp.as_ref().map(vpp),
    });
    Ok(json!({
        "log": content.log,
        "tone_map": content.tone_map,
        "dolby_render": content.dolby_render,
        "dropped_metadata": content.dropped_metadata,
        "vpp": content.vpp,
    }))
}

#[derive(Deserialize)]
struct ColorFilterDto {
    #[serde(default)]
    primaries: Option<String>,
    #[serde(default)]
    transfer: Option<String>,
    #[serde(default)]
    space: Option<String>,
    #[serde(default)]
    range: Option<String>,
    /// Sample range the conversion writes: `limited` or `full`.
    conversion_range: String,
    pixel_format: String,
    purpose: String,
    depth: u8,
    #[serde(default)]
    preserve_dynamic: bool,
}

fn plan_color_filters(payload: &Value) -> CoreResult<Value> {
    let request: ColorFilterDto = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let tags = luma_core::render::ColorTags {
        primaries: request.primaries,
        transfer: request.transfer,
        space: request.space,
        range: request.range,
    };
    let color_request = luma_core::render::ColorFilterRequest {
        tags,
        range: request.conversion_range,
        pixel_format: request.pixel_format,
        purpose: Purpose::parse(&request.purpose)?,
        depth: request.depth,
        preserve_dynamic: request.preserve_dynamic,
    };
    let filters = luma_core::render::software_color_filters(&color_request);
    Ok(json!({ "filters": filters }))
}

#[derive(Deserialize)]
struct HdrMetadataDto {
    codec: String,
    output: String,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    preserve_dynamic: bool,
    #[serde(default)]
    dolby_vision_profile: Option<i32>,
    #[serde(default)]
    hdr10_plus: bool,
    /// Video and first-frame side data, as the probe reported it.
    #[serde(default)]
    side_data: Vec<Value>,
}

fn plan_hdr_metadata(payload: &Value) -> CoreResult<Value> {
    let request: HdrMetadataDto = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let source_kind = match request.source_kind.as_deref() {
        None | Some("") => luma_core::SourceKind::Unknown,
        Some(value) => luma_core::SourceKind::parse(value)?,
    };
    let core_request = luma_core::metadata::HdrMetadataRequest {
        codec: Codec::parse(&request.codec)?,
        output: Output::parse(&request.output)?,
        source_kind,
        preserve_dynamic: request.preserve_dynamic,
        dolby_vision_profile: request.dolby_vision_profile,
        hdr10_plus: request.hdr10_plus,
        side_data: &request.side_data,
    };
    let arguments = luma_core::metadata::encoder_metadata(&core_request);
    let parameters = arguments
        .parameters
        .iter()
        .map(|(key, value)| json!({ "key": key, "value": value }))
        .collect::<Vec<_>>();
    Ok(json!({ "parameters": parameters, "appended": arguments.appended }))
}

fn probe_summarize(payload: &Value) -> CoreResult<Value> {
    let request: PresetDocument = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "probe"))?;
    let summary = luma_core::probe::summarize(&request.document)?;
    let source = summary.source.clone();
    Ok(json!({
        "width": summary.width,
        "height": summary.height,
        "rotation": summary.rotation,
        "kind": summary.source.kind,
        "pixel_format": summary.source.pixel_format,
        "depth": summary.source.bits_per_raw_sample,
        "dolby_vision": summary.dolby_vision_profile,
        "hdr10_plus": summary.hdr10_plus,
        "audio": source.audio.len(),
        // The normalized description is ready for `policy.*` and `plan.*`: a host must
        // not have to re-read ffprobe output to reach the same decision.
        "source": source,
        "video_index": summary.video_index,
        "frame_rate": summary.frame_rate,
        "side_data": summary.side_data,
    }))
}

#[derive(Deserialize)]
struct PresetDocument {
    document: String,
}

fn presets_validate(payload: &Value) -> CoreResult<Value> {
    let request: PresetDocument = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "presets"))?;
    let registry = PresetRegistry::parse(&request.document)?;
    let catalog = registry
        .catalog()
        .into_iter()
        .map(|preset| {
            json!({
                "selector": preset.selector,
                "encoder": preset.encoder,
                "quality": preset.quality,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "count": catalog.len(), "presets": catalog }))
}

#[derive(Deserialize)]
struct RequirementDto {
    purpose: String,
    codec: String,
    output: String,
    depth: u8,
    sampling: String,
    #[serde(default)]
    dynamic_hdr: bool,
}

impl RequirementDto {
    fn into_requirement(self) -> CoreResult<Requirement> {
        Ok(Requirement {
            purpose: Purpose::parse(&self.purpose)?,
            codec: Codec::parse(&self.codec)?,
            output: Output::parse(&self.output)?,
            depth: self.depth,
            sampling: Sampling::from_label(&self.sampling)
                .ok_or_else(|| CoreError::with_detail(codes::OPTION_SAMPLING, &self.sampling))?,
            dynamic_hdr: self.dynamic_hdr,
        })
    }
}

fn parse_devices(payload: &Value) -> CoreResult<Vec<Device>> {
    let entries: Vec<DeviceDto> = serde_json::from_value(object_field(payload, "devices")?.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "devices"))?;
    Ok(entries
        .into_iter()
        .filter_map(DeviceDto::into_device)
        .collect())
}

fn capability_select(payload: &Value) -> CoreResult<Value> {
    let devices = parse_devices(payload)?;
    let request: RequirementDto =
        serde_json::from_value(object_field(payload, "requirement")?.clone())
            .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "requirement"))?;
    let request = request.into_requirement()?;
    let selection = select(&devices, &request)?;
    Ok(json!({
        "device_id": selection.device_id,
        "device_name": selection.device_name,
        "backend": selection.backend.as_str(),
        "score": selection.score,
        "reasons": selection.reasons,
        "fallback": selection.fallback,
    }))
}

#[derive(Deserialize)]
struct SummaryRequest {
    devices: Value,
    codec: String,
}

fn capability_summary(payload: &Value) -> CoreResult<Value> {
    let request: SummaryRequest = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "summary"))?;
    let entries: Vec<DeviceDto> = serde_json::from_value(request.devices)
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "devices"))?;
    let devices: Vec<Device> = entries
        .into_iter()
        .filter_map(DeviceDto::into_device)
        .collect();
    let codec = Codec::parse(&request.codec)?;
    let summary = summarize(&devices, codec);
    Ok(json!({
        "backends": summary.backends.iter().map(|backend| backend.as_str()).collect::<Vec<_>>(),
        "max_depth": summary.max_depth,
        "samplings": summary.samplings.iter().map(|value| value.label()).collect::<Vec<_>>(),
        "hdr": summary.hdr,
        "dynamic_hdr": summary.dynamic_hdr,
    }))
}

#[derive(Deserialize)]
struct PlanDto {
    source: Value,
    #[serde(default)]
    options: Option<Value>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    frame_window: bool,
    #[serde(default)]
    motion_photo: bool,
    #[serde(default)]
    preview_size: u32,
}

fn plan_build(payload: &Value) -> CoreResult<Value> {
    let request: PlanDto = serde_json::from_value(payload.clone())
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "plan"))?;
    let source: SourceDto = serde_json::from_value(request.source)
        .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "source"))?;
    let source = source.into_source()?;
    let options = match request.options {
        None => EncodeOptions::default(),
        Some(value) => {
            let dto: OptionsDto = serde_json::from_value(value)
                .map_err(|_| CoreError::with_detail(codes::FFI_ENVELOPE, "options"))?;
            dto.into_options()?
        }
    };
    let backend = match request.backend.as_deref() {
        None | Some("") => Backend::Software,
        Some(value) => Backend::parse(value)
            .ok_or_else(|| CoreError::with_detail(codes::FFI_ENVELOPE, value))?,
    };
    let plan = plan::build(&PlanRequest {
        options,
        source,
        backend,
        frame_window: request.frame_window,
        motion_photo: request.motion_photo,
        preview_size: request.preview_size,
    })?;
    Ok(json!({
        "resolved": resolved_json(&plan.resolved),
        "decode": plan.decode.as_str(),
        "transport": plan.transport.as_str(),
        "steps": plan.steps.iter().map(|step| json!({
            "kind": step.kind.as_str(),
            "transport": step.transport.as_str(),
        })).collect::<Vec<_>>(),
        "notes": plan.notes,
    }))
}
