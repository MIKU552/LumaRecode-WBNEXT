namespace VideoConverter.Core;

/// <summary>Produces viewport-sized SDR PNGs through one bounded FFmpeg pipe.</summary>
/// <remarks>Only previews resize before color rendering. Exports retain their full-resolution processing path.</remarks>
public sealed class FramePreview(Transcoder engine)
{
    internal static string ScaleFilter(int maximum) => $"scale=w='min(iw,{maximum})':h='min(ih,{maximum})':force_original_aspect_ratio=decrease:force_divisible_by=2";

    public async Task<byte[]> RenderAsync(MediaTools tools, MediaInfo source, FrameTimeline timeline,
        ImageExportOptions options, CancellationToken cancellation = default)
    {
        return await RenderAsync(tools, source, timeline, options, false, cancellation).ConfigureAwait(false);
    }

    /// <summary>Renders one frame; <paramref name="approximate"/> trades exactness for speed.</summary>
    public async Task<byte[]> RenderAsync(MediaTools tools, MediaInfo source, FrameTimeline timeline,
        ImageExportOptions options, bool approximate, CancellationToken cancellation = default)
    {
        var window = timeline.Resolve(new(options.Frame, options.Frame + 1));
        // While the slider moves a smaller frame arrives sooner and still shows the colour;
        // the exact frame is asked for again on release.
        var plan = engine.Plan(tools, source, FrameExporter.RenderOptions(source, options, sdr: true), "preview.mkv",
            window: window, previewSize: approximate ? 512 : 1024);
        int input = Array.IndexOf(plan.Arguments, "-i");
        var arguments = plan.Arguments.Take(input + 2).Skip(1).ToList();
        // Frame-threaded decoders otherwise decode many 4K frames ahead before
        // delivering the single requested frame. Limit that latency and memory.
        arguments.InsertRange(0, ["-threads:v", "4", "-filter_threads", "2"]);
        if (approximate)
        {
            // Long-GOP 4K spends seconds decoding from the keyframe to the exact frame; while
            // the slider is moving, the nearest keyframe keeps the viewport responsive.
            arguments.InsertRange(0, ["-noaccurate_seek"]);
        }
        string filter = plan.Arguments[Array.IndexOf(plan.Arguments, "-vf") + 1]
            + ",zscale=primaries=bt709:transfer=iec61966-2-1:matrix=gbr:range=full,format=gbrp16le,format=rgb24"
            + ",iccgen=color_primaries=bt709:color_trc=iec61966-2-1:force=1";
        arguments.AddRange(["-map", "0:" + source.Video.Text("index"), "-an", "-sn", "-dn", "-map_metadata", "-1", "-vf", filter,
            "-frames:v", "1", "-fps_mode", "passthrough", "-c:v", "png", "-compression_level", "3", "-f", "image2pipe", "pipe:1"]);
        DirectoryInfo? scratch = null;
        try
        {
            if (source.Raw is not null)
            {
                scratch = Directory.CreateTempSubdirectory("LumaRecode-preview-");
                await RawDevelopment.PrepareAsync(tools, source, Path.Combine(scratch.FullName, "raw-calibration.cmd"), null, cancellation, window).ConfigureAwait(false);
            }
            var result = await ProcessRunner.CaptureBinaryAsync(tools.Ffmpeg, arguments, cancellation,
                directory: scratch?.FullName ?? engine.Assets).ConfigureAwait(false);
            if (result.ExitCode != 0 || !result.Output.AsSpan().StartsWith((ReadOnlySpan<byte>)[137, 80, 78, 71, 13, 10, 26, 10]))
            {
                throw new InvalidDataException("图片提取失败：" + result.Error);
            }
            return result.Output;
        }
        finally { scratch?.Delete(recursive: true); }
    }
}
