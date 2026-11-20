using System.Diagnostics;
using System.Globalization;
using System.Reflection;
using System.Text.Json;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace VideoConverter;

/// <summary>Shared source-key translation and current GUI culture configuration.</summary>
public static class L
{
    private static readonly TranslationCatalog catalog = new(Load());
    private static readonly string systemLanguage = ReadSystemLanguage();
    public static string Code { get; private set; } = "en";
    public static string SystemLanguage => systemLanguage;

    public static void Configure(string selection)
    {
        Code = LanguagePolicy.Resolve(selection, systemLanguage);
        var culture = CultureInfo.GetCultureInfo(LanguagePolicy.Culture(Code));
        CultureInfo.CurrentUICulture = CultureInfo.CurrentCulture = culture;
        CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.DefaultThreadCurrentCulture = culture;
        try
        {
            Microsoft.Windows.Globalization.ApplicationLanguages.PrimaryLanguageOverride = LanguagePolicy.XamlLanguage(Code);
        }
        catch (System.Runtime.InteropServices.COMException) { }
        UiTypography.Configure();
    }
    private static string ReadSystemLanguage()
    {
        try
        {
            return Windows.System.UserProfile.GlobalizationPreferences.Languages.FirstOrDefault() ?? CultureInfo.InstalledUICulture.Name;
        }
        catch { return CultureInfo.InstalledUICulture.Name; }
    }
    private static Dictionary<string, Dictionary<string, string>> Load()
    {
        var result = new Dictionary<string, Dictionary<string, string>>();
        foreach (var code in new[] { "en", "ja", "es" })
        {
            using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream($"LumaRecode.Strings.{code}.json")
                ?? throw new InvalidOperationException($"The embedded {code} language resource is missing.");
            result[code] = JsonSerializer.Deserialize(stream, AppJsonContext.Default.DictionaryStringString)!;
        }
        return result;
    }
    public static string T(string? text) => catalog.Translate(text, Code);
    public static string Translate(string? text, string code) => catalog.Translate(text, code);
}

/// <summary>Applies the build-generated list of localizable XAML properties without runtime reflection.</summary>
public sealed class ViewLocalization
{
    public void Apply(FrameworkElement root)
    {
        root.Language = LanguagePolicy.XamlLanguage(L.Code);
        foreach (var (name, property, original) in ViewText.Entries)
        {
            if (root.FindName(name) is not DependencyObject control)
            {
                continue;
            }

            var translated = L.T(original);
            if (property == "AutomationProperties.Name")
            {
                AutomationProperties.SetName(control, translated);
            }
            else if (property == "ToolTipService.ToolTip")
            {
                ToolTipService.SetToolTip(control, translated);
            }
            else
            {
                switch (property, control)
                {
                    case ("Text", TextBlock text):
                        text.Text = translated;
                        break;
                    case ("Text", SelectorBarItem page):
                        page.Text = translated;
                        break;
                    case ("Content", ContentControl content):
                        content.Content = translated;
                        break;
                    case ("Header", ComboBox combo):
                        combo.Header = translated;
                        break;
                    case ("Header", NumberBox number):
                        number.Header = translated;
                        break;
                    case ("Header", Slider slider):
                        slider.Header = translated;
                        break;
                    case ("Header", RadioButtons choices):
                        choices.Header = translated;
                        break;
                    case ("Header", Expander expander):
                        expander.Header = translated;
                        break;
                    // A text field carries a header too, and the frame box became one.
                    case ("Header", TextBox box):
                        box.Header = translated;
                        break;
                    case ("Header", PasswordBox secret):
                        secret.Header = translated;
                        break;
                    case ("PlaceholderText", TextBox input):
                        input.PlaceholderText = translated;
                        break;
                    default:
                        // Skipping keeps a renamed or retyped control from bricking the app;
                        // the pair simply keeps its markup text until someone updates the table.
                        Debug.WriteLine($"Unsupported localization target: {name}.{property}");
                        break;
                }
                if (property == "Header" && control is Expander)
                {
                    AutomationProperties.SetName(control, translated);
                }
            }
        }
    }
}
