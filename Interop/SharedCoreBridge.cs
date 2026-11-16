using System.Text.Json.Nodes;
using VideoConverter.Core;

namespace VideoConverter.Native;

/// <summary>Connects <see cref="SharedCore.Resolver"/> to the native library.</summary>
/// <remarks>
/// Installing is safe when the library is missing: the resolver stays null and every call
/// keeps using the managed rules, so the application starts and converts exactly as before.
/// </remarks>
public static class SharedCoreBridge
{
    /// <summary>True when the shared core answers resolve and validate in this process.</summary>
    public static bool Active
    {
        get;
        private set;
    }

    /// <summary>Version reported by the core, or an empty string when it is absent.</summary>
    public static string Version => SharedCore.CoreVersion;

    /// <summary>Routes format resolution through the shared core when it is available.</summary>
    public static bool Install()
    {
        if (!LumaCoreNative.IsAvailable)
        {
            SharedCore.Reset();
            Active = false;
            return false;
        }

        SharedCore.Resolver = Resolve;
        SharedCore.Validator = Validate;
        SharedCore.InputBuilder = BuildInputMap;
        SharedCore.ColorBuilder = BuildColorArguments;
        SharedCore.EncoderBuilder = BuildEncoderArguments;
        SharedCore.ParameterBuilder = BuildParameterString;
        SharedCore.FilterBuilder = BuildFilterChain;
        SharedCore.Gop = ComputeGop;
        SharedCore.CoreVersion = LumaCoreNative.CoreVersion;
        Active = true;
        return true;
    }

    /// <summary>Builds the input/mapping block through the core.</summary>
    private static IReadOnlyList<string>? BuildInputMap(InputMapRequest request)
    {
        JsonObject payload = new();
        payload["input"] = request.Input;
        payload["video_index"] = request.VideoIndex;
        payload["audio_map"] = request.AudioMap;
        payload["preserve_metadata"] = request.PreserveMetadata;
        payload["preserve_chapters"] = request.PreserveChapters;
        if (LumaCoreNative.Call("plan.input_map", payload)?["arguments"] is not JsonArray arguments)
        {
            return null;
        }

        List<string> result = new(arguments.Count);
        foreach (JsonNode? node in arguments)
        {
            result.Add(node?.ToString() ?? "");
        }

        return result;
    }

    /// <summary>Builds the color-tag block through the core.</summary>
    private static IReadOnlyList<string>? BuildColorArguments(ColorTagRequest request)
    {
        JsonObject payload = new();
        if (request.Primaries is not null)
        {
            payload["primaries"] = request.Primaries;
        }

        if (request.Transfer is not null)
        {
            payload["transfer"] = request.Transfer;
        }

        if (request.Space is not null)
        {
            payload["space"] = request.Space;
        }

        if (request.Range is not null)
        {
            payload["range"] = request.Range;
        }

        payload["suppress_dv5_tags"] = request.SuppressDv5Tags;
        if (LumaCoreNative.Call("plan.color_args", payload)?["arguments"] is not JsonArray arguments)
        {
            return null;
        }

        List<string> result = new(arguments.Count);
        foreach (JsonNode? node in arguments)
        {
            result.Add(node?.ToString() ?? "");
        }

        return result;
    }

    /// <summary>Builds the encoder block through the core.</summary>
    private static IReadOnlyList<string>? BuildEncoderArguments(EncoderRequest request)
    {
        JsonObject payload = new();
        payload["encoder"] = request.Encoder;
        payload["preset"] = request.Preset;
        payload["quality"] = request.Quality;
        payload["share"] = request.Share;
        payload["codec"] = request.Codec;
        payload["depth"] = request.Depth;
        payload["gop"] = request.Gop;
        payload["auto_low_power"] = request.AutoLowPower;
        payload["timing_compatibility"] = request.TimingCompatibility;
        payload["async_depth"] = request.AsyncDepth;
        payload["pixel_format"] = request.PixelFormat;
        if (LumaCoreNative.Call("plan.encoder_args", payload)?["arguments"] is not JsonArray arguments)
        {
            return null;
        }

        List<string> result = new(arguments.Count);
        foreach (JsonNode? node in arguments)
        {
            result.Add(node?.ToString() ?? "");
        }

        return result;
    }

    /// <summary>Formats encoder parameters through the core.</summary>
    private static string? BuildParameterString(IReadOnlyList<ParameterPair> values, IReadOnlyList<string> appended)
    {
        JsonArray pairs = new();
        foreach (ParameterPair pair in values)
        {
            JsonObject item = new();
            item["key"] = pair.Key;
            item["value"] = pair.Value;
            pairs.Add((JsonNode)item);
        }

        JsonArray tokens = new();
        foreach (string token in appended)
        {
            tokens.Add((JsonNode)token);
        }

        JsonObject payload = new();
        payload["values"] = pairs;
        payload["appended"] = tokens;
        return LumaCoreNative.Call("plan.parameter_string", payload)?["text"]?.ToString();
    }

    /// <summary>Joins the filter chain through the core.</summary>
    private static string? BuildFilterChain(IReadOnlyList<string> filters)
    {
        JsonArray items = new();
        foreach (string filter in filters)
        {
            items.Add((JsonNode)filter);
        }

        JsonObject payload = new();
        payload["filters"] = items;
        return LumaCoreNative.Call("plan.filter_chain", payload)?["text"]?.ToString();
    }

    /// <summary>Computes the GOP length through the core.</summary>
    private static long? ComputeGop(GopRequest request)
    {
        JsonObject payload = new();
        payload["frame_rate"] = request.FrameRate;
        payload["share"] = request.Share;
        payload["motion_photo"] = request.MotionPhoto;
        JsonNode? frames = LumaCoreNative.Call("plan.gop", payload)?["frames"];
        return frames is null ? null : frames.GetValue<long>();
    }

    /// <summary>Restores the managed rules.</summary>
    public static void Uninstall()
    {
        SharedCore.Reset();
        Active = false;
    }


    private static SharedValidation Validate(EncodeOptions options, MediaInfo media)
    {
        JsonObject request = new()
        {
            ["source"] = ProbeAdapter.Describe(media),
            ["options"] = options.ToJson()
        };
        if (!LumaCoreNative.TryCall("policy.validate", request, out JsonNode? data, out LumaCoreError? error))
        {
            return new SharedValidation(null, error?.Code, error?.Detail);
        }

        return new SharedValidation(data?["source_kind"]?.ToString(), null, null);
    }

    private static SharedResolution Resolve(EncodeOptions options, MediaInfo media)
    {
        JsonObject request = new()
        {
            ["source"] = ProbeAdapter.Describe(media),
            ["options"] = options.ToJson()
        };
        if (!LumaCoreNative.TryCall("policy.resolve", request, out JsonNode? data, out LumaCoreError? error))
        {
            return new SharedResolution(null, error?.Code, error?.Detail);
        }

        return new SharedResolution(
            options with
            {
                Sampling = data?["sampling"]?.ToString() ?? options.Sampling,
                ColorRange = data?["color_range"]?.ToString() ?? options.ColorRange,
                BitDepth = data?["bit_depth"]?.ToString() ?? options.BitDepth
            },
            null,
            null);
    }
}
