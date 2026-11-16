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

// Preset parity: the shared registry and the managed table must agree on the
// encoder, preset and quality for every published selector.
string document = File.ReadAllText(Path.Combine(assets, "recommended-presets.json"));
(string Purpose, string Codec, string Variant, string Selector)[] selectors =
[
    ("share", "avc", "standard", "share/avc"),
    ("share", "hevc", "standard", "share/hevc"),
    ("share", "av1", "standard", "share/av1"),
    ("archive", "avc", "standard", "archive/avc"),
    ("archive", "hevc", "standard", "archive/hevc"),
    ("archive", "av1", "standard", "archive/av1"),
    ("archive", "av1", "fast", "archive_fast_av1"),
];

foreach ((string purpose, string codec, string variant, string selector) in selectors)
{
    JsonObject request = new()
    {
        ["document"] = document,
        ["purpose"] = purpose,
        ["codec"] = codec,
        ["variant"] = variant
    };
    JsonNode? resolved = LumaCoreNative.Call("presets.resolve", request);
    JsonObject? row = selector == "archive_fast_av1"
        ? transcoder.Config["archive_fast_av1"] as JsonObject
        : transcoder.Config[purpose] is JsonObject table ? table[codec] as JsonObject : null;
    string label = $"preset/{selector}";
    Compare(label, "selector", selector, resolved?["selector"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(label, "encoder", row?["encoder"]?.ToString() ?? "", resolved?["encoder"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(label, "preset", row?["preset"]?.ToString() ?? "", resolved?["preset"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(label, "quality", row?["quality"]?.ToString() ?? "", resolved?["quality"]?.ToString() ?? "", ref failures, ref comparisons);
    string encoder = row?["encoder"]?.ToString() ?? "";
    JsonNode? backend = LumaCoreNative.Call("capability.backend", new JsonObject { ["encoder"] = encoder });
    Compare(label, "backend", EncoderBackends.For(encoder) ?? "", backend?["backend"]?.ToString() ?? "", ref failures, ref comparisons);
}

// Rejection paths the format matrix cannot reach: Log transforms, dynamic HDR and
// FLAC container rules. The replay below also compares the message, so a code that
// still falls back to the managed checks is proven to produce identical text.
(string Fixture, string Label, EncodeOptions Options)[] edgeCases =
[
    ("milog-10", "log-no-profile", new EncodeOptions("share", "hevc", "hdr", "log-hdr")),
    ("pocket3-10", "log-target-mismatch", new EncodeOptions("share", "hevc", "sdr", "log-hdr", "pocket3")),
    ("420-8", "log-requires-log", new EncodeOptions("share", "hevc", "hdr", "log-hdr", "pocket3")),
    ("420-8", "hdr-sdr-on-sdr", new EncodeOptions("share", "hevc", "sdr", "hdr-sdr")),
    ("420-8", "sdr-to-hdr", new EncodeOptions("share", "hevc", "hdr", "none")),
    ("420-10", "hdr-to-sdr-required", new EncodeOptions("share", "hevc", "sdr", "none")),
    ("hdr10plus-10", "dvhdr-share", new EncodeOptions("share", "hevc", "sdr", "none")),
    ("420-8", "flac-mp4", new EncodeOptions("archive", "hevc", "sdr", "none", null, "auto", "standard", "flac", "auto", false, "mp4")),
    ("420-8", "flac-no-audio", new EncodeOptions("archive", "hevc", "sdr", "none", null, "auto", "standard", "flac", "auto", false, "mkv")),
];

foreach ((string fixtureName, string label, EncodeOptions options) in edgeCases)
{
    MediaInfo media = Fixture.All.First(item => item.Name == fixtureName).ToMedia();
    Compare(label, "edge", ManagedFormat(transcoder, options, media), NativeFormat(ProbeAdapter.Describe(media), options), ref failures, ref comparisons);
}

// Probe parity on the documents CI captured from ffprobe: the shared summary and
// the managed classification must agree about the same file.
foreach (string name in new[] { "ffprobe-testsrc.json", "ffprobe-tenbit.json" })
{
    string path = Path.Combine(Path.GetDirectoryName(assets)!, "native", "crates", "luma-core", "tests", "fixtures", name);
    string doc = File.ReadAllText(path);
    JsonNode? summary = LumaCoreNative.Call("probe.summarize", new JsonObject { ["document"] = doc });
    JsonObject root = JsonNode.Parse(doc)!.AsObject();
    JsonObject video = root["streams"]!.AsArray().Objects().First(stream => stream.Text("codec_type") == "video");
    JsonObject tags = root["format"]?["tags"] as JsonObject ?? new JsonObject();
    string label = $"probe/{name}";
    Compare(label, "kind", MediaProbe.Classify(video, tags).Kind, summary?["kind"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(label, "width", video.Text("width"), summary?["width"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(label, "height", video.Text("height"), summary?["height"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(label, "depth", video.Text("bits_per_raw_sample"), summary?["depth"]?.ToString() ?? "", ref failures, ref comparisons);
    int audio = root["streams"]!.AsArray().Objects().Count(stream => stream.Text("codec_type") == "audio");
    Compare(label, "audio", $"{audio}", summary?["audio"]?.ToString() ?? "", ref failures, ref comparisons);
}

// Timeline parity: identical frames and ranges must produce the same window.
FramePoint[] timelinePoints = Enumerable.Range(0, 100)
    .Select(index => new FramePoint(index * 1000L, 1000L, index % 30 == 0))
    .ToArray();
FrameTimeline timeline = new(timelinePoints, 1, 1000, false);
FrameTimeline packetTimeline = new(timelinePoints, 1, 1000, true);
CompareTimeline(timeline, false, 0, null, "full");
CompareTimeline(timeline, false, 10, 40, "middle");
CompareTimeline(timeline, false, 99, null, "last");
CompareTimeline(packetTimeline, true, 10, 40, "packet/middle");
CompareTimeline(packetTimeline, true, 99, null, "packet/last");

bool nativeRejects = NativeRejects(5, 5);
bool managedRejects = ManagedRejects(5, 5);
Compare("timeline/reject", "empty", $"{managedRejects}", $"{nativeRejects}", ref failures, ref comparisons);
nativeRejects = NativeRejects(0, 101);
managedRejects = ManagedRejects(0, 101);
Compare("timeline/reject", "beyond_end", $"{managedRejects}", $"{nativeRejects}", ref failures, ref comparisons);

bool ManagedRejects(int start, int? end)
{
    try
    {
        timeline.Resolve(new FrameRange(start, end));
        return false;
    }
    catch (InvalidDataException)
    {
        return true;
    }
}

bool NativeRejects(int start, int? end)
{
    JsonArray frames = new(timelinePoints.Select(point => (JsonNode?)new JsonObject
    {
        ["pts"] = point.Pts,
        ["duration"] = point.Duration,
        ["key_frame"] = point.KeyFrame
    }).ToArray());
    JsonObject range = new();
    range["start"] = start;
    if (end is { } stop)
    {
        range["end"] = stop;
    }

    JsonObject request = new()
    {
        ["frames"] = frames,
        ["time_base"] = "1/1000",
        ["packet_indexed"] = false,
        ["range"] = range
    };
    try
    {
        LumaCoreNative.Call("timeline.resolve", request);
        return false;
    }
    catch (InvalidOperationException)
    {
        return true;
    }
}

void CompareTimeline(FrameTimeline source, bool packetIndexed, int start, int? end, string label)
{
    JsonArray frames = new(timelinePoints.Select(point => (JsonNode?)new JsonObject
    {
        ["pts"] = point.Pts,
        ["duration"] = point.Duration,
        ["key_frame"] = point.KeyFrame
    }).ToArray());
    JsonObject range = new();
    range["start"] = start;
    if (end is { } stop)
    {
        range["end"] = stop;
    }

    JsonNode? native = LumaCoreNative.Call("timeline.resolve", new JsonObject
    {
        ["frames"] = frames,
        ["time_base"] = "1/1000",
        ["packet_indexed"] = packetIndexed,
        ["range"] = range
    });
    FrameWindow window = source.Resolve(new FrameRange(start, end));
    string id = $"timeline/{label}";
    Compare(id, "start", $"{window.Start}", native?["window"]?["start"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(id, "end", $"{window.End}", native?["window"]?["end"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(id, "count", $"{window.Count}", native?["window"]?["count"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(id, "seek_frame", $"{window.SeekFrame}", native?["window"]?["seek_frame"]?.ToString() ?? "", ref failures, ref comparisons);
    Compare(id, "seek_seconds", Seconds(window.SeekSeconds), NativeSeconds(native?["window"], "seek_seconds"), ref failures, ref comparisons);
    Compare(id, "start_seconds", Seconds(window.StartSeconds), NativeSeconds(native?["window"], "start_seconds"), ref failures, ref comparisons);
    Compare(id, "end_seconds", Seconds(window.EndSeconds), NativeSeconds(native?["window"], "end_seconds"), ref failures, ref comparisons);
}

// Static capability matrix: each family's depth and sampling limits must match.
foreach (string family in new[] { "software", "qsv", "nvenc", "amf", "media_foundation", "videotoolbox" })
{
    foreach (string codec in new[] { "hevc", "av1", "avc" })
    {
        foreach (int depth in new[] { 8, 10, 12 })
        {
            foreach (string sampling in new[] { "420", "422", "444", "gray" })
            {
                JsonNode? native = LumaCoreNative.Call("capability.supports", new JsonObject
                {
                    ["backend"] = family,
                    ["codec"] = codec,
                    ["depth"] = depth,
                    ["sampling"] = sampling
                });
                bool expected = EncoderBackends.Supports(family, codec, depth, sampling);
                bool actual = native?["supported"]?.GetValue<bool>() ?? false;
                Compare($"support/{family}/{codec}/{depth}/{sampling}", "supported", $"{expected}", $"{actual}", ref failures, ref comparisons);
            }
        }
    }
}

// GOP parity: the interval handed to the encoder must be identical whether the rule
// runs in the core or in the managed fallback. The rates include the tie cases where
// a double lands exactly on .5 after the factor, which is where .NET's default
// rounding and the core's "halves away from zero" would diverge by one frame.
double[] gopRates =
    [23.976, 24, 25, 29.97, 30, 30.0625, 30.25, 50, 59.94, 60, 119.88, 120, 240, 1000];
(string Fixture, string Purpose, string Output, bool MotionPhoto)[] gopCases =
[
    ("420-8", "share", "sdr", false),
    ("420-8", "archive", "sdr", false),
    ("420-10", "share", "hdr", false),
    ("420-10", "archive", "hdr", false),
    ("420-8", "archive", "sdr", true),
];
List<(string Label, MediaInfo Media, EncodeOptions Options, string Managed)> gopChecks = new();
foreach ((string fixtureName, string purpose, string output, bool motionPhoto) in gopCases)
{
    MediaInfo fixture = Fixture.All.First(item => item.Name == fixtureName).ToMedia();
    EncodeOptions options = new(purpose, "hevc", output, "none");
    foreach (double rate in gopRates)
    {
        MediaInfo media = AtRate(fixture, rate);
        string text = rate.ToString("R", CultureInfo.InvariantCulture);
        string label = $"gop/{fixtureName}/{purpose}/{output}/{text}{(motionPhoto ? "/motion" : "")}";
        gopChecks.Add((label, media, options, GopArgument(transcoder, media, options, motionPhoto)));
    }
}

// HDR metadata parity: the parameter string must carry the same tokens whether the rule
// runs in the core or in the managed fallback. The cases cover static tokens, the Dolby
// Vision rate-control floor and the HDR10+ payload.
(string Fixture, string Codec)[] metadataCases =
[
    ("pq-10", "hevc"),
    ("pq-10", "av1"),
    ("dovi-10", "hevc"),
    ("dovi-10", "av1"),
    ("hdr10plus-10", "hevc"),
];
List<(string Label, MediaInfo Media, EncodeOptions Options, string Managed)> metadataChecks = new();
foreach ((string fixtureName, string codec) in metadataCases)
{
    MediaInfo media = Fixture.All.First(item => item.Name == fixtureName).ToMedia();
    EncodeOptions options = new("archive", codec, "hdr", "none");
    metadataChecks.Add(($"metadata/{fixtureName}/{codec}", media, options, ParameterArgument(transcoder, media, options)));
}

// Behaviour preservation through the public API: install the bridge, replay every
// request and require the same decision, resolved values and message.
List<(string Label, EncodeOptions Options, MediaInfo Media)> replay = new();
Dictionary<string, string> managedOutcomes = new(StringComparer.Ordinal);
foreach (Fixture fixture in Fixture.All)
{
    MediaInfo media = fixture.ToMedia();
    foreach ((string purpose, string codec, string output, string transform, string? profile) in validateCases)
    {
        EncodeOptions options = new(purpose, codec, output, transform, profile);
        string label = $"{fixture.Name}/{purpose}/{codec}/{output}/{transform}";
        replay.Add((label, options, media));
        managedOutcomes[label] = ManagedOutcome(transcoder, options, media);
    }
}

foreach ((string fixtureName, string label, EncodeOptions options) in edgeCases)
{
    MediaInfo media = Fixture.All.First(item => item.Name == fixtureName).ToMedia();
    replay.Add((label, options, media));
    managedOutcomes[label] = ManagedOutcome(transcoder, options, media);
}

if (!SharedCoreBridge.Install())
{
    Console.Error.WriteLine("native core unavailable; bridge replay was not run");
    return 3;
}

try
{
    foreach ((string label, EncodeOptions options, MediaInfo media) in replay)
    {
        Compare(label, "bridge", managedOutcomes[label], ManagedOutcome(transcoder, options, media), ref failures, ref comparisons);
    }

    Compare("bridge", "gop-route", "shared", SharedCore.Gop is null ? "managed" : "shared", ref failures, ref comparisons);
    Compare("bridge", "metadata-route", "shared", SharedCore.HdrMetadata is null ? "managed" : "shared", ref failures, ref comparisons);
    foreach ((string label, MediaInfo media, EncodeOptions options, string managed) in gopChecks)
    {
        bool motionPhoto = label.EndsWith("/motion", StringComparison.Ordinal);
        Compare(label, "gop", managed, GopArgument(transcoder, media, options, motionPhoto), ref failures, ref comparisons);
    }

    foreach ((string label, MediaInfo media, EncodeOptions options, string managed) in metadataChecks)
    {
        Compare(label, "metadata", managed, ParameterArgument(transcoder, media, options), ref failures, ref comparisons);
    }

    // The interval must follow the bridge, not merely find it installed: a sentinel
    // proves the transcoder still routes the rule instead of recomputing it locally.
    Func<GopRequest, long?>? installed = SharedCore.Gop;
    SharedCore.Gop = _ => 12345L;
    var sentinelCase = gopChecks[0];
    string sentinel = GopArgument(transcoder, sentinelCase.Media, sentinelCase.Options, false);
    SharedCore.Gop = installed;
    Compare(sentinelCase.Label, "gop-sentinel", "12345", sentinel, ref failures, ref comparisons);
}
finally
{
    SharedCoreBridge.Uninstall();
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

/// <summary>Returns the source with both probe rate fields replaced by one value.</summary>
static MediaInfo AtRate(MediaInfo media, double rate)
{
    JsonObject video = media.Video.DeepClone().AsObject();
    string text = rate.ToString("R", CultureInfo.InvariantCulture);
    video["avg_frame_rate"] = text;
    video["r_frame_rate"] = text;
    return media with
    {
        Video = video
    };
}

/// <summary>Builds a plan and returns the `-g` value the encoder would receive.</summary>
static string GopArgument(Transcoder transcoder, MediaInfo media, EncodeOptions options, bool motionPhoto)
{
    EncodingPlan plan = transcoder.Plan(new MediaTools("ffmpeg", "ffprobe"), media, options, "out.mkv", motionPhoto: motionPhoto);
    int index = Array.IndexOf(plan.Arguments, "-g");
    return index >= 0 && index + 1 < plan.Arguments.Length ? plan.Arguments[index + 1] : "missing";
}

/// <summary>Builds a plan and returns the encoder parameter string it would pass.</summary>
static string ParameterArgument(Transcoder transcoder, MediaInfo media, EncodeOptions options)
{
    EncodingPlan plan = transcoder.Plan(new MediaTools("ffmpeg", "ffprobe"), media, options, "out.mkv");
    string flag = options.Codec == "av1" ? "-svtav1-params" : "-x265-params";
    int index = Array.IndexOf(plan.Arguments, flag);
    return index >= 0 && index + 1 < plan.Arguments.Length ? plan.Arguments[index + 1] : "missing";
}

/// <summary>Formats a timestamp so both sides compare at the same precision.</summary>
static string Seconds(double value) => Math.Round(value, 6).ToString("R", System.Globalization.CultureInfo.InvariantCulture);

/// <summary>Reads a timestamp from the native window and formats it the same way.</summary>
static string NativeSeconds(JsonNode? window, string key)
{
    string text = window?[key]?.ToString() ?? "";
    return double.TryParse(text, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out double value)
        ? Seconds(value)
        : "NaN";
}

/// <summary>Runs resolve and validate, keeping the rejection message for comparison.</summary>
static string ManagedOutcome(Transcoder transcoder, EncodeOptions options, MediaInfo media)
{
    try
    {
        EncodeOptions resolved = transcoder.ResolveFormat(options, media);
        transcoder.Validate(options, media);
        return $"{resolved.Sampling}|{resolved.ColorRange}|{resolved.BitDepth}";
    }
    catch (InvalidDataException error)
    {
        return "error:" + error.Message;
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
    bool MasteringDisplay = false,
    bool DolbyVision = false,
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
        new("pq-10", "yuv420p10le", "pq", null, null, [10, 10, 10], ChromaW: 1, ChromaH: 1, MasteringDisplay: true),
        new("dovi-10", "yuv420p10le", "pq", null, null, [10, 10, 10], ChromaW: 1, ChromaH: 1,
            MasteringDisplay: true, DolbyVision: true),
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

        JsonArray sideData = new();
        if (Hdr10Plus)
        {
            sideData.Add(new JsonObject { ["side_data_type"] = "HDR10+ Metadata" });
        }

        if (DolbyVision)
        {
            sideData.Add(new JsonObject
            {
                ["side_data_type"] = "DOVI configuration record",
                ["dv_profile"] = 8,
                ["dv_bl_signal_compatibility_id"] = 1,
                ["rpu_present_flag"] = 1,
                ["bl_present_flag"] = 1,
                ["el_present_flag"] = 0
            });
        }

        if (MasteringDisplay)
        {
            sideData.Add(new JsonObject
            {
                ["side_data_type"] = "Mastering display metadata",
                ["green_x"] = "13250/50000",
                ["green_y"] = "34500/50000",
                ["blue_x"] = "7500/50000",
                ["blue_y"] = "3000/50000",
                ["red_x"] = "34000/50000",
                ["red_y"] = "16000/50000",
                ["white_point_x"] = "15635/50000",
                ["white_point_y"] = "16450/50000",
                ["max_luminance"] = "10000000/10000",
                ["min_luminance"] = "50/10000"
            });
            sideData.Add(new JsonObject
            {
                ["side_data_type"] = "Content light level metadata",
                ["max_content"] = 1000,
                ["max_average"] = 400
            });
        }

        if (sideData.Count > 0)
        {
            video["side_data_list"] = sideData;
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
