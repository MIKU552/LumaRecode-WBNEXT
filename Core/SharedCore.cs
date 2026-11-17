using System.Text.Json.Nodes;

namespace VideoConverter.Core;

/// <summary>Result of one resolve call: a resolved request or a stable error identity.</summary>
/// <param name="Options">Resolved options, or null when the core rejected the request.</param>
/// <param name="Code">Stable core error code such as <c>policy.hdr_depth</c>.</param>
/// <param name="Detail">Rejected value, if the code carries one.</param>
public readonly record struct SharedResolution(EncodeOptions? Options, string? Code, string? Detail);

/// <summary>Result of one validate call: the effective input kind or a stable error identity.</summary>
/// <param name="Kind">Effective input kind, or null when the core rejected the request.</param>
/// <param name="Code">Stable core error code.</param>
/// <param name="Detail">Rejected value, if the code carries one.</param>
public readonly record struct SharedValidation(string? Kind, string? Code, string? Detail);

/// <summary>One input and the flags that decide what survives into the output.</summary>
/// <param name="Input">Absolute path of the source file.</param>
/// <param name="VideoIndex">Stream index of the selected video track.</param>
/// <param name="AudioMap">`0:a?` for all audio, `0:a:0?` for a motion photo.</param>
/// <param name="PreserveMetadata">Whether container metadata is copied.</param>
/// <param name="PreserveChapters">Whether chapters are copied.</param>
public sealed record InputMapRequest(string Input, int VideoIndex, string AudioMap, bool PreserveMetadata, bool PreserveChapters);

/// <summary>Color tags passed to the encoder, with empty values meaning "unspecified".</summary>
public sealed record ColorTagRequest(string? Primaries, string? Transfer, string? Space, string? Range, bool SuppressDv5Tags);

/// <summary>Encoder choice and rate-control values for one conversion.</summary>
public sealed record EncoderRequest(string Encoder, string Preset, string Quality, bool Share, string Codec,
    int Depth, long Gop, bool AutoLowPower, bool TimingCompatibility, string AsyncDepth, string PixelFormat);

/// <summary>The inputs of one GOP decision: rate, purpose and clip length.</summary>
/// <param name="FrameRate">Nominal frame rate, already resolved from the probe's rational form.</param>
/// <param name="Share">Quick share keeps the interval short; archive uses the long cadence.</param>
/// <param name="MotionPhoto">Motion photos are seconds long and use the short cadence too.</param>
public sealed record GopRequest(double FrameRate, bool Share, bool MotionPhoto);

/// <summary>One encoder parameter, in insertion order.</summary>
public sealed record ParameterPair(string Key, string Value);

/// <summary>A camera curve as the core describes it: the LUT file, its matrix and its range.</summary>
public sealed record LogProfileInfo(string Profile, string File, string Matrix, string Range);

/// <summary>Facts the color tag derivation needs: what the source declares and where it goes.</summary>
/// <param name="Kind">Effective interpretation: sdr, hlg, pq, log or raw.</param>
/// <param name="Output">Target transfer function.</param>
/// <param name="Rgb">The source is RGB, so its matrix names a surface rather than a delivery one.</param>
/// <param name="DolbyProfile5">A single-layer profile 5 stream carries its own surface name.</param>
/// <param name="SourceRange">Resolved source range: limited or full.</param>
/// <param name="OutputRange">Resolved output range: limited or full.</param>
/// <param name="Primaries">Primaries the source declares, or null.</param>
/// <param name="Transfer">Transfer the source declares, or null.</param>
/// <param name="Space">Matrix the source declares, or null.</param>
public sealed record ColorTagsRequest(string Kind, string Output, bool Rgb, bool DolbyProfile5,
    string SourceRange, string OutputRange, string? Primaries, string? Transfer, string? Space);

/// <summary>One set of color tags; null means the source never declared it.</summary>
public sealed record ColorTagsValue(string? Primaries, string? Transfer, string? Space, string? Range);

/// <summary>The tags to read with and the tags to write.</summary>
public sealed record ColorTagSet(ColorTagsValue Source, ColorTagsValue Target);

/// <summary>The software colour conversion: tags to write, range to convert, format to emit.</summary>
/// <param name="Primaries">Color primaries tag, or null when the source never declared it.</param>
/// <param name="Transfer">Transfer characteristic tag, or null.</param>
/// <param name="Space">Matrix tag, or null.</param>
/// <param name="Range">Range tag in encoder spelling: <c>pc</c> or <c>tv</c>.</param>
/// <param name="ConversionRange">Range the conversion writes: <c>limited</c> or <c>full</c>.</param>
/// <param name="PixelFormat">Resolved output pixel format.</param>
/// <param name="Purpose">Share and archive differ on the 10-bit format.</param>
/// <param name="Depth">Resolved output depth.</param>
/// <param name="PreserveDynamic">Dynamic metadata survives, so no conversion stage runs.</param>
public sealed record ColorFilterRequest(string? Primaries, string? Transfer, string? Space, string? Range,
    string ConversionRange, string PixelFormat, string Purpose, int Depth, bool PreserveDynamic);

