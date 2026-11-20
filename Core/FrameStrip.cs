namespace VideoConverter.Core;

/// <summary>A low-resolution still for every few frames, decoded in one sequential pass.</summary>
/// <remarks>Seeking a 4K long-GOP file per preview frame costs seconds; one pass over the
/// whole clip costs the same once and then costs nothing to browse.</remarks>
public sealed class FrameStrip : IDisposable
{
    private readonly DirectoryInfo folder;
    public IReadOnlyList<string> Files { get; }
    public int StepFrames { get; }
    public int Count => Files.Count;

    private FrameStrip(DirectoryInfo folder, IReadOnlyList<string> files, int stepFrames)
    {
        this.folder = folder;
        Files = files;
        StepFrames = stepFrames;
    }

    /// <summary>Index of the still that stands for a frame.</summary>
    public int IndexOf(int frame) => StepFrames <= 0 ? 0 : Math.Clamp(frame / StepFrames, 0, Files.Count - 1);

    public static async Task<FrameStrip> CreateAsync(MediaTools tools, MediaInfo source, FrameTimeline timeline,
        int width, int target, CancellationToken cancellation = default)
    {
        if (timeline.Count == 0 || !(timeline.Duration > 0))
        {
            throw new InvalidDataException("逐帧时间轴为空，无法生成预览缩略。");
        }
        target = Math.Clamp(target, 8, Math.Max(8, timeline.Count));
        var step = Math.Max(1, (int)Math.Ceiling(timeline.Count / (double)target));
        var folder = Directory.CreateTempSubdirectory("LumaRecode-strip-");
        var arguments = new List<string>
        {
            "-hide_banner", "-nostdin", "-n", "-loglevel", "error",
            "-i", source.Path,
            "-map", "0:" + source.Video.Text("index"),
            "-vf", $"fps={target / timeline.Duration:0.####},scale={width}:-2:flags=lanczos",
            "-frames:v", target.ToString(System.Globalization.CultureInfo.InvariantCulture),
            "-q:v", "4", "-f", "image2", "-start_number", "0",
            Path.Combine(folder.FullName, "s-%04d.jpg"),
        };
        // The whole point is one pass: let it use the machine.
        arguments.InsertRange(0, ["-threads:v", "0"]);
        try
        {
            // A whole 4K clip takes far longer than the default text-capture timeout.
            var result = await ProcessRunner.CaptureAsync(tools.Ffmpeg, arguments, cancellation,
                timeoutSeconds: 900).ConfigureAwait(false);
            if (result.ExitCode != 0)
            {
                throw new InvalidDataException("预览缩略生成失败：" + result.Error);
            }
            var files = folder.EnumerateFiles("s-*.jpg").OrderBy(file => file.Name, StringComparer.Ordinal)
                .Select(file => file.FullName).ToList();
            if (files.Count == 0)
            {
                throw new InvalidDataException("预览缩略生成失败：没有输出。");
            }
            return new FrameStrip(folder, files, Math.Max(1, timeline.Count / files.Count));
        }
        catch
        {
            folder.Delete(recursive: true);
            throw;
        }
    }

    public void Dispose()
    {
        try
        {
            folder.Delete(recursive: true);
        }
        catch (IOException) { }
    }
}
