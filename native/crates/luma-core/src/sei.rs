//! HDR10+ dynamic metadata, read from the SEI payload a stream carries.
//!
//! The host extracts the payload — it owns the file and the demuxer — and the core turns
//! the bits into the same pipe-delimited line FFprobe writes, so both hosts build the
//! `dhdr10-info` document from one reader.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::error::{CoreError, CoreResult, codes};

/// Bytes every HEVC HDR10+ payload starts with: the country code, terminal provider code,
/// provider oriented code and application identifier.
const T35_PREFIX: [u8; 6] = [0xb5, 0x00, 0x3c, 0x00, 0x01, 0x04];

/// One frame's metadata, spelled the way FFprobe's compact writer spells it.
pub fn t35_line(payload: &[u8]) -> CoreResult<String> {
    if !payload.starts_with(&T35_PREFIX) {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }

    let mut bits = Bits::new(&payload[T35_PREFIX.len()..]);
    let mut line = String::from("frame|side_data_type=HDR Dynamic Metadata SMPTE2094-40 (HDR10+)");
    let field = |line: &mut String, key: &str, value: u32, scale: Option<u32>| {
        line.push('|');
        line.push_str(key);
        line.push('=');
        line.push_str(&value.to_string());
        if let Some(scale) = scale {
            line.push('/');
            line.push_str(&scale.to_string());
        }
    };

    field(&mut line, "application version", bits.read(8)?, None);
    let windows = bits.read(2)?;
    if windows != 1 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    field(&mut line, "num_windows", windows, None);
    field(
        &mut line,
        "targeted_system_display_maximum_luminance",
        bits.read(27)?,
        None,
    );
    if bits.read(1)? != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    for _ in 0..3 {
        field(&mut line, "maxscl", bits.read(17)?, Some(100_000));
    }
    field(&mut line, "average_maxrgb", bits.read(17)?, Some(100_000));
    let count = bits.read(4)?;
    field(
        &mut line,
        "num_distribution_maxrgb_percentiles",
        count,
        None,
    );
    for _ in 0..count {
        field(
            &mut line,
            "distribution_maxrgb_percentage",
            bits.read(7)?,
            None,
        );
        field(
            &mut line,
            "distribution_maxrgb_percentile",
            bits.read(17)?,
            Some(100_000),
        );
    }
    field(
        &mut line,
        "fraction_bright_pixels",
        bits.read(10)?,
        Some(1000),
    );
    if bits.read(1)? != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    if bits.read(1)? != 0 {
        field(&mut line, "knee_point_x", bits.read(12)?, Some(4095));
        field(&mut line, "knee_point_y", bits.read(12)?, Some(4095));
        let anchors = bits.read(4)?;
        field(&mut line, "num_bezier_curve_anchors", anchors, None);
        for _ in 0..anchors {
            field(
                &mut line,
                "bezier_curve_anchors",
                bits.read(10)?,
                Some(1023),
            );
        }
    }
    if bits.read(1)? != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    // Only the trailing-bit padding may be left, and it has to be zeros.
    while bits.remaining() > 0 {
        if bits.read(1)? != 0 {
            return Err(CoreError::new(codes::METADATA_HDR10PLUS));
        }
    }

    Ok(line)
}

/// MSB-first bit reader over the payload that follows the six fixed bytes.
struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() * 8 - self.position
    }

    fn read(&mut self, count: u32) -> CoreResult<u32> {
        if count as usize > self.remaining() {
            return Err(CoreError::new(codes::METADATA_HDR10PLUS));
        }
        let mut value = 0u32;
        for _ in 0..count {
            let byte = self.bytes[self.position / 8];
            value = (value << 1) | u32::from(byte >> (7 - self.position % 8) & 1);
            self.position += 1;
        }
        Ok(value)
    }
}

/// Name FFprobe gives the metadata this module understands.
const TYPE_NAME: &str = "HDR Dynamic Metadata SMPTE2094-40 (HDR10+)";

/// Line keys that can be carried into `dhdr10-info` without loss.
const FIELDS: [&str; 13] = [
    "application version",
    "num_windows",
    "targeted_system_display_maximum_luminance",
    "maxscl",
    "average_maxrgb",
    "num_distribution_maxrgb_percentiles",
    "distribution_maxrgb_percentage",
    "distribution_maxrgb_percentile",
    "fraction_bright_pixels",
    "knee_point_x",
    "knee_point_y",
    "num_bezier_curve_anchors",
    "bezier_curve_anchors",
];

/// Builds the `dhdr10-info` document from one line per frame.
///
/// The document is what x265 reads back, and it carries three indices x265 needs: which
/// scene a frame belongs to, where it sits inside that scene, and where it sits in the
/// sequence. A frame whose metadata changes starts a new scene; a frame without metadata is
/// refused rather than silently shifting every later index by one.
pub fn document(lines: &[String]) -> CoreResult<Value> {
    let mut scenes: Vec<Value> = Vec::new();
    let mut profile: Option<&str> = None;
    let mut previous: Option<String> = None;
    let mut sequence = 0u64;
    let mut scene: i64 = -1;
    let mut scene_start = 0u64;

    for line in lines {
        let frame = frame_fields(line)?;
        let current = Value::Object(frame.clone()).to_string();
        let frame_profile = if frame.contains_key("BezierCurveData") {
            "B"
        } else {
            "A"
        };
        if let Some(known) = profile {
            if known != frame_profile {
                return Err(CoreError::new(codes::METADATA_HDR10PLUS));
            }
        }
        profile = Some(frame_profile);
        if previous.as_deref() != Some(current.as_str()) {
            scene += 1;
            scene_start = sequence;
            previous = Some(current);
        }
        let mut frame = frame;
        frame.insert("SceneFrameIndex".to_owned(), json!(sequence - scene_start));
        frame.insert("SceneId".to_owned(), json!(scene));
        frame.insert("SequenceFrameIndex".to_owned(), json!(sequence));
        sequence += 1;
        scenes.push(Value::Object(frame));
    }

    let Some(profile) = profile else {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    };
    Ok(json!({
        "SceneInfo": scenes,
        "JSONInfo": {"HDR10plusProfile": profile, "Version": "1.0"},
        "ToolInfo": {"Tool": "LumaRecode"},
    }))
}

