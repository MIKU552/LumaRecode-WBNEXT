//! Source-aware output rules.
//!
//! These rules are the migration baseline for `Core/OutputPolicy.cs` and
//! `Transcoder.ResolveFormat`/`Validate`. They are pure: no filesystem, no
//! process, no platform API. Both front ends and both media backends call them
//! so a GUI choice and a CLI choice can never disagree.

use crate::domain::{
    AudioChoice, Codec, ColorRange, ContainerChoice, DepthChoice, DynamicHdrMode, EncodeOptions,
    LogProfile, Output, Purpose, RangeChoice, ResolvedEncode, ResolvedFormat, Sampling,
    SamplingChoice, SourceKind, Transform, Variant,
};
use crate::error::{CoreError, CoreResult, codes};
use crate::source::{AudioStream, PictureFormat, SourceFormat};

/// RGB or palette storage, which has no chroma subsampling to preserve.
pub fn is_rgb(picture: &PictureFormat) -> bool {
    picture.rgb
        || picture.palette
        || ["rgb", "bgr", "gbr", "argb", "abgr", "rgba", "bgra"]
            .iter()
            .any(|prefix| picture.pixel_format.starts_with(prefix))
        || picture.pixel_format == "pal8"
}

/// Chroma layout of the source picture.
///
/// Bayer buffers report `unknown` rather than pretending to be 4:2:0: a RAW
/// plane is developed, not subsampled.
pub fn source_sampling(source: &SourceFormat) -> Sampling {
    if source.raw {
        return Sampling::Yuv444;
    }
    let picture = &source.picture;
    if picture.monochrome {
        return Sampling::Gray;
    }
    if picture.bayer {
        return Sampling::Unknown;
    }
    if is_rgb(picture) {
        return Sampling::Yuv444;
    }
    let name = picture.pixel_format.as_str();
    if matches!(picture.component_count, Some(1 | 2))
        || name.starts_with("gray")
        || name.starts_with("ya")
    {
        return Sampling::Gray;
    }
    if let (Some(width), Some(height)) = (picture.log2_chroma_w, picture.log2_chroma_h) {
        let (Ok(width), Ok(height)) = (u8::try_from(width), u8::try_from(height)) else {
            return Sampling::Unknown;
        };
        // Compare both axes: 4:4:0 must not look like it preserves 4:2:2 detail.
        return [
            Sampling::Yuv444,
            Sampling::Yuv422,
            Sampling::Yuv420,
            Sampling::Yuv440,
            Sampling::Yuv411,
            Sampling::Yuv410,
        ]
        .into_iter()
        .find(|sampling| sampling.shifts() == Some((width, height)))
        .unwrap_or(Sampling::Unknown);
    }
    // ffprobe names place the layout at the leftmost matching position.
    if let Some((_, sampling)) = Sampling::DELIVERABLE
        .iter()
        .chain([Sampling::Yuv440, Sampling::Yuv411, Sampling::Yuv410].iter())
        .filter_map(|sampling| name.find(sampling.label()).map(|at| (at, *sampling)))
        .min_by_key(|(at, _)| *at)
    {
        return sampling;
    }
    match name {
        "nv12" | "nv21" | "p010le" | "p012le" | "p016le" => Sampling::Yuv420,
        "yuyv422" | "uyvy422" => Sampling::Yuv422,
        _ => Sampling::Unknown,
    }
}

/// Usable precision of the source, excluding alpha and unused storage bits.
pub fn source_depth(source: &SourceFormat) -> u8 {
    if source.raw {
        // Bayer descriptors count sensel coverage; the developed plane is 16-bit.
        return 16;
    }
    let picture = &source.picture;
    let mut components = picture.components.clone();
    if picture.alpha && components.len() > 1 {
        components.pop();
    }
    let declared = if picture.bits_per_raw_sample > 0 {
        picture.bits_per_raw_sample
    } else {
        picture.bits_per_sample
    };
    let deepest = components.into_iter().filter(|depth| *depth > 0).max();
    if let Some(deepest) = deepest {
        let usable = if declared > 0 {
            deepest.min(declared)
        } else {
            deepest
        };
        return clamp_depth(usable);
    }
    if declared > 0 {
        return clamp_depth(declared);
    }
    let name = picture.pixel_format.as_str();
    if let Some((_, bits)) = ["10", "12", "14", "16"]
        .iter()
        .filter_map(|bits| {
            let at = name
                .find(&format!("p{bits}"))
                .or_else(|| name.find(&format!("gray{bits}")))?;
            Some((at, *bits))
        })
        .min_by_key(|(at, _)| *at)
    {
        return bits.parse().unwrap_or(8);
    }
    8
}

