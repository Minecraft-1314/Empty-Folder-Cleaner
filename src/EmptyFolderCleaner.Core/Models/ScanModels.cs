namespace EmptyFolderCleaner.Core.Models;

public sealed record FolderEntry(string Path, DateTime ModifiedUtc);

public sealed record ScanStats(long VisitedDirectories, long EmptyFolders, TimeSpan Elapsed);

public sealed record ScanReport(ScanStats Stats, bool WasCancelled);

public sealed record ScanOptions(
    string RootDirectory,
    IReadOnlySet<string> IgnoredPaths,
    IReadOnlyList<string> IgnorePatterns,
    int MaxDegreeOfParallelism);
