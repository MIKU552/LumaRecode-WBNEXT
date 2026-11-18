//! Encoder preset registry.
//!
//! The registry is data, not code: `Assets/recommended-presets.json` stays the
//! single source of measured values (QSV ICQ baselines included), and every
//! backend reads the same table. Selecting a preset never guesses a quality
//! number for a vendor encoder from another vendor's scale.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::domain::{Codec, Purpose, Variant};
use crate::error::{CoreError, CoreResult, codes};

/// Quality/rate-control preset for one purpose, codec and effort.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoPreset {
    /// Table the preset came from, e.g. `share/hevc` or `archive_fast_av1`.
    pub selector: String,
    pub encoder: String,
    pub preset: String,
    /// CRF for software encoders, ICQ/QP index for hardware encoders.
    pub quality: i64,
    /// Baseline pixel format declared by the table, if any.
    pub declared_pixel_format: Option<String>,
    pub tune: Option<i64>,
    pub lookahead: Option<i64>,
    pub async_depth: Option<i64>,
    /// Encoder-specific options forwarded as `-<key> <value>` pairs.
    pub parameters: BTreeMap<String, Value>,
}

/// Sample rate requested by an audio preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRate {
    /// Keep the source rate.
    Source,
    Fixed(u32),
}

/// Lossy audio preset used when a share output does not copy the source.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioPreset {
    pub encoder: String,
    pub bitrate_kbps: u32,
    pub channels: u8,
    pub sample_rate: SampleRate,
    pub vbr: Option<String>,
    pub application: Option<String>,
    pub compression_level: Option<i64>,
}

/// Audio handling implied by purpose and codec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioDefault {
    /// Archive keeps every original track.
    Copy,
    /// Share re-encodes to the named registry preset (`aac`, `opus`).
    Encode(String),
}

/// Parsed, validated view over the shipped preset tables.
#[derive(Debug, Clone)]
pub struct PresetRegistry {
    root: Value,
}

fn require_object<'a>(value: &'a Value, path: &str) -> CoreResult<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| CoreError::with_detail(codes::PRESET_INVALID, path))
}

fn map_text(map: &Map<String, Value>, key: &str, path: &str) -> CoreResult<String> {
    match map.get(key) {
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::Number(number)) => Ok(number.to_string()),
        _ => Err(CoreError::with_detail(
            codes::PRESET_INVALID,
            format!("{path}.{key}"),
        )),
    }
}

