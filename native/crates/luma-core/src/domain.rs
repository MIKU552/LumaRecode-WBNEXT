//! Domain vocabulary for encode requests.
//!
//! Hosts exchange the same string values as the existing CLI, so a GUI
//! resolution and a CLI resolution always describe the same request. Parsing
//! happens once at the boundary; the rest of the core works with enums.

use crate::error::{CoreError, CoreResult, codes};

/// What the output is for. Drives encoder family, defaults and container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// Quick sharing: hardware path, smaller files.
    Share,
    /// Software archive: precision and metadata retained.
    Archive,
}

impl Purpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Share => "share",
            Self::Archive => "archive",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "share" => Ok(Self::Share),
            "archive" => Ok(Self::Archive),
            other => Err(CoreError::with_detail(codes::OPTION_PURPOSE, other)),
        }
    }
}

/// Target video codec. Backend selection stays separate from this choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codec {
    Hevc,
    Av1,
    Avc,
}

impl Codec {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hevc => "hevc",
            Self::Av1 => "av1",
            Self::Avc => "avc",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "hevc" => Ok(Self::Hevc),
            "av1" => Ok(Self::Av1),
            "avc" => Ok(Self::Avc),
            other => Err(CoreError::with_detail(codes::OPTION_CODEC, other)),
        }
    }
}

/// Transfer-function family of the delivered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Output {
    Sdr,
    Hdr,
}

impl Output {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sdr => "sdr",
            Self::Hdr => "hdr",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "sdr" => Ok(Self::Sdr),
            "hdr" => Ok(Self::Hdr),
            other => Err(CoreError::with_detail(codes::OPTION_OUTPUT, other)),
        }
    }
}

/// Color handling group. `None` passes the source through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transform {
    None,
    LogHdr,
    LogSdr,
    HdrSdr,
}

impl Transform {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LogHdr => "log-hdr",
            Self::LogSdr => "log-sdr",
            Self::HdrSdr => "hdr-sdr",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "none" => Ok(Self::None),
            "log-hdr" => Ok(Self::LogHdr),
            "log-sdr" => Ok(Self::LogSdr),
            "hdr-sdr" => Ok(Self::HdrSdr),
            other => Err(CoreError::with_detail(codes::OPTION_TRANSFORM, other)),
        }
    }
}

/// Camera Log curves with a shipped LUT.
///
/// The matrix and range describe how YCbCr enters the LUT's RGB domain; they
/// do not describe the gamut the camera curve retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogProfile {
    Pocket3,
    Samsung,
    MiLog,
    AppleLog,
    AppleLog2,
}

impl LogProfile {
    pub const ALL: [Self; 5] = [
        Self::Pocket3,
        Self::Samsung,
        Self::MiLog,
        Self::AppleLog,
        Self::AppleLog2,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pocket3 => "pocket3",
            Self::Samsung => "samsung",
            Self::MiLog => "milog",
            Self::AppleLog => "applelog",
            Self::AppleLog2 => "applelog2",
        }
    }

    pub const fn lut_file(self) -> &'static str {
        match self {
            Self::Pocket3 => "DJI_Osmo_DLogM_to_Rec2100_HLG_BT2408_Neutral.cube",
            Self::Samsung => "Samsung_Log_to_Rec2100_HLG.cube",
            Self::MiLog => "REDMI_K90_MiLog_to_Rec2100_HLG.cube",
            Self::AppleLog => "Apple_Log_to_Rec2100_HLG.cube",
            Self::AppleLog2 => "Apple_Log_2_to_Rec2100_HLG.cube",
        }
    }

    /// Fallback YCbCr matrix when the file carries no usable `color_space` tag.
    pub const fn matrix(self) -> &'static str {
        match self {
            Self::Pocket3 => "709",
            Self::Samsung | Self::MiLog | Self::AppleLog | Self::AppleLog2 => "2020_ncl",
        }
    }

    /// Sample range implied by the camera curve when the file is untagged.
    pub const fn range(self) -> ColorRange {
        match self {
            Self::MiLog => ColorRange::Full,
            Self::Pocket3 | Self::Samsung | Self::AppleLog | Self::AppleLog2 => ColorRange::Limited,
        }
    }

    pub fn parse(value: &str) -> CoreResult<Option<Self>> {
        match value {
            "" | "auto" => Ok(None),
            "pocket3" => Ok(Some(Self::Pocket3)),
            "samsung" => Ok(Some(Self::Samsung)),
            "milog" => Ok(Some(Self::MiLog)),
            "applelog" => Ok(Some(Self::AppleLog)),
            "applelog2" => Ok(Some(Self::AppleLog2)),
            other => Err(CoreError::with_detail(codes::OPTION_PROFILE, other)),
        }
    }
}

