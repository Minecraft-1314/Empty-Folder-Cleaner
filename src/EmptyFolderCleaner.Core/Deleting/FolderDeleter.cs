using EmptyFolderCleaner.Core.Filtering;
using EmptyFolderCleaner.Core.Models;
using EmptyFolderCleaner.Core.Security;

namespace EmptyFolderCleaner.Core.Deleting;

public static class FolderDeleter
{
    private const int ProgressInterval = 8;

    public static Task<DeleteReport> DeleteAsync(
        DeleteRequest request,
        Action<DeleteEvent> onEvent,
        IProgress<DeleteProgress>? progress,
        CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(onEvent);
        cancellationToken.ThrowIfCancellationRequested();
        return Task.Run(
            () => ExecuteAsync(request, onEvent, progress, cancellationToken),
            cancellationToken);
    }

    private static async Task<DeleteReport> ExecuteAsync(
        DeleteRequest request,
        Action<DeleteEvent> onEvent,
        IProgress<DeleteProgress>? progress,
        CancellationToken cancellationToken)
    {
        if (!PathGuard.TryNormalize(request.RootDirectory, out var root))
        {
            throw new ArgumentException("root_directory_invalid", nameof(request));
        }

        var matcher = new IgnoreMatcher(
            request.IgnoredPaths,
            request.IgnorePatterns,
            caseInsensitive: OperatingSystem.IsWindows());
        var queue = new Queue<string>(BuildQueue(request, PathGuard.PathComparer));
        var scheduled = new HashSet<string>(PathGuard.PathComparer);
        foreach (var path in queue)
        {
            scheduled.Add(path);
        }

        var deleted = new List<string>();
        var failed = new List<string>();
        var remaining = new List<string>();
        var processed = 0;
        var announcedTotal = queue.Count;

        onEvent(new DeleteEvent(DeleteStage.Started, DeleteOutcome.Deleted, ""));

        while (queue.Count > 0)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                remaining.AddRange(queue);
                onEvent(new DeleteEvent(DeleteStage.BatchStopped, DeleteOutcome.Cancelled, ""));
                break;
            }

            var path = queue.Dequeue();
            var outcome = await TryDeleteAsync(root, path, request.UseRecycleBin, onEvent, cancellationToken)
                .ConfigureAwait(false);

            switch (outcome)
            {
                case DeleteOutcome.Deleted:
                case DeleteOutcome.MovedToRecycleBin:
                    deleted.Add(path);
                    CollectEmptiedAncestors(root, path, matcher, queue, scheduled, onEvent);
                    break;
                case DeleteOutcome.PermissionDenied:
                case DeleteOutcome.Failed:
                case DeleteOutcome.OutsideRoot:
                    failed.Add(path);
                    break;
                default:
                    remaining.Add(path);
                    break;
            }

            processed++;
            announcedTotal = Math.Max(announcedTotal, processed);
            if (processed % ProgressInterval == 0)
            {
                progress?.Report(new DeleteProgress(processed, announcedTotal));
            }
        }

        progress?.Report(new DeleteProgress(processed, announcedTotal));
        onEvent(new DeleteEvent(DeleteStage.Completed, DeleteOutcome.Deleted, ""));
        return new DeleteReport(
            deleted,
            failed,
            remaining,
            cancellationToken.IsCancellationRequested);
    }

    private static List<string> BuildQueue(DeleteRequest request, StringComparer comparer)
    {
        var ordered = new List<string>(request.Paths.Count);
        var seen = new HashSet<string>(comparer);
        foreach (var path in request.Paths)
        {
            if (PathGuard.TryNormalize(path, out var normalized) && seen.Add(normalized))
            {
                ordered.Add(normalized);
            }
        }

        ordered.Sort(static (left, right) =>
        {
            var byDepth = PathGuard.GetDepth(right).CompareTo(PathGuard.GetDepth(left));
            return byDepth != 0
                ? byDepth
                : string.Compare(left, right, PathGuard.PathComparison);
        });
        return ordered;
    }

    private static async Task<DeleteOutcome> TryDeleteAsync(
        string root,
        string path,
        bool useRecycleBin,
        Action<DeleteEvent> onEvent,
        CancellationToken cancellationToken)
    {
        if (!PathGuard.IsWithinRoot(root, path) || PathGuard.IsSamePath(root, path))
        {
            onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.OutsideRoot, path));
            return DeleteOutcome.OutsideRoot;
        }

        onEvent(new DeleteEvent(DeleteStage.Started, DeleteOutcome.Deleted, path));

        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var directory = new DirectoryInfo(path);
            if (!directory.Exists)
            {
                onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.AlreadyGone, path));
                return DeleteOutcome.AlreadyGone;
            }

            if ((directory.Attributes & FileAttributes.ReparsePoint) != 0 || HasAnyEntry(directory))
            {
                onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.NotEmpty, path));
                return DeleteOutcome.NotEmpty;
            }

            await Task.Yield();
            cancellationToken.ThrowIfCancellationRequested();

            if (useRecycleBin && RecycleBin.TryMove(path))
            {
                onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.MovedToRecycleBin, path));
                return DeleteOutcome.MovedToRecycleBin;
            }

            directory.Delete(recursive: false);
            onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.Deleted, path));
            return DeleteOutcome.Deleted;
        }
        catch (OperationCanceledException)
        {
            onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.Cancelled, path));
            return DeleteOutcome.Cancelled;
        }
        catch (UnauthorizedAccessException)
        {
            onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.PermissionDenied, path));
            return DeleteOutcome.PermissionDenied;
        }
        catch (Exception exception) when (IsRecoverable(exception))
        {
            onEvent(new DeleteEvent(DeleteStage.Finished, DeleteOutcome.Failed, path));
            return DeleteOutcome.Failed;
        }
    }

    private static void CollectEmptiedAncestors(
        string root,
        string deletedPath,
        IgnoreMatcher matcher,
        Queue<string> queue,
        HashSet<string> scheduled,
        Action<DeleteEvent> onEvent)
    {
        var parent = PathGuard.GetParent(deletedPath);
        while (!string.IsNullOrEmpty(parent)
            && PathGuard.IsWithinRoot(root, parent)
            && !PathGuard.IsSamePath(root, parent)
            && !matcher.IsIgnored(parent)
            && IsEmptyDirectory(parent)
            && scheduled.Add(parent))
        {
            onEvent(new DeleteEvent(DeleteStage.CascadeCandidate, DeleteOutcome.Deleted, parent));
            queue.Enqueue(parent);
            parent = PathGuard.GetParent(parent);
        }
    }

    private static bool HasAnyEntry(DirectoryInfo directory)
    {
        using var enumerator = directory.EnumerateFileSystemInfos().GetEnumerator();
        return enumerator.MoveNext();
    }

    private static bool IsEmptyDirectory(string path)
    {
        try
        {
            var directory = new DirectoryInfo(path);
            return directory.Exists && !HasAnyEntry(directory);
        }
        catch (Exception exception) when (IsRecoverable(exception))
        {
            return false;
        }
    }

    private static bool IsRecoverable(Exception exception)
    {
        return exception is IOException
            or UnauthorizedAccessException
            or System.Security.SecurityException
            or NotSupportedException;
    }
}