/// Effort name for a backend that has one.
///
/// Some backends take no effort knob at all — VideoToolbox has `-q:v` and nothing like
/// `-preset` — so the field is optional rather than a name the host has to invent.
fn optional_text(map: &Map<String, Value>, key: &str) -> String {
    map.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn map_int(map: &Map<String, Value>, key: &str, path: &str) -> CoreResult<i64> {
    match map.get(key) {
        Some(Value::Number(number)) => number
            .as_i64()
            .ok_or_else(|| CoreError::with_detail(codes::PRESET_INVALID, format!("{path}.{key}"))),
        Some(Value::String(text)) => text
            .parse::<i64>()
            .map_err(|_| CoreError::with_detail(codes::PRESET_INVALID, format!("{path}.{key}"))),
        _ => Err(CoreError::with_detail(
            codes::PRESET_INVALID,
            format!("{path}.{key}"),
        )),
    }
}

fn text_field(value: &Value, key: &str, path: &str) -> CoreResult<String> {
    map_text(require_object(value, path)?, key, path)
}

fn int_field(value: &Value, key: &str, path: &str) -> CoreResult<i64> {
    map_int(require_object(value, path)?, key, path)
}

fn optional_int(value: &Value, key: &str) -> Option<i64> {
    match value.get(key) {
        Some(Value::Number(number)) => number.as_i64(),
        Some(Value::String(text)) => text.parse().ok(),
        _ => None,
    }
}

impl PresetRegistry {
    /// Parses the preset document and rejects tables a backend would need but
    /// cannot find, so a broken asset fails at startup instead of mid-encode.
    pub fn parse(document: &str) -> CoreResult<Self> {
        let root: Value =
            serde_json::from_str(document).map_err(|_| CoreError::new(codes::PRESET_INVALID))?;
        let registry = Self { root };
        for purpose in [Purpose::Share, Purpose::Archive] {
            for codec in [Codec::Hevc, Codec::Av1, Codec::Avc] {
                registry.video(purpose, codec, Variant::Standard)?;
            }
        }
        registry.video(Purpose::Archive, Codec::Av1, Variant::Fast)?;
        registry.audio(Purpose::Share, Codec::Hevc)?;
        registry.audio(Purpose::Share, Codec::Avc)?;
        registry.audio(Purpose::Share, Codec::Av1)?;
        registry.audio(Purpose::Archive, Codec::Hevc)?;
        Ok(registry)
    }

    /// Video preset for a purpose, codec and effort level.
    pub fn video(
        &self,
        purpose: Purpose,
        codec: Codec,
        variant: Variant,
    ) -> CoreResult<VideoPreset> {
        if variant == Variant::Fast && !(purpose == Purpose::Archive && codec == Codec::Av1) {
            // The standby profile exists only for the slowest software encoder.
            return Err(CoreError::new(codes::PRESET_VARIANT));
        }
        let selector = if variant == Variant::Fast {
            "archive_fast_av1".to_owned()
        } else {
            format!("{}/{}", purpose.as_str(), codec.as_str())
        };
        let table = if variant == Variant::Fast {
            self.root
                .get(&selector)
                .ok_or_else(|| CoreError::with_detail(codes::PRESET_UNKNOWN, selector.clone()))?
        } else {
            self.root
                .get(purpose.as_str())
                .and_then(|node| node.get(codec.as_str()))
                .ok_or_else(|| CoreError::with_detail(codes::PRESET_UNKNOWN, selector.clone()))?
        };
        let mut parameters = BTreeMap::new();
        for (key, value) in require_object(table, &selector)? {
            match key.as_str() {
                "encoder" | "preset" | "quality" | "pixel_format" | "tune" | "lp"
                | "async_depth" => {}
                "encoder_parameters" => {
                    for (name, option) in
                        require_object(value, &format!("{selector}.encoder_parameters"))?
                    {
                        parameters.insert(name.clone(), option.clone());
                    }
                }
                other => {
                    parameters.insert(other.to_owned(), value.clone());
                }
            }
        }
        Ok(VideoPreset {
            selector: selector.clone(),
            encoder: text_field(table, "encoder", &selector)?,
            preset: optional_text(require_object(table, &selector)?, "preset"),
            quality: int_field(table, "quality", &selector)?,
            declared_pixel_format: table
                .get("pixel_format")
                .and_then(Value::as_str)
                .map(str::to_owned),
            tune: optional_int(table, "tune"),
            lookahead: optional_int(table, "lp"),
            async_depth: optional_int(table, "async_depth"),
            parameters,
        })
    }

    /// Audio handling implied by the purpose and codec.
    pub fn audio(&self, purpose: Purpose, codec: Codec) -> CoreResult<AudioDefault> {
        let path = format!("audio.{}", purpose.as_str());
        let table = self
            .root
            .get("audio")
            .and_then(Value::as_object)
            .ok_or_else(|| CoreError::with_detail(codes::PRESET_INVALID, "audio"))?;
        match purpose {
            Purpose::Archive => {
                let value = map_text(table, "archive_default", "audio")?;
                if value == "copy" {
                    Ok(AudioDefault::Copy)
                } else {
                    Ok(AudioDefault::Encode(value))
                }
            }
            Purpose::Share => {
                let defaults = table
                    .get("share_default")
                    .ok_or_else(|| CoreError::with_detail(codes::PRESET_INVALID, &path))?;
                let name = text_field(defaults, codec.as_str(), &path)?;
                Ok(AudioDefault::Encode(name))
            }
        }
    }

    /// Parameters for a named lossy audio preset.
    pub fn audio_preset(&self, name: &str) -> CoreResult<AudioPreset> {
        let path = format!("audio.{name}");
        let table = self
            .root
            .get("audio")
            .and_then(|audio| audio.get(name))
            .ok_or_else(|| CoreError::with_detail(codes::PRESET_UNKNOWN, path.clone()))?;
        let sample_rate = match table.get("sample_rate") {
            Some(Value::Number(number)) => {
                SampleRate::Fixed(number.as_u64().unwrap_or(48_000) as u32)
            }
            Some(Value::String(text)) if text == "source" => SampleRate::Source,
            Some(Value::String(text)) => SampleRate::Fixed(text.parse().map_err(|_| {
                CoreError::with_detail(codes::PRESET_INVALID, format!("{path}.sample_rate"))
            })?),
            _ => SampleRate::Source,
        };
        Ok(AudioPreset {
            encoder: text_field(table, "encoder", &path)?,
            bitrate_kbps: u32::try_from(int_field(table, "bitrate_kbps", &path)?).map_err(
                |_| CoreError::with_detail(codes::PRESET_INVALID, format!("{path}.bitrate_kbps")),
            )?,
            channels: u8::try_from(int_field(table, "channels", &path)?).map_err(|_| {
                CoreError::with_detail(codes::PRESET_INVALID, format!("{path}.channels"))
            })?,
            sample_rate,
            vbr: table.get("vbr").and_then(Value::as_str).map(str::to_owned),
            application: table
                .get("application")
                .and_then(Value::as_str)
                .map(str::to_owned),
            compression_level: optional_int(table, "compression_level"),
        })
    }

    /// Every selectable video preset, in a stable order for `presets` output.
    pub fn catalog(&self) -> Vec<VideoPreset> {
        let mut entries = Vec::new();
        for purpose in [Purpose::Share, Purpose::Archive] {
            for codec in [Codec::Avc, Codec::Hevc, Codec::Av1] {
                if let Ok(preset) = self.video(purpose, codec, Variant::Standard) {
                    entries.push(preset);
                }
            }
        }
        if let Ok(fast) = self.video(Purpose::Archive, Codec::Av1, Variant::Fast) {
            entries.push(fast);
        }
        entries
    }
}
