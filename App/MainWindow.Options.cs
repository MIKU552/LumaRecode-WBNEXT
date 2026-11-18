using System.Globalization;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using VideoConverter.Core;

namespace VideoConverter;

public sealed partial class MainWindow
{
    private void FillChoices()
    {
        Choices(InputKind, ("auto", L.T("自动识别")), ("sdr", L.T("手动指定 · SDR")), ("hlg", L.T("手动指定 · HLG HDR")), ("pq", L.T("手动指定 · PQ HDR")), ("log", L.T("手动指定 · Log")));
        Choices(Purpose, ("share", L.T("快速分享 · Intel 硬件")), ("archive", L.T("软件归档 · CPU")));
        Choices(Codec, ("hevc", "HEVC"), ("av1", "AV1"), ("avc", "H.264 / AVC"));
        Choices(Variant, ("standard", L.T("标准实测档")), ("fast", L.T("AV1 更快备用档")));
        Choices(Container, ("auto", L.T("自动 · 按用途选择")), ("mp4", "MP4"), ("mkv", "MKV"));
        Choices(Transform, ("none", L.T("不套用 · 保留 / 标准化")), ("log-hdr", L.T("Log → HDR（HLG）")), ("log-sdr", "Log → SDR"), ("hdr-sdr", "HDR → SDR"));
        Choices(Profile, ("", L.T("请选择 Log 曲线…")), ("pocket3", "DJI Pocket 3 D-Log M"), ("samsung", "Samsung Log"), ("milog", L.T("小米 Mi-Log")), ("applelog", "Apple Log"), ("applelog2", "Apple Log 2"));
        Choices(DynamicMetadata, ("preserve", L.T("保留动态 HDR")), ("standard", L.T("标准 HDR")));
        Choices(Sampling, ("source", L.T("保持原采样")), ("420", "4:2:0"), ("422", "4:2:2"), ("444", "4:4:4"), ("gray", L.T("灰度")));
        Choices(BitDepth, ("8", L.T("8 位")), ("10", L.T("10 位")), ("12", L.T("12 位")));
        Choices(ColorRange, ("source", L.T("保持原范围")), ("limited", L.T("Limited · 有限范围")), ("full", L.T("Full · 全范围")));
        Choices(Audio, ("copy", L.T("复制原音轨")), ("aac", "AAC · 256 kbps"), ("opus", "Opus · 192 kbps"), ("flac", L.T("FLAC · 无损")));
        Select(Purpose, PlatformDefaults.Purpose);
        ApplyPurposeDefaults();
    }
    private void ApplyDefaultAudio()
    {
        var audio = encodingConfig.GetProperty("audio");
        Select(Audio, (Value(Purpose) == "archive" ? audio.GetProperty("archive_default") : audio.GetProperty("share_default").GetProperty(Value(Codec))).GetString());
    }
    private void ApplyPurposeDefaults()
    {
        var archive = Value(Purpose) == "archive";
        Select(Sampling, DefaultSampling());
        Select(ColorRange, archive ? "source" : "limited");
        audioUsesDefault = true;
        ApplyDefaultAudio();
        depthUsesDefault = true;
    }
    private string DefaultSampling() => Value(Purpose) == "archive" && Value(Codec) != "av1" ? MediaText("archive_sampling", "source") : "420";
    private static JsonElement LoadEncodingConfig()
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("VideoConverter.EncodingPresets.json")!;
        using var document = JsonDocument.Parse(stream);
        return document.RootElement.Clone();
    }
    private JsonElement PresetConfig() => Value(Variant) == "fast"
        ? encodingConfig.GetProperty("archive_fast_av1")
        : encodingConfig.GetProperty(Value(Purpose)).GetProperty(Value(Codec));
    private string DescribePreset()
    {
        var preset = PresetConfig();
        var encoder = preset.GetProperty("encoder").GetString() switch
        {
            "libx265" => "x265",
            "libx264" => "x264",
            "libsvtav1" => "SVT-AV1",
            var name => name
        };
        var share = Value(Purpose) == "share";
        var result = new List<string> { encoder ?? "", share ? $"TU{preset.GetProperty("preset")} / ICQ{preset.GetProperty("quality")}" : $"{preset.GetProperty("preset")} / CRF{preset.GetProperty("quality")}" };
        if (share)
        {
            result.Add("low_power=1");
            result.Add($"async_depth={preset.GetProperty("async_depth")}");
        }
        else
        {
            if (Value(Codec) == "hevc")
            {
                result.Add("pools=16");
            }

            foreach (var key in new[] { "tune", "lp" })
            {
                if (preset.TryGetProperty(key, out var item))
                {
                    result.Add(key + "=" + item);
                }
            }

            if (preset.TryGetProperty("encoder_parameters", out var extra))
            {
                foreach (var item in extra.EnumerateObject())
                {
                    result.Add(item.Name + "=" + item.Value);
                }
            }
        }
        return string.Join(" · ", result);
    }
    private bool MediaFlag(string key) => media.Flag(key);
    private string SelectedSampling()
    {
        var selected = Value(Sampling);
        return selected == "source" ? MediaText("source_sampling", "420") : selected;
    }
    private string SelectedColorRange()
    {
        var selected = Value(ColorRange);
        if (selected != "source")
        {
            return selected;
        }

        return SourceColorRange();
    }
    private string SourceColorRange() => MediaFlag("range_inferred") && Value(Profile) == "milog" && Value(Transform).StartsWith("log-") ? "full" : MediaText("source_color_range", "limited");
    private string[] SourceSamplingChoices() => media?["allowed_sampling"] is JsonArray values
        ? values.Select(value => value?.ToString() ?? "").ToArray() : ["420"];
    private int[] AllowedDepths() => OutputPolicy.DepthChoices(SourceDepth, Value(Purpose), Value(Codec), Hdr ? "hdr" : "sdr");
    private void UpdateDepthChoices()
    {
        var allowed = AllowedDepths();
        foreach (var item in BitDepth.Items.OfType<ComboBoxItem>())
        {
            item.IsEnabled = allowed.Contains(int.Parse((string)item.Tag));
        }

        if (allowed.Length == 0)
        {
            BitDepth.SelectedIndex = -1;
            return;
        }
        if (depthUsesDefault || !allowed.Contains(Depth))
        {
            Select(BitDepth, (Value(Purpose) == "archive" ? allowed.Max() : Hdr ? 10 : 8).ToString(CultureInfo.InvariantCulture));
        }
    }
    /// <summary>Applies compatible choices, output naming and start-button availability as one guarded update.</summary>
    private void Refresh()
    {
        if (updating)
        {
            return;
        }

        updating = true;
        try
        {
            if (Kind == "sdr" && Hdr)
            {
                Range.SelectedIndex = 0;
            }

            Control<RadioButton>("HdrChoice").IsEnabled = Kind is "hlg" or "pq" or "log" or "raw";
            if (Hdr && Value(Codec) == "avc")
            {
                Select(Codec, "hevc");
            }

            if (MediaFlag("dynamic_hdr"))
            {
                Select(InputKind, "auto");
            }
            bool dynamic = MediaFlag("dynamic_hdr") && Hdr;
            DynamicMetadata.Visibility = dynamic ? Visibility.Visible : Visibility.Collapsed;
            Control<TextBlock>("DynamicNote").Visibility = dynamic ? Visibility.Visible : Visibility.Collapsed;
            bool preserve = dynamic && Value(DynamicMetadata) == "preserve";
            if (preserve)
            {
                Select(Purpose, "archive");
                if (Value(Codec) == "avc" || MediaFlag("hdr10plus") || media?["dolby_vision"].Integer("dv_profile") == 5)
                {
                    Select(Codec, "hevc");
                }
                Select(Sampling, "420");
                Select(BitDepth, "10");
                Select(ColorRange, "source");
            }
            Purpose.Items.OfType<ComboBoxItem>().First().IsEnabled = !preserve;
            Control<TextBlock>("DynamicNote").Text = L.T(preserve ? "软件编码保留逐帧动态元数据。" : "输出标准 HLG / PQ。");
            UpdateDepthChoices();
            foreach (var item in Codec.Items.OfType<ComboBoxItem>())
            {
                item.IsEnabled = (!Hdr || (string)item.Tag != "avc") && (!preserve || (string)item.Tag == "hevc" || (string)item.Tag == "av1" && !MediaFlag("hdr10plus") && media?["dolby_vision"].Integer("dv_profile") != 5);
            }

            var fullSampling = Value(Purpose) == "archive" && Value(Codec) != "av1";
            var sourceChoices = SourceSamplingChoices();
            foreach (var item in Sampling.Items.OfType<ComboBoxItem>())
            {
                var key = (string)item.Tag;
                item.IsEnabled = key == "source" ? fullSampling && DefaultSampling() == "source" && sourceChoices.Contains(MediaText("source_sampling", "420"))
                    : sourceChoices.Contains(key) && (fullSampling || key == "420");
            }
            if (Sampling.SelectedItem is ComboBoxItem selectedSampling && !selectedSampling.IsEnabled)
            {
                Select(Sampling, DefaultSampling());
            }

            if (preserve)
            {
                Select(BitDepth, "10");
                foreach (var item in BitDepth.Items.OfType<ComboBoxItem>())
                {
                    item.IsEnabled = (string)item.Tag == "10";
                }
                foreach (var item in Sampling.Items.OfType<ComboBoxItem>())
                {
                    item.IsEnabled = (string)item.Tag == "420";
                }
                foreach (var item in ColorRange.Items.OfType<ComboBoxItem>())
                {
                    item.IsEnabled = (string)item.Tag == "source";
                }
            }
            else
            {
                foreach (var item in ColorRange.Items.OfType<ComboBoxItem>())
                {
                    item.IsEnabled = true;
                }
            }
            var flacAllowed = MediaFlag("flac_eligible");
            Audio.Items.OfType<ComboBoxItem>().Single(item => (string)item.Tag == "flac").IsEnabled = flacAllowed;
            ((ComboBoxItem)Container.Items[1]).IsEnabled = Value(Audio) != "flac";
            if (Value(Audio) == "flac" && Value(Container) == "mp4")
            {
                Select(Container, "mkv");
            }

            Control<TextBlock>("AudioNote").Text = Value(Audio) == "flac" ? L.T("保留每条音轨的采样率、声道与整数精度。")
                : media is null ? L.T("FLAC 支持整数 PCM 和可确认的无损音轨。")
                : flacAllowed ? L.T("此素材可转为 FLAC。") : L.T(MediaText("flac_reason"));
            var fast = Value(Purpose) == "archive" && Value(Codec) == "av1";
            Variant.Visibility = fast ? Visibility.Visible : Visibility.Collapsed;
            if (!fast)
            {
                Select(Variant, "standard");
            }

            string[] allowed = Kind switch
            {
                "hlg" or "pq" => Hdr ? ["none"] : ["hdr-sdr"],
                "log" => Hdr ? ["log-hdr"] : ["log-sdr"],
                _ => ["none"]
            };
            foreach (var item in Transform.Items.OfType<ComboBoxItem>())
            {
                item.IsEnabled = allowed.Contains((string)item.Tag);
            }

            if (!allowed.Contains(Value(Transform)))
            {
                Select(Transform, allowed[0]);
            }

            Profile.IsEnabled = Value(Transform).StartsWith("log-");
            Profile.Visibility = Profile.IsEnabled ? Visibility.Visible : Visibility.Collapsed;
            var chosenSampling = SelectedSampling();
            var sampling = chosenSampling switch
            {
                "420" => "4:2:0",
                "422" => "4:2:2",
                "444" => "4:4:4",
                "gray" => L.T("灰度"),
                _ => L.T("采样待选择")
            };
            var dataRange = SelectedColorRange() == "full" ? "Full" : "Limited";
            var curve = Kind == "pq" && !Profile.IsEnabled ? "PQ" : "HLG";
            Control<TextBlock>("TargetNote").Text = (Hdr ? L.T($"BT.2020 {curve} · {Depth} 位 · {sampling}") : L.T($"Rec.709 · {Depth} 位 · {sampling}")) + " · " + dataRange;
            var formatNotes = new List<string>();
            if (MediaFlag("pixel_rgb") && Kind != "raw")
            {
                formatNotes.Add(L.T("RGB 输入转换为 YCbCr。"));
            }

            if (MediaFlag("pixel_alpha"))
            {
                formatNotes.Add(L.T("输出为不带透明通道的视频。"));
            }

            if (MediaFlag("range_inferred"))
            {
                formatNotes.Add(L.T($"原片未标明范围，按 {(SourceColorRange() == "full" ? "Full" : "Limited")} 处理。"));
            }

            Control<TextBlock>("FormatNote").Text = string.Join("\n", formatNotes);
            Control<TextBlock>("FormatNote").Visibility = formatNotes.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
            var codecName = Value(Codec) switch
            {
                "av1" => "AV1",
                "avc" => "H.264",
                _ => "HEVC"
            };
            var backend = EncoderBackends.Label(EncoderBackends.For(PresetConfig().GetProperty("encoder").GetString() ?? ""));
            Control<TextBlock>("PresetName").Text = $"{(Hdr ? "HDR" : "SDR")} · {codecName} · {(Value(Purpose) == "share" ? L.T("快速分享") : L.T("软件归档"))} · {backend}";
            var quality = DescribePreset();
            Control<TextBlock>("PresetDetail").Text = L.T($"{quality}\n{Depth} 位 {sampling} · {Suffix[1..].ToUpperInvariant()}") + " · " + dataRange;
            Control<TextBlock>("ContainerNote").Text = Value(Container) == "auto"
                ? Value(Audio) == "flac" ? L.T("自动 → MKV。FLAC 音轨默认使用 MKV 封装。") : L.T($"自动 → {Suffix[1..].ToUpperInvariant()}。快速分享默认 MP4，软件归档默认 MKV。")
                : Suffix == ".mp4" ? L.T("MP4。复制原音轨时，会在编码前检测封装兼容性。") : L.T("MKV。封装独立于编码质量档和动态范围。");
            Control<TextBlock>("TransformNote").Text = Kind == "raw" ? L.T("使用相机白平衡和色彩矩阵显影 RAW。") : Value(Transform) switch
            {
                "log-hdr" => L.T($"使用对应相机的曲线，转换为 {Depth} 位 HLG。"),
                "log-sdr" => L.T("解码对应 Log，并将高光映射到 Rec.709。"),
                "hdr-sdr" => L.T("按 HLG / PQ 原片做色调映射，输出 Rec.709。"),
                _ => L.T("保留原片动态范围，按目标格式标准化色彩。")
            };
            var warning = "";
            if (string.IsNullOrEmpty(ffmpeg) || string.IsNullOrEmpty(ffprobe))
            {
                warning = L.T("未找到 FFmpeg / ffprobe，请设置工具路径。");
            }
            else if (MediaFlag("dynamic_hdr") && !MediaFlag("dynamic_hdr_supported"))
            {
                warning = L.T("此 Dolby Vision 兼容层暂不受支持。");
            }
            else if (media?["raw"] is JsonObject rawVersion && !rawVersion.Flag("decoder_version_supported"))
            {
                warning = L.T("ProRes RAW 显影需要 FFmpeg 9 或更新版本。");
            }
            else if (media?["raw"] is JsonObject raw && !raw.Flag("decoder_supported"))
            {
                warning = L.T("当前 FFmpeg 无法解码此 ProRes RAW。请先用支持该相机的 RAW 软件显影为 ProRes 或 HEVC。");
            }
            else if (media is not null && Profile.IsEnabled && Value(Profile) == "")
            {
                warning = L.T("请确认相机的 Log 曲线。");
            }
            else if (media is not null && Kind == "unknown" && !Profile.IsEnabled)
            {
                warning = L.T("没有可靠的曲线标记，请确认输入解释。");
            }
            else if (media is not null && Profile.IsEnabled && SourceDepth < 10)
            {
                warning = L.T("Log 转换需要对应相机的高位深原片。");
            }
            else if (media is not null && AllowedDepths().Length == 0)
            {
                warning = L.T("HDR 输出需要至少 10 位的源素材，请选择 SDR 或高位深原片。");
            }
            else if (media is not null && !sourceChoices.Contains(chosenSampling))
            {
                warning = L.T("输出采样须在原片采样上限内选择。");
            }
            else if (media is not null && !new[] { "420", "422", "444", "gray" }.Contains(chosenSampling))
            {
                warning = L.T("当前编码器无法保持原采样，请手动选择 4:2:0、4:2:2 或 4:4:4。");
            }
            else if (media is not null && chosenSampling != "420" && !fullSampling)
            {
                warning = L.T("保持此采样需要软件 HEVC 或 H.264；也可手动选择 4:2:0。");
            }
            else if (MediaFlag("pixel_alpha") && Value(Sampling) == "source")
            {
                warning = L.T("素材含透明通道。请选择具体采样格式，输出为不带透明通道的视频。");
            }
            else if (Value(Audio) == "flac" && !flacAllowed)
            {
                warning = L.T(MediaText("flac_reason"));
            }

            var notice = Control<InfoBar>("Notice");
            notice.Message = warning;
            notice.IsOpen = warning != "";
            Control<TextBlock>("ChooseInputLabel").Text = L.T(media is null ? "选择视频" : "更换视频");
            Control<Button>("ChooseInput").Style = (Style)root.Resources[media is null ? "AccentActionButtonStyle" : "ActionButtonStyle"];
            Control<Button>("Start").IsEnabled = media is not null && !probing && activeJob is null && warning == "";
            // Regenerate only a name still owned by the application. A user
            // path survives option changes, except for a changed container suffix.
            if (!PhotoMode && media is not null && (Output.Text == "" || Output.Text == automaticOutput))
            {
                var source = MediaText("path");
                automaticOutput = Path.Combine(Path.GetDirectoryName(source)!, $"{Path.GetFileNameWithoutExtension(source)}_{(Hdr ? "HDR" : "SDR")}_{Value(Codec)}_{Value(Purpose)}{Suffix}");
                Output.Text = automaticOutput;
            }
            else if (media is not null && previousSuffix != Suffix && new[] { ".mp4", ".mkv", ".jpg", ".avif", ".png" }.Contains(Path.GetExtension(Output.Text).ToLowerInvariant()) && Output.Text != automaticOutput)
            {
                Output.Text = Path.ChangeExtension(Output.Text, Suffix);
            }

            UpdateFramePresentation();
            UpdateFrameOutput();
            previousSuffix = Suffix;
            UpdateEditingEnabled(media is not null && !probing && activeJob is null);
            Input.IsEnabled = activeJob is null && !probing;
            Control<Button>("ChooseInput").IsEnabled = Input.IsEnabled;
        }
        finally { updating = false; }
    }
}
