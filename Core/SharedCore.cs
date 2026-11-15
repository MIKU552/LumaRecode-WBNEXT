namespace VideoConverter.Core;

/// <summary>Result of one resolve call: a resolved request or a stable error identity.</summary>
/// <param name="Options">Resolved options, or null when the core rejected the request.</param>
/// <param name="Code">Stable core error code such as <c>policy.hdr_depth</c>.</param>
/// <param name="Detail">Rejected value, if the code carries one.</param>
public readonly record struct SharedResolution(EncodeOptions? Options, string? Code, string? Detail);

/// <summary>Optional bridge from the managed rules to the shared native core.</summary>
/// <remarks>
/// The bridge is installed by the host after it has loaded the native library. When it is
/// null the managed rules stay authoritative, so a machine without the library behaves
/// exactly as before. Rejections are translated back into the existing localized messages
/// rather than surfacing core codes in the UI.
/// </remarks>
public static class SharedCore
{
    /// <summary>Resolves a request through the shared core; null means "use the managed rules".</summary>
    public static Func<EncodeOptions, MediaInfo, SharedResolution>? Resolver { get; set; }

    /// <summary>Removes the bridge, which restores the managed rules.</summary>
    public static void Reset() => Resolver = null;

    /// <summary>Maps a core rejection onto the message the current UI already shows.</summary>
    public static string Message(string? code, string? detail) => (code, detail) switch
    {
        ("options.codec", "avc:hdr") => "HDR 输出请选择 HEVC 或 AV1。",
        ("options.purpose", _) or ("options.codec", _) => "请选择有效的用途和编码器。",
        ("options.sampling", _) => "请选择自动、保持原采样、4:2:0、4:2:2 或 4:4:4。",
        ("policy.alpha_sampling", _) => "素材含透明通道。请选择具体采样格式，输出为不带透明通道的视频。",
        ("policy.sampling_choice", _) => "当前编码器无法保持原采样，请手动选择 4:2:0、4:2:2 或 4:4:4。",
        ("policy.sampling_limit", _) => "输出采样须在原片采样上限内选择。",
        ("policy.sampling_software", _) => "保持此采样需要软件 HEVC 或 H.264；也可手动选择 4:2:0。",
        ("options.color_range", _) => "请选择自动、保持原范围、Limited 或 Full。",
        ("options.bit_depth", _) => "请选择自动、8、10 或 12 位输出。",
        ("policy.hdr_depth", _) => "HDR 输出需要至少 10 位的源素材，请选择 SDR 或高位深原片。",
        ("policy.depth_limit", _) => "输出位深须在原片和编码器的位深上限内选择；HDR 至少需要 10 位。",
        _ => $"共享核心拒绝了该输出组合（{code ?? "unknown"}）。"
    };
}
