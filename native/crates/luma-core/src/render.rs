//! Argument blocks that every host renders the same way.
//!
//! The full FFmpeg command still belongs to the host, which knows device quirks and filter
//! choices. The blocks here are the ones that must not drift between hosts: which streams are
//! mapped, whether metadata and chapters survive, and the flags that keep the process quiet.

use crate::domain::Purpose;

/// One input and its stream mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputMap<'a> {
    pub input: &'a str,
    pub video_index: i32,
    /// `0:a?` for every audio track, `0:a:0?` for a motion photo's single track.
    pub audio_map: &'a str,
    pub preserve_metadata: bool,
    pub preserve_chapters: bool,
}

/// Arguments from `-i` through the stream-selection flags.
pub fn input_map(request: &InputMap<'_>) -> Vec<String> {
    vec![
        "-i".to_owned(),
        request.input.to_owned(),
        "-map".to_owned(),
        format!("0:{}", request.video_index),
        "-map".to_owned(),
        request.audio_map.to_owned(),
        "-map_metadata".to_owned(),
        if request.preserve_metadata { "0" } else { "-1" }.to_owned(),
        "-map_chapters".to_owned(),
        if request.preserve_chapters { "0" } else { "-1" }.to_owned(),
        "-sn".to_owned(),
        "-dn".to_owned(),
    ]
}

/// Color tags a host may pass to the encoder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColorTags {
    pub primaries: Option<String>,
    pub transfer: Option<String>,
    pub space: Option<String>,
    pub range: Option<String>,
}

/// Arguments for the four color tags, in the order FFmpeg expects them.
///
/// Dolby Vision profile 5 carries its own color description, so the tags are suppressed for
/// that case; writing them again makes some decoders treat the stream as a different profile.
pub fn color_arguments(tags: &ColorTags, suppress_dv5_tags: bool) -> Vec<String> {
    if suppress_dv5_tags {
        return Vec::new();
    }

    let mut arguments = Vec::with_capacity(8);
    let pairs = [
        ("-color_primaries", &tags.primaries),
        ("-color_trc", &tags.transfer),
        ("-colorspace", &tags.space),
        ("-color_range", &tags.range),
    ];
    for (flag, value) in pairs {
        if let Some(value) = value.as_ref().filter(|text| !text.is_empty()) {
            arguments.push(flag.to_owned());
            arguments.push(value.clone());
        }
    }

    arguments
}

/// Encoder choice and the values that describe its rate control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderRequest {
    pub encoder: String,
    pub preset: String,
    pub quality: String,
    /// Quick share uses the global-quality path; archive uses CRF and a pixel format.
    pub share: bool,
    pub codec: crate::domain::Codec,
    pub depth: u8,
    pub gop: i64,
    pub auto_low_power: bool,
    pub timing_compatibility: bool,
    pub async_depth: String,
    pub pixel_format: String,
}

/// The `-c:v` block, including the options that depend on purpose and codec.
///
/// The order matches what the application has always emitted, so logs and support reports
/// stay comparable while the block moves into the core.
pub fn encoder_arguments(request: &EncoderRequest) -> Vec<String> {
    use crate::domain::Codec;

    let mut arguments = vec![
        "-c:v".to_owned(),
        request.encoder.clone(),
        "-preset".to_owned(),
        request.preset.clone(),
    ];
    if request.share {
        let low_power = if request.auto_low_power { "-1" } else { "1" };
        arguments.extend([
            "-global_quality:v".to_owned(),
            request.quality.clone(),
            "-low_power".to_owned(),
            low_power.to_owned(),
            "-async_depth".to_owned(),
            request.async_depth.clone(),
            "-g".to_owned(),
            request.gop.to_string(),
        ]);
        if request.timing_compatibility && matches!(request.codec, Codec::Avc | Codec::Hevc) {
            arguments.extend(["-bf:v".to_owned(), "0".to_owned()]);
        }
        if request.codec == Codec::Hevc {
            let profile = if request.depth == 10 {
                "main10"
            } else {
                "main"
            };
            arguments.extend(["-profile:v".to_owned(), profile.to_owned()]);
        }
    } else {
        arguments.extend([
            "-crf".to_owned(),
            request.quality.clone(),
            "-pix_fmt".to_owned(),
            request.pixel_format.clone(),
            "-g".to_owned(),
            request.gop.to_string(),
        ]);
    }

    arguments
}

