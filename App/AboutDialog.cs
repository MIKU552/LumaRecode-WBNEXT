using System.Reflection;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media.Imaging;
using VideoConverter.Core;

namespace VideoConverter;

/// <summary>Builds the on-demand identity, version and licensing dialog using the current language and typography.</summary>
internal static class AboutDialog
{
    public static async Task ShowAsync(FrameworkElement root)
    {
        var version = Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>()?
            .InformationalVersion.Split('+')[0] ?? "unknown";
        var panel = new StackPanel { Spacing = 20, MaxWidth = 520 };
        var identity = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 16 };
        identity.Children.Add(new Image { Source = new BitmapImage(new Uri("ms-appx:///Assets/LumaRecode.png")), Width = 64, Height = 64 });
        var name = new StackPanel { Spacing = 4, VerticalAlignment = VerticalAlignment.Center };
        name.Children.Add(new TextBlock { Text = L.T("映码 · LumaRecode"), FontSize = 24, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
        name.Children.Add(new TextBlock { Text = L.T("版本") + " " + version + " · core " + (SharedCore.CoreActive ? $"shared {SharedCore.CoreVersion}" : "managed"), FontSize = 13, IsTextSelectionEnabled = true });
        identity.Children.Add(name);
        panel.Children.Add(identity);
        static TextBlock Paragraph(string text) => new()
        {
            Text = L.T(text),
            TextWrapping = TextWrapping.Wrap,
            FontSize = 14
        };
        panel.Children.Add(Paragraph("面向 Windows 11 的视频转码与色彩转换工具，提供原生 WinUI 3 界面和完整命令行。"));
        var credits = new StackPanel { Spacing = 8 };
        credits.Children.Add(new TextBlock { Text = L.T("开发与核验"), FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
        credits.Children.Add(Paragraph("重构由 WorkBuddy（DeepSeek-V4.1-Flash-1017，极高推理，1M 上下文）协助完成，MIKU552 提出需求并进行实机核验。"));
        panel.Children.Add(credits);
        var licensing = new StackPanel { Spacing = 8 };
        licensing.Children.Add(new TextBlock { Text = L.T("许可与第三方组件"), FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
        licensing.Children.Add(Paragraph("项目自主内容采用 MIT License。第三方运行时、字体与 LUT 保留各自许可；FFmpeg 由用户提供。"));
        panel.Children.Add(licensing);
        var links = new StackPanel { Spacing = 4 };
        links.Children.Add(new HyperlinkButton
        {
            Content = L.T("GitHub 项目主页"),
            NavigateUri = new Uri("https://github.com/MIKU552/LumaRecode"),
            Padding = new Thickness(0, 6, 0, 6),
            HorizontalAlignment = HorizontalAlignment.Left
        });
        var licenses = new HyperlinkButton { Content = L.T("查看完整许可与第三方声明"), Padding = new Thickness(0, 6, 0, 6), HorizontalAlignment = HorizontalAlignment.Left };
        licenses.Click += (_, _) => LegalNotices.OpenFolder();
        links.Children.Add(licenses);
        panel.Children.Add(links);
        await UiTypography.Prepare(new ContentDialog
        {
            Title = L.T("关于"),
            CloseButtonText = L.T("关闭"),
            XamlRoot = root.XamlRoot,
            RequestedTheme = root.RequestedTheme,
            Language = root.Language,
            Content = new ScrollViewer
            {
                Content = panel,
                HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
                MaxHeight = Math.Clamp(root.ActualHeight - 220, 180, 560)
            }
        }).ShowAsync();
    }
}
