using System.Text.Json.Nodes;
using VideoConverter.Core;

namespace VideoConverter.Native;

/// <summary>Asks the shared core which cache directories may be reclaimed.</summary>
/// <remarks>
/// The host owns the filesystem and the core owns the rules, so the plan is computed from an
/// inventory the caller already measured. Nothing here deletes: it returns decisions other
/// code can act on, which keeps the destructive step small and reviewable.
/// </remarks>
public static class CacheBridge
{
    /// <summary>Plans retention for the entries in a cache inventory.</summary>
    /// <param name="inventory">Report produced by <see cref="CacheReport"/>.</param>
    public static JsonObject Plan(JsonObject inventory)
    {
        JsonArray entries = new();
        foreach (JsonObject entry in (inventory["entries"] as JsonArray ?? new JsonArray()).Objects())
        {
            entries.Add(new JsonObject
            {
                ["key"] = entry.Text("key"),
                ["age_days"] = entry.Integer("age_days"),
                ["bytes"] = entry["bytes"]?.GetValue<long>() ?? 0,
                ["in_use"] = false
            });
        }

        JsonObject request = new()
        {
            ["entries"] = entries,
            ["current"] = "",
            ["policy"] = new JsonObject
            {
                ["keep_recent"] = 2,
                ["max_age_days"] = 30,
                ["max_bytes"] = 2L * 1024 * 1024 * 1024
            }
        };
        return LumaCoreNative.Call("cache.plan", request) as JsonObject ?? new JsonObject();
    }
}
