// Cross-implementation parity check for the shared core.
// The Windows baseline (Core/OutputPolicy.cs and Transcoder) and the Rust core
// must answer the same questions identically for the same probed source. This
// harness is the P2 exit condition: it compares every format rule without a GPU,
// without a sample file and without an FFmpeg run.
//
// Exit codes: 0 = parity, 1 = mismatch, 3 = native library or assets missing.
using System.Globalization;
using System.Text.Json.Nodes;
using VideoConverter.Core;
using VideoConverter.Native;

if (!LumaCoreNative.IsAvailable)
{
    Console.Error.WriteLine("native core unavailable; parity checks were not run");
    Console.Error.WriteLine(LumaCoreNative.LoadError);
    return 3;
}

string? assets = FindAssets();
if (assets is null)
{
    Console.Error.WriteLine("Assets/recommended-presets.json not found; parity checks were not run");
    return 3;
}

Console.WriteLine($"core {LumaCoreNative.CoreVersion} (ABI {LumaCoreNative.AbiVersion}), {LumaCoreNative.Operations.Count} operations");

int failures = 0;
int comparisons = 0;
Transcoder transcoder = new(assets);

(string Purpose, string Codec, string Output, string Transform, string? Profile)[] validateCases =
[
    ("share", "hevc", "sdr", "none", null),
    ("archive", "hevc", "sdr", "none", null),
    ("share", "av1", "sdr", "none", null),
    ("archive", "hevc", "hdr", "none", null),
    ("share", "hevc", "sdr", "log-hdr", "pocket3"),
    ("share", "hevc", "sdr", "hdr-sdr", null),
];

foreach (Fixture fixture in Fixture.All)
{
    MediaInfo media = fixture.ToMedia();
    JsonObject source = ProbeAdapter.Describe(media);

    foreach (string purpose in new[] { "share", "archive" })
    {
        foreach (string codec in new[] { "hevc", "av1", "avc" })
        {
            foreach (string output in new[] { "sdr", "hdr" })
            {
                JsonObject request = new()
                {
                    ["source"] = source.DeepClone(),
                    ["options"] = new JsonObject { ["purpose"] = purpose, ["codec"] = codec, ["output"] = output }
                };
                JsonNode? described = LumaCoreNative.Call("policy.describe", request);
                string label = $"{fixture.Name}/{purpose}/{codec}/{output}";
                Compare(label, "sampling", OutputPolicy.SourceSampling(media), described?["source"]?["sampling"]?.ToString() ?? "", ref failures, ref comparisons);
                Compare(label, "depth", Number(OutputPolicy.SourceDepth(media)), described?["source"]?["depth"]?.ToString() ?? "", ref failures, ref comparisons);
                Compare(label, "range", OutputPolicy.SourceRange(media), described?["source"]?["range"]?.ToString() ?? "", ref failures, ref comparisons);
                Compare(label, "samplings", string.Join(',', OutputPolicy.SamplingChoices(media)), Join(described?["choices"]?["samplings"]), ref failures, ref comparisons);
                Compare(label, "depths", string.Join(',', OutputPolicy.DepthChoices(media, purpose, codec, output)), Join(described?["choices"]?["depths"]), ref failures, ref comparisons);
                Compare(label, "default-sampling", OutputPolicy.DefaultSampling(media, purpose, codec), described?["choices"]?["sampling"]?.ToString() ?? "", ref failures, ref comparisons);
                Compare(label, "default-depth", OutputPolicy.DefaultDepth(media, purpose, codec, output) is int depth ? Number(depth) : "null", described?["choices"]?["depth"]?.ToString() ?? "null", ref failures, ref comparisons);
            }
        }
    }

    foreach ((string purpose, string codec, string output, string transform, string? profile) in validateCases)
    {
        EncodeOptions options = new(purpose, codec, output, transform, profile);
        string label = $"{fixture.Name}/{purpose}/{codec}/{output}/{transform}";
        Compare(label, "validate", ManagedFormat(transcoder, options, media), NativeFormat(source, options), ref failures, ref comparisons);
    }
}

