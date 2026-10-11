namespace EmptyFolderCleaner.Core.Models;

public enum DeleteOutcome
{
    Deleted,
    MovedToRecycleBin,
    NotEmpty,
    AlreadyGone,
    OutsideRoot,
    PermissionDenied,
    Failed,
    Cancelled
}

public enum DeleteStage
{
    Started,
    Finished,
    CascadeCandidate,
    BatchStopped,
    Completed
}

public sealed record DeleteEvent(DeleteStage Stage, DeleteOutcome Outcome, string Path);

public sealed record DeleteProgress(int Completed, int Total);

public sealed record DeleteRequest(
    string RootDirectory,
    IReadOnlyList<string> Paths,
    bool UseRecycleBin,
    IReadOnlySet<string> IgnoredPaths,
    IReadOnlyList<string> IgnorePatterns);

public sealed record DeleteReport(
    IReadOnlyList<string> DeletedPaths,
    IReadOnlyList<string> FailedPaths,
    IReadOnlyList<string> RemainingPaths,
    bool WasCancelled);