/// <summary>The camera-curve chain: the LUT file plus the matrix and range it was graded in.</summary>
/// <param name="File">LUT file name, resolved by the host against its asset directory.</param>
/// <param name="Matrix">Matrix the curve was graded in, used when the source declares none.</param>
/// <param name="Range">Range the curve expects, used when the source declares none.</param>
/// <param name="SourceSpace">Matrix the source declares, or null.</param>
/// <param name="SourceRange">Range the source declares, or null.</param>
public sealed record LogFilterRequest(string File, string Matrix, string Range, string? SourceSpace, string? SourceRange);

/// <summary>Tone mapping to BT.709, with the metadata drop that follows it.</summary>
public sealed record ToneMapRequest(string Range, bool ApplyDolbyVision);

/// <summary>Dolby Vision rendered as plain HDR, without the RPU.</summary>
public sealed record DolbyRenderRequest(string Kind, string Range);

/// <summary>Hardware surface conversion for the QSV path.</summary>
public sealed record VppFilterRequest(int Depth, string OutRange, bool Sdr);

/// <summary>Signal-dependent filters one conversion may need.</summary>
public sealed record RenderFiltersRequest(LogFilterRequest? Log, ToneMapRequest? ToneMap,
    DolbyRenderRequest? DolbyRender, bool DropDynamicMetadata, VppFilterRequest? Vpp);

/// <summary>The entries that apply, each already spelled for FFmpeg.</summary>
public sealed record RenderFilters(string? Log, string? ToneMap, string? DolbyRender, string? DroppedMetadata, string? Vpp);

/// <summary>HDR metadata one encode should carry into the encoder parameters.</summary>
/// <param name="Codec">Encoder family; the two families spell the tokens differently.</param>
/// <param name="Output">Target transfer function; an SDR target keeps only the dynamic payload.</param>
/// <param name="SourceKind">Effective interpretation, so an override to SDR drops the tokens.</param>
/// <param name="PreserveDynamic">Dynamic metadata survives the encode.</param>
/// <param name="DolbyVisionProfile">Dolby Vision profile the probe found, when any.</param>
/// <param name="Hdr10Plus">HDR10+ dynamic metadata was found.</param>
/// <param name="SideData">Video and first-frame side data, as the probe reported it.</param>
public sealed record HdrMetadataRequest(string Codec, string Output, string SourceKind, bool PreserveDynamic,
    int? DolbyVisionProfile, bool Hdr10Plus, IReadOnlyList<JsonObject> SideData);

/// <summary>Parameter entries and bare tokens that carry the metadata.</summary>
/// <param name="Parameters">Entries to append after the preset table, in order.</param>
/// <param name="Appended">Bare tokens for the same parameter string.</param>
public sealed record HdrMetadataArguments(IReadOnlyList<ParameterPair> Parameters, IReadOnlyList<string> Appended);

/// <summary>Optional bridge from the managed rules to the shared native core.</summary>
/// <remarks>
/// The host installs the bridge after loading the native library. When a delegate is null the
/// managed rules stay authoritative, so a machine without the library behaves as before.
/// Rejections are translated back into the existing localized messages; an unmapped code returns
/// null so the managed checks can produce their exact wording instead of a generic sentence.
/// </remarks>
public static class SharedCore
{
    /// <summary>Resolves a request through the shared core; null means "use the managed rules".</summary>
    public static Func<EncodeOptions, MediaInfo, SharedResolution>? Resolver { get; set; }

    /// <summary>Validates a request through the shared core; null means "use the managed rules".</summary>
    public static Func<EncodeOptions, MediaInfo, SharedValidation>? Validator { get; set; }

    /// <summary>Builds the input/mapping arguments through the shared core; null means "use the managed builder".</summary>
    public static Func<InputMapRequest, IReadOnlyList<string>?>? InputBuilder { get; set; }

    /// <summary>Builds the color-tag arguments through the shared core; null means "use the managed builder".</summary>
    public static Func<ColorTagRequest, IReadOnlyList<string>?>? ColorBuilder { get; set; }

    /// <summary>Builds the encoder arguments through the shared core; null means "use the managed builder".</summary>
    public static Func<EncoderRequest, IReadOnlyList<string>?>? EncoderBuilder { get; set; }

    /// <summary>Formats encoder parameters through the shared core; null means "use the managed join".</summary>
    public static Func<IReadOnlyList<ParameterPair>, IReadOnlyList<string>, string?>? ParameterBuilder { get; set; }

    /// <summary>Joins the filter chain through the shared core; null means "use the managed join".</summary>
    public static Func<IReadOnlyList<string>, string?>? FilterBuilder { get; set; }

    /// <summary>Describes a camera curve through the shared core; null means "use the host table".</summary>
    public static Func<string, LogProfileInfo?>? LogProfile { get; set; }