/// One frame's metadata, validated the way x265's reader expects it.
fn frame_fields(line: &str) -> CoreResult<Map<String, Value>> {
    let mut values: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut selected = false;
    let mut found = false;
    for entry in line.split('|') {
        let Some(separator) = entry.find('=') else {
            continue;
        };
        let raw = &entry[..separator];
        let key = raw.rfind(':').map_or(raw, |index| &raw[index + 1..]);
        let value = &entry[separator + 1..];
        if key == "side_data_type" {
            selected = value == TYPE_NAME;
            if selected && found {
                return Err(CoreError::new(codes::METADATA_HDR10PLUS));
            }
            found |= selected;
        } else if selected {
            if !FIELDS.contains(&key) {
                return Err(CoreError::new(codes::METADATA_HDR10PLUS));
            }
            values.entry(key).or_default().push(value);
        }
    }
    if !found {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }

    let many = |key: &str, scale: i64, maximum: i64| -> CoreResult<Vec<i64>> {
        values
            .get(key)
            .map(|items| {
                items
                    .iter()
                    .map(|item| number(item, scale, maximum))
                    .collect()
            })
            .unwrap_or_else(|| Ok(Vec::new()))
    };
    let one = |key: &str, scale: i64, maximum: i64| -> CoreResult<i64> {
        let items = many(key, scale, maximum)?;
        if items.len() == 1 {
            Ok(items[0])
        } else {
            Err(CoreError::new(codes::METADATA_HDR10PLUS))
        }
    };

    // The reader below only carries what x265 can write back.
    if one("application version", 1, 255)? != 1
        || one("num_windows", 1, 1)? != 1
        || one("fraction_bright_pixels", 1000, 1000)? != 0
    {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    let target = one("targeted_system_display_maximum_luminance", 1, 10_000)?;
    let max_scl = many("maxscl", 100_000, 100_000)?;
    let indices = many("distribution_maxrgb_percentage", 1, 100)?;
    let distributions = many("distribution_maxrgb_percentile", 100_000, 100_000)?;
    let count = one("num_distribution_maxrgb_percentiles", 1, 15)?;
    let ascending = indices.windows(2).all(|pair| pair[0] < pair[1]);
    if max_scl.len() != 3
        || indices.len() as i64 != count
        || distributions.len() as i64 != count
        || !ascending
    {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }

    let mut frame = Map::new();
    frame.insert(
        "LuminanceParameters".to_owned(),
        json!({
            "AverageRGB": one("average_maxrgb", 100_000, 100_000)?,
            "LuminanceDistributions": {
                "DistributionIndex": indices,
                "DistributionValues": distributions,
            },
            "MaxScl": max_scl,
        }),
    );
    frame.insert("NumberOfWindows".to_owned(), json!(1));
    frame.insert(
        "TargetedSystemDisplayMaximumLuminance".to_owned(),
        json!(target),
    );
    if values.contains_key("knee_point_x") {
        let anchors = many("bezier_curve_anchors", 1023, 1023)?;
        let anchors_count = one("num_bezier_curve_anchors", 1, 15)?;
        if target == 0 || anchors_count == 0 || anchors.len() as i64 != anchors_count {
            return Err(CoreError::new(codes::METADATA_HDR10PLUS));
        }
        frame.insert(
            "BezierCurveData".to_owned(),
            json!({
                "Anchors": anchors,
                "KneePointX": one("knee_point_x", 4095, 4095)?,
                "KneePointY": one("knee_point_y", 4095, 4095)?,
            }),
        );
    }
    Ok(frame)
}

/// Reads `numerator/denominator` and scales it, refusing off-grid values.
fn number(text: &str, scale: i64, maximum: i64) -> CoreResult<i64> {
    let mut parts = text.split('/');
    let numerator: i64 = parts
        .next()
        .and_then(|part| part.parse().ok())
        .ok_or_else(|| CoreError::new(codes::METADATA_HDR10PLUS))?;
    let denominator: i64 = match parts.next() {
        None => 1,
        Some(part) => part
            .parse()
            .map_err(|_| CoreError::new(codes::METADATA_HDR10PLUS))?,
    };
    if parts.next().is_some() || denominator <= 0 || numerator < 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    if i128::from(numerator) > i128::from(maximum) * i128::from(denominator) / i128::from(scale) {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    let scaled = i128::from(numerator) * i128::from(scale);
    if scaled % i128::from(denominator) != 0 {
        return Err(CoreError::new(codes::METADATA_HDR10PLUS));
    }
    Ok((scaled / i128::from(denominator)) as i64)
}