// FLAC eligibility and the archive request that depends on it.
foreach (Fixture fixture in Fixture.AudioCases)
{
    MediaInfo media = fixture.ToMedia();
    JsonObject source = ProbeAdapter.Describe(media);
    bool managed = OutputPolicy.FlacEligibility(media).Eligible;
    bool native = LumaCoreNative.Call("policy.describe", new JsonObject { ["source"] = source.DeepClone() })
        ?["audio"]?["flac"]?.GetValue<bool>() ?? false;

    EncodeOptions archive = new("archive", "hevc", "sdr", Audio: "flac", Container: "mkv");
    bool managedRequest = !ManagedFormat(transcoder, archive, media).Equals("error", StringComparison.Ordinal);
    bool nativeRequest = !NativeFormat(source, archive).Equals("error", StringComparison.Ordinal);

    Compare(fixture.Name, "flac-eligibility", managed ? "true" : "false", native ? "true" : "false", ref failures, ref comparisons);
    Compare(fixture.Name, "flac-request", managedRequest ? "true" : "false", nativeRequest ? "true" : "false", ref failures, ref comparisons);
}

Console.WriteLine($"{comparisons} comparisons, {failures} mismatches");
return failures == 0 ? 0 : 1;

static string Number(int value) => value.ToString(CultureInfo.InvariantCulture);

static string Join(JsonNode? node) => node is JsonArray values
    ? string.Join(',', values.Select(value => value?.ToString() ?? ""))
    : "";

/// <summary>Resolves and validates with the Windows baseline; "error" means it rejected the request.</summary>
static string ManagedFormat(Transcoder transcoder, EncodeOptions options, MediaInfo media)
{
    try
    {
        EncodeOptions resolved = transcoder.ResolveFormat(options, media);
        transcoder.Validate(options, media);
        return $"{resolved.Sampling}|{resolved.ColorRange}|{resolved.BitDepth}";
    }
    catch (InvalidDataException)
    {
        return "error";
    }
}

static string NativeFormat(JsonObject source, EncodeOptions options)
{
    JsonObject request = new()
    {
        ["source"] = source.DeepClone(),
        ["options"] = options.ToJson()
    };
    return LumaCoreNative.TryCall("policy.validate", request, out JsonNode? data, out _)
        ? $"{data?["sampling"]}|{data?["color_range"]}|{data?["bit_depth"]}"
        : "error";
}

static void Compare(string fixture, string rule, string expected, string actual, ref int failures, ref int comparisons)
{
    comparisons++;
    if (string.Equals(expected, actual, StringComparison.Ordinal))
    {
        return;
    }

    failures++;
    Console.Error.WriteLine($"MISMATCH {fixture} {rule}: managed={expected} native={actual}");
}

static string? FindAssets()
{
    DirectoryInfo? directory = new(AppContext.BaseDirectory);
    while (directory is not null)
    {
        string candidate = Path.Combine(directory.FullName, "Assets", "recommended-presets.json");
        if (File.Exists(candidate))
        {
            return Path.Combine(directory.FullName, "Assets");
        }

        directory = directory.Parent;
    }

    return null;
}

/// <summary>One declared audio track, expressed the way ffprobe reports it.</summary>
internal sealed record AudioSpec(string Codec, string SampleFormat, int Depth, int Channels, string Layout);

