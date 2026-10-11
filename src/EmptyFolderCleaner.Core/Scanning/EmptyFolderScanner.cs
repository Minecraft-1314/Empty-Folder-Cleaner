using System.Diagnostics;
using System.Threading.Channels;
using EmptyFolderCleaner.Core.Filtering;
using EmptyFolderCleaner.Core.Models;
using EmptyFolderCleaner.Core.Security;

namespace EmptyFolderCleaner.Core.Scanning;

public static class EmptyFolderScanner
{
    private const int ResultBatchSize = 128;
    private const int ChannelCapacity = 512;
    private const int ProgressInterval = 256;
    private const int MaxDirectoryDepth = 512;

    public static async Task<ScanReport> ScanAsync(
        ScanOptions options,
        Action<IReadOnlyList<FolderEntry>> onBatch,
        Action<ScanStats>? onProgress,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(options);
        ArgumentNullException.ThrowIfNull(onBatch);

        if (!PathGuard.TryNormalize(options.RootDirectory, out var root))
        {
            throw new ArgumentException("root_directory_invalid", nameof(options));
        }

        if (!Directory.Exists(root))
        {
            throw new DirectoryNotFoundException(root);
        }

        var matcher = new IgnoreMatcher(
            options.IgnoredPaths,
            options.IgnorePatterns,
            caseInsensitive: OperatingSystem.IsWindows());
        var maxWorkers = Math.Clamp(options.MaxDegreeOfParallelism, 1, Math.Max(1, Environment.ProcessorCount));

        var counters = new ScanCounters();
        var batch = new List<FolderEntry>(ResultBatchSize);
        var channel = Channel.CreateBounded<FolderEntry>(new BoundedChannelOptions(ChannelCapacity)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait,
            AllowSynchronousContinuations = false
        });

        using var linkedCancellation = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        var stopwatch = Stopwatch.StartNew();
        var producer = ProduceAsync(root, matcher, maxWorkers, channel.Writer, counters, linkedCancellation.Token);
        var wasCancelled = false;

        try
        {
            await foreach (var entry in channel.Reader.ReadAllAsync(linkedCancellation.Token))
            {
                batch.Add(entry);
                if (batch.Count >= ResultBatchSize)
                {
                    onBatch(batch.ToArray());
                    batch.Clear();
                }

                if (counters.Visited % ProgressInterval == 0)
                {
                    onProgress?.Invoke(counters.ToStats(stopwatch));
                }
            }

            await producer.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            wasCancelled = true;
            await ObserveCancellationAsync(producer).ConfigureAwait(false);
        }

        if (batch.Count > 0)
        {
            onBatch(batch.ToArray());
        }

        stopwatch.Stop();
        var stats = counters.ToStats(stopwatch);
        onProgress?.Invoke(stats);
        return new ScanReport(stats, wasCancelled || cancellationToken.IsCancellationRequested);
    }

    private static async Task ProduceAsync(
        string root,
        IgnoreMatcher matcher,
        int maxWorkers,
        ChannelWriter<FolderEntry> writer,
        ScanCounters counters,
        CancellationToken cancellationToken)
    {
        Exception? failure = null;
        try
        {
            var roots = EnumerateScanRoots(root, matcher, cancellationToken);
            var parallelOptions = new ParallelOptions
            {
                MaxDegreeOfParallelism = maxWorkers,
                CancellationToken = cancellationToken
            };

            await Parallel.ForEachAsync(
                roots,
                parallelOptions,
                async (directory, token) => await ScanSubtreeAsync(directory, matcher, writer, counters, token).ConfigureAwait(false))
                .ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception exception)
        {
            failure = exception;
            throw;
        }
        finally
        {
            writer.TryComplete(failure);
        }
    }

    private static List<string> EnumerateScanRoots(string root, IgnoreMatcher matcher, CancellationToken cancellationToken)
    {
        var roots = new List<string>();
        try
        {
            foreach (var directory in new DirectoryInfo(root).EnumerateDirectories())
            {
                cancellationToken.ThrowIfCancellationRequested();
                if (IsLink(directory) || matcher.IsIgnored(directory.FullName))
                {
                    continue;
                }

                roots.Add(directory.FullName);
            }
        }
        catch (Exception exception) when (IsRecoverable(exception))
        {
            throw new DirectoryNotFoundException(root, exception);
        }

        return roots;
    }

    private static async ValueTask ScanSubtreeAsync(
        string startDirectory,
        IgnoreMatcher matcher,
        ChannelWriter<FolderEntry> writer,
        ScanCounters counters,
        CancellationToken cancellationToken)
    {
        var pending = new Stack<string>(64);
        pending.Push(startDirectory);

        while (pending.TryPop(out var current))
        {
            cancellationToken.ThrowIfCancellationRequested();
            var depth = PathGuard.GetDepth(current);
            if (depth > MaxDirectoryDepth)
            {
                continue;
            }

            var directory = new DirectoryInfo(current);
            FileAttributes attributes;
            try
            {
                attributes = directory.Attributes;
            }
            catch (Exception exception) when (IsRecoverable(exception))
            {
                continue;
            }

            if ((attributes & FileAttributes.ReparsePoint) != 0)
            {
                continue;
            }

            var hasChildren = false;
            var deferred = new List<string>();
            try
            {
                foreach (var entry in directory.EnumerateFileSystemInfos())
                {
                    hasChildren = true;
                    if ((entry.Attributes & FileAttributes.ReparsePoint) != 0
                        || (entry.Attributes & FileAttributes.Directory) == 0)
                    {
                        continue;
                    }

                    if (!matcher.IsIgnored(entry.FullName))
                    {
                        deferred.Add(entry.FullName);
                    }
                }
            }
            catch (Exception exception) when (IsRecoverable(exception))
            {
                hasChildren = true;
            }

            counters.IncrementVisited();
            if (!hasChildren)
            {
                counters.IncrementEmpty();
                await writer.WriteAsync(
                    new FolderEntry(directory.FullName, directory.LastWriteTimeUtc),
                    cancellationToken).ConfigureAwait(false);
                continue;
            }

            for (var index = deferred.Count - 1; index >= 0; index--)
            {
                pending.Push(deferred[index]);
            }
        }
    }

    private static async Task ObserveCancellationAsync(Task producer)
    {
        try
        {
            await producer.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception exception) when (IsRecoverable(exception))
        {
        }
    }

    private static bool IsLink(DirectoryInfo directory)
    {
        try
        {
            return (directory.Attributes & FileAttributes.ReparsePoint) != 0;
        }
        catch (Exception exception) when (IsRecoverable(exception))
        {
            return true;
        }
    }

    private static bool IsRecoverable(Exception exception)
    {
        return exception is IOException
            or UnauthorizedAccessException
            or System.Security.SecurityException
            or DirectoryNotFoundException;
    }

    private sealed class ScanCounters
    {
        private long _visited;
        private long _empty;

        public long Visited => Interlocked.Read(ref _visited);

        public void IncrementVisited() => Interlocked.Increment(ref _visited);

        public void IncrementEmpty() => Interlocked.Increment(ref _empty);

        public ScanStats ToStats(Stopwatch stopwatch)
        {
            return new ScanStats(
                Interlocked.Read(ref _visited),
                Interlocked.Read(ref _empty),
                stopwatch.Elapsed);
        }
    }
}
