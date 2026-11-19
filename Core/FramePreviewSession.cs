namespace VideoConverter.Core;

/// <summary>A frame key within one source timeline; output-format changes do not invalidate SDR previews.</summary>
/// <param name="Approximate">True while the user is still dragging: seek to the nearest
/// keyframe instead of decoding forward to the exact one.</param>
public sealed record FramePreviewRequest(int Frame, string SourceKind, string? Profile, bool Approximate = false);

/// <summary>Runs one decoder with one replaceable pending request and a bounded PNG cache.</summary>
/// <remarks>Callbacks run on the worker. A GUI must marshal presentation to its dispatcher.
/// Pointer movement does not kill the active decoder: completed frames keep the viewport moving,
/// and the newest pending frame is rendered next. Cancel invalidates all older callbacks.</remarks>
public sealed class FramePreviewSession(
    Func<FramePreviewRequest, CancellationToken, Task<byte[]>> render,
    Func<FramePreviewRequest, byte[], CancellationToken, Task> present,
    Func<Exception, CancellationToken, Task> failed,
    int maximumCacheBytes = 8 * 1024 * 1024) : IAsyncDisposable
{
    private readonly int cacheLimit = maximumCacheBytes >= 0 ? maximumCacheBytes
        : throw new ArgumentOutOfRangeException(nameof(maximumCacheBytes));
    private readonly Lock gate = new();
    private readonly Dictionary<FramePreviewRequest, LinkedListNode<(FramePreviewRequest Key, byte[] Data)>> cache = [];
    private readonly LinkedList<(FramePreviewRequest Key, byte[] Data)> recent = [];
    private FramePreviewRequest? pending, latest;
    private CancellationTokenSource? active;
    private Task completion = Task.CompletedTask;
    private bool draining, disposed;
    // Epoch changes invalidate an in-flight result even when the child returns
    // successfully before it observes cancellation. Cache ownership is per source.
    private int epoch, cacheBytes;
    public int CachedBytes
    {
        get
        {
            lock (gate)
            {
                return cacheBytes;
            }
        }
    }

    /// <summary>Replaces unstarted work. Repeated keys already pending or in flight are coalesced.</summary>
    public void Request(FramePreviewRequest request)
    {
        lock (gate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (request == latest && draining)
            {
                return;
            }
            latest = pending = request;
            if (!draining)
            {
                StartLocked();
            }
        }
    }

    private void StartLocked()
    {
        draining = true;
        completion = Task.Run(DrainAsync);
    }

    private byte[]? FindLocked(FramePreviewRequest request)
    {
        if (!cache.TryGetValue(request, out var node))
        {
            return null;
        }
        recent.Remove(node);
        recent.AddFirst(node);
        return node.Value.Data;
    }

    private void RememberLocked(FramePreviewRequest request, byte[] bytes)
    {
        if (cacheLimit == 0 || bytes.Length > cacheLimit || cache.ContainsKey(request))
        {
            return;
        }
        while (recent.Last is { } last && (bytes.Length > cacheLimit - cacheBytes || cache.Count >= 12))
        {
            recent.RemoveLast();
            cache.Remove(last.Value.Key);
            cacheBytes -= last.Value.Data.Length;
        }
        cache[request] = recent.AddFirst((request, bytes));
        cacheBytes += bytes.Length;
    }

    private async Task DrainAsync()
    {
        try
        {
            while (true)
            {
                FramePreviewRequest request;
                byte[]? bytes;
                int revision;
                using var cancellation = new CancellationTokenSource();
                lock (gate)
                {
                    if (disposed || pending is null)
                    {
                        return;
                    }
                    request = pending;
                    pending = null;
                    revision = epoch;
                    active = cancellation;
                    bytes = FindLocked(request);
                }
                try
                {
                    bytes ??= await render(request, cancellation.Token).ConfigureAwait(false);
                    bool publish;
                    lock (gate)
                    {
                        publish = !disposed && revision == epoch;
                        if (publish)
                        {
                            RememberLocked(request, bytes);
                            // Prefer an already-cached latest frame over an older
                            // decode. Other completed frames provide live drag feedback.
                            publish = pending is null || !cache.ContainsKey(pending);
                        }
                    }
                    cancellation.Token.ThrowIfCancellationRequested();
                    if (publish)
                    {
                        await present(request, bytes, cancellation.Token).ConfigureAwait(false);
                    }
                }
                catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
                catch (Exception error)
                {
                    bool current;
                    lock (gate)
                    {
                        current = !disposed && revision == epoch && request == latest;
                    }
                    if (current)
                    {
                        await failed(error, cancellation.Token).ConfigureAwait(false);
                    }
                }
                finally
                {
                    lock (gate)
                    {
                        if (ReferenceEquals(active, cancellation))
                        {
                            active = null;
                        }
                    }
                }
            }
        }
        finally
        {
            lock (gate)
            {
                draining = false;
                if (!disposed && pending is not null)
                {
                    StartLocked();
                }
            }
        }
    }

    /// <summary>Invalidates queued work and joins the decoder before an export takes ownership of resources.</summary>
    public Task CancelAsync()
    {
        Task join;
        CancellationTokenSource? cancellation;
        lock (gate)
        {
            epoch++;
            pending = latest = null;
            cancellation = active;
            join = completion;
        }
        return CancelAndJoinAsync(cancellation, join);
    }

    /// <summary>Releases cached frames immediately, then waits for outstanding rendering and callbacks.</summary>
    public async ValueTask DisposeAsync()
    {
        Task join;
        CancellationTokenSource? cancellation;
        lock (gate)
        {
            disposed = true;
            epoch++;
            pending = latest = null;
            cache.Clear();
            recent.Clear();
            cacheBytes = 0;
            cancellation = active;
            join = completion;
        }
        await CancelAndJoinAsync(cancellation, join).ConfigureAwait(false);
    }

    private static async Task CancelAndJoinAsync(CancellationTokenSource? cancellation, Task join)
    {
        // Cancellation callbacks may synchronously query the session. Never run
        // them under its lock; the worker may also finish and dispose its token.
        try
        {
            if (cancellation is not null)
            {
                await cancellation.CancelAsync().ConfigureAwait(false);
            }
        }
        catch (ObjectDisposedException) { }
        await join.ConfigureAwait(false);
    }

    /// <summary>Waits until both active work and any request arriving during its completion have drained.</summary>
    public async Task WaitForIdleAsync()
    {
        while (true)
        {
            Task wait;
            lock (gate)
            {
                if (!draining)
                {
                    return;
                }
                wait = completion;
            }
            await wait.ConfigureAwait(false);
        }
    }
}