/// <summary>One synthetic source description expressed for both implementations.</summary>
/// <remarks>
/// The managed side receives the probe snapshot shape and the native side receives
/// <see cref="ProbeAdapter"/> output, so a mapping mistake fails the same comparison
/// a rule mistake does.
/// </remarks>
internal sealed record Fixture(
    string Name,
    string Pixel,
    string Kind,
    string? Profile,
    string? Range,
    int[] Components,
    bool Rgb = false,
    bool Gray = false,
    bool Bayer = false,
    int? ChromaW = null,
    int? ChromaH = null,
    bool Hdr10Plus = false,
    AudioSpec[]? Audio = null)
{
    public static Fixture[] All =>
    [
        new("420-8", "yuv420p", "sdr", null, null, [8, 8, 8], ChromaW: 1, ChromaH: 1),
        new("420-10", "yuv420p10le", "hlg", null, null, [10, 10, 10], ChromaW: 1, ChromaH: 1),
        new("420-10-full", "yuv420p10le", "sdr", null, "pc", [10, 10, 10], ChromaW: 1, ChromaH: 1),
        new("422-10", "yuv422p10le", "sdr", null, null, [10, 10, 10], ChromaW: 1, ChromaH: 0),
        new("444-12", "yuv444p12le", "pq", null, null, [12, 12, 12], ChromaW: 0, ChromaH: 0),
        new("rgb-8", "rgb24", "sdr", null, null, [8, 8, 8], Rgb: true),
        new("gray-10", "gray10le", "sdr", null, null, [10], Gray: true),
        new("alpha-10", "yuva420p10le", "sdr", null, null, [10, 10, 10, 10], ChromaW: 1, ChromaH: 1),
        new("bayer-16", "bayer_rggb16le", "raw", null, null, [16, 16, 16, 16], Bayer: true),
        new("milog-10", "yuv420p10le", "log", "milog", null, [10, 10, 10], ChromaW: 1, ChromaH: 1),
        new("pocket3-10", "yuv420p10le", "log", "pocket3", null, [10, 10, 10], ChromaW: 1, ChromaH: 1),
        new("hdr10plus-10", "yuv420p10le", "hlg", null, null, [10, 10, 10], ChromaW: 1, ChromaH: 1, Hdr10Plus: true),
        new("yuvj-8", "yuvj420p", "sdr", null, null, [8, 8, 8], ChromaW: 1, ChromaH: 1),
    ];

    /// <summary>Sources with audio tracks, used only for the FLAC decisions.</summary>
    public static Fixture[] AudioCases =>
    [
        new("pcm24-stereo", "yuv420p", "sdr", null, null, [8, 8, 8], ChromaW: 1, ChromaH: 1,
            Audio: [new AudioSpec("pcm_s24le", "s32", 24, 2, "stereo")]),
        new("pcm16-51", "yuv420p", "sdr", null, null, [8, 8, 8], ChromaW: 1, ChromaH: 1,
            Audio: [new AudioSpec("pcm_s16le", "s16", 16, 6, "5.1")]),
        new("float-stereo", "yuv420p", "sdr", null, null, [8, 8, 8], ChromaW: 1, ChromaH: 1,
            Audio: [new AudioSpec("pcm_f32le", "flt", 32, 2, "stereo")]),
    ];

    private bool Alpha => Components.Length == 4 && !Rgb && !Bayer;

    public MediaInfo ToMedia()
    {
        JsonObject flags = new();
        if (Rgb)
        {
            flags["rgb"] = "1";
        }

        if (Bayer)
        {
            flags["bayer"] = "1";
        }

        if (Alpha)
        {
            flags["alpha"] = "1";
        }

        JsonArray components = new(Components.Select(depth => (JsonNode?)new JsonObject
        {
            ["bit_depth"] = depth.ToString(CultureInfo.InvariantCulture)
        }).ToArray());
        JsonObject pixel = new()
        {
            ["flags"] = flags,
            ["components"] = components
        };
        if (ChromaW is int width)
        {
            pixel["log2_chroma_w"] = width.ToString(CultureInfo.InvariantCulture);
        }

        if (ChromaH is int height)
        {
            pixel["log2_chroma_h"] = height.ToString(CultureInfo.InvariantCulture);
        }

        JsonObject video = new()
        {
            ["pix_fmt"] = Pixel
        };
        if (Range is not null)
        {
            video["color_range"] = Range;
        }

        if (Gray)
        {
            video["_monochrome"] = "1";
        }

        if (Hdr10Plus)
        {
            video["side_data_list"] = new JsonArray(new JsonObject { ["side_data_type"] = "HDR10+ Metadata" });
        }

        JsonArray streams = new();
        foreach (AudioSpec track in Audio ?? [])
        {
            streams.Add(new JsonObject
            {
                ["codec_type"] = "audio",
                ["codec_name"] = track.Codec,
                ["profile"] = "",
                ["sample_fmt"] = track.SampleFormat,
                ["bits_per_raw_sample"] = track.Depth.ToString(CultureInfo.InvariantCulture),
                ["bits_per_sample"] = track.Depth.ToString(CultureInfo.InvariantCulture),
                ["channels"] = track.Channels.ToString(CultureInfo.InvariantCulture),
                ["channel_layout"] = track.Layout
            });
        }

        return new MediaInfo("fixture.mov", video, [], new JsonObject(), Kind, Profile, "", 0, streams, pixel);
    }
}