    /// <summary>Derives the source and target color tags through the shared core; null means "use the managed rule".</summary>
    public static Func<ColorTagsRequest, ColorTagSet?>? ColorTagsBuilder { get; set; }

    /// <summary>Builds the signal-dependent filters through the shared core; null means "use the managed builder".</summary>
    public static Func<RenderFiltersRequest, RenderFilters?>? RenderFilterBuilder { get; set; }

    /// <summary>Builds the software colour conversion filters through the shared core; null means "use the managed builder".</summary>
    public static Func<ColorFilterRequest, IReadOnlyList<string>?>? ColorFilterBuilder { get; set; }

    /// <summary>Computes the GOP length through the shared core; null means "use the managed formula".</summary>
    public static Func<GopRequest, long?>? Gop { get; set; }

    /// <summary>Composes the HDR metadata parameters through the shared core; null means "use the managed rule".</summary>
    public static Func<HdrMetadataRequest, HdrMetadataArguments?>? HdrMetadata { get; set; }

    /// <summary>Version reported by the loaded core, or an empty string when absent.</summary>
    public static string CoreVersion { get; set; } = "";

    /// <summary>True when the bridges are installed and the core answers requests.</summary>
    public static bool CoreActive => Resolver is not null;

    /// <summary>Removes both bridges, which restores the managed rules.</summary>
    public static void Reset()
    {
        Resolver = null;
        Validator = null;
        InputBuilder = null;
        ColorBuilder = null;
        EncoderBuilder = null;
        ParameterBuilder = null;
        FilterBuilder = null;
        ColorFilterBuilder = null;
        RenderFilterBuilder = null;
        ColorTagsBuilder = null;
        LogProfile = null;
        Gop = null;
        HdrMetadata = null;
        CoreVersion = "";
    }

    /// <summary>Maps a core rejection onto the message the current UI already shows, or null when unmapped.</summary>
    public static string? Message(string? code, string? detail) => (code, detail) switch
    {
        ("options.container", "flac:mp4") => "FLAC 音轨请选择 MKV 封装，以保留声道布局。",
        ("options.container", _) => "请选择自动、MP4 或 MKV 封装。",
        ("options.codec", "avc:hdr") => "HDR 输出请选择 HEVC 或 AV1。",
        ("options.purpose", _) or ("options.codec", _) => "请选择有效的用途和编码器。",
        ("options.output", _) => "请选择 SDR 或 HDR 输出。",
        ("options.audio", _) => "请选择有效的音轨格式。",
        ("options.source_kind", _) => "请选择有效的输入类型。",
        ("options.sampling", _) => "请选择自动、保持原采样、4:2:0、4:2:2 或 4:4:4。",
        ("options.color_range", _) => "请选择自动、保持原范围、Limited 或 Full。",
        ("options.bit_depth", _) => "请选择自动、8、10 或 12 位输出。",
        ("options.transform", _) or ("transform.group", _) => "请选择有效的 LUT / 色调映射分组。",
        ("policy.alpha_sampling", _) => "素材含透明通道。请选择具体采样格式，输出为不带透明通道的视频。",
        ("policy.sampling_choice", _) => "当前编码器无法保持原采样，请手动选择 4:2:0、4:2:2 或 4:4:4。",
        ("policy.sampling_limit", _) => "输出采样须在原片采样上限内选择。",
        ("policy.sampling_software", _) => "保持此采样需要软件 HEVC 或 H.264；也可手动选择 4:2:0。",
        ("policy.hdr_depth", _) => "HDR 输出需要至少 10 位的源素材，请选择 SDR 或高位深原片。",
        ("policy.depth_limit", _) => "输出位深须在原片和编码器的位深上限内选择；HDR 至少需要 10 位。",
        ("preset.variant", _) => "快速备用质量档仅用于软件归档 AV1。",
        ("raw.interpretation", _) => "ProRes RAW 使用自动识别与相机参数显影。",
        ("raw.decoder_version", _) => "ProRes RAW 显影需要 FFmpeg 9 或更新版本。",
        ("raw.decoder_missing", _) => "当前 FFmpeg 无法解码此 ProRes RAW。请先用支持该相机的 RAW 软件显影为 ProRes 或 HEVC。",
        ("transform.profile_required", _) => "请选择具体的 Log 曲线。",
        ("transform.target_mismatch", _) => "Log 转换目标需要与 SDR/HDR 输出一致。",
        ("transform.requires_log", _) => "Log 转换需要将输入解释指定为 Log。",
        ("transform.requires_depth", _) => "这些 Log 转换需要对应相机的高位深原片。",
        ("transform.hdr_to_sdr", _) => "HDR → SDR 需要 HLG/PQ 输入及 SDR 输出。",
        ("transform.undetermined", _) => "请确认输入类型。Log 原片需要选择曲线后转换。",
        ("transform.sdr_to_hdr", _) => "SDR 输入请选择 SDR 输出。",
        ("transform.hdr_to_sdr_required", _) => "HDR 输入输出 SDR 时，请选择 HDR → SDR 转换。",
        _ => null
    };
}
