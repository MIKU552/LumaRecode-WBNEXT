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
    /// <summary>Routes format resolution through the shared core when it is available.</summary>
    public static bool Install()
    {
        if (!LumaCoreNative.IsAvailable)
        {
            SharedCore.Reset();
            return false;
        }

        SharedCore.Resolver = Resolve;
        SharedCore.Validator = Validate;
        return true;
    }

    /// <summary>Restores the managed rules.</summary>
    public static void Uninstall() => SharedCore.Reset();

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
