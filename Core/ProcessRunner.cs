using System.Buffers;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace VideoConverter.Core;

/// <summary>Complete output of a short-lived tool invocation; stderr remains separate from machine-readable stdout.</summary>
public sealed record ProcessResult(int ExitCode, string Output, string Error);
public sealed record BinaryProcessResult(int ExitCode, byte[] Output, string Error);

/// <summary>Starts external tools, drains their pipes and owns cancellation and file-identity checks.</summary>
public static class ProcessRunner
{
    /// <summary>Captures a bounded binary frame, drains stderr and joins the child on every exit path.</summary>
    public static async Task<BinaryProcessResult> CaptureBinaryAsync(string executable, IEnumerable<string> arguments,
        CancellationToken cancellation = default, int maximumBytes = 8 * 1024 * 1024, string? directory = null)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maximumBytes);
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(60));
        using var lifetime = CancellationTokenSource.CreateLinkedTokenSource(cancellation, timeout.Token);
        using var process = new Process { StartInfo = StartInfo(executable, arguments, directory) };
        cancellation.ThrowIfCancellationRequested();
        process.Start();
        using var registration = lifetime.Token.Register(() => Stop(process));
        var error = ReadTailAsync(process.StandardError, 8192);
        try
        {
            byte[] storage = ArrayPool<byte>.Shared.Rent(Math.Min(262144, maximumBytes));
            byte[] bytes;
            int length = 0;
            try
            {
                while (true)
                {
                    if (length == maximumBytes)
                    {
                        byte[] extra = new byte[1];
                        if (await process.StandardOutput.BaseStream.ReadAsync(extra, lifetime.Token).ConfigureAwait(false) != 0)
                        {
                            throw new InvalidDataException("Preview output exceeds its buffer limit.");
                        }
                        break;
                    }
                    if (length == storage.Length)
                    {
                        byte[] larger = ArrayPool<byte>.Shared.Rent((int)Math.Min((long)storage.Length * 2, maximumBytes));
                        storage.AsSpan(0, length).CopyTo(larger);
                        ArrayPool<byte>.Shared.Return(storage);
                        storage = larger;
                    }
                    int count = await process.StandardOutput.BaseStream.ReadAsync(storage.AsMemory(length,
                        Math.Min(storage.Length, maximumBytes) - length), lifetime.Token).ConfigureAwait(false);
                    if (count == 0)
                    {
                        break;
                    }
                    length += count;
                }
                bytes = storage.AsSpan(0, length).ToArray();
            }
            finally { ArrayPool<byte>.Shared.Return(storage); }
            await process.WaitForExitAsync().ConfigureAwait(false);
            cancellation.ThrowIfCancellationRequested();
            if (timeout.IsCancellationRequested)
            {
                throw new TimeoutException("External tool timed out.");
            }
            return new(process.ExitCode, bytes, await error.ConfigureAwait(false));
        }
        catch (OperationCanceledException) when (!cancellation.IsCancellationRequested && timeout.IsCancellationRequested)
        {
            throw new TimeoutException("External tool timed out.");
        }
        finally
        {
            Stop(process);
            await process.WaitForExitAsync().ConfigureAwait(false);
            await error.ConfigureAwait(false);
        }
    }

    /// <summary>Retains recent diagnostics while consuming the full pipe to prevent a blocked child.</summary>
    internal static async Task<string> ReadTailAsync(TextReader reader, int capacity)
    {
        char[] buffer = new char[Math.Min(capacity, 4096)];
        var tail = new StringBuilder();
        int read;
        while ((read = await reader.ReadAsync(buffer).ConfigureAwait(false)) > 0)
        {
            tail.Append(buffer, 0, read);
            if (tail.Length > capacity)
            {
                tail.Remove(0, tail.Length - capacity);
            }
        }
        return tail.ToString().Trim();
    }

    /// <summary>Creates a hidden, shell-free invocation with UTF-8 output and one ArgumentList entry per argument.</summary>
    public static ProcessStartInfo StartInfo(string executable, IEnumerable<string> arguments, string? directory = null)
    {
        var info = new ProcessStartInfo(executable)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8
        };
        if (directory is not null)
        {
            info.WorkingDirectory = directory;
        }

        foreach (var argument in arguments)
        {
            info.ArgumentList.Add(argument);
        }

        return info;
    }

    public static void Stop(Process process)
    {
        // The process may exit between HasExited and Kill. That race is normal;
        // an unrelated access error must still reach the caller.
        try
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
            }
        }
        catch (InvalidOperationException) { }
        catch (System.ComponentModel.Win32Exception) when (process.HasExited) { }
    }

    /// <summary>Captures a probe with independent elapsed-time and UTF-8 byte limits.</summary>
    /// <remarks>Cancellation kills the child tree and waits for EOF, so a cancelled probe cannot outlive its task.</remarks>
    public static async Task<ProcessResult> CaptureAsync(string executable, IEnumerable<string> arguments,
        CancellationToken cancellation = default, int timeoutSeconds = 25, string? directory = null,
        int maximumBytes = MediaResourceLimits.TextOutputBytes)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maximumBytes);
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(timeoutSeconds));
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellation, timeout.Token);
        using var process = new Process { StartInfo = StartInfo(executable, arguments, directory) };
        cancellation.ThrowIfCancellationRequested();
        process.Start();
        using var registration = linked.Token.Register(() => Stop(process));
        var stdout = ReadTextAsync(process.StandardOutput.BaseStream, maximumBytes, linked.Token);
        var stderr = ReadTailAsync(process.StandardError, 8192);
        try
        {
            // Observe reader failure before awaiting process exit: a tool may
            // otherwise block forever on the pipe after its output is rejected.
            var output = await stdout.ConfigureAwait(false);
            await process.WaitForExitAsync().ConfigureAwait(false);
            cancellation.ThrowIfCancellationRequested();
            if (timeout.IsCancellationRequested)
            {
                throw new TimeoutException("External tool timed out.");
            }
            return new(process.ExitCode, output, await stderr.ConfigureAwait(false));
        }
        catch (OperationCanceledException) when (!cancellation.IsCancellationRequested && timeout.IsCancellationRequested)
        {
            throw new TimeoutException("External tool timed out.");
        }
        finally
        {
            Stop(process);
            await process.WaitForExitAsync().ConfigureAwait(false);
            await stderr.ConfigureAwait(false);
        }
    }

    /// <summary>Decodes UTF-8 incrementally; byte limits also apply to multibyte text and unterminated output.</summary>
    private static async Task<string> ReadTextAsync(Stream stream, int maximumBytes, CancellationToken cancellation)
    {
        byte[] buffer = new byte[8192];
        char[] characters = new char[Encoding.UTF8.GetMaxCharCount(buffer.Length)];
        var decoder = Encoding.UTF8.GetDecoder();
        var text = new StringBuilder();
        int total = 0;
        bool started = false;
        while (true)
        {
            // Read at most one byte beyond the limit, including at exact EOF.
            int count = await stream.ReadAsync(buffer.AsMemory(0,
                (int)Math.Min(buffer.Length, (long)maximumBytes - total + 1)), cancellation).ConfigureAwait(false);
            if (count > maximumBytes - total)
            {
                throw new InvalidDataException($"外部工具的文本输出超过 {maximumBytes} 字节，已停止读取。");
            }
            total += count;
            int decoded = decoder.GetChars(buffer.AsSpan(0, count), characters, flush: count == 0);
            int start = 0;
            if (!started && decoded > 0)
            {
                started = true;
                start = characters[0] == '\uFEFF' ? 1 : 0;
            }
            text.Append(characters, start, decoded - start);
            if (count == 0)
            {
                return text.ToString();
            }
        }
    }

    /// <summary>Uses an existing saved path first, then searches PATH for a bare executable name.</summary>
    /// <returns>The full path, or an empty string when no candidate exists.</returns>
    public static string FindTool(string name, string saved = "")
    {
        if (File.Exists(saved))
        {
            return Path.GetFullPath(saved);
        }

        var executable = OperatingSystem.IsWindows() && !name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) ? name + ".exe" : name;
        // A package that carries FFmpeg keeps it beside the executable. That copy wins over
        // PATH, so the package behaves the same on a machine that has another build installed.
        var bundled = Path.Combine(AppContext.BaseDirectory, "tools", executable);
        if (File.Exists(bundled))
        {
            return bundled;
        }
        foreach (var folder in (Environment.GetEnvironmentVariable("PATH") ?? "").Split(Path.PathSeparator))
        {
            var path = Path.Combine(folder.Trim('"'), executable);
            if (File.Exists(path))
            {
                return Path.GetFullPath(path);
            }
        }
        return "";
    }

    /// <summary>Compares both path spelling and Windows volume/file IDs to detect aliases and hard links.</summary>
    /// <remarks>Used before opening output/log files; string equality alone cannot protect a hard-linked source.</remarks>
    public static bool SameFile(string left, string right)
    {
        if (string.Equals(Path.GetFullPath(left), Path.GetFullPath(right), OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal))
        {
            return true;
        }

        if (!File.Exists(left) || !File.Exists(right))
        {
            return false;
        }

        if (OperatingSystem.IsWindows())
        {
            using var a = File.OpenHandle(left, access: FileAccess.Read, share: FileShare.ReadWrite | FileShare.Delete);
            using var b = File.OpenHandle(right, access: FileAccess.Read, share: FileShare.ReadWrite | FileShare.Delete);
            if (!GetFileInformationByHandle(a, out var x) || !GetFileInformationByHandle(b, out var y))
            {
                throw new IOException("Unable to compare file identities.");
            }

            return x.Volume == y.Volume && x.IndexHigh == y.IndexHigh && x.IndexLow == y.IndexLow;
        }
        return new FileInfo(left).ResolveLinkTarget(true)?.FullName == Path.GetFullPath(right)
            || new FileInfo(right).ResolveLinkTarget(true)?.FullName == Path.GetFullPath(left);
    }

    [StructLayout(LayoutKind.Sequential)]
    // Native BY_HANDLE_FILE_INFORMATION layout; the volume serial and 64-bit
    // file index identify a file even when two directory entries have different names.
    private struct FileInformation
    {
        public uint Attributes;
        public System.Runtime.InteropServices.ComTypes.FILETIME Creation, Access, Write;
        public uint Volume, SizeHigh, SizeLow, Links, IndexHigh, IndexLow;
    }
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetFileInformationByHandle(SafeFileHandle file, out FileInformation information);
}
