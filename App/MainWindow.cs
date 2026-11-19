using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using VideoConverter.Core;
using Windows.ApplicationModel.DataTransfer;

namespace VideoConverter;

/// <summary>Owns GUI job state, source-dependent choices and the responsive WinUI layout.</summary>
/// <remarks>Core performs final validation and encoding; this class previews those constraints before starting a job.</remarks>
public sealed partial class MainWindow : Window
{
    private static readonly JsonElement encodingConfig = LoadEncodingConfig();
    private readonly Grid root;
    private readonly ViewLocalization viewLocalization = new();
    private readonly Dictionary<string, object> controls = new();
    private readonly List<(ComboBoxItem Item, string Original)> choiceLabels = new();
    private readonly Dictionary<string, RadioMenuFlyoutItem> languageItems = new();
    private readonly UserSettings settings = UserSettings.Load();
    private readonly Appearance appearance;
    private ConversionClient? activeJob;
    private readonly JobNotifications notifications;
    private string jobInputPath = "";
    private bool jobTerminalHandled;
    // Selection changes also fire during programmatic updates. The updating
    // guard prevents recursive Refresh calls; default flags preserve explicit
    // user audio/depth choices until the purpose or source changes.
    private bool updating, probing, closing, dialogOpen, audioUsesDefault = true, depthUsesDefault = true;
    // Ignore delayed probe events after the selected source/window lifecycle changes.
    private int generation;
    private string ffmpeg = "", ffprobe = "", automaticOutput = "", completedOutput = "";
    private JsonObject? media;
    private MediaInfo? sourceMedia;
    // Keep explicit preference separate from mandatory/automatic selection so
    // a temporary software-only workflow never becomes a sticky user choice.
    private bool preferSoftwareDecode, softwareDecodeFallback;
    private EncodeOptions? fallbackDecodeOptions;
    private bool? activeSoftwareDecode;
    private readonly ConversionLog conversionLog = new();
    private long lastLogUpdate;
    private int copyLogGeneration;
    private readonly Stopwatch clock = new();
    private readonly Microsoft.UI.Dispatching.DispatcherQueueTimer clockTimer;
    private double encodedSeconds, encodedFrames, conversionSpeed, currentFps;
    private bool completed;
    private readonly List<ConversionClient> probes = new();
    private TextBox Input => Control<TextBox>("InputPath");
    private TextBox Output => Control<TextBox>("Destination");
    private ComboBox Purpose => Control<ComboBox>("Purpose");
    private ComboBox Codec => Control<ComboBox>("Codec");
    private ComboBox Variant => Control<ComboBox>("Variant");
    private ComboBox Container => Control<ComboBox>("Container");
    private RadioButtons Range => Control<RadioButtons>("OutputRange");
    private ComboBox Transform => Control<ComboBox>("Transform");
    private ComboBox DynamicMetadata => Control<ComboBox>("DynamicMetadata");
    private ComboBox Profile => Control<ComboBox>("Profile");
    private ComboBox InputKind => Control<ComboBox>("InputKind");
    private ComboBox Sampling => Control<ComboBox>("Sampling");
    private ComboBox BitDepth => Control<ComboBox>("BitDepth");
    private ComboBox ColorRange => Control<ComboBox>("ColorRange");
    private ComboBox Audio => Control<ComboBox>("Audio");
    private string Kind => Value(InputKind) == "auto" ? MediaText("kind", "unknown") : Value(InputKind);
    private bool Hdr => Range.SelectedIndex == 1;
    private string previousSuffix = ".mp4";
    private string Suffix => PhotoMode ? ImageFormat == "hdr" ? ".avif" : ImageFormat == "png" ? ".png" : ".jpg"
        : Value(Container) switch
        {
            "mp4" => ".mp4",
            "mkv" => ".mkv",
            _ => Value(Purpose) == "archive" || Value(Audio) == "flac" ? ".mkv" : ".mp4"
        };
    private int Depth => int.TryParse(Value(BitDepth), out var depth) ? depth : 8;
    private int SourceDepth => media.Integer("source_depth", 8);