fn clamp_depth(depth: i32) -> u8 {
    depth.clamp(1, 32) as u8
}

/// Formats that do not raise chroma resolution on either axis.
///
/// Gray may be delivered as 4:2:0 with neutral chroma; RGB is converted to
/// YCbCr at full chroma resolution.
pub fn sampling_choices(source: &SourceFormat) -> Vec<Sampling> {
    let sampling = source_sampling(source);
    if sampling == Sampling::Gray {
        return vec![Sampling::Gray, Sampling::Yuv420];
    }
    let Some(source_shifts) = sampling.shifts() else {
        return Vec::new();
    };
    Sampling::DELIVERABLE
        .into_iter()
        .filter(|candidate| {
            candidate
                .shifts()
                .map(|shifts| shifts.0 >= source_shifts.0 && shifts.1 >= source_shifts.1)
                .unwrap_or(false)
        })
        .collect()
}

/// Precision values allowed by the encoder family and the source precision.
pub fn depth_choices(source_depth: u8, purpose: Purpose, codec: Codec, output: Output) -> Vec<u8> {
    let supported: &[u8] = match (purpose, codec) {
        (Purpose::Share, Codec::Avc) => &[8],
        (Purpose::Archive, Codec::Hevc) => &[8, 10, 12],
        _ => &[8, 10],
    };
    supported
        .iter()
        .copied()
        .filter(|depth| *depth <= source_depth.max(8) && (output != Output::Hdr || *depth >= 10))
        .collect()
}

/// Default precision for a resolved choice list.
pub fn default_depth(choices: &[u8], purpose: Purpose, output: Output) -> Option<u8> {
    if choices.is_empty() {
        return None;
    }
    Some(match purpose {
        Purpose::Archive => *choices.iter().max().unwrap_or(&8),
        Purpose::Share => {
            if output == Output::Hdr {
                10
            } else {
                8
            }
        }
    })
}

/// Default chroma layout before the source is consulted.
pub fn default_sampling(source: &SourceFormat, purpose: Purpose, codec: Codec) -> SamplingChoice {
    if purpose == Purpose::Share || codec == Codec::Av1 {
        return SamplingChoice::Fixed(Sampling::Yuv420);
    }
    if is_rgb(&source.picture) || source.raw {
        return SamplingChoice::Fixed(Sampling::Yuv444);
    }
    let sampling = source_sampling(source);
    if sampling == Sampling::Yuv440 {
        return SamplingChoice::Fixed(Sampling::Yuv420);
    }
    if source.has_alpha() {
        // Alpha is never delivered, so its plane cannot pin the layout.
        return SamplingChoice::Fixed(match sampling {
            Sampling::Yuv420 | Sampling::Yuv422 | Sampling::Yuv444 | Sampling::Gray => sampling,
            _ => Sampling::Yuv420,
        });
    }
    SamplingChoice::Source
}

/// Sample range from explicit tags, pixel format and camera-profile defaults.
///
/// Range is independent of the transfer function: Full-range HLG stays Full.
pub fn source_range(source: &SourceFormat, profile_range: Option<ColorRange>) -> ColorRange {
    let picture = &source.picture;
    match picture.color_range.as_deref() {
        Some("pc" | "full" | "jpeg") => return ColorRange::Full,
        Some("tv" | "limited" | "mpeg") => return ColorRange::Limited,
        _ => {}
    }
    if is_rgb(picture) || picture.pixel_format.starts_with("yuvj") {
        return ColorRange::Full;
    }
    profile_range.unwrap_or(if source.profile == Some(LogProfile::MiLog) {
        ColorRange::Full
    } else {
        ColorRange::Limited
    })
}