/// The frame geometry and white balance a RAW development needs.
#[derive(Debug, Clone, PartialEq)]
pub struct RawGeometry {
    /// Sensor width in pixels.
    pub width: u32,
    /// Sensor height in pixels.
    pub height: u32,
    /// Left, right, top and bottom borders the demosaic leaves out.
    pub crop: [u32; 4],
    /// Row-major 3x4 matrix the camera reported, applied per channel.
    pub matrix: [f64; 12],
}

/// How the input's color encoding is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Sdr,
    Hlg,
    Pq,
    Log,
    Raw,
    Unknown,
}

impl SourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sdr => "sdr",
            Self::Hlg => "hlg",
            Self::Pq => "pq",
            Self::Log => "log",
            Self::Raw => "raw",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_hdr(self) -> bool {
        matches!(self, Self::Hlg | Self::Pq)
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "sdr" => Ok(Self::Sdr),
            "hlg" => Ok(Self::Hlg),
            "pq" => Ok(Self::Pq),
            "log" => Ok(Self::Log),
            "raw" => Ok(Self::Raw),
            "unknown" => Ok(Self::Unknown),
            other => Err(CoreError::with_detail(codes::OPTION_SOURCE_KIND, other)),
        }
    }
}

/// Chroma sampling. Shifts are powers of two per axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sampling {
    Yuv420,
    Yuv422,
    Yuv444,
    Yuv440,
    Yuv411,
    Yuv410,
    Gray,
    Unknown,
}

impl Sampling {
    /// `(horizontal, vertical)` chroma shift, or `None` when the layout has no
    /// comparable YCbCr geometry.
    pub const fn shifts(self) -> Option<(u8, u8)> {
        match self {
            Self::Yuv444 => Some((0, 0)),
            Self::Yuv422 => Some((1, 0)),
            Self::Yuv420 => Some((1, 1)),
            Self::Yuv440 => Some((0, 1)),
            Self::Yuv411 => Some((2, 0)),
            Self::Yuv410 => Some((2, 2)),
            Self::Gray | Self::Unknown => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Yuv420 => "420",
            Self::Yuv422 => "422",
            Self::Yuv444 => "444",
            Self::Yuv440 => "440",
            Self::Yuv411 => "411",
            Self::Yuv410 => "410",
            Self::Gray => "gray",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "420" => Some(Self::Yuv420),
            "422" => Some(Self::Yuv422),
            "444" => Some(Self::Yuv444),
            "440" => Some(Self::Yuv440),
            "411" => Some(Self::Yuv411),
            "410" => Some(Self::Yuv410),
            "gray" => Some(Self::Gray),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Formats comparable for delivery decisions, widest first.
    pub const DELIVERABLE: [Self; 3] = [Self::Yuv420, Self::Yuv422, Self::Yuv444];
}

/// Sample range of the encoded signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRange {
    Limited,
    Full,
}

impl ColorRange {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Limited => "limited",
            Self::Full => "full",
        }
    }
}

/// Requested sampling, including the two source-dependent modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingChoice {
    Auto,
    Source,
    Fixed(Sampling),
}

impl SamplingChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Source => "source",
            Self::Fixed(sampling) => sampling.label(),
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "source" => Ok(Self::Source),
            other => Sampling::from_label(other)
                .map(Self::Fixed)
                .ok_or_else(|| CoreError::with_detail(codes::OPTION_SAMPLING, other)),
        }
    }
}

