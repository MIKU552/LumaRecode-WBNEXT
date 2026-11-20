using System.Globalization;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media.Imaging;
using VideoConverter.Core;
using Windows.Storage.Streams;

namespace VideoConverter;

public sealed partial class MainWindow
{
    // The packet/frame index and preview buffers are created only when needed.
    // A source change cancels both operations before clearing their UI state.
    private FrameTimeline? frameTimeline;
    private Task<FrameTimeline>? timelineTask;
    private CancellationTokenSource? timelineCancellation;
    private FramePreviewSession? previewSession;
    private Task previewTeardown = Task.CompletedTask;
    private (string Kind, string? Profile, MediaTools Tools)? previewIdentity;
    private int displayedFrame = -1;
    private bool frameUiUpdating, motionRangeEdited;
    private string previewError = "";
    private double jobDuration;
    private bool PhotoMode => Control<SelectorBar>("WorkPage").SelectedItem == Control<SelectorBarItem>("PhotoPage");
    private bool TrimVideo => !PhotoMode && Control<CheckBox>("TrimEnabled").IsChecked == true;
    private bool MotionImage => Control<CheckBox>("IncludeMotion").IsChecked == true;
    private int selectedFrame;
    private bool scrubbing;
    private long lastPreviewMs;

    /// <summary>True while the slider is held: previews may seek approximately.</summary>

    private int SelectedFrame => selectedFrame;
    private string ImageFormat => Value(Control<ComboBox>("ImageFormat"));
    private FrameRange VideoFrames => ReadRange("Trim");
    private FrameRange MotionFrames => ReadRange("Motion");
    private FrameRange ReadRange(string prefix) => new((int)Control<NumberBox>(prefix + "Start").Value - 1,
        (int)Control<NumberBox>(prefix + "End").Value);
    private ImageExportOptions SelectedImageOptions() => new(SelectedFrame, ImageFormat, Value(InputKind),
        Value(Profile) is { Length: > 0 } profile ? profile : null, MotionImage, MotionFrames,
        (int)Control<Slider>("ImageQuality").Value, Value(Control<ComboBox>("MotionEncoder")), Value(Control<ComboBox>("MotionResolution")));

