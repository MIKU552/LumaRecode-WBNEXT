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
