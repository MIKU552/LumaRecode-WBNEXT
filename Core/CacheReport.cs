using System.Text.Json.Nodes;

namespace VideoConverter.Core;

/// <summary>Read-only report of the extraction cache this program owns.</summary>
/// <remarks>
/// Plan §10.4 requires the application to know its cache before reclaiming it. This report
/// only measures: it never deletes, never follows links out of the root, and returns zeros
/// when the directory is absent so a fresh install reads the same shape as a used one.
/// </remarks>
public static class CacheReport
{
    /// <summary>Directory owned by this program inside the per-user cache location.</summary>
    public static string Root { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "luma-recode");

    /// <summary>Measures every build directory and totals them.</summary>
    public static JsonObject Read()
    {
        var result = new JsonObject { ["root"] = Root, ["entries"] = new JsonArray(), ["bytes"] = 0L, ["oldest_days"] = 0 };
        try
        {
            if (!Directory.Exists(Root))
            {
                return result;
            }

            long total = 0;
            var oldest = 0;
            foreach (var directory in Directory.GetDirectories(Root))
            {
                long size = 0;
                try
                {
                    foreach (var file in Directory.EnumerateFiles(directory, "*", SearchOption.AllDirectories))
                    {
                        try
                        {
                            size += new FileInfo(file).Length;
                        }
                        catch (IOException)
                        {
                            // A file removed while scanning simply does not count.
                        }
                    }
                }
                catch (Exception error) when (error is IOException or UnauthorizedAccessException)
                {
                    size = 0;
                }

                var age = (int)Math.Max(0, (DateTime.UtcNow - Directory.GetLastWriteTimeUtc(directory)).TotalDays);
                result["entries"]!.AsArray().Add(new JsonObject
                {
                    ["key"] = Path.GetFileName(directory),
                    ["bytes"] = size,
                    ["age_days"] = age
                });
                total += size;
                oldest = Math.Max(oldest, age);
            }

            result["bytes"] = total;
            result["oldest_days"] = oldest;
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            result["error"] = error is UnauthorizedAccessException ? "denied" : "unreadable";
        }

        return result;
    }

    /// <summary>Deletes the directories a plan marked for removal.</summary>
    /// <remarks>
    /// Only paths that resolve inside <see cref="Root"/> are touched. Each deletion is
    /// independent: a busy or protected directory stays on disk and is reported, which matches
    /// §10.4 rather than forcing the removal.
    /// </remarks>
    /// <param name="plan">Output of the core's cache plan.</param>
    public static JsonObject Purge(JsonObject plan)
    {
        JsonArray removed = new();
        JsonArray failed = new();
        long bytes = 0;
        string root = Path.GetFullPath(Root) + Path.DirectorySeparatorChar;
        JsonArray decisions = plan["decisions"] as JsonArray ?? new JsonArray();
        foreach (JsonObject decision in decisions.Objects())
        {
            string name = decision.Text("key");
            if (!decision.Text("decision").StartsWith("remove_", StringComparison.Ordinal) || string.IsNullOrWhiteSpace(name))
            {
                continue;
            }

            string path = Path.GetFullPath(Path.Combine(Root, name));
            if (!path.StartsWith(root, StringComparison.OrdinalIgnoreCase) || !Directory.Exists(path))
            {
                failed.Add((JsonNode)new JsonObject { ["key"] = name, ["reason"] = "outside_root" });
                continue;
            }

            try
            {
                long size = Directory.EnumerateFiles(path, "*", SearchOption.AllDirectories)
                    .Sum(file => new FileInfo(file).Length);
                Directory.Delete(path, recursive: true);
                removed.Add((JsonNode)new JsonObject { ["key"] = name, ["bytes"] = size });
                bytes += size;
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException)
            {
                failed.Add((JsonNode)new JsonObject { ["key"] = name, ["reason"] = error.GetType().Name });
            }
        }

        JsonObject result = new();
        result["removed"] = removed;
        result["failed"] = failed;
        result["bytes"] = bytes;
        return result;
    }
}
