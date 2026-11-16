using System.Reflection;
using VideoConverter.Core;

using VideoConverter.Native;

namespace VideoConverter;

/// <summary>Adapts desktop preferences, component terms and console cancellation to the UI-independent CLI.</summary>
internal static class CommandLineHost
{
    public static async Task<int> RunAsync(string[] args)
    {
        ConsoleConnection.Connect();
        var noSettings = args.TakeWhile(a => a != "--").Contains("--no-settings");
        var settings = noSettings ? new UserSettings() : UserSettings.Load();
        string selection = settings.Language;
        for (int i = 0; i < args.Length && args[i] != "--"; i++)
        {
            if (args[i] is "--lang" or "--language" && i + 1 < args.Length)
            {
                selection = args[++i];
            }
            else if (args[i].StartsWith("--lang=", StringComparison.Ordinal) || args[i].StartsWith("--language=", StringComparison.Ordinal))
            {
                selection = args[i][(args[i].IndexOf('=') + 1)..];
            }
        }
        var language = LanguagePolicy.Resolve(selection, L.SystemLanguage);
        var version = typeof(CommandLineHost).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion.Split('+')[0] ?? "unknown";
        using var cancellation = new CancellationTokenSource();
        ConsoleCancelEventHandler cancel = (_, e) => { e.Cancel = true; cancellation.Cancel(); };
        Console.CancelKeyPress += cancel;
        try
        {
            var context = new CliContext(version, LegalNotices.Folder, settings.AcceptedThirdPartyTerms == LegalNotices.Revision,
                new MediaTools(settings.Ffmpeg, settings.Ffprobe), text => L.Translate(text, language), language,
                cache => CacheBridge.Plan(cache));
            var code = await new NativeCli(ConversionClient.Engine, context).RunAsync(args, cancellation.Token);
            if (!noSettings && code == 0 && args.TakeWhile(a => a != "--").Contains("--accept-terms") && settings.AcceptedThirdPartyTerms != LegalNotices.Revision)
            {
                var current = UserSettings.Load();
                current.AcceptedThirdPartyTerms = LegalNotices.Revision;
                current.Save();
            }
            return code;
        }
        catch (Exception e) when (e is IOException or InvalidDataException or InvalidOperationException or System.Text.Json.JsonException or UnauthorizedAccessException)
        {
            try
            {
                if (args.Contains("--json"))
                {
                    Console.Out.WriteLine(BackendJson.Write(new System.Text.Json.Nodes.JsonObject
                    {
                        ["schema_version"] = 1,
                        ["event"] = "error",
                        ["data"] = new System.Text.Json.Nodes.JsonObject { ["code"] = "runtime_unavailable", ["message"] = e.Message }
                    }));
                }
                else
                {
                    Console.Error.WriteLine(e.Message);
                }
            }
            catch (IOException) { cancellation.Cancel(); return 130; }
            return 3;
        }
        finally { Console.CancelKeyPress -= cancel; }
    }
}
