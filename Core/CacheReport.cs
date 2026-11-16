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
}
