//! Normalized source description.
//!
//! The probe adapter fills this from ffprobe (Windows) or AVFoundation
//! (macOS). Rules never inspect backend-specific JSON, so both platforms reach
//! the same format decision from the same fields.

use crate::domain::{LogProfile, SourceKind};
use crate::error::{CoreError, CoreResult, codes};

/// Pixel-format facts needed by the output policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PictureFormat {
    pub pixel_format: String,
    /// `AV_PIX_FMT_FLAG_RGB`: components are color channels, not YCbCr.
    pub rgb: bool,
    /// Palette formats carry RGB values in a lookup table.
    pub palette: bool,
    /// Bayer/RAW descriptors count sensel coverage rather than chroma.
    pub bayer: bool,
    /// Alpha is stored but never delivered.
    pub alpha: bool,
    /// Detected grayscale source; chroma is neutral rather than subsampled.
    pub monochrome: bool,
    /// Per-component storage depth in file order, alpha included.
    pub components: Vec<i32>,
    /// `nb_components`, used when descriptors are unavailable.
    pub component_count: Option<i32>,
    pub bits_per_raw_sample: i32,
    pub bits_per_sample: i32,
    pub log2_chroma_w: Option<i32>,
    pub log2_chroma_h: Option<i32>,
    /// Container/stream range tag, before any camera-profile fallback.
    pub color_range: Option<String>,
}

/// Dolby Vision compatibility layer reported by the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
pub struct DolbyVisionConfig {
    #[serde(default)]
    pub profile: i32,
    #[serde(default)]
    pub bl_signal_compatibility_id: i32,
    #[serde(default)]
    pub rpu_present_flag: i32,
    #[serde(default)]
    pub bl_present_flag: i32,
    #[serde(default)]
    pub el_present_flag: i32,
}

impl DolbyVisionConfig {
    /// Single-layer profiles this release can carry through a re-encode.
    pub fn is_supported_layer(&self) -> bool {
        self.rpu_present_flag == 1
            && self.bl_present_flag == 1
            && self.el_present_flag == 0
            && (self.profile == 5 && self.bl_signal_compatibility_id == 0
                || matches!(self.profile, 8 | 10)
                    && matches!(self.bl_signal_compatibility_id, 1 | 4))
    }

    /// Profile 5 has no base-layer compatibility metadata and needs HEVC.
    pub fn is_profile5(&self) -> bool {
        self.profile == 5
    }
}

/// Dynamic HDR payloads found on the video stream or its first frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
pub struct DynamicHdrInfo {
    #[serde(default)]
    pub dolby_vision: Option<DolbyVisionConfig>,
    #[serde(default)]
    pub hdr10_plus: bool,
}

impl DynamicHdrInfo {
    pub const fn is_present(&self) -> bool {
        self.dolby_vision.is_some() || self.hdr10_plus
    }
}

/// A declared audio track, used by the FLAC eligibility rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStream {
    pub codec_name: String,
    pub profile: String,
    pub sample_format: String,
    pub bits_per_raw_sample: i32,
    pub bits_per_sample: i32,
    pub channels: i32,
    pub channel_layout: Option<String>,
}

/// Everything the core needs to know about one source file's first video stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFormat {
    pub picture: PictureFormat,
    /// Detected interpretation, before the manual override is applied.
    pub kind: SourceKind,
    /// Camera curve recognized from metadata, if any.
    pub profile: Option<LogProfile>,
    /// ProRes RAW is present and decodable by the configured FFmpeg.
    pub raw: bool,
    /// The configured FFmpeg supports the camera's RAW parameters.
    pub raw_decoder_supported: bool,
    /// The configured FFmpeg is new enough for the RAW development path.
    pub raw_decoder_version_supported: bool,
    /// A display matrix drives rotation/mirroring during decode.
    pub display_matrix: bool,
    pub dynamic_hdr: DynamicHdrInfo,
    pub audio: Vec<AudioStream>,
}

impl SourceFormat {
    /// Effective interpretation after the request's manual override.
    pub fn effective_kind(&self, override_kind: Option<SourceKind>) -> SourceKind {
        override_kind.unwrap_or(self.kind)
    }