    public MainWindow()
    {
        Title = L.T("映码 · LumaRecode WBNEXT");
        // Compiled XBF avoids runtime XML parsing and reflection over controls.
        // Appearance selects the appropriate native Mica host once the view exists.
        root = new MainView();
        Content = root;
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(Control<Grid>("TitleDrag"));
        AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "LumaRecode.ico"));
        var dpi = GetDpiForWindow(WinRT.Interop.WindowNative.GetWindowHandle(this)) / 96d;
        var work = Microsoft.UI.Windowing.DisplayArea.GetFromWindowId(AppWindow.Id, Microsoft.UI.Windowing.DisplayAreaFallback.Primary).WorkArea;
        var width = (int)Math.Min(1120 * dpi, work.Width - 64 * dpi);
        var height = (int)Math.Min(900 * dpi, work.Height - 64 * dpi);
        AppWindow.MoveAndResize(new Windows.Graphics.RectInt32(work.X + (work.Width - width) / 2,
            work.Y + (work.Height - height) / 2, width, height));
        clockTimer = DispatcherQueue.CreateTimer();
        clockTimer.Interval = TimeSpan.FromSeconds(1);
        clockTimer.Tick += (_, _) => { UpdateTransferStats(); FlushLog(); };
        FillChoices();
        viewLocalization.Apply(root);
        UiTypography.Attach(root);
        appearance = new Appearance(root, this, settings);
        BuildLanguageMenu();
        notifications = new JobNotifications(() => DispatcherQueue.TryEnqueue(() =>
        {
            if (closing)
            {
                return;
            }
            if (AppWindow.Presenter is Microsoft.UI.Windowing.OverlappedPresenter presenter &&
                presenter.State == Microsoft.UI.Windowing.OverlappedPresenterState.Minimized)
            {
                presenter.Restore();
            }
            Activate();
        }));
        Control<CheckBox>("NotifyWhenFinished").IsChecked = settings.NotifyWhenFinished;
        Closed += (_, _) => notifications.Close();
        if (StartupMode.IsNotificationActivation(Environment.GetCommandLineArgs()[1..]))
        {
            notifications.TryRegister(out _);
        }
        WireEvents();
        InitializeFrameControls();
        ResolveTools();
        Refresh();
        root.Loaded += async (_, _) =>
        {
            dialogOpen = true;
            try
            {
                if (!await LegalNotices.RequireAcceptanceAsync(root, settings, Relocalize))
                {
                    Close();
                    return;
                }
            }
            finally { dialogOpen = false; }
            Reflow(root.ActualWidth);
            if (string.IsNullOrEmpty(ffmpeg) || string.IsNullOrEmpty(ffprobe))
            {
                await ConfigureToolsAsync();
            }

            var args = Environment.GetCommandLineArgs();
            if (args.Length > 1 && File.Exists(args[1]))
            {
                Input.Text = args[1];
                await InspectAsync();
            }
        };
        AppWindow.Closing += OnClosing;
    }

    [DllImport("user32.dll")] private static extern uint GetDpiForWindow(IntPtr hwnd);
    private T Control<T>(string name) where T : class
    {
        if (!controls.TryGetValue(name, out var control))
        {
            controls[name] = control = root.FindName(name);
        }

        return (T)control;
    }
    private static string Value(ComboBox combo) => (combo.SelectedItem as ComboBoxItem)?.Tag as string ?? "";
    private static void Select(ComboBox combo, string? key)
    {
        var item = combo.Items.OfType<ComboBoxItem>().FirstOrDefault(i => (string)i.Tag == (key ?? ""));
        combo.SelectedItem = item ?? combo.Items[0];
    }
    private void Choices(ComboBox combo, params (string key, string name)[] choices)
    {
        foreach (var (key, name) in choices)
        {
            var item = new ComboBoxItem { Tag = key, Content = name };
            combo.Items.Add(item);
            choiceLabels.Add((item, name));
        }
        combo.SelectedIndex = 0;
    }
    private void WireEvents()
    {
        var copyLog = Control<Button>("CopyLog");
        // Native button handling runs first; stop pointer input here before it
        // reaches the enclosing Expander header's toggle button.
        copyLog.AddHandler(UIElement.PointerPressedEvent,
            new PointerEventHandler((_, args) => args.Handled = true), true);
        copyLog.AddHandler(UIElement.PointerReleasedEvent,
            new PointerEventHandler((_, args) => args.Handled = true), true);
        copyLog.Click += async (_, _) => await CopyLogAsync();
        Control<CheckBox>("NotifyWhenFinished").Click += (_, _) =>
        {
            settings.NotifyWhenFinished = Control<CheckBox>("NotifyWhenFinished").IsChecked == true;
            settings.Save();
        };
        Control<Button>("ChooseInput").Click += async (_, _) => await ChooseInputAsync();
        Control<Button>("ChooseOutput").Click += async (_, _) => await ChooseOutputAsync();
        Control<Button>("ToolsButton").Click += async (_, _) => await ConfigureToolsAsync();
        Control<Button>("AboutButton").Click += async (_, _) =>
        {
            if (dialogOpen || closing)
            {
                return;
            }

            dialogOpen = true;
            try
            {
                await AboutDialog.ShowAsync(root);
            }
            finally { dialogOpen = false; }
        };
        Control<Button>("Start").Click += async (_, _) => await StartAsync();
        Control<Button>("Cancel").Click += async (_, _) => { if (activeJob is not null) { Control<Button>("Cancel").IsEnabled = false; SetStatus("正在取消…"); await activeJob.CancelAsync(); } };
        Control<Button>("OpenFolder").Click += (_, _) => { if (File.Exists(completedOutput)) { Process.Start(new ProcessStartInfo("explorer.exe", Path.GetDirectoryName(completedOutput)!) { UseShellExecute = true }); } };
        Input.KeyDown += async (_, e) => { if (e.Key == Windows.System.VirtualKey.Enter) { await InspectAsync(); } };
        Input.LostFocus += async (_, _) => { if (File.Exists(Input.Text.Trim('"', ' '))) { await InspectAsync(); } };
        Input.TextChanged += (_, _) =>
        {
            if (activeJob is null && !probing && media is not null && !Input.Text.Trim('"', ' ').Equals(MediaText("path"), StringComparison.OrdinalIgnoreCase))
            {
                ClearMedia();
            }
        };
        foreach (var name in new[] { "Variant", "Container", "Sampling", "ColorRange", "Profile", "DynamicMetadata" })
        {
            Control<ComboBox>(name).SelectionChanged += (_, _) => Refresh();
        }

        Purpose.SelectionChanged += (_, _) =>
        {
            if (updating)
            {
                return;
            }

            updating = true;
            try
            {
                ApplyPurposeDefaults();
            }
            finally { updating = false; }
            Refresh();
        };
        Codec.SelectionChanged += (_, e) =>
        {
            if (updating)
            {
                return;
            }

            updating = true;
            try
            {
                if (audioUsesDefault)
                {
                    ApplyDefaultAudio();
                }

                if (Value(Codec) == "av1" || Value(Purpose) == "share" || e.RemovedItems.OfType<ComboBoxItem>().Any(item => (string)item.Tag == "av1"))
                {
                    Select(Sampling, DefaultSampling());
                }
            }
            finally { updating = false; }
            Refresh();
        };
        Audio.SelectionChanged += (_, _) => { if (!updating) { audioUsesDefault = false; Refresh(); } };
        BitDepth.SelectionChanged += (_, _) => { if (!updating) { depthUsesDefault = false; Refresh(); } };
        Range.SelectionChanged += (_, _) => Refresh();
        InputKind.SelectionChanged += (_, _) =>
        {
            if (updating)
            {
                return;
            }

            updating = true;
            try
            {
                Range.SelectedIndex = Kind is "hlg" or "pq" or "log" ? 1 : 0;
                depthUsesDefault = true;
            }
            finally { updating = false; }
            Refresh();
            QueueFramePreview();
        };
        Transform.SelectionChanged += (_, _) => Refresh();
        Control<CheckBox>("CpuDecode").Click += (_, _) =>
        {
            if (!updating && activeJob is null)
            {
                preferSoftwareDecode = Control<CheckBox>("CpuDecode").IsChecked == true;
                Refresh();
            }
        };
        var card = Control<Border>("SourceCard");
        card.DragOver += (_, e) => { if (activeJob is null && e.DataView.Contains(StandardDataFormats.StorageItems)) { e.AcceptedOperation = DataPackageOperation.Copy; } };
        card.Drop += async (_, e) =>
        {
            if (activeJob is not null || !e.DataView.Contains(StandardDataFormats.StorageItems))
            {
                return;
            }

            var files = await e.DataView.GetStorageItemsAsync();
            if (files.Count == 1)
            {
                Input.Text = files[0].Path;
                await InspectAsync();
            }
        };
        root.SizeChanged += (_, e) => Reflow(e.NewSize.Width);
    }
    /// <summary>Repositions the same controls at two width breakpoints, without rebuilding job state.</summary>
    /// <remarks>Auto rows stretch the paired cards together in two columns; a single column uses each card's natural height.</remarks>
    private (bool Narrow, bool Compact, bool Photo, bool Preview)? lastLayout;
    private void Reflow(double width)
    {
        var narrow = width < 960;
        var compact = width < 760;
        bool preview = sourceMedia is not null && (PhotoMode || Control<Microsoft.UI.Xaml.Controls.Primitives.ToggleButton>("ShowPreview").IsChecked == true);
        var layout = (narrow, compact, PhotoMode, preview);
        if (lastLayout == layout)
        {
            return;
        }
        lastLayout = layout;
        // Width changes within a breakpoint are handled by star columns. Only
        // cross the WinRT/layout boundary when the structure actually changes.
        var grid = Control<Grid>("SettingsGrid");
        int settingsRow = preview ? 2 : 1;
        var rows = PhotoMode ? settingsRow + 1 : settingsRow + (narrow ? 3 : 2);
        while (grid.RowDefinitions.Count < rows)
        {
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        }

        while (grid.RowDefinitions.Count > rows)
        {
            grid.RowDefinitions.RemoveAt(grid.RowDefinitions.Count - 1);
        }

        grid.ColumnDefinitions[0].MinWidth = narrow ? 0 : 380;
        grid.ColumnDefinitions[1].MinWidth = narrow ? 0 : 300;
        grid.ColumnDefinitions[0].Width = new GridLength(1, GridUnitType.Star);
        grid.ColumnDefinitions[1].Width = narrow ? new GridLength(0) : new GridLength(1, GridUnitType.Star);
        grid.ColumnSpacing = narrow ? 0 : 24;
        Grid.SetColumnSpan(Control<Border>("SourceCard"), narrow ? 1 : 2);
        Grid.SetColumnSpan(Control<Border>("PreviewCard"), narrow ? 1 : 2);
        Grid.SetColumnSpan(Control<Border>("PhotoSettings"), narrow ? 1 : 2);
        Grid.SetRow(Control<Border>("PhotoSettings"), settingsRow);
        Grid.SetRow(Control<Border>("OutputCard"), settingsRow);
        var color = Control<Border>("ColorSettings");
        Grid.SetColumn(color, narrow ? 0 : 1);
        Grid.SetRow(color, settingsRow + (narrow ? 1 : 0));
        var format = Control<Border>("FormatCard");
        Grid.SetRow(format, settingsRow + (narrow ? 2 : 1));
        Grid.SetColumnSpan(format, narrow ? 1 : 2);

        var frameNumber = Control<TextBox>("FrameNumber");
        Grid.SetColumn(frameNumber, compact ? 0 : 2);
        Grid.SetRow(frameNumber, compact ? 1 : 0);
        Grid.SetColumnSpan(frameNumber, compact ? 3 : 1);
        frameNumber.HorizontalAlignment = compact ? HorizontalAlignment.Stretch : HorizontalAlignment.Right;
        foreach (var (gridName, fieldsName) in new[] { ("VideoTrimFields", "TrimEndFields"), ("MotionFields", "MotionEndFields") })
        {
            var rangeGrid = Control<Grid>(gridName);
            rangeGrid.ColumnDefinitions[1].Width = compact ? new GridLength(0) : new GridLength(1, GridUnitType.Star);
            rangeGrid.ColumnSpacing = compact ? 0 : 16;
            var last = Control<StackPanel>(fieldsName);
            Grid.SetColumn(last, compact ? 0 : 1);
            Grid.SetRow(last, compact ? 1 : 0);
        }
        var fields = Control<Grid>("FormatFields");
        var fieldRows = compact ? 4 : 2;
        while (fields.RowDefinitions.Count < fieldRows)
        {
            fields.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        }

        while (fields.RowDefinitions.Count > fieldRows)
        {
            fields.RowDefinitions.RemoveAt(fields.RowDefinitions.Count - 1);
        }

        fields.ColumnDefinitions[1].Width = compact ? new GridLength(0) : new GridLength(1, GridUnitType.Star);
        fields.ColumnSpacing = compact ? 0 : 24;
        Grid.SetColumn(Audio, compact ? 0 : 1);
        Grid.SetRow(Audio, compact ? 2 : 0);
        var audioNote = Control<TextBlock>("AudioNote");
        Grid.SetColumn(audioNote, compact ? 0 : 1);
        Grid.SetRow(audioNote, compact ? 3 : 1);
        var details = Control<Grid>("SourceDetails");
        details.ColumnDefinitions[2].Width = new GridLength(compact ? 0 : 240);
        details.RowSpacing = compact ? 16 : 0;
        Grid.SetColumnSpan(Control<StackPanel>("SourceInfo"), compact ? 2 : 1);
        var interpretation = Control<StackPanel>("SourceInterpretation");
        Grid.SetColumn(interpretation, compact ? 1 : 2);
        Grid.SetRow(interpretation, compact ? 1 : 0);
        Grid.SetColumnSpan(interpretation, compact ? 2 : 1);

        var buttons = Control<StackPanel>("ActionButtons");
        Grid.SetColumn(buttons, compact ? 0 : 1);
        Grid.SetRow(buttons, compact ? 1 : 0);
        Grid.SetColumnSpan(buttons, compact ? 2 : 1);
        Grid.SetColumnSpan(Control<TextBlock>("Status"), compact ? 2 : 1);
        Control<Grid>("ActionGrid").RowSpacing = compact ? 12 : 0;
    }
    private string MediaText(string key, string fallback = "") => media.Text(key, fallback);
    private void SetStatus(string text) => Control<TextBlock>("Status").Text = L.T(text);
    private MediaTools ToolsObject() => new(ffmpeg, ffprobe);
    private void ResolveTools()
    {
        ffmpeg = ConversionClient.FindTool("ffmpeg", settings.Ffmpeg);
        ffprobe = ConversionClient.FindTool("ffprobe", settings.Ffprobe);
    }
    // Hold the window open until cancellation has joined FFmpeg and cleaned
    // the temporary output. Close() is called by the completed job's finally block.
    private async void OnClosing(Microsoft.UI.Windowing.AppWindow sender, Microsoft.UI.Windowing.AppWindowClosingEventArgs args)
    {
        if (closing || activeJob is null)
        {
            bool joinFrames = !closing && (previewSession is not null || timelineTask is not null || !previewTeardown.IsCompleted);
            // Invalidate queued probe results before releasing the window.
            // A cancelled probe can still have a previously queued media event.
            closing = true;
            generation++;
            foreach (var probe in probes)
            {
                probe.Dispose();
            }
            if (joinFrames)
            {
                args.Cancel = true;
                await StopFrameWorkAsync();
                // Complete this closing event before issuing the final close.
                DispatcherQueue.TryEnqueue(Close);
            }
            return;
        }
        args.Cancel = true;
        if (dialogOpen)
        {
            return;
        }

        dialogOpen = true;
        try
        {
            var result = await UiTypography.Prepare(new ContentDialog { Title = L.T("取消转换并退出？"), Content = L.T("本次临时输出将清理，原片保留。"), PrimaryButtonText = L.T("取消转换并退出"), CloseButtonText = L.T("继续转换"), XamlRoot = root.XamlRoot, RequestedTheme = root.RequestedTheme }).ShowAsync();
            if (result == ContentDialogResult.Primary)
            {
                closing = true;
                if (activeJob is not null)
                {
                    await activeJob.CancelAsync();
                }
                else
                {
                    Close();
                }
            }
        }
        finally { dialogOpen = false; }
    }
}
