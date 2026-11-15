using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace VideoConverter.Native;

/// <summary>A stable error identity returned by the shared core.</summary>
/// <remarks>The code is localized by the front end; <see cref="Detail"/> only carries the rejected value.</remarks>
public sealed record LumaCoreError(string Code, string? Detail);

/// <summary>Managed access to the versioned C ABI of the Rust shared core.</summary>
/// <remarks>
/// The library is optional at startup: a managed-only build, or a machine that cannot
/// load the binary, keeps <see cref="IsAvailable"/> false and every call reports
/// <c>ffi.unavailable</c>. Nothing here throws on a missing library, so the Windows
/// application can start and fall back while a native build is being validated.
/// </remarks>
public static class LumaCoreNative
{
    private const string Library = "lumacore";
    private static readonly Lazy<Probe> probe = new(Probe.Load, LazyThreadSafetyMode.ExecutionAndPublication);

    /// <summary>True when the native library loaded and answered the version query.</summary>
    public static bool IsAvailable => probe.Value.Available;

    /// <summary>ABI revision reported by the loaded library; zero when unavailable.</summary>
    public static uint AbiVersion => probe.Value.AbiVersion;

    /// <summary>Core crate version, or null when unavailable.</summary>
    public static string? CoreVersion => probe.Value.CoreVersion;

    /// <summary>Operations advertised by the loaded library.</summary>
    public static IReadOnlyList<string> Operations => probe.Value.Operations ?? [];

    /// <summary>Load failure description, for diagnostics only.</summary>
    public static string? LoadError => probe.Value.Error;

    /// <summary>Calls one operation and returns its JSON data envelope.</summary>
    public static bool TryCall(string operation, JsonNode? request, out JsonNode? data, out LumaCoreError? error)
    {
        data = null;
        if (!IsAvailable)
        {
            error = new LumaCoreError("ffi.unavailable", probe.Value.Error);
            return false;
        }

        string? response = Invoke(operation, request?.ToJsonString() ?? "{}");
        return TryReadEnvelope(response, out data, out error);
    }

    /// <summary>Calls one operation and throws when the core rejects it.</summary>
    public static JsonNode? Call(string operation, JsonNode? request = null)
        => TryCall(operation, request, out JsonNode? data, out LumaCoreError? error)
            ? data
            : throw new InvalidOperationException($"{error!.Code}: {error.Detail}");

    private static bool TryReadEnvelope(string? response, out JsonNode? data, out LumaCoreError? error)
    {
        data = null;
        error = null;
        if (response is null)
        {
            error = new LumaCoreError("ffi.null_result", null);
            return false;
        }

        JsonNode? envelope;
        try
        {
            envelope = JsonNode.Parse(response);
        }
        catch (JsonException)
        {
            error = new LumaCoreError("ffi.envelope", null);
            return false;
        }

        if (envelope is not JsonObject payload)
        {
            error = new LumaCoreError("ffi.envelope", null);
            return false;
        }

        bool ok = payload["ok"] is JsonValue flag && flag.TryGetValue(out bool value) && value;
        if (ok)
        {
            data = payload["data"];
            return true;
        }

        JsonObject? failure = payload["error"] as JsonObject;
        error = new LumaCoreError(
            failure?["code"] is JsonValue code && code.TryGetValue(out string? text) ? text : "ffi.error",
            failure?["detail"] is JsonValue detail && detail.TryGetValue(out string? described) ? described : null);
        return false;
    }

    /// <summary>Copies UTF-8 arguments into native memory, calls, and frees the result.</summary>
    private static string? Invoke(string operation, string request)
    {
        IntPtr op = IntPtr.Zero;
        IntPtr payload = IntPtr.Zero;
        IntPtr result = IntPtr.Zero;
        try
        {
            op = Marshal.StringToCoTaskMemUTF8(operation);
            payload = Marshal.StringToCoTaskMemUTF8(request);
            result = luma_call_json(op, payload);
            return result == IntPtr.Zero ? null : Marshal.PtrToStringUTF8(result);
        }
        finally
        {
            if (op != IntPtr.Zero)
            {
                Marshal.FreeCoTaskMem(op);
            }

            if (payload != IntPtr.Zero)
            {
                Marshal.FreeCoTaskMem(payload);
            }

            if (result != IntPtr.Zero)
            {
                luma_string_free(result);
            }
        }
    }

    private sealed record Probe(bool Available, uint AbiVersion, string? CoreVersion, IReadOnlyList<string>? Operations, string? Error)
    {
        public static Probe Load()
        {
            try
            {
                NativeLibrary.SetDllImportResolver(typeof(LumaCoreNative).Assembly, Resolve);
            }
            catch (InvalidOperationException)
            {
                // Another load path already registered a resolver; keep going.
            }

            try
            {
                uint abi = luma_abi_version();
                string? version = Marshal.PtrToStringUTF8(luma_core_version());
                IReadOnlyList<string>? operations = null;
                if (TryReadEnvelope(Invoke("core.info", "{}"), out JsonNode? info, out _)
                    && info?["operations"] is JsonArray declared)
                {
                    operations = declared.Select(node => node?.ToString() ?? "").Where(name => name.Length > 0).ToArray();
                }

                return new Probe(true, abi, version, operations, null);
            }
            catch (Exception e) when (e is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException or InvalidOperationException)
            {
                return new Probe(false, 0, null, null, e.Message);
            }
        }

        private static IntPtr Resolve(string name, Assembly assembly, DllImportSearchPath? path)
        {
            if (!string.Equals(name, Library, StringComparison.Ordinal))
            {
                return IntPtr.Zero;
            }

            foreach (string candidate in Candidates())
            {
                if (NativeLibrary.TryLoad(candidate, out IntPtr handle))
                {
                    return handle;
                }
            }

            return IntPtr.Zero;
        }

        private static IEnumerable<string> Candidates()
        {
            string directory = AppContext.BaseDirectory;
            if (OperatingSystem.IsWindows())
            {
                yield return Path.Combine(directory, "lumacore.dll");
            }
            else if (OperatingSystem.IsMacOS())
            {
                yield return Path.Combine(directory, "liblumacore.dylib");
            }
            else
            {
                yield return Path.Combine(directory, "liblumacore.so");
            }

            // Repository layout used by a development build.
            string arch = RuntimeInformation.ProcessArchitecture.ToString().ToLowerInvariant();
            string artifacts = Path.GetFullPath(Path.Combine(directory, "..", "..", "..", "..", "..", "artifacts", "native", arch));
            yield return OperatingSystem.IsWindows() ? Path.Combine(artifacts, "lumacore.dll") : Path.Combine(artifacts, "liblumacore.dylib");
            yield return Library;
        }
    }

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern uint luma_abi_version();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern IntPtr luma_core_version();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern IntPtr luma_call_json(IntPtr operation, IntPtr request);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern void luma_string_free(IntPtr value);
}