/// One audio track approved for FLAC compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlacTrack {
    /// 1-based position in the source, as reported to the user.
    pub number: usize,
    pub depth: u8,
    pub channels: u8,
}

const LOSSLESS_CODECS: [&str; 9] = [
    "flac",
    "alac",
    "ape",
    "tta",
    "tak",
    "shorten",
    "truehd",
    "mlp",
    "wmalossless",
];

/// Channel layouts FLAC can reproduce without remapping.
const FLAC_LAYOUTS: [&str; 9] = [
    "", "mono", "stereo", "3.0", "quad", "5.0", "5.1", "6.1", "7.1",
];

fn unspecified(value: &str) -> bool {
    matches!(value, "" | "unknown" | "unspecified" | "reserved")
}

/// Bit depth encoded in an integer PCM codec name such as `pcm_s24le`.
fn pcm_bits(codec: &str) -> Option<i32> {
    let rest = codec.strip_prefix("pcm_")?;
    let sign = rest.chars().next()?;
    if sign != 's' && sign != 'u' {
        return None;
    }
    let digits: String = rest[1..].chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let tail = &rest[1 + digits.len()..];
    matches!(
        tail,
        "" | "le" | "be" | "le_planar" | "be_planar" | "_planar"
    )
    .then(|| digits.parse().ok())
    .flatten()
}

fn audio_depth(track: &AudioStream) -> i32 {
    if track.bits_per_raw_sample > 0 {
        return track.bits_per_raw_sample;
    }
    if track.bits_per_sample > 0 {
        return track.bits_per_sample;
    }
    pcm_bits(&track.codec_name).unwrap_or(0)
}

fn is_integer_lossless(track: &AudioStream) -> bool {
    let codec = track.codec_name.as_str();
    LOSSLESS_CODECS.contains(&codec)
        || pcm_bits(codec).is_some()
        || (codec == "dts" && track.profile == "DTS-HD MA")
}

/// Every track must be a confirmable integer lossless source.
///
/// Floating-point PCM is rejected: converting it to 24-bit integers would lose
/// the very precision that justifies calling the result archival.
pub fn flac_tracks(source: &SourceFormat) -> CoreResult<Vec<FlacTrack>> {
    if source.audio.is_empty() {
        return Err(CoreError::new(codes::AUDIO_EMPTY));
    }
    source
        .audio
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let number = index + 1;
            if !is_integer_lossless(track) {
                return Err(CoreError::with_detail(
                    codes::AUDIO_NOT_LOSSLESS,
                    format!("{number}:{}", track.codec_name),
                ));
            }
            let depth = audio_depth(track);
            if !(1..=32).contains(&depth) || !(1..=8).contains(&track.channels) {
                return Err(CoreError::with_detail(
                    codes::AUDIO_UNKNOWN_DEPTH,
                    format!("{number}:{depth}/{}", track.channels),
                ));
            }
            let layout = track.channel_layout.as_deref().unwrap_or("");
            let expected = FLAC_LAYOUTS[track.channels as usize];
            if !unspecified(layout) && layout != expected
                || track.channels > 2 && unspecified(layout)
            {
                return Err(CoreError::with_detail(
                    codes::AUDIO_LAYOUT,
                    format!("{number}:{layout}"),
                ));
            }
            Ok(FlacTrack {
                number,
                depth: depth as u8,
                channels: track.channels as u8,
            })
        })
        .collect()
}

/// Whether FLAC output can be offered at all, and why not when it cannot.
pub fn flac_eligibility(source: &SourceFormat) -> Result<Vec<FlacTrack>, &'static str> {
    flac_tracks(source).map_err(|error| error.code())
}