    /// Whether the source carries an alpha channel that delivery would drop.
    pub fn has_alpha(&self) -> bool {
        self.picture.alpha
    }
}

/// Wire form of a source description.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SourceDto {
    #[serde(default)]
    pub pixel_format: Option<String>,
    #[serde(default)]
    pub rgb: bool,
    #[serde(default)]
    pub palette: bool,
    #[serde(default)]
    pub bayer: bool,
    #[serde(default)]
    pub alpha: bool,
    #[serde(default)]
    pub monochrome: bool,
    #[serde(default)]
    pub components: Vec<i32>,
    #[serde(default)]
    pub component_count: Option<i32>,
    #[serde(default)]
    pub bits_per_raw_sample: Option<i32>,
    #[serde(default)]
    pub bits_per_sample: Option<i32>,
    #[serde(default)]
    pub log2_chroma_w: Option<i32>,
    #[serde(default)]
    pub log2_chroma_h: Option<i32>,
    #[serde(default)]
    pub color_range: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub raw: bool,
    #[serde(default = "default_true")]
    pub raw_decoder_supported: bool,
    #[serde(default = "default_true")]
    pub raw_decoder_version_supported: bool,
    #[serde(default)]
    pub display_matrix: bool,
    #[serde(default)]
    pub dynamic_hdr: DynamicHdrInfo,
    #[serde(default)]
    pub audio: Vec<AudioDto>,
}

/// Wire form of one audio track.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct AudioDto {
    #[serde(default)]
    pub codec_name: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub sample_format: Option<String>,
    #[serde(default)]
    pub bits_per_raw_sample: Option<i32>,
    #[serde(default)]
    pub bits_per_sample: Option<i32>,
    #[serde(default)]
    pub channels: Option<i32>,
    #[serde(default)]
    pub channel_layout: Option<String>,
}

impl SourceDto {
    /// Converts the wire form, defaulting unknown interpretation to `unknown`.
    pub fn into_source(self) -> CoreResult<SourceFormat> {
        let kind = match self.kind.as_deref() {
            None | Some("") => SourceKind::Unknown,
            Some(value) => SourceKind::parse(value)?,
        };
        let profile = match self.profile.as_deref() {
            None | Some("") | Some("auto") => None,
            Some(value) => LogProfile::parse(value)?,
        };
        Ok(SourceFormat {
            picture: PictureFormat {
                pixel_format: self.pixel_format.unwrap_or_default(),
                rgb: self.rgb,
                palette: self.palette,
                bayer: self.bayer,
                alpha: self.alpha,
                monochrome: self.monochrome,
                components: self.components,
                component_count: self.component_count,
                bits_per_raw_sample: self.bits_per_raw_sample.unwrap_or(0),
                bits_per_sample: self.bits_per_sample.unwrap_or(0),
                log2_chroma_w: self.log2_chroma_w,
                log2_chroma_h: self.log2_chroma_h,
                color_range: self.color_range.filter(|value| !value.is_empty()),
            },
            kind,
            profile,
            raw: self.raw,
            raw_decoder_supported: self.raw_decoder_supported,
            raw_decoder_version_supported: self.raw_decoder_version_supported,
            display_matrix: self.display_matrix,
            dynamic_hdr: self.dynamic_hdr,
            audio: self
                .audio
                .into_iter()
                .map(|track| AudioStream {
                    codec_name: track.codec_name.unwrap_or_default(),
                    profile: track.profile.unwrap_or_default(),
                    sample_format: track.sample_format.unwrap_or_default(),
                    bits_per_raw_sample: track.bits_per_raw_sample.unwrap_or(0),
                    bits_per_sample: track.bits_per_sample.unwrap_or(0),
                    channels: track.channels.unwrap_or(0),
                    channel_layout: track.channel_layout.filter(|value| !value.is_empty()),
                })
                .collect(),
        })
    }
}

/// Rejects unknown sampling layouts before they reach a planning rule.
pub fn require_known_sampling(sampling: crate::domain::Sampling) -> CoreResult<()> {
    if sampling == crate::domain::Sampling::Unknown {
        return Err(CoreError::new(codes::POLICY_SAMPLING_CHOICE));
    }
    Ok(())
}

const fn default_true() -> bool {
    true
}