    private void InitializeFrameControls()
    {
        Choices(Control<ComboBox>("ImageFormat"), ("ultrahdr", "Ultra HDR · JPEG"), ("hdr", "HDR · AVIF"),
            ("sdr", "SDR · JPEG"), ("png", "SDR · PNG"));
        Choices(Control<ComboBox>("MotionEncoder"), ("archive", L.T("软件编码 · CPU")), ("share", L.T("硬件编码 · Intel")));
        Choices(Control<ComboBox>("MotionResolution"), ("source", L.T("原始分辨率")), ("1080", L.T("最高 1080p")), ("720", L.T("最高 720p")));
        Control<ComboBox>("MotionEncoder").SelectionChanged += (_, _) => { if (!updating) { Refresh(); } };
        Control<SelectorBar>("WorkPage").SelectionChanged += async (_, _) =>
        {
            Refresh();
            UpdateTransferStats();
            if (PhotoMode)
            {
                await PreparePreviewAsync();
            }
            else if (Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").IsChecked != true)
            {
                CancelFramePreview();
                Control<Image>("FramePreview").Source = null;
            }
        };
        Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").Click += async (_, _) =>
        {
            // The panel opens and closes on its own state rather than the button's: a toggle
            // that does not flip leaves a button that looks pressed and does nothing.
            var overlay = Control<Grid>("PreviewOverlay");
            var open = overlay is null || overlay.Visibility != Visibility.Visible;
            var toggle = Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview");
            toggle.IsChecked = open;
            if (overlay is not null)
            {
                overlay.Visibility = open ? Visibility.Visible : Visibility.Collapsed;
            }
            if (open)
            {
                frameUiUpdating = false;
                await PreparePreviewAsync();
                UpdateFrameSelection();
            }
            else
            {
                CancelFramePreview();
                Control<Image>("FramePreview").Source = null;
            }
            UpdateFramePresentation();
        };
        var seek = Control<Slider>("FrameSeek");
        seek.ValueChanged += (_, args) => ChangePreviewFrame((int)Math.Round(args.NewValue) - 1);
        seek.PointerPressed += (_, _) => scrubbing = true;
        seek.PointerReleased += (_, _) =>
        {
            scrubbing = false;
            QueueFramePreview();
        };
        seek.PointerCaptureLost += (_, _) =>
        {
            if (scrubbing)
            {
                scrubbing = false;
                QueueFramePreview();
            }
        };
        // A frame can be named by number or by time, and Enter or leaving the box commits it.
        var frameNumber = Control<TextBox>("FrameNumber");
        frameNumber.KeyDown += (_, args) =>
        {
            if (args.Key == Windows.System.VirtualKey.Enter)
            {
                CommitFrameInput();
                args.Handled = true;
            }
        };
        frameNumber.LostFocus += (_, _) => CommitFrameInput();
        Control<Button>("PreviousFrame").Click += (_, _) => ChangePreviewFrame(SelectedFrame - 1);
        Control<Button>("NextFrame").Click += (_, _) => ChangePreviewFrame(SelectedFrame + 1);
        Control<Button>("ClosePreview").Click += (_, _) =>
        {
            Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").IsChecked = false;
            CancelFramePreview();
            Control<Image>("FramePreview").Source = null;
            UpdateFramePresentation();
        };
        foreach (var prefix in new[] { "Trim", "Motion" })
        {
            foreach (var edge in new[] { "Start", "End" })
            {
                // A time beside each edge does the same job as typing into the frame box.
                // These names are built from the loop, so a missing one must not take the
                // window down with it.
                var time = Control<TextBox>(prefix + edge + "Text");
                if (time is not null)
                {
                    void Apply() => ApplyTimeInput(time, Control<NumberBox>(prefix + edge));
                    time.KeyDown += (_, args) =>
                    {
                        if (args.Key == Windows.System.VirtualKey.Enter)
                        {
                            Apply();
                            args.Handled = true;
                        }
                    };
                    time.LostFocus += (_, _) => Apply();
                }

                var box = Control<NumberBox>(prefix + edge);
                if (box is not null)
                box.ValueChanged += (_, _) =>
                {
                    if (frameUiUpdating || updating)
                    {
                        return;
                    }
                    if (prefix == "Motion")
                    {
                        motionRangeEdited = true;
                    }
                    Refresh();
                    UpdateTransferStats();
                };
                if (Control<Button>("Set" + prefix + edge) is { } button && box is not null)
                {
                    button.Click += (_, _) => box.Value = SelectedFrame + 1;
                }
            }
        }
        Control<CheckBox>("TrimEnabled").Click += (_, _) => { Refresh(); UpdateTransferStats(); };
        Control<CheckBox>("IncludeMotion").Click += (_, _) => { Refresh(); UpdateTransferStats(); };
        Control<ComboBox>("ImageFormat").SelectionChanged += (_, _) => { if (!updating) { Refresh(); UpdateTransferStats(); } };
        Profile.SelectionChanged += (_, _) =>
        {
            if (!updating)
            {
                Refresh();
                QueueFramePreview();
            }
        };
        Closed += (_, _) => ResetFrameSource();
    }

    private void ResetFrameSource()
    {
        timelineCancellation?.Cancel();
        CancelFramePreview();
        timelineCancellation = null;
        timelineTask = null;
        frameTimeline = null;
        displayedFrame = -1;
        previewError = "";
        motionRangeEdited = false;
        frameUiUpdating = true;
        Control<Image>("FramePreview").Source = null;
        Control<TextBlock>("PreviewStatus").Text = "";
        Control<CheckBox>("TrimEnabled").IsChecked = false;
        Control<CheckBox>("IncludeMotion").IsChecked = false;
        selectedFrame = 0;
        Control<TextBox>("FrameNumber").Text = "1";
        foreach (var name in new[] { "TrimStart", "TrimEnd", "MotionStart", "MotionEnd" })
        {
            Control<NumberBox>(name).Value = 1;
        }
        frameUiUpdating = false;
    }

    private async Task<FrameTimeline> EnsureTimelineAsync()
    {
        var source = sourceMedia ?? throw new InvalidDataException(L.T("请选择有效的视频文件。"));
        if (frameTimeline?.Matches(source.Path) == true)
        {
            return frameTimeline;
        }
        if (timelineTask is not null)
        {
            return await timelineTask;
        }
        using var cancellation = new CancellationTokenSource();
        timelineCancellation = cancellation;
        var task = FrameTimeline.LoadAsync(ToolsObject(), source, cancellation.Token);
        timelineTask = task;
        Control<ProgressRing>("PreviewBusy").IsActive = true;
        Control<TextBlock>("PreviewStatus").Text = L.T("正在读取逐帧时间轴…");
        try
        {
            var timeline = await task;
            cancellation.Token.ThrowIfCancellationRequested();
            frameTimeline = timeline;
            frameUiUpdating = true;
            try
            {
                Control<Slider>("FrameSeek").Maximum = Math.Max(2, timeline.Count);
                Control<Slider>("FrameSeek").Value = 1;
                // The frame box is a text field; only the range boxes carry a maximum.
                foreach (var name in new[] { "TrimStart", "TrimEnd", "MotionStart", "MotionEnd" })
                {
                    Control<NumberBox>(name).Maximum = timeline.Count;
                }
                Control<NumberBox>("TrimEnd").Value = timeline.Count;
                SetDefaultMotionRange();
            }
            finally
            {
                frameUiUpdating = false;
            }
            return timeline;
        }
        finally
        {
            if (ReferenceEquals(timelineTask, task))
            {
                timelineTask = null;
                timelineCancellation = null;
                Control<ProgressRing>("PreviewBusy").IsActive = false;
                Refresh();
            }
        }
    }

    private async Task PreparePreviewAsync()
    {
        if (sourceMedia is null)
        {
            return;
        }
        var expectedSource = sourceMedia;
        try
        {
            await EnsureTimelineAsync();
            QueueFramePreview();
        }
        catch (OperationCanceledException) { }
        catch (Exception error)
        {
            if (!closing && ReferenceEquals(sourceMedia, expectedSource))
            {
                previewError = error.Message;
                Control<TextBlock>("PreviewStatus").Text = L.T(error.Message);
            }
        }
    }

    /// <summary>Puts a typed time into the frame box beside it, or clears it when unreadable.</summary>
    private void ApplyTimeInput(TextBox source, NumberBox target)
    {
        if (frameTimeline is null || frameUiUpdating)
        {
            return;
        }
        var frame = FrameTimelineInput.Resolve(source.Text, frameTimeline);
        if (frame is null)
        {
            source.Text = "";
            return;
        }
        target.Value = frame.Value;
    }

    /// <summary>Reads the frame box: a number, or a time such as <c>00:01:23.12</c>.</summary>
    private void CommitFrameInput()
    {
        if (frameTimeline is null || frameUiUpdating)
        {
            return;
        }
        var text = Control<TextBox>("FrameNumber").Text;
        var frame = FrameTimelineInput.Resolve(text, frameTimeline);
        if (frame is null)
        {
            Control<TextBox>("FrameNumber").Text = (selectedFrame + 1).ToString();
            return;
        }
        ChangePreviewFrame(frame.Value - 1);
    }

    private void ChangePreviewFrame(int frame)
    {
        if (frameUiUpdating || frameTimeline is null || activeJob is not null)
        {
            return;
        }
        frame = Math.Clamp(frame, 0, frameTimeline.Count - 1);
        // ValueChanged fires after the originating control has changed. Compare
        // both controls so direct NumberBox edits still update the slider.
        if (frame == SelectedFrame && frame == (int)Control<Slider>("FrameSeek").Value - 1)
        {
            return;
        }
        frameUiUpdating = true;
        try
        {
            selectedFrame = frame;
            Control<TextBox>("FrameNumber").Text = (frame + 1).ToString();
            Control<Slider>("FrameSeek").Value = frame + 1;
            if (!motionRangeEdited)
            {
                SetDefaultMotionRange();
            }
        }
        finally
        {
            // A throw in here used to leave the flag set, and every later drag was ignored.
            frameUiUpdating = false;
        }
        UpdateFrameSelection();
        UpdateFrameOutput();
        UpdateTransferStats();
        QueueFramePreview();
    }

    private void SetDefaultMotionRange()
    {
        if (frameTimeline is null)
        {
            return;
        }
        var selection = FrameExporter.DefaultMotionRange(frameTimeline, SelectedFrame);
        Control<NumberBox>("MotionStart").Value = selection.Start + 1;
        Control<NumberBox>("MotionEnd").Value = selection.End ?? frameTimeline.Count;
    }

    /// <summary>Queues one viewport request without rebuilding encoding controls or the page layout.</summary>
    private void QueueFramePreview()
    {
        if (sourceMedia is not { } source || frameTimeline is not { } timeline || activeJob is not null ||
            (!PhotoMode && Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").IsChecked != true))
        {
            return;
        }
        var options = SelectedImageOptions();
        var tools = ToolsObject();
        var identity = (options.SourceKind, options.Profile, tools);
        if (previewSession is null || previewIdentity != identity)
        {
            CancelFramePreview();
            var exporter = new FramePreview(ConversionClient.Engine);
            FramePreviewSession? session = null;
            session = new FramePreviewSession(
                async (request, cancellation) =>
                {
                    var clock = System.Diagnostics.Stopwatch.StartNew();
                    try
                    {
                        return await exporter.RenderAsync(tools, source, timeline,
                            options with
                            {
                                Frame = request.Frame,
                                SourceKind = request.SourceKind,
                                Profile = request.Profile
                            }, request.Approximate, cancellation).ConfigureAwait(false);
                    }
                    finally
                    {
                        lastPreviewMs = clock.ElapsedMilliseconds;
                    }
                },
                (request, bytes, cancellation) => OnPreviewUiAsync(async () =>
                {
                    if (!ReferenceEquals(previewSession, session))
                    {
                        return;
                    }
                    using var stream = new InMemoryRandomAccessStream();
                    using (var writer = new DataWriter(stream.GetOutputStreamAt(0)))
                    {
                        writer.WriteBytes(bytes);
                        await writer.StoreAsync();
                    }
                    stream.Seek(0);
                    var bitmap = new BitmapImage();
                    await bitmap.SetSourceAsync(stream);
                    cancellation.ThrowIfCancellationRequested();
                    Control<Image>("FramePreview").Source = bitmap;
                    displayedFrame = request.Frame;
                    previewError = "";
                    Control<ProgressRing>("PreviewBusy").IsActive = displayedFrame != SelectedFrame;
                    UpdateFrameSelection();
                }, cancellation),
                (error, cancellation) => OnPreviewUiAsync(() =>
                {
                    if (ReferenceEquals(previewSession, session))
                    {
                        previewError = error.Message;
                        Control<ProgressRing>("PreviewBusy").IsActive = false;
                        UpdateFrameSelection();
                    }
                    return Task.CompletedTask;
                }, cancellation));
            previewSession = session;
            previewIdentity = identity;
        }
        Control<ProgressRing>("PreviewBusy").IsActive = displayedFrame != SelectedFrame;
        previewSession.Request(new(SelectedFrame, options.SourceKind, options.Profile, scrubbing));
    }

    /// <summary>Posts WinRT bitmap work explicitly; cancellation also releases a dispatcher callback during shutdown.</summary>
    private async Task OnPreviewUiAsync(Func<Task> action, CancellationToken cancellation)
    {
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        using var registration = cancellation.Register(() => completion.TrySetCanceled(cancellation));
        if (!DispatcherQueue.TryEnqueue(async () =>
        {
            try
            {
                if (!closing && !cancellation.IsCancellationRequested)
                {
                    await action();
                }
                completion.TrySetResult();
            }
            catch (OperationCanceledException) { completion.TrySetCanceled(cancellation); }
            catch (Exception error) { completion.TrySetException(error); }
        }))
        {
            completion.TrySetResult();
        }
        await completion.Task.ConfigureAwait(false);
    }

    private void CancelFramePreview()
    {
        var previous = previewSession;
        previewSession = null;
        previewIdentity = null;
        displayedFrame = -1;
        if (previous is not null)
        {
            var teardown = previous.DisposeAsync().AsTask();
            previewTeardown = previewTeardown.IsCompletedSuccessfully ? teardown : Task.WhenAll(previewTeardown, teardown);
        }
        Control<ProgressRing>("PreviewBusy").IsActive = false;
    }

    /// <summary>Joins preview/index children before the last window releases the application lifetime.</summary>
    private async Task StopFrameWorkAsync()
    {
        var index = timelineTask;
        timelineCancellation?.Cancel();
        CancelFramePreview();
        try
        {
            await previewTeardown;
        }
        catch (Exception error) { conversionLog.Append(error.Message); }
        if (index is not null)
        {
            try
            {
                await index;
            }
            catch (OperationCanceledException) { }
            // The index owner reports read failures; teardown only joins it.
            catch (Exception error) { conversionLog.Append(error.Message); }
        }
    }

    private static string FrameTime(double seconds) => TimeSpan.FromSeconds(Math.Max(0, seconds)).ToString(@"hh\:mm\:ss\.fff", CultureInfo.InvariantCulture);

    private void UpdateFramePresentation()
    {
        bool photo = PhotoMode;
        bool enabled = sourceMedia is not null && activeJob is null && !probing;
        bool indexed = frameTimeline is not null;
        bool preview = sourceMedia is not null && (photo || Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").IsChecked == true);
        foreach (var name in new[] { "OutputCard", "ColorSettings", "FormatCard" })
        {
            Control<Border>(name).Visibility = photo ? Visibility.Collapsed : Visibility.Visible;
        }
        Control<Border>("PhotoSettings").Visibility = photo ? Visibility.Visible : Visibility.Collapsed;
        Control<Grid>("PreviewOverlay").Visibility = preview ? Visibility.Visible : Visibility.Collapsed;
        // The photo range lives in the same panel, so the opener has to exist in both modes.
        Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").Visibility = Visibility.Visible;
        Control<CheckBox>("TrimEnabled").Visibility = photo ? Visibility.Collapsed : Visibility.Visible;
        Control<Grid>("VideoTrimFields").Visibility = TrimVideo ? Visibility.Visible : Visibility.Collapsed;
        Control<Grid>("MotionFields").Visibility = photo && MotionImage ? Visibility.Visible : Visibility.Collapsed;
        Control<ComboBox>("MotionEncoder").Visibility = photo && MotionImage ? Visibility.Visible : Visibility.Collapsed;
        Control<ComboBox>("MotionEncoder").IsEnabled = enabled;
        Control<ComboBox>("MotionResolution").Visibility = photo && MotionImage ? Visibility.Visible : Visibility.Collapsed;
        Control<ComboBox>("MotionResolution").IsEnabled = enabled;
        Control<TextBlock>("MotionEncoderDetail").Visibility = photo && MotionImage ? Visibility.Visible : Visibility.Collapsed;
        if (photo && MotionImage)
        {
            string purpose = Value(Control<ComboBox>("MotionEncoder"));
            var preset = ConversionClient.Engine.MotionConfig[purpose]![ImageFormat == "sdr" ? "avc" : "hevc"]!;
            bool software = purpose == "archive";
            string encoder = preset.Text("encoder").Replace("libx", "x");
            bool mono = sourceMedia?.Audio.FirstOrDefault().Integer("channels") == 1;
            Control<TextBlock>("MotionEncoderDetail").Text = $"{encoder} · {(software ? preset.Text("preset") + " / CRF" : "TU" + preset.Text("preset") + " / ICQ")}{preset.Text("quality")} · AAC-LC / 48 kHz / {(mono ? 80 : 160)} kbps";
        }
        Control<SelectorBar>("WorkPage").IsEnabled = activeJob is null;
        foreach (var name in new[] { "ShowPreview", "ImageFormat", "ImageQuality" })
        {
            Control<Microsoft.UI.Xaml.Controls.Control>(name).IsEnabled = enabled;
        }
        foreach (var name in new[] { "FrameSeek", "FrameNumber", "PreviousFrame", "NextFrame", "TrimEnabled", "TrimStart", "TrimEnd", "SetTrimStart", "SetTrimEnd", "MotionStart", "MotionEnd", "SetMotionStart", "SetMotionEnd" })
        {
            Control<Microsoft.UI.Xaml.Controls.Control>(name).IsEnabled = enabled && indexed;
        }
        bool hdr = Kind is "hlg" or "pq" or "log" or "raw";
        var format = Control<ComboBox>("ImageFormat");
        foreach (var item in format.Items.OfType<ComboBoxItem>())
        {
            item.IsEnabled = hdr || (string)item.Tag is "sdr" or "png";
        }
        if (!hdr && ImageFormat is "hdr" or "ultrahdr")
        {
            Select(format, "sdr");
        }
        bool jpeg = ImageFormat is "sdr" or "ultrahdr";
        if (!jpeg)
        {
            Control<CheckBox>("IncludeMotion").IsChecked = false;
        }
        Control<CheckBox>("IncludeMotion").IsEnabled = enabled && indexed && jpeg;
        Control<Slider>("ImageQuality").IsEnabled = enabled && ImageFormat != "png";
        Profile.Visibility = Kind == "log" ? Visibility.Visible : Visibility.Collapsed;
        Profile.IsEnabled = enabled && Kind == "log";
        Control<TextBlock>("LocalizedElement56").Text = L.T(photo ? MotionImage ? "导出动态照片" : "导出图片" : "开始转换");
        AutomationProperties.SetName(Control<Button>("Start"), Control<TextBlock>("LocalizedElement56").Text);
        Control<TextBlock>("ImageFormatNote").Text = L.T(ImageFormat switch
        {
            "hdr" => "BT.2020 · PQ · 10/12 位 AVIF",
            "ultrahdr" => "SDR 主图与 HDR 增益图，兼容 Ultra HDR 查看器。",
            "png" => "sRGB · 无损 PNG",
            _ => "sRGB · JPEG"
        });
        UpdateFrameSelection();
        if (root.ActualWidth > 0)
        {
            Reflow(root.ActualWidth);
        }
    }

    /// <summary>Updates only frame text, range validation and start availability during scrubbing.</summary>
    private void UpdateFrameSelection()
    {
        bool photo = PhotoMode;
        bool enabled = sourceMedia is not null && activeJob is null && !probing;
        bool indexed = frameTimeline is not null;
        string warning = "";
        if (Kind == "unknown")
        {
            warning = L.T("请手动选择输入解释。");
        }
        else if (Kind == "log" && Value(Profile) == "")
        {
            warning = L.T("请选择相机曲线。");
        }
        else if (media?["raw"] is System.Text.Json.Nodes.JsonObject raw && !raw.Flag("decoder_supported"))
        {
            warning = L.T("当前 FFmpeg 无法解码此 ProRes RAW。请先用支持该相机的 RAW 软件显影为 ProRes 或 HEVC。");
        }
        else if (photo && ImageFormat is "hdr" or "ultrahdr" && SourceDepth < 10)
        {
            warning = L.T("HDR 输出需要至少 10 位的源素材，请选择 SDR 或高位深原片。");
        }
        if (TrimVideo && !audioUsesDefault && Value(Audio) == "copy" && sourceMedia?.Audio.Any() == true)
        {
            warning = L.T("逐帧裁剪需要重新编码音轨，请选择 AAC、Opus 或 FLAC。");
        }
        if (frameTimeline is { } timeline)
        {
            var frame = Math.Clamp(SelectedFrame, 0, timeline.Count - 1);
            var timing = lastPreviewMs > 0 ? $" · {lastPreviewMs} ms" : "";
            Control<TextBlock>("PreviewStatus").Text = previewError.Length > 0 ? L.T(previewError)
                : $"{frame + 1:N0} / {timeline.Count:N0} · {FrameTime(timeline.Position(frame))} / {FrameTime(timeline.Duration)}{timing}";
            foreach (var prefix in new[] { "Trim", "Motion" })
            {
                var range = ReadRange(prefix);
                if (range.Start >= 0 && range.Start < timeline.Count)
                {
                    Control<TextBlock>(prefix + "StartTime").Text = FrameTime(timeline.Position(range.Start));
                }
                if (range.End is > 0 && range.End <= timeline.Count)
                {
                    Control<TextBlock>(prefix + "EndTime").Text = FrameTime(timeline.Position(range.End.Value - 1));
                }
                if (prefix == "Trim" && TrimVideo || prefix == "Motion" && photo && MotionImage)
                {
                    try
                    {
                        timeline.Resolve(range);
                        if (prefix == "Motion" && (frame < range.Start || frame >= range.End))
                        {
                            warning = L.T("封面帧必须位于动态照片的视频范围内。");
                        }
                    }
                    catch (InvalidDataException error) { warning = L.T(error.Message); }
                }
            }
        }
        Control<TextBlock>("PhotoWarning").Text = media is null ? "" : warning;
        if (photo)
        {
            Control<Button>("Start").IsEnabled = enabled && indexed && warning == "";
        }
        else if (TrimVideo && warning != "")
        {
            Control<Button>("Start").IsEnabled = false;
        }
    }

    private void UpdateFrameOutput()
    {
        if (PhotoMode && media is not null && (Output.Text == "" || Output.Text == automaticOutput))
        {
            string source = MediaText("path");
            automaticOutput = Path.Combine(Path.GetDirectoryName(source)!, $"{Path.GetFileNameWithoutExtension(source)}_{SelectedFrame + 1:000000}_{ImageFormat}{(MotionImage ? "_MP" : "")}{Suffix}");
            Output.Text = automaticOutput;
        }
        else if (PhotoMode && MotionImage && Output.Text.Length > 0 &&
            Path.GetExtension(Output.Text).Equals(".jpg", StringComparison.OrdinalIgnoreCase) &&
            !Path.GetFileNameWithoutExtension(Output.Text).EndsWith("MP", StringComparison.Ordinal))
        {
            Output.Text = Path.Combine(Path.GetDirectoryName(Output.Text) ?? "",
                Path.GetFileNameWithoutExtension(Output.Text) + "_MP.jpg");
        }
    }
}