/// Requested sample range, including the two source-dependent modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeChoice {
    Auto,
    Source,
    Fixed(ColorRange),
}

impl RangeChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Source => "source",
            Self::Fixed(range) => range.as_str(),
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "source" => Ok(Self::Source),
            "limited" => Ok(Self::Fixed(ColorRange::Limited)),
            "full" => Ok(Self::Fixed(ColorRange::Full)),
            other => Err(CoreError::with_detail(codes::OPTION_RANGE, other)),
        }
    }
}

/// Requested precision, including the source- and encoder-bounded mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthChoice {
    Auto,
    Bits(u8),
}

impl DepthChoice {
    pub fn label(self) -> String {
        match self {
            Self::Auto => "auto".to_owned(),
            Self::Bits(bits) => bits.to_string(),
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "8" => Ok(Self::Bits(8)),
            "10" => Ok(Self::Bits(10)),
            "12" => Ok(Self::Bits(12)),
            other => Err(CoreError::with_detail(codes::OPTION_DEPTH, other)),
        }
    }
}

/// Container request; `Auto` follows purpose and audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerChoice {
    Auto,
    Mp4,
    Mkv,
}

impl ContainerChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "mp4" => Ok(Self::Mp4),
            "mkv" => Ok(Self::Mkv),
            other => Err(CoreError::with_detail(codes::OPTION_CONTAINER, other)),
        }
    }
}

/// Audio track handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChoice {
    Auto,
    Copy,
    Aac,
    Opus,
    Flac,
}

impl AudioChoice {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Copy => "copy",
            Self::Aac => "aac",
            Self::Opus => "opus",
            Self::Flac => "flac",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "auto" => Ok(Self::Auto),
            "copy" => Ok(Self::Copy),
            "aac" => Ok(Self::Aac),
            "opus" => Ok(Self::Opus),
            "flac" => Ok(Self::Flac),
            other => Err(CoreError::with_detail(codes::OPTION_AUDIO, other)),
        }
    }
}

/// Encoder effort profile. `Fast` is the software archive AV1 standby.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Standard,
    Fast,
}

impl Variant {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Fast => "fast",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "standard" => Ok(Self::Standard),
            "fast" => Ok(Self::Fast),
            other => Err(CoreError::with_detail(codes::OPTION_VARIANT, other)),
        }
    }
}

/// Dynamic HDR intent: keep source metadata or produce standard HDR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicHdrMode {
    Preserve,
    Standard,
}

impl DynamicHdrMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::Standard => "standard",
        }
    }

    pub fn parse(value: &str) -> CoreResult<Self> {
        match value {
            "preserve" => Ok(Self::Preserve),
            "standard" => Ok(Self::Standard),
            other => Err(CoreError::with_detail(codes::OPTION_DYNAMIC_HDR, other)),
        }
    }
}

/// A complete encode request before source-dependent resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeOptions {
    pub purpose: Purpose,
    pub codec: Codec,
    pub output: Output,
    pub transform: Transform,
    pub profile: Option<LogProfile>,
    pub sampling: SamplingChoice,
    pub variant: Variant,
    pub audio: AudioChoice,
    /// `None` means detection decides the input interpretation.
    pub source_kind: Option<SourceKind>,
    pub cpu_decode: bool,
    pub container: ContainerChoice,
    pub color_range: RangeChoice,
    pub bit_depth: DepthChoice,
    pub dynamic_hdr: DynamicHdrMode,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            purpose: Purpose::Share,
            codec: Codec::Hevc,
            output: Output::Sdr,
            transform: Transform::None,
            profile: None,
            sampling: SamplingChoice::Auto,
            variant: Variant::Standard,
            audio: AudioChoice::Auto,
            source_kind: None,
            cpu_decode: false,
            container: ContainerChoice::Auto,
            color_range: RangeChoice::Auto,
            bit_depth: DepthChoice::Auto,
            dynamic_hdr: DynamicHdrMode::Preserve,
        }
    }
}