/// Joins encoder-specific parameters the way FFmpeg expects them.
///
/// Values are rendered as `key=value` in insertion order, then the appended tokens (dynamic
/// HDR metadata, for example) keep their own form. An empty list yields an empty string, which
/// the caller uses to decide whether the flag is worth emitting.
pub fn parameter_string(values: &[(String, String)], appended: &[String]) -> String {
    let mut parts = Vec::with_capacity(values.len() + appended.len());
    parts.extend(values.iter().map(|(key, value)| format!("{key}={value}")));
    parts.extend(appended.iter().cloned());
    parts.join(":")
}

/// Joins filter expressions, dropping empty entries.
///
/// The order is the caller's; this only decides how the chain is written. Empty entries are
/// skipped so a disabled stage leaves no stray comma, which FFmpeg rejects.
pub fn filter_chain(filters: &[String]) -> String {
    filters
        .iter()
        .filter(|filter| !filter.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(",")
}

/// GOP length for a conversion.
///
/// Quick share keeps the interval short so high-frame-rate sources stay below the
/// long-GOP range associated with some Arc driver failures. Archive follows the usual
/// eight-second cadence, and motion photos use two because they are short clips.
/// A source whose rate the probe could not resolve is treated as 30 fps, which is what
/// both hosts assume when no rate is available.
pub fn gop(frame_rate: f64, share: bool, motion_photo: bool) -> i64 {
    let rate = if frame_rate > 0.0 && frame_rate.is_finite() {
        frame_rate
    } else {
        30.0
    };
    let factor = if share || motion_photo { 2.0 } else { 8.0 };
    let frames = (factor * rate).round();
    if share {
        frames.clamp(1.0, 240.0) as i64
    } else {
        frames.max(1.0) as i64
    }
}

/// Everything the software colour conversion needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorFilterRequest {
    /// Tag values as the encoder takes them: `pc`/`tv` for the range.
    pub tags: ColorTags,
    /// Sample range for the conversion itself: `limited` or `full`.
    pub range: String,
    pub pixel_format: String,
    pub purpose: Purpose,
    pub depth: u8,
    /// Dynamic metadata survives, so no conversion stage runs at all.
    pub preserve_dynamic: bool,
}

/// Range conversion, output format and the tags that follow them.
///
/// Share and archive disagree on the 10-bit format: quick share hands the encoder a P010
/// surface, archive keeps the resolved planar format. Preserving dynamic HDR skips the
/// conversion entirely, because re-tagging the stream would invalidate the metadata.
pub fn software_color_filters(request: &ColorFilterRequest) -> Vec<String> {
    if request.preserve_dynamic {
        return Vec::new();
    }

    let mut conversion = vec![
        format!("range={}", request.range),
        "dither=error_diffusion".to_owned(),
    ];
    for (value, target) in [
        (&request.tags.primaries, "primaries"),
        (&request.tags.transfer, "transfer"),
        (&request.tags.space, "matrix"),
    ] {
        if let Some(value) = specified(value) {
            conversion.push(format!("{target}={value}"));
        }
    }

    let format = if request.purpose == Purpose::Share && request.depth == 10 {
        "p010le"
    } else {
        request.pixel_format.as_str()
    };
    vec![
        format!("zscale={}", conversion.join(":")),
        format!("format={format}"),
        frame_tags(&request.tags),
    ]
}

/// `setparams` leaves out tags the source never declared.
fn frame_tags(tags: &ColorTags) -> String {
    let mut parts = Vec::new();
    for (value, key) in [
        (&tags.primaries, "color_primaries"),
        (&tags.transfer, "color_trc"),
        (&tags.space, "colorspace"),
        (&tags.range, "range"),
    ] {
        let Some(value) = specified(value) else {
            continue;
        };
        // `setparams` spells the range out; the encoder flags take `pc` and `tv`.
        let value = match value {
            "pc" => "full",
            "tv" => "limited",
            other => other,
        };
        parts.push(format!("{key}={value}"));
    }

    format!("setparams={}", parts.join(":"))
}

/// Treats the probe's placeholder words as "not declared", as the hosts do.
fn specified(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .filter(|text| !matches!(*text, "" | "unknown" | "unspecified" | "reserved"))
}
