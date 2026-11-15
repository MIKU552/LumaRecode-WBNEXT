using System.Text.Json.Nodes;
using VideoConverter.Core;

namespace VideoConverter.Native;

/// <summary>Translates a probe snapshot into the shared core's normalized source description.</summary>
/// <remarks>
/// This is the only place that knows both shapes. Rules in the shared core must never read
/// ffprobe JSON, and probe code must never decide an output format, so the boundary stays narrow:
/// ffprobe fields in, <c>SourceDto</c> fields out.
/// </remarks>
public static class ProbeAdapter
{
    /// <summary>Builds the request payload for one probed source.</summary>
    public static JsonObject Describe(MediaInfo media)
    {
        JsonObject pixel = media.Pixel;
        JsonObject flags = pixel["flags"] as JsonObject ?? new JsonObject();
        JsonObject video = media.Video;
        JsonObject payload = new()
        {
            ["pixel_format"] = video.Text("pix_fmt"),
            ["rgb"] = flags.Flag("rgb"),
            ["palette"] = flags.Flag("palette"),
            ["bayer"] = flags.Flag("bayer"),
            ["alpha"] = flags.Flag("alpha"),
            ["monochrome"] = video.Flag("_monochrome"),
            ["components"] = ComponentDepths(pixel),
            ["kind"] = media.Kind,
            ["display_matrix"] = HasDisplayMatrix(media),
            ["dynamic_hdr"] = DynamicHdr(media),
            ["audio"] = AudioTracks(media),
        };
        int componentCount = pixel.Integer("nb_components");
        if (componentCount > 0)
        {
            payload["component_count"] = componentCount;
        }

        int rawSample = video.Integer("bits_per_raw_sample");
        if (rawSample > 0)
        {
            payload["bits_per_raw_sample"] = rawSample;
        }

        int declaredSample = video.Integer("bits_per_sample");
        if (declaredSample > 0)
        {
            payload["bits_per_sample"] = declaredSample;
        }

        if (pixel.ContainsKey("log2_chroma_w"))
        {
            payload["log2_chroma_w"] = pixel.Integer("log2_chroma_w");
        }

        if (pixel.ContainsKey("log2_chroma_h"))
        {
            payload["log2_chroma_h"] = pixel.Integer("log2_chroma_h");
        }

        if (video.Text("color_range") is { Length: > 0 } range)
        {
            payload["color_range"] = range;
        }

        if (media.Profile is { Length: > 0 } profile)
        {
            payload["profile"] = profile;
        }

        if (media.Raw is { } raw)
        {
            payload["raw"] = true;
            payload["raw_decoder_supported"] = raw.Flag("decoder_supported");
            payload["raw_decoder_version_supported"] = raw.Flag("decoder_version_supported");
        }

        return payload;
    }

    /// <summary>Per-component storage depth in file order; the core separates alpha itself.</summary>
    private static JsonArray ComponentDepths(JsonObject pixel)
    {
        JsonArray depths = new();
        foreach (JsonObject component in pixel["components"].Objects())
        {
            depths.Add((JsonNode?)JsonValue.Create(component.Integer("bit_depth")));
        }

        return depths;
    }

    /// <summary>A display matrix changes pixels during decode and must force system-memory frames.</summary>
    private static bool HasDisplayMatrix(MediaInfo media)
        => media.SideData.Any(side => side.Text("side_data_type") == "Display Matrix");

    private static JsonObject DynamicHdr(MediaInfo media)
    {
        JsonObject payload = new()
        {
            ["hdr10_plus"] = media.Hdr10Plus
        };
        if (media.DolbyVision is { } dolby)
        {
            payload["dolby_vision"] = new JsonObject
            {
                ["profile"] = dolby.Integer("dv_profile"),
                ["bl_signal_compatibility_id"] = dolby.Integer("dv_bl_signal_compatibility_id"),
                ["rpu_present_flag"] = dolby.Integer("rpu_present_flag"),
                ["bl_present_flag"] = dolby.Integer("bl_present_flag"),
                ["el_present_flag"] = dolby.Integer("el_present_flag"),
            };
        }

        return payload;
    }

    private static JsonArray AudioTracks(MediaInfo media)
    {
        JsonArray tracks = new();
        foreach (JsonObject stream in media.Audio)
        {
            JsonObject track = new()
            {
                ["codec_name"] = stream.Text("codec_name"),
                ["profile"] = stream.Text("profile"),
                ["sample_format"] = stream.Text("sample_fmt"),
                ["bits_per_raw_sample"] = stream.Integer("bits_per_raw_sample"),
                ["bits_per_sample"] = stream.Integer("bits_per_sample"),
                ["channels"] = stream.Integer("channels"),
            };
            if (stream.Text("channel_layout") is { Length: > 0 } layout)
            {
                track["channel_layout"] = layout;
            }

            tracks.Add((JsonNode)track);
        }

        return tracks;
    }
}
