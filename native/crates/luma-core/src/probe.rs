//! Normalizes an ffprobe document into the fields the shared rules consume.
//!
//! The host still decides *what* to probe; this module owns how the answer is read, so the
//! Windows adapter and a future macOS adapter agree on names, defaults and rounding.
//! Pixel-format component tables are not here yet: `components` stays empty, and callers
//! that need per-component depths must wait until that table moves into the core.

use serde_json::Value;

use crate::error::{CoreError, CoreResult, codes};
use crate::metadata;
use crate::source::{AudioDto, SourceDto};

/// Video and audio facts taken from one probe response.
#[derive(Debug, Clone, Default)]
pub struct ProbeSummary {
    /// Wire form the output rules already understand.
    pub source: SourceDto,
    pub width: i64,
    pub height: i64,
    /// Index of the video stream the summary describes.
    pub video_index: i64,
    /// Nominal frame rate, zero when the probe reported none.
    pub frame_rate: f64,
    /// Display-matrix rotation in degrees; zero when the stream is not rotated.
    pub rotation: i64,
    /// Dolby Vision profile from the side data, when present.
    pub dolby_vision_profile: Option<i64>,
    /// HDR10+ dynamic metadata is present in the container.
    pub hdr10_plus: bool,
}

/// Reads the probe document and returns the first video stream's summary.
pub fn summarize(document: &str) -> CoreResult<ProbeSummary> {
    let root: Value = serde_json::from_str(document)
        .map_err(|_| CoreError::with_detail(codes::PROBE_JSON, "document"))?;
    let streams = root.get("streams").and_then(Value::as_array);
    let video = streams.and_then(|list| {
        list.iter()
            .find(|stream| text(stream, "codec_type").as_deref() == Some("video"))
    });
    let Some(video) = video else {
        return Err(CoreError::new(codes::PROBE_NO_VIDEO));
    };

    let pixel_format = text(video, "pix_fmt").unwrap_or_default();
    let depth = number(video, "bits_per_raw_sample")
        .or_else(|| number(video, "bits_per_sample"))
        .map(|value| value as i32)
        .filter(|value| *value > 0)
        .unwrap_or_else(|| depth_from_pixel_format(&pixel_format));
    let side_data = side_data(video);
    let dynamic = metadata::parse(side_data);
    let source = SourceDto {
        pixel_format: Some(pixel_format.clone()),
        monochrome: pixel_format.starts_with("gray"),
        bits_per_raw_sample: Some(depth),
        bits_per_sample: Some(depth),
        color_range: text(video, "color_range"),
        kind: Some(kind_from_transfer(text(video, "color_transfer").as_deref())),
        audio: streams.map(|list| audio_tracks(list)).unwrap_or_default(),
        ..SourceDto::default()
    };

    Ok(ProbeSummary {
        source,
        width: number(video, "width").unwrap_or(0),
        height: number(video, "height").unwrap_or(0),
        video_index: number(video, "index").unwrap_or(0),
        frame_rate: nominal_rate(video),
        rotation: side_data
            .iter()
            .find(|item| side_type(item).contains("Display Matrix"))
            .and_then(|item| number(item, "rotation"))
            .unwrap_or(0),
        dolby_vision_profile: dynamic.dolby_vision.map(|config| i64::from(config.profile)),
        hdr10_plus: dynamic.hdr10_plus,
    })
}

fn audio_tracks(streams: &[Value]) -> Vec<AudioDto> {
    streams
        .iter()
        .filter(|stream| text(stream, "codec_type").as_deref() == Some("audio"))
        .map(|stream| AudioDto {
            codec_name: text(stream, "codec_name"),
            profile: text(stream, "profile"),
            sample_format: text(stream, "sample_fmt"),
            bits_per_raw_sample: number(stream, "bits_per_raw_sample").map(|value| value as i32),
            bits_per_sample: number(stream, "bits_per_sample").map(|value| value as i32),
            channels: number(stream, "channels").map(|value| value as i32),
            channel_layout: text(stream, "channel_layout"),
        })
        .collect()
}

fn side_data(stream: &Value) -> &[Value] {
    stream
        .get("side_data_list")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn side_type(item: &Value) -> String {
    text(item, "side_data_type").unwrap_or_default()
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn number(value: &Value, key: &str) -> Option<i64> {
    match value.get(key)? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// Nominal frame rate: the average rate wins because it reflects variable-rate files,
/// and the nominal rate is only a fallback the container may not report at all.
fn nominal_rate(video: &Value) -> f64 {
    let average = rate(text(video, "avg_frame_rate").as_deref());
    if average > 0.0 {
        return average;
    }
    rate(text(video, "r_frame_rate").as_deref())
}

/// Reads a rate the probe reports as `numerator/denominator`, or as a plain number.
fn rate(value: Option<&str>) -> f64 {
    let Some(text) = value.map(str::trim).filter(|text| !text.is_empty()) else {
        return 0.0;
    };
    let quotient = match text.split_once('/') {
        None => text.parse::<f64>().unwrap_or(0.0),
        Some((numerator, denominator)) => {
            let numerator: f64 = numerator.trim().parse().unwrap_or(0.0);
            let denominator: f64 = denominator.trim().parse().unwrap_or(0.0);
            if denominator > 0.0 {
                numerator / denominator
            } else {
                0.0
            }
        }
    };
    if quotient.is_finite() && quotient > 0.0 {
        quotient
    } else {
        0.0
    }
}

fn depth_from_pixel_format(pixel_format: &str) -> i32 {
    for marker in ["16", "12", "10", "9"] {
        if pixel_format.contains(marker) {
            return match marker {
                "16" => 16,
                "12" => 12,
                "10" => 10,
                _ => 9,
            };
        }
    }

    8
}

fn kind_from_transfer(transfer: Option<&str>) -> String {
    match transfer {
        None | Some("") | Some("unknown") => "unknown".to_owned(),
        Some("smpte2084") => "pq".to_owned(),
        Some("arib-std-b67") => "hlg".to_owned(),
        _ => "sdr".to_owned(),
    }
}
