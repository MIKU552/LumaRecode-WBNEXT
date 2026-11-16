//! Dynamic HDR metadata read from probe side data.
//!
//! Dolby Vision and HDR10+ arrive as side-data entries whose names differ slightly
//! between FFmpeg builds, so the reader matches on the type name instead of an exact key
//! and fills the values the output policy already understands.

use serde_json::Value;

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