/// Resolves source-dependent values and enforces the encoder limits.
pub fn resolve(options: &EncodeOptions, source: &SourceFormat) -> CoreResult<ResolvedEncode> {
    if options.output == Output::Hdr && options.codec == Codec::Avc {
        return Err(CoreError::with_detail(codes::OPTION_CODEC, "avc:hdr"));
    }
    let profile_range = match options.transform {
        Transform::LogHdr | Transform::LogSdr => options.profile.map(LogProfile::range),
        _ => None,
    };
    let sampling = match options.sampling {
        SamplingChoice::Auto => match default_sampling(source, options.purpose, options.codec) {
            SamplingChoice::Fixed(sampling) => sampling,
            // `Auto` may still land on "keep the source layout", which follows
            // the same alpha rule as an explicit `Source` request.
            _ => {
                if source.has_alpha() {
                    return Err(CoreError::new(codes::POLICY_ALPHA_SAMPLING));
                }
                source_sampling(source)
            }
        },
        SamplingChoice::Source => {
            if source.has_alpha() {
                return Err(CoreError::new(codes::POLICY_ALPHA_SAMPLING));
            }
            source_sampling(source)
        }
        SamplingChoice::Fixed(sampling) => sampling,
    };
    if matches!(
        sampling,
        Sampling::Unknown | Sampling::Yuv440 | Sampling::Yuv411 | Sampling::Yuv410
    ) {
        return Err(CoreError::with_detail(
            codes::POLICY_SAMPLING_CHOICE,
            sampling.label(),
        ));
    }
    if !sampling_choices(source).contains(&sampling) {
        return Err(CoreError::with_detail(
            codes::POLICY_SAMPLING_LIMIT,
            sampling.label(),
        ));
    }
    if sampling != Sampling::Yuv420
        && !(options.purpose == Purpose::Archive && options.codec != Codec::Av1)
    {
        return Err(CoreError::with_detail(
            codes::POLICY_SAMPLING_SOFTWARE,
            sampling.label(),
        ));
    }
    let color_range = match options.color_range {
        RangeChoice::Auto if options.purpose == Purpose::Archive => {
            source_range(source, profile_range)
        }
        RangeChoice::Auto => ColorRange::Limited,
        RangeChoice::Source => source_range(source, profile_range),
        RangeChoice::Fixed(range) => range,
    };
    let choices = depth_choices(
        source_depth(source),
        options.purpose,
        options.codec,
        options.output,
    );
    if choices.is_empty() {
        return Err(CoreError::new(codes::POLICY_HDR_DEPTH));
    }
    let bit_depth = match options.bit_depth {
        DepthChoice::Auto => default_depth(&choices, options.purpose, options.output)
            .ok_or_else(|| CoreError::new(codes::POLICY_HDR_DEPTH))?,
        DepthChoice::Bits(bits) => bits,
    };
    if !choices.contains(&bit_depth) {
        return Err(CoreError::with_detail(
            codes::POLICY_DEPTH_LIMIT,
            bit_depth.to_string(),
        ));
    }
    Ok(ResolvedEncode {
        options: options.clone(),
        format: ResolvedFormat {
            sampling,
            color_range,
            bit_depth,
        },
        source_kind: source.effective_kind(options.source_kind),
    })
}

/// Whether the source's dynamic HDR metadata can be carried unchanged.
pub fn preserve_dynamic_hdr(options: &EncodeOptions, source: &SourceFormat) -> bool {
    source.dynamic_hdr.is_present()
        && options.dynamic_hdr == DynamicHdrMode::Preserve
        && options.output == Output::Hdr
        && options.transform == Transform::None
}

