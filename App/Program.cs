using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using VideoConverter.Native;

namespace VideoConverter;

/// <summary>Selects the headless entry before initializing WinUI, COM wrappers or GUI resources.</summary>
internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        if (JitProcessHost.RestartIfNeeded(args) is { } exitCode)
        {
            return exitCode;
        }
        // Resolve output formats through the shared core when its native library is
        // present; the managed rules stay authoritative otherwise. LUMARECODE_CORE
        // pins the choice for support and for the CI fallback check.
        string coreChoice = Environment.GetEnvironmentVariable("LUMARECODE_CORE") ?? "auto";
        bool shared = !coreChoice.Equals("managed", StringComparison.OrdinalIgnoreCase)
            && SharedCoreBridge.Install();
        if (!shared)
        {
            SharedCoreBridge.Uninstall();
        }

        if (coreChoice.Equals("shared", StringComparison.OrdinalIgnoreCase) && !shared)
        {
            Console.Error.WriteLine("core: shared requested but unavailable");
            return 4;
        }
        if (Environment.GetEnvironmentVariable("LUMARECODE_DIAGNOSTICS") == "1")
        {
            // Support and CI use this line to prove which rules answered; it stays
            // silent unless the variable is set, so normal output is unchanged.
            Console.Error.WriteLine(shared
                ? $"core: shared {SharedCoreBridge.Version}"
                : "core: managed");
        }

        if (StartupMode.IsCommandLine(args))
        {
            return CommandLineHost.RunAsync(args[0] == "--cli" ? args[1..] : args).GetAwaiter().GetResult();
        }

        RunGui();
        return 0;
    }
    private static void RunGui()
    {
        JobNotifications.ConfigureIdentity();
        WinRT.ComWrappersSupport.InitializeComWrappers();
        // Stable WinUI 2.6 optimizations are opt-in for template compatibility.
        foreach (var change in new[] {
            Microsoft.UI.Xaml.Settings.XamlChangeId.IconNoGridOptimization,
            Microsoft.UI.Xaml.Settings.XamlChangeId.OptimizeApplyStyles,
            Microsoft.UI.Xaml.Settings.XamlChangeId.DefaultStyleOptimizations,
            Microsoft.UI.Xaml.Settings.XamlChangeId.DeferContextFlyoutInit })
        {
            Microsoft.UI.Xaml.Settings.XamlOptionalChanges.EnableChange(change);
        }

        Application.Start(parameters =>
        {
            SynchronizationContext.SetSynchronizationContext(new DispatcherQueueSynchronizationContext(DispatcherQueue.GetForCurrentThread()));
            try
            {
                _ = new ConverterApp();
            }
            catch (Exception error)
            {
                Directory.CreateDirectory(UserSettings.Folder);
                File.WriteAllText(Path.Combine(UserSettings.Folder, "error.log"), error.ToString());
                throw;
            }
        });
    }
}

public sealed partial class ConverterApp : Application
{
    private Window? window;
    public ConverterApp()
    {
        InitializeComponent();
        UnhandledException += (_, e) =>
        {
            var folder = UserSettings.Folder;
            Directory.CreateDirectory(folder);
            File.WriteAllText(Path.Combine(folder, "error.log"), e.Exception.ToString());
        };
    }
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        L.Configure(UserSettings.Load().Language);
        window = new MainWindow();
        window.Activate();
    }
}
