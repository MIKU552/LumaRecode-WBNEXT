namespace VideoConverter.Core;

/// <summary>Reads the frame box: a frame number, or a time such as <c>00:01:23.12</c>.</summary>
/// <remarks>The shared core owns the grammar; this maps its answer onto the timestamp index.</remarks>
public static class FrameTimelineInput
{
    /// <summary>Returns a one-based frame number, or null when the text cannot be read.</summary>
    public static int? Resolve(string? text, FrameTimeline timeline)
    {
        if (string.IsNullOrWhiteSpace(text) || timeline.Count == 0)
        {
            return null;
        }
        var trimmed = text.Trim();
        if (trimmed.All(char.IsAsciiDigit))
        {
            return int.TryParse(trimmed, out var plain) ? Math.Clamp(plain, 1, timeline.Count) : null;
        }
        if (SharedCore.TimeInputParser?.Invoke(trimmed) is not { } time || !(timeline.Duration > 0))
        {
            return null;
        }
        // Frames named after the dot ride on the average frame period; the index search
        // then lands on the first frame at or past that moment.
        double seconds = time.Seconds + time.Frames * (timeline.Duration / timeline.Count);
        var start = timeline.Seconds(0);
        int index = 0;
        while (index < timeline.Count - 1 && timeline.Seconds(index + 1) - start <= seconds)
        {
            index++;
        }
        return index + 1;
    }
}