fn validate_dynamic_hdr(
    options: &EncodeOptions,
    source: &SourceFormat,
    format: &ResolvedFormat,
) -> CoreResult<()> {
    if !source.dynamic_hdr.is_present() {
        return Ok(());
    }
    if let Some(requested) = options.source_kind
        && requested != source.kind
    {
        return Err(CoreError::new(codes::DVHDR_INTERPRETATION));
    }
    if let Some(dolby) = source.dynamic_hdr.dolby_vision {
        if !matches!(dolby.profile, 5 | 8 | 10) || dolby.el_present_flag != 0 {
            return Err(CoreError::new(codes::DVHDR_LAYER));
        }
        if !dolby.is_supported_layer() {
            return Err(CoreError::new(codes::DVHDR_COMPATIBILITY));
        }
    }
    if !preserve_dynamic_hdr(options, source) {
        return Ok(());
    }
    if options.purpose != Purpose::Archive || options.codec == Codec::Avc {
        return Err(CoreError::new(codes::DVHDR_NEEDS_ARCHIVE));
    }
    if source.dynamic_hdr.hdr10_plus && options.codec != Codec::Hevc {
        return Err(CoreError::new(codes::DVHDR_HDR10PLUS_CODEC));
    }
    let profile5 = source
        .dynamic_hdr
        .dolby_vision
        .map(|dolby| dolby.is_profile5())
        .unwrap_or(false);
    if profile5 && options.codec != Codec::Hevc {
        return Err(CoreError::new(codes::DVHDR_PROFILE5_CODEC));
    }
    if format.sampling != Sampling::Yuv420
        || format.bit_depth != 10
        || format.color_range != source_range(source, None)
    {
        return Err(CoreError::new(codes::DVHDR_FORMAT));
    }
    if source.display_matrix {
        return Err(CoreError::new(codes::DVHDR_DISPLAY_MATRIX));
    }
    Ok(())
}

/// Full request validation: format limits, audio, RAW, dynamic HDR and color.
pub fn validate(options: &EncodeOptions, source: &SourceFormat) -> CoreResult<ResolvedEncode> {
    let resolved = resolve(options, source)?;
    if options.audio == AudioChoice::Flac {
        if options.container == ContainerChoice::Mp4 {
            return Err(CoreError::with_detail(codes::OPTION_CONTAINER, "flac:mp4"));
        }
        flac_tracks(source)?;
    }
    if options.variant == Variant::Fast
        && !(options.purpose == Purpose::Archive && options.codec == Codec::Av1)
    {
        return Err(CoreError::new(codes::PRESET_VARIANT));
    }
    let kind = resolved.source_kind;
    if (kind == SourceKind::Raw) != source.raw {
        return Err(CoreError::with_detail(
            codes::RAW_INTERPRETATION,
            kind.as_str(),
        ));
    }
    if source.raw {
        // RAW development needs both a decoder for the camera and a version
        // that exposes its parameters; the two failures read differently.
        if !source.raw_decoder_version_supported {
            return Err(CoreError::new(codes::RAW_DECODER_VERSION));
        }
        if !source.raw_decoder_supported {
            return Err(CoreError::new(codes::RAW_DECODER_MISSING));
        }
    }
    validate_dynamic_hdr(options, source, &resolved.format)?;
    match options.transform {
        Transform::LogHdr | Transform::LogSdr => {
            if options.profile.is_none() {
                return Err(CoreError::new(codes::TRANSFORM_PROFILE_REQUIRED));
            }
            let expected = if options.output == Output::Hdr {
                Transform::LogHdr
            } else {
                Transform::LogSdr
            };
            if options.transform != expected {
                return Err(CoreError::new(codes::TRANSFORM_TARGET_MISMATCH));
            }
            if kind != SourceKind::Log {
                return Err(CoreError::new(codes::TRANSFORM_REQUIRES_LOG));
            }
            if source_depth(source) < 10 {
                return Err(CoreError::new(codes::TRANSFORM_REQUIRES_DEPTH));
            }
        }
        Transform::HdrSdr => {
            if !(options.output == Output::Sdr && kind.is_hdr()) {
                return Err(CoreError::new(codes::TRANSFORM_HDR_TO_SDR));
            }
        }
        Transform::None => {
            if matches!(kind, SourceKind::Log | SourceKind::Unknown) {
                return Err(CoreError::with_detail(
                    codes::TRANSFORM_UNDETERMINED,
                    kind.as_str(),
                ));
            }
            if options.output == Output::Hdr && kind == SourceKind::Sdr {
                return Err(CoreError::new(codes::TRANSFORM_SDR_TO_HDR));
            }
            if options.output == Output::Sdr && kind.is_hdr() {
                return Err(CoreError::new(codes::TRANSFORM_HDR_TO_SDR_REQUIRED));
            }
        }
    }
    Ok(resolved)
}
