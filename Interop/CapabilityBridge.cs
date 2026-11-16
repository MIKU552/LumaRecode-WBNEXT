using System.Text.Json.Nodes;
using VideoConverter.Core;

namespace VideoConverter.Native;

/// <summary>Asks the shared core to classify the encoders FFmpeg reports.</summary>
/// <remarks>
/// The host collects the encoder list because only it can run FFmpeg; the core decides what
/// each name means. A compiled-in encoder is not proof of hardware, so the answer separates
/// available families from ones that still need device evidence.
/// </remarks>
public static class CapabilityBridge
{
    /// <summary>Maps encoder names to backend states.</summary>
    public static JsonObject Detect(IEnumerable<string> encoders)
    {
        JsonArray names = new();
        foreach (string encoder in encoders)
        {
            names.Add(JsonValue.Create(encoder));
        }

        JsonObject request = new();
        request["encoders"] = names;
        return LumaCoreNative.Call("capability.detect", request) as JsonObject ?? new JsonObject();
    }
}
