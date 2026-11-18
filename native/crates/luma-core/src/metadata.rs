//! Dynamic HDR metadata read from probe side data.
//!
//! Dolby Vision and HDR10+ arrive as side-data entries whose names differ slightly
//! between FFmpeg builds, so the reader matches on the type name instead of an exact key
//! and fills the values the output policy already understands.
//!
//! The last section composes what the encoder is told to write: the parameters that keep
//! dynamic metadata alive, and the static mastering/light tokens either encoder family
//! expects. Both hosts must spell those identically, so they live here rather than in a
//! backend.

use serde_json::Value;

use crate::domain::{Codec, Output, SourceKind};
use crate::source::DolbyVisionConfig;

/// Dynamic metadata attached to one video stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DynamicMetadata {
    /// Dolby Vision configuration, when a DOVI record exists.
    pub dolby_vision: Option<DolbyVisionConfig>,
    /// HDR10+ dynamic metadata is present.
    pub hdr10_plus: bool,
}

impl DynamicMetadata {
    /// True when the stream carries metadata that a plain re-encode would lose.
    pub fn present(&self) -> bool {
        self.dolby_vision.is_some() || self.hdr10_plus
    }
}

/// Reads DOVI and HDR10+ entries from a stream's side-data list.
pub fn parse(side_data: &[Value]) -> DynamicMetadata {
    let mut metadata = DynamicMetadata::default();
    for item in side_data {
        let kind = item
            .get("side_data_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if kind.contains("DOVI configuration record") {
            metadata.dolby_vision = Some(DolbyVisionConfig {
                profile: number(item, "dv_profile").unwrap_or(0) as i32,
                bl_signal_compatibility_id: number(item, "dv_bl_signal_compatibility_id")
                    .unwrap_or(0) as i32,
                rpu_present_flag: number(item, "rpu_present_flag").unwrap_or(0) as i32,
                bl_present_flag: number(item, "bl_present_flag").unwrap_or(0) as i32,
                el_present_flag: number(item, "el_present_flag").unwrap_or(0) as i32,
            });
        } else if kind.contains("SMPTE2094-40") || kind.contains("HDR10+") {
            metadata.hdr10_plus = true;
        }
    }

    metadata
}

fn number(value: &Value, key: &str) -> Option<i64> {
    match value.get(key)? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// Mastering information from the display the grade was authored on.
///
/// The probe reports these as ratios (`13250/50000`); they are resolved to the
/// value they denote so an encoder scale can be applied.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MasteringDisplay {
    pub green_x: f64,
    pub green_y: f64,
    pub blue_x: f64,
    pub blue_y: f64,
    pub red_x: f64,
    pub red_y: f64,
    pub white_point_x: f64,
    pub white_point_y: f64,
    pub max_luminance: f64,
    pub min_luminance: f64,
}

impl MasteringDisplay {
    /// Field order is the order the tokens list them in, not alphabetical.
    const FIELDS: [&str; 10] = [
        "green_x",
        "green_y",
        "blue_x",
        "blue_y",
        "red_x",
        "red_y",
        "white_point_x",
        "white_point_y",
        "max_luminance",
        "min_luminance",
    ];

    /// Reads one side-data entry, or nothing when the entry is incomplete.
    ///
    /// A partially described display is skipped rather than guessed: writing zero
    /// primaries would tag the stream with a display that never existed.
    pub fn parse(entry: &Value) -> Option<Self> {
        let object = entry.as_object()?;
        if !Self::FIELDS.iter().all(|key| object.contains_key(*key)) {
            return None;
        }

        Some(Self {
            green_x: float(object.get("green_x")),
            green_y: float(object.get("green_y")),
            blue_x: float(object.get("blue_x")),
            blue_y: float(object.get("blue_y")),
            red_x: float(object.get("red_x")),
            red_y: float(object.get("red_y")),
            white_point_x: float(object.get("white_point_x")),
            white_point_y: float(object.get("white_point_y")),
            max_luminance: float(object.get("max_luminance")),
            min_luminance: float(object.get("min_luminance")),
        })
    }

    /// `G()B()R()WP()L()` in the order both encoder families expect.
    fn segments(&self, encode: impl Fn(f64, bool) -> String) -> String {
        let chromaticity = |value: f64| encode(value, true);
        let luminance = |value: f64| encode(value, false);
        format!(
            "G({},{})B({},{})R({},{})WP({},{})L({},{})",
            chromaticity(self.green_x),
            chromaticity(self.green_y),
            chromaticity(self.blue_x),
            chromaticity(self.blue_y),
            chromaticity(self.red_x),
            chromaticity(self.red_y),
            chromaticity(self.white_point_x),
            chromaticity(self.white_point_y),
            luminance(self.max_luminance),
            luminance(self.min_luminance)
        )
    }
}

/// Peak content light levels, kept as the probe wrote them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentLight {
    pub max_content: String,
    pub max_average: String,
}

