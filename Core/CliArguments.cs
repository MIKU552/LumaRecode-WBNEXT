using System.Text.Json.Nodes;

namespace VideoConverter.Core;

/// <summary>A stable machine-readable error code paired with the documented CLI exit status.</summary>
public sealed class CliException(string code, string message, int status = 2) : Exception(message)
{
    public string Code { get; } = code;
    public int Status { get; } = status;
}

/// <summary>Parses commands and options without shell reparsing or runtime reflection.</summary>
public sealed class CliArguments
{
    public static readonly string[] Commands = ["probe", "presets", "profiles", "doctor", "cache", "license", "convert", "plan", "frames", "image"];
    public static readonly Dictionary<string, string[]> Choices = new()
    {
        ["purpose"] = ["share", "archive"],
        ["codec"] = ["hevc", "av1", "avc"],
        ["range"] = ["auto", "sdr", "hdr"],
        ["transform"] = ["auto", "none", "log-hdr", "log-sdr", "hdr-sdr"],
        ["profile"] = ["auto", "pocket3", "samsung", "milog", "applelog", "applelog2"],
        ["sampling"] = ["auto", "source", "420", "422", "444", "gray"],
        ["variant"] = ["standard", "fast"],
        ["audio"] = ["auto", "copy", "aac", "opus", "flac"],
        ["input_kind"] = ["auto", "sdr", "hlg", "pq", "log", "raw"],
        ["color_range"] = ["auto", "source", "limited", "full"],
        ["bit_depth"] = ["auto", "8", "10", "12"],
        ["container"] = ["auto", "mp4", "mkv"],
        ["dynamic_hdr"] = ["preserve", "standard"]
    };
    private static readonly HashSet<string> booleans = ["json", "accept_terms", "no_settings", "text", "recursive", "overwrite", "skip_existing", "continue_on_error", "dry_run", "verbose", "motion", "purge"];
    private static readonly HashSet<string> commonValues = ["lang", "ffmpeg", "ffprobe"];
    private static readonly HashSet<string> taskValues = ["output", "output_dir", "config", "preset", "duration", "log", "start_frame", "end_frame", "start_time", "end_time", "frame", "image_format", "quality", "motion_start_frame", "motion_end_frame", "motion_size"];
    public string? Command
    {
        get; private set;
    }
    public bool Help
    {
        get; private set;
    }
    public bool Version
    {
        get; private set;
    }
    public List<string> Inputs { get; } = [];
    public Dictionary<string, string> Values { get; } = new();
    public HashSet<string> Flags { get; } = [];
    public bool Has(string flag) => Flags.Contains(flag);
    public string? Get(string name) => Values.GetValueOrDefault(name);
    public double? Duration => Get("duration") is { } value ? JsonFields.Number(value, double.NaN) : null;

