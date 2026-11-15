// Cross-implementation parity check for the shared core.
// The Windows baseline (Core/OutputPolicy.cs) and the Rust core must answer the
// same questions identically for the same source description. This harness is
// the P2 exit condition: it compares every format rule without a GPU, without a
// sample file and without an FFmpeg run.
//
// Exit codes: 0 = parity, 1 = mismatch, 3 = native library not built (not run).
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

Console.WriteLine($"core {LumaCoreNative.CoreVersion} (ABI {LumaCoreNative.AbiVersion}), {LumaCoreNative.Operations.Count} operations");

int failures = 0;
int comparisons = 0;

foreach (Fixture fixture in Fixture.All)
{
    MediaInfo media = fixture.ToMedia();
    JsonObject source = fixture.ToNativeSource();
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
                Compare(label, "sampling", OutputPolicy.SourceSampling(media), described?["source"]?["sampling"]?.ToString(), ref failures, ref comparisons);
                Compare(label, "depth", Number(OutputPolicy.SourceDepth(media)), described?["source"]?["depth"]?.ToString(), ref failures, ref comparisons);
                Compare(label, "range", OutputPolicy.SourceRange(media), described?["source"]?["range"]?.ToString(), ref failures, ref comparisons);
                Compare(label, "samplings", string.Join(',', OutputPolicy.SamplingChoices(media)), Join(described?["choices"]?["samplings"]), ref failures, ref comparisons);
                Compare(label, "depths", string.Join(',', OutputPolicy.DepthChoices(media, purpose, codec, output)), Join(described?["choices"]?["depths"]), ref failures, ref comparisons);
                Compare(label, "default-sampling", OutputPolicy.DefaultSampling(media, purpose, codec), described?["choices"]?["sampling"]?.ToString(), ref failures, ref comparisons);
                Compare(label, "default-depth", OutputPolicy.DefaultDepth(media, purpose, codec, output) is int depth ? Number(depth) : "null", described?["choices"]?["depth"]?.ToString() ?? "null", ref failures, ref comparisons);
            }
        }
    }
}

// Resolve parity needs the preset assets for the managed baseline; the resolver
// itself is already exercised above.
string? assets = FindAssets();
if (assets is null)
{
    Console.Error.WriteLine("Assets/recommended-presets.json not found; resolve parity was not run");
    return failures == 0 ? 0 : 1;
}

Transcoder transcoder = new(assets);
(string Purpose, string Codec, string Output)[] cases =
[
    ("share", "hevc", "sdr"),
    ("share", "av1", "sdr"),
    ("archive", "hevc", "sdr"),
];
foreach (Fixture fixture in Fixture.All)
{
    MediaInfo media = fixture.ToMedia();
    foreach ((string purpose, string codec, string output) in cases)
    {
        string expected;
        try
        {
            EncodeOptions resolved = transcoder.ResolveFormat(new EncodeOptions(purpose, codec, output), media);
            expected = $"{resolved.Sampling}|{resolved.ColorRange}|{resolved.BitDepth}";
        }
        catch (InvalidDataException)
        {
            expected = "error";
        }

        JsonObject request = new()
        {
            ["source"] = fixture.ToNativeSource(),
            ["options"] = new JsonObject { ["purpose"] = purpose, ["codec"] = codec, ["output"] = output }
        };
        string actual = LumaCoreNative.TryCall("policy.resolve", request, out JsonNode? data, out _)
            ? $"{data?["sampling"]}|{data?["color_range"]}|{data?["bit_depth"]}"
            : "error";
        Compare($"{fixture.Name}/{purpose}/{codec}/{output}", "resolve", expected, actual, ref failures, ref comparisons);
    }
}

Console.WriteLine($"{comparisons} comparisons, {failures} mismatches");
return failures == 0 ? 0 : 1;

static string Number(int value) => value.ToString(CultureInfo.InvariantCulture);

static string Join(JsonNode? node) => node is JsonArray values
    ? string.Join(',', values.Select(value => value?.ToString() ?? ""))
    : "";

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

/// <summary>One synthetic source description expressed for both implementations.</summary>
internal sealed record Fixture(
    string Name, string Pixel, string Kind, string? Profile, string? Range,
    int[] Components, bool Rgb = false, bool Gray = false, bool Bayer = false, int? ChromaW = null, int? ChromaH = null)
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
        new("yuvj-8", "yuvj420p", "sdr", null, null, [8, 8, 8], ChromaW: 1, ChromaH: 1)
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
        JsonObject pixel = new() { ["flags"] = flags, ["components"] = components };
        if (ChromaW is int width)
        {
            pixel["log2_chroma_w"] = width.ToString(CultureInfo.InvariantCulture);
        }

        if (ChromaH is int height)
        {
            pixel["log2_chroma_h"] = height.ToString(CultureInfo.InvariantCulture);
        }

        JsonObject video = new() { ["pix_fmt"] = Pixel };
        if (Range is not null)
        {
            video["color_range"] = Range;
        }

        if (Gray)
        {
            video["_monochrome"] = "1";
        }

        return new MediaInfo("fixture.mov", video, [], new JsonObject(), Kind, Profile, "", 0, new JsonArray(), pixel);
    }

    public JsonObject ToNativeSource() => new()
    {
        ["pixel_format"] = Pixel,
        ["kind"] = Kind,
        ["profile"] = Profile is null ? null : JsonValue.Create(Profile),
        ["color_range"] = Range is null ? null : JsonValue.Create(Range),
        ["components"] = new JsonArray(Components.Select(depth => (JsonNode?)JsonValue.Create(depth)).ToArray()),
        ["rgb"] = Rgb,
        ["bayer"] = Bayer,
        ["monochrome"] = Gray,
        ["alpha"] = Alpha,
        ["log2_chroma_w"] = ChromaW is int w ? JsonValue.Create(w) : null,
        ["log2_chroma_h"] = ChromaH is int h ? JsonValue.Create(h) : null
    };
}