impl EncodeOptions {
    /// Container implied by the request, without the leading dot.
    pub fn container(&self) -> &'static str {
        match self.container {
            ContainerChoice::Auto => {
                if self.purpose == Purpose::Archive || self.audio == AudioChoice::Flac {
                    "mkv"
                } else {
                    "mp4"
                }
            }
            ContainerChoice::Mp4 => "mp4",
            ContainerChoice::Mkv => "mkv",
        }
    }

    /// File suffix including the leading dot.
    pub fn suffix(&self) -> String {
        format!(".{}", self.container())
    }
}

/// Wire form of an encode request. Every field is optional so a host can send
/// only what the user changed; absent values keep the CLI defaults.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct OptionsDto {
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub transform: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub sampling: Option<String>,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub audio: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub cpu_decode: Option<bool>,
    #[serde(default)]
    pub container: Option<String>,
    #[serde(default)]
    pub color_range: Option<String>,
    #[serde(default)]
    pub bit_depth: Option<String>,
    #[serde(default)]
    pub dynamic_hdr: Option<String>,
}

fn text(value: Option<String>, fallback: &str) -> String {
    value.unwrap_or_else(|| fallback.to_owned())
}

impl OptionsDto {
    /// Converts the wire form into typed options, rejecting unknown values.
    pub fn into_options(self) -> CoreResult<EncodeOptions> {
        let defaults = EncodeOptions::default();
        Ok(EncodeOptions {
            purpose: Purpose::parse(&text(self.purpose, defaults.purpose.as_str()))?,
            codec: Codec::parse(&text(self.codec, defaults.codec.as_str()))?,
            output: Output::parse(&text(self.output, defaults.output.as_str()))?,
            transform: Transform::parse(&text(self.transform, defaults.transform.as_str()))?,
            profile: LogProfile::parse(&text(self.profile, "auto"))?,
            sampling: SamplingChoice::parse(&text(self.sampling, defaults.sampling.label()))?,
            variant: Variant::parse(&text(self.variant, defaults.variant.label()))?,
            audio: AudioChoice::parse(&text(self.audio, defaults.audio.label()))?,
            source_kind: match self.source_kind.as_deref() {
                None | Some("") | Some("auto") => None,
                Some(value) => Some(SourceKind::parse(value)?),
            },
            cpu_decode: self.cpu_decode.unwrap_or(false),
            container: ContainerChoice::parse(&text(self.container, defaults.container.label()))?,
            color_range: RangeChoice::parse(&text(self.color_range, defaults.color_range.label()))?,
            bit_depth: DepthChoice::parse(&text(self.bit_depth, "auto"))?,
            dynamic_hdr: DynamicHdrMode::parse(&text(
                self.dynamic_hdr,
                defaults.dynamic_hdr.label(),
            ))?,
        })
    }
}

/// Concrete format after source-dependent resolution. Never `Auto`/`Source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedFormat {
    pub sampling: Sampling,
    pub color_range: ColorRange,
    pub bit_depth: u8,
}

impl ResolvedFormat {
    /// FFmpeg-style pixel format for this sampling and precision.
    pub fn pixel_format(&self) -> String {
        if self.sampling == Sampling::Gray {
            return if self.bit_depth > 8 {
                format!("gray{}le", self.bit_depth)
            } else {
                "gray".to_owned()
            };
        }
        if self.bit_depth > 8 {
            format!("yuv{}p{}le", self.sampling.label(), self.bit_depth)
        } else {
            format!("yuv{}p", self.sampling.label())
        }
    }

    /// Tag value used by muxers (`tv` / `pc`).
    pub const fn range_tag(&self) -> &'static str {
        match self.color_range {
            ColorRange::Limited => "tv",
            ColorRange::Full => "pc",
        }
    }
}

/// A request plus the concrete format it resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEncode {
    pub options: EncodeOptions,
    pub format: ResolvedFormat,
    /// Effective input interpretation after applying detection and overrides.
    pub source_kind: SourceKind,
}
