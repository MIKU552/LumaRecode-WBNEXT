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
        SharedCore.ColorFilterBuilder = BuildColorFilters;
        SharedCore.RenderFilterBuilder = BuildRenderFilters;
        SharedCore.ColorTagsBuilder = BuildColorTags;
        SharedCore.Gop = ComputeGop;
        SharedCore.HdrMetadata = ComposeHdrMetadata;
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

    /// <summary>Derives the color tags through the core.</summary>
    private static ColorTagSet? BuildColorTags(ColorTagsRequest request)
    {
        JsonObject payload = new()
        {
            ["kind"] = request.Kind,
            ["output"] = request.Output,
            ["rgb"] = request.Rgb,
            ["dolby_profile5"] = request.DolbyProfile5,
            ["source_range"] = request.SourceRange,
            ["output_range"] = request.OutputRange,
        };
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

        JsonNode? data = LumaCoreNative.Call("plan.color_tags", payload);
        if (data?["source"] is not JsonObject source || data["target"] is not JsonObject target)
        {
            return null;
        }

        return new ColorTagSet(Tags(source), Tags(target));
    }

    /// <summary>Reads one tag set, mapping JSON null and empty values to null.</summary>
    private static ColorTagsValue Tags(JsonObject data)
        => new(Entry(data, "primaries"), Entry(data, "transfer"), Entry(data, "space"), Entry(data, "range"));

    /// <summary>Builds the signal-dependent filters through the core.</summary>
    private static RenderFilters? BuildRenderFilters(RenderFiltersRequest request)
    {
        JsonObject payload = new();
        if (request.Log is { } log)
        {
            JsonObject entry = new()
            {
                ["file"] = log.File,
                ["matrix"] = log.Matrix,
                ["range"] = log.Range,
            };
            if (log.SourceSpace is not null)
            {
                entry["source_space"] = log.SourceSpace;
            }

            if (log.SourceRange is not null)
            {
                entry["source_range"] = log.SourceRange;
            }

            payload["log"] = entry;
        }

        if (request.ToneMap is { } tone)
        {
            JsonObject entry = new()
            {
                ["range"] = tone.Range,
                ["apply_dolby_vision"] = tone.ApplyDolbyVision,
            };
            payload["tone_map"] = entry;
        }

        if (request.DolbyRender is { } dolby)
        {
            JsonObject entry = new()
            {
                ["kind"] = dolby.Kind,
                ["range"] = dolby.Range,
            };
            payload["dolby_render"] = entry;
        }

        if (request.Vpp is { } vpp)
        {
            JsonObject entry = new()
            {
                ["depth"] = vpp.Depth,
                ["out_range"] = vpp.OutRange,
                ["sdr"] = vpp.Sdr,
            };
            payload["vpp"] = entry;
        }

        payload["drop_dynamic_metadata"] = request.DropDynamicMetadata;
        if (LumaCoreNative.Call("plan.render_filters", payload) is not JsonObject data)
        {
            return null;
        }

        return new RenderFilters(
            Entry(data, "log"),
            Entry(data, "tone_map"),
            Entry(data, "dolby_render"),
            Entry(data, "dropped_metadata"),
            Entry(data, "vpp"));
    }

    /// <summary>Reads a string entry, mapping JSON null and empty values to null.</summary>
    private static string? Entry(JsonObject data, string key)
        => data[key]?.ToString() is { Length: > 0 } text ? text : null;

    /// <summary>Builds the software colour conversion filters through the core.</summary>
    private static IReadOnlyList<string>? BuildColorFilters(ColorFilterRequest request)
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

        payload["conversion_range"] = request.ConversionRange;
        payload["pixel_format"] = request.PixelFormat;
        payload["purpose"] = request.Purpose;
        payload["depth"] = request.Depth;
        payload["preserve_dynamic"] = request.PreserveDynamic;
        if (LumaCoreNative.Call("plan.color_filters", payload)?["filters"] is not JsonArray filters)
        {
            return null;
        }

        List<string> result = new(filters.Count);
        foreach (JsonNode? node in filters)
        {
            result.Add(node?.ToString() ?? "");
        }

        return result;
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

    /// <summary>Composes the HDR metadata parameters through the core.</summary>
    private static HdrMetadataArguments? ComposeHdrMetadata(HdrMetadataRequest request)
    {
        JsonObject payload = new();
        payload["codec"] = request.Codec;
        payload["output"] = request.Output;
        payload["source_kind"] = request.SourceKind;
        payload["preserve_dynamic"] = request.PreserveDynamic;
        payload["hdr10_plus"] = request.Hdr10Plus;
        if (request.DolbyVisionProfile is int profile)
        {
            payload["dolby_vision_profile"] = profile;
        }

        JsonArray sideData = new();
        foreach (JsonObject entry in request.SideData)
        {
            sideData.Add(entry.DeepClone());
        }

        payload["side_data"] = sideData;
        if (LumaCoreNative.Call("plan.hdr_metadata", payload) is not JsonObject data)
        {
            return null;
        }

        List<ParameterPair> parameters = new();
        JsonArray entries = data["parameters"] as JsonArray ?? new JsonArray();
        foreach (JsonNode? entry in entries)
        {
            parameters.Add(new ParameterPair(entry?["key"]?.ToString() ?? "", entry?["value"]?.ToString() ?? ""));
        }

        List<string> appended = new();
        JsonArray tokens = data["appended"] as JsonArray ?? new JsonArray();
        foreach (JsonNode? token in tokens)
        {
            appended.Add(token?.ToString() ?? "");
        }

        return new HdrMetadataArguments(parameters, appended);
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
