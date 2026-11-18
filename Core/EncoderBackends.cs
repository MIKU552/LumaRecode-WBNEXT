namespace VideoConverter.Core;

/// <summary>Maps an FFmpeg encoder name to the backend family that provides it.</summary>
/// <remarks>
/// The shared core owns the same table. The parity harness compares both, so a new
/// encoder cannot be classified on one side only; unknown names return null instead of
/// being reported as software.
/// </remarks>
public static class EncoderBackends
{
    /// <summary>Backend family for an encoder name, or null when it is not classified.</summary>
    public static string? For(string encoder) => encoder switch
    {
        "h264_qsv" or "hevc_qsv" or "av1_qsv" => "qsv",
        "h264_amf" or "hevc_amf" or "av1_amf" => "amf",
        "h264_nvenc" or "hevc_nvenc" or "av1_nvenc" => "nvenc",
        "h264_mf" or "hevc_mf" => "media_foundation",
        "h264_videotoolbox" or "hevc_videotoolbox" => "videotoolbox",
        _ when encoder.StartsWith("lib", StringComparison.Ordinal) => "software",
        _ => null
    };

    /// <summary>Whether a backend can encode one codec at a depth and chroma sampling.</summary>
    /// <remarks>The shared core owns this table; the harness compares both, so a family's limits
    /// cannot be relaxed on one side. A true result means "not refused by the family", not
    /// "available on this machine" — runtime probing still decides that.</remarks>
    public static bool Supports(string? backend, string codec, int depth, string sampling)
    {
        if (sampling is not ("420" or "422" or "444" or "gray"))
        {
            return false;
        }

        return backend switch
        {
            "software" => codec == "av1"
                ? sampling is "420" or "gray" && depth is 8 or 10
                : depth is 8 or 10 or 12,
            "qsv" or "nvenc" => sampling == "420" && depth is 8 or 10,
            "amf" => sampling == "420" && depth is 8 or 10 && codec != "av1",
            "media_foundation" => sampling == "420" && depth == 8 && codec != "av1",
            "videotoolbox" => sampling == "420" && depth is 8 or 10 && codec != "av1",
            _ => false
        };
    }

    /// <summary>Short product name for display; brand names are not localized.</summary>
    public static string Label(string? backend) => backend switch
    {
        "qsv" => "QSV",
        "amf" => "AMF",
        "nvenc" => "NVENC",
        "media_foundation" => "Media Foundation",
        "videotoolbox" => "VideoToolbox",
        "software" => "CPU",
        null => "—",
        _ => backend
    };
}