impl ContentLight {
    /// Reads one side-data entry, or nothing when either level is missing.
    pub fn parse(entry: &Value) -> Option<Self> {
        let object = entry.as_object()?;
        Some(Self {
            max_content: text(object.get("max_content")?),
            max_average: text(object.get("max_average")?),
        })
    }
}

/// Renders a side-data value exactly as the probe text reads it.
fn text(value: &Value) -> String {
    match value {
        Value::String(literal) => literal.clone(),
        other => other.to_string(),
    }
}

/// What one encode should tell the encoder about HDR metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HdrMetadataRequest<'a> {
    pub codec: Codec,
    pub output: Output,
    /// Effective interpretation, so an override to SDR drops the static tokens.
    pub source_kind: SourceKind,
    /// Dynamic metadata survives the encode; see `policy::preserve_dynamic_hdr`.
    pub preserve_dynamic: bool,
    /// Dolby Vision profile, when a DOVI record was found.
    pub dolby_vision_profile: Option<i32>,
    pub hdr10_plus: bool,
    /// Video and first-frame side data, as the probe reported it.
    pub side_data: &'a [Value],
}

/// Parameter entries and bare tokens that carry the metadata.
///
/// Entries are ordered: the rate-control floor, the HDR10+ payload, then the flags
/// that make the static tokens stick. Hosts append them after their own presets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HdrMetadataArguments {
    pub parameters: Vec<(String, String)>,
    pub appended: Vec<String>,
}

/// Composes the encoder parameters that carry HDR metadata.
pub fn encoder_metadata(request: &HdrMetadataRequest<'_>) -> HdrMetadataArguments {
    let mut parameters: Vec<(String, String)> = Vec::new();
    if request.preserve_dynamic
        && request.dolby_vision_profile.is_some()
        && request.codec == Codec::Hevc
    {
        // The Dolby Vision RPU needs headroom the default VBV cannot give it.
        parameters.push(("vbv-maxrate".to_owned(), "100000".to_owned()));
        parameters.push(("vbv-bufsize".to_owned(), "100000".to_owned()));
    }
    if request.preserve_dynamic && request.hdr10_plus {
        parameters.push(("dhdr10-info".to_owned(), "hdr10plus.json".to_owned()));
    }

    let mut appended = Vec::new();
    if request.output == Output::Hdr
        && request.source_kind == SourceKind::Pq
        && matches!(request.codec, Codec::Hevc | Codec::Av1)
    {
        appended = static_tokens(request.side_data, request.codec);
    }
    if request.codec == Codec::Hevc && !appended.is_empty() {
        parameters.push(("hdr10".to_owned(), "1".to_owned()));
        parameters.push(("hdr10-opt".to_owned(), "1".to_owned()));
        parameters.push(("repeat-headers".to_owned(), "1".to_owned()));
    }

    HdrMetadataArguments {
        parameters,
        appended,
    }
}

/// Master-display and content-light tokens in the encoder's own spelling.
fn static_tokens(side_data: &[Value], codec: Codec) -> Vec<String> {
    let mut tokens = Vec::new();
    let mastering = side_data
        .iter()
        .find(|entry| side_type(entry) == "Mastering display metadata")
        .and_then(MasteringDisplay::parse);
    if let Some(display) = mastering {
        tokens.push(match codec {
            // x265 takes the chromaticities in 1/50000 and the luminance in 1/10000.
            Codec::Hevc => format!("master-display={}", display.segments(hevc_segment)),
            // SVT-AV1 takes the same numbers as decimals with eight places.
            _ => format!("mastering-display={}", display.segments(av1_segment)),
        });
    }

    let light = side_data
        .iter()
        .find(|entry| side_type(entry) == "Content light level metadata")
        .and_then(ContentLight::parse);
    if let Some(light) = light {
        let flag = if codec == Codec::Hevc {
            "max-cll"
        } else {
            "content-light"
        };
        tokens.push(format!(
            "{flag}={},{}",
            light.max_content, light.max_average
        ));
    }

    tokens
}

fn hevc_segment(value: f64, chromaticity: bool) -> String {
    let scale = if chromaticity { 50000.0 } else { 10000.0 };
    ((value * scale).round() as i64).to_string()
}

fn av1_segment(value: f64, _chromaticity: bool) -> String {
    format!("{value:.8}")
}

fn side_type(entry: &Value) -> &str {
    entry
        .get("side_data_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// Reads a probe number: a plain float or a `numerator/denominator` ratio.
///
/// Unparsable or non-finite input becomes zero, which is what the host's shared
/// number reader has always answered for a missing field.
fn float(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(literal)) => ratio(literal.trim()),
        _ => 0.0,
    }
}

fn ratio(text: &str) -> f64 {
    if let Some((numerator, denominator)) = text.split_once('/') {
        let numerator: f64 = numerator.trim().parse().unwrap_or(f64::NAN);
        let denominator: f64 = denominator.trim().parse().unwrap_or(f64::NAN);
        let quotient = numerator / denominator;
        return if quotient.is_finite() { quotient } else { 0.0 };
    }

    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => value,
        _ => 0.0,
    }
}
