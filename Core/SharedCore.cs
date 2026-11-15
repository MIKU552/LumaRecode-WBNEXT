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

    /// <summary>Removes both bridges, which restores the managed rules.</summary>
    public static void Reset()
    {
        Resolver = null;
        Validator = null;
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
