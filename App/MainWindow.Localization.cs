using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;

namespace VideoConverter;

public sealed partial class MainWindow
{
    private void BuildLanguageMenu()
    {
        var menu = new MenuFlyout
        {
            Placement = FlyoutPlacementMode.BottomEdgeAlignedRight,
            ShouldConstrainToRootBounds = false
        };
        foreach (var (key, name) in new[] { ("system", L.T("跟随系统")) }.Concat(LanguagePolicy.Choices))
        {
            var code = key == "system" ? L.Code : key;
            var item = new RadioMenuFlyoutItem
            {
                Text = name,
                Tag = key,
                GroupName = "AppLanguage",
                IsChecked = settings.Language == key,
                FontFamily = UiTypography.Family(code),
                Language = LanguagePolicy.XamlLanguage(code)
            };
            item.Click += (_, _) =>
            {
                settings.Language = key;
                settings.Save();
                L.Configure(key);
                Relocalize();
            };
            menu.Items.Add(item);
            languageItems[key] = item;
        }
        Control<DropDownButton>("LanguageButton").Flyout = menu;
        UpdateLanguageMenu();
    }
    private void UpdateLanguageMenu()
    {
        foreach (var (key, item) in languageItems)
        {
            item.IsChecked = key == settings.Language;
        }

        languageItems["system"].Text = L.T("跟随系统");
        languageItems["system"].FontFamily = UiTypography.Family(L.Code);
        languageItems["system"].Language = LanguagePolicy.XamlLanguage(L.Code);
        var button = Control<DropDownButton>("LanguageButton");
        button.Content = LanguagePolicy.Choices.FirstOrDefault(choice => choice.Code == settings.Language).Name ?? L.T("语言 · 系统");

        // Size both surfaces from every label, including the menu's check column.
        double TextWidth(string text, FontFamily family, string language)
        {
            var label = new TextBlock
            {
                Text = text,
                FontFamily = family,
                FontSize = button.FontSize,
                FontWeight = button.FontWeight,
                Language = language
            };
            label.Measure(new Windows.Foundation.Size(double.PositiveInfinity, double.PositiveInfinity));
            return label.DesiredSize.Width;
        }
        var menuWidth = languageItems.Values.Max(item => TextWidth(item.Text, item.FontFamily, item.Language)) + 64;
        var buttonWidth = LanguagePolicy.Choices.Select(choice => choice.Name).Append(L.T("语言 · 系统"))
            .Max(text => TextWidth(text, UiTypography.Family(L.Code), root.Language)) + 48;
        var width = 4 * Math.Ceiling(Math.Max(128, Math.Max(menuWidth, buttonWidth)) / 4);
        button.Width = width;
        var presenter = (Style)XamlReader.Load("""
            <Style xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                   TargetType="MenuFlyoutPresenter" BasedOn="{StaticResource DefaultMenuFlyoutPresenterStyle}" />
            """);
        presenter.Setters.Add(new Setter(FrameworkElement.WidthProperty, width));
        presenter.Setters.Add(new Setter(FrameworkElement.MinWidthProperty, width));
        presenter.Setters.Add(new Setter(FrameworkElement.MaxWidthProperty, width));
        presenter.Setters.Add(new Setter(Microsoft.UI.Xaml.Controls.Control.FontFamilyProperty, UiTypography.Family(L.Code)));
        ((MenuFlyout)button.Flyout).MenuFlyoutPresenterStyle = presenter;
    }
    private void Relocalize()
    {
        // Keep controls and job state alive. Only replace presentation strings.
        var status = Control<TextBlock>("Status").Text;
        var source = new[] { "SourceSummary", "SourceEvidence", "SourceBadge" }.ToDictionary(name => name, name => Control<TextBlock>(name).Text);
        var cancelEnabled = Control<Button>("Cancel").IsEnabled;
        viewLocalization.Apply(root);
        Title = L.T("映码 · LumaRecode WBNEXT");
        updating = true;
        try
        {
            foreach (var (item, original) in choiceLabels)
            {
                item.Content = L.T(original);
            }

            foreach (var name in new[] { "InputKind", "Purpose", "Codec", "Variant", "Container", "Transform", "Profile", "Sampling", "BitDepth", "ColorRange", "Audio", "DynamicMetadata", "ImageFormat", "MotionEncoder", "MotionResolution" })
            {
                var combo = Control<ComboBox>(name);
                var selected = combo.SelectedIndex;
                // WinUI caches the selected label separately from the item's Content.
                combo.SelectedIndex = -1;
                combo.SelectedIndex = selected;
            }
        }
        finally { updating = false; }
        appearance.Relocalize();
        UpdateLanguageMenu();
        if (media is not null)
        {
            UpdateMediaPresentation();
        }
        else
        {
            foreach (var (name, text) in source)
            {
                Control<TextBlock>(name).Text = L.T(text);
            }
        }

        SetStatus(status);
        Refresh();
        UpdateTransferStats();
        UiTypography.Refresh(root);
        if (activeJob is not null)
        {
            SetBusy(true);
            Control<Button>("Cancel").IsEnabled = cancelEnabled;
        }
    }
}