    /// <summary>Accepts common options on either side of a command and honors "--" for literal filenames.</summary>
    public static CliArguments Parse(string[] words)
    {
        var parsed = new CliArguments();
        bool literal = false;
        for (int i = 0; i < words.Length; i++)
        {
            var word = words[i];
            if (!literal && word == "--")
            {
                literal = true;
                continue;
            }
            if (!literal && word == "--cli")
            {
                continue;
            }

            if (!literal && word is "-h" or "--help")
            {
                parsed.Help = true;
                break;
            }
            if (!literal && word == "--version")
            {
                parsed.Version = true;
                break;
            }
            if (!literal && word.StartsWith('-'))
            {
                var split = word.Split('=', 2);
                var name = split[0] switch
                {
                    "-o" => "output",
                    "-r" => "recursive",
                    "--dynamic-range" => "range",
                    "--language" => "lang",
                    _ => split[0].TrimStart('-').Replace('-', '_')
                };
                if (name is "cpu_decode" or "no_cpu_decode")
                {
                    if (split.Length > 1)
                    {
                        throw new CliException("invalid_arguments", "Unexpected value for " + split[0]);
                    }

                    parsed.Values["cpu_decode"] = name == "cpu_decode" ? "true" : "false";
                    continue;
                }
                if (booleans.Contains(name))
                {
                    if (split.Length > 1)
                    {
                        throw new CliException("invalid_arguments", "Unexpected value for " + split[0]);
                    }

                    parsed.Flags.Add(name);
                    continue;
                }
                if (!commonValues.Contains(name) && !taskValues.Contains(name) && !Choices.ContainsKey(name))
                {
                    throw new CliException("invalid_arguments", "Unknown option: " + word);
                }

                string value;
                if (split.Length > 1)
                {
                    value = split[1];
                }
                else
                {
                    // A following option is not a value. Negative numbers are
                    // retained for --duration so its normal validation can reject them.
                    if (++i >= words.Length || words[i].StartsWith('-') && !(name == "duration" && double.TryParse(words[i], System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out _)))
                    {
                        throw new CliException("invalid_arguments", "Missing value: " + word);
                    }

                    value = words[i];
                }
                if (Choices.TryGetValue(name, out var choices) && !choices.Contains(value))
                {
                    throw new CliException("invalid_arguments", $"Invalid {name}: {value}");
                }

                if (name == "lang" && value is not ("system" or "zh" or "en" or "ja" or "es"))
                {
                    throw new CliException("invalid_arguments", "Invalid language: " + value);
                }

                parsed.Values[name] = value;
            }
            else if (parsed.Command is null)
            {
                if (!Commands.Contains(word))
                {
                    throw new CliException("invalid_arguments", "Unknown command: " + word);
                }

                parsed.Command = word;
            }
            else
            {
                parsed.Inputs.Add(word);
            }
        }
        if (!parsed.Help && !parsed.Version)
        {
            if (parsed.Command is "probe" or "frames" or "image" && parsed.Inputs.Count != 1 || parsed.Command is "convert" or "plan" && parsed.Inputs.Count == 0
                || parsed.Command is not ("probe" or "convert" or "plan" or "frames" or "image") && parsed.Inputs.Count > 0)
            {
                throw new CliException("invalid_arguments", "Invalid number of input paths.");
            }

            if (parsed.Command is not ("convert" or "plan" or "image") && (parsed.Values.Keys.Any(n => taskValues.Contains(n) || Choices.ContainsKey(n) || n == "cpu_decode")
                || parsed.Flags.Any(n => n is "recursive" or "overwrite" or "skip_existing" or "continue_on_error" or "dry_run" or "verbose" or "motion")))
            {
                throw new CliException("invalid_arguments", "Encoding options require convert or plan.");
            }

            string[] imageOnly = ["frame", "image_format", "quality", "motion_start_frame", "motion_end_frame", "motion_size"];
            if (parsed.Command != "image" && (parsed.Has("motion") || imageOnly.Any(parsed.Values.ContainsKey)))
            {
                throw new CliException("invalid_arguments", "Image options require image.");
            }
            if (parsed.Command == "image" && (parsed.Values.Keys.Any(key => !commonValues.Contains(key) && key is not ("output" or "log" or "frame" or "image_format" or "quality" or "motion_start_frame" or "motion_end_frame" or "motion_size" or "input_kind" or "profile" or "purpose"))
                || parsed.Flags.Any(key => key is "recursive" or "continue_on_error")))
            {
                throw new CliException("invalid_arguments", "Unsupported image option.");
            }

            if (parsed.Has("text") && parsed.Command != "license")
            {
                throw new CliException("invalid_arguments", "--text requires license.");
            }

            if (parsed.Has("overwrite") && parsed.Has("skip_existing"))
            {
                throw new CliException("invalid_arguments", "--overwrite and --skip-existing are mutually exclusive.");
            }
        }
        return parsed;
    }

    /// <summary>Merges preset defaults, config-file values and explicit CLI values, in that order.</summary>
    /// <remarks>Only documented encoding keys are accepted; bit_depth also permits JSON integer values.</remarks>
    public JsonObject EncodingValues(JsonArray presets)
    {
        JsonObject values = new();
        if (Get("config") is { } path)
        {
            try
            {
                values = JsonNode.Parse(File.ReadAllText(path)) as JsonObject ?? throw new InvalidDataException();
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException or System.Text.Json.JsonException or InvalidDataException) { throw new CliException("invalid_config", "配置须为仅包含受支持编码选项的 JSON 对象。"); }
            if (values.Any(p => !Choices.ContainsKey(p.Key) && p.Key is not ("preset" or "cpu_decode")))
            {
                throw new CliException("invalid_config", "配置须为仅包含受支持编码选项的 JSON 对象。");
            }
        }
        string? preset = Get("preset") ?? values["preset"]?.ToString();
        values.Remove("preset");
        var selected = presets.Objects().FirstOrDefault(p => p.Text("id") == preset);
        if (preset is not null && selected is null)
        {
            throw new CliException("invalid_config", $"未知预设：{preset}");
        }

        var result = selected?["options"]?.DeepClone().AsObject() ?? new();
        foreach (var p in values)
        {
            result[p.Key] = p.Value?.DeepClone();
        }

        foreach (var p in Values.Where(p => Choices.ContainsKey(p.Key) || p.Key == "cpu_decode"))
        {
            result[p.Key] = p.Key == "cpu_decode" ? JsonValue.Create(p.Value == "true") : JsonValue.Create(p.Value);
        }

        foreach (var p in Choices)
        {
            if (result.ContainsKey(p.Key) && (!p.Value.Contains(result.Text(p.Key)) || result[p.Key]?.GetValueKind() != System.Text.Json.JsonValueKind.String && !(p.Key == "bit_depth" && result[p.Key]?.GetValueKind() == System.Text.Json.JsonValueKind.Number)))
            {
                throw new CliException("invalid_config", $"无效的 {p.Key}：{result[p.Key]}");
            }
        }

        if (result.ContainsKey("cpu_decode") && result["cpu_decode"]?.GetValueKind() is not (System.Text.Json.JsonValueKind.True or System.Text.Json.JsonValueKind.False))
        {
            throw new CliException("invalid_config", "cpu_decode 必须为 true 或 false。");
        }

        return result;
    }
}
