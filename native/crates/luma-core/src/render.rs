//! Argument blocks that every host renders the same way.
//!
//! The full FFmpeg command still belongs to the host, which knows device quirks and filter
//! choices. The blocks here are the ones that must not drift between hosts: which streams are
//! mapped, whether metadata and chapters survive, and the flags that keep the process quiet.

use crate::domain::{Output, Purpose, SourceKind};

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

/// Quotes a filter argument when it contains characters the filtergraph reads as syntax.
///
/// A LUT in a folder with a comma otherwise splits the chain, and FFmpeg answers
/// "No option name near ..." instead of opening the file. Names without such characters are
/// left alone, so the hosts keep producing the same command they always have.
fn quoted(value: &str) -> String {
    let syntax = [',', ':', ';', '[', ']', '\'', '\\', ' '];
    if value.contains(&syntax[..]) {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        value.to_owned()
    }
}

/// The `setparams` entry that declares a tag set to the filters that follow.
///
/// Hosts that run a conversion insert the source entry before the tone map and the output
/// entry after it, so both stages read the same description the encoder writes.
pub fn tag_filter(tags: &ColorTags) -> String {
    frame_tags(tags)
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

/// The camera-curve chain: decode the log transport, apply the LUT, retag for HLG.
///
/// The LUT file is named by the host, which ships it; everything else — the matrix and
/// range the curve was graded in, and the order of the stages — is decided here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogFilter<'a> {
    /// LUT file name, resolved by the host against its asset directory.
    pub file: &'a str,
    /// Matrix the curve was graded in, used when the source declares none.
    pub matrix: &'a str,
    /// Range the curve expects, used when the source declares none.
    pub range: &'a str,
    pub source_space: Option<&'a str>,
    pub source_range: Option<&'a str>,
}

impl LogFilter<'_> {
    fn build(&self) -> String {
        // The source's own tags win: a file that already says what it is needs no guess.
        let matrix = match self.source_space {
            Some("gbr") => "gbr",
            Some("bt709") => "709",
            Some("bt2020nc") => "2020_ncl",
            Some("smpte170m") => "170m",
            Some("bt470bg") => "470bg",
            _ => self.matrix,
        };
        let range = match self.source_range {
            Some("pc") => "full",
            Some("tv") => "limited",
            _ => self.range,
        };
        format!(
            "zscale=matrixin={matrix}:rangein={range}:range=full,format=gbrpf32le,lut3d=file={}:interp=tetrahedral,setparams=range=full:color_primaries=bt2020:color_trc=arib-std-b67:colorspace=gbr,zscale=matrix=2020_ncl:rangein=full:range=limited:dither=error_diffusion,format=yuv444p16le",
            quoted(self.file)
        )
    }
}

/// Tone mapping to BT.709, dropping the metadata the encoder would otherwise copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToneMapFilter<'a> {
    /// Range word libplacebo expects: `pc` or `tv`.
    pub range: &'a str,
    pub apply_dolby_vision: bool,
}

impl ToneMapFilter<'_> {
    fn build(&self) -> String {
        let dolby = i32::from(self.apply_dolby_vision);
        format!(
            "libplacebo=format=yuv444p16le:colorspace=bt709:color_primaries=bt709:color_trc=bt709:range={}:tonemapping=bt.2390:apply_dolbyvision={dolby},sidedata=delete:type=MASTERING_DISPLAY_METADATA,sidedata=delete:type=CONTENT_LIGHT_LEVEL,sidedata=delete:type=DYNAMIC_HDR_PLUS",
            self.range
        )
    }
}

/// Dolby Vision rendered as plain HDR, without the RPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DolbyRenderFilter<'a> {
    /// `hlg` keeps HLG output; every other HDR source is treated as PQ.
    pub kind: SourceKind,
    pub range: &'a str,
}

impl DolbyRenderFilter<'_> {
    fn build(&self) -> String {
        let trc = if self.kind == SourceKind::Hlg {
            "arib-std-b67"
        } else {
            "smpte2084"
        };
        format!(
            "libplacebo=format=yuv444p16le:colorspace=bt2020nc:color_primaries=bt2020:color_trc={trc}:range={}:apply_dolbyvision=1",
            self.range
        )
    }
}

/// Hardware surface conversion for the QSV path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VppFilter<'a> {
    pub depth: u8,
    /// Range tag as the source declares it: `pc` or `tv`.
    pub out_range: &'a str,
    /// SDR output converts to BT.709 on the same surface.
    pub sdr: bool,
}

impl VppFilter<'_> {
    fn build(&self) -> String {
        let format = match self.depth {
            10 => "p010le",
            _ => "nv12",
        };
        let mut options = format!(":out_range={}", self.out_range);
        if self.sdr {
            options.push_str(
                ":out_color_matrix=bt709:out_color_primaries=bt709:out_color_transfer=bt709",
            );
        }
        format!("vpp_qsv=format={format}{options}")
    }
}

/// Everything a conversion may need to drop when it changes the signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterContentRequest<'a> {
    pub log: Option<LogFilter<'a>>,
    pub tone_map: Option<ToneMapFilter<'a>>,
    pub dolby_render: Option<DolbyRenderFilter<'a>>,
    /// The source carries dynamic metadata that a conversion invalidates.
    pub drop_dynamic_metadata: bool,
    pub vpp: Option<VppFilter<'a>>,
}

/// The entries that apply, each already spelled for FFmpeg.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterContent {
    pub log: Option<String>,
    pub tone_map: Option<String>,
    pub dolby_render: Option<String>,
    pub dropped_metadata: Option<&'static str>,
    pub vpp: Option<String>,
}

/// Builds the filter entries that depend on the signal rather than on the frame size.
pub fn filter_content(request: &FilterContentRequest<'_>) -> FilterContent {
    FilterContent {
        log: request.log.map(|filter| filter.build()),
        tone_map: request.tone_map.map(|filter| filter.build()),
        dolby_render: request.dolby_render.map(|filter| filter.build()),
        dropped_metadata: request
            .drop_dynamic_metadata
            .then_some(DYNAMIC_METADATA_DROP),
        vpp: request.vpp.map(|filter| filter.build()),
    }
}

/// The metadata a conversion invalidates; keeping it would mislabel the output.
pub const DYNAMIC_METADATA_DROP: &str = "sidedata=delete:type=DOVI_RPU_BUFFER,sidedata=delete:type=DOVI_METADATA,sidedata=delete:type=DYNAMIC_HDR_PLUS";

/// Tags the source should be read with: the defaults fill what the container omits.
///
/// Probes often report a matrix but no primaries or transfer, especially for files written by
/// a tool that only took the matrix seriously. Filling the gaps keeps the filter chain and the
/// encoder flags describing the same signal; a tag the source did declare is never replaced.
pub fn source_tags(
    kind: SourceKind,
    rgb: bool,
    dolby_profile5: bool,
    source: &ColorTags,
    range: &str,
) -> ColorTags {
    let mut tags = source.clone();
    tags.range = Some(range_word(range).to_owned());
    match kind {
        SourceKind::Hlg | SourceKind::Pq => {
            // The transfer is not a fallback here: the interpretation decided it.
            tags.transfer = Some(hdr_transfer(kind).to_owned());
            fill_missing(&mut tags, &hlg_tags());
        }
        SourceKind::Sdr => fill_missing(&mut tags, &rec709_tags()),
        _ => {}
    }
    if dolby_profile5 {
        // Profile 5 stores IPTPQc2 code values; naming it YCbCr would mislabel the stream.
        tags.space = Some("ipt-c2".to_owned());
    }
    if rgb {
        tags.space = Some("gbr".to_owned());
    }
    tags
}

/// Tags the output should carry: SDR lands in BT.709, HDR keeps the source's description
/// unless the source is a camera curve or RAW, which develop to HLG.
pub fn target_tags(kind: SourceKind, output: Output, source: &ColorTags, range: &str) -> ColorTags {
    let mut tags = match output {
        Output::Sdr => rec709_tags(),
        Output::Hdr => match kind {
            SourceKind::Log | SourceKind::Raw => hlg_tags(),
            _ => source.clone(),
        },
    };
    if tags.space.as_deref() == Some("gbr") {
        // A RGB surface is not a delivery matrix; the output names the YCbCr one it becomes.
        tags.space = Some(ycbcr_space(output).to_owned());
    }
    tags.range = Some(range_word(range).to_owned());
    tags
}

fn rec709_tags() -> ColorTags {
    ColorTags {
        primaries: Some("bt709".to_owned()),
        transfer: Some("bt709".to_owned()),
        space: Some("bt709".to_owned()),
        range: None,
    }
}

fn hlg_tags() -> ColorTags {
    ColorTags {
        primaries: Some("bt2020".to_owned()),
        transfer: Some("arib-std-b67".to_owned()),
        space: Some("bt2020nc".to_owned()),
        range: None,
    }
}

fn hdr_transfer(kind: SourceKind) -> &'static str {
    if kind == SourceKind::Hlg {
        "arib-std-b67"
    } else {
        "smpte2084"
    }
}

fn ycbcr_space(output: Output) -> &'static str {
    match output {
        Output::Hdr => "bt2020nc",
        Output::Sdr => "bt709",
    }
}

fn range_word(range: &str) -> &'static str {
    if range == "full" { "pc" } else { "tv" }
}

/// Fills the tags the source never declared, leaving what it did declare alone.
fn fill_missing(tags: &mut ColorTags, defaults: &ColorTags) {
    if specified(&tags.primaries).is_none() {
        tags.primaries.clone_from(&defaults.primaries);
    }
    if specified(&tags.transfer).is_none() {
        tags.transfer.clone_from(&defaults.transfer);
    }
    if specified(&tags.space).is_none() {
        tags.space.clone_from(&defaults.space);
    }
}
